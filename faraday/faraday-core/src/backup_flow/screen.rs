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
use crate::ui::{Style, Ui};
use crate::vault::screen::{hold_button_in, stick_banner, stick_field, text_box};
use crate::vault::{self, Focus, VaultAction as V};
use crate::{Action, Faraday, flow};
use faraday_draw::tokens::{BODY_M, BODY_S, BUTTON_H, FIELD_H, LABEL_L, TITLE_M};
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
    // On a large window the page runs to the window's edge: the cards
    // keep the reading column, the board runs on to the supporting pane's
    // spacer, and its list is that pane (`docs/LAYOUT.md` §14, §17).
    // The other cards keep that column, so none moves when the board's
    // card opens.
    let wide = crate::screens::backup_work_end(app);
    let work = crate::screens::board_work_end(app);
    let panel_w = if work.is_none() && !ui.compact && app.board_shown() {
        crate::screens::side_panel_w(cw)
    } else {
        0.0
    };
    let col_x = x0 + 40.0;
    let col = flow::Column {
        area_x: x0,
        area_w: work.map_or(cw - panel_w, |end| end - x0 + faraday_draw::tokens::SPACER),
        x: col_x,
        w: match wide {
            Some(end) => (end - col_x).min(crate::board::screen::column_w()),
            None if panel_w > 0.0 => cw - panel_w - 72.0,
            None => (cw - 72.0).min(820.0),
        },
        h,
        back: Some(app.named_back((leave_name(f.kind), fa(F::Back)))),
        heading: f.kind.name(),
        // The setup of a wallet's vaults is Make a new wallet's, with its
        // Guided switch (owner, 2026-10-25).
        guided: f.is_setup() && app.guided(),
        switch: f.is_setup(),
        note: None,
        chip: None,
        chip_tap: None,
    };
    let scroll = f.live.scroll;
    let (next, again) = {
        let app_ref: &View = app;
        let f = app_ref.flow().unwrap();
        let mut body = |ui: &mut Ui, i: usize, x: f32, y: f32, w: f32| -> f32 {
            body(app_ref, f, ui, steps[i], x, y, w)
        };
        flow::column_wide(ui, &col, work, &cards, scroll, &mut body, None)
    };
    app.scroll(Scrolled::BackupFlow, next, again);
    if work.is_some() {
        let pane = crate::screens::support_pane_place((x0, cw, h));
        crate::board::screen::list_pane(app, ui, pane);
    } else if panel_w > 0.0 {
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
            // Passed with sticks not read: they hold only new vaults, read
            // as they are written (`docs/BACKUP-FLOW.md` §4.2).
            if bound == 0 && f.step > Step::Sticks {
                FEN.backup_flow_read_at_write.to_string()
            } else if bound < m && f.step > Step::Sticks {
                fill(
                    FEN.backup_flow_read_rest_at_write,
                    &[&bound.to_string(), &m.to_string()],
                )
            } else {
                fill(
                    FEN.backup_flow_read_count,
                    &[&bound.to_string(), &m.to_string()],
                )
            }
        }
        Step::Vaults if f.step > Step::Vaults => match f.plan.used_vaults().len() {
            1 => FEN.backup_flow_vaults_one.to_string(),
            n => fill(FEN.backup_flow_vaults_n, &[&n.to_string()]),
        },
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
        guide: setup_guide(app, f, s),
        learn: f
            .is_setup()
            .then_some(crate::learn::Topic::Vaults)
            .filter(|_| matches!(s, Step::Sticks | Step::Vaults)),
    }
}

/// The walk-through of the setup of a wallet's vaults, card by card, as
/// Make a new wallet's own cards have theirs; the backup's other flows
/// have none.
fn setup_guide(app: &View, f: &BackupFlow, s: Step) -> Option<String> {
    if !f.is_setup() {
        return None;
    }
    let (m, ms) = (app.medium.noun(), app.medium.nouns());
    match s {
        Step::Sticks => Some(fill(FEN.backup_flow_guide_sticks, &[m, ms, m])),
        Step::Vaults => Some(FEN.guide_create_vaults.to_string()),
        _ => None,
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
            BUTTON_H,
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
        ui.text_as(x, cy, LABEL_L, Role::OnSurface, &BackupFlow::place_name(p));
        ui.text_as(x + 90.0, cy, BODY_M, Role::OnSurfaceVariant, &keeps);
        cy += ui.advance_as(BODY_M, 26.0);
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
                    Style::chosen(on),
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
                    Style::chosen(f.plan.together.contains(&p)),
                    fa(F::Together(p)),
                ));
            }
            if !vs.is_empty() {
                items.push((
                    FEN.backup_flow_public_here.to_string(),
                    if f.plan.public_at == p {
                        Style::Selected
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
        BUTTON_H,
        FEN.backup_flow_add_place,
        Style::Secondary,
        fa(F::AddPlace),
        44.0,
    );
    ui.rule(x, cy, w, Role::OutlineVariant);
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
        ui.text_as(x, cy, LABEL_L, Role::OnSurface, &line);
        ui.text_as(
            x,
            cy + 22.0,
            BODY_S,
            Role::OnSurfaceVariant,
            &places.join(", "),
        );
        if vp.salt.is_none() {
            let other = if vp.existing {
                FEN.backup_flow_new_vault
            } else {
                FEN.backup_flow_existing_vault
            };
            let bw = ui.measure_as(LABEL_L, other) + 32.0;
            ui.button(
                x + w - bw,
                cy,
                Some(bw),
                BUTTON_H,
                other,
                Style::Secondary,
                fa(F::VaultExisting(v)),
            );
        }
        cy += 50.0;
    }
    if sticks < min {
        cy += ui.wrap_as(
            (x, cy, w),
            BODY_S,
            Role::Warning,
            &fill(FEN.backup_flow_at_least, &[&min.to_string()]),
        ) + 10.0;
    }
    let style = Style::Primary.when(sticks >= min);
    ui.button(
        x,
        cy,
        None,
        BUTTON_H,
        FEN.backup_flow_continue_make,
        style,
        fa(F::Continue),
    );
    cy + 52.0 - y
}

// ---- helpers ----

fn line(ui: &mut Ui, x: f32, y: f32, w: f32, text: &str, color: crate::ui::Paint) -> f32 {
    ui.wrap_as((x, y, w), BODY_M, color, text) + 10.0
}

/// The medium's icon and a line centred, as Insert is.
fn insert(app: &View, ui: &mut Ui, x: f32, y: f32, w: f32, text: &str) -> f32 {
    ui.icon(
        x + (w - 44.0) / 2.0,
        y,
        44.0,
        app.medium.icon(),
        22.0,
        Role::OnSurfaceVariant,
    );
    let tw = ui.measure_as(LABEL_L, text).min(w);
    let t = ui.fit_as(LABEL_L, text, w);
    ui.text_as(x + (w - tw) / 2.0, y + 54.0, LABEL_L, Role::OnSurface, &t);
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
            Some(t) => w - ui.measure_as(BODY_S.mono(), t) - 12.0,
            None => w,
        };
        let name = app
            .flow_slot_row(k)
            .unwrap_or_else(|| cap(&f.target_name(&s.target, m)));
        let name = ui.fit_as(BODY_S, &name, room);
        ui.text_as(
            x,
            cy,
            BODY_S,
            if done {
                Role::OnSurface
            } else {
                Role::OnSurfaceVariant
            },
            &name,
        );
        if let Some(t) = tail {
            ui.text_right_as((x + w, cy - 4.0, 22.0), BODY_S.mono(), Role::Ok, &t);
        }
        cy += 24.0;
        // The stick just taken for this place, while it is in: the
        // number of its pill, to mark on it (§18).
        if let Some(mark) = app.flow_mark_line(k) {
            cy += line(ui, x, cy - 4.0, w, &mark, Role::Primary.paint()) - 4.0;
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
            BUTTON_H,
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
            cy += line(ui, x, cy, w, &app.flow_problem_text(p), Role::Error.paint());
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
                Role::Warning.paint(),
            );
            return cy - y;
        }
        cy += insert(app, ui, x, cy, w, &app.flow_insert_line(k));
        // The stick the machine started from, read already; not once it
        // holds another place: one stick, one place.
        let boot_placed = (0..f.slots.len()).any(|j| j != k && f.boot_holds(j));
        if matches!(f.step, Step::Sticks | Step::Write) && !boot_placed {
            for (i, st) in app.stick.iter().enumerate().filter(|(_, s)| s.boot) {
                let at = read_time(app, app.clock_now());
                let label = fill(FEN.backup_flow_use_read, &[&st.label, &at]);
                // No wider than the column: a longer label fits down.
                let natural =
                    ui.measure_as(LABEL_L, &label) + 2.0 * faraday_draw::tokens::BUTTON_PAD;
                cy += btn(
                    ui,
                    x,
                    cy,
                    Some(natural.min(w)),
                    BUTTON_H,
                    &label,
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
        cy += ui.wrap_as(
            (x, cy, w),
            LABEL_L,
            Role::OnSurface,
            &app.flow_insert_line(r.slot),
        ) + 8.0;
    }
    if let Some(p) = &r.problem {
        cy += line(ui, x, cy, w, &app.flow_problem_text(p), Role::Error.paint());
        // Which of the backup's sticks it is, if any (§18).
        if let Some(which) = app.flow_stick_from_elsewhere() {
            cy += line(ui, x, cy, w, &which, Role::OnSurface.paint());
        }
        cy += btn(
            ui,
            x,
            cy,
            None,
            BUTTON_H,
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
            Role::OnSurfaceVariant.paint(),
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
                Role::Ok.paint(),
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
        // A state, not a link: in the text's colour, the work under way
        // shown by the indicator along its foot (`docs/DESIGN.md` §4.14).
        let lh = line(
            ui,
            x,
            cy,
            w,
            &fill(FEN.backup_flow_writing, &[&file]),
            Role::OnSurface.paint(),
        );
        crate::screens::working_bar(ui, x, cy, w, lh);
        cy += lh;
        return cy - y;
    }
    match r.purpose {
        crate::backup_flow::Purpose::Bind => {
            // A blank stick read alike: which of the backup's sticks it
            // reads as, above the hold (§18).
            if let Some(like) = app.flow_stick_reads_as() {
                cy += line(ui, x, cy, w, &like, Role::OnSurface.paint());
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
                    Role::OnSurfaceVariant.paint(),
                );
            }
        }
    }
    cy - y
}

/// The hold, and **Use another stick** beside it, or under it where the row
/// is too narrow. Returns the height.
fn hold_row(app: &View, ui: &mut Ui, x: f32, y: f32, w: f32, label: &str, other: &str) -> f32 {
    let bw = hold_button_in(
        app,
        ui,
        x,
        y,
        w,
        label,
        fa(F::Hold),
        faraday_ui::hold::HoldTone::Primary,
    );
    let ow = ui.measure_as(LABEL_L, other) + 32.0;
    if bw + 10.0 + ow <= w {
        ui.button(
            x + bw + 10.0,
            y,
            None,
            BUTTON_H,
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
            BUTTON_H,
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
            cy += ui.wrap_as((x, cy, w), BODY_S.mono(), Role::OnSurfaceVariant, &line) + 6.0;
        }
    }
    for n in &f.slots[slot].files {
        if let Some(it) = app.outbox.iter().find(|i| &i.name == n) {
            let line = format!("{} · {}", n, vault::file_text(it.bytes.len()));
            cy += ui.wrap_as((x, cy, w), BODY_S.mono(), Role::OnSurfaceVariant, &line) + 6.0;
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
            Role::Warning.paint(),
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
        cy += ui.wrap_as((x, cy, w), LABEL_L, Role::OnSurface, &title) + 8.0;
        if !vp.existing {
            cy += btn(
                ui,
                x,
                cy,
                Some(w),
                BUTTON_H,
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
            ui.fill(
                x,
                cy,
                w,
                44.0,
                faraday_draw::tokens::SHAPE_M,
                Role::SurfaceContainerLow,
            );
            ui.stroke(
                x,
                cy,
                w,
                44.0,
                faraday_draw::tokens::SHAPE_M,
                Role::OutlineVariant,
            );
            let t = ui.fit_as(BODY_S.mono(), &line, w - 24.0);
            ui.text_mid_as((x + 12.0, cy, 44.0), BODY_S.mono(), Role::OnSurface, &t);
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
                BUTTON_H,
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
            BUTTON_H,
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
    // Every stick is read before this step: it offers each vault found
    // to open and each new one to create, in any order; the one being
    // unlocked shows its passphrase (owner, 2026-10-26).
    let first = order.iter().copied().find(|&v| !open(v));
    for &v in &order {
        let name = BackupFlow::vault_name(v);
        if open(v) {
            ui.icon(x, cy, 22.0, Icon::Done, 12.0, Role::Ok);
            ui.text_as(
                x + 28.0,
                cy + 2.0,
                BODY_M,
                Role::OnSurface,
                &format!("{name} · {}", FEN.backup_flow_unlocked),
            );
            cy += ui.advance_as(BODY_M, 30.0);
            continue;
        }
        let vp = &f.plan.vaults[v];
        if f.live.unlocking != Some(v) || vp.salt.is_none() {
            let (label, action) = if vp.salt.is_some() {
                (FEN.backup_flow_open_vault, F::UnlockVault(v))
            } else {
                (FEN.backup_flow_create_vault, F::CreateVault(v))
            };
            let busy = f.live.unlocking.is_some() || Some(v) != first;
            let style = if busy {
                Style::Secondary
            } else {
                Style::Primary
            };
            let label = fill(label, &[&name]);
            cy += btn(ui, x, cy, None, BUTTON_H, &label, style, fa(action), 52.0);
            continue;
        }
        if vp.salt.is_some() {
            cy += ui.wrap_as(
                (x, cy, w),
                LABEL_L,
                Role::OnSurface,
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
            cy += FIELD_H + 12.0 + crate::screens::never_under(app, ui, x, cy, w);
            if app.vaults.working == Some(vault::Work::Unlock) {
                cy += line(
                    ui,
                    x,
                    cy,
                    w,
                    FEN.backup_flow_unlocking,
                    Role::Primary.paint(),
                );
            } else {
                let style = Style::Primary.when(app.may_load_keys());
                let bw = ui.button(
                    x,
                    cy,
                    None,
                    BUTTON_H,
                    &fill(FEN.backup_flow_unlock_vault, &[&name]),
                    style,
                    Action::Vault(V::Unlock),
                );
                let free = match app.vaults.memory_free_mib {
                    Some(mib) => fill(FEN.backup_flow_free, &[&vault::mib_text(mib)]),
                    None => String::new(),
                };
                ui.text_mid_as(
                    (x + bw + 14.0, cy, 40.0),
                    BODY_S,
                    Role::OnSurfaceVariant,
                    &free,
                );
                cy += 52.0;
            }
            if let Some(e) = &app.vaults.unlock_error {
                cy += line(ui, x, cy, w, e, Role::Error.paint());
            }
            // I do not have its passphrase.
            let np = f.live.no_pass == Some(v);
            cy += btn(
                ui,
                x,
                cy,
                None,
                BUTTON_H,
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
        cy += ui.wrap_as(
            (x, cy, w),
            TITLE_M,
            Role::Ok,
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
                Role::OnSurface.paint(),
            );
        }
        cy += btn(
            ui,
            x,
            cy,
            None,
            BUTTON_H,
            FEN.backup_flow_continue_make,
            Style::Primary,
            fa(F::MadeContinue),
            52.0,
        );
        return cy - y;
    }
    if f.adding {
        cy += line(
            ui,
            x,
            cy,
            w,
            FEN.backup_flow_copy_across,
            Role::OnSurfaceVariant.paint(),
        );
        cy += btn(
            ui,
            x,
            cy,
            None,
            BUTTON_H,
            FEN.backup_flow_copy_key,
            Style::Primary,
            fa(F::CopyKey),
            52.0,
        );
        return cy - y;
    }
    if app.vaults.working == Some(vault::Work::FlowMake) {
        cy += line(ui, x, cy, w, FEN.backup_flow_making, Role::Primary.paint());
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
            ui.text_as(x, cy, BODY_S, Role::OnSurfaceVariant, label);
            cy += ui.line_as(BODY_S) + faraday_draw::tokens::GAP_SMALL;
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
            cy += FIELD_H + 12.0;
        }
        if f.kind == crate::backup_flow::Kind::Gpg {
            ui.text_as(x, cy, BODY_S, Role::OnSurfaceVariant, FEN.vault_expires);
            cy += ui.advance_as(BODY_S, 22.0);
            cy += crate::vault::screen::years_row(
                ui,
                x,
                cy,
                w,
                Some(app.vaults.gpg_years),
                V::GpgYears,
            ) + 16.0;
        }
    }
    cy += btn(
        ui,
        x,
        cy,
        None,
        BUTTON_H,
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
            let at = super::record_in(&app.vaults, o, f.kind, &sub.id)?;
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
            let bw = ui.measure_as(LABEL_L, label) + 30.0;
            ui.button(bx, cy, Some(bw), BUTTON_H, label, Style::Secondary, action);
            bx += bw + 8.0;
        }
        48.0
    };
    match f.kind {
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
    ui.text_as(x, y, BODY_S, Role::OnSurfaceVariant, label);
    let h = ui.wrap_as(
        (x, y + 20.0, w),
        if mono { BODY_M.mono() } else { BODY_M },
        Role::OnSurface,
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
                .map(|l| (l.clone(), Role::OnSurface))
                .collect::<Vec<_>>(),
        ),
        (
            FEN.backup_flow_wiped,
            kw.wiped
                .iter()
                .map(|(l, nowhere, _)| {
                    (
                        l.clone(),
                        if *nowhere {
                            Role::Error
                        } else {
                            Role::OnSurface
                        },
                    )
                })
                .collect(),
        ),
    ] {
        if lines.is_empty() {
            continue;
        }
        ui.text_as(x, cy, BODY_S, Role::OnSurfaceVariant, title);
        cy += ui.advance_as(BODY_S, 22.0);
        for (l, tone) in lines {
            cy += ui.wrap_as((x, cy, w), BODY_M, tone, &l) + 6.0;
        }
        cy += 6.0;
    }
    cy += btn(
        ui,
        x,
        cy,
        None,
        BUTTON_H,
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
    cy += ui.wrap_as((x, cy, w), TITLE_M, Role::Ok, FEN.backup_flow_done) + 12.0;
    for (k, s) in f.slots.iter().enumerate() {
        let Some(wr) = &s.written else {
            continue;
        };
        let name = app
            .flow_slot_row(k)
            .unwrap_or_else(|| f.target_short(&s.target, app.medium.noun()));
        let at = read_time(app, wr.time);
        cy += ui.wrap_as(
            (x, cy, w),
            BODY_S,
            Role::OnSurface,
            &format!("{name} · {} · {at}", wr.label),
        ) + 4.0;
        // What was written to it, the public files with the copies.
        let files: Vec<&str> = wr.files.iter().map(|(n, _)| n.as_str()).collect();
        cy += ui.wrap_as(
            (x, cy, w),
            BODY_S.mono(),
            Role::OnSurfaceVariant,
            &files.join(", "),
        ) + 8.0;
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

/// Where the flow's Back lands when nothing else on the way back names a
/// screen (after a lock, which starts the trail again): Tools for the
/// keys Tools starts, Home for a wallet's.
fn leave_name(kind: crate::backup_flow::Kind) -> &'static str {
    if kind.is_wallet() {
        FEN.backup_flow_leave
    } else {
        FEN.backup_flow_leave_tools
    }
}
