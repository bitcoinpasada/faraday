//! The ways-in organism (09 §3.3): a key form's ways in (`docs/KEY-FORMS.md` §4.1). The forms
//! shown first as buttons in rows, and Other forms: one text button with a chevron that opens
//! its rows in place, or as a sheet on a small panel (the sheet is the page's).

use faraday_atoms::button::Look;
use faraday_atoms::{Icon, Pressed, Shown, Ui};
use faraday_tokens::dims::{
    BUTTON_H, BUTTON_PAD, DISCLOSE_GLYPH, DISCLOSE_ICON_Y, GAP, GAP_SMALL, ICON_GLYPH_SMALL,
    LABEL_L, TOUCH,
};
use faraday_tokens::palette::Role;

/// One way in: its words, how its button looks (the form shown chosen is selected) and what a
/// press does.
pub struct WayModel<'s, A> {
    pub label: Shown<'s>,
    pub look: Look,
    pub press: A,
}

/// The ways in of one opener.
pub struct WaysInModel<'s, A> {
    /// The forms shown first.
    pub first: Vec<WayModel<'s, A>>,
    /// The forms under Other forms: the ones the opener takes and does not show first.
    pub others: Vec<WayModel<'s, A>>,
    /// Other forms' own words.
    pub other_label: Shown<'s>,
    /// Other forms is open in place: its rows stand where its button was.
    pub other_open: bool,
    /// Other forms is a sheet on every panel: it never opens in place.
    pub sheet_only: bool,
    /// The press that opens Other forms.
    pub open_others: A,
}

/// Buttons [`BUTTON_H`] tall laid left to right, wrapping to a new row at the width. Returns the
/// height they took, a row being a button and the gap under it.
fn rows<A: Pressed>(
    ui: &mut Ui<'_, A>,
    (x, y, w): (f32, f32, f32),
    items: &[WayModel<'_, A>],
) -> f32 {
    // As tall as larger text needs (`docs/TEXT-SIZE.md` §5).
    let bh = ui.button_height(BUTTON_H);
    let pitch = (bh + GAP).max(ui.touch);
    let (mut bx, mut by) = (x, y);
    for it in items {
        ui.showing(&it.label, |ui| {
            let look = it.look;
            // No wider than the row; a long label is cut.
            let bw = ui.button_width(it.label.as_str(), look).min(w);
            let label = ui.fit_as(
                LABEL_L,
                it.label.as_str(),
                bw - ui.button_width("", look) + BUTTON_PAD,
            );
            if bx > x && bx + bw > x + w {
                bx = x;
                by += pitch;
            }
            bx += if look == Look::Ghost && bx == x {
                // A text button on its own at the row's start: its label on the edge.
                ui.text_button(bx, by, &label, look, it.press.clone()) + GAP
            } else {
                ui.button(bx, by, Some(bw), bh, &label, look, it.press.clone()) + GAP
            };
        });
    }
    by - y + pitch
}

/// Other forms of a screen that shows its ways as rows of its own: the text button with its
/// chevron, or, once opened, its rows in place. Returns the height.
fn other_control<A: Pressed>(
    m: &WaysInModel<'_, A>,
    ui: &mut Ui<'_, A>,
    at: (f32, f32, f32),
) -> f32 {
    let (x, y, _) = at;
    if m.other_open && !m.sheet_only {
        return rows(ui, at, &m.others);
    }
    let lw = ui.showing(&m.other_label, |ui| {
        ui.button(
            x,
            y,
            None,
            TOUCH,
            m.other_label.as_str(),
            Look::Ghost,
            m.open_others.clone(),
        )
    });
    ui.icon(
        x + lw + GAP_SMALL,
        y + DISCLOSE_ICON_Y,
        ICON_GLYPH_SMALL,
        Icon::ChevronRight,
        DISCLOSE_GLYPH,
        Role::OnSurfaceVariant,
    );
    TOUCH + GAP_SMALL
}

/// Draws the forms shown first, then Other forms: its text button with its chevron, or, once
/// opened, its rows. Returns the height.
pub fn draw<A: Pressed>(
    m: &WaysInModel<'_, A>,
    ui: &mut Ui<'_, A>,
    (x, y, w): (f32, f32, f32),
) -> f32 {
    let cy = y + rows(ui, (x, y, w), &m.first);
    cy + other_control(m, ui, (x, cy, w)) + if m.other_open { GAP_SMALL } else { 0.0 } - y
}

#[cfg(test)]
mod tests {
    use super::*;
    use faraday_atoms::ui::testing::{P, at_rest, canvas, frame};

    fn way(label: &'static str, n: u8) -> WayModel<'static, P> {
        WayModel {
            label: Shown::plain(label),
            look: Look::Secondary,
            press: P(n),
        }
    }

    /// Invariant: Other forms is one button until it is opened and then its rows stand in its place, each a press; opened on a sheet-only opener it stays one button.
    /// Regression: Other forms opening in a dialog that has no page to hold its rows, or its rows with no press.
    /// Owner: kit
    #[test]
    fn other_forms_opens_in_place_unless_it_is_a_sheet() {
        let model = |open, sheet_only| WaysInModel::<P> {
            first: vec![way("Words", 1)],
            others: vec![way("SeedQR", 2), way("Krux code", 3)],
            other_label: Shown::plain("Other forms"),
            other_open: open,
            sheet_only,
            open_others: P(9),
        };
        let mut hits = Vec::new();
        for (open, sheet_only) in [(false, false), (true, false), (true, true)] {
            let mut c = canvas(160);
            let out = frame(&mut c, &at_rest(None), |ui| {
                draw(&model(open, sheet_only), ui, (0.0, 0.0, 320.0));
            });
            hits.push(out.hits.len());
        }
        // Closed: Words and Other forms. Open in place: Words and the two rows.
        assert_eq!(hits, [2, 3, 2]);
    }
}
