//! The Wallets tab's session: keys and wallets held in memory, and the spend of
//! one transaction from loading it to the two results (`docs/WALLETS.md`
//! §2 and §4). Every piece of cryptography is `faraday-bip`'s or
//! `faraday-psbt`'s; this module only decides which call comes next.

use crate::modes::Mode;
use crate::strings::EN;
use crate::strings::fill;
use crate::{Action, Faraday, Screen, Sheet, family, result_stem, wallet};
use faraday_bip::bip39::{Language, Mnemonic};
use faraday_bip::bitcoin::consensus::encode::serialize_hex;
use faraday_bip::bitcoin::{Transaction, Txid};
use faraday_bip::keys::{Fingerprint, Network, ScriptType};
use faraday_bip::policy::{Template, WalletPolicy, Wrapper};
use faraday_crypto::work::Compute;
use faraday_psbt::verify::Verdict;
use faraday_psbt::{Context, Inspection, KeyRef, Psbt};

/// The network a session starts on, before a wallet or a PSBT says
/// otherwise.
pub const START_NETWORK: Network = Network::Mainnet;

// The description reader (`faraday-intake`),
// at the paths this crate has always had them.
pub use faraday_intake::wallet::{
    Refusal, network_name, network_of_key, policy_network, wallet_name,
};

pub use faraday_keys::identity::master_id;
pub use faraday_keys::{WordsId, words_id};

/// Costs a parse of the description and its keys: reads a wallet
/// description with `faraday_intake`'s reader, which counts through the
/// token. Every wallet the app takes in is read here.
pub fn read_wallet(c: &Compute, text: &str) -> Result<WalletPolicy, Refusal> {
    faraday_intake::wallet::read_wallet(c, text)
}

/// Costs a word-list search per word and a hash. Parses seed words in
/// `lang`: BIP-39's parser, counted through the token.
pub fn parse_words(
    c: &Compute,
    lang: Language,
    text: &str,
) -> Result<Mnemonic, faraday_bip::bip39::Error> {
    faraday_keys::words::parse(c, lang, text)
}

pub(crate) use faraday_intake::wallet::{json_label, network_of_kind};

/// The most scripts a wallet keeps ([`Wallet::script_at`]); the next one
/// empties the list.
pub(crate) const WALLET_SCRIPTS_KEPT: usize = 256;
/// The most addresses a wallet keeps ([`Wallet::address_on`]); the next
/// one empties the list.
pub(crate) const WALLET_ADDRESSES_KEPT: usize = 256;

/// Key texts by wallet kind, asked of a key of the store: derived once
/// per key and kept with it ([`faraday_keys::Keys::public_text`]).
pub trait KeyTexts {
    /// Costs a key derivation the first time. `[fingerprint/path]xpub` of
    /// key `i` at `kind`'s path with account `account`
    /// ([`crate::create::NewKind::spec`]): derived the first
    /// time it is asked, by whoever asks, a frame included, and kept on
    /// the key, a lookup after.
    fn account_text(
        &self,
        c: &Compute,
        i: usize,
        kind: crate::create::NewKind,
        account: u32,
    ) -> Result<String, String>;

    /// Costs a key derivation the first time. `[fingerprint/path]xpub` of
    /// key `i` at `path` ([`crate::create::path_spec`]), kept as
    /// [`KeyTexts::account_text`] is.
    fn text_at(&self, c: &Compute, i: usize, path: &str) -> Result<String, String>;
}

impl KeyTexts for faraday_keys::Keys {
    fn account_text(
        &self,
        c: &Compute,
        i: usize,
        kind: crate::create::NewKind,
        account: u32,
    ) -> Result<String, String> {
        let spec = kind.spec(self.network(), account)?;
        self.key_text(c, i, &spec)
            .unwrap_or_else(|| Err(String::new()))
    }

    fn text_at(&self, c: &Compute, i: usize, path: &str) -> Result<String, String> {
        let spec = crate::create::path_spec(path)?;
        self.key_text(c, i, &spec)
            .unwrap_or_else(|| Err(String::new()))
    }
}

/// What one signing pass did.
pub struct Signed {
    /// The fingerprints that signed.
    pub by: Vec<Fingerprint>,
}

/// A wallet described by its public keys.
pub struct Wallet {
    /// What the person called it: one line of text ([`wallet_name`]),
    /// as [`Wallet::new`] and a rename make it; a sheet written for the
    /// wallet makes it so again.
    pub name: String,
    /// The policy, as it was read.
    policy: WalletPolicy,
    /// Where it came from: "Test wallet" or the file's name.
    pub source: String,
    /// What [`Wallet::source`] is: every caller that adds a wallet
    /// ([`Session::add_wallet`]) already knows this, so it is given
    /// explicitly there rather than read back out of the text, which a
    /// second language would not carry. [`source_tag`] and Family's
    /// routing (`family.rs`) read this, not the words.
    pub origin: Origin,
    /// The addresses asked of it, each derived once, by network, chain
    /// and index ([`Wallet::address_on`]): public, and dropped with the
    /// policy they come from ([`Wallet::set_policy`]).
    addresses: std::cell::RefCell<Vec<(Network, bool, u32, String)>>,
    /// The scripts asked of it to find which wallet a PSBT spends from,
    /// each derived once, by chain and index ([`Wallet::script_at`]):
    /// public, and dropped with the policy they come from.
    scripts: std::cell::RefCell<Vec<(bool, u32, Option<faraday_bip::bitcoin::ScriptBuf>)>>,
}

impl Wallet {
    /// Makes a wallet, its name made one line of text ([`wallet_name`]).
    pub fn new(name: String, policy: WalletPolicy, source: String, origin: Origin) -> Wallet {
        Wallet {
            name: wallet_name(&name).into_owned(),
            policy,
            source,
            origin,
            addresses: std::cell::RefCell::new(Vec::new()),
            scripts: std::cell::RefCell::new(Vec::new()),
        }
    }

    /// Costs a script derivation the first time: its script at
    /// `change`/`index`, `None` when it has none there, derived the first
    /// time it is asked and kept on the wallet, a lookup after, as
    /// [`Wallet::address_on`] keeps addresses.
    pub fn script_at(
        &self,
        c: &Compute,
        change: bool,
        index: u32,
    ) -> Option<faraday_bip::bitcoin::ScriptBuf> {
        if let Some((.., s)) = self
            .scripts
            .borrow()
            .iter()
            .find(|(c, i, _)| *c == change && *i == index)
        {
            return s.clone();
        }
        let s = {
            crate::work::count(crate::work::Work::WalletMatch);
            self.policy.script_at(c, change, index).ok()
        };
        let mut kept = self.scripts.borrow_mut();
        // A long walk through places keeps the latest only.
        if kept.len() >= WALLET_SCRIPTS_KEPT {
            kept.clear();
        }
        kept.push((change, index, s.clone()));
        s
    }

    /// Costs an address derivation the first time: its address on
    /// `network` at `change`/`index`, or why there is none, derived the
    /// first time it is asked and kept on the wallet, a lookup after
    /// (`docs/SPEED.md` §2: a kept value is public and goes with what it
    /// describes). A frame that shows an address asks it again every
    /// frame; it is derived once.
    pub fn address_on(&self, c: &Compute, network: Network, change: bool, index: u32) -> String {
        if let Some((.., a)) = self
            .addresses
            .borrow()
            .iter()
            .find(|(n, c, i, _)| *n == network && *c == change && *i == index)
        {
            return a.clone();
        }
        let a = {
            self.policy
                .address_at(c, network, change, index)
                .map(|a| a.to_string())
                .unwrap_or_else(|e| fill(EN.wallet_no_address, &[&e.to_string()]))
        };
        let mut kept = self.addresses.borrow_mut();
        // A long walk through addresses keeps the latest only.
        if kept.len() >= WALLET_ADDRESSES_KEPT {
            kept.clear();
        }
        kept.push((network, change, index, a.clone()));
        a
    }

    /// The policy, as it was read.
    pub fn policy(&self) -> &WalletPolicy {
        &self.policy
    }

    /// The policy's descriptor checksum; a lookup, kept by the policy.
    pub fn checksum(&self) -> &str {
        self.policy.checksum()
    }

    /// Replaces the policy, dropping the addresses and scripts kept from
    /// the one before.
    pub fn set_policy(&mut self, policy: WalletPolicy) {
        self.policy = policy;
        self.addresses.borrow_mut().clear();
        self.scripts.borrow_mut().clear();
    }
}

/// What a wallet's [`Wallet::source`] is: Family's own "Typed words"
/// step, Seeds' own "Typed seeds" step, a vault's own wallet, text that
/// reads as a statement on its own ("Made here", "Dealt here"), or
/// anything else (a file's name, a search, "Test wallet"). Given by the
/// caller that adds the wallet, where it already knows which this is;
/// every other reader (`source_tag`, Family's routing) reads this
/// instead of the text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Origin {
    /// Family's own "Typed words" step ([`crate::family::WORDS_SOURCE`]).
    TypedWords,
    /// Seeds' own "Typed seeds" step ([`crate::seeds::SOURCE`]).
    TypedSeeds,
    /// A vault's own wallet.
    Vault,
    /// Reads as a statement on its own ("Made here", "Dealt here"):
    /// [`source_tag`] puts no "From"/"from" before it.
    Statement,
    /// Anything else: a file's name, a search, "Test wallet" —
    /// [`source_tag`] puts "From"/"from" before it.
    Named,
}

/// What the nonce check found ([`Session::nonce_check`]).
pub struct NonceCheck {
    /// Every signature, in input order.
    pub rows: Vec<NonceRow>,
    /// Nonces used twice under one key: the key, given away.
    pub reused: Vec<faraday_psbt::verify::NonceReuse>,
}

impl NonceCheck {
    /// Whether nothing is wrong and nothing was left out: every signature
    /// was checked and is valid, every one made here was recomputed, and
    /// no nonce repeats. A signature that could not be checked (no
    /// previous output, a script that names no key) is not sound: it is
    /// not known to be this transaction's. A lookup per row.
    pub fn sound(&self) -> bool {
        self.reused.is_empty() && self.rows.iter().all(|r| !r.wrong() && !r.unchecked())
    }

    /// Whether a signature is wrong or a nonce repeats: an invalid
    /// signature, one made here by no nonce rule, or a nonce used twice.
    /// A lookup per row.
    pub fn wrong(&self) -> bool {
        !self.reused.is_empty() || self.rows.iter().any(NonceRow::wrong)
    }

    /// How many signatures could not be checked. A lookup per row.
    pub fn unchecked(&self) -> usize {
        self.rows.iter().filter(|r| r.unchecked()).count()
    }

    /// The check's headline, as the spend screen shows it: how many
    /// signatures are valid and recomputed here when it is sound, that a
    /// signature is wrong when one is, and otherwise how many of them
    /// were not checked. A lookup per row.
    pub fn headline(&self) -> String {
        let total = self.rows.len();
        let noun = if total == 1 {
            EN.nonce_check_signature
        } else {
            EN.nonce_check_signatures
        };
        if self.wrong() {
            EN.nonce_check_nonce_check.to_string()
        } else if self.unchecked() > 0 {
            fill(
                EN.nonce_unchecked_head,
                &[&self.unchecked().to_string(), &total.to_string(), noun],
            )
        } else {
            let here = self.rows.iter().filter(|r| r.recomputed.is_some()).count();
            fill(
                EN.nonce_check_nonce_check_valid,
                &[&total.to_string(), noun, &here.to_string()],
            )
        }
    }
}

/// One signature, as the nonce check found it.
pub struct NonceRow {
    /// The input it signs.
    pub input: usize,
    /// Its key: the master fingerprint, or the key's first characters.
    pub name: String,
    /// Whether it is the key's over this transaction.
    pub verdict: Verdict,
    /// For a key held here: the nonce rule that recomputed it, or `None`
    /// inside when none did. `None` outside for a key held elsewhere,
    /// whose nonce cannot be recomputed without its private key.
    pub recomputed: Option<Option<faraday_psbt::verify::NonceMode>>,
}

impl NonceRow {
    /// Whether this signature is wrong: invalid, or made here by no
    /// nonce rule. A lookup.
    pub fn wrong(&self) -> bool {
        self.verdict.is_invalid() || matches!(self.recomputed, Some(None))
    }

    /// Whether this signature could not be checked. A lookup.
    pub fn unchecked(&self) -> bool {
        matches!(self.verdict, Verdict::Unchecked(_))
    }
}

/// One key slot of a wallet, in the descriptor's order.
pub struct Slot {
    /// The key's master fingerprint, when the descriptor gives one.
    pub fingerprint: Option<Fingerprint>,
    /// The label of the session key that fills it, when one does.
    pub held_by: Option<String>,
    /// The descriptor's key for it, which says whether a seed is this
    /// slot's where the fingerprint only finds a candidate; `None` for a
    /// silent payments wallet's key.
    pub key: Option<faraday_bip::policy::PolicyKey>,
}

impl Slot {
    /// A slot known by its master fingerprint alone: a silent payments
    /// wallet's key. A lookup.
    pub fn bare(fp: Fingerprint) -> Slot {
        Slot {
            fingerprint: Some(fp),
            held_by: None,
            key: None,
        }
    }
}

pub use faraday_keys::fp_text;

/// A wallet's source as a tag: "Vault" reads "From Vault" (`cap`) or
/// "from Vault" (inline, after another phrase), but a source whose
/// [`Origin`] already reads as a complete statement ("Made here",
/// "Dealt here") reads on its own, with no "From"/"from" before it
/// either way.
pub fn source_tag(source: &str, origin: Origin, cap: bool) -> String {
    if origin == Origin::Statement {
        source.to_string()
    } else if cap {
        fill(EN.wallet_source_tag, &[source])
    } else {
        fill(EN.wallet_from_inline, &[source])
    }
}

/// A key as a line: its fingerprint, then its label when the label is
/// not the fingerprint itself.
pub fn key_line(fp: Fingerprint, label: &str) -> String {
    let f = fp_text(fp);
    let label = label.trim();
    if label.is_empty() || label.eq_ignore_ascii_case(&f) {
        f
    } else {
        format!("{f} · {label}")
    }
}

/// Keys and wallets for this session. Nothing here is written anywhere.
pub struct Session {
    /// Keys that can sign, and the network they derive for.
    pub keys: faraday_keys::Keys,
    /// Wallets.
    pub wallets: Vec<Wallet>,
    /// The modes the app offers ([`crate::Faraday::modes`]): the ones a
    /// wallet or a PSBT of another network may move an empty session to
    /// ([`Session::follow`]).
    pub modes: &'static [crate::modes::Mode],
    /// The keys loaded from codex32 strings that went in by a correction,
    /// by fingerprint, with their mark (`docs/CODEX32.md` §8.4): the line
    /// beside the key's fingerprint wherever it is shown. Never taken
    /// away.
    pub key_marks: crate::codex32_correct::KeyMarks,
}

impl Default for Session {
    fn default() -> Self {
        Session {
            keys: faraday_keys::Keys::on(START_NETWORK),
            wallets: Vec::new(),
            modes: crate::modes::STICK_MODES,
            key_marks: crate::codex32_correct::KeyMarks::default(),
        }
    }
}

/// The network a PSBT is on, when it says: its global xpubs, or the coin
/// type of a BIP-44, 49, 84, 86 or 48 key path. A PSBT that says neither
/// is taken on the session's network.
pub fn psbt_network(psbt: &Psbt, current: Network) -> Option<Network> {
    use faraday_bip::bitcoin::bip32::ChildNumber;
    let inner = psbt.inner();
    if let Some(x) = inner.xpub.keys().next() {
        return Some(network_of_kind(x.network, current));
    }
    let paths = inner
        .inputs
        .iter()
        .flat_map(|i| {
            i.bip32_derivation
                .values()
                .map(|(_, p)| p)
                .chain(i.tap_key_origins.values().map(|(_, (_, p))| p))
        })
        .chain(inner.outputs.iter().flat_map(|o| {
            o.bip32_derivation
                .values()
                .map(|(_, p)| p)
                .chain(o.tap_key_origins.values().map(|(_, (_, p))| p))
        }));
    for path in paths {
        let v: Vec<ChildNumber> = path.into_iter().copied().collect();
        if let [
            ChildNumber::Hardened {
                index: 44 | 49 | 84 | 86 | 48,
            },
            ChildNumber::Hardened { index: c },
            ..,
        ] = v[..]
        {
            return Some(match c {
                0 => Network::Mainnet,
                _ => network_of_kind(faraday_bip::bitcoin::NetworkKind::Test, current),
            });
        }
    }
    None
}

impl Session {
    /// A session on `network`.
    pub fn on(network: Network) -> Session {
        Session {
            keys: faraday_keys::Keys::on(network),
            wallets: Vec::new(),
            modes: crate::modes::STICK_MODES,
            key_marks: crate::codex32_correct::KeyMarks::default(),
        }
    }

    /// The network every key derives for.
    pub fn network(&self) -> Network {
        self.keys.network()
    }

    /// Takes the network a wallet or a PSBT is on. The session moves to it
    /// only when all hold: no key and no wallet is loaded, it is not
    /// Practice, its network is not fixed, and the mode that network is
    /// belongs to [`Session::modes`]; that is the same as choosing the
    /// mode on an empty session, so no lock is needed (F81,
    /// `docs/MODES.md` §2.8). A network of the same kind needs no move.
    /// Anything else of another kind is refused, naming both networks.
    pub fn follow(&mut self, network: Option<Network>) -> Result<(), Refusal> {
        let Some(n) = network else { return Ok(()) };
        if n.kind() == self.network().kind() {
            return Ok(());
        }
        let empty = self.keys.is_empty() && self.wallets.is_empty();
        if self.keys.network_fixed() {
            return Err(Refusal::NetworkFixed(self.network()));
        }
        if !empty || self.keys.demo() || !self.modes.contains(&Mode::of(n)) {
            return Err(Refusal::Network {
                offered: n,
                loaded: self.network(),
            });
        }
        self.keys.set_network(n, false)
    }

    /// Takes the network of a PSBT that is about to be signed.
    pub fn follow_psbt(&mut self, psbt: &Psbt) -> Result<(), Refusal> {
        self.follow(psbt_network(psbt, self.network()))
    }

    /// Costs what `read_wallet` and `add_policy` cost. Adds the wallet the
    /// text describes, under `name`, unless the same descriptor is already
    /// loaded. `origin`
    /// is what [`Wallet::source`] is: the caller already knows this (it is
    /// the one that chose `source`'s words), so it says so here rather than
    /// this reading the text back.
    pub fn add_wallet(
        &mut self,
        c: &Compute,
        name: &str,
        text: &str,
        source: &str,
        origin: Origin,
    ) -> Result<usize, Refusal> {
        self.add_policy(c, name, read_wallet(c, text)?, source, origin)
    }

    /// Costs two key derivations per loaded key with a silent payments
    /// wallet's fingerprint, nothing for any other wallet.
    /// [`Session::add_wallet`] of a description read before (a vault's
    /// kept records): no parse. A silent payments wallet whose scan key or
    /// spend key is not what a loaded key with its fingerprint gives at
    /// its account is refused.
    pub fn add_policy(
        &mut self,
        c: &Compute,
        name: &str,
        policy: WalletPolicy,
        source: &str,
        origin: Origin,
    ) -> Result<usize, Refusal> {
        if let Some(w) = policy.silent()
            && crate::silent::check_record(c, &self.keys, w).0
                == crate::silent::RecordCheck::Differs
        {
            return Err(Refusal::SilentDiffers(fp_text(w.fingerprint).to_string()));
        }
        let descriptor = same_wallet(&policy);
        if let Some(i) = self
            .wallets
            .iter()
            .position(|w| same_wallet(&w.policy) == descriptor)
        {
            return Ok(i);
        }
        self.follow(policy_network(&policy, self.network()))?;
        self.wallets.push(Wallet::new(
            name.to_string(),
            policy,
            source.to_string(),
            origin,
        ));
        Ok(self.wallets.len() - 1)
    }

    /// Sets the demo up (F20, F81): the session takes only the public set
    /// `samples` holds (every sample key, with and without `password`),
    /// and New key hands out the next sample of the length chosen in
    /// place of the key it made ([`faraday_keys::Keys::next_handout`]). A
    /// lookup: derives nothing. A shell that wants Practice makes the session
    /// on Testnet first: its addresses are then real testnet addresses,
    /// which no mainnet wallet pays.
    pub fn set_demo(&mut self, samples: &crate::samples::Samples) {
        self.keys
            .set_demo(&samples.handouts(), &samples.public(), &samples.nodes());
    }

    /// Drops every key and wallet.
    pub fn wipe(&mut self) {
        self.keys.clear();
        self.wallets.clear();
    }

    /// The keys loaded that no wallet loaded uses, in the order they
    /// were added.
    pub fn loose_keys(&self) -> Vec<Fingerprint> {
        self.keys
            .all()
            .map(|k| k.fingerprint)
            .filter(|&fp| {
                !self
                    .wallets
                    .iter()
                    .any(|w| self.slots(w).iter().any(|s| s.fingerprint == Some(fp)))
            })
            .collect()
    }

    /// The wallet's slots, in the descriptor's order.
    pub fn slots(&self, wallet: &Wallet) -> Vec<Slot> {
        // A silent payments wallet's one slot is the key its two halves
        // were derived from.
        if let Some(r) = wallet.policy.silent() {
            return vec![Slot {
                fingerprint: Some(r.fingerprint),
                held_by: self.keys.key_label(r.fingerprint).map(str::to_string),
                key: None,
            }];
        }
        wallet
            .policy
            .keys()
            .iter()
            .map(|k| {
                let fingerprint = k.fingerprint();
                Slot {
                    fingerprint,
                    held_by: fingerprint
                        .and_then(|fp| self.keys.key_label(fp))
                        .map(str::to_string),
                    key: Some(k.clone()),
                }
            })
            .collect()
    }

    /// The quorum: (signatures needed, keys).
    pub fn quorum(wallet: &Wallet) -> (usize, usize) {
        wallet
            .policy
            .quorum()
            .or_else(|| wallet.policy.tapscript_quorum())
            .unwrap_or((1, wallet.policy.keys().len().max(1)))
    }

    /// The wallet's shape in a few words.
    pub fn shape(wallet: &Wallet) -> String {
        let script = |t: ScriptType| match t {
            ScriptType::Legacy => EN.wallet_legacy,
            ScriptType::NestedSegwit => EN.wallet_nested_segwit,
            ScriptType::NativeSegwit => EN.wallet_native_segwit,
            ScriptType::Taproot => EN.wallet_taproot,
        };
        let (m, n) = Self::quorum(wallet);
        match Kind::of(&wallet.policy) {
            Kind::Single(t) => fill(EN.wallet_single_key, &[script(t)]),
            Kind::Multi(Wrapper::Sh) => fill(
                EN.wallet_of_legacy_multisig,
                &[&m.to_string(), &n.to_string()],
            ),
            Kind::Multi(Wrapper::ShWsh) => fill(
                EN.wallet_of_nested_segwit,
                &[&m.to_string(), &n.to_string()],
            ),
            Kind::Multi(_) => fill(
                EN.wallet_of_native_segwit,
                &[&m.to_string(), &n.to_string()],
            ),
            Kind::TapMulti => fill(
                EN.wallet_of_taproot_multisig,
                &[&m.to_string(), &n.to_string()],
            ),
            Kind::Miniscript => fill(
                EN.wallet_keys_miniscript,
                &[&wallet.policy.keys().len().to_string()],
            ),
            Kind::Tree => fill(
                EN.wallet_keys_taproot,
                &[&wallet.policy.keys().len().to_string()],
            ),
            Kind::MuSig => fill(EN.wallet_of_musig2, &[&n.to_string(), &n.to_string()]),
            Kind::Silent => EN.wallet_1_key_silent.to_string(),
            Kind::Other => EN.wallet_wallet.to_string(),
        }
    }

    /// Costs an address derivation (through `address_on`). An address of
    /// the wallet, on the session's network. Derived once per wallet and
    /// kept on it ([`Wallet::address_on`]).
    pub fn address(&self, c: &Compute, wallet: &Wallet, change: bool, index: u32) -> String {
        wallet.address_on(c, self.network(), change, index)
    }

    /// Costs an address derivation (through `address`). An address of the
    /// wallet as the screen shows it: the address itself, in Practice too.
    pub fn address_shown(&self, c: &Compute, wallet: &Wallet, change: bool, index: u32) -> String {
        self.address(c, wallet, change, index)
    }

    /// Costs a key derivation (through `from_master`).
    fn key_refs(&self, c: &Compute) -> Vec<KeyRef> {
        self.keys.key_refs(c)
    }

    fn policies(&self) -> Vec<WalletPolicy> {
        self.wallets.iter().map(|w| w.policy.clone()).collect()
    }

    /// Costs what `key_refs` and `deterministic` cost. The nonce check at
    /// the end of a spend: every signature the PSBT carries checked against
    /// its key and this transaction, each one a key held here made
    /// recomputed from that key (RFC 6979 or BIP-340 with no extra
    /// randomness), and any nonce used twice under one key
    /// (`faraday_psbt::verify`).
    pub fn nonce_check(&self, c: &Compute, psbt: &Psbt) -> NonceCheck {
        let refs = self.key_refs(c);
        let policies = self.policies();
        let ctx = context(self.network(), &refs, &policies);
        let made = self.keys.recompute_signatures(c, psbt);
        let mut rows = Vec::new();
        for input in faraday_psbt::verify::verify_signatures(psbt, &ctx) {
            for sig in input.signatures {
                let recomputed = made
                    .iter()
                    .find(|d| d.input == input.index && d.key == sig.key)
                    .map(|d| d.mode);
                rows.push(NonceRow {
                    input: input.index,
                    name: sig.name(),
                    verdict: sig.verdict,
                    recomputed,
                });
            }
        }
        NonceCheck {
            rows,
            reused: faraday_psbt::verify::repeated_nonces(psbt, &ctx),
        }
    }

    /// What an inspection is worked out from beside the PSBT
    /// ([`InspectInputs`]). Lookups: each key's id and each wallet's
    /// checksum are kept.
    pub fn inspect_inputs(&self) -> InspectInputs {
        InspectInputs {
            network: self.network(),
            keys: self.keys.all().map(|k| k.id.0).collect(),
            wallets: self
                .wallets
                .iter()
                .map(|w| w.checksum().to_string())
                .collect(),
        }
    }

    /// Costs what `key_refs` and `inspect` cost. The inspection of `psbt`
    /// against this session's keys and wallets.
    pub fn inspect(&self, c: &Compute, psbt: &Psbt) -> Inspection {
        let refs = self.key_refs(c);
        let policies = self.policies();
        let ctx = context(self.network(), &refs, &policies);
        faraday_psbt::inspect(c, psbt, &ctx)
    }

    /// Costs what `sign_with` costs. Signs `psbt` with every session key it
    /// names. Returns the fingerprints that signed.
    pub fn sign(
        &self,
        c: &Compute,
        psbt: &mut Psbt,
        seed: &zeroize::Zeroizing<[u8; 32]>,
        musig: &mut Option<faraday_psbt::MusigSession>,
    ) -> Result<Vec<Fingerprint>, String> {
        self.sign_with(c, psbt, seed, musig, None).map(|s| s.by)
    }

    /// Costs what `key_refs`, `inspect` and the rest it calls cost. Signs
    /// `psbt` with every session key it names. `known` is the inspection
    /// of `psbt` with this session when the caller keeps one current (the
    /// open spend's): it chooses the keys without inspecting again.
    pub fn sign_with(
        &self,
        c: &Compute,
        psbt: &mut Psbt,
        seed: &zeroize::Zeroizing<[u8; 32]>,
        musig: &mut Option<faraday_psbt::MusigSession>,
        known: Option<&Inspection>,
    ) -> Result<Signed, String> {
        let refs = self.key_refs(c);
        let policies = self.policies();
        let ctx = context(self.network(), &refs, &policies);
        let own;
        let inspection = match known {
            Some(i) => i,
            None => {
                own = faraday_psbt::inspect(c, psbt, &ctx);
                &own
            }
        };
        let selection: Vec<Fingerprint> = inspection
            .participating_keys
            .iter()
            .copied()
            .filter(|fp| self.keys.position(*fp).is_some())
            .collect();
        if selection.is_empty() {
            return Err(EN.wallet_no_signing_key.to_string());
        }
        let result = self
            .keys
            .sign(c, psbt, &selection, ctx, musig, seed)
            .map_err(|e| e.to_string())?;
        let mut by: Vec<Fingerprint> = result.signed_inputs.iter().map(|s| s.fingerprint).collect();
        by.dedup();
        Ok(Signed { by })
    }
}

fn context<'a>(network: Network, refs: &'a [KeyRef], policies: &'a [WalletPolicy]) -> Context<'a> {
    Context {
        network,
        keys: refs,
        wallets: policies,
        musig_session: None,
    }
}

/// What kind of wallet a policy describes, which decides the steps of
/// its flows and the words they use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// One key: `pkh`, `sh(wpkh)`, `wpkh` or `tr` key path.
    Single(ScriptType),
    /// `multi` or `sortedmulti`, in `sh`, `wsh` or `sh(wsh)`.
    Multi(Wrapper),
    /// `tr` with an unspendable key and one `multi_a` or `sortedmulti_a`
    /// leaf.
    TapMulti,
    /// A `wsh` or `sh(wsh)` miniscript: spending paths with conditions.
    Miniscript,
    /// A `tr` with a key path and a tree of leaves.
    Tree,
    /// MuSig2: one aggregate key, every participant signs in two rounds.
    MuSig,
    /// Silent payments (BIP-352): one key's scan and spend halves, an
    /// `sp1…` address, and nothing to spend from yet.
    Silent,
    /// Anything else this build reads but has no flow for.
    Other,
}

/// The steps of a spend, in the order they may appear.
pub mod step {
    /// Which wallet the transaction spends from.
    pub const WALLET: u8 = 0;
    /// The wallet's first addresses, to compare with the online wallet.
    pub const CHECK: u8 = 1;
    /// The transaction in sentences, with its checks.
    pub const TRANSACTION: u8 = 2;
    /// The transaction id, before anyone signs.
    pub const TXID: u8 = 3;
    /// Which keys sign here and which elsewhere.
    pub const SIGNERS: u8 = 4;
    /// Signing with the keys held here.
    pub const SIGN: u8 = 5;
    /// Signatures arriving from other devices.
    pub const COLLECT: u8 = 6;
    /// The signed PSBT and the finished transaction.
    pub const FINISH: u8 = 7;
    /// Which way a wallet with several paths is spent.
    pub const PATH: u8 = 8;
    /// MuSig2's first round: everyone's nonces.
    pub const NONCES: u8 = 9;
}

impl Kind {
    /// The kind a policy describes.
    pub fn of(policy: &WalletPolicy) -> Kind {
        if policy.silent().is_some() {
            return Kind::Silent;
        }
        if policy.tapscript_quorum().is_some() {
            return Kind::TapMulti;
        }
        match policy.template() {
            Template::Single { script } => Kind::Single(script),
            Template::Multi { wrapper, .. } => Kind::Multi(wrapper),
            Template::Miniscript { .. } => Kind::Miniscript,
            Template::Tree => Kind::Tree,
            Template::MuSig => Kind::MuSig,
            _ => Kind::Other,
        }
    }

    /// Whether more than one key can take part.
    pub fn has_cosigners(self) -> bool {
        !matches!(self, Kind::Single(_) | Kind::Silent | Kind::Other)
    }

    /// The steps of a spend from a wallet of this kind
    /// (`docs/WALLETS.md` §4): Wallet, Transaction, Check, Signers, Sign,
    /// Signatures for this transaction, Finish, with Path and Nonces
    /// where the kind needs them. The transaction's id is a row of the
    /// Transaction card's table, not a step of its own.
    pub fn steps(self) -> Vec<u8> {
        let mut v = vec![step::WALLET];
        if matches!(self, Kind::Miniscript | Kind::Tree) {
            v.push(step::PATH);
        }
        v.push(step::TRANSACTION);
        v.push(step::CHECK);
        if self.has_cosigners() {
            v.push(step::SIGNERS);
        }
        if self == Kind::MuSig {
            v.push(step::NONCES);
        }
        v.push(step::SIGN);
        if self.has_cosigners() {
            v.push(step::COLLECT);
        }
        v.push(step::FINISH);
        v
    }
}

impl Session {
    /// Costs a txid and, the first time a wallet is asked a place, a script
    /// derivation ([`Wallet::script_at`]). The loaded wallet whose script
    /// the transaction's first input spends, found by re-deriving each
    /// wallet's script at the paths the input's own key origins state.
    pub fn wallet_for(&self, c: &Compute, psbt: &Psbt) -> Option<usize> {
        self.spends_from(c, &PsbtSeen::of(psbt))
    }

    /// Costs what `script_at` costs. [`Session::wallet_for`] of a PSBT read
    /// before ([`PsbtSeen`]): lookups once each wallet keeps its scripts at
    /// the input's places, so a frame may ask it of every file in Files.
    pub fn spends_from(&self, c: &Compute, seen: &PsbtSeen) -> Option<usize> {
        let (spk, places) = seen.first.as_ref()?;
        self.wallets.iter().position(|w| {
            places
                .iter()
                .any(|(change, index)| w.script_at(c, *change, *index).as_ref() == Some(spk))
        })
    }
}

/// What an inspection of a PSBT is worked out from beside it: the
/// network, each key (by [`master_id`]) and each wallet (by its
/// checksum). Two equal ones inspect a PSBT alike. Public.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InspectInputs {
    network: Network,
    keys: Vec<[u8; 32]>,
    wallets: Vec<String>,
}

/// What a PSBT says that the screens read, taken once when it is read
/// (`docs/SPEED.md` §3.3): its transaction's id, its signatures, and the
/// script and places its first input spends from, which find its wallet
/// ([`Session::spends_from`]) whatever wallets are loaded then. Public.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PsbtSeen {
    /// The unsigned transaction's id.
    pub txid: Txid,
    /// Partial signatures on all its inputs.
    pub sigs: usize,
    /// The first input's script, and each (change, index) its key
    /// origins state.
    first: Option<(faraday_bip::bitcoin::ScriptBuf, Vec<(bool, u32)>)>,
}

impl PsbtSeen {
    /// What `psbt` says. Costs a txid.
    pub fn of(psbt: &Psbt) -> PsbtSeen {
        let inner = psbt.inner();
        let sigs = inner.inputs.iter().map(|i| i.partial_sigs.len()).sum();
        let first = (|| {
            let input = inner.inputs.first()?;
            let txin = inner.unsigned_tx.input.first()?;
            let spk = match (&input.witness_utxo, &input.non_witness_utxo) {
                (Some(out), _) => out.script_pubkey.clone(),
                (None, Some(tx)) => tx
                    .output
                    .get(txin.previous_output.vout as usize)?
                    .script_pubkey
                    .clone(),
                _ => return None,
            };
            let mut places: Vec<(bool, u32)> = Vec::new();
            let paths = input
                .bip32_derivation
                .values()
                .map(|(_, p)| p.clone())
                .chain(input.tap_key_origins.values().map(|(_, (_, p))| p.clone()));
            for path in paths {
                let n: Vec<u32> = path.into_iter().map(|c| u32::from(*c)).collect();
                if n.len() >= 2 {
                    let (chain, index) = (n[n.len() - 2], n[n.len() - 1]);
                    if chain <= 1 && !places.contains(&(chain == 1, index)) {
                        places.push((chain == 1, index));
                    }
                }
            }
            Some((spk, places))
        })();
        PsbtSeen {
            txid: inner.unsigned_tx.compute_txid(),
            sigs,
            first,
        }
    }
}

/// Whether the transaction's id is fixed before it is signed: every
/// input spends a native SegWit or Taproot output, so no signature lands
/// in the part of the transaction its id is taken over.
pub fn txid_known(psbt: &Psbt) -> bool {
    let inner = psbt.inner();
    inner
        .inputs
        .iter()
        .zip(&inner.unsigned_tx.input)
        .all(|(input, txin)| {
            let spk = match (&input.witness_utxo, &input.non_witness_utxo) {
                (Some(out), _) => Some(out.script_pubkey.clone()),
                (None, Some(tx)) => tx
                    .output
                    .get(txin.previous_output.vout as usize)
                    .map(|o| o.script_pubkey.clone()),
                _ => None,
            };
            spk.is_some_and(|s| s.is_p2wpkh() || s.is_p2wsh() || s.is_p2tr())
        })
}

/// Signatures a spend from this wallet needs: the quorum of a multisig,
/// one for any other kind. A miniscript or a tree may want more on some
/// paths; for those the finish step says whether the script is satisfied.
pub fn needed(wallet: &Wallet) -> usize {
    if Kind::of(&wallet.policy) == Kind::MuSig {
        return wallet.policy.keys().len();
    }
    wallet
        .policy
        .quorum()
        .or_else(|| wallet.policy.tapscript_quorum())
        .map(|(m, _)| m)
        .unwrap_or(1)
}

/// A wallet's descriptor, the form the same wallet read from a
/// descriptor, a wallet .json or a multisig config compares equal in:
/// the policy writes every hardened step `h` however the file wrote it
/// (F66). Writes the descriptor out.
pub fn same_wallet(policy: &WalletPolicy) -> String {
    policy.to_descriptor()
}

/// What a file holds, as far as this build is concerned.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileKind {
    /// A PSBT, binary or base64.
    Psbt,
    /// A wallet description.
    Wallet,
    /// Another wallet's file that holds a private key or a seed beside its
    /// public keys (an Electrum wallet file with an `xprv` or a `seed`,
    /// `docs/INTEROP.md` §5 I6): a secret. It loads as the wallet its
    /// public keys describe; nothing secret in it is read.
    SecretWallet,
    /// A finished transaction, as hex.
    Transaction,
    /// A sheet to print on the desktop app.
    Sheet,
    /// A signed message: address, signature and text, one per line.
    Message,
    /// A cosigner's account key, `[fingerprint/path]xpub`.
    Key,
    /// A seed's words, English, checksum valid: a key that can be loaded.
    Words,
    /// A share of a split multisig backup: a config with some keys left off.
    Share,
    /// A Faraday vault (`docs/VAULT.md`), locked.
    Vault,
    /// Entries for a vault: `otpauth://` URIs and `field: value` lines.
    Entries,
    /// An EFI image to sign or check for Secure Boot.
    EfiImage,
    /// A KeePass database, sealed under its passphrase.
    Kdbx,
    /// A PDF to print: a backup sheet or a blank template.
    Pdf,
    /// A SLIP-39 share or a codex32 string: part of a seed, or a whole
    /// codex32 secret.
    SeedPart,
    /// Plain text Faraday reads as nothing else, such as recovery codes:
    /// kept as a note in a vault.
    Text,
    /// An ASCII-armored GPG public key or signature: public (F22). A
    /// revocation certificate is not (F32).
    Gpg,
    /// Nothing this build reads.
    Other,
}

/// What collecting signatures still asks for: "Collect 2 signatures"
/// before any is in, "Collect 1 more signature" after. "More" is said
/// only once there is one.
pub fn collect_line(missing: usize, collected: usize) -> String {
    fill(
        EN.wallet_collect,
        &[
            &missing.to_string(),
            &(if collected == 0 {
                String::new()
            } else {
                format!(" {}", EN.wallet_more)
            }),
            (if missing == 1 {
                EN.wallet_signature
            } else {
                EN.wallet_signatures
            }),
        ],
    )
}

/// Costs a PSBT parse (through `parse_bytes` and `parse_base64`). Reads
/// a PSBT from a file's bytes, binary or base64.
pub fn read_psbt(c: &Compute, bytes: &[u8]) -> Option<Psbt> {
    if let Ok(p) = Psbt::parse_bytes(c, bytes) {
        return Some(p);
    }
    let text = std::str::from_utf8(bytes).ok()?;
    Psbt::parse_base64(c, text.trim()).ok()
}

/// The most a plain text file may hold to be kept as a note.
pub const TEXT_MAX: usize = 64 * 1024;

/// Plain text a person wrote or saved: printable, lines, and short
/// enough for a note.
fn is_text(text: &str) -> bool {
    !text.trim().is_empty()
        && text.len() <= TEXT_MAX
        && text
            .chars()
            .all(|c| !c.is_control() || matches!(c, '\n' | '\r' | '\t'))
}

/// Costs what `secret_text` costs. Whether a piece of text, a user ID
/// or an armour header, is a seed's words, an extended private key or a
/// share: the detectors that read a plain text file.
fn text_may_be_secret(c: &Compute, text: &str) -> bool {
    faraday_intake::secret_text(c, text).is_some()
        || crate::forms::typed_mnemonic(Language::English, &crate::forms::file_words(text)).is_ok()
        || crate::restore::is_share(text)
        || crate::inbox::part_of(text).is_some()
}

/// Costs what `classify_read` costs. Says what a file holds.
pub fn classify(c: &Compute, name: &str, bytes: &[u8]) -> FileKind {
    classify_read(c, name, bytes).kind
}

/// A file classified ([`classify_read`]), with what was read to tell:
/// a PSBT's summary, a wallet file's wallet.
pub struct Classified {
    /// What it holds.
    pub kind: FileKind,
    /// A PSBT's summary, when it is one.
    pub psbt: Option<PsbtSeen>,
    /// The wallet a wallet file describes, when it is one.
    pub wallet: Option<WalletPolicy>,
}

impl From<FileKind> for Classified {
    fn from(kind: FileKind) -> Classified {
        Classified {
            kind,
            psbt: None,
            wallet: None,
        }
    }
}

/// Costs what [`classify`] costs. [`classify`], keeping what it read to
/// tell: a PSBT is parsed once and a wallet file read once when a file
/// comes in.
pub fn classify_read(c: &Compute, name: &str, bytes: &[u8]) -> Classified {
    if faraday_vault::read_header(bytes).is_ok() {
        return FileKind::Vault.into();
    }
    if bytes.starts_with(b"%PDF-") {
        return FileKind::Pdf.into();
    }
    if faraday_intake::is_kdbx(bytes) {
        return FileKind::Kdbx.into();
    }
    if crate::secure_boot::is_image(name) && bytes.starts_with(b"MZ") {
        return FileKind::EfiImage.into();
    }
    if let Some(p) = read_psbt(c, bytes) {
        return Classified {
            kind: FileKind::Psbt,
            psbt: Some(PsbtSeen::of(&p)),
            wallet: None,
        };
    }
    if let Ok(text) = std::str::from_utf8(bytes) {
        if text.starts_with("faraday-sheet 1") {
            return FileKind::Sheet.into();
        }
        if let Some(block) = faraday_pgp::read_public(bytes)
            && !block.texts.iter().any(|t| text_may_be_secret(c, t))
        {
            return FileKind::Gpg.into();
        }
        if let Some(m) = faraday_psbt::message::parse_signed(text)
            && m.address
                .parse::<faraday_bip::bitcoin::Address<_>>()
                .is_ok()
        {
            return FileKind::Message.into();
        }
        if crate::restore::is_share(text) {
            return FileKind::Share.into();
        }
        if faraday_intake::wallet::holds_secret(text) {
            return FileKind::SecretWallet.into();
        }
        if let Ok(w) = read_wallet(c, text) {
            return Classified {
                kind: FileKind::Wallet,
                psbt: None,
                wallet: Some(w),
            };
        }
        if crate::create::read_key(text).is_some() {
            return FileKind::Key.into();
        }
        if crate::forms::typed_mnemonic(Language::English, &crate::forms::file_words(text)).is_ok()
        {
            return FileKind::Words.into();
        }
        if faraday_vault::entries::looks_like_entries(text) {
            return FileKind::Entries.into();
        }
        if crate::inbox::part_of(text).is_some() {
            return FileKind::SeedPart.into();
        }
        // A raw transaction in hex, under any name: what a wallet or an
        // explorer gives to broadcast.
        let t = text.trim();
        if !t.is_empty()
            && t.bytes().all(|b| b.is_ascii_hexdigit())
            && faraday_psbt::transaction::read_raw(t.as_bytes()).is_some()
        {
            return FileKind::Transaction.into();
        }
        if is_text(text) {
            return FileKind::Text.into();
        }
    }
    FileKind::Other.into()
}

/// One transaction being signed, from the moment it is loaded.
pub struct Spend {
    /// The PSBT as it stands: every signature collected so far.
    pub psbt: Psbt,
    /// The file it came from.
    pub source: String,
    /// The unsigned transaction's id, which every collected copy must share.
    pub txid: Txid,
    /// Keys of this session that signed it here.
    pub signed_here: Vec<Fingerprint>,
    /// Fingerprints whose signatures arrived from other devices, with the
    /// file each came in.
    pub collected: Vec<(Fingerprint, String)>,
    /// The finished transaction, once there are enough signatures.
    pub finished: Option<Transaction>,
}

impl Spend {
    /// Starts a spend from a PSBT file.
    pub fn new(psbt: Psbt, source: &str) -> Spend {
        let txid = psbt.unsigned_tx().compute_txid();
        Spend {
            psbt,
            source: source.to_string(),
            txid,
            signed_here: Vec::new(),
            collected: Vec::new(),
            finished: None,
        }
    }

    /// Costs an inspection of the PSBT, but for MuSig2. Fingerprints with a
    /// valid signature on the first input; for a MuSig2 wallet, the
    /// participants whose partial signature is on it.
    pub fn signers(&self, c: &Compute, session: &Session) -> Vec<Fingerprint> {
        self.signers_by(c, session, None)
    }

    /// Costs what `wallet_for` and `inspect` cost. [`Spend::signers`], read
    /// from `inspection` of this PSBT when one is given (the spend's kept
    /// one), else inspected now.
    pub fn signers_by(
        &self,
        c: &Compute,
        session: &Session,
        inspection: Option<&Inspection>,
    ) -> Vec<Fingerprint> {
        if let Some(w) = session
            .wallet_for(c, &self.psbt)
            .and_then(|i| session.wallets.get(i))
            && Kind::of(&w.policy) == Kind::MuSig
        {
            let Some(input) = self.psbt.inner().inputs.first() else {
                return Vec::new();
            };
            if input.tap_key_sig.is_some() {
                return w
                    .policy
                    .keys()
                    .iter()
                    .filter_map(|k| k.fingerprint())
                    .collect();
            }
            let partials = faraday_psbt::musig::partial_sigs(input).unwrap_or_default();
            return w
                .policy
                .keys()
                .iter()
                .filter(|k| {
                    partials
                        .iter()
                        .any(|p| p.participant == k.xpub().public_key)
                })
                .filter_map(|k| k.fingerprint())
                .collect();
        }
        let own;
        let inspection = match inspection {
            Some(i) => i,
            None => {
                own = session.inspect(c, &self.psbt);
                &own
            }
        };
        let mut out: Vec<Fingerprint> = inspection
            .present_signatures()
            .filter(|s| s.input == 0 && s.verdict == Verdict::Valid)
            .filter_map(|s| s.fingerprint)
            .collect();
        out.dedup();
        out
    }

    /// Costs what `read_psbt`, `signers` and `inspect` cost. Adds the
    /// signatures another device put on a copy of this transaction. A PSBT
    /// for any other transaction is refused, never merged. Returns the
    /// fingerprints it added.
    pub fn collect(
        &mut self,
        c: &Compute,
        session: &Session,
        bytes: &[u8],
        source: &str,
    ) -> Result<Vec<Fingerprint>, String> {
        let other = read_psbt(c, bytes).ok_or(EN.wallet_not_a_psbt)?;
        if other.unsigned_tx().compute_txid() != self.txid {
            return Err(fill(EN.wallet_psbt_other_transaction, &[source]));
        }
        let before = self.signers(c, session);
        let mut merged = self.psbt.inner().clone();
        merged
            .combine(other.into_inner())
            .map_err(|e| fill(EN.wallet_cannot_combine, &[&e.to_string()]))?;
        // `combine` keeps the fields it has no type for, MuSig2's among them.
        let merged = Psbt::from(merged);
        if session
            .inspect(c, &merged)
            .present_signatures()
            .any(|s| s.verdict.is_invalid())
        {
            return Err(fill(EN.wallet_a_signature, &[source]));
        }
        self.psbt = merged;
        let added: Vec<Fingerprint> = self
            .signers(c, session)
            .into_iter()
            .filter(|fp| !before.contains(fp))
            .collect();
        for fp in &added {
            self.collected.push((*fp, source.to_string()));
        }
        Ok(added)
    }

    /// Finishes the transaction when it has enough signatures.
    pub fn finish(&mut self) -> Result<(), String> {
        let mut copy = self.psbt.clone();
        match faraday_psbt::finalize(&mut copy) {
            Ok(Some(tx)) => {
                self.finished = Some(tx);
                Ok(())
            }
            Ok(None) => Err(EN.wallet_not_enough_signatures.to_string()),
            Err(e) => Err(e.to_string()),
        }
    }

    /// Why the transaction cannot be finished yet, when it cannot.
    pub fn finish_reason(&self) -> Option<String> {
        let mut copy = self.psbt.clone();
        match faraday_psbt::finalize(&mut copy) {
            Ok(Some(_)) => None,
            Ok(None) => Some(EN.wallet_script_unsatisfied.to_string()),
            Err(e) => Some(e.to_string()),
        }
    }

    /// The finished transaction as hex.
    pub fn finished_hex(&self) -> Option<String> {
        self.finished.as_ref().map(serialize_hex)
    }
}

/// What checking a signed message found.
pub struct MessageCheck {
    /// The signed text, as read.
    pub signed: faraday_psbt::message::SignedText,
    /// The signature verifies against the address, and in which format;
    /// or why not.
    pub verdict: Result<faraday_psbt::message::Format, String>,
    /// The loaded wallet the address belongs to, with its chain and index.
    pub wallet: Option<(usize, bool, u32)>,
}

impl Session {
    /// The single-key wallets a message can be signed for here.
    pub fn message_wallets(&self) -> Vec<usize> {
        self.wallets
            .iter()
            .enumerate()
            .filter(|(_, w)| matches!(Kind::of(&w.policy), Kind::Single(_)))
            .filter(|(_, w)| {
                w.policy
                    .keys()
                    .first()
                    .and_then(|k| k.fingerprint())
                    .is_some_and(|fp| self.keys.key_label(fp).is_some())
            })
            .map(|(i, _)| i)
            .collect()
    }

    /// Costs what `derive` and `sign` cost. Signs `message` with the key
    /// behind address `index` of a single-key wallet's receive chain.
    pub fn sign_message(
        &self,
        c: &Compute,
        wallet: usize,
        index: u32,
        message: &str,
        format: faraday_psbt::message::Format,
    ) -> Result<faraday_psbt::message::Signed, String> {
        let w = self.wallets.get(wallet).ok_or(EN.wallet_no_such_wallet)?;
        let Kind::Single(script) = Kind::of(&w.policy) else {
            return Err(EN.wallet_messages_are.into());
        };
        let key = w.policy.keys().first().ok_or(EN.wallet_the_wallet_has)?;
        let fp = key.fingerprint().ok_or(EN.wallet_the_wallet_s_xpub)?;
        let place = self.keys.position(fp).ok_or(EN.wallet_the_wallet_s_key)?;
        let origin = key.path().ok_or(EN.wallet_the_wallet_s)?;
        let path = origin
            .child(
                faraday_bip::bitcoin::bip32::ChildNumber::from_normal_idx(0)
                    .map_err(|e| e.to_string())?,
            )
            .child(
                faraday_bip::bitcoin::bip32::ChildNumber::from_normal_idx(index)
                    .map_err(|e| e.to_string())?,
            );
        self.keys
            .sign_message(c, place, &path, script, message, format)
            .ok_or(EN.wallet_the_wallet_s_key)?
            .map_err(|e| format!("{e:?}"))
    }

    /// Costs an address derivation (through `find_address`). Checks a
    /// signed message, and finds its address among the loaded wallets'
    /// first hundred addresses on each chain.
    pub fn check_message(&self, c: &Compute, text: &str) -> Option<MessageCheck> {
        let signed = faraday_psbt::message::parse_signed(text)?;
        let address = signed
            .address
            .parse::<faraday_bip::bitcoin::Address<_>>()
            .ok()?
            .require_network(self.network().into())
            .ok()?;
        let verdict = faraday_psbt::message::verify(&address, &signed.message, &signed.signature)
            .map(|c| c.format)
            .map_err(|e| format!("{e:?}"));
        let wallet = self.wallets.iter().enumerate().find_map(|(i, w)| {
            w.policy
                .find_address(c, self.network(), &address, 100)
                .map(|(change, index)| (i, change, index))
        });
        Some(MessageCheck {
            signed,
            verdict,
            wallet,
        })
    }
}

/// What a path of a wallet with several needs, and whether this
/// transaction can take it.
pub struct PathView {
    /// The keys it needs, by slot number from 1, with whether each is here.
    pub keys: Vec<(usize, String, bool)>,
    /// Its locks, in words.
    pub locks: Vec<String>,
    /// Whether the first input's sequence allows every relative lock, and
    /// the absolute ones are not checked here.
    pub sequence_ok: bool,
}

/// Every path of a wallet, read against the transaction's first input.
pub fn paths(session: &Session, wallet: &Wallet, psbt: &Psbt) -> Vec<PathView> {
    use faraday_bip::spend::Lock;
    let seq = psbt
        .unsigned_tx()
        .input
        .first()
        .map(|i| i.sequence.0)
        .unwrap_or(0xffff_ffff);
    // BIP-68: a relative lock is in force when bit 31 is clear; bit 22
    // chooses 512-second intervals over blocks; the low 16 bits count.
    let relative = seq & (1 << 31) == 0;
    let in_time = seq & (1 << 22) != 0;
    let value = seq & 0xffff;
    let slots = session.slots(wallet);
    wallet
        .policy
        .spend_paths()
        .iter()
        .map(|p| {
            let keys = p
                .keys
                .iter()
                .map(|&k| {
                    let s = &slots[k];
                    (
                        k + 1,
                        s.fingerprint.map(fp_text).unwrap_or_default(),
                        s.held_by.is_some(),
                    )
                })
                .collect();
            let mut ok = true;
            let locks = p
                .locks
                .iter()
                .map(|l| match l {
                    Lock::Blocks(n) => {
                        ok &= relative && !in_time && value >= *n;
                        fill(
                            EN.wallet_blocks_after,
                            &[&n.to_string(), &(n / 144).to_string()],
                        )
                    }
                    Lock::Intervals(n) => {
                        ok &= relative && in_time && value >= *n;
                        fill(
                            EN.wallet_days_after_arrival,
                            &[&(u64::from(*n) * 512 / 86_400).to_string()],
                        )
                    }
                    Lock::Height(h) => fill(EN.wallet_after_block, &[&h.to_string()]),
                    Lock::Time(t) => fill(EN.wallet_after_time, &[&t.to_string()]),
                    Lock::Preimage => EN.wallet_a_secret_preimage.to_string(),
                })
                .collect();
            PathView {
                keys,
                locks,
                sequence_ok: ok,
            }
        })
        .collect()
}

/// MuSig2's first round on a spend: each participant's slot number,
/// fingerprint, whether its key is here, and whether its nonce is on the
/// PSBT.
pub fn musig_nonces(
    session: &Session,
    wallet: &Wallet,
    psbt: &Psbt,
) -> Vec<(usize, String, bool, bool)> {
    let nonces = psbt
        .inner()
        .inputs
        .first()
        .and_then(|i| faraday_psbt::musig::pub_nonces(i).ok())
        .unwrap_or_default();
    session
        .slots(wallet)
        .iter()
        .zip(wallet.policy.keys())
        .enumerate()
        .map(|(i, (slot, key))| {
            let has = nonces
                .iter()
                .any(|n| n.participant == key.xpub().public_key);
            (
                i + 1,
                slot.fingerprint.map(fp_text).unwrap_or_default(),
                slot.held_by.is_some(),
                has,
            )
        })
        .collect()
}

/// A transaction being signed, with its screen's state.
pub struct SpendState {
    /// The spend.
    pub spend: Spend,
    /// The open step, if any.
    pub open: Option<u8>,
    /// Steps closed as done.
    pub done: [bool; 10],
    /// The full table is shown.
    pub table: bool,
    /// The last refusal or failure.
    pub error: Option<String>,
    /// The inspection, kept current: worked out again only when what it
    /// is of changed ([`SpendState::inspected`]).
    pub inspection: faraday_psbt::Inspection,
    /// What [`SpendState::inspection`] is of: the PSBT, the session's
    /// network, keys and wallets. `refresh_spend` inspects again only when
    /// one of them changed.
    pub(crate) inspected: Option<(faraday_psbt::Psbt, InspectInputs)>,
    /// Fingerprints with a valid signature.
    pub signers: Vec<[u8; 4]>,
    /// Which results went to the Outbox.
    pub out_signed: bool,
    /// The finished transaction went to the Outbox.
    pub out_tx: bool,
    /// The finished transaction's hex is shown.
    pub show_hex: bool,
    /// The last nonce check, with the PSBT bytes it was made over: it
    /// signs again to compare, so it runs once per set of signatures,
    /// not once per frame.
    pub(crate) nonce_check: std::cell::RefCell<Option<(Vec<u8>, std::rc::Rc<NonceCheck>)>>,
    /// The step column's scroll, in design units.
    pub scroll: f32,
    /// The open step changed: scroll it into view on the next frame.
    pub follow: bool,
    /// The loaded wallet the transaction spends from.
    pub wallet: Option<usize>,
    /// The steps this spend shows, in order (`wallet::step`).
    pub steps: Vec<u8>,
    /// Signatures needed.
    pub needed: usize,
    /// Every input can be finalised now.
    pub complete: bool,
    /// A MuSig2 round this device opened: its secret nonce, until round
    /// two. Gone on lock.
    pub musig: Option<faraday_psbt::MusigSession>,
    /// The open step has been set once, at entry (`refresh_spend`): once
    /// true, closing every card (an interactive collapse) leaves `open`
    /// at `None` rather than reopening one.
    pub opened: bool,
}

impl SpendState {
    /// A spend opened on `spend`, its `inspection` made with the
    /// session's `inputs`. Its open step
    /// is computed in `refresh_spend`, once Check's done state is known:
    /// open at Check, or past it when this wallet is already checked
    /// this power-on.
    pub(crate) fn new(
        spend: Spend,
        inspection: faraday_psbt::Inspection,
        inputs: InspectInputs,
    ) -> Self {
        let inspected = Some((spend.psbt.clone(), inputs));
        SpendState {
            spend,
            open: None,
            done: [false; 10],
            table: false,
            error: None,
            inspection,
            inspected,
            signers: Vec::new(),
            out_signed: false,
            out_tx: false,
            show_hex: false,
            nonce_check: Default::default(),
            scroll: 0.0,
            follow: false,
            wallet: None,
            steps: Vec::new(),
            needed: 1,
            complete: false,
            musig: None,
            opened: false,
        }
    }

    /// Whether [`SpendState::inspection`] is of the PSBT as it is, with
    /// the session's `inputs` as they are.
    pub(crate) fn inspected_for(&self, inputs: &InspectInputs) -> bool {
        self.inspected
            .as_ref()
            .is_some_and(|(p, i)| i == inputs && *p == self.spend.psbt)
    }
}

/// The Wallets tab's area: the wallet chosen, the key in no wallet
/// chosen, the new name being typed, the wallet card's scroll and the
/// transaction being signed.
#[derive(Default)]
pub(crate) struct WalletsArea {
    /// The wallet chosen on the Wallets tab.
    wallet: usize,
    /// The key in no wallet chosen on the Wallets tab, by fingerprint,
    /// when several are loaded and none of them in a wallet.
    loose_pick: Option<[u8; 4]>,
    /// The chosen wallet's new name, while it is typed.
    renaming: Option<String>,
    /// Design units scrolled past in the wallet card's body, on a desktop.
    card_offset: f32,
    /// The transaction being signed.
    spend: Option<SpendState>,
}

impl WalletsArea {
    /// The wallet chosen on the Wallets tab.
    pub(crate) fn wallet(&self) -> usize {
        self.wallet
    }

    /// Chooses wallet `i`.
    pub(crate) fn set_wallet(&mut self, i: usize) {
        self.wallet = i;
    }

    /// The key in no wallet chosen, by fingerprint.
    pub(crate) fn loose_pick(&self) -> Option<[u8; 4]> {
        self.loose_pick
    }

    /// Chooses the key in no wallet `pick`.
    pub(crate) fn set_loose_pick(&mut self, pick: Option<[u8; 4]>) {
        self.loose_pick = pick;
    }

    /// The new name being typed, if one is.
    pub(crate) fn renaming(&self) -> Option<&String> {
        self.renaming.as_ref()
    }

    /// The new name being typed, to change it.
    pub(crate) fn renaming_mut(&mut self) -> Option<&mut String> {
        self.renaming.as_mut()
    }

    /// Starts, changes or ends the typing of a new name.
    pub(crate) fn set_renaming(&mut self, name: Option<String>) {
        self.renaming = name;
    }

    /// Design units scrolled past in the wallet card's body.
    pub(crate) fn card_offset(&self) -> f32 {
        self.card_offset
    }

    /// The wallet card's scroll, to move it.
    pub(crate) fn card_offset_mut(&mut self) -> &mut f32 {
        &mut self.card_offset
    }

    /// Scrolls the wallet card's body to `to`.
    pub(crate) fn set_card_offset(&mut self, to: f32) {
        self.card_offset = to;
    }

    /// The transaction being signed, if one is.
    pub(crate) fn spend(&self) -> Option<&SpendState> {
        self.spend.as_ref()
    }

    /// The transaction being signed, to change it.
    pub(crate) fn spend_mut(&mut self) -> Option<&mut SpendState> {
        self.spend.as_mut()
    }

    /// Starts signing `state`.
    pub(crate) fn set_spend(&mut self, state: SpendState) {
        self.spend = Some(state);
    }

    /// Ends the signing.
    pub(crate) fn clear_spend(&mut self) {
        self.spend = None;
    }
}

/// Sign a transaction's steps, plugged into Back and Forward (F104).
pub(crate) struct SpendSteps;

/// The one [`SpendSteps`].
pub(crate) static SPEND_STEPS: SpendSteps = SpendSteps;

impl SpendSteps {
    /// The first step Back may open in `s`: the one after Sign once a key
    /// here has signed, or after Nonces once a round is open here (its
    /// nonces are never made again). Only lookups.
    pub(crate) fn first(s: &SpendState) -> Option<u8> {
        let made = if !s.spend.signed_here.is_empty() {
            step::SIGN
        } else if s.musig.is_some() {
            step::NONCES
        } else {
            return None;
        };
        let at = s.steps.iter().position(|&k| k == made)?;
        s.steps.get(at + 1).copied()
    }
}

impl crate::trail::StepFlow for SpendSteps {
    fn steps(&self, app: &Faraday) -> Option<crate::trail::Steps> {
        let s = app.wallets.spend()?;
        let mut steps = crate::trail::Steps::new(s.steps.clone(), s.open);
        steps.first = SpendSteps::first(s);
        Some(steps)
    }

    fn open(&self, _: &Faraday, k: u8) -> Action {
        Action::Step(k)
    }

    fn name(&self, _: &Faraday, k: u8) -> &'static str {
        crate::screens::SPEND_TITLES[usize::from(k)]
    }
}

impl Faraday {
    /// The wallet chosen on the Wallets tab.
    pub fn wallet_index(&self) -> usize {
        self.wallets.wallet()
    }

    /// The key in no wallet chosen on the Wallets tab, by fingerprint.
    pub fn loose_pick(&self) -> Option<[u8; 4]> {
        self.wallets.loose_pick()
    }

    /// The chosen wallet's new name, while it is typed.
    pub fn renaming(&self) -> Option<&String> {
        self.wallets.renaming()
    }

    /// Design units scrolled past in the wallet card's body.
    pub fn card_offset(&self) -> f32 {
        self.wallets.card_offset()
    }

    /// The transaction being signed, if one is.
    pub fn spend(&self) -> Option<&SpendState> {
        self.wallets.spend()
    }

    /// Handles the open wallet's and spend's actions (`PickWallet`, `StartSpend`, `Step*`, `Sign*`, `Collect`, `Public*`, `WalletKey*` and the like).
    #[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
    pub(crate) fn wallet_act(&mut self, action: Action) {
        match action {
            Action::PickWallet(i) => {
                self.wallets.set_wallet(i);
                self.wallets.set_renaming(None);
                self.wallets.set_card_offset(0.0);
            }
            Action::PickKey(fp) => self.wallets.set_loose_pick(Some(fp)),
            Action::RemoveWallet(i) => {
                if i < self.session.wallets.len() {
                    self.session.wallets.remove(i);
                    self.wallets.set_wallet(0);
                }
            }
            Action::RemoveKeys => {
                self.session.keys.clear();
                self.wallets.clear_spend();
            }
            Action::StartSpend(i) => self.start_spend(i),
            Action::Step(n) => {
                if let Some(s) = self.wallets.spend_mut() {
                    // On the Spend tab a step opened from another card
                    // stays open.
                    let from_page = self.screen == Screen::Family
                        && self.family.state().open != Some(family::Open::Spend);
                    s.open = if s.open == Some(n) && !from_page {
                        None
                    } else {
                        Some(n)
                    };
                    s.error = None;
                    s.follow = true;
                }
                if self.screen == Screen::Family {
                    self.family_step_moved(false);
                }
            }
            Action::StepNext(n) => {
                if let Some(s) = self.wallets.spend_mut() {
                    s.done[n as usize] = true;
                    s.open = s.steps.iter().copied().find(|&i| !s.done[i as usize]);
                    s.error = None;
                    s.follow = true;
                }
                // Check's own Continue: this wallet is checked this
                // power-on (`docs/WALLETS.md` §4), kept across a lock.
                if n == wallet::step::CHECK
                    && let Some(w) = self.wallets.spend().and_then(|s| s.wallet)
                    && let Some(wallet) = self.session.wallets.get(w)
                {
                    let checksum = wallet.checksum().to_string();
                    self.mark_checked(&checksum);
                }
                if self.screen == Screen::Family {
                    self.family_step_moved(true);
                }
            }
            Action::ToggleTable => {
                if let Some(s) = self.wallets.spend_mut() {
                    s.table = !s.table;
                }
            }
            Action::SignHere => self.sign_here(),
            Action::Collect(i) => self.collect(i),
            Action::SignedToOutbox => {
                if let Some(s) = self.wallets.spend_mut() {
                    let name = format!("{}-signed.psbt", result_stem(&s.spend.source));
                    let bytes = s.spend.psbt.to_bytes();
                    s.out_signed = true;
                    self.put_outbox(&name, bytes);
                }
            }
            Action::TxToOutbox => {
                if let Some(s) = self.wallets.spend_mut()
                    && let Some(hex) = s.spend.finished_hex()
                {
                    let name = format!("{}-final.txn", result_stem(&s.spend.source));
                    s.out_tx = true;
                    self.put_outbox(&name, hex.into_bytes());
                }
            }
            Action::Primary => self.primary(),
            Action::LoadWallet(i) => self.load_wallet(i),
            Action::LoadKey(i) => {
                if self.load_key(i) {
                    self.inbox_load_named();
                }
            }
            Action::PublicOut(i, what) => self.public_out(i, what),
            Action::PublicMore(i) => {
                self.backup.public_more(i);
            }
            Action::PublicRemove(i, what) => self.public_remove(i, what),
            Action::WalletKeyOut(i, k) => self.wallet_key_out(i, k),
            Action::WalletKeyBsms(i, k) => self.wallet_key_bsms(i, k),
            Action::OpenWallet(i) => {
                if i < self.session.wallets.len() {
                    self.wallets.set_wallet(i);
                    self.screen = Screen::Wallets;
                    self.wallets.set_card_offset(0.0);
                    self.glance.set_open(false);
                    self.glance.set_focus(None);
                }
            }
            Action::WriteAsk => {
                if !self.clean() {
                    self.sheet = Some(Sheet::WriteOut);
                } else if self.stick.is_empty() {
                    self.toast(&fill(EN.act_plug_in_the_visit, &[self.medium.a()]));
                } else {
                    self.act(Action::Nav(Screen::Visit));
                }
            }
            Action::Rename => {
                // Pressing the name field again keeps what is typed.
                if self.wallets.renaming().is_none() {
                    let name = self
                        .session
                        .wallets
                        .get(self.wallets.wallet())
                        .map(|w| w.name.clone());
                    self.wallets.set_renaming(name);
                }
            }
            // `act_inner` passes only the actions matched above.
            _ => unreachable!(),
        }
    }
}

#[cfg(test)]
impl Wallet {
    /// How many addresses and scripts it keeps, for the test that bounds
    /// the caches.
    pub(crate) fn kept(&self) -> (usize, usize) {
        (self.addresses.borrow().len(), self.scripts.borrow().len())
    }
}
