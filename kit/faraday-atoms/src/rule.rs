//! Rules (09 §3.2).

use faraday_draw::Rect;
use faraday_tokens::palette::Paint;

use crate::Pressed;
use crate::ui::Ui;
use faraday_tokens::palette::Role;

/// A rule (04 §12.1): drawn from the theme alone.
pub struct RuleModel;

/// Draws a horizontal rule `w` wide at (x, y), design units.
pub fn draw<A: Pressed>(_m: &RuleModel, ui: &mut Ui<'_, A>, (x, y, w): (f32, f32, f32)) {
    ui.rule(x, y, w, Role::OutlineVariant);
}

impl<A: Pressed> Ui<'_, A> {
    /// A horizontal rule.
    pub fn rule(&mut self, x: f32, y: f32, w: f32, color: impl Into<Paint>) {
        let color: Paint = color.into();
        let color = self.col(self.solid(color));
        let r = self.rect(x, y, w, 1.0);
        let rect = Rect::new(r.x, r.y, r.w, 1.max(self.px(1.0)));
        self.c.fill_rect(rect, color);
    }

    /// A vertical rule: one unit wide, at least a pixel.
    pub fn vrule(&mut self, x: f32, y: f32, h: f32, color: impl Into<Paint>) {
        let color: Paint = color.into();
        let color = self.col(self.solid(color));
        let r = self.rect(x, y, 1.0, h);
        let rect = Rect::new(r.x, r.y, 1.max(self.px(1.0)), r.h);
        self.c.fill_rect(rect, color);
    }
}
