//! A sheet on a small panel: as tall as what it holds, its buttons kept
//! at its foot, and what is above them scrolled when it is taller than
//! the panel.

use faraday_draw::widgets::Icon;

use crate::pal::*;
use crate::{Style, Ui, W};

/// What a sheet holds: drawn at (x, y) in a width, it returns the height
/// it took.
pub type Body<'b, A> = dyn for<'r, 'u> FnMut(&'r mut Ui<'u, A>, f32, f32, f32) -> f32 + 'b;

/// A sheet on a small panel: the panel's width less a margin, as tall as
/// what `body` draws (measured first, drawing nothing), centred. `body`
/// draws at (x, y) in a width and returns the height it took.
pub fn sheet<A: Copy + PartialEq>(ui: &mut Ui<A>, w: f32, h: f32, body: &mut Body<'_, A>) {
    scroll_sheet(ui, (8.0, w - 16.0), h, 8.0, 16.0, body);
}

/// A sheet `sw` wide at `sx`, as tall as what `body` draws and at most
/// `h` less `margin` above and below, centred, with `pad` inside: what is
/// above its buttons scrolls when it is taller. [`sheet`] on a small
/// panel; a long sheet on the desktop too.
pub fn scroll_sheet<A: Copy + PartialEq>(
    ui: &mut Ui<A>,
    (sx, sw): (f32, f32),
    h: f32,
    margin: f32,
    pad: f32,
    body: &mut Body<'_, A>,
) {
    let (ix, iw) = (sx + pad, sw - 2.0 * pad);
    // The sheet's buttons ([`buttons`]) are kept at its foot, and what is
    // above them scrolls when the sheet is taller than the panel.
    let mark = ui.hits.len();
    ui.c.push_clip(faraday_draw::Rect::new(0, 0, 0, 0));
    ui.sheet_pin = None;
    ui.sheet_pinning = true;
    let ch = body(ui, ix, 0.0, iw);
    let pins = ui.sheet_pin.take();
    let items: Vec<(&str, Style, A)> = pins
        .iter()
        .flatten()
        .map(|(l, s, a)| (l.as_str(), *s, *a))
        .collect();
    ui.sheet_pinning = false;
    let bh = buttons(ui, ix, 0.0, iw, &items);
    ui.c.pop_clip();
    ui.hits.truncate(mark);
    let foot = if items.is_empty() { 0.0 } else { bh + pad };
    let sh = (ch + 2.0 * pad + foot).min(h - 2.0 * margin);
    let sy = ((h - sh) / 2.0).max(margin);
    if let Some(a) = ui.outside {
        ui.hit_around(sx, sy, sw, sh, a);
    }
    ui.shadow(sx, sy, sw, sh, 14.0);
    ui.fill(sx, sy, sw, sh, 14.0, SURFACE);
    ui.stroke(sx, sy, sw, sh, 14.0, BORDER);
    // Taller than the panel: what is above the buttons scrolls.
    let view = sh - foot;
    let max = (ch + 2.0 * pad - view).max(0.0);
    let off = ui.offset.clamp(0.0, max);
    let clip = ui.rect(sx, sy, sw, view);
    ui.c.push_clip(clip);
    ui.sheet_pinning = true;
    body(ui, ix, sy + pad - off, iw);
    ui.sheet_pinning = false;
    ui.sheet_pin = None;
    ui.c.pop_clip();
    ui.report_scroll(clip, max);
    if !items.is_empty() {
        if max > 0.0 {
            ui.fill(sx + 1.0, sy + view, sw - 2.0, 1.0, 0.0, LINE);
        }
        buttons(ui, ix, sy + view, iw, &items);
    }
}

/// A sheet's title: its icon, and the words wrapped beside it. Returns
/// its height.
pub fn sheet_head<A: Copy + PartialEq>(
    ui: &mut Ui<A>,
    x: f32,
    y: f32,
    w: f32,
    icon: impl Into<crate::Glyph>,
    tone: faraday_draw::Color,
    title: &str,
) -> f32 {
    ui.icon(x - 4.0, y - 3.0, 28.0, icon, 15.0, tone);
    let th = ui.wrap(x + 28.0, y, w - 28.0, 17.0, W::S, TEXT, title);
    th.max(22.0) + 10.0
}

/// A label over its value, the value wrapped. Returns its height.
pub fn kv<A: Copy + PartialEq>(
    ui: &mut Ui<A>,
    x: f32,
    y: f32,
    w: f32,
    key: &str,
    value: &str,
    tone: faraday_draw::Color,
) -> f32 {
    ui.text(x, y, 12.0, W::R, MUTED, key);
    let vh = ui.wrap(x, y + 18.0, w, 14.0, W::R, tone, value);
    ui.rule(x, y + 24.0 + vh, w, INNER);
    vh + 32.0
}

/// A row of buttons: side by side when every label fits its share, one
/// under another, full width, when not. Returns their height.
pub fn buttons<A: Copy + PartialEq>(
    ui: &mut Ui<A>,
    x: f32,
    y: f32,
    w: f32,
    items: &[(&str, Style, A)],
) -> f32 {
    if items.is_empty() {
        return 0.0;
    }
    // Inside a sheet, they go to its foot ([`sheet`]).
    if ui.sheet_pinning {
        ui.sheet_pin = Some(
            items
                .iter()
                .map(|&(l, s, a)| (l.to_string(), s, a))
                .collect(),
        );
        return 0.0;
    }
    let n = items.len() as f32;
    let share = (w - (n - 1.0) * 8.0) / n;
    let fits = items
        .iter()
        .all(|(l, ..)| ui.measure(15.0, W::S, l) + 24.0 <= share);
    if fits {
        for (i, &(label, style, action)) in items.iter().enumerate() {
            ui.button(
                x + i as f32 * (share + 8.0),
                y,
                Some(share),
                46.0,
                label,
                style,
                action,
            );
        }
        46.0
    } else {
        for (i, &(label, style, action)) in items.iter().enumerate() {
            ui.button(x, y + i as f32 * 54.0, Some(w), 46.0, label, style, action);
        }
        n * 54.0 - 8.0
    }
}

/// The sheets that say what a lock does: a title, a line, rows of what
/// happens to what, and their buttons.
pub fn kv_sheet<A: Copy + PartialEq>(
    ui: &mut Ui<A>,
    w: f32,
    h: f32,
    head: (Icon, faraday_draw::Color, &str),
    sub: &str,
    rows: &[(&str, String, faraday_draw::Color)],
    actions: &[(&str, Style, A)],
) {
    sheet(ui, w, h, &mut |ui, x, y, iw| {
        let mut cy = y;
        cy += sheet_head(ui, x, cy, iw, head.0, head.1, head.2);
        if !sub.is_empty() {
            cy += ui.wrap(x, cy, iw, 13.0, W::R, MUTED, sub) + 12.0;
        }
        for (k, v, tone) in rows {
            cy += kv(ui, x, cy, iw, k, v, *tone);
        }
        cy += 8.0;
        cy += buttons(ui, x, cy, iw, actions);
        cy - y
    });
}
