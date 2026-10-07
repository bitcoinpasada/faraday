//! The content-kind gallery: an [`App`] with one page per group of
//! `docs/DESIGN.md` §4, for review through the snapshot and desktop
//! shells. Behind the `gallery` feature; never part of a product build.
//!
//! §8 step 2 asks for "one gallery page per reusable screen and per
//! content kind, rendered at the four sizes". This is the content-kind
//! half: fourteen pages, one per §4 group, each showing every kind in
//! that group in every state that has a rendering. A reviewer reads a
//! page against the §4 table with the same number.
//!
//! Every page has a header with the page name and a "Next ▸" button whose
//! id is [`Gallery::NEXT`]; [`Gallery::next_button_center`] tells a shell
//! where to tap. Placeholder wording lives here, not in the application.

use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use osk_shell_api::{App, Command, DisplayInfo, Event, Frame, TouchPhase};

use crate::canvas::Canvas;
use crate::color::Theme;
use crate::components::{
    self, Entries, Network, OutputBadge, Record, Secret, Tile, Unit, badges, choice, codes, entry,
    identity, nav, paths, progress, records, secrets, strings,
};
use crate::geom::{Edges, Metrics, Point, Rect, Scale, SizeClass};
use crate::layout::{self, Align, Id, Layout, LayoutCtx, Node};
use crate::organisms::{self, SidebarItem};
use crate::screens::{self, Chrome};
use crate::state::{Action, UiState};
use crate::text::Font;
use crate::tokens;
use crate::widgets::keyboard::{self, KeyInput, KeyboardKind};
use crate::widgets::{self, ButtonStyle, Icon, Tone, WarningLevel, Widget};

/// Page file names, in order. The snapshot shell writes one PNG per
/// name; [`Gallery::page_title`] is what the page says on screen.
pub const PAGES: &[&str] = &[
    "4.1-navigation",
    "4.2-choice",
    "4.3-entry",
    "4.4-identity",
    "4.5-long-strings",
    "4.6-paths",
    "4.7-amounts",
    "4.8-badges",
    "4.9-codes",
    "4.10-secrets",
    "4.11-records",
    "4.12-text",
    "4.13-actions",
    "4.14-progress",
    "5-hub",
    "5-menu",
    "5-choice",
    "5-entry",
    "5-pad",
    "5-pad-dice",
    "5-words",
    "5-words-revealed",
    "5-secret",
    "5-secret-masked",
    "5-compare",
    "5-addresses",
    "5-address",
    "5-qr",
    "5-qr-animated",
    "5-record",
    "5-record-output",
    "5-result",
    "5-result-terminal",
    "5-hold",
    "5-scanner",
    "5-document",
];

/// The index of the first reusable-screen page. Everything before it is
/// a content-kind page of §4; everything from it on is one of the
/// sixteen screens of §5, drawn as a flow would draw it.
pub const SCREENS: usize = 14;

/// The heading each page carries, in the order of [`PAGES`].
const TITLES: &[&str] = &[
    "4.1 Navigation",
    "4.2 Choice",
    "4.3 Entry",
    "4.4 Identity",
    "4.5 Long strings",
    "4.6 Paths",
    "4.7 Amounts",
    "4.8 Badges",
    "4.9 Codes",
    "4.10 Secrets",
    "4.11 Records",
    "4.12 Text",
    "4.13 Actions",
    "4.14 Progress",
    "5 Hub",
    "5 Menu",
    "5 Choice",
    "5 Entry",
    "5 Pad",
    "5 Pad · dice",
    "5 Words",
    "5 Words · revealed",
    "5 Secret",
    "5 Secret · masked",
    "5 Compare",
    "5 Addresses",
    "5 Address",
    "5 QR",
    "5 QR · animated",
    "5 Record",
    "5 Record · output",
    "5 Result",
    "5 Result · terminal",
    "5 Hold",
    "5 Scanner",
    "5 Document",
];

/// A small stand-in for the BIP-39 wordlist, enough to exercise the
/// enabled-letter logic.
const WORDS: &[&str] = &[
    "abandon", "ability", "able", "about", "above", "absent", "absorb", "abstract", "absurd",
    "abuse", "access", "accident", "account", "accuse", "achieve", "acid", "acoustic", "acquire",
    "across", "act", "zebra", "zero", "zone", "zoo",
];

/// The twenty-four words a secret panel shows: the longest secret there
/// is, so a page shows what a phone and a desktop window hold in two
/// columns and what a 268 dp panel pages through six at a time (§4.10).
const SEED: &[&str] = &[
    "abandon", "ability", "able", "about", "above", "absent", "absorb", "abstract", "absurd",
    "abuse", "access", "accident", "account", "accuse", "achieve", "acid", "acoustic", "acquire",
    "across", "act", "action", "actor", "actress", "art",
];

/// Where each of [`SEED`]'s words sits in the wordlist, 0-based: what
/// "Numbers" puts beside the words, masked with them (§4.10).
const INDICES: [u16; 24] = [
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 100,
];

const ADDRESS: &str = "bc1pxwww0ct9ue7e8tdnlmug5m2tamfn7q06sahstg39ys4c9f3340qqxrdu9k";
const XPUB: &str = "xpub6CUGRUonZSQ4TWtTMmzXdrXDtypWKiKrhko4egpiMZbpiaQL2jkwSB1icqYh2cfDfVxdx4df189oLKnC5fSwqPfgyP3hooxujYzAu3fDVmz";
const DESCRIPTOR: &str = "wpkh([73c5da0a/84h/0h/0h]xpub6CUGRUonZSQ4TWtTMmzXdrXDtypWKiKrhko4egpiMZbpiaQL2jkwSB1icqYh2cfDfVxdx4df189oLKnC5fSwqPfgyP3hooxujYzAu3fDVmz/<0;1>/*)#kx0d7cvj";
const FINGERPRINT: &str = "73c5da0a";
const SECOND_KEY: &str = "0f1a2b3c";
const PATH: &str = "m/84h/0h/0h";
const XPRV: &str = "xprv9s21ZrQH143K3QTDL4LXw2F7HEK3wJUD2nW2nRk4stbPy6cq3jPPqjiChkVvvNKmPGJxWUtg6LnF5kejMRNNU3TGtRBeJgk33yuGBxrMPHi";
const TIERS: &str = "A tier states what the build was compiled from and what the host \
can hold against it. It is not a claim about the hardware in front of you.";
const EXPLAINER: &str = "The fingerprint is the first four bytes of the hash of the master \
public key. It identifies a key without revealing anything about it.";

// lint-tokens: sample data — the placeholder values the pages show. A
// roll count and a fee rate are content, not design tokens.
/// Dice rolls entered, and the target.
const ROLLS: (f32, f32) = (23.0, 50.0);
/// Coin flips entered, and the target.
const FLIPS: (f32, f32) = (40.0, 128.0);
/// Parts of an animated code shown, and the total.
const CODE_PARTS: (f32, f32) = (2.0, 5.0);
/// Parts of a transaction scanned, and the total.
const SCAN_PARTS: (f32, f32) = (3.0, 8.0);
/// The faces of the dice rolls the entry page shows.
const ROLL_FACES: &[&str] = &["3", "6", "1", "1", "4", "2", "5", "3"];
/// Seconds of the eye's reveal left, for the two rings the page shows.
const EYE_LEFT_S: u32 = 21;
/// A reveal that is nearly over.
const EYE_NEARLY_S: u32 = 4;
/// Words already accepted on the Entry page, and the length of the
/// secret being typed: the panel keeps a row for every word of it.
const ACCEPTED_WORDS: usize = 7;
/// The secret the Entry page is part-way through.
const SECRET_WORDS: usize = 12;
/// A fee rate in sat per virtual byte.
const FEE_RATE: f32 = 7.14;
/// An amount in satoshi, and a fee.
const AMOUNT_SAT: u64 = 60_000;
/// A fee in satoshi.
const FEE_SAT: u64 = 1_000;
/// A large amount, to show the grouping of a long number.
const TOTAL_SAT: u64 = 2_100_000_000_000;
// lint-tokens: end sample data

/// Ids the gallery uses. Tests and shells may rely on `NEXT`.
impl Gallery {
    /// The "Next ▸" button on every page.
    pub const NEXT: Id = Id(1);
    const SCROLL: Id = Id(2);
    const ROW: u32 = 100;
    const TOGGLE: Id = Id(20);
    const HOLD: Id = Id(21);
    const KEYBOARD: Id = Id(22);
    const CANDIDATES: Id = Id(23);
    const CANDIDATES_2: Id = Id(24);
    const PIN: Id = Id(25);
    const DICE: Id = Id(26);
    const COIN: Id = Id(27);
    const REVEAL: Id = Id(28);
    const REVEAL_2: Id = Id(29);
    const REVEAL_3: Id = Id(35);
    const PAGER_PREV: Id = Id(30);
    const PAGER_NEXT: Id = Id(31);
    const CHUNKED: Id = Id(32);
    const CHUNKED_2: Id = Id(33);
    const SETTING: u32 = 40;
    const CHOICE: u32 = 50;
    const TILE: u32 = 60;
    const SIDEBAR: u32 = 70;
    const SIGN_WITH: u32 = 80;
    const PAIR: u32 = 90;
    const EYE: Id = Id(34);
    const VIEWFINDER: Id = Id(36);
    const VIEWFINDER_STATE: Id = Id(37);
    const SCREEN: u32 = 200;
}

/// Rows the six hub tiles take at this class.
fn hub_rows(class: SizeClass) -> usize {
    6usize.div_ceil(tokens::hub_columns(class, false))
}

/// Height in dp the gallery gives the hub grid: what one tile needs for
/// its mark and its word, times the rows, plus the gaps between them.
/// The real hub takes the whole screen; a gallery page shows it beside
/// the other navigation kinds.
fn hub_height(class: SizeClass) -> f32 {
    let rows = hub_rows(class) as f32;
    // The mark, the word, and the line a dimmed tile's reason takes.
    let tile = tokens::tile_icon(class)
        + tokens::GAP
        + tokens::line_box(tokens::BODY)
        + tokens::line_box(tokens::CAPTION)
        + tokens::GAP;
    rows * tile + (rows - 1.0) * tokens::GAP
}

/// The gallery application.
pub struct Gallery {
    canvas: Option<Canvas>,
    ctx: Option<LayoutCtx>,
    state: UiState,
    layout: Option<Layout>,
    theme: Theme,
    commands: VecDeque<Command>,
    page: usize,
    toggle_on: bool,
    setting: u32,
    chosen: u32,
    change: bool,
    hold_done: bool,
    typed: String,
    signing: [bool; 2],
}

impl Default for Gallery {
    fn default() -> Self {
        Gallery::new()
    }
}

impl Gallery {
    /// A gallery on its first page.
    pub fn new() -> Self {
        Gallery {
            canvas: None,
            ctx: None,
            state: UiState::default(),
            layout: None,
            theme: Theme::DARK,
            commands: VecDeque::new(),
            page: 0,
            toggle_on: true,
            setting: 0,
            chosen: 0,
            change: false,
            hold_done: false,
            typed: String::from("ab"),
            signing: [true, false],
        }
    }

    /// Number of pages.
    pub fn page_count() -> usize {
        PAGES.len()
    }

    /// Current page index.
    pub fn page(&self) -> usize {
        self.page
    }

    /// Name of the current page, which is also its render's file name.
    pub fn page_name(&self) -> &'static str {
        PAGES[self.page]
    }

    /// The §4 heading of the current page.
    pub fn page_title(&self) -> &'static str {
        TITLES[self.page]
    }

    /// Jumps to a page directly.
    pub fn set_page(&mut self, page: usize) {
        self.page = page % PAGES.len();
        self.render();
    }

    /// Where a shell taps to reach the next page.
    ///
    /// On a §4 page that is the "Next ▸" button of the header row. A §5
    /// page has no header — it is the screen as a flow draws it — so
    /// there it is the middle of the app bar, between the back chevron
    /// and the trailing slot: [`Gallery::event`] takes a tap there as
    /// "next page". `page N` reaches any page directly either way.
    pub fn next_button_center(&self) -> Option<Point> {
        if self.page >= SCREENS {
            let bounds = self.canvas.as_ref()?.bounds();
            let bar = self.ctx().px(tokens::APP_BAR);
            return Some(Point {
                x: bounds.x + bounds.w / 2,
                y: bounds.y + bar / 2,
            });
        }
        self.layout.as_ref()?.rect(Self::NEXT).map(|r| r.center())
    }

    /// Whether a touch at `(x, y)` is the "next page" tap of a §5 page:
    /// in the app bar's band, clear of the back chevron at one end and
    /// the trailing slot at the other.
    fn is_next_tap(&self, x: i32, y: i32) -> bool {
        if self.page < SCREENS {
            return false;
        }
        let (Some(canvas), Some(ctx)) = (self.canvas.as_ref(), self.ctx) else {
            return false;
        };
        let bounds = canvas.bounds();
        let slot = ctx.px(tokens::TOUCH);
        y >= bounds.y
            && y < bounds.y + ctx.px(tokens::APP_BAR)
            && x >= bounds.x + slot
            && x < bounds.right() - slot
    }

    /// Moves to the next page and forgets the page before it.
    fn next_page(&mut self) {
        self.page = (self.page + 1) % PAGES.len();
        self.hold_done = false;
        self.state = UiState::new(self.ctx.map_or(Scale::IDENTITY, |c| c.scale));
    }

    fn class(&self) -> SizeClass {
        self.ctx.map_or(SizeClass::Small, |c| c.class)
    }

    fn ctx(&self) -> LayoutCtx {
        self.ctx
            .unwrap_or_else(|| LayoutCtx::new(Scale::IDENTITY, SizeClass::Small))
    }

    /// The gallery page's own metrics: the whole window, at its class.
    fn metrics(&self) -> Metrics {
        let ctx = self.ctx();
        let bounds = self
            .canvas
            .as_ref()
            .map_or(Rect::new(0, 0, 268, 358), Canvas::bounds);
        let factor = ctx.scale.factor();
        Metrics::new(
            bounds.w as f32 / factor,
            bounds.h as f32 / factor,
            ctx.class,
        )
        .with_insets(ctx.inset_top_dp, ctx.inset_bottom_dp)
    }

    fn header(&self) -> Node {
        Node::row()
            .padding(Edges::symmetric(tokens::PAD, tokens::GAP_SMALL))
            .gap(tokens::GAP)
            .align(Align::Center)
            .child(
                Node::widget(Widget::text(
                    alloc::format!("{} · {}/{}", self.page_title(), self.page + 1, PAGES.len()),
                    Font::semibold(tokens::CAPTION),
                    Tone::Text,
                ))
                .weight(1.0),
            )
            .child(
                Node::widget(Widget::button(Self::NEXT, "Next ▸", ButtonStyle::Secondary))
                    .height(tokens::CHIP_COMPACT + tokens::GAP),
            )
    }

    fn page_body(&self) -> Node {
        match self.page {
            0 => self.navigation(),
            1 => self.choice(),
            2 => self.entry(),
            3 => self.identity(),
            4 => self.long_strings(),
            5 => self.paths(),
            6 => self.amounts(),
            7 => self.badges(),
            8 => self.codes(),
            9 => self.secrets(),
            10 => self.records(),
            11 => self.text(),
            12 => self.actions(),
            13 => self.progress(),
            _ => self.screen_page(),
        }
    }

    /// A §4 page carries the gallery's own header row; a §5 page is the
    /// screen itself, edge to edge, with the page's name in its app bar
    /// (see [`Gallery::next_button_center`] for how a shell turns the
    /// page there).
    fn build(&self) -> Node {
        if self.page >= SCREENS {
            return self.page_body();
        }
        Node::column()
            .child(self.header())
            .child(Node::widget(Widget::Divider))
            .child(self.page_body().weight(1.0))
    }

    /// A page that scrolls when it outgrows the panel, which is what a
    /// gallery page is allowed to do.
    fn scroll_page(&self, content: Node) -> Node {
        Node::scroll(Self::SCROLL, content.pad(tokens::PAD).gap(tokens::GAP))
    }

    /// Two lists of record rows, one after the other. §4.7's fee is two
    /// rows now, so a page that shows a fee among other facts joins
    /// them rather than writing the rows out twice.
    fn rows(mut first: Vec<Record>, second: Vec<Record>) -> Vec<Record> {
        first.extend(second);
        first
    }

    /// A caption naming the kind under it, so a render can be read
    /// against the §4 table row by row.
    fn kind(&self, name: &str) -> Node {
        Node::widget(Widget::paragraph(
            String::from(name),
            Font::semibold(tokens::CAPTION),
            Tone::Primary,
        ))
    }

    // -----------------------------------------------------------------
    // 4.1 Navigation
    // -----------------------------------------------------------------

    fn navigation(&self) -> Node {
        let class = self.class();
        let tiles: Vec<Tile> = [
            (Icon::Scan, "Scan"),
            (Icon::Keys, "Keys"),
            (Icon::Sign, "Sign"),
            (Icon::Verify, "Verify"),
            (Icon::Explore, "Explore"),
            (Icon::Learn, "Learn"),
        ]
        .iter()
        .enumerate()
        .map(|(i, (icon, label))| Tile {
            id: Id(Self::TILE + i as u32),
            icon: *icon,
            label: String::from(*label),
            enabled: *icon != Icon::Learn,
            badge: (*icon == Icon::Keys).then(|| String::from("2")),
        })
        .collect();
        let sidebar_items = [
            SidebarItem {
                id: Some(Id(Self::SIDEBAR)),
                icon: Icon::Keys,
                label: String::from("Keys"),
                selected: true,
            },
            SidebarItem {
                id: Some(Id(Self::SIDEBAR + 1)),
                icon: Icon::Sign,
                label: String::from("Sign"),
                selected: false,
            },
            SidebarItem {
                id: None,
                icon: Icon::Learn,
                label: String::from("Learn"),
                selected: false,
            },
        ];
        self.scroll_page(
            Node::column()
                .gap(tokens::GAP)
                .child(self.kind("App bar"))
                .child(organisms::app_bar(
                    Some(Id(Self::ROW)),
                    "Key 73c5da0a",
                    Some(organisms::bar_action(Id(Self::ROW + 1), Icon::Eye)),
                ))
                .child(self.kind("Menu row · label above, value below, chevron"))
                .child(nav::menu_row(
                    class,
                    Id(Self::ROW + 2),
                    Some(Icon::Shield),
                    "Backup",
                    Some(String::from("not verified")),
                    Tone::Caution,
                ))
                .child(nav::menu_row(
                    class,
                    Id(Self::ROW + 3),
                    Some(Icon::Sign),
                    "Sign a transaction",
                    None,
                    Tone::Muted,
                ))
                .child(self.kind("Two-line rows · one height per class"))
                .child(choice::value_row(
                    class,
                    Id(Self::ROW + 4),
                    "Script type",
                    "SegWit",
                    Tone::Text,
                ))
                .child(identity::key_row(
                    class,
                    Id(Self::ROW + 5),
                    FINGERPRINT,
                    "mainnet · passphrase",
                ))
                .child(strings::reference_row(
                    class,
                    Id(Self::ROW + 6),
                    "Account key",
                    XPUB,
                ))
                .child(secrets::secret_row(
                    class,
                    Id(Self::ROW + 7),
                    "Master private key",
                    None,
                ))
                .child(self.kind("Status line · badges, lock; no countdown"))
                // The line is drawn at the screen's own width, as Home
                // draws it: it carries the screen padding itself, and
                // §4.8's badge is sized against the whole 268 dp line,
                // not against a page that has already been padded.
                .child(
                    Node::column()
                        .padding(Edges::symmetric(-tokens::PAD, 0.0))
                        .child(nav::status_line(
                            badges::badge_row(
                                Some(badges::tier_badge(
                                    Some(Id(Self::ROW + 8)),
                                    self.tier_badge(),
                                )),
                                badges::network_badge(Network::Signet),
                                None,
                            ),
                            Some(Id(Self::ROW + 9)),
                        )),
                )
                .child(self.kind("Pager · one grammar"))
                .child(nav::pager(
                    class,
                    Self::PAGER_PREV,
                    Self::PAGER_NEXT,
                    "Output 2 of 2",
                    false,
                    true,
                ))
                .child(self.kind("Hub · tile grid"))
                .child(nav::hub(&self.metrics(), tiles).height(hub_height(class)))
                .child(self.kind("Sidebar · wide"))
                .child(
                    nav::sidebar(&sidebar_items, false)
                        .height(tokens::menu_row(class) * sidebar_items.len() as f32)
                        .max_width(tokens::SIDEBAR_WIDTH),
                )
                .child(self.kind("Icons · the whole set, one row each"))
                .child(self.icon_set()),
        )
    }

    /// Every glyph the product draws, with its name (§3: one meaning per
    /// glyph, so the set is short enough to read in one list).
    fn icon_set(&self) -> Node {
        let class = self.class();
        Icon::ALL.iter().fold(Node::column(), |column, icon| {
            column.child(Node::widget(
                Widget::list_row(None, icon.name(), None, None)
                    .with_icon(*icon)
                    .with_height(tokens::menu_row(class)),
            ))
        })
    }

    // -----------------------------------------------------------------
    // 4.2 Choice
    // -----------------------------------------------------------------

    fn choice(&self) -> Node {
        let class = self.class();
        self.scroll_page(
            Node::column()
                .gap(tokens::GAP)
                .child(self.kind("Choice · the chosen row is checked, Continue below"))
                .child(choice::choice_list(
                    class,
                    vec![
                        choice::choice_row(
                            class,
                            Some(Id(Self::CHOICE)),
                            "12",
                            None,
                            self.chosen == 0,
                        ),
                        choice::choice_row(
                            class,
                            Some(Id(Self::CHOICE + 1)),
                            "24",
                            None,
                            self.chosen == 1,
                        ),
                        choice::choice_row(
                            class,
                            None,
                            "From a card",
                            Some(String::from("no reader")),
                            false,
                        ),
                    ],
                ))
                .child(components::primary(Id(Self::ROW + 12), "Continue", true))
                .child(self.kind("Choice · Continue dimmed until a row is checked"))
                .child(components::primary(Id(Self::ROW + 13), "Continue", false))
                .child(self.kind("Setting · the check is the current value, no Continue"))
                .child(choice::setting_list(
                    class,
                    vec![
                        choice::setting_row(class, Id(Self::SETTING), "sats", self.setting == 0),
                        choice::setting_row(class, Id(Self::SETTING + 1), "BTC", self.setting == 1),
                    ],
                ))
                .child(self.kind("Pair · exactly two short options"))
                .child(choice::pair(
                    (Id(Self::PAIR), "Receive"),
                    (Id(Self::PAIR + 1), "Change"),
                    self.change,
                ))
                .child(self.kind("Toggle · a boolean setting"))
                .child(choice::toggle_row(
                    class,
                    Self::TOGGLE,
                    "Shuffle the PIN pad",
                    self.toggle_on,
                    None,
                ))
                .child(self.kind("Value row · label above, value below"))
                .child(choice::value_row(
                    class,
                    Id(Self::ROW + 10),
                    "Script type",
                    "SegWit",
                    Tone::Text,
                ))
                .child(choice::value_row(
                    class,
                    Id(Self::ROW + 11),
                    "Network",
                    "Signet",
                    Tone::Text,
                )),
        )
    }

    // -----------------------------------------------------------------
    // 4.3 Entry
    // -----------------------------------------------------------------

    fn candidates(&self) -> Vec<String> {
        WORDS
            .iter()
            .filter(|w| w.starts_with(self.typed.as_str()))
            .take(tokens::candidates(self.class(), false))
            .map(|w| String::from(*w))
            .collect()
    }

    fn enabled_letters(&self) -> keyboard::KeyMask {
        WORDS
            .iter()
            .filter(|w| w.starts_with(self.typed.as_str()))
            .filter_map(|w| w[self.typed.len()..].chars().next())
            .fold(0, |m, c| m | keyboard::letter_bit(c))
    }

    fn entry(&self) -> Node {
        let class = self.class();
        let inset = self.ctx().inset_bottom_dp;
        let one = vec![String::from("zebra")];
        let rolls: Vec<String> = ROLL_FACES.iter().map(|f| String::from(*f)).collect();
        let masked = Entries {
            values: &rolls,
            revealed: false,
            flash: true,
        };
        let shown = Entries {
            values: &rolls,
            revealed: true,
            flash: false,
        };
        self.scroll_page(
            Node::column()
                .gap(tokens::GAP)
                .child(self.kind("Text field · 52 dp, 40 on the panel"))
                .child(entry::text_field(class, self.typed.clone(), false))
                .child(self.kind("Inline error · one caption line, always reserved"))
                .child(entry::inline_error(
                    class,
                    Some(String::from("Not a word in the list")),
                ))
                .child(self.kind("Candidates"))
                .child(entry::candidate_strip(entry::Strip {
                    class,
                    first: Self::CANDIDATES,
                    second: Self::CANDIDATES_2,
                    words: &self.candidates(),
                    accepting: false,
                    more: false,
                    one_char: false,
                    selected: None,
                }))
                .child(self.kind("Candidates · one selected, the next tap takes it"))
                .child(entry::candidate_strip(entry::Strip {
                    class,
                    first: Self::CANDIDATES,
                    second: Self::CANDIDATES_2,
                    words: &self.candidates(),
                    accepting: false,
                    more: false,
                    one_char: false,
                    selected: Some(1),
                }))
                .child(self.kind("Candidates · one left, the ✓ takes it"))
                .child(entry::candidate_strip(entry::Strip {
                    class,
                    first: Self::CANDIDATES,
                    second: Self::CANDIDATES_2,
                    words: &one,
                    accepting: true,
                    more: false,
                    one_char: false,
                    selected: None,
                }))
                .child(self.kind("PIN pad · the field directly above it, at its width"))
                .child(
                    entry::pin_pad(class, inset, Self::PIN, 2, self.toggle_on.then_some(7)).height(
                        tokens::field(class) + tokens::GAP + self.pad_block(KeyboardKind::Pin),
                    ),
                )
                .child(self.kind("Dice entries · masked, the newest one showing"))
                .child(entry::entropy_entries(
                    class,
                    tokens::PAD_MAX_WIDTH,
                    &masked,
                ))
                .child(self.kind("Dice entries · held or shown by the eye"))
                .child(entry::entropy_entries(class, tokens::PAD_MAX_WIDTH, &shown))
                .child(self.kind("Dice pad · progress line, bar and masked entries"))
                .child(
                    entry::dice_pad(
                        class,
                        inset,
                        Self::DICE,
                        "23 of 50 · 59 bits",
                        ROLLS.0 / ROLLS.1,
                        Some(&masked),
                    )
                    .height(self.entropy_block(KeyboardKind::Dice)),
                )
                .child(self.kind("Coin pad"))
                .child(
                    entry::coin_pad(
                        class,
                        inset,
                        Self::COIN,
                        "40 of 128 flips",
                        FLIPS.0 / FLIPS.1,
                        None,
                    )
                    .height(self.entropy_block(KeyboardKind::Coin)),
                )
                .child(self.kind("Keyboard · BIP-39, dead keys dimmed"))
                .child(
                    Node::widget(Widget::Keyboard {
                        id: Self::KEYBOARD,
                        kind: KeyboardKind::Bip39,
                        enabled: self.enabled_letters(),
                        scramble: None,
                    })
                    .height(self.keyboard_block(KeyboardKind::Bip39)),
                ),
        )
    }

    /// Height in dp of a pad on this class, as [`entry`] fixes it.
    fn pad_block(&self, kind: KeyboardKind) -> f32 {
        keyboard::rows(kind) as f32
            * keyboard::max_key_height(kind, self.class(), tokens::PAD_MAX_WIDTH)
            + tokens::key_bottom_reserve(self.ctx().inset_bottom_dp)
    }

    /// The pad plus the progress line, the bar and the masked entries
    /// above it.
    fn entropy_block(&self, kind: KeyboardKind) -> f32 {
        self.pad_block(kind)
            + tokens::line_box(tokens::LABEL)
            + tokens::GAP_SMALL
            + tokens::BAR_HEIGHT
            + tokens::GAP_SMALL
            + tokens::line_box(tokens::MONO)
            + tokens::GAP
    }

    /// Height in dp of a letter keyboard on this class.
    fn keyboard_block(&self, kind: KeyboardKind) -> f32 {
        keyboard::rows(kind) as f32 * tokens::key_height(self.class())
            + tokens::key_bottom_reserve(self.ctx().inset_bottom_dp)
    }

    // -----------------------------------------------------------------
    // 4.4 Identity and keys
    // -----------------------------------------------------------------

    fn identity(&self) -> Node {
        let class = self.class();
        let keys = vec![String::from(FINGERPRINT)];
        let both = vec![String::from(FINGERPRINT), String::from(SECOND_KEY)];
        let signing = vec![
            (
                Id(Self::SIGN_WITH),
                String::from(FINGERPRINT),
                self.signing[0],
            ),
            (
                Id(Self::SIGN_WITH + 1),
                String::from(SECOND_KEY),
                self.signing[1],
            ),
        ];
        self.scroll_page(
            Node::column()
                .gap(tokens::GAP)
                .child(self.kind("Fingerprint · eight hex, never chunked"))
                .child(identity::fingerprint(FINGERPRINT))
                .child(self.kind("Key row"))
                .child(identity::key_row(
                    class,
                    Id(Self::ROW + 20),
                    FINGERPRINT,
                    "mainnet · passphrase",
                ))
                .child(identity::key_row(
                    class,
                    Id(Self::ROW + 21),
                    SECOND_KEY,
                    "signet",
                ))
                .child(self.kind("Key context · one key"))
                .child(identity::key_context(
                    class,
                    Some(Id(Self::ROW + 22)),
                    "Key",
                    &keys,
                ))
                .child(self.kind("Key context · a multisig"))
                .child(identity::key_context(
                    class,
                    Some(Id(Self::ROW + 23)),
                    "Key",
                    &both,
                ))
                .child(self.kind("Sign with · a chip per key, tap toggles"))
                .child(identity::sign_with("Sign with", &signing)),
        )
    }

    // -----------------------------------------------------------------
    // 4.5 Long strings
    // -----------------------------------------------------------------

    fn long_strings(&self) -> Node {
        let class = self.class();
        let ctx = self.ctx();
        let avail = self.metrics().width_dp - 2.0 * tokens::PAD;
        self.scroll_page(
            Node::column()
                .gap(tokens::GAP)
                .child(self.kind("Comparison string · whole, chunked in fours"))
                .child(strings::comparison_string(
                    class,
                    Self::CHUNKED,
                    ADDRESS,
                    false,
                ))
                .child(self.kind("Reference row · head 8 · tail 8, the row opens it"))
                .child(strings::reference_row(
                    class,
                    Id(Self::ROW + 30),
                    "Account key",
                    XPUB,
                ))
                .child(strings::reference_row(
                    class,
                    Id(Self::ROW + 31),
                    "Signature",
                    ADDRESS,
                ))
                .child(self.kind("Descriptor · structure, the key elided"))
                .child(strings::descriptor(
                    &ctx,
                    avail,
                    DESCRIPTOR,
                    Some(Id(Self::ROW + 32)),
                ))
                .child(self.kind("Compare · the whole string, centred"))
                .child(strings::compare_label("Account key"))
                .child(strings::comparison_string(
                    class,
                    Self::CHUNKED_2,
                    XPUB,
                    true,
                )),
        )
    }

    // -----------------------------------------------------------------
    // 4.6 Paths and script types
    // -----------------------------------------------------------------

    fn paths(&self) -> Node {
        let class = self.class();
        self.scroll_page(
            Node::column()
                .gap(tokens::GAP)
                .child(self.kind("Path · a mono string, read-only"))
                .child(paths::path("m/84h/0h/0h/0/0"))
                .child(self.kind("Path row · opens the editor"))
                .child(paths::path_row(class, Id(Self::ROW + 40), "Path", PATH))
                .child(self.kind("Script type · a value, chosen from a list"))
                .child(paths::script_type_row(
                    class,
                    Id(Self::ROW + 41),
                    "Script type",
                    "SegWit",
                ))
                .child(paths::script_type_row(
                    class,
                    Id(Self::ROW + 42),
                    "Script type",
                    "Taproot",
                ))
                .child(self.kind("Path preset · a choice row above the field"))
                .child(choice::choice_list(
                    class,
                    vec![
                        choice::choice_row(
                            class,
                            Some(Id(Self::CHOICE + 10)),
                            "m/84h/0h/0h",
                            None,
                            true,
                        ),
                        choice::choice_row(
                            class,
                            Some(Id(Self::CHOICE + 11)),
                            "m/86h/0h/0h",
                            None,
                            false,
                        ),
                    ],
                )),
        )
    }

    // -----------------------------------------------------------------
    // 4.7 Amounts
    // -----------------------------------------------------------------

    fn amounts(&self) -> Node {
        let m = self.metrics();
        self.scroll_page(
            Node::column()
                .gap(tokens::GAP)
                .child(self.kind("Amount · sat, the default unit"))
                .child(records::record(
                    &m,
                    Self::rows(
                        vec![Record::mono(
                            "Amount",
                            components::amount(AMOUNT_SAT, Unit::Sat),
                        )],
                        components::fee_rows("Fee", "Rate", FEE_SAT, Unit::Sat, Some(FEE_RATE)),
                    ),
                ))
                .child(self.kind("Fee · the rate row goes when it is unknown"))
                .child(records::record(
                    &m,
                    components::fee_rows("Fee", "Rate", FEE_SAT, Unit::Sat, None),
                ))
                .child(self.kind("Amount · BTC, the same screen by setting"))
                .child(records::record(
                    &m,
                    Self::rows(
                        vec![Record::mono(
                            "Amount",
                            components::amount(AMOUNT_SAT, Unit::Btc),
                        )],
                        components::fee_rows("Fee", "Rate", FEE_SAT, Unit::Btc, Some(FEE_RATE)),
                    ),
                ))
                .child(records::record(
                    &m,
                    vec![Record::mono(
                        "Total",
                        components::amount(TOTAL_SAT, Unit::Btc),
                    )],
                )),
        )
    }

    // -----------------------------------------------------------------
    // 4.8 Badges
    // -----------------------------------------------------------------

    fn badges(&self) -> Node {
        let class = self.class();
        // One badge per line: four pills side by side do not fit the
        // smallest panel, and a badge is read, not aimed at.
        let mut networks = Node::column().gap(tokens::GAP_SMALL);
        for network in [
            Network::Mainnet,
            Network::Testnet,
            Network::Signet,
            Network::Regtest,
        ] {
            networks = networks.child(Node::row().gap(tokens::GAP).align(Align::Center).child(
                match badges::network_badge(network) {
                    Some(badge) => badge,
                    None => Node::widget(Widget::text(
                        "mainnet · no badge",
                        Font::regular(tokens::CAPTION),
                        Tone::Muted,
                    )),
                },
            ));
        }
        let mut kinds = Node::column().gap(tokens::GAP_SMALL);
        // §4.8 Mine lists these two and no third: an output to somebody
        // else carries no badge at all.
        for badge in [OutputBadge::Mine, OutputBadge::MineNotVerified] {
            kinds = kinds.child(
                Node::row()
                    .gap(tokens::GAP)
                    .align(Align::Center)
                    .child(badges::output_badge(badge)),
            );
        }
        kinds = kinds.child(
            Node::row()
                .gap(tokens::GAP)
                .align(Align::Center)
                .child(Node::widget(Widget::text(
                    "to someone else · no badge",
                    Font::regular(tokens::CAPTION),
                    Tone::Muted,
                ))),
        );
        self.scroll_page(
            Node::column()
                .gap(tokens::GAP)
                .child(self.kind("Network · absent on mainnet"))
                .child(networks)
                .child(self.kind("Tier"))
                .child(
                    Node::row()
                        .gap(tokens::GAP_SMALL)
                        .align(Align::Center)
                        .child(badges::tier_badge(
                            Some(Id(Self::ROW + 52)),
                            "Tier C · desktop",
                        )),
                )
                .child(self.kind("Mine · on an output that pays a loaded key"))
                .child(kinds)
                .child(self.kind("State · a row value, never a badge"))
                .child(badges::state_row(
                    class,
                    Id(Self::ROW + 50),
                    "Backup",
                    "not verified",
                    Tone::Caution,
                ))
                // §5: a neutral state keeps the body tone; only the
                // label is muted, as the key menu draws it.
                .child(badges::state_row(
                    class,
                    Id(Self::ROW + 51),
                    "Backup",
                    "verified",
                    Tone::Text,
                )),
        )
    }

    // -----------------------------------------------------------------
    // 4.9 Codes
    // -----------------------------------------------------------------

    fn codes(&self) -> Node {
        use alloc::rc::Rc;
        use osk_codec::qr::{Ecc, Payload, encode};
        let class = self.class();
        let m = self.metrics();
        let public = Rc::new(encode(Payload::Bytes(ADDRESS.as_bytes()), Ecc::Low).expect("fits"));
        let secret = Rc::new(encode(Payload::Numeric(&[b'0'; 48]), Ecc::Low).expect("fits"));
        let side = codes::viewfinder_side(
            class,
            m.width_dp - 2.0 * tokens::PAD,
            tokens::qr_side(class),
        );
        self.scroll_page(
            Node::column()
                .gap(tokens::GAP)
                .child(self.kind("QR out · the class's side, centred, label under it"))
                .child(codes::qr_with_label(class, public, None, "Wallet export"))
                .child(self.kind("Mode · one Animated switch"))
                .child(codes::animated_row(
                    class,
                    Id(Self::SETTING + 10),
                    "Animated",
                    self.toggle_on,
                    None,
                ))
                .child(self.kind("Mode · forced, dimmed with the reason"))
                .child(codes::animated_row(
                    class,
                    Id(Self::SETTING + 11),
                    "Animated",
                    true,
                    Some(String::from("too dense")),
                ))
                .child(codes::progress_bar(
                    CODE_PARTS.0 / CODE_PARTS.1,
                    Some(String::from("2 of 5")),
                ))
                .child(self.kind("QR out · a secret QR, masked"))
                .child(codes::qr_block(
                    class,
                    secret,
                    Some((Self::REVEAL_2, self.state.is_held(Self::REVEAL_2))),
                ))
                .child(self.kind("Scanner · a square, the state and the parts inside it"))
                .child(codes::viewfinder(
                    Self::VIEWFINDER,
                    Self::VIEWFINDER_STATE,
                    side,
                    "3 of 8",
                    Some(SCAN_PARTS.0 / SCAN_PARTS.1),
                    Some(preview_frame()),
                )),
        )
    }

    // -----------------------------------------------------------------
    // 4.10 Secrets
    // -----------------------------------------------------------------

    fn secrets(&self) -> Node {
        let class = self.class();
        let words: Vec<String> = SEED.iter().map(|w| String::from(*w)).collect();
        let room = secrets::panel_room(self.metrics().width_dp);
        let width = secrets::WordWidth::LATIN;
        let revealed_first = self.state.is_held(Self::REVEAL);
        let columns = secrets::word_columns(class, room, width, revealed_first, false);
        let pages = secrets::word_pages(class, columns, &words);
        let (first, page) = pages.first().cloned().unwrap_or((1, Vec::new()));
        let last = first + page.len() - 1;
        let revealed = self.state.is_held(Self::REVEAL);
        self.scroll_page(
            Node::column()
                .gap(tokens::GAP)
                .child(self.kind("Words panel · no caption, hold to show"))
                .child(secrets::secret_panel(
                    class,
                    Self::REVEAL,
                    Secret::Words {
                        words: &page,
                        first,
                        indices: revealed.then_some(&INDICES[..page.len()]),
                        width,
                        steel: false,
                    },
                    revealed,
                    room,
                ))
                .child(nav::pager(
                    class,
                    Self::PAGER_PREV,
                    Self::PAGER_NEXT,
                    alloc::format!("Words {first} to {last}"),
                    true,
                    pages.len() < 2,
                ))
                .child(self.kind("Value panel · one line"))
                .child(secrets::secret_panel(
                    class,
                    Self::REVEAL_2,
                    Secret::Value("c0ffee42"),
                    self.state.is_held(Self::REVEAL_2),
                    room,
                ))
                .child(self.kind("The eye · plain, and the ring while the reveal runs"))
                .child(
                    Node::row()
                        .gap(tokens::GAP)
                        .align(Align::Center)
                        .child(nav::bar_eye(Self::EYE, None))
                        .child(nav::bar_eye(
                            Id(Self::ROW + 62),
                            Some(progress::reveal_remaining(EYE_LEFT_S)),
                        ))
                        .child(nav::bar_eye(
                            Id(Self::ROW + 63),
                            Some(progress::reveal_remaining(EYE_NEARLY_S)),
                        )),
                )
                .child(self.kind("Long secret · a row that opens the screen"))
                .child(secrets::secret_row(
                    class,
                    Id(Self::ROW + 60),
                    "Master private key",
                    None,
                ))
                .child(secrets::secret_row(
                    class,
                    Id(Self::ROW + 61),
                    "Seed hex",
                    Some(XPUB),
                )),
        )
    }

    // -----------------------------------------------------------------
    // 4.11 Records and results
    // -----------------------------------------------------------------

    fn records(&self) -> Node {
        let m = self.metrics();
        self.scroll_page(
            Node::column()
                .gap(tokens::GAP)
                .child(self.kind("Result · icon and coloured title, top-grouped"))
                .child(records::result(Icon::Check, Tone::Success, "Signed"))
                .child(self.kind("Record · a table, long strings as reference rows"))
                .child(records::record(
                    &m,
                    Self::rows(
                        vec![Record::mono(
                            "Amount",
                            components::amount(AMOUNT_SAT, Unit::Sat),
                        )],
                        Self::rows(
                            components::fee_rows("Fee", "Rate", FEE_SAT, Unit::Sat, Some(FEE_RATE)),
                            vec![
                                Record::reference(Self::CHUNKED, "To", ADDRESS),
                                Record::badge(badges::output_badge(OutputBadge::Mine)),
                                Record::mono("Path", PATH),
                            ],
                        ),
                    ),
                ))
                .child(self.kind("Warning · the label above its value, ranked"))
                .child(records::warning_card(
                    &self.metrics(),
                    WarningLevel::Danger,
                    "Change",
                    "not verified",
                ))
                .child(records::warning_card(
                    &self.metrics(),
                    WarningLevel::Caution,
                    "Fee",
                    "12 % of the amount",
                ))
                .child(records::warning_card(
                    &self.metrics(),
                    WarningLevel::Info,
                    "Transfer",
                    "to this key",
                ))
                .child(self.kind("Dimmed control · the reason under the label"))
                .child(nav::dimmed_row(
                    self.class(),
                    Some(Icon::Scan),
                    "Scan a QR",
                    Some(String::from("no camera")),
                ))
                .child(self.kind("Dimmed control · nothing to say, so nothing said"))
                .child(nav::dimmed_row(
                    self.class(),
                    Some(Icon::Learn),
                    "Learn",
                    None,
                )),
        )
    }

    // -----------------------------------------------------------------
    // 4.12 Text
    // -----------------------------------------------------------------

    fn text(&self) -> Node {
        self.scroll_page(
            Node::column()
                .gap(tokens::GAP)
                .child(self.kind("Title"))
                .child(components::title("Word 7 of 12"))
                .child(self.kind("Label"))
                .child(components::label("Fingerprint"))
                .child(self.kind("Value · body and mono"))
                .child(components::value("Recipient", false, Tone::Text))
                .child(components::value(FINGERPRINT, true, Tone::Text))
                .child(components::value("not verified", false, Tone::Caution))
                .child(self.kind("Explainer · on a Learn page, never on a working screen"))
                .child(components::explainer(EXPLAINER)),
        )
    }

    // -----------------------------------------------------------------
    // 4.13 Actions
    // -----------------------------------------------------------------

    fn actions(&self) -> Node {
        let class = self.class();
        let hold_label = if self.hold_done {
            "Held ✓"
        } else {
            "Hold to sign"
        };
        self.scroll_page(
            Node::column()
                .gap(tokens::GAP)
                .child(self.kind("Primary · live and dead"))
                .child(components::primary(Id(Self::ROW + 70), "Continue", true))
                .child(components::primary(Id(Self::ROW + 71), "Continue", false))
                .child(self.kind("Secondary and primary · equal widths on touch"))
                .child(
                    Node::row()
                        .gap(tokens::GAP)
                        .child(
                            components::secondary(Id(Self::ROW + 72), "Show numbers").weight(1.0),
                        )
                        .child(
                            components::primary(Id(Self::ROW + 73), "Continue", true).weight(1.0),
                        ),
                )
                .child(self.kind("Hold · the label is the verb"))
                .child(components::hold(Self::HOLD, hold_label, false, true))
                .child(components::hold(
                    Id(Self::ROW + 74),
                    "Hold to wipe",
                    true,
                    true,
                ))
                .child(self.kind("Hold · dead until the step is available"))
                .child(components::hold(
                    Id(Self::ROW + 77),
                    "Hold to sign",
                    false,
                    false,
                ))
                .child(self.kind("Row action"))
                .child(components::row_action(
                    class,
                    Id(Self::ROW + 75),
                    Icon::Qr,
                    "Show as QR",
                ))
                .child(components::row_action(
                    class,
                    Id(Self::ROW + 76),
                    Icon::File,
                    "Save to file",
                )),
        )
    }

    // -----------------------------------------------------------------
    // 4.14 Progress and state
    // -----------------------------------------------------------------

    fn progress(&self) -> Node {
        let class = self.class();
        let m = self.metrics();
        self.scroll_page(
            Node::column()
                .gap(tokens::GAP)
                .child(self.kind("Progress · a bar and its count"))
                .child(progress::progress(CODE_PARTS.0 / CODE_PARTS.1, "2 of 5"))
                .child(progress::progress(ROLLS.0 / ROLLS.1, "23 of 50 · 59 bits"))
                .child(self.kind("Countdown · the eye's ring, and nothing else"))
                .child(
                    Node::row()
                        .gap(tokens::GAP)
                        .align(Align::Center)
                        .child(nav::bar_eye(
                            Id(Self::ROW + 82),
                            Some(progress::reveal_remaining(EYE_LEFT_S)),
                        ))
                        .child(nav::bar_eye(
                            Id(Self::ROW + 83),
                            Some(progress::reveal_remaining(EYE_NEARLY_S)),
                        )),
                )
                .child(self.kind("Empty state · a row that starts the flow"))
                .child(progress::empty_row(
                    class,
                    Id(Self::ROW + 80),
                    Icon::Download,
                    "Load a key",
                ))
                .child(progress::empty_row(
                    class,
                    Id(Self::ROW + 81),
                    Icon::Dice,
                    "Create a key",
                ))
                .child(self.kind("Terminal state · a result with no action and no way back"))
                .child(progress::terminal(
                    &m,
                    Icon::Error,
                    Tone::Danger,
                    "Self-test failed",
                    vec![Record::text("Vector", "3", Tone::Text)],
                ))
                .child(progress::terminal(
                    &m,
                    Icon::Lock,
                    Tone::Muted,
                    "Session ended",
                    Vec::new(),
                )),
        )
    }

    // -----------------------------------------------------------------
    // 5 The reusable screens
    // -----------------------------------------------------------------

    /// The areas the `wide` sidebar lists (§4.1: Home, then the three
    /// of the band).
    fn nav(&self) -> Vec<SidebarItem> {
        [
            (Icon::House, "Home", true),
            (Icon::Tools, "Tools", false),
            (Icon::Learn, "Learn", false),
            (Icon::Settings, "Settings", false),
        ]
        .iter()
        .enumerate()
        .map(|(i, (icon, label, selected))| SidebarItem {
            id: Some(Id(Self::SIDEBAR + i as u32)),
            icon: *icon,
            label: String::from(*label),
            selected: *selected,
        })
        .collect()
    }

    /// The chrome every §5 page draws in: the whole window, the areas
    /// the sidebar lists, and the back chevron.
    fn chrome<'a>(&self, m: &'a Metrics, nav: &'a [SidebarItem]) -> Chrome<'a> {
        Chrome {
            m,
            nav,
            back: Some(Id(Self::SCREEN)),
            dimmed: false,
            info: None,
        }
    }

    /// The facts a Sign review states, as record rows (§4.11). The
    /// address is a reference row in the table's flow (§4.5); there is
    /// no "Kind" row (§4.8).
    fn sign_rows(&self) -> Vec<Record> {
        Self::rows(
            vec![Record::mono(
                "Amount",
                components::amount(AMOUNT_SAT, Unit::Sat),
            )],
            Self::rows(
                components::fee_rows("Fee", "Rate", FEE_SAT, Unit::Sat, Some(FEE_RATE)),
                vec![Record::reference(Self::CHUNKED, "To", ADDRESS)],
            ),
        )
    }

    /// One QR, for the pages that show one.
    fn code(&self, text: &str) -> alloc::rc::Rc<osk_codec::qr::QrMatrix> {
        use osk_codec::qr::{Ecc, Payload, encode};
        alloc::rc::Rc::new(encode(Payload::Bytes(text.as_bytes()), Ecc::Low).expect("fits"))
    }

    /// The page's own name, which a §5 page carries in its app bar.
    fn screen_title(&self) -> &'static str {
        self.page_title()
    }

    /// §4.8 Tier: "Tier C · desktop" everywhere but the 268 dp status
    /// line, where the full form does not fit beside the network badge
    /// and two buttons.
    fn tier_badge(&self) -> &'static str {
        if self.class() == SizeClass::Small {
            "C · desktop"
        } else {
            "Tier C · desktop"
        }
    }

    fn screen_page(&self) -> Node {
        match self.page - SCREENS {
            0 => self.screen_hub(),
            1 => self.screen_menu(),
            2 => self.screen_choice(),
            3 => self.screen_entry(),
            4 => self.screen_pad(screens::PadKind::Pin {
                scramble: None,
                done: true,
            }),
            5 => self.screen_pad(screens::PadKind::Dice { done: true }),
            6 => self.screen_words(false),
            7 => self.screen_words(true),
            8 => self.screen_secret(true),
            9 => self.screen_secret(false),
            10 => self.screen_compare(),
            11 => self.screen_addresses(),
            12 => self.screen_address(),
            13 => self.screen_qr(false),
            14 => self.screen_qr(true),
            15 => self.screen_record(false),
            16 => self.screen_record(true),
            17 => self.screen_result(true),
            18 => self.screen_result(false),
            19 => self.screen_hold(),
            20 => self.screen_scanner(),
            _ => self.screen_document(),
        }
    }

    fn screen_hub(&self) -> Node {
        let m = self.metrics();
        let nav = self.nav();
        let c = Chrome {
            m: &m,
            nav: &nav,
            back: None,
            dimmed: false,
            info: None,
        };
        let status = screens::Status {
            tier: String::from(self.tier_badge()),
            tier_id: Some(Id(Self::SCREEN)),
            network: Network::Signet,
            lock: Some(Id(Self::SCREEN + 3)),
            session: None,
            notice: None,
        };
        let tiles = [
            (Icon::Wallet, "Wallets"),
            (Icon::Keys, "Keys"),
            (Icon::Scan, "Scan"),
            (Icon::Tools, "Tools"),
            (Icon::Learn, "Learn"),
            (Icon::Settings, "Settings"),
        ]
        .iter()
        .enumerate()
        .map(|(i, (icon, label))| Tile {
            id: Id(Self::TILE + i as u32),
            icon: *icon,
            label: String::from(*label),
            enabled: true,
            badge: None,
        })
        .collect();
        screens::launcher(&c, &status, tiles)
    }

    fn screen_menu(&self) -> Node {
        let m = self.metrics();
        let nav = self.nav();
        let c = self.chrome(&m, &nav);
        let rows = vec![
            screens::Row::Menu {
                id: Id(Self::SCREEN + 1),
                icon: Some(Icon::Shield),
                label: String::from("Backup"),
                value: Some(String::from("not verified")),
                tone: Tone::Caution,
            },
            screens::Row::Value {
                id: Id(Self::SCREEN + 2),
                label: String::from("Script type"),
                value: String::from("SegWit"),
                tone: Tone::Text,
            },
            screens::Row::Reference {
                id: Id(Self::SCREEN + 3),
                label: String::from("Account key"),
                value: String::from(XPUB),
            },
            screens::Row::Secret {
                id: Id(Self::SCREEN + 4),
                label: String::from("Master private key"),
                value: None,
            },
            screens::Row::Action {
                id: Id(Self::SCREEN + 5),
                icon: Icon::Qr,
                label: String::from("Show as QR"),
            },
            screens::Row::Dimmed {
                icon: Some(Icon::Scan),
                label: String::from("Scan a QR"),
                reason: Some(String::from("no camera")),
            },
        ];
        screens::menu(
            &c,
            self.screen_title(),
            Some(self.key_context()),
            rows,
            vec![],
        )
    }

    /// The key context §4.4 puts at the top of a body that acts on a
    /// key the user can switch.
    fn key_context(&self) -> screens::KeyContext {
        screens::KeyContext {
            id: Some(Id(Self::SCREEN + 6)),
            label: String::from("Key"),
            keys: vec![String::from(FINGERPRINT)],
        }
    }

    fn screen_choice(&self) -> Node {
        let m = self.metrics();
        let nav = self.nav();
        let c = self.chrome(&m, &nav);
        let items = vec![
            screens::Item::chosen(Id(Self::SCREEN + 1), "12", self.chosen == 0),
            screens::Item::chosen(Id(Self::SCREEN + 2), "24", self.chosen == 1),
            screens::Item::chosen(Id(Self::SCREEN + 3), "Dice", self.chosen == 2),
            screens::Item::dimmed("From a card", "no reader"),
        ];
        // §4.2: "The question is the title."
        screens::choice(
            &c,
            "How many words?",
            items,
            screens::Action::new(Id(Self::SCREEN + 4), "Continue"),
        )
    }

    fn screen_entry(&self) -> Node {
        let m = self.metrics();
        let nav = self.nav();
        let c = self.chrome(&m, &nav);
        // §4.3: the words accepted so far, masked, in the space above
        // the entry group. Seven of twelve are in, so the panel shows
        // five empty rows and keeps the rectangle it will have at the
        // last word.
        let accepted: Vec<String> = SEED
            .iter()
            .take(ACCEPTED_WORDS)
            .map(|w| String::from(*w))
            .collect();
        let e = screens::Entry {
            title: self.screen_title(),
            value: self.typed.clone(),
            mono: false,
            above: screens::Above::Nothing,
            candidates: Some(screens::Candidates {
                first: Self::CANDIDATES,
                second: Self::CANDIDATES_2,
                words: self.candidates(),
                accepting: false,
                more: false,
                one_char: false,
                selected: None,
            }),
            words: Some(screens::WordsSoFar {
                panel: Self::REVEAL_3,
                words: &accepted,
                total: SECRET_WORDS,
                revealed: self.state.is_held(Self::REVEAL_3),
                width: secrets::WordWidth::LATIN,
            }),
            eye: Some((Self::EYE, None)),
            keyboard: (Self::KEYBOARD, KeyboardKind::Bip39),
            enabled: self.enabled_letters(),
            error: None,
        };
        screens::entry(&c, e)
    }

    fn screen_pad(&self, kind: screens::PadKind) -> Node {
        let m = self.metrics();
        let nav = self.nav();
        let c = self.chrome(&m, &nav);
        let dice = !matches!(kind, screens::PadKind::Pin { .. });
        let rolls: Vec<String> = ROLL_FACES.iter().map(|f| String::from(*f)).collect();
        let p = screens::Pad {
            title: String::from(self.screen_title()),
            id: if dice { Self::DICE } else { Self::PIN },
            kind,
            field: if dice {
                screens::Field::Nothing
            } else {
                screens::Field::Dots(2)
            },
            progress: dice.then(|| (String::from("23 of 50 · 59 bits"), ROLLS.0 / ROLLS.1)),
            entries: dice.then(|| Entries {
                values: &rolls,
                revealed: self.state.is_held(Self::DICE),
                flash: true,
            }),
            words: None,
            eye: None,
            caption: None,
            error: None,
            action: dice.then(|| screens::Action::new(Id(Self::SCREEN + 1), "Continue")),
        };
        screens::pad(&c, p)
    }

    fn screen_words(&self, revealed: bool) -> Node {
        let m = self.metrics();
        let nav = self.nav();
        let c = self.chrome(&m, &nav);
        let words: Vec<String> = SEED.iter().map(|w| String::from(*w)).collect();
        let room = secrets::panel_room(c.column().width_dp);
        let width = secrets::WordWidth::LATIN;
        let pages = screens::words::pages(self.class(), room, width, revealed, false, &words);
        let (first, page) = pages.first().cloned().unwrap_or((1, Vec::new()));
        let last = page.len() + first - 1;
        // §5 Words: the title is the thing and the pager names the page.
        let w = screens::Words {
            title: "Words",
            words: &page,
            first,
            // The revealed page carries the wordlist numbers, which is
            // what "Numbers" puts beside the words.
            indices: revealed.then_some(&INDICES[..page.len()]),
            width,
            revealed,
            panel: Self::REVEAL,
            eye: Some(Id(Self::SCREEN + 1)),
            remaining: revealed.then(|| progress::reveal_remaining(EYE_LEFT_S)),
            pager: (pages.len() > 1).then(|| screens::Pager {
                prev: Self::PAGER_PREV,
                next: Self::PAGER_NEXT,
                label: alloc::format!("Words {first} to {last}"),
                at_start: true,
                at_end: false,
            }),
            steel: false,
            numbers_action: Some(screens::Action::new(
                Id(Self::SCREEN + 2),
                if self.class() == SizeClass::Small {
                    "Numbers"
                } else {
                    "Show numbers"
                },
            )),
            extra: None,
            action: screens::Action::new(Id(Self::SCREEN + 3), "Continue"),
        };
        screens::words(&c, w)
    }

    fn screen_secret(&self, revealed: bool) -> Node {
        let m = self.metrics();
        let nav = self.nav();
        let c = self.chrome(&m, &nav);
        let s = screens::Secret {
            rows: Vec::new(),
            action: None,
            secondary: None,
            title: self.screen_title(),
            value: screens::Value::Text(XPRV),
            revealed,
            panel: Self::REVEAL_2,
            eye: Some(Id(Self::SCREEN + 1)),
            remaining: revealed.then(|| progress::reveal_remaining(EYE_LEFT_S)),
            pager: None,
        };
        screens::secret(&c, s)
    }

    fn screen_compare(&self) -> Node {
        let m = self.metrics();
        let nav = self.nav();
        let c = self.chrome(&m, &nav);
        // §5 Compare: the title names the kind and the label the
        // instance, so the page shows the label line the app draws when
        // a string is one of a run.
        screens::compare(
            &c,
            screens::Compare {
                title: self.screen_title(),
                label: Some("#0 · 73c5da0a"),
                value: XPUB,
                id: Self::CHUNKED,
                done: screens::Action::new(Id(Self::SCREEN + 1), "Done"),
                key: None,
                copy: Some(screens::ActionRow {
                    id: Id(Self::SCREEN + 2),
                    icon: Icon::Copy,
                    label: String::from("Copy"),
                    reason: None,
                }),
                caption: Some(String::from("Copied")),
            },
        )
    }

    fn screen_addresses(&self) -> Node {
        let m = self.metrics();
        let nav = self.nav();
        let c = self.chrome(&m, &nav);
        let rows = (0..4)
            .map(|i| screens::AddressRow {
                id: Id(Self::SCREEN + 10 + i),
                label: alloc::format!("Receive {i}"),
                address: String::from(ADDRESS),
            })
            .collect();
        let a = screens::Addresses {
            title: self.screen_title(),
            script: (Some(Id(Self::SCREEN + 1)), "Script type", "Taproot"),
            chain: (
                Id(Self::PAIR),
                "Receive",
                Id(Self::PAIR + 1),
                "Change",
                self.change,
            ),
            rows,
            more: Some((Id(Self::SCREEN + 2), String::from("More"))),
        };
        screens::addresses(&c, a)
    }

    fn screen_address(&self) -> Node {
        let m = self.metrics();
        let nav = self.nav();
        let c = self.chrome(&m, &nav);
        // §5 Address: "the title is the address's name ('Receive 2') and
        // the label under the QR is its script type".
        let a = screens::Address {
            format: None,
            copy: Some(screens::ActionRow {
                id: Id(Self::SCREEN + 4),
                icon: Icon::Copy,
                label: String::from("Copy"),
                reason: None,
            }),
            save: None,
            caption: None,
            title: "Receive 2",
            label: "SegWit",
            address: (Self::CHUNKED, ADDRESS),
            code: self.code(ADDRESS),
        };
        screens::address(&c, a)
    }

    fn screen_qr(&self, animated: bool) -> Node {
        let m = self.metrics();
        let nav = self.nav();
        let c = self.chrome(&m, &nav);
        let q = screens::Qr {
            title: self.screen_title(),
            matrix: self.code(if animated { DESCRIPTOR } else { ADDRESS }),
            label: "Wallet export",
            toggle: Some((Id(Self::SCREEN + 1), "Animated")),
            animated,
            forced: animated.then(|| String::from("too dense")),
            progress: animated.then(|| (CODE_PARTS.0 / CODE_PARTS.1, String::from("2 of 5"))),
            save: Some(screens::ActionRow {
                id: Id(Self::SCREEN + 2),
                icon: Icon::File,
                label: String::from("Save as PNG"),
                reason: animated.then(|| String::from("too dense")),
            }),
            caption: None,
        };
        screens::qr(&c, q)
    }

    /// The Sign overview, and one output of it: §4.8's MINE badge and
    /// the address as a reference row.
    fn screen_record(&self, output: bool) -> Node {
        let m = self.metrics();
        let nav = self.nav();
        let c = self.chrome(&m, &nav);
        let (rows, warnings) = if output {
            (
                vec![
                    Record::mono("Amount", components::amount(AMOUNT_SAT, Unit::Sat)),
                    Record::reference(Self::CHUNKED, "To", ADDRESS),
                    Record::badge(badges::output_badge(OutputBadge::MineNotVerified)),
                    Record::mono("Path", PATH),
                ],
                vec![(
                    WarningLevel::Danger,
                    String::from("Change"),
                    String::from("not verified"),
                )],
            )
        } else {
            (
                self.sign_rows(),
                vec![(
                    WarningLevel::Caution,
                    String::from("Fee"),
                    String::from("12 % of the amount"),
                )],
            )
        };
        let r = screens::Record {
            title: self.screen_title(),
            key: output.then(|| self.key_context()),
            network: Network::Signet,
            rows,
            warnings,
            pager: None,
            action: Some(screens::Action::new(Id(Self::SCREEN + 2), "Continue")),
        };
        screens::record(&c, r)
    }

    fn screen_result(&self, passed: bool) -> Node {
        let m = self.metrics();
        let nav = self.nav();
        let c = self.chrome(&m, &nav);
        let v = if passed {
            screens::Result {
                caption: None,
                title: self.screen_title(),
                icon: Icon::Check,
                tone: Tone::Success,
                result: "Signed",
                rows: vec![
                    Record::mono("Key", FINGERPRINT),
                    Record::text("Inputs", "2 of 2", Tone::Text),
                ],
                actions: vec![
                    screens::Action::new(Id(Self::SCREEN + 1), "Show as QR"),
                    screens::Action::new(Id(Self::SCREEN + 2), "Done"),
                ],
            }
        } else {
            screens::Result {
                caption: None,
                title: self.screen_title(),
                icon: Icon::Error,
                tone: Tone::Danger,
                result: "Self-test failed",
                rows: vec![Record::text("Vector", "3", Tone::Text)],
                actions: Vec::new(),
            }
        };
        screens::result(&c, v)
    }

    fn screen_hold(&self) -> Node {
        let m = self.metrics();
        let nav = self.nav();
        let c = self.chrome(&m, &nav);
        let h = screens::Hold {
            warnings: Vec::new(),
            title: self.screen_title(),
            rows: self.sign_rows(),
            sign_with: Some((
                String::from("Sign with"),
                vec![
                    (
                        Id(Self::SIGN_WITH),
                        String::from(FINGERPRINT),
                        self.signing[0],
                    ),
                    (
                        Id(Self::SIGN_WITH + 1),
                        String::from(SECOND_KEY),
                        self.signing[1],
                    ),
                ],
            )),
            then_with: None,
            id: Self::HOLD,
            label: "Hold to sign",
            danger: false,
            enabled: true,
            secondary: None,
        };
        screens::hold(&c, h)
    }

    fn screen_scanner(&self) -> Node {
        let m = self.metrics();
        let nav = self.nav();
        let c = self.chrome(&m, &nav);
        let s = screens::Scanner {
            title: self.screen_title(),
            state: "3 of 8",
            parts: Some(SCAN_PARTS.0 / SCAN_PARTS.1),
            ways: vec![
                organisms::TileSpec {
                    id: Id(Self::SCREEN + 1),
                    icon: Icon::File,
                    label: String::from("Read a file"),
                    enabled: true,
                    badge: None,
                },
                organisms::TileSpec {
                    id: Id(Self::SCREEN + 2),
                    icon: Icon::Paste,
                    label: String::from("Paste"),
                    enabled: true,
                    badge: None,
                },
                organisms::TileSpec {
                    id: Id(Self::SCREEN + 3),
                    icon: Icon::Keyboard,
                    label: String::from("Type"),
                    enabled: false,
                    badge: None,
                },
            ],
            preview: Some(preview_frame()),
            action: None,
        };
        screens::scanner(&c, s)
    }

    fn screen_document(&self) -> Node {
        let m = self.metrics();
        let nav = self.nav();
        let c = self.chrome(&m, &nav);
        let sections = vec![
            screens::Section {
                id: None,
                heading: String::from("Tier C"),
                paragraphs: vec![String::from(EXPLAINER)],
            },
            screens::Section {
                id: None,
                heading: String::from("What a tier states"),
                paragraphs: vec![String::from(TIERS), String::from(EXPLAINER)],
            },
        ];
        screens::document(&c, self.screen_title(), sections)
    }

    fn apply(&mut self, action: Action) {
        match action {
            Action::Tap(id) => match id {
                Self::NEXT => self.next_page(),
                Self::TOGGLE => self.toggle_on = !self.toggle_on,
                Id(n) if n == Self::SETTING || n == Self::SETTING + 1 => {
                    self.setting = n - Self::SETTING;
                }
                Id(n) if n == Self::CHOICE || n == Self::CHOICE + 1 => {
                    self.chosen = n - Self::CHOICE;
                }
                Id(n) if n == Self::PAIR || n == Self::PAIR + 1 => {
                    self.change = n == Self::PAIR + 1;
                }
                Id(n) if n == Self::SIGN_WITH || n == Self::SIGN_WITH + 1 => {
                    let i = (n - Self::SIGN_WITH) as usize;
                    self.signing[i] = !self.signing[i];
                }
                _ => {}
            },
            Action::HoldCompleted(_) => self.hold_done = true,
            Action::Candidate(id, i) => {
                let offset = if id == Self::CANDIDATES_2 {
                    tokens::candidates_per_row(self.class(), false)
                } else {
                    0
                };
                if let Some(w) = self.candidates().get(offset + usize::from(i)) {
                    self.typed = w.clone();
                }
            }
            Action::KeyboardInput(id, input) => {
                if id != Self::KEYBOARD {
                    return;
                }
                match input {
                    KeyInput::Char(c) => self.typed.push(c),
                    KeyInput::Backspace => {
                        self.typed.pop();
                    }
                    KeyInput::Done => self.typed.clear(),
                    KeyInput::Shift | KeyInput::Symbols => {}
                }
            }
            Action::Back => self.page = self.page.saturating_sub(1),
            Action::Scrolled(_) | Action::PageChanged(..) | Action::Redraw => {}
        }
    }

    fn render(&mut self) {
        let Some(ctx) = self.ctx else {
            return;
        };
        let tree = self.build();
        let Some(canvas) = self.canvas.as_mut() else {
            return;
        };
        let layout = layout::solve_with(&tree, canvas.bounds(), &ctx, self.state.scroll_offsets());
        canvas.clear(self.theme.background);
        widgets::draw_tree(canvas, &tree, &layout, &self.theme, &self.state);
        self.layout = Some(layout);
        self.commands.push_back(Command::Draw);
    }
}

impl App for Gallery {
    fn event(&mut self, event: Event) {
        match event {
            Event::Display(info) => {
                let info = DisplayInfo {
                    width: info.width.max(1),
                    height: info.height.max(1),
                    ..info
                };
                let scale = Scale::new(info.dpi);
                self.canvas = Some(Canvas::new(&info));
                self.ctx = Some(LayoutCtx::new(scale, SizeClass::of(&info)).with_insets(
                    f32::from(info.inset_top) / scale.factor(),
                    f32::from(info.inset_bottom) / scale.factor(),
                ));
                self.state = UiState::new(scale);
                self.render();
            }
            Event::Touch {
                x,
                y,
                phase: TouchPhase::Up,
            } if self.is_next_tap(i32::from(x), i32::from(y)) => {
                self.next_page();
                self.render();
            }
            other => {
                let Some(layout) = self.layout.as_ref() else {
                    return;
                };
                let action = self.state.event(layout, other);
                if let Some(a) = action {
                    self.apply(a);
                    self.render();
                }
            }
        }
    }

    fn poll_command(&mut self) -> Option<Command> {
        self.commands.pop_front()
    }

    fn frame(&mut self) -> Frame<'_> {
        self.canvas.as_ref().map_or(
            Frame {
                width: 0,
                height: 0,
                rgba: &[],
            },
            Canvas::frame,
        )
    }
}

/// A stand-in camera frame for the two scanner pages: a diagonal
/// gradient with two dark blocks in it. The gallery has no camera, and
/// the state a preview puts the square in is worth documenting.
fn preview_frame() -> components::Preview {
    const W: usize = 64;
    const H: usize = 48;
    let mut pixels = vec![0u8; W * H];
    for y in 0..H {
        for x in 0..W {
            let block = (8..24).contains(&x) && (8..20).contains(&y)
                || (36..56).contains(&x) && (26..40).contains(&y);
            pixels[y * W + x] = if block {
                24
            } else {
                ((x * 255 / (W - 1) + y * 255 / (H - 1)) / 2) as u8
            };
        }
    }
    components::Preview {
        width: W as u16,
        height: H as u16,
        pixels: alloc::rc::Rc::new(zeroize::Zeroizing::new(pixels)),
        chroma: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use osk_shell_api::TouchPhase;

    fn display() -> DisplayInfo {
        DisplayInfo {
            width: 320,
            height: 240,
            dpi: 143,
            inset_bottom: 0,
            inset_top: 0,
            buttons: 0,
            camera_fixed: true,
            secure: osk_shell_api::SecureHardware::None,
            boot: osk_shell_api::BootState::Unknown,
            memory_mib: None,
        }
    }

    #[test]
    fn every_page_renders_and_next_advances() {
        let mut g = Gallery::new();
        g.event(Event::Display(display()));
        assert_eq!(g.poll_command(), Some(Command::Draw));
        assert_eq!(g.frame().rgba.len(), 320 * 240 * 4);
        for i in 0..Gallery::page_count() {
            assert_eq!(g.page(), i);
            let c = g.next_button_center().expect("next button");
            g.event(Event::Touch {
                x: c.x as u16,
                y: c.y as u16,
                phase: TouchPhase::Down,
            });
            g.event(Event::Touch {
                x: c.x as u16,
                y: c.y as u16,
                phase: TouchPhase::Up,
            });
            while g.poll_command().is_some() {}
        }
        assert_eq!(g.page(), 0);
    }

    #[test]
    fn bip39_enabled_set_follows_the_typed_prefix() {
        let mut g = Gallery::new();
        g.typed = String::from("ab");
        let m = g.enabled_letters();
        for c in "aloisu".chars() {
            assert!(m & keyboard::letter_bit(c) != 0, "{c}");
        }
        assert_eq!(m & keyboard::letter_bit('z'), 0);
        g.typed = String::new();
        assert_eq!(
            g.enabled_letters(),
            keyboard::letter_bit('a') | keyboard::letter_bit('z')
        );
    }
}
