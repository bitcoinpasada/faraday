//! The snackbar organism (09 §3.3): Material's snackbar, an inverse-surface bar with one line of
//! text, standing above the foot of the page. A note's time on show is the runtime's (04 §10):
//! it puts one up for `TOAST_SHOW_MS`; the bar rises in on the store's `Motion::Shown` under the
//! note's id and fades out over the last `MOTION_SHORT4` of the time ([`draw_timed`]). Its
//! action is Material's close affordance at its end, a `TOUCH` square whose press the page
//! gives.

use faraday_atoms::hits::Shape;
use faraday_atoms::{Anchor, Pressed, Shown, Ui, W};
use faraday_draw::ease::ease;
use faraday_tokens::dims::{
    BODY_M, GRID, ICON_BUTTON, ICON_GLYPH, MARGIN_COMPACT, SHAPE_XS, TOUCH, shape_full,
};
use faraday_tokens::elevation::ELEVATION_MENU;
use faraday_tokens::motion::{EASE_STANDARD_ACCELERATE, EASE_STANDARD_DECELERATE, MOTION_SHORT4};
use faraday_tokens::palette::Role;

/// The note on show: its words.
pub struct SnackbarModel<'s> {
    pub text: Shown<'s>,
}

/// The snackbar's height: one line and a margin over and under it.
pub fn height<A: Pressed>(ui: &Ui<'_, A>) -> f32 {
    ui.line_as(BODY_M) + 4.0 * GRID
}

/// Draws the snackbar standing on `foot`, the page's foot, in a window `w` wide.
pub fn draw_at<A: Pressed>(m: &SnackbarModel<'_>, ui: &mut Ui<'_, A>, (w, foot): (f32, f32)) {
    let h = height(ui);
    let bw = w - 2.0 * MARGIN_COMPACT;
    let (x, y) = (MARGIN_COMPACT, foot - h - MARGIN_COMPACT);
    ui.shadow(x, y, bw, h, SHAPE_XS, ELEVATION_MENU);
    ui.fill(x, y, bw, h, SHAPE_XS, Role::InverseSurface);
    let room = bw - 4.0 * GRID;
    let text = ui.fit_as(BODY_M, m.text.as_str(), room);
    ui.showing(&m.text, |ui| {
        ui.text_mid_as(
            (x + 2.0 * GRID, y, h),
            BODY_M,
            Role::InverseOnSurface,
            &text,
        );
    });
}

/// How a note is on show: which note it is, how much of its time is left (the runtime counts
/// `TOAST_SHOW_MS`) and, for a bar with Material's close affordance, the press of its ×.
pub struct OnShow<A> {
    /// The note's id: its rise is the motion store's `Motion::Shown` under it.
    pub id: Anchor,
    /// Milliseconds of its time left.
    pub left_ms: u64,
    pub close: Option<A>,
}

/// How much of the bar shows, 0 to 1, and how far below its place it stands, in units: it rises
/// in on Material's decelerating curve (`came` is its eased progress in, 0 to 1) and fades on the
/// accelerating one over the last `MOTION_SHORT4` of its time; under Reduce motion it does not
/// rise.
pub fn on_show(came: f32, left_ms: u64, still: bool) -> (f32, f32) {
    let rise = ease(EASE_STANDARD_DECELERATE, came.clamp(0.0, 1.0));
    let left = (left_ms as f32 / MOTION_SHORT4 as f32).min(1.0);
    let fade = 1.0 - ease(EASE_STANDARD_ACCELERATE, 1.0 - left);
    let lift = if still { 0.0 } else { 10.0 * (1.0 - rise) };
    (rise.min(fade), lift)
}

/// [`draw_at`] for a note on show: faded in and out as [`on_show`] says, with the close
/// affordance where the page gave a press for it. Faded out, nothing is drawn.
pub fn draw_timed<A: Pressed>(
    m: &SnackbarModel<'_>,
    ui: &mut Ui<'_, A>,
    (w, foot): (f32, f32),
    on: OnShow<A>,
) {
    let came = ui.shown_fraction(on.id, MOTION_SHORT4, EASE_STANDARD_DECELERATE);
    // The store has eased the entry already; the curve here only shapes the fade out.
    let (shown, lift) = on_show(came, on.left_ms, ui.reduce_motion());
    if shown <= 0.0 {
        return;
    }
    bar(m, ui, (w, foot), (shown, lift), on.close);
}

/// A notice that stays until its × is pressed, over the foot of the window: fully shown at once,
/// no rise and no fade. Returns the room it takes above `foot`, where a second one's foot goes.
pub fn draw_held<A: Pressed>(
    m: &SnackbarModel<'_>,
    ui: &mut Ui<'_, A>,
    (w, foot): (f32, f32),
    close: A,
) -> f32 {
    bar(m, ui, (w, foot), (1.0, 0.0), Some(close));
    height(ui) + MARGIN_COMPACT
}

/// The bar faded in to `shown`, `lift` units below its place, with its close where one is given.
fn bar<A: Pressed>(
    m: &SnackbarModel<'_>,
    ui: &mut Ui<'_, A>,
    (w, foot): (f32, f32),
    (shown, lift): (f32, f32),
    close: Option<A>,
) {
    let h = height(ui);
    let side = if close.is_some() { TOUCH } else { 0.0 };
    let bw = w - 2.0 * MARGIN_COMPACT;
    let (x, y) = (MARGIN_COMPACT, foot - h - MARGIN_COMPACT + lift);
    // The shadow's ring reaches under the fill's edge: at a linear share of its strength it
    // shows through the thinning fill as a dark rim. To the sixth power it is gone before the
    // fill is.
    ui.shadow_faded((x, y, bw, h, SHAPE_XS), ELEVATION_MENU, shown.powi(6));
    ui.wash(x, y, bw, h, SHAPE_XS, Role::InverseSurface.at(shown));
    let room = bw - 4.0 * GRID - side;
    let text = ui.fit_as(BODY_M, m.text.as_str(), room);
    ui.showing(&m.text, |ui| {
        ui.text_mid_as(
            (x + 2.0 * GRID, y, h),
            BODY_M,
            Role::InverseOnSurface.at(shown),
            &text,
        );
    });
    if let Some(close) = close {
        // Material's standard icon button, centred on the end of the bar in a touch target; the
        // × is the multiplication sign.
        let (bx, by) = (x + bw - TOUCH, y + (h - TOUCH) / 2.0);
        let edge = (TOUCH - ICON_BUTTON) / 2.0;
        let ink = Role::InverseOnSurface.at(shown);
        let shape = Shape::new(
            bx + edge,
            by + edge,
            ICON_BUTTON,
            ICON_BUTTON,
            shape_full(ICON_BUTTON),
        );
        ui.press_to(shape, (bx, by, TOUCH, TOUCH), ink, close);
        let mark = "\u{d7}";
        let mw = ui.measure(ICON_GLYPH, W::R, mark);
        ui.text_mid(
            bx + edge + (ICON_BUTTON - mw) / 2.0,
            by + edge,
            ICON_BUTTON,
            ICON_GLYPH,
            W::R,
            ink,
            mark,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use faraday_atoms::ui::testing::{P, at_rest, canvas, frame, frame_in};

    /// Invariant: A note on show draws its words in the bar standing above the foot, and takes no press.
    /// Regression: The note drawn off the window or as a control that swallows a press.
    /// Owner: kit
    #[test]
    fn a_note_stands_above_the_foot_and_takes_no_press() {
        let mut c = canvas(160);
        let out = frame(&mut c, &at_rest(None), |ui| {
            let m = SnackbarModel {
                text: Shown::plain("Saved"),
            };
            draw_at(&m, ui, (640.0, 480.0));
        });
        assert!(out.hits.is_empty());
    }

    /// Invariant: A note rises in to full, holds, and fades to nothing as its time runs out; under Reduce motion it does not rise, and its close button takes the press it was given.
    /// Regression: A note that stays half shown at the end of its time, or a close button that swallows its press.
    /// Owner: kit
    #[test]
    fn a_note_fades_with_its_time_and_its_close_takes_the_press() {
        let (full, lift) = on_show(1.0, 4000, false);
        assert_eq!((full, lift), (1.0, 0.0));
        let (half, _) = on_show(1.0, 100, false);
        assert!(half > 0.0 && half < 1.0);
        assert_eq!(on_show(1.0, 0, false).0, 0.0);
        let (_, still) = on_show(0.0, 4000, true);
        assert_eq!(still, 0.0);
        let mut c = canvas(160);
        let mut store = faraday_atoms::MotionStore::new();
        let mut out = None;
        // The first frame is the note's first, at the start of its rise; the second is after it.
        for now in [1000, 1400] {
            out = Some(frame_in(&mut c, &at_rest(None), (&mut store, now), |ui| {
                let m = SnackbarModel {
                    text: Shown::plain("Saved"),
                };
                let on = OnShow {
                    id: Anchor {
                        scope: "note",
                        n: 1,
                    },
                    left_ms: 4000,
                    close: Some(P(7)),
                };
                draw_timed(&m, ui, (640.0, 480.0), on);
            }));
        }
        let out = out.unwrap();
        let pressed: Vec<u8> = out.hits.iter().map(|h| h.press.0).collect();
        assert_eq!(pressed, vec![7]);
    }
}
