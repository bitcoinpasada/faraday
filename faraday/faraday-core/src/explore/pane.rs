//! Explore's keys as a pane at the left of the card on the wide layout
//! (`docs/FLOW-CONTAINER.md` §4): the rows, where the pair stands and its
//! lines. Labels only: nothing here holds or reads a secret.

use crate::Action;
use crate::drawing::View;
use crate::explore::Page;
use crate::pick;
use crate::ui::Ui;
use faraday_draw::widgets::Icon;

/// Where the pair stands, when the keys are in a pane at the card's left:
/// the wide layout, the Path page, a key loaded and the room holds the pair
/// (`flow::pair`). A lookup.
pub(crate) fn place(
    app: &crate::Faraday,
    compact: bool,
    room: f32,
) -> Option<crate::flow::unit::Place> {
    if compact || app.explore.state().is_none_or(|e| e.page != Page::Path) {
        return None;
    }
    app.session.keys.all().next()?;
    crate::flow::pair::pair(room, false)
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
pub(crate) fn lines(ui: &mut Ui, x0: f32, p: crate::flow::unit::Place, top: f32, end: f32) {
    let at = x0 + p.pane_x + p.pane_w + p.gap / 2.0;
    let edges = (x0 + p.pane_x, x0 + p.card_x + p.card_w);
    crate::flow::pair_lines(ui, (at, top, end), Some((edges, f32::MAX)));
}
