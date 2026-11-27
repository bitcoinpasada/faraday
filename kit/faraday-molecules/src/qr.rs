//! The QR molecule: a code on a white card with its caption under it (04 §12.1, 09 §3.2).

use faraday_atoms::{Pressed, Shown, Ui};
use faraday_codec::qr::QrMatrix;
use faraday_tokens::dims::BODY_S;
use faraday_tokens::palette::{Color, Role};

/// A QR code and what is written under it.
pub struct QrModel<'s> {
    /// The code's modules.
    pub code: &'s QrMatrix,
    /// The text under the code, in the mono face; empty for none. A secret's is `Shown::secret`.
    pub caption: Shown<'s>,
}

/// The side of the code's card on a compact screen and on a wide one.
const SIDE_COMPACT: f32 = 104.0;
const SIDE_WIDE: f32 = 160.0;

/// The gap between the card and its caption.
const CAPTION_GAP: f32 = 10.0;

/// The side of the card `w` wide.
fn side<A: Pressed>(ui: &Ui<'_, A>, w: f32) -> f32 {
    (if ui.compact { SIDE_COMPACT } else { SIDE_WIDE }).min(w)
}

/// A code on a white card, square modules, the quiet zone included, its top left at (x, y) and
/// `side` wide: each row of modules written straight into the canvas and nothing kept, so no
/// secret's code outlives its frame. Costs a write of each dark pixel.
pub fn code<A: Pressed>(ui: &mut Ui<'_, A>, m: &QrMatrix, (x, y): (f32, f32), side: f32) {
    ui.fill(x, y, side, side, 8.0, Color::WHITE);
    let n = m.size();
    let quiet = 4.0;
    // Whole pixels per module, so every module is the same size.
    let px_side = ui.px(side);
    let module_px = (px_side as f32 / (n as f32 + 2.0 * quiet)).floor().max(1.0) as i32;
    let used = module_px * n as i32;
    let r = ui.rect(x, y, side, side);
    let ox = r.x + (r.w - used) / 2;
    let oy = r.y + (r.h - used) / 2;
    ui.c.fill_cells(
        (ox, oy),
        (n, module_px),
        |col, row| m.module(col, row),
        Color::BLACK,
    );
}

/// Draws the code from (x, y), at most `w` wide, and under it its caption wrapped to `w`.
/// Returns the height.
pub fn draw<A: Pressed>(m: &QrModel<'_>, ui: &mut Ui<'_, A>, (x, y, w): (f32, f32, f32)) -> f32 {
    let side = side(ui, w);
    code(ui, m.code, (x, y), side);
    if m.caption.as_str().is_empty() {
        return side;
    }
    let cy = y + side + CAPTION_GAP;
    let h = ui.showing(&m.caption, |ui| {
        ui.wrap_as(
            (x, cy, w),
            BODY_S.mono(),
            Role::OnSurfaceVariant,
            m.caption.as_str(),
        )
    });
    side + CAPTION_GAP + h
}
