//! Vaults as the pair on the wide layout (`docs/WORKSPACE.md` §2, §4): the
//! session pane in Vaults' mode at the left (this session's keys and
//! wallets), and at its right the tile of the vault files, ending in the row
//! Make a new vault. Vaults never stacks. The pair stands at the flow pair's
//! place, as Wallets' does (`screens::wallets_pair`).

use super::hub_layout::{Left, Narrow, Panes, Place, Tile};
use super::session_pane::{Mode, draw_page_pane, page_groups};
use super::wallets_pair::{span, way_rows, ways_height};
use crate::Action;
use crate::drawing::View;
use crate::panes::screen::ListPane;
use crate::strings::EN;
use crate::ui::Ui;
use crate::ui::pal::*;
use crate::vault::VaultAction as V;
use faraday_draw::tokens::{BODY_L, ELEVATION_SHEET, GAP, LIST_PAD, PAD, SHAPE_M, TITLE_M};
use faraday_draw::widgets::Icon;

/// Vaults' panes: the session pane in Vaults' mode and the tile, at the
/// flow pair's place. Vaults never stacks and has no pair in a narrow room.
const PANES: Panes = Panes {
    left: Left::Session(Some(Mode::Vaults)),
    tile: Tile::Card,
    stacks: false,
    narrow: Narrow::OneColumn,
};

/// The pair's place in an area `cw` wide, none where the room holds less.
fn place(cw: f32, guide: bool) -> Option<Place> {
    PANES.place(cw, guide)
}

/// The tile's vault cards from the list's top, `w` wide: height of all.
fn cards_h(app: &View, ui: &mut Ui, w: f32) -> f32 {
    let files = app.vault_files();
    if files.is_empty() {
        return ui.line_as(BODY_L);
    }
    crate::menu::measure(ui, &mut |ui| {
        files
            .iter()
            .enumerate()
            .map(|(i, f)| crate::vault::panes_screen::card(app, ui, (0.0, 0.0, w), i, f) + GAP)
            .sum()
    })
}

/// Vaults as the pair in an area `cw` wide from `x0`, `h` tall: the pane,
/// and the tile: its heading, the vault files and the row Make a new vault,
/// at the list's foot where the list fits and pinned at the tile's foot
/// where it scrolls. False, drawing nothing, where the room holds no pair.
pub(super) fn draw(app: &View, ui: &mut Ui, (x0, cw, h): (f32, f32, f32)) -> bool {
    let Some(p) = place(cw, app.guide_beside_pair()) else {
        return false;
    };
    if ui.compact {
        return false;
    }
    let back = crate::vault::screen::vaults_back(app);
    super::bar_head(ui, back, EN.vault_vaults);
    let (top, foot) = span(h);
    let tile_h = (foot - top).max(0.0);
    let groups = page_groups(app, Mode::Vaults);
    draw_page_pane(
        ui,
        (x0 + p.pane_x, top, p.pane_w, tile_h),
        (app.scroll.home_box_offset(), app.scroll.home_band_offset()),
        &groups,
    );
    let inner = p.card_w - 2.0 * PAD;
    let head = ui.line_as(TITLE_M) + GAP;
    let body = cards_h(app, ui, inner);
    // Made from another flow, no second vault is started in its place (that
    // would drop the way back).
    let make = app
        .vaults
        .back_to
        .is_none()
        .then(|| vec![(Icon::Lock, EN.vaults_make_a_vault, Action::Vault(V::Create))]);
    let ways = make.map_or((Vec::new(), 1), |m| (m, 1));
    let ways_h = ways_height((ways.0.len(), ways.1));
    let below = if ways_h > 0.0 { GAP + ways_h } else { 0.0 };
    let fit = 2.0 * PAD + head + body + below;
    let tx = x0 + p.card_x;
    ui.shadow(tx, top, p.card_w, tile_h, SHAPE_M, ELEVATION_SHEET);
    ui.fill(
        tx,
        top,
        p.card_w,
        tile_h,
        SHAPE_M,
        Role::SurfaceContainerLow,
    );
    if ui.theme.is_high_contrast() {
        ui.stroke(tx, top, p.card_w, tile_h, SHAPE_M, Role::Outline);
    }
    let (ix, iy) = (tx + PAD, top + PAD);
    ui.text_as(ix + LIST_PAD, iy, TITLE_M, Role::OnSurface, EN.vault_vaults);
    let ly = iy + head;
    let pinned = fit > tile_h;
    let view = if pinned {
        (tile_h - 2.0 * PAD - head - below).max(0.0)
    } else {
        body
    };
    let ways_y = if pinned {
        top + tile_h - PAD - ways_h
    } else {
        ly + body + GAP
    };
    let files = app.vault_files();
    if files.is_empty() {
        ui.text_as(
            ix + LIST_PAD,
            ly,
            BODY_L,
            Role::OnSurfaceVariant,
            EN.vault_no_vault_in_files,
        );
        // The sample vault goes in the "Demo only" card (`docs/DEMO.md` §14).
        if app.put_on().is_some() {
            crate::samples::put::offer(ui, Action::UseSample);
        }
    } else {
        let mut list = ListPane::begin_moving(app, ui, (ix, ly, inner, view));
        for (i, f) in files.iter().enumerate() {
            let (x, _, y) = list.at();
            let ch = crate::vault::panes_screen::card(app, ui, (x, y, inner), i, f);
            list.advance(ch + GAP);
        }
        list.end(ui);
    }
    if ways_h > 0.0 {
        way_rows(ui, (ix, ways_y, inner), ways);
    }
    true
}
