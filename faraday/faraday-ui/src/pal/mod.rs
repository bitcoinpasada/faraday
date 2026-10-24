//! Faraday's colours: the page, text and tones as the Dark theme draws
//! them, and each other theme's table of what it draws them as
//! (`light`, `dark`, `high_contrast`).

use faraday_draw::Color;

/// The page.
pub const BG: Color = Color::rgb(0x11, 0x16, 0x1c);
/// The sidebar.
pub const SIDEBAR: Color = Color::rgb(0x0d, 0x12, 0x18);
/// A card.
pub const SURFACE: Color = Color::rgb(0x19, 0x21, 0x2a);
/// A card's edge and the sidebar's.
pub const LINE: Color = Color::rgb(0x1f, 0x29, 0x33);
/// A rule inside a card, and an idle badge.
pub const INNER: Color = Color::rgb(0x22, 0x2c, 0x37);
/// A secondary button's edge.
pub const BORDER: Color = Color::rgb(0x30, 0x3d, 0x49);
/// Text.
pub const TEXT: Color = Color::rgb(0xe0, 0xe9, 0xef);
/// Labels.
pub const MUTED: Color = Color::rgb(0x9a, 0xac, 0xb9);
/// What is not there yet.
pub const DIM: Color = Color::rgb(0x6f, 0x82, 0x91);
/// The accent.
pub const ACCENT: Color = Color::rgb(0x83, 0xd8, 0xef);
/// Text on the accent. One step from `SIDEBAR`, so the light theme
/// can tell them apart.
pub const ON_ACCENT: Color = Color::rgb(0x0d, 0x12, 0x19);
/// Done, verified, can sign.
pub const OK: Color = Color::rgb(0x8b, 0xd4, 0xb2);
/// Waiting, unsaved, needs attention.
pub const WARN: Color = Color::rgb(0xf0, 0xc0, 0x77);
/// Refused.
pub const ERR: Color = Color::rgb(0xef, 0x9b, 0x9b);

/// Over a camera's picture (`docs/QR.md` §8 item 4), whatever the
/// theme: none of these is in the themes' tables, since the picture
/// is the same in every theme. The corner brackets while looking,
/// and the words on the state line's band.
pub const VIEWFINDER: Color = Color::rgb(0xff, 0xff, 0xff);
/// The corner brackets while a QR that read is in view.
pub const VIEWFINDER_READ: Color = Color::rgb(0x4c, 0xd9, 0x7b);
/// The band under the state line, drawn at
/// `tokens::VIEWFINDER_BAND_ALPHA`.
pub const VIEWFINDER_BAND: Color = Color::rgb(0x05, 0x08, 0x0b);

// The backup board's boxes (`docs/BACKUP-FLOW.md` §17.15): one hue
// per kind of box, pale fills and strong edges, each theme its own.
/// A place on the backup board: its fill, a pale green (`docs/DESIGN.md` §4.17).
pub const PLACE_FILL: Color = Color::rgb(0x1e, 0x2e, 0x30);
/// A place's header band.
pub const PLACE_BAND: Color = Color::rgb(0x24, 0x3f, 0x38);
/// A place's edge and its thick left stripe: 3:1 or more against the page.
pub const PLACE_EDGE: Color = Color::rgb(0x5e, 0x9a, 0x73);
/// A place's name, glyph and ×: 4.5:1 or more on its band and its fill.
pub const PLACE_TEXT: Color = Color::rgb(0x86, 0xb3, 0x96);
/// A stick box on the board, and **+ stick**: a cool slate grey.
pub const STICK_FILL: Color = Color::rgb(0x2b, 0x34, 0x3e);
/// A stick box's edge: 3:1 or more against the place round it.
pub const STICK_EDGE: Color = Color::rgb(0x7d, 0x8b, 0x98);
/// "Stick n", its glyph and ×: 4.5:1 or more on the stick's fill.
pub const STICK_TEXT: Color = Color::rgb(0x96, 0xa1, 0xac);
/// A vault box on the board: a pale tint of the accent.
pub const VAULT_FILL: Color = Color::rgb(0x2a, 0x3e, 0x4a);
/// A vault box's edge and its solid header band, and the lock mark: the accent, moved where it must be to stand 3:1 against the stick round it. One step from `ACCENT` in this table, so the themes can tell them apart.
pub const VAULT_EDGE: Color = Color::rgb(0x83, 0xd8, 0xee);
/// "Vault n · sealed" and its × on the vault's band: 4.5:1 or more on it. One step from `ON_ACCENT`, so the themes can tell them apart.
pub const VAULT_ON: Color = Color::rgb(0x0d, 0x12, 0x1a);
/// Paper (and Scanned in) on the board: a warm cream.
pub const PAPER_FILL: Color = Color::rgb(0x34, 0x37, 0x35);
/// Paper's edge, a brown: 3:1 or more against the place round it.
pub const PAPER_EDGE: Color = Color::rgb(0xb8, 0x98, 0x50);
/// "Paper", its glyph: 4.5:1 or more on paper's fill.
pub const PAPER_TEXT: Color = Color::rgb(0xc8, 0xa8, 0x60);
/// The files box inside a stick box on the board (`docs/BACKUP-FLOW.md`
/// §17.17): the files kept on the stick outside any vault, a pale blue.
pub const FILES_FILL: Color = Color::rgb(0x2e, 0x40, 0x55);
/// The files box's edge: 3:1 or more against the stick round it.
pub const FILES_EDGE: Color = Color::rgb(0x53, 0x8c, 0xcd);
/// "Files · not sealed", its glyph: 4.5:1 or more on the files fill.
pub const FILES_TEXT: Color = Color::rgb(0x8d, 0xb0, 0xd9);
/// A chip's edge on the board: a neutral a step stronger than `BORDER`.
pub const CHIP_EDGE: Color = Color::rgb(0x69, 0x71, 0x79);

// The codex32 worksheets (`docs/CODEX32.md` §7.5): the booklet's colour
// roles, each theme its own, characters 4.5:1 or more on each (7:1 in
// the high-contrast themes).
/// The checksum squares, worked from the bottom up: the booklet's pink
/// on a light page, a muted rose on a dark one.
pub const CODEX32_CHECKSUM_FILL: Color = Color::rgb(0x41, 0x2e, 0x3b);
/// The rows read from a table (sums and lookups): a raised surface,
/// the booklet's light grey.
pub const CODEX32_LOOKUP_FILL: Color = Color::rgb(0x27, 0x2f, 0x38);

/// The boot stick's pill and render: Bitcoin's orange, as the Bitcoin
/// Orange theme's accent, on a dark page.
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
/// are not in the themes' tables: each theme draws them as they are.
pub fn stick_tone(n: usize, light: bool) -> Color {
    match (n, light) {
        (0, false) => STICK_BOOT,
        (0, true) => STICK_BOOT_LIGHT,
        (n, false) => STICKS[(n - 1) % STICKS.len()],
        (n, true) => STICKS_LIGHT[(n - 1) % STICKS_LIGHT.len()],
    }
}

/// The structure chart's line colours (`docs/DESIGN.md` §4.16), one
/// per key held here, in turn: the theme's own colours, so each theme
/// draws them as it draws the rest.
pub const KEY_LINES: [Color; 5] = [ACCENT, OK, WARN, ERR, MUTED];

mod dark;
mod high_contrast;
mod light;

pub use dark::*;
pub use high_contrast::*;
pub use light::*;
