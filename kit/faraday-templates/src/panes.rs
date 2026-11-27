//! The pane container (09 §3.3, `docs/HUB-LAYOUT.md`): where a page's panes stand and how long a
//! change of page takes. A page declares its panes and the container places them; the page
//! places none itself. Built as far as one menu page on the device needs: the session pane at the
//! left of a tile (the pair) or above it (one column). The detail, equal and pane-at-right
//! placements, Guide's pane and the stack under This session's card are not built.

use faraday_atoms::{Pressed, Ui};
use faraday_organisms::session_pane::{self, SessionPaneModel};
use faraday_tokens::dims::{
    CHOICES_PANE_MIN, COLUMN_MAX, FLOW_CARD_MIN, LIST_PANE, MARGIN_WIDE, MENU_TILE_WIDE_MAX, PAD,
    READING_MENU_W, SPACER,
};
use faraday_tokens::motion::MOTION_MEDIUM2;

use crate::menu_page::grid;

/// The width rule of what stands at a pair's right.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tile {
    /// A step card: up to `COLUMN_MAX`.
    Card,
    /// The room, where it holds two columns or more, up to `MENU_TILE_WIDE_MAX`; a card's place
    /// where it holds one.
    Fill,
}

/// A page's panes, declared once beside its code: what stands at the left (the session pane's
/// model, or none) and the width rule of the tile.
pub struct Panes<'s, A> {
    pub left: Option<SessionPaneModel<'s, A>>,
    pub tile: Tile,
}

/// Where a pair's two panes stand, from the room's left edge.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Place {
    pub card_x: f32,
    pub card_w: f32,
    pub pane_x: f32,
    pub pane_w: f32,
    pub gap: f32,
}

/// The pair in a `room` wide: the pane `LIST_PANE` wide and the card the rest up to `COLUMN_MAX`;
/// in a narrower room the card narrows to `FLOW_CARD_MIN`, then the pane down to
/// `CHOICES_PANE_MIN`. `None` where the pair at its narrowest does not fit.
pub fn pair(room: f32) -> Option<Place> {
    let avail = room - 2.0 * MARGIN_WIDE;
    if avail < FLOW_CARD_MIN + SPACER + CHOICES_PANE_MIN {
        return None;
    }
    let card_w = (avail - SPACER - LIST_PANE).clamp(FLOW_CARD_MIN, COLUMN_MAX);
    let pane_w = (avail - SPACER - card_w).clamp(CHOICES_PANE_MIN, LIST_PANE);
    let unit = pane_w + SPACER + card_w;
    let pane_x = ((room - unit) / 2.0).max(MARGIN_WIDE);
    Some(Place {
        card_x: pane_x + pane_w + SPACER,
        card_w,
        pane_x,
        pane_w,
        gap: SPACER,
    })
}

/// The pair whose tile fills the room in two columns or more: the pane at the room's left page
/// margin, one gap, the tile to the right page margin and no wider than `MENU_TILE_WIDE_MAX`.
/// `None` where the room holds one column.
pub fn fill(room: f32) -> Option<Place> {
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

impl<A> Panes<'_, A> {
    /// Where the panes stand side by side in a `room` wide; `None` where the page has one
    /// column there.
    pub fn place(&self, room: f32) -> Option<Place> {
        self.left.as_ref()?;
        match self.tile {
            Tile::Fill => fill(room).or_else(|| pair(room)),
            Tile::Card => pair(room),
        }
    }
}

/// Draws the panes in the `room` wide from `x0`, from `y`: the session pane and the tile side by
/// side where the room holds the pair, else the pane above the tile in one column (the menu
/// page's width, centred). `tile` draws the tile in the place it is given, `(x, y, w)`, and
/// returns its height. Returns the height of both.
pub fn draw<A: Pressed>(
    m: &Panes<'_, A>,
    ui: &mut Ui<'_, A>,
    (x0, room): (f32, f32),
    y: f32,
    tile: impl FnOnce(&mut Ui<'_, A>, (f32, f32, f32)) -> f32,
) -> f32 {
    if let (Some(p), Some(left)) = (m.place(room), &m.left) {
        let pane = session_pane::draw(left, ui, (x0 + p.pane_x, y, p.pane_w));
        let tile = tile(ui, (x0 + p.card_x, y, p.card_w));
        return pane.max(tile);
    }
    let (ox, w) = if room > READING_MENU_W {
        ((room - READING_MENU_W) / 2.0, READING_MENU_W)
    } else {
        (0.0, room)
    };
    let above = m
        .left
        .as_ref()
        .map_or(0.0, |left| session_pane::draw(left, ui, (x0 + ox, y, w)));
    above + tile(ui, (x0 + ox, y + above, w))
}

/// How a change of page looks under the Motion setting.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Motion {
    Smooth,
    Crossfade,
    Off,
}

/// What a change of page does.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Turn {
    /// The two frames blend, eased.
    Crossfade,
    /// The old page goes out to the surface and the new one comes in.
    Through,
}

/// How long a change of a flow's step takes to fade through (`docs/FLOW-CONTAINER.md` §3):
/// Material's medium 2. Not yet timed on the Pi.
pub const STEP_MS: u64 = MOTION_MEDIUM2;

/// What a change of page does under `motion` and how long it takes at the Speed setting's
/// `speed_ms`; `None` for a change shown at once. The runtime keeps the frames and runs the time
/// (04 §8.3); this is where the rule is.
pub fn page_turn(motion: Motion, speed_ms: u64) -> Option<(Turn, u64)> {
    let turn = match motion {
        Motion::Smooth => Turn::Through,
        Motion::Crossfade => Turn::Crossfade,
        Motion::Off => return None,
    };
    Some((turn, speed_ms))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Invariant: The pane stands at the left of the card with the gap between, inside the room's margins, in every room that holds the pair, and a room too small for it has none.
    /// Regression: The session card drawn over the tile, or past the window's edge.
    /// Owner: kit
    #[test]
    fn the_pair_stands_pane_then_card_inside_the_room() {
        for room in [872.0, 1366.0, 1600.0, 1920.0, 2560.0, 3840.0] {
            let p = pair(room).unwrap();
            assert!(p.pane_x + p.pane_w + p.gap <= p.card_x + 0.01, "{room}");
            assert!(p.pane_x > 0.0, "{room}");
            assert!(p.card_x + p.card_w < room, "{room}");
        }
        assert!(pair(800.0).is_none());
    }

    /// Invariant: A change of page under Off is shown at once; Smooth goes through the surface and Crossfade blends, both for the Speed's time.
    /// Regression: A person who turned motion off still seeing a fade.
    /// Owner: kit
    #[test]
    fn a_page_turn_follows_the_motion_setting() {
        assert_eq!(page_turn(Motion::Off, 250), None);
        assert_eq!(page_turn(Motion::Smooth, 250), Some((Turn::Through, 250)));
        assert_eq!(
            page_turn(Motion::Crossfade, 400),
            Some((Turn::Crossfade, 400))
        );
    }
}
