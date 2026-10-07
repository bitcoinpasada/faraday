//! Vanity address grinding: the counter that turns a key's own dials
//! until its first address begins with the characters a person chose
//! (`docs/PLANNING.md` §16.117, `docs/FEATURES.md` §8.1 row 19).
//!
//! Two dials, and nothing else:
//!
//! - **The passphrase.** The candidate is the key's own BIP-39
//!   passphrase with a base-62 counter appended, and the address is
//!   derived the standard way: PBKDF2-HMAC-SHA-512 over the words, the
//!   BIP-32 master, `m/purpose'/coin'/0'/0/0`. One candidate costs one
//!   PBKDF2 (2048 rounds), which is what makes this dial slow.
//! - **The account index.** The passphrase is the key's own, and the
//!   counter is the account: `m/purpose'/coin'/counter'/0/0`. One
//!   candidate costs one hardened derivation.
//!
//! Nothing here is random. The same key and the same counter give the
//! same address on any device, so a find is a fact about the key rather
//! than a secret this device made, and a person who knows the words and
//! the counter can reproduce it anywhere.
//!
//! **EntropyLab's order, so a find agrees.** The counter is written in
//! the alphabet and the odometer order EntropyLab's `vanity-wasm` uses
//! (`tools/reference/vanity/README.md`): the digits are
//! `a`–`z`, `A`–`Z`, `0`–`9`, the leftmost is the most significant, and
//! `a` is zero. EntropyLab grinds one width at a time, at the length its
//! screen asks for; this device has no such question, so it grinds width
//! 1 first, then width 2, and so on. Inside a width the two orders are
//! the same counter, which is what makes a find here and a find there
//! the same find.
//!
//! The address is compared on its characters after the fixed prefix its
//! script type and network give every address of that kind — `bc1q`,
//! `bc1p`, `1`, `3` on mainnet — case-insensitively for bech32, which
//! has one case, and exactly for base58, which has two.

use bitcoin::bip32::{ChildNumber, DerivationPath, Xpriv};
use bitcoin::secp256k1::{All, PublicKey, Secp256k1, SecretKey};
use bitcoin::{Address, CompressedPublicKey, XOnlyPublicKey};

use crate::bip39::Mnemonic;
use crate::keys::{MasterKey, Network, ScriptType};

/// The counter's digits, in EntropyLab's order: `a` is zero, `9` is 61.
pub const ALPHABET: &[u8; 62] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";

/// Bech32's character set, which has no `1`, `b`, `i` or `o`.
pub const BECH32: &str = "qpzry9x8gf2tvdw0s3jn54khce6mua7l";

/// Base58's character set, which has no `0`, `O`, `I` or `l`.
pub const BASE58: &str = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";

/// The longest counter this grinder writes. 62⁸ is 2.2 × 10¹⁴
/// candidates, past any prefix a person will wait for.
pub const MAX_COUNTER_CHARS: usize = 8;

/// The longest address any script type writes: a Taproot address is 62
/// characters on mainnet and 64 on regtest.
pub const MAX_ADDRESS_CHARS: usize = 68;

/// A counter written in [`ALPHABET`], in a fixed buffer that zeroizes on
/// drop. It is part of a passphrase, so it is as secret as the key.
pub struct Counter {
    buf: [u8; MAX_COUNTER_CHARS],
    len: u8,
}

impl Counter {
    /// The empty counter, which is what the account dial carries.
    pub fn empty() -> Self {
        Counter {
            buf: [0; MAX_COUNTER_CHARS],
            len: 0,
        }
    }

    /// The text.
    pub fn as_str(&self) -> &str {
        core::str::from_utf8(self.as_bytes()).expect("the alphabet is ascii")
    }

    /// The bytes, which are what a passphrase is built from.
    pub fn as_bytes(&self) -> &[u8] {
        &self.buf[..usize::from(self.len)]
    }
}

impl Drop for Counter {
    fn drop(&mut self) {
        use osk_crypto::Zeroize;
        self.buf.zeroize();
        self.len = 0;
    }
}

/// An address in text, in a fixed buffer. An address is public.
pub struct AddressText {
    buf: [u8; MAX_ADDRESS_CHARS],
    len: u8,
}

impl AddressText {
    fn new() -> Self {
        AddressText {
            buf: [0; MAX_ADDRESS_CHARS],
            len: 0,
        }
    }

    /// The text.
    pub fn as_str(&self) -> &str {
        core::str::from_utf8(&self.buf[..usize::from(self.len)]).expect("an address is ascii")
    }
}

impl core::fmt::Write for AddressText {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        let n = usize::from(self.len);
        let bytes = s.as_bytes();
        if n + bytes.len() > MAX_ADDRESS_CHARS {
            return Err(core::fmt::Error);
        }
        self.buf[n..n + bytes.len()].copy_from_slice(bytes);
        self.len += bytes.len() as u8;
        Ok(())
    }
}

/// What the grinder starts from: the words, which the passphrase dial
/// needs, or the master key, which the account dial turns.
pub enum Key<'a> {
    /// The key's words, for the passphrase dial.
    Words(&'a Mnemonic),
    /// The key's master, for the account dial.
    Master(&'a MasterKey),
}

/// Which dial the counter turns.
pub enum Method<'a> {
    /// The counter is appended to `base`, the key's own passphrase, and
    /// the account stays 0.
    Passphrase {
        /// The passphrase the key already carries, which every
        /// candidate begins with. Empty for a key with no passphrase.
        base: &'a [u8],
    },
    /// The counter is the account index, and the passphrase is the
    /// key's own. The purpose is the script type's, so that the account
    /// a find names is the account that script type's wallet uses.
    Account,
}

/// What one candidate was, once it matched.
pub struct Find {
    /// Where the candidate sat in the grinder's own order.
    pub counter: u64,
    /// The characters appended to the passphrase; empty for the account
    /// dial.
    pub suffix: Counter,
    /// The account the address was derived at; 0 for the passphrase
    /// dial.
    pub account: u32,
    /// The address itself.
    pub address: AddressText,
}

/// What one call did: how many candidates it tested, and the find that
/// stopped it.
pub struct Outcome {
    /// Candidates tested, which is the budget unless a find stopped the
    /// run or the counter space ran out.
    pub tested: u64,
    /// The first candidate whose address matched.
    pub find: Option<Find>,
}

impl Outcome {
    fn none() -> Self {
        Outcome {
            tested: 0,
            find: None,
        }
    }
}

/// The characters every address of `script` on `network` begins with.
///
/// A legacy address on a test network begins with `m` or `n`, so it has
/// no fixed prefix at all; [`first_free`] says which characters can
/// stand there.
pub fn fixed_prefix(script: ScriptType, network: Network) -> &'static str {
    let mainnet = network.is_mainnet();
    match (script, network) {
        (ScriptType::Legacy, Network::Mainnet) => "1",
        (ScriptType::Legacy, _) => "",
        (ScriptType::NestedSegwit, Network::Mainnet) => "3",
        (ScriptType::NestedSegwit, _) => "2",
        (ScriptType::NativeSegwit, Network::Regtest) => "bcrt1q",
        (ScriptType::NativeSegwit, _) if mainnet => "bc1q",
        (ScriptType::NativeSegwit, _) => "tb1q",
        (ScriptType::Taproot, Network::Regtest) => "bcrt1p",
        (ScriptType::Taproot, _) if mainnet => "bc1p",
        (ScriptType::Taproot, _) => "tb1p",
    }
}

/// The characters that can stand in the first place a person chooses,
/// where that place is not the whole alphabet's. A legacy address on a
/// test network is the one case: its version byte leaves `m` and `n`.
pub fn first_free(script: ScriptType, network: Network) -> Option<&'static str> {
    (script == ScriptType::Legacy && !network.is_mainnet()).then_some("mn")
}

/// Whether `script` writes bech32 addresses.
pub fn is_bech32(script: ScriptType) -> bool {
    matches!(script, ScriptType::NativeSegwit | ScriptType::Taproot)
}

/// The characters an address of this kind is written in after its fixed
/// prefix.
pub fn charset(script: ScriptType) -> &'static str {
    if is_bech32(script) { BECH32 } else { BASE58 }
}

/// Whether `prefix` — the whole prefix, fixed part and all — can begin
/// an address of `script` on `network`.
///
/// This is what refuses a character at entry: bech32 has no `1`, `b`,
/// `i` or `o` after the separator, and base58 has no `0`, `O`, `I` or
/// `l` anywhere.
pub fn can_begin(prefix: &str, script: ScriptType, network: Network) -> bool {
    let fixed = fixed_prefix(script, network);
    let common = fixed.len().min(prefix.len());
    if !prefix.as_bytes()[..common].eq_ignore_ascii_case(&fixed.as_bytes()[..common]) {
        return false;
    }
    let free = &prefix[common..];
    if free.is_empty() {
        return true;
    }
    let mut chars = free.chars();
    if let Some(set) = first_free(script, network) {
        let first = chars.next().expect("the free part is not empty");
        if !set.contains(first) {
            return false;
        }
    }
    let set = charset(script);
    let free_len = free.chars().count();
    if free_len > MAX_ADDRESS_CHARS {
        return false;
    }
    chars.all(|c| {
        if is_bech32(script) {
            set.contains(c.to_ascii_lowercase())
        } else {
            set.contains(c)
        }
    })
}

/// How many candidates a prefix is expected to take: one over the
/// chance that a random address of this kind begins with it.
///
/// Each free character is one of the character set's 32 or 58, except a
/// first place that carries fewer (a legacy address on a test network).
/// The count saturates rather than overflowing: a prefix long enough to
/// pass 2⁶⁴ is a prefix no one waits for.
pub fn expected_candidates(prefix: &str, script: ScriptType, network: Network) -> u64 {
    let fixed = fixed_prefix(script, network).chars().count();
    let free = prefix.chars().count().saturating_sub(fixed);
    if free == 0 {
        return 1;
    }
    let per = if is_bech32(script) { 32u64 } else { 58u64 };
    let (mut total, rest) = match first_free(script, network) {
        Some(set) => (set.chars().count() as u64, free - 1),
        None => (1u64, free),
    };
    for _ in 0..rest {
        total = total.saturating_mul(per);
    }
    total
}

/// How long a run of `expected` candidates takes at `per_second`, in
/// seconds. `None` while no rate has been measured.
pub fn expected_seconds(expected: u64, per_second: f64) -> Option<u64> {
    if !(per_second.is_finite() && per_second > 0.0) {
        return None;
    }
    let seconds = expected as f64 / per_second;
    if seconds >= u64::MAX as f64 {
        return Some(u64::MAX);
    }
    Some(seconds as u64)
}

/// The counter at `counter` in the grinder's order: the width the
/// counter has reached, and EntropyLab's odometer inside it.
///
/// Width 1 holds the first 62 counters, width 2 the next 3844, and so
/// on. `None` once the counter passes [`MAX_COUNTER_CHARS`] characters.
pub fn counter_text(counter: u64) -> Option<Counter> {
    let mut left = counter;
    let mut width = 1usize;
    let mut span = 62u64;
    while left >= span {
        left -= span;
        width += 1;
        if width > MAX_COUNTER_CHARS {
            return None;
        }
        span = span.saturating_mul(62);
    }
    let mut out = Counter::empty();
    out.len = width as u8;
    let mut value = left;
    for i in (0..width).rev() {
        out.buf[i] = ALPHABET[(value % 62) as usize];
        value /= 62;
    }
    Some(out)
}

/// Grinds at most `budget` candidates from `from`, and stops on the
/// first address that begins with `prefix`.
///
/// `prefix` carries the fixed prefix its script type gives every
/// address, as the screen shows it. A prefix that is the fixed prefix
/// and nothing more matches the first candidate, which is what it says:
/// every address of that kind begins with it.
///
/// The caller keeps the cursor: `from + outcome.tested` is where the
/// next call starts. That is how one screen ticks through a grind
/// without a thread — the core has one, and a call that ran until it
/// found something would hold it.
///
/// A dial the key cannot turn — the passphrase dial without words, the
/// account dial without a master — tests nothing.
pub fn grind(
    key: &Key<'_>,
    method: &Method<'_>,
    script: ScriptType,
    prefix: &str,
    network: Network,
    from: u64,
    budget: u64,
) -> Outcome {
    if !can_begin(prefix, script, network) {
        return Outcome::none();
    }
    match (key, method) {
        (Key::Words(words), Method::Passphrase { base }) => {
            grind_passphrase(words, base, script, prefix, network, from, budget)
        }
        (Key::Master(master), Method::Account) => {
            grind_account(master, script, prefix, network, from, budget)
        }
        _ => Outcome::none(),
    }
}

/// The passphrase dial: one PBKDF2 per candidate.
fn grind_passphrase(
    words: &Mnemonic,
    base: &[u8],
    script: ScriptType,
    prefix: &str,
    network: Network,
    from: u64,
    budget: u64,
) -> Outcome {
    let secp = Secp256k1::new();
    let path = account_path(script, network, 0);
    let mut tested = 0;
    while tested < budget {
        let Some(suffix) = counter_text(from + tested) else {
            break;
        };
        let mut passphrase = [0u8; MAX_COUNTER_CHARS + crate::bip39::MAX_PASSPHRASE_BYTES];
        let len = base.len() + suffix.as_bytes().len();
        if len > passphrase.len() {
            break;
        }
        passphrase[..base.len()].copy_from_slice(base);
        passphrase[base.len()..len].copy_from_slice(suffix.as_bytes());
        let seed = words.to_seed_unchecked(&passphrase[..len]);
        {
            use osk_crypto::Zeroize;
            passphrase.zeroize();
        }
        let Ok(seed) = seed else { break };
        tested += 1;
        let Ok(mut master) = Xpriv::new_master(network.kind(), seed.expose()) else {
            continue;
        };
        let address = address_at(&secp, &master, &path, script, network);
        erase_xpriv(&mut master);
        let Some(address) = address else { continue };
        if matches(address.as_str(), prefix, script) {
            return Outcome {
                tested,
                find: Some(Find {
                    counter: from + tested - 1,
                    suffix,
                    account: 0,
                    address,
                }),
            };
        }
    }
    Outcome { tested, find: None }
}

/// The account dial: one hardened derivation per candidate, from the
/// `purpose'/coin'` node, which is derived once.
fn grind_account(
    master: &MasterKey,
    script: ScriptType,
    prefix: &str,
    network: Network,
    from: u64,
    budget: u64,
) -> Outcome {
    let secp = Secp256k1::new();
    let hardened = |i: u32| ChildNumber::from_hardened_idx(i).expect("below 2^31");
    let head = DerivationPath::from(
        [hardened(script.purpose()), hardened(network.coin_type())].as_slice(),
    );
    let node = master.derive(&head);
    let mut parent = Xpriv {
        network: network.kind(),
        depth: 2,
        parent_fingerprint: Default::default(),
        child_number: hardened(network.coin_type()),
        private_key: *node.secret_key(),
        chain_code: node.to_xpub().chain_code,
    };
    let mut tested = 0;
    let mut outcome = Outcome::none();
    while tested < budget {
        let account = from + tested;
        if account >= u64::from(1u32 << 31) {
            break;
        }
        let account = account as u32;
        tested += 1;
        let path = DerivationPath::from(
            [
                hardened(account),
                ChildNumber::from_normal_idx(0).expect("0 is a normal index"),
                ChildNumber::from_normal_idx(0).expect("0 is a normal index"),
            ]
            .as_slice(),
        );
        let Some(address) = address_at(&secp, &parent, &path, script, network) else {
            continue;
        };
        if matches(address.as_str(), prefix, script) {
            outcome = Outcome {
                tested,
                find: Some(Find {
                    counter: u64::from(account),
                    suffix: Counter::empty(),
                    account,
                    address,
                }),
            };
            break;
        }
    }
    erase_xpriv(&mut parent);
    if outcome.find.is_none() {
        outcome.tested = tested;
    }
    outcome
}

/// The account path `purpose'/coin'/account'` of `script` on `network`,
/// with the first receive address under it.
fn account_path(script: ScriptType, network: Network, account: u32) -> DerivationPath {
    let hardened = |i: u32| ChildNumber::from_hardened_idx(i).expect("below 2^31");
    let normal = ChildNumber::from_normal_idx(0).expect("0 is a normal index");
    DerivationPath::from(
        [
            hardened(script.purpose()),
            hardened(network.coin_type()),
            hardened(account),
            normal,
            normal,
        ]
        .as_slice(),
    )
}

/// The address `path` derives below `parent`, in text.
fn address_at(
    secp: &Secp256k1<All>,
    parent: &Xpriv,
    path: &DerivationPath,
    script: ScriptType,
    network: Network,
) -> Option<AddressText> {
    let mut child = parent.derive_priv(secp, path).ok()?;
    let address = address_of(secp, &child.private_key, script, network);
    erase_xpriv(&mut child);
    address
}

/// The address `key` pays to under `script` on `network`, in text.
fn address_of(
    secp: &Secp256k1<All>,
    key: &SecretKey,
    script: ScriptType,
    network: Network,
) -> Option<AddressText> {
    let public = PublicKey::from_secret_key(secp, key);
    let network = bitcoin::Network::from(network);
    let compressed = CompressedPublicKey(public);
    let address = match script {
        ScriptType::Legacy => Address::p2pkh(compressed, network),
        ScriptType::NestedSegwit => Address::p2shwpkh(&compressed, network),
        ScriptType::NativeSegwit => Address::p2wpkh(&compressed, network),
        ScriptType::Taproot => Address::p2tr(secp, XOnlyPublicKey::from(public), None, network),
    };
    let mut out = AddressText::new();
    core::fmt::write(&mut out, format_args!("{address}")).ok()?;
    Some(out)
}

/// Whether `address` begins with `prefix`: case-insensitively for
/// bech32, which has one case, and exactly for base58, which has two.
fn matches(address: &str, prefix: &str, script: ScriptType) -> bool {
    if address.len() < prefix.len() {
        return false;
    }
    let head = &address.as_bytes()[..prefix.len()];
    if is_bech32(script) {
        head.eq_ignore_ascii_case(prefix.as_bytes())
    } else {
        head == prefix.as_bytes()
    }
}

/// Erases an extended private key this module made a copy of. `Xpriv`
/// is `Copy`, so the copy on this frame is erased where it stands.
fn erase_xpriv(xpriv: &mut Xpriv) {
    xpriv.private_key.non_secure_erase();
    xpriv.chain_code = bitcoin::bip32::ChainCode::from([0u8; 32]);
}
