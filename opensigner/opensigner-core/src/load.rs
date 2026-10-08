//! The Load wizard's state (UX.md §7.2): word count and language, the
//! word indices typed so far, the prefix of the word being typed, the
//! passphrase, and the key built from them; and [`LoadedKey`], the form
//! every loaded key takes once it is in the session.
//!
//! Everything secret is inline and fixed-size: word indices are `u16`
//! (`docs/PLANNING.md` §16.2), the prefix is a fixed array of keys, the passphrase
//! lives in the shared [`Finish`]. The whole struct is zeroized when the
//! wizard finishes or is cancelled; the built [`MasterKey`] erases itself
//! on drop. This file is listed in `tools/lint-secrets.sh` and may hold
//! no heap text; the views that draw the wizard live in `views/load.rs`
//! and receive only what the screen shows (the prefix, candidate words,
//! index numbers, fingerprints).
//!
//! [`MasterKey`]: osk_bip::keys::MasterKey

use osk_bip::bip39::{self, Language, MAX_WORDS, Mnemonic, Script, Typed};
use osk_bip::bitcoin::secp256k1::{PublicKey, Secp256k1};
use osk_bip::frost::SecShare;
use osk_bip::kana;
use osk_bip::keys::{Fingerprint, MasterKey, Network};
use osk_bip::slip39::{self, Share};
use osk_bip::threshold::share_fingerprint;
use osk_crypto::{
    MnemonicBytes, Sealed, SealedBytes, Secret, SeedBytes, SessionKey, Zeroize, ZeroizeOnDrop,
};
use osk_entropy::{RawHex, Strength, WORD_COUNTS};
use osk_ui::widgets::keyboard::{self, KeyInput, KeyMask, KeyboardKind};

use crate::codex32::{Codex32Entry, Submitted};
use crate::finish::{Finish, FinishStep, Material};
use crate::ids::{self, Id};
use crate::session::PinEntry;

pub use crate::finish::MASK_MS;

/// The list a word entry runs over: one of BIP-39's ten, or SLIP-39's
/// own 1024 words (`docs/PLANNING.md` §16.107 rule 7). The prefix
/// search, the candidate strip, the keyboard mask and the "Words so far"
/// panel are written once over this.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryList {
    /// A BIP-39 wordlist.
    Bip39(Language),
    /// The SLIP-39 wordlist: English, spelled on the Latin keyboard, and
    /// a word is identified by its first four letters.
    Slip39,
}

impl EntryList {
    /// The BIP-39 wordlist this is, if it is one.
    pub fn language(self) -> Option<Language> {
        match self {
            EntryList::Bip39(lang) => Some(lang),
            EntryList::Slip39 => None,
        }
    }

    /// The keyboard this list is typed on.
    pub fn keyboard(self) -> KeyboardKind {
        match self {
            EntryList::Bip39(lang) => keyboard_for(lang),
            EntryList::Slip39 => KeyboardKind::Bip39,
        }
    }

    /// The script the list is written in.
    pub fn script(self) -> Script {
        match self {
            EntryList::Bip39(lang) => lang.script(),
            EntryList::Slip39 => Script::Latin,
        }
    }

    /// Whether the list is looked up by a Mandarin reading.
    pub fn is_hanzi(self) -> bool {
        matches!(self, EntryList::Bip39(lang) if lang.is_hanzi())
    }

    /// Characters in the longest word as a reader sees it.
    pub fn max_display_chars(self) -> usize {
        match self {
            EntryList::Bip39(lang) => lang.max_display_chars(),
            EntryList::Slip39 => slip39::MAX_DISPLAY_CHARS,
        }
    }

    /// How many words the list holds.
    pub fn word_count(self) -> u16 {
        match self {
            EntryList::Bip39(_) => osk_bip::wordlists::WORDLIST_LEN as u16,
            EntryList::Slip39 => slip39::WORDLIST_LEN as u16,
        }
    }

    /// The word at `idx` in its published form.
    pub fn word(self, idx: u16) -> &'static str {
        match self {
            EntryList::Bip39(lang) => lang.word(idx),
            EntryList::Slip39 => slip39::word(idx),
        }
    }

    /// Every word whose published form starts with `prefix`, in list
    /// order. The quiz draws its decoys from this.
    pub fn candidates(self, prefix: &str) -> impl Iterator<Item = u16> + '_ {
        (0..self.word_count()).filter(move |&i| self.word(i).starts_with(prefix))
    }

    /// The word at `idx` as a reader sees it.
    pub fn word_display(self, idx: u16) -> &'static str {
        match self {
            EntryList::Bip39(lang) => lang.word_display(idx),
            EntryList::Slip39 => slip39::word(idx),
        }
    }

    /// The keys that type the word at `idx`.
    pub fn typed(self, idx: u16) -> Option<Typed> {
        match self {
            EntryList::Bip39(lang) => lang.typed(idx),
            EntryList::Slip39 => bip39::fold(slip39::word(idx)),
        }
    }

    /// Runs `f` over every word whose typed form starts with `prefix`,
    /// in list order, stopping when it returns `false`.
    fn each_typed(self, prefix: &[char], mut f: impl FnMut(u16) -> bool) {
        match self {
            EntryList::Bip39(lang) => {
                for i in lang.candidates_typed(prefix) {
                    if !f(i) {
                        return;
                    }
                }
            }
            EntryList::Slip39 => {
                for i in 0..slip39::WORDLIST_LEN as u16 {
                    if self.typed(i).is_some_and(|t| t.starts_with(prefix)) && !f(i) {
                        return;
                    }
                }
            }
        }
    }

    /// Whether any word's typed form starts with `prefix`.
    fn any_typed(self, prefix: &[char]) -> bool {
        let mut any = false;
        self.each_typed(prefix, |_| {
            any = true;
            false
        });
        any
    }
}

/// The keyboard a wordlist is typed on. The two Chinese lists are
/// looked up by a Mandarin reading rather than spelled, so they have
/// keyboards of their own: pinyin for Simplified, 注音 for Traditional.
pub fn keyboard_for(lang: Language) -> KeyboardKind {
    match lang.script() {
        Script::Latin => KeyboardKind::Bip39,
        Script::Kana => KeyboardKind::Kana,
        Script::Jamo => KeyboardKind::Jamo,
        Script::Pinyin => KeyboardKind::Pinyin,
        Script::Zhuyin => KeyboardKind::Zhuyin,
    }
}

/// Most candidates the word-entry step lists on a spelled list, whose
/// cell is as wide as the widest word it can hold.
pub const MAX_CANDIDATES: usize = 8;

/// Most it lists on a list whose words are one character: two rows of
/// ten at the keyboard's key pitch, which is the largest tone group
/// either Chinese list has (`docs/PLANNING.md` §16.44, §16.45).
pub const MAX_CANDIDATES_HANZI: usize =
    osk_ui::tokens::CANDIDATE_ROWS_HANZI * osk_ui::tokens::CANDIDATE_CELLS_HANZI;

/// Candidates the strip holds for `lang`. A character is one glyph, so a
/// Chinese strip holds a whole tone group; a spelled word needs a cell
/// wide enough for "abandon", so it holds [`MAX_CANDIDATES`].
pub fn max_candidates(lang: Language) -> usize {
    if lang.max_display_chars() == 1 {
        MAX_CANDIDATES_HANZI
    } else {
        MAX_CANDIDATES
    }
}

/// Bytes a syllable's spelling takes at most: sixteen keys, and a 注音
/// key is three bytes of UTF-8.
const SPELLING_BYTES: usize = bip39::MAX_TYPED * 3;

/// What the keys typed mean on a keyboard that looks a character up by
/// its Mandarin reading: the syllable's spelling in the list's script,
/// and the tone once the tone key has been pressed. The tone key is
/// always the last one — a digit on the pinyin keyboard, one of the
/// five marks on the 注音 one — so the split needs no state.
#[derive(Clone, Copy)]
struct Reading {
    buf: [u8; SPELLING_BYTES],
    len: usize,
    tone: Option<u8>,
}

impl Reading {
    fn of(kind: KeyboardKind, keys: &[char]) -> Self {
        let mut r = Reading {
            buf: [0; SPELLING_BYTES],
            len: 0,
            tone: None,
        };
        for (i, &c) in keys.iter().enumerate() {
            if i + 1 == keys.len()
                && let Some(tone) = tone_of(kind, c)
            {
                r.tone = Some(tone);
                break;
            }
            let mut bytes = [0u8; 4];
            let encoded = c.encode_utf8(&mut bytes);
            if r.len + encoded.len() <= SPELLING_BYTES {
                r.buf[r.len..r.len + encoded.len()].copy_from_slice(encoded.as_bytes());
                r.len += encoded.len();
            }
        }
        r
    }

    fn spelling(&self) -> &str {
        core::str::from_utf8(&self.buf[..self.len]).unwrap_or("")
    }
}

/// The tone a key stands for, or `None` for a key that spells.
fn tone_of(kind: KeyboardKind, c: char) -> Option<u8> {
    match kind {
        KeyboardKind::Pinyin => ('1'..='5').contains(&c).then(|| c as u8 - b'0'),
        KeyboardKind::Zhuyin => keyboard::ZHUYIN_MARKS
            .iter()
            .position(|&m| m == c)
            .map(|i| i as u8 + 1),
        _ => None,
    }
}

/// The mask bit of the key that gives `tone`. Every tone has a key on
/// both Chinese keyboards: a digit on the pinyin one, a mark on the
/// 注音 one, ˉ included.
fn tone_bit(kind: KeyboardKind, tone: u8) -> KeyMask {
    match kind {
        KeyboardKind::Pinyin if (1..=5).contains(&tone) => {
            keyboard::key_bit(kind, char::from(b'0' + tone))
        }
        KeyboardKind::Zhuyin if (1..=5).contains(&tone) => {
            keyboard::key_bit(kind, keyboard::ZHUYIN_MARKS[usize::from(tone) - 1])
        }
        _ => 0,
    }
}

/// Whether `kind` looks a word up by its reading rather than spelling it.
fn reads(kind: KeyboardKind) -> bool {
    matches!(kind, KeyboardKind::Pinyin | KeyboardKind::Zhuyin)
}

/// Number of steps the wizard's indicator shows. The session PIN step,
/// shown only for the first key of a session, is one more
/// ([`crate::OpenSigner::wizard_steps`]).
pub const STEPS: u8 = 7;

/// What the source step's checked row is. The two scanning sources open
/// the scanner rather than the rest of the wizard, so the tap that
/// confirms them leaves it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    /// "Type the words".
    Type,
    /// "Scan a SeedQR": a seed code, in either form.
    SeedCode,
    /// "Encrypted backup": an `osk-backup` and the passphrase that
    /// opens it.
    Backup,
    /// "Seed XOR parts": the parts typed one after another and XORed
    /// back into the key they were split from.
    SeedXor,
    /// "SLIP-39 shares": the shares of a backup typed one after another
    /// until the set opens its master secret (`docs/PLANNING.md`
    /// §16.107).
    Slip39,
    /// "Codex32": one string, or the shares of a split of one, typed on
    /// the bech32 keyboard until the secret is in hand
    /// (`docs/PLANNING.md` §16.109).
    Codex32,
    /// "Word numbers": the same words, each typed as its 1-based number
    /// on the wordlist — the number a backup written in numbers carries
    /// (`docs/PLANNING.md` §16.125 rule 2).
    Numbers,
    /// "Hex entropy": the key's entropy typed as hex digits, the words
    /// following from it and the language (§16.125 rule 3).
    Hex,
}

/// Where the wizard is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// Where the words come from (typing is the only source yet).
    Source,
    /// 12, 15, 18, 21 or 24 words.
    Count,
    /// Which wordlist.
    Language,
    /// Typing the words.
    Words,
    /// Checksum result.
    Checksum,
    /// "Add a passphrase?"
    PassphraseOffer,
    /// Typing the passphrase.
    Passphrase,
    /// Fingerprints without and with the passphrase.
    PassphraseConfirm,
    /// The final fingerprint and the hold to add.
    Confirm,
    /// Setting the session PIN (first key of the session only).
    Pin,
    /// Repeating the session PIN.
    PinConfirm,
}

impl Step {
    /// 1-based position on the step indicator. The three passphrase
    /// screens share step 6; the two PIN screens share step 8.
    pub fn number(self) -> u8 {
        match self {
            Step::Source => 1,
            Step::Count => 2,
            Step::Language => 3,
            Step::Words => 4,
            Step::Checksum => 5,
            Step::PassphraseOffer | Step::Passphrase | Step::PassphraseConfirm => 6,
            Step::Confirm => 7,
            Step::Pin | Step::PinConfirm => 8,
        }
    }

    /// The shared finishing step this is, if any.
    pub fn finish(self) -> Option<FinishStep> {
        match self {
            Step::PassphraseOffer => Some(FinishStep::PassphraseOffer),
            Step::Passphrase => Some(FinishStep::Passphrase),
            Step::PassphraseConfirm => Some(FinishStep::PassphraseConfirm),
            Step::Confirm => Some(FinishStep::Confirm),
            Step::Pin => Some(FinishStep::Pin),
            Step::PinConfirm => Some(FinishStep::PinConfirm),
            _ => None,
        }
    }

    fn from_finish(f: FinishStep) -> Self {
        match f {
            FinishStep::PassphraseOffer => Step::PassphraseOffer,
            FinishStep::Passphrase => Step::Passphrase,
            FinishStep::PassphraseConfirm => Step::PassphraseConfirm,
            FinishStep::Confirm => Step::Confirm,
            FinishStep::Pin => Step::Pin,
            FinishStep::PinConfirm => Step::PinConfirm,
        }
    }
}

/// Words the entry step holds at once: a SLIP-39 share's 33, which is
/// more than BIP-39's 24.
pub const ENTRY_WORDS: usize = slip39::MAX_WORDS;

/// A SLIP-39 share's two word counts, in the order the screen lists
/// them: 20 words carry a 128-bit master secret, 33 a 256-bit one.
pub const SLIP39_COUNTS: [u8; 2] = [slip39::MIN_WORDS as u8, slip39::MAX_WORDS as u8];

/// Shares the wizard holds at once. SLIP-39's own thresholds and counts
/// run 1 to 16, so sixteen is the most any one group can ask for, and
/// the wizard stops asking the moment the set is enough. Room for every
/// share the format allows to be present at once — sixteen groups of
/// sixteen — would be 256 shares and some 33 KB of secret sitting in the
/// wizard; a backup that needs more than sixteen shares in all is
/// refused on the seventeenth instead.
pub const MAX_SHARES: usize = slip39::MAX_COUNT;

/// Why a share was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShareRefusal {
    /// The codec's own reason: the checksum, or a share that does not
    /// belong with the ones already in.
    Codec(slip39::Error),
    /// The set already holds [`MAX_SHARES`].
    TooMany,
}

/// The shares typed so far. Fixed size, zeroized on drop, and never
/// more than [`MAX_SHARES`].
struct ShareSet {
    shares: [Share; MAX_SHARES],
    count: u8,
}

impl ShareSet {
    fn new() -> Self {
        ShareSet {
            shares: core::array::from_fn(|_| Share::default()),
            count: 0,
        }
    }

    fn len(&self) -> usize {
        usize::from(self.count)
    }

    fn iter(&self) -> impl Iterator<Item = &Share> {
        self.shares[..self.len()].iter()
    }

    /// Whether `share` belongs with the ones already in: the checks
    /// [`slip39::recover`] makes over a whole set, made one share at a
    /// time so that the share that does not fit is the one refused.
    fn admits(&self, share: &Share) -> Result<(), ShareRefusal> {
        use slip39::Error;
        if share.group_threshold() > share.group_count() {
            return Err(ShareRefusal::Codec(Error::GroupThresholdExceedsCount));
        }
        if share.group_index() >= share.group_count() {
            return Err(ShareRefusal::Codec(Error::GroupIndexOutOfRange));
        }
        for held in self.iter() {
            let reason = if held.identifier() != share.identifier() {
                Error::MismatchedIdentifiers
            } else if held.is_extendable() != share.is_extendable() {
                Error::MismatchedExtendable
            } else if held.iteration_exponent() != share.iteration_exponent() {
                Error::MismatchedIterationExponents
            } else if held.group_threshold() != share.group_threshold() {
                Error::MismatchedGroupThresholds
            } else if held.group_count() != share.group_count() {
                Error::MismatchedGroupCounts
            } else if held.secret_len() != share.secret_len() {
                Error::MismatchedValueLengths
            } else if held.group_index() != share.group_index() {
                continue;
            } else if held.member_threshold() != share.member_threshold() {
                Error::MismatchedMemberThresholds
            } else if held.member_index() == share.member_index() {
                Error::DuplicateMemberIndex
            } else {
                continue;
            };
            return Err(ShareRefusal::Codec(reason));
        }
        if self.len() == MAX_SHARES {
            return Err(ShareRefusal::TooMany);
        }
        Ok(())
    }

    fn push(&mut self, share: Share) {
        let n = self.len();
        if n < MAX_SHARES {
            self.shares[n] = share;
            self.count += 1;
        }
    }

    /// Drops the share last accepted, which is what Back on its result
    /// undoes.
    fn pop(&mut self) {
        if self.count > 0 {
            self.count -= 1;
            self.shares[self.len()].zeroize();
        }
    }

    /// How many groups must be present, once any share says so.
    fn group_threshold(&self) -> Option<u8> {
        self.iter().next().map(Share::group_threshold)
    }

    /// Every group seen, in the order first met: its 0-based index, the
    /// shares of it that are in, and the shares it needs.
    fn groups(&self) -> impl Iterator<Item = (u8, u8, u8)> + '_ {
        self.iter().enumerate().filter_map(move |(i, share)| {
            let gi = share.group_index();
            if self.iter().take(i).any(|s| s.group_index() == gi) {
                return None;
            }
            let held = self.iter().filter(|s| s.group_index() == gi).count() as u8;
            Some((gi, held, share.member_threshold()))
        })
    }

    /// Groups that have every share they need.
    fn complete_groups(&self) -> u8 {
        self.groups().filter(|(_, held, need)| held >= need).count() as u8
    }

    /// Whether the set opens the master secret: enough complete groups.
    fn enough(&self) -> bool {
        match self.group_threshold() {
            Some(t) => self.complete_groups() >= t,
            None => false,
        }
    }

    /// Runs `f` on exactly the shares [`slip39::recover`] takes: the
    /// first complete groups up to the threshold, and the first shares
    /// of each up to its own. The copies are erased when the call ends.
    fn with_recovery_set<R>(&self, f: impl FnOnce(&[Share]) -> R) -> Option<R> {
        let threshold = self.group_threshold()?;
        let mut picked: [Share; MAX_SHARES] = core::array::from_fn(|_| Share::default());
        let mut n = 0;
        let mut groups = 0;
        for (gi, held, need) in self.groups() {
            if held < need || groups == threshold {
                continue;
            }
            groups += 1;
            for share in self
                .iter()
                .filter(|s| s.group_index() == gi)
                .take(usize::from(need))
            {
                picked[n] = share.clone();
                n += 1;
            }
        }
        (groups == threshold).then(|| f(&picked[..n]))
    }
}

impl Zeroize for ShareSet {
    fn zeroize(&mut self) {
        for share in &mut self.shares {
            share.zeroize();
        }
        self.count = 0;
    }
}

impl Drop for ShareSet {
    fn drop(&mut self) {
        self.zeroize();
    }
}

/// A secret of a loaded key: sealed under the session key, or still in
/// plaintext while the session key is weak (`docs/PLANNING.md` §16.21:
/// nothing is sealed under a key without shell entropy). Either way it
/// zeroizes on drop.
enum Held<T: SealedBytes> {
    Plain(Secret<T>),
    Sealed(Sealed<T>),
}

impl<T: SealedBytes> Held<T> {
    fn with<R>(&self, key: &SessionKey, f: impl FnOnce(&T) -> R) -> Option<R> {
        match self {
            Held::Plain(s) => Some(s.with(f)),
            Held::Sealed(s) => s.with(key, f).ok(),
        }
    }

    /// Seals a plaintext value under `key`; a weak key leaves it as it
    /// is, and a sealed value is untouched.
    fn seal(&mut self, key: &mut SessionKey) {
        if let Held::Plain(s) = self
            && !key.is_weak()
        {
            let mut buf = T::ZEROED;
            s.with(|v| v.write_bytes(&mut buf));
            let value = T::from_bytes(&buf);
            buf.zeroize();
            if let Ok(sealed) = Sealed::seal(key, value) {
                *self = Held::Sealed(sealed);
            }
        }
    }

    /// Re-seals a sealed value under `new` (`old` opens it), or seals a
    /// plaintext one.
    fn reseal(&mut self, old: &SessionKey, new: &mut SessionKey) {
        match self {
            Held::Plain(_) => self.seal(new),
            Held::Sealed(s) => {
                if let Ok(sealed) = s.reseal(old, new) {
                    *self = Held::Sealed(sealed);
                }
            }
        }
    }

    fn is_sealed(&self) -> bool {
        matches!(self, Held::Sealed(_))
    }
}

/// What a loaded key was read from, which is the one fact its page
/// states and the fact its Backup menu is built from. A key with words
/// is [`KeyKind::Words`]; the two kinds without words are told apart
/// here, because a 16- or 32-byte seed could have come from either
/// (`docs/PLANNING.md` §16.107 rule 1, §16.109 rule 1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyKind {
    /// BIP-39 words, typed, scanned, created or derived.
    Words,
    /// A SLIP-39 master secret, which is the seed itself.
    Slip39,
    /// A codex32 master seed, 16 to 64 bytes.
    Codex32,
}

/// A key held in RAM (`docs/PLANNING.md` §5.1, §16.21). The seed and the
/// words are sealed under the session key; the [`MasterKey`] exists only
/// while the session is unlocked and is rebuilt from the seed on unlock.
/// The passphrase is never kept: the seed already includes it. Nothing
/// here can be printed; everything erases itself on drop.
pub struct LoadedKey {
    /// The BIP-32 master for the current network setting, while
    /// unlocked.
    pub master: Option<MasterKey>,
    /// The master fingerprint (with the passphrase applied, if any).
    pub fingerprint: Fingerprint,
    /// Whether a passphrase was applied.
    pub has_passphrase: bool,
    /// The network `master` is keyed for.
    pub network: Network,
    /// The public share this key is, when its words are 24 and their 32
    /// bytes are a scalar the curve takes, computed once at load so that
    /// matching a key against a FROST record's members costs nothing on
    /// any screen. `None` for every other key (`docs/PLANNING.md`
    /// section 16.104 rule 3).
    pub share: Option<PublicKey>,
    /// hash160 of that public share, first four bytes: the fingerprint a
    /// FROST wallet's Keys review shows, and the one printed beside the
    /// words this device dealt.
    pub share_fingerprint: Option<Fingerprint>,
    /// The wordlist of the words, when they are held.
    pub language: Language,
    /// Whether the backup quiz has been passed for this key.
    pub backup_verified: bool,
    /// Whether this key was derived from another loaded key — a
    /// passphrase key opened from its words, or a BIP-85 child. A
    /// derived key lives in memory for the session and is never written
    /// to the device (`docs/PLANNING.md` §16.67).
    pub derived: bool,
    /// The BIP-32 seed: 64 bytes from BIP-39 words, or the 16 or 32
    /// bytes of a SLIP-39 master secret, which is the seed itself
    /// (`docs/PLANNING.md` §16.107). One door, whatever the key was made
    /// from.
    seed: Held<SeedBytes>,
    /// What the key was read from.
    kind: KeyKind,
    /// The seed's length in bytes, which the key states without opening
    /// anything: 64 from words, 16 or 32 from a SLIP-39 master secret,
    /// any of BIP 93's six from codex32. What the device can keep and
    /// what a backup can be written as both follow from it.
    seed_len: u8,
    /// The words, kept in the temporary (RAM-only) storage mode, the only
    /// mode so far, so that the backup screens, the quiz and SeedQR
    /// export can use them (`docs/PLANNING.md` §16.19). `None` once a
    /// storage mode that seals the key on disk lands.
    mnemonic: Option<Held<MnemonicBytes>>,
}

impl LoadedKey {
    /// A key from its seed and the master derived from it. `mnemonic`
    /// is consumed into sealable form.
    pub fn new(
        seed: Secret<SeedBytes>,
        master: MasterKey,
        mnemonic: Option<Mnemonic>,
        has_passphrase: bool,
        backup_verified: bool,
    ) -> Self {
        let language = mnemonic
            .as_ref()
            .map_or(Language::English, Mnemonic::language);
        // A FROST member is the plain 24 words. A passphrase key is
        // made from the same words and would carry the same public
        // share as its parent, which would make one member two keys;
        // it carries none.
        let share = if has_passphrase {
            None
        } else {
            mnemonic.as_ref().and_then(public_share)
        };
        let kind = if mnemonic.is_some() {
            KeyKind::Words
        } else {
            KeyKind::Slip39
        };
        let seed_len = seed.with(|s| s.as_bytes().len()) as u8;
        let mnemonic = mnemonic.map(|m| Held::Plain(Secret::new(mnemonic_bytes(&m))));
        LoadedKey {
            fingerprint: master.fingerprint(),
            network: master.network(),
            share,
            share_fingerprint: share.as_ref().map(share_fingerprint),
            master: Some(master),
            has_passphrase,
            language,
            backup_verified,
            derived: false,
            kind,
            seed_len,
            seed: Held::Plain(seed),
            mnemonic,
        }
    }

    /// Marks the key as read from a codex32 string, which has no words
    /// and is not a SLIP-39 master secret.
    pub fn codex32(mut self) -> Self {
        self.kind = KeyKind::Codex32;
        self
    }

    /// What the key was read from.
    pub fn kind(&self) -> KeyKind {
        self.kind
    }

    /// The seed's length in bytes.
    pub fn seed_len(&self) -> usize {
        usize::from(self.seed_len)
    }

    /// Marks the key as derived from another loaded key.
    pub fn derived(mut self) -> Self {
        self.derived = true;
        self
    }

    /// Seals the seed and the words under `key`. Plaintext stays
    /// plaintext while `key` is weak.
    ///
    /// This is also where the master key's curve context takes the
    /// session's blind: a key meets the session key here, on the way in,
    /// on unlock and on every rotation, so it is the one place that has
    /// both (security review M1).
    pub fn seal(&mut self, key: &mut SessionKey) {
        self.blind(key);
        self.seed.seal(key);
        if let Some(m) = &mut self.mnemonic {
            m.seal(key);
        }
    }

    /// Randomizes the master key's curve context from the session key.
    fn blind(&mut self, key: &SessionKey) {
        if let Some(m) = &mut self.master {
            m.reblind(&key.secp_blind());
        }
    }

    /// Re-seals the seed and the words after a rotation from `old` to
    /// `key`, sealing what was still plaintext.
    pub fn reseal(&mut self, old: &SessionKey, key: &mut SessionKey) {
        self.blind(key);
        self.seed.reseal(old, key);
        if let Some(m) = &mut self.mnemonic {
            m.reseal(old, key);
        }
    }

    /// Whether the seed is sealed (rather than plaintext under a weak
    /// session key).
    pub fn is_sealed(&self) -> bool {
        self.seed.is_sealed()
    }

    /// Drops the live master key. The sealed seed stays.
    pub fn lock(&mut self) {
        self.master = None;
    }

    /// Rebuilds the master key from the sealed seed for this key's
    /// network. `false` if the seed does not open under `key`.
    pub fn unlock(&mut self, key: &SessionKey) -> bool {
        let network = self.network;
        let blind = key.secp_blind();
        match self.seed.with(key, |s| {
            let seed = Secret::new(SeedBytes::new(s.as_bytes())?);
            Some(MasterKey::from_seed_bytes(&seed, network).blinded(&blind))
        }) {
            Some(Some(m)) => {
                self.master = Some(m);
                true
            }
            _ => false,
        }
    }

    /// Runs `f` on the seed, unsealed for the duration of the call: 64
    /// bytes from words, 16 or 32 from a SLIP-39 master secret. `None`
    /// when it does not open under `key`. The explorer's seed row
    /// (UX.md H2) and the keys record are the readers besides
    /// [`unlock`].
    ///
    /// [`unlock`]: Self::unlock
    pub fn seed<R>(&self, key: &SessionKey, f: impl FnOnce(&[u8]) -> R) -> Option<R> {
        self.seed.with(key, |s| f(s.as_bytes()))
    }

    /// Whether the words are held.
    pub fn has_mnemonic(&self) -> bool {
        self.mnemonic.is_some()
    }

    /// Whether the device can keep this key: its words, or a SLIP-39
    /// key's master secret. A SLIP-39 key read under a passphrase is
    /// never in the blob (`docs/PLANNING.md` §16.104 rule 7, §16.107
    /// rule 3), because the passphrase is applied when the shares are
    /// read and the secret already carries it.
    pub fn is_keepable(&self) -> bool {
        if self.has_mnemonic() {
            return true;
        }
        // A key with no words is kept as its seed, in the room a slot
        // has beside the word count (`crate::keep`): a 64-byte codex32
        // seed does not fit and is not kept (§16.109 rule 1).
        !self.has_passphrase && osk_keep::keeps_secret(self.seed_len())
    }

    /// Runs `f` on the words, rebuilt from their sealed form for the
    /// duration of the call. `None` when they are not held or do not
    /// open under `key`.
    pub fn mnemonic<R>(&self, key: &SessionKey, f: impl FnOnce(&Mnemonic) -> R) -> Option<R> {
        self.mnemonic.as_ref()?.with(key, |b| {
            let lang = Language::ALL
                .get(usize::from(b.language))
                .copied()
                .unwrap_or(Language::English);
            Mnemonic::from_indices(lang, &b.words[..usize::from(b.len).min(MAX_WORDS)])
                .ok()
                .map(|m| f(&m))
        })?
    }

    /// Runs `f` on the secret share this key is, which exists only for
    /// the duration of the call. `None` unless [`share`] is set; this is
    /// the only door the scalar comes through.
    ///
    /// [`share`]: Self::share
    pub fn secret_share<R>(&self, key: &SessionKey, f: impl FnOnce(&SecShare) -> R) -> Option<R> {
        self.share?;
        self.mnemonic(key, |m| {
            let mut bytes = entropy32(m)?;
            let share = SecShare::from_bytes(&bytes).ok();
            bytes.zeroize();
            share.map(|s| f(&s))
        })?
    }
}

/// The 32 bytes of a 24-word phrase's entropy. `None` for any other
/// word count.
fn entropy32(m: &Mnemonic) -> Option<[u8; 32]> {
    if m.word_count() != 24 {
        return None;
    }
    let entropy = m.entropy();
    let exposed = entropy.expose();
    let exposed = exposed.as_bytes();
    if exposed.len() != 32 {
        return None;
    }
    let mut bytes = [0u8; 32];
    bytes.copy_from_slice(exposed);
    Some(bytes)
}

/// The public share 24 words state, when their bytes are a scalar the
/// curve takes.
fn public_share(m: &Mnemonic) -> Option<PublicKey> {
    let mut bytes = entropy32(m)?;
    let share = SecShare::from_bytes(&bytes).ok();
    bytes.zeroize();
    let secp = Secp256k1::verification_only();
    Some(share?.public_share(&secp))
}

/// The sealable form of a mnemonic: indices, count and the language's
/// position in `Language::ALL`.
pub(crate) fn mnemonic_bytes(m: &Mnemonic) -> MnemonicBytes {
    let mut words = [0u16; MAX_WORDS];
    words[..m.word_count()].copy_from_slice(m.indices());
    MnemonicBytes {
        words,
        len: m.word_count() as u8,
        language: Language::ALL
            .iter()
            .position(|l| *l == m.language())
            .unwrap_or(0) as u8,
    }
}

/// Wizard state. See the module documentation.
pub struct LoadWizard {
    step: Step,
    count: u8,
    lang: Language,
    words: [u16; ENTRY_WORDS],
    committed: u8,
    /// When fixing one word after a checksum failure: the word count to
    /// restore once that word is re-committed.
    resume_at: Option<u8>,
    prefix: Typed,
    /// How many candidates the strip has paged past. A syllable without
    /// a tone leaves dozens of characters, so the strip pages; every key
    /// puts it back on the first page.
    page: u16,
    /// A candidate is selected by one tap and accepted by the next
    /// (`docs/PLANNING.md` §16.50). Set from the size class: on the
    /// 240 × 320 panel a cell is 24 px and a fingertip covers it, so the
    /// typist is shown what they hit before it is taken; on the larger
    /// classes a tap accepts.
    two_tap: bool,
    /// The candidate a tap has selected, as its wordlist index. The next
    /// tap on it accepts it; a tap on another cell moves the selection;
    /// a key clears it.
    selected: Option<u16>,
    /// Bitmask over the wordlist of valid final words, computed when the
    /// penultimate word is committed.
    allowed_last: Option<[u64; 32]>,
    /// Whether the last word is any word of the list rather than one
    /// the BIP-39 checksum allows. An LND cipher seed's words carry no
    /// BIP-39 checksum at all (`docs/PLANNING.md` §16.116).
    free_last: bool,
    checksum_ok: bool,
    /// Bit `p` set: a single-word substitution at position `p` (0-based)
    /// among similar words makes the checksum pass.
    suspects: u32,
    /// The words came from a scan (SeedQR or plain text): the entry step
    /// is skipped and the language step leads straight to the checksum.
    scanned: bool,
    /// The source step's checked row. A choice is checked, then
    /// confirmed (§2.7), so the tap that checks does not open the
    /// scanner.
    source: Source,
    /// The SLIP-39 shares typed so far, when the source is
    /// [`Source::Slip39`].
    shares: ShareSet,
    /// Why the last share was refused, on the result that says so.
    share_error: Option<ShareRefusal>,
    /// The codex32 string being typed and the strings already taken,
    /// when the source is [`Source::Codex32`].
    codex: Codex32Entry,
    /// The hex digits typed, when the source is [`Source::Hex`].
    hex: RawHex,
    /// When the last hex digit was typed, which is when it masks
    /// (§4.10).
    typed_at: u64,
    finish: Finish,
}

impl Zeroize for LoadWizard {
    fn zeroize(&mut self) {
        self.words.zeroize();
        self.committed = 0;
        self.resume_at = None;
        self.prefix.zeroize();
        self.page = 0;
        self.selected = None;
        if let Some(mask) = &mut self.allowed_last {
            mask.zeroize();
        }
        self.allowed_last = None;
        self.checksum_ok = false;
        self.suspects = 0;
        self.scanned = false;
        self.source = Source::Type;
        self.shares.zeroize();
        self.share_error = None;
        self.codex.zeroize();
        self.hex.zeroize();
        self.typed_at = 0;
        self.finish.zeroize();
    }
}

impl Drop for LoadWizard {
    fn drop(&mut self) {
        self.zeroize();
    }
}

impl ZeroizeOnDrop for LoadWizard {}

impl Default for LoadWizard {
    fn default() -> Self {
        Self::new()
    }
}

impl LoadWizard {
    /// A wizard at its first step: 12 English words.
    pub fn new() -> Self {
        LoadWizard {
            step: Step::Source,
            count: 12,
            lang: Language::English,
            words: [0; ENTRY_WORDS],
            committed: 0,
            resume_at: None,
            prefix: Typed::new(),
            page: 0,
            two_tap: false,
            selected: None,
            allowed_last: None,
            free_last: false,
            checksum_ok: false,
            suspects: 0,
            scanned: false,
            source: Source::Type,
            shares: ShareSet::new(),
            share_error: None,
            codex: Codex32Entry::new(),
            hex: RawHex::new(),
            typed_at: 0,
            finish: Finish::new(),
        }
    }

    /// A wizard whose words are already known from a scan (UX.md §4:
    /// SeedQR and plain-text words skip the entry step): it starts at the
    /// language step with `indices` committed, and its checksum runs when
    /// the language is confirmed. `indices` must be 12–24 long; more are
    /// ignored, fewer leave the wizard at its first step.
    pub fn from_words(indices: &[u16], lang: Language) -> Self {
        let mut w = Self::new();
        if !WORD_COUNTS.contains(&(indices.len() as u8)) {
            return w;
        }
        let n = indices.len();
        w.words[..n].copy_from_slice(&indices[..n]);
        w.count = n as u8;
        w.committed = n as u8;
        w.lang = lang;
        w.scanned = true;
        // A decoded seed code necessarily has a valid checksum, so the
        // wizard lands on the confirm screen and the language is a row
        // on it (UX review 2026-09-07, Q4).
        w.check();
        w
    }

    /// The source step's checked row.
    pub fn source(&self) -> Source {
        self.source
    }

    /// Whether the words came from a scan.
    pub fn scanned(&self) -> bool {
        self.scanned
    }

    /// The current step.
    pub fn step(&self) -> Step {
        self.step
    }

    /// Moves to `step` without touching any state.
    pub fn go(&mut self, step: Step) {
        self.step = step;
    }

    /// Word count.
    pub fn count(&self) -> u8 {
        self.count
    }

    /// Sets the word count: 12, 15, 18, 21 or 24 words of a key, or 20
    /// or 33 words of a SLIP-39 share. Ignored otherwise.
    pub fn set_count(&mut self, count: u8) {
        if self.counts().contains(&count) {
            self.count = count;
        }
    }

    /// The word counts this source offers, in the order the screen
    /// lists them.
    pub fn counts(&self) -> &'static [u8] {
        if self.is_slip39() {
            &SLIP39_COUNTS
        } else {
            &WORD_COUNTS
        }
    }

    /// Whether this wizard is reading SLIP-39 shares.
    pub fn is_slip39(&self) -> bool {
        self.source == Source::Slip39
    }

    /// Whether this wizard is reading codex32 strings.
    pub fn is_codex32(&self) -> bool {
        self.source == Source::Codex32
    }

    /// Whether the words are being typed as their numbers
    /// (`docs/PLANNING.md` §16.125 rule 2). The words, the count, the
    /// language, the checksum and the key are the typed words' own; only
    /// the keys that reach a word differ.
    pub fn is_numbers(&self) -> bool {
        self.source == Source::Numbers
    }

    /// Whether the key is being loaded from its entropy in hex
    /// (§16.125 rule 3).
    pub fn is_hex(&self) -> bool {
        self.source == Source::Hex
    }

    /// How many hex digits of entropy are typed so far. The digits
    /// themselves are the key; the count is not.
    pub fn hex_len(&self) -> usize {
        self.hex.len()
    }

    /// Hex digits the word count asks for: 32, 40, 48, 56 or 64.
    pub fn hex_needed(&self) -> usize {
        Strength::for_words(usize::from(self.count)).map_or(0, Strength::hex_digits)
    }

    /// Whether the digits in hand are the entropy the count asks for,
    /// which is when the entry can be submitted.
    pub fn hex_ready(&self) -> bool {
        self.hex_needed() > 0 && self.hex.len() == self.hex_needed()
    }

    /// The last hex digit while it is still unmasked at `now_ms`
    /// (`docs/DESIGN.md` §4.10).
    pub fn hex_visible_last(&self, now_ms: u64) -> Option<char> {
        if self.hex.is_empty() || now_ms >= self.typed_at + MASK_MS {
            return None;
        }
        self.hex.last()
    }

    /// The codex32 entry, which the screens read.
    pub fn codex32(&self) -> &Codex32Entry {
        &self.codex
    }

    /// The list the words step runs over.
    pub fn list(&self) -> EntryList {
        if self.is_slip39() {
            EntryList::Slip39
        } else {
            EntryList::Bip39(self.lang)
        }
    }

    /// The 1-based number of the share being typed.
    pub fn share_number(&self) -> usize {
        self.shares.len() + 1
    }

    /// Why the share just typed was refused, on the result that says so.
    pub fn share_error(&self) -> Option<ShareRefusal> {
        self.share_error
    }

    /// Groups the set has seen, in the order first met: the group's
    /// 1-based number, the shares of it that are in, and the shares it
    /// needs.
    pub fn share_groups(&self) -> impl Iterator<Item = (u8, u8, u8)> + '_ {
        self.shares
            .groups()
            .map(|(gi, held, need)| (gi + 1, held, need))
    }

    /// Groups that are complete, and groups that must be.
    pub fn share_group_progress(&self) -> (u8, u8) {
        (
            self.shares.complete_groups(),
            self.shares.group_threshold().unwrap_or(1),
        )
    }

    /// Whether the shares in hand open the master secret.
    pub fn shares_enough(&self) -> bool {
        self.shares.enough()
            && self
                .shares
                .with_recovery_set(|set| slip39::recover(set, b"").is_ok())
                == Some(true)
    }

    /// Wordlist.
    pub fn language(&self) -> Language {
        self.lang
    }

    /// Lets any word of the list stand last, which is what an LND
    /// cipher seed needs: its words carry no BIP-39 checksum
    /// (`docs/PLANNING.md` §16.116).
    pub fn set_free_last(&mut self, free: bool) {
        self.free_last = free;
    }

    /// Sets the wordlist.
    pub fn set_language(&mut self, lang: Language) {
        self.lang = lang;
    }

    /// The shared finishing state.
    pub fn finish(&self) -> &Finish {
        &self.finish
    }

    /// Goes one step back. Returns `false` at the first step, where going
    /// back means cancelling the wizard. State that belongs to the step
    /// being left is zeroized.
    pub fn back(&mut self) -> bool {
        self.step = match self.step {
            Step::Source => return false,
            Step::Count => Step::Source,
            Step::Language if self.scanned => Step::Checksum,
            Step::Language => Step::Count,
            Step::Words if self.is_codex32() => {
                self.codex.zeroize();
                Step::Source
            }
            Step::Words if self.is_slip39() => {
                self.clear_words();
                Step::Count
            }
            Step::Words if self.is_hex() => {
                self.hex.zeroize();
                self.typed_at = 0;
                Step::Language
            }
            Step::Words => {
                self.clear_words();
                Step::Language
            }
            // A codex32 result goes back to the string it was about,
            // emptied; the strings already in are untouched.
            Step::Checksum if self.is_codex32() => {
                self.codex.clear_string();
                Step::Words
            }
            // A share's result goes back to the share's own words,
            // cleared, and the shares already in are untouched; an
            // accepted share is given back first.
            Step::Checksum if self.is_slip39() => {
                if self.share_error.take().is_none() {
                    self.shares.pop();
                }
                self.clear_words();
                Step::Words
            }
            // The entropy's words are computed, not typed, so the step
            // before the result is the digits, emptied (§16.125 rule 3).
            Step::Checksum if self.is_hex() => {
                self.clear_words();
                self.hex.zeroize();
                self.typed_at = 0;
                Step::Words
            }
            // Scanned words have no step before the confirm.
            Step::Checksum if self.scanned => return false,
            Step::Checksum => {
                // Re-type the last word.
                self.uncommit();
                Step::Words
            }
            other => match self.finish.back(other.finish().expect("a finishing step")) {
                Some(f) => Step::from_finish(f),
                None => Step::Checksum,
            },
        };
        true
    }

    // ----- taps and keys -----

    /// Handles a tap at time `now_ms` for `network`. The hold on the last
    /// step is handled by the caller.
    pub fn tap(&mut self, id: Id, network: Network) {
        match self.step {
            // A choice is checked, then confirmed (`docs/DESIGN.md`
            // §2.7): the tap that checks a row moves nothing.
            Step::Source => {
                if id == ids::LOAD_SOURCE_TYPE {
                    self.source = Source::Type;
                } else if id == ids::LOAD_SOURCE_SCAN {
                    self.source = Source::SeedCode;
                } else if id == ids::LOAD_SOURCE_BACKUP {
                    self.source = Source::Backup;
                } else if id == ids::LOAD_SOURCE_XOR {
                    self.source = Source::SeedXor;
                } else if id == ids::LOAD_SOURCE_SLIP39 {
                    self.source = Source::Slip39;
                } else if id == ids::LOAD_SOURCE_CODEX32 {
                    self.source = Source::Codex32;
                } else if id == ids::LOAD_SOURCE_NUMBERS {
                    self.source = Source::Numbers;
                } else if id == ids::LOAD_SOURCE_HEX {
                    self.source = Source::Hex;
                } else if id == ids::LOAD_SOURCE_CONTINUE && self.source == Source::Codex32 {
                    // A codex32 string states its own length, so there
                    // is no count to ask for and no wordlist to choose.
                    self.step = Step::Words;
                } else if id == ids::LOAD_SOURCE_CONTINUE
                    && matches!(
                        self.source,
                        Source::Type | Source::Slip39 | Source::Numbers | Source::Hex
                    )
                {
                    // A share's words are 20 or 33; a key's are 12 to 24.
                    // Numbers are words, and hex entropy is the same
                    // length in digits, so both ask the count and the
                    // language first (§16.125 rules 2 and 3).
                    self.count = self.counts()[0];
                    self.step = Step::Count;
                }
            }
            Step::Count => {
                let counts = self.counts();
                if let Some(i) = ids::index_in(id, ids::LOAD_COUNT_BASE, counts.len()) {
                    let count = counts[i];
                    self.set_count(count);
                } else if id == ids::LOAD_COUNT_CONTINUE {
                    // A SLIP-39 share is English on the Latin keyboard,
                    // so there is no language to choose.
                    self.step = if self.is_slip39() {
                        Step::Words
                    } else {
                        Step::Language
                    };
                }
            }
            Step::Language => {
                if let Some(i) = ids::index_in(id, ids::LOAD_LANG_BASE, Language::ALL.len()) {
                    self.lang = Language::ALL[i];
                } else if id == ids::LOAD_LANG_CONTINUE {
                    if self.scanned {
                        self.check();
                    } else {
                        self.step = Step::Words;
                    }
                }
            }
            // §4.3: "Tap accepts." A strip of several cells reports the
            // cell it was tapped on; the lone candidate is one chip and
            // reports its id, and the tap that takes it is the same tap
            // any other candidate costs.
            Step::Words => {
                if id == ids::LOAD_CANDIDATES {
                    self.commit_candidate(0);
                }
            }
            // The codex32 result: Continue asks the next string, or
            // goes on to the key once the secret is in hand.
            Step::Checksum if self.is_codex32() => {
                if id == ids::LOAD_CONTINUE && self.codex.refusal().is_none() {
                    if self.codex.enough() {
                        self.build_codex32(network);
                    } else {
                        self.codex.clear_string();
                        self.step = Step::Words;
                    }
                } else if id == ids::LOAD_START_OVER {
                    self.codex.clear_string();
                    self.step = Step::Words;
                }
            }
            Step::Checksum if self.is_slip39() => {
                if id == ids::LOAD_CONTINUE && self.share_error.is_none() {
                    if self.shares_enough() {
                        self.step = Step::PassphraseOffer;
                    } else {
                        self.clear_words();
                        self.step = Step::Words;
                    }
                } else if id == ids::LOAD_START_OVER {
                    self.share_error = None;
                    self.clear_words();
                    self.step = Step::Words;
                }
            }
            Step::Checksum => {
                if id == ids::LOAD_LANG_OTHER && self.scanned {
                    // The language changes the key, so it stays one tap
                    // from the result the scan landed on.
                    self.step = Step::Language;
                } else if id == ids::LOAD_CONTINUE && self.checksum_ok {
                    self.step = Step::PassphraseOffer;
                } else if id == ids::LOAD_START_OVER {
                    self.start_over();
                } else if let Some(p) =
                    ids::index_in(id, ids::LOAD_FIX_BASE, usize::from(self.count))
                {
                    self.fix_word(p);
                }
            }
            other => {
                let step = other.finish().expect("a finishing step");
                let finish = &mut self.finish;
                if let Some(next) = with_material(
                    self.source,
                    self.lang,
                    &self.words,
                    self.count,
                    &self.shares,
                    &self.codex,
                    |m| finish.tap(step, id, m, network),
                )
                .flatten()
                {
                    self.step = Step::from_finish(next);
                }
            }
        }
    }

    /// Handles keyboard input at time `now_ms` for `network`: the BIP-39
    /// keyboard on the words step, the passphrase keyboard on its step.
    pub fn key(&mut self, id: Id, input: KeyInput, network: Network, now_ms: u64) {
        if id == ids::LOAD_KEYBOARD && self.step == Step::Words && self.is_codex32() {
            match input {
                KeyInput::Char(c) => {
                    self.codex.type_char(c);
                }
                KeyInput::Backspace => self.codex.backspace(),
                KeyInput::Done => self.submit_codex32(network),
                KeyInput::Shift | KeyInput::Symbols => {}
            }
        } else if id == ids::LOAD_KEYBOARD && self.step == Step::Words && self.is_hex() {
            match input {
                KeyInput::Char(c) => {
                    if self.hex.push(c) {
                        self.typed_at = now_ms;
                    }
                }
                KeyInput::Backspace => {
                    self.hex.pop();
                    self.typed_at = 0;
                }
                KeyInput::Done => self.submit_hex(),
                KeyInput::Shift | KeyInput::Symbols => {}
            }
        } else if id == ids::LOAD_KEYBOARD && self.step == Step::Words {
            match input {
                KeyInput::Char(c) => {
                    self.type_char(c);
                }
                KeyInput::Backspace => self.backspace(),
                KeyInput::Done => {
                    self.commit_selected();
                }
                KeyInput::Shift | KeyInput::Symbols => {}
            }
        } else if id == ids::LOAD_PASS_KEYBOARD
            && let Some(step) = self.step.finish()
        {
            let finish = &mut self.finish;
            if let Some(next) = with_material(
                self.source,
                self.lang,
                &self.words,
                self.count,
                &self.shares,
                &self.codex,
                |m| finish.key(step, input, m, network, now_ms),
            )
            .flatten()
            {
                self.step = Step::from_finish(next);
            }
        } else if id == ids::LOAD_PIN_KEYBOARD
            && let Some(step) = self.step.finish()
            && let Some(next) = self.finish.pin_key(step, input)
        {
            self.step = Step::from_finish(next);
        }
    }

    /// The session PIN, once typed twice the same on the PIN steps.
    pub fn take_pin(&mut self) -> Option<PinEntry> {
        self.finish.take_pin()
    }

    /// When the last typed passphrase character, or the last hex digit
    /// of the entropy, masks, if one is showing.
    pub fn mask_deadline(&self) -> Option<u64> {
        if self.step == Step::Words && self.is_hex() {
            return (!self.hex.is_empty()).then_some(self.typed_at + MASK_MS);
        }
        (self.step == Step::Passphrase)
            .then(|| self.finish.mask_deadline())
            .flatten()
    }

    // ----- words -----

    /// 0-based position of the word being typed.
    pub fn position(&self) -> usize {
        usize::from(self.committed)
    }

    /// Number of committed words.
    pub fn committed(&self) -> usize {
        usize::from(self.committed)
    }

    /// The committed words' 1-based wordlist numbers, for the chip row.
    /// Numbers are what `docs/PLANNING.md` §4.6 allows on screen.
    pub fn committed_numbers(&self) -> impl Iterator<Item = u16> + '_ {
        self.words[..self.committed()].iter().map(|&w| w + 1)
    }

    /// The committed words' wordlist indices, for the masked panel of
    /// the words accepted so far (`docs/DESIGN.md` §4.3).
    pub fn committed_indices(&self) -> impl Iterator<Item = u16> + '_ {
        self.words[..self.committed()].iter().copied()
    }

    /// The keys typed so far for the current word.
    pub fn prefix(&self) -> &[char] {
        self.prefix.as_chars()
    }

    /// The keyboard this wizard's wordlist is typed on: the digit pad
    /// where the words are typed as numbers, the hex keyboard where the
    /// entropy is (§16.125).
    pub fn keyboard(&self) -> KeyboardKind {
        match self.source {
            Source::Numbers => KeyboardKind::Pin,
            Source::Hex => KeyboardKind::Hex,
            _ => self.list().keyboard(),
        }
    }

    /// Whether the current word is the last one.
    pub fn typing_last(&self) -> bool {
        self.committed + 1 == self.count
    }

    /// Whether `i` is a valid final word for the committed words.
    fn last_allowed(&self, i: u16) -> bool {
        match self.allowed_last {
            Some(mask) if self.typing_last() => mask[usize::from(i >> 6)] >> (i & 63) & 1 == 1,
            _ => true,
        }
    }

    /// Runs `f` over the words that match what has been typed, in list
    /// order, stopping when it returns `false`. One place decides which
    /// prefix search a wordlist uses: the key sequence for the eight
    /// spelled lists, the syllable and its tone for the two read ones.
    fn for_each_candidate(&self, mut f: impl FnMut(u16) -> bool) {
        let kind = self.keyboard();
        // A number names one word of the list, and that word is the one
        // candidate the strip carries (§16.125 rule 2).
        if self.is_numbers() {
            if let Some(i) = self.typed_number() {
                f(i - 1);
            }
            return;
        }
        if reads(kind) {
            let r = Reading::of(kind, self.prefix.as_chars());
            let tone = r.tone;
            if kind == KeyboardKind::Pinyin {
                for i in self.lang.candidates_pinyin(r.spelling(), tone) {
                    if !f(i) {
                        return;
                    }
                }
            } else {
                for i in self.lang.candidates_zhuyin(r.spelling(), tone) {
                    if !f(i) {
                        return;
                    }
                }
            }
        } else {
            self.list().each_typed(self.prefix.as_chars(), f);
        }
    }

    /// Candidate indices from `skip`, in list order, and how many were
    /// found: one more than the strip holds means another page follows.
    /// Empty until a key is pressed, and on the two read lists until the
    /// tone completes the reading ([`awaiting_tone`](Self::awaiting_tone)).
    /// For the final word, only the words
    /// whose checksum bits fit are offered while any match the prefix;
    /// when none do, every matching word is offered so that a wrong
    /// earlier word still leads to the checksum screen and its diagnosis
    /// rather than to a dead end.
    fn candidate_page(&self, skip: usize) -> ([u16; MAX_CANDIDATES_HANZI + 1], usize) {
        let mut out = [0u16; MAX_CANDIDATES_HANZI + 1];
        // One more than the strip holds is what says another page
        // follows, so the search stops there.
        let cap = (self.capacity() + 1).min(out.len());
        let mut n = 0;
        if self.prefix.is_empty() || self.awaiting_tone() {
            return (out, 0);
        }
        let mut filtered = false;
        self.for_each_candidate(|i| {
            filtered = self.last_allowed(i);
            !filtered
        });
        let mut skip = skip;
        self.for_each_candidate(|i| {
            if filtered && !self.last_allowed(i) {
                return true;
            }
            if skip > 0 {
                skip -= 1;
                return true;
            }
            out[n] = i;
            n += 1;
            n < cap
        });
        (out, n)
    }

    /// Candidates the strip holds on this wizard's list
    /// ([`max_candidates`]).
    pub fn capacity(&self) -> usize {
        if self.list().max_display_chars() == 1 {
            MAX_CANDIDATES_HANZI
        } else {
            MAX_CANDIDATES
        }
    }

    /// Candidate indices the strip offers now: the current page.
    pub fn candidates(&self) -> impl Iterator<Item = u16> {
        let (buf, n) = self.candidate_page(usize::from(self.page));
        buf.into_iter().take(n.min(self.capacity()))
    }

    /// The one word left, which is the one a tap takes without choosing
    /// (`docs/DESIGN.md` §4.3). Counted over every page, so a strip
    /// turned to its last page does not read as one word left.
    fn only_candidate(&self) -> Option<u16> {
        let (buf, n) = self.candidate_page(0);
        (n == 1).then_some(buf[0])
    }

    /// Whether one candidate is left, which the strip outlines.
    pub fn accepting(&self) -> bool {
        self.only_candidate().is_some()
    }

    /// Whether more candidates remain than a strip of `shown` cells
    /// holds, so its last cell turns the page. Only the two read lists
    /// page: a syllable and its tone can still leave dozens of
    /// characters, while a spelled word is narrowed by another key.
    ///
    /// Since §16.45 the Chinese strip holds twenty and the largest tone
    /// group of either list is twenty, so this is false throughout the
    /// readings as they stand. It is the fallback for a group of
    /// twenty-one or more — another reading, another list — and the
    /// wizard keeps the offset and the chevron for it.
    pub fn more_candidates(&self, shown: usize) -> bool {
        self.list().is_hanzi() && self.candidate_page(usize::from(self.page)).1 > shown
    }

    /// Turns the candidate strip on by one page of `step` candidates.
    pub fn page_forward(&mut self, step: usize) {
        self.selected = None;
        self.page = self
            .page
            .saturating_add(u16::try_from(step).unwrap_or(u16::MAX));
    }

    /// Whether a reading is being typed and its tone has not been
    /// pressed yet. A syllable without a tone leaves far too many
    /// characters to choose between, so the strip stays empty until the
    /// reading is complete — and an incomplete reading is not an error,
    /// so the screen says nothing either (`docs/PLANNING.md` §16.43).
    pub fn awaiting_tone(&self) -> bool {
        let kind = self.keyboard();
        reads(kind) && Reading::of(kind, self.prefix.as_chars()).tone.is_none()
    }

    /// The keys some reading still allows: the character keys that
    /// extend the spelling, the tone keys that end it, and whether what
    /// has been typed is already a whole syllable of some word.
    fn reading_keys(&self, kind: KeyboardKind, spelling: &str) -> (KeyMask, KeyMask, bool) {
        let mut chars = 0;
        let mut tones = 0;
        let mut whole = false;
        for i in 0..2048u16 {
            for (reading, tone) in self.lang.word_readings(i) {
                let Some(rest) = reading.strip_prefix(spelling) else {
                    continue;
                };
                match rest.chars().next() {
                    Some(c) => chars |= keyboard::key_bit(kind, c),
                    None => {
                        whole = true;
                        tones |= tone_bit(kind, tone);
                    }
                }
            }
        }
        (chars, tones, whole)
    }

    /// The keys that would type the first reading of the word at `idx`,
    /// for the field a backspace re-opens.
    fn reading_typed(&self, idx: u16) -> Option<Typed> {
        let kind = self.keyboard();
        let (spelling, tone) = self.lang.word_readings(idx).next()?;
        let mut out = Typed::new();
        for c in spelling.chars() {
            out.push(c);
        }
        match kind {
            KeyboardKind::Pinyin => {
                out.push(char::from(b'0' + tone.clamp(1, 5)));
            }
            _ => {
                out.push(keyboard::ZHUYIN_MARKS[usize::from(tone.clamp(1, 5)) - 1]);
            }
        }
        Some(out)
    }

    /// Whether some word of the list continues the prefix with `c`.
    fn continues_with(&self, c: char) -> bool {
        let n = self.prefix.len();
        let list = self.list();
        let mut found = false;
        list.each_typed(self.prefix.as_chars(), |i| {
            found = list
                .typed(i)
                .is_some_and(|t| t.as_chars().get(n) == Some(&c));
            !found
        });
        found
    }

    /// The prefix with the last kana swapped between its full-size and
    /// its small form, which is what the 小 key does.
    fn small_toggled(&self) -> Option<Typed> {
        let last = self.prefix.last()?;
        let other = kana::small(last).or_else(|| kana::large(last))?;
        let mut swapped = self.prefix;
        swapped.set_last(other);
        Some(swapped)
    }

    /// The 1-based word number the digits in the field spell, while they
    /// spell one the list has: 1 to 2048 (§16.125 rule 2).
    fn typed_number(&self) -> Option<u16> {
        let chars = self.prefix.as_chars();
        if chars.is_empty() {
            return None;
        }
        let mut n: u32 = 0;
        for c in chars {
            n = n * 10 + c.to_digit(10)?;
            if n > 2048 {
                return None;
            }
        }
        (n >= 1).then_some(n as u16)
    }

    /// The digits that can still lead to a number on the list: `0` is
    /// dead on an empty field, and every digit is dead once what is
    /// typed cannot grow into 1 to 2048 (§16.125 rule 2). The pad has
    /// no ✓, because a word is taken by a tap on its candidate
    /// (§16.118, §16.132 rule 1).
    fn number_keys(&self) -> KeyMask {
        let mut mask = keyboard::DONE_DISABLED | keyboard::DONE_ABSENT;
        let typed: u32 = match self.typed_number() {
            Some(n) => u32::from(n),
            None if self.prefix.is_empty() => 0,
            // A field that already holds 2048 takes no further digit.
            None => return mask,
        };
        for d in 0..10u32 {
            let n = typed * 10 + d;
            if (1..=2048).contains(&n) {
                mask |= keyboard::key_bit(KeyboardKind::Pin, char::from(b'0' + d as u8));
            }
        }
        mask
    }

    /// Keys that extend the prefix towards some word of the list. The
    /// final word is not restricted here, for the reason given on
    /// [`candidates`](Self::candidates).
    ///
    /// A key is live when a candidate's next character is typed by it: on
    /// the kana keyboard a small kana lights its full-size key, since 小
    /// is what makes it small, and the two voicing marks light ゛ and ゜.
    /// 小 itself is live when swapping the last kana for its other form
    /// leads somewhere.
    pub fn enabled_keys(&self) -> KeyMask {
        let kind = self.keyboard();
        if self.is_numbers() {
            return self.number_keys();
        }
        if reads(kind) {
            // A tone ends the syllable: nothing follows it but a
            // backspace, or a tap on one of the characters it left.
            let r = Reading::of(kind, self.prefix.as_chars());
            if r.tone.is_some() {
                return 0;
            }
            let (chars, tones, _) = self.reading_keys(kind, r.spelling());
            return chars | tones;
        }
        let n = self.prefix.len();
        let list = self.list();
        let mut mask = 0;
        list.each_typed(self.prefix.as_chars(), |i| {
            if let Some(c) = list.typed(i).and_then(|t| t.as_chars().get(n).copied()) {
                let key = if kind == KeyboardKind::Kana {
                    kana::key_for(c)
                } else {
                    c
                };
                mask |= keyboard::key_bit(kind, key);
            }
            true
        });
        if kind == KeyboardKind::Kana
            && let Some(swapped) = self.small_toggled()
            && list.any_typed(swapped.as_chars())
        {
            mask |= keyboard::key_bit(kind, kana::SMALL_KEY);
        }
        mask
    }

    /// Swaps the last kana typed for its small form, or back. False when
    /// there is nothing to swap or nothing follows from it.
    fn toggle_small(&mut self) -> bool {
        let Some(swapped) = self.small_toggled() else {
            return false;
        };
        if !self.list().any_typed(swapped.as_chars()) {
            return false;
        }
        self.prefix = swapped;
        self.selected = None;
        true
    }

    /// Types one key. Ignored when no candidate word continues with it.
    /// Nothing a key does commits a word: a word is taken by a tap on
    /// its candidate (`docs/PLANNING.md` §16.118).
    pub fn type_char(&mut self, c: char) -> bool {
        let kind = self.keyboard();
        if self.is_numbers() {
            let bit = keyboard::key_bit(KeyboardKind::Pin, c);
            if bit == 0 || self.number_keys() & bit == 0 || !self.prefix.push(c) {
                return false;
            }
            self.page = 0;
            self.selected = None;
            return true;
        }
        let c = if matches!(kind, KeyboardKind::Bip39 | KeyboardKind::Pinyin) {
            c.to_ascii_lowercase()
        } else {
            c
        };
        if reads(kind) {
            let bit = keyboard::key_bit(kind, c);
            if bit == 0 || self.enabled_keys() & bit == 0 || !self.prefix.push(c) {
                return false;
            }
            self.page = 0;
            self.selected = None;
            return true;
        }
        if kind == KeyboardKind::Kana && c == kana::SMALL_KEY {
            return self.toggle_small();
        }
        // The two mark keys carry the standalone marks; a word carries
        // the combining ones.
        let typed = match (kind, c) {
            (KeyboardKind::Kana, kana::DAKUTEN_KEY) => kana::DAKUTEN,
            (KeyboardKind::Kana, kana::HANDAKUTEN_KEY) => kana::HANDAKUTEN,
            _ => c,
        };
        let bit = keyboard::key_bit(kind, c);
        let allowed = if bit == 0 {
            // A character with no key of its own: a small kana from a
            // physical keyboard. It stands if a candidate wants it.
            self.continues_with(typed)
        } else {
            self.enabled_keys() & bit != 0
        };
        if !allowed || !self.prefix.push(typed) {
            return false;
        }
        self.page = 0;
        self.selected = None;
        true
    }

    /// Deletes the last key typed. On an empty field it steps back to
    /// the previous word, which returns to the field for editing
    /// (`docs/PLANNING.md` §4.6): the screen shows one word and nothing
    /// else, so the way back to the one before it is the backspace.
    pub fn backspace(&mut self) {
        self.page = 0;
        self.selected = None;
        if self.prefix.pop().is_some() {
            return;
        }
        if self.committed == 0 {
            return;
        }
        let previous = self.words[self.committed() - 1];
        self.uncommit();
        if self.is_numbers() {
            self.prefix = number_typed(previous + 1);
            return;
        }
        if let Some(typed) = self
            .list()
            .typed(previous)
            .or_else(|| self.reading_typed(previous))
        {
            self.prefix = typed;
        }
    }

    /// Takes the `n`-th candidate: on the small panel the first tap
    /// selects it and the next accepts it, elsewhere one tap accepts
    /// (`docs/PLANNING.md` §16.50). A tap on a cell that is not the
    /// selected one moves the selection and accepts nothing.
    pub fn commit_candidate(&mut self, n: usize) -> bool {
        let Some(picked) = self.candidates().nth(n) else {
            return false;
        };
        if self.two_tap && self.selected() != Some(picked) {
            self.selected = Some(picked);
            return true;
        }
        self.commit(picked);
        true
    }

    /// Whether a candidate is selected and waiting for the tap that
    /// accepts it. A lone candidate is selected as soon as it is the
    /// only one left, so it costs one tap like every other.
    pub fn selected(&self) -> Option<u16> {
        if !self.two_tap {
            return None;
        }
        self.selected.or_else(|| self.only_candidate())
    }

    /// Which cell of the strip carries the selected candidate, counting
    /// across both rows, when it is on the page the strip shows.
    pub fn selected_cell(&self) -> Option<usize> {
        let selected = self.selected()?;
        self.candidates().position(|i| i == selected)
    }

    /// Whether a candidate is selected by one tap and accepted by the
    /// next. The size class decides it, so the shell sets it before the
    /// wizard acts on anything.
    pub fn set_two_tap(&mut self, two_tap: bool) {
        if !two_tap {
            self.selected = None;
        }
        self.two_tap = two_tap;
    }

    /// §16.118 rule 3: a physical keyboard's Enter takes the selected
    /// candidate, or the one candidate left, and does nothing with
    /// several. There is no key on the screen that does this: a word is
    /// taken by a tap on its candidate.
    pub fn commit_selected(&mut self) -> bool {
        if self.prefix.is_empty() {
            return false;
        }
        let Some(picked) = self.selected().or_else(|| self.only_candidate()) else {
            return false;
        };
        self.commit(picked);
        true
    }

    fn commit(&mut self, index: u16) {
        self.words[self.committed()] = index;
        self.committed += 1;
        self.prefix.zeroize();
        self.page = 0;
        self.selected = None;
        if let Some(n) = self.resume_at.take() {
            self.committed = n;
        }
        // The final-word filter is BIP-39's checksum; a share is checked
        // whole after its last word (§16.107 rule 7).
        if self.committed + 1 == self.count && !self.is_slip39() && !self.free_last {
            self.allowed_last = Some(last_word_mask(&self.words[..self.committed()]));
        }
        if self.committed == self.count {
            if self.is_slip39() {
                self.check_share();
            } else {
                self.check();
            }
        }
    }

    fn uncommit(&mut self) {
        self.resume_at = None;
        if self.committed > 0 {
            self.committed -= 1;
            self.words[self.committed()] = 0;
        }
        if self.committed + 1 < self.count {
            self.allowed_last = None;
        }
    }

    fn clear_words(&mut self) {
        self.words.zeroize();
        self.committed = 0;
        self.resume_at = None;
        self.prefix.zeroize();
        self.page = 0;
        self.selected = None;
        self.allowed_last = None;
        self.checksum_ok = false;
        self.suspects = 0;
        self.share_error = None;
    }

    /// Discards every word and returns to the first one, typed — or,
    /// where the words came from entropy, to the empty digits.
    pub fn start_over(&mut self) {
        self.clear_words();
        self.hex.zeroize();
        self.typed_at = 0;
        self.scanned = false;
        self.step = Step::Words;
    }

    /// Returns to the Words step to re-type the word at 0-based `position`;
    /// the words after it are kept and restored once it is committed.
    pub fn fix_word(&mut self, position: usize) {
        if position >= usize::from(self.count) {
            return;
        }
        self.resume_at = Some(self.count);
        self.committed = position as u8;
        self.words[position] = 0;
        self.prefix.zeroize();
        self.page = 0;
        self.allowed_last = (position + 1 == usize::from(self.count))
            .then(|| last_word_mask(&self.words[..position]))
            .filter(|_| !self.free_last);
        self.checksum_ok = false;
        self.suspects = 0;
        self.scanned = false;
        self.step = Step::Words;
    }

    // ----- checksum -----

    fn check(&mut self) {
        let indices = &self.words[..usize::from(self.count)];
        match Mnemonic::from_indices(self.lang, indices) {
            Ok(_) => {
                self.checksum_ok = true;
                self.suspects = 0;
            }
            Err(_) => {
                self.checksum_ok = false;
                self.suspects = suspects(self.lang, indices);
            }
        }
        self.step = Step::Checksum;
    }

    // ----- codex32 -----

    /// A wizard holding the codex32 string `text`, as a scan, a paste
    /// or the scanner's Type hands one over (`docs/PLANNING.md` §16.109
    /// rule 3): the string goes through the same checks a typed one
    /// does, so a secret lands on the confirmation and a share starts
    /// the gathering. `None` where the field cannot hold the text.
    pub fn from_codex32(text: &str, network: Network) -> Option<Self> {
        let mut w = Self::new();
        w.source = Source::Codex32;
        w.step = Step::Words;
        if !w.codex.fill(text) {
            return None;
        }
        w.submit_codex32(network);
        Some(w)
    }

    /// A wizard holding the key a master seed is, which is what an
    /// encrypted backup of a key with no words carries
    /// (`docs/PLANNING.md` §16.112 rule 1). The key is built at once
    /// and the wizard opens on its confirmation, the way a codex32
    /// string that turns out to be a secret does. The seed alone does
    /// not say whether the key was read from SLIP-39 shares or from a
    /// codex32 string, and it does not have to: either way the key is
    /// its master secret and carries no words.
    pub fn from_seed(seed: &[u8], network: Network) -> Option<Self> {
        let mut w = Self::new();
        w.source = Source::Slip39;
        w.scanned = true;
        let finish = &mut w.finish;
        if !finish.build(&Material::Seed(seed), network) {
            return None;
        }
        w.step = Step::Confirm;
        Some(w)
    }

    /// ✓ on the codex32 entry: the secret goes on to the key, a share
    /// to the result that says what the set now has.
    fn submit_codex32(&mut self, network: Network) {
        match self.codex.submit() {
            None => {}
            Some(Submitted::Secret) => self.build_codex32(network),
            Some(Submitted::Accepted | Submitted::Refused(_)) => self.step = Step::Checksum,
        }
    }

    /// Builds the key from the secret in hand and goes to the
    /// confirmation. BIP 93 has no passphrase, so the passphrase steps
    /// are not asked (§16.109 rule 2).
    fn build_codex32(&mut self, network: Network) {
        let finish = &mut self.finish;
        let built = with_material(
            Source::Codex32,
            self.lang,
            &self.words,
            self.count,
            &self.shares,
            &self.codex,
            |m| finish.build(m, network),
        );
        if built == Some(true) {
            self.step = Step::Confirm;
        }
    }

    // ----- hex entropy -----

    /// ✓ on the hex entry: the digits are the key's entropy, so the
    /// words follow from them and the language, and the checksum result
    /// reports them (§16.125 rule 3).
    fn submit_hex(&mut self) {
        if !self.hex_ready() {
            return;
        }
        let Ok(entropy) = self.hex.entropy() else {
            return;
        };
        let Ok(m) = Mnemonic::from_entropy(self.lang, entropy.as_bytes()) else {
            return;
        };
        self.words[..m.word_count()].copy_from_slice(m.indices());
        self.committed = m.word_count() as u8;
        self.check();
    }

    /// The share the words state, checked whole and against the shares
    /// already in (§16.107 rule 2). It is kept when it passes; the
    /// reason stands on the result when it does not.
    fn check_share(&mut self) {
        let indices = &self.words[..usize::from(self.count)];
        self.share_error = match Share::from_indices(indices) {
            Err(e) => Some(ShareRefusal::Codec(e)),
            Ok(share) => match self.shares.admits(&share) {
                Err(refusal) => Some(refusal),
                Ok(()) => {
                    self.shares.push(share);
                    None
                }
            },
        };
        self.checksum_ok = self.share_error.is_none();
        self.step = Step::Checksum;
    }

    /// Whether the typed words form a valid mnemonic.
    pub fn checksum_ok(&self) -> bool {
        self.checksum_ok
    }

    /// 0-based positions where replacing the word with a similar one makes
    /// the checksum pass, after a failure.
    pub fn suspects(&self) -> impl Iterator<Item = usize> + '_ {
        (0..usize::from(self.count)).filter(move |&p| self.suspects >> p & 1 == 1)
    }

    // ----- key -----

    /// The typed words as a mnemonic, once they form a valid one.
    pub fn mnemonic(&self) -> Option<Mnemonic> {
        Mnemonic::from_indices(self.lang, &self.words[..usize::from(self.count)]).ok()
    }

    /// Takes the built key out; the rest of the wizard state is zeroized
    /// when `self` drops. A loaded key's backup is unverified until the
    /// quiz is run from Key detail.
    pub fn into_key(mut self) -> Option<LoadedKey> {
        if self.is_codex32() {
            return self.finish.take_key(None, false).map(LoadedKey::codex32);
        }
        if self.is_slip39() {
            return self.finish.take_key(None, false);
        }
        let m = self.mnemonic()?;
        self.finish.take_key(Some(m), false)
    }
}

/// Runs `f` on what this wizard builds its key from: the typed words, or
/// exactly the shares [`slip39::recover`] takes.
fn with_material<R>(
    source: Source,
    lang: Language,
    words: &[u16; ENTRY_WORDS],
    count: u8,
    shares: &ShareSet,
    codex: &Codex32Entry,
    f: impl FnOnce(&Material<'_>) -> R,
) -> Option<R> {
    if source == Source::Slip39 {
        return shares.with_recovery_set(|set| f(&Material::Shares(set)));
    }
    if source == Source::Codex32 {
        return codex.with_seed(|seed| f(&Material::Seed(seed)));
    }
    let m = Mnemonic::from_indices(lang, &words[..usize::from(count)]).ok()?;
    Some(f(&Material::Words(&m)))
}

/// The digits of `number`, for the field a backspace re-opens on the
/// word-numbers entry (§16.125 rule 2).
fn number_typed(number: u16) -> Typed {
    let mut out = Typed::new();
    let mut seen = false;
    for place in [1000u16, 100, 10, 1] {
        let d = number / place % 10;
        if d > 0 || seen || place == 1 {
            out.push(char::from(b'0' + d as u8));
            seen = true;
        }
    }
    out
}

/// Bitmask of the valid final words after `first_words`.
fn last_word_mask(first_words: &[u16]) -> [u64; 32] {
    let mut mask = [0u64; 32];
    if let Ok(candidates) = bip39::last_word_candidates(first_words) {
        for i in candidates {
            mask[usize::from(i >> 6)] |= 1 << (i & 63);
        }
    }
    mask
}

/// Positions (as a bitmask) at which some *similar* word makes the
/// checksum pass. Trying every word would flag every position, since a
/// random substitution passes a 4-bit checksum one time in sixteen;
/// restricting substitutes to words that share the first three letters or
/// are one edit away targets the two real error modes, picking the wrong
/// candidate on the keyboard and misreading a handwritten backup.
fn suspects(lang: Language, indices: &[u16]) -> u32 {
    let mut mask = 0u32;
    let mut trial = [0u16; MAX_WORDS];
    trial[..indices.len()].copy_from_slice(indices);
    for (p, &current) in indices.iter().enumerate() {
        let Some(cur) = lang.typed(current) else {
            continue;
        };
        for alt in 0..2048u16 {
            if alt == current {
                continue;
            }
            let Some(a) = lang.typed(alt) else {
                continue;
            };
            if !similar(cur.as_chars(), a.as_chars()) {
                continue;
            }
            trial[p] = alt;
            if Mnemonic::from_indices(lang, &trial[..indices.len()]).is_ok() {
                mask |= 1 << p;
                break;
            }
        }
        trial[p] = current;
    }
    trial.zeroize();
    mask
}

/// Same first three keys, or Levenshtein distance at most one.
fn similar(a: &[char], b: &[char]) -> bool {
    if a.len() >= 3 && b.len() >= 3 && a[..3] == b[..3] {
        return true;
    }
    edit_distance_at_most_one(a, b)
}

fn edit_distance_at_most_one(a: &[char], b: &[char]) -> bool {
    let (short, long) = if a.len() <= b.len() { (a, b) } else { (b, a) };
    match long.len() - short.len() {
        0 => short.iter().zip(long).filter(|(x, y)| x != y).count() <= 1,
        1 => {
            // One insertion: skip the first mismatch in the longer word.
            let mut i = 0;
            let mut j = 0;
            let mut skipped = false;
            while i < short.len() && j < long.len() {
                if short[i] == long[j] {
                    i += 1;
                    j += 1;
                } else if skipped {
                    return false;
                } else {
                    skipped = true;
                    j += 1;
                }
            }
            true
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn type_word(w: &mut LoadWizard, word: &str) {
        for c in word.chars() {
            assert!(w.type_char(c), "{word}: {c}");
        }
        // §16.118: nothing the letters do takes the word. A word typed
        // out in full is the first candidate, and a tap on it accepts —
        // two taps where a cell is selected before it is taken.
        let before = w.committed();
        assert!(w.commit_candidate(0), "{word}");
        if w.committed() == before {
            assert!(w.commit_candidate(0), "{word}");
        }
        assert!(w.prefix().is_empty(), "{word} was not taken");
    }

    fn keys(word: &str) -> alloc::vec::Vec<char> {
        word.chars().collect()
    }

    /// A wizard at the word step of `lang`.
    fn typing(lang: Language) -> LoadWizard {
        let mut w = LoadWizard::new();
        w.go(Step::Language);
        let i = Language::ALL
            .iter()
            .position(|l| *l == lang)
            .expect("a wordlist");
        w.tap(ids::at(ids::LOAD_LANG_BASE, i), Network::Mainnet);
        w.tap(ids::LOAD_LANG_CONTINUE, Network::Mainnet);
        assert_eq!(w.step(), Step::Words);
        w
    }

    /// All ten wordlists ship, and each has a keyboard that types it,
    /// so every row on the language step takes.
    #[test]
    fn every_wordlist_can_be_chosen() {
        for lang in Language::ALL {
            let mut w = LoadWizard::new();
            w.go(Step::Language);
            let i = Language::ALL
                .iter()
                .position(|l| *l == lang)
                .expect("a list");
            w.tap(ids::at(ids::LOAD_LANG_BASE, i), Network::Mainnet);
            assert_eq!(w.language(), lang, "{lang:?}");
            w.tap(ids::LOAD_LANG_CONTINUE, Network::Mainnet);
            assert_eq!(w.step(), Step::Words);
        }
    }

    /// The kana keyboard: the voicing key is live after a kana that can
    /// be voiced and dead after one that cannot, and 小 is live where a
    /// small kana follows.
    #[test]
    fn the_kana_keys_follow_what_the_words_allow() {
        let voiced = keyboard::key_bit(KeyboardKind::Kana, kana::DAKUTEN_KEY);
        let small = keyboard::key_bit(KeyboardKind::Kana, kana::SMALL_KEY);

        let mut w = typing(Language::Japanese);
        assert!(w.type_char('\u{304b}'), "か");
        assert!(w.enabled_keys() & voiced != 0, "か can take a voicing mark");

        let mut w = typing(Language::Japanese);
        assert!(w.type_char('\u{3042}'), "あ");
        assert!(w.type_char('\u{3093}'), "ん");
        assert_eq!(w.enabled_keys() & voiced, 0, "ん cannot be voiced");
        assert!(w.type_char('\u{304b}'), "か");
        assert!(w.enabled_keys() & voiced != 0, "あんがい");

        // きゃ: the や key types the full-size kana and 小 makes it small.
        let mut w = typing(Language::Japanese);
        assert!(w.type_char('\u{304d}'), "き");
        assert!(w.type_char('\u{3084}'), "や");
        assert!(w.enabled_keys() & small != 0, "小 makes it きゃ");
        assert!(w.type_char(kana::SMALL_KEY));
        assert!(w.candidates().count() > 0, "きゃ starts words");
        assert_eq!(w.prefix(), ['\u{304d}', '\u{3083}']);
        // And back again.
        assert!(w.type_char(kana::SMALL_KEY));
        assert_eq!(w.prefix(), ['\u{304d}', '\u{3084}']);
    }

    /// The jamo keyboard: a consonant that ends one syllable and a vowel
    /// that starts the next are both live after `ㄱㅏㄱ`, because the
    /// keys say nothing about which syllable they belong to.
    #[test]
    fn the_jamo_keys_leave_both_readings_of_a_final_open() {
        let mut w = typing(Language::Korean);
        for key in ['\u{3131}', '\u{314F}', '\u{3131}'] {
            assert!(w.type_char(key), "ㄱㅏㄱ");
        }
        let enabled = w.enabled_keys();
        // 가격 continues with a vowel, 각자 with a consonant.
        assert!(
            enabled & keyboard::key_bit(KeyboardKind::Jamo, '\u{3155}') != 0,
            "ㅕ, which makes 가격"
        );
        assert!(
            enabled & keyboard::key_bit(KeyboardKind::Jamo, '\u{3148}') != 0,
            "ㅈ, which makes 각자"
        );
        assert_eq!(
            enabled & keyboard::key_bit(KeyboardKind::Jamo, '\u{3142}'),
            0,
            "ㅂ starts no word from here"
        );
    }

    #[test]
    fn edit_distance_rules() {
        let d = |a: &str, b: &str| edit_distance_at_most_one(&keys(a), &keys(b));
        let s = |a: &str, b: &str| similar(&keys(a), &keys(b));
        assert!(d("sand", "send"));
        assert!(d("sand", "and"));
        assert!(d("sand", "sands"));
        assert!(!d("sand", "bend"));
        assert!(!d("sand", "sandals"));
        assert!(s("legal", "legend"), "same first three letters");
        assert!(!s("abandon", "ability"));
        assert!(!s("zoo", "abandon"));
    }

    #[test]
    fn typing_narrows_and_a_tap_takes_the_candidate() {
        let mut w = LoadWizard::new();
        w.go(Step::Words);
        assert_eq!(w.candidates().count(), 0, "nothing until a letter");
        assert!(w.type_char('a'));
        assert!(w.candidates().count() > 0);
        assert!(!w.type_char('q'), "no word starts with aq");
        assert!(w.type_char('c'));
        assert!(w.type_char('t'));
        assert_eq!(w.prefix(), keys("act"));
        // "act", "action", "actor", "actress" all start with it, so
        // Enter takes nothing and the strip is how one of them is taken
        // (§16.118).
        assert!(w.candidates().count() > 1);
        assert!(!w.commit_selected());
        assert!(w.commit_candidate(0));
        assert_eq!(w.committed(), 1);
        assert!(w.prefix().is_empty());
        // Backspace on an empty prefix un-commits.
        w.backspace();
        assert_eq!(w.committed(), 0);
        // One candidate left: it waits in the strip, and a tap takes it.
        let mut w = LoadWizard::new();
        w.go(Step::Words);
        for c in "abando".chars() {
            assert!(w.type_char(c));
        }
        assert_eq!(w.candidates().count(), 1);
        assert_eq!(w.committed(), 0);
        assert!(w.commit_candidate(0));
        assert_eq!(w.committed(), 1);
    }

    /// A word typed out in full is the one candidate left and stays in
    /// the strip until it is tapped, on a phone as on the panel
    /// (`docs/PLANNING.md` §16.118).
    #[test]
    fn a_fully_typed_word_is_taken_by_a_tap_and_by_nothing_else() {
        for two_tap in [false, true] {
            let mut w = LoadWizard::new();
            w.go(Step::Words);
            w.set_two_tap(two_tap);
            for c in "abandon".chars() {
                assert!(w.type_char(c));
            }
            assert_eq!(w.candidates().count(), 1);
            assert_eq!(w.committed(), 0, "the last letter took the word");
            assert_eq!(w.prefix(), keys("abandon"));
            // A lone candidate is already selected where the strip is
            // tapped twice, so one tap takes it on either class.
            assert!(w.commit_candidate(0));
            assert_eq!(w.committed(), 1);
            assert!(w.prefix().is_empty());
        }
    }

    /// The checksum leaves 128 of the 2048 words for the last place, so
    /// three letters can leave one word there. It is a candidate like
    /// any other: it waits for its tap.
    #[test]
    fn the_last_word_the_checksum_leaves_waits_for_its_tap() {
        let mut w = LoadWizard::new();
        w.go(Step::Words);
        for _ in 0..11 {
            type_word(&mut w, "abandon");
        }
        assert!(w.typing_last());
        for c in "abo".chars() {
            assert!(w.type_char(c));
        }
        assert_eq!(w.candidates().count(), 1, "one word can end this key");
        assert_eq!(w.committed(), 11);
        assert_eq!(w.step(), Step::Words);
        assert!(w.commit_candidate(0));
        assert_eq!(w.step(), Step::Checksum);
        assert!(w.checksum_ok());
    }

    #[test]
    fn a_typed_word_waits_for_its_tap_and_the_last_word_is_filtered() {
        let mut w = LoadWizard::new();
        w.go(Step::Words);
        for _ in 0..11 {
            type_word(&mut w, "abandon");
        }
        assert_eq!(w.committed(), 11);
        assert!(w.typing_last());
        // Only 128 of 2048 words are valid last words.
        w.type_char('a');
        let with_a = w.candidates().count();
        assert!(with_a > 0 && with_a <= MAX_CANDIDATES);
        assert_eq!(
            w.enabled_keys() & keyboard::letter_bit('b'),
            keyboard::letter_bit('b')
        );
        w.backspace();
        type_word(&mut w, "about");
        assert_eq!(w.step(), Step::Checksum);
        assert!(w.checksum_ok());
        w.tap(ids::LOAD_CONTINUE, Network::Mainnet);
        assert_eq!(w.step(), Step::PassphraseOffer);
        w.tap(ids::LOAD_SKIP, Network::Mainnet);
        assert_eq!(w.step(), Step::PassphraseOffer, "the tap only checks");
        w.tap(ids::LOAD_PASS_CONTINUE, Network::Mainnet);
        assert_eq!(w.step(), Step::Confirm);
        assert_eq!(w.finish().fingerprint().unwrap().to_hex(), *b"73c5da0a");
        assert_eq!(w.finish().fingerprint_with_passphrase(), None);
        let key = w.into_key().unwrap();
        assert!(!key.has_passphrase && !key.backup_verified);
        assert_eq!(key.fingerprint.to_hex(), *b"73c5da0a");
        assert!(key.has_mnemonic() && !key.is_sealed());
        let session = SessionKey::from_entropy(&[1; 32]);
        assert_eq!(
            key.mnemonic(&session, |m| m.indices()[11]),
            Some(3),
            "plaintext words open under any key"
        );
    }

    #[test]
    fn a_wrong_word_is_located_and_fixable() {
        // "legal winner thank year wave sausage worth useful legal winner
        // thank yellow" is valid. Mistype word 9 as a word that shares its
        // first three letters and pick one that breaks the checksum.
        const RIGHT: [&str; 12] = [
            "legal", "winner", "thank", "year", "wave", "sausage", "worth", "useful", "legal",
            "winner", "thank", "yellow",
        ];
        let lang = Language::English;
        let mut indices = [0u16; 12];
        for (i, w) in RIGHT.iter().enumerate() {
            indices[i] = lang.index_of(w).unwrap();
        }
        assert!(Mnemonic::from_indices(lang, &indices).is_ok());
        let wrong = ["legend", "lemon", "length", "leg"]
            .into_iter()
            .find(|w| {
                let mut t = indices;
                t[8] = lang.index_of(w).unwrap();
                Mnemonic::from_indices(lang, &t).is_err()
            })
            .expect("some similar word breaks the checksum");

        let mut w = LoadWizard::new();
        w.go(Step::Words);
        for (i, word) in RIGHT.iter().enumerate() {
            type_word(&mut w, if i == 8 { wrong } else { word });
        }
        assert_eq!(w.step(), Step::Checksum);
        assert!(!w.checksum_ok());
        let suspects: alloc::vec::Vec<usize> = w.suspects().collect();
        assert!(suspects.contains(&8), "{suspects:?}");
        w.fix_word(8);
        assert_eq!(w.step(), Step::Words);
        assert_eq!(w.position(), 8);
        type_word(&mut w, "legal");
        // Words 10–12 were kept, so the check runs again immediately.
        assert_eq!(w.step(), Step::Checksum);
        assert!(w.checksum_ok());
    }

    #[test]
    fn the_final_word_prefers_valid_words_but_never_blocks() {
        let mut w = LoadWizard::new();
        w.go(Step::Words);
        for _ in 0..11 {
            type_word(&mut w, "abandon");
        }
        w.type_char('a');
        // Every candidate shown is a valid final word.
        let mask = last_word_mask(&w.words[..11]);
        assert!(
            w.candidates()
                .all(|i| mask[usize::from(i >> 6)] >> (i & 63) & 1 == 1)
        );
        // Every letter of the list stays typable, so an invalid final word
        // can still be entered and diagnosed.
        w.backspace();
        for c in "abandon".chars() {
            assert!(w.type_char(c) || w.prefix().is_empty(), "{c}");
        }
    }

    #[test]
    fn scanned_words_skip_entry_and_land_on_the_confirm() {
        let lang = Language::English;
        let mut idx = [0u16; 12];
        idx[11] = 3; // "about"
        let mut w = LoadWizard::from_words(&idx, lang);
        // A decoded code has a valid checksum, so neither the entry nor
        // the language step is in the way (UX review 2026-09-07, Q4).
        assert_eq!(w.step(), Step::Checksum);
        assert!(w.scanned());
        assert!(w.checksum_ok());
        assert_eq!(w.committed(), 12);
        // The language row on the result opens the language step, and
        // back closes it onto the result.
        w.tap(ids::LOAD_LANG_OTHER, Network::Mainnet);
        assert_eq!(w.step(), Step::Language);
        assert!(w.back());
        assert_eq!(w.step(), Step::Checksum);
        assert_eq!(w.committed(), 12, "nothing un-committed");
        assert!(!w.back(), "the result is the first step of a scan");
        w.tap(ids::LOAD_CONTINUE, Network::Mainnet);
        w.tap(ids::LOAD_PASS_CONTINUE, Network::Mainnet);
        assert_eq!(w.step(), Step::Confirm);
        assert_eq!(w.finish().fingerprint().unwrap().to_hex(), *b"73c5da0a");
        // A scanned list with a bad checksum can be fixed by typing.
        let bad = [0u16; 12];
        let mut w = LoadWizard::from_words(&bad, lang);
        assert_eq!(w.step(), Step::Checksum);
        assert!(!w.checksum_ok());
        w.fix_word(11);
        assert_eq!(w.step(), Step::Words);
        assert!(!w.scanned());
        type_word(&mut w, "about");
        assert!(w.checksum_ok());
        // Not a valid count: an ordinary wizard.
        let w = LoadWizard::from_words(&[1, 2, 3], lang);
        assert_eq!(w.step(), Step::Source);
        assert!(!w.scanned());
    }

    #[test]
    fn back_zeroizes_what_the_step_owned() {
        let mut w = LoadWizard::new();
        w.go(Step::Words);
        type_word(&mut w, "zoo");
        assert_eq!(w.committed(), 1);
        assert!(w.back());
        assert_eq!(w.step(), Step::Language);
        assert_eq!(w.committed(), 0);
        assert!(w.words.iter().all(|&x| x == 0));
        for _ in 0..11 {
            type_word(&mut w, "abandon");
        }
        type_word(&mut w, "about");
        w.tap(ids::LOAD_CONTINUE, Network::Mainnet);
        w.tap(ids::LOAD_ADD_PASSPHRASE, Network::Mainnet);
        w.tap(ids::LOAD_PASS_CONTINUE, Network::Mainnet);
        assert_eq!(w.step(), Step::Passphrase);
        w.key(
            ids::LOAD_PASS_KEYBOARD,
            KeyInput::Char('x'),
            Network::Mainnet,
            0,
        );
        assert_eq!(w.mask_deadline(), Some(MASK_MS));
        assert!(w.back());
        assert_eq!(w.step(), Step::PassphraseOffer);
        assert_eq!(w.finish().passphrase_len(), 0);
        assert!(w.back());
        assert_eq!(w.step(), Step::Checksum);
        w.go(Step::Source);
        assert!(!w.back(), "back at the first step cancels");
    }
}
