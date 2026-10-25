//! The roller as it is drawn (`docs/DESIGN.md` §4's dice pad, the dice
//! row): "Rolled by this computer" over one die, its Roll button, and
//! Rolls per click. Drawing reads the roller; it works nothing out.

use super::{Act, PerClick, Roller};
use crate::Action;
use crate::strings::EN;
use crate::ui::{Style, Ui, W, pal::*};
use faraday_draw::tokens::{
    PAD, ROLLER_BOUNCE, ROLLER_CHIP, ROLLER_COUNT_FIELD, ROLLER_DIE, ROLLER_DIE_RADIUS, ROLLER_GAP,
    ROLLER_LABEL, ROLLER_PIP, ROLLER_PIP_OFFSET, ROLLER_SWAY, TOUCH,
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

/// What the host says to the drawing of its roller.
#[derive(Debug, Clone, Copy)]
pub struct Hosting {
    /// How many more values the host's entries take.
    pub room: usize,
    /// The number field has the keys.
    pub typing: bool,
}

/// The roller's one row (the "Demo only" card's strip, `docs/DEMO.md`
/// §14): its height, the die and the room for its bounce.
pub const ROW_H: f32 = ROLLER_DIE + ROLLER_BOUNCE;

/// The widths of the row's pieces, in order: the die, Roll (or Stop),
/// "Rolls per click", the choices as one segmented button, the number
/// field. Arithmetic on the font.
fn row_widths(ui: &Ui) -> [f32; 5] {
    let size = ROLLER_LABEL + 2.0;
    let roll = ui
        .measure(size, W::S, EN.roller_roll)
        .max(ui.measure(size, W::S, EN.roller_stop))
        + 2.0 * PAD;
    let label = ui.measure(ROLLER_LABEL, W::R, EN.roller_per_click);
    let labels = per_labels();
    let seg = ui.segment_width(ROLLER_LABEL + 1.0, &labels) * labels.len() as f32;
    [ROLLER_DIE, roll, label, seg, ROLLER_COUNT_FIELD]
}

/// The choices of Rolls per click, as the segmented button names them.
fn per_labels() -> [&'static str; 4] {
    [
        EN.roller_one,
        EN.roller_five,
        EN.roller_ten,
        EN.roller_to_the_end,
    ]
}

/// Where each piece of the row stands from (`x`, `y`) in `w`, its first
/// row beginning at `from`: left to right, `ROLLER_GAP` apart, a new row
/// [`ROW_H`] lower where the next does not fit. With where the last row
/// ends and its top. Arithmetic.
fn row_places(
    ws: &[f32; 5],
    (x, y, w): (f32, f32, f32),
    from: f32,
) -> ([(f32, f32); 5], (f32, f32)) {
    let mut at = [(0.0, 0.0); 5];
    let (mut cx, mut cy) = (from, y);
    for (k, bw) in ws.iter().enumerate() {
        if cx > x && cx + bw > x + w {
            cx = x;
            cy += ROW_H + ROLLER_GAP;
        }
        at[k] = (cx, cy);
        cx += bw + ROLLER_GAP;
    }
    (at, (cx, cy))
}

/// Where the row drawn at (`x`, `y`) in `w` from `from` ends: the x after
/// its last piece and its last row's top, measured and not drawn.
/// Arithmetic on the font.
pub fn row_end(ui: &Ui, (x, y, w): (f32, f32, f32), from: f32) -> (f32, f32) {
    row_places(&row_widths(ui), (x, y, w), from).1
}

/// The roller in one row, wrapping to `w` where it does not fit, its
/// first row beginning at `from` (after what stands before it): the die,
/// Roll (Stop while a click rolls), Rolls per click as Material's
/// segmented button, and the number field, each 48 dp tall beside the
/// die. Returns where it ends, as [`row_end`].
pub fn row(
    ui: &mut Ui,
    (x, y, w): (f32, f32, f32),
    from: f32,
    r: &Roller,
    host: Hosting,
    act: impl Fn(Act) -> Action,
) -> (f32, f32) {
    let ws = row_widths(ui);
    let (at, end) = row_places(&ws, (x, y, w), from);
    let mid = |top: f32| top + (ROW_H - TOUCH) / 2.0;
    let p = r.progress();
    let lift = ROLLER_BOUNCE * (p * std::f32::consts::PI).sin();
    let sway = ROLLER_SWAY * (p * std::f32::consts::TAU).sin() * (1.0 - p);
    die(
        ui,
        at[0].0,
        at[0].1 + ROLLER_BOUNCE,
        r.shown(),
        lift,
        sway,
        r.rolling(),
    );
    let (label, style, action) = if r.rolling() {
        (EN.roller_stop, Style::Secondary, act(Act::Stop))
    } else if host.room == 0 {
        (EN.roller_roll, Style::Disabled, act(Act::Roll))
    } else {
        (EN.roller_roll, Style::Primary, act(Act::Roll))
    };
    ui.button(
        at[1].0,
        mid(at[1].1),
        Some(ws[1]),
        TOUCH,
        label,
        style,
        action,
    );
    let muted = Role::OnSurfaceVariant;
    ui.text_mid(
        at[2].0,
        at[2].1,
        ROW_H,
        ROLLER_LABEL,
        W::R,
        muted,
        EN.roller_per_click,
    );
    let pers = [
        PerClick::One,
        PerClick::Five,
        PerClick::Ten,
        PerClick::ToEnd,
    ];
    let options: Vec<(&str, Action)> = per_labels()
        .iter()
        .zip(pers)
        .map(|(l, p)| (*l, act(Act::Per(p))))
        .collect();
    let chosen = pers.iter().position(|p| *p == r.per());
    ui.segmented(
        (at[3].0, mid(at[3].1), TOUCH),
        ROLLER_LABEL + 1.0,
        &options,
        chosen,
        None,
    );
    count_field(
        ui,
        (at[4].0, mid(at[4].1), TOUCH),
        r,
        host.typing,
        act(Act::Count),
    );
    end
}

/// The number field of Rolls per click at (`x`, `y`), `h` tall: the
/// number typed, or its hint, the caret while it has the keys.
fn count_field(ui: &mut Ui, (x, y, h): (f32, f32, f32), r: &Roller, typing: bool, action: Action) {
    let on = r.per() == PerClick::Typed;
    ui.fill(
        x,
        y,
        ROLLER_COUNT_FIELD,
        h,
        ROLLER_GAP,
        Role::SurfaceContainerHighest,
    );
    let edge = if on || typing {
        Role::Primary
    } else {
        Role::OutlineVariant
    };
    ui.stroke(x, y, ROLLER_COUNT_FIELD, h, ROLLER_GAP, edge);
    let (shown, color) = if r.typed().is_empty() {
        (EN.roller_number, Role::OnSurfaceVariant)
    } else {
        (r.typed(), Role::OnSurface)
    };
    let tx = x + ROLLER_GAP;
    let tw = ui.text_mid(tx, y, h, ROLLER_LABEL + 1.0, W::R, color, shown);
    if typing {
        ui.caret(tx + tw, y + ROLLER_GAP / 2.0, h - ROLLER_GAP);
    }
    ui.hit(x, y, ROLLER_COUNT_FIELD, h, action);
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
    let (ink, muted) = (Role::OnSurface, Role::OnSurfaceVariant);
    let Hosting { room, typing } = host;
    let mut cy = y;
    ui.text(x, cy, ROLLER_LABEL, W::R, muted, EN.roller_by_this_computer);
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
    ui.text(x, cy, ROLLER_LABEL, W::R, muted, EN.roller_per_click);
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
    ui.fill(
        bx,
        cy,
        ROLLER_COUNT_FIELD,
        ROLLER_CHIP,
        ROLLER_GAP,
        Role::SurfaceContainerHighest,
    );
    ui.stroke(
        bx,
        cy,
        ROLLER_COUNT_FIELD,
        ROLLER_CHIP,
        ROLLER_GAP,
        if on || typing {
            Role::Primary
        } else {
            Role::OutlineVariant
        },
    );
    let (shown, color) = if r.typed().is_empty() {
        (EN.roller_number, muted)
    } else {
        (r.typed(), ink)
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
