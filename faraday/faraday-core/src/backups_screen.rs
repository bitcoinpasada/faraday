//! The Backups screen (`docs/SIMPLIFY.md` §5.1): drawing only. What it
//! lists is `backups.rs`.

use crate::backup::Tone;
use crate::backups::{Entry, Line};
use crate::screens::title;
use crate::ui::pal::*;
use crate::ui::{Style, Ui, W};
use crate::{Action, Faraday};

/// A line's state colour.
fn tone(t: Tone) -> osk_ui::Color {
    match t {
        Tone::Ok => OK,
        Tone::Warn => WARN,
        Tone::Err => ERR,
        Tone::Dim => DIM,
    }
}

/// A tag's colour, as the backup's map draws it.
fn tag_tone(t: crate::plan::Tag) -> osk_ui::Color {
    match t {
        crate::plan::Tag::Secret => WARN,
        crate::plan::Tag::Sealed => ACCENT,
        crate::plan::Tag::Public => MUTED,
    }
}

/// The wide list's top, its title's height and the gap under it.
const TOP: f32 = 36.0;
const HEAD: f32 = 60.0;
/// A wallet's card: its head, a row per place, the foot, the gap.
const CARD_HEAD: f32 = 68.0;
const ROW: f32 = 28.0;
const CARD_FOOT: f32 = 12.0;
const GAP: f32 = 12.0;

fn card_h(e: &Entry) -> f32 {
    let rows = e.lines.as_ref().map_or(1, |l| l.len().max(1));
    CARD_HEAD + rows as f32 * ROW + CARD_FOOT
}

/// Where the wide list scrolls to show wallet `at` at its top.
pub(crate) fn offset_of(app: &Faraday, at: usize) -> f32 {
    app.backups()
        .iter()
        .take(at)
        .map(|e| card_h(e) + GAP)
        .sum::<f32>()
        + if at > 0 { HEAD } else { 0.0 }
}

pub(crate) fn draw(app: &Faraday, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    let entries = app.backups();
    if ui.compact {
        return compact(app, ui, x0, cw, h, &entries);
    }
    let x = x0 + 48.0;
    let w = cw - 96.0;
    let clip = ui.rect(x0, 0.0, cw, h);
    ui.c.push_clip(clip);
    let top = TOP - app.list_offset;
    let mut y = top;
    title(ui, x, y, "Backups");
    y += HEAD;
    if entries.is_empty() {
        ui.text(x, y, 15.0, W::R, MUTED, "No wallet");
        y += 30.0;
    }
    for (k, e) in entries.iter().enumerate() {
        let ch = card_h(e);
        let edge = if k == app.backups_at && app.backups_at > 0 {
            ACCENT.with_alpha(100)
        } else {
            LINE
        };
        ui.fill(x, y, w, ch, 12.0, SURFACE);
        ui.stroke(x, y, w, ch, 12.0, edge);
        let ix = x + 20.0;
        let iw = w - 40.0;
        let mut bw = 0.0;
        if let Some(i) = e.wallet {
            let label = "Back up";
            bw = ui.measure(14.0, W::S, label) + 36.0;
            ui.button(
                ix + iw - bw,
                y + 16.0,
                Some(bw),
                38.0,
                label,
                Style::Secondary,
                Action::Backup(i),
            );
        }
        let name = ui.fit(16.0, W::S, &e.name, iw - bw - 16.0);
        ui.text(ix, y + 16.0, 16.0, W::S, TEXT, &name);
        let shape = ui.fit(12.0, W::R, &e.shape, iw - bw - 16.0);
        ui.text(ix, y + 40.0, 12.0, W::R, MUTED, &shape);
        let mut ry = y + CARD_HEAD;
        match e.lines.as_ref() {
            None => {
                ui.text_mid(ix, ry, ROW, 13.0, W::S, WARN, "No backup plan");
            }
            Some(lines) => {
                for l in lines {
                    wide_row(ui, ix, ry, iw, l);
                    ry += ROW;
                }
            }
        }
        y += ch + GAP;
    }
    ui.c.pop_clip();
    app.content_h.set(y - top + 40.0);
    ui.report_scroll(clip, app.content_h.get() - h);
}

/// One place on the wide list: where, what it holds, its tag, and what
/// was seen of it.
fn wide_row(ui: &mut Ui, x: f32, y: f32, w: f32, l: &Line) {
    ui.fill(x, y + ROW - 1.0, w, 1.0, 0.0, LINE);
    let place_w = 170.0;
    let tag_w = 64.0;
    let state = l.state.as_str();
    let sw = ui.measure(12.0, W::S, state).min(w * 0.4);
    let state = ui.fit(12.0, W::S, state, sw);
    ui.text_right(x + w, y, ROW, 12.0, W::S, tone(l.tone), &state);
    let place = ui.fit(13.0, W::S, &l.place, place_w - 12.0);
    ui.text_mid(x, y, ROW, 13.0, W::S, TEXT, &place);
    let holds_w = w - place_w - tag_w - sw - 24.0;
    let holds = ui.fit(12.0, W::R, &l.holds, holds_w);
    ui.text_mid(x + place_w, y, ROW, 12.0, W::R, MUTED, &holds);
    if let Some(t) = l.tag {
        ui.text_mid(
            x + place_w + holds_w + 12.0,
            y,
            ROW,
            11.0,
            W::R,
            tag_tone(t),
            t.name(),
        );
    }
}

/// The small panel: one wallet a page, the pages turned at the foot.
fn compact(app: &Faraday, ui: &mut Ui, x0: f32, cw: f32, h: f32, entries: &[Entry]) {
    use crate::compact::M;
    let (x, w) = (x0 + M, cw - 2.0 * M);
    let top = 12.0 - app.list_offset;
    let mut y = top;
    let n = entries.len();
    let Some(e) = entries.get(app.backups_at.min(n.saturating_sub(1))) else {
        ui.text(x, y, 14.0, W::R, MUTED, "No wallet");
        return;
    };
    let at = app.backups_at.min(n - 1);
    y += ui.wrap(x, y, w, 16.0, W::S, TEXT, &e.name) + 4.0;
    y += ui.wrap(x, y, w, 12.0, W::R, MUTED, &e.shape) + 14.0;
    match e.lines.as_ref() {
        None => {
            ui.text(x, y, 13.0, W::S, WARN, "No backup plan");
            y += 28.0;
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
            44.0,
            "Back up",
            Style::Secondary,
            Action::Backup(i),
        );
        y += 56.0;
    }
    if n > 1 {
        let half = (w - 8.0) / 2.0;
        let page = format!("Wallet {} of {n}", at + 1);
        ui.text(x, y, 12.0, W::R, DIM, &page);
        y += 24.0;
        let (prev, next) = (at.checked_sub(1), (at + 1 < n).then_some(at + 1));
        ui.button(
            x,
            y,
            Some(half),
            44.0,
            "Previous",
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
            44.0,
            "Next",
            if next.is_some() {
                Style::Secondary
            } else {
                Style::Disabled
            },
            Action::BackupsPage(next.unwrap_or(at)),
        );
        y += 56.0;
    }
    crate::compact_screens::finish(app, ui, x0, cw, h, y - top + 12.0);
}

/// One place on the small panel: where and what was seen of it on one
/// line, what it holds and its tag under it. Returns the height it took.
fn compact_row(ui: &mut Ui, x: f32, y: f32, w: f32, l: &Line) -> f32 {
    let pw = ui.measure(13.0, W::S, &l.place);
    let sw = ui.measure(12.0, W::S, &l.state);
    let mut cy = y;
    if pw + sw + 12.0 <= w {
        ui.text_right(x + w, cy, 20.0, 12.0, W::S, tone(l.tone), &l.state);
        ui.text_mid(x, cy, 20.0, 13.0, W::S, TEXT, &l.place);
        cy += 22.0;
    } else {
        // Too long for one line: the state on its own, under the place.
        let place = ui.fit(13.0, W::S, &l.place, w);
        ui.text_mid(x, cy, 20.0, 13.0, W::S, TEXT, &place);
        cy += 22.0;
        cy += ui.wrap(x, cy, w, 12.0, W::S, tone(l.tone), &l.state) + 2.0;
    }
    if !l.holds.is_empty() {
        let holds = match l.tag {
            Some(t) => format!("{} · {}", l.holds, t.name()),
            None => l.holds.clone(),
        };
        cy += ui.wrap(x, cy, w, 12.0, W::R, MUTED, &holds);
    }
    cy += 10.0;
    ui.fill(x, cy - 5.0, w, 1.0, 0.0, LINE);
    cy - y
}
