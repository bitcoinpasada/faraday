//! The roller as it is drawn (`docs/DESIGN.md` §4's dice pad, the dice
//! row): "Rolled by this computer" over one die, its Roll button, and
//! Rolls per click. Drawing reads the roller; it works nothing out.

use super::{Act, PerClick, Roller};
use crate::Action;
use crate::strings::EN;
use crate::ui::{Style, Ui, W, pal::*};
use faraday_draw::tokens::{
    ROLLER_BOUNCE, ROLLER_CHIP, ROLLER_COUNT_FIELD, ROLLER_DIE, ROLLER_DIE_RADIUS, ROLLER_GAP,
    ROLLER_LABEL, ROLLER_PIP, ROLLER_PIP_OFFSET, ROLLER_SWAY,
};

/// Where the pips of each face stand, in steps of the pip offset from the
/// die's centre.
pub(super) fn pips(face: u8) -> &'static [(f32, f32)] {
    const TL: (f32, f32) = (-1.0, -1.0);
    const TR: (f32, f32) = (1.0, -1.0);
    const ML: (f32, f32) = (-1.0, 0.0);
    const MR: (f32, f32) = (1.0, 0.0);
    const BL: (f32, f32) = (-1.0, 1.0);
    const BR: (f32, f32) = (1.0, 1.0);
    const C: (f32, f32) = (0.0, 0.0);
    match face {
        1 => &[C],
        2 => &[TL, BR],
        3 => &[TL, C, BR],
        4 => &[TL, TR, BL, BR],
        5 => &[TL, TR, C, BL, BR],
        6 => &[TL, TR, ML, MR, BL, BR],
        _ => &[],
    }
}

/// One die with its top-left at (`x`, `y`), showing `face` (0: blank),
/// lifted by `lift` and swayed by `sway` while it tumbles; `tumbling`
/// outlines it in the accent.
fn die(ui: &mut Ui, x: f32, y: f32, face: u8, lift: f32, sway: f32, tumbling: bool) {
    let (x, y) = (x + sway, y - lift);
    ui.fill(x, y, ROLLER_DIE, ROLLER_DIE, ROLLER_DIE_RADIUS, INNER);
    ui.stroke(
        x,
        y,
        ROLLER_DIE,
        ROLLER_DIE,
        ROLLER_DIE_RADIUS,
        if tumbling { ACCENT } else { BORDER },
    );
    let (cx, cy) = (x + ROLLER_DIE / 2.0, y + ROLLER_DIE / 2.0);
    for &(dx, dy) in pips(face) {
        ui.dot(
            cx + dx * ROLLER_PIP_OFFSET,
            cy + dy * ROLLER_PIP_OFFSET,
            ROLLER_PIP,
            TEXT,
        );
    }
}

/// What the host says to the drawing of its roller.
#[derive(Debug, Clone, Copy)]
pub struct Hosting {
    /// How many more values the host's entries take.
    pub room: usize,
    /// The number field has the keys.
    pub typing: bool,
}

/// The roller at (`x`, `y`) in `w` units: the label, the die and its
/// button, and Rolls per click with the number field. `act` is the
/// host's action for each of the roller's. Returns the height used.
pub fn draw(
    ui: &mut Ui,
    (x, y, w): (f32, f32, f32),
    r: &Roller,
    host: Hosting,
    act: impl Fn(Act) -> Action,
) -> f32 {
    let Hosting { room, typing } = host;
    let mut cy = y;
    ui.text(x, cy, ROLLER_LABEL, W::R, MUTED, EN.roller_by_this_computer);
    cy += ui.line(ROLLER_LABEL, W::R) + ROLLER_GAP;
    // The die stands on the line the room for its bounce leaves.
    let p = r.progress();
    let lift = ROLLER_BOUNCE * (p * std::f32::consts::PI).sin();
    let sway = ROLLER_SWAY * (p * std::f32::consts::TAU).sin() * (1.0 - p);
    cy += ROLLER_BOUNCE;
    die(ui, x, cy, r.shown(), lift, sway, r.rolling());
    let (label, style, action) = if r.rolling() {
        (EN.roller_stop, Style::Secondary, act(Act::Stop))
    } else if room == 0 {
        (EN.roller_roll, Style::Disabled, act(Act::Roll))
    } else {
        (EN.roller_roll, Style::Primary, act(Act::Roll))
    };
    ui.button(
        x + ROLLER_DIE + ROLLER_GAP,
        cy + (ROLLER_DIE - ROLLER_CHIP) / 2.0,
        None,
        ROLLER_CHIP,
        label,
        style,
        action,
    );
    cy += ROLLER_DIE + ROLLER_GAP;
    ui.text(x, cy, ROLLER_LABEL, W::R, MUTED, EN.roller_per_click);
    cy += ui.line(ROLLER_LABEL, W::R) + ROLLER_GAP / 2.0;
    let choices = [
        (PerClick::One, EN.roller_one),
        (PerClick::Five, EN.roller_five),
        (PerClick::Ten, EN.roller_ten),
        (PerClick::ToEnd, EN.roller_to_the_end),
    ];
    let mut bx = x;
    let step = |bx: f32, cy: f32, bw: f32| -> (f32, f32) {
        if bx > x && bx + bw > x + w {
            (x, cy + ROLLER_CHIP + ROLLER_GAP / 2.0)
        } else {
            (bx, cy)
        }
    };
    for (p, label) in choices {
        let bw = ui.measure(ROLLER_LABEL + 1.0, W::S, label) + 2.0 * ROLLER_GAP + ROLLER_GAP;
        (bx, cy) = step(bx, cy, bw);
        let on = r.per() == p;
        bx += ui.button(
            bx,
            cy,
            Some(bw),
            ROLLER_CHIP,
            label,
            if on { Style::Primary } else { Style::Secondary },
            act(Act::Per(p)),
        ) + ROLLER_GAP / 2.0;
    }
    // The number typed.
    (bx, cy) = step(bx, cy, ROLLER_COUNT_FIELD);
    let on = r.per() == PerClick::Typed;
    ui.fill(bx, cy, ROLLER_COUNT_FIELD, ROLLER_CHIP, ROLLER_GAP, BG);
    ui.stroke(
        bx,
        cy,
        ROLLER_COUNT_FIELD,
        ROLLER_CHIP,
        ROLLER_GAP,
        if on || typing { ACCENT } else { BORDER },
    );
    let (shown, color) = if r.typed().is_empty() {
        (EN.roller_number, DIM)
    } else {
        (r.typed(), TEXT)
    };
    let tx = bx + ROLLER_GAP;
    let tw = ui.text_mid(tx, cy, ROLLER_CHIP, ROLLER_LABEL + 1.0, W::R, color, shown);
    if typing {
        ui.caret(tx + tw, cy + ROLLER_GAP / 2.0, ROLLER_CHIP - ROLLER_GAP);
    }
    ui.hit(bx, cy, ROLLER_COUNT_FIELD, ROLLER_CHIP, act(Act::Count));
    cy += ROLLER_CHIP;
    cy - y
}
