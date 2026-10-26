//! The vault screens' sheets and panels that save and sign: the GPG
//! signature sheet, the stick save sheet and the panels under a file's
//! detail that choose how long and why (`vault/screen.rs`).

use super::*;

/// **Sign with {key}**, opened by Sign a file's **Sign** (F63): the files
/// ticked, each with its size and the start of its SHA-256; the time the
/// signatures will carry, with **Set the time**; then the hold that signs
/// and Cancel. With the clock not known the time says so and the hold is
/// disabled. Hashes a file the page has not hashed yet.
pub(crate) fn gpg_sign_sheet(app: &View, ui: &mut Ui, w: f32, h: f32) {
    let warn = Role::Warning;
    let muted = Role::OnSurfaceVariant;
    let (ink, primary) = (Role::OnSurface, Role::Primary);
    let chosen = app.gpg_chosen();
    let who = app.gpg_user_id().unwrap_or_default();
    let title = fill(EN.vault_sign_sheet_title, &[&who]);
    let now = app.clock_now();
    let when = match now {
        Some(_) => {
            let zone = format!("UTC{}", vault::tz_offset_text(app.tz_min()));
            format!("{} {zone}", app.now_local_text())
        }
        None => EN.set_time_not_known.to_string(),
    };
    let label = match chosen.len() {
        0 => EN.vault_hold_to_sign.to_string(),
        1 => EN.vault_hold_to_sign_1_file.to_string(),
        n => fill(EN.vault_hold_to_sign_files, &[&n.to_string()]),
    };
    let ready = now.is_some() && !chosen.is_empty();
    let head = (Icon::Sign.into(), primary.paint(), title.as_str());
    let frame = (w, h, faraday_draw::tokens::DIALOG_MAX);
    let cancel = [(EN.vault_cancel, Style::Ghost, Action::Cancel)];
    crate::compact::dialog(
        ui,
        frame,
        head,
        &mut |ui, x, y, iw| {
            let mut cy = y;
            for f in &chosen {
                let line = format!("{} · {}", f.name, gpg_file_size(f.size as u64));
                let line = ui.fit(14.0, W::M, &line, iw);
                ui.text(x, cy, 14.0, W::M, ink, &line);
                cy += 22.0;
                let hex: String = f.hash[..8].iter().map(|b| format!("{b:02x}")).collect();
                ui.text(x, cy, 12.0, W::M, muted, &format!("{hex}…"));
                cy += 22.0;
                ui.rule(x, cy, iw, Role::OutlineVariant);
                cy += 10.0;
            }
            // The time the signatures will carry, and the way to change it.
            ui.text(x, cy, 12.0, W::R, muted, EN.vault_signature_time);
            cy += 20.0;
            let set_w = ui.measure(13.0, W::S, EN.vault_set_the_time) + 30.0;
            let tone = if now.is_some() { ink } else { warn };
            if ui.compact {
                cy += ui.wrap(x, cy, iw, 14.0, W::M, tone, &when) + 8.0;
                ui.button(
                    x,
                    cy,
                    Some(set_w),
                    36.0,
                    EN.vault_set_the_time,
                    Style::Secondary,
                    Action::OpenSetTime,
                );
                cy += 46.0;
            } else {
                let t = ui.fit(14.0, W::M, &when, iw - set_w - 12.0);
                ui.text_mid(x, cy, 36.0, 14.0, W::M, tone, &t);
                ui.button(
                    x + iw - set_w,
                    cy,
                    Some(set_w),
                    36.0,
                    EN.vault_set_the_time,
                    Style::Secondary,
                    Action::OpenSetTime,
                );
                cy += 46.0;
            }
            ui.rule(x, cy, iw, Role::OutlineVariant);
            cy += 14.0;
            // The hold signs; Cancel is the dialog's action.
            if ready {
                hold_button_in(
                    app,
                    ui,
                    x,
                    cy,
                    iw,
                    &label,
                    va(V::GpgHoldSign),
                    primary.paint(),
                );
            } else {
                ui.button(
                    x,
                    cy,
                    None,
                    36.0,
                    &label,
                    Style::Disabled,
                    va(V::GpgHoldSign),
                );
            }
            cy += 36.0;
            cy - y
        },
        &cancel,
    );
}

/// Save to a stick's popup (`docs/BACKUP-FLOW.md` §17.7): for a key,
/// the line that says what the stick gives whoever finds it, first; then
/// each form with its files' names, ticked or not; Save and Cancel.
pub(crate) fn stick_save_sheet(app: &View, ui: &mut Ui, w: f32, h: f32) {
    let (muted, warn) = (Role::OnSurfaceVariant, Role::Warning);
    let (ink, primary) = (Role::OnSurface, Role::Primary);
    let Some(s) = app.vaults.stick_save.as_ref() else {
        return;
    };
    let any = s.files.iter().any(|f| f.ticked);
    let items: Vec<(&str, Style, Action)> = vec![
        (EN.vault_cancel, Style::Ghost, Action::Cancel),
        (
            EN.vault_save,
            Style::Primary.when(any),
            va(V::SaveToStickGo),
        ),
    ];
    let body = |ui: &mut Ui, x: f32, y: f32, iw: f32| -> f32 {
        let mut cy = y;
        if s.seed {
            cy += ui.wrap(x, cy, iw, 13.0, W::S, warn, EN.vault_anyone_who_finds) + 14.0;
        }
        for (k, f) in s.files.iter().enumerate() {
            let top = cy;
            ui.checkbox(x, cy + 11.0, f.ticked, true);
            ui.text_mid(x + 30.0, cy, 40.0, 14.0, W::S, ink, &f.label);
            cy += 34.0;
            for n in &f.names {
                let n = ui.fit(12.0, W::M, n, iw - 30.0);
                ui.text(x + 30.0, cy, 12.0, W::M, muted, &n);
                cy += 18.0;
            }
            cy += 6.0;
            ui.hit(x, top, iw, cy - top, va(V::SaveTick(k as u8)));
            ui.rule(x, cy, iw, Role::OutlineVariant);
            cy += 6.0;
        }
        cy - y
    };
    let head = (app.medium.icon(), primary.paint(), EN.vault_save_to_a_stick);
    crate::compact::dialog(
        ui,
        (w, h, faraday_draw::tokens::DIALOG_MAX),
        head,
        &mut |ui, x, y, iw| body(ui, x, y, iw),
        &items,
    );
}

/// The session's keys or wallets that are not yet in this vault, each
/// with Save.
#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
pub(super) fn save_panel(
    app: &View,
    ui: &mut Ui,
    v: &vault::OpenVault,
    dx: f32,
    mut dy: f32,
    dw: f32,
) -> f32 {
    let (muted, ink) = (Role::OnSurfaceVariant, Role::OnSurface);
    let keys = app.vaults.category == 0;
    ui.text(
        dx,
        dy,
        18.0,
        W::S,
        ink,
        if keys {
            EN.vault_save_a_key
        } else {
            EN.vault_save_a_wallet
        },
    );
    dy += 40.0;
    let mut any = false;
    if keys {
        let held: Vec<String> = app
            .vaults
            .records(v.id)
            .of(kind::KEY)
            .filter_map(|(_, r)| app.vault_key_fingerprint(r))
            .collect();
        for (k, key) in app.session.keys.all().enumerate() {
            let fp = fp_text(key.fingerprint);
            // Any key loaded is offered: words, or the master seed a
            // codex32 or SLIP-39 key was loaded from (VAULT.md §7, type
            // 2's seed payload).
            if held.contains(&fp) || !(key.has_words() || app.session.keys.holds_seed_bytes(k)) {
                continue;
            }
            any = true;
            ui.text_mid(dx, dy, 40.0, 14.0, W::M, ink, &fp);
            let label = if key.has_passphrase {
                fill(EN.vault_bip_39_passphrase_n, &[&key.label.to_string()])
            } else {
                key.label.clone()
            };
            if ui.compact {
                // The key and its name, then its buttons under them.
                let label = ui.fit(13.0, W::R, &label, dw - 100.0);
                ui.text_mid(dx + 100.0, dy, 40.0, 13.0, W::R, muted, &label);
                dy += 42.0;
                let mut items = vec![(EN.vault_save, Style::Secondary, va(V::SaveKey(k)))];
                if key.has_passphrase {
                    items.push((
                        EN.vault_save_passphrase_button,
                        Style::Secondary,
                        va(V::SaveKeyWithPassphrase(k)),
                    ));
                }
                dy += crate::screens::wrap_buttons(ui, dx, dy, dw, 36.0, &items);
                ui.rule(dx, dy - 2.0, dw, Role::OutlineVariant);
                dy += 6.0;
                continue;
            }
            let two = key.has_passphrase;
            let room = if two { dw - 110.0 } else { dw - 200.0 };
            let label = ui.fit(13.0, W::R, &label, room);
            ui.text_mid(dx + 100.0, dy, 40.0, 13.0, W::R, muted, &label);
            if two {
                dy += 40.0;
            }
            ui.button(
                dx + dw - 80.0,
                dy + 2.0,
                Some(80.0),
                36.0,
                EN.vault_save,
                Style::Secondary,
                va(V::SaveKey(k)),
            );
            if key.has_passphrase {
                let pw = ui.measure(13.0, W::S, EN.vault_save_passphrase_button) + 26.0;
                ui.button(
                    dx + dw - 88.0 - pw,
                    dy + 2.0,
                    Some(pw),
                    36.0,
                    EN.vault_save_passphrase_button,
                    Style::Secondary,
                    va(V::SaveKeyWithPassphrase(k)),
                );
            }
            ui.rule(dx, dy + 44.0, dw, Role::OutlineVariant);
            dy += 48.0;
        }
        if !any {
            ui.text(dx, dy, 13.0, W::R, muted, EN.vault_every_key_saved);
            dy += 30.0;
        }
    } else {
        let held: Vec<String> = app
            .described_of(v.id, kind::WALLET)
            .iter()
            .map(|(_, _, d)| d.policy.to_descriptor())
            .collect();
        for (k, wl) in app.session.wallets.iter().enumerate() {
            if held.contains(&wl.policy().to_descriptor()) {
                continue;
            }
            any = true;
            if ui.compact {
                // The name over its shape, Save beside them.
                let name = ui.fit(14.0, W::S, &wl.name, dw - 96.0);
                ui.text(dx, dy + 2.0, 14.0, W::S, ink, &name);
                let shape = ui.fit(12.0, W::R, &Session::shape(wl), dw - 96.0);
                ui.text(dx, dy + 24.0, 12.0, W::R, muted, &shape);
                ui.button(
                    dx + dw - 80.0,
                    dy + 4.0,
                    Some(80.0),
                    36.0,
                    EN.vault_save,
                    Style::Secondary,
                    va(V::SaveWallet(k)),
                );
                ui.rule(dx, dy + 48.0, dw, Role::OutlineVariant);
                dy += 52.0;
                continue;
            }
            ui.text_mid(dx, dy, 40.0, 14.0, W::S, ink, &wl.name);
            let shape = ui.fit(12.0, W::R, &Session::shape(wl), dw - 260.0);
            ui.text_mid(dx + 150.0, dy, 40.0, 12.0, W::R, muted, &shape);
            ui.button(
                dx + dw - 80.0,
                dy + 2.0,
                Some(80.0),
                36.0,
                EN.vault_save,
                Style::Secondary,
                va(V::SaveWallet(k)),
            );
            ui.rule(dx, dy + 44.0, dw, Role::OutlineVariant);
            dy += 48.0;
        }
        if !any {
            ui.text(dx, dy, 13.0, W::R, muted, EN.vault_every_wallet);
            dy += 30.0;
        }
    }
    dy
}

/// The expiry choices for a GPG key, `chosen` highlighted. Returns the
/// row's height.
pub(crate) fn years_row(
    ui: &mut Ui,
    x: f32,
    y: f32,
    w: f32,
    chosen: Option<u32>,
    act: fn(u32) -> V,
) -> f32 {
    let items: Vec<(&str, Style, Action)> = [
        (EN.vault_1_year, 1u32),
        (EN.vault_2_years, 2),
        (EN.vault_3_years, 3),
        (EN.vault_5_years, 5),
        (EN.vault_never, 0),
    ]
    .into_iter()
    .map(|(label, years)| {
        let style = if chosen == Some(years) {
            Style::Primary
        } else {
            Style::Secondary
        };
        (label, style, va(act(years)))
    })
    .collect();
    crate::screens::wrap_buttons(ui, x, y, w, 32.0, &items) - 8.0
}

/// GnuPG's reasons for a revocation, in its menu's order. Returns the
/// row's height.
pub(super) fn reasons_row(app: &View, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    let items: Vec<(&str, Style, Action)> = [
        EN.vault_reason_0,
        EN.vault_reason_1,
        EN.vault_reason_2,
        EN.vault_reason_3,
    ]
    .into_iter()
    .enumerate()
    .map(|(n, label)| {
        let style = if usize::from(app.vaults.revoke_reason) == n {
            Style::Primary
        } else {
            Style::Secondary
        };
        (label, style, va(V::GpgReason(n as u8)))
    })
    .collect();
    crate::screens::wrap_buttons(ui, x, y, w, 32.0, &items) - 8.0
}

/// The Inbox's EFI images, each to sign or check with the selected db key;
/// or, once one is chosen to sign, its SHA-256 to compare with the build
/// record before the hold that signs it (`docs/history/PLAN.md` §8).
#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
pub(super) fn images_panel(app: &View, ui: &mut Ui, dx: f32, mut dy: f32, dw: f32) -> f32 {
    let primary = Role::Primary;
    let (muted, ink) = (Role::OnSurfaceVariant, Role::OnSurface);
    if let Some(k) = app.vaults.sb_sign
        && let Some(item) = app.inbox.get(k)
    {
        ui.text(
            dx,
            dy,
            18.0,
            W::S,
            ink,
            &fill(EN.vault_sign_n, &[&item.name.to_string()]),
        );
        dy += 40.0;
        ui.text(dx, dy, 12.0, W::R, muted, "SHA-256");
        dy += 20.0;
        // Taken when the image came in (`Item::sha`): a lookup.
        let hash = item
            .sha
            .map(|s| crate::secure_boot::image_sha256(&s))
            .unwrap_or_default();
        dy += ui.wrap(dx, dy, dw, 15.0, W::M, ink, &hash) + 14.0;
        dy += ui.wrap(dx, dy, dw, 13.0, W::R, muted, EN.vault_compare_build_record) + 16.0;
        let hw = hold_button(
            app,
            ui,
            dx,
            dy,
            EN.vault_hold_to_sign,
            va(V::SbHoldSign),
            primary.paint(),
        );
        ui.button(
            dx + hw + 10.0,
            dy,
            None,
            36.0,
            EN.vault_back,
            Style::Ghost,
            va(V::SbImages),
        );
        return dy + 48.0;
    }
    ui.text(dx, dy, 18.0, W::S, ink, EN.vault_sign_or_check);
    dy += 40.0;
    let images: Vec<(usize, &crate::Item)> = app
        .inbox
        .iter()
        .enumerate()
        .filter(|(_, i)| crate::secure_boot::is_image(&i.name))
        .collect();
    if images.is_empty() {
        ui.text(
            dx,
            dy,
            13.0,
            W::R,
            muted,
            &fill(EN.vault_no_efi_image, &[app.medium.from_the()]),
        );
        dy += 30.0;
    }
    for (k, it) in images {
        // On a small panel the name has a line of its own.
        let room = if ui.compact { dw } else { dw - 200.0 };
        let name = ui.fit(14.0, W::M, &it.name, room);
        ui.text_mid(dx, dy, 40.0, 14.0, W::M, ink, &name);
        if ui.compact {
            dy += 36.0;
        }
        if app.selected_waits().is_some() {
            let bw = ui.measure(13.0, W::S, EN.backup_flow_first) + 30.0;
            ui.button(
                dx + dw - 168.0 - bw - 8.0,
                dy + 2.0,
                Some(bw),
                36.0,
                EN.backup_flow_first,
                Style::Secondary,
                back_up_first(crate::backup_flow::Kind::SecureBoot),
            );
        } else {
            ui.button(
                dx + dw - 80.0,
                dy + 2.0,
                Some(80.0),
                36.0,
                EN.vault_sign,
                Style::Secondary,
                va(V::SbPick(k)),
            );
        }
        ui.button(
            dx + dw - 168.0,
            dy + 2.0,
            Some(80.0),
            36.0,
            EN.vault_check,
            Style::Secondary,
            va(V::SbCheck(k)),
        );
        ui.rule(dx, dy + 44.0, dw, Role::OutlineVariant);
        dy += 48.0;
    }
    if let Some(said) = &app.vaults.sb_checked {
        dy += 6.0;
        dy += ui.wrap(dx, dy, dw, 13.0, W::S, ink, said) + 10.0;
    }
    ui.button(
        dx,
        dy + 8.0,
        None,
        36.0,
        EN.vault_back_to_the_keys,
        Style::Ghost,
        va(V::SbImages),
    );
    dy + 56.0
}
