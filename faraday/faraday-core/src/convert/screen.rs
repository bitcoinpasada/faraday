//! The Convert a seed screen: the step-card column over [`crate::convert`].

use crate::convert::{
    ConvertAction as C, ConvertState, ROWS, Target, Value, cstep, same_as_source, source,
    standard_text,
};
use crate::drawing::{Scrolled, View};
use crate::strings::{EN, fill};
use crate::ui::pal::{Role, on_selected, selected_or};
use crate::ui::{Style, Ui};
use crate::wallet::{fp_text, key_line};
use crate::{Action, flow};
use faraday_bip::bip39::Language;
use faraday_bip::keys::Fingerprint;
use faraday_draw::tokens::{
    self, BODY_L, BODY_M, BODY_S, BUTTON_H, DISABLED_CONTENT, FIELD_GAP, FIELD_H, FIELD_PAD, GAP,
    GAP_SMALL, LABEL_L, PAD, SHAPE_S, TITLE_S,
};
use faraday_ui::parts::button_rows;

pub(crate) const TITLES: [&str; cstep::COUNT] = [
    EN.convert_from_title,
    EN.convert_to_title,
    EN.convert_settings_title,
    EN.convert_result_title,
];

pub(crate) fn draw(app: &View, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    let Some(s) = crate::convert::state(app) else {
        return;
    };
    let order = crate::convert::order(s);
    let cards: Vec<flow::Card> = order
        .iter()
        .map(|&step| flow::Card {
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
    // Back names where the flow was opened from; Tools' tile by Tools.
    let back_name = if s.back == crate::Screen::Catalog {
        EN.convert_tools
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
                Some(s) => card(app_ref, s, ui, order[i], (x, y, w)),
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
        cstep::SETTINGS => match (s.target, key_of(app, s)) {
            (Some(target), Some(k)) => standard_text(s, k, target),
            _ => String::new(),
        },
        _ => String::new(),
    }
}

/// The key chosen, if it is loaded.
fn key_of<'a>(app: &'a View, s: &ConvertState) -> Option<&'a faraday_keys::KeyInfo> {
    s.key
        .and_then(|fp| app.session.keys.all().find(|k| k.fingerprint.0 == fp))
}

fn card(app: &View, s: &ConvertState, ui: &mut Ui, step: u8, (x, y, w): (f32, f32, f32)) -> f32 {
    match step {
        cstep::FROM => from_card(app, s, ui, (x, y, w)),
        cstep::TO => to_card(app, s, ui, (x, y, w)),
        cstep::SETTINGS => settings_card(app, s, ui, (x, y, w)),
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
    // The open vaults' keys not loaded, after the loaded ones.
    let vaults = app.create_vaults();
    for group in &vaults.open {
        let held = crate::strings::fill(EN.convert_from_vault, &[&group.name]);
        for k in &group.keys {
            let press = Action::Convert(C::VaultKey(group.vault, k.record));
            cy += row(ui, (x, cy, w), (&k.line, &held), false, Some(press));
        }
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
        let press = r
            .target
            .filter(|&t| !crate::convert::dimmed(app, t))
            .map(|_| Action::Convert(C::To(place as u8)));
        let name = target_name(app, r.name, r.target);
        cy += row(ui, (x, cy, w), (name, r.line), false, press);
    }
    cy - y
}

/// Settings: Seed XOR's parts, a codex32 set's threshold and count, or a
/// SLIP-39 set's groups, groups needed and each group's shares needed and
/// made, each on a Stepper (as New key's); where the set draws randomness,
/// its Fact.
fn settings_card(_app: &View, s: &ConvertState, ui: &mut Ui, (x, y, w): (f32, f32, f32)) -> f32 {
    let mut cy = y;
    let steps = tokens::ICON_BUTTON + GAP_SMALL;
    let draws = match s.target {
        Some(Target::SeedXor) => {
            let (less, more) = (Action::Convert(C::Parts(-1)), Action::Convert(C::Parts(1)));
            ui.stepper(
                (x, cy, w),
                EN.convert_parts,
                usize::from(s.parts),
                (less, more),
                true,
            );
            cy += steps;
            true
        }
        Some(Target::SlipSet) => {
            let one = s.slip.len() == 1;
            let mut step = |ui: &mut Ui, label: &str, value: u8, (less, more): (C, C), on: bool| {
                let (less, more) = (Action::Convert(less), Action::Convert(more));
                ui.stepper((x, cy, w), label, usize::from(value), (less, more), on);
                cy += steps;
            };
            step(
                ui,
                EN.convert_slip39_groups,
                s.slip.len() as u8,
                (C::SlipGroups(-1), C::SlipGroups(1)),
                true,
            );
            if !one {
                step(
                    ui,
                    EN.convert_slip39_groups_needed,
                    s.slip_t,
                    (C::SlipGroupK(-1), C::SlipGroupK(1)),
                    true,
                );
            }
            for (g, &(k, n)) in s.slip.iter().enumerate() {
                let (g8, num) = (g as u8, (g + 1).to_string());
                let (needed, made) = if one {
                    (
                        EN.convert_slip39_needed.to_string(),
                        EN.convert_slip39_made.to_string(),
                    )
                } else {
                    (
                        fill(EN.convert_slip39_group_needed, &[&num]),
                        fill(EN.convert_slip39_group_made, &[&num]),
                    )
                };
                step(ui, &needed, k, (C::SlipK(g8, -1), C::SlipK(g8, 1)), n > 1);
                step(ui, &made, n, (C::SlipN(g8, -1), C::SlipN(g8, 1)), true);
            }
            true
        }
        Some(Target::Codex32) => {
            let (less, more) = (Action::Convert(C::K(-1)), Action::Convert(C::K(1)));
            ui.stepper(
                (x, cy, w),
                EN.convert_codex32_k,
                usize::from(s.k),
                (less, more),
                true,
            );
            cy += steps;
            let (less, more) = (Action::Convert(C::N(-1)), Action::Convert(C::N(1)));
            ui.stepper(
                (x, cy, w),
                EN.convert_codex32_n,
                usize::from(s.n),
                (less, more),
                s.k > 0,
            );
            cy += steps;
            s.k > 0
        }
        _ => false,
    };
    if draws {
        cy += crate::compact::kv(
            ui,
            x,
            cy,
            w,
            EN.convert_fresh,
            EN.convert_fresh_value,
            Role::OnSurface,
        );
    }
    ui.button(
        x,
        cy,
        None,
        BUTTON_H,
        EN.convert_make,
        Style::Primary,
        Action::Convert(C::Make),
    );
    cy + BUTTON_H - y + GAP_SMALL
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

/// How the last comparison went, at the top of the card: a long value
/// would put it out of sight under it. Returns its height.
fn found_line(app: &View, ui: &mut Ui, (x, y, w): (f32, f32, f32)) -> f32 {
    match crate::convert::found(app) {
        Some(true) => {
            let groups = crate::convert::state(app)
                .map(crate::convert::groups_made)
                .unwrap_or_default();
            let line = match crate::convert::found_part(app) {
                Some(n) if groups.len() > 1 => {
                    let (g, m) = crate::convert::place_in(&groups, n - 1);
                    fill(
                        EN.convert_copy_matches_group_share,
                        &[&g.to_string(), &m.to_string()],
                    )
                }
                Some(n) => fill(EN.convert_copy_matches_part, &[&n.to_string()]),
                None => EN.convert_copy_matches.to_string(),
            };
            ui.wrap_as((x, y, w), LABEL_L, Role::Ok, &line) + GAP
        }
        Some(false) => ui.wrap_as((x, y, w), LABEL_L, Role::Error, EN.convert_copy_differs) + GAP,
        None => 0.0,
    }
}

/// Check my copy: while it is open, the copy typed back as dots with its
/// count. Returns its height.
fn check_my_copy(app: &View, ui: &mut Ui, (x, y, w): (f32, f32, f32)) -> f32 {
    let mut cy = y;
    let check = app.tools.convert().check();
    if let Some(check) = check {
        let n = check.typed.chars().count();
        let label = crate::strings::fill(EN.convert_your_copy, &[&n.to_string()]);
        cy += ui.wrap_as((x, cy, w), BODY_S, Role::OnSurfaceVariant, &label) + FIELD_GAP;
        ui.field_box((x, cy, w), true, None);
        let here = Action::TypeHere(crate::edit::Here::Convert);
        ui.press_box(x, cy, w, FIELD_H, here);
        ui.typed(
            x + FIELD_PAD,
            cy,
            FIELD_H,
            BODY_L.size,
            crate::ui::W::R,
            Role::OnSurface,
            &"\u{2022}".repeat(n),
            w - 2.0 * FIELD_PAD,
            true,
            here,
        );
        cy += FIELD_H + GAP;
        let actions = [
            (
                EN.convert_compare.to_string(),
                Style::Secondary,
                Action::Convert(C::Compare),
            ),
            (
                EN.convert_clear.to_string(),
                Style::Ghost,
                Action::Convert(C::CheckClear),
            ),
        ];
        cy += button_rows(ui, x, cy, w, &actions);
    }
    cy - y
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
            cy += found_line(app, ui, (x, cy, w));
            // Worked out on the press, held until the card is left; only
            // read here.
            let held = app.convert_held();
            let eye = (Action::Convert(C::Show), s.shown);
            cy += if target.is_code() {
                let code = held.and_then(Value::code);
                let label = EN.convert_show_code;
                crate::vault::screen::secret::code_with(ui, (x, cy, w), code, label, eye)
            } else {
                // The closure runs only with the eye open; its copy lives
                // for the frame, wiped, and is on no state.
                let groups = crate::convert::groups_made(s);
                let text = || held.and_then(|v| v.display(&groups)).unwrap_or_default();
                crate::vault::screen::secret::secret_with(ui, (x, cy, w), text, true, eye)
            } + GAP;
            let mut fact = |ui: &mut Ui, label: &str, value: &str| {
                cy += crate::compact::kv(ui, x, cy, w, label, value, Role::OnSurface);
            };
            fact(ui, EN.convert_standard, &standard_text(s, k, target));
            fact(ui, EN.convert_restores_in, target.restores_in());
            fact(ui, EN.convert_fingerprint, &same_as_source(k.fingerprint));
            if target.lacks_list() && k.language != Language::English {
                fact(
                    ui,
                    EN.convert_word_list,
                    crate::forms::language_name(k.language),
                );
            }
            let paths = if s.paths.is_empty() {
                EN.convert_no_wallet.to_string()
            } else {
                s.paths.join("; ")
            };
            fact(ui, EN.convert_paths, &paths);
            if k.has_passphrase {
                let line = if target.keeps_passphrase_apart() {
                    EN.convert_keeps_passphrase
                } else {
                    EN.convert_holds_passphrase
                };
                fact(ui, EN.convert_passphrase, line);
            }
            cy += GAP;
            cy += check_my_copy(app, ui, (x, cy, w));
            let mut actions = Vec::new();
            if s.checkable {
                actions.push((
                    EN.convert_check.to_string(),
                    Style::Secondary,
                    Action::Convert(C::Check),
                ));
            }
            actions.push((
                EN.convert_save.to_string(),
                Style::Secondary,
                Action::Convert(C::Save),
            ));
            cy += button_rows(ui, x, cy, w, &actions);
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
