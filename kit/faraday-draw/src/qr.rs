//! A QR code's modules to squares (09 §3.2).

use faraday_codec::qr::QrMatrix;

use crate::canvas::Canvas;
use crate::geom::Rect;

/// Writes `code` into `at` on a white square: whole pixels per module, the quiet zone
/// included, centred, each row of modules written straight into the canvas. Nothing is kept,
/// so no secret's code outlives its frame. Costs a write of each dark pixel.
pub fn modules(_canvas: &mut Canvas, _code: &QrMatrix, _at: Rect) {
    todo!()
}
