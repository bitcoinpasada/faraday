//! The session's keys: the store of keys that can sign, held for this
//! session only (`docs/KEYS.md` §2 and §3).
//!
//! A key's words, its BIP-39 passphrase and its master key are private to
//! this module. What the rest of Faraday needs from a key is one of three
//! things:
//!
//! - a fact, which [`KeyInfo`] keeps beside each key, worked out once
//!   when the key is added (its fingerprint, its id, its label, its
//!   language, how many words it has, whether it has a passphrase);
//! - an operation, a method of [`Keys`] that computes with the secret and
//!   returns a public result (a comparison, a signature, a derived key
//!   text, a fingerprint, a record);
//! - a reveal, [`Keys::reveal`], the only way a key's words or passphrase
//!   leave as text, and Explore's three made for their own reason
//!   ([`Keys::seed_shown`], [`Keys::xprv_at`], [`Keys::wif_at`]). The
//!   reasons are the closed list [`Reveal`].
//!
//! A key is addressed by its place in the store, `i`, as the callers
//! already hold it; [`KeyInfo::id`] tells keys apart in all 32 bytes.
#![cfg_attr(not(test), warn(clippy::disallowed_methods))]

use faraday_bip::bip39::{Language, Mnemonic};
use faraday_bip::bitcoin::bip32::DerivationPath;
use faraday_bip::keys::{Fingerprint, MasterKey, Network, WifAscii, XprivAscii};
use faraday_crypto::work::Compute;
use faraday_intake::wallet::Refusal;
use faraday_text::{Text, text};
use subtle::ConstantTimeEq;
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
#[allow(
    clippy::disallowed_methods,
    reason = "a derived Clone: no secret type derives it"
)]
pub struct KeyInfo {
    /// All 32 bytes of what tells the key from another.
    pub id: KeyId,
    /// The master key's fingerprint; the same on every network.
    pub fingerprint: Fingerprint,
    /// What the person called it.
    pub label: Text,
    /// The BIP-39 list its words are from (English for a key with none).
    pub language: Language,
    /// How many words it has; `None` for a key loaded from a master seed.
    pub word_count: Option<u8>,
    /// How many bytes its master seed has; `None` for a key of words. A
    /// length, never the bytes.
    pub seed_len: Option<usize>,
    /// Whether it was loaded with a BIP-39 passphrase.
    pub has_passphrase: bool,
    /// The codex32 set the key's seed is split into, as its plain facts
    /// (`docs/CODEX32.md` §9.4): set when Make codex32 shares loads it,
    /// when it is loaded from strings, or read from its vault record.
    /// No string. `None` for a key with no set known.
    pub codex32: Option<faraday_vault::codex32::SetFacts>,
    /// The BIP-85 key this key is a child seed of, and where
    /// (`docs/decisions/F166.md`): set when the BIP-85 flow loads it and
    /// the person records it, or read from its vault record's field 6.
    /// Written into every vault record of the key; nowhere else. `None`
    /// for a key not recorded as a child.
    pub child_of: Option<faraday_vault::child::ChildOf>,
    /// The Seed XOR set the key's seed is split into, as its plain facts
    /// (`docs/VAULT.md` §7): the number of parts and each part's master
    /// fingerprint, never a part. One set per key ([`Keys::set_xor`]).
    /// Set when Convert saves a whole set of the key, or read from
    /// its vault record's field 7. `None` for a key with no set known.
    pub xor: Option<faraday_vault::xor::XorFacts>,
    /// The SLIP-39 set the key's seed is split into, as its plain facts
    /// (`docs/VAULT.md` §7): identifier, iteration exponent, group
    /// threshold, each group's threshold and count and the member indices
    /// made, never a share. One set per key ([`Keys::set_slip39`]). Read
    /// from its vault record's field 8. `None` for a key with no set known.
    pub slip39: Option<faraday_vault::slip39::Slip39Facts>,
}

impl KeyInfo {
    /// Whether the key has words to show, copy and write down.
    pub fn has_words(&self) -> bool {
        self.word_count.is_some()
    }

    /// Whether a Krux encrypted code can be made of the key (`docs/KEF.md`
    /// §6): 12 or 24 words of the English list. A lookup of what is
    /// kept beside the key; no word is parsed.
    pub fn kef_offered(&self) -> bool {
        self.language == Language::English && matches!(self.word_count, Some(12 | 24))
    }
}

/// Why a key's secret leaves the store as text. The list is closed
/// (`docs/KEYS.md` §2, F70): a new reason is a new variant, seen in
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
    /// A paper form (Seed XOR, codex32, SLIP-39), the master seed as hex
    /// and the xprv, of a key's words or seed bytes.
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
    /// The master seed of a key with no words, shown on Explore behind its
    /// eye ([`Keys::seed_shown`]): in hex, and where its codex32 set is
    /// known, the set's secret recovered from the strings this session
    /// holds, and those strings.
    SeedShown,
    /// An extended private key shown on Explore behind its eye
    /// ([`Keys::xprv_at`]): the root's, or the one at the explored
    /// path's account.
    ExtendedPrivateKey,
    /// The private key of one address, as WIF, shown on Explore behind its
    /// eye ([`Keys::wif_at`]).
    AddressKey,
    /// A seed's entropy, 16 or 32 bytes, sealed as a Krux encrypted code
    /// ([`Keys::kef_entropy`], `docs/KEF.md` §6): the bytes leave as a
    /// wiped-on-drop buffer and go straight into the code's encryption.
    /// It lends no stored text.
    KefCode,
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
    /// A codex32 string of the key's set, a share (`docs/CODEX32.md`
    /// §9.5): a file of it on a stick, or a share record in a vault.
    Codex32,
    /// A key file Faraday made for a vault, as a KeePass key file.
    KeyFile,
}

impl Reveal {
    /// Which of a key's or a record's texts this reason gives; `None` for
    /// a reason that lends no stored text (Explore's seed, extended
    /// private keys and address keys, each made by its own method).
    pub(crate) fn part(self) -> Option<Part> {
        Some(match self {
            Reveal::Passphrase
            | Reveal::VaultEntry(Part::Passphrase)
            | Reveal::OutboxFile(Part::Passphrase)
            | Reveal::BackupStick(Part::Passphrase) => Part::Passphrase,
            Reveal::VaultEntry(p) | Reveal::OutboxFile(p) | Reveal::BackupStick(p) => p,
            Reveal::SeedShown
            | Reveal::ExtendedPrivateKey
            | Reveal::AddressKey
            | Reveal::KefCode => return None,
            Reveal::Words
            | Reveal::SeedQr
            | Reveal::CopyByHand
            | Reveal::PaperForm
            | Reveal::Bip85Child
            | Reveal::LightningKey
            | Reveal::GpgPaperkey => Part::Words,
        })
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
    text: Source<'a>,
}

/// A revealed text is the store's own (lent), or made for the guard (a
/// vault record's words, written out from its payload) and wiped with it.
enum Source<'a> {
    Lent(&'a str),
    Made(Text),
}

impl<'a> Revealed<'a> {
    /// No text: what a reveal of a field that is not there gives.
    pub fn empty() -> Revealed<'a> {
        Revealed::lent("")
    }

    /// A text lent from the store.
    pub(crate) fn lent(text: &'a str) -> Revealed<'a> {
        Revealed {
            text: Source::Lent(text),
        }
    }

    /// A text made for this guard.
    pub(crate) fn made(text: Text) -> Revealed<'a> {
        Revealed {
            text: Source::Made(text),
        }
    }
}

impl std::ops::Deref for Revealed<'_> {
    type Target = str;
    fn deref(&self) -> &str {
        match &self.text {
            Source::Lent(t) => t,
            Source::Made(t) => t.as_str(),
        }
    }
}

/// The most key texts a key keeps ([`Keys::public_text`]); the next one
/// empties the list.
pub const KEY_TEXTS_KEPT: usize = 64;

/// A key that can sign, held for this session only.
pub(crate) struct Key {
    /// Behind an `Arc` that is never opened for change: a signing job
    /// holds a clone ([`Keys::sign_job`]), and the key is wiped as the last
    /// one drops (`docs/KEYS.md` §2). A change of network makes a new key
    /// (`MasterKey::for_network`); a reblind of a key a job holds is owed until it lets go.
    master: std::sync::Arc<MasterKey>,
    words: Option<Text>,
    passphrase: Option<Text>,
    /// The master seed bytes a key loaded by [`Keys::add_seed`] came
    /// from (codex32, SLIP-39), so that a paper form
    /// of it can be written again; `None` for a key made of words. Wiped
    /// with the key.
    seed: Option<Zeroizing<Vec<u8>>>,
    /// Its public key texts asked for, each derived once
    /// ([`Keys::public_text`]), by what was asked: public, and dropped
    /// when the master key moves network.
    public: std::cell::RefCell<Vec<(Spec, Result<Text, Text>)>>,
    /// Its words' [`WordsId`], once worked out: wiped with the key.
    words_id: std::cell::OnceCell<Option<WordsId>>,
    /// Its master seed's vault record id ([`Keys::seed_record_id`]), once
    /// worked out: a hash kept so a frame does not make it again, in a
    /// wiping type like [`WordsId`]: wiped with the key.
    seed_id: std::cell::OnceCell<Option<WordsId>>,
    /// The strings of its codex32 set this session has seen: made here,
    /// typed back, or read from a vault's share records
    /// ([`Keys::keep_codex32`]). Shares only, never the secret; each of
    /// the set [`KeyInfo::codex32`] names, one per index. Wiped with the
    /// key.
    ///
    /// Invariant: every string kept is of the set the key's facts name
    /// (threshold, identifier, length) at an index not destroyed; a
    /// change of the facts ([`Keys::set_codex32`]) drops those that no
    /// longer are.
    codex32: Vec<Text>,
}

/// A key's secrets as the operations of this crate use them, lent.
pub(crate) struct Secrets<'a> {
    pub(crate) master: &'a MasterKey,
    pub(crate) words: Option<&'a str>,
    pub(crate) passphrase: Option<&'a str>,
    /// The master seed bytes of a key loaded by `add_seed`.
    pub(crate) seed: Option<&'a [u8]>,
    pub(crate) language: Language,
}

pub(crate) struct Entry {
    key: Key,
    info: KeyInfo,
    /// A reblind came while a signing job held the key: it is randomized
    /// again as soon as no job holds it ([`Keys::reblind_owed`]).
    owes_blind: bool,
}

/// One sample New key hands out (`docs/MODES.md` §7): public data.
#[derive(Debug, Clone, Copy)]
pub struct Handout {
    /// The word it is made of, which the Key card names.
    pub name: &'static str,
    /// How many words it has.
    pub words: u8,
    /// Its fingerprint with no passphrase.
    pub plain: Fingerprint,
    /// Its fingerprint with the passphrase `faraday`.
    pub with_passphrase: Fingerprint,
}

/// What the shell gave the demo (F81, `docs/MODES.md` §2.3): public data,
/// kept as given, so nothing here is derived.
struct Demo {
    /// New key's handouts: a name, a word count, a fingerprint.
    handouts: Vec<Handout>,
    /// Every fingerprint the demo takes.
    public: Vec<Fingerprint>,
    /// The Lightning node ids the demo takes.
    nodes: Vec<[u8; 33]>,
}

/// The session's keys, the network every one derives for, and the demo's
/// public seeds when the shell set it (F81).
pub struct Keys {
    entries: Vec<Entry>,
    /// The network every key derives for and every address is shown on.
    network: Network,
    /// The demo's public set, when the shell set it ([`Keys::set_demo`]).
    /// `None`, as on the stick: the session takes any key.
    demo: Option<Demo>,
    /// While a stick is attached, the name the app gives it ("stick",
    /// "SD card"): no key is added (`docs/SECURITY-MODEL.md`). Kept in
    /// step with the sticks the shell reports by the one place they
    /// change ([`Keys::set_stick_attached`]).
    stick: Option<&'static str>,
    /// What every key's signing context is blinded from: the session's
    /// randomness as the app last handed it down ([`Keys::reblind`]), and
    /// how many blinds have been drawn from it, so that no two are the
    /// same. `None` until the app has randomness. Wiped when dropped.
    blind: Option<(Zeroizing<[u8; 32]>, u64)>,
    /// The private key being swept, while its sweep is open
    /// ([`Keys::sweep_read`], F171): one at most, never kept, erased when
    /// it is dropped.
    sweep: Option<crate::sweep::SweepKey>,
    /// The seed My seed makes my wallet holds, while the check is open
    /// ([`Keys::check_read`]): one at most, never one of the keys, erased
    /// when it is dropped.
    pub(crate) check: Option<crate::seed_check::HeldCheck>,
    /// An aezeed read for Convert, while the tool holds it
    /// ([`Keys::aezeed_hold`]): never one of the keys.
    pub(crate) aezeed: Option<crate::lightning::HeldAezeed>,
    /// A SLIP-39 set typed for Convert, while the tool holds it
    /// ([`Keys::slip39_set_hold`]): never one of the keys.
    pub(crate) slip39_set: Option<crate::slip39_set::HeldSet>,
    /// Published keys the person chose to keep for testing
    /// ([`Keys::keep_published`]): public fingerprints.
    kept_published: Vec<Fingerprint>,
    /// Keys derived from a published one (`Keys::note_child`).
    published_children: Vec<Fingerprint>,
    /// Keys derived from a public sample (`Keys::note_child`): Mainnet
    /// refuses them as it refuses the sample.
    sample_children: Vec<Fingerprint>,
    /// Keys a vault says were written by Faraday Demo on an online
    /// computer ([`Keys::note_made_online`]): public fingerprints.
    made_online: Vec<Fingerprint>,
    /// Fingerprints of keys added since [`Keys::take_added`] last said:
    /// public, so the open vaults' marks can be matched to them.
    added: Vec<Fingerprint>,
    /// Keys loaded from a vault ([`Keys::note_from_vault`]): public
    /// fingerprints.
    from_vault: Vec<Fingerprint>,
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
            demo: None,
            stick: None,
            blind: None,
            sweep: None,
            check: None,
            aezeed: None,
            slip39_set: None,
            kept_published: Vec::new(),
            published_children: Vec::new(),
            sample_children: Vec::new(),
            made_online: Vec::new(),
            added: Vec::new(),
            from_vault: Vec::new(),
        }
    }

    /// Costs an elliptic-curve multiplication per key held (a context
    /// randomized). The session's randomness changed (it arrived, or the
    /// person added their own): every key's signing context is randomized
    /// again from `seed`, and each key added after is randomized from it
    /// when it is added (`docs/KEYS.md` §2). Blinding changes no result.
    pub fn reblind(&mut self, c: &Compute, seed: &[u8; 32]) {
        self.blind = Some((Zeroizing::new(*seed), 0));
        for k in 0..self.entries.len() {
            self.blind_key(c, k);
        }
        self.blind_sweep(c);
    }

    /// Costs an elliptic-curve multiplication once the app has handed
    /// randomness down and a key is being swept: its context randomized
    /// from the next blind drawn, as [`Keys::blind_key`] draws one.
    fn blind_sweep(&mut self, c: &Compute) {
        if self.sweep.is_none() {
            return;
        }
        let Some(blind) = self.next_blind() else {
            return;
        };
        if let Some(s) = self.sweep.as_mut() {
            s.key_mut().reblind(c, &blind);
        }
    }

    /// The next blind drawn from the session's randomness: HKDF-Expand
    /// (RFC 5869) from the seed, with `info` "faraday key blind" and the
    /// count (eight bytes, little-endian), 32 bytes; `None` before the
    /// app has randomness.
    fn next_blind(&mut self) -> Option<Zeroizing<[u8; 32]>> {
        const INFO: &[u8] = b"faraday key blind";
        let (seed, count) = self.blind.as_mut()?;
        *count += 1;
        let mut info = [0u8; INFO.len() + 8];
        info[..INFO.len()].copy_from_slice(INFO);
        info[INFO.len()..].copy_from_slice(&count.to_le_bytes());
        let mut blind = Zeroizing::new([0u8; 32]);
        faraday_crypto::hkdf_sha256_expand(&seed[..], &info, &mut blind[..])
            .expect("32 bytes is one block");
        Some(blind)
    }

    /// Costs an elliptic-curve multiplication once the app has handed
    /// randomness down. Key `k`'s context randomized from the next blind
    /// drawn from the session's randomness: HKDF-Expand (RFC 5869) from
    /// the seed, with `info` "faraday key blind" and the count (eight
    /// bytes, little-endian), 32 bytes.
    fn blind_key(&mut self, c: &Compute, k: usize) {
        if self.blind.is_none() || k >= self.entries.len() {
            return;
        }
        // A key a signing job holds keeps the context the job started
        // with: blinding changes no result, and no second copy of the key
        // is made. It owes the reblind, paid by `reblind_owed` once the
        // job's answer is applied or dropped.
        if std::sync::Arc::get_mut(&mut self.entries[k].key.master).is_none() {
            self.entries[k].owes_blind = true;
            return;
        }
        let Some(blind) = self.next_blind() else {
            return;
        };
        if let Some(master) = std::sync::Arc::get_mut(&mut self.entries[k].key.master) {
            master.reblind(c, &blind);
        }
        self.entries[k].owes_blind = false;
    }

    /// Costs an elliptic-curve multiplication per key owing a reblind. Each
    /// key a reblind skipped because a signing job held it is randomized
    /// from the next blind drawn, once the job has let go of it; one still
    /// held stays owing. Called where a signing's answer is applied or
    /// dropped (`docs/WORKERS.md` §6).
    pub fn reblind_owed(&mut self, c: &Compute) {
        for k in 0..self.entries.len() {
            if self.entries[k].owes_blind {
                self.blind_key(c, k);
            }
        }
    }

    /// Whether any key still owes a reblind.
    pub fn owes_reblind(&self) -> bool {
        self.entries.iter().any(|e| e.owes_blind)
    }

    /// Costs what `blind_key` costs. A key joins the store, blinded from
    /// the session's randomness.
    fn push(&mut self, c: &Compute, entry: Entry) {
        self.entries.push(entry);
        self.blind_key(c, self.entries.len() - 1);
        let fp = self.entries[self.entries.len() - 1].info.fingerprint;
        self.added.extend([fp]);
        // A mark a vault gave this fingerprint before the key was held is
        // in `made_online` already, and the same words under another
        // passphrase as a key held and marked were written online too:
        // the mark spreads to every key held with these words, and to the
        // children.
        if self.made_online.contains(&fp) || self.same_words_as_marked(self.entries.len() - 1) {
            self.note_made_online(fp);
        }
    }

    /// Whether the key at `i` has words and another key held, already
    /// marked made online, has the same words.
    fn same_words_as_marked(&self, i: usize) -> bool {
        let Some(w) = self.entries[i].key.words.as_deref() else {
            return false;
        };
        self.entries.iter().enumerate().any(|(j, e)| {
            j != i
                && self.made_online.contains(&e.info.fingerprint)
                && e.key
                    .words
                    .as_deref()
                    .is_some_and(|x| x.split_whitespace().eq(w.split_whitespace()))
        })
    }

    /// The fingerprints of the keys added since this was last called, still
    /// held: the caller matches them against the open vaults' records of
    /// keys written on an online computer.
    pub fn take_added(&mut self) -> Vec<Fingerprint> {
        let added = std::mem::take(&mut self.added);
        added
            .into_iter()
            .filter(|fp| self.position(*fp).is_some())
            .collect()
    }

    /// A stick is attached, called `noun`, or none is (`None`): while one
    /// is, every way a key is added refuses ([`Refusal::StickAttached`]),
    /// before anything is derived.
    pub fn set_stick_attached(&mut self, noun: Option<&'static str>) {
        self.stick = noun;
    }

    /// The noun of the stick attached, if one is.
    pub(crate) fn stick_attached(&self) -> Option<&'static str> {
        self.stick
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
    /// (`docs/SECURITY-MODEL.md` "Stateless").
    pub fn holds_secret(&self) -> bool {
        !self.entries.is_empty()
            || self.sweep.is_some()
            || self.check.is_some()
            || self.aezeed.is_some()
            || self.slip39_set.is_some()
    }

    // ---- a private key swept (F171, `crate::sweep`) ----

    /// Costs what reading a WIF costs (a base58check decode, an
    /// elliptic-curve multiplication and the addresses) and a blind. The
    /// WIF in `text` becomes the key being swept, in place of any other;
    /// refused while a stick is attached, and for the other kind of
    /// network. Nothing is kept beyond the sweep.
    pub fn sweep_read(&mut self, c: &Compute, text: &str) -> Result<(), crate::sweep::Refusal> {
        if self.stick.is_some() {
            return Err(crate::sweep::Refusal::StickAttached);
        }
        self.sweep = Some(crate::sweep::SweepKey::read(c, text, self.network)?);
        self.blind_sweep(c);
        Ok(())
    }

    /// The key being swept, with what it shows; `None` when none is. A
    /// lookup.
    pub fn sweep(&self) -> Option<&crate::sweep::SweepKey> {
        self.sweep.as_ref()
    }

    /// The key being swept is erased: its sweep is done or left.
    pub fn sweep_forget(&mut self) {
        self.sweep = None;
    }

    /// Costs what `faraday_psbt::sweep::sign` costs. Signs the inputs of
    /// `psbt` that spend the key being swept, against the context and
    /// past the holdable dangers in `accepted` (F136); `None` when no key
    /// is being swept.
    pub fn sweep_sign(
        &self,
        c: &Compute,
        psbt: &mut faraday_psbt::Psbt,
        (ctx, accepted): (faraday_psbt::Context<'_>, &[faraday_psbt::WarningKind]),
    ) -> Option<Result<faraday_psbt::SignResult, faraday_psbt::Error>> {
        let s = self.sweep.as_ref()?;
        Some(faraday_psbt::sweep::sign(
            c,
            psbt,
            s.key(),
            &ctx,
            accepted,
            faraday_psbt::Nonce::default(),
        ))
    }

    /// Whether key `i` holds the master seed bytes it was loaded from
    /// (`add_seed`), so a paper form can be made without words. A lookup.
    pub fn holds_seed_bytes(&self, i: usize) -> bool {
        self.entries.get(i).is_some_and(|e| e.key.seed.is_some())
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

    /// The place of the key with id `id`, while it is loaded.
    pub fn place_of(&self, id: KeyId) -> Option<usize> {
        self.entries.iter().position(|e| e.info.id == id)
    }

    /// Whether the key with id `id` is loaded.
    pub fn holds(&self, id: KeyId) -> bool {
        self.place_of(id).is_some()
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

    /// Whether the demo is on: the shell set it up ([`Keys::set_demo`]).
    /// The network does not matter; the shell puts Practice on Testnet.
    pub fn demo(&self) -> bool {
        self.demo.is_some()
    }

    /// The sample New key adds next for a key of `words` words while the
    /// demo is on, as its name and fingerprint: the first handout of that
    /// length not loaded, with a passphrase or without. `None` with the
    /// demo off, as on the stick, and when every handout of that length
    /// is loaded.
    pub fn next_handout(&self, words: u8) -> Option<(&'static str, Fingerprint)> {
        self.demo
            .as_ref()?
            .handouts
            .iter()
            .find(|h| {
                h.words == words
                    && self.position(h.plain).is_none()
                    && self.position(h.with_passphrase).is_none()
            })
            .map(|h| (h.name, h.plain))
    }

    /// Whether the demo would take this key: always, when the shell set
    /// no demo.
    pub fn is_public(&self, fp: Fingerprint) -> bool {
        self.demo.as_ref().is_none_or(|d| d.public.contains(&fp))
    }

    /// Whether the demo would take the Lightning node with this id:
    /// always, when the shell set no demo.
    pub fn is_public_node(&self, id: &[u8; 33]) -> bool {
        self.demo.as_ref().is_none_or(|d| d.nodes.contains(id))
    }

    /// The first key held whose words are published (`crate::published`)
    /// that the person has not chosen to keep, on Mainnet with no demo:
    /// the key a caution waits on, and `None` otherwise. The session still
    /// takes any key; this only says which one to warn about.
    pub fn published_pending(&self) -> Option<Fingerprint> {
        if self.demo() || self.network != Network::Mainnet {
            return None;
        }
        self.entries.iter().map(|e| e.info.fingerprint).find(|fp| {
            self.is_sample(*fp)
                || ((self.is_published(*fp) || self.made_online.contains(fp))
                    && !self.kept_published.contains(fp))
        })
    }

    /// Whether key `fp` is one of Faraday's public samples, or a BIP-85
    /// child of one (known from its parent in this session, or from the
    /// device's short list of the samples' children): on Mainnet it is
    /// refused, whatever way it came in, with no way to keep it.
    pub fn is_sample(&self, fp: Fingerprint) -> bool {
        crate::published::is_sample(fp.0) || self.sample_children.contains(&fp)
    }

    /// Whether the Lightning node with this id is one of Faraday's public
    /// samples on Mainnet with no demo: refused, as a sample seed is.
    pub fn is_sample_node(&self, id: &[u8; 33]) -> bool {
        !self.demo() && self.network == Network::Mainnet && crate::published::is_sample_node(id)
    }

    /// Whether a vault said key `fp` was written on an online computer, or
    /// it came from a key that was ([`Keys::note_child`]). It stays true for
    /// the session once the person has held the key: the caution is
    /// [`Keys::published_pending`], which a hold clears, and the key is
    /// saved to a vault with the mark again.
    pub fn made_online(&self, fp: Fingerprint) -> bool {
        self.made_online.contains(&fp)
    }

    /// Notes that the vault the key `fp` was loaded from says it was
    /// written by Faraday Demo on an online computer.
    ///
    /// Every key held now with the same words takes the mark, under any
    /// passphrase (a key with the same words added later takes it in
    /// `push`), and a fingerprint no key held has is remembered until
    /// Lock: a key added later with it is marked already.
    pub fn note_made_online(&mut self, fp: Fingerprint) {
        self.insert_made_online(fp);
        if let Some(i) = self.position(fp) {
            let same: Vec<Fingerprint> = match self.entries[i].key.words.as_deref() {
                Some(w) => self
                    .entries
                    .iter()
                    .filter(|e| {
                        e.key
                            .words
                            .as_deref()
                            .is_some_and(|x| x.split_whitespace().eq(w.split_whitespace()))
                    })
                    .map(|e| e.info.fingerprint)
                    .collect(),
                None => Vec::new(),
            };
            for f in same {
                self.insert_made_online(f);
            }
        }
        // Every key loaded as a child of it was written there too, however
        // the two were ordered when loaded.
        self.carry_child_marks();
    }

    /// Marks `fp` made online, and nothing else.
    fn insert_made_online(&mut self, fp: Fingerprint) {
        if !self.made_online.contains(&fp) {
            // `extend`, not `push`: see `keep_published`.
            self.made_online.extend([fp]);
        }
    }

    /// Whether the key `fp` has published words: its master fingerprint is
    /// in the table, or it was derived from a key that has.
    fn is_published(&self, fp: Fingerprint) -> bool {
        crate::published::is_published(fp.0) || self.published_children.contains(&fp)
    }

    /// Notes that the key `child` was derived from the key `parent` (a
    /// BIP-85 child's words loaded as a key): anyone who has the parent's
    /// words has the child, so a published parent, kept for testing or not,
    /// makes the child published too, and a parent written on an online
    /// computer makes the child written there too.
    pub fn note_child(&mut self, parent: Fingerprint, child: Fingerprint) {
        if self.is_sample(parent) && !self.sample_children.contains(&child) {
            // `extend`, not `push`: see `keep_published`.
            self.sample_children.extend([child]);
        }
        if self.is_published(parent) && !self.published_children.contains(&child) {
            // `extend`, not `push`: see `keep_published`.
            self.published_children.extend([child]);
        }
        if self.made_online(parent) {
            self.insert_made_online(child);
        }
    }

    /// A vault record of key `fp` says it is a child of `of`: the key takes
    /// that link where it has none (typed words, say, met a record naming
    /// the parent), and gets whatever its parent's marks give it, now or
    /// when the parent is marked later.
    pub(crate) fn take_child_of(
        &mut self,
        fp: Fingerprint,
        of: Option<faraday_vault::child::ChildOf>,
    ) {
        let Some(of) = of else {
            return;
        };
        if let Some(i) = self.position(fp)
            && self.entries[i].info.child_of.is_none()
        {
            self.set_child_of(i, Some(of));
        }
        self.note_child(Fingerprint(of.parent), fp);
        self.carry_child_marks();
    }

    /// Keeps the published key `fp` for testing: it is not warned about
    /// again this session.
    pub fn keep_published(&mut self, fp: Fingerprint) {
        if !self.kept_published.contains(&fp) {
            // `extend`, not `push`: `Keys::push` derives a key, and the
            // crypto inventory would take a call by that name for it.
            self.kept_published.extend([fp]);
        }
    }

    /// Drops the published key `fp`, as the caution's Remove does.
    pub fn remove_published(&mut self, fp: Fingerprint) {
        if let Some(i) = self.position(fp) {
            self.remove(i);
        }
    }

    // ---- adding and forgetting ----

    /// Sets the demo up (F81). A lookup: the fingerprints are
    /// literals the samples crate's tests check. `handouts` are what New
    /// key hands out, `public` every fingerprint the demo takes (a key
    /// that takes a passphrase is there with none and with `faraday`),
    /// `nodes` the Lightning node ids it takes. While the demo is on the
    /// store takes only these.
    pub fn set_demo(&mut self, handouts: &[Handout], public: &[Fingerprint], nodes: &[[u8; 33]]) {
        self.demo = Some(Demo {
            #[expect(
                clippy::disallowed_methods,
                reason = "handouts are names, word counts and fingerprints: public"
            )]
            handouts: handouts.to_vec(),
            #[expect(clippy::disallowed_methods, reason = "public fingerprints")]
            public: public.to_vec(),
            #[expect(
                clippy::disallowed_methods,
                reason = "Lightning node ids are public keys"
            )]
            nodes: nodes.to_vec(),
        });
    }

    /// No key is added while a stick is attached.
    pub(crate) fn no_stick(&self) -> Result<(), Refusal> {
        match self.stick {
            Some(noun) => Err(Refusal::StickAttached(noun)),
            None => Ok(()),
        }
    }

    /// Refuses a key that is not a sample while the demo is on.
    pub(crate) fn allowed(&self, fp: Fingerprint) -> Result<(), Refusal> {
        if self.demo() && !self.is_public(fp) {
            return Err(Refusal::NotAllowed(Text::of(&fp_text(fp))));
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

    /// Costs a parse of the words and what
    /// `add_mnemonic` costs. Adds the key the words and a BIP-39
    /// passphrase give. The same words with another passphrase are
    /// another key. English is read first, then the other lists in
    /// BIP-39's order.
    pub fn add_words_with(
        &mut self,
        c: &Compute,
        words: &str,
        passphrase: &str,
        label: &str,
        wanted: Option<Fingerprint>,
    ) -> Result<Fingerprint, Refusal> {
        let m = parse_words(c, words)?;
        self.add_mnemonic(c, &m, passphrase, label, wanted)
    }

    /// Costs what `to_seed`, `from_seed`, `info_of` and `push` cost. Adds the key a
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
        self.no_stick()?;
        let master = mnemonic_master(c, mnemonic, passphrase, self.network)?;
        let fp = master.fingerprint();
        if let Some(w) = wanted
            && w != fp
        {
            return Err(Refusal::WrongKey {
                wanted: Text::of(&fp_text(w)),
                got: Text::of(&fp_text(fp)),
            });
        }
        self.allowed(fp)?;
        if self.position(fp).is_some() {
            return Err(Refusal::Duplicate(fp));
        }
        let entry = mnemonic_entry(c, master, mnemonic, passphrase, label);
        self.push(c, entry);
        Ok(fp)
    }

    /// Costs what `from_seed_bytes`, `info_of` and `push` cost. Adds the key a
    /// master seed gives, as a SLIP-39 or codex32 key does: there
    /// are no words to copy by hand. The seed bytes are kept with the key,
    /// wiped when it is dropped, so a paper form of it
    /// (`Keys::codex32_shares`) can be written again.
    pub fn add_seed(
        &mut self,
        c: &Compute,
        seed: &[u8],
        label: &str,
    ) -> Result<Fingerprint, Refusal> {
        self.no_stick()?;
        let master = seed_master(c, seed, self.network)?;
        let fp = master.fingerprint();
        self.allowed(fp)?;
        if self.position(fp).is_some() {
            return Err(Refusal::Duplicate(fp));
        }
        let entry = seed_entry(c, master, seed, label);
        self.push(c, entry);
        Ok(fp)
    }

    /// Costs two base58 checks (the text is decoded twice), two
    /// elliptic-curve multiplications and what `push` costs. Adds the key
    /// an extended private key at depth 0 is (`docs/KEY-FORMS.md` §6): it
    /// holds a master key and no words, seed or passphrase. Read by
    /// [`crate::xprv`]'s rules for this session's network.
    pub fn add_xprv(
        &mut self,
        c: &Compute,
        text: &str,
        label: &str,
    ) -> Result<Fingerprint, crate::xprv::XprvError> {
        use crate::xprv::XprvError;
        self.no_stick().map_err(XprvError::Session)?;
        let master = crate::xprv::read_master(c, text, self.network).map_err(XprvError::Read)?;
        let fp = master.fingerprint();
        self.allowed(fp).map_err(XprvError::Session)?;
        if self.position(fp).is_some() {
            return Err(XprvError::Session(Refusal::Duplicate(fp)));
        }
        let label = if label.trim().is_empty() {
            fp_text(fp)
        } else {
            Text::of(label.trim())
        };
        let entry = Entry::new(c, master, label, None, Language::English, None, None, None);
        self.push(c, entry);
        Ok(fp)
    }

    /// Costs what `push` costs for each key. Adds the keys a load's job
    /// worked out ([`crate::job::KeysJob`]), in the order the job made
    /// them, as `add_words_with` and `add_seed` add them: one the session
    /// refuses (a stick attached, not a sample in the demo, already held)
    /// is left out, and so is every key worked out on another network than
    /// the session's. Returns the fingerprints of the keys that joined.
    pub fn adopt_loaded(
        &mut self,
        c: &Compute,
        loaded: crate::job::KeysLoaded,
    ) -> Vec<Fingerprint> {
        self.adopt_loaded_derived(c, loaded).0
    }

    /// Costs what `adopt_loaded` costs. [`Keys::adopt_loaded`], and with the
    /// fingerprints of the keys that joined what each gives at the origin
    /// paths the job was asked for ([`crate::job::KeysJob::check_at`]); a
    /// key left out gives none, and why it was left out is given.
    pub fn adopt_loaded_derived(
        &mut self,
        c: &Compute,
        loaded: crate::job::KeysLoaded,
    ) -> (Vec<Fingerprint>, crate::job::Derived, Vec<Refusal>) {
        let mut out = Vec::new();
        if loaded.network != self.network {
            return (out, crate::job::Derived::default(), Vec::new());
        }
        let mut derived = Vec::new();
        // Why keys were left out: those the job could not make, then those
        // the session refuses.
        let mut refused = loaded.refused;
        for item in loaded.items {
            let fp = item.entry.info.fingerprint;
            if let Err(e) = self.no_stick().and_then(|()| self.allowed(fp)) {
                refused.push(e);
                continue;
            }
            // A child a vault names the parent of: its words are as public
            // as the parent's, however it came to be loaded.
            if let Some(of) = item.child {
                self.note_child(Fingerprint(of.parent), fp);
            }
            if self.position(fp).is_some() {
                self.take_child_of(fp, item.child);
                // The vault's mark counts for the key already held too.
                if item.online {
                    self.note_made_online(fp);
                }
                refused.push(Refusal::Duplicate(fp));
                continue;
            }
            self.push(c, item.entry);
            derived.extend(item.derived);
            let at = self.entries.len() - 1;
            if item.codex32.is_some() {
                self.set_codex32(at, item.codex32);
            }
            if item.child.is_some() {
                self.set_child_of(at, item.child);
            }
            if item.xor.is_some() {
                self.set_xor(at, item.xor);
            }
            if item.slip39.is_some() {
                self.set_slip39(at, item.slip39);
            }
            if item.online {
                self.note_made_online(fp);
            }
            out.push(fp);
        }
        // A child read before its parent in this load missed the parent's
        // marks: carry them on until nothing changes.
        self.carry_child_marks();
        (out, crate::job::Derived(derived), refused)
    }

    /// Notes each recorded child as published or made online when its
    /// parent is, however the two were ordered when loaded.
    fn carry_child_marks(&mut self) {
        loop {
            let before = (
                self.published_children.len(),
                self.sample_children.len(),
                self.made_online.len(),
            );
            let pairs: Vec<(Fingerprint, Fingerprint)> = self
                .entries
                .iter()
                .filter_map(|e| {
                    e.info
                        .child_of
                        .map(|o| (Fingerprint(o.parent), e.info.fingerprint))
                })
                .collect();
            for (parent, child) in pairs {
                self.note_child(parent, child);
            }
            let after = (
                self.published_children.len(),
                self.sample_children.len(),
                self.made_online.len(),
            );
            if before == after {
                break;
            }
        }
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
        #[expect(
            clippy::disallowed_methods,
            reason = "words copied are Zeroizing, wiped on drop"
        )]
        let words = self
            .entries
            .get(i)
            .and_then(|e| e.key.words.clone())
            .ok_or_else(|| Refusal::Words(Text::of("")))?;
        let online = self.made_online(self.entries[i].info.fingerprint);
        let fp = self.add_words_with(c, &words, passphrase, label, None)?;
        // The same words, written on an online computer: so is this key.
        if online {
            self.note_made_online(fp);
        }
        Ok(fp)
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
        #[expect(
            clippy::disallowed_methods,
            reason = "words and passphrase copied are Zeroizing, wiped on drop"
        )]
        let (words, base) = {
            let key = &self
                .entries
                .get(i)
                .ok_or_else(|| Refusal::Words(Text::of("")))?
                .key;
            (
                key.words
                    .clone()
                    .ok_or_else(|| Refusal::Words(Text::of("")))?,
                key.passphrase.clone(),
            )
        };
        let passphrase = text!("{}{suffix}", base.as_deref().unwrap_or(""));
        let lang = self.entries[i].info.language;
        let m = Mnemonic::parse(c, lang, &words)
            .map_err(|e| Refusal::Words(faraday_strings::bip::bip39_error(&e)))?;
        let online = self.made_online(self.entries[i].info.fingerprint);
        let fp = self.add_mnemonic(c, &m, &passphrase, label, None)?;
        // The same words, written on an online computer: so is this key.
        if online {
            self.note_made_online(fp);
        }
        Ok(fp)
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

    /// Drops every key and what is kept per key: what Remove all keys
    /// does. A mark a vault gave a fingerprint, and a key derived from a
    /// published one, stay while the session does ([`Keys::clear`]): a
    /// key typed again after this is marked as it was. Published keys
    /// kept for testing are not kept: the caution comes again. Returns the
    /// fingerprints of the keys dropped, in the list's order.
    pub fn remove_all(&mut self) -> Vec<Fingerprint> {
        let gone = self.entries.iter().map(|e| e.info.fingerprint).collect();
        self.entries.clear();
        self.sweep = None;
        self.check = None;
        self.aezeed = None;
        self.slip39_set = None;
        self.kept_published.clear();
        self.added.clear();
        self.from_vault.clear();
        gone
    }

    /// Drops every key and every mark the session holds: Lock and exit.
    pub fn clear(&mut self) {
        self.remove_all();
        self.published_children.clear();
        self.sample_children.clear();
        self.made_online.clear();
    }

    /// Sets key `i`'s label.
    pub fn set_label(&mut self, i: usize, label: &str) {
        if let Some(e) = self.entries.get_mut(i) {
            e.info.label = Text::of(label);
        }
    }

    /// Moves the store to `network`; every key is re-derived for it. For
    /// a shell's start and for `Session::follow` (an empty session
    /// taking the network of a wallet or a PSBT); no press of a person
    /// reaches it (F81, `docs/MODES.md` §2.7). The caller says whether
    /// wallets are loaded: they stop it moving between networks whose
    /// keys are written differently.
    pub fn set_network(&mut self, network: Network, wallets_loaded: bool) -> Result<(), Refusal> {
        if network == self.network {
            return Ok(());
        }
        if wallets_loaded && network.kind() != self.network.kind() {
            return Err(Refusal::Network {
                offered: network,
                loaded: self.network,
            });
        }
        for e in &mut self.entries {
            e.key.master = std::sync::Arc::new(e.key.master.for_network(network));
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
            words: e.key.words.as_deref(),
            passphrase: e.key.passphrase.as_deref(),
            seed: e.key.seed.as_deref().map(|v| v.as_slice()),
            language: e.info.language,
        })
    }

    // ---- the one door for text ----

    /// The key's words or passphrase as a reason names them, lent while
    /// the guard lives. Empty for a key without them, or no such key.
    pub fn reveal(&self, i: usize, why: Reveal) -> Revealed<'_> {
        let text = self.entries.get(i).and_then(|e| match why.part()? {
            Part::Words | Part::SeedQr => e.key.words.as_deref(),
            Part::Passphrase => e.key.passphrase.as_deref(),
            _ => None,
        });
        Revealed::lent(text.unwrap_or(""))
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
            // A hash of the entropy: compared in constant time.
            (Some(x), Some(y)) => bool::from(x.ct_eq(y)),
            _ => false,
        }
    }

    /// Costs a parse of the words the first time for each key. The places
    /// of the keys that have the words key `i` has in the same list, `i`
    /// included: the same words with and without a passphrase. The same
    /// entropy in another list is another wallet, and is not one of them.
    pub fn with_same_words(&self, c: &Compute, i: usize) -> Vec<usize> {
        let Some(language) = self.entries.get(i).map(|e| e.info.language) else {
            return Vec::new();
        };
        if self.words_id(c, i).is_none() {
            return Vec::new();
        }
        (0..self.entries.len())
            .filter(|&k| self.entries[k].info.language == language && self.same_words(c, i, k))
            .collect()
    }

    /// Whether key `i`'s passphrase is exactly `typed`, compared whole and
    /// in constant time, as subtle's slice comparison is: a passphrase of
    /// another length is refused at once, so the length is not kept secret.
    /// A key that keeps no passphrase matches nothing.
    pub fn passphrase_matches(&self, i: usize, typed: &str) -> bool {
        self.entries
            .get(i)
            .and_then(|e| e.key.passphrase.as_ref())
            .is_some_and(|p| bool::from(p.as_bytes().ct_eq(typed.as_bytes())))
    }

    /// Costs a parse of the words, and a SHA-256 for the copy's own
    /// checksum where a word differs. Compares a copy of key `i`'s words,
    /// scanned or typed, as BIP-39 numbers, with its words
    /// ([`compare_words`]); a copy that is a whole seed of its own, its
    /// checksum holding, with another length or more than half its words
    /// different, is another seed ([`CopyCheck::AnotherSeed`]). Nothing of
    /// either is given back. `None` where the key has no words.
    pub fn words_match(&self, c: &Compute, i: usize, copy: &[u16]) -> Option<CopyCheck> {
        let e = self.entries.get(i)?;
        let m = Mnemonic::parse(c, e.info.language, e.key.words.as_ref()?).ok()?;
        Some(match compare_words(copy, m.indices()) {
            CopyCheck::WrongWord(k) if another_seed(copy, m.indices()) => CopyCheck::AnotherSeed(k),
            found => found,
        })
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
        Some(bool::from(digits.expose().as_bytes().ct_eq(payload)))
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

    /// Costs a parse of the words, a PBKDF2, a key derivation of at most
    /// [`faraday_bip::keys::MAX_PATH_STEPS`] steps and an elliptic-curve
    /// multiplication. The extended public key at `path` below what key
    /// `i`'s words make with no passphrase; `None` for a key with no
    /// words and for a longer path.
    pub fn xpub_without_passphrase_at(
        &self,
        c: &Compute,
        i: usize,
        path: &DerivationPath,
    ) -> Option<faraday_bip::bitcoin::bip32::Xpub> {
        if path.len() > faraday_bip::keys::MAX_PATH_STEPS {
            return None;
        }
        let e = self.entries.get(i)?;
        let m = Mnemonic::parse(c, e.info.language, e.key.words.as_deref()?).ok()?;
        crate::identity::xpub_at(
            c,
            crate::identity::Material::Words(&m),
            "",
            self.network,
            path,
        )
    }

    /// Costs a parse of the words. Key `i`'s vault key record: its words,
    /// or the master seed of a key loaded from one, its label, `flags`
    /// when given, its BIP-39 passphrase when `with_passphrase`, and its
    /// codex32 set's plain facts when it has a set (`docs/CODEX32.md`
    /// §9.4, no string) and its BIP-85 origin when it has one, except the
    /// fields numbered in `without`, which the person left out. `None` for
    /// a key with neither words nor a seed.
    pub fn key_record(
        &self,
        c: &Compute,
        i: usize,
        flags: Option<u8>,
        with_passphrase: bool,
        without: &[u8],
        _why: Reveal,
    ) -> Option<Held<faraday_vault::Record>> {
        use faraday_vault::records::{self, field, kind};
        let e = self.entries.get(i)?;
        let payload = match (e.key.words.as_ref(), e.key.seed.as_ref()) {
            (Some(words), _) => {
                records::words_payload(&Mnemonic::parse(c, e.info.language, words).ok()?)
            }
            (None, Some(seed)) => faraday_vault::layouts::seed_payload(seed)?,
            (None, None) => return None,
        };
        let mut record = faraday_vault::Record::new(kind::KEY)
            .with(field::KEY, &payload)
            .with(field::KEY_LABEL, e.info.label.as_bytes());
        if let Some(f) = flags {
            record.push(field::KEY_FLAGS, &[f]);
        }
        // A BIP-39 passphrase is stored only when the person asks.
        if with_passphrase && let Some(p) = e.key.passphrase.as_ref() {
            record.push(field::KEY_PASSPHRASE, p.as_bytes());
        }
        if !without.contains(&field::KEY_CODEX32)
            && let Some(set) = e
                .info
                .codex32
                .as_ref()
                .and_then(faraday_vault::codex32::set_payload)
        {
            record.push(field::KEY_CODEX32, &set);
        }
        // Recorded as a BIP-85 child: only here, in the vault (F166).
        if !without.contains(&field::KEY_CHILD)
            && let Some(of) = e
                .info
                .child_of
                .as_ref()
                .and_then(faraday_vault::child::payload)
        {
            record.push(field::KEY_CHILD, &of);
        }
        // The plain facts of a Seed XOR or SLIP-39 set: no part, no share.
        if !without.contains(&field::KEY_XOR)
            && let Some(set) = e.info.xor.as_ref().and_then(faraday_vault::xor::payload)
        {
            record.push(field::KEY_XOR, &set);
        }
        if !without.contains(&field::KEY_SLIP39)
            && let Some(set) = e
                .info
                .slip39
                .as_ref()
                .and_then(faraday_vault::slip39::payload)
        {
            record.push(field::KEY_SLIP39, &set);
        }
        Some(Held(record))
    }

    /// Costs a parse of the words, a hash and an elliptic-curve
    /// multiplication; no PBKDF2, the master key being held. The id,
    /// fingerprint and [`master_id`] of the vault key record
    /// [`Session::key_record`] makes of key `i` with or without its
    /// passphrase, so that what a vault keeps of the record is known at
    /// the save and no stretch is left for the next press on a vault.
    /// `None` for a key with no record, and for a record of the words
    /// alone from a key that has a passphrase: its master key is another
    /// one, not held.
    pub fn saved_identity(
        &self,
        c: &Compute,
        i: usize,
        with_passphrase: bool,
    ) -> Option<([u8; 32], Fingerprint, [u8; 32])> {
        let e = self.entries.get(i)?;
        if !with_passphrase && e.key.passphrase.is_some() {
            return None;
        }
        let record = self.key_record(
            c,
            i,
            None,
            with_passphrase,
            &[],
            Reveal::VaultEntry(Part::Words),
        )?;
        let id = crate::vault::key_record_id(crate::vault::RecordView::of(&record.0))?;
        Some((id, e.key.master.fingerprint(), master_id(c, &e.key.master)))
    }

    /// Costs a hash. The id ([`crate::vault::key_record_id`]) a vault key
    /// record of key `i`'s master seed has, for a key loaded from one (a
    /// codex32 or SLIP-39 key): what tells a vault holds it
    /// (`docs/CODEX32.md` §9.6). `None` for a key with words.
    pub fn seed_record_id(&self, i: usize) -> Option<[u8; 32]> {
        use faraday_vault::records::{field, kind};
        let e = self.entries.get(i).filter(|e| e.key.words.is_none())?;
        let payload = faraday_vault::layouts::seed_payload(e.key.seed.as_ref()?)?;
        let r = faraday_vault::Record::new(kind::KEY).with(field::KEY, &payload);
        crate::vault::key_record_id(crate::vault::RecordView::of(&r))
    }

    /// Costs a hash the first time. [`Keys::seed_record_id`], worked out
    /// once and kept on the key (a [`WordsId`], wiped with the key), for
    /// a frame that asks every time.
    pub fn seed_record_id_kept(&self, i: usize) -> Option<&[u8; 32]> {
        let e = self.entries.get(i).filter(|e| e.key.words.is_none())?;
        e.key
            .seed_id
            .get_or_init(|| self.seed_record_id(i).map(Zeroizing::new))
            .as_deref()
    }

    /// Sets what is known of key `i`'s codex32 set, or that none is
    /// known. A lookup; refused (false) for no such key or facts the
    /// vault's layout cannot hold.
    pub fn set_codex32(&mut self, i: usize, set: Option<faraday_vault::codex32::SetFacts>) -> bool {
        let sound = set
            .as_ref()
            .is_none_or(faraday_vault::codex32::SetFacts::is_sound);
        match self.entries.get_mut(i) {
            Some(e) if sound => {
                e.info.codex32 = set;
                let facts = e.info.codex32.as_ref();
                e.key
                    .codex32
                    .retain(|s| facts.is_some_and(|f| of_set(f, s)));
                true
            }
            _ => false,
        }
    }

    /// Sets the BIP-85 key that key `i` is a child seed of, or that it is
    /// none (F166). A lookup; refused (false) for no such key, a key
    /// without words, or a record whose word count is not the key's.
    pub fn set_child_of(&mut self, i: usize, of: Option<faraday_vault::child::ChildOf>) -> bool {
        match self.entries.get_mut(i) {
            Some(e)
                if of.is_none_or(|o| {
                    o.is_sound() && e.key.words.is_some() && e.info.word_count == Some(o.words)
                }) =>
            {
                e.info.child_of = of;
                // A child's words are as public as its parent's, whichever
                // way the record reached the session.
                if let Some(o) = of {
                    let child = e.info.fingerprint;
                    self.note_child(Fingerprint(o.parent), child);
                    self.carry_child_marks();
                }
                true
            }
            _ => false,
        }
    }

    /// Sets the Seed XOR set key `i`'s seed is split into, its plain
    /// facts. One set per key: a lookup; refused (false) for no such key,
    /// facts the vault's layout cannot hold, or a key that already has a
    /// set (`None` takes it off).
    pub fn set_xor(&mut self, i: usize, set: Option<faraday_vault::xor::XorFacts>) -> bool {
        match self.entries.get_mut(i) {
            Some(e)
                if set
                    .as_ref()
                    .is_none_or(faraday_vault::xor::XorFacts::is_sound)
                    && (set.is_none() || e.info.xor.is_none()) =>
            {
                e.info.xor = set;
                true
            }
            _ => false,
        }
    }

    /// Replaces the Seed XOR set key `i` has with `set` (**Make new
    /// parts**, `docs/decisions/F285.md`): public facts only, the new
    /// parts' fingerprints, never a word. A lookup; refused (false) for
    /// no such key, a key with no set (that is [`Keys::set_xor`]), or
    /// facts the vault's layout cannot hold.
    pub fn replace_xor(&mut self, i: usize, set: faraday_vault::xor::XorFacts) -> bool {
        match self.entries.get_mut(i) {
            Some(e) if e.info.xor.is_some() && set.is_sound() => {
                e.info.xor = Some(set);
                true
            }
            _ => false,
        }
    }

    /// Sets the SLIP-39 set key `i`'s seed is split into, its plain
    /// facts. One set per key: a lookup; refused (false) for no such key,
    /// facts the vault's layout cannot hold, or a key that already has a
    /// set (`None` takes it off).
    pub fn set_slip39(
        &mut self,
        i: usize,
        set: Option<faraday_vault::slip39::Slip39Facts>,
    ) -> bool {
        match self.entries.get_mut(i) {
            Some(e)
                if set
                    .as_ref()
                    .is_none_or(faraday_vault::slip39::Slip39Facts::is_sound)
                    && (set.is_none() || e.info.slip39.is_none()) =>
            {
                e.info.slip39 = set;
                true
            }
            _ => false,
        }
    }

    /// Replaces the SLIP-39 set key `i` has with `set` (Convert's Rotate
    /// the set, `docs/decisions/F293.md`): public facts only. A lookup;
    /// refused (false) for no such key, a key with no set (that is
    /// [`Keys::set_slip39`]), or facts the vault's layout cannot hold.
    pub fn replace_slip39(&mut self, i: usize, set: faraday_vault::slip39::Slip39Facts) -> bool {
        match self.entries.get_mut(i) {
            Some(e) if e.info.slip39.is_some() && set.is_sound() => {
                e.info.slip39 = Some(set);
                true
            }
            _ => false,
        }
    }

    /// Costs a parse of each string and, for each one not yet proved, what
    /// `codex32_find` costs: for each of at most 16 choices of strings, an
    /// interpolation, an HMAC-SHA512 and an elliptic-curve multiplication.
    /// Keeps each of `strings` that is proved a share of key `i`'s seed
    /// (`docs/CODEX32.md` §9.5): its threshold, identifier and length the
    /// set's, at an index not destroyed, its checksum holding, and `k`
    /// distinct indices among it and the strings already kept or given
    /// rebuild a seed with the key's fingerprint; every string of that
    /// combination is kept. A string that cannot be proved, as when fewer
    /// than `k` indices are here, is not kept. One at an index already
    /// kept replaces it. The secret is never kept: it is the seed.
    /// Returns how many of `strings` were kept.
    pub fn keep_codex32(&mut self, c: &Compute, i: usize, strings: &[&str]) -> usize {
        self.keep_codex32_among(c, i, strings, &[])
    }

    /// Costs what [`Keys::keep_codex32`] costs. As `keep_codex32`, with
    /// `more` (an open vault's strings of the key's seed) also among the
    /// strings a given one is proved with: those of the set prove, and are
    /// kept only where they are part of the combination that proves it. A
    /// string not proved is still not kept.
    pub(crate) fn keep_codex32_among(
        &mut self,
        c: &Compute,
        i: usize,
        strings: &[&str],
        more: &[Text],
    ) -> usize {
        let Some(e) = self.entries.get(i) else {
            return 0;
        };
        let Some(facts) = e.info.codex32.as_ref() else {
            return 0;
        };
        let fp = e.info.fingerprint.0;
        let need = usize::from(facts.threshold);
        #[expect(
            clippy::disallowed_methods,
            reason = "a codex32 set's facts are public"
        )]
        let facts = facts.clone();
        let facts = &facts;
        // A string at a destroyed index, of another set or of another
        // length is neither a proof nor kept, whichever vault it is from.
        let more: Vec<&Text> = more
            .iter()
            .filter(|t| of_set(facts, &crate::texts::lower(t)))
            .collect();
        let mut cands: Vec<Text> = Vec::new();
        for s in strings {
            let lower = crate::texts::lower(s);
            if of_set(facts, &lower) && !cands.iter().any(|x| **x == *lower) {
                cands.push(lower);
            }
        }
        let mut kept = 0;
        for s in &cands {
            let Some(e) = self.entries.get_mut(i) else {
                return kept;
            };
            if !e.key.codex32.iter().any(|x| **x == **s) {
                let found = {
                    let pool: Vec<&str> = e
                        .key
                        .codex32
                        .iter()
                        .chain(cands.iter().filter(|x| ***x != **s))
                        .chain(more.iter().copied())
                        .map(|t| t.as_str())
                        .collect();
                    crate::vault::codex32_find(c, fp, need, &[s.as_str()], &pool)
                };
                let Some(Some((_, texts))) = found else {
                    continue;
                };
                for t in texts {
                    if !of_set(facts, &crate::texts::lower(&t)) {
                        continue;
                    }
                    e.key
                        .codex32
                        .retain(|x| x.as_bytes().get(8) != t.as_bytes().get(8));
                    e.key.codex32.push(t);
                }
            }
            kept += 1;
        }
        kept
    }

    /// Costs a parse of each string and, where `k` are here, for each of
    /// at most 16 choices of the strings an index has two of, an
    /// interpolation, an HMAC-SHA512 and an elliptic-curve multiplication.
    /// Whether `string`, a share whose header is that of key `i`'s codex32
    /// set, is of key `i`'s seed (`docs/CODEX32.md` §9.5): with the strings
    /// of its set at other indices this session holds for the key, then
    /// those of `more` (an open vault's), `k` strings rebuild a seed whose
    /// fingerprint is key `i`'s. `None` where fewer than `k` are here, or
    /// for no such key.
    pub(crate) fn codex32_of_seed(
        &self,
        c: &Compute,
        i: usize,
        string: &faraday_bip::codex32::Codex32,
        more: &[Text],
    ) -> Option<bool> {
        let e = self.entries.get(i)?;
        let need = usize::from(string.threshold());
        if need < 2 || string.is_secret() {
            return None;
        }
        let typed = crate::texts::wiped(string.encode().as_str());
        // As in `keep_codex32_among`: a string at a destroyed index, of
        // another set or of another length decides nothing.
        let facts = e.info.codex32.as_ref();
        let pool: Vec<&str> = e
            .key
            .codex32
            .iter()
            .chain(
                more.iter()
                    .filter(|t| facts.is_none_or(|f| of_set(f, &crate::texts::lower(t)))),
            )
            .map(|t| t.as_str())
            .collect();
        let found =
            crate::vault::codex32_find(c, e.info.fingerprint.0, need, &[typed.as_str()], &pool)?;
        Some(found.is_some())
    }

    /// Whether this session holds the string at share index `index` (a
    /// bech32 character, lower case) of key `i`'s codex32 set. A lookup.
    pub fn holds_codex32(&self, i: usize, index: u8) -> bool {
        self.entries.get(i).is_some_and(|e| {
            e.key
                .codex32
                .iter()
                .any(|s| s.as_bytes().get(8) == Some(&index))
        })
    }

    /// The string at share index `index` of key `i`'s codex32 set, lent
    /// for a file of it on a stick (`Reveal::BackupStick(Part::Codex32)`).
    /// `None` for another reason, or a string this session does not hold.
    pub fn codex32_string(&self, i: usize, index: u8, why: Reveal) -> Option<Revealed<'_>> {
        if why != Reveal::BackupStick(Part::Codex32) {
            return None;
        }
        self.entries
            .get(i)?
            .key
            .codex32
            .iter()
            .find(|s| s.as_bytes().get(8) == Some(&index))
            .map(|s| Revealed::lent(s.as_str()))
    }

    /// Whether `payload`, what a Codex32QR's codes hold, is the string at
    /// share index `index` of key `i`'s codex32 set, in either case. `None`
    /// where this session does not hold that string. A comparison.
    pub fn codex32_reads_as(&self, i: usize, index: u8, payload: &[u8]) -> Option<bool> {
        let s = self
            .entries
            .get(i)?
            .key
            .codex32
            .iter()
            .find(|s| s.as_bytes().get(8) == Some(&index))?;
        Some(s.as_bytes().eq_ignore_ascii_case(payload))
    }

    /// Key `i`'s codex32 share record (`docs/VAULT.md` §7 type 15) of the
    /// string at share index `index`: the string and the key's
    /// fingerprint, for a vault (`Reveal::VaultEntry(Part::Codex32)`).
    /// `None` for another reason, or a string this session does not hold.
    pub fn codex32_share_record(
        &self,
        i: usize,
        index: u8,
        online: bool,
        why: Reveal,
    ) -> Option<Held<faraday_vault::Record>> {
        use faraday_vault::records::{field, kind};
        if why != Reveal::VaultEntry(Part::Codex32) {
            return None;
        }
        let e = self.entries.get(i)?;
        let s = e
            .key
            .codex32
            .iter()
            .find(|s| s.as_bytes().get(8) == Some(&index))?;
        let mut r = faraday_vault::Record::new(kind::CODEX32_SHARE)
            .with(field::CODEX32_STRING, s.as_bytes())
            .with(field::CODEX32_KEY, &e.info.fingerprint.0);
        // Written on an online computer, or from a key that was: the
        // share says so, as the key's record does.
        if online || self.made_online(e.info.fingerprint) {
            faraday_vault::records::mark_made_online(&mut r);
        }
        Some(Held(r))
    }

    // ---- shown on Explore, each behind its eye ----

    /// Key `i`'s master seed as Explore shows it (`Reveal::SeedShown`):
    /// the seed in lower-case hex; then, where its codex32 set is known,
    /// the set's secret when this session holds enough of the set's
    /// strings to recover it (BIP 93: the secret as it was written, its
    /// padding included, which the seed alone does not give), and the
    /// strings it holds, by share index. A codex32 recovery and a checksum
    /// per string. `None` for another reason, a key with words (they are
    /// lent by [`Keys::reveal`]) or no such key.
    pub fn seed_shown(&self, i: usize, why: Reveal) -> Option<Held<Vec<Text>>> {
        use faraday_bip::codex32::Codex32;
        if why != Reveal::SeedShown {
            return None;
        }
        let e = self.entries.get(i).filter(|e| e.key.words.is_none())?;
        let mut out = vec![faraday_crypto::hex::encode_secret(e.key.seed.as_ref()?)];
        let Some(facts) = e.info.codex32.as_ref() else {
            return Some(Held(out));
        };
        let mut held: Vec<&Text> = e.key.codex32.iter().collect();
        held.sort_by_key(|s| s.as_bytes().get(8).copied());
        let parsed: Vec<Codex32> = held
            .iter()
            .take(usize::from(facts.threshold))
            .filter_map(|s| Codex32::parse(s).ok())
            .collect();
        if facts.threshold >= 2
            && parsed.len() == usize::from(facts.threshold)
            && let Ok(secret) = Codex32::recover(&parsed)
        {
            out.push(crate::texts::wiped(secret.encode().as_str()));
        }
        out.extend(held.into_iter().cloned());
        Some(Held(out))
    }

    /// Costs a parse of the words. Key `i`'s seed entropy for a Krux
    /// encrypted code (`Reveal::KefCode`, `docs/KEF.md` §6): 16 bytes for
    /// 12 words, 32 for 24. `None` for another reason, for a seed of 15,
    /// 18 or 21 words, for another list's words, for a key with no words
    /// (Krux's loader takes 12 or 24 words of the English list) or no such
    /// key.
    pub fn kef_entropy(
        &self,
        c: &Compute,
        i: usize,
        why: Reveal,
    ) -> Option<Held<Zeroizing<Vec<u8>>>> {
        if why != Reveal::KefCode {
            return None;
        }
        let e = self.entries.get(i)?;
        if e.info.language != Language::English {
            return None;
        }
        let m = Mnemonic::parse(c, e.info.language, e.key.words.as_deref()?).ok()?;
        if !matches!(m.word_count(), 12 | 24) {
            return None;
        }
        let entropy = m.entropy();
        Some(Held::of(crate::texts::wiped_bytes(
            entropy.expose().as_bytes(),
        )))
    }

    /// Costs a parse of the words. Whether `plaintext` is key `i`'s seed
    /// entropy as a Krux encrypted code holds it (`Reveal::KefCode`,
    /// `docs/KEF.md` §15.1): a yes or a no, and the entropy goes no
    /// further. `false` for another reason, for a seed with no entropy to
    /// give (`kef_entropy`) or no such key.
    pub fn kef_matches(&self, c: &Compute, i: usize, why: Reveal, plaintext: &[u8]) -> bool {
        self.kef_entropy(c, i, why)
            .is_some_and(|held| held.release().as_slice() == plaintext)
    }

    /// Costs a key derivation. Key `i`'s extended private key at `path`
    /// below its master (the empty path: the root's), as text (`xprv…` on
    /// mainnet, `tprv…` on the test networks), for Explore to show
    /// (`Reveal::ExtendedPrivateKey`). `None` for another reason or no
    /// such key.
    pub fn xprv_at(
        &self,
        c: &Compute,
        i: usize,
        path: &DerivationPath,
        why: Reveal,
    ) -> Option<Held<XprivAscii>> {
        if why != Reveal::ExtendedPrivateKey {
            return None;
        }
        let master = &self.entries.get(i)?.key.master;
        Some(Held(master.derive(c, path).xpriv_ascii()))
    }

    /// Costs a key derivation. The private key at `path` below key `i`'s
    /// master, as compressed WIF on the store's network, for Explore to
    /// show beside its address (`Reveal::AddressKey`). `None` for another
    /// reason or no such key.
    pub fn wif_at(
        &self,
        c: &Compute,
        i: usize,
        path: &DerivationPath,
        why: Reveal,
    ) -> Option<Held<WifAscii>> {
        if why != Reveal::AddressKey {
            return None;
        }
        let key = self.entries.get(i)?.key.master.derive(c, path);
        Some(Held(WifAscii::new(key.secret_key(), self.network)))
    }

    /// Costs, for a key with a passphrase, a parse of the words, a PBKDF2
    /// and an elliptic-curve multiplication. The fingerprint of key `i`'s
    /// words alone, without its BIP-39 passphrase: the id of a Krux code of
    /// the words (`docs/KEF.md` §6). The key's own where it has no
    /// passphrase; `None` for a key with no words or words that do not
    /// parse, never the fingerprint with the passphrase.
    pub fn words_fingerprint(&self, c: &Compute, i: usize) -> Option<Fingerprint> {
        let e = self.entries.get(i)?;
        let words = e.key.words.as_ref()?;
        if !e.info.has_passphrase {
            return Some(e.info.fingerprint);
        }
        let m = Mnemonic::parse(c, e.info.language, words).ok()?;
        crate::identity::fingerprint_of_mnemonic(c, &m, "").ok()
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
    pub fn key_text(&self, c: &Compute, i: usize, spec: &Spec) -> Option<Result<Text, Text>> {
        let key = &self.entries.get(i)?.key;
        if let Some((_, t)) = key.public.borrow().iter().find(|(a, _)| a == spec) {
            #[expect(clippy::disallowed_methods, reason = "a key text is public")]
            return Some(t.clone());
        }
        let t = crate::texts::key_text(c, &key.master, spec);
        let mut kept = key.public.borrow_mut();
        if kept.len() >= KEY_TEXTS_KEPT {
            kept.clear();
        }
        #[expect(
            clippy::disallowed_methods,
            reason = "a key text and its spec are public"
        )]
        kept.push((spec.clone(), t.clone()));
        Some(t)
    }

    /// Costs a key derivation. The multisig account key of key `i` for
    /// `script` at `account`, on BIP-87's path when `bip87`: public.
    pub fn multisig_account(
        &self,
        c: &Compute,
        i: usize,
        script: faraday_bip::keys::MultisigScriptType,
        account: u32,
        bip87: bool,
    ) -> Option<Result<crate::texts::AccountKey, Text>> {
        Some(crate::texts::multisig_account(
            c,
            &self.entries.get(i)?.key.master,
            script,
            account,
            bip87,
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
        bip87: bool,
        description: &str,
    ) -> Option<Text> {
        crate::texts::bsms_record(
            c,
            &self.entries.get(i)?.key.master,
            script,
            account,
            bip87,
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

    /// Whether key `i` holds its words (a key from a seed holds none).
    pub fn has_words(&self, i: usize) -> bool {
        self.entries.get(i).is_some_and(|e| e.key.words.is_some())
    }

    /// Costs a parse of the words. Key `i`'s words as a vault key
    /// record's payload, in wiped memory; `None` for a key with no words.
    pub(crate) fn words_payload(
        &self,
        c: &Compute,
        i: usize,
    ) -> Option<zeroize::Zeroizing<Vec<u8>>> {
        let e = self.entries.get(i)?;
        let words = e.key.words.as_ref()?;
        let m = Mnemonic::parse(c, e.info.language, words).ok()?;
        Some(faraday_vault::records::words_payload(&m))
    }

    /// Costs a parse of the words. Whether key `i`'s words, as a vault key
    /// record's payload, are among `held`.
    pub fn words_payload_among(&self, c: &Compute, i: usize, held: &[&[u8]]) -> bool {
        self.words_payload(c, i)
            .is_some_and(|p| held.contains(&p.as_slice()))
    }

    // ---- signing and inspecting ----

    /// Costs a key derivation per key. The keys as the PSBT crate refers to
    /// them.
    pub fn key_refs(&self, c: &Compute) -> Vec<faraday_psbt::KeyRef> {
        let masters: Vec<&MasterKey> = self.entries.iter().map(|e| &*e.key.master).collect();
        crate::psbt::key_refs(c, &masters)
    }

    /// Costs what `deterministic` costs. The signatures `psbt` carries that
    /// a key held here recomputes, and by which nonce rule.
    pub fn recompute_signatures(
        &self,
        c: &Compute,
        psbt: &faraday_psbt::Psbt,
    ) -> Vec<faraday_psbt::verify::Determinism> {
        let masters: Vec<&MasterKey> = self.entries.iter().map(|e| &*e.key.master).collect();
        crate::psbt::recompute_signatures(c, psbt, &masters)
    }

    /// Costs what `deterministic_all` costs. As [`Self::recompute_signatures`],
    /// with a result also for a signature whose hash cannot be computed.
    pub fn recompute_signatures_all(
        &self,
        c: &Compute,
        psbt: &faraday_psbt::Psbt,
    ) -> Vec<faraday_psbt::verify::Determinism> {
        let masters: Vec<&MasterKey> = self.entries.iter().map(|e| &*e.key.master).collect();
        crate::psbt::recompute_signatures_all(c, psbt, &masters)
    }

    /// A job that signs `psbt` with the keys in `selection`, against the
    /// `context` (network, key references, policies), past the holdable dangers in `accepted` (F136). It holds a clone of
    /// each selected key's `Arc`, not a copy of the key; a key forgotten or
    /// a lock while it runs leaves the key alive until the job ends.
    pub fn sign_job(
        &self,
        psbt: faraday_psbt::Psbt,
        selection: Vec<Fingerprint>,
        context: (
            Network,
            Vec<faraday_psbt::KeyRef>,
            Vec<faraday_bip::policy::WalletPolicy>,
        ),
        (accepted, musig, seed): (
            Vec<faraday_psbt::WarningKind>,
            Option<faraday_psbt::MusigSession>,
            Option<Zeroizing<[u8; 32]>>,
        ),
    ) -> crate::job::SignJob {
        #[expect(
            clippy::disallowed_methods,
            reason = "an Arc count, not a copy of the key"
        )]
        let masters = self
            .entries
            .iter()
            .filter(|e| selection.contains(&e.info.fingerprint))
            .map(|e| std::sync::Arc::clone(&e.key.master))
            .collect();
        crate::job::SignJob::new(psbt, masters, selection, context, (accepted, musig, seed))
    }

    /// A store holding every key's master and public facts and nothing
    /// else, for work off the main thread that derives from keys (an
    /// inspection, a nonce check): the masters behind the same `Arc` a
    /// signing job holds, never a copy. No words, passphrase, seed, codex32
    /// string, text kept, blind, sweep or demo set comes with it.
    #[expect(
        clippy::disallowed_methods,
        reason = "an Arc count and public facts, not a copy of a key"
    )]
    pub fn job_view(&self) -> Keys {
        let mut view = Keys::on(self.network);
        view.entries = self
            .entries
            .iter()
            .map(|e| Entry {
                key: Key {
                    master: std::sync::Arc::clone(&e.key.master),
                    words: None,
                    passphrase: None,
                    seed: None,
                    public: std::cell::RefCell::new(Vec::new()),
                    words_id: std::cell::OnceCell::new(),
                    seed_id: std::cell::OnceCell::new(),
                    codex32: Vec::new(),
                },
                info: e.info.clone(),
                owes_blind: false,
            })
            .collect();
        view
    }
}

impl Keys {
    /// A store holding, for a job that compares public keys with a seed's
    /// (Compare in My seed makes my wallet), only that seed: the check's
    /// (`None`) or key `loaded`'s, as key 0 of the store. Its master is
    /// behind the same `Arc`, never a copy; its words are copied for the
    /// job's time, wiped as the copy drops, so that the key the same words
    /// make with no passphrase can be worked out there. `None` for a key
    /// that is not there.
    #[expect(
        clippy::disallowed_methods,
        reason = "an Arc count, and the words for the job's time only"
    )]
    pub fn compare_view(&self, loaded: Option<usize>) -> Option<Keys> {
        let mut view = Keys::on(self.network);
        match loaded {
            None => view.check = Some(self.check.as_ref()?.job_copy()),
            Some(i) => {
                let e = self.entries.get(i)?;
                view.entries = vec![Entry {
                    key: Key {
                        master: std::sync::Arc::clone(&e.key.master),
                        words: e.key.words.clone(),
                        passphrase: None,
                        seed: None,
                        public: std::cell::RefCell::new(Vec::new()),
                        words_id: std::cell::OnceCell::new(),
                        seed_id: std::cell::OnceCell::new(),
                        codex32: Vec::new(),
                    },
                    info: e.info.clone(),
                    owes_blind: false,
                }];
            }
        }
        Some(view)
    }
}

/// Costs a parse of the words. The mnemonic `words` are, in English if
/// they read in it, else in the first of the other lists in BIP-39's
/// order that reads them.
pub(crate) fn parse_words(c: &Compute, words: &str) -> Result<Mnemonic, Refusal> {
    let normal = crate::texts::normal_words(words);
    let mut first = None;
    for lang in Language::ALL {
        match Mnemonic::parse(c, lang, &normal) {
            Ok(m) => return Ok(m),
            Err(e) => {
                first.get_or_insert(e);
            }
        }
    }
    Err(Refusal::Words(
        first
            .map(|e| faraday_strings::bip::bip39_error(&e))
            .unwrap_or_default(),
    ))
}

/// Costs a PBKDF2 (`to_seed`) and what `from_seed` costs. The master key
/// a mnemonic and a BIP-39 passphrase give on `network`.
pub(crate) fn mnemonic_master(
    c: &Compute,
    mnemonic: &Mnemonic,
    passphrase: &str,
    network: Network,
) -> Result<MasterKey, Refusal> {
    let seed = mnemonic
        .to_seed(c, passphrase.as_bytes())
        .map_err(|e| Refusal::Words(faraday_strings::bip::bip39_error(&e)))?;
    Ok(MasterKey::from_seed(c, &seed, network))
}

/// Costs what `Entry::new` costs. The key `master` is, made of the
/// mnemonic and passphrase it came from, labelled `label` (its
/// fingerprint when the label is empty).
pub(crate) fn mnemonic_entry(
    c: &Compute,
    master: MasterKey,
    mnemonic: &Mnemonic,
    passphrase: &str,
    label: &str,
) -> Entry {
    let lang = mnemonic.language();
    let label = if label.trim().is_empty() {
        fp_text(master.fingerprint())
    } else {
        Text::of(label.trim())
    };
    let mut words = crate::texts::room();
    for (k, &i) in mnemonic.indices().iter().enumerate() {
        if k > 0 {
            words.push(' ');
        }
        words.push_str(lang.word(i));
    }
    let word_count = u8::try_from(mnemonic.word_count()).ok();
    Entry::new(
        c,
        master,
        label,
        Some(words),
        lang,
        word_count,
        (!passphrase.is_empty()).then(|| crate::texts::wiped(passphrase)),
        None,
    )
}

/// Costs what `from_seed_bytes` costs. The master key a master seed gives
/// on `network`; a refusal for a seed of no length a master seed has.
pub(crate) fn seed_master(
    c: &Compute,
    seed: &[u8],
    network: Network,
) -> Result<MasterKey, Refusal> {
    let bytes = faraday_crypto::SeedBytes::new(seed)
        .ok_or_else(|| Refusal::Words(Text::of(faraday_strings::EN.wallet_a_master_seed)))?;
    Ok(MasterKey::from_seed_bytes(
        c,
        &faraday_crypto::Secret::new(bytes),
        network,
    ))
}

/// Costs what `Entry::new` costs. The key `master` is, made of the master
/// seed `seed`, labelled `label` (its fingerprint when the label is empty).
pub(crate) fn seed_entry(c: &Compute, master: MasterKey, seed: &[u8], label: &str) -> Entry {
    let label = if label.trim().is_empty() {
        fp_text(master.fingerprint())
    } else {
        Text::of(label.trim())
    };
    Entry::new(
        c,
        master,
        label,
        None,
        Language::English,
        None,
        None,
        Some(crate::texts::wiped_bytes(seed)),
    )
}

impl Entry {
    #[allow(
        clippy::too_many_arguments,
        reason = "a session's master key, label, words, language, word count, passphrase and the rest of what it holds"
    )]
    /// Costs an elliptic-curve multiplication with the master private
    /// key, for its id. Makes a key and its [`KeyInfo`].
    fn new(
        c: &Compute,
        master: MasterKey,
        label: Text,
        words: Option<Text>,
        language: Language,
        word_count: Option<u8>,
        passphrase: Option<Text>,
        seed: Option<Zeroizing<Vec<u8>>>,
    ) -> Entry {
        let info = KeyInfo {
            id: KeyId(master_id(c, &master)),
            fingerprint: master.fingerprint(),
            label,
            language,
            word_count,
            seed_len: seed.as_ref().map(|b| b.len()),
            has_passphrase: passphrase.is_some(),
            codex32: None,
            child_of: None,
            xor: None,
            slip39: None,
        };
        Entry {
            key: Key {
                master: std::sync::Arc::new(master),
                words,
                passphrase,
                seed,
                public: std::cell::RefCell::new(Vec::new()),
                words_id: std::cell::OnceCell::new(),
                seed_id: std::cell::OnceCell::new(),
                codex32: Vec::new(),
            },
            info,
            owes_blind: false,
        }
    }
}

/// What comparing a copy of a seed with the seed says.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(
    clippy::disallowed_methods,
    reason = "a derived Clone: no secret type derives it"
)]
pub enum CopyCheck {
    /// Every word matches.
    Matches,
    /// A word differs, counting from one.
    WrongWord(usize),
    /// The copy is another seed (`Keys::words_match`): a whole BIP-39
    /// seed whose own checksum holds, of another length or with more than
    /// half its words different. The first that differs, counting from
    /// one, as [`CopyCheck::WrongWord`] would give it.
    AnotherSeed(usize),
}

/// A SHA-256. Whether `copy`, which differs from `seed`, is another seed
/// rather than a copy of it with a word wrong: its own BIP-39 checksum
/// holds (the numbers are the same in every list), and its length is
/// another or more than half its words differ.
fn another_seed(copy: &[u16], seed: &[u16]) -> bool {
    if Mnemonic::from_indices(Language::English, copy).is_err() {
        return false;
    }
    let differ = copy.iter().zip(seed).filter(|(a, b)| a != b).count();
    copy.len() != seed.len() || 2 * differ > copy.len()
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

/// Whether `s`, lower case, is a share of the set `facts` names: its
/// checksum holding, its threshold, identifier and length the set's, not
/// the secret, at an index not destroyed. A parse.
fn of_set(facts: &faraday_vault::codex32::SetFacts, s: &str) -> bool {
    use faraday_vault::codex32::State;
    let Ok(c) = faraday_bip::codex32::Codex32::parse(s) else {
        return false;
    };
    let index = c.share_index();
    c.threshold() == facts.threshold
        && facts.threshold >= 2
        && !c.is_secret()
        && c.identifier() == facts.identifier
        && usize::from(facts.length) == s.len()
        && facts.state(index) != Some(State::Destroyed)
}
