//! The viewfinder organism (09 §3.3): the scanner's camera picture and what is drawn over it,
//! the found code's outline, the brackets, the state line and the parts bar. One drawing for the
//! scanner sheet and the pane that holds it beside a card; both read the same scan state, so
//! there is one camera.

use faraday_atoms::{Pressed, Shown, Ui};
use faraday_draw::geom::Rect;
use faraday_tokens::dims::{
    BODY_S, PROGRESS_H, VIEWFINDER_BAND_ALPHA, VIEWFINDER_BAND_PAD, VIEWFINDER_BRACKET_INSET,
    VIEWFINDER_BRACKET_SHARE, VIEWFINDER_BRACKET_STROKE, VIEWFINDER_OUTLINE_EDGE,
    VIEWFINDER_OUTLINE_STROKE,
};
use faraday_tokens::palette::{Role, VIEWFINDER, VIEWFINDER_BAND, VIEWFINDER_READ};

/// What a scan pass saw of a code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Seen {
    /// The frame's size.
    pub width: u16,
    /// The frame's height.
    pub height: u16,
    /// The code's corners in the frame.
    pub corners: [(u16, u16); 4],
    /// It read.
    pub read: bool,
}

/// The camera's picture: its size, its luma and, where the camera gives one, its NV12 chroma.
pub struct Picture<'s> {
    pub width: u16,
    pub height: u16,
    pub luma: &'s [u8],
    pub chroma: Option<&'s [u8]>,
}

/// The viewfinder to draw. Nothing is kept: the picture is the scan state's, drawn from it each
/// frame.
pub struct ViewfinderModel<'s, A> {
    /// The picture, or none before the first frame.
    pub picture: Option<Picture<'s>>,
    /// The code the scanner last saw, given only while it was seen within the last ten preview
    /// frames: gone after that.
    pub seen: Option<Seen>,
    /// What the scan says it is doing.
    pub state: Shown<'s>,
    /// The share of a multi-part code read, where it is read in parts.
    pub parts: Option<f32>,
    press: core::marker::PhantomData<A>,
}

impl<'s, A> ViewfinderModel<'s, A> {
    /// A viewfinder with a state line.
    pub fn new(state: Shown<'s>) -> Self {
        Self {
            picture: None,
            seen: None,
            state,
            parts: None,
            press: core::marker::PhantomData,
        }
    }
}

/// Draws the frame in the `(x, y, w, h)` box, on the surface the caller filled: the camera's
/// picture with the found code's outline while a picture is drawn, then the brackets, the state
/// line and the parts bar. A QR found and read and still in view turns the brackets green.
pub fn draw<A: Pressed>(
    m: &ViewfinderModel<'_, A>,
    ui: &mut Ui<'_, A>,
    (x, y, w, h): (f32, f32, f32, f32),
) {
    let rect = ui.rect(x, y, w, h);
    let mut read_in_view = false;
    if let Some(p) = &m.picture {
        ui.c.luma(rect, p.width, p.height, p.luma, p.chroma);
        if let Some(seen) = &m.seen {
            read_in_view = seen.read;
            found_outline(ui, rect, (p.width, p.height), seen);
        }
    }
    over(m, ui, (x, y, w, h), read_in_view);
}

/// The code the scanner last found, outlined where it is in the picture drawn in `rect` from a
/// `frame`-sized preview: ok once read, warning while found and not read, its corners rounded,
/// edged each side in the role's on-colour so the line reads over a light picture and a dark one.
fn found_outline<A: Pressed>(ui: &mut Ui<'_, A>, rect: Rect, (fw, fh): (u16, u16), seen: &Seen) {
    if seen.width == 0 || seen.height == 0 || fw == 0 || fh == 0 {
        return;
    }
    // The preview fills its box and crops the longer side, as `Canvas::luma` draws it: the same
    // scale and crop here. The scanner's frame may be larger than the preview (the shells reduce
    // the frame they draw), so its corners are taken to the preview's pixels first.
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
    let lw = VIEWFINDER_OUTLINE_STROKE * ui.f;
    let ew = lw + 2.0 * VIEWFINDER_OUTLINE_EDGE * ui.f;
    // Round the code, not over its modules: each corner moved out from the middle by half the
    // stroke across the corner's diagonal (half of `ew` times the square root of two).
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

/// What is drawn over the camera's picture: the corner brackets, white while looking and green
/// while a QR that read is in view, and at the bottom, on a translucent band, the state line and
/// the bar of parts read.
fn over<A: Pressed>(
    m: &ViewfinderModel<'_, A>,
    ui: &mut Ui<'_, A>,
    (x, y, w, h): (f32, f32, f32, f32),
    read: bool,
) {
    let tone = if read { VIEWFINDER_READ } else { VIEWFINDER };
    let arm = w.min(h) * VIEWFINDER_BRACKET_SHARE;
    let (st, inset) = (VIEWFINDER_BRACKET_STROKE, VIEWFINDER_BRACKET_INSET);
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
    let pad = VIEWFINDER_BAND_PAD;
    let line_h = ui.line_as(BODY_S);
    let bar = m.parts.map_or(0.0, |_| pad + PROGRESS_H);
    let band_h = pad + line_h + bar + pad;
    let band_y = y + h - band_h;
    ui.fill(
        x,
        band_y,
        w,
        band_h,
        0.0,
        VIEWFINDER_BAND.with_alpha(VIEWFINDER_BAND_ALPHA),
    );
    let line = ui.fit_as(BODY_S, m.state.as_str(), w - 2.0 * pad);
    let lw = ui.measure_as(BODY_S, &line);
    ui.text_as(x + (w - lw) / 2.0, band_y + pad, BODY_S, VIEWFINDER, &line);
    if let Some(share) = m.parts {
        let (bx, by, bw) = (x + pad, band_y + pad + line_h + pad, w - 2.0 * pad);
        ui.progress((bx, by, bw), Some(share), None);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use faraday_atoms::ui::testing::{P, at_rest, canvas, frame};

    /// Invariant: With no picture the viewfinder still draws its brackets and its state line, and a code seen with all four corners at the middle draws without leaving the box.
    /// Regression: A scan that shows nothing before the camera's first frame, or an outline drawn outside the picture.
    /// Owner: kit
    #[test]
    fn it_draws_with_and_without_a_picture() {
        let mut c = canvas(160);
        frame(&mut c, &at_rest(None), |ui| {
            let mut m = ViewfinderModel::<P>::new(Shown::plain("Looking for a code"));
            draw(&m, ui, (0.0, 0.0, 200.0, 150.0));
            let luma = vec![128u8; 64 * 48];
            m.picture = Some(Picture {
                width: 64,
                height: 48,
                luma: &luma,
                chroma: None,
            });
            m.seen = Some(Seen {
                width: 64,
                height: 48,
                corners: [(10, 10), (50, 10), (50, 40), (10, 40)],
                read: true,
            });
            m.parts = Some(0.5);
            draw(&m, ui, (0.0, 0.0, 200.0, 150.0));
        });
    }
}
