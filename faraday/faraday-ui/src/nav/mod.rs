//! Navigation by window class (`docs/LAYOUT.md` §3): Material 3's
//! navigation rail, navigation bar, top app bar and the expanded rail's
//! destination row, drawn in Faraday's look. Each draws what it is given
//! and names the press each region takes; which destinations there are,
//! and which is current, is the app's.
//!
//! The roles are Material's (`docs/THEMES.md` §7): the current
//! destination's pill is secondary-container, its icon on-secondary-
//! container and its label on-surface; the others are on-surface-variant;
//! hover and press are state layers, and the focus ring follows the
//! shape the state layer takes. The bars carry no rule along their
//! edge (Material's top app bar and navigation bar); the rail's divider
//! is outline-variant.

mod app_bar;
mod bar;
mod chip;
mod rail;

pub use app_bar::{
    AppBar, app_bar, app_bar_fill, bar_title_one_line, bar_title_whole, close_button,
    close_button_in, fill_behind, help, icon_action, icon_action_at, more, pill_top,
};
pub use bar::bar;
pub use chip::{Chip, status_chip};
pub use rail::{Rail, RailFab, destination_row, rail};

use crate::pal::*;
use crate::{Glyph, Shape, Ui, W};
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
    /// How many flows in progress stand there, on its icon too.
    pub flows: usize,
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
/// its top right, as Material places them, in Bitcoin orange with a white count, in every theme (F311).
pub fn badge<A: Copy + PartialEq>(ui: &mut Ui<A>, glyph: (f32, f32, f32), badge: Badge) {
    let (gx, gy, gs) = glyph;
    match badge {
        Badge::None | Badge::Count(0) => {}
        Badge::Dot => {
            let d = BADGE_SMALL;
            let (x, y) = (gx + gs - d / 2.0, gy - d / 2.0);
            ui.fill(x, y, d, d, d / 2.0, crate::pal::BADGE);
            let r = ui.rect(x, y, d, d);
            ui.c.note(faraday_draw::canvas::InkKind::Dot, r);
        }
        Badge::Count(n) => {
            let label = count_label(n);
            let at = (gx + gs / 2.0, gy - BADGE_LARGE / 2.0);
            count_pill(ui, at, &label, crate::pal::BADGE, crate::pal::ON_BADGE);
        }
    }
}

/// A large badge's count as it is written.
fn count_label(n: usize) -> String {
    if n > 999 {
        EN.nav_badge_many.to_string()
    } else {
        n.to_string()
    }
}

/// A large badge's width for `label`.
fn count_width<A: Copy + PartialEq>(ui: &mut Ui<A>, label: &str) -> f32 {
    (ui.measure(BADGE_LABEL, W::S, label) + 2.0 * BADGE_PAD).max(BADGE_LARGE)
}

/// A large badge's pill at `(x, y)`, `label` centred in it.
fn count_pill<A: Copy + PartialEq>(
    ui: &mut Ui<A>,
    (x, y): (f32, f32),
    label: &str,
    fill: impl Into<Paint>,
    ink: impl Into<Paint>,
) {
    let h = BADGE_LARGE;
    let w = count_width(ui, label);
    ui.fill(x, y, w, h, h / 2.0, fill.into());
    let tw = ui.measure(BADGE_LABEL, W::S, label);
    ui.text_mid(
        x + (w - tw) / 2.0,
        y,
        h,
        BADGE_LABEL,
        W::S,
        ink.into(),
        label,
    );
}

/// The mark of `flows` in progress on a rail's or the bar's icon: a pill
/// in the primary colour at the corner, beside the destination's own
/// badge `own` where it has one (`docs/WORKSPACE.md` §16).
fn flow_pill<A: Copy + PartialEq>(
    ui: &mut Ui<A>,
    (gx, gy, gs): (f32, f32, f32),
    own: Badge,
    flows: usize,
) {
    if flows == 0 {
        return;
    }
    let corner = gx + gs / 2.0;
    let x = match own {
        Badge::None | Badge::Count(0) => corner,
        Badge::Dot => gx + gs + BADGE_SMALL / 2.0 + GAP_SMALL,
        Badge::Count(n) => corner + count_width(ui, &count_label(n)) + GAP_SMALL,
    };
    let at = (x, gy - BADGE_LARGE / 2.0);
    count_pill(ui, at, &count_label(flows), Role::Primary, Role::OnPrimary);
}

/// The height a destination's pill and label take together, its label's
/// line at the text size.
fn item_content<A: Copy + PartialEq>(ui: &Ui<A>) -> f32 {
    NAV_PILL_H + NAV_LABEL_GAP + ui.line_as(LABEL_M)
}

/// A destination as the rail and the bar draw it: its icon on a pill
/// over its label, centred in the box at (x, y), `w` by `h`, the whole
/// box pressable.
fn item<A: Copy + PartialEq>(ui: &mut Ui<A>, x: f32, y: f32, w: f32, h: f32, d: &Dest<A>) {
    let top = y + ((h - item_content(ui)) / 2.0).max(0.0);
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
    // Ringed round its pill, the shape its state layer takes.
    let shape = Shape::new(pill.0, pill.1, pill.2, pill.3, pill.4);
    ui.press_to(shape, (x, y, w, h), icon_fg.paint(), d.action);
    let ix = x + (w - NAV_PILL_H) / 2.0;
    ui.icon(ix, top, NAV_PILL_H, d.icon, NAV_ICON, icon_fg);
    let inset = (NAV_PILL_H - NAV_ICON) / 2.0;
    let glyph = (ix + inset, top + inset, NAV_ICON);
    badge(ui, glyph, d.badge);
    flow_pill(ui, glyph, d.badge, d.flows);
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
}
