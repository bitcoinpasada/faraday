//! Faraday's colours (`docs/THEMES.md`): Material 3's colour roles, each
//! scheme's values generated from its theme's seed ([`schemes`]), and
//! the paints a drawing call takes. A paint is a role at an opacity,
//! looked up in the scheme the frame is drawn in, or a colour drawn as
//! it is in every theme (a QR code, a camera's overlay, a stick's pill).
//!
//! The names below are Faraday's colours from before the roles, each a
//! paint standing for its main role (`docs/THEMES.md` §6), so every
//! screen draws on the generated schemes; each use moves to its role by
//! name, and the names go once none is left.

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

/// Fields, word cells and boxes inset on the page.
pub const BG: Paint = Paint::role(Role::SurfaceContainerHighest);
/// The expanded rail, a shade apart from the page.
pub const SIDEBAR: Paint = Paint::role(Role::SurfaceContainerLow);
/// The page and a sheet; a card.
pub const SURFACE: Paint = Paint::role(Role::Surface);
/// A card's edge and the sidebar's.
pub const LINE: Paint = Paint::role(Role::OutlineVariant);
/// An idle badge, a step's number, a track, a hover or a press on the
/// page: surface-container-highest (`docs/THEMES.md` §6), on which text
/// and icons read in every scheme. A rule inside a card is `BORDER`.
pub const INNER: Paint = Paint::role(Role::SurfaceContainerHighest);
/// A box inset on the page (`BG`) while pressed or chosen: a shade apart
/// from it.
pub const BG_PRESSED: Paint = Paint::role(Role::SurfaceContainerHigh);
/// A card's, a rule's and a sheet's edge.
pub const BORDER: Paint = Paint::role(Role::OutlineVariant);
/// Text.
pub const TEXT: Paint = Paint::role(Role::OnSurface);
/// Labels.
pub const MUTED: Paint = Paint::role(Role::OnSurfaceVariant);
/// What is not there yet; a control not available now.
pub const DIM: Paint = Paint::role(Role::OnSurfaceVariant);
/// The accent: filled controls.
pub const ACCENT: Paint = Paint::role(Role::Primary);
/// Text on the accent.
pub const ON_ACCENT: Paint = Paint::role(Role::OnPrimary);
/// Done, verified, can sign.
pub const OK: Paint = Paint::role(Role::Ok);
/// Waiting, unsaved, needs attention.
pub const WARN: Paint = Paint::role(Role::Warning);
/// Refused.
pub const ERR: Paint = Paint::role(Role::Error);
/// A control's edge: an outlined button, a segmented button, a field,
/// the field chip, a ⋮.
pub const OUTLINE: Paint = Paint::role(Role::Outline);
/// A chosen option and a tonal button's fill.
pub const SELECTED: Paint = Paint::role(Role::SecondaryContainer);
/// Text and icons on `SELECTED`.
pub const ON_SELECTED: Paint = Paint::role(Role::OnSecondaryContainer);
/// The toast's fill.
pub const INVERSE: Paint = Paint::role(Role::InverseSurface);
/// The toast's text.
pub const ON_INVERSE: Paint = Paint::role(Role::InverseOnSurface);
/// The accent where it is read rather than filled: text, icons and the
/// focus ring.
pub const ACCENT_TEXT: Paint = Paint::role(Role::Primary);
/// What a sheet's page is dimmed with over its frost, at
/// `tokens::SCRIM_ALPHA`.
pub const SCRIM: Paint = Paint::role(Role::Scrim);

/// Over a camera's picture (`docs/QR.md` §8 item 4), whatever the
/// theme: the picture is the same in every theme. The corner brackets
/// while looking, and the words on the state line's band.
pub const VIEWFINDER: Color = Color::rgb(0xff, 0xff, 0xff);
/// The corner brackets while a QR that read is in view.
pub const VIEWFINDER_READ: Color = Color::rgb(0x4c, 0xd9, 0x7b);
/// The band under the state line, drawn at
/// `tokens::VIEWFINDER_BAND_ALPHA`.
pub const VIEWFINDER_BAND: Color = Color::rgb(0x05, 0x08, 0x0b);

// The backup board's boxes (`docs/BACKUP-FLOW.md` §17.15): places
// neutral, sticks the highest surface, vaults the primary family, files
// the secondary family, paper the tertiary family (`docs/THEMES.md` §6).
// A vault and paper show their family by edge, band and label on the
// lowest surface, not by a container fill: a fidelity scheme (Cobalt,
// Bitcoin Orange) puts its primary-container at the seed's own tone,
// dark blue in Light, and its tertiary-container at a fixed saturated
// tone, brown or cyan in both modes.
/// A place on the backup board: its fill.
pub const PLACE_FILL: Paint = Paint::role(Role::SurfaceContainerLow);
/// A place's header band.
pub const PLACE_BAND: Paint = Paint::role(Role::SurfaceContainerHigh);
/// A place's edge and its thick left stripe.
pub const PLACE_EDGE: Paint = Paint::role(Role::Outline);
/// A place's name, glyph and ×.
pub const PLACE_TEXT: Paint = Paint::role(Role::OnSurface);
/// A stick box on the board, and **+ stick**.
pub const STICK_FILL: Paint = Paint::role(Role::SurfaceContainerHighest);
/// A stick box's edge.
pub const STICK_EDGE: Paint = Paint::role(Role::Outline);
/// "Stick n", its glyph and ×.
pub const STICK_TEXT: Paint = Paint::role(Role::OnSurfaceVariant);
/// A vault box on the board.
pub const VAULT_FILL: Paint = Paint::role(Role::SurfaceContainerLowest);
/// A vault box's edge and its solid header band, and the lock mark.
pub const VAULT_EDGE: Paint = Paint::role(Role::Primary);
/// "Vault n · sealed" and its × on the vault's band.
pub const VAULT_ON: Paint = Paint::role(Role::OnPrimary);
/// Paper (and Scanned in) on the board.
pub const PAPER_FILL: Paint = Paint::role(Role::SurfaceContainerLowest);
/// Paper's edge.
pub const PAPER_EDGE: Paint = Paint::role(Role::Tertiary);
/// "Paper", its glyph.
pub const PAPER_TEXT: Paint = Paint::role(Role::Tertiary);
/// The files box inside a stick box on the board (`docs/BACKUP-FLOW.md`
/// §17.17): the files kept on the stick outside any vault.
pub const FILES_FILL: Paint = Paint::role(Role::SecondaryContainer);
/// The files box's edge.
pub const FILES_EDGE: Paint = Paint::role(Role::Secondary);
/// "Files · not sealed", its glyph.
pub const FILES_TEXT: Paint = Paint::role(Role::OnSecondaryContainer);
/// A chip's edge on the board.
pub const CHIP_EDGE: Paint = Paint::role(Role::Outline);

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
pub const KEY_LINES: [Paint; 5] = [ACCENT, OK, WARN, ERR, MUTED];
