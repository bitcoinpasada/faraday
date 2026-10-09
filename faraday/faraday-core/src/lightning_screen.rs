//! The Lightning node key screen.

use crate::lightning::NodeSource;
use crate::screens::{button_rows, guide_text, title};
use crate::ui::pal::*;
use crate::ui::{Style, Ui, W};
use crate::wallet::fp_text;
use crate::{Action, Faraday};

pub(crate) fn draw(app: &Faraday, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    if app.lightning.is_none() {
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
    let guide = "A Lightning node's key, made the way its software makes it: ldk-node from a BIP-39 key, \
                 LND from its own 24-word cipher seed. Its public key is the node id, safe to share; its \
                 private key is the node, and goes into a vault.";
    let mut y = if compact { top } else { 36.0 };
    if !compact {
        y = header(app, ui, x, y, width, guide);
    }
    let end = body(app, ui, x, y, width, (vx, vw, below));
    if compact {
        let y = end + crate::compact_screens::about(ui, x, end, width, guide);
        crate::compact_screens::finish(app, ui, x0, cw, h, y - top + 16.0);
    }
}

/// The way back, the title and what explains the page, as the desktop
/// draws them. Returns where the page goes on.
fn header(app: &Faraday, ui: &mut Ui, x: f32, mut y: f32, width: f32, guide: &str) -> f32 {
    let Some(l) = app.lightning.as_ref() else {
        return y;
    };
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
    y + guide_text(app, ui, x, y, width, guide)
}

/// The sources, the aezeed's words, and the node. `vx`, `vw` are where
/// values go; `below` is how far a value sits under its name. Returns
/// where it ends.
fn body(
    app: &Faraday,
    ui: &mut Ui,
    x: f32,
    mut y: f32,
    width: f32,
    (vx, vw, below): (f32, f32, f32),
) -> f32 {
    let Some(l) = app.lightning.as_ref() else {
        return y;
    };
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
            crate::secret_text::SecretText::of("The 24 words")
        } else {
            let mut s = l.typed.clone();
            s.push_str(ui.caret_char());
            s
        };
        let shown = ui.fit_secret(14.0, W::M, &shown, width - 90.0);
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
        // On a small panel the field goes under its name, Read it under
        // the field.
        let (px, pw) = if below > 0.0 {
            y += 36.0;
            (vx, vw)
        } else {
            (vx, 300.0f32.min(vw))
        };
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
        let p = ui.fit(13.0, W::M, &p, pw - 24.0);
        ui.text_mid(px + 12.0, y, 40.0, 13.0, W::M, TEXT, &p);
        ui.hit(px, y, pw, 40.0, Action::LPassphrase);
        let (rx, ry) = if below > 0.0 {
            (px, y + 48.0)
        } else {
            (px + pw + 12.0, y)
        };
        ui.button(
            rx,
            ry,
            None,
            40.0,
            "Read it",
            Style::Primary,
            Action::LResolve,
        );
        y = ry + 52.0;
    }
    if let Some(e) = &l.error {
        y += ui.wrap(x, y, width, 13.0, W::R, ERR, e) + 10.0;
    }
    let Some(n) = &l.node else {
        return y;
    };
    let id: String = n.public.iter().map(|b| format!("{b:02x}")).collect();
    ui.text(x, y, 13.0, W::R, MUTED, "Node id");
    y += below;
    y += ui.wrap(vx, y, vw, 14.0, W::M, TEXT, &id).max(20.0) + 10.0;
    if let Some((yy, m, d)) = n.birthday {
        ui.text(x, y, 13.0, W::R, MUTED, "Made");
        y += below;
        ui.text(vx, y, 14.0, W::M, TEXT, &format!("{yy}-{m:02}-{d:02}"));
        y += 30.0;
    }
    ui.text(x, y, 13.0, W::R, MUTED, "Private key");
    y += below;
    let private: zeroize::Zeroizing<String> = if l.shown {
        crate::secret_text::hex(&n.private[..])
    } else {
        zeroize::Zeroizing::new("••••••••••••".to_string())
    };
    y += ui.wrap(vx, y, vw, 14.0, W::M, TEXT, &private).max(20.0) + 12.0;
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
        // Make or unlock one, and back to this key.
        None => row.extend(
            app.vault_way(crate::Screen::Lightning)
                .map(|(label, a)| (label, Style::Primary, a)),
        ),
    }
    row.push(("Out unprotected…".to_string(), Style::Ghost, Action::LOut));
    y + button_rows(ui, vx, y, vw, &row)
}
