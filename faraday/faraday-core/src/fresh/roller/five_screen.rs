//! The five-dice roller as it is drawn (`docs/CODEX32.md` §5.5): "Or let
//! Faraday roll", "Rolled by this computer" over each die's two rolls
//! side by side; its controls are the step-through's
//! (`codex32/walk_screen.rs`). Drawing reads the roller and the entry; it
//! works nothing out.
//!
//! [`pairs`] is each die's first roll (its marker) and second roll (the
//! die) side by side in the die's colour, and the pairs struck as equal
//! under them (owner, 2026-10-24); the codex32 dice sheet draws it too.

use super::five::FiveRoller;
use super::screen::pips;
use crate::codex32::dice_screen::paint;
use crate::codex32::{DieColour, DieRow};
use crate::strings::{EN, fill};
use crate::ui::Paint;
use crate::ui::{Ui, pal::*};
use faraday_draw::tokens::{BODY_S, LABEL_L, LABEL_M, TypeRole};
use faraday_draw::tokens::{
    GAP_SMALL, INPUT_ROLLS_COL, INPUT_ROLLS_LINE, INPUT_ROLLS_STRUCK, ROLLER_BOUNCE, ROLLER_DIE,
    ROLLER_DIE_RADIUS, ROLLER_FIVE_GAP, ROLLER_GAP, ROLLER_PAIR_COL, ROLLER_PAIR_DIE,
    ROLLER_PAIR_LABEL_W, ROLLER_PIP, ROLLER_PIP_OFFSET, ROLLER_SWAY,
};
use faraday_entropy::booklet::BOOKLET_DICE;

/// What masked values are drawn as.
const MASK: &str = "\u{2022}";

/// Each die's two rolls of the character in progress, as drawing reads
/// them from the entry.
pub struct Pairs<'a> {
    /// Each die's first roll, its marker's place, die 1 first.
    pub first: [Option<u8>; BOOKLET_DICE],
    /// Each die's second roll.
    pub second: [Option<u8>; BOOKLET_DICE],
    /// The pairs completed for this character, struck ones included.
    pub struck: &'a [DieRow],
    /// Each die's colour.
    pub colours: [DieColour; BOOKLET_DICE],
    /// The dice's faces.
    pub faces: u8,
    /// The values are drawn as dots (`docs/CODEX32.md` §6.4.6).
    pub masked: bool,
}

/// Where a die is drawn and how: its top-left, its side, the face it
/// shows (0: none yet), the dice's faces, its colour, and whether it
/// tumbles (outlined in the accent).
struct Die {
    x: f32,
    y: f32,
    side: f32,
    face: u8,
    faces: u8,
    colour: DieColour,
    tumbling: bool,
}

/// A die: a rounded square in its colour, pips for a face of a die of
/// six faces or fewer, the number for larger dice; masked, a dot.
fn die(ui: &mut Ui, d: &Die, masked: bool) {
    let scale = d.side / ROLLER_DIE;
    let r = ROLLER_DIE_RADIUS * scale;
    let (body, ink) = paint(d.colour);
    if d.face == 0 {
        ui.stroke(d.x, d.y, d.side, d.side, r, Role::OutlineVariant);
        return;
    }
    ui.fill(d.x, d.y, d.side, d.side, r, body);
    ui.stroke(
        d.x,
        d.y,
        d.side,
        d.side,
        r,
        if d.tumbling {
            Role::Primary
        } else {
            Role::OutlineVariant
        },
    );
    let (cx, cy) = (d.x + d.side / 2.0, d.y + d.side / 2.0);
    if masked {
        number(ui, (cx, cy), MASK, ink);
    } else if d.faces <= 6 {
        for &(dx, dy) in pips(d.face) {
            ui.dot(
                cx + dx * ROLLER_PIP_OFFSET * scale,
                cy + dy * ROLLER_PIP_OFFSET * scale,
                ROLLER_PIP * scale,
                ink,
            );
        }
    } else {
        number(ui, (cx, cy), &d.face.to_string(), ink);
    }
}

/// `s` centred on (`cx`, `cy`).
fn number(ui: &mut Ui, (cx, cy): (f32, f32), s: &str, ink: Paint) {
    let w = ui.measure_as(LABEL_L, s);
    let h = ui.line_as(LABEL_L);
    ui.text_as(cx - w / 2.0, cy - h / 2.0, LABEL_L, ink, s);
}

/// A die's marker: a disc in the die's colour, ringed, its value on it;
/// an empty ring before the first roll.
fn marker(ui: &mut Ui, (cx, cy): (f32, f32), value: Option<u8>, c: DieColour, masked: bool) {
    let r = ROLLER_PAIR_DIE / 2.0;
    let Some(v) = value else {
        ui.stroke(cx - r, cy - r, 2.0 * r, 2.0 * r, r, Role::OutlineVariant);
        return;
    };
    let (body, ink) = paint(c);
    ui.dot(cx, cy, r, Role::OutlineVariant);
    ui.dot(cx, cy, r - 1.0, body);
    let s = if masked {
        MASK.to_string()
    } else {
        v.to_string()
    };
    number(ui, (cx, cy), &s, ink);
}

/// Each die's two rolls side by side at (`x`, `y`): heads "First roll"
/// and "Second roll"; a row per die, its name, its marker with the first
/// roll, the die with the second, and what the pair gives ("lower · 0");
/// then the pairs struck as equal, "Die 2 · 5 then 5 · equal · roll both
/// again". While `r` tumbles a die, the die tumbles in the cell its
/// value goes to. Returns the height.
pub fn pairs(ui: &mut Ui, (x, y, w): (f32, f32, f32), p: &Pairs, r: Option<&FiveRoller>) -> f32 {
    let col = ROLLER_PAIR_COL;
    let lw = ROLLER_PAIR_LABEL_W;
    let d = ROLLER_PAIR_DIE;
    let mut cy = y;
    for (i, head) in [EN.codex32_first_roll, EN.codex32_second_roll]
        .iter()
        .enumerate()
    {
        // A one-line slot over its column: larger text fits down to
        // Standard there, so the two heads never meet
        // (`docs/TEXT-SIZE.md` §3).
        let hx = x + lw + i as f32 * col;
        let room = col - faraday_draw::tokens::GAP_SMALL;
        let hw = ui.text_fit_width(LABEL_M, head, room);
        let at = (hx + (col - hw) / 2.0, cy, ui.line_as(LABEL_M), room);
        ui.text_fit_as(at, LABEL_M, Role::OnSurfaceVariant, head);
    }
    cy += ui.line_as(LABEL_M) + ROLLER_GAP / 2.0;
    let progress = r.map_or(0.0, FiveRoller::progress);
    let lift = ROLLER_BOUNCE * (progress * std::f32::consts::PI).sin();
    let sway = ROLLER_SWAY * (progress * std::f32::consts::TAU).sin() * (1.0 - progress);
    if r.is_some() {
        cy += ROLLER_BOUNCE;
    }
    let row = d + ROLLER_FIVE_GAP;
    for (die_i, &colour) in p.colours.iter().enumerate() {
        let name = fill(EN.codex32_die_n, &[&(die_i + 1).to_string()]);
        ui.text_mid_as((x, cy, d), LABEL_M, Role::OnSurface, &name);
        let tumbling = r.is_some_and(|r| r.tumbling(die_i));
        let shown = r.map_or(0, |r| r.shown(die_i));
        let (l, s) = if tumbling { (lift, sway) } else { (0.0, 0.0) };
        // The first roll: the marker, or the die tumbling towards it.
        let (mx, dx) = (x + lw + (col - d) / 2.0, x + lw + col + (col - d) / 2.0);
        let into_first = p.first[die_i].is_none();
        if tumbling && into_first {
            let td = Die {
                x: mx + s,
                y: cy - l,
                side: d,
                face: shown,
                faces: p.faces,
                colour,
                tumbling,
            };
            die(ui, &td, false);
        } else {
            marker(
                ui,
                (mx + d / 2.0, cy + d / 2.0),
                p.first[die_i],
                colour,
                p.masked,
            );
        }
        let face = if tumbling && !into_first {
            shown
        } else {
            p.second[die_i].unwrap_or(0)
        };
        let dd = Die {
            x: dx + if tumbling && !into_first { s } else { 0.0 },
            y: cy - if tumbling && !into_first { l } else { 0.0 },
            side: d,
            face,
            faces: p.faces,
            colour,
            tumbling: tumbling && !into_first,
        };
        die(ui, &dd, p.masked && !dd.tumbling);
        if let Some(text) = pair_text(p, die_i) {
            let tx = x + lw + 2.0 * col;
            ui.text_mid_as((tx, cy, d), BODY_S.mono(), Role::Primary, &text);
        }
        cy += row;
    }
    cy += struck(ui, (x, cy, w), p);
    cy - y
}

/// Each die's two rolls laid across in one row, for the input panel at
/// the window's foot (`docs/LAYOUT.md` §16.3; owner, 2026-10-24): a
/// column per die, its name over its marker with the first roll beside
/// the die with the second, as the dice sheet's tracks draw them, and
/// what the pair gives under them; under the row the latest pair struck
/// as equal, unless `struck` is false (it then stands elsewhere,
/// [`struck_across`]). Always [`pairs_across_h`] tall, or
/// [`pairs_row_h`] without the struck pair.
pub fn pairs_across(ui: &mut Ui, (x, y, w): (f32, f32, f32), p: &Pairs, struck: bool) -> f32 {
    let k = ui.text_factor();
    let (col, d, line) = (INPUT_ROLLS_COL, ROLLER_PAIR_DIE, rolls_line(k));
    let mut cy = y;
    let pair_w = 2.0 * d + GAP_SMALL;
    let left = |i: usize| x + i as f32 * col;
    let centre = |i: usize| left(i) + pair_w / 2.0;
    // Each die's column is a one-line slot: larger text fits down to
    // Standard there (`docs/TEXT-SIZE.md` §3).
    let room = col - GAP_SMALL;
    let centred = |ui: &mut Ui, i: usize, y: f32, role: TypeRole, ink: Role, s: &str| {
        let sw = ui.text_fit_width(role, s, room);
        ui.text_fit_as((centre(i) - sw / 2.0, y, line, room), role, ink, s);
    };
    for i in 0..BOOKLET_DICE {
        let name = fill(EN.codex32_die_n, &[&(i + 1).to_string()]);
        centred(ui, i, cy, LABEL_M, Role::OnSurface, &name);
    }
    cy += line + GAP_SMALL;
    for (i, &colour) in p.colours.iter().enumerate() {
        marker(
            ui,
            (left(i) + d / 2.0, cy + d / 2.0),
            p.first[i],
            colour,
            p.masked,
        );
        let dd = Die {
            x: left(i) + d + GAP_SMALL,
            y: cy,
            side: d,
            face: p.second[i].unwrap_or(0),
            faces: p.faces,
            colour,
            tumbling: false,
        };
        die(ui, &dd, p.masked);
    }
    cy += d + GAP_SMALL;
    for i in 0..BOOKLET_DICE {
        if let Some(text) = pair_text(p, i) {
            centred(ui, i, cy, BODY_S.mono(), Role::Primary, &text);
        }
    }
    if !struck {
        return pairs_row_h(k);
    }
    cy += line + GAP_SMALL;
    struck_across(ui, (x, cy, w, line), p);
    pairs_across_h(k)
}

/// The latest pair struck as equal, struck through, in a row (x, y) `w`
/// wide and `h` tall, its text cut to the width: "Die 2 · 5 then 5 ·
/// equal · roll both again".
pub fn struck_across(ui: &mut Ui, (x, y, w, h): (f32, f32, f32, f32), p: &Pairs) {
    let equal: Vec<&DieRow> = p.struck.iter().filter(|r| r.bit().is_none()).collect();
    if let Some(row) = equal.last() {
        let (f, sc) = if p.masked {
            (MASK.to_string(), MASK.to_string())
        } else {
            (row.first.to_string(), row.second.to_string())
        };
        let s = fill(
            EN.codex32_die_equal,
            &[&(usize::from(row.die) + 1).to_string(), &f, &sc],
        );
        let tw = ui.text_fit_as((x, y, h, w), BODY_S, Role::Warning, &s);
        ui.fill(x, y + h / 2.0, tw.min(w), 1.0, 0.0, Role::Warning);
    }
}

/// A line of [`pairs_across`] at the text factor `k`
/// (`docs/TEXT-SIZE.md` §5). Arithmetic.
pub fn rolls_line(k: f32) -> f32 {
    INPUT_ROLLS_LINE * k
}

/// The height of [`pairs_across`] without its struck pair, at the text
/// factor `k`: the names, the row of rolls and what the pairs give.
/// Arithmetic.
pub fn pairs_row_h(k: f32) -> f32 {
    let line = rolls_line(k) + GAP_SMALL;
    line + ROLLER_PAIR_DIE + GAP_SMALL + rolls_line(k)
}

/// The height of [`pairs_across`] at the text factor `k`: the names, the
/// row of rolls, what the pairs give, and the room for the struck pair.
/// Arithmetic.
pub fn pairs_across_h(k: f32) -> f32 {
    let line = rolls_line(k) + GAP_SMALL;
    line + ROLLER_PAIR_DIE + GAP_SMALL + line + INPUT_ROLLS_STRUCK as f32 * line - GAP_SMALL
}

/// The width of [`pairs_across`]: the last column ends at its die.
/// Arithmetic.
pub fn pairs_across_w() -> f32 {
    (BOOKLET_DICE - 1) as f32 * INPUT_ROLLS_COL + 2.0 * ROLLER_PAIR_DIE + GAP_SMALL
}

/// What die `d`'s pair gives, once both its rolls are in: "lower · 0" or
/// "higher · 1".
fn pair_text(p: &Pairs, d: usize) -> Option<String> {
    let (f, s) = (p.first[d]?, p.second[d]?);
    if p.masked {
        return Some(fill(EN.codex32_pair_bit, &[MASK, MASK]));
    }
    let bit = faraday_entropy::booklet::bit(f, s)?;
    let word = if bit == 0 {
        EN.codex32_lower
    } else {
        EN.codex32_higher
    };
    Some(fill(EN.codex32_pair_bit, &[word, &bit.to_string()]))
}

/// The pairs struck as equal, struck through: "Die 2 · 5 then 5 · equal
/// · roll both again". Returns the height.
fn struck(ui: &mut Ui, (x, y, w): (f32, f32, f32), p: &Pairs) -> f32 {
    let mut cy = y;
    for row in p.struck.iter().filter(|r| r.bit().is_none()) {
        let (f, sc) = if p.masked {
            (MASK.to_string(), MASK.to_string())
        } else {
            (row.first.to_string(), row.second.to_string())
        };
        let s = fill(
            EN.codex32_die_equal,
            &[&(usize::from(row.die) + 1).to_string(), &f, &sc],
        );
        let tw = ui.text_as(x, cy, BODY_S, Role::Warning, &s);
        let lh = ui.line_as(BODY_S);
        ui.fill(x, cy + lh / 2.0, tw.min(w), 1.0, 0.0, Role::Warning);
        cy += lh + ROLLER_GAP / 2.0;
    }
    cy - y
}

/// The Result's entropy row for faces the roller made, in part or all:
/// `None` where it made none.
pub fn entropy_label(sim: super::Sim) -> Option<&'static str> {
    match sim {
        super::Sim::No => None,
        super::Sim::Some => Some(EN.roller_entropy_both),
        super::Sim::All => Some(EN.roller_entropy_simulated),
    }
}

/// What the snapshot tour draws of the roller alone, before a flow hosts
/// it: on the page's background, `width` × `height` pixels at `dpi`,
/// "Or let Faraday roll" over each die's two rolls `p`, a die tumbling
/// where `r` tumbles one. The frame's pixels, RGBA.
#[cfg(feature = "testkit")]
pub fn preview(r: &FiveRoller, p: &Pairs, width: u16, height: u16, dpi: u16) -> Vec<u8> {
    use faraday_draw::tokens::PAD;
    use faraday_shell_api::{BootState, DisplayInfo, SecureHardware};
    let d = DisplayInfo {
        width,
        height,
        dpi,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    };
    let mut canvas = faraday_draw::Canvas::new(&d);
    let f = canvas.scale().factor();
    let mut hits: Vec<(faraday_draw::Rect, crate::Action)> = Vec::new();
    let (w, h) = (f32::from(width) / f, f32::from(height) / f);
    {
        let mut ui = Ui::new(&mut canvas, f, &mut hits, None);
        ui.fill(0.0, 0.0, w, h, 0.0, Role::Surface);
        let (x, mut y) = (PAD, PAD);
        ui.text_as(
            x,
            y,
            LABEL_L,
            Role::OnSurface,
            EN.roller_or_let_faraday_roll,
        );
        y += ui.line_as(LABEL_L) + ROLLER_GAP / 2.0;
        ui.text_as(
            x,
            y,
            BODY_S,
            Role::OnSurfaceVariant,
            EN.roller_by_this_computer,
        );
        y += ui.line_as(BODY_S) + ROLLER_GAP;
        pairs(&mut ui, (x, y, w - PAD - PAD), p, Some(r));
    }
    canvas.frame().rgba.to_vec()
}
