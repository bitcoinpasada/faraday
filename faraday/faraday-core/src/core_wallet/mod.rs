//! A Bitcoin Core descriptor wallet's records, read and checked
//! (`docs/decisions/F229.md`): the `(key, value)` pairs of the table
//! `main` of a `wallet.dat` and the application id its header carries,
//! read into what the wallet holds. [`read_file`] takes the file's bytes
//! (read by `faraday-sqlite`) and does the whole; nothing is written.
//!
//! [`read`] gives the network (the application id is its message start),
//! the version, the flags, each descriptor with its id, range and next
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
    KeyVerdict, MAX_ROUNDS, METHOD_SHA512_AES, OpenError, check_encrypted_keys, check_private_key,
};
use faraday_strings::{EN, fill};
use std::collections::{HashMap, HashSet};
use zeroize::Zeroizing;

use crate::wallet::network_name;

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
const UNREAD: [&str; 6] = [
    "orderposnext",
    "walletdescriptorcache",
    "walletdescriptorlhcache",
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
    Legacy(Vec<(String, usize)>),
    /// The flags (none written counts as none) lack descriptors.
    NotDescriptors,
    /// A descriptor with a key that carries no origin: the record.
    NoOrigin(String),
    /// The file is not a database Bitcoin Core wrote: the SQLite reader's
    /// refusal, named.
    File(faraday_sqlite::Error),
    /// A lock, a journal or a write-ahead log beside the file: its name.
    Beside(String),
    /// A database version (`PRAGMA user_version`) other than the 0
    /// Bitcoin Core writes: the version.
    SchemaVersion(i32),
}

impl Refusal {
    /// The refusal as text.
    pub fn message(&self) -> String {
        match self {
            Refusal::Application(id) => fill(EN.core_wallet_application, &[&format!("{id:08x}")]),
            Refusal::Network { offered, loaded } => fill(
                EN.wallet_is_not_the_network,
                &[network_name(*offered), network_name(*loaded)],
            ),
            Refusal::Legacy(found) => {
                let names: Vec<String> = found.iter().map(|(n, k)| format!("{n} {k}")).collect();
                fill(EN.core_wallet_legacy, &[&names.join(", ")])
            }
            Refusal::NotDescriptors => EN.core_wallet_not_descriptors.to_string(),
            Refusal::NoOrigin(record) => fill(EN.core_wallet_no_origin, &[record]),
            Refusal::File(faraday_sqlite::Error::NotSqlite) => {
                EN.core_wallet_not_sqlite.to_string()
            }
            Refusal::File(faraday_sqlite::Error::Wal) => EN.core_wallet_wal.to_string(),
            Refusal::File(faraday_sqlite::Error::OtherTable(name)) => {
                fill(EN.core_wallet_other_table, &[name])
            }
            Refusal::File(faraday_sqlite::Error::NoTable(name)) => {
                fill(EN.core_wallet_no_table, &[name])
            }
            Refusal::File(faraday_sqlite::Error::TooManyRows) => fill(
                EN.core_wallet_too_many_rows,
                &[&faraday_sqlite::MAX_ROWS.to_string()],
            ),
            Refusal::File(_) => EN.core_wallet_file_damaged.to_string(),
            Refusal::Beside(name) => fill(EN.core_wallet_beside, &[name]),
            Refusal::SchemaVersion(v) => fill(EN.core_wallet_schema_version, &[&v.to_string()]),
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
    /// The range or next index is negative or the range is inverted.
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
}

/// A failed record and what failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Failure {
    /// The record's type and its key's bytes in hex.
    pub record: String,
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
    pub text: String,
    /// Unix time the descriptor was made.
    pub creation_time: i64,
    /// The next index to hand out, as written.
    pub next_index: i32,
    /// The first index derived, as written.
    pub range_start: i32,
    /// One past the last index derived, as written (`listdescriptors`
    /// gives the last, one less).
    pub range_end: i32,
    /// Private keys of it that are not encrypted and passed.
    pub private_keys: usize,
    /// Encrypted private keys of it; checked when the passphrase is
    /// given ([`Wallet::passphrase`]).
    pub encrypted_keys: usize,
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
    pub address: String,
    /// Its `name` record.
    pub name: Option<String>,
    /// Its `purpose` record.
    pub purpose: Option<String>,
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
    pub unread: Vec<(String, usize)>,
    /// Record types Faraday does not know, by name and count: the first
    /// [`MAX_LISTED`] of at most 64 bytes each.
    pub unknown: Vec<(String, usize)>,
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

fn hex(bytes: &[u8]) -> String {
    faraday_crypto::hex::encode_string(bytes)
}

/// The id a descriptor's text has: SHA-256 of the text with every hardened
/// mark in its key paths written `'` and the checksum over that
/// (`DescriptorID` in Bitcoin Core: `ToString` in its compatible form).
/// `None` for a text with no `#` checksum part or characters no
/// descriptor holds.
fn descriptor_id(text: &str) -> Option<[u8; 32]> {
    let (body, _) = text.split_once('#')?;
    let mut compat = String::with_capacity(body.len() + 9);
    let mut token = String::new();
    for ch in body.chars() {
        if matches!(ch, '(' | ')' | ',' | '{' | '}') {
            compat.push_str(&apostrophes(&token));
            token.clear();
            compat.push(ch);
        } else {
            token.push(ch);
        }
    }
    compat.push_str(&apostrophes(&token));
    let sum = descriptor_checksum(&compat)?;
    compat.push('#');
    compat.push_str(core::str::from_utf8(&sum).ok()?);
    Some(sha256(compat.as_bytes()))
}

/// One key expression with its paths' hardened marks as `'`: the
/// origin's after its fingerprint, and the path after the key.
fn apostrophes(token: &str) -> String {
    let mark = |path: &str| path.replace(['h', 'H'], "'");
    let (origin, key) = match token.strip_prefix('[').and_then(|t| t.split_once(']')) {
        Some((inside, key)) => (Some(inside), key),
        None => (None, token),
    };
    let mut out = String::with_capacity(token.len() + 4);
    if let Some(inside) = origin {
        out.push('[');
        match inside.split_once('/') {
            Some((fingerprint, path)) => {
                out.push_str(fingerprint);
                out.push('/');
                out.push_str(&mark(path));
            }
            None => out.push_str(inside),
        }
        out.push(']');
    }
    match key.split_once('/') {
        Some((head, path)) => {
            out.push_str(head);
            out.push('/');
            out.push_str(&mark(path));
        }
        None => out.push_str(key),
    }
    out
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

struct Reader<'a> {
    c: &'a Compute,
    wallet: Wallet,
    held: HashSet<[u8; 32]>,
    /// Where each descriptor of the wallet is in its list.
    at: HashMap<[u8; 32], usize>,
    /// Where each label's address is in the wallet's list.
    label_at: HashMap<String, usize>,
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
    list: Vec<(String, usize)>,
    more: usize,
}

impl Counts {
    fn bump(&mut self, name: &str) {
        if let Some(entry) = self.list.iter_mut().find(|(n, _)| n == name) {
            entry.1 += 1;
        } else if self.list.len() < MAX_LISTED && name.len() <= MAX_NAMED {
            self.list.push((name.to_string(), 1));
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
        self.wallet.failures.push(Failure {
            record: format!("{kind} {}{more}", hex(shown))
                .trim_end()
                .to_string(),
            fault,
        });
    }
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
    beside: &[String],
    mode: Network,
    passphrase: Option<&SecretText>,
) -> Result<Wallet, Refusal> {
    if let Some(name) = faraday_sqlite::refusing_siblings(file_name)
        .into_iter()
        .find(|n| beside.iter().any(|b| b.eq_ignore_ascii_case(n)))
    {
        return Err(Refusal::Beside(name));
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

/// Costs a descriptor parse for each descriptor, an elliptic-curve
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
pub fn read(
    c: &Compute,
    application_id: u32,
    records: &[(Vec<u8>, Vec<u8>)],
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
    let fresh: Vec<&(Vec<u8>, Vec<u8>)> = records
        .iter()
        .filter(|(key, _)| {
            let new = seen.insert(key.as_slice());
            if !new {
                repeats.push(key.as_slice());
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
                flags = value.as_slice().try_into().ok().map(u64::from_le_bytes);
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
        label_at: HashMap::new(),
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
    // Descriptors first: the other records name them.
    for (key, value) in fresh.iter().copied() {
        if let Some(("walletdescriptor", tail)) = split_key(key) {
            r.descriptor(tail, value)?;
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
            "walletdescriptor" => {}
            "version" => match <[u8; 4]>::try_from(value.as_slice()) {
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
                Ok(id) => mkeys.push((u32::from_le_bytes(id), value.as_slice())),
                Err(_) => r.fail(name, tail, Fault::Layout),
            },
            "walletdescriptorkey" | "walletdescriptorckey" => {
                let parsed = tail.split_at_checked(32).and_then(|(id, rest)| {
                    Some((<[u8; 32]>::try_from(id).ok()?, key_pubkey(rest)?.to_vec()))
                });
                match parsed {
                    Some(_) if value.len() > MAX_KEY_VALUE => r.fail(name, tail, Fault::Layout),
                    Some((id, pubkey)) if r.held.contains(&id) => {
                        let item = (id, pubkey, Zeroizing::new(value.clone()), tail.to_vec());
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

impl Reader<'_> {
    fn descriptor(&mut self, tail: &[u8], value: &[u8]) -> Result<(), Refusal> {
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
                    i64::from_le_bytes(*time),
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
        let Some(facts) = key_facts(self.c, text) else {
            self.fail(name, tail, Fault::Descriptor);
            return Ok(());
        };
        if facts.without_origin > 0 {
            return Err(Refusal::NoOrigin(format!("{name} {}", hex(tail))));
        }
        if (facts.main > 0 && !self.app_kind_main) || (facts.test > 0 && self.app_kind_main) {
            self.fail(name, tail, Fault::KeyNetwork);
            return Ok(());
        }
        if descriptor_id(text) != Some(id) {
            self.fail(name, tail, Fault::Id);
            return Ok(());
        }
        if next_index < 0 || range_start < 0 || range_end < range_start {
            self.fail(name, tail, Fault::Range);
            return Ok(());
        }
        self.at.insert(id, self.wallet.descriptors.len());
        self.wallet.descriptors.push(Descriptor {
            id,
            text: text.to_string(),
            creation_time,
            next_index,
            range_start,
            range_end,
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
        if !self.held.contains(&id) {
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
                self.label_at.insert(address.to_string(), at);
                self.wallet.labels.push(Label {
                    address: address.to_string(),
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
        *slot = Some(text.to_string());
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
            let v = check_private_key(self.c, pubkey, value.clone());
            self.verdict("walletdescriptorkey", tail, id, v, false);
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
        let list = keys
            .iter()
            .map(|(_, pubkey, value, _)| (pubkey.as_slice(), value.as_slice()));
        match check_encrypted_keys(self.c, mkey, passphrase, list) {
            Ok(verdicts) => {
                self.wallet.passphrase = Passphrase::Opened;
                self.wallet.keys_unchecked = 0;
                for ((id, _, _, tail), v) in keys.iter().zip(verdicts) {
                    self.verdict(name, tail, id, v, true);
                }
            }
            Err(OpenError::WrongPassphrase) => self.wallet.passphrase = Passphrase::Wrong,
            // The record was read and its method and rounds checked above.
            Err(_) => self.fail("mkey", &[], Fault::Layout),
        }
    }
}
