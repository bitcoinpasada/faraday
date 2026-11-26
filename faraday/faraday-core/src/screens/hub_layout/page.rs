//! A menu page drawn placed on the wide layout (`docs/HUB-LAYOUT.md` §2):
//! stacked first, the page's one tile under This session's card where it
//! all fits, else the pair, the session pane in the page's own mode at the
//! left and the tile at its right. The place is the page's and the room's,
//! never what is loaded. Draws one tile of the page's sections; the rows
//! are the pane's. A typed search's results take the tile's place the same
//! way, on Home (the pane then holds Home's rows) and on Tools.

use super::pair::left_of_guide;
use super::{Left, Panes, Place, guide_room, home_area, level};
use crate::drawing::View;
use crate::screens::catalog::home_results;
use crate::screens::menu_page::{Page, Section, page_span, sections_height};
use crate::screens::session_pane::{
    draw_page_pane, page_content_h, pane_groups, stacked_card, stacked_card_h, stacked_scan,
};
use crate::ui::pal::*;
use crate::ui::{Region, Ui};
use faraday_draw::tokens::{ELEVATION_SHEET, MARGIN_WIDE, PAD, PANE_TOP_BAR, SHAPE_M, SPACER};

/// Where Home's card stands in a room `room` wide: its left edge from the
/// room's and its width.
fn home_span(room: f32) -> (f32, f32) {
    let (ox, area) = home_area(room);
    page_span(false, ox, area)
}

/// The tile's surface: raised, as the cards beside it.
pub(crate) fn tile_surface(ui: &mut Ui, (x, y, w, h): (f32, f32, f32, f32)) {
    // Everything raised on a menu page rests at level 1 (F331); a High
    // contrast theme casts no shadow, so an outline stands in its place.
    ui.shadow(x, y, w, h, SHAPE_M, ELEVATION_SHEET);
    ui.fill(x, y, w, h, SHAPE_M, Role::SurfaceContainerLow);
    if ui.theme.is_high_contrast() {
        ui.stroke(x, y, w, h, SHAPE_M, Role::Outline);
    }
}

/// What fills the tile: draws from `(x, y)`, `w` wide, and returns its
/// height.
type Body<'a> = &'a dyn Fn(&mut Ui, (f32, f32), f32) -> f32;

/// `body` as the tile's content: a padding in from the tile's edge.
/// Returns the tile's height.
fn tile_content(ui: &mut Ui, (x, y, w): (f32, f32, f32), body: Body) -> f32 {
    body(ui, (x + PAD, y + PAD), w - 2.0 * PAD) + 2.0 * PAD
}

/// Draws the page placed as `panes` declares, in the area `cw` wide from
/// `x0`, `h` tall: the stack where it fits, else the pair. Returns false,
/// drawing nothing, where the room holds neither (the page draws its one
/// column).
pub(crate) fn draw(
    app: &View,
    ui: &mut Ui,
    area: (f32, f32, f32),
    (page, sections): (&Page, &[Section]),
    panes: Panes,
) -> bool {
    let body =
        |ui: &mut Ui, at: (f32, f32), w: f32| sections_height(app, ui, at, w, page, sections);
    placed(app, ui, area, &body, panes)
}

/// Draws the results of the search typed placed the same way.
/// Returns false, drawing nothing, where the room holds neither.
pub(crate) fn draw_results(app: &View, ui: &mut Ui, area: (f32, f32, f32), panes: Panes) -> bool {
    let body = |ui: &mut Ui, at: (f32, f32), w: f32| home_results(app, ui, at, w);
    placed(app, ui, area, &body, panes)
}

/// Draws the results of the search typed alone, in the place of a
/// flow's card (`docs/WORKSPACE.md` §4): no This session beside them. The
/// flow underneath is not drawn, so a secret it shows is gone at once.
pub(crate) fn draw_results_alone(app: &View, ui: &mut Ui, (x0, cw, h): (f32, f32, f32)) {
    let body = |ui: &mut Ui, at: (f32, f32), w: f32| home_results(app, ui, at, w);
    // The one card is the pair's outline: Your choices and the step.
    // A flow's unit has the whole room and makes the pane's room here; any
    // other flow's page is already beside the pane.
    let guide = app.unit_on_show() && app.guide_pane_shown();
    let (cx, w) = super::pair::pair(cw, guide).map_or_else(
        || super::detail_alone(cw),
        |p| {
            let left = p.pane_x - PAD;
            (left, p.card_x + p.card_w + PAD - left)
        },
    );
    let top = PANE_TOP_BAR;
    let tile_h = crate::menu::measure(ui, &mut |ui| tile_content(ui, (0.0, 0.0, w), &body));
    let clip = ui.rect(x0, 0.0, cw, h);
    let open = ui.region_open(Region::page(clip).from_zero(app.list_offset));
    tile_surface(ui, (x0 + cx, top, w, tile_h));
    tile_content(ui, (x0 + cx, top, w), &body);
    let content = top + tile_h + MARGIN_WIDE;
    app.scroll.content_h().set(content);
    ui.region_close(open, content - h);
}

/// Draws the open row's details placed the same way, on Home: under
/// This session's card where they fit, else beside it. Returns false,
/// drawing nothing, where the room holds neither.
pub(crate) fn draw_details(app: &View, ui: &mut Ui, area: (f32, f32, f32), panes: Panes) -> bool {
    let body =
        |ui: &mut Ui, _: (f32, f32), w: f32| crate::screens::row_details::content_h(app, ui, w);
    placed(app, ui, area, &body, panes)
}

/// The pair's place in an area `cw` wide that stands `ox` into the room
/// beside the navigation, from the area's left edge. Beside Guide's pane
/// (`guide`) an area narrower than the room (Home's) keeps the place it
/// has without the guide and moves left only as far as the pane needs;
/// where it cannot, the pair is placed beside the pane in the whole room
/// (`docs/WORKSPACE.md` §7.4). Arithmetic.
fn place_in_area(app: &View, panes: &Panes, (ox, cw): (f32, f32), guide: bool) -> Option<Place> {
    let room = app.window().width - crate::screens::SIDEBAR_W;
    if !guide || cw >= room {
        return panes.place(cw, guide);
    }
    let into = |p: Place, by: f32| Place {
        card_x: p.card_x + by,
        pane_x: p.pane_x + by,
        ..p
    };
    panes
        .place(cw, false)
        .and_then(|p| left_of_guide(into(p, ox), room))
        .or_else(|| panes.place(room, true))
        .map(|p| into(p, -ox))
}

/// The stack where `body`'s tile fits under the card, else the pair. The
/// pair is measured only where the stack does not fit.
fn placed(app: &View, ui: &mut Ui, (x0, cw, h): (f32, f32, f32), body: Body, panes: Panes) -> bool {
    let Left::Session(mode) = panes.left else {
        return false;
    };
    let top = PANE_TOP_BAR;
    // A page stacked beside Guide's pane keeps to the room the pane
    // leaves; a pair is placed in the whole room.
    let guide = app.guide_beside_pair();
    let left_of_guide = if guide { cw - guide_room() } else { cw };
    let (hx, hw) = home_span(left_of_guide);
    let card_h = stacked_card_h(app, ui, hw);
    let tile_h = crate::menu::measure(ui, &mut |ui| tile_content(ui, (0.0, 0.0, hw), body));
    if let Some(under) = panes.stack(top, card_h, tile_h, h) {
        // Details scroll in their own area, under a card that stands
        // still; a region left open here would reach the top bar.
        if crate::row_open::opened(app).is_some() {
            stacked_card(app, ui, (x0 + hx, top), hw);
            let rest = (h - MARGIN_WIDE - under).max(tile_h);
            return crate::screens::row_details::draw(app, ui, (x0 + hx, under, hw, rest));
        }
        let clip = ui.rect(x0, 0.0, left_of_guide, h);
        let open = ui.region_open(Region::page(clip).from_zero(app.list_offset));
        // The sidebar's Scan: the strip grows to the viewfinder's box, as
        // wide as the tile's columns, and the tile moves down under it.
        // The stack was chosen by This session's own height.
        let columns = (x0 + hx + PAD, top, hw - 2.0 * PAD);
        let under = match stacked_scan(app, ui, columns, h - top - MARGIN_WIDE) {
            Some(box_h) => top + box_h + SPACER,
            None => {
                stacked_card(app, ui, (x0 + hx, top), hw);
                under
            }
        };
        tile_surface(ui, (x0 + hx, under, hw, tile_h));
        tile_content(ui, (x0 + hx, under, hw), body);
        let content = under + tile_h + MARGIN_WIDE;
        app.scroll.content_h().set(content);
        ui.region_close(open, (content - h).max(0.0));
        return true;
    }
    let Some(p) = place_in_area(app, &panes, (ui.ox, cw), guide) else {
        return false;
    };
    app.stood_apart();
    let groups = pane_groups(app, mode);
    let tx = x0 + p.card_x;
    // A row's details stand in the tile's place, both panes the window's
    // height; their page is not measured again here.
    let room = (h - top - MARGIN_WIDE).max(0.0);
    if crate::row_open::opened(app).is_some() {
        draw_page_pane(
            app,
            ui,
            (x0 + p.pane_x, top, p.pane_w, room),
            (app.scroll.home_box_offset(), app.scroll.home_band_offset()),
            &groups,
        );
        crate::screens::row_details::draw(app, ui, (tx, top, p.card_w, room));
        return true;
    }
    let pane_need = page_content_h(ui, p.pane_w, &groups);
    let tile_need =
        crate::menu::measure(ui, &mut |ui| tile_content(ui, (0.0, 0.0, p.card_w), body));
    let both = level(pane_need, tile_need, room);
    draw_page_pane(
        app,
        ui,
        (x0 + p.pane_x, top, p.pane_w, both),
        (app.scroll.home_box_offset(), app.scroll.home_band_offset()),
        &groups,
    );
    tile_surface(ui, (tx, top, p.card_w, both));
    let clip = ui.rect(tx, top, p.card_w, both);
    let open = ui.region_open(Region::page(clip).from_zero(app.list_offset));
    tile_content(ui, (tx, top, p.card_w), body);
    app.scroll.content_h().set(tile_need);
    ui.region_close(open, (tile_need - both).max(0.0));
    true
}
