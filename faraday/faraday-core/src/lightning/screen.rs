//! The Lightning node key screen.

use crate::Action;
use crate::drawing::View;
use crate::lightning::NodeSource;
use crate::screens::{button_rows, guide_text_learn};
use crate::strings::EN;
use crate::strings::fill;
use crate::ui::pal::*;
use crate::ui::{Style, Ui, W};
use faraday_draw::tokens as T;
use faraday_draw::tokens::{BODY_M, BODY_S, BUTTON_H};

pub(crate) fn draw(app: &View, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    if app.lightning.state().is_none() {
        return;
    }
    // A small panel has its bar for the way back and the name, and puts
    // each value under its name.
    let compact = ui.compact;
    let (x, width) = if compact {
        (x0 + crate::compact::M, cw - 2.0 * crate::compact::M)
    } else {
        (x0 + 56.0, (cw - 112.0).min(900.0))
    };
    let (vx, vw) = if compact {
        (x, width)
    } else {
        (x + 120.0, width - 120.0)
    };
    let below = if compact { 20.0 } else { 0.0 };
    let top = 12.0 - app.list_offset;
    let guide = EN.lightning_a_lightning_node;
    let y = if compact {
        top
    } else {
        header(app, ui, x, width, guide)
    };
    let end = body(app, ui, x, y, width, (vx, vw, below));
    if compact {
        let y = end
            + crate::compact::screen::about_learn(
                ui,
                x,
                end,
                width,
                guide,
                Some(crate::learn::Topic::Tools),
            );
        crate::compact::screen::finish(app, ui, x0, cw, h, y - top + 16.0);
    }
}

/// The way back and the name, handed to the top app bar, and what
/// explains the page, as the desktop draws them. Returns where the page
/// goes on.
fn header(app: &View, ui: &mut Ui, x: f32, width: f32, guide: &str) -> f32 {
    let Some(l) = app.lightning.state() else {
        return 0.0;
    };
    let back = app.named_back((EN.lightning_wallets, Action::Nav(l.back)));
    let y = crate::screens::bar_head(ui, Some(back), EN.lightning_lightning_node);
    y + guide_text_learn(app, ui, x, y, width, guide, crate::learn::Topic::Tools)
}

/// The sources, the aezeed's words, and the node. `vx`, `vw` are where
/// values go; `below` is how far a value sits under its name. Returns
/// where it ends.
#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
fn body(
    app: &View,
    ui: &mut Ui,
    x: f32,
    mut y: f32,
    width: f32,
    (vx, vw, below): (f32, f32, f32),
) -> f32 {
    let Some(l) = app.lightning.state() else {
        return y;
    };
    let mut row: Vec<(String, Style, Action)> = app
        .session
        .keys
        .all()
        .map(|k| {
            let fp = k.fingerprint;
            (
                fill(
                    EN.lightning_ldk_node,
                    &[&crate::wallet::key_line(fp, &k.label).to_string()],
                ),
                Style::chosen(l.source == Some(NodeSource::Key(fp.0))),
                Action::LKey(fp.0),
            )
        })
        .collect();
    row.push((
        EN.lightning_lnd_type.to_string(),
        Style::chosen(l.source == Some(NodeSource::Aezeed)),
        Action::LAezeed,
    ));
    y += button_rows(ui, x, y, width, &row) + 6.0;
    if l.source == Some(NodeSource::Aezeed) {
        ui.fill(x, y, width, 44.0, 8.0, Role::SurfaceContainerHighest);
        ui.stroke(
            x,
            y,
            width,
            44.0,
            8.0,
            if l.on_passphrase {
                Role::OutlineVariant
            } else {
                Role::Primary
            },
        );
        let n = l.typed.split_whitespace().count();
        // Use a sample goes in the "Demo only" card while the field is
        // empty (`docs/DEMO.md` §14).
        if app.lightning_sample_shown() {
            crate::samples::put::offer(ui, Action::UseSample);
        }
        // A press places the caret, or moves typing here from the
        // passphrase (`docs/TEXT-CURSOR.md` §3).
        let here = Action::TypeHere(crate::edit::Here::Aezeed);
        let (tx, fh, room) = (
            x + faraday_draw::tokens::TYPED_INSET,
            faraday_draw::tokens::TEXT_FIELD_SHEET,
            width - 90.0,
        );
        ui.press_box(x, y, room, fh, here);
        if l.typed.is_empty() {
            let hint = ui.fit_as(BODY_M.mono(), EN.lightning_the_24_words, room);
            ui.text_mid_as((tx, y, fh), BODY_M.mono(), Role::OnSurface, &hint);
            if !l.on_passphrase {
                ui.caret(
                    tx,
                    y + (fh - faraday_draw::tokens::CARET_MIN) / 2.0,
                    faraday_draw::tokens::CARET_MIN,
                );
            }
        } else {
            ui.typed(
                tx,
                y,
                fh,
                BODY_M.size,
                W::M,
                Role::OnSurface,
                &l.typed,
                room - faraday_draw::tokens::TYPED_INSET,
                !l.on_passphrase,
                here,
            );
        }
        ui.text_right_as(
            (x + width - 12.0, y, 44.0),
            BODY_S,
            Role::OnSurfaceVariant,
            &fill(EN.lightning_word_count, &[&n.to_string()]),
        );
        y += 52.0;
        y += crate::screens::never_real_seed(app, ui, x, y, width);
        ui.text_mid_as(
            (x, y, 40.0),
            BODY_S,
            Role::OnSurfaceVariant,
            EN.lightning_passphrase,
        );
        // On a small panel the field goes under its name, Read it under
        // the field.
        let (px, pw) = if below > 0.0 {
            y += 36.0;
            (vx, vw)
        } else {
            (vx, 300.0f32.min(vw))
        };
        ui.fill(px, y, pw, 40.0, 8.0, Role::SurfaceContainerHighest);
        ui.stroke(
            px,
            y,
            pw,
            40.0,
            8.0,
            if l.on_passphrase {
                Role::Primary
            } else {
                Role::OutlineVariant
            },
        );
        if app.pass_is_public(&l.passphrase) {
            crate::screens::code_field(ui, px, y, pw, T::TEXT_FIELD, &l.passphrase);
        } else if l.passphrase.is_empty() && !l.on_passphrase {
            let p = ui.fit_as(
                BODY_M.mono(),
                EN.lightning_aezeed_lnd,
                pw - 2.0 * faraday_draw::tokens::TYPED_INSET,
            );
            let at = (
                px + faraday_draw::tokens::TYPED_INSET,
                y,
                faraday_draw::tokens::TEXT_FIELD,
            );
            ui.text_mid_as(at, BODY_M.mono(), Role::OnSurface, &p);
        } else {
            // A dot for each character, so the caret and a press fall
            // between the characters they stand for (`docs/TEXT-CURSOR.md`
            // §3).
            let dots = "•".repeat(l.passphrase.chars().count());
            ui.typed(
                px + faraday_draw::tokens::TYPED_INSET,
                y,
                faraday_draw::tokens::TEXT_FIELD,
                BODY_M.size,
                W::M,
                Role::OnSurface,
                &dots,
                pw - 2.0 * faraday_draw::tokens::TYPED_INSET,
                l.on_passphrase,
                Action::LPassphrase,
            );
        }
        ui.press_box(px, y, pw, 40.0, Action::LPassphrase);
        app.pass_chip_draw(ui, l.passphrase.is_empty(), crate::demo::PassField::Aezeed);
        let (rx, ry) = if below > 0.0 {
            (px, y + 48.0)
        } else {
            (px + pw + 12.0, y)
        };
        ui.button(
            rx,
            ry,
            None,
            BUTTON_H,
            EN.lightning_read_button,
            Style::Primary,
            Action::LResolve,
        );
        y = ry + 52.0;
    }
    if let Some(e) = &l.error {
        y += ui.wrap_as((x, y, width), BODY_S, Role::Error, e) + 10.0;
    }
    let Some(n) = &l.node else {
        return y;
    };
    let id: String = n.public().iter().map(|b| format!("{b:02x}")).collect();
    ui.text_as(x, y, BODY_S, Role::OnSurfaceVariant, EN.lightning_node_id);
    y += below;
    y += ui
        .wrap_as((vx, y, vw), BODY_M.mono(), Role::OnSurface, &id)
        .max(20.0)
        + 10.0;
    // The id as a code and as a picture, a row of their own
    // (`docs/QR.md` §7.1).
    let code = crate::Code::NodeId;
    let codes = [
        (
            EN.lightning_show_as_qr.to_string(),
            Style::Secondary,
            Action::ShowCode(code),
        ),
        (
            EN.lightning_png.to_string(),
            Style::Secondary,
            Action::CodePng(code),
        ),
    ];
    y += button_rows(ui, vx, y, vw, &codes) + T::GAP;
    if let Some((yy, m, d)) = n.birthday() {
        ui.text_as(x, y, BODY_S, Role::OnSurfaceVariant, EN.lightning_made);
        y += below;
        ui.text_as(
            vx,
            y,
            BODY_M.mono(),
            Role::OnSurface,
            &format!("{yy}-{m:02}-{d:02}"),
        );
        y += ui.advance_as(BODY_M.mono(), 30.0);
    }
    ui.text_as(
        x,
        y,
        BODY_S,
        Role::OnSurfaceVariant,
        EN.lightning_private_key,
    );
    y += below;
    let private: zeroize::Zeroizing<String> = if l.shown {
        n.private_hex(faraday_keys::Reveal::LightningKey)
            .unwrap_or_default()
    } else {
        zeroize::Zeroizing::new("••••••••••••".to_string())
    };
    y += ui
        .wrap_as((vx, y, vw), BODY_M.mono(), Role::OnSurface, &private)
        .max(20.0)
        + 12.0;
    let mut row = vec![(
        if l.shown {
            EN.lightning_hide
        } else {
            EN.lightning_show
        }
        .to_string(),
        Style::Secondary,
        Action::LShow,
    )];
    match app.vaults.open.get(app.vaults.current) {
        Some(v) => row.push((
            fill(EN.lightning_save_into, &[&v.name.to_string()]),
            Style::Primary,
            Action::LVault,
        )),
        // Make or unlock one, and back to this key.
        None => row.extend(
            app.vault_way(crate::Screen::Lightning)
                .map(|(label, a)| (label, Style::Primary, a)),
        ),
    }
    row.push((
        EN.lightning_out_unprotected.to_string(),
        Style::Ghost,
        Action::LOut,
    ));
    y + button_rows(ui, vx, y, vw, &row)
}
