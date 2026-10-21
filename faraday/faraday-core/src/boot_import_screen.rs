//! The boot import's sheet (`boot_import`, F60): what was read from the
//! boot stick, each file with its checkbox, with Later and Import; after
//! Import, each vault it brought in turn with its passphrase field, once
//! the stick is out, and Cancel.

use crate::drawing::View;
use crate::strings::EN;
use crate::strings::fill;
use osk_ui::widgets::Icon;

use crate::Action;
use crate::boot_import::{ImportAction as I, ImportCount, ImportVault, ImportView};
use crate::compact::{buttons, scroll_sheet, sheet_head};
use crate::ui::pal::*;
use crate::ui::{Glyph, Style, Ui, W};
use crate::vault_screens::{secret_box, stick_banner, stick_field};
use crate::vaults::{Focus, VaultAction as V, Work};

fn ia(a: I) -> Action {
    Action::Import(a)
}

/// The sheet over Home.
pub(crate) fn draw(app: &View, ui: &mut Ui, w: f32, h: f32) {
    let Some(imp) = app.import.as_ref() else {
        return;
    };
    let label = imp.label.clone();
    let count = app.import_count().unwrap_or_default();
    let view = app.import_view().unwrap_or_default();
    let field = app.import_field();
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
        &mut |ui, x, y, iw| match &field {
            None => picker(app, ui, x, y, iw, &label, &count, &view),
            Some(v) => unlock(app, ui, x, y, iw, &label, v),
        },
    );
}

/// The sheet's title, with its icon. Returns its height.
fn head(ui: &mut Ui, x: f32, y: f32, w: f32, icon: Glyph, title: &str) -> f32 {
    if ui.compact {
        return sheet_head(ui, x, y, w, icon, ACCENT, title);
    }
    ui.icon(x, y, 30.0, icon, 18.0, ACCENT);
    let t = ui.fit(20.0, W::S, title, w - 42.0);
    ui.text_mid(x + 42.0, y, 30.0, 20.0, W::S, TEXT, &t);
    46.0
}

/// What was read: "1 vault · 2 other files", every file with its
/// checkbox, Later and Import. Returns its height.
#[allow(clippy::too_many_arguments)]
fn picker(
    app: &View,
    ui: &mut Ui,
    x: f32,
    y: f32,
    w: f32,
    label: &str,
    c: &ImportCount,
    v: &ImportView,
) -> f32 {
    let mut cy = y;
    cy += head(
        ui,
        x,
        cy,
        w,
        app.medium.icon(),
        &fill(EN.boot_import_read_from, &[label]),
    );
    cy += ui.wrap(x, cy, w, 14.0, W::S, TEXT, &c.line()) + 16.0;
    // Unselect all while any is ticked; Select all once none is.
    let any = v.files.iter().any(|f| f.chosen);
    if v.files.iter().any(|f| f.enabled) {
        let t = if any {
            EN.visit_unselect_all
        } else {
            EN.visit_select_all
        };
        let tw = ui.measure(13.0, W::S, t);
        ui.text_mid(x, cy - 6.0, 30.0, 13.0, W::S, ACCENT, t);
        ui.hit(x - 6.0, cy - 6.0, tw + 12.0, 30.0, ia(I::All));
        cy += 30.0;
    }
    cy += files(ui, x, cy, w, v);
    cy += buttons(
        ui,
        x,
        cy,
        w,
        &[
            (EN.boot_import_later, Style::Secondary, ia(I::Later)),
            (
                EN.boot_import_import,
                if any { Style::Primary } else { Style::Disabled },
                ia(I::Go),
            ),
        ],
    );
    cy - y
}

/// A vault the import brought: its row, its passphrase field once the
/// stick is out, Cancel and Unlock. Returns its height.
#[allow(clippy::too_many_arguments)]
fn unlock(
    app: &View,
    ui: &mut Ui,
    x: f32,
    y: f32,
    w: f32,
    label: &str,
    vault: &ImportVault,
) -> f32 {
    let may = app.may_load_keys();
    let mut foot: Vec<(&str, Style, Action)> =
        vec![(EN.boot_import_cancel, Style::Secondary, ia(I::Cancel))];
    if app.vaults.working != Some(Work::Unlock) {
        foot.push((
            EN.boot_import_unlock,
            if may { Style::Primary } else { Style::Disabled },
            ia(I::Submit),
        ));
    }
    // A small panel with its keyboard up has room for the field alone.
    if ui.compact && app.osk_shown() {
        let cy = y + typing(app, ui, x, y, w, &vault.title, vault.file);
        return cy + buttons(ui, x, cy, w, &foot) - y;
    }
    let mut cy = y;
    cy += head(
        ui,
        x,
        cy,
        w,
        Icon::Download.into(),
        &fill(EN.boot_import_read_from, &[label]),
    );
    if !may {
        let pull = fill(EN.boot_import_pull_the, &[app.medium.noun()]);
        cy += stick_banner(ui, x, cy, w, &pull);
    }
    ui.icon(x - 2.0, cy + 2.0, 22.0, Icon::Lock, 12.0, MUTED);
    let name = ui.fit(14.0, W::M, &vault.title, w - 28.0);
    ui.text(x + 28.0, cy + 2.0, 14.0, W::M, TEXT, &name);
    let line = fill(
        EN.boot_import_locked,
        &[&crate::vaults::file_text(vault.len).to_string()],
    );
    let line = ui.fit(12.0, W::R, &line, w - 28.0);
    ui.text(x + 28.0, cy + 24.0, 12.0, W::R, MUTED, &line);
    cy += 52.0;
    cy += passphrase(app, ui, x, cy, w, vault.file);
    cy += buttons(ui, x, cy, w, &foot);
    cy - y
}

/// The passphrase field under vault file `file`, and what the unlock
/// says: that it is under way, or why it failed. Returns its height.
fn passphrase(app: &View, ui: &mut Ui, x: f32, y: f32, w: f32, file: usize) -> f32 {
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
fn typing(app: &View, ui: &mut Ui, x: f32, y: f32, w: f32, title: &str, file: usize) -> f32 {
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
fn unlocking(app: &View, file: usize) -> String {
    let time = app
        .vault_files()
        .get(file)
        .map(|f| {
            app.vaults
                .time_text(f.header.cost.memory_kib / 1024, f.header.cost.passes)
        })
        .unwrap_or_default();
    fill(EN.boot_import_unlocking, &[&time.to_string()])
}

/// Every file the stick held, each with its checkbox. Returns their
/// height.
fn files(ui: &mut Ui, x: f32, y: f32, w: f32, v: &ImportView) -> f32 {
    let mut cy = y;
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
    cy - y
}
