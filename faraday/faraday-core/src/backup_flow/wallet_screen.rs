//! A wallet's cards of the backup flow (`docs/BACKUP-FLOW.md` §10): the
//! plan drawn from its answers once the keys exist; the card that sends
//! the person on to make the keys; and the cards Make a new wallet and
//! Restore gain before the keys: whether vaults are part of the backup,
//! how many, and the two of their setup. Drawing only; what a press does
//! is `backup_flow/steps.rs` and `backup_flow/wallet.rs`.

use crate::Action;
use crate::backup_flow::steps::FlowAction as F;
use crate::backup_flow::{BackupFlow, Step};
use crate::drawing::View;
use crate::screens::stepper;
use crate::strings::{EN as FEN, fill};
use crate::ui::Shape;
use crate::ui::pal::*;
use crate::ui::{Style, Ui};
use faraday_draw::tokens::{BODY_M, BODY_S, BUTTON_H, LABEL_L, TITLE_M};

fn fa(a: F) -> Action {
    Action::Flow(a)
}

/// The plan (the card is "The plan"): where the wallet's keys will be
/// kept, asked place by place with Back up's own cards
/// (`docs/BACKUP-FLOW.md` §15), drawn by the same code: Places, a card
/// per place with its rows' sheets, the wallet description, where it
/// goes, the software, and The plan, the map with its checks. While a
/// stick is written nothing takes a press. Under The plan, each vault
/// whose stick is not read yet is New or Existing, then what holds the
/// way on, or Continue.
pub(crate) fn where_body(app: &View, f: &BackupFlow, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    use crate::plan::cards::{self as cards, PlanCtx};
    let Some(ctx) = PlanCtx::flow(app, app.flow_index_of(f)) else {
        return 0.0;
    };
    let mut cy = y;
    let open = f.live.plan_q;
    for q in crate::plan_cards_of(&ctx) {
        cy += card_head(
            ui,
            x,
            cy,
            w,
            &cards::title(&ctx, q),
            &cards::summary(&ctx, q),
            open == Some(q),
            ctx.act(crate::PlanPress::Q(q)),
        );
        if open != Some(q) {
            continue;
        }
        cy += cards::body(app, ui, &ctx, q, x + 12.0, cy, w - 12.0) + 8.0;
        if q != crate::qstep::PLAN {
            continue;
        }
        // New, or one that exists already: the latter is read from its
        // stick at step 2.
        for v in ctx.answers.vaults_made(&ctx.shape) {
            let Some(vp) = f.plan.vaults.get(v).filter(|vp| vp.salt.is_none()) else {
                continue;
            };
            let label = fill(FEN.plan_vault, &[&(v + 1).to_string()]);
            let state = if vp.existing {
                FEN.backup_flow_existing_vault
            } else {
                FEN.backup_flow_new_vault
            };
            let other = if vp.existing {
                FEN.backup_flow_new_vault
            } else {
                FEN.backup_flow_existing_vault
            };
            ui.text_mid_as(
                (x + 12.0, cy, 32.0),
                LABEL_L,
                Role::OnSurface,
                &fill(FEN.backup_flow_pick_title, &[&label, state]),
            );
            let bw = ui.measure_as(LABEL_L, other) + 32.0;
            ui.button(
                x + w - bw,
                cy,
                Some(bw),
                BUTTON_H,
                other,
                if ctx.enabled {
                    Style::Secondary
                } else {
                    Style::Disabled
                },
                fa(F::VaultExisting(v)),
            );
            cy += 40.0;
        }
        // The plan card's way on is the flow's Continue, the one way on
        // (§17.7); what holds it takes its place.
        if !ctx.enabled {
            continue;
        }
        if let Some(line) = cards::stickless_line(app, &ctx) {
            cy += ui.wrap_as((x + 12.0, cy, w - 12.0), LABEL_L, Role::Warning, &line) + 12.0;
        }
        if let Some(line) = cards::held_line(app, &ctx, None) {
            if ui.pinning {
                ui.pin = Some((line, Style::Disabled, fa(F::Continue)));
            } else {
                cy += ui.wrap_as((x + 12.0, cy, w - 12.0), LABEL_L, Role::Warning, &line) + 12.0;
            }
            continue;
        }
        if crate::screens::next_button(
            ui,
            x + 12.0,
            cy,
            w - 12.0,
            FEN.backup_flow_continue_make,
            fa(F::Continue),
        ) {
            cy += 48.0;
        }
    }
    cy + 8.0 - y
}

/// A plan card's head in the flow: its title, and its answers while it is
/// closed; a press opens or closes it. Returns its height.
#[allow(clippy::too_many_arguments)]
fn card_head(
    ui: &mut Ui,
    x: f32,
    y: f32,
    w: f32,
    title: &str,
    summary: &str,
    open: bool,
    toggle: Action,
) -> f32 {
    let h = 44.0;
    ui.press(Shape::new(x, y, w, h, 8.0), Role::OnSurface.paint(), toggle);
    ui.stroke(x, y, w, h, 8.0, Role::OutlineVariant);
    let tw = ui.measure_as(LABEL_L, title).min(w * 0.5);
    let t = ui.fit_as(LABEL_L, title, w * 0.5);
    ui.text_mid_as((x + 12.0, y, h), LABEL_L, Role::OnSurface, &t);
    if !open {
        let s = ui.fit_as(BODY_S, summary, w - tw - 36.0);
        ui.text_right_as((x + w - 12.0, y, h), BODY_S, Role::OnSurfaceVariant, &s);
    }
    h + 8.0
}

/// The Make card of a wallet's flow: the keys are made in Make a new
/// wallet, not here.
pub(crate) fn make_body(app: &View, f: &BackupFlow, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    let mut cy = y;
    if f.made {
        let name = f.subject.as_ref().map_or(String::new(), |s| s.name.clone());
        cy += ui.wrap_as((x, cy, w), TITLE_M, Role::Ok, &name) + 12.0;
        for &v in &f.made_in {
            cy += ui.wrap_as(
                (x, cy, w),
                BODY_M,
                Role::OnSurface,
                &fill(FEN.backup_flow_in_vault, &[&BackupFlow::vault_name(v)]),
            ) + 10.0;
        }
        if f.adding {
            cy += ui.wrap_as(
                (x, cy, w),
                BODY_M,
                Role::OnSurfaceVariant,
                FEN.backup_flow_copy_across,
            ) + 10.0;
            // A seed kept only on paper is typed back from its copy.
            let missing = app.wallet_round_missing(app.flow_index_of(f));
            if !missing.is_empty() {
                cy += ui.wrap_as(
                    (x, cy, w),
                    LABEL_L,
                    Role::Warning,
                    &fill(FEN.backup_flow_on_paper_only, &[&missing.join(", ")]),
                ) + 10.0;
                ui.button(
                    x,
                    cy,
                    None,
                    BUTTON_H,
                    FEN.backup_flow_add_from_copy,
                    Style::Secondary,
                    Action::Entry(None),
                );
                cy += 52.0;
            }
            // A codex32 string the copy stopped on, not held since the
            // power-on: typed here from its copy (`docs/CODEX32.md` §9.5).
            if let Some(want) = f.live.c32_wanted.as_ref() {
                use faraday_draw::tokens::{BUTTON_H, GAP, GAP_SMALL, LABEL_L};
                let line = fill(FEN.c32_round_not_here, &[&want.share(), &want.id_text()]);
                cy += ui.wrap_as((x, cy, w), LABEL_L, Role::Warning, &line) + GAP;
                let label = fill(FEN.c32_round_type, &[&want.share()]);
                let style = Style::Secondary;
                ui.button(x, cy, None, BUTTON_H, &label, style, fa(F::TypeString));
                cy += BUTTON_H + GAP + GAP_SMALL;
            }
            ui.button(
                x,
                cy,
                None,
                BUTTON_H,
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
                BUTTON_H,
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
        BUTTON_H,
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
    next: Option<Action>,
) -> f32 {
    let mut cy = y;
    cy += ui.wrap_as(
        (x, cy, w),
        BODY_M,
        Role::OnSurface,
        FEN.backup_flow_use_vaults_question,
    ) + 6.0;
    // A vault on a stick is read before the keys only: after them the
    // lock a new stick needs would wipe them (`docs/BACKUP-FLOW.md` §17.21
    // item 5).
    cy += ui.wrap_as(
        (x, cy, w),
        BODY_S,
        Role::OnSurfaceVariant,
        FEN.backup_flow_vaults_on_sticks,
    ) + 14.0;
    let style = |on: bool| {
        if locked {
            Style::Disabled
        } else {
            Style::chosen(on)
        }
    };
    let bw = 96.0;
    ui.button(
        x,
        cy,
        Some(bw),
        BUTTON_H,
        FEN.plan_yes,
        style(ask.answer == Some(true)),
        yes,
    );
    ui.button(
        x + bw + 10.0,
        cy,
        Some(bw),
        BUTTON_H,
        FEN.plan_no,
        style(ask.answer == Some(false)),
        no,
    );
    cy += 52.0;
    // Where an answer moves the flow on by itself there is no Continue
    // (F103).
    if let (false, Some(next)) = (locked, next) {
        let label = FEN.create_continue;
        let lw = ui.measure_as(LABEL_L, label) + 36.0;
        // Nothing is asked of the person until they have answered.
        let style = Style::Primary.when(ask.answer.is_some());
        ui.button(x + w - lw, cy, Some(lw), BUTTON_H, label, style, next);
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
    cy += ui.wrap_as(
        (x, cy, w),
        BODY_M,
        Role::OnSurface,
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
        let lw = ui.measure_as(LABEL_L, label) + 36.0;
        ui.button(
            x + w - lw,
            cy,
            Some(lw),
            BUTTON_H,
            label,
            Style::Primary,
            next,
        );
        cy += 48.0;
    }
    cy - y
}

/// Make a new wallet's question of vaults.
pub(crate) fn create_where(app: &View, ui: &mut Ui, x: f32, y: f32, w: f32) -> Option<f32> {
    let c = app.create.state()?;
    Some(ask_card(
        ui,
        (x, y, w),
        c.vaults,
        c.built.is_some(),
        (Action::CVaults(true), Action::CVaults(false)),
        Some(Action::CNext(crate::cstep::WHERE)),
    ))
}

/// Make a new wallet's count of vaults.
pub(crate) fn create_count(app: &View, ui: &mut Ui, x: f32, y: f32, w: f32) -> Option<f32> {
    let c = app.create.state()?;
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
pub(crate) fn restore_where(app: &View, ui: &mut Ui, x: f32, y: f32, w: f32) -> Option<f32> {
    let r = app.restore.state()?;
    Some(ask_card(
        ui,
        (x, y, w),
        r.vaults,
        r.done[crate::rstep::SEEDS as usize],
        (Action::RVaults(true), Action::RVaults(false)),
        None,
    ))
}

/// Restore's count of vaults.
pub(crate) fn restore_count(app: &View, ui: &mut Ui, x: f32, y: f32, w: f32) -> Option<f32> {
    let r = app.restore.state()?;
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
    app: &View,
    ui: &mut Ui,
    step: Step,
    x: f32,
    y: f32,
    w: f32,
) -> Option<f32> {
    let i = app.wallet_flow_of_owner(app.create.state()?.owner)?;
    let f = &app.flows[i];
    let mut cy = y;
    let line = match step {
        Step::Sticks => {
            let (read, of) = f.setup_sticks_read();
            fill(
                FEN.backup_flow_read_count,
                &[&read.to_string(), &of.to_string()],
            )
        }
        _ => {
            let used = f.setup_vaults();
            let open = used
                .iter()
                .filter(|&&v| app.flow_open_index(i, v).is_some())
                .count();
            fill(
                FEN.backup_flow_unlocked_count,
                &[&open.to_string(), &used.len().to_string()],
            )
        }
    };
    cy += ui.wrap_as((x, cy, w), BODY_M, Role::OnSurface, &line) + 14.0;
    let label = FEN.backup_flow_change;
    let bw = ui.measure_as(LABEL_L, label) + 36.0;
    ui.button(
        x + w - bw,
        cy,
        Some(bw),
        BUTTON_H,
        label,
        Style::Secondary,
        fa(F::Show(i)),
    );
    cy += 48.0;
    Some(cy - y)
}
