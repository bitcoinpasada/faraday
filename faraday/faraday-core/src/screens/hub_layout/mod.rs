//! The pane container (`docs/HUB-LAYOUT.md`): where every page's panes
//! stand (`docs/WORKSPACE.md` §4), and how long a change of page takes. A
//! page or a flow declares its panes ([`Panes`]) and asks; it places none
//! itself. Arithmetic on the room; nothing here holds state.

mod lines;
mod page;
mod pair;
mod turn;

pub(crate) use lines::{limits, lines};
pub(crate) use page::{draw, draw_results};
pub(crate) use turn::{Kind, STEP_MS, page_turn};

use super::session_pane::Mode;
use faraday_draw::tokens::{
    CHOICES_PANE_MAX, CHOICES_PANE_MIN, CHOICES_PANE_SHARE, FLOW_CARD_MIN, MARGIN_WIDE,
    READING_MENU_W, SPACER,
};

/// What stands at a pair's left.
#[derive(Clone, Copy)]
pub(crate) enum Left {
    /// The session pane, drawn by the container, in a page's mode, or with
    /// Home's rows for `None`.
    Session(Option<Mode>),
    /// A pane the page or flow draws itself at the place it is given.
    Own,
}

/// The width rule of what stands at a pair's right.
#[derive(Clone, Copy)]
pub(crate) enum Tile {
    /// A step card: up to `COLUMN_MAX`.
    Card,
    /// The room, where it holds two columns or more, up to
    /// `MENU_TILE_WIDE_MAX`; a card's place where it holds one. The room
    /// it is given is the one Guide's pane leaves.
    Fill,
}

/// What a room too narrow for the pair gets.
#[derive(Clone, Copy)]
pub(crate) enum Narrow {
    /// The card, then its pane at the right.
    PaneAtRight,
    /// No pair: the page draws its one column.
    OneColumn,
}

/// A page's or a flow's panes, declared once beside its code.
#[derive(Clone, Copy)]
pub(crate) struct Panes {
    pub(crate) left: Left,
    pub(crate) tile: Tile,
    /// Whether the tile stands under This session's card where the window
    /// has the height.
    pub(crate) stacks: bool,
    pub(crate) narrow: Narrow,
}

/// Where a pair's two panes stand, from the room's left edge.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Place {
    /// The card's left edge.
    pub(crate) card_x: f32,
    /// The card's width.
    pub(crate) card_w: f32,
    /// The pane's left edge.
    pub(crate) pane_x: f32,
    /// The pane's width.
    pub(crate) pane_w: f32,
    /// The gap between the card and the pane.
    pub(crate) gap: f32,
}

impl Place {
    /// Whether the pane stands at the card's left: the pair.
    pub(crate) fn pane_left(&self) -> bool {
        self.pane_x < self.card_x
    }
}

impl Panes {
    /// Where the panes stand side by side in a `room` wide, with `guide`
    /// when Guide's pane stands at the room's right edge; `None` where the
    /// page has one column there.
    pub(crate) fn place(&self, room: f32, guide: bool) -> Option<Place> {
        let guide = guide && holds_guide(room);
        if let (Tile::Fill, false) = (self.tile, guide)
            && let Some(p) = pair::fill(room)
        {
            return Some(p);
        }
        pair::pair(room, guide).or_else(|| match self.narrow {
            Narrow::PaneAtRight => Some(pair::pane_at_right(room, guide)),
            Narrow::OneColumn => None,
        })
    }

    /// The tile's top, stacked under This session's card `card_h` tall
    /// from `top`, where the page stacks and the tile `tile_h` tall ends a
    /// page margin above the foot of a window `h` tall.
    fn stack(&self, top: f32, card_h: f32, tile_h: f32, h: f32) -> Option<f32> {
        let under = top + card_h + SPACER;
        (self.stacks && under + tile_h + MARGIN_WIDE <= h).then_some(under)
    }

    /// The pane's width at the right of content that fills an area
    /// `area_w` wide.
    pub(crate) fn pane_beside_work(&self, area_w: f32) -> f32 {
        side_panel_w(area_w)
    }
}

/// Where a pair ends, level: the longer of its two sides, at most `most`.
pub(crate) fn level(a: f32, b: f32, most: f32) -> f32 {
    a.max(b).min(most)
}

/// The room Guide's pane takes at the right edge: the pane, the spacer
/// before it and the margin after it.
fn guide_room() -> f32 {
    crate::screens::support_pane_room()
}

/// Whether a `room` beside the navigation holds a flow's pair at its
/// narrowest and Guide's pane beside it.
pub(crate) fn holds_guide(room: f32) -> bool {
    room >= MARGIN_WIDE + FLOW_CARD_MIN + SPACER + CHOICES_PANE_MIN + guide_room()
}

/// The side panel beside a step-by-step column (Spend, Back up, Create a
/// vault): 320 wide in a full-size window, down to 240 as the window
/// narrows, so that the column keeps its room.
pub(crate) fn side_panel_w(cw: f32) -> f32 {
    (cw * CHOICES_PANE_SHARE).clamp(CHOICES_PANE_MIN, CHOICES_PANE_MAX)
}

/// The area Home has in a room `room` wide: how far from the room's left
/// edge and how wide. A page with no pair in this room stands in it.
pub(crate) fn home_area(room: f32) -> (f32, f32) {
    if room > READING_MENU_W {
        ((room - READING_MENU_W) / 2.0, READING_MENU_W)
    } else {
        (0.0, room)
    }
}
