//! Explore's keys as a pane at the left of the card on the wide layout
//! (`docs/FLOW-CONTAINER.md` §4): the rows, where the pair stands, its
//! card and divider. Labels only: nothing here holds or reads a secret.

use crate::Action;
use crate::drawing::View;
use crate::explore::Page;
use crate::pick;
use crate::screens::hub_layout::{self, Left, Narrow, Panes, Tile};
use crate::ui::Ui;

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
    if !stands(app, compact) {
        return None;
    }
    PANES.place(room, app.guide_beside_pair())
}

/// Whether the page is one that stands as the pair, given the room. A
/// lookup.
pub(crate) fn stands(app: &crate::Faraday, compact: bool) -> bool {
    !compact
        && app.explore.state().is_some_and(|e| e.page == Page::Path)
        && app.session.keys.all().next().is_some()
}

/// Whether the pair stands in a `room`: the page's area is then the work
/// width. A lookup.
pub(crate) fn place_in(app: &crate::Faraday, room: f32) -> bool {
    place(app, app.is_compact(), room).is_some()
}

/// The loaded keys as the pane's group, `chosen` ticked.
pub(crate) fn key_groups(app: &View, chosen: Option<[u8; 4]>) -> Vec<crate::flow::PickGroup> {
    pick::Decl::new("")
        .keys(app, |_, k| {
            Some(pick::Take::of(
                chosen == Some(k.fingerprint.0),
                Action::XKey(k.fingerprint.0),
            ))
        })
        .groups()
}

/// The pair's card, under content that starts at `top`.
pub(crate) fn card(app: &View, ui: &mut Ui, x0: f32, p: hub_layout::Place, top: f32) {
    let edges = (x0 + p.pane_x, x0 + p.card_x + p.card_w);
    hub_layout::pair_card(ui, app.scroll.card_foot(), edges, top);
}

/// The end of a page where the pair stands, whatever its card shows: the
/// divider from `y_pane` to the lower of the column (`y`) and the pane
/// (`pane_end`), and the scroll height to there, with the card's foot
/// below it.
pub(crate) fn foot(
    app: &View,
    ui: &mut Ui,
    (x0, h): (f32, f32),
    (p, y_pane): (hub_layout::Place, f32),
    (y, pane_end): (f32, f32),
) {
    let end = hub_layout::level(pane_end, y, f32::MAX);
    let at = x0 + p.pane_x + p.pane_w + p.gap / 2.0;
    hub_layout::divider(ui, (at, y_pane, end));
    hub_layout::pair_card_ends(ui, app.scroll.card_foot(), end);
    // The card ends a padding below `end`: the page scrolls to a padding
    // below that.
    let pad = faraday_draw::tokens::PAD;
    crate::compact::screen::finish(app, ui, h, end + 2.0 * pad);
}
