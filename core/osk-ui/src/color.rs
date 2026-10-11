//! Colours and the theme.

/// A straight (non-premultiplied) RGBA colour.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color {
    /// Red.
    pub r: u8,
    /// Green.
    pub g: u8,
    /// Blue.
    pub b: u8,
    /// Alpha; 255 is opaque.
    pub a: u8,
}

impl Color {
    /// An opaque colour.
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Color { r, g, b, a: 255 }
    }

    /// A colour with alpha.
    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Color { r, g, b, a }
    }

    /// Fully transparent.
    pub const TRANSPARENT: Color = Color::rgba(0, 0, 0, 0);
    /// White.
    pub const WHITE: Color = Color::rgb(255, 255, 255);
    /// Black.
    pub const BLACK: Color = Color::rgb(0, 0, 0);

    /// The same colour with a different alpha.
    pub const fn with_alpha(self, a: u8) -> Self {
        Color { a, ..self }
    }

    /// Linear blend toward `other` by `t` in `0..=1`.
    pub fn mix(self, other: Color, t: f32) -> Color {
        let t = t.clamp(0.0, 1.0);
        let lerp = |a: u8, b: u8| (f32::from(a) + (f32::from(b) - f32::from(a)) * t + 0.5) as u8;
        Color {
            r: lerp(self.r, other.r),
            g: lerp(self.g, other.g),
            b: lerp(self.b, other.b),
            a: lerp(self.a, other.a),
        }
    }

    pub(crate) fn to_skia(self) -> tiny_skia::Color {
        tiny_skia::Color::from_rgba8(self.r, self.g, self.b, self.a)
    }
}

/// The colour roles every widget draws with. Dark by default; a light theme
/// is another set of values for the same roles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Theme {
    /// Screen background.
    pub background: Color,
    /// Cards, keys, list rows.
    pub surface: Color,
    /// A surface one step lighter: pressed rows, keys and tiles.
    pub surface_raised: Color,
    /// Borders and dividers.
    pub outline: Color,
    /// Primary accent: primary buttons, selection, progress.
    pub primary: Color,
    /// Text drawn on top of `primary`.
    pub on_primary: Color,
    /// Body text.
    pub text: Color,
    /// Secondary text, labels, disabled items.
    pub muted: Color,
    /// Danger: destructive actions, danger warnings.
    pub danger: Color,
    /// Caution warnings.
    pub caution: Color,
    /// Info warnings and neutral badges.
    pub info: Color,
    /// Success states (check marks, "nonce OK").
    pub success: Color,
    /// Background of the secret frame.
    pub secret_background: Color,
    /// Border of the secret frame.
    pub secret_border: Color,
}

impl Theme {
    /// The default dark theme (UX.md §6): black ground, one accent, and
    /// the four status colours.
    pub const DARK: Theme = Theme {
        background: Color::rgb(0x00, 0x00, 0x00),
        surface: Color::rgb(0x1c, 0x1c, 0x1e),
        surface_raised: Color::rgb(0x3a, 0x3a, 0x3c),
        outline: Color::rgb(0x2c, 0x2c, 0x2e),
        primary: Color::rgb(0xff, 0x9f, 0x0a),
        on_primary: Color::rgb(0x00, 0x00, 0x00),
        text: Color::rgb(0xfc, 0xfc, 0xfc),
        muted: Color::rgb(0x98, 0x98, 0x9e),
        danger: Color::rgb(0xff, 0x45, 0x3a),
        caution: Color::rgb(0xff, 0xd6, 0x0a),
        info: Color::rgb(0x40, 0x9c, 0xff),
        success: Color::rgb(0x30, 0xd1, 0x58),
        secret_background: Color::rgb(0x14, 0x0d, 0x00),
        secret_border: Color::rgb(0xff, 0x9f, 0x0a),
    };
}

impl Default for Theme {
    fn default() -> Self {
        Theme::DARK
    }
}
