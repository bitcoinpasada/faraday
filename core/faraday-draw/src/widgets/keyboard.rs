//! On-screen keyboard geometry (`docs/PLANNING.md` §4.5).
//!
//! One function, [`keys`], produces the key rectangles for a keyboard kind
//! inside a rectangle; drawing and hit-testing both use it, so they cannot
//! disagree. Key height comes from the height the screen offers: a letter
//! key takes about a twelfth of it, between 36 and 56 dp, except on
//! `Small`, where the panel is short and the key stays 40 dp. The PIN,
//! dice and coin pads are capped at [`PAD_MAX_WIDTH`] and take their
//! height from their width, so their cells stay near square. Key width is
//! whatever the columns divide the width into (about 27 dp of pitch on a
//! 2.8" panel).
//!
//! Anti-observation defaults: no popup previews, no press highlight that
//! persists after release. A key is drawn identically before, during and
//! after a press.

use alloc::vec::Vec;

use crate::geom::{Rect, SizeClass};
use crate::layout::LayoutCtx;
use crate::tokens;

/// Which keyboard.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyboardKind {
    /// QWERTY letters only, for BIP-39 words. Keys that cannot lead to a
    /// valid word are disabled; the enabled set comes from the application.
    Bip39,
    /// Digits 0–9, optionally scrambled.
    Pin,
    /// Hex digits 0–9 A–F.
    Hex,
    /// Printable ASCII with shift and a symbol layer.
    Passphrase,
    /// The dice pad: six large keys `1`–`6` in two rows and a backspace
    /// spanning both rows (UX.md §6 "dice pad").
    Dice,
    /// The coin pad: `Heads`, `Tails` and a backspace in one row of tall
    /// keys. Heads is [`KeyInput::Char`]`('H')`, tails `'T'`.
    Coin,
    /// The binary pad: `0`, `1`, a backspace and ✓ in one row of tall
    /// keys, for typing a value bit by bit.
    Binary,
    /// The card pad: thirteen rank keys and four suit keys in three
    /// rows, with a backspace and ✓. A card is typed rank-then-suit or
    /// suit-then-rank; the pad does not care which comes first.
    Cards,
    /// The derivation-path keyboard: digits, `/`, `h` (hardened),
    /// backspace and ✓, in three rows (UX.md H4).
    Path,
    /// The address keyboard (`docs/DESIGN.md` §4.3): the bech32
    /// alphabet on the QWERTY positions, and a shift that swaps it for
    /// base58 so that a legacy or nested address can be typed. Keys
    /// that cannot continue an address are disabled; the enabled set
    /// comes from the application.
    Address,
    /// The codex32 keyboard (`docs/DESIGN.md` §4.3): the same bech32
    /// alphabet on the same QWERTY positions as [`KeyboardKind::Address`],
    /// without the `1` and `b` an address needs for its human-readable
    /// part and without the shift that reaches base58, because a codex32
    /// string is bech32 characters and nothing else. Keys that cannot
    /// continue a string are disabled; the enabled set comes from the
    /// application.
    Codex32,
    /// The kana keyboard, for the Japanese wordlist: the 五十音 as ten
    /// columns of five, with the voicing marks, the small-kana key,
    /// backspace in the cells the grid leaves empty.
    Kana,
    /// The jamo keyboard, for the Korean wordlist: the two-set layout on
    /// the QWERTY positions, with shift for the doubled consonants and
    /// the two shifted vowels.
    Jamo,
    /// The pinyin keyboard, for the Simplified Chinese wordlist: the
    /// BIP-39 letter rows and a fourth row of the five tone keys, the
    /// backspace ending it.
    Pinyin,
    /// The 注音 keyboard, for the Traditional Chinese wordlist: the 大千
    /// layout, four rows, the backspace at the end of the last.
    Zhuyin,
}

/// What a key does when tapped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyInput {
    /// Insert a character.
    Char(char),
    /// Delete the previous character.
    Backspace,
    /// Confirm / done.
    Done,
    /// Toggle shift (passphrase keyboard).
    Shift,
    /// Toggle the symbol layer (passphrase keyboard).
    Symbols,
}

/// Modifier state the keyboard is drawn with (kept in [`crate::UiState`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Modifiers {
    /// Shift is on (letters upper-case).
    pub shift: bool,
    /// The symbol layer is showing.
    pub symbols: bool,
}

/// One key's geometry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyCap {
    /// Where the key is.
    pub rect: Rect,
    /// What it does.
    pub input: KeyInput,
    /// Whether it accepts taps.
    pub enabled: bool,
}

const BIP39_ROWS: [&str; 3] = ["qwertyuiop", "asdfghjkl", "zxcvbnm"];
const PASS_ROWS: [&str; 4] = ["1234567890", "qwertyuiop", "asdfghjkl", "zxcvbnm"];
const SYMBOL_ROWS: [&str; 4] = ["!@#$%^&*()", "-_=+[]{}\\|", ";:'\",.<>/?", "`~"];
const HEX_ROWS: [&str; 3] = ["123456", "7890AB", "CDEF"];
const PATH_ROWS: [&str; 3] = ["12345", "67890", "/h"];

/// The address keyboard's bech32 layer. The alphabet is
/// `qpzry9x8gf2tvdw0s3jn54khce6mua7l`, which is every lower-case letter
/// but `b`, `i` and `o` and every digit but `1`; the rows put each
/// where QWERTY does, so the letters a person knows are where they
/// expect them. `1` and `b` have keys of their own because an address
/// begins with a human-readable part and the separator — `bc1`, `tb1`,
/// `bcrt1` — and neither character is in the alphabet that follows it.
const ADDRESS_ROWS: [&str; 4] = ["0123456789", "qwertyup", "asdfghjkl", "zxcvbnm"];

/// The codex32 keyboard's rows: the bech32 alphabet on the same
/// positions the address keyboard puts it, less `1` and `b`, which are
/// an address's human-readable part and no part of the alphabet.
const CODEX32_ROWS: [&str; 4] = ["023456789", "qwertyup", "asdfghjkl", "zxcvnm"];

/// The address keyboard's base58 layer, lower case: the alphabet
/// without `0`, `O`, `I` and `l`, which is what a legacy or nested
/// address is written in.
const BASE58_LOWER_ROWS: [&str; 4] = ["123456789", "qwertyuiop", "asdfghjk", "zxcvbnm"];

/// The same layer in upper case.
const BASE58_UPPER_ROWS: [&str; 4] = ["123456789", "QWERTYUP", "ASDFGHJKL", "ZXCVBNM"];

/// The rank keys, in two rows of seven and six. `T` is the ten.
const CARD_RANK_ROWS: [&str; 2] = ["A234567", "89TJQK"];

/// The suit keys: ♠ ♥ ♦ ♣.
pub const CARD_SUITS: [char; 4] = ['\u{2660}', '\u{2665}', '\u{2666}', '\u{2663}'];

/// The standalone voiced sound mark ゛, which the dakuten key shows.
pub const DAKUTEN_KEY: char = '\u{309B}';
/// The standalone semi-voiced sound mark ゜, which the handakuten key shows.
pub const HANDAKUTEN_KEY: char = '\u{309C}';
/// 小, the key that turns the last kana typed into its small form.
pub const SMALL_KEY: char = '\u{5C0F}';

/// One kana cell, so that the grid below reads as the 五十音 it is.
const fn k(c: char) -> Option<KeyInput> {
    Some(KeyInput::Char(c))
}

/// The kana keyboard, cell by cell: five rows of the vowels あいうえお,
/// ten columns of the consonant groups あかさたなはまやらわ. The 五十音
/// leaves five cells empty — や at い and え, わ at い, う and え — and
/// they carry ゛, ゜ and 小; ん takes the わ column's え cell and the
/// backspace the bottom-right one, where the archaic を would sit
/// (`docs/PLANNING.md` §16.118). Four keys for five cells leaves one
/// empty, and an empty cell is a cell: nothing is drawn there and a tap
/// on it does nothing.
const KANA_GRID: [[Option<KeyInput>; 10]; 5] = [
    [
        k('あ'),
        k('か'),
        k('さ'),
        k('た'),
        k('な'),
        k('は'),
        k('ま'),
        k('や'),
        k('ら'),
        k('わ'),
    ],
    [
        k('い'),
        k('き'),
        k('し'),
        k('ち'),
        k('に'),
        k('ひ'),
        k('み'),
        k(DAKUTEN_KEY),
        k('り'),
        k(SMALL_KEY),
    ],
    [
        k('う'),
        k('く'),
        k('す'),
        k('つ'),
        k('ぬ'),
        k('ふ'),
        k('む'),
        k('ゆ'),
        k('る'),
        None,
    ],
    [
        k('え'),
        k('け'),
        k('せ'),
        k('て'),
        k('ね'),
        k('へ'),
        k('め'),
        k(HANDAKUTEN_KEY),
        k('れ'),
        k('ん'),
    ],
    [
        k('お'),
        k('こ'),
        k('そ'),
        k('と'),
        k('の'),
        k('ほ'),
        k('も'),
        k('よ'),
        k('ろ'),
        Some(KeyInput::Backspace),
    ],
];

/// The two-set jamo rows on the QWERTY positions.
const JAMO_ROWS: [&str; 3] = [
    "ㅂㅈㄷㄱㅅㅛㅕㅑㅐㅔ",
    "ㅁㄴㅇㄹㅎㅗㅓㅏㅣ",
    "ㅋㅌㅊㅍㅠㅜㅡ",
];

/// The same rows with shift down: the five doubled consonants and the
/// two shifted vowels; everything else keeps its key.
const JAMO_SHIFT_ROWS: [&str; 3] = [
    "ㅃㅉㄸㄲㅆㅛㅕㅑㅒㅖ",
    "ㅁㄴㅇㄹㅎㅗㅓㅏㅣ",
    "ㅋㅌㅊㅍㅠㅜㅡ",
];

/// The tone keys of the pinyin keyboard: 1 to 4 as a dictionary marks
/// them, 5 for the neutral tone. A syllable is not a word until one of
/// them is pressed.
const PINYIN_TONES: &str = "12345";

/// ˉ, the first tone. The 大千 layout leaves the first tone to the space
/// bar, which this keyboard has no room for, so it is a key of its own.
pub const TONE_HIGH: char = '\u{02C9}';
/// ˊ, the second tone.
pub const TONE_RISING: char = '\u{02CA}';
/// ˇ, the third tone.
pub const TONE_LOW: char = '\u{02C7}';
/// ˋ, the fourth tone.
pub const TONE_FALLING: char = '\u{02CB}';
/// ˙, the neutral tone.
pub const TONE_NEUTRAL: char = '\u{02D9}';

/// The 注音 tone marks, in tone order from the first. Every tone has a
/// key, so every reading ends the same way.
pub const ZHUYIN_MARKS: [char; 5] = [TONE_HIGH, TONE_RISING, TONE_LOW, TONE_FALLING, TONE_NEUTRAL];

/// The 大千 layout, four rows on the QWERTY positions, with ㄦ at the end
/// of the top row where the layout puts it and ˉ after it on the second
/// row, where the layout has the space bar this keyboard lacks. Eleven
/// keys in the first two rows and ten in the others: the rows are the
/// ones people have on a Taiwanese keyboard, so the key width follows
/// the row.
const ZHUYIN_ROWS: [&str; 4] = [
    "\u{3105}\u{3109}\u{02C7}\u{02CB}\u{3113}\u{02CA}\u{02D9}\u{311A}\u{311E}\u{3122}\u{3126}",
    "\u{3106}\u{310A}\u{310D}\u{3110}\u{3114}\u{3117}\u{3127}\u{311B}\u{311F}\u{3123}\u{02C9}",
    "\u{3107}\u{310B}\u{310E}\u{3111}\u{3115}\u{3118}\u{3128}\u{311C}\u{3120}\u{3124}",
    "\u{3108}\u{310C}\u{310F}\u{3112}\u{3116}\u{3119}\u{3129}\u{311D}\u{3121}\u{3125}",
];

/// The seven keys shift swaps in, which have no unshifted position of
/// their own and so are numbered after the rows.
const JAMO_SHIFTED: [char; 7] = ['ㅃ', 'ㅉ', 'ㄸ', 'ㄲ', 'ㅆ', 'ㅒ', 'ㅖ'];

/// Bitmask of the keys a keyboard still offers: bit `n` is the `n`-th
/// key of that kind ([`key_bit`]), and bit 63 is [`DONE_DISABLED`].
/// [`ALL_KEYS`] enables everything.
pub type KeyMask = u64;

/// Every key enabled.
pub const ALL_KEYS: KeyMask = !(DONE_DISABLED | DONE_ABSENT);

/// Mask bit that disables the ✓ key of the dice, coin, hex, PIN and
/// path pads, where ✓ means "continue" and is live only once what has
/// been entered is usable. The other kinds ignore it, and the five that
/// type a wordlist have no ✓ at all (`docs/PLANNING.md` §16.118).
pub const DONE_DISABLED: KeyMask = 1 << 63;

/// Mask bit that leaves the digit pad's ✓ cell empty: no key drawn and
/// no target, the digits and the backspace where the PIN pad has them.
/// Word numbers' pad carries it, because a word is taken by a tap on its
/// candidate and the pad has nothing to confirm (`docs/PLANNING.md`
/// §16.132 rule 1). The other kinds ignore it; no kind numbers a key at
/// this bit.
pub const DONE_ABSENT: KeyMask = 1 << 62;

/// The mask bit of a key on `kind`'s keyboard, or 0 for a character
/// that has no key there. The BIP-39 keyboard numbers its letters `a` to
/// `z`; the kana and jamo keyboards number their keys in layout order.
pub fn key_bit(kind: KeyboardKind, c: char) -> KeyMask {
    let index = match kind {
        KeyboardKind::Kana => KANA_GRID
            .iter()
            .flatten()
            .position(|cell| *cell == Some(KeyInput::Char(c))),
        KeyboardKind::Jamo => JAMO_ROWS
            .iter()
            .flat_map(|r| r.chars())
            .chain(JAMO_SHIFTED)
            .position(|k| k == c),
        KeyboardKind::Zhuyin => ZHUYIN_ROWS
            .iter()
            .flat_map(|r| r.chars())
            .position(|k| k == c),
        // The letters keep the BIP-39 numbering, so a pinyin syllable
        // and a Latin word light the same bits; the five tone keys
        // follow them.
        KeyboardKind::Pinyin => {
            return if c.is_ascii_lowercase() {
                1 << (c as u32 - 'a' as u32)
            } else if let Some(i) = PINYIN_TONES.find(c) {
                1 << (26 + i)
            } else {
                0
            };
        }
        // The digit pad numbers its keys by the digit itself, so a mask
        // can dim the digits that cannot lead where the screen is going
        // (`docs/PLANNING.md` §16.125 rule 2).
        KeyboardKind::Pin => {
            return if c.is_ascii_digit() {
                1 << (c as u32 - '0' as u32)
            } else {
                0
            };
        }
        // The address keyboard carries both cases and the digits, so
        // its bits run lower case, digits, upper case: 62 of the 63 the
        // mask has under [`DONE_DISABLED`]. The codex32 keyboard is a
        // subset of the same keys and keeps the same numbering, so one
        // mask reads on either.
        KeyboardKind::Address | KeyboardKind::Codex32 => {
            return if c.is_ascii_lowercase() {
                1 << (c as u32 - 'a' as u32)
            } else if c.is_ascii_digit() {
                1 << (26 + (c as u32 - '0' as u32))
            } else if c.is_ascii_uppercase() {
                1 << (36 + (c as u32 - 'A' as u32))
            } else {
                0
            };
        }
        _ => {
            return if c.is_ascii_lowercase() {
                1 << (c as u32 - 'a' as u32)
            } else {
                0
            };
        }
    };
    index.map_or(0, |i| 1 << i)
}

/// The mask bit for a letter of the BIP-39 keyboard, or 0 for a
/// non-letter.
pub fn letter_bit(c: char) -> KeyMask {
    key_bit(KeyboardKind::Bip39, c)
}

/// Every character `kind` has a key for, in layout order. The font baker
/// reads this, so a keycap can never be drawn that was not baked.
pub fn keycaps(kind: KeyboardKind) -> impl Iterator<Item = char> {
    let kana = (kind == KeyboardKind::Kana)
        .then(|| KANA_GRID.iter().flatten())
        .into_iter()
        .flatten()
        .filter_map(|cell| match cell {
            Some(KeyInput::Char(c)) => Some(*c),
            _ => None,
        });
    let jamo = (kind == KeyboardKind::Jamo)
        .then(|| JAMO_ROWS.iter().flat_map(|r| r.chars()).chain(JAMO_SHIFTED))
        .into_iter()
        .flatten();
    let pinyin = (kind == KeyboardKind::Pinyin)
        .then(|| {
            BIP39_ROWS
                .iter()
                .flat_map(|r| r.chars())
                .chain(PINYIN_TONES.chars())
        })
        .into_iter()
        .flatten();
    let zhuyin = (kind == KeyboardKind::Zhuyin)
        .then(|| ZHUYIN_ROWS.iter().flat_map(|r| r.chars()))
        .into_iter()
        .flatten();
    let cards = (kind == KeyboardKind::Cards)
        .then(|| {
            CARD_RANK_ROWS
                .iter()
                .flat_map(|r| r.chars())
                .chain(CARD_SUITS)
        })
        .into_iter()
        .flatten();
    kana.chain(jamo).chain(pinyin).chain(zhuyin).chain(cards)
}

/// Builds a mask from the letters in `s`.
pub fn mask_of(s: &str) -> KeyMask {
    s.chars()
        .fold(0, |m, c| m | letter_bit(c.to_ascii_lowercase()))
}

/// Number of rows a keyboard kind has (for height measurement).
pub fn rows(kind: KeyboardKind) -> usize {
    match kind {
        KeyboardKind::Bip39 => 3,
        KeyboardKind::Pin => 4,
        KeyboardKind::Hex => 3,
        KeyboardKind::Passphrase => 5,
        KeyboardKind::Dice => 2,
        KeyboardKind::Coin | KeyboardKind::Binary => 1,
        KeyboardKind::Path => 3,
        KeyboardKind::Address | KeyboardKind::Codex32 => 4,
        KeyboardKind::Cards => 3,
        KeyboardKind::Kana => 5,
        KeyboardKind::Jamo => 3,
        KeyboardKind::Pinyin | KeyboardKind::Zhuyin => 4,
    }
}

/// Preferred key height in dp for a size class and kind, before the
/// offered space is taken into account. This is what `Small` keeps: the
/// 2.8" panel is short, and 40 dp letter keys are right for it.
pub fn preferred_key_height(kind: KeyboardKind, class: SizeClass) -> f32 {
    match (kind, class) {
        (KeyboardKind::Pin, SizeClass::Small) => tokens::PIN_KEY_HEIGHT_SMALL,
        // The dice and coin pads are the screen's whole lower half, as
        // on the reference panel: six large faces, or two large sides.
        (KeyboardKind::Dice, SizeClass::Small) => tokens::DICE_KEY_HEIGHT_SMALL,
        (KeyboardKind::Coin | KeyboardKind::Binary, SizeClass::Small) => {
            tokens::COIN_KEY_HEIGHT_SMALL
        }
        // The 五十音 is five rows of ten, and a short panel has the
        // height for that only at a shorter key. The card pad is three
        // rows of seven under a field and its caption line, which a
        // short panel has the height for at the same key. The pinyin and
        // 注音 keyboards are four rows over the two-row one-character
        // strip, which is the same height problem, so they take the same
        // key (docs/PLANNING.md §16.121).
        (
            KeyboardKind::Kana | KeyboardKind::Cards | KeyboardKind::Pinyin | KeyboardKind::Zhuyin,
            SizeClass::Small,
        ) => tokens::KEY_HEIGHT_GRID,
        (KeyboardKind::Pin | KeyboardKind::Dice, _) => tokens::PAD_KEY_HEIGHT,
        (KeyboardKind::Coin | KeyboardKind::Binary, _) => tokens::COIN_KEY_HEIGHT,
        // A letter key: the class's own height on `Small`, which is
        // what the short panel keeps, and the floor elsewhere, from
        // which `key_height` grows it towards `tokens::KEY_MAX_HEIGHT`.
        (_, SizeClass::Small) => tokens::key_height(SizeClass::Small),
        _ => tokens::touch_floor(SizeClass::Small),
    }
}

/// Smallest acceptable key height in dp ([`tokens::KEY_MIN_HEIGHT`]).
pub const MIN_KEY_HEIGHT: f32 = tokens::KEY_MIN_HEIGHT;

/// Smallest key height in dp for `kind` on `class`: the 五十音's five
/// rows, and the reading keyboards' four over a two-row strip, squeeze
/// further on a short panel than a three-row keyboard does, so that the
/// field and its candidates keep their places above them.
pub fn min_key_height(kind: KeyboardKind, class: SizeClass) -> f32 {
    match (kind, class) {
        (
            KeyboardKind::Kana | KeyboardKind::Cards | KeyboardKind::Pinyin | KeyboardKind::Zhuyin,
            SizeClass::Small,
        ) => tokens::KEY_MIN_HEIGHT_GRID,
        // A one-row pad of two large keys has no rows to give up: its
        // least height is the height it is drawn at, or a screen that
        // squeezes its keyboard would leave a 28 dp strip where the pad
        // should be.
        (KeyboardKind::Coin | KeyboardKind::Binary, _) => preferred_key_height(kind, class),
        _ => MIN_KEY_HEIGHT,
    }
}

/// Largest letter-key height in dp ([`tokens::KEY_MAX_HEIGHT`]).
pub const MAX_KEY_HEIGHT: f32 = tokens::KEY_MAX_HEIGHT;

/// Share of the height a screen offers that one letter-key row takes
/// ([`tokens::KEY_HEIGHT_FRACTION`]). Three rows plus the reserve come to
/// about a quarter of a phone screen, which is what a system keyboard
/// uses.
pub const KEY_HEIGHT_FRACTION: f32 = tokens::KEY_HEIGHT_FRACTION;

/// Largest width in dp a PIN, dice or coin pad takes
/// ([`tokens::PAD_MAX_WIDTH`]).
pub const PAD_MAX_WIDTH: f32 = tokens::PAD_MAX_WIDTH;

/// Largest key height in dp for a pad
/// ([`tokens::PAD_MAX_KEY_HEIGHT`]).
pub const MAX_PAD_KEY_HEIGHT: f32 = tokens::PAD_MAX_KEY_HEIGHT;

/// Widest a PIN cell may be against its height
/// ([`tokens::PAD_KEY_ASPECT`]).
const PIN_ASPECT: f32 = tokens::PAD_KEY_ASPECT;

/// Columns a pad divides its width into.
fn pad_columns(kind: KeyboardKind) -> f32 {
    match kind {
        KeyboardKind::Pin => tokens::PIN_PAD_COLUMNS,
        _ => tokens::PAD_COLUMNS,
    }
}

/// Whether `kind` is a pad whose height follows from its width.
fn is_pad(kind: KeyboardKind) -> bool {
    matches!(kind, KeyboardKind::Pin | KeyboardKind::Dice)
}

/// Largest key height in dp `kind` may take when the keyboard is
/// `width_dp` wide. This is the ceiling [`height`] applies to whatever
/// height it is offered; [`key_height`] is what a screen asks for.
pub fn max_key_height(kind: KeyboardKind, class: SizeClass, width_dp: f32) -> f32 {
    if class == SizeClass::Small {
        return preferred_key_height(kind, class);
    }
    if is_pad(kind) {
        let cell = width_dp.min(PAD_MAX_WIDTH) / pad_columns(kind);
        let aspect = if kind == KeyboardKind::Pin {
            PIN_ASPECT
        } else {
            1.0
        };
        return (cell / aspect).clamp(MIN_KEY_HEIGHT, MAX_PAD_KEY_HEIGHT);
    }
    if matches!(kind, KeyboardKind::Coin | KeyboardKind::Binary) {
        return preferred_key_height(kind, class);
    }
    MAX_KEY_HEIGHT
}

/// The key height in dp a screen should ask for: a share of the height
/// it has under the app bar, clamped to the class's range.
pub fn key_height(kind: KeyboardKind, class: SizeClass, width_dp: f32, offered_h_dp: f32) -> f32 {
    let ceiling = max_key_height(kind, class, width_dp);
    if class == SizeClass::Small
        || is_pad(kind)
        || matches!(kind, KeyboardKind::Coin | KeyboardKind::Binary)
    {
        return ceiling;
    }
    (offered_h_dp * KEY_HEIGHT_FRACTION).clamp(MIN_KEY_HEIGHT, ceiling)
}

/// The least and greatest height in dp the node that holds `kind` should
/// be given, [`BOTTOM_RESERVE`] included.
///
/// [`height`] takes the reserve off the top of whatever it is offered,
/// so a cap of `rows × key height` alone leaves every key
/// `BOTTOM_RESERVE / rows` dp short. Every screen asks for its bounds
/// here so that arithmetic lives in one place.
pub fn node_heights(
    kind: KeyboardKind,
    class: SizeClass,
    inset_bottom_dp: f32,
    width_dp: f32,
    offered_h_dp: f32,
    min_key_dp: f32,
) -> (f32, f32) {
    let n = rows(kind) as f32;
    let max_key = key_height(kind, class, width_dp, offered_h_dp).max(min_key_dp);
    let reserve = tokens::key_bottom_reserve(inset_bottom_dp);
    (n * min_key_dp + reserve, n * max_key + reserve)
}

/// Gap between key cells in dp. Zero: keys tile the keyboard area with no
/// dead zones, and each one paints a face inset by [`KEY_FACE_INSET`], so
/// the visible gaps between keys are still tappable (UX.md §6).
pub const KEY_GAP: f32 = tokens::KEY_GAP;

/// Inset in dp from a key's cell to its visible face, left and right.
pub const KEY_FACE_INSET: f32 = tokens::KEY_FACE_INSET;

/// Inset in dp from a key's cell to its visible face, top and bottom.
/// Smaller than the horizontal one: height is the scarce axis, and a
/// key that reads tall enough is worth more than an even border.
pub const KEY_FACE_INSET_Y: f32 = tokens::KEY_FACE_INSET_Y;

/// Inset in dp under the bottom row's faces: the visible keys sit this
/// far above the bottom edge of the screen, while their hit rectangles
/// reach it (UX.md §6).
pub const KEY_BOTTOM_INSET: f32 = tokens::KEY_BOTTOM_INSET;

/// Height in dp the keyboard region takes on top of its rows, so that
/// the bottom row keeps a full-height face and still reaches the screen
/// edge with its hit rectangle.
pub const BOTTOM_RESERVE: f32 = tokens::KEY_BOTTOM_RESERVE;

/// Intrinsic height in pixels for `kind` when offered `max` pixels. The
/// ceiling comes from the offered width (see [`max_key_height`]); the
/// screen's own cap, from [`node_heights`], decides the rest.
pub fn height(kind: KeyboardKind, ctx: &LayoutCtx, max: crate::geom::Size) -> i32 {
    let n = rows(kind) as i32;
    let reserve = ctx.px(tokens::key_bottom_reserve(ctx.inset_bottom_dp));
    let width_dp = max.w as f32 / ctx.scale.factor();
    let ceiling = ctx.px(max_key_height(kind, ctx.class, width_dp));
    let min = ctx.px(MIN_KEY_HEIGHT);
    let fits = ((max.h - reserve) / n).max(0);
    let key_h = fits.min(ceiling).max(min);
    key_h * n + reserve
}

/// Deterministic permutation of the digits 0–9 for `seed` (xorshift32 +
/// Fisher–Yates). The same seed always gives the same order.
pub fn scrambled_digits(seed: u32) -> [char; 10] {
    let mut digits = ['0', '1', '2', '3', '4', '5', '6', '7', '8', '9'];
    let mut x = seed | 1;
    for i in (1..10).rev() {
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        let j = (x % (i as u32 + 1)) as usize;
        digits.swap(i, j);
    }
    digits
}

/// The key rectangles for `kind` filling `area`.
///
/// `enabled` is the BIP-39 letter mask (ignored by other kinds);
/// `scramble` is the PIN scramble seed (`None` = natural order).
pub fn keys(
    kind: KeyboardKind,
    area: Rect,
    ctx: &LayoutCtx,
    enabled: KeyMask,
    scramble: Option<u32>,
    mods: Modifiers,
) -> Vec<KeyCap> {
    let gap = ctx.px(KEY_GAP);
    let n_rows = rows(kind) as i32;
    // The rows tile the whole region: every row but the last is
    // `key_h` tall, and the last one takes the remainder, which carries
    // the bottom reserve down to the screen edge.
    let key_h =
        ((area.h - ctx.px(tokens::key_bottom_reserve(ctx.inset_bottom_dp))) / n_rows).max(1);
    let row_y = |r: i32| area.y + r * key_h;
    let row_h = |r: i32| {
        if r + 1 == n_rows {
            (area.bottom() - row_y(r)).max(1)
        } else {
            key_h
        }
    };
    let mut out = Vec::new();

    // A row of `units` total width units; each entry is (units, input,
    // enabled). Keys are laid out left to right with `gap` between.
    let lay_row = |r: i32, entries: &[(f32, KeyInput, bool)], out: &mut Vec<KeyCap>| {
        let (y, h) = (row_y(r), row_h(r));
        let total_units: f32 = entries.iter().map(|e| e.0).sum();
        let n = entries.len() as i32;
        let usable = (area.w - gap * (n - 1)) as f32;
        let unit = usable / total_units;
        let mut x = area.x as f32;
        for (i, &(units, input, en)) in entries.iter().enumerate() {
            let w = units * unit;
            let x0 = round(x);
            let x1 = if i as i32 == n - 1 {
                area.right()
            } else {
                round(x + w)
            };
            out.push(KeyCap {
                rect: Rect::new(x0, y, (x1 - x0).max(1), h),
                input,
                enabled: en,
            });
            x += w + gap as f32;
        }
    };

    match kind {
        KeyboardKind::Bip39 => {
            for (r, letters) in BIP39_ROWS.iter().enumerate() {
                let mut entries: Vec<(f32, KeyInput, bool)> = letters
                    .chars()
                    .map(|c| (1.0, KeyInput::Char(c), enabled & letter_bit(c) != 0))
                    .collect();
                if r == 2 {
                    // §16.118: a wordlist keyboard has no ✓ — a word is
                    // taken by a tap on its candidate — so row three is
                    // the seven letters and the backspace at the right.
                    entries.push((tokens::KEY_UNITS_EDGE, KeyInput::Backspace, true));
                }
                lay_row(r as i32, &entries, &mut out);
            }
        }
        KeyboardKind::Pin => {
            let digits = scramble.map_or(
                ['0', '1', '2', '3', '4', '5', '6', '7', '8', '9'],
                scrambled_digits,
            );
            let live = |c: char| enabled & key_bit(KeyboardKind::Pin, c) != 0;
            for r in 0..3 {
                let entries: Vec<_> = (0..3)
                    .map(|c| {
                        let d = digits[1 + r * 3 + c];
                        (1.0, KeyInput::Char(d), live(d))
                    })
                    .collect();
                lay_row(r as i32, &entries, &mut out);
            }
            let last = [
                (1.0, KeyInput::Backspace, true),
                (1.0, KeyInput::Char(digits[0]), live(digits[0])),
                (1.0, KeyInput::Done, enabled & DONE_DISABLED == 0),
            ];
            lay_row(3, &last, &mut out);
            // The row is laid out whole so the two keys keep their
            // columns, and then the ✓ cell is emptied.
            if enabled & DONE_ABSENT != 0 {
                out.pop();
            }
        }
        KeyboardKind::Hex => {
            for (r, chars) in HEX_ROWS.iter().enumerate() {
                let mut entries: Vec<(f32, KeyInput, bool)> = chars
                    .chars()
                    .map(|c| (1.0, KeyInput::Char(c), true))
                    .collect();
                if r == 2 {
                    entries.push((1.0, KeyInput::Backspace, true));
                    entries.push((1.0, KeyInput::Done, enabled & DONE_DISABLED == 0));
                }
                lay_row(r as i32, &entries, &mut out);
            }
        }
        KeyboardKind::Dice => {
            // Four columns: three dice keys per row and one backspace on
            // the right spanning both rows.
            let usable = (area.w - gap * 3) as f32;
            let unit = usable / tokens::PAD_COLUMNS;
            let col_x = |c: i32| area.x + round(c as f32 * (unit + gap as f32));
            for r in 0..2 {
                for c in 0..3 {
                    let digit = (b'1' + (r * 3 + c) as u8) as char;
                    let x0 = col_x(c);
                    let x1 = col_x(c + 1) - gap;
                    out.push(KeyCap {
                        rect: Rect::new(x0, row_y(r), (x1 - x0).max(1), row_h(r)),
                        input: KeyInput::Char(digit),
                        enabled: true,
                    });
                }
            }
            // The fourth column: backspace over ✓.
            let x0 = col_x(3);
            let w = (area.right() - x0).max(1);
            out.push(KeyCap {
                rect: Rect::new(x0, row_y(0), w, row_h(0)),
                input: KeyInput::Backspace,
                enabled: true,
            });
            out.push(KeyCap {
                rect: Rect::new(x0, row_y(1), w, row_h(1)),
                input: KeyInput::Done,
                enabled: enabled & DONE_DISABLED == 0,
            });
        }
        KeyboardKind::Coin => {
            let entries = [
                (2.0, KeyInput::Char('H'), true),
                (2.0, KeyInput::Char('T'), true),
                (1.0, KeyInput::Backspace, true),
                (1.0, KeyInput::Done, enabled & DONE_DISABLED == 0),
            ];
            lay_row(0, &entries, &mut out);
        }
        KeyboardKind::Binary => {
            let entries = [
                (2.0, KeyInput::Char('0'), true),
                (2.0, KeyInput::Char('1'), true),
                (1.0, KeyInput::Backspace, true),
                (1.0, KeyInput::Done, enabled & DONE_DISABLED == 0),
            ];
            lay_row(0, &entries, &mut out);
        }
        KeyboardKind::Cards => {
            // Seven columns: seven ranks, then six ranks and the
            // backspace, then the four suits and a ✓ three columns wide.
            for (r, chars) in CARD_RANK_ROWS.iter().enumerate() {
                let mut entries: Vec<(f32, KeyInput, bool)> = chars
                    .chars()
                    .map(|c| (1.0, KeyInput::Char(c), true))
                    .collect();
                if r == 1 {
                    entries.push((1.0, KeyInput::Backspace, true));
                }
                lay_row(r as i32, &entries, &mut out);
            }
            let mut entries: Vec<(f32, KeyInput, bool)> = CARD_SUITS
                .iter()
                .map(|&c| (1.0, KeyInput::Char(c), true))
                .collect();
            entries.push((
                tokens::KEY_UNITS_DONE,
                KeyInput::Done,
                enabled & DONE_DISABLED == 0,
            ));
            lay_row(2, &entries, &mut out);
        }
        KeyboardKind::Path => {
            // Five columns: two rows of digits, then `/`, `h` and the
            // wider backspace and ✓ sharing the same five units.
            for (r, chars) in PATH_ROWS.iter().enumerate() {
                let mut entries: Vec<(f32, KeyInput, bool)> = chars
                    .chars()
                    .map(|c| (1.0, KeyInput::Char(c), true))
                    .collect();
                if r == 2 {
                    entries.push((tokens::KEY_UNITS_EDGE, KeyInput::Backspace, true));
                    // Dead while the path does not parse: ✓ applies the
                    // text on the field (UX review 2026-09-07, §3.7).
                    entries.push((
                        tokens::KEY_UNITS_EDGE,
                        KeyInput::Done,
                        enabled & DONE_DISABLED == 0,
                    ));
                }
                lay_row(r as i32, &entries, &mut out);
            }
        }
        KeyboardKind::Address => {
            // Shift swaps the bech32 alphabet for base58, and on the
            // base58 layer the key in the digit row's tenth place
            // swaps the case. The bottom row is the same four places on
            // every layer, so shift, backspace and ✓ do not move when
            // the alphabet does.
            let rows_src: &[&str; 4] = match (mods.shift, mods.symbols) {
                (false, _) => &ADDRESS_ROWS,
                (true, false) => &BASE58_LOWER_ROWS,
                (true, true) => &BASE58_UPPER_ROWS,
            };
            for (r, chars) in rows_src.iter().enumerate() {
                let mut entries: Vec<(f32, KeyInput, bool)> = chars
                    .chars()
                    .map(|c| {
                        let input = KeyInput::Char(c);
                        (1.0, input, live(kind, input, enabled))
                    })
                    .collect();
                if r == 0 && mods.shift {
                    entries.push((1.0, KeyInput::Symbols, true));
                }
                if r == 3 {
                    entries.insert(0, (tokens::KEY_UNITS_EDGE, KeyInput::Shift, true));
                    entries.push((tokens::KEY_UNITS_EDGE, KeyInput::Backspace, true));
                    entries.push((
                        tokens::KEY_UNITS_EDGE,
                        KeyInput::Done,
                        enabled & DONE_DISABLED == 0,
                    ));
                }
                lay_row(r as i32, &entries, &mut out);
            }
        }
        KeyboardKind::Codex32 => {
            // One layer: no shift to another alphabet, so the bottom row
            // is the letters it has, then backspace and ✓.
            for (r, chars) in CODEX32_ROWS.iter().enumerate() {
                let mut entries: Vec<(f32, KeyInput, bool)> = chars
                    .chars()
                    .map(|c| {
                        let input = KeyInput::Char(c);
                        (1.0, input, live(kind, input, enabled))
                    })
                    .collect();
                if r == 3 {
                    entries.push((tokens::KEY_UNITS_EDGE, KeyInput::Backspace, true));
                    entries.push((
                        tokens::KEY_UNITS_EDGE,
                        KeyInput::Done,
                        enabled & DONE_DISABLED == 0,
                    ));
                }
                lay_row(r as i32, &entries, &mut out);
            }
        }
        KeyboardKind::Kana => {
            // The grid is laid cell by cell rather than key by key, so
            // that the columns line up down the five rows whether a
            // cell carries a key or not.
            let cols = tokens::KANA_COLUMNS;
            let cell_w = (area.w - gap * (cols as i32 - 1)) as f32 / cols as f32;
            for (r, row) in KANA_GRID.iter().enumerate() {
                let (y, h) = (row_y(r as i32), row_h(r as i32));
                for (c, cell) in row.iter().enumerate() {
                    let Some(input) = *cell else { continue };
                    let x0 = area.x + round(c as f32 * (cell_w + gap as f32));
                    let x1 = if c + 1 == cols {
                        area.right()
                    } else {
                        area.x + round(c as f32 * (cell_w + gap as f32) + cell_w)
                    };
                    out.push(KeyCap {
                        rect: Rect::new(x0, y, (x1 - x0).max(1), h),
                        input,
                        enabled: live(kind, input, enabled),
                    });
                }
            }
        }
        KeyboardKind::Jamo => {
            let rows_src = if mods.shift {
                &JAMO_SHIFT_ROWS
            } else {
                &JAMO_ROWS
            };
            for (r, chars) in rows_src.iter().enumerate() {
                let mut entries: Vec<(f32, KeyInput, bool)> = chars
                    .chars()
                    .map(|c| {
                        let input = KeyInput::Char(c);
                        (1.0, input, live(kind, input, enabled))
                    })
                    .collect();
                if r == 2 {
                    entries.insert(0, (tokens::KEY_UNITS_EDGE, KeyInput::Shift, true));
                    entries.push((tokens::KEY_UNITS_EDGE, KeyInput::Backspace, true));
                }
                lay_row(r as i32, &entries, &mut out);
            }
        }
        KeyboardKind::Pinyin => {
            // The letter rows keep the shape they have on the BIP-39
            // keyboard; the tones are a fourth row of their own, ending
            // with the backspace that row 3 carries there.
            for (r, letters) in BIP39_ROWS.iter().enumerate() {
                let entries: Vec<(f32, KeyInput, bool)> = letters
                    .chars()
                    .map(|c| {
                        let input = KeyInput::Char(c);
                        (1.0, input, live(kind, input, enabled))
                    })
                    .collect();
                lay_row(r as i32, &entries, &mut out);
            }
            let mut tones: Vec<(f32, KeyInput, bool)> = PINYIN_TONES
                .chars()
                .map(|c| {
                    let input = KeyInput::Char(c);
                    (1.0, input, live(kind, input, enabled))
                })
                .collect();
            tones.push((tokens::KEY_UNITS_EDGE, KeyInput::Backspace, true));
            lay_row(3, &tones, &mut out);
        }
        KeyboardKind::Zhuyin => {
            for (r, chars) in ZHUYIN_ROWS.iter().enumerate() {
                let mut entries: Vec<(f32, KeyInput, bool)> = chars
                    .chars()
                    .map(|c| {
                        let input = KeyInput::Char(c);
                        (1.0, input, live(kind, input, enabled))
                    })
                    .collect();
                if r == 3 {
                    entries.push((tokens::KEY_UNITS_EDGE, KeyInput::Backspace, true));
                }
                lay_row(r as i32, &entries, &mut out);
            }
        }
        KeyboardKind::Passphrase => {
            let rows_src: &[&str; 4] = if mods.symbols {
                &SYMBOL_ROWS
            } else {
                &PASS_ROWS
            };
            for (r, chars) in rows_src.iter().enumerate() {
                let mut entries: Vec<(f32, KeyInput, bool)> = chars
                    .chars()
                    .map(|c| {
                        let c = if mods.shift && !mods.symbols {
                            c.to_ascii_uppercase()
                        } else {
                            c
                        };
                        (1.0, KeyInput::Char(c), true)
                    })
                    .collect();
                if r == 3 {
                    if mods.symbols {
                        // "`~" plus a wide backspace.
                        entries.push((tokens::KEY_UNITS_EDGE, KeyInput::Backspace, true));
                    } else {
                        entries.insert(0, (tokens::KEY_UNITS_EDGE, KeyInput::Shift, true));
                        entries.push((tokens::KEY_UNITS_EDGE, KeyInput::Backspace, true));
                    }
                }
                lay_row(r as i32, &entries, &mut out);
            }
            let bottom = [
                (2.0, KeyInput::Symbols, true),
                (tokens::KEY_UNITS_SPACE, KeyInput::Char(' '), true),
                (2.0, KeyInput::Done, enabled & DONE_DISABLED == 0),
            ];
            lay_row(4, &bottom, &mut out);
        }
    }
    out
}

/// Whether a key of `kind` accepts taps under `enabled`: a character
/// key follows its own bit, backspace is always live, and ✓ follows
/// [`DONE_DISABLED`].
fn live(kind: KeyboardKind, input: KeyInput, enabled: KeyMask) -> bool {
    match input {
        KeyInput::Char(c) => enabled & key_bit(kind, c) != 0,
        KeyInput::Done => enabled & DONE_DISABLED == 0,
        _ => true,
    }
}

fn round(v: f32) -> i32 {
    (v + 0.5) as i32
}

/// The enabled key under `(x, y)`, if any.
pub fn key_at(caps: &[KeyCap], x: i32, y: i32) -> Option<KeyInput> {
    caps.iter()
        .find(|k| k.enabled && k.rect.contains(x, y))
        .map(|k| k.input)
}

/// Whether a physical-keyboard character is acceptable input for `kind`.
pub fn accepts_char(kind: KeyboardKind, c: char) -> Option<char> {
    match kind {
        KeyboardKind::Bip39 => c.is_ascii_alphabetic().then(|| c.to_ascii_lowercase()),
        // A PIN pad takes the keyboard's digits as well as touch
        // (§16.99): a laptop booted from the stick has a keyboard, and
        // the shuffle still guards a touched entry.
        KeyboardKind::Pin => c.is_ascii_digit().then_some(c),
        KeyboardKind::Hex => c.is_ascii_hexdigit().then(|| c.to_ascii_uppercase()),
        KeyboardKind::Passphrase => (c.is_ascii_graphic() || c == ' ').then_some(c),
        KeyboardKind::Dice => ('1'..='6').contains(&c).then_some(c),
        KeyboardKind::Binary => matches!(c, '0' | '1').then_some(c),
        KeyboardKind::Coin => match c.to_ascii_uppercase() {
            'H' => Some('H'),
            'T' => Some('T'),
            _ => None,
        },
        // The ranks in either case, and the four suit glyphs. `10` is
        // typed as `T`, which is what the key shows.
        KeyboardKind::Cards => {
            let upper = c.to_ascii_uppercase();
            if CARD_RANK_ROWS.iter().any(|r| r.contains(upper)) {
                Some(upper)
            } else {
                CARD_SUITS.contains(&c).then_some(c)
            }
        }
        // An address is ASCII letters and digits, in either case; which
        // of them can follow what has been typed is the mask's business
        // and not this one's.
        KeyboardKind::Address => c.is_ascii_alphanumeric().then_some(c),
        // A codex32 string is written in one case and read in either,
        // so the keys type lower case whichever case is pressed.
        KeyboardKind::Codex32 => c.is_ascii_alphanumeric().then(|| c.to_ascii_lowercase()),
        // `'` is the other common hardened marker; it types as `h`.
        KeyboardKind::Path => match c {
            '0'..='9' | '/' => Some(c),
            'h' | 'H' | '\'' => Some('h'),
            _ => None,
        },
        // Hiragana and the two voicing marks, in either the standalone
        // or the combining form. No romanisation: a kana keyboard types
        // kana.
        KeyboardKind::Kana => match c {
            '\u{3041}'..='\u{3096}' | DAKUTEN_KEY | HANDAKUTEN_KEY | SMALL_KEY => Some(c),
            '\u{3099}' => Some(DAKUTEN_KEY),
            '\u{309A}' => Some(HANDAKUTEN_KEY),
            _ => None,
        },
        // The Hangul compatibility jamo, which is what the keys carry.
        KeyboardKind::Jamo => ('\u{3131}'..='\u{3163}').contains(&c).then_some(c),
        // Latin letters spell the syllable and a digit gives the tone.
        KeyboardKind::Pinyin => match c {
            'a'..='z' | 'A'..='Z' => Some(c.to_ascii_lowercase()),
            '1'..='5' => Some(c),
            _ => None,
        },
        // Bopomofo and the four tone marks. No romanisation.
        KeyboardKind::Zhuyin => match c {
            '\u{3105}'..='\u{3129}' => Some(c),
            _ => ZHUYIN_MARKS.contains(&c).then_some(c),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geom::Scale;

    fn ctx(class: SizeClass) -> LayoutCtx {
        LayoutCtx::new(Scale::IDENTITY, class)
    }

    #[test]
    fn bip39_disables_letters_outside_the_mask() {
        let area = Rect::new(0, 0, 360, 140);
        let caps = keys(
            KeyboardKind::Bip39,
            area,
            &ctx(SizeClass::Mobile),
            mask_of("abc"),
            None,
            Modifiers::default(),
        );
        // Twenty-six letters and the backspace: no ✓ on a wordlist
        // keyboard (`docs/PLANNING.md` §16.118).
        assert_eq!(caps.len(), 26 + 1);
        assert_eq!(caps[caps.len() - 1].input, KeyInput::Backspace);
        let find = |c: char| caps.iter().find(|k| k.input == KeyInput::Char(c)).unwrap();
        assert!(find('a').enabled && find('b').enabled && find('c').enabled);
        assert!(!find('d').enabled && !find('q').enabled);
        assert!(
            caps.iter()
                .any(|k| k.input == KeyInput::Backspace && k.enabled)
        );
        // A disabled key does not hit.
        let q = find('q').rect.center();
        assert_eq!(key_at(&caps, q.x, q.y), None);
        let a = find('a').rect.center();
        assert_eq!(key_at(&caps, a.x, a.y), Some(KeyInput::Char('a')));
    }

    /// A word is taken by a tap on its candidate, so the five keyboards
    /// that type a wordlist carry no ✓ and their backspace holds the
    /// bottom-right cell (`docs/PLANNING.md` §16.118).
    #[test]
    fn a_wordlist_keyboard_has_no_check_and_ends_in_the_backspace() {
        for kind in [
            KeyboardKind::Bip39,
            KeyboardKind::Kana,
            KeyboardKind::Jamo,
            KeyboardKind::Pinyin,
            KeyboardKind::Zhuyin,
        ] {
            let area = Rect::new(0, 0, 268, 200);
            let caps = keys(
                kind,
                area,
                &ctx(SizeClass::Small),
                ALL_KEYS,
                None,
                Modifiers::default(),
            );
            assert!(
                !caps.iter().any(|k| k.input == KeyInput::Done),
                "{kind:?} carries a ✓"
            );
            let last = caps
                .iter()
                .max_by_key(|k| (k.rect.bottom(), k.rect.right()))
                .expect("a key");
            assert_eq!(
                last.input,
                KeyInput::Backspace,
                "{kind:?}: the bottom-right key"
            );
            assert!(last.enabled, "{kind:?}: the backspace is always live");
        }
    }

    #[test]
    fn keys_tile_the_area_without_overlap() {
        for kind in [
            KeyboardKind::Bip39,
            KeyboardKind::Pin,
            KeyboardKind::Hex,
            KeyboardKind::Passphrase,
            KeyboardKind::Dice,
            KeyboardKind::Coin,
            KeyboardKind::Path,
            KeyboardKind::Address,
            KeyboardKind::Kana,
            KeyboardKind::Jamo,
            KeyboardKind::Pinyin,
            KeyboardKind::Zhuyin,
        ] {
            let area = Rect::new(7, 11, 353, 211);
            let caps = keys(
                kind,
                area,
                &ctx(SizeClass::Small),
                ALL_KEYS,
                Some(3),
                Modifiers::default(),
            );
            for (i, a) in caps.iter().enumerate() {
                assert!(
                    a.rect.x >= area.x && a.rect.right() <= area.right(),
                    "{kind:?} x"
                );
                assert!(
                    a.rect.y >= area.y && a.rect.bottom() <= area.bottom(),
                    "{kind:?} y"
                );
                for b in &caps[i + 1..] {
                    assert!(
                        a.rect.intersect(&b.rect).is_empty(),
                        "{kind:?} overlap {a:?} {b:?}"
                    );
                }
            }
            // Each row ends flush with the right edge.
            let max_right = caps.iter().map(|k| k.rect.right()).max().unwrap();
            assert_eq!(max_right, area.right());
        }
    }

    #[test]
    fn pin_scramble_is_deterministic_and_a_permutation() {
        let a = scrambled_digits(42);
        let b = scrambled_digits(42);
        assert_eq!(a, b);
        let mut sorted = a;
        sorted.sort_unstable();
        assert_eq!(sorted, ['0', '1', '2', '3', '4', '5', '6', '7', '8', '9']);
        assert_ne!(scrambled_digits(1), scrambled_digits(2));
        // The unscrambled pad has 1 top-left and 0 bottom-centre.
        let caps = keys(
            KeyboardKind::Pin,
            Rect::new(0, 0, 300, 400),
            &ctx(SizeClass::Mobile),
            0,
            None,
            Modifiers::default(),
        );
        assert_eq!(caps[0].input, KeyInput::Char('1'));
        assert_eq!(caps[10].input, KeyInput::Char('0'));
    }

    /// The hit rectangles cover the whole keyboard region with no gaps
    /// and no overlaps, so every pixel of it, out to the screen edges,
    /// belongs to exactly one key (UX.md §6).
    #[test]
    fn hit_rects_tile_the_region_out_to_its_edges() {
        for kind in [
            KeyboardKind::Bip39,
            KeyboardKind::Pin,
            KeyboardKind::Hex,
            KeyboardKind::Passphrase,
            KeyboardKind::Dice,
            KeyboardKind::Coin,
            KeyboardKind::Path,
            KeyboardKind::Kana,
            KeyboardKind::Jamo,
            KeyboardKind::Pinyin,
            KeyboardKind::Zhuyin,
        ] {
            for class in [SizeClass::Small, SizeClass::Mobile] {
                let c = ctx(class);
                // The region a screen gives a keyboard: full width, down
                // to the bottom edge.
                let area = Rect::new(0, 358 - 190, 268, 190);
                let caps = keys(kind, area, &c, ALL_KEYS, Some(3), Modifiers::default());
                // Rows partition the height, columns partition each row.
                let mut ys: alloc::vec::Vec<(i32, i32)> =
                    caps.iter().map(|k| (k.rect.y, k.rect.bottom())).collect();
                ys.sort_unstable();
                ys.dedup();
                assert_eq!(ys[0].0, area.y, "{kind:?} starts at the top");
                assert_eq!(
                    ys[ys.len() - 1].1,
                    area.bottom(),
                    "{kind:?} reaches the bottom edge"
                );
                for w in ys.windows(2) {
                    assert_eq!(w[0].1, w[1].0, "{kind:?} rows are adjacent");
                }
                // The kana grid is the one keyboard with an empty cell:
                // the 五十音 leaves five cells for four keys (§16.118).
                let mut empty: i64 = 0;
                let mut gaps = 0;
                for &(y, bottom) in &ys {
                    let mut row: alloc::vec::Vec<&KeyCap> = caps
                        .iter()
                        .filter(|k| k.rect.y == y && k.rect.bottom() == bottom)
                        .collect();
                    row.sort_by_key(|k| k.rect.x);
                    assert_eq!(row[0].rect.x, area.x, "{kind:?} reaches the left edge");
                    let h = i64::from(bottom - y);
                    let mut gap = |w: i32| {
                        if w > 0 {
                            gaps += 1;
                            empty += i64::from(w) * h;
                        }
                    };
                    gap(area.right() - row[row.len() - 1].rect.right());
                    for w in row.windows(2) {
                        gap(w[1].rect.x - w[0].rect.right());
                    }
                }
                assert_eq!(
                    gaps,
                    usize::from(kind == KeyboardKind::Kana),
                    "{kind:?} leaves {empty} square pixels to no key"
                );
                // Every other point of the region resolves to a cell.
                let area_px: i64 = i64::from(area.w) * i64::from(area.h);
                let covered: i64 = caps
                    .iter()
                    .map(|k| i64::from(k.rect.w) * i64::from(k.rect.h))
                    .sum();
                assert_eq!(covered + empty, area_px, "{kind:?} covers the whole region");
            }
        }
    }

    #[test]
    fn passphrase_layers_cover_printable_ascii() {
        let area = Rect::new(0, 0, 400, 240);
        let c = ctx(SizeClass::Mobile);
        let mut seen = alloc::collections::BTreeSet::new();
        for (shift, symbols) in [(false, false), (true, false), (false, true)] {
            for k in keys(
                KeyboardKind::Passphrase,
                area,
                &c,
                0,
                None,
                Modifiers { shift, symbols },
            ) {
                if let KeyInput::Char(ch) = k.input {
                    seen.insert(ch);
                }
            }
        }
        for b in 0x20u8..=0x7E {
            assert!(seen.contains(&(b as char)), "missing {:?}", b as char);
        }
        assert_eq!(seen.len(), 95);
    }

    #[test]
    fn physical_keys_are_filtered_per_kind() {
        assert_eq!(accepts_char(KeyboardKind::Bip39, 'Q'), Some('q'));
        assert_eq!(accepts_char(KeyboardKind::Bip39, '1'), None);
        assert_eq!(accepts_char(KeyboardKind::Pin, '7'), Some('7'));
        assert_eq!(accepts_char(KeyboardKind::Pin, 'a'), None);
        assert_eq!(accepts_char(KeyboardKind::Hex, 'b'), Some('B'));
        assert_eq!(accepts_char(KeyboardKind::Hex, 'g'), None);
        assert_eq!(accepts_char(KeyboardKind::Passphrase, ' '), Some(' '));
        assert_eq!(accepts_char(KeyboardKind::Passphrase, 'é'), None);
        assert_eq!(accepts_char(KeyboardKind::Dice, '6'), Some('6'));
        assert_eq!(accepts_char(KeyboardKind::Dice, '0'), None);
        assert_eq!(accepts_char(KeyboardKind::Dice, '7'), None);
        assert_eq!(accepts_char(KeyboardKind::Coin, 'h'), Some('H'));
        assert_eq!(accepts_char(KeyboardKind::Coin, 'T'), Some('T'));
        assert_eq!(accepts_char(KeyboardKind::Coin, 'x'), None);
        assert_eq!(accepts_char(KeyboardKind::Path, '7'), Some('7'));
        assert_eq!(accepts_char(KeyboardKind::Path, '/'), Some('/'));
        assert_eq!(accepts_char(KeyboardKind::Path, '\''), Some('h'));
        assert_eq!(accepts_char(KeyboardKind::Path, 'H'), Some('h'));
        assert_eq!(accepts_char(KeyboardKind::Path, 'm'), None);
        // The kana keyboard takes kana and the voicing marks in either
        // form, and no romanisation.
        assert_eq!(
            accepts_char(KeyboardKind::Kana, '\u{3042}'),
            Some('\u{3042}')
        );
        assert_eq!(
            accepts_char(KeyboardKind::Kana, '\u{3063}'),
            Some('\u{3063}')
        );
        assert_eq!(
            accepts_char(KeyboardKind::Kana, '\u{3099}'),
            Some(DAKUTEN_KEY)
        );
        assert_eq!(accepts_char(KeyboardKind::Kana, 'a'), None);
        assert_eq!(
            accepts_char(KeyboardKind::Kana, '\u{30A2}'),
            None,
            "katakana"
        );
        // The jamo keyboard takes compatibility jamo only.
        assert_eq!(
            accepts_char(KeyboardKind::Jamo, '\u{3131}'),
            Some('\u{3131}')
        );
        assert_eq!(
            accepts_char(KeyboardKind::Jamo, '\u{AC00}'),
            None,
            "a syllable"
        );
        assert_eq!(accepts_char(KeyboardKind::Jamo, 'r'), None);
    }

    /// The kana keyboard is the 五十音: ten columns of five, with the
    /// two voicing marks, 小 and the backspace in the cells the grid
    /// leaves empty, and every kana the Japanese list uses on a key.
    #[test]
    fn the_kana_grid_carries_the_fifty_sounds_and_its_four_own_keys() {
        let caps = keys(
            KeyboardKind::Kana,
            Rect::new(0, 0, 400, 250),
            &ctx(SizeClass::Mobile),
            ALL_KEYS,
            None,
            Modifiers::default(),
        );
        // Fifty cells, one of them empty: four keys for the five the
        // 五十音 leaves over (§16.118).
        assert_eq!(caps.len(), 49);
        // Row order is the vowels; column order the consonant groups.
        let at = |row: i32, column: usize| {
            let mut ys: Vec<i32> = caps.iter().map(|k| k.rect.y).collect();
            ys.sort_unstable();
            ys.dedup();
            let y = ys[row as usize];
            let mut cells: Vec<&KeyCap> = caps.iter().filter(|k| k.rect.y == y).collect();
            cells.sort_by_key(|k| k.rect.x);
            cells.get(column).map(|k| k.input)
        };
        assert_eq!(at(0, 0), Some(KeyInput::Char('\u{3042}')), "あ");
        assert_eq!(at(1, 7), Some(KeyInput::Char(DAKUTEN_KEY)));
        assert_eq!(at(3, 7), Some(KeyInput::Char(HANDAKUTEN_KEY)));
        assert_eq!(at(1, 9), Some(KeyInput::Char(SMALL_KEY)));
        // The backspace holds the bottom-right cell, ん the one above
        // it, and the cell over that is empty.
        assert_eq!(at(4, 9), Some(KeyInput::Backspace));
        assert_eq!(at(3, 9), Some(KeyInput::Char('\u{3093}')), "ん");
        assert_eq!(at(2, 9), None);
        assert!(
            !caps.iter().any(|k| k.input == KeyInput::Done),
            "no ✓ on a wordlist keyboard"
        );
        // A mask dims what it leaves out.
        let one = keys(
            KeyboardKind::Kana,
            Rect::new(0, 0, 400, 250),
            &ctx(SizeClass::Mobile),
            key_bit(KeyboardKind::Kana, '\u{3042}') | DONE_DISABLED,
            None,
            Modifiers::default(),
        );
        let live: Vec<KeyInput> = one.iter().filter(|k| k.enabled).map(|k| k.input).collect();
        assert_eq!(live, [KeyInput::Char('\u{3042}'), KeyInput::Backspace]);
    }

    /// The jamo keyboard is the two-set layout, and shift swaps the five
    /// doubled consonants and the two shifted vowels.
    #[test]
    fn the_jamo_rows_carry_the_two_set_layout_and_shift_doubles_five() {
        let area = Rect::new(0, 0, 400, 160);
        let plain = keys(
            KeyboardKind::Jamo,
            area,
            &ctx(SizeClass::Mobile),
            ALL_KEYS,
            None,
            Modifiers::default(),
        );
        // Ten, nine, and seven with shift at the left of the last row
        // and the backspace at its right.
        assert_eq!(plain.len(), 10 + 9 + 9);
        let chars = |caps: &[KeyCap]| -> alloc::string::String {
            caps.iter()
                .filter_map(|k| match k.input {
                    KeyInput::Char(c) => Some(c),
                    _ => None,
                })
                .collect()
        };
        assert_eq!(
            chars(&plain),
            concat!(
                "\u{3142}\u{3148}\u{3137}\u{3131}\u{3145}\u{315B}\u{3155}\u{3151}\u{3150}\u{3154}",
                "\u{3141}\u{3134}\u{3147}\u{3139}\u{314E}\u{3157}\u{3153}\u{314F}\u{3163}",
                "\u{314B}\u{314C}\u{314A}\u{314D}\u{3160}\u{315C}\u{3161}"
            )
        );
        assert_eq!(plain[19].input, KeyInput::Shift);
        assert!(
            plain
                .iter()
                .any(|k| k.input == KeyInput::Backspace && k.enabled)
        );
        let shifted = keys(
            KeyboardKind::Jamo,
            area,
            &ctx(SizeClass::Mobile),
            ALL_KEYS,
            None,
            Modifiers {
                shift: true,
                symbols: false,
            },
        );
        assert_eq!(
            chars(&shifted)
                .chars()
                .take(10)
                .collect::<alloc::string::String>(),
            "\u{3143}\u{3149}\u{3138}\u{3132}\u{3146}\u{315B}\u{3155}\u{3151}\u{3152}\u{3156}"
        );
        // A shifted key follows the enabled set of the character it
        // types, not of the one under it.
        let only_double = keys(
            KeyboardKind::Jamo,
            area,
            &ctx(SizeClass::Mobile),
            key_bit(KeyboardKind::Jamo, '\u{3132}') | DONE_DISABLED,
            None,
            Modifiers {
                shift: true,
                symbols: false,
            },
        );
        let live: Vec<char> = only_double
            .iter()
            .filter(|k| k.enabled)
            .filter_map(|k| match k.input {
                KeyInput::Char(c) => Some(c),
                _ => None,
            })
            .collect();
        assert_eq!(live, ['\u{3132}']);
    }

    /// The pinyin keyboard is the BIP-39 letter rows and a fourth row of
    /// the five tones, ending with the backspace that row 3 carries on
    /// the letters-only keyboard.
    #[test]
    fn the_pinyin_rows_are_the_letters_and_a_row_of_tones() {
        let area = Rect::new(0, 0, 400, 200);
        let caps = keys(
            KeyboardKind::Pinyin,
            area,
            &ctx(SizeClass::Mobile),
            ALL_KEYS,
            None,
            Modifiers::default(),
        );
        assert_eq!(caps.len(), 26 + 5 + 1);
        let chars: alloc::string::String = caps
            .iter()
            .filter_map(|k| match k.input {
                KeyInput::Char(c) => Some(c),
                _ => None,
            })
            .collect();
        assert_eq!(chars, "qwertyuiopasdfghjklzxcvbnm12345");
        // The tone row: the five tones and the backspace after them.
        assert_eq!(caps[31].input, KeyInput::Backspace);
        // Rows: ten, nine, seven, and the six of the tone row.
        let rows_of = |caps: &[KeyCap]| -> Vec<usize> {
            let mut ys: Vec<i32> = caps.iter().map(|k| k.rect.y).collect();
            ys.sort_unstable();
            ys.dedup();
            ys.iter()
                .map(|y| caps.iter().filter(|k| k.rect.y == *y).count())
                .collect()
        };
        assert_eq!(rows_of(&caps), [10, 9, 7, 6]);
        // A tone key is dead until the mask offers it.
        let one = keys(
            KeyboardKind::Pinyin,
            area,
            &ctx(SizeClass::Mobile),
            key_bit(KeyboardKind::Pinyin, '1') | DONE_DISABLED,
            None,
            Modifiers::default(),
        );
        let live: Vec<KeyInput> = one.iter().filter(|k| k.enabled).map(|k| k.input).collect();
        assert_eq!(live, [KeyInput::Char('1'), KeyInput::Backspace]);
    }

    /// The 注音 keyboard is the 大千 layout: the marks in the top row
    /// where that layout puts them, ㄦ at its end, every bopomofo letter
    /// on a key, and ˉ as an eleventh key ending the second row, where
    /// 大千 leaves the first tone to a space bar this keyboard lacks.
    #[test]
    fn the_zhuyin_rows_are_the_dachien_layout() {
        let caps = keys(
            KeyboardKind::Zhuyin,
            Rect::new(0, 0, 400, 200),
            &ctx(SizeClass::Mobile),
            ALL_KEYS,
            None,
            Modifiers::default(),
        );
        assert_eq!(caps.len(), 42 + 1);
        let chars: alloc::string::String = caps
            .iter()
            .filter_map(|k| match k.input {
                KeyInput::Char(c) => Some(c),
                _ => None,
            })
            .collect();
        assert_eq!(
            chars,
            concat!(
                "\u{3105}\u{3109}\u{02C7}\u{02CB}\u{3113}\u{02CA}\u{02D9}\u{311A}\u{311E}\u{3122}\u{3126}",
                "\u{3106}\u{310A}\u{310D}\u{3110}\u{3114}\u{3117}\u{3127}\u{311B}\u{311F}\u{3123}\u{02C9}",
                "\u{3107}\u{310B}\u{310E}\u{3111}\u{3115}\u{3118}\u{3128}\u{311C}\u{3120}\u{3124}",
                "\u{3108}\u{310C}\u{310F}\u{3112}\u{3116}\u{3119}\u{3129}\u{311D}\u{3121}\u{3125}",
            )
        );
        // Every bopomofo letter and every mark is on a key, once.
        let mut set: alloc::collections::BTreeSet<char> = keycaps(KeyboardKind::Zhuyin).collect();
        assert_eq!(set.len(), 42);
        for c in '\u{3105}'..='\u{3129}' {
            assert!(set.remove(&c), "{c} has no key");
        }
        for m in ZHUYIN_MARKS {
            assert!(set.remove(&m), "{m} has no key");
        }
        assert!(set.is_empty());
        assert_eq!(caps[42].input, KeyInput::Backspace);
        // The rows: eleven, eleven ending in ˉ, ten, and ten with the
        // backspace after them.
        let mut ys: Vec<i32> = caps.iter().map(|k| k.rect.y).collect();
        ys.sort_unstable();
        ys.dedup();
        let row: Vec<Vec<KeyInput>> = ys
            .iter()
            .map(|y| {
                caps.iter()
                    .filter(|k| k.rect.y == *y)
                    .map(|k| k.input)
                    .collect()
            })
            .collect();
        assert_eq!(
            row.iter().map(Vec::len).collect::<Vec<_>>(),
            [11, 11, 10, 11]
        );
        assert_eq!(row[1][10], KeyInput::Char(TONE_HIGH));
    }

    #[test]
    fn path_keyboard_has_digits_slash_and_hardened_marker() {
        let area = Rect::new(0, 0, 360, 140);
        let caps = keys(
            KeyboardKind::Path,
            area,
            &ctx(SizeClass::Mobile),
            0,
            None,
            Modifiers::default(),
        );
        assert_eq!(caps.len(), 10 + 2 + 2);
        for c in "0123456789/h".chars() {
            assert!(caps.iter().any(|k| k.input == KeyInput::Char(c)), "{c}");
        }
        assert!(caps.iter().all(|k| k.enabled));
    }

    #[test]
    fn the_dice_and_coin_pads_carry_their_faces_and_a_dead_check() {
        let area = Rect::new(0, 0, 400, 100);
        let caps = keys(
            KeyboardKind::Dice,
            area,
            &ctx(SizeClass::Small),
            0,
            None,
            Modifiers::default(),
        );
        assert_eq!(caps.len(), 8);
        for d in '1'..='6' {
            assert!(caps.iter().any(|k| k.input == KeyInput::Char(d)), "{d}");
        }
        assert!(
            caps.iter()
                .find(|k| k.input == KeyInput::Done)
                .unwrap()
                .enabled
        );
        let caps_off = keys(
            KeyboardKind::Dice,
            area,
            &ctx(SizeClass::Small),
            DONE_DISABLED,
            None,
            Modifiers::default(),
        );
        assert!(
            !caps_off
                .iter()
                .find(|k| k.input == KeyInput::Done)
                .unwrap()
                .enabled
        );
        assert_eq!(key_at(&caps, 5, 5), Some(KeyInput::Char('1')));
        assert_eq!(key_at(&caps, 395, 5), Some(KeyInput::Backspace));
        assert_eq!(key_at(&caps, 395, 95), Some(KeyInput::Done));
        let coin = keys(
            KeyboardKind::Coin,
            area,
            &ctx(SizeClass::Mobile),
            0,
            None,
            Modifiers::default(),
        );
        assert_eq!(coin.len(), 4);
        assert_eq!(coin[0].input, KeyInput::Char('H'));
        assert_eq!(coin[1].input, KeyInput::Char('T'));
    }
}
