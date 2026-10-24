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
mod message;
mod parts;
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
use crate::ui::{Style, Ui, W};
use crate::{Action, Screen, Sheet};
pub(crate) use backup::{SOFTWARE_ROWS, extra_copies, held_item, item_title, item_title_now};
pub(crate) use create::CSTEPS;
pub(crate) use decode::decode_back;
pub(crate) use entry::{
    codex32_check_button, codex32_found, codex32_head, codex32_mark, codex32_tick, entry_form,
    entry_scan, entry_stick, entry_typed, entry_words, keyed_status, word_count_row,
};
use faraday_draw::tokens::{COLUMN_MAX, NAV_BAR_H, RAIL_W};
use faraday_draw::widgets::Icon;
pub use faraday_ui::parts::qr_code;
pub(crate) use faraday_ui::parts::{button_rows, section_label, sheet_box, title, wrap_buttons};
pub(crate) use files::vault_panel;
pub(crate) use home::{home_lead_beside_tiles, tile_grid};
pub(crate) use message::{MESSAGE_STEPS, MSTEPS};
pub(crate) use parts::{
    buttons_and_next, code_field, descriptor_row, first_addresses, grid_never, guide_text,
    guide_text_learn, keyed_pills, kind_line, kind_name, locked_tag, never_below, never_real_seed,
    never_real_seed_if, never_under, next_button, pass_field, pin_button, side_panel_w,
    sign_readiness, size_text, stepper, ticked, word_keys, working_bar, working_line,
};
pub(crate) use restore::RSTEPS;
use sheets::{
    devices_sheet, idle_warn_sheet, learn_sheet, lock_ask_sheet, lock_sheet, locked_sheet,
    new_input_sheet, potential_sheet, power_sheet, pull_sheet, qr_sheet, scan_sheet,
    secret_out_sheet, write_out_sheet,
};
use sidebar::sidebar;
pub(crate) use sidebar::{lock_held_pill, name_suffix};
pub(crate) use spend::{STEPS as SPEND_TITLES, signers_count, step_body, step_summary};
pub(crate) use wallets::{backup_line, loose_card, missing_keys_line, slot_marks};

const SIDEBAR_W: f32 = 240.0;

/// The widest the sidebar and a screen together grow, in design units. A
/// wide display shows the screen at this width, centred beside the
/// sidebar, instead of stretching cards across it.
const MAX_W: f32 = 1600.0;

/// Draws the whole frame.
#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
pub(crate) fn draw(app: &View, ui: &mut Ui) {
    let (full_w, full_h) = app.size();
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
    if !frosted && app.is_compact() {
        ui.clear(SURFACE);
        // The stick pills take a strip across the top; the page is
        // drawn below it, beside the rail or above the bar
        // (`docs/LAYOUT.md` §3), both drawn after it, over it.
        let strip = crate::stick::pills::strip_h(app, ui, full_w);
        let (rail_w, bar_h) = nav_room(app, kb > 0.0);
        let page_h = full_h - strip - bar_h;
        ui.oy = strip;
        draw_compact(app, ui, (rail_w, full_w - rail_w), page_h, kb > 0.0, strip);
        ui.oy = strip;
        if rail_w > 0.0 {
            crate::nav::screen::rail(app, ui, win_h - strip);
        }
        if bar_h > 0.0 {
            crate::nav::screen::bar(app, ui, page_h, full_w);
        }
        ui.oy = 0.0;
    } else if !frosted && app.held {
        ui.clear(SURFACE);
        let page_h = if kb > 0.0 { win_h } else { full_h };
        held_frame(app, ui, full_w, full_h, (page_h, lift));
    } else if !frosted {
        ui.clear(SURFACE);
        sidebar(app, ui, win_h);
        let room = full_w - SIDEBAR_W;
        let cw = room.min(MAX_W - SIDEBAR_W);
        let h = full_h;
        ui.ox = (room - cw) / 2.0;
        let x = SIDEBAR_W;
        // The stick pills' strip lies over the room above the page's
        // first line; a taller strip moves the page down by the rest.
        let strip = crate::stick::pills::strip_h(app, ui, full_w);
        let drop = crate::stick::pills::page_drop(strip);
        ui.oy = drop - lift;
        ui.top_row_left = top_row_left(app, ui, x, cw);
        dispatch(app, ui, x, cw, if kb > 0.0 { win_h } else { h } - drop);
        ui.oy = 0.0;
        ui.top_row_left = f32::INFINITY;
        // The stick pills' strip, over whatever scrolls under it; the
        // pills themselves are drawn last, over any sheet.
        if strip > 0.0 {
            ui.fill(x - ui.ox, 0.0, room, strip, 0.0, SURFACE);
        }
        let kx = header_network_pill(app, ui, x + cw - faraday_draw::tokens::TOP_ROW_QUESTION);
        // Under a sheet the keyboard button is drawn over it, below.
        if app.sheet.is_none() {
            crate::compact::wide_keyboard_button(app, ui, kx);
            // The codex32 sheets' eye, in the right margin under the ?.
            crate::compact::wide_eye_button(app, ui, x + cw);
        }
        // The ? at the top right opens what explains this screen.
        if !app.learn_pages().is_empty() {
            // In the right margin, clear of every screen's own title row.
            let (qx, qy) = (x + cw - faraday_draw::tokens::TOP_ROW_QUESTION, 12.0);
            let pressed = ui.is_pressed(Action::Learn);
            ui.fill(
                qx,
                qy,
                28.0,
                28.0,
                14.0,
                if pressed { INNER } else { SURFACE },
            );
            ui.stroke(qx, qy, 28.0, 28.0, 14.0, BORDER);
            let tw = ui.measure(14.0, W::S, "?");
            ui.text_mid(qx + 14.0 - tw / 2.0, qy, 28.0, 14.0, W::S, MUTED, "?");
            ui.hit(qx - 6.0, qy - 6.0, 40.0, 40.0, Action::Learn);
        }
        ui.ox = 0.0;
        ui.oy = 0.0;
        // Centred on the window, sidebar and all.
        notices(app, ui, 0.0, full_w, h);
    }
    let (w, h) = (full_w, full_h);
    if let Some(sheet) = app.sheet {
        // Everything under a sheet is out of reach.
        ui.clear_hits();
        ui.in_sheet = true;
        ui.offset = app.sheet_offset;
        // A chip's ⋮ menu is a popup on `wide`: the board stays in view
        // under it, undimmed (`docs/BACKUP-FLOW.md` §17.20).
        if matches!(sheet, Sheet::BoardMenu | Sheet::FileMenu) && !app.is_compact() {
            ui.still();
        } else {
            ui.backdrop();
        }
        // A press beside the sheet leaves it as its own way out does. The
        // word list takes such a press itself.
        ui.outside = match sheet {
            Sheet::Lock => Some(Action::NotNow),
            Sheet::Import => Some(crate::boot_import::LATER),
            Sheet::UnlockLoad => Some(Action::Vault(crate::vault::VaultAction::LoadNotNow)),
            Sheet::NewInput => app.new_inputs().first().map(|d| Action::InputIgnore(d.id)),
            // Zeroing stops by its own Stop, never by a press beside it.
            Sheet::WordList | Sheet::IdleWarn | Sheet::Zeroing => None,
            Sheet::BoardMenu => Some(Action::Board(crate::board::BoardPress::MenuClose)),
            _ => Some(Action::Cancel),
        };
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
            Sheet::Delete => crate::stick::wipe::delete_sheet(app, ui, w, h),
            Sheet::Zeroing => crate::stick::wipe::zeroing_sheet(app, ui, w, h),
            Sheet::Taken => crate::visit_taken::taken_sheet(app, ui, w, h),
            Sheet::Devices => devices_sheet(app, ui, w, h),
            Sheet::VaultSave => crate::vault::screen::stick_save_sheet(app, ui, w, h),
            Sheet::CancelFlow(g) => crate::in_progress::cancel_sheet(app, ui, g, w, h),
            Sheet::FileMenu => crate::file_menu::menu_sheet(app, ui, w, h),
            Sheet::Rename => crate::file_menu::rename_sheet(app, ui, w, h),
            Sheet::Details => crate::file_menu::details_sheet(app, ui, w, h),
            Sheet::UnlockLoad => crate::vault::load_screen::draw(app, ui, w, h),
            Sheet::PassphraseSeeds => crate::vault::load_screen::pass_seeds(app, ui, w, h),
            Sheet::GpgSign => crate::vault::screen::gpg_sign_sheet(app, ui, w, h),
            Sheet::Mode => crate::modes::screen::sheet(app, ui, w, h),
            Sheet::Samples => crate::samples::screen::sheet(app, ui, w, h),
            Sheet::NavMenu => crate::nav::screen::menu_sheet(ui, w, h),
        }
        ui.oy = 0.0;
    }
    // A sheet with a field of its own: the wide layout's keyboard button
    // stays within reach, over it, where the page has it.
    if !app.is_compact() && app.sheet.is_some() && app.osk_wanted() {
        let (x, cw) = wide_page(app, full_w);
        ui.ox = 0.0;
        let right = if app.held {
            x + cw - faraday_draw::tokens::TOP_ROW_QUESTION - faraday_draw::tokens::GAP
        } else {
            network_pill_left(app, ui, x + cw - faraday_draw::tokens::TOP_ROW_QUESTION)
        };
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
    // The stick pills, pinned at the top over the page and any sheet.
    crate::stick::pills::draw(app, ui, full_w);
}

/// The height of the strip across the top of a held app's frame
/// ([`Faraday::held`]): the way back to the holder's Home.
const HELD_TOP: f32 = 52.0;

/// A held app's frame on a wide display: no sidebar, the way back to the
/// holder's Home across the top, and the screen under it in the width
/// the sidebar frame gives it. Its Home draws nothing: the holder's Home
/// is the one a person sees.
/// `page` is the page's height and how far it is drawn up: under the
/// keyboard, the window's height, drawn up to keep the field in view.
fn held_frame(app: &View, ui: &mut Ui, full_w: f32, full_h: f32, page: (f32, f32)) {
    let (page_h, lift) = page;
    let cw = full_w.min(MAX_W - SIDEBAR_W);
    ui.ox = (full_w - cw) / 2.0;
    if app.screen != Screen::Home {
        // The screen's own top margin sits under the strip.
        let top = HELD_TOP - 24.0;
        ui.oy = top - lift;
        dispatch(app, ui, 0.0, cw, page_h - top);
        ui.oy = 0.0;
        ui.fill(-ui.ox, 0.0, full_w, HELD_TOP, 0.0, SURFACE);
    }
    let back = Action::Nav(Screen::Home);
    let label = EN.held_frame_home;
    let bw = ui.measure(14.0, W::S, label) + 44.0;
    let (bx, by) = (24.0, 10.0);
    if ui.is_pressed(back) || ui.is_hovered(back) {
        ui.fill(bx, by, bw, 34.0, 8.0, INNER);
    }
    ui.icon(bx + 4.0, by + 5.0, 24.0, Icon::ChevronLeft, 12.0, MUTED);
    ui.text_mid(bx + 30.0, by, 34.0, 14.0, W::S, MUTED, label);
    ui.hit(bx, by, bw, 34.0, back);
    ui.back(back);
    if app.screen != Screen::Home && !app.learn_pages().is_empty() {
        let (qx, qy) = (cw - faraday_draw::tokens::TOP_ROW_QUESTION, 12.0);
        let pressed = ui.is_pressed(Action::Learn);
        ui.fill(
            qx,
            qy,
            28.0,
            28.0,
            14.0,
            if pressed { INNER } else { SURFACE },
        );
        ui.stroke(qx, qy, 28.0, 28.0, 14.0, BORDER);
        let tw = ui.measure(14.0, W::S, "?");
        ui.text_mid(qx + 14.0 - tw / 2.0, qy, 28.0, 14.0, W::S, MUTED, "?");
        ui.hit(qx - 6.0, qy - 6.0, 40.0, 40.0, Action::Learn);
    }
    if app.screen != Screen::Home && app.sheet.is_none() {
        crate::compact::wide_keyboard_button(
            app,
            ui,
            cw - faraday_draw::tokens::TOP_ROW_QUESTION - faraday_draw::tokens::GAP,
        );
    }
    notices(app, ui, 0.0, cw, full_h);
    ui.ox = 0.0;
    ui.oy = 0.0;
}

/// The screen itself, in the column at `x`, `cw` wide.
fn dispatch(app: &View, ui: &mut Ui, x: f32, cw: f32, h: f32) {
    match app.screen {
        Screen::Home => home::draw(app, ui, x, cw, h),
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
        Screen::Codes => crate::codes::screen::draw(app, ui, x, cw, h),
        Screen::Vaults => crate::vault::screen::list(app, ui, x, cw, h),
        Screen::Backups => crate::backups::screen::draw(app, ui, x, cw, h),
        Screen::CreateVault => crate::vault::screen::create(app, ui, x, cw, h),
        Screen::Unlock => crate::vault::screen::unlock(app, ui, x, cw, h),
        Screen::VaultContents => crate::vault::screen::contents(app, ui, x, cw, h),
        Screen::Family => crate::family::screen::draw(app, ui, x, cw, h),
        Screen::Vanity => crate::vanity::screen::draw(app, ui, x, cw, h),
        Screen::Decode => decode::draw(app, ui, x, cw, h),
        Screen::Catalog => catalog::draw(app, ui, x, cw, h),
        Screen::Transfer => crate::transfer::screen::draw(app, ui, x, cw, h),
        Screen::Upgrade => crate::upgrade::screen::draw(app, ui, x, cw, h),
        Screen::BackupFlow => crate::backup_flow::screen::draw(app, ui, x, cw, h),
    }
}

/// The room the navigation takes beside one-column pages: the rail's
/// width at the left and the bar's height at the foot, none on the menu,
/// on a step that must not be left, or (for the bar) while the keyboard
/// takes the foot (`docs/LAYOUT.md` §3). A lookup.
fn nav_room(app: &View, kb: bool) -> (f32, f32) {
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
    let w = if nav.beside() { aw.min(COLUMN_MAX) } else { aw };
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
        let top = if crate::compact::draws_own_bar(app.screen) {
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
        dispatch(app, ui, 0.0, w, page_h - top);
        ui.pinning = false;
        ui.oy = y0;
        ui.c.pop_clip();
        if top == 0.0 {
            crate::compact::flow_bar_tools(app, ui, w);
        }
        let pin = ui.pin.take().filter(|_| !kb);
        let want = if pin.is_some() { PIN_H } else { 0.0 };
        if want != had {
            // The bar came or went: the page again, laid out for it, in
            // this same frame.
            app.pin_h.set(want);
            ui.clear(SURFACE);
            ui.clear_hits();
            return draw_compact(app, ui, area, h, kb, y0);
        }
        if let Some((label, style, action)) = pin {
            let y = h - PIN_H;
            ui.ox = ax;
            ui.fill(0.0, y, aw, PIN_H, 0.0, SIDEBAR);
            ui.fill(0.0, y, aw, 1.0, 0.0, LINE);
            ui.ox = cx;
            let m = crate::compact::M;
            let label = ui.fit(15.0, W::S, &label, w - 2.0 * m - 24.0);
            ui.button(m, y + 10.0, Some(w - 2.0 * m), 44.0, &label, style, action);
            foot = y;
        }
    }
    notices(app, ui, 0.0, w, foot);
    ui.ox = 0.0;
    ui.oy = 0.0;
}

/// "Remove the stick" and the toast, centred in `cw` from `x`, at the foot.
fn notices(app: &View, ui: &mut Ui, x: f32, cw: f32, h: f32) {
    if app.stick.not_now() && !app.stick.is_empty() && app.sheet.is_none() {
        let msg = &fill(EN.notices_remove_medium, &[app.medium.noun()]);
        let bw = ui.measure(14.0, W::S, msg) + 40.0;
        let bx = x + (cw - bw) / 2.0;
        ui.shadow(
            bx,
            h - 64.0,
            bw,
            44.0,
            10.0,
            faraday_draw::tokens::ELEVATION_FAB,
        );
        ui.fill(bx, h - 64.0, bw, 44.0, 10.0, BG);
        ui.fill(bx, h - 64.0, bw, 44.0, 10.0, WARN.with_alpha(40));
        ui.stroke(bx, h - 64.0, bw, 44.0, 10.0, WARN.with_alpha(110));
        ui.text_mid(bx + 20.0, h - 64.0, 44.0, 14.0, W::S, WARN, msg);
    }
    if let Some(t) = app.toast_text().map(str::to_string) {
        // Material's snackbar: in the inverse colours, so it reads as a
        // message over the page. It rises in, and fades as its time runs
        // out.
        use faraday_draw::tokens::{SHAPE_XS, TOAST_H, TOAST_PAD};
        let (shown, rise) = app.toast_shown();
        let a = |c: faraday_draw::Color| c.with_alpha((f32::from(c.a) * shown) as u8);
        let ty = h - 64.0 + rise;
        let tw = ui.measure(14.0, W::R, &t) + 2.0 * TOAST_PAD;
        let tx = x + (cw - tw) / 2.0;
        if shown >= 1.0 {
            ui.shadow(
                tx,
                ty,
                tw,
                TOAST_H,
                SHAPE_XS,
                faraday_draw::tokens::ELEVATION_FAB,
            );
        }
        ui.wash(tx, ty, tw, TOAST_H, SHAPE_XS, a(INVERSE));
        ui.text_mid(tx + TOAST_PAD, ty, TOAST_H, 14.0, W::R, a(ON_INVERSE), &t);
    }
}

/// The mode the session is in, at the top right of every page beside
/// the ? (at `qx`, where the ? is or would be), centred on it, a token
/// gap to its left. Off Mainnet only; pressed, it opens the mode sheet.
/// Returns where the next thing in the row ends, a gap to its left.
fn header_network_pill(app: &View, ui: &mut Ui, qx: f32) -> f32 {
    let left = network_pill_left(app, ui, qx);
    if crate::modes::screen::shows_pill(app) {
        let pw = crate::modes::screen::pill_width(app, ui);
        crate::modes::screen::pill(app, ui, qx - faraday_draw::tokens::GAP - pw, 12.0);
    }
    left
}

/// Where the wide layout's top-right row (the mode pill and the ?)
/// begins, on the page `x` to `x + cw`: a flow's column that scrolls
/// under it ends left of it.
pub(crate) fn top_row_left(app: &View, ui: &Ui, x: f32, cw: f32) -> f32 {
    network_pill_left(app, ui, x + cw - faraday_draw::tokens::TOP_ROW_QUESTION)
}

/// Where the top-right row's next item ends, left of the ? (at `qx`)
/// and the mode pill when there is one, a gap between each.
fn network_pill_left(app: &View, ui: &Ui, qx: f32) -> f32 {
    let gap = faraday_draw::tokens::GAP;
    if !crate::modes::screen::shows_pill(app) {
        return qx - gap;
    }
    qx - gap - crate::modes::screen::pill_width(app, ui) - gap
}

/// The wide layout's page area: its left edge and width, the screen
/// centred beside the sidebar on a wide display (or, held, centred in
/// the window), as the frame draws it.
fn wide_page(app: &View, full_w: f32) -> (f32, f32) {
    if app.held {
        let cw = full_w.min(MAX_W - SIDEBAR_W);
        ((full_w - cw) / 2.0, cw)
    } else {
        let room = full_w - SIDEBAR_W;
        let cw = room.min(MAX_W - SIDEBAR_W);
        (SIDEBAR_W + (room - cw) / 2.0, cw)
    }
}

/// The start-up self-test failed: which check, and Exit. A build whose
/// published vectors do not reproduce has nowhere to go but out.
fn selftest_failed(ui: &mut Ui, w: f32, h: f32, check: &str) {
    let s = &EN;
    ui.clear(SURFACE);
    let cw = (w - 64.0).min(560.0);
    let x = (w - cw) / 2.0;
    let mut y = (h / 2.0 - 120.0).max(24.0);
    ui.icon(x, y, 30.0, Icon::Error, 18.0, ERR);
    ui.text_mid(x + 42.0, y, 30.0, 20.0, W::S, TEXT, s.selftest_failed_title);
    y += 56.0;
    ui.text(x, y, 12.0, W::R, MUTED, s.selftest_check_row);
    y += 20.0;
    y += ui.wrap(x, y, cw, 15.0, W::S, ERR, check) + 32.0;
    ui.button(
        x,
        y,
        Some(140.0),
        46.0,
        s.selftest_exit,
        Style::Primary,
        Action::PowerOff,
    );
}
