//! The boot import's sheet (`boot_import`): what the boot stick brought
//! while it is attached; once it is out, its vaults, the wallets and keys
//! found, and every file, each with its checkbox, and the one press that
//! imports them.

use osk_ui::widgets::Icon;

use crate::boot_import::{ImportAction as I, ImportCount, ImportView};
use crate::compact::{buttons, scroll_sheet, sheet_head};
use crate::ui::pal::*;
use crate::ui::{Style, Ui, W};
use crate::vault_screens::stick_banner;
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
            Some(v) => body(app, ui, x, y, iw, &label, &count, v),
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

/// "12 files copied into memory", and what they are.
fn counted(ui: &mut Ui, x: f32, y: f32, w: f32, c: &ImportCount) -> f32 {
    let n = |k: usize, one: &str, many: &str| format!("{k} {}", if k == 1 { one } else { many });
    let mut first = format!("{} copied into memory", n(c.copied, "file", "files"));
    if c.reading > 0 {
        first.push_str(&format!(" · {} being read", c.reading));
    }
    let mut what: Vec<String> = Vec::new();
    if c.vaults > 0 {
        what.push(n(c.vaults, "vault", "vaults"));
    }
    if c.wallets > 0 {
        what.push(n(c.wallets, "wallet", "wallets"));
    }
    if c.keys > 0 {
        what.push(n(c.keys, "key", "keys"));
    }
    if c.other > 0 {
        what.push(n(c.other, "other file", "other files"));
    }
    if c.not_read > 0 {
        what.push(format!("{} not read", c.not_read));
    }
    let mut h = ui.wrap(x, y, w, 15.0, W::S, TEXT, &first) + 6.0;
    if !what.is_empty() {
        h += ui.wrap(x, y + h, w, 13.0, W::R, MUTED, &what.join(" · ")) + 4.0;
    }
    h + 12.0
}

/// The stick is still in: what was copied, and the way on.
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
        &format!("Data found on {label}"),
    );
    cy += counted(ui, x, cy, w, c);
    let pull = format!("Remove the {} to start the import", app.medium.noun());
    cy += stick_banner(ui, x, cy, w, &pull);
    cy += buttons(
        ui,
        x,
        cy,
        w,
        &[("Import later", Style::Secondary, ia(I::Later))],
    );
    cy - y
}

/// A group's name over its rows.
fn group(ui: &mut Ui, x: f32, y: f32, w: f32, name: &str) -> f32 {
    ui.rule(x, y, w, INNER);
    ui.text(x, y + 14.0, 13.0, W::S, MUTED, name);
    40.0
}

/// The stick is out: the vaults, the wallets, the keys and the files.
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
) -> f32 {
    let may = app.may_load_keys();
    let mut cy = y;
    cy += head(
        ui,
        x,
        cy,
        w,
        Icon::Download,
        &format!("Import from {label}"),
    );
    cy += counted(ui, x, cy, w, c);
    if !may {
        let pull = format!("Remove the {} to import", app.medium.noun());
        cy += stick_banner(ui, x, cy, w, &pull);
    }

    // The vaults the stick brought, each unlocked here.
    if !v.vaults.is_empty() {
        cy += group(ui, x, cy, w, "Vaults");
        for (i, vault) in v.vaults.iter().enumerate() {
            let open = vault.open.is_some();
            let (bw, room) = if open {
                (0.0, w - 28.0)
            } else {
                let bw = ui.measure(13.0, W::S, "Unlock") + 32.0;
                (bw, w - 28.0 - bw - 12.0)
            };
            ui.icon(
                x - 2.0,
                cy + 2.0,
                22.0,
                Icon::Lock,
                12.0,
                if open { OK } else { MUTED },
            );
            let name = ui.fit(14.0, W::M, &vault.name, room);
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
            if !open {
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
        }
        cy += 4.0;
    }

    // The wallets, with whether they sign here and what carries them.
    if !v.wallets.is_empty() {
        cy += group(ui, x, cy, w, "Wallets to import");
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

    // Every file, for the Inbox.
    if !v.files.is_empty() {
        cy += group(ui, x, cy, w, "Files to copy in");
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

    ui.rule(x, cy, w, INNER);
    cy += 14.0;
    cy += ui.wrap(
        x,
        cy,
        w,
        13.0,
        W::R,
        MUTED,
        &format!(
            "Files not chosen are wiped from memory. To bring one in later, insert the {} again.",
            app.medium.noun()
        ),
    ) + 6.0;
    cy += buttons(
        ui,
        x,
        cy,
        w,
        &[
            ("Import later", Style::Secondary, ia(I::Later)),
            (
                "Import",
                if may { Style::Primary } else { Style::Disabled },
                ia(I::Go),
            ),
        ],
    );
    cy - y
}
