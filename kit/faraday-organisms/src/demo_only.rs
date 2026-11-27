//! The "Demo only" card (09 §3.3): what a page or a sheet offers only in Practice
//! (`docs/DEMO.md` §14): a sample, the demo's passphrase, a line saying what Practice puts in
//! place of the person's own. A device-composite page in Practice hands each item to the model
//! where it would have drawn it; nothing demo-only is drawn in any other page.

use faraday_atoms::button::{self, ButtonModel};
use faraday_atoms::{Offered, Pressed, Shown, Ui};
use faraday_tokens::dims::{BODY_S, BUTTON_H, GAP_SMALL, PAD, SHAPE_M};
use faraday_tokens::palette::Role;

/// What a page hands over for the demo.
pub enum DemoItem<'s, A> {
    /// A button: its label and its press.
    Button(ButtonModel<'s, A>),
    /// A line saying what Practice does in place of the person's own.
    Line(Shown<'s>),
}

/// The items a page offers only in Practice, and the card's name.
pub struct DemoOnlyModel<'s, A> {
    /// The card's name.
    pub title: Shown<'s>,
    /// The items, in the order handed over.
    pub items: Vec<DemoItem<'s, A>>,
}

impl<'s, A: Pressed> DemoOnlyModel<'s, A> {
    /// A card with no items.
    pub fn new(title: Shown<'s>) -> Self {
        DemoOnlyModel {
            title,
            items: Vec::new(),
        }
    }

    /// Hands the card a demo button, once however often the page offers it: one that presses
    /// what an item already presses is not added again.
    pub fn offer(&mut self, button: ButtonModel<'s, A>) {
        let press = |b: &ButtonModel<'_, A>| match &b.offered {
            Offered::Yes(a) | Offered::Refused(a, _) => Some(a.clone()),
            Offered::No => None,
        };
        let known = self.items.iter().any(|i| match i {
            DemoItem::Button(b) => press(b).is_some() && press(b) == press(&button),
            DemoItem::Line(_) => false,
        });
        if !known {
            self.items.push(DemoItem::Button(button));
        }
    }

    /// Hands the card a demo line, once however often the page offers it.
    pub fn line(&mut self, text: Shown<'s>) {
        let known = self
            .items
            .iter()
            .any(|i| matches!(i, DemoItem::Line(l) if l.as_str() == text.as_str()));
        if !known {
            self.items.push(DemoItem::Line(text));
        }
    }
}

/// Draws the card's name and items from (x, y), `w` wide. Returns the height they take.
fn items<A: Pressed>(
    m: &DemoOnlyModel<'_, A>,
    ui: &mut Ui<'_, A>,
    (x, y, w): (f32, f32, f32),
) -> f32 {
    let mut cy = y;
    ui.showing(&m.title, |ui| {
        ui.text_as(x, cy, BODY_S, Role::OnSurfaceVariant, m.title.as_str());
        cy += ui.advance_as(BODY_S, 22.0);
    });
    for i in &m.items {
        match i {
            DemoItem::Line(t) => {
                cy += ui.showing(t, |ui| {
                    ui.wrap_as((x, cy, w), BODY_S, Role::OnSurface, t.as_str())
                }) + GAP_SMALL;
            }
            DemoItem::Button(b) => {
                let h = ui.button_height(BUTTON_H);
                button::draw(b, ui, (x, cy, Some(w), h));
                cy += h + GAP_SMALL;
            }
        }
    }
    cy - y
}

/// Draws the card from (x, y), `w` wide: its name, then each item, on a container. Nothing when
/// it holds no item. Returns the height it took, with no gap under it.
pub fn draw<A: Pressed>(
    m: &DemoOnlyModel<'_, A>,
    ui: &mut Ui<'_, A>,
    (x, y, w): (f32, f32, f32),
) -> f32 {
    if m.items.is_empty() {
        return 0.0;
    }
    let inner = ui.measured(|ui| items(m, ui, (0.0, 0.0, w - 2.0 * PAD)));
    let h = inner + 2.0 * PAD - GAP_SMALL;
    ui.fill(x, y, w, h, SHAPE_M, Role::SurfaceContainerLow);
    items(m, ui, (x + PAD, y + PAD, w - 2.0 * PAD));
    h
}

#[cfg(test)]
mod tests {
    use super::*;
    use faraday_atoms::ui::testing::{P, at_rest, canvas, frame};
    use faraday_atoms::{Offered, Style};

    fn button(p: u8) -> ButtonModel<'static, P> {
        ButtonModel {
            label: Shown::plain("Use sample"),
            style: Style::Tonal,
            offered: Offered::Yes(P(p)),
            chosen: false,
            icon: None,
        }
    }

    /// Invariant: A page that offers the same Practice button on every frame leaves one button on the card, and a card with nothing offered draws nothing.
    /// Regression: The card grows a button each frame the page offers it, or an empty card is drawn as an empty box.
    /// Owner: kit
    #[test]
    fn the_card_holds_each_offer_once_and_draws_nothing_when_empty() {
        let mut m = DemoOnlyModel::new(Shown::plain("Demo only"));
        let mut c = canvas(160);
        let empty = frame(&mut c, &at_rest(None), |ui| {
            assert_eq!(draw(&m, ui, (0.0, 0.0, 300.0)), 0.0);
        });
        assert!(empty.hits.is_empty());
        for _ in 0..3 {
            m.offer(button(1));
            m.line(Shown::plain("Practice uses a sample"));
        }
        assert_eq!(m.items.len(), 2);
        let out = frame(&mut c, &at_rest(None), |ui| {
            assert!(draw(&m, ui, (0.0, 0.0, 300.0)) > 0.0);
        });
        assert_eq!(out.hits.len(), 1);
    }
}
