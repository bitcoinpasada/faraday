//! Check a multisig setup's screen: the step-card column over
//! [`crate::multisig_check`].

use super::{MsAction as M, MsState, SeedAnswer, Shown, mstep, state};
use crate::Action;
use crate::drawing::{Scrolled, View};
use crate::flow;
use crate::strings::EN;
use crate::ui::pal::Role;
use crate::ui::{Style, Ui};
use faraday_draw::tokens::{self, BODY_M, BODY_S, BUTTON_H, GAP, ICON_GLYPH, PAD};
use faraday_draw::widgets::Icon;
use faraday_text::Text;

pub(crate) const TITLES: [&str; mstep::COUNT] = [EN.mscheck_setup, EN.mscheck_result];

pub(crate) fn draw(app: &View, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    let Some(s) = state(app) else { return };
    let cards: Vec<flow::Card> = (0..mstep::COUNT as u8)
        .map(|step| flow::Card {
            title: TITLES[usize::from(step)].to_string(),
            summary: summary(s, step),
            mono: false,
            done: s.done(step),
            open: s.open == Some(step),
            default: step == mstep::SETUP && s.shown.is_some(),
            toggle: Action::MultisigCheck(M::Step(step)),
            lead: None,
        })
        .collect();
    let back_name = app.back_name(s.back);
    let col = flow::Column {
        area_x: x0,
        area_w: cw,
        x: x0 + tokens::FLOW_INSET,
        w: (cw - tokens::FLOW_INSET - tokens::FLOW_INSET).min(tokens::FLOW_MAX),
        h,
        back: Some(app.named_back((back_name, Action::Nav(s.back)))),
        heading: EN.mscheck_title,
        wrap_closed: false,
        total_fixed: true,
        note: None,
        chip: None,
        chip_tap: None,
        moves: crate::ui::Moves::No,
    };
    let scroll = s.scroll;
    let (next, again) = {
        let app_ref: &View = app;
        let mut body = |ui: &mut Ui, i: usize, x: f32, y: f32, w: f32| -> f32 {
            match state(app_ref) {
                Some(s) => card(app_ref, s, ui, i as u8, (x, y, w)),
                None => 0.0,
            }
        };
        flow::column(ui, &col, &cards, scroll, &mut body)
    };
    app.scroll(Scrolled::MultisigCheck, next, again);
}

fn summary(s: &MsState, step: u8) -> String {
    match (step, &s.shown) {
        (mstep::SETUP, Some(_)) => s.source.as_ref().map(|t| String::from(&**t)),
        (_, Some(r)) => Some(String::from(&*r.rows.shape)),
        _ => None,
    }
    .unwrap_or_default()
}

fn card(app: &View, s: &MsState, ui: &mut Ui, step: u8, at: (f32, f32, f32)) -> f32 {
    match step {
        mstep::SETUP => setup(app, s, ui, at),
        _ => result(app, s, ui, at),
    }
}

fn setup(app: &View, s: &MsState, ui: &mut Ui, (x, y, w): (f32, f32, f32)) -> f32 {
    let mut cy = 0.0;
    // Use a sample goes in the "Demo only" card (`docs/DEMO.md` §14).
    if super::sample_shown(app) {
        crate::samples::put::offer(ui, Action::UseSample);
    }
    if super::paste_shown(app) {
        ui.demo_offer(EN.mscheck_paste, Action::MultisigCheck(M::Paste));
    }
    let files = super::waiting(app);
    let loaded = super::loaded(app);
    if files.is_empty() && loaded.is_empty() {
        cy += crate::screens::parts::nothing_to_work_on(
            app,
            ui,
            (x, y + cy, w),
            EN.mscheck_nothing_waits,
            (
                Some(Action::MultisigCheck(M::Scan)),
                crate::Screen::MultisigCheck,
            ),
        ) + GAP;
    } else {
        ui.button(
            x,
            y + cy,
            None,
            BUTTON_H,
            EN.mscheck_scan,
            Style::Secondary,
            Action::MultisigCheck(M::Scan),
        );
        cy += BUTTON_H + GAP;
        if !files.is_empty() {
            let rows: Vec<(&str, Action)> = files
                .iter()
                .map(|&k| (&*app.inbox[k].name, Action::MultisigCheck(M::Inbox(k))))
                .collect();
            cy += crate::screens::parts::file_choices(
                ui,
                (x, y + cy, w),
                EN.mscheck_choose_setup,
                &rows,
            ) + GAP;
        }
    }
    if !loaded.is_empty() {
        cy += ui.wrap_as(
            (x, y + cy, w),
            BODY_S,
            Role::OnSurfaceVariant,
            EN.mscheck_loaded,
        ) + GAP;
        for i in loaded {
            let Some(wallet) = app.session.wallets.get(i) else {
                continue;
            };
            ui.button(
                x,
                y + cy,
                Some(w),
                BUTTON_H,
                &wallet.name,
                Style::Secondary,
                Action::MultisigCheck(M::Loaded(i)),
            );
            cy += BUTTON_H + GAP;
        }
    }
    if let Some(why) = &s.refused {
        cy += ui.wrap_as((x, y + cy, w), BODY_M, Role::Error, why) + GAP;
    }
    cy - GAP
}

fn result(app: &View, s: &MsState, ui: &mut Ui, at: (f32, f32, f32)) -> f32 {
    let (x, y, w) = at;
    let Some(r) = &s.shown else {
        return ui.wrap_as(
            (x, y, w),
            BODY_S,
            Role::OnSurfaceVariant,
            EN.mscheck_result_waits,
        );
    };
    let mut cy = rows(r, s.seed.as_ref(), ui, (x, y, w));
    cy += GAP;
    if let Some(name) = &r.loaded {
        cy += ui.wrap_as(
            (x, y + cy, w),
            BODY_M,
            Role::Ok,
            &Text::fill(EN.mscheck_loaded_as, &[name]),
        ) + GAP;
    }
    ui.button(
        x,
        y + cy,
        None,
        BUTTON_H,
        EN.mscheck_check_seed,
        Style::Ghost,
        Action::MultisigCheck(M::CheckSeed),
    );
    cy += BUTTON_H + GAP;
    cy += seed(app, s, ui, (x, y + cy, w));
    let (next, style, action) = match r.place {
        Some(i) => (EN.made_wallet_open, Style::Primary, Action::OpenWallet(i)),
        None => (
            EN.mscheck_add,
            Style::Primary,
            Action::MultisigCheck(M::Add),
        ),
    };
    ui.button(x, y + cy, None, BUTTON_H, next, style, action);
    cy += BUTTON_H + GAP;
    ui.button(
        x,
        y + cy,
        None,
        BUTTON_H,
        EN.mscheck_another,
        Style::Ghost,
        Action::MultisigCheck(M::Another),
    );
    cy += BUTTON_H + GAP;
    ui.button(
        x,
        y + cy,
        None,
        BUTTON_H,
        EN.mscheck_done,
        Style::Ghost,
        Action::Nav(s.back),
    );
    cy + BUTTON_H
}

/// The Result's rows: the confirmation sheet's, then the page's own.
fn rows(r: &Shown, seed: Option<&SeedAnswer>, ui: &mut Ui, (x, y, w): (f32, f32, f32)) -> f32 {
    use faraday_ui::sheet::kv;
    let ours: &[usize] = match seed {
        Some(SeedAnswer::Key(ours, _)) => ours,
        _ => &[],
    };
    let mut cy = crate::wallet::draw_rows(&r.rows, ours, ui, (x, y, w));
    if let Some(q) = &r.quorum {
        cy += kv(ui, x, y + cy, w, EN.mscheck_quorum, q, Role::OnSurface);
    }
    if r.ways_too_many {
        cy += ui.wrap_as(
            (x, y + cy, w),
            BODY_M,
            Role::OnSurface,
            EN.mscheck_ways_too_many,
        ) + GAP;
    } else if !r.ways.is_empty() {
        cy += ui.wrap_as(
            (x, y + cy, w),
            BODY_S,
            Role::OnSurfaceVariant,
            EN.mscheck_ways,
        ) + GAP;
        for line in &r.ways {
            cy += ui.wrap_as((x, y + cy, w), BODY_M, Role::OnSurface, line) + GAP;
        }
    }
    if let Some(a) = &r.first {
        cy += kv(
            ui,
            x,
            y + cy,
            w,
            EN.mscheck_first_receive,
            a,
            Role::OnSurface,
        );
    }
    cy
}

/// The seed given to Check a seed against it: its steps while it is being
/// taken, then its answer as one result line. The height used, with the
/// gap after it.
fn seed(app: &View, s: &MsState, ui: &mut Ui, (x, y, w): (f32, f32, f32)) -> f32 {
    let Some(flow) = app.tools.seed_check.state().filter(|f| f.for_check()) else {
        return 0.0;
    };
    let line = |ui: &mut Ui, cy: &mut f32, role: Role, text: &str| {
        *cy += ui.wrap_as((x, y + *cy, w), BODY_M, role, text) + GAP;
    };
    let warn = |ui: &mut Ui, why: &str, at: f32| {
        ui.icon(x, at, ICON_GLYPH, Icon::Warning, PAD, Role::Warning);
        let tw = w - ICON_GLYPH - PAD;
        ui.wrap_as((x + ICON_GLYPH + PAD, at, tw), BODY_M, Role::Warning, why) + GAP
    };
    let mut cy = 0.0;
    match &s.seed {
        None => {
            cy += crate::seed_check::screen::check_step(app, flow, ui, (x, y, w)) + GAP;
            if let Some(e) = flow.error() {
                line(ui, &mut cy, Role::Error, e);
            }
        }
        Some(SeedAnswer::Key(ours, warning)) => {
            let at: Vec<String> = ours.iter().map(|i| (i + 1).to_string()).collect();
            let at = Text::formatted(format_args!("{}", at.join(", ")));
            let line_text = if ours.len() == 1 {
                EN.mscheck_seed_is_key
            } else {
                EN.mscheck_seed_is_keys
            };
            line(ui, &mut cy, Role::Ok, &Text::fill(line_text, &[&at]));
            if let Some(why) = warning {
                cy += warn(ui, why, y + cy);
            }
        }
        Some(answer) => {
            line(ui, &mut cy, Role::Error, EN.app_not_wallet_key);
            match answer {
                SeedAnswer::Not(true) => {
                    line(
                        ui,
                        &mut cy,
                        Role::OnSurfaceVariant,
                        EN.seed_check_without_pass,
                    );
                }
                SeedAnswer::Changed(why) => cy += warn(ui, why, y + cy),
                _ => {}
            }
        }
    }
    cy
}
