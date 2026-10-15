//! A wallet's cards of the backup flow (`docs/BACKUP-FLOW.md` §10): the
//! plan drawn from its answers once the keys exist; the card that sends
//! the person on to make the keys; and the cards Make a new wallet and
//! Restore gain before the keys: whether vaults are part of the backup,
//! how many, and the two of their setup. Drawing only; what a press does
//! is `backup_flow_steps.rs` and `backup_flow_wallet.rs`.

use crate::backup_flow::{BackupFlow, Step};
use crate::backup_flow_steps::FlowAction as F;
use crate::plan;
use crate::qrow;
use crate::screens::{section_label, stepper};
use crate::strings::{EN as FEN, fill};
use crate::ui::pal::*;
use crate::ui::{Style, Ui, W};
use crate::{Action, Faraday};

fn fa(a: F) -> Action {
    Action::Flow(a)
}

fn ans(list: u8, row: usize) -> Action {
    fa(F::Answer(list, row.min(255) as u8))
}

/// What a place is called before any vault has named it.
fn place(p: usize) -> String {
    BackupFlow::place_name(p)
}

/// The plan (the card is "The plan"): where the wallet's keys will be
/// kept, from the plan's answers. Into vaults or not, the places, and for each vault the seeds it
/// holds and the places that keep its stick; then the check lines, and the
/// way on.
pub(crate) fn where_body(
    app: &Faraday,
    f: &BackupFlow,
    ui: &mut Ui,
    x: f32,
    y: f32,
    w: f32,
) -> f32 {
    let Some(wf) = f.wallet.as_ref() else {
        return 0.0;
    };
    let (a, shape) = (&wf.answers, &wf.shape);
    let writing = f.live.writing.is_some();
    let mut cy = y;
    cy += ui.multi_list(
        x,
        cy,
        w,
        &[(
            FEN.app_into_vaults.to_string(),
            a.seeds[plan::seeds::VAULT],
            !writing,
            ans(qrow::SEEDS, plan::seeds::VAULT),
        )],
    );
    cy += 14.0;
    let p = a.places;
    stepper(
        ui,
        x,
        cy,
        w,
        FEN.plan_places_with_paper,
        p,
        ans(qrow::PLACES, p.saturating_sub(1)),
        ans(qrow::PLACES, p + 1),
        !writing,
    );
    cy += 52.0;
    let names: Vec<String> = (0..p).map(place).collect();
    for v in a.vaults_made(shape) {
        section_label(ui, x, cy, &fill(FEN.plan_vault, &[&(v + 1).to_string()]));
        // New, or one that exists already: the latter is read from its
        // stick at step 2.
        if let Some(vp) = f.plan.vaults.get(v)
            && vp.salt.is_none()
        {
            let other = if vp.existing {
                FEN.backup_flow_new_vault
            } else {
                FEN.backup_flow_existing_vault
            };
            let bw = ui.measure(13.0, W::S, other) + 32.0;
            ui.button(
                x + w - bw,
                cy - 6.0,
                Some(bw),
                32.0,
                other,
                Style::Secondary,
                fa(F::VaultExisting(v)),
            );
        }
        cy += 34.0;
        let seeds: Vec<(String, bool, bool, Action)> = shape
            .here()
            .into_iter()
            .map(|i| {
                (
                    shape.seeds[i].name.clone(),
                    a.vault_seeds(shape, v).contains(&i),
                    !writing,
                    ans(qrow::VAULTS.saturating_add(v.min(63) as u8), i),
                )
            })
            .collect();
        if shape.here().len() > 1 {
            cy += ui.multi_list(x, cy, w, &seeds) + 8.0;
        }
        section_label(
            ui,
            x,
            cy,
            &fill(FEN.plan_vault_s, &[&(v + 1).to_string(), app.medium.noun()]),
        );
        cy += 28.0;
        let sticks: Vec<(String, bool, bool, Action)> = (0..p)
            .map(|q| {
                (
                    names[q].clone(),
                    a.stick_at(v, q),
                    !writing,
                    ans(qrow::STICKS.saturating_add(v.min(63) as u8), q),
                )
            })
            .collect();
        cy += ui.multi_list(x, cy, w, &sticks) + 14.0;
    }
    cy += checks(app, f, ui, x, cy, w);
    if let Some(v) = a.stickless(shape) {
        cy += ui.wrap(
            x,
            cy,
            w,
            13.0,
            W::S,
            WARN,
            &fill(
                FEN.plan_vault_s_needs,
                &[&(v + 1).to_string(), app.medium.noun()],
            ),
        ) + 12.0;
        return cy - y;
    }
    ui.button(
        x,
        cy,
        None,
        40.0,
        FEN.backup_flow_continue_make,
        Style::Primary,
        fa(F::Continue),
    );
    cy + 52.0 - y
}

/// The plan's check lines, with the two lines of fact under them, as the
/// map panel draws them.
fn checks(app: &Faraday, f: &BackupFlow, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    let Some(wf) = f.wallet.as_ref() else {
        return 0.0;
    };
    let c = plan::check(&wf.shape, &wf.answers);
    let _ = app;
    crate::glance::check_lines(ui, x, y, w, c, false, &place) + 16.0
}

/// The Make card of a wallet's flow: the keys are made in Make a new
/// wallet, not here.
pub(crate) fn make_body(app: &Faraday, f: &BackupFlow, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    let mut cy = y;
    if f.made {
        let name = f.subject.as_ref().map_or(String::new(), |s| s.name.clone());
        cy += ui.wrap(x, cy, w, 15.0, W::S, OK, &name) + 12.0;
        for &v in &f.made_in {
            cy += ui.wrap(
                x,
                cy,
                w,
                14.0,
                W::R,
                TEXT,
                &fill(FEN.backup_flow_in_vault, &[&BackupFlow::vault_name(v)]),
            ) + 10.0;
        }
        if f.adding {
            cy += ui.wrap(x, cy, w, 14.0, W::R, MUTED, FEN.backup_flow_copy_across) + 10.0;
            // A seed kept only on paper is typed back from its copy.
            let missing = app.wallet_round_missing(app.flow_index_of(f));
            if !missing.is_empty() {
                cy += ui.wrap(
                    x,
                    cy,
                    w,
                    14.0,
                    W::S,
                    WARN,
                    &fill(FEN.backup_flow_on_paper_only, &[&missing.join(", ")]),
                ) + 10.0;
                ui.button(
                    x,
                    cy,
                    None,
                    40.0,
                    FEN.backup_flow_add_from_copy,
                    Style::Secondary,
                    Action::Entry(None),
                );
                cy += 52.0;
            }
            ui.button(
                x,
                cy,
                None,
                40.0,
                FEN.backup_flow_copy_key,
                Style::Primary,
                fa(F::CopyKey),
            );
            cy += 52.0;
        } else {
            ui.button(
                x,
                cy,
                None,
                40.0,
                FEN.backup_flow_continue_make,
                Style::Primary,
                fa(F::MadeContinue),
            );
            cy += 52.0;
        }
        return cy - y;
    }
    ui.button(
        x,
        cy,
        None,
        40.0,
        FEN.backup_flow_make_the_keys,
        Style::Primary,
        fa(F::WalletMake),
    );
    cy + 52.0 - y
}

/// The question of vaults, the same in Make a new wallet and Restore: the
/// question, Yes and No, and Continue once it is answered.
fn ask_card(
    ui: &mut Ui,
    (x, y, w): (f32, f32, f32),
    ask: crate::VaultAsk,
    locked: bool,
    (yes, no): (Action, Action),
    next: Action,
) -> f32 {
    let mut cy = y;
    cy += ui.wrap(
        x,
        cy,
        w,
        14.0,
        W::R,
        TEXT,
        FEN.backup_flow_use_vaults_question,
    ) + 14.0;
    let style = |on: bool| {
        if locked {
            Style::Disabled
        } else if on {
            Style::Primary
        } else {
            Style::Secondary
        }
    };
    let bw = 96.0;
    ui.button(
        x,
        cy,
        Some(bw),
        40.0,
        FEN.plan_yes,
        style(ask.answer == Some(true)),
        yes,
    );
    ui.button(
        x + bw + 10.0,
        cy,
        Some(bw),
        40.0,
        FEN.plan_no,
        style(ask.answer == Some(false)),
        no,
    );
    cy += 52.0;
    if !locked {
        let label = FEN.create_continue;
        let lw = ui.measure(14.0, W::S, label) + 36.0;
        // Nothing is asked of the person until they have answered.
        let style = if ask.answer.is_some() {
            Style::Primary
        } else {
            Style::Disabled
        };
        ui.button(x + w - lw, cy, Some(lw), 40.0, label, style, next);
        cy += 48.0;
    }
    cy - y
}

/// The count of vaults, the same in Make a new wallet and Restore: the
/// question over a stepper from one up, and Continue.
fn count_card(
    ui: &mut Ui,
    (x, y, w): (f32, f32, f32),
    count: usize,
    locked: bool,
    set: fn(u16) -> Action,
    next: Action,
) -> f32 {
    let mut cy = y;
    cy += ui.wrap(
        x,
        cy,
        w,
        14.0,
        W::R,
        TEXT,
        FEN.backup_flow_how_many_question,
    ) + 14.0;
    let n = count.min(usize::from(u16::MAX)) as u16;
    stepper(
        ui,
        x,
        cy,
        w,
        FEN.backup_flow_vault_count_label,
        count,
        set(n.saturating_sub(1).max(1)),
        set(n.saturating_add(1)),
        !locked,
    );
    cy += 52.0;
    if !locked {
        let label = FEN.create_continue;
        let lw = ui.measure(14.0, W::S, label) + 36.0;
        ui.button(x + w - lw, cy, Some(lw), 40.0, label, Style::Primary, next);
        cy += 48.0;
    }
    cy - y
}

/// Make a new wallet's question of vaults.
pub(crate) fn create_where(app: &Faraday, ui: &mut Ui, x: f32, y: f32, w: f32) -> Option<f32> {
    let c = app.create.as_ref()?;
    Some(ask_card(
        ui,
        (x, y, w),
        c.vaults,
        c.built.is_some(),
        (Action::CVaults(true), Action::CVaults(false)),
        Action::CNext(crate::cstep::WHERE),
    ))
}

/// Make a new wallet's count of vaults.
pub(crate) fn create_count(app: &Faraday, ui: &mut Ui, x: f32, y: f32, w: f32) -> Option<f32> {
    let c = app.create.as_ref()?;
    Some(count_card(
        ui,
        (x, y, w),
        c.vaults.count,
        c.built.is_some(),
        Action::CVaultN,
        Action::CNext(crate::cstep::HOW_MANY),
    ))
}

/// Restore's question of vaults.
pub(crate) fn restore_where(app: &Faraday, ui: &mut Ui, x: f32, y: f32, w: f32) -> Option<f32> {
    let r = app.restore.as_ref()?;
    Some(ask_card(
        ui,
        (x, y, w),
        r.vaults,
        r.done[crate::rstep::SEEDS as usize],
        (Action::RVaults(true), Action::RVaults(false)),
        Action::RNext(crate::rstep::WHERE),
    ))
}

/// Restore's count of vaults.
pub(crate) fn restore_count(app: &Faraday, ui: &mut Ui, x: f32, y: f32, w: f32) -> Option<f32> {
    let r = app.restore.as_ref()?;
    Some(count_card(
        ui,
        (x, y, w),
        r.vaults.count,
        r.done[crate::rstep::SEEDS as usize],
        Action::RVaultN,
        Action::RNext(crate::rstep::HOW_MANY),
    ))
}

/// Make a new wallet's Bring in the sticks and Vaults cards: where the
/// flow stands, and **Change**, which opens the flow on that card.
pub(crate) fn create_flow_card(
    app: &Faraday,
    ui: &mut Ui,
    step: Step,
    x: f32,
    y: f32,
    w: f32,
) -> Option<f32> {
    let i = app.wallet_flow_of_owner(app.create.as_ref()?.owner)?;
    let f = &app.backup_flows[i];
    let mut cy = y;
    let line = match step {
        Step::Sticks => {
            let bound = f.slots.iter().filter(|s| s.bound.is_some()).count();
            fill(
                FEN.backup_flow_read_of,
                &[&bound.to_string(), &f.slots.len().to_string()],
            )
        }
        _ => {
            let used = f.plan.used_vaults();
            let open = used
                .iter()
                .filter(|&&v| app.flow_open_index(i, v).is_some())
                .count();
            fill(
                FEN.backup_flow_unlocked_of,
                &[&open.to_string(), &used.len().to_string()],
            )
        }
    };
    cy += ui.wrap(x, cy, w, 14.0, W::R, TEXT, &line) + 14.0;
    let label = FEN.backup_flow_change;
    let bw = ui.measure(14.0, W::S, label) + 36.0;
    ui.button(
        x + w - bw,
        cy,
        Some(bw),
        40.0,
        label,
        Style::Secondary,
        fa(F::Show(i)),
    );
    cy += 48.0;
    Some(cy - y)
}
