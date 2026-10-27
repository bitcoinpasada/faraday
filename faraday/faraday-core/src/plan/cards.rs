//! A wallet's plan (`docs/BACKUP-FLOW.md` §15, §17): its cards, drawn by
//! one code for Back up's plan and for a wallet flow's own plan card, so
//! that there is one way to edit a plan. The places and what each keeps
//! are one card, the board (§19, `board.rs`, `board/screen.rs`), on both
//! (§19.12 item 1). What a press on the other cards does is `lib.rs`
//! (Back up) and `backup_flow/wallet.rs` (the flow); both edit the map as
//! `plan.rs` says.

use crate::drawing::View;
use crate::plan::{self, At};
use crate::screens::{SOFTWARE_ROWS, held_item, next_button, section_label, ticked};
use crate::strings::EN;
use crate::strings::fill;
use crate::ui::Ui;
use crate::ui::pal::*;
use crate::{Action, PlanPress, qrow, qstep};
use faraday_draw::tokens::{BODY_S, LABEL_L};

/// The plan a set of cards draws, and whose it is.
pub(crate) struct PlanCtx {
    /// The wallet's shape.
    pub shape: plan::Shape,
    /// The plan.
    pub answers: plan::Answers,
    /// The plan mapped, with its check: once a frame through the drawing
    /// memo ([`Kept::plan_mapped`]).
    pub mapped: std::rc::Rc<plan::Mapped>,
    /// The plan's check, kept with its map ([`View::plan_check`]).
    pub check: plan::Check,
    /// What each place is called; "Place 1" where it has no name.
    pub names: Vec<String>,
    /// A wallet flow's plan, by index; `None` for Back up's.
    pub flow: Option<usize>,
    /// The rows take presses: not while a stick is written.
    pub enabled: bool,
    /// The loaded wallet, where it is: its shares' keys and the slider.
    pub wallet: Option<usize>,
    /// Work an earlier plan did that this one dropped.
    pub extras: Vec<String>,
    /// Back up's checklist is showing: the map's lines carry their
    /// items' marks.
    pub checklist: bool,
}

impl PlanCtx {
    /// Back up's plan.
    pub(crate) fn backup(app: &View) -> Option<PlanCtx> {
        let b = app.backup()?;
        let shape = app.plan_shape(b.wallet);
        let a = b.answers.clone();
        let names = (0..a.places.max(1) + 1)
            .map(|p| app.place_name(p))
            .collect();
        Some(PlanCtx {
            mapped: app.plan_mapped(&shape, &a),
            check: app.plan_check(&shape, &a),
            shape,
            answers: a,
            names,
            flow: None,
            enabled: true,
            wallet: Some(b.wallet),
            extras: b.extras.clone(),
            checklist: b.stage == crate::BStage::Checklist,
        })
    }

    /// Wallet flow `i`'s plan, drawn in the flow.
    pub(crate) fn flow(app: &View, i: usize) -> Option<PlanCtx> {
        let f = app.flows.get(i)?;
        let wf = f.wallet.as_ref()?;
        let a = wf.answers.clone();
        // Before the lock, with Back up open on the wallet, the names
        // typed there (kept in the vaults alone); after it, "Place n"
        // (§17.7). The flow keeps no name.
        let backup_names = !f.made
            && app.backup().is_some_and(|b| {
                app.session.wallets.get(b.wallet).map(|w| w.checksum()) == wf.checksum.as_deref()
            });
        let names = (0..a.places.max(1) + 1)
            .map(|p| {
                if backup_names {
                    app.place_name(p)
                } else {
                    crate::backup_flow::BackupFlow::place_name(p)
                }
            })
            .collect();
        Some(PlanCtx {
            mapped: app.plan_mapped(&wf.shape, &a),
            check: app.plan_check(&wf.shape, &a),
            shape: wf.shape.clone(),
            answers: a,
            names,
            flow: Some(i),
            enabled: f.live.writing.is_none(),
            wallet: None,
            extras: Vec::new(),
            checklist: false,
        })
    }

    /// Place `p`'s name.
    pub(crate) fn name(&self, p: usize) -> String {
        self.names
            .get(p)
            .cloned()
            .unwrap_or_else(|| fill(EN.place_name_place, &[&(p + 1).to_string()]))
    }

    /// A press as this plan's own action.
    pub(crate) fn act(&self, press: PlanPress) -> Action {
        match self.flow {
            None => press.backup(),
            Some(_) => Action::Flow(crate::backup_flow::steps::FlowAction::Plan(press)),
        }
    }
}

/// A plan card's title (`docs/BACKUP-FLOW.md` §15.1): a place's card is
/// titled with the place's name.
pub(crate) fn title(_ctx: &PlanCtx, q: u8) -> String {
    q_title(q)
}

/// A plan card's title by its number alone.
pub(crate) fn q_title(q: u8) -> String {
    match q {
        qstep::BOARD => EN.board_title,
        qstep::SOFTWARE => EN.question_title_software,
        _ => EN.backup_flow_the_plan,
    }
    .to_string()
}

/// A plan card's line while it is closed: its answers.
pub(crate) fn summary(ctx: &PlanCtx, q: u8) -> String {
    let a = &ctx.answers;
    match q {
        qstep::BOARD => {
            if a.places == 1 {
                EN.board_one_place.to_string()
            } else {
                fill(EN.board_places, &[&a.places.to_string()])
            }
        }
        qstep::SOFTWARE => ticked(&SOFTWARE_ROWS, &a.software),
        _ => {
            let c = ctx.check;
            let lost = if c.single && c.lost == plan::Lost::No {
                EN.plan_no_single_place
            } else {
                c.lost.text()
            };
            fill(EN.question_summary_any, &[lost])
        }
    }
}

/// The warning over The plan's way on, which does not hold it: a vault
/// that exists already whose stick no place keeps
/// (`docs/BACKUP-FLOW.md` §17.16 item 3, §19.5).
pub(crate) fn stickless_line(app: &View, ctx: &PlanCtx) -> Option<String> {
    let v = ctx.answers.stickless(&ctx.shape)?;
    Some(fill(
        EN.plan_vault_s_needs,
        &[&(v + 1).to_string(), app.medium.noun()],
    ))
}

/// What holds The plan's way on (`docs/history/NEW-WALLET.md` §14.2, §14.4): a
/// vault that holds nothing, a key made here kept nowhere
/// (`kept_nowhere`, the caller's). A vault's stick at no place is a
/// warning only ([`stickless_line`], `docs/BACKUP-FLOW.md` §17.16).
pub(crate) fn held_line(
    _app: &View,
    ctx: &PlanCtx,
    kept_nowhere: Option<String>,
) -> Option<String> {
    if let Some(v) = ctx.answers.empty_vault() {
        return Some(fill(
            EN.backup_flow_vault_holds_nothing,
            &[&(v + 1).to_string()],
        ));
    }
    kept_nowhere
}

/// The open card of a plan (`docs/BACKUP-FLOW.md` §15.1): its lists, then
/// Continue to the card after it. The plan's own card draws the map and
/// its checks and leaves the way on to the caller. Returns its height.
pub(crate) fn body(app: &View, ui: &mut Ui, ctx: &PlanCtx, q: u8, x: f32, y: f32, w: f32) -> f32 {
    let a = &ctx.answers;
    let on = ctx.enabled;
    let act = |p: PlanPress| ctx.act(p);
    let mut cy = y;
    // F1's one line of fact, where a key made here waits for this backup
    // (`docs/history/NEW-WALLET.md` §14.5): over the board too, which a new plan
    // opens on.
    let held = ctx.wallet.is_some_and(|wl| !app.wallet_held(wl).is_empty());
    if q == qstep::BOARD {
        if held {
            cy += ui.wrap_as(
                (x, cy, w),
                BODY_S,
                Role::OnSurfaceVariant,
                EN.plan_a_key_made_here,
            ) + 12.0;
        }
        // The board (`docs/BACKUP-FLOW.md` §17), then Continue.
        cy += crate::board::screen::board(app, ui, x, cy, w);
    } else {
        match q {
            qstep::SOFTWARE => {
                let rows = |list: u8, labels: &[&str], ticked: &[bool]| {
                    labels
                        .iter()
                        .zip(ticked)
                        .enumerate()
                        .map(|(r, (l, o))| {
                            (l.to_string(), *o, on, act(PlanPress::Answer(list, r as u8)))
                        })
                        .collect::<Vec<_>>()
                };
                cy += ui.multi_list(x, cy, w, &rows(qrow::SOFTWARE, &SOFTWARE_ROWS, &a.software))
                    + 12.0;
                section_label(ui, x, cy, EN.plan_form);
                cy += 28.0;
                cy += ui.multi_list(
                    x,
                    cy,
                    w,
                    &rows(qrow::FORM, &[EN.plan_qr_picture, EN.plan_text], &a.form),
                );
            }
            _ => {
                cy += map_rows(app, ui, ctx, x, cy, w, false);
                return cy - y;
            }
        }
    }
    cy += 8.0;
    if on && next_button(ui, x, cy, w, EN.plan_continue, act(PlanPress::Next(q))) {
        cy += 48.0;
    }
    cy - y
}

/// The map: one box per place, then the vault, a stick of files, the
/// software and the seeds on their own devices, each listing what it
/// holds and its tag; in Back up's checklist, each line marked by its
/// item's state. `boxed` draws each as a box, as the side panel does; not
/// boxed, the checks follow. Returns its height.
pub(crate) fn map_rows(
    app: &View,
    ui: &mut Ui,
    ctx: &PlanCtx,
    x: f32,
    y: f32,
    w: f32,
    boxed: bool,
) -> f32 {
    use crate::plan::Tag;
    let (shape, a) = (&ctx.shape, &ctx.answers);
    let items = if ctx.checklist {
        app.backup_items()
    } else {
        Vec::new()
    };
    let mut cy = y;
    // Each place with what its sticks keep.
    for spot in &ctx.mapped.places {
        let name = match spot.at {
            At::Place(p) | At::Stick(p, _) => ctx.name(p),
            At::Vault(v) => a.vault_name(shape, v),
            At::Files => fill(EN.map_rows_of_files, &[app.medium.cap()]),
            At::Software => EN.map_rows_watch_only.to_string(),
            At::Away => EN.map_rows_on_its_own.to_string(),
        };
        let top = cy;
        let (ix, iw) = if boxed { (x + 12.0, w - 24.0) } else { (x, w) };
        if boxed {
            cy += 10.0;
        }
        let name = ui.fit_as(LABEL_L, &name, iw);
        ui.text_as(ix, cy, LABEL_L, Role::OnSurface, &name);
        cy += 22.0;
        if spot.holds.is_empty() {
            ui.text_as(ix, cy, BODY_S, Role::OnSurfaceVariant, EN.map_rows_nothing);
            cy += 20.0;
        }
        for (what, tag) in &spot.holds {
            let tone = match tag {
                Tag::Secret => Role::Warning,
                Tag::Sealed => Role::Primary,
                Tag::Public => Role::OnSurfaceVariant,
            };
            let tw = ui.measure_as(BODY_S, tag.label());
            ui.text_right_as((ix + iw, cy, 18.0), BODY_S, tone, tag.label());
            let mut lx = ix;
            if ctx.checklist {
                let state = held_item(spot.at, *what, &items).map(|it| app.backup_item_done(it));
                if let Some(done) = state {
                    ui.dot(
                        ix + 4.0,
                        cy + 9.0,
                        3.5,
                        if done {
                            Role::Ok
                        } else {
                            Role::OnSurfaceVariant
                        },
                    );
                }
                lx += 14.0;
            }
            let label = ui.fit_as(
                BODY_S,
                &what.label(shape, app.medium),
                ix + iw - lx - tw - 8.0,
            );
            ui.text_mid_as((lx, cy, 18.0), BODY_S, Role::OnSurface, &label);
            cy += 20.0;
        }
        if boxed {
            cy += 6.0;
            ui.stroke(x, top, w, cy - top, 8.0, Role::OutlineVariant);
        }
        cy += 8.0;
    }
    // Work an earlier plan did that this one dropped, as it is: the copy
    // exists until it is destroyed (`docs/history/NEW-WALLET.md` §14.4).
    for line in &ctx.extras {
        let top = cy;
        let (ix, iw) = if boxed { (x + 12.0, w - 24.0) } else { (x, w) };
        if boxed {
            cy += 10.0;
        }
        let tag = plan::Tag::Sealed.name();
        let tw = ui.measure_as(BODY_S, tag);
        ui.text_right_as((ix + iw, cy, 18.0), BODY_S, Role::Primary, tag);
        cy += ui
            .wrap_as((ix, cy, iw - tw - 8.0), BODY_S, Role::OnSurface, line)
            .max(18.0)
            + 2.0;
        if boxed {
            cy += 6.0;
            ui.stroke(x, top, w, cy - top, 8.0, Role::OutlineVariant);
        }
        cy += 8.0;
    }
    if !boxed {
        cy += 4.0;
        cy += check_rows(app, ui, ctx, x, cy, w);
    }
    cy - y
}

/// The check under the map: its three lines, each with its value, and
/// why "Any one place lost" reads No. Returns its height.
pub(crate) fn check_rows(app: &View, ui: &mut Ui, ctx: &PlanCtx, x: f32, y: f32, w: f32) -> f32 {
    let c = ctx.check;
    let name = |p: usize| ctx.name(p);
    let mut h = crate::glance::check_lines(ui, x, y, w, c, false, &name);
    // Why "Any one place lost" reads No (`docs/history/NEW-WALLET.md` §14.2).
    if let Some(why) = plan::lost_why_on(
        &ctx.shape,
        &ctx.answers,
        &ctx.mapped,
        &name,
        app.medium.noun(),
    ) {
        h += ui.wrap_as((x, y + h, w), BODY_S, Role::Error, &why) + 8.0;
    }
    h
}
