//! The boot import's sheet (`boot_import`): with the stick in, what was
//! read and that the stick is to be pulled; once it is out, the vaults it
//! brought, each unlocked on the sheet, and "Choose what to import", which
//! lists the wallets, keys and files with a checkbox each.

use osk_ui::widgets::Icon;

use crate::boot_import::{ImportAction as I, ImportCount, ImportView};
use crate::compact::{buttons, scroll_sheet, sheet_head};
use crate::ui::pal::*;
use crate::ui::{Style, Ui, W};
use crate::vault_screens::{secret_box, stick_banner, stick_field};
use crate::vaults::{Focus, VaultAction as V, Work};
use crate::{Action, Faraday};

fn ia(a: I) -> Action {
    Action::Import(a)
}

/// The sheet over Home.
pub(crate) fn draw(app: &Faraday, ui: &mut Ui, w: f32, h: f32) {
    let Some(imp) = app.import.as_ref() else {
        return;
    };
    let label = imp.label.clone();
    let choosing = imp.choosing;
    let count = app.import_count().unwrap_or_default();
    let view = (!app.import_stick_present())
        .then(|| app.import_view())
        .flatten();
    let place = if ui.compact {
        ((8.0, w - 16.0), 8.0, 16.0)
    } else {
        let sw = (w - 48.0).min(640.0);
        (((w - sw) / 2.0, sw), 40.0, 32.0)
    };
    scroll_sheet(
        ui,
        place.0,
        h,
        place.1,
        place.2,
        &mut |ui, x, y, iw| match &view {
            None => present(app, ui, x, y, iw, &label, &count),
            Some(v) => body(app, ui, x, y, iw, &label, &count, v, choosing),
        },
    );
}

/// The sheet's title, with its icon. Returns its height.
fn head(ui: &mut Ui, x: f32, y: f32, w: f32, icon: Icon, title: &str) -> f32 {
    if ui.compact {
        return sheet_head(ui, x, y, w, icon, ACCENT, title);
    }
    ui.icon(x, y, 30.0, icon, 18.0, ACCENT);
    let t = ui.fit(20.0, W::S, title, w - 42.0);
    ui.text_mid(x + 42.0, y, 30.0, 20.0, W::S, TEXT, &t);
    46.0
}

/// "1 vault · 2 PSBTs · 4 other files". Returns its height.
fn counted(ui: &mut Ui, x: f32, y: f32, w: f32, c: &ImportCount) -> f32 {
    ui.wrap(x, y, w, 14.0, W::S, TEXT, &c.line()) + 16.0
}

/// The stick is still in: what was read, and that it is to be pulled.
fn present(
    app: &Faraday,
    ui: &mut Ui,
    x: f32,
    y: f32,
    w: f32,
    label: &str,
    c: &ImportCount,
) -> f32 {
    let mut cy = y;
    cy += head(
        ui,
        x,
        cy,
        w,
        app.medium.icon(),
        &format!("Read from {label}"),
    );
    cy += counted(ui, x, cy, w, c);
    let pull = format!("Pull the {} to continue", app.medium.noun());
    cy += stick_banner(ui, x, cy, w, &pull);
    cy += buttons(ui, x, cy, w, &[("Not now", Style::Secondary, ia(I::Later))]);
    cy - y
}

/// A group's name over its rows.
fn group(ui: &mut Ui, x: f32, y: f32, w: f32, name: &str) -> f32 {
    ui.rule(x, y, w, INNER);
    ui.text(x, y + 14.0, 13.0, W::S, MUTED, name);
    40.0
}

/// The stick is out: its vaults, each unlocked here; and, once asked
/// for, the wallets, the keys and the files to import.
#[allow(clippy::too_many_arguments)]
fn body(
    app: &Faraday,
    ui: &mut Ui,
    x: f32,
    y: f32,
    w: f32,
    label: &str,
    c: &ImportCount,
    v: &ImportView,
    choosing: bool,
) -> f32 {
    let may = app.may_load_keys();
    let field = app.import_field();
    // A small panel with its keyboard up has room for the field alone.
    if ui.compact
        && app.osk_shown()
        && let Some(vault) = field.and_then(|i| v.vaults.get(i))
    {
        let cy = y + typing(app, ui, x, y, w, &vault.title, vault.file);
        let foot = [
            ("Not now", Style::Secondary, ia(I::Later)),
            ("Unlock", Style::Primary, ia(I::Submit)),
        ];
        let n = if app.vaults.working == Some(Work::Unlock) {
            1
        } else {
            2
        };
        return cy + buttons(ui, x, cy, w, &foot[..n]) - y;
    }
    let mut cy = y;
    cy += head(ui, x, cy, w, Icon::Download, &format!("Read from {label}"));
    cy += counted(ui, x, cy, w, c);
    if !may {
        let pull = format!("Pull the {} to continue", app.medium.noun());
        cy += stick_banner(ui, x, cy, w, &pull);
    }

    // The vaults the stick brought; the passphrase field under the one
    // being unlocked.
    let several = v.vaults.len() > 1;
    for (i, vault) in v.vaults.iter().enumerate() {
        let open = vault.open.is_some();
        let button = several && !open && field != Some(i);
        let (bw, room) = if button {
            let bw = ui.measure(13.0, W::S, "Unlock") + 32.0;
            (bw, w - 28.0 - bw - 12.0)
        } else {
            (0.0, w - 28.0)
        };
        ui.icon(
            x - 2.0,
            cy + 2.0,
            22.0,
            Icon::Lock,
            12.0,
            if open { OK } else { MUTED },
        );
        let name = ui.fit(14.0, W::M, &vault.title, room);
        ui.text(x + 28.0, cy + 2.0, 14.0, W::M, TEXT, &name);
        let line = match &vault.open {
            Some(slot) => format!("Open · {slot}"),
            None => format!("Locked · {}", crate::vaults::file_text(vault.len)),
        };
        let line = ui.fit(12.0, W::R, &line, room);
        ui.text(
            x + 28.0,
            cy + 24.0,
            12.0,
            W::R,
            if open { OK } else { MUTED },
            &line,
        );
        if button {
            ui.button(
                x + w - bw,
                cy + 4.0,
                Some(bw),
                36.0,
                "Unlock",
                if may {
                    Style::Secondary
                } else {
                    Style::Disabled
                },
                ia(I::Unlock(i)),
            );
        }
        cy += 52.0;
        if field == Some(i) {
            cy += passphrase(app, ui, x, cy, w, vault.file);
        }
    }

    // What to import, ticked, once asked for.
    if !choosing {
        let t = "Choose what to import";
        let tw = ui.measure(13.0, W::S, t);
        ui.text_mid(x, cy, 36.0, 13.0, W::S, ACCENT, t);
        ui.hit(x - 6.0, cy, tw + 12.0, 36.0, ia(I::Choose));
        cy += 44.0;
    } else {
        cy += lists(ui, x, cy, w, v);
    }

    // Not now; Import, for what is ticked without unlocking, once the
    // lists are shown; Unlock under a field. A small panel has room for
    // two, and a tap beside the sheet is Not now there.
    let mut foot: Vec<(&str, Style, Action)> = Vec::new();
    if !(ui.compact && choosing) {
        foot.push(("Not now", Style::Secondary, ia(I::Later)));
    }
    if choosing {
        let style = match (may, field.is_none()) {
            (false, _) => Style::Disabled,
            (true, true) => Style::Primary,
            (true, false) => Style::Secondary,
        };
        foot.push(("Import", style, ia(I::Go)));
    }
    if field.is_some() && app.vaults.working != Some(Work::Unlock) {
        foot.push((
            "Unlock",
            if may { Style::Primary } else { Style::Disabled },
            ia(I::Submit),
        ));
    }
    cy += buttons(ui, x, cy, w, &foot);
    cy - y
}

/// The passphrase field under vault file `file`, and what the unlock
/// says: that it is under way, or why it failed. Returns its height.
fn passphrase(app: &Faraday, ui: &mut Ui, x: f32, y: f32, w: f32, file: usize) -> f32 {
    let mut cy = y;
    if app.may_load_keys() {
        secret_box(
            ui,
            x,
            cy,
            w,
            &app.vaults.passphrase,
            app.vaults.typed_shown,
            app.vaults.focus == Some(Focus::Passphrase),
            Action::Vault(V::FocusPassphrase),
            Action::Vault(V::ShowTyped),
        );
    } else {
        stick_field(app.medium, ui, x, cy, w);
    }
    cy += 50.0;
    if app.vaults.working == Some(Work::Unlock) {
        cy += ui.wrap(x, cy, w, 14.0, W::S, ACCENT, &unlocking(app, file)) + 10.0;
    }
    if let Some(e) = &app.vaults.unlock_error {
        cy += ui.wrap(x, cy, w, 13.0, W::R, ERR, e) + 10.0;
    }
    cy + 4.0 - y
}

/// The field alone, for a small panel with its keyboard up: over it, the
/// vault's name, or that the unlock is under way, or why it failed.
/// Returns its height.
fn typing(app: &Faraday, ui: &mut Ui, x: f32, y: f32, w: f32, title: &str, file: usize) -> f32 {
    let mut cy = y;
    let (line, tone) = if app.vaults.working == Some(Work::Unlock) {
        (unlocking(app, file), ACCENT)
    } else if let Some(e) = &app.vaults.unlock_error {
        (e.clone(), ERR)
    } else {
        (title.to_string(), MUTED)
    };
    cy += ui.wrap(x, cy, w, 12.0, W::S, tone, &line) + 6.0;
    secret_box(
        ui,
        x,
        cy,
        w,
        &app.vaults.passphrase,
        app.vaults.typed_shown,
        app.vaults.focus == Some(Focus::Passphrase),
        Action::Vault(V::FocusPassphrase),
        Action::Vault(V::ShowTyped),
    );
    cy += 44.0;
    cy - y
}

/// "Unlocking · 2 s": the unlock of vault file `file` under way.
fn unlocking(app: &Faraday, file: usize) -> String {
    let time = app
        .vault_files()
        .get(file)
        .map(|f| {
            app.vaults
                .time_text(f.header.cost.memory_kib / 1024, f.header.cost.passes)
        })
        .unwrap_or_default();
    format!("Unlocking · {time}")
}

/// The wallets, the keys no wallet here uses, and every file, each with
/// its checkbox. Returns their height.
fn lists(ui: &mut Ui, x: f32, y: f32, w: f32, v: &ImportView) -> f32 {
    let mut cy = y;
    // The wallets, with whether they sign here and what carries them.
    if !v.wallets.is_empty() {
        cy += group(ui, x, cy, w, "Wallets");
        for (i, wt) in v.wallets.iter().enumerate() {
            let top = cy;
            ui.checkbox(x, cy + 4.0, wt.chosen, true);
            let rx = x + 28.0;
            ui.icon(rx, cy + 2.0, 20.0, Icon::Wallet, 11.0, MUTED);
            let tx = rx + 26.0;
            let tw = x + w - tx;
            let name = ui.fit(14.0, W::S, &wt.name, tw);
            ui.text(tx, cy + 2.0, 14.0, W::S, TEXT, &name);
            cy += 24.0;
            let status = wt.status();
            let tone = if wt.can_sign() {
                OK
            } else if wt.here > 0 {
                WARN
            } else {
                MUTED
            };
            let shape = format!("{} · ", wt.shape);
            let (shw, stw) = (
                ui.measure(12.0, W::R, &shape),
                ui.measure(12.0, W::S, &status),
            );
            if shw + stw <= tw {
                ui.text(tx, cy, 12.0, W::R, MUTED, &shape);
                ui.text(tx + shw, cy, 12.0, W::S, tone, &status);
                cy += 20.0;
            } else {
                let shape = ui.fit(12.0, W::R, &wt.shape, tw);
                ui.text(tx, cy, 12.0, W::R, MUTED, &shape);
                cy += 20.0;
                let status = ui.fit(12.0, W::S, &status, tw);
                ui.text(tx, cy, 12.0, W::S, tone, &status);
                cy += 20.0;
            }
            if !wt.files.is_empty() {
                cy += ui.wrap(tx, cy, tw, 12.0, W::R, DIM, &wt.files.join(" · ")) + 2.0;
            }
            ui.hit(x - 4.0, top, w + 8.0, cy - top, ia(I::Wallet(i)));
            cy += 12.0;
        }
    }

    // The keys no wallet here uses.
    if !v.keys.is_empty() {
        cy += group(ui, x, cy, w, "Keys with no wallet here");
        for (i, k) in v.keys.iter().enumerate() {
            let top = cy;
            ui.checkbox(x, cy + 4.0, k.chosen, true);
            let rx = x + 28.0;
            ui.icon(rx, cy + 2.0, 20.0, Icon::Keys, 11.0, MUTED);
            let tx = rx + 26.0;
            let tw = x + w - tx;
            ui.text(tx, cy + 2.0, 14.0, W::M, TEXT, &k.fingerprint);
            cy += 24.0;
            if !k.files.is_empty() {
                cy += ui.wrap(tx, cy, tw, 12.0, W::R, DIM, &k.files.join(" · ")) + 2.0;
            }
            ui.hit(x - 4.0, top, w + 8.0, cy - top, ia(I::Key(i)));
            cy += 12.0;
        }
    }

    // Every file, for From the stick.
    if !v.files.is_empty() {
        cy += group(ui, x, cy, w, "Files");
        for (i, f) in v.files.iter().enumerate() {
            let top = cy;
            ui.checkbox(x, cy + 4.0, f.chosen, f.enabled);
            let tx = x + 28.0;
            let tw = w - 28.0;
            let fg = if f.enabled { TEXT } else { DIM };
            let name = ui.fit(14.0, W::M, &f.name, tw);
            ui.text(tx, cy + 2.0, 14.0, W::M, fg, &name);
            cy += 24.0;
            cy += ui.wrap(
                tx,
                cy,
                tw,
                12.0,
                W::R,
                if f.enabled { MUTED } else { DIM },
                &f.line,
            ) + 2.0;
            if f.enabled {
                ui.hit(x - 4.0, top, w + 8.0, cy - top, ia(I::File(i)));
            }
            cy += 10.0;
        }
    }
    cy - y
}
