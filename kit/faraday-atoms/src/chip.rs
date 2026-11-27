//! Chips (09 §3.2).

use faraday_tokens::palette::Paint;

use crate::Pressed;
use crate::hits::Shape;
use crate::offered::Offered;
use crate::shown::Shown;
use crate::ui::Ui;

/// A chip in a content and a fill colour, that takes a press when one is offered.
pub struct ChipModel<'s, A> {
    pub label: Shown<'s>,
    pub fg: Paint,
    pub bg: Paint,
    pub offered: Option<Offered<A>>,
}

/// Draws the chip with its top-left at (x, y), design units. Returns its width.
pub fn draw<A: Pressed>(m: &ChipModel<'_, A>, ui: &mut Ui<'_, A>, (x, y): (f32, f32)) -> f32 {
    let label = m.label.as_str();
    ui.showing(m.label, |ui| match &m.offered {
        Some(offered) => ui.offered(offered, |ui, press, _| {
            ui.chip_press(x, y, label, (m.fg, m.bg), press)
        }),
        None => ui.chip_press(x, y, label, (m.fg, m.bg), None),
    })
}

impl<A: Pressed> Ui<'_, A> {
    /// A rounded label with a dot, in any two paints: [`Ui::tag`] for a
    /// status. Returns its width.
    pub fn chip(
        &mut self,
        x: f32,
        y: f32,
        label: &str,
        fg: impl Into<Paint>,
        bg: impl Into<Paint>,
    ) -> f32 {
        self.chip_press(x, y, label, (fg.into(), bg.into()), None)
    }

    /// [`Ui::chip`] that a press takes when `action` is given: its state
    /// layer in its content's colour, between its fill and its dot and
    /// text (Material: the layer sits between container and content).
    /// Returns its width.
    pub fn chip_press(
        &mut self,
        x: f32,
        y: f32,
        label: &str,
        (fg, bg): (Paint, Paint),
        action: Option<A>,
    ) -> f32 {
        use faraday_tokens::dims::{BODY_S, TAG_DOT_R, TAG_DOT_TEXT_X, TAG_H as H};
        let width = self.chip_width(label);
        self.fill(x, y, width, H, H / 2.0, bg);
        if let Some(a) = action {
            self.press(Shape::new(x, y, width, H, H / 2.0), fg, a);
        }
        self.dot(x + H / 2.0, y + H / 2.0, TAG_DOT_R, fg);
        self.text_mid_as((x + TAG_DOT_TEXT_X, y, H), BODY_S, fg, label);
        width
    }

    /// How wide [`Ui::chip`] draws `label`.
    pub fn chip_width(&self, label: &str) -> f32 {
        use faraday_tokens::dims::{BODY_S, TAG_DOT_TEXT_X, TAG_PAD};
        self.measure_as(BODY_S, label) + TAG_DOT_TEXT_X + TAG_PAD
    }
}
