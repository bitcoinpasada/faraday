//! Where Tools and Check stand on the wide layout (`docs/WORKSPACE.md` §4,
//! §5): stacked first, the page's one tile under This session's card where
//! it all fits, else the pair, the pane in the page's own mode at the left
//! and the tile at its right, standing where a flow's pair stands. The place
//! is the page's and the room's, never what is loaded. Arithmetic and
//! drawing of one tile of the page's sections; the rows are the pane's.

use super::menu_page::{Page, Section, page_span, sections_height};
use super::session_pane::{Mode, draw_page_pane, page_content_h, page_groups, stacked_card};
use crate::drawing::View;
use crate::ui::pal::*;
use crate::ui::{Region, Ui};
use faraday_draw::tokens::{
    ELEVATION_FAB, MARGIN_WIDE, PAD, PANE_TOP_BAR, READING_MENU_W, SHAPE_M, SPACER,
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

/// The page's sections as the tile's content: a padding in from the tile's
/// edge, in the columns that fit. Returns the tile's height.
fn tile_content(
    app: &View,
    ui: &mut Ui,
    (x, y, w): (f32, f32, f32),
    (page, sections): (&Page, &[Section]),
) -> f32 {
    sections_height(app, ui, (x + PAD, y + PAD), w - 2.0 * PAD, page, sections) + 2.0 * PAD
}

/// Draws the page placed, in the area `cw` wide from `x0`, `h` tall: the
/// stack where it fits, else the pair. Returns false, drawing nothing,
/// where the room holds neither (the page draws as it did).
pub(super) fn draw(
    app: &View,
    ui: &mut Ui,
    (x0, cw, h): (f32, f32, f32),
    (page, sections): (&Page, &[Section]),
    mode: Mode,
) -> bool {
    let top = PANE_TOP_BAR;
    let (hx, hw) = home_span(cw);
    let card_h = crate::menu::measure(ui, &mut |ui| stacked_card(app, ui, (0.0, 0.0), hw));
    let tile_h = crate::menu::measure(ui, &mut |ui| {
        tile_content(app, ui, (0.0, 0.0, hw), (page, sections))
    });
    let under = top + card_h + SPACER;
    if under + tile_h + MARGIN_WIDE <= h {
        let clip = ui.rect(x0, 0.0, cw, h);
        let open = ui.region_open(Region::page(clip).from_zero(app.list_offset));
        stacked_card(app, ui, (x0 + hx, top), hw);
        tile_surface(ui, (x0 + hx, under, hw, tile_h));
        tile_content(app, ui, (x0 + hx, under, hw), (page, sections));
        let content = under + tile_h + MARGIN_WIDE;
        app.scroll.content_h().set(content);
        ui.region_close(open, 0.0);
        return true;
    }
    let Some(p) = crate::flow::pair::pair(cw, false) else {
        return false;
    };
    let groups = page_groups(app, mode);
    let pane_need = page_content_h(ui, p.pane_w, &groups);
    let tile_need = crate::menu::measure(ui, &mut |ui| {
        tile_content(app, ui, (0.0, 0.0, p.card_w), (page, sections))
    });
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
    tile_content(app, ui, (tx, top, p.card_w), (page, sections));
    app.scroll.content_h().set(tile_need);
    ui.region_close(open, (tile_need - both).max(0.0));
    true
}
