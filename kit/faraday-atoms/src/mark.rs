//! Faraday's mark, the keyhole shield (`docs/DESIGN.md` §2.4, F10, F137,
//! F140): its three outlines in the mark's own units, worked out once and
//! kept, so that drawing it only scales them.
//!
//! The shield is `WIDTH` by `HEIGHT`. Its sides run straight to the
//! height over the golden ratio, then curve to the tip; its top corners
//! are rounded at the band times the golden ratio. The band, the lighter
//! rim, is `BAND` wide everywhere: the inner shield is the outer one
//! inset by it. The keyhole's head is as wide as its foot, the inner
//! width over the golden ratio squared; the slot runs from the head's
//! centre to the foot the head's width times the golden ratio; its neck
//! keeps the proportion it had to the head; and its middle, between the
//! head's top and the foot, stays where F137 put it.

use faraday_draw::canvas::ShapeKey;
use faraday_tokens::palette::{Color, Paint, Role};
use std::sync::OnceLock;

use crate::Pressed;
use crate::ui::Ui;

/// The shield's width, in the mark's units.
pub(crate) const WIDTH: f32 = 100.0;
/// The shield's height, in the mark's units.
pub(crate) const HEIGHT: f32 = 135.3;
/// The band's width, in the mark's units.
const BAND: f32 = 7.0;
/// The golden ratio.
const PHI: f32 = 1.618_034;
/// The neck's width over the head's (9 to 30 as F10 drew it).
const NECK: f32 = 0.3;
/// The keyhole's middle, between the head's top and the foot, in the
/// mark's units from the top (F137's).
const KEY_MIDDLE: f32 = 56.776;
/// How far down from a side's foot to the tip the curve's control point
/// sits, as a share of that drop; it sits `CURVE_IN` inside the side.
const CURVE_DOWN: f32 = 0.625;
/// How far inside the side the curve's control point sits.
const CURVE_IN: f32 = 2.0;
/// Straight pieces per quarter of a rounded corner.
const CORNER_PIECES: usize = 8;
/// Straight pieces per curve from a side to the tip.
const CURVE_PIECES: usize = 16;
/// Straight pieces of the keyhole's head.
const HEAD_PIECES: usize = 48;

/// The mark's outlines in its own units: the outer shield and the inner
/// one clockwise, the keyhole wound against them so that it is a hole.
pub(crate) struct Outlines {
    pub outer: Vec<(f32, f32)>,
    pub inner: Vec<(f32, f32)>,
    pub hole: Vec<(f32, f32)>,
}

/// The mark's outlines, worked out on the first call and kept.
pub(crate) fn outlines() -> &'static Outlines {
    static OUTLINES: OnceLock<Outlines> = OnceLock::new();
    OUTLINES.get_or_init(|| {
        let outer = outer();
        let inner = inset(&outer, BAND);
        Outlines {
            outer,
            inner,
            hole: hole(),
        }
    })
}

/// The outer shield, clockwise from the top of the left side.
fn outer() -> Vec<(f32, f32)> {
    use std::f32::consts::{FRAC_PI_2, PI};
    let r = BAND * PHI;
    let side = HEIGHT / PHI;
    let mut out = Vec::new();
    for (cx, from) in [(r, PI), (WIDTH - r, PI + FRAC_PI_2)] {
        out.extend((0..=CORNER_PIECES).map(|i| {
            let a = from + FRAC_PI_2 * i as f32 / CORNER_PIECES as f32;
            (cx + r * a.cos(), r + r * a.sin())
        }));
    }
    let tip = (WIDTH / 2.0, HEIGHT);
    let ctrl_y = side + CURVE_DOWN * (HEIGHT - side);
    // A quadratic curve from the foot of a side to the tip.
    let curve = |sx: f32, cx: f32| {
        (0..=CURVE_PIECES).map(move |i| {
            let t = i as f32 / CURVE_PIECES as f32;
            let u = 1.0 - t;
            (
                u * u * sx + 2.0 * u * t * cx + t * t * tip.0,
                u * u * side + 2.0 * u * t * ctrl_y + t * t * tip.1,
            )
        })
    };
    out.extend(curve(WIDTH, WIDTH - CURVE_IN));
    let left: Vec<(f32, f32)> = curve(0.0, CURVE_IN).collect();
    out.extend(left.into_iter().rev().skip(1));
    out
}

/// A clockwise convex outline moved `by` inward everywhere: the part of
/// the plane inside every edge's line moved in by `by`.
fn inset(outline: &[(f32, f32)], by: f32) -> Vec<(f32, f32)> {
    let far = WIDTH + HEIGHT;
    let mut poly = vec![(-far, -far), (far, -far), (far, far), (-far, far)];
    for (i, &a) in outline.iter().enumerate() {
        let b = outline[(i + 1) % outline.len()];
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        let len = dx.hypot(dy);
        if len <= f32::EPSILON {
            continue;
        }
        // Inward, for a clockwise outline with y down.
        let (nx, ny) = (-dy / len, dx / len);
        let at = (a.0 + nx * by, a.1 + ny * by);
        let side = |p: (f32, f32)| (p.0 - at.0) * nx + (p.1 - at.1) * ny;
        let mut kept = Vec::with_capacity(poly.len() + 1);
        for j in 0..poly.len() {
            let (p, q) = (poly[(j + poly.len() - 1) % poly.len()], poly[j]);
            let (sp, sq) = (side(p), side(q));
            let cross = || {
                let t = sp / (sp - sq);
                (p.0 + t * (q.0 - p.0), p.1 + t * (q.1 - p.1))
            };
            if sq >= 0.0 {
                if sp < 0.0 {
                    kept.push(cross());
                }
                kept.push(q);
            } else if sp >= 0.0 {
                kept.push(cross());
            }
        }
        poly = kept;
    }
    poly
}

/// The keyhole, counter-clockwise: the round head and the slot widening
/// below it to the foot.
fn hole() -> Vec<(f32, f32)> {
    let head = (WIDTH - 2.0 * BAND) / (PHI * PHI);
    let r = head / 2.0;
    let slot = head * PHI;
    let waist = NECK * r;
    let cx = WIDTH / 2.0;
    let cy = KEY_MIDDLE - (r + slot) / 2.0 + r;
    let join = (r * r - waist * waist).sqrt();
    let from = join.atan2(-waist);
    let to = join.atan2(waist) + std::f32::consts::TAU;
    let mut hole: Vec<(f32, f32)> = (0..=HEAD_PIECES)
        .map(|i| {
            let a = from + (to - from) * i as f32 / HEAD_PIECES as f32;
            (cx + r * a.cos(), cy + r * a.sin())
        })
        .collect();
    hole.extend([(cx + r, cy + slot), (cx - r, cy + slot)]);
    hole.reverse();
    hole
}

/// Faraday's name as outlines: SVG path data in font units.
const WORDMARK: &str = include_str!("../assets/wordmark.txt");

/// The letters of the word after the wordmark: `assets/wordmark-suffix.txt`.
const WORDMARK_SUFFIX: &str = include_str!("../assets/wordmark-suffix.txt");

/// Font units from the wordmark's right edge to where the suffix's first
/// letter starts (its pen, before the letter's own left bearing).
const SUFFIX_GAP: f32 = 364.0;

/// The name's width with its capitals `cap` units tall, as
/// [`Ui::wordmark_named`] draws it with `suffix`. Arithmetic on outlines
/// read once.
pub(crate) fn wordmark_width(cap: f32, suffix: NameSuffix) -> f32 {
    let mark = wordmark_outlines();
    let right = match suffix_word(suffix) {
        Some(word) => mark.right + SUFFIX_GAP + word.right,
        None => mark.right,
    };
    (right - mark.left) * cap / WORDMARK_CAP
}

/// The word drawn after the wordmark in an app's name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NameSuffix {
    /// FARADAY alone.
    None,
    /// Faraday Starter.
    Starter,
    /// Faraday Demo.
    Demo,
    /// Faraday Offline: the stick and the Pi.
    Offline,
}

/// The wordmark's cap height in its font units.
const WORDMARK_CAP: f32 = 687.0;

/// The shapes a canvas keeps filled for the name and the mark
/// ([`faraday_draw::canvas::ShapeKey`]): FARADAY; the word after it, and
/// the word under it, each plus the suffix's own number; the mark's lit
/// half and its body.
const SHAPE_WORDMARK: u32 = 1;
const SHAPE_SUFFIX: u32 = 0x10;
const SHAPE_SUFFIX_UNDER: u32 = 0x20;
const SHAPE_MARK_LIT: u32 = 0x30;
const SHAPE_MARK_BODY: u32 = 0x31;

/// The wordmark's outlines, each curve as straight pieces, and the left
/// and right edges of them all, in font units.
struct Wordmark {
    outlines: Vec<Vec<(f32, f32)>>,
    left: f32,
    right: f32,
}

fn wordmark_outlines() -> &'static Wordmark {
    static OUTLINES: OnceLock<Wordmark> = OnceLock::new();
    OUTLINES.get_or_init(|| {
        let outlines = path_outlines(WORDMARK);
        let xs = || outlines.iter().flatten().map(|p| p.0);
        let left = xs().fold(f32::MAX, f32::min);
        let right = xs().fold(f32::MIN, f32::max);
        Wordmark {
            outlines,
            left,
            right,
        }
    })
}

/// Closed outlines in font units.
type WordOutlines = Vec<Vec<(f32, f32)>>;

/// One suffix's outlines in font units from the pen's start, and the
/// right edge of the last letter.
struct SuffixWord {
    outlines: Vec<Vec<(f32, f32)>>,
    right: f32,
}

/// The suffix `which` spelled in the letters of `assets/wordmark-suffix.txt`.
fn suffix_word(which: NameSuffix) -> Option<&'static SuffixWord> {
    static WORDS: OnceLock<[SuffixWord; 3]> = OnceLock::new();
    let key = match which {
        NameSuffix::None => return None,
        NameSuffix::Starter => 0,
        NameSuffix::Demo => 1,
        NameSuffix::Offline => 2,
    };
    let words = WORDS.get_or_init(|| {
        let mut letters: Vec<(char, f32, WordOutlines)> = Vec::new();
        let mut spelled: Vec<&str> = Vec::new();
        for line in WORDMARK_SUFFIX.lines().filter(|l| !l.starts_with('#')) {
            let parts: Vec<&str> = line.split('\t').collect();
            match parts.as_slice() {
                [name, word] if name.starts_with('@') => spelled.push(word),
                [letter, advance, path] => letters.push((
                    letter.chars().next().unwrap_or(' '),
                    advance.parse().unwrap_or(0.0),
                    path_outlines(path),
                )),
                _ => {}
            }
        }
        let build = |word: &str| {
            let (mut pen, mut right) = (0.0_f32, 0.0_f32);
            let mut outlines = Vec::new();
            for c in word.chars() {
                if let Some((_, advance, shapes)) = letters.iter().find(|l| l.0 == c) {
                    for o in shapes {
                        let o: Vec<(f32, f32)> = o.iter().map(|&(x, y)| (x + pen, y)).collect();
                        right = o.iter().fold(right, |r, p| r.max(p.0));
                        outlines.push(o);
                    }
                    pen += advance;
                }
            }
            SuffixWord { outlines, right }
        };
        [
            build(spelled.first().copied().unwrap_or("")),
            build(spelled.get(1).copied().unwrap_or("")),
            build(spelled.get(2).copied().unwrap_or("")),
        ]
    });
    words.get(key)
}

/// The closed outlines of SVG path data made of absolute M, L, Q and Z,
/// with `#` lines skipped; each quadratic curve becomes straight pieces.
fn path_outlines(src: &str) -> Vec<Vec<(f32, f32)>> {
    const PIECES: usize = 6;
    let mut spaced = String::new();
    for line in src.lines().filter(|l| !l.starts_with('#')) {
        for c in line.chars() {
            if c.is_ascii_alphabetic() {
                spaced.extend([' ', c, ' ']);
            } else {
                spaced.push(c);
            }
        }
        spaced.push(' ');
    }
    let words: Vec<&str> = spaced.split_whitespace().collect();
    let num = |i: usize| words.get(i).and_then(|w| w.parse().ok()).unwrap_or(0.0);
    let mut out: Vec<Vec<(f32, f32)>> = Vec::new();
    let mut at = (0.0, 0.0);
    let mut i = 0;
    while i < words.len() {
        match words[i] {
            "M" => {
                at = (num(i + 1), num(i + 2));
                out.push(vec![at]);
                i += 3;
            }
            "L" => {
                at = (num(i + 1), num(i + 2));
                if let Some(o) = out.last_mut() {
                    o.push(at);
                }
                i += 3;
            }
            "Q" => {
                let (c, to) = ((num(i + 1), num(i + 2)), (num(i + 3), num(i + 4)));
                if let Some(o) = out.last_mut() {
                    o.extend((1..=PIECES).map(|k| {
                        let t = k as f32 / PIECES as f32;
                        let u = 1.0 - t;
                        (
                            u * u * at.0 + 2.0 * u * t * c.0 + t * t * to.0,
                            u * u * at.1 + 2.0 * u * t * c.1 + t * t * to.1,
                        )
                    }));
                }
                at = to;
                i += 5;
            }
            _ => i += 1,
        }
    }
    out
}

/// Faraday's mark (04 §12.1): drawn from the theme alone.
pub struct MarkModel;

/// Draws the mark fitted and centred in the box (x, y, w, h), design units.
pub fn draw<A: Pressed>(_m: &MarkModel, ui: &mut Ui<'_, A>, (x, y, w, h): (f32, f32, f32, f32)) {
    ui.mark(x, y, w, h);
}

impl<A: Pressed> Ui<'_, A> {
    /// Costs filling its outlines the first time it is drawn at a place
    /// and size, then a blend of what the canvas kept
    /// ([`Canvas::fill_kept`]). Faraday's mark in a box `w` by `h` units
    /// at (x, y): the keyhole shield (`mark`), fitted to the box and
    /// centred, the shield in primary with its band lit half with white
    /// and the keyhole cut through.
    pub fn mark(&mut self, x: f32, y: f32, w: f32, h: f32) {
        let m = outlines();
        let area = self.rect(x, y, w, h);
        let s = (area.w as f32 / WIDTH).min(area.h as f32 / HEIGHT);
        let ox = area.x as f32 + (area.w as f32 - s * WIDTH) / 2.0;
        let oy = area.y as f32 + (area.h as f32 - s * HEIGHT) / 2.0;
        let place = move |o: &[(f32, f32)]| -> Vec<(f32, f32)> {
            o.iter()
                .map(|&(px, py)| (ox + px * s, oy + py * s))
                .collect()
        };
        let stroke = self.col(Role::Primary);
        let lit = stroke.mix(Color::rgb(0xff, 0xff, 0xff), 0.5);
        let key = |shape| ShapeKey::new(shape, ox, oy, s);
        self.c.fill_kept(
            key(SHAPE_MARK_LIT),
            || vec![place(&m.outer), place(&m.hole)],
            lit,
        );
        self.c.fill_kept(
            key(SHAPE_MARK_BODY),
            || vec![place(&m.inner), place(&m.hole)],
            stroke,
        );
    }

    /// Costs as [`Ui::wordmark_named`]. Faraday's name, the wordmark of
    /// `assets/wordmark.txt`, its capitals `cap` units tall with their
    /// top-left at (x, y). Returns its width.
    pub fn wordmark(&mut self, x: f32, y: f32, cap: f32, color: impl Into<Paint>) -> f32 {
        let color: Paint = color.into();
        self.wordmark_named(x, y, cap, color, NameSuffix::None)
    }

    /// Costs filling its outlines the first time it is drawn at a place
    /// and size, then a blend of what the canvas kept
    /// ([`Canvas::fill_kept`]). The wordmark with the word of an app's
    /// name after it
    /// ("Faraday Starter", "Faraday Demo", "Faraday Offline"), as one name
    /// in one lettering.
    /// The suffix is in Archivo Expanded Regular, in the grey of
    /// secondary text (`docs/DECISIONS.md` F16).
    pub fn wordmark_named(
        &mut self,
        x: f32,
        y: f32,
        cap: f32,
        color: impl Into<Paint>,
        suffix: NameSuffix,
    ) -> f32 {
        let color: Paint = color.into();
        let mark = wordmark_outlines();
        let s = cap / WORDMARK_CAP;
        let k = s * self.f;
        let (ox, oy) = (
            (x + self.ox) * self.f,
            (y + self.oy) * self.f + self.oy_px as f32,
        );
        let place = |os: &[Vec<(f32, f32)>], dx: f32| -> Vec<Vec<(f32, f32)>> {
            os.iter()
                .map(|o| {
                    o.iter()
                        .map(|&(fx, fy)| {
                            (ox + (fx + dx - mark.left) * k, oy + (fy + WORDMARK_CAP) * k)
                        })
                        .collect()
                })
                .collect()
        };
        let fill = self.col(color);
        self.c.fill_kept(
            ShapeKey::new(SHAPE_WORDMARK, ox, oy, k),
            || place(&mark.outlines, 0.0),
            fill,
        );
        let mut right = mark.right;
        if let Some(word) = suffix_word(suffix) {
            let start = mark.right + SUFFIX_GAP;
            let grey = self.col(Role::OnSurfaceVariant);
            self.c.fill_kept(
                ShapeKey::new(SHAPE_SUFFIX + suffix as u32, ox, oy, k),
                || place(&word.outlines, start),
                grey,
            );
            right = start + word.right;
        }
        let w = (right - mark.left) * s;
        self.note_name(x, y, w, cap);
        w
    }

    /// Notes the name drawn in the box `w` by `h` units at (x, y) as one
    /// thing read, while the canvas records what it draws.
    fn note_name(&mut self, x: f32, y: f32, w: f32, h: f32) {
        let r = self.rect(x, y, w, h);
        self.c.note(faraday_draw::canvas::InkKind::Wordmark, r);
    }

    /// Costs as [`Ui::wordmark_named`], for each of its two lines. The
    /// name with the word of the app under FARADAY in place of after
    /// it, for a bar too narrow for the two side by side: FARADAY's
    /// capitals `cap` units tall with their top-left at (x, y), the word's
    /// `WORDMARK_STACK_WORD` of that, `WORDMARK_STACK_GAP` of it under
    /// FARADAY, both starting at the same left edge. Returns its width and
    /// height.
    pub fn wordmark_stacked(
        &mut self,
        x: f32,
        y: f32,
        cap: f32,
        color: impl Into<Paint>,
        suffix: NameSuffix,
    ) -> (f32, f32) {
        let color: Paint = color.into();
        let mark_w = self.wordmark_named(x, y, cap, color, NameSuffix::None);
        let Some(word) = suffix_word(suffix) else {
            return (mark_w, cap);
        };
        let wc = cap * faraday_tokens::dims::WORDMARK_STACK_WORD;
        let wy = y + cap * (1.0 + faraday_tokens::dims::WORDMARK_STACK_GAP);
        let s = wc / WORDMARK_CAP;
        let k = s * self.f;
        let left = word
            .outlines
            .iter()
            .flatten()
            .map(|p| p.0)
            .fold(f32::MAX, f32::min);
        let (ox, oy) = (
            (x + self.ox) * self.f,
            (wy + self.oy) * self.f + self.oy_px as f32,
        );
        let outlines = || {
            word.outlines
                .iter()
                .map(|o| {
                    o.iter()
                        .map(|&(fx, fy)| (ox + (fx - left) * k, oy + (fy + WORDMARK_CAP) * k))
                        .collect()
                })
                .collect()
        };
        let grey = self.col(Role::OnSurfaceVariant);
        self.c.fill_kept(
            ShapeKey::new(SHAPE_SUFFIX_UNDER + suffix as u32, ox, oy, k),
            outlines,
            grey,
        );
        let word_w = (word.right - left) * s;
        self.note_name(x, wy, word_w, wc);
        (mark_w.max(word_w), wy + wc - y)
    }

    /// Costs measuring the name, and drawing it as
    /// [`Ui::wordmark_named`] does. An app's name
    /// starting at `x` and ending within `room` units of it, its capitals
    /// `cap` tall on the line whose top is `y`: beside its word where the
    /// two fit; else, for an app with a word, the word under FARADAY
    /// ([`Ui::wordmark_stacked`]), centred on the same line and made
    /// smaller where FARADAY needs it, down to
    /// [`faraday_tokens::dims::WORDMARK_CAP_MIN`]; else nothing. Returns
    /// whether it was drawn.
    pub fn wordmark_fitted(
        &mut self,
        (x, y, room): (f32, f32, f32),
        cap: f32,
        color: impl Into<Paint>,
        suffix: NameSuffix,
    ) -> bool {
        let color: Paint = color.into();
        if wordmark_width(cap, suffix) <= room {
            self.wordmark_named(x, y, cap, color, suffix);
            return true;
        }
        if suffix == NameSuffix::None {
            return false;
        }
        let small = cap * (room / wordmark_width(cap, NameSuffix::None)).min(1.0);
        if small < faraday_tokens::dims::WORDMARK_CAP_MIN {
            return false;
        }
        let h = small
            * (1.0
                + faraday_tokens::dims::WORDMARK_STACK_GAP
                + faraday_tokens::dims::WORDMARK_STACK_WORD);
        self.wordmark_stacked(x, y + (cap - h) / 2.0, small, color, suffix);
        true
    }
}
