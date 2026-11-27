//! The pane container (09 §3.3, `docs/HUB-LAYOUT.md`): where a page's panes stand and how long
//! a change of page takes. A page declares its panes and the container places them; the page
//! places none itself. It owns where the pair stands (the session pane at the left of a tile, a
//! detail beside a list, two equal columns, the pane at the right of a card), the tile stacked
//! under This session's card, the level ends of a pair, the dividers, Guide's pane at the right
//! edge and the rule of a change of page.
//!
//! The pane container reports where the page stood, as `Laid::Placed`, when the page's
//! [`Parts`] ask for it.

use faraday_atoms::region::Region;
use faraday_atoms::{Anchor, Laid, Placement, Pressed, Ui};
use faraday_organisms::row_details::{self, RowDetailsModel};
use faraday_organisms::session_pane::{self, SessionPaneModel};
use faraday_tokens::dims::{
    CHOICES_PANE_MAX, CHOICES_PANE_MIN, CHOICES_PANE_SHARE, COLUMN_MAX, FLOW_CARD_MIN, LIST_PANE,
    MARGIN_WIDE, MENU_TILE_WIDE_MAX, PAD, PAGE_INSET, PANE_TOP_BAR, READING_MENU_W,
    READING_VISIT_W, SHAPE_L, SHAPE_M, SPACER, SUPPORT_PANE, SUPPORT_PANE_TOP, TOUCH, VISIT_GAP,
};
use faraday_tokens::elevation::{ELEVATION_FAB, ELEVATION_SHEET};
use faraday_tokens::motion::MOTION_MEDIUM2;
use faraday_tokens::palette::Role;

use crate::menu_page::{grid, page_span};
use crate::screen::page_room;

/// The width rule of what stands at a pair's right.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tile {
    /// A step card: up to `COLUMN_MAX`.
    Card,
    /// The room, where it holds two columns or more, up to `MENU_TILE_WIDE_MAX`; a card's place
    /// where it holds one. The room it is given is the one Guide's pane leaves.
    Fill,
    /// A detail beside a list pane: the pane at the room's left margin, the detail from its gap
    /// to the right margin; `None` where the room does not hold both, then the detail stands
    /// alone ([`detail_alone`]).
    Detail,
    /// Two columns as wide as each other, `VISIT_GAP` apart, in the width Files reads at between
    /// two `PAGE_INSET`s, centred in the room; `None` under the pair's least.
    Equal,
}

/// What stands at a pair's left.
pub enum Left<'s, A> {
    /// The session pane, drawn by the container.
    Session(SessionPaneModel<'s, A>),
    /// A pane the page draws itself at the place it is given ([`Parts::own`]).
    Own,
}

/// What a room too narrow for the pair gets.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Narrow {
    /// The card, then its pane at the right.
    PaneAtRight,
    /// No pair: the page draws its one column.
    OneColumn,
}

/// A page's panes, declared once beside its code: what stands at the left (the session pane's
/// model, or a pane of the page's own) and the width rule of the tile.
pub struct Panes<'s, A> {
    pub left: Left<'s, A>,
    pub tile: Tile,
    /// Whether the tile stands under This session's card where the window has the height.
    pub stacks: bool,
    pub narrow: Narrow,
}

/// A pane the page draws itself, at `(x, y, w)`; returns its height.
pub type OwnPane<'a, A> = &'a mut dyn FnMut(&mut Ui<'_, A>, (f32, f32, f32)) -> f32;
/// Guide's pane, drawn at `(x, y, w, h)`.
pub type GuidePane<'a, A> = &'a mut dyn FnMut(&mut Ui<'_, A>, (f32, f32, f32, f32));
/// Where the page stood: the laid fact's anchor and the message it becomes.
pub type OnPlaced<A> = (Anchor, fn(Laid) -> A);

/// What a page gives the container besides its [`Panes`]: all of it optional, none by default
/// ([`Parts::none`]).
pub struct Parts<'a, 's, A: Pressed> {
    /// The page's own left pane, drawn at the place it is given, `(x, y, w)`; returns its height.
    pub own: Option<OwnPane<'a, A>>,
    /// The open row's details, which stand in the tile's place: under the card where they fit,
    /// else beside it.
    pub details: Option<&'a RowDetailsModel<'s, A>>,
    /// Guide's pane, drawn in the place it is given, `(x, y, w, h)`, where the room holds it
    /// beside the pair.
    pub guide: Option<GuidePane<'a, A>>,
    /// Where the page stood, reported as `Laid::Placed`.
    pub on_placed: Option<OnPlaced<A>>,
    /// The scroll of the tile in a pair, which scrolls within the window's height.
    pub scroll: Option<Anchor>,
}

impl<A: Pressed> Parts<'_, '_, A> {
    /// Nothing besides the panes.
    pub fn none() -> Self {
        Parts {
            own: None,
            details: None,
            guide: None,
            on_placed: None,
            scroll: None,
        }
    }
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

impl Place {
    /// Whether the pane stands at the card's left: the pair.
    pub fn pane_left(&self) -> bool {
        self.pane_x < self.card_x
    }
}

/// The room Guide's pane takes at the right edge: the pane, the spacer before it and the margin
/// after it.
pub fn guide_room() -> f32 {
    SUPPORT_PANE + SPACER + MARGIN_WIDE
}

/// Whether a `room` beside the navigation holds a flow's pair at its narrowest and Guide's pane
/// beside it.
pub fn holds_guide(room: f32) -> bool {
    room >= MARGIN_WIDE + FLOW_CARD_MIN + SPACER + CHOICES_PANE_MIN + guide_room()
}

/// Where a supporting pane stands: for the page area at `x0`, `cw` wide, in a window `h` tall,
/// the pane's left edge, its top, its width and the height it shows. Its right edge is
/// `MARGIN_WIDE` from the area's, and its top is below the top-right strip.
pub fn guide_place((x0, cw, h): (f32, f32, f32)) -> (f32, f32, f32, f32) {
    guide_place_at(x0 + cw - MARGIN_WIDE - SUPPORT_PANE, h)
}

/// The supporting pane's place with its left edge at `x`, in a window `h` tall.
pub fn guide_place_at(x: f32, h: f32) -> (f32, f32, f32, f32) {
    (
        x,
        SUPPORT_PANE_TOP,
        SUPPORT_PANE,
        (h - SUPPORT_PANE_TOP - MARGIN_WIDE).max(TOUCH),
    )
}

/// The side panel beside a step-by-step column: as wide as its share of `cw`, between the
/// choices pane's least and most.
pub fn side_panel_w(cw: f32) -> f32 {
    (cw * CHOICES_PANE_SHARE).clamp(CHOICES_PANE_MIN, CHOICES_PANE_MAX)
}

/// The widest side panel a page's column can have beside it.
pub fn side_panel_widest() -> f32 {
    side_panel_w(f32::INFINITY)
}

/// Where a detail stands alone in a `room`, the list pane given way: from margin to margin, as
/// (left edge, width).
pub fn detail_alone(room: f32) -> (f32, f32) {
    (MARGIN_WIDE, (room - 2.0 * MARGIN_WIDE).max(0.0))
}

/// The area Home has in a room `room` wide: how far from the room's left edge and how wide.
pub fn home_area(room: f32) -> (f32, f32) {
    if room > READING_MENU_W {
        ((room - READING_MENU_W) / 2.0, READING_MENU_W)
    } else {
        (0.0, room)
    }
}

/// Where a pair ends, level: the longer of its two sides, at most `most`.
pub fn level(a: f32, b: f32, most: f32) -> f32 {
    a.max(b).min(most)
}

/// The pair in a `room` wide: the pane `LIST_PANE` wide and the card the rest up to
/// `COLUMN_MAX`; in a narrower room the card narrows to `FLOW_CARD_MIN`, then the pane down to
/// `CHOICES_PANE_MIN`. `None` where the pair at its narrowest does not fit.
pub fn pair(room: f32) -> Option<Place> {
    pair_by(room, false)
}

/// [`pair`] with `guide` when Guide's pane stands at the room's right edge: the pair then
/// stands left of its room, as far left as the room needs.
pub fn pair_by(room: f32, guide: bool) -> Option<Place> {
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

/// `p` as it stands beside Guide's pane in a `room`: where it is when it leaves the pane's room
/// at its right, else moved left as far as that needs; `None` where it does not fit at its
/// width.
pub fn left_of_guide(p: Place, room: f32) -> Option<Place> {
    let over = (p.card_x + p.card_w - (room - guide_room())).max(0.0);
    (p.pane_x - over >= MARGIN_WIDE).then_some(Place {
        pane_x: p.pane_x - over,
        card_x: p.card_x - over,
        ..p
    })
}

/// The pair of two equal columns (Files): the room up to `READING_VISIT_W`, centred, a
/// `PAGE_INSET` inside it at each side, the two columns a `VISIT_GAP` apart. `None` under the
/// pair's least.
pub fn equal(room: f32, guide: bool) -> Option<Place> {
    if room < 2.0 * MARGIN_WIDE + FLOW_CARD_MIN + SPACER + CHOICES_PANE_MIN {
        return None;
    }
    // With Guide's pane the columns stand left of its room, narrowing and moving only as far as
    // it needs.
    let limit = if guide { room - guide_room() } else { room };
    let edge = if guide { limit + PAGE_INSET } else { limit };
    let reading = edge.min(READING_VISIT_W);
    let col = (reading - 2.0 * PAGE_INSET - VISIT_GAP) / 2.0;
    let pane_x = ((room - reading) / 2.0).min(edge - reading) + PAGE_INSET;
    Some(Place {
        card_x: pane_x + col + VISIT_GAP,
        card_w: col,
        pane_x,
        pane_w: col,
        gap: VISIT_GAP,
    })
}

/// A list pane and its detail: the pane `LIST_PANE` wide at the left margin, one gap, the detail
/// to the right margin. `None` where the room does not hold the pane and a detail at least as
/// wide.
pub fn list_detail(room: f32) -> Option<Place> {
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

/// A room too narrow for the pair: the card's column, a spacer and the pane at its right,
/// centred in the `room`. With `guide` the card narrows from its widest first, then the pane, as
/// far as Guide's pane needs, and they stand left of its room.
pub fn pane_at_right(room: f32, guide: bool) -> Place {
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

impl<A> Panes<'_, A> {
    /// Where the panes stand side by side in a `room` wide; `None` where the page has one
    /// column there.
    pub fn place(&self, room: f32) -> Option<Place> {
        self.place_by(room, false)
    }

    /// [`Panes::place`] with `guide` when Guide's pane stands at the room's right edge.
    pub fn place_by(&self, room: f32, guide: bool) -> Option<Place> {
        let guide = guide && holds_guide(room);
        match self.tile {
            Tile::Detail => return list_detail(room),
            Tile::Equal => return equal(room, guide),
            // A filled tile stands beside Guide's pane where it fits at that width; else the
            // tile takes its narrower form.
            Tile::Fill => {
                if let Some(p) = fill(room)
                    && let Some(p) = if guide {
                        left_of_guide(p, room)
                    } else {
                        Some(p)
                    }
                {
                    return Some(p);
                }
            }
            Tile::Card => {}
        }
        pair_by(room, guide).or_else(|| match self.narrow {
            Narrow::PaneAtRight => Some(pane_at_right(room, guide)),
            Narrow::OneColumn => None,
        })
    }

    /// The pair's place in an area `cw` wide that stands `ox` into the `room` beside the
    /// navigation, from the area's left edge. Beside Guide's pane an area narrower than the room
    /// (Home's) keeps the place it has without the guide and moves left only as far as the pane
    /// needs; where it cannot, the pair is placed beside the pane in the whole room.
    pub fn place_in_area(&self, room: f32, (ox, cw): (f32, f32), guide: bool) -> Option<Place> {
        if !guide || cw >= room {
            return self.place_by(cw, guide);
        }
        let into = |p: Place, by: f32| Place {
            card_x: p.card_x + by,
            pane_x: p.pane_x + by,
            ..p
        };
        self.place_by(cw, false)
            .and_then(|p| left_of_guide(into(p, ox), room))
            .or_else(|| self.place_by(room, true))
            .map(|p| into(p, -ox))
    }

    /// The tile's top, stacked under This session's card `card_h` tall from `top`, where the
    /// page stacks and the tile `tile_h` tall ends a page margin above the foot of a window `h`
    /// tall.
    pub fn stack(&self, top: f32, card_h: f32, tile_h: f32, h: f32) -> Option<f32> {
        let under = top + card_h + SPACER;
        (self.stacks && under + tile_h + MARGIN_WIDE <= h).then_some(under)
    }

    /// The pane's width at the right of content that fills an area `area_w` wide.
    pub fn pane_beside_work(&self, area_w: f32) -> f32 {
        side_panel_w(area_w)
    }
}

// ---- the lines of a pair ----

/// The tile's surface: raised at level 1 as the cards beside it, an outline in its place at High
/// contrast (which casts no shadow).
pub fn tile_surface<A: Pressed>(ui: &mut Ui<'_, A>, (x, y, w, h): (f32, f32, f32, f32)) {
    ui.shadow(x, y, w, h, SHAPE_M, ELEVATION_SHEET);
    ui.fill(x, y, w, h, SHAPE_M, Role::SurfaceContainerLow);
    if ui.theme.is_high_contrast() {
        ui.stroke(x, y, w, h, SHAPE_M, Role::Outline);
    }
}

/// How far the pair's limit stands above the card's top: the heading starts a padding under the
/// top bar, so it stands half a padding under it.
const OUTLINE_ABOVE: f32 = PAD / 2.0;

/// A flow's one card: from `from` to `to` (the corners, x and y), filled a step off the page at
/// level 3, with a thin outline at High contrast only. Drawn before what stands on it.
pub fn card<A: Pressed>(ui: &mut Ui<'_, A>, from: (f32, f32), to: (f32, f32)) {
    let (x, y) = from;
    let (w, h) = (to.0 - x, to.1 - y);
    if h <= 0.0 {
        return;
    }
    ui.shadow(x, y, w, h, SHAPE_L, ELEVATION_FAB);
    ui.fill(x, y, w, h, SHAPE_L, Role::SurfaceContainerLow);
    if ui.theme.is_high_contrast() {
        ui.stroke(x, y, w, h, SHAPE_L, Role::OutlineVariant);
    }
}

/// The raised card of a pair on a page that scrolls whole: a padding outside `edges` (the
/// panes' left and right) and above `top`, where their content starts, down to a padding under
/// `end`, where the longer side ends. The content is measured first, so the card is drawn to
/// the foot it has this frame, never one kept from the last.
pub fn pair_card<A: Pressed>(
    ui: &mut Ui<'_, A>,
    (left, right): (f32, f32),
    (top, end): (f32, f32),
) {
    card(ui, (left - PAD, top - PAD), (right + PAD, end + PAD));
}

/// The card's top and foot limits: where the pair stands, its card's top and the line a padding
/// above the window's foot; else the page's.
pub fn limits(left: bool, compact: bool, top: f32, view_h: f32) -> (f32, f32) {
    if left {
        (top - OUTLINE_ABOVE, view_h - PAD)
    } else {
        (if compact { top } else { 0.0 }, view_h)
    }
}

/// One card's divider: at `at`, the middle of the gap, inset by the card's padding, from `top` to
/// where the taller side ends.
pub fn divider<A: Pressed>(ui: &mut Ui<'_, A>, (at, top, end): (f32, f32, f32)) {
    if end - top > 2.0 * PAD {
        ui.fill(
            at,
            top + PAD,
            1.0,
            end - top - 2.0 * PAD,
            0.0,
            Role::OutlineVariant,
        );
    }
}

// ---- drawing the panes ----

/// Draws the panes in the `room` wide from `x0`, from `y`: the session pane and the tile side by
/// side where the room holds the pair, else the pane above the tile in one column (the menu
/// page's width, centred). `tile` draws the tile in the place it is given, `(x, y, w)`, and
/// returns its height. Returns the height of both.
pub fn draw<A: Pressed>(
    m: &Panes<'_, A>,
    ui: &mut Ui<'_, A>,
    (x0, room): (f32, f32),
    y: f32,
    tile: impl Fn(&mut Ui<'_, A>, (f32, f32, f32)) -> f32,
) -> f32 {
    draw_with(m, ui, (x0, room), y, Parts::none(), tile)
}

/// [`draw`] with what the page gives besides its panes: its own left pane, the open row's
/// details, Guide's pane, the report of where the page stood and the tile's scroll.
pub fn draw_with<A: Pressed>(
    m: &Panes<'_, A>,
    ui: &mut Ui<'_, A>,
    (x0, room): (f32, f32),
    y: f32,
    mut parts: Parts<'_, '_, A>,
    tile: impl Fn(&mut Ui<'_, A>, (f32, f32, f32)) -> f32,
) -> f32 {
    let (_, h) = page_room(ui);
    let guide = parts.guide.is_some() && holds_guide(room);
    let report = |ui: &mut Ui<'_, A>, parts: &Parts<'_, '_, A>, placement: Placement| {
        if let Some((id, to)) = parts.on_placed {
            ui.laid(id, Laid::Placed(placement), to);
        }
    };
    if guide && let Some(g) = parts.guide.as_deref_mut() {
        g(ui, guide_place((x0, room, h)));
    }
    // The area the page has beside Guide's pane: Home's span within the room the pane leaves.
    let beside = if guide { room - guide_room() } else { room };
    let (hx, hw) = {
        let (ox, area) = home_area(beside);
        page_span(false, ox, area)
    };
    let details_h = |ui: &mut Ui<'_, A>, d: &RowDetailsModel<'_, A>, w: f32| {
        row_details::content_h(d, ui, w - 2.0 * PAD) + 2.0 * PAD
    };
    // Stacked: the tile under This session's card where it all fits.
    if let Left::Session(left) = &m.left
        && m.stacks
    {
        let card_h = session_pane::height(left, ui, hw);
        let tile_h = match parts.details {
            Some(d) => details_h(ui, d, hw),
            None => ui.measured(|ui| tile(ui, (0.0, 0.0, hw))),
        };
        if let Some(under) = m.stack(y, card_h, tile_h, h) {
            report(ui, &parts, Placement::Stacked);
            let first = session_pane::draw_stacked(left, ui, (x0 + hx, y, hw), h - y - MARGIN_WIDE);
            let under = if left.scan.is_some() {
                y + first + SPACER
            } else {
                under
            };
            return match parts.details {
                // Details scroll in their own card, under a card that stands still.
                Some(d) => {
                    let rest = (h - MARGIN_WIDE - under).max(tile_h);
                    under - y + row_details::draw(d, ui, (x0 + hx, under, hw, rest))
                }
                None => under - y + tile(ui, (x0 + hx, under, hw)),
            };
        }
    }
    let placed = m.place_in_area(room, (0.0, beside), guide);
    let Some(p) = placed else {
        report(ui, &parts, Placement::OneColumn);
        return one_column(m, ui, (x0, room), y, &tile);
    };
    report(ui, &parts, Placement::Beside);
    let tx = x0 + p.card_x;
    let avail = (h - y - MARGIN_WIDE).max(0.0);
    // A row's details stand in the tile's place, both panes the window's height.
    if let Some(d) = parts.details {
        match &m.left {
            Left::Session(left) => {
                session_pane::draw_page(left, ui, (x0 + p.pane_x, y, p.pane_w, avail))
            }
            Left::Own => {
                if let Some(own) = parts.own.as_deref_mut() {
                    own(ui, (x0 + p.pane_x, y, p.pane_w));
                }
            }
        }
        row_details::draw(d, ui, (tx, y, p.card_w, avail));
        return avail;
    }
    let tile_need = ui.measured(|ui| tile(ui, (0.0, 0.0, p.card_w)));
    let both = match &m.left {
        Left::Session(left) => {
            let pane_need = session_pane::page_content_h(left, ui, p.pane_w);
            let both = level(pane_need, tile_need, avail);
            session_pane::draw_page(left, ui, (x0 + p.pane_x, y, p.pane_w, both));
            both
        }
        Left::Own => {
            let own_h = parts
                .own
                .as_deref_mut()
                .map_or(0.0, |own| own(ui, (x0 + p.pane_x, y, p.pane_w)));
            own_h.max(tile_need)
        }
    };
    match parts.scroll {
        Some(id) if tile_need > both => {
            let clip = ui.rect(tx, y, p.card_w, both);
            ui.region(Region::new(id, clip), |ui| {
                tile(ui, (tx, y, p.card_w));
                tile_need - both
            });
        }
        _ => {
            tile(ui, (tx, y, p.card_w));
        }
    }
    both
}

/// The pane above the tile in the menu page's one column, centred.
fn one_column<A: Pressed>(
    m: &Panes<'_, A>,
    ui: &mut Ui<'_, A>,
    (x0, room): (f32, f32),
    y: f32,
    tile: &impl Fn(&mut Ui<'_, A>, (f32, f32, f32)) -> f32,
) -> f32 {
    let (ox, w) = if room > READING_MENU_W {
        ((room - READING_MENU_W) / 2.0, READING_MENU_W)
    } else {
        (0.0, room)
    };
    let above = match &m.left {
        Left::Session(left) => session_pane::draw(left, ui, (x0 + ox, y, w)),
        // A page with a pane of its own draws it in the column itself.
        Left::Own => 0.0,
    };
    above + tile(ui, (x0 + ox, y + above, w))
}

/// Draws the body alone in the place of a flow's card (a typed search's results): no This
/// session beside it. `body` draws from `(x, y)`, `w` wide, and returns its height; the tile
/// stands raised and padded, from the pair's left edge to its right if the room holds the pair,
/// else margin to margin, and scrolls whole in `scroll` within an area `cw` wide from `x0`, `h`
/// tall. Returns the content's height.
pub fn draw_alone<A: Pressed>(
    ui: &mut Ui<'_, A>,
    scroll: Anchor,
    (x0, cw, h): (f32, f32, f32),
    guide: bool,
    body: impl Fn(&mut Ui<'_, A>, (f32, f32), f32) -> f32,
) -> f32 {
    let (cx, w) = pair_by(cw, guide).map_or_else(
        || detail_alone(cw),
        |p| {
            let left = p.pane_x - PAD;
            (left, p.card_x + p.card_w + PAD - left)
        },
    );
    let top = PANE_TOP_BAR;
    let inner = |ui: &mut Ui<'_, A>, x: f32, y: f32| {
        body(ui, (x + PAD, y + PAD), w - 2.0 * PAD) + 2.0 * PAD
    };
    let tile_h = ui.measured(|ui| inner(ui, 0.0, 0.0));
    let clip = ui.rect(x0, 0.0, cw, h);
    let content = top + tile_h + MARGIN_WIDE;
    ui.region(Region::new(scroll, clip), |ui| {
        tile_surface(ui, (x0 + cx, top, w, tile_h));
        inner(ui, x0 + cx, top);
        content - h
    });
    content
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

/// How far a change of page begun at `began_ms` has come at the frame's time, 0 to 1, and what
/// it does: `None` for a change shown at once (Motion Off, or Reduce motion) or one that is
/// over. The time is the frame's (`Ui::now_ms`); the blending of the kept frames is the
/// runtime's (04 §8.3), eased by the emphasized curve.
pub fn turn_at<A: Pressed>(
    ui: &Ui<'_, A>,
    (motion, speed_ms): (Motion, u64),
    began_ms: u64,
) -> Option<(Turn, f32)> {
    if ui.reduce_motion() {
        return None;
    }
    let (turn, ms) = page_turn(motion, speed_ms)?;
    let t = faraday_atoms::motion::progress(began_ms, ui.now_ms(), ms);
    (t < 1.0).then_some((turn, t))
}

#[cfg(test)]
mod tests {
    use super::*;
    use faraday_atoms::ui::testing::{P, at_rest, canvas, frame, frame_in};
    use faraday_atoms::{MotionStore, Shown};

    const ROOMS: [f32; 6] = [872.0, 1366.0, 1600.0, 1920.0, 2560.0, 3840.0];

    fn session<'s>() -> SessionPaneModel<'s, P> {
        SessionPaneModel::empty(
            Shown::plain("This session"),
            Shown::plain("Nothing loaded"),
            false,
        )
    }

    fn panes<'s>(tile: Tile, stacks: bool, narrow: Narrow) -> Panes<'s, P> {
        Panes {
            left: Left::Session(session()),
            tile,
            stacks,
            narrow,
        }
    }

    /// Invariant: The pane stands at the left of the card with the gap between, inside the room's margins, in every room that holds the pair, and a room too small for it has none.
    /// Regression: The session card drawn over the tile, or past the window's edge.
    /// Owner: kit
    #[test]
    fn the_pair_stands_pane_then_card_inside_the_room() {
        for room in ROOMS {
            let p = pair(room).unwrap();
            assert!(p.pane_x + p.pane_w + p.gap <= p.card_x + 0.01, "{room}");
            assert!(p.pane_x > 0.0, "{room}");
            assert!(p.card_x + p.card_w < room, "{room}");
        }
        assert!(pair(800.0).is_none());
    }

    /// Invariant: Beside Guide's pane the pair ends where the pane's room begins, and a room that cannot hold both has no guide.
    /// Regression: A flow's card drawn under Guide's pane at the right edge.
    /// Owner: kit
    #[test]
    fn the_pair_beside_guide_ends_before_its_room() {
        for room in ROOMS {
            if !holds_guide(room) {
                continue;
            }
            let p = pair_by(room, true).unwrap();
            assert!(p.card_x + p.card_w <= room - guide_room() + 0.01, "{room}");
            assert!(p.pane_x >= 0.0);
        }
        assert!(!holds_guide(900.0));
    }

    /// Invariant: In a wide room the pair is centred and holds the same widest sizes however wide the room; in a narrow one the card keeps its least before the pane narrows.
    /// Regression: A pair that stretches with the window, or a pane squeezed while the card has room to give.
    /// Owner: kit
    #[test]
    fn a_wide_room_centres_the_pair_and_a_narrow_one_narrows_the_pane_last() {
        let (a, b) = (pair(2560.0).unwrap(), pair(3840.0).unwrap());
        assert_eq!((a.card_w, a.pane_w), (b.card_w, b.pane_w));
        for p in [a, b] {
            let room = if p == a { 2560.0 } else { 3840.0 };
            assert!(((room - (p.card_x + p.card_w)) - p.pane_x).abs() < 0.01);
        }
        let narrow = pair(872.0).unwrap();
        let wide = pair(1366.0).unwrap();
        assert!(narrow.card_w < wide.card_w);
        assert!(narrow.pane_w <= wide.pane_w);
        assert!(narrow.pane_w >= wide.pane_w * 0.5);
    }

    /// Invariant: Equal columns are as wide as each other, the gap between them is the visit gap, and they are centred in the room.
    /// Regression: Files' two columns of different widths, or off-centre.
    /// Owner: kit
    #[test]
    fn equal_columns_are_as_wide_as_each_other_and_centred() {
        for room in [872.0, 1366.0, 1920.0, 3840.0] {
            let p = equal(room, false).unwrap();
            assert_eq!(p.pane_w, p.card_w);
            assert_eq!(p.card_x, p.pane_x + p.pane_w + p.gap);
            assert!(
                ((room - (p.card_x + p.card_w)) - p.pane_x).abs() < 0.01,
                "{room}"
            );
        }
        assert!(equal(800.0, false).is_none());
    }

    /// Invariant: A list and its detail take the room from margin to margin with the list at the left; a room too small for both has none and the detail stands alone from margin to margin.
    /// Regression: A detail drawn over its list pane.
    /// Owner: kit
    #[test]
    fn a_list_and_its_detail_fill_the_room_or_the_detail_stands_alone() {
        let room = 1600.0;
        let p = list_detail(room).unwrap();
        assert!(p.pane_x + p.pane_w + p.gap <= p.card_x + 0.01);
        assert!((room - (p.card_x + p.card_w) - p.pane_x).abs() < 0.01);
        assert!(list_detail(600.0).is_none());
        let (x, w) = detail_alone(600.0);
        assert!((600.0 - (x + w) - x).abs() < 0.01);
    }

    /// Invariant: A room too narrow for the pair gives the card its column with the pane at its right, inside the room, where the page asks for it; a page that asks for one column gets none.
    /// Regression: A flow's side list lost on a narrow window.
    /// Owner: kit
    #[test]
    fn a_narrow_room_puts_the_pane_at_the_right_of_the_card_when_asked() {
        let room = 800.0;
        let at_right = panes(Tile::Card, false, Narrow::PaneAtRight);
        let p = at_right.place(room).unwrap();
        assert!(p.card_x + p.card_w + p.gap <= p.pane_x + 0.01);
        assert!(p.pane_x + p.pane_w <= room);
        assert!(
            panes(Tile::Card, false, Narrow::OneColumn)
                .place(room)
                .is_none()
        );
    }

    /// Invariant: The tile stacks under the card only where the page asks for it and the window holds the card, the tile and a margin; the pair's two sides end level at the longer, never past the room.
    /// Regression: A tile stacked off the window's foot, or a pair whose two cards end at different heights.
    /// Owner: kit
    #[test]
    fn the_stack_needs_the_height_and_the_pair_ends_level() {
        let stacking = panes(Tile::Fill, true, Narrow::OneColumn);
        assert!(stacking.stack(8.0, 100.0, 200.0, 800.0).is_some());
        assert!(stacking.stack(8.0, 100.0, 700.0, 800.0).is_none());
        assert!(
            panes(Tile::Fill, false, Narrow::OneColumn)
                .stack(8.0, 100.0, 200.0, 800.0)
                .is_none()
        );
        assert_eq!(level(300.0, 500.0, 900.0), 500.0);
        assert_eq!(level(300.0, 500.0, 400.0), 400.0);
    }

    /// Invariant: The container reports where the page stood, beside in a room that holds the pair and one column in a room too narrow, and the tile is drawn.
    /// Regression: The frame told the wrong layout so the sidebar acts on it, or a tile never drawn.
    /// Owner: kit
    #[test]
    fn the_container_reports_where_the_page_stood() {
        fn said(l: Laid) -> P {
            P(match l {
                Laid::Placed(Placement::Beside) => 1,
                Laid::Placed(Placement::OneColumn) => 2,
                Laid::Placed(Placement::Stacked) => 3,
                _ => 0,
            })
        }
        let mut c = canvas(160);
        for (room, told) in [(1920.0_f32, 1), (700.0, 2)] {
            let mut h = 0.0;
            let out = frame(&mut c, &at_rest(None), |ui| {
                let m = panes(Tile::Card, false, Narrow::OneColumn);
                let mut parts = Parts::none();
                parts.on_placed = Some((
                    Anchor {
                        scope: "panes",
                        n: 0,
                    },
                    said,
                ));
                h = draw_with(&m, ui, (0.0, room), 8.0, parts, |_, _| 120.0);
            });
            assert!(h >= 120.0, "{room}");
            let laid: Vec<u8> = out.laid.iter().map(|(_, _, p)| p.0).collect();
            assert_eq!(laid, vec![told], "{room}");
        }
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

    /// Invariant: A turn begun at the frame's time is at its start, half way through at half its time and over after it, and a person who reduces motion sees none.
    /// Regression: A turn that never ends, or one that runs under Reduce motion.
    /// Owner: kit
    #[test]
    fn a_page_turn_animates_by_the_frames_time() {
        let mut c = canvas(160);
        let mut seen = Vec::new();
        for now in [1000_u64, 1125, 1300] {
            frame_in(
                &mut c,
                &at_rest(None),
                (&mut MotionStore::new(), now),
                |ui| {
                    seen.push(turn_at(ui, (Motion::Smooth, 250), 1000));
                },
            );
        }
        assert_eq!(seen[0], Some((Turn::Through, 0.0)));
        assert_eq!(seen[1], Some((Turn::Through, 0.5)));
        assert_eq!(seen[2], None);
    }
}
