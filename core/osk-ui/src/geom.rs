//! Units and geometry: density-independent pixels, the DPI scale, size
//! classes, and integer pixel rectangles.
//!
//! Everything an application declares is in dp; everything the layout engine
//! produces and the canvas draws is in whole pixels. One [`Scale`] per
//! session converts between them (`docs/PLANNING.md` §4.3).

use osk_shell_api::DisplayInfo;

/// A length in density-independent pixels: 1 dp is 1/160 inch.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Default)]
pub struct Dp(pub f32);

/// Converts dp to pixels for one display, and carries the size class's
/// type scale (`widgets::tokens::type_scale_pct`).
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
    /// (`widgets::tokens::narrow_text`).
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
            type_pct: crate::widgets::tokens::type_scale_pct(class),
            narrow: crate::widgets::tokens::narrow_text(class),
        }
    }

    /// Whether text on this scale takes the narrow cut of the text
    /// faces.
    pub fn narrow(self) -> bool {
        self.narrow
    }

    /// The same scale at `class`'s type ramp.
    pub fn with_class(self, class: SizeClass) -> Self {
        Scale::for_class(self.dpi, class)
    }

    /// Pixels per dp of type, which is [`Scale::factor`] times the
    /// class's type scale.
    pub fn type_factor(self) -> f32 {
        self.factor() * f32::from(self.type_pct) / 100.0
    }

    /// A type size in dp as fractional pixels.
    pub fn type_px_f(self, dp: Dp) -> f32 {
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
    pub fn px_f(self, dp: Dp) -> f32 {
        dp.0 * self.factor()
    }

    /// Converts a dp length to whole pixels (alias for [`Scale::px`]).
    pub fn dp_to_px(self, dp: Dp) -> i32 {
        self.px(dp)
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

/// Screen size class, derived from physical size (UX.md §2).
///
/// Size classes swap component variants (2- vs 3-column hub, compact vs
/// full keyboard); they never reorder steps or hide information.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SizeClass {
    /// Under 3.5 inch diagonal: the 2.8" reference panel. Single column,
    /// two-column hub grid.
    Small,
    /// 3.5 to 7 inch: phones. Single column, hub fits one screen.
    Mobile,
    /// 7 inch and up: tablets and the desktop window. Sidebar plus
    /// content.
    Wide,
}

impl SizeClass {
    /// Classifies a display by its physical diagonal.
    pub fn of(display: &DisplayInfo) -> Self {
        let w = f32::from(display.width);
        let h = f32::from(display.height);
        let diagonal_px = sqrt(w * w + h * h);
        let inches = diagonal_px / f32::from(display.dpi.max(1));
        if inches < 3.5 {
            SizeClass::Small
        } else if inches < 7.0 {
            SizeClass::Mobile
        } else {
            SizeClass::Wide
        }
    }
}

/// What a screen knows about the space it is drawn in: how big that
/// space is in dp, and which class it belongs to.
///
/// The class swaps component variants; the two lengths let a composite
/// branch on the room it actually has, so a 411 x 891 dp phone and a
/// 411 x 600 dp phone can lay out differently (UX review 2026-09-07,
/// §2b.3). Organisms take `&Metrics` wherever they used to take a bare
/// [`SizeClass`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Metrics {
    /// Width in dp of the area the screen is drawn in.
    pub width_dp: f32,
    /// Height in dp of that area.
    pub height_dp: f32,
    /// The display's size class.
    pub class: SizeClass,
    /// The unusable strip at the top of the display in dp, as the shell
    /// reported it in [`DisplayInfo::inset_top`]. The app bar starts
    /// below it.
    pub inset_top_dp: f32,
    /// The unusable strip at the bottom of the display in dp, as the
    /// shell reported it in [`DisplayInfo::inset_bottom`]. What is
    /// anchored to the bottom stands this far clear of the edge.
    pub inset_bottom_dp: f32,
}

impl Metrics {
    /// Metrics for an area of `width_dp` by `height_dp` on `class`, with
    /// no insets.
    pub fn new(width_dp: f32, height_dp: f32, class: SizeClass) -> Self {
        Metrics {
            width_dp,
            height_dp,
            class,
            inset_top_dp: 0.0,
            inset_bottom_dp: 0.0,
        }
    }

    /// The same metrics with the shell's insets, in dp.
    pub fn with_insets(self, top_dp: f32, bottom_dp: f32) -> Self {
        Metrics {
            inset_top_dp: top_dp,
            inset_bottom_dp: bottom_dp,
            ..self
        }
    }

    /// Metrics for a whole display, insets included.
    pub fn of(display: &DisplayInfo) -> Self {
        let scale = Scale::new(display.dpi);
        let factor = scale.factor();
        Metrics::new(
            f32::from(display.width) / factor,
            f32::from(display.height) / factor,
            SizeClass::of(display),
        )
        .with_insets(
            f32::from(display.inset_top) / factor,
            f32::from(display.inset_bottom) / factor,
        )
    }

    /// The same class over a smaller area: a pane beside a sidebar, a
    /// capped wizard column.
    pub fn area(self, width_dp: f32, height_dp: f32) -> Self {
        Metrics {
            width_dp,
            height_dp,
            ..self
        }
    }

    /// Whether this is the `Wide` class.
    pub fn is_wide(self) -> bool {
        self.class == SizeClass::Wide
    }

    /// Whether this is the `Small` class.
    pub fn is_small(self) -> bool {
        self.class == SizeClass::Small
    }
}

/// Square root without std (Newton's method; plenty for a diagonal).
fn sqrt(v: f32) -> f32 {
    if v <= 0.0 {
        return 0.0;
    }
    let mut x = v;
    for _ in 0..30 {
        x = 0.5 * (x + v / x);
    }
    x
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

/// A point in whole pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Point {
    /// Horizontal position.
    pub x: i32,
    /// Vertical position.
    pub y: i32,
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

    /// Horizontal and vertical lengths.
    pub const fn symmetric(horizontal: f32, vertical: f32) -> Self {
        Edges {
            left: horizontal,
            top: vertical,
            right: horizontal,
            bottom: vertical,
        }
    }

    /// Converts to pixels.
    pub fn to_px(self, scale: Scale) -> PxEdges {
        PxEdges {
            left: scale.px(Dp(self.left)),
            top: scale.px(Dp(self.top)),
            right: scale.px(Dp(self.right)),
            bottom: scale.px(Dp(self.bottom)),
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

impl PxEdges {
    /// Left plus right.
    pub fn horizontal(&self) -> i32 {
        self.left + self.right
    }

    /// Top plus bottom.
    pub fn vertical(&self) -> i32 {
        self.top + self.bottom
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn display(width: u16, height: u16, dpi: u16) -> DisplayInfo {
        DisplayInfo {
            width,
            height,
            dpi,
            inset_bottom: 0,
            inset_top: 0,
            buttons: 0,
            camera_fixed: true,
            secure: osk_shell_api::SecureHardware::None,
            boot: osk_shell_api::BootState::Unknown,
            memory_mib: None,
        }
    }

    #[test]
    fn size_classes_of_reference_displays() {
        assert_eq!(SizeClass::of(&display(320, 240, 143)), SizeClass::Small);
        assert_eq!(SizeClass::of(&display(640, 480, 286)), SizeClass::Small);
        assert_eq!(SizeClass::of(&display(1080, 2340, 420)), SizeClass::Mobile);
        // The desktop window: 960x640 at 160 dpi is 7.2 inches.
        assert_eq!(SizeClass::of(&display(960, 640, 160)), SizeClass::Wide);
    }
}
