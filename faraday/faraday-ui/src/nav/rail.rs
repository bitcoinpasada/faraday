//! The navigation rail (`docs/LAYOUT.md` §3, DESIGN §4.1): `RAIL_W` at
//! the window's left, its destinations from the top, those opened then
//! closed under a divider, and what ends the session at its foot. And
//! the expanded rail's destination row: icon and label in a row, the
//! current one on a pill.

use super::{Dest, item};
use crate::Ui;
use crate::pal::*;
use faraday_draw::tokens::{
    GAP, GRID, ICON_SMALL, LABEL_L, LABEL_M, NAV_ICON, NAV_ICON_GAP, NAV_ITEM_H, NAV_RAIL_TOP, PAD,
    RAIL_W, TOUCH,
};

/// What a rail holds.
pub struct Rail<'s, A> {
    /// The destinations, from the top.
    pub top: &'s [Dest<'s, A>],
    /// Under a divider: what is opened, then closed (Learn, Settings).
    pub under: &'s [Dest<'s, A>],
    /// At its foot (Lock, Power off).
    pub foot: &'s [Dest<'s, A>],
}

/// The rail down the window's left, `h` tall, on the surface as the
/// page is (Material's navigation rail, `docs/THEMES.md` §7), a divider
/// at its edge. Each destination takes `NAV_ITEM_H` where the rail has
/// room and never less than a touch target, so a low window keeps every
/// one in reach.
pub fn rail<A: Copy + PartialEq>(ui: &mut Ui<A>, h: f32, r: &Rail<A>) {
    ui.fill(0.0, 0.0, RAIL_W, h, 0.0, Role::Surface);
    ui.fill(RAIL_W - 1.0, 0.0, 1.0, h, 0.0, Role::OutlineVariant);
    let n = r.top.len() + r.under.len() + r.foot.len();
    let divider = if r.under.is_empty() { 0.0 } else { 2.0 * GAP };
    let room = h - 2.0 * NAV_RAIL_TOP - divider;
    // Taller by what its label's line grows (`docs/TEXT-SIZE.md` §5).
    let ih = (room / n.max(1) as f32).clamp(TOUCH, NAV_ITEM_H + ui.line_grown_as(LABEL_M));
    let mut y = NAV_RAIL_TOP;
    for d in r.top {
        item(ui, 0.0, y, RAIL_W, ih, d);
        y += ih;
    }
    if !r.under.is_empty() {
        ui.rule(PAD, y + GAP, RAIL_W - 2.0 * PAD, Role::OutlineVariant);
        y += divider;
        for d in r.under {
            item(ui, 0.0, y, RAIL_W, ih, d);
            y += ih;
        }
    }
    // The foot from the bottom up, never over what is above it.
    let foot_top = (h - NAV_RAIL_TOP - r.foot.len() as f32 * ih).max(y);
    for (i, d) in r.foot.iter().enumerate() {
        item(ui, 0.0, foot_top + i as f32 * ih, RAIL_W, ih, d);
    }
}

/// An expanded rail's destination `d` at `(x, y, w)`, `w` wide and a
/// touch target tall: its icon and label in a row, on a pill while it is
/// current, and `end` at its right end in the label's colour. The whole row
/// is pressable.
pub fn destination_row<A: Copy + PartialEq>(
    ui: &mut Ui<A>,
    (x, y, w): (f32, f32, f32),
    d: &Dest<A>,
    end: Option<&str>,
) {
    let (icon, label, current, action) = (d.icon, d.label, d.current, d.action);
    let r = TOUCH / 2.0;
    if current {
        ui.fill(x, y, w, TOUCH, r, Role::SecondaryContainer);
    }
    let fg = if current {
        Role::OnSecondaryContainer
    } else {
        Role::OnSurfaceVariant
    };
    let pill = (x, y, w, TOUCH, r);
    ui.state_layer(pill, fg.paint(), action);
    let ix = x + PAD;
    let iy = y + (TOUCH - ICON_SMALL) / 2.0;
    ui.icon(ix, iy, ICON_SMALL, icon, NAV_ICON, fg);
    let inset = (ICON_SMALL - NAV_ICON) / 2.0;
    super::badge(ui, (ix + inset, iy + inset, NAV_ICON), d.badge);
    let tx = ix + ICON_SMALL + NAV_ICON_GAP;
    let right = x + w - PAD;
    let end_w = end.map_or(0.0, |e| ui.measure_as(LABEL_M.regular(), e) + GRID);
    // Material's label large, the current destination's heavier.
    let role = if current { LABEL_L } else { LABEL_L.regular() };
    // A one-line slot: larger text fits down to Standard, and is
    // shortened there only where it still does not fit
    // (`docs/TEXT-SIZE.md` §3).
    let room = right - tx - end_w;
    ui.text_fit_as((tx, y, TOUCH, room), role, fg, label);
    if let Some(e) = end {
        ui.text_right_as((right, y, TOUCH), LABEL_M.regular(), fg, e);
    }
    ui.hit_ringed(pill, (x, y, w, TOUCH), action);
}
