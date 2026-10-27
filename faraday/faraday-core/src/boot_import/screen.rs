//! The boot import's sheet (`boot_import`, F60, F130): the boot stick's
//! files by name and size, each with its checkbox, with Later and Import;
//! while Import reads the ticked ones, how many are read; after, each
//! vault it brought in turn with its passphrase field, once the stick is
//! out, and Cancel.

use crate::drawing::View;
use crate::strings::EN;
use crate::strings::fill;
use faraday_draw::tokens::{BODY_M, BODY_S, GAP, HEADLINE_S, LABEL_L, LABEL_M, PROGRESS_H};
use faraday_draw::widgets::Icon;

use crate::Action;
use crate::boot_import::{ImportAction as I, ImportCount, ImportVault, ImportView};
use crate::compact::{buttons, scroll_sheet, sheet_head};
use crate::ui::pal::*;
use crate::ui::{Glyph, Style, Ui};
use crate::vault::screen::{stick_banner, stick_field};
use crate::vault::{Focus, VaultAction as V, Work};

fn ia(a: I) -> Action {
    Action::Import(a)
}

/// The sheet over Home.
pub(crate) fn draw(app: &View, ui: &mut Ui, w: f32, h: f32) {
    let Some(imp) = app.import.state() else {
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
        return sheet_head(ui, x, y, w, icon, Role::Primary, title);
    }
    ui.icon(x, y, 30.0, icon, 18.0, Role::Primary);
    let t = ui.fit_as(HEADLINE_S.medium(), title, w - 42.0);
    ui.text_mid_as(
        (x + 42.0, y, 30.0),
        HEADLINE_S.medium(),
        Role::OnSurface,
        &t,
    );
    46.0
}

/// The stick's files: "1 vault · 2 other files, by their names", or
/// while Import reads, "3 of 7 files read"; every file with its checkbox,
/// Later and Import. Returns its height.
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
        &fill(EN.boot_import_read_source, &[label]),
    );
    if app.import_reading() {
        // The ticked files being read, Import pressed: a count, so
        // Material's determinate indicator under it.
        cy += ui.wrap_as((x, cy, w), LABEL_L, Role::OnSurface, &c.line()) + GAP;
        let done = (c.asked - c.reading) as f32 / c.asked.max(1) as f32;
        ui.progress((x, cy, w), Some(done), None);
        cy += PROGRESS_H + GAP;
    } else {
        cy += ui.wrap_as((x, cy, w), LABEL_L, Role::OnSurface, &c.line()) + 16.0;
        // Nothing is read until Import, and Import reads the stick.
        if !app.import_stick_in() {
            let back = fill(EN.boot_import_put_back, &[app.medium.noun()]);
            cy += stick_banner(ui, x, cy, w, &back);
        }
    }
    // Unselect all while any is ticked; Select all once none is.
    let any = v.files.iter().any(|f| f.chosen);
    if v.files.iter().any(|f| f.enabled) {
        let t = if any {
            EN.visit_unselect_all
        } else {
            EN.visit_select_all
        };
        let tw = ui.measure_as(LABEL_L, t);
        ui.text_mid_as((x, cy - 6.0, 30.0), LABEL_L, Role::Primary, t);
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
                Style::Primary.when(app.import_may_go()),
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
            Style::Primary.when(may),
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
        &fill(EN.boot_import_read_source, &[label]),
    );
    if !may {
        let pull = fill(EN.boot_import_pull_medium, &[app.medium.noun()]);
        cy += stick_banner(ui, x, cy, w, &pull);
    }
    ui.icon(
        x - 2.0,
        cy + 2.0,
        22.0,
        Icon::Lock,
        12.0,
        Role::OnSurfaceVariant,
    );
    let name = ui.fit_as(BODY_M.mono(), &vault.title, w - 28.0);
    ui.text_as(x + 28.0, cy + 2.0, BODY_M.mono(), Role::OnSurface, &name);
    let line = fill(
        EN.boot_import_locked,
        &[&crate::vault::file_text(vault.len).to_string()],
    );
    let line = ui.fit_as(BODY_S, &line, w - 28.0);
    ui.text_as(x + 28.0, cy + 24.0, BODY_S, Role::OnSurfaceVariant, &line);
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
        crate::demo::pass_box(
            app,
            ui,
            (x, cy, w),
            &app.vaults.passphrase,
            (
                app.vaults.typed_shown,
                app.vaults.focus == Some(Focus::Passphrase),
            ),
            (
                Action::Vault(V::FocusPassphrase),
                Action::Vault(V::ShowTyped),
            ),
            crate::demo::PassField::Unlock,
        );
    } else {
        stick_field(app.medium, ui, x, cy, w);
    }
    cy += 50.0 + crate::screens::never_under(app, ui, x, cy, w);
    if app.vaults.working == Some(Work::Unlock) {
        let lh = ui.wrap_as((x, cy, w), LABEL_L, Role::Primary, &unlocking(app, file)) + 10.0;
        crate::screens::working_bar(ui, x, cy, w, lh);
        cy += lh;
    }
    if let Some(e) = &app.vaults.unlock_error {
        cy += ui.wrap_as((x, cy, w), BODY_S, Role::Error, e) + 10.0;
    }
    cy + 4.0 - y
}

/// The field alone, for a small panel with its keyboard up: over it, the
/// vault's name, or that the unlock is under way, or why it failed.
/// Returns its height.
fn typing(app: &View, ui: &mut Ui, x: f32, y: f32, w: f32, title: &str, file: usize) -> f32 {
    let mut cy = y;
    let (line, tone) = if app.vaults.working == Some(Work::Unlock) {
        (unlocking(app, file), Role::Primary)
    } else if let Some(e) = &app.vaults.unlock_error {
        (e.clone(), Role::Error)
    } else {
        (title.to_string(), Role::OnSurfaceVariant)
    };
    cy += ui.wrap_as((x, cy, w), LABEL_M, tone, &line) + 6.0;
    crate::demo::pass_box(
        app,
        ui,
        (x, cy, w),
        &app.vaults.passphrase,
        (
            app.vaults.typed_shown,
            app.vaults.focus == Some(Focus::Passphrase),
        ),
        (
            Action::Vault(V::FocusPassphrase),
            Action::Vault(V::ShowTyped),
        ),
        crate::demo::PassField::Unlock,
    );
    cy += 44.0 + crate::screens::never_under(app, ui, x, cy, w);
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
        let fg = if f.enabled {
            Role::OnSurface.paint()
        } else {
            crate::ui::off()
        };
        let name = ui.fit_as(BODY_M.mono(), &f.name, tw);
        ui.text_as(tx, cy + 2.0, BODY_M.mono(), fg, &name);
        cy += 24.0;
        cy += ui.wrap_as(
            (tx, cy, tw),
            BODY_S,
            if f.enabled {
                Role::OnSurfaceVariant.paint()
            } else {
                crate::ui::off()
            },
            &f.line,
        ) + 2.0;
        if f.enabled {
            ui.hit(x - 4.0, top, w + 8.0, cy - top, ia(I::File(i)));
        }
        cy += 10.0;
    }
    cy - y
}
