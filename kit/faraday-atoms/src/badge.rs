//! Badges and pips (09 §3.2).

use faraday_tokens::palette::Role;

use crate::Pressed;
use crate::shown::Shown;
use crate::ui::Ui;
use faraday_draw::icons::Icon;

/// A step's number badge: done, open or neither.
pub struct BadgeModel<'s> {
    pub label: Shown<'s>,
    pub done: bool,
    pub open: bool,
}

/// Draws the badge with its top-left at (x, y), design units.
pub fn draw<A: Pressed>(m: &BadgeModel<'_>, ui: &mut Ui<'_, A>, (x, y): (f32, f32)) {
    ui.showing(&m.label, |ui| {
        ui.badge(x, y, m.label.as_str(), m.done, m.open)
    });
}

impl<A: Pressed> Ui<'_, A> {
    /// Threshold pips: `filled` of `n`.
    pub fn pips(&mut self, x: f32, y: f32, n: usize, filled: usize) -> f32 {
        use faraday_tokens::dims::{PIP_GAP, PIP_H, PIP_W};
        let mut cx = x;
        for i in 0..n {
            if i < filled {
                self.fill(cx, y, PIP_W, PIP_H, PIP_H / 2.0, Role::Ok);
            } else {
                self.stroke(cx, y, PIP_W, PIP_H, PIP_H / 2.0, Role::Outline);
            }
            cx += PIP_W + PIP_GAP;
        }
        cx - x - PIP_GAP
    }

    /// A numbered or ticked step badge.
    pub fn badge(&mut self, x: f32, y: f32, label: &str, done: bool, open: bool) {
        let (bg, fg) = if done {
            (Role::OkContainer, Role::OnOkContainer)
        } else if open {
            (Role::Primary, Role::OnPrimary)
        } else {
            (Role::SurfaceContainerHighest, Role::OnSurfaceVariant)
        };
        use faraday_tokens::dims::{LABEL_M, STEP_BADGE as D, STEP_BADGE_TICK};
        let r = D / 2.0;
        self.dot(x + r, y + r, r, bg);
        if done {
            self.icon(x, y, D, Icon::Done, STEP_BADGE_TICK, fg);
        } else {
            let tw = self.measure_as(LABEL_M, label);
            self.text_mid_as((x + r - tw / 2.0, y, D), LABEL_M, fg, label);
        }
    }
}
