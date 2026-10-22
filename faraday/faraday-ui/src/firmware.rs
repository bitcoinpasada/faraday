//! A firmware screen's sketch (`docs/DECISIONS.md` F18, `docs/STARTER.md`
//! §7.8): a mock-up of a firmware setup screen drawn from a
//! [`faraday_learn::Sketch`], so Faraday Starter
//! draws it. It is drawn to look like a firmware screen, not
//! like Faraday: square corners, a text-mode face (Terminus), and the
//! text-mode colours ([`crate::pal::vga`]) of the palette the sketch
//! names, the nearest to the family's own screen. The row to find is on
//! a highlight bar, its change shown as `[Enabled] → [Disabled]`, and
//! under the frame one line, in Faraday's own secondary text, says it is
//! a mock-up.

use faraday_draw::tokens::{BODY, CAPTION, GAP, GAP_SMALL, GRID, LABEL};
use faraday_learn::sketch::{Dialog, KEY_NAMES, Mark, Nav, Palette, Row, Sketch};
use faraday_strings::EN;

use crate::pal::MUTED;
use crate::pal::vga::*;
use crate::{Ui, W};

/// The most of the frame's width the side list takes before it is
/// folded to the open item alone.
const SIDE_SHARE: f32 = 0.3;

/// The size every word in a sketch is set at, by the frame's width:
/// near a text-mode screen's eighty columns on a wide page, and smaller
/// on a phone so a row still fits.
fn size_for(w: f32) -> f32 {
    if w >= GRID * 80.0 {
        BODY
    } else if w >= GRID * 56.0 {
        LABEL
    } else {
        CAPTION
    }
}

/// The face every word in a sketch is set in.
const FACE: W = W::F;

/// The colours one palette draws a sketch in, all of them
/// [`crate::pal::vga`]'s.
#[derive(Clone, Copy)]
struct Ink {
    /// The size every word is set at ([`size_for`]).
    size: f32,
    /// The screen.
    screen: faraday_draw::Color,
    /// Text.
    text: faraday_draw::Color,
    /// Headings and group names.
    bright: faraday_draw::Color,
    /// The title bar and the tabs' strip.
    bar: faraday_draw::Color,
    /// Text on the bar.
    bar_text: faraday_draw::Color,
    /// The open tab or side item.
    on: faraday_draw::Color,
    /// Text on the open tab or side item.
    on_text: faraday_draw::Color,
    /// The highlight bar on the row to find and the button to choose.
    row: faraday_draw::Color,
    /// Text on the highlight bar.
    row_text: faraday_draw::Color,
    /// The value to set.
    new: faraday_draw::Color,
    /// The frame's lines and the rules inside it.
    line: faraday_draw::Color,
    /// Whether the frame is a double line.
    double: bool,
    /// The key hints' words.
    hint: faraday_draw::Color,
    /// The key a hint names.
    key: faraday_draw::Color,
    /// A dialog's box.
    dialog: faraday_draw::Color,
    /// Text in a dialog.
    dialog_text: faraday_draw::Color,
    /// A dialog's double line.
    dialog_line: faraday_draw::Color,
}

impl Ink {
    /// The firmware palettes, in one place.
    #[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
    const fn of(p: Palette, size: f32) -> Ink {
        match p {
            Palette::Black => Ink {
                size,
                screen: BLACK,
                text: LIGHT_GREY,
                bright: WHITE,
                bar: BLUE,
                bar_text: WHITE,
                on: LIGHT_GREY,
                on_text: BLUE,
                row: BLUE,
                row_text: WHITE,
                new: YELLOW,
                line: CYAN,
                double: false,
                hint: LIGHT_GREY,
                key: LIGHT_CYAN,
                dialog: LIGHT_GREY,
                dialog_text: BLACK,
                dialog_line: BLACK,
            },
            Palette::Blue => Ink {
                size,
                screen: BLUE,
                text: LIGHT_GREY,
                bright: WHITE,
                bar: LIGHT_GREY,
                bar_text: BLUE,
                on: BLACK,
                on_text: WHITE,
                row: RED,
                row_text: WHITE,
                new: YELLOW,
                line: WHITE,
                double: true,
                hint: LIGHT_GREY,
                key: YELLOW,
                dialog: LIGHT_GREY,
                dialog_text: BLACK,
                dialog_line: BLACK,
            },
            Palette::Grey => Ink {
                size,
                screen: LIGHT_GREY,
                text: BLACK,
                bright: BLUE,
                bar: BLUE,
                bar_text: WHITE,
                on: WHITE,
                on_text: BLUE,
                row: BLUE,
                row_text: WHITE,
                new: YELLOW,
                line: BLUE,
                double: false,
                hint: BLACK,
                key: BLUE,
                dialog: WHITE,
                dialog_text: BLACK,
                dialog_line: BLUE,
            },
            Palette::White => Ink {
                size,
                screen: WHITE,
                text: BLACK,
                bright: BLUE,
                bar: BLUE,
                bar_text: WHITE,
                on: LIGHT_GREY,
                on_text: BLACK,
                row: BLUE,
                row_text: WHITE,
                new: YELLOW,
                line: DARK_GREY,
                double: false,
                hint: BLACK,
                key: BLUE,
                dialog: LIGHT_GREY,
                dialog_text: BLACK,
                dialog_line: BLUE,
            },
            Palette::Red => Ink {
                size,
                screen: BLACK,
                text: LIGHT_GREY,
                bright: WHITE,
                bar: RED,
                bar_text: WHITE,
                on: WHITE,
                on_text: RED,
                row: RED,
                row_text: WHITE,
                new: YELLOW,
                line: RED,
                double: false,
                hint: LIGHT_GREY,
                key: LIGHT_RED,
                dialog: LIGHT_GREY,
                dialog_text: BLACK,
                dialog_line: RED,
            },
            Palette::Gold => Ink {
                size,
                screen: BLACK,
                text: LIGHT_GREY,
                bright: YELLOW,
                bar: BROWN,
                bar_text: WHITE,
                on: YELLOW,
                on_text: BLACK,
                row: BROWN,
                row_text: WHITE,
                new: LIGHT_GREEN,
                line: BROWN,
                double: false,
                hint: LIGHT_GREY,
                key: YELLOW,
                dialog: LIGHT_GREY,
                dialog_text: BLACK,
                dialog_line: BROWN,
            },
        }
    }
}

/// Draws `s` with its top-left at (x, y), `w` wide, and under it the
/// line that says it is a mock-up. Returns the height drawn. The height
/// follows the content; at a narrow width the tabs or the side list fold
/// to the open one, and long words are cut.
#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
pub fn sketch<A: Copy + PartialEq>(ui: &mut Ui<A>, s: &Sketch, x: f32, y: f32, w: f32) -> f32 {
    let ink = Ink::of(s.palette, size_for(w));
    let rh = row_height(ui, ink);
    let pad = GAP;
    let inner = w - 2.0 * pad;

    // What goes across the top: the title and the tabs.
    let title_h = if s.title.is_empty() { 0.0 } else { rh };
    let side_w = side_width(ui, s, ink, w);
    let strip: Option<Vec<usize>> = match s.nav {
        Nav::None => None,
        Nav::Tabs => {
            let all: f32 = s
                .items
                .iter()
                .map(|t| ui.measure(ink.size, FACE, t) + 2.0 * GAP)
                .sum();
            Some(if all <= inner {
                (0..s.items.len()).collect()
            } else {
                vec![s.selected]
            })
        }
        // A side list folded to its open item is a strip of one.
        Nav::Side if side_w.is_none() => Some(vec![s.selected]),
        Nav::Side => None,
    };
    let strip_h = if strip.is_some() { rh } else { 0.0 };
    let top_h = title_h + strip_h;

    // The body beside the side list, and the hints under it.
    let (bx, bw) = match side_w {
        Some(sw) => (x + sw + pad, w - sw - 2.0 * pad),
        None => (x + pad, inner),
    };
    let body_h = crate::parts::measured(ui, |ui| body(ui, s, ink, bx, 0.0, bw));
    let side_h = side_w.map_or(0.0, |_| s.items.len() as f32 * rh + 2.0 * GAP_SMALL);
    let keys = key_lines(ui, ink, s.keys, inner);
    let lh = ui.line(ink.size, FACE);
    let keys_h = if keys.is_empty() {
        0.0
    } else {
        keys.len() as f32 * lh + 2.0 * GAP_SMALL
    };
    let box_w = (w - 2.0 * pad).min(GRID * 72.0);
    let dialog_h = s
        .dialog
        .map_or(0.0, |d| dialog_height(ui, &d, ink, box_w) + 4.0 * GAP);
    let mid_h = body_h.max(side_h).max(dialog_h);
    let total = top_h + mid_h + keys_h;

    // The screen, the title and the tabs.
    ui.fill(x, y, w, total, 0.0, ink.screen);
    if title_h > 0.0 {
        ui.fill(x, y, w, title_h, 0.0, ink.bar);
        let t = ui.fit(ink.size, FACE, s.title, inner);
        let tw = ui.measure(ink.size, FACE, &t);
        ui.text_mid(
            x + (w - tw) / 2.0,
            y,
            title_h,
            ink.size,
            FACE,
            ink.bar_text,
            &t,
        );
    }
    if let Some(shown) = &strip {
        let sy = y + title_h;
        ui.fill(x, sy, w, strip_h, 0.0, ink.bar);
        let mut tx = x + pad;
        for &i in shown {
            let label = s.items.get(i).copied().unwrap_or("");
            let room = x + w - pad - tx;
            let label = ui.fit(ink.size, FACE, label, room - 2.0 * GAP);
            let tw = ui.measure(ink.size, FACE, &label) + 2.0 * GAP;
            let on = i == s.selected;
            if on {
                ui.fill(tx, sy, tw, strip_h, 0.0, ink.on);
            }
            let color = if on { ink.on_text } else { ink.bar_text };
            ui.text_mid(tx + GAP, sy, strip_h, ink.size, FACE, color, &label);
            tx += tw;
        }
    }

    // The side list, ruled off from the rows.
    let my = y + top_h;
    if let Some(sw) = side_w {
        let mut iy = my + GAP_SMALL;
        for (i, item) in s.items.iter().enumerate() {
            let on = i == s.selected;
            if on {
                ui.fill(x, iy, sw, rh, 0.0, ink.on);
            }
            let color = if on { ink.on_text } else { ink.text };
            let t = ui.fit(ink.size, FACE, item, sw - 2.0 * GAP);
            ui.text_mid(x + GAP, iy, rh, ink.size, FACE, color, &t);
            iy += rh;
        }
        ui.vrule(x + sw, my, mid_h, ink.line);
    }
    body(ui, s, ink, bx, my, bw);

    // The key hints, ruled off from the rows.
    if keys_h > 0.0 {
        let ky = my + mid_h;
        ui.rule(x, ky, w, ink.line);
        for (n, line) in keys.iter().enumerate() {
            let ly = ky + GAP_SMALL + n as f32 * lh;
            let mut kx = x + pad;
            for k in line {
                kx += hint(ui, ink, kx, ly, k) + 2.0 * GAP;
            }
        }
    }
    frame(ui, x, y, w, total, ink.line, ink.double);

    if let Some(d) = s.dialog {
        let bh = dialog_height(ui, &d, ink, box_w);
        let dx = x + (w - box_w) / 2.0;
        let dy = y + top_h + (mid_h - bh) / 2.0;
        draw_dialog(ui, &d, ink, dx, dy, box_w, bh);
    }

    // Faraday's own line under the frame: this is a mock-up.
    let cy = y + total + GAP_SMALL;
    let caption = ui.wrap(x, cy, w, CAPTION, W::R, MUTED, EN.ui_sketch_mock_up);
    total + GAP_SMALL + caption
}

/// A row's height: one line of the text-mode face and a little air.
fn row_height<A: Copy + PartialEq>(ui: &Ui<A>, ink: Ink) -> f32 {
    ui.line(ink.size, FACE) + GAP_SMALL
}

/// A sharp-cornered frame, single or double.
fn frame<A: Copy + PartialEq>(
    ui: &mut Ui<A>,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    color: faraday_draw::Color,
    double: bool,
) {
    ui.stroke(x, y, w, h, 0.0, color);
    if double {
        let i = GAP_SMALL / 2.0;
        ui.stroke(x + i, y + i, w - 2.0 * i, h - 2.0 * i, 0.0, color);
    }
}

/// `v` as an option in brackets, unless it is empty or already marked.
fn option(v: &str) -> String {
    if v.is_empty() || v.starts_with(['[', '<', '(']) || v.chars().count() == 1 {
        v.to_string()
    } else {
        format!("[{v}]")
    }
}

/// Whether a hint's first word names a key: a function key, an arrow, a
/// sign, or one of the named keys, not a button's first word.
fn is_key(word: &str) -> bool {
    KEY_NAMES.contains(&word)
        || (word.chars().count() <= 5 && word.chars().any(|c| !c.is_ascii_alphabetic()))
}

/// One key hint at (x, y): the key in the key colour, the rest in the
/// hint colour. Returns its width.
fn hint<A: Copy + PartialEq>(ui: &mut Ui<A>, ink: Ink, x: f32, y: f32, k: &str) -> f32 {
    match k.split_once(' ') {
        Some((key, rest)) if is_key(key) => {
            let kw = ui.text(x, y, ink.size, FACE, ink.key, key);
            let sp = ui.measure(ink.size, FACE, " ");
            kw + sp + ui.text(x + kw + sp, y, ink.size, FACE, ink.hint, rest)
        }
        _ => ui.text(x, y, ink.size, FACE, ink.hint, k),
    }
}

/// The side list's width, or `None` when the frame is too narrow for it
/// beside the rows.
fn side_width<A: Copy + PartialEq>(ui: &Ui<A>, s: &Sketch, ink: Ink, w: f32) -> Option<f32> {
    if s.nav != Nav::Side {
        return None;
    }
    let widest = s
        .items
        .iter()
        .map(|t| ui.measure(ink.size, FACE, t))
        .fold(0.0, f32::max);
    let sw = widest + 2.0 * GAP;
    (sw <= w * SIDE_SHARE).then_some(sw)
}

/// The heading, the message and the rows, from (x, y). Returns the
/// height with the gaps around them.
fn body<A: Copy + PartialEq>(ui: &mut Ui<A>, s: &Sketch, ink: Ink, x: f32, y: f32, w: f32) -> f32 {
    let mut cy = y + GAP;
    if !s.heading.is_empty() {
        let t = ui.fit(ink.size, FACE, s.heading, w);
        ui.text(x, cy, ink.size, FACE, ink.bright, &t);
        cy += ui.line(ink.size, FACE) + GAP;
    }
    if !s.text.is_empty() {
        cy += ui.wrap(x, cy, w, ink.size, FACE, ink.text, s.text) + GAP;
    }
    let col = column(ui, ink, s.rows, w);
    for r in s.rows {
        cy += row(ui, r, ink, ink.text, col, x, cy, w);
    }
    cy + GAP - y
}

/// One row: its label, and its value in the value column. The row to
/// find is on the highlight bar, its new value in the value colour.
/// `text` is the colour of a row off the bar; `col` is where the rows'
/// values start, from `x` ([`column`]). Returns its height.
#[allow(clippy::too_many_arguments)]
fn row<A: Copy + PartialEq>(
    ui: &mut Ui<A>,
    r: &Row,
    ink: Ink,
    text: faraday_draw::Color,
    col: f32,
    x: f32,
    y: f32,
    w: f32,
) -> f32 {
    let rh = row_height(ui, ink);
    if r.mark == Mark::Group {
        let t = ui.fit(ink.size, FACE, r.label, w);
        ui.text_mid(x, y, rh, ink.size, FACE, ink.bright, &t);
        return rh;
    }
    let marked = r.mark != Mark::Plain;
    let (lx, right) = if marked {
        (x + GAP_SMALL, x + w - GAP_SMALL)
    } else {
        (x, x + w)
    };
    // A changed setting shows its values as options, `[Enabled]`, unless
    // the firmware's own words already mark them.
    let changed = matches!(r.mark, Mark::Change(_));
    let value = if changed {
        option(r.value)
    } else {
        r.value.to_string()
    };
    let to = match r.mark {
        Mark::Change(to) => Some(format!("→ {}", option(to))),
        _ => None,
    };
    let sp = ui.measure(ink.size, FACE, " ");
    let value_w = ui.measure(ink.size, FACE, &value);
    let to_w = to
        .as_ref()
        .map_or(0.0, |t| ui.measure(ink.size, FACE, t) + sp);
    let label_w = ui.measure(ink.size, FACE, r.label);
    // The value in the rows' common column; else right after the label,
    // at the row's right; else on a line of its own under the label, so
    // that no label is cut.
    let col = x + col;
    let after = right - value_w - to_w;
    let (two, column) = if value.is_empty() {
        (false, right)
    } else if col >= lx + label_w + sp && col <= after {
        (false, col)
    } else if lx + label_w + sp <= after {
        (false, after)
    } else {
        (true, lx + GAP)
    };
    // Every row keeps its whole label: a label wider than the row wraps.
    let wraps = label_w > right - lx;
    let lh = ui.line(ink.size, FACE);
    let label_h = if wraps {
        crate::parts::measure_wrap(ui, right - lx, ink.size, FACE, r.label) + GAP_SMALL
    } else {
        rh
    };
    let h = label_h + if two { rh } else { 0.0 };
    let label_color = if marked {
        ui.fill(x, y, w, h, 0.0, ink.row);
        ink.row_text
    } else {
        text
    };
    if wraps {
        ui.wrap(
            lx,
            y + (rh - lh) / 2.0,
            right - lx,
            ink.size,
            FACE,
            label_color,
            r.label,
        );
    } else {
        let room = if two || value.is_empty() {
            right - lx
        } else {
            column - sp - lx
        };
        let t = ui.fit(ink.size, FACE, r.label, room);
        ui.text_mid(lx, y, rh, ink.size, FACE, label_color, &t);
    }
    let vy = if two { y + label_h } else { y };
    let mut vx = column;
    if !value.is_empty() {
        let v = ui.fit(ink.size, FACE, &value, right - vx + sp / 2.0);
        vx += ui.text_mid(vx, vy, rh, ink.size, FACE, label_color, &v) + sp;
    }
    if let Some(to) = &to {
        let t = ui.fit(ink.size, FACE, to, right - vx + sp / 2.0);
        ui.text_mid(vx, vy, rh, ink.size, FACE, ink.new, &t);
    }
    h
}

/// Where the values of `rows` start, from the rows' left, at width `w`:
/// two spaces after the widest label, or as far right as leaves room
/// for the widest plain value, whichever is less.
fn column<A: Copy + PartialEq>(ui: &Ui<A>, ink: Ink, rows: &[Row], w: f32) -> f32 {
    let sp = ui.measure(ink.size, FACE, " ");
    let (mut label, mut value) = (0.0f32, 0.0f32);
    for r in rows.iter().filter(|r| r.mark != Mark::Group) {
        let inset = if r.mark == Mark::Plain {
            0.0
        } else {
            GAP_SMALL
        };
        label = label.max(ui.measure(ink.size, FACE, r.label) + inset);
        if r.mark == Mark::Plain {
            value = value.max(ui.measure(ink.size, FACE, r.value));
        }
    }
    (label + 2.0 * sp).min(w - value)
}

/// The key hints laid in lines across `w`; a hint wider than `w` is
/// broken at its spaces over lines of its own.
fn key_lines<A: Copy + PartialEq>(ui: &Ui<A>, ink: Ink, keys: &[&str], w: f32) -> Vec<Vec<String>> {
    let mut lines: Vec<Vec<String>> = Vec::new();
    let mut used = w;
    for k in keys {
        let kw = ui.measure(ink.size, FACE, k);
        if kw > w {
            let mut cur = String::new();
            for word in k.split_whitespace() {
                let next = if cur.is_empty() {
                    word.to_string()
                } else {
                    format!("{cur} {word}")
                };
                if !cur.is_empty() && ui.measure(ink.size, FACE, &next) > w {
                    lines.push(vec![core::mem::take(&mut cur)]);
                    cur = word.to_string();
                } else {
                    cur = next;
                }
            }
            lines.push(vec![ui.fit(ink.size, FACE, &cur, w)]);
            used = w;
            continue;
        }
        if used + kw > w {
            lines.push(Vec::new());
            used = 0.0;
        }
        used += kw + 2.0 * GAP;
        if let Some(l) = lines.last_mut() {
            l.push((*k).to_string());
        }
    }
    lines
}

/// A dialog's height at width `w`.
fn dialog_height<A: Copy + PartialEq>(ui: &mut Ui<A>, d: &Dialog, ink: Ink, w: f32) -> f32 {
    crate::parts::measured(ui, |ui| dialog_body(ui, d, ink, 0.0, 0.0, w))
}

/// A dialog's box over the screen: a double line, the title set into
/// its top edge.
fn draw_dialog<A: Copy + PartialEq>(
    ui: &mut Ui<A>,
    d: &Dialog,
    ink: Ink,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
) {
    ui.fill(x, y, w, h, 0.0, ink.dialog);
    let i = GAP_SMALL;
    frame(
        ui,
        x + i,
        y + i,
        w - 2.0 * i,
        h - 2.0 * i,
        ink.dialog_line,
        true,
    );
    if !d.title.is_empty() {
        let t = ui.fit(ink.size, FACE, d.title, w - 4.0 * GAP);
        let tw = ui.measure(ink.size, FACE, &t) + 2.0 * GAP_SMALL;
        let tx = x + (w - tw) / 2.0;
        let lh = ui.line(ink.size, FACE);
        let ty = y + i + GAP_SMALL / 4.0 - lh / 2.0;
        ui.fill(tx, ty, tw, lh, 0.0, ink.dialog);
        ui.text(tx + GAP_SMALL, ty, ink.size, FACE, ink.dialog_text, &t);
    }
    dialog_body(ui, d, ink, x, y, w);
}

/// A dialog's message, choices and buttons. Returns its height.
fn dialog_body<A: Copy + PartialEq>(
    ui: &mut Ui<A>,
    d: &Dialog,
    ink: Ink,
    x: f32,
    y: f32,
    w: f32,
) -> f32 {
    let pad = 2.0 * GAP;
    let iw = w - 2.0 * pad;
    let mut cy = y + pad + GAP_SMALL;
    if !d.text.is_empty() {
        cy += ui.wrap(x + pad, cy, iw, ink.size, FACE, ink.dialog_text, d.text) + GAP;
    }
    let col = column(ui, ink, d.rows, iw);
    for r in d.rows {
        cy += row(ui, r, ink, ink.dialog_text, col, x + pad, cy, iw);
    }
    if !d.buttons.is_empty() {
        cy += GAP;
        let bh = row_height(ui, ink);
        let labels: Vec<String> = d.buttons.iter().map(|b| format!("[ {b} ]")).collect();
        let widths: Vec<f32> = labels
            .iter()
            .map(|b| ui.measure(ink.size, FACE, b) + 2.0 * GAP_SMALL)
            .collect();
        let all = widths.iter().sum::<f32>() + 2.0 * GAP * (widths.len() as f32 - 1.0);
        let mut bx = x + (w - all.min(iw)) / 2.0;
        for (i, (b, bw)) in labels.iter().zip(&widths).enumerate() {
            let on = i == d.choose;
            if on {
                ui.fill(bx, cy, *bw, bh, 0.0, ink.row);
            }
            let color = if on { ink.row_text } else { ink.dialog_text };
            ui.text_mid(bx + GAP_SMALL, cy, bh, ink.size, FACE, color, b);
            bx += bw + 2.0 * GAP;
        }
        cy += bh;
    }
    cy + pad - y
}
