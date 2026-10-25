//! The computer's die as it is drawn (`docs/DESIGN.md` §4.3): one die,
//! its pips, tumbling while a click rolls. Faraday Desktop's host strip
//! draws it beside its own Roll and Rolls per click
//! (`docs/DESKTOP-HOST.md` §3.2). Drawing reads the roller; it works
//! nothing out.

use super::{PerClick, Roller};
use crate::strings::EN;
use crate::ui::{Ui, pal::*};
use faraday_draw::tokens::{
    ROLLER_BOUNCE, ROLLER_DIE, ROLLER_DIE_RADIUS, ROLLER_PIP, ROLLER_PIP_OFFSET, ROLLER_SWAY,
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
    ui.fill(
        x,
        y,
        ROLLER_DIE,
        ROLLER_DIE,
        ROLLER_DIE_RADIUS,
        Role::SurfaceContainerHighest,
    );
    ui.stroke(
        x,
        y,
        ROLLER_DIE,
        ROLLER_DIE,
        ROLLER_DIE_RADIUS,
        if tumbling {
            Role::Primary
        } else {
            Role::OutlineVariant
        },
    );
    let (cx, cy) = (x + ROLLER_DIE / 2.0, y + ROLLER_DIE / 2.0);
    for &(dx, dy) in pips(face) {
        ui.dot(
            cx + dx * ROLLER_PIP_OFFSET,
            cy + dy * ROLLER_PIP_OFFSET,
            ROLLER_PIP,
            Role::OnSurface,
        );
    }
}

/// The die's height with the room for its bounce: the host strip grows
/// to it while the die shows.
pub const ROW_H: f32 = ROLLER_DIE + ROLLER_BOUNCE;

/// The choices of Rolls per click, as the host's menu names them.
fn per_labels() -> [&'static str; 4] {
    [
        EN.roller_one,
        EN.roller_five,
        EN.roller_ten,
        EN.roller_to_the_end,
    ]
}

/// The die alone with its top-left at (`x`, `y`), tumbling as the row's
/// does: Faraday Desktop's host strip draws it beside its own Roll and
/// Rolls per click (`docs/DESKTOP-HOST.md` §3.2). Takes `ROW_H`.
pub(crate) fn die_alone(ui: &mut Ui, (x, y): (f32, f32), r: &Roller) {
    let p = r.progress();
    let lift = ROLLER_BOUNCE * (p * std::f32::consts::PI).sin();
    let sway = ROLLER_SWAY * (p * std::f32::consts::TAU).sin() * (1.0 - p);
    die(ui, x, y + ROLLER_BOUNCE, r.shown(), lift, sway, r.rolling());
}

/// Rolls per click's choices, each its name and what it sets, in the
/// order the row shows them.
pub(crate) fn per_choices() -> [(&'static str, PerClick); 4] {
    let l = per_labels();
    [
        (l[0], PerClick::One),
        (l[1], PerClick::Five),
        (l[2], PerClick::Ten),
        (l[3], PerClick::ToEnd),
    ]
}
