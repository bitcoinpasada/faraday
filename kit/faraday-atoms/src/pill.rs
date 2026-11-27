//! Pills (09 §3.2).

use faraday_tokens::palette::{Paint, Role};

use crate::Pressed;
use crate::hits::Shape;
use crate::offered::Offered;
use crate::shown::Shown;
use crate::ui::Ui;
use faraday_tokens::dims::TAG_H;

/// A pill: an outlined label, or a word that a press takes when one is offered.
pub struct PillModel<'s, A> {
    pub label: Shown<'s>,
    pub offered: Option<Offered<A>>,
}

/// Draws the pill with its top-left at (x, y), design units. Returns its width.
pub fn draw<A: Pressed>(m: &PillModel<'_, A>, ui: &mut Ui<'_, A>, (x, y): (f32, f32)) -> f32 {
    let label = m.label.as_str();
    ui.showing(&m.label, |ui| match &m.offered {
        Some(offered) => ui.offered(offered, |ui, press, _| match press {
            Some(word) => ui.word_pill(x, y, label, word),
            None => ui.pill(x, y, TAG_H, label),
        }),
        None => ui.pill(x, y, TAG_H, label),
    })
}

impl<A: Pressed> Ui<'_, A> {
    /// A word in a pill that a press opens in its list, where words are
    /// shown as they are made. Returns its width.
    pub fn word_pill(&mut self, x: f32, y: f32, word: &str, action: A) -> f32 {
        use faraday_tokens::dims::{BODY_M, SHAPE_S, TAG_H as h, TAG_PAD};
        let role = BODY_M.mono();
        let width = self.measure_as(role, word) + 2.0 * TAG_PAD;
        self.fill(x, y, width, h, SHAPE_S, Role::SurfaceContainerHighest);
        self.stroke(x, y, width, h, SHAPE_S, Role::Outline);
        self.text_mid_as((x + TAG_PAD, y, h), role, Role::OnSurface, word);
        self.press(
            Shape::new(x, y, width, h, SHAPE_S),
            Role::OnSurface.paint(),
            action,
        );
        width
    }

    /// A pill with a plain label and an edge. Returns its width.
    pub fn pill(&mut self, x: f32, y: f32, h: f32, label: &str) -> f32 {
        self.pill_in(x, y, h, label, Role::OnSurfaceVariant)
    }

    /// [`Ui::pill`] with its label in `color`.
    pub fn pill_in(&mut self, x: f32, y: f32, h: f32, label: &str, color: impl Into<Paint>) -> f32 {
        let color: Paint = color.into();
        use faraday_tokens::dims::{BODY_S, TAG_PAD};
        let width = self.measure_as(BODY_S, label) + 2.0 * TAG_PAD;
        // An assist chip (Material's): no container, an outline.
        self.stroke(x, y, width, h, h / 2.0, Role::Outline);
        self.text_mid_as((x + TAG_PAD, y, h), BODY_S, color, label);
        width
    }
}
