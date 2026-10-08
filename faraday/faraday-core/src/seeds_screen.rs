//! A wallet made from the seeds in hand, drawn (`seeds.rs`): the seeds
//! typed so far with the ways to add another, then the wallet's shape:
//! M of N on two sliders, a box for each cosigner whose seed is not here,
//! the kind, the path and the first address. Restore's seeds card and
//! the Spend tab's words route draw it.

use crate::create::NewKind;
use crate::screens::{buttons_and_next, stepper, wrap_buttons};
use crate::seeds::{self, Focus, SeedsAction as S, SeedsState};
use crate::ui::pal::*;
use crate::ui::{Style, Ui, W, grouped};
use crate::wallet::{FileKind, fp_text};
use crate::{Action, Faraday};

fn sa(a: S) -> Action {
    Action::Seeds(a)
}

/// The seeds typed so far, each with its fingerprint and label, and the
/// ways to add another: Add a key, Scan a SeedQR, a seed an open vault
/// holds, a seed already loaded. `words` names them as the Spend tab
/// does. Returns the height used.
pub(crate) fn keys(
    app: &Faraday,
    ui: &mut Ui,
    s: &SeedsState,
    words: bool,
    x: f32,
    y: f32,
    w: f32,
) -> f32 {
    let mut cy = y;
    let here = app.seeds_here(s);
    for (k, fp) in here.iter().enumerate() {
        let f = osk_bip::keys::Fingerprint(*fp);
        let label = app.session.key_label(f).unwrap_or("");
        ui.text_mid(x, cy, 40.0, 12.0, W::R, DIM, &(k + 1).to_string());
        ui.text_mid(x + 22.0, cy, 40.0, 14.0, W::M, TEXT, &fp_text(f));
        // A key named after its fingerprint is not named twice.
        let line = if label.is_empty() || label.eq_ignore_ascii_case(&fp_text(f)) {
            "Here".to_string()
        } else {
            format!("Here · {label}")
        };
        let line = ui.fit(13.0, W::R, &line, w - 124.0);
        ui.text_mid(x + 124.0, cy, 40.0, 13.0, W::R, OK, &line);
        ui.rule(x, cy + 40.0, w, INNER);
        cy += 42.0;
    }
    if !here.is_empty() {
        cy += 10.0;
    }
    let first = here.is_empty();
    let add = match (words, first) {
        (true, true) => "Type the words",
        (true, false) => "Add another seed",
        (false, true) => "Add a key",
        (false, false) => "Add another key",
    };
    let add_style = if first {
        Style::Primary
    } else {
        Style::Secondary
    };
    let mut labels: Vec<(String, Style, Action)> = vec![
        (add.to_string(), add_style, Action::Entry(None)),
        (
            "Scan a SeedQR".to_string(),
            Style::Secondary,
            Action::ScanSeed,
        ),
    ];
    for (v, r, fp) in seeds::vault_seeds(app) {
        let name = &app.vaults.open[v].name;
        labels.push((
            format!("Load {fp} from {name}"),
            Style::Secondary,
            sa(S::FromVault(v, r)),
        ));
    }
    for k in &app.session.keys {
        let f = k.master.fingerprint();
        if !here.contains(&f.0) {
            labels.push((
                format!("Use {} · {}", fp_text(f), k.label),
                Style::Secondary,
                sa(S::UseLoaded(f.0)),
            ));
        }
    }
    let items: Vec<(&str, Style, Action)> = labels
        .iter()
        .map(|(l, st, a)| (l.as_str(), *st, *a))
        .collect();
    cy += wrap_buttons(ui, x, cy, w, 40.0, &items);
    cy - y
}

/// The wallet's shape: M of N, the cosigners' account keys, the kind,
/// the path, the first address, and Make the wallet. `back` adds Back
/// to the seeds. Returns the height used.
#[allow(clippy::too_many_arguments)]
pub(crate) fn shape(
    app: &Faraday,
    ui: &mut Ui,
    s: &SeedsState,
    back: bool,
    x: f32,
    y: f32,
    w: f32,
) -> f32 {
    let mut cy = y;
    let net = app.session.network();
    let n = s.n.max(1);
    let have = app.seeds_here(s).len();
    // M of N, large, and its two sliders.
    let big = format!("{} of {n}", s.m.max(1));
    ui.text(x, cy, 22.0, W::S, TEXT, &big);
    let bw = ui.measure(22.0, W::S, &big);
    let line = if n == 1 {
        "one key signs".to_string()
    } else {
        format!(
            "{} {} of {n} keys",
            s.m,
            if s.m == 1 { "signature" } else { "signatures" }
        )
    };
    ui.text_mid(x + bw + 14.0, cy, 30.0, 13.0, W::R, MUTED, &line);
    cy += 42.0;
    let (sx, sw) = if ui.compact {
        (x, w)
    } else {
        (x + 130.0, (w - 130.0).min(520.0))
    };
    for (id, label) in [(seeds::SLIDE_M, "Signatures"), (seeds::SLIDE_N, "Keys")] {
        let Some((lo, hi, v)) = s.slider_at(id) else {
            continue;
        };
        if ui.compact {
            ui.text(x, cy, 13.0, W::S, MUTED, label);
            cy += 20.0;
        } else {
            ui.text_mid(x, cy, 40.0, 13.0, W::S, MUTED, label);
        }
        cy += ui.slider(sx, cy, sw, id, lo as u8, hi as u8, v as u8) + 4.0;
    }
    // A box for each key whose seed is not here.
    if !s.cosigners.is_empty() {
        cy += 6.0;
        cy += ui.wrap(
            x,
            cy,
            w,
            13.0,
            W::S,
            MUTED,
            "Cosigners' account keys (xpub)",
        ) + 10.0;
        let files: Vec<(usize, &str)> = app
            .inbox
            .iter()
            .enumerate()
            .filter(|(_, i)| i.kind == FileKind::Key)
            .map(|(k, i)| (k, i.name.as_str()))
            .collect();
        let open = s.open_cosigner();
        for (k, text) in s.cosigners.iter().enumerate() {
            let slot = have + k + 1;
            let focused = s.focus == Some(Focus::Cosigner(k as u8));
            ui.text_mid(x, cy, 40.0, 12.0, W::R, DIM, &slot.to_string());
            match seeds::cosigner_key(text).filter(|_| !focused) {
                Some(key) => {
                    let cw = ui.measure(13.0, W::S, "Change") + 32.0;
                    let shown = ui.fit(14.0, W::M, &seeds::cosigner_label(&key), w - 34.0 - cw);
                    ui.text_mid(x + 22.0, cy, 40.0, 14.0, W::M, OK, &shown);
                    ui.button(
                        x + w - cw,
                        cy + 2.0,
                        Some(cw),
                        36.0,
                        "Change",
                        Style::Ghost,
                        sa(S::Clear(k as u8)),
                    );
                    cy += 46.0;
                }
                None => {
                    field(
                        ui,
                        x + 22.0,
                        cy,
                        w - 22.0,
                        text,
                        "Type or paste an xpub",
                        focused,
                        sa(S::Focus(Focus::Cosigner(k as u8))),
                    );
                    cy += 48.0;
                    if !text.trim().is_empty() && !focused {
                        cy += ui.wrap(
                            x + 22.0,
                            cy,
                            w - 22.0,
                            12.0,
                            W::R,
                            ERR,
                            "Not an account key",
                        ) + 6.0;
                    }
                    // Scan and the key files in Files fill the first open
                    // box.
                    if open == Some(k) {
                        let labels: Vec<String> =
                            files.iter().map(|(_, n)| format!("Use {n}")).collect();
                        let mut items: Vec<(&str, Style, Action)> =
                            vec![("Scan", Style::Secondary, Action::Scan)];
                        for ((i, _), l) in files.iter().zip(&labels) {
                            items.push((l.as_str(), Style::Secondary, sa(S::UseFile(*i))));
                        }
                        cy += wrap_buttons(ui, x + 22.0, cy, w - 22.0, 36.0, &items);
                    }
                }
            }
        }
    }
    // The kind.
    cy += 8.0;
    ui.text(x, cy, 13.0, W::S, MUTED, "Kind");
    cy += 24.0;
    for (k, kind) in s.kinds().iter().enumerate() {
        let on = *kind == s.kind;
        ui.checkbox(x, cy + 11.0, on, true);
        let name = kind_label(*kind);
        ui.text_mid(
            x + 30.0,
            cy,
            40.0,
            14.0,
            W::S,
            if on { TEXT } else { MUTED },
            name,
        );
        let rh = if ui.compact {
            let l = ui.fit(12.0, W::R, kind.line(), w - 30.0);
            ui.text_mid(x + 30.0, cy + 28.0, 20.0, 12.0, W::R, DIM, &l);
            52.0
        } else {
            let nw = ui.measure(14.0, W::S, name);
            let l = ui.fit(12.0, W::R, kind.line(), w - 60.0 - nw);
            ui.text_right(x + w, cy, 40.0, 12.0, W::R, DIM, &l);
            40.0
        };
        ui.hit(x, cy, w, rh, sa(S::Kind(k as u8)));
        ui.rule(x, cy + rh, w, INNER);
        cy += rh + 2.0;
    }
    // The path.
    cy += 12.0;
    ui.text(x, cy, 13.0, W::S, MUTED, "Derivation path");
    cy += 24.0;
    let standard = s.custom.is_none();
    ui.checkbox(x, cy + 11.0, standard, true);
    let std_path = s.kind.path_at(net, s.account);
    ui.text_mid(x + 30.0, cy, 40.0, 14.0, W::S, TEXT, "Standard");
    let pw = ui.measure(14.0, W::S, "Standard") + 44.0;
    ui.text_mid(
        x + pw,
        cy,
        40.0,
        14.0,
        W::M,
        if standard { TEXT } else { MUTED },
        &std_path,
    );
    ui.hit(x, cy, w, 40.0, sa(S::Standard));
    cy += 44.0;
    // The account, where the kind's path has one.
    if s.kind != NewKind::MultiLegacy {
        stepper(
            ui,
            x + 30.0,
            cy,
            w - 30.0,
            "Account",
            s.account as usize,
            sa(S::Account(false)),
            sa(S::Account(true)),
            true,
        );
        cy += 50.0;
    }
    ui.checkbox(x, cy + 11.0, !standard, true);
    ui.text_mid(x + 30.0, cy, 40.0, 14.0, W::S, TEXT, "Custom path");
    ui.hit(x, cy, w, 40.0, sa(S::Custom));
    cy += 44.0;
    if let Some(p) = &s.custom {
        let focused = s.focus == Some(Focus::Path);
        field(
            ui,
            x + 30.0,
            cy,
            (w - 30.0).min(420.0),
            p,
            "m/48'/0'/0'/2'",
            focused,
            sa(S::Focus(Focus::Path)),
        );
        cy += 50.0;
    }
    // What it makes.
    cy += 8.0;
    match app.seeds_address(s) {
        Ok(a) => {
            ui.text(x, cy, 12.0, W::R, MUTED, "First receive address · 0/0");
            cy += 22.0;
            cy += ui.wrap(x, cy, w, 15.0, W::M, TEXT, &grouped(&a)) + 14.0;
        }
        Err(e) => {
            cy += ui.wrap(x, cy, w, 13.0, W::R, WARN, &e) + 12.0;
        }
    }
    if let Some(e) = &s.error {
        cy += ui.wrap(x, cy, w, 13.0, W::R, ERR, e) + 10.0;
    }
    let left: &[(&str, Style, Action)] = if back {
        &[("Back to the seeds", Style::Secondary, sa(S::Keys))]
    } else {
        &[]
    };
    cy += buttons_and_next(ui, x, cy, w, left, Some(("Make the wallet", sa(S::Make))));
    cy - y
}

/// The kind's name with its class: single key or multisig.
fn kind_label(kind: NewKind) -> &'static str {
    kind.name()
}

/// A line of typing, `h` 40: what is typed, its end in view, or `hint`
/// dimmed when empty, with a caret while typing goes to it.
#[allow(clippy::too_many_arguments)]
fn field(
    ui: &mut Ui,
    x: f32,
    y: f32,
    w: f32,
    text: &str,
    hint: &str,
    focused: bool,
    action: Action,
) {
    ui.fill(x, y, w, 40.0, 8.0, BG);
    ui.stroke(x, y, w, 40.0, 8.0, if focused { ACCENT } else { BORDER });
    if text.is_empty() && !focused {
        let hint = ui.fit(13.0, W::R, hint, w - 24.0);
        ui.text_mid(x + 12.0, y, 40.0, 13.0, W::R, DIM, &hint);
    } else {
        // The end typed last stays in view.
        let room = w - 30.0;
        let mut shown = text.to_string();
        if ui.measure(14.0, W::M, &shown) > room {
            let chars: Vec<char> = text.chars().collect();
            let mut from = 0;
            while from < chars.len() {
                let tail: String = std::iter::once('…')
                    .chain(chars[from..].iter().copied())
                    .collect();
                if ui.measure(14.0, W::M, &tail) <= room {
                    shown = tail;
                    break;
                }
                from += 1;
            }
        }
        ui.selection(x + 12.0, y, 40.0, 14.0, W::M, &shown, focused);
        ui.text_mid(x + 12.0, y, 40.0, 14.0, W::M, TEXT, &shown);
        if focused && !ui.select_all {
            let cx = x + 13.0 + ui.measure(14.0, W::M, &shown);
            ui.caret(cx, y + 11.0, 18.0);
        }
    }
    ui.hit(x, y, w, 40.0, action);
}
