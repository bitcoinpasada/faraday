//! The session's keys: the store of keys that can sign, held for this
//! session only (`docs/KEYS.md` §2 and §3).
//!
//! A key's words, its BIP-39 passphrase and its master key are private to
//! this module. What the rest of Faraday needs from a key is one of three
//! things:
//!
//! - a fact, which [`KeyInfo`] keeps beside each key, worked out once
//!   when the key is added (its fingerprint, its id, its label, its
//!   language, how many words it has, whether it has a passphrase, its
//!   FROST share);
//! - an operation, a method of [`Keys`] that computes with the secret and
//!   returns a public result (a comparison, a signature, a derived key
//!   text, a fingerprint, a record);
//! - a reveal, [`Keys::reveal`], the only way a key's words or passphrase
//!   leave as text. The reasons are the closed list [`Reveal`].
//!
//! A key is addressed by its place in the store, `i`, as the callers
//! already hold it; [`KeyInfo::id`] tells keys apart in all 32 bytes.

use faraday_bip::bip39::{Language, Mnemonic};
use faraday_bip::bitcoin::secp256k1::PublicKey;
use faraday_bip::keys::{Fingerprint, MasterKey, Network};
use faraday_crypto::work::Compute;
use faraday_intake::wallet::Refusal;
use zeroize::Zeroizing;

use crate::identity::{fp_text, master_id};
use crate::texts::Spec;

/// What tells one seed's words from another's, from whichever list they
/// are read: the SHA-256 of their entropy, all 32 bytes, so that two
/// seeds never pass for one (F65: what cannot be told apart is not
/// counted as kept). A hash of secret entropy: kept only beside what it
/// is of (a key, an open vault's records), wiped when dropped, never
/// written anywhere.
pub type WordsId = Zeroizing<[u8; 32]>;

/// Costs a hash; the words are parsed already. [`WordsId`] of `m`'s
/// words.
pub fn words_id(c: &Compute, m: &Mnemonic) -> WordsId {
    let entropy = m.entropy();
    Zeroizing::new(faraday_crypto::sha256_whole(c, entropy.expose().as_bytes()))
}

/// What tells one master key from another in all 32 bytes, the passphrase
/// applied: a hash of public values ([`master_id`]), the same on every
/// network. Public.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeyId(pub [u8; 32]);

/// What a key says about itself in public. Worked out once, when the key
/// is added, and kept beside the key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyInfo {
    /// All 32 bytes of what tells the key from another.
    pub id: KeyId,
    /// The master key's fingerprint; the same on every network.
    pub fingerprint: Fingerprint,
    /// What the person called it.
    pub label: String,
    /// The BIP-39 list its words are from (English for a key with none).
    pub language: Language,
    /// How many words it has; `None` for a key loaded from a master seed.
    pub word_count: Option<u8>,
    /// Whether it was loaded with a BIP-39 passphrase.
    pub has_passphrase: bool,
    /// The public share these words are as a FROST share: 24 words'
    /// 32 bytes read as the share's scalar. Every 24-word key has one; it
    /// signs as a share only for a loaded threshold record that lists it.
    pub share: Option<PublicKey>,
}

impl KeyInfo {
    /// Whether the key has words to show, copy and write down.
    pub fn has_words(&self) -> bool {
        self.word_count.is_some()
    }
}

/// Why a key's secret leaves the store as text. The list is closed
/// (`docs/KEYS.md` §9, F70): a new reason is a new variant, seen in
/// review.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reveal {
    /// The seed's words, shown on screen.
    Words,
    /// The key's BIP-39 passphrase, shown on screen.
    Passphrase,
    /// The words a SeedQR shown on screen is drawn from.
    SeedQr,
    /// The words shown for the person to copy by hand.
    CopyByHand,
    /// The words a paper form (Seed XOR, codex32, SLIP-39) is made from.
    PaperForm,
    /// The words a BIP-85 child is derived from.
    Bip85Child,
    /// The words a Lightning node key is derived from.
    LightningKey,
    /// The key a GPG paperkey is made from.
    GpgPaperkey,
    /// A secret of a vault record put on screen, into a form or into a
    /// record: a key's words or passphrase, an entry's password, TOTP
    /// secret or notes, a note's text.
    VaultEntry(Part),
    /// An unprotected file to the Outbox.
    OutboxFile(Part),
    /// The backup flow writing words, a passphrase or a SeedQR picture to
    /// a stick.
    BackupStick(Part),
}

/// Which of a key's two texts a file carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Part {
    /// The seed's words.
    Words,
    /// The BIP-39 passphrase.
    Passphrase,
    /// The seed's SeedQR picture (made by an operation, not lent).
    SeedQr,
    /// An entry's password.
    Password,
    /// An entry's TOTP secret, and the code made from it.
    Totp,
    /// A note's text.
    Note,
    /// An entry's notes.
    EntryNotes,
    /// A GPG key's revocation certificate, as a `.rev` file.
    Revocation,
    /// A silent payments scan key, BIP-392's `spscan1q…`.
    ScanKey,
    /// A Lightning node's private key.
    NodeKey,
}

impl Reveal {
    /// Which text this reason gives.
    pub(crate) fn part(self) -> Part {
        match self {
            Reveal::Passphrase
            | Reveal::VaultEntry(Part::Passphrase)
            | Reveal::OutboxFile(Part::Passphrase)
            | Reveal::BackupStick(Part::Passphrase) => Part::Passphrase,
            Reveal::VaultEntry(p) | Reveal::OutboxFile(p) | Reveal::BackupStick(p) => p,
            _ => Part::Words,
        }
    }
}

/// A secret an operation made, held until it is dropped. Like
/// [`Revealed`] it is neither `Clone` nor `Debug`; what it holds wipes
/// itself when dropped (`Zeroizing` or a type that zeroizes). Asking for
/// one names a [`Reveal`] reason.
pub struct Held<T>(T);

impl<T> Held<T> {
    /// Holds `secret`.
    pub(crate) fn of(secret: T) -> Held<T> {
        Held(secret)
    }

    /// Takes the secret out of the guard, to be put where the reason it
    /// was asked for says (a vault record, a form on screen).
    pub fn release(self) -> T {
        self.0
    }
}

impl<T> std::ops::Deref for Held<T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.0
    }
}

/// A key's secret text, lent by [`Keys::reveal`]. It borrows the store,
/// derefs to the text, and is neither `Clone` nor `Debug`: it lends the
/// store's own wiped text and makes no copy of it. Empty for a key that
/// has no such text.
pub struct Revealed<'a> {
    text: Text<'a>,
}

/// A revealed text is the store's own (lent), or made for the guard (a
/// vault record's words, written out from its payload) and wiped with it.
enum Text<'a> {
    Lent(&'a str),
    Made(Zeroizing<String>),
}

impl<'a> Revealed<'a> {
    /// No text: what a reveal of a field that is not there gives.
    pub fn empty() -> Revealed<'a> {
        Revealed::lent("")
    }

    /// A text lent from the store.
    pub(crate) fn lent(text: &'a str) -> Revealed<'a> {
        Revealed {
            text: Text::Lent(text),
        }
    }

    /// A text made for this guard.
    pub(crate) fn made(text: Zeroizing<String>) -> Revealed<'a> {
        Revealed {
            text: Text::Made(text),
        }
    }
}

impl std::ops::Deref for Revealed<'_> {
    type Target = str;
    fn deref(&self) -> &str {
        match &self.text {
            Text::Lent(t) => t,
            Text::Made(t) => t.as_str(),
        }
    }
}

/// The most key texts a key keeps ([`Keys::public_text`]); the next one
/// empties the list.
pub const KEY_TEXTS_KEPT: usize = 64;

/// A key that can sign, held for this session only.
struct Key {
    master: MasterKey,
    words: Option<Zeroizing<String>>,
    passphrase: Option<Zeroizing<String>>,
    /// Its public key texts asked for, each derived once
    /// ([`Keys::public_text`]), by what was asked: public, and dropped
    /// when the master key moves network.
    public: std::cell::RefCell<Vec<(Spec, Result<String, String>)>>,
    /// Its words' [`WordsId`], once worked out: wiped with the key.
    words_id: std::cell::OnceCell<Option<WordsId>>,
}

/// A key's secrets as the operations of this crate use them, lent.
pub(crate) struct Secrets<'a> {
    pub(crate) master: &'a MasterKey,
    pub(crate) words: Option<&'a str>,
    pub(crate) passphrase: Option<&'a str>,
    pub(crate) language: Language,
}

struct Entry {
    key: Key,
    info: KeyInfo,
}

/// The session's keys, the network every one derives for, and the demo's
/// public seeds when the shell set it (F20).
pub struct Keys {
    entries: Vec<Entry>,
    /// The network every key derives for and every address is shown on.
    network: Network,
    /// The demo's public seeds and their fingerprints, when the shell set
    /// it ([`Keys::set_demo`]). Empty, as on the stick: the session takes
    /// any key.
    demo: Vec<(&'static str, Fingerprint)>,
    /// Set by a shell that must stay on the network it started on
    /// ([`Keys::fix_network`]): every other one is refused.
    network_fixed: bool,
}

impl Default for Keys {
    fn default() -> Self {
        Keys::on(Network::Mainnet)
    }
}

impl Keys {
    /// A store on `network`, with no keys.
    pub fn on(network: Network) -> Keys {
        Keys {
            entries: Vec::new(),
            network,
            demo: Vec::new(),
            network_fixed: false,
        }
    }

    // ---- public reads ----

    /// How many keys are held.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether no key is held.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Whether any secret is held: the stick rule's test
    /// (`docs/history/PLAN.md` §5.1).
    pub fn holds_secret(&self) -> bool {
        !self.entries.is_empty()
    }

    /// What key `i` says about itself; a lookup, kept when it was added.
    pub fn info(&self, i: usize) -> Option<&KeyInfo> {
        self.entries.get(i).map(|e| &e.info)
    }

    /// Every key's [`KeyInfo`], in the order they were added.
    pub fn all(&self) -> Infos<'_> {
        Infos(self.entries.iter())
    }

    /// The first key's public facts.
    pub fn first(&self) -> Option<&KeyInfo> {
        self.entries.first().map(|e| &e.info)
    }

    /// The last key added, public facts.
    pub fn last(&self) -> Option<&KeyInfo> {
        self.entries.last().map(|e| &e.info)
    }

    /// How many key texts key `i` keeps ([`Keys::public_text`]), for the
    /// test that bounds the caches.
    pub fn texts_kept(&self, i: usize) -> usize {
        self.entries
            .get(i)
            .map_or(0, |e| e.key.public.borrow().len())
    }

    /// The place of the key with `fp`.
    pub fn position(&self, fp: Fingerprint) -> Option<usize> {
        self.entries.iter().position(|e| e.info.fingerprint == fp)
    }

    /// The place of the key whose fingerprint is `fp`, four bytes.
    pub fn position_of(&self, fp: [u8; 4]) -> Option<usize> {
        self.entries.iter().position(|e| e.info.fingerprint.0 == fp)
    }

    /// The key with `fp`.
    pub fn find(&self, fp: Fingerprint) -> Option<&KeyInfo> {
        self.info(self.position(fp)?)
    }

    /// The key whose fingerprint is `fp`, four bytes.
    pub fn find_of(&self, fp: [u8; 4]) -> Option<&KeyInfo> {
        self.info(self.position_of(fp)?)
    }

    /// The label of the key with `fp`.
    pub fn key_label(&self, fp: Fingerprint) -> Option<&str> {
        self.find(fp).map(|k| k.label.as_str())
    }

    /// The network every key derives for.
    pub fn network(&self) -> Network {
        self.network
    }

    /// Whether the store's network is fixed ([`Keys::fix_network`]).
    pub fn network_fixed(&self) -> bool {
        self.network_fixed
    }

    /// Keeps the store on the network it is on: every move to another is
    /// refused, a wallet's or a PSBT's included.
    pub fn fix_network(&mut self) {
        self.network_fixed = true;
    }

    /// Whether the demo is on: the shell set it up ([`Keys::set_demo`]).
    /// The network does not matter; the shell puts Demo on Testnet.
    pub fn demo(&self) -> bool {
        !self.demo.is_empty()
    }

    /// The public seed New key adds next while the demo is on, as its
    /// name and fingerprint: the first one not loaded. `None` with the
    /// demo off, as on the stick.
    pub fn next_handout(&self) -> Option<(&'static str, Fingerprint)> {
        if !self.demo() {
            return None;
        }
        self.demo
            .iter()
            .find(|(_, fp)| self.position(*fp).is_none())
            .copied()
    }

    /// Whether the demo would take this key: always, when the shell set
    /// no demo.
    pub fn is_public(&self, fp: Fingerprint) -> bool {
        self.demo.is_empty() || self.demo.iter().any(|(_, f)| *f == fp)
    }

    // ---- adding and forgetting ----

    /// Costs what `set_demo`'s probes cost, an `add_words` each. Sets the
    /// demo up (F20): `seeds` are the public seeds, each a name and its
    /// words. While the demo is on the store takes only these.
    pub fn set_demo(&mut self, c: &Compute, seeds: &[(&'static str, String)]) {
        self.demo = seeds
            .iter()
            .filter_map(|(name, words)| {
                let mut probe = Keys::on(self.network);
                probe
                    .add_words(c, words, "", None)
                    .ok()
                    .map(|fp| (*name, fp))
            })
            .collect();
    }

    /// Refuses a key that is not a public seed while the demo is on.
    fn allowed(&self, fp: Fingerprint) -> Result<(), Refusal> {
        if self.demo() && !self.is_public(fp) {
            return Err(Refusal::NotAllowed(fp_text(fp)));
        }
        Ok(())
    }

    /// Costs what `add_words_with` costs. Adds the key the words give.
    /// `wanted` is the slot's fingerprint when the key is meant for one.
    pub fn add_words(
        &mut self,
        c: &Compute,
        words: &str,
        label: &str,
        wanted: Option<Fingerprint>,
    ) -> Result<Fingerprint, Refusal> {
        self.add_words_with(c, words, "", label, wanted)
    }

    /// Costs a parse of the words and what `add_mnemonic` costs. Adds the
    /// key the words and a BIP-39 passphrase give. The same words with
    /// another passphrase are another key. English is read first, then the
    /// other lists in BIP-39's order.
    pub fn add_words_with(
        &mut self,
        c: &Compute,
        words: &str,
        passphrase: &str,
        label: &str,
        wanted: Option<Fingerprint>,
    ) -> Result<Fingerprint, Refusal> {
        let normal = Zeroizing::new(words.split_whitespace().collect::<Vec<_>>().join(" "));
        let mut first = None;
        for lang in Language::ALL {
            match Mnemonic::parse(c, lang, &normal) {
                Ok(m) => return self.add_mnemonic(c, &m, passphrase, label, wanted),
                Err(e) => {
                    first.get_or_insert(e);
                }
            }
        }
        Err(Refusal::Words(
            first.map(|e| e.to_string()).unwrap_or_default(),
        ))
    }

    /// Costs what `to_seed`, `from_seed` and `info_of` cost. Adds the key a
    /// mnemonic and a BIP-39 passphrase give, keeping its words in its own
    /// list.
    pub fn add_mnemonic(
        &mut self,
        c: &Compute,
        mnemonic: &Mnemonic,
        passphrase: &str,
        label: &str,
        wanted: Option<Fingerprint>,
    ) -> Result<Fingerprint, Refusal> {
        let lang = mnemonic.language();
        let seed = mnemonic
            .to_seed(c, passphrase.as_bytes())
            .map_err(|e| Refusal::Words(e.to_string()))?;
        let master = MasterKey::from_seed(c, &seed, self.network);
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
        if self.position(fp).is_some() {
            return Err(Refusal::Duplicate(fp_text(fp)));
        }
        let label = if label.trim().is_empty() {
            fp_text(fp)
        } else {
            label.trim().to_string()
        };
        let mut words = faraday_intake::secret_buf::room();
        for (k, &i) in mnemonic.indices().iter().enumerate() {
            if k > 0 {
                words.push(' ');
            }
            words.push_str(lang.word(i));
        }
        let share = crate::frost::public_share(mnemonic);
        let word_count = u8::try_from(mnemonic.word_count()).ok();
        self.entries.push(Entry::new(
            c,
            master,
            label,
            Some(words),
            lang,
            word_count,
            share,
            (!passphrase.is_empty()).then(|| Zeroizing::new(passphrase.to_string())),
        ));
        Ok(fp)
    }

    /// Costs what `from_seed_bytes` and `info_of` cost. Adds the key a
    /// master seed gives, as an `.oskb` kind-2 backup carries one: there
    /// are no words to copy by hand.
    pub fn add_seed(
        &mut self,
        c: &Compute,
        seed: &[u8],
        label: &str,
    ) -> Result<Fingerprint, Refusal> {
        let bytes = faraday_crypto::SeedBytes::new(seed)
            .ok_or_else(|| Refusal::Words(faraday_strings::EN.wallet_a_master_seed.to_string()))?;
        let master =
            MasterKey::from_seed_bytes(c, &faraday_crypto::Secret::new(bytes), self.network);
        let fp = master.fingerprint();
        self.allowed(fp)?;
        if self.position(fp).is_some() {
            return Err(Refusal::Duplicate(fp_text(fp)));
        }
        let label = if label.trim().is_empty() {
            fp_text(fp)
        } else {
            label.trim().to_string()
        };
        self.entries.push(Entry::new(
            c,
            master,
            label,
            None,
            Language::English,
            None,
            None,
            None,
        ));
        Ok(fp)
    }

    /// Costs what `add_words_with` costs. Adds the key key `i`'s words and
    /// `passphrase` give, under `label`: the same words with another
    /// passphrase. A refusal for a key with no words.
    pub fn add_with_passphrase(
        &mut self,
        c: &Compute,
        i: usize,
        passphrase: &str,
        label: &str,
    ) -> Result<Fingerprint, Refusal> {
        let words = self
            .entries
            .get(i)
            .and_then(|e| e.key.words.clone())
            .ok_or_else(|| Refusal::Words(String::new()))?;
        self.add_words_with(c, &words, passphrase, label, None)
    }

    /// Costs what `add_with_passphrase` costs. Adds the key key `i`'s
    /// words and its own passphrase followed by `suffix` give: the
    /// passphrase a vanity search found.
    pub fn add_with_suffix(
        &mut self,
        c: &Compute,
        i: usize,
        suffix: &str,
        label: &str,
    ) -> Result<Fingerprint, Refusal> {
        let (words, base) = {
            let key = &self
                .entries
                .get(i)
                .ok_or_else(|| Refusal::Words(String::new()))?
                .key;
            (
                key.words
                    .clone()
                    .ok_or_else(|| Refusal::Words(String::new()))?,
                key.passphrase.clone(),
            )
        };
        let passphrase = Zeroizing::new(format!(
            "{}{suffix}",
            base.as_deref().map_or("", |p| p.as_str())
        ));
        let lang = self.entries[i].info.language;
        let m = Mnemonic::parse(c, lang, &words).map_err(|e| Refusal::Words(e.to_string()))?;
        self.add_mnemonic(c, &m, &passphrase, label, None)
    }

    /// Drops key `i`.
    pub fn remove(&mut self, i: usize) {
        if i < self.entries.len() {
            self.entries.remove(i);
        }
    }

    /// Keeps the keys `keep` says yes to.
    pub fn retain(&mut self, mut keep: impl FnMut(&KeyInfo) -> bool) {
        self.entries.retain(|e| keep(&e.info));
    }

    /// Drops every key.
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Sets key `i`'s label.
    pub fn set_label(&mut self, i: usize, label: &str) {
        if let Some(e) = self.entries.get_mut(i) {
            e.info.label = label.to_string();
        }
    }

    /// Moves the store to `network`; every key is re-derived for it. For
    /// a shell's start and for `Session::follow` (an empty session
    /// taking the network of a wallet or a PSBT); no press of a person
    /// reaches it (F81, `docs/MODES.md` §2.7). The caller says whether
    /// wallets are loaded: they stop it moving between networks whose
    /// keys are written differently. A fixed store stays on its network.
    pub fn set_network(&mut self, network: Network, wallets_loaded: bool) -> Result<(), Refusal> {
        if network == self.network {
            return Ok(());
        }
        if self.network_fixed {
            return Err(Refusal::NetworkFixed(self.network));
        }
        if wallets_loaded && network.kind() != self.network.kind() {
            return Err(Refusal::Network {
                offered: network,
                loaded: self.network,
            });
        }
        for e in &mut self.entries {
            e.key.master = e.key.master.for_network(network);
            e.key.public.borrow_mut().clear();
        }
        self.network = network;
        Ok(())
    }

    /// What the operations of this crate compute with: key `i`'s master
    /// key, words and passphrase, borrowed. Never leaves the crate.
    pub(crate) fn secrets(&self, i: usize) -> Option<Secrets<'_>> {
        let e = self.entries.get(i)?;
        Some(Secrets {
            master: &e.key.master,
            words: e.key.words.as_deref().map(|w| w.as_str()),
            passphrase: e.key.passphrase.as_deref().map(|p| p.as_str()),
            language: e.info.language,
        })
    }

    // ---- the one door for text ----

    /// The key's words or passphrase as a reason names them, lent while
    /// the guard lives. Empty for a key without them, or no such key.
    pub fn reveal(&self, i: usize, why: Reveal) -> Revealed<'_> {
        let text = self.entries.get(i).and_then(|e| match why.part() {
            Part::Words | Part::SeedQr => e.key.words.as_deref(),
            Part::Passphrase => e.key.passphrase.as_deref(),
            _ => None,
        });
        Revealed::lent(text.map_or("", |t| t.as_str()))
    }

    // ---- operations on a key's secret ----

    /// Costs a parse of the words the first time. What tells key `i`'s
    /// words apart from other words, whatever list they were typed from
    /// and whatever passphrase is added ([`WordsId`]): worked out the
    /// first time it is asked, by whoever asks, a frame included, and kept
    /// on the key until it is dropped. `None` for a key with no words.
    pub fn words_id(&self, c: &Compute, i: usize) -> Option<&[u8; 32]> {
        let e = self.entries.get(i)?;
        let words = e.key.words.as_ref()?;
        e.key
            .words_id
            .get_or_init(|| {
                Some(words_id(
                    c,
                    &Mnemonic::parse(c, e.info.language, words).ok()?,
                ))
            })
            .as_deref()
    }

    /// Costs a comparison of the words' hashes and a parse the first time.
    /// Whether keys `a` and `b` have the same words, from whichever list.
    pub fn same_words(&self, c: &Compute, a: usize, b: usize) -> bool {
        match (self.words_id(c, a), self.words_id(c, b)) {
            (Some(x), Some(y)) => x == y,
            _ => false,
        }
    }

    /// The places of the keys that have the words key `i` has, `i`
    /// included: the same words with and without a passphrase.
    pub fn with_same_words(&self, i: usize) -> Vec<usize> {
        let Some(words) = self.entries.get(i).and_then(|e| e.key.words.as_ref()) else {
            return Vec::new();
        };
        self.entries
            .iter()
            .enumerate()
            .filter(|(_, o)| o.key.words.as_ref() == Some(words))
            .map(|(k, _)| k)
            .collect()
    }

    /// Whether key `i`'s passphrase is exactly `text`.
    pub fn passphrase_is(&self, i: usize, text: &str) -> bool {
        self.entries
            .get(i)
            .and_then(|e| e.key.passphrase.as_ref())
            .is_some_and(|p| p.as_str() == text)
    }

    /// Whether key `i`'s passphrase is exactly `typed`, compared whole and
    /// in constant time. A key that keeps no passphrase matches nothing.
    pub fn passphrase_matches(&self, i: usize, typed: &str) -> bool {
        self.entries
            .get(i)
            .and_then(|e| e.key.passphrase.as_ref())
            .is_some_and(|p| {
                faraday_crypto::Secret::new(p.as_bytes().to_vec())
                    == faraday_crypto::Secret::new(typed.as_bytes().to_vec())
            })
    }

    /// Costs a parse of the words. Compares a copy of key `i`'s words,
    /// scanned or typed, as BIP-39 numbers, with its words
    /// ([`compare_words`]). `None` where the key has no words.
    pub fn words_match(&self, c: &Compute, i: usize, copy: &[u16]) -> Option<CopyCheck> {
        let e = self.entries.get(i)?;
        let m = Mnemonic::parse(c, e.info.language, e.key.words.as_ref()?).ok()?;
        Some(compare_words(copy, m.indices()))
    }

    /// Costs a parse of the words and a digit stream. Whether `payload` is
    /// the digits key `i`'s SeedQR holds (English words only, as the
    /// picture is made). `None` where the words are not here.
    pub fn seedqr_reads_as(&self, c: &Compute, i: usize, payload: &[u8]) -> Option<bool> {
        let e = self.entries.get(i)?;
        if e.info.language != Language::English {
            return None;
        }
        let m = Mnemonic::parse(c, e.info.language, e.key.words.as_deref()?).ok()?;
        let digits = faraday_codec::seedqr::to_digits(&m);
        Some(digits.expose().as_bytes() == payload)
    }

    /// Costs a parse of the words and a QR encoding. Key `i`'s SeedQR as a
    /// picture's matrix, standard or `compact`, with its word count: for
    /// the picture a stick or a file carries. `None` where the words are
    /// not here or the encoding refuses them.
    pub fn seedqr_matrix(
        &self,
        c: &Compute,
        i: usize,
        compact: bool,
        _why: Reveal,
    ) -> Option<(Held<faraday_codec::qr::QrMatrix>, usize)> {
        let e = self.entries.get(i)?;
        let m = Mnemonic::parse(c, e.info.language, e.key.words.as_deref()?).ok()?;
        let matrix = if compact {
            faraday_codec::seedqr::encode_compact(c, &m)
        } else {
            faraday_codec::seedqr::encode_seedqr(c, &m)
        }
        .ok()?;
        Some((Held(matrix), m.word_count()))
    }

    /// Costs a PBKDF2 and an elliptic-curve multiplication. The
    /// fingerprint key `i`'s words make with `passphrase`; `None` for a key
    /// with no words or a passphrase BIP-39 refuses.
    pub fn fingerprint_with(&self, c: &Compute, i: usize, passphrase: &str) -> Option<Fingerprint> {
        let e = self.entries.get(i)?;
        let m = Mnemonic::parse(c, e.info.language, e.key.words.as_deref()?).ok()?;
        crate::identity::fingerprint(
            c,
            crate::identity::Material::Words(&m),
            passphrase,
            self.network,
        )
    }

    /// Costs a parse of the words. Key `i`'s vault key record: its words,
    /// its label, `flags` when given and its BIP-39 passphrase when
    /// `with_passphrase`. `None` for a key with no words.
    pub fn key_record(
        &self,
        c: &Compute,
        i: usize,
        flags: Option<u8>,
        with_passphrase: bool,
        _why: Reveal,
    ) -> Option<Held<faraday_vault::Record>> {
        use faraday_vault::records::{self, field, kind};
        let e = self.entries.get(i)?;
        let m = Mnemonic::parse(c, e.info.language, e.key.words.as_ref()?).ok()?;
        let mut record = faraday_vault::Record::new(kind::KEY)
            .with(field::KEY, &records::words_payload(&m))
            .with(field::KEY_LABEL, e.info.label.as_bytes());
        if let Some(f) = flags {
            record.push(field::KEY_FLAGS, &[f]);
        }
        // A BIP-39 passphrase is stored only when the person asks.
        if with_passphrase && let Some(p) = e.key.passphrase.as_ref() {
            record.push(field::KEY_PASSPHRASE, p.as_bytes());
        }
        Some(Held(record))
    }

    /// Costs a parse of the words. A vault key record of key `i`'s words
    /// alone, with no label and no passphrase, kept in this crate.
    fn words_alone(&self, c: &Compute, i: usize) -> Option<faraday_vault::Record> {
        use faraday_vault::records::{self, field, kind};
        let e = self.entries.get(i)?;
        let m = Mnemonic::parse(c, e.info.language, e.key.words.as_ref()?).ok()?;
        Some(faraday_vault::Record::new(kind::KEY).with(field::KEY, &records::words_payload(&m)))
    }

    /// Costs a parse of the words and a hash. The id
    /// ([`crate::vault::key_record_id`]) a vault key record of key `i`'s
    /// words alone, with no passphrase, has: what is kept of such a record
    /// is found by it. `None` for a key with no words.
    pub fn words_record_id(&self, c: &Compute, i: usize) -> Option<[u8; 32]> {
        let r = self.words_alone(c, i)?;
        crate::vault::key_record_id(crate::vault::RecordView::of(&r))
    }

    /// Costs a parse of the words, a PBKDF2 and two elliptic-curve
    /// multiplications. What [`crate::vault::key_identity`] gives for a
    /// vault key record of key `i`'s words alone, without its passphrase:
    /// the fingerprint and master id of the seed the words are when no
    /// passphrase is added. `None` for a key with no words.
    pub fn words_identity(&self, c: &Compute, i: usize) -> Option<(Fingerprint, [u8; 32])> {
        let r = self.words_alone(c, i)?;
        crate::vault::key_identity(c, self.network, crate::vault::RecordView::of(&r))
    }

    /// Key `i`'s master key, for tests (feature `master-for-tests`, which
    /// no shipped binary has). Panics for a place with no key.
    #[cfg(feature = "master-for-tests")]
    pub fn master_for_tests(&self, i: usize) -> &MasterKey {
        &self.entries[i].key.master
    }

    /// Costs a key derivation the first time. The public key text of key
    /// `i` as `spec` says (`[fingerprint/path]xpub`): derived the first
    /// time it is asked, by whoever asks, a frame included, and kept on the
    /// key, a lookup after. `None` for a place with no key.
    pub fn key_text(&self, c: &Compute, i: usize, spec: &Spec) -> Option<Result<String, String>> {
        let key = &self.entries.get(i)?.key;
        if let Some((_, t)) = key.public.borrow().iter().find(|(a, _)| a == spec) {
            return Some(t.clone());
        }
        let t = crate::texts::key_text(c, &key.master, spec);
        let mut kept = key.public.borrow_mut();
        if kept.len() >= KEY_TEXTS_KEPT {
            kept.clear();
        }
        kept.push((spec.clone(), t.clone()));
        Some(t)
    }

    /// Costs a key derivation. The multisig account key of key `i` for
    /// `script` at `account`: public.
    pub fn multisig_account(
        &self,
        c: &Compute,
        i: usize,
        script: faraday_bip::keys::MultisigScriptType,
        account: u32,
    ) -> Option<Result<crate::texts::AccountKey, String>> {
        Some(crate::texts::multisig_account(
            c,
            &self.entries.get(i)?.key.master,
            script,
            account,
        ))
    }

    /// Costs a key derivation and a signature. Key `i`'s BIP 129 key
    /// record for the multisig `script` at `account`, signed by the account
    /// key itself: the record's text.
    pub fn bsms_record(
        &self,
        c: &Compute,
        i: usize,
        script: faraday_bip::keys::MultisigScriptType,
        account: u32,
        description: &str,
    ) -> Option<String> {
        crate::texts::bsms_record(
            c,
            &self.entries.get(i)?.key.master,
            script,
            account,
            description,
        )
    }

    /// Costs a key derivation and a signature. A message signed in
    /// `format` by the key at `path` below key `i`, for an address of
    /// `script` on the store's network.
    pub fn sign_message(
        &self,
        c: &Compute,
        i: usize,
        path: &faraday_bip::bitcoin::bip32::DerivationPath,
        script: faraday_bip::keys::ScriptType,
        message: &str,
        format: faraday_psbt::message::Format,
    ) -> Option<Result<faraday_psbt::message::Signed, faraday_psbt::message::Error>> {
        Some(crate::message::sign(
            c,
            &self.entries.get(i)?.key.master,
            path,
            script,
            self.network,
            message,
            format,
        ))
    }

    /// Costs a parse of the words. Whether key `i`'s words, as a vault key
    /// record's payload, are among `held`.
    pub fn words_payload_among(&self, c: &Compute, i: usize, held: &[&[u8]]) -> bool {
        use faraday_vault::records;
        let Some(e) = self.entries.get(i) else {
            return false;
        };
        let Some(words) = e.key.words.as_ref() else {
            return false;
        };
        Mnemonic::parse(c, e.info.language, words)
            .ok()
            .is_some_and(|m| held.contains(&records::words_payload(&m).as_slice()))
    }

    // ---- signing and inspecting ----

    /// Costs a key derivation per key. The keys as the PSBT crate refers to
    /// them.
    pub fn key_refs(&self, c: &Compute) -> Vec<faraday_psbt::KeyRef> {
        let masters: Vec<&MasterKey> = self.entries.iter().map(|e| &e.key.master).collect();
        crate::psbt::key_refs(c, &masters)
    }

    /// The loaded keys that are FROST shares, by their public share.
    pub fn share_refs(&self) -> Vec<faraday_psbt::ShareRef> {
        self.entries
            .iter()
            .filter_map(|e| {
                e.info
                    .share
                    .map(|pubshare| faraday_psbt::ShareRef { pubshare })
            })
            .collect()
    }

    /// Costs what `deterministic` costs. The signatures `psbt` carries that
    /// a key held here recomputes, and by which nonce rule.
    pub fn recompute_signatures(
        &self,
        c: &Compute,
        psbt: &faraday_psbt::Psbt,
    ) -> Vec<faraday_psbt::verify::Determinism> {
        let masters: Vec<&MasterKey> = self.entries.iter().map(|e| &e.key.master).collect();
        crate::psbt::recompute_signatures(c, psbt, &masters)
    }

    /// Costs what `faraday_psbt::sign` costs. Signs `psbt` with the keys in
    /// `selection` and every share that signs; `others` are the shares
    /// chosen to sign with this one, and `carry` the section the spend
    /// arrived with. A share's scalar is rebuilt from its words only inside
    /// the call the signer makes through its door. The result, and the
    /// carry section the pass leaves for the shares still to sign, taken
    /// out of it.
    #[allow(clippy::too_many_arguments)]
    pub fn sign(
        &self,
        c: &Compute,
        psbt: &mut faraday_psbt::Psbt,
        selection: &[Fingerprint],
        ctx: faraday_psbt::Context<'_>,
        carry: Option<&crate::carry::Section>,
        musig: &mut Option<faraday_psbt::MusigSession>,
        seed: [u8; 32],
        others: &[u32],
    ) -> Result<(faraday_psbt::SignResult, Option<crate::carry::Section>), faraday_psbt::Error>
    {
        let mut ctx = ctx;
        ctx.carry = carry.map(|s| &s.0);
        let masters: Vec<&MasterKey> = self.entries.iter().map(|e| &e.key.master).collect();
        let holders: Vec<crate::psbt::ShareHolder<'_>> = self
            .entries
            .iter()
            .filter_map(|e| {
                Some(crate::psbt::ShareHolder {
                    pubshare: e.info.share?,
                    language: e.info.language,
                    words: e.key.words.as_ref().map(|w| w.as_str()),
                })
            })
            .collect();
        let mut result = crate::psbt::sign(
            c, psbt, &masters, selection, &ctx, musig, seed, &holders, others,
        )?;
        let left = result
            .threshold
            .as_mut()
            .and_then(|t| t.carry.take())
            .map(crate::carry::Section);
        Ok((result, left))
    }
}

impl Entry {
    #[allow(clippy::too_many_arguments)]
    /// Costs an elliptic-curve multiplication with the master private
    /// key, for its id. Makes a key and its [`KeyInfo`].
    fn new(
        c: &Compute,
        master: MasterKey,
        label: String,
        words: Option<Zeroizing<String>>,
        language: Language,
        word_count: Option<u8>,
        share: Option<PublicKey>,
        passphrase: Option<Zeroizing<String>>,
    ) -> Entry {
        let info = KeyInfo {
            id: KeyId(master_id(c, &master)),
            fingerprint: master.fingerprint(),
            label,
            language,
            word_count,
            has_passphrase: passphrase.is_some(),
            share,
        };
        Entry {
            key: Key {
                master,
                words,
                passphrase,
                public: std::cell::RefCell::new(Vec::new()),
                words_id: std::cell::OnceCell::new(),
            },
            info,
        }
    }
}

/// What comparing a copy of a seed with the seed says.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CopyCheck {
    /// Every word matches.
    Matches,
    /// A word differs, counting from one.
    WrongWord(usize),
}

/// Compares the words of a copy, scanned or typed, with the seed's, by
/// their BIP-39 numbers: the first that differs, counting from one, or a
/// copy of another length at the first word one has and the other lacks.
pub fn compare_words(copy: &[u16], seed: &[u16]) -> CopyCheck {
    match copy.iter().zip(seed).position(|(a, b)| a != b) {
        Some(i) => CopyCheck::WrongWord(i + 1),
        None if copy.len() == seed.len() => CopyCheck::Matches,
        None => CopyCheck::WrongWord(copy.len().min(seed.len()) + 1),
    }
}

/// The keys' public facts, in the order the keys were added
/// ([`Keys::all`]).
pub struct Infos<'a>(std::slice::Iter<'a, Entry>);

impl<'a> Iterator for Infos<'a> {
    type Item = &'a KeyInfo;
    fn next(&mut self) -> Option<&'a KeyInfo> {
        self.0.next().map(|e| &e.info)
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }
}

impl DoubleEndedIterator for Infos<'_> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.0.next_back().map(|e| &e.info)
    }
}

impl ExactSizeIterator for Infos<'_> {}

impl<'a> IntoIterator for &'a Keys {
    type Item = &'a KeyInfo;
    type IntoIter = Infos<'a>;
    fn into_iter(self) -> Infos<'a> {
        self.all()
    }
}

impl std::ops::Index<usize> for Keys {
    type Output = KeyInfo;
    /// Key `i`'s public facts; panics for a place with no key, as a
    /// slice does.
    fn index(&self, i: usize) -> &KeyInfo {
        &self.entries[i].info
    }
}
