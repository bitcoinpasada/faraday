//! The Wallets tab's session: keys and wallets held in memory, and the spend of
//! one transaction from loading it to the two results (`docs/WALLETS.md`
//! §2 and §4). Every piece of cryptography is `faraday-bip`'s or
//! `faraday-psbt`'s; this module only decides which call comes next.

use crate::modes::Mode;
use crate::strings::{EN, fill};
use crate::{Action, Faraday, Screen, Sheet, family, result_stem, wallet};
use faraday_bip::bip39::{Language, Mnemonic};
use faraday_bip::bitcoin::consensus::encode::serialize_hex;
use faraday_bip::bitcoin::{Transaction, Txid};
use faraday_bip::keys::{Fingerprint, Network, ScriptType};
use faraday_bip::policy::{KeyPath, Template, WalletPolicy, Wrapper};
use faraday_crypto::work::Compute;
use faraday_psbt::verify::Verdict;
use faraday_psbt::{Context, Inspection, KeyRef, Psbt};
use faraday_shell_api::Key as KeyIn;
use faraday_strings::bip::policy_error;

/// The network a session starts on, before a wallet or a PSBT says
/// otherwise.
pub(crate) const START_NETWORK: Network = Network::Mainnet;

// The description reader (`faraday-intake`),
// at the paths this crate has always had them.
pub use faraday_intake::wallet::{
    Refusal, network_name, network_of_key, policy_network, wallet_name,
};

pub use faraday_keys::identity::master_id;
pub use faraday_keys::{WordsId, words_id};

mod nonce;
pub use nonce::{NonceCheck, NonceRow, Unmatched};
mod v2;
pub use v2::{read_psbt, read_psbt_kept};
mod account;
pub use account::{Account, AccountAsk};

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

pub(crate) use faraday_intake::wallet::{carried_name, network_of_kind};

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
    scripts: std::cell::RefCell<Vec<ScriptKept>>,
    /// [`Wallet::same`], kept; dropped with the policy.
    same: std::cell::OnceCell<String>,
}

/// A script a wallet derived at a place, with the keys derived for it.
type ScriptKept = (
    bool,
    u32,
    Option<faraday_bip::bitcoin::ScriptBuf>,
    Option<Vec<(Fingerprint, faraday_bip::bitcoin::PublicKey)>>,
);

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
            same: std::cell::OnceCell::new(),
        }
    }

    /// A copy for work off the main thread: the policy, and the scripts, the
    /// addresses and [`Wallet::same`] already kept, all public.
    pub fn job_copy(&self) -> Wallet {
        Wallet {
            name: self.name.clone(),
            policy: self.policy.clone(),
            source: self.source.clone(),
            origin: self.origin,
            addresses: std::cell::RefCell::new(self.addresses.borrow().clone()),
            scripts: std::cell::RefCell::new(self.scripts.borrow().clone()),
            same: self.same.clone(),
        }
    }

    /// Takes home what a [`Wallet::job_copy`] kept while it worked: the
    /// scripts (and the keys derived beside them) and the addresses this
    /// wallet does not hold yet, and its [`Wallet::same`]. Nothing from a
    /// wallet of another checksum, and nothing already kept is replaced.
    pub fn take_kept(&self, from: Wallet) {
        if from.checksum() != self.checksum() {
            return;
        }
        let mut kept = self.scripts.borrow_mut();
        for (change, index, script, keys) in from.scripts.into_inner() {
            match kept
                .iter_mut()
                .find(|(c, i, ..)| *c == change && *i == index)
            {
                Some((.., have)) => {
                    if have.is_none() {
                        *have = keys;
                    }
                }
                None => {
                    // A long walk through places keeps the latest only.
                    if kept.len() >= WALLET_SCRIPTS_KEPT {
                        kept.clear();
                    }
                    kept.push((change, index, script, keys));
                }
            }
        }
        let mut addresses = self.addresses.borrow_mut();
        for a in from.addresses.into_inner() {
            if !addresses
                .iter()
                .any(|(n, c, i, _)| (*n, *c, *i) == (a.0, a.1, a.2))
            {
                addresses.push(a);
            }
        }
        if let Some(same) = from.same.into_inner() {
            let _ = self.same.set(same);
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
        if let Some((_, _, s, _)) = self
            .scripts
            .borrow()
            .iter()
            .find(|(c, i, ..)| *c == change && *i == index)
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
        kept.push((change, index, s.clone(), None));
        s
    }

    /// Costs what `script_at` costs where the place is not kept yet, and a
    /// derivation of each key of the wallet at the place the first time.
    /// Whether the key `stated` is what master `fp`'s key of this wallet
    /// derives to at `path`: a place that is `key/change/index` for one of
    /// that master's keys, and the keys the wallet derives there, kept
    /// beside its script (public data, so a read after is a lookup). A path
    /// that is no place of the wallet is not derived.
    pub fn derives(
        &self,
        c: &Compute,
        fp: Fingerprint,
        path: &faraday_bip::bitcoin::bip32::DerivationPath,
        stated: &faraday_psbt::verify::StatedKey,
    ) -> bool {
        let Some((change, index)) = self.policy.leaf_of(fp, path) else {
            return false;
        };
        self.script_at(c, change, index);
        let mut kept = self.scripts.borrow_mut();
        let Some((.., keys)) = kept
            .iter_mut()
            .find(|(ch, i, ..)| *ch == change && *i == index)
        else {
            return false;
        };
        let keys = keys.get_or_insert_with(|| {
            c.count(faraday_crypto::work::Work::Address);
            self.policy.keys_at(change, index)
        });
        keys.iter().any(|(f, k)| *f == fp && stated.is(&k.inner))
    }

    /// Costs a key derivation where the wallet names the master. `None`
    /// when no key of this wallet names `fp`; else whether one of its keys
    /// for that master derives, over the rest of `path` past its origin,
    /// to the key `stated`.
    pub(crate) fn master_proof(
        &self,
        c: &Compute,
        fp: Fingerprint,
        path: &faraday_bip::bitcoin::bip32::DerivationPath,
        stated: &faraday_psbt::verify::StatedKey,
    ) -> Option<bool> {
        let keys = self.policy.derive_named(c, fp, path)?;
        Some(keys.iter().any(|k| stated.is(&k.inner)))
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
                .unwrap_or_else(|e| fill(EN.wallet_no_address, &[&policy_error(&e)]))
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

    /// [`same_wallet`] of the policy, written out once and kept.
    pub fn same(&self) -> &str {
        self.same.get_or_init(|| same_wallet(&self.policy))
    }

    /// The policy's descriptor checksum; a lookup, kept by the policy.
    pub fn checksum(&self) -> &str {
        self.policy.checksum()
    }

    /// Replaces the policy, dropping the addresses and scripts kept from
    /// the one before.
    pub(crate) fn set_policy(&mut self, policy: WalletPolicy) {
        self.policy = policy;
        self.same = std::cell::OnceCell::new();
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

/// Whether a key that is not one of the wallet's spends it alone by the
/// key path (F132). A lookup: the key path was worked out when the
/// wallet was read. Its signatures are then counted against the script
/// path's own quorum, and its readiness never reads as a plain "Can
/// spend".
pub(crate) fn another_key_spends(wallet: &Wallet) -> bool {
    matches!(wallet.policy.key_path(), Some(KeyPath::Other(_)))
}

/// Whether a press that asked for a wallet whose key path another key
/// holds is run again once the hold confirms it (F132): only the presses
/// that do nothing before they add the wallet but read Files and work
/// names out, so that a second run is the first run with the wallet
/// added. Every other press (a bulk load from Files, a vault's records,
/// typed seeds, a backup flow's step, a boot import) has its wallet added
/// by the hold alone, with a toast.
pub fn replays(action: Action) -> bool {
    use crate::family::FamilyAction as F;
    matches!(
        action,
        Action::LoadWallet(_)
            | Action::RUse(_)
            | Action::RRebuild
            | Action::RestoreShares
            | Action::Family(F::UseWallet(_) | F::Rebuild)
    )
}

/// The danger line of a wallet whose key path a key not of its own
/// spends, naming that key in groups of four (F132).
pub fn key_path_danger(key: &str) -> String {
    fill(EN.wallet_key_path_other, &[&faraday_ui::grouped(key)])
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
    /// A wallet whose key path a key not of the wallet spends, waiting
    /// for the person to confirm it before it is added (F132): what
    /// [`Session::add_policy`] was asked to add. `None` while nothing
    /// waits.
    key_path_asked: Option<Box<KeyPathAsk>>,
    /// The descriptors of the wallets of that kind the person confirmed
    /// in this session, which [`Session::add_policy`] adds without asking
    /// again.
    key_path_confirmed: Vec<String>,
    /// A Coldcard export of two accounts waiting for one to be chosen, and the choices made.
    accounts: account::Accounts,
    /// The extended public keys worked out from loaded keys at the paths
    /// wallets name under their fingerprints ([`Session::key_refs`]), by
    /// network, key ([`master_id`]) and path. Each is a function of
    /// those three alone, so an entry is never wrong while it is kept,
    /// whatever keys and wallets come and go; it is derived once, the
    /// first time a wallet names the path, and dropped with the keys
    /// ([`Session::wipe`]).
    pub(crate) key_xpubs: std::cell::RefCell<Vec<crate::register::KeyXpub>>,
}

/// A wallet waiting for its key path to be confirmed: what
/// [`Session::add_policy`] was called with.
pub struct KeyPathAsk {
    /// The name it is added under.
    pub name: String,
    /// The wallet, read.
    pub policy: WalletPolicy,
    /// Where it came from, as [`Wallet::source`] has it.
    pub source: String,
    /// What that source is.
    pub origin: Origin,
    /// The press that asked for it, run again once the hold confirms it,
    /// so the flow that asked goes on as if nothing had been asked, with
    /// the Inbox's names and sizes as they were then
    /// ([`Faraday::inbox_print`]). `None` for a wallet that came in
    /// without a press, or by one that does not run twice the same
    /// ([`replays`]); that wallet is added by the hold itself.
    pub replay: Option<(Action, u64)>,
}

/// How a `tr()` wallet's key path line reads (F132): its text, and
/// whether it is the danger line, a key that is not one of the wallet's.
pub struct KeyPathLine {
    /// The line.
    pub text: String,
    /// A key not of the wallet spends everything alone.
    pub danger: bool,
}

impl Default for Session {
    fn default() -> Self {
        Session {
            keys: faraday_keys::Keys::on(START_NETWORK),
            wallets: Vec::new(),
            modes: crate::modes::STICK_MODES,
            key_marks: crate::codex32_correct::KeyMarks::default(),
            key_path_asked: None,
            key_path_confirmed: Vec::new(),
            accounts: account::Accounts::default(),
            key_xpubs: std::cell::RefCell::new(Vec::new()),
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

/// Whether `policy` is a MuSig2 wallet two of whose keys share a master
/// fingerprint: refused wherever a wallet is added.
pub(crate) fn musig_one_seed(policy: &WalletPolicy) -> bool {
    if policy.template() != Template::MuSig {
        return false;
    }
    let fps: Vec<_> = policy
        .keys()
        .iter()
        .filter_map(|k| k.fingerprint())
        .collect();
    fps.iter().enumerate().any(|(i, f)| fps[..i].contains(f))
}

impl Session {
    /// A session on `network`.
    pub fn on(network: Network) -> Session {
        Session {
            keys: faraday_keys::Keys::on(network),
            wallets: Vec::new(),
            modes: crate::modes::STICK_MODES,
            key_marks: crate::codex32_correct::KeyMarks::default(),
            key_path_asked: None,
            key_path_confirmed: Vec::new(),
            accounts: account::Accounts::default(),
            key_xpubs: std::cell::RefCell::new(Vec::new()),
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
    pub(crate) fn follow_psbt(&mut self, psbt: &Psbt) -> Result<(), Refusal> {
        self.follow(psbt_network(psbt, self.network()))
    }

    /// Costs what `read_wallet`, `account_of` and `add_policy` cost. Adds
    /// the wallet the text describes (of a Coldcard export of two
    /// accounts, the one chosen), under `name`, unless the same
    /// descriptor is already loaded. `origin`
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
        let read = read_wallet(c, text)?;
        let policy = self.account_of(c, text, read, (name, source, origin))?;
        self.add_policy(c, name, policy, source, origin)
    }

    /// Costs two key derivations per loaded key with a silent payments
    /// wallet's fingerprint, nothing for any other wallet.
    /// [`Session::add_wallet`] of a description read before (a vault's
    /// kept records): no parse. A silent payments wallet whose scan key or
    /// spend key is not what a loaded key with its fingerprint gives at
    /// its account is refused, and so is a MuSig2 wallet two of whose keys
    /// have one master fingerprint, loaded or not: the MuSig2 check
    /// matches one participant to each master fingerprint, so a second
    /// account of one seed never signs.
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
        if musig_one_seed(&policy) {
            return Err(Refusal::MuSigOneSeed);
        }
        let descriptor = same_wallet(&policy);
        if let Some(i) = self
            .wallets
            .iter()
            .position(|w| same_wallet(&w.policy) == descriptor)
        {
            return Ok(i);
        }
        // A wallet a key not of its own can spend alone is added only
        // once the person has confirmed it (F132): it waits, and the
        // sheet that asks reads it from here.
        if matches!(policy.key_path(), Some(KeyPath::Other(_)))
            && !self.key_path_confirmed.contains(&descriptor)
        {
            if self.key_path_asked.is_none() {
                self.key_path_asked = Some(Box::new(KeyPathAsk {
                    name: name.to_string(),
                    policy,
                    source: source.to_string(),
                    origin,
                    replay: None,
                }));
            }
            return Err(Refusal::KeyPathUnconfirmed);
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

    /// The wallet waiting for its key path to be confirmed, if one is.
    pub fn key_path_asked(&self) -> Option<&KeyPathAsk> {
        self.key_path_asked.as_deref()
    }

    /// The press `action` asked for the waiting wallet, if one waits and
    /// no press is noted for it yet.
    pub(crate) fn key_path_replay(&mut self, action: Action, inbox: u64) {
        if let Some(ask) = self.key_path_asked.as_mut() {
            ask.replay.get_or_insert((action, inbox));
        }
    }

    /// Writes the descriptor out. The person confirmed the waiting
    /// wallet's key path: the same wallet is added without asking again
    /// in this session, and the waiting wallet is handed back for its
    /// press to run again or, with none, to be added. `None` when none
    /// waits.
    pub(crate) fn key_path_confirm(&mut self) -> Option<KeyPathAsk> {
        let ask = self.key_path_asked.take()?;
        self.key_path_confirmed.push(same_wallet(&ask.policy));
        Some(*ask)
    }

    /// The waiting wallet is not added.
    pub(crate) fn key_path_drop(&mut self) {
        self.key_path_asked = None;
    }

    /// Who can spend `policy` by its key path, as a line (F132): `None`
    /// for a wallet with no script tree. A lookup: the key path was
    /// worked out when the policy was read.
    pub(crate) fn key_path_line(&self, policy: &WalletPolicy) -> Option<KeyPathLine> {
        Some(match policy.key_path()? {
            KeyPath::Unspendable => KeyPathLine {
                text: EN.wallet_key_path_none.to_string(),
                danger: false,
            },
            KeyPath::WalletKey(i) => {
                let fp = policy.keys().get(*i).and_then(|k| k.fingerprint());
                let holder = match fp {
                    Some(fp) => match self.keys.key_label(fp) {
                        Some(label) => key_line(fp, label),
                        None => fp_text(fp),
                    },
                    None => fill(EN.wallet_key_n, &[&(i + 1).to_string()]),
                };
                KeyPathLine {
                    text: fill(EN.wallet_key_path_alone, &[&holder]),
                    danger: false,
                }
            }
            KeyPath::Other(key) => KeyPathLine {
                text: key_path_danger(key),
                danger: true,
            },
        })
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
        self.key_xpubs.borrow_mut().clear();
    }

    /// The keys loaded that no wallet loaded uses, in the order they
    /// were added.
    pub(crate) fn loose_keys(&self) -> Vec<Fingerprint> {
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
            .or_else(|| another_key_spends(wallet).then(|| wallet.policy.leaf_quorum())?)
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

    /// Costs a key derivation (through `from_master`), and what
    /// `register::wallet_xpubs` costs. The keys as the inspection sees
    /// them, with the extended public keys each gives at its wallets'
    /// paths.
    pub(crate) fn key_refs(&self, c: &Compute) -> Vec<KeyRef> {
        let mut refs = self.keys.key_refs(c);
        crate::register::wallet_xpubs(self, c, &mut refs);
        refs
    }

    pub(crate) fn policies(&self) -> Vec<WalletPolicy> {
        self.wallets.iter().map(|w| w.policy.clone()).collect()
    }

    /// Costs what `deterministic_all` costs. The nonce lines of the signer
    /// check: `view` (`faraday_psbt::signer::nonce_view`) recomputed with
    /// every key held here, for `faraday_psbt::signer::with_nonces`.
    pub(crate) fn recompute_all(
        &self,
        c: &Compute,
        view: &Psbt,
    ) -> Vec<faraday_psbt::verify::Determinism> {
        self.keys.recompute_signatures_all(c, view)
    }

    /// Costs one derivation per signature, and what `key_refs` and
    /// `deterministic` cost, a few classifies
    /// per input, and, per final input, what [`Spend::refused`] costs twice
    /// over (the refusal and the rows), a taproot script-path one's
    /// interpreter run and signature checks included. The cost is per input, linear in the file
    /// (the sum over its inputs), with no file-wide bound. The nonce check at
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
        let signatures = faraday_psbt::verify::verify_signatures(psbt, &ctx);
        let reused = faraday_psbt::verify::repeated_nonces(psbt, &signatures);
        for input in signatures {
            for sig in input.signatures {
                // A row not checked, a leaf not run among them, gets no
                // verdict from a recompute that cannot match.
                let recomputed = made
                    .iter()
                    .find(|d| {
                        d.input == input.index
                            && d.key == sig.key
                            && d.leaf == sig.leaf
                            && d.signature == sig.bytes
                    })
                    .map(|d| d.mode)
                    .filter(|mode| mode.is_some() || !matches!(sig.verdict, Verdict::Unchecked(_)));
                let unmatched = match (recomputed, sig.fingerprint) {
                    (Some(_), _) => None,
                    (None, None) if sig.origin_left_out => Some(Unmatched::NotRun),
                    (None, None) => Some(Unmatched::NoOrigin),
                    (None, Some(_)) if sig.ours && sig.finalized => Some(Unmatched::Finalized),
                    (None, Some(_)) => None,
                };
                // Only a signature that is valid is looked at, and only a
                // key proved here is kept and compared: derived at its
                // stated place by a registered wallet's key or by a loaded
                // master. A file could otherwise fill the history with keys
                // of its own, or cost a derivation with a signature that
                // is false.
                let entry = crate::memory::nonces::NonceEntry::of_signature(&sig).filter(|_| {
                    sig.fingerprint.is_some_and(|fp| {
                        let Some(inner) = psbt.inner().inputs.get(input.index) else {
                            return false;
                        };
                        let Some((path, stated)) = faraday_psbt::verify::stated_place(inner, &sig)
                        else {
                            return false;
                        };
                        self.wallets
                            .iter()
                            .any(|w| w.derives(c, fp, &path, &stated))
                            || self.keys.position(fp).is_some_and(|pos| {
                                self.keys
                                    .xpub_at(c, pos, &path)
                                    .is_some_and(|x| stated.is(&x.public_key))
                            })
                    })
                });
                rows.push(NonceRow {
                    input: input.index,
                    name: sig.name(),
                    entry,
                    verdict: sig.verdict,
                    preimage: sig.preimage,
                    recomputed,
                    unmatched,
                });
            }
        }
        let mismatched = (0..psbt.inner().inputs.len())
            .filter(|i| faraday_psbt::verify::witness_mismatch(psbt.inner(), *i))
            .collect();
        let fields_refused = (0..psbt.inner().inputs.len())
            .filter(|i| {
                faraday_psbt::verify::witness_mismatch(psbt.inner(), *i)
                    && !faraday_psbt::verify::witness_shape_mismatch(psbt.inner(), *i)
            })
            .collect();
        let not_evaluated = (0..psbt.inner().inputs.len())
            .filter(|i| faraday_psbt::verify::not_evaluated(psbt.inner(), *i))
            .collect();
        let (unread, disagree) = faraday_psbt::verify::unread_counts(psbt.inner());
        NonceCheck {
            not_evaluated,
            fields_refused,
            rows,
            mismatched,
            reused,
            unread,
            disagree,
            tx: crate::memory::tx_hash(psbt),
            partials: Vec::new(),
            used_before: Vec::new(),
            history: crate::memory::nonces::History::Off,
            history_gen: 0,
        }
    }

    /// Costs a signature verification and an aggregate nonce per MuSig2
    /// input that carries partial signatures. The partial signatures of
    /// `psbt` that verify, as entries by input and the first eight hex
    /// characters of the participant's key, for [`NonceCheck::partials`];
    /// `inspection` is the spend's.
    pub fn nonce_partials(
        &self,
        c: &Compute,
        psbt: &Psbt,
        inspection: &faraday_psbt::Inspection,
    ) -> Vec<(usize, String, crate::memory::nonces::NonceEntry)> {
        let mut out = Vec::new();
        for (i, input) in inspection.inputs.iter().enumerate() {
            let Some(round) = input.musig.as_ref() else {
                continue;
            };
            for p in faraday_psbt::musig::verified_rounds(c, psbt, i, round) {
                let name: String = p.participant.serialize()[1..33]
                    .iter()
                    .map(|b| format!("{b:02x}"))
                    .collect::<String>()
                    .chars()
                    .take(8)
                    .collect();
                out.push((i, name, crate::memory::nonces::NonceEntry::of_partial(&p)));
            }
        }
        out
    }

    /// A copy for work off the main thread: every key's master and public
    /// facts and nothing else ([`faraday_keys::Keys::job_view`]), the wallets
    /// with what they kept ([`Wallet::job_copy`]), the modes and the extended public
    /// keys worked out so far. No words, no seeds, no wallet waiting for a
    /// key path.
    pub fn job_copy(&self) -> Session {
        Session {
            keys: self.keys.job_view(),
            wallets: self.wallets.iter().map(Wallet::job_copy).collect(),
            modes: self.modes,
            key_xpubs: std::cell::RefCell::new(self.key_xpubs.borrow().clone()),
            ..Session::default()
        }
    }

    /// What an inspection is worked out from beside the PSBT
    /// ([`InspectInputs`]). Lookups: each key's id and each wallet's
    /// checksum are kept.
    pub(crate) fn inspect_inputs(&self) -> InspectInputs {
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
        crate::review::name_scripts(faraday_psbt::inspect(c, psbt, &ctx))
    }

    /// Costs what `sign_with` costs. Signs `psbt` with every session key it
    /// names, past no danger. Returns the fingerprints that signed.
    pub fn sign(
        &self,
        c: &Compute,
        psbt: &mut Psbt,
        seed: &zeroize::Zeroizing<[u8; 32]>,
        musig: &mut Option<faraday_psbt::MusigSession>,
    ) -> Result<Vec<Fingerprint>, String> {
        self.sign_with(c, psbt, seed, musig, None, &[])
            .map(|s| s.by)
    }

    /// Costs what `key_refs`, `inspect` and the rest cost. Signs `psbt` with
    /// every session key it names, past the holdable dangers in `accepted`
    /// alone (F136), on this thread: the job of [`Session::sign_job`], run.
    /// `known` is as there.
    pub fn sign_with(
        &self,
        c: &Compute,
        psbt: &mut Psbt,
        seed: &zeroize::Zeroizing<[u8; 32]>,
        musig: &mut Option<faraday_psbt::MusigSession>,
        known: Option<&Inspection>,
        accepted: &[faraday_psbt::WarningKind],
    ) -> Result<Signed, String> {
        let job = self.sign_job(c, psbt, seed, musig, known, accepted)?;
        let (made, result) = crate::review::signed(musig, job.run(c));
        *psbt = made;
        result
    }
}

pub(crate) fn context<'a>(
    network: Network,
    refs: &'a [KeyRef],
    policies: &'a [WalletPolicy],
) -> Context<'a> {
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
    pub(crate) const NONCES: u8 = 9;
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
    pub(crate) fn has_cosigners(self) -> bool {
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
    /// Costs what `PsbtSeen::of` costs and, the first time a wallet is asked
    /// a place, a script derivation ([`Wallet::script_at`]). The loaded wallet whose script
    /// the transaction's first input spends, found by re-deriving each
    /// wallet's script at the paths the input's own key origins state.
    pub(crate) fn wallet_for(&self, c: &Compute, psbt: &Psbt) -> Option<usize> {
        self.spends_from(c, &PsbtSeen::of(psbt))
    }

    /// Costs what `script_at` costs. [`Session::wallet_for`] of a PSBT read
    /// before ([`PsbtSeen`]): lookups once each wallet keeps its scripts at
    /// the input's places, so a frame may ask it of every file in Files.
    pub(crate) fn spends_from(&self, c: &Compute, seen: &PsbtSeen) -> Option<usize> {
        let (spk, places) = seen.first.as_ref()?;
        self.wallet_at(c, spk, places)
    }

    /// Costs what `script_at` costs. The loaded wallet whose script input
    /// `i` of `inner` spends, by the places that input's own key origins
    /// state: each input has its own wallet.
    pub(crate) fn wallet_of_input(
        &self,
        c: &Compute,
        inner: &faraday_bip::bitcoin::Psbt,
        i: usize,
    ) -> Option<usize> {
        let (spk, places) = input_place(inner, i)?;
        self.wallet_at(c, &spk, &places)
    }

    /// Costs what `script_at` costs. The first loaded wallet whose script at
    /// one of `places` is `spk`.
    fn wallet_at(
        &self,
        c: &Compute,
        spk: &faraday_bip::bitcoin::ScriptBuf,
        places: &[(bool, u32)],
    ) -> Option<usize> {
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
    /// Signatures on all its inputs: partial, taproot and those in final
    /// fields.
    pub sigs: usize,
    /// It carries a signature: a partial one, a taproot one or an input
    /// finalized.
    pub signed: bool,
    /// The masters a key origin names on an input not yet final whose
    /// key has not signed that input, each once.
    pub unsigned_by: Vec<Fingerprint>,
    /// The first input's script, and each (change, index) its key
    /// origins state.
    first: Option<(faraday_bip::bitcoin::ScriptBuf, Vec<(bool, u32)>)>,
}

impl PsbtSeen {
    /// What `psbt` says. Costs a txid, a classify per input and, per final
    /// input, what [`signing_state`] costs: its signature checks, through
    /// `accepts_tap` for a taproot script-path spend. An event runs it
    /// (`Session::wallet_for`), never a frame.
    pub fn of(psbt: &Psbt) -> PsbtSeen {
        let inner = psbt.inner();
        let sigs = faraday_psbt::verify::count_signatures(psbt);
        let first = input_place(inner, 0);
        let (signed, unsigned_by) = signing_state(inner);
        PsbtSeen {
            txid: inner.unsigned_tx.compute_txid(),
            sigs,
            signed,
            unsigned_by,
            first,
        }
    }

    /// Whether it is checked rather than signed: it carries a signature
    /// and no key `held` holds is left to sign it (`docs/FLOWS.md`,
    /// Nonce check). A lookup per master its origins name.
    pub fn signed_elsewhere(&self, held: impl Fn(Fingerprint) -> bool) -> bool {
        self.signed && !self.unsigned_by.iter().any(|fp| held(*fp))
    }
}

/// Costs a classify. Input `i`'s spent script, and each `(change, index)`
/// its key origins state.
fn input_place(
    inner: &faraday_bip::bitcoin::Psbt,
    i: usize,
) -> Option<(faraday_bip::bitcoin::ScriptBuf, Vec<(bool, u32)>)> {
    let input = inner.inputs.get(i)?;
    let spk = faraday_psbt::verify::spent_output_of(inner, i)?.script_pubkey;
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
}

/// Whether a PSBT carries a signature, and the masters its key origins
/// name on an input not yet final whose key has not signed that input
/// ([`PsbtSeen::unsigned_by`]). A lookup per field and, per final input,
/// what `faraday_psbt::verify::witness_mismatch` costs: a classify and the
/// interpreter's signature checks (one per signature the script asks for),
/// a taproot script-path spend of a miniscript leaf included
/// (`faraday_psbt::finalize::accepts_tap`, a signature hash and a Schnorr
/// check each), and a tweak for a taproot key-path one.
fn signing_state(inner: &faraday_bip::bitcoin::Psbt) -> (bool, Vec<Fingerprint>) {
    let mut signed = false;
    let mut unsigned_by: Vec<Fingerprint> = Vec::new();
    for (index, input) in inner.inputs.iter().enumerate() {
        if input.final_script_sig.is_some() || input.final_script_witness.is_some() {
            // A `scriptSig` or witness that does not match its output signs
            // nothing.
            signed |= !faraday_psbt::verify::witness_mismatch(inner, index);
            continue;
        }
        signed |= !input.partial_sigs.is_empty()
            || input.tap_key_sig.is_some()
            || !input.tap_script_sigs.is_empty();
        let mut waiting: Vec<Fingerprint> = Vec::new();
        for (pk, (fp, _)) in &input.bip32_derivation {
            if !input
                .partial_sigs
                .contains_key(&faraday_bip::bitcoin::PublicKey::new(*pk))
            {
                waiting.push(Fingerprint::from(*fp));
            }
        }
        for (xonly, (leaves, (fp, _))) in &input.tap_key_origins {
            let key_path = input.tap_internal_key == Some(*xonly) && input.tap_key_sig.is_some();
            let leaf = leaves
                .iter()
                .any(|l| input.tap_script_sigs.contains_key(&(*xonly, *l)));
            if !key_path && !leaf {
                waiting.push(Fingerprint::from(*fp));
            }
        }
        for fp in waiting {
            if !unsigned_by.contains(&fp) {
                unsigned_by.push(fp);
            }
        }
    }
    (signed, unsigned_by)
}

/// Costs a txid per input with a previous transaction. Whether the
/// transaction's id is fixed before it is signed: every
/// input spends a native SegWit or Taproot output, so no signature lands
/// in the part of the transaction its id is taken over.
pub(crate) fn txid_known(psbt: &Psbt) -> bool {
    let inner = psbt.inner();
    faraday_psbt::verify::spent_outputs(inner)
        .iter()
        .all(|out| {
            out.as_ref().is_some_and(|o| {
                let s = &o.script_pubkey;
                s.is_p2wpkh() || s.is_p2wsh() || s.is_p2tr()
            })
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
        .or_else(|| another_key_spends(wallet).then(|| wallet.policy.leaf_quorum())?)
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
    /// `docs/INTEROP.md` §5, Electrum wallet files): a secret. It loads as the wallet its
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
    /// A KeePass database of version 3 or older, sealed under its
    /// passphrase: named, not opened.
    Kdbx,
    /// A file that begins as a KeePass database does and is not a vault
    /// or an older one: KDBX 5, or KDBX 4 of a length no vault has, or cut
    /// short. Sealed; named, not opened.
    KdbxOther,
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

/// The most a plain text file may hold to be kept as a note.
pub(crate) const TEXT_MAX: usize = 64 * 1024;

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
    if faraday_intake::is_vault(bytes) {
        return FileKind::Vault.into();
    }
    if bytes.starts_with(b"%PDF-") {
        return FileKind::Pdf.into();
    }
    if faraday_intake::is_older_kdbx(bytes) {
        return FileKind::Kdbx.into();
    }
    if faraday_intake::is_kdbx(bytes) {
        return FileKind::KdbxOther.into();
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
#[derive(Clone)]
pub struct Spend {
    /// The PSBT as it stands: every signature collected so far.
    pub psbt: Psbt,
    /// The file it came from.
    pub source: String,
    /// The unsigned transaction's id, which every collected copy must share.
    pub txid: Txid,
    /// Keys of this session that signed it here.
    pub signed_here: Vec<Fingerprint>,
    /// The MuSig2 partial signatures this device wrote in this session: the
    /// only entries in the file it reads as its own (F220).
    pub wrote: Vec<Written>,
    /// Fingerprints whose signatures arrived from other devices, with the
    /// file each came in.
    pub collected: Vec<(Fingerprint, String)>,
    /// The finished transaction, once there are enough signatures.
    pub finished: Option<Transaction>,
    /// The nonce check refuses the transaction: a nonce used twice in it, or
    /// before on another message in an open vault. Nothing is finished,
    /// saved or shown as a code while it holds ([`Faraday::nonce_spend_sync`]).
    pub nonce_hold: bool,
    /// The PSBT as read when it came as version 2 (BIP-370), `psbt` its
    /// version 0 form (`wallet/v2.rs`). Set as the spend starts, never
    /// changed: `collect` keeps `psbt` of its transaction.
    pub v2: Option<faraday_psbt::v2::PsbtV2>,
}

/// A MuSig2 partial signature this device wrote: the input, the
/// participant's key and the partial's bytes, all public.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Written {
    /// The input it is on.
    pub input: usize,
    /// The participant it is for.
    pub participant: faraday_bip::bitcoin::secp256k1::PublicKey,
    /// The partial signature.
    pub partial: faraday_bip::musig::PartialSig,
}

/// What one participant of a MuSig2 wallet is on one open input
/// ([`Spend::signing`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Partial {
    /// The input carries the aggregate signature, verified under the
    /// output key: everyone's.
    Aggregate,
    /// This device wrote the partial signature, and it verifies.
    Here,
    /// A partial signature that verifies is in the file and this device
    /// cannot confirm it: the file's word.
    Word,
    /// A partial signature is in the file and does not verify.
    Fails,
    /// No partial signature is in the file, or this device holds the
    /// participant's secret nonce, which proves any entry false: still to
    /// sign.
    Open,
}

/// The masters of one input, from its participants' [`Partial`]s: a master
/// is only as far along as its worst key (two accounts of one master are
/// two participants).
#[derive(Debug, Clone, Default)]
struct Tally {
    counted: Vec<Fingerprint>,
    to_sign: Vec<Fingerprint>,
    in_file: Vec<Fingerprint>,
    failed: Vec<Fingerprint>,
}

impl Tally {
    fn of(states: &[(Fingerprint, Partial)]) -> Tally {
        let mut t = Tally::default();
        let mut seen: Vec<Fingerprint> = Vec::new();
        for (fp, _) in states {
            if seen.contains(fp) {
                continue;
            }
            seen.push(*fp);
            let own: Vec<Partial> = states
                .iter()
                .filter(|(f, _)| f == fp)
                .map(|(_, p)| *p)
                .collect();
            if own.contains(&Partial::Fails) {
                t.failed.push(*fp);
            }
            if own
                .iter()
                .any(|p| matches!(p, Partial::Open | Partial::Fails))
            {
                t.to_sign.push(*fp);
            } else if own.contains(&Partial::Word) {
                t.in_file.push(*fp);
            } else {
                t.counted.push(*fp);
            }
        }
        t
    }
}

/// What [`Spend::signing`] says of a spend.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Signing {
    /// The masters with a signature counted on every open input.
    pub signers: Vec<Fingerprint>,
    /// The masters still to sign.
    pub to_sign: Vec<Fingerprint>,
    /// The masters whose partial signature is in the file, verifies, and
    /// is not this device's own: the file's word, not counted.
    pub in_file: Vec<Fingerprint>,
    /// The masters, still to sign, with a partial signature in the file
    /// that does not verify.
    pub failed: Vec<Fingerprint>,
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
            wrote: Vec::new(),
            collected: Vec::new(),
            finished: None,
            nonce_hold: false,
            v2: None,
        }
    }

    /// Costs an inspection of the PSBT, but for MuSig2. Fingerprints with a
    /// valid signature on the first input; for a MuSig2 wallet, the
    /// participants whose partial signature is confirmed on it (this
    /// device's own, or the aggregate verifying).
    pub fn signers(&self, c: &Compute, session: &Session) -> Vec<Fingerprint> {
        self.signers_by(c, session, None)
    }

    /// Costs what `wallet_for` and `inspect` cost, and a key derivation per
    /// signature where the wallet is known. [`Spend::signers`], read from
    /// `inspection` of this PSBT when one is given (the spend's kept one),
    /// else inspected now. A master counts once, for a key of the script
    /// that signed (the row's origin says nothing else, [`faraday_psbt::verify`]),
    /// and, where a wallet holds the spent script, only if that wallet's key
    /// for the master derives to it. It counts only if it signed every input
    /// not yet final.
    pub fn signers_by(
        &self,
        c: &Compute,
        session: &Session,
        inspection: Option<&Inspection>,
    ) -> Vec<Fingerprint> {
        self.signing(c, session, inspection, None).signers
    }

    /// Costs a scan of the inspection's signatures. Whether input `i` carries
    /// the aggregate signature of the MuSig2 wallet's key, verified under
    /// the input's own output key: every participant's.
    fn musig_verified(
        inner: &faraday_bip::bitcoin::Psbt,
        inspection: &Inspection,
        i: usize,
    ) -> bool {
        let output_key = faraday_psbt::verify::spent_output_of(inner, i)
            .filter(|o| o.script_pubkey.is_p2tr())
            .and_then(|o| {
                faraday_bip::bitcoin::secp256k1::XOnlyPublicKey::from_slice(
                    &o.script_pubkey.as_bytes()[2..],
                )
                .ok()
            })
            .map(faraday_psbt::verify::SigKey::Schnorr);
        inner.inputs[i].tap_key_sig.is_some()
            && output_key.is_some()
            && inspection.present_signatures().any(|s| {
                s.input == i
                    && Some(&s.key) == output_key.as_ref()
                    && s.leaf.is_none()
                    && !s.finalized
                    && s.kind == faraday_psbt::SigKind::Schnorr
                    && s.verdict == Verdict::Valid
            })
    }

    /// Costs a BIP-327 `PartialSigVerify` per partial signature of input
    /// `i` and its round's aggregate nonce once
    /// ([`faraday_psbt::musig::verified_partials`]), and a scan of the
    /// inspection's signatures; nothing where the input carries no partial
    /// signature. What each participant of the MuSig2 wallet `w` is on the
    /// open input `i`, by participant key ([`Partial`]): a partial in the
    /// file is the file's word, never proof, because the online side writes
    /// every nonce and so can build one that verifies. Three things prove a
    /// signature: the aggregate verifying, this device having written that
    /// very partial on that input (`wrote`), and this device holding the participant's
    /// secret nonce, which proves any entry for it false.
    fn musig_states(
        &self,
        c: &Compute,
        inspection: &Inspection,
        view: Option<&faraday_psbt::MusigSessionView>,
        w: &Wallet,
        i: usize,
    ) -> Vec<(Fingerprint, Partial)> {
        let keys = w
            .policy
            .keys()
            .iter()
            .filter_map(|k| k.fingerprint().map(|fp| (fp, k.xpub().public_key)));
        let inner = self.psbt.inner();
        if Self::musig_verified(inner, inspection, i) {
            return keys.map(|(fp, _)| (fp, Partial::Aggregate)).collect();
        }
        let round = inspection.inputs.get(i).and_then(|x| x.musig.as_ref());
        // The entries the signer and the aggregation read: one per
        // participant, under the form of the aggregate key they prefer.
        let entries = match (inner.inputs.get(i), round) {
            (Some(x), Some(round)) => faraday_psbt::musig::picked_partials(x, round),
            _ => Vec::new(),
        };
        let verified = match round {
            Some(round) if !entries.is_empty() => {
                faraday_psbt::musig::verified_partials(c, &self.psbt, i, round)
            }
            _ => Vec::new(),
        };
        keys.map(|(fp, pk)| {
            let held = view.is_some_and(|v| v.holds(self.txid, i, &pk));
            let entry = entries.iter().find(|(p, _)| *p == pk).map(|(_, v)| *v);
            let state = if held {
                Partial::Open
            } else if verified.contains(&pk) {
                // Here only for the very entry this device wrote.
                let mine = entry.is_some_and(|v| {
                    self.wrote
                        .iter()
                        .any(|w| w.input == i && w.participant == pk && w.partial == v)
                });
                if mine { Partial::Here } else { Partial::Word }
            } else if entry.is_some() {
                Partial::Fails
            } else {
                Partial::Open
            };
            (fp, state)
        })
        .collect()
    }

    /// What a signing leaves on the spend: `psbt` is its PSBT, and the
    /// fingerprints that signed are noted, unless the pass only opened a
    /// MuSig2 round (`nonce_only`: a nonce and no signature). Notes the
    /// partial signatures first, which the signers' lists read.
    pub fn take_signed(&mut self, psbt: Psbt, fps: Vec<Fingerprint>, nonce_only: bool) {
        if nonce_only {
            self.psbt = psbt;
            return;
        }
        self.note_written(&psbt);
        self.psbt = psbt;
        for fp in fps {
            if !self.signed_here.contains(&fp) {
                self.signed_here.push(fp);
            }
        }
    }

    /// Notes the MuSig2 partial signatures `now` holds that this spend's
    /// PSBT does not hold as they are: what a signing here is about to
    /// write. Call before `now` replaces the PSBT.
    pub(crate) fn note_written(&mut self, now: &Psbt) {
        for (i, input) in now.inner().inputs.iter().enumerate() {
            let old = self
                .psbt
                .inner()
                .inputs
                .get(i)
                .and_then(|x| faraday_psbt::musig::partial_sigs(x).ok())
                .unwrap_or_default();
            for e in faraday_psbt::musig::partial_sigs(input).unwrap_or_default() {
                let known = old.iter().any(|o| {
                    o.participant == e.participant
                        && o.aggregate == e.aggregate
                        && o.value == e.value
                });
                let w = Written {
                    input: i,
                    participant: e.participant,
                    partial: e.value,
                };
                if !known && !self.wrote.contains(&w) {
                    self.wrote.push(w);
                }
            }
        }
    }

    /// Costs what `signers_by` and `wallet_for` cost. Aggregates the
    /// partial signatures of every open input of a MuSig2 wallet that has
    /// one from every participant ([`faraday_psbt::musig::aggregate`]),
    /// whatever key this device holds: the signature is written only when
    /// it verifies under the input's output key. Whether any input gained
    /// its signature.
    pub fn aggregate_musig(&mut self, c: &Compute, inspection: &Inspection) -> bool {
        let mut changed = false;
        for (i, x) in inspection.inputs.iter().enumerate() {
            let Some(round) = x.musig.as_ref() else {
                continue;
            };
            let Some(input) = self.psbt.inner().inputs.get(i) else {
                continue;
            };
            // A signature already there stops the aggregate only if it
            // verifies: a false one is replaced.
            if (input.tap_key_sig.is_some()
                && Self::musig_verified(self.psbt.inner(), inspection, i))
                || input.final_script_sig.is_some()
                || input.final_script_witness.is_some()
            {
                continue;
            }
            let entries = faraday_psbt::musig::partial_sigs(input).unwrap_or_default();
            if !round
                .participants
                .iter()
                .all(|p| entries.iter().any(|e| e.participant == *p))
            {
                continue;
            }
            if let Some(sig) = faraday_psbt::musig::aggregate(c, &self.psbt, i, round) {
                self.psbt.inner_mut().inputs[i].tap_key_sig = Some(sig);
                changed = true;
            }
        }
        changed
    }

    /// Costs what `derives` costs per signature row. Adds to `per_input` the
    /// master of each valid signature row on an open input that no MuSig2
    /// wallet holds, once it is proven (see [`Spend::signing`]).
    fn count_rows(
        &self,
        c: &Compute,
        session: &Session,
        inspection: &Inspection,
        open: &[usize],
        owners: &[Option<usize>],
        per_input: &mut [Vec<Fingerprint>],
    ) {
        let inner = self.psbt.inner();
        let is_musig = |w: usize| Kind::of(&session.wallets[w].policy) == Kind::MuSig;
        for s in inspection
            .present_signatures()
            .filter(|s| s.verdict == Verdict::Valid)
        {
            let (Some(fp), Some(input)) = (s.fingerprint, inner.inputs.get(s.input)) else {
                continue;
            };
            // Only an open input of a wallet other than MuSig2's counts a row.
            if !open.contains(&s.input) || owners[s.input].is_some_and(is_musig) {
                continue;
            }
            if per_input[s.input].contains(&fp) {
                continue;
            }
            let Some((path, stated)) = faraday_psbt::verify::stated_place(input, s) else {
                continue;
            };
            let owner = owners[s.input].and_then(|w| session.wallets.get(w));
            let proven = match owner {
                Some(w) => w.derives(c, fp, &path, &stated),
                None => match session.keys.position(fp) {
                    Some(pos) => session
                        .keys
                        .xpub_at(c, pos, &path)
                        .is_some_and(|x| stated.is(&x.public_key)),
                    // A master a loaded wallet names derives from the xpub
                    // that wallet holds; one none names stays word-only.
                    None => {
                        let mut named = session
                            .wallets
                            .iter()
                            .filter_map(|w| w.master_proof(c, fp, &path, &stated))
                            .peekable();
                        named.peek().is_none() || named.any(|ok| ok)
                    }
                },
            };
            if proven {
                per_input[s.input].push(fp);
            }
        }
    }

    /// Costs what [`Spend::signers_by`] costs, and what `musig_states`
    /// costs per open input of a MuSig2 wallet. The signers, and the masters
    /// still to sign: one the PSBT names on an input not yet final that
    /// has no signature counted there, per input (so two inputs of two
    /// owners leave neither owner "still to sign" once each has signed its
    /// own). A key whose origin is stated falsely has none counted.
    ///
    /// An origin is the online side's word, so a master counts only on
    /// proof: where a wallet holds the spent script, its key for the master
    /// derives to the key that signed ([`Wallet::derives`]); else, for a
    /// master this device holds, the held master derives it at the stated
    /// place. A cosigner's master with no wallet found stays word-only.
    ///
    /// On a MuSig2 input a master counts, per participant key, for the
    /// aggregate verifying or a partial this device wrote; one whose
    /// partial is only in the file is in `in_file`, and one whose partial
    /// does not verify is also `failed` and still to sign. `musig` is the
    /// round this device holds, if any: a participant whose secret nonce it
    /// holds is still to sign whatever the file carries for it.
    pub fn signing(
        &self,
        c: &Compute,
        session: &Session,
        inspection: Option<&Inspection>,
        musig: Option<&faraday_psbt::MusigSessionView>,
    ) -> Signing {
        let own;
        let inspection = match inspection {
            Some(i) => i,
            None => {
                own = session.inspect(c, &self.psbt);
                &own
            }
        };
        let inner = self.psbt.inner();
        let open: Vec<usize> = (0..inner.inputs.len())
            .filter(|i| {
                let input = &inner.inputs[*i];
                input.final_script_sig.is_none() && input.final_script_witness.is_none()
            })
            .collect();
        // Each open input's own wallet: the one that holds the script it
        // spends, by the places its own origins state. An input of no
        // known owner has none.
        let owners: Vec<Option<usize>> = (0..inner.inputs.len())
            .map(|i| {
                open.contains(&i)
                    .then(|| session.wallet_of_input(c, inner, i))
                    .flatten()
            })
            .collect();
        let is_musig = |w: usize| Kind::of(&session.wallets[w].policy) == Kind::MuSig;
        let mut per_input: Vec<Vec<Fingerprint>> = vec![Vec::new(); inner.inputs.len()];
        let mut tallies: Vec<Option<Tally>> = vec![None; inner.inputs.len()];
        for i in &open {
            // An input of a MuSig2 wallet is signed by its participants,
            // each by its own key, never by a row's stated key.
            if let Some(w) = owners[*i].filter(|w| is_musig(*w)) {
                let t =
                    Tally::of(&self.musig_states(c, inspection, musig, &session.wallets[w], *i));
                per_input[*i] = t.counted.clone();
                tallies[*i] = Some(t);
            }
        }
        self.count_rows(c, session, inspection, &open, &owners, &mut per_input);
        let signers = match open.first() {
            Some(first) => per_input[*first]
                .iter()
                .filter(|fp| open.iter().all(|i| per_input[*i].contains(fp)))
                .copied()
                .collect(),
            None => Vec::new(),
        };
        let mut to_sign: Vec<Fingerprint> = Vec::new();
        let mut in_file: Vec<Fingerprint> = Vec::new();
        let mut failed: Vec<Fingerprint> = Vec::new();
        for i in &open {
            let input = &inner.inputs[*i];
            if let Some(t) = &tallies[*i] {
                // The wallet's participants, not the aggregate key's origin.
                for fp in &t.to_sign {
                    if !to_sign.contains(fp) {
                        to_sign.push(*fp);
                    }
                }
                for (from, into) in [(&t.in_file, &mut in_file), (&t.failed, &mut failed)] {
                    for fp in from {
                        if !into.contains(fp) {
                            into.push(*fp);
                        }
                    }
                }
                continue;
            }
            let named = input
                .bip32_derivation
                .values()
                .map(|(fp, _)| Fingerprint::from(*fp))
                .chain(
                    input
                        .tap_key_origins
                        .values()
                        .map(|(_, (fp, _))| Fingerprint::from(*fp)),
                );
            for fp in named {
                if !per_input[*i].contains(&fp) && !to_sign.contains(&fp) {
                    to_sign.push(fp);
                }
            }
        }
        // A master with a partial in the file on one input and still to
        // sign on another is still to sign, not "in the file".
        in_file.retain(|fp| !signers.contains(fp) && !to_sign.contains(fp));
        failed.retain(|fp| !signers.contains(fp));
        Signing {
            signers,
            to_sign,
            in_file,
            failed,
        }
    }

    /// Costs what [`Spend::signing`] costs. The masters with a signature
    /// counted or a partial in the file, which is what a file added.
    fn counted(
        &self,
        c: &Compute,
        session: &Session,
        inspection: Option<&Inspection>,
    ) -> Vec<Fingerprint> {
        let mut s = self.signing(c, session, inspection, None);
        s.signers.extend(s.in_file);
        s.signers
    }

    /// Costs what `read_psbt`, `signers` and `inspect` cost, what
    /// [`Spend::refused`] costs for the file read, and what
    /// `keep_own_over_file` costs per input. Adds the
    /// signatures another device put on a copy of this transaction. A PSBT
    /// for any other transaction is refused, never merged, and so is one
    /// that asks another sighash of an input, or carries a signature made
    /// under a sighash other than the one the input asks for: a good
    /// signature that answers another question. Returns the fingerprints
    /// it added.
    pub fn collect(
        &mut self,
        c: &Compute,
        session: &Session,
        bytes: &[u8],
        source: &str,
    ) -> Result<Vec<Fingerprint>, String> {
        let other = (read_psbt(c, bytes)).ok_or(EN.wallet_not_a_psbt)?;
        if other.unsigned_tx().compute_txid() != self.txid {
            return Err(fill(EN.wallet_psbt_other_transaction, &[source]));
        }
        // An input's sighash field: none written, SIGHASH_ALL and taproot's
        // DEFAULT ask the same.
        let asked = |t: Option<faraday_bip::bitcoin::psbt::PsbtSighashType>| {
            t.map(|t| t.to_u32()).filter(|&n| n > 1)
        };
        let asks_other = other
            .inner()
            .inputs
            .iter()
            .zip(&self.psbt.inner().inputs)
            .any(|(o, s)| asked(o.sighash_type) != asked(s.sighash_type));
        if asks_other {
            return Err(fill(EN.wallet_sighash_field_other, &[source]));
        }
        // The incoming file on its own, so a failing witness in it, and
        // none in the spend's own PSBT, is what it is blamed for.
        if let Some(why) = file_witness_refusal(other.inner(), source) {
            return Err(why);
        }
        // An own input still unread is left to the merged PSBT below: the
        // file may state the output it spends.
        if let Some(i) = (0..self.psbt.inner().inputs.len())
            .find(|i| faraday_psbt::verify::witness_shape_mismatch(self.psbt.inner(), *i))
        {
            return Err(fill(EN.psbt_warn_witness_mismatch, &[&i.to_string()]));
        }
        // A file that lies is refused and named, whatever the spend's own
        // copy holds: an input of it that states something false on its
        // own, or a `witness_utxo` that differs from the output of a
        // previous transaction of the right txid the spend holds. The
        // spend's own PSBT stays as it was.
        if let Some((i, bare)) = file_lies(self.psbt.inner(), other.inner()) {
            return Err(lie_refusal(source, i, bare));
        }
        let before = self.counted(c, session, None);
        let mut merged = self.psbt.inner().clone();
        let incoming = other.into_inner();
        let file_inputs = incoming.inputs.clone();
        // `Input::combine` drops one of `non_witness_utxo` and
        // `witness_utxo` where it takes the other, which would let a file
        // replace the previous transaction by an amount of its own, or
        // hide a disagreement. Both are kept, own first, except that a
        // previous transaction of the right txid beats one of another txid
        // from either side, its hash proving it; a bare `witness_utxo`
        // proves nothing and repairs nothing. An input that then states
        // something false is refused below.
        let stated: Vec<_> = merged
            .inputs
            .iter()
            .zip(&incoming.inputs)
            .zip(&merged.unsigned_tx.input)
            .map(|((own, file), txin)| {
                let right = |t: &&Transaction| t.compute_txid() == txin.previous_output.txid;
                let prev = match (&own.non_witness_utxo, &file.non_witness_utxo) {
                    (Some(a), _) if right(&a) => Some(a.clone()),
                    (_, Some(b)) if right(&b) => Some(b.clone()),
                    (a, b) => a.clone().or_else(|| b.clone()),
                };
                (
                    prev,
                    own.witness_utxo
                        .clone()
                        .or_else(|| file.witness_utxo.clone()),
                )
            })
            .collect();
        let own_false: Vec<bool> = (0..merged.inputs.len())
            .map(|i| faraday_psbt::verify::states_false(&merged, i))
            .collect();
        // The spend's own inputs as they were: what a file brings for a
        // script not run must not replace what the spend holds.
        let own_inputs = merged.inputs.clone();
        merged
            .combine(incoming)
            .map_err(|e| fill(EN.wallet_cannot_combine, &[&e.to_string()]))?;
        for (input, (prev, witness)) in merged.inputs.iter_mut().zip(stated) {
            input.non_witness_utxo = prev;
            input.witness_utxo = witness;
        }
        keep_own_over_file(&mut merged, &own_inputs, &file_inputs)
            .map_err(|c| c.refusal(source))?;
        // What is left of a false statement after the merge is the spend's
        // own: a file that lies was refused above, so this is an amount
        // of the spend's own that the honest file's previous transaction
        // contradicts. The reason is the spend's, naming no file, and the
        // spend's own PSBT stays as it was.
        if let Some(i) = (0..merged.inputs.len())
            .find(|&i| !own_false[i] && faraday_psbt::verify::states_false(&merged, i))
        {
            return Err(fill(EN.psbt_final_prev_disagree, &[&i.to_string()]));
        }
        // `combine` keeps the fields it has no type for, MuSig2's among them.
        let merged = Psbt::from(merged);
        // A final input that cannot stand, rows or none, is no signing: the
        // spend's own PSBT holding one blames no file.
        match refusal_of(merged.inner()) {
            Some(FinalRefusal::Signature(_)) => return Err(fill(EN.wallet_a_signature, &[source])),
            Some(FinalRefusal::Text(why)) => return Err(why),
            None => {}
        }
        let inspection = session.inspect(c, &merged);
        if inspection
            .present_signatures()
            .any(|s| s.verdict.is_invalid())
        {
            return Err(fill(EN.wallet_a_signature, &[source]));
        }
        if inspection
            .warnings
            .iter()
            .any(|w| matches!(w.kind, faraday_psbt::WarningKind::SighashMismatch { .. }))
        {
            return Err(fill(EN.wallet_sighash_other, &[source]));
        }
        self.psbt = merged;
        let added: Vec<Fingerprint> = (self.counted(c, session, Some(&inspection)))
            .into_iter()
            .filter(|fp| !before.contains(fp))
            .collect();
        for fp in &added {
            self.collected.push((*fp, source.to_string()));
        }
        Ok(added)
    }

    /// Costs a classify per input, final or not, a tweak per final taproot
    /// witness and, per final input, the interpreter's signature checks
    /// (a taproot leaf's over the annex-aware message) and up to 201
    /// signature verifications, each against the keys of its script
    /// (`faraday_psbt::verify::final_invalid`; an input over consensus's
    /// limits verifies none). Why no transaction is finished from `psbt`: its
    /// first input whose finalized `scriptSig` or witness does not match
    /// its output, that
    /// states an output its previous transaction does not, that is final
    /// with an output that cannot be read, or whose previous transaction
    /// is not the one it spends or lacks the output, final or not. A PSBT
    /// read with one reaches `finish` without `collect`. Drawing reads
    /// `SpendState::refused`, kept from this, and not this.
    pub fn refused(&self) -> Option<String> {
        refusal_of(self.psbt.inner()).map(FinalRefusal::text)
    }

    /// Costs what [`Spend::refused`] costs, and a finalize of a copy.
    /// Finishes the transaction when it has enough signatures.
    pub fn finish(&mut self) -> Result<(), String> {
        if self.nonce_hold {
            return Err(EN.spend_nonce_held.to_string());
        }
        if let Some(why) = self.refused() {
            return Err(why);
        }
        let mut copy = self.psbt.clone();
        match faraday_psbt::finalize(&mut copy) {
            Ok(Some(tx)) => {
                self.finished = Some(tx);
                Ok(())
            }
            Ok(None) => Err(EN.wallet_not_enough_signatures.to_string()),
            Err(e) => Err(crate::review::sign_error_text(&e)),
        }
    }

    /// Why the transaction cannot be finished yet, when it cannot.
    /// `refused` is what [`Spend::refused`] said for this PSBT, kept where
    /// the spend changes (`SpendState::refused`): a lookup, and costly
    /// work of the finalize of a copy.
    pub fn finish_reason(&self, refused: Option<&str>) -> Option<String> {
        if self.nonce_hold {
            return Some(EN.spend_nonce_held.to_string());
        }
        if let Some(why) = refused {
            return Some(why.to_string());
        }
        let mut copy = self.psbt.clone();
        match faraday_psbt::finalize(&mut copy) {
            Ok(Some(_)) => None,
            Ok(None) => Some(EN.wallet_script_unsatisfied.to_string()),
            Err(e) => Some(crate::review::sign_error_text(&e)),
        }
    }

    /// The finished transaction as hex.
    pub fn finished_hex(&self) -> Option<String> {
        self.finished.as_ref().map(serialize_hex)
    }
}

/// Costs a classify per input. The first input of `file` that lies to the
/// spend `own`, whatever `own` holds there, and whether it is a bare
/// `witness_utxo` that differs (`true`): it states something false on
/// its own, or its `witness_utxo` differs from the output of a previous
/// transaction of the right txid that `own` holds, or from the
/// `witness_utxo` `own` states where neither side holds a previous
/// transaction of the right txid (a bare one proves nothing: the spend
/// keeps its value and the file is refused, neither proven).
fn file_lies(
    own: &faraday_bip::bitcoin::Psbt,
    file: &faraday_bip::bitcoin::Psbt,
) -> Option<(usize, bool)> {
    (0..file.inputs.len().min(own.inputs.len())).find_map(|i| {
        let txin = &own.unsigned_tx.input[i];
        // A previous transaction of the right txid proves the output.
        let proven = |p: &faraday_bip::bitcoin::Psbt| {
            p.inputs[i]
                .non_witness_utxo
                .as_ref()
                .is_some_and(|t| t.compute_txid() == txin.previous_output.txid)
        };
        let own_output = own.inputs[i]
            .non_witness_utxo
            .as_ref()
            .filter(|t| t.compute_txid() == txin.previous_output.txid)
            .and_then(|t| t.output.get(txin.previous_output.vout as usize));
        if faraday_psbt::verify::states_false(file, i)
            || matches!(
                (own_output, &file.inputs[i].witness_utxo),
                (Some(o), Some(w)) if o != w
            )
        {
            return Some((i, false));
        }
        (!proven(own)
            && !proven(file)
            && matches!(
                (&own.inputs[i].witness_utxo, &file.inputs[i].witness_utxo),
                (Some(o), Some(w)) if o != w
            ))
        .then_some((i, true))
    })
}

/// Why a file is refused for an input it states different outputs for: a
/// bare `witness_utxo` against the spend's, neither proven (`bare`), or a
/// statement its previous transaction does not bear out.
fn lie_refusal(source: &str, input: usize, bare: bool) -> String {
    let why = if bare {
        EN.wallet_outputs_differ_unproven
    } else {
        EN.wallet_file_states_false
    };
    fill(why, &[source, &input.to_string()])
}

/// Where `keep_own_over_file` finds the spend and a file at odds with no
/// way to tell which is right: the input.
enum Conflict {
    /// Two different final witnesses, neither one the device can judge true.
    Finals(usize),
    /// A file's row for a key and leaf that the spend lacks or holds
    /// differently, with no output stated by either.
    Rows(usize),
    /// Two different scripts for one control block, with no output stated.
    Scripts(usize),
    /// The spend's own statement of the output is false (a previous
    /// transaction of another txid, or without the output): that is the
    /// reason, not that no output is stated.
    False(usize),
}

/// The refusal for input `i` where no output is read: the spend's own false
/// statement of one when it makes one, else `otherwise`.
fn no_output(merged: &faraday_bip::bitcoin::Psbt, i: usize, otherwise: Conflict) -> Conflict {
    if faraday_psbt::verify::states_false(merged, i) {
        Conflict::False(i)
    } else {
        otherwise
    }
}

/// Costs, per input, a comparison of the file's rows and entries with the
/// spend's; where the spend's rows differ from the merged file's and an
/// output is read, two `script_row_verdicts` (a copy of the file, a classify, a signature
/// check per row, a control block check per script entry, and per row a
/// read of its leaf's script); a `tap_entry_commits` (a control block
/// check) per differing script entry, each a classify of the whole PSBT;
/// and, for an input with a final, a `settle_finals` and a classify, a
/// finalize of a copy of the whole file. After a merge, what the spend keeps of its own over what a
/// file brought: a `tap_scripts` entry that commits to the output, over a
/// file's entry of the same control block that does not; an own partial
/// Schnorr row over the file's row for the same key and leaf where the
/// file's reads not checked and the own does not read wrong; a final
/// witness the output judges true (a run leaf, the key path), over one for a
/// script not run, whichever came first; and, on an input whose final
/// witness is for a script not run, no final witness at all where the rows
/// the input holds finish it (from the spend or from any file, in any
/// order). A file's row that reads wrong is not saved here: it is refused
/// after. Where the two differ and the device cannot judge between them,
/// the merge is refused in either order: two different rows for one key and
/// leaf, or two different scripts for one control block, with no output
/// stated on either side; two different finals for scripts not run, or with
/// no output to read them against.
fn keep_own_over_file(
    merged: &mut faraday_bip::bitcoin::Psbt,
    own_inputs: &[faraday_bip::bitcoin::psbt::Input],
    file_inputs: &[faraday_bip::bitcoin::psbt::Input],
) -> Result<(), Conflict> {
    use faraday_psbt::verify::{
        Verdict, not_evaluated, script_row_verdicts, spent_output_of, tap_entry_commits,
    };
    for (i, own) in own_inputs.iter().enumerate() {
        let file = file_inputs.get(i);
        let output_read = spent_output_of(merged, i).is_some();
        for (control, entry) in &own.tap_scripts {
            if merged.inputs[i].tap_scripts.get(control) == Some(entry) {
                continue;
            }
            if !output_read {
                return Err(no_output(merged, i, Conflict::Scripts(i)));
            }
            let (script, version) = entry;
            let kept = tap_entry_commits(merged, i, control, script, *version);
            let replaced = merged.inputs[i]
                .tap_scripts
                .get(control)
                .is_some_and(|(s, v)| tap_entry_commits(merged, i, control, s, *v));
            if kept && !replaced {
                merged.inputs[i]
                    .tap_scripts
                    .insert(control.clone(), entry.clone());
            }
        }
        // With no output stated by either side a file's row cannot be
        // checked, and one the spend lacks or holds differently is refused;
        // a row equal to the spend's is no new row.
        if !output_read
            && file.is_some_and(|f| {
                f.tap_script_sigs
                    .iter()
                    .any(|(k, v)| own.tap_script_sigs.get(k) != Some(v))
            })
        {
            return Err(no_output(merged, i, Conflict::Rows(i)));
        }
        // The own rows the file's differ from, read together.
        let differ: Vec<_> = own
            .tap_script_sigs
            .iter()
            .filter(|(k, mine)| merged.inputs[i].tap_script_sigs.get(k) != Some(mine))
            .collect();
        if output_read && !differ.is_empty() {
            let theirs = script_row_verdicts(merged, i);
            let mut swapped = Vec::new();
            for (&(key, leaf), &mine) in differ {
                let unchecked = theirs
                    .iter()
                    .any(|(k, l, v)| *k == key && *l == leaf && matches!(v, Verdict::Unchecked(_)));
                if unchecked {
                    let file_row = merged.inputs[i].tap_script_sigs.insert((key, leaf), mine);
                    swapped.push(((key, leaf), file_row));
                }
            }
            if !swapped.is_empty() {
                let ours = script_row_verdicts(merged, i);
                for (k, file_row) in swapped {
                    let wrong = ours
                        .iter()
                        .any(|(key, leaf, v)| (*key, *leaf) == k && v.is_invalid());
                    if let Some(row) = file_row
                        && wrong
                    {
                        merged.inputs[i].tap_script_sigs.insert(k, row);
                    }
                }
            }
        }
        if let Some(file) = file {
            settle_finals(merged, own, file, i, output_read)?;
        }
        if has_final(&merged.inputs[i]) && not_evaluated(merged, i) {
            let sig = merged.inputs[i].final_script_sig.take();
            let witness = merged.inputs[i].final_script_witness.take();
            if !faraday_psbt::input_finishes(merged, i) {
                merged.inputs[i].final_script_sig = sig;
                merged.inputs[i].final_script_witness = witness;
            }
        }
    }
    Ok(())
}

/// Costs a classify of the input twice, and, where two scripts are not
/// run, a copy of the whole file and the finalizer's work on the input
/// (`input_finishes`). Where the spend
/// and a file hold different finals for input `i`: the one the output
/// judges true over one for a script not run; two scripts not run, or no
/// output to read them against, refused unless the rows finish.
fn settle_finals(
    merged: &mut faraday_bip::bitcoin::Psbt,
    own: &faraday_bip::bitcoin::psbt::Input,
    file: &faraday_bip::bitcoin::psbt::Input,
    i: usize,
    output_read: bool,
) -> Result<(), Conflict> {
    use faraday_psbt::verify::not_evaluated;
    if !(differ_final(&own.final_script_witness, &file.final_script_witness)
        || differ_final(&own.final_script_sig, &file.final_script_sig))
    {
        return Ok(());
    }
    if !output_read {
        return Err(no_output(merged, i, Conflict::Finals(i)));
    }
    // Each final as the input would read it with that one in place: put in
    // the input for the one read and taken out again.
    let mut with = |first: &faraday_bip::bitcoin::psbt::Input,
                    second: &faraday_bip::bitcoin::psbt::Input| {
        let held = (
            merged.inputs[i].final_script_sig.take(),
            merged.inputs[i].final_script_witness.take(),
        );
        let placed = (
            first
                .final_script_sig
                .clone()
                .or_else(|| second.final_script_sig.clone()),
            first
                .final_script_witness
                .clone()
                .or_else(|| second.final_script_witness.clone()),
        );
        merged.inputs[i].final_script_sig = placed.0;
        merged.inputs[i].final_script_witness = placed.1;
        let unread = not_evaluated(merged, i);
        let placed = (
            merged.inputs[i].final_script_sig.take(),
            merged.inputs[i].final_script_witness.take(),
        );
        merged.inputs[i].final_script_sig = held.0;
        merged.inputs[i].final_script_witness = held.1;
        (unread, placed)
    };
    let ((mine, _), (theirs, theirs_fields)) = (with(own, file), with(file, own));
    match (mine, theirs) {
        // The file's final is one the output judges: it stands over a
        // script not run.
        (true, false) => {
            merged.inputs[i].final_script_sig = theirs_fields.0;
            merged.inputs[i].final_script_witness = theirs_fields.1;
        }
        // Two scripts not run: rows that finish beat both, else neither
        // can be checked.
        (true, true) => {
            let held = (
                merged.inputs[i].final_script_sig.take(),
                merged.inputs[i].final_script_witness.take(),
            );
            let finishes = faraday_psbt::input_finishes(merged, i);
            merged.inputs[i].final_script_sig = held.0;
            merged.inputs[i].final_script_witness = held.1;
            if !finishes {
                return Err(Conflict::Finals(i));
            }
        }
        _ => {}
    }
    Ok(())
}

/// Whether the spend and a file each hold a value for one final field and
/// the two are not the same.
fn differ_final<T: PartialEq>(own: &Option<T>, file: &Option<T>) -> bool {
    matches!((own, file), (Some(a), Some(b)) if a != b)
}

impl Conflict {
    /// The refusal of the file `source`: the spend and it differ, and it is
    /// not said to be the one that is wrong.
    fn refusal(self, source: &str) -> String {
        let (why, i) = match self {
            Conflict::Finals(i) => (EN.wallet_finals_differ, i),
            Conflict::Rows(i) => (EN.wallet_signature_unchecked, i),
            Conflict::Scripts(i) => (EN.wallet_scripts_differ, i),
            Conflict::False(i) => return fill(EN.psbt_final_prev_disagree, &[&i.to_string()]),
        };
        fill(why, &[source, &i.to_string()])
    }
}

/// Whether an input holds a final scriptSig or witness.
fn has_final(input: &faraday_bip::bitcoin::psbt::Input) -> bool {
    input.final_script_sig.is_some() || input.final_script_witness.is_some()
}

/// Why a cosigner's file is refused for a final input that fails its
/// output: a signature the output refuses names the file as a bad
/// signature does; any other fault is the input's.
fn file_witness_refusal(file: &faraday_bip::bitcoin::Psbt, source: &str) -> Option<String> {
    use faraday_psbt::verify::{final_signature_wrong, witness_mismatch, witness_shape_mismatch};
    let i = (0..file.inputs.len()).find(|i| witness_mismatch(file, *i))?;
    if witness_shape_mismatch(file, i) {
        return Some(fill(EN.psbt_warn_witness_mismatch, &[&i.to_string()]));
    }
    if final_signature_wrong(file, i) {
        return Some(fill(EN.wallet_a_signature, &[source]));
    }
    Some(fill(EN.psbt_final_signature_refused, &[&i.to_string()]))
}

/// Costs a copy of the whole file for the signature checks of every input
/// (once), a few classifies per input and a tweak per final taproot
/// witness, and for each final input a signature check per row and the
/// interpreter's run. Why
/// nothing finished is made from `psbt`: the first input, in order, with
/// anything to say, and for that input the first of these: its finalized
/// `scriptSig` or witness does not match the output it spends, its `witness_utxo` and
/// previous transaction differ, it is final and its previous output does
/// not read, or its previous transaction is not the one it spends or lacks
/// the output, or one of its final signatures reads Invalid (an ECDSA or a
/// taproot key-path one, or the interpreter refuses a Tap miniscript leaf's)
/// ([`faraday_psbt::verify::final_invalid`]). There is no priority across inputs.
fn refusal_of(psbt: &faraday_bip::bitcoin::Psbt) -> Option<FinalRefusal> {
    // One copy of the file for the signature checks of every input.
    let mut pass: Option<faraday_psbt::verify::Pass> = None;
    (0..psbt.inputs.len()).find_map(|i| {
        let text = |t: &str| Some(FinalRefusal::Text(fill(t, &[&i.to_string()])));
        // The script refuses the final fields: a signature of the file's
        // where one reads wrong, else the fields alone.
        let fields = |i: usize, pass: &mut Option<faraday_psbt::verify::Pass>| {
            let line = fill(EN.psbt_final_signature_refused, &[&i.to_string()]);
            if pass
                .get_or_insert_with(|| faraday_psbt::verify::Pass::new(psbt))
                .final_signature_wrong(psbt, i)
            {
                Some(FinalRefusal::Signature(line))
            } else {
                Some(FinalRefusal::Text(line))
            }
        };
        if faraday_psbt::verify::witness_shape_mismatch(psbt, i) {
            text(EN.psbt_warn_witness_mismatch)
        } else if faraday_psbt::verify::witness_mismatch(psbt, i) {
            fields(i, &mut pass)
        } else if faraday_psbt::verify::utxos_disagree(psbt, i) {
            text(EN.psbt_final_prev_disagree)
        } else if faraday_psbt::verify::final_unread(psbt, i) {
            text(EN.psbt_final_prev_unread)
        } else if faraday_psbt::verify::states_false(psbt, i) {
            text(EN.psbt_prev_false)
        } else if pass
            .get_or_insert_with(|| faraday_psbt::verify::Pass::new(psbt))
            .final_invalid(psbt, i)
        {
            fields(i, &mut pass)
        } else {
            None
        }
    })
}

/// Why a PSBT's final input cannot stand: a signature the output refuses
/// (`collect` names the file that brings one), or another fault.
enum FinalRefusal {
    Signature(String),
    Text(String),
}

impl FinalRefusal {
    fn text(self) -> String {
        match self {
            FinalRefusal::Signature(t) | FinalRefusal::Text(t) => t,
        }
    }
}

/// What checking a signed message found.
pub struct MessageCheck {
    /// The signed text, as read.
    pub signed: faraday_psbt::message::SignedText,
    /// The signature verifies against the address, and in which format;
    /// or why not.
    pub verdict: Result<faraday_psbt::message::Format, faraday_psbt::message::Error>,
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
            .map_err(|e| crate::state::message_error_text(e).to_string())
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
            .map(|c| c.format);
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
pub(crate) fn musig_nonces(
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

/// The names a spend's files went out under (`None` until put there).
#[derive(Default, Clone, Debug)]
pub struct OutNames {
    /// The signed PSBT.
    pub signed: Option<String>,
    /// The finished transaction.
    pub tx: Option<String>,
    /// The part-signed PSBT.
    pub part: Option<String>,
}

/// A transaction being signed, with its screen's state.
pub struct SpendState {
    /// The spend.
    pub spend: Spend,
    /// The open step, if any.
    pub open: Option<u8>,
    /// Steps closed as done.
    pub done: [bool; 10],
    /// The holdable dangers held on Review (F136), passed to signing.
    pub held: Vec<faraday_psbt::WarningKind>,
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
    /// The masters still to sign: named on an input not yet final with no
    /// signature counted there ([`Spend::signing`]).
    pub to_sign: Vec<[u8; 4]>,
    /// The masters whose partial signature is in the file, verifies, and
    /// that this device cannot confirm ([`Signing::in_file`]).
    pub in_file: Vec<[u8; 4]>,
    /// The masters whose partial signature in the file does not verify
    /// ([`Signing::failed`]).
    pub failed: Vec<[u8; 4]>,
    /// Which results went to the Outbox.
    pub out_signed: bool,
    /// The finished transaction went to the Outbox.
    pub out_tx: bool,
    /// The names this spend's files went out under, beside any other
    /// transaction's file of the name: the signed PSBT, the finished
    /// transaction and the part-signed PSBT.
    pub out_names: OutNames,
    /// The finished transaction's hex is shown.
    pub show_hex: bool,
    /// The last nonce check, with the PSBT it was made over: it signs
    /// again to compare, so it runs once per set of signatures, not once
    /// per frame. [`Faraday::refresh_spend`] makes it where the file
    /// changes; a frame that finds the PSBT other than this one makes it.
    /// The keys found used before are carried into every check of the spend
    /// until it closes (F227): a new check never starts without them.
    pub(crate) nonce_check: std::cell::RefCell<Option<(Psbt, std::rc::Rc<NonceCheck>)>>,
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
    /// The register sheet, while it is open over this spend.
    pub register: Option<crate::register::RegisterAsk>,
    /// [`Spend::refused`] of the PSBT as it is, worked out with the
    /// inspection (`refresh_spend`), read when drawing.
    pub refused: Option<String>,
}

impl SpendState {
    /// The keys this device has signed with that are not still to sign: the
    /// ones the Sign card names as signed.
    pub(crate) fn signed_keys(&self) -> Vec<Fingerprint> {
        self.spend
            .signed_here
            .iter()
            .filter(|f| !self.to_sign.contains(&f.0))
            .copied()
            .collect()
    }

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
        let refused = spend.refused();
        SpendState {
            refused,
            spend,
            open: None,
            done: [false; 10],
            held: Vec::new(),
            table: false,
            error: None,
            inspection,
            inspected,
            signers: Vec::new(),
            to_sign: Vec::new(),
            in_file: Vec::new(),
            failed: Vec::new(),
            out_signed: false,
            out_tx: false,
            out_names: OutNames::default(),
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
            register: None,
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
    /// After a press: the press that asked for a waiting wallet is noted
    /// for the hold to run again, where it [`replays`] (`asked` is
    /// whether one waited before it), and the key path sheet opens.
    pub(crate) fn key_path_after(&mut self, asked: bool, action: Action) {
        if !asked && replays(action) {
            let inbox = self.inbox_print();
            self.session.key_path_replay(action, inbox);
        }
        self.key_path_sheet_open();
    }

    /// The Inbox's file names and sizes, as one number: a press that
    /// names a file by its place reads the same file only while this is
    /// unchanged. A lookup over the names; no file is read.
    pub(crate) fn inbox_print(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        for item in self.inbox.iter() {
            item.name.hash(&mut h);
            item.bytes.len().hash(&mut h);
        }
        h.finish()
    }

    /// Opens the key path sheet over what is on screen while a wallet
    /// waits for its key path to be confirmed (F132). Whether it opened.
    pub(crate) fn key_path_sheet_open(&mut self) -> bool {
        if self.session.key_path_asked().is_some() && self.sheet != Some(Sheet::KeyPath) {
            self.sheet = Some(Sheet::KeyPath);
            return true;
        }
        false
    }

    /// Costs what `add_policy` costs: handles the key path sheet's
    /// actions (`KeyPathHold`, `KeyPathCancel`).
    pub(crate) fn key_path_act(&mut self, action: Action) {
        match action {
            Action::KeyPathHold => {
                self.sheet = None;
                let Some(ask) = self.session.key_path_confirm() else {
                    return;
                };
                // The press that asked runs again and goes on as it would
                // have, while Files holds what it held; a wallet that came
                // in any other way is added here, and the toast says so.
                let descriptor = same_wallet(&ask.policy);
                if let Some((press, inbox)) = ask.replay
                    && inbox == self.inbox_print()
                {
                    self.act(press);
                    if self
                        .session
                        .wallets
                        .iter()
                        .any(|w| same_wallet(&w.policy) == descriptor)
                    {
                        return;
                    }
                }
                let c = self.compute();
                match self
                    .session
                    .add_policy(&c, &ask.name, ask.policy, &ask.source, ask.origin)
                {
                    Ok(i) => {
                        let name = self.session.wallets[i].name.clone();
                        self.wallets.set_wallet(i);
                        self.refresh_spend();
                        self.toast(&fill(EN.wallet_key_path_added, &[&name]));
                    }
                    Err(e) => self.toast(&e.text()),
                }
            }
            Action::KeyPathCancel => {
                self.session.key_path_drop();
                self.sheet = None;
            }
            _ => {}
        }
    }

    /// Costs what `faraday_psbt::register` and `add_policy` cost: the
    /// register sheet's actions (`RegisterOpen`, `RegisterField`,
    /// `RegisterReading`, `RegisterHold`, `RegisterCancel`). The hold
    /// registers the wallet the spend's PSBT defines, as the reading
    /// chosen, once its descriptor gives the spend's scripts and one of
    /// its keys is a loaded key's own at its path, adds it as a wallet
    /// file is added, and inspects the spend again; a refusal stays on
    /// the sheet.
    pub(crate) fn register_act(&mut self, action: Action) {
        let Some(s) = self.wallets.spend_mut() else {
            return;
        };
        match action {
            Action::RegisterOpen if s.inspection.register.is_some() => {
                s.register = Some(crate::register::RegisterAsk::new(&s.spend.source));
                self.sheet = Some(Sheet::Register);
            }
            Action::RegisterField => {
                if let Some(ask) = s.register.as_mut() {
                    ask.typing = true;
                }
            }
            Action::RegisterReading(k) => {
                if let Some(ask) = s.register.as_mut() {
                    ask.reading = usize::from(k);
                    ask.refused = None;
                }
            }
            Action::RegisterCancel => {
                s.register = None;
                self.sheet = None;
            }
            Action::RegisterHold => {
                let (Some(ask), Some(offer)) = (&s.register, &s.inspection.register) else {
                    return;
                };
                let name = ask.name_or(&crate::register::name_of_file(&s.spend.source));
                let reading = ask.reading;
                let (source, psbt, offer) =
                    (s.spend.source.clone(), s.spend.psbt.clone(), offer.clone());
                let c = self.compute();
                let keys = &self.session.keys;
                let yours =
                    |fp: Fingerprint, path: &faraday_bip::bitcoin::bip32::DerivationPath| {
                        keys.xpub_at(&c, keys.position(fp)?, path)
                    };
                let added = faraday_psbt::register(&c, &psbt, &offer, reading, &yours)
                    .map_err(|r| crate::register::refusal_text(r, &offer))
                    .and_then(|policy| {
                        self.session
                            .add_policy(&c, &name, policy, &source, Origin::Named)
                            .map_err(|e| e.text())
                    });
                match added {
                    Ok(i) => {
                        if let Some(s) = self.wallets.spend_mut() {
                            s.register = None;
                        }
                        self.sheet = None;
                        self.wallets.set_wallet(i);
                        self.refresh_spend();
                        self.toast(&fill(EN.register_added, &[&name]));
                    }
                    Err(why) => {
                        if let Some(ask) =
                            self.wallets.spend_mut().and_then(|s| s.register.as_mut())
                        {
                            ask.refused = Some(why);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    /// Costs an inspection and the nonce check when the PSBT or the session changed, and
    /// what `aggregate_musig`, `signers_by` and `wallet_for` cost. Keeps the open spend in
    /// step with the session: its inspection, signers, wallet and steps.
    pub(crate) fn refresh_spend(&mut self) {
        let c = self.compute();
        if let Some(s) = self.wallets.spend_mut() {
            // Inspected once each time the PSBT or the session changes,
            // not on every press that refreshes the spend. The signers, the
            // wallet and whether it is complete are read from that
            // inspection, so they are worked out when it is made (a spend
            // opens with them worked out, `SpendState::opened_with`).
            let inputs = self.session.inspect_inputs();
            let view = s.musig.as_ref().map(|m| m.view());
            if !s.inspected_for(&inputs) {
                let fresh = crate::signing::fresh::worked_out(
                    &c,
                    &self.session,
                    &mut s.spend,
                    view.as_ref(),
                    true,
                );
                s.take(fresh);
            }
            let wallet = s.wallet.and_then(|w| self.session.wallets.get(w));
            let kind = wallet.map(|w| wallet::Kind::of(w.policy()));
            s.needed = wallet.map(wallet::needed).unwrap_or(1);
            // With no wallet loaded the signers come from the PSBT alone,
            // and a second device may still be needed.
            let kind = kind.unwrap_or(if s.inspection.participating_keys.len() > 1 {
                wallet::Kind::Multi(faraday_bip::policy::Wrapper::Wsh)
            } else {
                wallet::Kind::Single(faraday_bip::keys::ScriptType::NativeSegwit)
            });
            s.steps = kind.steps();
            // Check is done already, this power-on, when this wallet was
            // checked before (`docs/WALLETS.md` §4): the flow opens on
            // the next card without a default instead.
            if let Some(wallet) = wallet {
                let checksum = wallet.checksum().to_string();
                if self.memory.is_checked(&checksum) {
                    s.done[wallet::step::CHECK as usize] = true;
                }
            }
            let stale = s.open.is_some_and(|open| !s.steps.contains(&open));
            if stale || !s.opened {
                s.open = s.steps.iter().copied().find(|&n| !s.done[n as usize]);
                s.opened = true;
            }
        }
        self.nonce_spend_sync();
        self.spend_files_out();
    }

    /// Costs a map lookup per signature when only the nonce history
    /// changed (noted again only then), and the nonce check (a derivation per signature, a
    /// verification per partial signature) when a transaction that can be
    /// finished, or is held back, has none kept yet. The open spend's nonce check compared
    /// with the history, and what it found noted for a vault. While a nonce
    /// is used twice in the transaction or was used before in an open vault
    /// nothing is finished (`Spend::nonce_hold`), so nothing is saved or
    /// shown as a code; a key found to have used a nonce before stays in
    /// the check for as long as the spend is open, so a closed vault or the
    /// history turned off does not lift it (F227).
    pub(crate) fn nonce_spend_sync(&mut self) {
        let history = self.nonce_snapshot();
        let c = self.compute();
        let session = &self.session;
        let Some(s) = self.wallets.spend_mut() else {
            return;
        };
        let kept = s
            .nonce_check
            .borrow()
            .as_ref()
            .filter(|(p, _)| *p == s.spend.psbt)
            .map(|(_, check)| check.clone());
        // A key found stays for as long as the spend is open, also in the
        // check of another PSBT of it (F227).
        let found_before = s
            .nonce_check
            .borrow()
            .as_ref()
            .map(|(_, check)| check.used_before.clone())
            .unwrap_or_default();
        let (mut check, mut stored) = match kept {
            Some(check) => (check, false),
            // A spend that cannot be finished yet, was not held and carries
            // no partial signature leaves the check to the card that shows
            // it.
            None if !(s.complete
                || s.spend.nonce_hold
                || s.spend.psbt.inner().inputs.iter().any(|i| {
                    !i.partial_sigs.is_empty()
                        || i.tap_key_sig.is_some()
                        || !i.tap_script_sigs.is_empty()
                        || i.final_script_sig.is_some()
                        || i.final_script_witness.is_some()
                        || faraday_psbt::musig::partial_sigs(i).is_ok_and(|p| !p.is_empty())
                })) =>
            {
                return;
            }
            None => {
                let mut fresh = session.nonce_check(&c, &s.spend.psbt);
                fresh.partials = session.nonce_partials(&c, &s.spend.psbt, &s.inspection);
                fresh.used_before = found_before;
                (std::rc::Rc::new(fresh), true)
            }
        };
        if stored || check.history_gen != history.generation() {
            std::rc::Rc::make_mut(&mut check).against(&history);
            stored = true;
        }
        if stored {
            *s.nonce_check.borrow_mut() = Some((s.spend.psbt.clone(), check.clone()));
        }
        let held = check.held_back();
        let mut taken_back = Vec::new();
        let mut lifted = false;
        if held != s.spend.nonce_hold {
            s.spend.nonce_hold = held;
            if held {
                s.spend.finished = None;
                s.complete = false;
                // What this spend already put in the Outbox goes back.
                s.out_signed = false;
                s.out_tx = false;
                taken_back.extend(s.out_names.signed.take());
                taken_back.extend(s.out_names.tx.take());
                taken_back.extend(s.out_names.part.take());
            } else {
                s.complete = s.spend.finish().is_ok();
                lifted = s.complete;
            }
        }
        if !taken_back.is_empty() && self.outbox.iter().any(|i| taken_back.contains(&i.name)) {
            self.outbox.retain(|i| !taken_back.contains(&i.name));
            self.save_boxes();
        }
        // Noted only when the check was stored or compared again: a note
        // checks each entry against the transaction's.
        if stored {
            self.note_nonces(&check);
        }
        // The transaction finished again: its files are made at once.
        if lifted {
            self.spend_files_out();
        }
    }

    /// Whether the register sheet's name field has focus. A lookup.
    pub(crate) fn register_typing(&self) -> bool {
        self.wallets
            .spend()
            .and_then(|s| s.register.as_ref())
            .is_some_and(|ask| ask.typing)
    }

    /// A key on the register sheet: typed into the name while it has
    /// focus, Escape closing the sheet. Whether it took the key.
    pub(crate) fn register_key(&mut self, key: KeyIn) -> bool {
        if self.sheet != Some(Sheet::Register) {
            return false;
        }
        if key == KeyIn::Escape {
            self.register_act(Action::RegisterCancel);
            return true;
        }
        self.wallets
            .spend_mut()
            .and_then(|s| s.register.as_mut())
            .is_some_and(|ask| ask.key(key))
    }

    /// Costs what `add_wallet` costs. Loads the wallet an Inbox file
    /// describes, named after the file.
    pub(crate) fn load_wallet(&mut self, index: usize) {
        let c = self.compute();
        let Some(item) = self.inbox.get(index) else {
            return;
        };
        let text = String::from_utf8_lossy(&item.bytes).into_owned();
        let source = item.name.clone();
        let name = crate::register::name_of_file(&item.name);
        // A wallet .json or an addwallet command carries its own name.
        let name = wallet::carried_name(&text).unwrap_or(name);
        match self
            .session
            .add_wallet(&c, &name, &text, &source, wallet::Origin::Named)
        {
            Ok(i) => {
                self.wallets.set_wallet(i);
                self.screen = Screen::Wallets;
                self.refresh_spend();
            }
            Err(e) => self.toast(&e.text()),
        }
    }

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
            Action::StartSpend(i) => {
                crate::signing::read::start_spend(self, i, crate::signing::read::Then::Spend);
            }
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
            Action::ReviewHold => {
                let inputs = self.session.inspect_inputs();
                if crate::review::hold(self.wallets.spend_mut(), &inputs) {
                    self.wallet_act(Action::StepNext(wallet::step::TRANSACTION));
                }
            }
            Action::StepNext(n) => {
                if let Some(s) = self.wallets.spend_mut() {
                    if n == wallet::step::TRANSACTION && crate::review::stopped(s) {
                        return;
                    }
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
            Action::SignHere => crate::signing::sign_here(self),
            Action::Collect(i) => crate::signing::collect::start_collect(self, i),
            Action::SignedToOutbox => {
                if let Some(s) = self.wallets.spend_mut().filter(|s| !s.spend.nonce_hold) {
                    let name = format!("{}-signed.psbt", result_stem(&s.spend.source));
                    let Ok(bytes) = s.spend.out_bytes() else {
                        return self.toast(EN.spend_v2_not_written);
                    };
                    s.out_signed = true;
                    let name = self.put_outbox_beside(&name, bytes);
                    if let Some(s) = self.wallets.spend_mut() {
                        s.out_names.signed = Some(name);
                    }
                }
            }
            Action::TxToOutbox => {
                if let Some(s) = self.wallets.spend_mut()
                    && let Some(hex) = s.spend.finished_hex()
                {
                    let name = format!("{}-final.txn", result_stem(&s.spend.source));
                    s.out_tx = true;
                    let name = self.put_outbox_beside(&name, hex.into_bytes());
                    if let Some(s) = self.wallets.spend_mut() {
                        s.out_names.tx = Some(name);
                    }
                }
            }
            Action::Primary => self.primary(),
            Action::LoadWallet(i) => self.load_wallet(i),
            Action::LoadKey(i) => {
                if self.load_key(i) {
                    self.inbox_load_named();
                }
            }
            Action::PublicOut(i, what) => self.public_out_job(i, what),
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

#[cfg(test)]
mod partial_tests {
    use super::*;

    fn fp(n: u8) -> Fingerprint {
        Fingerprint([n, 0, 0, 0])
    }

    #[test]
    fn a_master_with_two_accounts_counts_only_when_both_do() {
        // The same master twice, as two participants of one wallet; the
        // first signed here, the second not yet.
        let first_only = Tally::of(&[
            (fp(1), Partial::Here),
            (fp(1), Partial::Open),
            (fp(2), Partial::Open),
        ]);
        assert!(first_only.counted.is_empty());
        assert_eq!(first_only.to_sign, vec![fp(1), fp(2)]);
        let both = Tally::of(&[
            (fp(1), Partial::Here),
            (fp(1), Partial::Here),
            (fp(2), Partial::Word),
        ]);
        assert_eq!(both.counted, vec![fp(1)]);
        assert_eq!(both.in_file, vec![fp(2)]);
        assert!(both.to_sign.is_empty());
    }
}
