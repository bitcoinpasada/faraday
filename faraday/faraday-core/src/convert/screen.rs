//! The Convert a seed screen: the step-card column over [`crate::convert`].

use crate::convert::set::{self, SetAct};
use crate::convert::{
    ConvertAction as C, ConvertState, ROWS, Target, Value, cstep, same_as_source, standard_text,
};
use crate::drawing::{Scrolled, View};
use crate::screens::next_button_when;
use crate::strings::{EN, fill};
use crate::ui::pal::{Role, on_selected, selected_or};
use crate::ui::{Style, Ui};
use crate::wallet::{fp_text, key_line};
use crate::{Action, flow, pick};
use faraday_bip::bip39::Language;
use faraday_bip::keys::Fingerprint;
use faraday_draw::tokens::{
    self, BODY_L, BODY_M, BODY_S, BUTTON_H, DISABLED_CONTENT, FIELD_GAP, FIELD_H, FIELD_PAD, GAP,
    GAP_SMALL, LABEL_L, PAD, SHAPE_S, TITLE_S,
};
use faraday_draw::widgets::Icon;
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
    // Back names where the flow was opened from; Tools' tile by Tools.
    let back_name = if s.back == crate::Screen::Catalog {
        EN.convert_tools
    } else {
        app.back_name(s.back)
    };
    let page = flow::Page {
        area: (x0, cw),
        h,
        foot_h: 0.0,
        pin: None,
        back: Some(app.named_back((back_name, Action::Nav(s.back)))),
        heading: EN.convert_title,
        chip: None,
        note: None,
        chip_tap: None,
        step: None,
        moves: crate::ui::Moves::Yes,
    };
    let Some(contained) = crate::trail::StepFlow::contained(&crate::convert::CONVERT_STEPS) else {
        return;
    };
    let scroll = s.scroll;
    let (next, again, _) = {
        let app_ref: &View = app;
        let mut body = |ui: &mut Ui, step: u8, x: f32, y: f32, w: f32| -> f32 {
            match crate::convert::state(app_ref) {
                Some(s) => card(app_ref, s, ui, step, (x, y, w), cw),
                None => 0.0,
            }
        };
        let mut under = |_: &mut Ui, _: f32, _: f32, _: f32| 0.0;
        flow::contained(ui, app_ref, contained, &page, scroll, &mut body, &mut under)
    };
    app.scroll(Scrolled::Convert, next, again);
}

/// The name of a target, as this network writes it.
fn target_name(app: &crate::Faraday, name: &'static str, target: Option<Target>) -> &'static str {
    let test = app.session.keys.network() != faraday_bip::keys::Network::Mainnet;
    if test && target == Some(Target::Xprv) {
        EN.convert_tprv
    } else {
        name
    }
}

fn set_act_name(act: SetAct) -> &'static str {
    match act {
        SetAct::Add => EN.convert_set_add,
        SetAct::Replace => EN.convert_set_replace,
        SetAct::Rotate => EN.convert_set_rotate,
    }
}

pub(crate) fn summary(app: &crate::Faraday, s: &ConvertState, step: u8) -> String {
    if let Some(x) = &s.set {
        return match (step, app.session.keys.slip39_set_facts()) {
            (cstep::FROM, Some(f)) => {
                fill(EN.convert_set_summary, &[&format!("{:04x}", f.identifier)])
            }
            (cstep::FROM, None) => EN.convert_set_source.to_string(),
            (cstep::TO, _) => x
                .act
                .map_or(EN.convert_not_chosen, set_act_name)
                .to_string(),
            (cstep::SETTINGS, _) if x.act == Some(SetAct::Rotate) => {
                String::from(&*crate::convert::slip_set_standard(s))
            }
            _ => String::new(),
        };
    }
    match step {
        cstep::FROM => s.key.map_or_else(
            || EN.convert_not_chosen.to_string(),
            |fp| String::from(&*fp_text(Fingerprint(fp))),
        ),
        cstep::TO => ROWS
            .iter()
            .find(|r| r.target.is_some() && r.target == s.target)
            .map_or(EN.convert_not_chosen, |r| {
                target_name(app, r.name, r.target)
            })
            .to_string(),
        cstep::SETTINGS => match (s.target, key_of(app, s)) {
            (Some(target), Some(k)) => String::from(&*standard_text(s, k, target)),
            _ => String::new(),
        },
        _ => String::new(),
    }
}

/// The key chosen, if it is loaded.
fn key_of<'a>(app: &'a crate::Faraday, s: &ConvertState) -> Option<&'a faraday_keys::KeyInfo> {
    crate::convert::source_info(&app.session.keys, s)
}

fn card(
    app: &View,
    s: &ConvertState,
    ui: &mut Ui,
    step: u8,
    (x, y, w): (f32, f32, f32),
    area_w: f32,
) -> f32 {
    match step {
        cstep::FROM => from_card(app, s, ui, (x, y, w), area_w),
        cstep::TO => to_card(app, s, ui, (x, y, w)),
        cstep::SETTINGS => settings_card(app, s, ui, (x, y, w)),
        _ => result_card(app, s, ui, (x, y, w)),
    }
}

/// The From step's pick for the pane where the pair stands: the loaded keys
/// and the aezeed held, then the ways in. Labels and fingerprints only.
pub(super) fn pick(app: &crate::Faraday, s: &ConvertState) -> flow::Pick {
    // The aezeed read for this flow, held by faraday-keys: not a key.
    let aezeed: Vec<pick::PickRow> = key_of(app, s)
        .filter(|_| s.aezeed)
        .map(|info| {
            let name = key_line(info.fingerprint, EN.convert_lnd_aezeed);
            pick::PickRow::pick(
                Icon::Keys,
                String::from(&*name),
                EN.convert_from_aezeed.to_string(),
                true,
                Action::Convert(C::Step(cstep::TO)),
            )
        })
        .into_iter()
        .collect();
    let mut ways = vec![
        pick::PickRow::way_in(Icon::Keys, EN.convert_add_a_key, Action::Entry(None)),
        pick::PickRow::way_in(
            Icon::List,
            EN.entry_other_forms,
            Action::Convert(C::OtherForms),
        ),
    ];
    if s.other_forms {
        let opener = &crate::ways_in::CONVERT_FROM;
        if opener.other_forms().contains(&crate::forms::Form::Slip39) {
            ways.push(pick::PickRow::way_in(
                Icon::LayerGroup,
                EN.convert_set_source,
                Action::Convert(C::SetSource),
            ));
        }
        if opener.other_ways.contains(&crate::ways_in::Way::Aezeed) {
            ways.push(pick::PickRow::way_in(
                Icon::Keys,
                EN.convert_lnd_aezeed,
                Action::Convert(C::Aezeed),
            ));
        }
    }
    pick::Decl::new(EN.convert_which_seed)
        .keys(app, |_, k| {
            Some(pick::Take::of(
                !s.aezeed && s.key == Some(k.fingerprint.0),
                Action::Convert(C::Key(k.fingerprint.0)),
            ))
        })
        .also_keys(aezeed)
        .ways(ways)
        .pick()
}

fn from_card(
    app: &View,
    s: &ConvertState,
    ui: &mut Ui,
    (x, y, w): (f32, f32, f32),
    area_w: f32,
) -> f32 {
    let mut cy = y;
    // Where the pair stands the pane holds the keys and the ways in and
    // the card the vaults' keys, the set's entry and Next
    // (`flow::Contained::pick`).
    if flow::pair_left(app, &crate::convert::CONVERT_STEPS, ui.compact, area_w) {
        cy += vault_keys(app, ui, (x, cy, w));
        if s.set.is_some() {
            cy += set_entry(app, s, ui, (x, cy, w));
        } else if next_button_when(
            ui,
            (x, cy, w),
            EN.convert_set_continue,
            Action::Convert(C::Step(cstep::TO)),
            s.done[usize::from(cstep::FROM)],
        ) {
            cy += BUTTON_H;
        }
        return cy - y;
    }
    // The keys and the aezeed are the pick's rows (the pane's, drawn from
    // one place); the open vaults' keys follow them.
    let keys_group = pick(app, s)
        .groups
        .into_iter()
        .find(|g| g.head == EN.session_group_keys);
    if let Some(g) = keys_group {
        cy += pick::draw_groups(ui, x, cy, w, std::slice::from_ref(&g));
    }
    cy += vault_keys(app, ui, (x, cy, w));
    let opener = &crate::ways_in::CONVERT_FROM;
    let mut ways = vec![(
        EN.convert_add_a_key.to_string(),
        Style::Secondary,
        Action::Entry(None),
    )];
    ways.push((
        EN.entry_other_forms.to_string(),
        Style::chosen(s.other_forms),
        Action::Convert(C::OtherForms),
    ));
    if s.other_forms {
        for f in opener.other_forms() {
            // The one form Convert reads as a set, not a key.
            if f == crate::forms::Form::Slip39 {
                ways.push((
                    EN.convert_set_source.to_string(),
                    Style::chosen(s.set.is_some()),
                    Action::Convert(C::SetSource),
                ));
            }
        }
        if opener.other_ways.contains(&crate::ways_in::Way::Aezeed) {
            ways.push((
                EN.convert_lnd_aezeed.to_string(),
                Style::Secondary,
                Action::Convert(C::Aezeed),
            ));
        }
    }
    cy += button_rows(ui, x, cy, w, &ways);
    if s.set.is_some() {
        cy += set_entry(app, s, ui, (x, cy, w));
    }
    cy - y
}

/// The open vaults' keys not loaded, after the loaded ones. Returns the
/// height used.
fn vault_keys(app: &View, ui: &mut Ui, (x, y, w): (f32, f32, f32)) -> f32 {
    let mut cy = y;
    let vaults = app.create_vaults();
    for group in &vaults.open {
        let held = crate::strings::fill(EN.convert_from_vault, &[&group.name]);
        for k in &group.keys {
            let press = Action::Convert(C::VaultKey(group.vault, k.record));
            cy += row(ui, (x, cy, w), (&k.line, &held), false, Some(press));
        }
    }
    cy - y
}

/// The set's entry: the share typed, what is held of the set, and the way on.
fn set_entry(app: &View, s: &ConvertState, ui: &mut Ui, (x, y, w): (f32, f32, f32)) -> f32 {
    let mut cy = y;
    cy += ui.wrap_as(
        (x, cy, w),
        BODY_S,
        Role::OnSurfaceVariant,
        EN.convert_set_field,
    ) + FIELD_GAP;
    ui.field_box((x, cy, w), true, None);
    let here = Action::TypeHere(crate::edit::Here::Share);
    ui.press_box(x, cy, w, FIELD_H, here);
    ui.typed(
        x + FIELD_PAD,
        cy,
        FIELD_H,
        BODY_L.size,
        crate::ui::W::R,
        Role::OnSurface,
        app.tools.convert.set_typed(),
        w - 2.0 * FIELD_PAD,
        true,
        here,
    );
    cy += FIELD_H + GAP;
    if let Some(e) = s.set.as_ref().and_then(|x| x.error.as_ref()) {
        let line = fill(EN.convert_set_not_added, &[e]);
        cy += ui.wrap_as((x, cy, w), LABEL_L, Role::Error, &line) + GAP;
    }
    let mut actions = vec![(
        EN.convert_set_hold.to_string(),
        Style::Primary,
        Action::Convert(C::SetHold),
    )];
    if let Some(f) = app.session.keys.slip39_set_facts() {
        actions.push((
            EN.convert_set_again.to_string(),
            Style::Ghost,
            Action::Convert(C::SetAgain),
        ));
        let id = format!("{:04x}", f.identifier);
        let value = fill(
            EN.convert_set_id_value,
            &[
                &id,
                &f.group_threshold.to_string(),
                &f.group_count.to_string(),
            ],
        );
        cy += button_rows(ui, x, cy, w, &actions);
        cy += crate::compact::kv(ui, x, cy, w, EN.convert_set_label, &value, Role::OnSurface);
        for g in &f.groups {
            let typed: Vec<String> = g.members.iter().map(|m| (m + 1).to_string()).collect();
            let typed = if typed.is_empty() {
                EN.convert_set_none.to_string()
            } else {
                typed.join(", ")
            };
            let state = if g.quorum {
                EN.convert_set_enough
            } else {
                EN.convert_set_not_enough
            };
            let value = fill(
                EN.convert_set_group_value,
                &[&g.threshold.to_string(), &typed, state],
            );
            let label = fill(EN.convert_set_group, &[&(g.index + 1).to_string()]);
            cy += crate::compact::kv(ui, x, cy, w, &label, &value, Role::OnSurface);
        }
        if s.done[usize::from(cstep::FROM)] {
            let go = [(
                EN.convert_set_continue.to_string(),
                Style::Primary,
                Action::Convert(C::Step(cstep::TO)),
            )];
            cy += GAP + button_rows(ui, x, cy, w, &go);
        }
    } else {
        cy += button_rows(ui, x, cy, w, &actions);
    }
    cy - y
}

/// To for a set held: its three actions, each dimmed with what it still
/// needs.
fn set_to_card(app: &View, ui: &mut Ui, (x, y, w): (f32, f32, f32)) -> f32 {
    let Some(f) = app.session.keys.slip39_set_facts() else {
        return 0.0;
    };
    let mut cy = y;
    let rows = [
        (SetAct::Add, EN.convert_set_add_line),
        (SetAct::Replace, EN.convert_set_replace_line),
        (SetAct::Rotate, EN.convert_set_rotate_line),
    ];
    for (act, line) in rows {
        let (line, press) = match set::offered(f, act) {
            Ok(()) => (line.to_string(), Some(Action::Convert(C::SetAct(act)))),
            Err(n) => {
                let said = if act == SetAct::Rotate {
                    EN.convert_set_needs_groups
                } else {
                    EN.convert_set_needs_shares
                };
                (fill(said, &[&n.to_string()]), None)
            }
        };
        cy += row(ui, (x, cy, w), (set_act_name(act), &line), false, press);
    }
    cy - y
}

/// Add's and Replace's Settings: the group where several have a quorum, and
/// the share number. Returns the row below them.
fn member_settings(
    f: &faraday_keys::slip39_set::SetFacts,
    st: &set::SetState,
    ui: &mut Ui,
    (x, mut cy, w): (f32, f32, f32),
) -> f32 {
    let steps = tokens::STEPPER_ROW;
    if set::quorum_groups(f).len() > 1 {
        let (less, more) = (
            Action::Convert(C::SetGroup(-1)),
            Action::Convert(C::SetGroup(1)),
        );
        ui.stepper(
            (x, cy, w),
            EN.convert_set_group_step,
            usize::from(st.group) + 1,
            (less, more),
            true,
        );
        cy += steps;
    }
    let (less, more) = (
        Action::Convert(C::SetMember(-1)),
        Action::Convert(C::SetMember(1)),
    );
    ui.stepper(
        (x, cy, w),
        EN.convert_set_member,
        usize::from(st.member) + 1,
        (less, more),
        true,
    );
    cy += steps;
    if st.act == Some(SetAct::Add) {
        cy += ui.wrap_as(
            (x, cy, w),
            BODY_S,
            Role::OnSurfaceVariant,
            EN.convert_set_add_note,
        ) + GAP;
    }
    cy
}

/// Settings for a set's action.
fn set_settings(app: &View, s: &ConvertState, ui: &mut Ui, (x, y, w): (f32, f32, f32)) -> f32 {
    let (Some(f), Some(st)) = (app.session.keys.slip39_set_facts(), s.set.as_ref()) else {
        return 0.0;
    };
    let mut cy = y + GAP + GAP_SMALL;
    let steps = tokens::STEPPER_ROW;
    let mut make = true;
    match st.act {
        Some(SetAct::Rotate) => {
            for g in 0..f.group_count {
                let label = fill(EN.convert_set_lost, &[&(g + 1).to_string()]);
                let value = usize::from(st.lost.get(usize::from(g)).copied().unwrap_or(0));
                let (less, more) = (
                    Action::Convert(C::Lost(g, -1)),
                    Action::Convert(C::Lost(g, 1)),
                );
                ui.stepper((x, cy, w), &label, value, (less, more), true);
                cy += steps;
            }
            let count = faraday_keys::slip39_set::rotate_count(
                f.group_threshold,
                &set::rotate_groups(f, &st.lost),
            );
            if count.may_be_in_other_hands {
                return cy
                    + ui.wrap_as((x, cy, w), BODY_M, Role::Error, EN.convert_set_other_hands)
                    - y
                    + GAP;
            }
            let mut presses = Vec::new();
            for g in &f.groups {
                for &m in &g.members {
                    let on = st.destroyed.contains(&(g.index, m));
                    let label = fill(
                        EN.convert_set_destroyed,
                        &[&(g.index + 1).to_string(), &(m + 1).to_string()],
                    );
                    presses.push((
                        label,
                        Style::chosen(on),
                        Action::Convert(C::Destroy(g.index, m)),
                    ));
                }
            }
            presses.push((
                EN.convert_set_destroyed_others.to_string(),
                Style::chosen(st.others),
                Action::Convert(C::DestroyOthers),
            ));
            cy += button_rows(ui, x, cy, w, &presses);
            if set::all_destroyed(f, st) {
                cy = slip_steppers(app, s, ui, (x, cy, w));
                cy += crate::compact::kv(
                    ui,
                    x,
                    cy,
                    w,
                    EN.convert_fresh,
                    EN.convert_fresh_value,
                    Role::OnSurface,
                );
            } else {
                make = false;
            }
        }
        _ => cy = member_settings(f, st, ui, (x, cy, w)),
    }
    if make {
        ui.button(
            x,
            cy,
            None,
            BUTTON_H,
            EN.convert_make,
            Style::Primary,
            Action::Convert(C::Make),
        );
        cy += BUTTON_H;
    }
    cy - y + GAP_SMALL
}

/// Previous share and Next share under a result of several shares, which
/// shows and saves one at a time (Material 3, Buttons: tonal). Nothing for
/// any other result. Returns the height taken.
fn part_buttons(app: &View, ui: &mut Ui, (x, y, w): (f32, f32, f32)) -> f32 {
    let Some((at, n)) = app.convert_part() else {
        return 0.0;
    };
    let mut items = Vec::new();
    if at > 0 {
        items.push((
            EN.convert_previous_share,
            Style::Secondary,
            Action::Convert(C::Part(-1)),
        ));
    }
    if at + 1 < n {
        items.push((
            EN.convert_next_share,
            Style::Secondary,
            Action::Convert(C::Part(1)),
        ));
    }
    button_rows(ui, x, y, w, &items) + GAP
}

/// Save's label: every share, one file each, for a result of several, else
/// the file.
fn save_label(app: &View) -> &'static str {
    if app.convert_part().is_some() {
        EN.convert_save_shares
    } else {
        EN.convert_save
    }
}

/// Result for a share or a set made.
fn set_result(app: &View, s: &ConvertState, ui: &mut Ui, (x, y, w): (f32, f32, f32)) -> f32 {
    let mut cy = y;
    if let (Some(f), Some(st)) = (app.session.keys.slip39_set_facts(), s.set.as_ref()) {
        if let Some(line) = &st.refused {
            cy += ui.wrap_as((x, cy, w), BODY_M, Role::OnSurface, line) + GAP;
        } else {
            cy += found_line(app, ui, (x, cy, w));
            let eye = (Action::Convert(C::Show), s.shown);
            let text = || app.convert_shown_text().unwrap_or_default();
            cy += crate::vault::screen::secret::secret_with(ui, (x, cy, w), text, true, eye) + GAP;
            cy += part_buttons(app, ui, (x, cy, w));
            let mut fact = |ui: &mut Ui, label: &str, value: &str| {
                cy += crate::compact::kv(ui, x, cy, w, label, value, Role::OnSurface);
            };
            fact(ui, EN.convert_standard, &set::standard(f, s, st));
            fact(ui, EN.convert_restores_in, EN.convert_slip39_restores);
            fact(ui, EN.convert_passphrase, EN.convert_set_passphrase);
            if st.act == Some(SetAct::Rotate) {
                fact(ui, EN.convert_set_old, EN.convert_set_old_value);
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
                save_label(app).to_string(),
                Style::Primary,
                Action::Convert(C::Save),
            ));
            cy += button_rows(ui, x, cy, w, &actions) + GAP_SMALL;
        }
    }
    cy += ending(ui, (x, cy, w));
    cy - y + GAP_SMALL
}

fn to_card(app: &View, s: &ConvertState, ui: &mut Ui, (x, y, w): (f32, f32, f32)) -> f32 {
    if s.set.is_some() {
        return set_to_card(app, ui, (x, y, w));
    }
    let Some(key) = key_of(app, s) else {
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

/// The SLIP-39 steppers: groups, how many are needed, and each group's
/// shape. Returns the y under the last.
fn slip_steppers(app: &View, s: &ConvertState, ui: &mut Ui, (x, y, w): (f32, f32, f32)) -> f32 {
    let mut cy = y;
    let steps = tokens::STEPPER_ROW;
    let one = s.slip.len() == 1;
    let len = crate::convert::seed_len_of(&app.session.keys, s);
    let mut step = |ui: &mut Ui, label: &str, value: u8, (less, more): (C, C), on: bool| {
        // A press that would leave a shape the randomness cannot
        // pay for is not taken: its plus shows disabled.
        if !crate::convert::slip_fits(len, s.slip_t, &s.slip, more) {
            ui.dimmed.push(Action::Convert(more));
        }
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
    cy
}

/// Settings: Seed XOR's parts, a codex32 set's threshold and count, or a
/// SLIP-39 set's groups, groups needed and each group's shares needed and
/// made, each on a Stepper (as New key's); where the set draws randomness,
/// its Fact.
fn settings_card(app: &View, s: &ConvertState, ui: &mut Ui, (x, y, w): (f32, f32, f32)) -> f32 {
    if s.set.is_some() {
        return set_settings(app, s, ui, (x, y, w));
    }
    // Room under the card's title, which a press on a small panel shares
    // with the first stepper.
    let mut cy = y + GAP + GAP_SMALL;
    let steps = tokens::STEPPER_ROW;
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
            cy = slip_steppers(app, s, ui, (x, cy, w));
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
    let check = app.tools.convert.check();
    if let Some(check) = check {
        let n = check.text().chars().count();
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
    if s.set.is_some() {
        return set_result(app, s, ui, (x, y, w));
    }
    let mut cy = y;
    let key = key_of(app, s);
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
                // Joined and numbered by the press that opened the eye and
                // held while it is open: drawing only reads it.
                let text = || app.convert_shown_text().unwrap_or_default();
                crate::vault::screen::secret::secret_with(ui, (x, cy, w), text, true, eye)
            } + GAP;
            cy += part_buttons(app, ui, (x, cy, w));
            let mut fact = |ui: &mut Ui, label: &str, value: &str| {
                cy += crate::compact::kv(ui, x, cy, w, label, value, Role::OnSurface);
            };
            fact(ui, EN.convert_standard, &standard_text(s, k, target));
            fact(ui, EN.convert_restores_in, target.restores_in());
            if s.aezeed {
                // No key was loaded, so there is no source to be the same as.
                fact(ui, EN.convert_fingerprint, &fp_text(k.fingerprint));
            } else {
                fact(ui, EN.convert_fingerprint, &same_as_source(k.fingerprint));
            }
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
            if let Some((y, m, d)) = app.session.keys.aezeed_birthday().filter(|_| s.aezeed) {
                fact(ui, EN.convert_birthday, &format!("{y}-{m:02}-{d:02}"));
                fact(ui, EN.convert_channels, EN.convert_aezeed_onchain);
            }
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
                save_label(app).to_string(),
                Style::Primary,
                Action::Convert(C::Save),
            ));
            cy += button_rows(ui, x, cy, w, &actions) + GAP_SMALL;
        }
        _ => {}
    }
    cy += ending(ui, (x, cy, w));
    cy - y + GAP_SMALL
}

/// The end of a result: Another starts Convert again, empty, and Done goes
/// back to where it was opened from; the next step above is the filled one.
fn ending(ui: &mut Ui, (x, y, w): (f32, f32, f32)) -> f32 {
    let end = [
        (
            EN.convert_another.to_string(),
            Style::Secondary,
            Action::Convert(C::Another),
        ),
        (
            EN.convert_done.to_string(),
            Style::Secondary,
            Action::Convert(C::Done),
        ),
    ];
    button_rows(ui, x, y, w, &end)
}
