//! The Wallets tab's session: keys and wallets held in memory, and the spend of
//! one transaction from loading it to the two results (`docs/WALLETS.md`
//! §2 and §4). Every piece of cryptography is `osk-bip`'s or
//! `osk-psbt`'s; this module only decides which call comes next.

use crate::strings::EN;
use opensigner_core::strings::fill;
use osk_bip::bip39::{Language, Mnemonic};
use osk_bip::bitcoin::consensus::encode::serialize_hex;
use osk_bip::bitcoin::{Transaction, Txid};
use osk_bip::keys::{Fingerprint, MasterKey, Network, ScriptType};
use osk_bip::policy::{Template, WalletPolicy, Wrapper};
use osk_psbt::verify::Verdict;
use osk_psbt::{Aux, Context, Inspection, KeyRef, Nonce, Psbt};

/// The network a session starts on, before a wallet or a PSBT says
/// otherwise.
pub const START_NETWORK: Network = Network::Mainnet;

// The description reader (`faraday-intake`),
// at the paths this crate has always had them.
pub use faraday_intake::wallet::{Refusal, network_name, network_of_key, policy_network};

/// What tells one seed's words from another's, from whichever list they
/// are read: the SHA-256 of their entropy, all 32 bytes, so that two
/// seeds never pass for one (F65: what cannot be told apart is not
/// counted as kept). A hash of secret entropy: kept only beside what it
/// is of (a key, an open vault's records), wiped when dropped, never
/// written anywhere.
pub type WordsId = zeroize::Zeroizing<[u8; 32]>;

/// What tells one master key from another in all 32 bytes, the
/// passphrase applied: the SHA-256 of its public key and chain code, the
/// same on every network. Public (a hash of public data); what "this
/// vault holds this key" compares, where the 4-byte fingerprint only
/// finds a candidate (F65). Costs an elliptic-curve multiplication.
pub fn master_id(m: &MasterKey) -> [u8; 32] {
    let x = m.xpub();
    let mut bytes = x.public_key.serialize().to_vec();
    bytes.extend_from_slice(x.chain_code.as_bytes());
    crate::sha256_of(&bytes)
}

/// [`WordsId`] of `m`'s words. Costs a hash; the words are parsed already.
pub fn words_id(m: &Mnemonic) -> WordsId {
    let entropy = m.entropy();
    zeroize::Zeroizing::new(crate::sha256_of(entropy.expose().as_bytes()))
}

/// Reads a wallet description: `faraday_intake`'s reader, counted
/// ([`crate::work`]). Costs a parse of the description and its keys.
pub fn read_wallet(text: &str) -> Result<WalletPolicy, Refusal> {
    crate::work::count(crate::work::Work::DescriptorParse);
    faraday_intake::wallet::read_wallet(text)
}

/// Parses seed words in `lang`: BIP-39's parser, counted
/// ([`crate::work`]). Costs a word-list search per word and a hash.
pub fn parse_words(lang: Language, text: &str) -> Result<Mnemonic, osk_bip::bip39::Error> {
    crate::work::count(crate::work::Work::WordsParse);
    Mnemonic::parse(lang, text)
}

/// A vault key record's seed words, when it holds words: the vault's
/// reader, counted ([`crate::work`]). Costs a parse of the words.
pub fn record_words(r: &faraday_vault::Record) -> Option<Mnemonic> {
    crate::work::count(crate::work::Work::WordsParse);
    faraday_vault::records::words_of(r)
}
pub(crate) use faraday_intake::wallet::{json_label, network_of_kind};

/// A key that can sign, held for this session only.
pub struct Key {
    /// The master key.
    master: MasterKey,
    /// [`master_id`] of the master key, worked out once when the key is
    /// made; the same on every network.
    master_id: [u8; 32],
    /// What the person called it.
    pub label: String,
    /// The words it was loaded from, for the backup's copy-by-hand step.
    /// Wiped with the session.
    pub words: Option<zeroize::Zeroizing<String>>,
    /// The BIP-39 list those words are from.
    pub language: Language,
    /// The public share these words are as a FROST share: 24 words'
    /// 32 bytes read as the share's scalar (`docs/PLANNING.md` §16.103).
    /// Every 24-word key has one; it signs as a share only for a loaded
    /// threshold record that lists it.
    pub share: Option<osk_bip::bitcoin::secp256k1::PublicKey>,
    /// The BIP-39 passphrase it was loaded with, when it has one. Wiped
    /// with the session.
    pub passphrase: Option<zeroize::Zeroizing<String>>,
    /// Its public key texts asked for, each derived once
    /// ([`Key::account_text`], [`Key::text_at`]), by what was asked:
    /// public, and dropped when the master key moves network.
    public: std::cell::RefCell<Vec<(PublicAsk, Result<String, String>)>>,
    /// Its words' [`WordsId`] ([`Key::words_id`]), once worked out:
    /// wiped with the key.
    words_id: std::cell::OnceCell<Option<WordsId>>,
}

/// A public key text a [`Key`] keeps: a kind's account, or a path.
#[derive(Debug, Clone, PartialEq, Eq)]
enum PublicAsk {
    Account(crate::create::NewKind, u32),
    Path(String),
}

impl Key {
    /// Makes a key; costs an elliptic-curve multiplication with the master
    /// private key, for its [`master_id`].
    pub fn new(
        master: MasterKey,
        label: String,
        words: Option<zeroize::Zeroizing<String>>,
        language: Language,
        share: Option<osk_bip::bitcoin::secp256k1::PublicKey>,
        passphrase: Option<zeroize::Zeroizing<String>>,
    ) -> Key {
        crate::work::count(crate::work::Work::MasterKey);
        let master_id = master_id(&master);
        Key {
            master,
            master_id,
            label,
            words,
            language,
            share,
            passphrase,
            public: std::cell::RefCell::new(Vec::new()),
            words_id: std::cell::OnceCell::new(),
        }
    }

    /// What tells its words apart from other words, whatever list they
    /// were typed from and whatever passphrase is added ([`WordsId`]):
    /// worked out the first time it is asked, by whoever asks, a frame
    /// included, and kept on the key until it is dropped. `None` for a key
    /// with no words. Costs a parse of the words the first time.
    pub fn words_id(&self) -> Option<&[u8; 32]> {
        let words = self.words.as_ref()?;
        self.words_id
            .get_or_init(|| {
                crate::work::keeping(|| Some(words_id(&parse_words(self.language, words).ok()?)))
            })
            .as_deref()
    }

    /// `[fingerprint/path]xpub` at `kind`'s path with account `account`
    /// ([`crate::create::NewKind::key_text_account`]): derived the first
    /// time it is asked, by whoever asks, a frame included, and kept on
    /// the key, a lookup after. Costs a key derivation the first time.
    pub fn account_text(
        &self,
        kind: crate::create::NewKind,
        account: u32,
    ) -> Result<String, String> {
        self.public_text(PublicAsk::Account(kind, account), || {
            kind.key_text_account(&self.master, account)
        })
    }

    /// `[fingerprint/path]xpub` at `path` ([`crate::create::key_text_at`]),
    /// kept as [`Key::account_text`] is. Costs a key derivation the first
    /// time.
    pub fn text_at(&self, path: &str) -> Result<String, String> {
        self.public_text(PublicAsk::Path(path.to_string()), || {
            crate::create::key_text_at(&self.master, path)
        })
    }

    fn public_text(
        &self,
        ask: PublicAsk,
        derive: impl FnOnce() -> Result<String, String>,
    ) -> Result<String, String> {
        if let Some((_, t)) = self.public.borrow().iter().find(|(a, _)| *a == ask) {
            return t.clone();
        }
        let t = crate::work::keeping(derive);
        let mut kept = self.public.borrow_mut();
        if kept.len() >= 64 {
            kept.clear();
        }
        kept.push((ask, t.clone()));
        t
    }

    /// The master key.
    pub fn master(&self) -> &MasterKey {
        &self.master
    }

    /// [`master_id`] of its master key; a lookup, kept when the key was
    /// made.
    pub fn master_id(&self) -> [u8; 32] {
        self.master_id
    }

    /// The master key's fingerprint; a lookup, kept by the master key.
    pub fn fingerprint(&self) -> Fingerprint {
        self.master.fingerprint()
    }

    /// Moves the master key to `network`; the fingerprint is the same on
    /// every network.
    pub fn set_network(&mut self, network: Network) {
        self.master = self.master.for_network(network);
        self.public.borrow_mut().clear();
    }

    /// Whether the key's words are the ones with these BIP-39 `indices`;
    /// a lookup per word, with no parsing and no copy of the words.
    pub fn words_are(&self, indices: &[u16]) -> bool {
        let Some(words) = self.words.as_ref() else {
            return false;
        };
        let mut have = words.as_str().split(' ');
        for &i in indices {
            if have.next() != Some(self.language.word(i)) {
                return false;
            }
        }
        have.next().is_none()
    }

    /// Runs `take` on the secret share these words are, rebuilt for the
    /// call and wiped after it.
    pub fn with_secret_share(&self, take: &mut dyn FnMut(&osk_bip::frost::SecShare)) {
        if let Some(share) = self
            .words
            .as_ref()
            .and_then(|w| crate::wallet::parse_words(self.language, w).ok())
            .and_then(|m| secret_share(&m))
        {
            take(&share);
        }
    }
}

/// The FROST share 24 words are, when they are 24 words.
fn secret_share(m: &Mnemonic) -> Option<osk_bip::frost::SecShare> {
    if m.word_count() != 24 {
        return None;
    }
    let entropy = m.entropy();
    let mut bytes = [0u8; 32];
    bytes.copy_from_slice(entropy.expose().as_bytes());
    let share = osk_bip::frost::SecShare::from_bytes(&bytes).ok();
    zeroize::Zeroize::zeroize(&mut bytes);
    share
}

/// What one signing pass did.
pub struct Signed {
    /// The fingerprints that signed: keys, and shares by their own
    /// fingerprints.
    pub by: Vec<Fingerprint>,
    /// The carry section a threshold pass leaves for the shares still to
    /// sign: the secret nonce that travels on the stick.
    pub carry: Option<osk_psbt::threshold::CarrySection>,
}

/// A wallet described by its public keys.
pub struct Wallet {
    /// What the person called it.
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
    scripts: std::cell::RefCell<Vec<(bool, u32, Option<osk_bip::bitcoin::ScriptBuf>)>>,
}

impl Wallet {
    /// Makes a wallet.
    pub fn new(name: String, policy: WalletPolicy, source: String, origin: Origin) -> Wallet {
        Wallet {
            name,
            policy,
            source,
            origin,
            addresses: std::cell::RefCell::new(Vec::new()),
            scripts: std::cell::RefCell::new(Vec::new()),
        }
    }

    /// Its script at `change`/`index`, `None` when it has none there:
    /// derived the first time it is asked and kept on the wallet, a
    /// lookup after, as [`Wallet::address_on`] keeps addresses. Costs a
    /// script derivation the first time.
    pub fn script_at(&self, change: bool, index: u32) -> Option<osk_bip::bitcoin::ScriptBuf> {
        if let Some((.., s)) = self
            .scripts
            .borrow()
            .iter()
            .find(|(c, i, _)| *c == change && *i == index)
        {
            return s.clone();
        }
        let s = crate::work::keeping(|| {
            crate::work::count(crate::work::Work::WalletMatch);
            self.policy.script_at(change, index).ok()
        });
        let mut kept = self.scripts.borrow_mut();
        // A long walk through places keeps the latest only.
        if kept.len() >= 256 {
            kept.clear();
        }
        kept.push((change, index, s.clone()));
        s
    }

    /// Its address on `network` at `change`/`index`, or why there is
    /// none: derived the first time it is asked and kept on the wallet, a
    /// lookup after (`docs/SPEED.md` §2: a kept value is public and goes
    /// with what it describes). A frame that shows an address asks it
    /// again every frame; it is derived once. Costs an address derivation
    /// the first time.
    pub fn address_on(&self, network: Network, change: bool, index: u32) -> String {
        if let Some((.., a)) = self
            .addresses
            .borrow()
            .iter()
            .find(|(n, c, i, _)| *n == network && *c == change && *i == index)
        {
            return a.clone();
        }
        let a = crate::work::keeping(|| {
            crate::work::count(crate::work::Work::Address);
            self.policy
                .address_at(network, change, index)
                .map(|a| a.to_string())
                .unwrap_or_else(|e| fill(EN.wallet_no_address, &[&e.to_string()]))
        });
        let mut kept = self.addresses.borrow_mut();
        // A long walk through addresses keeps the latest only.
        if kept.len() >= 256 {
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
    pub reused: Vec<osk_psbt::verify::NonceReuse>,
}

impl NonceCheck {
    /// Whether nothing is wrong: every signature checked is valid, every
    /// one made here was recomputed, and no nonce repeats.
    pub fn sound(&self) -> bool {
        self.reused.is_empty()
            && self
                .rows
                .iter()
                .all(|r| !r.verdict.is_invalid() && !matches!(r.recomputed, Some(None)))
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
    pub recomputed: Option<Option<osk_psbt::verify::NonceMode>>,
}

/// One key slot of a wallet, in the descriptor's order.
pub struct Slot {
    /// The key's master fingerprint, when the descriptor gives one.
    pub fingerprint: Option<Fingerprint>,
    /// The label of the session key that fills it, when one does.
    pub held_by: Option<String>,
}

/// The fingerprint as eight hex digits.
pub fn fp_text(fp: Fingerprint) -> String {
    String::from_utf8_lossy(&fp.to_hex()).into_owned()
}

/// A wallet's source as a tag: "Vault" reads "From Vault" (`cap`) or
/// "from Vault" (inline, after another phrase), but a source whose
/// [`Origin`] already reads as a complete statement ("Made here",
/// "Dealt here") reads on its own, with no "From"/"from" before it
/// either way.
pub fn source_tag(source: &str, origin: Origin, cap: bool) -> String {
    if origin == Origin::Statement {
        source.to_string()
    } else if cap {
        fill(EN.wallet_from, &[source])
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
    /// Keys that can sign.
    pub keys: Vec<Key>,
    /// Wallets.
    pub wallets: Vec<Wallet>,
    /// The network every key derives for and every address is shown on.
    network: Network,
    /// The demo's public seeds and their fingerprints, when the shell
    /// set it ([`Session::set_demo`], F20). Empty, as on the stick: the
    /// session takes any key.
    demo: Vec<(&'static str, Fingerprint)>,
    /// Set by a shell that must stay on the network it started on
    /// ([`Session::fix_network`]): every other one is refused.
    network_fixed: bool,
}

impl Default for Session {
    fn default() -> Self {
        Session {
            keys: Vec::new(),
            wallets: Vec::new(),
            network: START_NETWORK,
            demo: Vec::new(),
            network_fixed: false,
        }
    }
}

/// The network a PSBT is on, when it says: its global xpubs, or the coin
/// type of a BIP-44, 49, 84, 86 or 48 key path. A PSBT that says neither
/// is taken on the session's network.
pub fn psbt_network(psbt: &Psbt, current: Network) -> Option<Network> {
    use osk_bip::bitcoin::bip32::ChildNumber;
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
                _ => network_of_kind(osk_bip::bitcoin::NetworkKind::Test, current),
            });
        }
    }
    None
}

impl Session {
    /// A session on `network`.
    pub fn on(network: Network) -> Session {
        Session {
            network,
            ..Session::default()
        }
    }

    /// The network every key derives for.
    pub fn network(&self) -> Network {
        self.network
    }

    /// Moves the session to `network`. Keys are re-derived for it. While
    /// wallets are loaded it moves only between networks whose keys are
    /// written alike (the test networks), never between mainnet and a test
    /// network. A session whose network is fixed stays on it, and the
    /// demo goes to mainnet only with nothing but public seeds loaded.
    pub fn set_network(&mut self, network: Network) -> Result<(), Refusal> {
        if network == self.network {
            return Ok(());
        }
        if self.network_fixed {
            return Err(Refusal::NetworkFixed(self.network));
        }
        if network.is_mainnet()
            && let Some(k) = self.keys.iter().find(|k| !self.is_public(k.fingerprint()))
        {
            return Err(Refusal::NotPublicLoaded(fp_text(k.fingerprint())));
        }
        if !self.wallets.is_empty() && network.kind() != self.network.kind() {
            return Err(Refusal::Network {
                offered: network,
                loaded: self.network,
            });
        }
        for k in &mut self.keys {
            k.set_network(network);
        }
        self.network = network;
        Ok(())
    }

    /// Takes the network a wallet or a PSBT is on: the session moves to it
    /// when no wallet holds it elsewhere, and refuses it otherwise.
    pub fn follow(&mut self, network: Option<Network>) -> Result<(), Refusal> {
        let Some(n) = network else { return Ok(()) };
        if n.kind() == self.network.kind() {
            return Ok(());
        }
        self.set_network(n)
    }

    /// Takes the network of a PSBT that is about to be signed.
    pub fn follow_psbt(&mut self, psbt: &Psbt) -> Result<(), Refusal> {
        self.follow(psbt_network(psbt, self.network))
    }

    /// Whether any secret is held: the stick rule's test
    /// (`PLAN.md` §5.1).
    pub fn holds_secret(&self) -> bool {
        !self.keys.is_empty()
    }

    /// Adds the key the words give. `wanted` is the slot's fingerprint
    /// when the key is meant for one.
    pub fn add_words(
        &mut self,
        words: &str,
        label: &str,
        wanted: Option<Fingerprint>,
    ) -> Result<Fingerprint, Refusal> {
        self.add_words_with(words, "", label, wanted)
    }

    /// Adds the key the words and a BIP-39 passphrase give. The same words
    /// with another passphrase are another key. English is read first, then
    /// the other lists in BIP-39's order.
    pub fn add_words_with(
        &mut self,
        words: &str,
        passphrase: &str,
        label: &str,
        wanted: Option<Fingerprint>,
    ) -> Result<Fingerprint, Refusal> {
        let normal =
            zeroize::Zeroizing::new(words.split_whitespace().collect::<Vec<_>>().join(" "));
        let mut first = None;
        for lang in Language::ALL {
            match crate::wallet::parse_words(lang, &normal) {
                Ok(m) => return self.add_mnemonic(&m, passphrase, label, wanted),
                Err(e) => {
                    first.get_or_insert(e);
                }
            }
        }
        Err(Refusal::Words(
            first.map(|e| e.to_string()).unwrap_or_default(),
        ))
    }

    /// Adds the key a mnemonic and a BIP-39 passphrase give, keeping its
    /// words in its own list.
    pub fn add_mnemonic(
        &mut self,
        mnemonic: &Mnemonic,
        passphrase: &str,
        label: &str,
        wanted: Option<Fingerprint>,
    ) -> Result<Fingerprint, Refusal> {
        let lang = mnemonic.language();
        let seed = mnemonic
            .to_seed(passphrase.as_bytes())
            .map_err(|e| Refusal::Words(e.to_string()))?;
        let master = MasterKey::from_seed(&seed, self.network);
        let fp = master.fingerprint();
        if let Some(w) = wanted
            && w != fp
        {
            return Err(Refusal::WrongKey {
                wanted: fp_text(w),
                got: fp_text(fp),
            });
        }
        self.allowed(fp)?;
        if self.keys.iter().any(|k| k.fingerprint() == fp) {
            return Err(Refusal::Duplicate(fp_text(fp)));
        }
        let label = if label.trim().is_empty() {
            fp_text(fp)
        } else {
            label.trim().to_string()
        };
        let mut words = crate::secret_text::room();
        for (k, &i) in mnemonic.indices().iter().enumerate() {
            if k > 0 {
                words.push(' ');
            }
            words.push_str(lang.word(i));
        }
        let secp = osk_bip::bitcoin::secp256k1::Secp256k1::new();
        let share = secret_share(mnemonic).map(|s| s.public_share(&secp));
        self.keys.push(Key::new(
            master,
            label,
            Some(words),
            lang,
            share,
            (!passphrase.is_empty()).then(|| zeroize::Zeroizing::new(passphrase.to_string())),
        ));
        Ok(fp)
    }

    /// Refuses a key that is not a public seed while the demo is on
    /// ([`Session::demo`]).
    fn allowed(&self, fp: Fingerprint) -> Result<(), Refusal> {
        if self.demo() && !self.is_public(fp) {
            return Err(Refusal::NotAllowed(fp_text(fp)));
        }
        Ok(())
    }

    /// Whether the demo would take this key: always, when the shell set
    /// no demo.
    fn is_public(&self, fp: Fingerprint) -> bool {
        self.demo.is_empty() || self.demo.iter().any(|(_, f)| *f == fp)
    }

    /// Sets the demo up (F20): while the session is on mainnet it takes
    /// only the public seeds ([`crate::demo::SEEDS`]), every address it
    /// shows carries a broken checksum ([`broken_checksum`]) so that no
    /// wallet sends to it (scripts, PSBTs, files, signatures and every
    /// comparison keep the real address), and New key adds the next
    /// public seed in place of the key it made ([`Session::next_handout`]).
    /// On a test network none of that applies.
    pub fn set_demo(&mut self) {
        self.demo = crate::demo::SEEDS
            .iter()
            .filter_map(|&w| {
                let mut probe = Session::on(self.network);
                probe
                    .add_words(&crate::demo::words(w), "", None)
                    .ok()
                    .map(|fp| (w, fp))
            })
            .collect();
    }

    /// Whether the shell set the demo up, on any network.
    pub fn has_demo(&self) -> bool {
        !self.demo.is_empty()
    }

    /// Whether the demo is on: set up, and the session on mainnet.
    pub fn demo(&self) -> bool {
        !self.demo.is_empty() && self.network.is_mainnet()
    }

    /// The public seed a key is, as its word, when the demo is set up.
    pub fn public_seed(&self, fp: Fingerprint) -> Option<&'static str> {
        self.demo.iter().find(|(_, f)| *f == fp).map(|(w, _)| *w)
    }

    /// Keeps the session on the network it is on: every move to another
    /// is refused, a wallet's or a PSBT's included.
    pub fn fix_network(&mut self) {
        self.network_fixed = true;
    }

    /// Whether the session's network is fixed ([`Session::fix_network`]).
    pub fn network_fixed(&self) -> bool {
        self.network_fixed
    }

    /// Adds the key a master seed gives, as an `osk-backup` kind-2 backup
    /// carries one: there are no words to copy by hand.
    pub fn add_seed(&mut self, seed: &[u8], label: &str) -> Result<Fingerprint, Refusal> {
        let bytes = osk_crypto::SeedBytes::new(seed)
            .ok_or_else(|| Refusal::Words(EN.wallet_a_master_seed.to_string()))?;
        let master = MasterKey::from_seed_bytes(&osk_crypto::Secret::new(bytes), self.network);
        let fp = master.fingerprint();
        self.allowed(fp)?;
        if self.keys.iter().any(|k| k.fingerprint() == fp) {
            return Err(Refusal::Duplicate(fp_text(fp)));
        }
        self.keys.push(Key::new(
            master,
            if label.trim().is_empty() {
                fp_text(fp)
            } else {
                label.trim().to_string()
            },
            None,
            Language::English,
            None,
            None,
        ));
        Ok(fp)
    }

    /// Adds the wallet the text describes, under `name`, unless the same
    /// descriptor is already loaded. `origin` is what [`Wallet::source`]
    /// is: the caller already knows this (it is the one that chose
    /// `source`'s words), so it says so here rather than this reading
    /// the text back.
    pub fn add_wallet(
        &mut self,
        name: &str,
        text: &str,
        source: &str,
        origin: Origin,
    ) -> Result<usize, Refusal> {
        self.add_policy(name, read_wallet(text)?, source, origin)
    }

    /// [`Session::add_wallet`] of a description read before (a vault's
    /// kept records): no parse.
    pub fn add_policy(
        &mut self,
        name: &str,
        policy: WalletPolicy,
        source: &str,
        origin: Origin,
    ) -> Result<usize, Refusal> {
        let descriptor = same_wallet(&policy);
        if let Some(i) = self
            .wallets
            .iter()
            .position(|w| same_wallet(&w.policy) == descriptor)
        {
            return Ok(i);
        }
        self.follow(policy_network(&policy, self.network))?;
        self.wallets.push(Wallet::new(
            name.to_string(),
            policy,
            source.to_string(),
            origin,
        ));
        Ok(self.wallets.len() - 1)
    }

    /// Drops every key and wallet.
    pub fn wipe(&mut self) {
        self.keys.clear();
        self.wallets.clear();
    }

    /// The label of the session key with `fp`.
    pub fn key_label(&self, fp: Fingerprint) -> Option<&str> {
        self.keys
            .iter()
            .find(|k| k.fingerprint() == fp)
            .map(|k| k.label.as_str())
    }

    /// The keys loaded that no wallet loaded uses, in the order they
    /// were added.
    pub fn loose_keys(&self) -> Vec<Fingerprint> {
        self.keys
            .iter()
            .map(|k| k.fingerprint())
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
                held_by: self.key_label(r.fingerprint).map(str::to_string),
            }];
        }
        // A threshold wallet's slots are its shares, each named by its
        // own fingerprint.
        if let Some(record) = wallet.policy.record() {
            let here = self.shares_here(wallet);
            return (0..record.n())
                .map(|i| Slot {
                    fingerprint: record.share_fingerprint(i),
                    held_by: here
                        .iter()
                        .find(|(id, _)| *id as usize == i)
                        .map(|(_, k)| k.label.clone()),
                })
                .collect();
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
                        .and_then(|fp| self.key_label(fp))
                        .map(str::to_string),
                }
            })
            .collect()
    }

    /// The quorum: (signatures needed, keys).
    pub fn quorum(wallet: &Wallet) -> (usize, usize) {
        if let Some(r) = wallet.policy.record() {
            return (r.t(), r.n());
        }
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
            Kind::Threshold => fill(EN.wallet_of_frost, &[&m.to_string(), &n.to_string()]),
            Kind::Silent => EN.wallet_1_key_silent.to_string(),
            Kind::Other => EN.wallet_wallet.to_string(),
        }
    }

    /// An address of the wallet, on the session's network.
    /// Derived once per wallet and kept on it ([`Wallet::address_on`]).
    pub fn address(&self, wallet: &Wallet, change: bool, index: u32) -> String {
        wallet.address_on(self.network, change, index)
    }

    /// The public seed New key adds next while the demo is on, as its
    /// word and fingerprint: the first one not loaded. `None` with the
    /// demo off, as on the stick.
    pub fn next_handout(&self) -> Option<(&'static str, Fingerprint)> {
        if !self.demo() {
            return None;
        }
        self.demo
            .iter()
            .find(|(_, fp)| !self.keys.iter().any(|k| k.fingerprint() == *fp))
            .copied()
    }

    /// An address of the wallet as the screen shows it ([`Session::shown`]).
    pub fn address_shown(&self, wallet: &Wallet, change: bool, index: u32) -> String {
        self.shown(&self.address(wallet, change, index))
    }

    /// `address` as the screen shows it: as it is, or with its checksum
    /// broken while the demo is on ([`Session::set_demo`]).
    pub fn shown(&self, address: &str) -> String {
        if self.demo() {
            broken_checksum(address)
        } else {
            address.to_string()
        }
    }

    fn key_refs(&self) -> Vec<KeyRef> {
        self.keys
            .iter()
            .filter_map(|k| KeyRef::from_master(&k.master, 0).ok())
            .collect()
    }

    fn policies(&self) -> Vec<WalletPolicy> {
        self.wallets.iter().map(|w| w.policy.clone()).collect()
    }

    /// The nonce check at the end of a spend: every signature
    /// the PSBT carries checked against its key and this transaction, each
    /// one a key held here made recomputed from that key (RFC 6979 or
    /// BIP-340 with no extra randomness), and any nonce used twice under
    /// one key. OpenSigner's own checks (`osk_psbt::verify`).
    pub fn nonce_check(&self, psbt: &Psbt) -> NonceCheck {
        let refs = self.key_refs();
        let policies = self.policies();
        let ctx = context(self.network, &refs, &policies);
        let masters: Vec<&MasterKey> = self.keys.iter().map(|k| &k.master).collect();
        let made = osk_psbt::verify::deterministic(psbt, &masters);
        let mut rows = Vec::new();
        for input in osk_psbt::verify::verify_signatures(psbt, &ctx) {
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
            reused: osk_psbt::verify::repeated_nonces(psbt, &ctx),
        }
    }

    fn share_refs(&self) -> Vec<osk_psbt::ShareRef> {
        self.keys
            .iter()
            .filter_map(|k| k.share.map(|pubshare| osk_psbt::ShareRef { pubshare }))
            .collect()
    }

    /// What an inspection is worked out from beside the PSBT
    /// ([`InspectInputs`]). Lookups: each key's id and each wallet's
    /// checksum are kept.
    pub fn inspect_inputs(&self) -> InspectInputs {
        InspectInputs {
            network: self.network,
            keys: self.keys.iter().map(Key::master_id).collect(),
            wallets: self
                .wallets
                .iter()
                .map(|w| w.checksum().to_string())
                .collect(),
        }
    }

    /// The inspection of `psbt` against this session's keys and wallets.
    pub fn inspect(&self, psbt: &Psbt) -> Inspection {
        self.inspect_with(psbt, None)
    }

    /// The inspection, with the carry section a threshold spend arrived
    /// with.
    pub fn inspect_with(
        &self,
        psbt: &Psbt,
        carry: Option<&osk_psbt::threshold::CarrySection>,
    ) -> Inspection {
        let refs = self.key_refs();
        let policies = self.policies();
        let shares = self.share_refs();
        let mut ctx = context(self.network, &refs, &policies);
        ctx.shares = &shares;
        ctx.carry = carry;
        osk_psbt::inspect(psbt, &ctx)
    }

    /// The loaded keys that are shares of the threshold wallet `w`, by
    /// identifier.
    pub fn shares_here(&self, w: &Wallet) -> Vec<(u32, &Key)> {
        let Some(record) = w.policy.record() else {
            return Vec::new();
        };
        let mut out = Vec::new();
        for (id, pubshare) in record.info.pubshares.iter().enumerate() {
            let Some(pubshare) = pubshare else { continue };
            if let Some(k) = self
                .keys
                .iter()
                .find(|k| k.share.as_ref() == Some(pubshare))
            {
                out.push((id as u32, k));
            }
        }
        out
    }

    /// Signs `psbt` with every session key it names. Returns the
    /// fingerprints that signed.
    pub fn sign(
        &self,
        psbt: &mut Psbt,
        seed: [u8; 32],
        musig: &mut Option<osk_psbt::MusigSession>,
    ) -> Result<Vec<Fingerprint>, String> {
        self.sign_with(psbt, seed, musig, &[], None, None)
            .map(|s| s.by)
    }

    /// Signs `psbt` with every session key and share it names. `others`
    /// are the shares chosen to sign with this one, by identifier, at a
    /// threshold spend's first location; `carry` is the section a later
    /// location reads. `known` is the inspection of `psbt` with this
    /// session and `carry` when the caller keeps one current (the open
    /// spend's): it chooses the keys without inspecting again.
    pub fn sign_with(
        &self,
        psbt: &mut Psbt,
        seed: [u8; 32],
        musig: &mut Option<osk_psbt::MusigSession>,
        others: &[u32],
        carry: Option<&osk_psbt::threshold::CarrySection>,
        known: Option<&Inspection>,
    ) -> Result<Signed, String> {
        let refs = self.key_refs();
        let policies = self.policies();
        let share_refs = self.share_refs();
        let mut ctx = context(self.network, &refs, &policies);
        ctx.shares = &share_refs;
        ctx.carry = carry;
        let own;
        let inspection = match known {
            Some(i) => i,
            None => {
                own = osk_psbt::inspect(psbt, &ctx);
                &own
            }
        };
        let selection: Vec<Fingerprint> = inspection
            .participating_keys
            .iter()
            .copied()
            .filter(|fp| self.keys.iter().any(|k| k.fingerprint() == *fp))
            .collect();
        let shares_sign = inspection
            .inputs
            .iter()
            .any(|i| i.threshold.as_ref().is_some_and(|t| !t.ours.is_empty()));
        if selection.is_empty() && !shares_sign {
            return Err(EN.wallet_no_key_in_this.to_string());
        }
        let masters: Vec<&MasterKey> = self.keys.iter().map(|k| &k.master).collect();
        // A share's scalar is rebuilt from its words only inside the call
        // the signer makes through its door.
        let members: Vec<&Key> = self.keys.iter().filter(|k| k.share.is_some()).collect();
        let doors: Vec<Box<osk_psbt::ShareDoor<'_>>> = members
            .iter()
            .map(|key| -> Box<osk_psbt::ShareDoor<'_>> {
                Box::new(move |take: &mut dyn FnMut(&osk_bip::frost::SecShare)| {
                    key.with_secret_share(take);
                })
            })
            .collect();
        let share_keys: Vec<osk_psbt::ShareKey> = members
            .iter()
            .zip(&doors)
            .filter_map(|(key, door)| {
                Some(osk_psbt::ShareKey {
                    pubshare: key.share?,
                    open: door.as_ref(),
                })
            })
            .collect();
        let result = osk_psbt::sign(
            psbt,
            &masters,
            &selection,
            &ctx,
            false,
            Nonce::default(),
            Aux::default(),
            musig,
            seed,
            &share_keys,
            others,
        )
        .map_err(|e| e.to_string())?;
        let mut by: Vec<Fingerprint> = result.signed_inputs.iter().map(|s| s.fingerprint).collect();
        by.dedup();
        Ok(Signed {
            by,
            carry: result.threshold.and_then(|t| t.carry),
        })
    }
}

fn context<'a>(network: Network, refs: &'a [KeyRef], policies: &'a [WalletPolicy]) -> Context<'a> {
    Context {
        network,
        keys: refs,
        wallets: policies,
        musig_session: None,
        shares: &[],
        carry: None,
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
    /// FROST: a threshold of shares signs in two rounds.
    Threshold,
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
            Template::Threshold { .. } => Kind::Threshold,
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
        // A threshold spend is not collected here: each share signs in
        // turn, the carry file going from one to the next.
        if self.has_cosigners() && self != Kind::Threshold {
            v.push(step::COLLECT);
        }
        v.push(step::FINISH);
        v
    }
}

impl Session {
    /// The loaded wallet whose script the transaction's first input
    /// spends, found by re-deriving each wallet's script at the paths the
    /// input's own key origins state. Costs a txid and, the first time a
    /// wallet is asked a place, a script derivation ([`Wallet::script_at`]).
    pub fn wallet_for(&self, psbt: &Psbt) -> Option<usize> {
        self.spends_from(&PsbtSeen::of(psbt))
    }

    /// [`Session::wallet_for`] of a PSBT read before ([`PsbtSeen`]):
    /// lookups once each wallet keeps its scripts at the input's places,
    /// so a frame may ask it of every file in Files.
    pub fn spends_from(&self, seen: &PsbtSeen) -> Option<usize> {
        let (spk, places) = seen.first.as_ref()?;
        self.wallets.iter().position(|w| {
            places
                .iter()
                .any(|(change, index)| w.script_at(*change, *index).as_ref() == Some(spk))
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
    first: Option<(osk_bip::bitcoin::ScriptBuf, Vec<(bool, u32)>)>,
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
    if let Some(r) = wallet.policy.record() {
        return r.t();
    }
    wallet
        .policy
        .quorum()
        .or_else(|| wallet.policy.tapscript_quorum())
        .map(|(m, _)| m)
        .unwrap_or(1)
}

/// A wallet's descriptor in one form, so that the same wallet read from a
/// descriptor, a wallet .json or a multisig config compares equal: they
/// write a hardened step as `h` or as `'`.
pub fn same_wallet(policy: &WalletPolicy) -> String {
    policy.to_descriptor().replace('\'', "h")
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
    /// An OpenSigner encrypted backup (`osk-backup`).
    Backup,
    /// An EFI image to sign or check for Secure Boot.
    EfiImage,
    /// A threshold spend part-signed at another location: the PSBT with
    /// the secret nonce the next share signs with (`docs/PLANNING.md`
    /// §16.103 item 4).
    Carry,
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
    /// An APK: a ZIP whose central directory names `AndroidManifest.xml`
    /// (`docs/APK-SIGNING.md` §4.4). Public.
    Apk,
    /// One PEM `CERTIFICATE` block, nothing else: an APK signing key's
    /// certificate. Public.
    Certificate,
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

/// Reads a PSBT from a file's bytes, binary or base64.
pub fn read_psbt(bytes: &[u8]) -> Option<Psbt> {
    if let Ok(p) = Psbt::parse_bytes(bytes) {
        return Some(p);
    }
    let text = std::str::from_utf8(bytes).ok()?;
    Psbt::parse_base64(text.trim()).ok()
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

/// Whether a piece of text, a user ID or an armour header, is a seed's
/// words, an extended private key or a share: the detectors that read a
/// plain text file.
fn text_may_be_secret(text: &str) -> bool {
    faraday_intake::secret_text(text).is_some()
        || crate::forms::typed_mnemonic(Language::English, &crate::forms::file_words(text)).is_ok()
        || crate::restore::is_share(text)
        || crate::inbox::part_of(text).is_some()
}

/// Says what a file holds.
pub fn classify(name: &str, bytes: &[u8]) -> FileKind {
    classify_read(name, bytes).kind
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

/// [`classify`], keeping what it read to tell: a PSBT is parsed once and a
/// wallet file read once when a file comes in. Costs what [`classify`]
/// costs.
pub fn classify_read(name: &str, bytes: &[u8]) -> Classified {
    if faraday_vault::read_header(bytes).is_ok() {
        return FileKind::Vault.into();
    }
    if osk_backup::oskb::is_backup(bytes) {
        return FileKind::Backup.into();
    }
    if bytes.starts_with(b"%PDF-") {
        return FileKind::Pdf.into();
    }
    if faraday_intake::is_kdbx(bytes) {
        return FileKind::Kdbx.into();
    }
    if crate::secureboot::is_image(name) && bytes.starts_with(b"MZ") {
        return FileKind::EfiImage.into();
    }
    if faraday_apk::zip::names_manifest(bytes) {
        return FileKind::Apk.into();
    }
    if faraday_apk::pem::read(bytes).is_some() {
        return FileKind::Certificate.into();
    }
    if osk_psbt::threshold::Carry::looks_like_carry(bytes) {
        return FileKind::Carry.into();
    }
    if let Some(p) = read_psbt(bytes) {
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
            && !block.texts.iter().any(|t| text_may_be_secret(t))
        {
            return FileKind::Gpg.into();
        }
        if let Some(m) = osk_psbt::message::parse_signed(text)
            && m.address.parse::<osk_bip::bitcoin::Address<_>>().is_ok()
        {
            return FileKind::Message.into();
        }
        if crate::restore::is_share(text) {
            return FileKind::Share.into();
        }
        if faraday_intake::wallet::holds_secret(text) {
            return FileKind::SecretWallet.into();
        }
        if let Ok(w) = read_wallet(text) {
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
            && osk_psbt::transaction::read_raw(t.as_bytes()).is_some()
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

    /// Fingerprints with a valid signature on the first input; for a
    /// MuSig2 wallet, the participants whose partial signature is on it.
    /// Costs an inspection of the PSBT, but for MuSig2.
    pub fn signers(&self, session: &Session) -> Vec<Fingerprint> {
        self.signers_by(session, None)
    }

    /// [`Spend::signers`], read from `inspection` of this PSBT when one
    /// is given (the spend's kept one), else inspected now.
    pub fn signers_by(
        &self,
        session: &Session,
        inspection: Option<&Inspection>,
    ) -> Vec<Fingerprint> {
        if let Some(w) = session
            .wallet_for(&self.psbt)
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
            let partials = osk_psbt::musig::partial_sigs(input).unwrap_or_default();
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
                own = session.inspect(&self.psbt);
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

    /// Adds the signatures another device put on a copy of this
    /// transaction. A PSBT for any other transaction is refused, never
    /// merged. Returns the fingerprints it added.
    pub fn collect(
        &mut self,
        session: &Session,
        bytes: &[u8],
        source: &str,
    ) -> Result<Vec<Fingerprint>, String> {
        let other = read_psbt(bytes).ok_or(EN.wallet_not_a_psbt)?;
        if other.unsigned_tx().compute_txid() != self.txid {
            return Err(fill(EN.wallet_is_a_psbt_for, &[source]));
        }
        let before = self.signers(session);
        let mut merged = self.psbt.inner().clone();
        merged
            .combine(other.into_inner())
            .map_err(|e| fill(EN.wallet_cannot_combine, &[&e.to_string()]))?;
        // `combine` keeps the fields it has no type for, MuSig2's among them.
        let merged = Psbt::from(merged);
        if session
            .inspect(&merged)
            .present_signatures()
            .any(|s| s.verdict.is_invalid())
        {
            return Err(fill(EN.wallet_a_signature, &[source]));
        }
        self.psbt = merged;
        let added: Vec<Fingerprint> = self
            .signers(session)
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
        match osk_psbt::finalize(&mut copy) {
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
        match osk_psbt::finalize(&mut copy) {
            Ok(Some(_)) => None,
            Ok(None) => Some(EN.wallet_the_script_is.to_string()),
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
    pub signed: osk_psbt::message::SignedText,
    /// The signature verifies against the address, and in which format;
    /// or why not.
    pub verdict: Result<osk_psbt::message::Format, String>,
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
                    .is_some_and(|fp| self.key_label(fp).is_some())
            })
            .map(|(i, _)| i)
            .collect()
    }

    /// Signs `message` with the key behind address `index` of a
    /// single-key wallet's receive chain.
    pub fn sign_message(
        &self,
        wallet: usize,
        index: u32,
        message: &str,
        format: osk_psbt::message::Format,
    ) -> Result<osk_psbt::message::Signed, String> {
        let w = self.wallets.get(wallet).ok_or(EN.wallet_no_such_wallet)?;
        let Kind::Single(script) = Kind::of(&w.policy) else {
            return Err(EN.wallet_messages_are.into());
        };
        let key = w.policy.keys().first().ok_or(EN.wallet_the_wallet_has)?;
        let fp = key.fingerprint().ok_or(EN.wallet_the_wallet_s_xpub)?;
        let master = &self
            .keys
            .iter()
            .find(|k| k.fingerprint() == fp)
            .ok_or(EN.wallet_the_wallet_s_key)?
            .master;
        let origin = key.path().ok_or(EN.wallet_the_wallet_s)?;
        let path = origin
            .child(
                osk_bip::bitcoin::bip32::ChildNumber::from_normal_idx(0)
                    .map_err(|e| e.to_string())?,
            )
            .child(
                osk_bip::bitcoin::bip32::ChildNumber::from_normal_idx(index)
                    .map_err(|e| e.to_string())?,
            );
        let derived = master.derive(&path);
        osk_psbt::message::sign(
            derived.secp(),
            derived.secret_key(),
            script,
            self.network,
            message,
            format,
            Nonce::default(),
            Aux::default(),
        )
        .map_err(|e| format!("{e:?}"))
    }

    /// Checks a signed message, and finds its address among the loaded
    /// wallets' first hundred addresses on each chain.
    pub fn check_message(&self, text: &str) -> Option<MessageCheck> {
        let signed = osk_psbt::message::parse_signed(text)?;
        let address = signed
            .address
            .parse::<osk_bip::bitcoin::Address<_>>()
            .ok()?
            .require_network(self.network.into())
            .ok()?;
        let verdict = osk_psbt::message::verify(&address, &signed.message, &signed.signature)
            .map(|c| c.format)
            .map_err(|e| format!("{e:?}"));
        let wallet = self.wallets.iter().enumerate().find_map(|(i, w)| {
            w.policy
                .find_address(self.network, &address, 100)
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
    use osk_bip::spend::Lock;
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
                            EN.wallet_days_after_the,
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
        .and_then(|i| osk_psbt::musig::pub_nonces(i).ok())
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

/// `address` with the last character of its checksum changed to the next
/// one in its alphabet: a bech32 or bech32m address then fails its
/// checksum whatever the character (both detect any one substitution),
/// and a base58 one keeps its payload under a checksum that no longer
/// matches it. Text that is not an address is left as it is.
pub fn broken_checksum(address: &str) -> String {
    const BECH32: &str = "qpzry9x8gf2tvdw0s3jn54khce6mua7l";
    const BASE58: &str = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
    let lower = address.to_ascii_lowercase();
    let alphabet = match lower.rfind('1') {
        Some(sep)
            if sep > 0
                && address.len() - sep > 6
                && lower[sep + 1..].chars().all(|c| BECH32.contains(c)) =>
        {
            BECH32
        }
        _ if address.len() >= 26 && address.chars().all(|c| BASE58.contains(c)) => BASE58,
        _ => return address.to_string(),
    };
    let mut chars: Vec<char> = address.chars().collect();
    let Some(last) = chars.last_mut() else {
        return address.to_string();
    };
    let lc = last.to_ascii_lowercase();
    let at = alphabet
        .chars()
        .position(|c| c == if alphabet == BECH32 { lc } else { *last });
    if let Some(i) = at {
        let next = alphabet.chars().cycle().nth(i + 1).unwrap_or(*last);
        *last = if last.is_ascii_uppercase() && alphabet == BECH32 {
            next.to_ascii_uppercase()
        } else {
            next
        };
    }
    chars.into_iter().collect()
}
