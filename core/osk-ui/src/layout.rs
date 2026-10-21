//! The layout context: the display facts that sizes depend on.

use crate::geom::{Dp, Scale, SizeClass};

/// Everything measurement needs to know about the display.
#[derive(Debug, Clone, Copy)]
pub struct LayoutCtx {
    /// dp scale.
    pub scale: Scale,
    /// Size class.
    pub class: SizeClass,
    /// The shell's top inset in dp (`DisplayInfo::inset_top`).
    pub inset_top_dp: f32,
    /// The shell's bottom inset in dp (`DisplayInfo::inset_bottom`):
    /// what a keyboard's bottom row and a bottom action stand clear of.
    pub inset_bottom_dp: f32,
}

impl LayoutCtx {
    /// A context with no insets. The scale takes the class's type ramp,
    /// so measurement and drawing always agree about how large a font
    /// is.
    pub fn new(scale: Scale, class: SizeClass) -> Self {
        LayoutCtx {
            scale: scale.with_class(class),
            class,
            inset_top_dp: 0.0,
            inset_bottom_dp: 0.0,
        }
    }

    /// The same context with the insets the shell reported, in dp.
    pub fn with_insets(self, top_dp: f32, bottom_dp: f32) -> Self {
        LayoutCtx {
            inset_top_dp: top_dp,
            inset_bottom_dp: bottom_dp,
            ..self
        }
    }

    /// dp to whole pixels.
    pub fn px(&self, dp: f32) -> i32 {
        self.scale.px(Dp(dp))
    }
}
