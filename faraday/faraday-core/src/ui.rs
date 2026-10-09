//! Drawing in design units.
//!
//! Every screen is laid out on a canvas of at least 1280 × 800 design
//! units (the prototype's frame), scaled to fill the display, with the
//! prototype's palette (`design/prototype/`). Text is `osk-ui`'s Noto
//! Sans and Noto Sans Mono: the Inter and JetBrains Mono of `PLAN.md` §9.2
//! wait for `faraday-ui`.

use osk_ui::fonts::Family;
use osk_ui::text::{self, TextAlign};
use osk_ui::widgets::Icon;
use osk_ui::{Canvas, Color, Font, Rect};

use crate::Action;

/// The overlay scrollbar's thumb, units wide.
const BAR_W: f32 = 6.0;
/// How far in from a scrolled region's right edge a press takes the
/// scrollbar rather than the content, in units.
pub const BAR_GRAB: f32 = 16.0;

/// Where the overlay scrollbar of a region seen through `view` (pixels)
/// runs: the track's top and length and the thumb's length, in pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BarGeometry {
    /// The track's top.
    pub top: i32,
    /// The track's length.
    pub track: i32,
    /// The thumb's length.
    pub thumb: i32,
}

impl BarGeometry {
    /// The bar of a region whose content scrolls `max` units, at `f`
    /// pixels a unit.
    pub fn of(view: Rect, max: f32, f: f32) -> BarGeometry {
        let inset = (4.0 * f).round() as i32;
        let track = view.h - 2 * inset;
        let vh = view.h as f32;
        let thumb = ((track as f32 * vh / (vh + max * f)) as i32)
            .max((30.0 * f) as i32)
            .min(track);
        BarGeometry {
            top: view.y + inset,
            track,
            thumb,
        }
    }
}

/// The prototype's palette.
pub mod pal {
    use osk_ui::Color;

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

    /// Each colour above and what it is in the light theme
    /// (`docs/MOTION.md` §3.6).
    pub const LIGHT: [(Color, Color); 14] = [
        (BG, Color::rgb(0xf4, 0xf6, 0xf8)),
        (SIDEBAR, Color::rgb(0xe9, 0xed, 0xf1)),
        (SURFACE, Color::rgb(0xff, 0xff, 0xff)),
        (LINE, Color::rgb(0xdc, 0xe2, 0xe8)),
        (INNER, Color::rgb(0xe8, 0xed, 0xf1)),
        (BORDER, Color::rgb(0xc6, 0xd0, 0xd8)),
        (TEXT, Color::rgb(0x15, 0x1c, 0x23)),
        (MUTED, Color::rgb(0x4f, 0x5d, 0x69)),
        (DIM, Color::rgb(0x7a, 0x88, 0x94)),
        (ACCENT, Color::rgb(0x0b, 0x7a, 0x9e)),
        (ON_ACCENT, Color::rgb(0xff, 0xff, 0xff)),
        (OK, Color::rgb(0x1d, 0x85, 0x56)),
        (WARN, Color::rgb(0x9a, 0x5d, 0x00)),
        (ERR, Color::rgb(0xbf, 0x34, 0x34)),
    ];

    /// Nord (nordtheme.com): Polar Night, Snow Storm and Frost.
    pub const NORD: [(Color, Color); 14] = [
        (BG, Color::rgb(0x2e, 0x34, 0x40)),
        (SIDEBAR, Color::rgb(0x27, 0x2c, 0x36)),
        (SURFACE, Color::rgb(0x3b, 0x42, 0x52)),
        (LINE, Color::rgb(0x43, 0x4c, 0x5e)),
        (INNER, Color::rgb(0x47, 0x4f, 0x62)),
        (BORDER, Color::rgb(0x5a, 0x65, 0x7c)),
        (TEXT, Color::rgb(0xec, 0xef, 0xf4)),
        (MUTED, Color::rgb(0xb7, 0xc0, 0xcf)),
        (DIM, Color::rgb(0x86, 0x91, 0xa7)),
        (ACCENT, Color::rgb(0x88, 0xc0, 0xd0)),
        (ON_ACCENT, Color::rgb(0x2e, 0x34, 0x40)),
        (OK, Color::rgb(0xa3, 0xbe, 0x8c)),
        (WARN, Color::rgb(0xeb, 0xcb, 0x8b)),
        (ERR, Color::rgb(0xeb, 0xa1, 0xa8)),
    ];

    /// Catppuccin Mocha (catppuccin.com): base, mantle, surfaces, mauve.
    pub const CATPPUCCIN: [(Color, Color); 14] = [
        (BG, Color::rgb(0x1e, 0x1e, 0x2e)),
        (SIDEBAR, Color::rgb(0x18, 0x18, 0x25)),
        (SURFACE, Color::rgb(0x31, 0x32, 0x44)),
        (LINE, Color::rgb(0x3b, 0x3d, 0x52)),
        (INNER, Color::rgb(0x45, 0x47, 0x5a)),
        (BORDER, Color::rgb(0x58, 0x5b, 0x70)),
        (TEXT, Color::rgb(0xcd, 0xd6, 0xf4)),
        (MUTED, Color::rgb(0xa6, 0xad, 0xc8)),
        (DIM, Color::rgb(0x7f, 0x84, 0x9c)),
        (ACCENT, Color::rgb(0xcb, 0xa6, 0xf7)),
        (ON_ACCENT, Color::rgb(0x1e, 0x1e, 0x2e)),
        (OK, Color::rgb(0xa6, 0xe3, 0xa1)),
        (WARN, Color::rgb(0xf9, 0xe2, 0xaf)),
        (ERR, Color::rgb(0xf3, 0x8b, 0xa8)),
    ];

    /// Tokyo Night (enkia's theme): night background, storm cards, blue.
    pub const TOKYO_NIGHT: [(Color, Color); 14] = [
        (BG, Color::rgb(0x1a, 0x1b, 0x26)),
        (SIDEBAR, Color::rgb(0x16, 0x16, 0x1e)),
        (SURFACE, Color::rgb(0x24, 0x28, 0x3b)),
        (LINE, Color::rgb(0x2f, 0x34, 0x4d)),
        (INNER, Color::rgb(0x2f, 0x35, 0x49)),
        (BORDER, Color::rgb(0x3b, 0x42, 0x61)),
        (TEXT, Color::rgb(0xc0, 0xca, 0xf5)),
        (MUTED, Color::rgb(0xa9, 0xb1, 0xd6)),
        (DIM, Color::rgb(0x73, 0x7a, 0xa2)),
        (ACCENT, Color::rgb(0x7a, 0xa2, 0xf7)),
        (ON_ACCENT, Color::rgb(0x1a, 0x1b, 0x26)),
        (OK, Color::rgb(0x9e, 0xce, 0x6a)),
        (WARN, Color::rgb(0xe0, 0xaf, 0x68)),
        (ERR, Color::rgb(0xf7, 0x76, 0x8e)),
    ];

    /// Gruvbox dark (morhetz): bg0 to bg2, fg, yellow, and its bright accents.
    pub const GRUVBOX: [(Color, Color); 14] = [
        (BG, Color::rgb(0x28, 0x28, 0x28)),
        (SIDEBAR, Color::rgb(0x1d, 0x20, 0x21)),
        (SURFACE, Color::rgb(0x32, 0x30, 0x2f)),
        (LINE, Color::rgb(0x3c, 0x38, 0x36)),
        (INNER, Color::rgb(0x45, 0x40, 0x3d)),
        (BORDER, Color::rgb(0x50, 0x49, 0x45)),
        (TEXT, Color::rgb(0xeb, 0xdb, 0xb2)),
        (MUTED, Color::rgb(0xbd, 0xae, 0x93)),
        (DIM, Color::rgb(0x92, 0x83, 0x74)),
        (ACCENT, Color::rgb(0xfa, 0xbd, 0x2f)),
        (ON_ACCENT, Color::rgb(0x28, 0x28, 0x28)),
        (OK, Color::rgb(0xb8, 0xbb, 0x26)),
        (WARN, Color::rgb(0xfe, 0x80, 0x19)),
        (ERR, Color::rgb(0xfd, 0x6f, 0x5a)),
    ];

    /// Rosé Pine Dawn (rosepinetheme.com): base, surface, overlay, pine. Its
    /// gold, love and a green beside them are darkened to read on its paper.
    pub const ROSE_PINE: [(Color, Color); 14] = [
        (BG, Color::rgb(0xfa, 0xf4, 0xed)),
        (SIDEBAR, Color::rgb(0xf2, 0xe9, 0xe1)),
        (SURFACE, Color::rgb(0xff, 0xfa, 0xf3)),
        (LINE, Color::rgb(0xe6, 0xdf, 0xd8)),
        (INNER, Color::rgb(0xf4, 0xed, 0xe8)),
        (BORDER, Color::rgb(0xce, 0xca, 0xcd)),
        (TEXT, Color::rgb(0x57, 0x52, 0x79)),
        (MUTED, Color::rgb(0x6e, 0x6a, 0x86)),
        (DIM, Color::rgb(0x98, 0x93, 0xa5)),
        (ACCENT, Color::rgb(0x28, 0x69, 0x83)),
        (ON_ACCENT, Color::rgb(0xff, 0xfa, 0xf3)),
        (OK, Color::rgb(0x3d, 0x7a, 0x52)),
        (WARN, Color::rgb(0xa8, 0x64, 0x12)),
        (ERR, Color::rgb(0xa4, 0x50, 0x6a)),
    ];

    /// Bitcoin Orange: a warm near-black page, Bitcoin's orange
    /// (#F7931A) as the accent at full strength. Light text on a dark
    /// page needs no darkening to read against it.
    pub const BITCOIN_ORANGE: [(Color, Color); 14] = [
        (BG, Color::rgb(0x1c, 0x14, 0x10)),
        (SIDEBAR, Color::rgb(0x14, 0x0e, 0x0a)),
        (SURFACE, Color::rgb(0x27, 0x1d, 0x17)),
        (LINE, Color::rgb(0x33, 0x26, 0x19)),
        (INNER, Color::rgb(0x3a, 0x2c, 0x1e)),
        (BORDER, Color::rgb(0x4a, 0x38, 0x28)),
        (TEXT, Color::rgb(0xf2, 0xe9, 0xdf)),
        (MUTED, Color::rgb(0xc9, 0xb8, 0xa4)),
        (DIM, Color::rgb(0x8f, 0x7d, 0x6c)),
        (ACCENT, Color::rgb(0xf7, 0x93, 0x1a)),
        (ON_ACCENT, Color::rgb(0x1c, 0x14, 0x10)),
        (OK, Color::rgb(0x8e, 0xcb, 0x86)),
        (WARN, Color::rgb(0xe8, 0xb9, 0x4c)),
        (ERR, Color::rgb(0xe5, 0x48, 0x4d)),
    ];

    /// Bitcoin Orange Light: a warm paper page, Bitcoin's orange
    /// darkened (its hue kept) to 4.5:1 against the page for text and
    /// the focus ring (#F7931A is 2.1:1 there; #9E5906 is 4.9:1).
    pub const BITCOIN_ORANGE_LIGHT: [(Color, Color); 14] = [
        (BG, Color::rgb(0xfd, 0xf1, 0xe7)),
        (SIDEBAR, Color::rgb(0xf5, 0xe6, 0xd7)),
        (SURFACE, Color::rgb(0xff, 0xfc, 0xf8)),
        (LINE, Color::rgb(0xe9, 0xd9, 0xc5)),
        (INNER, Color::rgb(0xf2, 0xe4, 0xd3)),
        (BORDER, Color::rgb(0xd8, 0xc0, 0xa3)),
        (TEXT, Color::rgb(0x2a, 0x1c, 0x0f)),
        (MUTED, Color::rgb(0x6b, 0x54, 0x40)),
        (DIM, Color::rgb(0x9c, 0x88, 0x72)),
        (ACCENT, Color::rgb(0x9e, 0x59, 0x06)),
        (ON_ACCENT, Color::rgb(0xff, 0xff, 0xff)),
        (OK, Color::rgb(0x1d, 0x85, 0x56)),
        (WARN, Color::rgb(0x7a, 0x64, 0x00)),
        (ERR, Color::rgb(0xbf, 0x34, 0x34)),
    ];
}

use pal::*;

/// Faraday's mark as a pixel grid, the one the boot logo is drawn from.
const MARK: &str = include_str!("../../image/overlay/common/faraday-mark.txt");

/// Which palette the screens are drawn in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Theme {
    /// Light text on a dark page: the prototype's palette.
    Dark,
    /// Dark text on a light page.
    Light,
    /// Arctic Ice Studio's Nord: grey-blue, with its frost accent.
    Nord,
    /// Catppuccin Mocha: deep violet-grey, with its mauve accent.
    Catppuccin,
    /// Tokyo Night: ink blue, with its blue accent. The theme a first
    /// start is in, on every shell.
    #[default]
    TokyoNight,
    /// Gruvbox dark: warm brown-grey, with its yellow accent.
    Gruvbox,
    /// Rosé Pine Dawn: dark text on warm paper, with its pine accent.
    RosePine,
    /// Light text on a dark page, with Bitcoin's orange (#F7931A) as
    /// the accent, taken at full strength.
    BitcoinOrange,
    /// Dark text on a light page, with Bitcoin's orange as the accent,
    /// darkened (its hue kept) to read at 4.5:1 against the page.
    BitcoinOrangeLight,
}

impl Theme {
    /// Every theme, in the order Settings offers them.
    pub const ALL: [Theme; 9] = [
        Theme::Dark,
        Theme::Light,
        Theme::Nord,
        Theme::Catppuccin,
        Theme::TokyoNight,
        Theme::Gruvbox,
        Theme::RosePine,
        Theme::BitcoinOrange,
        Theme::BitcoinOrangeLight,
    ];

    /// What Settings calls it.
    pub fn name(self) -> &'static str {
        match self {
            Theme::Dark => "Dark",
            Theme::Light => "Light",
            Theme::Nord => "Nord",
            Theme::Catppuccin => "Catppuccin",
            Theme::TokyoNight => "Tokyo Night",
            Theme::Gruvbox => "Gruvbox",
            Theme::RosePine => "Rosé Pine",
            Theme::BitcoinOrange => "Bitcoin Orange",
            Theme::BitcoinOrangeLight => "Bitcoin Orange Light",
        }
    }

    /// The word the kept settings write it as (`memory.rs`).
    pub fn id(self) -> &'static str {
        match self {
            Theme::Dark => "dark",
            Theme::Light => "light",
            Theme::Nord => "nord",
            Theme::Catppuccin => "catppuccin",
            Theme::TokyoNight => "tokyo-night",
            Theme::Gruvbox => "gruvbox",
            Theme::RosePine => "rose-pine",
            Theme::BitcoinOrange => "bitcoin-orange",
            Theme::BitcoinOrangeLight => "bitcoin-orange-light",
        }
    }

    /// The theme a kept `id` names.
    pub fn from_id(id: &str) -> Option<Theme> {
        Theme::ALL.into_iter().find(|t| t.id() == id)
    }

    /// Dark text on a light page: shadows are lighter.
    pub fn is_light(self) -> bool {
        matches!(
            self,
            Theme::Light | Theme::RosePine | Theme::BitcoinOrangeLight
        )
    }

    fn table(self) -> Option<&'static [(Color, Color); 14]> {
        match self {
            Theme::Dark => None,
            Theme::Light => Some(&pal::LIGHT),
            Theme::Nord => Some(&pal::NORD),
            Theme::Catppuccin => Some(&pal::CATPPUCCIN),
            Theme::TokyoNight => Some(&pal::TOKYO_NIGHT),
            Theme::Gruvbox => Some(&pal::GRUVBOX),
            Theme::RosePine => Some(&pal::ROSE_PINE),
            Theme::BitcoinOrange => Some(&pal::BITCOIN_ORANGE),
            Theme::BitcoinOrangeLight => Some(&pal::BITCOIN_ORANGE_LIGHT),
        }
    }

    /// `c`, one of [`pal`]'s colours at any opacity, as this theme draws
    /// it. Any other colour, a QR code's black and white or a camera's
    /// picture, is drawn as it is.
    pub fn color(self, c: Color) -> Color {
        let Some(table) = self.table() else {
            return c;
        };
        table
            .iter()
            .find(|(dark, _)| (dark.r, dark.g, dark.b) == (c.r, c.g, c.b))
            .map_or(c, |(_, to)| to.with_alpha(c.a))
    }
}

/// A face: text, emphasis, or data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum W {
    /// Noto Sans.
    R,
    /// Noto Sans SemiBold.
    S,
    /// Noto Sans Mono.
    M,
}

/// How a button looks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Style {
    /// The one action a screen leads with.
    Primary,
    /// Any other action.
    Secondary,
    /// A quiet action beside others.
    Ghost,
    /// Shown but not available now.
    Disabled,
}

/// One frame's drawing context.
pub struct Ui<'a> {
    /// The canvas.
    pub c: &'a mut Canvas,
    /// Pixels per design unit.
    pub f: f32,
    /// The canvas's type ramp over its length scale.
    tr: f32,
    /// Where each action can be pressed, in pixels.
    pub hits: &'a mut Vec<(Rect, Action)>,
    /// The action under a finger that is down.
    pub pressed: Option<Action>,
    /// When set, hits are only taken inside this layer (a sheet).
    pub layer: u8,
    /// Where design (0, 0) is, in design units: screens are drawn in a
    /// box that may sit inside a larger window.
    pub ox: f32,
    /// See [`Ui::ox`].
    pub oy: f32,
    /// All of the focused field is selected, and is drawn so.
    pub select_all: bool,
    /// The regions this frame scrolls, as the screen or sheet that drew
    /// them last reported them: one a slot, the last report of a slot
    /// replacing an earlier one (a sheet's region over the screen's).
    pub scrolled: Vec<Scrolled>,
    /// The region the wheel and a finger move now: its stretch and its
    /// overlay scrollbar are drawn, no other's.
    pub active: Slot,
    /// How far the scrolling region is shown stretched past an end, in
    /// pixels: negative when its content is pulled down past its top.
    pub stretch: i32,
    /// The stretch belongs to a sheet's region rather than the screen's.
    pub stretch_in_sheet: bool,
    /// What is being drawn now is a sheet.
    pub in_sheet: bool,
    /// On a small panel, the page's forward action asks to be pinned at
    /// the panel's foot rather than drawn where it is (the frame draws
    /// it, and the page scrolls above it).
    pub pinning: bool,
    /// The forward action a page pinned this frame.
    pub pin: Option<(String, Style, Action)>,
    /// A small panel's sheet keeps its buttons at its foot: they are
    /// handed to it rather than drawn in its body.
    pub sheet_pinning: bool,
    /// The buttons a sheet's body handed over.
    pub sheet_pin: Option<Vec<(String, Style, Action)>>,
    /// The overlay scrollbar for the scrolling region, when it shows:
    /// its opacity (0–255) and the region's offset in units.
    pub bar: Option<(u8, f32)>,
    /// The palette.
    pub theme: Theme,
    /// The action under the pointer, with no button down.
    pub hovered: Option<Action>,
    /// The screen's or the scrolling sheet's own region's offset
    /// ([`Slot::Page`]), units.
    pub offset: f32,
    /// The frosted copy of the page under the open sheet, kept from the
    /// frame the sheet opened on.
    pub frost: Option<Vec<u8>>,
    /// Where the Guided switch's pill is: 0 on Steps only, 1 on Guided.
    pub guided_shown: f32,
    /// On a small panel, which walk-through is shown on this screen: a
    /// step's place, or [`crate::ABOUT_PAGE`].
    pub about_open: Option<u8>,
    /// A step card opening, and the one closing, while they move.
    pub disclosure: Option<Disclosure>,
    /// What the step column drew: which card is open and how tall its
    /// body is, units.
    pub column: Option<(Option<usize>, f32)>,
    /// Where the step column wants to glide to, to bring the open card
    /// into view, units.
    pub follow_to: Option<f32>,
    /// The display is a small panel: one column, a page per step.
    pub compact: bool,
    /// Hits are clipped to this rather than to the drawing's clip while
    /// a card's body is revealed: what is not yet uncovered can still be
    /// pressed where it will be.
    hit_clip: Option<Rect>,
    /// The hit being taken draws its own hover look.
    quiet_hover: bool,
    /// What a press beside the open sheet does: the sheet's own way out.
    pub outside: Option<Action>,
    /// The text caret shows this frame: it blinks.
    pub caret_on: bool,
    /// A caret was drawn this frame, shown or in the off half of its
    /// blink, so the next half wants a frame.
    pub caret_drawn: bool,
}

/// A step card opening and the one closing, part of the way.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Disclosure {
    /// The card opening.
    pub open: Option<usize>,
    /// The card closing, and how tall its body was, units.
    pub closing: Option<(usize, f32)>,
    /// How far through, 0 to 1, eased.
    pub shown: f32,
}

/// Which offset a scrolled region moves. A screen can scroll more than
/// one region at once, each by its own offset; the wheel, a trackpad and
/// a finger move the one under the pointer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Slot {
    /// The screen's region, or the open sheet's when it scrolls: the
    /// offset `Faraday::scroll_slot` names for it.
    #[default]
    Page,
    /// Stick visit's Outbox list (`VisitState::out_offset`).
    VisitOut,
}

/// A scrolled region as drawn: where it is and how far it goes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Scrolled {
    /// The offset it moves.
    pub slot: Slot,
    /// Where it is seen on the panel, in pixels.
    pub view: Rect,
    /// How far its content scrolls, in design units.
    pub max: f32,
    /// It draws its own scrollbar.
    pub own_bar: bool,
}

impl<'a> Ui<'a> {
    /// A context over `c`.
    pub fn new(
        c: &'a mut Canvas,
        f: f32,
        hits: &'a mut Vec<(Rect, Action)>,
        pressed: Option<Action>,
    ) -> Self {
        let s = c.scale();
        let tr = s.type_factor() / s.factor();
        Ui {
            c,
            f,
            tr,
            hits,
            pressed,
            layer: 0,
            ox: 0.0,
            oy: 0.0,
            select_all: false,
            scrolled: Vec::new(),
            active: Slot::Page,
            stretch: 0,
            stretch_in_sheet: false,
            in_sheet: false,
            bar: None,
            theme: Theme::default(),
            hovered: None,
            offset: 0.0,
            frost: None,
            guided_shown: 1.0,
            about_open: None,
            pinning: false,
            pin: None,
            sheet_pinning: false,
            sheet_pin: None,
            disclosure: None,
            column: None,
            follow_to: None,
            compact: false,
            hit_clip: None,
            quiet_hover: false,
            outside: None,
            caret_on: true,
            caret_drawn: false,
        }
    }

    /// `c` in this frame's theme.
    pub fn col(&self, c: Color) -> Color {
        self.theme.color(c)
    }

    /// Fills the whole frame with `c`.
    pub fn clear(&mut self, c: Color) {
        let c = self.col(c);
        self.c.clear(c);
    }

    /// Says that this frame scrolls the region seen through `view`
    /// (pixels), whose content scrolls as far as `max` units. Called once
    /// the region's content is drawn: a stretch past an end
    /// (`docs/MOTION.md` §3.3) moves what was drawn in it, and what is
    /// pressable there, and the page shows in the gap.
    pub fn report_scroll(&mut self, view: Rect, max: f32) {
        let offset = self.offset;
        self.report(Slot::Page, view, max, offset, false);
    }

    /// [`Ui::report_scroll`], for a region that draws its own scrollbar.
    pub fn report_scroll_own_bar(&mut self, view: Rect, max: f32) {
        let offset = self.offset;
        self.report(Slot::Page, view, max, offset, true);
    }

    /// [`Ui::report_scroll_own_bar`], for a region beside the page's that
    /// moves by its own offset, `offset` units now.
    pub fn report_scroll_in(&mut self, slot: Slot, view: Rect, max: f32, offset: f32) {
        self.report(slot, view, max, offset, true);
    }

    fn report(&mut self, slot: Slot, view: Rect, max: f32, offset: f32, own_bar: bool) {
        let max = max.max(0.0);
        let region = Scrolled {
            slot,
            view,
            max,
            own_bar,
        };
        match self.scrolled.iter_mut().find(|s| s.slot == slot) {
            Some(s) => *s = region,
            None => self.scrolled.push(region),
        }
        if self.in_sheet != self.stretch_in_sheet {
            return;
        }
        if slot == self.active {
            self.stretch_view(view);
        }
        self.edge_fades(view, max, offset);
        // A small panel draws no scrollbar: the edge fades say the page
        // goes on.
        if let Some((alpha, offset)) = self.bar
            && slot == self.active
            && !self.compact
            && !own_bar
            && max > 0.0
            && alpha > 0
        {
            self.scroll_bar(view, max, offset, alpha);
        }
    }

    /// The overlay scrollbar: a thumb at the region's right edge, as long
    /// as the share of the content in view. It can be held and dragged
    /// ([`BarGeometry`]).
    fn scroll_bar(&mut self, view: Rect, max: f32, offset: f32, alpha: u8) {
        let f = self.f;
        let BarGeometry {
            top: track_top,
            track,
            thumb,
        } = BarGeometry::of(view, max, f);
        let inset = track_top - view.y;
        if track <= 0 || thumb <= 0 {
            return;
        }
        let at = ((track - thumb) as f32 * (offset / max).clamp(0.0, 1.0)).round() as i32;
        // Shortened by what the stretch pulls past the end, as a native
        // bar is.
        let squeeze = self.stretch.abs().min(thumb / 2);
        let (y, h) = if self.stretch < 0 {
            (view.y + inset, thumb - squeeze)
        } else {
            (
                view.y + inset + at + squeeze.min(track - thumb - at).max(0),
                thumb - squeeze,
            )
        };
        let w = (BAR_W * f).round().max(3.0) as i32;
        let rect = Rect::new(view.right() - inset - w, y, w, h);
        let color = self.col(MUTED.with_alpha((u16::from(alpha) * 150 / 255) as u8));
        self.c.fill_rounded_rect(rect, w as f32 / 2.0, color);
    }

    /// The region's content fades into the page at an edge it continues
    /// past: under the top once scrolled, above the bottom while there is
    /// more. Each fade comes in over the first 24 units scrolled.
    fn edge_fades(&mut self, view: Rect, max: f32, offset: f32) {
        if max <= 0.0 || view.h <= 0 {
            return;
        }
        // Deeper on a small panel, where it is the sign there is more.
        let depth = if self.compact { 44.0 } else { 24.0 };
        let band = ((depth * self.f).round() as i32).clamp(1, view.h / 4 + 1);
        let page = self.col(if self.in_sheet { SURFACE } else { BG });
        let reach = band as f32 / self.f;
        let top = (offset / reach).clamp(0.0, 1.0);
        let bottom = ((max - offset) / reach).clamp(0.0, 1.0);
        for i in 0..band {
            let k = 1.0 - (i as f32 + 0.5) / band as f32;
            let a = k * k * 230.0;
            if top > 0.0 {
                let row = Rect::new(view.x, view.y + i, view.w, 1);
                self.c.fill_rect(row, page.with_alpha((a * top) as u8));
            }
            if bottom > 0.0 {
                let row = Rect::new(view.x, view.bottom() - 1 - i, view.w, 1);
                self.c.fill_rect(row, page.with_alpha((a * bottom) as u8));
            }
        }
    }

    /// What a sheet stands on: the page, frosted (blurred once, the
    /// first frame, and kept) and dimmed.
    pub fn backdrop(&mut self) {
        match self.frost.as_deref() {
            Some(page) => self.c.restore(page),
            None => {
                // Reduced eight units to a pixel and blurred a little: as
                // soft as more blur at a fraction of the work.
                let scale = (8.0 * self.f).round().max(4.0) as u32;
                let dim = self.col(BG.with_alpha(150));
                self.c.frost(scale, 2, dim);
                // Kept dimmed: each frame after is one copy.
                self.frost = Some(self.c.snapshot());
            }
        }
    }

    /// A soft shadow under something that floats at (x, y), w by h with
    /// corners of `r`: drawn before it, in the theme's depth.
    pub fn shadow(&mut self, x: f32, y: f32, w: f32, h: f32, r: f32) {
        let rect = self.rect(x, y, w, h);
        let color = if self.theme.is_light() {
            Color::rgb(0x10, 0x18, 0x20).with_alpha(46)
        } else {
            Color::rgb(0, 0, 0).with_alpha(140)
        };
        let blur = 28.0 * self.f;
        let dy = (8.0 * self.f).round() as i32;
        self.c.shadow(rect, r * self.f, blur, dy, color);
    }

    /// Moves what was drawn in `view` by the stretch, and what is
    /// pressable there with it; the page shows in the gap.
    fn stretch_view(&mut self, view: Rect) {
        let n = self.stretch.abs().min(view.h);
        if n == 0 {
            return;
        }
        self.c.shift(view, -self.stretch);
        let gap = if self.stretch < 0 {
            Rect::new(view.x, view.y, view.w, n)
        } else {
            Rect::new(view.x, view.bottom() - n, view.w, n)
        };
        let page = self.col(if self.in_sheet { SURFACE } else { BG });
        self.c.fill_rect(gap, page);
        for (r, _) in self.hits.iter_mut() {
            if r.intersect(&view) == *r {
                *r = Rect::new(r.x, r.y - self.stretch, r.w, r.h).intersect(&view);
            }
        }
    }

    /// The selection behind text `shown` drawn at (x, y) in a field of
    /// height `h`, when the field is focused and all of it is selected.
    #[allow(clippy::too_many_arguments)]
    pub fn selection(&mut self, x: f32, y: f32, h: f32, size: f32, w: W, shown: &str, on: bool) {
        if on && self.select_all && !shown.is_empty() {
            let tw = self.measure(size, w, shown);
            self.fill(
                x - 2.0,
                y + h * 0.2,
                tw + 4.0,
                h * 0.6,
                3.0,
                ACCENT.with_alpha(90),
            );
        }
    }

    /// Design units to pixels.
    pub fn px(&self, v: f32) -> i32 {
        (v * self.f).round() as i32
    }

    /// A rectangle in design units, in pixels.
    pub fn rect(&self, x: f32, y: f32, w: f32, h: f32) -> Rect {
        let (x, y) = (x + self.ox, y + self.oy);
        let x0 = self.px(x);
        let y0 = self.px(y);
        Rect::new(x0, y0, self.px(x + w) - x0, self.px(y + h) - y0)
    }

    /// A filled circle.
    pub fn dot(&mut self, cx: f32, cy: f32, r: f32, color: Color) {
        let color = self.col(color);
        self.c.fill_circle(
            (cx + self.ox) * self.f,
            (cy + self.oy) * self.f,
            r * self.f,
            color,
        );
    }

    /// A filled rectangle with rounded corners.
    pub fn fill(&mut self, x: f32, y: f32, w: f32, h: f32, r: f32, color: Color) {
        let color = self.col(color);
        let rect = self.rect(x, y, w, h);
        if r <= 0.0 {
            self.c.fill_rect(rect, color);
        } else {
            self.c.fill_rounded_rect(rect, r * self.f, color);
        }
    }

    /// An outlined rectangle with rounded corners.
    pub fn stroke(&mut self, x: f32, y: f32, w: f32, h: f32, r: f32, color: Color) {
        let color = self.col(color);
        let rect = self.rect(x, y, w, h);
        let width = self.f.max(1.0);
        self.c.stroke_rounded_rect(rect, r * self.f, width, color);
    }

    /// A card: a surface with a one-unit edge.
    pub fn card(&mut self, x: f32, y: f32, w: f32, h: f32, edge: Color) {
        self.fill(x, y, w, h, 12.0, SURFACE);
        self.stroke(x, y, w, h, 12.0, edge);
    }

    /// A theme as a tile drawn in its own colours: its page, a card with
    /// a primary button and the three states, and its name. The theme on
    /// screen is ringed in its accent; another is ringed while hovered.
    #[allow(clippy::too_many_arguments)]
    pub fn theme_tile(
        &mut self,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        theme: Theme,
        current: bool,
        action: Action,
    ) {
        let ring = if current {
            Some(ACCENT)
        } else if self.is_hovered(action) || self.is_pressed(action) {
            Some(DIM)
        } else {
            None
        };
        if let Some(ring) = ring {
            self.stroke(x - 3.0, y - 3.0, w + 6.0, h + 6.0, 13.0, ring);
            self.stroke(x - 2.0, y - 2.0, w + 4.0, h + 4.0, 12.0, ring);
        }
        let now = self.theme;
        self.theme = theme;
        self.fill(x, y, w, h, 10.0, BG);
        self.stroke(x, y, w, h, 10.0, LINE);
        self.fill(x + 10.0, y + 10.0, w - 20.0, 30.0, 6.0, SURFACE);
        self.fill(x + 17.0, y + 19.0, 24.0, 12.0, 6.0, ACCENT);
        for (i, c) in [OK, WARN, ERR].into_iter().enumerate() {
            self.dot(x + w - 38.0 + 9.0 * i as f32, y + 25.0, 3.0, c);
        }
        // A name too long for the tile wraps at its spaces, the tile being
        // drawn taller for it (`theme_tile_h`).
        let lines = self.theme_name_lines(theme, w - 20.0);
        let mut ty = y + h - 22.0 - 15.0 * (lines.len() - 1) as f32;
        for line in lines {
            let line = self.fit(12.0, W::S, &line, w - 20.0);
            self.text(x + 10.0, ty, 12.0, W::S, TEXT, &line);
            ty += 15.0;
        }
        self.theme = now;
        self.hit(x, y, w, h, action);
    }

    /// The narrowest a theme tile can be with every theme's name in
    /// full.
    pub fn theme_tile_min(&self) -> f32 {
        Theme::ALL
            .into_iter()
            .map(|t| self.measure(12.0, W::S, t.name()) + 20.0)
            .fold(96.0, f32::max)
    }

    /// How tall theme tiles `w` wide are drawn: a line taller for each
    /// line a name wraps onto.
    pub fn theme_tile_h(&self, w: f32) -> f32 {
        let lines = Theme::ALL
            .into_iter()
            .map(|t| self.theme_name_lines(t, w - 20.0).len())
            .max()
            .unwrap_or(1);
        72.0 + 15.0 * (lines - 1) as f32
    }

    /// A theme's name in lines no wider than `max`, broken at spaces.
    fn theme_name_lines(&self, theme: Theme, max: f32) -> Vec<String> {
        let mut lines: Vec<String> = Vec::new();
        for word in theme.name().split(' ') {
            match lines.last_mut() {
                Some(last) if self.measure(12.0, W::S, &format!("{last} {word}")) <= max => {
                    last.push(' ');
                    last.push_str(word);
                }
                _ => lines.push(word.to_string()),
            }
        }
        lines
    }

    /// Faraday's mark in a box `w` by `h` units at (x, y): the keyhole
    /// shield of the boot logo (`faraday/image/overlay/common/faraday-mark.txt`),
    /// drawn as smooth outlines at the grid's proportions rather than its
    /// squares, the shield in the accent with its rim lighter and the
    /// keyhole cut through.
    pub fn mark(&mut self, x: f32, y: f32, w: f32, h: f32) {
        let rows: Vec<&[u8]> = MARK
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(str::as_bytes)
            .collect();
        let used = |c: &u8| *c != b'.';
        let Some(top) = rows.iter().position(|r| r.iter().any(used)) else {
            return;
        };
        let bottom = rows.iter().rposition(|r| r.iter().any(used)).unwrap_or(top) + 1;
        let left = rows
            .iter()
            .filter_map(|r| r.iter().position(used))
            .min()
            .unwrap_or(0);
        let right = rows
            .iter()
            .filter_map(|r| r.iter().rposition(used))
            .max()
            .unwrap_or(left)
            + 1;
        // The grid's proportions, fitted to the box and centred.
        let aspect = (bottom - top) as f32 / (right - left) as f32;
        let area = self.rect(x, y, w, h);
        let sw = (area.w as f32).min(area.h as f32 / aspect);
        let sh = sw * aspect;
        let ox = area.x as f32 + (area.w as f32 - sw) / 2.0;
        let oy = area.y as f32 + (area.h as f32 - sh) / 2.0;
        let stroke = self.col(ACCENT);
        let lit = stroke.mix(Color::rgb(0xff, 0xff, 0xff), 0.5);
        // The keyhole: a round head and a slot widening below it, wound
        // against the shield so that it is a hole.
        let (cx, head_y, head_r) = (ox + sw / 2.0, oy + 0.28 * sh, 0.15 * sw);
        let (waist, foot, foot_y) = (0.045 * sw, 0.15 * sw, oy + 0.61 * sh);
        let join = (head_r * head_r - waist * waist).sqrt();
        let from = join.atan2(-waist);
        let to = join.atan2(waist) + std::f32::consts::TAU;
        let mut hole: Vec<(f32, f32)> = (0..=48)
            .map(|i| {
                let a = from + (to - from) * i as f32 / 48.0;
                (cx + head_r * a.cos(), head_y + head_r * a.sin())
            })
            .collect();
        hole.extend([(cx + foot, foot_y), (cx - foot, foot_y)]);
        hole.reverse();
        let rim = (0.07 * sw).max(1.0);
        let outer = shield(ox, oy, sw, sh);
        let inner = shield(ox + rim, oy + rim, sw - 2.0 * rim, sh - 2.4 * rim);
        self.c.fill_contours(&[&outer, &hole], lit);
        self.c.fill_contours(&[&inner, &hole], stroke);
    }

    /// The caret of the field typing goes to, `h` units tall at (x, y),
    /// in the off half of its blink not drawn.
    pub fn caret(&mut self, x: f32, y: f32, h: f32) {
        self.caret_drawn = true;
        if self.caret_on {
            self.fill(x, y, 2.0, h, 1.0, ACCENT);
        }
    }

    /// The caret as a character after the text typed, for a field drawn
    /// as one string: a bar, or nothing in the off half of its blink.
    pub fn caret_char(&mut self) -> &'static str {
        self.caret_drawn = true;
        if self.caret_on { "|" } else { "" }
    }

    /// A horizontal rule.
    pub fn rule(&mut self, x: f32, y: f32, w: f32, color: Color) {
        let color = self.col(color);
        let r = self.rect(x, y, w, 1.0);
        let rect = Rect::new(r.x, r.y, r.w, 1.max(self.px(1.0)));
        self.c.fill_rect(rect, color);
    }

    fn font(&self, size: f32, w: W) -> Font {
        let family = match w {
            W::R => Family::Regular,
            W::S => Family::SemiBold,
            W::M => Family::Mono,
        };
        let size = if w == W::M {
            size / self.tr * 0.94
        } else {
            size / self.tr
        };
        Font::new(family, size)
    }

    /// The width of `s` in design units.
    pub fn measure(&self, size: f32, w: W, s: &str) -> f32 {
        let face = self.font(size, w).sized(self.c.scale());
        text::width(&face, s) as f32 / self.f
    }

    /// The height of one line in design units.
    pub fn line(&self, size: f32, w: W) -> f32 {
        let face = self.font(size, w).sized(self.c.scale());
        face.line_height() as f32 / self.f
    }

    /// `s` with its top-left at (x, y). Returns its width.
    pub fn text(&mut self, x: f32, y: f32, size: f32, w: W, color: Color, s: &str) -> f32 {
        let color = self.col(color);
        let font = self.font(size, w);
        let px = self
            .c
            .text(self.px(x + self.ox), self.px(y + self.oy), s, font, color);
        px as f32 / self.f
    }

    /// `s` centred vertically in a band of height `h` starting at y.
    #[allow(clippy::too_many_arguments)]
    pub fn text_mid(
        &mut self,
        x: f32,
        y: f32,
        h: f32,
        size: f32,
        w: W,
        color: Color,
        s: &str,
    ) -> f32 {
        let lh = self.line(size, w);
        self.text(x, y + (h - lh) / 2.0, size, w, color, s)
    }

    /// `s` ending at `right`, centred in a band of height `h`.
    #[allow(clippy::too_many_arguments)]
    pub fn text_right(
        &mut self,
        right: f32,
        y: f32,
        h: f32,
        size: f32,
        w: W,
        color: Color,
        s: &str,
    ) {
        let width = self.measure(size, w, s);
        self.text_mid(right - width, y, h, size, w, color, s);
    }

    /// `s` cut with an ellipsis to fit `max` units.
    pub fn fit(&self, size: f32, w: W, s: &str, max: f32) -> String {
        if self.measure(size, w, s) <= max {
            return s.to_string();
        }
        let mut chars: Vec<char> = s.chars().collect();
        while !chars.is_empty() {
            chars.pop();
            let t: String = chars.iter().collect::<String>() + "…";
            if self.measure(size, w, &t) <= max {
                return t;
            }
        }
        String::new()
    }

    /// [`Ui::fit`] for text that may be a secret: every shortened try is
    /// a [`SecretText`](crate::secret_text::SecretText), wiped as it goes.
    pub fn fit_secret(&self, size: f32, w: W, s: &str, max: f32) -> crate::secret_text::SecretText {
        use crate::secret_text::SecretText;
        if self.measure(size, w, s) <= max {
            return SecretText::of(s);
        }
        let mut t = SecretText::of(s);
        while t.pop().is_some() {
            t.push('…');
            if self.measure(size, w, &t) <= max {
                return t;
            }
            t.pop();
        }
        SecretText::new()
    }

    /// `s` wrapped to `width`. Returns the height used.
    #[allow(clippy::too_many_arguments)]
    pub fn wrap(
        &mut self,
        x: f32,
        y: f32,
        width: f32,
        size: f32,
        w: W,
        color: Color,
        s: &str,
    ) -> f32 {
        let color = self.col(color);
        let font = self.font(size, w);
        let rect = Rect::new(
            self.px(x + self.ox),
            self.px(y + self.oy),
            self.px(width),
            self.px(2000.0),
        );
        let px = self.c.text_wrapped(rect, s, font, color, TextAlign::Start);
        px as f32 / self.f
    }

    /// An icon centred in a square of side `boxw` at (x, y).
    pub fn icon(&mut self, x: f32, y: f32, boxw: f32, icon: Icon, size: f32, color: Color) {
        let color = self.col(color);
        let rect = self.rect(x, y, boxw, boxw);
        self.c.icon(rect, icon, size, color);
    }

    /// Makes a region pressable: the part of it inside the clip, so what
    /// is scrolled out of sight cannot be pressed.
    ///
    /// Under the pointer it shows so: a faint wash over what was drawn
    /// there, unless it is pressed, draws its own hover look (a button),
    /// or covers so much of the screen that a wash would light the page.
    pub fn hit(&mut self, x: f32, y: f32, w: f32, h: f32, action: Action) {
        let clip = self.hit_clip.unwrap_or_else(|| self.c.clip());
        let rect = self.rect(x, y, w, h).intersect(&clip);
        if rect.w > 0 && rect.h > 0 {
            let screen = i64::from(self.c.width()) * i64::from(self.c.height());
            if self.hovered == Some(action)
                && self.pressed != Some(action)
                && !self.quiet_hover
                && i64::from(rect.w) * i64::from(rect.h) * 4 < screen
            {
                let wash = self.col(TEXT.with_alpha(14));
                self.c.fill_rounded_rect(rect, 8.0 * self.f, wash);
            }
            self.hits.push((rect, action));
        }
    }

    /// Presses everywhere outside the box at (x, y), w by h, take
    /// `action`: a sheet that a press beside it closes. No hover look.
    pub fn hit_around(&mut self, x: f32, y: f32, w: f32, h: f32, action: Action) {
        const FAR: f32 = 10_000.0;
        self.quiet_hover = true;
        self.hit(x - FAR, y - FAR, w + 2.0 * FAR, FAR, action);
        self.hit(x - FAR, y + h, w + 2.0 * FAR, FAR, action);
        self.hit(x - FAR, y, FAR, h, action);
        self.hit(x + w, y, FAR, h, action);
        self.quiet_hover = false;
    }

    /// Draws only inside `rect` until [`Ui::unreveal`], while what is
    /// drawn stays pressable as if it all showed.
    pub fn reveal(&mut self, rect: Rect) {
        self.hit_clip = Some(self.c.clip());
        self.c.push_clip(rect);
    }

    /// Ends [`Ui::reveal`].
    pub fn unreveal(&mut self) {
        self.c.pop_clip();
        self.hit_clip = None;
    }

    /// Whether `action` is under the pointer.
    pub fn is_hovered(&self, action: Action) -> bool {
        self.hovered == Some(action) && self.pressed != Some(action)
    }

    /// Whether `action` is under a finger.
    pub fn is_pressed(&self, action: Action) -> bool {
        self.pressed == Some(action)
    }

    /// A button. Returns its width.
    #[allow(clippy::too_many_arguments)]
    pub fn button(
        &mut self,
        x: f32,
        y: f32,
        w: Option<f32>,
        h: f32,
        label: &str,
        style: Style,
        action: Action,
    ) -> f32 {
        let size = if h >= 44.0 {
            15.0
        } else if h >= 38.0 {
            14.0
        } else {
            13.0
        };
        let weight = W::S;
        let width = w.unwrap_or_else(|| self.measure(size, weight, label) + 32.0);
        let pressed = self.is_pressed(action) && style != Style::Disabled;
        let hovered = self.is_hovered(action) && style != Style::Disabled;
        match style {
            Style::Primary => {
                // Mixed in the theme's own colours.
                let bg = if pressed {
                    self.col(ACCENT).mix(self.col(BG), 0.25)
                } else if hovered {
                    self.col(ACCENT).mix(self.col(TEXT), 0.12)
                } else {
                    ACCENT
                };
                self.fill(x, y, width, h, 10.0, bg);
            }
            Style::Secondary => {
                if pressed {
                    self.fill(x, y, width, h, 10.0, INNER);
                } else if hovered {
                    self.fill(x, y, width, h, 10.0, INNER.with_alpha(150));
                }
                let edge = if hovered { DIM } else { BORDER };
                self.stroke(x, y, width, h, 10.0, edge);
            }
            Style::Ghost => {
                if pressed {
                    self.fill(x, y, width, h, 10.0, INNER);
                } else if hovered {
                    self.fill(x, y, width, h, 10.0, INNER.with_alpha(150));
                }
            }
            Style::Disabled => {
                self.stroke(x, y, width, h, 10.0, INNER);
            }
        }
        let fg = match style {
            Style::Primary => ON_ACCENT,
            Style::Secondary => TEXT,
            Style::Ghost => MUTED,
            Style::Disabled => DIM,
        };
        let tw = self.measure(size, weight, label);
        self.text_mid(x + (width - tw) / 2.0, y, h, size, weight, fg, label);
        if style != Style::Disabled {
            self.quiet_hover = true;
            self.hit(x, y, width, h, action);
            self.quiet_hover = false;
        }
        width
    }

    /// A floating action: a pill in the accent colour over the page, its
    /// icon and its word, its bottom-right corner at (`right`, `bottom`).
    /// Returns its width.
    pub fn fab(&mut self, right: f32, bottom: f32, icon: Icon, label: &str, action: Action) -> f32 {
        // Smaller on a small panel, where it floats over more of the page.
        let (h, size, pad) = if self.compact {
            (40.0, 14.0, 10.0)
        } else {
            (48.0, 15.0, 14.0)
        };
        let width = self.measure(size, W::S, label) + 2.0 * pad + 34.0;
        let (x, y) = (right - width, bottom - h);
        let bg = if self.is_pressed(action) {
            self.col(ACCENT).mix(self.col(BG), 0.25)
        } else if self.is_hovered(action) {
            self.col(ACCENT).mix(self.col(TEXT), 0.12)
        } else {
            ACCENT
        };
        self.shadow(x, y, width, h, h / 2.0);
        self.fill(x, y, width, h, h / 2.0, bg);
        self.icon(
            x + pad,
            y + (h - 26.0) / 2.0,
            26.0,
            icon,
            size - 2.0,
            ON_ACCENT,
        );
        self.text_mid(x + pad + 30.0, y, h, size, W::S, ON_ACCENT, label);
        self.quiet_hover = true;
        self.hit(x, y, width, h, action);
        self.quiet_hover = false;
        width
    }

    /// A slider over the whole numbers `lo..=hi`, at `value`: a track
    /// with a stop for each number and a round thumb carrying the value,
    /// the two ends' numbers under it. Each stop's share of the track
    /// presses as [`Action::Slide`]`(id, n)`; a finger held on it and
    /// dragged along presses each stop it crosses (`Faraday::touch`), and
    /// the arrow keys step the slider last pressed. Returns the height
    /// used.
    #[allow(clippy::too_many_arguments)]
    pub fn slider(&mut self, x: f32, y: f32, w: f32, id: u8, lo: u8, hi: u8, value: u8) -> f32 {
        let (lo, hi) = (lo.min(hi), hi.max(lo));
        let value = value.clamp(lo, hi);
        // Room at each end for the thumb.
        let pad = 14.0;
        let track = (w - 2.0 * pad).max(1.0);
        let gap = if hi > lo {
            track / f32::from(hi - lo)
        } else {
            0.0
        };
        let at = |n: u8| x + pad + gap * f32::from(n - lo);
        let mid = y + 20.0;
        let held = matches!(self.pressed, Some(Action::Slide(i, _)) if i == id);
        self.fill(x + pad, mid - 2.0, track, 4.0, 2.0, INNER);
        let vx = at(value);
        self.fill(x + pad, mid - 2.0, vx - x - pad, 4.0, 2.0, ACCENT);
        for n in lo..=hi {
            if n != value {
                let c = if n < value { ACCENT } else { BORDER };
                self.dot(at(n), mid, 3.0, c);
            }
        }
        let r = if held { 15.0 } else { 13.0 };
        let bg = if held {
            self.col(ACCENT).mix(self.col(TEXT), 0.12)
        } else {
            ACCENT
        };
        self.dot(vx, mid, r, bg);
        let v = value.to_string();
        let vw = self.measure(12.0, W::S, &v);
        self.text_mid(vx - vw / 2.0, mid - 10.0, 20.0, 12.0, W::S, ON_ACCENT, &v);
        let (l, h) = (lo.to_string(), hi.to_string());
        self.text(
            x + pad - self.measure(11.0, W::R, &l) / 2.0,
            y + 38.0,
            11.0,
            W::R,
            DIM,
            &l,
        );
        self.text(
            x + pad + track - self.measure(11.0, W::R, &h) / 2.0,
            y + 38.0,
            11.0,
            W::R,
            DIM,
            &h,
        );
        // Each stop takes the presses nearest it, the ends out to the
        // slider's edges.
        self.quiet_hover = true;
        for n in lo..=hi {
            let left = if n == lo { x } else { at(n) - gap / 2.0 };
            let right = if n == hi { x + w } else { at(n) + gap / 2.0 };
            self.hit(left, y, right - left, 40.0, Action::Slide(id, n));
        }
        self.quiet_hover = false;
        56.0
    }

    /// A rounded label with a dot. Returns its width.
    pub fn chip(&mut self, x: f32, y: f32, label: &str, fg: Color, bg: Color) -> f32 {
        let h = 26.0;
        let width = self.measure(12.0, W::R, label) + 34.0;
        self.fill(x, y, width, h, 13.0, bg);
        self.dot(x + 13.0, y + h / 2.0, 3.0, fg);
        self.text_mid(x + 22.0, y, h, 12.0, W::R, fg, label);
        width
    }

    /// A word in a pill that a press opens in its list, where words are
    /// shown as they are made. Returns its width.
    pub fn word_pill(&mut self, x: f32, y: f32, word: &str, action: Action) -> f32 {
        let h = 26.0;
        let width = self.measure(14.0, W::M, word) + 24.0;
        self.fill(x, y, width, h, h / 2.0, INNER);
        self.stroke(x, y, width, h, h / 2.0, BORDER);
        self.text_mid(x + 12.0, y, h, 14.0, W::M, TEXT, word);
        self.hit(x, y, width, h, action);
        width
    }

    /// A pill with a plain label and an edge. Returns its width.
    pub fn pill(&mut self, x: f32, y: f32, h: f32, label: &str) -> f32 {
        let width = self.measure(12.0, W::R, label) + 24.0;
        self.fill(x, y, width, h, h / 2.0, SURFACE);
        self.stroke(x, y, width, h, h / 2.0, LINE);
        self.text_mid(x + 12.0, y, h, 12.0, W::R, MUTED, label);
        width
    }

    /// Threshold pips: `filled` of `n`.
    pub fn pips(&mut self, x: f32, y: f32, n: usize, filled: usize) -> f32 {
        let mut cx = x;
        for i in 0..n {
            if i < filled {
                self.fill(cx, y, 28.0, 10.0, 5.0, OK);
            } else {
                self.stroke(cx, y, 28.0, 10.0, 5.0, BORDER);
            }
            cx += 34.0;
        }
        cx - x - 6.0
    }

    /// A numbered or ticked step badge.
    pub fn badge(&mut self, x: f32, y: f32, label: &str, done: bool, open: bool) {
        let (bg, fg) = if done {
            (OK.with_alpha(40), OK)
        } else if open {
            (ACCENT, ON_ACCENT)
        } else {
            (INNER, MUTED)
        };
        self.dot(x + 13.0, y + 13.0, 13.0, bg);
        if done {
            self.icon(x, y, 26.0, Icon::Done, 12.0, fg);
        } else {
            let tw = self.measure(12.0, W::S, label);
            self.text_mid(x + 13.0 - tw / 2.0, y, 26.0, 12.0, W::S, fg, label);
        }
    }

    /// A checkbox.
    pub fn checkbox(&mut self, x: f32, y: f32, on: bool, enabled: bool) {
        if on && enabled {
            self.fill(x, y, 18.0, 18.0, 4.0, ACCENT);
            self.icon(x, y, 18.0, Icon::Done, 11.0, ON_ACCENT);
        } else {
            self.stroke(x, y, 18.0, 18.0, 4.0, if enabled { MUTED } else { INNER });
        }
    }
}

/// An address or a txid in groups of four.
pub fn grouped(s: &str) -> String {
    s.chars()
        .collect::<Vec<_>>()
        .chunks(4)
        .map(|c| c.iter().collect::<String>())
        .collect::<Vec<_>>()
        .join(" ")
}

/// The first two groups of four, an ellipsis and the last four
/// characters.
pub fn short(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= 16 {
        return grouped(s);
    }
    let head: String = chars[..8].iter().collect();
    let tail: String = chars[chars.len() - 4..].iter().collect();
    format!("{} … {}", grouped(&head), tail)
}

/// An amount in BTC with eight decimals.
pub fn btc(sat: u64) -> String {
    format!("{}.{:08}", sat / 100_000_000, sat % 100_000_000)
}

/// A whole number with thousands separators.
pub fn thousands(n: u64) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, ch) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(ch);
    }
    out
}

/// The shield's outline in a box `w` by `h` pixels at (x, y), clockwise:
/// a flat top with rounded corners, straight sides to six tenths of the
/// way down, and curved sides meeting in a point.
fn shield(x: f32, y: f32, w: f32, h: f32) -> Vec<(f32, f32)> {
    use std::f32::consts::{FRAC_PI_2, PI};
    let r = 0.12 * w;
    let mut out = Vec::new();
    for (cx, from) in [(x + r, PI), (x + w - r, PI + FRAC_PI_2)] {
        out.extend((0..=8).map(|i| {
            let a = from + FRAC_PI_2 * i as f32 / 8.0;
            (cx + r * a.cos(), y + r + r * a.sin())
        }));
    }
    let side = y + 0.60 * h;
    let tip = (x + w / 2.0, y + h);
    // A quadratic curve from the foot of a side to the tip, its control
    // point just inside the side.
    let curve = |sx: f32, cx: f32| {
        (0..=16).map(move |i| {
            let t = i as f32 / 16.0;
            let u = 1.0 - t;
            let ctrl = (cx, side + 0.25 * h);
            (
                u * u * sx + 2.0 * u * t * ctrl.0 + t * t * tip.0,
                u * u * side + 2.0 * u * t * ctrl.1 + t * t * tip.1,
            )
        })
    };
    out.extend(curve(x + w, x + w - 0.02 * w));
    let left: Vec<(f32, f32)> = curve(x, x + 0.02 * w).collect();
    out.extend(left.into_iter().rev());
    out
}
