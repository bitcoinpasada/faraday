//! Read a Bitcoin Core wallet's screen: the step-card column over
//! [`crate::core_wallet::tool`].

use super::tool::{CoreWalletAction as A, CoreWalletState, Verdict, cstep, state};
use crate::Action;
use crate::drawing::{Scrolled, View};
use crate::flow;
use crate::screens::parts::{ending, file_choices, nothing_to_work_on};
use crate::strings::EN;
use crate::ui::pal::Role;
use crate::ui::{Region, Style, Ui};
use faraday_draw::tokens::{
    self, BODY_L, BODY_M, BODY_S, BUTTON_H, FIELD_H, GAP, ICON, ICON_GLYPH_SMALL, LIST_1,
    LIST_PAD_V,
};
use faraday_draw::widgets::Icon;

pub(crate) fn draw(app: &View, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    let Some(s) = state(app) else { return };
    let back_name = if s.back == crate::Screen::Catalog {
        EN.core_wallet_tools
    } else {
        app.back_name(s.back)
    };
    let back = Some(app.named_back((back_name, Action::Nav(s.back))));
    if s.landing {
        landing_page(app, ui, (x0, cw, h), back);
        return;
    }
    let page = flow::Page {
        area: (x0, cw),
        h,
        foot_h: 0.0,
        back,
        heading: EN.core_wallet_title,
        chip: None,
        note: None,
        chip_tap: None,
        step: None,
        moves: crate::ui::Moves::No,
    };
    let Some(contained) = crate::trail::StepFlow::contained(&super::tool::CORE_WALLET_STEPS) else {
        return;
    };
    let scroll = s.scroll;
    let (next, again, _) = {
        let app_ref: &View = app;
        let mut body = |ui: &mut Ui, step: u8, x: f32, y: f32, w: f32| -> f32 {
            match state(app_ref) {
                Some(s) => card(app_ref, s, ui, step, (x, y, w)),
                None => 0.0,
            }
        };
        let mut under = |_: &mut Ui, _: f32, _: f32, _: f32| 0.0;
        flow::contained(ui, app_ref, contained, &page, scroll, &mut body, &mut under)
    };
    app.scroll(Scrolled::CoreWallet, next, again);
}

/// No file chosen: the files to choose from, or the empty part, in place
/// of the cards.
fn landing_page(
    app: &View,
    ui: &mut Ui,
    (x0, cw, h): (f32, f32, f32),
    back: Option<(&str, Action)>,
) {
    let clip = ui.rect(x0, 0.0, cw, h);
    let open = ui.region_open(Region::page(clip).from_zero(app.list_offset));
    let top = if ui.compact {
        crate::compact::BAR_H + tokens::GAP
    } else {
        crate::screens::bar_head(ui, back, EN.core_wallet_title)
    };
    let (x, w) = if ui.compact {
        (x0 + crate::compact::M, cw - 2.0 * crate::compact::M)
    } else {
        (
            x0 + tokens::FLOW_INSET,
            (cw - 2.0 * tokens::FLOW_INSET).min(tokens::FLOW_MAX),
        )
    };
    landing(app, ui, (x, top, w));
    ui.region_close(open, 0.0);
}

/// No file chosen: the wallet files of Files to choose from, or the empty
/// part. Returns the height.
fn landing(app: &View, ui: &mut Ui, at: (f32, f32, f32)) -> f32 {
    let rows: Vec<(&str, Action)> = app
        .inbox
        .iter()
        .enumerate()
        .filter(|(_, i)| super::is_wallet_file(&i.bytes))
        .map(|(k, i)| (&*i.name, Action::CoreWallet(A::File(k))))
        .collect();
    if rows.is_empty() {
        nothing_to_work_on(
            app,
            ui,
            at,
            EN.core_wallet_no_file_waits,
            (None, crate::Screen::CoreWallet),
        )
    } else {
        file_choices(ui, at, EN.core_wallet_choose_file, &rows)
    }
}

pub(super) fn title(step: u8) -> &'static str {
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

pub(super) fn summary(s: &CoreWalletState, step: u8) -> String {
    match step {
        cstep::FILE => s.file.as_ref().map(|f| String::from(&**f)),
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
        _ => {
            let (x, y, w) = at;
            let h = result(s, ui, at);
            if s.shown.is_none() {
                return h;
            }
            // The end of the last card: Another and Done.
            let end = ending(
                ui,
                (x, y + h + GAP, w),
                None,
                (Action::CoreWallet(A::Another), Action::Nav(s.back)),
            );
            h + GAP + end
        }
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
    crate::pass_field::screen::tool_box(
        app,
        ui,
        (x, y + cy, w),
        s.passphrase.as_str(),
        (s.typing, Action::CoreWallet(A::Focus)),
    );
    app.pass_chip_draw(
        ui,
        s.passphrase.is_empty(),
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
        cy += pair_row(ui, (x, y + cy, w), pair.label, A::Take(i)) + GAP;
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
