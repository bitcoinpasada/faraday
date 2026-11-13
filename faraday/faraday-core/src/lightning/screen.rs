//! The Lightning node key screen.

use crate::Action;
use crate::drawing::View;
use crate::lightning::NodeSource;
use crate::screens::{button_rows, guide_text};
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
    ui.page_begin((x0, cw, h), app.list_offset);
    let top = 12.0;
    let guide = if app.lightning.for_convert() {
        EN.convert_aezeed_guide
    } else {
        EN.lightning_a_lightning_node
    };
    let y = if compact {
        top
    } else {
        header(app, ui, x, width, guide)
    };
    let end = body(app, ui, x, y, width, (vx, vw, below));
    if compact {
        let y = end + crate::compact::screen::about(ui, x, end, width, guide);
        crate::compact::screen::finish(app, ui, h, y - top + 16.0);
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
    let title = if l.for_convert {
        EN.convert_lnd_aezeed
    } else {
        EN.lightning_lightning_node
    };
    let y = crate::screens::bar_head(ui, Some(back), title);
    y + guide_text(app, ui, x, y, width, guide)
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
    // Opened from Convert, the entry stands alone: the source is the aezeed.
    let keys = app.session.keys.all().filter(|_| !l.for_convert);
    let mut row: Vec<(String, Style, Action)> = keys
        .flat_map(|k| {
            let fp = k.fingerprint;
            let name = crate::wallet::key_line(fp, &k.label).to_string();
            [
                (
                    fill(EN.lightning_ldk_node, &[&name]),
                    Style::chosen(l.source == Some(NodeSource::Key(fp.0))),
                    Action::LKey(fp.0),
                ),
                (
                    fill(EN.lightning_cln_node, &[&name]),
                    Style::chosen(l.source == Some(NodeSource::KeyCln(fp.0))),
                    Action::LCln(Some(fp.0)),
                ),
            ]
        })
        .collect();
    if !l.for_convert {
        row.push((
            EN.lightning_lnd_type.to_string(),
            Style::chosen(l.source == Some(NodeSource::Aezeed)),
            Action::LAezeed,
        ));
        row.push((
            EN.lightning_cln_type.to_string(),
            Style::chosen(l.source == Some(NodeSource::CoreLightning)),
            Action::LCln(None),
        ));
        y += button_rows(ui, x, y, width, &row) + 6.0;
    }
    let cln = l.source == Some(NodeSource::CoreLightning);
    if l.source == Some(NodeSource::Aezeed) || cln {
        if cln {
            y += hsm_files(app, ui, l, (x, y, width));
        }
        if !(cln && l.file.is_some()) {
            y += words_field(app, ui, l, (x, y, width), cln);
        }
        // A file that asks for nothing is read when it is chosen.
        if !(cln && l.file.is_some() && !l.file_asks) {
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
            ui.fill(
                px,
                y,
                pw,
                T::TEXT_FIELD,
                T::RADIUS_SMALL,
                Role::SurfaceContainerHighest,
            );
            ui.stroke(
                px,
                y,
                pw,
                T::TEXT_FIELD,
                T::RADIUS_SMALL,
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
                    if cln {
                        EN.lightning_no_passphrase
                    } else {
                        EN.lightning_aezeed_lnd
                    },
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
            ui.press_box(px, y, pw, T::TEXT_FIELD, Action::LPassphrase);
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
    }
    if let Some(e) = &l.error {
        y += ui.wrap_as((x, y, width), BODY_S, Role::Error, e) + 10.0;
    }
    let Some(n) = &l.node else {
        return y;
    };
    let id: String = String::from(&*faraday_crypto::hex::encode_string(n.public()));
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

/// The words' field with its count and the sample chip; `cln` for Core
/// Lightning's, which takes 12 to 24. Returns the height used.
fn words_field(
    app: &View,
    ui: &mut Ui,
    l: &crate::lightning::LightningState,
    (x, y0, width): (f32, f32, f32),
    cln: bool,
) -> f32 {
    let mut y = y0;
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
        let hint = ui.fit_as(
            BODY_M.mono(),
            if cln {
                EN.lightning_the_words
            } else {
                EN.lightning_the_24_words
            },
            room,
        );
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
        &if cln {
            fill(EN.lightning_words_typed, &[&n.to_string()])
        } else {
            fill(EN.lightning_word_count, &[&n.to_string()])
        },
    );
    y += 52.0;
    y += crate::screens::never_real_seed(app, ui, x, y, width);
    y - y0
}

/// The `hsm_secret` files in Files, each a button to read it, and the
/// line saying so where there are none. Returns the height used.
fn hsm_files(
    app: &View,
    ui: &mut Ui,
    l: &crate::lightning::LightningState,
    (x, y, width): (f32, f32, f32),
) -> f32 {
    let files: Vec<(usize, String)> = app
        .inbox
        .iter()
        .enumerate()
        .filter(|(_, i)| crate::lightning::could_be_hsm(i.kind, i.bytes.len()))
        .map(|(k, i)| (k, i.name.clone()))
        .collect();
    if files.is_empty() {
        let used = ui.wrap_as(
            (x, y, width),
            BODY_S,
            Role::OnSurfaceVariant,
            EN.lightning_no_hsm_file,
        );
        return used + 8.0;
    }
    let row: Vec<(String, Style, Action)> = files
        .into_iter()
        .map(|(k, name)| {
            (
                fill(EN.lightning_use_file, &[&name]),
                Style::chosen(l.file.as_deref() == Some(name.as_str())),
                Action::LFile(k),
            )
        })
        .collect();
    button_rows(ui, x, y, width, &row) + 8.0
}
