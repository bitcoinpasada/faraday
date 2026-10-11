//! Explore (`docs/DESIGN.md` §5): the live BIP-39/BIP-32 workspace's
//! state. It works on a loaded key, or, with none loaded, on words typed
//! through the Load wizard's entry step, which hands its indices here
//! without adding a key.
//!
//! The area is a Menu of rows and one screen behind each of them
//! ([`Step`]): the key chooser is a Choice, the path and the passphrase
//! are Entry screens, "Words and bits" is a second Menu, the words are
//! the Words screen and each long secret its own Secret screen.
//!
//! The typed words and passphrase live inline in this struct and are
//! zeroized when Explore is left; the [`MasterKey`] built from them is
//! held with them (it is what the typed words are for, and rebuilding it
//! means a PBKDF2 run per frame) and erases itself on drop. The seed is
//! never kept: the seed row derives it while its hold button is held.
//! Indices and 11-bit groups are treated as secret too: each reveals the
//! word. This file is listed in `tools/lint-secrets.sh` and may hold no
//! heap text; the derivation path is text the user typed and is public.

use osk_bip::bip39::{Language, MAX_PASSPHRASE_BYTES, MAX_WORDS, Mnemonic};
use osk_bip::keys::{ChildNumber, DerivationPath, MasterKey, Network, ScriptType};
use osk_crypto::{Zeroize, ZeroizeOnDrop};

pub use crate::finish::MASK_MS;

/// Deepest path the explorer derives.
pub const MAX_PATH_LEVELS: usize = 10;
/// Longest path text after `m/`: ten levels of `2147483647h/`.
pub const MAX_PATH_BYTES: usize = MAX_PATH_LEVELS * 12;

/// The screen of the workspace on display. Every one of them is a
/// reusable screen of `docs/DESIGN.md` §5, and a row of the key menu
/// opens it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// The key menu, which the area opens on.
    Keys,
    /// The Choice that picks what to explore.
    Chooser,
    /// The Entry that types the derivation path.
    Path,
    /// The Entry that types the passphrase (typed words only).
    Passphrase,
    /// The "Words and bits" menu.
    Bits,
    /// The Words screen.
    Words,
    /// One secret on its own Secret screen.
    Secret(SecretKind),
    /// The Result that asks before typed words are dropped.
    Discard,
}

impl Step {
    /// Which of the two menus this screen belongs to, for the eye's
    /// reveal scope and for what the chevron returns to.
    fn menu(self) -> Step {
        match self {
            Step::Words => Step::Bits,
            Step::Secret(secret) => secret.menu(),
            _ => Step::Keys,
        }
    }

    /// A number that tells one Explore screen from another, so that the
    /// app bar's eye stops when the screen changes.
    pub fn scope(self) -> u8 {
        match self {
            Step::Keys => 0,
            Step::Chooser => 1,
            Step::Path => 2,
            Step::Passphrase => 3,
            Step::Bits => 4,
            Step::Words => 5,
            Step::Discard => 6,
            Step::Secret(secret) => 7 + secret.index() as u8,
        }
    }
}

/// The long secrets Explore shows, each on a Secret screen of its own
/// (§2.8: "Long secrets and long strings live on their own screen,
/// reached by a row").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecretKind {
    /// The extended private key at the applied path.
    Xprv,
    /// The entropy the words encode.
    Entropy,
    /// The checksum bits, as ones and zeros.
    ChecksumBits,
    /// The 64-byte seed.
    Seed,
    /// The master private key.
    MasterXprv,
}

impl SecretKind {
    /// Every secret, in the order the two menus list them.
    pub const ALL: [SecretKind; 5] = [
        SecretKind::Xprv,
        SecretKind::Entropy,
        SecretKind::ChecksumBits,
        SecretKind::Seed,
        SecretKind::MasterXprv,
    ];

    /// Position in [`ALL`](Self::ALL).
    pub fn index(self) -> usize {
        SecretKind::ALL
            .iter()
            .position(|s| *s == self)
            .expect("every secret is listed")
    }

    /// The menu whose row opens it.
    fn menu(self) -> Step {
        match self {
            SecretKind::Xprv => Step::Keys,
            _ => Step::Bits,
        }
    }
}

/// What the workspace derives from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    /// Nothing yet: no key loaded and no words typed.
    None,
    /// The loaded key at this index.
    Loaded(usize),
    /// The words typed into Explore.
    Typed,
}

/// Why the path text does not parse. The last valid path stays applied.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathError {
    /// Two slashes in a row, or a slash first: `m//0`.
    EmptyLevel,
    /// A level ends with a slash and nothing after it.
    Unfinished,
    /// A level is not digits with an optional `h`.
    NotANumber,
    /// An index of 2³¹ or more.
    IndexTooLarge,
    /// More than [`MAX_PATH_LEVELS`] levels.
    TooDeep,
}

impl PathError {
    /// The inline message under the path field.
    pub fn message(self, s: &crate::strings::Strings) -> &'static str {
        match self {
            PathError::EmptyLevel => s.explore_path_empty,
            PathError::Unfinished => s.explore_path_unfinished,
            PathError::NotANumber => s.explore_path_not_a_number,
            PathError::IndexTooLarge => s.explore_path_too_large,
            PathError::TooDeep => s.explore_path_too_deep,
        }
    }
}

/// Parses path text without its `m/` prefix into a derivation path.
/// Empty text is the master itself.
pub fn parse_path(text: &str) -> Result<DerivationPath, PathError> {
    let mut levels = [ChildNumber::Normal { index: 0 }; MAX_PATH_LEVELS];
    let mut n = 0usize;
    if text.is_empty() {
        return Ok(DerivationPath::from(&levels[..0]));
    }
    for (i, level) in text.split('/').enumerate() {
        if level.is_empty() {
            return Err(if i > 0 && i + 1 == text.split('/').count() {
                PathError::Unfinished
            } else {
                PathError::EmptyLevel
            });
        }
        if n == MAX_PATH_LEVELS {
            return Err(PathError::TooDeep);
        }
        let (digits, hardened) = match level.strip_suffix('h') {
            Some(d) => (d, true),
            None => (level, false),
        };
        if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
            return Err(PathError::NotANumber);
        }
        let index: u32 = digits.parse().map_err(|_| PathError::IndexTooLarge)?;
        levels[n] = if hardened {
            ChildNumber::from_hardened_idx(index).map_err(|_| PathError::IndexTooLarge)?
        } else {
            ChildNumber::from_normal_idx(index).map_err(|_| PathError::IndexTooLarge)?
        };
        n += 1;
    }
    Ok(DerivationPath::from(&levels[..n]))
}

/// The script type a path's purpose level implies, when it is one of
/// BIP-44/49/84/86 and hardened.
pub fn implied_script(path: &DerivationPath) -> Option<ScriptType> {
    match path.as_ref().first()? {
        ChildNumber::Hardened { index } => ScriptType::ALL
            .iter()
            .copied()
            .find(|s| s.purpose() == *index),
        ChildNumber::Normal { .. } => None,
    }
}

/// The account index every purpose preset of the path editor writes: the
/// first account (`docs/DESIGN.md` §4.6).
pub const PRESET_ACCOUNT: u32 = 0;

/// Levels of a path down to and including the account: purpose, coin,
/// account. A path with more than these has an account key of its own
/// above its leaf; a shorter one does not, and its leaf *is* the
/// account key the key menu would show (§4.5).
pub const ACCOUNT_LEVELS: usize = 3;

/// Which level of a path is the chain: purpose, coin, account, chain.
const CHAIN_LEVEL: usize = ACCOUNT_LEVELS;

/// Levels in `text`; the empty path is the master itself and has none.
fn levels(text: &str) -> usize {
    if text.is_empty() {
        0
    } else {
        text.matches('/').count() + 1
    }
}

/// Byte offset of the `/` that begins level `n` of `text` (levels count
/// from zero, so `n` is at least one), or `None` where the path has no
/// such level.
fn level_start(text: &str, n: usize) -> Option<usize> {
    text.char_indices()
        .filter(|(_, c)| *c == '/')
        .map(|(i, _)| i)
        .nth(n - 1)
}

/// The chain level of a path: `Some(false)` on a receive path,
/// `Some(true)` on a change path, `None` where the path has no chain
/// level yet (`docs/DESIGN.md` §4.6's Chain pair).
pub fn chain_of(text: &str) -> Option<bool> {
    let start = level_start(text, CHAIN_LEVEL)?;
    match text[start + 1..].split('/').next()? {
        "0" => Some(false),
        "1" => Some(true),
        _ => None,
    }
}

/// Appends `src` to `out` at `len`, as far as it fits, and returns the
/// new length. The path is a fixed buffer, so its edits are byte copies
/// and never heap text (`tools/lint-secrets.sh` rule 3).
fn append(out: &mut [u8; MAX_PATH_BYTES], len: usize, src: &[u8]) -> usize {
    let take = src.len().min(MAX_PATH_BYTES - len);
    out[len..len + take].copy_from_slice(&src[..take]);
    len + take
}

/// `text` with its chain level set to receive or change, written into
/// `out`; returns the length. A path with no chain level yet is padded
/// out to one, so the pair always writes the level §4.6 names.
fn with_chain(text: &str, change: bool, out: &mut [u8; MAX_PATH_BYTES]) -> usize {
    let digit = if change { b"1" } else { b"0" };
    let bytes = text.as_bytes();
    let mut len = 0;
    if levels(text) > CHAIN_LEVEL {
        let start = level_start(text, CHAIN_LEVEL).unwrap_or(bytes.len());
        len = append(out, len, &bytes[..start]);
        len = append(out, len, b"/");
        len = append(out, len, digit);
        match level_start(text, CHAIN_LEVEL + 1) {
            Some(next) => append(out, len, &bytes[next..]),
            None => append(out, len, b"/0"),
        }
    } else {
        len = append(out, len, bytes);
        for _ in levels(text)..CHAIN_LEVEL {
            len = append(out, len, b"/0");
        }
        len = append(out, len, b"/");
        len = append(out, len, digit);
        append(out, len, b"/0")
    }
}

/// The levels of `text` from the chain level on — `/0/0` — written into
/// `out`; zero where the path has none. A purpose preset keeps them, so
/// the Purpose group and the Chain pair stay the two independent choices
/// §4.6 makes them.
fn chain_tail(text: &str, out: &mut [u8; MAX_PATH_BYTES]) -> usize {
    match level_start(text, CHAIN_LEVEL) {
        Some(start) => append(out, 0, &text.as_bytes()[start..]),
        None => 0,
    }
}

/// The workspace state. See the module documentation.
pub struct Explore {
    source: Source,
    /// What the chooser has checked, which Continue applies.
    picked: Source,
    step: Step,
    /// Page of the Words screen.
    word_page: u8,
    /// "Numbers" is on for the Words screen.
    numbers: bool,
    words: [u16; MAX_WORDS],
    word_count: u8,
    lang: Language,
    passphrase: [u8; MAX_PASSPHRASE_BYTES],
    passphrase_len: u16,
    typed_at: u64,
    /// The typed words' master key for the current network and
    /// passphrase.
    master: Option<MasterKey>,
    /// The session's curve blind, once it has been handed down, so that
    /// a key rebuilt while the passphrase is being typed is blinded like
    /// a loaded one (security review M1). `None` leaves a rebuilt key
    /// with the blind it derives for itself.
    blind: Option<[u8; 32]>,
    /// The path text the editor is typing, which ✓ applies.
    path: [u8; MAX_PATH_BYTES],
    path_len: u8,
    path_error: Option<PathError>,
    applied: DerivationPath,
}

impl Zeroize for Explore {
    fn zeroize(&mut self) {
        self.words.zeroize();
        self.word_count = 0;
        self.passphrase.zeroize();
        self.passphrase_len = 0;
        self.typed_at = 0;
        self.master = None;
        if self.source == Source::Typed {
            self.source = Source::None;
        }
        if self.picked == Source::Typed {
            self.picked = Source::None;
        }
        if self.step == Step::Discard {
            self.step = Step::Keys;
        }
    }
}

impl Drop for Explore {
    fn drop(&mut self) {
        self.zeroize();
    }
}

impl ZeroizeOnDrop for Explore {}

impl Explore {
    /// A workspace on nothing, at the BIP-84 receive path for `network`.
    pub fn new(network: Network) -> Self {
        let mut e = Explore {
            source: Source::None,
            picked: Source::None,
            step: Step::Keys,
            word_page: 0,
            numbers: false,
            words: [0; MAX_WORDS],
            word_count: 0,
            lang: Language::English,
            passphrase: [0; MAX_PASSPHRASE_BYTES],
            passphrase_len: 0,
            typed_at: 0,
            master: None,
            blind: None,
            path: [0; MAX_PATH_BYTES],
            path_len: 0,
            path_error: None,
            applied: DerivationPath::from(&[][..]),
        };
        e.preset(ScriptType::NativeSegwit, network);
        e.set_chain(false);
        e.check_path();
        e.apply();
        e
    }

    // ----- source -----

    /// What is being explored.
    pub fn source(&self) -> Source {
        self.source
    }

    /// Explores loaded key `key`.
    pub fn use_loaded(&mut self, key: usize) {
        self.source = Source::Loaded(key);
        self.picked = self.source;
    }

    /// Explores the typed words, if any.
    pub fn use_typed(&mut self) {
        if self.word_count > 0 {
            self.source = Source::Typed;
            self.picked = self.source;
        }
    }

    /// Sets the typed words and derives their master key for `network`
    /// under the current passphrase. Invalid words leave nothing.
    pub fn set_typed(&mut self, indices: &[u16], lang: Language, network: Network) -> bool {
        if Mnemonic::from_indices(lang, indices).is_err() {
            return false;
        }
        self.words.zeroize();
        self.words[..indices.len()].copy_from_slice(indices);
        self.word_count = indices.len() as u8;
        self.lang = lang;
        self.rebuild(network);
        self.source = Source::Typed;
        self.picked = self.source;
        true
    }

    /// Whether typed words are held.
    pub fn has_input(&self) -> bool {
        self.word_count > 0
    }

    /// The typed words' wordlist.
    pub fn typed_language(&self) -> Language {
        self.lang
    }

    /// The typed words as a mnemonic, while some are held.
    pub fn typed_mnemonic(&self) -> Option<Mnemonic> {
        if self.word_count == 0 {
            return None;
        }
        Mnemonic::from_indices(self.lang, &self.words[..usize::from(self.word_count)]).ok()
    }

    /// The typed words' master key.
    pub fn typed_master(&self) -> Option<&MasterKey> {
        self.master.as_ref()
    }

    /// Hands the session's curve blind down, and applies it to the key
    /// that is already built. Every key built after this takes it too.
    pub fn set_blind(&mut self, blind: [u8; 32]) {
        self.blind = Some(blind);
        if let Some(m) = &mut self.master {
            m.reblind(&blind);
        }
    }

    /// Rebuilds the typed words' master key for `network`.
    pub fn rebuild(&mut self, network: Network) {
        let blind = self.blind;
        self.master = self.typed_mnemonic().and_then(|m| {
            m.to_seed(self.passphrase_bytes()).ok().map(|seed| {
                let master = MasterKey::from_seed(&seed, network);
                match &blind {
                    Some(b) => master.blinded(b),
                    None => master,
                }
            })
        });
    }

    /// What the chooser has checked, which is not yet what is explored:
    /// §2.7 checks a row and confirms it with Continue.
    pub fn picked(&self) -> Source {
        self.picked
    }

    /// Checks `source` on the chooser.
    pub fn pick(&mut self, source: Source) {
        self.picked = source;
    }

    /// Continue on the chooser: what is checked becomes what is
    /// explored, and the key menu comes back.
    pub fn confirm_pick(&mut self) {
        match self.picked {
            Source::Typed => self.use_typed(),
            Source::Loaded(k) => self.use_loaded(k),
            Source::None => {}
        }
        self.step = Step::Keys;
    }

    /// A loaded key was removed: a source past it moves down, the
    /// removed one becomes nothing.
    pub fn key_removed(&mut self, key: usize) {
        for source in [&mut self.source, &mut self.picked] {
            match *source {
                Source::Loaded(k) if k == key => *source = Source::None,
                Source::Loaded(k) if k > key => *source = Source::Loaded(k - 1),
                _ => {}
            }
        }
    }

    // ----- screens -----

    /// The screen showing.
    pub fn step(&self) -> Step {
        self.step
    }

    /// Opens `step`. The path editor starts from the applied path, so
    /// what it types is what is on the screen behind it.
    pub fn go(&mut self, step: Step) {
        if step == Step::Path {
            self.path_error = None;
        }
        self.step = step;
    }

    /// The chevron inside the area: a screen returns to the menu whose
    /// row opened it, and a menu returns `false` so the area is left.
    /// Typed words are not dropped silently: leaving with some opens the
    /// discard confirm first.
    pub fn back(&mut self) -> bool {
        match self.step {
            Step::Keys => {
                if self.source == Source::Typed && self.has_input() {
                    self.step = Step::Discard;
                    return true;
                }
                false
            }
            Step::Discard => {
                self.step = Step::Keys;
                false
            }
            step => {
                self.step = step.menu();
                true
            }
        }
    }

    /// The page of the Words screen.
    pub fn word_page(&self) -> u8 {
        self.word_page
    }

    /// Shows word page `page`.
    pub fn set_word_page(&mut self, page: u8) {
        self.word_page = page;
    }

    /// Whether the Words screen shows each word's place in the wordlist.
    pub fn numbers(&self) -> bool {
        self.numbers
    }

    /// Turns those numbers on or off.
    pub fn toggle_numbers(&mut self) {
        self.numbers = !self.numbers;
    }

    // ----- passphrase (typed words only) -----

    /// The passphrase bytes.
    pub fn passphrase_bytes(&self) -> &[u8] {
        &self.passphrase[..usize::from(self.passphrase_len)]
    }

    /// Passphrase length in characters.
    pub fn passphrase_len(&self) -> usize {
        usize::from(self.passphrase_len)
    }

    /// Appends a printable ASCII character at `now_ms` and rebuilds the
    /// key for `network`.
    pub fn passphrase_push(&mut self, c: char, now_ms: u64, network: Network) -> bool {
        let n = usize::from(self.passphrase_len);
        if !c.is_ascii() || !(0x20..=0x7E).contains(&(c as u32)) || n >= MAX_PASSPHRASE_BYTES {
            return false;
        }
        self.passphrase[n] = c as u8;
        self.passphrase_len += 1;
        self.typed_at = now_ms;
        self.rebuild(network);
        true
    }

    /// Deletes the last passphrase character and rebuilds the key.
    pub fn passphrase_pop(&mut self, network: Network) {
        if self.passphrase_len > 0 {
            self.passphrase_len -= 1;
            self.passphrase[usize::from(self.passphrase_len)] = 0;
            self.rebuild(network);
        }
    }

    /// The last typed character while it is still unmasked at `now_ms`
    /// (`docs/PLANNING.md` §4.6).
    pub fn passphrase_visible_char(&self, now_ms: u64) -> Option<char> {
        if self.passphrase_len == 0 || now_ms >= self.mask_deadline()? {
            return None;
        }
        Some(self.passphrase[usize::from(self.passphrase_len) - 1] as char)
    }

    /// When the last typed character masks, if one is showing.
    pub fn mask_deadline(&self) -> Option<u64> {
        (self.step == Step::Passphrase && self.passphrase_len > 0)
            .then_some(self.typed_at + MASK_MS)
    }

    // ----- path -----

    /// The path text after `m/`, which is what the editor is typing.
    pub fn path_text(&self) -> &str {
        core::str::from_utf8(&self.path[..usize::from(self.path_len)]).expect("ascii")
    }

    /// The applied path: what every derived value on the key menu comes
    /// from. Only ✓ on the editor changes it (§4.6).
    pub fn applied_path(&self) -> &DerivationPath {
        &self.applied
    }

    /// Why the text does not parse, if it does not.
    pub fn path_error(&self) -> Option<PathError> {
        self.path_error
    }

    /// ✓ on the path editor: the text becomes the applied path and the
    /// key menu comes back. `false` while the text does not parse, which
    /// is what keeps ✓ dead (§4.6).
    pub fn apply(&mut self) -> bool {
        let Ok(path) = parse_path(self.path_text()) else {
            return false;
        };
        self.applied = path;
        self.path_error = None;
        self.step = Step::Keys;
        true
    }

    /// Types one character of the path.
    pub fn path_push(&mut self, c: char) -> bool {
        let n = usize::from(self.path_len);
        if !(c.is_ascii_digit() || c == '/' || c == 'h') || n >= MAX_PATH_BYTES {
            return false;
        }
        self.path[n] = c as u8;
        self.path_len += 1;
        self.check_path();
        true
    }

    /// Deletes the last path character.
    pub fn path_pop(&mut self) {
        if self.path_len > 0 {
            self.path_len -= 1;
            self.path[usize::from(self.path_len)] = 0;
            self.check_path();
        }
    }

    /// Sets the path to `m/purpose'/coin'/0'` for `script` on `network`,
    /// keeping the chain and index levels the text already had, so the
    /// Purpose group and the Chain pair stay the two independent choices
    /// §4.6 makes them.
    pub fn preset(&mut self, script: ScriptType, network: Network) {
        let mut tail = [0u8; MAX_PATH_BYTES];
        let tail_len = chain_tail(self.path_text(), &mut tail);
        self.path_len = 0;
        self.push_number(script.purpose());
        self.push_raw(b"h/");
        self.push_number(network.coin_type());
        self.push_raw(b"h/");
        self.push_number(PRESET_ACCOUNT);
        self.push_raw(b"h");
        self.push_raw(&tail[..tail_len]);
        self.check_path();
    }

    /// Whether the typed path is the account path of `script` on
    /// `network`: the check the Purpose group carries follows the text, so
    /// editing to a path no preset writes unchecks all four (§4.6).
    pub fn at_preset(&self, script: ScriptType, network: Network) -> bool {
        let Ok(path) = parse_path(self.path_text()) else {
            return false;
        };
        let want = [script.purpose(), network.coin_type(), PRESET_ACCOUNT];
        let levels = path.as_ref();
        levels.len() >= want.len()
            && levels
                .iter()
                .zip(want)
                .all(|(level, index)| *level == ChildNumber::Hardened { index })
    }

    /// Whether the typed path is on the change chain (§4.6's Chain pair).
    pub fn on_change_chain(&self) -> bool {
        chain_of(self.path_text()) == Some(true)
    }

    /// Sets the chain level to receive or change, padding the path out to
    /// one where it has none: the Chain pair's tap (§4.6).
    pub fn set_chain(&mut self, change: bool) {
        let mut buf = [0u8; MAX_PATH_BYTES];
        let len = with_chain(self.path_text(), change, &mut buf);
        self.path_len = 0;
        self.push_raw(&buf[..len]);
        self.check_path();
    }

    fn push_raw(&mut self, bytes: &[u8]) {
        for &b in bytes {
            let n = usize::from(self.path_len);
            if n >= MAX_PATH_BYTES {
                return;
            }
            self.path[n] = b;
            self.path_len += 1;
        }
    }

    fn push_number(&mut self, mut v: u32) {
        let mut digits = [0u8; 10];
        let mut n = 0;
        loop {
            digits[n] = b'0' + (v % 10) as u8;
            n += 1;
            v /= 10;
            if v == 0 {
                break;
            }
        }
        digits[..n].reverse();
        self.push_raw(&digits[..n]);
    }

    /// The reason the text does not parse, after every keystroke. The
    /// applied path does not move: ✓ is what applies it.
    fn check_path(&mut self) {
        self.path_error = parse_path(self.path_text()).err();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn abandon() -> [u16; 12] {
        let mut w = [0u16; 12];
        w[11] = 3;
        w
    }

    #[test]
    fn paths_parse_and_reject() {
        let p = parse_path("84h/0h/0h/0/0").unwrap();
        assert_eq!(p.len(), 5);
        assert_eq!(implied_script(&p), Some(ScriptType::NativeSegwit));
        assert_eq!(parse_path("").unwrap().len(), 0);
        assert_eq!(implied_script(&parse_path("").unwrap()), None);
        assert_eq!(implied_script(&parse_path("84/0").unwrap()), None);
        assert_eq!(implied_script(&parse_path("45h").unwrap()), None);
        assert_eq!(parse_path("/0"), Err(PathError::EmptyLevel));
        assert_eq!(parse_path("84h//0"), Err(PathError::EmptyLevel));
        assert_eq!(parse_path("84h/"), Err(PathError::Unfinished));
        assert_eq!(parse_path("2147483648"), Err(PathError::IndexTooLarge));
        assert_eq!(parse_path("2147483647h").unwrap().len(), 1);
        assert_eq!(parse_path("hh"), Err(PathError::NotANumber));
        assert_eq!(parse_path("h"), Err(PathError::NotANumber));
        assert_eq!(parse_path("0/0/0/0/0/0/0/0/0/0/0"), Err(PathError::TooDeep));
    }

    #[test]
    fn typing_leaves_the_applied_path_alone_until_the_tick() {
        let mut e = Explore::new(Network::Mainnet);
        assert_eq!(e.path_text(), "84h/0h/0h/0/0");
        assert_eq!(e.applied_path().len(), 5);
        e.go(Step::Path);
        assert!(e.path_push('/'));
        assert_eq!(e.path_error(), Some(PathError::Unfinished));
        assert!(!e.apply(), "the tick is dead while the text does not parse");
        assert_eq!(e.step(), Step::Path, "and the editor stays open");
        assert!(e.path_push('/'));
        assert_eq!(e.path_error(), Some(PathError::EmptyLevel));
        e.path_pop();
        e.path_pop();
        assert_eq!(e.path_error(), None);
        assert!(!e.path_push('x'));
        // §4.6: the Purpose group and the Chain pair are two
        // independent choices, so a preset rewrites the account levels
        // and keeps the chain and index the text already had.
        e.preset(ScriptType::Taproot, Network::Testnet);
        assert_eq!(e.path_text(), "86h/1h/0h/0/0");
        assert_eq!(
            e.applied_path().len(),
            5,
            "a preset types the text; it does not apply it"
        );
        assert!(!e.on_change_chain());
        e.set_chain(true);
        assert!(e.on_change_chain());
        assert_eq!(e.path_text(), "86h/1h/0h/1/0");
        e.set_chain(false);
        assert_eq!(e.path_text(), "86h/1h/0h/0/0");
        assert!(e.at_preset(ScriptType::Taproot, Network::Testnet));
        assert!(!e.at_preset(ScriptType::NativeSegwit, Network::Testnet));
        assert!(e.apply());
        assert_eq!(e.applied_path().len(), 5);
        assert_eq!(e.step(), Step::Keys, "the tick returns to the key menu");
    }

    #[test]
    fn typed_words_build_a_key_and_zeroize() {
        let mut e = Explore::new(Network::Mainnet);
        assert_eq!(e.source(), Source::None);
        assert!(!e.set_typed(&[0; 12], Language::English, Network::Mainnet));
        assert!(e.set_typed(&abandon(), Language::English, Network::Mainnet));
        assert_eq!(e.source(), Source::Typed);
        assert!(e.has_input());
        assert_eq!(
            e.typed_master().unwrap().fingerprint().to_hex(),
            *b"73c5da0a"
        );
        assert!(e.passphrase_push('T', 100, Network::Mainnet));
        assert_eq!(e.passphrase_visible_char(200), None, "not on that screen");
        e.go(Step::Passphrase);
        assert_eq!(e.passphrase_visible_char(200), Some('T'));
        assert_eq!(e.passphrase_visible_char(700), None);
        assert_ne!(
            e.typed_master().unwrap().fingerprint().to_hex(),
            *b"73c5da0a"
        );
        e.passphrase_pop(Network::Mainnet);
        assert_eq!(
            e.typed_master().unwrap().fingerprint().to_hex(),
            *b"73c5da0a"
        );
        e.zeroize();
        assert!(!e.has_input());
        assert!(e.typed_master().is_none());
        assert_eq!(e.source(), Source::None);
        assert!(e.words.iter().all(|w| *w == 0));
    }

    /// Every screen but the two menus returns to the menu whose row
    /// opened it; the key menu asks before it drops typed words.
    #[test]
    fn back_walks_out_one_menu_at_a_time() {
        let mut e = Explore::new(Network::Mainnet);
        for (step, menu) in [
            (Step::Chooser, Step::Keys),
            (Step::Path, Step::Keys),
            (Step::Passphrase, Step::Keys),
            (Step::Secret(SecretKind::Xprv), Step::Keys),
            (Step::Words, Step::Bits),
            (Step::Secret(SecretKind::Seed), Step::Bits),
            (Step::Secret(SecretKind::Entropy), Step::Bits),
            (Step::Secret(SecretKind::ChecksumBits), Step::Bits),
            (Step::Secret(SecretKind::MasterXprv), Step::Bits),
        ] {
            e.go(step);
            assert!(e.back(), "{step:?} has a screen to return to");
            assert_eq!(e.step(), menu);
        }
        e.go(Step::Bits);
        assert!(e.back());
        assert_eq!(e.step(), Step::Keys);
        assert!(!e.back(), "the key menu is the way out of the area");

        // Typed words are not dropped silently.
        assert!(e.set_typed(&abandon(), Language::English, Network::Mainnet));
        assert!(e.back());
        assert_eq!(e.step(), Step::Discard);
        assert!(!e.back(), "keeping them leaves the area on the next step");
        assert_eq!(e.step(), Step::Keys);
    }

    /// §2.7: the chooser checks a row and Continue applies it.
    #[test]
    fn the_chooser_applies_on_continue() {
        let mut e = Explore::new(Network::Mainnet);
        e.use_loaded(0);
        e.go(Step::Chooser);
        e.pick(Source::Loaded(2));
        assert_eq!(e.picked(), Source::Loaded(2));
        assert_eq!(e.source(), Source::Loaded(0), "not until Continue");
        e.confirm_pick();
        assert_eq!(e.source(), Source::Loaded(2));
        assert_eq!(e.step(), Step::Keys);
        // A key below the one being explored moves it down.
        e.key_removed(0);
        assert_eq!(e.source(), Source::Loaded(1));
        e.key_removed(1);
        assert_eq!(e.source(), Source::None);
    }
}
