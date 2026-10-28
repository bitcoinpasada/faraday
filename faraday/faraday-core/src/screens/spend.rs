//! `Screen::Spend`: signing a transaction.

use super::parts::{made_place, side_panel_w, spaced};
use crate::drawing::{Scrolled, View};
use crate::strings::{EN, fill};
use crate::ui::pal::*;
use crate::ui::{Style, Ui, btc, short, thousands};
use crate::wallet::{Session, fp_text, network_name, step, txid_known};
use crate::{Action, Screen, flow, guide};
use faraday_draw::tokens::{BODY_M, BODY_S, LABEL_L, LABEL_M, TITLE_M, TITLE_S};
use faraday_draw::tokens::{
    BUTTON_H, CHIP_LINE, GAP, GAP_SMALL, PAD, TABLE_AMOUNT_W, TABLE_LABEL_W, TypeRole,
};
use faraday_draw::widgets::Icon;
use faraday_psbt::{Level, OutputKind};
use faraday_ui::Status;
use faraday_ui::parts::wrap_buttons;

/// The nonce check: each signature against its key and this
/// transaction, each one made here recomputed from its key, and any nonce
/// used twice. Returns its height.
fn nonce_check(app: &View, ui: &mut Ui, psbt: &faraday_psbt::Psbt, x: f32, y: f32, w: f32) -> f32 {
    use faraday_psbt::verify::Verdict;
    let Some(check) = app.nonce_check(psbt) else {
        return 0.0;
    };
    let mut cy = y;
    let head = check.headline();
    // Sound: green. A signature wrong: red. Otherwise some were not
    // checked: amber, as the review's own card for one reads.
    let (tone, icon) = if check.sound() {
        (Role::Ok, Icon::Success)
    } else if check.wrong() {
        (Role::Error, Icon::Error)
    } else {
        (Role::Warning, Icon::Warning)
    };
    ui.icon(x, cy + 8.0, 20.0, icon, 12.0, tone);
    if ui.compact {
        // Wrapped beside its icon on a small panel.
        cy += ui
            .wrap_as((x + 26.0, cy + 8.0, w - 26.0), TITLE_S, tone, &head)
            .max(20.0)
            + 16.0;
    } else {
        ui.text_mid_as((x + 26.0, cy, 36.0), TITLE_S, tone, &head);
        cy += 40.0;
    }
    for reuse in &check.reused {
        let inputs: Vec<String> = reuse.inputs.iter().map(|i| i.to_string()).collect();
        let line = fill(
            EN.nonce_check_one_nonce,
            &[&reuse.name().to_string(), &inputs.join(", ").to_string()],
        );
        cy += ui.wrap_as((x, cy, w), LABEL_L, Role::Error, &line) + 8.0;
    }
    const SHOWN: usize = 12;
    for r in check.rows.iter().take(SHOWN) {
        ui.text_mid_as(
            (x, cy, 30.0),
            BODY_S,
            Role::OnSurfaceVariant,
            &fill(EN.nonce_check_input, &[&r.input.to_string()]),
        );
        ui.text_mid_as(
            (x + 70.0, cy, 30.0),
            BODY_M.mono(),
            Role::OnSurface,
            &r.name,
        );
        let (t, c) = match (r.verdict, r.recomputed) {
            (Verdict::Invalid, _) => (EN.nonce_check_invalid.to_string(), Role::Error),
            (Verdict::Unchecked(why), _) => (
                fill(EN.nonce_check_not_checked, &[unchecked_text(why)]),
                Role::OnSurfaceVariant,
            ),
            (Verdict::Valid, Some(Some(mode))) => (
                fill(EN.nonce_check_valid_nonce, &[nonce_mode_text(mode)]),
                Role::Ok,
            ),
            (Verdict::Valid, Some(None)) => (EN.nonce_check_wrong_nonce.to_string(), Role::Error),
            (Verdict::Valid, None) => (
                EN.nonce_check_valid_signed.to_string(),
                Role::OnSurfaceVariant,
            ),
        };
        if ui.compact {
            // The verdict in full, under the input.
            let th = ui.wrap_as((x, cy + 28.0, w), BODY_S, c, &t);
            ui.rule(x, cy + 34.0 + th, w, Role::OutlineVariant);
            cy += th + 36.0;
            continue;
        }
        let t = ui.fit_as(BODY_S, &t, w - 170.0);
        ui.text_right_as((x + w, cy, 30.0), BODY_S, c, &t);
        ui.rule(x, cy + 30.0, w, Role::OutlineVariant);
        cy += 32.0;
    }
    if check.rows.len() > SHOWN {
        let rest = &check.rows[SHOWN..];
        let ok = !rest.iter().any(crate::wallet::NonceRow::wrong);
        ui.text_mid_as(
            (x, cy, 30.0),
            BODY_S,
            if ok {
                Role::OnSurfaceVariant
            } else {
                Role::Error
            },
            &fill(
                EN.nonce_check_more,
                &[
                    &rest.len().to_string(),
                    (if ok {
                        EN.nonce_check_none_wrong
                    } else {
                        EN.nonce_check_one_wrong
                    }),
                ],
            ),
        );
        cy += 32.0;
    }
    cy + 12.0 - y
}

pub(crate) const STEPS: [&str; 10] = [
    EN.app_wallet,
    EN.app_check,
    EN.app_transaction,
    EN.app_transaction_id,
    EN.app_signers,
    EN.app_sign,
    EN.app_step_signatures,
    EN.app_finish,
    EN.app_path,
    EN.app_nonces,
];

pub(super) fn draw(app: &View, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    let panel_w = side_panel_w(cw);
    let col_x = x0 + 40.0;
    let col_w = cw - panel_w - 72.0;
    let needed = app.spend_needed();
    if app.wallets.spend().is_none() {
        // On the wide layout the top app bar names the page; on the
        // one-column layouts its own head does.
        let top = if ui.compact {
            faraday_ui::parts::title(ui, col_x, 28.0, EN.spend_sign_a_transaction);
            80.0
        } else {
            let back = app.named_back((EN.spend_wallets, Action::Nav(Screen::WalletsStart)));
            super::bar_head(ui, Some(back), EN.spend_sign_a_transaction)
        };
        ui.text_as(
            col_x,
            top,
            BODY_M,
            Role::OnSurfaceVariant,
            EN.spend_no_transaction,
        );
        ui.button(
            col_x,
            top + if ui.compact {
                36.0
            } else {
                ui.line_as(BODY_M) + faraday_draw::tokens::PAD
            },
            None,
            BUTTON_H,
            EN.spend_open_files,
            Style::Secondary,
            Action::Nav(Screen::Files),
        );
        return;
    }
    let (wallet_idx, steps) = {
        let s = app.wallets.spend().expect("spend");
        (s.wallet, s.steps.clone())
    };

    let (cards, scroll, guided) = {
        let s = app.wallets.spend().expect("spend");
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
                guide: Some(guide::spend(n, kind, needed, s, app.medium)).filter(|g| !g.is_empty()),
                learn: guide::spend_topic(n, kind, s),
            })
            .collect();
        (
            cards,
            flow::Scroll {
                y: s.scroll,
                follow: s.follow,
            },
            app.guided(),
        )
    };
    let compact = ui.compact;
    let col = flow::Column {
        area_x: x0,
        area_w: if compact { cw } else { cw - panel_w },
        x: col_x,
        w: col_w,
        h: if compact { h - spend_bar_h(ui) } else { h },
        back: Some(app.named_back((EN.spend_wallets, Action::Nav(Screen::WalletsStart)))),
        heading: EN.spend_sign_a_transaction,
        guided,
        switch: true,
        note: None,
        chip: None,
        chip_tap: None,
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
        let bar_y = h - spend_bar_h(ui);
        spend_bar(app, ui, x0, cw, bar_y, needed, step);
    } else {
        spend_panel(app, ui, x0 + cw - panel_w, panel_w, h, wallet_idx, needed);
    }
}

/// Where a signing stands: the signatures there, how many are missing,
/// the line that says what is left, the main button's label and its
/// style: off while a check on Transaction stops signing here (F136).
fn spend_status(app: &View, needed: usize) -> (usize, usize, String, ActionLabel, Style) {
    let s = app.wallets.spend().expect("spend");
    let i = &s.inspection;
    let signed_count = s.signers.len();
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
    let can_sign_here = i
        .participating_keys
        .iter()
        .filter(|fp| app.session.keys.key_label(**fp).is_some() && !s.signers.contains(&fp.0))
        .map(|fp| fp_text(*fp))
        .collect::<Vec<_>>();
    let stopped = s.spend.finished.is_none()
        && missing > 0
        && !can_sign_here.is_empty()
        && crate::review::review_stop(i, &s.held).is_some();
    let need_line = if stopped {
        fill(EN.spend_status_stopped, &[EN.app_transaction])
    } else if s.spend.finished.is_some() && s.out_signed && s.out_tx {
        fill(EN.spend_status_both_wait, &[app.medium.for_the()])
    } else if missing == 0 {
        EN.spend_status_complete.to_string()
    } else if !can_sign_here.is_empty() && missing > 1 {
        fill(
            EN.spend_status_sign_and_collect,
            &[&(missing - 1).to_string()],
        )
    } else if !can_sign_here.is_empty() {
        EN.spend_status_sign_here.to_string()
    } else {
        crate::wallet::collect_line(missing, count)
    };
    let label = if s.spend.finished.is_some() {
        // Both went For the stick at Finish: the next step is writing.
        if app.stick.is_empty() {
            fill(EN.spend_status_insert, &[app.medium.a()])
        } else {
            EN.spend_status_open_files.to_string()
        }
    } else if missing == 0 {
        EN.spend_status_finish.to_string()
    } else if !can_sign_here.is_empty() {
        fill(
            EN.spend_status_sign_with_keys,
            &[&can_sign_here.join(", ").to_string()],
        )
    } else {
        crate::wallet::collect_line(missing, count)
    };
    // Two keys or more and Finish not reached: counted where named they
    // would not fit the button.
    let short = (s.spend.finished.is_none() && missing > 0 && can_sign_here.len() > 1).then(|| {
        fill(
            EN.spend_status_sign_with_count,
            &[&can_sign_here.len().to_string()],
        )
    });
    let label = ActionLabel { long: label, short };
    let style = if stopped {
        Style::PrimaryOff
    } else {
        Style::Primary
    };
    (count, missing, need_line, label, style)
}

/// The main button's label: the keys that sign here named, and where
/// two or more sign, the same counted, for a button too narrow for
/// their names.
struct ActionLabel {
    long: String,
    short: Option<String>,
}

impl ActionLabel {
    /// The label as it fits `room` in `role`: the keys named where they
    /// fit, else counted; whatever is left too wide, cut.
    fn fitted(&self, ui: &Ui, role: TypeRole, room: f32) -> String {
        let label = match &self.short {
            Some(short) if ui.measure_as(role, &self.long) > room => short,
            _ => &self.long,
        };
        ui.fit_as(role, label, room)
    }
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
    let (count, missing, need_line, label, style) = spend_status(app, needed);
    let (x, w) = (x0 + crate::compact::M, cw - 2.0 * crate::compact::M);
    // The button's room for its label.
    let room = w - 24.0;
    let label = label.fitted(ui, TITLE_M, room);
    let (label, style, action) = step.unwrap_or((label, style, Action::Primary));
    ui.fill(x0, y, cw, spend_bar_h(ui), 0.0, Role::SurfaceContainerLow);
    ui.fill(x0, y, cw, 1.0, 0.0, Role::OutlineVariant);
    // Its lines and its button as tall as their text (`docs/TEXT-SIZE.md`
    // §5).
    let (g1, g2) = (ui.line_grown_as(LABEL_L), ui.line_grown_as(BODY_S));
    let pw = ui.pips(x, y + 14.0 + g1 / 2.0, needed, count);
    ui.text_mid_as(
        (x + pw + 10.0, y + 4.0, 26.0 + g1),
        LABEL_L,
        Role::OnSurface,
        &fill(
            EN.spend_bar_count,
            &[&count.to_string(), &needed.to_string()],
        ),
    );
    let line = ui.fit_as(BODY_S, &need_line, w);
    ui.text_as(
        x,
        y + 30.0 + g1,
        BODY_S,
        if missing == 0 {
            Role::Ok
        } else {
            Role::Warning
        },
        &line,
    );
    let label = ui.fit_as(TITLE_M, &label, room);
    ui.button(
        x,
        y + 52.0 + g1 + g2,
        Some(w),
        BUTTON_H,
        &label,
        style,
        action,
    );
}

/// The height of that foot: 104 at Standard, and what its two lines and
/// its button grow at a larger text size. Arithmetic.
fn spend_bar_h(ui: &Ui) -> f32 {
    104.0
        + ui.line_grown_as(LABEL_L)
        + ui.line_grown_as(BODY_S)
        + (ui.button_height(BUTTON_H) - BUTTON_H)
}

/// The outputs that pay away from the wallet: every output but verified
/// change. One that claims to be change and cannot be verified is a
/// payment like any other (F136).
fn recipients(inspection: &faraday_psbt::Inspection) -> Vec<&faraday_psbt::OutputInfo> {
    inspection.outputs.iter().filter(|o| !o.is_ours()).collect()
}

pub(crate) fn step_summary(app: &View, n: u8, wallet: Option<usize>, needed: usize) -> String {
    let s = app.wallets.spend().expect("spend");
    let i = &s.inspection;
    let reached = s.done[n as usize] || s.open == Some(n);
    match n {
        0 => match wallet.and_then(|w| app.session.wallets.get(w)) {
            Some(w) => format!("{} · {}", w.name, Session::shape(w)),
            None => EN.step_summary_no_loaded.to_string(),
        },
        1 if reached => EN.step_summary_addresses.to_string(),
        2 if reached => match i.fee_rate_sat_vb {
            Some(rate) => fill(
                EN.step_summary_sends,
                &[&btc(i.amount_to_others.to_sat()), &format!("{rate:.1}")],
            ),
            None => fill(
                EN.step_summary_sends_fee_not_known,
                &[&btc(i.amount_to_others.to_sat())],
            ),
        },
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
                fill(EN.step_summary_signed_keys, &[&fps.join(", ").to_string()])
            }
        }
        6 => fill(
            EN.step_summary_count,
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
                    EN.step_summary_way_one
                } else {
                    EN.step_summary_ways_many
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
    let s = app.wallets.spend().expect("spend");
    let i = &s.inspection;
    let mut cy = y;
    // A step's Continue: on a small panel it takes the foot's button
    // ([`spend_bar`]) while the step is read.
    let next = |ui: &mut Ui, cy: f32, label: &str| {
        if ui.pinning {
            ui.pin = Some((label.to_string(), Style::Primary, Action::StepNext(n)));
            return;
        }
        let bw = ui.measure_as(TITLE_S, label) + 36.0;
        ui.button(
            x + w - bw,
            cy,
            Some(bw),
            BUTTON_H,
            label,
            Style::Primary,
            Action::StepNext(n),
        );
    };
    match n {
        0 => {
            let mut row = BUTTON_H + GAP_SMALL;
            match wallet.and_then(|wi| app.session.wallets.get(wi)) {
                Some(wl) => {
                    // The name and its source wrap beside Continue, or
                    // across the column where Continue is at the foot.
                    let room = if ui.pinning {
                        w
                    } else {
                        w - ui.measure_as(TITLE_S, EN.step_continue) - 2.0 * PAD - GAP
                    };
                    let name = ui.wrap_as((x, cy, room), TITLE_S, Role::OnSurface, &wl.name);
                    let source = ui.wrap_as(
                        (x, cy + name + GAP_SMALL, room),
                        BODY_S,
                        Role::OnSurfaceVariant,
                        &fill(
                            EN.step_wallet_source,
                            &[&Session::shape(wl).to_string(), &wl.source.to_string()],
                        ),
                    );
                    row = row.max(name + GAP_SMALL + source + GAP);
                }
                None => {
                    ui.wrap_as(
                        (x, cy, w - 140.0),
                        BODY_M,
                        Role::Warning,
                        EN.step_no_loaded_wallet,
                    );
                }
            }
            next(ui, cy, EN.step_continue);
            cy += row;
        }
        1 => {
            match wallet.and_then(|wi| app.session.wallets.get(wi)) {
                Some(wl) => {
                    // Where an address starts beside its label.
                    let beside = 96.0;
                    for (label, change, idx) in [
                        (EN.step_receive_0_0, false, 0),
                        (EN.step_receive_0_1, false, 1),
                        (EN.step_change_1_0, true, 0),
                    ] {
                        ui.text_as(x, cy + 2.0, BODY_S, Role::OnSurfaceVariant, label);
                        let (a, held) = app.receive_line(wl, change, idx);
                        // On a small panel, or where the label leaves it
                        // no room beside, the address goes under its label.
                        let ax = if ui.compact || ui.measure_as(BODY_S, label) + GAP_SMALL > beside
                        {
                            cy += ui.advance_as(BODY_S, 20.0);
                            x
                        } else {
                            x + beside
                        };
                        let col = if held { Role::Warning } else { Role::OnSurface };
                        let used = ui.wrap_as((ax, cy, x + w - ax), BODY_M.mono(), col, &a);
                        cy += used.max(20.0) + 8.0;
                        ui.rule(x, cy - 4.0, w, Role::OutlineVariant);
                    }
                }
                None => {
                    ui.text_as(
                        x,
                        cy,
                        BODY_S,
                        Role::OnSurfaceVariant,
                        EN.step_no_wallet_to_show,
                    );
                    cy += ui.line_as(BODY_S) + faraday_draw::tokens::GRID;
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
                    fill(EN.step_to_address, &[&short(&o.address).to_string()]),
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
                                EN.step_to_path,
                                &[
                                    &(if change { 1 } else { 0 }).to_string(),
                                    &index.to_string(),
                                ],
                            ),
                            Some((EN.step_verified, Role::Ok.paint())),
                        ));
                    }
                    OutputKind::UnverifiedChange { .. } | OutputKind::Recipient => {}
                }
            }
            // A fee that cannot be worked out is not known, never zero.
            rows.push(match i.fee_rate_sat_vb {
                Some(rate) => (
                    EN.step_fee.into(),
                    btc(i.fee.to_sat()),
                    fill(
                        EN.step_sat_sat_vb,
                        &[
                            &thousands(i.fee.to_sat()).to_string(),
                            &format!("{rate:.1}"),
                        ],
                    ),
                    None,
                ),
                None => (
                    EN.step_fee.into(),
                    EN.step_fee_not_known.to_string(),
                    String::new(),
                    None,
                ),
            });
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
                    Some((EN.step_amount_unchecked, Role::Warning.paint()))
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
            ui.fill(
                x,
                rows_top,
                w,
                box_h,
                faraday_draw::tokens::SHAPE_M,
                Role::SurfaceContainerHighest,
            );
            ui.stroke(
                x,
                rows_top,
                w,
                box_h,
                faraday_draw::tokens::SHAPE_M,
                Role::OutlineVariant,
            );
            for (k, (label, amount, rest, note)) in rows.iter().enumerate() {
                let ry = rows_top + k as f32 * row_h;
                if compact {
                    ui.text_mid_as(
                        (x + 12.0, ry + 4.0, 24.0),
                        BODY_S,
                        Role::OnSurfaceVariant,
                        label,
                    );
                    ui.text_right_as(
                        (x + w - 12.0, ry + 4.0, 24.0),
                        BODY_M.mono(),
                        Role::OnSurface,
                        amount,
                    );
                    let nw = note.map_or(0.0, |(t, _)| ui.measure_as(BODY_S, t) + 8.0);
                    let rest = ui.fit_as(BODY_S, rest, w - 24.0 - nw);
                    ui.text_mid_as(
                        (x + 12.0, ry + 26.0, 22.0),
                        BODY_S,
                        Role::OnSurfaceVariant,
                        &rest,
                    );
                    if let Some((t, c)) = note {
                        ui.text_right_as((x + w - 12.0, ry + 26.0, 22.0), BODY_S, *c, t);
                    }
                } else {
                    ui.text_mid_as((x + 14.0, ry, row_h), BODY_S, Role::OnSurfaceVariant, label);
                    let aw = ui.text_mid_as(
                        (x + 84.0, ry, row_h),
                        BODY_M.mono(),
                        Role::OnSurface,
                        amount,
                    );
                    ui.text_mid_as(
                        (x + 98.0 + aw, ry, row_h),
                        BODY_S,
                        Role::OnSurfaceVariant,
                        rest,
                    );
                    if let Some((t, c)) = note {
                        ui.text_right_as((x + w - 14.0, ry, row_h), BODY_S, *c, t);
                    }
                }
                if k + 1 < rows.len() {
                    ui.rule(x, ry + row_h, w, Role::Surface);
                }
            }
            cy += box_h + 12.0;
            // Warnings, every one as the inspector words it, but an
            // input left unsigned for want of its wallet (F141).
            let mut warned = false;
            // A key not of the wallet spends it alone (F132): the wallet's
            // danger line, repeated here as a caution, first.
            let caution = wallet
                .and_then(|wi| app.session.wallets.get(wi))
                .and_then(|wl| app.session.key_path_line(wl.policy()))
                .filter(|k| k.danger)
                .map(|k| k.text);
            let lines = caution
                .iter()
                .map(|t| (Role::Warning, std::borrow::Cow::Borrowed(t.as_str())))
                .chain(i.warnings.iter().map(|wn| {
                    let c = match wn.level {
                        Level::Info => Role::OnSurfaceVariant,
                        Level::Caution => Role::Warning,
                        Level::Danger | Level::Blocked => Role::Error,
                    };
                    (c, crate::review::warning_text(i, wn))
                }));
            for (c, text) in lines {
                let used = ui.wrap_as((x + 22.0, cy, w - 22.0), BODY_M, c, &text);
                ui.icon(x, cy, 16.0, Icon::Warning, 10.0, c);
                cy += used.max(18.0) + 6.0;
                warned = true;
            }
            // Held for want of a registered wallet: register the one the
            // PSBT defines, or, where it defines none, load its
            // descriptor (`docs/FLOWS.md`, Spend).
            if i.warnings
                .iter()
                .any(|wn| wn.kind == faraday_psbt::WarningKind::WalletNotRegistered)
            {
                if i.register.is_some() {
                    let bw = ui.measure_as(LABEL_L, EN.register_head) + 2.0 * PAD;
                    ui.button(
                        x,
                        cy,
                        Some(bw.min(w)),
                        BUTTON_H,
                        EN.register_head,
                        Style::Tonal,
                        Action::RegisterOpen,
                    );
                    cy += BUTTON_H + GAP;
                } else {
                    cy += ui.wrap_as(
                        (x, cy, w),
                        BODY_M,
                        Role::OnSurfaceVariant,
                        EN.register_load_descriptor,
                    ) + GAP;
                }
            }
            if !warned {
                let mut chx = x;
                chx += ui.tag(chx, cy, EN.step_no_warnings, Status::Ok) + 6.0;
                let net = app.session.network();
                let status = if net.is_mainnet() {
                    Status::Ok
                } else {
                    Status::Warning
                };
                ui.tag(chx, cy, network_name(net), status);
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
                    .chain(
                        i.outputs
                            .iter()
                            .map(|o| (EN.step_output, btc(o.amount.to_sat()), o.address.clone())),
                    )
                    .collect::<Vec<_>>();
                for (label, amount, what) in &lines {
                    ui.text_as(x, cy, BODY_S, Role::OnSurfaceVariant, label);
                    ui.text_right_as(
                        (x + w, cy - 4.0, 20.0),
                        BODY_S.mono(),
                        Role::OnSurface,
                        amount,
                    );
                    cy += ui.advance_as(BODY_S, 18.0);
                    cy += ui.wrap_as((x, cy, w), BODY_S.mono(), Role::OnSurface, what) + 10.0;
                }
                cy += 6.0;
            } else if s.table {
                // As wide as its columns (`docs/LAYOUT.md` §17): the name,
                // the longest outpoint or address, the amount; no wider
                // than the step.
                let outpoints: Vec<String> = i
                    .inputs
                    .iter()
                    .map(|inp| format!("{}:{}", short(&inp.txid.to_string()), inp.vout))
                    .collect();
                let what = outpoints
                    .iter()
                    .map(String::as_str)
                    .chain(i.outputs.iter().map(|o| o.address.as_str()))
                    .map(|t| ui.measure_as(BODY_S.mono(), t))
                    .fold(0.0, f32::max);
                let tw = (TABLE_LABEL_W + what + GAP_SMALL + TABLE_AMOUNT_W).min(w);
                for (inp, at) in i.inputs.iter().zip(&outpoints) {
                    let a = inp
                        .amount
                        .map(|a| btc(a.to_sat()))
                        .unwrap_or_else(|| "?".into());
                    ui.text_as(x, cy, BODY_S, Role::OnSurfaceVariant, EN.step_input);
                    ui.text_as(x + TABLE_LABEL_W, cy, BODY_S.mono(), Role::OnSurface, at);
                    ui.text_right_as((x + tw, cy - 4.0, 20.0), BODY_S.mono(), Role::OnSurface, &a);
                    cy += ui.advance_as(BODY_S, 22.0);
                }
                for o in &i.outputs {
                    ui.text_as(x, cy, BODY_S, Role::OnSurfaceVariant, EN.step_output);
                    let used = ui.wrap_as(
                        (x + TABLE_LABEL_W, cy, tw - TABLE_LABEL_W - TABLE_AMOUNT_W),
                        BODY_S.mono(),
                        Role::OnSurface,
                        &o.address,
                    );
                    ui.text_right_as(
                        (x + tw, cy - 4.0, 20.0),
                        BODY_S.mono(),
                        Role::OnSurface,
                        &btc(o.amount.to_sat()),
                    );
                    cy += used.max(18.0) + 4.0;
                }
                cy += 6.0;
            }
            // A transaction nothing signs stops here, and a holdable
            // danger is held past here (F136).
            let stop = crate::review::review_stop(i, &s.held);
            if let Some(why) = stop {
                cy += review_stop_block(app, ui, (x, cy, w), i, why);
            }
            ui.button(
                x,
                cy,
                None,
                BUTTON_H,
                if s.table {
                    EN.step_hide_the_table
                } else {
                    EN.step_show_the_table
                },
                Style::Secondary,
                Action::ToggleTable,
            );
            if stop.is_none() {
                next(ui, cy, EN.step_continue);
            }
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
                        ui.text_mid_as(
                            (x, cy, 42.0),
                            BODY_S,
                            Role::OnSurfaceVariant,
                            &(k + 1).to_string(),
                        );
                        let fp = slot.fingerprint.map(fp_text).unwrap_or_default();
                        ui.text_mid_as((x + 22.0, cy, 42.0), BODY_M.mono(), Role::OnSurface, &fp);
                        // A label that is its fingerprint is not drawn
                        // twice (`docs/history/SCAN-TO-SIGN.md` §3.3).
                        let label = slot
                            .held_by
                            .as_deref()
                            .map(str::trim)
                            .filter(|l| !l.is_empty() && !l.eq_ignore_ascii_case(&fp))
                            .unwrap_or_default();
                        let label = ui.fit_as(BODY_M, label, w - 124.0);
                        ui.text_mid_as(
                            (x + 124.0, cy, 42.0),
                            BODY_S,
                            Role::OnSurfaceVariant,
                            &label,
                        );
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
                                let bw = ui.measure_as(LABEL_L, l) + 28.0;
                                ui.button(
                                    x + w - bw,
                                    cy + 5.0,
                                    Some(bw),
                                    BUTTON_H,
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
                                let bw = ui.measure_as(LABEL_L, l) + 28.0;
                                ui.button(
                                    x + w - bw,
                                    cy + 5.0,
                                    Some(bw),
                                    BUTTON_H,
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
                                let (t, status) = if slot.held_by.is_some() {
                                    (EN.step_signs_here, Status::Ok)
                                } else {
                                    (EN.step_not_here, Status::Neutral)
                                };
                                let cw2 = ui.measure_as(BODY_S, t) + 34.0;
                                let chip_x = if compact { x + 22.0 } else { x + w - cw2 };
                                ui.tag(chip_x, cy + 8.0, t, status);
                            }
                        }
                        ui.rule(x, cy + 42.0, w, Role::OutlineVariant);
                        cy += 44.0;
                    }
                }
                None => {
                    for fp in &i.participating_keys {
                        ui.text_mid_as(
                            (x, cy, 30.0),
                            BODY_M.mono(),
                            Role::OnSurface,
                            &fp_text(*fp),
                        );
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
                    let scan = Style::Primary.when(may);
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
                cy += wrap_buttons(ui, x, cy, w, BUTTON_H, &items) + 2.0;
                if !may {
                    let pull = fill(EN.step_remove_the_first, &[app.medium.noun()]);
                    ui.text_as(x, cy, LABEL_L, Role::Warning, &pull);
                    cy += ui.line_as(LABEL_L) + faraday_draw::tokens::GRID;
                }
                cy += 6.0;
            }
            cy = signers_count(ui, x, cy, w, here, needed);
            next(ui, cy, EN.step_continue);
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
                let text = fill(EN.step_signed_keys, &[&fps.join(", ").to_string()]);
                ui.icon(x, cy + 10.0, 20.0, Icon::Done, 12.0, Role::Ok);
                let tx = x + 26.0;
                if ui.measure_as(TITLE_S, &text) <= x + w - tx {
                    ui.text_mid_as((tx, cy, BUTTON_H), TITLE_S, Role::Ok, &text);
                    cy += BUTTON_H + GAP_SMALL;
                } else {
                    // The keys wrap under one another on a narrow panel.
                    cy += ui.wrap_as((tx, cy + GAP, x + w - tx), TITLE_S, Role::Ok, &text)
                        + 2.0 * GAP;
                }
            } else if mine.is_empty() {
                ui.text_mid_as(
                    (x, cy, 40.0),
                    BODY_S,
                    Role::OnSurfaceVariant,
                    EN.step_no_signing_key,
                );
                cy += 44.0;
            } else {
                let label = fill(EN.step_sign_with_keys, &[&mine.join(", ").to_string()]);
                let inputs = format!(
                    "{} {}",
                    i.inputs.len(),
                    if i.inputs.len() == 1 {
                        EN.step_input_inline
                    } else {
                        EN.step_inputs
                    }
                );
                cy += sign_row(ui, (x, cy, w), &inputs, &label, Style::Primary);
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
                ui.text_mid_as((x, cy, 44.0), BODY_M.mono(), Role::OnSurface, fp);
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
                let label = ui.fit_as(BODY_M, &label, x + w - lx - 90.0);
                ui.text_mid_as((lx, ly, 44.0), BODY_S, Role::OnSurfaceVariant, &label);
                let signed = raw.map(|r| s.signers.contains(&r)).unwrap_or(false);
                let here = raw
                    .map(|r| s.spend.signed_here.iter().any(|f| f.0 == r))
                    .unwrap_or(false);
                let (t, c) = if signed && here {
                    (EN.step_signed_here, Role::Ok)
                } else if signed {
                    (EN.step_verified, Role::Ok)
                } else if complete {
                    (EN.step_not_needed, Role::OnSurfaceVariant)
                } else {
                    (EN.step_waiting, Role::OnSurfaceVariant)
                };
                ui.text_right_as((x + w, cy, 44.0), BODY_S, c, t);
                ui.rule(x, ly + 44.0, w, Role::OutlineVariant);
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
                    ui.text_mid_as(
                        (x, cy, 30.0),
                        BODY_S,
                        Role::OnSurfaceVariant,
                        EN.step_no_cosigner_s,
                    );
                    cy += 34.0;
                }
                for (k, name) in &copies {
                    // No wider than the column; a long name is cut.
                    let label = fill(EN.step_add_source, &[&name.to_string()]);
                    let bw = (ui.measure_as(TITLE_S, &label) + 32.0).min(w);
                    let label = ui.fit_as(TITLE_S, &label, bw - 24.0);
                    ui.button(
                        x,
                        cy,
                        Some(bw),
                        BUTTON_H,
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
                    BUTTON_H,
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
                ui.wrap_as((x, cy, w), BODY_M, Role::Warning, &line);
                cy += 44.0;
            } else {
                // Side by side where each card's head, its place and its
                // buttons fit its half; on a small panel, or where the
                // text size leaves them no room, one under the other.
                let stem = crate::result_stem(&s.spend.source);
                use faraday_draw::tokens::{BUTTON_PAD, FINISH_CARD_GAP, FINISH_CARD_INSET, GAP};
                let side = (w - FINISH_CARD_GAP) / 2.0 - 2.0 * FINISH_CARD_INSET;
                let fits = |ui: &Ui, head: &str, file: String| {
                    let tag = made_place(app, &file).map_or(0.0, |(t, _)| {
                        ui.measure_as(LABEL_M, &t) + faraday_draw::tokens::GAP_SMALL
                    });
                    ui.measure_as(TITLE_S, head) + tag <= side
                };
                let buttons = ui.measure_as(LABEL_L, EN.step_make_again)
                    + ui.measure_as(LABEL_L, EN.step_show_as_qr)
                    + 2.0 * BUTTON_PAD
                    + 2.0 * BUTTON_PAD
                    + GAP;
                let compact = ui.compact
                    || buttons > side
                    || !fits(ui, EN.step_signed_psbt, format!("{stem}-signed.psbt"))
                    || !fits(
                        ui,
                        EN.step_finished_transaction,
                        format!("{stem}-final.txn"),
                    );
                let half = if compact { w } else { (w - 12.0) / 2.0 };
                let mut card_y = cy;
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
                    let lw = ui.measure_as(LABEL_L, label) + 32.0;
                    let qw = ui.measure_as(LABEL_L, EN.step_show_as_qr) + 32.0;
                    let stacked = k == 0 && lw + 8.0 + qw > half - 28.0;
                    let by = if compact { cy + 92.0 } else { cy + 112.0 };
                    let card_h = if stacked {
                        by - cy + 36.0 + 8.0 + 36.0 + 16.0
                    } else if compact {
                        by - cy + 36.0 + 16.0
                    } else {
                        168.0
                    };
                    ui.fill(
                        bx,
                        cy,
                        half,
                        card_h,
                        faraday_draw::tokens::SHAPE_M,
                        Role::SurfaceContainerHighest,
                    );
                    ui.stroke(
                        bx,
                        cy,
                        half,
                        card_h,
                        faraday_draw::tokens::SHAPE_M,
                        Role::OutlineVariant,
                    );
                    ui.text_as(bx + 14.0, cy + 14.0, TITLE_S, Role::OnSurface, head);
                    let sub = ui.fit_as(BODY_S, sub, half - 28.0);
                    ui.text_as(bx + 14.0, cy + 38.0, BODY_S, Role::OnSurfaceVariant, &sub);
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
                    let detail = ui.fit_as(BODY_S.mono(), &detail, half - 28.0);
                    ui.text_as(
                        bx + 14.0,
                        cy + 66.0,
                        BODY_S.mono(),
                        Role::OnSurface,
                        &detail,
                    );
                    let bw = ui.button(bx + 14.0, by, Some(lw), BUTTON_H, label, style, action);
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
                            BUTTON_H,
                            EN.step_show_as_qr,
                            Style::Secondary,
                            Action::QrSigned,
                        );
                    }
                    if let Some((t, _)) = &place {
                        ui.text_right_as((bx + half - 14.0, cy + 14.0, 20.0), LABEL_M, Role::Ok, t);
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
                    BUTTON_H,
                    &[
                        (
                            EN.step_decode_button,
                            Style::Secondary,
                            Action::DecodeFinished,
                        ),
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
                    let th = ui.wrap_as(
                        (x + 12.0, cy + 10.0, w - 24.0),
                        BODY_S.mono(),
                        Role::OnSurface,
                        &spaced(&hex),
                    );
                    ui.stroke(x, cy, w, th + 20.0, 8.0, Role::OutlineVariant);
                    cy += th + 32.0;
                }
            }
        }
        9 => {
            if let Some(wl) = wallet.and_then(|wi| app.session.wallets.get(wi)) {
                let rows = crate::wallet::musig_nonces(&app.session, wl, &s.spend.psbt);
                for (slot, fp, here, has) in &rows {
                    ui.text_mid_as(
                        (x, cy, 40.0),
                        BODY_S,
                        Role::OnSurfaceVariant,
                        &slot.to_string(),
                    );
                    ui.text_mid_as((x + 22.0, cy, 40.0), BODY_M.mono(), Role::OnSurface, fp);
                    if *here {
                        // On a small panel it goes under the key.
                        let (hx, hy) = if ui.compact {
                            (x + 22.0, cy + 24.0)
                        } else {
                            (x + 124.0, cy)
                        };
                        ui.text_mid_as(
                            (hx, hy, 40.0),
                            BODY_S,
                            Role::OnSurfaceVariant,
                            EN.step_this_device,
                        );
                    }
                    if *here && ui.compact {
                        cy += 24.0;
                    }
                    let (t, c) = if *has {
                        (EN.step_nonce_here, Role::Ok)
                    } else {
                        (EN.step_waiting, Role::OnSurfaceVariant)
                    };
                    ui.text_right_as((x + w, cy, 40.0), BODY_S, c, t);
                    ui.rule(x, cy + 40.0, w, Role::OutlineVariant);
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
                    .map(|(_, name)| fill(EN.step_add_source, &[&name.to_string()]))
                    .collect();
                let items: Vec<(&str, Style, Action)> = labels
                    .iter()
                    .zip(&copies)
                    .map(|(l, (k, _))| (l.as_str(), Style::Secondary, Action::Collect(*k)))
                    .collect();
                if !items.is_empty() {
                    cy += wrap_buttons(ui, x, cy, w, BUTTON_H, &items) + 2.0;
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
                    BUTTON_H,
                    &[(EN.step_show_as_qr, Style::Secondary, Action::QrPart)],
                ) + 2.0;
                if !others_in && !mine_in {
                    ui.button(
                        x,
                        cy,
                        None,
                        BUTTON_H,
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
                    ui.fill(x, cy, w, 0.0, 0.0, Role::SurfaceContainerHighest);
                    ui.text_as(
                        x + 14.0,
                        cy + 12.0,
                        LABEL_L,
                        Role::OnSurface,
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
                        let status = if *here { Status::Ok } else { Status::Neutral };
                        let kw = ui.measure_as(BODY_S, &t) + 34.0;
                        if kx > left && kx + kw > x + w - 10.0 {
                            kx = left;
                            ky += 30.0;
                        }
                        kx += ui.tag(kx, ky, &t, status) + 6.0;
                    }
                    let mut ly = ky + 32.0;
                    if p.locks.is_empty() {
                        ui.text_as(
                            x + 14.0,
                            ly,
                            BODY_S,
                            Role::OnSurfaceVariant,
                            EN.step_no_wait,
                        );
                        ly += ui.line_as(BODY_S) + faraday_draw::tokens::GAP_SMALL;
                    }
                    for l in &p.locks {
                        ui.text_as(
                            x + 14.0,
                            ly,
                            BODY_S,
                            Role::OnSurfaceVariant,
                            &fill(EN.step_waits, &[&l.to_string()]),
                        );
                        ly += ui.line_as(BODY_S) + faraday_draw::tokens::GAP_SMALL;
                    }
                    let (verdict, c) = if p.sequence_ok {
                        (EN.step_this_transaction_s, Role::Ok)
                    } else {
                        (EN.step_this_transaction, Role::OnSurfaceVariant)
                    };
                    ly += ui.wrap_as((x + 14.0, ly, w - 28.0), BODY_S, c, verdict) + 12.0;
                    ui.stroke(
                        x,
                        top,
                        w,
                        ly - top,
                        faraday_draw::tokens::SHAPE_M,
                        Role::OutlineVariant,
                    );
                    cy = ly + 8.0;
                }
            }
            next(ui, cy, EN.step_continue);
            cy += 48.0;
        }
        _ => {}
    }
    if let Some(e) = &s.error {
        let used = ui.wrap_as((x, cy, w), BODY_M, Role::Error, e);
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
    Option<(&'static str, crate::ui::Paint)>,
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
    let s = app.wallets.spend().expect("spend");
    let i = &s.inspection;
    ui.fill(px, 0.0, pw, h, 0.0, Role::Surface);
    ui.fill(px, 0.0, 1.0, h, 0.0, Role::OutlineVariant);
    let x = px + 26.0;
    let w = pw - 52.0;
    let mut y = faraday_draw::tokens::SIDE_PANEL_TOP;
    ui.text_as(
        x,
        y,
        LABEL_L,
        Role::OnSurfaceVariant,
        EN.spend_panel_this_transaction,
    );
    y += ui.advance_as(LABEL_L, 30.0);
    let to = recipients(i)
        .first()
        .map(|o| short(&o.address))
        .unwrap_or_else(|| "·".into());
    // Change is verified change only: an output that claims to be change
    // and cannot be verified is among what it sends (F136).
    let change = if i.outputs.iter().any(|o| o.is_ours()) {
        EN.spend_panel_verified
    } else {
        EN.spend_panel_none
    }
    .to_string();
    let facts: [(&str, String, TypeRole, u8); 6] = [
        (
            EN.spend_panel_wallet,
            wallet
                .and_then(|wi| app.session.wallets.get(wi))
                .map(|wl| wl.name.clone())
                .unwrap_or_else(|| EN.spend_panel_not_loaded.into()),
            BODY_M,
            0,
        ),
        (
            EN.spend_panel_sends,
            fill(
                EN.spend_panel_btc,
                &[&btc(i.amount_to_others.to_sat()).to_string()],
            ),
            BODY_M.mono(),
            2,
        ),
        (EN.spend_panel_to_label, to, BODY_M.mono(), 2),
        (
            EN.spend_panel_fee,
            match i.fee_rate_sat_vb {
                Some(rate) => fill(
                    EN.spend_panel_sat_sat,
                    &[&thousands(i.fee.to_sat()), &format!("{rate:.1}")],
                ),
                None => EN.step_fee_not_known.to_string(),
            },
            BODY_M,
            2,
        ),
        (EN.spend_panel_change, change, BODY_M, 2),
        (
            EN.spend_panel_txid,
            short(&s.spend.txid.to_string()),
            BODY_M.mono(),
            3,
        ),
    ];
    for (k, v, wt, step) in facts.iter() {
        ui.text_mid_as((x, y, 38.0), BODY_S, Role::OnSurfaceVariant, k);
        let v = ui.fit_as(*wt, v, w - 70.0);
        ui.text_right_as((x + w, y, 38.0), *wt, Role::OnSurface, &v);
        ui.rule(x, y + 38.0, w, Role::OutlineVariant);
        ui.hit(x, y, w, 38.0, Action::Step(*step));
        y += 40.0;
    }
    let (count, missing, need_line, action_label, style) = spend_status(app, needed);
    let fy = h - 26.0 - 46.0 - 14.0 - 82.0;
    ui.fill(x, fy, w, 82.0, 12.0, Role::SurfaceContainerHighest);
    ui.stroke(x, fy, w, 82.0, 12.0, Role::OutlineVariant);
    ui.text_mid_as(
        (x + 16.0, fy + 10.0, 26.0),
        BODY_S,
        Role::OnSurfaceVariant,
        EN.spend_panel_signatures,
    );
    let label = fill(
        EN.spend_panel_count,
        &[&count.to_string(), &needed.to_string()],
    );
    let lw = ui.measure_as(LABEL_L, &label);
    ui.text_mid_as(
        (x + w - 16.0 - lw, fy + 10.0, 26.0),
        LABEL_L,
        Role::OnSurface,
        &label,
    );
    let pips_w = needed as f32 * 34.0 - 6.0;
    ui.pips(x + w - 26.0 - lw - pips_w, fy + 18.0, needed, count);
    ui.text_mid_as(
        (x + 16.0, fy + 44.0, 26.0),
        BODY_M,
        if missing == 0 {
            Role::Ok
        } else {
            Role::Warning
        },
        &need_line,
    );
    let by = h - 26.0 - 46.0;
    let label = action_label.fitted(ui, TITLE_M, w - 32.0);
    ui.button(x, by, Some(w), BUTTON_H, &label, style, Action::Primary);
}

/// Who signs' count (`docs/history/SCAN-TO-SIGN.md` §3.1): enough, the count and
/// a green Can spend beside it; short, the count and how many more are
/// needed, in `warning`. Returns the y its step's Continue goes at: beside
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
    let line = ui.fit_as(LABEL_L, &line, w);
    let mut cy = y;
    let mut used = ui.text_mid_as(
        (x, cy, 40.0),
        LABEL_L,
        if enough {
            Role::OnSurface
        } else {
            Role::Warning
        },
        &line,
    );
    if enough {
        let t = EN.signers_count_can_spend;
        let cw = ui.measure_as(BODY_S, t) + 34.0;
        if used + 10.0 + cw > w {
            cy += 36.0;
            ui.tag(x, cy + 7.0, t, Status::Ok);
            used = cw;
        } else {
            ui.tag(x + used + 10.0, cy + 7.0, t, Status::Ok);
            used += 10.0 + cw;
        }
    }
    let nw = ui.measure_as(TITLE_S, EN.signers_count_continue) + 36.0;
    if !ui.pinning && used + nw + 12.0 > w {
        cy += 44.0;
    }
    cy
}

/// Review's foot when it does not go on (F136), `why` as
/// `review::review_stop` gives it, in a column `w` wide at (`x`, `y`):
/// for a transaction nothing signs, what to do instead; for a fee a hold
/// accepts, the fee, its rate and its share of what is sent, the cases
/// such a fee is for, and the hold. Returns its height.
fn review_stop_block(
    app: &View,
    ui: &mut Ui,
    (x, y, w): (f32, f32, f32),
    i: &faraday_psbt::Inspection,
    why: &str,
) -> f32 {
    let mut cy = y;
    if why != EN.step_review_hold_first {
        cy += ui.wrap_as((x, cy, w), BODY_M, Role::Error, why) + GAP;
        return cy - y;
    }
    let facts = fill(
        EN.step_fee_hold_facts,
        &[
            &thousands(i.fee.to_sat()),
            &i.fee_rate_sat_vb
                .map(|r| format!("{r:.1}"))
                .unwrap_or_default(),
            &i.fee_share_pct().unwrap_or_default().to_string(),
        ],
    );
    cy += ui.wrap_as((x, cy, w), BODY_M, Role::Error, &facts) + GAP_SMALL;
    cy += ui.wrap_as(
        (x, cy, w),
        BODY_M,
        Role::OnSurfaceVariant,
        EN.step_fee_hold_cases,
    ) + GAP;
    crate::vault::screen::hold_button_in(
        app,
        ui,
        x,
        cy,
        w,
        EN.step_fee_hold,
        Action::ReviewHold,
        faraday_ui::hold::HoldTone::Error,
    );
    cy += CHIP_LINE + GAP;
    cy - y
}

/// The Sign row in a column `w` wide at (`x`, `y`): `lead` at its start
/// and the Sign button, labelled `label`, at its end. Where the two do
/// not fit side by side the button goes under the line, and where its
/// label is wider than the column the label is a line of its own above a
/// button reading Sign. Returns its height.
fn sign_row(ui: &mut Ui, (x, y, w): (f32, f32, f32), lead: &str, label: &str, style: Style) -> f32 {
    let row = BUTTON_H + GAP_SMALL;
    let bw = ui.measure_as(TITLE_S, label) + 2.0 * PAD;
    let lw = if lead.is_empty() {
        0.0
    } else {
        ui.text_mid_as((x, y, BUTTON_H), BODY_S, Role::OnSurfaceVariant, lead);
        ui.measure_as(BODY_S, lead) + GAP
    };
    if lw + bw <= w {
        ui.button(
            x + w - bw,
            y,
            Some(bw),
            BUTTON_H,
            label,
            style,
            Action::SignHere,
        );
        return row;
    }
    let mut cy = if lead.is_empty() { y } else { y + BUTTON_H };
    let label = if bw <= w {
        label
    } else {
        let used = ui.wrap_as((x, cy, w), BODY_S, Role::OnSurfaceVariant, label);
        cy += used + GAP;
        EN.step_sign
    };
    ui.button(x, cy, None, BUTTON_H, label, style, Action::SignHere);
    cy + row - y
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
        let bw = ui.measure_as(LABEL_L, EN.made_line_remove) + 32.0;
        right -= bw;
        ui.button(
            right,
            y,
            Some(bw),
            BUTTON_H,
            EN.made_line_remove,
            Style::Ghost,
            Action::OutboxRemove(k),
        );
        right -= 10.0;
    }
    let sw = ui.measure_as(LABEL_L, &state);
    ui.text_mid_as((right - sw, y, 34.0), LABEL_L, Role::Ok, &state);
    let n = ui.fit_as(BODY_M.mono(), file, (right - sw - 10.0 - x).max(0.0));
    ui.text_mid_as((x, y, 34.0), BODY_M.mono(), Role::OnSurface, &n);
    44.0
}

/// How a nonce was recomputed, as the Nonce check says it.
fn nonce_mode_text(m: faraday_psbt::verify::NonceMode) -> &'static str {
    use faraday_psbt::verify::NonceMode as M;
    match m {
        M::LowR => EN.nonce_mode_low_r,
        M::First => EN.nonce_mode_first,
        M::Bip340 => EN.nonce_mode_bip340,
    }
}

/// Why a signature was not checked, as the Nonce check says it.
fn unchecked_text(u: faraday_psbt::verify::Unchecked) -> &'static str {
    use faraday_psbt::verify::Unchecked as U;
    match u {
        U::NoPrevout => EN.nonce_unchecked_no_prevout,
        U::UnsupportedScript => EN.nonce_unchecked_script,
        U::UnknownKey => EN.nonce_unchecked_no_key,
    }
}
