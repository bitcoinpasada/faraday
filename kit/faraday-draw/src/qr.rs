//! A QR code's modules to squares (09 §3.2).

use faraday_codec::qr::QrMatrix;
use faraday_tokens::palette::Color;

use crate::canvas::Canvas;
use crate::geom::Rect;

/// Writes `code` into `at` on a white square: whole pixels per module, the quiet zone
/// included, centred, each row of modules written straight into the canvas. Nothing is kept,
/// so no secret's code outlives its frame. Costs a write of each dark pixel.
pub fn modules(canvas: &mut Canvas, code: &QrMatrix, at: Rect) {
    canvas.fill_rect(at, Color::WHITE);
    let n = code.size();
    let quiet = 4.0;
    // Whole pixels per module, so every module is the same size.
    let module_px = (at.w.min(at.h) as f32 / (n as f32 + 2.0 * quiet))
        .floor()
        .max(1.0) as i32;
    let used = module_px * n as i32;
    let ox = at.x + (at.w - used) / 2;
    let oy = at.y + (at.h - used) / 2;
    canvas.fill_cells((ox, oy), (n, module_px), |col, row| code.module(col, row), Color::BLACK);
}
