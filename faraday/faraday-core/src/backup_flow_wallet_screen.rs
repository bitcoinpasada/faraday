//! A wallet's cards of the backup flow (`docs/BACKUP-FLOW.md` §10): where
//! its keys will be kept, drawn from the plan's answers; the card that
//! sends the person on to make the keys; and the three cards Make a new
//! wallet gains before Keys. Drawing only; what a press does is
//! `backup_flow_steps.rs` and `backup_flow_wallet.rs`.

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

/// Where the wallet's keys will be kept: the plan's vault part, from its
/// answers. Into vaults or not, the places, and for each vault the seeds it
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

/// Make a new wallet's Where card: not in a vault, or in vaults; and for
/// a plan kept in vaults, what was decided, with **Change**.
pub(crate) fn create_where(app: &Faraday, ui: &mut Ui, x: f32, y: f32, w: f32) -> Option<f32> {
    let c = app.create.as_ref()?;
    let locked = c.built.is_some();
    let into = app.create_in_vaults();
    let flow = app.wallet_flow_of_owner(c.owner);
    let mut cy = y;
    let rows = [
        (
            FEN.backup_flow_not_in_vault.to_string(),
            !into,
            !locked,
            Action::CWhere(false),
        ),
        (
            FEN.app_into_vaults.to_string(),
            into,
            !locked,
            Action::CWhere(true),
        ),
    ];
    cy += ui.multi_list(x, cy, w, &rows) + 12.0;
    if let Some(i) = flow.filter(|_| into) {
        cy += flow_lines(app, &app.backup_flows[i], ui, x, cy, w);
    }
    if !locked {
        let label = if flow.is_some() && into {
            FEN.backup_flow_change
        } else {
            FEN.create_continue
        };
        let bw = ui.measure(14.0, W::S, label) + 36.0;
        ui.button(
            x + w - bw,
            cy,
            Some(bw),
            40.0,
            label,
            Style::Primary,
            Action::CNext(crate::cstep::WHERE),
        );
        cy += 48.0;
    }
    Some(cy - y)
}

/// Restore's Where card: the seeds are not kept in a vault, or in vaults;
/// for a plan kept in vaults, what was decided, with **Change**.
pub(crate) fn restore_where(app: &Faraday, ui: &mut Ui, x: f32, y: f32, w: f32) -> Option<f32> {
    let r = app.restore.as_ref()?;
    let locked = r.wallet.is_some();
    let mut cy = y;
    let rows = [
        (
            FEN.backup_flow_not_in_vault.to_string(),
            !r.into_vaults,
            !locked,
            Action::RWhere(false),
        ),
        (
            FEN.app_into_vaults.to_string(),
            r.into_vaults,
            !locked,
            Action::RWhere(true),
        ),
    ];
    cy += ui.multi_list(x, cy, w, &rows) + 12.0;
    if let Some(i) = app.wallet_flow_pre().filter(|_| r.into_vaults) {
        cy += flow_lines(app, &app.backup_flows[i], ui, x, cy, w);
    }
    if !locked {
        let label = if app.wallet_flow_pre().is_some() && r.into_vaults {
            FEN.backup_flow_change
        } else {
            FEN.create_continue
        };
        let bw = ui.measure(14.0, W::S, label) + 36.0;
        ui.button(
            x + w - bw,
            cy,
            Some(bw),
            40.0,
            label,
            Style::Primary,
            Action::RNext(crate::rstep::WHERE),
        );
        cy += 48.0;
    }
    Some(cy - y)
}

/// The lines under the Where card once the flow has decided: the places
/// and what each keeps, the vaults and where their sticks are.
fn flow_lines(app: &Faraday, f: &BackupFlow, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    let mut cy = y;
    for (p, vs) in f.plan.places.iter().enumerate() {
        if vs.is_empty() {
            continue;
        }
        let names: Vec<String> = vs.iter().map(|&v| BackupFlow::vault_name(v)).collect();
        let line = format!(
            "{} · {} {}",
            place(p),
            fill(FEN.backup_flow_place_keeps, &[app.medium.noun()]),
            names.join(", ")
        );
        cy += ui.wrap(x, cy, w, 13.0, W::R, TEXT, &line) + 6.0;
    }
    cy + 8.0 - y
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
