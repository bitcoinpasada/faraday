//! Kana: the voicing marks and the small forms the Japanese list is
//! written with, and the composition the entry field reads back.
//!
//! The published Japanese list is NFKD: base kana followed by the
//! combining voicing marks U+3099 and U+309A, and the small kana as
//! their own code points. That is also the order the fingers type in, so
//! a typed sequence is the word itself. What a reader needs is the
//! composed form, which is what [`compose`] gives: `か` + U+3099 is `が`.
//!
//! No Unicode tables: twenty-five voiced kana and five small ones are
//! the whole of what the list uses.

/// The combining voiced sound mark, which the ゛ key types.
pub const DAKUTEN: char = '\u{3099}';

/// The combining semi-voiced sound mark, which the ゜ key types.
pub const HANDAKUTEN: char = '\u{309A}';

/// The standalone ゛ the keycap shows. The key types [`DAKUTEN`].
pub const DAKUTEN_KEY: char = '\u{309B}';

/// The standalone ゜ the keycap shows. The key types [`HANDAKUTEN`].
pub const HANDAKUTEN_KEY: char = '\u{309C}';

/// The 小 key, which turns the last kana typed into its small form and
/// back.
pub const SMALL_KEY: char = '\u{5C0F}';

/// Base kana, mark, and the kana the two compose to. Twenty for the
/// voiced mark and five for the semi-voiced one, which is the whole of
/// what the Japanese list uses.
const VOICED: [(char, char, char); 25] = [
    ('か', DAKUTEN, 'が'),
    ('き', DAKUTEN, 'ぎ'),
    ('く', DAKUTEN, 'ぐ'),
    ('け', DAKUTEN, 'げ'),
    ('こ', DAKUTEN, 'ご'),
    ('さ', DAKUTEN, 'ざ'),
    ('し', DAKUTEN, 'じ'),
    ('す', DAKUTEN, 'ず'),
    ('せ', DAKUTEN, 'ぜ'),
    ('そ', DAKUTEN, 'ぞ'),
    ('た', DAKUTEN, 'だ'),
    ('ち', DAKUTEN, 'ぢ'),
    ('つ', DAKUTEN, 'づ'),
    ('て', DAKUTEN, 'で'),
    ('と', DAKUTEN, 'ど'),
    ('は', DAKUTEN, 'ば'),
    ('ひ', DAKUTEN, 'び'),
    ('ふ', DAKUTEN, 'ぶ'),
    ('へ', DAKUTEN, 'べ'),
    ('ほ', DAKUTEN, 'ぼ'),
    ('は', HANDAKUTEN, 'ぱ'),
    ('ひ', HANDAKUTEN, 'ぴ'),
    ('ふ', HANDAKUTEN, 'ぷ'),
    ('へ', HANDAKUTEN, 'ぺ'),
    ('ほ', HANDAKUTEN, 'ぽ'),
];

/// Full-size kana and its small form. The 小 key moves between the two.
const SMALL: [(char, char); 5] = [
    ('い', 'ぃ'),
    ('つ', 'っ'),
    ('や', 'ゃ'),
    ('ゆ', 'ゅ'),
    ('よ', 'ょ'),
];

/// The kana `base` and `mark` compose to, or `None` when they do not
/// compose.
pub fn voiced(base: char, mark: char) -> Option<char> {
    VOICED
        .iter()
        .find(|&&(b, m, _)| b == base && m == mark)
        .map(|&(_, _, c)| c)
}

/// The small form of `c`, or `None` when it has none.
pub fn small(c: char) -> Option<char> {
    SMALL.iter().find(|&&(big, _)| big == c).map(|&(_, s)| s)
}

/// The full-size form of a small kana, or `None` when `c` is not one.
pub fn large(c: char) -> Option<char> {
    SMALL.iter().find(|&&(_, s)| s == c).map(|&(big, _)| big)
}

/// Whether `c` is one of the two combining voicing marks.
pub fn is_mark(c: char) -> bool {
    c == DAKUTEN || c == HANDAKUTEN
}

/// The key a typed character is reached by: a kana is its own key, a
/// small kana is its full-size key (the 小 key makes it small), and the
/// two combining marks are the ゛ and ゜ keys.
pub fn key_for(c: char) -> char {
    if let Some(big) = large(c) {
        return big;
    }
    match c {
        DAKUTEN => DAKUTEN_KEY,
        HANDAKUTEN => HANDAKUTEN_KEY,
        other => other,
    }
}

/// Longest composed run [`compose`] returns, which is one character per
/// character given.
pub const MAX_COMPOSED: usize = 16;

/// A composed run of kana, inline.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Composed {
    buf: [char; MAX_COMPOSED],
    len: u8,
}

impl Composed {
    /// The composed characters.
    pub fn as_chars(&self) -> &[char] {
        &self.buf[..usize::from(self.len)]
    }

    fn push(&mut self, c: char) {
        if usize::from(self.len) < MAX_COMPOSED {
            self.buf[usize::from(self.len)] = c;
            self.len += 1;
        }
    }
}

/// The NFKD characters of a Japanese word, or of a prefix being typed,
/// as a reader sees them: each voicing mark joined to the kana before
/// it. A mark with nothing to join is kept, so a half-typed `か` `゛` is
/// never lost.
pub fn compose(chars: impl IntoIterator<Item = char>) -> Composed {
    let mut out = Composed {
        buf: ['\0'; MAX_COMPOSED],
        len: 0,
    };
    for c in chars {
        if is_mark(c)
            && out.len > 0
            && let Some(joined) = voiced(out.buf[usize::from(out.len) - 1], c)
        {
            out.buf[usize::from(out.len) - 1] = joined;
            continue;
        }
        out.push(c);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bip39::Language;

    /// Every Japanese word reads as its published composed form once the
    /// voicing marks are joined to the kana they belong to.
    #[test]
    fn every_japanese_word_composes_to_the_form_a_reader_sees() {
        for i in 0..2048u16 {
            let composed = compose(Language::Japanese.word_nfkd(i).chars());
            let shown: alloc::vec::Vec<char> = Language::Japanese.word_display(i).chars().collect();
            assert_eq!(composed.as_chars(), shown.as_slice(), "word {i}");
        }
    }

    #[test]
    fn a_mark_with_nothing_to_join_stays_on_the_field() {
        let c = compose(['あ', DAKUTEN]);
        assert_eq!(c.as_chars(), ['あ', DAKUTEN]);
        let c = compose(['か', DAKUTEN]);
        assert_eq!(c.as_chars(), ['が']);
    }
}
