//! Every screen, drawn from the app's state each frame: a 240-unit
//! sidebar and the screen beside it. This file is the frame around
//! every screen, the dispatch from `Screen` to the file that draws it
//! (`lib.rs`'s code map lists them), and what the frame itself draws. The
//! sidebar, the sheets and the parts several screens share are
//! `sidebar.rs`, `sheets.rs` and `parts.rs`; each other file here draws
//! one screen.

mod backup;
mod catalog;
mod check_message;
mod create;
mod decode;
mod entry;
mod files;
mod home;
mod learn_pane;
mod message;
mod nonce_check;
mod parts;
pub(crate) mod randomness;
mod restore;
mod set_time;
mod settings;
mod sheets;
mod sidebar;
mod spend;
mod visit;
mod wallets;
mod wallets_start;

use crate::drawing::View;
use crate::nav::Nav;
use crate::strings::{EN, fill};
use crate::ui::pal::*;
use crate::ui::{Style, Ui};
use crate::{Action, Screen, Sheet};
pub(crate) use backup::{SOFTWARE_ROWS, extra_copies, held_item, item_title, item_title_now};
pub(crate) use create::CSTEPS;
pub(crate) use decode::{decode_back, decode_back_to};
pub(crate) use entry::{
    codex32_check_button, codex32_found, codex32_head, codex32_mark, codex32_tick, entry_form,
    entry_scan, entry_stick, entry_typed, entry_words, keyed_status, word_count_row,
};
use faraday_draw::tokens::{APP_BAR_H, BODY_S, BUTTON_H, TITLE_M};
use faraday_draw::tokens::{
    COLUMN_MAX, MARGIN_WIDE, NAV_BAR_H, PAGE_AREA_UNTIL_W, RAIL_W, READING_EXPLORE_W,
    READING_PAGE_W, READING_RANDOM_W, READING_TIME_W, SPACER, SUPPORT_PANE, SUPPORT_PANE_TOP,
    TOP_ROW_QUESTION, TOUCH,
};
use faraday_draw::widgets::Icon;
pub use faraday_ui::parts::qr_code;
pub(crate) use faraday_ui::parts::{button_rows, section_label, sheet_box, wrap_buttons};
pub(crate) use files::{stick_chip as files_stick_chip, vault_panel};
pub(crate) use home::{home_lead_beside_tiles, tile_grid};
pub(crate) use message::{MESSAGE_STEPS, MSTEPS};
pub(crate) use parts::{
    CopyGrid, address_press, buttons_and_next, code_field, copy_blocks, copy_grid, descriptor_row,
    first_addresses, grid_never, guide_text, guide_text_learn, keyed_pills, kind_line, kind_name,
    locked_tag, never_below, never_real_seed, never_real_seed_if, never_under, next_button,
    next_button_when, pass_field, pin_button, row_press, side_panel_w, sign_readiness, size_text,
    stepper, ticked, word_keys, working_bar, working_line,
};
pub(crate) use parts::{camera_row, card_pad, coin_pad, dice_pad, key_path_line, word_cell_edge};
pub(crate) use restore::RSTEPS;
pub(crate) use settings::display_section as settings_display_section;
pub(crate) use settings::open_head as settings_open;
use sheets::{
    devices_sheet, idle_warn_sheet, key_path_sheet, learn_sheet, lock_ask_sheet, lock_sheet,
    locked_sheet, new_input_sheet, potential_sheet, power_sheet, pull_sheet, qr_sheet, scan_sheet,
    secret_out_sheet, write_out_sheet,
};
use sidebar::sidebar;
pub(crate) use sidebar::{lock_held_pill, name_suffix};
pub(crate) use spend::{STEPS as SPEND_TITLES, signers_count, step_body, step_summary};
pub(crate) use wallets::{
    backup_line, loose_card, missing_keys_line, silent_address_lines, slot_marks,
};

const SIDEBAR_W: f32 = 240.0;

/// How a page takes the room beside the navigation on a wide window
/// (`docs/LAYOUT.md` §14, §15).
enum Area {
    /// Reading content only: an area this wide, centred in the room.
    Reading(f32),
    /// Wide content: the whole room, from the navigation to the
    /// window's edge.
    Work,
    /// No wide layout built yet: the area the old frame gave it,
    /// centred, until its batch changes this line (`docs/LAYOUT.md` §19
    /// lists them).
    Until,
}

/// How `screen` takes the room.
fn area_of(screen: Screen) -> Area {
    match screen {
        // Back up and a wallet's backup flow hold wide content, the
        // backup board on their plan's card: they start at the navigation
        // whether the board is open or not, so the page stands still when
        // it opens (`docs/LAYOUT.md` §14, §17).
        Screen::Backup | Screen::BackupFlow => Area::Work,
        Screen::SetTime => Area::Reading(READING_TIME_W),
        Screen::Randomness => Area::Reading(READING_RANDOM_W),
        // Explore is reading content: its fields, values and the
        // addresses in the column (`docs/LAYOUT.md` §17).
        Screen::Explore => Area::Reading(READING_EXPLORE_W),
        Screen::Spend
        | Screen::Message
        | Screen::CheckMessage
        | Screen::Create
        | Screen::Restore
        | Screen::CreateVault
        | Screen::Entry
        | Screen::Bip85
        | Screen::Silent
        | Screen::Lightning
        | Screen::Vanity
        | Screen::Decode
        | Screen::NonceCheck
        | Screen::Family => Area::Reading(READING_PAGE_W),
        // Tools, Transfer and a stick visit are reading content
        // (fields, rows, two lists side by side), each in its own area;
        // the tools' catalogue is a grid of tiles, wide content
        // (`docs/LAYOUT.md` §14, §17).
        Screen::Tools => Area::Reading(faraday_draw::tokens::READING_TOOLS_W),
        Screen::Transfer => Area::Reading(faraday_draw::tokens::READING_TRANSFER_W),
        Screen::Visit => Area::Reading(faraday_draw::tokens::READING_VISIT_W),
        Screen::Catalog => Area::Work,
        // New key, which holds Make codex32 shares and its supporting
        // pane; Home, whose tiles are a feed across the work width; and
        // the lists, a list pane and the one chosen beside it to the
        // window's edge (`docs/LAYOUT.md` §5, §17).
        // Settings' cards, two columns of them where they fit
        // (`docs/LAYOUT.md` §17), centred in the room.
        Screen::KeyGen | Screen::Home | Screen::Settings => Area::Work,
        s if crate::panes::has_list(s) => Area::Work,
        // Wallets' first page and the codes are reading content; an open
        // vault's contents are list and detail, its kinds, items and the
        // item, to the window's edge (`docs/LAYOUT.md` §5, §17).
        Screen::WalletsStart | Screen::Codes => Area::Reading(READING_PAGE_W),
        Screen::VaultContents => Area::Work,
        _ => Area::Until,
    }
}

/// The page area is the window less the navigation at every size, with
/// no cap on the frame (`docs/LAYOUT.md` §15): what stands at the right
/// stands against the window's edge. Within a room wider than the page
/// needs (an extra-large window), a reading page is laid out in its own
/// area, centred; a page with wide content starts at the navigation and
/// runs to the edge; a page with no wide layout yet keeps the area the
/// old frame gave it. Returns how far the page is moved right and its
/// width. Arithmetic.
fn page_area(screen: Screen, room: f32) -> (f32, f32) {
    if room <= PAGE_AREA_UNTIL_W {
        return (0.0, room);
    }
    match area_of(screen) {
        Area::Reading(w) => ((room - w) / 2.0, w),
        Area::Work => (0.0, room),
        Area::Until => ((room - PAGE_AREA_UNTIL_W) / 2.0, PAGE_AREA_UNTIL_W),
    }
}

/// Where a supporting pane stands (`docs/LAYOUT.md` §6, §15): the one
/// place for every pane. For the page area at `x0`, `cw` wide, in a
/// window `h` tall: the pane's left edge, its top, its width and the
/// height it shows. Its right edge is `MARGIN_WIDE` from the area's, which
/// is the window's right edge, and its top is below the top-right strip.
/// Arithmetic.
pub(crate) fn support_pane_place((x0, cw, h): (f32, f32, f32)) -> (f32, f32, f32, f32) {
    let w = SUPPORT_PANE;
    let top = SUPPORT_PANE_TOP;
    (
        x0 + cw - MARGIN_WIDE - w,
        top,
        w,
        (h - top - MARGIN_WIDE).max(TOUCH),
    )
}

/// The right end of the backup board's work width, in the page's x, where
/// the board is shown on a window whose room is wider than the old page
/// area (`docs/LAYOUT.md` §14, §17): the page then runs from the
/// navigation to the window's edge, the board's list stands in the
/// supporting pane and the places stand in columns up to the pane's
/// spacer. `None` elsewhere: there the board is laid out as before.
/// Costs a lookup.
pub(crate) fn board_work_end(app: &crate::Faraday) -> Option<f32> {
    backup_work_end(app).filter(|_| app.board_shown())
}

/// [`board_work_end`] whether the board is shown or not: on such a window
/// Back up and a wallet's backup flow keep their reading column where it
/// stands with the board open, so no card moves when the board's card
/// opens or closes (`docs/LAYOUT.md` §14). Costs a lookup.
pub(crate) fn backup_work_end(app: &crate::Faraday) -> Option<f32> {
    if app.is_compact() {
        return None;
    }
    let full_w = app.size().0;
    let nav = if app.held { 0.0 } else { SIDEBAR_W };
    let room = full_w - nav;
    (room > PAGE_AREA_UNTIL_W).then(|| nav + room - support_pane_room())
}

/// A wide page's head: its name and its way back (the name of where it
/// lands, and the press) handed to the top app bar the frame draws over
/// the page (`docs/LAYOUT.md` §3, DESIGN §4.1). Returns where the page's
/// first line stands under the bar. A lookup.
pub(crate) fn bar_head(ui: &mut Ui, back: Option<(&str, Action)>, name: &str) -> f32 {
    ui.head = Some((name.to_string(), back.map(|(l, a)| (l.to_string(), a))));
    faraday_draw::tokens::PAGE_TOP_BAR
}

/// The room a supporting pane takes from the page at its right: the pane,
/// the spacer before it and the margin after it.
pub(crate) fn support_pane_room() -> f32 {
    SUPPORT_PANE + SPACER + MARGIN_WIDE
}

/// Draws the whole frame.
#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
pub(crate) fn draw(app: &View, ui: &mut Ui) {
    let (full_w, full_h) = app.size();
    ui.dice_shown = app.keygen_dice_shown();
    ui.press_shown = app.keygen.press_shown(app.now_ms, app.reduce_motion());
    // A small panel's keyboard, when it is up, takes the foot of the
    // panel; everything else is laid out above it. The wide layout's,
    // when brought up from its button, takes the foot of the page area,
    // beside the sidebar. Whether it came up is worked out before the
    // frame is drawn (`Faraday::osk_track`).
    let kb = app.osk_height();
    let win_h = full_h;
    let full_h = full_h - kb;
    // The wide layout's page keeps its height under its keyboard, drawn
    // up as far as keeps the focused field in view.
    let lift = if kb > 0.0 && !app.is_compact() && app.sheet.is_none() {
        app.osk_lift(full_h, ui.f)
    } else {
        0.0
    };
    // A failed self-test is the whole screen: no sidebar, no sheet.
    if let Some(Err(check)) = app.selftest() {
        selftest_failed(ui, full_w, full_h, check);
        return;
    }
    // Under a sheet that has its frosted copy of the page, the page
    // is not drawn again: the copy stands for it.
    let frosted = app.sheet.is_some() && ui.frost.is_some();
    let side = crate::glance::sheet::side_now(app);
    if !frosted && app.is_compact() {
        ui.clear(Role::Surface);
        // The stick pills take a strip across the top; the page is
        // drawn below it, beside the rail or above the bar
        // (`docs/LAYOUT.md` §3), both drawn after it, over it.
        let strip = crate::stick::pills::strip_h(app, ui, full_w);
        let (rail_w, bar_h) = nav_room(app, kb > 0.0);
        let page_h = full_h - strip - bar_h;
        ui.oy = strip;
        draw_compact(app, ui, (rail_w, full_w - rail_w), page_h, kb > 0.0, strip);
        ui.oy = strip;
        ui.ox = 0.0;
        if rail_w > 0.0 {
            crate::nav::screen::rail(app, ui, win_h - strip);
        }
        if bar_h > 0.0 {
            crate::nav::screen::bar(app, ui, page_h, full_w);
        }
        ui.oy = 0.0;
    } else if !frosted && app.held {
        ui.clear(Role::Surface);
        let page_h = if kb > 0.0 { win_h } else { full_h };
        held_frame(app, ui, full_w, full_h, (page_h, lift));
    } else if !frosted {
        ui.clear(Role::Surface);
        sidebar(app, ui, win_h);
        // A chart's side sheet takes its width from the room, so the
        // chart stands beside it (`docs/DESIGN.md` §4.16).
        let room =
            full_w - SIDEBAR_W - side.map_or(0.0, |s| s.1 * faraday_draw::tokens::SIDE_SHEET_W);
        // Learn's supporting pane takes its room at the right, so the
        // page is laid out beside it and nothing of it is under the pane
        // (`docs/LAYOUT.md` §6).
        let pane = app.learn_pane_shown();
        let beside = if pane { support_pane_room() } else { 0.0 };
        let (ox, cw) = page_area(app.screen, room - beside);
        let h = full_h;
        ui.ox = ox;
        let x = SIDEBAR_W;
        let page_h = if kb > 0.0 { win_h } else { h };
        // Under the top app bar, which is drawn after the page and which
        // the page scrolls under: nothing of the page is drawn or pressed
        // above the bar's foot (`docs/LAYOUT.md` §3). A device's stick
        // pills stand in the bar, so the page never moves for them.
        let top = APP_BAR_H;
        ui.ox = 0.0;
        ui.oy = 0.0;
        let clip = ui.rect(x, top, room, (page_h - top).max(0.0));
        ui.c.push_clip(clip);
        ui.ox = ox;
        ui.oy = top - lift;
        ui.head = None;
        dispatch(app, ui, x, cw, page_h - top);
        ui.c.pop_clip();
        ui.oy = 0.0;
        ui.ox = 0.0;
        if pane {
            // Under the top app bar, as the page is.
            ui.oy = top;
            learn_pane::draw(app, ui, (x, full_w - x, h - top));
            ui.oy = 0.0;
        }
        // Home's Scan floats at the window's right edge and foot
        // (`docs/LAYOUT.md` §15), whatever area its tiles are laid out
        // in; beside Learn's pane, at the page's edge.
        if app.screen == Screen::Home {
            ui.fab(
                full_w - if pane { beside } else { MARGIN_WIDE },
                full_h - MARGIN_WIDE,
                Icon::Scan,
                EN.home_scan,
                Action::Scan,
            );
        }
        // The top app bar over the page, a device's stick pills in it.
        ui.ox = x;
        ui.oy = 0.0;
        crate::nav::screen::app_bar(app, ui, room, Nav::ExpandedRail);
        ui.ox = 0.0;
        ui.oy = 0.0;
        // Centred on the window, sidebar and all.
        notices(app, ui, 0.0, full_w, h);
    }
    let (w, h) = (full_w, full_h);
    if let Some(s) = side {
        // Beside the page, which stays as it is and takes presses.
        crate::glance::sheet::draw_side(app, ui, s, w, h);
    }
    if let Some(sheet) = app.sheet.filter(|_| !side.is_some_and(|s| s.2)) {
        // Everything under a sheet is out of reach.
        ui.clear_hits();
        ui.in_sheet = true;
        ui.offset = app.sheet_offset;
        // A ⋮ menu, and Length's Other, is a popup on `wide`: the page
        // stays in view under it, undimmed (`docs/BACKUP-FLOW.md` §17.20).
        if matches!(
            sheet,
            Sheet::BoardMenu | Sheet::FileMenu | Sheet::KeyLengths
        ) && !app.is_compact()
        {
            ui.still();
        } else if matches!(sheet, Sheet::Learn) && app.held {
            // Held, the Learn sheet is the page under the holder's top
            // app bar (`docs/STARTER.md` §3.1d).
            ui.clear(Role::Surface);
        } else {
            ui.backdrop();
        }
        // A press beside the sheet leaves it as its own way out does. The
        // word list takes such a press itself.
        ui.outside = match sheet {
            Sheet::Lock => Some(Action::NotNow),
            Sheet::Import => Some(crate::boot_import::LATER),
            Sheet::UnlockLoad => Some(Action::Vault(crate::vault::VaultAction::LoadNotNow)),
            Sheet::OlderCopy => Some(Action::Vault(crate::vault::VaultAction::OlderBack)),
            // A new touchscreen's sheet is the whole screen: only its
            // Ignore keeps the device out.
            Sheet::NewInput => app
                .new_inputs()
                .first()
                .filter(|d| !d.by_target())
                .map(|d| Action::InputIgnore(d.id)),
            // Zeroing stops by its own Stop, never by a press beside it.
            Sheet::WordList | Sheet::IdleWarn | Sheet::Zeroing => None,
            Sheet::BoardMenu => Some(Action::Board(crate::board::BoardPress::MenuClose)),
            Sheet::KeyPath => Some(Action::KeyPathCancel),
            Sheet::Register => Some(Action::RegisterCancel),
            _ => Some(Action::Cancel),
        };
        // What the page under it handed over for the demo is not the
        // sheet's: the host's strip shows the sheet's own
        // (`docs/DESKTOP-HOST.md` §3.1).
        let _ = ui.take_demo();
        match sheet {
            Sheet::Lock => lock_sheet(app, ui, w, h),
            Sheet::LockAsk => lock_ask_sheet(app, ui, w, h),
            Sheet::Power => power_sheet(app, ui, w, h),
            Sheet::Qr => qr_sheet(app, ui, w, h),
            Sheet::Scan => scan_sheet(app, ui, w, h),
            Sheet::Locked => locked_sheet(app, ui, w, h),
            Sheet::NewInput => new_input_sheet(app, ui, w, h),
            Sheet::Learn => learn_sheet(app, ui, w, h),
            Sheet::WordList => crate::wordlist::screen::draw(app, ui, w, h),
            Sheet::SecretOut => secret_out_sheet(app, ui, w, h),
            Sheet::WriteOut => write_out_sheet(app, ui, w, h),
            Sheet::IdleWarn => idle_warn_sheet(app, ui, w, h),
            Sheet::Potential => potential_sheet(app, ui, w, h),
            Sheet::Import => crate::boot_import::screen::draw(app, ui, w, h),
            Sheet::Pull => pull_sheet(app, ui, w, h),
            Sheet::Chart => crate::glance::sheet::draw(app, ui, w, h),
            Sheet::BoardList => crate::board::screen::list_sheet(app, ui, w, h),
            Sheet::BoardSees => crate::board::screen::sees_sheet(app, ui, w, h),
            Sheet::BoardPartials => crate::board::screen::partials_sheet(app, ui, w, h),
            Sheet::BoardMenu => crate::board::screen::menu_sheet(app, ui, w, h),
            Sheet::SeedRead => crate::seeds::read::sheet(app, ui, w, h),
            Sheet::Sweep => crate::seeds::sweep::sheet(app, ui, w, h),
            Sheet::Delete => crate::stick::wipe::delete_sheet(app, ui, w, h),
            Sheet::Zeroing => crate::stick::wipe::zeroing_sheet(app, ui, w, h),
            Sheet::Taken => crate::visit_taken::taken_sheet(app, ui, w, h),
            Sheet::Devices => devices_sheet(app, ui, w, h),
            Sheet::KeyPath => key_path_sheet(app, ui, w, h),
            Sheet::Register => crate::register::sheet(app, ui, w, h),
            Sheet::VaultSave => crate::vault::screen::stick_save_sheet(app, ui, w, h),
            Sheet::CancelFlow(g) => crate::in_progress::cancel_sheet(app, ui, g, w, h),
            Sheet::CancelKey => crate::in_progress::cancel_key_sheet(app, ui, w, h),
            Sheet::FileMenu => crate::file_menu::menu_sheet(app, ui, w, h),
            Sheet::Rename => crate::file_menu::rename_sheet(app, ui, w, h),
            Sheet::Details => crate::file_menu::details_sheet(app, ui, w, h),
            Sheet::UnlockLoad => crate::vault::load_screen::draw(app, ui, w, h),
            Sheet::OlderCopy => crate::vault::load_screen::older_copy(app, ui, w, h),
            Sheet::ChildRecord => crate::bip85::screen::record_dialog(app, ui, w, h),
            Sheet::ChildNotice => crate::screens::create::child_notice(app, ui, w, h),
            Sheet::PassphraseSeeds => crate::vault::load_screen::pass_seeds(app, ui, w, h),
            Sheet::GpgSign => crate::vault::screen::gpg_sign_sheet(app, ui, w, h),
            Sheet::Mode => crate::modes::screen::sheet(app, ui, w, h),
            Sheet::Samples => crate::samples::screen::sheet(app, ui, w, h),
            Sheet::NavMenu(None) => crate::nav::screen::menu_sheet(ui, w, h),
            Sheet::NavMenu(Some(o)) => crate::compact::overflow_sheet(app, ui, w, h, o),
            Sheet::KeyLengths => crate::keygen::screen::lengths_menu(app, ui, w, h),
            Sheet::GpgRestore => crate::backup_flow::gpg_restore_screen::sheet(app, ui, w, h),
        }
        ui.oy = 0.0;
        ui.ox = 0.0;
    }
    // A sheet with a field of its own: the wide layout's keyboard button
    // stays within reach, over it, where the page has it.
    if !app.is_compact() && app.sheet.is_some() && app.osk_wanted() {
        ui.ox = 0.0;
        let right = full_w - TOP_ROW_QUESTION - faraday_draw::tokens::GAP;
        crate::compact::wide_keyboard_button(app, ui, right);
    }
    if kb > 0.0 {
        if app.is_compact() {
            let x = nav_room(app, true).0;
            crate::compact::keyboard(app, ui, x, full_w - x, full_h);
        } else {
            let (x, w) = if app.held {
                (0.0, full_w)
            } else {
                (SIDEBAR_W, full_w - SIDEBAR_W)
            };
            crate::compact::keyboard(app, ui, x, w, full_h);
        }
    }
    drop_edge(app, ui, (full_w, full_h), kb > 0.0);
    // A device's stick pills, pinned at the top over the page and any
    // sheet; Faraday Demo's, in its top bar, again over a sheet, and
    // its Sticks menu as it closes.
    crate::stick::pills::draw(app, ui, full_w);
    // Faraday Demo's host takes what the page and its sheet offered
    // for the demo into its strip (`docs/DESKTOP-HOST.md` §3).
    if app.hosted() {
        app.sample_area.note_aids(ui.take_demo());
    }
}

/// A file dragged over Faraday Demo's window: the page beside the
/// navigation edged in the accent as the place it drops, as the board
/// edges its drop targets (`docs/DESIGN.md` §4.17); over a sheet, the
/// whole window. Reads the state.
fn drop_edge(app: &View, ui: &mut Ui, (full_w, full_h): (f32, f32), kb: bool) {
    use faraday_draw::tokens::BOARD_EDGE;
    if !app.transfer.hovered() {
        return;
    }
    let (x, w, h) = if app.sheet.is_some() || app.held {
        (0.0, full_w, full_h)
    } else if app.is_compact() {
        let (rail, bar) = nav_room(app, kb);
        (rail, full_w - rail, full_h - bar)
    } else {
        (SIDEBAR_W, full_w - SIDEBAR_W, full_h)
    };
    ui.ox = 0.0;
    ui.oy = 0.0;
    // The stroke lies inside the rectangle, along the page's own edges.
    let rect = ui.rect(x, 0.0, w, h);
    let c = ui.col(Role::Primary);
    ui.c.stroke_rounded_rect(rect, 0.0, BOARD_EDGE * ui.f, c);
}

/// A held app's frame on a wide display ([`Faraday::held`]): no sidebar
/// and no way back of its own, since the holder draws its top app bar
/// above this frame (`docs/STARTER.md` §3.1d); the screen in the width
/// the sidebar frame gives it. Its Home draws nothing: the holder's Home
/// is the one a person sees.
/// `page` is the page's height and how far it is drawn up: under the
/// keyboard, the window's height, drawn up to keep the field in view.
fn held_frame(app: &View, ui: &mut Ui, full_w: f32, full_h: f32, page: (f32, f32)) {
    let (page_h, lift) = page;
    let (ox, cw) = page_area(app.screen, full_w);
    ui.ox = ox;
    if app.screen != Screen::Home {
        ui.oy = -lift;
        dispatch(app, ui, 0.0, cw, page_h);
        ui.oy = 0.0;
    }
    // The holder's way back is the one Back and Escape take.
    ui.back(Action::Nav(Screen::Home));
    ui.back_hit = None;
    notices(app, ui, 0.0, cw, full_h);
    ui.ox = 0.0;
    ui.oy = 0.0;
}

/// The screen itself, in the column at `x`, `cw` wide.
fn dispatch(app: &View, ui: &mut Ui, x: f32, cw: f32, h: f32) {
    match app.screen {
        Screen::Home => home::draw(app, ui, x, cw),
        Screen::WalletsStart => wallets_start::draw(app, ui, x, cw, h),
        Screen::Backup => backup::draw(app, ui, x, cw, h),
        Screen::Message => message::draw(app, ui, x, cw, h),
        Screen::CheckMessage => check_message::draw(app, ui, x, cw, h),
        Screen::Create => create::draw(app, ui, x, cw, h),
        Screen::Restore => restore::draw(app, ui, x, cw, h),
        Screen::Wallets => wallets::draw(app, ui, x, cw, h),
        Screen::Spend => spend::draw(app, ui, x, cw, h),
        Screen::Files => files::draw(app, ui, x, cw, h),
        Screen::Visit => visit::draw(app, ui, x, cw, h),
        Screen::Entry => entry::draw(app, ui, x, cw, h),
        Screen::KeyGen => crate::keygen::screen::draw(app, ui, x, cw, h),
        Screen::Bip85 => crate::bip85::screen::draw(app, ui, x, cw, h),
        Screen::Silent => crate::silent::screen::draw(app, ui, x, cw, h),
        Screen::Explore => crate::explore::screen::draw(app, ui, x, cw, h),
        Screen::Lightning => crate::lightning::screen::draw(app, ui, x, cw, h),
        Screen::Tools => crate::tools::screen::draw(app, ui, x, cw, h),
        Screen::Settings => settings::draw(app, ui, x, cw, h),
        Screen::SetTime => set_time::draw(app, ui, x, cw, h),
        Screen::Randomness => randomness::draw(app, ui, x, cw, h),
        Screen::Codes => crate::codes::screen::draw(app, ui, x, cw, h),
        Screen::Vaults | Screen::Unlock if crate::panes::screen::shown(app, app.screen) => {
            crate::vault::panes_screen::draw(app, ui, x, cw, h);
        }
        Screen::Vaults => crate::vault::screen::list(app, ui, x, cw, h),
        Screen::Backups => crate::backups::screen::draw(app, ui, x, cw, h),
        Screen::CreateVault => crate::vault::screen::create(app, ui, x, cw, h),
        Screen::Unlock => crate::vault::screen::unlock(app, ui, x, cw, h),
        Screen::VaultContents => crate::vault::screen::contents(app, ui, x, cw, h),
        Screen::Family => crate::family::screen::draw(app, ui, x, cw, h),
        Screen::Vanity => crate::vanity::screen::draw(app, ui, x, cw, h),
        Screen::Decode => decode::draw(app, ui, x, cw, h),
        Screen::NonceCheck => nonce_check::draw(app, ui, x, cw, h),
        Screen::Catalog => catalog::draw(app, ui, x, cw, h),
        Screen::Transfer => crate::transfer::screen::draw(app, ui, x, cw, h),
        Screen::BackupFlow => crate::backup_flow::screen::draw(app, ui, x, cw, h),
    }
}

/// The room the navigation takes beside one-column pages: the rail's
/// width at the left and the bar's height at the foot, none on the menu,
/// on a step that must not be left, or (for the bar) while the keyboard
/// takes the foot (`docs/LAYOUT.md` §3). A lookup.
fn nav_room(app: &crate::Faraday, kb: bool) -> (f32, f32) {
    if app.nav_hidden() {
        return (0.0, 0.0);
    }
    match app.nav() {
        Nav::Rail => (RAIL_W, 0.0),
        Nav::Bar if !kb => (0.0, NAV_BAR_H),
        _ => (0.0, 0.0),
    }
}

/// The height of a small panel's pinned bar: the page's forward action
/// across the foot.
const PIN_H: f32 = 64.0;

/// A small panel's frame: Home is the menu; every other page has a bar
/// back, unless it is a step flow, which draws its own. A page's forward
/// action is pinned in a bar at the foot, with the page scrolling above
/// it, except while the keyboard is up, whose Done puts it away.
///
/// On the rail's and the bar's windows (`docs/LAYOUT.md` §3) the page
/// is the area `area` (its left edge and width) beside the rail, its top
/// app bar across the area and the page in a column at most `COLUMN_MAX`
/// wide, centred in it; flows in progress show their chips under the
/// top app bar.
fn draw_compact(app: &View, ui: &mut Ui, area: (f32, f32), h: f32, kb: bool, y0: f32) {
    let nav = app.nav();
    let (ax, aw) = area;
    // A list and the one chosen stand side by side across the whole
    // area from expanded windows up (`docs/LAYOUT.md` §5).
    let panes = crate::panes::screen::shown(app, app.screen);
    let w = if nav.beside() && !panes && !whole_area(app) {
        aw.min(COLUMN_MAX)
    } else {
        aw
    };
    let cx = ax + (aw - w) / 2.0;
    // The top app bar across the area; the page's column under it.
    let app_bar = |ui: &mut Ui| {
        ui.ox = ax;
        let head = crate::nav::screen::app_bar(app, ui, aw, nav);
        ui.ox = cx;
        head
    };
    ui.ox = cx;
    let mut foot = h;
    if app.screen == Screen::Home && app.held {
        // The holder's Home is the one a person sees.
    } else if app.screen == Screen::Home {
        let head = if nav.beside() { app_bar(ui) } else { 0.0 };
        ui.oy = y0 + head;
        crate::compact::home(app, ui, w, h - head);
        ui.oy = y0;
    } else {
        // Room for the bar as the last frame had it; a change draws again.
        let had = if kb { 0.0 } else { app.pin_h.get() };
        let page_h = h - had;
        // A held app's way back is drawn by its holder, above this
        // frame; Back and Escape take it here.
        let top = if crate::compact::draws_own_bar(app.screen) {
            0.0
        } else if app.held {
            ui.back(crate::compact::page_head(app).0.1);
            ui.back_hit = None;
            0.0
        } else if nav.beside() {
            let head = app_bar(ui);
            let m = crate::compact::M;
            head + crate::nav::screen::flow_chips(app, ui, m, head, w - 2.0 * m)
        } else {
            crate::compact::page(app, ui, w, page_h).0
        };
        let clip = ui.rect(0.0, top, w, page_h - top);
        ui.c.push_clip(clip);
        ui.oy = y0 + top;
        ui.pin = None;
        ui.pinning = !kb;
        ui.bar_tools_x = crate::compact::draws_own_bar(app.screen)
            .then(|| crate::compact::flow_bar_tools_x(app, ui, w));
        dispatch(app, ui, 0.0, w, page_h - top);
        ui.bar_tools_x = None;
        ui.pinning = false;
        ui.oy = y0;
        ui.c.pop_clip();
        if crate::compact::draws_own_bar(app.screen) {
            crate::compact::flow_bar_tools(app, ui, w);
        }
        let pin = ui.pin.take().filter(|_| !kb);
        let want = if pin.is_some() { PIN_H } else { 0.0 };
        if want != had {
            // The bar came or went: the page again, laid out for it, in
            // this same frame.
            app.pin_h.set(want);
            ui.clear(Role::Surface);
            ui.clear_hits();
            return draw_compact(app, ui, area, h, kb, y0);
        }
        if let Some((label, style, action)) = pin {
            let y = h - PIN_H;
            ui.ox = ax;
            ui.fill(0.0, y, aw, PIN_H, 0.0, Role::SurfaceContainerLow);
            ui.fill(0.0, y, aw, 1.0, 0.0, Role::OutlineVariant);
            ui.ox = cx;
            let m = crate::compact::M;
            let label = ui.fit_as(TITLE_M, &label, w - 2.0 * m - 24.0);
            ui.button(
                m,
                y + 10.0,
                Some(w - 2.0 * m),
                BUTTON_H,
                &label,
                style,
                action,
            );
            foot = y;
        }
    }
    notices(app, ui, 0.0, w, foot);
    ui.ox = 0.0;
    ui.oy = 0.0;
}

/// "Remove the stick" and the toast in `cw` from `x`, at the foot `h`:
/// each Material's snackbar, the toast standing above the notice while
/// both show.
fn notices(app: &View, ui: &mut Ui, x: f32, cw: f32, h: f32) {
    let mut foot = h;
    if app.stick.not_now() && !app.stick.is_empty() && app.sheet.is_none() {
        let msg = &fill(EN.notices_remove_medium, &[app.medium.noun()]);
        foot -= ui.snackbar((x, cw, h), msg, 1.0, 0.0);
    }
    if let Some(t) = app.toast_text().map(str::to_string) {
        // Material's snackbar, rising in and fading as its time runs out.
        let (shown, rise) = app.toast_shown();
        ui.snackbar((x, cw, foot), &t, shown, rise);
    }
}

/// New key's page area's width on the wide layout, as the frame gives
/// it (`docs/LAYOUT.md` §15): what the codex32 worksheet's fit is worked
/// out from. Arithmetic.
pub(crate) fn keygen_page_w(app: &crate::Faraday) -> f32 {
    let full_w = app.size().0;
    if app.compact {
        // One-column pages: the area beside the rail, as `draw_compact`
        // gives it.
        let aw = full_w - nav_room(app, false).0;
        return if app.nav().beside() && !whole_area(app) {
            aw.min(COLUMN_MAX)
        } else {
            aw
        };
    }
    let room = if app.held { full_w } else { full_w - SIDEBAR_W };
    page_area(Screen::KeyGen, room).1
}

/// Whether a one-column page takes the whole area beside the rail rather
/// than a reading column centred in it: Make codex32 shares, whose
/// string, sheets and input panel are wide content (`docs/LAYOUT.md`
/// §14, F114; owner, 2026-10-25). A lookup.
fn whole_area(app: &crate::Faraday) -> bool {
    app.screen == Screen::KeyGen && app.keygen.state().is_some_and(|k| k.codex32.is_some())
}

/// The start-up self-test failed: which check, and Exit. A build whose
/// published vectors do not reproduce has nowhere to go but out.
fn selftest_failed(ui: &mut Ui, w: f32, h: f32, check: &str) {
    let s = &EN;
    ui.clear(Role::Surface);
    let cw = (w - 64.0).min(560.0);
    let x = (w - cw) / 2.0;
    let mut y = (h / 2.0 - 120.0).max(24.0);
    ui.icon(x, y, 30.0, Icon::Error, 18.0, Role::Error);
    ui.text_mid_as(
        (x + 42.0, y, 30.0),
        faraday_draw::tokens::sheet_head(false),
        Role::OnSurface,
        s.selftest_failed_title,
    );
    y += 56.0;
    ui.text_as(x, y, BODY_S, Role::OnSurfaceVariant, s.selftest_check_row);
    y += ui.line_as(BODY_S) + faraday_draw::tokens::GAP_SMALL;
    y += ui.wrap_as((x, y, cw), TITLE_M, Role::Error, check) + 32.0;
    ui.button(
        x,
        y,
        Some(140.0),
        BUTTON_H,
        s.selftest_exit,
        Style::Primary,
        Action::PowerOff,
    );
}
