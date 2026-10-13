//! Pieces of a page that the desktop app's screens and sheets and
//! Faraday's other apps share: a title, a section's label, buttons in
//! rows, a sheet's box, a QR code, and the height of what is not drawn.

use crate::pal::*;
use crate::{Style, Ui, W};

/// A page's title. Returns its height.
pub fn title<A: Copy + PartialEq>(ui: &mut Ui<A>, x: f32, y: f32, s: &str) -> f32 {
    ui.text(x, y, 28.0, W::S, TEXT, s);
    ui.line(28.0, W::S)
}

/// A section's label.
pub fn section_label<A: Copy + PartialEq>(ui: &mut Ui<A>, x: f32, y: f32, s: &str) {
    ui.text(x, y, 14.0, W::S, MUTED, s);
}

/// Buttons laid left to right, wrapping to a new row at the width.
/// Returns the height they took, a row being 44.
pub fn button_rows<A: Copy + PartialEq>(
    ui: &mut Ui<A>,
    x: f32,
    y: f32,
    w: f32,
    items: &[(String, Style, A)],
) -> f32 {
    let (mut bx, mut by) = (x, y);
    for (label, style, action) in items {
        // No wider than the row; a long label is cut.
        let bw = (ui.measure(13.0, W::S, label) + 32.0).min(w);
        if bx > x && bx + bw > x + w {
            bx = x;
            by += 40.0;
        }
        let label = ui.fit(13.0, W::S, label, bw - 24.0);
        bx += ui.button(bx, by, Some(bw), 32.0, &label, *style, *action) + 6.0;
    }
    by - y + 44.0
}

/// Buttons `h` high laid left to right, wrapping to a new row at the
/// width, as a narrow column needs. Returns the height they took and the
/// gap under them.
pub fn wrap_buttons<A: Copy + PartialEq>(
    ui: &mut Ui<A>,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    items: &[(&str, Style, A)],
) -> f32 {
    let size = if h >= 38.0 { 14.0 } else { 13.0 };
    let (mut bx, mut by) = (x, y);
    for &(label, style, action) in items {
        let bw = (ui.measure(size, W::S, label) + 32.0).min(w);
        if bx > x && bx + bw > x + w {
            bx = x;
            by += h + 8.0;
        }
        let label = ui.fit(size, W::S, label, bw - 24.0);
        bx += ui.button(bx, by, Some(bw), h, &label, style, action) + 8.0;
    }
    by - y + h + 8.0
}

/// The height `draw` takes, drawing nothing and keeping no hits: what a
/// card is measured with before its surface goes down.
pub fn measured<A: Copy + PartialEq>(ui: &mut Ui<A>, draw: impl FnOnce(&mut Ui<A>) -> f32) -> f32 {
    let mark = ui.hits.len();
    ui.c.push_clip(osk_ui::Rect::new(0, 0, 0, 0));
    let h = draw(ui);
    ui.c.pop_clip();
    ui.hits.truncate(mark);
    h
}

/// The height `s` takes wrapped to `width`, drawn nowhere.
pub fn measure_wrap<A: Copy + PartialEq>(
    ui: &mut Ui<A>,
    width: f32,
    size: f32,
    w: W,
    s: &str,
) -> f32 {
    ui.c.push_clip(osk_ui::Rect::new(0, 0, 0, 0));
    let h = ui.wrap(0.0, 0.0, width, size, w, TEXT, s);
    ui.c.pop_clip();
    h
}

/// A sheet's box, `sw` by `sh`, centred in a frame `w` by `h` over the
/// page, with its shadow; a press beside it takes [`Ui::outside`].
/// Returns its top-left corner.
pub fn sheet_box<A: Copy + PartialEq>(
    ui: &mut Ui<A>,
    w: f32,
    h: f32,
    sw: f32,
    sh: f32,
) -> (f32, f32) {
    let x = (w - sw) / 2.0;
    let y = (h - sh) / 2.0;
    if let Some(a) = ui.outside {
        ui.hit_around(x, y, sw, sh, a);
    }
    ui.shadow(x, y, sw, sh, 16.0);
    ui.fill(x, y, sw, sh, 16.0, SURFACE);
    ui.stroke(x, y, sw, sh, 16.0, BORDER);
    (x, y)
}

/// A QR code on a white card, square modules, the quiet zone included.
pub fn qr_code<A: Copy + PartialEq>(
    ui: &mut Ui<A>,
    m: &osk_codec::qr::QrMatrix,
    x: f32,
    y: f32,
    side: f32,
) {
    ui.fill(x, y, side, side, 8.0, osk_ui::Color::WHITE);
    let n = m.size();
    let quiet = 4.0;
    // Whole pixels per module, so every module is the same size.
    let px_side = ui.px(side);
    let module_px = (px_side as f32 / (n as f32 + 2.0 * quiet)).floor().max(1.0) as i32;
    let used = module_px * n as i32;
    let r = ui.rect(x, y, side, side);
    let ox = r.x + (r.w - used) / 2;
    let oy = r.y + (r.h - used) / 2;
    for row in 0..n {
        let mut col = 0;
        while col < n {
            if m.module(col, row) {
                let start = col;
                while col < n && m.module(col, row) {
                    col += 1;
                }
                let rect = osk_ui::Rect::new(
                    ox + start as i32 * module_px,
                    oy + row as i32 * module_px,
                    (col - start) as i32 * module_px,
                    module_px,
                );
                ui.c.fill_rect(rect, osk_ui::Color::BLACK);
            } else {
                col += 1;
            }
        }
    }
}
