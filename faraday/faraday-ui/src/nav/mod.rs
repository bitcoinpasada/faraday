//! Navigation by window class (`docs/LAYOUT.md` §3): Material 3's
//! navigation rail, navigation bar, top app bar and the expanded rail's
//! destination row, drawn in Faraday's look. Each draws what it is given
//! and names the press each region takes; which destinations there are,
//! and which is current, is the app's.
//!
//! The roles are Material's (`docs/THEMES.md` §7): the current
//! destination's pill is secondary-container, its icon on-secondary-
//! container and its label on-surface; the others are on-surface-variant;
//! hover and press are state layers; an edge is outline-variant.

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
use faraday_draw::tokens::{
    BADGE_LABEL, BADGE_LARGE, BADGE_PAD, BADGE_SMALL, GAP_SMALL, LABEL_M, NAV_ICON, NAV_LABEL_GAP,
    NAV_PILL_H, NAV_PILL_W,
};
use faraday_strings::EN;

/// One destination: its icon, its label, the press it is, whether it
/// is the one shown now, and its badge.
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
    /// What waits there, on its icon.
    pub badge: Badge,
}

/// Material's badge on a destination's icon: a small dot where something
/// waits, or a large badge with a count.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Badge {
    /// None.
    #[default]
    None,
    /// The small badge: a dot.
    Dot,
    /// The large badge with a count, "999+" past 999 (Material's most
    /// characters, four).
    Count(usize),
}

/// Draws `badge` on an icon glyph whose box is `glyph` (x, y, side): the
/// dot on its top right corner, the large badge from its middle across
/// its top right, as Material places them, in the error colour.
pub fn badge<A: Copy + PartialEq>(ui: &mut Ui<A>, glyph: (f32, f32, f32), badge: Badge) {
    let (gx, gy, gs) = glyph;
    match badge {
        Badge::None | Badge::Count(0) => {}
        Badge::Dot => {
            let d = BADGE_SMALL;
            let (x, y) = (gx + gs - d / 2.0, gy - d / 2.0);
            ui.fill(x, y, d, d, d / 2.0, Role::Error);
            let r = ui.rect(x, y, d, d);
            ui.c.note(faraday_draw::canvas::InkKind::Dot, r);
        }
        Badge::Count(n) => {
            let label = if n > 999 {
                EN.nav_badge_many.to_string()
            } else {
                n.to_string()
            };
            let label = label.as_str();
            let h = BADGE_LARGE;
            let w = (ui.measure(BADGE_LABEL, W::S, label) + 2.0 * BADGE_PAD).max(h);
            let (x, y) = (gx + gs / 2.0, gy - h / 2.0);
            ui.fill(x, y, w, h, h / 2.0, Role::Error);
            let tw = ui.measure(BADGE_LABEL, W::S, label);
            ui.text_mid(
                x + (w - tw) / 2.0,
                y,
                h,
                BADGE_LABEL,
                W::S,
                Role::OnError,
                label,
            );
        }
    }
}

/// The height a destination's pill and label take together.
fn item_content() -> f32 {
    NAV_PILL_H + NAV_LABEL_GAP + LABEL_M.line
}

/// A destination as the rail and the bar draw it: its icon on a pill
/// over its label, centred in the box at (x, y), `w` by `h`, the whole
/// box pressable.
fn item<A: Copy + PartialEq>(ui: &mut Ui<A>, x: f32, y: f32, w: f32, h: f32, d: &Dest<A>) {
    let top = y + ((h - item_content()) / 2.0).max(0.0);
    let px = x + (w - NAV_PILL_W) / 2.0;
    let pill = (px, top, NAV_PILL_W, NAV_PILL_H, NAV_PILL_H / 2.0);
    if d.current {
        ui.fill(
            px,
            top,
            NAV_PILL_W,
            NAV_PILL_H,
            NAV_PILL_H / 2.0,
            Role::SecondaryContainer,
        );
    }
    let (icon_fg, fg) = if d.current {
        (Role::OnSecondaryContainer, Role::OnSurface)
    } else {
        (Role::OnSurfaceVariant, Role::OnSurfaceVariant)
    };
    ui.state_layer(pill, icon_fg.paint(), d.action);
    let ix = x + (w - NAV_PILL_H) / 2.0;
    ui.icon(ix, top, NAV_PILL_H, d.icon, NAV_ICON, icon_fg);
    let inset = (NAV_PILL_H - NAV_ICON) / 2.0;
    badge(ui, (ix + inset, top + inset, NAV_ICON), d.badge);
    // Material's label medium, the current destination's heavier.
    let role = if d.current {
        LABEL_M
    } else {
        LABEL_M.regular()
    };
    // A one-line slot: fitted down to Standard (`docs/TEXT-SIZE.md` §3).
    let room = w - 2.0 * GAP_SMALL;
    let lw = ui.text_fit_width(role, d.label, room);
    let ly = top + NAV_PILL_H + NAV_LABEL_GAP;
    // The band its own line tall, so the label's top is at `ly`.
    let band = (
        x + (w - lw) / 2.0,
        ly,
        ui.line(role.size, role.face.into()),
        room,
    );
    ui.text_fit_as(band, role, fg, d.label);
    ui.hit_quiet(x, y, w, h, d.action);
}
