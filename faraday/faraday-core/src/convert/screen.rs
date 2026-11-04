//! The Convert a seed screen: the step-card column over [`crate::convert`].

use crate::convert::{
    ConvertAction as C, ConvertState, ROWS, Target, cstep, same_as_source, source,
};
use crate::drawing::{Scrolled, View};
use crate::strings::EN;
use crate::ui::pal::{Role, on_selected, selected_or};
use crate::ui::{Style, Ui};
use crate::wallet::{fp_text, key_line};
use crate::{Action, flow};
use faraday_bip::keys::Fingerprint;
use faraday_draw::tokens::{
    self, BODY_M, BODY_S, BUTTON_H, DISABLED_CONTENT, GAP, GAP_SMALL, PAD, SHAPE_S, TITLE_S,
};

pub(crate) const TITLES: [&str; cstep::COUNT] = [
    EN.convert_from_title,
    EN.convert_to_title,
    EN.convert_result_title,
];

pub(crate) fn draw(app: &View, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    let Some(s) = crate::convert::state(app) else {
        return;
    };
    let cards: Vec<flow::Card> = (0..cstep::COUNT as u8)
        .map(|step| flow::Card {
            title: TITLES[usize::from(step)].to_string(),
            summary: summary(app, s, step),
            mono: false,
            done: s.done[usize::from(step)],
            open: s.open == Some(step),
            default: false,
            toggle: Action::Convert(C::Step(step)),
            guide: None,
            learn: None,
        })
        .collect();
    let col = flow::Column {
        area_x: x0,
        area_w: cw,
        x: x0 + tokens::FLOW_INSET,
        w: (cw - tokens::FLOW_INSET - tokens::FLOW_INSET).min(tokens::FLOW_MAX),
        h,
        back: Some(app.named_back((EN.convert_tools, Action::Nav(s.back)))),
        heading: EN.convert_title,
        guided: app.guided(),
        switch: true,
        note: None,
        chip: None,
        chip_tap: None,
    };
    let scroll = s.scroll;
    let (next, again) = {
        let app_ref: &View = app;
        let mut body = |ui: &mut Ui, i: usize, x: f32, y: f32, w: f32| -> f32 {
            match crate::convert::state(app_ref) {
                Some(s) => card(app_ref, s, ui, i as u8, (x, y, w)),
                None => 0.0,
            }
        };
        flow::column(ui, &col, &cards, scroll, &mut body)
    };
    app.scroll(Scrolled::Convert, next, again);
}

/// The name of a target, as this network writes it.
fn target_name(app: &View, name: &'static str, target: Option<Target>) -> &'static str {
    let test = app.session.keys.network() != faraday_bip::keys::Network::Mainnet;
    if test && target == Some(Target::Xprv) {
        EN.convert_tprv
    } else {
        name
    }
}

fn summary(app: &View, s: &ConvertState, step: u8) -> String {
    match step {
        cstep::FROM => s.key.map_or_else(
            || EN.convert_not_chosen.to_string(),
            |fp| fp_text(Fingerprint(fp)),
        ),
        cstep::TO => ROWS
            .iter()
            .find(|r| r.target.is_some() && r.target == s.target)
            .map_or(EN.convert_not_chosen, |r| {
                target_name(app, r.name, r.target)
            })
            .to_string(),
        _ => String::new(),
    }
}

fn card(app: &View, s: &ConvertState, ui: &mut Ui, step: u8, (x, y, w): (f32, f32, f32)) -> f32 {
    match step {
        cstep::FROM => from_card(app, s, ui, (x, y, w)),
        cstep::TO => to_card(app, s, ui, (x, y, w)),
        _ => result_card(app, s, ui, (x, y, w)),
    }
}

fn from_card(app: &View, s: &ConvertState, ui: &mut Ui, (x, y, w): (f32, f32, f32)) -> f32 {
    let mut cy = y;
    for k in app.session.keys.all() {
        let fp = k.fingerprint;
        let press = Action::Convert(C::Key(fp.0));
        let name = key_line(fp, &crate::bip85::key_name(k));
        let chosen = s.key == Some(fp.0);
        cy += row(ui, (x, cy, w), (&name, &source(k)), chosen, Some(press));
    }
    ui.button(
        x,
        cy,
        None,
        BUTTON_H,
        EN.convert_add_a_key,
        Style::Secondary,
        Action::KeyGen(None),
    );
    cy + BUTTON_H - y
}

fn to_card(app: &View, s: &ConvertState, ui: &mut Ui, (x, y, w): (f32, f32, f32)) -> f32 {
    let Some(key) = s
        .key
        .and_then(|fp| app.session.keys.all().find(|k| k.fingerprint.0 == fp))
    else {
        return 0.0;
    };
    let mut cy = y;
    for (place, r) in ROWS.iter().enumerate() {
        if !r.applies(key) {
            continue;
        }
        let press = r.target.map(|_| Action::Convert(C::To(place as u8)));
        let name = target_name(app, r.name, r.target);
        cy += row(ui, (x, cy, w), (name, r.line), false, press);
    }
    cy - y
}

/// An Option row with its line under its name, the line being long: it
/// grows to hold them. With no press it is dimmed and takes none. Returns
/// the height with the gap under it.
fn row(
    ui: &mut Ui,
    (x, y, w): (f32, f32, f32),
    (name, line): (&str, &str),
    on: bool,
    press: Option<Action>,
) -> f32 {
    let (left, inner) = (x + PAD, w - 2.0 * PAD);
    // Measured in a pass that draws nothing, so the box goes down before
    // the words and does not cover them.
    ui.c.push_clip(faraday_draw::Rect::new(0, 0, 0, 0));
    let name_h = ui.wrap_as((left, y, inner), TITLE_S, Role::OnSurface, name);
    let line_h = ui.wrap_as((left, y, inner), BODY_S, Role::OnSurface, line);
    ui.c.pop_clip();
    let h = (name_h + line_h + 2.0 * GAP).max(tokens::TOUCH);
    ui.fill(
        x,
        y,
        w,
        h,
        SHAPE_S,
        selected_or(on, Role::SurfaceContainerHighest),
    );
    ui.stroke(x, y, w, h, SHAPE_S, selected_or(on, Role::OutlineVariant));
    if let Some(press) = press {
        ui.press_box(x, y, w, h, press);
    }
    let top = y + (h - name_h - line_h) / 2.0;
    if press.is_some() {
        let (ink, muted) = (
            on_selected(on, Role::OnSurface),
            on_selected(on, Role::OnSurfaceVariant),
        );
        ui.wrap_as((left, top, inner), TITLE_S, ink, name);
        ui.wrap_as((left, top + name_h, inner), BODY_S, muted, line);
    } else {
        let dim = Role::OnSurface.at(DISABLED_CONTENT);
        ui.wrap_as((left, top, inner), TITLE_S, dim, name);
        ui.wrap_as((left, top + name_h, inner), BODY_S, dim, line);
    }
    h + GAP
}

fn result_card(app: &View, s: &ConvertState, ui: &mut Ui, (x, y, w): (f32, f32, f32)) -> f32 {
    let mut cy = y;
    let key = s
        .key
        .and_then(|fp| app.session.keys.all().find(|k| k.fingerprint.0 == fp));
    match (s.refused, s.target, key) {
        (Some(line), ..) => {
            cy += ui.wrap_as((x, cy, w), BODY_M, Role::OnSurface, line) + GAP;
        }
        (None, Some(target), Some(k)) => {
            // Worked out on the press, held until the card is left; only
            // read here.
            let held = app.convert_held();
            cy += crate::vault::screen::secret::secret_with(
                ui,
                (x, cy, w),
                || {
                    held.map(|v| v.as_str().to_string().into())
                        .unwrap_or_default()
                },
                true,
                (Action::Convert(C::Show), s.shown),
            ) + GAP;
            let mut fact = |ui: &mut Ui, label: &str, value: &str| {
                cy += crate::compact::kv(ui, x, cy, w, label, value, Role::OnSurface);
            };
            fact(ui, EN.convert_standard, target.standard());
            fact(ui, EN.convert_restores_in, target.restores_in());
            fact(ui, EN.convert_fingerprint, &same_as_source(k.fingerprint));
            let paths = if s.paths.is_empty() {
                EN.convert_no_wallet.to_string()
            } else {
                s.paths.join("; ")
            };
            fact(ui, EN.convert_paths, &paths);
            if k.has_passphrase {
                fact(ui, EN.convert_passphrase, EN.convert_holds_passphrase);
            }
        }
        _ => {}
    }
    ui.button(
        x,
        cy,
        None,
        BUTTON_H,
        EN.convert_done,
        Style::Primary,
        Action::Convert(C::Done),
    );
    cy + BUTTON_H - y + GAP_SMALL
}
