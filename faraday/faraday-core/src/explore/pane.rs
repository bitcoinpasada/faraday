//! Explore's keys as a pane at the left of the card on the wide layout
//! (`docs/FLOW-CONTAINER.md` §4): the rows, where the pair stands and its
//! lines. Labels only: nothing here holds or reads a secret.

use crate::Action;
use crate::drawing::View;
use crate::explore::Page;
use crate::pick;
use crate::screens::hub_layout::{self, Left, Narrow, Panes, Tile};
use crate::ui::Ui;
use faraday_draw::widgets::Icon;

/// Explore's panes: the keys and the card.
const PANES: Panes = Panes {
    left: Left::Own,
    tile: Tile::Card,
    stacks: false,
    narrow: Narrow::OneColumn,
};

/// Where the pair stands, when the keys are in a pane at the card's left:
/// the wide layout, the Path page, a key loaded and the room holds the pair
/// (`screens::hub_layout`). A lookup.
pub(crate) fn place(app: &crate::Faraday, compact: bool, room: f32) -> Option<hub_layout::Place> {
    if compact || app.explore.state().is_none_or(|e| e.page != Page::Path) {
        return None;
    }
    app.session.keys.all().next()?;
    PANES.place(room, false)
}

/// Whether the pair stands in a `room`: the page's area is then the work
/// width. A lookup.
pub(crate) fn place_in(app: &crate::Faraday, room: f32) -> bool {
    place(app, app.is_compact(), room).is_some()
}

/// The loaded keys as the pane's rows, `chosen` ticked.
pub(crate) fn key_rows(app: &View, chosen: Option<[u8; 4]>) -> Vec<pick::PickRow> {
    app.session
        .keys
        .all()
        .map(|k| {
            let fp = k.fingerprint;
            pick::PickRow::pick(
                Icon::Keys,
                crate::wallet::key_line(fp, &crate::bip85::key_name(k)).to_string(),
                String::new(),
                chosen == Some(fp.0),
                Action::XKey(fp.0),
            )
        })
        .collect()
}

/// A pane's rows from labels, each with whether it is ticked and what its
/// press does. Labels only.
pub(crate) fn key_choices(
    items: impl IntoIterator<Item = (impl AsRef<str>, bool, Action)>,
) -> Vec<pick::PickRow> {
    items
        .into_iter()
        .map(|(label, chosen, action)| {
            pick::PickRow::pick(
                Icon::Keys,
                label.as_ref().to_owned(),
                String::new(),
                chosen,
                action,
            )
        })
        .collect()
}

/// The pair's divider and outline, from `top` to `end`.
pub(crate) fn lines(ui: &mut Ui, x0: f32, p: hub_layout::Place, top: f32, end: f32) {
    let at = x0 + p.pane_x + p.pane_w + p.gap / 2.0;
    let edges = (x0 + p.pane_x, x0 + p.card_x + p.card_w);
    hub_layout::lines(ui, (at, top, end), Some((edges, f32::MAX)));
}

/// The end of a page where the pair stands, whatever its card shows: the
/// pair's lines from `y_pane` to the lower of the card (`y`) and the pane
/// (`pane_end`), and the scroll height to there, with the outline's foot
/// below it.
pub(crate) fn foot(
    app: &View,
    ui: &mut Ui,
    (x0, h): (f32, f32),
    (p, y_pane): (hub_layout::Place, f32),
    (y, pane_end): (f32, f32),
) {
    let end = hub_layout::level(pane_end, y, f32::MAX);
    lines(ui, x0, p, y_pane, end);
    // The outline ends a padding below `end`: the page scrolls to a padding
    // below that.
    let pad = faraday_draw::tokens::PAD;
    crate::compact::screen::finish(app, ui, h, end + 2.0 * pad);
}
