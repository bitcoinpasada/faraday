//! The boot import's sheet (`boot_import`, F60, F130): the boot stick's
//! files by name and size, each with its checkbox, with Later and Import;
//! while Import reads the ticked ones, how many are read; after, each
//! vault it brought in turn with its passphrase field, once the stick is
//! out, and Cancel.

use crate::drawing::View;
use crate::strings::EN;
use crate::strings::fill;
use faraday_draw::tokens::{
    BODY_M, BODY_S, FIELD_H, GAP, HEADLINE_S, LABEL_L, LABEL_M, PROGRESS_H,
};
use faraday_draw::widgets::Icon;

use crate::Action;
use crate::boot_import::{ImportAction as I, ImportCount, ImportVault, ImportView};
use crate::compact::{buttons, scroll_sheet, sheet_head};
use crate::ui::pal::*;
use crate::ui::{Glyph, Style, Ui};
use crate::vault::screen::{stick_banner, stick_field};
use crate::vault::{Focus, VaultAction as V, Work};
use faraday_draw::tokens::{
    GRID, IMPORT_TILE_FOOT_GRIDS, IMPORT_TILE_ICON, IMPORT_TILE_ICON_BOX, IMPORT_TILE_MIN_ROWS,
    IMPORT_TILE_TITLE_X, ROW, SHAPE_M, TITLE_M, VISIT_CARD_HEAD, VISIT_PAD,
};

fn ia(a: I) -> Action {
    Action::Import(a)
}

/// The import's vault step: the sheet over Home.
pub(crate) fn draw(app: &View, ui: &mut Ui, w: f32, h: f32) {
    let Some(imp) = app.import.state() else {
        return;
    };
    let label = imp.label.clone();
    // The vault step only: the list is the tile on Files.
    let Some(field) = app.import_field() else {
        return;
    };
    let place = if ui.compact {
        ((8.0, w - 16.0), 8.0, 16.0)
    } else {
        let sw = (w - 48.0).min(640.0);
        (((w - sw) / 2.0, sw), 40.0, 32.0)
    };
    scroll_sheet(ui, place.0, h, place.1, place.2, &mut |ui, x, y, iw| {
        unlock(app, ui, x, y, iw, &label, &field)
    });
}

/// The list step on Files, where the stick's own card stands: the boot
/// stick's files by name and size with their ticks, a count line, Later
/// and Import at the foot. The stick in: that it is to be removed once
/// the files are in, and that a key from it loads as it is pulled. The
/// stick out: Put the stick back in to import. Nothing is read before
/// Import.
pub(crate) fn tile(
    app: &View,
    ui: &mut Ui,
    (x, y, w, h): (f32, f32, f32, f32),
    (count, view): &(ImportCount, ImportView),
) -> f32 {
    let Some(imp) = app.import.state() else {
        return 0.0;
    };
    let label = imp.label.clone();
    ui.card(x, y, w, h, Role::OutlineVariant);
    let (ix, iw) = (x + VISIT_PAD, w - 2.0 * VISIT_PAD);
    let noun = app.medium.noun();
    let mut cy = y;
    if ui.compact {
        // A small panel's tab names the card.
        cy += SHAPE_M + faraday_draw::tokens::GAP_SMALL;
    } else {
        ui.icon(
            ix,
            y + VISIT_PAD,
            IMPORT_TILE_ICON_BOX,
            app.medium.icon(),
            IMPORT_TILE_ICON,
            Role::Primary,
        );
        let t = ui.fit_as(
            TITLE_M,
            &fill(EN.boot_import_read_source, &[&label]),
            iw - IMPORT_TILE_TITLE_X,
        );
        ui.text_mid_as(
            (
                ix + IMPORT_TILE_TITLE_X,
                y + VISIT_PAD,
                IMPORT_TILE_ICON_BOX,
            ),
            TITLE_M,
            Role::OnSurface,
            &t,
        );
        cy += VISIT_CARD_HEAD;
    }
    if app.import_reading() {
        cy += ui.wrap_as((ix, cy, iw), LABEL_L, Role::OnSurface, &count.line()) + GAP;
        let done = (count.asked - count.reading) as f32 / count.asked.max(1) as f32;
        ui.progress((ix, cy, iw), Some(done), None);
        cy += PROGRESS_H + GAP;
    } else {
        cy += ui.wrap_as((ix, cy, iw), LABEL_L, Role::OnSurface, &count.line()) + GRID;
        if app.import_stick_in() {
            // After a secret was held or typed Import stays off.
            let unclean = (!app.clean_now()).then_some(EN.boot_import_not_clean);
            for line in [
                EN.boot_import_remove_when_in,
                EN.boot_import_key_loads_at_pull,
            ]
            .into_iter()
            .chain(unclean)
            {
                cy += ui.wrap_as(
                    (ix, cy, iw),
                    BODY_S,
                    Role::OnSurfaceVariant,
                    &fill(line, &[noun]),
                ) + GRID;
            }
        } else {
            let back = fill(EN.boot_import_put_back, &[app.medium.started_from()]);
            cy += stick_banner(ui, ix, cy, iw, &back);
        }
    }
    cy += select_all(ui, ix, cy, view);
    // Later and Import at the foot; the files scroll between.
    // Their height is the buttons' own, by the rule that stacks them, over
    // the longer words whichever are drawn, so ticking moves nothing.
    let longer = if ui.measure_as(LABEL_L, EN.boot_import_import_keys)
        > ui.measure_as(LABEL_L, EN.boot_import_import)
    {
        EN.boot_import_import_keys
    } else {
        EN.boot_import_import
    };
    let feet = faraday_ui::sheet::buttons_height(ui, iw, &[EN.boot_import_later, longer]);
    let foot = y + h - VISIT_PAD - feet;
    // The "more" line the list bar draws stands under the track.
    let track_h = (foot - GRID * IMPORT_TILE_FOOT_GRIDS - cy).max(ROW);
    let list_top = cy;
    // What the card needs to show three files and both buttons.
    let needed = (list_top - y)
        + IMPORT_TILE_MIN_ROWS * ROW
        + GRID * IMPORT_TILE_FOOT_GRIDS
        + feet
        + VISIT_PAD;
    let clip = ui.rect(x, list_top, w, track_h);
    let total = {
        let zero = ui.rect(0.0, 0.0, 0.0, 0.0);
        ui.c.push_clip(zero);
        let total = files(ui, ix, list_top, iw, view);
        ui.c.pop_clip();
        total
    };
    let max_shift = (total - track_h).max(0.0);
    let shift = app.scroll.visit_in_offset().min(max_shift);
    let open = ui.region_open(
        crate::ui::Region::beside(crate::ui::Slot::VisitIn, clip, app.scroll.visit_in_offset())
            .on(Role::SurfaceContainerLow)
            .own_bar()
            .from_zero(shift),
    );
    files(ui, ix, list_top, iw, view);
    ui.region_close(open, max_shift);
    crate::screens::visit::visit_list_bar(
        app,
        ui,
        crate::Column::Stick,
        (x, w),
        list_top,
        track_h,
        total,
        shift,
    );
    faraday_ui::sheet::buttons_by(
        ui,
        ix,
        foot,
        iw,
        &[
            (EN.boot_import_later, Style::Secondary, ia(I::Later)),
            (
                if app.import_may_load_keys() {
                    EN.boot_import_import_keys
                } else {
                    EN.boot_import_import
                },
                Style::Primary.when(app.import_may_go()),
                ia(I::Go),
            ),
        ],
        &[EN.boot_import_later, longer],
    );
    needed
}

/// Unselect all while any is ticked, Select all once none is, at (x, y):
/// the height it took, none when no file can be ticked.
fn select_all(ui: &mut Ui, x: f32, y: f32, v: &ImportView) -> f32 {
    if !v.files.iter().any(|f| f.enabled) {
        return 0.0;
    }
    let any = v.files.iter().any(|f| f.chosen);
    let t = if any {
        EN.visit_unselect_all
    } else {
        EN.visit_select_all
    };
    let tw = ui.measure_as(LABEL_L, t);
    let row = ui.touch;
    ui.text_mid_as((x, y - 6.0, row), LABEL_L, Role::Primary, t);
    ui.press_box(x - 6.0, y - 6.0, tw + 12.0, row, ia(I::All));
    row
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

/// A vault the import brought: its row, its passphrase field once the
/// stick is out, Cancel and Unlock. Returns its height.
#[allow(
    clippy::too_many_arguments,
    reason = "the unlock card's view, position, width, label and the vault being imported"
)]
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
        let pull = fill(EN.boot_import_pull_medium, &[app.medium.started_from()]);
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
        &[&crate::vault::file_text(vault.len)],
    );
    let line = ui.fit_as(BODY_S, &line, w - 28.0);
    ui.text_as(x + 28.0, cy + 24.0, BODY_S, Role::OnSurfaceVariant, &line);
    cy += ui.advance_as(BODY_S, 52.0);
    cy += passphrase(app, ui, x, cy, w, vault.file);
    cy += buttons(ui, x, cy, w, &foot);
    cy - y
}

/// The passphrase field under vault file `file`, and what the unlock
/// says: that it is under way, or why it failed. Returns its height.
fn passphrase(app: &View, ui: &mut Ui, x: f32, y: f32, w: f32, file: usize) -> f32 {
    let mut cy = y;
    if app.may_load_keys() {
        crate::pass_field::screen::vault_box(
            app,
            ui,
            (x, cy, w),
            (&app.vaults.passphrase, crate::pass_field::Which::First),
            (
                Action::Vault(V::FocusPassphrase),
                app.vaults.focus == Some(Focus::Passphrase),
            ),
            crate::demo::PassField::Unlock,
        );
    } else {
        stick_field(app.medium, ui, x, cy, w);
    }
    cy +=
        FIELD_H + 10.0 + crate::screens::never_real_seed_if(app.may_load_keys(), app, ui, x, cy, w);
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
        (String::from(e.as_str()), Role::Error)
    } else {
        (title.to_string(), Role::OnSurfaceVariant)
    };
    cy += ui.wrap_as((x, cy, w), LABEL_M, tone, &line) + 6.0;
    crate::pass_field::screen::vault_box(
        app,
        ui,
        (x, cy, w),
        (&app.vaults.passphrase, crate::pass_field::Which::First),
        (
            Action::Vault(V::FocusPassphrase),
            app.vaults.focus == Some(Focus::Passphrase),
        ),
        crate::demo::PassField::Unlock,
    );
    cy += FIELD_H + crate::screens::never_under(app, ui, x, cy, w);
    cy - y
}

/// "Unlocking · 2 s": the unlock of vault file `file` under way.
fn unlocking(app: &View, file: usize) -> String {
    let time = app
        .vault_files()
        .get(file)
        .map(|f| {
            app.vaults.time_text(
                f.facts.header.cost.memory_kib / 1024,
                f.facts.header.cost.passes,
            )
        })
        .unwrap_or_default();
    fill(EN.boot_import_unlocking, &[&time])
}

/// Every file the stick held, each with its checkbox. Returns their
/// height.
fn files(ui: &mut Ui, x: f32, y: f32, w: f32, v: &ImportView) -> f32 {
    let mut cy = y;
    for f in &v.files {
        let top = cy;
        let tw = w - 28.0;
        let height = ui.line_as(BODY_M.mono())
            + faraday_draw::tokens::GAP_SMALL
            + ui.wrap_height_as(BODY_S, &f.line, tw)
            + 2.0
            + 10.0;
        // A row out of sight is passed over, its height alone counted.
        if !ui.shows(top, height) {
            cy += height;
            continue;
        }
        ui.checkbox(x, cy + 4.0, f.chosen, f.enabled);
        let tx = x + 28.0;
        let fg = if f.enabled {
            Role::OnSurface.paint()
        } else {
            crate::ui::off()
        };
        let name = ui.fit_as(BODY_M.mono(), &f.name, tw);
        ui.text_as(tx, cy + 2.0, BODY_M.mono(), fg, &name);
        cy += ui.line_as(BODY_M.mono()) + faraday_draw::tokens::GAP_SMALL;
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
            ui.press_box(x - 4.0, top, w + 8.0, cy - top, ia(I::File(f.id)));
        }
        cy += 10.0;
    }
    cy - y
}
