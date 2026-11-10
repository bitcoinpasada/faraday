//! Read a Bitcoin Core wallet's screen: the step-card column over
//! [`crate::core_wallet::tool`].

use super::tool::{CoreWalletAction as A, CoreWalletState, Verdict, cstep, state};
use crate::Action;
use crate::drawing::{Scrolled, View};
use crate::flow;
use crate::strings::EN;
use crate::ui::pal::Role;
use crate::ui::{Style, Ui};
use faraday_draw::tokens::{
    self, BODY_L, BODY_M, BODY_S, BUTTON_H, FIELD_H, GAP, ICON, ICON_GLYPH_SMALL, LIST_1,
    LIST_PAD_V,
};
use faraday_draw::widgets::Icon;

pub(crate) fn draw(app: &View, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    let Some(s) = state(app) else { return };
    let cards: Vec<flow::Card> = (0..cstep::COUNT as u8)
        .map(|step| flow::Card {
            title: title(step).to_string(),
            summary: summary(s, step),
            mono: false,
            done: s.done(step),
            open: s.open == Some(step),
            default: false,
            toggle: Action::CoreWallet(A::Step(step)),
            guide: None,
            learn: (step == cstep::FILE).then_some(crate::learn::Topic::CoreWallet),
        })
        .collect();
    let back_name = if s.back == crate::Screen::Catalog {
        EN.core_wallet_tools
    } else {
        app.back_name(s.back)
    };
    let col = flow::Column {
        area_x: x0,
        area_w: cw,
        x: x0 + tokens::FLOW_INSET,
        w: (cw - tokens::FLOW_INSET - tokens::FLOW_INSET).min(tokens::FLOW_MAX),
        h,
        back: Some(app.named_back((back_name, Action::Nav(s.back)))),
        heading: EN.core_wallet_title,
        guided: app.guided(),
        switch: true,
        wrap_closed: false,
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
    app.scroll(Scrolled::CoreWallet, next, again);
}

fn title(step: u8) -> &'static str {
    match step {
        cstep::FILE => EN.core_wallet_card_file,
        _ => EN.core_wallet_card_result,
    }
}

fn verdict_line(v: Verdict) -> &'static str {
    match v {
        Verdict::Whole => EN.core_wallet_whole,
        Verdict::NotWhole => EN.core_wallet_not_whole,
        Verdict::WrongPassphrase => EN.core_wallet_wrong_passphrase,
    }
}

fn summary(s: &CoreWalletState, step: u8) -> String {
    match step {
        cstep::FILE => s.file.clone(),
        _ => s
            .shown
            .as_ref()
            .map(|r| verdict_line(r.verdict).to_string()),
    }
    .unwrap_or_default()
}

fn card(app: &View, s: &CoreWalletState, ui: &mut Ui, step: u8, at: (f32, f32, f32)) -> f32 {
    match step {
        cstep::FILE => file(app, s, ui, at),
        _ => result(s, ui, at),
    }
}

/// The wallet files of Files, one button each; a line when there is none;
/// the passphrase field and Read. Returns the height.
fn file(app: &View, s: &CoreWalletState, ui: &mut Ui, (x, y, w): (f32, f32, f32)) -> f32 {
    let mut cy = 0.0;
    let mut any = false;
    for (i, item) in app.inbox.iter().enumerate() {
        if !super::is_wallet_file(&item.bytes) {
            continue;
        }
        any = true;
        let chosen = s.file.as_deref() == Some(item.name.as_str());
        ui.button(
            x,
            y + cy,
            Some(w),
            BUTTON_H,
            &item.name,
            if chosen {
                Style::Selected
            } else {
                Style::Secondary
            },
            Action::CoreWallet(A::File(i)),
        );
        cy += BUTTON_H + GAP;
    }
    if !any {
        cy += ui.wrap_as(
            (x, y, w),
            BODY_S,
            Role::OnSurfaceVariant,
            EN.core_wallet_none_in_files,
        ) + GAP;
    }
    cy += ui.field_label(x, y + cy, EN.core_wallet_passphrase);
    crate::demo::pass_box(
        app,
        ui,
        (x, y + cy, w),
        &s.passphrase,
        (s.revealed, s.typing),
        (Action::CoreWallet(A::Focus), Action::CoreWallet(A::Eye)),
        crate::demo::PassField::CoreWallet,
    );
    cy += FIELD_H + GAP;
    if let Some(why) = &s.refused {
        cy += ui.wrap_as((x, y + cy, w), BODY_M, Role::Error, why) + GAP;
    }
    ui.button(
        x,
        y + cy,
        None,
        BUTTON_H,
        EN.core_wallet_read,
        if s.file.is_some() {
            Style::Primary
        } else {
            Style::PrimaryOff
        },
        Action::CoreWallet(A::Read),
    );
    cy + BUTTON_H
}

fn result(s: &CoreWalletState, ui: &mut Ui, (x, y, w): (f32, f32, f32)) -> f32 {
    let Some(r) = &s.shown else {
        return ui.wrap_as(
            (x, y, w),
            BODY_S,
            Role::OnSurfaceVariant,
            EN.core_wallet_result_waits,
        );
    };
    let ok = r.verdict == Verdict::Whole;
    let tone = if ok { Role::Ok } else { Role::Error };
    let icon = if ok { Icon::Success } else { Icon::Error };
    ui.icon(x, y, ICON, icon, ICON_GLYPH_SMALL, tone);
    let indent = ICON + GAP;
    let mut cy = ui
        .wrap_as(
            (x + indent, y, w - indent),
            BODY_M,
            tone,
            verdict_line(r.verdict),
        )
        .max(ICON)
        + GAP;
    for (k, v) in &r.facts {
        cy += faraday_ui::sheet::kv(ui, x, y + cy, w, k, v, Role::OnSurface);
    }
    if r.no_active {
        cy += ui.wrap_as(
            (x, y + cy, w),
            BODY_M,
            Role::OnSurface,
            EN.core_wallet_no_active,
        ) + GAP;
    }
    for (i, pair) in s.take.iter().enumerate() {
        cy += pair_row(ui, (x, y + cy, w), &pair.label, A::Take(i)) + GAP;
    }
    for d in &r.descriptors {
        cy += ui.wrap_as(
            (x, y + cy, w),
            BODY_S,
            Role::OnSurfaceVariant,
            EN.core_wallet_descriptor,
        );
        cy += ui.wrap_as((x, y + cy, w), BODY_M.mono(), Role::OnSurface, &d.text);
        for n in &d.notes {
            cy += ui.wrap_as((x, y + cy, w), BODY_S, Role::OnSurfaceVariant, n);
        }
        cy += GAP;
    }
    if let Some((label, rest)) = r.kept.split_first() {
        cy += ui.wrap_as((x, y + cy, w), BODY_S, Role::OnSurfaceVariant, label);
        for t in rest {
            cy += ui.wrap_as((x, y + cy, w), BODY_M.mono(), Role::OnSurface, t);
        }
        cy += GAP;
    }
    for l in &r.lines {
        cy += ui.wrap_as((x, y + cy, w), BODY_M, Role::Error, l) + GAP;
    }
    cy - GAP
}

/// One active pair as Material 3's one-line list item (Lists): the pair's
/// label the headline, Take to Wallets its trailing text button; where
/// the label does not fit beside the button, the button goes under it,
/// at the right. Returns its height.
fn pair_row(ui: &mut Ui, (x, y, w): (f32, f32, f32), label: &str, take: A) -> f32 {
    let action = Action::CoreWallet(take);
    let bw = crate::compact::screen::measured(ui, |ui| {
        ui.button(
            0.0,
            0.0,
            None,
            BUTTON_H,
            EN.core_wallet_take,
            Style::Ghost,
            action,
        )
    });
    let head = ui.line_as(BODY_L);
    let tw = (w - bw - GAP).max(0.0);
    if ui.measure_as(BODY_L, label) > tw {
        let top = y + LIST_PAD_V;
        let lines = ui.wrap_as((x, top, w), BODY_L, Role::OnSurface, label);
        let by = top + lines + LIST_PAD_V;
        ui.button(
            x + w - bw,
            by,
            Some(bw),
            BUTTON_H,
            EN.core_wallet_take,
            Style::Ghost,
            action,
        );
        return by + BUTTON_H + LIST_PAD_V - y;
    }
    let h = (head + 2.0 * LIST_PAD_V).max(LIST_1);
    ui.text_mid_as(
        (x, y + (h - head) / 2.0, head),
        BODY_L,
        Role::OnSurface,
        label,
    );
    ui.button(
        x + w - bw,
        y + (h - BUTTON_H) / 2.0,
        Some(bw),
        BUTTON_H,
        EN.core_wallet_take,
        Style::Ghost,
        action,
    );
    h
}
