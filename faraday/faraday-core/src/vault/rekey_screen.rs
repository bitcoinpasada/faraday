//! Change passphrase and Memory on an open vault: two Material basic
//! dialogs (`docs/VAULT.md` §8). Both read the state.

use crate::Action;
use crate::drawing::View;
use crate::strings::{EN, fill};
use crate::ui::pal::*;
use crate::ui::{Style, Ui};
use crate::vault::VaultAction as V;
use crate::vault::mib_text;
use crate::vault::screen::secret_box;
use faraday_draw::tokens::{
    BODY_S, DIALOG_MAX, FIELD_H, GAP, GAP_SMALL, GRID, LABEL_L, LIST_2, LIST_PAD_V, PAD,
    RADIUS_SMALL,
};
use faraday_draw::widgets::Icon;
use faraday_vault::file::STEPS_KIB;

fn va(a: V) -> Action {
    Action::Vault(a)
}

/// **Change passphrase**: the new passphrase and the same again, **Save**
/// and **Cancel**.
pub(crate) fn passphrase_dialog(app: &View, ui: &mut Ui, w: f32, h: f32) {
    let Some(form) = app.vaults.form.as_ref() else {
        return;
    };
    let head = (
        Icon::Lock.into(),
        Role::Primary.paint(),
        EN.vault_change_passphrase,
    );
    let actions = [
        (EN.vault_cancel, Style::Secondary, va(V::FormCancel)),
        (
            EN.vault_save,
            Style::Primary.when(crate::screens::pass_pair::ready(
                form.fields[0].2.text.as_str(),
                form.fields[1].2.text.as_str(),
            )),
            va(V::FormSave),
        ),
    ];
    crate::compact::dialog(
        ui,
        (w, h, DIALOG_MAX),
        head,
        &mut |ui, x, y, iw| {
            let (first, again) = (
                form.fields[0].2.text.as_str(),
                form.fields[1].2.text.as_str(),
            );
            let mut cy = y;
            cy += crate::screens::pass_pair::pass_pair(
                ui,
                (x, cy, iw),
                (first, again),
                |ui, k, fy| {
                    let (_, label, b, _) = &form.fields[k as usize];
                    let k = k as usize;
                    let mut cy = fy;
                    ui.text_as(x, cy, BODY_S, Role::OnSurfaceVariant, label);
                    cy += ui.line_as(BODY_S) + GAP_SMALL;
                    let focused = app.vaults.focus == Some(crate::vault::Focus::Field(k));
                    secret_box(
                        ui,
                        x,
                        cy,
                        iw,
                        b,
                        app.field_shows(form, k),
                        focused,
                        va(V::FocusField(k)),
                        va(V::ShowTyped),
                    );
                    cy + FIELD_H + GAP_SMALL - fy
                },
            );
            cy + GAP - GAP_SMALL - y
        },
        &actions,
    );
}

/// **Memory**: the file's five steps, the next seal's marked, each with
/// the passes and bits it adds, the machines it is weighed for, and the
/// most the weakest of them gives a vault.
pub(crate) fn memory_dialog(app: &View, ui: &mut Ui, w: f32, h: f32) {
    let Some(id) = app.vaults.current_id() else {
        return;
    };
    let Some(next) = app.vaults.store().next_cost(id) else {
        return;
    };
    let (weakest, limit) = app.vaults.memory_weakest(id);
    let ticks = app.vaults.memory_ticks();
    let here = crate::vault::screen::here_text(app, app.vaults.memory_here_limit(id));
    let head = (Icon::Lock.into(), Role::Primary.paint(), EN.vault_memory);
    let actions = [(EN.vault_cancel, Style::Secondary, va(V::FormCancel))];
    let (muted, ink) = (Role::OnSurfaceVariant, Role::OnSurface);
    crate::compact::dialog(
        ui,
        (w, h, DIALOG_MAX),
        head,
        &mut |ui, x, y, iw| {
            let mut cy = y;
            for (k, kib) in STEPS_KIB.iter().copied().enumerate() {
                let mib = kib / 1024;
                let on = kib == next.memory_kib;
                // The same limit Create states: the weakest computer's.
                // What this computer could not unlock is not offered.
                let usable = on || app.vaults.step_refusal(id, kib).is_none();
                let fits = mib <= limit && usable;
                let (ink, muted) = (on_selected(on, ink), on_selected(on, muted));
                // Material's two-line list item: the size over its passes
                // and bits, the pair centred in the row.
                let (head, sup) = (ui.line_as(LABEL_L), ui.line_as(BODY_S.mono()));
                let rh = LIST_2.max(head + sup + 2.0 * LIST_PAD_V);
                let fill_role = selected_or(on, Role::SurfaceContainerHighest);
                ui.fill(x, cy, iw, rh, RADIUS_SMALL, fill_role);
                ui.stroke(
                    x,
                    cy,
                    iw,
                    rh,
                    RADIUS_SMALL,
                    selected_or(on, Role::OutlineVariant),
                );
                let top = cy + (rh - head - sup) / 2.0;
                ui.text_as(x + PAD, top, LABEL_L, ink, &mib_text(mib));
                let note = if on {
                    EN.vault_next_seal.to_string()
                } else if fits {
                    EN.vault_fits.to_string()
                } else if usable {
                    fill(EN.vault_too_large, &[&weakest])
                } else {
                    // Refused for this computer, ticked or not.
                    fill(EN.vault_too_large, &[EN.vaults_this_computer_name])
                };
                let color = on_selected(
                    on,
                    if on || fits {
                        muted
                    } else {
                        Role::Warning.paint()
                    },
                );
                ui.text_right_as((x + iw - PAD, top, PAD + GRID), BODY_S, color, &note);
                let detail = fill(
                    EN.vault_passes_bits,
                    &[
                        &next.passes.to_string(),
                        &format!("{:.1}", crate::vault::cost_bits(mib, next.passes)),
                    ],
                );
                let detail = ui.fit_as(BODY_S.mono(), &detail, iw - PAD - PAD);
                ui.text_as(x + PAD, top + head, BODY_S.mono(), muted, &detail);
                if usable {
                    ui.press_box(x, cy, iw, rh, va(V::MemoryStep(k)));
                }
                cy += rh + GAP_SMALL;
            }
            cy += GAP_SMALL;
            cy = crate::vault::screen::machine_rows(
                ui,
                (x, cy, iw),
                ticks,
                &here,
                (va(V::MemoryHere), V::MemoryMachine),
            );
            cy += GAP_SMALL;
            let has = fill(EN.vault_memory_limit, &[&weakest, &mib_text(limit)]);
            cy += ui.wrap_as((x, cy, iw), BODY_S, muted, &has) + GAP_SMALL;
            if next.memory_kib / 1024 > limit {
                let why = fill(
                    EN.vault_memory_mismatch,
                    &[
                        EN.vault_memory,
                        &mib_text(next.memory_kib / 1024),
                        &weakest,
                        &mib_text(limit),
                    ],
                );
                cy += ui.wrap_as((x, cy, iw), BODY_S, Role::Warning, &why) + GAP_SMALL;
            }
            cy + GAP - GAP_SMALL - y
        },
        &actions,
    );
}
