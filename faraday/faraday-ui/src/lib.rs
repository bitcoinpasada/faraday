//! Faraday's drawing: what the desktop app and Faraday Starter draw
//! their screens with.
//!
//! Drawing in design units. Every screen is laid out on a canvas scaled
//! to fill the display, with Faraday's palette and the
//! themes beside it. Text is `faraday-draw`'s
//! Noto Sans and Noto Sans Mono. [`Ui`] is generic over the app's own
//! action type: what a press on a region does is the caller's to name.

mod appearance;
mod controls;
pub mod demo;
pub mod hold;
pub mod how;
pub mod keyboard;
mod lists;
mod mark;
pub mod menu;
pub mod nav;
pub mod parts;
mod press;
pub mod region;
pub mod roles;
pub mod sheet;
pub mod snackbar;
pub mod stick;
pub mod tabs;
mod theme;
pub mod tooltip;

pub use controls::{Reach, Style, focus_step, reach_at};
pub use pal::{Paint, Role, ThemeName};
pub use press::{Plain, PressMark, Ring, Shape, Unpressed};
pub use region::{Scrolled, Slot};
pub use theme::{Contrast, Mode, TextSize, Theme};

use faraday_draw::canvas::ShapeKey;
use faraday_draw::fonts::Family;
use faraday_draw::text::{self, TextAlign};
use faraday_draw::widgets::Icon;
use faraday_draw::{Canvas, Color, Font, Rect};
use std::sync::OnceLock;

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

    /// Where the thumb starts along the track, the region scrolled
    /// `offset` of `max` units.
    pub fn at(&self, offset: f32, max: f32) -> i32 {
        ((self.track - self.thumb) as f32 * (offset / max).clamp(0.0, 1.0)).round() as i32
    }
}

/// An overlay scrollbar's thickness, in pixels at `f` to the unit.
fn bar_thickness(f: f32) -> i32 {
    (BAR_W * f).round().max(3.0) as i32
}

/// Faraday's colours: the roles, the generated schemes and the paints.
pub mod pal;

/// Faraday's name as outlines: SVG path data in font units.
const WORDMARK: &str = include_str!("../assets/wordmark.txt");

/// The letters of the word after the wordmark: `assets/wordmark-suffix.txt`.
const WORDMARK_SUFFIX: &str = include_str!("../assets/wordmark-suffix.txt");

/// Font units from the wordmark's right edge to where the suffix's first
/// letter starts (its pen, before the letter's own left bearing).
const SUFFIX_GAP: f32 = 364.0;

/// The name's width with its capitals `cap` units tall, as
/// [`Ui::wordmark_named`] draws it with `suffix`. Arithmetic on outlines
/// read once.
pub fn wordmark_width(cap: f32, suffix: NameSuffix) -> f32 {
    let mark = wordmark_outlines();
    let right = match suffix_word(suffix) {
        Some(word) => mark.right + SUFFIX_GAP + word.right,
        None => mark.right,
    };
    (right - mark.left) * cap / WORDMARK_CAP
}

/// The word drawn after the wordmark in an app's name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NameSuffix {
    /// FARADAY alone.
    None,
    /// Faraday Starter.
    Starter,
    /// Faraday Demo.
    Demo,
    /// Faraday Offline: the stick and the Pi.
    Offline,
}

/// The wordmark's cap height in its font units.
const WORDMARK_CAP: f32 = 687.0;

/// The shapes a canvas keeps filled for the name and the mark
/// ([`faraday_draw::canvas::ShapeKey`]): FARADAY; the word after it, and
/// the word under it, each plus the suffix's own number; the mark's lit
/// half and its body.
const SHAPE_WORDMARK: u32 = 1;
const SHAPE_SUFFIX: u32 = 0x10;
const SHAPE_SUFFIX_UNDER: u32 = 0x20;
const SHAPE_MARK_LIT: u32 = 0x30;
const SHAPE_MARK_BODY: u32 = 0x31;

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
    static WORDS: OnceLock<[SuffixWord; 3]> = OnceLock::new();
    let key = match which {
        NameSuffix::None => return None,
        NameSuffix::Starter => 0,
        NameSuffix::Demo => 1,
        NameSuffix::Offline => 2,
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
            build(spelled.get(2).copied().unwrap_or("")),
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
    /// The person's text size factor (`docs/TEXT-SIZE.md` §2): every
    /// type role's size and line height times it, 1 inside
    /// [`Ui::fixed_text`].
    text: f32,
    /// Where each action can be pressed, in pixels. Private: a press is
    /// made with the functions of `press.rs` and kept or trimmed with
    /// its mark, rewind, clip and take methods.
    hits: &'a mut Vec<(Rect, A)>,
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
    /// On a small panel, where a step flow's bar tools begin (the ?, the
    /// eye, Cancel and the rest at its right end), set by the frame
    /// before the page is drawn so the bar's title ends before it.
    pub bar_tools_x: Option<f32>,
    /// A small panel's sheet keeps its buttons at its foot: they are
    /// handed to it rather than drawn in its body.
    pub sheet_pinning: bool,
    /// The buttons a sheet's body handed over.
    pub sheet_pin: Option<Vec<(String, Style, A)>>,
    /// Where this frame's floating action button stands, in the
    /// window's units (x, y, width, height): a snackbar drawn after it
    /// stands above it, as Material places one (`snackbar.rs`).
    fab_at: Option<(f32, f32, f32, f32)>,
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
    /// How open the codex32 dice sheet shows: 0 closed, 1 open.
    pub dice_shown: f32,
    /// How strongly the codex32 pad's key of the roll shown is emphasised
    /// over its pressed state: 1 as it is pressed, 0 once settled (and
    /// always under Reduce motion).
    pub press_shown: f32,
    /// On the wide layout, the page's name and its way back (the name of
    /// where it lands, and the press), handed over by the page for the
    /// top app bar the frame draws over it (`docs/LAYOUT.md` §3).
    pub head: Option<(String, Option<(String, A)>)>,
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
    /// The hits taken this frame whose box ran past the left or right
    /// side of their clip, each with its box uncut: a control a person
    /// sees cut off.
    pub cut: Vec<(Rect, A)>,
    /// Hits are clipped to this rather than to the drawing's clip while
    /// a card's body is revealed: what is not yet uncovered can still be
    /// pressed where it will be.
    hit_clip: Option<Rect>,
    /// The hit being taken draws its own hover look.
    quiet_hover: bool,
    /// The hit being taken shows what it holds as drawn while pressed: a
    /// held secret ([`Ui::hit_held`]).
    held: bool,
    /// The hit being taken draws its own focus ring too
    /// ([`Ui::hit_drawn`]).
    own_ring: bool,
    /// The hit being taken is the press beside a sheet: no target and
    /// no place in the keyboard's order.
    around: bool,
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
    /// The field typing goes to, as drawn this frame: its box, the line
    /// typed in it and its caret, with its label where
    /// [`Ui::field_label`] drew one just above it; in pixels down the
    /// window. What a page scrolls into view above the keyboard.
    pub focus_box: Option<Rect>,
    /// The action that field is pressed with, where it drew a line of
    /// typing ([`Ui::typed`]): tells one focused field from another.
    pub focus_action: Option<A>,
    /// The last label [`Ui::field_label`] drew: its top and the foot of
    /// the gap under it, units down the window.
    label_at: Option<(f32, f32)>,
    /// Actions every button draws dimmed this frame, whatever style the
    /// screen asked for: what the app refuses while it is in this state
    /// (the demo's actions that would make a secret, `docs/DEMO.md` §7).
    pub dimmed: Vec<A>,
    /// Where each press drawn this frame takes a press, as large as the
    /// touch floor where its control is drawn smaller, in the order
    /// drawn: also the order Tab moves the keyboard through
    /// ([`Reach`]). Private, as `hits` is.
    reach: Vec<Reach<A>>,
    /// How many filled (primary) buttons this frame has drawn, for a
    /// test of one primary action a screen.
    pub primaries: usize,
    /// The control the keyboard is on: ringed, and Enter or Space
    /// presses it.
    pub focused: Option<A>,
    /// The tooltips' anchors drawn and the one shown ([`tooltip`]).
    pub tips: tooltip::TipFrame<A>,
    /// What the page or sheet drawn so far hands to the frame's "Practice
    /// only" card ([`demo`]), in the order offered.
    pub demo: Vec<demo::DemoItem<A>>,
}

/// The fill behind selected text: secondary-container, the selected
/// colour (`docs/THEMES.md` §7).
pub const SELECTION_FILL: Role = Role::SecondaryContainer;
/// Text on [`SELECTION_FILL`] where it is not drawn in on-surface:
/// on-secondary-container.
pub const SELECTION_INK: Role = Role::OnSecondaryContainer;

/// The smallest rectangle holding both.
fn union(a: Rect, b: Rect) -> Rect {
    let (x, y) = (a.x.min(b.x), a.y.min(b.y));
    Rect::new(
        x,
        y,
        a.right().max(b.right()) - x,
        a.bottom().max(b.bottom()) - y,
    )
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
            text: 1.0,
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
            dice_shown: 1.0,
            press_shown: 0.0,
            head: None,
            about_open: None,
            pinning: false,
            pin: None,
            bar_tools_x: None,
            sheet_pinning: false,
            sheet_pin: None,
            fab_at: None,
            disclosure: None,
            column: None,
            follow_to: None,
            compact: false,
            chip_at: None,
            cut: Vec::new(),
            hit_clip: None,
            quiet_hover: false,
            held: false,
            own_ring: false,
            around: false,
            outside: None,
            caret_on: true,
            caret_drawn: false,
            caret_foot: None,
            focus_box: None,
            focus_action: None,
            label_at: None,
            dimmed: Vec::new(),
            reach: Vec::new(),
            primaries: 0,
            focused: None,
            tips: tooltip::TipFrame::default(),
            demo: Vec::new(),
        }
    }

    /// `c` in this frame's theme.
    pub fn col(&self, c: impl Into<Paint>) -> Color {
        let c: Paint = c.into();
        self.theme.color(c)
    }

    /// Fills the whole frame with `c`.
    pub fn clear(&mut self, c: impl Into<Paint>) {
        let c: Paint = c.into();
        let c = self.col(c);
        self.c.clear(c);
    }

    /// Says that this frame scrolls the region seen through `view`
    /// (pixels), whose content scrolls as far as `max` units. Called once
    /// the region's content is drawn: a stretch past an end
    /// (`docs/MOTION.md` §3.3) moves what was drawn in it, and what is
    /// pressable there, and the page shows in the gap.
    pub fn report_scroll(&mut self, view: Rect, max: f32) {
        self.report_scroll_on(view, max, Role::Surface);
    }

    /// [`Ui::report_scroll`] for a region that sits on the container
    /// `ground` rather than on the page (a dialog, a sheet, a menu): its
    /// edges fade into that container, and that container shows in the
    /// gap a stretch past an end opens (`docs/THEMES.md` §6, §7).
    pub fn report_scroll_on(&mut self, view: Rect, max: f32, ground: Role) {
        let offset = self.offset;
        self.report(Slot::Page, (view, ground), max, offset, false);
    }

    /// [`Ui::report_scroll`], for a region that draws its own scrollbar.
    pub fn report_scroll_own_bar(&mut self, view: Rect, max: f32) {
        let offset = self.offset;
        self.report(Slot::Page, (view, Role::Surface), max, offset, true);
    }

    /// [`Ui::report_scroll_own_bar`], for a region beside the page's that
    /// moves by its own offset, `offset` units now.
    pub fn report_scroll_in(&mut self, slot: Slot, view: Rect, max: f32, offset: f32) {
        self.report(slot, (view, Role::Surface), max, offset, true);
    }

    /// [`Ui::report_scroll_in`] for a region that shows the page's own
    /// overlay scrollbar at its right edge while it is the one scrolled.
    pub fn report_scroll_beside(&mut self, slot: Slot, view: Rect, max: f32, offset: f32) {
        self.report_scroll_beside_on(slot, (view, Role::Surface), max, offset);
    }

    /// [`Ui::report_scroll_beside`] for a region that sits on the
    /// container `ground` (Learn's supporting pane): its edges fade into
    /// that container, as [`Ui::report_scroll_on`]'s do.
    pub fn report_scroll_beside_on(
        &mut self,
        slot: Slot,
        (view, ground): (Rect, Role),
        max: f32,
        offset: f32,
    ) {
        self.report(slot, (view, ground), max, offset, false);
    }

    /// [`Ui::report_scroll_beside`] for a region whose content also pans
    /// sideways: `max` is how far it goes across and down, in units, and
    /// `offset` how far it is panned each way. While it is the one
    /// scrolled, its overlay scrollbar runs along its foot too, as
    /// Material's horizontal scrollbar does.
    pub fn report_scroll_both(
        &mut self,
        slot: Slot,
        view: Rect,
        max: (f32, f32),
        offset: (f32, f32),
    ) {
        self.report(slot, (view, Role::Surface), max.1, offset.1, false);
        if let Some(s) = self.scrolled.iter_mut().find(|s| s.slot == slot) {
            s.max_x = max.0.max(0.0);
        }
        if let Some((alpha, _)) = self.bar
            && slot == self.active
            && self.in_sheet == self.stretch_in_sheet
            && !self.compact
            && max.0 > 0.0
            && alpha > 0
        {
            self.scroll_bar_across(view, max.0, offset.0, alpha);
        }
    }

    /// The overlay scrollbar along a region's foot: a thumb as long as the
    /// share of the content's width in view, clear of the bar at its right
    /// edge, drawn as [`Ui::scroll_bar`] draws its own.
    fn scroll_bar_across(&mut self, view: Rect, max: f32, offset: f32, alpha: u8) {
        let h = bar_thickness(self.f);
        // The vertical bar's geometry along the width, its end kept clear
        // of the bar at the right edge.
        let along = Rect::new(view.y, view.x, view.h, view.w - h);
        let g = BarGeometry::of(along, max, self.f);
        if g.track <= 0 || g.thumb <= 0 {
            return;
        }
        let inset = g.top - view.x;
        let x = g.top + g.at(offset, max);
        let rect = Rect::new(x, view.bottom() - inset - h, g.thumb, h);
        let color = self.bar_color(alpha);
        self.c.fill_rounded_rect(rect, h as f32 / 2.0, color);
    }

    /// The overlay scrollbar's colour at `alpha`: faint, but at full
    /// strength at High contrast, where it is the one sign the page goes
    /// on.
    fn bar_color(&self, alpha: u8) -> Color {
        let most = if self.theme.is_high_contrast() {
            255
        } else {
            150
        };
        self.col(Role::OnSurfaceVariant.with_alpha((u16::from(alpha) * most / 255) as u8))
    }

    /// Keeps the region `slot` seen through `view` (pixels) on the
    /// container `ground`, its content scrolling `max` units and `offset`
    /// of them now; stretches it, fades its edges into `ground` and draws
    /// its scrollbar.
    fn report(
        &mut self,
        slot: Slot,
        (view, ground): (Rect, Role),
        max: f32,
        offset: f32,
        own_bar: bool,
    ) {
        let max = max.max(0.0);
        let region = Scrolled {
            slot,
            view,
            max,
            own_bar,
            max_x: 0.0,
        };
        match self.scrolled.iter_mut().find(|s| s.slot == slot) {
            Some(s) => *s = region,
            None => self.scrolled.push(region),
        }
        if self.in_sheet != self.stretch_in_sheet {
            return;
        }
        if slot == self.active {
            self.stretch_view(view, ground);
        }
        self.edge_fades(view, max, offset, ground);
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
        let at = BarGeometry::of(view, max, f).at(offset, max);
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
        let w = bar_thickness(f);
        let rect = Rect::new(view.right() - inset - w, y, w, h);
        let color = self.bar_color(alpha);
        self.c.fill_rounded_rect(rect, w as f32 / 2.0, color);
    }

    /// The region's content fades into `ground`, the surface it sits on,
    /// at an edge it continues past: under the top once scrolled, above
    /// the bottom while there is more. Each fade comes in over the first
    /// 24 units scrolled. At High contrast a scheme draws none: a fade
    /// lowers the contrast of what is under it, and the scrollbar says
    /// the page goes on.
    fn edge_fades(&mut self, view: Rect, max: f32, offset: f32, ground: Role) {
        if max <= 0.0 || view.h <= 0 || self.theme.is_high_contrast() {
            return;
        }
        // Deeper on a small panel, where it is the sign there is more.
        let depth = if self.compact { 44.0 } else { 24.0 };
        let band = ((depth * self.f).round() as i32).clamp(1, view.h / 4 + 1);
        let page = self.col(ground);
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
    /// first frame, and kept) and dimmed by Material's scrim.
    pub fn backdrop(&mut self) {
        match self.frost.as_deref() {
            Some(page) => self.c.restore(page),
            None => {
                // Reduced eight units to a pixel and blurred a little: as
                // soft as more blur at a fraction of the work.
                let scale = (8.0 * self.f).round().max(4.0) as u32;
                let dim = self.col(Role::Scrim.with_alpha(faraday_draw::tokens::SCRIM_ALPHA));
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
    /// before it, black in every theme. High contrast draws
    /// none: a shadow there would lower the contrast of the text and
    /// edges beside it, and what floats has its outline.
    pub fn shadow(&mut self, x: f32, y: f32, w: f32, h: f32, r: f32, level: u8) {
        if self.theme.is_high_contrast() {
            return;
        }
        self.shadow_faded((x, y, w, h, r), level, 1.0);
    }

    /// [`Ui::shadow`] under something `shown` (0 to 1) of the way faded
    /// in, at that share of its strength, at (x, y), w by h with corners
    /// of `r`.
    pub fn shadow_faded(
        &mut self,
        (x, y, w, h, r): (f32, f32, f32, f32, f32),
        level: u8,
        shown: f32,
    ) {
        if self.theme.is_high_contrast() || shown <= 0.0 {
            return;
        }
        let rect = self.rect(x, y, w, h);
        self.c
            .elevation_faded(rect, r * self.f, level, self.f, shown);
    }

    /// Moves what was drawn in `view` by the stretch, and what is
    /// pressable there with it; `ground`, the surface the region sits on,
    /// shows in the gap.
    fn stretch_view(&mut self, view: Rect, ground: Role) {
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
        let page = self.col(ground);
        self.c.fill_rect(gap, page);
        for (r, _) in self.hits.iter_mut() {
            if r.intersect(&view) == *r {
                *r = Rect::new(r.x, r.y - self.stretch, r.w, r.h).intersect(&view);
            }
        }
        for t in self.reach.iter_mut() {
            if t.drawn.intersect(&view) == t.drawn {
                let (d, g) = (t.drawn, t.target);
                t.drawn = Rect::new(d.x, d.y - self.stretch, d.w, d.h).intersect(&view);
                t.target = Rect::new(g.x, g.y - self.stretch, g.w, g.h).intersect(&view);
            }
        }
    }

    /// The selection behind text `shown` drawn at (x, y) in a field of
    /// height `h`, when the field is focused and all of it is selected:
    /// for a field drawn as one string, whose caret stays at its end. The
    /// text drawn on it is written in [`SELECTION_INK`].
    #[allow(clippy::too_many_arguments)]
    pub fn selection(&mut self, x: f32, y: f32, h: f32, size: f32, w: W, shown: &str, on: bool) {
        if on && self.edit.all(shown.chars().count()) {
            let tw = self.measure(size, w, shown);
            let _ = self.selected_band(x, y, h, tw);
        }
    }

    /// The band behind selected text `tw` wide from x, in a field `h`
    /// tall at y ([`Ui::selection_band`]). Returns where it is, in
    /// pixels.
    fn selected_band(&mut self, x: f32, y: f32, h: f32, tw: f32) -> Rect {
        use faraday_draw::tokens::{SELECTION_BAND, SELECTION_PAD};
        let bh = h * SELECTION_BAND;
        let at = (x - SELECTION_PAD, y + (h - bh) / 2.0);
        self.selection_band(at.0, at.1, tw + 2.0 * SELECTION_PAD, bh);
        self.rect(at.0, at.1, tw + 2.0 * SELECTION_PAD, bh)
    }

    /// The band behind selected text, at (x, y), `w` by `h`: one selection
    /// paint wherever text is selected, [`SELECTION_FILL`], the text on
    /// it in [`SELECTION_INK`], which reads on it at 4.5:1 or more in
    /// every scheme and 7:1 at High contrast.
    pub fn selection_band(&mut self, x: f32, y: f32, w: f32, h: f32) {
        use faraday_draw::tokens::SHAPE_XS;
        self.fill(x, y, w, h, SHAPE_XS, SELECTION_FILL);
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
        color: impl Into<Paint>,
        text: &str,
        room: f32,
        focused: bool,
        action: A,
    ) -> f32 {
        let color: Paint = color.into();
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
        let shown = &text[at[from]..at[to]];
        let band = selected.and_then(|(lo, hi)| {
            let (lo, hi) = (lo.clamp(from, to), hi.clamp(from, to));
            (hi > lo).then(|| self.selected_band(left(lo), y, h, edge[hi] - edge[lo]))
        });
        match band {
            // What is selected is written in the selection's own ink, the
            // rest beside it as asked.
            Some(b) => {
                // The row, with room for a letter's overhang at its ends.
                let row = self.rect(x - h, y, room + 2.0 * h, h);
                let before = Rect::new(row.x, row.y, (b.x - row.x).max(0), row.h);
                let after = Rect::new(b.right(), row.y, (row.right() - b.right()).max(0), row.h);
                for (clip, ink) in [
                    (before, color),
                    (after, color),
                    (Rect::new(b.x, row.y, b.w, row.h), SELECTION_INK.paint()),
                ] {
                    if clip.w > 0 {
                        self.c.push_clip(clip);
                        self.text_mid(x, y, h, size, w, ink, shown);
                        self.c.pop_clip();
                    }
                }
            }
            None => {
                self.text_mid(x, y, h, size, w, color, shown);
            }
        }
        if focused {
            self.note_focus(x, y, room, h);
            self.focus_action = Some(action);
        }
        if focused && selected.is_none() {
            use faraday_draw::tokens::{CARET_MIN, CARET_OVER};
            let ch = (size + CARET_OVER).max(CARET_MIN);
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

    /// A filled circle; at High contrast a tint is its ring
    /// ([`Ui::fill`]).
    pub fn dot(&mut self, cx: f32, cy: f32, r: f32, color: impl Into<Paint>) {
        let color: Paint = color.into();
        if let Some(edge) = self.tint_edge(color) {
            let rect = self.rect(cx - r, cy - r, 2.0 * r, 2.0 * r);
            let width = self.f.max(1.0);
            self.c.stroke_rounded_rect(rect, r * self.f, width, edge);
            return;
        }
        let color = self.col(self.solid(color));
        self.c.fill_circle(
            (cx + self.ox) * self.f,
            (cy + self.oy) * self.f,
            r * self.f,
            color,
        );
    }

    /// A filled rectangle with rounded corners. At High contrast
    /// a see-through theme colour is drawn at full strength, or, as a
    /// tint (under [`faraday_draw::tokens::HIGH_CONTRAST_TINT_MAX`]), as
    /// its edge: no see-through layer lowers the contrast of what is on
    /// it. A state layer, a control not available now or something
    /// fading is a [`Ui::wash`].
    pub fn fill(&mut self, x: f32, y: f32, w: f32, h: f32, r: f32, color: impl Into<Paint>) {
        let color: Paint = color.into();
        if let Some(edge) = self.tint_edge(color) {
            let rect = self.rect(x, y, w, h);
            let width = self.f.max(1.0);
            self.c.stroke_rounded_rect(rect, r * self.f, width, edge);
            return;
        }
        let color = self.solid(color);
        self.wash(x, y, w, h, r, color);
    }

    /// An outlined rectangle with rounded corners; at High contrast
    /// a see-through colour is drawn at full strength.
    pub fn stroke(&mut self, x: f32, y: f32, w: f32, h: f32, r: f32, color: impl Into<Paint>) {
        let color: Paint = color.into();
        let color = self.col(self.solid(color));
        let rect = self.rect(x, y, w, h);
        let width = self.f.max(1.0);
        self.c.stroke_rounded_rect(rect, r * self.f, width, color);
    }

    /// A card: a surface with a one-unit edge. The page is the surface
    /// too (`docs/DECISIONS.md` F39), so the edge is what sets a card
    /// apart; its fill is surface-container-low, Material's elevated
    /// card drawn flat (`docs/THEMES.md` §7).
    pub fn card(&mut self, x: f32, y: f32, w: f32, h: f32, edge: impl Into<Paint>) {
        use faraday_draw::tokens::SHAPE_M;
        let edge: Paint = edge.into();
        self.fill(x, y, w, h, SHAPE_M, Role::SurfaceContainerLow);
        self.stroke(x, y, w, h, SHAPE_M, edge);
    }

    /// Costs filling its outlines the first time it is drawn at a place
    /// and size, then a blend of what the canvas kept
    /// ([`Canvas::fill_kept`]). Faraday's mark in a box `w` by `h` units
    /// at (x, y): the keyhole shield (`mark`), fitted to the box and
    /// centred, the shield in primary with its band lit half with white
    /// and the keyhole cut through.
    pub fn mark(&mut self, x: f32, y: f32, w: f32, h: f32) {
        let m = mark::outlines();
        let area = self.rect(x, y, w, h);
        let s = (area.w as f32 / mark::WIDTH).min(area.h as f32 / mark::HEIGHT);
        let ox = area.x as f32 + (area.w as f32 - s * mark::WIDTH) / 2.0;
        let oy = area.y as f32 + (area.h as f32 - s * mark::HEIGHT) / 2.0;
        let place = move |o: &[(f32, f32)]| -> Vec<(f32, f32)> {
            o.iter()
                .map(|&(px, py)| (ox + px * s, oy + py * s))
                .collect()
        };
        let stroke = self.col(Role::Primary);
        let lit = stroke.mix(Color::rgb(0xff, 0xff, 0xff), 0.5);
        let key = |shape| ShapeKey::new(shape, ox, oy, s);
        self.c.fill_kept(
            key(SHAPE_MARK_LIT),
            || vec![place(&m.outer), place(&m.hole)],
            lit,
        );
        self.c.fill_kept(
            key(SHAPE_MARK_BODY),
            || vec![place(&m.inner), place(&m.hole)],
            stroke,
        );
    }

    /// Costs as [`Ui::wordmark_named`]. Faraday's name, the wordmark of
    /// `assets/wordmark.txt`, its capitals `cap` units tall with their
    /// top-left at (x, y). Returns its width.
    pub fn wordmark(&mut self, x: f32, y: f32, cap: f32, color: impl Into<Paint>) -> f32 {
        let color: Paint = color.into();
        self.wordmark_named(x, y, cap, color, NameSuffix::None)
    }

    /// Costs filling its outlines the first time it is drawn at a place
    /// and size, then a blend of what the canvas kept
    /// ([`Canvas::fill_kept`]). The wordmark with the word of an app's
    /// name after it
    /// ("Faraday Starter", "Faraday Demo", "Faraday Offline"), as one name
    /// in one lettering.
    /// The suffix is in Archivo Expanded Regular, in the grey of
    /// secondary text (`docs/DECISIONS.md` F16).
    pub fn wordmark_named(
        &mut self,
        x: f32,
        y: f32,
        cap: f32,
        color: impl Into<Paint>,
        suffix: NameSuffix,
    ) -> f32 {
        let color: Paint = color.into();
        let mark = wordmark_outlines();
        let s = cap / WORDMARK_CAP;
        let k = s * self.f;
        let (ox, oy) = ((x + self.ox) * self.f, (y + self.oy) * self.f);
        let place = |os: &[Vec<(f32, f32)>], dx: f32| -> Vec<Vec<(f32, f32)>> {
            os.iter()
                .map(|o| {
                    o.iter()
                        .map(|&(fx, fy)| {
                            (ox + (fx + dx - mark.left) * k, oy + (fy + WORDMARK_CAP) * k)
                        })
                        .collect()
                })
                .collect()
        };
        let fill = self.col(color);
        self.c.fill_kept(
            ShapeKey::new(SHAPE_WORDMARK, ox, oy, k),
            || place(&mark.outlines, 0.0),
            fill,
        );
        let mut right = mark.right;
        if let Some(word) = suffix_word(suffix) {
            let start = mark.right + SUFFIX_GAP;
            let grey = self.col(Role::OnSurfaceVariant);
            self.c.fill_kept(
                ShapeKey::new(SHAPE_SUFFIX + suffix as u32, ox, oy, k),
                || place(&word.outlines, start),
                grey,
            );
            right = start + word.right;
        }
        let w = (right - mark.left) * s;
        self.note_name(x, y, w, cap);
        w
    }

    /// Notes the name drawn in the box `w` by `h` units at (x, y) as one
    /// thing read, while the canvas records what it draws.
    fn note_name(&mut self, x: f32, y: f32, w: f32, h: f32) {
        let r = self.rect(x, y, w, h);
        self.c.note(faraday_draw::canvas::InkKind::Wordmark, r);
    }

    /// Costs as [`Ui::wordmark_named`], for each of its two lines. The
    /// name with the word of the app under FARADAY in place of after
    /// it, for a bar too narrow for the two side by side: FARADAY's
    /// capitals `cap` units tall with their top-left at (x, y), the word's
    /// `WORDMARK_STACK_WORD` of that, `WORDMARK_STACK_GAP` of it under
    /// FARADAY, both starting at the same left edge. Returns its width and
    /// height.
    pub fn wordmark_stacked(
        &mut self,
        x: f32,
        y: f32,
        cap: f32,
        color: impl Into<Paint>,
        suffix: NameSuffix,
    ) -> (f32, f32) {
        let color: Paint = color.into();
        let mark_w = self.wordmark_named(x, y, cap, color, NameSuffix::None);
        let Some(word) = suffix_word(suffix) else {
            return (mark_w, cap);
        };
        let wc = cap * faraday_draw::tokens::WORDMARK_STACK_WORD;
        let wy = y + cap * (1.0 + faraday_draw::tokens::WORDMARK_STACK_GAP);
        let s = wc / WORDMARK_CAP;
        let k = s * self.f;
        let left = word
            .outlines
            .iter()
            .flatten()
            .map(|p| p.0)
            .fold(f32::MAX, f32::min);
        let (ox, oy) = ((x + self.ox) * self.f, (wy + self.oy) * self.f);
        let outlines = || {
            word.outlines
                .iter()
                .map(|o| {
                    o.iter()
                        .map(|&(fx, fy)| (ox + (fx - left) * k, oy + (fy + WORDMARK_CAP) * k))
                        .collect()
                })
                .collect()
        };
        let grey = self.col(Role::OnSurfaceVariant);
        self.c.fill_kept(
            ShapeKey::new(SHAPE_SUFFIX_UNDER + suffix as u32, ox, oy, k),
            outlines,
            grey,
        );
        let word_w = (word.right - left) * s;
        self.note_name(x, wy, word_w, wc);
        (mark_w.max(word_w), wy + wc - y)
    }

    /// Costs measuring the name, and drawing it as
    /// [`Ui::wordmark_named`] does. An app's name
    /// starting at `x` and ending within `room` units of it, its capitals
    /// `cap` tall on the line whose top is `y`: beside its word where the
    /// two fit; else, for an app with a word, the word under FARADAY
    /// ([`Ui::wordmark_stacked`]), centred on the same line and made
    /// smaller where FARADAY needs it, down to
    /// [`faraday_draw::tokens::WORDMARK_CAP_MIN`]; else nothing. Returns
    /// whether it was drawn.
    pub fn wordmark_fitted(
        &mut self,
        (x, y, room): (f32, f32, f32),
        cap: f32,
        color: impl Into<Paint>,
        suffix: NameSuffix,
    ) -> bool {
        let color: Paint = color.into();
        if wordmark_width(cap, suffix) <= room {
            self.wordmark_named(x, y, cap, color, suffix);
            return true;
        }
        if suffix == NameSuffix::None {
            return false;
        }
        let small = cap * (room / wordmark_width(cap, NameSuffix::None)).min(1.0);
        if small < faraday_draw::tokens::WORDMARK_CAP_MIN {
            return false;
        }
        let h = small
            * (1.0
                + faraday_draw::tokens::WORDMARK_STACK_GAP
                + faraday_draw::tokens::WORDMARK_STACK_WORD);
        self.wordmark_stacked(x, y + (cap - h) / 2.0, small, color, suffix);
        true
    }

    /// The caret of the field typing goes to, `h` units tall at (x, y),
    /// in the off half of its blink not drawn.
    pub fn caret(&mut self, x: f32, y: f32, h: f32) {
        self.caret_drawn = true;
        self.caret_foot = Some(y + h + self.oy);
        self.note_focus(x, y, 2.0, h);
        if self.caret_on {
            self.fill(x, y, 2.0, h, 1.0, Role::Primary);
        }
    }

    /// Part of the field typing goes to, drawn at (x, y), `w` by `h`
    /// units: its box, its line or its caret, joined into
    /// [`Ui::focus_box`] with the label drawn just above it.
    pub fn note_focus(&mut self, x: f32, y: f32, w: f32, h: f32) {
        let top = match self.label_at {
            Some((label, foot)) if (foot - (y + self.oy)).abs() < 0.5 => label - self.oy,
            _ => y,
        };
        let part = self.rect(x, top, w, y + h - top);
        self.focus_box = Some(self.focus_box.map_or(part, |b| union(b, part)));
    }

    /// The caret as a character after the text typed, for a field drawn
    /// as one string: a bar, or nothing in the off half of its blink.
    pub fn caret_char(&mut self) -> &'static str {
        self.caret_drawn = true;
        if self.caret_on { "|" } else { "" }
    }

    /// A horizontal rule.
    pub fn rule(&mut self, x: f32, y: f32, w: f32, color: impl Into<Paint>) {
        let color: Paint = color.into();
        let color = self.col(self.solid(color));
        let r = self.rect(x, y, w, 1.0);
        let rect = Rect::new(r.x, r.y, r.w, 1.max(self.px(1.0)));
        self.c.fill_rect(rect, color);
    }

    /// A vertical rule: one unit wide, at least a pixel.
    pub fn vrule(&mut self, x: f32, y: f32, h: f32, color: impl Into<Paint>) {
        let color: Paint = color.into();
        let color = self.col(self.solid(color));
        let r = self.rect(x, y, 1.0, h);
        let rect = Rect::new(r.x, r.y, 1.max(self.px(1.0)), r.h);
        self.c.fill_rect(rect, color);
    }

    fn font(&self, size: f32, w: W) -> Font {
        // Nothing is drawn under Material's smallest size.
        let size = match w {
            W::M => size.max(faraday_draw::tokens::TEXT_MIN / 0.94),
            _ => size.max(faraday_draw::tokens::TEXT_MIN),
        };
        let family = match w {
            W::R => Family::Regular,
            W::S => Family::SemiBold,
            W::M => Family::Mono,
        };
        // The text size applies here, once, so every measure, line, wrap
        // and draw built on it sees it (`docs/TEXT-SIZE.md` §3).
        let size = size * self.text;
        let size = if w == W::M {
            size / self.tr * 0.94
        } else {
            size / self.tr
        };
        Font::new(family, size)
    }

    /// Draws text `t` times its size from here on: the person's text size
    /// (`docs/TEXT-SIZE.md` §2), set once a frame before anything is
    /// drawn.
    pub fn set_text_size(&mut self, t: crate::TextSize) {
        self.text = t.factor();
    }

    /// What every text size and line height is multiplied by now: the
    /// person's text size, or 1 inside [`Ui::fixed_text`].
    pub fn text_factor(&self) -> f32 {
        self.text
    }

    /// `draw` with text at its Standard size, whatever the text size: what
    /// keeps its size as the text grows (a keyboard's keys, a pad). The
    /// factor is put back after.
    pub fn fixed_text<R>(&mut self, draw: impl FnOnce(&mut Self) -> R) -> R {
        let text = std::mem::replace(&mut self.text, 1.0);
        let r = draw(self);
        self.text = text;
        r
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

    /// `c` as text and icons are drawn in this frame's theme: as
    /// [`Ui::col`], since the scheme's primary reads as text on every
    /// surface (`docs/THEMES.md` §6).
    pub fn ink(&self, c: impl Into<Paint>) -> Color {
        self.col(c)
    }

    /// `s` with its top-left at (x, y). Returns its width.
    pub fn text(
        &mut self,
        x: f32,
        y: f32,
        size: f32,
        w: W,
        color: impl Into<Paint>,
        s: &str,
    ) -> f32 {
        let color: Paint = color.into();
        let color = self.ink(color);
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
        color: impl Into<Paint>,
        s: &str,
    ) -> f32 {
        let color: Paint = color.into();
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
        color: impl Into<Paint>,
        s: &str,
    ) {
        let color: Paint = color.into();
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
        color: impl Into<Paint>,
        s: &str,
    ) -> f32 {
        let color: Paint = color.into();
        let color = self.ink(color);
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
        colors: (Paint, Paint),
        s: &str,
        mark: std::ops::Range<usize>,
    ) -> f32 {
        let (color, mark_color) = (self.ink(colors.0), self.ink(colors.1));
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
        color: impl Into<Paint>,
    ) {
        let color: Paint = color.into();
        let color = self.ink(color);
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
    fn stick_glyph(&mut self, cx: f32, cy: f32, size: f32, color: impl Into<Paint>) {
        let color = self.col(color);
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
    /// It shows its state as Material's state layer over what was drawn
    /// there, hovered, with the keyboard on it (and the focus ring) and
    /// pressed, in small corners ([`Ui::press`] takes the control's
    /// own), unless it draws its own (a button) or covers so much of the
    /// screen that a layer would light the page. It takes a press at
    /// least [`faraday_draw::tokens::TOUCH`] square ([`Reach`]).
    pub(crate) fn hit(&mut self, x: f32, y: f32, w: f32, h: f32, action: A) {
        // A dimmed action takes no press, wherever it is drawn.
        if self.dimmed.contains(&action) {
            return;
        }
        let clip = self.hit_clip.unwrap_or_else(|| self.c.clip());
        let whole = self.rect(x, y, w, h);
        let rect = whole.intersect(&clip);
        // A clip with no room is a pass that measures and draws nothing.
        let level = !clip.is_empty() && whole.y < clip.bottom() && whole.bottom() > clip.y;
        if !self.around && level && (whole.x < clip.x || whole.right() > clip.right()) {
            self.cut.push((whole, action));
        }
        if rect.w > 0 && rect.h > 0 {
            let screen = i64::from(self.c.width()) * i64::from(self.c.height());
            let small = i64::from(rect.w) * i64::from(rect.h) * 4 < screen;
            use faraday_draw::tokens::SHAPE_S;
            // Pressed, a held secret shows what it holds as drawn, its
            // code readable under the finger ([`Ui::hit_held`]).
            if small && !self.quiet_hover && !(self.held && self.is_pressed(action)) {
                self.state_layer((x, y, w, h, SHAPE_S), Role::OnSurface.paint(), action);
            }
            // A region with its own hover look is still ringed while the
            // keyboard is on it, unless it rings itself.
            if small && !self.own_ring {
                self.focus_ring((x, y, w, h, SHAPE_S), action);
            }
            self.hits.push((rect, action));
            if !self.around {
                self.reach_for(rect, clip, rect != whole, action);
            }
        }
    }

    /// [`Ui::hit`] for a held secret (a code shown while a finger is on
    /// it): no pressed layer over it, so what it shows reads as drawn;
    /// hovered and focused as [`Ui::hit`].
    #[cfg(test)]
    pub(crate) fn hit_held(&mut self, x: f32, y: f32, w: f32, h: f32, action: A) {
        self.held = true;
        self.hit(x, y, w, h, action);
        self.held = false;
    }

    /// [`Ui::hit`] for a region that draws its own hover look: no state
    /// layer; the focus ring is still drawn round it.
    pub(crate) fn hit_quiet(&mut self, x: f32, y: f32, w: f32, h: f32, action: A) {
        self.quiet_hover = true;
        self.hit(x, y, w, h, action);
        self.quiet_hover = false;
    }

    /// [`Ui::hit`] for a control that draws its own state layer and focus
    /// ring in its own shape (a button, a segment, the floating action).
    pub(crate) fn hit_drawn(&mut self, x: f32, y: f32, w: f32, h: f32, action: A) {
        self.own_ring = true;
        self.hit_quiet(x, y, w, h, action);
        self.own_ring = false;
    }

    /// Presses everywhere outside the box at (x, y), w by h, take
    /// `action`: a sheet that a press beside it closes. No hover look.
    pub(crate) fn hit_around(&mut self, x: f32, y: f32, w: f32, h: f32, action: A) {
        const FAR: f32 = 10_000.0;
        self.quiet_hover = true;
        self.around = true;
        self.hit(x - FAR, y - FAR, w + 2.0 * FAR, FAR, action);
        self.hit(x - FAR, y + h, w + 2.0 * FAR, FAR, action);
        self.hit(x - FAR, y, FAR, h, action);
        self.hit(x + w, y, FAR, h, action);
        self.around = false;
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

    /// A button (`docs/DESIGN.md` §4.13): Material's filled, tonal,
    /// outlined or text button by `style`, round at its ends, its state
    /// layer in its label's colour and the focus ring round it; a kind
    /// not available now keeps its shape, faint, and takes no press. A
    /// chosen option carries a tick before its label. Returns its width.
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
        // Taller where larger text needs it, about the same middle
        // (`docs/TEXT-SIZE.md` §5).
        let grown = self.button_height(h);
        let y = y - (grown - h) / 2.0;
        let r = faraday_draw::tokens::shape_full(grown);
        let size = faraday_draw::tokens::LABEL_L.size;
        self.button_drawn((x, y, w, grown, r), size, label, style, action, None)
    }

    /// [`Ui::button`] with corners of `r` rather than round ends: a key
    /// of the on-screen keyboard, its label the key's
    /// ([`faraday_draw::tokens::key_label`]).
    pub fn button_in(
        &mut self,
        at: (f32, f32, Option<f32>, f32, f32),
        label: &str,
        style: Style,
        action: A,
    ) -> f32 {
        // A key keeps its size whatever the text size: the keyboards
        // and pads are set by what must fit on the panel.
        let size = faraday_draw::tokens::key_label(at.3);
        self.fixed_text(|ui| ui.button_drawn(at, size, label, style, action, None))
    }

    /// [`Ui::button_in`] for a key of the on-screen keyboard: it shows
    /// hover and focus, and no pressed layer, so a watcher reads nothing
    /// from a press ([`Unpressed::Keyboard`]).
    pub fn key_in(
        &mut self,
        at: (f32, f32, Option<f32>, f32, f32),
        label: &str,
        style: Style,
        action: A,
    ) -> f32 {
        let size = faraday_draw::tokens::key_label(at.3);
        self.fixed_text(|ui| {
            ui.button_drawn(at, size, label, style, action, Some(Unpressed::Keyboard))
        })
    }

    /// A button or a key with its label at `size` in the medium face:
    /// [`Ui::button`] and [`Ui::button_in`].
    fn button_drawn(
        &mut self,
        (x, y, w, h, r): (f32, f32, Option<f32>, f32, f32),
        size: f32,
        label: &str,
        style: Style,
        action: A,
        unpressed: Option<Unpressed>,
    ) -> f32 {
        use faraday_draw::tokens::{BUTTON_PAD, ICON_GAP, ICON_GLYPH_BUTTON};
        let weight = W::S;
        let style = if self.dimmed.contains(&action) {
            style.off()
        } else {
            style
        };
        let tick = if style == Style::Selected {
            ICON_GLYPH_BUTTON + ICON_GAP
        } else {
            0.0
        };
        let tw = self.measure(size, weight, label);
        let width = w.unwrap_or(tw + 2.0 * BUTTON_PAD + tick);
        let faint = Role::OnSurface.at(faraday_draw::tokens::DISABLED_CONTAINER);
        match style {
            Style::Primary => {
                self.primaries += 1;
                self.fill(x, y, width, h, r, Role::Primary);
            }
            Style::Tonal | Style::Selected => {
                self.fill(x, y, width, h, r, Role::SecondaryContainer);
                // High contrast edges every control at 3:1.
                if self.theme.is_high_contrast() {
                    self.stroke(x, y, width, h, r, Role::Outline);
                }
            }
            Style::Secondary => self.stroke(x, y, width, h, r, Role::Outline),
            Style::Disabled => {
                // Faint in every theme: it is not available now.
                let (rect, edge) = (self.rect(x, y, width, h), self.col(faint));
                let line = self.f.max(1.0);
                self.c.stroke_rounded_rect(rect, r * self.f, line, edge);
            }
            Style::PrimaryOff | Style::TonalOff => self.wash(x, y, width, h, r, faint),
            Style::Ghost | Style::GhostOff => {}
        }
        let fg = style.label();
        if !style.is_off() {
            let shape = Shape::new(x, y, width, h, r);
            match unpressed {
                Some(why) => self.press_unpressed(shape, fg, why, action),
                None => self.press(shape, fg, action),
            }
        }
        // At a larger text size a label wider than its button fits down
        // to Standard, and is shortened there only where it still does
        // not fit (`docs/TEXT-SIZE.md` §3).
        let room = width - tick - 2.0 * BUTTON_PAD;
        if self.text > 1.0 && tw > room {
            let size = size * self.fit_factor_of(tw, room);
            return self.fixed_text(|ui| {
                let shown = ui.fit(size, weight, label, room);
                let tw = ui.measure(size, weight, &shown);
                let lx = x + (width - tw - tick) / 2.0;
                if tick > 0.0 {
                    let iy = y + (h - ICON_GLYPH_BUTTON) / 2.0;
                    ui.icon(lx, iy, ICON_GLYPH_BUTTON, Icon::Done, ICON_GLYPH_BUTTON, fg);
                }
                ui.text_mid(lx + tick, y, h, size, weight, fg, &shown);
                width
            });
        }
        let lx = x + (width - tw - tick) / 2.0;
        if tick > 0.0 {
            let iy = y + (h - ICON_GLYPH_BUTTON) / 2.0;
            self.icon(lx, iy, ICON_GLYPH_BUTTON, Icon::Done, ICON_GLYPH_BUTTON, fg);
        }
        self.text_mid(lx + tick, y, h, size, weight, fg, label);
        width
    }

    /// A chip inside the empty entry field at (`x`, `y`, `w`, `h`), at its
    /// text inset, secondary-styled and vertically centred
    /// (`docs/MODES.md` §6.1). Its hit target is the chip, at the touch
    /// floor's height on a small panel and the field's on a wide one.
    /// Returns the chip's width; the caller passes `label`, the shorter
    /// one where the field is narrower than the chip.
    pub fn field_action(&mut self, x: f32, y: f32, w: f32, h: f32, label: &str, action: A) -> f32 {
        use faraday_draw::tokens::SHAPE_S;
        use faraday_draw::tokens::{
            FIELD_ACTION_HEIGHT as CH, FIELD_ACTION_INSET as IN, FIELD_ACTION_TEXT as TX, TOUCH,
        };
        // A one-line slot: the field's inset either side of the chip and
        // the chip's own inside it; larger text fits down to Standard
        // (`docs/TEXT-SIZE.md` §3).
        let room = (w - 2.0 * IN) - 2.0 * IN;
        let tw = self.text_fit_width(TX, label, room);
        let width = (tw + 2.0 * IN).min(w - 2.0 * IN);
        let (cx, cy) = (x + IN, y + (h - CH) / 2.0);
        let hit_h = if self.compact {
            TOUCH.max(CH)
        } else {
            h.max(CH)
        };
        let target = (cx, y + (h - hit_h) / 2.0, width, hit_h);
        let shape = Shape::new(cx, cy, width, CH, SHAPE_S);
        self.press_to(shape, target, Role::Primary.paint(), action);
        self.stroke(cx, cy, width, CH, SHAPE_S, Role::Outline);
        let band = (cx + (width - tw) / 2.0, cy, CH, room);
        self.text_fit_as(band, TX, Role::Primary, label);
        width
    }

    /// A floating action: a pill over the page in Material's primary FAB
    /// colour style (filled in primary, its icon and its word in
    /// on-primary; F138), its bottom-right corner at (`right`, `bottom`).
    /// Returns its width.
    pub fn fab(&mut self, right: f32, bottom: f32, icon: Icon, label: &str, action: A) -> f32 {
        // Smaller on a small panel, where it floats over more of the page.
        use faraday_draw::tokens::{
            FAB_H, FAB_H_COMPACT, FAB_PAD, FAB_PAD_COMPACT, ICON_GAP, ICON_GLYPH, LABEL_L, SHAPE_L,
        };
        let (h, pad) = if self.compact {
            (FAB_H_COMPACT, FAB_PAD_COMPACT)
        } else {
            (FAB_H, FAB_PAD)
        };
        let lead = ICON_GLYPH + ICON_GAP;
        let width = self.measure_as(LABEL_L, label) + 2.0 * pad + lead;
        let (x, y) = (right - width, bottom - h);
        self.fab_at = Some((x + self.ox, y + self.oy, width, h));
        let level = if self.is_hovered(action) {
            faraday_draw::tokens::ELEVATION_RAISED
        } else {
            faraday_draw::tokens::ELEVATION_FAB
        };
        self.shadow(x, y, width, h, SHAPE_L, level);
        self.fill(x, y, width, h, SHAPE_L, Role::Primary);
        self.press(
            Shape::new(x, y, width, h, SHAPE_L),
            Role::OnPrimary.paint(),
            action,
        );
        let iy = y + (h - ICON_GLYPH) / 2.0;
        self.icon(x + pad, iy, ICON_GLYPH, icon, ICON_GLYPH, Role::OnPrimary);
        let at = (x + pad + lead, y, h);
        self.text_mid_as(at, LABEL_L, Role::OnPrimary, label);
        width
    }

    /// A status tag (`docs/DESIGN.md` §3 Status): a rounded label with
    /// a dot in `status`'s container colour, its dot and text in its
    /// on-container colour. Returns its width.
    pub fn tag(&mut self, x: f32, y: f32, label: &str, status: Status) -> f32 {
        let (bg, fg) = status.roles();
        self.chip(x, y, label, fg, bg)
    }

    /// A rounded label with a dot, in any two paints: [`Ui::tag`] for a
    /// status. Returns its width.
    pub fn chip(
        &mut self,
        x: f32,
        y: f32,
        label: &str,
        fg: impl Into<Paint>,
        bg: impl Into<Paint>,
    ) -> f32 {
        self.chip_press(x, y, label, (fg.into(), bg.into()), None)
    }

    /// [`Ui::chip`] that a press takes when `action` is given: its state
    /// layer in its content's colour, between its fill and its dot and
    /// text (Material: the layer sits between container and content).
    /// Returns its width.
    pub fn chip_press(
        &mut self,
        x: f32,
        y: f32,
        label: &str,
        (fg, bg): (Paint, Paint),
        action: Option<A>,
    ) -> f32 {
        use faraday_draw::tokens::{BODY_S, TAG_DOT_R, TAG_DOT_TEXT_X, TAG_H as H};
        let width = self.chip_width(label);
        self.fill(x, y, width, H, H / 2.0, bg);
        if let Some(a) = action {
            self.press(Shape::new(x, y, width, H, H / 2.0), fg, a);
        }
        self.dot(x + H / 2.0, y + H / 2.0, TAG_DOT_R, fg);
        self.text_mid_as((x + TAG_DOT_TEXT_X, y, H), BODY_S, fg, label);
        width
    }

    /// How wide [`Ui::chip`] draws `label`.
    pub fn chip_width(&self, label: &str) -> f32 {
        use faraday_draw::tokens::{BODY_S, TAG_DOT_TEXT_X, TAG_PAD};
        self.measure_as(BODY_S, label) + TAG_DOT_TEXT_X + TAG_PAD
    }

    /// A word in a pill that a press opens in its list, where words are
    /// shown as they are made. Returns its width.
    pub fn word_pill(&mut self, x: f32, y: f32, word: &str, action: A) -> f32 {
        use faraday_draw::tokens::{BODY_M, SHAPE_S, TAG_H as h, TAG_PAD};
        let role = BODY_M.mono();
        let width = self.measure_as(role, word) + 2.0 * TAG_PAD;
        self.fill(x, y, width, h, SHAPE_S, Role::SurfaceContainerHighest);
        self.stroke(x, y, width, h, SHAPE_S, Role::Outline);
        self.text_mid_as((x + TAG_PAD, y, h), role, Role::OnSurface, word);
        self.press(
            Shape::new(x, y, width, h, SHAPE_S),
            Role::OnSurface.paint(),
            action,
        );
        width
    }

    /// A pill with a plain label and an edge. Returns its width.
    pub fn pill(&mut self, x: f32, y: f32, h: f32, label: &str) -> f32 {
        self.pill_in(x, y, h, label, Role::OnSurfaceVariant)
    }

    /// [`Ui::pill`] with its label in `color`.
    pub fn pill_in(&mut self, x: f32, y: f32, h: f32, label: &str, color: impl Into<Paint>) -> f32 {
        let color: Paint = color.into();
        use faraday_draw::tokens::{BODY_S, TAG_PAD};
        let width = self.measure_as(BODY_S, label) + 2.0 * TAG_PAD;
        // An assist chip (Material's): no container, an outline.
        self.stroke(x, y, width, h, h / 2.0, Role::Outline);
        self.text_mid_as((x + TAG_PAD, y, h), BODY_S, color, label);
        width
    }

    /// Threshold pips: `filled` of `n`.
    pub fn pips(&mut self, x: f32, y: f32, n: usize, filled: usize) -> f32 {
        use faraday_draw::tokens::{PIP_GAP, PIP_H, PIP_W};
        let mut cx = x;
        for i in 0..n {
            if i < filled {
                self.fill(cx, y, PIP_W, PIP_H, PIP_H / 2.0, Role::Ok);
            } else {
                self.stroke(cx, y, PIP_W, PIP_H, PIP_H / 2.0, Role::Outline);
            }
            cx += PIP_W + PIP_GAP;
        }
        cx - x - PIP_GAP
    }

    /// A numbered or ticked step badge.
    pub fn badge(&mut self, x: f32, y: f32, label: &str, done: bool, open: bool) {
        let (bg, fg) = if done {
            (Role::OkContainer, Role::OnOkContainer)
        } else if open {
            (Role::Primary, Role::OnPrimary)
        } else {
            (Role::SurfaceContainerHighest, Role::OnSurfaceVariant)
        };
        use faraday_draw::tokens::{LABEL_M, STEP_BADGE as D, STEP_BADGE_TICK};
        let r = D / 2.0;
        self.dot(x + r, y + r, r, bg);
        if done {
            self.icon(x, y, D, Icon::Done, STEP_BADGE_TICK, fg);
        } else {
            let tw = self.measure_as(LABEL_M, label);
            self.text_mid_as((x + r - tw / 2.0, y, D), LABEL_M, fg, label);
        }
    }

    /// A checkbox. One ticked and not enabled shows its tick dimmed: it is
    /// done, and a press does not undo it.
    pub fn checkbox(&mut self, x: f32, y: f32, on: bool, enabled: bool) {
        use faraday_draw::tokens::{ICON_GLYPH_BUTTON as SIDE, SHAPE_CHECKBOX as R};
        if on {
            // Not enabled, it is done and a press does not undo it:
            // on-surface at the disabled opacity, its tick in surface
            // (Material's disabled checkbox).
            let (fill, tick) = if enabled {
                (Role::Primary.paint(), Role::OnPrimary.paint())
            } else {
                (
                    Role::OnSurface.at(faraday_draw::tokens::DISABLED_CONTENT),
                    Role::Surface.paint(),
                )
            };
            self.wash(x, y, SIDE, SIDE, R, fill);
            self.icon(
                x,
                y,
                SIDE,
                Icon::Done,
                faraday_draw::tokens::CHECK_TICK,
                tick,
            );
        } else {
            let edge = self.col(if enabled {
                Role::OnSurfaceVariant.paint()
            } else {
                Role::OnSurface.at(faraday_draw::tokens::DISABLED_CONTENT)
            });
            let rect = self.rect(x, y, SIDE, SIDE);
            self.c
                .stroke_rounded_rect(rect, R * self.f, 2.0 * self.f, edge);
        }
    }
}

/// What a status tag says (`docs/DESIGN.md` §3 Status,
/// `docs/THEMES.md` §6): done, needs attention, refused, or neither.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    /// Done, verified, can sign: ok-container.
    Ok,
    /// Waiting, unsaved, needs attention: warning-container.
    Warning,
    /// Refused: error-container.
    Error,
    /// Neither: surface-container-highest.
    Neutral,
}

impl Status {
    /// Its container and the colour on it.
    pub const fn roles(self) -> (Role, Role) {
        match self {
            Status::Ok => (Role::OkContainer, Role::OnOkContainer),
            Status::Warning => (Role::WarningContainer, Role::OnWarningContainer),
            Status::Error => (Role::ErrorContainer, Role::OnErrorContainer),
            Status::Neutral => (Role::SurfaceContainerHighest, Role::OnSurfaceVariant),
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

#[cfg(test)]
mod read_tests;

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

    /// A chip tinted in a tone and a translucent edge, as `theme` draws
    /// them on its page: (inside the chip, on its edge, the page, on the
    /// translucent edge).
    fn tinted(theme: Theme) -> [Color; 4] {
        let mut canvas = canvas(160);
        let mut hits = Vec::new();
        let page;
        {
            let mut ui: Ui<u8> = Ui::new(&mut canvas, 1.0, &mut hits, None);
            ui.theme = theme;
            page = ui.col(pal::Role::Surface);
            ui.clear(pal::Role::Surface);
            ui.fill(100.0, 100.0, 120.0, 40.0, 0.0, pal::Role::Ok.with_alpha(30));
            ui.stroke(
                100.0,
                200.0,
                120.0,
                40.0,
                0.0,
                pal::Role::Primary.with_alpha(110),
            );
        }
        let f = canvas.frame();
        let at = |x: usize, y: usize| {
            let i = (y * usize::from(f.width) + x) * 4;
            Color::rgb(f.rgba[i], f.rgba[i + 1], f.rgba[i + 2])
        };
        [at(160, 120), at(160, 100), page, at(160, 200)]
    }

    /// At High contrast nothing see-through lowers the contrast of what
    /// is on it: a tint is drawn as its edge, the page left under what it
    /// holds, and a translucent edge at full strength. Elsewhere a tint
    /// is a tint.
    #[test]
    fn high_contrast_draws_a_tint_as_its_edge_and_edges_at_full_strength() {
        for theme in Theme::all().filter(|t| t.is_high_contrast()) {
            let [inside, edge, page, line] = tinted(theme);
            let rgb = |c: Color| (c.r, c.g, c.b);
            assert_eq!(rgb(inside), rgb(page), "{theme:?}");
            assert!(contrast(edge, page) >= 3.0, "{theme:?} tint's edge");
            assert!(contrast(line, page) >= 3.0, "{theme:?} edge");
        }
        let [inside, _, page, _] = tinted(Theme::default());
        assert_ne!((inside.r, inside.g, inside.b), (page.r, page.g, page.b));
    }

    /// A canvas of 640 × 480 pixels at `dpi`.
    fn canvas(dpi: u16) -> Canvas {
        Canvas::new(&DisplayInfo {
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
        })
    }

    #[test]
    fn a_kept_theme_mode_and_contrast_read_back_as_written() {
        for theme in Theme::all() {
            assert_eq!(Theme::from_row(theme.row()), Some(theme));
            assert_eq!(ThemeName::from_id(theme.name.id()), Some(theme.name));
            assert_eq!(Mode::from_id(theme.mode.id()), Some(theme.mode));
            assert_eq!(Contrast::from_id(theme.contrast.id()), Some(theme.contrast));
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
