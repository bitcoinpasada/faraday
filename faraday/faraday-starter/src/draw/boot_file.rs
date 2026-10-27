//! What to write's Boot file and Confirm's row for it (`docs/STARTER.md`
//! §4.5, F133): the release's own or one the person signed, the `.efi`
//! files in Downloads to take one from, who signed the one taken, and a
//! refused one's reason by name.

use faraday_draw::widgets::Icon;
use faraday_strings::fill;
use faraday_ui::Role;

use super::{S, Ui, fact_row, grouped, label, line, mono_row, nav_row, reason_text, tick_row};
use crate::api::BootSigner;
use crate::{Starter, StarterAction};

/// The last part of a path, as a person names the file.
fn file_name(path: &str) -> &str {
    path.rsplit(['/', '\\']).next().unwrap_or(path)
}

/// The signing certificate's name and its fingerprint, as rows: the
/// name is the file's own choice, the fingerprint what is compared.
/// Returns their height.
fn signer_rows(ui: &mut Ui, x: f32, y: f32, w: f32, signer: &BootSigner) -> f32 {
    let name = if signer.subject.is_empty() {
        S.starter_boot_signer_unnamed
    } else {
        signer.subject.as_str()
    };
    let h = fact_row(ui, x, y, w, S.starter_boot_cert_name, name);
    h + mono_row(
        ui,
        x,
        y + h,
        w,
        S.starter_boot_fingerprint,
        &grouped(&signer.fingerprint),
    )
}

/// What to write's Boot file, for a stick for a PC: the two choices,
/// then for "One I signed" the one taken and who signed it, the files to
/// take one from, and why the last one was refused. Returns its height.
pub(super) fn section(starter: &Starter, ui: &mut Ui, x: f32, y0: f32, w: f32) -> f32 {
    let Some(pick) = starter.boot_pick().filter(|_| starter.boot_file_offered()) else {
        return 0.0;
    };
    let mut y = y0 + 20.0;
    y += label(ui, x, y, S.starter_boot_file);
    ui.rule(x, y, w, Role::OutlineVariant);
    y += tick_row(
        ui,
        x,
        y,
        w,
        S.starter_boot_release,
        !pick.signed,
        StarterAction::BootChoice(false),
    );
    y += tick_row(
        ui,
        x,
        y,
        w,
        S.starter_boot_signed,
        pick.signed,
        StarterAction::BootChoice(true),
    );
    if !pick.signed {
        return y - y0;
    }
    y += 12.0;
    match (&pick.chosen, &pick.checking) {
        (Some((file, signer)), _) => {
            y += signer_rows(ui, x, y, w, signer);
            y += fact_row(
                ui,
                x,
                y,
                w,
                S.starter_boot_chosen_file,
                file_name(&file.path),
            );
            y += 12.0;
        }
        (None, None) => {
            y += line(
                ui,
                x,
                y,
                w,
                S.starter_boot_signed_sub,
                Role::OnSurfaceVariant,
            );
        }
        (None, Some(_)) => {}
    }
    if let Some((path, why)) = &pick.refused {
        let s = format!("{}: {}", file_name(path), reason_text(why));
        y += line(ui, x, y, w, &s, Role::Error);
    }
    match &pick.found {
        Some((folder, files)) if files.is_empty() => {
            y += line(
                ui,
                x,
                y,
                w,
                &fill(S.starter_boot_none_in, &[folder]),
                Role::OnSurfaceVariant,
            );
        }
        Some((folder, files)) => {
            y += label(ui, x, y, &fill(S.starter_boot_in, &[folder]));
            for (n, path) in files.iter().enumerate() {
                let checking = pick.checking.as_ref() == Some(path);
                let sub = if checking {
                    S.starter_boot_checking
                } else {
                    ""
                };
                let action = (pick.checking.is_none() && n <= usize::from(u16::MAX))
                    .then_some(StarterAction::BootFile(n as u16));
                y += nav_row(ui, x, y, w, Icon::File, file_name(path), sub, "", action);
            }
        }
        None => {}
    }
    y - y0
}

/// Confirm's Boot file row, for a stick for a PC: the release's own, or
/// who signed the one taken and its certificate's fingerprint. Returns
/// its height.
pub(super) fn confirm_rows(starter: &Starter, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    if !starter.boot_file_offered() {
        return 0.0;
    }
    let chosen = starter
        .boot_pick()
        .filter(|b| b.signed)
        .and_then(|b| b.chosen.as_ref());
    match chosen {
        None => fact_row(ui, x, y, w, S.starter_boot_file, S.starter_boot_release),
        Some((_, signer)) => {
            let h = fact_row(ui, x, y, w, S.starter_boot_file, S.starter_boot_signed);
            h + signer_rows(ui, x, y + h, w, signer)
        }
    }
}
