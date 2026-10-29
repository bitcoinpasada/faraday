//! Create a vault's foot: what it will make, and Create vault with the
//! Randomness row above it.

use super::*;
use crate::screens::randomness::row as own_row;

/// What Create vault will make: warnings about its cost, the file, its
/// size, memory and unlock time, and the passphrases' strength. Returns
/// its height.
pub(super) fn create_summary(app: &View, ui: &mut Ui, x: f32, y0: f32, w: f32) -> f32 {
    let (bad, warn) = (Role::Error, Role::Warning);
    let (muted, ink) = (Role::OnSurfaceVariant, Role::OnSurface);
    let Some(c) = app.vaults.create.as_ref() else {
        return 0.0;
    };
    let mut y = y0;
    let (k, mem, passes) = app.vaults.form_cost().unwrap_or((0, 64, 3));
    let (weakest, limit) = app.vaults.weakest();
    let name = if k == CUSTOM {
        EN.vault_custom
    } else {
        PRESETS[k].0
    };
    let mut warnings: Vec<(String, crate::ui::Paint)> = Vec::new();
    let slot = app
        .vaults
        .create
        .as_ref()
        .map_or(faraday_vault::DEFAULT_SLOT, |c| c.slot);
    if let Some(free) = app.vaults.memory_free_mib
        && vault::vault_need_mib(mem, slot) > free
    {
        warnings.push((
            fill(EN.vault_this_computer_n, &[&mib_text(mem).to_string()]),
            bad.paint(),
        ));
    }
    if mem > limit {
        warnings.push((
            fill(
                EN.vault_memory_mismatch,
                &[
                    name,
                    &mib_text(mem).to_string(),
                    &weakest.to_string(),
                    &mib_text(limit).to_string(),
                ],
            ),
            warn.paint(),
        ));
    }
    if app.vaults.seconds(mem, passes).is_some_and(|s| s > 10.0) {
        warnings.push((
            fill(
                EN.vault_unlocking_takes,
                &[&app.vaults.time_text(mem, passes).to_string()],
            ),
            warn.paint(),
        ));
    }
    if c.on[1..].iter().any(|on| *on) {
        warnings.push((EN.vault_raspberry_pi.to_string(), muted.paint()));
    }
    y += 4.0;
    for (t, color) in &warnings {
        // In its tone's container, never a tint (`docs/DESIGN.md` §3).
        let (bg, fg) = crate::ui::container_of(*color);
        let th = ui.wrap_as((x + 12.0, y + 10.0, w - 24.0), BODY_S, fg, t);
        ui.fill(x, y, w, th + 20.0, 8.0, bg);
        ui.wrap_as((x + 12.0, y + 10.0, w - 24.0), BODY_S, fg, t);
        y += th + 28.0;
    }
    y += 8.0;
    ui.text_as(x, y, LABEL_L, muted, EN.vault_will_create);
    y += ui.advance_as(LABEL_L, 26.0);
    let cost = vault::cost_bits(mem, passes);
    let strength = c.weakest_text(cost, app.demo_passphrase());
    let facts = [
        (
            EN.vault_file,
            format!("{}.fvlt", vault::vault_stem(&c.name.text)),
        ),
        (EN.vault_size, file_text(file_len(c.slot))),
        (EN.vault_unlock_memory, mib_text(mem)),
        (EN.vault_unlock_here, app.vaults.time_text(mem, passes)),
        (
            EN.vault_unlock_cost_adds,
            fill(EN.vault_bits, &[&format!("{cost:.1}")]),
        ),
        (
            if c.phrases.len() > 1 {
                EN.vault_weakest_passphrase
            } else {
                EN.vault_strength
            },
            strength,
        ),
        (EN.vault_goes_to_label, app.medium.for_box().to_string()),
    ];
    for (k, v) in facts.iter().filter(|(_, v)| !v.is_empty()) {
        ui.text_mid_as((x, y, 30.0), BODY_S, muted, k);
        ui.text_right_as((x + w, y, 30.0), BODY_S, ink, v);
        y += 30.0;
    }
    if let Some(e) = &c.error {
        y += 6.0;
        y += ui.wrap_as((x, y, w), BODY_S, bad, e) + 6.0;
    }
    y - y0
}

/// Create vault and the Randomness row above it, or how long creating
/// takes while it works. Returns its height.
pub(super) fn create_button(app: &View, ui: &mut Ui, x: f32, by: f32, w: f32) -> f32 {
    let (_, mem, passes) = app.vaults.form_cost().unwrap_or((0, 64, 3));
    if app.vaults.working == Some(vault::Work::Create) {
        let t = app.vaults.time_text(mem, passes).to_string();
        let line = fill(EN.vault_creating, &[&t]);
        crate::screens::working_line(ui, (x, by, w, 46.0), &line);
        return 46.0;
    }
    // Creating draws the vault's keys from the system's randomness: the
    // Randomness row above the press (`docs/OWN-RANDOMNESS.md` §8.1 row 5).
    let row = own_row(app, ui, (x, by, w));
    let style = Style::Primary.when(app.may_load_keys());
    let label = EN.vault_create_vault;
    if !crate::screens::pin_button(ui, x, by + row, Some(w), 46.0, label, style, va(V::CGo)) {
        // Pinned at the panel's foot.
        return row;
    }
    row + 46.0
}

/// The height of [`create_button`] `w` wide, drawn nowhere: the side
/// panel sets it at its foot.
pub(super) fn create_button_h(app: &View, ui: &mut Ui, w: f32) -> f32 {
    crate::compact::screen::measured(ui, |ui| create_button(app, ui, 0.0, 0.0, w))
}
