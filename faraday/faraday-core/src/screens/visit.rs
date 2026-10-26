//! `Screen::Visit`: a stick visit.

use super::parts::{guide_text_learn, kind_line, pull_line, size_text};
use crate::drawing::View;
use crate::strings::{EN, fill};
use crate::ui::pal::*;
use crate::ui::{Style, Ui, W};
use crate::wallet::FileKind;
use crate::{Action, Faraday, Screen};
use faraday_draw::widgets::Icon;
use faraday_ui::parts::{title, wrap_buttons};

/// [`visit`] on a small panel: one scrolled column. The sticks, what is
/// left to do, then what to write and what to import, each a list with
/// its buttons under it.
#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
fn visit_compact(app: &View, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    use crate::compact::M;
    use crate::compact::screen::{about_learn, finish};
    let (x, w) = (x0 + M, cw - 2.0 * M);
    let top = 12.0 - app.list_offset;
    let mut y = top;
    app.visit.bar(crate::Column::Outbox).set(None);
    app.visit.bar(crate::Column::Stick).set(None);
    let Some(stick) = app.stick.get(app.visit.stick) else {
        ui.text(
            x,
            y,
            15.0,
            W::R,
            Role::OnSurfaceVariant,
            &fill(EN.visit_no_attached, &[app.medium.noun()]),
        );
        return;
    };
    // Where pulling the stick goes.
    let then = ui.fit(12.0, W::R, &app.visit_then(), w - 34.0);
    ui.chip(
        x,
        y,
        &then,
        Role::OnSecondaryContainer,
        Role::SecondaryContainer,
    );
    y += 36.0;
    // One button per stick, the chosen one filled.
    if app.stick.len() > 1 {
        let items: Vec<(&str, Style, Action)> = app
            .stick
            .iter()
            .enumerate()
            .map(|(k, st)| {
                let style = if k == app.visit.stick {
                    Style::Primary
                } else {
                    Style::Secondary
                };
                (st.label.as_str(), style, Action::VisitStick(k))
            })
            .collect();
        y += wrap_buttons(ui, x, y, w, 34.0, &items) + 4.0;
    } else {
        ui.text(x, y, 13.0, W::S, Role::OnSurfaceVariant, &stick.label);
        y += 26.0;
    }
    // What is left to do: pull the stick, and where the copied files went.
    y += crate::vault::screen::stick_banner(ui, x, y, w, &pull_line(app));
    ui.button(
        x,
        y,
        Some(w),
        40.0,
        EN.visit_open_files,
        Style::Secondary,
        Action::Nav(Screen::Files),
    );
    y += 50.0;
    for (line, ok) in app.visit.log.iter().rev().take(2) {
        let c = if *ok { Role::Ok } else { Role::Error };
        y += ui.wrap(x, y, w, 13.0, W::R, c, line) + 6.0;
    }
    y += 8.0;

    // Import into the Inbox.
    ui.icon(x, y, 20.0, Icon::Download, 13.0, Role::Primary);
    ui.text_mid(
        x + 28.0,
        y - 4.0,
        28.0,
        15.0,
        W::S,
        Role::OnSurface,
        app.medium.on_box(),
    );
    y += 34.0;
    // Everything comes in by default: Unselect all, while any is ticked.
    if !app.visit.inn.is_empty() {
        let t = EN.visit_unselect_all;
        let tw = ui.measure(13.0, W::S, t);
        ui.text(x, y + 2.0, 13.0, W::S, Role::Primary, t);
        ui.hit(x - 6.0, y - 6.0, tw + 12.0, 34.0, Action::VisitInAll);
        y += 34.0;
    }
    for (k, (name, size)) in stick.files.iter().enumerate() {
        // A file larger than the disk process reads is listed, not read.
        let big = *size > crate::READ_MAX;
        let kind = crate::stick_kind(name).filter(|_| !big);
        let on = app.visit.inn.contains(name);
        ui.checkbox(x + 2.0, y + 10.0, on, kind.is_some());
        let fg = if kind.is_some() {
            Role::OnSurface.paint()
        } else {
            crate::ui::off()
        };
        // Its ⋮ at the right end (decision 61 item 4).
        let menu = Faraday::stick_row_menu(name);
        let dw = if menu { 32.0 + 8.0 } else { 0.0 };
        let held = app.inbox_holds(name, *size);
        let tag = EN.visit_in_faraday;
        let tw = if held {
            ui.measure(12.0, W::S, tag) + 8.0
        } else {
            0.0
        };
        let n = ui.fit(14.0, W::M, name, w - 32.0 - tw - dw);
        ui.text(x + 32.0, y + 4.0, 14.0, W::M, fg, &n);
        if held {
            ui.text_right(
                x + w - dw,
                y + 2.0,
                20.0,
                12.0,
                W::S,
                Role::OnSurfaceVariant,
                tag,
            );
        }
        let sub = match kind {
            // Asked for: a picture's codes take a moment.
            Some(_) if app.visit.reading.contains(name) => {
                fill(EN.visit_being_read, &[&size_text(*size as usize)])
            }
            Some(k) => format!("{k} · {}", size_text(*size as usize)),
            None if big => EN.visit_file_too_large.to_string(),
            // Read at boot, not copied in.
            None => EN.visit_settings.to_string(),
        };
        let sub = ui.fit(12.0, W::R, &sub, w - 32.0 - dw);
        ui.text(
            x + 32.0,
            y + 26.0,
            12.0,
            W::R,
            if kind.is_some() {
                Role::OnSurfaceVariant.paint()
            } else {
                crate::ui::off()
            },
            &sub,
        );
        if kind.is_some() {
            ui.hit(x - 4.0, y, w + 8.0 - dw, 48.0, Action::VisitIn(k));
        }
        if menu {
            crate::file_menu::dots(
                app,
                ui,
                x + w - 32.0,
                y + 8.0,
                32.0,
                crate::file_menu::Row::Stick(k),
            );
        }
        y += 50.0;
        ui.rule(x, y, w, Role::OutlineVariant);
        y += 6.0;
    }
    if stick.files.is_empty() {
        let none = fill(EN.visit_no_files, &[app.medium.noun()]);
        ui.text(x, y, 13.0, W::R, Role::OnSurfaceVariant, &none);
        y += 28.0;
    }
    // Delete files: the one way into deleting from the stick.
    y += wrap_buttons(
        ui,
        x,
        y,
        w,
        36.0,
        &[(EN.visit_delete_files, Style::Ghost, Action::DeleteFiles)],
    );
    let nin = app.visit.inn.len();
    let enabled = nin > 0;
    let label = fill(
        EN.visit_import,
        &[
            &nin.to_string(),
            (if nin == 1 {
                EN.visit_file
            } else {
                EN.visit_files
            }),
        ],
    );
    y += 6.0;
    y += wrap_buttons(
        ui,
        x,
        y,
        w,
        42.0,
        &[
            (
                &label,
                if enabled {
                    Style::Secondary
                } else {
                    Style::Disabled
                },
                Action::VisitCopy,
            ),
            (
                EN.visit_import_and_load,
                Style::Primary.when(enabled),
                Action::VisitCopyAndLoad,
            ),
        ],
    ) + 12.0;

    // Write to the stick: the Outbox, and copies from the Inbox.
    ui.icon(x, y, 20.0, Icon::Export, 13.0, Role::Primary);
    ui.text_mid(
        x + 28.0,
        y - 4.0,
        28.0,
        15.0,
        W::S,
        Role::OnSurface,
        EN.visit_in_faraday,
    );
    y += 34.0;
    // The settings, which are not an Outbox file.
    let settings_on = app.visit_settings_on();
    let all = visit_all_out(app);
    ui.checkbox(x + 2.0, y + 4.0, all, true);
    ui.text(
        x + 32.0,
        y + 2.0,
        13.0,
        W::S,
        Role::OnSurfaceVariant,
        EN.visit_select_all,
    );
    ui.hit(x - 4.0, y - 6.0, w + 8.0, 34.0, Action::VisitOutAll);
    y += 34.0;
    let row = |ui: &mut Ui,
               y: f32,
               on: bool,
               name: &str,
               line: &str,
               tag: (&str, crate::ui::Paint),
               action: Option<Action>| {
        // No action: the stick holds the file already, drawn greyed.
        let held = action.is_none();
        ui.checkbox(x + 2.0, y + 10.0, on && !held, !held);
        let tw = ui.measure(12.0, W::S, tag.0) + 8.0;
        let n = ui.fit(14.0, W::M, name, w - 32.0 - tw);
        ui.text(
            x + 32.0,
            y + 4.0,
            14.0,
            W::M,
            if held {
                crate::ui::off()
            } else {
                Role::OnSurface.paint()
            },
            &n,
        );
        ui.text_right(x + w, y + 2.0, 20.0, 12.0, W::S, tag.1, tag.0);
        let l = ui.fit(12.0, W::R, line, w - 32.0);
        ui.text(
            x + 32.0,
            y + 26.0,
            12.0,
            W::R,
            if held {
                crate::ui::off()
            } else {
                Role::OnSurfaceVariant.paint()
            },
            &l,
        );
        if let Some(action) = action {
            ui.hit(x - 4.0, y, w + 8.0, 48.0, action);
        }
        ui.rule(x, y + 50.0, w, Role::OutlineVariant);
        56.0
    };
    y += row(
        ui,
        y,
        settings_on,
        crate::stick::settings::FILE,
        EN.visit_settings,
        (EN.visit_public, Role::OnSurfaceVariant.paint()),
        Some(Action::VisitSettings),
    );
    for (k, item) in app.outbox.iter().enumerate() {
        let on = app.visit.out.contains(&item.name);
        let held = app.in_the_backup(&item.name);
        let tag = match item.exposure() {
            _ if held => (
                crate::strings::EN.backup_flow_in_the_backup,
                Role::OnSurfaceVariant.paint(),
            ),
            crate::secrets::Exposure::Public => (EN.visit_public, Role::OnSurfaceVariant.paint()),
            crate::secrets::Exposure::Sealed => (EN.visit_sealed, Role::Ok.paint()),
            crate::secrets::Exposure::Secret => (EN.visit_unprotected_secret, Role::Error.paint()),
        };
        y += row(
            ui,
            y,
            on,
            &item.name,
            &kind_line(item.kind, item.bytes.len()),
            tag,
            (!held).then_some(Action::VisitOut(k)),
        );
    }
    if app.outbox.is_empty() {
        ui.text(
            x,
            y,
            13.0,
            W::R,
            Role::OnSurfaceVariant,
            &fill(EN.visit_nothing_waits, &[app.medium.for_the()]),
        );
        y += 28.0;
    }
    let shown = app.visit_inbox_shown();
    if !shown.is_empty() {
        ui.text_mid(
            x,
            y,
            30.0,
            13.0,
            W::S,
            Role::OnSurfaceVariant,
            app.medium.from_box(),
        );
        y += 32.0;
        for &(k, held) in &shown {
            let item = &app.inbox[k];
            y += row(
                ui,
                y,
                app.visit.from_inbox.contains(&item.name),
                &item.name,
                &visit_inbox_line(item),
                if held {
                    (app.medium.on_this_box(), Role::OnSurfaceVariant.paint())
                } else {
                    visit_inbox_tag(item)
                },
                (!held).then_some(Action::VisitInbox(k)),
            );
        }
    }
    let nout = visit_write_count(app);
    let label = fill(
        EN.visit_write,
        &[
            &nout.to_string(),
            (if nout == 1 {
                EN.visit_file
            } else {
                EN.visit_files
            }),
        ],
    );
    y += 6.0;
    ui.button(
        x,
        y,
        Some(w),
        42.0,
        &label,
        Style::Primary.when(nout > 0),
        Action::VisitWrite,
    );
    y += 54.0;
    y += about_learn(
        ui,
        x,
        y,
        w,
        &fill(
            EN.visit_write_what_waits,
            &[app.medium.noun(), app.medium.noun(), app.medium.noun()],
        ),
        Some(crate::learn::Topic::AirGap),
    );
    finish(app, ui, x0, cw, h, y - top + 16.0);
}

#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
pub(super) fn draw(app: &View, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    if ui.compact {
        return visit_compact(app, ui, x0, cw, h);
    }
    use faraday_draw::tokens::{PAGE_INSET, VISIT_GAP};
    let x = x0 + PAGE_INSET;
    let width = cw - 2.0 * PAGE_INSET;
    let mut y = 36.0;
    title(ui, x, y, app.medium.visit());
    // Where pulling the stick goes.
    let tw = ui.measure(28.0, W::S, app.medium.visit());
    ui.chip(
        x + tw + 14.0,
        y + 4.0,
        &app.visit_then(),
        Role::OnSecondaryContainer,
        Role::SecondaryContainer,
    );
    // One chip per stick; the chosen one selected (Material's filter
    // chip), below the top bar's row of pills.
    let mut chx = x + width;
    let chy = {
        use faraday_draw::tokens::{GAP_SMALL, MODE_PILL_HEIGHT, TOP_ROW_MID};
        y.max(TOP_ROW_MID + MODE_PILL_HEIGHT / 2.0 + GAP_SMALL)
    };
    for (k, st) in app.stick.iter().enumerate().rev() {
        let label = st.label.clone();
        let wdt = ui.measure(13.0, W::S, &label) + 28.0;
        chx -= wdt;
        let current = k == app.visit.stick;
        ui.fill(
            chx,
            chy,
            wdt,
            32.0,
            16.0,
            selected_or(current, Role::Surface),
        );
        let edge = selected_or(current, Role::OutlineVariant);
        ui.stroke(chx, chy, wdt, 32.0, 16.0, edge);
        let ink = on_selected(current, Role::OnSurface);
        ui.text_mid(chx + 14.0, chy, 32.0, 13.0, W::S, ink, &label);
        ui.hit(chx, chy, wdt, 32.0, Action::VisitStick(k));
        chx -= 8.0;
    }
    y += 64.0;
    y += guide_text_learn(
        app,
        ui,
        x,
        y,
        width.min(900.0),
        &fill(
            EN.visit_write_what_waits,
            &[app.medium.noun(), app.medium.noun(), app.medium.noun()],
        ),
        crate::learn::Topic::AirGap,
    );
    let Some(stick) = app.stick.get(app.visit.stick) else {
        ui.text(
            x,
            y,
            15.0,
            W::R,
            Role::OnSurfaceVariant,
            &fill(EN.visit_no_attached, &[app.medium.noun()]),
        );
        return;
    };
    let colw = (width - VISIT_GAP) / 2.0;
    let colh = h - y - 32.0 - 108.0;
    const ROW: f32 = 56.0;
    // Rows between the heading and the buttons at the card's foot, and
    // over the stick's buttons a row for zeroing it.
    let list_top = y + 62.0;
    let max_rows = ((colh - 140.0 - 40.0) / ROW).max(1.0) as usize;
    let track_h = max_rows as f32 * ROW;
    // Write to the stick: the Outbox, and copies from the Inbox.
    ui.card(x, y, colw, colh, Role::OutlineVariant);
    ui.icon(x + 20.0, y + 20.0, 22.0, Icon::Export, 14.0, Role::Primary);
    ui.text_mid(
        x + 50.0,
        y + 20.0,
        22.0,
        16.0,
        W::S,
        Role::OnSurface,
        EN.visit_in_faraday,
    );
    let settings_on = app.visit_settings_on();
    let shown = app.visit_inbox_shown();
    visit_select_all(ui, x + colw, y, visit_all_out(app), Action::VisitOutAll);
    // The settings, which are not an Outbox file, then the Outbox, then
    // the Inbox under its heading.
    const HEAD: f32 = 36.0;
    let all_h = (1 + app.outbox.len()) as f32 * ROW
        + if app.outbox.is_empty() { HEAD } else { 0.0 }
        + if shown.is_empty() {
            0.0
        } else {
            HEAD + shown.len() as f32 * ROW
        };
    let max_shift = (all_h - track_h).max(0.0);
    let shift = app.visit.out_offset.min(max_shift);
    let clip = ui.rect(x, list_top, colw, track_h);
    ui.c.push_clip(clip);
    let mut ry = list_top - shift;
    ui.checkbox(x + 22.0, ry + 15.0, settings_on, true);
    ui.text(
        x + 54.0,
        ry + 6.0,
        14.0,
        W::M,
        Role::OnSurface,
        crate::stick::settings::FILE,
    );
    ui.text(
        x + 54.0,
        ry + 28.0,
        12.0,
        W::R,
        Role::OnSurfaceVariant,
        EN.visit_settings,
    );
    ui.text_right(
        x + colw - 20.0,
        ry + 6.0,
        20.0,
        12.0,
        W::S,
        Role::OnSurfaceVariant,
        EN.visit_public,
    );
    ui.hit(x + 12.0, ry, colw - 24.0, 50.0, Action::VisitSettings);
    ui.rule(x + 20.0, ry + 52.0, colw - 40.0, Role::OutlineVariant);
    ry += ROW;
    let file_row = |ui: &mut Ui,
                    ry: f32,
                    on: bool,
                    name: &str,
                    line: &str,
                    tag: (&str, crate::ui::Paint),
                    action: Option<Action>| {
        // A row scrolled out of the list's sight is passed over.
        if !ui.shows(ry, ROW) {
            return;
        }
        // No action: the stick holds the file already, drawn greyed.
        let held = action.is_none();
        ui.checkbox(x + 22.0, ry + 15.0, on && !held, !held);
        let tw = ui.measure(12.0, W::S, tag.0) + 12.0;
        let name = ui.fit(14.0, W::M, name, colw - 74.0 - tw);
        ui.text(
            x + 54.0,
            ry + 6.0,
            14.0,
            W::M,
            if held {
                crate::ui::off()
            } else {
                Role::OnSurface.paint()
            },
            &name,
        );
        let line = ui.fit(12.0, W::R, line, colw - 90.0);
        ui.text(
            x + 54.0,
            ry + 28.0,
            12.0,
            W::R,
            if held {
                crate::ui::off()
            } else {
                Role::OnSurfaceVariant.paint()
            },
            &line,
        );
        ui.text_right(x + colw - 20.0, ry + 6.0, 20.0, 12.0, W::S, tag.1, tag.0);
        if let Some(action) = action {
            ui.hit(x + 12.0, ry, colw - 24.0, 50.0, action);
        }
        ui.rule(x + 20.0, ry + 52.0, colw - 40.0, Role::OutlineVariant);
    };
    for (k, item) in app.outbox.iter().enumerate() {
        let on = app.visit.out.contains(&item.name);
        let held = app.in_the_backup(&item.name);
        let tag = match item.exposure() {
            _ if held => (
                crate::strings::EN.backup_flow_in_the_backup,
                Role::OnSurfaceVariant.paint(),
            ),
            crate::secrets::Exposure::Public => (EN.visit_public, Role::OnSurfaceVariant.paint()),
            crate::secrets::Exposure::Sealed => (EN.visit_sealed, Role::Ok.paint()),
            crate::secrets::Exposure::Secret => (EN.visit_unprotected_secret, Role::Error.paint()),
        };
        file_row(
            ui,
            ry,
            on,
            &item.name,
            &kind_line(item.kind, item.bytes.len()),
            tag,
            (!held).then_some(Action::VisitOut(k)),
        );
        ry += ROW;
    }
    if app.outbox.is_empty() {
        ui.text_mid(
            x + 22.0,
            ry,
            HEAD,
            13.0,
            W::R,
            Role::OnSurfaceVariant,
            &fill(EN.visit_nothing_waits, &[app.medium.for_the()]),
        );
        ry += HEAD;
    }
    if !shown.is_empty() {
        ui.text_mid(
            x + 22.0,
            ry + 4.0,
            HEAD - 4.0,
            13.0,
            W::S,
            Role::OnSurfaceVariant,
            app.medium.from_box(),
        );
        ry += HEAD;
        for &(k, held) in &shown {
            let item = &app.inbox[k];
            let on = app.visit.from_inbox.contains(&item.name);
            file_row(
                ui,
                ry,
                on,
                &item.name,
                &visit_inbox_line(item),
                if held {
                    (app.medium.on_this_box(), Role::OnSurfaceVariant.paint())
                } else {
                    visit_inbox_tag(item)
                },
                (!held).then_some(Action::VisitInbox(k)),
            );
            ry += ROW;
        }
    }
    ui.c.pop_clip();
    ui.report_scroll_in(
        crate::ui::Slot::VisitOut,
        clip,
        max_shift,
        app.visit.out_offset,
    );
    visit_list_bar(
        app,
        ui,
        crate::Column::Outbox,
        (x, colw),
        list_top,
        track_h,
        all_h,
        shift,
    );
    let nout = visit_write_count(app);
    let label = fill(
        EN.visit_write,
        &[
            &nout.to_string(),
            (if nout == 1 {
                EN.visit_file
            } else {
                EN.visit_files
            }),
        ],
    );
    let bw = ui.measure(14.0, W::S, &label) + 36.0;
    ui.button(
        x + colw - 20.0 - bw,
        y + colh - 62.0,
        Some(bw),
        42.0,
        &label,
        Style::Primary.when(nout > 0),
        Action::VisitWrite,
    );

    // Import into the Inbox.
    let ix = x + colw + VISIT_GAP;
    ui.card(ix, y, colw, colh, Role::OutlineVariant);
    ui.icon(
        ix + 20.0,
        y + 20.0,
        22.0,
        Icon::Download,
        14.0,
        Role::Primary,
    );
    ui.text_mid(
        ix + 50.0,
        y + 20.0,
        22.0,
        16.0,
        W::S,
        Role::OnSurface,
        app.medium.on_box(),
    );
    let total = stick.files.len();
    let max_shift = (total as f32 * ROW - track_h).max(0.0);
    let shift = app.list_offset.min(max_shift);
    // Everything comes in by default: Unselect all, while any is ticked.
    if !app.visit.inn.is_empty() {
        visit_unselect_all(ui, ix + colw, y);
    }
    let clip = ui.rect(ix, list_top, colw, track_h);
    ui.c.push_clip(clip);
    for (k, (name, size)) in stick.files.iter().enumerate() {
        let ry = list_top - shift + k as f32 * ROW;
        // A row scrolled out of the list's sight is passed over.
        if !ui.shows(ry, ROW) {
            continue;
        }
        // A file larger than the disk process reads is listed, not read.
        let big = *size > crate::READ_MAX;
        let kind = crate::stick_kind(name).filter(|_| !big);
        let on = app.visit.inn.contains(name);
        ui.checkbox(ix + 22.0, ry + 15.0, on, kind.is_some());
        let fg = if kind.is_some() {
            Role::OnSurface
        } else {
            Role::OnSurfaceVariant
        };
        // Its ⋮ at the right end: every file but the settings file
        // (decision 61 item 4).
        let menu = Faraday::stick_row_menu(name);
        let dw = if menu { 32.0 + 8.0 } else { 0.0 };
        let room = 90.0 + dw;
        // A file Faraday holds already says so, as the left column's tags.
        let held = app.inbox_holds(name, *size);
        let tag = EN.visit_in_faraday;
        let tw = if held {
            ui.measure(12.0, W::S, tag) + 12.0
        } else {
            0.0
        };
        let n = ui.fit(
            14.0,
            W::M,
            name,
            colw - if held { 74.0 + tw + dw } else { room },
        );
        ui.text(ix + 54.0, ry + 6.0, 14.0, W::M, fg, &n);
        if held {
            ui.text_right(
                ix + colw - 20.0 - dw,
                ry + 6.0,
                20.0,
                12.0,
                W::S,
                Role::OnSurfaceVariant,
                tag,
            );
        }
        let sub = match kind {
            // Asked for: a picture's codes take a moment.
            Some(_) if app.visit.reading.contains(name) => {
                fill(EN.visit_being_read, &[&size_text(*size as usize)])
            }
            Some(k) => format!("{k} · {}", size_text(*size as usize)),
            None if big => EN.visit_file_too_large.to_string(),
            // Read at boot, not copied in.
            None => EN.visit_settings.to_string(),
        };
        let sub = ui.fit(12.0, W::R, &sub, colw - room);
        ui.text(
            ix + 54.0,
            ry + 28.0,
            12.0,
            W::R,
            Role::OnSurfaceVariant,
            &sub,
        );
        if kind.is_some() {
            ui.hit(ix + 12.0, ry, colw - 24.0 - dw, 50.0, Action::VisitIn(k));
        }
        if menu {
            crate::file_menu::dots(
                app,
                ui,
                ix + colw - 20.0 - 32.0,
                ry + 9.0,
                32.0,
                crate::file_menu::Row::Stick(k),
            );
        }
        ui.rule(ix + 20.0, ry + 52.0, colw - 40.0, Role::OutlineVariant);
    }
    ui.c.pop_clip();
    ui.report_scroll_own_bar(clip, max_shift);
    visit_list_bar(
        app,
        ui,
        crate::Column::Stick,
        (ix, colw),
        list_top,
        track_h,
        total as f32 * ROW,
        shift,
    );
    let ry = list_top + track_h;
    if stick.files.is_empty() {
        let none = fill(EN.visit_no_files, &[app.medium.noun()]);
        ui.text(ix + 22.0, ry, 13.0, W::R, Role::OnSurfaceVariant, &none);
    }
    // Delete files: the one way into deleting from the stick.
    let zy = y + colh - 62.0 - 44.0;
    ui.button(
        ix + 20.0,
        zy,
        None,
        32.0,
        EN.visit_delete_files,
        Style::Ghost,
        Action::DeleteFiles,
    );
    let nin = app.visit.inn.len();
    let enabled = nin > 0;
    let load_label = EN.visit_import_and_load;
    let load_bw = ui.measure(14.0, W::S, load_label) + 36.0;
    ui.button(
        ix + colw - 20.0 - load_bw,
        y + colh - 62.0,
        Some(load_bw),
        42.0,
        load_label,
        Style::Primary.when(enabled),
        Action::VisitCopyAndLoad,
    );
    let label = fill(
        EN.visit_import,
        &[
            &nin.to_string(),
            (if nin == 1 {
                EN.visit_file
            } else {
                EN.visit_files
            }),
        ],
    );
    let bw = ui.measure(14.0, W::S, &label) + 36.0;
    ui.button(
        ix + colw - 28.0 - load_bw - bw,
        y + colh - 62.0,
        Some(bw),
        42.0,
        &label,
        if enabled {
            Style::Secondary
        } else {
            Style::Disabled
        },
        Action::VisitCopy,
    );

    // What is left to do: pull the stick, and where the copied files went.
    let ay = h - 32.0 - 44.0 - 44.0;
    ui.fill(x, ay, width, 44.0, 10.0, Role::WarningContainer);
    ui.stroke(x, ay, width, 44.0, 10.0, Role::Warning.with_alpha(90));
    ui.icon(
        x + 16.0,
        ay + 12.0,
        20.0,
        app.medium.icon(),
        13.0,
        Role::OnWarningContainer,
    );
    ui.text_mid(
        x + 44.0,
        ay,
        44.0,
        14.0,
        W::S,
        Role::OnWarningContainer,
        &pull_line(app),
    );
    let open_label = EN.visit_open_files;
    let obw = ui.measure(14.0, W::S, open_label) + 32.0;
    ui.button(
        x + width - 12.0 - obw,
        ay + 2.0,
        Some(obw),
        40.0,
        open_label,
        Style::Secondary,
        Action::Nav(Screen::Files),
    );

    // The log.
    let fy = h - 32.0 - 44.0;
    let mut lx = x;
    for (line, ok) in app.visit.log.iter().rev().take(2) {
        let c = if *ok { Role::Ok } else { Role::Error };
        let t = ui.fit(13.0, W::R, line, width / 2.0 - 20.0);
        let used = ui.text_mid(lx, fy, 44.0, 13.0, W::R, c, &t);
        lx += used + 24.0;
    }
}

/// Whether everything Select all chooses on a visit's write list is
/// chosen: the settings, every Outbox file but an unprotected secret, and
/// every public or sealed Inbox file listed.
fn visit_all_out(app: &View) -> bool {
    app.visit_settings_on()
        && app
            .outbox
            .iter()
            .filter(|i| i.exposure() != crate::secrets::Exposure::Secret)
            .filter(|i| !app.in_the_backup(&i.name))
            .all(|i| app.visit.out.contains(&i.name))
        && app
            .visit_inbox_rows()
            .into_iter()
            .filter_map(|k| app.inbox.get(k))
            .filter(|i| i.copy_ack().is_none())
            .all(|i| app.visit.from_inbox.contains(&i.name))
}

/// How many files a visit's Write writes: the settings, the Outbox files
/// and the Inbox files chosen.
fn visit_write_count(app: &View) -> usize {
    let out = app
        .outbox
        .iter()
        .filter(|i| app.visit.out.contains(&i.name))
        .count();
    let inbox = app
        .visit_inbox_rows()
        .into_iter()
        .filter_map(|k| app.inbox.get(k))
        .filter(|i| app.visit.from_inbox.contains(&i.name))
        .count();
    out + inbox + usize::from(app.visit_settings_on())
}

/// An Inbox file's second line on a visit's write list: a picture says
/// what its codes hold.
fn visit_inbox_line(item: &crate::Item) -> String {
    match item.picture {
        Some(FileKind::Other) => fill(
            EN.visit_inbox_line_png_no_qr,
            &[&size_text(item.bytes.len())],
        ),
        Some(k) => fill(
            EN.visit_inbox_line_png,
            &[&kind_line(k, item.bytes.len()).to_string()],
        ),
        None => kind_line(item.kind, item.bytes.len()),
    }
}

/// An Inbox file's tag on a visit's write list: public and sealed files
/// are written as they are; the rest only past the secret sheet.
fn visit_inbox_tag(item: &crate::Item) -> (&'static str, crate::ui::Paint) {
    match item.copy_ack() {
        None if item.exposure() == crate::secrets::Exposure::Sealed => {
            (EN.visit_inbox_tag_sealed, Role::Ok.paint())
        }
        None => (EN.visit_inbox_tag_public, Role::OnSurfaceVariant.paint()),
        Some(crate::secrets::Ack::Unknown) => (EN.visit_inbox_tag_may, Role::Warning.paint()),
        Some(_) => (EN.visit_inbox_tag_secret, Role::Error.paint()),
    }
}

/// A stick visit column's "Select all", at the right of its heading: the
/// card's right edge at `right`, its top at `y`.
fn visit_select_all(ui: &mut Ui, right: f32, y: f32, all: bool, action: Action) {
    let label = EN.visit_select_all_select;
    let lw = ui.measure(13.0, W::S, label);
    let sx = right - 24.0 - lw - 28.0;
    ui.checkbox(sx, y + 22.0, all, true);
    ui.text_mid(
        sx + 28.0,
        y + 16.0,
        30.0,
        13.0,
        W::S,
        Role::OnSurfaceVariant,
        label,
    );
    ui.hit(sx - 8.0, y + 12.0, lw + 44.0, 38.0, action);
}

/// The stick visit's "Unselect all", at the right of the From the stick
/// heading: the card's right edge at `right`, its top at `y`.
fn visit_unselect_all(ui: &mut Ui, right: f32, y: f32) {
    let label = EN.visit_unselect_all_unselect;
    let lw = ui.measure(13.0, W::S, label);
    let sx = right - 24.0 - lw;
    ui.text_mid(sx, y + 16.0, 30.0, 13.0, W::S, Role::Primary, label);
    ui.hit(sx - 8.0, y + 12.0, lw + 16.0, 38.0, Action::VisitInAll);
}

/// A stick visit column's list's scrollbar, which a finger or the mouse
/// drags, and under the list how many rows are below it: the card's left
/// edge and width in `card`, the list's rows from `top`, `track_h` of
/// them shown of `all_h`, scrolled by `shift`.
#[allow(clippy::too_many_arguments)]
fn visit_list_bar(
    app: &View,
    ui: &mut Ui,
    column: crate::Column,
    card: (f32, f32),
    top: f32,
    track_h: f32,
    all_h: f32,
    shift: f32,
) {
    const ROW: f32 = 56.0;
    let right = card.0 + card.1;
    let max_shift = (all_h - track_h).max(0.0);
    let bar = app.visit.bar(column);
    if max_shift > 0.0 {
        let thumb = (track_h * track_h / all_h).max(32.0);
        let tx = right - 14.0;
        let ty = top + (track_h - thumb) * shift / max_shift;
        ui.fill(tx, top, 6.0, track_h, 3.0, Role::SurfaceContainerHighest);
        let held = ui.is_pressed(Action::VisitBar(column));
        ui.fill(
            tx,
            ty,
            6.0,
            thumb,
            3.0,
            if held {
                Role::Primary
            } else {
                Role::OutlineVariant
            },
        );
        ui.hit(tx - 8.0, top, 22.0, track_h, Action::VisitBar(column));
        let r = ui.rect(tx, top, 6.0, track_h);
        bar.set(Some((r.y, r.h, ui.px(thumb), max_shift)));
    } else {
        bar.set(None);
    }
    let below = all_h - shift - track_h;
    if below > 0.0 {
        ui.text(
            card.0 + 22.0,
            top + track_h,
            12.0,
            W::R,
            Role::OnSurfaceVariant,
            &fill(
                EN.visit_list_bar_more,
                &[&((below / ROW).ceil() as usize).to_string()],
            ),
        );
    }
}
