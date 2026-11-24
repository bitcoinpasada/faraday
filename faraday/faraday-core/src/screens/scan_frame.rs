//! The scanner's camera picture and what is drawn over it: the found
//! code's outline, the brackets, the state line and the parts bar
//! (`docs/DESIGN.md` §4.9, `docs/SCANNING.md` §7). One drawing for the
//! scanner sheet and the pane that holds it beside a card; both read the
//! same scan state, so there is one camera.

use crate::ui::Ui;
use crate::ui::pal::*;
use faraday_draw::tokens as T;
use faraday_draw::tokens::BODY_S;

/// The frame in the `(x, y, w, h)` box, on the surface the caller filled: the
/// camera's picture with the found code's outline while a picture is
/// drawn, then the brackets, the state line and the parts bar. A QR found
/// and read and still in view turns the brackets green. Nothing is kept:
/// the picture is the scan state's, drawn from it each frame.
pub(crate) fn draw(ui: &mut Ui, (x, y, w, h): (f32, f32, f32, f32), sc: &crate::ScanState) {
    let rect = ui.rect(x, y, w, h);
    let mut read_in_view = false;
    if let Some((fw, fh, luma)) = sc.frame.as_ref() {
        ui.c.luma(
            rect,
            *fw,
            *fh,
            luma,
            sc.chroma.as_ref().map(|c| c.as_slice()),
        );
        // Gone ten preview frames after the scanner stops seeing it.
        if let Some((seen, at)) = sc.seen
            && sc.frames.wrapping_sub(at) <= crate::SEEN_FRAMES
        {
            read_in_view = seen.read;
            found_outline(ui, rect, (*fw, *fh), &seen);
        }
    }
    viewfinder_over(ui, (x, y, w, h), sc, read_in_view);
}

/// The code the scanner last found, outlined where it is in the picture
/// drawn in `rect` from a `frame`-sized preview (`docs/QR.md` §8 item
/// 4): ok once read, warning while found and not read, its corners
/// rounded, edged each side in the role's on-colour so the line reads
/// over a light picture and a dark one.
fn found_outline(
    ui: &mut Ui,
    rect: faraday_draw::geom::Rect,
    (fw, fh): (u16, u16),
    seen: &crate::Seen,
) {
    if seen.width == 0 || seen.height == 0 || fw == 0 || fh == 0 {
        return;
    }
    // The preview fills its box and crops the longer side, as
    // `Canvas::luma` draws it: the same scale and crop here. The
    // scanner's frame may be larger than the preview (the shells reduce
    // the frame they draw), so its corners are taken to the preview's
    // pixels first.
    let (fw, fh) = (f32::from(fw), f32::from(fh));
    let scale = (rect.w as f32 / fw).max(rect.h as f32 / fh);
    let left = (fw - rect.w as f32 / scale) / 2.0;
    let top = (fh - rect.h as f32 / scale) / 2.0;
    let points = seen.corners.map(|(cx, cy)| {
        let px = f32::from(cx) * fw / f32::from(seen.width);
        let py = f32::from(cy) * fh / f32::from(seen.height);
        (
            rect.x as f32 + (px - left) * scale,
            rect.y as f32 + (py - top) * scale,
        )
    });
    let (line, edge) = if seen.read {
        (Role::Ok, Role::OnOk)
    } else {
        (Role::Warning, Role::OnWarning)
    };
    let lw = T::VIEWFINDER_OUTLINE_STROKE * ui.f;
    let ew = lw + 2.0 * T::VIEWFINDER_OUTLINE_EDGE * ui.f;
    // Round the code, not over its modules: each corner moved out from
    // the middle by half the stroke across the corner's diagonal
    // (half of `ew` times the square root of two).
    let n = points.len() as f32;
    let (mx, my) = (
        points.iter().map(|p| p.0).sum::<f32>() / n,
        points.iter().map(|p| p.1).sum::<f32>() / n,
    );
    let points = points.map(|(px, py)| {
        let (dx, dy) = (px - mx, py - my);
        let d = (dx * dx + dy * dy).sqrt().max(1.0);
        let out = ew * std::f32::consts::FRAC_1_SQRT_2;
        (px + dx / d * out, py + dy / d * out)
    });
    ui.c.push_clip(rect);
    let (edge, line) = (ui.col(edge), ui.col(line));
    ui.c.stroke_closed(&points, ew, edge);
    ui.c.stroke_closed(&points, lw, line);
    ui.c.pop_clip();
}

/// What is drawn over the camera's picture (`docs/QR.md` §8 item 4):
/// the corner brackets, white while looking and green while a QR that
/// read is in view, and at the bottom, on a translucent band, the state
/// line and the bar of parts read.
fn viewfinder_over(
    ui: &mut Ui,
    (x, y, w, h): (f32, f32, f32, f32),
    sc: &crate::ScanState,
    read: bool,
) {
    use faraday_draw::tokens;
    let tone = if read { VIEWFINDER_READ } else { VIEWFINDER };
    let arm = w.min(h) * tokens::VIEWFINDER_BRACKET_SHARE;
    let (st, inset) = (
        tokens::VIEWFINDER_BRACKET_STROKE,
        tokens::VIEWFINDER_BRACKET_INSET,
    );
    let (l, t) = (x + inset, y + inset);
    let (r, b) = (x + w - inset, y + h - inset);
    for (cx, right) in [(l, false), (r, true)] {
        for (cy, bottom) in [(t, false), (b, true)] {
            let ax = if right { cx - arm } else { cx };
            let ay = if bottom { cy - st } else { cy };
            ui.fill(ax, ay, arm, st, 0.0, tone);
            let bx = if right { cx - st } else { cx };
            let by = if bottom { cy - arm } else { cy };
            ui.fill(bx, by, st, arm, 0.0, tone);
        }
    }
    let size = BODY_S;
    let pad = tokens::VIEWFINDER_BAND_PAD;
    let line_h = ui.line_as(size);
    let parts = sc.parts_read();
    let bar = parts.map_or(0.0, |_| pad + tokens::PROGRESS_H);
    let band_h = pad + line_h + bar + pad;
    let band_y = y + h - band_h;
    ui.fill(
        x,
        band_y,
        w,
        band_h,
        0.0,
        VIEWFINDER_BAND.with_alpha(tokens::VIEWFINDER_BAND_ALPHA),
    );
    let line = ui.fit_as(size, sc.state_line(), w - 2.0 * pad);
    let lw = ui.measure_as(size, &line);
    ui.text_as(x + (w - lw) / 2.0, band_y + pad, size, VIEWFINDER, &line);
    if let Some(share) = parts {
        let (bx, by, bw) = (x + pad, band_y + pad + line_h + pad, w - 2.0 * pad);
        ui.progress((bx, by, bw), Some(share), None);
    }
}
