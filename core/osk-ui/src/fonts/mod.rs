//! Glyph outlines, the size a display asks for, and the glyph cache.
//!
//! `tools/fontbake` turns the vendored fonts into one outline file per
//! face (`assets/<face>.outl`); `generated.rs` embeds them. This module
//! reads the format described in `tools/fontbake/README.md`. There is no
//! TTF parsing on the device and no baked size list: a glyph is stored
//! once, in font units, and filled with tiny-skia at whatever pixel size
//! a display asks for.

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use tiny_skia::{FillRule, Mask, PathBuilder, Transform};

mod generated;

pub use generated::FACES;

/// A font family (face).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Family {
    /// Noto Sans Regular: body text.
    Regular,
    /// Noto Sans SemiBold: titles, row labels, buttons.
    SemiBold,
    /// Noto Sans SemiCondensed: [`Family::Regular`] on a class that sets
    /// its text narrow (`widgets::tokens::narrow_text`). No [`Font`]
    /// asks for it; [`Font::sized`] resolves to it.
    ///
    /// [`Font`]: crate::text::Font
    /// [`Font::sized`]: crate::text::Font::sized
    RegularNarrow,
    /// Noto Sans SemiCondensed SemiBold: [`Family::SemiBold`] on the same
    /// class.
    SemiBoldNarrow,
    /// Noto Sans Mono: addresses, fingerprints, keys.
    Mono,
    /// The icon face: SeedSigner's icons and the Font Awesome glyphs the
    /// UI names ([`crate::widgets::Icon`]).
    Icon,
    /// Noto Sans CJK: every code point the wordlists and the kana, jamo
    /// and bopomofo keyboards need and the text faces lack. No [`Font`]
    /// asks for it; it is the fallback the text faces read.
    ///
    /// [`Font`]: crate::text::Font
    Cjk,
}

/// Every face text is drawn in, the narrow cuts included: what a check
/// that every character has a glyph walks.
pub const TEXT_FAMILIES: [Family; 5] = [
    Family::Regular,
    Family::SemiBold,
    Family::RegularNarrow,
    Family::SemiBoldNarrow,
    Family::Mono,
];

/// One face's outline file.
#[derive(Debug)]
pub struct Face {
    /// Family.
    pub family: Family,
    /// The outline bytes.
    pub data: &'static [u8],
}

const MAGIC: &[u8; 4] = b"OSKO";
const HEADER_LEN: usize = 20;
const RECORD_LEN: usize = 20;

/// Command tags in the outline stream.
const TAG_MOVE: u8 = b'M';
const TAG_LINE: u8 = b'L';
const TAG_QUAD: u8 = b'Q';
const TAG_CUBIC: u8 = b'C';
const TAG_CLOSE: u8 = b'Z';

/// A parsed face file. Cheap to construct: it only validates the header.
#[derive(Debug, Clone, Copy)]
pub struct Outlines {
    data: &'static [u8],
    count: usize,
    /// Which face's file this is.
    pub family: Family,
    /// Font units per em.
    pub units_per_em: u16,
    /// Font units above the baseline.
    pub ascent: i32,
    /// Font units below the baseline, negative as `hhea` reports it.
    pub descent: i32,
    /// Extra font units between lines.
    pub line_gap: i32,
}

/// One glyph's outline: metrics in font units and its command stream.
#[derive(Debug, Clone, Copy)]
pub struct Outline {
    /// Horizontal advance in font units.
    pub advance: u16,
    /// `x_min`, `y_min`, `x_max`, `y_max` in font units, y up.
    pub bbox: [i16; 4],
    /// The command stream (`tools/fontbake/README.md`).
    pub commands: &'static [u8],
}

fn u16_at(d: &[u8], i: usize) -> u16 {
    u16::from_le_bytes([d[i], d[i + 1]])
}

fn i16_at(d: &[u8], i: usize) -> i16 {
    i16::from_le_bytes([d[i], d[i + 1]])
}

fn u32_at(d: &[u8], i: usize) -> u32 {
    u32::from_le_bytes([d[i], d[i + 1], d[i + 2], d[i + 3]])
}

impl Outlines {
    /// Parses a face file. Returns `None` if the header is not a
    /// version-1 outline file or the file is too short for its glyph
    /// table.
    pub fn parse(family: Family, data: &'static [u8]) -> Option<Outlines> {
        if data.len() < HEADER_LEN || &data[0..4] != MAGIC || u16_at(data, 4) != 1 {
            return None;
        }
        let count = u32_at(data, 16) as usize;
        if data.len() < HEADER_LEN + count * RECORD_LEN {
            return None;
        }
        let units_per_em = u16_at(data, 8);
        if units_per_em == 0 {
            return None;
        }
        Some(Outlines {
            data,
            count,
            family,
            units_per_em,
            ascent: i32::from(i16_at(data, 10)),
            descent: i32::from(i16_at(data, 12)),
            line_gap: i32::from(i16_at(data, 14)),
        })
    }

    /// Number of glyphs.
    pub fn len(&self) -> usize {
        self.count
    }

    /// Whether the face has no glyphs.
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    fn record(&self, i: usize) -> &'static [u8] {
        let start = HEADER_LEN + i * RECORD_LEN;
        &self.data[start..start + RECORD_LEN]
    }

    /// Looks up a code point in this file alone. Binary search over the
    /// sorted table.
    pub fn glyph_in_file(&self, c: char) -> Option<Outline> {
        let target = c as u32;
        let (mut lo, mut hi) = (0usize, self.count);
        while lo < hi {
            let mid = (lo + hi) / 2;
            let cp = u32_at(self.record(mid), 0);
            if cp == target {
                return self.glyph_at(mid);
            } else if cp < target {
                lo = mid + 1;
            } else {
                hi = mid;
            }
        }
        None
    }

    fn glyph_at(&self, i: usize) -> Option<Outline> {
        let r = self.record(i);
        let outlines = HEADER_LEN + self.count * RECORD_LEN;
        let start = outlines + u32_at(r, 14) as usize;
        let len = usize::from(u16_at(r, 18));
        if start + len > self.data.len() {
            return None;
        }
        Some(Outline {
            advance: u16_at(r, 4),
            bbox: [i16_at(r, 6), i16_at(r, 8), i16_at(r, 10), i16_at(r, 12)],
            commands: &self.data[start..start + len],
        })
    }

    /// The outline for `c`: this file's own glyph, else the CJK
    /// fallback face's. `None` when neither has it.
    ///
    /// A text face carries the Latin glyph set; every code point the ten
    /// wordlists and the kana, jamo and bopomofo keyboards add lives once,
    /// in `cjk.outl`. The icon face and the fallback face itself do not
    /// fall back.
    pub fn glyph(&self, c: char) -> Option<Resolved> {
        if let Some(outline) = self.glyph_in_file(c) {
            return Some(Resolved {
                code_point: c,
                units_per_em: self.units_per_em,
                outline,
            });
        }
        if matches!(self.family, Family::Icon | Family::Cjk) {
            return None;
        }
        let cjk = outlines(Family::Cjk);
        cjk.glyph_in_file(c).map(|outline| Resolved {
            code_point: c,
            units_per_em: cjk.units_per_em,
            outline,
        })
    }

    /// The outline for `c`, or the fallback `?` when neither this face
    /// nor the CJK face has it. Only `None` if even `?` is missing,
    /// which the baker forbids.
    pub fn glyph_or_fallback(&self, c: char) -> Option<Resolved> {
        self.glyph(c).or_else(|| self.glyph('?'))
    }
}

/// A glyph found for a code point, and the coordinate system it is in.
///
/// A fallback glyph comes from `cjk.outl`, whose em is a different number
/// of font units than the requesting face's, so the units are carried with
/// the outline: a caller scales by its pixel size over `units_per_em` and
/// gets the right advance and bounding box at that size. The requesting
/// face's own ascent, descent and line height are untouched, so a CJK
/// glyph may reach a little past the line box.
#[derive(Debug, Clone, Copy)]
pub struct Resolved {
    /// The code point actually drawn: `c` itself, or `?`.
    pub code_point: char,
    /// Font units per em of the file the outline came from.
    pub units_per_em: u16,
    /// The outline, in that file's font units.
    pub outline: Outline,
}

/// The outline file of `family`.
pub fn outlines(family: Family) -> Outlines {
    let face = FACES
        .iter()
        .find(|f| f.family == family)
        .expect("every family has an outline file");
    Outlines::parse(family, face.data).expect("embedded outline files are valid")
}

/// A family at a whole pixel size: the outlines plus the metrics scaled
/// to that size. This is what a `Font` resolves to on a display; nothing
/// is rasterised until a glyph is drawn.
#[derive(Debug, Clone, Copy)]
pub struct SizedFace {
    /// The face's outlines.
    pub outlines: Outlines,
    /// Family.
    pub family: Family,
    /// Pixel size (the em size, rounded to a whole pixel).
    pub px: u16,
    /// Pixels above the baseline.
    pub ascent: i32,
    /// Pixels below the baseline (positive).
    pub descent: i32,
    /// Extra pixels between lines.
    pub line_gap: i32,
}

fn floor_i32(v: f32) -> i32 {
    let t = v as i32;
    if v < 0.0 && (t as f32) != v { t - 1 } else { t }
}

fn ceil_i32(v: f32) -> i32 {
    -floor_i32(-v)
}

fn round_i32(v: f32) -> i32 {
    floor_i32(v + 0.5)
}

impl SizedFace {
    /// The face of `family` at `px` pixels (at least one).
    pub fn new(family: Family, px: u16) -> SizedFace {
        let outlines = outlines(family);
        let px = px.max(1);
        let s = f32::from(px) / f32::from(outlines.units_per_em);
        SizedFace {
            outlines,
            family,
            px,
            // Rounded the way the previous baker rounded its scaled line
            // metrics, so a line box is the height it always was.
            ascent: ceil_i32(outlines.ascent as f32 * s),
            descent: ceil_i32(-outlines.descent as f32 * s),
            line_gap: round_i32(outlines.line_gap as f32 * s),
        }
    }

    /// Pixels per font unit.
    pub fn scale(&self) -> f32 {
        f32::from(self.px) / f32::from(self.outlines.units_per_em)
    }

    /// Ascent plus descent: the height of a line of text without gap.
    pub fn line_height(&self) -> i32 {
        self.ascent + self.descent
    }

    /// Horizontal advance of `c` in 1/64 px, falling back to the CJK
    /// face and then to `?`; zero when none of them has it.
    pub fn advance_64(&self, c: char) -> u32 {
        self.outlines
            .glyph_or_fallback(c)
            .map_or(0, |g| self.advance_64_of(&g))
    }

    /// Pixels per font unit for a glyph from a file with `upem` units
    /// per em: a fallback glyph is scaled by this face's pixel size over
    /// the file it came from.
    fn scale_of(&self, upem: u16) -> f32 {
        f32::from(self.px) / f32::from(upem)
    }

    fn advance_64_of(&self, g: &Resolved) -> u32 {
        round_i32(f32::from(g.outline.advance) * self.scale_of(g.units_per_em) * 64.0).max(0) as u32
    }
}

/// A glyph's coverage samples, one byte each, borrowed from the cache.
#[derive(Debug, Clone, Copy)]
pub struct Coverage<'a> {
    data: &'a [u8],
}

impl Coverage<'_> {
    /// Number of samples.
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Whether there are no samples.
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Coverage of sample `i` in `0..=255`; zero past the end.
    pub fn get(&self, i: usize) -> u8 {
        self.data.get(i).copied().unwrap_or(0)
    }
}

/// One rasterised glyph: its metrics and its coverage bitmap.
#[derive(Debug, Clone, Copy)]
pub struct Glyph<'a> {
    /// Horizontal advance in 1/64 px.
    pub advance_64: u32,
    /// Bitmap left edge relative to the pen position.
    pub bearing_x: i32,
    /// Bitmap top edge above the baseline.
    pub bearing_y: i32,
    /// Bitmap width in pixels.
    pub width: usize,
    /// Bitmap height in pixels.
    pub height: usize,
    /// Coverage, row-major, top row first, `width × height` samples.
    pub bitmap: Coverage<'a>,
}

/// How many bytes of rasterised glyphs are kept before the cache is
/// dropped whole. The working set of a screen is a few thousand glyph
/// bitmaps at one or two sizes; a quarter of a megabyte holds the text
/// ramp of the largest reference display several times over, and the
/// cost of overflowing it is one screen redrawn from the outlines. It is
/// a memory budget, not a screen dimension, so it is not a design token.
const CACHE_BUDGET: usize = 256 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Key {
    family: Family,
    px: u16,
    code_point: char,
}

#[derive(Debug)]
struct Entry {
    advance_64: u32,
    bearing_x: i32,
    bearing_y: i32,
    width: usize,
    height: usize,
    bitmap: Vec<u8>,
}

/// Glyph bitmaps filled from the outlines, kept for the next frame.
#[derive(Debug, Default)]
pub struct GlyphCache {
    entries: BTreeMap<Key, Entry>,
    bytes: usize,
}

impl GlyphCache {
    /// An empty cache.
    pub fn new() -> Self {
        GlyphCache::default()
    }

    /// Drops every cached bitmap.
    pub fn clear(&mut self) {
        self.entries.clear();
        self.bytes = 0;
    }

    /// The glyph for `c` in `face`, filling and caching it on a miss.
    /// Falls back to `?` as the outlines do.
    pub fn glyph(&mut self, face: &SizedFace, c: char) -> Option<Glyph<'_>> {
        let resolved = face.outlines.glyph_or_fallback(c)?;
        let key = Key {
            family: face.family,
            px: face.px,
            code_point: resolved.code_point,
        };
        if !self.entries.contains_key(&key) {
            let entry = rasterise(face, &resolved);
            if self.bytes + entry.bitmap.len() > CACHE_BUDGET {
                self.clear();
            }
            self.bytes += entry.bitmap.len();
            self.entries.insert(key, entry);
        }
        let e = self.entries.get(&key)?;
        Some(Glyph {
            advance_64: e.advance_64,
            bearing_x: e.bearing_x,
            bearing_y: e.bearing_y,
            width: e.width,
            height: e.height,
            bitmap: Coverage { data: &e.bitmap },
        })
    }
}

/// Fills one glyph's outline into a coverage bitmap.
///
/// tiny-skia is built with `default-features = false` and `no-std-float`,
/// so its whole pipeline is scalar (no SIMD path is compiled in) and the
/// coverage bytes are the same on every architecture: a frame on the Pi
/// and a frame in the desktop window are the same pixels.
fn rasterise(face: &SizedFace, resolved: &Resolved) -> Entry {
    let outline = &resolved.outline;
    let s = face.scale_of(resolved.units_per_em);
    let advance_64 = face.advance_64_of(resolved);
    let [x_min, y_min, x_max, y_max] = outline.bbox;
    let left = floor_i32(f32::from(x_min) * s);
    let right = ceil_i32(f32::from(x_max) * s);
    let bottom = floor_i32(f32::from(y_min) * s);
    let top = ceil_i32(f32::from(y_max) * s);
    let width = (right - left).max(0) as usize;
    let height = (top - bottom).max(0) as usize;
    let blank = Entry {
        advance_64,
        bearing_x: left,
        bearing_y: top,
        width: 0,
        height: 0,
        bitmap: Vec::new(),
    };
    if width == 0 || height == 0 || outline.commands.is_empty() {
        return blank;
    }

    // Font units are y up with the origin at the pen on the baseline;
    // the bitmap is y down with its origin at whole pixels, so that a
    // glyph drawn at an integer pen position needs no resampling.
    let px = |x: i16| f32::from(x) * s - left as f32;
    let py = |y: i16| top as f32 - f32::from(y) * s;
    let mut pb = PathBuilder::new();
    let cmds = outline.commands;
    let mut i = 0usize;
    let coord = |i: usize| i16::from_le_bytes([cmds[i], cmds[i + 1]]);
    while i < cmds.len() {
        let tag = cmds[i];
        i += 1;
        let args = match tag {
            TAG_MOVE | TAG_LINE => 2,
            TAG_QUAD => 4,
            TAG_CUBIC => 6,
            TAG_CLOSE => 0,
            _ => return blank,
        };
        if i + args * 2 > cmds.len() {
            return blank;
        }
        match tag {
            TAG_MOVE => pb.move_to(px(coord(i)), py(coord(i + 2))),
            TAG_LINE => pb.line_to(px(coord(i)), py(coord(i + 2))),
            TAG_QUAD => pb.quad_to(
                px(coord(i)),
                py(coord(i + 2)),
                px(coord(i + 4)),
                py(coord(i + 6)),
            ),
            TAG_CUBIC => pb.cubic_to(
                px(coord(i)),
                py(coord(i + 2)),
                px(coord(i + 4)),
                py(coord(i + 6)),
                px(coord(i + 8)),
                py(coord(i + 10)),
            ),
            _ => pb.close(),
        }
        i += args * 2;
    }
    let Some(path) = pb.finish() else {
        return blank;
    };
    let Some(mut mask) = Mask::new(width as u32, height as u32) else {
        return blank;
    };
    mask.fill_path(&path, FillRule::Winding, true, Transform::identity());
    Entry {
        advance_64,
        bearing_x: left,
        bearing_y: top,
        width,
        height,
        bitmap: mask.data().to_vec(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text_faces() -> impl Iterator<Item = &'static Face> {
        FACES
            .iter()
            .filter(|f| !matches!(f.family, Family::Icon | Family::Cjk))
    }

    fn text_set() -> Vec<char> {
        let mut set: Vec<char> = (0x20u32..=0x7E)
            .chain(0xA0..=0xFF)
            .filter(|&c| c != 0xAD)
            .filter_map(char::from_u32)
            .collect();
        set.extend_from_slice(&[
            '•', '▾', '▸', '←', '→', '↑', '↓', '✓', '✗', '×', '…', '°', '₿',
        ]);
        set
    }

    #[test]
    fn every_face_parses_and_has_its_glyph_set() {
        for face in text_faces() {
            let o = Outlines::parse(face.family, face.data).expect("parse");
            assert!(o.ascent > 0 && o.descent < 0);
            for c in text_set() {
                let g = o.glyph(c).unwrap_or_else(|| panic!("no glyph for {c:?}"));
                assert!(g.outline.advance > 0, "{c:?} has no advance");
                assert!(
                    c == ' ' || c == '\u{a0}' || !g.outline.commands.is_empty(),
                    "{c:?} has no outline"
                );
            }
            assert!(o.glyph('?').is_some());
            // A hanzi is not in a text face's own file; it is drawn from
            // the CJK face, in that file's font units.
            assert!(o.glyph_in_file('\u{4e2d}').is_none());
            let hanzi = o.glyph('\u{4e2d}').expect("hanzi from the CJK face");
            assert_eq!(hanzi.code_point, '\u{4e2d}');
            assert!(!hanzi.outline.commands.is_empty());
            // A code point no face carries still falls back to `?`.
            let snowman = o.glyph_or_fallback('\u{2603}').expect("? fallback");
            assert_eq!(snowman.code_point, '?');
        }
        let icons = Outlines::parse(
            Family::Icon,
            FACES
                .iter()
                .find(|f| f.family == Family::Icon)
                .unwrap()
                .data,
        )
        .expect("parse");
        assert_eq!(icons.ascent, i32::from(icons.units_per_em));
        let scan = icons.glyph('\u{e900}').expect("scan icon");
        assert!(scan.outline.advance > 0 && !scan.outline.commands.is_empty());
    }

    #[test]
    fn filled_glyphs_have_ink_where_expected() {
        // A capital I in a sans face is a vertical bar: every row of the
        // bitmap has coverage.
        let face = SizedFace::new(Family::Regular, 48);
        let mut cache = GlyphCache::new();
        let g = cache.glyph(&face, 'I').unwrap();
        assert!(g.height > g.width);
        for row in 0..g.height {
            assert!(
                (0..g.width).any(|x| g.bitmap.get(row * g.width + x) > 0),
                "row {row} of I is blank"
            );
        }
        assert!(g.bearing_y > 0 && g.bearing_y <= face.ascent);
        // A descender: `g` reaches below the baseline.
        let lower_g = cache.glyph(&face, 'g').unwrap();
        assert!(lower_g.bearing_y < lower_g.height as i32);
    }

    /// Every whole pixel size draws, not a chosen few: a run of sizes
    /// each gives a glyph one pixel taller than the last, with ink in it.
    #[test]
    fn a_glyph_is_filled_at_whatever_size_is_asked_for() {
        let mut cache = GlyphCache::new();
        let mut last = 0usize;
        for px in 11u16..40 {
            let face = SizedFace::new(Family::Mono, px);
            let g = cache.glyph(&face, '8').unwrap();
            assert!(g.height >= last, "{px} px is shorter than {} px", px - 1);
            assert!(
                (0..g.bitmap.len()).any(|i| g.bitmap.get(i) > 0),
                "{px} px of '8' is blank"
            );
            last = g.height;
        }
    }
}
