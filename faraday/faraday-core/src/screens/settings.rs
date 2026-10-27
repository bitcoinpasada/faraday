//! `Screen::Settings`: Settings and About.

use super::parts::{section, tz_row};
use crate::Action;
use crate::drawing::View;
use crate::strings::{EN, fill};
use crate::ui::pal::*;
use crate::ui::{Style, Ui};
use faraday_draw::tokens::{
    APPEARANCE_LABEL_W, APPEARANCE_ROW_GAP, COLUMN_MAX, GAP, GUIDE_AFTER, MARGIN_WIDE, PANE_TOP,
    PANE_TOP_BAR, SETTINGS_CARD_MIN, SPACER, TOUCH,
};
use faraday_draw::tokens::{BODY_M, BODY_S, BUTTON_H, LABEL_L, SEGMENTED_H, TITLE_M};
use faraday_ui::parts::{title, wrap_buttons};
use std::borrow::Cow;
use std::sync::OnceLock;

/// The panic key, beside Power off on Settings: the stick shell powers
/// off at once when Super and S are held this long (`keyboard.rs`).
const PANIC_KEY: &str = EN.app_panic_key_hint;

/// Settings' Appearance (`docs/THEMES.md` §12): Theme, Mode and
/// Contrast, then the Reduce motion switch (`docs/MOTION.md` §3.5), from
/// (x, y) `w` wide, each row's label in a column `label_w` wide or over
/// its control. Returns the height.
fn appearance_body(
    app: &View,
    ui: &mut Ui,
    (x, y, w): (f32, f32, f32),
    label_w: Option<f32>,
) -> f32 {
    let h = ui.appearance((x, y, w), label_w, app.theme(), Action::Theme) + APPEARANCE_ROW_GAP;
    let on = app.reduce_motion();
    let motion = Action::ReduceMotion(!on);
    h + ui.switch_row((x, y + h, w), EN.settings_reduce_motion, on, motion)
}

/// Settings' sections, in their order (`docs/LAYOUT.md` §17): Mode
/// first, where a short panel reaches it without scrolling (on Mainnet,
/// where no pill shows, it is the only door to the sheet), then the rest.
/// On two columns the first holds Mode to Randomness, the second
/// Appearance on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Part {
    Mode,
    Session,
    Time,
    Randomness,
    Appearance,
    Scale,
    Signed,
    About,
}

/// The sections, the same in every app. The preview size is Faraday
/// Demo's host's (`docs/DESKTOP-HOST.md`).
fn parts() -> Vec<Part> {
    use Part::*;
    vec![
        Mode, Session, Time, Randomness, Appearance, Scale, Signed, About,
    ]
}

/// A section's name, as its card and the list of sections show it.
fn part_title(p: Part) -> &'static str {
    match p {
        Part::Mode => EN.settings_mode,
        Part::Session => EN.settings_session,
        Part::Time => EN.settings_time,
        Part::Randomness => EN.own_randomness,
        Part::Appearance => EN.settings_appearance,
        Part::Scale => EN.settings_display_scale,
        Part::Signed => EN.settings_signed_amounts,
        Part::About => EN.settings_about,
    }
}

/// Section `p`'s explainer, shown under its title while Guided is on
/// (the flows' setting): what the section does, in a sentence or two.
fn part_guide(app: &View, p: Part) -> Option<Cow<'static, str>> {
    if !app.guided() {
        return None;
    }
    Some(match p {
        Part::Mode => EN.settings_guide_mode.into(),
        Part::Session => EN.settings_guide_session.into(),
        Part::Time => EN.settings_guide_time.into(),
        Part::Randomness => EN.settings_guide_randomness.into(),
        Part::Appearance => EN.settings_guide_appearance.into(),
        Part::Scale => EN.settings_guide_scale.into(),
        Part::Signed => EN.settings_guide_signed.into(),
        Part::About => EN.settings_guide_about.into(),
    })
}

/// Section `p` as a card from (x, y) `w` wide, its explainer under its
/// title while Guided is on. Returns its height and the gap under it.
fn part(app: &View, ui: &mut Ui, p: Part, (x, y, w): (f32, f32, f32)) -> f32 {
    let guide = part_guide(app, p);
    if p == Part::Mode {
        return mode_card(app, ui, (x, y, w), guide.as_deref()) + 14.0;
    }
    section(ui, x, y, w, part_title(p), &|ui, x, y0, w| {
        let g = guide.as_deref().map_or(0.0, |t| {
            crate::flow::guide_text(ui, (x, y0, w), t) + GUIDE_AFTER
        });
        g + part_body(app, ui, p, (x, y0 + g, w))
    })
}

/// What section `p` holds, from (x, y) `w` wide. Returns its height.
fn part_body(app: &View, ui: &mut Ui, p: Part, at: (f32, f32, f32)) -> f32 {
    match p {
        Part::Mode => 0.0,
        Part::Session => session_body(app, ui, at),
        Part::Time => time_body(app, ui, at),
        Part::Randomness => randomness_body(app, ui, at),
        // The labels beside their controls where the column has room.
        Part::Appearance => {
            let label = (!ui.compact).then_some(APPEARANCE_LABEL_W);
            appearance_body(app, ui, at, label)
        }
        Part::Scale => scale_body(app, ui, at),
        Part::Signed => signed_body(app, ui, at),
        Part::About => about_body(app, ui, at),
    }
}

/// Session: the keys loaded, Lock, Remove every key, Power off (or
/// Quit), the panic key, and the idle timers. Returns its height.
fn session_body(app: &View, ui: &mut Ui, (x, y0, w): (f32, f32, f32)) -> f32 {
    let mut y = y0;
    let n = app.session.keys.len();
    let line = fill(
        EN.settings_loaded,
        &[
            &n.to_string(),
            (if n == 1 {
                EN.settings_key
            } else {
                EN.settings_keys
            }),
        ],
    );
    y += ui.wrap_as((x, y, w), BODY_S, Role::OnSurfaceVariant, &line) + 10.0;
    let mut items: Vec<(&str, Style, Action)> = Vec::new();
    if app.holds_secret() {
        items.push((EN.settings_lock, Style::Secondary, Action::LockAsk));
        items.push((
            EN.settings_remove_every,
            Style::Secondary,
            Action::RemoveKeys,
        ));
    }
    let leave = if app.online {
        EN.settings_quit
    } else {
        EN.settings_power_off_button
    };
    items.push((leave, Style::Secondary, Action::PowerAsk));
    y += wrap_buttons(ui, x, y, w, BUTTON_H, &items) + 4.0;
    if !app.online {
        y += ui.wrap_as((x, y, w), BODY_S, Role::OnSurfaceVariant, PANIC_KEY) + 10.0;
    }
    for (label, now, choices, action) in idle_rows(app) {
        ui.text_as(x, y, BODY_S, Role::OnSurfaceVariant, label);
        y += 22.0;
        let items: Vec<(&str, Action)> = choices.iter().map(|&m| (minutes(m), action(m))).collect();
        let on = choices.iter().position(|&m| now == m);
        y += ui.choices((x, y, w, SEGMENTED_H), LABEL_L.size, &items, on) + 10.0;
    }
    if !app.ignored_inputs().is_empty() {
        let names: Vec<&str> = app
            .ignored_inputs()
            .iter()
            .map(|(_, n)| n.as_str())
            .collect();
        let line = fill(EN.settings_ignored_until, &[&names.join(", ").to_string()]);
        y += ui.wrap_as((x, y, w), BODY_M, Role::Warning, &line) + 6.0;
    }
    y - y0
}

/// Time (`docs/TOTP.md` §2): Now, the time zone and Set the time; what
/// reads the clock is its explainer, under Guided. Returns its height.
fn time_body(app: &View, ui: &mut Ui, (x, y0, w): (f32, f32, f32)) -> f32 {
    let mut y = y0;
    ui.text_as(x, y, BODY_S, Role::OnSurfaceVariant, EN.settings_now);
    y += 20.0;
    y += ui.wrap_as((x, y, w), TITLE_M, Role::OnSurface, &app.now_local_text()) + 10.0;
    y += tz_row(ui, x, y, w, app.tz_min()) + 10.0;
    y += ui.button(
        x,
        y,
        None,
        BUTTON_H,
        EN.settings_set_the_time,
        Style::Secondary,
        Action::OpenSetTime,
    );
    y - y0
}

/// Randomness (`docs/OWN-RANDOMNESS.md` §2.2, §2.4): the status line and
/// Add randomness. Returns its height.
fn randomness_body(app: &View, ui: &mut Ui, (x, y0, w): (f32, f32, f32)) -> f32 {
    use faraday_draw::tokens::{GAP, OWN_BUTTON};
    let status = app.own_status();
    let mut y = y0;
    y += ui.wrap_as((x, y, w), BODY_S, Role::OnSurfaceVariant, &status) + GAP;
    ui.button(
        x,
        y,
        None,
        OWN_BUTTON,
        EN.own_add_randomness,
        Style::Secondary,
        Action::Own(crate::fresh::page::Act::Open),
    );
    y += OWN_BUTTON;
    y - y0
}

/// What an idle timer's choice of `m` minutes says ("Never" for 0),
/// made once and kept: drawing reads it and allocates nothing.
fn minutes(m: u16) -> &'static str {
    use crate::stick::settings::{IDLE_LOCK_CHOICES, IDLE_OFF_CHOICES};
    static LABELS: OnceLock<Vec<(u16, String)>> = OnceLock::new();
    let labels = LABELS.get_or_init(|| {
        IDLE_LOCK_CHOICES
            .iter()
            .chain(&IDLE_OFF_CHOICES)
            .map(|&m| (m, fill(EN.settings_min, &[&m.to_string()])))
            .collect()
    });
    match labels.iter().find(|(n, _)| *n == m) {
        Some((_, l)) if m != 0 => l,
        _ => EN.settings_never,
    }
}

/// Display scale's choices, in percent; 100 is Auto.
const SCALES: &[u16] = &[75, 90, 100, 110, 125, 150, 200];

/// Display scale's choices. Returns their height.
fn scale_body(app: &View, ui: &mut Ui, (x, y0, w): (f32, f32, f32)) -> f32 {
    // Made once and kept: drawing reads them and allocates nothing.
    static LABELS: OnceLock<Vec<String>> = OnceLock::new();
    let labels = LABELS.get_or_init(|| SCALES.iter().map(|pct| format!("{pct} %")).collect());
    let items: Vec<(&str, Action)> = SCALES
        .iter()
        .zip(labels)
        .map(|(&pct, label)| {
            let label = if pct == 100 {
                EN.settings_auto
            } else {
                label.as_str()
            };
            (label, Action::Scale(pct))
        })
        .collect();
    let on = SCALES.iter().position(|&pct| app.scale_pct() == pct);
    ui.choices((x, y0, w, SEGMENTED_H), LABEL_L.size, &items, on)
}

/// Signed amounts: how many are remembered, and whether they are sealed
/// into the vault or kept until power-off. Returns its height.
fn signed_body(app: &View, ui: &mut Ui, (x, y0, w): (f32, f32, f32)) -> f32 {
    let mut y = y0;
    let n = app.memory.signed().len();
    let line = fill(
        EN.settings_remembered,
        &[
            &n.to_string(),
            (if n == 1 {
                EN.settings_transaction
            } else {
                EN.settings_transactions
            }),
        ],
    );
    ui.text_as(x, y, BODY_S, Role::OnSurfaceVariant, &line);
    y += 24.0;
    let items = [
        (EN.settings_sealed_into, Action::SealAmounts(true)),
        (EN.settings_kept_until, Action::SealAmounts(false)),
    ];
    let on = Some(usize::from(!app.seal_amounts()));
    y += ui.choices((x, y, w, SEGMENTED_H), LABEL_L.size, &items, on);
    y - y0
}

/// About: the version and what the app is, the start-up self-test with
/// Run again, and the Nakamoto Institute. Returns its height.
fn about_body(app: &View, ui: &mut Ui, (x, y0, w): (f32, f32, f32)) -> f32 {
    let mut y = y0;
    let lines = [
        fill(EN.settings_faraday, &[&crate::version_label().to_string()]),
        EN.settings_keys_are_typed.to_string(),
        fill(
            EN.settings_vaults_files,
            &[app.medium.noun(), app.medium.a()],
        ),
        if app.online {
            fill(EN.settings_are_folders, &[app.medium.caps()])
        } else {
            fill(EN.settings_medium_reader_note, &[app.medium.caps()])
        },
    ];
    for l in &lines {
        y += ui.wrap_as((x, y, w), BODY_S, Role::OnSurfaceVariant, l) + 8.0;
    }
    let s = &EN;
    let (result, tone) = match app.selftest() {
        Some(Ok(n)) => (
            s.settings_selftest_passed.replacen("{}", &n.to_string(), 1),
            Role::Ok,
        ),
        Some(Err(c)) => (s.settings_selftest_failed.replacen("{}", c, 1), Role::Error),
        None => (EN.settings_not_run.to_string(), Role::OnSurfaceVariant),
    };
    y += 4.0;
    ui.text_as(x, y, BODY_S, Role::OnSurfaceVariant, s.settings_selftest);
    y += 20.0;
    y += ui.wrap_as((x, y, w), LABEL_L, tone, &result) + 8.0;
    y += wrap_buttons(
        ui,
        x,
        y,
        w,
        BUTTON_H,
        &[(
            s.settings_selftest_run,
            Style::Secondary,
            Action::SelfTestRun,
        )],
    ) + 4.0;
    y += institute_note(app, ui, x, y, w);
    y - y0
}

/// Steps only or Guided at (x, y), the flows' control for the same
/// setting: with Guided on, each card carries its explainer. Returns the
/// height its row takes.
fn guided_row(app: &View, ui: &mut Ui, x: f32, y: f32) -> f32 {
    crate::flow::guided_choice(ui, (x, y), app.guided());
    TOUCH
}

/// [`draw`] on a compact window (`docs/LAYOUT.md` §17): the sections as
/// Material's list items, Mode's opening its sheet and each other's
/// opening that section as its own page, whose Back is the list again.
fn listed(app: &View, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    use crate::compact::M;
    use crate::panes::screen::{Item, list_item};
    let (x, w) = (x0 + M, cw - 2.0 * M);
    let top = M - app.list_offset;
    let mut y = top;
    y += guided_row(app, ui, x, y);
    let parts = parts();
    if let Some(p) = open_part(app, &parts) {
        y += part(app, ui, p, (x, y, w));
    } else {
        for (k, &q) in parts.iter().enumerate() {
            let mode = q == Part::Mode;
            let item = Item {
                headline: part_title(q),
                supporting: if mode { app.mode().name() } else { "" },
                trailing: None,
                action: if mode {
                    Action::ModeSheet
                } else {
                    Action::SettingsSection(Some(k as u8))
                },
            };
            y += list_item(ui, (x, y, w), &item, false);
        }
    }
    // The margin above the first row and under the last.
    crate::compact::screen::finish(app, ui, x0, cw, h, y - top + 2.0 * M);
}

/// The section open as its own page on a compact window, if one is.
fn open_part(app: &View, parts: &[Part]) -> Option<Part> {
    let k = app.scroll.panes().settings()?;
    parts.get(usize::from(k)).copied()
}

/// The name of the Settings section open as its own page, for the top
/// app bar, and its way back, the list: none while the list shows or
/// every section is on the page. A lookup.
pub(crate) fn open_head(app: &View) -> Option<&'static str> {
    if !crate::panes::settings_listed(app.window()) {
        return None;
    }
    open_part(app, &parts()).map(part_title)
}

/// Settings (`docs/LAYOUT.md` §17): on a compact window the list of
/// sections, each opening as its own page (Material's list and detail
/// one pane at a time); from medium windows up every section on one
/// page, in two columns where the page holds two cards
/// `SETTINGS_CARD_MIN` wide (owner, 2026-10-25).
pub(super) fn draw(app: &View, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    if crate::panes::settings_listed(app.window()) {
        return listed(app, ui, x0, cw, h);
    }
    let room = cw - 2.0 * MARGIN_WIDE;
    let two = room >= 2.0 * SETTINGS_CARD_MIN + SPACER;
    let col_w = if two { (room - SPACER) / 2.0 } else { room }.min(COLUMN_MAX);
    let across = if two { 2.0 * col_w + SPACER } else { col_w };
    let x = x0 + (cw - across) / 2.0;
    let mut top = (if ui.compact { PANE_TOP_BAR } else { PANE_TOP }) - app.list_offset;
    if !ui.compact {
        title(ui, x, top - PANE_TOP + 28.0, EN.settings_settings);
    }
    top += guided_row(app, ui, x, top);
    let (mut left, mut right) = (top, top);
    for p in parts() {
        if two && (p as u8) >= (Part::Appearance as u8) {
            right += part(app, ui, p, (x + col_w + SPACER, right, col_w));
        } else {
            left += part(app, ui, p, (x, left, col_w));
        }
    }
    let foot = left.max(right) + MARGIN_WIDE;
    crate::compact::screen::finish(app, ui, x0, cw, h, foot + app.list_offset);
}

/// About's foot: the Nakamoto Institute, `w` wide. In the online app its
/// name is a link that opens the site in this computer's browser; the
/// stick and the Pi have no browser and no network, and show it as text.
/// Returns its height.
fn institute_note(app: &View, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    const SITE: &str = "nakamotoinstitute.org";
    let note = fill(EN.institute_note_visit, &[SITE]);
    if !app.online {
        return ui.wrap_as((x, y, w), BODY_S, Role::OnSurfaceVariant, &note);
    }
    // Word by word, breaking at a space as `Ui::wrap` does, the site's
    // name in the accent, underlined and pressable.
    let line_h = ui.line_as(BODY_S);
    let space = ui.measure_as(BODY_S, " ");
    let (mut cx, mut cy) = (x, y);
    for word in note.split_whitespace() {
        let ww = ui.measure_as(BODY_S, word);
        if cx > x && cx + ww > x + w {
            cx = x;
            cy += line_h;
        }
        if word == SITE {
            let tone = if ui.is_pressed(Action::OpenInstitute) {
                Role::OnSurface
            } else {
                Role::Primary
            };
            ui.text_as(cx, cy, BODY_S, tone, word);
            ui.fill(cx, cy + line_h - 3.0, ww, 1.0, 0.0, tone);
            ui.hit(
                cx - 2.0,
                cy - 2.0,
                ww + 4.0,
                line_h + 4.0,
                Action::OpenInstitute,
            );
        } else {
            ui.text_as(cx, cy, BODY_S, Role::OnSurfaceVariant, word);
        }
        cx += ww + space;
    }
    cy + line_h - y
}

/// One idle timer on Settings: its label, its minutes now, the choices
/// (0 for never), and the action that sets it.
type IdleRow = (&'static str, u16, &'static [u16], fn(u16) -> Action);

/// Settings' idle timers: lock always, power off only where the device
/// can power off (the desktop app quits by the button alone, no timer).
fn idle_rows(app: &View) -> Vec<IdleRow> {
    let mut rows: Vec<IdleRow> = vec![(
        EN.settings_lock_after,
        app.idle_lock_min(),
        &crate::stick::settings::IDLE_LOCK_CHOICES,
        Action::IdleLock,
    )];
    if !app.online {
        rows.push((
            EN.settings_power_off,
            app.idle_off_min(),
            &crate::stick::settings::IDLE_OFF_CHOICES,
            Action::IdleOff,
        ));
    }
    rows
}

/// The card holding the Mode row, which opens the mode sheet, and its
/// explainer under the row while Guided is on. Returns its height.
fn mode_card(app: &View, ui: &mut Ui, (x, y, w): (f32, f32, f32), guide: Option<&str>) -> f32 {
    use faraday_draw::tokens::MODE_ROW;
    let pad = 22.0;
    let (ix, iw) = (x + pad, w - 2.0 * pad);
    let at = (ix, y + pad + MODE_ROW, iw);
    let gh = guide.map_or(0.0, |t| {
        crate::compact::screen::measured(ui, |ui| crate::flow::guide_text(ui, at, t)) + GAP
    });
    let h = MODE_ROW + gh + 2.0 * pad;
    if !ui.shows(y, h) {
        return h;
    }
    ui.card(x, y, w, h, Role::OutlineVariant);
    crate::modes::screen::settings_row(app, ui, ix, y + pad, iw);
    if let Some(t) = guide {
        crate::flow::guide_text(ui, (ix, y + pad + MODE_ROW + GAP, iw), t);
    }
    h
}
