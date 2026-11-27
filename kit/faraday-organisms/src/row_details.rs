//! A row's details in the page's second pane (09 §3.3): "What you can do" first, one list row
//! for each thing the open item offers, then its facts, in a card that scrolls when the pane is
//! shorter than it. What the item offers and says comes in; this draws it.

use faraday_atoms::hits::Shape;
use faraday_atoms::region::Region;
use faraday_atoms::{Anchor, Icon, Pressed, Shown, Ui};
use faraday_tokens::dims::{
    ACTIONS_UNDER, BODY_L, BODY_M, GRID, ICON_SMALL, LIST_1, LIST_2, LIST_PAD, LIST_PAD_V, SHAPE_L,
    SHAPE_M, TITLE_S,
};
use faraday_tokens::elevation::ELEVATION_SHEET;
use faraday_tokens::palette::Role;

/// One thing the open item offers: its words, its icon where it has one and what a press does.
pub struct OfferModel<'s, A> {
    pub label: Shown<'s>,
    pub icon: Option<Icon>,
    pub press: A,
    /// The press opens a page or a flow: the row ends in a chevron.
    pub opens: bool,
    /// The press begins a dangerous act: the row is in the error tone.
    pub danger: bool,
}

/// What is known of the open item: its name, then a line under it.
pub struct FactsModel<'s> {
    pub head: Shown<'s>,
    pub line: Shown<'s>,
    /// The name is set in mono (a key's fingerprint).
    pub mono: bool,
}

/// The details of the open row.
pub struct RowDetailsModel<'s, A> {
    pub offers_heading: Shown<'s>,
    pub offers: Vec<OfferModel<'s, A>>,
    pub facts_heading: Shown<'s>,
    pub facts: Option<FactsModel<'s>>,
    /// Keys the scroll of the card's content.
    pub scroll: Anchor,
}

fn section_label<A: Pressed>(ui: &mut Ui<'_, A>, x: f32, y: f32, label: &Shown<'_>) {
    ui.showing(label, |ui| {
        ui.text_as(x, y, TITLE_S, Role::OnSurfaceVariant, label.as_str());
    });
}

/// The "What you can do" group at (x, y), `w` wide: the heading, then a row a touch target tall
/// for each offer, the whole row its press. The first row's icon is in the primary accent.
/// Returns its height.
fn offers<A: Pressed>(
    m: &RowDetailsModel<'_, A>,
    ui: &mut Ui<'_, A>,
    (x, y, w): (f32, f32, f32),
) -> f32 {
    section_label(ui, x, y, &m.offers_heading);
    let mut cy = y + ui.line_as(TITLE_S) + GRID;
    // A group none of whose offers has an icon leaves no gap for one.
    let lead = if m.offers.iter().any(|o| o.icon.is_some()) {
        2.0 * LIST_PAD + ICON_SMALL
    } else {
        LIST_PAD
    };
    let room = w - lead - LIST_PAD - ICON_SMALL - GRID;
    for (i, o) in m.offers.iter().enumerate() {
        let ink = if o.danger {
            Role::Error
        } else {
            Role::OnSurface
        };
        let label = o.label.as_str();
        let line = ui.line_as(BODY_L);
        // A label that does not fit takes a second line, as a list item's headline does.
        let two = ui.measure_as(BODY_L, label) > room;
        let h = if two {
            LIST_2.max(2.0 * line + 2.0 * LIST_PAD_V)
        } else {
            LIST_1
        };
        if let Some(icon) = o.icon {
            let tone = if o.danger {
                Role::Error
            } else if i == 0 {
                Role::Primary
            } else {
                Role::OnSurfaceVariant
            };
            ui.icon(
                x + LIST_PAD,
                cy + (h - ICON_SMALL) / 2.0,
                ICON_SMALL,
                icon,
                ICON_SMALL,
                tone,
            );
        }
        ui.showing(&o.label, |ui| {
            if two {
                let at = ui.first_line_end(BODY_L, label, room);
                let (one, rest) = label.split_at(at);
                let top = cy + (h - 2.0 * line) / 2.0;
                ui.text_mid_as((x + lead, top, line), BODY_L, ink, one.trim_end());
                let rest = ui.fit_as(BODY_L, rest.trim_start(), room);
                ui.text_mid_as((x + lead, top + line, line), BODY_L, ink, &rest);
            } else {
                ui.text_mid_as((x + lead, cy + (h - line) / 2.0, line), BODY_L, ink, label);
            }
        });
        if o.opens {
            ui.row_chevron(x + w - LIST_PAD, cy, h, Role::OnSurfaceVariant);
        }
        ui.press(
            Shape::new(x, cy, w, h, SHAPE_L),
            ink.paint(),
            o.press.clone(),
        );
        cy += h;
    }
    cy - y
}

/// The group, then the facts, from (x, y), `w` wide. Returns the height.
fn body<A: Pressed>(
    m: &RowDetailsModel<'_, A>,
    ui: &mut Ui<'_, A>,
    (x, y): (f32, f32),
    w: f32,
) -> f32 {
    let mut cy = y;
    if !m.offers.is_empty() {
        cy += offers(m, ui, (x, cy, w)) + ACTIONS_UNDER;
    }
    if let Some(f) = &m.facts {
        section_label(ui, x, cy, &m.facts_heading);
        cy += ui.line_as(TITLE_S) + GRID;
        let style = if f.mono { BODY_L.mono() } else { BODY_L };
        ui.showing(&f.head, |ui| {
            cy += ui.wrap_as((x, cy, w), style, Role::OnSurface, f.head.as_str()) + GRID;
        });
        if !f.line.as_str().is_empty() {
            ui.showing(&f.line, |ui| {
                cy += ui.wrap_as((x, cy, w), BODY_M, Role::OnSurfaceVariant, f.line.as_str());
            });
        }
    }
    cy - y
}

/// What the details take inside their padding, `w` wide: the room a stack asks for.
pub fn content_h<A: Pressed>(m: &RowDetailsModel<'_, A>, ui: &mut Ui<'_, A>, w: f32) -> f32 {
    ui.measured(|ui| body(m, ui, (0.0, 0.0), w))
}

/// Draws the details in the tile `(x, y, w, h)`: the raised card, its content scrolled where it
/// is taller. Returns how tall the content is with its padding.
pub fn draw<A: Pressed>(
    m: &RowDetailsModel<'_, A>,
    ui: &mut Ui<'_, A>,
    (x, y, w, h): (f32, f32, f32, f32),
) -> f32 {
    use faraday_tokens::dims::PAD;
    ui.shadow(x, y, w, h, SHAPE_M, ELEVATION_SHEET);
    ui.fill(x, y, w, h, SHAPE_M, Role::SurfaceContainerLow);
    if ui.theme.is_high_contrast() {
        ui.stroke(x, y, w, h, SHAPE_M, Role::Outline);
    }
    let inner = w - 2.0 * PAD;
    let need = content_h(m, ui, inner) + 2.0 * PAD;
    let clip = ui.rect(x, y, w, h);
    ui.region(Region::new(m.scroll, clip), |ui| {
        body(m, ui, (x + PAD, y + PAD), inner);
        (need - h).max(0.0)
    });
    need
}

#[cfg(test)]
mod tests {
    use super::*;
    use faraday_atoms::ui::testing::{P, at_rest, canvas, frame};

    /// Invariant: Each thing the open item offers is a row that takes a press, and a long label takes a second line, so the group is as tall as its rows.
    /// Regression: An offer that cannot be pressed, or a label cut where it wraps.
    /// Owner: kit
    #[test]
    fn each_offer_is_a_row_that_takes_a_press() {
        let mut c = canvas(160);
        let mut one = 0.0;
        let mut two = 0.0;
        let mut presses = 0;
        let offer = |label| OfferModel {
            label: Shown::plain(label),
            icon: None,
            press: P(1),
            opens: true,
            danger: false,
        };
        let model = |label| RowDetailsModel::<P> {
            offers_heading: Shown::plain("What you can do"),
            offers: vec![offer(label)],
            facts_heading: Shown::plain("Facts"),
            facts: None,
            scroll: Anchor {
                scope: "row-details",
                n: 0,
            },
        };
        let out = frame(&mut c, &at_rest(None), |ui| {
            one = content_h(&model("Explore"), ui, 300.0);
            two = content_h(
                &model("Make a wallet from this key with a name long enough to wrap"),
                ui,
                300.0,
            );
            draw(&model("Explore"), ui, (0.0, 0.0, 316.0, 400.0));
        });
        presses += out.hits.len();
        assert!(two > one);
        assert!(presses >= 1);
    }
}
