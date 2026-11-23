//! The pair: Your choices at the left and the step card at its right, one
//! gap between, centred in the room beside the navigation
//! (`docs/WORKSPACE.md` §4, `docs/FLOW-CONTAINER.md` §4). Arithmetic on the
//! room alone, so a menu page's pair and a flow's pair stand at the same
//! place.

use super::unit::Place;
use faraday_draw::tokens::{
    CHOICES_PANE_MIN, COLUMN_MAX, FLOW_CARD_MIN, LIST_PANE, MARGIN_WIDE, SPACER,
};

/// The pair in a `room` wide, with `guide` when Guide's pane stands at the
/// room's right edge. The pane is `LIST_PANE` wide and the card takes the
/// rest up to `COLUMN_MAX`; in a narrower room the card narrows to
/// `FLOW_CARD_MIN`, then the pane down to `CHOICES_PANE_MIN`. With the
/// guide the pair stands left of its room, as far left as the room
/// needs. `None` where the pair at its narrowest does not fit.
pub(crate) fn pair(room: f32, guide: bool) -> Option<Place> {
    let limit = if guide {
        room - crate::screens::support_pane_room()
    } else {
        room
    };
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
                let limit = if guide {
                    room - crate::screens::support_pane_room()
                } else {
                    room
                };
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
    fn a_room_too_small_for_the_pair_has_none() {
        assert!(pair(800.0, false).is_none());
    }
}
