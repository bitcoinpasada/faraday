//! The Lightning page's keys as a pane at the left of the card on the wide
//! layout (`docs/FLOW-CONTAINER.md` §4): where the pair stands and the
//! rows. Labels only: nothing here holds or reads a secret. The pair's
//! lines are Explore's (`explore::pane::lines`).

use crate::Action;
use crate::drawing::View;
use crate::lightning::NodeSource;
use crate::pick;
use crate::screens::hub_layout::{Left, Narrow, Panes, Tile};
use crate::strings::EN;
use faraday_text::Text;

/// The Lightning page's panes: the keys and the card.
const PANES: Panes = Panes {
    left: Left::Own,
    tile: Tile::Card,
    stacks: false,
    narrow: Narrow::OneColumn,
};

/// Where the pair stands, when the keys are in a pane at the card's left:
/// the wide layout, a key loaded, not opened from Convert (the entry
/// stands alone there) and the room holds the pair (`screens::hub_layout`). A
/// lookup.
pub(crate) fn place(
    app: &crate::Faraday,
    compact: bool,
    room: f32,
) -> Option<crate::screens::hub_layout::Place> {
    if compact || app.lightning.state().is_none_or(|l| l.for_convert) {
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

/// The loaded keys as the pane's rows, two for each (its ldk-node and its
/// Core Lightning node), the one the source names ticked. Each row presses
/// what the button of the same name presses.
pub(crate) fn rows(app: &View, source: Option<NodeSource>) -> Vec<pick::PickRow> {
    let items = app.session.keys.all().flat_map(|k| {
        let fp = k.fingerprint;
        let name = crate::wallet::key_line(fp, &k.label);
        [
            (
                Text::fill(EN.lightning_ldk_node, &[&name]),
                source == Some(NodeSource::Key(fp.0)),
                Action::LKey(fp.0),
            ),
            (
                Text::fill(EN.lightning_cln_node, &[&name]),
                source == Some(NodeSource::KeyCln(fp.0)),
                Action::LCln(Some(fp.0)),
            ),
        ]
    });
    crate::explore::pane::key_choices(items)
}
