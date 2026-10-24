//! The five-dice roller as it is drawn (`docs/CODEX32.md` §5.5): "Or let
//! Faraday roll", "Rolled by this computer" over five numbered dice, the
//! Roll button, and what one click rolls. Drawing reads the roller; it
//! works nothing out.

use super::five::{Act, FiveRoller, REACHES, Reach};
use super::screen::pips;
use crate::Action;
use crate::strings::{EN, fill};
use crate::ui::{Style, Ui, W, pal::*};
use faraday_draw::tokens::{
    ROLLER_BOUNCE, ROLLER_CHIP, ROLLER_DIE, ROLLER_DIE_RADIUS, ROLLER_FIVE_DIE, ROLLER_FIVE_GAP,
    ROLLER_GAP, ROLLER_LABEL, ROLLER_PIP, ROLLER_PIP_OFFSET, ROLLER_SWAY,
};

/// One die with its top-left at (`x`, `y`), showing `face` (0: blank),
/// lifted by `lift` and swayed by `sway` while it tumbles.
fn die(ui: &mut Ui, (x, y): (f32, f32), face: u8, (lift, sway): (f32, f32), tumbling: bool) {
    let scale = ROLLER_FIVE_DIE / ROLLER_DIE;
    let (x, y) = (x + sway, y - lift);
    ui.fill(
        x,
        y,
        ROLLER_FIVE_DIE,
        ROLLER_FIVE_DIE,
        ROLLER_DIE_RADIUS,
        INNER,
    );
    ui.stroke(
        x,
        y,
        ROLLER_FIVE_DIE,
        ROLLER_FIVE_DIE,
        ROLLER_DIE_RADIUS,
        if tumbling { ACCENT } else { BORDER },
    );
    let (cx, cy) = (x + ROLLER_FIVE_DIE / 2.0, y + ROLLER_FIVE_DIE / 2.0);
    for &(dx, dy) in pips(face) {
        ui.dot(
            cx + dx * ROLLER_PIP_OFFSET * scale,
            cy + dy * ROLLER_PIP_OFFSET * scale,
            ROLLER_PIP * scale,
            TEXT,
        );
    }
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

/// The label of a reach.
fn label(r: Reach) -> &'static str {
    match r {
        Reach::Pass => EN.roller_reach_pass,
        Reach::Character => EN.roller_reach_character,
        Reach::Share => EN.roller_reach_share,
        Reach::End => EN.roller_to_the_end,
    }
}

/// The roller at (`x`, `y`) in `w` units. `can_roll` is whether the
/// entry has anything left to take. `act` is the host's action for each
/// of the roller's. Returns the height used.
pub fn draw(
    ui: &mut Ui,
    (x, y, w): (f32, f32, f32),
    r: &FiveRoller,
    can_roll: bool,
    act: impl Fn(Act) -> Action,
) -> f32 {
    let mut cy = y;
    ui.text(
        x,
        cy,
        ROLLER_LABEL + 1.0,
        W::S,
        TEXT,
        EN.roller_or_let_faraday_roll,
    );
    cy += ui.line(ROLLER_LABEL + 1.0, W::S) + ROLLER_GAP / 2.0;
    ui.text(x, cy, ROLLER_LABEL, W::R, MUTED, EN.roller_by_this_computer);
    cy += ui.line(ROLLER_LABEL, W::R) + ROLLER_GAP;
    // The dice stand on the line the room for their bounce leaves.
    let p = r.progress();
    let lift = ROLLER_BOUNCE * (p * std::f32::consts::PI).sin();
    let sway = ROLLER_SWAY * (p * std::f32::consts::TAU).sin() * (1.0 - p);
    cy += ROLLER_BOUNCE;
    for d in 0..5 {
        let dx = x + d as f32 * (ROLLER_FIVE_DIE + ROLLER_FIVE_GAP);
        let moving = r.tumbling(d);
        let (l, s) = if moving { (lift, sway) } else { (0.0, 0.0) };
        die(ui, (dx, cy), r.shown(d), (l, s), moving);
        let n = (d + 1).to_string();
        let name = fill(EN.roller_die, &[&n]);
        let nw = ui.measure(ROLLER_LABEL, W::R, &name);
        ui.text(
            dx + (ROLLER_FIVE_DIE - nw) / 2.0,
            cy + ROLLER_FIVE_DIE + ROLLER_GAP / 2.0,
            ROLLER_LABEL,
            W::R,
            MUTED,
            &name,
        );
    }
    cy += ROLLER_FIVE_DIE + ROLLER_GAP / 2.0 + ui.line(ROLLER_LABEL, W::R) + ROLLER_GAP;
    let (text, style, action) = if r.rolling() {
        (EN.roller_stop, Style::Secondary, act(Act::Stop))
    } else if can_roll {
        (EN.roller_roll, Style::Primary, act(Act::Roll))
    } else {
        (EN.roller_roll, Style::Disabled, act(Act::Roll))
    };
    ui.button(x, cy, None, ROLLER_CHIP, text, style, action);
    cy += ROLLER_CHIP + ROLLER_GAP;
    ui.text(
        x,
        cy,
        ROLLER_LABEL,
        W::R,
        MUTED,
        EN.roller_per_click_codex32,
    );
    cy += ui.line(ROLLER_LABEL, W::R) + ROLLER_GAP / 2.0;
    let mut bx = x;
    for reach in REACHES {
        let name = label(reach);
        let bw = ui.measure(ROLLER_LABEL + 1.0, W::S, name) + ROLLER_GAP + ROLLER_GAP + ROLLER_GAP;
        if bx > x && bx + bw > x + w {
            bx = x;
            cy += ROLLER_CHIP + ROLLER_GAP / 2.0;
        }
        let style = if r.reach() == reach {
            Style::Primary
        } else {
            Style::Secondary
        };
        bx += ui.button(
            bx,
            cy,
            Some(bw),
            ROLLER_CHIP,
            name,
            style,
            act(Act::Reach(reach)),
        ) + ROLLER_GAP / 2.0;
    }
    cy += ROLLER_CHIP;
    cy - y
}

/// What the snapshot tour draws of the roller alone, before a flow hosts
/// it: on the page's background, `width` × `height` pixels at `dpi`. The
/// frame's pixels, RGBA.
#[cfg(feature = "testkit")]
pub fn preview(r: &FiveRoller, can_roll: bool, width: u16, height: u16, dpi: u16) -> Vec<u8> {
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
    let mut hits: Vec<(faraday_draw::Rect, Action)> = Vec::new();
    let (w, h) = (f32::from(width) / f, f32::from(height) / f);
    {
        let mut ui = Ui::new(&mut canvas, f, &mut hits, None);
        ui.fill(0.0, 0.0, w, h, 0.0, BG);
        draw(&mut ui, (PAD, PAD, w - 2.0 * PAD), r, can_roll, |_| {
            Action::ReduceMotion(false)
        });
    }
    canvas.frame().rgba.to_vec()
}
