//! A menu page drawn placed on the wide layout (`docs/HUB-LAYOUT.md` §2):
//! stacked first, the page's one tile under This session's card where it
//! all fits, else the pair, the session pane in the page's own mode at the
//! left and the tile at its right. The place is the page's and the room's,
//! never what is loaded. Draws one tile of the page's sections; the rows
//! are the pane's. A typed search's results take the tile's place the same
//! way, on Home (the pane then holds Home's rows) and on Tools.

use super::{Left, Panes, home_area, level};
use crate::drawing::View;
use crate::screens::catalog::{Bar, home_results};
use crate::screens::menu_page::{Page, Section, page_span, sections_height};
use crate::screens::session_pane::{draw_page_pane, page_content_h, pane_groups, stacked_card};
use crate::ui::pal::*;
use crate::ui::{Region, Ui};
use faraday_draw::tokens::{ELEVATION_FAB, MARGIN_WIDE, PAD, PANE_TOP_BAR, SHAPE_M};

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

/// Draws the results of the search typed in `bar` placed the same way.
/// Returns false, drawing nothing, where the room holds neither.
pub(crate) fn draw_results(
    app: &View,
    ui: &mut Ui,
    area: (f32, f32, f32),
    bar: Bar,
    panes: Panes,
) -> bool {
    let body = |ui: &mut Ui, at: (f32, f32), w: f32| home_results(app, ui, at, w, bar);
    placed(app, ui, area, &body, panes)
}

/// The stack where `body`'s tile fits under the card, else the pair. The
/// pair is measured only where the stack does not fit.
fn placed(app: &View, ui: &mut Ui, (x0, cw, h): (f32, f32, f32), body: Body, panes: Panes) -> bool {
    let Left::Session(mode) = panes.left else {
        return false;
    };
    let top = PANE_TOP_BAR;
    let (hx, hw) = home_span(cw);
    let card_h = crate::menu::measure(ui, &mut |ui| stacked_card(app, ui, (0.0, 0.0), hw));
    let tile_h = crate::menu::measure(ui, &mut |ui| tile_content(ui, (0.0, 0.0, hw), body));
    if let Some(under) = panes.stack(top, card_h, tile_h, h) {
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
    // The room is the one Guide's pane leaves.
    let Some(p) = panes.place(cw, false) else {
        return false;
    };
    app.stood_apart();
    let groups = pane_groups(app, mode);
    let pane_need = page_content_h(ui, p.pane_w, &groups);
    let tile_need =
        crate::menu::measure(ui, &mut |ui| tile_content(ui, (0.0, 0.0, p.card_w), body));
    let both = level(pane_need, tile_need, (h - top - MARGIN_WIDE).max(0.0));
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
