//! The checkbox (09 §3.2).

use faraday_tokens::palette::Role;

use crate::Pressed;
use crate::offered::Offered;
use crate::ui::Ui;
use faraday_draw::icons::Icon;

/// The checkbox: its press is the row's, so the model says whether it is enabled.
pub struct CheckModel<'s, A> {
    pub on: bool,
    pub offered: Offered<A>,
    /// The row's label, which the row draws.
    pub label: core::marker::PhantomData<&'s ()>,
}

/// Draws the box with its top-left at (x, y), design units.
pub fn draw<A: Pressed>(m: &CheckModel<'_, A>, ui: &mut Ui<'_, A>, (x, y): (f32, f32)) {
    ui.checkbox(x, y, m.on, !matches!(m.offered, Offered::No));
}

impl<A: Pressed> Ui<'_, A> {
    /// A checkbox. One ticked and not enabled shows its tick dimmed: it is
    /// done, and a press does not undo it.
    pub fn checkbox(&mut self, x: f32, y: f32, on: bool, enabled: bool) {
        use faraday_tokens::dims::{ICON_GLYPH_BUTTON as SIDE, SHAPE_CHECKBOX as R};
        if on {
            // Not enabled, it is done and a press does not undo it:
            // on-surface at the disabled opacity, its tick in surface
            // (Material's disabled checkbox).
            let (fill, tick) = if enabled {
                (Role::Primary.paint(), Role::OnPrimary.paint())
            } else {
                (
                    Role::OnSurface.at(faraday_tokens::dims::DISABLED_CONTENT),
                    Role::Surface.paint(),
                )
            };
            self.wash(x, y, SIDE, SIDE, R, fill);
            self.icon(
                x,
                y,
                SIDE,
                Icon::Done,
                faraday_tokens::dims::CHECK_TICK,
                tick,
            );
        } else {
            let edge = self.col(if enabled {
                Role::OnSurfaceVariant.paint()
            } else {
                Role::OnSurface.at(faraday_tokens::dims::DISABLED_CONTENT)
            });
            let rect = self.rect(x, y, SIDE, SIDE);
            self.c
                .stroke_rounded_rect(rect, R * self.f, 2.0 * self.f, edge);
        }
    }
}
