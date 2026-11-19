//! The stick pane of `Screen::Files` (F235): the stick's files beside Files'
//! own lists from expanded windows up, and below that Material's primary
//! tabs In Faraday and On the stick.

use super::parts::{kind_line, pull_line, size_text, written_and_checked};
use crate::drawing::View;
use crate::strings::EN;
use crate::ui::Region;
use crate::ui::Shape;
use crate::ui::pal::*;
use crate::ui::{Style, Ui};
use crate::wallet::FileKind;
use crate::{Action, Faraday};
use faraday_draw::tokens::{
    BODY_M, BODY_S, BUTTON_H, BUTTON_PAD, GAP_SMALL, GRID, LABEL_L, LABEL_M, ROW, TITLE_M, TITLE_S,
    VISIT_CARD_FOOT, VISIT_CARD_HEAD, VISIT_FOOT_ROW, VISIT_GROUP_HEAD, VISIT_NAME_MIN,
    VISIT_ROW_BUTTON_H,
};
use faraday_draw::widgets::Icon;
use faraday_text::Text;
use faraday_ui::parts::wrap_buttons;

/// [`visit`] on a small panel: one scrolled column. The sticks, what is
/// left to do, then what to write and what to import, each a list with
/// its buttons under it.
fn visit_compact(app: &View, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    use crate::compact::M;
    use crate::compact::screen::finish;
    let (x, w) = (x0 + M, cw - 2.0 * M);
    ui.page_begin((x0, cw, h), app.list_offset);
    let top = 12.0;
    let mut y = top;
    app.visit.bar(crate::Column::Outbox).set(None);
    app.visit.bar(crate::Column::Stick).set(None);
    let Some(stick) = app.stick.get(app.visit.stick) else {
        ui.text_as(
            x,
            y,
            BODY_M,
            Role::OnSurfaceVariant,
            &Text::fill(EN.visit_no_attached, &[app.medium.noun()]),
        );
        return;
    };
    // Material's primary tabs: In Faraday, On the stick.
    let on_stick = app.visit.tab_stick;
    let on_label = Text::fill(EN.backups_on_stick, &[&stick.label]);
    y += ui.tabs(
        (x, y, w, faraday_draw::tokens::TAB_H),
        faraday_ui::tabs::TabKind::Primary,
        &[
            faraday_ui::tabs::Tab {
                label: EN.visit_in_faraday,
                on: !on_stick,
                done: false,
                action: Action::VisitTab(crate::Column::Outbox),
            },
            faraday_ui::tabs::Tab {
                label: &on_label,
                on: on_stick,
                done: false,
                action: Action::VisitTab(crate::Column::Stick),
            },
        ],
    ) + 12.0;
    // Where pulling the stick goes.
    let then = ui.fit_as(BODY_S, &app.visit_then(), w - 34.0);
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
                let style = Style::chosen(k == app.visit.stick);
                (st.label.as_str(), style, Action::VisitStick(k))
            })
            .collect();
        y += wrap_buttons(ui, x, y, w, BUTTON_H, &items) + GAP_SMALL;
    } else {
        ui.text_as(x, y, LABEL_L, Role::OnSurfaceVariant, &stick.label);
        y += ui.advance_as(LABEL_L, 26.0);
    }
    if let Some(line) = super::files::boot_medium_line(app) {
        y += ui.wrap_as((x, y, w), BODY_M, Role::Error, line) + GRID;
    }
    // What is left to do: pull the stick, and where the copied files went.
    y += crate::vault::screen::stick_banner(ui, x, y, w, &pull_line(app));
    if signed_end(app) {
        y += super::spend::end_card(app, ui, (x, y, w), 0) + GAP_SMALL;
    }
    if let Some((line, _)) = app.visit.unprotected_of(&stick.id) {
        y += ui.wrap_as((x, y, w), BODY_M, Role::Error, line) + GAP_SMALL;
        y += wrap_buttons(
            ui,
            x,
            y,
            w,
            BUTTON_H,
            &[(
                EN.unprotected_delete,
                Style::Secondary,
                Action::DeleteUnprotected,
            )],
        ) + GAP_SMALL;
    }
    for (line, ok) in app.visit.log.iter().rev().take(2) {
        let c = if *ok { Role::Ok } else { Role::Error };
        y += ui.wrap_as((x, y, w), BODY_M, c, line) + 6.0;
    }
    y += GRID;
    if !on_stick {
        return visit_compact_in(app, ui, (x, y, w), (top, h));
    }

    // Import into the Inbox.
    ui.icon(x, y, 20.0, Icon::Download, 13.0, Role::Primary);
    ui.text_mid_as(
        (x + 28.0, y - GAP_SMALL, 28.0),
        TITLE_M,
        Role::OnSurface,
        app.medium.on_box(),
    );
    y += 34.0;
    // Everything comes in by default: Unselect all, while any is ticked.
    if !app.visit.inn.is_empty() {
        let t = EN.visit_unselect_all;
        let tw = ui.measure_as(LABEL_L, t);
        ui.text_as(x, y + 2.0, LABEL_L, Role::Primary, t);
        ui.press_box(x - 6.0, y - 6.0, tw + 12.0, 34.0, Action::VisitInAll);
        y += 34.0;
    }
    for (k, (name, size)) in stick.files.iter().enumerate() {
        // A file larger than the disk process reads is listed, not read.
        let why = stick.unread_why(&app.vaults, name, *size);
        let big = why.is_some();
        let kind = stick.kind_of(name).filter(|_| !big);
        let on = app.visit.inn.contains(name.as_str());
        ui.checkbox(x + 2.0, y + 10.0, on, kind.is_some());
        let fg = if kind.is_some() {
            Role::OnSurface.paint()
        } else {
            crate::ui::off()
        };
        // Its ⋮ at the right end (decision 61 item 4).
        let menu = Faraday::stick_row_menu(name);
        let dw = if menu { 32.0 + GRID } else { 0.0 };
        let held = app.inbox_holds(name, *size);
        let tag = EN.visit_in_faraday;
        let tw = if held {
            ui.measure_as(LABEL_M, tag) + GRID
        } else {
            0.0
        };
        let n = ui.fit_as(BODY_M.mono(), name, w - 32.0 - tw - dw);
        ui.text_as(x + 32.0, y + GAP_SMALL, BODY_M.mono(), fg, &n);
        if held {
            ui.text_right_as(
                (x + w - dw, y + 2.0, 20.0),
                LABEL_M,
                Role::OnSurfaceVariant,
                tag,
            );
        }
        let sub = match kind {
            // Asked for: a picture's codes take a moment.
            Some(_) if app.visit.reading.contains(name.as_str()) => {
                Text::fill(EN.visit_being_read, &[&size_text(*size)])
            }
            Some(k) => kind_and_size(k, *size),
            None if big => why.map(|w| Text::of(&w)).unwrap_or_default(),
            // Read at boot, not copied in.
            None => Text::of(EN.visit_settings),
        };
        let sub = ui.fit_as(BODY_S, &sub, w - 32.0 - dw);
        ui.text_as(
            x + 32.0,
            y + 26.0,
            BODY_S,
            if kind.is_some() {
                Role::OnSurfaceVariant.paint()
            } else {
                crate::ui::off()
            },
            &sub,
        );
        if kind.is_some() {
            ui.press_box(x - GAP_SMALL, y, w + GRID - dw, 48.0, Action::VisitIn(k));
        }
        if menu {
            crate::file_menu::dots(
                app,
                ui,
                x + w - 32.0,
                y + GRID,
                32.0,
                crate::file_menu::Row::Stick(k),
            );
        }
        y += 50.0;
        ui.rule(x, y, w, Role::OutlineVariant);
        y += 6.0;
    }
    if stick.files.is_empty() {
        let none = Text::fill(EN.visit_no_files, &[app.medium.noun()]);
        ui.text_as(x, y, BODY_S, Role::OnSurfaceVariant, &none);
        y += ui.advance_as(BODY_S, 28.0);
    }
    // Delete files: the one way into deleting from the stick; and the
    // stick's own name (`docs/STICK-IDENTITY.md` §6).
    let rename = Text::fill(EN.stick_name_button, &[app.medium.noun()]);
    y += wrap_buttons(
        ui,
        x,
        y,
        w,
        BUTTON_H,
        &[
            (EN.visit_delete_files, Style::Ghost, Action::DeleteFiles),
            (&rename, Style::Ghost, NAME_STICK),
        ],
    );
    let nin = app.visit.inn.len();
    let enabled = nin > 0;
    let label = Text::fill(
        EN.visit_import,
        &[
            &Text::num(nin as u64),
            (if nin == 1 {
                EN.visit_file
            } else {
                EN.visit_files
            }),
        ],
    );
    y += 6.0;
    let sign = app.visit.sign_file().cloned();
    let sign_label = sign
        .as_ref()
        .map(|n| Text::fill(EN.visit_import_and_sign, &[n]));
    let (primary_label, primary_action) = match &sign_label {
        Some(l) => (l.as_str(), Action::VisitCopyAndSign),
        None => (EN.visit_import_and_load, Action::VisitCopyAndLoad),
    };
    y += wrap_buttons(
        ui,
        x,
        y,
        w,
        BUTTON_H,
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
            (primary_label, Style::Primary.when(enabled), primary_action),
        ],
    ) + 12.0;
    finish(app, ui, h, y - top + 16.0);
}

/// [`visit_compact`]'s In Faraday tab: what is written to the stick, the
/// Outbox and copies from the Inbox, from (x, y) `w` wide.
fn visit_compact_in(app: &View, ui: &mut Ui, (x, y, w): (f32, f32, f32), (top, h): (f32, f32)) {
    use crate::compact::screen::{about, finish};
    let mut y = y;
    if let Some(r) = app.visit.receipt().as_ref() {
        y += super::files::receipt_rows(ui, r, app.tz_min(), x, y, w) + GRID;
    }
    // Write to the stick: the Outbox, and copies from the Inbox.
    ui.icon(x, y, 20.0, Icon::Export, 13.0, Role::Primary);
    ui.text_mid_as(
        (x + 28.0, y - GAP_SMALL, 28.0),
        TITLE_M,
        Role::OnSurface,
        EN.visit_in_faraday,
    );
    y += 34.0;
    // The settings, which are not an Outbox file.
    let settings_on = app.visit_settings_on();
    let all = visit_all_out(app);
    ui.checkbox(x + 2.0, y + GAP_SMALL, all, true);
    ui.text_as(
        x + 32.0,
        y + 2.0,
        LABEL_L,
        Role::OnSurfaceVariant,
        EN.visit_select_all,
    );
    ui.press_box(x - GAP_SMALL, y - 6.0, w + GRID, 34.0, Action::VisitOutAll);
    y += 34.0;
    let row = |ui: &mut Ui,
               y: f32,
               on: bool,
               name: &str,
               line: &str,
               tag: (&str, crate::ui::Paint),
               (action, menu, main): RowParts| {
        // No action: the stick holds the file already, drawn greyed.
        let held = action.is_none();
        ui.checkbox(x + 2.0, y + 10.0, on && !held, !held);
        // Its ⋮ at the right end (F54), its main action before it where
        // the name keeps its room (the Pi's column drops it).
        let dw = if menu.is_some() { 40.0 } else { 0.0 };
        let main = main.filter(|(l, ..)| {
            let bw = ui.measure_as(TITLE_S, l) + 2.0 * BUTTON_PAD;
            w - 32.0 - dw - bw - ui.measure_as(LABEL_M, tag.0) - 16.0 >= VISIT_NAME_MIN
        });
        let bw = main.as_ref().map_or(0.0, |(l, ..)| {
            ui.measure_as(TITLE_S, l) + 2.0 * BUTTON_PAD + GRID
        });
        if let Some((l, st, a)) = main {
            ui.button(
                x + w - dw - bw + GRID,
                y + GRID,
                Some(bw - GRID),
                VISIT_ROW_BUTTON_H,
                &l,
                st,
                a,
            );
        }
        let dw = dw + bw;
        let tw = ui.measure_as(LABEL_M, tag.0) + GRID + dw;
        let n = ui.fit_as(BODY_M.mono(), name, w - 32.0 - tw);
        ui.text_as(
            x + 32.0,
            y + GAP_SMALL,
            BODY_M.mono(),
            if held {
                crate::ui::off()
            } else {
                Role::OnSurface.paint()
            },
            &n,
        );
        ui.text_right_as((x + w - dw, y + 2.0, 20.0), LABEL_M, tag.1, tag.0);
        let l = ui.fit_as(BODY_S, line, w - 32.0 - dw);
        ui.text_as(
            x + 32.0,
            y + 26.0,
            BODY_S,
            if held {
                crate::ui::off()
            } else {
                Role::OnSurfaceVariant.paint()
            },
            &l,
        );
        if let Some(action) = action {
            ui.press_box(x - GAP_SMALL, y, w + GRID - dw, 48.0, action);
        }
        if let Some(menu) = menu {
            crate::file_menu::dots(app, ui, x + w - 32.0, y + GRID, 32.0, menu);
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
        (Some(Action::VisitSettings), None, None),
    );
    for (k, item) in app.outbox.iter().enumerate() {
        let on = app.visit.out.contains(&*item.name);
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
            (
                (!held).then_some(Action::VisitOut(k)),
                Some(crate::file_menu::Row::Out(k)),
                None,
            ),
        );
    }
    if app.outbox.is_empty() {
        ui.text_as(
            x,
            y,
            BODY_S,
            Role::OnSurfaceVariant,
            &Text::fill(EN.visit_nothing_waits, &[app.medium.for_the()]),
        );
        y += ui.advance_as(BODY_S, 28.0);
    }
    let shown = app.visit_inbox_shown();
    if !shown.is_empty() {
        ui.text_mid_as(
            (x, y, 30.0),
            LABEL_L,
            Role::OnSurfaceVariant,
            app.medium.from_box(),
        );
        y += 32.0;
        for &(k, held) in &shown {
            let item = &app.inbox[k];
            y += row(
                ui,
                y,
                app.visit.from_inbox.contains(&*item.name),
                &item.name,
                &visit_inbox_line(item),
                if held {
                    (app.medium.on_this_box(), Role::OnSurfaceVariant.paint())
                } else {
                    visit_inbox_tag(item)
                },
                (
                    (!held).then_some(Action::VisitInbox(k)),
                    Some(crate::file_menu::Row::In(k)),
                    main_action(app, k, item),
                ),
            );
        }
    }
    let nout = visit_write_count(app);
    let label = Text::fill(
        EN.visit_write,
        &[
            &Text::num(nout as u64),
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
        BUTTON_H,
        &label,
        Style::Primary.when(nout > 0),
        Action::VisitWrite,
    );
    y += 54.0;
    y += about(
        ui,
        x,
        y,
        w,
        &Text::fill(
            EN.visit_write_what_waits,
            &[app.medium.noun(), app.medium.noun(), app.medium.noun()],
        ),
    );
    finish(app, ui, h, y - top + 16.0);
}

/// Files from expanded windows up: the same two cards whether a stick is
/// in or not, In Faraday and On the stick, from under the top app bar to
/// the page's foot (F290).
pub(super) fn draw(app: &View, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    if ui.compact {
        return visit_compact(app, ui, x0, cw, h);
    }
    use faraday_draw::tokens::{
        FAB_H, PAGE_INSET, READING_VISIT_W, VISIT_GAP, VISIT_NOTICE_BUTTON_PAD, VISIT_NOTICE_GAP,
        VISIT_NOTICE_H,
    };
    // The two columns keep the width they read at, centred in Files' room.
    let cw_all = cw;
    let cw = cw.min(READING_VISIT_W);
    let x0 = x0 + (cw_all - cw) / 2.0;
    let x = x0 + PAGE_INSET;
    let width = cw - 2.0 * PAGE_INSET;
    // The top app bar names the page and carries the stick's status chip;
    // the cards begin under it.
    let y = super::bar_head(ui, None, EN.files_files) + GAP_SMALL;
    // A stick in with nothing held: its files can be written to and read.
    let usable = app.stick_pane();
    let stick = app.stick.get(app.visit.stick).filter(|_| usable);
    // A stick holding a secret file is named under the cards, in the
    // error tone, with its button.
    let notice = stick
        .and_then(|s| app.visit.unprotected_of(&s.id))
        .map(|(line, _)| line.clone());
    let notice_h = if notice.is_some() {
        VISIT_NOTICE_H
    } else {
        0.0
    };
    let end_h = if usable { end_room(app) } else { 0.0 };
    let log_h = if app.visit.log.is_empty() { 0.0 } else { 44.0 };
    let colw = (width - VISIT_GAP) / 2.0;
    // The cards end above the Scan button, which stands at the page's foot.
    let colh = (h - y - FAB_H - PAGE_INSET - notice_h - end_h - log_h).max(0.0);
    in_faraday(app, ui, (x, y, colw, colh), usable);
    on_the_stick(app, ui, (x + colw + VISIT_GAP, y, colw, colh), stick);
    let foot = y + colh;
    if let Some(line) = notice {
        let label = EN.unprotected_delete;
        let bw = ui.measure_as(TITLE_S, label) + VISIT_NOTICE_BUTTON_PAD;
        let ny = foot + GAP_SMALL;
        ui.wrap_as(
            (x, ny, width - bw - VISIT_NOTICE_GAP),
            BODY_M,
            Role::Error,
            &line,
        );
        ui.button(
            x + width - bw,
            ny,
            Some(bw),
            BUTTON_H,
            label,
            Style::Secondary,
            Action::DeleteUnprotected,
        );
    }
    // Where the copied files went, and the log.
    if usable {
        end_row(app, ui, (x, foot + notice_h, width));
    }
    let fy = foot + notice_h + end_h;
    let mut lx = x;
    for (line, ok) in app.visit.log.iter().rev().take(2) {
        let c = if *ok { Role::Ok } else { Role::Error };
        let t = ui.fit_as(BODY_M, line, width / 2.0 - 20.0);
        let used = ui.text_mid_as((lx, fy, 44.0), BODY_M, c, &t);
        lx += used + 24.0;
    }
}

/// One row of a card's list: the file, how it is tagged, and what it does.
struct ListRow<'a> {
    name: &'a str,
    line: &'a str,
    tag: (&'a str, crate::ui::Paint),
    /// A tick box, ticked or not; none while no stick is in.
    ticked: Option<bool>,
    /// The stick holds the file already: drawn greyed.
    greyed: bool,
    /// What pressing the row does.
    action: Option<Action>,
    /// Its ⋮ menu.
    menu: Option<crate::file_menu::Row>,
    /// Its main action, before the ⋮.
    main: Option<(Text, Style, Action)>,
}

/// One [`ListRow`] in a card `colw` wide at `x`, its top at `ry`.
fn list_row(app: &View, ui: &mut Ui, (x, colw, ry): (f32, f32, f32), r: ListRow) {
    // A row scrolled out of the list's sight is passed over.
    if !ui.shows(ry, ROW) {
        return;
    }
    let lead = if r.ticked.is_some() { 54.0 } else { 22.0 };
    if let Some(on) = r.ticked {
        ui.checkbox(x + 22.0, ry + 15.0, on && !r.greyed, !r.greyed);
    }
    // Its ⋮ at the right end (F54), its main action before it.
    // A row with no ⋮ keeps its place, so its button stays off the scrollbar.
    // The ⋮ ends 20 from the card's edge, clear of the scrollbar.
    let mut dw = 48.0;
    if let Some((l, st, a)) = r.main {
        let bw = ui.measure_as(TITLE_S, &l) + 2.0 * BUTTON_PAD;
        ui.button(
            x + colw - 12.0 - dw - bw,
            ry + GRID,
            Some(bw),
            VISIT_ROW_BUTTON_H,
            &l,
            st,
            a,
        );
        dw += bw + GRID;
    }
    let tw = ui.measure_as(LABEL_M, r.tag.0) + 12.0 + dw;
    let name = ui.fit_as(BODY_M.mono(), r.name, colw - lead - 20.0 - tw);
    let (ink, sub) = if r.greyed {
        (crate::ui::off(), crate::ui::off())
    } else {
        (Role::OnSurface.paint(), Role::OnSurfaceVariant.paint())
    };
    ui.text_as(x + lead, ry + 6.0, BODY_M.mono(), ink, &name);
    let line = ui.fit_as(BODY_S, r.line, colw - lead - 36.0 - dw);
    ui.text_as(x + lead, ry + 28.0, BODY_S, sub, &line);
    ui.text_right_as(
        (x + colw - 20.0 - dw, ry + 6.0, 20.0),
        LABEL_M,
        r.tag.1,
        r.tag.0,
    );
    if let Some(action) = r.action {
        ui.press_box(x + 12.0, ry, colw - 24.0 - dw, 50.0, action);
    }
    if let Some(menu) = r.menu {
        crate::file_menu::dots(app, ui, x + colw - 20.0 - 32.0, ry + GRID, 32.0, menu);
    }
    ui.rule(x + 20.0, ry + 52.0, colw - 40.0, Role::OutlineVariant);
}

/// The Outbox's groups by who may read them, in the order In Faraday lists
/// them: public files, vaults sealed under a passphrase, and any secret
/// the person let out unprotected.
fn out_groups() -> [(crate::secrets::Exposure, &'static str, Role); 3] {
    use crate::secrets::Exposure;
    [
        (Exposure::Public, EN.files_public, Role::OnSurfaceVariant),
        (Exposure::Sealed, EN.files_written_sealed, Role::Ok),
        (Exposure::Secret, EN.files_unprotected_secrets, Role::Error),
    ]
}

/// The left card, In Faraday, `(x, y, colw, colh)`: everything Faraday
/// holds, and at its foot Write while a stick is in, else the line that
/// files stay in memory.
fn in_faraday(app: &View, ui: &mut Ui, (x, y, colw, colh): (f32, f32, f32, f32), usable: bool) {
    ui.card(x, y, colw, colh, Role::OutlineVariant);
    ui.icon(x + 20.0, y + 20.0, 22.0, Icon::Export, 14.0, Role::Primary);
    ui.text_mid_as(
        (x + 50.0, y + 20.0, 22.0),
        TITLE_M,
        Role::OnSurface,
        EN.visit_in_faraday,
    );
    if usable {
        visit_select_all(ui, x + colw, y, visit_all_out(app), Action::VisitOutAll);
    }
    let list_top = y + VISIT_CARD_HEAD;
    let track_h = (colh - VISIT_CARD_HEAD - VISIT_CARD_FOOT).max(ROW);
    let shown = app.visit_inbox_shown();
    let receipt_h = app
        .visit
        .receipt()
        .as_ref()
        .map_or(0.0, |r| super::files::receipt_height(ui, r));
    let summary_h = super::files::inbox_panel_h(app, ui, colw - 24.0);
    let summary_room = if summary_h > 0.0 {
        summary_h + 12.0
    } else {
        0.0
    };
    let groups = out_groups()
        .iter()
        .filter(|(e, ..)| app.outbox.iter().any(|i| i.exposure() == *e))
        .count();
    let all_h = summary_room
        + app.vaults.open.len() as f32 * ROW
        + ROW
        + groups as f32 * VISIT_GROUP_HEAD
        + app.outbox.len() as f32 * ROW
        + if app.outbox.is_empty() {
            VISIT_GROUP_HEAD
        } else {
            0.0
        }
        + VISIT_GROUP_HEAD
        + shown.len().max(1) as f32 * ROW
        + receipt_h;
    let max_shift = (all_h - track_h).max(0.0);
    let shift = app.visit.out_offset.min(max_shift);
    let clip = ui.rect(x, list_top, colw, track_h);
    let open = ui.region_open(
        Region::beside(crate::ui::Slot::VisitOut, clip, app.visit.out_offset)
            .on(Role::SurfaceContainerLow)
            .own_bar()
            .from_zero(shift),
    );
    let mut ry = list_top;
    // What the Inbox gives, put together, then the open vaults.
    if summary_h > 0.0 {
        super::files::inbox_panel(app, ui, x + 12.0, ry, colw - 24.0);
        ry += summary_room;
    }
    for (v, vault) in app.vaults.open.iter().enumerate() {
        let line = vault.label(app.vaults.store());
        list_row(
            app,
            ui,
            (x, colw, ry),
            ListRow {
                name: &vault.name,
                line: &line,
                tag: (EN.files_open, Role::Ok.paint()),
                ticked: None,
                greyed: false,
                action: None,
                menu: None,
                main: Some((
                    Text::of(EN.files_contents),
                    Style::Secondary,
                    Action::Vault(crate::vault::VaultAction::Show(v)),
                )),
            },
        );
        ry += ROW;
    }
    // The settings, which are not an Outbox file.
    if ui.shows(ry, ROW) {
        let lead = if usable { 54.0 } else { 22.0 };
        if usable {
            ui.checkbox(x + 22.0, ry + 15.0, app.visit_settings_on(), true);
            ui.press_box(x + 12.0, ry, colw - 24.0, 50.0, Action::VisitSettings);
        }
        ui.text_as(
            x + lead,
            ry + 6.0,
            BODY_M.mono(),
            Role::OnSurface,
            crate::stick::settings::FILE,
        );
        ui.text_as(
            x + lead,
            ry + 28.0,
            BODY_S,
            Role::OnSurfaceVariant,
            EN.visit_settings,
        );
        ui.text_right_as(
            (x + colw - 20.0, ry + 6.0, 20.0),
            LABEL_M,
            Role::OnSurfaceVariant,
            EN.visit_public,
        );
        ui.rule(x + 20.0, ry + 52.0, colw - 40.0, Role::OutlineVariant);
    }
    ry += ROW;
    for (exposure, label, tone) in out_groups() {
        if !app.outbox.iter().any(|i| i.exposure() == exposure) {
            continue;
        }
        ui.text_mid_as(
            (x + 22.0, ry + GAP_SMALL, VISIT_GROUP_HEAD - GAP_SMALL),
            LABEL_L,
            tone,
            label,
        );
        ry += VISIT_GROUP_HEAD;
        for (k, item) in app.outbox.iter().enumerate() {
            if item.exposure() != exposure {
                continue;
            }
            let held = app.in_the_backup(&item.name);
            let tag = if held {
                (EN.backup_flow_in_the_backup, Role::OnSurfaceVariant.paint())
            } else {
                ("", Role::OnSurfaceVariant.paint())
            };
            list_row(
                app,
                ui,
                (x, colw, ry),
                ListRow {
                    name: &item.name,
                    line: &kind_line(item.kind, item.bytes.len()),
                    tag,
                    ticked: usable.then(|| app.visit.out.contains(&*item.name)),
                    greyed: held,
                    action: (usable && !held).then_some(Action::VisitOut(k)),
                    menu: Some(crate::file_menu::Row::Out(k)),
                    main: None,
                },
            );
            ry += ROW;
        }
    }
    if app.outbox.is_empty() {
        ui.text_mid_as(
            (x + 22.0, ry, VISIT_GROUP_HEAD),
            BODY_S,
            Role::OnSurfaceVariant,
            &Text::fill(EN.visit_nothing_waits, &[app.medium.for_the()]),
        );
        ry += VISIT_GROUP_HEAD;
    }
    // The Inbox under its heading.
    ui.text_mid_as(
        (x + 22.0, ry + GAP_SMALL, VISIT_GROUP_HEAD - GAP_SMALL),
        LABEL_L,
        Role::OnSurfaceVariant,
        app.medium.from_box(),
    );
    ry += VISIT_GROUP_HEAD;
    // An incomplete share's "n of m keys" says so on its row.
    let share_note = shown
        .iter()
        .any(|&(k, _)| app.inbox[k].kind == FileKind::Share)
        .then(|| share_note(app))
        .flatten();
    for &(k, held) in &shown {
        let item = &app.inbox[k];
        let mut line = visit_inbox_line(item);
        if let (FileKind::Share, Some(note)) = (item.kind, &share_note) {
            line = faraday_text::text!("{} · {}", &*line, &**note);
        }
        list_row(
            app,
            ui,
            (x, colw, ry),
            ListRow {
                name: &item.name,
                line: &line,
                tag: if held {
                    (app.medium.on_this_box(), Role::OnSurfaceVariant.paint())
                } else if usable {
                    visit_inbox_tag(item)
                } else {
                    ("", Role::OnSurfaceVariant.paint())
                },
                ticked: usable.then(|| app.visit.from_inbox.contains(&*item.name)),
                greyed: held,
                action: (usable && !held).then_some(Action::VisitInbox(k)),
                menu: Some(crate::file_menu::Row::In(k)),
                main: main_action(app, k, item),
            },
        );
        ry += ROW;
    }
    if shown.is_empty() {
        ui.text_mid_as(
            (x + 22.0, ry, ROW),
            BODY_S,
            Role::OnSurfaceVariant,
            &Text::fill(EN.files_empty_files_are, &[app.medium.a()]),
        );
        ry += ROW;
    }
    // What the last write wrote, under the lists (F235).
    if let Some(r) = app.visit.receipt().as_ref() {
        super::files::receipt_rows(ui, r, app.tz_min(), x + 22.0, ry + GAP_SMALL, colw - 44.0);
    }
    ui.region_close(open, max_shift);
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
    let fy = y + colh - VISIT_FOOT_ROW;
    if usable {
        let nout = visit_write_count(app);
        let label = Text::fill(
            EN.visit_write,
            &[
                &Text::num(nout as u64),
                (if nout == 1 {
                    EN.visit_file
                } else {
                    EN.visit_files
                }),
            ],
        );
        let bw = ui.measure_as(TITLE_S, &label) + 36.0;
        ui.button(
            x + colw - 20.0 - bw,
            fy,
            Some(bw),
            BUTTON_H,
            &label,
            Style::Primary.when(nout > 0),
            Action::VisitWrite,
        );
    } else {
        // Nothing to write to: what is here stays in memory.
        ui.icon(
            x + 20.0,
            fy + 8.0,
            20.0,
            Icon::Power,
            11.0,
            Role::OnSurfaceVariant,
        );
        ui.text_as(
            x + 46.0,
            fy + 6.0,
            BODY_S,
            Role::OnSurfaceVariant,
            EN.files_in_memory_emptied,
        );
        if !app.outbox.is_empty() {
            ui.text_as(
                x + 46.0,
                fy + 26.0,
                BODY_S,
                Role::OnSurfaceVariant,
                &Text::fill(EN.files_power_off_asks, &[app.medium.for_the()]),
            );
        }
    }
}

/// The stick's heading buttons, right-aligned in the card: one chip per
/// stick, the chosen one selected (Material's filter chip), which is also
/// how one of several sticks is picked, and Rename stick left of them
/// (`docs/STICK-IDENTITY.md` §6).
fn stick_heading(app: &View, ui: &mut Ui, (right, y): (f32, f32)) {
    let chy = y + 15.0;
    let mut chx = right;
    for (k, st) in app.stick.iter().enumerate().rev() {
        let label = st.label.clone();
        let wdt = ui.measure_as(LABEL_L, &label) + 28.0;
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
        ui.text_mid_as((chx + 14.0, chy, 32.0), LABEL_L, ink, &label);
        ui.press_box(chx, chy, wdt, 32.0, Action::VisitStick(k));
        chx -= GRID;
    }
    let rename = Text::fill(EN.stick_name_button, &[app.medium.noun()]);
    let bw = ui.measure_as(LABEL_L, &rename) + BUTTON_PAD + BUTTON_PAD;
    ui.button(
        chx - bw,
        chy - GAP_SMALL,
        Some(bw),
        BUTTON_H,
        &rename,
        Style::Ghost,
        NAME_STICK,
    );
}

/// The right card, On the stick, `(x, y, colw, colh)`: the stick's files
/// and at its foot Delete files and the imports; with no stick an empty
/// state, and with one in while anything is held, Lock.
fn on_the_stick(
    app: &View,
    ui: &mut Ui,
    (ix, y, colw, colh): (f32, f32, f32, f32),
    stick: Option<&crate::storage_api::StickInfo>,
) {
    ui.card(ix, y, colw, colh, Role::OutlineVariant);
    ui.icon(
        ix + 20.0,
        y + 20.0,
        22.0,
        Icon::Download,
        14.0,
        Role::Primary,
    );
    ui.text_mid_as(
        (ix + 50.0, y + 20.0, 22.0),
        TITLE_M,
        Role::OnSurface,
        app.medium.on_box(),
    );
    let mut top = y + VISIT_CARD_HEAD;
    if stick.is_some() {
        stick_heading(app, ui, (ix + colw - 20.0, y));
    }
    if let Some(line) = super::files::boot_medium_line(app) {
        top += ui.wrap_as((ix + 22.0, top, colw - 44.0), BODY_M, Role::Error, line) + GRID;
    }
    let Some(stick) = stick else {
        return on_no_stick(app, ui, (ix, top, colw, y + colh - top));
    };
    let track_h = (y + colh - top - VISIT_CARD_FOOT - BUTTON_H - GRID).max(ROW);
    let list_top = top;
    let total = stick.files.len();
    let max_shift = (total as f32 * ROW - track_h).max(0.0);
    let shift = app.scroll.visit_in_offset().min(max_shift);
    let clip = ui.rect(ix, list_top, colw, track_h);
    let open = ui.region_open(
        Region::beside(crate::ui::Slot::VisitIn, clip, app.scroll.visit_in_offset())
            .on(Role::SurfaceContainerLow)
            .own_bar()
            .from_zero(shift),
    );
    for (k, (name, size)) in stick.files.iter().enumerate() {
        let ry = list_top + k as f32 * ROW;
        // A row scrolled out of the list's sight is passed over.
        if !ui.shows(ry, ROW) {
            continue;
        }
        // A file larger than the disk process reads is listed, not read.
        let why = stick.unread_why(&app.vaults, name, *size);
        let big = why.is_some();
        let kind = stick.kind_of(name).filter(|_| !big);
        let on = app.visit.inn.contains(name.as_str());
        ui.checkbox(ix + 22.0, ry + 15.0, on, kind.is_some());
        let fg = if kind.is_some() {
            Role::OnSurface
        } else {
            Role::OnSurfaceVariant
        };
        // Its ⋮ at the right end: every file but the settings file
        // (decision 61 item 4).
        let menu = Faraday::stick_row_menu(name);
        let dw = if menu { 32.0 + GRID } else { 0.0 };
        let room = 90.0 + dw;
        // A file Faraday holds already says so, as the left column's tags.
        let held = app.inbox_holds(name, *size);
        let tag = EN.visit_in_faraday;
        let tw = if held {
            ui.measure_as(LABEL_M, tag) + 12.0
        } else {
            0.0
        };
        let n = ui.fit_as(
            BODY_M.mono(),
            name,
            colw - if held { 74.0 + tw + dw } else { room },
        );
        ui.text_as(ix + 54.0, ry + 6.0, BODY_M.mono(), fg, &n);
        if held {
            ui.text_right_as(
                (ix + colw - 20.0 - dw, ry + 6.0, 20.0),
                LABEL_M,
                Role::OnSurfaceVariant,
                tag,
            );
        }
        let sub = match kind {
            // Asked for: a picture's codes take a moment.
            Some(_) if app.visit.reading.contains(name.as_str()) => {
                Text::fill(EN.visit_being_read, &[&size_text(*size)])
            }
            Some(k) => kind_and_size(k, *size),
            None if big => why.map(|w| Text::of(&w)).unwrap_or_default(),
            // Read at boot, not copied in.
            None => Text::of(EN.visit_settings),
        };
        let sub = ui.fit_as(BODY_S, &sub, colw - room);
        ui.text_as(ix + 54.0, ry + 28.0, BODY_S, Role::OnSurfaceVariant, &sub);
        if kind.is_some() {
            // Ends clear of the ⋮'s touch target.
            let gap = if menu { GRID } else { 0.0 };
            ui.press_box(
                ix + 12.0,
                ry,
                colw - 24.0 - dw - gap,
                50.0,
                Action::VisitIn(k),
            );
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
    ui.region_close(open, max_shift);
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
    if stick.files.is_empty() {
        let none = Text::fill(EN.visit_no_files, &[app.medium.noun()]);
        ui.text_as(
            ix + 22.0,
            list_top + GRID,
            BODY_S,
            Role::OnSurfaceVariant,
            &none,
        );
    }
    // Delete files: the one way into deleting from the stick, a grid step
    // above the buttons at the foot so their touch targets keep apart;
    // Unselect all at the row's right end, while any file is ticked.
    let zy = y + colh - VISIT_FOOT_ROW - BUTTON_H - GRID;
    ui.button(
        ix + 20.0,
        zy,
        None,
        BUTTON_H,
        EN.visit_delete_files,
        Style::Ghost,
        Action::DeleteFiles,
    );
    if !app.visit.inn.is_empty() {
        visit_unselect_all(ui, ix + colw, zy + BUTTON_H / 2.0 - 31.0);
    }
    let nin = app.visit.inn.len();
    let enabled = nin > 0;
    let load_bw = import_primary(
        app,
        ui,
        (ix + colw, y + colh - VISIT_FOOT_ROW),
        colw,
        enabled,
    );
    let label = Text::fill(
        EN.visit_import,
        &[
            &Text::num(nin as u64),
            (if nin == 1 {
                EN.visit_file
            } else {
                EN.visit_files
            }),
        ],
    );
    let bw = ui.measure_as(TITLE_S, &label) + 36.0;
    ui.button(
        ix + colw - 28.0 - load_bw - bw,
        y + colh - VISIT_FOOT_ROW,
        Some(bw),
        BUTTON_H,
        &label,
        if enabled {
            Style::Secondary
        } else {
            Style::Disabled
        },
        Action::VisitCopy,
    );
}

/// On the stick with no stick usable, in the body `(x, top, w, h)` of its
/// card: Waiting for the stick, with nothing to press; or, a stick in
/// while anything is held, Lock to use it, with its button.
fn on_no_stick(app: &View, ui: &mut Ui, (x, top, w, h): (f32, f32, f32, f32)) {
    if app.stick.is_empty() {
        let line = super::files::stick_status(app).0;
        let tw = ui.measure_as(BODY_M, &line);
        ui.text_mid_as(
            (x + (w - tw) / 2.0, top, h),
            BODY_M,
            Role::OnSurfaceVariant,
            &line,
        );
        return;
    }
    let line = Text::fill(EN.files_lock_to_use, &[app.medium.noun()]);
    let tw = ui.measure_as(BODY_M, &line);
    let mid = top + h / 2.0;
    ui.text_mid_as(
        (x + (w - tw) / 2.0, mid - BUTTON_H, BUTTON_H),
        BODY_M,
        Role::OnSurfaceVariant,
        &line,
    );
    let bw = ui.measure_as(LABEL_L, EN.files_lock) + BUTTON_PAD + BUTTON_PAD;
    ui.button(
        x + (w - bw) / 2.0,
        mid + GRID,
        Some(bw),
        BUTTON_H,
        EN.files_lock,
        Style::Primary,
        Action::Lock,
    );
}

/// Whether everything Select all chooses on a visit's write list is
/// chosen: the settings, every Outbox file but an unprotected secret, and
/// every public or sealed Inbox file listed.
/// A pane row's press, its ⋮ menu and its main action.
type RowParts = (
    Option<Action>,
    Option<crate::file_menu::Row>,
    Option<(Text, Style, Action)>,
);
/// An Inbox file's main action in its row, as Files offers it, except
/// unlocking a vault while a stick is in: that waits until it is pulled.
fn main_action(app: &View, k: usize, item: &crate::Item) -> Option<(Text, Style, Action)> {
    let waits = !app.stick.is_empty();
    super::files::inbox_actions(app, k, item)
        .0
        .into_iter()
        .find(|(.., a)| !(waits && matches!(a, Action::Vault(crate::vault::VaultAction::Open(_)))))
        .map(|(label, style, action)| (Text::of(&label), style, action))
}

/// Whether the files written by "Lock and write" are on the stick and
/// checked: the page ends on the card Finish has.
fn signed_end(app: &View) -> bool {
    app.visit.signed_end() && written_and_checked(app)
}

/// The wide page's filled button, Import and load or, with one PSBT on the
/// stick, Import and sign it, right-aligned at `(right, y)`. Returns its
/// width.
fn import_primary(app: &View, ui: &mut Ui, (right, y): (f32, f32), colw: f32, on: bool) -> f32 {
    let sign = app.visit.sign_file().cloned();
    let sign_label = sign
        .as_ref()
        .map(|n| Text::fill(EN.visit_import_and_sign, &[n]));
    let load_label = sign_label.as_deref().unwrap_or(EN.visit_import_and_load);
    let pad = 36.0;
    let load_bw = (ui.measure_as(TITLE_S, load_label) + pad).min(colw / 2.0);
    let load_label = ui.fit_as(TITLE_S, load_label, load_bw - pad);
    ui.button(
        right - 20.0 - load_bw,
        y,
        Some(load_bw),
        BUTTON_H,
        &load_label,
        Style::Primary.when(on),
        if sign.is_some() {
            Action::VisitCopyAndSign
        } else {
            Action::VisitCopyAndLoad
        },
    );
    load_bw
}

/// The room the end card takes over the wide page's lists.
fn end_room(app: &View) -> f32 {
    if signed_end(app) {
        BUTTON_H + faraday_draw::tokens::GAP
    } else {
        0.0
    }
}

/// The end card's buttons on the wide page, once the files are written.
fn end_row(app: &View, ui: &mut Ui, at: (f32, f32, f32)) {
    if signed_end(app) {
        super::spend::end_card(app, ui, at, 0);
    }
}

/// A file's kind and its size, as a visit's row says them.
fn kind_and_size(kind: &str, size: u64) -> Text {
    let mut out = Text::of(kind);
    out.push_str(" · ");
    out.push_str(&size_text(size));
    out
}

fn visit_all_out(app: &View) -> bool {
    app.visit_settings_on()
        && app
            .outbox
            .iter()
            .filter(|i| i.exposure() != crate::secrets::Exposure::Secret)
            .filter(|i| !app.in_the_backup(&i.name))
            .all(|i| app.visit.out.contains(&*i.name))
        && app
            .visit_inbox_rows()
            .into_iter()
            .filter_map(|k| app.inbox.get(k))
            .filter(|i| i.copy_ack().is_none())
            .all(|i| app.visit.from_inbox.contains(&*i.name))
}

/// How many files a visit's Write writes: the settings, the Outbox files
/// and the Inbox files chosen.
fn visit_write_count(app: &View) -> usize {
    let out = app
        .outbox
        .iter()
        .filter(|i| app.visit.out.contains(&*i.name))
        .count();
    let inbox = app
        .visit_inbox_rows()
        .into_iter()
        .filter_map(|k| app.inbox.get(k))
        .filter(|i| app.visit.from_inbox.contains(&*i.name))
        .count();
    out + inbox + usize::from(app.visit_settings_on())
}

/// An Inbox file's second line on a visit's write list: a picture says
/// what its codes hold.
fn visit_inbox_line(item: &crate::Item) -> Text {
    match item.picture {
        Some(FileKind::Other) => Text::fill(
            EN.visit_inbox_line_png_no_qr,
            &[&size_text(item.bytes.len() as u64)],
        ),
        Some(k) => Text::fill(EN.visit_inbox_line_png, &[&kind_line(k, item.bytes.len())]),
        None => kind_line(item.kind, item.bytes.len()),
    }
}

/// What the Inbox's split-sheet shares say together while they do not make
/// the wallet: how many keys are in hand, or why they do not fit.
fn share_note(app: &View) -> Option<Text> {
    let texts: Vec<Text> = app
        .inbox
        .iter()
        .filter(|i| i.kind == FileKind::Share)
        .map(|i| Text::from_utf8_lossy(&i.bytes))
        .collect();
    match crate::restore::merge(&texts) {
        Ok(m) if m.whole.is_some() => None,
        Ok(m) => Some(Text::fill(
            EN.inbox_actions_of_keys,
            &[
                &faraday_text::text!("{}", m.have.len()),
                &faraday_text::text!("{}", m.n),
            ],
        )),
        Err(e) => Some(Text::of(&e)),
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
    let lw = ui.measure_as(LABEL_L, label);
    let sx = right - 24.0 - lw - 28.0;
    ui.checkbox(sx, y + 22.0, all, true);
    ui.text_mid_as(
        (sx + 28.0, y + 16.0, 30.0),
        LABEL_L,
        Role::OnSurfaceVariant,
        label,
    );
    ui.press_box(sx - GRID, y + 12.0, lw + 44.0, 38.0, action);
}

/// The stick visit's "Unselect all", at the right of the From the stick
/// heading: the card's right edge at `right`, its top at `y`.
fn visit_unselect_all(ui: &mut Ui, right: f32, y: f32) {
    let label = EN.visit_unselect_all_unselect;
    let lw = ui.measure_as(LABEL_L, label);
    let sx = right - 24.0 - lw;
    ui.text_mid_as((sx, y + 16.0, 30.0), LABEL_L, Role::Primary, label);
    ui.press_box(sx - GRID, y + 12.0, lw + 16.0, 38.0, Action::VisitInAll);
}

/// A stick visit column's list's scrollbar, which a finger or the mouse
/// drags, and under the list how many rows are below it: the card's left
/// edge and width in `card`, the list's rows from `top`, `track_h` of
/// them shown of `all_h`, scrolled by `shift`.
#[allow(
    clippy::too_many_arguments,
    reason = "the visit list bar's view, column, card size, top, track and shift"
)]
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
    let right = card.0 + card.1;
    let max_shift = (all_h - track_h).max(0.0);
    let bar = app.visit.bar(column);
    if max_shift > 0.0 {
        let thumb = (track_h * track_h / all_h).max(32.0);
        let tx = right - 14.0;
        let ty = top + (track_h - thumb) * shift / max_shift;
        ui.fill(tx, top, 6.0, track_h, 3.0, Role::SurfaceContainerHighest);
        // One press: its layer and ring round the thumb, over the track
        // and under the thumb; the press on the whole track.
        let (gx, gw) = (tx - GRID, 22.0);
        ui.press_to(
            Shape::new(gx, ty, gw, thumb, faraday_draw::tokens::shape_full(gw)),
            (gx, top, gw, track_h),
            Role::OnSurface.paint(),
            Action::VisitBar(column),
        );
        ui.fill(tx, ty, 6.0, thumb, 3.0, Role::OutlineVariant);
        let r = ui.rect(tx, top, 6.0, track_h);
        bar.set(Some((r.y, r.h, ui.px(thumb), max_shift)));
    } else {
        bar.set(None);
    }
    let below = all_h - shift - track_h;
    if below > 0.0 {
        ui.text_as(
            card.0 + 22.0,
            top + track_h,
            BODY_S,
            Role::OnSurfaceVariant,
            &Text::fill(
                EN.visit_list_bar_more,
                &[&Text::num((below / ROW).ceil() as u64)],
            ),
        );
    }
}

/// The visit's Rename stick: the popup for the stick's own name.
const NAME_STICK: Action = Action::File(crate::file_menu::FilePress::NameStick);
