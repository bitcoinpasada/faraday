//! The booklet's dice de-biasing worksheet, printed (`docs/CODEX32.md`
//! §11): its two landscape pages as the booklet's PostScript of
//! 2023-03-07 draws them (`drawDiceRow` and the tree of its page 18), in
//! the booklet's own points. Page one: the 32-leaf tree, the "Die
//! tracks" head with its rule, "Die pads" and "Die pads (d7+)", and one
//! track; page two: four tracks more. Each track is the free space,
//! pads 1 to 6 as diamonds and pads 7 to 20 as hexagons. The booklet's
//! instructions are not printed; they are the codex32 Learn page's.
//!
//! The booklet is MIT, "Copyright (c) 2022 Blockstream": its notice is
//! kept in the repository (`tools/vectors/codex32-booklet/LICENSE`) and
//! on the codex32 Learn page; these two pages also carry one small
//! credit line at their foot (owner, 2026-10-24, amending §1.1 item 6),
//! and no other sheet does.

use super::{ALPHA, Canvas, L_H};
use faraday_strings::{EN, fill};

/// The booklet's pad size and the gap between pads (`sz`, `gapsz`).
const SZ: f32 = 54.0;
const GAP: f32 = 2.0;
/// The left edge of the tracks, the booklet's margin.
const X0: f32 = 36.0;
/// The tree's origin on the page, up from its foot (the booklet's
/// `325 190 translate`), its leaves' spacing radius and its levels.
const TREE_X: f32 = 325.0;
const TREE_Y: f32 = 190.0;
const RADIUS: f32 = 0.705 * 72.0 / 8.5;
const SPACING: f32 = 60.0;
/// A pad's outline and its grey (the booklet's `0.95 setgray`).
const LINE: f32 = 0.4;
const PAD_GREY: f32 = 0.95;
/// Helvetica's digit width, in ems: numbers are centred by it.
const DIGIT_EM: f32 = 0.556;
/// Where the first track of page two stands, and the step to the next,
/// from the page's top (the booklet's 132 pt, a little closer so the
/// page's title fits above).
const PAGE2_FIRST: f32 = 133.0;
const PAGE2_STEP: f32 = 128.0;

/// The two pages.
pub(super) fn pages() -> Vec<Canvas> {
    let mut one = Canvas::landscape();
    title(&mut one, 1);
    tree(&mut one);
    // "Die tracks" at 200 up, its rule 5 under it over three quarters of
    // the page's width, the pads' heads 23 under it, and the track 105
    // under it (the booklet's page 18).
    let head = L_H - 200.0;
    one.sans(X0, head - 18.0, 18.0, true, EN.codex32_die_tracks);
    one.line(X0, head + 5.0, X0 + 0.75 * 770.0, head + 5.0, 1.0, 0.0);
    one.sans(
        X0 + 160.0,
        head + 23.0 - 18.0,
        18.0,
        true,
        EN.codex32_die_pads,
    );
    one.sans(
        X0 + 440.0,
        head + 23.0 - 18.0,
        18.0,
        true,
        EN.codex32_die_pads_large,
    );
    track(&mut one, head + 105.0);
    one.footer();
    credit(&mut one);
    let mut two = Canvas::landscape();
    title(&mut two, 2);
    for k in 0..4 {
        track(&mut two, PAGE2_FIRST + k as f32 * PAGE2_STEP);
    }
    two.footer();
    credit(&mut two);
    vec![one, two]
}

/// The booklet's credit, small at the page's foot beside the footer
/// (owner, 2026-10-24): printed on the dice worksheet's own pages only,
/// which never share a page with a share card or a grid.
fn credit(p: &mut Canvas) {
    p.sans(
        super::L_W / 2.0 - super::L_MARGIN,
        L_H - 22.0,
        8.0,
        false,
        EN.pdf_c32_dice_credit,
    );
}

/// The page's title.
fn title(p: &mut Canvas, page: usize) {
    let s = fill(EN.pdf_c32_dice, &[&page.to_string(), "2"]);
    p.sans(super::L_MARGIN, super::L_MARGIN, 18.0, true, &s);
}

/// A closed shape through `pts` (from the page's top), filled grey and
/// outlined.
fn poly(p: &mut Canvas, pts: &[(f32, f32)]) {
    let mut op = format!("{PAD_GREY:.2} g 0 G {LINE:.2} w ");
    for (i, &(x, y)) in pts.iter().enumerate() {
        let y = p.h - y;
        op.push_str(&format!(
            "{x:.2} {y:.2} {} ",
            if i == 0 { "m" } else { "l" }
        ));
    }
    op.push_str("h B");
    p.op(&op);
}

/// A circle's outline, centred at (`cx`, `cy`) from the page's top.
fn circle(p: &mut Canvas, (cx, cy): (f32, f32), r: f32, width: f32) {
    // Four Bézier quarters.
    let k = 0.5523 * r;
    let cy = p.h - cy;
    p.op(&format!(
        "0 G {width:.2} w {:.2} {cy:.2} m \
         {:.2} {:.2} {:.2} {:.2} {cx:.2} {:.2} c \
         {:.2} {:.2} {:.2} {:.2} {:.2} {cy:.2} c \
         {:.2} {:.2} {:.2} {:.2} {cx:.2} {:.2} c \
         {:.2} {:.2} {:.2} {:.2} {:.2} {cy:.2} c S",
        cx + r,
        cx + r,
        cy + k,
        cx + k,
        cy + r,
        cy + r,
        cx - k,
        cy + r,
        cx - r,
        cy + k,
        cx - r,
        cx - r,
        cy - k,
        cx - k,
        cy - r,
        cy - r,
        cx + k,
        cy - r,
        cx + r,
        cy - k,
        cx + r,
    ));
}

/// A pad's number, centred across `cx`, its foot 5 under `cy` (the
/// booklet's Helvetica-Bold 14).
fn number(p: &mut Canvas, cx: f32, cy: f32, n: usize) {
    let s = n.to_string();
    let size = 14.0;
    let w = s.len() as f32 * DIGIT_EM * size;
    p.sans(cx - w / 2.0, cy + 5.0 - size, size, true, &s);
}

/// One die track, its pads' centre line `base` from the page's top: the
/// free space (a circle with its name), pads 1 to 6 as diamonds, every
/// other one raised, and pads 7 to 20 as hexagons, every other one
/// raised, each numbered. The booklet's `drawDiceRow`.
fn track(p: &mut Canvas, base: f32) {
    let at = |bx: f32, by: f32| (X0 + bx, base - by);
    circle(p, at(36.0, 44.0), 30.0, LINE * 2.0);
    let name = EN.codex32_free_space;
    let size = 9.0;
    let (cx, cy) = at(36.0, 40.0);
    let w = name.chars().count() as f32 * 0.5 * size;
    p.sans(cx - w / 2.0, cy - size / 2.0, size, false, name);
    let s2 = SZ / 2f32.sqrt();
    for idx in 1..=6usize {
        let lx = (s2 + GAP) * idx as f32;
        let by = if idx % 2 == 0 { s2 + GAP } else { 0.0 };
        let pts = [
            at(lx, by),
            at(lx + s2, by + s2),
            at(lx + 2.0 * s2, by),
            at(lx + s2, by - s2),
        ];
        poly(p, &pts);
        let (cx, cy) = at(lx + s2, by);
        number(p, cx, cy, idx);
    }
    let sz3 = SZ * (3.0f32 / 8.0).sqrt();
    let (ox0, oy0) = (s2 * 2.5, -sz3 + 15.0);
    let (h, step) = (s2 * 0.8, sz3 * 0.8 + GAP);
    let hw = h * 3f32.sqrt();
    for idx in 7..=20usize {
        let ox = step * idx as f32 + ox0;
        let oy = oy0 + if idx % 2 == 0 { h * 1.5 + GAP } else { 0.0 };
        let pts = [
            at(ox, oy),
            at(ox, oy + h),
            at(ox + hw / 2.0, oy + 1.5 * h),
            at(ox + hw, oy + h),
            at(ox + hw, oy),
            at(ox + hw / 2.0, oy - h / 2.0),
        ];
        poly(p, &pts);
        let (cx, cy) = at(ox + hw / 2.0, oy + h / 2.0);
        number(p, cx, cy, idx);
    }
}

/// The tree as the booklet's page 18 draws it: 31 nodes, small circles,
/// over 32 leaves, the leaves in the bech32 alphabet's value order (the
/// booklet's `code` array), level `i` up from the leaves at
/// `(i + 0.5) × 60`.
fn tree(p: &mut Canvas) {
    // Node `j` of level `i` (0 the leaves, 5 the root), from the page's top.
    let coord = |i: u32, j: usize| {
        let x = 2f32.powi(i as i32) * (j as f32 + 0.5) * 2.0 * 1.1 * RADIUS;
        let y = (i as f32 + 0.5) * SPACING;
        (TREE_X + x, L_H - (TREE_Y + y))
    };
    for i in 0..5u32 {
        for j in 0..(1usize << (5 - i)) {
            let (x1, y1) = coord(i, j);
            let (x2, y2) = coord(i + 1, j / 2);
            p.line(x1, y1, x2, y2, 0.3, 0.0);
        }
    }
    for i in 1..=5u32 {
        for j in 0..(1usize << (5 - i)) {
            circle(p, coord(i, j), RADIUS / 5.0, 1.0);
        }
    }
    let size = 14.0;
    for (j, &c) in ALPHA.iter().enumerate() {
        let x = TREE_X + 1.1 * RADIUS * (2 * j + 1) as f32;
        // The leaf's foot at 30 − 0.3 × 16 − 5 up from the tree's origin.
        let foot = L_H - (TREE_Y + SPACING / 2.0 - 0.3 * 16.0 - 5.0);
        let pitch = 2.2 * RADIUS;
        p.mono_char(
            x - pitch / 2.0,
            foot - size,
            pitch,
            size,
            size,
            c.to_ascii_uppercase(),
        );
    }
}
