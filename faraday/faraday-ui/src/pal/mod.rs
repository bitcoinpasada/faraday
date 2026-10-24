//! Faraday's colours (`docs/THEMES.md`): Material 3's colour roles, each
//! scheme's values generated from its theme's seed ([`schemes`]), and
//! the paints a drawing call takes. A paint is a role at an opacity,
//! looked up in the scheme the frame is drawn in, or a colour drawn as
//! it is in every theme (a QR code, a camera's overlay, a stick's pill).
//!
//! A screen names the role it draws in (`docs/THEMES.md` §6, §7); what
//! is here beside the roles is what no role stands for: the codex32
//! worksheets' paints, a camera's overlay and the sticks' pills.

use faraday_draw::Color;

mod schemes;

pub use schemes::{ROLE_COUNT, Role, ThemeName};

/// One scheme's colours, in [`Role`]'s order.
pub type Scheme = [Color; ROLE_COUNT];

/// Every scheme, as [`Theme::scheme`](crate::Theme::scheme) finds them:
/// the generated table's hex words made colours once, when compiled.
pub(crate) static SCHEMES: [Scheme; schemes::SCHEMES.len()] = {
    let mut out = [[Color::BLACK; ROLE_COUNT]; schemes::SCHEMES.len()];
    let mut s = 0;
    while s < out.len() {
        let mut r = 0;
        while r < ROLE_COUNT {
            let w = schemes::SCHEMES[s][r];
            out[s][r] = Color::rgb((w >> 16) as u8, (w >> 8) as u8, w as u8);
            r += 1;
        }
        s += 1;
    }
    out
};

/// What a drawing call fills, strokes or writes in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Paint {
    /// A role of the scheme the frame is drawn in, at an opacity (of
    /// 255).
    Role(Role, u8),
    /// A colour drawn as it is in every theme.
    Fixed(Color),
}

impl Paint {
    /// `role` at full strength.
    pub const fn role(role: Role) -> Paint {
        Paint::Role(role, 255)
    }

    /// The same paint at opacity `a` (of 255).
    pub const fn with_alpha(self, a: u8) -> Paint {
        match self {
            Paint::Role(r, _) => Paint::Role(r, a),
            Paint::Fixed(c) => Paint::Fixed(c.with_alpha(a)),
        }
    }

    /// Its opacity, of 255.
    pub const fn alpha(self) -> u8 {
        match self {
            Paint::Role(_, a) => a,
            Paint::Fixed(c) => c.a,
        }
    }

    /// Whether it is one of the scheme's roles, rather than a colour
    /// drawn as it is in every theme.
    pub const fn is_role(self) -> bool {
        matches!(self, Paint::Role(..))
    }

    /// The colour it is in `scheme`: a lookup.
    pub fn in_scheme(self, scheme: &Scheme) -> Color {
        match self {
            Paint::Role(r, a) => scheme[r as usize].with_alpha(a),
            Paint::Fixed(c) => c,
        }
    }
}

impl From<Color> for Paint {
    fn from(c: Color) -> Paint {
        Paint::Fixed(c)
    }
}

impl Role {
    /// This role at full strength, as a paint.
    pub const fn paint(self) -> Paint {
        Paint::role(self)
    }

    /// This role at an opacity (of 255): a state layer, a disabled
    /// control, a tint.
    pub const fn with_alpha(self, a: u8) -> Paint {
        Paint::Role(self, a)
    }

    /// This role at a state or disabled opacity from
    /// `faraday_draw::tokens` (0 to 1).
    pub fn at(self, opacity: f32) -> Paint {
        Paint::Role(self, faraday_draw::tokens::alpha_of(opacity))
    }
}

impl From<Role> for Paint {
    fn from(r: Role) -> Paint {
        Paint::role(r)
    }
}

/// The fill of a chosen row, card, option or the row of the place on
/// screen (Material's selected state, `docs/THEMES.md` §7):
/// secondary-container when `on`, `otherwise` when not.
pub fn selected_or(on: bool, otherwise: impl Into<Paint>) -> Paint {
    if on {
        Paint::role(Role::SecondaryContainer)
    } else {
        otherwise.into()
    }
}

/// What is written or drawn on a [`selected_or`] fill: everything on it
/// in on-secondary-container when `on`, `content` as it is when not.
pub fn on_selected(on: bool, content: impl Into<Paint>) -> Paint {
    if on {
        Paint::role(Role::OnSecondaryContainer)
    } else {
        content.into()
    }
}

/// Over a camera's picture (`docs/QR.md` §8 item 4), whatever the
/// theme: the picture is the same in every theme. The corner brackets
/// while looking, and the words on the state line's band.
pub const VIEWFINDER: Color = Color::rgb(0xff, 0xff, 0xff);
/// The corner brackets while a QR that read is in view.
pub const VIEWFINDER_READ: Color = Color::rgb(0x4c, 0xd9, 0x7b);
/// The band under the state line, drawn at
/// `tokens::VIEWFINDER_BAND_ALPHA`.
pub const VIEWFINDER_BAND: Color = Color::rgb(0x05, 0x08, 0x0b);

// The codex32 worksheets (`docs/CODEX32.md` §7.5).
/// The checksum squares, worked from the bottom up: the booklet's pink,
/// Faraday's checksum custom colour (`docs/THEMES.md` §5).
pub const CODEX32_CHECKSUM_FILL: Paint = Paint::role(Role::ChecksumContainer);
/// The characters, box numbers and letters on a checksum square.
pub const CODEX32_ON_CHECKSUM: Paint = Paint::role(Role::OnChecksumContainer);
/// The rows read from a table (sums and lookups).
pub const CODEX32_LOOKUP_FILL: Paint = Paint::role(Role::SurfaceContainerHighest);

/// The boot stick's pill and render: Bitcoin's orange, on a dark page.
pub const STICK_BOOT: Color = Color::rgb(0xf7, 0x93, 0x1a);
/// The backup sticks' pills and renders on a dark page, Stick 1, 2,
/// 3 and on in turn, then again from the first: red, yellow, green,
/// cyan, blue, violet, magenta. Orange is the boot stick's alone.
pub const STICKS: [Color; 7] = [
    Color::rgb(0xef, 0x7a, 0x7a),
    Color::rgb(0xe9, 0xd8, 0x6b),
    Color::rgb(0x8b, 0xd4, 0x7a),
    Color::rgb(0x6f, 0xd6, 0xe0),
    Color::rgb(0x7a, 0xa2, 0xf0),
    Color::rgb(0xa9, 0x8b, 0xf0),
    Color::rgb(0xe3, 0x7d, 0xd0),
];
/// [`STICK_BOOT`] on a light page, darkened to read there as text.
pub const STICK_BOOT_LIGHT: Color = Color::rgb(0xc2, 0x5e, 0x00);
/// [`STICKS`] on a light page, darkened to read there as text.
pub const STICKS_LIGHT: [Color; 7] = [
    Color::rgb(0xb8, 0x2f, 0x2f),
    Color::rgb(0x8a, 0x6d, 0x00),
    Color::rgb(0x2a, 0x7d, 0x2a),
    Color::rgb(0x00, 0x7a, 0x8a),
    Color::rgb(0x2c, 0x5f, 0xc9),
    Color::rgb(0x6a, 0x45, 0xc8),
    Color::rgb(0xb0, 0x2f, 0x98),
];

/// The colour of a stick's pill: `n` 0 the boot stick, `n` the
/// backup Stick `n` otherwise, on a light page or a dark one. These
/// are not in the schemes: they are chosen by Mode alone.
pub fn stick_tone(n: usize, light: bool) -> Color {
    match (n, light) {
        (0, false) => STICK_BOOT,
        (0, true) => STICK_BOOT_LIGHT,
        (n, false) => STICKS[(n - 1) % STICKS.len()],
        (n, true) => STICKS_LIGHT[(n - 1) % STICKS_LIGHT.len()],
    }
}

/// The structure chart's line colours (`docs/DESIGN.md` §4.16), one
/// per key held here, in turn.
pub const KEY_LINES: [Paint; 5] = [
    Paint::role(Role::Primary),
    Paint::role(Role::Ok),
    Paint::role(Role::Warning),
    Paint::role(Role::Error),
    Paint::role(Role::OnSurfaceVariant),
];
