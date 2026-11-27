//! Units and geometry: density-independent pixels, the DPI scale, size
//! classes, and integer pixel rectangles.
//!
//! Everything an application declares is in dp; everything the layout engine
//! produces and the canvas draws is in whole pixels. One [`Scale`] per
//! session converts between them.

use faraday_port::values::Display;
pub use faraday_port::values::Point;
pub use faraday_tokens::dims::{SizeClass, Window, WindowClass};

/// A length in density-independent pixels: 1 dp is 1/160 inch.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Default)]
pub struct Dp(pub f32);

/// Converts dp to pixels for one display, and carries the size class's
/// type scale (`tokens::type_scale_pct`).
///
/// Lengths are unaffected by the type scale: a 48 dp touch target is 48 dp
/// on every class. Only [`crate::text::Font`] applies it, so the ramp
/// grows with the class without a screen asking for a larger size.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Scale {
    /// Physical dots per inch of the display.
    pub dpi: u16,
    /// The type ramp's scale, in percent.
    type_pct: u16,
    /// Whether text takes the class's narrow cut of the text faces
    /// (`tokens::narrow_text`).
    narrow: bool,
}

impl Scale {
    /// The dp-equals-px scale (160 dpi) at the `Small` type scale. Used
    /// by tests.
    pub const IDENTITY: Scale = Scale {
        dpi: 160,
        type_pct: 100,
        narrow: false,
    };

    /// Scale for a display at the `Small` type scale, in the text
    /// faces' ordinary width: the scale before a class is known.
    pub fn new(dpi: u16) -> Self {
        Scale {
            dpi: dpi.max(1),
            type_pct: 100,
            narrow: false,
        }
    }

    /// Scale for a display of `class`, whose type ramp is scaled with it
    /// and whose text faces are the class's width.
    pub fn for_class(dpi: u16, class: SizeClass) -> Self {
        Scale {
            dpi: dpi.max(1),
            type_pct: faraday_tokens::dims::type_scale_pct(class),
            narrow: faraday_tokens::dims::narrow_text(class),
        }
    }

    /// Whether text on this scale takes the narrow cut of the text
    /// faces.
    pub fn narrow(self) -> bool {
        self.narrow
    }

    /// Pixels per dp of type, which is [`Scale::factor`] times the
    /// class's type scale.
    pub fn type_factor(self) -> f32 {
        self.factor() * f32::from(self.type_pct) / 100.0
    }

    /// A type size in dp as fractional pixels.
    pub(crate) fn type_px_f(self, dp: Dp) -> f32 {
        dp.0 * self.type_factor()
    }

    /// Pixels per dp.
    pub fn factor(self) -> f32 {
        f32::from(self.dpi) / 160.0
    }

    /// Converts a dp length to whole pixels, rounding to nearest.
    pub fn px(self, dp: Dp) -> i32 {
        round(dp.0 * self.factor())
    }

    /// Converts a dp length to fractional pixels.
    pub(crate) fn px_f(self, dp: Dp) -> f32 {
        dp.0 * self.factor()
    }
}

fn round(v: f32) -> i32 {
    // `f32::round` is not available without std; libm-free rounding to
    // nearest with ties away from zero is enough for pixel snapping.
    if v >= 0.0 {
        (v + 0.5) as i32
    } else {
        (v - 0.5) as i32
    }
}

/// A size in whole pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Size {
    /// Width in pixels.
    pub w: i32,
    /// Height in pixels.
    pub h: i32,
}

impl Size {
    /// A size.
    pub const fn new(w: i32, h: i32) -> Self {
        Size { w, h }
    }

    /// Zero size.
    pub const ZERO: Size = Size { w: 0, h: 0 };

    /// Component-wise minimum.
    pub fn min(self, other: Size) -> Size {
        Size::new(self.w.min(other.w), self.h.min(other.h))
    }

    /// Component-wise maximum.
    pub fn max(self, other: Size) -> Size {
        Size::new(self.w.max(other.w), self.h.max(other.h))
    }
}

/// An axis-aligned rectangle in whole pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Rect {
    /// Left edge.
    pub x: i32,
    /// Top edge.
    pub y: i32,
    /// Width.
    pub w: i32,
    /// Height.
    pub h: i32,
}

impl Rect {
    /// A rectangle from position and size.
    pub const fn new(x: i32, y: i32, w: i32, h: i32) -> Self {
        Rect { x, y, w, h }
    }

    /// Right edge (exclusive).
    pub fn right(&self) -> i32 {
        self.x + self.w
    }

    /// Bottom edge (exclusive).
    pub fn bottom(&self) -> i32 {
        self.y + self.h
    }

    /// Size.
    pub fn size(&self) -> Size {
        Size::new(self.w, self.h)
    }

    /// Centre point.
    pub fn center(&self) -> Point {
        Point {
            x: self.x + self.w / 2,
            y: self.y + self.h / 2,
        }
    }

    /// Whether the rectangle has positive area.
    pub fn is_empty(&self) -> bool {
        self.w <= 0 || self.h <= 0
    }

    /// Whether `(x, y)` lies inside.
    pub fn contains(&self, x: i32, y: i32) -> bool {
        x >= self.x && y >= self.y && x < self.right() && y < self.bottom()
    }

    /// Intersection; empty (zero-size) when the rectangles do not overlap.
    pub fn intersect(&self, other: &Rect) -> Rect {
        let x = self.x.max(other.x);
        let y = self.y.max(other.y);
        let r = self.right().min(other.right());
        let b = self.bottom().min(other.bottom());
        Rect::new(x, y, (r - x).max(0), (b - y).max(0))
    }

    /// Shrinks by `edges` (in pixels).
    pub fn inset(&self, edges: PxEdges) -> Rect {
        Rect::new(
            self.x + edges.left,
            self.y + edges.top,
            (self.w - edges.left - edges.right).max(0),
            (self.h - edges.top - edges.bottom).max(0),
        )
    }

    /// Translates by `(dx, dy)`.
    pub fn offset(&self, dx: i32, dy: i32) -> Rect {
        Rect::new(self.x + dx, self.y + dy, self.w, self.h)
    }
}

/// Four edge lengths in dp.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Edges {
    /// Left.
    pub left: f32,
    /// Top.
    pub top: f32,
    /// Right.
    pub right: f32,
    /// Bottom.
    pub bottom: f32,
}

impl Edges {
    /// The same length on all four sides.
    pub const fn all(dp: f32) -> Self {
        Edges {
            left: dp,
            top: dp,
            right: dp,
            bottom: dp,
        }
    }
}

/// Four edge lengths in pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PxEdges {
    /// Left.
    pub left: i32,
    /// Top.
    pub top: i32,
    /// Right.
    pub right: i32,
    /// Bottom.
    pub bottom: i32,
}

/// The size class of a display, by its physical diagonal.
pub fn size_class_of(display: &Display) -> SizeClass {
    SizeClass::of_px(display.width, display.height, display.dpi)
}

/// The window a display is at its own density, with text `text_factor` times its standard size.
pub fn window_of(display: &Display, text_factor: f32) -> Window {
    Window::of_px(display.width, display.height, display.dpi, text_factor)
}
