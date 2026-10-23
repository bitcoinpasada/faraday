//! The booklet's second alphabet is drawn as its glyphs: every symbol of
//! `SYMBOL_GLYPHS` has an outline of its own in each text face, not the
//! fallback `?` and not the name of the glyph.

use faraday_bip::codex32::SYMBOL_GLYPHS;
use faraday_draw::fonts::{Family, outlines};

#[test]
fn every_second_alphabet_symbol_is_in_every_text_face() {
    for family in [
        Family::Regular,
        Family::SemiBold,
        Family::RegularNarrow,
        Family::SemiBoldNarrow,
        Family::Mono,
    ] {
        let o = outlines(family);
        // Both code points of Delta and of mu, as the booklet's PDF and
        // the Greek block give them.
        let extra = ['\u{2206}', '\u{b5}'];
        for c in SYMBOL_GLYPHS.iter().copied().chain(extra) {
            let g = o
                .glyph(c)
                .unwrap_or_else(|| panic!("{family:?} has no glyph for U+{:04X}", c as u32));
            assert!(
                g.outline.advance > 0,
                "{family:?} U+{:04X} has no advance",
                c as u32
            );
            assert!(
                !g.outline.commands.is_empty(),
                "{family:?} U+{:04X} has no outline",
                c as u32
            );
        }
    }
}
