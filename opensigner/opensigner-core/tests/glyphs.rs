//! Every character the app can put on screen has a glyph to draw with.
//!
//! This crate is the one that sees both the wordlists and the faces, so
//! it is where the two are checked against each other: a word of any of
//! the ten lists, in either the published or the composed form, and a
//! keycap of any of the three non-Latin keyboards, must reach a real
//! outline and not the `?` a missing code point falls back to.

use osk_bip::bip39::Language;
use osk_ui::fonts::{Family, SizedFace, TEXT_FAMILIES, outlines};
use osk_ui::widgets::keyboard::{self, KeyboardKind};

/// The faces text is drawn in. The icon face carries icons alone.
const TEXT_FACES: [Family; 5] = TEXT_FAMILIES;

/// Every keycap of every non-Latin keyboard, from the keyboards
/// themselves, so this cannot drift from what is drawn.
fn keycaps() -> Vec<char> {
    keyboard::keycaps(KeyboardKind::Kana)
        .chain(keyboard::keycaps(KeyboardKind::Jamo))
        .chain(keyboard::keycaps(KeyboardKind::Zhuyin))
        .collect()
}

/// Asserts that `c` is drawn as itself in every text face, with ink.
fn draws(c: char) {
    for family in TEXT_FACES {
        let g = outlines(family)
            .glyph_or_fallback(c)
            .unwrap_or_else(|| panic!("{family:?} draws nothing at all for U+{:04X}", c as u32));
        assert_eq!(
            g.code_point, c,
            "{family:?} falls back to {:?} for U+{:04X} {c:?}",
            g.code_point, c as u32
        );
        assert!(
            !g.outline.commands.is_empty(),
            "{family:?} has an empty outline for U+{:04X} {c:?}",
            c as u32
        );
    }
}

#[test]
fn every_word_of_every_list_can_be_shown() {
    for lang in Language::ALL {
        for i in 0..2048u16 {
            for c in lang.word(i).chars().chain(lang.word_display(i).chars()) {
                draws(c);
            }
        }
    }
}

#[test]
fn every_keycap_of_every_keyboard_can_be_shown() {
    for c in keycaps() {
        draws(c);
    }
}

/// The Japanese separator. It is a space, so it has an advance and no
/// ink; what matters is that it is not drawn as `?` between two words.
#[test]
fn the_ideographic_space_is_a_space() {
    for family in TEXT_FACES {
        let g = outlines(family)
            .glyph_or_fallback(Language::Japanese.separator().chars().next().unwrap())
            .expect("the ideographic space");
        assert_eq!(g.code_point, '\u{3000}');
        assert!(g.outline.commands.is_empty());
        assert!(g.outline.advance > 0);
    }
}

/// A fallback glyph keeps its own proportions: a hanzi is about one em
/// wide at the pixel size that was asked for, whatever the requesting
/// face's own units per em happen to be, and it is wider than a Latin
/// letter at the same size, as it is in any book.
#[test]
fn a_borrowed_glyph_is_the_size_it_was_asked_for() {
    for family in TEXT_FACES {
        let face = SizedFace::new(family, 32);
        let hanzi = f64::from(face.advance_64('中')) / 64.0;
        let latin = f64::from(face.advance_64('M')) / 64.0;
        assert!(
            (30.0..=34.0).contains(&hanzi),
            "{family:?}: 中 is {hanzi} px wide at 32 px"
        );
        assert!(hanzi > latin, "{family:?}: 中 is narrower than M");
    }
}
