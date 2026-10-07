//! BIP-39: mnemonic sentences, entropy, checksums and seed derivation.
//!
//! ```
//! use osk_bip::bip39::{Language, Mnemonic};
//!
//! let m = Mnemonic::from_entropy(Language::English, &[0u8; 16]).unwrap();
//! assert_eq!(m.word_count(), 12);
//! assert_eq!(Language::English.word(m.indices()[11]), "about");
//! let seed = m.to_seed(b"TREZOR").unwrap();
//! assert_eq!(seed.expose()[0], 0xc5);
//! ```

use core::fmt;

use osk_crypto::{Secret, Zeroize, ZeroizeOnDrop, pbkdf2_hmac_sha512, sha256};

use crate::wordlists;

/// Fewest words in a mnemonic (128-bit entropy).
pub const MIN_WORDS: usize = 12;
/// Most words in a mnemonic (256-bit entropy).
pub const MAX_WORDS: usize = 24;
/// Longest accepted passphrase, in bytes.
pub const MAX_PASSPHRASE_BYTES: usize = 256;
/// PBKDF2 rounds fixed by BIP-39.
pub const PBKDF2_ROUNDS: u32 = 2048;

/// Bytes needed for the longest possible NFKD sentence: 24 words plus 23
/// ASCII-space separators.
const MAX_SENTENCE_BYTES: usize = MAX_WORDS * wordlists::MAX_NFKD_BYTES + (MAX_WORDS - 1);
/// Largest packed bit buffer: 24 words of 11 bits = 264 bits = 33 bytes.
const BITS_BUF: usize = 33;
const SALT_PREFIX: &[u8] = b"mnemonic";

/// Why a mnemonic or passphrase was rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// The word count is not 12, 15, 18, 21 or 24 (or, for final-word
    /// candidates, one fewer).
    InvalidWordCount,
    /// Entropy is not 16, 20, 24, 28 or 32 bytes.
    InvalidEntropyLength,
    /// The word at `position` (0-based) is not in the wordlist, or the index
    /// is outside `0..2048`.
    UnknownWord {
        /// 0-based position of the offending word.
        position: u8,
    },
    /// The checksum bits carried by the last word do not match the entropy.
    BadChecksum,
    /// The passphrase contains a byte outside printable ASCII (0x20..=0x7E).
    PassphraseNotAscii,
    /// The passphrase is longer than [`MAX_PASSPHRASE_BYTES`].
    PassphraseTooLong,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::InvalidWordCount => f.write_str("word count must be 12, 15, 18, 21 or 24"),
            Error::InvalidEntropyLength => {
                f.write_str("entropy must be 16, 20, 24, 28 or 32 bytes")
            }
            Error::UnknownWord { position } => {
                write!(f, "word {} is not in the wordlist", position + 1)
            }
            Error::BadChecksum => f.write_str("checksum does not match"),
            Error::PassphraseNotAscii => f.write_str("passphrase must be printable ASCII"),
            Error::PassphraseTooLong => {
                write!(f, "passphrase exceeds {MAX_PASSPHRASE_BYTES} bytes")
            }
        }
    }
}

impl core::error::Error for Error {}

/// One of the ten official BIP-39 wordlists.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Language {
    /// English.
    English,
    /// Japanese.
    Japanese,
    /// Korean.
    Korean,
    /// Spanish.
    Spanish,
    /// Chinese (Simplified).
    ChineseSimplified,
    /// Chinese (Traditional).
    ChineseTraditional,
    /// French.
    French,
    /// Italian.
    Italian,
    /// Czech.
    Czech,
    /// Portuguese.
    Portuguese,
}

impl Language {
    /// Every supported language, English first.
    pub const ALL: [Language; 10] = [
        Language::English,
        Language::Japanese,
        Language::Korean,
        Language::Spanish,
        Language::ChineseSimplified,
        Language::ChineseTraditional,
        Language::French,
        Language::Italian,
        Language::Czech,
        Language::Portuguese,
    ];

    /// The wordlist's file name in the BIP repository, e.g. `"english"`.
    pub fn name(self) -> &'static str {
        match self {
            Language::English => "english",
            Language::Japanese => "japanese",
            Language::Korean => "korean",
            Language::Spanish => "spanish",
            Language::ChineseSimplified => "chinese_simplified",
            Language::ChineseTraditional => "chinese_traditional",
            Language::French => "french",
            Language::Italian => "italian",
            Language::Czech => "czech",
            Language::Portuguese => "portuguese",
        }
    }

    /// The 2048 words as published, for display and input matching.
    pub fn words(self) -> &'static [&'static str; 2048] {
        match self {
            Language::English => &wordlists::english::WORDS,
            Language::Japanese => &wordlists::japanese::WORDS,
            Language::Korean => &wordlists::korean::WORDS,
            Language::Spanish => &wordlists::spanish::WORDS,
            Language::ChineseSimplified => &wordlists::chinese_simplified::WORDS,
            Language::ChineseTraditional => &wordlists::chinese_traditional::WORDS,
            Language::French => &wordlists::french::WORDS,
            Language::Italian => &wordlists::italian::WORDS,
            Language::Czech => &wordlists::czech::WORDS,
            Language::Portuguese => &wordlists::portuguese::WORDS,
        }
    }

    /// The 2048 words in NFKD form, for seed derivation.
    pub fn words_nfkd(self) -> &'static [&'static str; 2048] {
        match self {
            Language::English => wordlists::english::NFKD,
            Language::Japanese => wordlists::japanese::NFKD,
            Language::Korean => wordlists::korean::NFKD,
            Language::Spanish => wordlists::spanish::NFKD,
            Language::ChineseSimplified => wordlists::chinese_simplified::NFKD,
            Language::ChineseTraditional => wordlists::chinese_traditional::NFKD,
            Language::French => wordlists::french::NFKD,
            Language::Italian => wordlists::italian::NFKD,
            Language::Czech => wordlists::czech::NFKD,
            Language::Portuguese => wordlists::portuguese::NFKD,
        }
    }

    /// The 2048 words as a reader sees them: the NFC form, which composes
    /// the Japanese list's voicing marks and the Korean list's conjoining
    /// jamo. Never used for derivation or for matching typed input.
    pub fn words_display(self) -> &'static [&'static str; 2048] {
        match self {
            Language::English => wordlists::english::DISPLAY,
            Language::Japanese => wordlists::japanese::DISPLAY,
            Language::Korean => wordlists::korean::DISPLAY,
            Language::Spanish => wordlists::spanish::DISPLAY,
            Language::ChineseSimplified => wordlists::chinese_simplified::DISPLAY,
            Language::ChineseTraditional => wordlists::chinese_traditional::DISPLAY,
            Language::French => wordlists::french::DISPLAY,
            Language::Italian => wordlists::italian::DISPLAY,
            Language::Czech => wordlists::czech::DISPLAY,
            Language::Portuguese => wordlists::portuguese::DISPLAY,
        }
    }

    /// Hex SHA-256 of the published wordlist file, for self-tests.
    pub fn wordlist_sha256(self) -> &'static str {
        match self {
            Language::English => wordlists::english::SHA256,
            Language::Japanese => wordlists::japanese::SHA256,
            Language::Korean => wordlists::korean::SHA256,
            Language::Spanish => wordlists::spanish::SHA256,
            Language::ChineseSimplified => wordlists::chinese_simplified::SHA256,
            Language::ChineseTraditional => wordlists::chinese_traditional::SHA256,
            Language::French => wordlists::french::SHA256,
            Language::Italian => wordlists::italian::SHA256,
            Language::Czech => wordlists::czech::SHA256,
            Language::Portuguese => wordlists::portuguese::SHA256,
        }
    }

    /// The word at `idx`, as published.
    ///
    /// # Panics
    /// If `idx >= 2048`. Indices produced by this module are always in
    /// range; validate external input with [`index_of`](Self::index_of) or
    /// [`Mnemonic::from_indices`] first.
    pub fn word(self, idx: u16) -> &'static str {
        self.words()[usize::from(idx)]
    }

    /// The word at `idx` in NFKD form. Panics like [`word`](Self::word).
    pub fn word_nfkd(self, idx: u16) -> &'static str {
        self.words_nfkd()[usize::from(idx)]
    }

    /// The word at `idx` as a reader sees it. Panics like
    /// [`word`](Self::word).
    pub fn word_display(self, idx: u16) -> &'static str {
        self.words_display()[usize::from(idx)]
    }

    /// Index of `word`, matching either the published or the NFKD form
    /// exactly. No normalization is applied to `word`; callers that accept
    /// free-form input must supply the stored form (the on-screen keyboard
    /// does, since it only offers words from the list).
    pub fn index_of(self, word: &str) -> Option<u16> {
        let words = self.words();
        let nfkd = self.words_nfkd();
        (0..wordlists::WORDLIST_LEN)
            .find(|&i| words[i] == word || nfkd[i] == word)
            .map(|i| i as u16)
    }

    /// Indices of every word whose published form starts with `prefix`,
    /// in list order. Drives the predictive keyboard.
    pub fn candidates<'a>(self, prefix: &'a str) -> impl Iterator<Item = u16> + 'a {
        let words = self.words();
        (0..wordlists::WORDLIST_LEN as u16)
            .filter(move |&i| words[usize::from(i)].starts_with(prefix))
    }

    /// Whether the list is written in Latin script, so that its words can
    /// be typed on the letters-only BIP-39 keyboard after
    /// [`fold`]ing. False for Japanese, Korean and Chinese.
    pub fn is_latin(self) -> bool {
        matches!(self.script(), Script::Latin)
    }

    /// How the list is typed, which is which keyboard it needs.
    pub fn script(self) -> Script {
        match self {
            Language::Japanese => Script::Kana,
            Language::Korean => Script::Jamo,
            Language::ChineseSimplified => Script::Pinyin,
            Language::ChineseTraditional => Script::Zhuyin,
            _ => Script::Latin,
        }
    }

    /// The keys that type the word at `idx`, in order.
    ///
    /// Latin lists: the ASCII fold, so `niño` types `nino`. Japanese: the
    /// NFKD word itself, which is base kana, the combining voicing marks
    /// and the small kana. Korean: the two-set key sequence of the word's
    /// conjoining jamo ([`crate::hangul`]), so a consonant that ends one
    /// syllable and starts the next is the same key either way. `None`
    /// for the two Chinese lists, which are read rather than spelled
    /// ([`candidates_pinyin`](Self::candidates_pinyin)).
    ///
    /// Panics like [`word`](Self::word).
    pub fn typed(self, idx: u16) -> Option<Typed> {
        let word = self.word_nfkd(idx);
        match self.script() {
            Script::Latin => fold(word),
            Script::Kana => {
                let mut out = Typed::new();
                for c in word.chars() {
                    if !out.push(c) {
                        return None;
                    }
                }
                Some(out)
            }
            Script::Jamo => {
                let mut out = Typed::new();
                for jamo in word.chars() {
                    for &key in crate::hangul::keys_of(jamo)? {
                        if !out.push(key) {
                            return None;
                        }
                    }
                }
                Some(out)
            }
            Script::Pinyin | Script::Zhuyin => None,
        }
    }

    /// Indices of every word whose typed form (see [`typed`](Self::typed))
    /// starts with `prefix`, in list order. Drives the predictive
    /// keyboard; yields nothing for a list that is read rather than
    /// spelled.
    pub fn candidates_typed<'a>(self, prefix: &'a [char]) -> impl Iterator<Item = u16> + 'a {
        (0..wordlists::WORDLIST_LEN as u16)
            .filter(move |&i| self.typed(i).is_some_and(|t| t.starts_with(prefix)))
    }

    /// Characters in the longest word of this list as a reader sees it,
    /// which is what a panel of words is sized for.
    pub fn max_display_chars(self) -> usize {
        match self {
            Language::English => wordlists::english::MAX_DISPLAY_CHARS,
            Language::Japanese => wordlists::japanese::MAX_DISPLAY_CHARS,
            Language::Korean => wordlists::korean::MAX_DISPLAY_CHARS,
            Language::Spanish => wordlists::spanish::MAX_DISPLAY_CHARS,
            Language::ChineseSimplified => wordlists::chinese_simplified::MAX_DISPLAY_CHARS,
            Language::ChineseTraditional => wordlists::chinese_traditional::MAX_DISPLAY_CHARS,
            Language::French => wordlists::french::MAX_DISPLAY_CHARS,
            Language::Italian => wordlists::italian::MAX_DISPLAY_CHARS,
            Language::Czech => wordlists::czech::MAX_DISPLAY_CHARS,
            Language::Portuguese => wordlists::portuguese::MAX_DISPLAY_CHARS,
        }
    }

    /// Whether the list is written with Chinese characters, so that its
    /// words are looked up by a Mandarin reading rather than by spelling:
    /// [`readings`](Self::readings) and the two prefix searches apply to
    /// these two lists only.
    pub fn is_hanzi(self) -> bool {
        matches!(
            self,
            Language::ChineseSimplified | Language::ChineseTraditional
        )
    }

    /// The Mandarin readings of `c`: an index into
    /// [`SYLLABLES`](wordlists::readings::SYLLABLES) and a tone, 1 to 4 as
    /// it is marked and 5 for the neutral tone. The first is the reading a
    /// dictionary heads the entry with. Empty for a character no Chinese
    /// list contains.
    pub fn readings(self, c: char) -> &'static [(u16, u8)] {
        if !self.is_hanzi() {
            return &[];
        }
        readings_of(c)
    }

    /// Indices of every word of this list read with a syllable whose
    /// toneless pinyin starts with `prefix`, or, once a tone is given, is exactly `prefix` (`lv` for `lü`), in list
    /// order, which for the two Chinese lists is roughly the order of use.
    /// `tone` narrows the match to one tone, 1 to 5. Yields nothing for a
    /// list that is not written with Chinese characters.
    ///
    /// Each word of those lists is one character, so this is the candidate
    /// strip the pinyin keyboard offers.
    pub fn candidates_pinyin<'a>(
        self,
        prefix: &'a str,
        tone: Option<u8>,
    ) -> impl Iterator<Item = u16> + 'a {
        self.candidates_read(prefix, tone, |s| s.pinyin)
    }

    /// The same by 注音 spelling: every word read with a syllable whose
    /// bopomofo starts with `prefix`, or, once a tone is given, is exactly `prefix`. Drives the 注音 keyboard.
    pub fn candidates_zhuyin<'a>(
        self,
        prefix: &'a str,
        tone: Option<u8>,
    ) -> impl Iterator<Item = u16> + 'a {
        self.candidates_read(prefix, tone, |s| s.zhuyin)
    }

    /// The readings of the word at `idx` as this list's own keyboard
    /// types them: the syllable's spelling — pinyin for Simplified, 注音
    /// for Traditional — and the tone, 1 to 5. Empty for a list that is
    /// spelled rather than read.
    ///
    /// This is what the keyboard needs that a prefix search cannot give:
    /// which key continues a spelling, which tone ends it, and whether
    /// what has been typed is already a whole syllable.
    pub fn word_readings(self, idx: u16) -> impl Iterator<Item = (&'static str, u8)> {
        let zhuyin = self.script() == Script::Zhuyin;
        let hanzi = self
            .is_hanzi()
            .then(|| {
                let mut chars = self.word(idx).chars();
                match (chars.next(), chars.next()) {
                    (Some(c), None) => Some(c),
                    _ => None,
                }
            })
            .flatten();
        hanzi.into_iter().flat_map(move |c| {
            readings_of(c).iter().map(move |&(syllable, tone)| {
                let s = &wordlists::readings::SYLLABLES[usize::from(syllable)];
                (if zhuyin { s.zhuyin } else { s.pinyin }, tone)
            })
        })
    }

    fn candidates_read<'a>(
        self,
        prefix: &'a str,
        tone: Option<u8>,
        spelling: fn(&'static wordlists::readings::Syllable) -> &'static str,
    ) -> impl Iterator<Item = u16> + 'a {
        let hanzi = self.is_hanzi();
        let words = self.words();
        (0..wordlists::WORDLIST_LEN as u16).filter(move |&i| {
            if !hanzi {
                return false;
            }
            let word = words[usize::from(i)];
            let mut chars = word.chars();
            let (Some(c), None) = (chars.next(), chars.next()) else {
                return false;
            };
            readings_of(c).iter().any(|&(syllable, t)| {
                let spelt = spelling(&wordlists::readings::SYLLABLES[usize::from(syllable)]);
                // A tone ends the syllable, so with one the spelling is
                // whole; without one it is still being typed.
                match tone {
                    Some(want) => want == t && spelt == prefix,
                    None => spelt.starts_with(prefix),
                }
            })
        })
    }

    /// Separator to put between words when displaying a sentence: the
    /// ideographic space U+3000 for Japanese, ASCII space otherwise.
    ///
    /// Seed derivation does not use this: BIP-39 derives from the NFKD form
    /// of the sentence, and NFKD maps U+3000 to U+0020, so the password is
    /// always joined with ASCII spaces.
    pub fn separator(self) -> &'static str {
        match self {
            Language::Japanese => "\u{3000}",
            _ => " ",
        }
    }
}

/// The readings of `c` in [`wordlists::readings::READINGS`], empty when
/// no Chinese list contains it. Binary search over the sorted table.
fn readings_of(c: char) -> &'static [(u16, u8)] {
    wordlists::readings::READINGS
        .binary_search_by_key(&c, |&(k, _)| k)
        .map_or(&[], |i| wordlists::readings::READINGS[i].1)
}

/// How a wordlist is typed: which keyboard its words are spelled on, or
/// which reading they are looked up by.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Script {
    /// Letters, after folding the accents off ([`fold`]).
    Latin,
    /// Hiragana, with the voicing marks and the small forms.
    Kana,
    /// The two-set Hangul keyboard ([`crate::hangul`]).
    Jamo,
    /// Chinese characters, looked up by their Mandarin reading in pinyin.
    Pinyin,
    /// Chinese characters, looked up by their reading in 注音.
    Zhuyin,
}

/// Longest typed sequence over all ten lists, in keys.
pub const MAX_TYPED: usize = 16;

/// The keys that type one word, inline. Latin lists hold letters, the
/// Japanese list its NFKD kana and marks, the Korean list two-set keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Typed {
    buf: [char; MAX_TYPED],
    len: u8,
}

impl Default for Typed {
    fn default() -> Self {
        Self::new()
    }
}

impl Typed {
    /// Nothing typed.
    pub const fn new() -> Self {
        Typed {
            buf: ['\0'; MAX_TYPED],
            len: 0,
        }
    }

    /// The keys, in the order they are pressed.
    pub fn as_chars(&self) -> &[char] {
        &self.buf[..usize::from(self.len)]
    }

    /// How many keys.
    pub fn len(&self) -> usize {
        usize::from(self.len)
    }

    /// Whether nothing has been typed.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Adds a key. False when there is no room for it.
    pub fn push(&mut self, c: char) -> bool {
        if usize::from(self.len) >= MAX_TYPED {
            return false;
        }
        self.buf[usize::from(self.len)] = c;
        self.len += 1;
        true
    }

    /// Removes the last key and returns it.
    pub fn pop(&mut self) -> Option<char> {
        if self.len == 0 {
            return None;
        }
        self.len -= 1;
        let c = self.buf[usize::from(self.len)];
        self.buf[usize::from(self.len)] = '\0';
        Some(c)
    }

    /// The last key.
    pub fn last(&self) -> Option<char> {
        (self.len > 0).then(|| self.buf[usize::from(self.len) - 1])
    }

    /// Replaces the last key.
    pub fn set_last(&mut self, c: char) {
        if self.len > 0 {
            self.buf[usize::from(self.len) - 1] = c;
        }
    }

    /// Whether these keys start with `prefix`.
    pub fn starts_with(&self, prefix: &[char]) -> bool {
        self.as_chars().starts_with(prefix)
    }

    /// Forgets everything typed.
    pub fn clear(&mut self) {
        self.buf = ['\0'; MAX_TYPED];
        self.len = 0;
    }
}

impl Zeroize for Typed {
    fn zeroize(&mut self) {
        self.clear();
    }
}

/// Folds an NFKD word to lower-case ASCII letters by dropping combining
/// marks (U+0300–U+036F), so `niño` folds to `nino` and `ěšč` to `esc`.
/// Returns `None` if anything else non-ASCII remains or the result
/// exceeds [`MAX_TYPED`].
pub fn fold(word: &str) -> Option<Typed> {
    let mut out = Typed::new();
    for c in word.chars() {
        if ('\u{0300}'..='\u{036F}').contains(&c) {
            continue;
        }
        if !c.is_ascii_alphabetic() || !out.push(c.to_ascii_lowercase()) {
            return None;
        }
    }
    Some(out)
}

/// Raw entropy behind a mnemonic: 16, 20, 24, 28 or 32 bytes.
pub struct Entropy {
    bytes: [u8; 32],
    len: u8,
}

impl Entropy {
    /// The entropy bytes.
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes[..usize::from(self.len)]
    }
}

impl AsRef<[u8]> for Entropy {
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}

impl Zeroize for Entropy {
    fn zeroize(&mut self) {
        self.bytes.zeroize();
        self.len.zeroize();
    }
}

/// A validated mnemonic: language, word count and word indices.
///
/// Stored inline on the stack, zeroized on drop, never printed or cloned.
/// Word `i` is `words[i]`; only the first `word_count()` entries are used.
pub struct Mnemonic {
    lang: Language,
    len: u8,
    words: [u16; MAX_WORDS],
}

impl Zeroize for Mnemonic {
    fn zeroize(&mut self) {
        self.words.zeroize();
        self.len.zeroize();
    }
}

impl Drop for Mnemonic {
    fn drop(&mut self) {
        self.zeroize();
    }
}

impl ZeroizeOnDrop for Mnemonic {}

impl Mnemonic {
    /// Builds the mnemonic for `entropy` (16, 20, 24, 28 or 32 bytes).
    pub fn from_entropy(lang: Language, entropy: &[u8]) -> Result<Self, Error> {
        if !matches!(entropy.len(), 16 | 20 | 24 | 28 | 32) {
            return Err(Error::InvalidEntropyLength);
        }
        let word_count = entropy.len() * 3 / 4;

        let mut bits = Secret::new([0u8; BITS_BUF]);
        bits.expose_mut()[..entropy.len()].copy_from_slice(entropy);
        bits.expose_mut()[entropy.len()] = checksum_byte(entropy);
        // The checksum byte's low bits lie beyond the sentence and are never
        // read.

        let mut words = [0u16; MAX_WORDS];
        for (i, w) in words.iter_mut().enumerate().take(word_count) {
            *w = get11(bits.expose(), i);
        }
        Ok(Self {
            lang,
            len: word_count as u8,
            words,
        })
    }

    /// Builds a mnemonic from word indices, checking count and checksum.
    pub fn from_indices(lang: Language, indices: &[u16]) -> Result<Self, Error> {
        if !matches!(indices.len(), 12 | 15 | 18 | 21 | 24) {
            return Err(Error::InvalidWordCount);
        }
        if let Some(position) = indices
            .iter()
            .position(|&w| usize::from(w) >= wordlists::WORDLIST_LEN)
        {
            return Err(Error::UnknownWord {
                position: position as u8,
            });
        }
        let mut words = [0u16; MAX_WORDS];
        words[..indices.len()].copy_from_slice(indices);
        let m = Self {
            lang,
            len: indices.len() as u8,
            words,
        };
        m.validate_checksum()?;
        Ok(m)
    }

    /// Parses a sentence split on Unicode whitespace (which includes the
    /// ideographic space U+3000). Reports the position of the first word
    /// not found in `lang`'s list, so a typo can be pointed at.
    pub fn parse(lang: Language, sentence: &str) -> Result<Self, Error> {
        let mut indices = Secret::new([0u16; MAX_WORDS]);
        let mut count = 0usize;
        for (position, word) in sentence.split_whitespace().enumerate() {
            if position >= MAX_WORDS {
                return Err(Error::InvalidWordCount);
            }
            let idx = lang.index_of(word).ok_or(Error::UnknownWord {
                position: position as u8,
            })?;
            indices.expose_mut()[position] = idx;
            count = position + 1;
        }
        Self::from_indices(lang, &indices.expose()[..count])
    }

    /// Number of words: 12, 15, 18, 21 or 24.
    pub fn word_count(&self) -> usize {
        usize::from(self.len)
    }

    /// The word indices, `word_count()` of them.
    pub fn indices(&self) -> &[u16] {
        &self.words[..self.word_count()]
    }

    /// The wordlist this mnemonic uses.
    pub fn language(&self) -> Language {
        self.lang
    }

    /// The entropy the mnemonic encodes.
    pub fn entropy(&self) -> Secret<Entropy> {
        let mut bits = Secret::new([0u8; BITS_BUF]);
        for (i, &w) in self.indices().iter().enumerate() {
            set11(bits.expose_mut(), i, w);
        }
        let len = entropy_len(self.word_count());
        let mut e = Secret::new(Entropy {
            bytes: [0u8; 32],
            len: len as u8,
        });
        e.expose_mut().bytes[..len].copy_from_slice(&bits.expose()[..len]);
        e
    }

    /// The checksum carried in the last word: `(value, bit_count)`.
    /// `bit_count` is 4 for 12 words up to 8 for 24 words; `value` occupies
    /// its low `bit_count` bits.
    pub fn checksum_bits(&self) -> (u8, u8) {
        let cs = checksum_bit_count(self.word_count());
        let last = self.words[self.word_count() - 1];
        ((last & ((1u16 << cs) - 1)) as u8, cs as u8)
    }

    /// Derives the 64-byte BIP-39 seed with PBKDF2-HMAC-SHA512, 2048
    /// rounds. `passphrase` must be printable ASCII (0x20..=0x7E) and at
    /// most [`MAX_PASSPHRASE_BYTES`] long; ASCII is NFKD-invariant, so no
    /// normalization is needed (`docs/PLANNING.md` §16.2).
    pub fn to_seed(&self, passphrase: &[u8]) -> Result<Secret<[u8; 64]>, Error> {
        if passphrase.iter().any(|&b| !(0x20..=0x7E).contains(&b)) {
            return Err(Error::PassphraseNotAscii);
        }
        self.to_seed_unchecked(passphrase)
    }

    /// Like [`to_seed`](Self::to_seed) but takes the passphrase as
    /// already-NFKD-normalized bytes without the ASCII check. Its callers
    /// are the vector tests and the start-up self-test, which run the
    /// Japanese BIP-39 vector and its non-ASCII passphrase; nothing a
    /// person types reaches it, and no flow calls it.
    #[doc(hidden)]
    pub fn to_seed_unchecked(&self, passphrase_nfkd: &[u8]) -> Result<Secret<[u8; 64]>, Error> {
        if passphrase_nfkd.len() > MAX_PASSPHRASE_BYTES {
            return Err(Error::PassphraseTooLong);
        }

        let mut sentence = Secret::new([0u8; MAX_SENTENCE_BYTES]);
        let mut n = 0usize;
        for (i, &w) in self.indices().iter().enumerate() {
            let buf = sentence.expose_mut();
            if i > 0 {
                buf[n] = b' ';
                n += 1;
            }
            let word = self.lang.word_nfkd(w).as_bytes();
            buf[n..n + word.len()].copy_from_slice(word);
            n += word.len();
        }

        let mut salt = Secret::new([0u8; SALT_PREFIX.len() + MAX_PASSPHRASE_BYTES]);
        let salt_len = SALT_PREFIX.len() + passphrase_nfkd.len();
        salt.expose_mut()[..SALT_PREFIX.len()].copy_from_slice(SALT_PREFIX);
        salt.expose_mut()[SALT_PREFIX.len()..salt_len].copy_from_slice(passphrase_nfkd);

        let mut seed = Secret::new([0u8; 64]);
        pbkdf2_hmac_sha512(
            &sentence.expose()[..n],
            &salt.expose()[..salt_len],
            PBKDF2_ROUNDS,
            seed.expose_mut(),
        );
        Ok(seed)
    }

    fn validate_checksum(&self) -> Result<(), Error> {
        let mut bits = Secret::new([0u8; BITS_BUF]);
        for (i, &w) in self.indices().iter().enumerate() {
            set11(bits.expose_mut(), i, w);
        }
        let len = entropy_len(self.word_count());
        let cs = checksum_bit_count(self.word_count());
        let expected = checksum_byte(&bits.expose()[..len]) >> (8 - cs);
        let (actual, _) = self.checksum_bits();
        if expected == actual {
            Ok(())
        } else {
            Err(Error::BadChecksum)
        }
    }
}

/// Every valid final word for an incomplete mnemonic of 11, 14, 17, 20 or
/// 23 words. The last word carries `11 - checksum_bits` free entropy bits
/// followed by the checksum, so there are `2^(11 - checksum_bits)`
/// candidates: 128 for a 12-word mnemonic down to 8 for 24 words. They are
/// yielded in increasing index order. Indices are language-independent, so
/// no language is needed; look the results up with [`Language::word`].
pub fn last_word_candidates(first_words: &[u16]) -> Result<LastWordCandidates, Error> {
    if !matches!(first_words.len(), 11 | 14 | 17 | 20 | 23) {
        return Err(Error::InvalidWordCount);
    }
    if let Some(position) = first_words
        .iter()
        .position(|&w| usize::from(w) >= wordlists::WORDLIST_LEN)
    {
        return Err(Error::UnknownWord {
            position: position as u8,
        });
    }
    let word_count = first_words.len() + 1;
    let mut bits = [0u8; BITS_BUF];
    for (i, &w) in first_words.iter().enumerate() {
        set11(&mut bits, i, w);
    }
    Ok(LastWordCandidates {
        bits,
        word_count,
        next: 0,
    })
}

/// Iterator returned by [`last_word_candidates`]. Holds the entropy prefix
/// and zeroizes it on drop.
pub struct LastWordCandidates {
    bits: [u8; BITS_BUF],
    word_count: usize,
    next: u16,
}

impl Iterator for LastWordCandidates {
    type Item = u16;

    fn next(&mut self) -> Option<u16> {
        let cs = checksum_bit_count(self.word_count);
        let free_bits = 11 - cs;
        if self.next >= (1u16 << free_bits) {
            return None;
        }
        let free = self.next;
        self.next += 1;

        // Place the free bits where the last word's entropy bits go, then
        // recompute the checksum over the completed entropy.
        let mut bits = Secret::new(self.bits);
        set11(bits.expose_mut(), self.word_count - 1, free << cs);
        let len = entropy_len(self.word_count);
        let checksum = checksum_byte(&bits.expose()[..len]) >> (8 - cs);
        Some((free << cs) | u16::from(checksum))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let total = 1usize << (11 - checksum_bit_count(self.word_count));
        let left = total - usize::from(self.next);
        (left, Some(left))
    }
}

impl ExactSizeIterator for LastWordCandidates {}

impl Zeroize for LastWordCandidates {
    fn zeroize(&mut self) {
        self.bits.zeroize();
    }
}

impl Drop for LastWordCandidates {
    fn drop(&mut self) {
        self.zeroize();
    }
}

impl ZeroizeOnDrop for LastWordCandidates {}

/// Checksum length in bits for a word count: `words / 3`.
fn checksum_bit_count(word_count: usize) -> usize {
    word_count / 3
}

/// Entropy length in bytes for a word count: `words * 4 / 3`.
fn entropy_len(word_count: usize) -> usize {
    word_count * 4 / 3
}

/// First byte of SHA-256(entropy); the checksum is its top bits.
fn checksum_byte(entropy: &[u8]) -> u8 {
    sha256(entropy)[0]
}

/// Reads the 11-bit group `i` (MSB-first) from a packed bit buffer.
fn get11(buf: &[u8], i: usize) -> u16 {
    let mut v = 0u16;
    for b in 0..11 {
        let bit = 11 * i + b;
        let set = (buf[bit / 8] >> (7 - bit % 8)) & 1;
        v = (v << 1) | u16::from(set);
    }
    v
}

/// Writes the 11-bit group `i` (MSB-first) into a packed bit buffer.
fn set11(buf: &mut [u8], i: usize, v: u16) {
    for b in 0..11 {
        let bit = 11 * i + b;
        if (v >> (10 - b)) & 1 == 1 {
            buf[bit / 8] |= 1 << (7 - bit % 8);
        } else {
            buf[bit / 8] &= !(1 << (7 - bit % 8));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn folding_strips_combining_marks_only() {
        assert_eq!(
            fold("abandon").unwrap().as_chars(),
            "abandon".chars().collect::<alloc::vec::Vec<_>>()
        );
        assert_eq!(
            fold("nin\u{0303}o").unwrap().as_chars(),
            ['n', 'i', 'n', 'o']
        );
        assert_eq!(fold("e\u{030C}s\u{030C}").unwrap().as_chars(), ['e', 's']);
        assert_eq!(fold("\u{3042}"), None, "non-Latin");
        assert_eq!(fold("abandonabandonabc"), None, "too long");
    }

    /// Every word of every list a keyboard can type is reachable by
    /// typing its keys, and is the only candidate left at the end.
    #[test]
    fn every_word_is_typed_by_its_own_keys() {
        for lang in Language::ALL {
            if matches!(lang.script(), Script::Pinyin | Script::Zhuyin) {
                assert!(lang.typed(0).is_none(), "{lang:?}");
                continue;
            }
            let mut seen = alloc::collections::BTreeSet::new();
            for i in 0..2048u16 {
                let typed = lang.typed(i).unwrap_or_else(|| panic!("{lang:?} {i}"));
                assert!(!typed.is_empty(), "{lang:?} {i}");
                assert!(
                    lang.candidates_typed(typed.as_chars()).any(|j| j == i),
                    "{lang:?} {i} does not answer to its own keys"
                );
                assert!(
                    seen.insert(alloc::vec::Vec::from(typed.as_chars())),
                    "{lang:?} {i} types the same keys as another word"
                );
            }
        }
    }

    #[test]
    fn typed_candidates_match_accented_words() {
        // Spanish "ábaco" is word 0; its typed form starts with "ab", the
        // published form does not.
        assert_eq!(
            Language::Spanish.candidates_typed(&['a', 'b']).next(),
            Some(0)
        );
        assert_ne!(Language::Spanish.candidates("ab").next(), Some(0));
        // English: folding is the identity.
        assert_eq!(
            Language::English.candidates_typed(&['z', 'o']).count(),
            Language::English.candidates("zo").count()
        );
        assert_eq!(Language::English.candidates_typed(&[]).count(), 2048);
        // The Chinese lists are read, not spelled.
        assert_eq!(Language::ChineseSimplified.candidates_typed(&[]).count(), 0);
    }

    #[test]
    fn sentence_buffer_is_large_enough() {
        for lang in Language::ALL {
            let longest = lang.words_nfkd().iter().map(|w| w.len()).max().unwrap();
            assert!(MAX_WORDS * longest + MAX_WORDS - 1 <= MAX_SENTENCE_BYTES);
        }
    }
}
