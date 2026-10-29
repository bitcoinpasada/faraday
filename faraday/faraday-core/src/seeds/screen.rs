//! A wallet made from the seeds in hand, drawn (`seeds.rs`): the wallet's
//! shape: M of N on two sliders, a box for each cosigner whose seed is
//! not here, the kind, the path and the first address. Restore's seeds
//! card and the Spend tab's What do you have? draw it.

use crate::Action;
use crate::create::NewKind;
use crate::drawing::View;
use crate::screens::{buttons_and_next, stepper, wrap_buttons};
use crate::seeds::{self, Focus, SeedsAction as S, SeedsState};
use crate::strings::EN;
use crate::strings::fill;
use crate::ui::pal::*;
use crate::ui::{CoreUi, Style, Ui, W, grouped};
use crate::wallet::FileKind;
use faraday_draw::tokens::{BODY_L, BODY_M, BODY_S, BUTTON_H, LABEL_L, TITLE_L};

fn sa(a: S) -> Action {
    Action::Seeds(a)
}

/// The other ways a seed comes in, after typing and scanning: a seed an
/// open vault holds, Unlock for a vault in Files (`unlock`, and back to
/// this step), a key already loaded and not in `here`.
pub(crate) fn more_ways(
    app: &View,
    here: &[[u8; 4]],
    unlock: bool,
    labels: &mut Vec<(String, Style, Action)>,
) {
    for (v, r, fp) in app.vault_seeds() {
        let name = &app.vaults.open[v].name;
        labels.push((
            fill(
                EN.seeds_load_from_vault,
                &[&fp.to_string(), &name.to_string()],
            ),
            Style::Secondary,
            sa(S::FromVault(v, r)),
        ));
    }
    // A vault made now holds no seed, so there is no Make a vault.
    if unlock
        && let Some((label, a)) = app.vault_way(crate::Screen::Restore)
        && !matches!(a, Action::Vault(crate::vault::VaultAction::CreateFrom(_)))
    {
        labels.push((label, Style::Secondary, a));
    }
    for k in &app.session.keys {
        let f = k.fingerprint;
        if !here.contains(&f.0) {
            labels.push((
                fill(
                    EN.seeds_use,
                    &[&crate::wallet::key_line(f, &k.label).to_string()],
                ),
                Style::Secondary,
                sa(S::UseLoaded(f.0)),
            ));
        }
    }
}

/// M of N, large, and its two sliders (one for MuSig2, where every key
/// signs). Returns the height used.
pub(crate) fn quorum(ui: &mut Ui, s: &SeedsState, x: f32, y: f32, w: f32) -> f32 {
    let mut cy = y;
    let n = s.n.max(1);
    let big = fill(EN.seeds_quorum, &[&s.m.max(1).to_string(), &n.to_string()]);
    ui.text_as(x, cy, TITLE_L.medium(), Role::OnSurface, &big);
    let bw = ui.measure_as(TITLE_L.medium(), &big);
    let line = if n == 1 {
        EN.seeds_one_key_signs.to_string()
    } else {
        fill(
            EN.seeds_of_keys,
            &[
                &s.m.to_string(),
                (if s.m == 1 {
                    EN.seeds_signature
                } else {
                    EN.seeds_signatures_inline
                }),
                &n.to_string(),
            ],
        )
    };
    ui.text_mid_as(
        (x + bw + 14.0, cy, 30.0),
        BODY_S,
        Role::OnSurfaceVariant,
        &line,
    );
    cy += 42.0;
    let (sx, sw) = if ui.compact {
        (x, w)
    } else {
        (x + 130.0, (w - 130.0).min(520.0))
    };
    for (id, label) in [
        (seeds::SLIDE_M, EN.seeds_signatures),
        (seeds::SLIDE_N, EN.seeds_keys),
    ] {
        let Some((lo, hi, v)) = s.slider_at(id) else {
            continue;
        };
        if ui.compact {
            ui.text_as(x, cy, LABEL_L, Role::OnSurfaceVariant, label);
            cy += ui.line_as(LABEL_L);
        } else {
            ui.text_mid_as((x, cy, 40.0), LABEL_L, Role::OnSurfaceVariant, label);
        }
        cy += ui.slider(sx, cy, sw, id, lo as u8, hi as u8, v as u8) + 4.0;
    }
    cy - y
}

/// Cosigner `k`'s box, numbered `slot`: its key once one reads, with
/// Change; else the field, and for the first box without a key, Scan and
/// the key files in Files. Returns the height used.
#[allow(clippy::too_many_arguments)]
pub(crate) fn cosigner(
    app: &View,
    ui: &mut Ui,
    s: &SeedsState,
    k: usize,
    slot: usize,
    x: f32,
    y: f32,
    w: f32,
) -> f32 {
    let mut cy = y;
    let text = s.cosigners.get(k).map(String::as_str).unwrap_or("");
    let focused = s.focus == Some(Focus::Cosigner(k as u8));
    ui.text_mid_as(
        (x, cy, 40.0),
        BODY_S,
        Role::OnSurfaceVariant,
        &slot.to_string(),
    );
    match seeds::cosigner_key(text).filter(|_| !focused) {
        Some(key) => {
            let cw = ui.measure_as(LABEL_L, EN.seeds_change) + 32.0;
            let shown = ui.fit_as(BODY_M.mono(), &seeds::cosigner_label(&key), w - 34.0 - cw);
            ui.text_mid_as((x + 22.0, cy, 40.0), BODY_M.mono(), Role::Ok, &shown);
            ui.button(
                x + w - cw,
                cy + 2.0,
                Some(cw),
                BUTTON_H,
                EN.seeds_change,
                Style::Ghost,
                sa(S::Clear(k as u8)),
            );
            cy += 46.0;
        }
        None => {
            // Use a sample goes in the "Demo only" card, for the first
            // empty box (`docs/DEMO.md` §14).
            let first = (0..k).all(|j| !app.seeds_sample_shown(s, j));
            if first && app.seeds_sample_shown(s, k) {
                crate::samples::put::offer(ui, sa(S::Sample(k as u8)));
            }
            field(
                ui,
                x + 22.0,
                cy,
                w - 22.0,
                text,
                EN.seeds_type_or_paste,
                focused,
                sa(S::Focus(Focus::Cosigner(k as u8))),
            );
            cy += 48.0;
            if !text.trim().is_empty() && !focused {
                cy += ui.wrap_as(
                    (x + 22.0, cy, w - 22.0),
                    BODY_S,
                    Role::Error,
                    EN.seeds_not_an_account,
                ) + 6.0;
            }
            // Scan and the key files in Files fill the box typing goes
            // to, else the first open one.
            let target = if s.fixed {
                focused || s.focus.is_none() && s.open_cosigner() == Some(k)
            } else {
                s.open_cosigner() == Some(k)
            };
            if target {
                let files: Vec<(usize, &str)> = app
                    .inbox
                    .iter()
                    .enumerate()
                    .filter(|(_, i)| i.kind == FileKind::Key)
                    .map(|(k, i)| (k, i.name.as_str()))
                    .collect();
                let labels: Vec<String> = files
                    .iter()
                    .map(|(_, n)| fill(EN.seeds_use, &[n]))
                    .collect();
                let mut items: Vec<(&str, Style, Action)> =
                    vec![(EN.seeds_scan, Style::Secondary, Action::Scan)];
                for ((i, _), l) in files.iter().zip(&labels) {
                    items.push((l.as_str(), Style::Secondary, sa(S::UseFile(*i))));
                }
                cy += wrap_buttons(ui, x + 22.0, cy, w - 22.0, 36.0, &items);
            }
        }
    }
    cy - y
}

/// The wallet's shape: M of N, the cosigners' account keys, the kind,
/// the path, the first address, and Make the wallet. `back` adds Back
/// to the seeds. Returns the height used.
#[allow(clippy::too_many_arguments)]
#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
pub(crate) fn shape(
    app: &View,
    ui: &mut Ui,
    s: &SeedsState,
    back: bool,
    x: f32,
    y: f32,
    w: f32,
) -> f32 {
    let mut cy = y;
    let net = app.session.network();
    let have = app.seeds_here(s).len();
    cy += quorum(ui, s, x, cy, w);
    // A box for each key whose seed is not here.
    if !s.cosigners.is_empty() {
        cy += 6.0;
        cy += ui.wrap_as(
            (x, cy, w),
            LABEL_L,
            Role::OnSurfaceVariant,
            EN.seeds_cosigners_account,
        ) + 10.0;
        for k in 0..s.cosigners.len() {
            cy += cosigner(app, ui, s, k, have + k + 1, x, cy, w);
        }
    }
    // The kind.
    cy += 8.0;
    ui.text_as(x, cy, LABEL_L, Role::OnSurfaceVariant, EN.seeds_kind);
    cy += ui.line_as(LABEL_L) + faraday_draw::tokens::GAP_SMALL;
    for (k, kind) in s.kinds().iter().enumerate() {
        let on = *kind == s.kind;
        ui.checkbox(x, cy + 11.0, on, true);
        let name = kind_label(*kind);
        ui.text_mid_as(
            (x + 30.0, cy, 40.0),
            LABEL_L,
            if on {
                Role::OnSurface
            } else {
                Role::OnSurfaceVariant
            },
            name,
        );
        let rh = if ui.compact {
            let l = ui.fit_as(BODY_S, kind.line(), w - 30.0);
            ui.text_mid_as(
                (x + 30.0, cy + 28.0, 20.0),
                BODY_S,
                Role::OnSurfaceVariant,
                &l,
            );
            52.0
        } else {
            let nw = ui.measure_as(LABEL_L, name);
            let l = ui.fit_as(BODY_S, kind.line(), w - 60.0 - nw);
            ui.text_right_as((x + w, cy, 40.0), BODY_S, Role::OnSurfaceVariant, &l);
            40.0
        };
        ui.press_box(x, cy, w, rh, sa(S::Kind(k as u8)));
        ui.rule(x, cy + rh, w, Role::OutlineVariant);
        cy += rh + 2.0;
    }
    // The path.
    cy += 12.0;
    ui.text_as(
        x,
        cy,
        LABEL_L,
        Role::OnSurfaceVariant,
        EN.seeds_derivation_path,
    );
    cy += ui.line_as(LABEL_L) + faraday_draw::tokens::GAP_SMALL;
    let standard = s.custom.is_none();
    ui.checkbox(x, cy + 11.0, standard, true);
    let std_path = s.kind.path_at(net, s.account);
    ui.text_mid_as(
        (x + 30.0, cy, 40.0),
        LABEL_L,
        Role::OnSurface,
        EN.seeds_standard,
    );
    let pw = ui.measure_as(LABEL_L, EN.seeds_standard) + 44.0;
    ui.text_mid_as(
        (x + pw, cy, 40.0),
        BODY_M.mono(),
        if standard {
            Role::OnSurface
        } else {
            Role::OnSurfaceVariant
        },
        &std_path,
    );
    ui.press_box(x, cy, w, 40.0, sa(S::Standard));
    cy += 44.0;
    // The account, where the kind's path has one.
    if s.kind != NewKind::MultiLegacy {
        stepper(
            ui,
            x + 30.0,
            cy,
            w - 30.0,
            EN.seeds_account,
            s.account as usize,
            sa(S::Account(false)),
            sa(S::Account(true)),
            true,
        );
        cy += 50.0;
    }
    ui.checkbox(x, cy + 11.0, !standard, true);
    ui.text_mid_as(
        (x + 30.0, cy, 40.0),
        LABEL_L,
        Role::OnSurface,
        EN.seeds_custom_path,
    );
    ui.press_box(x, cy, w, 40.0, sa(S::Custom));
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
            ui.text_as(
                x,
                cy,
                BODY_S,
                Role::OnSurfaceVariant,
                EN.seeds_first_receive,
            );
            cy += ui.advance_as(BODY_S, 22.0);
            cy += ui.wrap_as((x, cy, w), BODY_L.mono(), Role::OnSurface, &grouped(&a)) + 14.0;
        }
        Err(e) => {
            cy += ui.wrap_as((x, cy, w), BODY_S, Role::Warning, &e) + 12.0;
        }
    }
    if let Some(e) = &s.error {
        cy += ui.wrap_as((x, cy, w), BODY_S, Role::Error, e) + 10.0;
    }
    let left: &[(&str, Style, Action)] = if back {
        &[(EN.seeds_back_to_the_seeds, Style::Secondary, sa(S::Keys))]
    } else {
        &[]
    };
    cy += buttons_and_next(
        ui,
        x,
        cy,
        w,
        left,
        Some((EN.seeds_make_the_wallet, sa(S::Make))),
    );
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
    ui.fill(x, y, w, 40.0, 8.0, Role::SurfaceContainerHighest);
    ui.stroke(
        x,
        y,
        w,
        40.0,
        8.0,
        if focused {
            Role::Primary
        } else {
            Role::OutlineVariant
        },
    );
    if text.is_empty() && !focused {
        let hint = ui.fit_as(BODY_S, hint, w - 24.0);
        ui.text_mid_as((x + 12.0, y, 40.0), BODY_S, Role::OnSurfaceVariant, &hint);
    } else {
        // The end typed last stays in view, or the caret.
        ui.typed(
            x + 12.0,
            y,
            40.0,
            BODY_M.size,
            W::M,
            Role::OnSurface,
            text,
            w - 30.0,
            focused,
            action,
        );
    }
    ui.press_box(x, y, w, 40.0, action);
}
