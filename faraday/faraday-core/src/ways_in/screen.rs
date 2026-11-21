//! Drawing the ways in: the forms shown first as chips, and Other forms,
//! one text button with a chevron that opens its rows in place, or as a
//! sheet on a small panel (`docs/KEY-FORMS.md` §4.1).

use super::Opening;
use crate::Action;
use crate::drawing::View;
use crate::forms::Form;
use crate::strings::EN;
use crate::ui::pal::*;
use crate::ui::{Style, Ui};
use faraday_draw::tokens::{DISCLOSE_GLYPH, DISCLOSE_ICON_Y, GAP_SMALL, ICON_GLYPH_SMALL, TOUCH};
use faraday_draw::widgets::Icon;
use faraday_ui::parts::button_rows;

/// One chip row of `forms`, the form shown chosen. Returns the height.
fn rows(
    app: &View,
    ui: &mut Ui,
    (x, y, w): (f32, f32, f32),
    opening: Opening,
    forms: &[Form],
) -> f32 {
    let shown: Vec<(&str, Style, Action)> = forms
        .iter()
        .map(|f| {
            (
                f.name(),
                Style::chosen(*f == app.entry.state().form),
                opening.form_press(*f),
            )
        })
        .collect();
    button_rows(ui, x, y, w, &shown)
}

/// The rows under Other forms: the forms the opener takes and does not
/// show first. Returns the height.
pub(crate) fn other_rows(app: &View, ui: &mut Ui, at: (f32, f32, f32), opening: Opening) -> f32 {
    rows(app, ui, at, opening, &opening.opener().other_forms())
}

/// The forms shown first, then Other forms: its text button with its
/// chevron, or, once opened, its rows. Returns the height.
pub(crate) fn forms(app: &View, ui: &mut Ui, at: (f32, f32, f32), opening: Opening) -> f32 {
    let (x, y, w) = at;
    let first: Vec<Form> = opening.opener().up_front_forms().collect();
    let mut cy = y + rows(app, ui, at, opening, &first);
    if app.entry.state().other_forms {
        return cy + other_rows(app, ui, (x, cy, w), opening) + GAP_SMALL - y;
    }
    let lw = ui.button(
        x,
        cy,
        None,
        TOUCH,
        EN.entry_other_forms,
        Style::Ghost,
        Action::EntryOtherForms(opening),
    );
    ui.icon(
        x + lw + GAP_SMALL,
        cy + DISCLOSE_ICON_Y,
        ICON_GLYPH_SMALL,
        Icon::ChevronRight,
        DISCLOSE_GLYPH,
        Role::OnSurfaceVariant,
    );
    cy += TOUCH + GAP_SMALL;
    cy - y
}
