//! The Lightning node key screen.

use crate::lightning::NodeSource;
use crate::screens::{button_rows, guide_text, title};
use crate::ui::pal::*;
use crate::ui::{Style, Ui, W};
use crate::wallet::fp_text;
use crate::{Action, Faraday};

pub(crate) fn draw(app: &Faraday, ui: &mut Ui, x0: f32, cw: f32, _h: f32) {
    let Some(l) = app.lightning.as_ref() else {
        return;
    };
    let x = x0 + 56.0;
    let width = (cw - 112.0).min(900.0);
    let mut y = 36.0;
    ui.icon(
        x - 4.0,
        y,
        16.0,
        osk_ui::widgets::Icon::ChevronLeft,
        10.0,
        MUTED,
    );
    ui.text(x + 14.0, y, 12.0, W::R, MUTED, "Wallets");
    ui.hit(x - 4.0, y - 4.0, 70.0, 24.0, Action::Nav(l.back));
    y += 22.0;
    title(ui, x, y, "Lightning node key");
    y += 52.0;
    y += guide_text(
        app,
        ui,
        x,
        y,
        width,
        "A Lightning node's key, made the way its software makes it: ldk-node from a BIP-39 key, LND \
         from its own 24-word cipher seed. Its public key is the node id, safe to share; its private key \
         is the node, and goes into a vault.",
    );
    let mut row: Vec<(String, Style, Action)> = app
        .session
        .keys
        .iter()
        .map(|k| {
            let fp = k.master.fingerprint();
            (
                format!("ldk-node · {} · {}", fp_text(fp), k.label),
                if l.source == Some(NodeSource::Key(fp.0)) {
                    Style::Primary
                } else {
                    Style::Secondary
                },
                Action::LKey(fp.0),
            )
        })
        .collect();
    row.push((
        "LND · type an aezeed".to_string(),
        if l.source == Some(NodeSource::Aezeed) {
            Style::Primary
        } else {
            Style::Secondary
        },
        Action::LAezeed,
    ));
    y += button_rows(ui, x, y, width, &row) + 6.0;
    if l.source == Some(NodeSource::Aezeed) {
        ui.fill(x, y, width, 44.0, 8.0, BG);
        ui.stroke(
            x,
            y,
            width,
            44.0,
            8.0,
            if l.on_passphrase { BORDER } else { ACCENT },
        );
        let n = l.typed.split_whitespace().count();
        let shown = if l.typed.is_empty() {
            "The 24 words".to_string()
        } else {
            format!("{}{}", l.typed.as_str(), ui.caret_char())
        };
        let shown = ui.fit(14.0, W::M, &shown, width - 90.0);
        ui.text_mid(x + 12.0, y, 44.0, 14.0, W::M, TEXT, &shown);
        ui.text_right(
            x + width - 12.0,
            y,
            44.0,
            12.0,
            W::R,
            DIM,
            &format!("{n} of 24"),
        );
        y += 52.0;
        ui.text_mid(x, y, 40.0, 13.0, W::R, MUTED, "Passphrase");
        let (px, pw) = (x + 120.0, 300.0f32.min(width - 120.0));
        ui.fill(px, y, pw, 40.0, 8.0, BG);
        ui.stroke(
            px,
            y,
            pw,
            40.0,
            8.0,
            if l.on_passphrase { ACCENT } else { BORDER },
        );
        let p = if l.passphrase.is_empty() && !l.on_passphrase {
            "aezeed (LND's default)".to_string()
        } else {
            format!("{}{}", "•".repeat(l.passphrase.len()), ui.caret_char())
        };
        ui.text_mid(px + 12.0, y, 40.0, 13.0, W::M, TEXT, &p);
        ui.hit(px, y, pw, 40.0, Action::LPassphrase);
        ui.button(
            px + pw + 12.0,
            y,
            None,
            40.0,
            "Read it",
            Style::Primary,
            Action::LResolve,
        );
        y += 52.0;
    }
    if let Some(e) = &l.error {
        ui.text(x, y, 13.0, W::R, ERR, e);
        y += 26.0;
    }
    let Some(n) = &l.node else {
        return;
    };
    let id: String = n.public.iter().map(|b| format!("{b:02x}")).collect();
    ui.text(x, y, 13.0, W::R, MUTED, "Node id");
    y += ui
        .wrap(x + 120.0, y, width - 120.0, 14.0, W::M, TEXT, &id)
        .max(20.0)
        + 10.0;
    if let Some((yy, m, d)) = n.birthday {
        ui.text(x, y, 13.0, W::R, MUTED, "Made");
        ui.text(
            x + 120.0,
            y,
            14.0,
            W::M,
            TEXT,
            &format!("{yy}-{m:02}-{d:02}"),
        );
        y += 30.0;
    }
    ui.text(x, y, 13.0, W::R, MUTED, "Private key");
    let private: zeroize::Zeroizing<String> = zeroize::Zeroizing::new(if l.shown {
        n.private.iter().map(|b| format!("{b:02x}")).collect()
    } else {
        "••••••••••••".to_string()
    });
    y += ui
        .wrap(x + 120.0, y, width - 120.0, 14.0, W::M, TEXT, &private)
        .max(20.0)
        + 12.0;
    let mut row = vec![(
        if l.shown { "Hide" } else { "Show" }.to_string(),
        Style::Secondary,
        Action::LShow,
    )];
    match app.vaults.open.get(app.vaults.current) {
        Some(v) => row.push((
            format!("Save into {}", v.name),
            Style::Primary,
            Action::LVault,
        )),
        None => row.push(("No vault open".to_string(), Style::Disabled, Action::LVault)),
    }
    row.push(("Out unprotected…".to_string(), Style::Ghost, Action::LOut));
    button_rows(ui, x + 120.0, y, width - 120.0, &row);
}
