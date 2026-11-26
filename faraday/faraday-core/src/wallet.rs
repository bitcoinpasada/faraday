//! The Wallets tab's session: keys and wallets held in memory, and the spend of
//! one transaction from loading it to the two results (`docs/WALLETS.md`
//! §2 and §4). Every piece of cryptography is `faraday-bip`'s or
//! `faraday-psbt`'s; this module only decides which call comes next.

use crate::modes::Mode;
use crate::strings::EN;
use crate::{Action, Faraday, Screen, Sheet, wallet};
use crate::{WalletKey, create};
use faraday_bip::bip39::{Language, Mnemonic};
use faraday_bip::bitcoin::consensus::encode::serialize_hex;
use faraday_bip::bitcoin::{OutPoint, Transaction, Txid};
use faraday_bip::keys::{Fingerprint, Network, ScriptType};
use faraday_bip::policy::{KeyPath, Template, WalletId, WalletPolicy, Wrapper};
use faraday_crypto::work::Compute;
use faraday_keys::vault::VaultId;
use faraday_psbt::verify::Verdict;
use faraday_psbt::{Context, Inspection, KeyRef, Psbt};
use faraday_shell_api::Key as KeyIn;
use faraday_strings::bip::policy_error;
use faraday_text::{Text, text};

/// The network a session starts on, before a wallet or a PSBT says
/// otherwise.
pub(crate) const START_NETWORK: Network = Network::Mainnet;

// The description reader (`faraday-intake`),
// at the paths this crate has always had them.
pub use faraday_intake::wallet::{
    Refusal, WALLET_NAME_MAX, network_name, network_of_key, policy_network, wallet_name,
};

pub use faraday_keys::identity::master_id;
pub use faraday_keys::{WordsId, words_id};

mod nonce;
pub use nonce::{NonceCheck, NonceRow, Rest, Unmatched};
mod v2;
pub use v2::{is_silent_payment_psbt, read_psbt, read_psbt_kept};
mod account;
mod own_name;
pub use account::{Account, AccountAsk};
mod confirm;
pub use confirm::Rows;
pub(crate) use confirm::{draw_rows, draw_rows_over, draw_rows_without_shape};
mod keycheck;
pub use keycheck::{KeyBlockOf, KeyCheck};
pub(crate) use keycheck::{draw_key_block, draw_review_keys, keys_caution_text};
mod state;
pub use state::SpendState;
pub(crate) use state::{SPEND_STEPS, SpendSteps, WalletsArea};

/// Where a wallet kept at `place` is after the wallet at `gone` left the
/// list: the same place when it was before it, one less when after it,
/// none when it is the one gone.
pub(crate) fn place_after_removal(place: usize, gone: usize) -> Option<usize> {
    match place.cmp(&gone) {
        std::cmp::Ordering::Less => Some(place),
        std::cmp::Ordering::Equal => None,
        std::cmp::Ordering::Greater => Some(place - 1),
    }
}

/// [`place_after_removal`] of a place that may be unset.
pub(crate) fn kept_place_after_removal(place: &mut Option<usize>, gone: usize) {
    *place = place.and_then(|p| place_after_removal(p, gone));
}

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

use crate::public_files;
pub(crate) use faraday_intake::wallet::{carried_name, network_of_kind};

/// Costs a text check of a name long enough to be secret text. The name a
/// wallet is added under: `name`, or the default name when `name` is itself
/// secret text (seed words, an extended private key), as a file's `Name:`
/// line or its stem can be. The file stays a secret file as the intake
/// classifies it. A name under 26 bytes is no key or seed and is not
/// checked (the shortest is 12 words); 16 or 32 bytes of plain text are
/// not read as a compact seed QR, as the name of a wallet is text.
pub(crate) fn safe_name(c: &Compute, name: &str) -> Text {
    use crate::scan_read::Kind;
    use crate::text_check::{TextCheck, check};
    let default = || Text::of(EN.wallet_default_name);
    // The secret check reads the name as given and the whole line it
    // makes (a dropped character inside a word does not hide it), before
    // it is cut short.
    let line = faraday_intake::wallet::one_line(name);
    for (i, text) in [name, &*line].into_iter().enumerate() {
        if text.trim().len() >= 26 && (i == 0 || text != name) {
            match check(c, text) {
                TextCheck::Secret(Kind::CompactSeedQr) => {}
                TextCheck::Secret(_) | TextCheck::Inside(_) | TextCheck::TooLong => {
                    return default();
                }
                _ => {}
            }
        }
    }
    let name = wallet_name(&line);
    if name.is_empty() {
        return default();
    }
    Text::of(&name)
}

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
    ) -> Result<Text, Text>;

    /// Costs a key derivation the first time. `[fingerprint/path]xpub` of
    /// key `i` at `path` ([`crate::create::path_spec`]), kept as
    /// [`KeyTexts::account_text`] is.
    fn text_at(&self, c: &Compute, i: usize, path: &str) -> Result<Text, Text>;
}

impl KeyTexts for faraday_keys::Keys {
    fn account_text(
        &self,
        c: &Compute,
        i: usize,
        kind: crate::create::NewKind,
        account: u32,
    ) -> Result<Text, Text> {
        let spec = kind
            .spec(self.network(), account)
            .map_err(|e| Text::of(&e))?;
        self.key_text(c, i, &spec)
            .unwrap_or_else(|| Err(Text::default()))
    }

    fn text_at(&self, c: &Compute, i: usize, path: &str) -> Result<Text, Text> {
        let spec = crate::create::path_spec(path).map_err(|e| Text::of(&e))?;
        self.key_text(c, i, &spec)
            .unwrap_or_else(|| Err(Text::default()))
    }
}

/// What one signing pass did.
pub struct Signed {
    /// The fingerprints that signed.
    pub by: Vec<Fingerprint>,
}

pub(crate) mod paths;

/// A wallet described by its public keys.
pub struct Wallet {
    /// What the person called it: one line of text ([`wallet_name`]),
    /// as [`Wallet::new`] and a rename make it; a sheet written for the
    /// wallet makes it so again.
    pub name: Text,
    /// The policy, as it was read.
    policy: WalletPolicy,
    /// Where it came from: "Test wallet" or the file's name.
    pub source: Text,
    /// What [`Wallet::source`] is: every caller that adds a wallet
    /// ([`Session::add_wallet`]) already knows this, so it is given
    /// explicitly there rather than read back out of the text, which a
    /// second language would not carry. [`source_tag`] and Spend from a backup's
    /// routing (`spend_backup/mod.rs`) read this, not the words.
    pub origin: Origin,
    /// The addresses asked of it, each derived once, by network, chain
    /// and index ([`Wallet::address_on`]): public, and dropped with the
    /// policy they come from ([`Wallet::set_policy`]).
    addresses: std::cell::RefCell<Vec<(Network, bool, u32, Text)>>,
    /// The scripts asked of it to find which wallet a PSBT spends from,
    /// each derived once, by chain and index ([`Wallet::script_at`]):
    /// public, and dropped with the policy they come from.
    scripts: std::cell::RefCell<Vec<ScriptKept>>,
    /// [`Wallet::same`], kept; dropped with the policy.
    same: std::cell::OnceCell<Text>,
    /// The fingerprints of the keys loaded when its keys were last
    /// checked against them ([`Session::drop_forged`]).
    checked: Vec<Fingerprint>,
    /// The fingerprints of the keys waiting for their backup when its keys
    /// were last checked against them ([`Session::drop_forged`]).
    waited: Vec<Fingerprint>,
    /// What vouches for each key of a wallet of several keys
    /// ([`Wallet::key_checks`]).
    key_checks: keycheck::KeyChecks,
    /// The spend paths, read once from the policy when it is set
    /// ([`Wallet::waits`]).
    waits: Waits,
}

/// A wallet's spend paths with their waits, read once from its policy
/// (`docs/BACKUP-FLOW.md` §18.4), and the shortest relative wait.
#[derive(Debug, Clone, Default)]
pub struct Waits {
    /// The ways to spend, as [`WalletPolicy::spend_paths`] lists them.
    pub paths: Vec<faraday_bip::spend::SpendPath>,
    /// The shortest relative wait among them, `None` where no path has
    /// one.
    pub shortest: Option<Shortest>,
}

/// The shortest relative wait of a wallet's paths (`docs/BACKUP-FLOW.md`
/// §18.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Shortest {
    /// The path it belongs to, by place in [`Waits::paths`].
    pub path: usize,
    /// The wait: [`Lock::Blocks`] or [`Lock::Intervals`].
    pub lock: faraday_bip::spend::Lock,
    /// It in whole days, rounded down.
    pub days: u32,
    /// The margin: 90 % of the wait, in whole days rounded down.
    pub margin: u32,
}

impl Waits {
    /// Reads the paths of `policy`: its spend paths and the shortest
    /// relative wait.
    fn of(policy: &WalletPolicy) -> Waits {
        use faraday_bip::spend::Lock;
        let paths = policy.spend_paths();
        let mut shortest: Option<(u64, Shortest)> = None;
        for (i, p) in paths.iter().enumerate() {
            for &lock in &p.locks {
                // Seconds, so that blocks and 512-second units compare.
                let (seconds, days, margin) = match lock {
                    Lock::Blocks(n) => {
                        let n = u64::from(n);
                        (n * 600, n / 144, n * 9 / 10 / 144)
                    }
                    Lock::Intervals(n) => {
                        let s = u64::from(n) * 512;
                        (s, s / 86_400, s * 9 / 10 / 86_400)
                    }
                    _ => continue,
                };
                if shortest.is_none_or(|(best, _)| seconds < best) {
                    shortest = Some((
                        seconds,
                        Shortest {
                            path: i,
                            lock,
                            days: days as u32,
                            margin: margin as u32,
                        },
                    ));
                }
            }
        }
        Waits {
            paths,
            shortest: shortest.map(|(_, s)| s),
        }
    }
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
    pub fn new(name: &str, policy: WalletPolicy, source: &str, origin: Origin) -> Wallet {
        Wallet {
            name: Text::of(&wallet_name(name)),
            waits: Waits::of(&policy),
            policy,
            source: Text::of(source),
            origin,
            addresses: std::cell::RefCell::new(Vec::new()),
            scripts: std::cell::RefCell::new(Vec::new()),
            same: std::cell::OnceCell::new(),
            checked: Vec::new(),
            waited: Vec::new(),
            key_checks: keycheck::KeyChecks::default(),
        }
    }

    /// What vouches for each key of the policy, in its order, worked out
    /// when the wallet was added and when a key was loaded or removed:
    /// none for a wallet of one key.
    pub fn key_checks(&self) -> &[KeyCheck] {
        self.key_checks.states()
    }

    /// How many keys nothing vouches for.
    pub fn keys_unchecked(&self) -> usize {
        self.key_checks.unchecked()
    }

    /// The person's ticks only, one per key (none for a wallet of one
    /// key), for a vault to keep.
    pub fn key_ticks(&self) -> Vec<bool> {
        self.key_checks.ticks()
    }

    /// Merges the ticks of a sheet that asked about `from` into this
    /// wallet's, key by key ([`KeyChecks::merge_ticks`]).
    pub(crate) fn merge_key_ticks(&mut self, from: &WalletPolicy, ticks: &[bool]) {
        self.key_checks.merge_ticks(&self.policy, from, ticks);
    }

    /// The rows of the keys as drawn.
    pub(crate) fn key_rows(&self) -> &[confirm::KeyRow] {
        self.key_checks.rows()
    }

    /// The wallet's spend paths and the shortest relative wait, kept
    /// when the policy was read.
    pub fn waits(&self) -> &Waits {
        &self.waits
    }

    /// A copy for work off the main thread: the policy, and the scripts, the
    /// addresses and [`Wallet::same`] already kept, all public.
    pub fn job_copy(&self) -> Wallet {
        Wallet {
            name: self.name.duplicate(),
            policy: self.policy.clone(),
            source: self.source.duplicate(),
            origin: self.origin,
            addresses: std::cell::RefCell::new(self.addresses.borrow().clone()),
            scripts: std::cell::RefCell::new(self.scripts.borrow().clone()),
            same: self.same.clone(),
            checked: Vec::new(),
            waited: Vec::new(),
            key_checks: self.key_checks.clone(),
            waits: self.waits.clone(),
        }
    }

    /// Takes home what a [`Wallet::job_copy`] kept while it worked: the
    /// scripts (and the keys derived beside them) and the addresses this
    /// wallet does not hold yet, and its [`Wallet::same`]. Nothing from a
    /// wallet of another id, and nothing already kept is replaced.
    pub fn take_kept(&self, from: Wallet) {
        if from.id() != self.id() {
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
    pub fn address_on(&self, c: &Compute, network: Network, change: bool, index: u32) -> Text {
        if let Some((.., a)) = self
            .addresses
            .borrow()
            .iter()
            .find(|(n, c, i, _)| *n == network && *c == change && *i == index)
        {
            return a.duplicate();
        }
        let a = {
            self.policy
                .address_at(c, network, change, index)
                .map(|a| text!("{a}"))
                .unwrap_or_else(|e| Text::fill(EN.wallet_no_address, &[&policy_error(&e)]))
        };
        let mut kept = self.addresses.borrow_mut();
        // A long walk through addresses keeps the latest only.
        if kept.len() >= WALLET_ADDRESSES_KEPT {
            kept.clear();
        }
        kept.push((network, change, index, a.duplicate()));
        a
    }

    /// Empties the kept addresses, so that a test counts the next one
    /// asked as derived.
    #[cfg(test)]
    pub(crate) fn forget_addresses(&self) {
        self.addresses.borrow_mut().clear();
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
    /// For what a person reads; which wallet this is, is [`Wallet::id`].
    pub fn checksum(&self) -> &str {
        self.policy.checksum()
    }

    /// Which wallet this is (a hash of its whole descriptor); a lookup,
    /// kept by the policy.
    pub fn id(&self) -> WalletId {
        self.policy.id()
    }

    /// Replaces the policy, dropping the addresses and scripts kept from
    /// the one before.
    pub(crate) fn set_policy(&mut self, policy: WalletPolicy) {
        self.waits = Waits::of(&policy);
        self.policy = policy;
        self.same = std::cell::OnceCell::new();
        self.addresses.borrow_mut().clear();
        self.scripts.borrow_mut().clear();
    }
}

/// What a wallet's [`Wallet::source`] is: Spend from a backup's own "Typed words"
/// step, Seeds' own "Typed seeds" step, a vault's own wallet, text that
/// reads as a statement on its own ("Made here", "Dealt here"), or
/// anything else (a file's name, a search, "Test wallet"). Given by the
/// caller that adds the wallet, where it already knows which this is;
/// every other reader (`source_tag`, Spend from a backup's routing) reads this
/// instead of the text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Origin {
    /// Spend from a backup's own "Typed words" step ([`crate::spend_backup::WORDS_SOURCE`]).
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
    pub held_by: Option<Text>,
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

/// A fingerprint as eight hex digits (public).
pub fn fp_text(fp: Fingerprint) -> Text {
    Text::from_utf8_lossy(&fp.to_hex())
}

/// A fingerprint as eight hex digits, as one exactly sized [`Text`]
/// (public).
pub fn fp_label(fp: Fingerprint) -> faraday_text::Text {
    faraday_text::Text::from_utf8_lossy(&fp.to_hex())
}

/// A wallet's source as a tag: "Vault" reads "From Vault" (`cap`) or
/// "from Vault" (inline, after another phrase), but a source whose
/// [`Origin`] already reads as a complete statement ("Made here",
/// "Dealt here") reads on its own, with no "From"/"from" before it
/// either way.
pub fn source_tag(source: &str, origin: Origin, cap: bool) -> Text {
    if origin == Origin::Statement {
        Text::of(source)
    } else if cap {
        Text::fill(EN.wallet_source_tag, &[source])
    } else {
        Text::fill(EN.wallet_from_inline, &[source])
    }
}

/// A key as a line: its fingerprint, then its label when the label is
/// not the fingerprint itself.
pub fn key_line(fp: Fingerprint, label: &str) -> Text {
    let f = fp_text(fp);
    let label = label.trim();
    if label.is_empty() || label.eq_ignore_ascii_case(&f) {
        f
    } else {
        text!("{} · {label}", &*f)
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
    use crate::spend_backup::SpendBackupAction as F;
    matches!(
        action,
        Action::LoadWallet(_)
            | Action::TTake
            | Action::RUse(_)
            | Action::RRebuild
            | Action::RestoreShares
            | Action::RestoreShareAt(_)
            | Action::Seeds(crate::seeds::SeedsAction::Make)
            | Action::CNext(crate::cstep::KEYS)
            | Action::SpendBackup(F::UseWallet(_) | F::TakeFile(_) | F::Rebuild | F::GuessWay(_))
            | Action::CoreWallet(crate::core_wallet::tool::CoreWalletAction::Take(_))
            | Action::MultisigCheck(crate::multisig_check::MsAction::Add)
    )
}

/// The danger line of a wallet whose key path a key not of its own
/// spends, naming that key in groups of four (F132).
pub fn key_path_danger(key: &str) -> Text {
    Text::fill(EN.wallet_key_path_other, &[&faraday_ui::grouped(key)])
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
    /// The wallets waiting for the person to confirm them before they
    /// are added (F254, F132): what [`Session::add_policy`] was asked to
    /// add and held back, in the order found, one sheet each. Empty while
    /// nothing waits.
    wallet_asked: std::collections::VecDeque<WalletAsk>,
    /// The keys here whose backup is not done: a loaded key of a wallet
    /// reads "signs after its backup" (`Session::set_waiting`).
    waiting: Vec<Fingerprint>,
    /// The sign of what `waiting` was last worked out from
    /// (`Faraday::sync_waiting_if_changed`).
    waiting_inputs: Option<u64>,
    /// The Inbox PSBT each loaded wallet spends from, by wallet place, as
    /// last worked out (`Faraday::sync_psbt_wallets_if_changed`).
    psbt_wallets: Vec<Option<usize>>,
    /// The sign of what `psbt_wallets` was last worked out from.
    psbt_wallets_inputs: Option<u64>,
    /// How many wallets of the run now waiting were confirmed or
    /// cancelled already: the sheet's "{i} of {n}". Zero once none waits.
    wallet_asked_past: usize,
    /// Where in `wallet_asked` the last wallet refused as unconfirmed
    /// waits: the one just pushed, or the one already waiting for the
    /// same descriptor.
    wallet_asked_last: usize,
    /// The sheet the confirmation sheet opened over, put back once no
    /// wallet waits ([`Faraday::confirm_act`]).
    wallet_covered: Option<crate::Sheet>,
    /// The descriptors of the wallets the person confirmed in this
    /// session, which [`Session::add_policy`] adds without asking again.
    wallet_confirmed: Vec<Text>,
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
    /// The silent payments labels handed out, kept until
    /// [`Session::wipe`].
    pub(crate) silent_handed: crate::silent::HandedOut,
}

/// A wallet waiting to be confirmed: what [`Session::add_policy`] was
/// called with, and the rows its sheet shows.
pub struct WalletAsk {
    /// The sheet's rows, worked out when the wallet was held back.
    pub rows: Rows,
    /// The name it is added under.
    pub name: Text,
    /// The wallet, read.
    pub policy: WalletPolicy,
    /// Where it came from, as [`Wallet::source`] has it.
    pub source: Text,
    /// What that source is.
    pub origin: Origin,
    /// The press that asked for it, run again once the hold confirms it,
    /// so the flow that asked goes on as if nothing had been asked, with
    /// the Inbox's names and sizes as they were then
    /// ([`Faraday::inbox_print`]). `None` for a wallet that came in
    /// without a press, or by one that does not run twice the same
    /// ([`replays`]); that wallet is added by the hold itself.
    pub replay: Option<(Action, u64)>,
    /// The wallet was asked for by Restore, which goes on from the wallet
    /// once the hold adds it ([`Session::wallet_for_restore`]).
    pub restore: bool,
    /// Add to vault asked: the hold puts the wallet in this vault and
    /// the session does not load it.
    pub(crate) vault: Option<VaultId>,
}

/// How a `tr()` wallet's key path line reads (F132): its text, and
/// whether it is the danger line, a key that is not one of the wallet's.
pub struct KeyPathLine {
    /// The line.
    pub text: Text,
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
            wallet_asked: std::collections::VecDeque::new(),
            waiting: Vec::new(),
            waiting_inputs: None,
            psbt_wallets: Vec::new(),
            psbt_wallets_inputs: None,
            wallet_asked_past: 0,
            wallet_asked_last: 0,
            wallet_covered: None,
            wallet_confirmed: Vec::new(),
            silent_handed: Default::default(),
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
    /// The place of the loaded wallet whose descriptor is `same`
    /// ([`Wallet::same`]): whether the session holds a wallet.
    pub(crate) fn held_wallet(&self, same: &str) -> Option<usize> {
        self.wallets.iter().position(|w| w.same() == same)
    }

    /// A session on `network`.
    pub fn on(network: Network) -> Session {
        Session {
            keys: faraday_keys::Keys::on(network),
            wallets: Vec::new(),
            modes: crate::modes::STICK_MODES,
            key_marks: crate::codex32_correct::KeyMarks::default(),
            wallet_asked: std::collections::VecDeque::new(),
            waiting: Vec::new(),
            waiting_inputs: None,
            psbt_wallets: Vec::new(),
            psbt_wallets_inputs: None,
            wallet_asked_past: 0,
            wallet_asked_last: 0,
            wallet_covered: None,
            wallet_confirmed: Vec::new(),
            silent_handed: Default::default(),
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
    /// Practice, and the mode that network is belongs to
    /// [`Session::modes`]; that is the same as choosing the mode on an
    /// empty session, so no lock is needed (F81, `docs/MODES.md` §2.8). A
    /// network of the same kind needs no move.
    /// Anything else of another kind is refused, naming both networks.
    pub fn follow(&mut self, network: Option<Network>) -> Result<(), Refusal> {
        let Some(n) = network else { return Ok(()) };
        if n.kind() == self.network().kind() {
            return Ok(());
        }
        let empty = self.keys.is_empty() && self.wallets.is_empty();
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
        self.add_wallet_in(c, name, text, source, origin, true)
    }

    /// Costs what [`Session::add_wallet`] costs, and no confirmation
    /// sheet: [`Session::add_kept`] for a description read here, a vault's
    /// wallet loaded from the open vault or restored from it (F254).
    pub fn add_wallet_kept(
        &mut self,
        c: &Compute,
        name: &str,
        text: &str,
        source: &str,
        origin: Origin,
    ) -> Result<usize, Refusal> {
        self.add_wallet_in(c, name, text, source, origin, false)
    }

    /// Costs what `add_wallet` costs; `confirm` as in `add_checked`.
    fn add_wallet_in(
        &mut self,
        c: &Compute,
        name: &str,
        text: &str,
        source: &str,
        origin: Origin,
        confirm: bool,
    ) -> Result<usize, Refusal> {
        let name = &*safe_name(c, name);
        let read = read_wallet(c, text)?;
        let policy = self.account_of(c, text, read, (name, source, origin), None)?;
        self.add_checked(
            c,
            name,
            policy,
            source,
            origin,
            &Default::default(),
            confirm,
        )
    }

    /// Costs a key derivation (of at most `MAX_PATH_STEPS` steps) per key
    /// of `policy` under a loaded fingerprint (one of `only`, when given),
    /// and a key derivation per key waiting for its backup (one of `only`,
    /// when given) for each key of `policy` under a fingerprint that is
    /// not loaded: the second is none when no key waits or
    /// `see_waiting` is false.
    ///
    /// The refusal for a key of `policy` that is under a loaded
    /// fingerprint and is not what that key gives at the key's origin path
    /// ([`Refusal::NotYourKey`]), or that is under no loaded fingerprint
    /// and is what a key waiting for its backup gives there
    /// ([`Refusal::NotBackedUp`], never when `see_waiting` is false,
    /// so that no waiting key hides a later key of the first kind). Only
    /// the stated origin path is
    /// compared: a file that also states another origin path for the same
    /// key is not caught, and whoever can write that could name a key of
    /// their own.
    fn forged_key(
        &self,
        c: &Compute,
        policy: &WalletPolicy,
        only: Option<&[Fingerprint]>,
        derived: &faraday_keys::job::Derived,
        memo: &mut confirm::Gives,
        see_waiting: bool,
    ) -> Option<Refusal> {
        for key in policy.keys() {
            let (Some(fp), Some(path)) = (key.fingerprint(), key.path()) else {
                continue;
            };
            let Some(i) = self.keys.position(fp) else {
                // Another fingerprint over the extended public key of a
                // key waiting for its backup: the hold finds a key by
                // fingerprint, so the wallet is refused.
                if self.keys.all().enumerate().any(|(j, k)| {
                    see_waiting
                        && self.waiting.contains(&k.fingerprint)
                        && only.is_none_or(|o| o.contains(&k.fingerprint))
                        && self.keys.xpub_at(c, j, path).is_some_and(|x| {
                            x.public_key == key.xpub().public_key
                                && x.chain_code == key.xpub().chain_code
                        })
                }) {
                    return Some(Refusal::NotBackedUp(fp_text(fp)));
                }
                continue;
            };
            if only.is_some_and(|o| !o.contains(&fp)) {
                continue;
            }
            // What a key a load's job added gives there was worked out in
            // the job.
            let gives = match derived.given_at(fp, path) {
                Some(x) => Some(*x),
                None => self.keys.xpub_at(c, i, path),
            };
            memo.push((fp, path.clone(), gives));
            if !gives.is_some_and(|x| {
                x.public_key == key.xpub().public_key && x.chain_code == key.xpub().chain_code
            }) {
                return Some(Refusal::NotYourKey(fp_text(fp)));
            }
        }
        None
    }

    /// Costs two key derivations per loaded key (four at an account other
    /// than 0) for each silent payments wallet while a key was loaded since
    /// it was checked, a key derivation per wallet key under a fingerprint loaded
    /// since the wallet was checked, and one per key waiting for its backup
    /// since the wallet was checked for each wallet key under an unloaded
    /// fingerprint: none when no key was loaded and none began to wait. A
    /// key that begins to wait also costs one derivation for each wallet key
    /// under its own (loaded) fingerprint, since a key that begins to wait
    /// is rechecked against every wallet. Per event (a key loaded, a vault
    /// opened, a backup started or done), never per frame.
    ///
    /// Takes out of the session every wallet with a key under a
    /// fingerprint that was not loaded when the wallet was added (or last
    /// checked) that is not what that key gives at its path (a wallet
    /// added before its key), a silent payments record that names a
    /// fingerprint no loaded key has over the points a loaded key gives,
    /// or that is under no loaded fingerprint and is
    /// a key that began to wait for its backup since, and returns each
    /// one's place (as the list was when it went, the earlier ones already
    /// gone), checksum, name and the refusal's text.
    pub(crate) fn drop_forged(&mut self, c: &Compute) -> Vec<(usize, Text, WalletId, Text, Text)> {
        let now: Vec<Fingerprint> = self.keys.all().map(|k| k.fingerprint).collect();
        let waiting = self.waiting.clone();
        let mut dropped = Vec::new();
        let mut i = 0;
        while i < self.wallets.len() {
            if self.wallets[i].checked == now && self.wallets[i].waited == waiting {
                i += 1;
                continue;
            }
            let new: Vec<Fingerprint> = now
                .iter()
                .copied()
                .filter(|f| !self.wallets[i].checked.contains(f))
                .collect();
            // Keys loaded, and keys that began to wait, since the check.
            let mut only = new.clone();
            only.extend(
                waiting
                    .iter()
                    .copied()
                    .filter(|f| !self.wallets[i].waited.contains(f) && !new.contains(f)),
            );
            let mut memo = confirm::Gives::new();
            // A silent payments record that names another fingerprint over
            // the points of a key loaded since it was checked.
            let silent = self.wallets[i].policy.silent().filter(|_| !new.is_empty());
            let silent = silent
                .and_then(|w| crate::silent::other_key_gives(c, &self.keys, w).map(|k| (k, w)))
                .map(|(k, w)| Refusal::SilentOtherKey {
                    key: Text::of(&fp_text(k)),
                    record: Text::of(&fp_text(w.fingerprint)),
                });
            let forged = if silent.is_some() {
                silent
            } else if only.is_empty() {
                None
            } else {
                self.forged_key(
                    c,
                    &self.wallets[i].policy,
                    Some(&only),
                    &Default::default(),
                    &mut memo,
                    true,
                )
            };
            match forged {
                Some(why) => {
                    let w = self.wallets.remove(i);
                    dropped.push((i, Text::of(w.checksum()), w.id(), w.name, why.text()));
                }
                None => {
                    // A key loaded or removed: what the keys give is
                    // worked out again, the person's ticks kept.
                    self.refresh_key_checks(c, i, (&now, &new), &memo);
                    self.wallets[i].checked.clone_from(&now);
                    self.wallets[i].waited.clone_from(&waiting);
                    i += 1;
                }
            }
        }
        dropped
    }

    /// Costs two key derivations per loaded key with a silent payments
    /// wallet's fingerprint, and a key derivation (of at most
    /// `MAX_PATH_STEPS` steps) per key of any other wallet that is under a
    /// loaded fingerprint, and, for a wallet held back for its sheet, a
    /// key derivation per key under a loaded fingerprint and the
    /// descriptor's checksum, and a key derivation per key waiting for its
    /// backup for each key under an unloaded fingerprint (none when no
    /// key waits), worked out once here, never while drawing.
    /// [`Session::add_wallet`] of a description read before (a vault's
    /// kept records): no parse. A silent payments wallet whose scan key or
    /// spend key is not what the loaded key of its fingerprint gives at
    /// its account, or is another loaded key's, is refused, and so is a MuSig2 wallet two of whose keys
    /// have one master fingerprint, loaded or not: the MuSig2 check
    /// matches one participant to each master fingerprint, so a second
    /// account of one seed never signs. A wallet of several keys of which
    /// one is no loaded key's own at its path, or one a key not of its
    /// own spends alone, is held back with [`Refusal::Unconfirmed`] until
    /// its sheet is confirmed (F254); [`Session::add_kept`] is the one
    /// way past it.
    pub fn add_policy(
        &mut self,
        c: &Compute,
        name: &str,
        policy: WalletPolicy,
        source: &str,
        origin: Origin,
    ) -> Result<usize, Refusal> {
        self.add_checked(c, name, policy, source, origin, &Default::default(), true)
    }

    /// Costs what `add_policy` costs, less the derivations in `derived`,
    /// and no confirmation sheet: the one named way a wallet is added
    /// without the person confirming it (F254), for wallets that are the
    /// person's own records, a vault's at unlock or loaded from the open
    /// vault, and the public samples. [`Session::add_policy`] for a
    /// wallet whose keys under the fingerprints of keys a load added were
    /// derived at their origin paths in that load's job: those are not
    /// derived again. Every other way in goes through `add_policy`.
    pub fn add_kept(
        &mut self,
        c: &Compute,
        name: &str,
        policy: WalletPolicy,
        source: &str,
        origin: Origin,
        derived: &faraday_keys::job::Derived,
    ) -> Result<usize, Refusal> {
        self.add_checked(c, name, policy, source, origin, derived, false)
    }

    /// Costs what [`Session::add_wallet`] costs, for a test's wallet
    /// already confirmed: no sheet (`testkit` only).
    #[cfg(feature = "testkit")]
    pub fn add_wallet_confirmed(
        &mut self,
        c: &Compute,
        name: &str,
        text: &str,
        source: &str,
        origin: Origin,
    ) -> Result<usize, Refusal> {
        self.add_wallet_kept(c, name, text, source, origin)
    }

    /// Costs what [`Session::add_policy`] costs, for a test's wallet
    /// already confirmed: no sheet (`testkit` only).
    #[cfg(feature = "testkit")]
    pub fn add_policy_confirmed(
        &mut self,
        c: &Compute,
        name: &str,
        policy: WalletPolicy,
        source: &str,
        origin: Origin,
    ) -> Result<usize, Refusal> {
        self.add_kept(c, name, policy, source, origin, &Default::default())
    }

    /// Costs what `add_policy` costs; `confirm`: a wallet of several keys
    /// of which one is no loaded key's own, or one a key not of its own
    /// spends alone, is held back until its sheet is confirmed.
    #[allow(
        clippy::too_many_arguments,
        reason = "the checked add is one function of the wallet and how it came"
    )]
    fn add_checked(
        &mut self,
        c: &Compute,
        name: &str,
        policy: WalletPolicy,
        source: &str,
        origin: Origin,
        derived: &faraday_keys::job::Derived,
        confirm: bool,
    ) -> Result<usize, Refusal> {
        let name = &*safe_name(c, name);
        let mut memo = confirm::Gives::new();
        if let Some(refusal) = self.refused_before_sheet(c, &policy, derived, &mut memo, true) {
            return Err(refusal);
        }
        let descriptor = same_wallet(&policy);
        if let Some(i) = self.held_wallet(&descriptor) {
            // How many labels were handed out is no part of which wallet
            // a silent payments record is: the one loaded takes a higher
            // count, never a lower.
            // Only the count: the held record keeps the rest, its test
            // network among it.
            if self.wallets[i].id() == policy.id()
                && let Some(mut held) = self.wallets[i].policy.silent().cloned()
                && let Some(new) = policy.silent()
                && new.labels > held.labels
            {
                held.labels = new.labels;
                self.wallets[i].set_policy(WalletPolicy::of_silent(held));
            }
            return Ok(i);
        }
        // No key here is forged: refused above.
        if confirm && self.held_for_sheet(&policy, &descriptor) {
            self.ask_wallet(c, name, policy, source, origin, None);
            return Err(Refusal::Unconfirmed);
        }
        self.follow(policy_network(&policy, self.network()))?;
        let key_checks = keycheck::KeyChecks::of(self, c, &policy, &memo);
        let mut wallet = Wallet::new(name, policy, source, origin);
        wallet.key_checks = key_checks;
        wallet.checked = self.keys.all().map(|k| k.fingerprint).collect();
        wallet.waited.clone_from(&self.waiting);
        self.wallets.push(wallet);
        Ok(self.wallets.len() - 1)
    }

    /// Costs what [`Self::forged_key`] costs, and for a silent payments
    /// record its check against a loaded key: why `policy` is refused
    /// before any sheet asks about it. `see_waiting` as in
    /// [`Self::forged_key`].
    fn refused_before_sheet(
        &self,
        c: &Compute,
        policy: &WalletPolicy,
        derived: &faraday_keys::job::Derived,
        memo: &mut confirm::Gives,
        see_waiting: bool,
    ) -> Option<Refusal> {
        if let Some(w) = policy.silent()
            && crate::silent::check_record(c, &self.keys, w).0
                == crate::silent::RecordCheck::Differs
        {
            return Some(silent_refusal(c, &self.keys, w));
        }
        if musig_one_seed(policy) {
            return Some(Refusal::MuSigOneSeed);
        }
        self.forged_key(c, policy, None, derived, memo, see_waiting)
    }

    /// Costs a key derivation (of at most `MAX_PATH_STEPS` steps) per key
    /// of `policy` under a loaded fingerprint, and for a silent payments
    /// record its check against the loaded keys
    /// ([`crate::silent::check_record`]); none for a waiting key. On Add
    /// to vault's press and again on its sheet's hold, never while
    /// drawing.
    ///
    /// Why Add to vault does not keep `policy`: what
    /// [`Self::refused_before_sheet`] refuses a load, except a key waiting
    /// for its backup under another fingerprint
    /// ([`Refusal::NotBackedUp`]), which the vault keeps.
    pub(crate) fn add_to_vault_refusal(
        &self,
        c: &Compute,
        policy: &WalletPolicy,
    ) -> Option<Refusal> {
        let derived = faraday_keys::job::Derived::default();
        let mut memo = confirm::Gives::new();
        self.refused_before_sheet(c, policy, &derived, &mut memo, false)
    }

    /// Costs what [`Self::held_wallet`] does: whether `policy`
    /// (`descriptor`, [`same_wallet`]) is kept only once the person has
    /// confirmed it (F254, F132). A wallet of several keys, or one a key
    /// not of its own can spend alone, is, unless the session holds it or
    /// it was confirmed already. A wallet whose every key is a loaded
    /// key's own at its path came from nothing outside and is not held
    /// back; the caller has refused a key under a loaded fingerprint that
    /// is not that key's.
    fn held_for_sheet(&self, policy: &WalletPolicy, descriptor: &str) -> bool {
        let outside = policy.keys().len() > 1 && !confirm::all_keys_loaded(self, policy);
        let other = matches!(policy.key_path(), Some(KeyPath::Other(_)));
        (outside || other)
            && self.held_wallet(descriptor).is_none()
            && !self.wallet_confirmed.iter().any(|d| **d == *descriptor)
    }

    /// Costs what [`Self::held_for_sheet`] does: whether Add to vault
    /// asks about `policy`, which [`Self::add_to_vault_refusal`] did not
    /// refuse, before the vault keeps it.
    pub(crate) fn asks_before_keeping(&self, policy: &WalletPolicy) -> bool {
        self.held_for_sheet(policy, &same_wallet(policy))
    }

    /// Costs what [`Rows::of`] costs: `policy` waits for its sheet, for
    /// the session, or with `vault` for that vault alone, unless it waits
    /// for the same already.
    pub(crate) fn ask_wallet(
        &mut self,
        c: &Compute,
        name: &str,
        policy: WalletPolicy,
        source: &str,
        origin: Origin,
        vault: Option<VaultId>,
    ) {
        let descriptor = same_wallet(&policy);
        if let Some(at) = self
            .wallet_asked
            .iter()
            .position(|a| a.vault == vault && same_wallet(&a.policy) == descriptor)
        {
            self.wallet_asked_last = at;
            return;
        }
        let rows = Rows::of(self, c, &policy, false);
        self.wallet_asked.push_back(WalletAsk {
            rows,
            name: Text::of(name),
            policy,
            source: Text::of(source),
            origin,
            replay: None,
            restore: false,
            vault,
        });
        self.wallet_asked_last = self.wallet_asked.len() - 1;
    }

    /// Toggles the person's tick of key `key` in the rows of the wallet
    /// whose sheet is next.
    pub(crate) fn ask_tick(&mut self, key: usize) {
        if let Some(a) = self.wallet_asked.front_mut() {
            a.rows.tick(key);
        }
    }

    /// The wallet whose sheet is next, if one waits.
    pub fn wallet_asked(&self) -> Option<&WalletAsk> {
        self.wallet_asked.front()
    }

    /// Notes the sheet the confirmation sheet opens over.
    pub(crate) fn wallet_cover(&mut self, sheet: Option<crate::Sheet>) {
        if self.wallet_covered.is_none() {
            self.wallet_covered = sheet;
        }
    }

    /// The sheet the confirmation sheet covered, taken.
    pub(crate) fn wallet_uncover(&mut self) -> Option<crate::Sheet> {
        self.wallet_covered.take()
    }

    /// The keys here whose backup is not done, as last worked out
    /// ([`Session::set_waiting`]). A lookup.
    pub(crate) fn waiting(&self) -> &[Fingerprint] {
        &self.waiting
    }

    /// The Inbox PSBT wallet `w` spends from, as last worked out
    /// ([`Session::set_psbt_wallets`]). A lookup.
    pub(crate) fn psbt_wallet(&self, w: usize) -> Option<usize> {
        self.psbt_wallets.get(w).copied().flatten()
    }

    /// The sign of what the PSBTs' wallets were last worked out from.
    pub(crate) fn psbt_wallets_inputs(&self) -> Option<u64> {
        self.psbt_wallets_inputs
    }

    /// Keeps the Inbox PSBT of each wallet and the sign it was worked out
    /// from.
    pub(crate) fn set_psbt_wallets(&mut self, found: Vec<Option<usize>>, inputs: u64) {
        self.psbt_wallets = found;
        self.psbt_wallets_inputs = Some(inputs);
    }

    /// How many wallets wait for their sheet.
    pub(crate) fn wallets_waiting(&self) -> usize {
        self.wallet_asked.len()
    }

    /// Which wallet of the run now waiting the next sheet is, from 1, and
    /// how many there are: "{i} of {n}".
    pub fn wallet_asked_place(&self) -> (usize, usize) {
        (
            self.wallet_asked_past + 1,
            self.wallet_asked_past + self.wallet_asked.len(),
        )
    }

    /// Notes that Restore asked for the wallet it was just told is held
    /// back, so it goes on from the wallet once the hold adds it.
    pub(crate) fn wallet_for_restore(&mut self) {
        if let Some(ask) = self.wallet_asked.get_mut(self.wallet_asked_last) {
            ask.restore = true;
        }
    }

    /// The press `action` asked for the wallet waiting first, if one
    /// waits and no press is noted for it yet.
    pub(crate) fn wallet_replay(&mut self, action: Action, inbox: u64) {
        if let Some(ask) = self.wallet_asked.front_mut() {
            ask.replay.get_or_insert((action, inbox));
        }
    }

    /// Writes the descriptor out. The person confirmed the wallet waiting
    /// first: the same wallet is added without asking again in this
    /// session, and it is handed back for its press to run again or, with
    /// none, to be added. `None` when none waits. Add to vault's hold
    /// counts only once its vault keeps the wallet
    /// ([`Self::wallet_confirmed_by_add`]).
    pub(crate) fn wallet_confirm(&mut self) -> Option<WalletAsk> {
        let ask = self.wallet_asked.pop_front()?;
        if ask.vault.is_none() {
            self.wallet_confirmed.push(same_wallet(&ask.policy));
        }
        self.wallet_asked_next();
        Some(ask)
    }

    /// Costs what [`Self::refused_before_sheet`] does: a vault kept
    /// `policy` on the hold of Add to vault's sheet. A load of it asks no
    /// more, unless a load would refuse it before its sheet now: that
    /// hold never stands in for the sheet such a load comes to once the
    /// refusal is gone.
    pub(crate) fn wallet_confirmed_by_add(&mut self, c: &Compute, policy: &WalletPolicy) {
        let derived = faraday_keys::job::Derived::default();
        let mut memo = confirm::Gives::new();
        if self
            .refused_before_sheet(c, policy, &derived, &mut memo, true)
            .is_none()
        {
            self.wallet_confirmed.push(same_wallet(policy));
        }
    }

    /// The person confirmed `policy` on the register sheet, whose hold is
    /// its confirmation: it is added without a sheet of its own.
    pub(crate) fn wallet_confirmed_by_register(&mut self, policy: &WalletPolicy) {
        let same = same_wallet(policy);
        if !self.wallet_confirmed.contains(&same) {
            self.wallet_confirmed.push(same);
        }
    }

    /// The wallet waiting first is not added; the next one waits.
    pub(crate) fn wallet_drop(&mut self) {
        if self.wallet_asked.pop_front().is_some() {
            self.wallet_asked_next();
        }
    }

    /// One wallet of the run is done: counted, and the count reset once
    /// none waits.
    fn wallet_asked_next(&mut self) {
        self.wallet_asked_past = if self.wallet_asked.is_empty() {
            0
        } else {
            self.wallet_asked_past + 1
        };
    }

    /// Who can spend `policy` by its key path, as a line (F132): `None`
    /// for a wallet with no script tree. A lookup: the key path was
    /// worked out when the policy was read.
    pub(crate) fn key_path_line(&self, policy: &WalletPolicy) -> Option<KeyPathLine> {
        Some(match policy.key_path()? {
            KeyPath::Unspendable => KeyPathLine {
                text: Text::of(EN.wallet_key_path_none),
                danger: false,
            },
            KeyPath::WalletKey(i) => {
                let fp = policy.keys().get(*i).and_then(|k| k.fingerprint());
                let holder = match fp {
                    Some(fp) => match self.keys.key_label(fp) {
                        Some(label) => key_line(fp, label),
                        None => fp_text(fp),
                    },
                    None => Text::fill(EN.wallet_key_n, &[&Text::num((i + 1) as u64)]),
                };
                KeyPathLine {
                    text: Text::fill(EN.wallet_key_path_alone, &[&holder]),
                    danger: false,
                }
            }
            KeyPath::Other(key) => KeyPathLine {
                text: key_path_danger(key),
                danger: true,
            },
        })
    }

    /// Sets the demo up (F81): the session takes only the public set
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
        self.wallet_asked.clear();
        self.wallet_asked_past = 0;
        self.wallet_asked_last = 0;
        self.wallet_covered = None;
        self.wallet_confirmed.clear();
        self.key_xpubs.borrow_mut().clear();
        self.silent_handed = Default::default();
    }

    /// Takes out the wallets made from typed words or seeds that hold the
    /// key under `fp`. The place and checksum of each, as the list was when
    /// it went (the earlier ones already gone). A lookup over the wallets.
    pub(crate) fn drop_typed_with(&mut self, fp: Fingerprint) -> Vec<(usize, Text, WalletId)> {
        let mut gone = Vec::new();
        let mut i = 0;
        while i < self.wallets.len() {
            let w = &self.wallets[i];
            let typed = matches!(w.origin, Origin::TypedWords | Origin::TypedSeeds);
            if typed && self.slots(w).iter().any(|s| s.fingerprint == Some(fp)) {
                let w = self.wallets.remove(i);
                gone.push((i, Text::of(w.checksum()), w.id()));
            } else {
                i += 1;
            }
        }
        gone
    }

    /// Whether wallet `w` holds the key the published-words caution
    /// waits on: its addresses are not drawn before the person answers.
    /// A lookup over the wallet's slots.
    pub(crate) fn awaits_caution(&self, w: &Wallet) -> bool {
        self.keys
            .published_pending()
            .is_some_and(|fp| self.slots(w).iter().any(|s| s.fingerprint == Some(fp)))
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
                held_by: self.keys.key_label(r.fingerprint).map(Text::of),
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
                        .map(Text::of),
                    key: Some(k.clone()),
                }
            })
            .collect()
    }

    /// The quorum: (seeds a spend needs, seeds or keys). Where the script states
    /// none, the fewest seeds of any way to spend, and every seed where
    /// the ways cannot be worked out, never one.
    pub fn quorum(wallet: &Wallet) -> (usize, usize) {
        let (m, n, _) = Self::spend_ways(wallet);
        (m, n)
    }

    /// How many of the seeds a spend needs are held here, at most `m`:
    /// where the script states no quorum, distinct seeds, else key slots.
    pub fn held_toward_quorum(&self, wallet: &Wallet) -> usize {
        let (m, _) = Self::quorum(wallet);
        let slots = self.slots(wallet);
        let held = slots.iter().filter(|s| s.held_by.is_some());
        let count = if Session::stated_quorum(wallet).is_some() {
            held.count()
        } else {
            let mut fps: Vec<_> = held.filter_map(|s| s.fingerprint).collect();
            fps.sort_by_key(|f| f.0);
            fps.dedup();
            fps.len()
        };
        count.min(m)
    }

    /// [`Session::quorum`] with the key count as n where the script states
    /// no quorum: for what is split or indexed by key.
    pub fn quorum_keys(wallet: &Wallet) -> (usize, usize) {
        let (m, n) = Self::quorum(wallet);
        match Session::stated_quorum(wallet) {
            Some(_) => (m, n),
            None => (m, wallet.policy().keys().len().max(1)),
        }
    }

    /// What the seeds of `wallet` must do to spend: `(signatures needed,
    /// distinct seeds where none is stated, ways to spend)`, the ways as masks of seed places (the places
    /// `Faraday::backup_seed_fps_of` lists). Where the script states a
    /// quorum, that quorum. A MuSig2 wallet needs every seed. A script
    /// with several ways to spend lists them. A wallet whose ways cannot
    /// be worked out needs every seed, so the plan never reads it as
    /// rebuilding from fewer.
    pub(crate) fn spend_ways(wallet: &crate::wallet::Wallet) -> (usize, usize, Vec<u64>) {
        let keys = wallet.policy().keys();
        if let Some((m, n)) = Session::stated_quorum(wallet) {
            return (m, n, Vec::new());
        }
        let mut fps: Vec<faraday_bip::keys::Fingerprint> = Vec::new();
        for f in keys.iter().filter_map(|k| k.fingerprint()) {
            if !fps.contains(&f) {
                fps.push(f);
            }
        }
        // Where no quorum is stated, n counts distinct seeds, the unit of m.
        let n = fps.len().max(1);
        let all = n;
        if fps.len() <= 1 {
            return (1, n, Vec::new());
        }
        // Each way as the seeds it needs; a way through a key with no
        // seed of its own, or past 64 seeds, cannot be met by seeds.
        let paths: Vec<u64> = wallet
            .waits()
            .paths
            .iter()
            .filter(|p| {
                // No seed can supply a hash's preimage, nor a way that
                // needs no key.
                !p.keys.is_empty() && !p.locks.contains(&faraday_bip::spend::Lock::Preimage)
            })
            .filter_map(|p| {
                p.keys.iter().try_fold(0u64, |mask, &k| {
                    let fp = keys.get(k)?.fingerprint()?;
                    let i = fps.iter().position(|&f| f == fp)?;
                    (i < 64).then_some(mask | 1 << i)
                })
            })
            .collect();
        let musig = matches!(
            wallet.policy().template(),
            faraday_bip::policy::Template::MuSig
        );
        if musig || paths.is_empty() {
            return (all, n, Vec::new());
        }
        let fewest = paths
            .iter()
            .map(|p| p.count_ones() as usize)
            .min()
            .unwrap_or(1);
        (fewest, n, paths)
    }

    /// [`Session::quorum`] where the script states one; `None` for a
    /// wallet whose keys must sign in a way no `m` of `n` says (MuSig2, a
    /// miniscript, a tree with several ways to spend).
    pub fn stated_quorum(wallet: &Wallet) -> Option<(usize, usize)> {
        wallet
            .policy
            .quorum()
            .or_else(|| wallet.policy.tapscript_quorum())
            .or_else(|| another_key_spends(wallet).then(|| wallet.policy.leaf_quorum())?)
    }

    /// The wallet's shape in a few words.
    pub fn shape(wallet: &Wallet) -> Text {
        let script = |t: ScriptType| match t {
            ScriptType::Legacy => EN.wallet_legacy,
            ScriptType::NestedSegwit => EN.wallet_nested_segwit,
            ScriptType::NativeSegwit => EN.wallet_native_segwit,
            ScriptType::Taproot => EN.wallet_taproot,
        };
        let (m, n) = Self::quorum(wallet);
        match Kind::of(&wallet.policy) {
            Kind::Single(t) => Text::fill(EN.wallet_single_key, &[script(t)]),
            Kind::Multi(Wrapper::Sh) => Text::fill(
                EN.wallet_of_legacy_multisig,
                &[&Text::num(m as u64), &Text::num(n as u64)],
            ),
            Kind::Multi(Wrapper::ShWsh) => Text::fill(
                EN.wallet_of_nested_segwit,
                &[&Text::num(m as u64), &Text::num(n as u64)],
            ),
            Kind::Multi(_) => Text::fill(
                EN.wallet_of_native_segwit,
                &[&Text::num(m as u64), &Text::num(n as u64)],
            ),
            Kind::TapMulti => Text::fill(
                EN.wallet_of_taproot_multisig,
                &[&Text::num(m as u64), &Text::num(n as u64)],
            ),
            Kind::Miniscript => Text::fill(
                EN.wallet_keys_miniscript,
                &[&Text::num(wallet.policy.keys().len() as u64)],
            ),
            Kind::Tree => Text::fill(
                EN.wallet_keys_taproot,
                &[&Text::num(wallet.policy.keys().len() as u64)],
            ),
            Kind::MuSig => Text::fill(
                EN.wallet_of_musig2,
                &[
                    &Text::num(wallet.policy.keys().len() as u64),
                    &Text::num(wallet.policy.keys().len() as u64),
                ],
            ),
            Kind::Silent => Text::of(EN.wallet_1_key_silent),
            Kind::Other => Text::of(EN.wallet_wallet),
        }
    }

    /// Costs an address derivation (through `address_on`). An address of
    /// the wallet, on the session's network. Derived once per wallet and
    /// kept on it ([`Wallet::address_on`]).
    pub fn address(&self, c: &Compute, wallet: &Wallet, change: bool, index: u32) -> Text {
        wallet.address_on(c, self.network(), change, index)
    }

    /// Costs an address derivation (through `address`). An address of the
    /// wallet as the screen shows it: the address itself, in Practice too.
    pub fn address_shown(&self, c: &Compute, wallet: &Wallet, change: bool, index: u32) -> Text {
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
    ) -> Vec<(usize, Text, crate::memory::nonces::NonceEntry)> {
        let mut out = Vec::new();
        for (i, input) in inspection.inputs.iter().enumerate() {
            let Some(round) = input.musig.as_ref() else {
                continue;
            };
            for p in faraday_psbt::musig::verified_rounds(c, psbt, i, round) {
                let name: Text = p.participant.serialize()[1..33]
                    .iter()
                    .flat_map(|b| {
                        let hex = b"0123456789abcdef";
                        [
                            hex[usize::from(b >> 4)] as char,
                            hex[usize::from(b & 15)] as char,
                        ]
                    })
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
    /// id are kept.
    pub fn inspect_inputs(&self) -> InspectInputs {
        InspectInputs {
            network: self.network(),
            keys: self.keys.all().map(|k| k.id.0).collect(),
            wallets: self.wallets.iter().map(|w| w.id()).collect(),
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
    ) -> Result<Vec<Fingerprint>, Text> {
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
    ) -> Result<Signed, Text> {
        let job = self.sign_job(c, psbt, Some(seed), musig, known, accepted)?;
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
    pub fn wallet_for(&self, c: &Compute, psbt: &Psbt) -> Option<usize> {
        self.spends_from(c, &PsbtSeen::of(psbt))
    }

    /// Costs what `script_at` costs. [`Session::wallet_for`] of a PSBT read
    /// before ([`PsbtSeen`]): lookups once each wallet keeps its scripts at
    /// the input's places, so a frame may ask it of every file in Files.
    pub(crate) fn spends_from(&self, c: &Compute, seen: &PsbtSeen) -> Option<usize> {
        let (spk, places) = seen.first.as_ref()?;
        self.wallet_at(c, spk, places)
    }

    /// Costs what `script_at` costs, for at most [`MAX_PSBT_PLACES`]
    /// places in all. For each input of `inner` not yet final, the loaded
    /// wallet whose script it spends, by the places that input's own key
    /// origins state (each input has its own wallet); an input of no known
    /// owner has none, and so has every input once the places read so far
    /// across the PSBT reach the bound, so a PSBT stuffed with inputs that
    /// each state many places cannot make a check derive in proportion to
    /// its size.
    pub(crate) fn wallets_of_open_inputs(
        &self,
        c: &Compute,
        inner: &faraday_bip::bitcoin::Psbt,
    ) -> Vec<Option<usize>> {
        let mut room = MAX_PSBT_PLACES;
        (0..inner.inputs.len())
            .map(|i| {
                let input = &inner.inputs[i];
                if input.final_script_sig.is_some() || input.final_script_witness.is_some() {
                    return None;
                }
                let (spk, places) = input_place(inner, i)?;
                if places.len() > room {
                    room = 0;
                    return None;
                }
                room -= places.len();
                self.wallet_at(c, &spk, &places)
            })
            .collect()
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
/// id). Two equal ones inspect a PSBT alike. Public.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InspectInputs {
    network: Network,
    keys: Vec<[u8; 32]>,
    wallets: Vec<WalletId>,
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

/// The most distinct places (`(change, index)`) one input's key origins may
/// state. A wallet's keys are read as `/<0;1>/*`, so all of an input's
/// keys are derived at one place of its wallet and a real input states
/// one; 20 leaves room for stray origins. Past it the input's places are
/// not read.
const MAX_INPUT_PLACES: usize = 20;

/// The most places read across all the inputs of one PSBT when each is
/// matched to its wallet. A real spend states one or two places an input
/// and the network's standard size limit allows a few thousand inputs;
/// past this the remaining inputs are matched to no wallet.
const MAX_PSBT_PLACES: usize = 8192;

/// Costs a classify. Input `i`'s spent script, and each `(change, index)`
/// its key origins state; no place at all when they state more than
/// [`MAX_INPUT_PLACES`], so an input stuffed with origins derives nothing
/// and is matched to no wallet.
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
                if places.len() == MAX_INPUT_PLACES {
                    return Some((spk, Vec::new()));
                }
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

/// Seeds a spend from this wallet needs, the unit
/// [`Session::held_toward_quorum`] counts in: the quorum the script
/// states, else the fewest seeds of any way to spend (every seed for a
/// MuSig2 wallet).
pub fn needed(wallet: &Wallet) -> usize {
    Session::quorum(wallet).0
}

/// Costs what `check_record` costs. Why silent payments record `w`, which
/// a loaded key contradicts, is refused: the loaded key under another
/// fingerprint that gives its points, else the key with its own fingerprint.
pub(crate) fn silent_refusal(
    c: &Compute,
    keys: &faraday_keys::Keys,
    w: &faraday_bip::silent_wallet::SilentWallet,
) -> Refusal {
    match crate::silent::other_key_gives(c, keys, w) {
        Some(k) => Refusal::SilentOtherKey {
            key: Text::of(&fp_text(k)),
            record: Text::of(&fp_text(w.fingerprint)),
        },
        None => Refusal::SilentDiffers(Text::of(&fp_text(w.fingerprint))),
    }
}

/// A wallet's descriptor, the form the same wallet read from a
/// descriptor, a wallet .json or a multisig config compares equal in:
/// the policy writes every hardened step `h` however the file wrote it
/// (F66). A `sortedmulti` sorts its keys whatever order they were
/// written in, so its keys are put in text order here; `multi` keeps the
/// order it was written in, which it pays to. Writes the descriptor out.
pub fn same_wallet(policy: &WalletPolicy) -> Text {
    let descriptor = policy.to_descriptor();
    // A silent payments wallet is its address, and the key and account
    // its record states: a record of another fingerprint or account over
    // the same address is another wallet.
    if let Some(w) = policy.silent() {
        return text!("{}{}", &*descriptor, w.origin());
    }
    const OPEN: &str = "sortedmulti(";
    let mut out = Text::of("");
    let mut rest: &str = &descriptor;
    while let Some(at) = rest.find(OPEN) {
        let args = &rest[at + OPEN.len()..];
        let Some(end) = args.find(')') else { break };
        let mut parts: Vec<&str> = args[..end].split(',').collect();
        // The threshold stays first; only the keys after it are sorted.
        if parts.len() > 1 {
            parts[1..].sort_unstable();
        }
        out.push_str(&rest[..at + OPEN.len()]);
        for (i, part) in parts.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            out.push_str(part);
        }
        rest = &args[end..];
    }
    out.push_str(rest);
    out
}

impl FileKind {
    /// A file `read_keys` reads, a secret beside its key or not.
    pub fn is_key(self) -> bool {
        matches!(self, FileKind::Key | FileKind::KeyWithSecret)
    }
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
    /// An account key file that holds a secret beside its key (seed words
    /// or an extended private key in a note, a comment or a line of its
    /// own): it imports as the key file it is, wherever a key file does,
    /// and is a secret. Nothing secret in it is read.
    KeyWithSecret,
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
    /// A whole key as one line of text: an extended private key, or a
    /// master seed in hex of a BIP-32 length (`docs/KEY-FORMS.md` §6.3).
    /// A hex file may be a hash; it loads only when the person opens it.
    KeyLine,
    /// Plain text Faraday reads as nothing else, such as recovery codes:
    /// kept as a note in a vault.
    Text,
    /// An ASCII-armored GPG public key or signature: public (F22). A
    /// revocation certificate is not (F32).
    Gpg,
    /// A SQLite database, which may be a Bitcoin Core wallet: read in
    /// Tools' Read a Bitcoin Core wallet. It may hold private keys.
    CoreWallet,
    /// A Krux encrypted code's bytes, as the Krux tool writes them to a
    /// stick (`docs/KEF.md` §8): sealed under its passphrase and read by
    /// Open, as Add a key's Krux code form reads a file.
    KruxCode,
    /// Nothing this build reads.
    Other,
}

/// A refusal that names input `i`: its wording with the input number.
fn at_input(wording: &str, i: usize) -> Text {
    Text::fill(wording, &[&Text::num(i as u64)])
}

/// What collecting signatures still asks for: "Collect 2 signatures"
/// before any is in, "Collect 1 more signature" after. "More" is said
/// only once there is one.
pub fn collect_line(missing: usize, collected: usize) -> Text {
    Text::fill(
        EN.wallet_collect,
        &[
            &Text::num(missing as u64),
            &(if collected == 0 {
                Text::default()
            } else {
                text!(" {}", EN.wallet_more)
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

/// The length in bytes under which a piece of text is no key, seed or
/// share, unless it starts with an extended private key's version:
/// BIP-392's `spspend1` marker, 8 bytes, is the shortest of the rest, while
/// `secret_text` names a text by its 4-byte private version alone.
const MIN_SECRET_LEN: usize = 8;

/// Whether `text` is too short for any detector, which is not so for one
/// that starts with a private version.
fn too_short_to_be_secret(text: &str) -> bool {
    text.len() < MIN_SECRET_LEN && !faraday_intake::starts_with_private_version(text)
}

/// Costs what `secret_text` costs. Whether a piece of text, a user ID
/// or an armour header, is a seed's words, an extended private key or a
/// share: the detectors that read a plain text file.
fn text_may_be_secret(c: &Compute, text: &str) -> bool {
    // Apart from a private version's 4 bytes, the shortest secret is a
    // silent payments spend key's `spspend1`, 8 bytes; a shorter piece is
    // none, and a file of short lines is not read line by line at the
    // cost of each.
    if too_short_to_be_secret(text.trim()) {
        return false;
    }
    faraday_intake::secret_text(c, text).is_some()
        || seed_in_words(c, &crate::forms::file_words(text))
        || crate::restore::is_share(text)
        || crate::inbox::part_of(text).is_some()
}

/// Costs what `text_may_be_secret` costs on the body and on each of its
/// lines, and a `words_across` over them. The kind a signed message's
/// file takes when its body holds a secret: a seed's words, a seed part
/// or entries as in a plain text file, and a secret wallet file for a
/// private key, an Electrum file with a seed, or a seed, a seed part or an
/// authenticator's export beside other text, a `#` line among them. None
/// for any other body, a multisig share among them, which holds public
/// keys and stays a message.
fn secret_body_kind(c: &Compute, body: &str) -> Option<FileKind> {
    if too_short_to_be_secret(body.trim()) {
        return None;
    }
    if crate::inbox::part_of(body).is_some() {
        return Some(FileKind::SeedPart);
    }
    if seed_in_words(c, &crate::forms::file_words(body)) {
        return Some(FileKind::Words);
    }
    if faraday_codec::classify::looks_like_entries(body)
        || faraday_intake::is_otpauth_migration(body)
    {
        return Some(FileKind::Entries);
    }
    (faraday_intake::secret_text(c, body).is_some()
        || faraday_intake::wallet::holds_secret(body)
        || lines_hold_secret(c, &body.lines().collect::<Vec<_>>()))
    .then_some(FileKind::SecretWallet)
}

/// A line without its comment marks, however many.
fn bare(l: &str) -> &str {
    l.trim()
        .trim_start_matches(|ch: char| ch == '#' || ch.is_whitespace())
}

/// The pieces of a line a secret can sit in beside other text: each
/// white-space separated word, cut at quotes, commas and brackets, and
/// what follows a label's colon. At most 64.
fn fragments(l: &str) -> impl Iterator<Item = &str> {
    l.split(|ch: char| {
        ch.is_whitespace()
            || matches!(
                ch,
                '"' | '\''
                    | '`'
                    | '\u{201c}'
                    | '\u{201d}'
                    | ','
                    | ';'
                    | '('
                    | ')'
                    | '['
                    | ']'
                    | '<'
                    | '>'
            )
    })
    .filter(|f| !f.is_empty())
    .flat_map(|f| {
        std::iter::once(f).chain(
            f.split_once(':')
                .map(|(_, r)| r)
                .filter(|r| !r.is_empty() && !r.starts_with("//")),
        )
    })
    .take(64)
}

/// Costs `secret_text` on the line and on each fragment of it, and a
/// part's parse of each. Whether one line is a secret by itself, as the
/// whole line or after a label, in quotes or beside prose.
fn line_is_secret(c: &Compute, l: &str) -> bool {
    let is = |t: &str| {
        !too_short_to_be_secret(t)
            && (faraday_intake::secret_text(c, t).is_some()
                || faraday_intake::is_otpauth_migration(t)
                || crate::inbox::part_of(t).is_some())
    };
    is(bare(l)) || fragments(bare(l)).any(is)
}

/// Costs a `line_is_secret` on each line, a seed part's parse of each
/// two or three lines joined, and a `words_across` over the lines.
/// Whether `lines` hold a secret: a line that is one or has one in it, a
/// SLIP-39 share wrapped over lines, or a seed's words among other text,
/// punctuation read as a space.
fn lines_hold_secret(c: &Compute, lines: &[&str]) -> bool {
    if lines.iter().any(|l| line_is_secret(c, l)) {
        return true;
    }
    let bares: Vec<&str> = lines
        .iter()
        .map(|l| bare(l))
        .filter(|l| !l.is_empty())
        .collect();
    for n in [2, 3] {
        for w in bares.windows(n) {
            let joined = Text::joined(w, " ");
            if crate::inbox::part_of(joined.as_str()).is_some() {
                return true;
            }
        }
    }
    if slip39_across(bares.iter().copied()) {
        return true;
    }
    let spaced: Vec<Text> = lines
        .iter()
        .map(|l| {
            let mut t = Text::with_room(l.len());
            for ch in bare(l).chars() {
                t.push(if ch.is_ascii_punctuation() { ' ' } else { ch });
            }
            t
        })
        .collect();
    words_across(spaced.iter().map(Text::as_str))
}

/// Costs `slip39::index_of` for each word of 4 to 8 letters, and a
/// checksum for each of the 14 lengths at every word of a run (a run past
/// 48 is dropped). Whether the words among `parts` hold a SLIP-39 share:
/// a run of consecutive list words in any case, broken by any other
/// token (a place number does not break it), whose last 20 to 33 are a
/// share, however many lines it is over.
fn slip39_across<'a>(parts: impl Iterator<Item = &'a str>) -> bool {
    use faraday_bip::slip39;
    let mut run = zeroize::Zeroizing::new(Vec::<u16>::with_capacity(slip39::MAX_WORDS));
    let mut seen = 0usize;
    for part in parts {
        let words = crate::forms::file_words(part);
        for w in words.as_str().split_whitespace() {
            let mut low = [0u8; 8];
            let index = if (4..=8).contains(&w.len()) && w.bytes().all(|b| b.is_ascii_alphabetic())
            {
                low[..w.len()].copy_from_slice(w.as_bytes());
                low.make_ascii_lowercase();
                std::str::from_utf8(&low[..w.len()])
                    .ok()
                    .and_then(slip39::index_of)
            } else {
                None
            };
            let Some(i) = index else {
                run.clear();
                seen = 0;
                continue;
            };
            seen += 1;
            if seen > 48 {
                continue;
            }
            if run.len() == slip39::MAX_WORDS {
                run.remove(0);
            }
            run.push(i);
            for n in slip39::MIN_WORDS..=slip39::MAX_WORDS {
                if run.len() >= n && slip39::Share::from_indices(&run[run.len() - n..]).is_ok() {
                    return true;
                }
            }
        }
    }
    false
}

/// The lines of a multisig share that `restore::parts` does not read as a
/// quorum, a path or a key: comments, a name, a format and any other key.
fn share_skipped_lines(text: &str) -> Vec<&str> {
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .filter(|l| {
            let Some((k, _)) = l.split_once(':') else {
                return true;
            };
            let k = k.trim();
            l.starts_with('#')
                || !(k.eq_ignore_ascii_case("policy")
                    || k.eq_ignore_ascii_case("derivation")
                    || (k.len() == 8 && k.chars().all(|ch| ch.is_ascii_hexdigit())))
        })
        .collect()
}

/// Costs a `read_keys` on the line: base58 decodes. Whether a line of a
/// key file is a key `read_keys` reads.
fn is_key_line(line: &str) -> bool {
    !too_short_to_be_secret(line) && !crate::create::read_keys(line).is_empty()
}

/// Costs a pass over the words of every part, with a checksum for each
/// of five lengths at every list word of a run (a run past 48 is
/// dropped, and a token that cannot be a list word is not looked up).
/// Whether the words among `parts` hold a seed: a run of consecutive list
/// words, in the order they come, broken by any other token (a place
/// number does not break it), of which some 12, 15, 18, 21 or 24 in a row
/// are a mnemonic. A seed spread over lines, numbered or in a JSON array
/// is found with other text around it, headings and labels that are list
/// words too. Each part is read as a file's words are (place numbers
/// dropped).
fn words_across<'a>(parts: impl Iterator<Item = &'a str>) -> bool {
    // The words of every part, each part read as a file's words are,
    // held in wiping room.
    let parts: Vec<Text> = parts.map(crate::forms::file_words).collect();
    // One pass counts, for each list, the words that are on it; a list
    // with fewer than 12 of them cannot hold a seed and is not walked.
    let mut on_list = vec![0usize; Language::ALL.len()];
    for part in &parts {
        for w in part.as_str().split_whitespace() {
            for e in crate::forms::other_list_entries(w).iter() {
                on_list[e.0 as usize] += 1;
            }
        }
    }
    Language::ALL.iter().enumerate().any(|(at, &lang)| {
        (lang == Language::English || on_list[at] >= 12) && words_across_in(lang, &parts)
    })
}

/// Costs what `words_across` costs for one list. Whether the words in
/// `parts` hold a seed written in `lang`'s list.
fn words_across_in(lang: Language, parts: &[Text]) -> bool {
    // The word numbers of the run: a wiping buffer sized once.
    let mut run = zeroize::Zeroizing::new(Vec::<u16>::with_capacity(25));
    let mut overlong = false;
    let mut seen = 0usize;
    for part in parts {
        for w in part.as_str().split_whitespace() {
            let index = if lang == Language::English {
                // The list's words are 3 to 8 ASCII letters, read in any
                // case as a mnemonic is, folded in a buffer of the word's
                // own size.
                let mut low = [0u8; 8];
                if (3..=8).contains(&w.len()) && w.bytes().all(|b| b.is_ascii_alphabetic()) {
                    low[..w.len()].copy_from_slice(w.as_bytes());
                    low.make_ascii_lowercase();
                    std::str::from_utf8(&low[..w.len()])
                        .ok()
                        .and_then(|l| crate::forms::typed_index(Language::English, l))
                } else {
                    None
                }
            } else {
                crate::forms::other_list_index(lang, w)
            };
            let Some(i) = index else {
                run.clear();
                overlong = false;
                seen = 0;
                continue;
            };
            if overlong {
                continue;
            }
            if run.len() == 24 {
                // Slide: the window of the last 24 words is the longest
                // that is checked.
                run.remove(0);
            }
            seen += 1;
            if seen > 48 {
                // Past 48 words of one run, no more are checked.
                overlong = true;
                continue;
            }
            run.push(i);
            for n in [12, 15, 18, 21, 24] {
                if run.len() >= n
                    && faraday_keys::words::from_indices(lang, &run[run.len() - n..]).is_ok()
                {
                    return true;
                }
            }
        }
    }
    false
}

/// Costs a parse of the words in each list that holds half of them. Whether `words` (as
/// [`crate::forms::file_words`] gives them) are a seed in any list the
/// device imports: English as typed, any other as a sentence is parsed.
fn seed_in_words(c: &Compute, words: &str) -> bool {
    seed_mnemonic(c, words).is_some()
}

/// Costs a parse of the words in each list that holds half of them. The
/// seed `words` hold (as [`crate::forms::file_words`] gives them) and
/// is read in, found as [`seed_in_words`] finds it.
pub(crate) fn seed_mnemonic(c: &Compute, words: &str) -> Option<faraday_bip::bip39::Mnemonic> {
    if let Ok(m) = crate::forms::typed_mnemonic(Language::English, words) {
        return Some(m);
    }
    let n = words.split_whitespace().count();
    if !matches!(n, 12 | 15 | 18 | 21 | 24) {
        return None;
    }
    // One lookup per word counts it for every list; only a list that holds
    // at least half of the words is parsed.
    let mut on_list = vec![0usize; Language::ALL.len()];
    for w in words.split_whitespace() {
        for e in crate::forms::other_list_entries(w).iter() {
            on_list[e.0 as usize] += 1;
        }
    }
    Language::ALL
        .iter()
        .enumerate()
        .filter(|&(at, &l)| l != Language::English && on_list[at] * 2 >= n)
        .find_map(|(_, &l)| faraday_keys::words::parse(c, l, words).ok())
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
    /// The outpoints a PSBT or a finished transaction spends, sorted;
    /// empty for any other file.
    pub outpoints: Vec<OutPoint>,
    /// The id of the unsigned transaction a PSBT or a finished
    /// transaction holds (a finished one with its signatures taken off,
    /// as a PSBT holds it); `None` for any other file.
    pub unsigned_txid: Option<Txid>,
}

impl From<FileKind> for Classified {
    fn from(kind: FileKind) -> Classified {
        Classified {
            kind,
            psbt: None,
            wallet: None,
            outpoints: Vec::new(),
            unsigned_txid: None,
        }
    }
}

/// The id of `tx` with its signatures taken off, as a PSBT holds it.
fn unsigned_id(mut tx: Transaction) -> Txid {
    for i in &mut tx.input {
        i.script_sig = faraday_bip::bitcoin::ScriptBuf::new();
        i.witness = faraday_bip::bitcoin::Witness::new();
    }
    tx.compute_txid()
}

/// The outpoints `tx` spends, sorted.
fn sorted_outpoints(tx: &Transaction) -> Vec<OutPoint> {
    let mut out: Vec<OutPoint> = tx.input.iter().map(|i| i.previous_output).collect();
    out.sort();
    out
}

/// Costs a parse of the first line. The text as a signed message, when
/// its first line is an address: the one reading of "this is a signed
/// message", for what Files calls the file and for Add to vault.
pub(crate) fn signed_message(text: &str) -> Option<faraday_psbt::message::SignedText> {
    // An address is at least 14 characters and has no white space; a
    // shorter first line (a word of a seed written one to a line) is
    // not handed to bitcoin's address parser, which would decode it
    // into a buffer of its own.
    faraday_psbt::message::parse_signed(text).filter(|m| {
        m.address.len() >= 14
            && !m.address.contains(char::is_whitespace)
            && m.address
                .parse::<faraday_bip::bitcoin::Address<_>>()
                .is_ok()
    })
}

/// Costs what [`classify`] costs, and for text, a few passes over it and
/// a check of each line, each linear in the file's length (a file of
/// many short lines or of one line of many short tokens costs by its
/// length, held by the operation `classify-short-lines-32kib` of
/// `tools/op-cost`). [`classify`], keeping what it read to
/// tell: a PSBT is parsed once and a wallet file read once when a file
/// comes in.
pub fn classify_read(c: &Compute, name: &str, bytes: &[u8]) -> Classified {
    if faraday_codec::vault_file::is_vault_file(bytes) {
        return FileKind::Vault.into();
    }
    if bytes.starts_with(b"%PDF-") {
        return FileKind::Pdf.into();
    }
    if crate::core_wallet::is_sqlite(bytes) {
        return FileKind::CoreWallet.into();
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
            outpoints: sorted_outpoints(p.unsigned_tx()),
            unsigned_txid: Some(p.unsigned_tx().compute_txid()),
        };
    }
    if let Ok(text) = std::str::from_utf8(bytes) {
        if text.starts_with("faraday-sheet 1") {
            return FileKind::Sheet.into();
        }
        if let Some(block) = faraday_pgp::read_public(bytes)
            && !block.texts.iter().any(|t| text_may_be_secret(c, t))
            && !words_across(block.texts.iter().map(Text::as_str))
        {
            return FileKind::Gpg.into();
        }
        if let Some(m) = signed_message(text) {
            // The body is checked for a secret only here, so a text file
            // that is no signed message does not pay for it.
            return secret_body_kind(c, m.message.as_str())
                .unwrap_or(FileKind::Message)
                .into();
        }
        if crate::restore::is_share(text) {
            // `parts` skips comments and lines it does not know: a seed
            // there makes the file a secret one.
            if lines_hold_secret(c, &share_skipped_lines(text)) {
                return FileKind::SecretWallet.into();
            }
            return FileKind::Share.into();
        }
        if faraday_intake::wallet::holds_secret(text) {
            return FileKind::SecretWallet.into();
        }
        if let Ok(w) = read_wallet(c, text) {
            // Secret text beside a descriptor, in a JSON member the reader
            // skips or a comment, makes the wallet file a secret one. A
            // file that is not a wallet keeps the kind it has below.
            let unread = faraday_intake::wallet::unread_texts(text);
            if unread.iter().any(|t| text_may_be_secret(c, t.as_str()))
                || words_across(unread.iter().map(Text::as_str))
            {
                return Classified {
                    kind: FileKind::SecretWallet,
                    psbt: None,
                    wallet: Some(w),
                    outpoints: Vec::new(),
                    unsigned_txid: None,
                };
            }
            return Classified {
                kind: FileKind::Wallet,
                psbt: None,
                wallet: Some(w),
                outpoints: Vec::new(),
                unsigned_txid: None,
            };
        }
        if crate::create::read_key(text).is_some() {
            // `read_keys` drops every line it cannot read: a seed beside
            // the key, on a line or in a JSON member, makes a secret file.
            let unread = faraday_intake::wallet::unread_texts(text);
            let skipped = text.lines().map(str::trim).filter(|l| !is_key_line(l));
            if unread
                .iter()
                .map(Text::as_str)
                .chain(text.lines())
                .any(|t| text_may_be_secret(c, t.trim().trim_start_matches('#').trim()))
                || words_across(skipped)
                || words_across(unread.iter().map(Text::as_str))
            {
                return FileKind::KeyWithSecret.into();
            }
            return FileKind::Key.into();
        }
        if seed_in_words(c, &crate::forms::file_words(text)) {
            return FileKind::Words.into();
        }
        // Google Authenticator's export holds every TOTP secret of the
        // account: kept as entries are, never filed as text.
        if faraday_codec::classify::looks_like_entries(text)
            || faraday_intake::is_otpauth_migration(text)
        {
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
            && let Some(tx) = faraday_psbt::transaction::read_raw(t.as_bytes())
        {
            return Classified {
                outpoints: sorted_outpoints(&tx),
                unsigned_txid: Some(unsigned_id(tx)),
                ..FileKind::Transaction.into()
            };
        }
        if crate::one_line::line_form(c, text).is_some() {
            return FileKind::KeyLine.into();
        }
        if is_text(text) {
            return FileKind::Text.into();
        }
    }
    // The weakest test there is (an envelope has no magic number), so the
    // last: a `.kef` file of the bytes the Krux tool writes, never a text
    // form and never another name (a renamed code stays `Other`, which From
    // Files still lists).
    if name
        .len()
        .checked_sub(4)
        .and_then(|i| name.get(i..))
        .is_some_and(|ext| ext.eq_ignore_ascii_case(".kef"))
        && faraday_codec::kef::recognize(bytes)
            .is_some_and(|r| r.form == faraday_codec::kef::Form::Raw)
    {
        return FileKind::KruxCode.into();
    }
    FileKind::Other.into()
}

/// One transaction being signed, from the moment it is loaded.
#[derive(Clone)]
pub struct Spend {
    /// The PSBT as it stands: every signature collected so far.
    pub psbt: Psbt,
    /// The file it came from.
    pub source: Text,
    /// The unsigned transaction's id, which every collected copy must share.
    pub txid: Txid,
    /// Keys of this session that signed it here.
    pub signed_here: Vec<Fingerprint>,
    /// The MuSig2 public nonces this device's passes wrote in this
    /// session, for its own keys (round 1 draws and mends): a version 2
    /// file's older nonce for such a key on such an input is replaced by
    /// exactly that value (`PsbtV2::merge_signed_by`).
    pub nonces_written: Vec<faraday_psbt::NonceWritten>,
    /// The MuSig2 partial signatures this device wrote in this session: the
    /// only entries in the file it reads as its own (F220).
    pub wrote: Vec<Written>,
    /// Fingerprints whose signatures arrived from other devices, with the
    /// file each came in.
    pub collected: Vec<(Fingerprint, Text)>,
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
    /// The id of the loaded wallet the transaction was last read as
    /// spending from, so that Check is owed again when another wallet
    /// takes its place (`SpendState::keep_fresh`).
    pub wallet_id: Option<WalletId>,
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
            source: Text::of(source),
            txid,
            signed_here: Vec::new(),
            nonces_written: Vec::new(),
            wrote: Vec::new(),
            collected: Vec::new(),
            finished: None,
            nonce_hold: false,
            v2: None,
            wallet_id: None,
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
    pub fn take_signed(
        &mut self,
        psbt: Psbt,
        fps: Vec<Fingerprint>,
        nonces: Vec<faraday_psbt::NonceWritten>,
        nonce_only: bool,
    ) {
        for n in nonces {
            self.nonces_written
                .retain(|w| (w.input, w.participant) != (n.input, n.participant));
            self.nonces_written.push(n);
        }
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
        let owners: Vec<Option<usize>> = session.wallets_of_open_inputs(c, inner);
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
    ) -> Result<Vec<Fingerprint>, Text> {
        let other = (read_psbt(c, bytes)).ok_or_else(|| Text::of(EN.wallet_not_a_psbt))?;
        if other.unsigned_tx().compute_txid() != self.txid {
            return Err(Text::fill(EN.wallet_psbt_other_transaction, &[source]));
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
            return Err(Text::fill(EN.wallet_sighash_field_other, &[source]));
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
            return Err(at_input(EN.psbt_warn_witness_mismatch, i));
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
            .map_err(|e| Text::fill(EN.wallet_cannot_combine, &[&text!("{e}")]))?;
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
            return Err(at_input(EN.psbt_final_prev_disagree, i));
        }
        // `combine` keeps the fields it has no type for, MuSig2's among them.
        let merged = Psbt::from(merged);
        // A final input that cannot stand, rows or none, is no signing: the
        // spend's own PSBT holding one blames no file.
        match refusal_of(merged.inner()) {
            Some(FinalRefusal::Signature(_)) => {
                return Err(Text::fill(EN.wallet_a_signature, &[source]));
            }
            Some(FinalRefusal::Text(why)) => return Err(why),
            None => {}
        }
        let inspection = session.inspect(c, &merged);
        if inspection
            .present_signatures()
            .any(|s| s.verdict.is_invalid())
        {
            return Err(Text::fill(EN.wallet_a_signature, &[source]));
        }
        if inspection
            .warnings
            .iter()
            .any(|w| matches!(w.kind, faraday_psbt::WarningKind::SighashMismatch { .. }))
        {
            return Err(Text::fill(EN.wallet_sighash_other, &[source]));
        }
        self.psbt = merged;
        let added: Vec<Fingerprint> = (self.counted(c, session, Some(&inspection)))
            .into_iter()
            .filter(|fp| !before.contains(fp))
            .collect();
        for fp in &added {
            self.collected.push((*fp, Text::of(source)));
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
    pub fn refused(&self) -> Option<Text> {
        refusal_of(self.psbt.inner()).map(FinalRefusal::text)
    }

    /// Costs what [`Spend::refused`] costs, and a finalize of a copy.
    /// Finishes the transaction when it has enough signatures.
    pub fn finish(&mut self) -> Result<(), Text> {
        if self.nonce_hold {
            return Err(Text::of(EN.spend_nonce_held));
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
            Ok(None) => Err(Text::of(EN.wallet_not_enough_signatures)),
            Err(e) => Err(crate::review::sign_error_text(&e)),
        }
    }

    /// Why the transaction cannot be finished yet, when it cannot.
    /// `refused` is what [`Spend::refused`] said for this PSBT, kept where
    /// the spend changes (`SpendState::refused`): a lookup, and costly
    /// work of the finalize of a copy.
    pub fn finish_reason(&self, refused: Option<&str>) -> Option<Text> {
        if self.nonce_hold {
            return Some(Text::of(EN.spend_nonce_held));
        }
        if let Some(why) = refused {
            return Some(Text::of(why));
        }
        let mut copy = self.psbt.clone();
        match faraday_psbt::finalize(&mut copy) {
            Ok(Some(_)) => None,
            Ok(None) => Some(Text::of(EN.wallet_script_unsatisfied)),
            Err(e) => Some(crate::review::sign_error_text(&e)),
        }
    }

    /// The finished transaction as hex.
    pub fn finished_hex(&self) -> Option<Text> {
        self.finished.as_ref().map(|t| Text::of(&serialize_hex(t)))
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
fn lie_refusal(source: &str, input: usize, bare: bool) -> Text {
    let why = if bare {
        EN.wallet_outputs_differ_unproven
    } else {
        EN.wallet_file_states_false
    };
    Text::fill(why, &[source, &Text::num(input as u64)])
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
    fn refusal(self, source: &str) -> Text {
        let (why, i) = match self {
            Conflict::Finals(i) => (EN.wallet_finals_differ, i),
            Conflict::Rows(i) => (EN.wallet_signature_unchecked, i),
            Conflict::Scripts(i) => (EN.wallet_scripts_differ, i),
            Conflict::False(i) => {
                return Text::fill(EN.psbt_final_prev_disagree, &[&Text::num(i as u64)]);
            }
        };
        Text::fill(why, &[source, &Text::num(i as u64)])
    }
}

/// Whether an input holds a final scriptSig or witness.
fn has_final(input: &faraday_bip::bitcoin::psbt::Input) -> bool {
    input.final_script_sig.is_some() || input.final_script_witness.is_some()
}

/// Why a cosigner's file is refused for a final input that fails its
/// output: a signature the output refuses names the file as a bad
/// signature does; any other fault is the input's.
fn file_witness_refusal(file: &faraday_bip::bitcoin::Psbt, source: &str) -> Option<Text> {
    use faraday_psbt::verify::{final_signature_wrong, witness_mismatch, witness_shape_mismatch};
    let i = (0..file.inputs.len()).find(|i| witness_mismatch(file, *i))?;
    if witness_shape_mismatch(file, i) {
        return Some(at_input(EN.psbt_warn_witness_mismatch, i));
    }
    if final_signature_wrong(file, i) {
        return Some(Text::fill(EN.wallet_a_signature, &[source]));
    }
    Some(Text::fill(
        EN.psbt_final_signature_refused,
        &[&Text::num(i as u64)],
    ))
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
        let text = |t: &str| Some(FinalRefusal::Text(Text::fill(t, &[&Text::num(i as u64)])));
        // The script refuses the final fields: a signature of the file's
        // where one reads wrong, else the fields alone.
        let fields = |i: usize, pass: &mut Option<faraday_psbt::verify::Pass>| {
            let line = Text::fill(EN.psbt_final_signature_refused, &[&Text::num(i as u64)]);
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
    Signature(Text),
    Text(Text),
}

impl FinalRefusal {
    fn text(self) -> Text {
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

/// An address of a loaded key by its full path: what Sign a message signs
/// for when the address check found it outside a wallet's receive chain.
#[derive(Clone)]
pub struct MessageAt {
    /// The key's fingerprint.
    pub fingerprint: Fingerprint,
    /// The full path: the account or origin, the chain and the index.
    pub path: faraday_bip::bitcoin::bip32::DerivationPath,
    /// The script the address is of.
    pub script: ScriptType,
    /// Whether the address is on the change chain.
    pub change: bool,
    /// The address's index on its chain.
    pub index: u32,
    /// The address found, as the check shows it.
    pub address: Text,
    /// The key's fingerprint and the full path, as the check's Path row
    /// says it.
    pub line: Text,
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
    ) -> Result<faraday_psbt::message::Signed, Text> {
        let w = self
            .wallets
            .get(wallet)
            .ok_or_else(|| Text::of(EN.wallet_no_such_wallet))?;
        let Kind::Single(script) = Kind::of(&w.policy) else {
            return Err(Text::of(EN.wallet_messages_are));
        };
        let key = w
            .policy
            .keys()
            .first()
            .ok_or_else(|| Text::of(EN.wallet_the_wallet_has))?;
        let fp = key
            .fingerprint()
            .ok_or_else(|| Text::of(EN.wallet_the_wallet_s_xpub))?;
        let place = self
            .keys
            .position(fp)
            .ok_or_else(|| Text::of(EN.wallet_the_wallet_s_key))?;
        let origin = key.path().ok_or_else(|| Text::of(EN.wallet_the_wallet_s))?;
        let path = origin
            .child(
                faraday_bip::bitcoin::bip32::ChildNumber::from_normal_idx(0)
                    .map_err(|e| text!("{e}"))?,
            )
            .child(
                faraday_bip::bitcoin::bip32::ChildNumber::from_normal_idx(index)
                    .map_err(|e| text!("{e}"))?,
            );
        let signed = self
            .keys
            .sign_message(c, place, &path, script, message, format)
            .ok_or_else(|| Text::of(EN.wallet_the_wallet_s_key))?
            .map_err(|e| Text::of(crate::state::message_error_text(e)))?;
        if signed.address != *self.address(c, w, false, index) {
            return Err(Text::fill(EN.wallet_not_your_key, &[&fp_text(fp)]));
        }
        Ok(signed)
    }

    /// Costs two key derivations (one to check, one inside the signing), an
    /// address derivation and a signature. Signs `message` with the loaded
    /// key at `at`'s path, once the address that key gives there on this
    /// network is the address found.
    pub fn sign_message_at(
        &self,
        c: &Compute,
        at: &MessageAt,
        message: &str,
        format: faraday_psbt::message::Format,
    ) -> Result<faraday_psbt::message::Signed, Text> {
        let not_loaded = || Text::fill(EN.message_key_not_loaded, &[&fp_text(at.fingerprint)]);
        let place = self.keys.position(at.fingerprint).ok_or_else(not_loaded)?;
        let cannot = || Text::fill(EN.message_key_cannot_derive, &[&fp_text(at.fingerprint)]);
        let gives = self
            .keys
            .xpub_at(c, place, &at.path)
            .map(|x| {
                let address = crate::explore::address_of(
                    &faraday_bip::bitcoin::CompressedPublicKey(x.public_key),
                    at.script,
                    self.network().into(),
                );
                text!("{address}")
            })
            .ok_or_else(cannot)?;
        let refused = || Text::fill(EN.message_key_not_this_address, &[&fp_text(at.fingerprint)]);
        if gives != at.address {
            return Err(refused());
        }
        let signed = self
            .keys
            .sign_message(c, place, &at.path, at.script, message, format)
            .ok_or_else(not_loaded)?
            .map_err(|e| Text::of(crate::state::message_error_text(e)))?;
        if signed.address != at.address {
            return Err(refused());
        }
        Ok(signed)
    }

    /// Costs an address derivation per address tried (through
    /// `WalletPolicy::find_address`), up to 202 for each loaded wallet: its
    /// receive and change addresses at indexes 0 to 100. Checks a signed
    /// message, and finds its address among them.
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
    pub keys: Vec<(usize, Text, bool)>,
    /// Its locks, in words.
    pub locks: Vec<Text>,
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
        .waits
        .paths
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
                        Text::fill(
                            EN.wallet_blocks_after,
                            &[&Text::num(u64::from(*n)), &Text::num(u64::from(*n) / 144)],
                        )
                    }
                    Lock::Intervals(n) => {
                        ok &= relative && in_time && value >= *n;
                        Text::fill(
                            EN.wallet_days_after_arrival,
                            &[&Text::num(u64::from(*n) * 512 / 86_400)],
                        )
                    }
                    Lock::Height(h) => {
                        Text::fill(EN.wallet_after_block, &[&Text::num(u64::from(*h))])
                    }
                    Lock::Time(t) => Text::fill(EN.wallet_after_time, &[&Text::num(u64::from(*t))]),
                    Lock::Preimage => Text::of(EN.wallet_a_secret_preimage),
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
) -> Vec<(usize, Text, bool, bool)> {
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
    pub signed: Option<Text>,
    /// The finished transaction.
    pub tx: Option<Text>,
    /// The part-signed PSBT.
    pub part: Option<Text>,
}

impl Faraday {
    /// After a press: the press that asked for a waiting wallet is noted
    /// for the hold to run again, where it [`replays`] (`asked` is
    /// whether one waited before it), and the wallet confirmation sheet
    /// opens.
    pub(crate) fn wallet_after(&mut self, asked: bool, action: Action) {
        if !asked && replays(action) {
            let inbox = self.inbox_print();
            self.session.wallet_replay(action, inbox);
        }
        self.wallet_sheet_open();
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

    /// Opens **Published words** when a key held on Mainnet has published
    /// words and no sheet is open. Whether it opened. A lookup over the keys.
    pub(crate) fn published_sheet_open(&mut self) -> bool {
        if self.sheet.is_none() && self.session.keys.published_pending().is_some() {
            self.sheet = Some(Sheet::Published);
            return true;
        }
        false
    }

    /// The published-words sheet's actions: **Remove** drops the key and
    /// what a spend was reading, **Keep for testing** stops the caution.
    pub(crate) fn published_act(&mut self, action: Action) {
        if let Some(fp) = self.session.keys.published_pending() {
            match action {
                Action::PublishedRemove => {
                    self.session.keys.remove_published(fp);
                    self.wallets.clear_spend();
                    let gone = self.session.drop_typed_with(fp);
                    if !gone.is_empty() {
                        // Wallets after the one taken out moved up: a key
                        // tick drawn before is dropped.
                        for (i, cs, id) in gone {
                            self.wallet_left(i, cs.as_str(), id);
                        }
                        let last = self.session.wallets.len().saturating_sub(1);
                        self.wallets.set_wallet(self.wallets.wallet().min(last));
                    }
                    self.restore_key_gone(fp);
                    crate::wallet_backup::kef::dropped(self);
                    self.backup_key_removed();
                    self.spend_backup_key_gone(fp);
                    self.create.drop_key(fp.0);
                    // "Key … added" said of the key just removed.
                    self.out.toast = None;
                }
                // A public sample seed has no keeping on Mainnet.
                _ if self.session.keys.is_sample(fp) => return,
                // A key written on an online computer is kept only by the
                // hold; a plain press of Keep is not that.
                Action::PublishedKeep if self.session.keys.made_online(fp) => return,
                _ => self.session.keys.keep_published(fp),
            }
        }
        self.sheet = None;
        self.refresh_spend();
    }

    /// Opens the wallet confirmation sheet over what is on screen while a
    /// wallet waits to be confirmed (F254). Whether it opened.
    pub(crate) fn wallet_sheet_open(&mut self) -> bool {
        if self.session.wallet_asked().is_some() && self.sheet != Some(Sheet::WalletConfirm) {
            self.session.wallet_cover(self.sheet);
            self.sheet = Some(Sheet::WalletConfirm);
            return true;
        }
        false
    }

    /// Costs what `add_policy` costs: handles the wallet confirmation
    /// sheet's actions (`WalletHold`, `WalletCancel`), then opens the sheet
    /// of the next wallet that waits.
    pub(crate) fn confirm_act(&mut self, action: Action) {
        self.sheet = None;
        match action {
            Action::WalletHold => self.confirm_held(),
            Action::WalletCancel => self.session.wallet_drop(),
            _ => {}
        }
        if self.wallet_sheet_open() {
            return;
        }
        // The sheet the confirmation sheet covered comes back, where what
        // it asked is still asked: the scanner while the camera is on, the
        // import's passphrase sheet while it is.
        let covered = self.session.wallet_uncover();
        if self.sheet.is_none() {
            self.sheet = match covered {
                Some(Sheet::Scan) if self.camera.scan().is_none() => None,
                Some(Sheet::Import) if self.import_field().is_none() => None,
                Some(Sheet::Pull) => None,
                other => other,
            };
        }
        // The import's passphrase step, by the way `Open` brings it up: the
        // page is Home and its field takes typing.
        if matches!(self.sheet, None | Some(Sheet::Import)) && self.import_field().is_some() {
            self.import_vault_step();
        }
    }

    /// Costs what `add_policy` and the replayed press cost: the hold, where
    /// the wallet waiting first is confirmed and added.
    fn confirm_held(&mut self) {
        let Some(ask) = self.session.wallet_confirm() else {
            return;
        };
        // Add to vault loads nothing: nothing below runs for it.
        if let Some(vault) = ask.vault {
            self.vault_add_held(&ask, vault);
            return;
        }
        // The press that asked runs again and goes on as it would
        // have, while Files holds what it held; a wallet that came
        // in any other way is added here, and the toast says so.
        let descriptor = same_wallet(&ask.policy);
        let ticks = ask.rows.ticks();
        let asked = ask.policy.clone();
        if let Some((press, inbox)) = ask.replay
            && inbox == self.inbox_print()
        {
            self.act(press);
            if let Some(i) = self
                .session
                .wallets
                .iter()
                .position(|w| same_wallet(&w.policy) == descriptor)
            {
                self.session.wallets[i].merge_key_ticks(&asked, &ticks);
                self.vault_keep_sheet_ticks(i, &asked, &ticks);
                return;
            }
        }
        let c = self.compute();
        match self
            .session
            .add_policy(&c, &ask.name, ask.policy, &ask.source, ask.origin)
        {
            Ok(i) if ask.restore && self.restore.state().is_some() => {
                self.session.wallets[i].merge_key_ticks(&asked, &ticks);
                self.vault_keep_sheet_ticks(i, &asked, &ticks);
                self.restore_added(i, ask.origin);
                if ask.origin == Origin::TypedSeeds {
                    // Made from the typed seeds, as Make would have.
                    self.restore.made_from_seeds();
                    self.refresh_spend();
                }
            }
            Ok(i) => {
                self.session.wallets[i].merge_key_ticks(&asked, &ticks);
                self.vault_keep_sheet_ticks(i, &asked, &ticks);
                self.wallets.set_wallet(i);
                self.refresh_spend();
                self.toast_added(i);
            }
            Err(e) => self.toast(&e.text()),
        }
    }

    /// Toasts that wallet `i` was added, and how many of its keys nothing
    /// vouches for.
    pub(crate) fn toast_added(&mut self, i: usize) {
        let w = &self.session.wallets[i];
        let text = match w.keys_unchecked() {
            0 => Text::fill(EN.wallet_confirm_added, &[&w.name]),
            1 => Text::fill(EN.keycheck_added_one, &[&w.name]),
            n => Text::fill(EN.keycheck_added_many, &[&w.name, &Text::num(n as u64)]),
        };
        self.toast(&text);
    }

    /// Toggles the person's tick of key `key` in the rows `of` names,
    /// where the tick is theirs to make.
    pub(crate) fn key_tick(&mut self, of: KeyBlockOf, key: u8) {
        let key = usize::from(key);
        match of {
            KeyBlockOf::Asked => self.session.ask_tick(key),
            KeyBlockOf::Register => self.wallets.register_tick(key),
            // A wallet taken out since the frame the press was drawn on
            // leaves the next one at its place: the press is dropped.
            KeyBlockOf::Wallet(_) if self.wallets.list_changed() => {}
            KeyBlockOf::Wallet(i) => {
                if self.session.tick_key(usize::from(i), key) {
                    self.vault_keep_tick(usize::from(i), key);
                }
            }
            // The same, and a Result whose wallet is gone ticks nothing.
            KeyBlockOf::Check if self.wallets.list_changed() => {}
            KeyBlockOf::Check => match self.tools.mscheck.of_wallet() {
                Some(_) => {
                    if let Some(i) = self.tools.mscheck.own_wallet(&self.session)
                        && self.session.tick_key(i, key)
                    {
                        self.vault_keep_tick(i, key);
                    }
                }
                None => self.tools.mscheck.tick(key),
            },
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
        let Some(s) = self.wallets.spend() else {
            return;
        };
        match action {
            Action::RegisterOpen if s.inspection.register.is_some() => {
                self.wallets.open_register();
                self.register_rows(0);
                self.sheet = Some(Sheet::Register);
            }
            Action::RegisterField => self.wallets.register_typing(),
            Action::RegisterReading(k) => {
                self.wallets.register_reading(k);
                self.register_rows(usize::from(k));
            }
            Action::RegisterCancel => {
                self.wallets.close_register();
                self.sheet = None;
            }
            Action::RegisterHold => {
                let (Some(ask), Some(offer)) = (&s.register, &s.inspection.register) else {
                    return;
                };
                let name = ask.name_or(&crate::register::name_of_file(&s.spend.source));
                let reading = ask.reading;
                let ticks = ask.rows.as_ref().map(Rows::ticks).unwrap_or_default();
                let (source, psbt, offer) =
                    (s.spend.source.clone(), s.spend.psbt.clone(), offer.clone());
                let c = self.compute();
                let keys = &self.session.keys;
                let yours =
                    |fp: Fingerprint, path: &faraday_bip::bitcoin::bip32::DerivationPath| {
                        keys.xpub_at(&c, keys.position(fp)?, path)
                    };
                let mut asked = None;
                let added = faraday_psbt::register(&c, &psbt, &offer, reading, &yours)
                    .map_err(|r| (Text::of(&crate::register::refusal_text(r, &offer)), false))
                    .and_then(|policy| {
                        // The hold, after every check, is the wallet's
                        // confirmation: no second sheet.
                        self.session.wallet_confirmed_by_register(&policy);
                        asked = Some(policy.clone());
                        self.session
                            .add_policy(&c, &name, policy, &source, Origin::Named)
                            .map_err(|e| (e.text(), e == Refusal::Unconfirmed))
                    });
                match added {
                    // Held for its own sheet, which opens next.
                    Err((_, true)) => {
                        self.wallets.close_register();
                        self.sheet = None;
                    }
                    Ok(i) => {
                        if let Some(asked) = &asked {
                            self.session.wallets[i].merge_key_ticks(asked, &ticks);
                            self.vault_keep_sheet_ticks(i, asked, &ticks);
                        }
                        self.wallets.close_register();
                        self.sheet = None;
                        self.wallets.set_wallet(i);
                        self.refresh_spend();
                        let text = match self.session.wallets[i].keys_unchecked() {
                            0 => Text::fill(EN.register_added, &[&name]),
                            1 => Text::fill(EN.keycheck_registered_one, &[&name]),
                            n => Text::fill(
                                EN.keycheck_registered_many,
                                &[&name, &Text::num(n as u64)],
                            ),
                        };
                        self.toast(&text);
                    }
                    Err((why, _)) => {
                        self.wallets.register_refused(why);
                    }
                }
            }
            _ => {}
        }
    }

    /// Costs a key derivation for each key of reading `k`'s wallet under
    /// a loaded fingerprint, and its checksum: the register sheet's rows,
    /// worked out at the press so drawing only reads.
    fn register_rows(&mut self, k: usize) {
        let descriptor = self
            .wallets
            .spend()
            .and_then(|s| s.inspection.register.as_ref())
            .and_then(|o| o.readings.get(k))
            .map(|r| r.descriptor.clone());
        let c = self.compute();
        let rows = descriptor.and_then(|d| Rows::of_descriptor(&self.session, &c, &d));
        self.wallets.register_set_rows(rows);
    }

    /// The lines that name a wallet under its code: its shape and
    /// network, and its keys' fingerprints.
    pub(crate) fn wallet_label(&self, w: &wallet::Wallet) -> Vec<Text> {
        let net = self.session.network();
        let shape = if net.is_mainnet() {
            Session::shape(w)
        } else {
            text!("{} · {}", &*Session::shape(w), net.name())
        };
        let mut lines = vec![shape];
        let all: Vec<Text> = w
            .policy()
            .keys()
            .iter()
            .filter_map(|k| k.fingerprint().map(fp_text))
            .collect();
        if !all.is_empty() {
            let word = if all.len() == 1 {
                EN.wallet_label_key
            } else {
                EN.wallet_label_keys
            };
            let mut line = text!("{word} ");
            for (n, fp) in all.iter().enumerate() {
                if n > 0 {
                    line.push_str(" · ");
                }
                line.push_str(fp);
            }
            lines.push(line);
        }
        lines
    }

    /// Takes wallet `i` out of the list, the one place a wallet leaves it:
    /// a key tick drawn on the list before is dropped
    /// ([`WalletsArea::list_changed`]).
    pub(crate) fn remove_wallet(&mut self, i: usize) -> wallet::Wallet {
        let w = self.session.wallets.remove(i);
        self.wallet_left(i, w.checksum(), w.id());
        w
    }

    /// Wallet `gone` was taken out of the session list (every removal ends
    /// here: [`Self::remove_wallet`], [`Session::drop_forged`] and
    /// [`Session::drop_typed_with`]): every state that keeps a wallet by its
    /// place moves it up one when it was after the one gone, and a flow on
    /// the one gone ends (Back up, Restore) or forgets it (the pages that
    /// hold one only once there is one).
    pub(crate) fn wallet_left(&mut self, gone: usize, checksum: &str, id: WalletId) {
        self.wallets.set_list_changed();
        // First: restore finds the flow by its place, before any flow is
        // dropped below.
        let create_back = self
            .create
            .wallet_left(gone)
            .map(|taken| taken.before.and_then(|was| self.flows.restore(*was)));
        if self.backup.wallet_left(gone) {
            // The flow of the wallet gone, by its id: the place
            // now holds the wallet that moved into it.
            let flow = self.wallet_flow_at(checksum, Some(id));
            crate::wallet_backup::kef::dropped(self);
            self.backup.close();
            self.flow_drop_unmade(flow);
            if self.screen == Screen::Backup {
                self.screen = Screen::Wallets;
            }
        } else if let Some(f) = self.wallet_flow_active(checksum, Some(id)) {
            // Another wallet's Back up is the one open: the unmade flow of
            // the wallet gone still ends, its row with it.
            self.flow_drop_unmade(Some(f));
        }
        if self.restore.wallet_left(gone)
            && let Some(r) = self.restore.take()
        {
            self.flow_drop_unmade(self.wallet_flow_of_owner(r.owner));
            if self.screen == Screen::Restore {
                self.screen = r.back_to();
            }
        }
        // After the flows above ended: no flow of Create's owner left, the
        // snapshot is its flow again where that overwrites nothing, else
        // the vault choice the keys came from is not kept.
        if let Some(unused) = create_back
            && let Some(owner) = self.create.owner()
            && self.wallet_flow_of_owner(owner).is_none()
        {
            match unused {
                Some(was) if !self.flows.has_owner(owner) => self.flows.put_back(was),
                _ => self.create.clear_vaults(),
            }
        }
        // The flow's own screen, once the flow it showed is gone.
        if self.screen == Screen::BackupFlow && self.flow().is_none() {
            self.screen = Screen::Wallets;
        }
        // The vault form opened from the flow went with it.
        if self.screen == Screen::CreateVault && self.vaults.create.is_none() {
            self.screen = Screen::Wallets;
        }
        self.entry.wallet_left(gone);
        self.message.wallet_left(gone);
        if let Some(back) = self.silent.wallet_left(gone) {
            // The page of the wallet gone closes: it does not turn into its
            // key's own page.
            if self.screen == Screen::Silent {
                self.screen = back;
            }
        }
        if let Some(mut form) = self.tools.kef_tool.wallet_left(gone) {
            form.close(self);
        }
        crate::seeds::kef::kef_wallet_left(self, gone);
        self.trail.wallet_left(gone);
        self.in_progress.wallet_left(gone);
        self.explore.wallet_left(gone);
        self.tools.address_check.wallet_left(gone);
        self.import.wallet_left(gone);
        self.vaults.wallet_left(gone);
        self.seed_read_wallet_left(gone);
        self.pass_seeds_wallet_left(gone);
        if self.camera.wallet_left(gone) && self.camera.take().is_some() {
            self.out
                .commands
                .push_back(faraday_shell_api::Command::CameraOff);
            if self.sheet == Some(Sheet::Scan) {
                self.sheet = None;
            }
        }
        // A press held for a device may name a place of the list: it is
        // not done.
        if self.devices.after.is_some() {
            let over_import = self.devices.over_import;
            self.devices.cancel();
            if self.sheet == Some(Sheet::Devices) {
                self.sheet = None;
                // A device held back under the sheet shows at once, as
                // Cancel does.
                self.input_sheet();
                // The boot import under it comes back, as its Cancel does:
                // at once, or when the held-back device's sheet closes.
                if over_import && self.import_field().is_some() {
                    if self.sheet.is_none() {
                        self.import_vault_step();
                    } else {
                        self.import.set_under_input(true);
                    }
                }
            } else {
                // Another sheet is on show: the import under the Devices
                // sheet is still owed its return.
                self.devices.over_import = over_import;
            }
        }
        // The made wallet's dialog shows nothing once no wallet is made.
        if self.sheet == Some(Sheet::MadeWallet) && self.made_place().is_none() {
            self.sheet = None;
        }
    }

    /// Takes out the wallets a key loaded by this press shows to hold
    /// another seed's key under its fingerprint, and toasts the first one
    /// ([`Session::drop_forged`]). A lookup unless a key was loaded or what
    /// the held keys read changed.
    pub(crate) fn drop_forged_wallets(&mut self) {
        let c = self.compute();
        self.sync_waiting_if_changed();
        let dropped = self.session.drop_forged(&c);
        self.sync_waiting_if_changed();
        self.sync_psbt_wallets_if_changed();
        for (i, cs, id, ..) in &dropped {
            self.wallet_left(*i, cs.as_str(), *id);
        }
        if let Some((_, _, _, name, why)) = dropped.first() {
            let text = Text::fill(EN.vaults_refused, &[name, why]);
            self.wallets.set_wallet(0);
            self.spend_after_wallet_removed();
            // Sign a message's kept address is of a place that has moved;
            // after the loop, when every place is where it ends.
            self.message_address(&c);
            self.toast(&text);
        }
    }

    /// Costs what `refresh_spend` costs. Keeps the open spend in step after
    /// a wallet was removed: it is closed when the wallet it was from is
    /// gone.
    pub(crate) fn spend_after_wallet_removed(&mut self) {
        let had = self.wallets.spend().is_some_and(|s| s.wallet.is_some());
        self.refresh_spend();
        // The wallet an open spend was from is gone (the wallets after it
        // moved up one place): the spend closes, not another wallet's.
        if had && self.wallets.spend().is_some_and(|s| s.wallet.is_none()) {
            self.wallets.clear_spend();
        }
    }

    /// Costs an inspection and the nonce check when the PSBT or the session changed, and
    /// what `aggregate_musig`, `signers_by` and `wallet_for` cost. Keeps the open spend in
    /// step with the session: its inspection, signers, wallet and steps.
    pub(crate) fn refresh_spend(&mut self) {
        let c = self.compute();
        let memory = &self.memory;
        self.wallets
            .refresh(&c, &self.session, |id| memory.is_checked(id));
        self.refresh_others(&c);
        self.nonce_spend_sync();
        self.spend_files_out();
    }

    /// Costs what `check_others::make` costs (an inspection of each
    /// other transaction's file), only when the spend was inspected again
    /// or the files that count are not the ones the results were made
    /// from (`docs/COMPARE.md` §4.3); a lookup over Files otherwise.
    pub(crate) fn refresh_others(&mut self, c: &Compute) {
        let Some(s) = self.wallets.spend() else {
            return;
        };
        let mut spent: Vec<_> = s
            .spend
            .psbt
            .unsigned_tx()
            .input
            .iter()
            .map(|i| i.previous_output)
            .collect();
        spent.sort();
        let now: Vec<_> = crate::decode::check_others::counting(
            self.inbox().iter().chain(self.outbox.iter()),
            s.spend.txid,
            &spent,
        )
        .into_iter()
        .filter_map(|(i, t)| {
            Some((
                Text::of(&i.name),
                t,
                crate::decode::check_others::place_of(self, i)?,
            ))
        })
        .collect();
        if !s.others_stale && s.others_from.as_ref() == Some(&now) {
            return;
        }
        let made = if now.is_empty() {
            Vec::new()
        } else {
            crate::decode::check_others::make(
                c,
                self,
                &s.spend.psbt,
                s.spend.v2.is_some(),
                &s.inspection,
            )
        };
        self.wallets.keep_others(made, now);
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
        let Some(s) = self.wallets.spend() else {
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
        let (taken_back, lifted) = self.wallets.apply_nonce_hold(held);
        if !taken_back.is_empty()
            && self
                .outbox
                .iter()
                .any(|i| taken_back.iter().any(|n| i.name == n.as_str()))
        {
            self.outbox
                .retain(|i| !taken_back.iter().any(|n| i.name == n.as_str()));
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
        self.wallets.register_key(key)
    }

    /// Costs what `add_wallet` costs. Loads the wallet an Inbox file
    /// describes, named after the file.
    pub(crate) fn load_wallet(&mut self, index: usize) {
        let c = self.compute();
        let Some(item) = self.inbox.get(index) else {
            return;
        };
        let text = Text::from_utf8_lossy(&item.bytes);
        let source = Text::of(&item.name);
        let name = Text::of(&crate::register::name_of_file(&item.name));
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
            // Held for its sheet: its hold runs this again.
            Err(Refusal::Unconfirmed) => {}
            Err(e) => self.toast(&e.text()),
        }
    }

    /// The wallet chosen on the Wallets tab.
    #[cfg(feature = "testkit")]
    pub fn wallet_index(&self) -> usize {
        self.wallets.wallet()
    }

    /// The key in no wallet chosen on the Wallets tab, by fingerprint.
    pub fn loose_pick(&self) -> Option<[u8; 4]> {
        self.wallets.loose_pick()
    }

    /// The chosen wallet's new name, while it is typed.
    pub fn renaming(&self) -> Option<&Text> {
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
                    self.remove_wallet(i);
                    self.wallets.set_wallet(0);
                    self.spend_after_wallet_removed();
                }
            }
            Action::RemoveKeys | Action::RemoveKeysHold => {
                if action == Action::RemoveKeys {
                    let c = self.compute();
                    if self.keys_kept_nowhere(&c) {
                        self.sheet = Some(crate::Sheet::RemoveKeys);
                        return;
                    }
                }
                if self.sheet == Some(crate::Sheet::RemoveKeys) {
                    self.sheet = None;
                }
                let gone = self.session.keys.remove_all();
                self.wallets.clear_spend();
                // After the removal: the tab puts back a plain seed that
                // is still held.
                for fp in gone {
                    self.restore_key_gone(fp);
                    self.spend_backup_forget_key(fp);
                    self.create.key_removed_with_all(fp.0);
                }
                crate::wallet_backup::kef::dropped(self);
                self.backup_key_removed();
                if self.spend_backup.state().open.is_some() {
                    self.spend_backup_settle_cards();
                }
            }
            Action::StartSpend(i) => {
                // A spend from anywhere but the stick's "Import and sign"
                // has no sequence of its own (that read sets it after).
                self.visit.set_sign_stick(None);
                self.visit.set_sign_from(None);
                crate::signing::read::start_spend(self, i, crate::signing::read::Then::Spend);
            }
            Action::Step(n) => {
                // On Spend from a backup and the Spend tab a step is
                // always open: a press on the open one does nothing.
                let keep_open = matches!(self.screen, Screen::SpendBackup | Screen::Spend);
                self.wallets.press_step(n, keep_open);
                if self.screen == Screen::SpendBackup {
                    self.spend_backup_step_moved(false);
                }
            }
            Action::ReviewScroll(by) => {
                self.wallets.scroll_spend_by(f32::from(by));
            }
            Action::ReviewHold => {
                let inputs = self.session.inspect_inputs();
                if self.wallets.hold_review(&inputs) {
                    self.wallet_act(Action::StepNext(wallet::step::TRANSACTION));
                }
            }
            Action::StepNext(n) => {
                if !self.wallets.continue_step(n) {
                    return;
                }
                // Check's own Continue: this wallet is checked this
                // power-on (`docs/WALLETS.md` §4), kept across a lock.
                if n == wallet::step::CHECK
                    && let Some(w) = self.wallets.spend().and_then(|s| s.wallet)
                    && let Some(wallet) = self.session.wallets.get(w)
                {
                    let id = wallet.id();
                    self.mark_checked(id);
                }
                // The steps with nothing to decide are passed (never the
                // review): Continue on one carries on past them.
                let c = self.compute();
                let mut one_wallet = None;
                while let Some(next) = self.wallets.spend().and_then(|s| s.open)
                    && crate::signing::nothing_to_decide(self, &c, next, &mut one_wallet)
                    && self.wallets.continue_step(next)
                {}
                // A complete PSBT waits for this: its files go out once
                // Review (and Check) is gone through.
                self.spend_files_out();
                if self.screen == Screen::SpendBackup {
                    self.spend_backup_step_moved(true);
                }
            }
            Action::ToggleTable => {
                self.wallets.toggle_table();
            }
            Action::SignHere => crate::signing::sign_here(self),
            Action::Collect(i) => crate::signing::collect::start_collect(self, i),
            Action::SignedToOutbox => {
                if let Some(file) = self.wallets.take_signed_file() {
                    let Ok((name, bytes)) = file else {
                        return self.toast(EN.spend_v2_not_written);
                    };
                    let name = self.put_outbox_beside(&name, bytes);
                    self.wallets.set_signed_name(Text::of(&name));
                }
            }
            Action::TxToOutbox => {
                if let Some((name, bytes)) = self.wallets.take_tx_file() {
                    let name = self.put_outbox_beside(&name, bytes);
                    self.wallets.set_tx_name(Text::of(&name));
                }
            }
            Action::Primary => self.primary(),
            Action::LoadWallet(i) => self.load_wallet(i),
            Action::LoadKey(i) => {
                if self.load_key(i) {
                    self.inbox_load_named();
                }
            }
            Action::PublicOut(i, what) => {
                if let Some(kind) = crate::public_kind::PublicFile::from_wire(what) {
                    self.public_out_job(i, kind);
                }
            }
            Action::PublicMore(i) => {
                self.backup.public_more(i);
            }
            Action::PublicRemove(i, what) => {
                if let Some(kind) = crate::public_kind::PublicFile::from_wire(what) {
                    self.public_remove(i, kind);
                }
            }
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
                    self.toast(&Text::fill(EN.act_plug_in_the_visit, &[self.medium.a()]));
                } else {
                    self.act(Action::Nav(Screen::Files));
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

    /// Finish makes both results For the stick the moment the transaction is
    /// finished: the signed PSBT and the finished transaction. One the person
    /// removed is not made again unless they ask; neither is made where a
    /// version 2 PSBT cannot take the signatures back (`Spend::out_bytes`).
    fn spend_files_out(&mut self) {
        let Ok(made) = self.wallets.take_result_files() else {
            return self.toast(EN.spend_v2_not_written);
        };
        for (signed, name, bytes) in made {
            let name = self.put_outbox_beside(&name, bytes);
            if signed {
                self.wallets.set_signed_name(faraday_text::Text::of(&name));
            } else {
                self.wallets.set_tx_name(faraday_text::Text::of(&name));
            }
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

impl Faraday {
    /// Costs what `account_text` costs. The account key in slot `slot` of
    /// wallet `i`, when this session holds its seed: for the cosigners of a
    /// wallet of more than one key. `None` for a silent payments wallet,
    /// whose slot is not an account key.
    pub fn wallet_key(
        &self,
        c: &Compute,
        i: usize,
        slot: u8,
    ) -> Result<WalletKey, faraday_text::Text> {
        let w = self
            .session
            .wallets
            .get(i)
            .ok_or_else(|| faraday_text::Text::of(EN.wallet_key_that_wallet))?;
        if w.policy().silent().is_some() {
            return Err(faraday_text::Text::of(EN.wallet_key_this_wallet_s));
        }
        let pk = w
            .policy()
            .keys()
            .get(usize::from(slot))
            .ok_or_else(|| faraday_text::Text::of(EN.wallet_key_this_wallet))?;
        let fp = pk
            .fingerprint()
            .ok_or_else(|| faraday_text::Text::of(EN.wallet_key_that_key_names))?;
        let key = self
            .session
            .keys
            .position(fp)
            .ok_or_else(|| faraday_text::Text::of(EN.wallet_key_that_key))?;
        let kind = create::NewKind::of(w.policy());
        let bip87 = create::bip87_key(pk);
        let standard = kind.filter(|&k| create::at_own_account(c, &self.session.keys, key, k, pk));
        Ok(WalletKey {
            fp: fp_text(fp),
            text: faraday_text::Text::of(&pk.key_text()),
            kind: kind.map_or_else(|| Session::shape(w), |k| faraday_text::Text::of(k.name())),
            standard,
            bip87,
            key,
        })
    }

    /// The slots of wallet `i` whose seeds are held here, one a key, as
    /// its public files list them: none for a wallet of one key.
    pub(crate) fn wallet_keys_here(&self, i: usize) -> Vec<u8> {
        let Some(w) = self.session.wallets.get(i) else {
            return Vec::new();
        };
        let mut seen = Vec::new();
        let mut out = Vec::new();
        if self.session.slots(w).len() < 2 {
            return out;
        }
        for slot in 0..w.policy().keys().len().min(usize::from(u8::MAX)) {
            let slot = slot as u8;
            if let Some(fp) = self.wallet_key_here(i, slot)
                && !seen.contains(&fp)
            {
                seen.push(fp);
                out.push(slot);
            }
        }
        out
    }

    /// Costs what `wallet_key` and `bsms_record` cost. A wallet key's BIP
    /// 129 key record, signed by the key: for a `wsh` or `sh(wsh)` multisig
    /// key at Create's own account.
    pub(crate) fn wallet_key_record(
        &self,
        c: &Compute,
        i: usize,
        slot: u8,
    ) -> Result<(faraday_text::Text, faraday_text::Text), faraday_text::Text> {
        let k = self.wallet_key(c, i, slot)?;
        k.standard
            .and_then(|kind| {
                let bip87 = kind.has_bip87() && k.bip87;
                kind.bsms_record(c, &self.session.keys, k.key, 0, bip87, &k.fp)
            })
            .map(|r| (k.fp.clone(), faraday_text::Text::of(r.as_str())))
            .ok_or_else(|| faraday_text::Text::of(EN.wallet_key_record_bip))
    }
}

// The session's keys, and its wallets.
impl crate::at_lock::AtLock for Session {
    fn holds(&self) -> crate::at_lock::Holds {
        crate::at_lock::Holds {
            kept: self.keys.holds_secret(),
            typed: false,
            session: !self.wallets.is_empty() || !self.silent_handed.is_empty(),
        }
    }
    fn lock(&mut self) {
        self.wipe();
    }
}

#[cfg(test)]
mod partial_tests;

impl Faraday {
    /// The master fingerprint of slot `slot` of wallet `i` when
    /// [`Faraday::wallet_key`] has that key: its seed is here and the
    /// wallet is no silent payments wallet. A lookup; it
    /// derives nothing.
    fn wallet_key_here(&self, i: usize, slot: u8) -> Option<faraday_bip::keys::Fingerprint> {
        let w = self.session.wallets.get(i)?;
        if w.policy().silent().is_some() {
            return None;
        }
        let fp = w.policy().keys().get(usize::from(slot))?.fingerprint()?;
        self.session
            .keys
            .all()
            .any(|k| k.fingerprint == fp)
            .then_some(fp)
    }

    fn wallet_key_out(&mut self, i: usize, slot: u8) {
        let c = self.compute();
        match self.wallet_key(&c, i, slot) {
            Ok(k) => {
                let name = faraday_text::text!("xpub-{}.txt", &*k.fp);
                let bytes =
                    faraday_text::text!("# Account xpub {}, {}\n{}\n", &*k.fp, &*k.kind, &*k.text);
                let name = self.put_outbox_beside(&name, bytes.as_bytes().to_vec());
                self.backup_xpub_sent(i, slot, false, &name);
                self.toast_out(&name);
            }
            Err(e) => self.toast(&e),
        }
    }

    fn wallet_key_bsms(&mut self, i: usize, slot: u8) {
        let c = self.compute();
        match self.wallet_key_record(&c, i, slot) {
            Ok((fp, text)) => {
                let name = faraday_text::text!("xpub-{}-bsms.txt", &*fp);
                let name = self.put_outbox_beside(&name, text.as_bytes().to_vec());
                self.backup_xpub_sent(i, slot, true, &name);
                self.toast_out(&name);
            }
            Err(e) => self.toast(&e),
        }
    }

    /// Costs an address derivation (through `address_at`), or `checked_config`.
    /// A wallet's public text file `what` (as [`Faraday::public_out`] names
    /// them): the multisig config where it rebuilds the wallet, BIP 129's.
    pub(crate) fn wallet_file_text(
        &self,
        c: &Compute,
        wallet: usize,
        what: crate::public_kind::PublicFile,
    ) -> Option<faraday_text::Text> {
        let w = self.session.wallets.get(wallet)?;
        public_files::file_text(c, self.session.network(), w, what)
            .map(|t| faraday_text::Text::of(&t))
    }

    /// The keys loaded that no wallet uses, and the one of them the
    /// Wallets card offers a wallet from: the one picked, else the first.
    pub(crate) fn loose_keys(&self) -> (Vec<faraday_bip::keys::Fingerprint>, Option<[u8; 4]>) {
        let loose = self.session.loose_keys();
        let picked = self
            .wallets
            .loose_pick()
            .filter(|p| loose.iter().any(|f| f.0 == *p))
            .or_else(|| loose.first().map(|f| f.0));
        (loose, picked)
    }
}
