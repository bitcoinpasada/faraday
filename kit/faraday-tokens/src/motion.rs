//! Material's durations and easing curves (09 §3.2).

/// Material's durations, ms: short 1 to 4.
pub const MOTION_SHORT1: u64 = 50;
/// Short 3: a switch's slide.
pub const MOTION_SHORT3: u64 = 150;
/// Short 4: a fade between screens, the toast coming in.
pub const MOTION_SHORT4: u64 = 200;
/// Medium 1.
pub const MOTION_MEDIUM1: u64 = 250;
/// Medium 2: a step card opening, which moves much of the page.
pub const MOTION_MEDIUM2: u64 = 300;
/// Medium 3: a change of theme, which changes every pixel.
pub const MOTION_MEDIUM3: u64 = 350;
/// Medium 4.
pub const MOTION_MEDIUM4: u64 = 400;
/// Long 4: the Relaxed speed, every page change and step inside a flow
/// by default (`docs/MOTION.md` §3.5).
pub const MOTION_LONG4: u64 = 600;
/// Extra-long 4.
pub const MOTION_EXTRA_LONG4: u64 = 1000;

/// An easing curve: a cubic Bézier's two control points, x1, y1, x2,
/// y2, from (0, 0) to (1, 1), as CSS writes one.
pub type Easing = [f32; 4];
/// Material's standard easing: what moves on screen and stays there.
pub const EASE_STANDARD: Easing = [0.2, 0.0, 0.0, 1.0];
/// Standard decelerate: what comes onto the screen.
pub const EASE_STANDARD_DECELERATE: Easing = [0.0, 0.0, 0.0, 1.0];
/// Standard accelerate: what leaves the screen.
pub const EASE_STANDARD_ACCELERATE: Easing = [0.3, 0.0, 1.0, 1.0];
/// Emphasized decelerate: a large part of the page coming in, a step
/// card opening.
pub const EASE_EMPHASIZED_DECELERATE: Easing = [0.05, 0.7, 0.1, 1.0];
/// Emphasized accelerate: a large part of the page leaving.
pub const EASE_EMPHASIZED_ACCELERATE: Easing = [0.3, 0.0, 0.8, 0.15];

/// An easing path: two cubic Béziers end to end, the first from (0, 0)
/// to `join`, the second from `join` to (1, 1); each holds its two
/// control points as x1, y1, x2, y2.
pub struct EasingPath {
    /// Where the first segment ends and the second begins.
    pub join: [f32; 2],
    /// The control points of the segment from (0, 0) to `join`.
    pub first: [f32; 4],
    /// The control points of the segment from `join` to (1, 1).
    pub second: [f32; 4],
}
/// Material 3's emphasized easing (`m3_sys_motion_easing_emphasized` in
/// material-components-android's motion tokens): `path(M 0,0 C 0.05,0, 0.133333,0.06,
/// 0.166666,0.4 C 0.208333,0.82, 0.25,1, 1,1)`.
pub const EASE_EMPHASIZED: EasingPath = EasingPath {
    join: [0.166_666, 0.4],
    first: [0.05, 0.0, 0.133_333, 0.06],
    second: [0.208_333, 0.82, 0.25, 1.0],
};
