//! Every screen, drawn from the app's state each frame. The layouts are
//! the prototype's (`design/prototype/project/`): a 240-unit sidebar and
//! the screen beside it.

use osk_psbt::{Level, OutputKind};
use osk_ui::widgets::Icon;

use crate::ui::pal::*;
use crate::ui::{Style, Theme, Ui, W, btc, grouped, short, thousands};
use crate::wallet::{FileKind, Session, fp_text, key_line, network_name, step, txid_known};
use crate::{Action, Code, Faraday, Screen, Sheet, flow, guide};

const SIDEBAR_W: f32 = 240.0;

/// Draws the whole frame.
/// The widest the sidebar and a screen together grow, in design units. A
/// wide display shows the screen at this width, centred beside the
/// sidebar, instead of stretching cards across it.
const MAX_W: f32 = 1600.0;

pub(crate) fn draw(app: &mut Faraday, ui: &mut Ui) {
    let (full_w, full_h) = app.size();
    // A small panel's keyboard, when it is up, takes the foot of the
    // panel; everything else is laid out above it.
    let kb = if app.is_compact() {
        let rose = app.osk_track();
        let kb = app.osk_height();
        if rose && kb > 0.0 {
            app.osk_reveal(ui.f, full_h - kb);
        }
        kb
    } else {
        0.0
    };
    let full_h = full_h - kb;
    // A failed self-test is the whole screen: no sidebar, no sheet.
    if let Some(Err(check)) = app.selftest() {
        selftest_failed(ui, full_w, full_h, check);
        return;
    }
    // Under a sheet that has its frosted copy of the page, the page
    // is not drawn again: the copy stands for it.
    let frosted = app.sheet.is_some() && ui.frost.is_some();
    if !frosted && app.is_compact() {
        ui.clear(BG);
        draw_compact(app, ui, full_w, full_h, kb > 0.0);
    } else if !frosted {
        ui.clear(BG);
        sidebar(app, ui, full_h);
        let room = full_w - SIDEBAR_W;
        let cw = room.min(MAX_W - SIDEBAR_W);
        let h = full_h;
        ui.ox = (room - cw) / 2.0;
        let x = SIDEBAR_W;
        dispatch(app, ui, x, cw, h);
        // The ? at the top right opens what explains this screen.
        if !app.learn_pages().is_empty() {
            // In the right margin, clear of every screen's own title row.
            let (qx, qy) = (x + cw - 38.0, 12.0);
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
        notices(app, ui, x, cw, h);
        ui.ox = 0.0;
        ui.oy = 0.0;
    }
    let (w, h) = (full_w, full_h);
    if let Some(sheet) = app.sheet {
        // Everything under a sheet is out of reach.
        ui.hits.clear();
        ui.in_sheet = true;
        ui.backdrop();
        // A press beside the sheet leaves it as its own way out does. The
        // word list takes such a press itself.
        ui.outside = match sheet {
            Sheet::Lock => Some(Action::NotNow),
            Sheet::Import => Some(crate::boot_import::LATER),
            Sheet::NewInput => app.inputs.first().map(|d| Action::InputIgnore(d.id)),
            Sheet::WordList | Sheet::IdleWarn => None,
            // Reached mainnet without asking: only the acknowledgement.
            Sheet::NotAirgapped if app.mainnet_asked.is_none() => None,
            _ => Some(Action::Cancel),
        };
        match sheet {
            Sheet::Lock => lock_sheet(app, ui, w, h),
            Sheet::LockAsk => crate::vault_screens::lock_ask(app, ui, w, h),
            Sheet::Power => power_sheet(app, ui, w, h),
            Sheet::Qr => qr_sheet(app, ui, w, h),
            Sheet::Scan => scan_sheet(app, ui, w, h),
            Sheet::NotAirgapped => not_airgapped_sheet(app, ui, w, h),
            Sheet::Locked => locked_sheet(app, ui, w, h),
            Sheet::NewInput => new_input_sheet(app, ui, w, h),
            Sheet::Learn => learn_sheet(app, ui, w, h),
            Sheet::WordList => crate::wordlist_screen::draw(app, ui, w, h),
            Sheet::SecretOut => secret_out_sheet(app, ui, w, h),
            Sheet::WriteOut => write_out_sheet(app, ui, w, h),
            Sheet::IdleWarn => idle_warn_sheet(app, ui, w, h),
            Sheet::Potential => potential_sheet(app, ui, w, h),
            Sheet::Import => crate::boot_import_screen::draw(app, ui, w, h),
            Sheet::Pull => pull_sheet(app, ui, w, h),
            Sheet::Chart => crate::glance_sheet::draw(app, ui, w, h),
        }
        ui.oy = 0.0;
    }
    if kb > 0.0 {
        crate::compact::keyboard(app, ui, full_w, full_h);
    }
}

/// The screen itself, in the column at `x`, `cw` wide.
fn dispatch(app: &mut Faraday, ui: &mut Ui, x: f32, cw: f32, h: f32) {
    match app.screen {
        Screen::Home => home(app, ui, x, cw, h),
        Screen::Start => start(app, ui, x, cw, h),
        Screen::Backup => backup_screen(app, ui, x, cw, h),
        Screen::Message => message_screen(app, ui, x, cw, h),
        Screen::CheckMessage => check_screen(app, ui, x, cw, h),
        Screen::Create => create_screen(app, ui, x, cw, h),
        Screen::Restore => restore_screen(app, ui, x, cw, h),
        Screen::Wallets => wallets(app, ui, x, cw, h),
        Screen::Spend => spend(app, ui, x, cw, h),
        Screen::Files => files(app, ui, x, cw, h),
        Screen::Visit => visit(app, ui, x, cw, h),
        Screen::Entry => entry(app, ui, x, cw, h),
        Screen::KeyGen => crate::keygen_screen::draw(app, ui, x, cw, h),
        Screen::Bip85 => crate::bip85_screen::draw(app, ui, x, cw, h),
        Screen::Silent => crate::silent_screen::draw(app, ui, x, cw, h),
        Screen::Explore => crate::explore_screen::draw(app, ui, x, cw, h),
        Screen::Lightning => crate::lightning_screen::draw(app, ui, x, cw, h),
        Screen::Tools => crate::tools_screen::draw(app, ui, x, cw, h),
        Screen::Settings => settings(app, ui, x, cw, h),
        Screen::Vaults => crate::vault_screens::list(app, ui, x, cw, h),
        Screen::Backups => crate::backups_screen::draw(app, ui, x, cw, h),
        Screen::CreateVault => crate::vault_screens::create(app, ui, x, cw, h),
        Screen::Unlock => crate::vault_screens::unlock(app, ui, x, cw, h),
        Screen::VaultContents => crate::vault_screens::contents(app, ui, x, cw, h),
        Screen::Family => crate::family_screen::draw(app, ui, x, cw, h),
        Screen::Vanity => crate::vanity_screen::draw(app, ui, x, cw, h),
        Screen::Decode => decode_screen(app, ui, x, cw, h),
        Screen::Catalog => catalog_screen(app, ui, x, cw, h),
        Screen::Transfer => crate::transfer_screen::draw(app, ui, x, cw, h),
        Screen::Upgrade => crate::upgrade_screen::draw(app, ui, x, cw, h),
    }
}

/// The height of a small panel's pinned bar: the page's forward action
/// across the foot.
const PIN_H: f32 = 64.0;

/// A small panel's frame: Home is the menu; every other page has a bar
/// back, unless it is a step flow, which draws its own. A page's forward
/// action is pinned in a bar at the foot, with the page scrolling above
/// it, except while the keyboard is up, whose Done puts it away.
fn draw_compact(app: &mut Faraday, ui: &mut Ui, w: f32, h: f32, kb: bool) {
    let mut foot = h;
    if app.screen == Screen::Home {
        crate::compact::home(app, ui, w, h);
    } else {
        // Room for the bar as the last frame had it; a change draws again.
        let had = if kb { 0.0 } else { app.pin_h.get() };
        let page_h = h - had;
        let top = if crate::compact::draws_own_bar(app.screen) {
            0.0
        } else {
            crate::compact::page(app, ui, w, page_h).0
        };
        let clip = ui.rect(0.0, top, w, page_h - top);
        ui.c.push_clip(clip);
        ui.oy = top;
        ui.pin = None;
        ui.pinning = !kb;
        dispatch(app, ui, 0.0, w, page_h - top);
        ui.pinning = false;
        ui.oy = 0.0;
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
            ui.clear(BG);
            ui.hits.clear();
            return draw_compact(app, ui, w, h, kb);
        }
        if let Some((label, style, action)) = pin {
            let y = h - PIN_H;
            ui.fill(0.0, y, w, PIN_H, 0.0, SIDEBAR);
            ui.fill(0.0, y, w, 1.0, 0.0, LINE);
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

/// "Remove the stick" and the toast, centred at the foot of the column.
fn notices(app: &Faraday, ui: &mut Ui, x: f32, cw: f32, h: f32) {
    if app.not_now && !app.sticks.is_empty() && app.sheet.is_none() {
        let msg = &format!("Remove the {} to keep working", app.medium.noun());
        let bw = ui.measure(14.0, W::S, msg) + 40.0;
        let bx = x + (cw - bw) / 2.0;
        ui.shadow(bx, h - 64.0, bw, 44.0, 10.0);
        ui.fill(bx, h - 64.0, bw, 44.0, 10.0, BG);
        ui.fill(bx, h - 64.0, bw, 44.0, 10.0, WARN.with_alpha(40));
        ui.stroke(bx, h - 64.0, bw, 44.0, 10.0, WARN.with_alpha(110));
        ui.text_mid(bx + 20.0, h - 64.0, 44.0, 14.0, W::S, WARN, msg);
    }
    if let Some(t) = app.toast_text().map(str::to_string) {
        // It rises in, and fades as its time runs out.
        let (shown, rise) = app.toast_shown();
        let a = |c: osk_ui::Color| c.with_alpha((f32::from(c.a) * shown) as u8);
        let ty = h - 64.0 + rise;
        let tw = ui.measure(14.0, W::R, &t) + 40.0;
        let tx = x + (cw - tw) / 2.0;
        if shown >= 1.0 {
            ui.shadow(tx, ty, tw, 44.0, 10.0);
        }
        ui.fill(tx, ty, tw, 44.0, 10.0, a(INNER));
        ui.stroke(tx, ty, tw, 44.0, 10.0, a(BORDER));
        ui.text_mid(tx + 20.0, ty, 44.0, 14.0, W::R, a(TEXT), &t);
    }
}

pub(crate) fn title(ui: &mut Ui, x: f32, y: f32, s: &str) -> f32 {
    ui.text(x, y, 28.0, W::S, TEXT, s);
    ui.line(28.0, W::S)
}

/// A screen's walk-through in Guided mode, under its title. Returns the
/// height it took: nothing in Steps only.
pub(crate) fn guide_text(app: &Faraday, ui: &mut Ui, x: f32, y: f32, w: f32, text: &str) -> f32 {
    if !app.guided {
        return 0.0;
    }
    let h = ui.wrap(x + 14.0, y, w - 14.0, 14.0, W::R, TEXT, text);
    ui.fill(x, y, 3.0, h, 1.5, ACCENT.with_alpha(140));
    h + 18.0
}

pub(crate) fn section_label(ui: &mut Ui, x: f32, y: f32, s: &str) {
    ui.text(x, y, 14.0, W::S, MUTED, s);
}

/// The network the session is on, at the right of a screen's title row.
/// A badge, off mainnet only: it opens nothing. The chooser is in
/// Settings.
fn network_pill(app: &Faraday, ui: &mut Ui, right: f32, y: f32) {
    let net = app.session.network();
    if net.is_mainnet() {
        return;
    }
    let label = network_name(net);
    let pw = ui.measure(12.0, W::S, label) + 28.0;
    let px = right - pw;
    ui.fill(px, y, pw, 30.0, 15.0, WARN.with_alpha(30));
    ui.stroke(px, y, pw, 30.0, 15.0, WARN.with_alpha(110));
    ui.text_mid(px + 14.0, y, 30.0, 12.0, W::S, WARN, label);
}

// ---------------------------------------------------------------------
// Sidebar
// ---------------------------------------------------------------------

/// How many rows of seeds and of wallets the sidebar shows, in `rows`:
/// all of both when they fit, else the room shared, the smaller group
/// keeping all it needs when that leaves the other at least half. A
/// group cut short gives its last row to "+ N more".
fn sidebar_caps(seeds: usize, wallets: usize, rows: usize) -> (usize, usize) {
    if seeds + wallets <= rows {
        return (seeds, wallets);
    }
    let half = rows / 2;
    let cap_s = seeds.min(half.max(rows.saturating_sub(wallets)));
    let cap_w = wallets.min(rows - cap_s);
    // What wallets leave unused goes back to seeds.
    let cap_s = seeds.min(rows - cap_w);
    (cap_s, cap_w)
}

/// Whether the seeds loaded here sign for `w` on their own (2), some of
/// its seeds are here (1), or none (0).
pub(crate) fn sign_readiness(app: &Faraday, w: &crate::wallet::Wallet) -> u8 {
    let here = app
        .session
        .slots(w)
        .iter()
        .filter(|s| s.held_by.is_some())
        .count();
    if here >= crate::wallet::needed(w) {
        2
    } else if here > 0 {
        1
    } else {
        0
    }
}

fn sidebar(app: &Faraday, ui: &mut Ui, h: f32) {
    ui.fill(0.0, 0.0, SIDEBAR_W, h, 0.0, SIDEBAR);
    ui.fill(SIDEBAR_W - 1.0, 0.0, 1.0, h, 0.0, LINE);
    ui.mark(22.0, 20.0, 28.0, 32.0);
    ui.text_mid(58.0, 22.0, 28.0, 17.0, W::S, TEXT, "Faraday");
    let net = app.session.network();
    if !net.is_mainnet() {
        // A test network is marked on every screen, by the pill alone.
        let label = network_name(net);
        let tw = ui.measure(12.0, W::S, label) + 24.0;
        let tx = SIDEBAR_W - 16.0 - tw;
        ui.fill(tx, 23.0, tw, 26.0, 13.0, WARN.with_alpha(30));
        ui.stroke(tx, 23.0, tw, 26.0, 13.0, WARN.with_alpha(110));
        ui.text_mid(tx + 12.0, 23.0, 26.0, 12.0, W::S, WARN, label);
    }

    // The places that are always offered, in order; Learn (not a place,
    // a sheet) is drawn between Tools and whatever comes after it.
    let items = [
        (Icon::House, "Home", Screen::Home),
        (Icon::Lock, "Vaults", Screen::Vaults),
        (Icon::File, "Files", Screen::Files),
        (Icon::Wallet, "Wallets", Screen::Start),
        (Icon::Tools, "Tools", Screen::Catalog),
    ];
    let mut tail = Vec::new();
    if !app.sticks.is_empty() && !app.holds_secret() {
        tail.push((app.medium.icon(), app.medium.visit(), Screen::Visit));
    }
    // The online app is the device's QR link.
    if app.online {
        tail.push((Icon::Qr, "Transfer", Screen::Transfer));
    }
    tail.push((Icon::Settings, "Settings", Screen::Settings));
    let mut y = 76.0;
    // Add a key opened from the Spend tab belongs to it.
    let from_spend = app.screen == Screen::Entry && app.entry.back == Some(Screen::Family);
    let v = app.vaults.open.len();
    let row = |ui: &mut Ui, y: f32, icon: Icon, label: &str, active: bool, action: Action| {
        if active || ui.is_pressed(action) {
            ui.fill(12.0, y, SIDEBAR_W - 24.0, 40.0, 8.0, ACCENT.with_alpha(30));
        }
        let fg = if active { TEXT } else { MUTED };
        ui.icon(24.0, y + 11.0, 18.0, icon, 15.0, fg);
        ui.text_mid(54.0, y, 40.0, 14.0, W::S, fg, label);
        // Vaults carries how many are open, at the row's right end.
        if icon == Icon::Lock && v > 0 {
            ui.text_right(
                SIDEBAR_W - 24.0,
                y,
                40.0,
                12.0,
                W::R,
                MUTED,
                &format!("{v} open"),
            );
        }
        ui.hit(12.0, y, SIDEBAR_W - 24.0, 40.0, action);
    };
    for (icon, label, screen) in items.into_iter().chain(tail) {
        let active = app.screen == screen
            || (screen == Screen::Start
                && !from_spend
                && matches!(
                    app.screen,
                    Screen::Wallets
                        | Screen::Vanity
                        | Screen::Spend
                        | Screen::Entry
                        | Screen::KeyGen
                        | Screen::Bip85
                        | Screen::Silent
                        | Screen::Explore
                        | Screen::Lightning
                        | Screen::Tools
                        | Screen::Backup
                        | Screen::Backups
                        | Screen::Message
                        | Screen::CheckMessage
                        | Screen::Create
                        | Screen::Restore
                ))
            || (screen == Screen::Vaults
                && matches!(
                    app.screen,
                    Screen::CreateVault | Screen::Unlock | Screen::VaultContents
                ));
        row(ui, y, icon, label, active, Action::Nav(screen));
        y += 44.0;
        // Learn, after Tools: opens the Learn sheet, not a screen.
        if screen == Screen::Catalog {
            row(
                ui,
                y,
                Icon::Learn,
                "Learn",
                app.sheet == Some(Sheet::Learn),
                Action::Learn,
            );
            y += 44.0;
        }
    }

    // Status at the foot, and above it what is loaded in two groups,
    // seeds and wallets, as high as the last tab. A group that does not
    // fit shows its first rows and how many more, which opens the whole
    // list; each row opens what it names.
    let foot = h - 172.0;
    let seeds = app.session.keys.len();
    let wallets = app.session.wallets.len();
    const ROW: f32 = 22.0;
    const HEAD: f32 = 26.0;
    let groups = usize::from(seeds > 0) + usize::from(wallets > 0);
    let room = (foot - (y + 8.0) - 54.0 - groups as f32 * HEAD).max(0.0);
    let (cap_s, cap_w) = sidebar_caps(seeds, wallets, (room / ROW).floor() as usize);
    let gap = if groups > 0 { 8.0 } else { 0.0 };
    let block = groups as f32 * HEAD + (cap_s + cap_w) as f32 * ROW + gap;
    let mut fy = foot - block;
    ui.rule(12.0, fy, SIDEBAR_W - 24.0, LINE);
    fy += 16.0;
    // The session strip, in the dot line's old place and height: the
    // four stages of the lock cycle, the one now current in the accent,
    // the rest dim. It explains the cycle without prose and opens Files,
    // costing the seed and wallet list below no room of its own.
    {
        let stage = app.session_stage();
        let open_files = Action::Nav(Screen::Files);
        if ui.is_pressed(open_files) {
            ui.fill(
                16.0,
                fy - 4.0,
                SIDEBAR_W - 32.0,
                28.0,
                6.0,
                ACCENT.with_alpha(30),
            );
        }
        let mut sx = 24.0;
        for (i, label) in ["Bring in", "Open", "Work", "Write out"]
            .into_iter()
            .enumerate()
        {
            if i > 0 {
                sx += ui.text_mid(sx, fy, 20.0, 11.0, W::R, DIM, " · ");
            }
            let fg = if label == stage { ACCENT } else { DIM };
            sx += ui.text_mid(sx, fy, 20.0, 11.0, W::S, fg, label);
        }
        ui.hit(16.0, fy - 4.0, SIDEBAR_W - 32.0, 28.0, open_files);
    }
    fy += 30.0;
    let rows_w = SIDEBAR_W - 56.0;
    // One group: its header with the count, its rows, and the rest as a
    // line that opens the whole list.
    let group =
        |ui: &mut Ui,
         fy: &mut f32,
         title: &str,
         total: usize,
         cap: usize,
         rows: &mut dyn Iterator<Item = (String, String, bool, Action, Option<osk_ui::Color>)>,
         more: Action| {
            if total == 0 {
                return;
            }
            ui.text_mid(28.0, *fy, HEAD, 12.0, W::S, MUTED, title);
            ui.text_right(
                SIDEBAR_W - 24.0,
                *fy,
                HEAD,
                12.0,
                W::S,
                MUTED,
                &total.to_string(),
            );
            *fy += HEAD;
            let shown = if total > cap {
                cap.saturating_sub(1)
            } else {
                total
            };
            for (head, tail, mono, action, mark) in rows.take(shown) {
                if ui.is_pressed(action) {
                    ui.fill(16.0, *fy, SIDEBAR_W - 32.0, ROW, 6.0, ACCENT.with_alpha(30));
                }
                // A wallet's dot: green when the seeds here sign for it alone,
                // grey when some of them are here.
                if let Some(c) = mark {
                    ui.dot(18.0, *fy + ROW / 2.0, 3.5, c);
                }
                let face = if mono { W::M } else { W::S };
                let head = ui.fit(12.0, face, &head, rows_w * 0.55);
                let hw = ui.text_mid(28.0, *fy, ROW, 12.0, face, TEXT, &head);
                if !tail.is_empty() {
                    let tone = match mark {
                        Some(c) if c == OK => OK,
                        _ => MUTED,
                    };
                    let tail = ui.fit(12.0, W::R, &tail, rows_w - hw - 8.0);
                    ui.text_mid(36.0 + hw, *fy, ROW, 12.0, W::R, tone, &tail);
                }
                ui.hit(16.0, *fy, SIDEBAR_W - 32.0, ROW, action);
                *fy += ROW;
            }
            if total > cap && cap > 0 {
                let line = format!("+ {} more", total - shown);
                if ui.is_pressed(more) {
                    ui.fill(16.0, *fy, SIDEBAR_W - 32.0, ROW, 6.0, ACCENT.with_alpha(30));
                }
                ui.text_mid(28.0, *fy, ROW, 12.0, W::S, ACCENT, &line);
                ui.hit(16.0, *fy, SIDEBAR_W - 32.0, ROW, more);
                *fy += ROW;
            }
        };
    let mut seed_rows = app.session.keys.iter().map(|k| {
        let fp = fp_text(k.master.fingerprint());
        // A seed named by its fingerprint is not named twice.
        let label = if k.label.eq_ignore_ascii_case(&fp) {
            String::new()
        } else {
            k.label.clone()
        };
        (
            fp,
            label,
            true,
            Action::ExploreKey(k.master.fingerprint().0),
            None,
        )
    });
    group(
        ui,
        &mut fy,
        "Seeds",
        seeds,
        cap_s,
        &mut seed_rows,
        Action::Explore,
    );
    // Wallets the seeds here sign for alone first, then those with some
    // of their seeds here, then watch-only ones: a cut list keeps the
    // ready ones in view.
    let mut ranked: Vec<(usize, u8)> = app
        .session
        .wallets
        .iter()
        .enumerate()
        .map(|(i, w)| (i, sign_readiness(app, w)))
        .collect();
    ranked.sort_by_key(|&(i, r)| (std::cmp::Reverse(r), i));
    let mut wallet_rows = ranked.into_iter().map(|(i, ready)| {
        let w = &app.session.wallets[i];
        let (m, n) = Session::quorum(w);
        let shape = if n > 1 {
            format!("{m} of {n}")
        } else {
            "single key".to_string()
        };
        let mark = match ready {
            2 => Some(OK),
            1 => Some(DIM),
            _ => None,
        };
        (w.name.clone(), shape, false, Action::OpenWallet(i), mark)
    });
    group(
        ui,
        &mut fy,
        "Wallets",
        wallets,
        cap_w,
        &mut wallet_rows,
        Action::Nav(Screen::Wallets),
    );
    fy += gap;
    let files = app.files_count();
    if app.screen == Screen::Files {
        ui.fill(
            16.0,
            fy - 4.0,
            SIDEBAR_W - 32.0,
            28.0,
            6.0,
            ACCENT.with_alpha(30),
        );
    }
    ui.icon(22.0, fy, 20.0, Icon::File, 12.0, MUTED);
    ui.text_mid(46.0, fy, 20.0, 12.0, W::R, MUTED, "Files");
    ui.text_right(SIDEBAR_W - 24.0, fy, 20.0, 12.0, W::R, TEXT, &files);
    ui.hit(
        16.0,
        fy - 4.0,
        SIDEBAR_W - 32.0,
        28.0,
        Action::Nav(Screen::Files),
    );
    fy += 30.0;
    // With no stick attached, an import the boot stick brought waits
    // behind this row.
    let waiting = app.import.is_some() && app.sticks.is_empty();
    let sticks = match app.sticks.len() {
        0 if waiting => "import waiting".to_string(),
        0 => "none".to_string(),
        1 => app.sticks[0].label.clone(),
        k => format!("{k} attached"),
    };
    if waiting && app.sheet == Some(Sheet::Import) {
        ui.fill(
            16.0,
            fy - 4.0,
            SIDEBAR_W - 32.0,
            28.0,
            6.0,
            ACCENT.with_alpha(30),
        );
    }
    ui.icon(22.0, fy, 20.0, app.medium.icon(), 12.0, MUTED);
    ui.text_mid(46.0, fy, 20.0, 12.0, W::R, MUTED, app.medium.caps());
    let sticks = ui.fit(12.0, W::R, &sticks, 110.0);
    ui.text_right(
        SIDEBAR_W - 24.0,
        fy,
        20.0,
        12.0,
        W::R,
        if waiting { ACCENT } else { TEXT },
        &sticks,
    );
    if waiting {
        ui.hit(
            16.0,
            fy - 4.0,
            SIDEBAR_W - 32.0,
            28.0,
            crate::boot_import::OPEN,
        );
    }
    fy += 34.0;
    if app.holds_secret() {
        ui.button(
            16.0,
            fy,
            Some(SIDEBAR_W - 32.0 - 44.0),
            36.0,
            "Lock",
            Style::Secondary,
            Action::LockAsk,
        );
    }
    let px = SIDEBAR_W - 16.0 - 36.0;
    if ui.is_pressed(Action::PowerAsk) {
        ui.fill(px, fy, 36.0, 36.0, 8.0, INNER);
    }
    ui.stroke(px, fy, 36.0, 36.0, 8.0, BORDER);
    ui.icon(px, fy, 36.0, Icon::Power, 14.0, MUTED);
    ui.hit(px, fy, 36.0, 36.0, Action::PowerAsk);
}

// ---------------------------------------------------------------------
// Home
// ---------------------------------------------------------------------

/// Home's one lead job: the first rule of §1.2's ranking that applies.
/// `enabled` is false for a prompt with no target of its own (pull the
/// stick first).
pub(crate) fn home_lead(app: &Faraday) -> (Icon, String, String, Action, bool) {
    use crate::vaults::VaultAction as V;
    // Rule 1: a spend under way; of a wallet loaded, as its card says it
    // (`docs/NEW-WALLET.md` §11.2).
    if let Some(s) = app.spend.as_ref()
        && let Some(w) = s.wallet.and_then(|w| app.session.wallets.get(w))
    {
        return (
            Icon::Sign,
            "Carry on the spend".to_string(),
            format!(
                "{} · Signatures {} of {}",
                w.name,
                s.signers.len(),
                app.spend_needed()
            ),
            app.carry_on(),
            true,
        );
    }
    if let Some(s) = app.spend.as_ref() {
        return (
            Icon::Sign,
            format!("Continue signing {}", s.spend.source),
            format!("{} of {} signatures", s.signers.len(), app.spend_needed()),
            Action::Nav(Screen::Spend),
            true,
        );
    }
    // Rule 2: a PSBT in Files.
    if let Some(i) = app.lead_psbt() {
        return (
            Icon::Sign,
            format!("Sign {}", app.inbox[i].name),
            "From Files".to_string(),
            Action::StartSpend(i),
            true,
        );
    }
    // Rule 3: an import waiting. Put off with the stick's vault locked,
    // it is that vault's Unlock, on the sheet (§4.4).
    if let Some(imp) = app.import.as_ref() {
        if app.import_stick_present() {
            return (
                app.medium.icon(),
                format!("Remove the {} to start the import", app.medium.noun()),
                String::new(),
                crate::boot_import::OPEN,
                true,
            );
        }
        let locked = app
            .import_view()
            .and_then(|v| v.vaults.into_iter().find(|v| v.open.is_none()));
        if let Some(v) = locked {
            return (
                Icon::Lock,
                format!("Unlock {}", v.title),
                format!("Read from {}", imp.label),
                crate::boot_import::OPEN,
                true,
            );
        }
        return (
            Icon::Download,
            format!("Import from {}", imp.label),
            String::new(),
            crate::boot_import::OPEN,
            true,
        );
    }
    // Rule 4: a receipt from this power-on's last stick visit, and
    // nothing loaded.
    let fresh = !app.holds_secret() && app.session.wallets.is_empty();
    if fresh && let Some(r) = app.receipt.as_ref() {
        let names: Vec<&str> = r.files.iter().map(|f| f.name.as_str()).collect();
        return (
            Icon::Done,
            format!("Written to {}", r.label),
            names.join(", "),
            Action::Nav(Screen::Files),
            true,
        );
    }
    // Rule 5: a locked vault file and nothing loaded.
    if fresh {
        let files = app.vault_files();
        if let Some(i) = files.iter().position(|f| f.open.is_none()) {
            let f = &files[i];
            // By its name, with what it held when last seen open (§3.4),
            // when it was.
            let (name, line) = match app.vault_summary(f) {
                Some(s) => (s.name.clone(), s.line()),
                None => (f.name.clone(), format!("{} · in Files", f.name)),
            };
            if app.sticks.is_empty() {
                return (
                    Icon::Lock,
                    format!("Unlock {name}"),
                    line,
                    Action::Vault(V::Open(i)),
                    true,
                );
            }
            return (
                Icon::Lock,
                format!("Unlock {name}"),
                format!("Pull the {} to unlock", app.medium.noun()),
                Action::Nav(Screen::Home),
                false,
            );
        }
    }
    // Rule 6: a vault changed since it was written.
    if let Some(name) = app.vault_changed() {
        return (
            Icon::Export,
            format!("Write {name} to {}", app.medium.a()),
            crate::vaults::Currency::Changed.line(),
            Action::Nav(Screen::Files),
            true,
        );
    }
    // Rule 7: otherwise.
    (
        Icon::Wallet,
        "Make a wallet".to_string(),
        "Single key or multisig".to_string(),
        Action::CreateWallet,
        true,
    )
}

/// The wallet card's backup line (`docs/SIMPLIFY.md` §5.2) for loaded
/// wallet `i`, which opens Backups on it. Returns the height it took.
pub(crate) fn backup_line(app: &Faraday, ui: &mut Ui, x: f32, y: f32, w: f32, i: usize) -> f32 {
    let line = app.backup_line(i);
    let action = Action::BackupsOf(i);
    let tone = if ui.is_pressed(action) {
        ACCENT
    } else if line.starts_with("Backup:") {
        MUTED
    } else {
        WARN
    };
    let h = ui.wrap(x, y, w, 13.0, W::R, tone, &line);
    ui.hit(x, y - 6.0, w, h + 12.0, action);
    h + 16.0
}

/// Home's secondary tiles: the first two of §1.2's list that apply.
pub(crate) fn home_secondaries(app: &Faraday) -> Vec<(Icon, String, String, Action)> {
    let wallets = app.session.wallets.len();
    let wallets_tile = (
        Icon::Wallet,
        "Wallets".to_string(),
        if wallets == 0 {
            "Create or restore".to_string()
        } else {
            format!(
                "{wallets} {}",
                if wallets == 1 { "wallet" } else { "wallets" }
            )
        },
        // Nothing loaded: the three-ways-in start page, not the
        // wallet card with nothing to show.
        Action::Nav(if wallets == 0 {
            Screen::Start
        } else {
            Screen::Wallets
        }),
    );
    let known = app.backups();
    let second = if !app.sticks.is_empty() && !app.holds_secret() {
        (
            app.medium.icon(),
            app.medium.visit().to_string(),
            "A stick attached".to_string(),
            Action::Nav(Screen::Visit),
        )
    } else if !known.is_empty() {
        // Backups once any wallet is known (§5).
        let without = known.iter().filter(|e| e.lines.is_none()).count();
        (
            Icon::Shield,
            "Backups".to_string(),
            if without == 0 {
                format!("{} with a plan", known.len())
            } else {
                format!("{without} not backed up")
            },
            Action::Nav(Screen::Backups),
        )
    } else {
        let vault_files = app.vault_files().len();
        let open = app.vaults.open.len();
        (
            Icon::Lock,
            "Vaults".to_string(),
            match (open, vault_files) {
                (0, 0) => "None yet".to_string(),
                (0, n) => format!("{n} locked"),
                (o, _) => format!("{o} open"),
            },
            Action::Nav(Screen::Vaults),
        )
    };
    vec![wallets_tile, second]
}

fn home(app: &Faraday, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    let x = x0 + 56.0;
    let width = cw - 112.0;
    let mut y = 44.0;
    title(ui, x, y, "Home");
    network_pill(app, ui, x + width, y + 2.0);
    y += 64.0;

    section_label(ui, x, y, "Start");
    y += 30.0;

    let gap = 16.0;
    // The lead: full width, the first job of §1.2's ranking that applies.
    let (icon, label, sub, action, enabled) = home_lead(app);
    let lead_h = 96.0;
    let pressed = enabled && ui.is_pressed(action);
    ui.fill(
        x,
        y,
        width,
        lead_h,
        14.0,
        if pressed { INNER } else { SURFACE },
    );
    ui.stroke(
        x,
        y,
        width,
        lead_h,
        14.0,
        if enabled { LINE } else { WARN.with_alpha(110) },
    );
    let (ibg, ifg) = if enabled {
        (ACCENT.with_alpha(26), ACCENT)
    } else {
        (INNER, DIM)
    };
    ui.fill(x + 20.0, y + 24.0, 48.0, 48.0, 12.0, ibg);
    ui.icon(x + 20.0, y + 24.0, 48.0, icon, 20.0, ifg);
    let label = ui.fit(19.0, W::S, &label, width - 184.0);
    ui.text(x + 86.0, y + 22.0, 19.0, W::S, TEXT, &label);
    if !sub.is_empty() {
        let sub = ui.fit(14.0, W::R, &sub, width - 184.0);
        ui.text(
            x + 86.0,
            y + 54.0,
            14.0,
            W::R,
            if enabled { MUTED } else { WARN },
            &sub,
        );
    }
    if enabled {
        ui.hit(x, y, width, lead_h, action);
    }
    y += lead_h + gap;

    // At most two secondary tiles, half width each.
    let secondaries = home_secondaries(app);
    let tilew = (width - gap) / 2.0;
    let tile_h = 84.0;
    for (i, (icon, label, sub, action)) in secondaries.iter().enumerate() {
        let tx = x + i as f32 * (tilew + gap);
        let pressed = ui.is_pressed(*action);
        ui.fill(
            tx,
            y,
            tilew,
            tile_h,
            12.0,
            if pressed { INNER } else { SURFACE },
        );
        ui.stroke(tx, y, tilew, tile_h, 12.0, LINE);
        ui.fill(tx + 18.0, y + 22.0, 40.0, 40.0, 10.0, ACCENT.with_alpha(26));
        ui.icon(tx + 18.0, y + 22.0, 40.0, *icon, 17.0, ACCENT);
        let label = ui.fit(15.0, W::S, label, tilew - 96.0);
        ui.text(tx + 74.0, y + 20.0, 15.0, W::S, TEXT, &label);
        let sub = ui.fit(13.0, W::R, sub, tilew - 96.0);
        ui.text(tx + 74.0, y + 46.0, 13.0, W::R, MUTED, &sub);
        ui.hit(tx, y, tilew, tile_h, *action);
    }

    // Scan floats at the foot, on the right.
    ui.fab(x + width, h - 32.0, Icon::Scan, "Scan", Action::Scan);
}

// ---------------------------------------------------------------------
// Wallets: the start page
// ---------------------------------------------------------------------

fn start(app: &Faraday, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    let compact = ui.compact;
    let (x, width) = if compact {
        (x0 + crate::compact::M, cw - 2.0 * crate::compact::M)
    } else {
        (x0 + 48.0, cw - 96.0)
    };
    let mut y = if compact { 14.0 } else { 36.0 } - app.list_offset;
    if !compact {
        title(ui, x, y, "Wallets");
        network_pill(app, ui, x + width, y + 2.0);
        y += 60.0;
    }
    // No wallet and no vault yet: two ways in.
    let empty = app.session.wallets.is_empty()
        && app.vaults.open.is_empty()
        && !app.create_unfinished()
        && app.spend.is_none();
    if app.guided && !compact {
        let text = if empty {
            format!(
                "No wallet is loaded. Create one with new keys or keys you load here, or restore one \
                 from its backups: {}, a vault or paper.",
                app.medium.a()
            )
        } else {
            "Start from what you have in front of you. Every way in ends at the same place: the wallet, \
             with each of its keys marked as here, elsewhere or missing, and the one thing still to do."
                .to_string()
        };
        let used = ui.wrap(x, y, width.min(820.0), 14.0, W::R, MUTED, &text);
        y += used + 20.0;
    }
    section_label(ui, x, y, "Your wallets");
    y += if compact { 24.0 } else { 30.0 };
    y += wallet_rows(app, ui, x, y, width, 2, &Action::OpenWallet) + 18.0;
    // On the small panel this is the wallet list: a key in no wallet is
    // listed after the wallets with its way on, as on the desktop's
    // Wallets column. With no wallet loaded the wallet page says it.
    let (loose, _) = app.loose_keys();
    if compact && !app.session.wallets.is_empty() && !loose.is_empty() {
        section_label(ui, x, y, "Keys without a wallet");
        y += 24.0;
        for fp in &loose {
            let line = key_line(*fp, app.session.key_label(*fp).unwrap_or(""));
            let line = ui.fit(14.0, W::M, &line, width);
            ui.text_mid(x, y, 30.0, 14.0, W::M, TEXT, &line);
            y += 34.0;
            ui.button(
                x,
                y,
                Some(width),
                44.0,
                FROM_KEY,
                Style::Secondary,
                Action::KeyWallet(fp.0, 1),
            );
            y += 56.0;
        }
        y += 6.0;
    }
    // Wallets loaded: the list and the wallet card carry every job
    // (Sign a transaction, Back up, Show wallet QR, Sign a message) and
    // Tools has the rest; Add a key stays under the list.
    // `docs/SIMPLIFY.md` §1.3.
    if !empty {
        if compact {
            y += crate::compact_screens::row(
                ui,
                x,
                y,
                width,
                Some(Icon::Keys),
                "Add a key",
                "Type, scan or bring in a seed",
                MUTED,
                Some(Action::Entry(None)),
            );
        } else {
            ui.button(
                x,
                y,
                None,
                40.0,
                "Add a key",
                Style::Secondary,
                Action::Entry(None),
            );
            y += 40.0 + 18.0;
        }
        if compact {
            crate::compact_screens::finish(app, ui, x0, cw, h, y + 16.0 + app.list_offset);
        } else {
            app.content_h.set(y + app.list_offset + 24.0);
            let view = ui.rect(x0, 0.0, cw, h);
            ui.report_scroll(view, app.content_h.get() - h);
        }
        return;
    }
    if empty && compact {
        y += crate::compact_screens::row(
            ui,
            x,
            y,
            width,
            Some(Icon::Flag),
            "Create a wallet",
            "Single key or multisig",
            MUTED,
            Some(Action::CreateWallet),
        );
        y += crate::compact_screens::row(
            ui,
            x,
            y,
            width,
            Some(Icon::Wallet),
            "Load or restore a wallet",
            "From its backups",
            MUTED,
            Some(Action::RestoreWallet),
        );
        y += crate::compact_screens::row(
            ui,
            x,
            y,
            width,
            Some(Icon::Keys),
            "Add a key",
            "Type, scan or bring in a seed",
            MUTED,
            Some(Action::Entry(None)),
        );
        y += crate::compact_screens::row(
            ui,
            x,
            y,
            width,
            Some(Icon::Sign),
            "Spend from a backup, step by step",
            "A stick, words, or paper",
            MUTED,
            Some(Action::Nav(Screen::Family)),
        );
        y += 8.0;
        let files = app.vault_files();
        match files.iter().position(|f| f.open.is_none()) {
            Some(i) => {
                let line = format!("{} is in Files, locked", files[i].name);
                y += crate::compact_screens::row(
                    ui,
                    x,
                    y,
                    width,
                    Some(Icon::Lock),
                    "Unlock",
                    &line,
                    MUTED,
                    Some(Action::Vault(crate::vaults::VaultAction::Open(i))),
                );
            }
            None => {
                y += ui.wrap(
                    x,
                    y,
                    width,
                    12.0,
                    W::R,
                    MUTED,
                    &format!(
                        "{} with a Faraday vault brings its wallets and keys",
                        app.medium.a_cap()
                    ),
                ) + 12.0;
            }
        }
        crate::compact_screens::finish(app, ui, x0, cw, h, y + app.list_offset);
        return;
    }
    if empty {
        let gap = 16.0;
        // Four ways in (`docs/SIMPLIFY.md` §6.2): Create, Load or
        // restore, Add a key, and Spend from a backup.
        let tw = (width - 3.0 * gap) / 4.0;
        let ways = [
            (
                Icon::Flag,
                "Create a wallet",
                "Single key or multisig",
                Action::CreateWallet,
            ),
            (
                Icon::Wallet,
                "Load or restore a wallet",
                "From its backups",
                Action::RestoreWallet,
            ),
            (
                Icon::Keys,
                "Add a key",
                "Type, scan or bring in a seed",
                Action::Entry(None),
            ),
            (
                Icon::Sign,
                "Spend from a backup, step by step",
                "A stick, words, or paper",
                Action::Nav(Screen::Family),
            ),
        ];
        // A label or sub-line that does not fit at its size breaks onto
        // further lines (`docs/DESIGN.md` "Menu row"): every card in the
        // row grows to the tallest label and sub among them, so the row
        // stays even and nothing is cut off.
        let label_room = tw - 100.0;
        let label_line_h = ui.line(16.0, W::S);
        let sub_line_h = ui.line(13.0, W::R);
        let max_label_lines = ways
            .iter()
            .map(|(_, label, _, _)| ui.wrap_lines(16.0, W::S, label, label_room))
            .max()
            .unwrap_or(1);
        let max_sub_lines = ways
            .iter()
            .map(|(_, _, sub, _)| ui.wrap_lines(13.0, W::R, sub, label_room))
            .max()
            .unwrap_or(1);
        let card_h = 92.0
            + (max_label_lines - 1) as f32 * label_line_h
            + (max_sub_lines - 1) as f32 * sub_line_h;
        for (k, (icon, label, sub, action)) in ways.iter().enumerate() {
            let tx = x + k as f32 * (tw + gap);
            let pressed = ui.is_pressed(*action);
            ui.fill(
                tx,
                y,
                tw,
                card_h,
                12.0,
                if pressed { INNER } else { SURFACE },
            );
            ui.stroke(tx, y, tw, card_h, 12.0, LINE);
            ui.fill(tx + 20.0, y + 24.0, 44.0, 44.0, 10.0, ACCENT.with_alpha(26));
            ui.icon(tx + 20.0, y + 24.0, 44.0, *icon, 18.0, ACCENT);
            if ui.wrap_lines(16.0, W::S, label, label_room) > 1 {
                ui.wrap(tx + 82.0, y + 24.0, label_room, 16.0, W::S, TEXT, label);
            } else {
                let label = ui.fit(16.0, W::S, label, label_room);
                ui.text(tx + 82.0, y + 24.0, 16.0, W::S, TEXT, &label);
            }
            let sub_y = y + 24.0 + max_label_lines as f32 * label_line_h + 2.0;
            if ui.wrap_lines(13.0, W::R, sub, label_room) > 1 {
                ui.wrap(tx + 82.0, sub_y, label_room, 13.0, W::R, MUTED, sub);
            } else {
                let sub = ui.fit(13.0, W::R, sub, label_room);
                ui.text(tx + 82.0, sub_y, 13.0, W::R, MUTED, &sub);
            }
            ui.hit(tx, y, tw, card_h, *action);
        }
        y += card_h + 28.0;
        let files = app.vault_files();
        match files.iter().position(|f| f.open.is_none()) {
            Some(i) => {
                let line = format!("{} is in Files, locked", files[i].name);
                ui.text_mid(x, y, 40.0, 14.0, W::R, MUTED, &line);
                let lw = ui.measure(14.0, W::R, &line);
                ui.button(
                    x + lw + 16.0,
                    y,
                    None,
                    40.0,
                    "Unlock",
                    Style::Secondary,
                    Action::Vault(crate::vaults::VaultAction::Open(i)),
                );
            }
            None => {
                ui.icon(x, y + 10.0, 20.0, app.medium.icon(), 13.0, MUTED);
                ui.text_mid(
                    x + 30.0,
                    y,
                    40.0,
                    14.0,
                    W::R,
                    MUTED,
                    &format!(
                        "{} with a Faraday vault brings its wallets and keys",
                        app.medium.a_cap()
                    ),
                );
            }
        }
    }
}

/// The nonce check: each signature against its key and this
/// transaction, each one made here recomputed from its key, and any nonce
/// used twice. Returns its height.
fn nonce_check(app: &Faraday, ui: &mut Ui, psbt: &osk_psbt::Psbt, x: f32, y: f32, w: f32) -> f32 {
    use osk_psbt::verify::Verdict;
    let Some(s) = app.spend.as_ref() else {
        return 0.0;
    };
    let bytes = psbt.to_bytes();
    let check = {
        let mut cache = s.nonce_check.borrow_mut();
        match cache.as_ref() {
            Some((b, c)) if *b == bytes => c.clone(),
            _ => {
                let c = std::rc::Rc::new(app.session.nonce_check(psbt));
                *cache = Some((bytes, c.clone()));
                c
            }
        }
    };
    let mut cy = y;
    let here = check.rows.iter().filter(|r| r.recomputed.is_some()).count();
    let (head, tone) = if check.sound() {
        (
            format!(
                "Nonce check · {} {} valid · {here} recomputed here",
                check.rows.len(),
                if check.rows.len() == 1 {
                    "signature"
                } else {
                    "signatures"
                }
            ),
            OK,
        )
    } else {
        ("Nonce check · a signature is wrong".to_string(), ERR)
    };
    ui.icon(
        x,
        cy + 8.0,
        20.0,
        if check.sound() {
            Icon::Success
        } else {
            Icon::Error
        },
        12.0,
        tone,
    );
    if ui.compact {
        // Wrapped beside its icon on a small panel.
        cy += ui
            .wrap(x + 26.0, cy + 8.0, w - 26.0, 14.0, W::S, tone, &head)
            .max(20.0)
            + 16.0;
    } else {
        ui.text_mid(x + 26.0, cy, 36.0, 14.0, W::S, tone, &head);
        cy += 40.0;
    }
    for reuse in &check.reused {
        let inputs: Vec<String> = reuse.inputs.iter().map(|i| i.to_string()).collect();
        let line = format!(
            "{}: one nonce on inputs {} · this gives the key away",
            reuse.name(),
            inputs.join(", ")
        );
        cy += ui.wrap(x, cy, w, 13.0, W::S, ERR, &line) + 8.0;
    }
    const SHOWN: usize = 12;
    for r in check.rows.iter().take(SHOWN) {
        ui.text_mid(x, cy, 30.0, 12.0, W::R, DIM, &format!("Input {}", r.input));
        ui.text_mid(x + 70.0, cy, 30.0, 13.0, W::M, TEXT, &r.name);
        let (t, c) = match (r.verdict, r.recomputed) {
            (Verdict::Invalid, _) => (
                "Invalid: not this key's over this transaction".to_string(),
                ERR,
            ),
            (Verdict::Unchecked(why), _) => (format!("Not checked · {why}"), DIM),
            (Verdict::Valid, Some(Some(mode))) => (format!("Valid · nonce recomputed, {mode}"), OK),
            (Verdict::Valid, Some(None)) => (
                "Valid · not the nonce this key makes: do not broadcast".to_string(),
                ERR,
            ),
            (Verdict::Valid, None) => (
                "Valid · signed elsewhere, its nonce needs that device's key".to_string(),
                MUTED,
            ),
        };
        if ui.compact {
            // The verdict in full, under the input.
            let th = ui.wrap(x, cy + 28.0, w, 12.0, W::R, c, &t);
            ui.rule(x, cy + 34.0 + th, w, INNER);
            cy += th + 36.0;
            continue;
        }
        let t = ui.fit(12.0, W::R, &t, w - 170.0);
        ui.text_right(x + w, cy, 30.0, 12.0, W::R, c, &t);
        ui.rule(x, cy + 30.0, w, INNER);
        cy += 32.0;
    }
    if check.rows.len() > SHOWN {
        let rest = &check.rows[SHOWN..];
        let ok = rest
            .iter()
            .all(|r| !r.verdict.is_invalid() && !matches!(r.recomputed, Some(None)));
        ui.text_mid(
            x,
            cy,
            30.0,
            12.0,
            W::R,
            if ok { MUTED } else { ERR },
            &format!(
                "{} more {}",
                rest.len(),
                if ok {
                    "· none wrong"
                } else {
                    "· one is wrong"
                }
            ),
        );
        cy += 32.0;
    }
    cy + 12.0 - y
}

/// What the Inbox gives, put together: its wallets, once each, with the
/// seeds here that are their keys; seeds no wallet names, each a potential
/// wallet; and backups short of parts or sealed. Nothing when it gives
/// nothing to load. Returns its height.
fn inbox_panel(app: &Faraday, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    if ui.compact {
        return inbox_panel_compact(app, ui, x, y, w);
    }
    let found = app.inbox_found();
    let wallets = found.to_load();
    let potential: Vec<&crate::inbox::FoundSeed> = found.potential();
    if wallets.is_empty()
        && potential.is_empty()
        && found.waiting.is_empty()
        && found.xpubs.is_empty()
    {
        return 0.0;
    }
    // Three rows a group at most, so the Inbox and Outbox keep their room.
    const ROWS: usize = 3;
    let wrows = wallets.len().min(ROWS);
    let prows = potential.len().min(ROWS);
    let qrows = found.waiting.len().min(ROWS);
    let xrows = found.xpubs.len().min(ROWS);
    let mut h = 52.0;
    if !wallets.is_empty() {
        h += 26.0 + wrows as f32 * 38.0 + if wallets.len() > ROWS { 22.0 } else { 0.0 } + 54.0;
    }
    if !potential.is_empty() {
        h += 26.0 + prows as f32 * 42.0 + if potential.len() > ROWS { 22.0 } else { 0.0 } + 10.0;
    }
    if !found.xpubs.is_empty() {
        h += 26.0 + xrows as f32 * 42.0 + 10.0;
    }
    if !found.waiting.is_empty() {
        h += 26.0 + qrows as f32 * 40.0 + 6.0;
    }
    ui.card(x, y, w, h, LINE);
    ui.icon(x + 20.0, y + 18.0, 20.0, Icon::Download, 13.0, ACCENT);
    ui.text_mid(
        x + 48.0,
        y + 14.0,
        28.0,
        15.0,
        W::S,
        TEXT,
        app.medium.from_box(),
    );
    let mut ty = y + 52.0;
    let may = app.may_load_keys();
    if !wallets.is_empty() {
        ui.text(x + 20.0, ty, 13.0, W::S, MUTED, "Wallets");
        ty += 26.0;
        let many = wallets.len() > 1;
        for (i, fw) in wallets.iter().enumerate().take(ROWS) {
            let chosen = !app.inbox_skip.contains(&fw.descriptor);
            let mut rx = x + 20.0;
            if many {
                ui.checkbox(rx, ty + 8.0, chosen, true);
                ui.hit(x + 16.0, ty, w - 32.0, 34.0, Action::InboxChoose(i));
                rx += 28.0;
            }
            ui.icon(rx, ty + 7.0, 20.0, Icon::Wallet, 11.0, MUTED);
            let nw = ui.text_mid(rx + 26.0, ty, 34.0, 14.0, W::S, TEXT, &fw.name);
            let n = fw.keys.len().max(1);
            let line = format!(
                "{} · {} of {n} {} here{}",
                fw.shape,
                fw.seeds_here,
                if n == 1 { "seed" } else { "seeds" },
                if fw.files.len() > 1 {
                    format!(" · {} files say it", fw.files.len())
                } else if fw.from_shares {
                    " · from split sheets".to_string()
                } else {
                    String::new()
                }
            );
            let line = ui.fit(12.0, W::R, &line, w - nw - 260.0);
            ui.text_mid(rx + 40.0 + nw, ty, 34.0, 12.0, W::R, MUTED, &line);
            if fw.seeds_here >= fw.needed {
                let tag = "Ready to sign once loaded";
                let tw = ui.measure(12.0, W::S, tag) + 20.0;
                ui.fill(
                    x + w - 20.0 - tw,
                    ty + 5.0,
                    tw,
                    24.0,
                    12.0,
                    OK.with_alpha(30),
                );
                ui.text_mid(x + w - 10.0 - tw, ty, 34.0, 12.0, W::S, OK, tag);
            }
            ty += 38.0;
        }
        if wallets.len() > ROWS {
            ui.text_mid(
                x + 20.0,
                ty - 6.0,
                22.0,
                12.0,
                W::R,
                MUTED,
                &format!("+ {} more, loaded with these", wallets.len() - ROWS),
            );
            ty += 22.0;
        }
        let chosen = wallets
            .iter()
            .filter(|w| !many || !app.inbox_skip.contains(&w.descriptor))
            .count();
        let with_seeds = wallets
            .iter()
            .filter(|w| !many || !app.inbox_skip.contains(&w.descriptor))
            .any(|w| w.seeds_here > 0);
        let label = match (chosen, with_seeds) {
            (0, _) => "Nothing chosen".to_string(),
            (1, true) => "Load the wallet with its seeds".to_string(),
            (1, false) => "Load the wallet".to_string(),
            (n, true) => format!("Load {n} wallets with their seeds"),
            (n, false) => format!("Load {n} wallets"),
        };
        ui.button(
            x + 20.0,
            ty + 4.0,
            None,
            40.0,
            &label,
            if chosen > 0 {
                Style::Primary
            } else {
                Style::Disabled
            },
            Action::InboxLoad,
        );
        if !may {
            ui.text_mid(
                x + 20.0 + ui.measure(14.0, W::S, &label) + 52.0,
                ty + 4.0,
                40.0,
                12.0,
                W::R,
                WARN,
                &format!("Seeds load once the {} is out", app.medium.noun()),
            );
        }
        ty += 54.0;
    }
    if !potential.is_empty() {
        ui.text(x + 20.0, ty, 13.0, W::S, MUTED, "Seeds with no wallet");
        ty += 26.0;
        for s in potential.iter().take(ROWS) {
            ui.icon(x + 20.0, ty + 9.0, 20.0, Icon::Keys, 11.0, MUTED);
            ui.text_mid(
                x + 46.0,
                ty,
                38.0,
                14.0,
                W::M,
                TEXT,
                &fp_text(s.fingerprint),
            );
            let from: Vec<String> = s.sources.iter().map(|src| src.name(app)).collect();
            let line = ui.fit(12.0, W::R, &from.join(" · "), w - 420.0);
            ui.text_mid(x + 140.0, ty, 38.0, 12.0, W::R, MUTED, &line);
            let label = "Make it a wallet";
            let bw = ui.measure(13.0, W::S, label) + 28.0;
            ui.button(
                x + w - 20.0 - bw,
                ty + 3.0,
                Some(bw),
                32.0,
                label,
                if may {
                    Style::Secondary
                } else {
                    Style::Disabled
                },
                Action::PotentialOpen(s.fingerprint.0),
            );
            ty += 42.0;
        }
        if potential.len() > ROWS {
            ui.text_mid(
                x + 20.0,
                ty - 6.0,
                22.0,
                12.0,
                W::R,
                MUTED,
                &format!("+ {} more", potential.len() - ROWS),
            );
            ty += 22.0;
        }
        ty += 10.0;
    }
    if !found.xpubs.is_empty() {
        ui.text(x + 20.0, ty, 13.0, W::S, MUTED, "Xpubs with no wallet");
        ty += 26.0;
        for (i, xp) in found.xpubs.iter().enumerate().take(ROWS) {
            ui.icon(x + 20.0, ty + 9.0, 20.0, Icon::Wallet, 11.0, MUTED);
            let fp = xp
                .fingerprint
                .map(fp_text)
                .unwrap_or_else(|| "no origin".to_string());
            ui.text_mid(x + 46.0, ty, 38.0, 14.0, W::M, TEXT, &fp);
            let line = format!(
                "{} · {}",
                xp.path.clone().unwrap_or_default(),
                xp.files
                    .first()
                    .and_then(|k| app.inbox.get(*k))
                    .map_or(String::new(), |it| it.name.clone())
            );
            let line = ui.fit(12.0, W::R, &line, w - 460.0);
            ui.text_mid(x + 140.0, ty, 38.0, 12.0, W::R, MUTED, &line);
            let label = "Make it a watch-only wallet";
            let bw = ui.measure(13.0, W::S, label) + 28.0;
            ui.button(
                x + w - 20.0 - bw,
                ty + 3.0,
                Some(bw),
                32.0,
                label,
                Style::Secondary,
                Action::XpubOpen(i),
            );
            ty += 42.0;
        }
        ty += 10.0;
    }
    if !found.waiting.is_empty() {
        ui.text(x + 20.0, ty, 13.0, W::S, MUTED, "Not complete or sealed");
        ty += 26.0;
        for q in found.waiting.iter().take(ROWS) {
            let line = ui.fit(13.0, W::R, &q.line, w - 200.0);
            ui.text_mid(x + 20.0, ty, 36.0, 13.0, W::R, MUTED, &line);
            if let Some(k) = q.backup {
                let label = "Open with its passphrase";
                let bw = ui.measure(13.0, W::S, label) + 28.0;
                ui.button(
                    x + w - 20.0 - bw,
                    ty + 2.0,
                    Some(bw),
                    32.0,
                    label,
                    if may {
                        Style::Secondary
                    } else {
                        Style::Disabled
                    },
                    Action::BackupOpen(k),
                );
            }
            ty += 40.0;
        }
    }
    h
}

/// Making a seed from the Inbox a wallet: a passphrase or none, the fingerprint it gives,
/// the wallet here it opens if any, else the kind of single-key wallet.
/// Or a sealed backup's passphrase.
/// The potential sheet on a small panel: the same fields, one under
/// another.
#[allow(clippy::too_many_arguments)]
fn potential_compact(
    app: &Faraday,
    p: &crate::inbox::Potential,
    ui: &mut Ui,
    w: f32,
    h: f32,
    backup: Option<&crate::Item>,
    seed: Option<&crate::inbox::FoundSeed>,
    xpub: Option<&(String, bool)>,
    takes: bool,
    matched: Option<&str>,
) {
    let title = match (backup, xpub, p.fingerprint) {
        (Some(item), _, _) => format!("Open {}", item.name),
        (None, Some(_), Some(fp)) => format!("A watch-only wallet from {}", fp_text(fp)),
        (None, Some(_), None) => "A watch-only wallet from an xpub".to_string(),
        (None, None, Some(fp)) => format!("A wallet from {}", fp_text(fp)),
        _ => String::new(),
    };
    let kinds = |ui: &mut Ui, x: f32, y: f32, w: f32| -> f32 {
        let row: Vec<(String, Style, Action)> = crate::inbox::KINDS
            .iter()
            .enumerate()
            .map(|(k, kind)| {
                (
                    crate::family::kind_name(*kind).to_string(),
                    if *kind == p.kind {
                        Style::Primary
                    } else {
                        Style::Secondary
                    },
                    Action::PotentialKind(k as u8),
                )
            })
            .collect();
        button_rows(ui, x, y, w, &row) - 4.0
    };
    crate::compact::sheet(ui, w, h, &mut |ui, x, y, iw| {
        let mut cy = y;
        cy += crate::compact::sheet_head(ui, x, cy, iw, Icon::Keys, ACCENT, &title);
        if let Some(s) = seed {
            let from: Vec<String> = s.sources.iter().map(|src| src.name(app)).collect();
            cy += ui.wrap(
                x,
                cy,
                iw,
                12.0,
                W::R,
                MUTED,
                &format!("From {}", from.join(" · ")),
            ) + 10.0;
        }
        if takes {
            let label = match (backup.is_some(), seed.and_then(|s| s.sources.first())) {
                (true, _) => "The backup's passphrase",
                (false, Some(crate::inbox::SeedSource::Slip39(_))) => {
                    "SLIP-39 passphrase · none if left empty"
                }
                _ => "BIP-39 passphrase · none if left empty",
            };
            cy += ui.wrap(x, cy, iw, 13.0, W::S, TEXT, label) + 6.0;
            if app.may_load_keys() {
                let shown = if p.shown {
                    p.passphrase.to_string()
                } else {
                    "•".repeat(p.passphrase.chars().count().min(48))
                };
                ui.fill(x, cy, iw, 44.0, 8.0, BG);
                ui.stroke(x, cy, iw, 44.0, 8.0, if p.typing { ACCENT } else { BORDER });
                let shown = ui.fit(14.0, W::M, &shown, iw - 60.0);
                let tw = ui.text_mid(x + 12.0, cy, 44.0, 14.0, W::M, TEXT, &shown);
                if p.typing {
                    ui.caret(x + 13.0 + tw, cy + 13.0, 18.0);
                }
                ui.hit(x, cy, iw - 44.0, 44.0, Action::PotentialType);
                ui.icon(
                    x + iw - 44.0,
                    cy,
                    44.0,
                    if p.shown { Icon::EyeOff } else { Icon::Eye },
                    15.0,
                    if p.shown { ACCENT } else { MUTED },
                );
                ui.hit(x + iw - 44.0, cy, 44.0, 44.0, Action::PotentialShow);
            } else {
                crate::vault_screens::stick_field(app.medium, ui, x, cy, iw);
            }
            cy += 56.0;
        }
        if let Some((key, fixed)) = xpub {
            let k = ui.fit(12.0, W::M, key, iw);
            ui.text(x, cy, 12.0, W::M, MUTED, &k);
            cy += 24.0;
            if *fixed {
                let line = format!("{} · as its path says", crate::family::kind_name(p.kind));
                cy += ui.wrap(x, cy, iw, 14.0, W::S, TEXT, &line) + 10.0;
            } else {
                cy += kinds(ui, x, cy, iw);
            }
        } else if backup.is_none() {
            if let Some(with) = p.with {
                ui.text(x, cy, 12.0, W::R, MUTED, "Fingerprint");
                ui.text(x + 90.0, cy, 13.0, W::M, TEXT, &fp_text(with));
                cy += 24.0;
            }
            match matched {
                Some(name) => {
                    let line = format!("A key of {name}: it loads with that wallet");
                    cy += ui.wrap(x, cy, iw, 13.0, W::S, OK, &line) + 10.0;
                }
                None => {
                    ui.text(x, cy, 12.0, W::R, MUTED, "Single-key wallet");
                    cy += 22.0;
                    cy += kinds(ui, x, cy, iw);
                }
            }
        }
        if let Some(e) = &p.error {
            cy += ui.wrap(x, cy, iw, 13.0, W::R, ERR, e) + 8.0;
        }
        let go = match (backup.is_some(), matched) {
            (true, _) => "Open it".to_string(),
            (false, Some(name)) if xpub.is_none() => format!("Load {name}"),
            _ => "Make the wallet".to_string(),
        };
        cy += crate::compact::buttons(
            ui,
            x,
            cy,
            iw,
            &[
                ("Cancel", Style::Secondary, Action::Cancel),
                (
                    &go,
                    if app.may_load_keys() || xpub.is_some() {
                        Style::Primary
                    } else {
                        Style::Disabled
                    },
                    Action::PotentialMake,
                ),
            ],
        );
        cy - y
    });
}

fn potential_sheet(app: &Faraday, ui: &mut Ui, w: f32, h: f32) {
    let Some(p) = app.potential.as_ref() else {
        return;
    };
    let sw = 620.0;
    let backup = p.backup.and_then(|k| app.inbox.get(k));
    let seed = p.fingerprint.and_then(|fp| {
        app.inbox_found()
            .seeds
            .into_iter()
            .find(|s| s.fingerprint == fp)
    });
    let xpub = p.xpub.as_ref();
    let takes = xpub.is_none()
        && seed
            .as_ref()
            .is_none_or(|s| s.sources.first().is_none_or(|src| src.takes_passphrase()));
    let matched = app.potential_match();
    if ui.compact {
        potential_compact(
            app,
            p,
            ui,
            w,
            h,
            backup,
            seed.as_ref(),
            xpub,
            takes,
            matched.as_deref(),
        );
        return;
    }
    let sh = if backup.is_some() {
        300.0
    } else if xpub.is_some() {
        370.0
    } else if takes {
        440.0
    } else {
        330.0
    };
    let (x, y) = sheet_box(ui, w, h, sw, sh);
    let ix = x + 32.0;
    let iw = sw - 64.0;
    let mut cy = y + 30.0;
    let title_text = match (backup, xpub, p.fingerprint) {
        (Some(item), _, _) => format!("Open {}", item.name),
        (None, Some(_), Some(fp)) => format!("A watch-only wallet from {}", fp_text(fp)),
        (None, Some(_), None) => "A watch-only wallet from an xpub".to_string(),
        (None, None, Some(fp)) => format!("A wallet from {}", fp_text(fp)),
        _ => String::new(),
    };
    ui.icon(ix, cy, 30.0, Icon::Keys, 18.0, ACCENT);
    let tt = ui.fit(20.0, W::S, &title_text, iw - 42.0);
    ui.text_mid(ix + 42.0, cy, 30.0, 20.0, W::S, TEXT, &tt);
    cy += 46.0;
    if let Some(s) = &seed {
        let from: Vec<String> = s.sources.iter().map(|src| src.name(app)).collect();
        let line = ui.fit(13.0, W::R, &format!("From {}", from.join(" · ")), iw);
        ui.text(ix, cy, 13.0, W::R, MUTED, &line);
        cy += 30.0;
    }
    if takes {
        let label = match (
            backup.is_some(),
            seed.as_ref().and_then(|s| s.sources.first()),
        ) {
            (true, _) => "The backup's passphrase",
            (false, Some(crate::inbox::SeedSource::Slip39(_))) => {
                "SLIP-39 passphrase · none if left empty"
            }
            _ => "BIP-39 passphrase · none if left empty",
        };
        ui.text(ix, cy, 13.0, W::S, TEXT, label);
        cy += 24.0;
        if app.may_load_keys() {
            let shown = if p.shown {
                p.passphrase.to_string()
            } else {
                "•".repeat(p.passphrase.chars().count().min(48))
            };
            ui.fill(ix, cy, iw, 40.0, 8.0, BG);
            ui.stroke(
                ix,
                cy,
                iw,
                40.0,
                8.0,
                if p.typing { ACCENT } else { BORDER },
            );
            let shown = ui.fit(14.0, W::M, &shown, iw - 60.0);
            let tw = ui.text_mid(ix + 12.0, cy, 40.0, 14.0, W::M, TEXT, &shown);
            if p.typing {
                ui.caret(ix + 13.0 + tw, cy + 11.0, 18.0);
            }
            ui.hit(ix, cy, iw - 44.0, 40.0, Action::PotentialType);
            ui.icon(
                ix + iw - 40.0,
                cy,
                40.0,
                if p.shown { Icon::EyeOff } else { Icon::Eye },
                15.0,
                if p.shown { ACCENT } else { MUTED },
            );
            ui.hit(ix + iw - 40.0, cy, 40.0, 40.0, Action::PotentialShow);
        } else {
            crate::vault_screens::stick_field(app.medium, ui, ix, cy, iw);
        }
        cy += 52.0;
    }
    if let Some((key, fixed)) = xpub {
        let k = ui.fit(12.0, W::M, key, iw);
        ui.text(ix, cy, 12.0, W::M, MUTED, &k);
        cy += 30.0;
        ui.text_mid(ix, cy, 30.0, 13.0, W::R, MUTED, "Single-key wallet");
        cy += 32.0;
        if *fixed {
            ui.text_mid(
                ix,
                cy,
                34.0,
                14.0,
                W::S,
                TEXT,
                &format!("{} · as its path says", crate::family::kind_name(p.kind)),
            );
        } else {
            let mut bx = ix;
            for (k, kind) in crate::inbox::KINDS.iter().enumerate() {
                let style = if *kind == p.kind {
                    Style::Primary
                } else {
                    Style::Secondary
                };
                bx += ui.button(
                    bx,
                    cy,
                    None,
                    34.0,
                    crate::family::kind_name(*kind),
                    style,
                    Action::PotentialKind(k as u8),
                ) + 8.0;
            }
        }
        cy += 46.0;
    } else if backup.is_none() {
        if let Some(with) = p.with {
            ui.text_mid(ix, cy, 24.0, 13.0, W::R, MUTED, "Fingerprint");
            ui.text_mid(ix + 110.0, cy, 24.0, 14.0, W::M, TEXT, &fp_text(with));
            cy += 30.0;
        }
        match &matched {
            Some(name) => {
                ui.text_mid(
                    ix,
                    cy,
                    30.0,
                    13.0,
                    W::S,
                    OK,
                    &format!("A key of {name}: it loads with that wallet"),
                );
                cy += 40.0;
            }
            None => {
                ui.text_mid(ix, cy, 30.0, 13.0, W::R, MUTED, "Single-key wallet");
                cy += 32.0;
                let mut bx = ix;
                for (k, kind) in crate::inbox::KINDS.iter().enumerate() {
                    let style = if *kind == p.kind {
                        Style::Primary
                    } else {
                        Style::Secondary
                    };
                    bx += ui.button(
                        bx,
                        cy,
                        None,
                        34.0,
                        crate::family::kind_name(*kind),
                        style,
                        Action::PotentialKind(k as u8),
                    ) + 8.0;
                }
                cy += 46.0;
            }
        }
    }
    if let Some(e) = &p.error {
        let e = ui.fit(13.0, W::R, e, iw);
        ui.text(ix, cy, 13.0, W::R, ERR, &e);
    }
    let by = y + sh - 32.0 - 46.0;
    let bw = (iw - 12.0) / 2.0;
    ui.button(
        ix,
        by,
        Some(bw),
        46.0,
        "Cancel",
        Style::Secondary,
        Action::Cancel,
    );
    let go = match (backup.is_some(), &matched) {
        (true, _) => "Open it".to_string(),
        (false, Some(name)) if xpub.is_none() => format!("Load {name}"),
        _ => "Make the wallet".to_string(),
    };
    ui.button(
        ix + bw + 12.0,
        by,
        Some(bw),
        46.0,
        &go,
        if app.may_load_keys() || xpub.is_some() {
            Style::Primary
        } else {
            Style::Disabled
        },
        Action::PotentialMake,
    );
}

/// Tools: every flow by group, three tiles to a row, each with its
/// standards; Find a tool narrows them as it is typed.
fn catalog_screen(app: &Faraday, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    use crate::catalog::{GROUPS, TILES, matches};
    // A small panel has the name in its bar, Find across the panel, and a
    // tile a row.
    let compact = ui.compact;
    let (x, width) = if compact {
        (x0 + crate::compact::M, cw - 2.0 * crate::compact::M)
    } else {
        (x0 + 48.0, cw - 96.0)
    };
    let clip = ui.rect(x0, 0.0, cw, h);
    ui.c.push_clip(clip);
    let top = if compact { 14.0 } else { 36.0 } - app.list_offset;
    let mut y = top;
    if !compact {
        title(ui, x, y, "Tools");
    }
    // Find a tool: typing on this page goes here.
    let fw = if compact {
        width
    } else {
        320.0f32.min(width * 0.4)
    };
    let fx = x + width - fw;
    ui.fill(fx, y - 2.0, fw, 40.0, 8.0, BG);
    ui.stroke(fx, y - 2.0, fw, 40.0, 8.0, ACCENT.with_alpha(110));
    let (shown, tone) = if app.catalog_find.is_empty() {
        ("Find a tool: a name, a BIP number".to_string(), DIM)
    } else {
        (app.catalog_find.clone(), TEXT)
    };
    let shown = ui.fit(14.0, W::R, &shown, fw - 28.0);
    let tw = ui.text_mid(fx + 12.0, y - 2.0, 40.0, 14.0, W::R, tone, &shown);
    // Typing on this page always lands here, with no click to focus it
    // first, so the caret is drawn whether or not anything is typed yet:
    // the one sign that this box is already live.
    let caret_x = if app.catalog_find.is_empty() {
        fx + 12.0
    } else {
        fx + 14.0 + tw
    };
    ui.caret(caret_x, y + 9.0, 18.0);
    y += if compact { 54.0 } else { 64.0 };
    let gap = 14.0;
    let per = if compact { 1 } else { 3 };
    let tw3 = (width - (per as f32 - 1.0) * gap) / per as f32;
    let th = 82.0;
    let mut any = false;
    for group in GROUPS {
        let tiles: Vec<(usize, &crate::catalog::Tile)> = TILES
            .iter()
            .enumerate()
            .filter(|(_, t)| t.group == group && matches(t, &app.catalog_find))
            .collect();
        if tiles.is_empty() {
            continue;
        }
        any = true;
        section_label(ui, x, y, group);
        y += 28.0;
        for (k, (i, tile)) in tiles.iter().enumerate() {
            let tx = x + (k % per) as f32 * (tw3 + gap);
            let ty = y + (k / per) as f32 * (th + 10.0);
            let need = app.tile_need(tile.go);
            let opens = app.tile_opens(tile.go);
            let action = Action::Catalog(*i as u8);
            let pressed = ui.is_pressed(action);
            ui.fill(tx, ty, tw3, th, 10.0, if pressed { INNER } else { SURFACE });
            ui.stroke(tx, ty, tw3, th, 10.0, LINE);
            let name = ui.fit(14.0, W::S, tile.name, tw3 - 32.0);
            ui.text(
                tx + 16.0,
                ty + 12.0,
                14.0,
                W::S,
                if opens { TEXT } else { DIM },
                &name,
            );
            let line = ui.fit(12.0, W::R, need.unwrap_or(tile.line), tw3 - 32.0);
            ui.text(
                tx + 16.0,
                ty + 34.0,
                12.0,
                W::R,
                if need.is_some() { WARN } else { MUTED },
                &line,
            );
            // The standards, as small tags.
            let mut gx = tx + 16.0;
            for tag in tile.tags {
                let gw = ui.measure(11.0, W::M, tag) + 14.0;
                if gx + gw > tx + tw3 - 12.0 {
                    break;
                }
                ui.fill(gx, ty + 56.0, gw, 18.0, 9.0, INNER);
                ui.text_mid(gx + 7.0, ty + 56.0, 18.0, 11.0, W::M, MUTED, tag);
                gx += gw + 6.0;
            }
            if opens {
                ui.hit(tx, ty, tw3, th, action);
            }
        }
        y += tiles.len().div_ceil(per) as f32 * (th + 10.0) + 16.0;
    }
    if !any {
        ui.text(x, y, 14.0, W::R, MUTED, "No tool matches");
        y += 30.0;
    }
    ui.c.pop_clip();
    app.content_h.set(y - top + 40.0);
    ui.report_scroll(clip, app.content_h.get() - h);
}

/// Hex in groups of sixteen, so a long run wraps.
pub(crate) fn spaced(hex: &str) -> String {
    hex.as_bytes()
        .chunks(16)
        .map(|c| String::from_utf8_lossy(c).into_owned())
        .collect::<Vec<_>>()
        .join(" ")
}

/// A transaction taken apart: what it is, what goes in, what comes out
/// and to whom, and its bytes.
/// What the way back from Decode a transaction is called.
pub(crate) fn decode_back(back: Screen) -> &'static str {
    match back {
        Screen::Spend => "Sign a transaction",
        Screen::Family => "Spend",
        Screen::Files => "Files",
        Screen::Start => "Wallets",
        Screen::Catalog => "Tools",
        _ => "Back",
    }
}

fn decode_screen(app: &Faraday, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    use crate::decode::btc;
    let Some(d) = app.decode.as_ref() else {
        return;
    };
    let t = &d.decoded;
    // A small panel has the way back in its bar, and one column: each
    // value under its name.
    let compact = ui.compact;
    let (x, w) = if compact {
        (x0 + crate::compact::M, cw - 2.0 * crate::compact::M)
    } else {
        (x0 + 48.0, (cw - 96.0).min(960.0))
    };
    let clip = ui.rect(x0, 0.0, cw, h);
    ui.c.push_clip(clip);
    // Scrolled as far as the end it last drew to (`content_h`).
    let top = if compact { 14.0 } else { 28.0 } - app.list_offset;
    let mut y = top;
    if !compact {
        ui.icon(x - 4.0, y, 16.0, Icon::ChevronLeft, 10.0, MUTED);
        let lw = ui.text(x + 14.0, y, 12.0, W::R, MUTED, decode_back(d.back));
        ui.hit(x - 4.0, y - 4.0, lw + 30.0, 24.0, Action::Nav(d.back));
        y += 22.0;
        title(ui, x, y, "Decode a transaction");
        y += 44.0;
    }
    y += ui.wrap(x, y, w, 13.0, W::R, MUTED, &t.source) + 14.0;

    let fee = match t.fee {
        Some(f) => format!("{f} sat · {:.1} sat/vB", f as f64 / t.size.1.max(1) as f64),
        None => "Not stated: the amounts going in are not here".to_string(),
    };
    let facts: [(&str, String, bool); 6] = [
        ("Transaction id", t.txid.clone(), true),
        ("Witness id", t.wtxid.clone(), true),
        ("Version", t.version.to_string(), false),
        ("Locktime", t.locktime.clone(), false),
        (
            "Size",
            format!("{} bytes · {} vB · {} WU", t.size.0, t.size.1, t.size.2),
            false,
        ),
        ("Fee", fee, false),
    ];
    for (k, v, mono) in facts.iter() {
        let face = if *mono { W::M } else { W::R };
        if compact {
            ui.text(x, y, 12.0, W::R, MUTED, k);
            let v = if *mono { grouped(v) } else { v.clone() };
            let vh = ui.wrap(x, y + 18.0, w, 13.0, face, TEXT, &v);
            ui.rule(x, y + vh + 26.0, w, INNER);
            y += vh + 34.0;
            continue;
        }
        ui.text_mid(x, y, 34.0, 13.0, W::R, MUTED, k);
        let v = ui.fit(13.0, face, v, w - 170.0);
        ui.text_mid(x + 160.0, y, 34.0, 13.0, face, TEXT, &v);
        ui.rule(x, y + 34.0, w, INNER);
        y += 36.0;
    }
    y += 18.0;

    section_label(ui, x, y, &format!("Inputs · {}", t.inputs.len()));
    y += 28.0;
    for (i, input) in t.inputs.iter().enumerate() {
        ui.text_mid(x, y, 26.0, 12.0, W::R, DIM, &i.to_string());
        if let Some(a) = input.amount {
            ui.text_right(x + w, y, 26.0, 13.0, W::S, TEXT, &btc(a));
        }
        if compact {
            // The outpoint whole, under its number and amount.
            y += 26.0;
            y += ui.wrap(x + 26.0, y, w - 26.0, 12.0, W::M, TEXT, &input.outpoint) + 4.0;
        } else {
            let op = ui.fit(13.0, W::M, &input.outpoint, w - 220.0);
            ui.text_mid(x + 26.0, y, 26.0, 13.0, W::M, TEXT, &op);
            y += 26.0;
        }
        let wit = if input.witness.is_empty() {
            "No witness".to_string()
        } else {
            let sizes: Vec<String> = input
                .witness
                .iter()
                .map(|h| (h.len() / 2).to_string())
                .collect();
            format!(
                "Witness · {} {} · {} bytes",
                input.witness.len(),
                if input.witness.len() == 1 {
                    "item"
                } else {
                    "items"
                },
                sizes.join(", ")
            )
        };
        let line = format!(
            "Sequence {} · {wit}{}",
            input.sequence,
            if input.script_sig > 0 {
                format!(" · scriptSig {} bytes", input.script_sig)
            } else {
                String::new()
            }
        );
        if compact {
            let lh = ui.wrap(x + 26.0, y + 2.0, w - 26.0, 12.0, W::R, MUTED, &line);
            ui.rule(x, y + lh + 8.0, w, INNER);
            y += lh + 12.0;
            continue;
        }
        let line = ui.fit(12.0, W::R, &line, w - 26.0);
        ui.text_mid(x + 26.0, y, 22.0, 12.0, W::R, MUTED, &line);
        ui.rule(x, y + 26.0, w, INNER);
        y += 30.0;
    }
    y += 18.0;

    section_label(ui, x, y, &format!("Outputs · {}", t.outputs.len()));
    y += 28.0;
    for (i, out) in t.outputs.iter().enumerate() {
        ui.text_mid(x, y, 26.0, 12.0, W::R, DIM, &i.to_string());
        ui.text_right(x + w, y, 26.0, 13.0, W::S, TEXT, &btc(out.amount));
        let (whose, tone) = match &out.ours {
            Some((name, true, n)) => (format!("Change · back to {name} · 1/{n}"), OK),
            Some((name, false, n)) => (format!("Back to {name} · receive 0/{n}"), OK),
            None => ("Pays out of the loaded wallets".to_string(), WARN),
        };
        if compact {
            // The address whole, then its kind, then whose it is.
            y += 26.0;
            let ah = ui.wrap(
                x + 26.0,
                y,
                w - 26.0,
                12.0,
                W::M,
                TEXT,
                &grouped(&app.session.shown(&out.address)),
            );
            y += ah + 4.0;
            ui.text_mid(x + 26.0, y, 22.0, 12.0, W::R, MUTED, out.kind);
            y += 22.0;
            let wh = ui.wrap(x + 26.0, y + 2.0, w - 26.0, 12.0, W::S, tone, &whose);
            ui.rule(x, y + wh + 8.0, w, INNER);
            y += wh + 12.0;
            continue;
        }
        let addr = ui.fit(
            13.0,
            W::M,
            &grouped(&app.session.shown(&out.address)),
            w - 220.0,
        );
        ui.text_mid(x + 26.0, y, 26.0, 13.0, W::M, TEXT, &addr);
        y += 26.0;
        let kw = ui.text_mid(x + 26.0, y, 22.0, 12.0, W::R, MUTED, out.kind);
        ui.text_mid(x + 40.0 + kw, y, 22.0, 12.0, W::S, tone, &whose);
        ui.rule(x, y + 26.0, w, INNER);
        y += 30.0;
    }
    y += 18.0;

    y += wrap_buttons(
        ui,
        x,
        y,
        w,
        38.0,
        &[
            (
                if d.show_hex {
                    "Hide the hex"
                } else {
                    "Show the hex"
                },
                Style::Secondary,
                Action::DecodeHex,
            ),
            ("Show as QR", Style::Secondary, Action::QrDecoded),
        ],
    ) + 4.0;
    if d.show_hex {
        let th = ui.wrap(
            x + 12.0,
            y + 10.0,
            w - 24.0,
            12.0,
            W::M,
            TEXT,
            &spaced(&t.hex),
        );
        ui.stroke(x, y, w, th + 20.0, 8.0, INNER);
        y += th + 32.0;
    }
    ui.c.pop_clip();
    app.content_h.set(y - top + 40.0);
    ui.report_scroll(clip, app.content_h.get() - h);
}

/// Where a wallet's missing keys may be, and the way to look: a locked
/// vault to unlock (coming back to `back`), the open vaults that do not
/// hold them, and the other places a seed is kept. Nothing when an open
/// vault holds one, since its row loads it. Returns its height.
#[allow(clippy::too_many_arguments)]
pub(crate) fn missing_keys_line(
    app: &Faraday,
    ui: &mut Ui,
    slots: &[crate::wallet::Slot],
    back: Screen,
    x: f32,
    y: f32,
    w: f32,
) -> f32 {
    let missing: Vec<osk_bip::keys::Fingerprint> = slots
        .iter()
        .filter(|s| s.held_by.is_none())
        .filter_map(|s| s.fingerprint)
        .collect();
    if missing.iter().any(|f| app.vault_key_for(*f).is_some()) {
        return 0.0;
    }
    let files = app.vault_files();
    let locked = files.iter().position(|f| f.open.is_none());
    let open_names: Vec<String> = app.vaults.open.iter().map(|v| v.name.clone()).collect();
    let line = match (locked, open_names.is_empty()) {
        (Some(i), _) => format!("{} is locked · it may hold them", files[i].name),
        (None, false) => format!(
            "Not in {} · perhaps another vault, a paper backup or a SeedQR",
            open_names.join(", ")
        ),
        (None, true) => format!(
            "A vault on {}, a paper backup or a SeedQR may hold them",
            app.medium.a()
        ),
    };
    if ui.compact {
        ui.icon(x - 4.0, y - 2.0, 20.0, Icon::Info, 11.0, MUTED);
        let mut cy = y + ui.wrap(x + 20.0, y, w - 20.0, 12.0, W::R, MUTED, &line) + 8.0;
        if let Some(i) = locked {
            ui.button(
                x + 20.0,
                cy,
                Some(w - 20.0),
                36.0,
                "Unlock",
                if app.may_load_keys() {
                    Style::Secondary
                } else {
                    Style::Disabled
                },
                Action::Vault(crate::vaults::VaultAction::OpenFrom(i, back)),
            );
            cy += 44.0;
        }
        return cy - y;
    }
    ui.icon(x, y + 10.0, 20.0, Icon::Info, 11.0, MUTED);
    let line = ui.fit(13.0, W::R, &line, w - 140.0);
    let lw = ui.text_mid(x + 26.0, y, 40.0, 13.0, W::R, MUTED, &line);
    if let Some(i) = locked {
        ui.button(
            x + 40.0 + lw,
            y + 4.0,
            None,
            32.0,
            "Unlock",
            if app.may_load_keys() {
                Style::Secondary
            } else {
                Style::Disabled
            },
            Action::Vault(crate::vaults::VaultAction::OpenFrom(i, back)),
        );
    }
    44.0
}

/// The loaded wallets, two to a row, at most `rows` rows: each with its
/// shape and whether the seeds here sign for it. Returns its height.
pub(crate) fn wallet_rows(
    app: &Faraday,
    ui: &mut Ui,
    x: f32,
    y: f32,
    width: f32,
    rows: usize,
    open: &dyn Fn(usize) -> Action,
) -> f32 {
    if app.session.wallets.is_empty() {
        ui.text_mid(x, y, 30.0, 13.0, W::R, DIM, "None loaded");
        return 38.0;
    }
    let total = app.session.wallets.len();
    if ui.compact {
        let mut order: Vec<usize> = (0..total).collect();
        order.sort_by_key(|&i| std::cmp::Reverse(sign_readiness(app, &app.session.wallets[i])));
        let mut cy = y;
        for &i in &order {
            let w = &app.session.wallets[i];
            let ready = sign_readiness(app, w);
            let (m, _) = Session::quorum(w);
            let here = app
                .session
                .slots(w)
                .iter()
                .filter(|s| s.held_by.is_some())
                .count()
                .min(m);
            let (state, tone) = match ready {
                2 => (format!("Ready to sign · {here} of {m}"), OK),
                1 => (format!("{here} of {m} here"), TEXT),
                _ => ("Watch only".to_string(), DIM),
            };
            let sub = format!("{} · {state}", Session::shape(w));
            cy += crate::compact_screens::row(
                ui,
                x,
                cy,
                width,
                None,
                &w.name,
                &sub,
                tone,
                Some(open(i)),
            );
        }
        return cy - y;
    }
    let gap = 16.0;
    let cols = 2;
    let rw = (width - gap) / 2.0;
    // Ready wallets first, as the corner lists them.
    let mut order: Vec<usize> = (0..total).collect();
    order.sort_by_key(|&i| std::cmp::Reverse(sign_readiness(app, &app.session.wallets[i])));
    let shown = (rows * cols).min(total);
    for (k, &i) in order.iter().enumerate().take(shown) {
        let w = &app.session.wallets[i];
        let rx = x + (k % cols) as f32 * (rw + gap);
        let ry = y + (k / cols) as f32 * 52.0;
        let action = open(i);
        let ready = sign_readiness(app, w);
        ui.fill(
            rx,
            ry,
            rw,
            44.0,
            10.0,
            if ui.is_pressed(action) {
                INNER
            } else {
                SURFACE
            },
        );
        ui.stroke(
            rx,
            ry,
            rw,
            44.0,
            10.0,
            if ready == 2 { OK.with_alpha(90) } else { LINE },
        );
        let (m, _) = Session::quorum(w);
        let here = app
            .session
            .slots(w)
            .iter()
            .filter(|s| s.held_by.is_some())
            .count()
            .min(m);
        // The marker on the right: ready, how many here, or none.
        let marker = match ready {
            2 => format!("Ready to sign · {here} of {m}"),
            1 => format!("{here} of {m} here"),
            _ => "Watch only".to_string(),
        };
        let tone = match ready {
            2 => OK,
            1 => TEXT,
            _ => DIM,
        };
        let mw = ui.measure(12.0, W::S, &marker) + 20.0;
        if ready == 2 {
            ui.fill(
                rx + rw - 10.0 - mw,
                ry + 10.0,
                mw,
                24.0,
                12.0,
                OK.with_alpha(30),
            );
        }
        ui.text_mid(rx + rw - mw, ry, 44.0, 12.0, W::S, tone, &marker);
        let name = ui.fit(14.0, W::S, &w.name, rw * 0.4);
        let nw = ui.text_mid(rx + 14.0, ry, 44.0, 14.0, W::S, TEXT, &name);
        let shape = ui.fit(12.0, W::R, &Session::shape(w), rw - nw - mw - 50.0);
        ui.text_mid(rx + 26.0 + nw, ry, 44.0, 12.0, W::R, MUTED, &shape);
        ui.hit(rx, ry, rw, 44.0, action);
    }
    let mut used = shown.div_ceil(cols) as f32 * 52.0;
    if shown < total {
        let more = format!("+ {} more", total - shown);
        let mw = ui.measure(12.0, W::S, &more) + 8.0;
        ui.text_mid(x, y + used - 4.0, 24.0, 12.0, W::S, ACCENT, &more);
        ui.hit(x, y + used - 4.0, mw, 24.0, open(order[shown]));
        used += 24.0;
    }
    used + 6.0
}

// ---------------------------------------------------------------------
// Wallets
// ---------------------------------------------------------------------

/// What the Wallets tab offers for a key in no wallet.
pub(crate) const FROM_KEY: &str = "Make a wallet from this key";

/// The Wallets card with keys loaded and no wallet: each key, the one
/// picked when there are several, and the ways on to a wallet. Returns
/// the height used.
pub(crate) fn loose_card(app: &Faraday, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    let (loose, picked) = app.loose_keys();
    let Some(picked) = picked else {
        return 0.0;
    };
    let mut cy = y;
    if let [only] = loose.as_slice() {
        let line = format!("Key {} is loaded. No wallet uses it.", fp_text(*only));
        cy += ui.wrap(x, cy, w, 15.0, W::R, TEXT, &line) + 16.0;
    } else {
        let line = format!("{} keys are loaded. No wallet uses them.", loose.len());
        cy += ui.wrap(x, cy, w, 15.0, W::R, TEXT, &line) + 10.0;
        for fp in &loose {
            let on = fp.0 == picked;
            ui.checkbox(x, cy + 11.0, on, true);
            let line = key_line(*fp, app.session.key_label(*fp).unwrap_or(""));
            let line = ui.fit(14.0, W::M, &line, w - 30.0);
            ui.text_mid(
                x + 30.0,
                cy,
                40.0,
                14.0,
                W::M,
                if on { TEXT } else { MUTED },
                &line,
            );
            ui.hit(x, cy, w, 40.0, Action::PickKey(fp.0));
            ui.rule(x, cy + 40.0, w, INNER);
            cy += 42.0;
        }
        cy += 14.0;
    }
    let items = [
        (FROM_KEY, Style::Primary, Action::KeyWallet(picked, 1)),
        (
            "Add another key",
            Style::Secondary,
            Action::KeyWallet(picked, 2),
        ),
        (
            "Load or restore a wallet",
            Style::Secondary,
            Action::RestoreWallet,
        ),
    ];
    cy += if ui.compact {
        crate::compact_screens::stack(ui, x, cy, w, &items)
    } else {
        wrap_buttons(ui, x, cy, w, 40.0, &items)
    };
    cy - y
}

fn wallets(app: &Faraday, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    if ui.compact {
        crate::compact_screens::wallet_card(app, ui, x0, cw, h);
        return;
    }
    let x = x0 + 40.0;
    let width = cw - 80.0;
    let mut y = 28.0;
    title(ui, x, y, "Wallets");
    network_pill(app, ui, x + width, y + 2.0);
    y += 56.0;

    // An Inbox PSBT leads.
    let lead = match app.spend.as_ref() {
        Some(s) => Some((
            s.spend.source.clone(),
            "Signing",
            Action::Nav(Screen::Spend),
            "Continue",
        )),
        None => app.lead_psbt().map(|i| {
            (
                app.inbox[i].name.clone(),
                "in Files",
                Action::StartSpend(i),
                "Review",
            )
        }),
    };
    if let Some((name, where_, action, verb)) = lead
        && !app.session.wallets.is_empty()
    {
        let bg = if ui.is_pressed(action) {
            ACCENT.with_alpha(40)
        } else {
            ACCENT.with_alpha(18)
        };
        ui.fill(x, y, width, 48.0, 12.0, bg);
        ui.stroke(x, y, width, 48.0, 12.0, ACCENT.with_alpha(76));
        ui.icon(x + 14.0, y + 14.0, 20.0, Icon::File, 13.0, ACCENT);
        let nw = ui.text_mid(x + 44.0, y, 48.0, 14.0, W::M, TEXT, &name);
        ui.text_mid(x + 56.0 + nw, y, 48.0, 14.0, W::R, MUTED, where_);
        ui.text_right(x + width - 18.0, y, 48.0, 14.0, W::S, ACCENT, verb);
        ui.hit(x, y, width, 48.0, action);
        y += 64.0;
    }

    let listw = 250.0;
    let bottom = h - 28.0;
    let may = app.may_load_keys();
    let total = app.session.wallets.len();
    let none = total == 0;
    let (loose, _) = app.loose_keys();
    // The card offers the loose keys' next steps, and Load or restore,
    // when no wallet is loaded.
    let card_offers = none && !loose.is_empty();

    // The left column, top down: the wallets, the keys in no wallet,
    // then the buttons; scrolled when taller than the screen.
    const ROW: f32 = 60.0;
    const KEY_ROW: f32 = 22.0;
    const KEY_BUTTON: f32 = 46.0;
    let mut content = 0.0;
    if !none {
        content += 28.0 + total as f32 * ROW;
    }
    if !loose.is_empty() {
        let each = if none { KEY_ROW } else { KEY_ROW + KEY_BUTTON };
        content += 8.0 + 26.0 + loose.len() as f32 * each;
    }
    content += 8.0 + if none && !card_offers { 48.0 } else { 0.0 } + 40.0;
    let view_h = bottom - y;
    let max_shift = (content - view_h).max(0.0);
    let shift = app.list_offset.min(max_shift);
    let clip = ui.rect(x - 4.0, y, listw + 8.0, view_h);
    ui.c.push_clip(clip);
    let mut ly = y - shift;
    if !none {
        section_label(ui, x + 4.0, ly, "Wallets");
        ly += 28.0;
    }
    for (i, wlt) in app.session.wallets.iter().enumerate() {
        let current = i == app.wallet;
        let action = Action::PickWallet(i);
        let (bg, edge) = if current {
            (ACCENT.with_alpha(20), ACCENT.with_alpha(100))
        } else {
            (SURFACE, LINE)
        };
        ui.fill(x, ly, listw, 54.0, 10.0, bg);
        ui.stroke(x, ly, listw, 54.0, 10.0, edge);
        let name = ui.fit(14.0, W::S, &wlt.name, listw - 28.0);
        ui.text(x + 14.0, ly + 9.0, 14.0, W::S, TEXT, &name);
        let (m, _) = Session::quorum(wlt);
        let here = app
            .session
            .slots(wlt)
            .iter()
            .filter(|s| s.held_by.is_some())
            .count()
            .min(m);
        let shape = Session::shape(wlt);
        let (t, c) = if here == 0 {
            (format!("{shape} · watch only"), DIM)
        } else if sign_readiness(app, wlt) == 2 {
            (format!("{shape} · {here} of {m} here"), OK)
        } else {
            (format!("{shape} · {here} of {m} here"), MUTED)
        };
        let t = ui.fit(12.0, W::R, &t, listw - 28.0);
        ui.text(x + 14.0, ly + 31.0, 12.0, W::R, c, &t);
        ui.hit(x, ly, listw, 54.0, action);
        ly += ROW;
    }
    // Keys not in any wallet.
    if !loose.is_empty() {
        ly += 8.0;
        section_label(ui, x + 4.0, ly, "Keys without a wallet");
        ly += 26.0;
        for fp in &loose {
            let line = key_line(*fp, app.session.key_label(*fp).unwrap_or(""));
            let line = ui.fit(13.0, W::R, &line, listw - 8.0);
            ui.text(x + 4.0, ly, 13.0, W::R, TEXT, &line);
            ly += KEY_ROW;
            // With wallets loaded the card shows one of them: the key's
            // own row offers the wallet from it.
            if !none {
                ui.button(
                    x,
                    ly,
                    Some(listw),
                    38.0,
                    FROM_KEY,
                    Style::Secondary,
                    Action::KeyWallet(fp.0, 1),
                );
                ly += KEY_BUTTON;
            }
        }
    }
    ly += 8.0;
    if none && !card_offers {
        ui.button(
            x,
            ly,
            Some(listw),
            40.0,
            "Load or restore a wallet",
            Style::Secondary,
            Action::RestoreWallet,
        );
        ly += 48.0;
    }
    ui.button(
        x,
        ly,
        Some(listw),
        40.0,
        "Add a key",
        if may {
            Style::Secondary
        } else {
            Style::Disabled
        },
        Action::Entry(None),
    );
    ui.c.pop_clip();
    ui.report_scroll(clip, max_shift);

    // The wallet card.
    let cx = x + listw + 16.0;
    let cwid = x + width - cx;
    let ch = bottom - y;
    ui.card(cx, y, cwid, ch, LINE);
    let Some(wlt) = app.session.wallets.get(app.wallet) else {
        if card_offers {
            loose_card(app, ui, cx + 24.0, y + 24.0, cwid - 48.0);
        } else {
            let msg = "No wallet loaded";
            ui.wrap(cx + 24.0, y + 24.0, cwid - 48.0, 15.0, W::R, MUTED, msg);
        }
        return;
    };
    let ix = cx + 24.0;
    let iw = cwid - 48.0;
    let mut cy = y + 22.0;
    match app.renaming.as_ref() {
        Some(name) => {
            ui.fill(ix - 6.0, cy - 4.0, 360.0, 38.0, 8.0, BG);
            ui.stroke(ix - 6.0, cy - 4.0, 360.0, 38.0, 8.0, ACCENT.with_alpha(110));
            ui.selection(ix, cy - 4.0, 38.0, 22.0, W::S, name, true);
            let nw = ui.text(ix, cy, 22.0, W::S, TEXT, name);
            if !ui.select_all {
                // The caret, drawn: a glyph would be one more character.
                ui.caret(ix + nw + 2.0, cy + 2.0, 24.0);
            }
            ui.hit(ix - 6.0, cy - 4.0, 360.0, 38.0, Action::Rename);
            ui.text(
                ix + 370.0,
                cy + 8.0,
                12.0,
                W::R,
                DIM,
                "Enter keeps it · Esc leaves it",
            );
        }
        None => {
            let nw = ui.text(ix, cy, 22.0, W::S, TEXT, &wlt.name);
            ui.button(
                ix + nw + 14.0,
                cy,
                None,
                30.0,
                "Rename",
                Style::Ghost,
                Action::Rename,
            );
        }
    }
    let src = format!("From {}", wlt.source);
    let sw = ui.measure(12.0, W::R, &src) + 24.0;
    ui.pill(ix + iw - sw, cy, 28.0, &src);
    cy += 34.0;
    ui.text(ix, cy, 14.0, W::R, MUTED, &Session::shape(wlt));
    cy += 26.0;
    if let Some(r) = wlt.policy.silent() {
        // Where its backup is (§5.2): a press opens Backups on it.
        cy += backup_line(app, ui, ix, cy, iw, app.wallet);
        silent_wallet_card(app, ui, r, ix, iw, cy, y + ch);
        return;
    }

    let (m, _) = Session::quorum(wlt);
    let slots = app.session.slots(wlt);
    let here = slots.iter().filter(|s| s.held_by.is_some()).count().min(m);

    // Actions: the first along the foot, Remove from session at the
    // end; a row more above when they do not fit the card's width.
    // The first: from the vault to a spend (`docs/NEW-WALLET.md` §11.2).
    let (spend_label, spend_action) = app.spend_button(app.wallet);
    let mut items: Vec<(&str, Style, Action)> = vec![(&spend_label, Style::Primary, spend_action)];
    items.push((
        "Show wallet QR",
        Style::Secondary,
        Action::QrWallet(app.wallet),
    ));
    items.push(("Back up", Style::Secondary, Action::Backup(app.wallet)));
    if app.session.message_wallets().contains(&app.wallet) {
        items.push(("Sign a message", Style::Secondary, Action::SignMessage));
    }
    let foot = card_actions(
        ui,
        ix,
        y + ch - 62.0,
        iw,
        &items,
        (
            "Remove from session",
            Style::Ghost,
            Action::RemoveWallet(app.wallet),
        ),
    );

    // The body, between the head and the actions: scrolled when the
    // chart makes it taller than the card.
    let view_top = cy - 6.0;
    let view_h = (foot - 8.0 - view_top).max(0.0);
    let clip = ui.rect(cx + 1.0, view_top, cwid - 2.0, view_h);
    ui.c.push_clip(clip);
    let top = cy;
    cy -= app.card_offset;
    // Where its backup is (§5.2): a press opens Backups on it.
    cy += backup_line(app, ui, ix, cy, iw, app.wallet);
    ui.fill(ix, cy, iw, 50.0, 10.0, BG);
    ui.stroke(ix, cy, iw, 50.0, 10.0, INNER);
    let pw = ui.pips(ix + 16.0, cy + 20.0, m, here);
    ui.text_mid(
        ix + 30.0 + pw,
        cy,
        50.0,
        14.0,
        W::S,
        TEXT,
        &format!("{here} of {m} signatures possible here"),
    );
    let short_by = m - here;
    let need = if short_by == 0 {
        "Nothing missing".to_string()
    } else {
        // "More" only once there is some already.
        let more = if here > 0 { " more" } else { "" };
        format!(
            "Add {short_by}{more} {} or collect {short_by}{more} {}",
            if short_by == 1 { "key" } else { "keys" },
            if short_by == 1 {
                "signature"
            } else {
                "signatures"
            }
        )
    };
    ui.text_right(
        ix + iw - 16.0,
        cy,
        50.0,
        13.0,
        W::R,
        if short_by == 0 { OK } else { WARN },
        &need,
    );
    cy += 66.0;

    // The wallet at a glance (`docs/NEW-WALLET.md` §6): its keys and
    // its backup, where each thing is and what each place gives.
    ui.text(ix, cy, 13.0, W::S, MUTED, "At a glance");
    cy += 24.0;
    if let Some(g) = crate::glance::of(app, app.wallet) {
        cy += crate::glance::draw(ui, &g, crate::glance::Direction::WalletFirst, ix, cy, iw);
    }
    // Where the keys still missing may be.
    let quorum_here =
        slots.iter().filter(|s| s.held_by.is_some()).count() >= crate::wallet::needed(wlt);
    if !quorum_here {
        cy += 4.0;
        cy += missing_keys_line(app, ui, &slots, Screen::Wallets, ix, cy, iw);
    }
    cy += 16.0;
    ui.text(ix, cy, 13.0, W::S, MUTED, "First receive address · 0/0");
    cy += 24.0;
    let addr = grouped(&app.session.address_shown(wlt, false, 0));
    cy += ui.wrap(ix, cy, iw, 15.0, W::M, TEXT, &addr) + 8.0;
    if !app.wallet_held(app.wallet).is_empty() {
        cy += ui.wrap(ix, cy, iw, 13.0, W::S, WARN, crate::glance::RECEIVE_WARNING) + 8.0;
    }
    ui.c.pop_clip();
    let content = cy + app.card_offset - top;
    let max = (content - (view_h - (top - view_top))).max(0.0);
    ui.report_scroll_in(crate::ui::Slot::Card, clip, max, app.card_offset);
}

/// A wallet card's actions along its foot, the last row's top at `ay`:
/// `items` from the left, `end` at the right of the last row. Where they
/// do not fit `w` they take more rows, upward, so the last row stays at
/// the foot. A rule runs above the first row. Returns the rule's y.
fn card_actions(
    ui: &mut Ui,
    x: f32,
    ay: f32,
    w: f32,
    items: &[(&str, Style, Action)],
    end: (&str, Style, Action),
) -> f32 {
    const H: f32 = 40.0;
    const GAP: f32 = 8.0;
    let width = |ui: &Ui, label: &str| (ui.measure(14.0, W::S, label) + 32.0).min(w);
    // Each item's row and x, then the end's.
    let mut placed: Vec<(usize, f32, f32)> = Vec::new();
    let (mut row, mut bx) = (0usize, 0.0f32);
    for &(label, ..) in items {
        let bw = width(ui, label);
        if bx > 0.0 && bx + bw > w {
            row += 1;
            bx = 0.0;
        }
        placed.push((row, bx, bw));
        bx += bw + GAP;
    }
    let ew = width(ui, end.0);
    if bx > 0.0 && bx + ew > w {
        row += 1;
    }
    let rows = row + 1;
    let top = ay - (rows - 1) as f32 * (H + GAP);
    ui.rule(x, top - 14.0, w, INNER);
    for (&(label, style, action), &(r, px, bw)) in items.iter().zip(&placed) {
        let label = ui.fit(14.0, W::S, label, bw - 24.0);
        ui.button(
            x + px,
            top + r as f32 * (H + GAP),
            Some(bw),
            H,
            &label,
            style,
            action,
        );
    }
    let label = ui.fit(14.0, W::S, end.0, ew - 24.0);
    ui.button(x + w - ew, ay, Some(ew), H, &label, end.1, end.2);
    top - 14.0
}

/// A silent payments wallet's card: its key, its address and the labels
/// handed out, then its page. It has nothing to sign: sending to and from
/// silent payments waits on upstream.
fn silent_wallet_card(
    app: &Faraday,
    ui: &mut Ui,
    r: &osk_bip::silent_wallet::SilentWallet,
    ix: f32,
    iw: f32,
    y: f32,
    bottom: f32,
) {
    let may = app.may_load_keys();
    let mut cy = y;
    ui.text(ix, cy, 13.0, W::S, MUTED, "Key");
    cy += 24.0;
    let held = app.session.key_label(r.fingerprint);
    ui.text_mid(ix, cy, 46.0, 14.0, W::M, TEXT, &fp_text(r.fingerprint));
    if let Some(label) = held {
        ui.text_mid(ix + 104.0, cy, 46.0, 13.0, W::R, MUTED, label);
    }
    let (state, fg, bg) = if held.is_some() {
        ("Here", OK, OK.with_alpha(30))
    } else {
        ("Not here", MUTED, INNER)
    };
    let button = held.is_none().then(|| {
        if app.vault_key_for(r.fingerprint).is_some() {
            (
                "Load from vault",
                Style::Primary,
                Action::Vault(crate::vaults::VaultAction::LoadKeyOf(r.fingerprint.0)),
            )
        } else {
            (
                "Add its key",
                Style::Secondary,
                Action::Entry(Some(r.fingerprint.0)),
            )
        }
    });
    let bw = button.map_or(0.0, |(l, _, _)| ui.measure(13.0, W::S, l) + 28.0);
    let chipw = ui.measure(12.0, W::R, state) + 34.0;
    ui.chip(ix + iw - bw - 10.0 - chipw, cy + 10.0, state, fg, bg);
    if let Some((label, style, action)) = button {
        let style = if may { style } else { Style::Disabled };
        ui.button(ix + iw - bw, cy + 7.0, Some(bw), 32.0, label, style, action);
    }
    ui.rule(ix, cy + 46.0, iw, INNER);
    cy += 64.0;
    ui.text(ix, cy, 13.0, W::S, MUTED, "Address");
    cy += 24.0;
    cy += ui.wrap(
        ix,
        cy,
        iw,
        14.0,
        W::M,
        TEXT,
        &app.session.shown(&r.address()),
    ) + 18.0;
    ui.text(ix, cy, 13.0, W::S, MUTED, "Labels handed out");
    ui.text(ix + 180.0, cy, 13.0, W::M, TEXT, &r.labels.to_string());
    let _ = card_actions(
        ui,
        ix,
        bottom - 62.0,
        iw,
        &[
            (
                "Address, labels and payments",
                Style::Primary,
                Action::SWallet(app.wallet),
            ),
            (
                "Show wallet QR",
                Style::Secondary,
                Action::QrWallet(app.wallet),
            ),
        ],
        (
            "Remove from session",
            Style::Ghost,
            Action::RemoveWallet(app.wallet),
        ),
    );
}

// ---------------------------------------------------------------------
// Sign a transaction
// ---------------------------------------------------------------------

const STEPS: [&str; 10] = [
    "Wallet",
    "Check",
    "Transaction",
    "Transaction id",
    "Signers",
    "Sign",
    "Signatures for this transaction",
    "Finish",
    "Path",
    "Nonces",
];

fn spend(app: &mut Faraday, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    let panel_w = 320.0;
    let col_x = x0 + 40.0;
    let col_w = cw - panel_w - 72.0;
    let needed = app.spend_needed();
    if app.spend.is_none() {
        title(ui, col_x, 28.0, "Sign a transaction");
        ui.text(col_x, 80.0, 15.0, W::R, MUTED, "No transaction loaded");
        ui.button(
            col_x,
            116.0,
            None,
            40.0,
            "Open Files",
            Style::Secondary,
            Action::Nav(Screen::Files),
        );
        return;
    }
    let (wallet_idx, steps) = {
        let s = app.spend.as_ref().expect("spend");
        (s.wallet, s.steps.clone())
    };

    let (cards, scroll, guided) = {
        let s = app.spend.as_ref().expect("spend");
        let kind = wallet_idx
            .and_then(|w| app.session.wallets.get(w))
            .map(|w| crate::wallet::Kind::of(&w.policy));
        let cards: Vec<flow::Card> = steps
            .iter()
            .map(|&n| flow::Card {
                title: STEPS[n as usize].to_string(),
                summary: step_summary(app, n, wallet_idx, needed),
                mono: false,
                done: s.done[n as usize],
                open: s.open == Some(n),
                default: n == step::CHECK,
                toggle: Action::Step(n),
                guide: Some(guide::spend(n, kind, needed, s, app.medium)),
            })
            .collect();
        (
            cards,
            flow::Scroll {
                y: s.scroll,
                follow: s.follow,
            },
            app.guided,
        )
    };
    let compact = ui.compact;
    let col = flow::Column {
        area_x: x0,
        area_w: if compact { cw } else { cw - panel_w },
        x: col_x,
        w: col_w,
        h: if compact { h - SPEND_BAR } else { h },
        back: Some(("Wallets", Action::Nav(Screen::Start))),
        heading: "Sign a transaction",
        guided,
        switch: true,
        note: None,
        chip: None,
        chip_tap: None,
    };
    let (next, again) = {
        let app_ref: &Faraday = app;
        let mut body = |ui: &mut Ui, i: usize, x: f32, y: f32, w: f32| -> f32 {
            step_body(app_ref, ui, steps[i], x, y, w, wallet_idx, needed)
        };
        flow::column(ui, &col, &cards, scroll, &mut body)
    };
    if let Some(s) = app.spend.as_mut() {
        s.scroll = next.y;
        s.follow = next.follow;
    }
    if again {
        app.dirty = true;
        app.commands.push_back(osk_shell_api::Command::Draw);
    }

    // The panel; on a small panel, the foot, whose button is the open
    // step's Continue while it is read.
    if compact {
        let step = ui.pin.take();
        spend_bar(app, ui, x0, cw, h - SPEND_BAR, needed, step);
    } else {
        spend_panel(app, ui, x0 + cw - panel_w, panel_w, h, wallet_idx, needed);
    }
}

/// Where a signing stands: the signatures there, how many are missing,
/// the line that says what is left, and the main button's label.
fn spend_status(app: &Faraday, needed: usize) -> (usize, usize, String, String) {
    let s = app.spend.as_ref().expect("spend");
    let i = &s.inspection;
    // A threshold spend counts partial signatures and signs as shares.
    let threshold = threshold_of(s);
    let signed_count = match threshold {
        Some(t) if !s.complete => t.signed.len(),
        _ => s.signers.len(),
    };
    let count = if s.complete {
        needed
    } else {
        signed_count.min(needed)
    };
    let missing = if s.complete {
        0
    } else {
        needed.saturating_sub(signed_count).max(1)
    };
    let can_sign_here = match threshold {
        Some(t) => t
            .ours
            .iter()
            .filter(|id| !t.signed.contains(id))
            .map(|id| format!("share {}", id + 1))
            .collect::<Vec<_>>(),
        None => i
            .participating_keys
            .iter()
            .filter(|fp| app.session.key_label(**fp).is_some() && !s.signers.contains(&fp.0))
            .map(|fp| fp_text(*fp))
            .collect::<Vec<_>>(),
    };
    let carry = threshold.is_some() && (s.carry_out.is_some() || s.out_signed) && !s.complete;
    let need_line = if carry {
        if s.out_signed {
            "Carry file kept · the next share signs with it".to_string()
        } else {
            "The next share signs with the carry file".to_string()
        }
    } else if s.spend.finished.is_some() && s.out_signed && s.out_tx {
        format!("Both wait {}", app.medium.for_the())
    } else if missing == 0 {
        "Complete".to_string()
    } else if threshold.is_some() && !can_sign_here.is_empty() && missing > 1 {
        format!(
            "Sign here · {} more at {} own place",
            missing - 1,
            if missing == 2 { "its" } else { "their" }
        )
    } else if !can_sign_here.is_empty() && missing > 1 {
        format!("Sign here and collect {} more", missing - 1)
    } else if !can_sign_here.is_empty() {
        "Sign here".to_string()
    } else {
        crate::wallet::collect_line(missing, count)
    };
    let label = if carry {
        if s.out_signed {
            "Open Files".to_string()
        } else {
            "Keep the carry file".to_string()
        }
    } else if s.spend.finished.is_some() {
        // Both went For the stick at Finish: the next step is writing.
        if app.sticks.is_empty() {
            format!("Insert {} to write it", app.medium.a())
        } else {
            "Open Files".to_string()
        }
    } else if missing == 0 {
        "Finish".to_string()
    } else if !can_sign_here.is_empty() {
        format!("Sign with {}", can_sign_here.join(", "))
    } else {
        crate::wallet::collect_line(missing, count)
    };
    (count, missing, need_line, label)
}

/// The foot of Sign a transaction on a small panel: the signatures, what
/// is left, and the main button. Returns its height.
fn spend_bar(
    app: &Faraday,
    ui: &mut Ui,
    x0: f32,
    cw: f32,
    y: f32,
    needed: usize,
    step: Option<(String, Style, Action)>,
) {
    let (count, missing, need_line, label) = spend_status(app, needed);
    let (label, style, action) = step.unwrap_or((label, Style::Primary, Action::Primary));
    let (x, w) = (x0 + crate::compact::M, cw - 2.0 * crate::compact::M);
    ui.fill(x0, y, cw, SPEND_BAR, 0.0, SIDEBAR);
    ui.fill(x0, y, cw, 1.0, 0.0, LINE);
    let pw = ui.pips(x, y + 14.0, needed, count);
    ui.text_mid(
        x + pw + 10.0,
        y + 4.0,
        26.0,
        13.0,
        W::S,
        TEXT,
        &format!("{count} of {needed}"),
    );
    let line = ui.fit(12.0, W::R, &need_line, w);
    ui.text(
        x,
        y + 30.0,
        12.0,
        W::R,
        if missing == 0 { OK } else { WARN },
        &line,
    );
    let label = ui.fit(15.0, W::S, &label, w - 24.0);
    ui.button(x, y + 52.0, Some(w), 44.0, &label, style, action);
}

/// The height of that foot.
const SPEND_BAR: f32 = 104.0;

/// The first input's threshold view, when the spend is from a threshold
/// wallet.
fn threshold_of(s: &crate::SpendState) -> Option<&osk_psbt::ThresholdInput> {
    s.inspection
        .inputs
        .first()
        .and_then(|i| i.threshold.as_ref())
}

fn recipients(inspection: &osk_psbt::Inspection) -> Vec<&osk_psbt::OutputInfo> {
    inspection
        .outputs
        .iter()
        .filter(|o| matches!(o.kind, OutputKind::Recipient))
        .collect()
}

pub(crate) fn step_summary(app: &Faraday, n: u8, wallet: Option<usize>, needed: usize) -> String {
    let s = app.spend.as_ref().expect("spend");
    let i = &s.inspection;
    let reached = s.done[n as usize] || s.open == Some(n);
    match n {
        0 => match wallet.and_then(|w| app.session.wallets.get(w)) {
            Some(w) => format!("{} · {}", w.name, Session::shape(w)),
            None => "No loaded wallet matches".to_string(),
        },
        1 if reached => "Addresses · Compare".to_string(),
        2 if reached => format!(
            "Sends {} BTC · fee {} sat/vB",
            btc(i.amount_to_others.to_sat()),
            i.fee_rate_sat_vb
                .map(|r| format!("{r:.1}"))
                .unwrap_or_else(|| "?".into())
        ),
        4 if reached && threshold_of(s).is_some() => {
            let t = threshold_of(s).expect("threshold");
            format!("{} here · {needed} needed", t.ours.len())
        }
        4 if reached => {
            let here = i
                .participating_keys
                .iter()
                .filter(|fp| app.session.key_label(**fp).is_some())
                .count();
            format!("{here} here · {needed} needed")
        }
        5 => {
            if s.spend.signed_here.is_empty() {
                "Not signed here yet".to_string()
            } else {
                let fps: Vec<String> = s.spend.signed_here.iter().map(|f| fp_text(*f)).collect();
                format!("Signed with {}", fps.join(", "))
            }
        }
        6 => format!("{} of {needed}", s.signers.len()),
        9 => match wallet.and_then(|w| app.session.wallets.get(w)) {
            Some(w) => {
                let rows = crate::wallet::musig_nonces(&app.session, w, &s.spend.psbt);
                let have = rows.iter().filter(|r| r.3).count();
                format!("{have} of {} nonces", rows.len())
            }
            None => String::new(),
        },
        8 => {
            let n = wallet
                .and_then(|w| app.session.wallets.get(w))
                .map(|w| w.policy.spend_paths().len())
                .unwrap_or(0);
            format!(
                "{n} {}",
                if n == 1 {
                    "way to spend"
                } else {
                    "ways to spend"
                }
            )
        }
        7 => {
            if s.spend.finished.is_some() {
                "Signed PSBT and finished transaction".to_string()
            } else {
                "Not finished yet".to_string()
            }
        }
        _ => "Not reached yet".to_string(),
    }
}

/// Draws a step's body with its top-left at (x, y). Returns its height.
#[allow(clippy::too_many_arguments)]
pub(crate) fn step_body(
    app: &Faraday,
    ui: &mut Ui,
    n: u8,
    x: f32,
    y: f32,
    w: f32,
    wallet: Option<usize>,
    needed: usize,
) -> f32 {
    let s = app.spend.as_ref().expect("spend");
    let i = &s.inspection;
    let mut cy = y;
    // A step's Continue: on a small panel it takes the foot's button
    // ([`spend_bar`]) while the step is read.
    let next = |ui: &mut Ui, cy: f32, label: &str| {
        if ui.pinning {
            ui.pin = Some((label.to_string(), Style::Primary, Action::StepNext(n)));
            return;
        }
        let bw = ui.measure(14.0, W::S, label) + 36.0;
        ui.button(
            x + w - bw,
            cy,
            Some(bw),
            40.0,
            label,
            Style::Primary,
            Action::StepNext(n),
        );
    };
    match n {
        0 => {
            match wallet.and_then(|wi| app.session.wallets.get(wi)) {
                Some(wl) => {
                    ui.text(x, cy, 14.0, W::S, TEXT, &wl.name);
                    ui.text(
                        x,
                        cy + 22.0,
                        12.0,
                        W::R,
                        MUTED,
                        &format!("{} · from {}", Session::shape(wl), wl.source),
                    );
                }
                None => {
                    ui.wrap(x, cy, w - 140.0, 13.0, W::R, WARN,
                        "No loaded wallet matches this transaction. Change cannot be verified until its wallet is loaded.");
                }
            }
            next(ui, cy, "Continue");
            cy += 44.0;
        }
        1 => {
            match wallet.and_then(|wi| app.session.wallets.get(wi)) {
                Some(wl) => {
                    for (label, change, idx) in [
                        ("Receive 0/0", false, 0),
                        ("Receive 0/1", false, 1),
                        ("Change 1/0", true, 0),
                    ] {
                        ui.text(x, cy + 2.0, 12.0, W::R, MUTED, label);
                        let a = grouped(&app.session.address_shown(wl, change, idx));
                        // On a small panel the address goes under its label.
                        let ax = if ui.compact {
                            cy += 20.0;
                            x
                        } else {
                            x + 96.0
                        };
                        let used = ui.wrap(ax, cy, x + w - ax, 13.0, W::M, TEXT, &a);
                        cy += used.max(20.0) + 8.0;
                        ui.rule(x, cy - 4.0, w, INNER);
                    }
                }
                None => {
                    ui.text(x, cy, 13.0, W::R, MUTED, "No wallet to show addresses from");
                    cy += 24.0;
                }
            }
            cy += 8.0;
            next(ui, cy, "Continue");
            cy += 44.0;
        }
        2 => {
            let rows_top = cy;
            let mut rows: Vec<ReviewRow> = Vec::new();
            for o in recipients(i) {
                rows.push((
                    "Sends".into(),
                    btc(o.amount.to_sat()),
                    format!("to {}", short(&app.session.shown(&o.address))),
                    None,
                ));
            }
            for o in &i.outputs {
                match o.kind {
                    OutputKind::WalletChange { change, index, .. }
                    | OutputKind::Change { change, index, .. } => {
                        rows.push((
                            "Change".into(),
                            btc(o.amount.to_sat()),
                            format!("to {}/{index}", if change { 1 } else { 0 }),
                            Some(("Verified", OK)),
                        ));
                    }
                    OutputKind::UnverifiedChange { .. } => {
                        rows.push((
                            "Change".into(),
                            btc(o.amount.to_sat()),
                            format!("to {}", short(&app.session.shown(&o.address))),
                            Some(("Unverified", WARN)),
                        ));
                    }
                    OutputKind::Recipient => {}
                }
            }
            rows.push((
                "Fee".into(),
                btc(i.fee.to_sat()),
                format!(
                    "{} sat · {} sat/vB",
                    thousands(i.fee.to_sat()),
                    i.fee_rate_sat_vb
                        .map(|r| format!("{r:.1}"))
                        .unwrap_or_else(|| "?".into())
                ),
                None,
            ));
            let unchecked = i.inputs.iter().any(|inp| inp.amount.is_none());
            rows.push((
                "Spends".into(),
                btc(i.total_in.to_sat()),
                format!(
                    "{} {}",
                    i.inputs.len(),
                    if i.inputs.len() == 1 {
                        "input"
                    } else {
                        "inputs"
                    }
                ),
                if unchecked {
                    Some(("Amount unchecked", WARN))
                } else {
                    None
                },
            ));
            // The transaction's id, at the foot: known before signing
            // for a segwit spend, not for a legacy one.
            if txid_known(&s.spend.psbt) {
                let segwit = i.inputs.iter().all(|inp| {
                    !matches!(inp.script_type, osk_psbt::ScriptKind::P2pkh)
                        && !format!("{:?}", inp.script_type).contains("Sh")
                        || format!("{:?}", inp.script_type).contains("Wsh")
                });
                rows.push((
                    "Txid".into(),
                    short(&s.spend.txid.to_string()),
                    if segwit {
                        "Unchanged by signing"
                    } else {
                        "Changes when signed"
                    }
                    .to_string(),
                    None,
                ));
            } else {
                rows.push((
                    "Txid".into(),
                    "Not known until signed".to_string(),
                    String::new(),
                    None,
                ));
            }
            // On a small panel each row is two lines: what and how much,
            // then where and its note.
            let compact = ui.compact;
            let row_h = if compact { 52.0 } else { 40.0 };
            let box_h = rows.len() as f32 * row_h;
            ui.fill(x, rows_top, w, box_h, 10.0, BG);
            ui.stroke(x, rows_top, w, box_h, 10.0, INNER);
            for (k, (label, amount, rest, note)) in rows.iter().enumerate() {
                let ry = rows_top + k as f32 * row_h;
                if compact {
                    ui.text_mid(x + 12.0, ry + 4.0, 24.0, 12.0, W::R, MUTED, label);
                    ui.text_right(x + w - 12.0, ry + 4.0, 24.0, 14.0, W::M, TEXT, amount);
                    let nw = note.map_or(0.0, |(t, _)| ui.measure(12.0, W::R, t) + 8.0);
                    let rest = ui.fit(12.0, W::R, rest, w - 24.0 - nw);
                    ui.text_mid(x + 12.0, ry + 26.0, 22.0, 12.0, W::R, MUTED, &rest);
                    if let Some((t, c)) = note {
                        ui.text_right(x + w - 12.0, ry + 26.0, 22.0, 12.0, W::R, *c, t);
                    }
                } else {
                    ui.text_mid(x + 14.0, ry, row_h, 12.0, W::R, MUTED, label);
                    let aw = ui.text_mid(x + 84.0, ry, row_h, 14.0, W::M, TEXT, amount);
                    ui.text_mid(x + 98.0 + aw, ry, row_h, 13.0, W::R, MUTED, rest);
                    if let Some((t, c)) = note {
                        ui.text_right(x + w - 14.0, ry, row_h, 12.0, W::R, *c, t);
                    }
                }
                if k + 1 < rows.len() {
                    ui.rule(x, ry + row_h, w, SURFACE);
                }
            }
            cy += box_h + 12.0;
            // Warnings, every one as the inspector words it.
            let mut warned = false;
            for wn in &i.warnings {
                let c = match wn.level {
                    Level::Info => MUTED,
                    Level::Caution => WARN,
                    Level::Danger | Level::Blocked => ERR,
                };
                let used = ui.wrap(x + 22.0, cy, w - 22.0, 13.0, W::R, c, &wn.text);
                ui.icon(x, cy, 16.0, Icon::Warning, 10.0, c);
                cy += used.max(18.0) + 6.0;
                warned = true;
            }
            if !warned {
                let mut chx = x;
                chx += ui.chip(chx, cy, "No warnings", OK, OK.with_alpha(26)) + 6.0;
                let net = app.session.network();
                let c = if net.is_mainnet() { OK } else { WARN };
                ui.chip(chx, cy, network_name(net), c, c.with_alpha(26));
                cy += 34.0;
            }
            if s.table && compact {
                // What and how much on a line, the outpoint or address
                // under it.
                let lines = i
                    .inputs
                    .iter()
                    .map(|inp| {
                        (
                            "Input",
                            inp.amount
                                .map(|a| btc(a.to_sat()))
                                .unwrap_or_else(|| "?".into()),
                            format!("{}:{}", short(&inp.txid.to_string()), inp.vout),
                        )
                    })
                    .chain(i.outputs.iter().map(|o| {
                        (
                            "Output",
                            btc(o.amount.to_sat()),
                            app.session.shown(&o.address),
                        )
                    }))
                    .collect::<Vec<_>>();
                for (label, amount, what) in &lines {
                    ui.text(x, cy, 12.0, W::R, MUTED, label);
                    ui.text_right(x + w, cy - 4.0, 20.0, 12.0, W::M, TEXT, amount);
                    cy += 18.0;
                    cy += ui.wrap(x, cy, w, 12.0, W::M, TEXT, what) + 10.0;
                }
                cy += 6.0;
            } else if s.table {
                for inp in &i.inputs {
                    let a = inp
                        .amount
                        .map(|a| btc(a.to_sat()))
                        .unwrap_or_else(|| "?".into());
                    ui.text(x, cy, 12.0, W::R, MUTED, "Input");
                    ui.text(
                        x + 64.0,
                        cy,
                        12.0,
                        W::M,
                        TEXT,
                        &format!("{}:{}", short(&inp.txid.to_string()), inp.vout),
                    );
                    ui.text_right(x + w, cy - 4.0, 20.0, 12.0, W::M, TEXT, &a);
                    cy += 22.0;
                }
                for o in &i.outputs {
                    ui.text(x, cy, 12.0, W::R, MUTED, "Output");
                    let shown = app.session.shown(&o.address);
                    let used = ui.wrap(x + 64.0, cy, w - 64.0 - 110.0, 12.0, W::M, TEXT, &shown);
                    ui.text_right(
                        x + w,
                        cy - 4.0,
                        20.0,
                        12.0,
                        W::M,
                        TEXT,
                        &btc(o.amount.to_sat()),
                    );
                    cy += used.max(18.0) + 4.0;
                }
                cy += 6.0;
            }
            ui.button(
                x,
                cy,
                None,
                38.0,
                if s.table {
                    "Hide the table"
                } else {
                    "Show the table"
                },
                Style::Secondary,
                Action::ToggleTable,
            );
            next(ui, cy, "Continue");
            cy += 44.0;
        }
        4 if threshold_of(s).is_some() => {
            let t = threshold_of(s).expect("threshold");
            // The first location chooses who signs; after it the PSBT
            // names them.
            let first = t.signers.is_empty();
            if let Some(wl) = wallet.and_then(|wi| app.session.wallets.get(wi)) {
                // On a small panel a share is two lines: the share, then
                // who holds it with what it does.
                let compact = ui.compact;
                for (k, slot) in app.session.slots(wl).iter().enumerate() {
                    let id = k as u32;
                    ui.text_mid(x, cy, 42.0, 12.0, W::R, DIM, &format!("Share {}", k + 1));
                    let fp = slot.fingerprint.map(fp_text).unwrap_or_default();
                    ui.text_mid(x + 64.0, cy, 42.0, 14.0, W::M, TEXT, &fp);
                    let (lx, ly) = if compact {
                        (x + 64.0, cy + 36.0)
                    } else {
                        (x + 166.0, cy)
                    };
                    let here = t.ours.contains(&id);
                    let (tag, fg, bg) = if t.signed.contains(&id) {
                        ("Signed", OK, OK.with_alpha(30))
                    } else if here {
                        ("Signs here", OK, OK.with_alpha(30))
                    } else if t.signers.contains(&id) {
                        ("Signs at its own place", ACCENT, ACCENT.with_alpha(26))
                    } else if first {
                        ("", MUTED, INNER)
                    } else {
                        ("Not signing", DIM, INNER)
                    };
                    let right = if first && !here {
                        let on = s.others.contains(&id);
                        let label = if on { "Signs next" } else { "Choose" };
                        let bw = ui.measure(13.0, W::S, label) + 28.0;
                        ui.button(
                            x + w - bw,
                            ly + 5.0,
                            Some(bw),
                            32.0,
                            label,
                            if on { Style::Primary } else { Style::Secondary },
                            Action::TOther(id),
                        );
                        bw
                    } else if !tag.is_empty() {
                        let cw2 = ui.measure(12.0, W::R, tag) + 34.0;
                        ui.chip(x + w - cw2, ly + 8.0, tag, fg, bg);
                        cw2
                    } else {
                        0.0
                    };
                    let label = slot.held_by.clone().unwrap_or_default();
                    let label = ui.fit(13.0, W::R, &label, x + w - lx - right - 8.0);
                    ui.text_mid(lx, ly, 42.0, 13.0, W::R, MUTED, &label);
                    ui.rule(x, ly + 42.0, w, INNER);
                    cy = ly + 44.0;
                }
            }
            cy += 8.0;
            let line = if first {
                format!(
                    "Needs {needed} · {} here · {} chosen",
                    t.ours.len(),
                    s.others.len()
                )
            } else {
                format!("Needs {needed} · {} signed", t.signed.len())
            };
            ui.text_mid(x, cy, 40.0, 13.0, W::R, MUTED, &line);
            // Under the line when both do not fit on one row.
            let nw = ui.measure(14.0, W::S, "Continue") + 36.0;
            if ui.measure(13.0, W::R, &line) + nw + 12.0 > w {
                cy += 40.0;
            }
            next(ui, cy, "Continue");
            cy += 44.0;
        }
        4 => {
            match wallet.and_then(|wi| app.session.wallets.get(wi)) {
                Some(wl) => {
                    let slots = app.session.slots(wl);
                    let enough = slots.iter().filter(|s| s.held_by.is_some()).count() >= needed;
                    // On a small panel a key is two lines: the key, then
                    // where it signs and the way to bring it here.
                    let compact = ui.compact;
                    for (k, slot) in slots.iter().enumerate() {
                        ui.text_mid(x, cy, 42.0, 12.0, W::R, DIM, &(k + 1).to_string());
                        let fp = slot.fingerprint.map(fp_text).unwrap_or_default();
                        ui.text_mid(x + 22.0, cy, 42.0, 14.0, W::M, TEXT, &fp);
                        let label = slot.held_by.clone().unwrap_or_default();
                        let label = ui.fit(13.0, W::R, &label, w - 124.0);
                        ui.text_mid(x + 124.0, cy, 42.0, 13.0, W::R, MUTED, &label);
                        if compact {
                            cy += 36.0;
                        }
                        let (t, fg, bg) = if slot.held_by.is_some() {
                            ("Signs here", OK, OK.with_alpha(30))
                        } else if enough {
                            ("Not needed", DIM, INNER)
                        } else {
                            ("Another device", MUTED, INNER)
                        };
                        // A key not here can still be brought here: from an
                        // open vault, or typed or scanned.
                        let button = (slot.held_by.is_none() && !enough).then(|| {
                            match slot.fingerprint.filter(|f| app.vault_key_for(*f).is_some()) {
                                Some(f) => (
                                    "Load from vault",
                                    Action::Vault(crate::vaults::VaultAction::LoadKeyOf(f.0)),
                                ),
                                None => {
                                    ("Add its key", Action::Entry(slot.fingerprint.map(|f| f.0)))
                                }
                            }
                        });
                        let bw = button.map_or(0.0, |(l, _)| ui.measure(13.0, W::S, l) + 28.0);
                        let cw2 = ui.measure(12.0, W::R, t) + 34.0;
                        let chip_x = if compact {
                            x + 22.0
                        } else {
                            x + w - bw - 10.0 - cw2
                        };
                        ui.chip(chip_x, cy + 8.0, t, fg, bg);
                        if let Some((l, a)) = button {
                            ui.button(
                                x + w - bw,
                                cy + 5.0,
                                Some(bw),
                                32.0,
                                l,
                                if app.may_load_keys() {
                                    Style::Secondary
                                } else {
                                    Style::Disabled
                                },
                                a,
                            );
                        }
                        ui.rule(x, cy + 42.0, w, INNER);
                        cy += 44.0;
                    }
                    if !enough {
                        cy += 4.0;
                        cy += missing_keys_line(app, ui, &slots, Screen::Spend, x, cy, w);
                    }
                }
                None => {
                    for fp in &i.participating_keys {
                        ui.text_mid(x, cy, 30.0, 14.0, W::M, TEXT, &fp_text(*fp));
                        cy += 30.0;
                    }
                }
            }
            cy += 8.0;
            let here = i
                .participating_keys
                .iter()
                .filter(|fp| app.session.key_label(**fp).is_some())
                .count();
            ui.text_mid(
                x,
                cy,
                40.0,
                13.0,
                W::R,
                MUTED,
                &format!("Needs {needed} · {here} here"),
            );
            next(ui, cy, "Continue");
            cy += 44.0;
        }
        5 if threshold_of(s).is_some() && s.spend.signed_here.is_empty() => {
            let t = threshold_of(s).expect("threshold");
            let mine: Vec<u32> = t
                .ours
                .iter()
                .copied()
                .filter(|id| !t.signed.contains(id))
                .collect();
            let want = needed.saturating_sub(1);
            let (label, style, reason) = if mine.is_empty() {
                (
                    String::new(),
                    Style::Disabled,
                    Some("No share of this wallet is loaded here".to_string()),
                )
            } else if t.signers.is_empty() && s.others.len() != want {
                (
                    String::new(),
                    Style::Disabled,
                    Some(format!(
                        "Choose {want} other {} under Signers",
                        if want == 1 { "share" } else { "shares" }
                    )),
                )
            } else {
                let names: Vec<String> =
                    mine.iter().map(|id| format!("share {}", id + 1)).collect();
                (
                    format!("Sign as {}", names.join(", ")),
                    Style::Primary,
                    None,
                )
            };
            if let Some(r) = reason {
                ui.text_mid(x, cy, 40.0, 13.0, W::R, MUTED, &r);
            } else {
                let bw = ui.measure(14.0, W::S, &label) + 36.0;
                ui.button(
                    x + w - bw,
                    cy,
                    Some(bw),
                    40.0,
                    &label,
                    style,
                    Action::SignHere,
                );
            }
            cy += 44.0;
        }
        5 => {
            let mine: Vec<String> = i
                .participating_keys
                .iter()
                .filter(|fp| app.session.key_label(**fp).is_some() && !s.signers.contains(&fp.0))
                .map(|fp| fp_text(*fp))
                .collect();
            if !s.spend.signed_here.is_empty() {
                let fps: Vec<String> = s.spend.signed_here.iter().map(|f| fp_text(*f)).collect();
                ui.icon(x, cy + 10.0, 20.0, Icon::Done, 12.0, OK);
                ui.text_mid(
                    x + 26.0,
                    cy,
                    40.0,
                    14.0,
                    W::S,
                    OK,
                    &format!("Signed with {}", fps.join(", ")),
                );
                cy += 44.0;
            } else if mine.is_empty() {
                ui.text_mid(
                    x,
                    cy,
                    40.0,
                    13.0,
                    W::R,
                    MUTED,
                    "No key in this session signs this transaction",
                );
                cy += 44.0;
            } else {
                let label = format!("Sign with {}", mine.join(", "));
                ui.text_mid(
                    x,
                    cy,
                    40.0,
                    13.0,
                    W::R,
                    MUTED,
                    &format!(
                        "{} {}",
                        i.inputs.len(),
                        if i.inputs.len() == 1 {
                            "input"
                        } else {
                            "inputs"
                        }
                    ),
                );
                let bw = ui.measure(14.0, W::S, &label) + 36.0;
                ui.button(
                    x + w - bw,
                    cy,
                    Some(bw),
                    40.0,
                    &label,
                    Style::Primary,
                    Action::SignHere,
                );
                cy += 44.0;
            }
        }
        6 => {
            let slots: Vec<(String, String, Option<[u8; 4]>)> =
                match wallet.and_then(|wi| app.session.wallets.get(wi)) {
                    Some(wl) => app
                        .session
                        .slots(wl)
                        .iter()
                        .map(|sl| {
                            (
                                sl.fingerprint.map(fp_text).unwrap_or_default(),
                                sl.held_by.clone().unwrap_or_default(),
                                sl.fingerprint.map(|f| f.0),
                            )
                        })
                        .collect(),
                    None => i
                        .participating_keys
                        .iter()
                        .map(|fp| (fp_text(*fp), String::new(), Some(fp.0)))
                        .collect(),
                };
            let complete = s.complete;
            for (fp, label, raw) in &slots {
                ui.text_mid(x, cy, 44.0, 14.0, W::M, TEXT, fp);
                let label = if label.is_empty() {
                    s.spend
                        .collected
                        .iter()
                        .find(|(f, _)| Some(f.0) == *raw)
                        .map(|(_, src)| format!("from {src}"))
                        .unwrap_or_default()
                } else {
                    label.clone()
                };
                // On a small panel where it came from goes under the key.
                let (lx, ly) = if ui.compact && !label.is_empty() {
                    (x, cy + 26.0)
                } else if ui.compact {
                    (x, cy)
                } else {
                    (x + 102.0, cy)
                };
                let label = ui.fit(13.0, W::R, &label, x + w - lx - 90.0);
                ui.text_mid(lx, ly, 44.0, 13.0, W::R, MUTED, &label);
                let signed = raw.map(|r| s.signers.contains(&r)).unwrap_or(false);
                let here = raw
                    .map(|r| s.spend.signed_here.iter().any(|f| f.0 == r))
                    .unwrap_or(false);
                let (t, c) = if signed && here {
                    ("Signed here", OK)
                } else if signed {
                    ("Verified", OK)
                } else if complete {
                    ("Not needed", DIM)
                } else {
                    ("Waiting", DIM)
                };
                ui.text_right(x + w, cy, 44.0, 12.0, W::R, c, t);
                ui.rule(x, ly + 44.0, w, INNER);
                cy = ly + 46.0;
            }
            cy += 10.0;
            // Cosigners' copies in the Inbox.
            let txid = s.spend.txid;
            let copies: Vec<(usize, String)> = app
                .inbox
                .iter()
                .enumerate()
                .filter(|(_, it)| it.kind == FileKind::Psbt && it.name != s.spend.source)
                .filter(|(_, it)| {
                    crate::wallet::read_psbt(&it.bytes)
                        .map(|p| p.unsigned_tx().compute_txid() == txid)
                        .unwrap_or(false)
                })
                .map(|(k, it)| (k, it.name.clone()))
                .collect();
            if !complete {
                if copies.is_empty() {
                    ui.text_mid(
                        x,
                        cy,
                        30.0,
                        13.0,
                        W::R,
                        MUTED,
                        "No cosigner's copy in Files",
                    );
                    cy += 34.0;
                }
                for (k, name) in &copies {
                    // No wider than the column; a long name is cut.
                    let label = format!("Add from {name}");
                    let bw = (ui.measure(14.0, W::S, &label) + 32.0).min(w);
                    let label = ui.fit(14.0, W::S, &label, bw - 24.0);
                    ui.button(
                        x,
                        cy,
                        Some(bw),
                        38.0,
                        &label,
                        Style::Secondary,
                        Action::Collect(*k),
                    );
                    cy += 46.0;
                }
            }
            if !complete {
                let part = format!("{}-part.psbt", crate::result_stem(&s.spend.source));
                cy += made_line(app, ui, x, cy, w, &part);
                cy += wrap_buttons(
                    ui,
                    x,
                    cy,
                    w,
                    38.0,
                    &[
                        ("Show as QR", Style::Secondary, Action::QrPart),
                        ("Scan a copy", Style::Secondary, Action::Scan),
                    ],
                );
            } else {
                next(ui, cy, "Continue");
                cy += 44.0;
            }
        }
        7 if s.spend.finished.is_none()
            && (s.carry_out.is_some() || s.out_signed)
            && threshold_of(s).is_some() =>
        {
            let t = threshold_of(s).expect("threshold");
            let next_ids: Vec<String> = t
                .signers
                .iter()
                .filter(|id| !t.signed.contains(id))
                .map(|id| format!("share {}", id + 1))
                .collect();
            ui.fill(x, cy, w, 150.0, 10.0, BG);
            ui.stroke(x, cy, w, 150.0, 10.0, INNER);
            ui.text(x + 14.0, cy + 14.0, 14.0, W::S, TEXT, "Carry file");
            let sub = format!(
                "For {} · holds its secret nonce until it signs",
                if next_ids.is_empty() {
                    "the next share".to_string()
                } else {
                    next_ids.join(", ")
                }
            );
            let sub = ui.fit(12.0, W::R, &sub, w - 28.0);
            ui.text(x + 14.0, cy + 38.0, 12.0, W::R, MUTED, &sub);
            let name = format!("{}-partly-signed.osk", crate::result_stem(&s.spend.source));
            ui.text(x + 14.0, cy + 62.0, 12.0, W::M, TEXT, &name);
            let (label, style) = if s.out_signed {
                ("Kept", Style::Disabled)
            } else {
                ("Keep it", Style::Secondary)
            };
            ui.button(
                x + 14.0,
                cy + 98.0,
                None,
                36.0,
                label,
                style,
                Action::CarryToOutbox,
            );
            if s.out_signed {
                ui.icon(x + w - 34.0, cy + 106.0, 20.0, Icon::Done, 12.0, OK);
            }
            cy += 162.0;
        }
        7 => {
            if s.spend.finished.is_none() {
                let missing = needed.saturating_sub(s.signers.len()).max(1);
                let line = match s.spend.finish_reason() {
                    Some(r) if s.signers.len() >= needed => r,
                    _ => format!(
                        "Needs {missing}{} {}",
                        if s.signers.is_empty() { "" } else { " more" },
                        if missing == 1 {
                            "signature"
                        } else {
                            "signatures"
                        }
                    ),
                };
                ui.wrap(x, cy, w, 13.0, W::R, WARN, &line);
                cy += 44.0;
            } else {
                // Side by side; on a small panel one under the other.
                let compact = ui.compact;
                let half = if compact { w } else { (w - 12.0) / 2.0 };
                let mut card_y = cy;
                let stem = crate::result_stem(&s.spend.source);
                for (k, (head, sub, file, action)) in [
                    (
                        "Signed PSBT",
                        "For the wallet that wrote it · not finalised",
                        format!("{stem}-signed.psbt"),
                        Action::SignedToOutbox,
                    ),
                    (
                        "Finished transaction",
                        "Ready to broadcast",
                        format!("{stem}-final.txn"),
                        Action::TxToOutbox,
                    ),
                ]
                .iter()
                .enumerate()
                {
                    let (bx, cy) = if compact {
                        (x, card_y)
                    } else {
                        (x + k as f32 * (half + 12.0), cy)
                    };
                    // Made For the stick at Finish: Remove while it waits,
                    // made again only when asked.
                    let place = made_place(app, file);
                    let (label, style, action) = match &place {
                        Some((_, Some(i))) => ("Remove", Style::Ghost, Action::OutboxRemove(*i)),
                        _ => ("Make again", Style::Secondary, *action),
                    };
                    // Where the buttons go: in a row when they fit.
                    let lw = ui.measure(13.0, W::S, label) + 32.0;
                    let qw = ui.measure(13.0, W::S, "Show as QR") + 32.0;
                    let stacked = k == 0 && lw + 8.0 + qw > half - 28.0;
                    let by = if compact { cy + 92.0 } else { cy + 112.0 };
                    let card_h = if stacked {
                        by - cy + 36.0 + 8.0 + 36.0 + 16.0
                    } else if compact {
                        by - cy + 36.0 + 16.0
                    } else {
                        168.0
                    };
                    ui.fill(bx, cy, half, card_h, 10.0, BG);
                    ui.stroke(bx, cy, half, card_h, 10.0, INNER);
                    ui.text(bx + 14.0, cy + 14.0, 14.0, W::S, TEXT, head);
                    let sub = ui.fit(12.0, W::R, sub, half - 28.0);
                    ui.text(bx + 14.0, cy + 38.0, 12.0, W::R, MUTED, &sub);
                    let detail = if k == 0 {
                        format!("{}-signed.psbt", crate::result_stem(&s.spend.source))
                    } else {
                        let Some(tx) = s.spend.finished.as_ref() else {
                            continue;
                        };
                        format!(
                            "{} · {} vB",
                            short(&tx.compute_txid().to_string()),
                            tx.vsize()
                        )
                    };
                    let detail = ui.fit(12.0, W::M, &detail, half - 28.0);
                    ui.text(bx + 14.0, cy + 66.0, 12.0, W::M, TEXT, &detail);
                    let bw = ui.button(bx + 14.0, by, Some(lw), 36.0, label, style, action);
                    if k == 0 {
                        let (qx, qy) = if stacked {
                            (bx + 14.0, by + 44.0)
                        } else {
                            (bx + 22.0 + bw, by)
                        };
                        ui.button(
                            qx,
                            qy,
                            Some(qw),
                            36.0,
                            "Show as QR",
                            Style::Secondary,
                            Action::QrSigned,
                        );
                    }
                    if let Some((t, _)) = &place {
                        ui.text_right(bx + half - 14.0, cy + 14.0, 20.0, 12.0, W::S, OK, t);
                    }
                    card_y += card_h + 10.0;
                }
                cy = if compact { card_y + 2.0 } else { cy + 180.0 };
                cy += nonce_check(app, ui, &s.spend.psbt, x, cy, w);
                // The finished transaction itself: its bytes, and taken
                // apart.
                cy += wrap_buttons(
                    ui,
                    x,
                    cy,
                    w,
                    38.0,
                    &[
                        ("Decode it", Style::Secondary, Action::DecodeFinished),
                        (
                            if s.show_hex {
                                "Hide the raw hex"
                            } else {
                                "Show the raw hex"
                            },
                            Style::Secondary,
                            Action::SpendHex,
                        ),
                    ],
                ) + 4.0;
                if s.show_hex
                    && let Some(hex) = s.spend.finished_hex()
                {
                    let th = ui.wrap(
                        x + 12.0,
                        cy + 10.0,
                        w - 24.0,
                        12.0,
                        W::M,
                        TEXT,
                        &spaced(&hex),
                    );
                    ui.stroke(x, cy, w, th + 20.0, 8.0, INNER);
                    cy += th + 32.0;
                }
            }
        }
        9 => {
            if let Some(wl) = wallet.and_then(|wi| app.session.wallets.get(wi)) {
                let rows = crate::wallet::musig_nonces(&app.session, wl, &s.spend.psbt);
                for (slot, fp, here, has) in &rows {
                    ui.text_mid(x, cy, 40.0, 12.0, W::R, DIM, &slot.to_string());
                    ui.text_mid(x + 22.0, cy, 40.0, 14.0, W::M, TEXT, fp);
                    if *here {
                        // On a small panel it goes under the key.
                        let (hx, hy) = if ui.compact {
                            (x + 22.0, cy + 24.0)
                        } else {
                            (x + 124.0, cy)
                        };
                        ui.text_mid(hx, hy, 40.0, 13.0, W::R, MUTED, "This device");
                    }
                    if *here && ui.compact {
                        cy += 24.0;
                    }
                    let (t, c) = if *has {
                        ("Nonce here", OK)
                    } else {
                        ("Waiting", DIM)
                    };
                    ui.text_right(x + w, cy, 40.0, 12.0, W::R, c, t);
                    ui.rule(x, cy + 40.0, w, INNER);
                    cy += 42.0;
                }
                cy += 10.0;
                let txid = s.spend.txid;
                let copies: Vec<(usize, String)> = app
                    .inbox
                    .iter()
                    .enumerate()
                    .filter(|(_, it)| it.kind == FileKind::Psbt && it.name != s.spend.source)
                    .filter(|(_, it)| {
                        crate::wallet::read_psbt(&it.bytes)
                            .map(|p| p.unsigned_tx().compute_txid() == txid)
                            .unwrap_or(false)
                    })
                    .map(|(k, it)| (k, it.name.clone()))
                    .collect();
                let labels: Vec<String> = copies
                    .iter()
                    .map(|(_, name)| format!("Add from {name}"))
                    .collect();
                let items: Vec<(&str, Style, Action)> = labels
                    .iter()
                    .zip(&copies)
                    .map(|(l, (k, _))| (l.as_str(), Style::Secondary, Action::Collect(*k)))
                    .collect();
                if !items.is_empty() {
                    cy += wrap_buttons(ui, x, cy, w, 36.0, &items) + 2.0;
                }
                let part = format!("{}-part.psbt", crate::result_stem(&s.spend.source));
                cy += made_line(app, ui, x, cy, w, &part);
                let others_in = rows.iter().filter(|r| !r.2).all(|r| r.3);
                let mine_in = rows.iter().filter(|r| r.2).all(|r| r.3);
                cy += wrap_buttons(
                    ui,
                    x,
                    cy,
                    w,
                    38.0,
                    &[("Show as QR", Style::Secondary, Action::QrPart)],
                ) + 2.0;
                if !others_in && !mine_in {
                    ui.button(
                        x,
                        cy,
                        None,
                        38.0,
                        "Go first: share this device's nonce",
                        Style::Secondary,
                        Action::SignHere,
                    );
                    cy += 48.0;
                }
                next(ui, cy, "Continue");
                cy += 48.0;
            }
        }
        8 => {
            if let Some(wl) = wallet.and_then(|wi| app.session.wallets.get(wi)) {
                for (k, p) in crate::wallet::paths(&app.session, wl, &s.spend.psbt)
                    .iter()
                    .enumerate()
                {
                    let top = cy;
                    ui.fill(x, cy, w, 0.0, 0.0, BG);
                    ui.text(
                        x + 14.0,
                        cy + 12.0,
                        13.0,
                        W::S,
                        TEXT,
                        &format!("Path {}", k + 1),
                    );
                    // The keys, wrapping; on a small panel under the name.
                    let (left, mut ky) = if ui.compact {
                        (x + 14.0, cy + 36.0)
                    } else {
                        (x + 90.0, cy + 8.0)
                    };
                    let mut kx = left;
                    for (slot, fp, here) in &p.keys {
                        let t = format!("key {slot} {fp}");
                        let (fg, bg) = if *here {
                            (OK, OK.with_alpha(30))
                        } else {
                            (MUTED, INNER)
                        };
                        let kw = ui.measure(12.0, W::R, &t) + 34.0;
                        if kx > left && kx + kw > x + w - 10.0 {
                            kx = left;
                            ky += 30.0;
                        }
                        kx += ui.chip(kx, ky, &t, fg, bg) + 6.0;
                    }
                    let mut ly = ky + 32.0;
                    if p.locks.is_empty() {
                        ui.text(x + 14.0, ly, 12.0, W::R, MUTED, "No wait");
                        ly += 20.0;
                    }
                    for l in &p.locks {
                        ui.text(x + 14.0, ly, 12.0, W::R, MUTED, &format!("Waits {l}"));
                        ly += 20.0;
                    }
                    let (verdict, c) = if p.sequence_ok {
                        ("This transaction's sequence allows this path", OK)
                    } else {
                        ("This transaction's sequence does not allow this path", DIM)
                    };
                    ly += ui.wrap(x + 14.0, ly, w - 28.0, 12.0, W::R, c, verdict) + 12.0;
                    ui.stroke(x, top, w, ly - top, 10.0, INNER);
                    cy = ly + 8.0;
                }
            }
            next(ui, cy, "Continue");
            cy += 48.0;
        }
        _ => {}
    }
    if let Some(e) = &s.error {
        let used = ui.wrap(x, cy, w, 13.0, W::R, ERR, e);
        cy += used + 6.0;
    }
    cy - y
}

/// One line of the review: its label, the amount, the rest of the line,
/// and a note with its colour.
type ReviewRow = (
    String,
    String,
    String,
    Option<(&'static str, osk_ui::Color)>,
);

#[allow(clippy::too_many_arguments)]
fn spend_panel(
    app: &Faraday,
    ui: &mut Ui,
    px: f32,
    pw: f32,
    h: f32,
    wallet: Option<usize>,
    needed: usize,
) {
    let s = app.spend.as_ref().expect("spend");
    let i = &s.inspection;
    ui.fill(px, 0.0, pw, h, 0.0, SURFACE);
    ui.fill(px, 0.0, 1.0, h, 0.0, LINE);
    let x = px + 26.0;
    let w = pw - 52.0;
    let mut y = 32.0;
    ui.text(x, y, 13.0, W::S, MUTED, "This transaction");
    y += 30.0;
    let to = recipients(i)
        .first()
        .map(|o| short(&app.session.shown(&o.address)))
        .unwrap_or_else(|| "·".into());
    let change = if i.outputs.iter().any(|o| o.is_ours()) {
        "Verified".to_string()
    } else if i
        .outputs
        .iter()
        .any(|o| matches!(o.kind, OutputKind::UnverifiedChange { .. }))
    {
        "Unverified".to_string()
    } else {
        "None".to_string()
    };
    let facts: [(&str, String, W, u8); 6] = [
        (
            "Wallet",
            wallet
                .and_then(|wi| app.session.wallets.get(wi))
                .map(|wl| wl.name.clone())
                .unwrap_or_else(|| "Not loaded".into()),
            W::R,
            0,
        ),
        (
            "Sends",
            format!("{} BTC", btc(i.amount_to_others.to_sat())),
            W::M,
            2,
        ),
        ("To", to, W::M, 2),
        (
            "Fee",
            format!(
                "{} sat · {} sat/vB",
                thousands(i.fee.to_sat()),
                i.fee_rate_sat_vb
                    .map(|r| format!("{r:.1}"))
                    .unwrap_or_else(|| "?".into())
            ),
            W::R,
            2,
        ),
        ("Change", change, W::R, 2),
        ("Txid", short(&s.spend.txid.to_string()), W::M, 3),
    ];
    for (k, v, wt, step) in facts.iter() {
        ui.text_mid(x, y, 38.0, 12.0, W::R, MUTED, k);
        let v = ui.fit(13.0, *wt, v, w - 70.0);
        ui.text_right(x + w, y, 38.0, 13.0, *wt, TEXT, &v);
        ui.rule(x, y + 38.0, w, INNER);
        ui.hit(x, y, w, 38.0, Action::Step(*step));
        y += 40.0;
    }
    let (count, missing, need_line, action_label) = spend_status(app, needed);
    let fy = h - 26.0 - 46.0 - 14.0 - 82.0;
    ui.fill(x, fy, w, 82.0, 12.0, BG);
    ui.stroke(x, fy, w, 82.0, 12.0, INNER);
    ui.text_mid(x + 16.0, fy + 10.0, 26.0, 13.0, W::R, MUTED, "Signatures");
    let label = format!("{count} of {needed}");
    let lw = ui.measure(13.0, W::S, &label);
    ui.text_mid(x + w - 16.0 - lw, fy + 10.0, 26.0, 13.0, W::S, TEXT, &label);
    let pips_w = needed as f32 * 34.0 - 6.0;
    ui.pips(x + w - 26.0 - lw - pips_w, fy + 18.0, needed, count);
    ui.text_mid(
        x + 16.0,
        fy + 44.0,
        26.0,
        13.0,
        W::R,
        if missing == 0 { OK } else { WARN },
        &need_line,
    );
    let by = h - 26.0 - 46.0;
    let label = ui.fit(15.0, W::S, &action_label, w - 32.0);
    ui.button(
        x,
        by,
        Some(w),
        46.0,
        &label,
        Style::Primary,
        Action::Primary,
    );
}

// ---------------------------------------------------------------------
// Back up a wallet
// ---------------------------------------------------------------------

/// A plan question's title.
fn question_title(q: u8) -> &'static str {
    use crate::qstep;
    match q {
        qstep::PRESET => "Start from",
        qstep::SEEDS => "The seeds go",
        qstep::PLACES => "Places",
        qstep::WALLET => "The wallet description goes",
        qstep::SOFTWARE => "Software",
        qstep::PASSPHRASE => "Passphrases",
        _ => "Map",
    }
}

/// The rows of the seeds question.
const SEED_ROWS: [&str; 4] = [
    "On paper, words by hand",
    "On paper, SeedQR by hand",
    "Into vaults",
    "As a file, unprotected",
];

/// The rows of the software question.
const SOFTWARE_ROWS: [&str; 5] = [
    "Sparrow",
    "Coldcard, Keystone, Passport",
    "Nunchuk",
    "Bitcoin Core",
    "Not sure",
];

/// The rows of the wallet description question, by whether each place
/// keeps a share.
fn description_rows(shares: bool, medium: crate::Medium) -> [String; 4] {
    [
        if shares {
            "A share in each place"
        } else {
            "A sheet in each place"
        }
        .to_string(),
        "In the vault".to_string(),
        "Into watch-only software".to_string(),
        format!("As files on {}", medium.a()),
    ]
}

/// The labels of the rows ticked, joined; "None" when none is.
fn ticked(labels: &[&str], on: &[bool]) -> String {
    let v: Vec<&str> = labels
        .iter()
        .zip(on)
        .filter(|(_, o)| **o)
        .map(|(l, _)| *l)
        .collect();
    if v.is_empty() {
        "None".to_string()
    } else {
        v.join(" · ")
    }
}

/// A checklist item's title and the line shown while it is closed.
fn item_title(app: &Faraday, n: u8) -> (String, String) {
    use crate::plan::Item;
    let Some(b) = app.backup.as_ref() else {
        return (String::new(), String::new());
    };
    let Some(wallet) = app.session.wallets.get(b.wallet) else {
        return (String::new(), String::new());
    };
    let shape = app.plan_shape(b.wallet);
    let a = &b.answers;
    let (_, keys_n) = Session::quorum(wallet);
    let item = crate::bstep::item(n).unwrap_or(Item::Envelopes);
    let done = app.backup_item_done(item);
    match item {
        Item::Templates => {
            let k = crate::plan::templates(&shape, a).max(1);
            (
                format!(
                    "Print {k} blank {}",
                    if k == 1 { "template" } else { "templates" }
                ),
                format!("Template for {} words", b.words),
            )
        }
        Item::Copy(i) => (
            format!(
                "Copy seed {} by hand",
                shape.seeds.get(i).map_or("", |s| s.name.as_str())
            ),
            if done { "Checked" } else { "Not checked" }.to_string(),
        ),
        Item::Vault(v) => {
            let names: Vec<&str> = a
                .vault_seeds(&shape, v)
                .into_iter()
                .map(|i| shape.seeds[i].name.as_str())
                .collect();
            let title = if names.is_empty() {
                format!("Vault {}", v + 1)
            } else {
                format!("Vault {}: {}", v + 1, names.join(", "))
            };
            let line = match app.backup_vault_fits(v) {
                Some(name) => format!("In {name}"),
                None => "Not saved".to_string(),
            };
            (title, line)
        }
        Item::SeedFiles => (
            "The seeds as files".to_string(),
            if done {
                app.medium.for_box()
            } else {
                "Not made"
            }
            .to_string(),
        ),
        Item::Sheets if a.split && shape.splits => (
            "The shares".to_string(),
            format!(
                "{keys_n} shares, {} keys each",
                keys_n - a.omit.min(shape.m.saturating_sub(1))
            ),
        ),
        Item::Sheets => (
            "The wallet sheet".to_string(),
            if done {
                app.medium.for_box()
            } else {
                "A PDF to print"
            }
            .to_string(),
        ),
        Item::PublicFiles => (
            "The public files".to_string(),
            ticked(&SOFTWARE_ROWS, &a.software),
        ),
        Item::ShowDescriptor => (
            "Show the descriptor to the software".to_string(),
            if done { "Shown" } else { "Not shown" }.to_string(),
        ),
        Item::Envelopes => (
            "One envelope per place".to_string(),
            format!(
                "{} {}",
                a.places,
                if a.places == 1 {
                    "envelope"
                } else {
                    "envelopes"
                }
            ),
        ),
    }
}

/// A plan question's line while it is closed: its answers.
fn question_summary(app: &Faraday, q: u8) -> String {
    use crate::qstep;
    let Some(b) = app.backup.as_ref() else {
        return String::new();
    };
    let shape = app.plan_shape(b.wallet);
    let a = &b.answers;
    match q {
        qstep::PRESET => "3 presets".to_string(),
        qstep::SEEDS => ticked(&SEED_ROWS, &a.seeds),
        qstep::PLACES => {
            let mut s = format!(
                "{} {}",
                a.places,
                if a.places == 1 { "place" } else { "places" }
            );
            if shape.splits && a.wallet[crate::plan::wallet::PAPER] {
                s.push_str(if a.split {
                    " · a share each"
                } else {
                    " · the whole sheet"
                });
            }
            s
        }
        qstep::WALLET => {
            let rows = description_rows(a.split && shape.splits, app.medium);
            let labels: Vec<&str> = rows.iter().map(String::as_str).collect();
            ticked(&labels, &a.wallet)
        }
        qstep::SOFTWARE => ticked(&SOFTWARE_ROWS, &a.software),
        qstep::PASSPHRASE => {
            let k = shape
                .seeds
                .iter()
                .filter(|s| s.here && s.passphrase)
                .count();
            format!("{k} {}", if k == 1 { "passphrase" } else { "passphrases" })
        }
        _ => {
            let c = crate::plan::check(&shape, a);
            format!("Any one place lost: {}", c.lost.text())
        }
    }
}

fn backup_screen(app: &mut Faraday, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    use crate::BStage;
    let panel_w = 320.0;
    let col_x = x0 + 40.0;
    let col_w = cw - panel_w - 72.0;
    let Some(b) = app.backup.as_ref() else {
        title(ui, col_x, 28.0, "Back up a wallet");
        return;
    };
    let Some(wallet) = app.session.wallets.get(b.wallet) else {
        return;
    };
    let keys = app.backup_keys(b.wallet);
    let (m, n) = Session::quorum(wallet);
    let checklist = b.stage == BStage::Checklist;
    let steps = if checklist {
        app.backup_items()
    } else {
        app.backup_questions()
    };
    // The items after one not done wait for it (`docs/NEW-WALLET.md`
    // §14.1): each says which.
    let waiting = if checklist {
        app.backup_waiting()
    } else {
        None
    };
    let waiting_title = waiting.map(|n| item_title(app, n).0);
    let cards: Vec<flow::Card> = steps
        .iter()
        .map(|&k| {
            if checklist {
                let (title, summary) = item_title(app, k);
                let summary = match (&waiting_title, waiting) {
                    (Some(t), Some(n))
                        if n != k
                            && !app.backup_reachable(k)
                            && !crate::bstep::item(k)
                                .is_some_and(|it| app.backup_item_done(it)) =>
                    {
                        format!("After: {t}")
                    }
                    _ => summary,
                };
                flow::Card {
                    title,
                    summary,
                    mono: false,
                    done: crate::bstep::item(k).is_some_and(|it| app.backup_item_done(it)),
                    open: b.open == Some(k),
                    default: false,
                    toggle: Action::BStep(k),
                    guide: Some(guide::backup(k, m, n, keys.len(), app.medium))
                        .filter(|g| !g.is_empty()),
                }
            } else {
                flow::Card {
                    title: question_title(k).to_string(),
                    summary: question_summary(app, k),
                    mono: false,
                    done: false,
                    open: b.q == Some(k),
                    default: false,
                    toggle: Action::BQ(k),
                    guide: None,
                }
            }
        })
        .collect();
    // With more than one wallet loaded, the one backed up is a chip that
    // opens the list of them.
    let many = app.session.wallets.len() > 1;
    let heading = if many {
        "Back up a wallet".to_string()
    } else {
        format!("Back up {}", wallet.name)
    };
    let pick = b.pick && many;
    // A small panel has no room for the side panel: the map is a page of
    // the plan, and the envelopes list the places.
    let compact = ui.compact;
    let col = flow::Column {
        area_x: x0,
        area_w: if compact { cw } else { cw - panel_w },
        x: col_x,
        w: col_w,
        h,
        back: Some(if b.from_vault {
            ("Vault contents", Action::Nav(Screen::VaultContents))
        } else {
            ("Wallets", Action::OpenWallet(b.wallet))
        }),
        heading: &heading,
        guided: app.guided,
        switch: checklist,
        note: None,
        // From Create the wallet is the one just made, and the way on is
        // its card.
        chip: if b.from_create {
            Some("Then: Wallets")
        } else {
            many.then_some(wallet.name.as_str())
        },
        chip_tap: (many && !b.from_create).then_some(Action::BWallets),
    };
    let scroll = b.scroll;
    ui.chip_at = None;
    let all_done = checklist
        && steps
            .iter()
            .all(|&k| crate::bstep::item(k).is_some_and(|it| app.backup_item_done(it)));
    let (next, again) = {
        let app_ref: &Faraday = app;
        let mut body = |ui: &mut Ui, i: usize, x: f32, y: f32, w: f32| -> f32 {
            if checklist {
                backup_body(app_ref, ui, steps[i], x, y, w)
            } else {
                plan_body(app_ref, ui, &steps, i, x, y, w)
            }
        };
        let mut foot = |ui: &mut Ui, x: f32, y: f32, w: f32| -> f32 {
            if !checklist {
                // Held, as on the last question: the line says by what.
                let held = match (app_ref.backup_stickless(), app_ref.backup_kept_nowhere()) {
                    (Some(v), _) => Some(format!(
                        "Vault {}'s {} needs a place",
                        v + 1,
                        app_ref.medium.noun()
                    )),
                    (None, line) => line,
                };
                if let Some(line) = held {
                    return ui.wrap(x, y, w, 13.0, W::S, WARN, &line) + 12.0;
                }
                return wrap_buttons(
                    ui,
                    x,
                    y,
                    w,
                    40.0,
                    &[("Make the checklist", Style::Primary, Action::BChecklist)],
                );
            }
            if all_done {
                backup_done(app_ref, ui, x, y, w)
            } else {
                wrap_buttons(
                    ui,
                    x,
                    y,
                    w,
                    40.0,
                    &[("Change the plan", Style::Secondary, Action::BPlan)],
                )
            }
        };
        flow::column_foot(ui, &col, &cards, scroll, &mut body, Some(&mut foot))
    };
    if let Some(b) = app.backup.as_mut() {
        b.scroll = next;
    }
    if again {
        app.dirty = true;
        app.commands.push_back(osk_shell_api::Command::Draw);
    }
    if !compact {
        backup_panel(app, ui, x0 + cw - panel_w, panel_w, h);
    }
    if pick && let Some((cx, cy, chip_w)) = ui.chip_at {
        let right = if compact { x0 + cw } else { x0 + cw - panel_w } - 16.0;
        wallet_pick(app, ui, cx, cy + 6.0, chip_w, right);
    }
}

/// The open question of the backup's plan: its multi-choice lists, then
/// Continue, or on the last question the way to the checklist. Returns
/// its height.
fn plan_body(app: &Faraday, ui: &mut Ui, qs: &[u8], i: usize, x: f32, y: f32, w: f32) -> f32 {
    use crate::plan::{Preset, wallet as pw};
    use crate::{qrow, qstep};
    let Some(b) = app.backup.as_ref() else {
        return 0.0;
    };
    let shape = app.plan_shape(b.wallet);
    let a = &b.answers;
    let q = qs[i];
    let mut cy = y;
    let rows = |list: u8, labels: &[&str], on: &[bool]| -> Vec<(String, bool, bool, Action)> {
        labels
            .iter()
            .zip(on)
            .enumerate()
            .map(|(r, (l, o))| (l.to_string(), *o, true, Action::BAnswer(list, r as u8)))
            .collect()
    };
    match q {
        qstep::PRESET => {
            // F1's one line of fact, where a key made here waits for this
            // backup (`docs/NEW-WALLET.md` §14.5).
            if !app.wallet_held(b.wallet).is_empty() {
                cy += ui.wrap(
                    x,
                    cy,
                    w,
                    13.0,
                    W::R,
                    MUTED,
                    "A key made here signs only once this backup is done.",
                ) + 12.0;
            }
            for (k, p) in Preset::ALL.iter().enumerate() {
                ui.button(
                    x,
                    cy,
                    Some(w),
                    40.0,
                    p.name(),
                    Style::Secondary,
                    Action::BPreset(k as u8),
                );
                cy += 48.0;
            }
            return cy - y;
        }
        qstep::SEEDS => {
            cy += ui.multi_list(x, cy, w, &rows(qrow::SEEDS, &SEED_ROWS, &a.seeds));
            // A section per vault: the seeds here, more than one may be
            // ticked into one.
            if a.seeds[crate::plan::seeds::VAULT] && !shape.watch_only() {
                for (v, row) in a.vaults.iter().enumerate() {
                    cy += 12.0;
                    section_label(ui, x, cy, &format!("Vault {}", v + 1));
                    cy += 28.0;
                    let list = qrow::VAULTS.saturating_add(v.min(63) as u8);
                    let items: Vec<(String, bool, bool, Action)> = shape
                        .here()
                        .into_iter()
                        .map(|i| {
                            (
                                shape.seeds[i].name.clone(),
                                row.get(i) == Some(&true),
                                true,
                                Action::BAnswer(list, i.min(255) as u8),
                            )
                        })
                        .collect();
                    cy += ui.multi_list(x, cy, w, &items);
                }
            }
        }
        qstep::PLACES => {
            let p = a.places;
            stepper(
                ui,
                x,
                cy,
                w,
                "Places with paper",
                p,
                Action::BAnswer(qrow::PLACES, p.saturating_sub(1) as u8),
                Action::BAnswer(qrow::PLACES, (p + 1).min(255) as u8),
                true,
            );
            cy += 52.0;
            if shape.splits && a.wallet[pw::PAPER] {
                section_label(ui, x, cy, "Each place keeps");
                cy += 28.0;
                let list = ui.multi_list(
                    x,
                    cy,
                    w,
                    &[
                        (
                            "The whole wallet sheet".to_string(),
                            !a.split,
                            true,
                            Action::BAnswer(qrow::SPLIT, 0),
                        ),
                        (
                            "Its own share".to_string(),
                            a.split,
                            true,
                            Action::BAnswer(qrow::SPLIT, 1),
                        ),
                    ],
                );
                // Beside Its own share, drawn over its row.
                let label = "What is a share?";
                let bw = ui.measure(13.0, W::S, label) + 24.0;
                let rh = list / 2.0;
                ui.button(
                    x + w - bw,
                    cy + rh + (rh - 32.0) / 2.0,
                    Some(bw),
                    32.0,
                    label,
                    Style::Ghost,
                    Action::LearnShares,
                );
                cy += list + 12.0;
                if a.split {
                    cy += shares_card(app, ui, x, cy, w, b.wallet, a.omit);
                }
            }
            let names: Vec<String> = (0..p).map(|k| app.place_name(k)).collect();
            let labels: Vec<&str> = names.iter().map(String::as_str).collect();
            for v in a.vaults_made(&shape) {
                section_label(
                    ui,
                    x,
                    cy,
                    &format!("Vault {}'s {}", v + 1, app.medium.noun()),
                );
                cy += 28.0;
                let list = qrow::STICKS.saturating_add(v.min(63) as u8);
                let on = a.sticks.get(v).cloned().unwrap_or_default();
                cy += ui.multi_list(x, cy, w, &rows(list, &labels, &on)) + 12.0;
            }
            cy += place_names(app, ui, x, cy, w);
        }
        qstep::WALLET => {
            let rows_ = description_rows(a.split && shape.splits, app.medium);
            let labels: Vec<&str> = rows_.iter().map(String::as_str).collect();
            cy += ui.multi_list(x, cy, w, &rows(qrow::WALLET, &labels, &a.wallet));
        }
        qstep::SOFTWARE => {
            cy +=
                ui.multi_list(x, cy, w, &rows(qrow::SOFTWARE, &SOFTWARE_ROWS, &a.software)) + 12.0;
            section_label(ui, x, cy, "Form");
            cy += 28.0;
            cy += ui.multi_list(
                x,
                cy,
                w,
                &rows(qrow::FORM, &["QR picture", "Text"], &a.form),
            );
        }
        qstep::PASSPHRASE => {
            for (s, seed) in shape.seeds.iter().enumerate() {
                if !(seed.here && seed.passphrase) {
                    continue;
                }
                section_label(ui, x, cy, &format!("Passphrase of {}", seed.name));
                cy += 28.0;
                let words: Vec<usize> = (0..a.places)
                    .filter(|&p| a.words_at(&shape, p).contains(&s))
                    .collect();
                let list = qrow::PASS.saturating_add(s.min(111) as u8);
                let mut items: Vec<(String, bool, bool, Action)> = (0..a.places)
                    .map(|p| {
                        let with = words.contains(&p);
                        let label = if with {
                            format!("{} · its words", app.place_name(p))
                        } else {
                            app.place_name(p)
                        };
                        (
                            label,
                            a.pass_at(s, p),
                            !with,
                            Action::BAnswer(list, p as u8),
                        )
                    })
                    .collect();
                if a.seeds[crate::plan::seeds::VAULT] {
                    items.push((
                        "In the vault, with its seed".to_string(),
                        a.pass_in_vault(s),
                        true,
                        Action::BAnswer(list, a.places as u8),
                    ));
                }
                cy += ui.multi_list(x, cy, w, &items) + 12.0;
            }
        }
        _ => {
            cy += map_rows(app, ui, x, cy, w, false);
        }
    }
    cy += 8.0;
    let last = i + 1 == qs.len();
    // In place of the way on, what holds it (`docs/NEW-WALLET.md` §14.2,
    // §14.4): a vault's stick at no place, a key made here kept nowhere.
    let stickless = app
        .backup_stickless()
        .filter(|_| q == qstep::PLACES || last);
    let nowhere = app.backup_kept_nowhere().filter(|_| last);
    let held = match (stickless, nowhere) {
        (Some(v), _) => Some(format!(
            "Vault {}'s {} needs a place",
            v + 1,
            app.medium.noun()
        )),
        (None, Some(line)) => Some(line),
        (None, None) => None,
    };
    if let Some(line) = held {
        // On a small panel, where the way on is pinned at the foot, the
        // line takes its place there, greyed.
        if ui.pinning {
            let on = if last {
                Action::BChecklist
            } else {
                Action::BQNext(q)
            };
            ui.pin = Some((line, Style::Disabled, on));
            return cy - y;
        }
        cy += ui.wrap(x, cy, w, 13.0, W::S, WARN, &line) + 12.0;
        return cy - y;
    }
    let drawn = if last {
        next_button(ui, x, cy, w, "Make the checklist", Action::BChecklist)
    } else {
        next_button(ui, x, cy, w, "Continue", Action::BQNext(q))
    };
    if drawn {
        cy += 48.0;
    }
    cy - y
}

/// The places' names, on the places question: typed and kept in the open
/// vault, or with none open, the way to one. Returns its height.
fn place_names(app: &Faraday, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    let Some(b) = app.backup.as_ref() else {
        return 0.0;
    };
    let mut cy = y;
    let v = app.vaults.current;
    let Some(open) = app.vaults.open.get(v) else {
        section_label(ui, x, cy, "Names");
        cy += 28.0;
        ui.text(x, cy, 13.0, W::R, MUTED, "Only in a vault");
        cy += 26.0;
        if let Some((label, a)) = app.vault_way(Screen::Backup) {
            cy += wrap_buttons(ui, x, cy, w, 36.0, &[(label.as_str(), Style::Secondary, a)]);
        }
        return cy - y;
    };
    let head = format!("Names, kept in {}", open.name);
    let head = ui.fit(13.0, W::S, &head, w);
    section_label(ui, x, cy, &head);
    cy += 28.0;
    for p in 0..b.answers.places {
        let typing = b.naming == Some(p);
        let name = b.names.get(p).map_or("", String::as_str);
        let bw = 80.0;
        let fw = w - bw - 8.0;
        ui.fill(x, cy, fw, 40.0, 8.0, BG);
        ui.stroke(
            x,
            cy,
            fw,
            40.0,
            8.0,
            if typing {
                ACCENT.with_alpha(110)
            } else {
                INNER
            },
        );
        let hint = name.is_empty();
        let shown = if hint {
            format!("Place {}", p + 1)
        } else {
            name.to_string()
        };
        let shown = ui.fit(14.0, W::R, &shown, fw - 28.0);
        let tw = ui.text_mid(
            x + 12.0,
            cy,
            40.0,
            14.0,
            W::R,
            if hint { DIM } else { TEXT },
            &shown,
        );
        if typing {
            ui.caret(
                x + 12.0 + if hint { 0.0 } else { tw + 1.0 },
                cy + 11.0,
                18.0,
            );
        }
        ui.hit(x, cy, fw, 40.0, Action::BName(p as u8));
        ui.button(
            x + fw + 8.0,
            cy,
            Some(bw),
            40.0,
            if typing { "Done" } else { "Name" },
            Style::Secondary,
            Action::BName(p as u8),
        );
        cy += 48.0;
    }
    cy - y
}

/// What a thing on the map is done by, in the checklist.
fn held_item(
    at: crate::plan::At,
    what: crate::plan::What,
    items: &[u8],
) -> Option<crate::plan::Item> {
    use crate::plan::{At, Item, What};
    let has = |it: Item| items.contains(&crate::bstep::of(it));
    let item = match (at, what) {
        (At::Away, _) | (_, What::Passphrase(_) | What::VaultStick(_)) => return None,
        (_, What::Words(i) | What::SeedQr(i)) => Item::Copy(i),
        (_, What::Sheet | What::Share(_)) => Item::Sheets,
        (At::Vault(v), _) => Item::Vault(v),
        (At::Software, _) if has(Item::ShowDescriptor) => Item::ShowDescriptor,
        (_, What::SeedFile(_)) => Item::SeedFiles,
        _ => Item::PublicFiles,
    };
    has(item).then_some(item)
}

/// The map: one box per place, then the vault, a stick of files, the
/// software and the seeds on their own devices, each listing what it
/// holds and its tag; in the checklist, each line marked by its item's
/// state. `boxed` draws each as a box, as the side panel does. Returns
/// its height.
fn map_rows(app: &Faraday, ui: &mut Ui, x: f32, y: f32, w: f32, boxed: bool) -> f32 {
    use crate::plan::{At, Tag};
    let Some(b) = app.backup.as_ref() else {
        return 0.0;
    };
    let shape = app.plan_shape(b.wallet);
    let checklist = b.stage == crate::BStage::Checklist;
    let items = if checklist {
        app.backup_items()
    } else {
        Vec::new()
    };
    let mut cy = y;
    for spot in crate::plan::map(&shape, &b.answers) {
        let name = match spot.at {
            At::Place(p) => app.place_name(p),
            At::Vault(v) => b.answers.vault_name(&shape, v),
            At::Files => format!("{} of files", app.medium.cap()),
            At::Software => "Watch-only software".to_string(),
            At::Away => "On its own device".to_string(),
        };
        let top = cy;
        let (ix, iw) = if boxed { (x + 12.0, w - 24.0) } else { (x, w) };
        if boxed {
            cy += 10.0;
        }
        let name = ui.fit(13.0, W::S, &name, iw);
        ui.text(ix, cy, 13.0, W::S, TEXT, &name);
        cy += 22.0;
        if spot.holds.is_empty() {
            ui.text(ix, cy, 12.0, W::R, DIM, "Nothing");
            cy += 20.0;
        }
        for (what, tag) in &spot.holds {
            let tone = match tag {
                Tag::Secret => WARN,
                Tag::Sealed => ACCENT,
                Tag::Public => MUTED,
            };
            let tw = ui.measure(11.0, W::R, tag.name());
            ui.text_right(ix + iw, cy, 18.0, 11.0, W::R, tone, tag.name());
            let mut lx = ix;
            if checklist {
                let state = held_item(spot.at, *what, &items).map(|it| app.backup_item_done(it));
                if let Some(done) = state {
                    ui.dot(ix + 4.0, cy + 9.0, 3.5, if done { OK } else { DIM });
                }
                lx += 14.0;
            }
            let label = ui.fit(
                12.0,
                W::R,
                &what.label(&shape, app.medium),
                ix + iw - lx - tw - 8.0,
            );
            ui.text_mid(lx, cy, 18.0, 12.0, W::R, TEXT, &label);
            cy += 20.0;
        }
        if boxed {
            cy += 6.0;
            ui.stroke(x, top, w, cy - top, 8.0, LINE);
        }
        cy += 8.0;
    }
    // Work an earlier plan did that this one dropped, as it is: the copy
    // exists until it is destroyed (`docs/NEW-WALLET.md` §14.4).
    for line in &b.extras {
        let top = cy;
        let (ix, iw) = if boxed { (x + 12.0, w - 24.0) } else { (x, w) };
        if boxed {
            cy += 10.0;
        }
        let tag = crate::plan::Tag::Sealed.name();
        let tw = ui.measure(11.0, W::R, tag);
        ui.text_right(ix + iw, cy, 18.0, 11.0, W::R, ACCENT, tag);
        cy += ui
            .wrap(ix, cy, iw - tw - 8.0, 12.0, W::R, TEXT, line)
            .max(18.0)
            + 2.0;
        if boxed {
            cy += 6.0;
            ui.stroke(x, top, w, cy - top, 8.0, LINE);
        }
        cy += 8.0;
    }
    if !boxed {
        cy += 4.0;
        cy += check_rows(app, ui, x, cy, w);
    }
    cy - y
}

/// The check under the map: its three lines, each with its value.
/// Returns its height.
fn check_rows(app: &Faraday, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    let Some(b) = app.backup.as_ref() else {
        return 0.0;
    };
    let shape = app.plan_shape(b.wallet);
    let c = crate::plan::check(&shape, &b.answers);
    let mut h = crate::glance::check_lines(ui, x, y, w, c, false);
    // Why "Any one place lost" reads No (`docs/NEW-WALLET.md` §14.2).
    let name = |p: usize| app.place_name(p);
    if let Some(why) = crate::plan::lost_why(&shape, &b.answers, &name, app.medium.noun()) {
        h += ui.wrap(x, y + h, w, 12.0, W::R, ERR, &why) + 8.0;
    }
    h
}

/// The loaded wallets, one row each, under the backup's wallet chip at
/// (x, y): a press on one starts its backup, a press beside the list
/// closes it.
fn wallet_pick(app: &Faraday, ui: &mut Ui, x: f32, y: f32, chip_w: f32, right: f32) {
    let current = app.backup.as_ref().map_or(0, |b| b.wallet);
    let lw = 340.0_f32.max(chip_w).min(right - x);
    let rh = 44.0;
    let n = app.session.wallets.len();
    let lh = n as f32 * rh + 8.0;
    ui.hit_around(x, y, lw, lh, Action::BWallets);
    ui.shadow(x, y, lw, lh, 12.0);
    ui.fill(x, y, lw, lh, 12.0, SURFACE);
    ui.stroke(x, y, lw, lh, 12.0, BORDER);
    for (k, wl) in app.session.wallets.iter().enumerate() {
        let ry = y + 4.0 + k as f32 * rh;
        let (fg, weight) = if k == current {
            (ACCENT, W::S)
        } else {
            (TEXT, W::R)
        };
        // The name first, up to half the row; the shape in what is left.
        let nw = ui.measure(14.0, weight, &wl.name).min(lw / 2.0);
        let name = ui.fit(14.0, weight, &wl.name, nw);
        let shape = ui.fit(12.0, W::R, &Session::shape(wl), lw - nw - 48.0);
        ui.text_mid(x + 14.0, ry, rh, 14.0, weight, fg, &name);
        ui.text_right(x + lw - 14.0, ry, rh, 12.0, W::R, MUTED, &shape);
        ui.hit(x + 4.0, ry, lw - 8.0, rh, Action::Backup(k));
    }
}

/// Under the backup's cards once every one is done
/// (`docs/NEW-WALLET.md` §14.6): the plan's three checks as a strip, the
/// wallet's chart of the plan just completed, what waits for the stick,
/// and Write to a stick, Open the wallet, Change the plan. Returns its
/// height.
fn backup_done(app: &Faraday, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    use crate::glance::{self, Backup, Direction};
    let Some(b) = app.backup.as_ref() else {
        return 0.0;
    };
    let mut cy = y;
    ui.icon(x, cy, 24.0, Icon::Success, 13.0, OK);
    ui.text_mid(x + 32.0, cy, 24.0, 15.0, W::S, OK, "Backup done");
    cy += 40.0;
    if let Some(g) = glance::of(app, b.wallet) {
        if let Backup::Plan { check, .. } = &g.backup {
            cy += glance::check_lines(ui, x, cy, w, *check, true) + 16.0;
        }
        cy += if ui.compact {
            glance::draw_column_with(ui, &g, Direction::WalletFirst, x, cy, w, false)
        } else {
            glance::draw_with(ui, &g, Direction::WalletFirst, x, cy, w, false)
        };
        cy += 16.0;
    }
    let n = app.outbox.len();
    let line = format!(
        "{n} {} {} · {} from this backup",
        if n == 1 { "file" } else { "files" },
        app.medium.for_the(),
        b.sent.len()
    );
    cy += ui.wrap(x, cy, w, 13.0, W::R, MUTED, &line) + 12.0;
    let write_style = if n == 0 {
        Style::Disabled
    } else {
        Style::Primary
    };
    let write = format!("Write to {}", app.medium.a());
    let mut items: Vec<(&str, Style, Action)> = Vec::new();
    // Pinned at the foot on a small panel.
    if ui.pinning {
        ui.pin = Some((write.clone(), write_style, Action::WriteAsk));
    } else {
        items.push((write.as_str(), write_style, Action::WriteAsk));
    }
    items.push((
        "Open the wallet",
        Style::Secondary,
        Action::OpenWallet(b.wallet),
    ));
    items.push(("Change the plan", Style::Secondary, Action::BPlan));
    cy += wrap_buttons(ui, x, cy, w, 40.0, &items);
    cy - y
}

/// Buttons laid left to right, wrapping to a new row at the width.
/// Returns the height they took, a row being 44.
pub(crate) fn button_rows(
    ui: &mut Ui,
    x: f32,
    y: f32,
    w: f32,
    items: &[(String, Style, Action)],
) -> f32 {
    let (mut bx, mut by) = (x, y);
    for (label, style, action) in items {
        // No wider than the row; a long label is cut.
        let bw = (ui.measure(13.0, W::S, label) + 32.0).min(w);
        if bx > x && bx + bw > x + w {
            bx = x;
            by += 40.0;
        }
        let label = ui.fit(13.0, W::S, label, bw - 24.0);
        bx += ui.button(bx, by, Some(bw), 32.0, &label, *style, *action) + 6.0;
    }
    by - y + 44.0
}

/// `button_rows`, except a label too wide for the row even alone is
/// never cut (`docs/DESIGN.md` "Menu row"): it takes the row whole and
/// wraps onto further lines, its chip growing to hold them. Used where
/// `button_rows`'s "a long label is cut" must not hold — the Learn
/// sheet's tabs, whose first row names a label the spec gives in full
/// (`docs/SIMPLIFY.md` §6.2).
pub(crate) fn learn_tabs(
    ui: &mut Ui,
    x: f32,
    y: f32,
    w: f32,
    items: &[(String, Style, Action)],
) -> f32 {
    let (mut bx, mut by) = (x, y);
    for (label, style, action) in items {
        let natural = ui.measure(13.0, W::S, label) + 32.0;
        if natural <= w {
            if bx > x && bx + natural > x + w {
                bx = x;
                by += 40.0;
            }
            bx += ui.button(bx, by, Some(natural), 32.0, label, *style, *action) + 6.0;
        } else {
            if bx > x {
                bx = x;
                by += 40.0;
            }
            let lines = ui.wrap_lines(13.0, W::S, label, w - 24.0).max(1);
            let line_h = ui.line(13.0, W::S);
            let bh = 32.0 + (lines - 1) as f32 * line_h;
            // An empty-labelled button draws this style's background,
            // border and hit region exactly as any other; the label is
            // then wrapped over it, since `Ui::button` itself only ever
            // draws one line.
            ui.button(bx, by, Some(w), bh, "", *style, *action);
            let fg = match style {
                Style::Primary => ON_ACCENT,
                Style::Secondary => TEXT,
                Style::Ghost => MUTED,
                Style::Disabled => DIM,
            };
            let top = by + (bh - lines as f32 * line_h) / 2.0;
            ui.wrap(bx + 12.0, top, w - 24.0, 13.0, W::S, fg, label);
            by += bh + 6.0;
            bx = x;
        }
    }
    by - y + 44.0
}

/// Buttons `h` high laid left to right, wrapping to a new row at the
/// width, as a narrow column needs. Returns the height they took and the
/// gap under them.
pub(crate) fn wrap_buttons(
    ui: &mut Ui,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    items: &[(&str, Style, Action)],
) -> f32 {
    let size = if h >= 38.0 { 14.0 } else { 13.0 };
    let (mut bx, mut by) = (x, y);
    for &(label, style, action) in items {
        let bw = (ui.measure(size, W::S, label) + 32.0).min(w);
        if bx > x && bx + bw > x + w {
            bx = x;
            by += h + 8.0;
        }
        let label = ui.fit(size, W::S, label, bw - 24.0);
        bx += ui.button(bx, by, Some(bw), h, &label, style, action) + 8.0;
    }
    by - y + h + 8.0
}

/// The page's forward action, `h` high, `w` wide or as wide as its
/// label: on a small panel pinned at its foot instead. Returns whether it
/// was drawn here, so the caller makes room for it only then.
#[allow(clippy::too_many_arguments)]
#[must_use]
pub(crate) fn pin_button(
    ui: &mut Ui,
    x: f32,
    y: f32,
    w: Option<f32>,
    h: f32,
    label: &str,
    style: Style,
    action: Action,
) -> bool {
    if ui.pinning {
        ui.pin = Some((label.to_string(), style, action));
        return false;
    }
    ui.button(x, y, w, h, label, style, action);
    true
}

/// A step's next, at the column's right end; on a small panel pinned at
/// its foot instead. Returns whether it was drawn here, so the caller
/// makes room for it only then.
#[must_use]
pub(crate) fn next_button(
    ui: &mut Ui,
    x: f32,
    y: f32,
    w: f32,
    label: &str,
    action: Action,
) -> bool {
    if ui.pinning {
        ui.pin = Some((label.to_string(), Style::Primary, action));
        return false;
    }
    let bw = ui.measure(14.0, W::S, label) + 36.0;
    ui.button(x + w - bw, y, Some(bw), 40.0, label, Style::Primary, action);
    true
}

/// Buttons on the left and the step's next on the right, on one row; on
/// a small panel too narrow for both, the next goes on a row of its own
/// under them. Returns the height used.
pub(crate) fn buttons_and_next(
    ui: &mut Ui,
    x: f32,
    y: f32,
    w: f32,
    left: &[(&str, Style, Action)],
    next: Option<(&str, Action)>,
) -> f32 {
    // Pinned at the foot: only the other buttons are here.
    let next = match next {
        Some((label, action)) if ui.pinning => {
            ui.pin = Some((label.to_string(), Style::Primary, action));
            None
        }
        n => n,
    };
    if left.is_empty() && next.is_none() {
        return 0.0;
    }
    let nw = next.map_or(0.0, |(l, _)| ui.measure(14.0, W::S, l) + 36.0);
    if ui.compact {
        // The buttons wrap to the panel's width, each cut to fit; the
        // next goes on their last row when there is room, else under it.
        let (mut bx, mut by) = (x, y);
        for &(label, style, action) in left {
            let bw = (ui.measure(14.0, W::S, label) + 32.0).min(w);
            if bx > x && bx + bw > x + w {
                bx = x;
                by += 48.0;
            }
            let label = ui.fit(14.0, W::S, label, bw - 24.0);
            bx += ui.button(bx, by, Some(bw), 40.0, &label, style, action) + 6.0;
        }
        if next.is_some() && !left.is_empty() && bx + nw + 2.0 > x + w {
            by += 48.0;
        }
        if let Some((label, action)) = next {
            let _ = next_button(ui, x, by, w, label, action);
        }
        return by - y + 48.0;
    }
    let lw: f32 = left
        .iter()
        .map(|(l, ..)| ui.measure(14.0, W::S, l) + 32.0 + 6.0)
        .sum();
    let mut bx = x;
    for &(label, style, action) in left {
        bx += ui.button(bx, y, None, 40.0, label, style, action) + 6.0;
    }
    let below = !left.is_empty() && lw + nw + 8.0 > w;
    let ny = if below { y + 48.0 } else { y };
    if let Some((label, action)) = next {
        let _ = next_button(ui, x, ny, w, label, action);
    }
    ny - y + 48.0
}

/// A labelled count with − and + beside it. On a small panel the buttons
/// sit at the row's right end.
#[allow(clippy::too_many_arguments)]
pub(crate) fn stepper(
    ui: &mut Ui,
    x: f32,
    y: f32,
    w: f32,
    label: &str,
    value: usize,
    minus: Action,
    plus: Action,
    enabled: bool,
) {
    let style = if enabled {
        Style::Secondary
    } else {
        Style::Disabled
    };
    let bx = if ui.compact { x + w - 124.0 } else { x + 180.0 };
    ui.text_mid(x, y, 40.0, 13.0, W::R, MUTED, label);
    ui.button(bx, y, Some(40.0), 40.0, "-", style, minus);
    let v = value.to_string();
    let vw = ui.measure(18.0, W::S, &v);
    ui.text_mid(bx + 62.0 - vw / 2.0, y, 40.0, 18.0, W::S, TEXT, &v);
    ui.button(bx + 84.0, y, Some(40.0), 40.0, "+", style, plus);
}

/// A multisig's shares: **Keys left off each share** on a slider from 0
/// to the most the quorum allows, the shares and their keys'
/// fingerprints, and what the split audit finds. The plan's Places
/// question and the checklist's Shares card both draw it. Returns the
/// height used.
fn shares_card(app: &Faraday, ui: &mut Ui, x: f32, y: f32, w: f32, i: usize, omit: usize) -> f32 {
    let Some(wallet) = app.session.wallets.get(i) else {
        return 0.0;
    };
    let (m, keys_n) = Session::quorum(wallet);
    let most = m.saturating_sub(1).min(keys_n.saturating_sub(1));
    let omit = omit.min(most);
    let mut cy = y;
    let label = "Keys left off each share";
    if most == 0 {
        ui.text_mid(x, cy, 36.0, 13.0, W::R, MUTED, label);
        ui.text_mid(x + w - 20.0, cy, 36.0, 14.0, W::S, TEXT, "0");
        cy += 44.0;
    } else {
        // On a small panel the slider goes under the label.
        let (sx, sw) = if ui.compact {
            ui.text(x, cy, 13.0, W::S, MUTED, label);
            cy += 20.0;
            (x, w)
        } else {
            ui.text_mid(x, cy, 40.0, 13.0, W::R, MUTED, label);
            (x + 190.0, (w - 190.0).min(320.0))
        };
        let top = u8::try_from(most).unwrap_or(u8::MAX);
        let at = u8::try_from(omit).unwrap_or(top);
        cy += ui.slider(sx, cy, sw, crate::seeds::SLIDE_OMIT, 0, top, at) + 4.0;
    }
    let plan = crate::backup::split_plan(keys_n, m, omit);
    let slots = app.session.slots(wallet);
    for (k, row) in plan.iter().enumerate() {
        let fps: Vec<String> = row
            .iter()
            .map(|&j| crate::backup::fp_of(slots.get(j).and_then(|s| s.fingerprint)))
            .collect();
        ui.text(x, cy, 12.0, W::R, MUTED, &format!("Share {}", k + 1));
        // On a narrow panel a share's keys go over more than one line.
        let gap = ui.measure(13.0, W::M, "  ");
        let mut fx = x + 70.0;
        for fp in &fps {
            let fw = ui.measure(13.0, W::M, fp);
            if fx > x + 70.0 && fx + fw > x + w {
                fx = x + 70.0;
                cy += 20.0;
            }
            ui.text(fx, cy, 13.0, W::M, TEXT, fp);
            fx += fw + gap;
        }
        cy += 24.0;
    }
    let a = crate::backup::audit(&plan, m);
    cy += 8.0;
    let lines = [
        (
            if a.every_quorum_rebuilds {
                format!("Any {m} shares rebuild the wallet")
            } else {
                format!("Some {m} shares do not rebuild the wallet")
            },
            a.every_quorum_rebuilds,
        ),
        (
            format!("The fewest shares that rebuild it: {}", a.smallest_rebuild),
            true,
        ),
        (format!("Each key is on {} shares", a.copies_per_key), true),
        (
            if a.keys_per_sheet < keys_n {
                "No single share holds every key".to_string()
            } else {
                "Each share holds every key".to_string()
            },
            a.keys_per_sheet < keys_n,
        ),
    ];
    for (l, ok) in lines.iter() {
        cy += ui.wrap(x, cy, w, 13.0, W::R, if *ok { OK } else { WARN }, l) + 4.0;
    }
    cy + 10.0 - y
}

fn backup_body(app: &Faraday, ui: &mut Ui, n: u8, x: f32, y: f32, w: f32) -> f32 {
    use crate::bstep;
    let b = app.backup.as_ref().expect("backup");
    let wallet = &app.session.wallets[b.wallet];
    let mut cy = y;
    match n {
        bstep::BLANK => {
            ui.text_mid(x, cy, 38.0, 13.0, W::R, MUTED, "Words on the seed");
            let (mut bx, bw) = if ui.compact {
                (x + w - 112.0, 52.0)
            } else {
                (x + 140.0, 64.0)
            };
            for words in [12usize, 24] {
                let style = if b.words == words {
                    Style::Primary
                } else {
                    Style::Secondary
                };
                bx += ui.button(
                    bx,
                    cy,
                    Some(bw),
                    38.0,
                    &words.to_string(),
                    style,
                    Action::BWords(words),
                ) + 8.0;
            }
            cy += 52.0;
            // Made For the stick with the checklist, and again for
            // another length.
            let file = format!("blank-template-{}-words.pdf", b.words);
            let (state, button) = made_state(
                app,
                &file,
                Some(Action::PublicRemove(b.wallet, 0)),
                ("Make it", Action::BOut(0)),
            );
            cy += file_row(
                ui,
                x,
                cy,
                w,
                "Blank template",
                &format!("{} words · a PDF to print", b.words),
                state.as_deref(),
                button,
                &[],
            );
            cy += 12.0;
            if item_next(app, ui, x, cy, w, n) {
                cy += 48.0;
            }
        }
        n if bstep::is_copy(n) => {
            let keys = app.backup_keys(b.wallet);
            if keys.is_empty() {
                ui.wrap(
                    x,
                    cy,
                    w,
                    13.0,
                    W::R,
                    MUTED,
                    "No key of this wallet was loaded from words in this session",
                );
                cy += 30.0;
                // The small panel has no side panel: where each is kept
                // is on this page.
                if ui.compact {
                    cy += kept_rows(app, ui, x, cy, w);
                }
                let drawn = item_next(app, ui, x, cy, w, n);
                return cy + (if drawn { 48.0 } else { 8.0 }) - y;
            }
            let key = &app.session.keys[b.key];
            let words = key.words.as_ref().map(|z| z.as_str()).unwrap_or("");
            let list: Vec<&str> = words.split_whitespace().collect();
            let bw = ui.button(
                x,
                cy,
                None,
                38.0,
                if b.reveal {
                    "Hide the seed"
                } else {
                    "Show the seed"
                },
                if b.reveal {
                    Style::Secondary
                } else {
                    Style::Primary
                },
                Action::BReveal,
            );
            // Beside the button; on a small panel under it.
            let (ix, iy) = if ui.compact {
                (x, cy + 44.0)
            } else {
                (x + bw + 14.0, cy)
            };
            let info = format!(
                "{} · {} words",
                key_line(key.master.fingerprint(), &key.label),
                list.len()
            );
            let info = ui.fit(13.0, W::R, &info, x + w - ix);
            ui.text_mid(ix, iy, 38.0, 13.0, W::R, MUTED, &info);
            cy = iy + 52.0;
            if b.reveal {
                // The words, with each word's index as SeedQR writes it.
                let lang = key.language;
                // Two columns on a small panel, the index at each one's end.
                let cols = if ui.compact { 2 } else { 4 };
                let colw = w / cols as f32;
                let rows = list.len().div_ceil(cols);
                for (i, wd) in list.iter().enumerate() {
                    let cx = x + (i / rows) as f32 * colw;
                    let ry = cy + (i % rows) as f32 * 26.0;
                    let idx = lang
                        .index_of(wd)
                        .map(|v| format!("{v:04}"))
                        .unwrap_or_default();
                    ui.text(cx, ry, 12.0, W::R, DIM, &format!("{:>2}", i + 1));
                    if ui.compact {
                        ui.text(cx + 20.0, ry, 13.0, W::M, TEXT, wd);
                        ui.text_right(cx + colw - 8.0, ry - 3.0, 20.0, 11.0, W::M, MUTED, &idx);
                    } else {
                        ui.text(cx + 26.0, ry, 14.0, W::M, TEXT, wd);
                        ui.text(cx + 104.0, ry, 12.0, W::M, MUTED, &idx);
                    }
                }
                cy += rows as f32 * 26.0 + 16.0;
                // SeedQR numbers the English list's words: another list's
                // words have no SeedQR another signer would read right.
                if lang != osk_bip::bip39::Language::English {
                    let lh = ui.wrap(
                        x,
                        cy,
                        w,
                        13.0,
                        W::R,
                        MUTED,
                        "SeedQR is for English words only: copy these words by hand",
                    );
                    let cy = cy + lh + 12.0;
                    let cy = cy + paper_section(app, ui, x, cy, w);
                    let cy = cy + copy_check(app, ui, x, cy, w);
                    let cy = if ui.compact {
                        cy + 8.0 + kept_rows(app, ui, x, cy + 8.0, w)
                    } else {
                        cy
                    };
                    let drawn = item_next(app, ui, x, cy, w, n);
                    return cy + (if drawn { 48.0 } else { 8.0 }) - y;
                }
                // The SeedQR as a ruled grid to copy.
                let items: Vec<(&str, Style, Action)> =
                    [("Standard SeedQR", false), ("Compact SeedQR", true)]
                        .into_iter()
                        .map(|(label, c)| {
                            let style = if b.compact == c {
                                Style::Primary
                            } else {
                                Style::Secondary
                            };
                            (label, style, Action::BCompact(c))
                        })
                        .collect();
                cy += wrap_buttons(ui, x, cy, w, 34.0, &items) + 6.0;
                if let Ok(mn) = osk_bip::bip39::Mnemonic::parse(lang, words) {
                    let matrix = if b.compact {
                        osk_codec::seedqr::encode_compact(&mn)
                    } else {
                        osk_codec::seedqr::encode_seedqr(&mn)
                    };
                    if let Ok(mx) = matrix {
                        cy += seed_grid(ui, &mx, x, cy, w.min(560.0), b.pin) + 12.0;
                    }
                }
                cy += paper_section(app, ui, x, cy, w);
            }
            // The copy checked: scanned, or its words' numbers typed back;
            // the foot's Check my copy starts it.
            cy += copy_check(app, ui, x, cy, w);
            if ui.compact {
                cy += 8.0;
                cy += kept_rows(app, ui, x, cy, w);
            }
            if item_next(app, ui, x, cy, w, n) {
                cy += 48.0;
            }
        }
        bstep::PUBLIC => {
            cy += public_rows_for(app, ui, x, cy, w, b.wallet, Some(&app.backup_public()));
            cy += 12.0;
            if item_next(app, ui, x, cy, w, n) {
                cy += 48.0;
            }
        }
        n if n >= bstep::VAULT => {
            cy += vault_item(app, ui, x, cy, w, usize::from(n - bstep::VAULT));
            cy += 8.0;
            if item_next(app, ui, x, cy, w, n) {
                cy += 48.0;
            }
        }
        bstep::FILES => {
            cy += seed_files(app, ui, x, cy, w);
            cy += 8.0;
            if item_next(app, ui, x, cy, w, n) {
                cy += 48.0;
            }
        }
        bstep::SHOW => {
            let shown = if b.shown { "Shown" } else { "Not shown" };
            ui.text_mid(
                x,
                cy,
                38.0,
                13.0,
                W::S,
                if b.shown { OK } else { MUTED },
                shown,
            );
            cy += 50.0;
            if item_next(app, ui, x, cy, w, n) {
                cy += 48.0;
            }
        }
        bstep::SHEETS if !(b.answers.split && crate::backup::splits(wallet)) => {
            let stem = crate::file_stem(&wallet.name);
            let file = format!("{stem}-backup.pdf");
            let (state, button) = made_state(
                app,
                &file,
                Some(Action::PublicRemove(b.wallet, 3)),
                ("Make it", Action::BOut(3)),
            );
            cy += file_row(
                ui,
                x,
                cy,
                w,
                "Backup sheet",
                "A PDF to print",
                state.as_deref(),
                button,
                &[],
            );
            cy += 12.0;
            if item_next(app, ui, x, cy, w, n) {
                cy += 48.0;
            }
        }
        bstep::SHEETS => {
            cy += shares_card(app, ui, x, cy, w, b.wallet, b.answers.omit);
            // Each share's sheet, text and picture, made For the stick
            // with the checklist; Remove takes them all.
            let stem = crate::file_stem(&wallet.name);
            let file = format!("{stem}-share-1-of-{}.pdf", Session::quorum(wallet).1);
            let (state, button) = made_state(
                app,
                &file,
                Some(Action::PublicRemove(b.wallet, 4)),
                ("Make them", Action::BOut(4)),
            );
            cy += file_row(
                ui,
                x,
                cy,
                w,
                "The shares",
                "Sheets to print, text files and pictures",
                state.as_deref(),
                button,
                &[],
            );
            cy += 12.0;
            if item_next(app, ui, x, cy, w, n) {
                cy += 48.0;
            }
        }
        _ => {
            // On a small panel the map is here, as a list of places.
            if ui.compact {
                cy += map_rows(app, ui, x, cy, w, false);
            } else {
                let shape = app.plan_shape(b.wallet);
                for spot in crate::plan::map(&shape, &b.answers) {
                    let crate::plan::At::Place(p) = spot.at else {
                        continue;
                    };
                    let head = ui.fit(13.0, W::S, &app.place_name(p), w);
                    ui.text(x, cy, 13.0, W::S, TEXT, &head);
                    cy += 22.0;
                    let holds: Vec<String> = spot
                        .holds
                        .iter()
                        .map(|(h, _)| h.label(&shape, app.medium))
                        .collect();
                    let line = if holds.is_empty() {
                        "Nothing".to_string()
                    } else {
                        holds.join(" · ")
                    };
                    cy += ui.wrap(x + 12.0, cy, w - 12.0, 12.0, W::R, MUTED, &line) + 10.0;
                }
            }
            cy += 10.0;
            cy += buttons_and_next(
                ui,
                x,
                cy,
                w,
                &[(
                    "Back to the wallet",
                    Style::Secondary,
                    Action::OpenWallet(b.wallet),
                )],
                Some(("Done", Action::BNext(n))),
            );
        }
    }
    cy - y
}

/// The SeedQR as a ruled grid to copy by hand: numbers every five
/// squares, the standard's fixed squares grey, the pinned row marked.
/// Returns its height.
/// The key's other paper forms: a Seed XOR split and codex32 shares,
/// shown to copy by hand like the words.
/// The copy of the seed on screen checked: what scanning it found, and
/// with no camera, once Check my copy is pressed, the numbers typed back
/// and how they compare. Returns its height.
fn copy_check(app: &Faraday, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    let Some(b) = app.backup.as_ref() else {
        return 0.0;
    };
    let Some(key) = app.session.keys.get(b.key) else {
        return 0.0;
    };
    let Some(words) = key.words.as_ref() else {
        return 0.0;
    };
    let Ok(mn) = osk_bip::bip39::Mnemonic::parse(key.language, words.as_str()) else {
        return 0.0;
    };
    let digits = osk_codec::seedqr::to_digits(&mn);
    let digits = digits.expose().as_bytes().to_vec();
    let mut cy = y;
    if let Some(found) = b.scanned.as_ref() {
        let c = if *found == crate::backup::CopyCheck::Matches {
            OK
        } else {
            ERR
        };
        let line = crate::backup::copy_scan_line(found);
        cy += ui.wrap(x, cy, w, 13.0, W::S, c, &line) + 12.0;
    }
    if !(b.checking || !b.typed.is_empty()) {
        return cy - y;
    }
    let hint = "Type the four-digit number beside each word, in order";
    cy += ui.wrap(x, cy, w, 12.0, W::R, MUTED, hint) + 10.0;
    let typed: String = b
        .typed
        .chars()
        .collect::<Vec<_>>()
        .chunks(4)
        .map(|c| c.iter().collect::<String>())
        .collect::<Vec<_>>()
        .join(" ");
    ui.fill(x, cy, w, 44.0, 8.0, BG);
    ui.stroke(
        x,
        cy,
        w,
        44.0,
        8.0,
        if b.checking {
            ACCENT.with_alpha(110)
        } else {
            INNER
        },
    );
    let empty = typed.is_empty();
    let shown = if empty {
        "Type here".to_string()
    } else {
        typed
    };
    let shown = ui.fit(14.0, W::M, &shown, w - 28.0);
    // The caret stands before the hint while nothing is typed, after the
    // digits once some are.
    let lead = if b.checking && empty { 6.0 } else { 0.0 };
    let tw = ui.text_mid(
        x + 14.0 + lead,
        cy,
        44.0,
        14.0,
        W::M,
        if empty { DIM } else { TEXT },
        &shown,
    );
    if b.checking {
        ui.caret(
            x + 14.0 + if empty { 0.0 } else { tw + 1.0 },
            cy + 13.0,
            18.0,
        );
    }
    cy += 54.0;
    let (line, c) = match crate::backup::check_copy(&b.typed, &digits) {
        crate::backup::CopyCheck::Matches => ("Your copy matches the seed".to_string(), OK),
        crate::backup::CopyCheck::WrongWord(k) => {
            (format!("Word {k} differs: check its four digits"), ERR)
        }
        crate::backup::CopyCheck::SoFar { typed, of } => (
            format!("{typed} of {of} digits typed · all match so far"),
            MUTED,
        ),
    };
    let lh = ui.wrap(x, cy, w - 88.0, 13.0, W::S, c, &line);
    ui.button(
        x + w - 80.0,
        cy - 8.0,
        Some(80.0),
        32.0,
        "Clear",
        Style::Ghost,
        Action::BCheckClear,
    );
    cy += lh.max(18.0) + 12.0;
    cy - y
}

/// A checklist item's next action (`docs/NEW-WALLET.md` §14.1): its own
/// until it is done, then Continue. A vault item: Make a vault (or the
/// way to one), then Save into the open vault, then Continue.
fn item_action(app: &Faraday, n: u8) -> (String, Action) {
    use crate::bstep;
    use crate::plan::Item;
    let item = bstep::item(n).unwrap_or(Item::Envelopes);
    let next = ("Continue".to_string(), Action::BNext(n));
    if item == Item::Envelopes || app.backup_item_done(item) {
        return next;
    }
    let Some(b) = app.backup.as_ref() else {
        return next;
    };
    let split = b.answers.split
        && app
            .session
            .wallets
            .get(b.wallet)
            .is_some_and(crate::backup::splits);
    match item {
        Item::Templates => ("Make it".to_string(), Action::BOut(0)),
        Item::Copy(_) if !app.cameras.is_empty() => ("Check my copy".to_string(), Action::BScan),
        Item::Copy(_) if b.checking => ("Stop typing".to_string(), Action::BCheck),
        Item::Copy(_) => ("Check my copy".to_string(), Action::BCheck),
        Item::Vault(v) => {
            let cur = app.vaults.current;
            match app.vaults.open.get(cur) {
                Some(_) if app.backup_vault_taken(v, cur) => (
                    "Lock it and make a new vault".to_string(),
                    Action::BNewVault,
                ),
                Some(open) => (
                    format!("Save into {}", open.name),
                    Action::BVaultSave(v.min(255) as u8),
                ),
                None => app.vault_way(Screen::Backup).unwrap_or(next),
            }
        }
        Item::SeedFiles => ("Make it".to_string(), Action::BFile),
        Item::Sheets if split => ("Make them".to_string(), Action::BOut(4)),
        Item::Sheets => ("Make it".to_string(), Action::BOut(3)),
        Item::PublicFiles => ("Make them".to_string(), Action::BFilesMake),
        Item::ShowDescriptor => ("Show as QR".to_string(), Action::QrWallet(b.wallet)),
        Item::Envelopes => next,
    }
}

/// The item's next action as its foot button. Returns whether it was
/// drawn in the column (on a small panel it is pinned instead).
fn item_next(app: &Faraday, ui: &mut Ui, x: f32, y: f32, w: f32, n: u8) -> bool {
    let (label, action) = item_action(app, n);
    let label = ui.fit(14.0, W::S, &label, w - 36.0);
    next_button(ui, x, y, w, &label, action)
}

/// A vault's checklist item: what Save into the open vault saves, a row
/// each, marked once the vault holds it; with the open vault holding
/// another vault's seed, that. Its one button is the foot's. Returns its
/// height.
fn vault_item(app: &Faraday, ui: &mut Ui, x: f32, y: f32, w: f32, v: usize) -> f32 {
    use crate::vault_screens::vault_has_wallet;
    let Some(b) = app.backup.as_ref() else {
        return 0.0;
    };
    let mut cy = y;
    let cur = app.vaults.current;
    let open = app.vaults.open.get(cur);
    if let Some(o) = open
        && app.backup_vault_taken(v, cur)
    {
        let line = format!("{} holds another vault's seed", o.name);
        let line = ui.fit(13.0, W::S, &line, w);
        ui.text(x, cy + 4.0, 13.0, W::S, WARN, &line);
        cy += 32.0;
    }
    let head = match open {
        Some(o) => format!("Into {}", o.name),
        None => "Into the vault".to_string(),
    };
    let head = ui.fit(13.0, W::S, &head, w);
    section_label(ui, x, cy, &head);
    cy += 28.0;
    let (own, _) = app.backup_vault_seeds(v);
    let mut rows: Vec<(String, bool)> = own
        .iter()
        .filter_map(|&(k, with)| {
            let key = app.session.keys.get(k)?;
            let name = fp_text(key.master.fingerprint());
            let label = if with {
                format!("Seed {name} with its passphrase")
            } else {
                format!("Seed {name}")
            };
            let has = open.is_some() && app.vault_holds_seed(cur, k, with);
            Some((label, has))
        })
        .collect();
    if b.answers.wallet[crate::plan::wallet::VAULT] {
        let has = open.is_some()
            && app
                .session
                .wallets
                .get(b.wallet)
                .is_some_and(|wl| vault_has_wallet(app, cur, wl));
        rows.push(("Wallet description".to_string(), has));
    }
    for (label, has) in rows {
        ui.dot(x + 4.0, cy + 9.0, 3.5, if has { OK } else { DIM });
        let label = ui.fit(13.0, W::R, &label, w - 18.0);
        ui.text_mid(x + 18.0, cy, 18.0, 13.0, W::R, TEXT, &label);
        cy += 24.0;
    }
    cy + 4.0 - y
}

/// The seeds' files, on their checklist item: a row per seed here,
/// marked once its file is made. The foot's Make it makes the next one.
/// Returns the height used.
fn seed_files(app: &Faraday, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    let Some(b) = app.backup.as_ref() else {
        return 0.0;
    };
    let Some(wallet) = app.session.wallets.get(b.wallet) else {
        return 0.0;
    };
    let stem = crate::file_stem(&wallet.name);
    let mut cy = y + 4.0;
    for k in app.backup_keys(b.wallet) {
        let key = &app.session.keys[k];
        let made = app.seed_file_made(&stem, key);
        ui.dot(x + 4.0, cy + 9.0, 3.5, if made { OK } else { DIM });
        let name = key_line(key.master.fingerprint(), &key.label);
        let state = if made {
            app.medium.for_box()
        } else {
            "Not made"
        };
        let sw = ui.measure(12.0, W::R, state);
        let name = ui.fit(13.0, W::S, &name, w - sw - 30.0);
        ui.text_mid(x + 18.0, cy, 18.0, 13.0, W::S, TEXT, &name);
        ui.text_right(
            x + w,
            cy,
            18.0,
            12.0,
            W::R,
            if made { OK } else { MUTED },
            state,
        );
        cy += 26.0;
    }
    cy + 4.0 - y
}

fn paper_section(app: &Faraday, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    use crate::paper::PaperForm;
    let Some(b) = app.backup.as_ref() else {
        return 0.0;
    };
    let mut cy = y + 8.0;
    section_label(ui, x, cy, "Other paper forms");
    cy += 28.0;
    match &b.paper {
        None => {
            let row: Vec<(String, Style, Action)> = vec![
                (
                    "Seed XOR, 2 parts".into(),
                    Style::Secondary,
                    Action::BXor(2),
                ),
                (
                    "Seed XOR, 3 parts".into(),
                    Style::Secondary,
                    Action::BXor(3),
                ),
                (
                    "codex32, 2 of 3".into(),
                    Style::Secondary,
                    Action::BCodex32(2, 3),
                ),
                (
                    "codex32, 3 of 5".into(),
                    Style::Secondary,
                    Action::BCodex32(3, 5),
                ),
            ];
            cy += button_rows(ui, x, cy, w, &row);
        }
        Some(form) => {
            let (lines, line) = match form {
                PaperForm::Xor(parts) => (
                    parts,
                    format!("{} parts · every one is needed", parts.len()),
                ),
                PaperForm::Codex32 { k, strings } => (
                    strings,
                    format!("{} shares · any {k} make the seed", strings.len()),
                ),
            };
            ui.text(x, cy, 13.0, W::S, WARN, &line);
            cy += 26.0;
            for (i, l) in lines.iter().enumerate() {
                ui.text(x, cy, 12.0, W::R, MUTED, &format!("{}", i + 1));
                cy += ui
                    .wrap(x + 28.0, cy, w - 28.0, 13.0, W::M, TEXT, l)
                    .max(18.0)
                    + 10.0;
            }
            ui.button(
                x,
                cy,
                None,
                34.0,
                "Put them away",
                Style::Ghost,
                Action::BPaperHide,
            );
            cy += 44.0;
        }
    }
    cy - y
}

fn seed_grid(ui: &mut Ui, m: &osk_codec::qr::QrMatrix, x: f32, y: f32, w: f32, pin: usize) -> f32 {
    let n = m.size();
    let margin = 26.0;
    let cell = ((w - margin) / n as f32).floor().max(6.0);
    let gx = x + margin;
    let gy = y + margin;
    let side = cell * n as f32;
    let pin = pin.min(n - 1);
    ui.fill(gx, gy, side, side, 0.0, osk_ui::Color::WHITE);
    ui.fill(
        gx,
        gy + pin as f32 * cell,
        side,
        cell,
        0.0,
        ACCENT.with_alpha(90),
    );
    for row in 0..n {
        for col in 0..n {
            if m.module(col, row) {
                let c = if crate::backup::structural(col, row, n) {
                    osk_ui::Color::rgb(0x8a, 0x94, 0x9e)
                } else {
                    osk_ui::Color::BLACK
                };
                ui.fill(
                    gx + col as f32 * cell,
                    gy + row as f32 * cell,
                    cell,
                    cell,
                    0.0,
                    c,
                );
            }
        }
        ui.hit(gx, gy + row as f32 * cell, side, cell, Action::BPin(row));
    }
    // The ruling: light every square, darker every five.
    for i in 0..=n {
        let c = if i % 5 == 0 {
            osk_ui::Color::rgb(0x6f, 0x82, 0x91)
        } else {
            osk_ui::Color::rgb(0xc8, 0xd0, 0xd6)
        };
        ui.fill(gx + i as f32 * cell, gy, 1.0, side, 0.0, c);
        ui.fill(gx, gy + i as f32 * cell, side, 1.0, 0.0, c);
        if i % 5 == 0 && i < n {
            ui.text(
                gx + i as f32 * cell + 2.0,
                y + 4.0,
                11.0,
                W::M,
                MUTED,
                &i.to_string(),
            );
            ui.text(
                x,
                gy + i as f32 * cell + 2.0,
                11.0,
                W::M,
                MUTED,
                &i.to_string(),
            );
        }
    }
    // Beside the pinned row; on a small panel, under the grid.
    if ui.compact {
        ui.text(
            gx,
            gy + side + 8.0,
            12.0,
            W::S,
            ACCENT,
            &format!("row {pin}"),
        );
        return margin + side + 34.0;
    }
    ui.text(
        gx + side + 10.0,
        gy + pin as f32 * cell - 3.0,
        12.0,
        W::S,
        ACCENT,
        &format!("row {pin}"),
    );
    margin + side + 10.0
}

/// The backup's side panel: the map, its boxes listing what each holds,
/// in the checklist each line marked by its item's state; the check at
/// its foot.
fn backup_panel(app: &Faraday, ui: &mut Ui, px: f32, pw: f32, h: f32) {
    if app.backup.is_none() {
        return;
    }
    ui.fill(px, 0.0, pw, h, 0.0, SURFACE);
    ui.fill(px, 0.0, 1.0, h, 0.0, LINE);
    let x = px + 22.0;
    let w = pw - 44.0;
    // The check is measured first, so the map stops above it.
    ui.c.push_clip(osk_ui::Rect::new(0, 0, 0, 0));
    let mark = ui.hits.len();
    let ch = check_rows(app, ui, x, 0.0, w);
    ui.hits.truncate(mark);
    ui.c.pop_clip();
    let foot = h - ch - 16.0;
    ui.text(x, 28.0, 13.0, W::S, MUTED, "Map");
    let clip = ui.rect(px, 52.0, pw, foot - 60.0);
    ui.c.push_clip(clip);
    map_rows(app, ui, x, 56.0, w, true);
    ui.c.pop_clip();
    ui.rule(x, foot - 4.0, w, INNER);
    check_rows(app, ui, x, foot + 8.0, w);
}

/// The colour of a line of [`crate::backup::Kept`].
fn tone(t: crate::backup::Tone) -> osk_ui::Color {
    use crate::backup::Tone;
    match t {
        Tone::Ok => OK,
        Tone::Warn => WARN,
        Tone::Err => ERR,
        Tone::Dim => DIM,
    }
}

/// Where the wallet and each of its seeds are kept: the wallet's line,
/// then "Seeds" and a row per key, its fingerprint and its lines. The
/// backup's side panel and the small panel's seeds page draw it. Returns
/// its height.
fn kept_rows(app: &Faraday, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    let Some(kept) = app.backup_kept() else {
        return 0.0;
    };
    let mut cy = y;
    let (line, t) = &kept.wallet;
    cy += ui.wrap(x, cy, w, 13.0, W::S, tone(*t), line) + 16.0;
    ui.text(x, cy, 13.0, W::S, MUTED, "Seeds");
    cy += 26.0;
    for seed in &kept.seeds {
        let name = ui.fit(13.0, W::M, &seed.name, w);
        ui.text(x, cy, 13.0, W::M, TEXT, &name);
        cy += 22.0;
        for (line, t) in &seed.lines {
            cy += ui.wrap(x + 12.0, cy, w - 12.0, 12.0, W::R, tone(*t), line) + 4.0;
        }
        cy += 8.0;
    }
    cy - y
}

// ---------------------------------------------------------------------
// Messages
// ---------------------------------------------------------------------

const MSTEPS: [&str; 4] = ["Address", "Message", "Format", "Signature"];

fn message_screen(app: &mut Faraday, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    use crate::mstep;
    let col_x = x0 + 40.0;
    let col_w = (cw - 80.0).min(880.0);
    let Some(m) = app.message.as_ref() else {
        return;
    };
    let address = m
        .wallet
        .and_then(|w| app.session.wallets.get(w))
        .map(|w| app.session.address_shown(w, false, m.index));
    let cards: Vec<flow::Card> = (0..4u8)
        .map(|k| flow::Card {
            title: MSTEPS[k as usize].to_string(),
            summary: match k {
                mstep::ADDRESS => address
                    .as_deref()
                    .map(short)
                    .unwrap_or_else(|| "None chosen".into()),
                mstep::TEXT => {
                    let t = m.text.lines().next().unwrap_or("");
                    if t.is_empty() {
                        "Not written yet".to_string()
                    } else {
                        format!("\"{t}\"")
                    }
                }
                mstep::FORMAT => (if m.bip322 { "BIP-322" } else { "BIP-137" }).to_string(),
                _ => {
                    if m.signed.is_some() {
                        "Signed".to_string()
                    } else {
                        "Not signed yet".to_string()
                    }
                }
            },
            mono: k == mstep::ADDRESS && address.is_some(),
            done: m.done[k as usize],
            open: m.open == Some(k),
            default: false,
            toggle: Action::MStep(k),
            guide: Some(guide::message(k)),
        })
        .collect();
    let col = flow::Column {
        area_x: x0,
        area_w: cw,
        x: col_x,
        w: col_w,
        h,
        back: Some(("Wallets", Action::Nav(Screen::Start))),
        heading: "Sign a message",
        guided: app.guided,
        switch: true,
        note: None,
        chip: None,
        chip_tap: None,
    };
    let scroll = m.scroll;
    let (next, again) = {
        let app_ref: &Faraday = app;
        let mut body = |ui: &mut Ui, i: usize, x: f32, y: f32, w: f32| -> f32 {
            message_body(app_ref, ui, i as u8, x, y, w)
        };
        flow::column(ui, &col, &cards, scroll, &mut body)
    };
    if let Some(m) = app.message.as_mut() {
        m.scroll = next;
    }
    if again {
        app.dirty = true;
        app.commands.push_back(osk_shell_api::Command::Draw);
    }
}

fn message_body(app: &Faraday, ui: &mut Ui, n: u8, x: f32, y: f32, w: f32) -> f32 {
    use crate::mstep;
    let m = app.message.as_ref().expect("message");
    let mut cy = y;
    match n {
        mstep::ADDRESS => {
            let mut bx = x;
            for wi in app.session.message_wallets() {
                let wl = &app.session.wallets[wi];
                let style = if m.wallet == Some(wi) {
                    Style::Primary
                } else {
                    Style::Secondary
                };
                bx += ui.button(bx, cy, None, 36.0, &wl.name, style, Action::MWallet(wi)) + 8.0;
            }
            cy += 48.0;
            if let Some(wl) = m.wallet.and_then(|wi| app.session.wallets.get(wi)) {
                ui.text_mid(
                    x,
                    cy,
                    36.0,
                    12.0,
                    W::R,
                    MUTED,
                    &format!("Receive address 0/{}", m.index),
                );
                ui.button(
                    x + 170.0,
                    cy,
                    Some(40.0),
                    36.0,
                    "-",
                    Style::Secondary,
                    Action::MIndex(-1),
                );
                ui.button(
                    x + 216.0,
                    cy,
                    Some(40.0),
                    36.0,
                    "+",
                    Style::Secondary,
                    Action::MIndex(1),
                );
                cy += 46.0;
                let a = grouped(&app.session.address_shown(wl, false, m.index));
                cy += ui.wrap(x, cy, w, 14.0, W::M, TEXT, &a) + 12.0;
            }
            if next_button(ui, x, cy, w, "Continue", Action::MNext(n)) {
                cy += 48.0;
            }
        }
        mstep::TEXT => {
            let box_h = 120.0;
            ui.fill(x, cy, w, box_h, 8.0, BG);
            ui.stroke(
                x,
                cy,
                w,
                box_h,
                8.0,
                if m.typing {
                    ACCENT.with_alpha(110)
                } else {
                    INNER
                },
            );
            let shown = if m.text.is_empty() {
                if m.typing {
                    "Type the message".to_string()
                } else {
                    "Press Type to write the message".to_string()
                }
            } else if m.typing && !ui.select_all {
                format!("{}{}", m.text, ui.caret_char())
            } else {
                m.text.clone()
            };
            ui.c.push_clip(ui.rect(x, cy, w, box_h));
            if m.typing && ui.select_all && !m.text.is_empty() {
                ui.fill(
                    x + 4.0,
                    cy + 4.0,
                    w - 8.0,
                    box_h - 8.0,
                    6.0,
                    ACCENT.with_alpha(40),
                );
            }
            ui.wrap(
                x + 14.0,
                cy + 12.0,
                w - 28.0,
                14.0,
                W::R,
                if m.text.is_empty() { DIM } else { TEXT },
                &shown,
            );
            ui.c.pop_clip();
            cy += box_h + 12.0;
            ui.button(
                x,
                cy,
                None,
                38.0,
                if m.typing { "Stop typing" } else { "Type" },
                Style::Secondary,
                Action::MType,
            );
            ui.text_mid(
                x + 120.0,
                cy,
                38.0,
                12.0,
                W::R,
                DIM,
                &format!("{} characters", m.text.chars().count()),
            );
            if next_button(ui, x, cy, w, "Continue", Action::MNext(n)) {
                cy += 48.0;
            }
        }
        mstep::FORMAT => {
            let script = m
                .wallet
                .and_then(|wi| app.session.wallets.get(wi))
                .and_then(|wl| match crate::wallet::Kind::of(&wl.policy) {
                    crate::wallet::Kind::Single(t) => Some(t),
                    _ => None,
                });
            let allowed = script
                .map(osk_psbt::message::Format::for_script)
                .unwrap_or(&[]);
            let mut bx = x;
            for (label, is322) in [("BIP-322", true), ("BIP-137", false)] {
                let f = if is322 {
                    osk_psbt::message::Format::Bip322
                } else {
                    osk_psbt::message::Format::Bip137
                };
                let style = if !allowed.contains(&f) {
                    Style::Disabled
                } else if m.bip322 == is322 {
                    Style::Primary
                } else {
                    Style::Secondary
                };
                bx += ui.button(
                    bx,
                    cy,
                    Some(110.0),
                    38.0,
                    label,
                    style,
                    Action::MFormat(is322),
                ) + 8.0;
            }
            cy += 52.0;
            if next_button(ui, x, cy, w, "Continue", Action::MNext(n)) {
                cy += 48.0;
            }
        }
        _ => {
            match m.signed.as_ref() {
                None => {
                    if pin_button(
                        ui,
                        x,
                        cy,
                        None,
                        40.0,
                        "Sign the message",
                        Style::Primary,
                        Action::MSign,
                    ) {
                        cy += 52.0;
                    }
                }
                Some(sig) => {
                    ui.text(x, cy, 12.0, W::R, MUTED, "Signature");
                    cy += 20.0;
                    cy += ui.wrap(x, cy, w, 13.0, W::M, TEXT, &sig.signature) + 14.0;
                    // Made For the stick as it was signed.
                    let file = format!(
                        "message-{}.txt",
                        &sig.address[sig.address.len().saturating_sub(6)..]
                    );
                    let (state, (label, style, action)) =
                        made_state(app, &file, None, ("Make the file", Action::MOut));
                    let png = format!("PNG {}", app.medium.for_the());
                    cy += file_row(
                        ui,
                        x,
                        cy,
                        w,
                        &file,
                        "Signed message",
                        state.as_deref(),
                        (label, style, action),
                        &[],
                    ) + 6.0;
                    cy += wrap_buttons(
                        ui,
                        x,
                        cy,
                        w,
                        38.0,
                        &[
                            ("Show as QR", Style::Secondary, Action::MQr),
                            (&png, Style::Secondary, Action::CodePng(Code::Message)),
                        ],
                    ) + 4.0;
                }
            }
            if let Some(e) = &m.error {
                cy += ui.wrap(x, cy, w, 13.0, W::R, ERR, e) + 6.0;
            }
        }
    }
    cy - y
}

fn check_screen(app: &Faraday, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    // A small panel has the way back and the name in its bar, and one
    // column that scrolls: each value under its name.
    let compact = ui.compact;
    let (x, width) = if compact {
        (x0 + crate::compact::M, cw - 2.0 * crate::compact::M)
    } else {
        (x0 + 48.0, (cw - 96.0).min(900.0))
    };
    let top = if compact {
        14.0 - app.list_offset
    } else {
        28.0
    };
    let mut y = top;
    if !compact {
        ui.icon(x - 4.0, y, 16.0, Icon::ChevronLeft, 10.0, MUTED);
        ui.text(x + 14.0, y, 12.0, W::R, MUTED, "Wallets");
        ui.hit(x - 4.0, y - 4.0, 80.0, 24.0, Action::Nav(Screen::Start));
        y += 22.0;
        title(ui, x, y, "Check a signed message");
        y += 60.0;
    }
    let guide = "A valid signature proves that whoever holds this address's key signed exactly this \
                 text. It says nothing about who is showing it to you, or when it was signed.";
    // On a small panel it comes after what it explains.
    if !compact {
        y += guide_text(app, ui, x, y, width, guide);
    }
    let Some(item) = app.checking.and_then(|i| app.inbox.get(i)) else {
        ui.text(x, y, 14.0, W::R, MUTED, "No signed message chosen");
        return;
    };
    let Some(c) = app
        .session
        .check_message(&String::from_utf8_lossy(&item.bytes))
    else {
        ui.wrap(
            x,
            y,
            width,
            14.0,
            W::R,
            ERR,
            &format!(
                "Not a signed message for a {} address",
                app.session.network().name()
            ),
        );
        return;
    };
    let (line, color) = match &c.verdict {
        Ok(f) => (
            format!(
                "The signature is valid ({})",
                if *f == osk_psbt::message::Format::Bip322 {
                    "BIP-322"
                } else {
                    "BIP-137"
                }
            ),
            OK,
        ),
        Err(e) => (format!("The signature does not verify: {e}"), ERR),
    };
    // The verdict's card grows to its wrapped line.
    let (size, lx) = if compact { (14.0, 46.0) } else { (16.0, 54.0) };
    let line_h = if compact {
        ui.c.push_clip(osk_ui::Rect::new(0, 0, 0, 0));
        let lh = ui.wrap(x + lx, y, width - lx - 14.0, size, W::S, color, &line);
        ui.c.pop_clip();
        lh
    } else {
        0.0
    };
    let card_h = if compact { line_h + 32.0 } else { 64.0 };
    ui.card(
        x,
        y,
        width,
        card_h,
        if c.verdict.is_ok() {
            OK.with_alpha(110)
        } else {
            ERR.with_alpha(110)
        },
    );
    ui.icon(
        x + lx - 36.0,
        y + card_h / 2.0 - 12.0,
        24.0,
        if c.verdict.is_ok() {
            Icon::Success
        } else {
            Icon::Error
        },
        16.0,
        color,
    );
    if compact {
        ui.wrap(
            x + lx,
            y + 16.0,
            width - lx - 14.0,
            size,
            W::S,
            color,
            &line,
        );
    } else {
        ui.text_mid(x + 54.0, y, 64.0, 16.0, W::S, color, &line);
    }
    y += card_h + 20.0;
    let rows = [
        ("File", item.name.clone(), W::M),
        (
            "Address",
            grouped(&app.session.shown(&c.signed.address)),
            W::M,
        ),
        (
            "Wallet",
            match c.wallet {
                Some((i, change, index)) => format!(
                    "{} · {} {}/{index}",
                    app.session.wallets[i].name,
                    if change { "change" } else { "receive" },
                    u8::from(change)
                ),
                None => "Not an address of a loaded wallet".to_string(),
            },
            W::R,
        ),
    ];
    let vx = if compact { 0.0 } else { 100.0 };
    for (k, v, face) in rows.iter() {
        ui.text(x, y, 12.0, W::R, MUTED, k);
        if compact {
            y += 18.0;
        }
        let used = ui.wrap(x + vx, y, width - vx, 14.0, *face, TEXT, v);
        y += used.max(20.0) + 12.0;
    }
    ui.text(x, y, 12.0, W::R, MUTED, "Message");
    if compact {
        y += 18.0;
    } else {
        ui.fill(x + 100.0, y - 4.0, width - 100.0, 1.0, 0.0, INNER);
        y += 12.0;
    }
    y += ui.wrap(x + vx, y, width - vx, 14.0, W::R, TEXT, &c.signed.message);
    if compact {
        y += 16.0 + crate::compact_screens::about(ui, x, y + 16.0, width, guide);
        crate::compact_screens::finish(app, ui, x0, cw, h, y - top + 24.0);
    }
}

// ---------------------------------------------------------------------
// Create a wallet
// ---------------------------------------------------------------------

const CSTEPS: [&str; crate::cstep::COUNT] = ["Kind", "Quorum", "Keys", "Check", "Back up"];

fn create_screen(app: &mut Faraday, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    use crate::cstep;
    let col_x = x0 + 40.0;
    let col_w = (cw - 80.0).min(900.0);
    let Some(c) = app.create.as_ref() else { return };
    let steps = crate::create_steps(c.kind);
    let filled = c
        .slots
        .iter()
        .filter(|s| **s != crate::create::Source::Empty)
        .count();
    let cards: Vec<flow::Card> = steps
        .iter()
        .map(|&k| flow::Card {
            title: CSTEPS[k as usize].to_string(),
            summary: match k {
                cstep::KIND => c.kind.name().to_string(),
                cstep::QUORUM => format!("{} of {}", c.m, c.n),
                cstep::KEYS if c.kind.threshold() => format!(
                    "{filled} of {} chosen · {} computed",
                    c.slots.len(),
                    c.n - c.m
                ),
                cstep::KEYS => match app.create_waiting() {
                    0 => format!("{filled} of {} chosen", c.slots.len()),
                    k => format!("{} of {} chosen · {k} to come", filled - k, c.slots.len()),
                },
                cstep::CHECK => match c.built.and_then(|i| app.session.wallets.get(i)) {
                    Some(w) => format!("{} · first addresses", w.name),
                    None => "Not made yet".to_string(),
                },
                _ => "Paper, vault, watch-only software".to_string(),
            },
            mono: false,
            done: c.done[k as usize],
            open: c.open == Some(k),
            default: k == cstep::KIND || k == cstep::QUORUM,
            toggle: Action::CStep(k),
            guide: Some(guide::create(k, c.kind, c.m, c.n, app.medium)),
        })
        .collect();
    let col = flow::Column {
        area_x: x0,
        area_w: cw,
        x: col_x,
        w: col_w,
        h,
        back: Some(("Wallets", Action::Nav(Screen::Start))),
        heading: "Create a wallet",
        guided: app.guided,
        switch: true,
        note: None,
        chip: None,
        chip_tap: None,
    };
    let scroll = c.scroll;
    let (next, again) = {
        let app_ref: &Faraday = app;
        let mut body = |ui: &mut Ui, i: usize, x: f32, y: f32, w: f32| -> f32 {
            create_body(app_ref, ui, steps[i], x, y, w)
        };
        flow::column(ui, &col, &cards, scroll, &mut body)
    };
    if let Some(c) = app.create.as_mut() {
        c.scroll = next;
    }
    if again {
        app.dirty = true;
        app.commands.push_back(osk_shell_api::Command::Draw);
    }
}

fn create_body(app: &Faraday, ui: &mut Ui, n: u8, x: f32, y: f32, w: f32) -> f32 {
    use crate::create::{NewKind, Source};
    use crate::cstep;
    let c = app.create.as_ref().expect("create");
    let locked = c.built.is_some();
    let mut cy = y;
    match n {
        cstep::KIND => {
            // Single key and Multisig, both native SegWit, are the two
            // everyday kinds; the other eight are behind More kinds.
            let shown: Vec<usize> = if c.more_kinds {
                (0..NewKind::ALL.len()).collect()
            } else {
                vec![0, 4]
            };
            for &i in &shown {
                let k = &NewKind::ALL[i];
                let on = c.kind == *k;
                let action = Action::CKind(i as u8);
                ui.fill(
                    x,
                    cy,
                    w,
                    48.0,
                    8.0,
                    if on { ACCENT.with_alpha(26) } else { BG },
                );
                ui.stroke(
                    x,
                    cy,
                    w,
                    48.0,
                    8.0,
                    if on { ACCENT.with_alpha(110) } else { INNER },
                );
                if ui.compact {
                    let name = ui.fit(14.0, W::S, k.name(), w - 28.0);
                    ui.text(x + 14.0, cy + 6.0, 14.0, W::S, TEXT, &name);
                    let line = ui.fit(12.0, W::R, k.line(), w - 28.0);
                    ui.text(x + 14.0, cy + 26.0, 12.0, W::R, MUTED, &line);
                } else {
                    ui.text_mid(x + 14.0, cy, 48.0, 14.0, W::S, TEXT, k.name());
                    let line = ui.fit(12.0, W::R, k.line(), w - 260.0);
                    ui.text_right(x + w - 14.0, cy, 48.0, 12.0, W::R, MUTED, &line);
                }
                if !locked {
                    ui.hit(x, cy, w, 48.0, action);
                }
                cy += 54.0;
            }
            if !c.more_kinds {
                ui.button(
                    x,
                    cy,
                    None,
                    34.0,
                    "More kinds",
                    Style::Ghost,
                    Action::CMoreKinds,
                );
                cy += 44.0;
            }
            cy += 6.0;
            let over: &[(&str, Style, Action)] = if app.create_unfinished() {
                &[("Start over", Style::Ghost, Action::CreateOver)]
            } else {
                &[]
            };
            cy += buttons_and_next(ui, x, cy, w, over, Some(("Continue", Action::CNext(n))));
        }
        cstep::QUORUM => {
            // MuSig2 is every key: only the count is chosen.
            let rows: &[(&str, usize, Action, Action)] = if c.kind.all_sign() {
                &[("Keys", c.n, Action::CN(-1), Action::CN(1))]
            } else {
                &[
                    ("Signatures needed", c.m, Action::CM(-1), Action::CM(1)),
                    ("Keys", c.n, Action::CN(-1), Action::CN(1)),
                ]
            };
            for &(label, value, minus, plus) in rows {
                stepper(ui, x, cy, w, label, value, minus, plus, !locked);
                cy += 50.0;
            }
            let sentence = if c.kind.all_sign() {
                format!(
                    "All {} keys sign together, as one signature on chain. Losing any one key loses the money: there is no spare.",
                    c.n
                )
            } else {
                format!(
                    "Any {} of the {} keys can spend. Losing {} {} still leaves the money spendable; {} keys together can take it.",
                    c.m,
                    c.n,
                    c.n - c.m,
                    if c.n - c.m == 1 { "key" } else { "keys" },
                    c.m
                )
            };
            cy += ui.wrap(x, cy, w, 13.0, W::R, TEXT, &sentence) + 14.0;
            if next_button(ui, x, cy, w, "Continue", Action::CNext(n)) {
                cy += 48.0;
            }
        }
        cstep::KEYS => {
            let key_files: Vec<(usize, String, String)> = app
                .inbox
                .iter()
                .enumerate()
                // Files with a key for this kind: an account xpub, or a
                // wallet file that carries account keys (Coldcard's export).
                .filter_map(|(i, it)| {
                    if !matches!(it.kind, FileKind::Key | FileKind::Wallet) {
                        return None;
                    }
                    let text = crate::create::key_for(c.kind, &String::from_utf8_lossy(&it.bytes))?;
                    Some((i, it.name.clone(), text))
                })
                .collect();
            // The way on leads: New key on the first slot still empty,
            // Continue once every slot is filled.
            let first_empty = c.slots.iter().position(|s| *s == Source::Empty);
            for (slot, src) in c.slots.iter().enumerate() {
                let label = format!(
                    "{} {}",
                    if c.kind.threshold() { "Share" } else { "Key" },
                    slot + 1
                );
                ui.text(x, cy + 4.0, 13.0, W::S, TEXT, &label);
                // A key locked in with a passphrase here says so
                // (`docs/NEW-WALLET.md` §3.6).
                if let Source::Here(fp) = src
                    && c.pass_locked.contains(fp)
                {
                    let tw = ui.measure(12.0, W::S, "Locked in") + 40.0;
                    locked_tag(ui, x + w - tw, cy);
                }
                let (state, color) = match src {
                    Source::Empty => ("Not chosen".to_string(), DIM),
                    Source::Here(fp) => {
                        let fpt = fp_text(osk_bip::keys::Fingerprint(*fp));
                        let name = app
                            .session
                            .key_label(osk_bip::keys::Fingerprint(*fp))
                            .unwrap_or("");
                        (format!("{fpt} · {name} · loaded here"), OK)
                    }
                    Source::Cosigner(t) => (
                        format!("{} · a cosigner's xpub", &t[1..9.min(t.len())]),
                        MUTED,
                    ),
                    Source::Later => (
                        "A cosigner adds it later · waiting for their xpub file".to_string(),
                        WARN,
                    ),
                };
                let indent = if ui.compact {
                    cy += 22.0;
                    let state = ui.fit(13.0, W::R, &state, w);
                    ui.text(x, cy + 4.0, 13.0, W::R, color, &state);
                    0.0
                } else {
                    ui.text(x + 70.0, cy + 4.0, 13.0, W::R, color, &state);
                    70.0
                };
                cy += 28.0;
                if !locked {
                    // A key chosen for one slot is offered in no other
                    // (`docs/NEW-WALLET.md` §2.2): clearing the slot
                    // offers it again.
                    let used_elsewhere_here = |fp: [u8; 4]| {
                        c.slots
                            .iter()
                            .enumerate()
                            .any(|(i, s)| i != slot && *s == Source::Here(fp))
                    };
                    let used_elsewhere_cosigner = |text: &str| {
                        c.slots.iter().enumerate().any(|(i, s)| {
                            i != slot && matches!(s, Source::Cosigner(t) if t == text)
                        })
                    };
                    // The choices for the slot, wrapped to the card.
                    let mut row: Vec<(String, Style, Action)> = Vec::new();
                    for k in &app.session.keys {
                        if c.kind.threshold() && k.share.is_none() {
                            continue;
                        }
                        let fp = k.master.fingerprint();
                        if used_elsewhere_here(fp.0) {
                            continue;
                        }
                        let on = *src == Source::Here(fp.0);
                        row.push((
                            key_line(fp, &k.label),
                            if on { Style::Primary } else { Style::Secondary },
                            Action::CSlotHere(slot as u8, fp.0),
                        ));
                    }
                    row.push((
                        "New key".to_string(),
                        if first_empty == Some(slot) {
                            Style::Primary
                        } else {
                            Style::Secondary
                        },
                        Action::KeyGen(Some(slot as u8)),
                    ));
                    for (i, name, text) in key_files.iter().filter(|_| !c.kind.threshold()) {
                        if used_elsewhere_cosigner(text) {
                            continue;
                        }
                        row.push((
                            format!("From {name}"),
                            Style::Secondary,
                            Action::CSlotFile(slot as u8, *i),
                        ));
                    }
                    if c.kind.multi()
                        && !c.kind.threshold()
                        && matches!(src, Source::Empty | Source::Later)
                    {
                        row.push((
                            "Scan their xpub".to_string(),
                            Style::Secondary,
                            Action::Scan,
                        ));
                    }
                    if c.kind.multi() && !c.kind.threshold() && *src == Source::Empty {
                        row.push((
                            "Someone else adds it later".to_string(),
                            Style::Secondary,
                            Action::CSlotLater(slot as u8),
                        ));
                    }
                    // A key from BIP-39 words with no passphrase can take
                    // one here: on the slot's own row on a single-key
                    // wallet, beside Show xpub QR on a multisig.
                    let pass_offered = app.create_pass_offered(slot);
                    if pass_offered && !c.kind.multi() {
                        row.push((
                            "Add a passphrase".to_string(),
                            if c.pass_slot == Some(slot as u8) {
                                Style::Primary
                            } else {
                                Style::Secondary
                            },
                            Action::CPassOpen(slot as u8),
                        ));
                    }
                    if *src != Source::Empty {
                        row.push((
                            "Clear".to_string(),
                            Style::Ghost,
                            Action::CSlotClear(slot as u8),
                        ));
                    }
                    cy += button_rows(ui, x + indent, cy, w - indent, &row) - 44.0;
                    cy += 44.0;
                    // A key held here goes out to the cosigners, who need
                    // it to make the same wallet.
                    if c.kind.multi() && !c.kind.threshold() && matches!(src, Source::Here(_)) {
                        let k = slot as u8;
                        let mut row = vec![
                            (
                                "Show xpub QR".to_string(),
                                Style::Secondary,
                                Action::CKeyQr(k),
                            ),
                            (
                                format!("Xpub PNG {}", app.medium.for_the()),
                                Style::Secondary,
                                Action::CodePng(Code::Key(k)),
                            ),
                            (
                                format!("Xpub file {}", app.medium.for_the()),
                                Style::Secondary,
                                Action::CKeyOut(k),
                            ),
                        ];
                        if pass_offered {
                            row.insert(
                                1,
                                (
                                    "Add a passphrase".to_string(),
                                    if c.pass_slot == Some(k) {
                                        Style::Primary
                                    } else {
                                        Style::Secondary
                                    },
                                    Action::CPassOpen(k),
                                ),
                            );
                        }
                        cy += button_rows(ui, x + indent, cy, w - indent, &row);
                    }
                    // The passphrase fields, under the slot they are for.
                    if c.pass_slot == Some(slot as u8) && pass_offered {
                        let (px, pw) = (x + indent, w - indent);
                        cy += pass_field(
                            ui,
                            px,
                            cy,
                            pw,
                            "Passphrase",
                            &c.pass,
                            c.pass_focus == Some(0),
                            Action::CPassField(0),
                        );
                        cy += pass_field(
                            ui,
                            px,
                            cy,
                            pw,
                            "Passphrase again",
                            &c.pass2,
                            c.pass_focus == Some(1),
                            Action::CPassField(1),
                        );
                        if let Some(n) = &c.pass_note {
                            cy += ui.wrap(px, cy, pw, 13.0, W::R, ERR, n) + 6.0;
                        }
                        let bw = ui.measure(14.0, W::S, "Lock in") + 36.0;
                        ui.button(
                            px + pw - bw,
                            cy,
                            Some(bw),
                            40.0,
                            "Lock in",
                            Style::Primary,
                            Action::CPassLock,
                        );
                        cy += 52.0;
                    }
                }
                ui.rule(x, cy, w, INNER);
                cy += 10.0;
            }
            if c.kind.threshold() {
                for k in c.m..c.n {
                    ui.text(x, cy + 4.0, 13.0, W::S, TEXT, &format!("Share {}", k + 1));
                    ui.text(
                        x + 70.0,
                        cy + 4.0,
                        13.0,
                        W::R,
                        MUTED,
                        "Computed by the deal",
                    );
                    cy += 34.0;
                    ui.rule(x, cy, w, INNER);
                    cy += 10.0;
                }
            }
            if let Some(e) = &c.error {
                cy += ui.wrap(x, cy, w, 13.0, W::R, ERR, e) + 6.0;
            }
            // While a slot is empty, the foot button opens New key for
            // it; once every slot is filled it reads Continue
            // (`docs/NEW-WALLET.md` §2.3).
            let (label, action) = match first_empty {
                Some(slot) if !locked => (
                    format!(
                        "New key for {} {}",
                        if c.kind.threshold() { "Share" } else { "Key" },
                        slot + 1
                    ),
                    Action::KeyGen(Some(slot as u8)),
                ),
                _ => ("Continue".to_string(), Action::CNext(n)),
            };
            let bw = ui.measure(14.0, W::S, &label) + 36.0;
            ui.button(
                x + w - bw,
                cy,
                Some(bw),
                40.0,
                &label,
                Style::Primary,
                action,
            );
            cy += 48.0;
        }
        cstep::CHECK => {
            if let Some(wl) = c.built.and_then(|i| app.session.wallets.get(i)) {
                // The descriptor as a summary row: tapping it opens the
                // wallet's code with the descriptor's full text under it.
                cy += descriptor_row(ui, x, cy, w, wl, Action::QrWallet(c.built.unwrap_or(0)));
                // So that a multisig's three rows do not read as one
                // address per key (`docs/NEW-WALLET.md` §2.4).
                section_label(ui, x, cy, "The wallet's first addresses");
                cy += 24.0;
                if let Some(i) = c.built
                    && !app.wallet_held(i).is_empty()
                {
                    cy += ui.wrap(x, cy, w, 13.0, W::S, WARN, crate::glance::RECEIVE_WARNING) + 8.0;
                }
                for (label, change, idx) in [
                    ("Receive 0/0", false, 0),
                    ("Receive 0/1", false, 1),
                    ("Change 1/0", true, 0),
                ] {
                    ui.text(x, cy + 2.0, 12.0, W::R, MUTED, label);
                    let a = grouped(&app.session.address_shown(wl, change, idx));
                    let ax = if ui.compact {
                        cy += 20.0;
                        0.0
                    } else {
                        96.0
                    };
                    cy += ui.wrap(x + ax, cy, w - ax, 13.0, W::M, TEXT, &a).max(20.0) + 10.0;
                }
                cy += 6.0;
                cy += buttons_and_next(ui, x, cy, w, &[], Some(("Continue", Action::CNext(n))));
            } else {
                ui.text(x, cy, 13.0, W::R, DIM, "Make the wallet first");
                cy += 30.0;
            }
        }
        _ => {
            // The backup plan's presets: each opens Back up a wallet on
            // the wallet just made, with that preset applied.
            if c.built.is_some() {
                for (k, p) in crate::plan::Preset::ALL.iter().enumerate() {
                    ui.button(
                        x,
                        cy,
                        Some(w),
                        40.0,
                        p.name(),
                        Style::Secondary,
                        Action::CBackup(k as u8),
                    );
                    cy += 48.0;
                }
            } else {
                ui.text(x, cy, 13.0, W::R, DIM, "Make the wallet first");
                cy += 30.0;
            }
        }
    }
    cy - y
}

/// Where a file a flow made stands: For the stick, with its place in
/// the list for Remove; or written, by the receipt. `None` when neither.
fn made_place(app: &Faraday, file: &str) -> Option<(String, Option<usize>)> {
    if let Some(k) = app.outbox.iter().position(|i| i.name == file) {
        return Some((app.medium.for_box().to_string(), Some(k)));
    }
    app.receipt
        .as_ref()
        .filter(|r| r.wrote(file))
        .map(|r| (format!("Written to {}", r.label), None))
}

/// A file a flow made, as one line: its name, where it is, and Remove
/// while it waits For the stick. Nothing when it is neither waiting nor
/// written. Returns its height.
fn made_line(app: &Faraday, ui: &mut Ui, x: f32, y: f32, w: f32, file: &str) -> f32 {
    let Some((state, k)) = made_place(app, file) else {
        return 0.0;
    };
    let mut right = x + w;
    if let Some(k) = k {
        let bw = ui.measure(13.0, W::S, "Remove") + 32.0;
        right -= bw;
        ui.button(
            right,
            y,
            Some(bw),
            34.0,
            "Remove",
            Style::Ghost,
            Action::OutboxRemove(k),
        );
        right -= 10.0;
    }
    let sw = ui.measure(13.0, W::S, &state);
    ui.text_mid(right - sw, y, 34.0, 13.0, W::S, OK, &state);
    let n = ui.fit(13.0, W::M, file, (right - sw - 10.0 - x).max(0.0));
    ui.text_mid(x, y, 34.0, 13.0, W::M, TEXT, &n);
    44.0
}

/// A made file's row state and its button: For the stick with Remove;
/// written, by the receipt, with `make` to make it again; or not made,
/// with `make`. `remove` takes it off For the stick.
fn made_state<'a>(
    app: &Faraday,
    file: &str,
    remove: Option<Action>,
    make: (&'a str, Action),
) -> (Option<String>, (&'a str, Style, Action)) {
    match made_place(app, file) {
        Some((t, Some(k))) => (
            Some(t),
            (
                "Remove",
                Style::Ghost,
                remove.unwrap_or(Action::OutboxRemove(k)),
            ),
        ),
        Some((t, None)) => (Some(t), ("Make again", Style::Secondary, make.1)),
        None => (None, (make.0, Style::Secondary, make.1)),
    }
}

/// One row of a file a flow makes: its name and what it is, and at the
/// right where it is, when it is made, and its button (Remove while it
/// waits For the stick, else the one that makes it), with `extra`
/// buttons before them (Show as QR, PNG). On a small panel the name and what it is are two
/// lines, and the buttons go on a row under the name. Returns the row's
/// height.
#[allow(clippy::too_many_arguments)]
fn file_row(
    ui: &mut Ui,
    x: f32,
    y: f32,
    w: f32,
    name: &str,
    detail: &str,
    done: Option<&str>,
    button: (&str, Style, Action),
    extra: &[(&str, Action)],
) -> f32 {
    if !extra.is_empty() {
        return file_row_extra(ui, x, y, w, name, detail, done, button, extra);
    }
    let bw = ui.measure(13.0, W::S, button.0) + 32.0;
    let done_w = done.map_or(0.0, |t| ui.measure(13.0, W::S, t) + 10.0);
    let right = bw + done_w;
    let rh = if ui.compact { 52.0 } else { 36.0 };
    let by = y + (rh - 34.0) / 2.0;
    ui.button(x + w - bw, by, Some(bw), 34.0, button.0, button.1, button.2);
    if ui.compact {
        // Where it is takes the second line, in place of what it is, so
        // the name keeps the room beside the button.
        let room = w - bw - 10.0;
        let name = ui.fit(14.0, W::S, name, room);
        ui.text(x, y + 6.0, 14.0, W::S, TEXT, &name);
        let (line, tone) = match done {
            Some(t) => (t, OK),
            None => (detail, MUTED),
        };
        let line = ui.fit(12.0, W::R, line, room);
        ui.text(x, y + 28.0, 12.0, W::R, tone, &line);
    } else {
        if let Some(t) = done {
            ui.text_right(x + w - bw - 10.0, y, rh, 13.0, W::S, OK, t);
        }
        let name_w = 200.0_f32.min(w - right - 10.0);
        let n = ui.fit(14.0, W::S, name, name_w);
        ui.text_mid(x, y, 36.0, 14.0, W::S, TEXT, &n);
        let room = w - right - 220.0 - 10.0;
        if room > 60.0 {
            let d = ui.fit(12.0, W::R, detail, room);
            ui.text_mid(x + 220.0, y, 36.0, 12.0, W::R, MUTED, &d);
        }
    }
    ui.rule(x, y + rh + 4.0, w, INNER);
    rh + 8.0
}

/// A file offered on a card: its name, what reads it, the file's name,
/// its own button's action, and Show as QR and PNG when it has them.
type FileRow<'a, N> = (N, &'a str, String, Action, Vec<(&'a str, Action)>);

#[allow(clippy::too_many_arguments)]
fn file_row_extra(
    ui: &mut Ui,
    x: f32,
    y: f32,
    w: f32,
    name: &str,
    detail: &str,
    done: Option<&str>,
    button: (&str, Style, Action),
    extra: &[(&str, Action)],
) -> f32 {
    let done_w = done.map_or(0.0, |t| ui.measure(13.0, W::S, t));
    if ui.compact {
        let room = w - if done_w > 0.0 { done_w + 10.0 } else { 0.0 };
        let n = ui.fit(14.0, W::S, name, room);
        ui.text(x, y + 6.0, 14.0, W::S, TEXT, &n);
        if let Some(t) = done {
            ui.text_right(x + w, y, 28.0, 13.0, W::S, OK, t);
        }
        let d = ui.fit(12.0, W::R, detail, w);
        ui.text(x, y + 28.0, 12.0, W::R, MUTED, &d);
        let mut items: Vec<(&str, Style, Action)> = extra
            .iter()
            .map(|&(l, a)| (l, Style::Secondary, a))
            .collect();
        items.push(button);
        // One row across the panel when the labels fit it, the space
        // left shared out; else wrapped.
        let gap = 6.0;
        let widths: Vec<f32> = items
            .iter()
            .map(|(l, ..)| ui.measure(13.0, W::S, l) + 20.0)
            .collect();
        let used = widths.iter().sum::<f32>() + gap * (items.len() as f32 - 1.0);
        let rh = if used <= w {
            let extra = (w - used) / items.len() as f32;
            let mut bx = x;
            for (&(label, style, action), bw) in items.iter().zip(&widths) {
                bx += ui.button(bx, y + 50.0, Some(bw + extra), 34.0, label, style, action) + gap;
            }
            50.0 + 34.0
        } else {
            50.0 + wrap_buttons(ui, x, y + 50.0, w, 34.0, &items) - 8.0
        };
        ui.rule(x, y + rh + 4.0, w, INNER);
        return rh + 8.0;
    }
    let rh = 36.0;
    let by = y + (rh - 34.0) / 2.0;
    let mut right = x + w;
    let bw = ui.measure(13.0, W::S, button.0) + 32.0;
    right -= bw;
    ui.button(right, by, Some(bw), 34.0, button.0, button.1, button.2);
    right -= 6.0;
    if let Some(t) = done {
        ui.text_right(right - 4.0, y, rh, 13.0, W::S, OK, t);
        right -= done_w + 16.0;
    }
    for &(label, action) in extra.iter().rev() {
        let bw = ui.measure(13.0, W::S, label) + 32.0;
        right -= bw;
        ui.button(right, by, Some(bw), 34.0, label, Style::Secondary, action);
        right -= 6.0;
    }
    let name_w = 200.0_f32.min(right - x - 10.0);
    let n = ui.fit(14.0, W::S, name, name_w);
    ui.text_mid(x, y, rh, 14.0, W::S, TEXT, &n);
    let room = right - (x + 220.0) - 10.0;
    if room > 60.0 {
        let d = ui.fit(12.0, W::R, detail, room);
        ui.text_mid(x + 220.0, y, rh, 12.0, W::R, MUTED, &d);
    }
    ui.rule(x, y + rh + 4.0, w, INNER);
    rh + 8.0
}

/// A wallet's descriptor as a summary row (`docs/DESIGN.md` §4.5): label
/// above; the shape, the keys' fingerprints and the checksum below; a
/// chevron; the whole row opens `action`. Returns its height.
/// A masked passphrase field with its label at the left (above it on a
/// small panel): dots for what is typed, "Not set" while it is empty and
/// not focused, the caret while it is. Returns the height used.
#[allow(clippy::too_many_arguments)]
pub(crate) fn pass_field(
    ui: &mut Ui,
    x: f32,
    y: f32,
    w: f32,
    label: &str,
    text: &crate::secret_text::SecretText,
    on: bool,
    action: Action,
) -> f32 {
    let (px, py, pw) = if ui.compact {
        ui.text(x, y, 13.0, W::R, MUTED, label);
        (x, y + 22.0, w)
    } else {
        ui.text_mid(x, y, 40.0, 13.0, W::R, MUTED, label);
        (x + 150.0, y, 380.0f32.min(w - 150.0))
    };
    ui.fill(px, py, pw, 40.0, 8.0, BG);
    ui.stroke(px, py, pw, 40.0, 8.0, if on { ACCENT } else { BORDER });
    let (shown, tone) = if text.as_str().is_empty() && !on {
        ("Not set".to_string(), DIM)
    } else {
        let mut d = "•".repeat(text.as_str().chars().count().min(40));
        ui.selection(px + 12.0, py, 40.0, 14.0, W::M, &d, on);
        if on && !ui.select_all {
            d.push_str(ui.caret_char());
        }
        (d, TEXT)
    };
    ui.text_mid(px + 12.0, py, 40.0, 14.0, W::M, tone, &shown);
    ui.hit(px, py, pw, 40.0, action);
    py - y + 48.0
}

/// The **Locked in** tag: a lock and the words, in `OK`. Returns its
/// width.
pub(crate) fn locked_tag(ui: &mut Ui, x: f32, y: f32) -> f32 {
    let label = "Locked in";
    let width = ui.measure(12.0, W::S, label) + 40.0;
    ui.fill(x, y, width, 26.0, 13.0, OK.with_alpha(30));
    ui.icon(x + 6.0, y + 3.0, 20.0, Icon::Lock, 10.0, OK);
    ui.text_mid(x + 28.0, y, 26.0, 12.0, W::S, OK, label);
    width
}

pub(crate) fn descriptor_row(
    ui: &mut Ui,
    x: f32,
    y: f32,
    w: f32,
    wl: &crate::wallet::Wallet,
    action: Action,
) -> f32 {
    let fps: Vec<String> = wl
        .policy
        .keys()
        .iter()
        .map(|k| crate::backup::fp_of(k.fingerprint()))
        .collect();
    let mut value = format!("{} · {}", Session::shape(wl), fps.join(", "));
    if wl.policy.record().is_none() {
        value.push_str(&format!(" · #{}", wl.policy.checksum()));
    }
    ui.text(x, y, 12.0, W::R, MUTED, "Descriptor");
    let vh = ui
        .wrap(x, y + 20.0, w - 28.0, 13.0, W::M, TEXT, &value)
        .max(18.0);
    let h = 20.0 + vh + 8.0;
    ui.icon(
        x + w - 20.0,
        y + (h - 20.0) / 2.0,
        20.0,
        Icon::ChevronRight,
        9.0,
        DIM,
    );
    ui.hit(x, y - 4.0, w, h, action);
    ui.rule(x, y + h, w, INNER);
    h + 12.0
}

/// Wallet `i`'s public files, as the backup's public step offers them:
/// the descriptor, the wallet file, the multisig config, the backup
/// sheet, the BSMS record, Bitcoin Core's import, and each key held here
/// with its BSMS record, for the cosigners. Each has its own button, and Show as QR and PNG beside the
/// ones a wallet or a person reads from a code.
fn public_files(app: &Faraday, i: usize) -> Vec<FileRow<'static, String>> {
    use osk_bip::policy::{Template, Wrapper};
    let Some(wl) = app.session.wallets.get(i) else {
        return Vec::new();
    };
    let stem = crate::file_stem(&wl.name);
    let codes = |show: Action, png: Action| vec![("Show as QR", show), ("PNG", png)];
    let mut rows: Vec<FileRow<String>> = vec![
        (
            "Descriptor".to_string(),
            "Any wallet software",
            format!("{stem}-descriptor.txt"),
            Action::PublicOut(i, 1),
            codes(Action::QrWallet(i), Action::PublicOut(i, 8)),
        ),
        (
            "Wallet file".to_string(),
            "Sparrow, Specter",
            format!("{stem}-wallet.json"),
            Action::PublicOut(i, 5),
            Vec::new(),
        ),
    ];
    if crate::backup::multisig_config(wl, None).is_some() {
        rows.push((
            "Multisig config".to_string(),
            "Coldcard, Keystone, Passport",
            format!("{stem}-multisig-config.txt"),
            Action::PublicOut(i, 2),
            codes(
                Action::ShowCode(Code::MultisigConfig(i)),
                Action::CodePng(Code::MultisigConfig(i)),
            ),
        ));
    }
    rows.push((
        "Backup sheet".to_string(),
        "A PDF to print",
        format!("{stem}-backup.pdf"),
        Action::PublicOut(i, 3),
        Vec::new(),
    ));
    // BIP 129 covers wsh and sh(wsh) multisig.
    if matches!(
        wl.policy.template(),
        Template::Multi {
            wrapper: Wrapper::Wsh | Wrapper::ShWsh,
            ..
        }
    ) {
        rows.push((
            "BSMS descriptor record".to_string(),
            "BIP 129: Coldcard, Sparrow, Nunchuk",
            format!("{stem}-bsms.txt"),
            Action::PublicOut(i, 6),
            codes(
                Action::ShowCode(Code::Bsms(i)),
                Action::CodePng(Code::Bsms(i)),
            ),
        ));
    }
    // A silent payments wallet has no descriptor Bitcoin Core imports.
    if wl.policy.silent().is_none() {
        rows.push((
            "Bitcoin Core import".to_string(),
            "importdescriptors, watch-only",
            format!("{stem}-bitcoin-core.json"),
            Action::PublicOut(i, 7),
            Vec::new(),
        ));
    }
    // The account key of each seed held here, for the cosigners.
    for slot in app.wallet_keys_here(i) {
        let Ok(k) = app.wallet_key(i, slot) else {
            continue;
        };
        let fp = k.fp.clone();
        let record = matches!(
            k.standard,
            Some(crate::create::NewKind::Multi | crate::create::NewKind::MultiNested)
        );
        rows.push((
            format!("Key {fp}"),
            "For the cosigners",
            format!("xpub-{fp}.txt"),
            Action::WalletKeyOut(i, slot),
            codes(
                Action::ShowCode(Code::WalletKey(i, slot)),
                Action::CodePng(Code::WalletKey(i, slot)),
            ),
        ));
        if record {
            rows.push((
                format!("Key {fp}, BSMS"),
                "Signed xpub record, BIP 129",
                format!("xpub-{fp}-bsms.txt"),
                Action::WalletKeyBsms(i, slot),
                codes(
                    Action::ShowCode(Code::WalletKeyBsms(i, slot)),
                    Action::CodePng(Code::WalletKeyBsms(i, slot)),
                ),
            ));
        }
    }
    rows
}

/// Draws [`public_files`] of wallet `i` on the backup's checklist, each
/// marked once its file is in the Outbox: only the files `only`
/// names (as `public_out` numbers them) and the keys for the cosigners,
/// the codes beside them only where the plan's form has QR pictures.
fn public_rows_for(
    app: &Faraday,
    ui: &mut Ui,
    x: f32,
    y: f32,
    w: f32,
    i: usize,
    only: Option<&[u8]>,
) -> f32 {
    let qr = app
        .backup
        .as_ref()
        .is_none_or(|b| only.is_none() || b.answers.form[crate::plan::form::QR]);
    let mut cy = y;
    for (name, detail, file, action, extra) in &public_files(app, i) {
        let wanted = match (only, action) {
            (Some(o), Action::PublicOut(_, what)) => o.contains(what),
            _ => true,
        };
        if !wanted {
            continue;
        }
        let extra: &[(&str, Action)] = if qr { extra } else { &[] };
        // Made For the stick with the checklist: Remove while it waits.
        let remove = match action {
            Action::PublicOut(w, what) => Some(Action::PublicRemove(*w, *what)),
            _ => None,
        };
        let (state, button) = made_state(app, file, remove, ("Make the file", *action));
        cy += file_row(ui, x, cy, w, name, detail, state.as_deref(), button, extra);
    }
    cy - y
}

// ---------------------------------------------------------------------
// Restore a wallet
// ---------------------------------------------------------------------

const RSTEPS: [&str; 5] = [
    "A transaction to sign",
    "The wallet",
    "The seeds",
    "Check",
    "Done",
];

fn restore_screen(app: &mut Faraday, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    let col_x = x0 + 40.0;
    let col_w = (cw - 80.0).min(900.0);
    let Some(r) = app.restore.as_ref() else {
        return;
    };
    let wl = r.wallet.and_then(|i| app.session.wallets.get(i));
    let psbts = app
        .inbox
        .iter()
        .filter(|i| i.kind == FileKind::Psbt)
        .count();
    let cards: Vec<flow::Card> = (0..crate::rstep::COUNT)
        .map(|k| flow::Card {
            title: RSTEPS[k as usize].to_string(),
            summary: match (k, wl) {
                (0, _) => match (app.spend.as_ref(), psbts) {
                    (Some(s), _) => format!("Signing {}", s.spend.source),
                    (None, 0) => "None in Files".to_string(),
                    (None, 1) => "1 PSBT in Files".to_string(),
                    (None, n) => format!("{n} PSBTs in Files"),
                },
                (1, Some(w)) => format!("{} · {}", w.name, Session::shape(w)),
                (1, None) if r.seeds.is_some() => "From the seeds".to_string(),
                (1, None) => "Not chosen yet".to_string(),
                (2, None) => match r.seeds.as_ref().map(|s| app.seeds_here(s).len()) {
                    Some(1) => "1 seed typed".to_string(),
                    Some(n) => format!("{n} seeds typed"),
                    None => String::new(),
                },
                (2, Some(w)) => {
                    let slots = app.session.slots(w);
                    let here = slots.iter().filter(|s| s.held_by.is_some()).count();
                    format!("{here} of {} seeds typed", slots.len())
                }
                (3, Some(w)) => short(&app.session.address_shown(w, false, 0)),
                _ => String::new(),
            },
            mono: k == 3,
            done: r.done[k as usize],
            open: r.open == Some(k),
            default: false,
            toggle: Action::RStep(k),
            guide: Some(guide::restore(k, app.medium)),
        })
        .collect();
    let col = flow::Column {
        area_x: x0,
        area_w: cw,
        x: col_x,
        w: col_w,
        h,
        back: Some((
            "Wallets",
            Action::Nav(if r.from_key {
                Screen::Wallets
            } else {
                Screen::Start
            }),
        )),
        heading: "Load or restore a wallet",
        guided: app.guided,
        switch: true,
        note: None,
        chip: None,
        chip_tap: None,
    };
    let scroll = r.scroll;
    let (next, again) = {
        let app_ref: &Faraday = app;
        let mut body = |ui: &mut Ui, i: usize, x: f32, y: f32, w: f32| -> f32 {
            restore_body(app_ref, ui, i as u8, x, y, w)
        };
        flow::column(ui, &col, &cards, scroll, &mut body)
    };
    if let Some(r) = app.restore.as_mut() {
        r.scroll = next;
    }
    if again {
        app.dirty = true;
        app.commands.push_back(osk_shell_api::Command::Draw);
    }
}

fn restore_body(app: &Faraday, ui: &mut Ui, n: u8, x: f32, y: f32, w: f32) -> f32 {
    let r = app.restore.as_ref().expect("restore");
    let wl = r.wallet.and_then(|i| app.session.wallets.get(i));
    let mut cy = y;
    match n {
        0 => cy += restore_psbt_body(app, ui, x, cy, w),
        1 => {
            cy += restore_sources(app, ui, x, cy, w);
            let wallets: Vec<(usize, &str)> = app
                .inbox
                .iter()
                .enumerate()
                .filter(|(_, i)| i.kind == FileKind::Wallet)
                .map(|(k, i)| (k, i.name.as_str()))
                .collect();
            let shares: Vec<(usize, &str)> = app
                .inbox
                .iter()
                .enumerate()
                .filter(|(_, i)| i.kind == FileKind::Share)
                .map(|(k, i)| (k, i.name.as_str()))
                .collect();
            if wallets.is_empty() && shares.is_empty() {
                ui.wrap(
                    x,
                    cy,
                    w,
                    13.0,
                    W::R,
                    MUTED,
                    "No descriptor, config or share in Files",
                );
                cy += 30.0;
            }
            if !wallets.is_empty() {
                ui.text(x, cy, 12.0, W::R, MUTED, "Whole wallets");
                cy += 22.0;
                for (k, name) in &wallets {
                    // No wider than the column; a long name is cut.
                    let label = format!("Restore from {name}");
                    let bw = (ui.measure(13.0, W::S, &label) + 32.0).min(w);
                    let label = ui.fit(13.0, W::S, &label, bw - 24.0);
                    ui.button(
                        x,
                        cy,
                        Some(bw),
                        34.0,
                        &label,
                        Style::Secondary,
                        Action::RUse(*k),
                    );
                    cy += 42.0;
                }
            }
            if !shares.is_empty() {
                ui.text(x, cy, 12.0, W::R, MUTED, "Shares of a split backup");
                cy += 22.0;
                for (k, name) in &shares {
                    let on = r.shares.iter().any(|s| s == name);
                    ui.checkbox(x, cy + 8.0, on, true);
                    let name = ui.fit(13.0, W::M, name, w - 30.0);
                    ui.text_mid(x + 30.0, cy, 34.0, 13.0, W::M, TEXT, &name);
                    ui.hit(x, cy, w, 34.0, Action::RShare(*k));
                    cy += 38.0;
                }
                let texts = app.restore_share_texts();
                if !texts.is_empty() {
                    match crate::restore::merge(&texts) {
                        Ok(m) => {
                            let line = format!(
                                "{} of {} keys in hand · {}",
                                m.have.len(),
                                m.n,
                                m.have.join(" ")
                            );
                            cy += ui.wrap(
                                x,
                                cy,
                                w,
                                13.0,
                                W::M,
                                if m.whole.is_some() { OK } else { WARN },
                                &line,
                            ) + 10.0;
                            let style = if m.whole.is_some() {
                                Style::Primary
                            } else {
                                Style::Disabled
                            };
                            ui.button(
                                x,
                                cy,
                                None,
                                38.0,
                                "Rebuild the wallet",
                                style,
                                Action::RRebuild,
                            );
                            cy += 48.0;
                        }
                        Err(e) => {
                            cy += ui.wrap(x, cy, w, 13.0, W::R, ERR, &e) + 10.0;
                        }
                    }
                }
            }
            if let Some(e) = &r.error {
                cy += ui.wrap(x, cy, w, 13.0, W::R, ERR, e) + 6.0;
            }
        }
        2 => {
            if let Some(w2) = wl {
                let may = app.may_load_keys();
                let slots = app.session.slots(w2);
                let enough = slots.iter().filter(|s| s.held_by.is_some()).count()
                    >= crate::wallet::needed(w2);
                for (k, slot) in slots.iter().enumerate() {
                    ui.text_mid(x, cy, 40.0, 12.0, W::R, DIM, &(k + 1).to_string());
                    let fp = slot.fingerprint.map(fp_text).unwrap_or_default();
                    ui.text_mid(x + 22.0, cy, 40.0, 14.0, W::M, TEXT, &fp);
                    match &slot.held_by {
                        Some(l) => {
                            let here = ui.fit(13.0, W::R, &format!("Here · {l}"), w - 124.0);
                            ui.text_mid(x + 124.0, cy, 40.0, 13.0, W::R, OK, &here);
                        }
                        None if enough => {
                            ui.text_right(x + w, cy, 40.0, 12.0, W::R, DIM, "Not needed");
                        }
                        None => {
                            let style = if may {
                                Style::Secondary
                            } else {
                                Style::Disabled
                            };
                            let (label, action) = match slot
                                .fingerprint
                                .filter(|f| app.vault_key_for(*f).is_some())
                            {
                                Some(f) => (
                                    "Load from vault",
                                    Action::Vault(crate::vaults::VaultAction::LoadKeyOf(f.0)),
                                ),
                                None => (
                                    "Type its words",
                                    Action::Entry(slot.fingerprint.map(|f| f.0)),
                                ),
                            };
                            ui.button(
                                x + w - 150.0,
                                cy + 2.0,
                                Some(150.0),
                                36.0,
                                label,
                                style,
                                action,
                            );
                        }
                    }
                    ui.rule(x, cy + 42.0, w, INNER);
                    cy += 44.0;
                }
                if !enough {
                    cy += 4.0;
                    cy += missing_keys_line(app, ui, &slots, Screen::Restore, x, cy, w);
                }
                cy += 10.0;
                if next_button(ui, x, cy, w, "Continue", Action::RNext(n)) {
                    cy += 48.0;
                }
            } else if let Some(s) = r.seeds.as_ref() {
                // Seeds first: the seeds, then the wallet they make.
                if s.shaping {
                    cy += crate::seeds_screen::shape(app, ui, s, true, x, cy, w);
                } else {
                    cy += crate::seeds_screen::keys(app, ui, s, false, x, cy, w);
                    if !app.seeds_here(s).is_empty() {
                        cy += 4.0;
                        cy += buttons_and_next(
                            ui,
                            x,
                            cy,
                            w,
                            &[],
                            Some((
                                "Make the wallet",
                                Action::Seeds(crate::seeds::SeedsAction::Shape),
                            )),
                        );
                    }
                }
            } else {
                ui.text(x, cy, 13.0, W::R, DIM, "Choose the wallet first");
                cy += 30.0;
                cy += wrap_buttons(
                    ui,
                    x,
                    cy,
                    w,
                    40.0,
                    &[("Type the seeds", Style::Secondary, Action::RSeeds)],
                );
            }
        }
        3 => {
            if let Some(w2) = wl {
                ui.text(x, cy, 12.0, W::R, MUTED, "First receive address · 0/0");
                cy += 22.0;
                cy += ui.wrap(
                    x,
                    cy,
                    w,
                    15.0,
                    W::M,
                    TEXT,
                    &grouped(&app.session.address_shown(w2, false, 0)),
                ) + 14.0;
                cy += buttons_and_next(
                    ui,
                    x,
                    cy,
                    w,
                    &[(
                        "Show wallet QR",
                        Style::Secondary,
                        Action::QrWallet(r.wallet.unwrap_or(0)),
                    )],
                    Some(("It matches", Action::RNext(n))),
                );
            }
        }
        _ => {
            if let Some(i) = r.wallet
                && ui.compact
            {
                // Open the wallet pinned at the foot.
                ui.pin = Some((
                    "Open the wallet".to_string(),
                    Style::Primary,
                    Action::OpenWallet(i),
                ));
                cy += wrap_buttons(
                    ui,
                    x,
                    cy,
                    w,
                    40.0,
                    &[("Back it up again", Style::Secondary, Action::Backup(i))],
                ) + 4.0;
            } else if let Some(i) = r.wallet {
                ui.button(
                    x,
                    cy,
                    None,
                    40.0,
                    "Open the wallet",
                    Style::Primary,
                    Action::OpenWallet(i),
                );
                let bw = ui.measure(14.0, W::S, "Back it up again") + 32.0;
                ui.button(
                    x + w - bw,
                    cy,
                    Some(bw),
                    40.0,
                    "Back it up again",
                    Style::Secondary,
                    Action::Backup(i),
                );
                cy += 52.0;
            }
        }
    }
    cy - y
}

/// Restore's first card: a transaction to sign, by stick while nothing
/// secret is open, or by QR now or later.
fn restore_psbt_body(app: &Faraday, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    let mut cy = y;
    for it in app.inbox.iter().filter(|i| i.kind == FileKind::Psbt) {
        ui.icon(x, cy + 8.0, 20.0, Icon::Done, 11.0, OK);
        let name = ui.fit(14.0, W::M, &it.name, w - 28.0);
        ui.text_mid(x + 28.0, cy, 36.0, 14.0, W::M, TEXT, &name);
        cy += 38.0;
    }
    let closed = app.holds_secret() || !app.vaults.open.is_empty();
    let (style, sub) = match (closed, app.sticks.is_empty()) {
        (true, _) => (
            Style::Disabled,
            format!("{} only before a key or vault is open", app.medium.a_cap()),
        ),
        (false, true) => (Style::Disabled, format!("Insert the {}", app.medium.noun())),
        (false, false) => (Style::Secondary, String::new()),
    };
    let from = format!("From {}", app.medium.a());
    if ui.compact {
        // The two ways in, then why the stick cannot be used, each on
        // its own line.
        cy += wrap_buttons(
            ui,
            x,
            cy,
            w,
            40.0,
            &[
                (from.as_str(), style, Action::VisitFrom(Screen::Restore)),
                ("Scan a QR code", Style::Secondary, Action::Scan),
            ],
        );
        if !sub.is_empty() {
            cy += ui.wrap(x, cy, w, 13.0, W::R, WARN, &sub) + 10.0;
        }
        let has = app.spend.is_some() || app.inbox.iter().any(|i| i.kind == FileKind::Psbt);
        let drawn = next_button(
            ui,
            x,
            cy,
            w,
            if has { "Continue" } else { "Later" },
            Action::RNext(0),
        );
        return cy + (if drawn { 52.0 } else { 8.0 }) - y;
    }
    let bw = ui.button(
        x,
        cy,
        None,
        40.0,
        &from,
        style,
        Action::VisitFrom(Screen::Restore),
    );
    let sw = ui.button(
        x + bw + 8.0,
        cy,
        None,
        40.0,
        "Scan a QR code",
        Style::Secondary,
        Action::Scan,
    );
    if !sub.is_empty() {
        ui.text_mid(x + bw + sw + 24.0, cy, 40.0, 13.0, W::R, WARN, &sub);
    }
    cy += 52.0;
    let has = app.spend.is_some() || app.inbox.iter().any(|i| i.kind == FileKind::Psbt);
    if next_button(
        ui,
        x,
        cy,
        w,
        if has { "Continue" } else { "Later" },
        Action::RNext(0),
    ) {
        cy += 52.0;
    }
    cy - y
}

/// Where a wallet comes back from: the files of a stick, a vault on a
/// second stick, or paper.
fn restore_sources(app: &Faraday, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    use crate::vaults::VaultAction as V;
    use faraday_vault::records::{field, kind};
    if ui.compact {
        return restore_sources_compact(app, ui, x, y, w);
    }
    let mut cy = y;
    // A stick.
    let stick = !app.sticks.is_empty() && !app.holds_secret();
    ui.text_mid(x, cy, 40.0, 13.0, W::S, MUTED, app.medium.cap());
    let bw = ui.button(
        x + 90.0,
        cy,
        None,
        40.0,
        "Copy files in",
        if stick {
            Style::Secondary
        } else {
            Style::Disabled
        },
        Action::VisitFrom(Screen::Restore),
    );
    if !stick {
        ui.text_mid(
            x + 90.0 + bw + 12.0,
            cy,
            40.0,
            13.0,
            W::R,
            DIM,
            &if app.holds_secret() {
                "Not while a key is loaded".to_string()
            } else {
                format!("Insert the {}", app.medium.noun())
            },
        );
    }
    cy += 48.0;
    // A vault: locked in Files, or open.
    ui.text_mid(x, cy, 40.0, 13.0, W::S, MUTED, "Vault");
    let files = app.vault_files();
    let mut bx = x + 90.0;
    let mut any = false;
    for (k, f) in files.iter().enumerate().filter(|(_, f)| f.open.is_none()) {
        bx += ui.button(
            bx,
            cy,
            None,
            40.0,
            &format!("Unlock {}", f.name),
            Style::Secondary,
            Action::Vault(V::OpenFrom(k, Screen::Restore)),
        ) + 8.0;
        any = true;
    }
    for (v, open) in app.vaults.open.iter().enumerate() {
        for (r, rec) in open.contents.of(kind::WALLET) {
            let name = rec.text(field::WALLET_NAME).unwrap_or("Wallet");
            bx += ui.button(
                bx,
                cy,
                None,
                40.0,
                &format!("{name} · {}", open.name),
                Style::Secondary,
                Action::RFromVault(v, r),
            ) + 8.0;
            any = true;
            if bx > x + w - 160.0 {
                bx = x + 90.0;
                cy += 48.0;
            }
        }
    }
    if !any {
        ui.text_mid(
            bx,
            cy,
            40.0,
            13.0,
            W::R,
            DIM,
            &format!("Copy the vault in from its {}", app.medium.noun()),
        );
    }
    cy += 48.0;
    // Paper.
    ui.text_mid(x, cy, 40.0, 13.0, W::S, MUTED, "Paper");
    let sw = ui.button(
        x + 90.0,
        cy,
        None,
        40.0,
        "Scan the descriptor or a share",
        Style::Secondary,
        Action::Scan,
    );
    ui.button(
        x + 90.0 + sw + 8.0,
        cy,
        None,
        40.0,
        "Type the seeds",
        Style::Secondary,
        Action::RSeeds,
    );
    cy += 56.0;
    ui.rule(x, cy - 6.0, w, INNER);
    cy += 10.0;
    cy - y
}

/// [`restore_sources`] on a small panel: each source's name over its
/// buttons, and what stands in the way under them.
fn restore_sources_compact(app: &Faraday, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    use crate::vaults::VaultAction as V;
    use faraday_vault::records::{field, kind};
    let mut cy = y;
    // A stick.
    let stick = !app.sticks.is_empty() && !app.holds_secret();
    ui.text(x, cy, 13.0, W::S, MUTED, app.medium.cap());
    cy += 22.0;
    cy += wrap_buttons(
        ui,
        x,
        cy,
        w,
        40.0,
        &[(
            "Copy files in",
            if stick {
                Style::Secondary
            } else {
                Style::Disabled
            },
            Action::VisitFrom(Screen::Restore),
        )],
    );
    if !stick {
        let why = if app.holds_secret() {
            "Not while a key is loaded".to_string()
        } else {
            format!("Insert the {}", app.medium.noun())
        };
        cy += ui.wrap(x, cy, w, 13.0, W::R, DIM, &why) + 8.0;
    }
    cy += 8.0;
    // A vault: locked in Files, or open.
    ui.text(x, cy, 13.0, W::S, MUTED, "Vault");
    cy += 22.0;
    let files = app.vault_files();
    let mut labels: Vec<(String, Action)> = files
        .iter()
        .enumerate()
        .filter(|(_, f)| f.open.is_none())
        .map(|(k, f)| {
            (
                format!("Unlock {}", f.name),
                Action::Vault(V::OpenFrom(k, Screen::Restore)),
            )
        })
        .collect();
    for (v, open) in app.vaults.open.iter().enumerate() {
        for (r, rec) in open.contents.of(kind::WALLET) {
            let name = rec.text(field::WALLET_NAME).unwrap_or("Wallet");
            labels.push((format!("{name} · {}", open.name), Action::RFromVault(v, r)));
        }
    }
    if labels.is_empty() {
        cy += ui.wrap(
            x,
            cy,
            w,
            13.0,
            W::R,
            DIM,
            &format!("Copy the vault in from its {}", app.medium.noun()),
        ) + 8.0;
    } else {
        let items: Vec<(&str, Style, Action)> = labels
            .iter()
            .map(|(l, a)| (l.as_str(), Style::Secondary, *a))
            .collect();
        cy += wrap_buttons(ui, x, cy, w, 40.0, &items);
    }
    cy += 8.0;
    // Paper.
    ui.text(x, cy, 13.0, W::S, MUTED, "Paper");
    cy += 22.0;
    cy += wrap_buttons(
        ui,
        x,
        cy,
        w,
        40.0,
        &[
            (
                "Scan the descriptor or a share",
                Style::Secondary,
                Action::Scan,
            ),
            ("Type the seeds", Style::Secondary, Action::RSeeds),
        ],
    ) + 6.0;
    ui.rule(x, cy - 6.0, w, INNER);
    cy += 10.0;
    cy - y
}

// ---------------------------------------------------------------------
// Files
// ---------------------------------------------------------------------

pub(crate) fn kind_line(kind: FileKind, bytes: usize) -> String {
    let size = if bytes >= 1_000_000 {
        format!("{:.1} MB", bytes as f32 / 1e6)
    } else if bytes >= 1000 {
        format!("{:.1} KB", bytes as f32 / 1e3)
    } else {
        format!("{bytes} B")
    };
    format!("{} · {size}", kind_name(kind))
}

/// What a file of this kind is called.
pub(crate) fn kind_name(kind: FileKind) -> &'static str {
    match kind {
        FileKind::Psbt => "PSBT",
        FileKind::Wallet => "Wallet descriptor",
        FileKind::Transaction => "Finished transaction",
        FileKind::Sheet => "Sheet for printing",
        FileKind::Message => "Signed message",
        FileKind::Key => "Account xpub",
        FileKind::Words => "Seed words",
        FileKind::Share => "Share of a split backup",
        FileKind::Carry => "Threshold spend, part-signed",
        FileKind::Vault => "Vault, locked",
        FileKind::Entries => "Entries for a vault",
        FileKind::Backup => "OpenSigner backup, encrypted",
        FileKind::Kdbx => "KeePass database, encrypted",
        FileKind::Pdf => "PDF to print",
        FileKind::EfiImage => "EFI image",
        FileKind::SeedPart => "Part of a seed: SLIP-39 or codex32",
        FileKind::Text => "Text",
        FileKind::Other => "File",
    }
}

/// Add to vault, for an Inbox file a vault keeps; with none open, Make a
/// vault or Unlock, which come back to Files. Returns the width used.
fn add_to_vault(app: &Faraday, ui: &mut Ui, x: f32, y: f32, k: usize, style: Style) -> f32 {
    let (label, action) = app.vault_way(Screen::Files).unwrap_or((
        "Add to vault".to_string(),
        Action::Vault(crate::vaults::VaultAction::AddFile(k)),
    ));
    ui.button(x, y, None, 36.0, &label, style, action) + 8.0
}

/// Restore the wallet, on a share of a split backup: the shares in the
/// Inbox put together, or how many keys are still missing.
fn restore_shares_button(app: &Faraday, ui: &mut Ui, x: f32, y: f32) -> f32 {
    let texts: Vec<String> = app
        .inbox
        .iter()
        .filter(|i| i.kind == FileKind::Share)
        .map(|i| String::from_utf8_lossy(&i.bytes).into_owned())
        .collect();
    match crate::restore::merge(&texts) {
        Ok(m) if m.whole.is_some() => {
            ui.button(
                x,
                y,
                None,
                36.0,
                &format!("Restore the wallet · {} shares", texts.len()),
                Style::Primary,
                Action::RestoreShares,
            ) + 8.0
        }
        Ok(m) => {
            let line = format!("{} of {} keys in hand", m.have.len(), m.n);
            ui.text_mid(x, y, 36.0, 13.0, W::R, WARN, &line);
            ui.measure(13.0, W::R, &line) + 16.0
        }
        Err(e) => {
            ui.text_mid(x, y, 36.0, 13.0, W::R, ERR, &e);
            ui.measure(13.0, W::R, &e) + 16.0
        }
    }
}

fn files(app: &Faraday, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    if ui.compact {
        return files_compact(app, ui, x0, cw, h);
    }
    let x = x0 + 48.0;
    let width = cw - 96.0;
    let mut y = 36.0;
    title(ui, x, y, "Files");
    let label = "In memory · emptied at power-off";
    let pw = ui.measure(12.0, W::R, label) + 24.0;
    ui.pill(x + width - pw, y + 2.0, 30.0, label);
    let sw = ui.measure(13.0, W::S, "Scan a QR code") + 32.0;
    ui.button(
        x + width - pw - 12.0 - sw,
        y,
        Some(sw),
        36.0,
        "Scan a QR code",
        Style::Secondary,
        Action::Scan,
    );
    y += 64.0;
    y += guide_text(
        app,
        ui,
        x,
        y,
        width.min(900.0),
        &format!(
            "{} holds what was copied in from {}; {} holds what waits to be written to one, and what \
             the last visit wrote. Both live in memory and are emptied at power-off. Public files are \
             listed apart from vaults, which carry secrets sealed under a passphrase; a secret goes out \
             unprotected only when you say so after a warning, and is listed apart.",
            app.medium.from_box(),
            app.medium.nouns(),
            app.medium.for_box()
        ),
    );
    // Arrived from Import and load: pulling the stick is the next step.
    if !app.sticks.is_empty() && !app.visit.load_after.is_empty() {
        ui.fill(x, y, width, 48.0, 10.0, WARN.with_alpha(22));
        ui.stroke(x, y, width, 48.0, 10.0, WARN.with_alpha(90));
        ui.icon(x + 16.0, y + 14.0, 20.0, app.medium.icon(), 13.0, WARN);
        ui.text_mid(x + 44.0, y, 48.0, 14.0, W::S, WARN, &pull_line(app));
        y += 60.0;
    }
    for v in 0..app.vaults.open.len() {
        y += vault_panel(app, ui, v, x, y, width) + 20.0;
    }
    let ip = inbox_panel(app, ui, x, y, width);
    if ip > 0.0 {
        y += ip + 20.0;
    }
    let colw = (width - 20.0) / 2.0;
    // Inbox.
    let lw = ui.text(x, y, 15.0, W::S, MUTED, app.medium.from_box());
    ui.text(
        x + lw + 10.0,
        y + 2.0,
        13.0,
        W::R,
        DIM,
        &app.inbox.len().to_string(),
    );
    // The two lists scroll together under their titles, above the foot.
    let top = y + 28.0;
    let bottom = h - 32.0 - 48.0 - 12.0;
    let groups = 3.0;
    // The receipt under For the stick: its heading and a row per file.
    let receipt_h = app.receipt.as_ref().map_or(0.0, |r| {
        30.0 + r.files.len() as f32 * 28.0 + if app.outbox.is_empty() { 84.0 } else { 0.0 }
    });
    let content = (app.inbox.len() as f32 * 124.0)
        .max(app.outbox.len() as f32 * 124.0 + groups * 30.0 + receipt_h)
        + 8.0;
    let shift = app.list_offset.min((content - (bottom - top)).max(0.0));
    let clip = ui.rect(x0, top, cw, (bottom - top).max(0.0));
    ui.c.push_clip(clip);
    let mut iy = y + 32.0 - shift;
    for (k, item) in app.inbox.iter().enumerate() {
        ui.card(x, iy, colw, 112.0, LINE);
        ui.fill(x + 16.0, iy + 14.0, 36.0, 36.0, 10.0, ACCENT.with_alpha(26));
        ui.icon(x + 16.0, iy + 14.0, 36.0, Icon::Download, 14.0, ACCENT);
        let name = ui.fit(14.0, W::M, &item.name, colw - 90.0);
        ui.text(x + 66.0, iy + 14.0, 14.0, W::M, TEXT, &name);
        let line = if item.kind == FileKind::Vault
            && app
                .vault_files()
                .iter()
                .any(|f| f.name == item.name && f.open.is_some())
        {
            kind_line(item.kind, item.bytes.len()).replace("locked", "open")
        } else {
            kind_line(item.kind, item.bytes.len())
        };
        ui.text(x + 66.0, iy + 36.0, 12.0, W::R, MUTED, &line);
        let mut bx = x + 66.0;
        let same_spend = app.spend.as_ref().is_some_and(|s| {
            crate::wallet::read_psbt(&item.bytes)
                .map(|p| {
                    p.unsigned_tx().compute_txid() == s.spend.txid && item.name != s.spend.source
                })
                .unwrap_or(false)
        });
        match item.kind {
            FileKind::Psbt if same_spend => {
                bx += ui.button(
                    bx,
                    iy + 64.0,
                    None,
                    36.0,
                    "Add to signatures",
                    Style::Primary,
                    Action::Collect(k),
                ) + 8.0;
            }
            FileKind::Entries | FileKind::Backup => {
                let action = if item.kind == FileKind::Entries {
                    crate::vaults::VaultAction::ImportEntries(k)
                } else {
                    crate::vaults::VaultAction::ImportBackup(k)
                };
                // No vault open: make or unlock one, and back here.
                let (label, action) = app
                    .vault_way(Screen::Files)
                    .unwrap_or(("Import into the vault".to_string(), Action::Vault(action)));
                bx += ui.button(bx, iy + 64.0, None, 36.0, &label, Style::Primary, action) + 8.0;
            }
            FileKind::Vault => {
                let files = app.vault_files();
                let at = files.iter().position(|f| f.name == item.name).unwrap_or(0);
                let open = files.get(at).is_some_and(|f| f.open.is_some());
                bx += ui.button(
                    bx,
                    iy + 64.0,
                    None,
                    36.0,
                    if open { "Contents" } else { "Unlock" },
                    if open {
                        Style::Secondary
                    } else {
                        Style::Primary
                    },
                    Action::Vault(crate::vaults::VaultAction::Open(at)),
                ) + 8.0;
            }
            FileKind::Carry => {
                bx += ui.button(
                    bx,
                    iy + 64.0,
                    None,
                    36.0,
                    "Sign",
                    Style::Primary,
                    Action::StartSpend(k),
                ) + 8.0;
            }
            FileKind::Psbt => {
                let style = if app.session.wallets.is_empty() && app.session.keys.is_empty() {
                    Style::Secondary
                } else {
                    Style::Primary
                };
                bx += ui.button(
                    bx,
                    iy + 64.0,
                    None,
                    36.0,
                    "Sign",
                    style,
                    Action::StartSpend(k),
                ) + 8.0;
            }
            FileKind::Wallet => {
                bx += ui.button(
                    bx,
                    iy + 64.0,
                    None,
                    36.0,
                    "Load the wallet",
                    Style::Primary,
                    Action::LoadWallet(k),
                ) + 8.0;
                bx += add_to_vault(app, ui, bx, iy + 64.0, k, Style::Secondary);
            }
            FileKind::Text => {
                bx += add_to_vault(app, ui, bx, iy + 64.0, k, Style::Primary);
            }
            FileKind::Transaction => {
                bx += ui.button(
                    bx,
                    iy + 64.0,
                    None,
                    36.0,
                    "Decode",
                    Style::Primary,
                    Action::DecodeInbox(k),
                ) + 8.0;
            }
            FileKind::Share => {
                bx += restore_shares_button(app, ui, bx, iy + 64.0);
            }
            FileKind::Words => {
                bx += ui.button(
                    bx,
                    iy + 64.0,
                    None,
                    36.0,
                    "Load this key",
                    Style::Primary,
                    Action::LoadKey(k),
                ) + 8.0;
                bx += add_to_vault(app, ui, bx, iy + 64.0, k, Style::Secondary);
            }
            FileKind::Message => {
                bx += ui.button(
                    bx,
                    iy + 64.0,
                    None,
                    36.0,
                    "Check the signature",
                    Style::Primary,
                    Action::CheckMessage(k),
                ) + 8.0;
            }
            FileKind::Sheet if app.online => {
                bx += ui.button(
                    bx,
                    iy + 64.0,
                    None,
                    36.0,
                    "Make PDF",
                    Style::Primary,
                    Action::PdfInbox(k),
                ) + 8.0;
            }
            _ => {}
        }
        ui.button(
            bx,
            iy + 64.0,
            None,
            36.0,
            "Remove",
            Style::Ghost,
            Action::InboxRemove(k),
        );
        iy += 124.0;
    }
    if app.inbox.is_empty() {
        ui.stroke(x, iy, colw, 72.0, 12.0, BORDER);
        let t = &format!("Empty · files are copied in on {} visit", app.medium.a());
        let tw = ui.measure(13.0, W::R, t);
        ui.text_mid(x + (colw - tw) / 2.0, iy, 72.0, 13.0, W::R, DIM, t);
    }
    // Outbox, its title above the scrolled lists.
    let ox = x + colw + 20.0;
    ui.c.pop_clip();
    let lw = ui.text(ox, y, 15.0, W::S, MUTED, app.medium.for_box());
    ui.text(
        ox + lw + 10.0,
        y + 2.0,
        13.0,
        W::R,
        DIM,
        &app.outbox.len().to_string(),
    );
    ui.c.push_clip(clip);
    let mut oy = y + 32.0 - shift;
    // Grouped by who may read them: public files, vaults sealed under a
    // passphrase, and any secret the person let out unprotected.
    use crate::secrets::Exposure;
    for (exposure, label, line, tone) in [
        (
            Exposure::Public,
            "Public",
            "Anyone may read these".to_string(),
            MUTED,
        ),
        (
            Exposure::Sealed,
            "Written sealed",
            format!(
                "Sealed under their passphrases before they reach {}",
                app.medium.a()
            ),
            OK,
        ),
        (
            Exposure::Secret,
            "Unprotected secrets",
            format!("Anyone who has the {} can use these", app.medium.noun()),
            ERR,
        ),
    ] {
        let group: Vec<(usize, &crate::Item)> = app
            .outbox
            .iter()
            .enumerate()
            .filter(|(_, i)| i.exposure() == exposure)
            .collect();
        if group.is_empty() {
            continue;
        }
        let lw = ui.text(ox, oy, 12.0, W::S, tone, label);
        ui.text(ox + lw + 10.0, oy, 12.0, W::R, DIM, &line);
        oy += 24.0;
        for (k, item) in group {
            let (edge, icon, ink) = match exposure {
                Exposure::Public => (LINE, Icon::Export, ACCENT),
                Exposure::Sealed => (OK.with_alpha(70), Icon::Lock, OK),
                Exposure::Secret => (ERR.with_alpha(110), Icon::Lock, ERR),
            };
            ui.card(ox, oy, colw, 112.0, edge);
            ui.fill(ox + 16.0, oy + 14.0, 36.0, 36.0, 10.0, ink.with_alpha(26));
            ui.icon(ox + 16.0, oy + 14.0, 36.0, icon, 14.0, ink);
            let name = ui.fit(14.0, W::M, &item.name, colw - 160.0);
            ui.text(ox + 66.0, oy + 14.0, 14.0, W::M, TEXT, &name);
            // A vault open in this session: the file here is the copy
            // sealed before it was opened, and Lock seals it again.
            let open_here = (item.kind == FileKind::Vault)
                .then(|| faraday_vault::read_header(&item.bytes).ok())
                .flatten()
                .and_then(|h| app.vaults.open.iter().find(|v| v.header().salt == h.salt));
            let line = match open_here {
                Some(v) if v.changes > 0 => format!(
                    "Vault, open · {} unsaved {} · sealed and locked at Lock",
                    v.changes,
                    if v.changes == 1 { "change" } else { "changes" }
                ),
                Some(_) => "Vault, open · sealed and locked at Lock".to_string(),
                None => kind_line(item.kind, item.bytes.len()),
            };
            let line = ui.fit(12.0, W::R, &line, colw - 160.0);
            ui.text(ox + 66.0, oy + 36.0, 12.0, W::R, MUTED, &line);
            let state = match exposure {
                Exposure::Secret => ("Unprotected", ERR),
                _ if open_here.is_some() => ("Written after Lock", ACCENT),
                _ => ("Ready", OK),
            };
            ui.text_right(
                ox + colw - 16.0,
                oy + 14.0,
                20.0,
                12.0,
                W::R,
                state.1,
                state.0,
            );
            // A vault is larger than a QR transfer carries: it goes by
            // stick only.
            let shown_as = if item.kind == FileKind::Sheet && app.online {
                Some(("Make PDF", Action::PdfOutbox(k)))
            } else if crate::qr_fits(item) {
                Some(("Show as QR", Action::QrOutbox(k)))
            } else {
                None
            };
            let qw = match shown_as {
                Some((label, action)) => {
                    ui.button(
                        ox + 66.0,
                        oy + 64.0,
                        None,
                        36.0,
                        label,
                        Style::Secondary,
                        action,
                    ) + 8.0
                }
                None => 0.0,
            };
            ui.button(
                ox + 66.0 + qw,
                oy + 64.0,
                None,
                36.0,
                "Remove",
                Style::Ghost,
                Action::OutboxRemove(k),
            );
            oy += 124.0;
        }
        oy += 6.0;
    }
    if app.outbox.is_empty() {
        ui.stroke(ox, oy, colw, 72.0, 12.0, BORDER);
        let t = "Empty";
        let tw = ui.measure(13.0, W::R, t);
        ui.text_mid(ox + (colw - tw) / 2.0, oy, 72.0, 13.0, W::R, DIM, t);
        oy += 84.0;
    }
    if let Some(r) = app.receipt.as_ref() {
        receipt_rows(ui, r, ox, oy, colw);
    }
    ui.c.pop_clip();
    ui.report_scroll(clip, content - (bottom - top));
    // Foot.
    let fy = h - 32.0 - 48.0;
    if !app.outbox.is_empty() {
        ui.icon(x, fy + 14.0, 20.0, Icon::Power, 11.0, MUTED);
        ui.text_mid(
            x + 26.0,
            fy,
            48.0,
            12.0,
            W::R,
            MUTED,
            &format!(
                "Power off asks first while files wait {}",
                app.medium.for_the()
            ),
        );
    }
    let (label, action) = if app.sticks.is_empty() {
        (format!("Waiting for {}", app.medium.a()), None)
    } else if app.holds_secret() {
        (
            format!("{} attached · lock to use it", app.medium.cap()),
            Some(Action::Lock),
        )
    } else {
        (
            format!(
                "Visit {}",
                app.sticks[app.visit.stick.min(app.sticks.len() - 1)].label
            ),
            Some(Action::Nav(Screen::Visit)),
        )
    };
    let bw = ui.measure(14.0, W::R, &label) + 52.0;
    let bx = x + width - bw;
    ui.fill(bx, fy, bw, 48.0, 12.0, ACCENT.with_alpha(16));
    ui.stroke(bx, fy, bw, 48.0, 12.0, ACCENT.with_alpha(110));
    ui.dot(bx + 22.0, fy + 24.0, 4.0, ACCENT);
    ui.text_mid(bx + 36.0, fy, 48.0, 14.0, W::R, TEXT, &label);
    if let Some(a) = action {
        ui.hit(bx, fy, bw, 48.0, a);
    }
}

/// The last visit's receipt under For the stick: "Written to STICK at
/// 14:02", then a row per file, "verified" or why it was not. Nothing on
/// it is written again. Returns its height.
fn receipt_rows(ui: &mut Ui, r: &crate::Receipt, x: f32, y: f32, w: f32) -> f32 {
    let head = ui.fit(12.0, W::S, &r.heading(), w);
    ui.text(x, y, 12.0, W::S, OK, &head);
    let mut ry = y + 30.0;
    for f in &r.files {
        let (state, tone) = match &f.failure {
            None => ("verified".to_string(), OK),
            Some(why) => (why.clone(), ERR),
        };
        let state = ui.fit(12.0, W::R, &state, w / 2.0);
        let sw = ui.measure(12.0, W::R, &state);
        ui.text_right(x + w, ry - 4.0, 20.0, 12.0, W::R, tone, &state);
        let name = ui.fit(13.0, W::M, &f.name, (w - sw - 12.0).max(0.0));
        ui.text(x, ry, 13.0, W::M, TEXT, &name);
        ry += 28.0;
    }
    ry - y
}

// ---------------------------------------------------------------------
// Stick visit
// ---------------------------------------------------------------------

/// An open vault's keys and wallets on Files, each chosen or not, with
/// loading every one or the chosen ones into Wallets. Returns its height.
/// An open vault's seeds and wallets, each ticked to load or not, with
/// Select all and a Load button that counts what it loads. What loaded
/// stays listed, marked Loaded.
pub(crate) fn vault_panel(app: &Faraday, ui: &mut Ui, v: usize, x: f32, y: f32, w: f32) -> f32 {
    if ui.compact {
        return vault_panel_compact(app, ui, v, x, y, w);
    }
    use crate::vaults::VaultAction as V;
    let Some(open) = app.vaults.open.get(v) else {
        return 0.0;
    };
    let rows = app.vault_rows(v);
    let waiting: Vec<&crate::vaults::VaultRow> = rows.iter().filter(|r| !r.loaded).collect();
    let per_col = rows.len().div_ceil(2).max(1);
    // Wallets this vault holds with seeds of theirs: one press loads each
    // with its seeds.
    let together: Vec<crate::vaults::VaultWallet> = app
        .vault_wallets_with_keys(v)
        .into_iter()
        .filter(|w| !w.done)
        .collect();
    // Three rows at most: the rest are ticked in the full list below,
    // which shares the same choice.
    const ROWS: usize = 3;
    let more = together.len().saturating_sub(ROWS);
    let top = if together.is_empty() {
        0.0
    } else {
        30.0 + together.len().min(ROWS) as f32 * 40.0 + 64.0 + if more > 0 { 26.0 } else { 0.0 }
    };
    // With wallets and their seeds to load together, the list of each
    // seed and wallet folds away until asked for.
    let folded = !together.is_empty() && !open.each_shown;
    let h = if folded {
        52.0 + top + 8.0
    } else {
        132.0 + per_col as f32 * 38.0 + top
    };
    ui.card(x, y, w, h, LINE);
    ui.icon(x + 20.0, y + 18.0, 20.0, Icon::Lock, 13.0, ACCENT);
    ui.text_mid(
        x + 48.0,
        y + 14.0,
        28.0,
        15.0,
        W::S,
        TEXT,
        &format!("{} · {} · open", open.name, open.label()),
    );
    // Select all: every row not loaded yet, or none when all are chosen.
    if !waiting.is_empty() && !folded {
        let all = waiting.iter().all(|r| r.chosen);
        let label = "Select all";
        let lw = ui.measure(13.0, W::S, label);
        let sx = x + w - 24.0 - lw - 28.0;
        ui.checkbox(sx, y + 19.0, all, true);
        ui.text_mid(sx + 28.0, y + 14.0, 28.0, 13.0, W::S, MUTED, label);
        ui.hit(
            sx - 8.0,
            y + 8.0,
            lw + 44.0,
            40.0,
            Action::Vault(V::ChooseAll(v)),
        );
    }
    if !together.is_empty() {
        let mut ty = y + 52.0;
        ui.text(x + 20.0, ty, 13.0, W::S, MUTED, "Wallets with their seeds");
        ty += 26.0;
        let many = together.len() > 1;
        for wt in together.iter().take(ROWS) {
            let action = Action::Vault(V::Choose(v, wt.record));
            let mut rx = x + 20.0;
            if many {
                ui.checkbox(rx, ty + 9.0, wt.chosen, true);
                rx += 28.0;
            }
            ui.icon(rx, ty + 8.0, 20.0, Icon::Wallet, 11.0, MUTED);
            let nw = ui.text_mid(rx + 26.0, ty, 36.0, 14.0, W::S, TEXT, &wt.name);
            let here = wt.keys.len();
            let line = format!(
                "{here} of {} {} in this vault",
                wt.total,
                if wt.total == 1 { "seed" } else { "seeds" }
            );
            ui.text_mid(rx + 40.0 + nw, ty, 36.0, 12.0, W::R, MUTED, &line);
            if here >= wt.needed {
                let tag = "Ready to sign once loaded";
                let tw = ui.measure(12.0, W::S, tag) + 20.0;
                ui.fill(
                    x + w - 20.0 - tw,
                    ty + 6.0,
                    tw,
                    24.0,
                    12.0,
                    OK.with_alpha(30),
                );
                ui.text_mid(x + w - 10.0 - tw, ty, 36.0, 12.0, W::S, OK, tag);
            }
            if many {
                ui.hit(x + 16.0, ty, w - 32.0, 36.0, action);
            }
            ty += 40.0;
        }
        if more > 0 {
            ui.text_mid(
                x + 20.0,
                ty - 4.0,
                26.0,
                12.0,
                W::R,
                MUTED,
                &format!("+ {more} more · listed under Each seed and wallet"),
            );
            ty += 26.0;
        }
        let chosen = together.iter().filter(|w| w.chosen || !many).count();
        let label = match chosen {
            1 if !many => "Load wallet with keys".to_string(),
            0 => "Nothing chosen".to_string(),
            1 => "Load 1 wallet with keys".to_string(),
            n => format!("Load {n} wallets with keys"),
        };
        let lw = ui.button(
            x + 20.0,
            ty + 4.0,
            None,
            40.0,
            &label,
            if chosen > 0 {
                Style::Primary
            } else {
                Style::Disabled
            },
            Action::Vault(V::LoadWithKeys(v)),
        );
        let each = if open.each_shown {
            "Hide each seed and wallet".to_string()
        } else {
            format!("Each seed and wallet · {}", rows.len())
        };
        let ew = ui.button(
            x + 20.0 + lw + 12.0,
            ty + 4.0,
            None,
            40.0,
            &each,
            Style::Ghost,
            Action::Vault(V::EachShown(v)),
        );
        if folded && rows.iter().any(|r| r.loaded) && app.screen != Screen::Family {
            ui.button(
                x + 20.0 + lw + ew + 24.0,
                ty + 4.0,
                None,
                40.0,
                "Go to Wallets",
                Style::Secondary,
                Action::Nav(Screen::Start),
            );
        }
        if !folded {
            ui.rule(x + 20.0, ty + 56.0, w - 40.0, INNER);
        }
    }
    if folded {
        return h;
    }
    let colw = (w - 60.0) / 2.0;
    for (k, row) in rows.iter().enumerate() {
        let rx = x + 20.0 + (k / per_col) as f32 * (colw + 20.0);
        let ry = y + 56.0 + top + (k % per_col) as f32 * 38.0;
        if row.loaded {
            ui.icon(rx - 2.0, ry + 6.0, 22.0, Icon::Done, 12.0, OK);
        } else {
            ui.checkbox(rx, ry + 8.0, row.chosen, true);
        }
        let icon = if row.seed { Icon::Keys } else { Icon::Wallet };
        ui.icon(rx + 26.0, ry + 7.0, 20.0, icon, 11.0, MUTED);
        let name = ui.fit(14.0, W::S, &row.name, colw * 0.5 - 24.0);
        ui.text_mid(rx + 52.0, ry, 34.0, 14.0, W::S, TEXT, &name);
        let detail = if row.loaded {
            "Loaded".to_string()
        } else {
            row.detail.clone()
        };
        let face = if row.seed && !row.loaded { W::M } else { W::R };
        let detail = ui.fit(12.0, face, &detail, colw * 0.45);
        ui.text_right(
            rx + colw,
            ry,
            34.0,
            12.0,
            face,
            if row.loaded { OK } else { MUTED },
            &detail,
        );
        if !row.loaded {
            ui.hit(rx, ry, colw, 34.0, Action::Vault(V::Choose(v, row.record)));
        }
    }
    let by = y + h - 20.0 - 40.0;
    let seeds = waiting.iter().filter(|r| r.chosen && r.seed).count();
    let wallets = waiting.iter().filter(|r| r.chosen && !r.seed).count();
    let count =
        |n: usize, one: &str, many: &str| format!("{n} {}", if n == 1 { one } else { many });
    let label = match (seeds, wallets) {
        (0, 0) if waiting.is_empty() => "Everything is loaded".to_string(),
        (0, 0) => "Nothing chosen".to_string(),
        (s, 0) => format!("Load {}", count(s, "seed", "seeds")),
        (0, wl) => format!("Load {}", count(wl, "wallet", "wallets")),
        (s, wl) => format!(
            "Load {} and {}",
            count(s, "seed", "seeds"),
            count(wl, "wallet", "wallets")
        ),
    };
    let bw = ui.button(
        x + 20.0,
        by,
        None,
        40.0,
        &label,
        if seeds + wallets > 0 {
            Style::Primary
        } else {
            Style::Disabled
        },
        Action::Vault(V::LoadChosen(v)),
    );
    // Once something has loaded, the way on: Files stays put.
    if rows.iter().any(|r| r.loaded) && app.screen != Screen::Family {
        ui.button(
            x + 20.0 + bw + 12.0,
            by,
            None,
            40.0,
            "Go to Wallets",
            Style::Secondary,
            Action::Nav(Screen::Start),
        );
    }
    h
}

/// [`inbox_panel`] on a small panel: one column, each row's line under
/// its name and its button under that, measured before its card is drawn.
fn inbox_panel_compact(app: &Faraday, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    use crate::compact_screens::measured;
    let found = app.inbox_found();
    let wallets = found.to_load();
    let potential: Vec<&crate::inbox::FoundSeed> = found.potential();
    if wallets.is_empty()
        && potential.is_empty()
        && found.waiting.is_empty()
        && found.xpubs.is_empty()
    {
        return 0.0;
    }
    const ROWS: usize = 3;
    let may = app.may_load_keys();
    let (ix, iw) = (x + 12.0, w - 24.0);
    let draw = |ui: &mut Ui| -> f32 {
        let mut ty = y + 14.0;
        ui.icon(ix, ty, 20.0, Icon::Download, 13.0, ACCENT);
        ui.text_mid(
            ix + 28.0,
            ty - 4.0,
            28.0,
            15.0,
            W::S,
            TEXT,
            app.medium.from_box(),
        );
        ty += 36.0;
        if !wallets.is_empty() {
            ui.text(ix, ty, 13.0, W::S, MUTED, "Wallets");
            ty += 24.0;
            let many = wallets.len() > 1;
            for (i, fw) in wallets.iter().enumerate().take(ROWS) {
                let chosen = !app.inbox_skip.contains(&fw.descriptor);
                let mut rx = ix;
                let top = ty;
                if many {
                    ui.checkbox(rx, ty + 4.0, chosen, true);
                    rx += 28.0;
                }
                ui.icon(rx, ty + 2.0, 20.0, Icon::Wallet, 11.0, MUTED);
                let name = ui.fit(14.0, W::S, &fw.name, ix + iw - rx - 26.0);
                ui.text(rx + 26.0, ty + 2.0, 14.0, W::S, TEXT, &name);
                ty += 24.0;
                let n = fw.keys.len().max(1);
                let line = format!(
                    "{} · {} of {n} {} here{}",
                    fw.shape,
                    fw.seeds_here,
                    if n == 1 { "seed" } else { "seeds" },
                    if fw.files.len() > 1 {
                        format!(" · {} files say it", fw.files.len())
                    } else if fw.from_shares {
                        " · from split sheets".to_string()
                    } else {
                        String::new()
                    }
                );
                let line = ui.fit(12.0, W::R, &line, ix + iw - rx - 26.0);
                ui.text(rx + 26.0, ty, 12.0, W::R, MUTED, &line);
                ty += 20.0;
                if fw.seeds_here >= fw.needed {
                    ui.text(rx + 26.0, ty, 12.0, W::S, OK, "Ready to sign once loaded");
                    ty += 20.0;
                }
                if many {
                    ui.hit(ix - 4.0, top, iw + 8.0, ty - top, Action::InboxChoose(i));
                }
                ty += 8.0;
            }
            if wallets.len() > ROWS {
                let more = format!("+ {} more, loaded with these", wallets.len() - ROWS);
                ty += ui.wrap(ix, ty, iw, 12.0, W::R, MUTED, &more) + 8.0;
            }
            let chosen = wallets
                .iter()
                .filter(|w| !many || !app.inbox_skip.contains(&w.descriptor))
                .count();
            let with_seeds = wallets
                .iter()
                .filter(|w| !many || !app.inbox_skip.contains(&w.descriptor))
                .any(|w| w.seeds_here > 0);
            let label = match (chosen, with_seeds) {
                (0, _) => "Nothing chosen".to_string(),
                (1, true) => "Load the wallet with its seeds".to_string(),
                (1, false) => "Load the wallet".to_string(),
                (n, true) => format!("Load {n} wallets with their seeds"),
                (n, false) => format!("Load {n} wallets"),
            };
            let style = if chosen > 0 {
                Style::Primary
            } else {
                Style::Disabled
            };
            ty += wrap_buttons(ui, ix, ty, iw, 40.0, &[(&label, style, Action::InboxLoad)]);
            if !may {
                ty += ui.wrap(
                    ix,
                    ty,
                    iw,
                    12.0,
                    W::R,
                    WARN,
                    &format!("Seeds load once the {} is out", app.medium.noun()),
                ) + 8.0;
            }
            ty += 6.0;
        }
        if !potential.is_empty() {
            ui.text(ix, ty, 13.0, W::S, MUTED, "Seeds with no wallet");
            ty += 24.0;
            for s in potential.iter().take(ROWS) {
                ui.icon(ix, ty + 2.0, 20.0, Icon::Keys, 11.0, MUTED);
                ui.text(
                    ix + 26.0,
                    ty + 2.0,
                    14.0,
                    W::M,
                    TEXT,
                    &fp_text(s.fingerprint),
                );
                ty += 24.0;
                let from: Vec<String> = s.sources.iter().map(|src| src.name(app)).collect();
                let line = ui.fit(12.0, W::R, &from.join(" · "), iw - 26.0);
                ui.text(ix + 26.0, ty, 12.0, W::R, MUTED, &line);
                ty += 22.0;
                let style = if may {
                    Style::Secondary
                } else {
                    Style::Disabled
                };
                ty += wrap_buttons(
                    ui,
                    ix + 26.0,
                    ty,
                    iw - 26.0,
                    32.0,
                    &[(
                        "Make it a wallet",
                        style,
                        Action::PotentialOpen(s.fingerprint.0),
                    )],
                );
            }
            if potential.len() > ROWS {
                let more = format!("+ {} more", potential.len() - ROWS);
                ty += ui.wrap(ix, ty, iw, 12.0, W::R, MUTED, &more) + 8.0;
            }
            ty += 4.0;
        }
        if !found.xpubs.is_empty() {
            ui.text(ix, ty, 13.0, W::S, MUTED, "Xpubs with no wallet");
            ty += 24.0;
            for (i, xp) in found.xpubs.iter().enumerate().take(ROWS) {
                ui.icon(ix, ty + 2.0, 20.0, Icon::Wallet, 11.0, MUTED);
                let fp = xp
                    .fingerprint
                    .map(fp_text)
                    .unwrap_or_else(|| "no origin".to_string());
                ui.text(ix + 26.0, ty + 2.0, 14.0, W::M, TEXT, &fp);
                ty += 24.0;
                let line = format!(
                    "{} · {}",
                    xp.path.clone().unwrap_or_default(),
                    xp.files
                        .first()
                        .and_then(|k| app.inbox.get(*k))
                        .map_or(String::new(), |it| it.name.clone())
                );
                let line = ui.fit(12.0, W::R, &line, iw - 26.0);
                ui.text(ix + 26.0, ty, 12.0, W::R, MUTED, &line);
                ty += 22.0;
                ty += wrap_buttons(
                    ui,
                    ix + 26.0,
                    ty,
                    iw - 26.0,
                    32.0,
                    &[(
                        "Make it a watch-only wallet",
                        Style::Secondary,
                        Action::XpubOpen(i),
                    )],
                );
            }
            ty += 4.0;
        }
        if !found.waiting.is_empty() {
            ui.text(ix, ty, 13.0, W::S, MUTED, "Not complete or sealed");
            ty += 24.0;
            for q in found.waiting.iter().take(ROWS) {
                ty += ui.wrap(ix, ty, iw, 13.0, W::R, MUTED, &q.line) + 8.0;
                if let Some(k) = q.backup {
                    let style = if may {
                        Style::Secondary
                    } else {
                        Style::Disabled
                    };
                    ty += wrap_buttons(
                        ui,
                        ix,
                        ty,
                        iw,
                        32.0,
                        &[("Open with its passphrase", style, Action::BackupOpen(k))],
                    );
                }
            }
        }
        ty - y + 6.0
    };
    let h = measured(ui, |ui| draw(ui));
    ui.card(x, y, w, h, LINE);
    draw(ui);
    h
}

/// [`vault_panel`] on a small panel: one column of the vault's wallets
/// and seeds, each name over what it is, measured before its card.
fn vault_panel_compact(app: &Faraday, ui: &mut Ui, v: usize, x: f32, y: f32, w: f32) -> f32 {
    use crate::compact_screens::measured;
    use crate::vaults::VaultAction as V;
    let Some(open) = app.vaults.open.get(v) else {
        return 0.0;
    };
    let rows = app.vault_rows(v);
    let waiting: Vec<&crate::vaults::VaultRow> = rows.iter().filter(|r| !r.loaded).collect();
    let together: Vec<crate::vaults::VaultWallet> = app
        .vault_wallets_with_keys(v)
        .into_iter()
        .filter(|w| !w.done)
        .collect();
    const ROWS: usize = 3;
    let more = together.len().saturating_sub(ROWS);
    let folded = !together.is_empty() && !open.each_shown;
    let (ix, iw) = (x + 12.0, w - 24.0);
    let go = rows.iter().any(|r| r.loaded) && app.screen != Screen::Family;
    let draw = |ui: &mut Ui| -> f32 {
        let mut ty = y + 14.0;
        ui.icon(ix, ty, 20.0, Icon::Lock, 13.0, ACCENT);
        let title = format!("{} · {} · open", open.name, open.label());
        let title = ui.fit(15.0, W::S, &title, iw - 28.0);
        ui.text_mid(ix + 28.0, ty - 4.0, 28.0, 15.0, W::S, TEXT, &title);
        ty += 36.0;
        if !together.is_empty() {
            ui.text(ix, ty, 13.0, W::S, MUTED, "Wallets with their seeds");
            ty += 24.0;
            let many = together.len() > 1;
            for wt in together.iter().take(ROWS) {
                let top = ty;
                let mut rx = ix;
                if many {
                    ui.checkbox(rx, ty + 4.0, wt.chosen, true);
                    rx += 28.0;
                }
                ui.icon(rx, ty + 2.0, 20.0, Icon::Wallet, 11.0, MUTED);
                let name = ui.fit(14.0, W::S, &wt.name, ix + iw - rx - 26.0);
                ui.text(rx + 26.0, ty + 2.0, 14.0, W::S, TEXT, &name);
                ty += 24.0;
                let here = wt.keys.len();
                let line = format!(
                    "{here} of {} {} in this vault",
                    wt.total,
                    if wt.total == 1 { "seed" } else { "seeds" }
                );
                ui.text(rx + 26.0, ty, 12.0, W::R, MUTED, &line);
                ty += 20.0;
                if here >= wt.needed {
                    ui.text(rx + 26.0, ty, 12.0, W::S, OK, "Ready to sign once loaded");
                    ty += 20.0;
                }
                if many {
                    ui.hit(
                        ix - 4.0,
                        top,
                        iw + 8.0,
                        ty - top,
                        Action::Vault(V::Choose(v, wt.record)),
                    );
                }
                ty += 8.0;
            }
            if more > 0 {
                let line = format!("+ {more} more · listed under Each seed and wallet");
                ty += ui.wrap(ix, ty, iw, 12.0, W::R, MUTED, &line) + 8.0;
            }
            let chosen = together.iter().filter(|w| w.chosen || !many).count();
            let label = match chosen {
                1 if !many => "Load wallet with keys".to_string(),
                0 => "Nothing chosen".to_string(),
                1 => "Load 1 wallet with keys".to_string(),
                n => format!("Load {n} wallets with keys"),
            };
            let each = if open.each_shown {
                "Hide each seed and wallet".to_string()
            } else {
                format!("Each seed and wallet · {}", rows.len())
            };
            let mut items: Vec<(&str, Style, Action)> = vec![
                (
                    &label,
                    if chosen > 0 {
                        Style::Primary
                    } else {
                        Style::Disabled
                    },
                    Action::Vault(V::LoadWithKeys(v)),
                ),
                (&each, Style::Ghost, Action::Vault(V::EachShown(v))),
            ];
            if folded && go {
                items.push((
                    "Go to Wallets",
                    Style::Secondary,
                    Action::Nav(Screen::Start),
                ));
            }
            ty += wrap_buttons(ui, ix, ty, iw, 40.0, &items);
            if !folded {
                ui.rule(ix, ty, iw, INNER);
                ty += 12.0;
            }
        }
        if folded {
            return ty - y + 4.0;
        }
        // Select all: every row not loaded yet, or none when all are chosen.
        if !waiting.is_empty() {
            let all = waiting.iter().all(|r| r.chosen);
            ui.checkbox(ix, ty + 4.0, all, true);
            ui.text(ix + 28.0, ty + 2.0, 13.0, W::S, MUTED, "Select all");
            ui.hit(
                ix - 4.0,
                ty - 4.0,
                iw + 8.0,
                32.0,
                Action::Vault(V::ChooseAll(v)),
            );
            ty += 32.0;
        }
        for row in &rows {
            if row.loaded {
                ui.icon(ix - 2.0, ty + 2.0, 22.0, Icon::Done, 12.0, OK);
            } else {
                ui.checkbox(ix, ty + 4.0, row.chosen, true);
            }
            let icon = if row.seed { Icon::Keys } else { Icon::Wallet };
            ui.icon(ix + 28.0, ty + 2.0, 20.0, icon, 11.0, MUTED);
            let detail = if row.loaded {
                "Loaded".to_string()
            } else {
                row.detail.clone()
            };
            let face = if row.seed && !row.loaded { W::M } else { W::R };
            let dw = ui.measure(12.0, face, &detail).min(iw * 0.4);
            let detail = ui.fit(12.0, face, &detail, dw);
            ui.text_right(
                ix + iw,
                ty,
                26.0,
                12.0,
                face,
                if row.loaded { OK } else { MUTED },
                &detail,
            );
            let name = ui.fit(14.0, W::S, &row.name, iw - 54.0 - dw - 8.0);
            ui.text(ix + 54.0, ty + 3.0, 14.0, W::S, TEXT, &name);
            if !row.loaded {
                ui.hit(
                    ix - 4.0,
                    ty - 4.0,
                    iw + 8.0,
                    34.0,
                    Action::Vault(V::Choose(v, row.record)),
                );
            }
            ty += 34.0;
        }
        ty += 6.0;
        let seeds = waiting.iter().filter(|r| r.chosen && r.seed).count();
        let wallets = waiting.iter().filter(|r| r.chosen && !r.seed).count();
        let count =
            |n: usize, one: &str, many: &str| format!("{n} {}", if n == 1 { one } else { many });
        let label = match (seeds, wallets) {
            (0, 0) if waiting.is_empty() => "Everything is loaded".to_string(),
            (0, 0) => "Nothing chosen".to_string(),
            (s, 0) => format!("Load {}", count(s, "seed", "seeds")),
            (0, wl) => format!("Load {}", count(wl, "wallet", "wallets")),
            (s, wl) => format!(
                "Load {} and {}",
                count(s, "seed", "seeds"),
                count(wl, "wallet", "wallets")
            ),
        };
        let mut items: Vec<(&str, Style, Action)> = vec![(
            &label,
            if seeds + wallets > 0 {
                Style::Primary
            } else {
                Style::Disabled
            },
            Action::Vault(V::LoadChosen(v)),
        )];
        if go {
            items.push((
                "Go to Wallets",
                Style::Secondary,
                Action::Nav(Screen::Start),
            ));
        }
        ty += wrap_buttons(ui, ix, ty, iw, 40.0, &items);
        ty - y + 4.0
    };
    let h = measured(ui, |ui| draw(ui));
    ui.card(x, y, w, h, LINE);
    draw(ui);
    h
}

/// Buttons by label, style and action.
type Buttons = Vec<(String, Style, Action)>;

/// A line and its colour.
type Note = Option<(String, osk_ui::Color)>;

/// What an Inbox file offers, in the order its card shows it: the way it
/// is used, then Remove. A split sheet that does not add up yet says how
/// many keys are in hand instead of offering a button.
fn inbox_actions(app: &Faraday, k: usize, item: &crate::Item) -> (Buttons, Note) {
    let mut out: Vec<(String, Style, Action)> = Vec::new();
    let mut note = None;
    // With no vault open, what goes into one offers Make a vault or
    // Unlock instead, which come back to Files.
    let into_vault = |label: &str, style, action| {
        let (label, action) = app
            .vault_way(Screen::Files)
            .unwrap_or((label.to_string(), action));
        (label, style, action)
    };
    let add_to_vault = |style| {
        into_vault(
            "Add to vault",
            style,
            Action::Vault(crate::vaults::VaultAction::AddFile(k)),
        )
    };
    let same_spend = app.spend.as_ref().is_some_and(|s| {
        crate::wallet::read_psbt(&item.bytes)
            .map(|p| p.unsigned_tx().compute_txid() == s.spend.txid && item.name != s.spend.source)
            .unwrap_or(false)
    });
    match item.kind {
        FileKind::Psbt if same_spend => {
            out.push((
                "Add to signatures".into(),
                Style::Primary,
                Action::Collect(k),
            ));
        }
        FileKind::Entries | FileKind::Backup => {
            let action = if item.kind == FileKind::Entries {
                crate::vaults::VaultAction::ImportEntries(k)
            } else {
                crate::vaults::VaultAction::ImportBackup(k)
            };
            out.push(into_vault(
                "Import into the vault",
                Style::Primary,
                Action::Vault(action),
            ));
        }
        FileKind::Vault => {
            let files = app.vault_files();
            let at = files.iter().position(|f| f.name == item.name).unwrap_or(0);
            let open = files.get(at).is_some_and(|f| f.open.is_some());
            out.push((
                if open { "Contents" } else { "Unlock" }.into(),
                if open {
                    Style::Secondary
                } else {
                    Style::Primary
                },
                Action::Vault(crate::vaults::VaultAction::Open(at)),
            ));
        }
        FileKind::Carry => out.push(("Sign".into(), Style::Primary, Action::StartSpend(k))),
        FileKind::Psbt => {
            let style = if app.session.wallets.is_empty() && app.session.keys.is_empty() {
                Style::Secondary
            } else {
                Style::Primary
            };
            out.push(("Sign".into(), style, Action::StartSpend(k)));
        }
        FileKind::Wallet => {
            out.push((
                "Load the wallet".into(),
                Style::Primary,
                Action::LoadWallet(k),
            ));
            out.push(add_to_vault(Style::Secondary));
        }
        FileKind::Text => out.push(add_to_vault(Style::Primary)),
        FileKind::Transaction => {
            out.push(("Decode".into(), Style::Primary, Action::DecodeInbox(k)));
        }
        FileKind::Share => {
            let texts: Vec<String> = app
                .inbox
                .iter()
                .filter(|i| i.kind == FileKind::Share)
                .map(|i| String::from_utf8_lossy(&i.bytes).into_owned())
                .collect();
            match crate::restore::merge(&texts) {
                Ok(m) if m.whole.is_some() => out.push((
                    format!("Restore the wallet · {} shares", texts.len()),
                    Style::Primary,
                    Action::RestoreShares,
                )),
                Ok(m) => note = Some((format!("{} of {} keys in hand", m.have.len(), m.n), WARN)),
                Err(e) => note = Some((e, ERR)),
            }
        }
        FileKind::Words => {
            out.push(("Load this key".into(), Style::Primary, Action::LoadKey(k)));
            out.push(add_to_vault(Style::Secondary));
        }
        FileKind::Message => {
            out.push((
                "Check the signature".into(),
                Style::Primary,
                Action::CheckMessage(k),
            ));
        }
        FileKind::Sheet if app.online => {
            out.push(("Make PDF".into(), Style::Primary, Action::PdfInbox(k)));
        }
        _ => {}
    }
    out.push(("Remove".into(), Style::Ghost, Action::InboxRemove(k)));
    (out, note)
}

/// One file's card on a small panel: its icon, name and line, a tag at
/// the right, and its buttons wrapped under them. Returns its height.
#[allow(clippy::too_many_arguments)]
fn file_card(
    ui: &mut Ui,
    x: f32,
    y: f32,
    w: f32,
    (icon, ink, edge): (Icon, osk_ui::Color, osk_ui::Color),
    name: &str,
    line: &str,
    tag: Option<(&str, osk_ui::Color)>,
    note: Option<&(String, osk_ui::Color)>,
    buttons: &[(String, Style, Action)],
) -> f32 {
    use crate::compact_screens::measured;
    let draw = |ui: &mut Ui| -> f32 {
        let mut cy = y + 12.0;
        ui.fill(x + 12.0, cy, 32.0, 32.0, 8.0, ink.with_alpha(26));
        ui.icon(x + 12.0, cy, 32.0, icon, 13.0, ink);
        let tw = tag.map_or(0.0, |(t, _)| ui.measure(12.0, W::S, t) + 8.0);
        let room = w - 56.0 - 12.0;
        let n = ui.fit(14.0, W::M, name, room - tw);
        ui.text(x + 56.0, cy, 14.0, W::M, TEXT, &n);
        if let Some((t, c)) = tag {
            ui.text_right(x + w - 12.0, cy - 2.0, 20.0, 12.0, W::S, c, t);
        }
        let l = ui.fit(12.0, W::R, line, room);
        ui.text(x + 56.0, cy + 20.0, 12.0, W::R, MUTED, &l);
        cy += 46.0;
        if let Some((t, c)) = note {
            cy += ui.wrap(x + 12.0, cy, w - 24.0, 13.0, W::R, *c, t) + 8.0;
        }
        let items: Vec<(&str, Style, Action)> = buttons
            .iter()
            .map(|(l, s, a)| (l.as_str(), *s, *a))
            .collect();
        cy += wrap_buttons(ui, x + 12.0, cy, w - 24.0, 36.0, &items);
        cy - y + 2.0
    };
    let h = measured(ui, |ui| draw(ui));
    ui.card(x, y, w, h, edge);
    draw(ui);
    h
}

/// [`files`] on a small panel: one scrolled column of what is loaded from
/// open vaults and the Inbox, then the Inbox, then the Outbox by who may
/// read it, then the stick.
fn files_compact(app: &Faraday, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    use crate::compact::M;
    use crate::compact_screens::{about, finish};
    use crate::secrets::Exposure;
    let (x, w) = (x0 + M, cw - 2.0 * M);
    let top = 12.0 - app.list_offset;
    let mut y = top;
    // The stick, or what waits for one.
    let (label, action) = if app.sticks.is_empty() {
        (format!("Waiting for {}", app.medium.a()), None)
    } else if app.holds_secret() {
        (
            format!("{} attached · lock to use it", app.medium.cap()),
            Some(Action::Lock),
        )
    } else {
        (
            format!(
                "Visit {}",
                app.sticks[app.visit.stick.min(app.sticks.len() - 1)].label
            ),
            Some(Action::Nav(Screen::Visit)),
        )
    };
    ui.fill(x, y, w, 44.0, 10.0, ACCENT.with_alpha(16));
    ui.stroke(x, y, w, 44.0, 10.0, ACCENT.with_alpha(110));
    ui.dot(x + 18.0, y + 22.0, 4.0, ACCENT);
    let label = ui.fit(14.0, W::R, &label, w - 44.0);
    ui.text_mid(x + 32.0, y, 44.0, 14.0, W::R, TEXT, &label);
    if let Some(a) = action {
        ui.hit(x, y, w, 44.0, a);
    }
    y += 52.0;
    ui.button(
        x,
        y,
        Some(w),
        40.0,
        "Scan a QR code",
        Style::Secondary,
        Action::Scan,
    );
    y += 48.0;
    ui.text(x, y, 12.0, W::R, DIM, "In memory · emptied at power-off");
    y += 26.0;
    // Arrived from Import and load: pulling the stick is the next step.
    if !app.sticks.is_empty() && !app.visit.load_after.is_empty() {
        y += crate::vault_screens::stick_banner(ui, x, y, w, &pull_line(app));
    }
    for v in 0..app.vaults.open.len() {
        y += vault_panel(app, ui, v, x, y, w) + 14.0;
    }
    let ip = inbox_panel(app, ui, x, y, w);
    if ip > 0.0 {
        y += ip + 14.0;
    }
    // Inbox.
    let lw = ui.text(x, y, 15.0, W::S, MUTED, app.medium.from_box());
    ui.text(
        x + lw + 10.0,
        y + 2.0,
        13.0,
        W::R,
        DIM,
        &app.inbox.len().to_string(),
    );
    y += 28.0;
    for (k, item) in app.inbox.iter().enumerate() {
        let line = if item.kind == FileKind::Vault
            && app
                .vault_files()
                .iter()
                .any(|f| f.name == item.name && f.open.is_some())
        {
            kind_line(item.kind, item.bytes.len()).replace("locked", "open")
        } else {
            kind_line(item.kind, item.bytes.len())
        };
        let (buttons, note) = inbox_actions(app, k, item);
        y += file_card(
            ui,
            x,
            y,
            w,
            (Icon::Download, ACCENT, LINE),
            &item.name,
            &line,
            None,
            note.as_ref(),
            &buttons,
        ) + 10.0;
    }
    if app.inbox.is_empty() {
        y += ui.wrap(
            x,
            y,
            w,
            13.0,
            W::R,
            DIM,
            &format!("Empty · files are copied in on {} visit", app.medium.a()),
        ) + 14.0;
    }
    // Outbox, grouped by who may read it.
    y += 8.0;
    let lw = ui.text(x, y, 15.0, W::S, MUTED, app.medium.for_box());
    ui.text(
        x + lw + 10.0,
        y + 2.0,
        13.0,
        W::R,
        DIM,
        &app.outbox.len().to_string(),
    );
    y += 28.0;
    for (exposure, label, line, tone) in [
        (
            Exposure::Public,
            "Public",
            "Anyone may read these".to_string(),
            MUTED,
        ),
        (
            Exposure::Sealed,
            "Written sealed",
            format!(
                "Sealed under their passphrases before they reach {}",
                app.medium.a()
            ),
            OK,
        ),
        (
            Exposure::Secret,
            "Unprotected secrets",
            format!("Anyone who has the {} can use these", app.medium.noun()),
            ERR,
        ),
    ] {
        let group: Vec<(usize, &crate::Item)> = app
            .outbox
            .iter()
            .enumerate()
            .filter(|(_, i)| i.exposure() == exposure)
            .collect();
        if group.is_empty() {
            continue;
        }
        ui.text(x, y, 12.0, W::S, tone, label);
        y += 18.0;
        y += ui.wrap(x, y, w, 12.0, W::R, DIM, &line) + 8.0;
        for (k, item) in group {
            let (edge, icon, ink) = match exposure {
                Exposure::Public => (LINE, Icon::Export, ACCENT),
                Exposure::Sealed => (OK.with_alpha(70), Icon::Lock, OK),
                Exposure::Secret => (ERR.with_alpha(110), Icon::Lock, ERR),
            };
            let open_here = (item.kind == FileKind::Vault)
                .then(|| faraday_vault::read_header(&item.bytes).ok())
                .flatten()
                .and_then(|h| app.vaults.open.iter().find(|v| v.header().salt == h.salt));
            let line = match open_here {
                Some(v) if v.changes > 0 => format!(
                    "Vault, open · {} unsaved {} · sealed and locked at Lock",
                    v.changes,
                    if v.changes == 1 { "change" } else { "changes" }
                ),
                Some(_) => "Vault, open · sealed and locked at Lock".to_string(),
                None => kind_line(item.kind, item.bytes.len()),
            };
            let state = match exposure {
                Exposure::Secret => ("Unprotected", ERR),
                _ if open_here.is_some() => ("Written after Lock", ACCENT),
                _ => ("Ready", OK),
            };
            let mut buttons: Vec<(String, Style, Action)> = Vec::new();
            if item.kind == FileKind::Sheet && app.online {
                buttons.push(("Make PDF".into(), Style::Secondary, Action::PdfOutbox(k)));
            } else if crate::qr_fits(item) {
                buttons.push(("Show as QR".into(), Style::Secondary, Action::QrOutbox(k)));
            }
            buttons.push(("Remove".into(), Style::Ghost, Action::OutboxRemove(k)));
            // The state goes on the line, where the name leaves no room.
            let line = format!("{} · {line}", state.0);
            y += file_card(
                ui,
                x,
                y,
                w,
                (icon, ink, edge),
                &item.name,
                &line,
                None,
                None,
                &buttons,
            ) + 10.0;
        }
        y += 4.0;
    }
    if app.outbox.is_empty() {
        ui.text(x, y, 13.0, W::R, DIM, "Empty");
        y += 26.0;
    }
    if let Some(r) = app.receipt.as_ref() {
        y += receipt_rows(ui, r, x, y, w) + 8.0;
    }
    if !app.outbox.is_empty() {
        y += 6.0;
        ui.icon(x, y, 18.0, Icon::Power, 10.0, MUTED);
        y += ui.wrap(
            x + 24.0,
            y + 1.0,
            w - 24.0,
            12.0,
            W::R,
            MUTED,
            &format!(
                "Power off asks first while files wait {}",
                app.medium.for_the()
            ),
        ) + 10.0;
    }
    y += about(
        ui,
        x,
        y,
        w,
        &format!(
            "{} holds what was copied in from {}; {} holds what waits to be written to one, and what \
             the last visit wrote. Both live in memory and are emptied at power-off. Public files are \
             listed apart from vaults, which carry secrets sealed under a passphrase; a secret goes out \
             unprotected only when you say so after a warning, and is listed apart.",
            app.medium.from_box(),
            app.medium.nouns(),
            app.medium.for_box()
        ),
    );
    finish(app, ui, x0, cw, h, y - top + 16.0);
}

/// [`visit`] on a small panel: one scrolled column. The sticks, what is
/// left to do, then what to write and what to import, each a list with
/// its buttons under it.
fn visit_compact(app: &Faraday, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    use crate::compact::M;
    use crate::compact_screens::{about, finish};
    let (x, w) = (x0 + M, cw - 2.0 * M);
    let top = 12.0 - app.list_offset;
    let mut y = top;
    app.visit.bar(crate::Column::Outbox).set(None);
    app.visit.bar(crate::Column::Stick).set(None);
    let Some(stick) = app.sticks.get(app.visit.stick) else {
        ui.text(
            x,
            y,
            15.0,
            W::R,
            MUTED,
            &format!("No {} attached", app.medium.noun()),
        );
        return;
    };
    // Where pulling the stick goes.
    let then = ui.fit(12.0, W::R, &app.visit_then(), w - 34.0);
    ui.chip(x, y, &then, ACCENT, ACCENT.with_alpha(30));
    y += 36.0;
    // One button per stick, the chosen one filled.
    if app.sticks.len() > 1 {
        let items: Vec<(&str, Style, Action)> = app
            .sticks
            .iter()
            .enumerate()
            .map(|(k, st)| {
                let style = if k == app.visit.stick {
                    Style::Primary
                } else {
                    Style::Secondary
                };
                (st.label.as_str(), style, Action::VisitStick(k))
            })
            .collect();
        y += wrap_buttons(ui, x, y, w, 34.0, &items) + 4.0;
    } else {
        ui.text(x, y, 13.0, W::S, MUTED, &stick.label);
        y += 26.0;
    }
    // What is left to do: pull the stick, and where the copied files went.
    y += crate::vault_screens::stick_banner(ui, x, y, w, &pull_line(app));
    ui.button(
        x,
        y,
        Some(w),
        40.0,
        "Open Files",
        Style::Secondary,
        Action::Nav(Screen::Files),
    );
    y += 50.0;
    for (line, ok) in app.visit.log.iter().rev().take(2) {
        let c = if *ok { OK } else { ERR };
        y += ui.wrap(x, y, w, 13.0, W::R, c, line) + 6.0;
    }
    y += 8.0;

    // Import into the Inbox.
    ui.icon(x, y, 20.0, Icon::Download, 13.0, ACCENT);
    ui.text_mid(
        x + 28.0,
        y - 4.0,
        28.0,
        15.0,
        W::S,
        TEXT,
        &format!("Copy {}", app.medium.from_the()),
    );
    y += 34.0;
    // Everything comes in by default: Unselect all, while any is ticked.
    if !app.visit.inn.is_empty() {
        let t = "Unselect all";
        let tw = ui.measure(13.0, W::S, t);
        ui.text(x, y + 2.0, 13.0, W::S, ACCENT, t);
        ui.hit(x - 6.0, y - 6.0, tw + 12.0, 34.0, Action::VisitInAll);
        y += 34.0;
    }
    for (k, (name, size)) in stick.files.iter().enumerate() {
        // A file larger than the disk process reads is listed, not read.
        let big = *size > crate::READ_MAX;
        let kind = crate::stick_kind(name).filter(|_| !big);
        let on = app.visit.inn.contains(name);
        ui.checkbox(x + 2.0, y + 10.0, on, kind.is_some());
        let fg = if kind.is_some() { TEXT } else { DIM };
        let n = ui.fit(14.0, W::M, name, w - 32.0);
        ui.text(x + 32.0, y + 4.0, 14.0, W::M, fg, &n);
        let sub = match kind {
            Some(k) => format!(
                "{k} · {}",
                kind_line(FileKind::Other, *size as usize).trim_start_matches("File · ")
            ),
            None if big => "Larger than 18 MB: not read".to_string(),
            // Read at boot, not copied in.
            None => "Settings".to_string(),
        };
        let sub = ui.fit(12.0, W::R, &sub, w - 32.0);
        ui.text(
            x + 32.0,
            y + 26.0,
            12.0,
            W::R,
            if kind.is_some() { MUTED } else { DIM },
            &sub,
        );
        if kind.is_some() {
            ui.hit(x - 4.0, y, w + 8.0, 48.0, Action::VisitIn(k));
        }
        y += 50.0;
        ui.rule(x, y, w, INNER);
        y += 6.0;
    }
    if stick.files.is_empty() {
        let none = format!("No files on this {}", app.medium.noun());
        ui.text(x, y, 13.0, W::R, DIM, &none);
        y += 28.0;
    }
    let nin = app.visit.inn.len();
    let enabled = nin > 0;
    let label = format!("Import {nin} {}", if nin == 1 { "file" } else { "files" });
    y += 6.0;
    y += wrap_buttons(
        ui,
        x,
        y,
        w,
        42.0,
        &[
            (
                &label,
                if enabled {
                    Style::Secondary
                } else {
                    Style::Disabled
                },
                Action::VisitCopy,
            ),
            (
                "Import and load",
                if enabled {
                    Style::Primary
                } else {
                    Style::Disabled
                },
                Action::VisitCopyAndLoad,
            ),
        ],
    ) + 12.0;

    // Write to the stick: the Outbox, and copies from the Inbox.
    ui.icon(x, y, 20.0, Icon::Export, 13.0, ACCENT);
    ui.text_mid(
        x + 28.0,
        y - 4.0,
        28.0,
        15.0,
        W::S,
        TEXT,
        &format!("Write to the {}", app.medium.noun()),
    );
    y += 34.0;
    // The settings, which are not an Outbox file.
    let settings_on = app.visit_settings_on();
    let all = visit_all_out(app);
    ui.checkbox(x + 2.0, y + 4.0, all, true);
    ui.text(x + 32.0, y + 2.0, 13.0, W::S, MUTED, "Select all");
    ui.hit(x - 4.0, y - 6.0, w + 8.0, 34.0, Action::VisitOutAll);
    y += 34.0;
    let row = |ui: &mut Ui,
               y: f32,
               on: bool,
               name: &str,
               line: &str,
               tag: (&str, osk_ui::Color),
               action: Action| {
        ui.checkbox(x + 2.0, y + 10.0, on, true);
        let tw = ui.measure(12.0, W::S, tag.0) + 8.0;
        let n = ui.fit(14.0, W::M, name, w - 32.0 - tw);
        ui.text(x + 32.0, y + 4.0, 14.0, W::M, TEXT, &n);
        ui.text_right(x + w, y + 2.0, 20.0, 12.0, W::S, tag.1, tag.0);
        let l = ui.fit(12.0, W::R, line, w - 32.0);
        ui.text(x + 32.0, y + 26.0, 12.0, W::R, MUTED, &l);
        ui.hit(x - 4.0, y, w + 8.0, 48.0, action);
        ui.rule(x, y + 50.0, w, INNER);
        56.0
    };
    y += row(
        ui,
        y,
        settings_on,
        crate::stick_settings::FILE,
        "Settings",
        ("Public", MUTED),
        Action::VisitSettings,
    );
    for (k, item) in app.outbox.iter().enumerate() {
        let on = app.visit.out.contains(&item.name);
        let tag = match item.exposure() {
            crate::secrets::Exposure::Public => ("Public", MUTED),
            crate::secrets::Exposure::Sealed => ("Sealed", OK),
            crate::secrets::Exposure::Secret => ("Unprotected secret", ERR),
        };
        y += row(
            ui,
            y,
            on,
            &item.name,
            &kind_line(item.kind, item.bytes.len()),
            tag,
            Action::VisitOut(k),
        );
    }
    if app.outbox.is_empty() {
        ui.text(
            x,
            y,
            13.0,
            W::R,
            DIM,
            &format!("Nothing waits {}", app.medium.for_the()),
        );
        y += 28.0;
    }
    let inbox_rows = app.visit_inbox_rows();
    if !inbox_rows.is_empty() {
        ui.text_mid(x, y, 30.0, 13.0, W::S, MUTED, app.medium.from_box());
        y += 32.0;
        for &k in &inbox_rows {
            let item = &app.inbox[k];
            y += row(
                ui,
                y,
                app.visit.from_inbox.contains(&item.name),
                &item.name,
                &visit_inbox_line(item),
                visit_inbox_tag(item),
                Action::VisitInbox(k),
            );
        }
    }
    let nout = visit_write_count(app);
    let label = format!("Write {nout} {}", if nout == 1 { "file" } else { "files" });
    y += 6.0;
    ui.button(
        x,
        y,
        Some(w),
        42.0,
        &label,
        if nout > 0 {
            Style::Primary
        } else {
            Style::Disabled
        },
        Action::VisitWrite,
    );
    y += 54.0;
    y += about(
        ui,
        x,
        y,
        w,
        &format!(
            "Write what waits for the {noun} and copy in what you need from it. Each file written is read \
             back and compared before it counts. Remove the {noun} when you are done: keys load only with \
             no {noun} attached.",
            noun = app.medium.noun()
        ),
    );
    finish(app, ui, x0, cw, h, y - top + 16.0);
}

fn visit(app: &Faraday, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    if ui.compact {
        return visit_compact(app, ui, x0, cw, h);
    }
    let x = x0 + 48.0;
    let width = cw - 96.0;
    let mut y = 36.0;
    title(ui, x, y, app.medium.visit());
    // Where pulling the stick goes.
    let tw = ui.measure(28.0, W::S, app.medium.visit());
    ui.chip(
        x + tw + 14.0,
        y + 4.0,
        &app.visit_then(),
        ACCENT,
        ACCENT.with_alpha(30),
    );
    // One chip per stick; the chosen one is outlined.
    let mut chx = x + width;
    for (k, st) in app.sticks.iter().enumerate().rev() {
        let label = st.label.clone();
        let wdt = ui.measure(13.0, W::S, &label) + 28.0;
        chx -= wdt;
        let current = k == app.visit.stick;
        ui.fill(
            chx,
            y,
            wdt,
            32.0,
            16.0,
            if current {
                ACCENT.with_alpha(30)
            } else {
                SURFACE
            },
        );
        ui.stroke(
            chx,
            y,
            wdt,
            32.0,
            16.0,
            if current {
                ACCENT.with_alpha(110)
            } else {
                LINE
            },
        );
        ui.text_mid(chx + 14.0, y, 32.0, 13.0, W::S, TEXT, &label);
        ui.hit(chx, y, wdt, 32.0, Action::VisitStick(k));
        chx -= 8.0;
    }
    y += 64.0;
    y += guide_text(
        app,
        ui,
        x,
        y,
        width.min(900.0),
        &format!(
            "Write what waits for the {noun} and copy in what you need from it. Each file written is read \
             back and compared before it counts. Remove the {noun} when you are done: keys load only with \
             no {noun} attached.",
            noun = app.medium.noun()
        ),
    );
    let Some(stick) = app.sticks.get(app.visit.stick) else {
        ui.text(
            x,
            y,
            15.0,
            W::R,
            MUTED,
            &format!("No {} attached", app.medium.noun()),
        );
        return;
    };
    let colw = (width - 20.0) / 2.0;
    let colh = h - y - 32.0 - 108.0;
    const ROW: f32 = 56.0;
    // Rows between the heading and the buttons at the card's foot.
    let list_top = y + 62.0;
    let max_rows = ((colh - 140.0) / ROW).max(1.0) as usize;
    let track_h = max_rows as f32 * ROW;
    // Write to the stick: the Outbox, and copies from the Inbox.
    ui.card(x, y, colw, colh, LINE);
    ui.icon(x + 20.0, y + 20.0, 22.0, Icon::Export, 14.0, ACCENT);
    ui.text_mid(
        x + 50.0,
        y + 20.0,
        22.0,
        16.0,
        W::S,
        TEXT,
        &format!("Write to the {}", app.medium.noun()),
    );
    let settings_on = app.visit_settings_on();
    let inbox_rows = app.visit_inbox_rows();
    visit_select_all(ui, x + colw, y, visit_all_out(app), Action::VisitOutAll);
    // The settings, which are not an Outbox file, then the Outbox, then
    // the Inbox under its heading.
    const HEAD: f32 = 36.0;
    let all_h = (1 + app.outbox.len()) as f32 * ROW
        + if app.outbox.is_empty() { HEAD } else { 0.0 }
        + if inbox_rows.is_empty() {
            0.0
        } else {
            HEAD + inbox_rows.len() as f32 * ROW
        };
    let max_shift = (all_h - track_h).max(0.0);
    let shift = app.visit.out_offset.min(max_shift);
    let clip = ui.rect(x, list_top, colw, track_h);
    ui.c.push_clip(clip);
    let mut ry = list_top - shift;
    ui.checkbox(x + 22.0, ry + 15.0, settings_on, true);
    ui.text(
        x + 54.0,
        ry + 6.0,
        14.0,
        W::M,
        TEXT,
        crate::stick_settings::FILE,
    );
    ui.text(x + 54.0, ry + 28.0, 12.0, W::R, MUTED, "Settings");
    ui.text_right(x + colw - 20.0, ry + 6.0, 20.0, 12.0, W::S, MUTED, "Public");
    ui.hit(x + 12.0, ry, colw - 24.0, 50.0, Action::VisitSettings);
    ui.rule(x + 20.0, ry + 52.0, colw - 40.0, INNER);
    ry += ROW;
    let file_row = |ui: &mut Ui,
                    ry: f32,
                    on: bool,
                    name: &str,
                    line: &str,
                    tag: (&str, osk_ui::Color),
                    action: Action| {
        ui.checkbox(x + 22.0, ry + 15.0, on, true);
        let tw = ui.measure(12.0, W::S, tag.0) + 12.0;
        let name = ui.fit(14.0, W::M, name, colw - 74.0 - tw);
        ui.text(x + 54.0, ry + 6.0, 14.0, W::M, TEXT, &name);
        let line = ui.fit(12.0, W::R, line, colw - 90.0);
        ui.text(x + 54.0, ry + 28.0, 12.0, W::R, MUTED, &line);
        ui.text_right(x + colw - 20.0, ry + 6.0, 20.0, 12.0, W::S, tag.1, tag.0);
        ui.hit(x + 12.0, ry, colw - 24.0, 50.0, action);
        ui.rule(x + 20.0, ry + 52.0, colw - 40.0, INNER);
    };
    for (k, item) in app.outbox.iter().enumerate() {
        let on = app.visit.out.contains(&item.name);
        let tag = match item.exposure() {
            crate::secrets::Exposure::Public => ("Public", MUTED),
            crate::secrets::Exposure::Sealed => ("Sealed", OK),
            crate::secrets::Exposure::Secret => ("Unprotected secret", ERR),
        };
        file_row(
            ui,
            ry,
            on,
            &item.name,
            &kind_line(item.kind, item.bytes.len()),
            tag,
            Action::VisitOut(k),
        );
        ry += ROW;
    }
    if app.outbox.is_empty() {
        ui.text_mid(
            x + 22.0,
            ry,
            HEAD,
            13.0,
            W::R,
            DIM,
            &format!("Nothing waits {}", app.medium.for_the()),
        );
        ry += HEAD;
    }
    if !inbox_rows.is_empty() {
        ui.text_mid(
            x + 22.0,
            ry + 4.0,
            HEAD - 4.0,
            13.0,
            W::S,
            MUTED,
            app.medium.from_box(),
        );
        ry += HEAD;
        for &k in &inbox_rows {
            let item = &app.inbox[k];
            let on = app.visit.from_inbox.contains(&item.name);
            file_row(
                ui,
                ry,
                on,
                &item.name,
                &visit_inbox_line(item),
                visit_inbox_tag(item),
                Action::VisitInbox(k),
            );
            ry += ROW;
        }
    }
    ui.c.pop_clip();
    ui.report_scroll_in(
        crate::ui::Slot::VisitOut,
        clip,
        max_shift,
        app.visit.out_offset,
    );
    visit_list_bar(
        app,
        ui,
        crate::Column::Outbox,
        (x, colw),
        list_top,
        track_h,
        all_h,
        shift,
    );
    let nout = visit_write_count(app);
    let label = format!("Write {nout} {}", if nout == 1 { "file" } else { "files" });
    let bw = ui.measure(14.0, W::S, &label) + 36.0;
    ui.button(
        x + colw - 20.0 - bw,
        y + colh - 62.0,
        Some(bw),
        42.0,
        &label,
        if nout > 0 {
            Style::Primary
        } else {
            Style::Disabled
        },
        Action::VisitWrite,
    );

    // Import into the Inbox.
    let ix = x + colw + 20.0;
    ui.card(ix, y, colw, colh, LINE);
    ui.icon(ix + 20.0, y + 20.0, 22.0, Icon::Download, 14.0, ACCENT);
    ui.text_mid(
        ix + 50.0,
        y + 20.0,
        22.0,
        16.0,
        W::S,
        TEXT,
        &format!("Copy {}", app.medium.from_the()),
    );
    let total = stick.files.len();
    let max_shift = (total as f32 * ROW - track_h).max(0.0);
    let shift = app.list_offset.min(max_shift);
    // Everything comes in by default: Unselect all, while any is ticked.
    if !app.visit.inn.is_empty() {
        visit_unselect_all(ui, ix + colw, y);
    }
    let clip = ui.rect(ix, list_top, colw, track_h);
    ui.c.push_clip(clip);
    for (k, (name, size)) in stick.files.iter().enumerate() {
        let ry = list_top - shift + k as f32 * ROW;
        // A file larger than the disk process reads is listed, not read.
        let big = *size > crate::READ_MAX;
        let kind = crate::stick_kind(name).filter(|_| !big);
        let on = app.visit.inn.contains(name);
        ui.checkbox(ix + 22.0, ry + 15.0, on, kind.is_some());
        let fg = if kind.is_some() { TEXT } else { DIM };
        let room = 90.0;
        let n = ui.fit(14.0, W::M, name, colw - room);
        ui.text(ix + 54.0, ry + 6.0, 14.0, W::M, fg, &n);
        let sub = match kind {
            Some(k) => format!(
                "{k} · {}",
                kind_line(FileKind::Other, *size as usize).trim_start_matches("File · ")
            ),
            None if big => "Larger than 18 MB: not read".to_string(),
            // Read at boot, not copied in.
            None => "Settings".to_string(),
        };
        let sub = ui.fit(12.0, W::R, &sub, colw - room);
        ui.text(
            ix + 54.0,
            ry + 28.0,
            12.0,
            W::R,
            if kind.is_some() { MUTED } else { DIM },
            &sub,
        );
        if kind.is_some() {
            ui.hit(ix + 12.0, ry, colw - 24.0, 50.0, Action::VisitIn(k));
        }
        ui.rule(ix + 20.0, ry + 52.0, colw - 40.0, INNER);
    }
    ui.c.pop_clip();
    ui.report_scroll_own_bar(clip, max_shift);
    visit_list_bar(
        app,
        ui,
        crate::Column::Stick,
        (ix, colw),
        list_top,
        track_h,
        total as f32 * ROW,
        shift,
    );
    let ry = list_top + track_h;
    if stick.files.is_empty() {
        let none = format!("No files on this {}", app.medium.noun());
        ui.text(ix + 22.0, ry, 13.0, W::R, DIM, &none);
    }
    let nin = app.visit.inn.len();
    let enabled = nin > 0;
    let load_label = "Import and load";
    let load_bw = ui.measure(14.0, W::S, load_label) + 36.0;
    ui.button(
        ix + colw - 20.0 - load_bw,
        y + colh - 62.0,
        Some(load_bw),
        42.0,
        load_label,
        if enabled {
            Style::Primary
        } else {
            Style::Disabled
        },
        Action::VisitCopyAndLoad,
    );
    let label = format!("Import {nin} {}", if nin == 1 { "file" } else { "files" });
    let bw = ui.measure(14.0, W::S, &label) + 36.0;
    ui.button(
        ix + colw - 28.0 - load_bw - bw,
        y + colh - 62.0,
        Some(bw),
        42.0,
        &label,
        if enabled {
            Style::Secondary
        } else {
            Style::Disabled
        },
        Action::VisitCopy,
    );

    // What is left to do: pull the stick, and where the copied files went.
    let ay = h - 32.0 - 44.0 - 44.0;
    ui.fill(x, ay, width, 44.0, 10.0, WARN.with_alpha(22));
    ui.stroke(x, ay, width, 44.0, 10.0, WARN.with_alpha(90));
    ui.icon(x + 16.0, ay + 12.0, 20.0, app.medium.icon(), 13.0, WARN);
    ui.text_mid(x + 44.0, ay, 44.0, 14.0, W::S, WARN, &pull_line(app));
    let open_label = "Open Files";
    let obw = ui.measure(14.0, W::S, open_label) + 32.0;
    ui.button(
        x + width - 12.0 - obw,
        ay + 2.0,
        Some(obw),
        40.0,
        open_label,
        Style::Secondary,
        Action::Nav(Screen::Files),
    );

    // The log.
    let fy = h - 32.0 - 44.0;
    let mut lx = x;
    for (line, ok) in app.visit.log.iter().rev().take(2) {
        let c = if *ok { OK } else { ERR };
        let t = ui.fit(13.0, W::R, line, width / 2.0 - 20.0);
        let used = ui.text_mid(lx, fy, 44.0, 13.0, W::R, c, &t);
        lx += used + 24.0;
    }
}

/// Whether everything Select all chooses on a visit's write list is
/// chosen: the settings, every Outbox file but an unprotected secret, and
/// every public or sealed Inbox file listed.
fn visit_all_out(app: &Faraday) -> bool {
    app.visit_settings_on()
        && app
            .outbox
            .iter()
            .filter(|i| i.exposure() != crate::secrets::Exposure::Secret)
            .all(|i| app.visit.out.contains(&i.name))
        && app
            .visit_inbox_rows()
            .into_iter()
            .filter_map(|k| app.inbox.get(k))
            .filter(|i| i.copy_ack().is_none())
            .all(|i| app.visit.from_inbox.contains(&i.name))
}

/// How many files a visit's Write writes: the settings, the Outbox files
/// and the Inbox files chosen.
fn visit_write_count(app: &Faraday) -> usize {
    let out = app
        .outbox
        .iter()
        .filter(|i| app.visit.out.contains(&i.name))
        .count();
    let inbox = app
        .visit_inbox_rows()
        .into_iter()
        .filter_map(|k| app.inbox.get(k))
        .filter(|i| app.visit.from_inbox.contains(&i.name))
        .count();
    out + inbox + usize::from(app.visit_settings_on())
}

/// An Inbox file's second line on a visit's write list: a picture says
/// what its codes hold.
fn visit_inbox_line(item: &crate::Item) -> String {
    match item.picture {
        Some(FileKind::Other) => format!(
            "PNG, no QR code read · {}",
            kind_line(FileKind::Other, item.bytes.len()).trim_start_matches("File · ")
        ),
        Some(k) => format!("PNG · {}", kind_line(k, item.bytes.len())),
        None => kind_line(item.kind, item.bytes.len()),
    }
}

/// An Inbox file's tag on a visit's write list: public and sealed files
/// are written as they are; the rest only past the secret sheet.
fn visit_inbox_tag(item: &crate::Item) -> (&'static str, osk_ui::Color) {
    match item.copy_ack() {
        None if item.exposure() == crate::secrets::Exposure::Sealed => ("Sealed", OK),
        None => ("Public", MUTED),
        Some(crate::secrets::Ack::Unknown) => ("May be a secret", WARN),
        Some(_) => ("Secret", ERR),
    }
}

/// A stick visit column's "Select all", at the right of its heading: the
/// card's right edge at `right`, its top at `y`.
fn visit_select_all(ui: &mut Ui, right: f32, y: f32, all: bool, action: Action) {
    let label = "Select all";
    let lw = ui.measure(13.0, W::S, label);
    let sx = right - 24.0 - lw - 28.0;
    ui.checkbox(sx, y + 22.0, all, true);
    ui.text_mid(sx + 28.0, y + 16.0, 30.0, 13.0, W::S, MUTED, label);
    ui.hit(sx - 8.0, y + 12.0, lw + 44.0, 38.0, action);
}

/// The stick visit's "Unselect all", at the right of the From the stick
/// heading: the card's right edge at `right`, its top at `y`.
fn visit_unselect_all(ui: &mut Ui, right: f32, y: f32) {
    let label = "Unselect all";
    let lw = ui.measure(13.0, W::S, label);
    let sx = right - 24.0 - lw;
    ui.text_mid(sx, y + 16.0, 30.0, 13.0, W::S, ACCENT, label);
    ui.hit(sx - 8.0, y + 12.0, lw + 16.0, 38.0, Action::VisitInAll);
}

/// A stick visit column's list's scrollbar, which a finger or the mouse
/// drags, and under the list how many rows are below it: the card's left
/// edge and width in `card`, the list's rows from `top`, `track_h` of
/// them shown of `all_h`, scrolled by `shift`.
#[allow(clippy::too_many_arguments)]
fn visit_list_bar(
    app: &Faraday,
    ui: &mut Ui,
    column: crate::Column,
    card: (f32, f32),
    top: f32,
    track_h: f32,
    all_h: f32,
    shift: f32,
) {
    const ROW: f32 = 56.0;
    let right = card.0 + card.1;
    let max_shift = (all_h - track_h).max(0.0);
    let bar = app.visit.bar(column);
    if max_shift > 0.0 {
        let thumb = (track_h * track_h / all_h).max(32.0);
        let tx = right - 14.0;
        let ty = top + (track_h - thumb) * shift / max_shift;
        ui.fill(tx, top, 6.0, track_h, 3.0, INNER);
        let held = ui.is_pressed(Action::VisitBar(column));
        ui.fill(tx, ty, 6.0, thumb, 3.0, if held { ACCENT } else { BORDER });
        ui.hit(tx - 8.0, top, 22.0, track_h, Action::VisitBar(column));
        let r = ui.rect(tx, top, 6.0, track_h);
        bar.set(Some((r.y, r.h, ui.px(thumb), max_shift)));
    } else {
        bar.set(None);
    }
    let below = all_h - shift - track_h;
    if below > 0.0 {
        ui.text(
            card.0 + 22.0,
            top + track_h,
            12.0,
            W::R,
            DIM,
            &format!(
                "{} more · scroll to see them",
                (below / ROW).ceil() as usize
            ),
        );
    }
}

/// What pulling the stick does next: the keys an Import and load copied
/// in load, or the vault it brought asks for its passphrase.
fn pull_line(app: &Faraday) -> String {
    // Only words copied in for Import and load are keys waiting to load.
    let n = app
        .inbox
        .iter()
        .filter(|i| i.kind == FileKind::Words && app.visit.load_after.contains(&i.name))
        .count();
    let vault = app
        .vault_files()
        .into_iter()
        .find(|f| f.open.is_none() && !f.in_outbox)
        .map(|f| f.name);
    let noun = app.medium.noun();
    match (n, vault) {
        (0, Some(v)) if !app.holds_secret() => format!("Remove the {noun} to unlock {v}"),
        (0, _) => format!("Remove the {noun} when you are done"),
        (1, _) => format!("1 key copied in · remove the {noun} to load it"),
        (n, _) => format!("{n} keys copied in · remove the {noun} to load them"),
    }
}

// ---------------------------------------------------------------------
// Add a key
// ---------------------------------------------------------------------

/// The one word the typed prefix can complete to, if there is one.
fn entry(app: &Faraday, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    if ui.compact {
        crate::compact_screens::entry(app, ui, x0, cw, h);
        return;
    }
    let x = x0 + 56.0;
    let width = (cw - 112.0).min(900.0);
    let mut y = 36.0;
    let back = Action::Nav(app.entry_leave());
    ui.icon(x - 4.0, y, 16.0, Icon::ChevronLeft, 10.0, MUTED);
    ui.text(
        x + 14.0,
        y,
        12.0,
        W::R,
        MUTED,
        if app.entry_leave() == Screen::Family {
            "Spend"
        } else {
            "Wallets"
        },
    );
    ui.hit(x - 4.0, y - 4.0, 70.0, 24.0, back);
    y += 22.0;
    let heading = match app.entry.wanted {
        Some(fp) => format!("Add key {}", fp_text(osk_bip::keys::Fingerprint(fp))),
        None if app.entry.form != crate::forms::Form::Words => {
            format!("Add a key · {}", app.entry.form.name())
        }
        None => "Add a key".to_string(),
    };
    ui.text(x, y, 26.0, W::S, TEXT, &heading);
    y += 48.0;
    // The words form is the one everyday way in; the other three are
    // behind Other forms, or already shown when a Tools tile opened one
    // of them (`docs/SIMPLIFY.md` §2.4).
    if app.entry.other_forms {
        let forms: Vec<(String, Style, Action)> = crate::forms::Form::ALL
            .iter()
            .enumerate()
            .map(|(i, f)| {
                (
                    f.name().to_string(),
                    if *f == app.entry.form {
                        Style::Primary
                    } else {
                        Style::Secondary
                    },
                    Action::EntryForm(i as u8),
                )
            })
            .collect();
        y += button_rows(ui, x, y, width, &forms) + 4.0;
    }
    if app.entry.form != crate::forms::Form::Words {
        entry_form(app, ui, x, y, width, h);
        return;
    }
    // Which list the words are from: English, the others one press away;
    // and, unless the chips are already shown, the way to them. Under the
    // word fields, or above the keyboard of a list typed on its own keys.
    let lang = app.entry.language();
    let english = lang == osk_bip::bip39::Language::English;
    let mut langs: Vec<(String, Style, Action)> = if english && !app.entry.languages {
        vec![(
            "Other languages".to_string(),
            Style::Ghost,
            Action::EntryLanguages,
        )]
    } else {
        crate::forms::LANGUAGES
            .iter()
            .enumerate()
            .map(|(i, l)| {
                (
                    crate::forms::language_name(*l).to_string(),
                    if *l == lang {
                        Style::Primary
                    } else {
                        Style::Ghost
                    },
                    Action::EntryLanguage(i as u8),
                )
            })
            .collect()
    };
    if !app.entry.other_forms {
        langs.insert(
            0,
            (
                "Other forms".to_string(),
                Style::Ghost,
                Action::EntryOtherForms,
            ),
        );
    }
    if let Some(w) = app.entry.keys.as_deref() {
        y += button_rows(ui, x, y, width, &langs);
        y = keyed_words(app, w, ui, x, y, width, h);
        entry_finish(app, ui, x, y, width, h);
        return;
    }
    y += guide_text(
        app,
        ui,
        x,
        y,
        width,
        "Type the words in order. Each word is checked against the BIP-39 list as you type, and Tab \
         finishes a word once only one fits. The last word carries a checksum, so a mistyped word is \
         caught when the count is complete. The words stay in memory for this session only.",
    );

    let words: Vec<&str> = app.entry.typed.split_whitespace().collect();
    let typing_word = !app.entry.typed.is_empty() && !app.entry.typed.ends_with(' ');
    // A grid of 24 numbered word cells.
    let cols = 4;
    let cellw = (width - 3.0 * 10.0) / cols as f32;
    let shown = if words.len() > 12 || app.entry.typed.len() > 80 {
        24
    } else {
        12
    };
    for k in 0..shown {
        let cx = x + (k % cols) as f32 * (cellw + 10.0);
        let cy = y + (k / cols) as f32 * 46.0;
        let current = k == words.len() - usize::from(typing_word && !words.is_empty()) && k < shown;
        let edge = if current {
            ACCENT.with_alpha(110)
        } else {
            INNER
        };
        ui.fill(cx, cy, cellw, 38.0, 8.0, BG);
        ui.stroke(cx, cy, cellw, 38.0, 8.0, edge);
        ui.text_mid(cx + 10.0, cy, 38.0, 11.0, W::R, DIM, &(k + 1).to_string());
        if let Some(wd) = words.get(k) {
            let found = crate::forms::typed_index(lang, wd);
            let last_partial = typing_word && k + 1 == words.len();
            let c = if found.is_some() || last_partial {
                TEXT
            } else {
                ERR
            };
            // A finished word in the list's own spelling.
            let shown = match found {
                Some(i) if !last_partial => lang.word(i),
                _ => wd,
            };
            let tw = ui.text_mid(cx + 36.0, cy, 38.0, 14.0, W::M, c, shown);
            if last_partial && !app.entry.on_passphrase {
                ui.caret(cx + 37.0 + tw, cy + 10.0, 18.0);
            }
        } else if current && !app.entry.on_passphrase {
            ui.caret(cx + 37.0, cy + 10.0, 18.0);
        }
    }
    y += (shown / cols) as f32 * 46.0 + 12.0;

    // Candidates for the word being typed.
    if typing_word && let Some(prefix) = words.last() {
        let cands: Vec<&str> = crate::forms::typed_candidates(lang, prefix)
            .into_iter()
            .take(8)
            .map(|i| lang.word(i))
            .collect();
        let line = if cands.is_empty() {
            "No word starts with that".to_string()
        } else {
            format!("{}   · Tab completes a single match", cands.join("  "))
        };
        ui.text(
            x,
            y,
            13.0,
            W::M,
            if cands.is_empty() { ERR } else { MUTED },
            &line,
        );
    }
    y += 30.0;
    y += button_rows(ui, x, y, width, &langs);

    // Status.
    let n = words.len();
    let complete = [12, 15, 18, 21, 24].contains(&n)
        && words
            .iter()
            .all(|wd| crate::forms::typed_index(lang, wd).is_some());
    let m = complete.then(|| crate::forms::typed_mnemonic(lang, &app.entry.typed));
    y = entry_status(app, ui, x, y, n, m);
    entry_finish(app, ui, x, y, width, h);
}

/// Words typed on a list's own on-screen keyboard, through OpenSigner's
/// word entry: the words taken so far in a grid, the keys typed for the
/// next one, the words they can be as pills to take one, the keyboard
/// with every key that leads nowhere disabled, and the status. Returns
/// where the next row starts.
fn keyed_words(
    app: &Faraday,
    w: &opensigner_core::load::LoadWizard,
    ui: &mut Ui,
    x: f32,
    y: f32,
    width: f32,
    h: f32,
) -> f32 {
    use osk_ui::widgets::keyboard;
    let lang = app.entry.language();
    let kind = w.keyboard();
    let mut y = y;
    y += guide_text(
        app,
        ui,
        x,
        y,
        width,
        match kind {
            keyboard::KeyboardKind::Pinyin => {
                "Type each word's pinyin and then its tone, 1 to 5, on the keys below or the \
                 computer's keyboard, and press the character among those it offers. Keys that \
                 lead to no word are off. The last word carries a checksum, checked when the \
                 count is complete."
            }
            keyboard::KeyboardKind::Zhuyin => {
                "Type each word's 注音 and then its tone on the keys below, and press the \
                 character among those it offers. Keys that lead to no word are off. The last \
                 word carries a checksum, checked when the count is complete."
            }
            _ => {
                "Type each word on the keys below and press it among the words it can be. Keys \
                 that lead to no word are off. The last word carries a checksum, checked when the \
                 count is complete."
            }
        },
    );
    // The words taken, in a grid, the next one's keys in its cell.
    let taken: Vec<u16> = w.committed_indices().collect();
    let shown = if taken.len() >= 12 { 24 } else { 12 };
    let cols = 6;
    let cellw = (width - (cols - 1) as f32 * 8.0) / cols as f32;
    let prefix: String = w.prefix().iter().collect();
    for k in 0..shown {
        let cx = x + (k % cols) as f32 * (cellw + 8.0);
        let cy = y + (k / cols) as f32 * 42.0;
        let current = k == taken.len();
        ui.fill(cx, cy, cellw, 36.0, 8.0, BG);
        ui.stroke(
            cx,
            cy,
            cellw,
            36.0,
            8.0,
            if current {
                ACCENT.with_alpha(110)
            } else {
                INNER
            },
        );
        ui.text_mid(cx + 8.0, cy, 36.0, 11.0, W::R, DIM, &(k + 1).to_string());
        let word = match taken.get(k) {
            Some(&i) => Some((lang.word_display(i).to_string(), TEXT)),
            None if current && !prefix.is_empty() => Some((prefix.clone(), ACCENT)),
            None => None,
        };
        let tw = match word {
            Some((t, c)) => {
                let t = ui.fit(14.0, W::M, &t, cellw - 36.0);
                ui.text_mid(cx + 30.0, cy, 36.0, 14.0, W::M, c, &t)
            }
            None => 0.0,
        };
        if current && !app.entry.on_passphrase {
            ui.caret(cx + 31.0 + tw, cy + 9.0, 18.0);
        }
    }
    y += (shown / cols) as f32 * 42.0 + 8.0;
    // The count and, when the words are a whole key, its checksum and
    // fingerprint, or the last refusal: on the line above the
    // candidates, to keep the keyboard on screen.
    let (status, tone) = match &app.entry.error {
        Some(e) => (e.clone(), ERR),
        None => keyed_status(app, lang, &taken),
    };
    ui.text_right(x + width, y - 4.0, 24.0, 13.0, W::S, tone, &status);
    // What the keys typed can be: a pill each, pressed to take it.
    let cands: Vec<u16> = w.candidates().collect();
    if cands.is_empty() {
        let line = if prefix.is_empty() {
            "Type the next word"
        } else if w.awaiting_tone() {
            "Now its tone"
        } else {
            "No word starts with that"
        };
        ui.text_mid(x, y, 30.0, 13.0, W::R, MUTED, line);
        y += 34.0;
    } else {
        ui.text(x, y, 12.0, W::R, MUTED, "Press a word to take it");
        y += 22.0;
        let mut bx = x;
        for (n, &i) in cands.iter().enumerate() {
            let word = lang.word_display(i);
            let bw = ui.measure(14.0, W::M, word) + 24.0;
            if bx > x && bx + bw > x + width {
                bx = x;
                y += 32.0;
            }
            bx += ui.word_pill(bx, y, word, Action::EntryCandidate(n as u8)) + 6.0;
        }
        y += 36.0;
    }
    // The keyboard, laid out in design units, its keys as tall as the
    // room above the status, the passphrase and the buttons allows.
    let rows = osk_ui::widgets::keyboard::rows(kind) as f32;
    let room = h - 82.0 - 8.0 - 40.0 - 12.0 - y;
    let kh = (room / rows).clamp(22.0, 36.0);
    word_keys(app, w, ui, x, y, width, kh)
}

/// The word keyboard: OpenSigner's, for the list being typed, every key
/// that leads to no word off. Returns where the next row starts.
pub(crate) fn word_keys(
    app: &Faraday,
    w: &opensigner_core::load::LoadWizard,
    ui: &mut Ui,
    x: f32,
    y: f32,
    width: f32,
    kh: f32,
) -> f32 {
    use osk_ui::widgets::keyboard::{self, KeyInput, Modifiers};
    let kind = w.keyboard();
    let rows = keyboard::rows(kind) as f32;
    let area = osk_ui::geom::Rect::new(x as i32, y as i32, width as i32, (rows * kh) as i32);
    let ctx = osk_ui::layout::LayoutCtx::new(
        osk_ui::geom::Scale::new(160),
        osk_ui::geom::SizeClass::Wide,
    );
    let mods = Modifiers {
        shift: app.entry.shift,
        symbols: false,
    };
    for cap in keyboard::keys(kind, area, &ctx, w.enabled_keys(), None, mods) {
        let r = cap.rect;
        let (kx, ky, kw, kh) = (r.x as f32, r.y as f32, r.w as f32 - 4.0, r.h as f32 - 4.0);
        match cap.input {
            KeyInput::Char(c) => {
                let style = if cap.enabled {
                    Style::Secondary
                } else {
                    Style::Disabled
                };
                ui.button(
                    kx,
                    ky,
                    Some(kw),
                    kh,
                    &c.to_string(),
                    style,
                    Action::EntryKey(c),
                );
            }
            KeyInput::Backspace => {
                ui.button(
                    kx,
                    ky,
                    Some(kw),
                    kh,
                    "",
                    Style::Secondary,
                    Action::EntryKeyBack,
                );
                ui.icon(
                    kx + kw / 2.0 - 9.0,
                    ky + kh / 2.0 - 9.0,
                    18.0,
                    Icon::Delete,
                    12.0,
                    TEXT,
                );
            }
            KeyInput::Shift => {
                let style = if app.entry.shift {
                    Style::Primary
                } else {
                    Style::Secondary
                };
                ui.button(kx, ky, Some(kw), kh, "Shift", style, Action::EntryShift);
            }
            _ => {}
        }
    }
    y + rows * kh + 12.0
}

/// The words typed on a list's own keyboard, as a status line: the
/// count, and once it is a whole key, the checksum and fingerprint.
pub(crate) fn keyed_status(
    app: &Faraday,
    lang: osk_bip::bip39::Language,
    taken: &[u16],
) -> (String, osk_ui::Color) {
    let n = taken.len();
    if ![12, 15, 18, 21, 24].contains(&n) {
        return (format!("{n} words"), MUTED);
    }
    let mut probe = crate::wallet::Session::default();
    let added = osk_bip::bip39::Mnemonic::from_indices(lang, taken)
        .map_err(|e| crate::wallet::Refusal::Words(e.to_string()))
        .and_then(|m| probe.add_mnemonic(&m, &app.entry.passphrase, "", None));
    match added {
        Ok(fp) => (
            format!("{n} words · checksum correct · {}", fp_text(fp)),
            OK,
        ),
        Err(e) => (e.text(), ERR),
    }
}

/// The count of words and, when they are a whole key, its checksum and
/// fingerprint; then the last refusal. Returns where the next row starts.
fn entry_status(
    app: &Faraday,
    ui: &mut Ui,
    x: f32,
    y: f32,
    n: usize,
    m: Option<Result<osk_bip::bip39::Mnemonic, String>>,
) -> f32 {
    let mut y = y;
    let status = match m {
        Some(made) => {
            let mut probe = crate::wallet::Session::default();
            let added = made
                .map_err(crate::wallet::Refusal::Words)
                .and_then(|m| probe.add_mnemonic(&m, &app.entry.passphrase, "", None));
            match added {
                Ok(fp) => (
                    format!("{n} words · checksum correct · {}", fp_text(fp)),
                    OK,
                ),
                Err(e) => (e.text(), ERR),
            }
        }
        None => (format!("{n} words"), MUTED),
    };
    ui.text(x, y, 14.0, W::S, status.1, &status.0);
    y += 30.0;
    if let Some(e) = &app.entry.error {
        ui.text(x, y, 13.0, W::R, ERR, e);
    }
    y + 34.0
}

/// The BIP-39 passphrase and the buttons under the words.
fn entry_finish(app: &Faraday, ui: &mut Ui, x: f32, y: f32, width: f32, h: f32) {
    // An optional BIP-39 passphrase: the same words with another
    // passphrase are another key.
    ui.text_mid(x, y, 40.0, 13.0, W::R, MUTED, "BIP-39 passphrase");
    let px = x + 150.0;
    let pw = 380.0f32.min(width - 150.0);
    let on = app.entry.on_passphrase;
    ui.fill(px, y, pw, 40.0, 8.0, BG);
    ui.stroke(px, y, pw, 40.0, 8.0, if on { ACCENT } else { BORDER });
    let shown = if app.entry.passphrase.is_empty() && !on {
        "None".to_string()
    } else {
        let mut d = "•".repeat(app.entry.passphrase.len().min(40));
        ui.selection(px + 12.0, y, 40.0, 14.0, W::M, &d, on);
        if on && !ui.select_all {
            d.push_str(ui.caret_char());
        }
        d
    };
    ui.text_mid(
        px + 12.0,
        y,
        40.0,
        14.0,
        W::M,
        if app.entry.passphrase.is_empty() && !on {
            DIM
        } else {
            TEXT
        },
        &shown,
    );
    ui.hit(px, y, pw, 40.0, Action::EntryPassphrase);
    let by = h - 36.0 - 46.0;
    entry_stick(app, ui, x, by - 48.0);
    ui.button(
        x,
        by,
        Some(140.0),
        46.0,
        "Add key",
        Style::Primary,
        Action::EntryAdd,
    );
    let clear_w = ui.button(
        x + 152.0,
        by,
        None,
        46.0,
        "Clear",
        Style::Ghost,
        Action::EntryClear,
    );
    let scan_x = x + 152.0 + clear_w + 8.0;
    let scan_w = ui.button(
        scan_x,
        by,
        None,
        46.0,
        "Scan a SeedQR",
        Style::Secondary,
        Action::ScanSeed,
    );
    let new_x = scan_x + scan_w + 8.0;
    let new_w = ui.button(
        new_x,
        by,
        None,
        46.0,
        "Make a new key",
        Style::Secondary,
        Action::KeyGen(None),
    );
    ui.text_mid(
        new_x + new_w + 20.0,
        by,
        46.0,
        12.0,
        W::R,
        DIM,
        "Type on the keyboard · Enter adds · Esc goes back",
    );
}

/// Add a key's way in from a stick: a key file on it is copied in on the
/// stick visit and loads when the stick is pulled.
pub(crate) fn entry_stick(app: &Faraday, ui: &mut Ui, x: f32, y: f32) {
    if app.sticks.is_empty() {
        ui.icon(x - 2.0, y + 8.0, 20.0, app.medium.icon(), 12.0, DIM);
        ui.text_mid(
            x + 24.0,
            y,
            36.0,
            13.0,
            W::R,
            DIM,
            &format!("From {} · plug it in", app.medium.a()),
        );
    } else {
        ui.button(
            x,
            y,
            None,
            36.0,
            "Copy files in",
            Style::Secondary,
            Action::Nav(Screen::Visit),
        );
    }
}

/// Add a key in another form than BIP-39 words: one share, string or part
/// typed at a time, the ones collected listed, then put together.
fn entry_form(app: &Faraday, ui: &mut Ui, x: f32, y: f32, width: f32, h: f32) {
    use crate::forms::Form;
    let form = app.entry.form;
    let mut y = y;
    let guide = match form {
        Form::Slip39 => {
            "Type one share at a time, its 20 or 33 words, and press Enter. Shares of every group the \
             backup needs go in, in any order; then the passphrase, if the backup has one, and Make the key. \
             A SLIP-39 key has no BIP-39 words."
        }
        Form::Codex32 => {
            "Type one codex32 string at a time, ms1 and all, and press Enter. The secret string is the key \
             at once; shares need as many as the digit after ms1 says."
        }
        _ => {
            "Type one part at a time, its BIP-39 words, and press Enter. Every part is needed, and they \
             combine into the words of the key."
        }
    };
    y += guide_text(app, ui, x, y, width, guide);
    // What is being typed.
    ui.fill(x, y, width, 44.0, 8.0, BG);
    ui.stroke(
        x,
        y,
        width,
        44.0,
        8.0,
        if app.entry.on_passphrase {
            BORDER
        } else {
            ACCENT
        },
    );
    let on = !app.entry.on_passphrase;
    let empty = app.entry.typed.is_empty();
    let typed = if empty && on {
        // The caret stands before the hint while nothing is typed.
        ui.caret(x + 12.0, y + 13.0, 18.0);
        match form {
            Form::Codex32 => crate::secret_text::SecretText::of("ms1…"),
            _ => crate::secret_text::SecretText::of("Words of one share"),
        }
    } else {
        let mut t = crate::secret_text::SecretText::of(&app.entry.typed);
        if on {
            t.push_str(ui.caret_char());
        }
        t
    };
    let shown = ui.fit_secret(14.0, W::M, &typed, width - 24.0);
    ui.text_mid(
        x + if empty && on { 18.0 } else { 12.0 },
        y,
        44.0,
        14.0,
        W::M,
        if app.entry.typed.is_empty() {
            DIM
        } else {
            TEXT
        },
        &shown,
    );
    y += 54.0;
    if let Some(e) = &app.entry.error {
        y += ui.wrap(x, y, width, 13.0, W::R, ERR, e) + 8.0;
    }
    // What is in.
    let lines = app.entry.parts.lines(form);
    section_label(
        ui,
        x,
        y,
        &match lines.len() {
            0 => "Nothing in yet".to_string(),
            1 => "1 in".to_string(),
            n => format!("{n} in"),
        },
    );
    y += 26.0;
    for l in &lines {
        ui.icon(x, y - 2.0, 18.0, Icon::Done, 10.0, OK);
        ui.text(x + 24.0, y, 13.0, W::R, TEXT, l);
        y += 24.0;
    }
    y += 8.0;
    // A SLIP-39 backup's passphrase; a Seed XOR key's BIP-39 one.
    if form != Form::Codex32 {
        ui.text_mid(x, y, 40.0, 13.0, W::R, MUTED, "Passphrase");
        let px = x + 150.0;
        let pw = 380.0f32.min(width - 150.0);
        let on = app.entry.on_passphrase;
        ui.fill(px, y, pw, 40.0, 8.0, BG);
        ui.stroke(px, y, pw, 40.0, 8.0, if on { ACCENT } else { BORDER });
        let p = if app.entry.passphrase.is_empty() && !on {
            "None".to_string()
        } else {
            let mut d = "•".repeat(app.entry.passphrase.len().min(40));
            ui.selection(px + 12.0, y, 40.0, 14.0, W::M, &d, on);
            if on && !ui.select_all {
                d.push_str(ui.caret_char());
            }
            d
        };
        ui.text_mid(px + 12.0, y, 40.0, 14.0, W::M, TEXT, &p);
        ui.hit(px, y, pw, 40.0, Action::EntryPassphrase);
    }
    let by = h - 36.0 - 46.0;
    entry_stick(app, ui, x, by - 48.0);
    let ready = app.entry.parts.ready(form);
    let aw = ui.button(
        x,
        by,
        None,
        46.0,
        "Add this one",
        if app.entry.typed.trim().is_empty() {
            Style::Disabled
        } else {
            Style::Secondary
        },
        Action::EntryPart,
    );
    let sw = ui.button(
        x + aw + 8.0,
        by,
        None,
        46.0,
        "Scan a QR code",
        Style::Secondary,
        Action::ScanPart,
    );
    let aw = aw + sw + 8.0;
    let mw = ui.button(
        x + aw + 8.0,
        by,
        None,
        46.0,
        "Make the key",
        if ready {
            Style::Primary
        } else {
            Style::Disabled
        },
        Action::EntryRecover,
    );
    ui.text_mid(
        x + aw + mw + 28.0,
        by,
        46.0,
        12.0,
        W::R,
        DIM,
        "Enter adds what is typed · Enter on an empty line makes the key",
    );
}

// ---------------------------------------------------------------------
// Settings
// ---------------------------------------------------------------------

/// A section of a small panel's page: a card with its title, measured
/// first so its surface goes down under what `draw` puts in it at the
/// inner (x, y, w). Returns its height with the gap under it.
fn section(
    ui: &mut Ui,
    x: f32,
    y: f32,
    w: f32,
    title: &str,
    draw: &dyn Fn(&mut Ui, f32, f32, f32) -> f32,
) -> f32 {
    let (ix, iw) = (x + 14.0, w - 28.0);
    let inner = crate::compact_screens::measured(ui, |ui| draw(ui, ix, y + 44.0, iw));
    let h = 44.0 + inner + 12.0;
    ui.card(x, y, w, h, LINE);
    ui.text(ix, y + 16.0, 15.0, W::S, TEXT, title);
    draw(ui, ix, y + 44.0, iw);
    h + 14.0
}

/// The panic key, beside Power off on Settings: the stick shell powers
/// off at once when Super and S are held this long (`keyboard.rs`).
const PANIC_KEY: &str = "Super + S held 2 s: power off at once";

/// [`settings`] on a small panel: the same sections, one column, each
/// row of choices wrapping to the panel.
fn settings_compact(app: &Faraday, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    let (x, w) = (x0 + crate::compact::M, cw - 2.0 * crate::compact::M);
    let top = 12.0 - app.list_offset;
    let mut y = top;
    y += section(ui, x, y, w, "Session", &|ui, x, y0, w| {
        let mut y = y0;
        let n = app.session.keys.len();
        let line = format!(
            "{n} {} loaded · lost on lock or power-off",
            if n == 1 { "key" } else { "keys" }
        );
        y += ui.wrap(x, y, w, 13.0, W::R, MUTED, &line) + 10.0;
        let mut items: Vec<(&str, Style, Action)> = Vec::new();
        if app.holds_secret() {
            items.push(("Lock", Style::Secondary, Action::LockAsk));
            items.push(("Remove every key", Style::Secondary, Action::RemoveKeys));
        }
        items.push(("Power off", Style::Secondary, Action::PowerAsk));
        y += wrap_buttons(ui, x, y, w, 38.0, &items) + 4.0;
        if !app.online {
            y += ui.wrap(x, y, w, 13.0, W::R, MUTED, PANIC_KEY) + 10.0;
        }
        let rows: [IdleRow; 2] = [
            (
                "Lock after",
                app.idle_lock_min,
                &crate::stick_settings::IDLE_LOCK_CHOICES,
                Action::IdleLock,
            ),
            (
                "Power off after",
                app.idle_off_min,
                &crate::stick_settings::IDLE_OFF_CHOICES,
                Action::IdleOff,
            ),
        ];
        for (label, now, choices, action) in rows {
            ui.text(x, y, 13.0, W::R, MUTED, label);
            y += 22.0;
            let labels: Vec<String> = choices
                .iter()
                .map(|&m| {
                    if m == 0 {
                        "Never".to_string()
                    } else {
                        format!("{m} min")
                    }
                })
                .collect();
            let items: Vec<(&str, Style, Action)> = labels
                .iter()
                .zip(choices.iter())
                .map(|(l, &m)| {
                    let style = if now == m {
                        Style::Primary
                    } else {
                        Style::Secondary
                    };
                    (l.as_str(), style, action(m))
                })
                .collect();
            y += wrap_buttons(ui, x, y, w, 36.0, &items) + 2.0;
        }
        if !app.ignored_inputs.is_empty() {
            let names: Vec<&str> = app.ignored_inputs.iter().map(|(_, n)| n.as_str()).collect();
            let line = format!("Ignored until unplugged: {}", names.join(", "));
            y += ui.wrap(x, y, w, 13.0, W::R, WARN, &line) + 6.0;
        }
        y - y0
    });
    y += section(ui, x, y, w, "Network", &|ui, x, y0, w| {
        let mut y = y0;
        y += wrap_buttons(ui, x, y, w, 38.0, &network_row_items(app));
        if let Some(note) = network_row_note(app) {
            y += ui.wrap(x, y, w, 13.0, W::R, MUTED, note) + 4.0;
        }
        y - y0 - 8.0
    });
    y += section(ui, x, y, w, "Appearance", &|ui, x, y0, w| {
        let per_row = 2;
        let tile_w = (w - 8.0) / 2.0;
        let tile_h = ui.theme_tile_h(tile_w);
        for (i, theme) in Theme::ALL.into_iter().enumerate() {
            ui.theme_tile(
                x + (i % per_row) as f32 * (tile_w + 8.0),
                y0 + (i / per_row) as f32 * (tile_h + 10.0),
                tile_w,
                tile_h,
                theme,
                app.theme == theme,
                Action::Theme(theme),
            );
        }
        Theme::ALL.len().div_ceil(per_row) as f32 * (tile_h + 10.0) - 10.0
    });
    y += section(ui, x, y, w, "Motion", &|ui, x, y0, w| {
        let items: Vec<(&str, Style, Action)> = [("Full", false), ("Reduced", true)]
            .into_iter()
            .map(|(label, reduced)| {
                let style = if app.reduce_motion == reduced {
                    Style::Primary
                } else {
                    Style::Secondary
                };
                (label, style, Action::ReduceMotion(reduced))
            })
            .collect();
        wrap_buttons(ui, x, y0, w, 38.0, &items) - 8.0
    });
    y += section(ui, x, y, w, "Display scale", &|ui, x, y0, w| {
        let labels: Vec<(String, u16)> = [75u16, 90, 100, 110, 125, 150, 200]
            .into_iter()
            .map(|pct| {
                (
                    if pct == 100 {
                        "Auto".to_string()
                    } else {
                        format!("{pct} %")
                    },
                    pct,
                )
            })
            .collect();
        let items: Vec<(&str, Style, Action)> = labels
            .iter()
            .map(|(l, pct)| {
                let style = if app.scale_pct == *pct {
                    Style::Primary
                } else {
                    Style::Secondary
                };
                (l.as_str(), style, Action::Scale(*pct))
            })
            .collect();
        wrap_buttons(ui, x, y0, w, 36.0, &items) - 8.0
    });
    y += section(ui, x, y, w, "Signed amounts", &|ui, x, y0, w| {
        let mut y = y0;
        let n = app.signed_amounts.len();
        let line = format!(
            "{n} {} remembered",
            if n == 1 {
                "transaction"
            } else {
                "transactions"
            }
        );
        ui.text(x, y, 13.0, W::R, MUTED, &line);
        y += 24.0;
        let items: Vec<(&str, Style, Action)> = [
            ("Sealed into an open vault", true),
            ("Kept until power-off only", false),
        ]
        .into_iter()
        .map(|(label, on)| {
            let style = if app.seal_amounts == on {
                Style::Primary
            } else {
                Style::Secondary
            };
            (label, style, Action::SealAmounts(on))
        })
        .collect();
        y += wrap_buttons(ui, x, y, w, 38.0, &items) - 8.0;
        y - y0
    });
    if !app.online {
        y += section(ui, x, y, w, app.medium.caps(), &|ui, x, y0, w| {
            wrap_buttons(
                ui,
                x,
                y0,
                w,
                38.0,
                &[(app.medium.upgrade(), Style::Secondary, Action::UpgradeOpen)],
            ) - 8.0
        });
    }
    y += section(ui, x, y, w, "About", &|ui, x, y0, w| {
        let mut y = y0;
        let lines = [
            format!("Faraday {}", crate::version_label()),
            "Keys are typed, scanned as SeedQR or loaded from a vault, and kept for the session only"
                .to_string(),
            format!(
                "Vaults · files move by {noun} · QR by camera and from PNG files on {a}",
                noun = app.medium.noun(),
                a = app.medium.a()
            ),
            if app.online {
                format!("{} are folders in ~/faraday-sticks", app.medium.caps())
            } else {
                format!(
                    "{} are read by an unprivileged disk process; the kernel mounts none",
                    app.medium.caps()
                )
            },
        ];
        for l in &lines {
            y += ui.wrap(x, y, w, 13.0, W::R, MUTED, l) + 8.0;
        }
        let s = &opensigner_core::strings::EN;
        let (result, tone) = match app.selftest() {
            Some(Ok(n)) => (
                s.settings_selftest_passed.replacen("{}", &n.to_string(), 1),
                OK,
            ),
            Some(Err(c)) => (s.settings_selftest_failed.replacen("{}", c, 1), ERR),
            None => ("not run".to_string(), MUTED),
        };
        y += 4.0;
        ui.text(x, y, 13.0, W::R, MUTED, s.settings_selftest);
        y += 20.0;
        y += ui.wrap(x, y, w, 13.0, W::S, tone, &result) + 8.0;
        y += wrap_buttons(
            ui,
            x,
            y,
            w,
            34.0,
            &[(
                s.settings_selftest_run,
                Style::Secondary,
                Action::SelfTestRun,
            )],
        ) + 4.0;
        let note = "Visit nakamotoinstitute.org to learn about Bitcoin’s history, economics, and \
                    technology. Not affiliated.";
        y += ui.wrap(x, y, w, 13.0, W::R, MUTED, note);
        y - y0
    });
    crate::compact_screens::finish(app, ui, x0, cw, h, y - top + 8.0);
}

fn settings(app: &Faraday, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    if ui.compact {
        return settings_compact(app, ui, x0, cw, h);
    }
    let x = x0 + 56.0;
    let width = (cw - 112.0).min(760.0);
    let top = 44.0 - app.list_offset;
    let mut y = top;
    title(ui, x, y, "Settings");
    y += 64.0;
    // Devices kept out until unplugged (`PLAN.md` §4.6) take a line.
    let ignored_h = if app.ignored_inputs.is_empty() {
        0.0
    } else {
        30.0
    };
    ui.card(x, y, width, 224.0 + ignored_h, LINE);
    ui.text(x + 22.0, y + 20.0, 15.0, W::S, TEXT, "Session");
    let n = app.session.keys.len();
    ui.text(
        x + 22.0,
        y + 46.0,
        13.0,
        W::R,
        MUTED,
        &format!(
            "{n} {} loaded · lost on lock or power-off",
            if n == 1 { "key" } else { "keys" }
        ),
    );
    let mut bx = x + 22.0;
    if app.holds_secret() {
        bx += ui.button(
            bx,
            y + 74.0,
            None,
            38.0,
            "Lock",
            Style::Secondary,
            Action::LockAsk,
        ) + 8.0;
        bx += ui.button(
            bx,
            y + 74.0,
            None,
            38.0,
            "Remove every key",
            Style::Secondary,
            Action::RemoveKeys,
        ) + 8.0;
    }
    bx += ui.button(
        bx,
        y + 74.0,
        None,
        38.0,
        "Power off",
        Style::Secondary,
        Action::PowerAsk,
    ) + 14.0;
    if !app.online {
        let room = x + width - 22.0 - bx;
        let line = ui.fit(13.0, W::R, PANIC_KEY, room);
        ui.text_mid(bx, y + 74.0, 38.0, 13.0, W::R, MUTED, &line);
    }
    // The idle timers: lock, then power off with nothing left to lose.
    let rows: [IdleRow; 2] = [
        (
            "Lock after",
            app.idle_lock_min,
            &crate::stick_settings::IDLE_LOCK_CHOICES,
            Action::IdleLock,
        ),
        (
            "Power off after",
            app.idle_off_min,
            &crate::stick_settings::IDLE_OFF_CHOICES,
            Action::IdleOff,
        ),
    ];
    let mut ry = y + 126.0;
    for (label, now, choices, action) in rows {
        ui.text_mid(x + 22.0, ry, 38.0, 13.0, W::R, MUTED, label);
        let mut sx = x + 150.0;
        for &m in choices {
            let text = if m == 0 {
                "Never".to_string()
            } else {
                format!("{m} min")
            };
            let style = if now == m {
                Style::Primary
            } else {
                Style::Secondary
            };
            sx += ui.button(sx, ry, Some(76.0), 38.0, &text, style, action(m)) + 8.0;
        }
        ry += 46.0;
    }
    if !app.ignored_inputs.is_empty() {
        let names: Vec<&str> = app.ignored_inputs.iter().map(|(_, n)| n.as_str()).collect();
        let line = format!("Ignored until unplugged: {}", names.join(", "));
        let line = ui.fit(13.0, W::R, &line, width - 44.0);
        ui.text_mid(x + 22.0, ry, 26.0, 13.0, W::R, WARN, &line);
    }
    y += 244.0 + ignored_h;
    ui.card(x, y, width, 94.0, LINE);
    ui.text(x + 22.0, y + 20.0, 15.0, W::S, TEXT, "Network");
    let mut sx = x + 22.0;
    for (label, style, action) in network_row_items(app) {
        sx += ui.button(sx, y + 46.0, None, 38.0, label, style, action) + 8.0;
    }
    if let Some(note) = network_row_note(app) {
        ui.text_mid(sx + 6.0, y + 46.0, 38.0, 13.0, W::R, MUTED, note);
    }
    y += 114.0;
    // Each theme a tile in its own colours, in rows as wide as the card.
    let inner = width - 44.0;
    let per_row =
        (((inner + 8.0) / (ui.theme_tile_min() + 8.0)).floor() as usize).clamp(1, Theme::ALL.len());
    let tile_w = (inner - 8.0 * (per_row - 1) as f32) / per_row as f32;
    let rows = Theme::ALL.len().div_ceil(per_row);
    let tile_h = 72.0;
    let card_h = 54.0 + rows as f32 * (tile_h + 10.0) - 10.0 + 22.0;
    ui.card(x, y, width, card_h, LINE);
    ui.text(x + 22.0, y + 20.0, 15.0, W::S, TEXT, "Appearance");
    for (i, theme) in Theme::ALL.into_iter().enumerate() {
        ui.theme_tile(
            x + 22.0 + (i % per_row) as f32 * (tile_w + 8.0),
            y + 54.0 + (i / per_row) as f32 * (tile_h + 10.0),
            tile_w,
            tile_h,
            theme,
            app.theme == theme,
            Action::Theme(theme),
        );
    }
    y += card_h + 20.0;
    ui.card(x, y, width, 112.0, LINE);
    ui.text(x + 22.0, y + 20.0, 15.0, W::S, TEXT, "Motion");
    let mut sx = x + 22.0;
    for (label, reduced) in [("Full", false), ("Reduced", true)] {
        let style = if app.reduce_motion == reduced {
            Style::Primary
        } else {
            Style::Secondary
        };
        sx += ui.button(
            sx,
            y + 54.0,
            Some(96.0),
            38.0,
            label,
            style,
            Action::ReduceMotion(reduced),
        ) + 8.0;
    }
    y += 132.0;
    ui.card(x, y, width, 112.0, LINE);
    ui.text(x + 22.0, y + 20.0, 15.0, W::S, TEXT, "Display scale");
    let mut sx = x + 22.0;
    for pct in [75u16, 90, 100, 110, 125, 150, 200] {
        let label = if pct == 100 {
            "Auto".to_string()
        } else {
            format!("{pct} %")
        };
        let style = if app.scale_pct == pct {
            Style::Primary
        } else {
            Style::Secondary
        };
        sx += ui.button(
            sx,
            y + 54.0,
            Some(76.0),
            38.0,
            &label,
            style,
            Action::Scale(pct),
        ) + 8.0;
    }
    y += 132.0;
    ui.card(x, y, width, 112.0, LINE);
    ui.text(x + 22.0, y + 20.0, 15.0, W::S, TEXT, "Signed amounts");
    let n = app.signed_amounts.len();
    ui.text_right(
        x + width - 22.0,
        y + 16.0,
        24.0,
        13.0,
        W::R,
        MUTED,
        &format!(
            "{n} {} remembered",
            if n == 1 {
                "transaction"
            } else {
                "transactions"
            }
        ),
    );
    let mut sx = x + 22.0;
    for (label, on) in [
        ("Sealed into an open vault", true),
        ("Kept until power-off only", false),
    ] {
        let style = if app.seal_amounts == on {
            Style::Primary
        } else {
            Style::Secondary
        };
        sx += ui.button(
            sx,
            y + 54.0,
            None,
            38.0,
            label,
            style,
            Action::SealAmounts(on),
        ) + 8.0;
    }
    y += 132.0;
    // Upgrading another stick from this one (`PLAN.md` §5.5): never in the
    // online app, which has no boot partition to copy.
    if !app.online {
        ui.card(x, y, width, 112.0, LINE);
        ui.text(x + 22.0, y + 20.0, 15.0, W::S, TEXT, app.medium.caps());
        ui.button(
            x + 22.0,
            y + 54.0,
            None,
            38.0,
            app.medium.upgrade(),
            Style::Secondary,
            Action::UpgradeOpen,
        );
        y += 132.0;
    }
    // The note at its foot, measured first: it wraps on a narrow column.
    let note = "Visit nakamotoinstitute.org to learn about Bitcoin’s history, economics, and \
                technology. Not affiliated.";
    ui.c.push_clip(osk_ui::Rect::new(0, 0, 0, 0));
    let note_h = ui.wrap(x + 22.0, 0.0, width - 44.0, 13.0, W::R, MUTED, note);
    ui.c.pop_clip();
    let about_h = 210.0 + note_h + 14.0;
    ui.card(x, y, width, about_h, LINE);
    ui.text(x + 22.0, y + 20.0, 15.0, W::S, TEXT, "About");
    let lines = [
        format!("Faraday {}", crate::version_label()),
        "Keys are typed, scanned as SeedQR or loaded from a vault, and kept for the session only"
            .to_string(),
        format!(
            "Vaults · files move by {noun} · QR by camera and from PNG files on {a}",
            noun = app.medium.noun(),
            a = app.medium.a()
        ),
        if app.online {
            format!("{} are folders in ~/faraday-sticks", app.medium.caps())
        } else {
            format!(
                "{} are read by an unprivileged disk process; the kernel mounts none",
                app.medium.caps()
            )
        },
    ];
    let mut ly = y + 50.0;
    for l in &lines {
        ui.text(x + 22.0, ly, 13.0, W::R, MUTED, l);
        ly += 26.0;
    }
    // The start-up self-test: what it found, and a run again.
    let s = &opensigner_core::strings::EN;
    let (result, tone) = match app.selftest() {
        Some(Ok(n)) => (
            s.settings_selftest_passed.replacen("{}", &n.to_string(), 1),
            OK,
        ),
        Some(Err(c)) => (s.settings_selftest_failed.replacen("{}", c, 1), ERR),
        None => ("not run".to_string(), MUTED),
    };
    ly += 4.0;
    ui.text_mid(x + 22.0, ly, 34.0, 13.0, W::R, MUTED, s.settings_selftest);
    ui.text_mid(x + 140.0, ly, 34.0, 13.0, W::S, tone, &result);
    let rw = ui.measure(13.0, W::S, s.settings_selftest_run) + 32.0;
    ui.button(
        x + width - 22.0 - rw,
        ly,
        Some(rw),
        34.0,
        s.settings_selftest_run,
        Style::Secondary,
        Action::SelfTestRun,
    );
    ui.wrap(x + 22.0, ly + 48.0, width - 44.0, 13.0, W::R, MUTED, note);
    // Taller than a short screen: it scrolls.
    let content = y + about_h + 44.0 - top;
    let view = ui.rect(x0, 0.0, cw, h);
    ui.report_scroll(view, content - h);
}

/// The start-up self-test failed: which check, and Exit. A build whose
/// published vectors do not reproduce has nowhere to go but out.
fn selftest_failed(ui: &mut Ui, w: f32, h: f32, check: &str) {
    let s = &opensigner_core::strings::EN;
    ui.clear(BG);
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

// ---------------------------------------------------------------------
// Sheets
// ---------------------------------------------------------------------

pub(crate) fn sheet_box(ui: &mut Ui, w: f32, h: f32, sw: f32, sh: f32) -> (f32, f32) {
    let x = (w - sw) / 2.0;
    let y = (h - sh) / 2.0;
    if let Some(a) = ui.outside {
        ui.hit_around(x, y, sw, sh, a);
    }
    ui.shadow(x, y, sw, sh, 16.0);
    ui.fill(x, y, sw, sh, 16.0, SURFACE);
    ui.stroke(x, y, sw, sh, 16.0, BORDER);
    (x, y)
}

/// Something that loads a key or turns the camera on was pressed with a
/// stick attached: what it was, and the stick. It closes, and what was pressed carries on, when
/// the stick is pulled.
fn pull_sheet(app: &Faraday, ui: &mut Ui, w: f32, h: f32) {
    let what = app.pull.and_then(|a| app.pull_what(a)).unwrap_or("go on");
    let title = format!("Pull the {} to {what}", app.medium.noun());
    let line = match app.sticks.len() {
        0 | 1 => format!(
            "{} · attached",
            app.sticks.first().map_or("", |s| s.label.as_str())
        ),
        k => format!("{k} {} attached", app.medium.nouns()),
    };
    if ui.compact {
        crate::compact::sheet(ui, w, h, &mut |ui, x, y, iw| {
            let mut cy = y;
            cy += crate::compact::sheet_head(ui, x, cy, iw, app.medium.icon(), WARN, &title);
            cy += ui.wrap(x, cy, iw, 13.0, W::R, MUTED, &line) + 16.0;
            cy += crate::compact::buttons(
                ui,
                x,
                cy,
                iw,
                &[("Cancel", Style::Secondary, Action::Cancel)],
            );
            cy - y
        });
        return;
    }
    let sh = 196.0;
    let (x, y) = sheet_box(ui, w, h, 480.0, sh);
    let ix = x + 32.0;
    let iw = 480.0 - 64.0;
    ui.icon(ix, y + 30.0, 30.0, app.medium.icon(), 18.0, WARN);
    let title = ui.fit(20.0, W::S, &title, iw - 42.0);
    ui.text_mid(ix + 42.0, y + 30.0, 30.0, 20.0, W::S, TEXT, &title);
    let line = ui.fit(13.0, W::R, &line, iw);
    ui.text(ix, y + 78.0, 13.0, W::R, MUTED, &line);
    ui.button(
        ix,
        y + sh - 32.0 - 46.0,
        Some(iw),
        46.0,
        "Cancel",
        Style::Secondary,
        Action::Cancel,
    );
}

/// The lock, write-out and power-off sheets' sections (§4.5): Kept,
/// one line per vault with its currency and one for the files For the
/// stick; then Wiped from memory, each seed and wallet with where else it
/// is, "nowhere else" alone in the danger tone; then Then.
fn kept_sections(
    app: &Faraday,
    power_off: bool,
    then: String,
) -> Vec<(&'static str, Vec<(String, osk_ui::Color)>)> {
    let kw = app.kept_wiped(power_off);
    vec![
        ("Kept", kw.kept.into_iter().map(|l| (l, TEXT)).collect()),
        (
            "Wiped from memory",
            kw.wiped
                .into_iter()
                .map(|(l, nowhere)| (l, if nowhere { ERR } else { TEXT }))
                .collect(),
        ),
        ("Then", vec![(then, TEXT)]),
    ]
}

/// A sheet that says what a lock or a power-off does: its title, a line,
/// the sections, each line wrapped to the sheet, and its buttons.
fn kept_sheet(
    ui: &mut Ui,
    w: f32,
    h: f32,
    head: (Icon, osk_ui::Color, &str),
    line: &str,
    sections: &[(&str, Vec<(String, osk_ui::Color)>)],
    actions: &[(&str, Style, Action)],
) {
    let (place, margin, pad) = if ui.compact {
        ((8.0, w - 16.0), 8.0, 16.0)
    } else {
        let sw = (w - 48.0).min(600.0);
        (((w - sw) / 2.0, sw), 40.0, 32.0)
    };
    crate::compact::scroll_sheet(ui, place, h, margin, pad, &mut |ui, x, y, iw| {
        let mut cy = y;
        if ui.compact {
            cy += crate::compact::sheet_head(ui, x, cy, iw, head.0, head.1, head.2);
        } else {
            ui.icon(x, cy, 30.0, head.0, 18.0, head.1);
            let t = ui.fit(20.0, W::S, head.2, iw - 42.0);
            ui.text_mid(x + 42.0, cy, 30.0, 20.0, W::S, TEXT, &t);
            cy += 46.0;
        }
        if !line.is_empty() {
            cy += ui.wrap(x, cy, iw, 13.0, W::R, MUTED, line) + 12.0;
        }
        for (name, lines) in sections {
            if lines.is_empty() {
                continue;
            }
            ui.rule(x, cy, iw, INNER);
            cy += 12.0;
            ui.text(x, cy, 12.0, W::R, MUTED, name);
            cy += 22.0;
            for (text, tone) in lines {
                cy += ui.wrap(x, cy, iw, 14.0, W::R, *tone, text) + 8.0;
            }
            cy += 4.0;
        }
        cy += 8.0;
        cy += crate::compact::buttons(ui, x, cy, iw, actions);
        cy - y
    });
}

fn lock_sheet(app: &Faraday, ui: &mut Ui, w: f32, h: f32) {
    let label = app
        .sticks
        .first()
        .map(|s| s.label.clone())
        .unwrap_or_default();
    // Asked for by Upgrade rather than by a stick arriving.
    let upgrade = app.upgrade_after_lock;
    let then = if upgrade {
        "A fresh start, and the upgrade".to_string()
    } else {
        format!("A fresh start, and the {} visit", app.medium.noun())
    };
    let (attached, use_it, line) = if upgrade {
        (
            app.medium.upgrade().to_string(),
            "Lock and upgrade".to_string(),
            "The upgrade runs in a fresh session".to_string(),
        )
    } else {
        (
            format!("{} is attached", app.medium.a_cap()),
            format!("Lock and use {}", app.medium.noun()),
            format!("{label} · nothing has been read from it"),
        )
    };
    kept_sheet(
        ui,
        w,
        h,
        (app.medium.icon(), WARN, &attached),
        &line,
        &kept_sections(app, false, then),
        &[
            ("Not now", Style::Secondary, Action::NotNow),
            (&use_it, Style::Primary, Action::Lock),
        ],
    );
}

/// Writing what is For the stick from a session that held a secret: the
/// lock comes first, and this says what it keeps and wipes.
fn write_out_sheet(app: &Faraday, ui: &mut Ui, w: f32, h: f32) {
    let then = format!(
        "Lock, plug in {}, and the visit writes them",
        app.medium.a()
    );
    let title = format!("Write to {}", app.medium.a());
    kept_sheet(
        ui,
        w,
        h,
        (app.medium.icon(), ACCENT, &title),
        "",
        &kept_sections(app, false, then),
        &[
            ("Not now", Style::Secondary, Action::Cancel),
            ("Lock", Style::Primary, Action::Lock),
        ],
    );
}

fn scan_sheet(app: &Faraday, ui: &mut Ui, w: f32, h: f32) {
    let Some(sc) = app.scan.as_ref() else { return };
    let pad = if ui.compact { 16.0 } else { 32.0 };
    let side_w = if ui.compact {
        w - 16.0 - 2.0 * pad
    } else {
        (w - 200.0).clamp(320.0, 640.0)
    };
    let side_h = side_w * 0.75;
    let sw = side_w + 2.0 * pad;
    let rows = if ui.compact && app.cameras.len() > 1 {
        app.cameras.len() as f32 * 40.0
    } else {
        0.0
    };
    let sh = side_h + 170.0 + rows;
    let (x, y) = sheet_box(ui, w, h, sw, sh);
    let x = x + pad - 32.0;
    let sw = sw - 2.0 * (pad - 32.0);
    let title = match sc.purpose {
        crate::ScanPurpose::Seed => "Scan a SeedQR",
        crate::ScanPurpose::CheckCopy(_) => "Scan your copy",
        crate::ScanPurpose::Transfer => "Receive into Downloads",
        _ => "Scan a QR code",
    };
    ui.text(x + 32.0, y + 26.0, 18.0, W::S, TEXT, title);
    let line = sc
        .note
        .clone()
        .unwrap_or_else(|| "Hold the code in front of the camera".to_string());
    ui.text(x + 32.0, y + 56.0, 12.0, W::R, MUTED, &line);
    let rect = ui.rect(x + 32.0, y + 82.0, side_w, side_h);
    ui.fill(x + 32.0, y + 82.0, side_w, side_h, 8.0, BG);
    if let Some((fw, fh, luma)) = sc.frame.as_ref() {
        ui.c.luma(rect, *fw, *fh, luma, sc.chroma.as_deref());
        // The code the scanner last found, outlined where it is: green
        // once read, amber while it is found and not read.
        // Gone ten preview frames after the scanner stops seeing it.
        if let Some((seen, at)) = sc.seen
            && sc.frames.wrapping_sub(at) <= 10
            && seen.width > 0
            && seen.height > 0
        {
            // The preview fills its box and crops the longer side, as
            // `Canvas::luma` draws it: the same scale and crop here.
            let (fw, fh) = (f32::from(*fw), f32::from(*fh));
            let scale = (rect.w as f32 / fw).max(rect.h as f32 / fh);
            let left = (fw - rect.w as f32 / scale) / 2.0;
            let top = (fh - rect.h as f32 / scale) / 2.0;
            let map = |(cx, cy): (u16, u16)| {
                let px = f32::from(cx) * fw / f32::from(seen.width);
                let py = f32::from(cy) * fh / f32::from(seen.height);
                (
                    rect.x as f32 + (px - left) * scale,
                    rect.y as f32 + (py - top) * scale,
                )
            };
            let color = if seen.read { OK } else { WARN };
            let lw = (3.0 * ui.f).max(2.0);
            for i in 0..4 {
                let (a, b) = (map(seen.corners[i]), map(seen.corners[(i + 1) % 4]));
                ui.c.line(a.0, a.1, b.0, b.1, lw, color);
            }
        }
    } else {
        ui.text_mid(
            x + 32.0 + side_w / 2.0 - 60.0,
            y + 82.0,
            side_h,
            13.0,
            W::R,
            DIM,
            "Starting the camera",
        );
    }
    // Which camera, when there is more than one.
    if app.cameras.len() > 1 && ui.compact {
        let chosen = app.camera.as_deref();
        let row: Vec<(String, Style, Action)> = app
            .cameras
            .iter()
            .enumerate()
            .map(|(i, (id, name))| {
                let on = chosen.map_or(i == 0, |c| c == id);
                (
                    ui.fit(12.0, W::S, name, sw - 96.0),
                    if on { Style::Primary } else { Style::Secondary },
                    Action::ScanCamera(i as u8),
                )
            })
            .collect();
        button_rows(ui, x + 32.0, y + 82.0 + side_h + 12.0, sw - 64.0, &row);
    } else if app.cameras.len() > 1 {
        let chosen = app.camera.as_deref();
        let row: Vec<(String, Style, Action)> = app
            .cameras
            .iter()
            .enumerate()
            .map(|(i, (id, name))| {
                let on = chosen.map_or(i == 0, |c| c == id);
                (
                    ui.fit(12.0, W::S, name, 180.0),
                    if on { Style::Primary } else { Style::Secondary },
                    Action::ScanCamera(i as u8),
                )
            })
            .collect();
        button_rows(
            ui,
            x + 32.0,
            y + sh - 32.0 - 44.0 + 6.0,
            sw - 64.0 - 140.0,
            &row,
        );
    }
    ui.button(
        x + sw - 32.0 - 120.0,
        y + sh - 32.0 - 44.0,
        Some(120.0),
        44.0,
        "Cancel",
        Style::Secondary,
        Action::Cancel,
    );
}

fn qr_sheet(app: &Faraday, ui: &mut Ui, w: f32, h: f32) {
    let Some(q) = app.qr.as_ref() else { return };
    if ui.compact {
        qr_sheet_compact(app, q, ui, w, h);
        return;
    }
    // A descriptor's whole text goes under the code, which gives up the
    // height it takes.
    let mut side = (h - 272.0).clamp(240.0, 560.0);
    let mut whole_h = 0.0;
    if let Some(t) = &q.whole {
        for _ in 0..2 {
            let tw = (side + 64.0).max(420.0) - 64.0;
            whole_h = measure_wrap(ui, tw, 11.0, W::M, t) + 10.0;
            side = (h - 272.0 - whole_h).clamp(240.0, 560.0);
        }
        whole_h = measure_wrap(ui, (side + 64.0).max(420.0) - 64.0, 11.0, W::M, t) + 10.0;
    }
    let sw = (side + 64.0).max(420.0);
    let sh = side + 242.0 + whole_h;
    let (x, y) = sheet_box(ui, w, h, sw, sh);
    let t = ui.fit(18.0, W::S, &q.title, sw - 64.0);
    ui.text(x + 32.0, y + 26.0, 18.0, W::S, TEXT, &t);
    let sub = if q.frames.len() > 1 {
        format!(
            "{} · frame {} of {}",
            q.subtitle,
            q.frame + 1,
            q.frames.len()
        )
    } else {
        q.subtitle.clone()
    };
    let sub = ui.fit(12.0, W::R, &sub, sw - 64.0);
    ui.text(x + 32.0, y + 50.0, 12.0, W::R, MUTED, &sub);
    // Whether scanning it is safe, said beside the code itself.
    let (tag, tone) = if q.secret {
        ("Secret · whoever scans this can use it", ERR)
    } else {
        ("Public · safe to scan", OK)
    };
    ui.text(x + 32.0, y + 66.0, 12.0, W::S, tone, tag);
    if let Some(m) = q.frames.get(q.frame) {
        qr_code(ui, m, x + (sw - side) / 2.0, y + 86.0, side);
    }
    if let Some(t) = &q.whole {
        ui.wrap(
            x + 32.0,
            y + 86.0 + side + 10.0,
            sw - 64.0,
            11.0,
            W::M,
            TEXT,
            t,
        );
    }
    // How it is written: the format, and for an animated code its speed
    // and how much each frame holds.
    let row = y + sh - 32.0 - 44.0 - 52.0;
    let mut bx = x + 32.0;
    // Numbered parts carry text only: a descriptor or a key.
    let text = matches!(
        q.source,
        crate::QrSource::Text(_) | crate::QrSource::Key(..)
    );
    for (i, (label, f)) in ["UR", "BBQr", "Parts"]
        .iter()
        .zip(crate::QrFormat::ALL)
        .enumerate()
    {
        if f == crate::QrFormat::Parts && !text {
            continue;
        }
        bx += ui.button(
            bx,
            row,
            Some(64.0),
            36.0,
            label,
            if q.format == f {
                Style::Primary
            } else {
                Style::Secondary
            },
            Action::QrFormat(i as u8),
        ) + 6.0;
    }
    if q.frames.len() > 1 {
        bx += 14.0;
        for (label, ms) in ["2/s", "3/s", "5/s"].iter().zip(crate::QR_SPEEDS) {
            let on = app.qr_frame_ms == ms;
            bx += ui.button(
                bx,
                row,
                Some(52.0),
                36.0,
                label,
                if on { Style::Primary } else { Style::Secondary },
                Action::QrSpeed(ms),
            ) + 6.0;
        }
    }
    let mut bx = x + 32.0;
    if q.offers_png() {
        // One code of public content: kept as a picture with its label.
        ui.button(
            bx,
            y + sh - 32.0 - 44.0,
            None,
            44.0,
            &format!("PNG {}", app.medium.for_the()),
            Style::Secondary,
            Action::QrPng,
        );
    } else if q.frames.len() > 1 || q.part != crate::QR_PARTS[1] {
        for (label, n) in ["Small", "Medium", "Large"].iter().zip(crate::QR_PARTS) {
            let on = q.part == n;
            bx += ui.button(
                bx,
                y + sh - 32.0 - 44.0,
                Some(76.0),
                44.0,
                label,
                if on { Style::Primary } else { Style::Secondary },
                Action::QrPartSize(n),
            ) + 6.0;
        }
    }
    let _ = bx;
    ui.button(
        x + sw - 32.0 - 100.0,
        y + sh - 32.0 - 44.0,
        Some(100.0),
        44.0,
        "Done",
        Style::Primary,
        Action::Cancel,
    );
}

/// The height `s` takes wrapped to `width`, drawn nowhere.
fn measure_wrap(ui: &mut Ui, width: f32, size: f32, w: W, s: &str) -> f32 {
    ui.c.push_clip(osk_ui::Rect::new(0, 0, 0, 0));
    let h = ui.wrap(0.0, 0.0, width, size, w, TEXT, s);
    ui.c.pop_clip();
    h
}

/// The QR sheet on a small panel: the code as large as the panel allows,
/// and its format, speed and part size as buttons that step through the
/// choices, with PNG beside them for one code of public content.
fn qr_sheet_compact(app: &Faraday, q: &crate::QrView, ui: &mut Ui, w: f32, h: f32) {
    let text = matches!(
        q.source,
        crate::QrSource::Text(_) | crate::QrSource::Key(..)
    );
    let formats: Vec<(usize, crate::QrFormat)> = crate::QrFormat::ALL
        .iter()
        .copied()
        .enumerate()
        .filter(|(_, f)| *f != crate::QrFormat::Parts || text)
        .collect();
    let at = formats
        .iter()
        .position(|(_, f)| *f == q.format)
        .unwrap_or(0);
    let (next_i, _) = formats[(at + 1) % formats.len()];
    let fname = ["UR", "BBQr", "Parts"][crate::QrFormat::ALL
        .iter()
        .position(|f| *f == q.format)
        .unwrap_or(0)];
    let animated = q.frames.len() > 1;
    let sp = crate::QR_SPEEDS
        .iter()
        .position(|ms| *ms == app.qr_frame_ms)
        .unwrap_or(0);
    let pp = crate::QR_PARTS
        .iter()
        .position(|n| *n == q.part)
        .unwrap_or(1);
    let mut cycles: Vec<(&str, Style, Action)> = Vec::new();
    if formats.len() > 1 {
        cycles.push((fname, Style::Secondary, Action::QrFormat(next_i as u8)));
    }
    if animated {
        cycles.push((
            ["2/s", "3/s", "5/s"][sp],
            Style::Secondary,
            Action::QrSpeed(crate::QR_SPEEDS[(sp + 1) % 3]),
        ));
    }
    if animated || q.part != crate::QR_PARTS[1] {
        cycles.push((
            ["Small", "Medium", "Large"][pp],
            Style::Secondary,
            Action::QrPartSize(crate::QR_PARTS[(pp + 1) % 3]),
        ));
    }
    // One code of public content: kept as a picture with its label.
    if q.offers_png() {
        cycles.push(("PNG", Style::Secondary, Action::QrPng));
    }
    let rows_h = if cycles.is_empty() { 0.0 } else { 48.0 } + 46.0;
    crate::compact::sheet(ui, w, h, &mut |ui, x, y, iw| {
        let mut cy = y;
        let t = ui.fit(16.0, W::S, &q.title, iw);
        ui.text(x, cy, 16.0, W::S, TEXT, &t);
        cy += 22.0;
        let sub = if animated {
            format!("{} · {} of {}", q.subtitle, q.frame + 1, q.frames.len())
        } else {
            q.subtitle.clone()
        };
        let sub = ui.fit(12.0, W::R, &sub, iw);
        ui.text(x, cy, 12.0, W::R, MUTED, &sub);
        cy += 18.0;
        let (tag, tone) = if q.secret {
            ("Secret · whoever scans this can use it", ERR)
        } else {
            ("Public · safe to scan", OK)
        };
        let tag = ui.fit(12.0, W::S, tag, iw);
        ui.text(x, cy, 12.0, W::S, tone, &tag);
        cy += 22.0;
        let whole_h = q
            .whole
            .as_ref()
            .map_or(0.0, |t| measure_wrap(ui, iw, 11.0, W::M, t) + 8.0);
        let side = iw
            .min(h - 16.0 - 32.0 - (cy - y) - rows_h - 8.0 - whole_h)
            .max(120.0);
        if let Some(m) = q.frames.get(q.frame) {
            qr_code(ui, m, x + (iw - side) / 2.0, cy, side);
        }
        cy += side + 8.0;
        if let Some(t) = &q.whole {
            ui.wrap(x, cy, iw, 11.0, W::M, TEXT, t);
            cy += whole_h;
        }
        if !cycles.is_empty() {
            let n = cycles.len() as f32;
            let bw = (iw - (n - 1.0) * 6.0) / n;
            for (i, &(label, style, action)) in cycles.iter().enumerate() {
                ui.button(
                    x + i as f32 * (bw + 6.0),
                    cy,
                    Some(bw),
                    40.0,
                    label,
                    style,
                    action,
                );
            }
            cy += 48.0;
        }
        cy += crate::compact::buttons(ui, x, cy, iw, &[("Done", Style::Primary, Action::Cancel)]);
        cy - y
    });
}

/// A QR code on a white card, square modules, the quiet zone included.
pub(crate) fn qr_code(ui: &mut Ui, m: &osk_codec::qr::QrMatrix, x: f32, y: f32, side: f32) {
    ui.fill(x, y, side, side, 8.0, osk_ui::Color::WHITE);
    let n = m.size();
    let quiet = 4.0;
    // Whole pixels per module, so every module is the same size.
    let px_side = ui.px(side);
    let module_px = (px_side as f32 / (n as f32 + 2.0 * quiet)).floor().max(1.0) as i32;
    let used = module_px * n as i32;
    let r = ui.rect(x, y, side, side);
    let ox = r.x + (r.w - used) / 2;
    let oy = r.y + (r.h - used) / 2;
    for row in 0..n {
        let mut col = 0;
        while col < n {
            if m.module(col, row) {
                let start = col;
                while col < n && m.module(col, row) {
                    col += 1;
                }
                let rect = osk_ui::Rect::new(
                    ox + start as i32 * module_px,
                    oy + row as i32 * module_px,
                    (col - start) as i32 * module_px,
                    module_px,
                );
                ui.c.fill_rect(rect, osk_ui::Color::BLACK);
            } else {
                col += 1;
            }
        }
    }
}

/// A new input device (`PLAN.md` §4.6): its name as it gives it, the
/// code a keyboard must type, and the person's choice.
fn new_input_sheet(app: &Faraday, ui: &mut Ui, w: f32, h: f32) {
    let Some(d) = app.inputs.first() else { return };
    if ui.compact {
        let what = match (d.keyboard, d.pointer) {
            (true, true) => "A new keyboard and pointer",
            (true, false) => "A new keyboard",
            _ => "A new pointer",
        };
        crate::compact::sheet(ui, w, h, &mut |ui, x, y, iw| {
            let mut cy = y;
            cy += crate::compact::sheet_head(ui, x, cy, iw, Icon::Warning, WARN, what);
            cy += ui.wrap(x, cy, iw, 14.0, W::M, TEXT, &d.name) + 6.0;
            cy += ui.wrap(
                x,
                cy,
                iw,
                12.0,
                W::R,
                MUTED,
                "The name is what the device says about itself",
            ) + 12.0;
            if d.keyboard {
                ui.text(x, cy, 13.0, W::S, MUTED, "Type this code on it");
                cy += 24.0;
                let n = d.code.chars().count().max(1) as f32;
                let bw = ((iw - (n - 1.0) * 6.0) / n).min(52.0);
                let typed = d.typed.chars().count();
                for (k, c) in d.code.chars().enumerate() {
                    let done = d.code.starts_with(&d.typed) && k < typed;
                    let cx = x + k as f32 * (bw + 6.0);
                    ui.fill(
                        cx,
                        cy,
                        bw,
                        52.0,
                        8.0,
                        if done { OK.with_alpha(40) } else { INNER },
                    );
                    ui.stroke(cx, cy, bw, 52.0, 8.0, if done { OK } else { BORDER });
                    let cs = c.to_string();
                    let tw = ui.measure(24.0, W::M, &cs);
                    ui.text_mid(cx + (bw - tw) / 2.0, cy, 52.0, 24.0, W::M, TEXT, &cs);
                }
                cy += 64.0;
            }
            let mut items = vec![(
                "Ignore this device",
                Style::Secondary,
                Action::InputIgnore(d.id),
            )];
            if !d.keyboard {
                items.push(("Use this pointer", Style::Primary, Action::InputUse(d.id)));
            }
            cy += crate::compact::buttons(ui, x, cy, iw, &items);
            cy - y
        });
        return;
    }
    let sh = if d.keyboard { 360.0 } else { 280.0 };
    let (x, y) = sheet_box(ui, w, h, 560.0, sh);
    let ix = x + 32.0;
    let iw = 560.0 - 64.0;
    let what = match (d.keyboard, d.pointer) {
        (true, true) => "A new keyboard and pointer",
        (true, false) => "A new keyboard",
        _ => "A new pointer",
    };
    ui.icon(ix, y + 28.0, 30.0, Icon::Warning, 17.0, WARN);
    ui.text_mid(ix + 42.0, y + 28.0, 30.0, 20.0, W::S, TEXT, what);
    let name = ui.fit(14.0, W::M, &d.name, iw);
    ui.text(ix, y + 76.0, 14.0, W::M, TEXT, &name);
    ui.text(
        ix,
        y + 100.0,
        13.0,
        W::R,
        MUTED,
        "The name is what the device says about itself",
    );
    let mut by = y + 140.0;
    if d.keyboard {
        ui.text(ix, by, 13.0, W::S, MUTED, "Type this code on it");
        by += 28.0;
        let mut cx = ix;
        let typed: Vec<char> = d.typed.chars().collect();
        for (k, c) in d.code.chars().enumerate() {
            // A letter already typed in order is marked.
            let done = d.code.starts_with(&d.typed) && k < typed.len();
            ui.fill(
                cx,
                by,
                52.0,
                60.0,
                8.0,
                if done { OK.with_alpha(40) } else { INNER },
            );
            ui.stroke(cx, by, 52.0, 60.0, 8.0, if done { OK } else { BORDER });
            ui.text_mid(cx + 16.0, by, 60.0, 28.0, W::M, TEXT, &c.to_string());
            cx += 60.0;
        }
        by += 84.0;
    }
    let bw = (iw - 12.0) / 2.0;
    ui.button(
        ix,
        y + sh - 32.0 - 46.0,
        Some(if d.keyboard { iw } else { bw }),
        46.0,
        "Ignore this device",
        Style::Secondary,
        Action::InputIgnore(d.id),
    );
    if !d.keyboard {
        ui.button(
            ix + bw + 12.0,
            y + sh - 32.0 - 46.0,
            Some(bw),
            46.0,
            "Use this pointer",
            Style::Primary,
            Action::InputUse(d.id),
        );
    }
    let _ = by;
}

/// One idle timer on Settings: its label, its minutes now, the choices
/// (0 for never), and the action that sets it.
type IdleRow = (&'static str, u16, &'static [u16], fn(u16) -> Action);

/// Minutes and seconds, `4:05`.
fn clock_text(ms: u64) -> String {
    let s = ms.div_ceil(1000);
    format!("{}:{:02}", s / 60, s % 60)
}

/// The idle lock coming: how long without input, when it locks, what it
/// wipes, seals and keeps, and when the machine powers off after it.
fn idle_warn_sheet(app: &Faraday, ui: &mut Ui, w: f32, h: f32) {
    if app.idle_lock_at().is_none() {
        return;
    }
    let keys = app.session.keys.len();
    let wallets = app.session.wallets.len();
    let sealed: Vec<String> = app
        .vaults
        .open
        .iter()
        .filter(|v| v.changes > 0)
        .map(|v| v.label())
        .collect();
    let mut rows: Vec<(&str, String)> = vec![(
        "Wiped",
        format!(
            "{keys} {} · {wallets} {} · {} open {}",
            if keys == 1 { "seed" } else { "seeds" },
            if wallets == 1 { "wallet" } else { "wallets" },
            app.vaults.open.len(),
            if app.vaults.open.len() == 1 {
                "vault"
            } else {
                "vaults"
            }
        ),
    )];
    if !sealed.is_empty() {
        rows.push((
            "Sealed",
            format!("{} · {}", sealed.join(", "), app.medium.for_the()),
        ));
    }
    rows.push((
        "Kept",
        format!(
            "{} {} · {} {}",
            app.inbox.len(),
            app.medium.from_the(),
            app.outbox.len(),
            app.medium.for_the()
        ),
    ));
    let off = match (app.idle_off_min, app.online) {
        (0, _) | (_, true) => "No power-off".to_string(),
        (m, false) => {
            format!(
                "Power off at {m} minutes without input, unless files wait {}",
                app.medium.for_the()
            )
        }
    };
    rows.push(("Then", off));
    let minutes = |n: u64| format!("{n} {}", if n == 1 { "minute" } else { "minutes" });
    let body = format!(
        "No input for {} · locks at {} without input · any key or touch keeps it",
        minutes(app.idle_warn_min),
        minutes(u64::from(app.idle_lock_min))
    );
    if ui.compact {
        let rows: Vec<(&str, String, osk_ui::Color)> =
            rows.into_iter().map(|(k, v)| (k, v, TEXT)).collect();
        crate::compact::kv_sheet(
            ui,
            w,
            h,
            (Icon::Lock, WARN, "Locking soon"),
            &body,
            &rows,
            &[("Keep working", Style::Primary, Action::Cancel)],
        );
        return;
    }
    let sw = 600.0;
    let sh = 250.0 + rows.len() as f32 * 44.0;
    let (x, y) = sheet_box(ui, w, h, sw, sh);
    let ix = x + 32.0;
    let iw = sw - 64.0;
    ui.icon(ix, y + 30.0, 30.0, Icon::Lock, 18.0, WARN);
    ui.text_mid(ix + 42.0, y + 30.0, 30.0, 20.0, W::S, TEXT, "Locking soon");
    ui.text(ix, y + 76.0, 13.0, W::R, MUTED, &body);
    let mut ry = y + 108.0;
    for (k, v) in &rows {
        ui.text_mid(ix, ry, 40.0, 13.0, W::R, MUTED, k);
        let v = ui.fit(14.0, W::R, v, iw - 90.0);
        ui.text_mid(ix + 90.0, ry, 40.0, 14.0, W::R, TEXT, &v);
        ui.rule(ix, ry + 40.0, iw, INNER);
        ry += 44.0;
    }
    ui.button(
        ix,
        y + sh - 32.0 - 46.0,
        Some(iw),
        46.0,
        "Keep working",
        Style::Primary,
        Action::Cancel,
    );
}

/// The lock screen after a lock for idleness: what waits in the Outbox,
/// which keeps the machine from powering off.
fn locked_sheet(app: &Faraday, ui: &mut Ui, w: f32, h: f32) {
    let shown = app.outbox.len().min(5);
    if ui.compact {
        crate::compact::sheet(ui, w, h, &mut |ui, x, y, iw| {
            let mut cy = y;
            cy += crate::compact::sheet_head(ui, x, cy, iw, Icon::Lock, ACCENT, "Locked");
            cy += ui.wrap(
                x,
                cy,
                iw,
                13.0,
                W::R,
                MUTED,
                "Locked after a time without input. Every key and vault was wiped.",
            ) + 12.0;
            if let Some(off) = app.idle_off_at() {
                let line = format!(
                    "Powers off in {} without input",
                    clock_text(off.saturating_sub(app.idle_ms()))
                );
                cy += ui.wrap(x, cy, iw, 13.0, W::S, TEXT, &line) + 10.0;
            }
            if shown > 0 {
                let line = format!(
                    "{} {} · no power-off until written",
                    app.outbox.len(),
                    app.medium.for_the()
                );
                cy += ui.wrap(x, cy, iw, 13.0, W::S, WARN, &line) + 8.0;
                for item in app.outbox.iter().take(shown) {
                    let n = ui.fit(13.0, W::M, &item.name, iw);
                    ui.text(x, cy, 13.0, W::M, TEXT, &n);
                    cy += 24.0;
                }
            }
            cy += 8.0;
            cy += crate::compact::buttons(
                ui,
                x,
                cy,
                iw,
                &[("Continue", Style::Primary, Action::Cancel)],
            );
            cy - y
        });
        return;
    }
    let sh = 200.0
        + if shown > 0 {
            40.0 + shown as f32 * 30.0
        } else {
            0.0
        };
    let (x, y) = sheet_box(ui, w, h, 520.0, sh);
    let ix = x + 32.0;
    let iw = 520.0 - 64.0;
    ui.icon(ix, y + 28.0, 28.0, Icon::Lock, 16.0, ACCENT);
    ui.text(ix + 40.0, y + 30.0, 20.0, W::S, TEXT, "Locked");
    ui.text(
        ix,
        y + 72.0,
        13.0,
        W::R,
        MUTED,
        "Locked after a time without input. Every key and vault was wiped.",
    );
    let mut ry = y + 108.0;
    if let Some(off) = app.idle_off_at() {
        ui.text(
            ix,
            ry,
            13.0,
            W::S,
            TEXT,
            &format!(
                "Powers off in {} without input",
                clock_text(off.saturating_sub(app.idle_ms()))
            ),
        );
    }
    if shown > 0 {
        ui.text(
            ix,
            ry,
            13.0,
            W::S,
            WARN,
            &format!(
                "{} {} · no power-off until written",
                app.outbox.len(),
                app.medium.for_the()
            ),
        );
        ry += 32.0;
        for item in app.outbox.iter().take(shown) {
            ui.text(ix, ry, 14.0, W::M, TEXT, &item.name);
            ry += 30.0;
        }
    }
    ui.button(
        ix,
        y + sh - 32.0 - 46.0,
        Some(iw),
        46.0,
        "Continue",
        Style::Primary,
        Action::Cancel,
    );
}

/// The Network row's buttons: every network, the one in use marked
/// primary, and any not reachable from here (between mainnet and a test
/// network, while wallets are loaded) disabled.
pub(crate) fn network_row_items(app: &Faraday) -> Vec<(&'static str, Style, Action)> {
    use osk_bip::keys::Network;
    let now = app.session.network();
    Network::ALL
        .iter()
        .map(|&n| {
            let here = n == now;
            let open = here || n.kind() == now.kind() || app.session.wallets.is_empty();
            let style = if here {
                Style::Primary
            } else if open {
                Style::Secondary
            } else {
                Style::Disabled
            };
            (network_name(n), style, Action::Network(n))
        })
        .collect()
}

/// "Wallets loaded", muted, after the Network row's buttons when one of
/// them is disabled: the reason a network can't be chosen, as the old
/// sheet said it.
pub(crate) fn network_row_note(app: &Faraday) -> Option<&'static str> {
    network_row_items(app)
        .iter()
        .any(|(_, style, _)| *style == Style::Disabled)
        .then_some("Wallets loaded")
}

/// The lines of the online app's mainnet warning.
const NOT_AIRGAPPED: [&str; 3] = [
    "This computer is online",
    "Anything running on it can read a mainnet key typed, scanned or loaded here",
    "For real funds, use Faraday booted from a stick on a computer with no network",
];

/// The online app going to mainnet, or found on it: this computer is not
/// air-gapped. Cancel only when mainnet was asked for from Network.
fn not_airgapped_sheet(app: &Faraday, ui: &mut Ui, w: f32, h: f32) {
    let asked = app.mainnet_asked.is_some();
    let mut buttons = Vec::new();
    if asked {
        buttons.push(("Cancel", Style::Secondary, Action::Cancel));
    }
    buttons.push(("I understand", Style::Primary, Action::AirgapUnderstood));
    if ui.compact {
        crate::compact::sheet(ui, w, h, &mut |ui, x, y, iw| {
            let mut cy = y;
            cy += crate::compact::sheet_head(ui, x, cy, iw, Icon::Warning, WARN, "Not air-gapped");
            for (i, line) in NOT_AIRGAPPED.iter().enumerate() {
                let fg = if i == 0 { WARN } else { TEXT };
                cy += ui.wrap(x, cy, iw, 13.0, W::R, fg, line) + 8.0;
            }
            cy += 8.0;
            cy += crate::compact::buttons(ui, x, cy, iw, &buttons);
            cy - y
        });
        return;
    }
    let sw = 520.0f32.min(w - 48.0);
    let sh = 290.0;
    let (x, y) = sheet_box(ui, w, h, sw, sh);
    let (bx, bw) = (x + 32.0, sw - 64.0);
    let mut cy = y + 26.0;
    ui.icon(bx - 4.0, cy - 2.0, 28.0, Icon::Warning, 14.0, WARN);
    ui.text(bx + 28.0, cy, 18.0, W::S, TEXT, "Not air-gapped");
    cy += 44.0;
    for (i, line) in NOT_AIRGAPPED.iter().enumerate() {
        let fg = if i == 0 { WARN } else { TEXT };
        cy += ui.wrap(bx, cy, bw, 14.0, W::R, fg, line) + 10.0;
    }
    let by = y + sh - 32.0 - 46.0;
    let n = buttons.len() as f32;
    let each = (bw - 12.0 * (n - 1.0)) / n;
    for (i, (label, style, action)) in buttons.into_iter().enumerate() {
        let at = bx + i as f32 * (each + 12.0);
        ui.button(at, by, Some(each), 46.0, label, style, action);
    }
}

fn power_sheet(app: &Faraday, ui: &mut Ui, w: f32, h: f32) {
    kept_sheet(
        ui,
        w,
        h,
        (Icon::Power, TEXT, "Power off"),
        "",
        &kept_sections(app, true, "The computer powers off".to_string()),
        &[
            ("Cancel", Style::Secondary, Action::Cancel),
            ("Power off", Style::Primary, Action::PowerOff),
        ],
    );
}

/// OpenSigner's Learn pages for the screen under the sheet: a tab for
/// each, the page's sections, scrolled by the wheel or the arrow keys.
fn learn_sheet(app: &mut Faraday, ui: &mut Ui, w: f32, h: f32) {
    let compact = ui.compact;
    let (sw, sh, pad) = if compact {
        (w - 16.0, h - 16.0, 14.0)
    } else {
        ((w - 80.0).min(860.0), h - 64.0, 28.0)
    };
    let (x, y) = sheet_box(ui, w, h, sw, sh);
    let Some(page) = app.learn.pages.get(app.learn.page).copied() else {
        return;
    };
    // Tabs, one per page, and Done; on a small panel they wrap.
    let done_w = if compact { 72.0 } else { 96.0 };
    ui.button(
        x + sw - pad - done_w,
        y + 14.0,
        Some(done_w),
        34.0,
        "Done",
        Style::Secondary,
        Action::Cancel,
    );
    // The sheet's first row (`docs/SIMPLIFY.md` §6.2): a direct way to
    // the Spend tab, except from the Spend tab itself.
    let mut tabs: Vec<(String, Style, Action)> = Vec::new();
    if app.screen != Screen::Family {
        tabs.push((
            "Spending, step by step".to_string(),
            Style::Secondary,
            Action::LearnSpend,
        ));
    }
    tabs.extend(app.learn.pages.iter().enumerate().map(|(i, p)| {
        let style = if i == app.learn.page {
            Style::Primary
        } else {
            Style::Secondary
        };
        (p.title.to_string(), style, Action::LearnPage(i as u8))
    }));
    let tabs_h = if tabs.len() > 1 || !compact {
        learn_tabs(ui, x + pad, y + 14.0, sw - 2.0 * pad - done_w - 8.0, &tabs) - 44.0 + 34.0
    } else {
        34.0
    };
    let top = y + 14.0 + tabs_h + 18.0;
    ui.rule(x + pad, top - 8.0, sw - 2.0 * pad, INNER);
    let (bx, bw) = (x + pad + 4.0, sw - 2.0 * pad - 8.0);
    let view = sh - (top - y) - 12.0;
    let clip = ui.rect(x, top, sw, view);
    ui.c.push_clip(clip);
    let mut cy = top + 12.0 - app.learn.scroll;
    cy += ui.wrap(
        bx,
        cy,
        bw,
        if compact { 18.0 } else { 22.0 },
        W::S,
        TEXT,
        page.title,
    ) + 16.0;
    for section in page.sections {
        if !section.heading.is_empty() {
            cy += ui.wrap(
                bx,
                cy,
                bw,
                if compact { 15.0 } else { 16.0 },
                W::S,
                ACCENT,
                section.heading,
            ) + 8.0;
        }
        for para in section.paragraphs {
            cy += ui.wrap(bx, cy, bw, 14.0, W::R, TEXT, para) + 12.0;
        }
        cy += 10.0;
    }
    ui.c.pop_clip();
    let content = cy + app.learn.scroll - top;
    app.learn.max = (content - view).max(0.0);
    if app.learn.scroll > app.learn.max {
        app.learn.scroll = app.learn.max;
    }
    // The overlay scrollbar, held and dragged as on any page.
    ui.report_scroll(clip, app.learn.max);
}

/// Where a secret goes: the open vault, or, once the person has said they
/// understand what it gives away, the Outbox unprotected.
fn secret_out_sheet(app: &Faraday, ui: &mut Ui, w: f32, h: f32) {
    let Some(out) = app.secret_out.as_ref() else {
        return;
    };
    // Where it would go into a vault: a seed the vault holds already
    // says so instead of offering to save it again.
    let v = app.vaults.current;
    let vault = app.vaults.open.get(v).map(|o| {
        let held = match out.keep {
            crate::secrets::Keep::Key { fp, .. } => crate::vault_screens::vault_has_key(app, v, fp),
            _ => false,
        };
        (o.name.as_str(), held)
    });
    // The forms the file can take, the one picked primary.
    let forms: Vec<(&str, Style, Action)> = out
        .forms
        .iter()
        .enumerate()
        .map(|(i, f)| {
            let style = if i == out.form {
                Style::Primary
            } else {
                Style::Secondary
            };
            (f.label, style, Action::SecretForm(i as u8))
        })
        .collect();
    let out_style = if app.secret_ack {
        Style::Secondary
    } else {
        Style::Disabled
    };
    // A file a visit would copy to another stick is ticked there, not put
    // in the Outbox; text Faraday cannot tell about may be a secret.
    let head = if out.ack == crate::secrets::Ack::Unknown {
        "This may be a secret"
    } else {
        "This is a secret"
    };
    let put_label = format!("Put it {} unprotected", app.medium.for_the());
    let (unprotected, out_label) = if out.to_visit {
        (
            format!("Unprotected, on the {}", app.medium.noun()),
            "Tick it to write",
        )
    } else {
        (
            format!("Unprotected, {}", app.medium.for_the()),
            put_label.as_str(),
        )
    };
    let to_vault = !matches!(out.keep, crate::secrets::Keep::None);
    if ui.compact {
        crate::compact::sheet(ui, w, h, &mut |ui, x, y, iw| {
            let mut cy = y;
            cy += crate::compact::sheet_head(ui, x, cy, iw, Icon::Lock, WARN, head);
            for (label, value) in [
                ("File", out.name.as_str()),
                ("What it is", out.what.as_str()),
                ("Who can use it", out.gives.as_str()),
            ] {
                cy += crate::compact::kv(ui, x, cy, iw, label, value, TEXT);
            }
            cy += 4.0;
            if to_vault {
                ui.text(x, cy, 13.0, W::S, OK, "Sealed in a vault");
                cy += 24.0;
            }
            match vault.filter(|_| to_vault) {
                Some((name, true)) => {
                    let line = ui.fit(13.0, W::S, &format!("In {name}"), iw);
                    ui.text(x, cy, 13.0, W::S, OK, &line);
                    cy += 30.0;
                }
                Some((name, false)) => {
                    // In place: the sheet's foot holds the Outbox's
                    // button and Cancel.
                    let label = format!("Save into {name}");
                    let label = ui.fit(15.0, W::S, &label, iw - 24.0);
                    ui.button(
                        x,
                        cy,
                        Some(iw),
                        46.0,
                        &label,
                        Style::Primary,
                        Action::SecretVault,
                    );
                    cy += 58.0;
                }
                // Make or unlock one: the sheet waits, and opens again.
                None if to_vault => {
                    if let Some((label, a)) = app.vault_way(app.screen) {
                        let label = ui.fit(15.0, W::S, &label, iw - 24.0);
                        ui.button(x, cy, Some(iw), 46.0, &label, Style::Primary, a);
                        cy += 58.0;
                    }
                }
                None => {}
            }
            ui.text(x, cy, 13.0, W::S, ERR, &unprotected);
            cy += 24.0;
            if forms.len() > 1 {
                cy += wrap_buttons(ui, x, cy, iw, 34.0, &forms) + 4.0;
            }
            ui.checkbox(x, cy + 1.0, app.secret_ack, true);
            let lh = ui.wrap(
                x + 28.0,
                cy,
                iw - 28.0,
                13.0,
                W::R,
                TEXT,
                &out.ack.text(app.medium),
            );
            ui.hit(x - 4.0, cy - 6.0, iw, lh + 12.0, Action::SecretAck);
            cy += lh + 12.0;
            cy += crate::compact::buttons(
                ui,
                x,
                cy,
                iw,
                &[
                    (out_label, out_style, Action::SecretUnprotected),
                    ("Cancel", Style::Ghost, Action::Cancel),
                ],
            );
            cy - y
        });
        return;
    }
    let sw = 600.0f32.min(w - 48.0);
    let (bx, bw) = (32.0, sw - 64.0);
    // The acknowledgement on one line, or on two when longer.
    let ack_h = if ui.measure(13.0, W::R, &out.ack.text(app.medium)) > bw - 28.0 {
        36.0
    } else {
        18.0
    };
    let sh = 452.0
        + ack_h
        + if forms.len() > 1 { 44.0 } else { 0.0 }
        + if to_vault { 0.0 } else { -96.0 };
    let (x, y) = sheet_box(ui, w, h, sw, sh);
    let bx = x + bx;
    let mut cy = y + 26.0;
    ui.icon(bx - 4.0, cy - 2.0, 28.0, Icon::Lock, 14.0, WARN);
    ui.text(bx + 28.0, cy, 18.0, W::S, TEXT, head);
    cy += 38.0;
    for (label, value, face) in [
        ("File", out.name.as_str(), W::M),
        ("What it is", out.what.as_str(), W::R),
        ("Who can use it", out.gives.as_str(), W::R),
    ] {
        ui.text(bx, cy, 12.0, W::R, MUTED, label);
        let used = ui.wrap(bx + 130.0, cy, bw - 130.0, 13.0, face, TEXT, value);
        cy += used.max(20.0) + 10.0;
    }
    cy += 6.0;
    // The vault, first, for what a vault keeps.
    if to_vault {
        ui.text(bx, cy, 13.0, W::S, OK, "Sealed in a vault");
        cy += 26.0;
    }
    match vault.filter(|_| to_vault) {
        Some((name, true)) => {
            ui.text_mid(bx, cy, 42.0, 13.0, W::S, OK, &format!("In {name}"));
            cy += 54.0;
        }
        Some((name, false)) => {
            let label = format!("Save into {name}");
            ui.button(
                bx,
                cy,
                None,
                42.0,
                &label,
                Style::Primary,
                Action::SecretVault,
            );
            cy += 54.0;
        }
        // Make or unlock one: the sheet waits, and opens again.
        None if to_vault => {
            if let Some((label, a)) = app.vault_way(app.screen) {
                ui.button(bx, cy, None, 42.0, &label, Style::Primary, a);
            }
            cy += 54.0;
        }
        None => {}
    }
    if to_vault {
        ui.rule(bx, cy, bw, INNER);
        cy += 16.0;
    }
    // The Outbox, or the stick, only once the person says they
    // understand.
    ui.text(bx, cy, 13.0, W::S, ERR, &unprotected);
    cy += 26.0;
    if forms.len() > 1 {
        cy += wrap_buttons(ui, bx, cy, bw, 34.0, &forms) + 2.0;
    }
    ui.checkbox(bx, cy + 1.0, app.secret_ack, true);
    let lh = ui.wrap(
        bx + 28.0,
        cy,
        bw - 28.0,
        13.0,
        W::R,
        TEXT,
        &out.ack.text(app.medium),
    );
    ui.hit(
        bx - 4.0,
        cy - 6.0,
        bw,
        lh.max(18.0) + 12.0,
        Action::SecretAck,
    );
    cy += lh.max(18.0) + 16.0;
    ui.button(
        bx,
        cy,
        None,
        40.0,
        out_label,
        out_style,
        Action::SecretUnprotected,
    );
    ui.button(
        x + sw - 32.0 - 110.0,
        y + sh - 32.0 - 40.0,
        Some(110.0),
        40.0,
        "Cancel",
        Style::Ghost,
        Action::Cancel,
    );
}
