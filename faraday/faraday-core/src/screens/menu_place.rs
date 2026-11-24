//! Where Tools and Check stand on the wide layout (`docs/WORKSPACE.md` §4,
//! §5): stacked first, the page's one tile under This session's card where
//! it all fits, else the pair, the pane in the page's own mode at the left
//! and the tile at its right, filling the room in as many columns as fit
//! (one column: where a flow's pair stands). The place
//! is the page's and the room's, never what is loaded. Arithmetic and
//! drawing of one tile of the page's sections; the rows are the pane's.
//! A typed search's results take the tile's place the same way, on Home
//! (the pane then holds Home's rows) and on Tools.

use super::catalog::Bar;
use super::menu_page::{Page, Section, grid, page_span, sections_height};
use super::session_pane::{Mode, draw_page_pane, page_content_h, pane_groups, stacked_card};
use crate::drawing::View;
use crate::ui::pal::*;
use crate::ui::{Region, Ui};
use faraday_draw::tokens::{
    ELEVATION_FAB, LIST_PANE, MARGIN_WIDE, MENU_TILE_WIDE_MAX, PAD, PANE_TOP_BAR, READING_MENU_W,
    SHAPE_M, SPACER,
};

/// The area Home has in a room `room` wide: how far from the room's left
/// edge and how wide.
pub(super) fn home_area(room: f32) -> (f32, f32) {
    if room > READING_MENU_W {
        ((room - READING_MENU_W) / 2.0, READING_MENU_W)
    } else {
        (0.0, room)
    }
}

/// Where Home's card stands in a room `room` wide: its left edge from the
/// room's and its width.
fn home_span(room: f32) -> (f32, f32) {
    let (ox, area) = home_area(room);
    page_span(false, ox, area)
}

/// The tile's surface: raised, as the cards beside it.
fn tile_surface(ui: &mut Ui, (x, y, w, h): (f32, f32, f32, f32)) {
    ui.shadow(x, y, w, h, SHAPE_M, ELEVATION_FAB);
    ui.fill(x, y, w, h, SHAPE_M, Role::SurfaceContainerLow);
}

/// What fills the tile: draws from `(x, y)`, `w` wide, and returns its
/// height.
type Body<'a> = &'a dyn Fn(&mut Ui, (f32, f32), f32) -> f32;

/// `body` as the tile's content: a padding in from the tile's edge.
/// Returns the tile's height.
fn tile_content(ui: &mut Ui, (x, y, w): (f32, f32, f32), body: Body) -> f32 {
    body(ui, (x + PAD, y + PAD), w - 2.0 * PAD) + 2.0 * PAD
}

/// Where the pair's two panes stand: left edge and width of each.
struct Placed {
    pane_x: f32,
    pane_w: f32,
    card_x: f32,
    card_w: f32,
}

/// The pair where the room holds the tile in two columns or more: the pane
/// at the room's left page margin, one gap, the tile to the right page
/// margin and no wider than `MENU_TILE_WIDE_MAX`, the pair centred where
/// the tile stops. `None` where the room holds one column (the pair is
/// then a flow's). Arithmetic on the room alone.
fn wide_pair(room: f32) -> Option<Placed> {
    let tile = (room - 2.0 * MARGIN_WIDE - LIST_PANE - SPACER).min(MENU_TILE_WIDE_MAX);
    if grid(tile - 2.0 * PAD, usize::MAX).columns() < 2 {
        return None;
    }
    let unit = LIST_PANE + SPACER + tile;
    let pane_x = ((room - unit) / 2.0).max(MARGIN_WIDE);
    Some(Placed {
        card_x: pane_x + LIST_PANE + SPACER,
        card_w: tile,
        pane_x,
        pane_w: LIST_PANE,
    })
}

/// Draws the page placed, in the area `cw` wide from `x0`, `h` tall: the
/// stack where it fits, else the pair. Returns false, drawing nothing,
/// where the room holds neither (the page draws as it did).
pub(super) fn draw(
    app: &View,
    ui: &mut Ui,
    area: (f32, f32, f32),
    (page, sections): (&Page, &[Section]),
    mode: Mode,
) -> bool {
    let body =
        |ui: &mut Ui, at: (f32, f32), w: f32| sections_height(app, ui, at, w, page, sections);
    place(app, ui, area, &body, Some(mode))
}

/// Draws the results of the search typed in `bar` placed the same way, the
/// pane being `mode`'s, or Home's rows for `None`. Returns false, drawing
/// nothing, where the room holds neither.
pub(super) fn draw_results(
    app: &View,
    ui: &mut Ui,
    area: (f32, f32, f32),
    bar: Bar,
    mode: Option<Mode>,
) -> bool {
    let body =
        |ui: &mut Ui, at: (f32, f32), w: f32| super::catalog::home_results(app, ui, at, w, bar);
    place(app, ui, area, &body, mode)
}

/// The stack where `body`'s tile fits under the card, else the pair.
fn place(
    app: &View,
    ui: &mut Ui,
    (x0, cw, h): (f32, f32, f32),
    body: Body,
    mode: Option<Mode>,
) -> bool {
    let top = PANE_TOP_BAR;
    let (hx, hw) = home_span(cw);
    let card_h = crate::menu::measure(ui, &mut |ui| stacked_card(app, ui, (0.0, 0.0), hw));
    let tile_h = crate::menu::measure(ui, &mut |ui| tile_content(ui, (0.0, 0.0, hw), body));
    let under = top + card_h + SPACER;
    if under + tile_h + MARGIN_WIDE <= h {
        let clip = ui.rect(x0, 0.0, cw, h);
        let open = ui.region_open(Region::page(clip).from_zero(app.list_offset));
        stacked_card(app, ui, (x0 + hx, top), hw);
        tile_surface(ui, (x0 + hx, under, hw, tile_h));
        tile_content(ui, (x0 + hx, under, hw), body);
        let content = under + tile_h + MARGIN_WIDE;
        app.scroll.content_h().set(content);
        ui.region_close(open, 0.0);
        return true;
    }
    let p = match wide_pair(cw) {
        Some(p) => p,
        None => match crate::flow::pair::pair(cw, false) {
            Some(p) => Placed {
                pane_x: p.pane_x,
                pane_w: p.pane_w,
                card_x: p.card_x,
                card_w: p.card_w,
            },
            None => return false,
        },
    };
    let groups = pane_groups(app, mode);
    let pane_need = page_content_h(ui, p.pane_w, &groups);
    let tile_need =
        crate::menu::measure(ui, &mut |ui| tile_content(ui, (0.0, 0.0, p.card_w), body));
    let room_h = (h - top - MARGIN_WIDE).max(0.0);
    let both = pane_need.max(tile_need).min(room_h);
    draw_page_pane(
        ui,
        (x0 + p.pane_x, top, p.pane_w, both),
        app.scroll.home_box_offset(),
        &groups,
    );
    let tx = x0 + p.card_x;
    tile_surface(ui, (tx, top, p.card_w, both));
    let clip = ui.rect(tx, top, p.card_w, both);
    let open = ui.region_open(Region::page(clip).from_zero(app.list_offset));
    tile_content(ui, (tx, top, p.card_w), body);
    app.scroll.content_h().set(tile_need);
    ui.region_close(open, (tile_need - both).max(0.0));
    true
}
