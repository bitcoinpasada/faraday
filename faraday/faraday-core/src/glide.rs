//! The arithmetic of a change of page: how far through its fade the
//! eased progress is, whether the old page is still going out, and where a
//! pane that moves between two places is (`docs/MOTION.md` §3.5). No
//! pixels are touched here.

use faraday_draw::Rect;

use crate::motion;
use crate::stick::settings_choice::MotionStyle;

/// The eased progress of a change of page `t` (0 to 1) of the way in:
/// Material's emphasized easing.
pub(crate) fn eased(t: f32) -> f32 {
    faraday_draw::easing::ease_path(&faraday_draw::tokens::EASE_EMPHASIZED, t)
}

/// A fade through, `e` the eased progress: whether the old page is still
/// going out to the surface, and how far, 0 to 1, in the half it is in.
pub(crate) fn through(e: f32) -> (bool, f32) {
    if e < motion::STEP_FADE_OUT {
        (true, e / motion::STEP_FADE_OUT)
    } else {
        (
            false,
            (e - motion::STEP_FADE_OUT) / (1.0 - motion::STEP_FADE_OUT),
        )
    }
}

/// `a` and `b` as numbers, `e` of the way from the first to the second.
fn lerp(a: i32, b: i32, e: f32) -> i32 {
    (a as f32 + (b - a) as f32 * e).round() as i32
}

/// The pane's place `e` of the way from `a` to `b`.
pub(crate) fn lerp_rect(a: Rect, b: Rect, e: f32) -> Rect {
    Rect::new(
        lerp(a.x, b.x, e),
        lerp(a.y, b.y, e),
        lerp(a.w, b.w, e),
        lerp(a.h, b.h, e),
    )
}

/// What a change of page does under the motion settings.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Kind {
    /// The two frames blend, eased.
    Crossfade,
    /// The old page goes out to the surface and the new one comes in.
    Through,
}

impl Kind {
    /// What `style` does, `None` for a change shown at once.
    pub(crate) fn of(style: MotionStyle) -> Option<Kind> {
        match style {
            MotionStyle::Smooth => Some(Kind::Through),
            MotionStyle::Crossfade => Some(Kind::Crossfade),
            MotionStyle::Off => None,
        }
    }
}
