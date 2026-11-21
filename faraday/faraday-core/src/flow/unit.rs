//! Where a flow's card and Your choices stand on the wide layout: one unit,
//! the card's column, a spacer and the pane, centred in the room beside the
//! navigation, and moved left only as far as Guide's pane needs when it
//! stands beside the flow (`docs/FLOW-CONTAINER.md` §4). Arithmetic.

use faraday_draw::tokens::{CHOICES_PANE_MIN, COLUMN_MAX, FLOW_CARD_MIN, MARGIN_WIDE, SPACER};

/// Where the unit's parts stand, from the room's left edge.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Place {
    /// The card column's left edge.
    pub(crate) card_x: f32,
    /// The card column's width.
    pub(crate) card_w: f32,
    /// Your choices' left edge.
    pub(crate) pane_x: f32,
    /// Your choices' width.
    pub(crate) pane_w: f32,
}

/// The room Guide's pane takes at the right edge: the pane, the spacer
/// before it and the margin after it.
fn guide_room() -> f32 {
    crate::screens::support_pane_room()
}

/// The narrowest room beside the navigation that holds the unit at its
/// narrowest and Guide's pane beside it.
pub(crate) fn guide_min_room() -> f32 {
    MARGIN_WIDE + FLOW_CARD_MIN + SPACER + CHOICES_PANE_MIN + guide_room()
}

/// The unit in a `room` wide: with `guide`, the card narrows from its
/// widest first, then Your choices, as far as Guide's pane needs, and the
/// unit stands left of the pane's room; with none it is centred, and
/// narrows only to fit the room.
pub(crate) fn place(room: f32, guide: bool) -> Place {
    let guide = guide && room >= guide_min_room();
    let limit = if guide { room - guide_room() } else { room };
    let avail = limit - MARGIN_WIDE;
    let pane0 = crate::screens::side_panel_w(room);
    let floor = if guide { FLOW_CARD_MIN } else { 0.0 };
    let card_w = (avail - SPACER - pane0).clamp(floor, COLUMN_MAX.max(floor));
    let pane_w = if guide {
        (avail - SPACER - card_w).clamp(CHOICES_PANE_MIN, pane0)
    } else {
        pane0
    };
    let unit = card_w + SPACER + pane_w;
    let centred = ((room - unit) / 2.0).max(MARGIN_WIDE);
    let card_x = if guide {
        centred.min(limit - unit)
    } else {
        centred
    };
    Place {
        card_x,
        card_w,
        pane_x: card_x + card_w + SPACER,
        pane_w,
    }
}
