//! `Screen::Settings`: Settings and About.

use super::menu_page::{BlockFn, Page, Section, draw_cards, grid};
use super::parts::{SECTION_GAP, section_on, tz_row};
use crate::Action;
use crate::drawing::View;
use crate::stick::settings_choice::{MotionSpeed, MotionStyle};
use crate::strings::{EN, fill};
use crate::ui::pal::*;
use crate::ui::{Style, Ui};
use faraday_draw::tokens::{APPEARANCE_LABEL_GAP, APPEARANCE_LABEL_W, APPEARANCE_ROW_GAP};
use faraday_draw::tokens::{BODY_M, BODY_S, BUTTON_H, LABEL_L, SEGMENTED_H, TITLE_M};
use faraday_draw::tokens::{PAD, SHAPE_M};
use faraday_shell_api::Orientation;
use faraday_ui::parts::wrap_buttons;
use std::sync::OnceLock;

pub(super) mod rows;

pub(crate) use rows::matching as cards_matching;

/// The panic key, beside Power off on Settings: the stick shell powers
/// off at once when Super and S are held this long (`keyboard.rs`).
const PANIC_KEY: &str = EN.app_panic_key_hint;

/// Settings' Appearance (`docs/THEMES.md` §12): Theme, Mode and
/// Contrast, Text size (`docs/TEXT-SIZE.md` §2), then Motion and
/// Speed (`docs/MOTION.md` §3.5), from (x, y) `w` wide, each row's label
/// in a column `label_w` wide or over its control. Returns the height.
fn appearance_body(
    app: &View,
    ui: &mut Ui,
    (x, y, w): (f32, f32, f32),
    label_w: Option<f32>,
) -> f32 {
    let mut h = ui.appearance((x, y, w), label_w, app.theme(), Action::Theme) + APPEARANCE_ROW_GAP;
    let text = (x, y + h, w);
    h += ui.text_size_row(text, label_w, app.text_size(), Action::TextSize) + APPEARANCE_ROW_GAP;
    let style = app.motion_style();
    h += choice_row(
        ui,
        (x, y + h, w),
        label_w,
        (EN.settings_motion, &MotionStyle::ALL),
        (MotionStyle::name, Action::Motion),
        style,
        false,
    );
    // What the chosen style does, as a fact.
    ui.text_as(
        x,
        y + h + APPEARANCE_LABEL_GAP,
        BODY_S,
        Role::OnSurfaceVariant,
        style.line(),
    );
    h += APPEARANCE_LABEL_GAP + ui.line_as(BODY_S) + APPEARANCE_ROW_GAP;
    // Speed applies to Smooth motion and Crossfade: shown off with Off.
    h + choice_row(
        ui,
        (x, y + h, w),
        label_w,
        (EN.settings_speed, &MotionSpeed::ALL),
        (MotionSpeed::name, Action::Speed),
        app.motion_speed(),
        style == MotionStyle::Off,
    )
}

/// How a choice is named, and the press that takes it.
type Name<T> = fn(T) -> &'static str;
type Press<T> = fn(T) -> Action;

/// One of Appearance's rows of choices: its label with every choice,
/// the way to name one and to press it, and the one in force; `off`
/// draws all dimmed, taking no press. Returns its height.
fn choice_row<T: Copy + PartialEq>(
    ui: &mut Ui,
    (x, y, w): (f32, f32, f32),
    label_w: Option<f32>,
    (label, all): (&str, &[T]),
    (name, act): (Name<T>, Press<T>),
    now: T,
    off: bool,
) -> f32 {
    let items: Vec<(&str, Action)> = all.iter().map(|&t| (name(t), act(t))).collect();
    let on = all.iter().position(|&t| t == now);
    let dim = vec![off; all.len()];
    let (cx, cw, top) = match label_w {
        Some(lw) => {
            let line = ui.button_height(SEGMENTED_H);
            ui.text_mid_as((x, y, line), BODY_M, Role::OnSurfaceVariant, label);
            (x + lw, w - lw, y)
        }
        None => {
            ui.text_as(x, y, BODY_M, Role::OnSurfaceVariant, label);
            (x, w, y + ui.line_as(BODY_M) + APPEARANCE_LABEL_GAP)
        }
    };
    top + ui.choices_with((cx, top, cw, SEGMENTED_H), LABEL_L.size, &items, on, &dim) - y
}

/// Settings' sections, listed in the order of the compact list
/// (`docs/LAYOUT.md` §17): Mode first, where a short panel reaches it
/// without scrolling (on Mainnet, where no pill shows, it is the only door
/// to the sheet), then the rest. The wide tile has its own order,
/// [`TILE`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Part {
    Mode,
    Session,
    Time,
    Randomness,
    Appearance,
    Display,
    Remembered,
    About,
}

/// The cards in the wide tile's order (`docs/WORKSPACE.md` §9). On two
/// columns they read down the first and then the second, divided where the
/// columns' heights come out closest (owner, 2026-11-21).
const TILE: [Part; 8] = {
    use Part::*;
    [
        Mode, Appearance, Session, Time, Randomness, Display, Remembered, About,
    ]
};

/// The sections, the same in every app. The preview size is Faraday
/// Demo's host's (`docs/DESKTOP-HOST.md`).
fn parts() -> Vec<Part> {
    use Part::*;
    vec![
        Mode, Session, Time, Randomness, Appearance, Display, Remembered, About,
    ]
}

/// The guide's key of section `i` of [`parts`] (`docs/GUIDE-IN-APP.md`
/// §1.2): read from the list, so a section moved keeps its key. A lookup.
pub(crate) fn part_guide_key(i: usize) -> Option<&'static str> {
    Some(match *parts().get(i)? {
        Part::Mode => "settings.mode",
        Part::Session => "settings.session",
        Part::Time => "settings.time",
        Part::Randomness => "settings.randomness",
        Part::Appearance => "settings.appearance",
        Part::Display => "settings.display",
        Part::Remembered => "settings.remembered",
        Part::About => "settings.about",
    })
}

/// A section's name, as its card and the list of sections show it.
fn part_title(p: Part) -> &'static str {
    match p {
        Part::Mode => EN.settings_mode,
        Part::Session => EN.settings_lock_and_power,
        Part::Time => EN.settings_time,
        Part::Randomness => EN.own_randomness,
        Part::Appearance => EN.settings_appearance,
        Part::Display => EN.settings_display,
        Part::Remembered => EN.settings_what_remembered,
        Part::About => EN.settings_about,
    }
}

/// Section `p` as a card from (x, y) `w` wide. Returns its height and the
/// gap under it.
fn part(app: &View, ui: &mut Ui, p: Part, at: (f32, f32, f32)) -> f32 {
    part_on(app, ui, p, at, Role::SurfaceContainerLow)
}

/// [`part`] with the card filled `fill`.
fn part_on(app: &View, ui: &mut Ui, p: Part, (x, y, w): (f32, f32, f32), fill: Role) -> f32 {
    if p == Part::Mode && ui.compact {
        return mode_card(app, ui, (x, y, w), fill) + SECTION_GAP;
    }
    section_on(ui, (x, y, w), fill, part_title(p), &|ui, x, y0, w| {
        part_body(app, ui, p, (x, y0, w))
    })
}

/// Section `i` of [`parts`] as its card, working as on Settings, from
/// (x, y) `w` wide: what a search shows for a card it found. Returns its
/// height and the gap under it.
pub(crate) fn card(app: &View, ui: &mut Ui, i: u8, at: (f32, f32, f32)) -> f32 {
    parts()
        .get(usize::from(i))
        .map_or(0.0, |&p| part(app, ui, p, at))
}

/// What section `p` holds, from (x, y) `w` wide. Returns its height.
fn part_body(app: &View, ui: &mut Ui, p: Part, at: (f32, f32, f32)) -> f32 {
    match p {
        Part::Mode => crate::modes::screen::settings_row(app, ui, at.0, at.1, at.2),
        Part::Session => session_body(app, ui, at),
        Part::Time => time_body(app, ui, at),
        Part::Randomness => randomness_body(app, ui, at),
        // The labels beside their controls where the column has room.
        Part::Appearance => {
            let label = (!ui.compact).then_some(APPEARANCE_LABEL_W);
            appearance_body(app, ui, at, label)
        }
        Part::Display => {
            let label = (!ui.compact).then_some(APPEARANCE_LABEL_W);
            display_body(app, ui, at, label)
        }
        Part::Remembered => remembered_body(app, ui, at),
        Part::About => about_body(app, ui, at),
    }
}

/// Lock and power: the keys loaded, Lock, Remove every key, Power off (or
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
        items.push((EN.settings_remove_every, Style::Danger, Action::RemoveKeys));
    }
    let leave = if app.online {
        EN.settings_quit
    } else {
        EN.settings_power_off_button
    };
    items.push((leave, Style::Secondary, Action::PowerAsk));
    y += wrap_buttons(ui, x, y, w, BUTTON_H, &items);
    if !app.online {
        y += ui.wrap_as((x, y, w), BODY_S, Role::OnSurfaceVariant, PANIC_KEY) + 6.0;
    }
    for (label, now, choices, action) in idle_rows(app) {
        ui.text_as(x, y, BODY_S, Role::OnSurfaceVariant, label);
        y += ui.advance_as(BODY_S, 22.0);
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

/// Time (`docs/TOTP.md` §2): Now, the time zone and Set the time. Returns
/// its height.
fn time_body(app: &View, ui: &mut Ui, (x, y0, w): (f32, f32, f32)) -> f32 {
    let mut y = y0;
    ui.text_as(x, y, BODY_S, Role::OnSurfaceVariant, EN.settings_now);
    y += ui.line_as(BODY_S) + faraday_draw::tokens::GAP_SMALL;
    y += ui.wrap_as((x, y, w), TITLE_M, Role::OnSurface, &app.now_local_text()) + 10.0;
    y += tz_row(ui, x, y, w, app.tz_chosen()) + 10.0;
    ui.button(
        x,
        y,
        None,
        BUTTON_H,
        EN.settings_set_the_time,
        Style::Secondary,
        Action::OpenSetTime,
    );
    y += ui.button_height(BUTTON_H);
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
pub(crate) const SCALES: &[u16] = &[75, 90, 100, 110, 125, 150, 200];

/// The Display section's place among the sections: where a press of
/// Rotation opens it on a compact window. A lookup.
pub(crate) fn display_section() -> u8 {
    parts()
        .iter()
        .position(|&p| p == Part::Display)
        .unwrap_or(0) as u8
}

/// A row of choices under its label, or beside it in a column
/// `label_w` wide, as Appearance's Text size is drawn. Returns its
/// height.
fn labelled_choices(
    ui: &mut Ui,
    (x, y, w): (f32, f32, f32),
    label_w: Option<f32>,
    label: &str,
    items: &[(&str, Action)],
    on: Option<usize>,
) -> f32 {
    let (cx, cw, top) = match label_w {
        Some(lw) => {
            let line = ui.button_height(SEGMENTED_H);
            ui.text_mid_as((x, y, line), BODY_M, Role::OnSurfaceVariant, label);
            (x + lw, w - lw, y)
        }
        None => {
            ui.text_as(x, y, BODY_M, Role::OnSurfaceVariant, label);
            (x, w, y + ui.line_as(BODY_M) + APPEARANCE_LABEL_GAP)
        }
    };
    top + ui.choices((cx, top, cw, SEGMENTED_H), LABEL_L.size, items, on) - y
}

/// Display (`docs/ORIENTATION.md` §6): Scale, and Rotation where the
/// shell can turn its frame. Returns its height.
fn display_body(app: &View, ui: &mut Ui, (x, y0, w): (f32, f32, f32), label_w: Option<f32>) -> f32 {
    let mut y = y0 + scale_body(app, ui, (x, y0, w), label_w);
    if app.rotation_offered() {
        y += APPEARANCE_ROW_GAP;
        let items = [
            (
                EN.settings_landscape,
                Action::Rotation(Orientation::Landscape),
            ),
            (
                EN.settings_portrait,
                Action::Rotation(Orientation::Portrait),
            ),
        ];
        let on = Some(usize::from(app.orientation() == Orientation::Portrait));
        y += labelled_choices(ui, (x, y, w), label_w, EN.settings_rotation, &items, on);
    }
    y - y0
}

/// Display's Scale row. Returns its height.
fn scale_body(app: &View, ui: &mut Ui, at: (f32, f32, f32), label_w: Option<f32>) -> f32 {
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
    labelled_choices(ui, at, label_w, EN.settings_scale, &items, on)
}

/// What is remembered: the amounts signed and the signatures made, each
/// with how many are remembered and where they are kept. Returns its
/// height.
fn remembered_body(app: &View, ui: &mut Ui, (x, y0, w): (f32, f32, f32)) -> f32 {
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
    let items = [
        (EN.settings_in_open_vault, Action::SealAmounts(true)),
        (EN.settings_until_power_off, Action::SealAmounts(false)),
    ];
    let on = Some(usize::from(!app.seal_amounts()));
    y += labelled_choices(ui, (x, y, w), None, EN.settings_amounts_signed, &items, on);
    y += faraday_draw::tokens::GRID;
    ui.text_as(x, y, BODY_S, Role::OnSurfaceVariant, &line);
    y += ui.line_as(BODY_S) + APPEARANCE_ROW_GAP;
    let n = app.nonces_remembered();
    let line = fill(
        EN.settings_signatures_remembered,
        &[
            &n.to_string(),
            (if n == 1 {
                EN.settings_signature
            } else {
                EN.settings_signatures
            }),
        ],
    );
    let items = [
        (EN.settings_in_open_vault, Action::NonceHistory(true)),
        (EN.settings_nonces_off, Action::NonceHistory(false)),
    ];
    let on = Some(usize::from(!app.nonce_history_kept()));
    y += labelled_choices(ui, (x, y, w), None, EN.settings_signatures_made, &items, on);
    y += faraday_draw::tokens::GRID;
    ui.text_as(x, y, BODY_S, Role::OnSurfaceVariant, &line);
    y += ui.line_as(BODY_S);
    y - y0
}

/// About: the version and what the app is, the start-up self-test with
/// Run again, and the Nakamoto Institute. Returns its height.
fn about_body(app: &View, ui: &mut Ui, (x, y0, w): (f32, f32, f32)) -> f32 {
    let mut y = y0;
    let lines = [
        fill(EN.settings_faraday, &[&crate::version_label().to_string()]),
        EN.settings_keys_are_typed.to_string(),
        EN.settings_own_risk.to_string(),
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
    y += ui.line_as(BODY_S) + faraday_draw::tokens::GAP_SMALL;
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

/// [`draw`] on a compact window (`docs/LAYOUT.md` §17): the sections as
/// Material's list items, Mode's opening its sheet and each other's
/// opening that section as its own page, whose Back is the list again.
fn listed(app: &View, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    use crate::compact::M;
    use crate::panes::screen::{Item, list_item_opens};
    let (x, w) = (x0 + M, cw - 2.0 * M);
    let parts = parts();
    let open = open_part(app, &parts);
    if open.is_some() {
        // A section's card covers the page: the edges fade into its fill.
        ui.moving_panel_page_begin((x0, cw, h), app.list_offset, Role::SurfaceContainerLow);
    } else {
        ui.moving_page_begin((x0, cw, h), app.list_offset);
    }
    let top = M;
    let mut y = top;
    if let Some(p) = open {
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
            y += list_item_opens(ui, (x, y, w), &item);
        }
    }
    // The margin above the first row and under the last.
    crate::compact::screen::finish(app, ui, h, y - top + 2.0 * M);
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
/// one pane at a time); from medium windows up every section a card on
/// the menu page (`docs/MENU-PAGE.md` §8), in two columns where the page
/// holds two, divided where the columns' heights come out closest at the
/// window's width and text size, so a card may move between them as these
/// change (owner, 2026-11-21).
pub(super) fn draw(app: &View, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    if crate::panes::settings_listed(app.window()) {
        return listed(app, ui, x0, cw, h);
    }
    // One tile (`docs/WORKSPACE.md` §9): the raised surface of Tools'
    // tile, the group cards in it as tonal cards in balanced columns.
    let tile = |ui: &mut Ui, (x, y): (f32, f32), w: f32| -> f32 {
        let (ix, iw) = (x + PAD, w - 2.0 * PAD);
        let columns = grid(iw, 2).columns().clamp(1, 2);
        let cards: Vec<BlockFn> = TILE
            .into_iter()
            .map(|p| {
                // The card without the gap `part_on` leaves under it.
                Box::new(move |ui: &mut Ui, (x, y): (f32, f32), w: f32| {
                    part_on(app, ui, p, (x, y, w), Role::SurfaceContainer) - SECTION_GAP
                }) as BlockFn
            })
            .collect();
        let inner = crate::menu::measure(ui, &mut |ui| {
            draw_cards(app, ui, (ix, 0.0, iw), &cards, columns)
        });
        let h = inner + 2.0 * PAD;
        if ui.shows(y, h) {
            crate::screens::hub_layout::tile_surface(ui, (x, y, w, h));
        }
        draw_cards(app, ui, (ix, y + PAD, iw), &cards, columns);
        h
    };
    let sections = [Section::block(None, tile)];
    let page = Page {
        name: EN.settings_settings,
        back: false,
        search: super::page_search(app),
        most_columns: 2,
        fixed_columns: None,
        above: None,
        bar_still: false,
        ways: false,
        panes: None,
    };
    super::menu_page::draw(app, ui, (x0, cw, h), &page, &sections);
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
            let tone = Role::Primary;
            ui.text_as(cx, cy, BODY_S, tone, word);
            ui.fill(cx, cy + line_h - 3.0, ww, 1.0, 0.0, tone);
            ui.press_box(
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

/// The small panel's card holding the Mode row, which opens the mode
/// sheet. Returns its height.
fn mode_card(app: &View, ui: &mut Ui, (x, y, w): (f32, f32, f32), fill: Role) -> f32 {
    use faraday_draw::tokens::MODE_ROW;
    let pad = 22.0;
    let (ix, iw) = (x + pad, w - 2.0 * pad);
    let h = MODE_ROW + 2.0 * pad;
    if !ui.shows(y, h) {
        return h;
    }
    ui.fill(x, y, w, h, SHAPE_M, fill);
    ui.stroke(x, y, w, h, SHAPE_M, Role::OutlineVariant);
    crate::modes::screen::settings_row(app, ui, ix, y + pad, iw);
    h
}
