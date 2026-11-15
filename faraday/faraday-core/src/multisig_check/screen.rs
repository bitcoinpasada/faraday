//! Check a multisig setup's screen: the step-card column over
//! [`crate::multisig_check`].

use super::{Mark, MsAction as M, MsState, SeedAnswer, Shown, mstep, state};
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
        (_, Some(r)) => Some(String::from(&*r.shape)),
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
    ui.button(
        x,
        y,
        None,
        BUTTON_H,
        EN.mscheck_scan,
        Style::Secondary,
        Action::MultisigCheck(M::Scan),
    );
    cy += BUTTON_H + GAP;
    cy += ui.wrap_as(
        (x, y + cy, w),
        BODY_S,
        Role::OnSurfaceVariant,
        EN.mscheck_from_files,
    ) + GAP;
    let mut any = false;
    for (k, item) in app.inbox.iter().enumerate() {
        let Some(policy) = item.wallet() else {
            continue;
        };
        any = true;
        let one = !super::is_setup(Some(policy));
        if one {
            let line = format!("{} · {}", &*item.name, EN.mscheck_one_key);
            cy += ui.wrap_as((x, y + cy, w), BODY_M, Role::OnSurfaceVariant, &line) + GAP;
        } else {
            ui.button(
                x,
                y + cy,
                Some(w),
                BUTTON_H,
                &item.name,
                Style::Secondary,
                Action::MultisigCheck(M::Inbox(k)),
            );
            cy += BUTTON_H + GAP;
        }
    }
    if !any {
        cy += ui.wrap_as(
            (x, y + cy, w),
            BODY_S,
            Role::OnSurfaceVariant,
            EN.mscheck_none_in_files,
        ) + GAP;
    }
    let loaded = super::loaded(app);
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
    ui.button(
        x,
        y + cy,
        None,
        BUTTON_H,
        EN.mscheck_done,
        Style::Primary,
        Action::Nav(s.back),
    );
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
    cy + BUTTON_H
}

/// The Result's rows: the confirmation sheet's, then the page's own.
fn rows(r: &Shown, seed: Option<&SeedAnswer>, ui: &mut Ui, (x, y, w): (f32, f32, f32)) -> f32 {
    use faraday_ui::sheet::kv;
    let mut cy = ui.wrap_as((x, y, w), BODY_M, Role::OnSurface, &r.shape) + GAP;
    for line in &r.repeats {
        cy += ui.wrap_as((x, y + cy, w), BODY_M, Role::Error, line) + GAP;
    }
    for (i, k) in r.keys.iter().enumerate() {
        let head = format!("{} · {}", &*k.fingerprint, &*k.path);
        cy += kv(ui, x, y + cy, w, &k.number, &head, Role::OnSurface);
        cy += ui.wrap_as((x, y + cy, w), BODY_M.mono(), Role::OnSurface, &k.xpub);
        match &k.mark {
            Mark::None => {}
            Mark::Yours(single) => {
                let line = if *single {
                    EN.mscheck_yours_single
                } else {
                    EN.mscheck_yours
                };
                cy += ui.wrap_as((x, y + cy, w), BODY_M, Role::Ok, line);
            }
            Mark::Wrong(line) => {
                cy += ui.wrap_as((x, y + cy, w), BODY_M, Role::Error, line);
            }
        }
        if matches!(seed, Some(SeedAnswer::Key(ours, _)) if ours.contains(&i)) {
            cy += ui.wrap_as((x, y + cy, w), BODY_M, Role::Ok, EN.mscheck_this_seed);
        }
        cy += GAP;
    }
    cy += kv(
        ui,
        x,
        y + cy,
        w,
        EN.mscheck_checksum,
        &r.checksum,
        Role::OnSurface,
    );
    if let Some((line, danger)) = &r.key_path {
        let tone = if *danger {
            Role::Error
        } else {
            Role::OnSurface
        };
        cy += ui.wrap_as((x, y + cy, w), BODY_M, tone, line) + GAP;
    }
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
