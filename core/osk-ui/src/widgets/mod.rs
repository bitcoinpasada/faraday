//! Widgets (UX.md §6): the leaves of the layout tree.
//!
//! Immediate mode: the application rebuilds the tree every frame from its
//! own state, so a widget is plain data — its label, whether it is enabled,
//! which application [`Id`] it reports. Everything that must survive
//! between frames (pressed state, scroll offsets, hold progress, keyboard
//! modifiers, pager pages) lives in [`crate::UiState`].
//!
//! Each widget knows three things: its intrinsic size ([`Widget::measure`]),
//! its [`HitTarget`] if it is interactive, and how to draw itself
//! ([`Widget::draw`]). Composite molecules that are just trees of simpler
//! widgets (warning cards, frames, grids) are builder functions in
//! [`crate::organisms`].

pub mod chunked;
pub mod icon;
pub mod keyboard;
mod render;

pub use chunked::is_chunked;
pub use icon::Icon;
pub use render::{draw_tree, draw_tree_within};

use alloc::rc::Rc;
use alloc::string::String;
use alloc::vec::Vec;
use zeroize::Zeroizing;

use osk_codec::qr::{QUIET_ZONE, QrMatrix};

use crate::canvas::Canvas;
use crate::color::{Color, Theme};
use crate::geom::{Rect, Size, SizeClass};
use crate::layout::{Id, LayoutCtx};
use crate::state::{PressLook, UiState};
use crate::text::{self, Font, TextAlign};

pub use keyboard::{ALL_KEYS, DONE_DISABLED, KeyInput, KeyMask, KeyboardKind};

/// Design tokens (`docs/DESIGN.md` §3). The module lives in
/// [`crate::tokens`]; this re-export keeps `widgets::tokens` working for
/// everything that already spells it that way.
pub use crate::tokens;

/// A colour role, resolved against the theme at draw time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    /// Body text.
    Text,
    /// Secondary text.
    Muted,
    /// Accent.
    Primary,
    /// Text on the accent colour.
    OnPrimary,
    /// Danger.
    Danger,
    /// Caution.
    Caution,
    /// Info.
    Info,
    /// Success.
    Success,
}

impl Tone {
    /// The colour for this role.
    pub fn color(self, theme: &Theme) -> Color {
        match self {
            Tone::Text => theme.text,
            Tone::Muted => theme.muted,
            Tone::Primary => theme.primary,
            Tone::OnPrimary => theme.on_primary,
            Tone::Danger => theme.danger,
            Tone::Caution => theme.caution,
            Tone::Info => theme.info,
            Tone::Success => theme.success,
        }
    }
}

/// Button emphasis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonStyle {
    /// The one action the screen is about.
    Primary,
    /// Everything else.
    Secondary,
    /// Destructive.
    Danger,
    /// A label in the accent colour on no fill: a secondary action that
    /// must not compete with the one the screen is about.
    Text,
    /// A label in the muted colour on no fill: the quietest action there
    /// is, for an escape hatch that must not draw the eye.
    TextMuted,
}

/// Warning rank (`docs/PLANNING.md` §13).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WarningLevel {
    /// Informational.
    Info,
    /// Needs attention.
    Caution,
    /// Cannot be dismissed with the tap that confirms.
    Danger,
}

impl WarningLevel {
    /// The tone that colours this level.
    pub fn tone(self) -> Tone {
        match self {
            WarningLevel::Info => Tone::Info,
            WarningLevel::Caution => Tone::Caution,
            WarningLevel::Danger => Tone::Danger,
        }
    }

    /// Short label shown in the badge.
    pub fn label(self) -> &'static str {
        match self {
            WarningLevel::Info => "INFO",
            WarningLevel::Caution => "CAUTION",
            WarningLevel::Danger => "DANGER",
        }
    }

    /// The icon that marks this level.
    pub fn icon(self) -> Icon {
        match self {
            WarningLevel::Info => Icon::Info,
            WarningLevel::Caution => Icon::Warning,
            WarningLevel::Danger => Icon::Error,
        }
    }
}

/// How a list row draws the value under its label
/// (`docs/DESIGN.md` §4.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ValueStyle {
    /// The monospace face, for a value made of characters rather than
    /// words: a path, a fingerprint, an elided key.
    pub mono: bool,
    /// The value's colour role: caution for "not verified" (§4.8).
    pub tone: Tone,
    /// Whether the value sits beside the label on one line, at the
    /// label token, rather than under it. The entry screen of
    /// `docs/DESIGN.md` §4.3 asks for it where the space above the
    /// field holds one line and not two.
    pub beside: bool,
    /// A glyph drawn before the value, and before each part of a value
    /// that lists several ([`VALUE_SEPARATOR`]): the fingerprint glyph
    /// before a fingerprint (`docs/PLANNING.md` §16.131).
    pub glyph: Option<Icon>,
}

/// What separates the parts of a value that lists several — two
/// fingerprints on one line — and so where a value's glyph is drawn
/// again: the middle dot every subtitle in the interface uses.
pub const VALUE_SEPARATOR: &str = " \u{00b7} ";

/// Whether `label` names a key: one of its parts, between
/// [`VALUE_SEPARATOR`]s, is a fingerprint — "73c5da0a", "73c5da0a ·
/// SegWit", "#0 · 73c5da0a". Such a label is read character by
/// character, so a row or a table sets it in the mono face
/// (`docs/DESIGN.md` §3: "fingerprints wherever they appear, including a
/// title and a row label").
pub fn names_a_key(label: &str) -> bool {
    label
        .split(VALUE_SEPARATOR)
        .any(|part| part.len() == 8 && part.bytes().all(|b| b.is_ascii_hexdigit()))
}

/// Background style of a [`Widget::Panel`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanelStyle {
    /// A card on the background.
    Surface,
    /// The secret frame: distinct background and border.
    Secret,
    /// A warning card tinted by level.
    Warning(WarningLevel),
    /// Outline only.
    Outline,
    /// A viewfinder: four corner brackets in the accent colour and
    /// nothing between them, so a camera preview behind it is not
    /// framed by a box that competes with the code (UX review
    /// 2026-09-07, §2b.1).
    Viewfinder,
    /// A translucent band of the background colour, laid under text that
    /// sits over an image: the scanner's state line over the camera
    /// preview, which is a light frame as often as a dark one
    /// (`docs/DESIGN.md` §4.9).
    Scrim,
}

/// What input a placed widget reacts to. Copied into the [`crate::Layout`]
/// so that [`crate::UiState`] can resolve taps without the tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HitTarget {
    /// A tap produces [`crate::Action::Tap`].
    Tap(Id),
    /// Press and hold for [`tokens::HOLD_MS`].
    Hold(Id),
    /// Press to reveal: the secret under it shows while a finger is
    /// down and re-masks on release. No duration, no completion.
    Reveal(Id),
    /// An on-screen keyboard.
    Keyboard {
        /// Application id.
        id: Id,
        /// Layout.
        kind: KeyboardKind,
        /// BIP-39 enabled letters.
        enabled: KeyMask,
        /// PIN scramble seed.
        scramble: Option<u32>,
    },
    /// A strip of `cells` equal-width cells, the first `count` of which
    /// carry a candidate.
    Candidates {
        /// Application id.
        id: Id,
        /// Number of candidates.
        count: u8,
        /// Cells the strip's width is divided into. Equal to `count` on a
        /// strip of words, and [`tokens::CANDIDATE_CELLS_HANZI`] on a
        /// one-character strip, whose cells keep the keyboard's key pitch
        /// however few characters are left.
        cells: u8,
    },
    /// A paged chunked string with `pages` pages.
    Pager {
        /// Application id.
        id: Id,
        /// Page count.
        pages: u8,
    },
}

/// A leaf of the layout tree.
pub enum Widget {
    /// Static text.
    Text {
        /// The text.
        text: String,
        /// Font.
        font: Font,
        /// Colour role.
        tone: Tone,
        /// Horizontal alignment.
        align: TextAlign,
        /// Word-wrap to the available width (otherwise one line).
        wrap: bool,
        /// A glyph before the text, which then stays on one line
        /// ([`Widget::with_value_glyph`]).
        glyph: Option<Icon>,
    },
    /// A tap button.
    Button {
        /// Application id.
        id: Id,
        /// Label.
        label: String,
        /// Emphasis.
        style: ButtonStyle,
        /// Whether it accepts taps.
        enabled: bool,
    },
    /// A press-and-hold button; fills over [`tokens::HOLD_MS`] and then
    /// reports [`crate::Action::HoldCompleted`].
    HoldButton {
        /// Application id.
        id: Id,
        /// Label.
        label: String,
        /// Emphasis.
        style: ButtonStyle,
        /// Whether it accepts a hold. A dead hold keeps its place and
        /// its size, so nothing moves when the step it confirms becomes
        /// available.
        enabled: bool,
    },
    /// A label on a rounded fill: the segments of a segmented control,
    /// the fingerprints of the key strip, the committed word numbers.
    Chip {
        /// Application id, when tappable.
        id: Option<Id>,
        /// Label.
        label: String,
        /// Drawn on the accent colour.
        selected: bool,
        /// Small text and padding, for dense rows (committed-word
        /// numbers, fingerprints). Not a touch target on its own.
        compact: bool,
        /// Measures to zero width, so that a row of weighted segments
        /// divides the width equally whatever the labels are.
        stretch: bool,
        /// Drawn in the muted colour, and still tappable: the choice is
        /// a poor one here and the caption under the row says why (UX
        /// review 2026-09-07, §2.8).
        dimmed: bool,
        /// Label size in dp, when the row it sits in picked one for every
        /// chip together. `None` takes the size [`chip_metrics`] gives.
        label_dp: Option<f32>,
        /// A glyph before the label ([`Widget::with_value_glyph`]).
        glyph: Option<Icon>,
    },
    /// A small bold label in a tinted pill.
    Badge {
        /// Application id, when the badge opens a screen: §4.8's tier
        /// badge opens the tiers page. The pill is the target, and it is
        /// as wide as its label, so a row of badges keeps its shape.
        id: Option<Id>,
        /// Label.
        label: String,
        /// Colour role.
        tone: Tone,
    },
    /// An on/off switch. Tapping reports `Tap(id)`; the application flips
    /// `on`.
    Toggle {
        /// Application id.
        id: Id,
        /// State.
        on: bool,
    },
    /// A one-dp horizontal rule.
    Divider,
    /// A one-dp vertical rule spanning the height it is given; the
    /// separator between a sidebar and its content pane.
    VerticalDivider,
    /// A tappable icon: a 48 dp target with the glyph centred in it.
    IconButton {
        /// Application id.
        id: Id,
        /// Which icon.
        icon: Icon,
        /// Glyph size in dp.
        size_dp: f32,
        /// Colour role.
        tone: Tone,
        /// Whether it accepts taps.
        enabled: bool,
    },
    /// An app-bar action drawn as a ring around its glyph, emptying
    /// clockwise as `fraction` falls from one to zero
    /// (`docs/DESIGN.md` §4.10: the eye while a reveal runs, which
    /// §4.14 makes the only countdown in the interface).
    Ring {
        /// Application id.
        id: Id,
        /// The glyph inside the ring.
        icon: Icon,
        /// The share of the reveal still to run.
        fraction: f32,
    },
    /// An icon from the icon face, centred in a `size_dp` square.
    Icon {
        /// Which icon.
        icon: Icon,
        /// Side of the square, in dp.
        size_dp: f32,
        /// Colour role.
        tone: Tone,
    },
    /// A hub tile: an icon over a label on a surface card, with an
    /// optional count in its top-right corner.
    Tile {
        /// Application id.
        id: Id,
        /// Icon.
        icon: Icon,
        /// Label.
        label: String,
        /// Whether it accepts taps.
        enabled: bool,
        /// A small count in the corner; `None` leaves the tile plain.
        badge: Option<String>,
    },
    /// A determinate progress bar.
    ProgressBar {
        /// Progress in `0..=1`.
        fraction: f32,
    },
    /// A background for a `Stack`: fill and border, no content.
    Panel {
        /// Style.
        style: PanelStyle,
    },
    /// A long string for human comparison, in groups of four.
    ChunkedString {
        /// Application id (for the pager).
        id: Id,
        /// The string.
        text: String,
        /// Largest size to try, in dp.
        max_size: f32,
        /// Smallest size before paging, in dp.
        min_size: f32,
        /// Centre each line rather than starting it at the left edge.
        center: bool,
        /// Draw nothing while keeping the rectangle the string would
        /// take (`docs/DESIGN.md` §4.10: a masked Secret screen shows
        /// one eye-slash icon in a panel of the same size).
        masked: bool,
    },
    /// A row on a surface card: optional leading icon, title, an
    /// optional subtitle and trailing value, and a chevron when it
    /// opens a screen.
    ListRow {
        /// Application id, when tappable.
        id: Option<Id>,
        /// Leading icon, drawn in the accent colour.
        icon: Option<Icon>,
        /// Title.
        title: String,
        /// Subtitle.
        subtitle: Option<String>,
        /// Trailing text (right-aligned), in front of the chevron.
        trailing: Option<String>,
        /// Colour role of the trailing text. A state worth acting on —
        /// "not verified" — is drawn in its own colour rather than as
        /// an aside (UX review 2026-09-07, §3.4).
        trailing_tone: Tone,
        /// Row height in dp.
        height_dp: f32,
        /// Title colour role.
        tone: Tone,
        /// Whether the row accepts taps. A disabled row is dimmed and
        /// has no hit target: that is how an unavailable choice reads
        /// (UX.md §6).
        enabled: bool,
        /// The current choice of a list: a check mark at the trailing
        /// edge.
        selected: bool,
        /// Whether a tappable row carries a chevron. A choice list does
        /// not: there the trailing mark means "this is the current
        /// one", and nothing else (UX review 2026-09-07, §2.2).
        chevron: bool,
        /// A switch at the trailing edge and its state, for the boolean
        /// setting of `docs/DESIGN.md` §4.2. The switch is drawn on the
        /// row rather than beside it, so the whole row is the one hit
        /// target and a dimmed row dims its switch with it.
        switch: Option<bool>,
        /// How a row that carries a value draws it: the title becomes
        /// the muted label above and the subtitle becomes the value
        /// below (`docs/DESIGN.md` §4.1: "label above, value below,
        /// everywhere a row carries a value"). `None` is the plain row,
        /// whose title is the name of the thing and whose subtitle is an
        /// aside — a key row, a menu row with no value.
        value_below: Option<ValueStyle>,
        /// Whether the row paints a surface under itself and indents its
        /// text by the screen padding. False is the flat row §4.11 draws
        /// inside a record table: the label starts at the label column
        /// and nothing boxes it in.
        surface: bool,
        /// Whether the two lines keep the dense space above and below
        /// rather than the grid step. Only the natural height changes:
        /// a row whose `height_dp` is taller is drawn exactly as before.
        /// The path editor's presets ask for it, so the block they sit in
        /// fits the space above the field on `wide` (`docs/DESIGN.md`
        /// §4.6, §2.6).
        tight: bool,
        /// Whether the title is read character by character — a
        /// fingerprint, a word being spelled — and so is set in the mono
        /// face (§16.126). A row whose value carries the characters says
        /// so through `value_below` instead. On a row with a value it is
        /// the label that is data: "#0 · 73c5da0a", a wallet named by its
        /// key.
        mono_title: bool,
        /// Whether a plain row's second line is read character by
        /// character — the path under a scanned cosigner's fingerprint —
        /// and so is set in the mono face.
        mono_subtitle: bool,
    },
    /// A small muted label over a value.
    LabelledValue {
        /// Label.
        label: String,
        /// Value.
        value: String,
        /// Draw the value in the monospace face.
        mono: bool,
        /// A glyph before the value, which then stays on one line
        /// ([`Widget::with_value_glyph`]).
        glyph: Option<Icon>,
    },
    /// Up to N tappable candidate words in equal-width cells.
    CandidateStrip {
        /// Application id.
        id: Id,
        /// Candidates.
        words: Vec<String>,
        /// More candidates follow than the strip holds, so its last cell
        /// is the key that turns the page: a chevron, no word
        /// (`docs/DESIGN.md` §4.3).
        more: bool,
        /// The cell of this strip that a tap has selected, drawn as a
        /// chosen cell is: on the accent colour. On the small panel a
        /// candidate is selected by one tap and accepted by the next
        /// (`docs/PLANNING.md` §16.50).
        selected: Option<u8>,
        /// The cell a tap accepts outright, outlined rather than filled:
        /// the lone candidate a word typed out in full leaves
        /// (`docs/DESIGN.md` §4.3).
        outlined: Option<u8>,
    },
    /// An on-screen keyboard.
    Keyboard {
        /// Application id.
        id: Id,
        /// Layout.
        kind: KeyboardKind,
        /// BIP-39: enabled letters. Ignored by other kinds.
        enabled: KeyMask,
        /// PIN: scramble seed. Ignored by other kinds.
        scramble: Option<u32>,
    },
    /// The touch surface of a secret panel (UX.md §5): the tinted
    /// ground and accent border of [`PanelStyle::Secret`], which reveals
    /// the content drawn over it while a finger is anywhere on it.
    SecretSurface {
        /// Application id, shared with the content that reads
        /// [`UiState::is_held`].
        id: Id,
    },
    /// A QR code: dark modules on a light square with the standard
    /// four-module quiet zone, at the largest whole-pixel module scale
    /// that fits, always light-on-dark-agnostic (a scanner needs a light
    /// background whatever the theme). With `hold` set, the code is a
    /// secret and stays blank, with a "hold to show" placeholder, unless
    /// that hold button is under a finger (UX.md §5 Secret frame).
    Qr {
        /// The modules, shared with the application's state.
        matrix: Rc<QrMatrix>,
        /// The secret panel that reveals a secret code.
        hold: Option<Id>,
        /// Whether the code is already revealed without a finger on the
        /// panel: the app bar's eye (UX review 2026-09-07, §2.7).
        revealed: bool,
    },
    /// A QR code drawn to be copied by hand: the modules as squares
    /// large enough for a pen, a thin line between them, a thick guide
    /// line every [`tokens::QR_GRID_BAND`] modules, and a gutter of row
    /// and column labels along the top and left edges
    /// (`docs/PLANNING.md` §8.2 item 5a). `origin` and `span` select the
    /// square of modules drawn, so a symbol too large for the class
    /// pages by quadrant. Like [`Widget::Qr`], a secret grid is blank
    /// until its panel is held or the app bar's eye is showing it.
    QrGrid {
        /// The modules, shared with the application's state.
        matrix: Rc<QrMatrix>,
        /// The first module of the region drawn, `(x, y)` in the symbol.
        origin: (u16, u16),
        /// The region's side in modules.
        span: u16,
        /// The secret panel that reveals a secret grid.
        hold: Option<Id>,
        /// Whether the grid is already revealed without a finger on the
        /// panel: the app bar's eye.
        revealed: bool,
    },
    /// A camera frame filling the rectangle it is given, scaled to
    /// cover it and cropped to it: the camera's latest frame inside the
    /// scanner's square (`docs/DESIGN.md` §4.9). Drawn in colour where
    /// the frame carries chroma and in grey where it does not.
    Luma {
        /// Image width in pixels.
        width: u16,
        /// Image height in pixels.
        height: u16,
        /// The luma, row-major, shared with the application's state
        /// and wiped when the last holder drops them.
        pixels: Rc<Zeroizing<Vec<u8>>>,
        /// The NV12 chroma plane for those pixels, shared and wiped the
        /// same way, or `None` for a frame with no colour in it.
        chroma: Option<Rc<Zeroizing<Vec<u8>>>>,
    },
}

impl Widget {
    /// A single-line text.
    pub fn text(text: impl Into<String>, font: Font, tone: Tone) -> Self {
        Widget::Text {
            text: text.into(),
            font,
            tone,
            align: TextAlign::Start,
            wrap: false,
            glyph: None,
        }
    }

    /// A word-wrapped paragraph.
    pub fn paragraph(text: impl Into<String>, font: Font, tone: Tone) -> Self {
        Widget::Text {
            text: text.into(),
            font,
            tone,
            align: TextAlign::Start,
            wrap: true,
            glyph: None,
        }
    }

    /// Body text in the default size.
    pub fn body(text: impl Into<String>) -> Self {
        Widget::paragraph(text, Font::regular(tokens::BODY), Tone::Text)
    }

    /// A title.
    pub fn title(text: impl Into<String>) -> Self {
        Widget::paragraph(text, Font::semibold(tokens::TITLE), Tone::Text)
    }

    /// Secondary text.
    pub fn small(text: impl Into<String>) -> Self {
        Widget::paragraph(text, Font::regular(tokens::CAPTION), Tone::Muted)
    }

    /// Sets the alignment of a text widget (no effect on others).
    pub fn align(mut self, a: TextAlign) -> Self {
        if let Widget::Text { align, .. } = &mut self {
            *align = a;
        }
        self
    }

    /// A button.
    pub fn button(id: Id, label: impl Into<String>, style: ButtonStyle) -> Self {
        Widget::Button {
            id,
            label: label.into(),
            style,
            enabled: true,
        }
    }

    /// A disabled button.
    pub fn disabled_button(id: Id, label: impl Into<String>, style: ButtonStyle) -> Self {
        Widget::Button {
            id,
            label: label.into(),
            style,
            enabled: false,
        }
    }

    /// A hold button.
    pub fn hold_button(id: Id, label: impl Into<String>, style: ButtonStyle) -> Self {
        Widget::hold_button_enabled(id, label, style, true)
    }

    /// A hold button that is dead until the screen says otherwise.
    pub fn hold_button_enabled(
        id: Id,
        label: impl Into<String>,
        style: ButtonStyle,
        enabled: bool,
    ) -> Self {
        Widget::HoldButton {
            id,
            label: label.into(),
            style,
            enabled,
        }
    }

    /// A chip.
    pub fn chip(id: Option<Id>, label: impl Into<String>, selected: bool) -> Self {
        Widget::Chip {
            id,
            label: label.into(),
            selected,
            compact: false,
            stretch: false,
            dimmed: false,
            label_dp: None,
            glyph: None,
        }
    }

    /// One segment of a segmented control: a chip that takes an equal
    /// share of the row whatever its label.
    pub fn segment(id: Id, label: impl Into<String>, selected: bool) -> Self {
        Widget::Chip {
            id: Some(id),
            label: label.into(),
            selected,
            compact: false,
            stretch: true,
            dimmed: false,
            label_dp: None,
            glyph: None,
        }
    }

    /// A segment whose choice is a poor one on this display: the label
    /// is muted, the segment still accepts taps, and the row says why.
    pub fn dimmed_segment(id: Id, label: impl Into<String>, selected: bool) -> Self {
        Widget::Chip {
            id: Some(id),
            label: label.into(),
            selected,
            compact: false,
            stretch: true,
            dimmed: true,
            label_dp: None,
            glyph: None,
        }
    }

    /// The dense two-line row §4.6's path presets ask for: the space
    /// above and below the two lines is the dense gap rather than the
    /// grid step, so the block of presets fits the space above the field
    /// (no effect on other widgets, and none on a row whose `height_dp`
    /// is already taller than its content).
    pub fn tight(mut self) -> Self {
        if let Widget::ListRow { tight, .. } = &mut self {
            *tight = true;
        }
        self
    }

    /// A compact chip: small monospace text, 24 dp tall.
    pub fn compact_chip(id: Option<Id>, label: impl Into<String>, selected: bool) -> Self {
        Widget::Chip {
            id,
            label: label.into(),
            selected,
            compact: true,
            stretch: false,
            dimmed: false,
            label_dp: None,
            glyph: None,
        }
    }

    /// Sets the label size in dp of a chip (no effect on other
    /// widgets). A segmented row that has to stay on one line picks one
    /// size for every chip in it, so the labels are the same size
    /// whichever word is the longest.
    pub fn with_label_size(mut self, dp: f32) -> Self {
        if let Widget::Chip { label_dp, .. } = &mut self {
            *label_dp = Some(dp);
        }
        self
    }

    /// A badge.
    pub fn badge(label: impl Into<String>, tone: Tone) -> Self {
        Widget::Badge {
            id: None,
            label: label.into(),
            tone,
        }
    }

    /// A badge that opens a screen (§4.8's tier badge).
    pub fn badge_button(id: Id, label: impl Into<String>, tone: Tone) -> Self {
        Widget::Badge {
            id: Some(id),
            label: label.into(),
            tone,
        }
    }

    /// An icon.
    pub fn icon(icon: Icon, size_dp: f32, tone: Tone) -> Self {
        Widget::Icon {
            icon,
            size_dp,
            tone,
        }
    }

    /// A tappable icon in a 48 dp target.
    pub fn icon_button(id: Id, icon: Icon, size_dp: f32, tone: Tone) -> Self {
        Widget::IconButton {
            id,
            icon,
            size_dp,
            tone,
            enabled: true,
        }
    }

    /// A tile.
    pub fn tile(id: Id, icon: Icon, label: impl Into<String>, enabled: bool) -> Self {
        Widget::Tile {
            id,
            icon,
            label: label.into(),
            enabled,
            badge: None,
        }
    }

    /// Sets the count in a tile's corner (no effect on other widgets).
    pub fn with_badge(mut self, count: Option<String>) -> Self {
        if let Widget::Tile { badge, .. } = &mut self {
            *badge = count;
        }
        self
    }

    /// The touch surface of a secret panel.
    pub fn secret_surface(id: Id) -> Self {
        Widget::SecretSurface { id }
    }

    /// A chunked string with the default size range (16 dp down to the
    /// 8 dp floor).
    pub fn chunked(id: Id, text: impl Into<String>) -> Self {
        Widget::ChunkedString {
            id,
            text: text.into(),
            max_size: tokens::MONO,
            min_size: CHUNK_MIN_SIZE,
            center: false,
            masked: false,
        }
    }

    /// A chunked string that starts at `max_size` dp instead of 16, for a
    /// place where the string shares the screen with a code.
    pub fn chunked_at(id: Id, text: impl Into<String>, max_size: f32) -> Self {
        Widget::ChunkedString {
            id,
            text: text.into(),
            max_size,
            min_size: CHUNK_MIN_SIZE,
            center: false,
            masked: false,
        }
    }

    /// A chunked string with both ends of the size range set, for a
    /// screen that would rather shrink the text than page it.
    pub fn chunked_between(id: Id, text: impl Into<String>, max_size: f32, min_size: f32) -> Self {
        Widget::ChunkedString {
            id,
            text: text.into(),
            max_size,
            min_size,
            center: false,
            masked: false,
        }
    }

    /// Centres a chunked string's lines (no effect on other widgets).
    pub fn centered(mut self) -> Self {
        if let Widget::ChunkedString { center, .. } = &mut self {
            *center = true;
        }
        self
    }

    /// Keeps a chunked string's rectangle and draws nothing in it, so a
    /// masked panel is the size of the value it hides (no effect on
    /// other widgets).
    pub fn masked(mut self) -> Self {
        if let Widget::ChunkedString { masked, .. } = &mut self {
            *masked = true;
        }
        self
    }

    /// A 56 dp list row.
    pub fn list_row(
        id: Option<Id>,
        title: impl Into<String>,
        subtitle: Option<String>,
        trailing: Option<String>,
    ) -> Self {
        let title = title.into();
        let mono_title = names_a_key(&title);
        Widget::ListRow {
            id,
            icon: None,
            title,
            subtitle,
            trailing,
            trailing_tone: Tone::Muted,
            height_dp: tokens::ROW,
            tone: Tone::Text,
            enabled: true,
            selected: false,
            chevron: true,
            switch: None,
            value_below: None,
            surface: true,
            tight: false,
            mono_title,
            mono_subtitle: false,
        }
    }

    /// A 72 dp menu row: icon, label, optional subtitle, chevron.
    pub fn menu_row(
        id: Id,
        icon: Icon,
        title: impl Into<String>,
        subtitle: Option<String>,
    ) -> Self {
        Widget::ListRow {
            id: Some(id),
            icon: Some(icon),
            title: title.into(),
            subtitle,
            trailing: None,
            trailing_tone: Tone::Muted,
            height_dp: tokens::MENU_ROW,
            tone: Tone::Text,
            enabled: true,
            selected: false,
            chevron: true,
            switch: None,
            value_below: None,
            surface: true,
            tight: false,
            mono_title: false,
            mono_subtitle: false,
        }
    }

    /// Puts a switch at a row's trailing edge, in the state given
    /// (no effect on other widgets).
    pub fn with_switch(mut self, on: bool) -> Self {
        if let Widget::ListRow {
            switch, chevron, ..
        } = &mut self
        {
            *switch = Some(on);
            *chevron = false;
        }
        self
    }

    /// Sets a row's leading icon (no effect on other widgets).
    pub fn with_icon(mut self, i: Icon) -> Self {
        if let Widget::ListRow { icon, .. } = &mut self {
            *icon = Some(i);
        }
        self
    }

    /// Sets a row's height in dp (no effect on other widgets).
    pub fn with_height(mut self, dp: f32) -> Self {
        if let Widget::ListRow { height_dp, .. } = &mut self {
            *height_dp = dp;
        }
        self
    }

    /// Sets a row's trailing text, which takes the place of its chevron
    /// (no effect on other widgets).
    pub fn with_trailing(mut self, t: Option<String>) -> Self {
        if let Widget::ListRow { trailing, .. } = &mut self {
            *trailing = t;
        }
        self
    }

    /// Sets the colour of a row's trailing text (no effect on other
    /// widgets).
    pub fn with_trailing_tone(mut self, t: Tone) -> Self {
        if let Widget::ListRow { trailing_tone, .. } = &mut self {
            *trailing_tone = t;
        }
        self
    }

    /// Sets a row's title colour (no effect on other widgets).
    pub fn with_tone(mut self, t: Tone) -> Self {
        if let Widget::ListRow { tone, .. } = &mut self {
            *tone = t;
        }
        self
    }

    /// Enables or disables a row (no effect on other widgets). A
    /// disabled row is dimmed and cannot be tapped.
    pub fn with_enabled(mut self, e: bool) -> Self {
        if let Widget::ListRow { enabled, .. } = &mut self {
            *enabled = e;
        }
        self
    }

    /// Takes the chevron off a row (no effect on other widgets): the
    /// rows of a choice list carry a check mark or nothing.
    pub fn without_chevron(mut self) -> Self {
        if let Widget::ListRow { chevron, .. } = &mut self {
            *chevron = false;
        }
        self
    }

    /// Draws a row's subtitle as the value under its label: the label
    /// in the label token, muted, and the value under it in the body or
    /// monospace face and its own tone (`docs/DESIGN.md` §4.1). No
    /// effect on other widgets.
    pub fn with_value_below(mut self, mono: bool, tone: Tone) -> Self {
        if let Widget::ListRow { value_below, .. } = &mut self {
            *value_below = Some(ValueStyle {
                mono,
                tone,
                beside: false,
                glyph: None,
            });
        }
        self
    }

    /// Draws a row's subtitle as the value beside its label, on one
    /// line, both at the label token: the row an entry screen falls back
    /// to when the space above its field holds one line and not two
    /// (`docs/DESIGN.md` §4.3). No effect on other widgets.
    pub fn with_value_beside(mut self, mono: bool, tone: Tone) -> Self {
        if let Widget::ListRow { value_below, .. } = &mut self {
            *value_below = Some(ValueStyle {
                mono,
                tone,
                beside: true,
                glyph: None,
            });
        }
        self
    }

    /// Sets a row's title in the mono face: the fingerprint that names a
    /// key, and anything else a row is titled by that is read character
    /// by character (§16.126). No effect on other widgets, or on a row
    /// whose value carries the characters.
    pub fn with_mono_title(mut self) -> Self {
        if let Widget::ListRow { mono_title, .. } = &mut self {
            *mono_title = true;
        }
        self
    }

    /// Sets a plain row's second line in the mono face: a path under a
    /// fingerprint. No effect on other widgets, or on a row whose value
    /// is drawn under its label (`with_value_below`).
    pub fn with_mono_subtitle(mut self) -> Self {
        if let Widget::ListRow { mono_subtitle, .. } = &mut self {
            *mono_subtitle = true;
        }
        self
    }

    /// Takes the surface and the indent off a row: §4.11's reference row
    /// drawn flat inside a record table (no effect on other widgets).
    pub fn flat(mut self) -> Self {
        if let Widget::ListRow { surface, .. } = &mut self {
            *surface = false;
        }
        self
    }

    /// Marks a row as the current choice: a check mark at its trailing
    /// edge (no effect on other widgets).
    pub fn with_selected(mut self, sel: bool) -> Self {
        if let Widget::ListRow { selected, .. } = &mut self {
            *selected = sel;
        }
        self
    }

    /// A labelled value.
    pub fn labelled(label: impl Into<String>, value: impl Into<String>, mono: bool) -> Self {
        Widget::LabelledValue {
            label: label.into(),
            value: value.into(),
            mono,
            glyph: None,
        }
    }

    /// Puts `glyph` before a value: a text, a chip's label, a labelled
    /// value, or the value of a row that already carries one
    /// ([`Widget::with_value_below`], [`Widget::with_value_beside`]).
    /// A value that lists several parts ([`VALUE_SEPARATOR`]) takes the
    /// glyph before each. The value is measured with its glyphs, so a
    /// value that no longer fits steps down a size as it would without
    /// them. The fingerprint glyph before a fingerprint shown as a value
    /// is the one use (`docs/PLANNING.md` §16.131). No effect on other
    /// widgets.
    pub fn with_value_glyph(mut self, g: Icon) -> Self {
        match &mut self {
            Widget::Text { glyph, .. }
            | Widget::Chip { glyph, .. }
            | Widget::LabelledValue { glyph, .. } => *glyph = Some(g),
            Widget::ListRow {
                value_below: Some(v),
                ..
            } => v.glyph = Some(g),
            _ => {}
        }
        self
    }

    /// A keyboard.
    pub fn keyboard(id: Id, kind: KeyboardKind) -> Self {
        Widget::Keyboard {
            id,
            kind,
            enabled: ALL_KEYS,
            scramble: None,
        }
    }

    /// A pad whose ✓ key is live only when `done` is true: the dice pad
    /// (keys `1`–`6`, backspace, ✓; physical digits as accelerators), the
    /// coin pad (`Heads`, `Tails`, backspace, ✓; physical `h`/`t`) or the
    /// hex keyboard (UX.md §6).
    pub fn pad(id: Id, kind: KeyboardKind, done: bool) -> Self {
        Widget::Keyboard {
            id,
            kind,
            enabled: if done {
                ALL_KEYS
            } else {
                ALL_KEYS | DONE_DISABLED
            },
            scramble: None,
        }
    }

    /// The dice pad with ✓ live.
    pub fn dice_pad(id: Id) -> Self {
        Widget::pad(id, KeyboardKind::Dice, true)
    }

    /// The coin pad with ✓ live.
    pub fn coin_pad(id: Id) -> Self {
        Widget::pad(id, KeyboardKind::Coin, true)
    }

    /// A public QR code.
    pub fn qr(matrix: Rc<QrMatrix>) -> Self {
        Widget::Qr {
            matrix,
            hold: None,
            revealed: false,
        }
    }

    /// A secret QR code, blank unless `hold` is held or `revealed` is
    /// already true (the app bar's eye).
    pub fn secret_qr(matrix: Rc<QrMatrix>, hold: Id, revealed: bool) -> Self {
        Widget::Qr {
            matrix,
            hold: Some(hold),
            revealed,
        }
    }

    /// A transcription grid over the region `origin`..`origin + span`
    /// of `matrix`, blank unless `hold` is held or `revealed` is already
    /// true (the app bar's eye).
    pub fn secret_qr_grid(
        matrix: Rc<QrMatrix>,
        origin: (u16, u16),
        span: u16,
        hold: Id,
        revealed: bool,
    ) -> Self {
        Widget::QrGrid {
            matrix,
            origin,
            span,
            hold: Some(hold),
            revealed,
        }
    }

    /// Whether a QR widget draws its modules right now: a public code
    /// always, a secret one while its panel is held or its screen's eye
    /// is showing it.
    pub fn qr_visible(&self, state: &UiState) -> bool {
        match self {
            Widget::Qr { hold: None, .. } | Widget::QrGrid { hold: None, .. } => true,
            Widget::Qr {
                hold: Some(id),
                revealed,
                ..
            }
            | Widget::QrGrid {
                hold: Some(id),
                revealed,
                ..
            } => *revealed || state.is_held(*id),
            _ => false,
        }
    }

    /// The application id, if the widget has one.
    pub fn id(&self) -> Option<Id> {
        match self {
            Widget::Button { id, .. }
            | Widget::HoldButton { id, .. }
            | Widget::IconButton { id, .. }
            | Widget::Toggle { id, .. }
            | Widget::Tile { id, .. }
            | Widget::ChunkedString { id, .. }
            | Widget::CandidateStrip { id, .. }
            | Widget::Keyboard { id, .. }
            | Widget::Ring { id, .. } => Some(*id),
            Widget::Chip { id, .. } | Widget::ListRow { id, .. } | Widget::Badge { id, .. } => *id,
            Widget::SecretSurface { id } => Some(*id),
            _ => None,
        }
    }

    /// The application id of a dimmed control, which has no hit target
    /// and still takes focus: `docs/DESIGN.md` §4.15, "a dimmed control
    /// holds focus and does nothing on Enter, as it does nothing on a
    /// tap".
    pub fn dimmed_id(&self) -> Option<Id> {
        match self {
            Widget::Button {
                id, enabled: false, ..
            }
            | Widget::HoldButton {
                id, enabled: false, ..
            }
            | Widget::Tile {
                id, enabled: false, ..
            }
            | Widget::IconButton {
                id, enabled: false, ..
            } => Some(*id),
            Widget::ListRow {
                id, enabled: false, ..
            } => *id,
            _ => None,
        }
    }

    /// The application id the focus ring is drawn for, if this widget
    /// can hold focus.
    pub fn focus_id(&self) -> Option<Id> {
        match self.hit_target() {
            Some(HitTarget::Tap(id) | HitTarget::Hold(id) | HitTarget::Reveal(id)) => Some(id),
            Some(HitTarget::Pager { id, .. }) => Some(id),
            Some(HitTarget::Keyboard { .. } | HitTarget::Candidates { .. }) => None,
            None => self.dimmed_id(),
        }
    }

    /// The colour the focus ring is drawn in: the accent, except on an
    /// item whose own fill is the accent — a primary button, a checked
    /// chip, a toggle that is on — where an accent ring would vanish
    /// into it, so the ring takes the accent's text colour instead
    /// (`docs/DESIGN.md` §4.15).
    fn focus_ring_color(&self, theme: &Theme) -> Color {
        match self {
            Widget::Button {
                style: ButtonStyle::Primary,
                enabled: true,
                ..
            }
            | Widget::Chip { selected: true, .. }
            | Widget::Toggle { on: true, .. } => theme.on_primary,
            _ => theme.primary,
        }
    }

    /// The corner radius the focus ring takes in `rect`: the widget's
    /// own, so the ring follows the shape under it.
    fn focus_radius(&self, ctx: &LayoutCtx, rect: Rect) -> f32 {
        match self {
            Widget::IconButton { .. } | Widget::Ring { .. } => rect.w.min(rect.h) as f32 / 2.0,
            Widget::Toggle { .. } => ctx.px(SWITCH_HEIGHT) as f32 / 2.0,
            Widget::Badge { .. } => ctx.px(tokens::BADGE_RADIUS) as f32,
            Widget::Chip { compact, .. } => ctx.px(if *compact {
                tokens::RADIUS_SMALL
            } else {
                tokens::GAP
            }) as f32,
            Widget::ListRow { surface: false, .. } => 0.0,
            _ => ctx.px(tokens::RADIUS) as f32,
        }
    }

    /// The hit target, if interactive.
    pub fn hit_target(&self) -> Option<HitTarget> {
        match self {
            Widget::Button {
                id, enabled: true, ..
            }
            | Widget::Toggle { id, .. } => Some(HitTarget::Tap(*id)),
            Widget::Tile {
                id, enabled: true, ..
            }
            | Widget::IconButton {
                id, enabled: true, ..
            }
            | Widget::Ring { id, .. } => Some(HitTarget::Tap(*id)),
            Widget::HoldButton {
                id, enabled: true, ..
            } => Some(HitTarget::Hold(*id)),
            Widget::SecretSurface { id } => Some(HitTarget::Reveal(*id)),
            Widget::Chip { id: Some(id), .. } | Widget::Badge { id: Some(id), .. } => {
                Some(HitTarget::Tap(*id))
            }
            Widget::ListRow {
                id: Some(id),
                enabled: true,
                ..
            } => Some(HitTarget::Tap(*id)),
            Widget::CandidateStrip { id, words, .. } if !words.is_empty() => {
                // The real counts need the rectangle the strip was given,
                // so [`crate::layout`] refills both when it places it.
                let count = words.len().min(255) as u8;
                Some(HitTarget::Candidates {
                    id: *id,
                    count,
                    cells: count,
                })
            }
            Widget::Keyboard {
                id,
                kind,
                enabled,
                scramble,
            } => Some(HitTarget::Keyboard {
                id: *id,
                kind: *kind,
                enabled: *enabled,
                scramble: *scramble,
            }),
            // Placeholder: the layout engine replaces the page count once
            // it knows the rectangle, and drops the target when there is
            // only one page (see `layout::Solver::place`).
            Widget::ChunkedString { id, .. } => Some(HitTarget::Pager { id: *id, pages: 0 }),
            _ => None,
        }
    }

    /// Where a list row's text goes and in which faces, for a row
    /// `width` pixels wide: measuring and drawing both ask, so a title
    /// that takes a second line is measured with it (`docs/PLANNING.md`
    /// §16.133). `None` for any other widget.
    fn row_text(&self, ctx: &LayoutCtx, width: i32) -> Option<RowText> {
        let Widget::ListRow {
            id,
            icon,
            title,
            subtitle,
            trailing,
            height_dp,
            enabled,
            selected,
            chevron,
            switch,
            value_below,
            surface,
            mono_title,
            mono_subtitle,
            ..
        } = self
        else {
            return None;
        };
        let scale = ctx.scale;
        let width_of = |s: &str, f: Font| text::width(&f.sized(scale), s);
        // §4.11: a reference row inside a record is drawn flat — no
        // surface, and its label starts at the label column rather than
        // a padding step inside a card.
        let pad = if *surface { ctx.px(tokens::PAD) } else { 0 };
        let mut left = pad;
        if icon.is_some() {
            // The label sits one grid step after the icon, not a padding
            // step: the icon and its label belong together, and the
            // label needs the width. A flat row keeps the step too, or
            // its label runs into the glyph.
            left += row_icon(ctx, *height_dp).1 + ctx.px(tokens::GAP);
        }
        let mut right = width - pad;
        if switch.is_some() {
            right -= ctx.px(SWITCH_WIDTH) + ctx.px(tokens::GAP);
        }
        let tappable = id.is_some() && *enabled;
        if *selected || (tappable && *chevron) {
            right -= ctx.px(tokens::ICON_SMALL) + ctx.px(tokens::GAP);
        }
        if let (false, Some(t)) = (*selected, trailing) {
            right -= width_of(t, Font::regular(tokens::CAPTION)) + ctx.px(tokens::GAP);
        }
        let text_w = (right - left).max(0);
        // A row that carries a value inverts the two lines: the label is
        // the muted line above and the value is what is read
        // (`docs/DESIGN.md` §4.1).
        let beside = value_below.as_ref().is_some_and(|v| v.beside);
        let glyph = value_below.as_ref().and_then(|v| v.glyph);
        let (title_font, sub_font) = match value_below {
            Some(v) if v.beside && v.mono => {
                (row_title_font(true, *mono_title), Font::mono(tokens::LABEL))
            }
            Some(v) if v.beside => (
                row_title_font(true, *mono_title),
                Font::regular(tokens::LABEL),
            ),
            Some(v) if v.mono => (row_title_font(true, *mono_title), Font::mono(tokens::MONO)),
            Some(_) => (
                row_title_font(true, *mono_title),
                Font::regular(tokens::BODY),
            ),
            None => (
                row_title_font(false, *mono_title),
                if *mono_subtitle {
                    Font::mono(tokens::LABEL)
                } else {
                    Font::regular(tokens::LABEL)
                },
            ),
        };
        let sub_line_h = sub_font.sized(scale).line_height();
        // A value that does not fit drops a size rather than losing its
        // last characters behind the trailing control: an elided key in
        // groups of four is twenty-one characters, which is more than a
        // 268 dp row holds at the mono size. It is measured with its
        // glyphs (§16.131 rule 3).
        let mut sub_font = sub_font;
        if let Some(sb) = subtitle {
            for size in [tokens::LABEL, tokens::CHIP_TIGHT, tokens::CAPTION] {
                if glyphed_width(ctx, &sub_font.sized(scale), sb, glyph) <= text_w {
                    break;
                }
                sub_font = sub_font.with_size(size);
            }
        }
        // On one line the value takes what it needs at the trailing edge
        // and the label takes the rest, so neither is drawn over the
        // other.
        let sub_w = match subtitle {
            Some(sb) if beside => {
                glyphed_width(ctx, &sub_font.sized(scale), sb, glyph) + ctx.px(tokens::GAP)
            }
            _ => 0,
        };
        let title_w = (text_w - sub_w).max(0);
        let mut title_h = title_font.sized(scale).line_height();
        // §16.133 rule 1: a title that does not fit at its size is drawn
        // at the label size, and one that still does not fit breaks at
        // the last space that leaves a first line that fits. A title
        // whose first word alone is wider than the row keeps one line
        // and is cut at the row's edge (rule 2). The one-line row keeps
        // its one line.
        let mut title_font = title_font;
        let mut title_break = None;
        if width_of(title, title_font) > title_w {
            title_font = title_font.with_size(tokens::LABEL);
            if !beside && width_of(title, title_font) > title_w {
                title_break = title
                    .rmatch_indices(' ')
                    .map(|(i, _)| i)
                    .find(|&i| width_of(&title[..i], title_font) <= title_w)
                    .map(|i| (i, i + 1));
                if title_break.is_some() {
                    title_h = 2 * title_font.sized(scale).line_height();
                }
            }
        }
        Some(RowText {
            pad,
            left,
            text_w,
            title_w,
            title_font,
            title_break,
            title_h,
            sub_font,
            sub_line_h,
            sub_w,
        })
    }

    /// Intrinsic size in pixels when offered at most `max`.
    pub fn measure(&self, ctx: &LayoutCtx, max: Size) -> Size {
        let scale = ctx.scale;
        match self {
            Widget::Text {
                text,
                font,
                wrap,
                glyph,
                ..
            } => {
                let face = font.sized(scale);
                if glyph.is_some() {
                    // A value with a glyph is one line, a size smaller
                    // where it does not fit whole.
                    let face = one_line_face_glyphed(ctx, text, *font, max.w, *glyph);
                    Size::new(
                        glyphed_width(ctx, &face, text, *glyph),
                        text::measure(&face, text).height,
                    )
                } else if *wrap {
                    Size::new(
                        text::wrapped_width(&face, text, max.w),
                        text::wrapped_height(&face, text, max.w),
                    )
                } else {
                    let m = text::measure(&face, text);
                    Size::new(runs_width(&face, text), m.height)
                }
            }
            Widget::Button { label, .. } | Widget::HoldButton { label, .. } => {
                let face = Font::semibold(tokens::BODY).sized(scale);
                let w = text::width(&face, label) + 2 * ctx.px(tokens::PAD);
                Size::new(w.min(max.w), ctx.px(tokens::CTA))
            }
            Widget::Icon { size_dp, .. } => {
                let s = ctx.px(*size_dp);
                Size::new(s, s)
            }
            Widget::IconButton { .. } | Widget::Ring { .. } => {
                let s = ctx.px(tokens::TOUCH);
                Size::new(s, s)
            }
            Widget::Chip {
                label,
                compact,
                stretch,
                label_dp,
                glyph,
                ..
            } => {
                let (font, pad, h) = chip_metrics(*compact, ctx.class);
                let font = chip_font(font, *label_dp);
                let face = font.sized(scale);
                let w = if *stretch {
                    0
                } else {
                    glyphed_width(ctx, &face, label, *glyph) + 2 * ctx.px(pad)
                };
                Size::new(w, ctx.px(h))
            }
            Widget::Badge { label, .. } => {
                let face = Font::semibold(tokens::CAPTION).sized(scale);
                Size::new(
                    text::width(&face, label) + 2 * ctx.px(tokens::BADGE_PAD),
                    face.line_height() + 2 * ctx.px(tokens::BADGE_PAD_Y),
                )
            }
            Widget::Toggle { .. } => Size::new(ctx.px(tokens::SWITCH_WIDTH), ctx.px(tokens::TOUCH)),
            Widget::Divider => Size::new(0, ctx.px(1.0)),
            Widget::VerticalDivider => Size::new(ctx.px(1.0).max(1), 0),
            Widget::Tile { .. } => Size::new(ctx.px(tokens::TILE_WIDTH), ctx.px(tokens::MENU_ROW)),
            Widget::ProgressBar { .. } => Size::new(0, ctx.px(tokens::BAR_HEIGHT)),
            Widget::Panel { .. } | Widget::SecretSurface { .. } | Widget::Luma { .. } => Size::ZERO,
            Widget::ChunkedString {
                text,
                max_size,
                min_size,
                ..
            } => {
                let (plan, metrics) = chunk_plan(ctx, text, *max_size, *min_size, max);
                let (_, _, line_h) = metrics[plan.size_index];
                let label_h = if plan.per_page.is_some() {
                    Font::regular(tokens::CAPTION).sized(scale).line_height()
                } else {
                    0
                };
                Size::new(max.w, plan.lines as i32 * line_h + label_h)
            }
            Widget::ListRow {
                subtitle,
                height_dp,
                tight,
                value_below,
                ..
            } => {
                let rt = self.row_text(ctx, max.w).unwrap_or_default();
                let beside = value_below.as_ref().is_some_and(|v| v.beside);
                let sub_h = match subtitle {
                    Some(_) if !beside => {
                        Font::regular(tokens::LABEL).sized(scale).line_height()
                            + ctx.px(tokens::STACKED_LINE_GAP)
                    }
                    _ => 0,
                };
                // One grid step above and below the text, so a row that
                // carries a second line still fits the height its screen
                // asked for; the dense step where the row asked for it.
                let pad = if *tight {
                    tokens::GAP_SMALL
                } else {
                    tokens::GAP
                };
                Size::new(
                    max.w,
                    (rt.title_h + sub_h + ctx.px(pad)).max(ctx.px(*height_dp)),
                )
            }
            Widget::LabelledValue {
                value, mono, glyph, ..
            } => {
                let label_h = Font::regular(tokens::CAPTION).sized(scale).line_height();
                let face = if *mono {
                    Font::mono(tokens::MONO).sized(scale)
                } else {
                    Font::regular(tokens::BODY).sized(scale)
                };
                let value_h = if glyph.is_some() {
                    face.line_height()
                } else {
                    text::wrapped_height(&face, value, max.w)
                };
                Size::new(max.w, label_h + value_h)
            }
            Widget::CandidateStrip { words, .. } => {
                // A one-character strip is the keyboard's ten-key row:
                // ten cells at the key pitch, so it takes the whole width
                // it is offered whatever is left in it.
                if one_character(words) {
                    return Size::new(max.w, ctx.px(tokens::candidate_cell(ctx.class, true)));
                }
                // The natural width: one cell per word, every cell as
                // wide as the widest of them. A column stretches the
                // strip across the screen; a row leaves it this width,
                // which is what `wide` wants (`docs/DESIGN.md` §4.3:
                // "one row of natural-width chips").
                let face = candidate_font(ctx.class).sized(scale);
                let pad = 2 * ctx.px(CANDIDATE_PAD);
                let widest = words
                    .iter()
                    .map(|w| text::width(&face, w) + pad)
                    .max()
                    .unwrap_or(0);
                let n = words.len() as i32;
                let natural = widest * n + ctx.px(CANDIDATE_GAP) * (n - 1).max(0);
                Size::new(
                    natural.min(max.w),
                    ctx.px(tokens::candidate_cell(ctx.class, false)),
                )
            }
            Widget::Keyboard { kind, .. } => Size::new(max.w, keyboard::height(*kind, ctx, max)),
            Widget::Qr { .. } => {
                let side = max
                    .w
                    .min(max.h)
                    .min(ctx.px(tokens::qr_side(ctx.class)))
                    .max(0);
                Size::new(side, side)
            }
            Widget::QrGrid { .. } => {
                let side = max
                    .w
                    .min(max.h)
                    .min(ctx.px(tokens::qr_grid_side(ctx.class)))
                    .max(0);
                Size::new(side, side)
            }
        }
    }

    /// Draws the widget into `rect`.
    pub fn draw(
        &self,
        c: &mut Canvas,
        rect: Rect,
        ctx: &LayoutCtx,
        theme: &Theme,
        state: &UiState,
    ) {
        match self {
            Widget::Text {
                text,
                font,
                tone,
                align,
                wrap,
                glyph,
            } => {
                let color = tone.color(theme);
                if *wrap && glyph.is_none() {
                    c.text_wrapped(rect, text, *font, color, *align);
                } else {
                    draw_one_line(c, rect, ctx, text, *font, color, *align, *glyph);
                }
            }
            Widget::Button {
                id,
                label,
                style,
                enabled,
            } => {
                let pressed = if *enabled { state.pressed(*id) } else { None };
                draw_button(c, rect, ctx, theme, label, *style, *enabled, pressed);
            }
            Widget::HoldButton {
                id,
                label,
                style,
                enabled,
            } => {
                let progress = if *enabled {
                    state.hold_progress(*id)
                } else {
                    0.0
                };
                draw_hold_button(c, rect, ctx, theme, label, *style, *enabled, progress);
            }
            Widget::Chip {
                id,
                label,
                selected,
                compact,
                dimmed,
                label_dp,
                glyph,
                ..
            } => {
                let (font, _, _) = chip_metrics(*compact, ctx.class);
                let mut font = chip_font(font, *label_dp);
                // A segment that is too narrow for its label drops a size
                // rather than clipping the word in half. A row that
                // picked one size for every chip already measured the
                // margin it kept, so there the check is that margin.
                let pad = 2 * ctx.px(if label_dp.is_some() {
                    tokens::CHIP_PAD_TIGHT
                } else {
                    tokens::GAP_SMALL
                });
                if glyphed_width(ctx, &font.sized(ctx.scale), label, *glyph) + pad > rect.w {
                    font = font.with_size(tokens::CAPTION);
                }
                let radius = ctx.px(if *compact {
                    tokens::RADIUS_SMALL
                } else {
                    tokens::GAP
                }) as f32;
                let pressed = id.and_then(|id| state.pressed(id));
                let (fill, color) = match (*selected, pressed) {
                    (true, Some(look)) => (pressed_accent(theme.primary, look), theme.on_primary),
                    (true, None) => (theme.primary, theme.on_primary),
                    (false, Some(look)) => {
                        (pressed_neutral(theme.surface, theme, look), theme.text)
                    }
                    (false, None) => (theme.surface, theme.text),
                };
                let color = if *dimmed && !*selected {
                    theme.outline
                } else {
                    color
                };
                c.fill_rounded_rect(rect, radius, fill);
                c.push_clip(rect);
                let face = font.sized(ctx.scale);
                let w = glyphed_width(ctx, &face, label, *glyph);
                let x = rect.x + (rect.w - w) / 2;
                let y = rect.y + (rect.h - face.line_height()) / 2;
                draw_glyphed(c, ctx, &face, x, y, label, color, *glyph);
                c.pop_clip();
            }
            Widget::Badge { label, tone, .. } => {
                let color = tone.color(theme);
                c.fill_rounded_rect(
                    rect,
                    ctx.px(tokens::BADGE_RADIUS) as f32,
                    color.with_alpha(48),
                );
                c.text_in(
                    rect,
                    label,
                    Font::semibold(tokens::CAPTION),
                    color,
                    TextAlign::Center,
                );
            }
            Widget::Toggle { on, .. } => {
                let track_h = ctx.px(SWITCH_HEIGHT);
                let track = Rect::new(
                    rect.x,
                    rect.y + (rect.h - track_h) / 2,
                    ctx.px(SWITCH_WIDTH).min(rect.w),
                    track_h,
                );
                draw_switch(c, track, ctx, theme, *on, true);
            }
            Widget::Divider => {
                c.fill_rect(
                    Rect::new(rect.x, rect.y, rect.w, ctx.px(1.0).max(1)),
                    theme.outline,
                );
            }
            Widget::VerticalDivider => {
                c.fill_rect(
                    Rect::new(rect.x, rect.y, ctx.px(1.0).max(1), rect.h),
                    theme.outline,
                );
            }
            Widget::Tile {
                id,
                icon,
                label,
                enabled,
                badge,
            } => {
                let pressed = if *enabled { state.pressed(*id) } else { None };
                let fill = match pressed {
                    Some(look) => pressed_neutral(theme.surface, theme, look),
                    None => theme.surface,
                };
                c.fill_rounded_rect(rect, ctx.px(tokens::RADIUS) as f32, fill);
                // A tile grows with its class: on a phone the grid is
                // the whole screen, so the mark and the word in it are
                // one step up the ramp (UX review 2026-09-07, §3.1).
                let icon_dp = tokens::tile_icon(ctx.class);
                // A word wider than the tile takes one step down rather
                // than reaching the tile's edges: the band under the
                // scanner holds "Read a file" beside two short words.
                let label_font = {
                    let wide = Font::semibold(tokens::tile_label(ctx.class));
                    let room = rect.w - 2 * ctx.px(tokens::GAP_SMALL);
                    if c.measure_text(label, wide).width > room {
                        Font::semibold(tokens::tile_label_tight(ctx.class))
                    } else {
                        wide
                    }
                };
                let icon_h = ctx.px(icon_dp);
                let label_h = label_font.sized(ctx.scale).line_height();
                let gap = ctx.px(tokens::GAP);
                let top = rect.y + (rect.h - icon_h - gap - label_h) / 2;
                draw_icon(
                    c,
                    Rect::new(rect.x, top, rect.w, icon_h),
                    ctx,
                    *icon,
                    icon_dp,
                    if *enabled {
                        theme.primary
                    } else {
                        theme.outline
                    },
                );
                c.push_clip(rect);
                c.text_in(
                    Rect::new(rect.x, top + icon_h + gap, rect.w, label_h),
                    label,
                    label_font,
                    if *enabled { theme.text } else { theme.outline },
                    TextAlign::Center,
                );
                c.pop_clip();
                if let Some(count) = badge {
                    let font = Font::semibold(tokens::CAPTION);
                    let pad = ctx.px(tokens::TILE_BADGE_PAD);
                    let w = c.measure_text(count, font).width + 2 * pad;
                    let h = font.sized(ctx.scale).line_height() + ctx.px(2.0);
                    let inset = ctx.px(tokens::GAP);
                    let pill = Rect::new(rect.right() - inset - w, rect.y + inset, w, h);
                    c.fill_rounded_rect(pill, h as f32 / 2.0, theme.primary);
                    c.text_in(pill, count, font, theme.on_primary, TextAlign::Center);
                }
            }
            Widget::Icon {
                icon,
                size_dp,
                tone,
            } => draw_icon(c, rect, ctx, *icon, *size_dp, tone.color(theme)),
            Widget::IconButton {
                id,
                icon,
                size_dp,
                tone,
                enabled,
            } => {
                if let Some(look) = state.pressed(*id).filter(|_| *enabled) {
                    let r = (rect.w.min(rect.h) / 2) as f32;
                    c.fill_circle(
                        rect.center().x as f32,
                        rect.center().y as f32,
                        r,
                        pressed_on_background(theme, look),
                    );
                }
                let color = if *enabled {
                    tone.color(theme)
                } else {
                    theme.outline
                };
                draw_icon(c, rect, ctx, *icon, *size_dp, color);
            }
            Widget::Ring { id, icon, fraction } => {
                let cx = rect.center().x as f32;
                let cy = rect.center().y as f32;
                // §16.132 rule 3: the eye's ring is pressed like the
                // other app bar buttons, a disc under the ring.
                if let Some(look) = state.pressed(*id) {
                    let r = (rect.w.min(rect.h) / 2) as f32;
                    c.fill_circle(cx, cy, r, pressed_on_background(theme, look));
                }
                let radius = ctx.px(tokens::RING_SIZE) as f32 / 2.0;
                let width = ctx.px(tokens::RING_STROKE).max(1) as f32;
                c.stroke_arc(cx, cy, radius, width, 1.0, theme.surface_raised);
                c.stroke_arc(cx, cy, radius, width, *fraction, theme.primary);
                draw_icon(c, rect, ctx, *icon, tokens::ICON_SMALL, theme.primary);
            }
            Widget::ProgressBar { fraction } => {
                let h = ctx.px(tokens::BAR_HEIGHT).max(2);
                let bar = Rect::new(rect.x, rect.y + (rect.h - h) / 2, rect.w, h);
                c.fill_rounded_rect(bar, h as f32 / 2.0, theme.surface_raised);
                let w = (bar.w as f32 * fraction.clamp(0.0, 1.0)) as i32;
                if w > 0 {
                    c.fill_rounded_rect(
                        Rect::new(bar.x, bar.y, w, h),
                        h as f32 / 2.0,
                        theme.primary,
                    );
                }
            }
            Widget::SecretSurface { .. } => {
                let radius = ctx.px(tokens::RADIUS) as f32;
                c.fill_rounded_rect(rect, radius, theme.secret_background);
                c.stroke_rounded_rect(rect, radius, ctx.px(2.0).max(2) as f32, theme.secret_border);
            }
            Widget::Panel { style } => {
                let radius = ctx.px(tokens::RADIUS) as f32;
                let stroke = ctx.px(1.0).max(1) as f32;
                match style {
                    PanelStyle::Surface => c.fill_rounded_rect(rect, radius, theme.surface),
                    PanelStyle::Outline => {
                        c.stroke_rounded_rect(rect, radius, stroke, theme.outline)
                    }
                    PanelStyle::Viewfinder => {
                        // Four corner brackets: two bars per corner, each
                        // laid along the edge it belongs to.
                        let arm = (rect.w.min(rect.h) / 6).max(ctx.px(tokens::PAD));
                        let w = ctx.px(2.0).max(2);
                        for (x, near_left) in [(rect.x, true), (rect.right() - arm, false)] {
                            for (y, near_top) in [(rect.y, true), (rect.bottom() - arm, false)] {
                                let bar_y = if near_top { y } else { y + arm - w };
                                let bar_x = if near_left { x } else { x + arm - w };
                                c.fill_rect(Rect::new(x, bar_y, arm, w), theme.primary);
                                c.fill_rect(Rect::new(bar_x, y, w, arm), theme.primary);
                            }
                        }
                    }
                    PanelStyle::Scrim => {
                        // Opaque enough that muted text on it reads over
                        // a white QR, sheer enough that the frame shows
                        // through.
                        c.fill_rect(rect, theme.background.with_alpha(200));
                    }
                    PanelStyle::Secret => {
                        c.fill_rounded_rect(rect, radius, theme.secret_background);
                        c.stroke_rounded_rect(
                            rect,
                            radius,
                            ctx.px(2.0).max(2) as f32,
                            theme.secret_border,
                        );
                    }
                    PanelStyle::Warning(level) => {
                        let color = level.tone().color(theme);
                        c.fill_rounded_rect(rect, radius, color.with_alpha(32));
                        c.stroke_rounded_rect(rect, radius, stroke, color);
                    }
                }
            }
            Widget::ChunkedString {
                id,
                text,
                max_size,
                min_size,
                center,
                masked,
            } => {
                if !*masked {
                    draw_chunked(
                        c, rect, ctx, theme, state, *id, text, *max_size, *min_size, *center,
                    );
                }
            }
            Widget::ListRow {
                id,
                icon,
                title,
                subtitle,
                trailing,
                trailing_tone,
                height_dp,
                tone,
                enabled,
                selected,
                chevron,
                switch,
                value_below,
                surface,
                tight: _,
                mono_title: _,
                mono_subtitle: _,
            } => {
                let tappable = id.is_some() && *enabled;
                let pressed = id.filter(|_| tappable).and_then(|id| state.pressed(id));
                // §4.11: a reference row inside a record is drawn flat —
                // no surface, and its label starts at the label column
                // rather than a padding step inside a card.
                if *surface {
                    let radius = ctx.px(tokens::RADIUS) as f32;
                    c.fill_rounded_rect(
                        rect,
                        radius,
                        match pressed {
                            Some(look) => pressed_neutral(theme.surface, theme, look),
                            None => theme.surface,
                        },
                    );
                }
                let rt = self.row_text(ctx, rect.w).unwrap_or_default();
                let beside = value_below.as_ref().is_some_and(|v| v.beside);
                let value_glyph = value_below.as_ref().and_then(|v| v.glyph);
                let sub_h = match subtitle {
                    Some(_) if !beside => rt.sub_line_h + ctx.px(tokens::STACKED_LINE_GAP),
                    _ => 0,
                };
                let top = rect.y + (rect.h - rt.title_h - sub_h) / 2;
                let left = rect.x + rt.left;
                if let Some(i) = icon {
                    let (size, px) = row_icon(ctx, *height_dp);
                    // A row's icon takes the row's colour when the row
                    // has one of its own: a danger row is danger
                    // throughout.
                    let icon_color = match (*enabled, tone) {
                        (false, _) => theme.outline,
                        (true, Tone::Text) => theme.primary,
                        (true, t) => t.color(theme),
                    };
                    draw_icon(
                        c,
                        Rect::new(rect.x + rt.pad, rect.y + (rect.h - px) / 2, px, px),
                        ctx,
                        *i,
                        size,
                        icon_color,
                    );
                }
                let mut right = rect.right() - rt.pad;
                // One trailing treatment per meaning (UX review
                // 2026-09-07, §2.2): a check mark marks the current item
                // of a choice list, and nothing else; every navigable
                // row carries a chevron, with its value in front of it.
                if let Some(on) = switch {
                    let w = ctx.px(SWITCH_WIDTH);
                    let track_h = ctx.px(SWITCH_HEIGHT);
                    draw_switch(
                        c,
                        Rect::new(right - w, rect.y + (rect.h - track_h) / 2, w, track_h),
                        ctx,
                        theme,
                        *on,
                        *enabled,
                    );
                    right -= w + ctx.px(tokens::GAP);
                }
                let trailing_icon = if *selected {
                    Some((Icon::Check, theme.primary))
                } else if tappable && *chevron {
                    Some((Icon::ChevronRight, theme.muted))
                } else {
                    None
                };
                if let Some((glyph, color)) = trailing_icon {
                    let px = ctx.px(tokens::ICON_SMALL);
                    draw_icon(
                        c,
                        Rect::new(right - px, rect.y + (rect.h - px) / 2, px, px),
                        ctx,
                        glyph,
                        tokens::ICON_SMALL,
                        color,
                    );
                    right -= px + ctx.px(tokens::GAP);
                }
                if let (false, Some(t)) = (*selected, trailing) {
                    // The trailing text is the caption size, whether it
                    // is a value ("not verified") or the reason a row is
                    // dimmed: either way it is an aside, and the label
                    // beside it is what the row is (UX review
                    // 2026-09-07, §2.2, §2.3).
                    let font = Font::regular(tokens::CAPTION);
                    let w = c.measure_text(t, font).width;
                    let color = if *enabled {
                        trailing_tone.color(theme)
                    } else {
                        theme.outline
                    };
                    c.text_in(
                        Rect::new(right - w, rect.y, w, rect.h),
                        t,
                        font,
                        color,
                        TextAlign::End,
                    );
                }
                let (title_color, sub_color) = match (*enabled, value_below) {
                    (false, _) => (theme.outline, theme.outline),
                    (true, Some(v)) => (theme.muted, v.tone.color(theme)),
                    (true, None) => (tone.color(theme), theme.muted),
                };
                // The clips stop text at its right edge. On the left they
                // leave the small gap free, where the hook of a J or a j
                // reaches past the pen's start.
                let hook = ctx.px(tokens::GAP_SMALL);
                // §16.133: a title that does not fit at the label size
                // either takes a second line, broken at a space.
                c.push_clip(Rect::new(left - hook, rect.y, rt.title_w + hook, rect.h));
                let title_face = rt.title_font.sized(ctx.scale);
                let line_h = title_face.line_height();
                match rt.title_break {
                    Some((end, next)) => {
                        c.text_with_face(&title_face, left, top, &title[..end], title_color);
                        let second = last_line(&title_face, &title[next..], rt.title_w);
                        c.text_with_face(&title_face, left, top + line_h, &second, title_color);
                    }
                    None => {
                        c.text_with_face(&title_face, left, top, title, title_color);
                    }
                }
                c.pop_clip();
                if let Some(sb) = subtitle {
                    c.push_clip(Rect::new(left - hook, rect.y, rt.text_w + hook, rect.h));
                    let sub_face = rt.sub_font.sized(ctx.scale);
                    if beside {
                        let w = rt.sub_w - ctx.px(tokens::GAP);
                        let x = left + rt.text_w - w;
                        let y = rect.y + (rect.h - sub_face.line_height()) / 2;
                        draw_glyphed(c, ctx, &sub_face, x, y, sb, sub_color, value_glyph);
                    } else {
                        draw_glyphed(
                            c,
                            ctx,
                            &sub_face,
                            left,
                            top + rt.title_h + ctx.px(tokens::STACKED_LINE_GAP),
                            sb,
                            sub_color,
                            value_glyph,
                        );
                    }
                    c.pop_clip();
                }
            }
            Widget::LabelledValue {
                label,
                value,
                mono,
                glyph,
            } => {
                let small = Font::regular(tokens::CAPTION);
                let label_h = small.sized(ctx.scale).line_height();
                c.text(rect.x, rect.y, label, small, theme.muted);
                let font = if *mono {
                    Font::mono(tokens::MONO)
                } else {
                    Font::regular(tokens::BODY)
                };
                if glyph.is_some() {
                    let face = font.sized(ctx.scale);
                    c.push_clip(rect);
                    draw_glyphed(
                        c,
                        ctx,
                        &face,
                        rect.x,
                        rect.y + label_h,
                        value,
                        theme.text,
                        *glyph,
                    );
                    c.pop_clip();
                    return;
                }
                c.text_wrapped(
                    Rect::new(rect.x, rect.y + label_h, rect.w, rect.h - label_h),
                    value,
                    font,
                    theme.text,
                    TextAlign::Start,
                );
            }
            Widget::CandidateStrip {
                id,
                words,
                more,
                selected,
                outlined,
            } => {
                let n = i32::from(candidate_count(self, ctx, rect.w));
                if n == 0 {
                    return;
                }
                let hanzi = one_character(words);
                let cells = i32::from(candidate_cells(self, ctx, rect.w));
                // A one-character strip tiles its width like the keyboard
                // under it — no gap, a face inset from each cell — so a
                // cell is a key wide and the gaps people see are still
                // tappable.
                let gap = if hanzi { 0 } else { ctx.px(CANDIDATE_GAP) };
                let cell_w = (rect.w - gap * (cells - 1)) / cells;
                // The strip's size is the largest at which a word of
                // `CANDIDATE_FIT_CHARS` letters fills its cell, up to the
                // class's own; a longer word — Italian has nine-letter
                // ones — takes the size at which it fits, in its own cell
                // alone.
                let room = cell_w - 2 * ctx.px(CANDIDATE_PAD);
                let font = if hanzi {
                    candidate_font(ctx.class)
                } else {
                    let probe = "0".repeat(tokens::CANDIDATE_FIT_CHARS);
                    fit_font(c, &probe, candidate_font(ctx.class), room)
                };
                let radius = ctx.px(tokens::RADIUS_SMALL) as f32;
                let pressed = state.pressed_cell(*id);
                let pressed = |i: usize| {
                    pressed
                        .filter(|(cell, _)| *cell == i as u8)
                        .map(|(_, look)| look)
                };
                for (i, w) in words.iter().take(n as usize).enumerate() {
                    let cell =
                        Rect::new(rect.x + i as i32 * (cell_w + gap), rect.y, cell_w, rect.h);
                    let face = if hanzi {
                        cell.inset(crate::geom::PxEdges {
                            left: ctx.px(tokens::KEY_FACE_INSET),
                            top: ctx.px(tokens::KEY_FACE_INSET_Y),
                            right: ctx.px(tokens::KEY_FACE_INSET),
                            bottom: ctx.px(tokens::KEY_FACE_INSET_Y),
                        })
                    } else {
                        cell
                    };
                    // A selected cell is a chosen cell: the accent fill
                    // and its own text colour, which is what carries at
                    // the panel's 24 px cell.
                    let chosen = *selected == Some(i as u8);
                    // §16.120: a cell under a finger shows it, the
                    // selected one included — on the panel the cell
                    // being tapped is often the selected one.
                    let (fill, ink) = match (chosen, pressed(i)) {
                        (true, None) => (theme.primary, theme.on_primary),
                        (true, Some(look)) => {
                            (pressed_accent(theme.primary, look), theme.on_primary)
                        }
                        (false, Some(look)) => {
                            (pressed_neutral(theme.surface, theme, look), theme.text)
                        }
                        (false, None) => (theme.surface, theme.text),
                    };
                    c.fill_rounded_rect(face, radius, fill);
                    // §4.3: the lone candidate is outlined where a tap
                    // accepts, so the strip says what one tap takes
                    // without reading as a chosen option.
                    if *outlined == Some(i as u8) && !chosen {
                        c.stroke_rounded_rect(
                            face,
                            radius,
                            ctx.px(2.0).max(1) as f32,
                            theme.primary,
                        );
                    }
                    // The last cell pages when more candidates follow
                    // than the strip can hold.
                    if *more && i as i32 == n - 1 {
                        c.icon(face, Icon::ChevronRight, tokens::ICON_SMALL, ink);
                        continue;
                    }
                    let font = if hanzi {
                        font
                    } else {
                        fit_font(c, w, font, room)
                    };
                    c.push_clip(face);
                    c.text_in(face, w, font, ink, TextAlign::Center);
                    c.pop_clip();
                }
            }
            Widget::Keyboard {
                id,
                kind,
                enabled,
                scramble,
            } => draw_keyboard(
                c,
                rect,
                ctx,
                theme,
                *kind,
                *enabled,
                *scramble,
                state.modifiers(),
                state.pressed_key(*id),
            ),
            Widget::Luma {
                width,
                height,
                pixels,
                chroma,
            } => c.luma(
                rect,
                *width,
                *height,
                pixels,
                chroma.as_ref().map(|uv| uv.as_slice()),
            ),
            Widget::QrGrid {
                matrix,
                origin,
                span,
                ..
            } => {
                if self.qr_visible(state) {
                    draw_qr_grid(c, rect, ctx, theme, matrix, *origin, *span);
                }
            }
            Widget::Qr { matrix, .. } => {
                // A masked secret code draws nothing at all: §4.10 gives
                // the masked panel "one centred eye-slash icon and
                // nothing else", and the screen lays that icon over this
                // rectangle. A grey square with a code's own mark in it
                // would be a second idiom for the same state.
                if self.qr_visible(state) {
                    draw_qr(c, rect, matrix);
                }
            }
        }
        // §4.15: the focused item is outlined by a ring just inside its
        // own rectangle, at its own radius, in the accent or, on an
        // accent-filled item, in the accent's text colour, and is
        // otherwise drawn exactly as it is without focus.
        if let Some(id) = self.focus_id()
            && state.is_focused(id)
        {
            c.stroke_rounded_rect(
                rect,
                self.focus_radius(ctx, rect),
                ctx.px(tokens::FOCUS_STROKE).max(1) as f32,
                self.focus_ring_color(theme),
            );
        }
    }
}

/// The face one line of `text` is finally drawn in, `width` pixels
/// across: the type ramp walked down step by step until the line fits,
/// as a button label is.
///
/// The ladder starts under the font's own size, whatever that size is,
/// so a line set below the body token — the steel rows' monospace — has
/// steps left to take and takes them rather than being cut.
fn one_line_face(ctx: &LayoutCtx, text: &str, font: Font, width: i32) -> crate::fonts::SizedFace {
    one_line_face_glyphed(ctx, text, font, width, None)
}

/// [`one_line_face`] for a line with a glyph before it, measured with
/// the glyph.
fn one_line_face_glyphed(
    ctx: &LayoutCtx,
    text: &str,
    font: Font,
    width: i32,
    glyph: Option<Icon>,
) -> crate::fonts::SizedFace {
    let mut font = font;
    for size in [tokens::BODY, tokens::LABEL, tokens::CAPTION] {
        let face = font.sized(ctx.scale);
        if glyphed_width(ctx, &face, text, glyph) <= width {
            break;
        }
        if size < font.size.0 {
            font = font.with_size(size);
        }
    }
    font.sized(ctx.scale)
}

/// The size in pixels one line of `text` takes in a box `width` across,
/// after the ramp has been walked down for it. A layout test reads it to
/// tell a line that is drawn smaller from a line that is cut off.
pub fn one_line_size(ctx: &LayoutCtx, text: &str, font: Font, width: i32) -> crate::geom::Size {
    let face = one_line_face(ctx, text, font, width);
    let m = text::measure(&face, text);
    crate::geom::Size::new(m.width, m.height)
}

/// Draws one line of text in `rect`, without ever cutting a glyph in
/// half. A line that is too wide drops a text size first, as a button
/// label does; a line aligned to the end — the value in a text field —
/// then keeps its tail, whole characters at a time, which is how a field
/// scrolls while it is typed into. The clip is the last resort.
#[allow(clippy::too_many_arguments)]
fn draw_one_line(
    c: &mut Canvas,
    rect: Rect,
    ctx: &LayoutCtx,
    text: &str,
    font: Font,
    color: Color,
    align: TextAlign,
    glyph: Option<Icon>,
) {
    let face = one_line_face_glyphed(ctx, text, font, rect.w, glyph);
    if glyph.is_some() {
        let w = glyphed_width(ctx, &face, text, glyph);
        let x = match align {
            TextAlign::Start => rect.x,
            TextAlign::Center => rect.x + (rect.w - w) / 2,
            TextAlign::End => rect.right() - w,
        };
        c.push_clip(rect);
        draw_glyphed(c, ctx, &face, x, rect.y, text, color, glyph);
        c.pop_clip();
        return;
    }
    // A value that is still too wide keeps its end: drop leading
    // characters until what is left fits.
    let shown = if align == TextAlign::End && text::width(&face, text) > rect.w {
        let mut start = 0;
        for (i, _) in text.char_indices() {
            start = i;
            if text::width(&face, &text[i..]) <= rect.w {
                break;
            }
        }
        &text[start..]
    } else {
        text
    };
    let w = runs_width(&face, shown);
    let x = match align {
        TextAlign::Start => rect.x,
        TextAlign::Center => rect.x + (rect.w - w) / 2,
        TextAlign::End => rect.right() - w,
    };
    // A single line never spills over its neighbours: an app bar title
    // stops at the back control.
    c.push_clip(rect);
    draw_runs(c, &face, x, rect.y, shown, color);
    c.pop_clip();
}

/// The fingerprints inside a line of text: byte ranges of eight lower-
/// case hex characters with a digit among them, standing alone between
/// characters that are not letters or digits — "19e4837c" in "Opens
/// 19e4837c, not 32534671". Empty for a line with none.
fn fingerprint_spans(line: &str) -> Vec<(usize, usize)> {
    let bytes = line.as_bytes();
    let alnum = |i: usize| bytes.get(i).is_some_and(u8::is_ascii_alphanumeric);
    let mut spans = Vec::new();
    let mut i = 0;
    while i + 8 <= bytes.len() {
        let word = &bytes[i..i + 8];
        let hex = word
            .iter()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(b));
        if hex && word.iter().any(u8::is_ascii_digit) && (i == 0 || !alnum(i - 1)) && !alnum(i + 8)
        {
            spans.push((i, i + 8));
            i += 8;
        } else {
            i += 1;
        }
    }
    spans
}

/// The face a fingerprint inside a line in `face` is drawn in: the mono
/// face at the same size, since a fingerprint is read character by
/// character wherever it appears (`docs/DESIGN.md` §3). `None` where the
/// line is already mono, or is not text.
fn run_mono_face(face: &crate::fonts::SizedFace) -> Option<crate::fonts::SizedFace> {
    use crate::fonts::Family;
    matches!(
        face.family,
        Family::Regular | Family::SemiBold | Family::RegularNarrow | Family::SemiBoldNarrow
    )
    .then(|| crate::fonts::SizedFace::new(Family::Mono, face.px))
}

/// Width of one line whose fingerprints are drawn in mono.
fn runs_width(face: &crate::fonts::SizedFace, line: &str) -> i32 {
    let (Some(mono), spans) = (run_mono_face(face), fingerprint_spans(line)) else {
        return text::width(face, line);
    };
    if spans.is_empty() {
        return text::width(face, line);
    }
    let mut w = 0;
    let mut at = 0;
    for (a, b) in spans {
        w += text::width(face, &line[at..a]) + text::width(&mono, &line[a..b]);
        at = b;
    }
    w + text::width(face, &line[at..])
}

/// Draws one line with its fingerprints in the mono face, every run on
/// the line's one baseline.
fn draw_runs(
    c: &mut Canvas,
    face: &crate::fonts::SizedFace,
    x: i32,
    y: i32,
    line: &str,
    color: Color,
) {
    let spans = fingerprint_spans(line);
    let Some(mono) = run_mono_face(face).filter(|_| !spans.is_empty()) else {
        c.text_with_face(face, x, y, line, color);
        return;
    };
    let baseline = y + face.ascent;
    let mono_y = baseline - mono.ascent;
    let mut pen = x;
    let mut at = 0;
    for (a, b) in spans {
        pen += c.text_with_face(face, pen, y, &line[at..a], color);
        pen += c.text_with_face(&mono, pen, mono_y, &line[a..b], color);
        at = b;
    }
    c.text_with_face(face, pen, y, &line[at..], color);
}

/// The side of the square a value's glyph is drawn in: the em of the
/// value's face, so the glyph stands as tall as the characters after it.
fn glyph_box(face: &crate::fonts::SizedFace) -> i32 {
    i32::from(face.px)
}

/// How wide `value` is in `face` with `glyph` before it and before each
/// of its parts ([`VALUE_SEPARATOR`]): the width a row or a cell has to
/// hold (`docs/PLANNING.md` §16.131 rule 3).
fn glyphed_width(
    ctx: &LayoutCtx,
    face: &crate::fonts::SizedFace,
    value: &str,
    glyph: Option<Icon>,
) -> i32 {
    let w = text::width(face, value);
    match glyph {
        None => w,
        Some(_) => {
            let parts = value.split(VALUE_SEPARATOR).count() as i32;
            w + parts * (glyph_box(face) + ctx.px(tokens::GAP_SMALL))
        }
    }
}

/// Draws `value` on one line with the top of its line box at `y`, and
/// `glyph` before it and before each of its parts, one small gap before
/// the first character, in the value's colour (`docs/PLANNING.md`
/// §16.131 rule 1). Returns the width drawn.
#[allow(clippy::too_many_arguments)]
fn draw_glyphed(
    c: &mut Canvas,
    ctx: &LayoutCtx,
    face: &crate::fonts::SizedFace,
    x: i32,
    y: i32,
    value: &str,
    color: Color,
    glyph: Option<Icon>,
) -> i32 {
    let Some(g) = glyph else {
        return c.text_with_face(face, x, y, value, color);
    };
    let side = glyph_box(face);
    let glyph_dp = side as f32 / ctx.scale.factor();
    let top = y + (face.line_height() - side) / 2;
    let mut pen = x;
    for (i, part) in value.split(VALUE_SEPARATOR).enumerate() {
        if i > 0 {
            pen += c.text_with_face(face, pen, y, VALUE_SEPARATOR, color);
        }
        c.icon(Rect::new(pen, top, side, side), g, glyph_dp, color);
        pen += side + ctx.px(tokens::GAP_SMALL);
        pen += c.text_with_face(face, pen, y, part, color);
    }
    pen - x
}

/// Smallest type size in dp a chunked string is drawn at
/// ([`tokens::CHUNK_MIN_SIZE`]).
pub const CHUNK_MIN_SIZE: f32 = tokens::CHUNK_MIN_SIZE;

/// Least module pitch in millimetres a phone camera reads first time
/// ([`tokens::QR_MIN_PITCH_MM`]).
pub const QR_MIN_PITCH_MM: f32 = tokens::QR_MIN_PITCH_MM;

/// How wide one module of a `modules`-square code is, in millimetres,
/// when the code and its quiet zone are drawn in a square of `side_px`
/// on a `dpi` display.
pub fn qr_module_pitch_mm(side_px: i32, modules: usize, dpi: u16) -> f32 {
    let (_, _, scale) = qr_geometry(Rect::new(0, 0, side_px, side_px), modules);
    scale as f32 * MM_PER_INCH / f32::from(dpi.max(1))
}

/// The largest QR version whose modules, with the quiet zone round them,
/// keep the module pitch at or above [`QR_MIN_PITCH_MM`] in a square of
/// `side_px` on a `dpi` display; `None` when even version 1 is below the
/// floor there.
///
/// This is the arithmetic behind §4.9's "the part count follows from the
/// side and the floor, not from a fixed fragment size": a payload is
/// split until every part encodes inside the version this returns.
pub fn qr_max_version_at_pitch(side_px: i32, dpi: u16) -> Option<u8> {
    // Whole pixels per module the floor asks for on this display; a
    // module is drawn at a whole number of pixels ([`qr_geometry`]), so
    // the requirement rounds up. `f32::ceil` needs std, which this crate
    // does not have.
    let needed = QR_MIN_PITCH_MM * f32::from(dpi.max(1)) / MM_PER_INCH;
    let truncated = needed as i32;
    let per_module = truncated + i32::from(needed > truncated as f32);
    let total = side_px / per_module.max(1);
    let modules = total - 2 * QUIET_ZONE as i32;
    // Version v is 4v + 17 modules square; version 1 is 21.
    let version = (modules - 17) / 4;
    (1..=40).contains(&version).then_some(version as u8)
}

/// Millimetres in an inch ([`tokens::MM_PER_INCH`]).
const MM_PER_INCH: f32 = tokens::MM_PER_INCH;

/// The square a QR code occupies inside `rect`: `(origin x, origin y,
/// pixels per module)` for the largest whole-pixel scale at which the
/// symbol and its quiet zone fit, centred.
pub fn qr_geometry(rect: Rect, modules: usize) -> (i32, i32, i32) {
    let total = (modules + 2 * QUIET_ZONE) as i32;
    let scale = (rect.w.min(rect.h) / total.max(1)).max(1);
    let side = total * scale;
    // A rect too small for one pixel per module clips at the bottom right
    // rather than shifting the code off the top left.
    (
        rect.x + (rect.w - side).max(0) / 2,
        rect.y + (rect.h - side).max(0) / 2,
        scale,
    )
}

/// The row label of module row `i`: `A`…`Z`, then `AA`, `AB`, … for a
/// symbol with more than twenty-six rows (version 2 upwards).
pub fn qr_grid_row_label(i: usize) -> alloc::string::String {
    let mut out = alloc::string::String::new();
    if i >= 26 {
        out.push((b'A' + (i / 26 - 1) as u8) as char);
    }
    out.push((b'A' + (i % 26) as u8) as char);
    out
}

/// Draws a QR as a grid to be copied by hand: the white square, the
/// modules as filled cells, a thin line between every two of them, a
/// thick line every [`tokens::QR_GRID_BAND`] modules counted from the
/// symbol's own origin so that two pages line up, and the row and column
/// labels in the gutter along the top and left edges.
fn draw_qr_grid(
    c: &mut Canvas,
    rect: Rect,
    ctx: &LayoutCtx,
    theme: &Theme,
    matrix: &QrMatrix,
    origin: (u16, u16),
    span: u16,
) {
    let n = matrix.size();
    let ox = usize::from(origin.0).min(n);
    let oy = usize::from(origin.1).min(n);
    let span = usize::from(span)
        .min(n - ox.min(n))
        .min(n - oy.min(n))
        .max(1);
    // One cell of gutter for the labels, then one per module.
    let cells = (span + 1) as i32;
    let cell = (rect.w.min(rect.h) / cells).max(1);
    let side = cell * cells;
    let gx = rect.x + (rect.w - side).max(0) / 2;
    let gy = rect.y + (rect.h - side).max(0) / 2;
    let x0 = gx + cell;
    let y0 = gy + cell;
    let inner = cell * span as i32;
    c.fill_rect(Rect::new(x0, y0, inner, inner), Color::WHITE);
    for y in 0..span {
        for x in 0..span {
            if matrix.module(ox + x, oy + y) {
                c.fill_rect(
                    Rect::new(x0 + x as i32 * cell, y0 + y as i32 * cell, cell, cell),
                    Color::BLACK,
                );
            }
        }
    }
    // The lines are drawn over the modules: a filled square keeps its
    // grid, which is what makes a hand-drawn copy checkable square by
    // square.
    let thin = Color::BLACK.mix(Color::WHITE, tokens::QR_GRID_LINE_MIX);
    let thin_w = ctx.px(tokens::QR_GRID_LINE).max(1);
    let guide_w = ctx.px(tokens::QR_GRID_GUIDE).max(1);
    for i in 0..=span {
        let at = i + ox;
        let (w, color) = if at % tokens::QR_GRID_BAND == 0 || i == span {
            (guide_w, Color::BLACK)
        } else {
            (thin_w, thin)
        };
        let x = x0 + i as i32 * cell - w / 2;
        c.fill_rect(Rect::new(x, y0, w, inner), color);
    }
    for j in 0..=span {
        let at = j + oy;
        let (w, color) = if at % tokens::QR_GRID_BAND == 0 || j == span {
            (guide_w, Color::BLACK)
        } else {
            (thin_w, thin)
        };
        let y = y0 + j as i32 * cell - w / 2;
        c.fill_rect(Rect::new(x0, y, inner, w), color);
    }
    // The labels sit in a gutter one cell wide, so they take the
    // largest size from the ramp at which the widest of them — "25",
    // "AA" — fits a cell. A 29-module symbol drawn whole has cells too
    // narrow for the label token, and a label that spills into its
    // neighbour is worse than a small one.
    let column = alloc::format!("{}", ox + span);
    let row = qr_grid_row_label(oy + span - 1);
    let mut font = Font::mono(tokens::QR_GRID_LABEL);
    let room = cell - ctx.px(tokens::QR_GRID_LABEL_GAP);
    for size in [tokens::QR_GRID_LABEL, tokens::QR_GRID_LABEL_SMALL] {
        font = font.with_size(size);
        let face = font.sized(ctx.scale);
        if text::width(&face, &column).max(text::width(&face, &row)) <= room {
            break;
        }
    }
    for i in 0..span {
        let label = alloc::format!("{}", ox + i + 1);
        c.text_in(
            Rect::new(x0 + i as i32 * cell, gy, cell, cell),
            &label,
            font,
            theme.muted,
            TextAlign::Center,
        );
        let label = qr_grid_row_label(oy + i);
        c.text_in(
            Rect::new(gx, y0 + i as i32 * cell, cell, cell),
            &label,
            font,
            theme.muted,
            TextAlign::Center,
        );
    }
}

/// Draws the light square and the dark modules, one rectangle per run of
/// dark modules in a row rather than one per module.
fn draw_qr(c: &mut Canvas, rect: Rect, matrix: &QrMatrix) {
    let n = matrix.size();
    let (x0, y0, scale) = qr_geometry(rect, n);
    let side = (n + 2 * QUIET_ZONE) as i32 * scale;
    c.fill_rect(Rect::new(x0, y0, side, side), Color::WHITE);
    let origin_x = x0 + QUIET_ZONE as i32 * scale;
    let origin_y = y0 + QUIET_ZONE as i32 * scale;
    for y in 0..n {
        let mut x = 0;
        while x < n {
            if !matrix.module(x, y) {
                x += 1;
                continue;
            }
            let start = x;
            while x < n && matrix.module(x, y) {
                x += 1;
            }
            c.fill_rect(
                Rect::new(
                    origin_x + start as i32 * scale,
                    origin_y + y as i32 * scale,
                    (x - start) as i32 * scale,
                    scale,
                ),
                Color::BLACK,
            );
        }
    }
}

/// The scrollbar thumb for a scroll region, or `None` when the content
/// fits. `view` is the viewport, `content_h` the content height and
/// `offset` the applied scroll offset, all in pixels. The track sits at
/// the right edge of the viewport; the thumb's height is the visible
/// fraction of the content, never shorter than
/// [`tokens::SCROLLBAR_MIN_THUMB`].
pub fn scrollbar_thumb(view: Rect, content_h: i32, offset: i32, ctx: &LayoutCtx) -> Option<Rect> {
    if content_h <= view.h || view.h <= 0 {
        return None;
    }
    let w = ctx.px(tokens::SCROLLBAR).max(2);
    let min_thumb = ctx.px(tokens::SCROLLBAR_MIN_THUMB).min(view.h);
    let thumb_h = ((view.h as i64 * view.h as i64 / content_h as i64) as i32).max(min_thumb);
    let max_offset = content_h - view.h;
    let travel = view.h - thumb_h;
    let y =
        view.y + (travel as i64 * offset.clamp(0, max_offset) as i64 / max_offset as i64) as i32;
    Some(Rect::new(view.right() - w, y, w, thumb_h))
}

/// The column a scroll region's scrollbar draws in: the track the thumb
/// travels down, at the right edge of `view`. What a caller repainting
/// only what a scroll moved has to draw again, wherever the thumb was
/// and wherever it now is.
pub fn scrollbar_track(view: Rect, ctx: &LayoutCtx) -> Rect {
    let w = ctx.px(tokens::SCROLLBAR).max(2);
    Rect::new(view.right() - w, view.y, w, view.h)
}

/// Draws `icon` centred in `rect` at the baked size nearest `size_dp`.
fn draw_icon(c: &mut Canvas, rect: Rect, _ctx: &LayoutCtx, icon: Icon, size_dp: f32, color: Color) {
    c.icon(rect, icon, size_dp, color);
}

/// The chip's own label size when its row chose one, the family's
/// default otherwise.
fn chip_font(font: Font, label_dp: Option<f32>) -> Font {
    match label_dp {
        Some(dp) => font.with_size(dp),
        None => font,
    }
}

/// `(font, horizontal padding dp, height dp)` of a chip. A chip that is
/// tapped is at least `tokens::touch_floor` tall; the compact chip is a
/// label with a rectangle around it and keeps its own height.
fn chip_metrics(compact: bool, class: SizeClass) -> (Font, f32, f32) {
    if compact {
        (
            Font::mono(tokens::CAPTION),
            tokens::CHIP_COMPACT_PAD,
            tokens::CHIP_COMPACT,
        )
    } else {
        (
            Font::semibold(tokens::LABEL),
            tokens::CHIP_PAD,
            tokens::touch_floor(class),
        )
    }
}

/// The fill a neutral surface takes under a finger: its own resting fill
/// tinted towards the accent, halved once the linger's first step has
/// passed (`docs/PLANNING.md` §16.124 rule 1). A key, a candidate cell,
/// a row, an unselected chip, a tile, a secondary button, a Text button's
/// background, the pager's arrows — all of them through here, so the
/// pressed look is one rule.
fn pressed_neutral(fill: Color, theme: &Theme, look: PressLook) -> Color {
    fill.mix(theme.primary, look.of(tokens::PRESSED_TINT))
}

/// The fill a control drawn on the background takes under a finger: a
/// Text button, the app bar's icon buttons and its eye. The black mixed
/// towards the accent is a dark brown that barely shows, so the tint
/// starts from the raised surface instead (`docs/PLANNING.md` §16.132
/// rule 2).
fn pressed_on_background(theme: &Theme, look: PressLook) -> Color {
    pressed_neutral(theme.surface_raised, theme, look)
}

/// The fill an accent surface takes under a finger: its own fill mixed
/// further towards white, halved on the linger's second step
/// (`docs/PLANNING.md` §16.124 rule 2). The primary button, ✓, a selected
/// candidate cell, a selected chip, the danger button.
fn pressed_accent(fill: Color, look: PressLook) -> Color {
    fill.mix(Color::WHITE, look.of(tokens::PRESSED_MIX))
}

#[allow(clippy::too_many_arguments)]
fn draw_button(
    c: &mut Canvas,
    rect: Rect,
    ctx: &LayoutCtx,
    theme: &Theme,
    label: &str,
    style: ButtonStyle,
    enabled: bool,
    pressed: Option<PressLook>,
) {
    let radius = ctx.px(tokens::RADIUS) as f32;
    let (fill, text) = match style {
        ButtonStyle::Primary => (theme.primary, theme.on_primary),
        ButtonStyle::Secondary => (theme.surface, theme.text),
        ButtonStyle::Danger => (theme.danger, Color::WHITE),
        ButtonStyle::Text => (theme.background, theme.primary),
        ButtonStyle::TextMuted => (theme.background, theme.muted),
    };
    // §16.124: the accent buttons mix towards white from their own fill,
    // the neutral ones take the accent tint.
    let accent = matches!(style, ButtonStyle::Primary | ButtonStyle::Danger);
    let on_background = matches!(style, ButtonStyle::Text | ButtonStyle::TextMuted);
    let (fill, text) = match pressed {
        _ if !enabled => (theme.surface, theme.outline),
        Some(look) if accent => (pressed_accent(fill, look), text),
        Some(look) if on_background => (pressed_on_background(theme, look), text),
        Some(look) => (pressed_neutral(fill, theme, look), text),
        None => (fill, text),
    };
    if !on_background || pressed.is_some() {
        c.fill_rounded_rect(rect, radius, fill);
    }
    let font = button_label_font(ctx, label, rect.w);
    c.push_clip(rect);
    c.text_in(rect, label, font, text, TextAlign::Center);
    c.pop_clip();
}

/// The face a button draws its label in (`docs/DESIGN.md` §4.13: "Both
/// buttons use one label font per class").
///
/// One size for every button, so the two buttons of a pair read as a
/// pair. A label that does not fit its button is shortened by the screen
/// that names it — "Numbers" — and the two steps below it are the last
/// resort that keeps a long label on the button rather than off its
/// edge. A layout test asks this what a pair of buttons will draw.
pub fn button_label_font(ctx: &LayoutCtx, label: &str, width_px: i32) -> Font {
    let room = width_px - 2 * ctx.px(tokens::GAP);
    let mut font = Font::semibold(tokens::BODY);
    for size in [tokens::LABEL, tokens::CAPTION] {
        if text::width(&font.sized(ctx.scale), label) <= room {
            break;
        }
        font = font.with_size(size);
    }
    font
}

/// A hold button: an outlined surface that the accent fills from the left
/// as the hold progresses (UX.md §6). The label is drawn twice, once in
/// each state, so it stays readable on both sides of the filling edge.
/// Width in dp of a switch's track ([`tokens::SWITCH_WIDTH`]).
const SWITCH_WIDTH: f32 = tokens::SWITCH_WIDTH;
/// Height in dp of a switch's track ([`tokens::SWITCH_HEIGHT`]).
const SWITCH_HEIGHT: f32 = tokens::SWITCH_HEIGHT;
/// Inset in dp of a switch's knob inside its track
/// ([`tokens::SWITCH_KNOB_INSET`]).
const SWITCH_KNOB_INSET: f32 = tokens::SWITCH_KNOB_INSET;

/// A switch: a rounded track with the knob at one end. `enabled` is
/// false on a control the screen has decided for the person; the row it
/// sits in says why (`docs/DESIGN.md` §4.9, §4.11).
fn draw_switch(
    c: &mut Canvas,
    track: Rect,
    ctx: &LayoutCtx,
    theme: &Theme,
    on: bool,
    enabled: bool,
) {
    let fill = match (on, enabled) {
        (true, true) => theme.primary,
        (true, false) => theme.muted,
        (false, _) => theme.surface_raised,
    };
    c.fill_rounded_rect(track, track.h as f32 / 2.0, fill);
    let inset = ctx.px(SWITCH_KNOB_INSET);
    let knob_r = (track.h - 2 * inset) as f32 / 2.0;
    let cx = if on {
        track.right() as f32 - knob_r - inset as f32
    } else {
        track.x as f32 + knob_r + inset as f32
    };
    let knob = match (on, enabled) {
        (true, true) => theme.on_primary,
        (true, false) => theme.surface,
        (false, false) => theme.outline,
        (false, true) => theme.text,
    };
    c.fill_circle(cx, track.center().y as f32 + 0.5, knob_r, knob);
}

#[allow(clippy::too_many_arguments)]
fn draw_hold_button(
    c: &mut Canvas,
    rect: Rect,
    ctx: &LayoutCtx,
    theme: &Theme,
    label: &str,
    style: ButtonStyle,
    enabled: bool,
    progress: f32,
) {
    let radius = ctx.px(tokens::RADIUS) as f32;
    let accent = match (style, enabled) {
        (_, false) => theme.muted,
        (ButtonStyle::Danger, _) => theme.danger,
        _ => theme.primary,
    };
    let font = Font::semibold(tokens::BODY);
    c.fill_rounded_rect(rect, radius, theme.surface);
    c.stroke_rounded_rect(rect, radius, ctx.px(2.0).max(1) as f32, accent);
    c.push_clip(rect);
    let label_color = if enabled { theme.text } else { theme.muted };
    c.text_in(rect, label, font, label_color, TextAlign::Center);
    c.pop_clip();
    let w = (rect.w as f32 * progress.clamp(0.0, 1.0)) as i32;
    if w > 0 {
        let on = if style == ButtonStyle::Danger {
            Color::WHITE
        } else {
            theme.on_primary
        };
        c.push_clip(Rect::new(rect.x, rect.y, w, rect.h));
        c.fill_rounded_rect(rect, radius, accent);
        c.text_in(rect, label, font, on, TextAlign::Center);
        c.pop_clip();
    }
}

/// The candidate sizes for a chunked string, largest first, in dp.
fn chunk_sizes(max_size: f32, min_size: f32) -> Vec<f32> {
    let mut sizes = Vec::new();
    let mut s = max_size.max(min_size);
    loop {
        sizes.push(s);
        let next = s - tokens::CHUNK_STEP;
        if next < min_size {
            break;
        }
        s = next;
    }
    if sizes.is_empty() {
        sizes.push(min_size);
    }
    sizes
}

/// Plans a chunked string: which size, how many chunks per line, whether
/// to page. Returns the plan and the per-size `(chunk_w, gap, line_h)`.
fn chunk_plan(
    ctx: &LayoutCtx,
    text: &str,
    max_size: f32,
    min_size: f32,
    max: Size,
) -> (chunked::Plan, Vec<(i32, i32, i32)>) {
    let chunks = chunked::chunks(text);
    let n = chunks.len();
    // A chunked string measures by a full group of four; an unchunked one
    // by its own length, which is at most the threshold.
    let widest = chunks
        .iter()
        .map(|&(s, e)| text[s..e].chars().count())
        .max()
        .unwrap_or(chunked::CHUNK);
    let metrics: Vec<(i32, i32, i32)> = chunk_sizes(max_size, min_size)
        .into_iter()
        .map(|dp| {
            let face = Font::mono(dp).sized(ctx.scale);
            (
                text::width(&face, "0") * widest as i32,
                text::width(&face, " "),
                face.line_height(),
            )
        })
        .collect();
    let label_h = Font::regular(tokens::CAPTION)
        .sized(ctx.scale)
        .line_height();
    (chunked::plan(n, &metrics, max.w, max.h, label_h), metrics)
}

#[allow(clippy::too_many_arguments)]
fn draw_chunked(
    c: &mut Canvas,
    rect: Rect,
    ctx: &LayoutCtx,
    theme: &Theme,
    state: &UiState,
    id: Id,
    text: &str,
    max_size: f32,
    min_size: f32,
    center: bool,
) {
    let (plan, metrics) = chunk_plan(ctx, text, max_size, min_size, rect.size());
    let dp = chunk_sizes(max_size, min_size)[plan.size_index];
    let font = Font::mono(dp);
    let face = font.sized(ctx.scale);
    let chunks = chunked::chunks(text);
    // The width the plan measured a chunk at: a group of four, or the
    // whole of a string short enough not to be chunked, which centred as
    // four characters would run off the right edge.
    let (chunk_w, gap, _) = metrics[plan.size_index];
    let lh = face.line_height();

    let (first, last) = match plan.per_page {
        Some(per_page) => {
            let page = state.page(id).min(plan.pages.saturating_sub(1) as u8) as usize;
            let first = page * per_page;
            (first, (first + per_page).min(chunks.len()))
        }
        None => (0, chunks.len()),
    };
    // Where a line of `n` chunks starts, so that a centred string sits
    // under the value it belongs to rather than at the left margin.
    let shown = last - first;
    let line_start = |line: usize| {
        if !center {
            return rect.x;
        }
        let n = (shown - line * plan.per_line).min(plan.per_line) as i32;
        let w = n * chunk_w + (n - 1).max(0) * gap;
        rect.x + (rect.w - w).max(0) / 2
    };
    let mut x = line_start(0);
    let mut y = rect.y;
    for (i, &(s, e)) in chunks[first..last].iter().enumerate() {
        if i > 0 && i % plan.per_line == 0 {
            x = line_start(i / plan.per_line);
            y += lh;
        }
        let color = if (first + i) % 2 == 0 {
            theme.text
        } else {
            theme.primary
        };
        c.text_with_face(&face, x, y, &text[s..e], color);
        x += chunk_w + gap;
    }
    if let Some(per_page) = plan.per_page {
        let n = text.chars().count();
        let a = first * chunked::CHUNK + 1;
        let b = (last * chunked::CHUNK).min(n);
        let page = first / per_page + 1;
        let label = alloc::format!("chars {a}-{b} of {n} · page {page}/{}", plan.pages);
        let small = Font::regular(tokens::CAPTION);
        let label_h = small.sized(ctx.scale).line_height();
        c.text(rect.x, rect.bottom() - label_h, &label, small, theme.muted);
    }
}

/// Where a list row's text goes, from [`Widget::row_text`]. Offsets are
/// from the row's left edge.
#[derive(Debug, Clone, Copy)]
struct RowText {
    /// The padding at each side: a step on a surface, none on a flat row.
    pad: i32,
    /// Where the title and the value start.
    left: i32,
    /// The width the title and the value share.
    text_w: i32,
    /// The width the title takes: all of it, less a value beside it.
    title_w: i32,
    /// The title's face after any step down.
    title_font: Font,
    /// Where a title on two lines breaks: the end of the first line and
    /// the start of the second.
    title_break: Option<(usize, usize)>,
    /// The height of the title's line or lines.
    title_h: i32,
    /// The value's face after any step down.
    sub_font: Font,
    /// The height of the value's line at its own size, before any step
    /// down, which is what the row's two lines are centred on.
    sub_line_h: i32,
    /// The width a value beside the title takes, with the gap before it.
    sub_w: i32,
}

impl Default for RowText {
    fn default() -> Self {
        RowText {
            pad: 0,
            left: 0,
            text_w: 0,
            title_w: 0,
            title_font: Font::regular(tokens::LABEL),
            title_break: None,
            title_h: 0,
            sub_font: Font::regular(tokens::LABEL),
            sub_line_h: 0,
            sub_w: 0,
        }
    }
}

/// The size in dp and the side in pixels of a row's leading icon: the
/// dense size on a row shorter than a menu row.
fn row_icon(ctx: &LayoutCtx, height_dp: f32) -> (f32, i32) {
    let size = if height_dp < tokens::MENU_ROW {
        tokens::ICON_DENSE
    } else {
        tokens::ICON
    };
    (size, ctx.px(size))
}
/// The second and last line of a row's title: whole where it fits
/// `width`, and otherwise the words that fit before an ellipsis, so that
/// a title never ends cut at the row's edge. A single word too long for
/// the line keeps its characters up to the ellipsis.
fn last_line(face: &crate::fonts::SizedFace, line: &str, width: i32) -> String {
    if text::width(face, line) <= width {
        return String::from(line);
    }
    let room = width - text::width(face, tokens::ELLIPSIS);
    let words = line
        .rmatch_indices(' ')
        .map(|(i, _)| &line[..i])
        .find(|head| text::width(face, head.trim_end()) <= room)
        .map(str::trim_end);
    let head = words.unwrap_or_else(|| {
        let mut end = 0;
        for (i, ch) in line.char_indices() {
            if text::width(face, &line[..i + ch.len_utf8()]) > room {
                break;
            }
            end = i + ch.len_utf8();
        }
        &line[..end]
    });
    alloc::format!("{head}{}", tokens::ELLIPSIS)
}

/// The face a row's first line takes, which measuring and drawing both
/// ask for so they agree on the line's height. A row that carries a
/// value has the muted label there; a plain row has the name of the
/// thing, in the mono face when that name is read character by
/// character (§16.126).
fn row_title_font(has_value: bool, mono_title: bool) -> Font {
    match (has_value, mono_title) {
        (true, true) => Font::mono(tokens::LABEL),
        (true, false) => Font::regular(tokens::LABEL),
        (false, true) => Font::mono(tokens::BODY),
        (false, false) => Font::semibold(tokens::BODY),
    }
}

/// Gap between candidate cells in dp.
const CANDIDATE_GAP: f32 = tokens::CANDIDATE_GAP;

/// Padding inside a candidate cell in dp.
const CANDIDATE_PAD: f32 = tokens::CANDIDATE_PAD;

/// A candidate is a word read character by character, so it is set in
/// the mono face like everything else that is (§16.126).
/// The largest size up to `font`'s own at which `text` is no wider than
/// `room` pixels, stepping down by [`tokens::CANDIDATE_SIZE_STEP`] and
/// stopping at the caption size.
fn fit_font(c: &Canvas, text: &str, font: Font, room: i32) -> Font {
    let width = c.measure_text(text, font).width;
    if width <= room {
        return font;
    }
    // Width grows with size, so the first guess is the size in proportion,
    // and the steps after it make up the pixels rounding costs.
    let mut dp = (font.size.0 * room.max(0) as f32 / width.max(1) as f32).min(font.size.0);
    loop {
        let f = font.with_size(dp.max(tokens::CAPTION));
        if dp <= tokens::CAPTION || c.measure_text(text, f).width <= room {
            return f;
        }
        dp -= tokens::CANDIDATE_SIZE_STEP;
    }
}

fn candidate_font(class: SizeClass) -> Font {
    Font::mono(tokens::candidate_label(class))
}

/// Whether every word of `words` is one character, which is the two
/// Chinese lists: a character is read rather than spelled, so its cell
/// needs no more width than a key and the strip is
/// [`tokens::CANDIDATE_CELLS_HANZI`] cells wide rather than as wide as
/// its longest word.
pub fn one_character(words: &[String]) -> bool {
    !words.is_empty() && words.iter().all(|w| w.chars().count() == 1)
}

/// How many cells a candidate strip divides `width` into: its shown words
/// on a strip of words, and [`tokens::CANDIDATE_CELLS_HANZI`] on a
/// one-character strip, whose cell keeps the keyboard's key pitch however
/// few characters are left. Drawing and hit-testing both use this.
pub fn candidate_cells(widget: &Widget, ctx: &LayoutCtx, width: i32) -> u8 {
    match widget {
        Widget::CandidateStrip { words, .. } if one_character(words) => {
            tokens::CANDIDATE_CELLS_HANZI as u8
        }
        // §4.3: the grid is the same whatever the count, so a cell does
        // not change width from one word to the next. On `wide` the
        // strip is natural-width chips in a row and keeps one cell per
        // word.
        Widget::CandidateStrip { .. } if ctx.class != SizeClass::Wide => {
            tokens::candidates_per_row(ctx.class, false) as u8
        }
        _ => candidate_count(widget, ctx, width),
    }
}

/// How many of a candidate strip's words are shown in `width` pixels: the
/// largest leading run whose longest word fits an equal-width cell, and at
/// least one when there are any. Drawing and hit-testing both use this.
pub fn candidate_count(widget: &Widget, ctx: &LayoutCtx, width: i32) -> u8 {
    let Widget::CandidateStrip { words, .. } = widget else {
        return 0;
    };
    if one_character(words) {
        return words.len().min(tokens::CANDIDATE_CELLS_HANZI) as u8;
    }
    // A fixed grid fills its cells from the left and leaves the rest
    // blank; a word too long for a cell drops a size rather than
    // costing the strip a cell.
    if ctx.class != SizeClass::Wide {
        return words
            .len()
            .min(tokens::candidates_per_row(ctx.class, false)) as u8;
    }
    let face = candidate_font(ctx.class).sized(ctx.scale);
    let gap = ctx.px(CANDIDATE_GAP);
    let pad = 2 * ctx.px(CANDIDATE_PAD);
    let mut widest = 0;
    let mut shown = 0usize;
    for w in words.iter().take(255) {
        widest = widest.max(text::width(&face, w) + pad);
        let n = shown as i32 + 1;
        let cell = (width - gap * (n - 1)) / n;
        if widest > cell && shown > 0 {
            break;
        }
        shown += 1;
    }
    shown as u8
}

/// How a chunked string was laid out inside the rectangle it was given.
/// Layout tests read it to assert that the whole string is on the screen
/// and that each line carries every group the width holds.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChunkFit {
    /// The monospace size in dp the string was drawn at: the largest step
    /// of the ramp at which the whole of it fits (`docs/DESIGN.md` §5
    /// Transcribe).
    pub size_dp: f32,
    /// Groups of four on one line.
    pub per_line: usize,
    /// Lines shown at once.
    pub lines: usize,
    /// Groups in the whole string.
    pub chunks: usize,
    /// Pages the string was split into; 1 when it is shown whole.
    pub pages: usize,
    /// Width in pixels of one full group, at the size that was chosen.
    pub chunk_w: i32,
    /// Width in pixels of the space between two groups.
    pub gap: i32,
    /// Width in pixels the string had to lay out in.
    pub width: i32,
}

/// How the chunked string `widget` is laid out inside `rect`. `None` for
/// any other widget.
pub fn chunk_fit(widget: &Widget, ctx: &LayoutCtx, rect: Rect) -> Option<ChunkFit> {
    match widget {
        Widget::ChunkedString {
            text,
            max_size,
            min_size,
            ..
        } => {
            let (plan, metrics) = chunk_plan(ctx, text, *max_size, *min_size, rect.size());
            let (chunk_w, gap, _) = metrics[plan.size_index];
            Some(ChunkFit {
                size_dp: chunk_sizes(*max_size, *min_size)[plan.size_index],
                per_line: plan.per_line,
                lines: plan.lines,
                chunks: chunked::chunks(text).len(),
                pages: plan.pages,
                chunk_w,
                gap,
                width: rect.w,
            })
        }
        _ => None,
    }
}

/// Number of pages a chunked string needs inside `rect`; 1 when it fits.
pub fn chunked_pages(widget: &Widget, ctx: &LayoutCtx, rect: Rect) -> usize {
    match widget {
        Widget::ChunkedString {
            text,
            max_size,
            min_size,
            ..
        } => {
            chunk_plan(ctx, text, *max_size, *min_size, rect.size())
                .0
                .pages
        }
        _ => 1,
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_keyboard(
    c: &mut Canvas,
    rect: Rect,
    ctx: &LayoutCtx,
    theme: &Theme,
    kind: KeyboardKind,
    enabled: KeyMask,
    scramble: Option<u32>,
    mods: keyboard::Modifiers,
    pressed: Option<(KeyInput, PressLook)>,
) {
    let caps = keyboard::keys(kind, rect, ctx, enabled, scramble, mods);
    let radius = ctx.px(tokens::GAP) as f32;
    let inset = ctx.px(keyboard::KEY_FACE_INSET).max(1);
    let inset_y = ctx.px(keyboard::KEY_FACE_INSET_Y).max(1);
    let bottom_inset = ctx.px(tokens::key_bottom_inset(ctx.inset_bottom_dp));
    // §16.126: a keycap is one character read as a character, so every
    // keycap is mono. A kana, a jamo or a bopomofo falls back to the CJK
    // face, which is the one face those scripts have.
    let letter_font = match kind {
        KeyboardKind::Pin | KeyboardKind::Dice => Font::mono(tokens::KEYCAP_PAD),
        KeyboardKind::Hex | KeyboardKind::Path => Font::mono(tokens::KEYCAP_LETTER),
        KeyboardKind::Coin => Font::mono(tokens::KEYCAP_LETTER),
        // A kana or a jamo is about an em wide, and the kana grid is ten
        // columns, so its keycaps take the smaller of the two steps.
        KeyboardKind::Kana | KeyboardKind::Jamo => Font::mono(tokens::KEYCAP_DENSE),
        _ => Font::mono(if ctx.class == SizeClass::Small {
            tokens::KEYCAP_DENSE
        } else {
            tokens::KEYCAP_LETTER
        }),
    };
    let special_font = Font::mono(tokens::LABEL);
    for k in &caps {
        // The cell owns the hit area, which reaches the screen edges;
        // the face is inset from it, and the bottom row's face is inset
        // by 16 dp so the visible keyboard is not flush with the edge.
        let bottom = if k.rect.bottom() >= rect.bottom() {
            bottom_inset
        } else {
            inset_y
        };
        let face = Rect::new(
            k.rect.x + inset,
            k.rect.y + inset_y,
            (k.rect.w - 2 * inset).max(1),
            (k.rect.h - inset_y - bottom).max(1),
        );
        let icon = match k.input {
            KeyInput::Backspace => Some(Icon::Delete),
            KeyInput::Char(' ') => Some(Icon::Space),
            KeyInput::Done => Some(Icon::Done),
            // The jamo keyboard's shift key swaps five doubled
            // consonants and two vowels, which no three letters name.
            KeyInput::Shift if kind == KeyboardKind::Jamo => Some(Icon::ChevronUp),
            _ => None,
        };
        let (label, font): (String, Font) = match k.input {
            KeyInput::Char('H') if kind == KeyboardKind::Coin => {
                (String::from("Heads"), letter_font)
            }
            KeyInput::Char('T') if kind == KeyboardKind::Coin => {
                (String::from("Tails"), letter_font)
            }
            KeyInput::Char(ch) => (String::from(ch), letter_font),
            // The address keyboard's two special keys name alphabets
            // rather than cases: shift goes between bech32 and base58,
            // and the key beside the digits carries the case inside
            // base58.
            KeyInput::Shift if kind == KeyboardKind::Address => (
                String::from(if mods.shift { "bc1" } else { "1Ab" }),
                special_font,
            ),
            KeyInput::Symbols if kind == KeyboardKind::Address => (
                String::from(if mods.symbols { "abc" } else { "ABC" }),
                special_font,
            ),
            KeyInput::Shift => (
                String::from(if mods.shift { "ABC" } else { "abc" }),
                special_font,
            ),
            KeyInput::Symbols => (
                String::from(if mods.symbols { "abc" } else { "?#+" }),
                special_font,
            ),
            _ => (String::new(), special_font),
        };
        let fill = match (k.enabled, k.input) {
            (false, _) => theme.surface.mix(theme.background, 0.5),
            (_, KeyInput::Done) => theme.primary,
            (_, KeyInput::Backspace | KeyInput::Shift | KeyInput::Symbols) => theme.surface_raised,
            _ => theme.surface,
        };
        // §16.120, §16.124: the key under the finger, and the key a
        // finger just lifted from, take the accent tint from their own
        // fill — ✓, the accent key, mixes towards white instead.
        let look = pressed
            .filter(|(input, _)| *input == k.input && k.enabled)
            .map(|(_, look)| look);
        let fill = match look {
            Some(look) if k.input == KeyInput::Done => pressed_accent(fill, look),
            Some(look) => pressed_neutral(fill, theme, look),
            None => fill,
        };
        let color = match (k.enabled, k.input) {
            (false, _) => theme.muted.with_alpha(70),
            (_, KeyInput::Done) => theme.on_primary,
            _ => theme.text,
        };
        c.fill_rounded_rect(face, radius, fill);
        match icon {
            Some(i) => c.icon(face, i, tokens::ICON_SMALL, color),
            None => {
                c.push_clip(face);
                c.text_in(face, &label, font, color, TextAlign::Center);
                c.pop_clip();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geom::Scale;

    fn ctx() -> LayoutCtx {
        LayoutCtx::new(Scale::IDENTITY, SizeClass::Mobile)
    }

    /// The ring the keyboard leaves is on the focused item and on
    /// nothing else (`docs/DESIGN.md` §4.15).
    #[test]
    fn the_focused_button_is_the_one_with_the_ring_round_it() {
        use crate::canvas::Canvas;
        use crate::color::Theme;
        use crate::layout::{Node, solve};
        use crate::state::UiState;
        use osk_shell_api::{BootState, DisplayInfo, Event, Key, SecureHardware};

        let tree = Node::column()
            .child(Node::widget(Widget::button(Id(1), "One", ButtonStyle::Secondary)).height(48.0))
            .child(Node::widget(Widget::button(Id(2), "Two", ButtonStyle::Secondary)).height(48.0));
        let display = DisplayInfo {
            width: 200,
            height: 200,
            dpi: 160,
            inset_bottom: 0,
            inset_top: 0,
            buttons: 0,
            camera_fixed: true,
            secure: SecureHardware::None,
            boot: BootState::Unknown,
            memory_mib: None,
        };
        let theme = Theme::DARK;
        let mut c = Canvas::new(&display);
        let layout = solve(&tree, c.bounds(), &ctx());
        let mut state = UiState::new(Scale::IDENTITY);
        state.event(&layout, Event::Key(Key::Tab));

        c.clear(theme.background);
        crate::widgets::draw_tree(&mut c, &tree, &layout, &theme, &state);
        let accent = |rect: Rect| {
            let f = c.frame();
            (rect.y..rect.bottom())
                .flat_map(|y| (rect.x..rect.right()).map(move |x| (x, y)))
                .filter(|(x, y)| {
                    let i = (*y as usize * usize::from(f.width) + *x as usize) * 4;
                    f.rgba[i] == theme.primary.r
                        && f.rgba[i + 1] == theme.primary.g
                        && f.rgba[i + 2] == theme.primary.b
                })
                .count()
        };
        let focused = layout.rect(Id(1)).expect("the focused button");
        let plain = layout.rect(Id(2)).expect("the other button");
        assert!(accent(focused) > 0, "the focused button carries the ring");
        assert_eq!(accent(plain), 0, "and nothing else does");
    }

    /// A primary button is filled with the accent, so its ring is drawn
    /// in the accent's text colour: a person tabbing to Continue sees
    /// where they are (`docs/DESIGN.md` §4.15).
    #[test]
    fn the_ring_on_a_primary_button_is_visible_against_its_fill() {
        use crate::canvas::Canvas;
        use crate::color::Theme;
        use crate::layout::{Node, solve};
        use crate::state::UiState;
        use osk_shell_api::{BootState, DisplayInfo, Event, Key, SecureHardware};

        let tree = Node::column().child(
            Node::widget(Widget::button(Id(1), "Continue", ButtonStyle::Primary)).height(48.0),
        );
        let display = DisplayInfo {
            width: 200,
            height: 200,
            dpi: 160,
            inset_bottom: 0,
            inset_top: 0,
            buttons: 0,
            camera_fixed: true,
            secure: SecureHardware::None,
            boot: BootState::Unknown,
            memory_mib: None,
        };
        let theme = Theme::DARK;
        let ctx = ctx();
        let layout = solve(&tree, c_bounds(&display), &ctx);
        let rect = layout.rect(Id(1)).expect("the button");
        // The ring's band: the top edge of the button, inset by the
        // stroke, away from the corners; no label ink reaches it.
        let band = Rect::new(
            rect.x + rect.w / 4,
            rect.y,
            rect.w / 2,
            ctx.px(tokens::FOCUS_STROKE).max(1),
        );
        let count = |c: &Canvas, color: crate::color::Color| {
            let f = c.frame();
            (band.y..band.bottom())
                .flat_map(|y| (band.x..band.right()).map(move |x| (x, y)))
                .filter(|(x, y)| {
                    let i = (*y as usize * usize::from(f.width) + *x as usize) * 4;
                    f.rgba[i] == color.r && f.rgba[i + 1] == color.g && f.rgba[i + 2] == color.b
                })
                .count()
        };

        let mut without = Canvas::new(&display);
        without.clear(theme.background);
        crate::widgets::draw_tree(
            &mut without,
            &tree,
            &layout,
            &theme,
            &UiState::new(Scale::IDENTITY),
        );
        assert!(
            count(&without, theme.primary) > 0,
            "at rest the band is the accent fill"
        );
        assert_eq!(count(&without, theme.on_primary), 0, "and carries no ring");

        let mut state = UiState::new(Scale::IDENTITY);
        state.event(&layout, Event::Key(Key::Tab));
        let mut with = Canvas::new(&display);
        with.clear(theme.background);
        crate::widgets::draw_tree(&mut with, &tree, &layout, &theme, &state);
        assert!(
            count(&with, theme.on_primary) > 0,
            "focused, the band is the ring in the accent's text colour"
        );
        assert_eq!(
            count(&with, theme.primary),
            0,
            "and not the fill, which the ring would vanish into"
        );
    }

    /// §16.120: the key under the finger is drawn pressed, and stays so
    /// for a moment after the finger lifts.
    #[test]
    fn the_key_under_the_finger_is_drawn_pressed() {
        use crate::canvas::Canvas;
        use crate::color::Theme;
        use crate::layout::{Node, solve};
        use crate::state::UiState;
        use osk_shell_api::{Event, TouchPhase};

        let tree = Node::column().child(Node::widget(Widget::keyboard(
            Id(1),
            keyboard::KeyboardKind::Bip39,
        )));
        let display = display(320, 320);
        let theme = Theme::DARK;
        let ctx = ctx();
        let layout = solve(&tree, c_bounds(&display), &ctx);
        let rect = layout.rect(Id(1)).expect("the keyboard");
        let caps = keyboard::keys(
            keyboard::KeyboardKind::Bip39,
            rect,
            &ctx,
            keyboard::ALL_KEYS,
            None,
            keyboard::Modifiers::default(),
        );
        let q = caps[0].rect;
        let w = caps[1].rect;
        let backspace = caps
            .iter()
            .find(|k| k.input == keyboard::KeyInput::Backspace)
            .expect("the backspace key")
            .rect;

        let mut state = UiState::new(Scale::IDENTITY);
        let draw = |state: &UiState| {
            let mut c = Canvas::new(&display);
            c.clear(theme.background);
            crate::widgets::draw_tree(&mut c, &tree, &layout, &theme, state);
            c
        };
        let plain = |c: &Canvas, r: Rect| count(c, r, theme.surface);
        // A pixel of a key's face, which is the fill it was drawn in.
        let face = |c: &Canvas, r: Rect| {
            let f = c.frame();
            let i = (r.center().y as usize * usize::from(f.width) + r.center().x as usize) * 4;
            [f.rgba[i], f.rgba[i + 1], f.rgba[i + 2]]
        };

        let at_rest = draw(&state);
        assert!(plain(&at_rest, q) > 0 && plain(&at_rest, w) > 0);
        let raised = face(&at_rest, backspace);
        assert_ne!(raised, face(&at_rest, q), "a raised key rests lighter");

        let touch = |x: i32, y: i32, phase| Event::Touch {
            x: x as u16,
            y: y as u16,
            phase,
        };
        let (x, y) = (q.center().x, q.center().y);
        state.event(&layout, Event::Tick { now_ms: 1_000 });
        state.event(&layout, touch(x, y, TouchPhase::Down));
        let down = draw(&state);
        assert_eq!(plain(&down, q), 0, "the key under the finger is changed");
        assert!(plain(&down, w) > 0, "and the key beside it is not");
        // §16.124: a pressed key is a colour nothing at rest has, so it
        // is not read as the raised keys beside it.
        assert_ne!(
            face(&down, q),
            raised,
            "a pressed key is not the fill a resting raised key has"
        );

        state.event(&layout, touch(x, y, TouchPhase::Up));
        assert_eq!(plain(&draw(&state), q), 0, "and stays so after the lift");

        state.event(
            &layout,
            Event::Tick {
                now_ms: 1_000 + tokens::PRESS_LINGER_MS,
            },
        );
        assert!(plain(&draw(&state), q) > 0, "until the window closes");
    }

    /// §16.124: the after-image fades in two steps — the look at half
    /// strength after the first, gone after the second.
    #[test]
    fn the_pressed_look_is_half_after_the_first_step_and_gone_after_the_second() {
        use crate::canvas::Canvas;
        use crate::color::Theme;
        use crate::layout::{Node, solve};
        use crate::state::UiState;
        use osk_shell_api::{Event, TouchPhase};

        let tree = Node::column().child(
            Node::widget(Widget::button(Id(1), "Continue", ButtonStyle::Secondary)).height(48.0),
        );
        let display = display(320, 120);
        let theme = Theme::DARK;
        let layout = solve(&tree, c_bounds(&display), &ctx());
        let rect = layout.rect(Id(1)).expect("the button");
        let mut state = UiState::new(Scale::IDENTITY);
        // The fill the button was drawn in, at its centre.
        let fill = |state: &UiState| {
            let mut c = Canvas::new(&display);
            c.clear(theme.background);
            crate::widgets::draw_tree(&mut c, &tree, &layout, &theme, state);
            let f = c.frame();
            let i = (rect.center().y as usize * usize::from(f.width)
                + (rect.x + rect.w / 4) as usize)
                * 4;
            [f.rgba[i], f.rgba[i + 1], f.rgba[i + 2]]
        };
        let touch = |phase| Event::Touch {
            x: (rect.x + rect.w / 4) as u16,
            y: rect.center().y as u16,
            phase,
        };

        let at_rest = fill(&state);
        state.event(&layout, Event::Tick { now_ms: 1_000 });
        state.event(&layout, touch(TouchPhase::Down));
        let full = fill(&state);
        assert_ne!(full, at_rest, "a finger on the button changes its fill");
        state.event(&layout, touch(TouchPhase::Up));
        assert_eq!(fill(&state), full, "which the lift leaves behind");

        state.event(
            &layout,
            Event::Tick {
                now_ms: 1_000 + tokens::PRESS_LINGER_FULL_MS,
            },
        );
        let half = fill(&state);
        assert_ne!(half, full, "the first step ends and the look weakens");
        assert_ne!(half, at_rest, "but is still on the button");

        state.event(
            &layout,
            Event::Tick {
                now_ms: 1_000 + tokens::PRESS_LINGER_MS,
            },
        );
        assert_eq!(fill(&state), at_rest, "the second step ends and it is gone");
    }

    /// §16.120, §16.124: the candidate cell under the finger is drawn
    /// pressed, and so is a selected one, each in a fill its resting
    /// self does not have.
    #[test]
    fn the_candidate_cell_under_the_finger_is_drawn_pressed() {
        use crate::canvas::Canvas;
        use crate::color::Theme;
        use crate::layout::{Node, solve};
        use crate::state::UiState;
        use osk_shell_api::{Event, TouchPhase};

        let words: Vec<String> = ["abandon", "ability", "able"]
            .iter()
            .map(|w| String::from(*w))
            .collect();
        let strip = |selected| Widget::CandidateStrip {
            id: Id(1),
            words: words.clone(),
            more: false,
            selected,
            outlined: None,
        };
        let display = display(320, 120);
        let theme = Theme::DARK;
        let ctx = ctx();
        let draw = |tree: &Node, layout: &crate::layout::Layout, state: &UiState| {
            let mut c = Canvas::new(&display);
            c.clear(theme.background);
            crate::widgets::draw_tree(&mut c, tree, layout, &theme, state);
            c
        };

        let tree = Node::column().child(Node::widget(strip(None)).height(48.0));
        let layout = solve(&tree, c_bounds(&display), &ctx);
        let rect = layout.rect(Id(1)).expect("the strip");
        // The strip is a fixed grid of cells, filled from the left.
        let cells = tokens::candidates_per_row(ctx.class, false) as i32;
        let cell_w = rect.w / cells;
        let (x, y) = (rect.x + cell_w + cell_w / 2, rect.y + rect.h / 2);
        let second = Rect::new(rect.x + cell_w, rect.y, cell_w, rect.h);
        let first = Rect::new(rect.x, rect.y, cell_w, rect.h);

        let touch = |phase| Event::Touch {
            x: x as u16,
            y: y as u16,
            phase,
        };
        let mut state = UiState::new(Scale::IDENTITY);
        let at_rest = draw(&tree, &layout, &state);
        assert!(
            count(&at_rest, second, theme.surface) > 0 && count(&at_rest, first, theme.surface) > 0,
            "at rest every cell is the resting fill"
        );
        state.event(&layout, Event::Tick { now_ms: 1_000 });
        state.event(&layout, touch(TouchPhase::Down));
        let down = draw(&tree, &layout, &state);
        assert_eq!(
            count(&down, second, theme.surface),
            0,
            "the cell under the finger has left its resting fill"
        );
        assert!(
            count(&down, first, theme.surface) > 0,
            "and the cell beside it has not"
        );

        // The first tap selected the cell. A selected cell shows the
        // press on its own accent, so the finger is answered there too,
        // and settles back to the accent when the window closes.
        state.event(&layout, touch(TouchPhase::Up));
        let chosen = Node::column().child(Node::widget(strip(Some(1))).height(48.0));
        let layout = solve(&chosen, c_bounds(&display), &ctx);
        let held = draw(&chosen, &layout, &state);
        assert_eq!(
            count(&held, second, theme.primary),
            0,
            "a selected cell under a finger is not left as it is"
        );
        assert_eq!(
            count(&held, second, theme.surface),
            0,
            "and is not the neutral cell's resting fill either"
        );
        state.event(
            &layout,
            Event::Tick {
                now_ms: 1_000 + tokens::PRESS_LINGER_MS,
            },
        );
        let settled = draw(&chosen, &layout, &state);
        assert!(
            count(&settled, second, theme.primary) > 0,
            "the window closes and the cell is the accent again"
        );
    }

    /// §4.3: the strip is a fixed grid, so a cell is the same width
    /// whatever the count and the lone candidate is the first of them.
    #[test]
    fn a_candidate_strips_cells_keep_their_width_whatever_the_count() {
        let strip = |n: usize| Widget::CandidateStrip {
            id: Id(1),
            words: ["abandon", "ability", "able", "about", "above", "absent"]
                .iter()
                .take(n)
                .map(|w| String::from(*w))
                .collect(),
            more: false,
            selected: None,
            outlined: None,
        };
        let ctx = ctx();
        let per_row = tokens::candidates_per_row(ctx.class, false) as u8;
        for n in 1..=6 {
            assert_eq!(
                candidate_cells(&strip(n), &ctx, 1000),
                per_row,
                "{n} words divide the width the same way"
            );
            assert_eq!(
                candidate_count(&strip(n), &ctx, 1000),
                (n as u8).min(per_row),
                "and fill the cells from the left"
            );
        }
        // A narrow strip keeps its cells: a word too long for one drops
        // a size rather than costing the strip a cell.
        assert_eq!(candidate_cells(&strip(3), &ctx, 200), per_row);
        assert_eq!(candidate_count(&strip(3), &ctx, 200), 3);
        // Nothing to offer, nothing to tap.
        assert_eq!(candidate_count(&strip(0), &ctx, 1000), 0);
        // §4.3: on `wide` the strip is natural-width chips in a row, one
        // cell per word, aimed at with a pointer.
        let wide = LayoutCtx::new(Scale::IDENTITY, SizeClass::Wide);
        assert_eq!(candidate_cells(&strip(3), &wide, 1000), 3);
        assert_eq!(candidate_cells(&strip(1), &wide, 1000), 1);
    }

    /// How wide and how tall the marks in `color` inside `rect` are.
    fn marks(c: &crate::canvas::Canvas, rect: Rect, color: crate::color::Color) -> (i32, i32) {
        let f = c.frame();
        let (mut left, mut right) = (i32::MAX, i32::MIN);
        let (mut top, mut bottom) = (i32::MAX, i32::MIN);
        for y in rect.y..rect.bottom() {
            for x in rect.x..rect.right() {
                let i = (y as usize * usize::from(f.width) + x as usize) * 4;
                if f.rgba[i] == color.r && f.rgba[i + 1] == color.g && f.rgba[i + 2] == color.b {
                    left = left.min(x);
                    right = right.max(x);
                    top = top.min(y);
                    bottom = bottom.max(y);
                }
            }
        }
        if right < left {
            return (0, 0);
        }
        (right - left + 1, bottom - top + 1)
    }

    /// The widest word of every Latin wordlist in `font`, in the form
    /// the strip shows it: what a candidate cell has to hold.
    fn widest_latin_words(font: &crate::fonts::SizedFace) -> Vec<(&'static str, String)> {
        use osk_bip::bip39::Language;
        [
            ("English", Language::English),
            ("Spanish", Language::Spanish),
            ("French", Language::French),
            ("Italian", Language::Italian),
            ("Czech", Language::Czech),
            ("Portuguese", Language::Portuguese),
        ]
        .into_iter()
        .map(|(name, lang)| {
            let word = lang
                .words_display()
                .iter()
                .max_by_key(|w| crate::text::width(font, w))
                .expect("a list has words");
            (name, String::from(*word))
        })
        .collect()
    }

    /// A candidate cell is sized for a word of eight letters, and a
    /// longer one takes the size at which it fits its own cell: the
    /// longest word of every Latin list — Italian's nine letters among
    /// them — is drawn whole in its cell on the 268 dp panel and on a
    /// phone, cut at neither edge.
    #[test]
    fn a_candidate_cell_holds_every_latin_lists_longest_word_whole() {
        use crate::canvas::Canvas;
        use crate::color::Theme;
        use crate::layout::{Node, solve};
        use crate::state::UiState;

        let theme = Theme::DARK;
        let state = UiState::default();
        // The 268 dp panel at both its densities, and the phone.
        for (class, w, h, dpi) in [
            (SizeClass::Small, 240u16, 320u16, 143u16),
            (SizeClass::Small, 480, 640, 286),
            (SizeClass::Mobile, 1080, 2340, 420),
        ] {
            let mut display = display(w, h);
            display.dpi = dpi;
            let ctx = LayoutCtx::new(Scale::new(dpi), class);
            for (name, word) in widest_latin_words(&candidate_font(class).sized(ctx.scale)) {
                // The strip at the width a screen gives it, with its
                // cells as a screen fills them.
                let widget = Widget::CandidateStrip {
                    id: Id(1),
                    words: alloc::vec![word.clone()],
                    more: false,
                    selected: None,
                    outlined: None,
                };
                let strip = Node::column()
                    .pad(tokens::PAD)
                    .child(Node::widget(widget).height(tokens::candidate_cell(class, false)));
                let layout = solve(&strip, c_bounds(&display), &ctx);
                let mut c = Canvas::new(&display);
                c.clear(theme.background);
                crate::widgets::draw_tree(&mut c, &strip, &layout, &theme, &state);
                assert!(
                    c.cut_texts().is_empty(),
                    "{name}'s {word} is cut in its cell on {class:?} at {dpi} dpi"
                );
            }
        }
    }

    /// §16.126: a fingerprint is read character by character wherever it
    /// appears, so the Keys row's label is drawn in the mono face and
    /// not in the semibold one every other row's label takes.
    #[test]
    fn a_key_rows_fingerprint_is_drawn_in_the_mono_face() {
        use crate::canvas::Canvas;
        use crate::color::Theme;
        use crate::layout::{Node, solve};
        use crate::state::UiState;

        let theme = Theme::DARK;
        let state = UiState::default();
        let display = display(411, 800);
        let ctx = ctx();
        let fingerprint = "73c5da0a";
        // How wide the fingerprint's own marks are: the row's label is
        // the one thing in it drawn in the text colour, the chevron
        // being muted and the subtitle line left empty.
        let width = |tree: &Node| {
            let layout = solve(tree, c_bounds(&display), &ctx);
            let mut c = Canvas::new(&display);
            c.clear(theme.background);
            crate::widgets::draw_tree(&mut c, tree, &layout, &theme, &state);
            let r = layout.rect(Id(1)).expect("the widget under test");
            marks(&c, r, theme.text).0
        };
        let row = Node::column()
            .pad(tokens::PAD)
            .child(crate::components::key_row(
                ctx.class,
                Id(1),
                fingerprint,
                "",
            ));
        let label = |font: Font| {
            Node::column()
                .pad(tokens::PAD)
                .child(Node::widget(Widget::text(fingerprint, font, Tone::Text)).id(Id(1)))
        };
        let drawn = width(&row);
        assert_eq!(
            drawn,
            width(&label(Font::mono(tokens::BODY))),
            "the fingerprint is as wide as the mono face sets it"
        );
        // The face every other row's label takes, drawn straight onto the
        // canvas: a text widget sets a fingerprint in mono wherever it
        // appears, so it cannot be the reference.
        let text_face = {
            let mut c = Canvas::new(&display);
            c.clear(theme.background);
            c.text(0, 0, fingerprint, Font::semibold(tokens::BODY), theme.text);
            marks(&c, c.bounds(), theme.text).0
        };
        assert_ne!(
            drawn, text_face,
            "and not as wide as the face every other row's label takes"
        );
    }

    /// A menu row label too long for the panel's row takes a second line
    /// and is drawn whole; one word longer than the row is cut at its
    /// edge, and the canvas says so (`docs/PLANNING.md` §16.133).
    #[test]
    fn a_long_row_label_wraps_rather_than_being_cut() {
        use crate::canvas::Canvas;
        use crate::color::Theme;
        use crate::layout::{Node, solve};
        use crate::state::UiState;

        let theme = Theme::DARK;
        let state = UiState::default();
        let mut display = display(240, 320);
        display.dpi = 143;
        let class = SizeClass::of(&display);
        let ctx = LayoutCtx::new(Scale::for_class(display.dpi, class), class);
        let draw = |label: &str| {
            let tree = Node::column().child(
                Node::widget(Widget::menu_row(Id(1), Icon::Fingerprint, label, None)).id(Id(1)),
            );
            let layout = solve(&tree, c_bounds(&display), &ctx);
            let mut c = Canvas::new(&display);
            c.clear(theme.background);
            crate::widgets::draw_tree(&mut c, &tree, &layout, &theme, &state);
            let r = layout.rect(Id(1)).expect("the row");
            (c.cut_texts().to_vec(), marks(&c, r, theme.text).1)
        };
        let one_line = Font::regular(tokens::LABEL).sized(ctx.scale).line_height();
        let (cut, tall) = draw("Create a key from the words of another key");
        assert!(cut.is_empty(), "the long label is cut: {cut:?}");
        assert!(tall > one_line, "the long label is on one line");
        let (cut, tall) = draw("Settings");
        assert!(
            cut.is_empty() && tall <= one_line,
            "a short label is one line"
        );
        let word = "Einstellungsmöglichkeitenverzeichnisse";
        let (cut, _) = draw(word);
        assert_eq!(cut, alloc::vec![String::from(word)], "the long word is cut");
    }

    /// Pixels of `color` inside `rect`.
    fn count(c: &crate::canvas::Canvas, rect: Rect, color: crate::color::Color) -> usize {
        let f = c.frame();
        (rect.y..rect.bottom())
            .flat_map(|y| (rect.x..rect.right()).map(move |x| (x, y)))
            .filter(|(x, y)| {
                let i = (*y as usize * usize::from(f.width) + *x as usize) * 4;
                f.rgba[i] == color.r && f.rgba[i + 1] == color.g && f.rgba[i + 2] == color.b
            })
            .count()
    }

    fn display(width: u16, height: u16) -> osk_shell_api::DisplayInfo {
        osk_shell_api::DisplayInfo {
            width,
            height,
            dpi: 160,
            inset_bottom: 0,
            inset_top: 0,
            buttons: 0,
            camera_fixed: true,
            secure: osk_shell_api::SecureHardware::None,
            boot: osk_shell_api::BootState::Unknown,
            memory_mib: None,
        }
    }

    fn c_bounds(display: &osk_shell_api::DisplayInfo) -> Rect {
        crate::canvas::Canvas::new(display).bounds()
    }

    #[test]
    fn disabled_widgets_have_no_hit_target() {
        assert!(
            Widget::disabled_button(Id(1), "x", ButtonStyle::Primary)
                .hit_target()
                .is_none()
        );
        assert!(
            Widget::tile(Id(1), Icon::Keys, "x", false)
                .hit_target()
                .is_none()
        );
        assert_eq!(Widget::chip(None, "x", false).hit_target(), None);
        assert_eq!(
            Widget::chip(Some(Id(3)), "x", false).hit_target(),
            Some(HitTarget::Tap(Id(3)))
        );
        assert_eq!(
            Widget::CandidateStrip {
                id: Id(4),
                words: Vec::new(),
                more: false,
                selected: None,
                outlined: None,
            }
            .hit_target(),
            None
        );
    }

    /// A character needs no more width than a key, so a one-character
    /// strip is ten cells wide whatever is in it and however narrow the
    /// panel — and the last cell pages for a tone group of more than the
    /// two rows of ten hold, which the readings as they stand never
    /// reach (`docs/PLANNING.md` §16.45).
    #[test]
    fn a_one_character_strip_keeps_ten_cells_and_pages_past_them() {
        let strip = |chars: &str, more: bool| Widget::CandidateStrip {
            id: Id(1),
            words: chars.chars().map(String::from).collect(),
            more,
            selected: None,
            outlined: None,
        };
        let twenty_one = strip("一以已亿义艺忆议亦异译易益意毅溢邑翼裔逸乙", true);
        assert_eq!(candidate_count(&twenty_one, &ctx(), 1000), 10);
        assert_eq!(candidate_cells(&twenty_one, &ctx(), 1000), 10);
        assert_eq!(candidate_count(&twenty_one, &ctx(), 240), 10);
        let three = strip("一以已", false);
        assert_eq!(candidate_count(&three, &ctx(), 1000), 3);
        assert_eq!(candidate_cells(&three, &ctx(), 1000), 10);
    }

    #[test]
    fn a_qr_fits_whole_modules_and_a_secret_one_is_blank_until_held() {
        use osk_codec::qr::{Ecc, Payload, encode};
        let m = Rc::new(encode(Payload::Bytes(b"hello"), Ecc::Low).unwrap());
        assert_eq!(m.size(), 21);
        let w = Widget::qr(m.clone());
        // 21 + 8 quiet = 29 modules: 300 px gives 10 px per module, centred.
        assert_eq!(qr_geometry(Rect::new(0, 0, 300, 300), 21), (5, 5, 10));
        assert_eq!(qr_geometry(Rect::new(10, 0, 20, 20), 21), (10, 0, 1));
        // A secret code is visible while its panel is held, or while
        // the app bar's eye is showing it.
        let secret = Widget::secret_qr(m.clone(), Id(7), false);
        let state = UiState::default();
        assert!(w.qr_visible(&state));
        assert!(!secret.qr_visible(&state));
        assert!(Widget::secret_qr(m, Id(7), true).qr_visible(&state));
        assert!(secret.hit_target().is_none() && secret.id().is_none());
    }

    #[test]
    fn fingerprints_are_never_chunked_but_addresses_are() {
        assert!(!is_chunked("73c5da0a"));
        assert!(is_chunked(
            "bc1pxwww0ct9ue7e8tdnlmug5m2tamfn7q06sahstg39ys4c9f3340qqxrdu9k"
        ));
        // An unchunked string is measured and drawn as one run, so its
        // width is its own, not a multiple of four characters.
        let w = Widget::chunked(Id(1), "73c5da0a");
        let s = w.measure(&ctx(), Size::new(300, 1000));
        assert_eq!(s.h, Font::mono(16.0).sized(ctx().scale).line_height());
    }
}
