//! Bitcoin Core wallet records, read only: the crate of the reader. Its design
//! and what it refuses are in `docs/decisions/F229.md`.
//!
//! A Bitcoin Core descriptor wallet's records, read and checked
//! (`docs/decisions/F229.md`): the `(key, value)` pairs of the table
//! `main` of a `wallet.dat` and the application id its header carries,
//! read into what the wallet holds. [`read_file`] takes the file's bytes
//! (read by `faraday-sqlite`) and does the whole; nothing is written.
//!
//! [`read`] gives the network (the application id is its message start),
//! the version, the flags, each descriptor with its id, its text as stored and as Bitcoin Core lists
//! it (`normal.rs`: origins and the cached account key), range and next
//! index, which descriptors are active, the labels, the master key's
//! method and rounds, how many transactions the file holds and its block
//! locator, and the record types it does not read by name and count.
//! Each record that fails is named with what failed
//! ([`Wallet::failures`]); the wallet is whole only when none does
//! ([`Wallet::is_whole`]). A wallet that is refused outright is a
//! [`Refusal`]: legacy records, flags without descriptors, another
//! network than the mode's, a descriptor with a key that has no origin.
//! A key that is in the file twice, more than one `mkey` record, a plain
//! private key in an encrypted wallet and an `mkey` record of a method
//! other than 0 or of rounds none or past the cap fail their records.
//! A `combo()` descriptor of one key (what `migratewallet` makes of a legacy
//! wallet's keys) is not a failure: it is checked as any descriptor is and
//! kept in [`Wallet::kept`], listed and never taken.
//!
//! The private keys are checked in `faraday-keys` ([`faraday_keys::core_wallet`]):
//! each `walletdescriptorkey` record's private key must give the public
//! key it names and match its stored hash, and with the passphrase each
//! `walletdescriptorckey` record is opened with the master key and
//! checked the same way. They are never given here, only pass or fail.

use faraday_bip::descriptor::{descriptor_checksum, key_facts, verify_checksum};
use faraday_bip::keys::Network;
use faraday_crypto::sha256;
use faraday_crypto::work::Compute;
use faraday_intake::secret_buf::SecretText;
use faraday_keys::core_wallet::{
    AccountCheck, KeyVerdict, MAX_ROUNDS, METHOD_SHA512_AES, OpenError,
    check_encrypted_keys_accounts, check_private_key_accounts,
};
use faraday_strings::EN;
use faraday_text::{Text, text};
use std::collections::{HashMap, HashSet};
use zeroize::{Zeroize, Zeroizing};

use faraday_intake::wallet::network_name;

mod normal;
use normal::{Failed, Need};

/// The 78 bytes of an extended key in base58check, or `None`: the decoder
/// the descriptor reader uses, for the cross-check only.
#[cfg(feature = "crosscheck")]
#[doc(hidden)]
#[must_use]
pub fn extended_key_bytes_for_crosscheck(text: &str) -> Option<Vec<u8>> {
    normal::extended_key_bytes(text).map(|k| k.to_vec())
}

/// The wallet flag bits Bitcoin Core knows (`walletutil.h`).
const KNOWN_FLAGS: u64 = 0b111 | (0b1111 << 32);
/// `WALLET_FLAG_DESCRIPTORS`.
const FLAG_DESCRIPTORS: u64 = 1 << 34;
/// The database version (`PRAGMA user_version`) Bitcoin Core writes
/// (`WALLET_SCHEMA_VERSION`).
const WALLET_SCHEMA_VERSION: i32 = 0;

/// The most failed records, and the most distinct unknown record types,
/// kept by name; the rest are counted ([`Wallet::failures_more`],
/// [`Wallet::unknown_more`]).
pub const MAX_LISTED: usize = 256;
/// The most bytes of a record's key after its type kept in a failure, in
/// hex: Bitcoin Core's longest is 98 (a 32-byte id, a length byte and a 65-byte
/// uncompressed public key).
const MAX_TAIL: usize = 128;
/// The longest unknown record type kept by name.
const MAX_NAMED: usize = 64;
/// The longest value of a private key record read: Bitcoin Core's are
/// under 400 bytes. A longer one fails its record as one of another layout.
const MAX_KEY_VALUE: usize = 1024;
/// The longest address a label record names: 128 bytes (Bitcoin Core's
/// addresses are under 100).
const MAX_ADDRESS: usize = 128;

/// Record types only a legacy (Berkeley DB era) wallet writes.
const LEGACY: [&str; 11] = [
    "key",
    "ckey",
    "hdchain",
    "keymeta",
    "watchs",
    "watchmeta",
    "cscript",
    "pool",
    "defaultkey",
    "wkey",
    "acentry",
];
/// Record types Bitcoin Core writes in a descriptor wallet that are
/// counted and not read.
const UNREAD: [&str; 5] = [
    "orderposnext",
    "walletdescriptorcache",
    "destdata",
    "lockedutxo",
    "settings",
];

/// The network whose message start is a wallet's SQLite application id:
/// mainnet, testnet (3), signet (the default one) and regtest. Another
/// id, testnet 4's included, names none Faraday has.
pub fn network_of_application_id(id: u32) -> Option<Network> {
    match id {
        0xF9BE_B4D9 => Some(Network::Mainnet),
        0x0B11_0907 => Some(Network::Testnet),
        0x0A03_CF40 => Some(Network::Signet),
        0xFABF_B5DA => Some(Network::Regtest),
        _ => None,
    }
}

/// Why a wallet is not read at all.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {
    /// The application id is not a network Faraday has.
    Application(u32),
    /// The wallet is of another kind of network than the mode's.
    Network {
        /// The wallet's.
        offered: Network,
        /// The mode's.
        loaded: Network,
    },
    /// Records only a legacy wallet writes, by type and count.
    Legacy(Vec<(Text, usize)>),
    /// The flags (none written counts as none) lack descriptors.
    NotDescriptors,
    /// A descriptor with a key that carries no origin: the record.
    NoOrigin(Text),
    /// The file is not a database Bitcoin Core wrote: the SQLite reader's
    /// refusal, named.
    File(faraday_sqlite::Error),
    /// A lock, a journal or a write-ahead log beside the file: its name.
    Beside(Text),
    /// A database version (`PRAGMA user_version`) other than the 0
    /// Bitcoin Core writes: the version.
    SchemaVersion(i32),
    /// A legacy wallet: a Berkeley DB file, told by its magic number.
    BerkeleyDb,
}

impl Refusal {
    /// The refusal as text.
    pub fn message(&self) -> Text {
        match self {
            Refusal::Application(id) => {
                Text::fill(EN.core_wallet_application, &[&text!("{id:08x}")])
            }
            Refusal::Network { offered, loaded } => Text::fill(
                EN.wallet_is_not_the_network,
                &[network_name(*offered), network_name(*loaded)],
            ),
            Refusal::Legacy(found) => {
                let mut names = Text::new();
                for (i, (n, k)) in found.iter().enumerate() {
                    if i > 0 {
                        names.push_str(", ");
                    }
                    names.push_str(&text!("{} {k}", n.as_str()));
                }
                Text::fill(EN.core_wallet_legacy, &[&names])
            }
            Refusal::NotDescriptors => Text::of(EN.core_wallet_not_descriptors),
            Refusal::NoOrigin(record) => Text::fill(EN.core_wallet_no_origin, &[record]),
            Refusal::File(faraday_sqlite::Error::NotSqlite) => Text::of(EN.core_wallet_not_sqlite),
            Refusal::File(faraday_sqlite::Error::Wal) => Text::of(EN.core_wallet_wal),
            Refusal::File(faraday_sqlite::Error::OtherTable(name)) => {
                Text::fill(EN.core_wallet_other_table, &[name])
            }
            Refusal::File(faraday_sqlite::Error::NoTable(name)) => {
                Text::fill(EN.core_wallet_no_table, &[name])
            }
            Refusal::File(faraday_sqlite::Error::TooManyRows) => Text::fill(
                EN.core_wallet_too_many_rows,
                &[&Text::num(faraday_sqlite::MAX_ROWS as u64)],
            ),
            Refusal::File(_) => Text::of(EN.core_wallet_file_damaged),
            Refusal::Beside(name) => Text::fill(EN.core_wallet_beside, &[name]),
            Refusal::SchemaVersion(v) => {
                Text::fill(EN.core_wallet_schema_version, &[&Text::int(i64::from(*v))])
            }
            Refusal::BerkeleyDb => Text::of(EN.core_wallet_berkeley),
        }
    }
}

/// What failed in a record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fault {
    /// The record is not the shape Bitcoin Core writes (a key or a value
    /// of the wrong length, text that is not UTF-8, bytes left over).
    Layout,
    /// The flags hold a bit Bitcoin Core does not know.
    UnknownFlags,
    /// The descriptor's text is not an output descriptor of public keys.
    Descriptor,
    /// The descriptor's checksum is missing or wrong.
    Checksum,
    /// The descriptor's id is not the hash of its text.
    Id,
    /// The range or next index is negative, the range is inverted, the range's
    /// size or the next index is 1,000,000 or more, or a descriptor with no `*`
    /// stores a range of more than one index.
    Range,
    /// The descriptor's keys are of another network than the file's.
    KeyNetwork,
    /// The record names a descriptor the file does not hold.
    NotHeld,
    /// A record's key is in the file twice (Bitcoin Core's table holds
    /// each key once), whatever its type.
    Repeated,
    /// A private key's record is not the shape Bitcoin Core writes.
    KeyMalformed,
    /// A private key's stored hash is not the one its bytes give.
    KeyHash,
    /// A private key gives another public key than the record names.
    KeyPublic,
    /// A `walletdescriptorckey` record and no `mkey` record.
    NoMasterKey,
    /// More than one `mkey` record (Bitcoin Core writes one); this is the
    /// second or a later one.
    ManyMasterKeys,
    /// A `walletdescriptorkey` record in an encrypted wallet, which
    /// Bitcoin Core does not unlock.
    PlainKeyEncrypted,
    /// The master key is of a derivation method Faraday does not open.
    MasterMethod,
    /// The master key's rounds are none or past what Faraday opens.
    MasterRounds,
    /// A descriptor's key at a hardened step needs the account key Core
    /// caches in a `walletdescriptorlhcache` record, and the file has none
    /// for it (the record named is the descriptor's).
    CacheMissing,
    /// A cached account key does not match: where the keys are checked and
    /// compared, a key that is not the one its master key gives (a bad point
    /// included) fails the `walletdescriptorlhcache` record; where they are
    /// not, a bad point, depth, child number or parent fingerprint fails the
    /// descriptor's record.
    CacheKey,
    /// A descriptor's cached account key was not compared with its master
    /// key, though the wallet's keys were checked, whatever its flags say:
    /// the descriptor has no key record for its master key.
    CacheUnchecked,
}

/// A failed record and what failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Failure {
    /// The record's type and its key's bytes in hex.
    pub record: Text,
    /// What failed.
    pub fault: Fault,
}

/// What a descriptor's output is: `OutputType` in Bitcoin Core.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputType {
    /// P2PKH.
    Legacy,
    /// P2SH-P2WPKH.
    P2shSegwit,
    /// P2WPKH.
    Bech32,
    /// P2TR.
    Bech32m,
}

/// One descriptor the file holds, with every check passed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Descriptor {
    /// The id: SHA-256 of the descriptor's text with its hardened marks
    /// as `'` and its checksum made over that.
    pub id: [u8; 32],
    /// The descriptor as stored, with its checksum.
    pub text: Text,
    /// The descriptor as Bitcoin Core's `listdescriptors` gives it, with
    /// its checksum: each key at a hardened step as `[fingerprint/path]`
    /// and the account key Core cached for it, a key with an origin with
    /// its origin, hardened steps written `h`.
    pub listed: Text,
    /// Unix time the descriptor was made, unsigned as Bitcoin Core writes
    /// it (`WalletDescriptor::creation_time`).
    pub creation_time: u64,
    /// The next index to hand out, as written.
    pub next_index: i32,
    /// The first and the last index derived as `listdescriptors` gives
    /// them (the file holds one past the last), for a descriptor that is
    /// ranged; `None` for one that is not, whatever the file holds.
    pub range: Option<(i32, i32)>,
    /// Private keys of it that are not encrypted and passed.
    pub private_keys: usize,
    /// Encrypted private keys of it; checked when the passphrase is
    /// given ([`Wallet::passphrase`]).
    pub encrypted_keys: usize,
}

/// A `combo()` descriptor (BIP 384) the file holds, which Bitcoin Core
/// keeps and Faraday does not take: Core's `migratewallet` makes one for
/// each key a legacy wallet held. Its text is public keys only, and its
/// checksum and id passed as any descriptor's do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Kept {
    /// The id: as [`Descriptor::id`].
    pub id: [u8; 32],
    /// The descriptor as stored, with its checksum.
    pub text: Text,
}

/// An active descriptor: the one a slot uses for new addresses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Active {
    /// Change (`activeinternalspk`) or receive (`activeexternalspk`).
    pub internal: bool,
    /// The kind of output.
    pub output_type: OutputType,
    /// The descriptor's id.
    pub id: [u8; 32],
}

/// An address's label.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Label {
    /// The address as stored.
    pub address: Text,
    /// Its `name` record.
    pub name: Option<Text>,
    /// Its `purpose` record.
    pub purpose: Option<Text>,
}

/// An `mkey` record's public part.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MasterRecord {
    /// The record's id.
    pub id: u32,
    /// The derivation method (0 is SHA-512 rounds, then AES-256-CBC).
    pub method: u32,
    /// The rounds.
    pub rounds: u32,
}

/// What came of the passphrase.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Passphrase {
    /// The wallet is not encrypted.
    NotNeeded,
    /// The wallet is encrypted and none was given: its encrypted keys
    /// are not checked.
    NotGiven,
    /// It opened the master key; each encrypted key was checked.
    Opened,
    /// It did not open the master key.
    Wrong,
}

/// A wallet read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Wallet {
    /// The network the application id names.
    pub network: Network,
    /// The `version` record: Bitcoin Core's version number that wrote it.
    pub version: Option<u32>,
    /// The flags.
    pub flags: u64,
    /// Each descriptor that passed its checks, in file order.
    pub descriptors: Vec<Descriptor>,
    /// The `combo()` descriptors that passed the same checks, in file
    /// order: listed, never taken.
    pub kept: Vec<Kept>,
    /// The active descriptors.
    pub active: Vec<Active>,
    /// The labels, in file order.
    pub labels: Vec<Label>,
    /// The `mkey` records: the wallet is encrypted when there is one.
    pub master_keys: Vec<MasterRecord>,
    /// What the passphrase came to.
    pub passphrase: Passphrase,
    /// `walletdescriptorkey` and `walletdescriptorckey` records checked
    /// and passed.
    pub keys_passed: usize,
    /// `walletdescriptorckey` records not checked, for want of the
    /// passphrase or because it was wrong.
    pub keys_unchecked: usize,
    /// How many `tx` records.
    pub tx_count: usize,
    /// How many hashes the block locator holds, where the file has one.
    pub block_locator: Option<usize>,
    /// Record types Bitcoin Core writes that are counted and not read.
    pub unread: Vec<(Text, usize)>,
    /// Record types Faraday does not know, by name and count: the first
    /// [`MAX_LISTED`] of at most 64 bytes each.
    pub unknown: Vec<(Text, usize)>,
    /// How many records are of an unknown type not in `unknown`: those
    /// past the first [`MAX_LISTED`] types, and types of a longer name.
    pub unknown_more: usize,
    /// The records that failed: the first [`MAX_LISTED`].
    pub failures: Vec<Failure>,
    /// How many more records failed than `failures` names.
    pub failures_more: usize,
}

impl Wallet {
    /// Whether every record read and every key checked passed, and the
    /// passphrase, if one was given, was right.
    pub fn is_whole(&self) -> bool {
        self.failures.is_empty() && self.failures_more == 0 && self.passphrase != Passphrase::Wrong
    }

    /// Whether the wallet is encrypted.
    pub fn is_encrypted(&self) -> bool {
        !self.master_keys.is_empty()
    }

    /// The descriptor of `id`.
    pub fn descriptor(&self, id: &[u8; 32]) -> Option<&Descriptor> {
        self.descriptors.iter().find(|d| &d.id == id)
    }
}

/// The `pkh()` descriptor of the one key of a `combo(KEY)` descriptor, with
/// its checksum: the text the descriptor library reads to check that key
/// (public, of a network, with its origin), since it does not read `combo`.
/// `None` for any other descriptor, and for a `combo` of anything but one
/// key expression (no parenthesis or comma inside it).
fn combo_twin(text: &str) -> Option<Text> {
    let (body, _) = text.split_once('#')?;
    let key = body.strip_prefix("combo(")?.strip_suffix(')')?;
    if key.is_empty() || key.contains(['(', ')', ',', '{', '}', '#']) {
        return None;
    }
    let twin = text!("pkh({key})");
    let sum = descriptor_checksum(&twin)?;
    Some(text!("{}#{}", &*twin, core::str::from_utf8(&sum).ok()?))
}

/// One record's key split: its type and the bytes after it.
fn split_key(key: &[u8]) -> Option<(&str, &[u8])> {
    let (len, used) = compact_size(key)?;
    let rest = key.get(used..)?;
    let len = usize::try_from(len).ok()?;
    let (name, tail) = rest.split_at_checked(len)?;
    Some((core::str::from_utf8(name).ok()?, tail))
}

/// Bitcoin Core's `CompactSize`, shortest form only: the value and the
/// bytes it takes.
fn compact_size(bytes: &[u8]) -> Option<(u64, usize)> {
    let (&first, rest) = bytes.split_first()?;
    let (value, width, least) = match first {
        0..=252 => return Some((u64::from(first), 1)),
        253 => (
            u64::from(u16::from_le_bytes(rest.get(..2)?.try_into().ok()?)),
            3,
            253,
        ),
        254 => (
            u64::from(u32::from_le_bytes(rest.get(..4)?.try_into().ok()?)),
            5,
            0x1_0000,
        ),
        255 => (
            u64::from_le_bytes(rest.get(..8)?.try_into().ok()?),
            9,
            0x1_0000_0000,
        ),
    };
    (value >= least).then_some((value, width))
}

/// A vector's bytes and what follows them.
fn vector(bytes: &[u8]) -> Option<(&[u8], &[u8])> {
    let (len, used) = compact_size(bytes)?;
    bytes
        .get(used..)?
        .split_at_checked(usize::try_from(len).ok()?)
}

/// A string and what follows it.
fn string(bytes: &[u8]) -> Option<(&str, &[u8])> {
    let (text, rest) = vector(bytes)?;
    Some((core::str::from_utf8(text).ok()?, rest))
}

/// A string that is all of `bytes`.
fn whole_string(bytes: &[u8]) -> Option<&str> {
    string(bytes)
        .filter(|(_, rest)| rest.is_empty())
        .map(|(s, _)| s)
}

fn hex(bytes: &[u8]) -> Text {
    faraday_crypto::hex::encode_string(bytes)
}

/// The id a descriptor's text has: SHA-256 of the text with every hardened
/// mark in its key paths written `'` and the checksum over that
/// (`DescriptorID` in Bitcoin Core: `ToString` in its compatible form).
/// `None` for a text with no `#` checksum part or characters no
/// descriptor holds.
fn descriptor_id(text: &str) -> Option<[u8; 32]> {
    let (body, _) = text.split_once('#')?;
    let mut compat = Text::with_room(body.len() + 9);
    let mut token = Text::new();
    for ch in body.chars() {
        if matches!(ch, '(' | ')' | ',' | '{' | '}') {
            apostrophes(&token, &mut compat);
            token.clear();
            compat.push(ch);
        } else {
            token.push(ch);
        }
    }
    apostrophes(&token, &mut compat);
    let sum = descriptor_checksum(&compat)?;
    compat.push('#');
    compat.push_str(core::str::from_utf8(&sum).ok()?);
    Some(sha256(compat.as_bytes()))
}

/// One key expression with its paths' hardened marks as `'`: the
/// origin's after its fingerprint, and the path after the key.
fn apostrophes(token: &str, out: &mut Text) {
    let mark = |out: &mut Text, path: &str| {
        for ch in path.chars() {
            out.push(if matches!(ch, 'h' | 'H') { '\'' } else { ch });
        }
    };
    let (origin, key) = match token.strip_prefix('[').and_then(|t| t.split_once(']')) {
        Some((inside, key)) => (Some(inside), key),
        None => (None, token),
    };
    if let Some(inside) = origin {
        out.push('[');
        match inside.split_once('/') {
            Some((fingerprint, path)) => {
                out.push_str(fingerprint);
                out.push('/');
                mark(out, path);
            }
            None => out.push_str(inside),
        }
        out.push(']');
    }
    match key.split_once('/') {
        Some((head, path)) => {
            out.push_str(head);
            out.push('/');
            mark(out, path);
        }
        None => out.push_str(key),
    }
}

/// A key record's public key, 33 bytes or 65 (uncompressed), after its
/// length.
fn key_pubkey(tail: &[u8]) -> Option<&[u8]> {
    let (pubkey, rest) = vector(tail)?;
    (matches!(pubkey.len(), 33 | 65) && rest.is_empty()).then_some(pubkey)
}

/// A key record held for its check: the descriptor's id, the public key,
/// the value (wiped on drop) and the record's key tail.
type KeyRecord = ([u8; 32], Vec<u8>, Zeroizing<Vec<u8>>, Vec<u8>);

/// The cached account keys a master key is the root of: their places in
/// the descriptor and what the check of each needs. The checks hold the
/// master key's chain code, so they are wiped when this is dropped.
#[derive(Default)]
struct Accounts {
    places: Vec<u32>,
    checks: Vec<AccountCheck>,
}

impl Drop for Accounts {
    fn drop(&mut self) {
        for check in &mut self.checks {
            check.chain_code.zeroize();
        }
    }
}

/// The cached account keys of a wallet's descriptors, borrowed from the
/// records: the descriptor's id and the key's place give the 74 bytes.
type Cache<'r> = HashMap<([u8; 32], u32), &'r [u8; 74]>;

/// A `walletdescriptorlhcache` record: the descriptor's id, the key's
/// place and the 74 bytes of the extended public key, where the record is
/// the shape Bitcoin Core writes (the key `id`, a place of 4 bytes; the
/// value a 74-byte vector holding a compressed public key).
fn lh_record<'r>(tail: &[u8], value: &'r [u8]) -> Option<([u8; 32], u32, &'r [u8; 74])> {
    let (id, place) = tail.split_at_checked(32)?;
    let place = <[u8; 4]>::try_from(place).ok()?;
    let (xpub, rest) = vector(value)?;
    let xpub = <&[u8; 74]>::try_from(xpub).ok()?;
    (rest.is_empty() && matches!(xpub[41], 2 | 3)).then_some((
        <[u8; 32]>::try_from(id).ok()?,
        u32::from_le_bytes(place),
        xpub,
    ))
}

struct Reader<'a> {
    c: &'a Compute,
    wallet: Wallet,
    held: HashSet<[u8; 32]>,
    /// Where each descriptor of the wallet is in its list.
    at: HashMap<[u8; 32], usize>,
    /// The ids of the kept `combo()` descriptors.
    kept_ids: HashSet<[u8; 32]>,
    /// Where each label's address is in the wallet's list.
    label_at: HashMap<Text, usize>,
    /// The cached account keys each descriptor's keys need checked.
    needs: HashMap<[u8; 32], Vec<Need>>,
    /// The places whose cached key was compared with a master key.
    compared: HashSet<([u8; 32], u32)>,
    unread: Counts,
    unknown: Counts,
    app_kind_main: bool,
}

/// Names and how many of each, in the order first met: at most
/// [`MAX_LISTED`] names of at most [`MAX_NAMED`] bytes, found by a search
/// (so a name is stored once); the records of any other name are counted
/// in `more`.
#[derive(Default)]
struct Counts {
    list: Vec<(Text, usize)>,
    more: usize,
}

impl Counts {
    fn bump(&mut self, name: &str) {
        if let Some(entry) = self.list.iter_mut().find(|(n, _)| n == name) {
            entry.1 += 1;
        } else if self.list.len() < MAX_LISTED && name.len() <= MAX_NAMED {
            self.list.push((Text::of(name), 1));
        } else {
            self.more += 1;
        }
    }
}

impl Reader<'_> {
    fn fail(&mut self, kind: &str, tail: &[u8], fault: Fault) {
        if self.wallet.failures.len() >= MAX_LISTED {
            self.wallet.failures_more += 1;
            return;
        }
        let shown = &tail[..tail.len().min(MAX_TAIL)];
        let more = if shown.len() < tail.len() { "..." } else { "" };
        let mut record = text!("{kind} {}{more}", &*hex(shown));
        let kept = record.trim_end().len();
        record.truncate(kept);
        self.wallet.failures.push(Failure { record, fault });
    }
}

/// SQLite's first 16 bytes.
const SQLITE_MAGIC: &[u8] = b"SQLite format 3\0";

/// Whether a file starts as a SQLite database does.
pub fn is_sqlite(file: &[u8]) -> bool {
    file.starts_with(SQLITE_MAGIC)
}

/// Whether a file is a Berkeley DB database of the B-tree kind a legacy
/// wallet is: its magic number, 0x00053162, at byte 12 in either byte order.
pub fn is_berkeley_db(file: &[u8]) -> bool {
    file.get(12..16)
        .is_some_and(|m| m == [0x62, 0x31, 0x05, 0x00] || m == [0x00, 0x05, 0x31, 0x62])
}

/// Whether a file is one this area reads or names a refusal for: a SQLite
/// database or a Berkeley DB one.
pub fn is_wallet_file(file: &[u8]) -> bool {
    is_sqlite(file) || is_berkeley_db(file)
}

/// Costs what [`read`] costs, after reading the file's pages. The wallet
/// in `file`, the bytes of a `wallet.dat` named `file_name`, with
/// `beside` the names of the other files in its folder: the file is
/// refused when a journal, write-ahead log or shared memory file of its
/// name is there (it is in use or holds data not yet in it; the names are
/// compared without regard to ASCII case, as a FAT or NTFS volume does),
/// when it is not a SQLite database of the kind Bitcoin Core writes (every
/// page of it reached by a table, an index, an overflow chain or the
/// freelist), when its database version is not 0, when a table other than
/// `main` holds a record, or when `main` has more than
/// [`faraday_sqlite::MAX_ROWS`] rows; then the rows of `main` are read
/// with the header's application id. Each refusal is named
/// ([`Refusal::File`], [`Refusal::Beside`]). Memory is at most three
/// times the file (the file, its records copied once, and what the
/// descriptors, labels and keys keep of them) and some 750 MB for the
/// at most 2^20 rows, about 4 GiB at the 1 GiB limit, and on top of that
/// what parsing one descriptor's text costs, bounded by that record's
/// size (a record is at most the file); failed records and
/// unknown record types past [`MAX_LISTED`] are counted, not kept.
pub fn read_file(
    c: &Compute,
    file_name: &str,
    file: &[u8],
    beside: &[impl AsRef<str>],
    mode: Network,
    passphrase: Option<&SecretText>,
) -> Result<Wallet, Refusal> {
    if let Some(name) = faraday_sqlite::refusing_siblings(file_name)
        .into_iter()
        .find(|n| beside.iter().any(|b| b.as_ref().eq_ignore_ascii_case(n)))
    {
        return Err(Refusal::Beside(Text::of(&name)));
    }
    if is_berkeley_db(file) {
        return Err(Refusal::BerkeleyDb);
    }
    let db = faraday_sqlite::Database::open(file).map_err(Refusal::File)?;
    // Bitcoin Core's `SQLiteDatabase::Verify` refuses any other version.
    if db.header().user_version != WALLET_SCHEMA_VERSION {
        return Err(Refusal::SchemaVersion(db.header().user_version));
    }
    db.only_table_with_rows("main").map_err(Refusal::File)?;
    let rows = db.rows("main").map_err(Refusal::File)?;
    read(c, db.header().application_id, &rows, mode, passphrase)
}

/// Costs a descriptor parse for each descriptor (a second one for a descriptor
/// whose cached account key is not compared), the derivation of each cached account key along its
/// path (one for each account key) where the keys are checked, an elliptic-curve
/// multiplication and two hashes for each private key and, with the
/// passphrase, the master key's SHA-512 rounds (some hundreds of
/// thousands, at most ten million) and a decryption for each encrypted
/// key. The wallet the records state, for `application_id` (the SQLite
/// header's) and `records` (every `(key, value)` of table `main`), on a
/// mode whose network is `mode`. `passphrase` opens an encrypted wallet's
/// keys; without it they are counted, not checked.
#[expect(
    clippy::too_many_lines,
    reason = "one pass over the records, one arm for each record type Bitcoin Core writes"
)]
pub fn read<V: AsRef<[u8]>>(
    c: &Compute,
    application_id: u32,
    records: &[(Vec<u8>, V)],
    mode: Network,
    passphrase: Option<&SecretText>,
) -> Result<Wallet, Refusal> {
    let network =
        network_of_application_id(application_id).ok_or(Refusal::Application(application_id))?;
    if network.kind() != mode.kind() {
        return Err(Refusal::Network {
            offered: network,
            loaded: mode,
        });
    }
    // Each key is in Bitcoin Core's table once; a second is refused and
    // not read.
    let mut seen: HashSet<&[u8]> = HashSet::with_capacity(records.len());
    let mut repeats: Vec<&[u8]> = Vec::new();
    let fresh: Vec<(&[u8], &[u8])> = records
        .iter()
        .map(|(key, value)| (key.as_slice(), value.as_ref()))
        .filter(|(key, _)| {
            let new = seen.insert(*key);
            if !new {
                repeats.push(key);
            }
            new
        })
        .collect();
    let mut legacy = Counts::default();
    let mut flags = None;
    for (key, value) in fresh.iter().copied() {
        match split_key(key) {
            Some((name, _)) if LEGACY.contains(&name) => legacy.bump(name),
            Some(("flags", [])) => {
                flags = value.try_into().ok().map(u64::from_le_bytes);
            }
            _ => {}
        }
    }
    if !legacy.list.is_empty() {
        return Err(Refusal::Legacy(legacy.list));
    }
    let flags = flags.unwrap_or(0);
    if flags & FLAG_DESCRIPTORS == 0 {
        return Err(Refusal::NotDescriptors);
    }
    let mut r = Reader {
        c,
        wallet: Wallet {
            network,
            version: None,
            flags,
            descriptors: Vec::new(),
            kept: Vec::new(),
            active: Vec::new(),
            labels: Vec::new(),
            master_keys: Vec::new(),
            passphrase: Passphrase::NotNeeded,
            keys_passed: 0,
            keys_unchecked: 0,
            tx_count: 0,
            block_locator: None,
            unread: Vec::new(),
            unknown: Vec::new(),
            unknown_more: 0,
            failures: Vec::new(),
            failures_more: 0,
        },
        held: HashSet::new(),
        at: HashMap::new(),
        kept_ids: HashSet::new(),
        label_at: HashMap::new(),
        needs: HashMap::new(),
        compared: HashSet::new(),
        unread: Counts::default(),
        unknown: Counts::default(),
        app_kind_main: network.is_mainnet(),
    };
    if flags & !KNOWN_FLAGS != 0 {
        r.fail("flags", &[], Fault::UnknownFlags);
    }
    for key in repeats {
        match split_key(key) {
            Some((name, tail)) => r.fail(name, tail, Fault::Repeated),
            None => r.fail("", key, Fault::Repeated),
        }
    }
    // The cached account keys first, then the descriptors, which list
    // with them: the other records name the descriptors.
    let mut cache: Cache = HashMap::new();
    let mut placed: Vec<([u8; 32], u32)> = Vec::new();
    for (key, value) in fresh.iter().copied() {
        if let Some(("walletdescriptorlhcache", tail)) = split_key(key) {
            match lh_record(tail, value) {
                Some((id, place, xpub)) => {
                    cache.insert((id, place), xpub);
                    placed.push((id, place));
                }
                None => r.fail("walletdescriptorlhcache", tail, Fault::Layout),
            }
        }
    }
    for (key, value) in fresh.iter().copied() {
        if let Some(("walletdescriptor", tail)) = split_key(key) {
            r.descriptor(tail, value, &cache)?;
        }
    }
    for (id, place) in placed {
        if !r.held.contains(&id) {
            let tail = [id.as_slice(), &place.to_le_bytes()].concat(); // not text: bytes
            r.fail("walletdescriptorlhcache", &tail, Fault::NotHeld);
        }
    }
    let mut mkeys: Vec<(u32, &[u8])> = Vec::new();
    let mut plain: Vec<KeyRecord> = Vec::new();
    let mut crypted: Vec<KeyRecord> = Vec::new();
    let mut locators: Vec<(bool, usize)> = Vec::new();
    for (key, value) in fresh.iter().copied() {
        let Some((name, tail)) = split_key(key) else {
            r.fail("", key, Fault::Layout);
            continue;
        };
        match name {
            "walletdescriptor" | "walletdescriptorlhcache" => {}
            "version" => match <[u8; 4]>::try_from(value) {
                Ok(v) if tail.is_empty() => r.wallet.version = Some(u32::from_le_bytes(v)),
                _ => r.fail(name, tail, Fault::Layout),
            },
            "minversion" => {
                if tail.is_empty() && value.len() == 4 {
                    r.unread.bump(name);
                } else {
                    r.fail(name, tail, Fault::Layout);
                }
            }
            "flags" => {
                if !tail.is_empty() || value.len() != 8 {
                    r.fail(name, tail, Fault::Layout);
                }
            }
            "activeexternalspk" | "activeinternalspk" => r.active(name, tail, value),
            "name" | "purpose" => r.label(name, tail, value),
            "mkey" => match <[u8; 4]>::try_from(tail) {
                Ok(id) => mkeys.push((u32::from_le_bytes(id), value)),
                Err(_) => r.fail(name, tail, Fault::Layout),
            },
            "walletdescriptorkey" | "walletdescriptorckey" => {
                let parsed = tail.split_at_checked(32).and_then(|(id, rest)| {
                    Some((<[u8; 32]>::try_from(id).ok()?, key_pubkey(rest)?.to_vec()))
                });
                match parsed {
                    Some(_) if value.len() > MAX_KEY_VALUE => r.fail(name, tail, Fault::Layout),
                    Some((id, pubkey)) if r.held.contains(&id) => {
                        let item = (id, pubkey, Zeroizing::new(value.to_vec()), tail.to_vec());
                        if name == "walletdescriptorkey" {
                            plain.push(item);
                        } else {
                            crypted.push(item);
                        }
                    }
                    Some(_) => r.fail(name, tail, Fault::NotHeld),
                    None => r.fail(name, tail, Fault::Layout),
                }
            }
            "tx" => r.wallet.tx_count += 1,
            "bestblock" | "bestblock_nomerkle" => {
                // The version, the count and a hash for each, sized with
                // checked arithmetic: a count is any 64-bit number.
                let hashes = value.get(4..).and_then(compact_size).and_then(|(n, used)| {
                    let n = usize::try_from(n).ok()?;
                    let size = n.checked_mul(32)?.checked_add(4)?.checked_add(used)?;
                    (value.len() == size).then_some(n)
                });
                match hashes {
                    Some(n) if tail.is_empty() => {
                        locators.push((name == "bestblock_nomerkle", n));
                    }
                    _ => r.fail(name, tail, Fault::Layout),
                }
            }
            n if UNREAD.contains(&n) => r.unread.bump(n),
            other => r.unknown.bump(other),
        }
    }
    locators.sort_by_key(|(nomerkle, _)| !nomerkle);
    r.wallet.block_locator = locators.first().map(|(_, n)| *n);
    mkeys.sort_by_key(|(id, _)| *id);
    // Bitcoin Core writes one `mkey`; the first by id is the one tried.
    let mut openable = None;
    for (i, (id, value)) in mkeys.iter().enumerate() {
        let tail = id.to_le_bytes();
        match master_info(*id, value) {
            None => r.fail("mkey", &tail, Fault::Layout),
            Some(m) => {
                r.wallet.master_keys.push(m);
                if m.method != METHOD_SHA512_AES {
                    r.fail("mkey", &tail, Fault::MasterMethod);
                } else if m.rounds == 0 || m.rounds > MAX_ROUNDS {
                    r.fail("mkey", &tail, Fault::MasterRounds);
                } else if i == 0 {
                    openable = Some(*value);
                }
            }
        }
        if i > 0 {
            r.fail("mkey", &tail, Fault::ManyMasterKeys);
        }
    }
    if mkeys.is_empty() {
        r.private_keys(&plain);
    } else {
        for (_, _, _, tail) in &plain {
            r.fail("walletdescriptorkey", tail, Fault::PlainKeyEncrypted);
        }
    }
    r.encrypted_keys(!mkeys.is_empty(), openable, &crypted, passphrase);
    r.uncompared();
    r.wallet.unread = r.unread.list;
    r.wallet.unknown = r.unknown.list;
    r.wallet.unknown_more = r.unknown.more;
    Ok(r.wallet)
}

/// An `mkey` record's method and rounds, where its value reads.
fn master_info(id: u32, value: &[u8]) -> Option<MasterRecord> {
    let (crypted, rest) = vector(value)?;
    let (salt, rest) = vector(rest)?;
    let (method, rest) = rest.split_first_chunk::<4>()?;
    let (rounds, rest) = rest.split_first_chunk::<4>()?;
    let (_, rest) = vector(rest)?;
    (crypted.len() == 48 && salt.len() == 8 && rest.is_empty()).then(|| MasterRecord {
        id,
        method: u32::from_le_bytes(*method),
        rounds: u32::from_le_bytes(*rounds),
    })
}

/// Bitcoin Core's limit on the size of a descriptor's range
/// (`CheckDescriptorRangeBounds`, src/script/descriptor.cpp).
const RANGE_LIMIT: i64 = 1_000_000;

/// Whether a record's next index and range are ones Faraday lists. Core's
/// `CheckDescriptorRangeBounds` refuses a range whose inclusive high minus
/// its low is `RANGE_LIMIT` or more ("Range is too large"); the stored end is
/// one past the high, so that is `(end - 1) - start`. The limit is on the
/// size, so a range that starts high and is small is read whole. The next
/// index is held to the same limit (the owner's call). A descriptor with no
/// `*` that stores a range of more than one index is refused by the caller:
/// Core calls such a file "Wallet corrupted" (SetCache derives the same
/// script twice); one index or none is ignored and reset by Core, and
/// Faraday lists no range for it.
fn stored_range_ok(next_index: i32, start: i32, end: i32) -> bool {
    next_index >= 0
        && start >= 0
        && end >= start
        && i64::from(next_index) < RANGE_LIMIT
        && i64::from(end) - 1 - i64::from(start) < RANGE_LIMIT
}

impl Reader<'_> {
    fn descriptor(&mut self, tail: &[u8], value: &[u8], cache: &Cache) -> Result<(), Refusal> {
        let name = "walletdescriptor";
        let Ok(id) = <[u8; 32]>::try_from(tail) else {
            self.fail(name, tail, Fault::Layout);
            return Ok(());
        };
        let parsed = string(value).and_then(|(text, rest)| {
            let (time, rest) = rest.split_first_chunk::<8>()?;
            let (next, rest) = rest.split_first_chunk::<4>()?;
            let (start, rest) = rest.split_first_chunk::<4>()?;
            let (end, rest) = rest.split_first_chunk::<4>()?;
            rest.is_empty().then(|| {
                (
                    text,
                    u64::from_le_bytes(*time),
                    i32::from_le_bytes(*next),
                    i32::from_le_bytes(*start),
                    i32::from_le_bytes(*end),
                )
            })
        });
        let Some((text, creation_time, next_index, range_start, range_end)) = parsed else {
            self.fail(name, tail, Fault::Layout);
            return Ok(());
        };
        if !self.held.insert(id) {
            self.fail(name, tail, Fault::Repeated);
            return Ok(());
        }
        if !verify_checksum(text) {
            self.fail(name, tail, Fault::Checksum);
            return Ok(());
        }
        let combo = combo_twin(text);
        let Some(facts) = key_facts(self.c, combo.as_deref().unwrap_or(text)) else {
            self.fail(name, tail, Fault::Descriptor);
            return Ok(());
        };
        if facts.without_origin > 0 {
            return Err(Refusal::NoOrigin(text!("{name} {}", &*hex(tail))));
        }
        if (facts.main > 0 && !self.app_kind_main) || (facts.test > 0 && self.app_kind_main) {
            self.fail(name, tail, Fault::KeyNetwork);
            return Ok(());
        }
        if descriptor_id(text) != Some(id) {
            self.fail(name, tail, Fault::Id);
            return Ok(());
        }
        if !stored_range_ok(next_index, range_start, range_end) {
            self.fail(name, tail, Fault::Range);
            return Ok(());
        }
        let not_ranged = |listed: &str| !listed.contains('*') && range_end - range_start > 1;
        if combo.is_some() {
            if not_ranged(text) {
                self.fail(name, tail, Fault::Range);
                return Ok(());
            }
            self.kept_ids.insert(id);
            self.wallet.kept.push(Kept {
                id,
                text: Text::of(text),
            });
            return Ok(());
        }
        let body = text.split_once('#').map_or(text, |(body, _)| body);
        let lookup = |place: u32| cache.get(&(id, place)).map(|x| **x);
        let (listed, needs) = match normal::normalize(body, &lookup) {
            Ok(done) => done,
            Err(Failed::Missing(_)) => {
                self.fail(name, tail, Fault::CacheMissing);
                return Ok(());
            }
            Err(Failed::Key) => {
                self.fail(name, tail, Fault::CacheKey);
                return Ok(());
            }
            Err(Failed::Unreadable) => {
                self.fail(name, tail, Fault::Descriptor);
                return Ok(());
            }
        };
        if not_ranged(&listed) {
            self.fail(name, tail, Fault::Range);
            return Ok(());
        }
        let range = listed.contains('*').then_some((range_start, range_end - 1));
        self.needs.insert(id, needs);
        self.at.insert(id, self.wallet.descriptors.len());
        self.wallet.descriptors.push(Descriptor {
            id,
            text: Text::of(text),
            listed,
            creation_time,
            next_index,
            range,
            private_keys: 0,
            encrypted_keys: 0,
        });
        Ok(())
    }

    fn active(&mut self, name: &str, tail: &[u8], value: &[u8]) {
        let output_type = match tail {
            [0] => OutputType::Legacy,
            [1] => OutputType::P2shSegwit,
            [2] => OutputType::Bech32,
            [3] => OutputType::Bech32m,
            _ => return self.fail(name, tail, Fault::Layout),
        };
        let Ok(id) = <[u8; 32]>::try_from(value) else {
            return self.fail(name, tail, Fault::Layout);
        };
        let internal = name == "activeinternalspk";
        if self
            .wallet
            .active
            .iter()
            .any(|a| a.internal == internal && a.output_type == output_type)
        {
            return self.fail(name, tail, Fault::Repeated);
        }
        if !self.held.contains(&id) || self.kept_ids.contains(&id) {
            return self.fail(name, tail, Fault::NotHeld);
        }
        self.wallet.active.push(Active {
            internal,
            output_type,
            id,
        });
    }

    fn label(&mut self, name: &str, tail: &[u8], value: &[u8]) {
        let (Some(address), Some(text)) = (whole_string(tail), whole_string(value)) else {
            return self.fail(name, tail, Fault::Layout);
        };
        if address.len() > MAX_ADDRESS {
            return self.fail(name, tail, Fault::Layout);
        }
        let at = match self.label_at.get(address) {
            Some(&at) => at,
            None => {
                let at = self.wallet.labels.len();
                self.label_at.insert(Text::of(address), at);
                self.wallet.labels.push(Label {
                    address: Text::of(address),
                    name: None,
                    purpose: None,
                });
                at
            }
        };
        let label = &mut self.wallet.labels[at];
        let slot = if name == "name" {
            &mut label.name
        } else {
            &mut label.purpose
        };
        if slot.is_some() {
            return self.fail(name, tail, Fault::Repeated);
        }
        *slot = Some(Text::of(text));
    }

    /// The cached account keys of descriptor `id` that this master key
    /// (`pubkey`) is the root of.
    fn accounts(&self, id: &[u8; 32], pubkey: &[u8]) -> Accounts {
        let Some(needs) = self.needs.get(id) else {
            return Accounts::default();
        };
        let mut found = Accounts {
            places: Vec::with_capacity(needs.len()),
            checks: Vec::with_capacity(needs.len()),
        };
        for n in needs.iter().filter(|n| n.root[..] == *pubkey) {
            found.places.push(n.index);
            found.checks.push(AccountCheck {
                meta: n.meta,
                chain_code: *n.chain_code,
                path: n.path.clone(),
                cached: n.cached,
            });
        }
        found
    }

    /// The cached keys no master key was compared with. Where the keys
    /// were checked such a record fails ([`Fault::CacheUnchecked`]),
    /// whatever the wallet's flags say. Only an encrypted wallet read without
    /// its passphrase takes the cached key unchecked, as a descriptor file's
    /// is read: a cached key that is not a point on the curve fails its
    /// descriptor ([`Fault::CacheKey`]).
    fn uncompared(&mut self) {
        let checked = self.wallet.passphrase == Passphrase::Opened
            || (!self.wallet.is_encrypted() && self.wallet.passphrase == Passphrase::NotNeeded);
        let mut ids: Vec<[u8; 32]> = self.needs.keys().copied().collect();
        ids.sort();
        for id in ids {
            let missed: Vec<u32> = self.needs[&id]
                .iter()
                .map(|n| n.index)
                .filter(|i| !self.compared.contains(&(id, *i)))
                .collect();
            if missed.is_empty() {
                continue;
            }
            if checked {
                for place in missed {
                    let tail = [id.as_slice(), &place.to_le_bytes()].concat(); // not text: bytes
                    self.fail("walletdescriptorlhcache", &tail, Fault::CacheUnchecked);
                }
            } else if let Some(&at) = self.at.get(&id) {
                let listed = self.wallet.descriptors[at].listed.clone();
                if key_facts(self.c, &listed).is_none() {
                    self.fail("walletdescriptor", &id, Fault::CacheKey);
                }
            }
        }
    }

    /// Fails each cached key of `id` that `matches` says is not the one
    /// its master key gives.
    fn cached_keys(&mut self, id: &[u8; 32], places: &[u32], matches: &[bool]) {
        for (place, ok) in places.iter().zip(matches) {
            self.compared.insert((*id, *place));
            if !ok {
                let tail = [id.as_slice(), &place.to_le_bytes()].concat(); // not text: bytes
                self.fail("walletdescriptorlhcache", &tail, Fault::CacheKey);
            }
        }
    }

    fn verdict(&mut self, name: &str, tail: &[u8], id: &[u8; 32], v: KeyVerdict, crypted: bool) {
        let fault = match v {
            KeyVerdict::Pass => {
                self.wallet.keys_passed += 1;
                if !crypted && let Some(&at) = self.at.get(id) {
                    self.wallet.descriptors[at].private_keys += 1;
                }
                return;
            }
            KeyVerdict::Malformed => Fault::KeyMalformed,
            KeyVerdict::HashDiffers => Fault::KeyHash,
            KeyVerdict::PublicDiffers => Fault::KeyPublic,
        };
        self.fail(name, tail, fault);
    }

    fn private_keys(&mut self, keys: &[KeyRecord]) {
        for (id, pubkey, value, tail) in keys {
            let found = self.accounts(id, pubkey);
            let (v, matches) =
                check_private_key_accounts(self.c, pubkey, value.clone(), &found.checks);
            self.verdict("walletdescriptorkey", tail, id, v, false);
            self.cached_keys(id, &found.places, &matches);
        }
    }

    /// `encrypted` is whether the file holds an `mkey` record; `mkey` is
    /// its value where it can be opened (the others were refused).
    fn encrypted_keys(
        &mut self,
        encrypted: bool,
        mkey: Option<&[u8]>,
        keys: &[KeyRecord],
        passphrase: Option<&SecretText>,
    ) {
        let name = "walletdescriptorckey";
        if !encrypted {
            for (_, _, _, tail) in keys {
                self.fail(name, tail, Fault::NoMasterKey);
            }
            return;
        }
        self.wallet.passphrase = Passphrase::NotGiven;
        self.wallet.keys_unchecked = keys.len();
        for (id, _, _, _) in keys {
            if let Some(&at) = self.at.get(id) {
                self.wallet.descriptors[at].encrypted_keys += 1;
            }
        }
        let (Some(passphrase), Some(mkey)) = (passphrase, mkey) else {
            return;
        };
        let found: Vec<Accounts> = keys
            .iter()
            .map(|(id, pubkey, _, _)| self.accounts(id, pubkey))
            .collect();
        let list = keys.iter().zip(&found).map(|((_, pubkey, value, _), f)| {
            (pubkey.as_slice(), value.as_slice(), f.checks.as_slice())
        });
        match check_encrypted_keys_accounts(self.c, mkey, passphrase, list) {
            Ok(verdicts) => {
                self.wallet.passphrase = Passphrase::Opened;
                self.wallet.keys_unchecked = 0;
                for (((id, _, _, tail), (v, matches)), f) in keys.iter().zip(verdicts).zip(&found) {
                    self.verdict(name, tail, id, v, true);
                    self.cached_keys(id, &f.places, &matches);
                }
            }
            Err(OpenError::WrongPassphrase) => self.wallet.passphrase = Passphrase::Wrong,
            // The record was read and its method and rounds checked above.
            Err(_) => self.fail("mkey", &[], Fault::Layout),
        }
    }
}
