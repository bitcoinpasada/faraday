//! A wallet's plan by place (`docs/BACKUP-FLOW.md` §17): its cards, their
//! rows and the sheet a row opens, drawn by one code for Back up's plan
//! and for a wallet flow's own plan card, so that there is one way to
//! edit a plan. What a press does is `lib.rs` (Back up) and
//! `backup_flow_wallet.rs` (the flow); both edit the map as `plan.rs`
//! says.

use crate::plan::{self, At, Form, Preset, Row};
use crate::screens::{
    SOFTWARE_ROWS, held_item, next_button, place_names, section_label, shares_card, stepper, ticked,
};
use crate::strings::EN;
use crate::ui::pal::*;
use crate::ui::{Style, Ui, W};
use crate::{Action, Faraday, PlanPress, qrow, qstep};
use opensigner_core::strings::fill;
use osk_ui::widgets::Icon;

/// The plan a set of cards draws, and whose it is.
pub(crate) struct PlanCtx {
    /// The wallet's shape.
    pub shape: plan::Shape,
    /// The plan.
    pub answers: plan::Answers,
    /// What each place is called; "Place 1" where it has no name.
    pub names: Vec<String>,
    /// What each vault of the plan is called.
    pub vault_titles: Vec<String>,
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
    /// A flow's places take names: before its lock, with a vault open
    /// and Back up open on the wallet.
    pub names_typed: bool,
}

impl PlanCtx {
    /// Back up's plan.
    pub(crate) fn backup(app: &Faraday) -> Option<PlanCtx> {
        let b = app.backup.as_ref()?;
        let shape = app.plan_shape(b.wallet);
        let a = b.answers.clone();
        let names = (0..a.places.max(1) + 1)
            .map(|p| app.place_name(p))
            .collect();
        let vault_titles = (0..a.vault_count(&shape))
            .map(|v| app.backup_vault_title(v))
            .collect();
        Some(PlanCtx {
            shape,
            answers: a,
            names,
            vault_titles,
            flow: None,
            enabled: true,
            wallet: Some(b.wallet),
            extras: b.extras.clone(),
            checklist: b.stage == crate::BStage::Checklist,
            names_typed: true,
        })
    }

    /// Wallet flow `i`'s plan, drawn in the flow.
    pub(crate) fn flow(app: &Faraday, i: usize) -> Option<PlanCtx> {
        let f = app.backup_flows.get(i)?;
        let wf = f.wallet.as_ref()?;
        let a = wf.answers.clone();
        // Before the lock, with Back up open on the wallet, the names
        // typed there (kept in the vaults alone); after it, "Place n"
        // (§17.7). The flow keeps no name.
        let backup_names = !f.made
            && app.backup.as_ref().is_some_and(|b| {
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
        let vault_titles = (0..a.vault_count(&wf.shape))
            .map(|v| fill(EN.plan_vault, &[&(v + 1).to_string()]))
            .collect();
        Some(PlanCtx {
            shape: wf.shape.clone(),
            answers: a,
            names,
            vault_titles,
            flow: Some(i),
            enabled: f.live.writing.is_none(),
            wallet: None,
            extras: Vec::new(),
            checklist: false,
            names_typed: backup_names && !app.vaults.open.is_empty(),
        })
    }

    /// The plan the open row's sheet edits.
    pub(crate) fn of_choosing(app: &Faraday) -> Option<PlanCtx> {
        match app.choosing.as_ref()?.flow {
            Some(i) => PlanCtx::flow(app, i),
            None => PlanCtx::backup(app),
        }
    }

    /// Place `p`'s name.
    pub(crate) fn name(&self, p: usize) -> String {
        self.names
            .get(p)
            .cloned()
            .unwrap_or_else(|| fill(EN.place_name_place, &[&(p + 1).to_string()]))
    }

    /// Vault `v`'s title.
    fn vault_title(&self, v: usize) -> String {
        self.vault_titles
            .get(v)
            .cloned()
            .unwrap_or_else(|| fill(EN.plan_vault, &[&(v + 1).to_string()]))
    }

    /// A press as this plan's own action.
    pub(crate) fn act(&self, press: PlanPress) -> Action {
        match self.flow {
            None => press.backup(),
            Some(_) => Action::Flow(crate::backup_flow_steps::FlowAction::Plan(press)),
        }
    }
}

/// A plan card's title (`docs/BACKUP-FLOW.md` §17.1): a place's card is
/// titled with the place's name.
pub(crate) fn title(ctx: &PlanCtx, q: u8) -> String {
    if let Some(p) = qstep::place_of(q) {
        return ctx.name(p);
    }
    match q {
        qstep::PRESET => EN.question_title_start,
        qstep::PLACES => EN.question_title_places,
        qstep::DESCRIPTION => EN.backup_flow_the_wallet_description,
        qstep::WHERE => EN.backup_flow_where_the_description,
        qstep::SOFTWARE => EN.question_title_software,
        _ => EN.backup_flow_the_plan,
    }
    .to_string()
}

/// A place's card's rows, in order: the paper forms, each vault, the seed
/// file, and the passphrase where a seed here has one. A wallet with no
/// seed here has its vaults' rows alone.
fn place_card_rows(ctx: &PlanCtx) -> Vec<(Row, String)> {
    let shape = &ctx.shape;
    let seeds = !shape.watch_only();
    let mut out = Vec::new();
    if seeds {
        out.push((Row::Words, EN.backup_flow_seed_words_by_hand.to_string()));
        out.push((Row::SeedQr, EN.backup_flow_seedqr_by_hand.to_string()));
    }
    for v in 0..ctx.answers.vault_count(shape) {
        out.push((Row::Vault(v), ctx.vault_title(v)));
    }
    if seeds {
        out.push((Row::File, EN.backup_flow_seed_as_a_file.to_string()));
    }
    if shape.seeds.iter().any(|s| s.here && s.passphrase) {
        out.push((Row::Passphrase, EN.backup_flow_passphrase_row.to_string()));
    }
    out
}

/// The forms the description takes at place `p`, as Where the
/// description goes offers them: on paper, as a file, and in each vault
/// whose stick the place keeps.
fn where_forms(ctx: &PlanCtx, p: usize) -> Vec<(Form, String)> {
    let a = &ctx.answers;
    let mut out = vec![
        (Form::Paper, EN.backup_flow_on_paper.to_string()),
        (Form::File, EN.backup_flow_as_a_file.to_string()),
    ];
    for v in 0..a.vault_count(&ctx.shape) {
        if a.row_seeds(&ctx.shape, p, Row::Vault(v)).is_some() {
            out.push((
                Form::Vault(v),
                fill(EN.backup_flow_in_vault, &[&ctx.vault_title(v)]),
            ));
        }
    }
    out
}

/// A plan card's line while it is closed: its answers.
pub(crate) fn summary(ctx: &PlanCtx, q: u8) -> String {
    let (shape, a) = (&ctx.shape, &ctx.answers);
    let joined = |v: Vec<String>| {
        if v.is_empty() {
            EN.ticked_none.to_string()
        } else {
            v.join(", ")
        }
    };
    if let Some(p) = qstep::place_of(q) {
        return joined(
            place_card_rows(ctx)
                .into_iter()
                .filter(|(r, _)| a.row_seeds(shape, p, *r).is_some())
                .map(|(_, l)| l)
                .collect(),
        );
    }
    match q {
        qstep::PRESET => EN.question_summary_3.to_string(),
        qstep::PLACES => format!(
            "{} {}",
            a.places,
            if a.places == 1 {
                EN.question_summary_place
            } else {
                EN.question_summary_places
            }
        ),
        qstep::DESCRIPTION => if a.split && shape.splits {
            EN.backup_flow_in_shares
        } else {
            EN.backup_flow_whole
        }
        .to_string(),
        qstep::WHERE => {
            let mut v: Vec<String> = Vec::new();
            for p in 0..a.places {
                for (f, l) in where_forms(ctx, p) {
                    if a.form_at(shape, p, f) && !v.contains(&l) {
                        v.push(l);
                    }
                }
            }
            if a.software_on(shape) {
                v.push(EN.backup_flow_into_watch_only.to_string());
            }
            joined(v)
        }
        qstep::SOFTWARE => ticked(&SOFTWARE_ROWS, &a.software),
        _ => {
            let c = plan::check(shape, a);
            fill(EN.question_summary_any, &[c.lost.text()])
        }
    }
}

/// What holds The plan's way on (`docs/NEW-WALLET.md` §14.2, §14.4): a
/// vault's stick at no place, a vault that holds nothing, a key made here
/// kept nowhere (`kept_nowhere`, the caller's).
pub(crate) fn held_line(
    app: &Faraday,
    ctx: &PlanCtx,
    kept_nowhere: Option<String>,
) -> Option<String> {
    if let Some(v) = ctx.answers.stickless(&ctx.shape) {
        return Some(fill(
            EN.plan_vault_s_needs,
            &[&(v + 1).to_string(), app.medium.noun()],
        ));
    }
    if let Some(v) = ctx.answers.empty_vault() {
        return Some(fill(
            EN.backup_flow_vault_holds_nothing,
            &[&(v + 1).to_string()],
        ));
    }
    kept_nowhere
}

/// The open card of a plan (`docs/BACKUP-FLOW.md` §17.1): its lists, then
/// Continue to the card after it. The plan's own card draws the map and
/// its checks and leaves the way on to the caller. Returns its height.
pub(crate) fn body(
    app: &Faraday,
    ui: &mut Ui,
    ctx: &PlanCtx,
    q: u8,
    x: f32,
    y: f32,
    w: f32,
) -> f32 {
    let (shape, a) = (&ctx.shape, &ctx.answers);
    let on = ctx.enabled;
    let act = |p: PlanPress| ctx.act(p);
    let mut cy = y;
    if let Some(p) = qstep::place_of(q) {
        // What the place keeps, a row each; a ticked row says what under
        // its label.
        let pc = p.min(255) as u8;
        let rows: Vec<PlaceRow> = place_card_rows(ctx)
            .into_iter()
            .map(|(row, label)| {
                let held = a.row_seeds(shape, p, row);
                // A vault's row says all the vault holds: its seeds, the
                // description, shares, or that it holds nothing yet.
                let sub = match (row, held.as_ref()) {
                    (Row::Vault(v), Some(_)) => {
                        let holds: Vec<String> = plan::map(shape, a)
                            .into_iter()
                            .filter(|s| s.at == At::Vault(v))
                            .flat_map(|s| s.holds)
                            .map(|(w, _)| match w {
                                plan::What::Seed(i) | plan::What::SeedPassphrase(i) => {
                                    shape.seeds[i].name.clone()
                                }
                                w => w.label(shape, app.medium),
                            })
                            .collect();
                        Some(if holds.is_empty() {
                            EN.map_rows_nothing.to_string()
                        } else {
                            holds.join(", ")
                        })
                    }
                    (_, Some(v)) => Some(
                        v.iter()
                            .map(|&i| shape.seeds[i].name.clone())
                            .collect::<Vec<_>>()
                            .join(", "),
                    ),
                    _ => None,
                };
                PlaceRow {
                    label,
                    sub,
                    on: held.is_some(),
                    open: act(PlanPress::Row(pc, row.code())),
                    off: act(PlanPress::RowOff(pc, row.code())),
                }
            })
            .collect();
        cy += place_rows(ui, x, cy, w, &rows, on) + 10.0;
        // The place's own line of fact: what finding it alone gives.
        let alone = plan::alone(shape, a, At::Place(p));
        cy += ui.wrap(
            x,
            cy,
            w,
            13.0,
            W::S,
            crate::glance::alone_color(alone.kind()),
            alone.text(),
        ) + 12.0;
    } else {
        match q {
            qstep::PRESET => {
                // F1's one line of fact, where a key made here waits for
                // this backup (`docs/NEW-WALLET.md` §14.5).
                if ctx.wallet.is_some_and(|wl| !app.wallet_held(wl).is_empty()) {
                    cy += ui.wrap(x, cy, w, 13.0, W::R, MUTED, EN.plan_a_key_made_here) + 12.0;
                }
                for (k, p) in Preset::ALL.iter().enumerate() {
                    ui.button(
                        x,
                        cy,
                        Some(w),
                        40.0,
                        p.name(),
                        Style::Secondary,
                        Action::BPreset(k as u8),
                    );
                    cy += 48.0;
                }
                return cy - y;
            }
            qstep::PLACES => {
                stepper(
                    ui,
                    x,
                    cy,
                    w,
                    EN.backup_flow_number_of_places,
                    a.places,
                    act(PlanPress::Places(false)),
                    act(PlanPress::Places(true)),
                    on,
                );
                cy += 52.0;
                // The places' names live in the open vaults: typed on Back
                // up's plan, and in a flow's before its lock while Back up
                // is open on the wallet (§17.7).
                if ctx.flow.is_none() || ctx.names_typed {
                    cy += place_names(app, ui, x, cy, w);
                }
            }
            qstep::DESCRIPTION => {
                let split = a.split && shape.splits;
                let list = ui.multi_list(
                    x,
                    cy,
                    w,
                    &[
                        (
                            EN.backup_flow_whole.to_string(),
                            !split,
                            on,
                            act(PlanPress::Split(false)),
                        ),
                        (
                            EN.backup_flow_in_shares.to_string(),
                            split,
                            on,
                            act(PlanPress::Split(true)),
                        ),
                    ],
                );
                // Beside In shares, drawn over its row.
                let label = EN.plan_what_is_a_share;
                let bw = ui.measure(13.0, W::S, label) + 24.0;
                let rh = list / 2.0;
                ui.button(
                    x + w - bw,
                    cy + rh + (rh - 32.0) / 2.0,
                    Some(bw),
                    32.0,
                    label,
                    Style::Ghost,
                    Action::LearnShares,
                );
                cy += list + 12.0;
                // The slider, the shares and their keys, the audit: Back
                // up's from the wallet loaded, a flow's from its shape
                // (§17.7), each setting its own plan's keys left off.
                if split {
                    match (ctx.flow, ctx.wallet) {
                        (None, Some(wl)) => cy += shares_card(app, ui, x, cy, w, wl, a.omit),
                        (Some(_), _) => {
                            let fp = |j: usize| {
                                shape
                                    .seeds
                                    .get(j)
                                    .map_or_else(String::new, |s| s.name.to_lowercase())
                            };
                            let slider = if on {
                                crate::seeds::SLIDE_FLOW_OMIT
                            } else {
                                u8::MAX
                            };
                            cy += crate::screens::shares_table(
                                ui,
                                (x, cy, w),
                                (shape.m, shape.keys, a.omit),
                                &fp,
                                slider,
                            );
                        }
                        _ => {}
                    }
                }
            }
            qstep::WHERE => {
                let split = a.split && shape.splits;
                for p in 0..a.places {
                    let pc = p.min(255) as u8;
                    section_label(ui, x, cy, &ctx.name(p));
                    cy += 28.0;
                    let forms: Vec<(String, bool, bool, Action)> = where_forms(ctx, p)
                        .into_iter()
                        .map(|(f, l)| {
                            (
                                l,
                                a.form_at(shape, p, f),
                                on,
                                act(PlanPress::Form(pc, f.code())),
                            )
                        })
                        .collect();
                    cy += ui.multi_list(x, cy, w, &forms) + 8.0;
                    if split {
                        section_label(ui, x, cy, EN.backup_flow_shares_here);
                        cy += 28.0;
                        let here = a.shares_here(shape, p);
                        let shares: Vec<(String, bool, bool, Action)> = (0..shape.keys)
                            .map(|j| {
                                (
                                    fill(EN.shares_card_share, &[&(j + 1).to_string()]),
                                    here.contains(&j),
                                    on,
                                    act(PlanPress::Share(pc, j.min(255) as u8)),
                                )
                            })
                            .collect();
                        cy += ui.multi_list(x, cy, w, &shares) + 8.0;
                    }
                    cy += 6.0;
                }
                section_label(ui, x, cy, EN.backup_flow_not_a_place);
                cy += 28.0;
                let mut rows = vec![(
                    EN.backup_flow_into_watch_only.to_string(),
                    a.software_on(shape),
                    on,
                    act(PlanPress::Software),
                )];
                // What a plan saved before the cards keeps on the stick of
                // files, each to untick.
                let stick = fill(EN.map_rows_of_files, &[app.medium.cap()]);
                for (k, what) in a.old_files(shape).into_iter().enumerate() {
                    rows.push((
                        fill(
                            EN.backup_flow_pick_title,
                            &[&stick, &what.label(shape, app.medium)],
                        ),
                        true,
                        on,
                        act(PlanPress::OldFile(k.min(255) as u8)),
                    ));
                }
                cy += ui.multi_list(x, cy, w, &rows);
            }
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

/// A row of a place's card.
struct PlaceRow {
    /// What the place keeps that way.
    label: String,
    /// What it holds, under the label, once ticked.
    sub: Option<String>,
    /// Ticked.
    on: bool,
    /// The press on the label: its sheet.
    open: Action,
    /// The press on a ticked row's box: unticked.
    off: Action,
}

/// A place's card's list (`docs/DESIGN.md` §4.2, Multi choice): a row per
/// thing the place may keep, a box at its start. A ticked row shows what
/// it holds under its label; a press on the label opens its sheet, a
/// press on the box of a ticked row unticks it. Not `enabled`, the rows
/// are dimmed and take no press. Returns its height.
fn place_rows(ui: &mut Ui, x: f32, y: f32, w: f32, rows: &[PlaceRow], enabled: bool) -> f32 {
    let base = if ui.compact { 44.0 } else { 40.0 };
    let mut ry = y;
    for (k, r) in rows.iter().enumerate() {
        let rh = if r.sub.is_some() { base + 16.0 } else { base };
        let hot = enabled
            && (ui.is_pressed(r.open)
                || ui.is_hovered(r.open)
                || (r.on && (ui.is_pressed(r.off) || ui.is_hovered(r.off))));
        if hot {
            ui.fill(x, ry + 2.0, w, rh - 4.0, 8.0, INNER);
        }
        ui.checkbox(x + 10.0, ry + (base - 18.0) / 2.0, r.on, enabled);
        let shown = ui.fit(14.0, W::R, &r.label, w - 50.0);
        ui.text_mid(
            x + 40.0,
            ry,
            base,
            14.0,
            W::R,
            if enabled { TEXT } else { DIM },
            &shown,
        );
        if let Some(sub) = &r.sub {
            let sub = ui.fit(12.0, W::M, sub, w - 50.0);
            ui.text(x + 40.0, ry + base - 8.0, 12.0, W::M, MUTED, &sub);
        }
        if k + 1 < rows.len() {
            ui.rule(x + 40.0, ry + rh, w - 40.0, INNER);
        }
        if enabled {
            if r.on {
                ui.hit(x, ry, 40.0, rh, r.off);
                ui.hit(x + 40.0, ry, w - 40.0, rh, r.open);
            } else {
                ui.hit(x, ry, w, rh, r.open);
            }
        }
        ry += rh;
    }
    ry - y
}

/// A place's row's sheet (`docs/BACKUP-FLOW.md` §17.1), "{row} · {place}":
/// the seeds loaded here, any number ticked; for a vault, under each
/// ticked seed with a passphrase, that seed with it, and where else its
/// stick is kept; then **Done**. Back up's plan's or a wallet flow's.
pub(crate) fn pick_sheet(app: &Faraday, ui: &mut Ui, w: f32, h: f32) {
    let Some(c) = app.choosing.as_ref() else {
        return;
    };
    let Some(ctx) = PlanCtx::of_choosing(app) else {
        return;
    };
    let (shape, a) = (&ctx.shape, &ctx.answers);
    let label = place_card_rows(&ctx)
        .into_iter()
        .find(|(r, _)| *r == c.row)
        .map(|(_, l)| l)
        .unwrap_or_default();
    let title = fill(EN.backup_flow_pick_title, &[&label, &ctx.name(c.place)]);
    let words = a.words_at(shape, c.place);
    let mut items: Vec<(String, bool, bool, Action)> = Vec::new();
    for i in shape.here() {
        let seed = &shape.seeds[i];
        if c.row == Row::Passphrase && !seed.passphrase {
            continue;
        }
        // A passphrase is not kept beside its seed's words, nor its
        // words beside it.
        let enabled = !(c.row == Row::Passphrase && words.contains(&i))
            && !(matches!(c.row, Row::Words | Row::SeedQr) && a.pass_at(i, c.place));
        let on = c.seeds.contains(&i) && enabled;
        items.push((
            seed.name.clone(),
            on,
            enabled,
            Action::BPick(i.min(255) as u8),
        ));
        if matches!(c.row, Row::Vault(_)) && on && seed.passphrase {
            items.push((
                fill(EN.backup_flow_with_its_passphrase, &[&seed.name]),
                c.pass.contains(&i),
                true,
                Action::BPickPass(i.min(255) as u8),
            ));
        }
    }
    let also: Vec<String> = match c.row {
        Row::Vault(v) => a
            .stick_elsewhere(shape, v, c.place)
            .into_iter()
            .map(|p| ctx.name(p))
            .collect(),
        _ => Vec::new(),
    };
    let place = if ui.compact {
        (8.0, w - 16.0)
    } else {
        let sw = (w - 48.0).min(560.0);
        ((w - sw) / 2.0, sw)
    };
    let margin = if ui.compact { 8.0 } else { 32.0 };
    let pad = if ui.compact { 16.0 } else { 32.0 };
    crate::compact::scroll_sheet(ui, place, h, margin, pad, &mut |ui, x, y, iw| {
        let mut cy = y;
        if ui.compact {
            cy += crate::compact::sheet_head(ui, x, cy, iw, Icon::Keys, TEXT, &title);
        } else {
            ui.icon(x, cy, 30.0, Icon::Keys, 18.0, TEXT);
            let t = ui.fit(20.0, W::S, &title, iw - 42.0);
            ui.text_mid(x + 42.0, cy, 30.0, 20.0, W::S, TEXT, &t);
            cy += 46.0;
        }
        cy += ui.multi_list(x, cy, iw, &items) + 12.0;
        if !also.is_empty() {
            let line = fill(EN.backup_flow_also_at, &[&also.join(", ")]);
            cy += ui.wrap(x, cy, iw, 13.0, W::R, MUTED, &line) + 12.0;
        }
        cy += crate::compact::buttons(
            ui,
            x,
            cy,
            iw,
            &[(EN.backup_flow_pick_done, Style::Primary, Action::BPickDone)],
        );
        cy - y
    });
}

/// The map: one box per place, then the vault, a stick of files, the
/// software and the seeds on their own devices, each listing what it
/// holds and its tag; in Back up's checklist, each line marked by its
/// item's state. `boxed` draws each as a box, as the side panel does; not
/// boxed, the checks follow. Returns its height.
pub(crate) fn map_rows(
    app: &Faraday,
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
    for spot in plan::map(shape, a) {
        let name = match spot.at {
            At::Place(p) => ctx.name(p),
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
        let name = ui.fit(13.0, W::S, &name, iw);
        ui.text(ix, cy, 13.0, W::S, TEXT, &name);
        cy += 22.0;
        if spot.holds.is_empty() {
            ui.text(ix, cy, 12.0, W::R, DIM, EN.map_rows_nothing);
            cy += 20.0;
        }
        for (what, tag) in &spot.holds {
            let tone = match tag {
                Tag::Secret => WARN,
                Tag::Sealed => ACCENT,
                Tag::Public => MUTED,
            };
            let tw = ui.measure(11.0, W::R, tag.label());
            ui.text_right(ix + iw, cy, 18.0, 11.0, W::R, tone, tag.label());
            let mut lx = ix;
            if ctx.checklist {
                let state = held_item(spot.at, *what, &items).map(|it| app.backup_item_done(it));
                if let Some(done) = state {
                    ui.dot(ix + 4.0, cy + 9.0, 3.5, if done { OK } else { DIM });
                }
                lx += 14.0;
            }
            let label = ui.fit(
                12.0,
                W::R,
                &what.label(shape, app.medium),
                ix + iw - lx - tw - 8.0,
            );
            ui.text_mid(lx, cy, 18.0, 12.0, W::R, TEXT, &label);
            cy += 20.0;
        }
        if boxed {
            cy += 6.0;
            ui.stroke(x, top, w, cy - top, 8.0, LINE);
        }
        cy += 8.0;
    }
    // Work an earlier plan did that this one dropped, as it is: the copy
    // exists until it is destroyed (`docs/NEW-WALLET.md` §14.4).
    for line in &ctx.extras {
        let top = cy;
        let (ix, iw) = if boxed { (x + 12.0, w - 24.0) } else { (x, w) };
        if boxed {
            cy += 10.0;
        }
        let tag = plan::Tag::Sealed.name();
        let tw = ui.measure(11.0, W::R, tag);
        ui.text_right(ix + iw, cy, 18.0, 11.0, W::R, ACCENT, tag);
        cy += ui
            .wrap(ix, cy, iw - tw - 8.0, 12.0, W::R, TEXT, line)
            .max(18.0)
            + 2.0;
        if boxed {
            cy += 6.0;
            ui.stroke(x, top, w, cy - top, 8.0, LINE);
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
pub(crate) fn check_rows(app: &Faraday, ui: &mut Ui, ctx: &PlanCtx, x: f32, y: f32, w: f32) -> f32 {
    let c = plan::check(&ctx.shape, &ctx.answers);
    let name = |p: usize| ctx.name(p);
    let mut h = crate::glance::check_lines(ui, x, y, w, c, false, &name);
    // Why "Any one place lost" reads No (`docs/NEW-WALLET.md` §14.2).
    if let Some(why) = plan::lost_why(&ctx.shape, &ctx.answers, &name, app.medium.noun()) {
        h += ui.wrap(x, y + h, w, 12.0, W::R, ERR, &why) + 8.0;
    }
    h
}
