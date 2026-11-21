//! Files (F235, F290): the two cards In Faraday and On the stick, side by
//! side from expanded windows up and, below that, Material's primary tabs.

use super::parts::{kind_line, size_text, written_and_checked};
use crate::drawing::View;
use crate::strings::EN;
use crate::ui::Moves;
use crate::ui::Region;
use crate::ui::Shape;
use crate::ui::pal::*;
use crate::ui::{Style, Ui};
use crate::wallet::FileKind;
use crate::{Action, Faraday};
use faraday_draw::tokens::{
    BODY_M, BODY_S, BUTTON_H, BUTTON_PAD, GAP_SMALL, GRID, LABEL_L, LABEL_M, ROW, SHAPE_M, TITLE_M,
    TITLE_S, TOUCH, VISIT_BUTTON_PAD, VISIT_CARD_FOOT, VISIT_CARD_HEAD, VISIT_CHIP_EXTRA,
    VISIT_CHIP_PRESS_RISE, VISIT_CHIP_TOP, VISIT_COMPACT_HEAD, VISIT_COUNT_GAP, VISIT_FOOT_ROW,
    VISIT_GROUP_HEAD, VISIT_LOG_H, VISIT_NAME_MIN, VISIT_PAD, VISIT_ROW_BUTTON_H, VISIT_TEXT_X,
    VISIT_TITLE_X,
};
use faraday_draw::widgets::Icon;
use faraday_text::Text;

/// Files: the same two cards whether a stick is in or not, In Faraday and
/// On the stick, from under the top app bar to the page's foot (F290).
/// From expanded windows up they stand side by side; below that they are
/// Material's primary tabs and one card shows at a time.
pub(super) fn draw(app: &View, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    use faraday_draw::tokens::{
        FAB_H, PAGE_INSET, READING_VISIT_W, TAB_H, VISIT_GAP, VISIT_NOTICE_BUTTON_PAD,
        VISIT_NOTICE_GAP, VISIT_NOTICE_H,
    };
    let compact = ui.compact;
    // The two columns keep the width they read at, centred in Files' room.
    let cw_all = cw;
    let cw = if compact { cw } else { cw.min(READING_VISIT_W) };
    let x0 = x0 + (cw_all - cw) / 2.0;
    let (x, width) = if compact {
        (x0 + crate::compact::M, cw - 2.0 * crate::compact::M)
    } else {
        (x0 + PAGE_INSET, cw - 2.0 * PAGE_INSET)
    };
    // The top app bar names the page and carries the stick's status chip;
    // the cards begin under it. A small panel's page head is drawn apart:
    // its cards begin under the tabs.
    let mut y = if compact {
        app.visit.bar(crate::Column::Outbox).set(None);
        app.visit.bar(crate::Column::Stick).set(None);
        12.0
    } else {
        super::bar_head(ui, None, EN.files_files) + GAP_SMALL
    };
    // A stick in with nothing held: its files can be written to and read.
    let usable = app.stick_pane();
    let stick = app.stick.get(app.visit.stick).filter(|_| usable);
    // A stick holding a secret file is named under the cards, in the
    // error tone, with its button.
    let notice = stick
        .and_then(|s| app.visit.unprotected_of(&s.id))
        .map(|(line, _)| line.clone());
    // The notice wraps beside its button: its band is as tall as its text.
    let notice_bw = ui.measure_as(TITLE_S, EN.unprotected_delete) + VISIT_NOTICE_BUTTON_PAD;
    let notice_w = width - notice_bw - VISIT_NOTICE_GAP;
    // Too narrow for the text beside the button, the button stands under it.
    let notice_under = notice_w < notice_bw;
    let notice_w = if notice_under { width } else { notice_w };
    let notice_h = notice.as_ref().map_or(0.0, |line| {
        let text_h = ui.wrap_height_as(BODY_M, line, notice_w);
        if notice_under {
            text_h + BUTTON_H + GAP_SMALL + GAP_SMALL + GAP_SMALL
        } else {
            (text_h + 2.0 * GAP_SMALL).max(VISIT_NOTICE_H)
        }
    });
    let end_h = if usable { end_room(app) } else { 0.0 };
    // The log's last two lines: side by side on a wide page, each wrapped
    // across the full width, one under the other, on a small panel.
    let log: Vec<_> = app.visit.log.iter().rev().take(2).collect();
    let log_h = if log.is_empty() {
        0.0
    } else if compact {
        log.iter()
            .map(|(line, _)| ui.wrap_height_as(BODY_M, line, width) + GAP_SMALL)
            .sum::<f32>()
            + GAP_SMALL
    } else {
        VISIT_LOG_H
    };
    if compact {
        let on_stick = app.visit.tab_stick;
        let on_label = match stick.or_else(|| app.stick.get(app.visit.stick)) {
            Some(st) => Text::fill(EN.backups_on_stick, &[&st.label]),
            None => Text::of(app.medium.on_box()),
        };
        y += ui.tabs(
            (x, y, width, TAB_H),
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
        ) + GAP_SMALL;
    }
    let colw = if compact {
        width
    } else {
        (width - VISIT_GAP) / 2.0
    };
    // Where Scan floats over the foot (`foot_room`), the cards end above
    // it; at the top of the rail or the sidebar it takes no room here.
    let fab = if ui.foot_room > 0.0 {
        FAB_H + PAGE_INSET
    } else {
        0.0
    };
    // The notice's button can stand over the Scan button's corner: a gap
    // keeps its press box whole.
    let scan_gap = if notice.is_some() {
        2.0 * GAP_SMALL
    } else {
        0.0
    };
    let mut colh = (h - y - fab - notice_h - scan_gap - end_h - log_h).max(0.0);
    // On a small panel the On the stick tab that does not fit scrolls as
    // one page: the card at the height it needs, the notice and the log
    // after it. The tabs stay under the top app bar.
    let mut page = None;
    if compact && app.visit.tab_stick && stick.is_some() {
        let needed = stick_card_needed(app, ui, (x, y, colw, colh), stick);
        if needed > colh {
            colh = needed;
            let clip = ui.rect(x0, y, cw, (h - y).max(0.0));
            page = Some(ui.region_open(Region::page(clip).from_zero(app.list_offset)));
        }
    }
    if !compact || !app.visit.tab_stick {
        in_faraday(app, ui, (x, y, colw, colh), usable);
    }
    if !compact || app.visit.tab_stick {
        let sx = if compact { x } else { x + colw + VISIT_GAP };
        on_the_stick(app, ui, (sx, y, colw, colh), stick, page.is_some());
    }
    let foot = y + colh;
    if let Some(line) = notice {
        let label = EN.unprotected_delete;
        let ny = foot + GAP_SMALL;
        let text_h = ui.wrap_as((x, ny, notice_w), BODY_M, Role::Error, &line);
        ui.button(
            x + width - notice_bw,
            if notice_under {
                ny + text_h + GAP_SMALL
            } else {
                ny
            },
            Some(notice_bw),
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
    let mut ly = fy + GAP_SMALL;
    for (line, ok) in log {
        let c = if *ok { Role::Ok } else { Role::Error };
        if compact {
            ly += ui.wrap_as((x, ly, width), BODY_M, c, line) + GAP_SMALL;
        } else {
            let t = ui.fit_as(BODY_M, line, width / 2.0 - VISIT_PAD);
            let used = ui.text_mid_as((lx, fy, VISIT_LOG_H), BODY_M, c, &t);
            lx += used + 24.0;
        }
    }
    if let Some(open) = page {
        let bottom = fy + log_h + scan_gap + fab.max(PAGE_INSET);
        ui.region_close(open, (bottom - h).max(0.0));
    }
}

/// The height the On the stick card needs on a small panel to show every
/// file, Delete files and the buttons, measured by a pass that draws
/// nothing.
fn stick_card_needed(
    app: &View,
    ui: &mut Ui,
    at: (f32, f32, f32, f32),
    stick: Option<&crate::storage_api::StickInfo>,
) -> f32 {
    let clip = ui.rect(0.0, 0.0, 0.0, 0.0);
    ui.c.push_clip(clip);
    let needed = on_the_stick(app, ui, at, stick, true);
    ui.c.pop_clip();
    needed
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
    /// The same file is on the stick: the row is green with a check.
    same: bool,
    /// What pressing the row does.
    action: Option<Action>,
    /// Its ⋮ menu.
    menu: Option<crate::file_menu::Row>,
    /// Its main action, before the ⋮.
    main: Option<(Text, Style, Action)>,
    /// The name wraps onto more lines rather than ending in "…".
    wrap: bool,
}

/// One [`ListRow`] in a card `colw` wide at `x`, its top at `ry`.
/// A row's left margin, its ⋮'s width and its main button's width with
/// the gap before it.
fn row_parts(ui: &Ui, r: &ListRow) -> (f32, f32, f32) {
    let lead = if r.ticked.is_some() { 54.0 } else { 22.0 };
    let main = r
        .main
        .as_ref()
        .map(|(l, ..)| ui.measure_as(TITLE_S, l) + 2.0 * BUTTON_PAD + GRID)
        .unwrap_or_default();
    (lead, 48.0, main)
}

/// The room of a row's name.
fn name_room(ui: &Ui, r: &ListRow, colw: f32) -> f32 {
    let (lead, menu, main) = row_parts(ui, r);
    let tw = ui.measure_as(LABEL_M, r.tag.0) + 12.0 + menu + main;
    colw - lead - 20.0 - tw - super::visit_same::width(r.same)
}

/// What a wrapped name adds to a row's height: its lines beyond the first.
fn extra_h(ui: &Ui, r: &ListRow, colw: f32) -> f32 {
    if !r.wrap {
        return f32::default();
    }
    let lines = ui.wrapped_as(BODY_M.mono(), r.name, name_room(ui, r, colw));
    let one = ui.line_as(BODY_M.mono());
    lines.h.max(one) - one
}

fn list_row(app: &View, ui: &mut Ui, (x, colw, ry): (f32, f32, f32), mut r: ListRow) -> f32 {
    // A name that does not wrap keeps `VISIT_NAME_MIN` of room, or the main
    // action is not drawn (as the stick pane's rows did before F290).
    // Pressing the row then does it: the tick box keeps the tick.
    let mut dropped = None;
    if !r.wrap && r.main.is_some() && name_room(ui, &r, colw) < VISIT_NAME_MIN {
        dropped = r.main.take().map(|(.., a)| a);
    }
    let extra = extra_h(ui, &r, colw);
    // A row scrolled out of the list's sight is passed over.
    if !ui.shows(ry, ROW + extra) {
        return ROW + extra;
    }
    let (lead, menu, main) = row_parts(ui, &r);
    let room = name_room(ui, &r, colw);
    let chk = super::visit_same::width(r.same);
    let dw = menu + main;
    if r.same {
        super::visit_same::row(ui, (x, colw, ry), dw);
        r.tag.1 = Role::OnOkContainer.paint();
    }
    if let Some(on) = r.ticked {
        ui.checkbox(x + 22.0, ry + 15.0, on && !r.greyed, !r.greyed);
    }
    // Its ⋮ at the right end (F54), its main action before it.
    // A row with no ⋮ keeps its place, so its button stays off the scrollbar.
    // The ⋮ ends 20 from the card's edge, clear of the scrollbar.
    if let Some((l, st, a)) = r.main {
        let bw = main - GRID;
        ui.button(
            x + colw - 12.0 - menu - bw,
            ry + GRID,
            Some(bw),
            VISIT_ROW_BUTTON_H,
            &l,
            st,
            a,
        );
    }
    let (ink, sub) = if r.same {
        (Role::OnOkContainer.paint(), Role::OnOkContainer.paint())
    } else if r.greyed {
        (crate::ui::off(), crate::ui::off())
    } else {
        (Role::OnSurface.paint(), Role::OnSurfaceVariant.paint())
    };
    let at = (x + lead, ry + 6.0);
    if r.wrap {
        let lines = ui.wrapped_as(BODY_M.mono(), r.name, room);
        ui.draw_wrapped(at, ink, &lines);
    } else {
        let name = ui.fit_as(BODY_M.mono(), r.name, room);
        ui.text_as(at.0, at.1, BODY_M.mono(), ink, &name);
    }
    let line = ui.fit_as(BODY_S, r.line, colw - lead - 36.0 - dw - chk);
    ui.text_as(x + lead, ry + 28.0 + extra, BODY_S, sub, &line);
    ui.text_right_as(
        (x + colw - 20.0 - dw - chk, ry + 6.0, 20.0),
        LABEL_M,
        r.tag.1,
        r.tag.0,
    );
    let (px, ph, pw) = (x + 12.0, 50.0, colw - 24.0);
    if let Some(main) = dropped {
        let mut from = px;
        if let (Some(tick), true) = (r.action, r.ticked.is_some()) {
            from = x + lead;
            ui.press_box(px, ry, from - px, ph, tick);
        }
        ui.press_box(from, ry, px + pw - dw - from, ph, main);
    } else if let Some(action) = r.action {
        ui.press_box(px, ry, pw - dw, ph, action);
    }
    if let Some(menu) = r.menu {
        crate::file_menu::dots(app, ui, x + colw - 20.0 - 32.0, ry + GRID, 32.0, menu);
    }
    ui.rule(
        x + 20.0,
        ry + 52.0 + extra,
        colw - 40.0,
        Role::OutlineVariant,
    );
    ROW + extra
}

fn open_label(app: &View, v: usize) -> faraday_text::Text {
    app.vaults.open[v].label(app.vaults.store())
}

/// An open vault's row line: its label, and while it holds changes not
/// yet saved, how many.
fn open_line(app: &View, v: usize) -> Text {
    let label = open_label(app, v);
    let n = app.vaults.open[v].changes;
    if n == 0 {
        return label;
    }
    let unsaved = Text::fill(
        EN.vault_open_unsaved,
        &[
            &Text::num(n as u64),
            (if n == 1 {
                EN.vault_change
            } else {
                EN.vault_changes
            }),
        ],
    );
    faraday_text::text!("{} · {}", &*label, &*unsaved)
}

/// An open vault's row: its name wraps rather than ending in "…".
fn vault_row<'a>(app: &'a View, v: usize, line: &'a str) -> ListRow<'a> {
    // Its ⋮ loads wallets: with none left to load, it has nothing to open.
    let loads = app
        .vault_wallets_with_keys(v)
        .iter()
        .any(|w| !w.done && w.chosen);
    ListRow {
        name: &app.vaults.open[v].name,
        line,
        tag: (EN.files_open, Role::Ok.paint()),
        ticked: None,
        greyed: false,
        same: false,
        action: None,
        menu: loads.then_some(crate::file_menu::Row::Vault(v)),
        main: Some((
            Text::of(EN.files_contents),
            Style::Secondary,
            Action::Vault(crate::vault::VaultAction::Show(v)),
        )),
        wrap: true,
    }
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

/// A group's heading at `(x, y)` with how many files it holds after it.
fn group_head(ui: &mut Ui, (x, y): (f32, f32), tone: Role, label: &str, n: usize) {
    let lw = ui.text_mid_as(
        (x, y + GAP_SMALL, VISIT_GROUP_HEAD - GAP_SMALL),
        LABEL_L,
        tone,
        label,
    );
    ui.text_mid_as(
        (
            x + lw + VISIT_COUNT_GAP,
            y + GAP_SMALL,
            VISIT_GROUP_HEAD - GAP_SMALL,
        ),
        BODY_S,
        Role::OnSurfaceVariant,
        &Text::num(n as u64),
    );
}

/// An Outbox file's line and state: what it is, or for a vault open in
/// this session (the file here is the copy sealed before it was opened,
/// and Lock seals it again) its changes; then ready, written after Lock,
/// or unprotected, in its colour.
fn out_state(app: &View, item: &crate::Item) -> (Text, (&'static str, Role)) {
    use crate::secrets::Exposure;
    let open_here = (item.kind == FileKind::Vault)
        .then(|| app.vaults.vault_id_of_file(item.sha))
        .flatten()
        .and_then(|id| app.vaults.open.iter().find(|v| v.vault_id() == id));
    let line = match open_here {
        // With no seed to seal with, Lock does not seal it.
        Some(v) if v.changes > 0 => Text::fill(
            if app.seeded() {
                EN.files_vault_open_unsaved
            } else {
                EN.vault_open_unsaved
            },
            &[
                &Text::num(v.changes as u64),
                (if v.changes == 1 {
                    EN.vault_change
                } else {
                    EN.vault_changes
                }),
            ],
        ),
        Some(_) => Text::of(EN.files_vault_open_sealed),
        None => kind_line(item.kind, item.bytes.len()),
    };
    let state = match item.exposure() {
        Exposure::Secret => (EN.files_unprotected, Role::Error),
        _ if open_here.is_some() => (EN.files_written_after, Role::Primary),
        _ => (EN.files_ready, Role::Ok),
    };
    (line, state)
}

/// The left card, In Faraday, `(x, y, colw, colh)`: everything Faraday
/// holds, and at its foot Write while a stick is in, else the line that
/// files stay in memory.
fn in_faraday(app: &View, ui: &mut Ui, (x, y, colw, colh): (f32, f32, f32, f32), usable: bool) {
    ui.card(x, y, colw, colh, Role::OutlineVariant);
    // A small panel's tab names the card, so its heading is left out and
    // only the controls that stood in it remain.
    let compact = ui.compact;
    if !compact {
        ui.icon(x + 20.0, y + 20.0, 22.0, Icon::Export, 14.0, Role::Primary);
        ui.text_mid_as(
            (x + 50.0, y + 20.0, 22.0),
            TITLE_M,
            Role::OnSurface,
            EN.visit_in_faraday,
        );
    }
    if usable {
        visit_select_all(ui, x + colw, y, visit_all_out(app), Action::VisitOutAll);
    }
    let head = match (compact, usable) {
        (false, _) => VISIT_CARD_HEAD,
        (true, true) => VISIT_COMPACT_HEAD,
        // Below the card's rounded top corners, which a moved list would drag.
        (true, false) => SHAPE_M + GAP_SMALL,
    };
    // On a small panel the stick's error line shows on both tabs.
    let head = match super::files::boot_medium_line(app).filter(|_| compact) {
        Some(line) => {
            let h = ui.wrap_as(
                (x + VISIT_TEXT_X, y + head, colw - 2.0 * VISIT_TEXT_X),
                BODY_M,
                Role::Error,
                line,
            );
            head + h + GRID
        }
        None => head,
    };
    // Without a stick the card has no foot row.
    let foot_h = if usable {
        VISIT_CARD_FOOT
    } else {
        VISIT_CARD_FOOT - VISIT_FOOT_ROW
    };
    let list_top = y + head;
    let track_h = (colh - head - foot_h).max(ROW);
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
        + (0..app.vaults.open.len())
            .map(|v| ROW + extra_h(ui, &vault_row(app, v, ""), colw))
            .sum::<f32>()
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
            .from_zero(shift)
            .moves(Moves::Yes),
    );
    let mut ry = list_top;
    // What the Inbox gives, put together, then the open vaults.
    if summary_h > 0.0 {
        super::files::inbox_panel(app, ui, x + 12.0, ry, colw - 24.0);
        ry += summary_room;
    }
    for v in 0..app.vaults.open.len() {
        let line = open_line(app, v);
        ry += list_row(app, ui, (x, colw, ry), vault_row(app, v, &line));
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
        let n = app
            .outbox
            .iter()
            .filter(|i| i.exposure() == exposure)
            .count();
        group_head(ui, (x + VISIT_TEXT_X, ry), tone, label, n);
        ry += VISIT_GROUP_HEAD;
        for (k, item) in app.outbox.iter().enumerate() {
            if item.exposure() != exposure {
                continue;
            }
            let held = app.in_the_backup(&item.name);
            let (line, state) = out_state(app, item);
            let tag = if held {
                (EN.backup_flow_in_the_backup, Role::OnSurfaceVariant.paint())
            } else {
                (state.0, state.1.paint())
            };
            list_row(
                app,
                ui,
                (x, colw, ry),
                ListRow {
                    name: &item.name,
                    line: &line,
                    tag,
                    ticked: usable.then(|| app.visit.out.contains(&*item.name)),
                    greyed: held,
                    same: usable && app.visit_same(&item.name, Some(item.bytes.len() as u64), true),
                    action: (usable && !held).then_some(Action::VisitOut(k)),
                    menu: Some(crate::file_menu::Row::Out(k)),
                    main: None,
                    wrap: false,
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
    group_head(
        ui,
        (x + VISIT_TEXT_X, ry),
        Role::OnSurfaceVariant,
        app.medium.from_box(),
        app.inbox.len(),
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
        let mut line = visit_inbox_line(app, item);
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
                same: usable && app.visit_same(&item.name, Some(item.bytes.len() as u64), false),
                action: (usable && !held).then_some(Action::VisitInbox(k)),
                menu: Some(crate::file_menu::Row::In(k)),
                main: main_action(app, k, item),
                wrap: false,
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
        super::files::receipt_rows(
            app,
            ui,
            r,
            app.tz_min(),
            x + 22.0,
            ry + GAP_SMALL,
            colw - 44.0,
        );
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
    }
}

/// The stick's heading buttons: Rename stick and one chip per stick, the
/// chosen one selected (Material's filter chip), which is also how one of
/// several sticks is picked (`docs/STICK-IDENTITY.md` §6). They stand
/// right-aligned in the card's heading while they fit beside its title
/// (`title_w`, none on a small panel, whose tab names the card), else
/// from the left in rows under it. `(ix, y, colw)` is the card's left
/// edge, top and width. Returns the height the rows add to the heading.
fn stick_heading(app: &View, ui: &mut Ui, (ix, y, colw): (f32, f32, f32), title_w: f32) -> f32 {
    let rename = Text::fill(EN.stick_name_button, &[app.medium.noun()]);
    let rename_w = ui.measure_as(LABEL_L, &rename) + BUTTON_PAD + BUTTON_PAD;
    let chips: Vec<f32> = app
        .stick
        .iter()
        .map(|st| ui.measure_as(LABEL_L, &st.label) + VISIT_CHIP_EXTRA)
        .collect();
    let total = rename_w + chips.iter().map(|w| w + GRID).sum::<f32>();
    let compact = ui.compact;
    let beside = if compact {
        0.0
    } else {
        VISIT_TITLE_X + title_w + VISIT_PAD
    };
    // The chips stand clear of the card's right edge by one pad; at the left
    // the Rename stick button's own padding is its margin.
    let one_line = total <= colw - VISIT_PAD - beside - if compact { 0.0 } else { VISIT_PAD };
    let line_h = TOUCH;
    // (x, chip top) of each item as it is laid: Rename stick first.
    let first_y = if compact || one_line {
        y + VISIT_CHIP_TOP
    } else {
        y + VISIT_CARD_HEAD
    };
    let mut cx = if one_line {
        ix + colw - VISIT_PAD - total
    } else {
        ix + VISIT_PAD
    };
    let mut cy = first_y;
    let mut place = |w: f32| {
        if !one_line && cx + w > ix + colw - VISIT_PAD && cx > ix + VISIT_PAD {
            cx = ix + VISIT_PAD;
            cy += line_h;
        }
        let at = (cx, cy);
        cx += w + GRID;
        at
    };
    let (rx, ry) = place(rename_w);
    ui.button(
        rx,
        ry - GAP_SMALL,
        Some(rename_w),
        BUTTON_H,
        &rename,
        Style::Ghost,
        NAME_STICK,
    );
    for (k, st) in app.stick.iter().enumerate() {
        let wdt = chips[k];
        let (chx, chy) = place(wdt);
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
        ui.text_mid_as((chx + 14.0, chy, 32.0), LABEL_L, ink, &st.label);
        // The press box is the touch floor tall, around the 32 px chip.
        ui.press_box(
            chx,
            chy - VISIT_CHIP_PRESS_RISE,
            wdt,
            TOUCH,
            Action::VisitStick(k),
        );
    }
    // The rows past the heading's own: a small panel's first stands in it.
    let rows = (cy - first_y) / line_h + usize::from(!(one_line || compact)) as f32;
    rows * line_h
}

/// The right card, On the stick, `(x, y, colw, colh)`: the stick's files
/// and at its foot Delete files and the imports; with no stick an empty
/// state, and with one in while anything is held, Lock.
fn on_the_stick(
    app: &View,
    ui: &mut Ui,
    (ix, y, colw, colh): (f32, f32, f32, f32),
    stick: Option<&crate::storage_api::StickInfo>,
    page: bool,
) -> f32 {
    ui.card(ix, y, colw, colh, Role::OutlineVariant);
    // A small panel's tab names the card: no heading, and room above the
    // list only for the stick chips.
    let compact = ui.compact;
    if !compact {
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
    }
    let mut top = y + match (compact, stick.is_some()) {
        (false, _) => VISIT_CARD_HEAD,
        (true, true) => VISIT_COMPACT_HEAD,
        (true, false) => GRID,
    };
    if stick.is_some() {
        let title_w = ui.measure_as(TITLE_M, app.medium.on_box());
        top += stick_heading(app, ui, (ix, y, colw), title_w);
    }
    if let Some(line) = super::files::boot_medium_line(app) {
        top += ui.wrap_as((ix + 22.0, top, colw - 44.0), BODY_M, Role::Error, line) + GRID;
    }
    let Some(stick) = stick else {
        on_no_stick(app, ui, (ix, top, colw, y + colh - top));
        return colh;
    };
    // What pulling the stick does next, one line at the card's foot.
    let pull = pull_line(app);
    let pull_h = pull.as_ref().map_or(0.0, |_| ui.line_as(BODY_S) + GRID);
    let colh = colh - pull_h;
    let nin = app.visit.inn.len();
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
    let natural_bw = ui.measure_as(TITLE_S, &label) + VISIT_BUTTON_PAD;
    // The two buttons stand one over the other in a card too narrow for
    // them side by side, each as wide as the card's inner width.
    let stacked = natural_bw + import_natural_w(app, ui) + TOUCH > colw;
    let bw = if stacked {
        colw - 2.0 * VISIT_PAD
    } else {
        natural_bw
    };
    let load_bw = import_primary_w(app, ui, colw, stacked);
    let foot_more = if stacked { BUTTON_H + GRID } else { 0.0 };
    let track_raw = y + colh - top - VISIT_CARD_FOOT - BUTTON_H - GRID - foot_more;
    let track_h = track_raw.max(ROW);
    let list_top = top;
    let total = stick.files.len();
    // The height the card needs to show every row, the Delete files row
    // and the buttons: what the small panel's page scrolls by.
    let needed = colh + pull_h + (total as f32 * ROW).max(ROW) - track_raw;
    let max_shift = if page {
        0.0
    } else {
        (total as f32 * ROW - track_h).max(0.0)
    };
    let shift = app.scroll.visit_in_offset().min(max_shift);
    let clip = ui.rect(ix, list_top, colw, track_h);
    // On the small panel's scrolling page the rows stand in the page's
    // scroll, not in a list scrolling within it.
    let open = (!page).then(|| {
        ui.region_open(
            Region::beside(crate::ui::Slot::VisitIn, clip, app.scroll.visit_in_offset())
                .on(Role::SurfaceContainerLow)
                .own_bar()
                .from_zero(shift),
        )
    });
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
        // Its ⋮ at the right end: every file but the settings file
        // (decision 61 item 4).
        let menu = Faraday::stick_row_menu(name);
        let dw = if menu { 32.0 + GRID } else { 0.0 };
        // The same file as Faraday's: green, with a check at the box's end.
        let same = kind.is_some() && app.visit_same(name, Some(*size), false);
        if same {
            let gap = if menu { GRID } else { 0.0 };
            super::visit_same::row(ui, (ix, colw, ry), dw + gap);
        }
        ui.checkbox(ix + 22.0, ry + 15.0, on, kind.is_some());
        let fg = if same {
            Role::OnOkContainer
        } else if kind.is_some() {
            Role::OnSurface
        } else {
            Role::OnSurfaceVariant
        };
        let room = 90.0 + dw + super::visit_same::width(same);
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
            colw - if held {
                74.0 + tw + dw + super::visit_same::width(same)
            } else {
                room
            },
        );
        ui.text_as(ix + 54.0, ry + 6.0, BODY_M.mono(), fg, &n);
        if held {
            ui.text_right_as(
                (
                    ix + colw - 20.0 - dw - super::visit_same::width(same),
                    ry + 6.0,
                    20.0,
                ),
                LABEL_M,
                if same {
                    Role::OnOkContainer
                } else {
                    Role::OnSurfaceVariant
                },
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
        let sub_ink = if same {
            Role::OnOkContainer
        } else {
            Role::OnSurfaceVariant
        };
        ui.text_as(ix + 54.0, ry + 28.0, BODY_S, sub_ink, &sub);
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
    if let Some(open) = open {
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
    }
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
    if let Some(line) = pull {
        let line = ui.fit_as(BODY_S, &line, colw - 2.0 * VISIT_TEXT_X);
        ui.text_as(
            ix + VISIT_TEXT_X,
            y + colh + GRID / 2.0,
            BODY_S,
            Role::OnSurfaceVariant,
            &line,
        );
    }
    // Delete files: the one way into deleting from the stick, a grid step
    // above the buttons at the foot so their touch targets keep apart;
    // Unselect all at the row's right end, while any file is ticked.
    // On a small panel a second grid step more, so the button's target reaches
    // the floor between the list's end and the foot.
    let zy = y + colh
        - VISIT_FOOT_ROW
        - foot_more
        - BUTTON_H
        - GRID
        - if ui.compact { GRID } else { 0.0 };
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
    let enabled = nin > 0;
    import_primary(
        app,
        ui,
        (ix + colw, y + colh - VISIT_FOOT_ROW),
        (colw, stacked),
        enabled,
    );
    let (bx, by) = if stacked {
        (
            ix + colw - VISIT_PAD - bw,
            y + colh - VISIT_FOOT_ROW - foot_more,
        )
    } else {
        (ix + colw - 28.0 - load_bw.0 - bw, y + colh - VISIT_FOOT_ROW)
    };
    ui.button(
        bx,
        by,
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
    needed
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

/// What pulling the stick does next, when it does something: the keys an
/// Import and load copied in load, or the vault it brought asks for its
/// passphrase; or the files written are on it and checked.
fn pull_line(app: &View) -> Option<Text> {
    // Only words copied in for Import and load are keys waiting to load.
    let n = app
        .inbox
        .iter()
        .filter(|i| i.kind == FileKind::Words && app.visit.load_after.contains(&*i.name))
        .count();
    let vault = app
        .vault_files()
        .into_iter()
        .find(|f| f.open.is_none() && !f.in_outbox)
        .map(|f| f.name);
    let noun = app.medium.noun();
    match (n, vault) {
        (0, Some(v)) if !app.holds_secret() => {
            Some(Text::fill(EN.pull_line_remove_to_unlock, &[noun, &v]))
        }
        (0, _) if written_and_checked(app) => Some(Text::of(EN.pull_line_written_checked)),
        (0, _) => None,
        (1, _) => Some(Text::fill(EN.pull_line_1_key_copied, &[noun])),
        (n, _) => Some(Text::fill(
            EN.pull_line_keys_copied,
            &[&Text::num(n as u64), noun],
        )),
    }
}

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

/// The width of the filled Import button in a card `colw` wide.
/// Stacked over the other button it is the card's inner width.
fn import_primary_w(app: &View, ui: &Ui, colw: f32, stacked: bool) -> (f32, f32) {
    let pad = VISIT_BUTTON_PAD;
    if stacked {
        return (colw - 2.0 * VISIT_PAD, pad);
    }
    (import_natural_w(app, ui).min(colw / 2.0), pad)
}

/// The filled Import button's width for its whole label.
fn import_natural_w(app: &View, ui: &Ui) -> f32 {
    let sign_label = app
        .visit
        .sign_file()
        .map(|n| Text::fill(EN.visit_import_and_sign, &[n]));
    let load_label = sign_label.as_deref().unwrap_or(EN.visit_import_and_load);
    ui.measure_as(TITLE_S, load_label) + VISIT_BUTTON_PAD
}

/// The filled button, Import and load or, with one PSBT on the stick,
/// Import and sign it, right-aligned at `(right, y)`. Returns its width.
fn import_primary(
    app: &View,
    ui: &mut Ui,
    (right, y): (f32, f32),
    (colw, stacked): (f32, bool),
    on: bool,
) -> f32 {
    let sign = app.visit.sign_file().cloned();
    let sign_label = sign
        .as_ref()
        .map(|n| Text::fill(EN.visit_import_and_sign, &[n]));
    let load_label = sign_label.as_deref().unwrap_or(EN.visit_import_and_load);
    let (load_bw, pad) = import_primary_w(app, ui, colw, stacked);
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
fn visit_inbox_line(app: &View, item: &crate::Item) -> Text {
    if item.kind == FileKind::Vault && item.picture.is_none() && vault_file_open(app, item) {
        let mut out = Text::of(EN.kind_name_vault_open);
        out.push_str(" · ");
        out.push_str(&size_text(item.bytes.len() as u64));
        return out;
    }
    match item.picture {
        Some(FileKind::Other) => Text::fill(
            EN.visit_inbox_line_png_no_qr,
            &[&size_text(item.bytes.len() as u64)],
        ),
        Some(k) => Text::fill(EN.visit_inbox_line_png, &[&kind_line(k, item.bytes.len())]),
        None => kind_line(item.kind, item.bytes.len()),
    }
}

/// Whether the vault an Inbox file holds is open: its own file, or a
/// copy of the one that is.
fn vault_file_open(app: &View, item: &crate::Item) -> bool {
    let files = app.vault_files();
    super::files::vault_at(&files, item).is_some_and(|k| files[k].open.is_some())
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
    ui.press_box(sx - GRID, y + 7.0, lw + 16.0, TOUCH, Action::VisitInAll);
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
