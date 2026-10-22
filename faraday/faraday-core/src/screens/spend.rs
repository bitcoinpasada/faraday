//! `Screen::Spend`: signing a transaction.

use super::parts::{made_place, side_panel_w, spaced};
use crate::drawing::{Scrolled, View};
use crate::strings::{EN, fill};
use crate::ui::pal::*;
use crate::ui::{Style, Ui, W, btc, short, thousands};
use crate::wallet::{Session, fp_text, network_name, step, txid_known};
use crate::{Action, Screen, flow, guide};
use faraday_draw::widgets::Icon;
use faraday_psbt::{Level, OutputKind};
use faraday_ui::parts::{title, wrap_buttons};

/// The nonce check: each signature against its key and this
/// transaction, each one made here recomputed from its key, and any nonce
/// used twice. Returns its height.
#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
fn nonce_check(app: &View, ui: &mut Ui, psbt: &faraday_psbt::Psbt, x: f32, y: f32, w: f32) -> f32 {
    use faraday_psbt::verify::Verdict;
    let Some(check) = app.nonce_check(psbt) else {
        return 0.0;
    };
    let mut cy = y;
    let here = check.rows.iter().filter(|r| r.recomputed.is_some()).count();
    let (head, tone) = if check.sound() {
        (
            fill(
                EN.nonce_check_nonce_check_valid,
                &[
                    &check.rows.len().to_string(),
                    (if check.rows.len() == 1 {
                        EN.nonce_check_signature
                    } else {
                        EN.nonce_check_signatures
                    }),
                    &here.to_string(),
                ],
            ),
            OK,
        )
    } else {
        (EN.nonce_check_nonce_check.to_string(), ERR)
    };
    ui.icon(
        x,
        cy + 8.0,
        20.0,
        if check.sound() {
            Icon::Success
        } else {
            Icon::Error
        },
        12.0,
        tone,
    );
    if ui.compact {
        // Wrapped beside its icon on a small panel.
        cy += ui
            .wrap(x + 26.0, cy + 8.0, w - 26.0, 14.0, W::S, tone, &head)
            .max(20.0)
            + 16.0;
    } else {
        ui.text_mid(x + 26.0, cy, 36.0, 14.0, W::S, tone, &head);
        cy += 40.0;
    }
    for reuse in &check.reused {
        let inputs: Vec<String> = reuse.inputs.iter().map(|i| i.to_string()).collect();
        let line = fill(
            EN.nonce_check_one_nonce,
            &[&reuse.name().to_string(), &inputs.join(", ").to_string()],
        );
        cy += ui.wrap(x, cy, w, 13.0, W::S, ERR, &line) + 8.0;
    }
    const SHOWN: usize = 12;
    for r in check.rows.iter().take(SHOWN) {
        ui.text_mid(
            x,
            cy,
            30.0,
            12.0,
            W::R,
            DIM,
            &fill(EN.nonce_check_input, &[&r.input.to_string()]),
        );
        ui.text_mid(x + 70.0, cy, 30.0, 13.0, W::M, TEXT, &r.name);
        let (t, c) = match (r.verdict, r.recomputed) {
            (Verdict::Invalid, _) => (EN.nonce_check_invalid.to_string(), ERR),
            (Verdict::Unchecked(why), _) => {
                (fill(EN.nonce_check_not_checked, &[&why.to_string()]), DIM)
            }
            (Verdict::Valid, Some(Some(mode))) => {
                (fill(EN.nonce_check_valid_nonce, &[&mode.to_string()]), OK)
            }
            (Verdict::Valid, Some(None)) => (EN.nonce_check_valid_not.to_string(), ERR),
            (Verdict::Valid, None) => (EN.nonce_check_valid_signed.to_string(), MUTED),
        };
        if ui.compact {
            // The verdict in full, under the input.
            let th = ui.wrap(x, cy + 28.0, w, 12.0, W::R, c, &t);
            ui.rule(x, cy + 34.0 + th, w, INNER);
            cy += th + 36.0;
            continue;
        }
        let t = ui.fit(12.0, W::R, &t, w - 170.0);
        ui.text_right(x + w, cy, 30.0, 12.0, W::R, c, &t);
        ui.rule(x, cy + 30.0, w, INNER);
        cy += 32.0;
    }
    if check.rows.len() > SHOWN {
        let rest = &check.rows[SHOWN..];
        let ok = rest
            .iter()
            .all(|r| !r.verdict.is_invalid() && !matches!(r.recomputed, Some(None)));
        ui.text_mid(
            x,
            cy,
            30.0,
            12.0,
            W::R,
            if ok { MUTED } else { ERR },
            &fill(
                EN.nonce_check_more,
                &[
                    &rest.len().to_string(),
                    (if ok {
                        EN.nonce_check_none_wrong
                    } else {
                        EN.nonce_check_one_is
                    }),
                ],
            ),
        );
        cy += 32.0;
    }
    cy + 12.0 - y
}

const STEPS: [&str; 10] = [
    EN.app_wallet,
    EN.app_check,
    EN.app_transaction,
    EN.app_transaction_id,
    EN.app_signers,
    EN.app_sign,
    EN.app_signatures_for,
    EN.app_finish,
    EN.app_path,
    EN.app_nonces,
];

pub(super) fn draw(app: &View, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    let panel_w = side_panel_w(cw);
    let col_x = x0 + 40.0;
    let col_w = cw - panel_w - 72.0;
    let needed = app.spend_needed();
    if app.spend.is_none() {
        title(ui, col_x, 28.0, EN.spend_sign_a_transaction);
        ui.text(col_x, 80.0, 15.0, W::R, MUTED, EN.spend_no_transaction);
        ui.button(
            col_x,
            116.0,
            None,
            40.0,
            EN.spend_open_files,
            Style::Secondary,
            Action::Nav(Screen::Files),
        );
        return;
    }
    let (wallet_idx, steps) = {
        let s = app.spend.as_ref().expect("spend");
        (s.wallet, s.steps.clone())
    };

    let (cards, scroll, guided) = {
        let s = app.spend.as_ref().expect("spend");
        let kind = wallet_idx
            .and_then(|w| app.session.wallets.get(w))
            .map(|w| crate::wallet::Kind::of(w.policy()));
        let cards: Vec<flow::Card> = steps
            .iter()
            .map(|&n| flow::Card {
                title: STEPS[n as usize].to_string(),
                summary: step_summary(app, n, wallet_idx, needed),
                mono: false,
                done: s.done[n as usize],
                open: s.open == Some(n),
                default: n == step::CHECK,
                toggle: Action::Step(n),
                guide: Some(guide::spend(n, kind, needed, s, app.medium)),
            })
            .collect();
        (
            cards,
            flow::Scroll {
                y: s.scroll,
                follow: s.follow,
            },
            app.guided,
        )
    };
    let compact = ui.compact;
    let col = flow::Column {
        area_x: x0,
        area_w: if compact { cw } else { cw - panel_w },
        x: col_x,
        w: col_w,
        h: if compact { h - SPEND_BAR } else { h },
        back: Some(app.named_back((EN.spend_wallets, Action::Nav(Screen::WalletsStart)))),
        heading: EN.spend_sign_a_transaction,
        guided,
        switch: true,
        note: None,
        chip: None,
        chip_tap: None,
        cancel: app.cancel_offered().is_some(),
    };
    let (next, again) = {
        let app_ref: &View = app;
        let mut body = |ui: &mut Ui, i: usize, x: f32, y: f32, w: f32| -> f32 {
            step_body(app_ref, ui, steps[i], x, y, w, wallet_idx, needed)
        };
        flow::column(ui, &col, &cards, scroll, &mut body)
    };
    app.scroll(Scrolled::Spend, next, again);

    // The panel; on a small panel, the foot, whose button is the open
    // step's Continue while it is read.
    if compact {
        let step = ui.pin.take();
        spend_bar(app, ui, x0, cw, h - SPEND_BAR, needed, step);
    } else {
        spend_panel(app, ui, x0 + cw - panel_w, panel_w, h, wallet_idx, needed);
    }
}

/// Where a signing stands: the signatures there, how many are missing,
/// the line that says what is left, and the main button's label.
fn spend_status(app: &View, needed: usize) -> (usize, usize, String, String) {
    let s = app.spend.as_ref().expect("spend");
    let i = &s.inspection;
    // A threshold spend counts partial signatures and signs as shares.
    let threshold = threshold_of(s);
    let signed_count = match threshold {
        Some(t) if !s.complete => t.signed.len(),
        _ => s.signers.len(),
    };
    let count = if s.complete {
        needed
    } else {
        signed_count.min(needed)
    };
    let missing = if s.complete {
        0
    } else {
        needed.saturating_sub(signed_count).max(1)
    };
    let can_sign_here = match threshold {
        Some(t) => t
            .ours
            .iter()
            .filter(|id| !t.signed.contains(id))
            .map(|id| fill(EN.spend_status_share, &[&(id + 1).to_string()]))
            .collect::<Vec<_>>(),
        None => i
            .participating_keys
            .iter()
            .filter(|fp| app.session.keys.key_label(**fp).is_some() && !s.signers.contains(&fp.0))
            .map(|fp| fp_text(*fp))
            .collect::<Vec<_>>(),
    };
    let carry = threshold.is_some() && (s.carry_out.is_some() || s.out_signed) && !s.complete;
    let need_line = if carry {
        if s.out_signed {
            EN.spend_status_carry.to_string()
        } else {
            EN.spend_status_the_next.to_string()
        }
    } else if s.spend.finished.is_some() && s.out_signed && s.out_tx {
        fill(EN.spend_status_both_wait, &[app.medium.for_the()])
    } else if missing == 0 {
        EN.spend_status_complete.to_string()
    } else if threshold.is_some() && !can_sign_here.is_empty() && missing > 1 {
        fill(
            EN.spend_status_sign_here_more,
            &[
                &(missing - 1).to_string(),
                (if missing == 2 {
                    EN.spend_status_its
                } else {
                    EN.spend_status_their
                }),
            ],
        )
    } else if !can_sign_here.is_empty() && missing > 1 {
        fill(EN.spend_status_sign_here_and, &[&(missing - 1).to_string()])
    } else if !can_sign_here.is_empty() {
        EN.spend_status_sign_here.to_string()
    } else {
        crate::wallet::collect_line(missing, count)
    };
    let label = if carry {
        if s.out_signed {
            EN.spend_status_open_files.to_string()
        } else {
            EN.spend_status_keep_the.to_string()
        }
    } else if s.spend.finished.is_some() {
        // Both went For the stick at Finish: the next step is writing.
        if app.sticks.is_empty() {
            fill(EN.spend_status_insert, &[app.medium.a()])
        } else {
            EN.spend_status_open_files.to_string()
        }
    } else if missing == 0 {
        EN.spend_status_finish.to_string()
    } else if !can_sign_here.is_empty() {
        fill(
            EN.spend_status_sign_with,
            &[&can_sign_here.join(", ").to_string()],
        )
    } else {
        crate::wallet::collect_line(missing, count)
    };
    (count, missing, need_line, label)
}

/// The foot of Sign a transaction on a small panel: the signatures, what
/// is left, and the main button. Returns its height.
fn spend_bar(
    app: &View,
    ui: &mut Ui,
    x0: f32,
    cw: f32,
    y: f32,
    needed: usize,
    step: Option<(String, Style, Action)>,
) {
    let (count, missing, need_line, label) = spend_status(app, needed);
    let (label, style, action) = step.unwrap_or((label, Style::Primary, Action::Primary));
    let (x, w) = (x0 + crate::compact::M, cw - 2.0 * crate::compact::M);
    ui.fill(x0, y, cw, SPEND_BAR, 0.0, SIDEBAR);
    ui.fill(x0, y, cw, 1.0, 0.0, LINE);
    let pw = ui.pips(x, y + 14.0, needed, count);
    ui.text_mid(
        x + pw + 10.0,
        y + 4.0,
        26.0,
        13.0,
        W::S,
        TEXT,
        &fill(EN.spend_bar_of, &[&count.to_string(), &needed.to_string()]),
    );
    let line = ui.fit(12.0, W::R, &need_line, w);
    ui.text(
        x,
        y + 30.0,
        12.0,
        W::R,
        if missing == 0 { OK } else { WARN },
        &line,
    );
    let label = ui.fit(15.0, W::S, &label, w - 24.0);
    ui.button(x, y + 52.0, Some(w), 44.0, &label, style, action);
}

/// The height of that foot.
const SPEND_BAR: f32 = 104.0;

/// The first input's threshold view, when the spend is from a threshold
/// wallet.
fn threshold_of(s: &crate::SpendState) -> Option<&faraday_psbt::ThresholdInput> {
    s.inspection
        .inputs
        .first()
        .and_then(|i| i.threshold.as_ref())
}

fn recipients(inspection: &faraday_psbt::Inspection) -> Vec<&faraday_psbt::OutputInfo> {
    inspection
        .outputs
        .iter()
        .filter(|o| matches!(o.kind, OutputKind::Recipient))
        .collect()
}

pub(crate) fn step_summary(app: &View, n: u8, wallet: Option<usize>, needed: usize) -> String {
    let s = app.spend.as_ref().expect("spend");
    let i = &s.inspection;
    let reached = s.done[n as usize] || s.open == Some(n);
    match n {
        0 => match wallet.and_then(|w| app.session.wallets.get(w)) {
            Some(w) => format!("{} · {}", w.name, Session::shape(w)),
            None => EN.step_summary_no_loaded.to_string(),
        },
        1 if reached => EN.step_summary_addresses.to_string(),
        2 if reached => fill(
            EN.step_summary_sends,
            &[
                &btc(i.amount_to_others.to_sat()).to_string(),
                &(i.fee_rate_sat_vb
                    .map(|r| format!("{r:.1}"))
                    .unwrap_or_else(|| "?".into()))
                .to_string(),
            ],
        ),
        4 if reached && threshold_of(s).is_some() => {
            let t = threshold_of(s).expect("threshold");
            fill(
                EN.step_summary_here_needed,
                &[&t.ours.len().to_string(), &needed.to_string()],
            )
        }
        4 if reached => {
            let here = i
                .participating_keys
                .iter()
                .filter(|fp| app.session.keys.key_label(**fp).is_some())
                .count();
            fill(
                EN.step_summary_here_needed,
                &[&here.to_string(), &needed.to_string()],
            )
        }
        5 => {
            if s.spend.signed_here.is_empty() {
                EN.step_summary_not_signed.to_string()
            } else {
                let fps: Vec<String> = s.spend.signed_here.iter().map(|f| fp_text(*f)).collect();
                fill(EN.step_summary_signed_with, &[&fps.join(", ").to_string()])
            }
        }
        6 => fill(
            EN.step_summary_of,
            &[&s.signers.len().to_string(), &needed.to_string()],
        ),
        9 => match wallet.and_then(|w| app.session.wallets.get(w)) {
            Some(w) => {
                let rows = crate::wallet::musig_nonces(&app.session, w, &s.spend.psbt);
                let have = rows.iter().filter(|r| r.3).count();
                fill(
                    EN.step_summary_of_nonces,
                    &[&have.to_string(), &rows.len().to_string()],
                )
            }
            None => String::new(),
        },
        8 => {
            let n = wallet
                .and_then(|w| app.session.wallets.get(w))
                .map(|w| w.policy().spend_paths().len())
                .unwrap_or(0);
            format!(
                "{n} {}",
                if n == 1 {
                    EN.step_summary_way_to
                } else {
                    EN.step_summary_ways_to
                }
            )
        }
        7 => {
            if s.spend.finished.is_some() {
                EN.step_summary_signed.to_string()
            } else {
                EN.step_summary_not_finished.to_string()
            }
        }
        _ => EN.step_summary_not_reached.to_string(),
    }
}

/// Draws a step's body with its top-left at (x, y). Returns its height.
#[allow(clippy::too_many_arguments)]
#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
pub(crate) fn step_body(
    app: &View,
    ui: &mut Ui,
    n: u8,
    x: f32,
    y: f32,
    w: f32,
    wallet: Option<usize>,
    needed: usize,
) -> f32 {
    let s = app.spend.as_ref().expect("spend");
    let i = &s.inspection;
    let mut cy = y;
    // A step's Continue: on a small panel it takes the foot's button
    // ([`spend_bar`]) while the step is read.
    let next = |ui: &mut Ui, cy: f32, label: &str| {
        if ui.pinning {
            ui.pin = Some((label.to_string(), Style::Primary, Action::StepNext(n)));
            return;
        }
        let bw = ui.measure(14.0, W::S, label) + 36.0;
        ui.button(
            x + w - bw,
            cy,
            Some(bw),
            40.0,
            label,
            Style::Primary,
            Action::StepNext(n),
        );
    };
    match n {
        0 => {
            match wallet.and_then(|wi| app.session.wallets.get(wi)) {
                Some(wl) => {
                    ui.text(x, cy, 14.0, W::S, TEXT, &wl.name);
                    ui.text(
                        x,
                        cy + 22.0,
                        12.0,
                        W::R,
                        MUTED,
                        &fill(
                            EN.step_from,
                            &[&Session::shape(wl).to_string(), &wl.source.to_string()],
                        ),
                    );
                }
                None => {
                    ui.wrap(x, cy, w - 140.0, 13.0, W::R, WARN, EN.step_no_loaded_wallet);
                }
            }
            next(ui, cy, EN.step_continue);
            cy += 44.0;
        }
        1 => {
            match wallet.and_then(|wi| app.session.wallets.get(wi)) {
                Some(wl) => {
                    for (label, change, idx) in [
                        (EN.step_receive_0_0, false, 0),
                        (EN.step_receive_0_1, false, 1),
                        (EN.step_change_1_0, true, 0),
                    ] {
                        ui.text(x, cy + 2.0, 12.0, W::R, MUTED, label);
                        let (a, held) = app.receive_line(wl, change, idx);
                        // On a small panel the address goes under its label.
                        let ax = if ui.compact {
                            cy += 20.0;
                            x
                        } else {
                            x + 96.0
                        };
                        let col = if held { WARN } else { TEXT };
                        let used = ui.wrap(ax, cy, x + w - ax, 13.0, W::M, col, &a);
                        cy += used.max(20.0) + 8.0;
                        ui.rule(x, cy - 4.0, w, INNER);
                    }
                }
                None => {
                    ui.text(x, cy, 13.0, W::R, MUTED, EN.step_no_wallet_to_show);
                    cy += 24.0;
                }
            }
            cy += 8.0;
            next(ui, cy, EN.step_continue);
            cy += 44.0;
        }
        2 => {
            let rows_top = cy;
            let mut rows: Vec<ReviewRow> = Vec::new();
            for o in recipients(i) {
                rows.push((
                    EN.step_sends.into(),
                    btc(o.amount.to_sat()),
                    fill(
                        EN.step_to,
                        &[&short(&app.session.shown(&o.address)).to_string()],
                    ),
                    None,
                ));
            }
            for o in &i.outputs {
                match o.kind {
                    OutputKind::WalletChange { change, index, .. }
                    | OutputKind::Change { change, index, .. } => {
                        rows.push((
                            EN.step_change.into(),
                            btc(o.amount.to_sat()),
                            fill(
                                EN.step_to_2,
                                &[
                                    &(if change { 1 } else { 0 }).to_string(),
                                    &index.to_string(),
                                ],
                            ),
                            Some((EN.step_verified, OK)),
                        ));
                    }
                    OutputKind::UnverifiedChange { .. } => {
                        rows.push((
                            EN.step_change.into(),
                            btc(o.amount.to_sat()),
                            fill(
                                EN.step_to,
                                &[&short(&app.session.shown(&o.address)).to_string()],
                            ),
                            Some((EN.step_unverified, WARN)),
                        ));
                    }
                    OutputKind::Recipient => {}
                }
            }
            rows.push((
                EN.step_fee.into(),
                btc(i.fee.to_sat()),
                fill(
                    EN.step_sat_sat_vb,
                    &[
                        &thousands(i.fee.to_sat()).to_string(),
                        &(i.fee_rate_sat_vb
                            .map(|r| format!("{r:.1}"))
                            .unwrap_or_else(|| "?".into()))
                        .to_string(),
                    ],
                ),
                None,
            ));
            let unchecked = i.inputs.iter().any(|inp| inp.amount.is_none());
            rows.push((
                EN.step_spends.into(),
                btc(i.total_in.to_sat()),
                format!(
                    "{} {}",
                    i.inputs.len(),
                    if i.inputs.len() == 1 {
                        EN.step_input_inline
                    } else {
                        EN.step_inputs
                    }
                ),
                if unchecked {
                    Some((EN.step_amount_unchecked, WARN))
                } else {
                    None
                },
            ));
            // The transaction's id, at the foot: known before signing
            // for a segwit spend, not for a legacy one.
            if txid_known(&s.spend.psbt) {
                let segwit = i.inputs.iter().all(|inp| {
                    !matches!(inp.script_type, faraday_psbt::ScriptKind::P2pkh)
                        && !format!("{:?}", inp.script_type).contains("Sh")
                        || format!("{:?}", inp.script_type).contains("Wsh")
                });
                rows.push((
                    EN.step_txid.into(),
                    short(&s.spend.txid.to_string()),
                    if segwit {
                        EN.step_unchanged_by_signing
                    } else {
                        EN.step_changes_when_signed
                    }
                    .to_string(),
                    None,
                ));
            } else {
                rows.push((
                    EN.step_txid.into(),
                    EN.step_not_known_until.to_string(),
                    String::new(),
                    None,
                ));
            }
            // On a small panel each row is two lines: what and how much,
            // then where and its note.
            let compact = ui.compact;
            let row_h = if compact { 52.0 } else { 40.0 };
            let box_h = rows.len() as f32 * row_h;
            ui.fill(x, rows_top, w, box_h, 10.0, BG);
            ui.stroke(x, rows_top, w, box_h, 10.0, INNER);
            for (k, (label, amount, rest, note)) in rows.iter().enumerate() {
                let ry = rows_top + k as f32 * row_h;
                if compact {
                    ui.text_mid(x + 12.0, ry + 4.0, 24.0, 12.0, W::R, MUTED, label);
                    ui.text_right(x + w - 12.0, ry + 4.0, 24.0, 14.0, W::M, TEXT, amount);
                    let nw = note.map_or(0.0, |(t, _)| ui.measure(12.0, W::R, t) + 8.0);
                    let rest = ui.fit(12.0, W::R, rest, w - 24.0 - nw);
                    ui.text_mid(x + 12.0, ry + 26.0, 22.0, 12.0, W::R, MUTED, &rest);
                    if let Some((t, c)) = note {
                        ui.text_right(x + w - 12.0, ry + 26.0, 22.0, 12.0, W::R, *c, t);
                    }
                } else {
                    ui.text_mid(x + 14.0, ry, row_h, 12.0, W::R, MUTED, label);
                    let aw = ui.text_mid(x + 84.0, ry, row_h, 14.0, W::M, TEXT, amount);
                    ui.text_mid(x + 98.0 + aw, ry, row_h, 13.0, W::R, MUTED, rest);
                    if let Some((t, c)) = note {
                        ui.text_right(x + w - 14.0, ry, row_h, 12.0, W::R, *c, t);
                    }
                }
                if k + 1 < rows.len() {
                    ui.rule(x, ry + row_h, w, SURFACE);
                }
            }
            cy += box_h + 12.0;
            // Warnings, every one as the inspector words it.
            let mut warned = false;
            for wn in &i.warnings {
                let c = match wn.level {
                    Level::Info => MUTED,
                    Level::Caution => WARN,
                    Level::Danger | Level::Blocked => ERR,
                };
                let used = ui.wrap(x + 22.0, cy, w - 22.0, 13.0, W::R, c, &wn.text);
                ui.icon(x, cy, 16.0, Icon::Warning, 10.0, c);
                cy += used.max(18.0) + 6.0;
                warned = true;
            }
            if !warned {
                let mut chx = x;
                chx += ui.chip(chx, cy, EN.step_no_warnings, OK, OK.with_alpha(26)) + 6.0;
                let net = app.session.network();
                let c = if net.is_mainnet() { OK } else { WARN };
                ui.chip(chx, cy, network_name(net), c, c.with_alpha(26));
                cy += 34.0;
            }
            if s.table && compact {
                // What and how much on a line, the outpoint or address
                // under it.
                let lines = i
                    .inputs
                    .iter()
                    .map(|inp| {
                        (
                            EN.step_input,
                            inp.amount
                                .map(|a| btc(a.to_sat()))
                                .unwrap_or_else(|| "?".into()),
                            format!("{}:{}", short(&inp.txid.to_string()), inp.vout),
                        )
                    })
                    .chain(i.outputs.iter().map(|o| {
                        (
                            EN.step_output,
                            btc(o.amount.to_sat()),
                            app.session.shown(&o.address),
                        )
                    }))
                    .collect::<Vec<_>>();
                for (label, amount, what) in &lines {
                    ui.text(x, cy, 12.0, W::R, MUTED, label);
                    ui.text_right(x + w, cy - 4.0, 20.0, 12.0, W::M, TEXT, amount);
                    cy += 18.0;
                    cy += ui.wrap(x, cy, w, 12.0, W::M, TEXT, what) + 10.0;
                }
                cy += 6.0;
            } else if s.table {
                for inp in &i.inputs {
                    let a = inp
                        .amount
                        .map(|a| btc(a.to_sat()))
                        .unwrap_or_else(|| "?".into());
                    ui.text(x, cy, 12.0, W::R, MUTED, EN.step_input);
                    ui.text(
                        x + 64.0,
                        cy,
                        12.0,
                        W::M,
                        TEXT,
                        &format!("{}:{}", short(&inp.txid.to_string()), inp.vout),
                    );
                    ui.text_right(x + w, cy - 4.0, 20.0, 12.0, W::M, TEXT, &a);
                    cy += 22.0;
                }
                for o in &i.outputs {
                    ui.text(x, cy, 12.0, W::R, MUTED, EN.step_output);
                    let shown = app.session.shown(&o.address);
                    let used = ui.wrap(x + 64.0, cy, w - 64.0 - 110.0, 12.0, W::M, TEXT, &shown);
                    ui.text_right(
                        x + w,
                        cy - 4.0,
                        20.0,
                        12.0,
                        W::M,
                        TEXT,
                        &btc(o.amount.to_sat()),
                    );
                    cy += used.max(18.0) + 4.0;
                }
                cy += 6.0;
            }
            ui.button(
                x,
                cy,
                None,
                38.0,
                if s.table {
                    EN.step_hide_the_table
                } else {
                    EN.step_show_the_table
                },
                Style::Secondary,
                Action::ToggleTable,
            );
            next(ui, cy, EN.step_continue);
            cy += 44.0;
        }
        4 if threshold_of(s).is_some() => {
            let t = threshold_of(s).expect("threshold");
            // The first location chooses who signs; after it the PSBT
            // names them.
            let first = t.signers.is_empty();
            if let Some(wl) = wallet.and_then(|wi| app.session.wallets.get(wi)) {
                // On a small panel a share is two lines: the share, then
                // who holds it with what it does.
                let compact = ui.compact;
                for (k, slot) in app.session.slots(wl).iter().enumerate() {
                    let id = k as u32;
                    ui.text_mid(
                        x,
                        cy,
                        42.0,
                        12.0,
                        W::R,
                        DIM,
                        &fill(EN.step_share_n, &[&(k + 1).to_string()]),
                    );
                    let fp = slot.fingerprint.map(fp_text).unwrap_or_default();
                    ui.text_mid(x + 64.0, cy, 42.0, 14.0, W::M, TEXT, &fp);
                    let (lx, ly) = if compact {
                        (x + 64.0, cy + 36.0)
                    } else {
                        (x + 166.0, cy)
                    };
                    let here = t.ours.contains(&id);
                    let (tag, fg, bg) = if t.signed.contains(&id) {
                        (EN.step_signed, OK, OK.with_alpha(30))
                    } else if here {
                        (EN.step_signs_here, OK, OK.with_alpha(30))
                    } else if t.signers.contains(&id) {
                        (EN.step_signs_at_its_own, ACCENT, ACCENT.with_alpha(26))
                    } else if first {
                        ("", MUTED, INNER)
                    } else {
                        (EN.step_not_signing, DIM, INNER)
                    };
                    let right = if first && !here {
                        let on = s.others.contains(&id);
                        let label = if on {
                            EN.step_signs_next
                        } else {
                            EN.step_choose
                        };
                        let bw = ui.measure(13.0, W::S, label) + 28.0;
                        ui.button(
                            x + w - bw,
                            ly + 5.0,
                            Some(bw),
                            32.0,
                            label,
                            if on { Style::Primary } else { Style::Secondary },
                            Action::TOther(id),
                        );
                        bw
                    } else if !tag.is_empty() {
                        let cw2 = ui.measure(12.0, W::R, tag) + 34.0;
                        ui.chip(x + w - cw2, ly + 8.0, tag, fg, bg);
                        cw2
                    } else {
                        0.0
                    };
                    let label = slot.held_by.clone().unwrap_or_default();
                    let label = ui.fit(13.0, W::R, &label, x + w - lx - right - 8.0);
                    ui.text_mid(lx, ly, 42.0, 13.0, W::R, MUTED, &label);
                    ui.rule(x, ly + 42.0, w, INNER);
                    cy = ly + 44.0;
                }
            }
            cy += 8.0;
            let line = if first {
                fill(
                    EN.step_needs_here_chosen,
                    &[
                        &needed.to_string(),
                        &t.ours.len().to_string(),
                        &s.others.len().to_string(),
                    ],
                )
            } else {
                fill(
                    EN.step_needs_signed,
                    &[&needed.to_string(), &t.signed.len().to_string()],
                )
            };
            ui.text_mid(x, cy, 40.0, 13.0, W::R, MUTED, &line);
            // Under the line when both do not fit on one row.
            let nw = ui.measure(14.0, W::S, EN.step_continue) + 36.0;
            if ui.measure(13.0, W::R, &line) + nw + 12.0 > w {
                cy += 40.0;
            }
            next(ui, cy, EN.step_continue);
            cy += 44.0;
        }
        4 => {
            let wl = wallet.and_then(|wi| app.session.wallets.get(wi).map(|wl| (wi, wl)));
            let may = app.may_load_keys();
            match wl {
                Some((wi, wl)) => {
                    let slots = app.session.slots(wl);
                    let passphrase_key = slots.iter().any(|s| s.held_by.is_none())
                        && !app.vault_passphrase_seeds().is_empty();
                    // On a small panel a key is two lines: the key, then
                    // whether it signs here and the way to bring it here.
                    let compact = ui.compact;
                    for (k, slot) in slots.iter().enumerate() {
                        ui.text_mid(x, cy, 42.0, 12.0, W::R, DIM, &(k + 1).to_string());
                        let fp = slot.fingerprint.map(fp_text).unwrap_or_default();
                        ui.text_mid(x + 22.0, cy, 42.0, 14.0, W::M, TEXT, &fp);
                        // A label that is its fingerprint is not drawn
                        // twice (`docs/history/SCAN-TO-SIGN.md` §3.3).
                        let label = slot
                            .held_by
                            .as_deref()
                            .map(str::trim)
                            .filter(|l| !l.is_empty() && !l.eq_ignore_ascii_case(&fp))
                            .unwrap_or_default();
                        let label = ui.fit(13.0, W::R, label, w - 124.0);
                        ui.text_mid(x + 124.0, cy, 42.0, 13.0, W::R, MUTED, &label);
                        if compact {
                            cy += 36.0;
                        }
                        // A key an open vault holds loads from it; any
                        // other not here says so.
                        let from_vault = slot.fingerprint.filter(|_| {
                            slot.held_by.is_none() && app.vault_key_for(slot).is_some()
                        });
                        // A seed an open vault keeps without the
                        // passphrase this wallet may use it with (F62).
                        let with_pass =
                            from_vault.is_none() && slot.held_by.is_none() && passphrase_key;
                        match from_vault {
                            None if with_pass => {
                                let l = EN.vault_load_with_a_passphrase;
                                let bw = ui.measure(13.0, W::S, l) + 28.0;
                                ui.button(
                                    x + w - bw,
                                    cy + 5.0,
                                    Some(bw),
                                    32.0,
                                    l,
                                    if may {
                                        Style::Secondary
                                    } else {
                                        Style::Disabled
                                    },
                                    Action::Vault(crate::vault::VaultAction::PassphraseKey(wi)),
                                );
                            }
                            Some(f) => {
                                let l = EN.step_load_from_vault;
                                let bw = ui.measure(13.0, W::S, l) + 28.0;
                                ui.button(
                                    x + w - bw,
                                    cy + 5.0,
                                    Some(bw),
                                    32.0,
                                    l,
                                    if may {
                                        Style::Secondary
                                    } else {
                                        Style::Disabled
                                    },
                                    Action::Vault(crate::vault::VaultAction::LoadKeyOf(
                                        f.0,
                                        crate::vault::SlotOf::Loaded(wi, k),
                                    )),
                                );
                            }
                            None => {
                                let (t, fg, bg) = if slot.held_by.is_some() {
                                    (EN.step_signs_here, OK, OK.with_alpha(30))
                                } else {
                                    (EN.step_not_here, MUTED, INNER)
                                };
                                let cw2 = ui.measure(12.0, W::R, t) + 34.0;
                                let chip_x = if compact { x + 22.0 } else { x + w - cw2 };
                                ui.chip(chip_x, cy + 8.0, t, fg, bg);
                            }
                        }
                        ui.rule(x, cy + 42.0, w, INNER);
                        cy += 44.0;
                    }
                }
                None => {
                    for fp in &i.participating_keys {
                        ui.text_mid(x, cy, 30.0, 14.0, W::M, TEXT, &fp_text(*fp));
                        cy += 30.0;
                    }
                }
            }
            cy += 8.0;
            let here = i
                .participating_keys
                .iter()
                .filter(|fp| app.session.keys.key_label(**fp).is_some())
                .count();
            // Short of signers: Scan a key, the vaults to unlock and
            // Collect a signature; a wallet whose paths differ keeps them
            // until the spend is signed (`docs/history/SCAN-TO-SIGN.md` §3.2).
            let paths = wl.is_some_and(|(_, wl)| {
                matches!(
                    crate::wallet::Kind::of(wl.policy()),
                    crate::wallet::Kind::Miniscript | crate::wallet::Kind::Tree
                )
            });
            if here < needed || (paths && !s.complete) {
                let style = if may {
                    Style::Secondary
                } else {
                    Style::Disabled
                };
                let unlocks: Vec<(String, Action)> = app
                    .vault_files()
                    .iter()
                    .enumerate()
                    .filter(|(_, f)| f.open.is_none())
                    .map(|(k, f)| {
                        (
                            fill(EN.step_unlock, &[&f.name.to_string()]),
                            Action::Vault(crate::vault::VaultAction::OpenFrom(k, Screen::Spend)),
                        )
                    })
                    .collect();
                let mut items: Vec<(&str, Style, Action)> = Vec::new();
                if let Some((wi, _)) = wl {
                    let scan = if may { Style::Primary } else { Style::Disabled };
                    items.push((EN.step_scan_a_key, scan, Action::ScanKey(wi)));
                }
                items.extend(unlocks.iter().map(|(l, a)| (l.as_str(), style, *a)));
                if s.steps.contains(&step::COLLECT) {
                    items.push((
                        EN.step_collect_a_signature,
                        Style::Secondary,
                        Action::Step(step::COLLECT),
                    ));
                }
                cy += wrap_buttons(ui, x, cy, w, 38.0, &items) + 2.0;
                if !may {
                    let pull = fill(EN.step_remove_the_first, &[app.medium.noun()]);
                    ui.text(x, cy, 13.0, W::S, WARN, &pull);
                    cy += 28.0;
                }
                cy += 6.0;
            }
            cy = signers_count(ui, x, cy, w, here, needed);
            next(ui, cy, EN.step_continue);
            cy += 44.0;
        }
        5 if threshold_of(s).is_some() && s.spend.signed_here.is_empty() => {
            let t = threshold_of(s).expect("threshold");
            let mine: Vec<u32> = t
                .ours
                .iter()
                .copied()
                .filter(|id| !t.signed.contains(id))
                .collect();
            let want = needed.saturating_sub(1);
            let (label, style, reason) = if mine.is_empty() {
                (
                    String::new(),
                    Style::Disabled,
                    Some(EN.step_no_share_of_this.to_string()),
                )
            } else if t.signers.is_empty() && s.others.len() != want {
                (
                    String::new(),
                    Style::Disabled,
                    Some(fill(
                        EN.step_choose_other_under,
                        &[
                            &want.to_string(),
                            (if want == 1 {
                                EN.step_share_inline
                            } else {
                                EN.step_shares
                            }),
                        ],
                    )),
                )
            } else {
                let names: Vec<String> = mine
                    .iter()
                    .map(|id| fill(EN.step_share_n_inline, &[&(id + 1).to_string()]))
                    .collect();
                (
                    fill(EN.step_sign_as, &[&names.join(", ").to_string()]),
                    Style::Primary,
                    None,
                )
            };
            if let Some(r) = reason {
                ui.text_mid(x, cy, 40.0, 13.0, W::R, MUTED, &r);
            } else {
                let bw = ui.measure(14.0, W::S, &label) + 36.0;
                ui.button(
                    x + w - bw,
                    cy,
                    Some(bw),
                    40.0,
                    &label,
                    style,
                    Action::SignHere,
                );
            }
            cy += 44.0;
        }
        5 => {
            let mine: Vec<String> = i
                .participating_keys
                .iter()
                .filter(|fp| {
                    app.session.keys.key_label(**fp).is_some() && !s.signers.contains(&fp.0)
                })
                .map(|fp| fp_text(*fp))
                .collect();
            if !s.spend.signed_here.is_empty() {
                let fps: Vec<String> = s.spend.signed_here.iter().map(|f| fp_text(*f)).collect();
                ui.icon(x, cy + 10.0, 20.0, Icon::Done, 12.0, OK);
                ui.text_mid(
                    x + 26.0,
                    cy,
                    40.0,
                    14.0,
                    W::S,
                    OK,
                    &fill(EN.step_signed_with, &[&fps.join(", ").to_string()]),
                );
                cy += 44.0;
            } else if mine.is_empty() {
                ui.text_mid(x, cy, 40.0, 13.0, W::R, MUTED, EN.step_no_key_in_this);
                cy += 44.0;
            } else {
                let label = fill(EN.step_sign_with, &[&mine.join(", ").to_string()]);
                ui.text_mid(
                    x,
                    cy,
                    40.0,
                    13.0,
                    W::R,
                    MUTED,
                    &format!(
                        "{} {}",
                        i.inputs.len(),
                        if i.inputs.len() == 1 {
                            EN.step_input_inline
                        } else {
                            EN.step_inputs
                        }
                    ),
                );
                let bw = ui.measure(14.0, W::S, &label) + 36.0;
                ui.button(
                    x + w - bw,
                    cy,
                    Some(bw),
                    40.0,
                    &label,
                    Style::Primary,
                    Action::SignHere,
                );
                cy += 44.0;
            }
        }
        6 => {
            let slots: Vec<(String, String, Option<[u8; 4]>)> =
                match wallet.and_then(|wi| app.session.wallets.get(wi)) {
                    Some(wl) => app
                        .session
                        .slots(wl)
                        .iter()
                        .map(|sl| {
                            (
                                sl.fingerprint.map(fp_text).unwrap_or_default(),
                                sl.held_by.clone().unwrap_or_default(),
                                sl.fingerprint.map(|f| f.0),
                            )
                        })
                        .collect(),
                    None => i
                        .participating_keys
                        .iter()
                        .map(|fp| (fp_text(*fp), String::new(), Some(fp.0)))
                        .collect(),
                };
            let complete = s.complete;
            for (fp, label, raw) in &slots {
                ui.text_mid(x, cy, 44.0, 14.0, W::M, TEXT, fp);
                let label = if label.is_empty() {
                    s.spend
                        .collected
                        .iter()
                        .find(|(f, _)| Some(f.0) == *raw)
                        .map(|(_, src)| fill(EN.step_from_inline, &[&src.to_string()]))
                        .unwrap_or_default()
                } else {
                    label.clone()
                };
                // On a small panel where it came from goes under the key.
                let (lx, ly) = if ui.compact && !label.is_empty() {
                    (x, cy + 26.0)
                } else if ui.compact {
                    (x, cy)
                } else {
                    (x + 102.0, cy)
                };
                let label = ui.fit(13.0, W::R, &label, x + w - lx - 90.0);
                ui.text_mid(lx, ly, 44.0, 13.0, W::R, MUTED, &label);
                let signed = raw.map(|r| s.signers.contains(&r)).unwrap_or(false);
                let here = raw
                    .map(|r| s.spend.signed_here.iter().any(|f| f.0 == r))
                    .unwrap_or(false);
                let (t, c) = if signed && here {
                    (EN.step_signed_here, OK)
                } else if signed {
                    (EN.step_verified, OK)
                } else if complete {
                    (EN.step_not_needed, DIM)
                } else {
                    (EN.step_waiting, DIM)
                };
                ui.text_right(x + w, cy, 44.0, 12.0, W::R, c, t);
                ui.rule(x, ly + 44.0, w, INNER);
                cy = ly + 46.0;
            }
            cy += 10.0;
            // Cosigners' copies in the Inbox.
            let txid = s.spend.txid;
            let copies: Vec<(usize, String)> = app
                .inbox
                .iter()
                .enumerate()
                .filter(|(_, it)| it.name != s.spend.source)
                .filter(|(_, it)| it.psbt().is_some_and(|p| p.txid == txid))
                .map(|(k, it)| (k, it.name.clone()))
                .collect();
            if !complete {
                if copies.is_empty() {
                    ui.text_mid(x, cy, 30.0, 13.0, W::R, MUTED, EN.step_no_cosigner_s);
                    cy += 34.0;
                }
                for (k, name) in &copies {
                    // No wider than the column; a long name is cut.
                    let label = fill(EN.step_add_from, &[&name.to_string()]);
                    let bw = (ui.measure(14.0, W::S, &label) + 32.0).min(w);
                    let label = ui.fit(14.0, W::S, &label, bw - 24.0);
                    ui.button(
                        x,
                        cy,
                        Some(bw),
                        38.0,
                        &label,
                        Style::Secondary,
                        Action::Collect(*k),
                    );
                    cy += 46.0;
                }
            }
            if !complete {
                let part = format!("{}-part.psbt", crate::result_stem(&s.spend.source));
                cy += made_line(app, ui, x, cy, w, &part);
                cy += wrap_buttons(
                    ui,
                    x,
                    cy,
                    w,
                    38.0,
                    &[
                        (EN.step_show_as_qr, Style::Secondary, Action::QrPart),
                        (EN.step_scan_a_copy, Style::Secondary, Action::Scan),
                    ],
                );
            } else {
                next(ui, cy, EN.step_continue);
                cy += 44.0;
            }
        }
        7 if s.spend.finished.is_none()
            && (s.carry_out.is_some() || s.out_signed)
            && threshold_of(s).is_some() =>
        {
            let t = threshold_of(s).expect("threshold");
            let next_ids: Vec<String> = t
                .signers
                .iter()
                .filter(|id| !t.signed.contains(id))
                .map(|id| fill(EN.step_share_n_inline, &[&(id + 1).to_string()]))
                .collect();
            ui.fill(x, cy, w, 150.0, 10.0, BG);
            ui.stroke(x, cy, w, 150.0, 10.0, INNER);
            ui.text(x + 14.0, cy + 14.0, 14.0, W::S, TEXT, EN.step_carry_file);
            let sub = fill(
                EN.step_for_holds_its,
                &[&(if next_ids.is_empty() {
                    EN.step_the_next_share.to_string()
                } else {
                    next_ids.join(", ")
                })
                .to_string()],
            );
            let sub = ui.fit(12.0, W::R, &sub, w - 28.0);
            ui.text(x + 14.0, cy + 38.0, 12.0, W::R, MUTED, &sub);
            let name = format!("{}-partly-signed.osk", crate::result_stem(&s.spend.source));
            ui.text(x + 14.0, cy + 62.0, 12.0, W::M, TEXT, &name);
            let (label, style) = if s.out_signed {
                (EN.step_kept, Style::Disabled)
            } else {
                (EN.step_keep_it, Style::Secondary)
            };
            ui.button(
                x + 14.0,
                cy + 98.0,
                None,
                36.0,
                label,
                style,
                Action::CarryToOutbox,
            );
            if s.out_signed {
                ui.icon(x + w - 34.0, cy + 106.0, 20.0, Icon::Done, 12.0, OK);
            }
            cy += 162.0;
        }
        7 => {
            if s.spend.finished.is_none() {
                let missing = needed.saturating_sub(s.signers.len()).max(1);
                let line = match s.spend.finish_reason() {
                    Some(r) if s.signers.len() >= needed => r,
                    _ => fill(
                        EN.step_needs,
                        &[
                            &missing.to_string(),
                            &(if s.signers.is_empty() {
                                String::new()
                            } else {
                                format!(" {}", EN.step_more)
                            }),
                            (if missing == 1 {
                                EN.step_signature
                            } else {
                                EN.step_signatures
                            }),
                        ],
                    ),
                };
                ui.wrap(x, cy, w, 13.0, W::R, WARN, &line);
                cy += 44.0;
            } else {
                // Side by side; on a small panel one under the other.
                let compact = ui.compact;
                let half = if compact { w } else { (w - 12.0) / 2.0 };
                let mut card_y = cy;
                let stem = crate::result_stem(&s.spend.source);
                for (k, (head, sub, file, action)) in [
                    (
                        EN.step_signed_psbt,
                        EN.step_for_the_wallet,
                        format!("{stem}-signed.psbt"),
                        Action::SignedToOutbox,
                    ),
                    (
                        EN.step_finished_transaction,
                        EN.step_ready_to_broadcast,
                        format!("{stem}-final.txn"),
                        Action::TxToOutbox,
                    ),
                ]
                .iter()
                .enumerate()
                {
                    let (bx, cy) = if compact {
                        (x, card_y)
                    } else {
                        (x + k as f32 * (half + 12.0), cy)
                    };
                    // Made For the stick at Finish: Remove while it waits,
                    // made again only when asked.
                    let place = made_place(app, file);
                    let (label, style, action) = match &place {
                        Some((_, Some(i))) => {
                            (EN.step_remove, Style::Ghost, Action::OutboxRemove(*i))
                        }
                        _ => (EN.step_make_again, Style::Secondary, *action),
                    };
                    // Where the buttons go: in a row when they fit.
                    let lw = ui.measure(13.0, W::S, label) + 32.0;
                    let qw = ui.measure(13.0, W::S, EN.step_show_as_qr) + 32.0;
                    let stacked = k == 0 && lw + 8.0 + qw > half - 28.0;
                    let by = if compact { cy + 92.0 } else { cy + 112.0 };
                    let card_h = if stacked {
                        by - cy + 36.0 + 8.0 + 36.0 + 16.0
                    } else if compact {
                        by - cy + 36.0 + 16.0
                    } else {
                        168.0
                    };
                    ui.fill(bx, cy, half, card_h, 10.0, BG);
                    ui.stroke(bx, cy, half, card_h, 10.0, INNER);
                    ui.text(bx + 14.0, cy + 14.0, 14.0, W::S, TEXT, head);
                    let sub = ui.fit(12.0, W::R, sub, half - 28.0);
                    ui.text(bx + 14.0, cy + 38.0, 12.0, W::R, MUTED, &sub);
                    let detail = if k == 0 {
                        format!("{}-signed.psbt", crate::result_stem(&s.spend.source))
                    } else {
                        let Some(tx) = s.spend.finished.as_ref() else {
                            continue;
                        };
                        fill(
                            EN.step_vb,
                            &[
                                &short(&tx.compute_txid().to_string()).to_string(),
                                &tx.vsize().to_string(),
                            ],
                        )
                    };
                    let detail = ui.fit(12.0, W::M, &detail, half - 28.0);
                    ui.text(bx + 14.0, cy + 66.0, 12.0, W::M, TEXT, &detail);
                    let bw = ui.button(bx + 14.0, by, Some(lw), 36.0, label, style, action);
                    if k == 0 {
                        let (qx, qy) = if stacked {
                            (bx + 14.0, by + 44.0)
                        } else {
                            (bx + 22.0 + bw, by)
                        };
                        ui.button(
                            qx,
                            qy,
                            Some(qw),
                            36.0,
                            EN.step_show_as_qr,
                            Style::Secondary,
                            Action::QrSigned,
                        );
                    }
                    if let Some((t, _)) = &place {
                        ui.text_right(bx + half - 14.0, cy + 14.0, 20.0, 12.0, W::S, OK, t);
                    }
                    card_y += card_h + 10.0;
                }
                cy = if compact { card_y + 2.0 } else { cy + 180.0 };
                cy += nonce_check(app, ui, &s.spend.psbt, x, cy, w);
                // The finished transaction itself: its bytes, and taken
                // apart.
                cy += wrap_buttons(
                    ui,
                    x,
                    cy,
                    w,
                    38.0,
                    &[
                        (EN.step_decode_it, Style::Secondary, Action::DecodeFinished),
                        (
                            if s.show_hex {
                                EN.step_hide_the_raw_hex
                            } else {
                                EN.step_show_the_raw_hex
                            },
                            Style::Secondary,
                            Action::SpendHex,
                        ),
                    ],
                ) + 4.0;
                if s.show_hex
                    && let Some(hex) = s.spend.finished_hex()
                {
                    let th = ui.wrap(
                        x + 12.0,
                        cy + 10.0,
                        w - 24.0,
                        12.0,
                        W::M,
                        TEXT,
                        &spaced(&hex),
                    );
                    ui.stroke(x, cy, w, th + 20.0, 8.0, INNER);
                    cy += th + 32.0;
                }
            }
        }
        9 => {
            if let Some(wl) = wallet.and_then(|wi| app.session.wallets.get(wi)) {
                let rows = crate::wallet::musig_nonces(&app.session, wl, &s.spend.psbt);
                for (slot, fp, here, has) in &rows {
                    ui.text_mid(x, cy, 40.0, 12.0, W::R, DIM, &slot.to_string());
                    ui.text_mid(x + 22.0, cy, 40.0, 14.0, W::M, TEXT, fp);
                    if *here {
                        // On a small panel it goes under the key.
                        let (hx, hy) = if ui.compact {
                            (x + 22.0, cy + 24.0)
                        } else {
                            (x + 124.0, cy)
                        };
                        ui.text_mid(hx, hy, 40.0, 13.0, W::R, MUTED, EN.step_this_device);
                    }
                    if *here && ui.compact {
                        cy += 24.0;
                    }
                    let (t, c) = if *has {
                        (EN.step_nonce_here, OK)
                    } else {
                        (EN.step_waiting, DIM)
                    };
                    ui.text_right(x + w, cy, 40.0, 12.0, W::R, c, t);
                    ui.rule(x, cy + 40.0, w, INNER);
                    cy += 42.0;
                }
                cy += 10.0;
                let txid = s.spend.txid;
                let copies: Vec<(usize, String)> = app
                    .inbox
                    .iter()
                    .enumerate()
                    .filter(|(_, it)| it.name != s.spend.source)
                    .filter(|(_, it)| it.psbt().is_some_and(|p| p.txid == txid))
                    .map(|(k, it)| (k, it.name.clone()))
                    .collect();
                let labels: Vec<String> = copies
                    .iter()
                    .map(|(_, name)| fill(EN.step_add_from, &[&name.to_string()]))
                    .collect();
                let items: Vec<(&str, Style, Action)> = labels
                    .iter()
                    .zip(&copies)
                    .map(|(l, (k, _))| (l.as_str(), Style::Secondary, Action::Collect(*k)))
                    .collect();
                if !items.is_empty() {
                    cy += wrap_buttons(ui, x, cy, w, 36.0, &items) + 2.0;
                }
                let part = format!("{}-part.psbt", crate::result_stem(&s.spend.source));
                cy += made_line(app, ui, x, cy, w, &part);
                let others_in = rows.iter().filter(|r| !r.2).all(|r| r.3);
                let mine_in = rows.iter().filter(|r| r.2).all(|r| r.3);
                cy += wrap_buttons(
                    ui,
                    x,
                    cy,
                    w,
                    38.0,
                    &[(EN.step_show_as_qr, Style::Secondary, Action::QrPart)],
                ) + 2.0;
                if !others_in && !mine_in {
                    ui.button(
                        x,
                        cy,
                        None,
                        38.0,
                        EN.step_go_first_share,
                        Style::Secondary,
                        Action::SignHere,
                    );
                    cy += 48.0;
                }
                next(ui, cy, EN.step_continue);
                cy += 48.0;
            }
        }
        8 => {
            if let Some(wl) = wallet.and_then(|wi| app.session.wallets.get(wi)) {
                for (k, p) in crate::wallet::paths(&app.session, wl, &s.spend.psbt)
                    .iter()
                    .enumerate()
                {
                    let top = cy;
                    ui.fill(x, cy, w, 0.0, 0.0, BG);
                    ui.text(
                        x + 14.0,
                        cy + 12.0,
                        13.0,
                        W::S,
                        TEXT,
                        &fill(EN.step_path, &[&(k + 1).to_string()]),
                    );
                    // The keys, wrapping; on a small panel under the name.
                    let (left, mut ky) = if ui.compact {
                        (x + 14.0, cy + 36.0)
                    } else {
                        (x + 90.0, cy + 8.0)
                    };
                    let mut kx = left;
                    for (slot, fp, here) in &p.keys {
                        let t = fill(EN.step_key, &[&slot.to_string(), &fp.to_string()]);
                        let (fg, bg) = if *here {
                            (OK, OK.with_alpha(30))
                        } else {
                            (MUTED, INNER)
                        };
                        let kw = ui.measure(12.0, W::R, &t) + 34.0;
                        if kx > left && kx + kw > x + w - 10.0 {
                            kx = left;
                            ky += 30.0;
                        }
                        kx += ui.chip(kx, ky, &t, fg, bg) + 6.0;
                    }
                    let mut ly = ky + 32.0;
                    if p.locks.is_empty() {
                        ui.text(x + 14.0, ly, 12.0, W::R, MUTED, EN.step_no_wait);
                        ly += 20.0;
                    }
                    for l in &p.locks {
                        ui.text(
                            x + 14.0,
                            ly,
                            12.0,
                            W::R,
                            MUTED,
                            &fill(EN.step_waits, &[&l.to_string()]),
                        );
                        ly += 20.0;
                    }
                    let (verdict, c) = if p.sequence_ok {
                        (EN.step_this_transaction_s, OK)
                    } else {
                        (EN.step_this_transaction, DIM)
                    };
                    ly += ui.wrap(x + 14.0, ly, w - 28.0, 12.0, W::R, c, verdict) + 12.0;
                    ui.stroke(x, top, w, ly - top, 10.0, INNER);
                    cy = ly + 8.0;
                }
            }
            next(ui, cy, EN.step_continue);
            cy += 48.0;
        }
        _ => {}
    }
    if let Some(e) = &s.error {
        let used = ui.wrap(x, cy, w, 13.0, W::R, ERR, e);
        cy += used + 6.0;
    }
    cy - y
}

/// One line of the review: its label, the amount, the rest of the line,
/// and a note with its colour.
type ReviewRow = (
    String,
    String,
    String,
    Option<(&'static str, faraday_draw::Color)>,
);

#[allow(clippy::too_many_arguments)]
#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
fn spend_panel(
    app: &View,
    ui: &mut Ui,
    px: f32,
    pw: f32,
    h: f32,
    wallet: Option<usize>,
    needed: usize,
) {
    let s = app.spend.as_ref().expect("spend");
    let i = &s.inspection;
    ui.fill(px, 0.0, pw, h, 0.0, SURFACE);
    ui.fill(px, 0.0, 1.0, h, 0.0, LINE);
    let x = px + 26.0;
    let w = pw - 52.0;
    let mut y = 32.0;
    ui.text(x, y, 13.0, W::S, MUTED, EN.spend_panel_this_transaction);
    y += 30.0;
    let to = recipients(i)
        .first()
        .map(|o| short(&app.session.shown(&o.address)))
        .unwrap_or_else(|| "·".into());
    let change = if i.outputs.iter().any(|o| o.is_ours()) {
        EN.spend_panel_verified.to_string()
    } else if i
        .outputs
        .iter()
        .any(|o| matches!(o.kind, OutputKind::UnverifiedChange { .. }))
    {
        EN.spend_panel_unverified.to_string()
    } else {
        EN.spend_panel_none.to_string()
    };
    let facts: [(&str, String, W, u8); 6] = [
        (
            EN.spend_panel_wallet,
            wallet
                .and_then(|wi| app.session.wallets.get(wi))
                .map(|wl| wl.name.clone())
                .unwrap_or_else(|| EN.spend_panel_not_loaded.into()),
            W::R,
            0,
        ),
        (
            EN.spend_panel_sends,
            fill(
                EN.spend_panel_btc,
                &[&btc(i.amount_to_others.to_sat()).to_string()],
            ),
            W::M,
            2,
        ),
        (EN.spend_panel_to, to, W::M, 2),
        (
            EN.spend_panel_fee,
            fill(
                EN.spend_panel_sat_sat,
                &[
                    &thousands(i.fee.to_sat()).to_string(),
                    &(i.fee_rate_sat_vb
                        .map(|r| format!("{r:.1}"))
                        .unwrap_or_else(|| "?".into()))
                    .to_string(),
                ],
            ),
            W::R,
            2,
        ),
        (EN.spend_panel_change, change, W::R, 2),
        (
            EN.spend_panel_txid,
            short(&s.spend.txid.to_string()),
            W::M,
            3,
        ),
    ];
    for (k, v, wt, step) in facts.iter() {
        ui.text_mid(x, y, 38.0, 12.0, W::R, MUTED, k);
        let v = ui.fit(13.0, *wt, v, w - 70.0);
        ui.text_right(x + w, y, 38.0, 13.0, *wt, TEXT, &v);
        ui.rule(x, y + 38.0, w, INNER);
        ui.hit(x, y, w, 38.0, Action::Step(*step));
        y += 40.0;
    }
    let (count, missing, need_line, action_label) = spend_status(app, needed);
    let fy = h - 26.0 - 46.0 - 14.0 - 82.0;
    ui.fill(x, fy, w, 82.0, 12.0, BG);
    ui.stroke(x, fy, w, 82.0, 12.0, INNER);
    ui.text_mid(
        x + 16.0,
        fy + 10.0,
        26.0,
        13.0,
        W::R,
        MUTED,
        EN.spend_panel_signatures,
    );
    let label = fill(
        EN.spend_panel_of,
        &[&count.to_string(), &needed.to_string()],
    );
    let lw = ui.measure(13.0, W::S, &label);
    ui.text_mid(x + w - 16.0 - lw, fy + 10.0, 26.0, 13.0, W::S, TEXT, &label);
    let pips_w = needed as f32 * 34.0 - 6.0;
    ui.pips(x + w - 26.0 - lw - pips_w, fy + 18.0, needed, count);
    ui.text_mid(
        x + 16.0,
        fy + 44.0,
        26.0,
        13.0,
        W::R,
        if missing == 0 { OK } else { WARN },
        &need_line,
    );
    let by = h - 26.0 - 46.0;
    let label = ui.fit(15.0, W::S, &action_label, w - 32.0);
    ui.button(
        x,
        by,
        Some(w),
        46.0,
        &label,
        Style::Primary,
        Action::Primary,
    );
}

/// Who signs' count (`docs/history/SCAN-TO-SIGN.md` §3.1): enough, the count and
/// a green Can spend beside it; short, the count and how many more are
/// needed, in `WARN`. Returns the y its step's Continue goes at: beside
/// it, or under it where both do not fit.
pub(crate) fn signers_count(
    ui: &mut Ui,
    x: f32,
    y: f32,
    w: f32,
    here: usize,
    needed: usize,
) -> f32 {
    let enough = here >= needed;
    let line = if enough {
        fill(
            EN.signers_count_of_signatures_here,
            &[&here.to_string(), &needed.to_string()],
        )
    } else {
        fill(
            EN.signers_count_of_signatures,
            &[
                &here.to_string(),
                &needed.to_string(),
                &(needed - here).to_string(),
            ],
        )
    };
    let line = ui.fit(13.0, W::S, &line, w);
    let mut cy = y;
    let mut used = ui.text_mid(
        x,
        cy,
        40.0,
        13.0,
        W::S,
        if enough { TEXT } else { WARN },
        &line,
    );
    if enough {
        let t = EN.signers_count_can_spend;
        let cw = ui.measure(12.0, W::R, t) + 34.0;
        if used + 10.0 + cw > w {
            cy += 36.0;
            ui.chip(x, cy + 7.0, t, OK, OK.with_alpha(30));
            used = cw;
        } else {
            ui.chip(x + used + 10.0, cy + 7.0, t, OK, OK.with_alpha(30));
            used += 10.0 + cw;
        }
    }
    let nw = ui.measure(14.0, W::S, EN.signers_count_continue) + 36.0;
    if !ui.pinning && used + nw + 12.0 > w {
        cy += 44.0;
    }
    cy
}

/// A file a flow made, as one line: its name, where it is, and Remove
/// while it waits For the stick. Nothing when it is neither waiting nor
/// written. Returns its height.
fn made_line(app: &View, ui: &mut Ui, x: f32, y: f32, w: f32, file: &str) -> f32 {
    let Some((state, k)) = made_place(app, file) else {
        return 0.0;
    };
    let mut right = x + w;
    if let Some(k) = k {
        let bw = ui.measure(13.0, W::S, EN.made_line_remove) + 32.0;
        right -= bw;
        ui.button(
            right,
            y,
            Some(bw),
            34.0,
            EN.made_line_remove,
            Style::Ghost,
            Action::OutboxRemove(k),
        );
        right -= 10.0;
    }
    let sw = ui.measure(13.0, W::S, &state);
    ui.text_mid(right - sw, y, 34.0, 13.0, W::S, OK, &state);
    let n = ui.fit(13.0, W::M, file, (right - sw - 10.0 - x).max(0.0));
    ui.text_mid(x, y, 34.0, 13.0, W::M, TEXT, &n);
    44.0
}
