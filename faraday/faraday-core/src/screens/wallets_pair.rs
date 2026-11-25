//! Wallets as the pair on the wide layout (`docs/WORKSPACE.md` §2, §4): the
//! session pane in Wallets' mode at the left, and at its right either the
//! tile of the wallets and the four ways in (Wallets' start page) or the open wallet's page
//! (`Screen::Wallets`), in the place of the tile. Wallets never stacks.
//! The pair stands at the flow pair's place.

use super::hub_layout::{Left, Narrow, Panes, Place, Tile};
use super::session_pane::{Mode, draw_page_pane, page_groups};
use super::wallets::wallet_entries;
use crate::drawing::View;
use crate::strings::EN;
use crate::ui::pal::*;
use crate::ui::{Shape, Ui};
use crate::{Action, Screen};
use faraday_draw::tokens::{
    BODY_L, ELEVATION_FAB, GAP, GAP_SMALL, ICON_SMALL, LIST_1, LIST_PAD, MARGIN_WIDE, PAD,
    PANE_TOP_BAR, SHAPE_L, SHAPE_M, TITLE_M,
};
use faraday_draw::widgets::Icon;

/// Wallets' panes: the session pane in Wallets' mode and the tile, at the
/// flow pair's place. Wallets never stacks and has no pair in a narrow room.
const PANES: Panes = Panes {
    left: Left::Session(Some(Mode::Wallets)),
    tile: Tile::Card,
    stacks: false,
    narrow: Narrow::OneColumn,
};

/// The pair's place in an area `cw` wide, none where the room holds less.
pub(super) fn place(cw: f32) -> Option<Place> {
    PANES.place(cw, false)
}

/// The pane at the pair's left, `top` down, `h` tall.
pub(super) fn pane(app: &View, ui: &mut Ui, x0: f32, p: &Place, (top, h): (f32, f32)) {
    let groups = page_groups(app, Mode::Wallets);
    draw_page_pane(
        ui,
        (x0 + p.pane_x, top, p.pane_w, h),
        (app.scroll.home_box_offset(), app.scroll.home_band_offset()),
        &groups,
    );
}

/// A row's details as the pair: the pane with the open row marked, and the
/// details in the tile's place, at the pair's place. False, drawing
/// nothing, where no row is open or the room holds no pair.
pub(super) fn draw_details(app: &View, ui: &mut Ui, (x0, cw, h): (f32, f32, f32)) -> bool {
    let Some(p) = place(cw) else {
        return false;
    };
    if crate::row_open::opened(app).is_none() {
        return false;
    }
    let (top, foot) = span(h);
    pane(app, ui, x0, &p, (top, (foot - top).max(0.0)));
    super::row_details::draw(
        app,
        ui,
        (x0 + p.card_x, top, p.card_w, (foot - top).max(0.0)),
    )
}

/// The tile's rows: the wallets as Wallets lists them, each opening its
/// page. Returns their height, whole.
fn rows_h(app: &View, ui: &mut Ui, w: f32) -> f32 {
    let entries = wallet_entries(app);
    if entries.is_empty() {
        return ui.line_as(BODY_L);
    }
    crate::menu::measure(ui, &mut |ui| {
        entries
            .iter()
            .map(|e| {
                crate::panes::screen::list_item(
                    ui,
                    (0.0, 0.0, w),
                    &e.item(&Action::OpenWallet),
                    false,
                )
            })
            .sum()
    })
}

/// The pair's top and foot in a window `h` tall: where the pane, the tile
/// and a wallet's page stand, one place for all three.
pub(super) fn span(h: f32) -> (f32, f32) {
    (PANE_TOP_BAR, h - MARGIN_WIDE)
}

/// The ways the tile offers, in Home's order (Make a new wallet, Restore an
/// existing wallet, Spend from a backup, Add a key), and how many columns
/// they stand in: where the pills stand in the bar the three they repeat
/// are left out and Add a key stands alone, else the four stand in two
/// columns and two rows (`docs/WORKSPACE.md` §2).
fn offered(pills: bool) -> (Vec<(Icon, &'static str, Action)>, usize) {
    let all = super::wallets_start::ways()
        .into_iter()
        .take(4)
        .map(|(icon, name, _, action)| match action {
            // The row has no line under it: the short name.
            Action::Nav(Screen::SpendBackup) => (icon, EN.catalog_spend_backup, action),
            _ => (icon, name, action),
        });
    if pills {
        (all.skip(3).collect(), 1)
    } else {
        (all.collect(), 2)
    }
}

/// The ways' height: their rows.
fn ways_height((n, cols): (usize, usize)) -> f32 {
    n.div_ceil(cols) as f32 * LIST_1
}

/// The tile's ways from `(x, y)`, `w` wide: neutral, each with its icon,
/// its name and a chevron, doing what the same words do elsewhere, in
/// `cols` columns, row by row.
fn way_rows(
    ui: &mut Ui,
    (x, y, w): (f32, f32, f32),
    (ways, cols): (Vec<(Icon, &'static str, Action)>, usize),
) {
    let cw = (w - (cols - 1) as f32 * GAP) / cols as f32;
    for (i, (icon, name, action)) in ways.into_iter().enumerate() {
        let (rx, ry) = (
            x + (i % cols) as f32 * (cw + GAP),
            y + (i / cols) as f32 * LIST_1,
        );
        ui.press(
            Shape::new(rx, ry, cw, LIST_1, SHAPE_L),
            Role::OnSurface.paint(),
            action,
        );
        ui.icon(
            rx + LIST_PAD,
            ry + (LIST_1 - ICON_SMALL) / 2.0,
            ICON_SMALL,
            icon,
            ICON_SMALL,
            Role::OnSurfaceVariant,
        );
        let line = ui.line_as(BODY_L);
        let tx = rx + LIST_PAD + ICON_SMALL + GAP;
        let room = cw - (tx - rx) - LIST_PAD - ICON_SMALL - GAP_SMALL;
        let name = ui.fit_as(BODY_L, name, room);
        ui.text_as(
            tx,
            ry + (LIST_1 - line) / 2.0,
            BODY_L,
            Role::OnSurface,
            &name,
        );
        ui.icon(
            rx + cw - LIST_PAD - ICON_SMALL,
            ry + (LIST_1 - ICON_SMALL) / 2.0,
            ICON_SMALL,
            Icon::ChevronRight,
            ICON_SMALL,
            Role::OnSurfaceVariant,
        );
    }
}

/// Wallets' start page as the pair in an area `cw` wide from `x0`, `h`
/// tall: the pane, and the tile: its heading, the wallets and the
/// ways in. The rows stand at the list's foot where the list fits and
/// pinned at the tile's foot where it scrolls. False, drawing nothing,
/// where the room holds no pair.
pub(super) fn draw_start(app: &View, ui: &mut Ui, (x0, cw, h): (f32, f32, f32)) -> bool {
    let Some(p) = place(cw) else {
        return false;
    };
    let (top, foot) = span(h);
    let inner = p.card_w - 2.0 * PAD;
    let head = ui.line_as(TITLE_M) + GAP;
    let body = rows_h(app, ui, inner);
    let ways = offered(super::bar_search::on(app.screen));
    let ways_h = ways_height((ways.0.len(), ways.1));
    let fit = 2.0 * PAD + head + body + GAP + ways_h;
    let room = (foot - top).max(0.0);
    // The tile fills the place, as a wallet's page does: the pane stands
    // at one place for both and Back moves nothing.
    let tile_h = room;
    pane(app, ui, x0, &p, (top, tile_h));
    let tx = x0 + p.card_x;
    ui.shadow(tx, top, p.card_w, tile_h, SHAPE_M, ELEVATION_FAB);
    ui.fill(
        tx,
        top,
        p.card_w,
        tile_h,
        SHAPE_M,
        Role::SurfaceContainerLow,
    );
    let (ix, iy) = (tx + PAD, top + PAD);
    ui.text_as(
        ix + LIST_PAD,
        iy,
        TITLE_M,
        Role::OnSurface,
        EN.start_wallets,
    );
    let ly = iy + head;
    // Where the list is cut, the rows are pinned at the tile's foot.
    let pinned = fit > tile_h;
    let view = if pinned {
        (tile_h - 2.0 * PAD - head - GAP - ways_h).max(0.0)
    } else {
        body
    };
    let ways_y = if pinned {
        top + tile_h - PAD - ways_h
    } else {
        ly + body + GAP
    };
    let entries = wallet_entries(app);
    if entries.is_empty() {
        ui.text_as(
            ix + LIST_PAD,
            ly,
            BODY_L,
            Role::OnSurfaceVariant,
            EN.wallets_no_wallet_loaded,
        );
    } else {
        let mut list = crate::panes::screen::ListPane::begin_moving(app, ui, (ix, ly, inner, view));
        for e in &entries {
            list.item_opens(ui, &e.item(&Action::OpenWallet));
        }
        list.end(ui);
    }
    way_rows(ui, (ix, ways_y, inner), ways);
    true
}
