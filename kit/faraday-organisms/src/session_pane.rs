//! The session pane organism (09 §3.3): what the session holds, drawn as This session's card
//! (`docs/SESSION-PANE.md`). Only the empty state is built: nothing held, the card with its
//! heading and the one line that says so. The groups (Keys, Wallets, the Waiting band), the pick
//! rows, the vault marks, the details and the scan in the pane are not.

use core::marker::PhantomData;

use faraday_atoms::{Pressed, Shown, Ui};
use faraday_tokens::dims::{
    BODY_L, ELEVATION_SHEET, GRID, HOME_CARD_GAP, HOME_CARD_HEAD, LIST_PAD, MENU_PAGE_W, PAD,
    SHAPE_M,
};
use faraday_tokens::palette::Role;

/// The pane's model: its heading and, while nothing is held, the line that says so.
pub struct SessionPaneModel<'s, A> {
    /// The card's heading.
    pub heading: Shown<'s>,
    /// The line shown while the session holds nothing.
    pub none: Shown<'s>,
    /// The search stands in the top bar: the card starts where a page's content does, with no
    /// gap over it.
    pub in_bar: bool,
    press: PhantomData<A>,
}

impl<'s, A> SessionPaneModel<'s, A> {
    /// The pane with nothing held.
    pub fn empty(heading: Shown<'s>, none: Shown<'s>, in_bar: bool) -> Self {
        Self {
            heading,
            none,
            in_bar,
            press: PhantomData,
        }
    }

    fn gap(&self) -> f32 {
        if self.in_bar { 0.0 } else { HOME_CARD_GAP }
    }
}

fn head_h<A: Pressed>(ui: &Ui<'_, A>) -> f32 {
    PAD + ui.line_as(HOME_CARD_HEAD) + GRID
}

fn card_h<A: Pressed>(ui: &Ui<'_, A>) -> f32 {
    head_h(ui) + ui.line_as(BODY_L) + PAD + GRID
}

/// The height the pane takes with the gap over it, in a window `w` wide: what a page stacks
/// the next thing under.
pub fn height<A: Pressed>(m: &SessionPaneModel<'_, A>, ui: &Ui<'_, A>) -> f32 {
    m.gap() + card_h(ui)
}

/// Draws the pane from (x, y) in `w`: the card at most `MENU_PAGE_W` wide and centred, a
/// surface-container-low card at level 1 (outlined at High contrast), its heading, the line
/// under it. Returns its height with the gap.
pub fn draw<A: Pressed>(
    m: &SessionPaneModel<'_, A>,
    ui: &mut Ui<'_, A>,
    (x, y, w): (f32, f32, f32),
) -> f32 {
    let cw = w.min(MENU_PAGE_W);
    let (cx, top, h) = (x + (w - cw) / 2.0, y + m.gap(), card_h(ui));
    ui.shadow(cx, top, cw, h, SHAPE_M, ELEVATION_SHEET);
    ui.fill(cx, top, cw, h, SHAPE_M, Role::SurfaceContainerLow);
    if ui.theme.is_high_contrast() {
        ui.stroke(cx, top, cw, h, SHAPE_M, Role::Outline);
    }
    ui.showing(m.heading, |ui| {
        ui.text_as(
            cx + PAD + LIST_PAD,
            top + PAD,
            HOME_CARD_HEAD,
            Role::OnSurface,
            m.heading.as_str(),
        );
    });
    ui.text_as(
        cx + PAD + LIST_PAD,
        top + head_h(ui),
        BODY_L,
        Role::OnSurfaceVariant,
        m.none.as_str(),
    );
    m.gap() + h
}

#[cfg(test)]
mod tests {
    use super::*;
    use faraday_atoms::ui::testing::{P, at_rest, canvas, frame};

    /// Invariant: With nothing held the card is centred in the room, at most the menu page's width, and the height it reports is the height it drew (a page stacks under it by it).
    /// Regression: The next section drawn over the card, or the card off the page's column.
    /// Owner: kit
    #[test]
    fn the_empty_card_reports_the_height_it_takes() {
        let mut c = canvas(160);
        let mut drawn = 0.0;
        let mut said = 0.0;
        frame(&mut c, &at_rest(None), |ui| {
            let m = SessionPaneModel::<P>::empty(
                Shown::plain("This session"),
                Shown::plain("Nothing loaded"),
                false,
            );
            said = height(&m, ui);
            drawn = draw(&m, ui, (0.0, 0.0, 400.0));
        });
        assert_eq!(drawn, said);
        assert!(drawn > 0.0);
    }
}
