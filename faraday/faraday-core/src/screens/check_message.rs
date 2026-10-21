//! `Screen::CheckMessage`: checking a signed message.

use super::parts::guide_text;
use crate::drawing::View;
use crate::strings::{EN, fill};
use crate::ui::pal::*;
use crate::ui::{Ui, W, grouped};
use crate::{Action, Screen};
use faraday_draw::widgets::Icon;
use faraday_ui::parts::title;

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
        14.0 - app.list_offset
    } else {
        28.0
    };
    let mut y = top;
    if !compact {
        ui.icon(x - 4.0, y, 16.0, Icon::ChevronLeft, 10.0, MUTED);
        ui.text(x + 14.0, y, 12.0, W::R, MUTED, EN.check_wallets);
        ui.hit(
            x - 4.0,
            y - 4.0,
            80.0,
            24.0,
            Action::Nav(Screen::WalletsStart),
        );
        ui.back(Action::Nav(Screen::WalletsStart));
        y += 22.0;
        title(ui, x, y, EN.check_check_a_signed);
        y += 60.0;
    }
    let guide = EN.check_a_valid_signature;
    // On a small panel it comes after what it explains.
    if !compact {
        y += guide_text(app, ui, x, y, width, guide);
    }
    let Some(item) = app.checking.and_then(|i| app.inbox.get(i)) else {
        ui.text(x, y, 14.0, W::R, MUTED, EN.check_no_signed_message);
        return;
    };
    let Some(c) = app.message_check(&String::from_utf8_lossy(&item.bytes)) else {
        ui.wrap(
            x,
            y,
            width,
            14.0,
            W::R,
            ERR,
            &fill(EN.check_not_a_signed, &[app.session.network().name()]),
        );
        return;
    };
    let (line, color) = match &c.verdict {
        Ok(f) => (
            fill(
                EN.check_the_signature_is,
                &[(if *f == faraday_psbt::message::Format::Bip322 {
                    "BIP-322"
                } else {
                    "BIP-137"
                })],
            ),
            OK,
        ),
        Err(e) => (fill(EN.check_the_signature, &[&e.to_string()]), ERR),
    };
    // The verdict's card grows to its wrapped line.
    let (size, lx) = if compact { (14.0, 46.0) } else { (16.0, 54.0) };
    let line_h = if compact {
        ui.c.push_clip(faraday_draw::Rect::new(0, 0, 0, 0));
        let lh = ui.wrap(x + lx, y, width - lx - 14.0, size, W::S, color, &line);
        ui.c.pop_clip();
        lh
    } else {
        0.0
    };
    let card_h = if compact { line_h + 32.0 } else { 64.0 };
    ui.card(
        x,
        y,
        width,
        card_h,
        if c.verdict.is_ok() {
            OK.with_alpha(110)
        } else {
            ERR.with_alpha(110)
        },
    );
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
        ui.wrap(
            x + lx,
            y + 16.0,
            width - lx - 14.0,
            size,
            W::S,
            color,
            &line,
        );
    } else {
        ui.text_mid(x + 54.0, y, 64.0, 16.0, W::S, color, &line);
    }
    y += card_h + 20.0;
    let rows = [
        (EN.check_file, item.name.clone(), W::M),
        (
            EN.check_address,
            grouped(&app.session.shown(&c.signed.address)),
            W::M,
        ),
        (
            EN.check_wallet,
            match c.wallet {
                Some((i, change, index)) => format!(
                    "{} · {} {}/{index}",
                    app.session.wallets[i].name,
                    if change {
                        EN.check_change
                    } else {
                        EN.check_receive
                    },
                    u8::from(change)
                ),
                None => EN.check_not_an_address.to_string(),
            },
            W::R,
        ),
    ];
    let vx = if compact { 0.0 } else { 100.0 };
    for (k, v, face) in rows.iter() {
        ui.text(x, y, 12.0, W::R, MUTED, k);
        if compact {
            y += 18.0;
        }
        let used = ui.wrap(x + vx, y, width - vx, 14.0, *face, TEXT, v);
        y += used.max(20.0) + 12.0;
    }
    ui.text(x, y, 12.0, W::R, MUTED, EN.check_message);
    if compact {
        y += 18.0;
    } else {
        ui.fill(x + 100.0, y - 4.0, width - 100.0, 1.0, 0.0, INNER);
        y += 12.0;
    }
    y += ui.wrap(x + vx, y, width - vx, 14.0, W::R, TEXT, &c.signed.message);
    if compact {
        y += 16.0 + crate::compact::screen::about(ui, x, y + 16.0, width, guide);
        crate::compact::screen::finish(app, ui, x0, cw, h, y - top + 24.0);
    }
}
