//! `Screen::Settings`: Settings and About.

use super::parts::{section, tz_row};
use crate::Action;
use crate::drawing::View;
use crate::strings::{EN, fill};
use crate::ui::pal::*;
use crate::ui::{Style, Ui, W};
use faraday_draw::tokens::{
    LABEL, SETTINGS_CARD_GAP, SETTINGS_CARD_HEAD, SETTINGS_CARD_INSET, SETTINGS_CARD_TITLE,
    SETTINGS_CARD_TITLE_Y, SETTINGS_CHOICE_H, SETTINGS_VALUE_GAP, SETTINGS_VALUE_LINE,
    SETTINGS_VALUE_TEXT,
};
use faraday_ui::parts::{title, wrap_buttons};

/// The panic key, beside Power off on Settings: the stick shell powers
/// off at once when Super and S are held this long (`keyboard.rs`).
const PANIC_KEY: &str = EN.app_panic_key_hint;

/// Settings' Time section (`docs/TOTP.md` §2): Now, the time zone and
/// Set the time. Returns its height.
fn time_section(app: &View, ui: &mut Ui, x: f32, y: f32, width: f32) -> f32 {
    // What reads the clock, under the title (`docs/TOTP.md` §2.1).
    let what = crate::compact::screen::measured(ui, |ui| {
        ui.wrap(
            x + 22.0,
            y + 44.0,
            width - 44.0,
            13.0,
            W::R,
            MUTED,
            TIME_FOR,
        )
    });
    let d = what + 8.0;
    let h = 208.0 + d;
    ui.card(x, y, width, h, LINE);
    ui.text(x + 22.0, y + 20.0, 15.0, W::S, TEXT, EN.time_section_time);
    ui.wrap(
        x + 22.0,
        y + 44.0,
        width - 44.0,
        13.0,
        W::R,
        MUTED,
        TIME_FOR,
    );
    ui.text_mid(
        x + 22.0,
        y + 50.0 + d,
        32.0,
        13.0,
        W::R,
        MUTED,
        EN.time_section_now,
    );
    let now = app.now_local_text();
    ui.text_right(x + width - 22.0, y + 50.0 + d, 32.0, 15.0, W::S, TEXT, &now);
    tz_row(ui, x + 22.0, y + 94.0 + d, width - 44.0, app.tz_min());
    ui.button(
        x + 22.0,
        y + 150.0 + d,
        Some(160.0),
        38.0,
        EN.time_section_set_button,
        Style::Secondary,
        Action::OpenSetTime,
    );
    h
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
    ui.text(x, y + h, SETTINGS_VALUE_TEXT, W::R, MUTED, &size);
    h += SETTINGS_VALUE_LINE;
    h
}

/// [`settings`] on a small panel: the same sections, one column, each
/// row of choices wrapping to the panel.
#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
fn settings_compact(app: &View, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    let (x, w) = (x0 + crate::compact::M, cw - 2.0 * crate::compact::M);
    let top = 12.0 - app.list_offset;
    let mut y = top;
    // First, where a short panel reaches it without scrolling: on
    // Mainnet, where no pill shows, it is the only door to the sheet.
    y += mode_card(app, ui, x, y, w) + 14.0;
    y += section(ui, x, y, w, EN.settings_session, &|ui, x, y0, w| {
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
        y += ui.wrap(x, y, w, 13.0, W::R, MUTED, &line) + 10.0;
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
            y += ui.wrap(x, y, w, 13.0, W::R, MUTED, PANIC_KEY) + 10.0;
        }
        for (label, now, choices, action) in idle_rows(app) {
            ui.text(x, y, 13.0, W::R, MUTED, label);
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
            y += ui.wrap(x, y, w, 13.0, W::R, WARN, &line) + 6.0;
        }
        y - y0
    });
    y += section(ui, x, y, w, EN.settings_time, &|ui, x, y0, w| {
        let mut y = y0;
        y += ui.wrap(x, y, w, 13.0, W::R, MUTED, TIME_FOR) + 12.0;
        ui.text(x, y, 13.0, W::R, MUTED, EN.settings_now);
        y += 20.0;
        y += ui.wrap(x, y, w, 15.0, W::S, TEXT, &app.now_local_text()) + 10.0;
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
    });
    y += section(ui, x, y, w, EN.settings_appearance, &|ui, x, y0, w| {
        ui.theme_groups(x, y0, w, 2, app.theme(), Action::Theme)
    });
    y += section(ui, x, y, w, EN.settings_motion, &|ui, x, y0, w| {
        let items = [
            (EN.settings_full, Action::ReduceMotion(false)),
            (EN.settings_reduced, Action::ReduceMotion(true)),
        ];
        let on = Some(usize::from(app.reduce_motion()));
        ui.choices((x, y0, w, 38.0), 14.0, &items, on)
    });
    y += section(ui, x, y, w, EN.settings_display_scale, &|ui, x, y0, w| {
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
    });
    // The desktop app's preview size (`docs/PREVIEW-SIZE.md`): only
    // where the shell offers one.
    if let Some(chosen) = app.preview() {
        y += section(ui, x, y, w, EN.settings_preview_size, &|ui, x, y0, w| {
            preview_body(app, ui, x, y0, w, chosen)
        });
    }
    y += section(ui, x, y, w, EN.settings_signed_amounts, &|ui, x, y0, w| {
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
        ui.text(x, y, 13.0, W::R, MUTED, &line);
        y += 24.0;
        let items = [
            (EN.settings_sealed_into, Action::SealAmounts(true)),
            (EN.settings_kept_until, Action::SealAmounts(false)),
        ];
        let on = Some(usize::from(!app.seal_amounts()));
        y += ui.choices((x, y, w, 38.0), 14.0, &items, on);
        y - y0
    });
    if !app.online {
        y += section(ui, x, y, w, app.medium.caps(), &|ui, x, y0, w| {
            wrap_buttons(
                ui,
                x,
                y0,
                w,
                38.0,
                &[(app.medium.upgrade(), Style::Secondary, Action::UpgradeOpen)],
            ) - 8.0
        });
    }
    y += section(ui, x, y, w, EN.settings_about, &|ui, x, y0, w| {
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
            y += ui.wrap(x, y, w, 13.0, W::R, MUTED, l) + 8.0;
        }
        let s = &EN;
        let (result, tone) = match app.selftest() {
            Some(Ok(n)) => (
                s.settings_selftest_passed.replacen("{}", &n.to_string(), 1),
                OK,
            ),
            Some(Err(c)) => (s.settings_selftest_failed.replacen("{}", c, 1), ERR),
            None => (EN.settings_not_run.to_string(), MUTED),
        };
        y += 4.0;
        ui.text(x, y, 13.0, W::R, MUTED, s.settings_selftest);
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
    });
    crate::compact::screen::finish(app, ui, x0, cw, h, y - top + 8.0);
}

/// About's foot: the Nakamoto Institute, `w` wide. In the online app its
/// name is a link that opens the site in this computer's browser; the
/// stick and the Pi have no browser and no network, and show it as text.
/// Returns its height.
fn institute_note(app: &View, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    const SITE: &str = "nakamotoinstitute.org";
    let note = fill(EN.institute_note_visit, &[SITE]);
    if !app.online {
        return ui.wrap(x, y, w, 13.0, W::R, MUTED, &note);
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
                TEXT
            } else {
                ACCENT
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
            ui.text(cx, cy, 13.0, W::R, MUTED, word);
        }
        cx += ww + space;
    }
    cy + line_h - y
}

#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
pub(super) fn draw(app: &View, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    if ui.compact {
        return settings_compact(app, ui, x0, cw, h);
    }
    let x = x0 + 56.0;
    let width = (cw - 112.0).min(760.0);
    let top = 44.0 - app.list_offset;
    let mut y = top;
    title(ui, x, y, EN.settings_settings);
    y += 64.0;
    // Devices kept out until unplugged (`docs/history/PLAN.md` §4.6) take a line.
    let ignored_h = if app.ignored_inputs().is_empty() {
        0.0
    } else {
        30.0
    };
    // One 46-unit row less where there is no power-off timer.
    let rows = idle_rows(app);
    let fewer = 46.0 * (2 - rows.len()) as f32;
    ui.card(x, y, width, 224.0 - fewer + ignored_h, LINE);
    ui.text(x + 22.0, y + 20.0, 15.0, W::S, TEXT, EN.settings_session);
    let n = app.session.keys.len();
    ui.text(
        x + 22.0,
        y + 46.0,
        13.0,
        W::R,
        MUTED,
        &fill(
            EN.settings_loaded_lost,
            &[
                &n.to_string(),
                (if n == 1 {
                    EN.settings_key
                } else {
                    EN.settings_keys
                }),
            ],
        ),
    );
    let mut bx = x + 22.0;
    if app.holds_secret() {
        bx += ui.button(
            bx,
            y + 74.0,
            None,
            38.0,
            EN.settings_lock,
            Style::Secondary,
            Action::LockAsk,
        ) + 8.0;
        bx += ui.button(
            bx,
            y + 74.0,
            None,
            38.0,
            EN.settings_remove_every,
            Style::Secondary,
            Action::RemoveKeys,
        ) + 8.0;
    }
    bx += ui.button(
        bx,
        y + 74.0,
        None,
        38.0,
        if app.online {
            EN.settings_quit
        } else {
            EN.settings_power_off_button
        },
        Style::Secondary,
        Action::PowerAsk,
    ) + 14.0;
    if !app.online {
        let room = x + width - 22.0 - bx;
        let line = ui.fit(13.0, W::R, PANIC_KEY, room);
        ui.text_mid(bx, y + 74.0, 38.0, 13.0, W::R, MUTED, &line);
    }
    // The idle timers: lock, then power off with nothing left to lose.
    let mut ry = y + 126.0;
    for &(label, now, choices, action) in &rows {
        ui.text_mid(x + 22.0, ry, 38.0, 13.0, W::R, MUTED, label);
        let texts: Vec<String> = choices
            .iter()
            .map(|&m| {
                if m == 0 {
                    EN.settings_never.to_string()
                } else {
                    fill(EN.settings_min, &[&m.to_string()])
                }
            })
            .collect();
        let items: Vec<(&str, Action)> = texts
            .iter()
            .zip(choices)
            .map(|(t, &m)| (t.as_str(), action(m)))
            .collect();
        let on = choices.iter().position(|&m| now == m);
        let room = x + width - 22.0 - (x + 150.0);
        ui.choices((x + 150.0, ry, room, 38.0), 14.0, &items, on);
        ry += 46.0;
    }
    if !app.ignored_inputs().is_empty() {
        let names: Vec<&str> = app
            .ignored_inputs()
            .iter()
            .map(|(_, n)| n.as_str())
            .collect();
        let line = fill(EN.settings_ignored_until, &[&names.join(", ").to_string()]);
        let line = ui.fit(13.0, W::R, &line, width - 44.0);
        ui.text_mid(x + 22.0, ry, 26.0, 13.0, W::R, WARN, &line);
    }
    y += 244.0 - fewer + ignored_h;
    y += time_section(app, ui, x, y, width) + 20.0;
    y += mode_card(app, ui, x, y, width) + 20.0;
    // Each theme a tile in its own colours, in its group, in rows as wide
    // as the card.
    let inner = width - 44.0;
    let per_row = ui.theme_grid(inner);
    let card_h = 54.0 + ui.theme_groups_h(inner, per_row) + 22.0;
    ui.card(x, y, width, card_h, LINE);
    ui.text(x + 22.0, y + 20.0, 15.0, W::S, TEXT, EN.settings_appearance);
    ui.theme_groups(
        x + 22.0,
        y + 54.0,
        inner,
        per_row,
        app.theme(),
        Action::Theme,
    );
    y += card_h + 20.0;
    ui.card(x, y, width, 112.0, LINE);
    ui.text(x + 22.0, y + 20.0, 15.0, W::S, TEXT, EN.settings_motion);
    let items = [
        (EN.settings_full, Action::ReduceMotion(false)),
        (EN.settings_reduced, Action::ReduceMotion(true)),
    ];
    let on = Some(usize::from(app.reduce_motion()));
    ui.choices((x + 22.0, y + 54.0, inner, 38.0), 14.0, &items, on);
    y += 132.0;
    ui.card(x, y, width, 112.0, LINE);
    ui.text(
        x + 22.0,
        y + 20.0,
        15.0,
        W::S,
        TEXT,
        EN.settings_display_scale,
    );
    let scales = [75u16, 90, 100, 110, 125, 150, 200];
    let labels: Vec<String> = scales
        .iter()
        .map(|&pct| {
            if pct == 100 {
                EN.settings_auto.to_string()
            } else {
                format!("{pct} %")
            }
        })
        .collect();
    let items: Vec<(&str, Action)> = labels
        .iter()
        .zip(scales)
        .map(|(l, pct)| (l.as_str(), Action::Scale(pct)))
        .collect();
    let on = scales.iter().position(|&pct| app.scale_pct() == pct);
    ui.choices((x + 22.0, y + 54.0, inner, 38.0), 14.0, &items, on);
    y += 132.0;
    // The desktop app's preview size (`docs/PREVIEW-SIZE.md`): only
    // where the shell offers one.
    if let Some(chosen) = app.preview() {
        let (bx, by) = (x + SETTINGS_CARD_INSET, y + SETTINGS_CARD_HEAD);
        let body =
            faraday_ui::parts::measured(ui, |ui| preview_body(app, ui, bx, by, inner, chosen));
        let card_h = SETTINGS_CARD_HEAD + body + SETTINGS_CARD_INSET;
        ui.card(x, y, width, card_h, LINE);
        ui.text(
            bx,
            y + SETTINGS_CARD_TITLE_Y,
            SETTINGS_CARD_TITLE,
            W::S,
            TEXT,
            EN.settings_preview_size,
        );
        preview_body(app, ui, bx, by, inner, chosen);
        y += card_h + SETTINGS_CARD_GAP;
    }
    ui.card(x, y, width, 112.0, LINE);
    ui.text(
        x + 22.0,
        y + 20.0,
        15.0,
        W::S,
        TEXT,
        EN.settings_signed_amounts,
    );
    let n = app.memory.signed().len();
    ui.text_right(
        x + width - 22.0,
        y + 16.0,
        24.0,
        13.0,
        W::R,
        MUTED,
        &fill(
            EN.settings_remembered,
            &[
                &n.to_string(),
                (if n == 1 {
                    EN.settings_transaction
                } else {
                    EN.settings_transactions
                }),
            ],
        ),
    );
    let items = [
        (EN.settings_sealed_into, Action::SealAmounts(true)),
        (EN.settings_kept_until, Action::SealAmounts(false)),
    ];
    let on = Some(usize::from(!app.seal_amounts()));
    ui.choices((x + 22.0, y + 54.0, inner, 38.0), 14.0, &items, on);
    y += 132.0;
    // Upgrading another stick from this one (`docs/history/PLAN.md` §5.5): never in the
    // online app, which has no boot partition to copy.
    if !app.online {
        ui.card(x, y, width, 112.0, LINE);
        ui.text(x + 22.0, y + 20.0, 15.0, W::S, TEXT, app.medium.caps());
        ui.button(
            x + 22.0,
            y + 54.0,
            None,
            38.0,
            app.medium.upgrade(),
            Style::Secondary,
            Action::UpgradeOpen,
        );
        y += 132.0;
    }
    // The note at its foot, measured first: it wraps on a narrow column.
    ui.c.push_clip(faraday_draw::Rect::new(0, 0, 0, 0));
    let note_h = institute_note(app, ui, x + 22.0, 0.0, width - 44.0);
    ui.c.pop_clip();
    let about_h = 210.0 + note_h + 14.0;
    ui.card(x, y, width, about_h, LINE);
    ui.text(x + 22.0, y + 20.0, 15.0, W::S, TEXT, EN.settings_about);
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
    let mut ly = y + 50.0;
    for l in &lines {
        ui.text(x + 22.0, ly, 13.0, W::R, MUTED, l);
        ly += 26.0;
    }
    // The start-up self-test: what it found, and a run again.
    let s = &EN;
    let (result, tone) = match app.selftest() {
        Some(Ok(n)) => (
            s.settings_selftest_passed.replacen("{}", &n.to_string(), 1),
            OK,
        ),
        Some(Err(c)) => (s.settings_selftest_failed.replacen("{}", c, 1), ERR),
        None => (EN.settings_not_run.to_string(), MUTED),
    };
    ly += 4.0;
    ui.text_mid(x + 22.0, ly, 34.0, 13.0, W::R, MUTED, s.settings_selftest);
    ui.text_mid(x + 140.0, ly, 34.0, 13.0, W::S, tone, &result);
    let rw = ui.measure(13.0, W::S, s.settings_selftest_run) + 32.0;
    ui.button(
        x + width - 22.0 - rw,
        ly,
        Some(rw),
        34.0,
        s.settings_selftest_run,
        Style::Secondary,
        Action::SelfTestRun,
    );
    institute_note(app, ui, x + 22.0, ly + 48.0, width - 44.0);
    // Taller than a short screen: it scrolls.
    let content = y + about_h + 44.0 - top;
    let view = ui.rect(x0, 0.0, cw, h);
    ui.report_scroll(view, content - h);
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
    ui.card(x, y, w, h, LINE);
    crate::modes::screen::settings_row(app, ui, x + pad, y + pad, w - 2.0 * pad);
    h
}
