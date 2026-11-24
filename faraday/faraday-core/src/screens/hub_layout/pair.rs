//! The pair's arithmetic (`docs/WORKSPACE.md` §4): the pane at the left and
//! the card or tile at its right, one gap between, centred in the room
//! beside the navigation; on the room alone, so a menu page's pair and a
//! flow's pair stand at the same place.

use super::{Place, guide_room, side_panel_w};
use crate::screens::menu_page::grid;
use faraday_draw::tokens::{
    CHOICES_PANE_MIN, COLUMN_MAX, FLOW_CARD_MIN, LIST_PANE, MARGIN_WIDE, MENU_TILE_WIDE_MAX, PAD,
    PAGE_INSET, READING_VISIT_W, SPACER, VISIT_GAP,
};

/// The pair in a `room` wide, with `guide` when Guide's pane stands at the
/// room's right edge. The pane is `LIST_PANE` wide and the card takes the
/// rest up to `COLUMN_MAX`; in a narrower room the card narrows to
/// `FLOW_CARD_MIN`, then the pane down to `CHOICES_PANE_MIN`. With the
/// guide the pair stands left of its room, as far left as the room
/// needs. `None` where the pair at its narrowest does not fit.
pub(super) fn pair(room: f32, guide: bool) -> Option<Place> {
    let limit = if guide { room - guide_room() } else { room };
    // A margin at the left, and at the right unless Guide's room has it.
    let avail = limit
        - if guide {
            MARGIN_WIDE
        } else {
            2.0 * MARGIN_WIDE
        };
    if avail < FLOW_CARD_MIN + SPACER + CHOICES_PANE_MIN {
        return None;
    }
    let card_w = (avail - SPACER - LIST_PANE).clamp(FLOW_CARD_MIN, COLUMN_MAX);
    let pane_w = (avail - SPACER - card_w).clamp(CHOICES_PANE_MIN, LIST_PANE);
    let unit = pane_w + SPACER + card_w;
    let centred = ((room - unit) / 2.0).max(MARGIN_WIDE);
    let pane_x = if guide {
        centred.min(limit - unit)
    } else {
        centred
    };
    Some(Place {
        card_x: pane_x + pane_w + SPACER,
        card_w,
        pane_x,
        pane_w,
        gap: SPACER,
    })
}

/// The pair whose tile fills the room in two columns or more: the pane
/// at the room's left page margin, one gap, the tile to the right page
/// margin and no wider than `MENU_TILE_WIDE_MAX`, the pair centred where
/// the tile stops. `None` where the room holds one column (the pair is
/// then a flow's). Arithmetic on the room alone.
pub(super) fn fill(room: f32) -> Option<Place> {
    let tile = (room - 2.0 * MARGIN_WIDE - LIST_PANE - SPACER).min(MENU_TILE_WIDE_MAX);
    if grid(tile - 2.0 * PAD, usize::MAX).columns() < 2 {
        return None;
    }
    let unit = LIST_PANE + SPACER + tile;
    let pane_x = ((room - unit) / 2.0).max(MARGIN_WIDE);
    Some(Place {
        card_x: pane_x + LIST_PANE + SPACER,
        card_w: tile,
        pane_x,
        pane_w: LIST_PANE,
        gap: SPACER,
    })
}

/// The pair of two equal columns (Files): the room up to `READING_VISIT_W`,
/// centred, a `PAGE_INSET` inside it at each side, the two columns a
/// `VISIT_GAP` apart. `None` where the room is under the pair's least.
pub(super) fn equal(room: f32) -> Option<Place> {
    if room < 2.0 * MARGIN_WIDE + FLOW_CARD_MIN + SPACER + CHOICES_PANE_MIN {
        return None;
    }
    let reading = room.min(READING_VISIT_W);
    let col = (reading - 2.0 * PAGE_INSET - VISIT_GAP) / 2.0;
    let pane_x = (room - reading) / 2.0 + PAGE_INSET;
    Some(Place {
        card_x: pane_x + col + VISIT_GAP,
        card_w: col,
        pane_x,
        pane_w: col,
        gap: VISIT_GAP,
    })
}

/// A list pane and its detail: the pane `LIST_PANE` wide at the left
/// margin, one gap, the detail to the right margin. `None` where the room
/// does not hold the pane and a detail at least as wide.
pub(super) fn list_detail(room: f32) -> Option<Place> {
    if room < 2.0 * (MARGIN_WIDE + LIST_PANE) + SPACER {
        return None;
    }
    let card_x = MARGIN_WIDE + LIST_PANE + SPACER;
    Some(Place {
        card_x,
        card_w: (room - MARGIN_WIDE - card_x).max(0.0),
        pane_x: MARGIN_WIDE,
        pane_w: LIST_PANE,
        gap: SPACER,
    })
}

/// A room too narrow for the pair: the card's column, a spacer and the
/// pane at its right, centred in the `room` beside the navigation. With
/// `guide` the card narrows from its widest first, then the pane, as far
/// as Guide's pane needs, and they stand left of its room.
pub(super) fn pane_at_right(room: f32, guide: bool) -> Place {
    let limit = if guide { room - guide_room() } else { room };
    let avail = limit - MARGIN_WIDE;
    let pane0 = side_panel_w(room);
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
        gap: SPACER,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rooms() -> [f32; 6] {
        [872.0, 1366.0, 1600.0, 1920.0, 2560.0, 3840.0]
    }

    #[test]
    fn pane_is_left_of_the_card_in_every_room_that_fits() {
        for guide in [false, true] {
            for room in rooms() {
                let Some(p) = pair(room, guide) else { continue };
                assert!(p.pane_x + p.pane_w + p.gap <= p.card_x + 0.01, "{room}");
                assert!(p.pane_x >= MARGIN_WIDE - 0.01, "{room}");
            }
        }
    }

    #[test]
    fn pair_stays_inside_its_room_and_beside_the_guide() {
        for guide in [false, true] {
            for room in rooms() {
                let Some(p) = pair(room, guide) else { continue };
                let end = p.card_x + p.card_w;
                let limit = if guide { room - guide_room() } else { room };
                assert!(end <= limit + 0.01, "{room} {guide}");
                if !guide {
                    assert!(end <= room - MARGIN_WIDE + 0.01, "{room}");
                }
            }
        }
    }

    #[test]
    fn a_wide_room_centres_the_pair_and_keeps_the_widest_sizes() {
        for room in [2560.0, 3840.0] {
            let p = pair(room, false).unwrap();
            assert_eq!(p.card_w, COLUMN_MAX);
            assert_eq!(p.pane_w, LIST_PANE);
            let right = room - (p.card_x + p.card_w);
            assert!((right - p.pane_x).abs() < 0.01);
        }
    }

    #[test]
    fn a_narrow_room_gives_the_card_its_least_before_the_pane_narrows() {
        let p = pair(872.0, false).unwrap();
        assert_eq!(p.card_w, FLOW_CARD_MIN);
        assert!(p.pane_w >= CHOICES_PANE_MIN);
        let q = pair(1366.0, false).unwrap();
        assert_eq!(q.pane_w, LIST_PANE);
        assert!(q.card_w >= FLOW_CARD_MIN);
    }

    #[test]
    fn equal_columns_are_as_wide_as_each_other_and_centred() {
        for room in [872.0, 1366.0, 1920.0, 3840.0] {
            let p = equal(room).unwrap();
            assert_eq!(p.pane_w, p.card_w);
            assert_eq!(p.card_x, p.pane_x + p.pane_w + VISIT_GAP);
            let right = room - (p.card_x + p.card_w);
            assert!((right - p.pane_x).abs() < 0.01, "{room}");
        }
        assert!(equal(800.0).is_none());
    }

    #[test]
    fn a_room_too_small_for_the_pair_has_none() {
        assert!(pair(800.0, false).is_none());
    }
}
