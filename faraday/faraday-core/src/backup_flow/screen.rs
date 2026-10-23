//! The backup flow's cards (`docs/BACKUP-FLOW.md` §4, §5): step cards on a
//! wide panel, pages with "Step 2 of 7" on a small one. Drawing only; what
//! a press does is `backup_flow/steps.rs`. The Create a vault form is
//! `vault::screen::create`, drawn in the flow's place while a vault is
//! made.

use crate::backup_flow::steps::FlowAction as F;
use crate::backup_flow::{BackupFlow, Confirm, Step};
use crate::drawing::{Scrolled, View};
use crate::strings::{EN as FEN, fill};
use crate::ui::pal::*;
use crate::ui::{Style, Ui, W};
use crate::vault::screen::{hold_button_in, stick_banner, stick_field, text_box};
use crate::vault::{self, Focus, VaultAction as V};
use crate::{Action, Faraday, flow};
use faraday_draw::widgets::Icon;

/// A button, and the room it takes with the gap under it.
#[allow(clippy::too_many_arguments)]
fn btn(
    ui: &mut Ui,
    x: f32,
    y: f32,
    w: Option<f32>,
    h: f32,
    label: &str,
    style: Style,
    action: Action,
    room: f32,
) -> f32 {
    ui.button(x, y, w, h, label, style, action);
    room
}

fn fa(a: F) -> Action {
    Action::Flow(a)
}

/// The flow's screen.
pub(crate) fn draw(app: &View, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    let Some(f) = app.flow() else {
        return;
    };
    // A vault made in the flow: Create a vault's own form, in the flow's
    // place, never another screen.
    if f.live.creating.is_some() && app.vaults.create.is_some() {
        return crate::vault::screen::create(app, ui, x0, cw, h);
    }
    let steps = steps_of(f);
    let cards: Vec<flow::Card> = steps.iter().map(|&s| card(app, f, s)).collect();
    // While the plan card's board is shown on `wide`, its list is a
    // right-hand column, as on Back up (`docs/BACKUP-FLOW.md` §17.14).
    let panel_w = if !ui.compact && app.board_shown() {
        crate::screens::side_panel_w(cw)
    } else {
        0.0
    };
    let col = flow::Column {
        area_x: x0,
        area_w: cw - panel_w,
        x: x0 + 40.0,
        w: if panel_w > 0.0 {
            cw - panel_w - 72.0
        } else {
            (cw - 72.0).min(820.0)
        },
        h,
        back: Some(app.named_back((FEN.backup_flow_leave, fa(F::Back)))),
        heading: f.kind.name(),
        guided: false,
        switch: false,
        note: None,
        chip: None,
        chip_tap: None,
        cancel: app.cancel_offered().is_some(),
    };
    let scroll = f.live.scroll;
    let (next, again) = {
        let app_ref: &View = app;
        let f = app_ref.flow().unwrap();
        let mut body = |ui: &mut Ui, i: usize, x: f32, y: f32, w: f32| -> f32 {
            body(app_ref, f, ui, steps[i], x, y, w)
        };
        flow::column(ui, &col, &cards, scroll, &mut body)
    };
    app.scroll(Scrolled::BackupFlow, next, again);
    if panel_w > 0.0 {
        crate::board::screen::list_panel(app, ui, x0 + cw - panel_w, panel_w, h);
    }
}

/// The cards the flow draws: a card a plan skips is not drawn and not
/// counted (§5). A plan with no stick has no stick card, no lock and no
/// write.
fn steps_of(f: &BackupFlow) -> Vec<Step> {
    // The setup of a wallet's vaults, before its keys, is two cards: the
    // sticks and the vaults.
    if f.is_setup() {
        return vec![Step::Sticks, Step::Vaults];
    }
    Step::ALL
        .iter()
        .copied()
        .filter(|s| !f.slots.is_empty() || !matches!(s, Step::Sticks | Step::Lock | Step::Write))
        .filter(|s| !(*s == Step::Vaults && f.plan.used_vaults().is_empty()))
        .collect()
}

pub(crate) fn card_title(app: &Faraday, f: &BackupFlow, s: Step) -> String {
    let m = app.medium.nouns();
    match s {
        Step::Where if f.kind.is_wallet() => FEN.backup_flow_card_plan.to_string(),
        Step::Where => FEN.backup_flow_card_where.to_string(),
        Step::Sticks => fill(FEN.backup_flow_card_sticks, &[m]),
        Step::Vaults => FEN.backup_flow_card_vaults.to_string(),
        Step::Make if f.kind.is_wallet() => FEN.backup_flow_card_make_wallet.to_string(),
        Step::Make => FEN.backup_flow_card_make.to_string(),
        Step::Lock => FEN.backup_flow_card_lock.to_string(),
        Step::Write => fill(FEN.backup_flow_card_write, &[m]),
        Step::Done => FEN.backup_flow_card_done.to_string(),
    }
}

fn card(app: &View, f: &BackupFlow, s: Step) -> flow::Card {
    let open = f.live.open == Some(s.n());
    let (n, m) = f.written_of();
    let summary = match s {
        Step::Where => fill(
            FEN.backup_flow_places_vaults,
            &[
                &f.plan.places.len().to_string(),
                &f.plan.used_vaults().len().to_string(),
            ],
        ),
        Step::Sticks => {
            let bound = f.slots.iter().filter(|x| x.bound.is_some()).count();
            fill(
                FEN.backup_flow_read_count,
                &[&bound.to_string(), &m.to_string()],
            )
        }
        Step::Vaults if f.step > Step::Vaults => fill(
            FEN.backup_flow_vaults_n,
            &[&f.plan.used_vaults().len().to_string()],
        ),
        Step::Vaults => {
            let open_n = f
                .plan
                .used_vaults()
                .iter()
                .filter(|&&v| {
                    f.plan.vaults[v]
                        .salt
                        .is_some_and(|s| app.vaults.open.iter().any(|o| o.header().salt == s))
                })
                .count();
            fill(
                FEN.backup_flow_unlocked_count,
                &[&open_n.to_string(), &f.plan.used_vaults().len().to_string()],
            )
        }
        Step::Make => match &f.subject {
            Some(s) => s.name.clone(),
            None => FEN.backup_flow_not_made.to_string(),
        },
        Step::Lock => {
            if f.step > Step::Lock {
                FEN.backup_flow_locked.to_string()
            } else {
                FEN.backup_flow_not_locked.to_string()
            }
        }
        Step::Write => fill(
            FEN.backup_flow_written_count,
            &[&n.to_string(), &m.to_string()],
        ),
        Step::Done => {
            if f.all_written() {
                FEN.backup_flow_done.to_string()
            } else {
                String::new()
            }
        }
    };
    flow::Card {
        title: card_title(app, f, s),
        summary,
        mono: false,
        done: s < f.step || (s == Step::Done && f.step == Step::Done),
        open,
        default: false,
        toggle: fa(F::Open(s.n())),
        guide: None,
    }
}

fn body(app: &View, f: &BackupFlow, ui: &mut Ui, s: Step, x: f32, y: f32, w: f32) -> f32 {
    let mut cy = y;
    cy += match s {
        Step::Where => where_body(app, f, ui, x, cy, w),
        Step::Sticks => sticks_body(app, f, ui, x, cy, w),
        Step::Vaults => vaults_body(app, f, ui, x, cy, w),
        Step::Make => make_body(app, f, ui, x, cy, w),
        Step::Lock => lock_body(app, f, ui, x, cy, w),
        Step::Write => write_body(app, f, ui, x, cy, w),
        Step::Done => done_body(app, f, ui, x, cy, w),
    };
    // Change the plan is under every card but the first (§4.8).
    if s != Step::Where
        && s != Step::Done
        && f.step == s
        && f.live.writing.is_none()
        && !f.is_setup()
    {
        cy += 8.0;
        cy += btn(
            ui,
            x,
            cy,
            None,
            32.0,
            FEN.backup_flow_change_plan,
            Style::Ghost,
            fa(F::ChangePlan),
            40.0,
        );
    }
    cy - y
}

// ---- 1: where it will be kept ----

#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
fn where_body(app: &View, f: &BackupFlow, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    if f.kind.is_wallet() {
        return crate::backup_flow::wallet_screen::where_body(app, f, ui, x, y, w);
    }
    let mut cy = y;
    let min = f.kind.min_places();
    let sticks = f.plan.stick_places().len();
    for (p, vs) in f.plan.places.iter().enumerate() {
        let written = f.slots_at(p).iter().any(|&k| f.slots[k].written.is_some());
        let keeps = if vs.is_empty() {
            String::new()
        } else {
            let names: Vec<String> = vs.iter().map(|&v| BackupFlow::vault_name(v)).collect();
            format!(
                "{} {}",
                fill(FEN.backup_flow_place_keeps, &[app.medium.noun()]),
                names.join(", ")
            )
        };
        ui.text(x, cy, 14.0, W::S, TEXT, &BackupFlow::place_name(p));
        ui.text(x + 90.0, cy, 14.0, W::R, MUTED, &keeps);
        cy += 26.0;
        let mut items: Vec<(String, Style, Action)> = Vec::new();
        if written {
            if sticks > min {
                items.push((
                    FEN.backup_flow_destroyed.to_string(),
                    Style::Secondary,
                    fa(F::Destroyed(p)),
                ));
            }
        } else {
            for v in 0..f.plan.vaults.len() {
                let on = vs.contains(&v);
                items.push((
                    BackupFlow::vault_name(v),
                    if on { Style::Primary } else { Style::Secondary },
                    fa(F::PlaceVault(p, v)),
                ));
            }
            items.push((
                FEN.backup_flow_new_vault.to_string(),
                Style::Secondary,
                fa(F::PlaceVault(p, f.plan.vaults.len())),
            ));
            // A place may keep a stick for each vault, or one stick for both.
            for v in 0..f.plan.vaults.len() {
                if !vs.contains(&v) && !vs.is_empty() {
                    items.push((
                        fill(FEN.backup_flow_also, &[&BackupFlow::vault_name(v)]),
                        Style::Ghost,
                        fa(F::AlsoVault(p, v)),
                    ));
                }
            }
            if vs.len() > 1 {
                items.push((
                    fill(FEN.backup_flow_both, &[app.medium.noun()]),
                    if f.plan.together.contains(&p) {
                        Style::Primary
                    } else {
                        Style::Secondary
                    },
                    fa(F::Together(p)),
                ));
            }
            if !vs.is_empty() {
                items.push((
                    FEN.backup_flow_public_here.to_string(),
                    if f.plan.public_at == p {
                        Style::Primary
                    } else {
                        Style::Ghost
                    },
                    fa(F::PublicAt(p)),
                ));
            }
            if sticks > min {
                items.push((
                    FEN.backup_flow_remove.to_string(),
                    Style::Ghost,
                    fa(F::RemovePlace(p)),
                ));
            }
        }
        let refs: Vec<(&str, Style, Action)> =
            items.iter().map(|(l, s, a)| (l.as_str(), *s, *a)).collect();
        cy += crate::screens::wrap_buttons(ui, x, cy, w, 32.0, &refs) + 6.0;
    }
    cy += btn(
        ui,
        x,
        cy,
        None,
        32.0,
        FEN.backup_flow_add_place,
        Style::Secondary,
        fa(F::AddPlace),
        44.0,
    );
    ui.rule(x, cy, w, INNER);
    cy += 14.0;
    for v in f.plan.used_vaults() {
        let vp = &f.plan.vaults[v];
        let name = BackupFlow::vault_name(v);
        let line = fill(
            if vp.existing {
                FEN.backup_flow_vault_existing
            } else {
                FEN.backup_flow_vault_new
            },
            &[&name],
        );
        let places: Vec<String> = f
            .plan
            .places_of(v)
            .iter()
            .map(|&p| BackupFlow::place_name(p))
            .collect();
        ui.text(x, cy, 14.0, W::S, TEXT, &line);
        ui.text(x, cy + 22.0, 12.0, W::R, MUTED, &places.join(", "));
        if vp.salt.is_none() {
            let other = if vp.existing {
                FEN.backup_flow_new_vault
            } else {
                FEN.backup_flow_existing_vault
            };
            let bw = ui.measure(13.0, W::S, other) + 32.0;
            ui.button(
                x + w - bw,
                cy,
                Some(bw),
                32.0,
                other,
                Style::Secondary,
                fa(F::VaultExisting(v)),
            );
        }
        cy += 50.0;
    }
    if sticks < min {
        cy += ui.wrap(
            x,
            cy,
            w,
            13.0,
            W::R,
            WARN,
            &fill(FEN.backup_flow_at_least, &[&min.to_string()]),
        ) + 10.0;
    }
    let style = if sticks >= min {
        Style::Primary
    } else {
        Style::Disabled
    };
    ui.button(
        x,
        cy,
        None,
        40.0,
        FEN.backup_flow_continue_make,
        style,
        fa(F::Continue),
    );
    cy + 52.0 - y
}

// ---- helpers ----

fn line(ui: &mut Ui, x: f32, y: f32, w: f32, text: &str, color: faraday_draw::Color) -> f32 {
    ui.wrap(x, y, w, 14.0, W::R, color, text) + 10.0
}

/// The medium's icon and a line centred, as Insert is.
fn insert(app: &View, ui: &mut Ui, x: f32, y: f32, w: f32, text: &str) -> f32 {
    ui.icon(
        x + (w - 44.0) / 2.0,
        y,
        44.0,
        app.medium.icon(),
        22.0,
        MUTED,
    );
    let tw = ui.measure(14.0, W::S, text).min(w);
    let t = ui.fit(14.0, W::S, text, w);
    ui.text(x + (w - tw) / 2.0, y + 54.0, 14.0, W::S, TEXT, &t);
    84.0
}

/// A name that starts a line: "the stick for Vault 1" is "The stick for
/// Vault 1".
fn cap(s: &str) -> String {
    let mut c = s.chars();
    c.next()
        .map(|f| f.to_uppercase().collect::<String>() + c.as_str())
        .unwrap_or_default()
}

fn read_time(app: &View, at: Option<u64>) -> String {
    at.map(|t| vault::time_of_day(vault::local_secs(t, app.tz_min())))
        .unwrap_or_default()
}

fn slot_rows(
    app: &View,
    f: &BackupFlow,
    ui: &mut Ui,
    x: f32,
    y: f32,
    w: f32,
    writing: bool,
) -> f32 {
    let mut cy = y;
    let m = app.medium.noun();
    for (k, s) in f.slots.iter().enumerate() {
        let done = if writing {
            s.written.is_some()
        } else {
            s.bound.is_some()
        };
        let tail = if writing {
            s.written.as_ref().map(|w| w.label.clone())
        } else {
            s.bound.as_ref().map(|b| b.label.clone())
        };
        let room = match &tail {
            Some(t) => w - ui.measure(13.0, W::M, t) - 12.0,
            None => w,
        };
        let name = app
            .flow_slot_row(k)
            .unwrap_or_else(|| cap(&f.target_name(&s.target, m)));
        let name = ui.fit(13.0, W::R, &name, room);
        ui.text(x, cy, 13.0, W::R, if done { TEXT } else { MUTED }, &name);
        if let Some(t) = tail {
            ui.text_right(x + w, cy - 4.0, 22.0, 13.0, W::M, OK, &t);
        }
        cy += 24.0;
        // The stick just taken for this place, while it is in: the
        // number of its pill, to mark on it (§18).
        if let Some(mark) = app.flow_mark_line(k) {
            cy += line(ui, x, cy - 4.0, w, &mark, ACCENT) - 4.0;
        }
    }
    cy - y + 6.0
}

// ---- 2: bring in the sticks ----

fn sticks_body(app: &View, f: &BackupFlow, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    let mut cy = y;
    if !app.clean_now() {
        cy += btn(
            ui,
            x,
            cy,
            None,
            40.0,
            &fill(FEN.backup_flow_lock_bring, &[app.medium.nouns()]),
            Style::Primary,
            fa(F::LockBring),
            52.0,
        );
        return cy - y;
    }
    cy += slot_rows(app, f, ui, x, cy, w, false);
    cy += stick_card(app, f, ui, x, cy, w);
    cy - y
}

/// The card for the stick being asked for or read, at step 2 or 6.
#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
fn stick_card(app: &View, f: &BackupFlow, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    let mut cy = y;
    let m = app.medium.noun();
    // The stick the flow wants now, as the pills edge it (§18).
    let wanted = app.flow_wanted_slot();
    let Some(r) = f.live.reading.as_ref() else {
        if f.step == Step::Write
            && let Some(p) = &f.live.problem
        {
            cy += line(ui, x, cy, w, &app.flow_problem_text(p), ERR);
        }
        let Some(k) = wanted else {
            return cy - y;
        };
        if f.live.crowd {
            cy += line(
                ui,
                x,
                cy,
                w,
                &fill(FEN.backup_flow_one_at_a_time, &[m]),
                WARN,
            );
            return cy - y;
        }
        cy += insert(app, ui, x, cy, w, &app.flow_insert_line(k));
        // The stick the machine started from, read already; not once it
        // holds another place: one stick, one place.
        let boot_placed = (0..f.slots.len()).any(|j| j != k && f.boot_holds(j));
        if matches!(f.step, Step::Sticks | Step::Write) && !boot_placed {
            for (i, st) in app.sticks.iter().enumerate().filter(|(_, s)| s.boot) {
                let at = read_time(app, app.clock_now());
                cy += btn(
                    ui,
                    x,
                    cy,
                    None,
                    36.0,
                    &fill(FEN.backup_flow_use_read, &[&st.label, &at]),
                    Style::Secondary,
                    fa(F::UseBoot(i)),
                    44.0,
                );
            }
        }
        if f.step == Step::Write {
            cy += files_rows(app, f, ui, x, cy, w, k);
        }
        return cy - y;
    };
    if !r.bound {
        cy += ui.wrap(x, cy, w, 14.0, W::S, TEXT, &app.flow_insert_line(r.slot)) + 8.0;
    }
    if let Some(p) = &r.problem {
        cy += line(ui, x, cy, w, &app.flow_problem_text(p), ERR);
        // Which of the backup's sticks it is, if any (§18).
        if let Some(which) = app.flow_stick_from_elsewhere() {
            cy += line(ui, x, cy, w, &which, TEXT);
        }
        cy += btn(
            ui,
            x,
            cy,
            None,
            36.0,
            &fill(FEN.backup_flow_use_another, &[m]),
            Style::Secondary,
            fa(F::UseAnother),
            44.0,
        );
        return cy - y;
    }
    if !r.pending.is_empty() || r.space_pending {
        cy += line(
            ui,
            x,
            cy,
            w,
            &fill(FEN.backup_flow_reading, &[&r.label]),
            MUTED,
        );
        return cy - y;
    }
    if r.bound {
        let done = f.slots[r.slot].written.is_some();
        if done {
            cy += line(
                ui,
                x,
                cy,
                w,
                &fill(FEN.backup_flow_written, &[&r.label]),
                OK,
            );
        }
        cy += insert(app, ui, x, cy, w, &fill(FEN.backup_flow_pull, &[m]));
        return cy - y;
    }
    if let Some(wr) = &f.live.writing {
        let file = wr
            .pending
            .first()
            .map(|(n, _)| n.clone())
            .unwrap_or_default();
        cy += line(
            ui,
            x,
            cy,
            w,
            &fill(FEN.backup_flow_writing, &[&file]),
            ACCENT,
        );
        return cy - y;
    }
    match r.purpose {
        crate::backup_flow::Purpose::Bind => {
            // A blank stick read alike: which of the backup's sticks it
            // reads as, above the hold (§18).
            if let Some(like) = app.flow_stick_reads_as() {
                cy += line(ui, x, cy, w, &like, TEXT);
            }
            cy += bind_card(app, f, r, ui, x, cy, w);
        }
        crate::backup_flow::Purpose::Check => {
            if r.hold {
                cy += files_rows(app, f, ui, x, cy, w, r.slot);
                let later = f
                    .vaults_of(r.slot)
                    .iter()
                    .map(|&v| f.copy_number(v, r.slot))
                    .max()
                    .unwrap_or(0);
                let label = if later >= 2 {
                    fill(FEN.backup_flow_hold_another, &[m])
                } else {
                    fill(FEN.backup_flow_hold_second, &[m])
                };
                cy += hold_row(
                    app,
                    ui,
                    x,
                    cy,
                    w,
                    &label,
                    &fill(FEN.backup_flow_use_another, &[m]),
                );
            } else {
                cy += line(
                    ui,
                    x,
                    cy,
                    w,
                    &fill(FEN.backup_flow_reading, &[&r.label]),
                    MUTED,
                );
            }
        }
    }
    cy - y
}

/// The hold, and **Use another stick** beside it, or under it where the row
/// is too narrow. Returns the height.
fn hold_row(app: &View, ui: &mut Ui, x: f32, y: f32, w: f32, label: &str, other: &str) -> f32 {
    let bw = hold_button_in(app, ui, x, y, w, label, fa(F::Hold), ACCENT);
    let ow = ui.measure(14.0, W::S, other) + 32.0;
    if bw + 10.0 + ow <= w {
        ui.button(
            x + bw + 10.0,
            y,
            None,
            36.0,
            other,
            Style::Ghost,
            fa(F::UseAnother),
        );
        48.0
    } else {
        ui.button(
            x,
            y + 44.0,
            None,
            36.0,
            other,
            Style::Ghost,
            fa(F::UseAnother),
        );
        92.0
    }
}

/// The files a stick will hold: a row each.
fn files_rows(app: &View, f: &BackupFlow, ui: &mut Ui, x: f32, y: f32, w: f32, slot: usize) -> f32 {
    let _ = app;
    let mut cy = y;
    for v in f.vaults_of(slot) {
        if let Some(s) = f.sealed_of(v) {
            let line = format!("{} · {}", s.name, vault::file_text(s.len));
            cy += ui.wrap(x, cy, w, 13.0, W::M, MUTED, &line) + 6.0;
        }
    }
    for n in &f.slots[slot].files {
        if let Some(it) = app.outbox.iter().find(|i| &i.name == n) {
            let line = format!("{} · {}", n, vault::file_text(it.bytes.len()));
            cy += ui.wrap(x, cy, w, 13.0, W::M, MUTED, &line) + 6.0;
        }
    }
    cy - y + 6.0
}

/// A stick read: which vault, which place.
#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
fn bind_card(
    app: &View,
    f: &BackupFlow,
    r: &crate::backup_flow::Reading,
    ui: &mut Ui,
    x: f32,
    y: f32,
    w: f32,
) -> f32 {
    let mut cy = y;
    let m = app.medium.noun();
    // A vault not decided yet: the stick's vault files are the choice.
    let undecided = f
        .vaults_of(r.slot)
        .into_iter()
        .find(|v| !r.picks.iter().any(|(x, _)| x == v));
    if let Some((v, name)) = &r.differs {
        // The same vault, other bytes: which copy to unlock.
        let salt = r
            .files
            .iter()
            .find(|(x, _)| &x.name == name)
            .map(|(x, _)| x.salt);
        let first = salt
            .map(|s| app.flow_files_copy_label(&s))
            .unwrap_or_default();
        cy += line(
            ui,
            x,
            cy,
            w,
            &fill(
                FEN.backup_flow_copies_differ,
                &[&r.label, &BackupFlow::vault_name(*v), &first],
            ),
            WARN,
        );
        let items = [
            (
                fill(FEN.backup_flow_use_copy, &[&r.label]),
                Style::Primary,
                fa(F::UseCopy(true)),
            ),
            (
                fill(FEN.backup_flow_use_copy, &[&first]),
                Style::Secondary,
                fa(F::UseCopy(false)),
            ),
        ];
        let refs: Vec<(&str, Style, Action)> =
            items.iter().map(|(l, s, a)| (l.as_str(), *s, *a)).collect();
        cy += crate::screens::wrap_buttons(ui, x, cy, w, 36.0, &refs) + 6.0;
        return cy - y;
    }
    if let Some(v) = undecided {
        let vp = &f.plan.vaults[v];
        let title = fill(FEN.backup_flow_which_vault, &[&BackupFlow::vault_name(v)]);
        cy += ui.wrap(x, cy, w, 14.0, W::S, TEXT, &title) + 8.0;
        if !vp.existing {
            cy += btn(
                ui,
                x,
                cy,
                Some(w),
                40.0,
                FEN.backup_flow_beside,
                Style::Primary,
                fa(F::PickBeside(v)),
                48.0,
            );
        }
        for (k, (vr, _)) in r.files.iter().enumerate() {
            let seen = app.vault_summary_for_salt(&vr.salt).unwrap_or_default();
            let line = format!(
                "{} · {}{}",
                vr.name,
                vault::file_text(vr.len),
                if seen.is_empty() {
                    String::new()
                } else {
                    format!(" · {seen}")
                }
            );
            ui.fill(x, cy, w, 44.0, 10.0, SURFACE);
            ui.stroke(x, cy, w, 44.0, 10.0, BORDER);
            let t = ui.fit(13.0, W::M, &line, w - 24.0);
            ui.text_mid(x + 12.0, cy, 44.0, 13.0, W::M, TEXT, &t);
            ui.hit(x, cy, w, 44.0, fa(F::PickFile(v, k)));
            cy += 52.0;
        }
        return cy - y;
    }
    // Which place: a tap, or a hold where two sticks read alike.
    // On a small panel the name is the short one: "Vault 2 (Place 3)".
    let target = if ui.compact {
        f.target_short(&f.slots[r.slot].target, m)
    } else {
        f.target_name(&f.slots[r.slot].target, m)
    };
    let both = f.vaults_of(r.slot).len() > 1;
    match app.flow_confirm_kind() {
        Confirm::Tap => {
            let label = if both {
                fill(FEN.backup_flow_both, &[m])
            } else {
                fill(FEN.backup_flow_confirm_place, &[&target])
            };
            cy += btn(
                ui,
                x,
                cy,
                None,
                40.0,
                &label,
                Style::Primary,
                fa(F::Tap),
                52.0,
            );
        }
        Confirm::Hold => {
            let label = fill(FEN.backup_flow_hold_confirm_place, &[&target]);
            cy += hold_row(
                app,
                ui,
                x,
                cy,
                w,
                &label,
                &fill(FEN.backup_flow_use_another, &[m]),
            );
        }
    }
    if matches!(app.flow_confirm_kind(), Confirm::Tap) {
        cy += btn(
            ui,
            x,
            cy - 8.0,
            None,
            32.0,
            &fill(FEN.backup_flow_use_another, &[m]),
            Style::Ghost,
            fa(F::UseAnother),
            36.0,
        );
    }
    cy - y
}

// ---- 3: the vaults ----

#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
fn vaults_body(app: &View, f: &BackupFlow, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    let mut cy = y;
    if !app.may_load_keys() {
        let pull = fill(FEN.backup_flow_pull_to_unlock, &[app.medium.noun()]);
        cy += stick_banner(ui, x, cy, w, &pull);
    }
    // The existing vaults first, then the new ones.
    let mut order: Vec<usize> = f.plan.used_vaults();
    order.sort_by_key(|&v| (!f.plan.vaults[v].existing, v));
    let needed = app.flow_needed_list();
    let order: Vec<usize> = order.into_iter().filter(|v| needed.contains(v)).collect();
    let open = |v: usize| {
        f.plan.vaults[v]
            .salt
            .is_some_and(|s| app.vaults.open.iter().any(|o| o.header().salt == s))
    };
    let active = order.iter().copied().find(|&v| !open(v));
    for &v in &order {
        let name = BackupFlow::vault_name(v);
        if open(v) {
            ui.icon(x, cy, 22.0, Icon::Done, 12.0, OK);
            ui.text(
                x + 28.0,
                cy + 2.0,
                14.0,
                W::R,
                TEXT,
                &format!("{name} · {}", FEN.backup_flow_unlocked),
            );
            cy += 30.0;
            continue;
        }
        if Some(v) != active {
            ui.text(x + 28.0, cy + 2.0, 14.0, W::R, MUTED, &name);
            cy += 30.0;
            continue;
        }
        let vp = &f.plan.vaults[v];
        if vp.salt.is_some() {
            cy += ui.wrap(
                x,
                cy,
                w,
                14.0,
                W::S,
                TEXT,
                &fill(FEN.backup_flow_unlock_vault, &[&name]),
            ) + 10.0;
            let focused = app.vaults.focus == Some(Focus::Passphrase);
            if app.may_load_keys() {
                crate::demo::pass_box(
                    app,
                    ui,
                    (x, cy, w),
                    &app.vaults.passphrase,
                    (app.vaults.typed_shown, focused),
                    (
                        Action::Vault(V::FocusPassphrase),
                        Action::Vault(V::ShowTyped),
                    ),
                    crate::demo::PassField::Unlock,
                );
            } else {
                stick_field(app.medium, ui, x, cy, w);
            }
            cy += 52.0 + crate::screens::never_under(app, ui, x, cy, w);
            if app.vaults.working == Some(vault::Work::Unlock) {
                cy += line(ui, x, cy, w, FEN.backup_flow_unlocking, ACCENT);
            } else {
                let style = if app.may_load_keys() {
                    Style::Primary
                } else {
                    Style::Disabled
                };
                let bw = ui.button(
                    x,
                    cy,
                    None,
                    40.0,
                    &fill(FEN.backup_flow_unlock_vault, &[&name]),
                    style,
                    Action::Vault(V::Unlock),
                );
                let free = match app.vaults.memory_free_mib {
                    Some(mib) => fill(FEN.backup_flow_free, &[&vault::mib_text(mib)]),
                    None => String::new(),
                };
                ui.text_mid(x + bw + 14.0, cy, 40.0, 13.0, W::R, MUTED, &free);
                cy += 52.0;
            }
            if let Some(e) = &app.vaults.unlock_error {
                cy += line(ui, x, cy, w, e, ERR);
            }
            // I do not have its passphrase.
            let np = f.live.no_pass == Some(v);
            cy += btn(
                ui,
                x,
                cy,
                None,
                32.0,
                FEN.backup_flow_no_passphrase,
                Style::Ghost,
                fa(F::NoPass(v)),
                40.0,
            );
            if np {
                let items = [
                    (
                        FEN.backup_flow_new_vault_beside.to_string(),
                        Style::Secondary,
                        fa(F::BesideIt(v)),
                    ),
                    (
                        fill(FEN.backup_flow_use_another, &[app.medium.noun()]),
                        Style::Secondary,
                        fa(F::AnotherStick(v)),
                    ),
                ];
                let refs: Vec<(&str, Style, Action)> =
                    items.iter().map(|(l, s, a)| (l.as_str(), *s, *a)).collect();
                cy += crate::screens::wrap_buttons(ui, x, cy, w, 32.0, &refs) + 6.0;
            }
        } else {
            let label = fill(FEN.backup_flow_create_vault, &[&name]);
            cy += btn(
                ui,
                x,
                cy,
                None,
                40.0,
                &label,
                Style::Primary,
                fa(F::CreateVault(v)),
                52.0,
            );
        }
    }
    cy - y
}

// ---- 4: make the key ----

#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
fn make_body(app: &View, f: &BackupFlow, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    if f.kind.is_wallet() {
        return crate::backup_flow::wallet_screen::make_body(app, f, ui, x, y, w);
    }
    let mut cy = y;
    if f.made && !f.adding {
        cy += ui.wrap(
            x,
            cy,
            w,
            15.0,
            W::S,
            OK,
            &fill(FEN.backup_flow_made, &[f.kind.name()]),
        ) + 12.0;
        cy += result_rows(app, f, ui, x, cy, w);
        for &v in &f.made_in {
            cy += line(
                ui,
                x,
                cy,
                w,
                &fill(FEN.backup_flow_in_vault, &[&BackupFlow::vault_name(v)]),
                TEXT,
            );
        }
        cy += btn(
            ui,
            x,
            cy,
            None,
            40.0,
            FEN.backup_flow_continue_make,
            Style::Primary,
            fa(F::MadeContinue),
            52.0,
        );
        return cy - y;
    }
    if f.adding {
        cy += line(ui, x, cy, w, FEN.backup_flow_copy_across, MUTED);
        cy += btn(
            ui,
            x,
            cy,
            None,
            40.0,
            FEN.backup_flow_copy_key,
            Style::Primary,
            fa(F::CopyKey),
            52.0,
        );
        return cy - y;
    }
    if app.vaults.working == Some(vault::Work::FlowMake) {
        cy += line(ui, x, cy, w, FEN.backup_flow_making, ACCENT);
        return cy - y;
    }
    if !app.may_load_keys() {
        cy += stick_banner(
            ui,
            x,
            cy,
            w,
            &fill(FEN.backup_flow_pull_to_unlock, &[app.medium.noun()]),
        );
    }
    if let Some(form) = &app.vaults.form {
        for (k, (_, label, b, _)) in form.fields.iter().enumerate() {
            ui.text(x, cy, 12.0, W::R, MUTED, label);
            cy += 20.0;
            let focused = app.vaults.focus == Some(Focus::Field(k));
            text_box(
                ui,
                x,
                cy,
                w,
                b,
                false,
                focused,
                Action::Vault(V::FocusField(k)),
            );
            cy += 52.0;
        }
        if f.kind == crate::backup_flow::Kind::Gpg {
            ui.text(x, cy, 12.0, W::R, MUTED, FEN.vault_expires);
            cy += 22.0;
            cy += crate::vault::screen::years_row(
                ui,
                x,
                cy,
                w,
                Some(app.vaults.gpg_years),
                V::GpgYears,
            ) + 16.0;
        }
        if f.kind == crate::backup_flow::Kind::Apk {
            cy += ui.wrap(x, cy, w, 12.0, W::R, DIM, FEN.apk_public_hint) + 12.0;
        }
    }
    cy += btn(
        ui,
        x,
        cy,
        None,
        40.0,
        FEN.backup_flow_make_the_key,
        Style::Primary,
        fa(F::Make),
        52.0,
    );
    cy - y
}

/// What was made, as the kind's page shows it: names, the public value, and
/// the public file For the stick with its code and picture (§4.2).
fn result_rows(app: &View, f: &BackupFlow, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    use crate::backup_flow::Kind;
    let mut cy = y;
    let Some(sub) = &f.subject else {
        return 0.0;
    };
    let record = f
        .made_in
        .iter()
        .filter_map(|&v| {
            let salt = f.plan.vaults.get(v)?.salt?;
            app.vaults.open.iter().position(|o| o.header().salt == salt)
        })
        .find_map(|o| {
            let at = app.flow_record_in(o, f.kind, &sub.id)?;
            app.vaults.records(app.vaults.open.get(o)?.id).get(at)
        });
    let Some(r) = record else {
        cy += row(ui, x, cy, w, FEN.backup_flow_name, &sub.name, false);
        return cy - y;
    };
    let buttons = |ui: &mut Ui, cy: f32, code: crate::Code| -> f32 {
        let mut bx = x;
        for (label, action) in [
            (FEN.vault_show_as_qr, Action::ShowCode(code)),
            ("PNG", Action::CodePng(code)),
        ] {
            let bw = ui.measure(13.0, W::S, label) + 30.0;
            ui.button(bx, cy, Some(bw), 36.0, label, Style::Secondary, action);
            bx += bw + 8.0;
        }
        48.0
    };
    match f.kind {
        Kind::Apk => {
            if let Some(s) = crate::apk::shown(r, app.tz_min()) {
                cy += row(ui, x, cy, w, FEN.apk_name, &s.name, false);
                cy += row(ui, x, cy, w, FEN.apk_package, &s.package, false);
                cy += row(
                    ui,
                    x,
                    cy,
                    w,
                    FEN.apk_certificate_sha_256,
                    &s.fingerprint,
                    true,
                );
                cy += row(ui, x, cy, w, FEN.apk_key, FEN.apk_rsa_3072, false);
                let file = crate::apk::fingerprint_of(r)
                    .map(|fp| format!("{}-certificate.pem", crate::apk::file_stem(&s.name, &fp)))
                    .unwrap_or_default();
                cy += row(ui, x, cy, w, FEN.apk_certificate, &file, false);
                cy += buttons(ui, cy, crate::Code::ApkCertificate);
            }
        }
        Kind::Gpg => {
            if let Some(g) = crate::gpg::shown(r, app.tz_min()) {
                for uid in &g.user_ids {
                    cy += row(ui, x, cy, w, FEN.vault_user_id, uid, false);
                }
                cy += row(ui, x, cy, w, FEN.vault_fingerprint, &g.fingerprint, true);
                if let Some(d) = &g.revocation_kept {
                    cy += row(
                        ui,
                        x,
                        cy,
                        w,
                        FEN.vault_revocation_certificate,
                        &fill(FEN.vault_kept_with_the_key, &[d]),
                        false,
                    );
                }
                cy += buttons(ui, cy, crate::Code::GpgKey);
            }
        }
        Kind::SecureBoot => {
            if let Some(sb) = crate::secure_boot::shown(r) {
                cy += row(ui, x, cy, w, FEN.vault_owner_guid, &sb.owner, true);
                for (label, name) in ["PK", "KEK", "db"].iter().zip(&sb.names) {
                    cy += row(ui, x, cy, w, label, name, false);
                }
            }
        }
        // A wallet's keys are shown by its own card.
        Kind::Wallet => {}
    }
    cy - y
}

fn row(ui: &mut Ui, x: f32, y: f32, w: f32, label: &str, value: &str, mono: bool) -> f32 {
    ui.text(x, y, 12.0, W::R, MUTED, label);
    let h = ui.wrap(
        x,
        y + 20.0,
        w,
        14.0,
        if mono { W::M } else { W::R },
        TEXT,
        value,
    );
    20.0 + h + 12.0
}

// ---- 5: lock ----

fn lock_body(app: &View, _f: &BackupFlow, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    let mut cy = y;
    let kw = app.kept_wiped(false);
    for (title, lines) in [
        (
            FEN.backup_flow_kept,
            kw.kept
                .iter()
                .map(|l| (l.clone(), TEXT))
                .collect::<Vec<_>>(),
        ),
        (
            FEN.backup_flow_wiped,
            kw.wiped
                .iter()
                .map(|(l, nowhere)| (l.clone(), if *nowhere { ERR } else { TEXT }))
                .collect(),
        ),
    ] {
        if lines.is_empty() {
            continue;
        }
        ui.text(x, cy, 12.0, W::R, MUTED, title);
        cy += 22.0;
        for (l, tone) in lines {
            cy += ui.wrap(x, cy, w, 14.0, W::R, tone, &l) + 6.0;
        }
        cy += 6.0;
    }
    cy += btn(
        ui,
        x,
        cy,
        None,
        40.0,
        FEN.backup_flow_lock_and_write,
        Style::Primary,
        fa(F::LockWrite),
        52.0,
    );
    cy - y
}

// ---- 6: write ----

fn write_body(app: &View, f: &BackupFlow, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    let mut cy = y;
    cy += slot_rows(app, f, ui, x, cy, w, true);
    cy += stick_card(app, f, ui, x, cy, w);
    cy - y
}

// ---- 7: done ----

fn done_body(app: &View, f: &BackupFlow, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    let mut cy = y;
    if !f.all_written() {
        return 0.0;
    }
    cy += ui.wrap(x, cy, w, 15.0, W::S, OK, FEN.backup_flow_done) + 12.0;
    for (k, s) in f.slots.iter().enumerate() {
        let Some(wr) = &s.written else {
            continue;
        };
        let name = app
            .flow_slot_row(k)
            .unwrap_or_else(|| f.target_short(&s.target, app.medium.noun()));
        let at = read_time(app, wr.time);
        cy += ui.wrap(
            x,
            cy,
            w,
            13.0,
            W::R,
            TEXT,
            &format!("{name} · {} · {at}", wr.label),
        ) + 4.0;
        // What was written to it, the public files with the copies.
        let files: Vec<&str> = wr.files.iter().map(|(n, _)| n.as_str()).collect();
        cy += ui.wrap(x, cy, w, 12.0, W::M, MUTED, &files.join(", ")) + 8.0;
    }
    cy += 8.0;
    // In Guided, extra copies (`docs/BACKUP-FLOW.md` §17.19 item 7).
    if f.wallet.is_some() {
        cy += crate::screens::extra_copies(app, ui, x, cy, w);
    }
    let mut items: Vec<(String, Style, Action)> = Vec::new();
    // Unlock the first of the plan's vaults; its key's page follows, or a
    // wallet's checklist.
    if let Some(v) = f.plan.used_vaults().first().copied() {
        items.push((
            fill(FEN.backup_flow_unlock_vault, &[&BackupFlow::vault_name(v)]),
            Style::Primary,
            fa(F::UnlockDone(v)),
        ));
    } else if f.kind.is_wallet() {
        // A plan with no vault leaves nothing to unlock.
        items.push((
            FEN.backup_flow_leave.to_string(),
            Style::Primary,
            fa(F::Back),
        ));
    }
    let refs: Vec<(&str, Style, Action)> =
        items.iter().map(|(l, s, a)| (l.as_str(), *s, *a)).collect();
    cy += crate::screens::wrap_buttons(ui, x, cy, w, 40.0, &refs) + 6.0;
    cy - y
}
