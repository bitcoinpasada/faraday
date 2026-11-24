//! The Krux code tool's screen: the step-card column over
//! [`crate::kef_tool`].

use super::{KefToolAction as K, KefToolState, Pick, kstep, state, wrap};
use crate::Action;
use crate::drawing::{Scrolled, View};
use crate::flow;
use crate::kef_form::Stage;
use crate::kef_form::screen::{Ask, draw as form_draw, sheet as form_sheet};
use crate::pick;
use crate::screens::next_button_when;
use crate::strings::{EN, fill};
use crate::ui::pal::Role;
use crate::ui::{Style, Ui};
use crate::vault::screen::secret_box;
use faraday_draw::tokens::{BODY_S, BUTTON_H, FIELD_H, GAP};
use faraday_draw::widgets::Icon;

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
    let back_name = if s.back == crate::Screen::Catalog {
        EN.kef_tool_tools
    } else {
        app.back_name(s.back)
    };
    let page = flow::Page {
        area: (x0, cw),
        h,
        foot_h: 0.0,
        back: Some(app.named_back((back_name, Action::Nav(s.back)))),
        heading: EN.kef_tool_title,
        chip: None,
        note: None,
        chip_tap: None,
        step: None,
        moves: crate::ui::Moves::No,
    };
    let Some(contained) = crate::trail::StepFlow::contained(&super::KEF_TOOL_STEPS) else {
        return;
    };
    let scroll = s.scroll;
    let (next, again, _) = {
        let app_ref: &View = app;
        let mut body = |ui: &mut Ui, step: u8, x: f32, y: f32, w: f32| -> f32 {
            match state(app_ref) {
                Some(_) if step == kstep::READ => {
                    crate::seeds::kef::read_rows(app_ref, ui, (x, y, w), false)
                }
                Some(s) => card(app_ref, s, ui, step, (x, y, w), cw),
                None => 0.0,
            }
        };
        let mut under = |_: &mut Ui, _: f32, _: f32, _: f32| 0.0;
        flow::contained(ui, app_ref, contained, &page, scroll, &mut body, &mut under)
    };
    app.scroll(Scrolled::KefTool, next, again);
}

/// The Options sheet, over the page.
pub(crate) fn options_sheet(app: &View, ui: &mut Ui, w: f32, h: f32) {
    if let Some(f) = state(app).and_then(|s| s.form()) {
        form_sheet(app, ui, (w, h), f, ask());
    }
}

pub(super) fn summary(app: &crate::Faraday, s: &KefToolState, step: u8) -> String {
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

fn card(
    app: &View,
    s: &KefToolState,
    ui: &mut Ui,
    step: u8,
    (x, y, w): (f32, f32, f32),
    area_w: f32,
) -> f32 {
    match step {
        kstep::CONTENTS => contents(app, s, ui, (x, y, w), area_w),
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

/// The Contents step's pick for the pane where the pair stands: the wallets,
/// then the keys a code can be made of, then typing. Labels and fingerprints
/// only.
pub(super) fn pick(app: &crate::Faraday, s: &KefToolState) -> flow::Pick {
    let mut groups = Vec::new();
    let wallets: Vec<_> = app
        .session
        .wallets
        .iter()
        .enumerate()
        .map(|(i, wl)| {
            pick::PickRow::pick(
                Icon::Wallet,
                String::from(&*wl.name),
                String::from(&*crate::wallet::Session::shape(wl)),
                s.wallet == Some(i),
                Action::KefTool(K::Wallet(i)),
            )
        })
        .collect();
    if !wallets.is_empty() {
        groups.push(flow::PickGroup {
            head: EN.session_group_wallets,
            rows: wallets,
        });
    }
    // Every loaded key stays: one that cannot be a code is dimmed with why.
    let offered = super::offered_keys(app);
    let keys: Vec<_> = app
        .session
        .keys
        .all()
        .enumerate()
        .map(|(i, k)| {
            let on = offered.contains(&i);
            let row = pick::PickRow::pick(
                Icon::Keys,
                String::from(&*crate::wallet::key_line(k.fingerprint, &k.label)),
                if on {
                    EN.kef_tool_key.to_string()
                } else {
                    String::new()
                },
                on && s.key == Some(i),
                Action::KefTool(K::Key(i)),
            );
            if on {
                row
            } else {
                row.dimmed_for(EN.kef_tool_not_this_key)
            }
        })
        .collect();
    let loaded = !groups.is_empty() || !keys.is_empty();
    if !keys.is_empty() {
        groups.push(flow::PickGroup {
            head: EN.session_group_keys,
            rows: keys,
        });
    }
    groups.push(flow::PickGroup {
        head: pick::ways_in_head(loaded),
        rows: vec![pick::PickRow::way_in(
            Icon::Keyboard,
            EN.kef_tool_type,
            Action::KefTool(K::Pick(Pick::Type)),
        )],
    });
    flow::Pick {
        question: EN.kef_tool_which,
        groups,
    }
}

fn contents(
    app: &View,
    s: &KefToolState,
    ui: &mut Ui,
    (x, y, w): (f32, f32, f32),
    area_w: f32,
) -> f32 {
    // Where the pair stands the pane holds the wallets and keys and the
    // card the typed text and Next (`flow::Contained::pick`).
    if flow::pair_left(app, &super::KEF_TOOL_STEPS, ui.compact, area_w) {
        return if s.pick == Some(Pick::Type) {
            type_card(app, s, ui, (x, y, w))
        } else {
            let on = false;
            let action = Action::KefTool(K::Next);
            if next_button_when(ui, (x, y, w), EN.kef_tool_continue, action, on) {
                BUTTON_H
            } else {
                0.0
            }
        };
    }
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
        Some(Pick::Type) => cy += type_card(app, s, ui, (x, cy, w)),
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

/// The typed text's field and Continue. Returns the height.
fn type_card(app: &View, s: &KefToolState, ui: &mut Ui, (x, y, w): (f32, f32, f32)) -> f32 {
    let mut cy = y;
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
    cy + BUTTON_H - y
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
