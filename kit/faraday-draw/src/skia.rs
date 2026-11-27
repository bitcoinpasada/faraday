//! Colours into tiny-skia's (09 §3.2).

use faraday_tokens::palette::Color;

/// A colour as tiny-skia holds it.
pub(crate) trait ToSkia {
    fn to_skia(self) -> tiny_skia::Color;
}

impl ToSkia for Color {
    fn to_skia(self) -> tiny_skia::Color {
        tiny_skia::Color::from_rgba8(self.r, self.g, self.b, self.a)
    }
}
