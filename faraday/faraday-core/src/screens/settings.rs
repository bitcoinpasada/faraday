//! `Screen::Settings`: Settings and About.

use super::parts::{section, tz_row};
use crate::Action;
use crate::drawing::View;
use crate::strings::{EN, fill};
use crate::ui::pal::*;
use crate::ui::{Style, Ui, W};
use faraday_draw::tokens::{
    APPEARANCE_LABEL_W, APPEARANCE_ROW_GAP, LABEL, SETTINGS_CHOICE_H, SETTINGS_VALUE_GAP,
    SETTINGS_VALUE_LINE, SETTINGS_VALUE_TEXT,
};
use faraday_ui::parts::{title, wrap_buttons};

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

/// What the Time card says reads the clock: everything that calls
/// `clock_now`.
const TIME_FOR: &str = EN.app_for_totp_codes;

/// Preview size's choices, in Settings' order (`docs/PREVIEW-SIZE.md`):
/// the window itself, then each of `PREVIEW_SIZES`.
fn preview_items() -> [(&'static str, Action); 8] {
    let s = faraday_draw::tokens::PREVIEW_SIZES;
    [
        (EN.settings_preview_this_window, Action::PreviewSize(None)),
        (EN.settings_preview_pi, Action::PreviewSize(Some(s[0]))),
        (EN.settings_preview_narrow, Action::PreviewSize(Some(s[1]))),
        (EN.settings_preview_half, Action::PreviewSize(Some(s[2]))),
        (EN.settings_preview_7in, Action::PreviewSize(Some(s[3]))),
        (EN.settings_preview_10in, Action::PreviewSize(Some(s[4]))),
        (EN.settings_preview_desktop, Action::PreviewSize(Some(s[5]))),
        (EN.settings_preview_monitor, Action::PreviewSize(Some(s[6]))),
    ]
}

/// Preview size's choices and, under them, the size the screens are laid
/// out at, in dp, from (x, y) `w` wide; `chosen` is the size shown.
/// Returns its height.
fn preview_body(
    app: &View,
    ui: &mut Ui,
    x: f32,
    y: f32,
    w: f32,
    chosen: Option<(u16, u16)>,
) -> f32 {
    let items = preview_items();
    let on = items
        .iter()
        .position(|(_, a)| *a == Action::PreviewSize(chosen));
    let mut h = ui.choices((x, y, w, SETTINGS_CHOICE_H), LABEL, &items, on) + SETTINGS_VALUE_GAP;
    let window = app.window();
    let size = fill(
        EN.settings_preview_dp,
        &[
            &format!("{:.0}", window.width),
            &format!("{:.0}", window.height),
        ],
    );
    ui.text(
        x,
        y + h,
        SETTINGS_VALUE_TEXT,
        W::R,
        Role::OnSurfaceVariant,
        &size,
    );
    h += SETTINGS_VALUE_LINE;
    h
}

/// Settings' sections, in their order (`docs/LAYOUT.md` §17): Mode
/// first, where a short panel reaches it without scrolling (on Mainnet,
/// where no pill shows, it is the only door to the sheet), then the rest.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Part {
    Mode,
    Session,
    Time,
    Randomness,
    Appearance,
    Scale,
    Preview,
    Signed,
    Stick,
    About,
}

/// The sections this app has: Preview size only where the shell offers
/// one (`docs/PREVIEW-SIZE.md`), the stick's upgrade never in the online
/// app, which has no boot partition to copy (`docs/history/PLAN.md` §5.5).
fn parts(app: &View) -> Vec<Part> {
    use Part::*;
    let mut v = vec![Mode, Session, Time, Randomness, Appearance, Scale];
    if app.preview().is_some() {
        v.push(Preview);
    }
    v.push(Signed);
    if !app.online {
        v.push(Stick);
    }
    v.push(About);
    v
}

/// A section's name, as its card and the list pane show it.
fn part_title(app: &View, p: Part) -> &'static str {
    match p {
        Part::Mode => EN.settings_mode,
        Part::Session => EN.settings_session,
        Part::Time => EN.settings_time,
        Part::Randomness => EN.own_randomness,
        Part::Appearance => EN.settings_appearance,
        Part::Scale => EN.settings_display_scale,
        Part::Preview => EN.settings_preview_size,
        Part::Signed => EN.settings_signed_amounts,
        Part::Stick => app.medium.caps(),
        Part::About => EN.settings_about,
    }
}

/// Section `p` as a card from (x, y) `w` wide. Returns its height and
/// the gap under it.
fn part(app: &View, ui: &mut Ui, p: Part, (x, y, w): (f32, f32, f32)) -> f32 {
    if p == Part::Mode {
        return mode_card(app, ui, x, y, w) + 14.0;
    }
    section(ui, x, y, w, part_title(app, p), &|ui, x, y0, w| {
        part_body(app, ui, p, (x, y0, w))
    })
}

/// What section `p` holds, from (x, y) `w` wide. Returns its height.
fn part_body(app: &View, ui: &mut Ui, p: Part, at: (f32, f32, f32)) -> f32 {
    let (x, y, w) = at;
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
        Part::Preview => app
            .preview()
            .map_or(0.0, |chosen| preview_body(app, ui, x, y, w, chosen)),
        Part::Signed => signed_body(app, ui, at),
        Part::Stick => {
            wrap_buttons(
                ui,
                x,
                y,
                w,
                38.0,
                &[(app.medium.upgrade(), Style::Secondary, Action::UpgradeOpen)],
            ) - 8.0
        }
        Part::About => about_body(app, ui, at),
    }
}

/// Session: what a lock loses, Lock, Remove every key, Power off (or
/// Quit), the panic key, and the idle timers. Returns its height.
fn session_body(app: &View, ui: &mut Ui, (x, y0, w): (f32, f32, f32)) -> f32 {
    let mut y = y0;
    let n = app.session.keys.len();
    let line = fill(
        EN.settings_loaded_lost,
        &[
            &n.to_string(),
            (if n == 1 {
                EN.settings_key
            } else {
                EN.settings_keys
            }),
        ],
    );
    y += ui.wrap(x, y, w, 13.0, W::R, Role::OnSurfaceVariant, &line) + 10.0;
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
    y += wrap_buttons(ui, x, y, w, 38.0, &items) + 4.0;
    if !app.online {
        y += ui.wrap(x, y, w, 13.0, W::R, Role::OnSurfaceVariant, PANIC_KEY) + 10.0;
    }
    for (label, now, choices, action) in idle_rows(app) {
        ui.text(x, y, 13.0, W::R, Role::OnSurfaceVariant, label);
        y += 22.0;
        let labels: Vec<String> = choices
            .iter()
            .map(|&m| {
                if m == 0 {
                    EN.settings_never.to_string()
                } else {
                    fill(EN.settings_min, &[&m.to_string()])
                }
            })
            .collect();
        let items: Vec<(&str, Action)> = labels
            .iter()
            .zip(choices.iter())
            .map(|(l, &m)| (l.as_str(), action(m)))
            .collect();
        let on = choices.iter().position(|&m| now == m);
        y += ui.choices((x, y, w, 36.0), 13.0, &items, on) + 10.0;
    }
    if !app.ignored_inputs().is_empty() {
        let names: Vec<&str> = app
            .ignored_inputs()
            .iter()
            .map(|(_, n)| n.as_str())
            .collect();
        let line = fill(EN.settings_ignored_until, &[&names.join(", ").to_string()]);
        y += ui.wrap(x, y, w, 13.0, W::R, Role::Warning, &line) + 6.0;
    }
    y - y0
}

/// Time (`docs/TOTP.md` §2): what reads the clock, Now, the time zone and
/// Set the time. Returns its height.
fn time_body(app: &View, ui: &mut Ui, (x, y0, w): (f32, f32, f32)) -> f32 {
    let mut y = y0;
    y += ui.wrap(x, y, w, 13.0, W::R, Role::OnSurfaceVariant, TIME_FOR) + 12.0;
    ui.text(x, y, 13.0, W::R, Role::OnSurfaceVariant, EN.settings_now);
    y += 20.0;
    y += ui.wrap(x, y, w, 15.0, W::S, Role::OnSurface, &app.now_local_text()) + 10.0;
    y += tz_row(ui, x, y, w, app.tz_min()) + 10.0;
    y += ui.button(
        x,
        y,
        None,
        38.0,
        EN.settings_set_the_time,
        Style::Secondary,
        Action::OpenSetTime,
    );
    y - y0
}

/// Randomness (`docs/OWN-RANDOMNESS.md` §2.2, §2.4): the status line and
/// Add randomness. Returns its height.
fn randomness_body(app: &View, ui: &mut Ui, (x, y0, w): (f32, f32, f32)) -> f32 {
    use faraday_draw::tokens::{GAP, OWN_BUTTON, OWN_LINE_TEXT};
    let status = app.own_status();
    let mut y = y0;
    y += ui.wrap(
        x,
        y,
        w,
        OWN_LINE_TEXT,
        W::R,
        Role::OnSurfaceVariant,
        &status,
    ) + GAP;
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

/// Display scale's choices. Returns their height.
fn scale_body(app: &View, ui: &mut Ui, (x, y0, w): (f32, f32, f32)) -> f32 {
    let labels: Vec<(String, u16)> = [75u16, 90, 100, 110, 125, 150, 200]
        .into_iter()
        .map(|pct| {
            (
                if pct == 100 {
                    EN.settings_auto.to_string()
                } else {
                    format!("{pct} %")
                },
                pct,
            )
        })
        .collect();
    let items: Vec<(&str, Action)> = labels
        .iter()
        .map(|(l, pct)| (l.as_str(), Action::Scale(*pct)))
        .collect();
    let on = labels.iter().position(|(_, pct)| app.scale_pct() == *pct);
    ui.choices((x, y0, w, 36.0), 13.0, &items, on)
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
    ui.text(x, y, 13.0, W::R, Role::OnSurfaceVariant, &line);
    y += 24.0;
    let items = [
        (EN.settings_sealed_into, Action::SealAmounts(true)),
        (EN.settings_kept_until, Action::SealAmounts(false)),
    ];
    let on = Some(usize::from(!app.seal_amounts()));
    y += ui.choices((x, y, w, 38.0), 14.0, &items, on);
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
        y += ui.wrap(x, y, w, 13.0, W::R, Role::OnSurfaceVariant, l) + 8.0;
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
    ui.text(
        x,
        y,
        13.0,
        W::R,
        Role::OnSurfaceVariant,
        s.settings_selftest,
    );
    y += 20.0;
    y += ui.wrap(x, y, w, 13.0, W::S, tone, &result) + 8.0;
    y += wrap_buttons(
        ui,
        x,
        y,
        w,
        34.0,
        &[(
            s.settings_selftest_run,
            Style::Secondary,
            Action::SelfTestRun,
        )],
    ) + 4.0;
    y += institute_note(app, ui, x, y, w);
    y - y0
}

/// [`draw`] on compact and medium windows: every section, one column,
/// each row of choices wrapping to the panel.
fn settings_compact(app: &View, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    let (x, w) = (x0 + crate::compact::M, cw - 2.0 * crate::compact::M);
    let top = 12.0 - app.list_offset;
    let mut y = top;
    for p in parts(app) {
        y += part(app, ui, p, (x, y, w));
    }
    crate::compact::screen::finish(app, ui, x0, cw, h, y - top + 8.0);
}

/// Settings (`docs/LAYOUT.md` §17): on compact and medium windows every
/// section in one column; from expanded windows up, list and detail,
/// the sections in the list pane and the one chosen beside it (owner,
/// 2026-10-24), Material's pattern for settings.
pub(super) fn draw(app: &View, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    use crate::panes::screen::{Item, ListPane, shown, split};
    if !shown(app, crate::Screen::Settings) {
        return settings_compact(app, ui, x0, cw, h);
    }
    let p = split(ui, x0, cw);
    if !ui.compact {
        title(ui, p.list.0, 28.0, EN.settings_settings);
    }
    let parts = parts(app);
    let at = usize::from(app.scroll.panes().settings()).min(parts.len() - 1);
    let mut pane = ListPane::begin(app, ui, (p.list.0, p.top, p.list.1, h - p.top));
    for (k, &q) in parts.iter().enumerate() {
        let item = Item {
            headline: part_title(app, q),
            supporting: "",
            trailing: None,
            action: Action::SettingsSection(k as u8),
        };
        pane.item(ui, &item, k == at);
    }
    pane.end(ui);
    // The chosen section, in the detail's reading width, scrolled on
    // its own.
    let (dx, dw) = p.detail;
    let clip = ui.rect(dx, p.top, dw, h - p.top);
    ui.c.push_clip(clip);
    let top = p.top - app.list_offset;
    let ch = part(app, ui, parts[at], (dx, top, p.reading()));
    ui.c.pop_clip();
    app.scroll.content_h().set(ch);
    ui.report_scroll(clip, ch - (h - p.top));
}

/// About's foot: the Nakamoto Institute, `w` wide. In the online app its
/// name is a link that opens the site in this computer's browser; the
/// stick and the Pi have no browser and no network, and show it as text.
/// Returns its height.
fn institute_note(app: &View, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    const SITE: &str = "nakamotoinstitute.org";
    let note = fill(EN.institute_note_visit, &[SITE]);
    if !app.online {
        return ui.wrap(x, y, w, 13.0, W::R, Role::OnSurfaceVariant, &note);
    }
    // Word by word, breaking at a space as `Ui::wrap` does, the site's
    // name in the accent, underlined and pressable.
    let line_h = ui.line(13.0, W::R);
    let space = ui.measure(13.0, W::R, " ");
    let (mut cx, mut cy) = (x, y);
    for word in note.split_whitespace() {
        let ww = ui.measure(13.0, W::R, word);
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
            ui.text(cx, cy, 13.0, W::R, tone, word);
            ui.fill(cx, cy + line_h - 3.0, ww, 1.0, 0.0, tone);
            ui.hit(
                cx - 2.0,
                cy - 2.0,
                ww + 4.0,
                line_h + 4.0,
                Action::OpenInstitute,
            );
        } else {
            ui.text(cx, cy, 13.0, W::R, Role::OnSurfaceVariant, word);
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

/// The card holding the Mode row, which opens the mode sheet. Returns
/// its height.
fn mode_card(app: &View, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    let pad = 22.0;
    let h = faraday_draw::tokens::MODE_ROW + 2.0 * pad;
    ui.card(x, y, w, h, Role::OutlineVariant);
    crate::modes::screen::settings_row(app, ui, x + pad, y + pad, w - 2.0 * pad);
    h
}
