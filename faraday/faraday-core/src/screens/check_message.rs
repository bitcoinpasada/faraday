//! `Screen::CheckMessage`: checking a signed message.

use crate::drawing::View;
use crate::strings::{EN, fill};
use crate::ui::pal::*;
use crate::ui::{Ui, grouped};
use crate::wallet::FileKind;
use crate::{Action, Faraday, Screen};
use faraday_draw::tokens::{BODY_M, BODY_S, TITLE_M, TITLE_S};
use faraday_draw::widgets::Icon;

/// Costs what `act` costs. Tools' Check a signed message: the message
/// itself when exactly one waits in Files, the files to choose from when
/// several do, and the empty part when none. Practice's sample message
/// stands in when there is none.
pub(crate) fn open_tile(app: &mut Faraday) {
    let waiting = waiting(app);
    match waiting[..] {
        [(k, _)] => app.act(Action::CheckMessage(k)),
        [] if app.put_on().is_some() => app.check_use_sample(),
        _ => {
            let back = match app.screen {
                Screen::CheckMessage => app.message.check_back(),
                s => s,
            };
            app.message.open_landing(back);
            app.list_offset = 0.0;
            app.screen = Screen::CheckMessage;
        }
    }
}

/// The signed messages in Files, by index and name.
fn waiting(app: &Faraday) -> Vec<(usize, &str)> {
    app.inbox
        .iter()
        .enumerate()
        .filter(|(_, i)| i.kind == FileKind::Message)
        .map(|(k, i)| (k, &*i.name))
        .collect()
}

/// No message chosen: the files to choose from, or the empty part.
fn landing(app: &View, ui: &mut Ui, at: (f32, f32, f32)) {
    let waiting = waiting(app);
    if waiting.is_empty() {
        super::parts::nothing_to_work_on(
            app,
            ui,
            at,
            EN.check_no_signed_message,
            (Action::Scan, Screen::CheckMessage),
        );
    } else {
        let rows: Vec<(&str, Action)> = waiting
            .into_iter()
            .map(|(k, name)| (name, Action::CheckMessage(k)))
            .collect();
        super::parts::file_choices(ui, at, EN.decode_choose_message, &rows);
    }
}

pub(super) fn draw(app: &View, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    // A small panel has the way back and the name in its bar, and one
    // column that scrolls: each value under its name.
    let compact = ui.compact;
    let (x, width) = if compact {
        (x0 + crate::compact::M, cw - 2.0 * crate::compact::M)
    } else {
        (x0 + 48.0, (cw - 96.0).min(900.0))
    };
    let top = if compact {
        ui.page_begin((x0, cw, h), app.list_offset);
        14.0
    } else {
        let to = app.message.check_back();
        let name = if to == Screen::WalletsStart {
            EN.check_wallets
        } else {
            super::decode_back(to)
        };
        let back = app.named_back((name, Action::Nav(to)));
        super::bar_head(ui, Some(back), EN.check_check_a_signed)
    };
    let mut y = top;
    let Some(item) = app.message.checking().and_then(|i| app.inbox.get(i)) else {
        landing(app, ui, (x, y, width));
        return;
    };
    let Some(c) = app.message_check(&String::from_utf8_lossy(&item.bytes)) else {
        ui.wrap_as(
            (x, y, width),
            BODY_M,
            Role::Error,
            &fill(EN.check_not_a_signed, &[app.session.network().name()]),
        );
        return;
    };
    let (line, color) = match &c.verdict {
        Ok(f) => (
            fill(
                EN.check_signature_valid,
                &[(if *f == faraday_psbt::message::Format::Bip322 {
                    "BIP-322"
                } else {
                    "BIP-137"
                })],
            ),
            Role::Ok,
        ),
        Err(e) => (
            crate::state::message_error_text(*e).to_string(),
            Role::Error,
        ),
    };
    // The verdict's card grows to its wrapped line.
    let (size, lx) = if compact {
        (TITLE_S, 46.0)
    } else {
        (TITLE_M, 54.0)
    };
    let line_h = if compact {
        ui.c.push_clip(faraday_draw::Rect::new(0, 0, 0, 0));
        let lh = ui.wrap_as((x + lx, y, width - lx - 14.0), size, color, &line);
        ui.c.pop_clip();
        lh
    } else {
        0.0
    };
    let card_h = if compact { line_h + 32.0 } else { 64.0 };
    ui.card(x, y, width, card_h, Role::OutlineVariant);
    ui.icon(
        x + lx - 36.0,
        y + card_h / 2.0 - 12.0,
        24.0,
        if c.verdict.is_ok() {
            Icon::Success
        } else {
            Icon::Error
        },
        16.0,
        color,
    );
    if compact {
        ui.wrap_as((x + lx, y + 16.0, width - lx - 14.0), size, color, &line);
    } else {
        ui.text_mid_as((x + 54.0, y, 64.0), TITLE_M, color, &line);
    }
    y += card_h + 20.0;
    let rows = [
        (EN.check_file, item.name.clone(), BODY_M.mono()),
        (EN.check_address, grouped(&c.signed.address), BODY_M.mono()),
        (
            EN.check_wallet,
            match c.wallet {
                Some((i, change, index)) => faraday_text::text!(
                    "{} · {} {}/{index}",
                    &*app.session.wallets[i].name,
                    if change {
                        EN.check_change
                    } else {
                        EN.check_receive
                    },
                    u8::from(change)
                ),
                None => faraday_text::Text::of(EN.check_not_an_address),
            },
            BODY_M,
        ),
    ];
    let vx = if compact { 0.0 } else { 100.0 };
    for (k, v, face) in rows.iter() {
        ui.text_as(x, y, BODY_S, Role::OnSurfaceVariant, k);
        if compact {
            y += 18.0;
        }
        let used = ui.wrap_as((x + vx, y, width - vx), *face, Role::OnSurface, v);
        y += used.max(20.0) + 12.0;
    }
    ui.text_as(x, y, BODY_S, Role::OnSurfaceVariant, EN.check_message);
    if compact {
        y += 18.0;
    } else {
        ui.fill(
            x + 100.0,
            y - 4.0,
            width - 100.0,
            1.0,
            0.0,
            Role::OutlineVariant,
        );
        y += 12.0;
    }
    let shown = app.message.shown().text(&c.signed.message);
    y += ui.wrap_as((x + vx, y, width - vx), BODY_M, Role::OnSurface, &shown);
    y += super::parts::ending(
        ui,
        (x, y, width),
        None,
        (
            Action::CheckMessageAnother,
            Action::Nav(app.message.check_back()),
        ),
    );
    if compact {
        let guide = EN.check_a_valid_signature;
        y += 16.0 + crate::compact::screen::about(ui, x, y + 16.0, width, guide);
        crate::compact::screen::finish(app, ui, h, y - top + 24.0);
    }
}
