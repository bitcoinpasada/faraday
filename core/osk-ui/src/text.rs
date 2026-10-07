//! Text measurement and word wrapping over the glyph outlines.
//!
//! Positions are integer pixels; advances accumulate in 1/64 px and each
//! glyph is placed at the rounded pen position, so a run of text is
//! pixel-deterministic for a given face and size. Measurement reads the
//! advances in the outline file and never rasterises anything.

use alloc::vec::Vec;

use crate::fonts::{Family, SizedFace};
use crate::geom::{Dp, Scale};

/// A family at a size in dp. The size is scaled by the display DPI and
/// rounded to a whole pixel; every size is drawn at its own size.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Font {
    /// Family.
    pub family: Family,
    /// Size in dp (roughly the em size).
    pub size: Dp,
}

impl Font {
    /// A font.
    pub const fn new(family: Family, size_dp: f32) -> Self {
        Font {
            family,
            size: Dp(size_dp),
        }
    }

    /// Noto Sans Regular at `size_dp`.
    pub const fn regular(size_dp: f32) -> Self {
        Font::new(Family::Regular, size_dp)
    }

    /// Noto Sans SemiBold at `size_dp`: titles, row labels, buttons.
    pub const fn semibold(size_dp: f32) -> Self {
        Font::new(Family::SemiBold, size_dp)
    }

    /// Noto Sans Mono at `size_dp`.
    pub const fn mono(size_dp: f32) -> Self {
        Font::new(Family::Mono, size_dp)
    }

    /// The icon face at `size_dp` (see [`crate::widgets::Icon`]).
    pub const fn icon(size_dp: f32) -> Self {
        Font::new(Family::Icon, size_dp)
    }

    /// The same font at another size.
    pub const fn with_size(self, size_dp: f32) -> Self {
        Font {
            family: self.family,
            size: Dp(size_dp),
        }
    }

    /// The face and pixel size this font resolves to on a display.
    ///
    /// Text takes the scale's type factor, which grows the ramp with the
    /// size class (`widgets::tokens::type_scale_pct`); the icon face does
    /// not, because an icon is drawn to fit a box a screen asked for in
    /// dp.
    pub fn sized(&self, scale: Scale) -> SizedFace {
        let px = if self.family == Family::Icon {
            scale.px_f(self.size)
        } else {
            scale.type_px_f(self.size)
        };
        // A class that sets its text narrow (`tokens::narrow_text`) takes
        // the narrow cut of the two text faces; the mono face and the
        // icons are the same on every class.
        let family = match (self.family, scale.narrow()) {
            (Family::Regular, true) => Family::RegularNarrow,
            (Family::SemiBold, true) => Family::SemiBoldNarrow,
            (f, _) => f,
        };
        SizedFace::new(family, (px + 0.5).max(1.0) as u16)
    }
}

/// Horizontal alignment of text within a rectangle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TextAlign {
    /// Left.
    #[default]
    Start,
    /// Centred.
    Center,
    /// Right.
    End,
}

/// The box a single line of text occupies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TextMetrics {
    /// Width in pixels.
    pub width: i32,
    /// Line height in pixels (ascent plus descent).
    pub height: i32,
    /// Baseline offset from the top of the box.
    pub baseline: i32,
}

/// Width of `text` in pixels (no wrapping; newlines are not special).
pub fn width(face: &SizedFace, text: &str) -> i32 {
    let mut pen = 0u32;
    for c in text.chars() {
        pen += face.advance_64(c);
    }
    px_from_64(pen)
}

/// Rounds a 1/64 px pen position to whole pixels.
pub fn px_from_64(v: u32) -> i32 {
    ((v + 32) >> 6) as i32
}

/// Single-line metrics of `text`.
pub fn measure(face: &SizedFace, text: &str) -> TextMetrics {
    TextMetrics {
        width: width(face, text),
        height: face.line_height(),
        baseline: face.ascent,
    }
}

/// Splits `text` into lines no wider than `max_width` pixels.
///
/// Breaks at spaces; a word wider than the line is broken between
/// characters; `\n` forces a break. Returns byte ranges into `text`, with
/// trailing spaces excluded from each line. Always returns at least one
/// (possibly empty) line.
pub fn wrap(face: &SizedFace, text: &str, max_width: i32) -> Vec<(usize, usize)> {
    let mut lines = Vec::new();
    for (para_start, para) in paragraphs(text) {
        wrap_paragraph(face, para, para_start, max_width, &mut lines);
    }
    if lines.is_empty() {
        lines.push((0, 0));
    }
    lines
}

fn paragraphs(text: &str) -> impl Iterator<Item = (usize, &str)> {
    let mut start = 0;
    text.split('\n').map(move |p| {
        let s = start;
        start += p.len() + 1;
        (s, p)
    })
}

fn wrap_paragraph(
    face: &SizedFace,
    para: &str,
    base: usize,
    max_width: i32,
    lines: &mut Vec<(usize, usize)>,
) {
    let advance = |c: char| face.advance_64(c);
    // Compare rounded pixel widths, exactly as `width` measures them, so
    // that a line measured to fit never wraps when drawn.
    let fits = |pen_64: u32| px_from_64(pen_64) <= max_width.max(0);
    let space = advance(' ');

    let mut line_start = 0usize;
    let mut line_end = 0usize; // end of the last word placed on the line
    let mut pen = 0u32; // width of the line so far
    let mut empty = true;

    // Words are maximal runs of non-space characters.
    let mut pos = 0usize;
    while pos < para.len() {
        let rest = &para[pos..];
        let skip = rest.len() - rest.trim_start_matches(' ').len();
        let ws = pos + skip;
        if ws >= para.len() {
            break;
        }
        let we = para[ws..].find(' ').map_or(para.len(), |i| ws + i);
        let word = &para[ws..we];
        let word_w: u32 = word.chars().map(advance).sum();
        pos = we;

        if !empty {
            if fits(pen + space + word_w) {
                pen += space + word_w;
                line_end = we;
                continue;
            }
            lines.push((base + line_start, base + line_end));
        }
        // The word starts a line. Split it if it is wider than the line:
        // after the last hyphen that fits, so "XChaCha20-Poly1305" breaks
        // as "XChaCha20-" and "Poly1305", and between characters where no
        // hyphen does.
        let mut start = ws;
        let mut w = 0u32;
        // The byte just past the last hyphen on this line, and the line's
        // width up to it.
        let mut hyphen: Option<(usize, u32)> = None;
        for (i, c) in word.char_indices() {
            let a = advance(c);
            if !fits(w + a) && max_width > 0 && ws + i > start {
                match hyphen.take() {
                    Some((at, w_at)) => {
                        lines.push((base + start, base + at));
                        start = at;
                        w -= w_at;
                    }
                    None => {
                        lines.push((base + start, base + ws + i));
                        start = ws + i;
                        w = 0;
                    }
                }
            }
            w += a;
            if c == '-' && ws + i + 1 < we {
                hyphen = Some((ws + i + 1, w));
            }
        }
        line_start = start;
        pen = w;
        line_end = we;
        empty = false;
    }
    if empty {
        lines.push((
            base + line_start,
            base + trim_end(para, line_start, para.len()),
        ));
    } else {
        lines.push((base + line_start, base + line_end));
    }
}

fn trim_end(s: &str, start: usize, end: usize) -> usize {
    let mut e = end;
    while e > start && s.as_bytes()[e - 1] == b' ' {
        e -= 1;
    }
    e
}

/// Height in pixels of `text` wrapped to `max_width`.
pub fn wrapped_height(face: &SizedFace, text: &str, max_width: i32) -> i32 {
    wrap(face, text, max_width).len() as i32 * face.line_height()
}

/// Width of the widest wrapped line.
pub fn wrapped_width(face: &SizedFace, text: &str, max_width: i32) -> i32 {
    wrap(face, text, max_width)
        .iter()
        .map(|&(s, e)| width(face, &text[s..e]))
        .max()
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use alloc::vec;

    use super::*;

    fn face() -> SizedFace {
        Font::regular(16.0).sized(Scale::IDENTITY)
    }

    fn lines(text: &str, max: i32) -> Vec<&str> {
        wrap(&face(), text, max)
            .into_iter()
            .map(|(s, e)| &text[s..e])
            .collect()
    }

    #[test]
    fn wrap_breaks_at_spaces() {
        let a = face();
        let w = width(&a, "hello world");
        let ls = lines("hello world foo", w);
        assert_eq!(ls, vec!["hello world", "foo"]);
        let ls = lines("hello world foo", w - 1);
        assert_eq!(ls, vec!["hello", "world foo"]);
    }

    #[test]
    fn wrap_breaks_long_words_between_characters() {
        let a = face();
        let w = width(&a, "abcde");
        let ls = lines("abcdefghij", w);
        assert_eq!(ls.len(), 2);
        assert_eq!(ls.concat(), "abcdefghij");
        for l in &ls {
            assert!(width(&a, l) <= w);
        }
    }

    #[test]
    fn wrap_breaks_a_long_word_after_its_hyphen() {
        let a = face();
        let w = width(&a, "XChaCha20-Poly");
        assert_eq!(
            lines("XChaCha20-Poly1305", w),
            vec!["XChaCha20-", "Poly1305"]
        );
    }

    #[test]
    fn wrap_honours_newlines() {
        assert_eq!(lines("a\nb\n\nc", 10_000), vec!["a", "b", "", "c"]);
    }
}
