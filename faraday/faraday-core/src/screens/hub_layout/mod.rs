//! The pane container (`docs/HUB-LAYOUT.md`): where every page's panes
//! stand (`docs/WORKSPACE.md` §4), and how long a change of page takes. A
//! page or a flow declares its panes ([`Panes`]) and asks; it places none
//! itself. Arithmetic on the room; nothing here holds state.

mod lines;
mod page;
mod pair;
mod turn;

pub(crate) use lines::{card, divider, limits, pair_card, pair_card_ends};
pub(crate) use page::{draw, draw_details, draw_results, draw_results_alone, tile_surface};
pub(crate) use turn::{Kind, STEP_MS, page_turn};

use super::session_pane::Mode;
use faraday_draw::tokens::{
    CHOICES_PANE_MAX, CHOICES_PANE_MIN, CHOICES_PANE_SHARE, FLOW_CARD_MIN, MARGIN_WIDE,
    READING_MENU_W, SPACER, SUPPORT_PANE, SUPPORT_PANE_TOP, TOUCH,
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
    /// A detail beside a list pane: the pane at the room's left margin,
    /// the detail from its gap to the right margin, as wide as the room
    /// leaves; `None` where the room does not hold both, then the detail
    /// stands alone ([`detail_alone`]).
    Detail,
    /// Two columns as wide as each other, as far apart as `VISIT_GAP`, in
    /// the width Files reads at (`READING_VISIT_W`) between two
    /// `PAGE_INSET`s, centred in the room; `None` where the room is under
    /// the pair's least.
    Equal,
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

/// The panes of a page whose work fills its area and whose list is a panel
/// at its right (the backup board's, `docs/WORKSPACE.md` §4): it asks
/// only [`Panes::pane_beside_work`].
pub(crate) const WORK_PANES: Panes = Panes {
    left: Left::Own,
    tile: Tile::Card,
    stacks: false,
    narrow: Narrow::PaneAtRight,
};

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
        if let Tile::Detail = self.tile {
            return pair::list_detail(room);
        }
        if let Tile::Equal = self.tile {
            return pair::equal(room, guide && holds_guide(room));
        }
        let guide = guide && holds_guide(room);
        // A filled tile stands beside Guide's pane where it fits at that
        // width; else the tile takes its narrower form.
        if let Tile::Fill = self.tile
            && let Some(p) = pair::fill(room)
            && let Some(p) = if guide {
                pair::left_of_guide(p, room)
            } else {
                Some(p)
            }
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
pub(crate) fn guide_room() -> f32 {
    SUPPORT_PANE + SPACER + MARGIN_WIDE
}

/// Where a supporting pane stands (`docs/LAYOUT.md` §6, §15): the one
/// place for every pane at the right edge. For the page area at `x0`,
/// `cw` wide, in a window `h` tall: the pane's left edge, its top, its
/// width and the height it shows. Its right edge is `MARGIN_WIDE` from
/// the area's, which is the window's right edge, and its top is below the
/// top-right strip. Arithmetic.
pub(crate) fn guide_place((x0, cw, h): (f32, f32, f32)) -> (f32, f32, f32, f32) {
    guide_place_at(x0 + cw - MARGIN_WIDE - SUPPORT_PANE, h)
}

/// The supporting pane's place with its left edge at `x`, in a window `h`
/// tall: for a page that stands the pane a spacer from its own area's
/// right edge (the dice sheet). Arithmetic.
pub(crate) fn guide_place_at(x: f32, h: f32) -> (f32, f32, f32, f32) {
    (
        x,
        SUPPORT_PANE_TOP,
        SUPPORT_PANE,
        (h - SUPPORT_PANE_TOP - MARGIN_WIDE).max(TOUCH),
    )
}

/// The widest side panel a page's column can have beside it (a quick
/// paper backup's pair is centred by it).
pub(crate) fn side_panel_widest() -> f32 {
    side_panel_w(f32::INFINITY)
}

/// Where a detail stands alone in a `room`, the list pane given way
/// ([`Tile::Detail`]): from margin to margin, as (left edge, width).
pub(crate) fn detail_alone(room: f32) -> (f32, f32) {
    (MARGIN_WIDE, (room - 2.0 * MARGIN_WIDE).max(0.0))
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
