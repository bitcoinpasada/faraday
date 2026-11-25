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

/// The rows under Other forms: the forms the opener takes and does not
/// show first, and its loaded keys. Returns the height.
pub(crate) fn other_rows(app: &View, ui: &mut Ui, at: (f32, f32, f32), o: Opening) -> f32 {
    other_rows_with(app, ui, at, o, o.holders())
}

/// [`other_rows`] with the loaded keys under it or not: where a pane at the
/// left holds them as the pick's rows, they are not here too.
fn other_rows_with(
    app: &View,
    ui: &mut Ui,
    (x, y, w): (f32, f32, f32),
    o: Opening,
    holders: bool,
) -> f32 {
    button_rows(ui, x, y, w, &o.others_with(app, holders))
}

/// Other forms of a screen that shows its ways as rows of its own: the
/// text button with its chevron, or, once opened, its rows in place.
/// Returns the height.
pub(crate) fn other_control(app: &View, ui: &mut Ui, at: (f32, f32, f32), o: Opening) -> f32 {
    other_control_with(app, ui, at, o, o.holders())
}

/// [`other_control`] with the loaded keys among its rows or not.
pub(crate) fn other_control_with(
    app: &View,
    ui: &mut Ui,
    at: (f32, f32, f32),
    o: Opening,
    holders: bool,
) -> f32 {
    let (x, y, _) = at;
    if app.entry.state().other_forms && !o.sheet_only() {
        return other_rows_with(app, ui, at, o, holders);
    }
    let lw = ui.button(
        x,
        y,
        None,
        TOUCH,
        EN.entry_other_forms,
        Style::Ghost,
        Action::EntryOtherForms(o),
    );
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

/// One chip row of the forms in `forms`, the form shown chosen. Returns
/// the height.
fn chips(app: &View, ui: &mut Ui, (x, y, w): (f32, f32, f32), o: Opening, forms: &[Form]) -> f32 {
    let shown: Vec<_> = forms
        .iter()
        .map(|f| o.item(app, super::Way::Form(*f)))
        .collect();
    button_rows(ui, x, y, w, &shown)
}

/// The forms shown first, then Other forms: its text button with its
/// chevron, or, once opened, its rows. Returns the height.
pub(crate) fn forms(app: &View, ui: &mut Ui, at: (f32, f32, f32), opening: Opening) -> f32 {
    let (x, y, w) = at;
    let first: Vec<Form> = opening.opener().up_front_forms().collect();
    let cy = y + chips(app, ui, at, opening, &first);
    cy + other_control(app, ui, (x, cy, w), opening)
        + if app.entry.state().other_forms {
            GAP_SMALL
        } else {
            0.0
        }
        - y
}

/// The ways in as the pick's rows, where a pane at the left holds them:
/// the forms shown first, then Other forms, which opens the rest in
/// place. A press is the card's own press.
pub(crate) fn way_rows(app: &crate::Faraday, o: Opening) -> Vec<crate::pick::PickRow> {
    let row = |(t, _, a): super::Item| {
        let icon = match a {
            Action::ScanSeed => Icon::Qr,
            Action::SeedCheck(crate::seed_check::SeedAction::Type) => Icon::Keyboard,
            _ => Icon::Keys,
        };
        crate::pick::PickRow::way_in(icon, t.as_str(), a)
    };
    let mut rows: Vec<_> = o.up_front(app).into_iter().map(row).collect();
    rows.push(crate::pick::PickRow::way_in(
        Icon::List,
        EN.entry_other_forms,
        Action::EntryOtherForms(o),
    ));
    if app.entry.state().other_forms {
        rows.extend(o.others_with(app, false).into_iter().map(row));
    }
    rows
}
