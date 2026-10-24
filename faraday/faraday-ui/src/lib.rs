//! Faraday's drawing: what the desktop app and Faraday Starter draw
//! their screens with.
//!
//! Drawing in design units. Every screen is laid out on a canvas scaled
//! to fill the display, with Faraday's palette and the
//! themes beside it. Text is `faraday-draw`'s
//! Noto Sans and Noto Sans Mono. [`Ui`] is generic over the app's own
//! action type: what a press on a region does is the caller's to name.

mod firmware;
pub mod keyboard;
mod lists;
pub mod parts;
pub mod sheet;

pub use firmware::sketch;

use faraday_draw::fonts::Family;
use faraday_draw::text::{self, TextAlign};
use faraday_draw::widgets::Icon;
use faraday_draw::{Canvas, Color, Font, Rect};
use faraday_strings::EN;
use std::sync::OnceLock;

/// The overlay scrollbar's thumb, units wide.
const BAR_W: f32 = 6.0;

/// How wide each of `per_row` theme tiles side by side in `w` is.
fn theme_tile_w(w: f32, per_row: usize) -> f32 {
    use faraday_draw::tokens::THEME_TILE_GAP;
    (w - THEME_TILE_GAP * (per_row - 1) as f32) / per_row as f32
}
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
pub mod pal;

use pal::*;

/// Faraday's mark as a pixel grid, the one the boot logo is drawn from.
const MARK: &str = include_str!("../../image/common/faraday-mark.txt");

/// Faraday's name as outlines: SVG path data in font units.
const WORDMARK: &str = include_str!("../assets/wordmark.txt");

/// The letters of the word after the wordmark: `assets/wordmark-suffix.txt`.
const WORDMARK_SUFFIX: &str = include_str!("../assets/wordmark-suffix.txt");

/// Font units from the wordmark's right edge to where the suffix's first
/// letter starts (its pen, before the letter's own left bearing).
const SUFFIX_GAP: f32 = 364.0;

/// The word drawn after the wordmark in an app's name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NameSuffix {
    /// Faraday alone: the stick, the Pi card.
    None,
    /// Faraday Starter.
    Starter,
    /// Faraday Desktop.
    Desktop,
}

/// The wordmark's cap height in its font units.
const WORDMARK_CAP: f32 = 687.0;

/// The wordmark's outlines, each curve as straight pieces, and the left
/// and right edges of them all, in font units.
struct Wordmark {
    outlines: Vec<Vec<(f32, f32)>>,
    left: f32,
    right: f32,
}

fn wordmark_outlines() -> &'static Wordmark {
    static OUTLINES: OnceLock<Wordmark> = OnceLock::new();
    OUTLINES.get_or_init(|| {
        let outlines = path_outlines(WORDMARK);
        let xs = || outlines.iter().flatten().map(|p| p.0);
        let left = xs().fold(f32::MAX, f32::min);
        let right = xs().fold(f32::MIN, f32::max);
        Wordmark {
            outlines,
            left,
            right,
        }
    })
}

/// Closed outlines in font units.
type Outlines = Vec<Vec<(f32, f32)>>;

/// One suffix's outlines in font units from the pen's start, and the
/// right edge of the last letter.
struct SuffixWord {
    outlines: Vec<Vec<(f32, f32)>>,
    right: f32,
}

/// The suffix `which` spelled in the letters of `assets/wordmark-suffix.txt`.
fn suffix_word(which: NameSuffix) -> Option<&'static SuffixWord> {
    static WORDS: OnceLock<[SuffixWord; 2]> = OnceLock::new();
    let key = match which {
        NameSuffix::None => return None,
        NameSuffix::Starter => 0,
        NameSuffix::Desktop => 1,
    };
    let words = WORDS.get_or_init(|| {
        let mut letters: Vec<(char, f32, Outlines)> = Vec::new();
        let mut spelled: Vec<&str> = Vec::new();
        for line in WORDMARK_SUFFIX.lines().filter(|l| !l.starts_with('#')) {
            let parts: Vec<&str> = line.split('\t').collect();
            match parts.as_slice() {
                [name, word] if name.starts_with('@') => spelled.push(word),
                [letter, advance, path] => letters.push((
                    letter.chars().next().unwrap_or(' '),
                    advance.parse().unwrap_or(0.0),
                    path_outlines(path),
                )),
                _ => {}
            }
        }
        let build = |word: &str| {
            let (mut pen, mut right) = (0.0_f32, 0.0_f32);
            let mut outlines = Vec::new();
            for c in word.chars() {
                if let Some((_, advance, shapes)) = letters.iter().find(|l| l.0 == c) {
                    for o in shapes {
                        let o: Vec<(f32, f32)> = o.iter().map(|&(x, y)| (x + pen, y)).collect();
                        right = o.iter().fold(right, |r, p| r.max(p.0));
                        outlines.push(o);
                    }
                    pen += advance;
                }
            }
            SuffixWord { outlines, right }
        };
        [
            build(spelled.first().copied().unwrap_or("")),
            build(spelled.get(1).copied().unwrap_or("")),
        ]
    });
    words.get(key)
}

/// The closed outlines of SVG path data made of absolute M, L, Q and Z,
/// with `#` lines skipped; each quadratic curve becomes straight pieces.
fn path_outlines(src: &str) -> Vec<Vec<(f32, f32)>> {
    const PIECES: usize = 6;
    let mut spaced = String::new();
    for line in src.lines().filter(|l| !l.starts_with('#')) {
        for c in line.chars() {
            if c.is_ascii_alphabetic() {
                spaced.extend([' ', c, ' ']);
            } else {
                spaced.push(c);
            }
        }
        spaced.push(' ');
    }
    let words: Vec<&str> = spaced.split_whitespace().collect();
    let num = |i: usize| words.get(i).and_then(|w| w.parse().ok()).unwrap_or(0.0);
    let mut out: Vec<Vec<(f32, f32)>> = Vec::new();
    let mut at = (0.0, 0.0);
    let mut i = 0;
    while i < words.len() {
        match words[i] {
            "M" => {
                at = (num(i + 1), num(i + 2));
                out.push(vec![at]);
                i += 3;
            }
            "L" => {
                at = (num(i + 1), num(i + 2));
                if let Some(o) = out.last_mut() {
                    o.push(at);
                }
                i += 3;
            }
            "Q" => {
                let (c, to) = ((num(i + 1), num(i + 2)), (num(i + 3), num(i + 4)));
                if let Some(o) = out.last_mut() {
                    o.extend((1..=PIECES).map(|k| {
                        let t = k as f32 / PIECES as f32;
                        let u = 1.0 - t;
                        (
                            u * u * at.0 + 2.0 * u * t * c.0 + t * t * to.0,
                            u * u * at.1 + 2.0 * u * t * c.1 + t * t * to.1,
                        )
                    }));
                }
                at = to;
                i += 5;
            }
            _ => i += 1,
        }
    }
    out
}

/// Which palette the screens are drawn in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Theme {
    /// Light text on a dark page: the prototype's palette.
    Dark,
    /// Dark text on a light page.
    Light,
    /// A cool blue-white page with a deep cobalt accent. The theme a first
    /// start is in, on every shell (owner, 2026-10-11).
    #[default]
    Cobalt,
    /// Arctic Ice Studio's Nord: grey-blue, with its frost accent.
    Nord,
    /// Catppuccin Mocha: deep violet-grey, with its mauve accent.
    Catppuccin,
    /// Tokyo Night: ink blue, with its blue accent.
    TokyoNight,
    /// Gruvbox dark: warm brown-grey, with its yellow accent.
    Gruvbox,
    /// Rosé Pine Dawn: dark text on warm paper, with its pine accent.
    RosePine,
    /// Light text on a dark page, with Bitcoin's orange (#F7931A) as
    /// the accent, taken at full strength.
    BitcoinOrange,
    /// Dark text on a light page, with Bitcoin's orange (#F7931A) as
    /// the accent, taken at full strength.
    BitcoinOrangeLight,
    /// Dark espresso text on warm cream paper, with a terracotta accent:
    /// light, warm and calm.
    Lamplight,
    /// Pure white text on a pure black page, one strong yellow accent:
    /// every text pair 7:1 or more, every edge 3:1 or more.
    HighContrastDark,
    /// Pure black text on a pure white page, one strong deep blue accent:
    /// every text pair 7:1 or more, every edge 3:1 or more.
    HighContrastLight,
    /// Mars: a warm near-black page, butterscotch text and the planet's
    /// rust red-orange as the accent.
    Mars,
}

impl Theme {
    /// Every theme. Settings offers them by [`Theme::group`], each group
    /// in order of its names ([`ThemeGroup::themes`]).
    pub const ALL: [Theme; 14] = [
        Theme::Dark,
        Theme::Light,
        Theme::Cobalt,
        Theme::Nord,
        Theme::Catppuccin,
        Theme::TokyoNight,
        Theme::Gruvbox,
        Theme::RosePine,
        Theme::BitcoinOrange,
        Theme::BitcoinOrangeLight,
        Theme::Lamplight,
        Theme::HighContrastDark,
        Theme::HighContrastLight,
        Theme::Mars,
    ];

    /// What Settings calls it.
    pub fn name(self) -> &'static str {
        match self {
            Theme::Dark => EN.ui_dark,
            Theme::Light => EN.ui_light,
            Theme::Cobalt => EN.ui_cobalt,
            Theme::Nord => EN.ui_nord,
            Theme::Catppuccin => EN.ui_catppuccin,
            Theme::TokyoNight => EN.ui_tokyo_night,
            Theme::Gruvbox => EN.ui_gruvbox,
            Theme::RosePine => EN.ui_rose_pine,
            Theme::BitcoinOrange => EN.ui_bitcoin_orange,
            Theme::BitcoinOrangeLight => EN.ui_bitcoin_orange_light,
            Theme::Lamplight => EN.ui_lamplight,
            Theme::HighContrastDark => EN.ui_high_contrast_dark,
            Theme::HighContrastLight => EN.ui_high_contrast_light,
            Theme::Mars => EN.ui_mars,
        }
    }

    /// The word the kept settings write it as (`memory.rs`).
    pub fn id(self) -> &'static str {
        match self {
            Theme::Dark => "dark",
            Theme::Light => "light",
            Theme::Cobalt => "cobalt",
            Theme::Nord => "nord",
            Theme::Catppuccin => "catppuccin",
            Theme::TokyoNight => "tokyo-night",
            Theme::Gruvbox => "gruvbox",
            Theme::RosePine => "rose-pine",
            Theme::BitcoinOrange => "bitcoin-orange",
            Theme::BitcoinOrangeLight => "bitcoin-orange-light",
            Theme::Lamplight => "lamplight",
            Theme::HighContrastDark => "high-contrast-dark",
            Theme::HighContrastLight => "high-contrast-light",
            Theme::Mars => "mars",
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
            Theme::Light
                | Theme::Cobalt
                | Theme::RosePine
                | Theme::BitcoinOrangeLight
                | Theme::Lamplight
                | Theme::HighContrastLight
        )
    }

    /// Text 7:1 or more on the page and the card, edges 3:1 or more.
    pub fn is_high_contrast(self) -> bool {
        matches!(self, Theme::HighContrastDark | Theme::HighContrastLight)
    }

    /// The group Settings offers it under.
    pub fn group(self) -> ThemeGroup {
        if self.is_high_contrast() {
            ThemeGroup::HighContrast
        } else if self.is_light() {
            ThemeGroup::Light
        } else {
            ThemeGroup::Dark
        }
    }

    fn table(self) -> Option<&'static [(Color, Color); 33]> {
        match self {
            Theme::Dark => None,
            Theme::Light => Some(&pal::LIGHT),
            Theme::Cobalt => Some(&pal::COBALT),
            Theme::Nord => Some(&pal::NORD),
            Theme::Catppuccin => Some(&pal::CATPPUCCIN),
            Theme::TokyoNight => Some(&pal::TOKYO_NIGHT),
            Theme::Gruvbox => Some(&pal::GRUVBOX),
            Theme::RosePine => Some(&pal::ROSE_PINE),
            Theme::BitcoinOrange => Some(&pal::BITCOIN_ORANGE),
            Theme::BitcoinOrangeLight => Some(&pal::BITCOIN_ORANGE_LIGHT),
            Theme::Lamplight => Some(&pal::LAMPLIGHT),
            Theme::HighContrastDark => Some(&pal::HIGH_CONTRAST_DARK),
            Theme::HighContrastLight => Some(&pal::HIGH_CONTRAST_LIGHT),
            Theme::Mars => Some(&pal::MARS),
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

/// The groups Settings offers the themes in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeGroup {
    /// Dark text on a light page.
    Light,
    /// Light text on a dark page.
    Dark,
    /// Text 7:1 or more, light or dark.
    HighContrast,
}

impl ThemeGroup {
    /// Every group, in the order Settings offers them.
    pub const ALL: [ThemeGroup; 3] = [
        ThemeGroup::Light,
        ThemeGroup::Dark,
        ThemeGroup::HighContrast,
    ];

    /// The label Settings puts over the group.
    pub fn name(self) -> &'static str {
        match self {
            ThemeGroup::Light => EN.ui_themes_light,
            ThemeGroup::Dark => EN.ui_themes_dark,
            ThemeGroup::HighContrast => EN.ui_themes_high_contrast,
        }
    }

    /// The themes in this group ([`Theme::group`]), in order of their
    /// names.
    pub fn themes(self) -> impl Iterator<Item = Theme> {
        let mut all = Theme::ALL;
        all.sort_unstable_by_key(|t| t.name());
        all.into_iter().filter(move |t| t.group() == self)
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
    /// Terminus: the text-mode face a firmware sketch is set in
    /// ([`sketch`]).
    F,
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

/// Where the caret and the selection are in the field typing goes to
/// (`docs/history/NEW-WALLET.md` §13.2), in characters from the field's start.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Edit<A> {
    /// The field they are in, by the action that focuses it. In any
    /// other field the caret is at the end and nothing is selected.
    pub field: Option<A>,
    /// The caret; at the end when `None`.
    pub caret: Option<usize>,
    /// The selection's other end, while something is selected.
    pub anchor: Option<usize>,
}

impl<A> Default for Edit<A> {
    fn default() -> Self {
        Edit {
            field: None,
            caret: None,
            anchor: None,
        }
    }
}

impl<A> Edit<A> {
    /// The caret in a text `len` characters long.
    pub fn caret_in(&self, len: usize) -> usize {
        self.caret.map_or(len, |c| c.min(len))
    }

    /// The characters selected in a text `len` characters long, from
    /// and to, when any are.
    pub fn selected(&self, len: usize) -> Option<(usize, usize)> {
        let a = self.anchor?.min(len);
        let c = self.caret_in(len);
        (a != c).then(|| (a.min(c), a.max(c)))
    }

    /// Whether the whole of a text `len` characters long is selected.
    pub fn all(&self, len: usize) -> bool {
        len > 0 && self.selected(len) == Some((0, len))
    }
}

/// Where a line of typing drew its characters: the pixel column of the
/// edge before each character in view, the first of them `first`, and
/// the one after the last. A press or a drag there places the caret.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldAt<A> {
    /// The action the field is pressed with.
    pub action: A,
    /// The first character in view.
    pub first: usize,
    /// The edges, in pixels.
    pub edges: Vec<i32>,
}

impl<A> FieldAt<A> {
    /// The place between characters nearest pixel column `x`.
    pub fn index_at(&self, x: i32) -> usize {
        let j = self
            .edges
            .iter()
            .enumerate()
            .min_by_key(|(_, e)| (**e - x).abs())
            .map_or(0, |(j, _)| j);
        self.first + j
    }
}

/// A glyph: one of the icon face's, or one Faraday draws itself where the
/// face has none.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Glyph {
    /// One of the icon face's.
    Icon(Icon),
    /// A USB stick (`docs/BACKUP-FLOW.md` §17.17): Faraday's own drawing.
    Stick,
}

impl From<Icon> for Glyph {
    fn from(icon: Icon) -> Glyph {
        Glyph::Icon(icon)
    }
}

/// One frame's drawing context.
pub struct Ui<'a, A> {
    /// The canvas.
    pub c: &'a mut Canvas,
    /// Pixels per design unit.
    pub f: f32,
    /// The canvas's type ramp over its length scale.
    tr: f32,
    /// Where each action can be pressed, in pixels.
    pub hits: &'a mut Vec<(Rect, A)>,
    /// The action under a finger that is down.
    pub pressed: Option<A>,
    /// When set, hits are only taken inside this layer (a sheet).
    pub layer: u8,
    /// Where design (0, 0) is, in design units: screens are drawn in a
    /// box that may sit inside a larger window.
    pub ox: f32,
    /// See [`Ui::ox`].
    pub oy: f32,
    /// Where the caret and the selection are in the focused field.
    pub edit: Edit<A>,
    /// Where each line of typing drew its characters this frame.
    pub fields: Vec<FieldAt<A>>,
    /// What the screen's back link does, when it drew one: the mouse's
    /// back button does the same.
    pub back_link: Option<A>,
    /// Where in `hits` the back link's own press is: a press there, and
    /// not another of the same action, is the way back.
    pub back_hit: Option<usize>,
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
    pub pin: Option<(String, Style, A)>,
    /// A small panel's sheet keeps its buttons at its foot: they are
    /// handed to it rather than drawn in its body.
    pub sheet_pinning: bool,
    /// The buttons a sheet's body handed over.
    pub sheet_pin: Option<Vec<(String, Style, A)>>,
    /// The overlay scrollbar for the scrolling region, when it shows:
    /// its opacity (0–255) and the region's offset in units.
    pub bar: Option<(u8, f32)>,
    /// The palette.
    pub theme: Theme,
    /// The action under the pointer, with no button down.
    pub hovered: Option<A>,
    /// The screen's or the scrolling sheet's own region's offset
    /// ([`Slot::Page`]), units.
    pub offset: f32,
    /// The frosted copy of the page under the open sheet, kept from the
    /// frame the sheet opened on.
    pub frost: Option<Vec<u8>>,
    /// Where the Guided switch's pill is: 0 on Steps only, 1 on Guided.
    pub guided_shown: f32,
    /// On a small panel, which walk-through is shown on this screen: a
    /// step's place, or the app's own number for its About.
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
    /// Where the step column drew a chip that a press opens, this
    /// frame: its left edge, its foot and its width, units.
    pub chip_at: Option<(f32, f32, f32)>,
    /// Hits are clipped to this rather than to the drawing's clip while
    /// a card's body is revealed: what is not yet uncovered can still be
    /// pressed where it will be.
    hit_clip: Option<Rect>,
    /// The hit being taken draws its own hover look.
    quiet_hover: bool,
    /// What a press beside the open sheet does: the sheet's own way out.
    pub outside: Option<A>,
    /// The text caret shows this frame: it blinks.
    pub caret_on: bool,
    /// A caret was drawn this frame, shown or in the off half of its
    /// blink, so the next half wants a frame.
    pub caret_drawn: bool,
    /// The foot of the caret drawn this frame, units down the window:
    /// what the wide layout's keyboard keeps in view above it.
    pub caret_foot: Option<f32>,
    /// Actions every button draws dimmed this frame, whatever style the
    /// screen asked for: what the app refuses while it is in this state
    /// (the demo's actions that would make a secret, `docs/DEMO.md` §7).
    pub dimmed: Vec<A>,
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
    /// The screen's region, or the open sheet's when it scrolls.
    #[default]
    Page,
    /// Stick visit's Outbox list (`VisitState::out_offset`).
    VisitOut,
    /// The wallet card's body on a desktop (`Faraday::card_offset`).
    Card,
    /// Vault contents' item on a desktop (`VaultArea::item_offset`, `faraday-core`).
    VaultItem,
    /// Files' For the stick list on a desktop, beside From the stick
    /// (`Faraday::files_out_offset`).
    FilesOut,
    /// The backup board's list in the right-hand column on a desktop
    /// (`Faraday::board_list_offset`).
    BoardList,
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

impl<'a, A: Copy + PartialEq> Ui<'a, A> {
    /// A context over `c`.
    pub fn new(
        c: &'a mut Canvas,
        f: f32,
        hits: &'a mut Vec<(Rect, A)>,
        pressed: Option<A>,
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
            edit: Edit::default(),
            fields: Vec::new(),
            back_link: None,
            back_hit: None,
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
            chip_at: None,
            hit_clip: None,
            quiet_hover: false,
            outside: None,
            caret_on: true,
            caret_drawn: false,
            caret_foot: None,
            dimmed: Vec::new(),
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

    /// [`Ui::report_scroll_in`] for a region that shows the page's own
    /// overlay scrollbar at its right edge while it is the one scrolled.
    pub fn report_scroll_beside(&mut self, slot: Slot, view: Rect, max: f32, offset: f32) {
        self.report(slot, view, max, offset, false);
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
        // The page and a sheet are both the surface (`docs/DECISIONS.md`
        // F39).
        let page = self.col(SURFACE);
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

    /// The page under a popup, kept as it is drawn, not dimmed: copied
    /// once, and each frame after is that copy, as [`Ui::backdrop`] keeps
    /// its frosted one.
    pub fn still(&mut self) {
        match self.frost.as_deref() {
            Some(page) => self.c.restore(page),
            None => self.frost = Some(self.c.snapshot()),
        }
    }

    /// A soft shadow under something that floats at (x, y), w by h with
    /// corners of `r`, at Material 3's elevation `level` (1 to 5): drawn
    /// before it, black in every theme. The high-contrast themes draw
    /// none: a shadow there would lower the contrast of the text and
    /// edges beside it, and what floats has its outline.
    pub fn shadow(&mut self, x: f32, y: f32, w: f32, h: f32, r: f32, level: u8) {
        if self.theme.is_high_contrast() {
            return;
        }
        let rect = self.rect(x, y, w, h);
        self.c.elevation(rect, r * self.f, level, self.f);
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
        // The page and a sheet are both the surface (`docs/DECISIONS.md`
        // F39).
        let page = self.col(SURFACE);
        self.c.fill_rect(gap, page);
        for (r, _) in self.hits.iter_mut() {
            if r.intersect(&view) == *r {
                *r = Rect::new(r.x, r.y - self.stretch, r.w, r.h).intersect(&view);
            }
        }
    }

    /// The selection behind text `shown` drawn at (x, y) in a field of
    /// height `h`, when the field is focused and all of it is selected:
    /// for a field drawn as one string, whose caret stays at its end.
    #[allow(clippy::too_many_arguments)]
    pub fn selection(&mut self, x: f32, y: f32, h: f32, size: f32, w: W, shown: &str, on: bool) {
        if on && self.edit.all(shown.chars().count()) {
            let tw = self.measure(size, w, shown);
            self.selected_band(x, y, h, tw);
        }
    }

    /// The band behind selected text `tw` wide from x, in a field `h`
    /// tall at y.
    fn selected_band(&mut self, x: f32, y: f32, h: f32, tw: f32) {
        self.fill(
            x - 2.0,
            y + h * 0.2,
            tw + 4.0,
            h * 0.6,
            3.0,
            ACCENT.with_alpha(90),
        );
    }

    /// One line of typing (`docs/history/NEW-WALLET.md` §13.2): `text`, dots
    /// already for a masked field, from x in the middle of a box `h`
    /// tall at y, at most `room` wide. While `focused`, what is selected
    /// is drawn behind it and the caret at its place, and the part in
    /// view is the end or, with the caret before that, from the caret.
    /// Where each character starts is kept for `action`, so a press or
    /// a drag there places the caret. Returns the width drawn.
    #[allow(clippy::too_many_arguments)]
    pub fn typed(
        &mut self,
        x: f32,
        y: f32,
        h: f32,
        size: f32,
        w: W,
        color: Color,
        text: &str,
        room: f32,
        focused: bool,
        action: A,
    ) -> f32 {
        // The byte offset of each character's start, and of the end: the
        // text itself is never copied.
        let at: Vec<usize> = text
            .char_indices()
            .map(|(i, _)| i)
            .chain(std::iter::once(text.len()))
            .collect();
        let n = at.len() - 1;
        let edge: Vec<f32> = at
            .iter()
            .map(|&b| self.measure(size, w, &text[..b]))
            .collect();
        let edit = if focused { self.edit } else { Edit::default() };
        let caret = edit.caret_in(n);
        let mut from = 0;
        while from < n && edge[n] - edge[from] > room {
            from += 1;
        }
        let mut to = n;
        if caret < from {
            from = caret;
            while to > from && edge[to] - edge[from] > room {
                to -= 1;
            }
        }
        let left = |i: usize| x + edge[i] - edge[from];
        let selected = edit.selected(n);
        if let Some((lo, hi)) = selected {
            let (lo, hi) = (lo.clamp(from, to), hi.clamp(from, to));
            if hi > lo {
                self.selected_band(left(lo), y, h, edge[hi] - edge[lo]);
            }
        }
        self.text_mid(x, y, h, size, w, color, &text[at[from]..at[to]]);
        if focused && selected.is_none() {
            let ch = (size + 4.0).max(18.0);
            self.caret(left(caret) + 1.0, y + (h - ch) / 2.0, ch);
        }
        let edges = (from..=to).map(|i| self.px(left(i) + self.ox)).collect();
        self.fields.push(FieldAt {
            action,
            first: from,
            edges,
        });
        edge[to] - edge[from]
    }

    /// Records the screen's back link: drawn, it is what the mouse's
    /// back button does.
    pub fn back(&mut self, action: A) {
        if self.layer == 0 {
            self.back_link = Some(action);
            self.back_hit = self.hits.len().checked_sub(1);
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

    /// Whether anything drawn in the band from `y` to `y + h`, design
    /// units, can be seen: the band meets the clip, with a few pixels to
    /// spare for a stroke's or a shadow's edge. A row of a long list out
    /// of it is passed over, its height alone counted.
    pub fn shows(&self, y: f32, h: f32) -> bool {
        let band = self.rect(0.0, y, 1.0, h);
        let clip = self.c.clip();
        let spare = 4;
        band.y - spare < clip.bottom() && band.bottom() + spare > clip.y
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

    /// A card: a surface with a one-unit edge. The page is the surface
    /// too (`docs/DECISIONS.md` F39), so a card's resting edge, `LINE`,
    /// is drawn in the stronger `BORDER`: the edge is what sets it apart.
    pub fn card(&mut self, x: f32, y: f32, w: f32, h: f32, edge: Color) {
        let edge = if edge == LINE { BORDER } else { edge };
        self.fill(x, y, w, h, 12.0, SURFACE);
        self.stroke(x, y, w, h, 12.0, edge);
    }

    /// A theme as a tile drawn in its own colours: its page, a card with
    /// a primary button and the three states, and its name. The theme on
    /// screen is ringed in its accent; another is ringed while hovered.
    /// The theme a first start is in carries a Default mark after its
    /// name.
    #[allow(clippy::too_many_arguments)]
    pub fn theme_tile(
        &mut self,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        theme: Theme,
        current: bool,
        action: A,
    ) {
        use faraday_draw::tokens::{
            THEME_TILE_INSET as IN, THEME_TILE_LINE as STEP, THEME_TILE_SAMPLE,
            THEME_TILE_TEXT as SIZE,
        };
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
        self.fill(x, y, w, h, IN, BG);
        self.stroke(x, y, w, h, IN, LINE);
        self.fill(
            x + IN,
            y + IN,
            w - 2.0 * IN,
            THEME_TILE_SAMPLE,
            6.0,
            SURFACE,
        );
        // The button and the three states, centred down the sample card.
        let mid = y + IN + THEME_TILE_SAMPLE / 2.0;
        self.fill(x + 17.0, mid - SIZE / 2.0, 24.0, SIZE, 6.0, ACCENT);
        for (i, c) in [OK, WARN, ERR].into_iter().enumerate() {
            self.dot(x + w - 38.0 + 9.0 * i as f32, mid, 3.0, c);
        }
        // A name too long for the tile wraps at its spaces, the tile being
        // drawn taller for a third line (`theme_tile_h`). Each line is a
        // part of the name itself: no text is made unless a word alone is
        // too wide and is cut.
        let room = w - 2.0 * IN;
        let (lines, n) = self.theme_name_lines(theme, room);
        let name = theme.name();
        let mut ty = y + h - 22.0 - STEP * (n - 1) as f32;
        let mut last = 0.0;
        for &(a, b) in &lines[..n] {
            let line = &name[a..b];
            last = self.measure(SIZE, W::S, line);
            if last <= room {
                self.text(x + IN, ty, SIZE, W::S, TEXT, line);
            } else {
                let cut = self.fit(SIZE, W::S, line, room);
                last = self.text(x + IN, ty, SIZE, W::S, TEXT, &cut);
            }
            ty += STEP;
        }
        // The theme a first start is in: a Default mark at the right of
        // its name's last line, where it fits beside the name.
        if theme == Theme::default() {
            let mark = self.measure(SIZE, W::R, EN.ui_default_theme);
            if last + faraday_draw::tokens::GAP + mark <= room {
                let at = x + w - IN - mark;
                self.text(at, ty - STEP, SIZE, W::R, MUTED, EN.ui_default_theme);
            }
        }
        self.theme = now;
        self.hit(x, y, w, h, action);
    }

    /// The narrowest a theme tile can be with every theme's name on at
    /// most two lines.
    pub fn theme_tile_min(&self) -> f32 {
        use faraday_draw::tokens::{THEME_TILE_INSET, THEME_TILE_MIN, THEME_TILE_TEXT};
        let width = |s: &str| self.measure(THEME_TILE_TEXT, W::S, s);
        Theme::ALL
            .into_iter()
            .map(|t| {
                let name = t.name();
                // Whole, or broken at the space that leaves the longer
                // line shortest.
                name.match_indices(' ')
                    .map(|(i, _)| width(&name[..i]).max(width(&name[i + 1..])))
                    .fold(width(name), f32::min)
                    + 2.0 * THEME_TILE_INSET
            })
            .fold(THEME_TILE_MIN, f32::max)
    }

    /// How tall theme tiles `w` wide are drawn: room for a name on two
    /// lines, and a line taller for each line beyond.
    pub fn theme_tile_h(&self, w: f32) -> f32 {
        use faraday_draw::tokens::{THEME_TILE_HEIGHT, THEME_TILE_INSET, THEME_TILE_LINE};
        let lines = Theme::ALL
            .into_iter()
            .map(|t| self.theme_name_lines(t, w - 2.0 * THEME_TILE_INSET).1)
            .max()
            .unwrap_or(1);
        THEME_TILE_HEIGHT + THEME_TILE_LINE * lines.saturating_sub(2) as f32
    }

    /// How many theme tiles a row `inner` units across holds: as many as
    /// tiles at [`Ui::theme_tile_min`] allow, so each group
    /// ([`ThemeGroup`]) takes as few rows as it can and the groups'
    /// tiles line up.
    pub fn theme_grid(&self, inner: f32) -> usize {
        use faraday_draw::tokens::THEME_TILE_GAP as GAP;
        let n = Theme::ALL.len();
        (((inner + GAP) / (self.theme_tile_min() + GAP)).floor() as usize).clamp(1, n)
    }

    /// The themes as tiles in their groups ([`ThemeGroup`]), each group
    /// under its label and its tiles in rows of `per_row`, `w` wide from
    /// (x, y); the theme on screen, `current`, is ringed. `action` names
    /// what a press on a theme's tile does. Returns the height used, as
    /// [`Ui::theme_groups_h`] works it out.
    pub fn theme_groups(
        &mut self,
        x: f32,
        y: f32,
        w: f32,
        per_row: usize,
        current: Theme,
        action: impl Fn(Theme) -> A,
    ) -> f32 {
        use faraday_draw::tokens::{
            CAPTION, THEME_GROUP_GAP, THEME_GROUP_HEAD, THEME_TILE_GAP, THEME_TILE_ROW_GAP,
        };
        let per_row = per_row.max(1);
        let tile_w = theme_tile_w(w, per_row);
        let tile_h = self.theme_tile_h(tile_w);
        let mut at = y;
        for (g, group) in ThemeGroup::ALL.into_iter().enumerate() {
            if g > 0 {
                at += THEME_GROUP_GAP;
            }
            self.text(x, at, CAPTION, W::R, MUTED, group.name());
            at += THEME_GROUP_HEAD;
            let mut n = 0;
            for theme in group.themes() {
                self.theme_tile(
                    x + (n % per_row) as f32 * (tile_w + THEME_TILE_GAP),
                    at + (n / per_row) as f32 * (tile_h + THEME_TILE_ROW_GAP),
                    tile_w,
                    tile_h,
                    theme,
                    theme == current,
                    action(theme),
                );
                n += 1;
            }
            at += n.div_ceil(per_row) as f32 * (tile_h + THEME_TILE_ROW_GAP) - THEME_TILE_ROW_GAP;
        }
        at - y
    }

    /// How tall [`Ui::theme_groups`] draws the groups, `w` wide in rows
    /// of `per_row`.
    pub fn theme_groups_h(&self, w: f32, per_row: usize) -> f32 {
        use faraday_draw::tokens::{THEME_GROUP_GAP, THEME_GROUP_HEAD, THEME_TILE_ROW_GAP};
        let per_row = per_row.max(1);
        let tile_h = self.theme_tile_h(theme_tile_w(w, per_row));
        let groups = ThemeGroup::ALL.into_iter().map(|g| {
            let rows = g.themes().count().div_ceil(per_row);
            THEME_GROUP_HEAD + rows as f32 * (tile_h + THEME_TILE_ROW_GAP) - THEME_TILE_ROW_GAP
        });
        groups.sum::<f32>() + THEME_GROUP_GAP * (ThemeGroup::ALL.len() - 1) as f32
    }

    /// A theme's name in lines no wider than `max`, broken at spaces: up
    /// to four lines, as byte ranges of the name, and how many there
    /// are. Words past a fourth line join the fourth.
    fn theme_name_lines(&self, theme: Theme, max: f32) -> ([(usize, usize); 4], usize) {
        use faraday_draw::tokens::THEME_TILE_TEXT;
        let name = theme.name();
        let mut lines = [(0, 0); 4];
        let mut n = 0;
        let mut a = 0;
        for word in name.split(' ') {
            let b = a + word.len();
            let joins = n > 0
                && (n == lines.len()
                    || self.measure(THEME_TILE_TEXT, W::S, &name[lines[n - 1].0..b]) <= max);
            if joins {
                lines[n - 1].1 = b;
            } else {
                lines[n] = (a, b);
                n += 1;
            }
            a = b + 1;
        }
        (lines, n)
    }

    /// Faraday's mark in a box `w` by `h` units at (x, y): the keyhole
    /// shield of the boot logo (`faraday/image/common/faraday-mark.txt`),
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

    /// Faraday's name, the wordmark of `assets/wordmark.txt`, its capitals
    /// `cap` units tall with their top-left at (x, y). Returns its width.
    pub fn wordmark(&mut self, x: f32, y: f32, cap: f32, color: Color) -> f32 {
        self.wordmark_named(x, y, cap, color, NameSuffix::None)
    }

    /// The wordmark with the word of an app's name after it
    /// ("Faraday Starter", "Faraday Desktop"), as one name in one lettering.
    /// The suffix is in Archivo Expanded Regular, in the grey of
    /// secondary text (`docs/DECISIONS.md` F16).
    pub fn wordmark_named(
        &mut self,
        x: f32,
        y: f32,
        cap: f32,
        color: Color,
        suffix: NameSuffix,
    ) -> f32 {
        let mark = wordmark_outlines();
        let s = cap / WORDMARK_CAP;
        let k = s * self.f;
        let (ox, oy) = ((x + self.ox) * self.f, (y + self.oy) * self.f);
        let place = |o: &Vec<(f32, f32)>, dx: f32| -> Vec<(f32, f32)> {
            o.iter()
                .map(|&(fx, fy)| (ox + (fx + dx - mark.left) * k, oy + (fy + WORDMARK_CAP) * k))
                .collect()
        };
        let outlines: Vec<Vec<(f32, f32)>> = mark.outlines.iter().map(|o| place(o, 0.0)).collect();
        let contours: Vec<&[(f32, f32)]> = outlines.iter().map(Vec::as_slice).collect();
        let fill = self.col(color);
        self.c.fill_contours(&contours, fill);
        let mut right = mark.right;
        if let Some(word) = suffix_word(suffix) {
            let start = mark.right + SUFFIX_GAP;
            let outlines: Vec<Vec<(f32, f32)>> =
                word.outlines.iter().map(|o| place(o, start)).collect();
            let contours: Vec<&[(f32, f32)]> = outlines.iter().map(Vec::as_slice).collect();
            let grey = self.col(DIM);
            self.c.fill_contours(&contours, grey);
            right = start + word.right;
        }
        (right - mark.left) * s
    }

    /// The caret of the field typing goes to, `h` units tall at (x, y),
    /// in the off half of its blink not drawn.
    pub fn caret(&mut self, x: f32, y: f32, h: f32) {
        self.caret_drawn = true;
        self.caret_foot = Some(y + h + self.oy);
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

    /// A vertical rule: one unit wide, at least a pixel.
    pub fn vrule(&mut self, x: f32, y: f32, h: f32, color: Color) {
        let color = self.col(color);
        let r = self.rect(x, y, 1.0, h);
        let rect = Rect::new(r.x, r.y, 1.max(self.px(1.0)), r.h);
        self.c.fill_rect(rect, color);
    }

    fn font(&self, size: f32, w: W) -> Font {
        let family = match w {
            W::R => Family::Regular,
            W::S => Family::SemiBold,
            W::M => Family::Mono,
            W::F => Family::Firmware,
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

    /// How many lines `s` breaks onto at `max` width: the same
    /// break-at-a-space rule [`Ui::wrap`] draws, counted rather than
    /// drawn, so a caller can grow a row before drawing its background.
    pub fn wrap_lines(&self, size: f32, w: W, s: &str, max: f32) -> usize {
        let mut lines = 1usize;
        let mut cur = String::new();
        for word in s.split_whitespace() {
            let candidate = if cur.is_empty() {
                word.to_string()
            } else {
                format!("{cur} {word}")
            };
            if cur.is_empty() || self.measure(size, w, &candidate) <= max {
                cur = candidate;
            } else {
                lines += 1;
                cur = word.to_string();
            }
        }
        lines
    }

    /// `s` cut with an ellipsis to fit `max` units: the longest start of
    /// it that fits with "…" after it, or nothing when "…" alone does
    /// not. One pass over its characters' advances, which only add up,
    /// so the first start that does not fit ends the search.
    pub fn fit(&self, size: f32, w: W, s: &str, max: f32) -> String {
        let face = self.font(size, w).sized(self.c.scale());
        let fits = |pen: u32| text::px_from_64(pen) as f32 / self.f <= max;
        let whole: u32 = s.chars().map(|c| face.advance_64(c)).sum();
        if fits(whole) {
            return s.to_string();
        }
        let ellipsis = face.advance_64('…');
        if !fits(ellipsis) {
            return String::new();
        }
        let (mut pen, mut end) = (0u32, 0usize);
        for (i, c) in s.char_indices() {
            pen += face.advance_64(c);
            if !fits(pen + ellipsis) {
                break;
            }
            end = i + c.len_utf8();
        }
        format!("{}…", &s[..end])
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

    /// `s` wrapped to `width` with the bytes `mark` of it drawn in
    /// `mark_color` and heavier: the monospace face has one weight, so the
    /// marked run is drawn again [`faraday_draw::tokens::MARK_BOLD`] to the right, which keeps
    /// every character where it would be unmarked. Breaks as [`Ui::wrap`]
    /// does. Returns the height used.
    pub fn wrap_marked(
        &mut self,
        (x, y, width): (f32, f32, f32),
        size: f32,
        w: W,
        colors: (Color, Color),
        s: &str,
        mark: std::ops::Range<usize>,
    ) -> f32 {
        let (color, mark_color) = (self.col(colors.0), self.col(colors.1));
        let font = self.font(size, w);
        let face = font.sized(self.c.scale());
        let lines = text::wrap(&face, s, self.px(width));
        let lh = face.line_height();
        let bold = self.px(faraday_draw::tokens::MARK_BOLD).max(1);
        let (x0, mut y0) = (self.px(x + self.ox), self.px(y + self.oy));
        for &(start, end) in &lines {
            let mut pen = x0;
            let cuts = [
                (start, mark.start.clamp(start, end), false),
                (
                    mark.start.clamp(start, end),
                    mark.end.clamp(start, end),
                    true,
                ),
                (mark.end.clamp(start, end), end, false),
            ];
            for (a, b, marked) in cuts {
                if a >= b {
                    continue;
                }
                let part = &s[a..b];
                if marked {
                    self.c.text(pen + bold, y0, part, font, mark_color);
                    pen += self.c.text(pen, y0, part, font, mark_color);
                } else {
                    pen += self.c.text(pen, y0, part, font, color);
                }
            }
            y0 += lh;
        }
        (lines.len() as i32 * lh) as f32 / self.f
    }

    /// An icon centred in a square of side `boxw` at (x, y).
    pub fn icon(
        &mut self,
        x: f32,
        y: f32,
        boxw: f32,
        icon: impl Into<Glyph>,
        size: f32,
        color: Color,
    ) {
        let color = self.col(color);
        match icon.into() {
            Glyph::Icon(icon) => {
                let rect = self.rect(x, y, boxw, boxw);
                self.c.icon(rect, icon, size, color);
            }
            Glyph::Stick => self.stick_glyph(x + boxw / 2.0, y + boxw / 2.0, size, color),
        }
    }

    /// The USB stick glyph centred on (cx, cy), `size` units across: a
    /// body with rounded corners and its connector at the right end, two
    /// holes in the connector. Drawn as outlines, at the icon face's size.
    fn stick_glyph(&mut self, cx: f32, cy: f32, size: f32, color: Color) {
        let f = self.f;
        let (ox, oy) = (self.ox, self.oy);
        // In units of the glyph's size, from its centre.
        let px = |u: f32, v: f32| ((cx + u * size + ox) * f, (cy + v * size + oy) * f);
        // The body, with its corners cut round in three steps each.
        let (l, r, t, b, k) = (-0.5, 0.18, -0.27, 0.27, 0.1);
        let mut body = Vec::with_capacity(16);
        let corners = [
            (r - k, t + k, -std::f32::consts::FRAC_PI_2),
            (r - k, b - k, 0.0),
            (l + k, b - k, std::f32::consts::FRAC_PI_2),
            (l + k, t + k, std::f32::consts::PI),
        ];
        for (ccx, ccy, a0) in corners {
            for j in 0..4 {
                let a = a0 + j as f32 * std::f32::consts::FRAC_PI_6;
                body.push(px(ccx + k * a.cos(), ccy + k * a.sin()));
            }
        }
        // The connector, overlapping the body's end, and its two holes
        // wound the other way round.
        let (cl, cr, ct, cb) = (0.12, 0.5, -0.16, 0.16);
        let plug = [px(cl, ct), px(cr, ct), px(cr, cb), px(cl, cb)];
        let hole = |hl: f32, hr: f32| [px(hl, -0.06), px(hl, 0.06), px(hr, 0.06), px(hr, -0.06)];
        let (h1, h2) = (hole(0.24, 0.33), hole(0.38, 0.45));
        // The holes are cut from the connector alone, where it is clear
        // of the body.
        self.c.fill_contours(&[&body, &plug, &h1, &h2], color);
    }

    /// Makes a region pressable: the part of it inside the clip, so what
    /// is scrolled out of sight cannot be pressed.
    ///
    /// Under the pointer it shows so: a faint wash over what was drawn
    /// there, unless it is pressed, draws its own hover look (a button),
    /// or covers so much of the screen that a wash would light the page.
    pub fn hit(&mut self, x: f32, y: f32, w: f32, h: f32, action: A) {
        // A dimmed action takes no press, wherever it is drawn.
        if self.dimmed.contains(&action) {
            return;
        }
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

    /// [`Ui::hit`] for a region that draws its own hover look: no wash.
    pub fn hit_quiet(&mut self, x: f32, y: f32, w: f32, h: f32, action: A) {
        self.quiet_hover = true;
        self.hit(x, y, w, h, action);
        self.quiet_hover = false;
    }

    /// Presses everywhere outside the box at (x, y), w by h, take
    /// `action`: a sheet that a press beside it closes. No hover look.
    pub fn hit_around(&mut self, x: f32, y: f32, w: f32, h: f32, action: A) {
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
    pub fn is_hovered(&self, action: A) -> bool {
        self.hovered == Some(action) && self.pressed != Some(action)
    }

    /// Whether `action` is under a finger.
    pub fn is_pressed(&self, action: A) -> bool {
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
        action: A,
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
        let style = if self.dimmed.contains(&action) {
            Style::Disabled
        } else {
            style
        };
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

    /// A chip inside the empty entry field at (`x`, `y`, `w`, `h`), at its
    /// text inset, secondary-styled and vertically centred
    /// (`docs/MODES.md` §6.1). Its hit target is the chip, at the touch
    /// floor's height on a small panel and the field's on a wide one.
    /// Returns the chip's width; the caller passes `label`, the shorter
    /// one where the field is narrower than the chip.
    pub fn field_action(&mut self, x: f32, y: f32, w: f32, h: f32, label: &str, action: A) -> f32 {
        use faraday_draw::tokens::{
            FIELD_ACTION_HEIGHT as CH, FIELD_ACTION_INSET as IN, FIELD_ACTION_TEXT as TX, TOUCH,
        };
        let width = (self.measure(TX, W::S, label) + 2.0 * IN).min(w - 2.0 * IN);
        let (cx, cy) = (x + IN, y + (h - CH) / 2.0);
        let pressed = self.is_pressed(action);
        let hovered = self.is_hovered(action);
        if pressed {
            self.fill(cx, cy, width, CH, CH / 2.0, INNER);
        } else if hovered {
            self.fill(cx, cy, width, CH, CH / 2.0, INNER.with_alpha(150));
        }
        self.stroke(
            cx,
            cy,
            width,
            CH,
            CH / 2.0,
            if hovered { DIM } else { BORDER },
        );
        let tw = self.measure(TX, W::S, label);
        self.text_mid(cx + (width - tw) / 2.0, cy, CH, TX, W::S, TEXT, label);
        let hit_h = if self.compact {
            TOUCH.max(CH)
        } else {
            h.max(CH)
        };
        self.quiet_hover = true;
        self.hit(cx, y + (h - hit_h) / 2.0, width, hit_h, action);
        self.quiet_hover = false;
        width
    }

    /// A floating action: a pill in the accent colour over the page, its
    /// icon and its word, its bottom-right corner at (`right`, `bottom`).
    /// Returns its width.
    pub fn fab(&mut self, right: f32, bottom: f32, icon: Icon, label: &str, action: A) -> f32 {
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
        let level = if self.is_hovered(action) {
            faraday_draw::tokens::ELEVATION_RAISED
        } else {
            faraday_draw::tokens::ELEVATION_FAB
        };
        self.shadow(x, y, width, h, h / 2.0, level);
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
    pub fn word_pill(&mut self, x: f32, y: f32, word: &str, action: A) -> f32 {
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
        self.pill_in(x, y, h, label, MUTED)
    }

    /// [`Ui::pill`] with its label in `color`.
    pub fn pill_in(&mut self, x: f32, y: f32, h: f32, label: &str, color: Color) -> f32 {
        let width = self.measure(12.0, W::R, label) + 24.0;
        self.fill(x, y, width, h, h / 2.0, SURFACE);
        self.stroke(x, y, width, h, h / 2.0, BORDER);
        self.text_mid(x + 12.0, y, h, 12.0, W::R, color, label);
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

    /// A checkbox. One ticked and not enabled shows its tick dimmed: it is
    /// done, and a press does not undo it.
    pub fn checkbox(&mut self, x: f32, y: f32, on: bool, enabled: bool) {
        if on {
            self.fill(x, y, 18.0, 18.0, 4.0, if enabled { ACCENT } else { INNER });
            let tick = if enabled { ON_ACCENT } else { DIM };
            self.icon(x, y, 18.0, Icon::Done, 11.0, tick);
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

#[cfg(test)]
mod tests {
    use super::*;
    use faraday_shell_api::{BootState, DisplayInfo, SecureHardware};

    /// WCAG relative luminance of a colour.
    fn luminance(c: Color) -> f32 {
        let lin = |v: u8| {
            let v = f32::from(v) / 255.0;
            if v <= 0.03928 {
                v / 12.92
            } else {
                ((v + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * lin(c.r) + 0.7152 * lin(c.g) + 0.0722 * lin(c.b)
    }

    /// WCAG contrast ratio of two colours.
    fn contrast(a: Color, b: Color) -> f32 {
        let (x, y) = (luminance(a), luminance(b));
        (x.max(y) + 0.05) / (x.min(y) + 0.05)
    }

    #[test]
    fn text_and_tones_are_legible_on_the_page_and_the_card_in_every_theme() {
        let mut short = Vec::new();
        for theme in Theme::ALL {
            let high = theme.is_high_contrast();
            for (name, fg, standard) in [
                ("text", pal::TEXT, 4.5),
                ("accent", pal::ACCENT, 4.5),
                ("positive", pal::OK, 4.5),
                ("warning", pal::WARN, 4.5),
                ("dimmed", pal::DIM, 3.0),
            ] {
                // Bitcoin Orange Light takes the orange itself as its
                // accent, at 2.1:1 (owner, 2026-10-11).
                if theme == Theme::BitcoinOrangeLight && name == "accent" {
                    continue;
                }
                let need = if high { 7.0 } else { standard };
                for (on_name, on) in [("page", pal::BG), ("card", pal::SURFACE)] {
                    let got = contrast(theme.color(fg), theme.color(on));
                    if got < need {
                        short.push(format!(
                            "{} {name} on the {on_name} is {got:.2}:1, needs {need}",
                            theme.id()
                        ));
                    }
                }
            }
        }
        assert!(short.is_empty(), "{short:#?}");
    }

    /// A codex32 worksheet's characters read on its checksum squares and
    /// its table rows in every theme (`docs/CODEX32.md` §7.5), and the
    /// rows stand apart from the card they are on.
    #[test]
    fn a_codex32_worksheets_characters_are_legible_on_every_fill_in_every_theme() {
        let mut short = Vec::new();
        for theme in Theme::ALL {
            let need = if theme.is_high_contrast() { 7.0 } else { 4.5 };
            for (name, fill) in [
                ("checksum square", pal::CODEX32_CHECKSUM_FILL),
                ("table row", pal::CODEX32_LOOKUP_FILL),
            ] {
                let got = contrast(theme.color(pal::TEXT), theme.color(fill));
                if got < need {
                    short.push(format!("{} text on a {name} is {got:.2}:1", theme.id()));
                }
                if theme.color(fill) == theme.color(pal::SURFACE) {
                    short.push(format!("{} {name} is the card's own colour", theme.id()));
                }
            }
        }
        assert!(short.is_empty(), "{short:#?}");
    }

    #[test]
    fn the_high_contrast_themes_draw_edges_at_3_to_1() {
        for theme in Theme::ALL.into_iter().filter(|t| t.is_high_contrast()) {
            for (name, edge) in [
                ("card edge", pal::LINE),
                ("rule", pal::INNER),
                ("button edge", pal::BORDER),
                ("place edge", pal::PLACE_EDGE),
                ("chip edge", pal::CHIP_EDGE),
                ("focus ring", pal::ACCENT),
            ] {
                let got = contrast(theme.color(edge), theme.color(pal::BG));
                assert!(got >= 3.0, "{} {name} is {got:.2}:1", theme.id());
            }
        }
    }

    #[test]
    fn a_kept_theme_id_names_its_theme() {
        for theme in Theme::ALL {
            assert_eq!(Theme::from_id(theme.id()), Some(theme));
        }
    }

    /// `fit` as it was until 2026-10-21: one character off the end at a
    /// time, the rest measured again with "…" after it each time.
    fn fit_by_popping(ui: &Ui<u8>, size: f32, w: W, s: &str, max: f32) -> String {
        if ui.measure(size, w, s) <= max {
            return s.to_string();
        }
        let mut chars: Vec<char> = s.chars().collect();
        while !chars.is_empty() {
            chars.pop();
            let t: String = chars.iter().collect::<String>() + "…";
            if ui.measure(size, w, &t) <= max {
                return t;
            }
        }
        String::new()
    }

    #[test]
    fn a_name_too_long_for_its_room_is_cut_where_it_always_was() {
        let long = "savings-2-of-3-multisig-wallet-backup-".repeat(6);
        let names = [
            "",
            "a",
            "…",
            "savings-unsigned.psbt",
            "Wallet · 2-of-3 · 8f3a91c2 · wsh(sortedmulti)",
            "Grüße – ñandú · 漢字のテスト · Ωmega",
            "WWWWWWWWWWWWWWWWWWWWiiiiiiiiiiiiiiiiiiii",
            long.as_str(),
        ];
        for dpi in [160, 286, 320] {
            let mut canvas = Canvas::new(&DisplayInfo {
                width: 640,
                height: 480,
                dpi,
                inset_bottom: 0,
                inset_top: 0,
                buttons: 0,
                camera_fixed: false,
                secure: SecureHardware::None,
                boot: BootState::Unknown,
                memory_mib: None,
            });
            let mut hits = Vec::new();
            let f = f32::from(dpi) / 160.0;
            let ui: Ui<u8> = Ui::new(&mut canvas, f, &mut hits, None);
            for name in names {
                for (size, w) in [(12.0, W::R), (14.0, W::M), (13.0, W::S)] {
                    for max in [0.0, 3.0, 9.5, 20.0, 47.25, 80.0, 133.0, 260.0, 1000.0] {
                        assert_eq!(
                            ui.fit(size, w, name, max),
                            fit_by_popping(&ui, size, w, name, max),
                            "{name:?} at {size} {w:?} in {max} at {dpi} dpi"
                        );
                    }
                }
            }
        }
    }
}
