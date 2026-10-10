//! The small panel (`Faraday::is_compact`): no sidebar. Home is the menu,
//! a button for each of the sidebar's entries, and every other screen has
//! a bar across its top that goes back to it. Flows are a page per step
//! (`flow::column`).

use osk_ui::widgets::Icon;

use crate::ui::pal::*;
use crate::ui::{Style, Ui, W};
use crate::wallet::network_name;
use crate::{Action, Faraday, Screen};

/// The side margin, design units.
pub(crate) const M: f32 = 12.0;
/// The bar across the top of every page but Home.
pub(crate) const BAR_H: f32 = 44.0;

/// The screens that draw their own top, as a step flow does.
pub(crate) fn draws_own_bar(s: Screen) -> bool {
    matches!(
        s,
        Screen::Home
            | Screen::Create
            | Screen::Restore
            | Screen::Backup
            | Screen::Message
            | Screen::Spend
            | Screen::KeyGen
            | Screen::Bip85
            | Screen::Silent
            | Screen::Vanity
            | Screen::CreateVault
            | Screen::Family
    )
}

/// What a page's bar calls it.
pub(crate) fn screen_name(s: Screen, medium: crate::Medium) -> &'static str {
    match s {
        Screen::Home => "Home",
        Screen::Start | Screen::Wallets => "Wallets",
        Screen::Spend | Screen::Family => "Spend",
        Screen::Files => "Files",
        Screen::Visit => medium.visit(),
        Screen::Entry => "Add a key",
        Screen::Backup => "Back up",
        Screen::Message => "Sign a message",
        Screen::CheckMessage => "Check a message",
        Screen::Create => "Create a wallet",
        Screen::Restore => "Load or restore a wallet",
        Screen::Settings => "Settings",
        Screen::Vaults | Screen::Unlock | Screen::VaultContents => "Vaults",
        Screen::Backups => "Backups",
        Screen::CreateVault => "Create a vault",
        Screen::KeyGen => "New key",
        Screen::Bip85 => "BIP-85",
        Screen::Silent => "Silent payments",
        Screen::Explore => "Explore a key",
        Screen::Lightning => "Lightning",
        Screen::Tools | Screen::Catalog => "Tools",
        Screen::Vanity => "Vanity address",
        Screen::Decode => "Transaction",
        Screen::Transfer => "Transfer",
        Screen::Upgrade => medium.upgrade(),
    }
}

/// The bar: a way back on the left, the page's name, and the ? when the
/// page has something in Learn. Returns its height.
pub(crate) fn bar(app: &Faraday, ui: &mut Ui, w: f32, back: (&str, Action), name: &str) -> f32 {
    ui.fill(0.0, 0.0, w, BAR_H, 0.0, SIDEBAR);
    ui.fill(0.0, BAR_H - 1.0, w, 1.0, 0.0, LINE);
    let (label, action) = back;
    if ui.is_pressed(action) {
        ui.fill(4.0, 4.0, BAR_H - 8.0, BAR_H - 8.0, 8.0, INNER);
    }
    ui.icon(4.0, 4.0, BAR_H - 8.0, Icon::ChevronLeft, 13.0, TEXT);
    ui.hit(0.0, 0.0, BAR_H + 8.0, BAR_H, action);
    let right = tools(app, ui, w);
    // The page's own name, and above it, small, where Back goes.
    let x = BAR_H;
    let room = right - x;
    let label = ui.fit(11.0, W::R, label, room);
    ui.text(x, 5.0, 11.0, W::R, MUTED, &label);
    let name = ui.fit(16.0, W::S, name, room);
    ui.text(x, 19.0, 16.0, W::S, TEXT, &name);
    BAR_H
}

/// A step flow's bar, which the flow draws: the ? and the keyboard at its
/// right end, over whatever of its name runs that far.
pub(crate) fn flow_bar_tools(app: &Faraday, ui: &mut Ui, w: f32) {
    let right = tools(app, ui, -w);
    if right < w - M {
        ui.fill(right, 0.0, w - right, BAR_H - 1.0, 0.0, SIDEBAR);
        tools(app, ui, w);
    }
}

/// The ? when the page has something in Learn, and the keyboard where
/// the page types, at the bar's right end. Returns where they begin.
/// A negative `w` measures without drawing.
fn tools(app: &Faraday, ui: &mut Ui, w: f32) -> f32 {
    let draw = w > 0.0;
    let w = w.abs();
    let mut right = if app.learn_pages().is_empty() {
        w - M
    } else {
        if draw {
            help(ui, w - M - 30.0, (BAR_H - 30.0) / 2.0);
        }
        w - M - 38.0
    };
    // The keyboard, where the page types, and to bring it back once put
    // away from a field.
    if app.osk_offered() || app.osk_shown() || app.osk_auto() {
        let shown = app.osk_shown();
        let action = Action::Osk(if shown {
            OskPress::Hide
        } else {
            OskPress::Show
        });
        let kx = right - 34.0;
        let ky = (BAR_H - 34.0) / 2.0;
        right = kx - 6.0;
        if !draw {
            return right;
        }
        if shown || ui.is_pressed(action) {
            ui.fill(
                kx,
                ky,
                34.0,
                34.0,
                8.0,
                if shown { ACCENT.with_alpha(40) } else { INNER },
            );
        }
        ui.icon(
            kx,
            ky,
            34.0,
            Icon::Keyboard,
            14.0,
            if shown { ACCENT } else { MUTED },
        );
        ui.hit(kx - 4.0, 0.0, 42.0, BAR_H, action);
    }
    right
}

/// The ? that opens what explains the page.
fn help(ui: &mut Ui, x: f32, y: f32) {
    let pressed = ui.is_pressed(Action::Learn);
    ui.fill(
        x,
        y,
        30.0,
        30.0,
        15.0,
        if pressed { INNER } else { SURFACE },
    );
    ui.stroke(x, y, 30.0, 30.0, 15.0, BORDER);
    let tw = ui.measure(14.0, W::S, "?");
    ui.text_mid(x + 15.0 - tw / 2.0, y, 30.0, 14.0, W::S, MUTED, "?");
    ui.hit(x - 6.0, y - 6.0, 42.0, 42.0, Action::Learn);
}

/// A page that is not one of the step flows: its bar, then the page
/// under it as the desktop draws it, in the panel's width.
pub(crate) fn page(app: &mut Faraday, ui: &mut Ui, w: f32, h: f32) -> (f32, f32) {
    let name = match app.session.wallets.get(app.wallet) {
        Some(w) if app.screen == Screen::Wallets => w.name.as_str(),
        _ => screen_name(app.screen, app.medium),
    };
    let back = match app.screen {
        Screen::Wallets | Screen::Explore | Screen::Entry | Screen::CheckMessage => {
            ("Wallets", Action::Nav(Screen::Start))
        }
        // An item of a vault goes back to the vault's list.
        Screen::VaultContents if crate::vault_screens::contents_detail(app) => (
            "Vault contents",
            Action::Vault(crate::vaults::VaultAction::ItemBack),
        ),
        Screen::Unlock => crate::vault_screens::unlock_back(app),
        Screen::VaultContents => ("Vaults", Action::Nav(Screen::Vaults)),
        // Made from another flow: the way back to it.
        Screen::Vaults if let Some(s) = app.vaults.back_to => (
            app.back_link_name(s),
            Action::Vault(crate::vaults::VaultAction::Back),
        ),
        // Decode goes back where it was opened from.
        Screen::Decode => match app.decode.as_ref() {
            Some(d) => (crate::screens::decode_back(d.back), Action::Nav(d.back)),
            None => ("Tools", Action::Nav(Screen::Catalog)),
        },
        Screen::Tools | Screen::Lightning => ("Tools", Action::Nav(Screen::Catalog)),
        Screen::Upgrade => ("Settings", Action::Nav(Screen::Settings)),
        _ => ("Home", Action::Nav(Screen::Home)),
    };
    bar(app, ui, w, back, name);
    (BAR_H, h - BAR_H)
}

/// The height Scan's floating button takes at the foot of Home (its own
/// height, the margin under it and a gap above it), so the scrollable
/// list can be clipped above it.
const FAB_ROOM: f32 = M + 40.0 + 8.0;

/// Home: what is loaded, then a button for each place the sidebar goes.
pub(crate) fn home(app: &Faraday, ui: &mut Ui, w: f32, h: f32) {
    // Scan floats over the foot in a strip of its own: the scrollable
    // list is clipped above it, so no tile's hit area ever falls under
    // it, at any scroll position, not only at the bottom.
    let view_h = (h - FAB_ROOM).max(0.0);
    let clip = ui.rect(0.0, 0.0, w, view_h);
    ui.c.push_clip(clip);
    let top = -app.list_offset;
    let mut y = top + 12.0;
    let inner = w - 2.0 * M;

    // The mark and the name; off mainnet, a badge on the right that
    // opens nothing. The chooser is in Settings.
    ui.mark(M, y, 20.0, 23.0);
    ui.text_mid(M + 28.0, y, 24.0, 17.0, W::S, TEXT, "Faraday");
    let net = app.session.network();
    if !net.is_mainnet() {
        let label = network_name(net);
        let pw = ui.measure(12.0, W::S, label) + 24.0;
        let px = w - M - pw;
        ui.fill(px, y - 2.0, pw, 28.0, 14.0, WARN.with_alpha(30));
        ui.stroke(px, y - 2.0, pw, 28.0, 14.0, WARN.with_alpha(110));
        ui.text_mid(px + 12.0, y - 2.0, 28.0, 12.0, W::S, WARN, label);
    }
    y += 38.0;

    // What this session holds.
    let keys = app.session.keys.len();
    let wallets = app.session.wallets.len();
    let vaults = app.vaults.open.len();
    let (dot, line) = match (keys + wallets, vaults) {
        (0, 0) => (DIM, "Nothing loaded".to_string()),
        (_, 0) => (
            OK,
            format!(
                "{keys} {} · {wallets} {}",
                if keys == 1 { "key" } else { "keys" },
                if wallets == 1 { "wallet" } else { "wallets" }
            ),
        ),
        (_, v) => (
            OK,
            format!("{v} {} open", if v == 1 { "vault" } else { "vaults" }),
        ),
    };
    let sub = app.home_files_line();
    let session = if keys + wallets == 0 {
        Screen::Start
    } else {
        Screen::Wallets
    };
    let action = Action::Nav(session);
    let pressed = ui.is_pressed(action);
    ui.fill(
        M,
        y,
        inner,
        52.0,
        10.0,
        if pressed { INNER } else { SURFACE },
    );
    ui.stroke(M, y, inner, 52.0, 10.0, LINE);
    ui.dot(M + 16.0, y + 18.0, 4.0, dot);
    let line = ui.fit(14.0, W::S, &line, inner - 40.0);
    ui.text_mid(M + 28.0, y + 6.0, 24.0, 14.0, W::S, TEXT, &line);
    let sub = ui.fit(12.0, W::R, &sub, inner - 40.0);
    ui.text_mid(M + 28.0, y + 26.0, 20.0, 12.0, W::R, MUTED, &sub);
    ui.hit(M, y, inner, 52.0, action);
    y += 52.0 + 10.0;

    // The session strip: the four stages of the lock cycle, the one now
    // current in the accent, the rest dim. Tapping it opens Files.
    {
        let stage = app.session_stage();
        let open_files = Action::Nav(Screen::Files);
        if ui.is_pressed(open_files) {
            ui.fill(M, y, inner, 22.0, 6.0, INNER);
        }
        let mut sx = M + 4.0;
        for (i, label) in ["Bring in", "Open", "Work", "Write out"]
            .into_iter()
            .enumerate()
        {
            if i > 0 {
                sx += ui.text_mid(sx, y, 22.0, 11.0, W::R, DIM, " · ");
            }
            let fg = if label == stage { ACCENT } else { DIM };
            sx += ui.text_mid(sx, y, 22.0, 11.0, W::S, fg, label);
        }
        ui.hit(M, y, inner, 22.0, open_files);
        y += 22.0 + 10.0;
    }

    // The one thing waiting, when there is one.
    for (icon, label, action, tone) in prompts(app) {
        let pressed = action.is_some_and(|a| ui.is_pressed(a));
        let edge = if tone == WARN {
            WARN.with_alpha(110)
        } else {
            ACCENT.with_alpha(110)
        };
        ui.fill(
            M,
            y,
            inner,
            44.0,
            10.0,
            if pressed { INNER } else { tone.with_alpha(26) },
        );
        ui.stroke(M, y, inner, 44.0, 10.0, edge);
        ui.icon(M + 8.0, y + 8.0, 28.0, icon, 14.0, tone);
        let label = ui.fit(14.0, W::S, &label, inner - 60.0);
        ui.text_mid(M + 44.0, y, 44.0, 14.0, W::S, TEXT, &label);
        if let Some(a) = action {
            ui.icon(
                M + inner - 26.0,
                y + 12.0,
                20.0,
                Icon::ChevronRight,
                9.0,
                DIM,
            );
            ui.hit(M, y, inner, 44.0, a);
        }
        y += 44.0 + 8.0;
    }
    y += 2.0;

    // The menu: the sidebar's places, two to a row.
    let gap = 8.0;
    let tw = (inner - gap) / 2.0;
    let th = 76.0;
    let tiles = tiles(app);
    for (i, (icon, label, sub, action, enabled)) in tiles.iter().enumerate() {
        let tx = M + (i % 2) as f32 * (tw + gap);
        let ty = y + (i / 2) as f32 * (th + gap);
        let pressed = ui.is_pressed(*action);
        ui.fill(tx, ty, tw, th, 12.0, if pressed { INNER } else { SURFACE });
        ui.stroke(tx, ty, tw, th, 12.0, LINE);
        let (ibg, ifg) = if *enabled {
            (ACCENT.with_alpha(26), ACCENT)
        } else {
            (INNER, DIM)
        };
        ui.fill(tx + 12.0, ty + 12.0, 30.0, 30.0, 8.0, ibg);
        ui.icon(tx + 12.0, ty + 12.0, 30.0, *icon, 14.0, ifg);
        let label = ui.fit(14.0, W::S, label, tw - 24.0);
        ui.text(
            tx + 12.0,
            ty + 48.0,
            14.0,
            W::S,
            if *enabled { TEXT } else { DIM },
            &label,
        );
        if !sub.is_empty() {
            let sub = ui.fit(11.0, W::R, sub, tw - 54.0);
            ui.text_right(
                tx + tw - 10.0,
                ty + 12.0,
                30.0,
                11.0,
                W::R,
                if *enabled { MUTED } else { WARN },
                &sub,
            );
        }
        if *enabled {
            ui.hit(tx, ty, tw, th, *action);
        }
    }
    y += tiles.len().div_ceil(2) as f32 * (th + gap) + 4.0;

    // Lock, when something is held, and power.
    let pw = 44.0;
    if app.holds_secret() {
        ui.button(
            M,
            y,
            Some(inner - pw - gap),
            44.0,
            "Lock",
            Style::Secondary,
            Action::LockAsk,
        );
    }
    let px = M + inner - pw;
    if ui.is_pressed(Action::PowerAsk) {
        ui.fill(px, y, pw, 44.0, 10.0, INNER);
    }
    ui.stroke(px, y, pw, 44.0, 10.0, BORDER);
    ui.icon(px, y, pw, Icon::Power, 15.0, MUTED);
    ui.hit(px, y, pw, 44.0, Action::PowerAsk);
    y += 44.0 + 16.0;

    let content = y - top;
    app.content_h.set(content);
    ui.report_scroll(clip, content - view_h);
    ui.c.pop_clip();

    ui.fab(w - M, h - M, Icon::Scan, "Scan", Action::Scan);
}

/// A tile: its icon, its name, a short count, where it goes, and
/// whether it can be pressed now.
type Tile = (Icon, &'static str, String, Action, bool);

fn tiles(app: &Faraday) -> Vec<Tile> {
    let wallets = app.session.wallets.len();
    let vault_files = app.vault_files().len();
    let open = app.vaults.open.len();
    // In two columns: wallets and keys first (every wallet flow,
    // including spending: `docs/SIMPLIFY.md` §6.2 moves Spend to the
    // Wallets empty state and the Learn sheet), then the ways in and
    // out, then what is used now and then.
    let mut t: Vec<Tile> = vec![
        (
            Icon::Wallet,
            "Wallets",
            if wallets == 0 {
                String::new()
            } else {
                wallets.to_string()
            },
            Action::Nav(Screen::Start),
            true,
        ),
        (
            Icon::Lock,
            "Vaults",
            match (open, vault_files) {
                (0, 0) => String::new(),
                (0, n) => format!("{n} locked"),
                (o, _) => format!("{o} open"),
            },
            Action::Nav(Screen::Vaults),
            true,
        ),
        (
            Icon::File,
            "Files",
            format!("{} · {}", app.inbox.len(), app.outbox.len()),
            Action::Nav(Screen::Files),
            true,
        ),
        (Icon::Learn, "Learn", String::new(), Action::Learn, true),
    ];
    // Backups once any wallet is known (§5), after Wallets.
    let known = app.backups();
    if !known.is_empty() {
        let without = known.iter().filter(|e| e.lines.is_none()).count();
        t.insert(
            2,
            (
                Icon::Shield,
                "Backups",
                if without == 0 {
                    String::new()
                } else {
                    format!("{without} not backed up")
                },
                Action::Nav(Screen::Backups),
                true,
            ),
        );
    }
    if !app.sticks.is_empty() && !app.holds_secret() {
        t.push((
            app.medium.icon(),
            app.medium.visit(),
            String::new(),
            Action::Nav(Screen::Visit),
            true,
        ));
    }
    t.push((
        Icon::Tools,
        "Tools",
        String::new(),
        Action::Nav(Screen::Catalog),
        true,
    ));
    // The online app is the device's QR link.
    if app.online {
        t.push((
            Icon::Qr,
            "Transfer",
            String::new(),
            Action::Nav(Screen::Transfer),
            true,
        ));
    }
    t.push((
        Icon::Settings,
        "Settings",
        String::new(),
        Action::Nav(Screen::Settings),
        true,
    ));
    t
}

/// What Home puts above the menu: the same one lead job §1.2 ranks for
/// the wide Home (`crate::screens::home_lead`), as one prompt row.
fn prompts(app: &Faraday) -> Vec<(Icon, String, Option<Action>, osk_ui::Color)> {
    let (icon, label, _sub, action, enabled) = crate::screens::home_lead(app);
    let tone = if enabled { ACCENT } else { WARN };
    vec![(icon, label, enabled.then_some(action), tone)]
}

/// A sheet on a small panel: the panel's width less a margin, as tall as
/// what `body` draws (measured first, drawing nothing), centred. `body`
/// draws at (x, y) in a width and returns the height it took.
pub(crate) fn sheet(
    ui: &mut Ui,
    w: f32,
    h: f32,
    body: &mut dyn FnMut(&mut Ui, f32, f32, f32) -> f32,
) {
    scroll_sheet(ui, (8.0, w - 16.0), h, 8.0, 16.0, body);
}

/// A sheet `sw` wide at `sx`, as tall as what `body` draws and at most
/// `h` less `margin` above and below, centred, with `pad` inside: what is
/// above its buttons scrolls when it is taller. [`sheet`] on a small
/// panel; a long sheet on the desktop too.
pub(crate) fn scroll_sheet(
    ui: &mut Ui,
    (sx, sw): (f32, f32),
    h: f32,
    margin: f32,
    pad: f32,
    body: &mut dyn FnMut(&mut Ui, f32, f32, f32) -> f32,
) {
    let (ix, iw) = (sx + pad, sw - 2.0 * pad);
    // The sheet's buttons ([`buttons`]) are kept at its foot, and what is
    // above them scrolls when the sheet is taller than the panel.
    let mark = ui.hits.len();
    ui.c.push_clip(osk_ui::Rect::new(0, 0, 0, 0));
    ui.sheet_pin = None;
    ui.sheet_pinning = true;
    let ch = body(ui, ix, 0.0, iw);
    let pins = ui.sheet_pin.take();
    let items: Vec<(&str, Style, Action)> = pins
        .iter()
        .flatten()
        .map(|(l, s, a)| (l.as_str(), *s, *a))
        .collect();
    ui.sheet_pinning = false;
    let bh = buttons(ui, ix, 0.0, iw, &items);
    ui.c.pop_clip();
    ui.hits.truncate(mark);
    let foot = if items.is_empty() { 0.0 } else { bh + pad };
    let sh = (ch + 2.0 * pad + foot).min(h - 2.0 * margin);
    let sy = ((h - sh) / 2.0).max(margin);
    if let Some(a) = ui.outside {
        ui.hit_around(sx, sy, sw, sh, a);
    }
    ui.shadow(sx, sy, sw, sh, 14.0);
    ui.fill(sx, sy, sw, sh, 14.0, SURFACE);
    ui.stroke(sx, sy, sw, sh, 14.0, BORDER);
    // Taller than the panel: what is above the buttons scrolls.
    let view = sh - foot;
    let max = (ch + 2.0 * pad - view).max(0.0);
    let off = ui.offset.clamp(0.0, max);
    let clip = ui.rect(sx, sy, sw, view);
    ui.c.push_clip(clip);
    ui.sheet_pinning = true;
    body(ui, ix, sy + pad - off, iw);
    ui.sheet_pinning = false;
    ui.sheet_pin = None;
    ui.c.pop_clip();
    ui.report_scroll(clip, max);
    if !items.is_empty() {
        if max > 0.0 {
            ui.fill(sx + 1.0, sy + view, sw - 2.0, 1.0, 0.0, LINE);
        }
        buttons(ui, ix, sy + view, iw, &items);
    }
}

/// A sheet's title: its icon, and the words wrapped beside it. Returns
/// its height.
pub(crate) fn sheet_head(
    ui: &mut Ui,
    x: f32,
    y: f32,
    w: f32,
    icon: Icon,
    tone: osk_ui::Color,
    title: &str,
) -> f32 {
    ui.icon(x - 4.0, y - 3.0, 28.0, icon, 15.0, tone);
    let th = ui.wrap(x + 28.0, y, w - 28.0, 17.0, W::S, TEXT, title);
    th.max(22.0) + 10.0
}

/// A label over its value, the value wrapped. Returns its height.
pub(crate) fn kv(
    ui: &mut Ui,
    x: f32,
    y: f32,
    w: f32,
    key: &str,
    value: &str,
    tone: osk_ui::Color,
) -> f32 {
    ui.text(x, y, 12.0, W::R, MUTED, key);
    let vh = ui.wrap(x, y + 18.0, w, 14.0, W::R, tone, value);
    ui.rule(x, y + 24.0 + vh, w, INNER);
    vh + 32.0
}

/// A row of buttons: side by side when every label fits its share, one
/// under another, full width, when not. Returns their height.
pub(crate) fn buttons(ui: &mut Ui, x: f32, y: f32, w: f32, items: &[(&str, Style, Action)]) -> f32 {
    if items.is_empty() {
        return 0.0;
    }
    // Inside a sheet, they go to its foot ([`sheet`]).
    if ui.sheet_pinning {
        ui.sheet_pin = Some(
            items
                .iter()
                .map(|&(l, s, a)| (l.to_string(), s, a))
                .collect(),
        );
        return 0.0;
    }
    let n = items.len() as f32;
    let share = (w - (n - 1.0) * 8.0) / n;
    let fits = items
        .iter()
        .all(|(l, ..)| ui.measure(15.0, W::S, l) + 24.0 <= share);
    if fits {
        for (i, &(label, style, action)) in items.iter().enumerate() {
            ui.button(
                x + i as f32 * (share + 8.0),
                y,
                Some(share),
                46.0,
                label,
                style,
                action,
            );
        }
        46.0
    } else {
        for (i, &(label, style, action)) in items.iter().enumerate() {
            ui.button(x, y + i as f32 * 54.0, Some(w), 46.0, label, style, action);
        }
        n * 54.0 - 8.0
    }
}

/// The sheets that say what a lock does: a title, a line, rows of what
/// happens to what, and their buttons.
pub(crate) fn kv_sheet(
    ui: &mut Ui,
    w: f32,
    h: f32,
    head: (Icon, osk_ui::Color, &str),
    sub: &str,
    rows: &[(&str, String, osk_ui::Color)],
    actions: &[(&str, Style, Action)],
) {
    sheet(ui, w, h, &mut |ui, x, y, iw| {
        let mut cy = y;
        cy += sheet_head(ui, x, cy, iw, head.0, head.1, head.2);
        if !sub.is_empty() {
            cy += ui.wrap(x, cy, iw, 13.0, W::R, MUTED, sub) + 12.0;
        }
        for (k, v, tone) in rows {
            cy += kv(ui, x, cy, iw, k, v, *tone);
        }
        cy += 8.0;
        cy += buttons(ui, x, cy, iw, actions);
        cy - y
    });
}

/// A press on the docked keyboard, or the bar's keyboard button.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OskPress {
    /// A character key.
    Char(char),
    /// Backspace.
    Back,
    /// Done: Enter.
    Done,
    /// Shift.
    Shift,
    /// The symbol layer.
    Symbols,
    /// Bring the keyboard up.
    Show,
    /// Put it away.
    Hide,
}

/// The docked keyboard: OpenSigner's own, at the foot of a small panel
/// while something on the page takes typing. A press is the key a
/// physical keyboard would send.
#[derive(Debug, Default)]
pub(crate) struct Osk {
    shift: bool,
    symbols: bool,
    /// Brought up from the bar on a page that types with no field.
    manual: bool,
    /// Put away while a field has focus.
    hidden: bool,
    /// Which field had focus on the last frame, when one did.
    was_field: Option<String>,
}

/// One row of keys, design units.
const KEY_ROW: f32 = 32.0;

impl Faraday {
    /// A field with focus takes typing now.
    fn osk_auto(&self) -> bool {
        if self.sheet == Some(crate::Sheet::Potential) {
            return self.potential.as_ref().is_some_and(|p| p.typing);
        }
        if self.sheet == Some(crate::Sheet::Import) {
            return self.vaults.focus == Some(crate::vaults::Focus::Passphrase);
        }
        if self.sheet.is_some() {
            return false;
        }
        match self.screen {
            Screen::Entry => {
                self.entry.on_passphrase
                    || self.entry.form != crate::forms::Form::Words
                    || self.entry.keys.is_none()
            }
            // The vault's list has no field; its item page may.
            Screen::VaultContents => {
                self.vaults.focus.is_some() && crate::vault_screens::contents_detail(self)
            }
            Screen::Vanity => self
                .vanity
                .as_ref()
                .is_some_and(|v| v.open == Some(crate::vanity::vstep::PREFIX) && !v.running),
            _ => self.typing_field(),
        }
    }

    /// A page that takes typing with no field to focus: its bar offers
    /// the keyboard.
    pub(crate) fn osk_offered(&self) -> bool {
        self.sheet.is_none()
            && matches!(
                self.screen,
                Screen::Explore
                    | Screen::Bip85
                    | Screen::Catalog
                    | Screen::Tools
                    | Screen::Lightning
            )
    }

    /// Whether the keyboard is up.
    pub(crate) fn osk_shown(&self) -> bool {
        self.compact
            && ((self.osk_auto() && !self.osk.hidden) || (self.osk.manual && self.osk_offered()))
    }

    /// Which of OpenSigner's keyboards: the path pad for a derivation
    /// path, else the passphrase keyboard, which types any printable
    /// character.
    fn osk_kind(&self) -> osk_ui::widgets::keyboard::KeyboardKind {
        use osk_ui::widgets::keyboard::KeyboardKind as K;
        match self.screen {
            Screen::Explore if self.sheet.is_none() => K::Path,
            _ => K::Passphrase,
        }
    }

    /// The keyboard's height when it is up, else nothing.
    pub(crate) fn osk_height(&self) -> f32 {
        if self.osk_shown() {
            osk_ui::widgets::keyboard::rows(self.osk_kind()) as f32 * KEY_ROW + 8.0
        } else {
            0.0
        }
    }

    /// Which field typing goes to, as far as telling one from another:
    /// none when no field takes typing.
    fn osk_field(&self) -> Option<String> {
        self.osk_auto().then(|| {
            format!(
                "{:?} {:?} {:?} {} {:?} {:?}",
                self.screen,
                self.sheet,
                self.vaults.focus,
                self.entry.on_passphrase,
                self.keygen.as_ref().and_then(|k| k.focus),
                self.create.as_ref().and_then(|c| c.pass_focus)
            )
        })
    }

    /// A field newly focused, or another field than before, brings the
    /// keyboard up again. Returns whether it came up on this frame.
    pub(crate) fn osk_track(&mut self) -> bool {
        let field = self.osk_field();
        let rose = field.is_some() && field != self.osk.was_field;
        if rose {
            self.osk.hidden = false;
        }
        self.osk.was_field = field;
        rose
    }

    /// The keyboard just came up over the panel's foot: when the press
    /// that focused the field was under where the keyboard now is, the
    /// page scrolls that far, so the field stays in view. `f` is pixels a
    /// design unit; `visible` the height left above the keyboard.
    pub(crate) fn osk_reveal(&mut self, f: f32, visible: f32) {
        let tapped = self.down_at.1 as f32 / f.max(0.01);
        let over = tapped + 56.0 - visible;
        if over > 0.0
            && let Some(slot) = self.scroll_slot()
        {
            *slot += over;
            self.dirty = true;
            self.commands.push_back(osk_shell_api::Command::Draw);
        }
    }

    /// Leaving a page puts away a keyboard brought up by hand.
    pub(crate) fn osk_leave(&mut self) {
        self.osk.manual = false;
        self.osk.shift = false;
        self.osk.symbols = false;
    }

    pub(crate) fn osk_press(&mut self, p: OskPress) {
        use osk_shell_api::Key as K;
        match p {
            OskPress::Char(c) => {
                self.key(K::Char(c));
                if c.is_alphabetic() {
                    self.osk.shift = false;
                }
            }
            OskPress::Back => self.key(K::Backspace),
            // Done is Enter, and puts the keyboard away, so the page's
            // pinned action shows again; another field brings it back.
            OskPress::Done => {
                self.key(K::Enter);
                self.osk.hidden = true;
                self.osk.manual = false;
            }
            OskPress::Shift => self.osk.shift = !self.osk.shift,
            OskPress::Symbols => self.osk.symbols = !self.osk.symbols,
            OskPress::Show => {
                self.osk.manual = true;
                self.osk.hidden = false;
            }
            OskPress::Hide => {
                self.osk.manual = false;
                self.osk.hidden = true;
            }
        }
    }
}

/// The keyboard, across the foot of the panel from `y`.
pub(crate) fn keyboard(app: &Faraday, ui: &mut Ui, w: f32, y: f32) {
    use osk_ui::widgets::keyboard::{self, KeyInput, Modifiers};
    let kind = app.osk_kind();
    let rows = keyboard::rows(kind) as f32;
    ui.fill(0.0, y, w, rows * KEY_ROW + 8.0, 0.0, SIDEBAR);
    ui.fill(0.0, y, w, 1.0, 0.0, LINE);
    let area = osk_ui::geom::Rect::new(
        4,
        (y + 6.0) as i32,
        (w - 4.0) as i32,
        (rows * KEY_ROW) as i32,
    );
    let ctx = osk_ui::layout::LayoutCtx::new(
        osk_ui::geom::Scale::new(160),
        osk_ui::geom::SizeClass::Small,
    );
    let mods = Modifiers {
        shift: app.osk.shift,
        symbols: app.osk.symbols,
    };
    for cap in keyboard::keys(kind, area, &ctx, keyboard::ALL_KEYS, None, mods) {
        let r = cap.rect;
        let (kx, ky, kw, kh) = (r.x as f32, r.y as f32, r.w as f32 - 3.0, r.h as f32 - 3.0);
        let (label, style, press) = match cap.input {
            KeyInput::Char(' ') => (String::new(), Style::Secondary, OskPress::Char(' ')),
            KeyInput::Char(c) => (c.to_string(), Style::Secondary, OskPress::Char(c)),
            KeyInput::Backspace => (String::new(), Style::Secondary, OskPress::Back),
            KeyInput::Done => ("Done".to_string(), Style::Primary, OskPress::Done),
            KeyInput::Shift => (
                "⇧".to_string(),
                if app.osk.shift {
                    Style::Primary
                } else {
                    Style::Secondary
                },
                OskPress::Shift,
            ),
            KeyInput::Symbols => (
                if app.osk.symbols { "abc" } else { "#+=" }.to_string(),
                Style::Secondary,
                OskPress::Symbols,
            ),
        };
        let style = if cap.enabled { style } else { Style::Disabled };
        ui.button(kx, ky, Some(kw), kh, &label, style, Action::Osk(press));
        match cap.input {
            KeyInput::Backspace => ui.icon(
                kx + kw / 2.0 - 9.0,
                ky + kh / 2.0 - 9.0,
                18.0,
                Icon::Delete,
                12.0,
                TEXT,
            ),
            KeyInput::Char(' ') => ui.icon(
                kx + kw / 2.0 - 9.0,
                ky + kh / 2.0 - 9.0,
                18.0,
                Icon::Space,
                12.0,
                MUTED,
            ),
            _ => {}
        }
    }
}

impl Faraday {
    /// Files by direction, as the sidebar's Files row reads it: "3 for
    /// the stick" while any wait, else "20 from the stick", else "none".
    pub fn files_count(&self) -> String {
        if !self.outbox.is_empty() {
            format!("{} {}", self.outbox.len(), self.medium.for_the())
        } else if !self.inbox.is_empty() {
            format!("{} {}", self.inbox.len(), self.medium.from_the())
        } else {
            "none".to_string()
        }
    }

    /// Home's status line on a small panel: Files by direction, and
    /// what is attached, by its label or by how many.
    pub fn home_files_line(&self) -> String {
        let sticks = match self.sticks.len() {
            0 => format!("no {}", self.medium.noun()),
            1 => self.sticks[0].label.clone(),
            k => format!("{k} {}", self.medium.nouns()),
        };
        format!("{} · {sticks}", self.files_count())
    }
}
