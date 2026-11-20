//! `Screen::Spend`: signing a transaction.

use super::parts::{made_place, spaced};
use crate::decode::check::SignerPress;
use crate::drawing::{Scrolled, View};
use crate::memory::nonces::History;
use crate::review::unchecked_text;
use crate::strings::EN;
use crate::ui::pal::*;
use crate::ui::{Style, Ui, btc, grouped, short, thousands};
use crate::wallet::{Session, fp_text, network_name, step, txid_known};
use crate::{Action, Screen, flow};
use faraday_draw::tokens::{BODY_M, BODY_S, LABEL_L, LABEL_M, TITLE_M, TITLE_S};
use faraday_draw::tokens::{
    BUTTON_H, CHIP_LINE, FACT_INDENT, GAP, GAP_SMALL, PAD, REVIEW_ADDR_INSET, REVIEW_ADDR_INSET_S,
    REVIEW_ADDR_LINE_S, REVIEW_ADDR_TOP, REVIEW_ADDR_TOP_S, REVIEW_ADDR_UNDER, REVIEW_SCROLL_KEEP,
    TABLE_AMOUNT_W, TABLE_LABEL_W, TypeRole,
};
use faraday_draw::widgets::Icon;
use faraday_psbt::{Level, OutputKind};
use faraday_text::{Text, text};
use faraday_ui::Status;
use faraday_ui::parts::wrap_buttons;

/// A key's fingerprint as text. A lookup.
pub(super) fn fp_label(fp: faraday_bip::keys::Fingerprint) -> Text {
    Text::of(&fp_text(fp))
}

/// The spend's nonce check (`nonce_card`), kept on the spend. Returns
/// its height.
fn nonce_check(app: &View, ui: &mut Ui, psbt: &faraday_psbt::Psbt, x: f32, y: f32, w: f32) -> f32 {
    let Some(check) = app.nonce_check(psbt) else {
        return 0.0;
    };
    nonce_card(ui, &check, x, y, w)
}

/// The nonce check: each signature against its key and this
/// transaction, each one made here recomputed from its key, and any nonce
/// used twice. Drawn on Spend's Send it card and on the nonce check of a
/// transaction brought in (`Screen::NonceCheck`). Returns its height.
pub(crate) fn nonce_card(
    ui: &mut Ui,
    check: &crate::wallet::NonceCheck,
    x: f32,
    y: f32,
    w: f32,
) -> f32 {
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
        let inputs: Vec<Text> = reuse.inputs.iter().map(|i| text!("{}", i)).collect();
        let line = Text::fill(
            EN.nonce_check_one_nonce,
            &[&reuse.name(), &Text::joined(&inputs, ", ")],
        );
        cy += ui.wrap_as((x, cy, w), LABEL_L, Role::Error, &line) + GAP;
    }
    cy += history_lines(ui, check, x, cy, w);
    for input in check.mismatched.iter().take(SHOWN) {
        let text = if check.fields_refused.contains(input) {
            EN.psbt_final_signature_refused
        } else {
            EN.psbt_warn_witness_mismatch
        };
        let line = Text::fill(text, &[&text!("{}", input)]);
        cy += ui.wrap_as((x, cy, w), LABEL_L, Role::Error, &line) + GAP;
    }
    const SHOWN: usize = 12;
    for r in check.rows.iter().take(SHOWN) {
        ui.text_mid_as(
            (x, cy, 30.0),
            BODY_S,
            Role::OnSurfaceVariant,
            &Text::fill(EN.nonce_check_input, &[&text!("{}", r.input)]),
        );
        ui.text_mid_as(
            (x + 70.0, cy, 30.0),
            BODY_M.mono(),
            Role::OnSurface,
            &r.name,
        );
        let (t, c) = row_verdict(r);
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
            &Text::fill(
                EN.nonce_check_more,
                &[
                    &text!("{}", rest.len()),
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

/// The nonce history's lines under the headline: a red line per key used
/// before, and what the history was compared against. Returns the height.
fn history_lines(ui: &mut Ui, check: &crate::wallet::NonceCheck, x: f32, y: f32, w: f32) -> f32 {
    let mut cy = y;
    for used in &check.used_before {
        let inputs: Vec<Text> = used.inputs.iter().map(|i| text!("{}", i)).collect();
        let line = Text::fill(
            EN.nonce_history_used,
            &[&used.name, &Text::joined(&inputs, ", ")],
        );
        cy += ui.wrap_as((x, cy, w), LABEL_L, Role::Error, &line) + GAP;
    }
    let (line, tone) = match check.history {
        History::Clean { .. } if !check.used_before.is_empty() => return cy - y,
        History::Off => return cy - y,
        History::Clean { vaults } if vaults > 1 => (
            Text::fill(EN.nonce_history_clean_vaults, &[&text!("{}", vaults)]),
            Role::OnSurfaceVariant,
        ),
        History::Clean { .. } => (Text::of(EN.nonce_history_clean), Role::OnSurfaceVariant),
        History::Unread { records, .. } => (
            Text::fill(EN.nonce_history_unread, &[&text!("{}", records)]),
            Role::Warning,
        ),
    };
    cy += ui.wrap_as((x, cy, w), LABEL_L, tone, &line) + GAP;
    cy - y
}

/// A nonce check row's verdict and its tone. A lookup.
fn row_verdict(r: &crate::wallet::NonceRow) -> (Text, Role) {
    use crate::wallet::Unmatched;
    use faraday_psbt::verify::Verdict;
    match (r.verdict, r.recomputed) {
        (Verdict::Invalid, _) => (Text::of(EN.nonce_check_invalid), Role::Error),
        (Verdict::Unchecked(_), _) if r.preimage => {
            (Text::of(EN.nonce_check_preimage), Role::OnSurfaceVariant)
        }
        (Verdict::Unchecked(why), _) => (
            Text::fill(EN.nonce_check_not_checked, &[unchecked_text(why)]),
            Role::OnSurfaceVariant,
        ),
        (Verdict::Valid, Some(Some(mode))) => (
            Text::fill(EN.nonce_check_valid_nonce, &[nonce_mode_text(mode)]),
            Role::Ok,
        ),
        (Verdict::Valid, Some(None)) => (Text::of(EN.nonce_check_wrong_nonce), Role::Error),
        (Verdict::Valid, None) => (
            Text::of(match r.unmatched {
                Some(Unmatched::NoOrigin) => EN.nonce_check_no_origin,
                Some(Unmatched::Finalized) => EN.nonce_check_finalized_here,
                Some(Unmatched::NotRun) => EN.nonce_check_script_not_run_key,
                None => EN.nonce_check_valid_signed,
            }),
            Role::OnSurfaceVariant,
        ),
    }
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

/// The head of the page with nothing loaded, and how far down it ends. On
/// the wide layout the top app bar names the page. On the small panel it
/// is the bar with its tools drawn over its right end
/// (`compact::flow_bar_tools`): the name is kept left of them, whole words
/// or none.
fn empty_head(app: &View, ui: &mut Ui, cw: f32) -> f32 {
    let back = app.named_back((EN.spend_wallets, Action::Nav(Screen::WalletsStart)));
    if !ui.compact {
        return super::bar_head(ui, Some(back), EN.spend_sign_a_transaction);
    }
    let head = faraday_ui::nav::AppBar {
        back: Some(back),
        title: EN.spend_sign_a_transaction,
        back_tip: EN.tip_back,
    };
    let right = ui.bar_tools_x.unwrap_or(cw);
    faraday_ui::nav::app_bar(ui, cw, &head, right);
    crate::compact::BAR_H + faraday_draw::tokens::GAP
}

pub(super) fn draw(app: &View, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    let col_x = x0 + 40.0;
    let needed = app.spend_needed();
    if app.wallets.spend().is_none() {
        let top = empty_head(app, ui, cw);
        super::parts::nothing_to_work_on(
            app,
            ui,
            (col_x, top, cw - 2.0 * 40.0),
            EN.spend_no_transaction,
            (Some(Action::Scan), Screen::Spend),
        );
        return;
    }
    let (wallet_idx, scroll) = {
        let s = app.wallets.spend().expect("spend");
        (
            s.wallet,
            flow::Scroll {
                y: s.scroll,
                follow: s.follow,
            },
        )
    };
    let compact = ui.compact;
    let sequence = sequence_line(app);
    let page = flow::Page {
        area: (x0, cw),
        h,
        foot_h: if compact { spend_bar_h(ui) } else { 0.0 },
        back: Some(app.named_back((EN.spend_wallets, Action::Nav(Screen::WalletsStart)))),
        heading: EN.spend_sign_a_transaction,
        chip: None,
        note: sequence.as_deref(),
        chip_tap: None,
        step: None,
        moves: crate::ui::Moves::Yes,
    };
    let Some(contained) = crate::trail::StepFlow::contained(&crate::wallet::SPEND_STEPS) else {
        return;
    };
    let (next, again) = {
        let app_ref: &View = app;
        let mut body = |ui: &mut Ui, k: u8, x: f32, y: f32, w: f32| -> f32 {
            step_body(app_ref, ui, k, x, y, w, wallet_idx, needed)
        };
        // Under Your choices' rows: the transaction's review.
        let mut under = |ui: &mut Ui, x: f32, y: f32, w: f32| -> f32 {
            review_under(app_ref, ui, (x, y, w), wallet_idx, needed)
        };
        let (next, again, _) =
            flow::contained(ui, app_ref, contained, &page, scroll, &mut body, &mut under);
        (next, again)
    };
    app.scroll(Scrolled::Spend, next, again);

    // On a small panel, the foot, whose button is the open step's
    // Continue while it is read.
    if compact {
        let step = ui.pin.take();
        let bar_y = h - spend_bar_h(ui);
        spend_bar(app, ui, x0, cw, bar_y, needed, step);
    }
}

/// Where a signing that came from a stick stands: its step of four, by
/// name. None for a spend that did not.
fn sequence_line(app: &View) -> Option<Text> {
    app.visit.sign_stick()?;
    let s = app.wallets.spend()?;
    let noun = app.medium.noun();
    let key_here = s
        .inspection
        .participating_keys
        .iter()
        .any(|fp| app.session.keys.key_label(*fp).is_some() && s.to_sign.contains(&fp.0));
    let (n, name) = if s.spend.finished.is_some() {
        (4, Text::fill(EN.seq_write, &[noun]))
    } else if key_here || s.complete {
        (3, Text::of(EN.seq_sign))
    } else if app.stick.is_empty() {
        (2, Text::of(EN.seq_load))
    } else {
        (1, Text::fill(EN.seq_pull, &[noun]))
    };
    Some(Text::fill(EN.seq_step, &[&Text::num(n), &name]))
}

/// The end of a signing: what waits for the stick, and what to do next:
/// another transaction, lock, power off. Returns its height.
pub(super) fn end_card(app: &View, ui: &mut Ui, (x, y, w): (f32, f32, f32), waits: usize) -> f32 {
    let mut cy = y + GAP_SMALL;
    if waits > 0 {
        let noun = app.medium.noun();
        let line = if waits == 1 {
            Text::fill(EN.end_one_waits, &[noun])
        } else {
            Text::fill(EN.end_files_wait, &[&text!("{}", waits), noun])
        };
        cy += ui.wrap_as((x, cy, w), BODY_M, Role::Warning, &line) + faraday_draw::tokens::GAP;
    }
    cy += wrap_buttons(
        ui,
        x,
        cy,
        w,
        BUTTON_H,
        &[
            (
                EN.spend_status_sign_another,
                Style::Secondary,
                Action::Nav(Screen::Files),
            ),
            (EN.spend_backup_lock, Style::Secondary, Action::LockAsk),
            (
                EN.spend_backup_power_off,
                Style::Secondary,
                Action::PowerAsk,
            ),
        ],
    );
    cy - y
}

/// Whether the spend has no Signers and no Signatures step (one key signs)
/// and its key is not loaded.
pub(crate) fn single_key_unloaded(
    session: &crate::wallet::Session,
    s: &crate::wallet::SpendState,
) -> bool {
    !s.steps.contains(&step::COLLECT)
        && !s.steps.contains(&step::SIGNERS)
        && s.wallet
            .and_then(|wi| session.wallets.get(wi))
            .and_then(|wl| session.slots(wl).into_iter().next())
            .is_some_and(|slot| slot.held_by.is_none())
}

/// The vaults not open, each as the button that unlocks it.
fn unlock_items(app: &View) -> Vec<(Text, Action)> {
    let back = app.screen;
    app.vault_files()
        .iter()
        .enumerate()
        .filter(|(_, f)| f.open.is_none())
        .map(|(k, f)| {
            (
                Text::fill(EN.step_unlock, &[&f.name]),
                Action::Vault(crate::vault::VaultAction::OpenFrom(k, back)),
            )
        })
        .collect()
}

/// The Sign card of a single-key spend whose key is not loaded: the ways
/// to load the wallet's key (type the words, scan a SeedQR, open a vault),
/// dimmed under the line that keys load once the stick is out while one
/// is attached. Returns the height used.
fn single_key_routes(app: &View, ui: &mut Ui, (x, y, w): (f32, f32, f32), wi: usize) -> f32 {
    let may = app.may_load_keys();
    let mut cy = y;
    let style = Style::Secondary.when(may);
    let from_vault = app
        .session
        .wallets
        .get(wi)
        .and_then(|wl| app.session.slots(wl).into_iter().next())
        .and_then(|slot| {
            slot.fingerprint
                .filter(|_| slot.held_by.is_none() && app.vault_key_for(&slot).is_some())
        });
    let files = app.vault_files();
    let back = app.screen;
    let mut opens: Vec<(Text, Action)> = Vec::new();
    for (k, f) in files.iter().enumerate().filter(|(_, f)| f.open.is_none()) {
        opens.push((
            Text::fill(EN.step_open_vault, &[&f.name]),
            Action::Vault(crate::vault::VaultAction::OpenFrom(k, back)),
        ));
    }
    if files.is_empty() {
        opens.push((Text::of(EN.step_open_a_vault), Action::Nav(Screen::Vaults)));
    }
    let mut items: Vec<(&str, Style, Action)> = vec![
        (
            EN.step_type_the_words,
            Style::Primary.when(may),
            Action::EntryWallet(wi),
        ),
        (EN.step_scan_a_seedqr, style, Action::ScanKey(wi)),
    ];
    if let Some(f) = from_vault {
        items.push((
            EN.step_load_from_vault,
            style,
            Action::Vault(crate::vault::VaultAction::LoadKeyOf(
                f.0,
                crate::vault::SlotOf::Loaded(wi, 0),
            )),
        ));
    }
    items.extend(opens.iter().map(|(l, a)| (l.as_str(), style, *a)));
    cy += wrap_buttons(ui, x, cy, w, BUTTON_H, &items) + 2.0;
    if !may {
        let pull = Text::fill(EN.inbox_panel_keys_load, &[app.medium.noun()]);
        ui.text_as(x, cy, LABEL_L, Role::Warning, &pull);
        cy += ui.line_as(LABEL_L) + faraday_draw::tokens::GRID;
    }
    cy - y
}

/// Where a signing stands: the signatures there, how many are missing,
/// the line that says what is left, the main button's label and its
/// style: off while a check on Transaction stops signing here (F136).
fn spend_status(app: &View, needed: usize) -> (usize, usize, Text, ActionLabel, Style) {
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
    // What is still to come: a partial in the file the device cannot
    // confirm is not counted as signed, and is not asked for again.
    let to_come = missing.saturating_sub(s.in_file.len()).max(1);
    let can_sign_here = i
        .participating_keys
        .iter()
        .filter(|fp| app.session.keys.key_label(**fp).is_some() && s.to_sign.contains(&fp.0))
        .map(|fp| fp_label(*fp))
        .collect::<Vec<_>>();
    let stopped = s.spend.finished.is_none()
        && missing > 0
        && !can_sign_here.is_empty()
        && crate::review::review_stop(i, &s.held).is_some();
    let need_line = if stopped {
        Text::fill(EN.spend_status_stopped, &[EN.app_transaction])
    } else if s.spend.finished.is_some() && app.spend_outbox_names().len() == 2 {
        Text::fill(EN.spend_status_both_wait, &[app.medium.for_the()])
    } else if missing == 0 {
        Text::of(EN.spend_status_complete)
    } else if !can_sign_here.is_empty() && to_come > 1 {
        Text::fill(
            EN.spend_status_sign_and_collect,
            &[&text!("{}", (to_come - 1))],
        )
    } else if !can_sign_here.is_empty() {
        Text::of(EN.spend_status_sign_here)
    } else if single_key_unloaded(&app.session, s) {
        Text::of(EN.step_no_signing_key)
    } else {
        Text::of(&crate::wallet::collect_line(to_come, count))
    };
    let label = if (s.spend.finished.is_some() || missing == 0) && s.review_owed_shown().is_some() {
        // Review comes first, also for a PSBT signed elsewhere.
        Text::of(EN.wallets_review)
    } else if s.spend.finished.is_some() {
        // Files wait For the stick: one press locks and writes them when
        // it is in. Nothing waits: the next transaction.
        if !app.spend_outbox_names().is_empty() {
            Text::fill(EN.spend_status_lock_write, &[app.medium.noun()])
        } else {
            Text::of(EN.spend_status_sign_another)
        }
    } else if missing == 0 {
        Text::of(EN.spend_status_finish)
    } else if !can_sign_here.is_empty() {
        Text::fill(
            EN.spend_status_sign_with_keys,
            &[&Text::joined(&can_sign_here, ", ")],
        )
    } else if single_key_unloaded(&app.session, s) {
        // No Signatures step here: the button does what the Sign
        // card's lead does.
        Text::of(EN.step_load_the_key)
    } else {
        Text::of(&crate::wallet::collect_line(to_come, count))
    };
    // Two keys or more and Finish not reached: counted where named they
    // would not fit the button.
    let short = (s.spend.finished.is_none() && missing > 0 && can_sign_here.len() > 1).then(|| {
        Text::fill(
            EN.spend_status_sign_with_count,
            &[&text!("{}", can_sign_here.len())],
        )
    });
    // A press that waits for the system's randomness says so.
    let waits = app.waits_for_randomness(Action::SignHere);
    let label = if waits {
        ActionLabel {
            long: Text::of(EN.waiting_for_randomness),
            short: None,
        }
    } else {
        ActionLabel { long: label, short }
    };
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
    long: Text,
    short: Option<Text>,
}

impl ActionLabel {
    /// The label as it fits `room` in `role`: the keys named where they
    /// fit, else counted; whatever is left too wide, cut.
    fn fitted(&self, ui: &Ui, role: TypeRole, room: f32) -> Text {
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
    step: Option<(faraday_text::Text, Style, Action)>,
) {
    let (count, missing, need_line, label, style) = spend_status(app, needed);
    let (x, w) = (x0 + crate::compact::M, cw - 2.0 * crate::compact::M);
    // The button's room for its label.
    let room = w - 24.0;
    let label = faraday_text::Text::of(&label.fitted(ui, TITLE_M, room));
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
        &Text::fill(
            EN.spend_bar_count,
            &[&text!("{}", count), &text!("{}", needed)],
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

pub(crate) fn step_summary(
    app: &crate::Faraday,
    n: u8,
    wallet: Option<usize>,
    needed: usize,
) -> Text {
    let s = app.wallets.spend().expect("spend");
    let i = &s.inspection;
    let reached = s.done[n as usize] || s.open == Some(n);
    match n {
        0 => match wallet.and_then(|w| app.session.wallets.get(w)) {
            Some(w) => text!("{} · {}", &*w.name, &*Session::shape(w)),
            None => Text::of(EN.step_summary_no_loaded),
        },
        1 if reached => Text::of(EN.step_summary_addresses),
        2 if reached => match i.fee_rate_sat_vb {
            Some(rate) => Text::fill(
                EN.step_summary_sends,
                &[&btc(i.amount_to_others.to_sat()), &text!("{rate:.1}")],
            ),
            None => Text::fill(
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
            Text::fill(
                EN.step_summary_here_needed,
                &[&text!("{}", here), &text!("{}", needed)],
            )
        }
        5 => {
            let signed = s.signed_keys();
            if signed.is_empty() {
                Text::of(EN.step_summary_not_signed)
            } else {
                let fps: Vec<Text> = signed.iter().map(|f| fp_label(*f)).collect();
                Text::fill(EN.step_summary_signed_keys, &[&Text::joined(&fps, ", ")])
            }
        }
        6 => Text::fill(
            EN.step_summary_count,
            &[&text!("{}", s.signers.len()), &text!("{}", needed)],
        ),
        9 => match wallet.and_then(|w| app.session.wallets.get(w)) {
            Some(w) => {
                let rows = crate::wallet::musig_nonces(&app.session, w, &s.spend.psbt);
                let have = rows.iter().filter(|r| r.3).count();
                Text::fill(
                    EN.step_summary_of_nonces,
                    &[&text!("{}", have), &text!("{}", rows.len())],
                )
            }
            None => Text::new(),
        },
        8 => {
            let n = wallet
                .and_then(|w| app.session.wallets.get(w))
                .map(|w| w.policy().spend_paths().len())
                .unwrap_or(0);
            text!(
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
                Text::of(EN.step_summary_signed)
            } else {
                Text::of(EN.step_summary_not_finished)
            }
        }
        _ => Text::of(EN.step_summary_not_reached),
    }
}

/// Draws a step's body with its top-left at (x, y). Returns its height.
#[allow(
    clippy::too_many_arguments,
    reason = "each spend step takes the view, step number, position, width, wallet and keys needed"
)]
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
    let v = StepView {
        app,
        n,
        x,
        w,
        wallet,
        needed,
        s,
        i: &s.inspection,
    };
    let mut cy = match n {
        0 => v.step_wallet(ui, y),
        1 => v.step_addresses(ui, y),
        2 => v.review(ui, y),
        4 => v.step_keys(ui, y),
        5 => v.step_sign(ui, y),
        6 => v.step_collect(ui, y),
        7 => v.step_finish(ui, y),
        9 => v.step_nonces(ui, y),
        8 => v.step_paths(ui, y),
        _ => y,
    };
    if let Some(e) = &s.error {
        let used = ui.wrap_as((x, cy, w), BODY_M, Role::Error, e);
        cy += used + 6.0;
    }
    cy - y
}

/// What a step's body draws from: the view, the step, where it goes and the
/// spend with its inspection.
#[derive(Clone, Copy)]
struct StepView<'a> {
    app: &'a View<'a>,
    n: u8,
    x: f32,
    w: f32,
    wallet: Option<usize>,
    needed: usize,
    s: &'a crate::wallet::SpendState,
    i: &'a faraday_psbt::Inspection,
}

impl StepView<'_> {
    /// A step's Continue: on a small panel it takes the foot's button
    /// ([`spend_bar`]) while the step is read.
    fn next(&self, ui: &mut Ui, cy: f32, label: &str) {
        let Self { n, x, w, .. } = *self;
        if ui.pinning {
            ui.pin = Some((
                faraday_text::Text::of(label),
                Style::Primary,
                Action::StepNext(n),
            ));
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
    }

    /// The review: its table, the other transactions, the warnings, the
    /// inputs and outputs, and the foot.
    fn review(&self, ui: &mut Ui, y: f32) -> f32 {
        let mut cy = self.review_rows(ui, y);
        cy = self.review_others(ui, cy);
        cy = self.review_warnings(ui, cy);
        cy = self.review_table(ui, cy);
        self.review_foot(ui, cy)
    }

    /// The wallet the spend is for, by name and where it came from.
    fn step_wallet(&self, ui: &mut Ui, y: f32) -> f32 {
        let Self {
            app, x, w, wallet, ..
        } = *self;
        let mut cy = y;
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
                    &Text::fill(EN.step_wallet_source, &[&*Session::shape(wl), &*wl.source]),
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
        self.next(ui, cy, EN.step_continue);
        cy += row;
        cy
    }

    /// The wallet's first addresses, for the person to compare.
    fn step_addresses(&self, ui: &mut Ui, y: f32) -> f32 {
        let Self {
            app, x, w, wallet, ..
        } = *self;
        let mut cy = y;
        match wallet.and_then(|wi| app.session.wallets.get(wi)) {
            Some(wl) => {
                // Where an address starts beside its label.
                let beside = 96.0;
                cy += super::parts::keys_caution(ui, wl, (x, cy, w));
                for (label, change, idx) in [
                    (EN.step_receive_0_0, false, 0),
                    (EN.step_receive_0_1, false, 1),
                    (EN.step_change_1_0, true, 0),
                ] {
                    ui.text_as(x, cy + 2.0, BODY_S, Role::OnSurfaceVariant, label);
                    let (a, held) = app.receive_line(wl, change, idx);
                    // On a small panel, or where the label leaves it
                    // no room beside, the address goes under its label.
                    let ax = if ui.compact || ui.measure_as(BODY_S, label) + GAP_SMALL > beside {
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
        self.next(ui, cy, EN.step_continue);
        cy += 44.0;
        cy
    }

    /// The review's table: what is sent, the change, the fee, the inputs and the transaction id.
    fn review_rows(&self, ui: &mut Ui, y: f32) -> f32 {
        let Self { x, w, s, i, .. } = *self;
        let mut cy = y;
        let rows_top = cy;
        let mut rows: Vec<ReviewRow> = Vec::new();
        // Each recipient's whole address, drawn under its Sends row
        // in groups of four: a person compares all of it, never its
        // first and last characters.
        let mut whole: Vec<Option<Text>> = Vec::new();
        // On a small panel the first row is what goes to others.
        if ui.compact {
            let n = recipients(i).len();
            rows.push((
                Text::of(EN.step_to_others),
                btc(i.amount_to_others.to_sat()),
                text!(
                    "{n} {}",
                    if n == 1 {
                        EN.step_recipient_inline
                    } else {
                        EN.step_recipients
                    }
                ),
                None,
            ));
            whole.push(None);
        }
        for (k, o) in i.outputs.iter().enumerate().filter(|(_, o)| !o.is_ours()) {
            // A payment into another of the person's wallets names it.
            let to = match s.own_names.get(k) {
                Some(Some(n)) => Text::fill(EN.step_to_wallet, &[n]),
                _ => Text::new(),
            };
            rows.push((Text::of(EN.step_sends), btc(o.amount.to_sat()), to, None));
            whole.push(Some(grouped(&o.address)));
        }
        for (k, o) in i.outputs.iter().enumerate() {
            match o.kind {
                OutputKind::WalletChange { change, index, .. }
                | OutputKind::Change { change, index, .. } => {
                    // Change is on the change chain; an own address on the
                    // receive chain reads as one that receives (F254).
                    let path = Text::fill(
                        EN.step_to_path,
                        &[
                            &text!("{}", (if change { 1 } else { 0 })),
                            &text!("{}", index),
                        ],
                    );
                    let detail = match s.own_names.get(k) {
                        Some(Some(n)) => text!("{} · {}", &**n, &*path),
                        _ => path,
                    };
                    rows.push((
                        Text::of(if change {
                            EN.step_change
                        } else {
                            EN.step_receive
                        }),
                        btc(o.amount.to_sat()),
                        detail,
                        Some((EN.step_verified, Role::Ok.paint())),
                    ));
                }
                // Drawn under Sends with the others (F254).
                OutputKind::Transfer { .. }
                | OutputKind::UnverifiedChange { .. }
                | OutputKind::Recipient => {}
            }
        }
        // A fee that cannot be worked out is not known, never zero.
        rows.push(match i.fee_rate_sat_vb {
            Some(rate) => (
                Text::of(EN.step_fee),
                btc(i.fee.to_sat()),
                Text::fill(
                    EN.step_sat_sat_vb,
                    &[&thousands(i.fee.to_sat()), &text!("{rate:.1}")],
                ),
                None,
            ),
            None => (
                Text::of(EN.step_fee),
                Text::of(EN.step_fee_not_known),
                Text::new(),
                None,
            ),
        });
        let unchecked = i.inputs.iter().any(|inp| inp.amount.is_none());
        rows.push((
            Text::of(EN.step_spends),
            btc(i.total_in.to_sat()),
            text!(
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
        if let (true, faraday_psbt::LocktimeKind::Height(h)) = (i.locktime.holds, i.locktime.kind) {
            rows.push((
                Text::of(EN.step_locked_until),
                Text::fill(EN.psbt_locktime_block, &[&text!("{h}")]),
                Text::new(),
                None,
            ));
        }
        rows.push((
            Text::of(EN.step_replacement),
            Text::of(if i.rbf {
                EN.step_replaceable
            } else {
                EN.step_not_signalled
            }),
            Text::new(),
            None,
        ));
        // The transaction's id, at the foot: known before signing
        // for a segwit spend, not for a legacy one.
        if txid_known(&s.spend.psbt) {
            let segwit = i
                .inputs
                .iter()
                .all(|inp| unchanged_by_signing(inp.script_type));
            rows.push((
                Text::of(EN.step_txid),
                short(&text!("{}", s.spend.txid)),
                Text::of(if segwit {
                    EN.step_unchanged_by_signing
                } else {
                    EN.step_changes_when_signed
                }),
                None,
            ));
        } else {
            rows.push((
                Text::of(EN.step_txid),
                Text::of(EN.step_not_known_until),
                Text::new(),
                None,
            ));
        }
        // On a small panel each row is two lines: what and how much,
        // then where and its note.
        let compact = ui.compact;
        let row_h = if compact { 52.0 } else { 40.0 };
        // A recipient's row holds its address's lines under the amount.
        let addr_role = BODY_S.mono();
        let inset = if compact {
            REVIEW_ADDR_INSET_S
        } else {
            REVIEW_ADDR_INSET
        };
        let addr_w = w - 2.0 * inset;
        let top = if compact {
            REVIEW_ADDR_TOP_S
        } else {
            REVIEW_ADDR_TOP
        };
        let row_hs: Vec<f32> = (0..rows.len())
            .map(|k| match whole.get(k).and_then(Option::as_ref) {
                Some(a) => {
                    top + ui.wrap_lines_as(addr_role, a, addr_w) as f32 * ui.line_as(addr_role)
                        + REVIEW_ADDR_UNDER
                }
                None => row_h,
            })
            .collect();
        let box_h: f32 = row_hs.iter().sum();
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
            let ry = rows_top + row_hs[..k].iter().sum::<f32>();
            let row_h = row_hs[k];
            if let Some(a) = whole.get(k).and_then(Option::as_ref) {
                let lh = if compact { REVIEW_ADDR_LINE_S } else { top };
                let (lx, ly) = (inset, if compact { GAP_SMALL } else { 0.0 });
                // A payment into another of the person's wallets names it
                // beside the label.
                let label = if rest.is_empty() {
                    label.clone()
                } else {
                    text!("{} · {}", &**label, &**rest)
                };
                ui.text_mid_as(
                    (x + lx, ry + ly, lh),
                    BODY_S,
                    Role::OnSurfaceVariant,
                    &label,
                );
                ui.text_right_as(
                    (x + w - lx, ry + ly, lh),
                    BODY_M.mono(),
                    Role::OnSurface,
                    amount,
                );
                ui.wrap_as(
                    (x + lx, ry + top, addr_w),
                    addr_role,
                    Role::OnSurfaceVariant,
                    a,
                );
            } else if compact {
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
        cy
    }

    /// The other transactions spending the same inputs, each with its fee bump result.
    fn review_others(&self, ui: &mut Ui, y: f32) -> f32 {
        let Self { x, w, s, .. } = *self;
        let mut cy = y;
        // The other transactions spending the same inputs, each with
        // its fee bump result, as kept with the inspection
        // (`docs/COMPARE.md` §4.2).
        if !s.others.is_empty() {
            ui.text_as(
                x,
                cy,
                LABEL_L,
                Role::OnSurfaceVariant,
                EN.compare_fee_bump_label,
            );
            cy += ui.line_as(LABEL_L) + GAP_SMALL;
            for o in &s.others {
                let tone = match o.outcome {
                    faraday_psbt::bump::Outcome::OnlyFee => Role::Ok,
                    faraday_psbt::bump::Outcome::MoreChanged => Role::Error,
                    faraday_psbt::bump::Outcome::CannotTell => Role::Warning,
                };
                cy += ui.wrap_as((x, cy, w), TITLE_S, tone, &o.result) + GAP_SMALL;
                for line in &o.lines {
                    cy += warning_row(ui, (x, cy, w), tone, line);
                }
                for fact in &o.facts {
                    ui.text_as(x, cy, BODY_S, Role::OnSurfaceVariant, fact.label);
                    cy += ui.line_as(BODY_S) + 2.0;
                    for v in &fact.values {
                        let text = match v.label {
                            Some(l) => text!("{l}: {}", v.text),
                            None => Text::of(&v.text),
                        };
                        cy += ui.wrap_as(
                            (x + FACT_INDENT, cy, w - FACT_INDENT),
                            BODY_M,
                            Role::OnSurface,
                            &text,
                        ) + 2.0;
                    }
                }
                let bw = ui.measure_as(LABEL_L, EN.compare_from_review) + 2.0 * PAD;
                ui.button(
                    x,
                    cy,
                    Some(bw.min(w)),
                    BUTTON_H,
                    EN.compare_from_review,
                    Style::Secondary,
                    Action::SignerCheck(SignerPress::FromReview(o.place.0, o.place.1)),
                );
                cy += BUTTON_H + GAP_SMALL;
            }
        }
        cy
    }

    /// The review's warnings, the way to register the wallet, and the tags
    /// shown when there are none.
    fn review_warnings(&self, ui: &mut Ui, y: f32) -> f32 {
        let Self {
            app,
            x,
            w,
            wallet,
            i,
            ..
        } = *self;
        let mut cy = y;
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
            .map(|t| (Role::Warning, crate::review::Words::Fixed(t.as_str())))
            .chain(i.warnings.iter().map(|wn| {
                let c = match wn.level {
                    Level::Info => Role::OnSurfaceVariant,
                    Level::Caution => Role::Warning,
                    Level::Danger | Level::Blocked => Role::Error,
                };
                (c, crate::review::warning_text(i, wn))
            }));
        for (c, text) in lines {
            cy += warning_row(ui, (x, cy, w), c, &text);
            warned = true;
        }
        // Held for want of a registered wallet: register the one the
        // PSBT defines, or, where it defines none, load its
        // descriptor (`docs/SPEND-FROM-BACKUP.md` §8).
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
        if let Some((wi, wl)) = wallet.and_then(|wi| Some((wi, app.session.wallets.get(wi)?))) {
            let of = crate::wallet::KeyBlockOf::Wallet(u16::try_from(wi).unwrap_or(u16::MAX));
            cy += crate::wallet::draw_review_keys(wl, of, ui, (x, cy, w));
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
        cy
    }

    /// The inputs and outputs listed under the review, when shown.
    fn review_table(&self, ui: &mut Ui, y: f32) -> f32 {
        let Self { x, w, s, i, .. } = *self;
        let mut cy = y;
        if s.table && ui.compact {
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
                            .unwrap_or_else(|| Text::of("?")),
                        text!("{}:{}", &*short(&text!("{}", inp.txid)), inp.vout),
                    )
                })
                .chain(
                    i.outputs
                        .iter()
                        .map(|o| (EN.step_output, btc(o.amount.to_sat()), Text::of(&o.address))),
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
            let outpoints: Vec<Text> = i
                .inputs
                .iter()
                .map(|inp| text!("{}:{}", &*short(&text!("{}", inp.txid)), inp.vout))
                .collect();
            let what = outpoints
                .iter()
                .map(Text::as_str)
                .chain(i.outputs.iter().map(|o| o.address.as_str()))
                .map(|t| ui.measure_as(BODY_S.mono(), t))
                .fold(0.0, f32::max);
            let tw = (TABLE_LABEL_W + what + GAP_SMALL + TABLE_AMOUNT_W).min(w);
            for (inp, at) in i.inputs.iter().zip(&outpoints) {
                let a = inp
                    .amount
                    .map(|a| btc(a.to_sat()))
                    .unwrap_or_else(|| Text::of("?"));
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
        cy
    }

    /// The review's stop, its table button and Continue.
    fn review_foot(&self, ui: &mut Ui, y: f32) -> f32 {
        let Self {
            app, x, w, s, i, ..
        } = *self;
        let mut cy = y;
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
            // On a small panel Continue waits for the end of the card
            // (the fee, every recipient, every warning) to be in view;
            // until then the foot's button scrolls a view down.
            match ui.page_view {
                Some((foot, view)) if ui.pinning && cy + BUTTON_H > foot => {
                    ui.pin = Some((
                        faraday_text::Text::of(EN.step_scroll_to_end),
                        Style::Primary,
                        Action::ReviewScroll((view - REVIEW_SCROLL_KEEP).max(1.0) as u16),
                    ));
                }
                _ => self.next(ui, cy, EN.step_continue),
            }
        }
        cy += 44.0;
        cy
    }

    /// The wallet's keys, which sign here, and the ways to bring the rest.
    fn step_keys(&self, ui: &mut Ui, y: f32) -> f32 {
        let Self {
            app,
            x,
            w,
            wallet,
            needed,
            s,
            i,
            ..
        } = *self;
        let mut cy = y;
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
                        &text!("{}", (k + 1)),
                    );
                    let fp = slot.fingerprint.map(fp_label).unwrap_or_default();
                    ui.text_mid_as((x + 22.0, cy, 42.0), BODY_M.mono(), Role::OnSurface, &fp);
                    // A label that is its fingerprint is not drawn
                    // twice.
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
                    let from_vault = slot
                        .fingerprint
                        .filter(|_| slot.held_by.is_none() && app.vault_key_for(slot).is_some());
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
                        &fp_label(*fp),
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
        // until the spend is signed.
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
            let unlocks = unlock_items(app);
            let mut items: Vec<(&str, Style, Action)> = Vec::new();
            if let Some((wi, _)) = wl {
                let scan = Style::Primary.when(may);
                items.push((EN.step_scan_a_key, scan, Action::ScanKey(wi)));
                items.push((
                    EN.step_type_the_words,
                    Style::Secondary.when(may),
                    Action::EntryWallet(wi),
                ));
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
                let pull = Text::fill(EN.step_remove_the_first, &[app.medium.noun()]);
                ui.text_as(x, cy, LABEL_L, Role::Warning, &pull);
                cy += ui.line_as(LABEL_L) + faraday_draw::tokens::GRID;
            }
            cy += 6.0;
        }
        let in_the_file = if s.in_file.is_empty() {
            0
        } else {
            s.signers.len() + s.in_file.len()
        };
        cy = signers_count(ui, x, cy, w, here, in_the_file, needed);
        self.next(ui, cy, EN.step_continue);
        cy += 44.0;
        cy
    }

    /// The sign step: the keys that sign here and the button to sign.
    fn step_sign(&self, ui: &mut Ui, y: f32) -> f32 {
        let Self {
            app, x, w, s, i, ..
        } = *self;
        let mut cy = y;
        let mine: Vec<Text> = i
            .participating_keys
            .iter()
            .filter(|fp| app.session.keys.key_label(**fp).is_some() && s.to_sign.contains(&fp.0))
            .map(|fp| fp_label(*fp))
            .collect();
        let signed = s.signed_keys();
        if !signed.is_empty() {
            let fps: Vec<Text> = signed.iter().map(|f| fp_label(*f)).collect();
            let text = Text::fill(EN.step_signed_keys, &[&Text::joined(&fps, ", ")]);
            ui.icon(x, cy + 10.0, 20.0, Icon::Done, 12.0, Role::Ok);
            let tx = x + 26.0;
            if ui.measure_as(TITLE_S, &text) <= x + w - tx {
                ui.text_mid_as((tx, cy, BUTTON_H), TITLE_S, Role::Ok, &text);
                cy += BUTTON_H + GAP_SMALL;
            } else {
                // The keys wrap under one another on a narrow panel.
                cy += ui.wrap_as((tx, cy + GAP, x + w - tx), TITLE_S, Role::Ok, &text) + 2.0 * GAP;
            }
        } else if mine.is_empty() {
            // A key of this session whose partial signature is in the
            // file is not "no key": it signed, and is not signed again.
            let line =
                if i.participating_keys.iter().any(|fp| {
                    app.session.keys.key_label(*fp).is_some() && s.in_file.contains(&fp.0)
                }) {
                    EN.step_in_file_unconfirmed
                } else {
                    EN.step_no_signing_key
                };
            ui.text_mid_as((x, cy, 40.0), BODY_S, Role::OnSurfaceVariant, line);
            cy += 44.0;
            // A single-key spend has no Signers step: the ways to bring
            // the wallet's key are here, unless it is already held.
            if let Some(wi) = s.wallet.filter(|_| single_key_unloaded(&app.session, s)) {
                cy += single_key_routes(app, ui, (x, cy, w), wi);
            }
        } else {
            let label = Text::fill(EN.step_sign_with_keys, &[&Text::joined(&mine, ", ")]);
            let inputs = text!(
                "{} {}",
                i.inputs.len(),
                if i.inputs.len() == 1 {
                    EN.step_input_inline
                } else {
                    EN.step_inputs
                }
            );
            // A MuSig2 round opened here draws its secret nonces from
            // the system's randomness (`docs/OWN-RANDOMNESS.md` §8.1
            // row 13).
            if crate::spend_opens_round(i) {
                cy += super::randomness::row(app, ui, (x, cy, w));
            }
            let label = app.wait_label(Action::SignHere, &label);
            cy += sign_row(ui, (x, cy, w), &inputs, label, Style::Primary);
        }
        cy
    }

    /// The keys that signed, the cosigners' copies and the ways to collect more.
    fn step_collect(&self, ui: &mut Ui, y: f32) -> f32 {
        let Self {
            app,
            x,
            w,
            wallet,
            s,
            i,
            ..
        } = *self;
        let mut cy = y;
        let slots: Vec<(Text, Text, Option<[u8; 4]>)> =
            match wallet.and_then(|wi| app.session.wallets.get(wi)) {
                Some(wl) => app
                    .session
                    .slots(wl)
                    .iter()
                    .map(|sl| {
                        (
                            sl.fingerprint.map(fp_label).unwrap_or_default(),
                            sl.held_by.as_deref().map(Text::of).unwrap_or_default(),
                            sl.fingerprint.map(|f| f.0),
                        )
                    })
                    .collect(),
                None => i
                    .participating_keys
                    .iter()
                    .map(|fp| (fp_label(*fp), Text::new(), Some(fp.0)))
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
                    .map(|(_, src)| Text::fill(EN.step_from_inline, &[src]))
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
            let listed = |v: &[[u8; 4]]| raw.is_some_and(|r| v.contains(&r));
            let (t, c) = if signed && here {
                (EN.step_signed_here, Role::Ok)
            } else if signed {
                (EN.step_verified, Role::Ok)
            } else if complete {
                (EN.step_not_needed, Role::OnSurfaceVariant)
            } else if listed(&s.failed) {
                (EN.step_does_not_verify, Role::Warning)
            } else if listed(&s.in_file) {
                (EN.step_in_file_unconfirmed, Role::OnSurfaceVariant)
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
        let copies: Vec<(usize, Text)> = app
            .inbox
            .iter()
            .enumerate()
            .filter(|(_, it)| it.name != *s.spend.source)
            .filter(|(_, it)| it.psbt().is_some_and(|p| p.txid == txid))
            .map(|(k, it)| (k, Text::of(&it.name)))
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
            // A MuSig2 round aggregated as a copy comes in may share a
            // nonce again (`docs/OWN-RANDOMNESS.md` §8.1 row 14).
            if !copies.is_empty() && crate::spend_opens_round(i) {
                cy += super::randomness::row(app, ui, (x, cy, w));
            }
            for (k, name) in &copies {
                // No wider than the column; a long name is cut.
                let label = Text::fill(EN.step_add_source, &[name]);
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
            if let Some(part) = s.out_names.part.as_deref() {
                cy += made_line(app, ui, x, cy, w, part);
            }
            let mut buttons = vec![(
                EN.step_scan_a_copy,
                Style::Secondary,
                Action::ScanFor(crate::Ask::SpendPsbt),
            )];
            // Held back, the part file gives out no code.
            if !s.spend.nonce_hold {
                buttons.insert(0, (EN.show_as_qr, Style::Secondary, Action::QrPart));
            }
            cy += wrap_buttons(ui, x, cy, w, BUTTON_H, &buttons);
        } else {
            self.next(ui, cy, EN.step_continue);
            cy += 44.0;
        }
        cy
    }

    /// The finish step: what is missing, or the results made.
    fn step_finish(&self, ui: &mut Ui, y: f32) -> f32 {
        let Self {
            app,
            x,
            w,
            needed,
            s,
            ..
        } = *self;
        let mut cy = y;
        if s.spend.finished.is_none() {
            let missing = needed.saturating_sub(s.signers.len()).max(1);
            let line = match s.spend.finish_reason(s.refused.as_deref()) {
                Some(r)
                    if s.spend.nonce_hold || s.refused.is_some() || s.signers.len() >= needed =>
                {
                    Text::of(&r)
                }
                _ => Text::fill(
                    EN.step_needs,
                    &[
                        &text!("{}", missing),
                        &(if s.signers.is_empty() {
                            Text::new()
                        } else {
                            text!(" {}", EN.step_more)
                        }),
                        (if missing == 1 {
                            EN.step_signature
                        } else {
                            EN.step_signatures
                        }),
                    ],
                ),
            };
            let tone = if s.spend.nonce_hold {
                Role::Error
            } else {
                Role::Warning
            };
            ui.wrap_as((x, cy, w), BODY_M, tone, &line);
            cy += 44.0;
            // The key and its inputs, as the nonce check names them.
            if s.spend.nonce_hold {
                cy += nonce_check(app, ui, &s.spend.psbt, x, cy, w);
            }
        } else if let Some((_, place)) = s.review_owed_shown() {
            // Review comes first, also for a PSBT signed elsewhere.
            let line = Text::fill(EN.primary_finish_step, &[&text!("{}", place)]);
            cy += ui.wrap_as((x, cy, w), BODY_M, Role::Warning, &line) + GAP;
            cy += nonce_check(app, ui, &s.spend.psbt, x, cy, w);
        } else {
            // Side by side where each card's head, its place and its
            // buttons fit its half; on a small panel, or where the
            // text size leaves them no room, one under the other.
            use faraday_draw::tokens::{BUTTON_PAD, FINISH_CARD_GAP, FINISH_CARD_INSET, GAP};
            let side = (w - FINISH_CARD_GAP) / 2.0 - 2.0 * FINISH_CARD_INSET;
            // Where each result went, under the name it went out as.
            let signed_file = s.out_names.signed.clone().unwrap_or_default();
            let tx_file = s.out_names.tx.clone().unwrap_or_default();
            let fits = |ui: &Ui, head: &str, file: &str| {
                let tag = made_place(app, file).map_or(0.0, |(t, _)| {
                    ui.measure_as(LABEL_M, &t) + faraday_draw::tokens::GAP_SMALL
                });
                ui.measure_as(TITLE_S, head) + tag <= side
            };
            let buttons = ui.measure_as(LABEL_L, EN.step_make_again)
                + ui.measure_as(LABEL_L, EN.show_as_qr)
                + 2.0 * BUTTON_PAD
                + 2.0 * BUTTON_PAD
                + GAP;
            let compact = ui.compact
                || buttons > side
                || !fits(ui, EN.step_signed_psbt, &signed_file)
                || !fits(ui, EN.step_finished_transaction, &tx_file);
            let half = if compact { w } else { (w - 12.0) / 2.0 };
            let mut card_y = cy;
            for (k, (head, sub, file, action)) in [
                (
                    EN.step_signed_psbt,
                    EN.step_for_the_wallet,
                    &signed_file,
                    Action::SignedToOutbox,
                ),
                (
                    EN.step_finished_transaction,
                    EN.step_ready_to_broadcast,
                    &tx_file,
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
                    Some((_, Some(i))) => (EN.step_remove, Style::Ghost, Action::OutboxRemove(*i)),
                    _ => (EN.step_make_again, Style::Secondary, *action),
                };
                // Where the buttons go: in a row when they fit.
                let lw = ui.measure_as(LABEL_L, label) + 32.0;
                let qw = ui.measure_as(LABEL_L, EN.show_as_qr) + 32.0;
                let stacked = lw + 8.0 + qw > half - 28.0;
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
                    s.out_names
                        .signed
                        .as_deref()
                        .map(Text::of)
                        .unwrap_or_else(|| {
                            text!(
                                "{}-signed.psbt",
                                crate::result_stem(&s.spend.source).as_str()
                            )
                        })
                } else {
                    let Some(tx) = s.spend.finished.as_ref() else {
                        continue;
                    };
                    Text::fill(
                        EN.step_vb,
                        &[
                            &short(&text!("{}", tx.compute_txid())),
                            &text!("{}", tx.vsize()),
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
                // A text button's label starts at the card's text.
                let right = if style == Style::Ghost {
                    bx + 14.0 + ui.text_button(bx + 14.0, by, label, style, action)
                } else {
                    bx + 14.0 + ui.button(bx + 14.0, by, Some(lw), BUTTON_H, label, style, action)
                };
                {
                    let (qx, qy) = if stacked {
                        (bx + 14.0, by + 44.0)
                    } else {
                        (right + 8.0, by)
                    };
                    ui.button(
                        qx,
                        qy,
                        Some(qw),
                        BUTTON_H,
                        EN.show_as_qr,
                        Style::Secondary,
                        if k == 0 {
                            Action::QrSigned
                        } else {
                            Action::QrTx
                        },
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
            cy += end_card(app, ui, (x, cy, w), app.spend_outbox_names().len());
        }
        cy
    }

    /// The MuSig2 nonces of each key and the ways to share them.
    fn step_nonces(&self, ui: &mut Ui, y: f32) -> f32 {
        let Self {
            app,
            x,
            w,
            wallet,
            s,
            ..
        } = *self;
        let mut cy = y;
        if let Some(wl) = wallet.and_then(|wi| app.session.wallets.get(wi)) {
            let rows = crate::wallet::musig_nonces(&app.session, wl, &s.spend.psbt);
            for (slot, fp, here, has) in &rows {
                ui.text_mid_as(
                    (x, cy, 40.0),
                    BODY_S,
                    Role::OnSurfaceVariant,
                    &text!("{}", slot),
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
            // Sharing a nonce and collecting draw the system's
            // randomness (`docs/OWN-RANDOMNESS.md` §8.1 rows 13, 14).
            cy += super::randomness::row(app, ui, (x, cy, w));
            let txid = s.spend.txid;
            let copies: Vec<(usize, Text)> = app
                .inbox
                .iter()
                .enumerate()
                .filter(|(_, it)| it.name != *s.spend.source)
                .filter(|(_, it)| it.psbt().is_some_and(|p| p.txid == txid))
                .map(|(k, it)| (k, Text::of(&it.name)))
                .collect();
            let labels: Vec<Text> = copies
                .iter()
                .map(|(_, name)| Text::fill(EN.step_add_source, &[name]))
                .collect();
            let items: Vec<(&str, Style, Action)> = labels
                .iter()
                .zip(&copies)
                .map(|(l, (k, _))| (l.as_str(), Style::Secondary, Action::Collect(*k)))
                .collect();
            if !items.is_empty() {
                cy += wrap_buttons(ui, x, cy, w, BUTTON_H, &items) + 2.0;
            }
            if let Some(part) = s.out_names.part.as_deref() {
                cy += made_line(app, ui, x, cy, w, part);
            }
            let others_in = rows.iter().filter(|r| !r.2).all(|r| r.3);
            let mine_in = rows.iter().filter(|r| r.2).all(|r| r.3);
            if !s.spend.nonce_hold {
                cy += wrap_buttons(
                    ui,
                    x,
                    cy,
                    w,
                    BUTTON_H,
                    &[(EN.show_as_qr, Style::Secondary, Action::QrPart)],
                ) + 2.0;
            }
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
            self.next(ui, cy, EN.step_continue);
            cy += 48.0;
        }
        cy
    }

    /// The spending paths of a wallet that has them.
    fn step_paths(&self, ui: &mut Ui, y: f32) -> f32 {
        let Self {
            app,
            x,
            w,
            wallet,
            s,
            ..
        } = *self;
        let mut cy = y;
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
                    &Text::fill(EN.step_path, &[&text!("{}", (k + 1))]),
                );
                // The keys, wrapping; on a small panel under the name.
                let (left, mut ky) = if ui.compact {
                    (x + 14.0, cy + 36.0)
                } else {
                    (x + 90.0, cy + 8.0)
                };
                let mut kx = left;
                for (slot, fp, here) in &p.keys {
                    let t = Text::fill(EN.step_key, &[&text!("{}", slot), fp]);
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
                        &Text::fill(EN.step_waits, &[l]),
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
        self.next(ui, cy, EN.step_continue);
        cy += 48.0;
        cy
    }
}

/// One line of the review: its label, the amount, the rest of the line,
/// and a note with its colour.
type ReviewRow = (Text, Text, Text, Option<(&'static str, crate::ui::Paint)>);

/// The transaction's review under Your choices' rows: "This transaction",
/// its six facts as plain lines, and the signatures box with its action.
/// Returns its height.
fn review_under(
    app: &View,
    ui: &mut Ui,
    (x, y0, w): (f32, f32, f32),
    wallet: Option<usize>,
    needed: usize,
) -> f32 {
    let s = app.wallets.spend().expect("spend");
    let i = &s.inspection;
    let mut y = y0 + faraday_draw::tokens::GAP;
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
        .unwrap_or_else(|| Text::of("·"));
    // Change is verified change only: an output that claims to be change
    // and cannot be verified is among what it sends (F136).
    // It names the wallet the change goes to (F254).
    let own = i
        .outputs
        .iter()
        .enumerate()
        .filter(|(_, o)| o.is_ours())
        .max_by_key(|(_, o)| {
            matches!(
                o.kind,
                OutputKind::Change { change: true, .. }
                    | OutputKind::WalletChange { change: true, .. }
            )
        });
    let change = match own {
        Some((k, _)) => match s.own_names.get(k) {
            Some(Some(n)) => n.clone(),
            _ => Text::of(EN.spend_panel_verified),
        },
        None => Text::of(EN.spend_panel_none),
    };
    let facts: [(&str, Text, TypeRole); 6] = [
        (
            EN.spend_panel_wallet,
            wallet
                .and_then(|wi| app.session.wallets.get(wi))
                .map(|wl| Text::of(&wl.name))
                .unwrap_or_else(|| Text::of(EN.spend_panel_not_loaded)),
            BODY_M,
        ),
        (
            EN.spend_panel_sends,
            Text::fill(EN.spend_panel_btc, &[&btc(i.amount_to_others.to_sat())]),
            BODY_M.mono(),
        ),
        (EN.spend_panel_to_label, to, BODY_M.mono()),
        (
            EN.spend_panel_fee,
            match i.fee_rate_sat_vb {
                Some(rate) => Text::fill(
                    EN.spend_panel_sat_sat,
                    &[&thousands(i.fee.to_sat()), &text!("{rate:.1}")],
                ),
                None => Text::of(EN.step_fee_not_known),
            },
            BODY_M,
        ),
        (EN.spend_panel_change, change, BODY_M),
        (
            EN.spend_panel_txid,
            short(&text!("{}", s.spend.txid)),
            BODY_M.mono(),
        ),
    ];
    for (k, v, wt) in facts.iter() {
        // Plain lines: Your choices' rows are the way to a step.
        let row = ui.line_as(BODY_M) + 2.0 * faraday_draw::tokens::GAP;
        ui.text_mid_as((x, y, row), BODY_S, Role::OnSurfaceVariant, k);
        let v = ui.fit_as(*wt, v, w - 70.0);
        ui.text_right_as((x + w, y, row), *wt, Role::OnSurface, &v);
        ui.rule(x, y + row, w, Role::OutlineVariant);
        y += row + 2.0;
    }
    let (count, missing, need_line, action_label, style) = spend_status(app, needed);
    y += faraday_draw::tokens::GAP;
    let fy = y;
    ui.fill(x, fy, w, 82.0, 12.0, Role::SurfaceContainerHighest);
    ui.stroke(x, fy, w, 82.0, 12.0, Role::OutlineVariant);
    ui.text_mid_as(
        (x + 16.0, fy + 10.0, 26.0),
        BODY_S,
        Role::OnSurfaceVariant,
        EN.spend_panel_signatures,
    );
    let label = Text::fill(
        EN.spend_panel_count,
        &[&text!("{}", count), &text!("{}", needed)],
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
    let by = fy + 82.0 + 14.0;
    let label = action_label.fitted(ui, TITLE_M, w - 32.0);
    let bh = ui.button_height(BUTTON_H);
    ui.button(x, by, Some(w), BUTTON_H, &label, style, Action::Primary);
    by + bh - y0
}

/// Who signs' count: enough, the count and
/// a green Can spend beside it; short, the count and how many more are
/// needed, in `warning`. Returns the y its step's Continue goes at: beside
/// it, or under it where both do not fit.
pub(crate) fn signers_count(
    ui: &mut Ui,
    x: f32,
    y: f32,
    w: f32,
    here: usize,
    in_the_file: usize,
    needed: usize,
) -> f32 {
    let enough = here >= needed;
    let line = if !enough && in_the_file > 0 {
        // With partial signatures this device cannot confirm, the count is
        // of what the file holds rather than of the keys here.
        let k = in_the_file;
        Text::fill(
            EN.signers_count_in_the_file,
            &[
                &text!("{}", k),
                &text!("{}", needed),
                &text!("{}", needed.saturating_sub(k)),
            ],
        )
    } else if enough {
        Text::fill(
            EN.signers_count_of_signatures_here,
            &[&text!("{}", here), &text!("{}", needed)],
        )
    } else {
        Text::fill(
            EN.signers_count_of_signatures,
            &[
                &text!("{}", here),
                &text!("{}", needed),
                &text!("{}", (needed - here)),
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
    let facts = Text::fill(
        EN.step_fee_hold_facts,
        &[
            &thousands(i.fee.to_sat()),
            &i.fee_rate_sat_vb
                .map(|r| text!("{r:.1}"))
                .unwrap_or_default(),
            &text!("{}", i.fee_share_pct().unwrap_or_default()),
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
    cy += ui.hold_button_height(w, EN.step_fee_hold).max(CHIP_LINE) + GAP;
    cy - y
}

/// The Sign row in a column `w` wide at (`x`, `y`): `lead` at its start
/// and the Sign button, labelled `label`, at its end. Where the two do
/// not fit side by side the button goes under the line, and where its
/// label is wider than the column the label is a line of its own above a
/// button reading Sign. Returns its height.
/// One warning line of the review, its icon before it; gives the height
/// it took with the space after it.
fn warning_row(ui: &mut Ui, (x, y, w): (f32, f32, f32), tone: Role, text: &str) -> f32 {
    let used = ui.wrap_as((x + 22.0, y, w - 22.0), BODY_M, tone, text);
    ui.icon(x, y, 16.0, Icon::Warning, 10.0, tone);
    used.max(18.0) + 6.0
}

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

/// Whether signing leaves the transaction's id as it is: it does for a
/// segwit or taproot input, not for a legacy one, whose id covers the
/// signature. A lookup.
fn unchanged_by_signing(kind: faraday_psbt::ScriptKind) -> bool {
    use faraday_psbt::{ScriptKind as K, Wrapper};
    match kind {
        K::P2pkh
        | K::Multisig {
            wrapper: Wrapper::P2sh,
            ..
        }
        | K::Miniscript {
            wrapper: Wrapper::P2sh,
        } => false,
        K::P2shP2wpkh
        | K::P2wpkh
        | K::P2trKey
        | K::P2trScript
        | K::Multisig {
            wrapper: Wrapper::P2wsh | Wrapper::P2shP2wsh,
            ..
        }
        | K::Miniscript {
            wrapper: Wrapper::P2wsh | Wrapper::P2shP2wsh,
        }
        | K::Unknown => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wallet::NonceRow;
    use faraday_psbt::verify::{Unchecked, Verdict};

    fn row(preimage: bool) -> NonceRow {
        NonceRow {
            input: 0,
            name: Text::of("k"),
            verdict: Verdict::Unchecked(Unchecked::UnsupportedScript),
            preimage,
            recomputed: None,
            unmatched: None,
            entry: None,
        }
    }

    #[test]
    fn a_preimage_row_is_not_called_an_unchecked_signature_and_another_reads_as_before() {
        assert_eq!(row_verdict(&row(true)).0, EN.nonce_check_preimage);
        assert_eq!(
            row_verdict(&row(false)).0,
            Text::fill(EN.nonce_check_not_checked, &[EN.nonce_unchecked_script])
        );
    }

    #[test]
    fn the_review_says_for_each_script_type_whether_signing_changes_the_transaction_id() {
        use faraday_psbt::{ScriptKind as K, Wrapper};
        let unchanged = |k| {
            if unchanged_by_signing(k) {
                EN.step_unchanged_by_signing
            } else {
                EN.step_changes_when_signed
            }
        };
        let legacy = [
            K::P2pkh,
            K::Multisig {
                wrapper: Wrapper::P2sh,
                m: 2,
                n: 3,
            },
            K::Miniscript {
                wrapper: Wrapper::P2sh,
            },
        ];
        let segwit = [
            K::P2shP2wpkh,
            K::P2wpkh,
            K::P2trKey,
            K::P2trScript,
            K::Multisig {
                wrapper: Wrapper::P2wsh,
                m: 2,
                n: 3,
            },
            K::Multisig {
                wrapper: Wrapper::P2shP2wsh,
                m: 2,
                n: 3,
            },
            K::Miniscript {
                wrapper: Wrapper::P2wsh,
            },
            K::Miniscript {
                wrapper: Wrapper::P2shP2wsh,
            },
            K::Unknown,
        ];
        for k in legacy {
            assert_eq!(unchanged(k), EN.step_changes_when_signed, "{k:?}");
        }
        for k in segwit {
            assert_eq!(unchanged(k), EN.step_unchanged_by_signing, "{k:?}");
        }
    }
}
