//! The Spend tab, drawn: one step-card column (`flow.rs`) of the tab's
//! pages and the spend's own steps (`docs/FAMILY.md`). Each card's lead
//! is always shown; its More about this is folded.

use osk_ui::widgets::Icon;

use crate::family::{CardId, FamilyAction as F, Route, more_bit, page};
use crate::family_text as text;
use crate::screens::{next_button, step_body, step_summary};
use crate::ui::pal::*;
use crate::ui::{Style, Ui, W, grouped};
use crate::vaults::{VaultAction, cost_text, file_text, mib_text};
use crate::wallet::{FileKind, Kind, Session, fp_text, step};
use crate::{Action, Faraday, flow};

fn fa(a: F) -> Action {
    Action::Family(a)
}

pub(crate) fn draw(app: &mut Faraday, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    if app.family_overview() {
        let content = loaded(app, ui, x0, cw);
        // Scrolled no further than its end.
        let max = (content - h).max(0.0);
        if app.family.scroll.y > max {
            app.family.scroll.y = max;
            app.dirty = true;
            app.commands.push_back(osk_shell_api::Command::Draw);
        }
        return;
    }
    let col_x = x0 + 40.0;
    let col_w = (cw - 80.0).min(900.0);
    let ids = app.family_cards();
    let open = app.family_open_card();
    let cards: Vec<flow::Card> = ids
        .iter()
        .map(|&id| flow::Card {
            title: text::title(app, id),
            summary: summary(app, id),
            mono: matches!(id, CardId::Step(step::TXID)),
            done: done(app, id),
            open: open == Some(id),
            toggle: crate::family::toggle(id),
            guide: Some(text::lead(app, id)).filter(|s| !s.is_empty()),
        })
        .collect();
    let col = flow::Column {
        area_x: x0,
        area_w: cw,
        x: col_x,
        w: col_w,
        h,
        back: None,
        heading: "Spend",
        guided: true,
        switch: false,
        note: Some(
            "This page takes someone spending for the first time through every step. With a \
             wallet or a seed already loaded, Spend opens on a shorter page instead: what is \
             loaded and what each wallet needs next.",
        ),
    };
    let scroll = app.family.scroll;
    let (next, again) = {
        let app_ref: &Faraday = app;
        let mut draw_body = |ui: &mut Ui, i: usize, x: f32, y: f32, w: f32| -> f32 {
            body(app_ref, ui, ids[i], x, y, w)
        };
        flow::column(ui, &col, &cards, scroll, &mut draw_body)
    };
    app.family.scroll = next;
    if again {
        app.dirty = true;
        app.commands.push_back(osk_shell_api::Command::Draw);
    }
}

/// The tab with something loaded already: each wallet and what it needs
/// next, seeds no wallet uses, and the walk-through one press away.
fn loaded(app: &Faraday, ui: &mut Ui, x0: f32, cw: f32) -> f32 {
    let x = x0 + 48.0;
    let w = (cw - 96.0).min(980.0);
    let scrolled = app.family.scroll.y;
    let mut y = 36.0 - scrolled;
    crate::screens::title(ui, x, y, "Spend");
    let lw = ui.measure(13.0, W::S, "Walk through from the start") + 32.0;
    ui.button(
        x + w - lw,
        y,
        Some(lw),
        36.0,
        "Walk through from the start",
        Style::Ghost,
        fa(F::Walkthrough),
    );
    y += 64.0;

    // A transaction being signed comes first.
    if let Some(s) = app.spend.as_ref() {
        let name = s
            .wallet
            .and_then(|i| app.session.wallets.get(i))
            .map_or("no wallet loaded".to_string(), |w| w.name.clone());
        y += next_row(
            ui,
            x,
            y,
            w,
            Icon::Sign,
            &format!("Signing {} · {name}", s.spend.source),
            Some((
                "Continue signing",
                s.wallet
                    .map_or(Action::Nav(crate::Screen::Spend), |i| fa(F::SpendFrom(i))),
            )),
        );
    }

    crate::screens::section_label(ui, x, y, "Wallets loaded");
    y += 30.0;
    if app.session.wallets.is_empty() {
        ui.text_mid(x, y, 30.0, 13.0, W::R, DIM, "None loaded");
        y += 42.0;
    }
    for (i, wl) in app.session.wallets.iter().enumerate() {
        let slots = app.session.slots(wl);
        let here = slots.iter().filter(|s| s.held_by.is_some()).count();
        let need = crate::wallet::needed(wl);
        let ready = here >= need;
        let ch = if ready { 64.0 } else { 108.0 };
        ui.card(x, y, w, ch, if ready { OK.with_alpha(90) } else { LINE });
        ui.icon(x + 18.0, y + 14.0, 20.0, Icon::Wallet, 12.0, MUTED);
        let nw = ui.text_mid(x + 46.0, y + 4.0, 40.0, 15.0, W::S, TEXT, &wl.name);
        ui.text_mid(
            x + 60.0 + nw,
            y + 4.0,
            40.0,
            12.0,
            W::R,
            MUTED,
            &Session::shape(wl),
        );
        let (marker, tone) = if ready {
            (format!("Ready to sign · {} of {need}", here.min(need)), OK)
        } else if here > 0 {
            (
                format!(
                    "{here} {} here · {need} needed",
                    if here == 1 { "key" } else { "keys" }
                ),
                WARN,
            )
        } else {
            ("No key here".to_string(), DIM)
        };
        ui.text_mid(x + 46.0, y + 30.0, 28.0, 12.0, W::S, tone, &marker);
        let label = if ready { "Spend from it" } else { "Open it" };
        let bw = ui.measure(14.0, W::S, label) + 36.0;
        ui.button(
            x + w - 16.0 - bw,
            y + 12.0,
            Some(bw),
            40.0,
            label,
            if ready {
                Style::Primary
            } else {
                Style::Secondary
            },
            if ready {
                fa(F::SpendFrom(i))
            } else {
                Action::OpenWallet(i)
            },
        );
        if !ready {
            // The keys still missing: where they may be.
            let used = crate::screens::missing_keys_line(
                app,
                ui,
                &slots,
                crate::Screen::Family,
                x + 46.0,
                y + 60.0,
                w - 62.0,
            );
            if used == 0.0 {
                ui.text_mid(
                    x + 46.0,
                    y + 62.0,
                    40.0,
                    13.0,
                    W::R,
                    MUTED,
                    "An open vault holds a missing key · Open it to load it",
                );
            }
        }
        y += ch + 10.0;
    }

    // Seeds that open no wallet loaded.
    let lonely: Vec<&crate::wallet::Key> = app
        .session
        .keys
        .iter()
        .filter(|k| {
            let fp = k.master.fingerprint();
            !app.session.wallets.iter().any(|w| {
                app.session
                    .slots(w)
                    .iter()
                    .any(|s| s.fingerprint == Some(fp))
            })
        })
        .collect();
    if !lonely.is_empty() {
        y += 8.0;
        crate::screens::section_label(ui, x, y, "Seeds with no wallet loaded");
        y += 30.0;
        for k in &lonely {
            ui.text_mid(
                x,
                y,
                30.0,
                14.0,
                W::M,
                TEXT,
                &fp_text(k.master.fingerprint()),
            );
            ui.text_mid(x + 100.0, y, 30.0, 13.0, W::R, MUTED, &k.label);
            y += 32.0;
        }
        let file = app.inbox.iter().position(|i| i.kind == FileKind::Wallet);
        let mut bx = x;
        if let Some(k) = file {
            bx += ui.button(
                bx,
                y + 4.0,
                None,
                38.0,
                &format!("Open {}", app.inbox[k].name),
                Style::Primary,
                fa(F::UseWallet(k)),
            ) + 8.0;
        }
        bx += ui.button(
            bx,
            y + 4.0,
            None,
            38.0,
            "Restore its wallet",
            Style::Secondary,
            Action::RestoreWallet,
        ) + 8.0;
        ui.button(
            bx,
            y + 4.0,
            None,
            38.0,
            "Spend as a single-key wallet",
            Style::Secondary,
            fa(F::SeedWallet),
        );
        y += 50.0;
    }
    y + scrolled + 32.0
}

/// One line of what to do next, with its button. Returns its height.
#[allow(clippy::too_many_arguments)]
fn next_row(
    ui: &mut Ui,
    x: f32,
    y: f32,
    w: f32,
    icon: Icon,
    line: &str,
    button: Option<(&str, Action)>,
) -> f32 {
    ui.card(x, y, w, 60.0, ACCENT.with_alpha(110));
    ui.icon(x + 18.0, y + 20.0, 20.0, icon, 12.0, ACCENT);
    let line = ui.fit(14.0, W::S, line, w - 260.0);
    ui.text_mid(x + 46.0, y, 60.0, 14.0, W::S, TEXT, &line);
    if let Some((label, action)) = button {
        let bw = ui.measure(14.0, W::S, label) + 36.0;
        ui.button(
            x + w - 16.0 - bw,
            y + 10.0,
            Some(bw),
            40.0,
            label,
            Style::Primary,
            action,
        );
    }
    76.0
}

fn done(app: &Faraday, id: CardId) -> bool {
    match id {
        CardId::Page(p) => app.family_page_done(p),
        CardId::Step(n) => app.spend.as_ref().is_some_and(|s| s.done[n as usize]),
        CardId::Later(_) => false,
    }
}

fn summary(app: &Faraday, id: CardId) -> String {
    let s = match id {
        CardId::Page(page::MAP) => "Six stops",
        CardId::Page(page::SAFE) if app.online => "Desktop copy",
        CardId::Page(page::SAFE) => "Running from the stick",
        CardId::Page(page::HOLDING) => {
            return match app.family.route {
                Some(r) => text::ANSWERS
                    .iter()
                    .find(|a| a.0 == r)
                    .map(|a| a.1.to_string())
                    .unwrap_or_default(),
                None => "Not answered yet".to_string(),
            };
        }
        CardId::Page(page::OPEN) => {
            return match app.family_wallet().map(|i| &app.session.wallets[i]) {
                Some(w) => format!("{} · {}", w.name, Session::shape(w)),
                None => "Not open yet".to_string(),
            };
        }
        CardId::Page(page::CHECK) if app.family_page_done(page::CHECK) => "First address matches",
        CardId::Page(page::WRITE) if app.family_page_done(page::WRITE) => "Done in Sparrow",
        CardId::Page(page::BRING) => {
            return match app.spend.as_ref() {
                Some(s) => format!("Signing {}", s.spend.source),
                None => "Not here yet".to_string(),
            };
        }
        CardId::Page(page::AWAY) => {
            return match app.outbox.len() {
                0 => "Outbox empty".to_string(),
                1 => "1 file in the Outbox".to_string(),
                n => format!("{n} files in the Outbox"),
            };
        }
        CardId::Page(_) => "",
        CardId::Step(n) => {
            return match app.spend.as_ref() {
                Some(s) => step_summary(app, n, s.wallet, app.spend_needed()),
                None => String::new(),
            };
        }
        CardId::Later(_) => "After the transaction",
    };
    s.to_string()
}

fn body(app: &Faraday, ui: &mut Ui, id: CardId, x: f32, y: f32, w: f32) -> f32 {
    let mut cy = y;
    cy += match id {
        CardId::Page(page::MAP) => map(ui, x, cy, w),
        CardId::Page(page::SAFE) => {
            next_button(ui, x, cy, w, "Continue", fa(F::Next(page::SAFE)));
            52.0
        }
        CardId::Page(page::HOLDING) => holding(app, ui, x, cy, w),
        CardId::Page(page::OPEN) => open(app, ui, x, cy, w),
        CardId::Page(page::CHECK) => check(app, ui, x, cy, w),
        CardId::Page(page::WRITE) => {
            next_button(ui, x, cy, w, "I have the PSBT", fa(F::Next(page::WRITE)));
            52.0
        }
        CardId::Page(page::BRING) => bring(app, ui, x, cy, w),
        CardId::Page(_) => away(app, ui, x, cy, w),
        CardId::Step(step::SIGNERS) if !threshold(app) => signers(app, ui, x, cy, w),
        CardId::Step(n) => {
            let Some(s) = app.spend.as_ref() else {
                return 0.0;
            };
            let mut used = step_body(app, ui, n, x, cy, w, s.wallet, app.spend_needed());
            if n == step::FINISH && s.spend.finished.is_some() {
                ui.button(
                    x,
                    cy + used,
                    None,
                    38.0,
                    "Show the finished transaction as QR",
                    Style::Secondary,
                    fa(F::QrTx),
                );
                used += 50.0;
            }
            used
        }
        CardId::Later(_) => 0.0,
    };
    let paras = text::more(app, id);
    if !paras.is_empty() {
        cy += more(app, ui, id, paras, x, cy, w);
    }
    cy - y
}

fn threshold(app: &Faraday) -> bool {
    app.spend
        .as_ref()
        .and_then(|s| s.wallet)
        .and_then(|w| app.session.wallets.get(w))
        .is_some_and(|w| Kind::of(&w.policy) == Kind::Threshold)
}

/// More about this: a quiet button, and the paragraphs while it is open.
fn more(app: &Faraday, ui: &mut Ui, id: CardId, paras: &[&str], x: f32, y: f32, w: f32) -> f32 {
    let bit = more_bit(id);
    let shown = app.family.more & (1 << u32::from(bit)) != 0;
    let mut cy = y + 4.0;
    ui.button(
        x - 12.0,
        cy,
        None,
        32.0,
        if shown {
            "Less about this"
        } else {
            "More about this"
        },
        Style::Ghost,
        fa(F::More(bit)),
    );
    cy += 40.0;
    if shown {
        for p in paras {
            cy += ui.wrap(x, cy, w, 13.0, W::R, MUTED, p) + 10.0;
        }
    }
    cy - y
}

fn map(ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    let mut cy = y;
    for (k, (place, head, line)) in text::MAP.iter().enumerate() {
        let c = match *place {
            "Offline" => OK,
            "Online" => WARN,
            _ => MUTED,
        };
        // The lane: one node a stop, joined.
        ui.dot(x + 9.0, cy + 11.0, 6.0, c);
        let used = 30.0 + ui.wrap(x + 112.0, cy + 26.0, w - 112.0, 13.0, W::R, MUTED, line);
        if k + 1 < text::MAP.len() {
            ui.fill(x + 8.0, cy + 20.0, 2.0, used - 12.0, 1.0, BORDER);
        }
        ui.text(x + 28.0, cy + 2.0, 12.0, W::S, c, place);
        ui.text(x + 112.0, cy + 2.0, 14.0, W::S, TEXT, head);
        cy += used + 10.0;
    }
    cy += 6.0;
    ui.icon(x, cy, 16.0, Icon::Warning, 10.0, WARN);
    cy += ui.wrap(
        x + 22.0,
        cy,
        w - 22.0,
        13.0,
        W::S,
        WARN,
        text::WORDS_ARE_MONEY,
    ) + 16.0;
    next_button(ui, x, cy, w, "Start", fa(F::Next(page::MAP)));
    cy + 52.0 - y
}

/// What are you holding?: big answers that turn the page.
fn holding(app: &Faraday, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    let mut cy = y;
    let answers = text::ANSWERS
        .iter()
        .map(|(r, head, sub)| {
            (
                *head,
                *sub,
                fa(F::Holding(*r)),
                app.family.route == Some(*r),
            )
        })
        .chain(std::iter::once((
            text::UNSURE.0,
            text::UNSURE.1,
            fa(F::Unsure),
            app.family.help,
        )));
    for (head, sub, action, on) in answers {
        let top = cy;
        let pressed = ui.is_pressed(action);
        let th = ui.wrap(x + 20.0, top + 44.0, w - 40.0, 13.0, W::R, MUTED, sub);
        let bh = 60.0 + th;
        ui.text(x + 20.0, top + 16.0, 16.0, W::S, TEXT, head);
        let edge = if on || pressed { ACCENT } else { BORDER };
        ui.stroke(x, top, w, bh, 12.0, edge);
        if on {
            ui.icon(x + w - 40.0, top + 14.0, 24.0, Icon::Done, 12.0, ACCENT);
        }
        ui.hit(x, top, w, bh, action);
        cy += bh + 10.0;
    }
    if app.family.help {
        cy += 8.0;
        for (look, what) in text::LOOKS {
            cy += ui.wrap(x, cy, w, 14.0, W::S, TEXT, look) + 4.0;
            cy += ui.wrap(x, cy, w, 13.0, W::R, MUTED, what) + 14.0;
        }
    }
    cy - y + 6.0
}

fn open(app: &Faraday, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    let mut cy = y;
    match app.family.route {
        Some(Route::Vault) => cy += vault(app, ui, x, cy, w),
        Some(Route::Words) => cy += words(app, ui, x, cy, w),
        Some(Route::Paper) => cy += paper(app, ui, x, cy, w),
        None => {
            ui.button(
                x,
                cy,
                None,
                38.0,
                "What are you holding?",
                Style::Secondary,
                fa(F::Card(page::HOLDING)),
            );
            return 50.0;
        }
    }
    if let Some(e) = &app.family.error {
        cy += ui.wrap(x, cy, w, 13.0, W::R, ERR, e) + 10.0;
    }
    cy += opened(app, ui, x, cy, w);
    cy - y
}

/// The vault route: the vaults, the stick, the passphrase, Unlock.
fn vault(app: &Faraday, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    let mut cy = y;
    let files = app.vault_files();
    // Open already: its seeds and wallets, all ticked at first, load
    // from the same list as on Files.
    for v in 0..app.vaults.open.len() {
        cy += crate::screens::vault_panel(app, ui, v, x, cy, w) + 14.0;
    }
    let locked: Vec<(usize, &crate::vaults::VaultFile)> = files
        .iter()
        .enumerate()
        .filter(|(_, f)| f.open.is_none())
        .collect();
    if locked.is_empty() && app.vaults.open.is_empty() {
        cy += ui.wrap(
            x,
            cy,
            w,
            13.0,
            W::R,
            MUTED,
            "No vault in Files. The stick this computer started from brings its vault in by \
             itself; a vault on another stick is copied in on a stick visit.",
        ) + 12.0;
        cy += stick_row(app, ui, x, cy, w, "Copy a vault in from a stick");
        return cy - y;
    }
    if locked.is_empty() {
        return cy - y;
    }
    if !app.may_load_keys() {
        cy += crate::vault_screens::stick_banner(
            ui,
            x,
            cy,
            w,
            "Remove the stick, then type the passphrase",
        );
    }
    for (i, f) in &locked {
        let on = *i == app.vaults.pick;
        let ch = 64.0;
        ui.fill(
            x,
            cy,
            w,
            ch,
            10.0,
            if on { ACCENT.with_alpha(18) } else { BG },
        );
        ui.stroke(x, cy, w, ch, 10.0, if on { ACCENT } else { INNER });
        ui.icon(
            x + 14.0,
            cy + 14.0,
            36.0,
            Icon::Lock,
            15.0,
            if on { ACCENT } else { MUTED },
        );
        ui.text(x + 62.0, cy + 12.0, 14.0, W::M, TEXT, &f.name);
        let where_ = if f.in_outbox {
            "Sealed here, in the Outbox"
        } else {
            "Copied in from a stick"
        };
        ui.text(
            x + 62.0,
            cy + 36.0,
            12.0,
            W::R,
            MUTED,
            &format!("{where_} · {}", file_text(f.len)),
        );
        let mem = f.header.cost.memory_kib / 1024;
        let time = format!(
            "Unlocks in {}",
            app.vaults.time_text(mem, f.header.cost.passes)
        );
        ui.text_right(x + w - 14.0, cy + 8.0, 24.0, 12.0, W::S, TEXT, &time);
        ui.text_right(
            x + w - 14.0,
            cy + 32.0,
            24.0,
            12.0,
            W::R,
            MUTED,
            &cost_text(&f.header.cost),
        );
        ui.hit(x, cy, w, ch, fa(F::Pick(*i)));
        cy += ch + 8.0;
    }
    cy += 8.0;
    ui.text(x, cy, 13.0, W::S, MUTED, "Passphrase");
    cy += 22.0;
    if app.may_load_keys() {
        let focused = app.vaults.focus == Some(crate::vaults::Focus::Passphrase);
        crate::vault_screens::secret_box(
            ui,
            x,
            cy,
            w,
            &app.vaults.passphrase,
            app.vaults.typed_shown,
            focused,
            Action::Vault(VaultAction::FocusPassphrase),
            Action::Vault(VaultAction::ShowTyped),
        );
    } else {
        crate::vault_screens::stick_field(ui, x, cy, w);
    }
    cy += 52.0;
    if app.vaults.working == Some(crate::vaults::Work::Unlock) {
        ui.text_mid(x, cy, 42.0, 14.0, W::S, ACCENT, "Unlocking");
    } else {
        let style = if app.may_load_keys() {
            Style::Primary
        } else {
            Style::Disabled
        };
        let bw = ui.button(x, cy, Some(140.0), 42.0, "Unlock", style, fa(F::Unlock));
        let free = match app.vaults.memory_free_mib {
            Some(m) => format!("This computer: {} free", mib_text(m)),
            None => String::new(),
        };
        ui.text_mid(x + bw + 16.0, cy, 42.0, 13.0, W::R, MUTED, &free);
    }
    cy += 54.0;
    if let Some(e) = &app.vaults.unlock_error {
        cy += ui.wrap(x, cy, w, 13.0, W::R, ERR, e) + 10.0;
    }
    cy - y
}

/// A stick visit from this tab, or what stops one.
fn stick_row(app: &Faraday, ui: &mut Ui, x: f32, y: f32, _w: f32, label: &str) -> f32 {
    let open = app.holds_secret() || !app.vaults.open.is_empty();
    let (style, note) = match (app.sticks.is_empty(), open) {
        (true, _) => (Style::Disabled, "Plug the stick in"),
        (false, true) => (
            Style::Disabled,
            "Faraday asks to lock first when a stick goes in",
        ),
        (false, false) => (Style::Secondary, ""),
    };
    let bw = ui.button(
        x,
        y,
        None,
        38.0,
        label,
        style,
        Action::VisitFrom(crate::Screen::Family),
    );
    if !note.is_empty() {
        ui.text_mid(x + bw + 14.0, y, 38.0, 13.0, W::R, DIM, note);
    }
    50.0
}

/// The words route: type or scan the words; the kind of wallet they
/// open, each with its first address.
fn words(app: &Faraday, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    let mut cy = y;
    let may = app.may_load_keys();
    let style = |s| if may { s } else { Style::Disabled };
    let typed = app
        .session
        .wallets
        .iter()
        .any(|wl| wl.source == crate::family::WORDS_SOURCE);
    let bw = ui.button(
        x,
        cy,
        None,
        40.0,
        if typed {
            "Type other words"
        } else {
            "Type the words"
        },
        style(if typed {
            Style::Secondary
        } else {
            Style::Primary
        }),
        fa(F::TypeWords),
    );
    ui.button(
        x + bw + 8.0,
        cy,
        None,
        40.0,
        "Scan a SeedQR",
        style(Style::Secondary),
        Action::ScanSeed,
    );
    cy += 52.0;
    if !may {
        ui.text(x, cy, 13.0, W::S, WARN, "Remove the stick first");
        cy += 28.0;
    }
    let (Some(key), Some(now)) = (app.session.keys.last(), app.family_words_kind_now()) else {
        return cy - y;
    };
    ui.text(
        x,
        cy,
        13.0,
        W::S,
        MUTED,
        &format!(
            "Key {} · the kind of wallet",
            fp_text(key.master.fingerprint())
        ),
    );
    cy += 26.0;
    for (k, kind) in crate::family::SINGLE.iter().enumerate() {
        let on = *kind == now;
        let addr = kind
            .key_text(&key.master)
            .ok()
            .and_then(|t| crate::create::build(*kind, 1, &[t]).ok())
            .and_then(|p| p.address_at(app.session.network(), false, 0).ok())
            .map(|a| a.to_string())
            .unwrap_or_default();
        ui.checkbox(x, cy + 11.0, on, true);
        ui.text_mid(
            x + 30.0,
            cy,
            40.0,
            14.0,
            W::S,
            if on { TEXT } else { MUTED },
            crate::family::kind_name(*kind),
        );
        let a = ui.fit(12.0, W::M, &addr, w - 190.0);
        ui.text_right(x + w, cy, 40.0, 12.0, W::M, MUTED, &a);
        ui.hit(x, cy, w, 40.0, fa(F::Kind(k as u8)));
        ui.rule(x, cy + 40.0, w, INNER);
        cy += 42.0;
    }
    cy + 12.0 - y
}

/// The paper route: the description by camera or from Files, whole or
/// in split sheets.
fn paper(app: &Faraday, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    let mut cy = y;
    let ready = app.family_ready();
    ui.button(
        x,
        cy,
        None,
        40.0,
        "Scan the QR code",
        if ready {
            Style::Secondary
        } else {
            Style::Primary
        },
        Action::Scan,
    );
    cy += 52.0;
    cy += stick_row(app, ui, x, cy, w, "Copy it in from a stick");
    let wallets: Vec<(usize, &str)> = app
        .inbox
        .iter()
        .enumerate()
        .filter(|(_, i)| i.kind == FileKind::Wallet)
        .map(|(k, i)| (k, i.name.as_str()))
        .collect();
    for (k, name) in &wallets {
        ui.button(
            x,
            cy,
            None,
            36.0,
            &format!("Open {name}"),
            Style::Secondary,
            fa(F::UseWallet(*k)),
        );
        cy += 44.0;
    }
    let shares: Vec<String> = app
        .inbox
        .iter()
        .filter(|i| i.kind == FileKind::Share)
        .map(|i| String::from_utf8_lossy(&i.bytes).into_owned())
        .collect();
    if !shares.is_empty() {
        match crate::restore::merge(&shares) {
            Ok(m) => {
                let line = format!(
                    "{} {} · {} of {} keys in hand · {}",
                    shares.len(),
                    if shares.len() == 1 { "sheet" } else { "sheets" },
                    m.have.len(),
                    m.n,
                    m.have.join(" ")
                );
                ui.text_mid(
                    x,
                    cy,
                    36.0,
                    13.0,
                    W::M,
                    if m.whole.is_some() { OK } else { WARN },
                    &line,
                );
                cy += 40.0;
                if m.whole.is_some() && !ready {
                    ui.button(
                        x,
                        cy,
                        None,
                        38.0,
                        "Rebuild the wallet",
                        Style::Primary,
                        fa(F::Rebuild),
                    );
                    cy += 50.0;
                }
            }
            Err(e) => {
                cy += ui.wrap(x, cy, w, 13.0, W::R, ERR, &e) + 10.0;
            }
        }
    }
    cy - y
}

/// What opened: the wallet to spend from, its keys here, and its first
/// address; a choice when the vault held several.
fn opened(app: &Faraday, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    let mut cy = y;
    let chosen = app.family_wallet();
    if app.session.wallets.is_empty() {
        // Said only of a vault that holds no wallet at all, not of one
        // whose wallets wait to be loaded.
        let wallet_in_vault =
            (0..app.vaults.open.len()).any(|v| app.vault_rows(v).iter().any(|r| !r.seed));
        if app.family.route == Some(Route::Vault) && !app.vaults.open.is_empty() && !wallet_in_vault
        {
            cy += ui.wrap(
                x,
                cy,
                w,
                13.0,
                W::R,
                WARN,
                "The vault holds keys but not the wallet they belong to. Load the wallet's \
                 description from the paper: scan its QR code, or copy it in.",
            ) + 12.0;
            cy += paper(app, ui, x, cy, w);
        }
        return cy - y;
    }
    if app.session.wallets.len() > 1 {
        ui.text(x, cy, 13.0, W::S, MUTED, "Spend from");
        cy += 26.0;
        for (i, wl) in app.session.wallets.iter().enumerate() {
            let on = chosen == Some(i);
            ui.checkbox(x, cy + 11.0, on, true);
            ui.text_mid(x + 30.0, cy, 40.0, 14.0, W::S, TEXT, &wl.name);
            // How many of its keys are here: a vault may hold only some.
            let slots = app.session.slots(wl);
            let here = slots.iter().filter(|s| s.held_by.is_some()).count();
            let line = format!(
                "{} · {here} of {} keys here",
                Session::shape(wl),
                slots.len()
            );
            ui.text_right(
                x + w,
                cy,
                40.0,
                12.0,
                W::R,
                if here > 0 { MUTED } else { DIM },
                &line,
            );
            ui.hit(x, cy, w, 40.0, fa(F::Choose(i)));
            cy += 42.0;
        }
        cy += 8.0;
    }
    let Some(i) = chosen else {
        return cy - y;
    };
    let wl = &app.session.wallets[i];
    ui.fill(x, cy, w, 1.0, 0.0, INNER);
    cy += 14.0;
    ui.text(x, cy, 15.0, W::S, TEXT, &wl.name);
    ui.text_right(
        x + w,
        cy - 4.0,
        24.0,
        12.0,
        W::R,
        MUTED,
        &Session::shape(wl),
    );
    cy += 28.0;
    let slots = app.session.slots(wl);
    let here = slots.iter().filter(|s| s.held_by.is_some()).count();
    let (needed, n) = Session::quorum(wl);
    let line = if slots.len() <= 1 {
        if here == 1 {
            "Its key is here: it can sign".to_string()
        } else {
            "Its key is not here yet".to_string()
        }
    } else {
        format!("{needed} of {n} keys sign · {here} here")
    };
    ui.text(
        x,
        cy,
        13.0,
        W::R,
        if here >= needed.min(1) { OK } else { MUTED },
        &line,
    );
    cy += 26.0;
    ui.text(x, cy, 12.0, W::R, MUTED, "Receive address 0");
    cy += 20.0;
    cy += ui.wrap(
        x,
        cy,
        w,
        14.0,
        W::M,
        TEXT,
        &grouped(&app.session.address(wl, false, 0)),
    ) + 16.0;
    next_button(ui, x, cy, w, "Continue", fa(F::Next(page::OPEN)));
    cy + 52.0 - y
}

/// Check the money: the wallet as a QR code, and its addresses.
fn check(app: &Faraday, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    let mut cy = y;
    let Some(i) = app.family_wallet() else {
        return 0.0;
    };
    let wl = &app.session.wallets[i];
    ui.button(
        x,
        cy,
        None,
        40.0,
        "Show the wallet as a QR code",
        Style::Primary,
        Action::QrWallet(i),
    );
    cy += 54.0;
    let n = 3 + app.family.addresses;
    for (head, change) in [("Receive", false), ("Change", true)] {
        ui.text(x, cy, 13.0, W::S, MUTED, head);
        cy += 24.0;
        for k in 0..n {
            ui.text_mid(x, cy, 28.0, 12.0, W::R, DIM, &k.to_string());
            let a = grouped(&app.session.address(wl, change, k));
            let a = ui.fit(13.0, W::M, &a, w - 40.0);
            ui.text_mid(x + 40.0, cy, 28.0, 13.0, W::M, TEXT, &a);
            cy += 30.0;
        }
        cy += 8.0;
    }
    if n < 50 {
        ui.button(
            x,
            cy,
            None,
            36.0,
            "Show more addresses",
            Style::Secondary,
            fa(F::MoreAddresses),
        );
    }
    next_button(ui, x, cy, w, "It matches", fa(F::Next(page::CHECK)));
    cy + 52.0 - y
}

/// Bring the transaction: scan it, copy it in, or one already in Files.
fn bring(app: &Faraday, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    let mut cy = y;
    ui.button(
        x,
        cy,
        None,
        40.0,
        "Scan",
        if app.spend.is_some() {
            Style::Secondary
        } else {
            Style::Primary
        },
        Action::Scan,
    );
    cy += 52.0;
    cy += stick_row(app, ui, x, cy, w, "Copy it in from a stick");
    let psbts: Vec<(usize, &str)> = app
        .inbox
        .iter()
        .enumerate()
        .filter(|(_, i)| i.kind == FileKind::Psbt)
        .map(|(k, i)| (k, i.name.as_str()))
        .collect();
    if !psbts.is_empty() {
        ui.text(x, cy, 13.0, W::S, MUTED, "In Files");
        cy += 24.0;
    }
    for (k, name) in &psbts {
        let current = app.spend.as_ref().is_some_and(|s| s.spend.source == *name);
        ui.text_mid(x, cy, 38.0, 14.0, W::M, TEXT, name);
        if current {
            ui.text_right(x + w, cy, 38.0, 13.0, W::S, OK, "Signing this one");
        } else {
            let label = "Use this one";
            let bw = ui.measure(13.0, W::S, label) + 32.0;
            ui.button(
                x + w - bw,
                cy + 1.0,
                Some(bw),
                36.0,
                label,
                Style::Secondary,
                fa(F::UsePsbt(*k)),
            );
        }
        cy += 44.0;
    }
    if let Some(s) = app.spend.as_ref() {
        cy += 6.0;
        match s.wallet.and_then(|w| app.session.wallets.get(w)) {
            Some(wl) => {
                let line = format!("Spends from {} · {}", wl.name, Session::shape(wl));
                ui.text(x, cy, 13.0, W::S, OK, &line);
                cy += 30.0;
            }
            None => {
                cy += ui.wrap(
                    x,
                    cy,
                    w,
                    13.0,
                    W::R,
                    WARN,
                    "No wallet open here matches this transaction. Change cannot be verified \
                     until its wallet is open: check that this is the transaction for this \
                     wallet.",
                ) + 12.0;
            }
        }
        next_button(ui, x, cy, w, "Read it", Action::Step(first_step(app)));
        cy += 52.0;
    }
    cy - y
}

fn first_step(app: &Faraday) -> u8 {
    app.spend
        .as_ref()
        .and_then(|s| s.steps.iter().copied().find(|&n| !s.done[n as usize]))
        .unwrap_or(step::TRANSACTION)
}

/// Who signs: each of the wallet's keys, here or not, and a way to add
/// a missing one's seed.
fn signers(app: &Faraday, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    let mut cy = y;
    let Some(s) = app.spend.as_ref() else {
        return 0.0;
    };
    let needed = app.spend_needed();
    let may = app.may_load_keys();
    match s.wallet.and_then(|i| app.session.wallets.get(i)) {
        Some(wl) => {
            for (k, slot) in app.session.slots(wl).iter().enumerate() {
                ui.text_mid(x, cy, 42.0, 12.0, W::R, DIM, &(k + 1).to_string());
                let fp = slot.fingerprint.map(fp_text).unwrap_or_default();
                ui.text_mid(x + 22.0, cy, 42.0, 14.0, W::M, TEXT, &fp);
                match &slot.held_by {
                    Some(label) => {
                        ui.text_mid(x + 124.0, cy, 42.0, 13.0, W::R, MUTED, label);
                        let t = "Signs here";
                        let cw2 = ui.measure(12.0, W::R, t) + 34.0;
                        ui.chip(x + w - cw2, cy + 8.0, t, OK, OK.with_alpha(30));
                    }
                    None => {
                        let label = "Type its words";
                        let bw = ui.measure(13.0, W::S, label) + 32.0;
                        ui.button(
                            x + w - bw,
                            cy + 3.0,
                            Some(bw),
                            36.0,
                            label,
                            if may {
                                Style::Secondary
                            } else {
                                Style::Disabled
                            },
                            Action::Entry(slot.fingerprint.map(|f| f.0)),
                        );
                    }
                }
                ui.rule(x, cy + 42.0, w, INNER);
                cy += 44.0;
            }
        }
        None => {
            for fp in &s.inspection.participating_keys {
                ui.text_mid(x, cy, 30.0, 14.0, W::M, TEXT, &fp_text(*fp));
                cy += 30.0;
            }
        }
    }
    cy += 10.0;
    let here = s
        .inspection
        .participating_keys
        .iter()
        .filter(|fp| app.session.key_label(**fp).is_some())
        .count();
    // The other keys: a SeedQR, typed words above, or another vault.
    let mut bx = x;
    bx += ui.button(
        x,
        cy,
        None,
        38.0,
        "Scan a SeedQR",
        if may {
            Style::Secondary
        } else {
            Style::Disabled
        },
        Action::ScanSeed,
    ) + 8.0;
    for (k, f) in app.vault_files().iter().enumerate() {
        if f.open.is_some() {
            continue;
        }
        let label = format!("Unlock {}", f.name);
        let bw = ui.measure(13.0, W::S, &label) + 32.0;
        if bx + bw > x + w {
            bx = x;
            cy += 46.0;
        }
        bx += ui.button(
            bx,
            cy,
            Some(bw),
            38.0,
            &label,
            if may {
                Style::Secondary
            } else {
                Style::Disabled
            },
            Action::Vault(VaultAction::OpenFrom(k, crate::Screen::Family)),
        ) + 8.0;
    }
    cy += 48.0;
    if !may {
        ui.text(x, cy, 13.0, W::S, WARN, "Remove the stick first");
        cy += 28.0;
    }
    ui.text_mid(
        x,
        cy,
        40.0,
        13.0,
        W::R,
        if here >= needed { OK } else { MUTED },
        &format!("Needs {needed} · {here} here"),
    );
    next_button(ui, x, cy, w, "Continue", Action::StepNext(step::SIGNERS));
    cy + 52.0 - y
}

/// Put everything away: lock, write what waits, power off.
fn away(app: &Faraday, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    let mut cy = y;
    for it in &app.outbox {
        ui.icon(x, cy + 6.0, 20.0, Icon::File, 11.0, MUTED);
        ui.text_mid(x + 28.0, cy, 32.0, 13.0, W::M, TEXT, &it.name);
        cy += 34.0;
    }
    if !app.outbox.is_empty() {
        cy += 8.0;
    }
    let bw = ui.button(x, cy, None, 40.0, "Lock", Style::Primary, Action::LockAsk);
    ui.button(
        x + bw + 8.0,
        cy,
        None,
        40.0,
        "Power off",
        Style::Secondary,
        Action::PowerAsk,
    );
    let label = "Start over";
    let sw = ui.measure(13.0, W::S, label) + 32.0;
    ui.button(
        x + w - sw,
        cy + 2.0,
        Some(sw),
        36.0,
        label,
        Style::Ghost,
        fa(F::StartOver),
    );
    cy + 52.0 - y
}
