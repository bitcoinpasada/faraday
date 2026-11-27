//! The frame out (02 §5).

/// An axis-aligned rectangle in whole pixels.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

/// A band moved vertically since the last frame (a scroll).
// Spec: 02 §5 leaves this open (its fields).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Moved {
    pub band: Rect,
    pub dy: i16,
}

/// A place a press belongs to the window, not the app.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Region {
    MovesWindow(Rect),
}

/// How soon the next tick wants to come (02 §3.3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pace {
    Rest,
    Moving,
}

pub struct Frame<'a> {
    pub width: u16,
    pub height: u16,
    /// Premultiplied RGBA8888, row-major, width x height x 4 bytes; the
    /// adapter converts to its panel's format.
    pub rgba: &'a [u8],
    /// Rectangles that may have changed since the last frame.
    pub changed: &'a [Rect],
    /// A band moved by dy since the last frame (a scroll).
    pub moved: Option<Moved>,
    /// Places a press belongs to the window, not the app.
    pub regions: &'a [Region],
    pub pace: Pace,
}
