//! The floating action button (09 §3.2).

use faraday_tokens::palette::Role;

use crate::Pressed;
use crate::hits::Shape;
use crate::offered::Offered;
use crate::shown::Shown;
use crate::ui::Ui;
use faraday_draw::icons::Icon;

/// The floating action button: nothing is drawn where it is not offered.
pub struct FabModel<'s, A> {
    pub icon: Icon,
    pub label: Shown<'s>,
    pub offered: Offered<A>,
}

/// Draws the button with its bottom-right corner at (right, bottom), design units. Returns its
/// width, 0 where none is drawn.
pub fn draw<A: Pressed>(
    m: &FabModel<'_, A>,
    ui: &mut Ui<'_, A>,
    (right, bottom): (f32, f32),
) -> f32 {
    ui.showing(&m.label, |ui| {
        ui.offered(&m.offered, |ui, press, off| match press {
            Some(action) if !off => ui.fab(right, bottom, m.icon, m.label.as_str(), action),
            _ => 0.0,
        })
    })
}

impl<A: Pressed> Ui<'_, A> {
    /// A floating action: a pill over the page in Material's primary FAB
    /// colour style (filled in primary, its icon and its word in
    /// on-primary; F138), its bottom-right corner at (`right`, `bottom`).
    /// Returns its width.
    pub fn fab(&mut self, right: f32, bottom: f32, icon: Icon, label: &str, action: A) -> f32 {
        // Smaller on a small panel, where it floats over more of the page.
        use faraday_tokens::dims::{
            FAB_H, FAB_H_COMPACT, FAB_PAD, FAB_PAD_COMPACT, ICON_GAP, ICON_GLYPH, LABEL_L,
        };
        let (h, pad) = if self.compact {
            (FAB_H_COMPACT, FAB_PAD_COMPACT)
        } else {
            (FAB_H, FAB_PAD)
        };
        let lead = ICON_GLYPH + ICON_GAP;
        let width = self.measure_as(LABEL_L, label) + 2.0 * pad + lead;
        let (x, y) = (right - width, bottom - h);
        self.fab_at = Some((x + self.ox, y + self.oy, width, h));
        self.fab_pill((x, y, width, h), pad, icon, Some(label), action);
        width
    }

    /// The primary FAB's body: `(x, y, width, h)`, its icon `pad` in, and
    /// its word after the icon when it has one.
    fn fab_pill(
        &mut self,
        (x, y, width, h): (f32, f32, f32, f32),
        pad: f32,
        icon: Icon,
        label: Option<&str>,
        action: A,
    ) {
        use faraday_tokens::dims::{ICON_GAP, ICON_GLYPH, LABEL_L, SHAPE_L};
        let lead = ICON_GLYPH + ICON_GAP;
        let level = if self.pointer_over(x, y, width, h) && !self.answering {
            faraday_tokens::dims::ELEVATION_RAISED
        } else {
            faraday_tokens::dims::ELEVATION_FAB
        };
        self.shadow(x, y, width, h, SHAPE_L, level);
        self.fill(x, y, width, h, SHAPE_L, Role::Primary);
        self.press(
            Shape::new(x, y, width, h, SHAPE_L),
            Role::OnPrimary.paint(),
            action,
        );
        // It is drawn last of its area: its whole grown target is its
        // own, and no control scrolled under it takes a share.
        if let Some(&i) = self.tab.last() {
            let clip = self.seen_clip();
            self.hits[i].target = self.grown(self.hits[i].rect, clip);
        }
        let iy = y + (h - ICON_GLYPH) / 2.0;
        self.icon(x + pad, iy, ICON_GLYPH, icon, ICON_GLYPH, Role::OnPrimary);
        if let Some(label) = label {
            self.text_mid_as((x + pad + lead, y, h), LABEL_L, Role::OnPrimary, label);
        }
    }
}
