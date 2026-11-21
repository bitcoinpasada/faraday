//! My seed makes my wallet's screen: the step-card column over
//! [`crate::seed_check`], Seed, Passphrase, Account and Result.

use super::area::SeedState;
use super::{ACCOUNTS, Account, Answer, SeedAction as S, coin, step};
use crate::Action;
use crate::demo::PassField;
use crate::drawing::{Scrolled, View};
use crate::flow;
use crate::pass_field::pair::{differ, pass_pair};
use crate::pass_field::screen::pass_field;
use crate::screens::parts::{address_press, row_press};
use crate::strings::EN;
use crate::ui::pal::Role;
use crate::ui::{Style, Ui, W};
use crate::wallet::FileKind;
use faraday_bip::keys::ScriptType;
use faraday_draw::tokens::{
    BODY_M, BODY_S, BUTTON_H, CARD_ENTRY_ROOM, FIELD_H, FIELD_PAD, GAP, GAP_SMALL, LABEL_L,
    ROW_CHEVRON_ROOM, SEGMENTED_H, TOUCH,
};
use faraday_text::Text;
use faraday_ui::parts::button_rows;

pub(crate) const TITLES: [&str; step::COUNT] = [
    EN.seed_check_card_seed,
    EN.seed_check_card_passphrase,
    EN.seed_check_card_account,
    EN.seed_check_card_result,
];

fn act(a: S) -> Action {
    Action::SeedCheck(a)
}

pub(crate) fn draw(app: &View, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    let Some(s) = app.tools.seed_check.state() else {
        return;
    };
    let back_name = if s.back == crate::Screen::Catalog {
        EN.ots_tools
    } else {
        app.back_name(s.back)
    };
    let page = flow::Page {
        area: (x0, cw),
        h,
        foot_h: 0.0,
        back: Some(app.named_back((back_name, Action::Nav(s.back)))),
        heading: EN.seed_check_title,
        chip: None,
        note: None,
        chip_tap: None,
        step: None,
        moves: crate::ui::Moves::No,
    };
    let Some(contained) = crate::trail::StepFlow::contained(&super::act::SEED_STEPS) else {
        return;
    };
    let (next, again, _) = {
        let app_ref: &View = app;
        // The error line and Cancel end the card, as they ended the last
        // card reached.
        let mut body = |ui: &mut Ui, k: u8, x: f32, y: f32, w: f32| -> f32 {
            match app_ref.tools.seed_check.state() {
                Some(s) => {
                    let used = card(app_ref, s, ui, k, (x, y, w));
                    used + foot(s, ui, (x, y + used, w))
                }
                None => 0.0,
            }
        };
        let mut under = |_: &mut Ui, _: f32, _: f32, _: f32| 0.0;
        flow::contained(
            ui, app_ref, contained, &page, s.scroll, &mut body, &mut under,
        )
    };
    app.scroll(Scrolled::SeedCheck, next, again);
}

/// The error line, if any, and Cancel, at the end of the card.
fn foot(s: &SeedState, ui: &mut Ui, (x, y, w): (f32, f32, f32)) -> f32 {
    let mut cy = GAP;
    if let Some(e) = &s.error {
        cy += ui.wrap_as((x, y + cy, w), BODY_M, Role::Error, e) + GAP;
    }
    ui.button(
        x,
        y + cy,
        None,
        BUTTON_H,
        EN.seed_check_cancel,
        Style::Ghost,
        act(S::Cancel),
    );
    cy + BUTTON_H
}

pub(super) fn summary(s: &SeedState, k: u8) -> Text {
    let text = match k {
        step::SEED => {
            if s.is_loaded() {
                Some(Text::of(EN.seed_check_loaded))
            } else {
                s.facts
                    .and_then(|f| f.words)
                    .map(|n| Text::fill(EN.seed_check_words_n, &[&Text::num(n as u64)]))
            }
        }
        step::PASSPHRASE => s.facts.map(|f| {
            let line = if f.with_pass {
                EN.seed_check_fingerprint_is_with
            } else {
                EN.seed_check_fingerprint_is
            };
            Text::fill(line, &[&crate::wallet::fp_text(f.fingerprint)])
        }),
        step::ACCOUNT => s.picked.as_ref().map(|(name, path)| {
            Text::formatted(format_args!("{} · {}", name.as_str(), path.as_str()))
        }),
        _ => s.made().map(|m| m.path.clone()),
    };
    text.unwrap_or_default()
}

fn card(app: &View, s: &SeedState, ui: &mut Ui, k: u8, at: (f32, f32, f32)) -> f32 {
    match k {
        step::SEED => seed(app, s, ui, at, true),
        step::PASSPHRASE => passphrase(app, s, ui, at),
        step::ACCOUNT => account(app, s, ui, at),
        _ => result(app, s, ui, at),
    }
}

/// The Seed card, or the Passphrase card once the words are taken, as
/// Check a multisig setup's Check a seed against it shows them under its
/// rows: no key loaded here is offered, and the Account card is never
/// reached.
pub(crate) fn check_step(app: &View, s: &SeedState, ui: &mut Ui, at: (f32, f32, f32)) -> f32 {
    if s.open_card() == Some(step::PASSPHRASE) {
        passphrase(app, s, ui, at)
    } else {
        seed(app, s, ui, at, false)
    }
}

fn seed(app: &View, s: &SeedState, ui: &mut Ui, (x, y, w): (f32, f32, f32), loaded: bool) -> f32 {
    if s.typing_words() && app.entry.state().form == crate::forms::Form::Krux {
        // Add a key's Krux code form: its two ways to read a code.
        let end = crate::seeds::kef::read_rows(app, ui, (x, y, w), true);
        return end + GAP;
    }
    if s.typing_words() && app.entry.state().form != crate::forms::Form::Words {
        // One of the forms of Add a key, drawn by its own code, with
        // Continue putting the parts together.
        let room = y + CARD_ENTRY_ROOM;
        let end = if ui.compact {
            crate::compact::screen::entry_form(app, ui, x, y, w, Some(act(S::FormDone)))
        } else {
            crate::screens::entry_form(app, ui, x, y, w, room, Some(act(S::FormDone)))
        };
        return end + GAP - y;
    }
    if s.typing_words() {
        let end = if ui.compact {
            crate::compact::screen::entry_words(app, ui, x, y, w, false)
        } else {
            crate::screens::entry_words(app, ui, x, y, w, y + CARD_ENTRY_ROOM)
        };
        let cy = end + GAP;
        ui.button(
            x,
            cy,
            None,
            BUTTON_H,
            EN.seed_check_continue,
            Style::Primary,
            act(S::Words),
        );
        return cy + BUTTON_H - y;
    }
    if app.seed_sample_shown() {
        ui.demo_offer(EN.scan_sample_seed, act(S::Sample));
    }
    // Type, Scan and codex32 first; the other forms, and the loaded keys
    // where this card offers them, under Other forms.
    let opening = if loaded {
        crate::Opening::SeedMakesWallet
    } else {
        crate::Opening::SeedAgainst
    };
    let up = button_rows(ui, x, y, w, &opening.up_front(app));
    up + crate::ways_in::screen::other_control(app, ui, (x, y + up, w), opening)
}

fn passphrase(app: &View, s: &SeedState, ui: &mut Ui, (x, y, w): (f32, f32, f32)) -> f32 {
    if !s.adding {
        let rows = [
            (
                EN.seed_check_no_passphrase,
                Style::Secondary,
                act(S::NoPass),
            ),
            (
                EN.seed_check_add_passphrase,
                Style::Secondary,
                act(S::AddPass),
            ),
        ];
        return button_rows(ui, x, y, w, &rows);
    }
    let pairs = pass_pair(
        ui,
        (x, y, w),
        (s.pass.as_str(), s.pass.again_str()),
        |ui, n, fy| {
            let (label, text) = if n == 0 {
                (EN.seed_check_passphrase, s.pass.as_str())
            } else {
                (EN.seed_check_passphrase_again, s.pass.again_str())
            };
            pass_field(
                ui,
                x,
                fy,
                w,
                label,
                text,
                (
                    s.focus == Some(n),
                    app.pass_chip(PassField::SeedCheck(n)),
                    app.typing.pass_eye.shown,
                    app.typing.pass_eye.waits(),
                ),
                act(S::PassField(n)),
            )
        },
    );
    let mut cy = pairs + crate::screens::keys_read_as_us_line(app, ui, (x, y + pairs, w)) + GAP;
    let ready = !s.pass.is_empty()
        && !differ(s.pass.as_str(), s.pass.again_str())
        && !app.typing.pass_eye.waits();
    ui.button(
        x,
        y + cy,
        None,
        BUTTON_H,
        EN.seed_check_use_passphrase,
        if ready {
            Style::Primary
        } else {
            Style::PrimaryOff
        },
        act(S::UsePass),
    );
    cy += BUTTON_H;
    cy
}

fn account(app: &View, s: &SeedState, ui: &mut Ui, (x, y, w): (f32, f32, f32)) -> f32 {
    let net = app.session.keys.network();
    let mut cy = 0.0;
    for a in ACCOUNTS {
        cy += account_row(ui, (x, y + cy, w), a, net) + GAP;
    }
    if !s.path_open {
        let rows = [(EN.seed_check_other_account, Style::Secondary, act(S::Other))];
        return cy + button_rows(ui, x, y + cy, w, &rows);
    }
    let focus = act(S::PathField);
    ui.field_box((x, y + cy, w), s.path_focus, None);
    let mw = ui.text_mid_as(
        (x + FIELD_PAD, y + cy, FIELD_H),
        BODY_M.mono(),
        Role::OnSurface,
        "m/",
    );
    ui.typed(
        x + FIELD_PAD + mw,
        y + cy,
        FIELD_H,
        BODY_M.mono().size,
        W::from(BODY_M.mono().face),
        Role::OnSurface,
        s.path.as_str(),
        w - 2.0 * FIELD_PAD - mw,
        s.path_focus,
        focus,
    );
    ui.press_box(x, y + cy, w, FIELD_H, focus);
    cy += FIELD_H + GAP;
    let scripts: Vec<(&str, Style, Action)> = ScriptType::ALL
        .iter()
        .enumerate()
        .map(|(i, t)| {
            (
                crate::explore::screen::script_name(*t),
                Style::chosen(*t == s.script),
                act(S::Script(i as u8)),
            )
        })
        .collect();
    cy += button_rows(ui, x, y + cy, w, &scripts) + GAP;
    let style = if s.path.is_empty() {
        Style::PrimaryOff
    } else {
        Style::Primary
    };
    ui.button(
        x,
        y + cy,
        None,
        BUTTON_H,
        EN.seed_check_use_path,
        style,
        act(S::UsePath),
    );
    cy + BUTTON_H
}

/// One account's row: its name, then its path and how its addresses
/// start; a press picks it. Returns the height.
fn account_row(
    ui: &mut Ui,
    (x, y, w): (f32, f32, f32),
    a: Account,
    net: faraday_bip::keys::Network,
) -> f32 {
    let tw = w - ROW_CHEVRON_ROOM;
    let line = Text::formatted(format_args!(
        "m/{} · {}",
        a.path(coin(net)).as_str(),
        a.starts(net).as_str()
    ));
    let one = ui.wrapped_as(BODY_M, a.name(), tw);
    let two = ui.wrapped_as(BODY_S, line.as_str(), tw);
    let h = (one.h + two.h).max(TOUCH);
    row_press(ui, (x, y, w, h), act(S::Pick(a)));
    let top = y + (h - one.h - two.h) / 2.0;
    ui.draw_wrapped((x, top), Role::OnSurface, &one);
    ui.draw_wrapped((x, top + one.h), Role::OnSurfaceVariant, &two);
    h
}

fn result(app: &View, s: &SeedState, ui: &mut Ui, (x, y, w): (f32, f32, f32)) -> f32 {
    let Some(m) = s.made() else {
        return 0.0;
    };
    let kv = |ui: &mut Ui, cy: f32, k: &str, v: &str| {
        faraday_ui::sheet::kv(ui, x, y + cy, w, k, v, Role::OnSurface)
    };
    let mut cy = kv(
        ui,
        0.0,
        EN.seed_check_fingerprint,
        crate::wallet::fp_text(m.fingerprint).as_str(),
    );
    if let Some(f) = m.without {
        cy += kv(
            ui,
            cy,
            EN.seed_check_without,
            crate::wallet::fp_text(f).as_str(),
        );
    }
    cy += kv(ui, cy, EN.seed_check_path, m.path.as_str());
    cy += kv(ui, cy, EN.seed_check_script, m.script);
    cy += GAP;
    // The xpub with its origin, in SLIP-132's spelling or plain.
    cy += ui.wrap_as(
        (x, y + cy, w),
        BODY_S,
        Role::OnSurfaceVariant,
        EN.seed_check_xpub,
    ) + GAP_SMALL;
    if let Some(sp) = &m.spelled {
        let options = [
            (sp.get(..4).unwrap_or(sp.as_str()), act(S::Spelling(false))),
            (
                m.plain.get(..4).unwrap_or(EN.seed_check_plain),
                act(S::Spelling(true)),
            ),
        ];
        ui.segmented(
            (x, y + cy, SEGMENTED_H),
            LABEL_L.size,
            &options,
            Some(usize::from(s.plain)),
            None,
        );
        cy += SEGMENTED_H + GAP;
    }
    cy += ui.wrap_as(
        (x, y + cy, w),
        BODY_S,
        Role::OnSurfaceVariant,
        m.origin.as_str(),
    ) + GAP_SMALL;
    let shown = match (&m.spelled, s.plain) {
        (Some(sp), false) => sp,
        _ => &m.plain,
    };
    cy += address_press(
        ui,
        (x, y + cy, w),
        (BODY_M.mono(), Role::OnSurface),
        faraday_ui::grouped(shown.as_str()).as_str(),
        Some(act(S::Qr(0))),
    ) + GAP;
    for (label, text, qr) in [
        (EN.seed_check_receive, &m.receive, Some(act(S::Qr(1)))),
        (EN.seed_check_change_address, &m.change, None),
    ] {
        let Some(t) = text else { continue };
        cy += ui.wrap_as((x, y + cy, w), BODY_S, Role::OnSurfaceVariant, label) + GAP_SMALL;
        cy += address_press(
            ui,
            (x, y + cy, w),
            (BODY_M.mono(), Role::OnSurface),
            faraday_ui::grouped(t.as_str()).as_str(),
            qr,
        ) + GAP;
    }
    cy += compare_block(app, s, ui, (x, y + cy, w));
    cy + result_actions(app, s, ui, (x, y + cy, w))
}

/// Compare with the wallet app's, once opened: its field, Scan and From
/// Files, Compare, and the answer. Returns the height, a gap after it.
fn compare_block(app: &View, s: &SeedState, ui: &mut Ui, (x, y, w): (f32, f32, f32)) -> f32 {
    let Some(cmp) = s.compare() else {
        return 0.0;
    };
    let mut cy = ui.wrap_as(
        (x, y, w),
        BODY_S,
        Role::OnSurfaceVariant,
        EN.seed_check_compare_field,
    ) + GAP_SMALL;
    if app.seed_compare_sample_shown() {
        crate::samples::put::offer(ui, act(S::CompareSample));
    }
    let focus = act(S::CompareField);
    ui.field_box((x, y + cy, w), cmp.focus, None);
    ui.typed(
        x + FIELD_PAD,
        y + cy,
        FIELD_H,
        BODY_M.mono().size,
        W::from(BODY_M.mono().face),
        Role::OnSurface,
        cmp.text.as_str(),
        w - 2.0 * FIELD_PAD,
        cmp.focus,
        focus,
    );
    ui.press_box(x, y + cy, w, FIELD_H, focus);
    cy += FIELD_H + GAP;
    let mut rows: Vec<(Text, Style, Action)> = vec![(
        Text::of(EN.seed_check_compare_scan),
        Style::Secondary,
        act(S::CompareScan),
    )];
    let files: Vec<(usize, &str)> = app
        .inbox
        .iter()
        .enumerate()
        .filter(|(_, i)| matches!(i.kind, FileKind::Wallet | FileKind::Key))
        .map(|(k, i)| (k, i.name.as_str()))
        .collect();
    if files.is_empty() {
        rows.push((
            Text::of(EN.seed_check_compare_files),
            Style::Disabled,
            act(S::Compare),
        ));
    }
    for (k, name) in files {
        rows.push((
            Text::fill(EN.seed_check_compare_file, &[name]),
            Style::Secondary,
            act(S::CompareFile(k)),
        ));
    }
    let go = if cmp.text.is_empty() {
        Style::PrimaryOff
    } else {
        Style::Primary
    };
    rows.push((Text::of(EN.seed_check_compare_go), go, act(S::CompareGo)));
    cy += button_rows(ui, x, y + cy, w, &rows) + GAP;
    let line = |ui: &mut Ui, cy: &mut f32, role: Role, text: &str| {
        *cy += ui.wrap_as((x, y + *cy, w), BODY_M, role, text) + GAP_SMALL;
    };
    match s.answer() {
        Some(Answer::Matches(more)) => {
            line(ui, &mut cy, Role::Ok, EN.seed_check_matches);
            if let Some(m) = more {
                line(ui, &mut cy, Role::OnSurfaceVariant, m.as_str());
            }
        }
        Some(Answer::Differs(more, warn)) => {
            line(ui, &mut cy, Role::Error, EN.seed_check_differs);
            if let Some(m) = more {
                let role = if *warn {
                    Role::Warning
                } else {
                    Role::OnSurfaceVariant
                };
                line(ui, &mut cy, role, m.as_str());
            }
        }
        Some(Answer::Refused(why)) => line(ui, &mut cy, Role::Error, why.as_str()),
        None => {}
    }
    cy + GAP
}

/// Result's actions: Another account, Another seed, Keep this key and Done,
/// with the line that a stick in refuses Keep.
fn result_actions(app: &View, s: &SeedState, ui: &mut Ui, (x, y, w): (f32, f32, f32)) -> f32 {
    let keep = if app.may_load_keys() {
        Style::Secondary
    } else {
        Style::Disabled
    };
    let mut rows = vec![
        (
            EN.seed_check_another_account,
            Style::Secondary,
            act(S::Another),
        ),
        (
            EN.seed_check_another_seed,
            Style::Secondary,
            act(S::AnotherSeed),
        ),
    ];
    if s.compare().is_none() {
        rows.push((EN.seed_check_compare, Style::Secondary, act(S::Compare)));
    }
    if !s.is_loaded() {
        rows.push((EN.seed_check_keep, keep, act(S::Keep)));
    }
    rows.push((EN.seed_check_done, Style::Primary, act(S::Done)));
    let mut cy = button_rows(ui, x, y, w, &rows);
    if !s.is_loaded() && !app.may_load_keys() {
        cy += GAP
            + ui.wrap_as(
                (x, y + cy + GAP, w),
                BODY_S,
                Role::OnSurfaceVariant,
                EN.read_need_no_stick,
            );
    }
    cy
}
