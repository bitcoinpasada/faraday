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

/// The pair's divider and outline, from `top` to `end`.
pub(crate) fn lines(ui: &mut Ui, x0: f32, p: hub_layout::Place, top: f32, end: f32) {
    let at = x0 + p.pane_x + p.pane_w + p.gap / 2.0;
    let edges = (x0 + p.pane_x, x0 + p.card_x + p.card_w);
    hub_layout::lines(ui, (at, top, end), Some((edges, f32::MAX)));
}
