//! The Krux code tool's screen: the step-card column over
//! [`crate::kef_tool`].

use super::{KefToolAction as K, KefToolState, Pick, kstep, state, wrap};
use crate::Action;
use crate::drawing::{Scrolled, View};
use crate::flow;
use crate::kef_form::Stage;
use crate::kef_form::screen::{Ask, draw as form_draw, sheet as form_sheet};
use crate::strings::{EN, fill};
use crate::ui::pal::Role;
use crate::ui::{Style, Ui};
use crate::vault::screen::secret_box;
use faraday_draw::tokens::{self, BODY_S, BUTTON_H, FIELD_H, GAP};

/// The cards' titles by step, the reading card's last.
pub(crate) const TITLES: [&str; kstep::COUNT + 1] = [
    EN.kef_tool_contents,
    EN.kef_tool_make,
    EN.kef_tool_code,
    EN.kef_tool_read,
];

fn ask() -> Ask {
    Ask {
        wrap,
        wrap_dice: |p| Action::KefTool(K::Dice(p)),
        on_rolls: |_| wrap(crate::kef_form::KefAct::DiceField),
        pin: |n| wrap(crate::kef_form::KefAct::Pin(n)),
        rows: |r| wrap(crate::kef_form::KefAct::Rows(r)),
    }
}

pub(crate) fn draw(app: &View, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    let Some(s) = state(app) else { return };
    // Reading a code leads, above the steps that make one; on the wide
    // layout it stands open, on the small panel it is a page of its own.
    let head = flow::Card {
        title: TITLES[usize::from(kstep::READ)].to_string(),
        summary: String::new(),
        mono: false,
        done: false,
        open: !ui.compact || s.open == Some(kstep::READ),
        default: false,
        toggle: Action::KefTool(K::Step(kstep::READ)),
        lead: None,
    };
    let cards: Vec<flow::Card> = std::iter::once(head)
        .chain((0..kstep::COUNT as u8).map(|step| flow::Card {
            title: TITLES[usize::from(step)].to_string(),
            summary: summary(app, s, step),
            mono: false,
            done: s.done[usize::from(step)],
            open: s.open == Some(step),
            default: false,
            toggle: Action::KefTool(K::Step(step)),
            lead: None,
        }))
        .collect();
    let back_name = if s.back == crate::Screen::Catalog {
        EN.kef_tool_tools
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
        heading: EN.kef_tool_title,
        wrap_closed: false,
        total_fixed: true,
        all_shown: false,
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
                Some(_) if i == 0 => crate::seeds::kef::read_rows(app_ref, ui, (x, y, w), false),
                Some(s) => card(app_ref, s, ui, i as u8 - 1, (x, y, w)),
                None => 0.0,
            }
        };
        flow::column(ui, &col, &cards, scroll, &mut body)
    };
    app.scroll(Scrolled::KefTool, next, again);
}

/// The Options sheet, over the page.
pub(crate) fn options_sheet(app: &View, ui: &mut Ui, w: f32, h: f32) {
    if let Some(f) = state(app).and_then(|s| s.form()) {
        form_sheet(app, ui, (w, h), f, ask());
    }
}

fn summary(app: &View, s: &KefToolState, step: u8) -> String {
    match step {
        kstep::CONTENTS if s.done[usize::from(step)] => match s.wallet {
            Some(i) => app
                .session
                .wallets
                .get(i)
                .map(|w| fill(EN.kef_tool_wallet_sum, &[&w.name]))
                .unwrap_or_default(),
            None if s.key.is_some() => s
                .key
                .and_then(|i| app.session.keys.info(i))
                .map(|k| fill(EN.kef_tool_key_sum, &[&k.label, &k.fingerprint.to_string()]))
                .unwrap_or_default(),
            None => fill(EN.kef_tool_typed_sum, &[&s.typed_len().to_string()]),
        },
        kstep::MAKE => s
            .form()
            .filter(|_| s.done[usize::from(step)])
            .map(|f| String::from(&*f.summary()))
            .unwrap_or_default(),
        kstep::CODE if s.done[usize::from(step)] => match s.form() {
            Some(f) if f.checked() => EN.kef_tool_code_checked_sum.to_string(),
            _ => EN.kef_tool_code_sum.to_string(),
        },
        _ => String::new(),
    }
}

fn card(app: &View, s: &KefToolState, ui: &mut Ui, step: u8, (x, y, w): (f32, f32, f32)) -> f32 {
    match step {
        kstep::CONTENTS => contents(app, s, ui, (x, y, w)),
        _ => {
            let Some(f) = s.form() else { return 0.0 };
            let made = matches!(f.stage(), Stage::Made | Stage::Check | Stage::Comparing);
            if made == (step == kstep::CODE) {
                let note = if made {
                    words_only(app, s, ui, (x, y, w))
                } else {
                    0.0
                };
                note + form_draw(app, ui, (x, y + note, w), f, ask())
            } else {
                0.0
            }
        }
    }
}

fn choice(on: bool) -> Style {
    if on {
        Style::Selected
    } else {
        Style::Secondary
    }
}

fn contents(app: &View, s: &KefToolState, ui: &mut Ui, (x, y, w): (f32, f32, f32)) -> f32 {
    let mut cy = y;
    for (label, pick) in [
        (EN.kef_tool_type, Pick::Type),
        (EN.kef_tool_wallet, Pick::Wallet),
    ]
    .into_iter()
    .chain((!app.session.keys.is_empty()).then_some((EN.kef_tool_key, Pick::Key)))
    {
        ui.button(
            x,
            cy,
            Some(w),
            BUTTON_H,
            label,
            choice(s.pick == Some(pick)),
            Action::KefTool(K::Pick(pick)),
        );
        cy += BUTTON_H + GAP;
    }
    match s.pick {
        Some(Pick::Type) => {
            cy += ui.field_label(x, cy, EN.kef_tool_text);
            let show = app.pass_shows(s.shown, s.typed.text.as_str());
            secret_box(
                ui,
                x,
                cy,
                w,
                &s.typed,
                show,
                s.typing,
                Action::KefTool(K::Field),
                Action::KefTool(K::Eye),
            );
            cy += FIELD_H + GAP;
            let style = if s.typed_len() == 0 {
                Style::PrimaryOff
            } else {
                Style::Primary
            };
            ui.button(
                x,
                cy,
                None,
                BUTTON_H,
                EN.kef_tool_continue,
                style,
                Action::KefTool(K::Next),
            );
            cy += BUTTON_H;
        }
        Some(Pick::Wallet) => {
            if app.session.wallets.is_empty() {
                cy += ui.wrap_as(
                    (x, cy, w),
                    BODY_S,
                    Role::OnSurfaceVariant,
                    EN.kef_tool_no_wallet,
                );
            }
            for (i, wl) in app.session.wallets.iter().enumerate() {
                let label = format!("{} · {}", &*wl.name, &*crate::wallet::Session::shape(wl));
                ui.button(
                    x,
                    cy,
                    Some(w),
                    BUTTON_H,
                    &label,
                    choice(s.wallet == Some(i)),
                    Action::KefTool(K::Wallet(i)),
                );
                cy += BUTTON_H + GAP;
            }
            cy -= GAP;
        }
        Some(Pick::Key) => cy += key_rows(app, s, ui, (x, cy, w)) - GAP,
        None => cy -= GAP,
    }
    cy - y
}

/// The Code card's line saying the code holds the words only, for a key
/// that has a passphrase: its height, 0 when there is none to say.
fn words_only(app: &View, s: &KefToolState, ui: &mut Ui, (x, y, w): (f32, f32, f32)) -> f32 {
    let has = s
        .key
        .and_then(|i| app.session.keys.info(i))
        .is_some_and(|k| k.has_passphrase);
    if !has {
        return 0.0;
    }
    ui.wrap_as(
        (x, y, w),
        BODY_S,
        Role::OnSurfaceVariant,
        EN.kef_tool_words_only,
    ) + GAP
}

/// The loaded keys a code can be made of, one row each, or the line
/// saying there is none. Returns the height.
fn key_rows(app: &View, s: &KefToolState, ui: &mut Ui, (x, y, w): (f32, f32, f32)) -> f32 {
    let mut cy = y;
    let offered = super::offered_keys(app);
    if offered.is_empty() {
        cy += ui.wrap_as(
            (x, cy, w),
            BODY_S,
            Role::OnSurfaceVariant,
            EN.kef_tool_no_key,
        );
    }
    for i in offered {
        let Some(k) = app.session.keys.info(i) else {
            continue;
        };
        let label = format!("{} · {}", &*k.label, k.fingerprint);
        ui.button(
            x,
            cy,
            Some(w),
            BUTTON_H,
            &label,
            choice(s.key == Some(i)),
            Action::KefTool(K::Key(i)),
        );
        cy += BUTTON_H + GAP;
    }
    cy - y
}
