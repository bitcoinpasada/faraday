//! Navigation by window class (`docs/LAYOUT.md` §3): Material 3's
//! navigation rail, navigation bar, top app bar and the expanded rail's
//! destination row, drawn in Faraday's look. Each draws what it is given
//! and names the press each region takes; which destinations there are,
//! and which is current, is the app's.
//!
//! Stand-ins until faraday-ui has Material's state layers and outline
//! colour: the current destination's pill is `ACCENT` at a low alpha,
//! a pressed one `INNER`, and an edge `LINE` or `BORDER`.

mod app_bar;
mod bar;
mod chip;
mod rail;

pub use app_bar::{AppBar, app_bar, help, icon_action, more};
pub use bar::bar;
pub use chip::{Chip, status_chip};
pub use rail::{Rail, destination_row, rail};

use crate::pal::*;
use crate::{Glyph, Ui, W};
use faraday_draw::tokens::{CAPTION, GAP_SMALL, NAV_ICON, NAV_LABEL_GAP, NAV_PILL_H, NAV_PILL_W};

/// One destination: its icon, its label, the press it is, and whether it
/// is the one shown now.
#[derive(Debug, Clone, Copy)]
pub struct Dest<'s, A> {
    /// Its icon.
    pub icon: Glyph,
    /// Its label.
    pub label: &'s str,
    /// What a press on it does.
    pub action: A,
    /// It is the destination shown now.
    pub current: bool,
}

/// The height a destination's pill and label take together.
fn item_content() -> f32 {
    NAV_PILL_H + NAV_LABEL_GAP + CAPTION + GAP_SMALL
}

/// A destination as the rail and the bar draw it: its icon on a pill
/// over its label, centred in the box at (x, y), `w` by `h`, the whole
/// box pressable.
fn item<A: Copy + PartialEq>(ui: &mut Ui<A>, x: f32, y: f32, w: f32, h: f32, d: &Dest<A>) {
    let top = y + ((h - item_content()) / 2.0).max(0.0);
    let px = x + (w - NAV_PILL_W) / 2.0;
    let pressed = ui.is_pressed(d.action) || ui.is_hovered(d.action);
    if d.current {
        ui.fill(
            px,
            top,
            NAV_PILL_W,
            NAV_PILL_H,
            NAV_PILL_H / 2.0,
            ACCENT.with_alpha(48),
        );
    } else if pressed {
        ui.fill(px, top, NAV_PILL_W, NAV_PILL_H, NAV_PILL_H / 2.0, INNER);
    }
    let fg = if d.current { TEXT } else { MUTED };
    let ix = x + (w - NAV_PILL_H) / 2.0;
    ui.icon(ix, top, NAV_PILL_H, d.icon, NAV_ICON, fg);
    let face = if d.current { W::S } else { W::R };
    let label = ui.fit(CAPTION, face, d.label, w - 2.0 * GAP_SMALL);
    let lw = ui.measure(CAPTION, face, &label);
    let ly = top + NAV_PILL_H + NAV_LABEL_GAP;
    ui.text(x + (w - lw) / 2.0, ly, CAPTION, face, fg, &label);
    ui.hit_quiet(x, y, w, h, d.action);
}
