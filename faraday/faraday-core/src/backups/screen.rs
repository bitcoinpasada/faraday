//! The Backups screen (`docs/history/SIMPLIFY.md` §5.1): drawing only. What it
//! lists is `backups.rs`.

use crate::Action;
use crate::backups::{Entry, Line};
use crate::drawing::View;
use crate::screens::title;
use crate::strings::EN;
use crate::strings::fill;
use crate::ui::pal::*;
use crate::ui::{Style, Ui};
use crate::wallet_backup::Tone;
use faraday_draw::tokens::{BODY_M, BODY_S, BUTTON_H, LABEL_L, LABEL_M, TITLE_M};

/// A line's state colour.
fn tone(t: Tone) -> crate::ui::Paint {
    match t {
        Tone::Ok => Role::Ok.paint(),
        Tone::Warn => Role::Warning.paint(),
        Tone::Err => Role::Error.paint(),
        Tone::Dim => Role::OnSurfaceVariant.paint(),
    }
}

/// A tag's colour, as the backup's map draws it.
fn tag_tone(t: crate::plan::Tag) -> crate::ui::Paint {
    match t {
        crate::plan::Tag::Secret => Role::Warning.paint(),
        crate::plan::Tag::Sealed => Role::Primary.paint(),
        crate::plan::Tag::Public => Role::OnSurfaceVariant.paint(),
    }
}

/// The wide page's title's top.
const TOP: f32 = 36.0;
/// A wallet's card: its head, a row per place, the foot.
const CARD_HEAD: f32 = 68.0;
const ROW: f32 = 28.0;
const CARD_FOOT: f32 = 12.0;

fn card_h(e: &Entry) -> f32 {
    let rows = e.lines.as_ref().map_or(1, |l| l.len().max(1));
    CARD_HEAD + rows as f32 * ROW + CARD_FOOT
}

pub(crate) fn draw(app: &View, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    use crate::panes::screen::{Item, ListPane, shown, split};
    let entries = app.backups();
    if ui.compact && !shown(app, crate::Screen::Backups) {
        return compact(app, ui, (x0, cw, h), &entries, true);
    }
    // List and detail (`docs/LAYOUT.md` §5): the wallets in the list
    // pane, the one chosen's map beside it.
    let p = split(ui, x0, cw);
    let at = app.backups_area.at().min(entries.len().saturating_sub(1));
    if !ui.compact {
        title(ui, p.list.0, TOP, EN.backups_backups);
    }
    let mut pane = ListPane::begin(app, ui, (p.list.0, p.top, p.list.1, h - p.top));
    for (k, e) in entries.iter().enumerate() {
        let item = Item {
            headline: &e.name,
            supporting: &e.shape,
            trailing: None,
            action: Action::BackupsPage(k),
        };
        pane.item(ui, &item, k == at);
    }
    pane.end(ui);
    let (dx, dw) = p.detail;
    if ui.compact {
        let m = crate::compact::M;
        let area = (dx - m, p.reading() + 2.0 * m, h);
        return compact(app, ui, area, &entries, false);
    }
    let Some(e) = entries.get(at) else {
        ui.text_as(
            dx,
            p.top,
            BODY_M,
            Role::OnSurfaceVariant,
            EN.backups_no_wallet,
        );
        return;
    };
    // The map's rows, its Back up and its name read together: the card
    // keeps the detail's reading width.
    let clip = ui.rect(dx, p.top, dw, h - p.top);
    ui.c.push_clip(clip);
    let top = p.top - app.list_offset;
    let ch = wide_card(app, ui, (dx, top, p.reading()), e);
    ui.c.pop_clip();
    app.scroll.content_h().set(ch);
    ui.report_scroll(clip, ch - (h - p.top));
}

/// One wallet's backup on the wide layout: its name and shape, Back up,
/// and a row per place of its map, from (x, y) `w` wide. Returns the
/// height.
fn wide_card(app: &View, ui: &mut Ui, (x, y, w): (f32, f32, f32), e: &Entry) -> f32 {
    let ch = card_h(e);
    ui.fill(x, y, w, ch, 12.0, Role::SurfaceContainerLow);
    ui.stroke(x, y, w, ch, 12.0, Role::OutlineVariant);
    let ix = x + 20.0;
    let iw = w - 40.0;
    let mut bw = 0.0;
    if let Some(i) = e.wallet {
        let label = back_up_label(app, i);
        bw = ui.measure_as(LABEL_L, label) + 36.0;
        ui.button(
            ix + iw - bw,
            y + 16.0,
            Some(bw),
            BUTTON_H,
            label,
            Style::Secondary,
            Action::Backup(i),
        );
    }
    let name = ui.fit_as(TITLE_M, &e.name, iw - bw - 16.0);
    ui.text_as(ix, y + 16.0, TITLE_M, Role::OnSurface, &name);
    let shape = ui.fit_as(BODY_S, &e.shape, iw - bw - 16.0);
    ui.text_as(ix, y + 40.0, BODY_S, Role::OnSurfaceVariant, &shape);
    let mut ry = y + CARD_HEAD;
    match e.lines.as_ref() {
        None => {
            ui.text_mid_as(
                (ix, ry, ROW),
                LABEL_L,
                Role::Warning,
                EN.backups_no_backup_plan,
            );
        }
        Some(lines) => {
            for l in lines {
                wide_row(ui, ix, ry, iw, l);
                ry += ROW;
            }
        }
    }
    ch
}

/// A wallet's Back up, or Continue the backup for one left part way
/// (`docs/history/NEW-WALLET.md` §12.3).
fn back_up_label(app: &View, i: usize) -> &'static str {
    if app.wallet_backup_unfinished(i) {
        EN.backup_flow_continue
    } else {
        EN.backups_back_up
    }
}

/// One place on the wide list: where, what it holds, its tag, and what
/// was seen of it.
fn wide_row(ui: &mut Ui, x: f32, y: f32, w: f32, l: &Line) {
    ui.fill(x, y + ROW - 1.0, w, 1.0, 0.0, Role::OutlineVariant);
    let place_w = 170.0;
    let tag_w = 64.0;
    let state = l.state.as_str();
    let sw = ui.measure_as(LABEL_M, state).min(w * 0.4);
    let state = ui.fit_as(LABEL_M, state, sw);
    ui.text_right_as((x + w, y, ROW), LABEL_M, tone(l.tone), &state);
    let place = ui.fit_as(LABEL_L, &l.place, place_w - 12.0);
    ui.text_mid_as((x, y, ROW), LABEL_L, Role::OnSurface, &place);
    let holds_w = w - place_w - tag_w - sw - 24.0;
    let holds = ui.fit_as(BODY_S, &l.holds, holds_w);
    ui.text_mid_as(
        (x + place_w, y, ROW),
        BODY_S,
        Role::OnSurfaceVariant,
        &holds,
    );
    if let Some(t) = l.tag {
        ui.text_mid_as(
            (x + place_w + holds_w + 12.0, y, ROW),
            BODY_S,
            tag_tone(t),
            t.name(),
        );
    }
}

/// The small panel: one wallet a page, the pages turned at the foot
/// (`pages`); beside the list pane, the one chosen, with no pages to turn.
fn compact(app: &View, ui: &mut Ui, (x0, cw, h): (f32, f32, f32), entries: &[Entry], pages: bool) {
    use crate::compact::M;
    let (x, w) = (x0 + M, cw - 2.0 * M);
    let top = 12.0 - app.list_offset;
    let mut y = top;
    let n = entries.len();
    let Some(e) = entries.get(app.backups_area.at().min(n.saturating_sub(1))) else {
        ui.text_as(x, y, BODY_M, Role::OnSurfaceVariant, EN.backups_no_wallet);
        return;
    };
    let at = app.backups_area.at().min(n - 1);
    y += ui.wrap_as((x, y, w), TITLE_M, Role::OnSurface, &e.name) + 4.0;
    y += ui.wrap_as((x, y, w), BODY_S, Role::OnSurfaceVariant, &e.shape) + 14.0;
    match e.lines.as_ref() {
        None => {
            ui.text_as(x, y, LABEL_L, Role::Warning, EN.backups_no_backup_plan);
            y += ui.line_as(LABEL_L) + faraday_draw::tokens::GRID;
        }
        Some(lines) => {
            for l in lines {
                y += compact_row(ui, x, y, w, l);
            }
            y += 6.0;
        }
    }
    if let Some(i) = e.wallet {
        ui.button(
            x,
            y,
            Some(w),
            BUTTON_H,
            back_up_label(app, i),
            Style::Secondary,
            Action::Backup(i),
        );
        y += 56.0;
    }
    if n > 1 && pages {
        let half = (w - 8.0) / 2.0;
        let page = fill(
            EN.backups_wallet_count,
            &[&(at + 1).to_string(), &n.to_string()],
        );
        ui.text_as(x, y, BODY_S, Role::OnSurfaceVariant, &page);
        y += ui.line_as(BODY_S) + faraday_draw::tokens::GRID;
        let (prev, next) = (at.checked_sub(1), (at + 1 < n).then_some(at + 1));
        ui.button(
            x,
            y,
            Some(half),
            BUTTON_H,
            EN.backups_previous,
            if prev.is_some() {
                Style::Secondary
            } else {
                Style::Disabled
            },
            Action::BackupsPage(prev.unwrap_or(at)),
        );
        ui.button(
            x + half + 8.0,
            y,
            Some(half),
            BUTTON_H,
            EN.backups_next,
            if next.is_some() {
                Style::Secondary
            } else {
                Style::Disabled
            },
            Action::BackupsPage(next.unwrap_or(at)),
        );
        y += 56.0;
    }
    crate::compact::screen::finish(app, ui, x0, cw, h, y - top + 12.0);
}

/// One place on the small panel: where and what was seen of it on one
/// line, what it holds and its tag under it. Returns the height it took.
fn compact_row(ui: &mut Ui, x: f32, y: f32, w: f32, l: &Line) -> f32 {
    let pw = ui.measure_as(LABEL_L, &l.place);
    let sw = ui.measure_as(LABEL_M, &l.state);
    let mut cy = y;
    if pw + sw + 12.0 <= w {
        ui.text_right_as((x + w, cy, 20.0), LABEL_M, tone(l.tone), &l.state);
        ui.text_mid_as((x, cy, 20.0), LABEL_L, Role::OnSurface, &l.place);
        cy += 22.0;
    } else {
        // Too long for one line: the state on its own, under the place.
        let place = ui.fit_as(LABEL_L, &l.place, w);
        ui.text_mid_as((x, cy, 20.0), LABEL_L, Role::OnSurface, &place);
        cy += 22.0;
        cy += ui.wrap_as((x, cy, w), LABEL_M, tone(l.tone), &l.state) + 2.0;
    }
    if !l.holds.is_empty() {
        let holds = match l.tag {
            Some(t) => format!("{} · {}", l.holds, t.name()),
            None => l.holds.clone(),
        };
        cy += ui.wrap_as((x, cy, w), BODY_S, Role::OnSurfaceVariant, &holds);
    }
    cy += 10.0;
    ui.fill(x, cy - 5.0, w, 1.0, 0.0, Role::OutlineVariant);
    cy - y
}
