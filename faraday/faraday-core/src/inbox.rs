//! What the Inbox holds, put together: every wallet its files describe,
//! once each however many forms it came in, every seed its files give,
//! once each, and which seeds belong to which wallets.
//!
//! A wallet description comes as a descriptor, a wallet .json, a BIP 129
//! record, a multisig config, Bitcoin Core's import file, or split sheets
//! that add up. A seed comes as words in a text file (a SeedQR or
//! CompactSeedQR picture read at a stick visit arrives as one), as SLIP-39
//! shares or codex32 strings that add up. A seed whose fingerprint names a key of a
//! wallet here loads with that wallet; a seed no wallet names is a
//! potential wallet: the person adds a BIP-39 passphrase or does not, and
//! chooses the kind of single-key wallet, before it becomes one.
//!
//! Seed XOR parts are BIP-39 words on their own, so each reads as a seed
//! here; they are put together in Add a key.

use crate::strings::EN;
use crate::wallet::KeyTexts;
use std::collections::BTreeSet;

use faraday_bip::bip39::{Language, Mnemonic};
use faraday_bip::codex32::Codex32;
use faraday_bip::keys::Fingerprint;
use faraday_bip::slip39;
use faraday_crypto::work::Compute;
use faraday_shell_api::Key as KeyIn;
use faraday_text::{Text, text};
use zeroize::Zeroizing;

use crate::create::NewKind;
use crate::wallet::{FileKind, Refusal, Session, Wallet, fp_text};
use crate::{Action, Faraday, Item, Sheet, forms, inbox};

/// Where a seed found in the Inbox comes from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SeedSource {
    /// A text file of BIP-39 words.
    Words(usize),
    /// A text file of BIP-39 words, with its BIP-39 passphrase in a file
    /// of its own, `passphrase-{fingerprint}.txt`, the fingerprint the
    /// words make with it (`docs/BACKUP-FLOW.md` §17.12 items 5 and 6).
    WordsWith(usize, usize),
    /// SLIP-39 shares that add up, by Inbox place.
    Slip39(Vec<usize>),
    /// A codex32 secret, or shares of it that add up.
    Codex32(Vec<usize>),
    /// Already loaded in this session.
    Loaded,
}

impl SeedSource {
    /// What the source is called on screen.
    pub fn name(&self, app: &Faraday) -> Text {
        match self {
            SeedSource::Words(k) | SeedSource::WordsWith(k, _) => {
                app.inbox.get(*k).map_or(Text::new(), |i| Text::of(&i.name))
            }
            SeedSource::Slip39(ks) => {
                Text::fill(EN.inbox_slip39_share_count, &[&*Text::num(ks.len() as u64)])
            }
            SeedSource::Codex32(ks) if ks.len() == 1 => Text::of("codex32"),
            SeedSource::Codex32(ks) => {
                Text::fill(EN.inbox_codex32_shares, &[&*Text::num(ks.len() as u64)])
            }
            SeedSource::Loaded => Text::of(EN.inbox_loaded),
        }
    }

    /// Whether a passphrase can go with it: BIP-39 words take a BIP-39
    /// passphrase and SLIP-39 shares their own; a codex32 seed takes none.
    pub fn takes_passphrase(&self) -> bool {
        !matches!(self, SeedSource::Codex32(_))
    }
}

/// A seed the Inbox gives, or one already loaded.
#[derive(Debug, Clone)]
pub struct FoundSeed {
    /// Its master fingerprint, with no passphrase.
    pub fingerprint: Fingerprint,
    /// A label: the file's name without its extension, or the loaded
    /// key's label.
    pub label: Text,
    /// Where it comes from: the first is the one used.
    pub sources: Vec<SeedSource>,
    /// The session has it already.
    pub loaded: bool,
}

/// A wallet the Inbox describes.
#[derive(Debug, Clone)]
pub struct FoundWallet {
    /// What it will be called.
    pub name: Text,
    /// The descriptor, as text: one wallet however many files say it.
    pub descriptor: Text,
    /// The files that describe it, by Inbox place.
    pub files: Vec<usize>,
    /// Rebuilt from split sheets rather than read from one file.
    pub from_shares: bool,
    /// Its keys' fingerprints.
    pub keys: Vec<Fingerprint>,
    /// Its shape: single key, or m of n and the script.
    pub shape: Text,
    /// Signatures it needs.
    pub needed: usize,
    /// Of its keys, how many a seed here or loaded gives.
    pub seeds_here: usize,
    /// The session has it already.
    pub loaded: bool,
}

/// How many times the Inbox pairs a words file with a passphrase file
/// beside it: each pair is a seed worked out (a PBKDF2), so past this many
/// the rest are not tried.
pub(crate) const PAIRS_TRIED: usize = 64;

/// A backup still short of parts.
#[derive(Debug, Clone)]
pub struct Waiting {
    /// What it is and what it lacks.
    pub line: Text,
}

/// The Inbox's files, with what they give ([`Found`]) kept worked out
/// (`docs/SPEED.md` §3.3, §9). Reads as the files themselves
/// (`Vec<Item>`).
///
/// Invariant: what [`InboxFiles::found`] answers is what
/// [`Faraday::found_in`] gives for the files and the session as they are
/// now. `found_in` reads exactly these inputs, and each is a way the kept
/// copy is dropped or told stale:
///
/// 1. the files: every change to them goes through `DerefMut`, which
///    drops the kept copy (a file added, removed, renamed, replaced or
///    marked); an [`Item`]'s own bytes, kind, wallet and sha never change
///    after it is made;
/// 2. the session's keys: each key's fingerprint (what a seed here is
///    loaded by, which wallet slots a key fills) and label (a key's seed
///    is named by it);
/// 3. the session's wallets: each one's name and policy (which wallets
///    here are loaded already, the wallets listed from the session);
/// 4. nothing else: the words' fingerprints and the texts read are
///    functions of the files alone, kept by what they are worked out from
///    (`fp_memo`, `texts_read`).
///
/// Inputs 2 and 3 are the session's own fields, which are changed in
/// many places; the kept copy holds what it was worked out from
/// ([`SessionSeen`]) and is worked out again when the session no longer
/// matches it, a comparison of a few keys and wallets, not of the files.
#[derive(Default)]
pub struct InboxFiles {
    items: Vec<Item>,
    kept: std::cell::RefCell<Option<(SessionSeen, Found)>>,
    rebuilds: std::cell::Cell<u64>,
}

/// The session as [`InboxFiles`]' kept copy was worked out from it.
struct SessionSeen {
    keys: Vec<(Fingerprint, Text)>,
    wallets: Vec<(Text, faraday_bip::policy::WalletPolicy)>,
}

impl SessionSeen {
    fn of(s: &Session) -> SessionSeen {
        SessionSeen {
            keys: s
                .keys
                .all()
                .map(|k| (k.fingerprint, Text::of(&k.label)))
                .collect(),
            wallets: s
                .wallets
                .iter()
                .map(|w| (Text::of(&w.name), w.policy().clone()))
                .collect(),
        }
    }

    /// Whether `s` is the session this was taken from. Costs a
    /// comparison of each wallet's policy.
    fn matches(&self, s: &Session) -> bool {
        self.keys.len() == s.keys.len()
            && self.wallets.len() == s.wallets.len()
            && self
                .keys
                .iter()
                .zip(s.keys.all())
                .all(|(a, k)| a.0 == k.fingerprint && *a.1 == *k.label)
            && self
                .wallets
                .iter()
                .zip(&s.wallets)
                .all(|(a, w)| *a.0 == *w.name && a.1 == *w.policy())
    }
}

impl InboxFiles {
    /// What `make` gives for these files and `session`, worked out
    /// again only when either changed since. Costs what `make` does
    /// (`found_in`: every file read once) the first time after a change,
    /// a comparison of the session's keys and wallets after.
    pub(crate) fn found(&self, session: &Session, make: impl FnOnce(&[Item]) -> Found) -> Found {
        if let Some((seen, found)) = &*self.kept.borrow()
            && seen.matches(session)
        {
            return found.clone();
        }
        self.rebuilds.set(self.rebuilds.get() + 1);
        let found = make(&self.items);
        *self.kept.borrow_mut() = Some((SessionSeen::of(session), found.clone()));
        found
    }

    /// How many times [`InboxFiles::found`] worked its answer out.
    pub fn rebuilds(&self) -> u64 {
        self.rebuilds.get()
    }
}

/// The Inbox's area: the files read from sticks, the wallets left out of
/// the next load, the seed being made a wallet, and when a passphrase was
/// last typed. Reads of the files go through [`std::ops::Deref`]; every
/// change is one of the methods below.
#[derive(Default)]
pub(crate) struct InboxArea {
    files: InboxFiles,
    /// Wallets the Inbox describes left out of its next load, by
    /// descriptor.
    skip: BTreeSet<Text>,
    /// A seed from the Inbox being made a wallet.
    potential: Option<Potential>,
    /// When a passphrase was last typed on a sheet that shows the key it
    /// makes (the potential wallet's, a seed read's), until the key is
    /// worked out once typing has paused ([`crate::motion::TYPED_PAUSE_MS`]).
    typed_at: Option<u64>,
}

impl std::ops::Deref for InboxArea {
    type Target = InboxFiles;
    fn deref(&self) -> &InboxFiles {
        &self.files
    }
}

impl InboxArea {
    /// Puts `item` at the end of the files.
    pub(crate) fn push(&mut self, item: Item) {
        self.files.push(item);
    }

    /// Puts `item` at place `at`.
    #[cfg(feature = "testkit")]
    pub(crate) fn insert(&mut self, at: usize, item: Item) {
        self.files.insert(at, item);
    }

    /// Takes file `at` out.
    pub(crate) fn remove(&mut self, at: usize) -> Item {
        self.files.remove(at)
    }

    /// Keeps the files `keep` says to.
    pub(crate) fn retain(&mut self, keep: impl FnMut(&Item) -> bool) {
        self.files.retain(keep);
    }

    /// Puts all of `items` at the end of the files.
    pub(crate) fn extend(&mut self, items: impl IntoIterator<Item = Item>) {
        self.files.extend(items);
    }

    /// Names the last file after `stem` and its own extension (`txt`
    /// when it has none) unless a name in `taken` is that; its name
    /// after, or `None` when there is no file.
    pub(crate) fn name_last_after(&mut self, stem: &str, taken: &[Text]) -> Option<Text> {
        let last = self.files.changed().last_mut()?;
        let ext = Text::of(last.name.rsplit_once('.').map_or("txt", |(_, e)| e));
        let wanted = text!("{stem}.{}", &*ext);
        if !taken.contains(&wanted) {
            last.name = (&*wanted).into();
        }
        Some(Text::of(&last.name))
    }

    /// Notes that every file named `name` is a picture holding `kind`.
    pub(crate) fn set_picture_kind(&mut self, name: &str, kind: FileKind) {
        for i in self.files.changed().iter_mut().filter(|i| i.name == name) {
            i.picture = Some(kind);
        }
    }

    /// Notes that the first file named `name` that is a picture holds
    /// `kind` now.
    pub(crate) fn retag_picture(&mut self, name: &str, kind: FileKind) {
        if let Some(pic) = self
            .files
            .changed()
            .iter_mut()
            .find(|i| i.name == name && i.picture.is_some())
        {
            pic.picture = Some(kind);
        }
    }

    /// Gives file `at` the name `name`.
    #[cfg(test)]
    pub(crate) fn rename(&mut self, at: usize, name: Text) {
        self.files.changed()[at].name = (&*name).into();
    }

    /// Puts `item` in place of file `at`.
    #[cfg(test)]
    pub(crate) fn replace_at(&mut self, at: usize, item: Item) {
        self.files.changed()[at] = item;
    }

    /// Gives file `at` the contents `bytes`.
    #[cfg(feature = "testkit")]
    pub(crate) fn set_bytes(&mut self, at: usize, bytes: Vec<u8>) {
        self.files.changed()[at].bytes = bytes.into();
    }

    /// Flips the lowest bit of byte `i` of file `at`.
    #[cfg(feature = "testkit")]
    pub(crate) fn flip_byte(&mut self, at: usize, i: usize) {
        self.files.changed()[at].bytes[i] ^= 1;
    }

    /// Types `text` as the passphrase and its repeat of the potential
    /// wallet, and clears its error; whether there is a potential wallet.
    pub(crate) fn type_potential_passphrase(&mut self, text: &str) -> bool {
        let Some(p) = self.potential.as_mut() else {
            return false;
        };
        p.passphrase.set(text);
        p.again.set(text);
        p.typing = true;
        p.error = None;
        true
    }

    /// Takes every file out.
    pub(crate) fn clear(&mut self) {
        self.files.clear();
    }

    /// Replaces all the files.
    pub(crate) fn replace(&mut self, files: InboxFiles) {
        self.files = files;
    }

    /// The wallets left out of the next load, by descriptor.
    pub(crate) fn skip(&self) -> &BTreeSet<Text> {
        &self.skip
    }

    /// The potential wallet's passphrase hidden (its eye shows it again).
    pub(crate) fn hide_secrets(&mut self) {
        if let Some(p) = self.potential.as_mut() {
            p.shown = false;
        }
    }

    /// The seed being made a wallet.
    pub(crate) fn potential(&self) -> Option<&Potential> {
        self.potential.as_ref()
    }

    /// Starts making a wallet of a seed.
    pub(crate) fn set_potential(&mut self, p: Potential) {
        self.potential = Some(p);
    }

    /// Forgets the potential wallet.
    pub(crate) fn clear_potential(&mut self) {
        self.potential = None;
    }

    /// When a passphrase was last typed on a sheet that shows its key.
    pub(crate) fn typed_at(&self) -> Option<u64> {
        self.typed_at
    }

    /// Notes when a passphrase was last typed, or that none waits.
    pub(crate) fn set_typed_at(&mut self, at: Option<u64>) {
        self.typed_at = at;
    }
}

impl Faraday {
    /// Whether the Inbox holds a file of this name and size: the stick
    /// row says "In Faraday".
    pub(crate) fn inbox_holds(&self, name: &str, size: u64) -> bool {
        self.inbox
            .iter()
            .any(|i| i.name == name && i.bytes.len() as u64 == size)
    }

    /// The Inbox PSBT to offer first: the one with the fewest signatures,
    /// so a cosigner's signed copy never stands in for the transaction.
    /// Reads what each file said when it came in ([`Item::psbt`]).
    pub(crate) fn lead_psbt(&self) -> Option<usize> {
        self.inbox
            .iter()
            .enumerate()
            .filter_map(|(i, it)| Some((it.psbt()?.sigs, i)))
            .min()
            .map(|(_, i)| i)
    }

    /// Signatures the open spend's wallet needs.
    pub fn spend_needed(&self) -> usize {
        self.wallets.spend().map(|s| s.needed).unwrap_or(1)
    }

    /// The files read from sticks.
    pub fn inbox(&self) -> &InboxFiles {
        &self.inbox.files
    }

    /// The seed being made a wallet.
    pub fn potential(&self) -> Option<&Potential> {
        self.inbox.potential()
    }

    /// Puts a file that came in into Files and returns its place: the
    /// same file read again is the one already there, not a second copy.
    pub(crate) fn inbox_file_in(&mut self, item: Item) -> usize {
        if let Some(at) = self.inbox.iter().position(|i| i.bytes == item.bytes) {
            return at;
        }
        self.inbox.push(item);
        self.save_boxes();
        self.inbox.len() - 1
    }
}

impl std::ops::Deref for InboxFiles {
    type Target = Vec<Item>;
    fn deref(&self) -> &Vec<Item> {
        &self.items
    }
}

/// Every change to the files drops what was worked out of them: the
/// invariant of [`InboxFiles`].
impl InboxFiles {
    fn changed(&mut self) -> &mut Vec<Item> {
        *self.kept.get_mut() = None;
        &mut self.items
    }

    fn push(&mut self, item: Item) {
        self.changed().push(item);
    }

    #[cfg(feature = "testkit")]
    fn insert(&mut self, at: usize, item: Item) {
        self.changed().insert(at, item);
    }

    fn remove(&mut self, at: usize) -> Item {
        self.changed().remove(at)
    }

    fn retain(&mut self, keep: impl FnMut(&Item) -> bool) {
        self.changed().retain(keep);
    }

    fn extend(&mut self, items: impl IntoIterator<Item = Item>) {
        self.changed().extend(items);
    }

    fn clear(&mut self) {
        self.changed().clear();
    }
}

impl From<Vec<Item>> for InboxFiles {
    fn from(items: Vec<Item>) -> InboxFiles {
        InboxFiles {
            items,
            ..InboxFiles::default()
        }
    }
}

impl<'a> IntoIterator for &'a InboxFiles {
    type Item = &'a Item;
    type IntoIter = std::slice::Iter<'a, Item>;
    fn into_iter(self) -> Self::IntoIter {
        self.items.iter()
    }
}

impl FromIterator<Item> for InboxFiles {
    fn from_iter<I: IntoIterator<Item = Item>>(i: I) -> InboxFiles {
        InboxFiles::from(i.into_iter().collect::<Vec<_>>())
    }
}

/// Everything the Inbox gives, put together.
#[derive(Debug, Clone, Default)]
pub struct Found {
    /// Wallets, once each.
    pub wallets: Vec<FoundWallet>,
    /// Seeds, once each.
    pub seeds: Vec<FoundSeed>,
    /// Backups short of parts.
    pub waiting: Vec<Waiting>,
    /// Account xpubs no wallet here uses and no seed here gives: each a
    /// potential watch-only wallet.
    pub xpubs: Vec<FoundXpub>,
}

/// An account xpub in the Inbox that no wallet here uses.
#[derive(Debug, Clone)]
pub struct FoundXpub {
    /// The key expression, `[fingerprint/path]xpub`.
    pub key: Text,
    /// Its master fingerprint, when it has an origin.
    pub fingerprint: Option<Fingerprint>,
    /// Its path from the master, as text, when it has an origin.
    pub path: Option<Text>,
    /// The single-key kind its path says (BIP-44, 49, 84 or 86), or
    /// `None` when it has no origin and the person chooses.
    pub kind: Option<NewKind>,
    /// The files it is in, by Inbox place.
    pub files: Vec<usize>,
}

/// The single-key kind an account path's purpose says; `Err` for a
/// multisig account (BIP-45, 48, 87), which is a cosigner's key and not a
/// wallet on its own.
fn kind_of_path(path: &faraday_bip::bitcoin::bip32::DerivationPath) -> Result<Option<NewKind>, ()> {
    let purpose = path.into_iter().next().map(|c| match c {
        faraday_bip::bitcoin::bip32::ChildNumber::Hardened { index }
        | faraday_bip::bitcoin::bip32::ChildNumber::Normal { index } => *index,
    });
    match purpose {
        Some(44) => Ok(Some(NewKind::Legacy)),
        Some(49) => Ok(Some(NewKind::NestedSegwit)),
        Some(84) => Ok(Some(NewKind::NativeSegwit)),
        Some(86) => Ok(Some(NewKind::Taproot)),
        Some(45 | 48 | 87) => Err(()),
        _ => Ok(None),
    }
}

impl Found {
    /// Seeds no wallet here names: potential wallets.
    pub fn potential(&self) -> Vec<&FoundSeed> {
        self.seeds
            .iter()
            .filter(|s| !self.wallets.iter().any(|w| w.keys.contains(&s.fingerprint)))
            .collect()
    }

    /// Wallets not loaded yet.
    pub fn to_load(&self) -> Vec<&FoundWallet> {
        self.wallets.iter().filter(|w| !w.loaded).collect()
    }
}

/// A seed as bytes to derive from, with what the person may add.
pub(crate) enum Secret {
    /// BIP-39 words.
    Words(Mnemonic),
    /// SLIP-39 shares.
    Slip39(Vec<slip39::Share>),
    /// A master seed from codex32 strings, with the plain facts of their
    /// set, kept with the key it loads (`docs/CODEX32.md` §9.4).
    Codex32(Zeroizing<Vec<u8>>, Option<faraday_vault::codex32::SetFacts>),
}

impl Secret {
    /// Costs a PBKDF2 and a master key. The fingerprint of the master key,
    /// with `passphrase` where the form takes one.
    pub(crate) fn fingerprint(
        &self,
        c: &Compute,
        passphrase: &str,
        network: faraday_bip::keys::Network,
    ) -> Option<Fingerprint> {
        let material = match self {
            Secret::Words(m) => faraday_keys::Material::Words(m),
            Secret::Slip39(shares) => faraday_keys::Material::Slip39(shares),
            Secret::Codex32(s, _) => faraday_keys::Material::Seed(s),
        };
        faraday_keys::fingerprint(c, material, passphrase, network)
    }

    /// Costs what `add_words_with`, `recover` and `add_seed` cost. Adds the
    /// key to the session, with `passphrase` where the form takes one.
    pub(crate) fn add(
        &self,
        c: &Compute,
        passphrase: &str,
        label: &str,
        keys: &mut faraday_keys::Keys,
    ) -> Result<Fingerprint, Refusal> {
        match self {
            Secret::Words(m) => keys.add_words_with(c, &phrase(m), passphrase, label, None),
            Secret::Slip39(shares) => {
                let secret = faraday_keys::slip39_seed(c, shares, passphrase.as_bytes())
                    .map_err(|e| Refusal::Words(faraday_strings::bip::slip39_error(&e)))?;
                keys.add_seed(c, &secret, label)
            }
            Secret::Codex32(s, facts) => {
                let fp = keys.add_seed(c, s, label)?;
                if let Some(i) = keys.position(fp) {
                    keys.set_codex32(i, facts.clone());
                }
                Ok(fp)
            }
        }
    }
}

/// A mnemonic's words, one space between each.
fn phrase(m: &Mnemonic) -> Text {
    let lang = m.language();
    let mut words = Text::new();
    for (n, &i) in m.indices().iter().enumerate() {
        if n > 0 {
            words.push(' ');
        }
        words.push_str(lang.word(i));
    }
    words
}

/// Costs what `parse_words` costs. The words a text file spells, in
/// whichever BIP-39 list they are in.
fn words_of(c: &Compute, text: &str) -> Option<Mnemonic> {
    let typed = forms::file_words(text);
    Language::ALL
        .into_iter()
        .find_map(|lang| crate::wallet::parse_words(c, lang, &typed).ok())
}

pub(crate) use faraday_intake::{Part, part_of};

impl Faraday {
    /// Costs what `secret_in` costs. The secret a seed source gives.
    pub(crate) fn inbox_secret(&self, c: &Compute, source: &SeedSource) -> Option<Secret> {
        self.secret_in(c, &self.inbox, source)
    }

    /// Costs what `words_of` costs. The secret a seed source among `items`
    /// gives.
    pub(crate) fn secret_in(
        &self,
        c: &Compute,
        items: &[Item],
        source: &SeedSource,
    ) -> Option<Secret> {
        let text = |k: &usize| items.get(*k).map(|i| Text::from_utf8_lossy(&i.bytes));
        match source {
            SeedSource::Words(k) | SeedSource::WordsWith(k, _) => {
                words_of(c, &text(k)?).map(Secret::Words)
            }
            SeedSource::Slip39(ks) => {
                let shares: Vec<slip39::Share> = ks
                    .iter()
                    .filter_map(|k| match part_of(&text(k)?) {
                        Some(Part::Slip39(s)) => Some(s),
                        _ => None,
                    })
                    .collect();
                Some(Secret::Slip39(shares))
            }
            SeedSource::Codex32(ks) => {
                let parts: Vec<Codex32> = ks
                    .iter()
                    .filter_map(|k| match part_of(&text(k)?) {
                        Some(Part::Codex32(c)) => Some(c),
                        _ => None,
                    })
                    .collect();
                let seed = faraday_keys::codex32_seed(&parts).ok()?;
                // The set's facts: the strings in Files; the plan asks how
                // many there are (§9.4).
                Some(Secret::Codex32(seed, crate::forms::set_facts(&parts)))
            }
            SeedSource::Loaded => None,
        }
    }

    /// Costs what `secret_in` and `master` cost. The fingerprint a source's
    /// seed among `items` has with no passphrase.
    fn source_fingerprint(
        &self,
        c: &Compute,
        items: &[Item],
        source: &SeedSource,
    ) -> Option<Fingerprint> {
        let tag: &[u8] = match source {
            SeedSource::Words(_) => b"words",
            SeedSource::WordsWith(..) => b"words+passphrase",
            SeedSource::Slip39(_) => b"slip39",
            SeedSource::Codex32(_) => b"codex32",
            SeedSource::Loaded => return None,
        };
        let mut material: Zeroizing<Vec<u8>> = Zeroizing::new(Vec::new());
        match source {
            SeedSource::Words(k) => {
                let b = &items.get(*k)?.bytes;
                material.reserve_exact(b.len());
                material.extend_from_slice(b);
            }
            SeedSource::WordsWith(k, p) => {
                let (w, p) = (&items.get(*k)?.bytes, &items.get(*p)?.bytes);
                material.reserve_exact(w.len() + 1 + p.len());
                material.extend_from_slice(w);
                material.push(0);
                material.extend_from_slice(p);
            }
            SeedSource::Slip39(ks) | SeedSource::Codex32(ks) => {
                let held: Vec<&Item> = ks.iter().filter_map(|k| items.get(*k)).collect();
                material.reserve_exact(held.iter().map(|i| i.bytes.len()).sum());
                for i in held {
                    material.extend_from_slice(&i.bytes);
                }
            }
            SeedSource::Loaded => return None,
        };
        self.kept.fingerprint_memo(tag, &material, || {
            self.secret_in(c, items, source)?.fingerprint(
                c,
                &self.source_passphrase(items, source),
                self.session.network(),
            )
        })
    }

    /// The passphrase a seed source among `items` brings: a passphrase
    /// file's one line, else none.
    pub(crate) fn source_passphrase(&self, items: &[Item], source: &SeedSource) -> Text {
        let SeedSource::WordsWith(_, p) = source else {
            return Text::new();
        };
        let Some(item) = items.get(*p) else {
            return Text::new();
        };
        let text = Text::from_utf8_lossy(&item.bytes);
        let line = text
            .strip_suffix('\n')
            .map(|t| t.strip_suffix('\r').unwrap_or(t))
            .unwrap_or(&text);
        Text::of(line)
    }

    /// Costs what `found_in` costs. Everything the Inbox gives, put
    /// together. While a frame is drawn it is the kept copy
    /// ([`InboxFiles`]), worked out again only when the files or the
    /// session's keys and wallets change, and once a frame in the drawing
    /// memo (`docs/SPEED.md` §3.3); a decision (a press, a load) works it
    /// out from the state as it is. Each wallet file is read when it came
    /// in ([`crate::Item::wallet`]) and each seed's fingerprint is kept
    /// once worked out; costs `found_in` (a merge of any split sheets,
    /// every file looked at) except while drawing, where it is that the
    /// first time after a change and a comparison of the session after.
    ///
    /// Works out [`crate::drawing::View::inbox_found`] from the state as it is: costs
    /// `found_in`.
    pub fn inbox_found_now(&self, c: &Compute) -> Found {
        self.found_in(c, &self.inbox)
    }

    /// The name a file read from a stick as `name` comes into Files under:
    /// its own, replacing a file of that name with the same bytes. A file
    /// Files holds under it with other bytes is kept, this one numbered
    /// beside it, and the visit says so. A lookup per file.
    pub(crate) fn inbox_name_for(&mut self, name: &str, bytes: &[u8]) -> Text {
        if !self
            .inbox
            .iter()
            .any(|i| i.name == name && *i.bytes != bytes)
        {
            self.inbox.retain(|i| i.name != name);
            return Text::of(name);
        }
        let kept = self.free_inbox_name(name);
        // The seed asked for is this copy's, not the file held before.
        self.visit.load_after_renamed(name, &kept);
        let told = Text::fill(EN.storage_event_kept_as, &[name, &kept]);
        self.visit.log_push((told, true));
        kept
    }

    /// Costs a SHA-256 of the vault (through `Item::new`). A vault written from For the stick stays in
    /// Files as the stick now holds it: its copy from before goes; another
    /// file of its name stays, the vault numbered beside it and said.
    pub(crate) fn inbox_vault_written(&mut self, c: &Compute, item: &Item) {
        if item.kind != FileKind::Vault {
            return;
        }
        let vaults = &self.vaults;
        self.inbox
            .retain(|i| !(i.kind == FileKind::Vault && vaults.same_vault_files(i.sha, item.sha)));
        let kept = self.free_inbox_name(&item.name);
        if kept.as_str() != item.name {
            let told = Text::fill(EN.storage_event_kept_as, &[&item.name, &kept]);
            self.visit.log_push((told, true));
        }
        self.inbox.push(Item::new(c, &kept, item.bytes.clone()));
    }

    /// Costs what `source_fingerprint`, `config_name_kept` and `read_kept`
    /// cost, a words file and a passphrase file paired at most
    /// [`PAIRS_TRIED`] times. Everything `items` give, put together, with
    /// the session's own keys and wallets: the Inbox, or the files a boot
    /// import holds. Places in what it returns are places in `items`.
    #[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
    pub(crate) fn found_in(&self, c: &Compute, items: &[Item]) -> Found {
        let mut found = Found::default();

        // Seeds: words files, then split seeds that add up, then the
        // session's own keys; once each, by fingerprint.
        let add_seed =
            |found: &mut Found, fp: Fingerprint, label: Text, src: SeedSource, loaded: bool| {
                match found.seeds.iter_mut().find(|s| s.fingerprint == fp) {
                    Some(s) => {
                        s.loaded |= loaded;
                        if !s.sources.contains(&src) {
                            s.sources.push(src);
                        }
                    }
                    None => found.seeds.push(FoundSeed {
                        fingerprint: fp,
                        label,
                        sources: vec![src],
                        loaded,
                    }),
                }
            };
        let loaded_fp = |fp: Fingerprint| self.session.keys.all().any(|k| k.fingerprint == fp);
        // A passphrase file a stick keeps beside a seed's words, named by
        // the fingerprint the words make with it (`docs/BACKUP-FLOW.md`
        // §17.12 item 6): the words with it are that key, and the words
        // alone no seed of their own.
        let passphrase_files: Vec<(usize, Text)> = items
            .iter()
            .enumerate()
            .filter_map(|(k, it)| {
                let fp = it.name.strip_prefix("passphrase-")?.strip_suffix(".txt")?;
                (fp.len() == 8 && fp.bytes().all(|b| b.is_ascii_hexdigit()))
                    .then(|| (k, Text::of(&fp.to_ascii_lowercase())))
            })
            .collect();
        // Each pair is a seed worked out: past the bound the rest are not
        // tried, and the Inbox says how many.
        let mut tried = 0usize;
        let mut untried = 0usize;
        for (k, item) in items.iter().enumerate() {
            if item.kind != FileKind::Words {
                continue;
            }
            let mut with = false;
            for (p, name) in &passphrase_files {
                if tried == PAIRS_TRIED {
                    untried += 1;
                    continue;
                }
                tried += 1;
                let src = SeedSource::WordsWith(k, *p);
                if let Some(fp) = self.source_fingerprint(c, items, &src)
                    && fp_text(fp).eq_ignore_ascii_case(name)
                {
                    add_seed(&mut found, fp, stem(&item.name), src, loaded_fp(fp));
                    with = true;
                }
            }
            if with {
                continue;
            }
            let src = SeedSource::Words(k);
            if let Some(fp) = self.source_fingerprint(c, items, &src) {
                add_seed(&mut found, fp, stem(&item.name), src, loaded_fp(fp));
            }
        }
        if untried > 0 {
            found.waiting.push(Waiting {
                line: Text::fill(
                    EN.inbox_pairs_untried,
                    &[&*Text::num(untried as u64), &*Text::num(PAIRS_TRIED as u64)],
                ),
            });
        }
        // Split seeds: SLIP-39 shares by their identifier, codex32 strings
        // by theirs.
        let mut slip: Vec<(u16, Vec<usize>, u8)> = Vec::new();
        let mut codex: Vec<([u8; 4], Vec<usize>, usize, bool)> = Vec::new();
        for (k, item) in items.iter().enumerate() {
            if item.kind != FileKind::SeedPart {
                continue;
            }
            match part_of(&Text::from_utf8_lossy(&item.bytes)) {
                Some(Part::Slip39(s)) => match slip.iter_mut().find(|g| g.0 == s.identifier()) {
                    Some(g) => g.1.push(k),
                    None => slip.push((s.identifier(), vec![k], s.member_threshold())),
                },
                Some(Part::Codex32(c)) => {
                    let mut id = [0u8; 4];
                    for (d, s) in id.iter_mut().zip(c.identifier()) {
                        *d = s;
                    }
                    match codex.iter_mut().find(|g| g.0 == id) {
                        Some(g) => {
                            g.1.push(k);
                            g.3 |= c.is_secret();
                        }
                        None => {
                            codex.push((id, vec![k], usize::from(c.threshold()), c.is_secret()))
                        }
                    }
                }
                None => {}
            }
        }
        for (_, ks, need) in slip {
            let src = SeedSource::Slip39(ks.clone());
            match (ks.len() >= usize::from(need))
                .then(|| self.source_fingerprint(c, items, &src))
                .flatten()
            {
                Some(fp) => add_seed(
                    &mut found,
                    fp,
                    Text::of(EN.inbox_slip_39_seed),
                    src,
                    loaded_fp(fp),
                ),
                None => found.waiting.push(Waiting {
                    line: Text::fill(
                        EN.inbox_slip_39_shares,
                        &[&*Text::num(ks.len() as u64), &*Text::num(u64::from(need))],
                    ),
                }),
            }
        }
        for (_, ks, need, secret) in codex {
            let src = SeedSource::Codex32(ks.clone());
            match (secret || ks.len() >= need)
                .then(|| self.source_fingerprint(c, items, &src))
                .flatten()
            {
                Some(fp) => add_seed(
                    &mut found,
                    fp,
                    Text::of(EN.inbox_codex32_seed),
                    src,
                    loaded_fp(fp),
                ),
                None => found.waiting.push(Waiting {
                    line: Text::fill(
                        EN.inbox_codex32_shares_inline,
                        &[&*Text::num(ks.len() as u64), &*Text::num(need as u64)],
                    ),
                }),
            }
        }
        for key in self.session.keys.all() {
            add_seed(
                &mut found,
                key.fingerprint,
                Text::of(&key.label),
                SeedSource::Loaded,
                true,
            );
        }

        // Wallets: every file that reads as one, once each by descriptor,
        // then split sheets that add up; and the session's own, so a seed
        // they name is not a potential wallet.
        let push_wallet = |found: &mut Found,
                           name: Text,
                           policy: faraday_bip::policy::WalletPolicy,
                           file: Option<usize>,
                           shares: bool,
                           loaded: bool| {
            let descriptor = crate::wallet::same_wallet(&policy);
            if let Some(w) = found
                .wallets
                .iter_mut()
                .find(|w| *w.descriptor == *descriptor)
            {
                if let Some(k) = file {
                    w.files.push(k);
                }
                w.loaded |= loaded;
                return;
            }
            let wallet = Wallet::new(
                &name,
                policy,
                Default::default(),
                crate::wallet::Origin::Named,
            );
            let keys: Vec<Fingerprint> = self
                .session
                .slots(&wallet)
                .iter()
                .filter_map(|s| s.fingerprint)
                .collect();
            found.wallets.push(FoundWallet {
                name,
                descriptor: Text::of(&descriptor),
                files: file.into_iter().collect(),
                from_shares: shares,
                keys,
                shape: Text::of(&Session::shape(&wallet)),
                needed: crate::wallet::needed(&wallet),
                seeds_here: 0,
                loaded,
            });
        };
        let loaded_desc: BTreeSet<Text> = self
            .session
            .wallets
            .iter()
            .map(|w| Text::of(&crate::wallet::same_wallet(w.policy())))
            .collect();
        for (k, item) in items.iter().enumerate() {
            // Read when the file came in.
            let Some(policy) = item.wallet() else {
                continue;
            };
            let loaded = loaded_desc.contains(&*crate::wallet::same_wallet(policy));
            push_wallet(
                &mut found,
                self.wallet_name_in(items, k),
                policy.clone(),
                Some(k),
                false,
                loaded,
            );
        }
        let shares: Vec<Text> = items
            .iter()
            .filter(|i| i.kind == FileKind::Share)
            .map(|i| Text::from_utf8_lossy(&i.bytes))
            .collect();
        if !shares.is_empty() {
            match crate::restore::merge(&shares) {
                Ok(m) => match m.whole {
                    Some(whole) => {
                        let name = self
                            .kept
                            .config_name_kept(c, &whole)
                            .map_or_else(|| Text::of(EN.inbox_restored_wallet), |n| Text::of(&n));
                        if let Some(p) = self.kept.read_kept(c, &whole) {
                            let loaded = loaded_desc.contains(&*crate::wallet::same_wallet(&p));
                            push_wallet(&mut found, name, p, None, true, loaded);
                        }
                    }
                    None => found.waiting.push(Waiting {
                        line: Text::fill(
                            EN.inbox_split_sheets_progress,
                            &[&*Text::num(m.have.len() as u64), &*Text::num(m.n as u64)],
                        ),
                    }),
                },
                Err(e) => found.waiting.push(Waiting {
                    line: Text::fill(EN.inbox_split_sheets, &[&e]),
                }),
            }
        }
        for w in &self.session.wallets {
            // As its descriptor reads: a silent payments wallet has none
            // (its "descriptor" is its address); any other is its own
            // policy, with nothing to read.
            if w.policy().silent().is_some() {
                continue;
            }
            let policy = w.policy().clone();
            push_wallet(&mut found, Text::of(&w.name), policy, None, false, true);
        }
        let seeds: Vec<Fingerprint> = found.seeds.iter().map(|s| s.fingerprint).collect();
        for w in found.wallets.iter_mut() {
            w.seeds_here = w.keys.iter().filter(|k| seeds.contains(k)).count();
        }
        // Account xpubs: once each, and only those no wallet here uses and
        // no seed here gives (the seed is the stronger of the two).
        for (k, item) in items.iter().enumerate() {
            if item.kind != FileKind::Key {
                continue;
            }
            let text = Text::from_utf8_lossy(&item.bytes);
            let Some(key) = crate::create::read_key(&text) else {
                continue;
            };
            let Ok(parsed) = faraday_bip::policy::PolicyKey::parse(&key) else {
                continue;
            };
            // A key with no fingerprint and path is refused as a wallet's
            // key, so it is not offered as a wallet.
            let (Some(_), Some(path)) = (parsed.fingerprint(), parsed.path()) else {
                continue;
            };
            let kind = match kind_of_path(path) {
                Err(()) => continue,
                Ok(k) => k,
            };
            let xpub = text!("{}", parsed.xpub());
            let fp = parsed.fingerprint();
            if found.wallets.iter().any(|w| w.descriptor.contains(&*xpub))
                || fp.is_some_and(|f| seeds.contains(&f))
            {
                continue;
            }
            match found.xpubs.iter_mut().find(|x| *x.key == *key) {
                Some(x) => x.files.push(k),
                None => found.xpubs.push(FoundXpub {
                    key: Text::of(&key),
                    fingerprint: fp,
                    path: parsed.path().map(|p| {
                        let mut path = Text::new();
                        for ch in text!("m/{p}").chars() {
                            path.push(if ch == '\'' { 'h' } else { ch });
                        }
                        path
                    }),
                    kind,
                    files: vec![k],
                }),
            }
        }
        found
    }

    /// The name a wallet file gives its wallet: a wallet .json's label or
    /// an addwallet command's name, or the file's name in words.
    pub(crate) fn inbox_wallet_name(&self, index: usize) -> Text {
        self.wallet_name_in(&self.inbox, index)
    }

    /// [`Faraday::inbox_wallet_name`] for a file among `items`.
    pub(crate) fn wallet_name_in(&self, items: &[Item], index: usize) -> Text {
        let Some(item) = items.get(index) else {
            return Text::new();
        };
        let text = Text::from_utf8_lossy(&item.bytes);
        let name = stem(&item.name);
        let mut words = Text::new();
        for (n, w) in name
            .strip_suffix("-wallet")
            .unwrap_or(&name)
            .split(['-', '_'])
            .enumerate()
        {
            if n > 0 {
                words.push(' ');
            }
            let mut c = w.chars();
            if let Some(f) = c.next() {
                words.extend(f.to_uppercase());
                words.push_str(c.as_str());
            }
        }
        let carried = crate::wallet::carried_name(&text).map_or(words, |t| Text::of(&t));
        Text::of(&crate::wallet::wallet_name(&carried))
    }

    /// Loads the chosen wallets the Inbox describes, each with the seeds
    /// here that are its keys. Seeds load only with no stick attached;
    /// the wallets load either way. Returns (wallets, seeds) loaded.
    pub(crate) fn inbox_load(&mut self) -> (usize, usize) {
        let inbox = std::mem::take(&mut self.inbox);
        let skip = inbox.skip().clone();
        let loaded = self.load_found_in(&inbox, &skip, &|_| false);
        self.inbox = inbox;
        self.refresh_spend();
        loaded
    }

    /// Loads the wallets `items` describe but those in `skip` (by
    /// descriptor), each with the seeds among `items` that are its keys,
    /// and the seeds `also` names whether a wallet chosen uses them or
    /// not. Seeds load only with no stick attached. Returns (wallets,
    /// seeds) loaded.
    pub(crate) fn load_found_in(
        &mut self,
        items: &[Item],
        skip: &BTreeSet<Text>,
        also: &dyn Fn(Fingerprint) -> bool,
    ) -> (usize, usize) {
        let c = self.compute();
        let found = self.found_in(&c, items);
        let chosen: Vec<FoundWallet> = found
            .to_load()
            .into_iter()
            .filter(|w| !skip.contains(&w.descriptor))
            .cloned()
            .collect();
        let mut keys = 0;
        if self.may_load_keys() {
            for s in &found.seeds {
                if s.loaded
                    || !(chosen.iter().any(|w| w.keys.contains(&s.fingerprint))
                        || also(s.fingerprint))
                {
                    continue;
                }
                let pass = s
                    .sources
                    .first()
                    .map(|src| self.source_passphrase(items, src))
                    .unwrap_or_default();
                if let Some(secret) = s
                    .sources
                    .first()
                    .and_then(|src| self.secret_in(&c, items, src))
                    && secret
                        .add(&c, &pass, &s.label, &mut self.session.keys)
                        .is_ok()
                {
                    keys += 1;
                }
            }
        }
        let mut wallets = 0;
        for w in &chosen {
            let (name, text) = match w.files.first() {
                Some(k) => (
                    self.wallet_name_in(items, *k),
                    Text::from_utf8_lossy(&items[*k].bytes),
                ),
                None => (w.name.clone(), w.descriptor.clone()),
            };
            let source = w
                .files
                .first()
                .map_or(Text::of(EN.inbox_its_shares), |k| Text::of(&items[*k].name));
            match self
                .session
                .add_wallet(&c, &name, &text, &source, crate::wallet::Origin::Named)
            {
                Ok(_) => wallets += 1,
                Err(e @ crate::wallet::Refusal::NotYourKey(_)) => {
                    self.toast(&Text::fill(EN.vaults_refused, &[&name, &e.text()]));
                }
                Err(_) => {}
            }
        }
        (wallets, keys)
    }

    /// After seeds load on their own (Import and load, Add a key): the
    /// wallets in the Inbox that name them load too.
    pub(crate) fn inbox_load_named(&mut self) -> usize {
        let c = self.compute();
        let found = self.inbox_found_now(&c);
        let loaded: Vec<Fingerprint> = self.session.keys.all().map(|k| k.fingerprint).collect();
        let mut n = 0;
        for w in found.to_load() {
            if !w.keys.iter().any(|k| loaded.contains(k))
                || self.inbox.skip().contains(&w.descriptor)
            {
                continue;
            }
            let (name, text, source) = match w.files.first() {
                Some(k) => (
                    self.inbox_wallet_name(*k),
                    Text::from_utf8_lossy(&self.inbox[*k].bytes),
                    Text::of(&self.inbox[*k].name),
                ),
                None => (
                    w.name.clone(),
                    w.descriptor.clone(),
                    Text::of(EN.inbox_its_shares),
                ),
            };
            match self
                .session
                .add_wallet(&c, &name, &text, &source, crate::wallet::Origin::Named)
            {
                Ok(_) => n += 1,
                Err(e @ crate::wallet::Refusal::NotYourKey(_)) => {
                    self.toast(&Text::fill(EN.vaults_refused, &[&name, &e.text()]));
                }
                Err(_) => {}
            }
        }
        if n > 0 {
            self.refresh_spend();
        }
        n
    }
}

/// A potential wallet being made: the seed, the passphrase the person
/// adds or not, and the kind of single-key wallet.
#[derive(Default)]
pub struct Potential {
    /// The seed's fingerprint with no passphrase, which names it.
    pub fingerprint: Option<Fingerprint>,
    /// The passphrase typed, empty for none.
    pub passphrase: crate::secret_text::SecretText,
    /// The passphrase typed again: it is typed twice, for nothing checks
    /// it and a typo makes another key.
    pub again: crate::secret_text::SecretText,
    /// Typing goes to the passphrase.
    pub typing: bool,
    /// With `typing`: it goes to the passphrase typed again.
    pub typing_again: bool,
    /// The passphrase is shown in the clear.
    pub shown: bool,
    /// The kind of single-key wallet it becomes.
    pub kind: NewKind,
    /// The fingerprint with the passphrase, worked out when it changes.
    pub with: Option<Fingerprint>,
    /// The last refusal.
    pub error: Option<Text>,
    /// An account xpub becoming a watch-only wallet instead, with whether
    /// its path fixes the kind.
    pub xpub: Option<(Text, bool)>,
}

/// The single-key kinds a potential wallet may become, the most common
/// first.
pub const KINDS: [NewKind; 4] = [
    NewKind::NativeSegwit,
    NewKind::Taproot,
    NewKind::NestedSegwit,
    NewKind::Legacy,
];

impl Faraday {
    /// The potential wallet's passphrase changed: with none, the seed's
    /// own fingerprint, a lookup; with one, the key it makes is worked
    /// out once typing pauses ([`Faraday::typed_tick`]), and none is
    /// shown until then.
    pub(crate) fn potential_typed(&mut self) {
        let Some(p) = self.inbox.potential.as_mut() else {
            return;
        };
        if p.passphrase.is_empty() || p.xpub.is_some() {
            self.inbox.set_typed_at(None);
            self.potential_refresh();
            return;
        }
        p.with = None;
        self.inbox.set_typed_at(Some(self.now_ms));
    }

    /// Typing into the potential wallet's sheet. Returns whether the key
    /// was taken.
    pub(crate) fn potential_key(&mut self, key: KeyIn) -> bool {
        if self.sheet != Some(Sheet::Potential) {
            return false;
        }
        match key {
            KeyIn::Escape => {
                self.inbox.clear_potential();
                self.sheet = None;
            }
            // A passphrase's Enter moves to the one again; the second's, or
            // an empty first's, makes the wallet.
            KeyIn::Enter
                if self.inbox.potential().is_some_and(|p| {
                    p.typing && !p.typing_again && p.xpub.is_none() && !p.passphrase.is_empty()
                }) =>
            {
                if let Some(p) = self.inbox.potential.as_mut() {
                    p.typing_again = true;
                }
            }
            KeyIn::Enter => self.potential_make(),
            // Nothing secret is typed with a stick attached.
            _ if !self.stick.is_empty() => {}
            KeyIn::Char(c) if !c.is_control() => {
                if let Some(p) = self.inbox.potential.as_mut()
                    && p.typing
                {
                    if p.typing_again {
                        p.again.push(c);
                    } else {
                        p.passphrase.push(c);
                    }
                    p.error = None;
                }
                self.potential_typed();
            }
            KeyIn::Backspace => {
                if let Some(p) = self.inbox.potential.as_mut()
                    && p.typing
                {
                    if p.typing_again {
                        p.again.pop();
                    } else {
                        p.passphrase.pop();
                    }
                    p.error = None;
                }
                self.potential_typed();
            }
            _ => {}
        }
        true
    }

    /// Opens the sheet for making seed `fp` a wallet.
    pub(crate) fn potential_open(&mut self, fp: Fingerprint) {
        self.inbox.set_potential(Potential {
            fingerprint: Some(fp),
            with: Some(fp),
            ..Potential::default()
        });
        self.sheet = Some(crate::Sheet::Potential);
    }

    /// Opens the sheet for making account xpub `i` of the Inbox a
    /// watch-only wallet.
    pub(crate) fn xpub_open(&mut self, i: usize) {
        let c = self.compute();
        let Some(x) = self.inbox_found_now(&c).xpubs.into_iter().nth(i) else {
            return;
        };
        self.inbox.set_potential(Potential {
            fingerprint: x.fingerprint,
            kind: x.kind.unwrap_or_default(),
            xpub: Some((x.key, x.kind.is_some())),
            ..Potential::default()
        });
        self.sheet = Some(crate::Sheet::Potential);
    }

    /// Makes the watch-only wallet an account xpub describes.
    fn xpub_make(&mut self) {
        let c = self.compute();
        let Some((key, kind, fp)) = self.inbox.potential().and_then(|p| {
            p.xpub
                .as_ref()
                .map(|(k, _)| (k.duplicate(), p.kind, p.fingerprint))
        }) else {
            return;
        };
        let descriptor = kind.descriptor(1, &[key.duplicate()]);
        let name = Text::fill(
            EN.inbox_watch_only,
            &[
                &fp.map_or_else(|| Text::of(EN.inbox_xpub), |f| Text::of(&fp_text(f))),
                crate::spend_backup::kind_name(kind),
            ],
        );
        match self.session.add_wallet(
            &c,
            &name,
            &descriptor,
            EN.inbox_an_xpub_in_files,
            crate::wallet::Origin::Named,
        ) {
            Ok(i) => {
                self.wallets.set_wallet(i);
                self.inbox.clear_potential();
                self.sheet = None;
                self.refresh_spend();
                self.toast(&Text::fill(EN.inbox_is_a_wallet, &[&name]));
            }
            Err(e) => {
                if let Some(p) = self.inbox.potential.as_mut() {
                    p.error = Some(Text::of(&e.text()));
                }
            }
        }
    }

    /// Costs what `inbox_found_now` and `inbox_secret` cost. The seed being
    /// made a wallet, as the Inbox or the session holds it.
    fn potential_seed(&self, c: &Compute) -> Option<(FoundSeed, Option<Secret>)> {
        let fp = self.inbox.potential()?.fingerprint?;
        let seed = self
            .inbox_found_now(c)
            .seeds
            .into_iter()
            .find(|s| s.fingerprint == fp)?;
        let secret = seed
            .sources
            .iter()
            .find(|s| **s != SeedSource::Loaded)
            .and_then(|s| self.inbox_secret(c, s));
        Some((seed, secret))
    }

    /// Works out the fingerprint the typed passphrase gives.
    pub(crate) fn potential_refresh(&mut self) {
        let c = self.compute();
        let Some(p) = self.inbox.potential() else {
            return;
        };
        if p.xpub.is_some() {
            return;
        }
        let with = if p.passphrase.is_empty() {
            p.fingerprint
        } else {
            let pass = p.passphrase.clone();
            match self.potential_seed(&c) {
                Some((_, Some(secret))) => secret.fingerprint(&c, &pass, self.session.network()),
                Some((seed, None)) => self
                    .session
                    .keys
                    .position(seed.fingerprint)
                    .and_then(|k| self.session.keys.fingerprint_with(&c, k, &pass)),
                None => None,
            }
        };
        if let Some(p) = self.inbox.potential.as_mut() {
            p.with = with;
        }
    }

    /// Costs what `inbox_found_now` costs. The wallet here, in the Inbox or
    /// loaded, whose key the potential wallet's seed with its passphrase
    /// is.
    pub(crate) fn potential_match(&self, c: &Compute) -> Option<Text> {
        let with = self.inbox.potential()?.with?;
        self.inbox_found_now(c)
            .wallets
            .into_iter()
            .find(|w| w.keys.contains(&with))
            .map(|w| w.name)
    }

    /// Makes the potential wallet a wallet: its seed loads with the
    /// passphrase typed, if any; a wallet in the Inbox that names the key
    /// so made loads with it; else a single-key wallet of the kind chosen.
    #[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
    pub(crate) fn potential_make(&mut self) {
        let c = self.compute();
        // A watch-only wallet holds nothing secret: a stick may stay in.
        if self.inbox.potential().is_some_and(|p| p.xpub.is_some()) {
            self.xpub_make();
            return;
        }
        // A passphrase made here is typed twice: a press with the two
        // differing does nothing.
        if self.inbox.potential().is_some_and(|p| {
            crate::screens::pass_pair::differ(p.passphrase.as_str(), p.again.as_str())
        }) {
            return;
        }
        if !self.may_load_keys() {
            let error = Text::fill(EN.inbox_remove_the_first, &[self.medium.noun()]);
            if let Some(p) = self.inbox.potential.as_mut() {
                p.error = Some(error);
            }
            return;
        }
        let Some(p) = self.inbox.potential() else {
            return;
        };
        let pass = p.passphrase.clone();
        let kind = p.kind;
        let Some((seed, secret)) = self.potential_seed(&c) else {
            return;
        };
        let label = if pass.is_empty() {
            seed.label.duplicate()
        } else {
            Text::fill(EN.inbox_with_passphrase, &[&seed.label])
        };
        // A passphrase file beside the words, where none is typed.
        let kept = seed
            .sources
            .first()
            .map(|src| self.source_passphrase(&self.inbox, src))
            .unwrap_or_default();
        // The key itself: already loaded with no passphrase, or added now.
        let added = match (&secret, pass.is_empty()) {
            (_, true) if seed.loaded => Ok(seed.fingerprint),
            (Some(s), true) => s.add(&c, &kept, &label, &mut self.session.keys),
            (Some(s), _) => s.add(&c, &pass, &label, &mut self.session.keys),
            (None, false) => {
                match self
                    .session
                    .keys
                    .position(seed.fingerprint)
                    .filter(|&k| self.session.keys[k].has_words())
                {
                    Some(k) => self.session.keys.add_with_passphrase(&c, k, &pass, &label),
                    None => Err(Refusal::Words(faraday_text::Text::of(
                        EN.inbox_this_seed_was,
                    ))),
                }
            }
            (None, true) => Ok(seed.fingerprint),
        };
        let fp = match added {
            Ok(fp) => fp,
            // Loaded already with this passphrase: the wallet is what is
            // left to make, for the key the refusal names, the one the
            // passphrase made.
            Err(Refusal::Duplicate(fp)) => fp,
            Err(e) => {
                if let Some(p) = self.inbox.potential.as_mut() {
                    p.error = Some(Text::of(&e.text()));
                }
                return;
            }
        };
        // A wallet here that names the key: it is the wallet.
        if self.potential_match(&c).is_some()
            || self
                .inbox_found_now(&c)
                .wallets
                .iter()
                .any(|w| w.keys.contains(&fp))
        {
            self.inbox_load_named();
            self.inbox.clear_potential();
            self.sheet = None;
            self.toast(&Text::fill(EN.inbox_loaded_with_wallet, &[&fp_text(fp)]));
            return;
        }
        // Else a single-key wallet of the kind chosen.
        let Some(key) = self.session.keys.position(fp) else {
            return;
        };
        let text = match self.session.keys.account_text(&c, key, kind, 0) {
            Ok(t) => t,
            Err(e) => {
                if let Some(p) = self.inbox.potential.as_mut() {
                    p.error = Some(Text::of(&e));
                }
                return;
            }
        };
        let descriptor = kind.descriptor(1, &[text]);
        let name = text!(
            "{} · {}",
            &*fp_text(fp),
            crate::spend_backup::kind_name(kind)
        );
        match self.session.add_wallet(
            &c,
            &name,
            &descriptor,
            EN.inbox_a_seed_in_files,
            crate::wallet::Origin::Named,
        ) {
            Ok(i) => {
                self.wallets.set_wallet(i);
                self.inbox.clear_potential();
                self.sheet = None;
                self.refresh_spend();
                self.toast(&Text::fill(EN.inbox_is_a_wallet, &[&name]));
            }
            Err(e) => {
                if let Some(p) = self.inbox.potential.as_mut() {
                    p.error = Some(Text::of(&e.text()));
                }
            }
        }
    }
}

impl Faraday {
    /// Costs what the actions below cost, the keys' and wallets' work among
    /// it: handles the Inbox's actions (`InboxLoad`, `InboxChoose`, `InboxRemove`, `OutboxRemove`, `Potential*`, `XpubOpen`).
    pub(crate) fn inbox_act(&mut self, action: Action, c: &Compute) {
        match action {
            Action::InboxRemove(i) => {
                if i < self.inbox.len() {
                    self.inbox.remove(i);
                    self.save_boxes();
                }
            }
            Action::OutboxRemove(i) => {
                if i < self.outbox.len()
                    && crate::vault::more::outbox_may_remove(self, &self.outbox[i].name)
                {
                    let gone = self.outbox.remove(i);
                    self.visit.renamed_remove(&gone.name);
                    crate::vault::more::outbox_removed(self, &gone.name);
                    self.save_boxes();
                }
            }
            Action::InboxLoad => {
                let (w, k) = self.inbox_load();
                self.toast(&Text::fill(
                    EN.act_and_loaded,
                    &[
                        &Text::num(w as u64),
                        (if w == 1 {
                            EN.act_wallet
                        } else {
                            EN.act_wallets
                        }),
                        &Text::num(k as u64),
                        (if k == 1 { EN.act_seed } else { EN.act_seeds }),
                    ],
                ));
            }
            Action::InboxChoose(i) => {
                if let Some(w) = self.inbox_found_now(c).to_load().get(i) {
                    let d = w.descriptor.duplicate();
                    if !self.inbox.skip.remove(&d) {
                        self.inbox.skip.insert(d);
                    }
                }
            }
            Action::PotentialOpen(fp) => {
                if self.may_load_keys() {
                    self.potential_open(faraday_bip::keys::Fingerprint(fp));
                } else {
                    self.toast(&Text::fill(EN.act_remove_the_first, &[self.medium.noun()]));
                }
            }
            Action::XpubOpen(i) => self.xpub_open(i),
            Action::PotentialKind(k) => {
                if let (Some(p), Some(&kind)) = (
                    self.inbox.potential.as_mut(),
                    inbox::KINDS.get(usize::from(k)),
                ) {
                    p.kind = kind;
                }
            }
            Action::PotentialShow => {
                if let Some(p) = self.inbox.potential.as_mut() {
                    p.shown = !p.shown;
                }
            }
            Action::PotentialType => {
                if let Some(p) = self.inbox.potential.as_mut() {
                    p.typing = self.stick.is_empty();
                    p.typing_again = false;
                }
            }
            Action::PotentialTypeAgain => {
                if let Some(p) = self.inbox.potential.as_mut() {
                    p.typing = self.stick.is_empty();
                    p.typing_again = true;
                }
            }
            Action::PotentialMake => self.potential_make(),
            // `act_inner` passes only the actions matched above.
            _ => unreachable!(),
        }
    }
}

/// A file name without its extension.
pub(crate) fn stem(name: &str) -> Text {
    match name.rfind('.') {
        Some(i) if i > 0 => Text::of(&name[..i]),
        _ => Text::of(name),
    }
}

/// The stem a wallet's public files are named by: its name in lower case,
/// with anything but letters and digits as a hyphen.
pub fn file_stem(name: &str) -> Text {
    name.chars()
        .flat_map(char::to_lowercase)
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect()
}

/// The name a result file takes from the PSBT it came from: the stem,
/// without an `-unsigned` the coordinator put on it.
pub(crate) fn result_stem(source: &str) -> Text {
    let s = stem(source);
    ["-unsigned", "-partly-signed"]
        .iter()
        .find_map(|end| s.strip_suffix(end))
        .map_or_else(|| s.duplicate(), Text::of)
}
