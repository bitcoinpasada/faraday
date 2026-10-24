//! The dice of Make codex32 shares as they are drawn (`docs/CODEX32.md`
//! §5.1, §5.5, §6.4.2): the dice chooser on the Entropy card (how many
//! faces, each die's colour), the pad for dice other than six-faced
//! ones, and the booklet's dice de-biasing worksheet filling live: each
//! die's two rolls side by side, the 32-leaf tree with its leaves in the
//! booklet's order, and the five die tracks as the booklet draws them,
//! the free space, pads 1 to 6 as diamonds and the d7+ pads 7 to 20 as
//! hexagons, each die's marker on its first roll and the die on its
//! second. Drawing reads the flow's state; nothing here works anything
//! out but where a shape goes.

use crate::ui::Paint;
use faraday_draw::Color;
use faraday_draw::tokens as t;
use faraday_draw::widgets::Icon;
use faraday_entropy::booklet::{BOOKLET_DICE, TREE};

use super::screen::{MASK, a, choice};
use super::{C32, Codex32Make, DIE_SIZES, DieColour};
use crate::fresh::roller::five_screen::{Pairs, pairs};
use crate::strings::{EN, fill};
use crate::ui::pal::*;
use crate::ui::{Style, Ui, W};

// ---------------------------------------------------------------------
// The dice' colours
// ---------------------------------------------------------------------

/// A die's body and the ink of its numbers and pips: the same in every
/// theme, as dice are things on the table, not the screen's colours.
pub(crate) fn paint(c: DieColour) -> (Paint, Paint) {
    let light = Color::rgb(0xff, 0xff, 0xff);
    let dark = Color::rgb(0x14, 0x14, 0x14);
    let (body, ink) = match c {
        DieColour::Red => (Color::rgb(0xc6, 0x28, 0x28), light),
        DieColour::Orange => (Color::rgb(0xef, 0x6c, 0x00), dark),
        DieColour::Yellow => (Color::rgb(0xfb, 0xc0, 0x2d), dark),
        DieColour::Green => (Color::rgb(0x2e, 0x7d, 0x32), light),
        DieColour::Blue => (Color::rgb(0x15, 0x65, 0xc0), light),
        DieColour::Purple => (Color::rgb(0x6a, 0x1b, 0x9a), light),
        DieColour::White => (Color::rgb(0xf7, 0xf7, 0xf7), dark),
        DieColour::Black => (Color::rgb(0x21, 0x21, 0x21), light),
    };
    (Paint::Fixed(body), Paint::Fixed(ink))
}

/// A colour's name.
pub(crate) fn colour_name(c: DieColour) -> &'static str {
    match c {
        DieColour::Red => EN.codex32_colour_red,
        DieColour::Orange => EN.codex32_colour_orange,
        DieColour::Yellow => EN.codex32_colour_yellow,
        DieColour::Green => EN.codex32_colour_green,
        DieColour::Blue => EN.codex32_colour_blue,
        DieColour::Purple => EN.codex32_colour_purple,
        DieColour::White => EN.codex32_colour_white,
        DieColour::Black => EN.codex32_colour_black,
    }
}

// ---------------------------------------------------------------------
// The Entropy card's dice (§5.1)
// ---------------------------------------------------------------------

/// The dice the person rolls, under Roll dice: their faces as a Choice
/// (taken between characters), and a chip per die naming its colour,
/// which a press steps to the next. Returns the height.
pub(super) fn chooser(m: &Codex32Make, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    let mut cy = y + sizes(m, ui, x, y, w);
    let dice = m.dice_choice();
    crate::screens::section_label(ui, x, cy, EN.codex32_die_colours);
    cy += t::PAD + t::GAP_SMALL;
    let mut cx = x;
    for (d, &c) in dice.colours.iter().enumerate() {
        let label = fill(
            EN.codex32_die_colour,
            &[&(d + 1).to_string(), colour_name(c)],
        );
        let cw = 2.0 * (t::CODEX32_SWATCH + t::GAP) + t::GAP + ui.measure(t::LABEL, W::S, &label);
        if cx > x && cx + cw > x + w {
            cx = x;
            cy += t::TOUCH;
        }
        colour_chip(ui, (cx, cy, cw), c, &label, a(C32::Colour(d as u8)));
        cx += cw + t::GAP;
    }
    cy + t::TOUCH + t::GAP - y
}

/// The dice's faces, "Dice" over a Choice of d4 to d20: on the Entropy
/// card, over "Your dice" on the entry, and in the "Demo only" box, one
/// value for all three. Returns the height.
pub(super) fn sizes(m: &Codex32Make, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    let faces = m.dice_choice().faces;
    crate::screens::section_label(ui, x, y, EN.codex32_dice);
    let cy = y + t::PAD + t::GAP_SMALL;
    let names: Vec<String> = DIE_SIZES
        .iter()
        .map(|n| fill(EN.codex32_die_size, &[&n.to_string()]))
        .collect();
    let items: Vec<(&str, bool, crate::Action)> = DIE_SIZES
        .iter()
        .zip(&names)
        .enumerate()
        .map(|(i, (&n, name))| (name.as_str(), faces == n, a(C32::Faces(i as u8))))
        .collect();
    cy - y + choice(ui, x, cy, w, &items)
}

/// Whether the entry has a "Demo only" box: the computer's dice on
/// Faraday Desktop, while a character is left to enter or a step of the
/// step-through to show; Demo's sample, while a character is left.
pub(super) fn demo_box_offered(m: &Codex32Make, roller: bool) -> bool {
    let dice = roller && m.way() == super::Way::Dice;
    (m.front().is_some() && (dice || m.demo())) || (dice && m.walk().shown().is_some())
}

/// What the "Demo only" card holds (owner, 2026-10-24), what is not the
/// person's own entry, from its heading (with `head`) down; `pane_screen.rs` places it
/// and outlines it. Demo's **Use a sample**; on Faraday
/// Desktop the dice's faces, "Rolls d20", and the computer's dice with
/// each die's two rolls, a step at a time (`walk_screen.rs`). Returns its
/// height, nothing where it has nothing to hold.
pub(super) fn demo_content(
    m: &Codex32Make,
    ui: &mut Ui,
    (x, y, w): (f32, f32, f32),
    roller: bool,
    head: bool,
) -> f32 {
    if !demo_box_offered(m, roller) {
        return 0.0;
    }
    let (ix, iw) = (x + t::PAD, w - 2.0 * t::PAD);
    let mut cy = y + t::PAD;
    if head {
        ui.text(ix, cy, t::BODY, W::S, Role::OnSurface, EN.codex32_demo_only);
        cy += ui.line(t::BODY, W::S) + t::GAP;
    }
    if m.demo() && m.front().is_some() {
        cy += faraday_ui::parts::wrap_buttons(
            ui,
            ix,
            cy,
            iw,
            t::TOUCH,
            &[(EN.use_a_sample, Style::Secondary, a(C32::Sample))],
        );
    }
    if roller && m.way() == super::Way::Dice {
        cy += sizes(m, ui, ix, cy, iw);
        let die = fill(EN.codex32_die_size, &[&m.dice_choice().faces.to_string()]);
        let rolls = fill(EN.codex32_rolls_die, &[&die]);
        cy += ui.wrap(ix, cy, iw, t::LABEL, W::S, Role::OnSurfaceVariant, &rolls) + t::GAP;
        cy += super::walk_screen::draw(m, ui, (ix, cy, iw));
    }
    cy += t::PAD;
    cy - y
}

/// One die's colour chip: Material's assist chip, the colour's swatch
/// and its name, in a 48 dp row.
fn colour_chip(
    ui: &mut Ui,
    (x, y, w): (f32, f32, f32),
    c: DieColour,
    label: &str,
    act: crate::Action,
) {
    let top = y + (t::TOUCH - t::CODEX32_CHIP_H) / 2.0;
    let r = t::RADIUS_SMALL;
    if ui.is_pressed(act) || ui.is_hovered(act) {
        ui.wash(
            x,
            top,
            w,
            t::CODEX32_CHIP_H,
            r,
            Role::OnSurface.at(faraday_draw::tokens::STATE_PRESS),
        );
    }
    ui.stroke(x, top, w, t::CODEX32_CHIP_H, r, Role::OutlineVariant);
    let sr = t::CODEX32_SWATCH;
    let (body, _) = paint(c);
    let (sx, sy) = (x + t::GAP + sr, top + t::CODEX32_CHIP_H / 2.0);
    ui.dot(sx, sy, sr + 1.0, Role::OutlineVariant);
    ui.dot(sx, sy, sr, body);
    ui.text_mid(
        sx + sr + t::GAP,
        top,
        t::CODEX32_CHIP_H,
        t::LABEL,
        W::S,
        Role::OnSurface,
        label,
    );
    ui.hit(x, y, w, t::TOUCH, act);
}

// ---------------------------------------------------------------------
// The pad for dice other than six-faced ones (§5.1)
// ---------------------------------------------------------------------

/// The pad of a die of `faces` faces: a key for each face 1 to N in
/// rows, Backspace and Done in the last column, drawn as the six-face
/// pad is. A tens digit typed on the keyboard and waiting shows over
/// it. Returns the height.
pub(super) fn face_pad(m: &Codex32Make, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    let mut cy = y;
    if let Some(d) = m.pending() {
        let s = if m.masked() {
            fill(EN.codex32_face_pending, &[MASK])
        } else {
            fill(EN.codex32_face_pending, &[&d.to_string()])
        };
        cy += ui.wrap(x, cy, w, t::LABEL, W::M, Role::Primary, &s) + t::GAP_SMALL;
    }
    let n = usize::from(m.dice_choice().faces);
    if n == 10 {
        cy += ui.wrap(
            x,
            cy,
            w,
            t::LABEL,
            W::R,
            Role::OnSurfaceVariant,
            EN.codex32_d10_zero,
        ) + t::GAP_SMALL;
    }
    let cols = super::pad_columns(n);
    let rows = n.div_ceil(cols);
    let row = t::CODEX32_KEY_ROW;
    let gap = t::GAP_SMALL;
    let h = rows as f32 * row + 2.0 * gap;
    ui.fill(x, cy, w, h, 0.0, Role::SurfaceContainerLow);
    ui.fill(x, cy, w, 1.0, 0.0, Role::OutlineVariant);
    let kw = (w - gap * (cols as f32 + 2.0)) / (cols as f32 + 1.0);
    let key = |c: usize, r: usize| (x + gap + c as f32 * (kw + gap), cy + gap + r as f32 * row);
    for f in 1..=n {
        let (kx, ky) = key((f - 1) % cols, (f - 1) / cols);
        let shape = (kx, ky, Some(kw), row - gap, t::RADIUS_SMALL);
        ui.button_in(
            shape,
            &f.to_string(),
            Style::Secondary,
            a(C32::Face(f as u8)),
        );
    }
    let (bx, by) = key(cols, 0);
    let back = a(C32::Key(super::OskPress::Back));
    ui.button_in(
        (bx, by, Some(kw), row - gap, t::RADIUS_SMALL),
        "",
        Style::Secondary,
        back,
    );
    let ic = t::ICON_SMALL;
    ui.icon(
        bx + (kw - ic) / 2.0,
        by + (row - gap - ic) / 2.0,
        ic,
        Icon::Delete,
        t::CODEX32_TAB_GLYPH,
        Role::OnSurface,
    );
    if rows > 1 {
        let (dx, dy) = key(cols, rows - 1);
        let done = a(C32::Key(super::OskPress::Done));
        ui.button_in(
            (dx, dy, Some(kw), row - gap, t::RADIUS_SMALL),
            EN.compact_done,
            Style::Primary,
            done,
        );
    }
    cy + h + t::GAP - y
}

// ---------------------------------------------------------------------
// The dice de-biasing sheet (§6.4.2)
// ---------------------------------------------------------------------

/// Each die's two rolls of the character in progress, as the pairs and
/// the roller draw them.
pub(crate) fn pairs_of(m: &Codex32Make) -> Pairs<'_> {
    let d = m.shown_dice();
    let choice = m.dice_choice();
    Pairs {
        first: core::array::from_fn(|i| d.first(i)),
        second: core::array::from_fn(|i| d.second(i)),
        struck: m.shown_rows(),
        colours: choice.colours,
        faces: choice.faces,
        masked: m.masked(),
    }
}

/// The booklet's dice sheet for the character being rolled. Where the
/// card is wide (`rows_only` false), as the booklet's page: each die's
/// two rolls on the left and the tree on the right, then the five die
/// tracks; else the dice as rows of words over the tree. Without `rolls`
/// the input panel holds each die's two rolls (`docs/LAYOUT.md` §16.3),
/// and the sheet keeps the tree and the tracks.
pub(super) fn dice_sheet(
    m: &Codex32Make,
    ui: &mut Ui,
    (x, y, w): (f32, f32, f32),
    rows_only: bool,
    rolls: bool,
) -> f32 {
    let mut cy = y;
    let head = EN.codex32_dice_sheet;
    cy += ui.wrap(x, cy, w, t::LABEL, W::S, Role::OnSurface, head) + t::GAP_SMALL;
    if rows_only {
        if rolls {
            cy += dice_rows(m, ui, x, cy, w, false);
        }
        cy += rolled_line(m, ui, x, cy, w);
        cy += tree(m, ui, x, cy, w.min(t::CODEX32_TREE_MAX_W));
        return cy - y + t::GAP;
    }
    let pw = t::ROLLER_PAIR_LABEL_W + 2.0 * t::ROLLER_PAIR_COL + t::ROLLER_PAIR_COL;
    // The tracks at their full size and the tree at its largest side by
    // side, where the work width holds both (`docs/LAYOUT.md` §16.1).
    let tracks_w = t::CODEX32_TRACK_LABEL_W
        + t::CODEX32_PANEL_GAP
        + t::CODEX32_TRACK_W * t::CODEX32_TRACK_UNIT;
    if w >= tracks_w + t::CODEX32_PANEL_GAP + t::CODEX32_TREE_MAX_W {
        let (tw, lw) = (t::CODEX32_TREE_MAX_W, tracks_w);
        let th = tree(m, ui, x + lw + t::CODEX32_PANEL_GAP, cy, tw);
        let mut ly = cy;
        if rolls {
            ly += pairs(ui, (x, cy, pw), &pairs_of(m), None);
        }
        ly += rolled_line(m, ui, x, ly, lw);
        ly += tracks(m, ui, x, ly, lw);
        return ly.max(cy + th) - y + t::GAP;
    }
    if !rolls {
        cy += tree(m, ui, x, cy, w.min(t::CODEX32_TREE_MAX_W));
        cy += rolled_line(m, ui, x, cy, w);
        cy += tracks(m, ui, x, cy, w);
        return cy - y + t::GAP;
    }
    let ph = pairs(ui, (x, cy, pw), &pairs_of(m), None);
    let tw = (w - pw - t::CODEX32_PANEL_GAP).min(t::CODEX32_TREE_MAX_W);
    if tw >= t::CODEX32_TREE_MIN_W {
        // The tree beside the rolls, as on the booklet's page.
        let th = tree(m, ui, x + w - tw, cy, tw);
        cy += ph.max(th);
    } else {
        cy += ph + t::GAP;
        cy += tree(m, ui, x, cy, w.min(t::CODEX32_TREE_MAX_W));
    }
    cy += rolled_line(m, ui, x, cy, w);
    cy += tracks(m, ui, x, cy, w);
    cy - y + t::GAP
}

/// "Die 1, Die 3 rolled by this computer": the dice of the character in
/// progress whose value the roller made (§5.5), or nothing.
pub(super) fn rolled_line(m: &Codex32Make, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    let dice: Vec<String> = (0..BOOKLET_DICE)
        .filter(|&d| m.die_rolled(d))
        .map(|d| fill(EN.codex32_die_short, &[&(d + 1).to_string()]))
        .collect();
    if dice.is_empty() {
        return 0.0;
    }
    let line = fill(EN.codex32_dice_rolled, &[&dice.join(", ")]);
    ui.wrap(x, y, w, t::CAPTION, W::R, Role::OnSurfaceVariant, &line) + t::GAP_SMALL
}

/// The line a die's row reads: "4 then 2 · lower · 0", or the first value
/// alone, or nothing.
fn die_line(m: &Codex32Make, die: usize) -> String {
    let d = m.shown_dice();
    if m.masked() {
        return match (d.first(die), d.second(die)) {
            (Some(_), Some(_)) => EN.codex32_die_masked.to_string(),
            (Some(_), None) => MASK.to_string(),
            _ => String::new(),
        };
    }
    match (d.first(die), d.second(die)) {
        (Some(f), Some(s)) => {
            let bit = faraday_entropy::booklet::bit(f, s);
            fill(
                EN.codex32_die_then,
                &[
                    &f.to_string(),
                    &s.to_string(),
                    if bit == Some(0) {
                        EN.codex32_lower
                    } else {
                        EN.codex32_higher
                    },
                    &bit.unwrap_or(0).to_string(),
                ],
            )
        }
        (Some(f), None) => f.to_string(),
        _ => String::new(),
    }
}

/// The five dice as rows of words, "Die 3 · 4 then 2 · lower · 0", each
/// die's name beside its colour, the struck pairs under them (`mobile`
/// and `small`). With `path`, the bits as a line, "0 1 1 0 ·".
pub(super) fn dice_rows(m: &Codex32Make, ui: &mut Ui, x: f32, y: f32, w: f32, path: bool) -> f32 {
    let mut cy = y;
    let colours = m.dice_choice().colours;
    for (die, &colour) in colours.iter().enumerate() {
        let line = die_line(m, die);
        let on = m.shown_dice().first(die).is_none() || m.shown_dice().second(die).is_none();
        let text = fill(EN.codex32_die_n, &[&(die + 1).to_string()]);
        let lh = ui.line(t::LABEL, W::R);
        swatch(ui, x + t::CODEX32_SWATCH, cy + lh / 2.0, colour);
        let tx = x + 2.0 * t::CODEX32_SWATCH + t::GAP_SMALL;
        let marked = m.walk_die_mark().is_some_and(|(d, _)| d == die);
        let ink = if marked {
            Role::Primary
        } else if on {
            Role::OnSurface
        } else {
            Role::OnSurfaceVariant
        };
        ui.text(tx, cy, t::LABEL, W::S, ink, &text);
        ui.text(
            x + t::CODEX32_LABEL_W,
            cy,
            t::LABEL,
            W::M,
            Role::OnSurface,
            &line,
        );
        cy += lh + t::GAP_SMALL;
    }
    cy += struck(m, ui, x, cy, w);
    if path {
        let bits = known_bits(m, false);
        let s: Vec<String> = (0..BOOKLET_DICE)
            .map(|d| match bits[d] {
                Some(_) if m.masked() => MASK.to_string(),
                Some(b) => b.to_string(),
                None => "·".to_string(),
            })
            .collect();
        ui.text(x, cy, t::BODY, W::M, Role::Primary, &s.join(" "));
        cy += ui.line(t::BODY, W::M) + t::GAP;
    }
    cy - y
}

/// A die's colour as a small swatch centred at (`cx`, `cy`), ringed so a
/// white die shows on a light page.
fn swatch(ui: &mut Ui, cx: f32, cy: f32, c: DieColour) {
    let r = t::CODEX32_SWATCH / 2.0 + 1.0;
    ui.dot(cx, cy, r + 1.0, Role::OutlineVariant);
    ui.dot(cx, cy, r, paint(c).0);
}

/// The struck pairs of the character in progress: "Die 3 · 3 then 3 ·
/// equal · roll both again", struck through.
pub(super) fn struck(m: &Codex32Make, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    let mut cy = y;
    for row in m.shown_rows().iter().filter(|r| r.bit().is_none()) {
        let (f, sc) = if m.masked() {
            (MASK.to_string(), MASK.to_string())
        } else {
            (row.first.to_string(), row.second.to_string())
        };
        let s = fill(
            EN.codex32_die_equal,
            &[&(usize::from(row.die) + 1).to_string(), &f, &sc],
        );
        let tw = ui.text(x, cy, t::CAPTION, W::R, Role::Warning, &s);
        let lh = ui.line(t::CAPTION, W::R);
        ui.fill(x, cy + lh / 2.0, tw.min(w), 1.0, 0.0, Role::Warning);
        cy += lh + t::GAP_SMALL;
    }
    cy - y
}

/// Each die's bit, once its pair is in and not equal.
/// Where the sheets are masked, with `shown` the bits are none: the
/// tree draws no path.
fn known_bits(m: &Codex32Make, shown: bool) -> [Option<u8>; BOOKLET_DICE] {
    let mut out = [None; BOOKLET_DICE];
    if shown && m.masked() {
        return out;
    }
    for (d, slot) in out.iter_mut().enumerate() {
        *slot = m.shown_dice().die_bit(d).flatten();
    }
    out
}

/// A closed shape through `pts` (design units), filled with `fill` if
/// given and outlined in `edge`.
fn shape(ui: &mut Ui, pts: &[(f32, f32)], fill_c: Option<Paint>, edge: Paint) {
    let (f, ox, oy) = (ui.f, ui.ox, ui.oy);
    let px: Vec<(f32, f32)> = pts
        .iter()
        .map(|&(x, y)| ((x + ox) * f, (y + oy) * f))
        .collect();
    if let Some(c) = fill_c {
        let c = ui.col(c);
        ui.c.fill_contours(&[&px], c);
    }
    let e = ui.col(edge);
    for (i, &(x0, y0)) in px.iter().enumerate() {
        let (x1, y1) = px[(i + 1) % px.len()];
        ui.c.line(x0, y0, x1, y1, f, e);
    }
}

/// The outline of the closed shape through `pts` (design units), `width`
/// thick.
fn outline(ui: &mut Ui, pts: &[(f32, f32)], width: f32, c: impl Into<faraday_ui::Paint>) {
    let (f, ox, oy) = (ui.f, ui.ox, ui.oy);
    let e = ui.col(c);
    for (i, &(x0, y0)) in pts.iter().enumerate() {
        let (x1, y1) = pts[(i + 1) % pts.len()];
        ui.c.line(
            (x0 + ox) * f,
            (y0 + oy) * f,
            (x1 + ox) * f,
            (y1 + oy) * f,
            width * f,
            e,
        );
    }
}

/// A ring centred at (`cx`, `cy`), `r` across its middle, `width` thick.
fn ring(ui: &mut Ui, (cx, cy): (f32, f32), r: f32, width: f32, c: Paint) {
    let rect = ui.rect(cx - r, cy - r, 2.0 * r, 2.0 * r);
    let c = ui.col(c);
    let f = ui.f;
    ui.c.stroke_rounded_rect(rect, r * f, width * f, c);
}

/// Where pad `pad` (1 to 20) of a track sits, in the booklet's units
/// from the track's left end and its low pads' centre line (up is
/// positive): its outline, and its centre.
fn pad_shape(pad: u8) -> (Vec<(f32, f32)>, (f32, f32)) {
    let i = f32::from(pad);
    let even = pad.is_multiple_of(2);
    if pad <= 6 {
        // A diamond, its left corner on the centre line, every other one
        // raised by its height and a gap.
        let step = 1.0 + t::CODEX32_TRACK_GAP;
        let (lx, base) = (i * step, if even { step } else { 0.0 });
        let pts = vec![
            (lx, base),
            (lx + 1.0, base + 1.0),
            (lx + 2.0, base),
            (lx + 1.0, base - 1.0),
        ];
        (pts, (lx + 1.0, base))
    } else {
        // A hexagon, pointed at top and bottom.
        let (s, hw) = (t::CODEX32_HEX_SIDE, t::CODEX32_HEX_W);
        let ox = t::CODEX32_HEX_X + i * t::CODEX32_HEX_PITCH;
        let oy = t::CODEX32_HEX_Y + if even { t::CODEX32_HEX_RAISE } else { 0.0 };
        let pts = vec![
            (ox, oy),
            (ox, oy + s),
            (ox + hw / 2.0, oy + s + s / 2.0),
            (ox + hw, oy + s),
            (ox + hw, oy),
            (ox + hw / 2.0, oy - s / 2.0),
        ];
        (pts, (ox + hw / 2.0, oy + s / 2.0))
    }
}

/// The five die tracks, as the booklet draws them under "Die tracks": a
/// rule, "Die pads" over the diamonds and "Die pads (d7+)" over the
/// hexagons; each track the die's name and colour ([`track`]) and the
/// bit after it. The struck pairs are under each die's two rolls.
fn tracks(m: &Codex32Make, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    let lw = t::CODEX32_TRACK_LABEL_W;
    let u = ((w - lw - t::CODEX32_PANEL_GAP) / t::CODEX32_TRACK_W).min(t::CODEX32_TRACK_UNIT);
    let tx = x + lw;
    let mut cy = y + t::GAP;
    ui.text(
        x,
        cy,
        t::LABEL,
        W::S,
        Role::OnSurface,
        EN.codex32_die_tracks,
    );
    cy += ui.line(t::LABEL, W::S);
    ui.rule(x, cy, lw + t::CODEX32_TRACK_W * u, Role::OutlineVariant);
    cy += t::GAP_SMALL;
    // The heads over the pads, where the booklet sets them.
    let heads = [
        (0.0, EN.codex32_free_space),
        (t::CODEX32_PADS_HEAD_X, EN.codex32_die_pads),
        (t::CODEX32_PADS_LARGE_HEAD_X, EN.codex32_die_pads_large),
    ];
    for (i, &(hx, s)) in heads.iter().enumerate() {
        let next = heads.get(i + 1).map_or(f32::MAX, |n| tx + n.0 * u);
        let sw = ui.measure(t::CAPTION, W::S, s);
        if tx + hx * u + sw + t::GAP_SMALL <= next {
            ui.text(tx + hx * u, cy, t::CAPTION, W::S, Role::OnSurfaceVariant, s);
        }
    }
    cy += ui.line(t::CAPTION, W::S) + t::GAP_SMALL;
    let colours = m.dice_choice().colours;
    let th = (t::CODEX32_TRACK_TOP + 1.0) * u;
    for (die, &colour) in colours.iter().enumerate() {
        let name = fill(EN.codex32_die_n, &[&(die + 1).to_string()]);
        let lh = ui.line(t::CAPTION, W::S);
        swatch(ui, x + t::CODEX32_SWATCH, cy + th / 2.0 - lh / 2.0, colour);
        let nx = x + 2.0 * t::CODEX32_SWATCH + t::GAP_SMALL;
        ui.text(
            nx,
            cy + th / 2.0 - lh,
            t::CAPTION,
            W::S,
            Role::OnSurface,
            &name,
        );
        track(m, ui, (tx, cy, u), die);
        if let Some(b) = m.shown_dice().die_bit(die).flatten() {
            let text = if m.masked() {
                MASK.to_string()
            } else {
                b.to_string()
            };
            let bx = tx + t::CODEX32_TRACK_W * u + t::GAP;
            ui.text_mid(bx, cy, th, t::LABEL, W::M, Role::Primary, &text);
        }
        cy += th + t::CODEX32_TRACK_SPACE;
    }
    cy - y
}

/// `s` centred on (`cx`, `cy`) in the caption size.
fn centred(ui: &mut Ui, (cx, cy): (f32, f32), s: &str, ink: Paint) {
    let w = ui.measure(t::CAPTION, W::S, s);
    let h = ui.line(t::CAPTION, W::S);
    ui.text(cx - w / 2.0, cy - h / 2.0, t::CAPTION, W::S, ink, s);
}

/// Die `die`'s track with its top-left at (`tx`, `top`), `u` dp to the
/// booklet's unit: its free space, pads 1 to 20 (those past the dice's
/// faces outlined only), the die's marker ringed in its colour on its
/// first value (on the free space before it), and the die in its colour
/// on its second.
fn track(m: &Codex32Make, ui: &mut Ui, (tx, top, u): (f32, f32, f32), die: usize) {
    // The booklet's y runs up from the low pads' centre line.
    let at = |(bx, by): (f32, f32)| (tx + bx * u, top + (t::CODEX32_TRACK_TOP - by) * u);
    let faces = m.dice_choice().faces;
    let free = at((t::CODEX32_FREE_X, t::CODEX32_FREE_Y));
    ring(
        ui,
        free,
        t::CODEX32_FREE_R * u,
        1.0,
        Role::OutlineVariant.paint(),
    );
    for pad in 1..=20u8 {
        let (pts, c) = pad_shape(pad);
        let pts: Vec<(f32, f32)> = pts.into_iter().map(at).collect();
        let (fill_c, edge, ink) = if pad <= faces {
            (
                Some(Role::SurfaceContainerHighest.paint()),
                Role::OutlineVariant.paint(),
                Role::OnSurfaceVariant.paint(),
            )
        } else {
            (
                None,
                Role::OutlineVariant.paint(),
                Role::OnSurfaceVariant.paint(),
            )
        };
        shape(ui, &pts, fill_c, edge);
        centred(ui, at(c), &pad.to_string(), ink);
    }
    // The pad the step-through's step touches, outlined in the accent.
    if let Some((_, pad)) = m.walk_die_mark().filter(|&(d, _)| d == die) {
        let (pts, _) = pad_shape(pad);
        let pts: Vec<(f32, f32)> = pts.into_iter().map(at).collect();
        outline(ui, &pts, t::CODEX32_FRONT_STROKE, Role::Primary);
    }
    let (first, second) = (m.shown_dice().first(die), m.shown_dice().second(die));
    let (body, ink) = paint(m.dice_choice().colours[die]);
    // The marker: on the first value's pad, on the free space before; a
    // masked sheet shows where neither is.
    if !(m.masked() && first.is_some()) {
        let marker = first.map_or(free, |v| at(pad_shape(v).1));
        let r = t::CODEX32_MARKER * u / 2.0;
        let stroke = t::CODEX32_MARKER_STROKE;
        ring(ui, marker, r + stroke, stroke, Role::OutlineVariant.paint());
        ring(ui, marker, r, stroke, body);
    }
    // The die, on its second value's pad.
    if let (Some(v), false) = (second, m.masked()) {
        let (cx, cy) = at(pad_shape(v).1);
        let side = t::CODEX32_TRACK_DIE * u;
        let (dx, dy) = (cx - side / 2.0, cy - side / 2.0);
        ui.fill(dx, dy, side, side, t::CODEX32_RADIUS, body);
        ui.stroke(dx, dy, side, side, t::CODEX32_RADIUS, Role::OutlineVariant);
        centred(ui, (cx, cy), &v.to_string(), ink);
    }
}

/// The tree of 31 nodes over the 32 leaves, the leaves in the booklet's
/// order and its proportions (a level is [`t::CODEX32_TREE_LEVEL`] times
/// the space between two leaves); the path from the root drawn in the
/// accent for each bit known from die 1 on, and the leaf reached named
/// once die 5's bit is in.
fn tree(m: &Codex32Make, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    let bits = known_bits(m, true);
    let lw = w / TREE.len() as f32;
    let lv = t::CODEX32_TREE_LEVEL * lw;
    let top = y + t::CODEX32_TREE_NODE;
    let node = |level: usize, idx: usize| -> (f32, f32) {
        let span = 32usize >> level;
        (
            x + (idx * span) as f32 * lw + span as f32 * lw / 2.0,
            top + level as f32 * lv,
        )
    };
    let (f, ox, oy) = (ui.f, ui.ox, ui.oy);
    let faint = ui.col(Role::OnSurfaceVariant);
    let accent = ui.col(Role::Primary);
    let mut at = 0usize;
    let mut path = true;
    for (level, bit) in bits.iter().enumerate() {
        for idx in 0..(1usize << level) {
            let (px, py) = node(level, idx);
            for c in 0..2 {
                let (cx, cyy) = node(level + 1, idx * 2 + c);
                ui.c.line(
                    (px + ox) * f,
                    (py + oy) * f,
                    (cx + ox) * f,
                    (cyy + oy) * f,
                    f,
                    faint,
                );
            }
        }
        match *bit {
            Some(b) if path => {
                let (px, py) = node(level, at);
                at = at * 2 + usize::from(b);
                let (cx, cyy) = node(level + 1, at);
                let sw = t::CODEX32_PATH_STROKE * f;
                ui.c.line(
                    (px + ox) * f,
                    (py + oy) * f,
                    (cx + ox) * f,
                    (cyy + oy) * f,
                    sw,
                    accent,
                );
            }
            _ => path = false,
        }
    }
    for level in 0..BOOKLET_DICE {
        for idx in 0..(1usize << level) {
            let (px, py) = node(level, idx);
            ui.dot(px, py, t::CODEX32_TREE_NODE, Role::OnSurface);
        }
    }
    let ly = top + BOOKLET_DICE as f32 * lv + t::GAP_SMALL;
    for (i, &leaf) in TREE.iter().enumerate() {
        let s = char::from(leaf.to_ascii_uppercase()).to_string();
        let reached = path && i == at;
        let cw = ui.measure(t::CAPTION, W::M, &s);
        ui.text(
            x + i as f32 * lw + (lw - cw) / 2.0,
            ly,
            t::CAPTION,
            W::M,
            if reached {
                Role::Primary
            } else {
                Role::OnSurface
            },
            &s,
        );
    }
    let mut h = ly - y + ui.line(t::CAPTION, W::M) + t::GAP_SMALL;
    if path && bits.iter().all(Option::is_some) {
        let b: String = bits.iter().map(|b| b.unwrap_or(0).to_string()).collect();
        let s = fill(
            EN.codex32_path_leaf,
            &[&b, &at.to_string(), &char::from(TREE[at]).to_string()],
        );
        h += ui.wrap(x, y + h, w, t::LABEL, W::M, Role::Primary, &s) + t::GAP_SMALL;
    }
    h
}
