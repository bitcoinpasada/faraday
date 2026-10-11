//! The reusable screens (`docs/DESIGN.md` §5).
//!
//! §5 is sixteen screens, and every flow is built from them: "A screen
//! spec names one of them." This module has one function per screen, one
//! file each, and each function's doc comment quotes its §5 row and names
//! the §4 components it is made of.
//!
//! A screen owns its chrome: the app bar, where the body sits between the
//! bar and the bottom action, and the action itself. It takes content —
//! strings, values, ids, states — and never a font, a size or a
//! placement: those come from [`crate::tokens`] and the size class, which
//! is what §2.2 ("one kind of content, one rendering") and §2.3 ("one
//! screen per kind of data") mean in code.
//!
//! Placement is one law for all of them (§2.6), and [`Chrome::place`] is
//! the only place it is written down: the actions and the controls the
//! thumb works are bottom-anchored; lists and documents start at the top
//! and scroll; everything else is one block, vertically centred in the
//! space between the app bar and them. A block that does not fit scrolls
//! from the top. The law holds on every class, so a screen looks the
//! same on a 268 dp panel, a phone and a desktop window; on `wide` the
//! block is capped at [`tokens::COLUMN_MAX_WIDTH`] and the action is
//! bottom-right at its natural width, inside [`wide_frame`], so the
//! sidebar's rectangle is the same on all sixteen. The sidebar is dimmed
//! and inert on the screens §4.1 names: a wizard step (Entry, Pad), a
//! secret (Words, Secret), the scanner, the hold screens and a terminal
//! result.

pub mod address;
pub mod addresses;
pub mod choice;
pub mod compare;
pub mod document;
pub mod entry;
pub mod hold;
pub mod hub;
pub mod menu;
pub mod pad;
pub mod qr;
pub mod record;
pub mod result;
pub mod scanner;
pub mod secret;
pub mod words;

pub use address::{Address, address};
pub use addresses::{Addresses, Row as AddressRow, addresses};
pub use choice::{Item, choice, choice_with_progress, settings};
pub use compare::{Compare, compare, compare_descriptor};
pub use document::{Reading, Section, document, document_with, reading};
pub use entry::{
    Above, Candidates, Entry, EntryRoom, FactRow, WordsSoFar, entry, words_panel_fits,
};
pub use hold::{Hold, SigningKey, hold};
pub use hub::{Status, launcher};
pub use menu::{Row, menu, menu_mono_title};
pub use pad::{Field, Pad, PadKind, pad, pad_words_panel_fits};
pub use qr::{Qr, qr, qr_with_actions};
pub use record::{Record, record, record_mono_title};
pub use result::{Result, result};
pub use scanner::{Scanner, scanner};
pub use secret::{Secret, Value, secret};
pub use words::{Words, words};

use alloc::string::String;
use alloc::vec::Vec;

use crate::components;
use crate::geom::{Edges, Metrics, SizeClass};
use crate::layout::{Id, Justify, Node};
use crate::organisms::{self, Body, SidebarItem};
use crate::tokens;

/// One row under a block: the scanner's "Read a file", "Paste" and
/// "Type", and the Compare and Address screens' "Copy" (§4.13). A row
/// the shell cannot serve is dimmed with the reason under its label
/// (§4.11).
pub struct ActionRow {
    /// Application id.
    pub id: Id,
    /// The row's mark.
    pub icon: crate::widgets::Icon,
    /// The verb.
    pub label: String,
    /// Why the row is dead, where it is: "no clipboard". `None` on a
    /// live row.
    pub reason: Option<String>,
}

impl ActionRow {
    /// The row at the class's height, live or dimmed.
    pub fn node(self, class: SizeClass) -> Node {
        match self.reason {
            Some(reason) => {
                components::dimmed_row(class, Some(self.icon), self.label, Some(reason))
            }
            None => components::row_action(class, self.id, self.icon, self.label),
        }
    }

    /// The height the row takes at that class.
    pub fn height(&self, class: SizeClass) -> f32 {
        if self.reason.is_some() {
            tokens::stacked_row(class)
        } else {
            tokens::menu_row(class)
        }
    }
}

/// The sidebar's own rectangle on `wide`. Reserved by this crate rather
/// than passed in, so that every screen names the same rectangle and a
/// layout test can assert it never moves.
pub const SIDEBAR: Id = Id(u32::MAX);

/// The scroll region of a screen that scrolls. One screen is on the
/// display at a time, so one reserved id carries its offset.
pub const SCROLL: Id = Id(u32::MAX - 1);

/// The text field of an [`entry`] or a [`pad`] screen. Reserved so that
/// a layout test can assert the field sits directly above the strip and
/// the keyboard.
pub const FIELD: Id = Id(u32::MAX - 3);

/// The rectangle a [`pad`] screen reserves for its pad. Reserved here so
/// that a layout test can assert the pad keeps one place whatever the
/// screen above it says (§2.5).
pub const PAD_SLOT: Id = Id(u32::MAX - 2);

/// The bottom group of §2.6: the touch controls a screen puts against
/// its bottom edge, above the action row. Reserved so a layout test can
/// assert where the group sits and where it ends.
pub const GROUP: Id = Id(u32::MAX - 4);

/// The run a dice or coin [`pad`] screen shows inside its pad group:
/// the masked entries, the progress line and the bar (§4.3). Reserved so
/// a layout test can assert it sits directly above the pad, at the pad's
/// width.
pub const ENTRIES: Id = Id(u32::MAX - 6);

/// The block §2.6 centres: everything on the screen that is neither a
/// list from the top nor a control under the thumb. Reserved so a
/// layout test can measure it against the space it sits in.
pub const BLOCK: Id = Id(u32::MAX - 5);

/// The space the block is centred in: what is left between the app bar
/// and the bottom group. Reserved so a layout test can compare the two
/// centres.
pub const SPACE: Id = Id(u32::MAX - 7);

/// The square a [`scanner`] screen draws its preview in (§4.9).
/// Reserved so that a layout test can assert the square is square and
/// that the state line sits inside it.
pub const VIEWFINDER: Id = Id(u32::MAX - 10);

/// The state a [`scanner`] screen draws inside that square: "Looking
/// for a code", "Part 2 of 5" and the bar under it. Reserved so that a
/// layout test can put the one rectangle inside the other.
pub const VIEWFINDER_STATE: Id = Id(u32::MAX - 11);

/// §4.11's reason above a dead primary, inside the action row.
/// Reserved so a layout test can put the caption above the button.
pub const REASON: Id = Id(u32::MAX - 13);

/// The pane a screen of this module draws, whatever else is on it.
/// Reserved so that a layout test can tell a screen built from §5 from
/// one still built out of the older composites: every one of the sixteen
/// goes through [`Chrome::framed`], and nothing else claims this id.
pub const SCREEN: Id = Id(u32::MAX - 9);

/// The square a screen draws its QR in (§4.9). Reserved so that a layout
/// test can assert the square is the class's side
/// ([`tokens::qr_side`]) on every screen that shows a QR, rather than
/// whatever the rows above and below it left over.
pub const CODE: Id = Id(u32::MAX - 12);

/// The reserved caption line under the field of an [`entry`] or a
/// [`pad`] screen (§4.3: "One caption line under the field, always
/// reserved at the caption's line box for the class"). Reserved here so
/// that a layout test can assert the line is there whether or not it
/// says anything, which is what keeps the keyboard and the pad still.
pub const ERROR: Id = Id(u32::MAX - 8);

/// What an [`entry`] screen draws above its field: the fact or mode row,
/// the table of facts, the typed value, or the path editor's preset
/// groups (§4.3). [`BLOCK`] is the space that block is given, which a
/// block measuring nothing overflows; this is what is drawn. Reserved so
/// that a layout test can put the one rectangle against the field's and
/// say that nothing is drawn over the field
/// (`docs/PLANNING.md` §16.121).
pub const ABOVE: Id = Id(u32::MAX - 14);

/// The `wide` template: the navigation sidebar beside the screen's own
/// pane (`docs/DESIGN.md` §4.1).
///
/// Every screen goes through this one function, so the sidebar's
/// rectangle is identical on all sixteen. `dimmed` draws it inert during
/// a wizard, the scanner, a secret, a hold step and a terminal screen.
pub fn wide_frame(items: &[SidebarItem], dimmed: bool, pane: Node) -> Node {
    organisms::sidebar_frame(SIDEBAR, items, dimmed, pane)
}

/// What every screen needs from its host and none of them decides:
/// the area it is drawn in, the areas the sidebar lists, and the id of
/// the app bar's back chevron.
pub struct Chrome<'a> {
    /// The area the screen is drawn in, and its size class.
    pub m: &'a Metrics,
    /// The areas the `wide` sidebar lists. Empty on the other classes,
    /// where there is no sidebar.
    pub nav: &'a [SidebarItem],
    /// The app bar's back chevron, which §4.1 makes "the only way back
    /// and the only cancel". `None` on a screen with no way back.
    pub back: Option<Id>,
    /// The `wide` sidebar is inert because of where the host is: inside
    /// a wizard, on a hold step, on a terminal screen (§4.1). A screen
    /// that is inert wherever it is drawn — Entry, Pad, Words, Secret,
    /// Scanner — says so itself, so this only adds to that.
    pub dimmed: bool,
    /// The id of the app bar's info button, on a screen the host has a
    /// Learn page for (`docs/PLANNING.md` §16.105). `None` elsewhere,
    /// and the eye wins the slot wherever a screen carries one, so no
    /// screen draws two controls there.
    pub info: Option<Id>,
}

impl Chrome<'_> {
    /// The size class the screen is drawn at.
    pub fn class(&self) -> SizeClass {
        self.m.class
    }

    /// The strip at the bottom of the display the shell said the person
    /// cannot use, in dp. What the screen anchors to the bottom — the
    /// action row, a pad's last row, a keyboard's bottom faces — stands
    /// this far clear of the edge.
    pub fn inset_bottom(&self) -> f32 {
        self.m.inset_bottom_dp
    }

    /// The metrics of the pane the screen's own content sits in: the
    /// whole area, less the sidebar on `wide`. A record's label column
    /// and a hub's tiles are measured against this, not against the
    /// window.
    pub fn pane(&self) -> Metrics {
        if self.m.is_wide() {
            self.m
                .area(self.m.width_dp - tokens::SIDEBAR_WIDTH, self.m.height_dp)
        } else {
            *self.m
        }
    }

    /// The metrics a record or a list is measured against: the pane,
    /// capped at [`tokens::COLUMN_MAX_WIDTH`] on `wide` (§3).
    pub fn column(&self) -> Metrics {
        let m = self.pane();
        if m.is_wide() {
            m.area(
                m.width_dp.min(tokens::COLUMN_MAX_WIDTH + 2.0 * tokens::PAD),
                m.height_dp,
            )
        } else {
            m
        }
    }

    /// §3 and §2.6: a form or list column is capped on `wide` and
    /// centred in the pane; on the touch classes it takes the width.
    ///
    /// §3: "On `wide` the capped column, and every list in it, is centred
    /// in the pane under the centred title." The app bar and the action
    /// row take the whole pane, so the title stays centred above the
    /// column and the primary action stays at the pane's bottom-right.
    fn capped(&self, body: Node) -> Node {
        match self.cap() {
            Some(cap) => organisms::capped(cap, body),
            None => body,
        }
    }

    /// The width in dp a column is capped at on this class, or `None`
    /// where the pane is the column (§3).
    fn cap(&self) -> Option<f32> {
        self.m.is_wide().then_some(tokens::COLUMN_MAX_WIDTH)
    }

    /// `pane` inside the `wide` frame, or by itself on the other
    /// classes.
    fn framed(&self, dimmed: bool, pane: Node) -> Node {
        let pane = pane.id(SCREEN);
        let frame = if self.m.is_wide() {
            wide_frame(self.nav, dimmed || self.dimmed, pane)
        } else {
            pane
        };
        self.under_cutout(frame)
    }

    /// The whole screen below the strip the shell reported at the top
    /// edge, so the app bar clears a camera cutout. A shell that handed
    /// the core only the usable area reports none, and the screen keeps
    /// the frame it was given.
    fn under_cutout(&self, frame: Node) -> Node {
        if self.m.inset_top_dp <= 0.0 {
            return frame;
        }
        Node::column()
            .child(frame.weight(1.0).min_height(0.0))
            .padding(Edges {
                left: 0.0,
                right: 0.0,
                top: self.m.inset_top_dp,
                bottom: 0.0,
            })
    }

    /// A screen that scrolls when its content is longer than the panel:
    /// a menu, a choice, a review whose table outgrows the screen, a
    /// document. The body carries the screen padding and fills the space
    /// between the app bar and the footer, so a body placed by
    /// [`place`](Chrome::place) is centred in it and only one that
    /// outgrows it starts at the top; the footer is pinned.
    fn scrolling(&self, s: Frame<'_>) -> Node {
        let back = self.back(&s);
        let footer = (!s.footer.is_empty())
            .then(|| organisms::cta_at(s.footer, tokens::action_bottom(self.inset_bottom())));
        // §4.13's action row carries both the grid step above it and the
        // screen's margin below it, so a body over one needs no padding
        // of its own: four choice rows and a Continue are 268 × 358 dp
        // exactly, and the second margin is what put the fourth row
        // below the fold.
        let bottom = if footer.is_some() { 0.0 } else { tokens::PAD };
        // §3: the cap goes round the scroll region, not inside it, so
        // the scrollbar is drawn against the column it scrolls and not
        // against the window edge. The padding inside the region is the
        // screen's own, so the column is [`tokens::COLUMN_MAX_WIDTH`]
        // wide either way.
        let cap = self.cap().map(|cap| cap + 2.0 * tokens::PAD);
        self.framed(
            s.dimmed,
            organisms::screen_at(
                SCROLL,
                if s.mono_title {
                    organisms::app_bar_mono(back, s.title, self.trailing(s.trailing))
                } else {
                    organisms::app_bar(back, s.title, self.trailing(s.trailing))
                },
                s.body,
                0.0,
                footer,
                bottom,
                cap,
            ),
        )
    }

    /// §2.6, the placement law, and the only place it is written down.
    ///
    /// "Actions and the controls the thumb works are bottom-anchored;
    /// lists and documents start at the top and scroll; everything else
    /// is one block, vertically centred in the space between the app bar
    /// and the actions." `bottom` is what the thumb works and what is
    /// left of the height falls above it; `block` is what is read, and
    /// it is centred in that space on every class. A block taller than
    /// the space gives its height back — a code shrinks, a string takes
    /// a smaller size — and one that still does not fit starts at the
    /// top of a screen that scrolls.
    ///
    /// [`SPACE`] names the space and [`BLOCK`] the block, so a layout
    /// test can put one centre against the other; [`GROUP`] names the
    /// bottom group. The screen's own action row is not part of either:
    /// it is the frame's footer, under both.
    fn place(&self, block: Node, bottom: Option<Node>) -> Node {
        let space = Node::column()
            .id(SPACE)
            .weight(1.0)
            .min_height(0.0)
            .justify(Justify::Center)
            .child(block.id(BLOCK).shrink(1.0).min_height(0.0));
        let body = Node::column().gap(self.body_gap()).child(space);
        match bottom {
            Some(b) => body.child(b.id(GROUP)),
            None => body,
        }
    }

    /// A screen that must not scroll: the body takes the space between
    /// the app bar and the action row, and [`place`](Chrome::place) has
    /// already said where in it everything sits.
    fn fixed(&self, s: Frame<'_>) -> Node {
        let back = self.back(&s);
        self.framed(
            s.dimmed,
            organisms::action_screen_at(
                back,
                self.trailing(s.trailing),
                s.title,
                Body::Fill,
                self.capped(s.body),
                s.footer,
                tokens::action_bottom(self.inset_bottom()),
            ),
        )
    }

    /// §4.13's bottom action row, at the width this screen's pane has.
    fn actions(&self, actions: Vec<Action>) -> Vec<Node> {
        action_row(self.class(), self.pane().width_dp, actions)
    }

    /// The same row for a footer whose one control is not a button.
    fn buttons(&self, buttons: Vec<Node>) -> Vec<Node> {
        buttons_row(self.class(), self.pane().width_dp, buttons)
    }

    /// What the app bar's trailing slot holds: the screen's own control
    /// where it has one — the eye — and otherwise the info button the
    /// host asked for (§4.1, §16.105).
    fn trailing(&self, own: Option<Node>) -> Option<Node> {
        own.or_else(|| self.info.map(components::bar_info))
    }

    /// The gap in dp between the blocks of a screen's body: one grid
    /// step, or the dense list's step on a 268 dp panel.
    fn body_gap(&self) -> f32 {
        if self.class() == SizeClass::Small {
            tokens::GAP_SMALL
        } else {
            tokens::GAP
        }
    }

    /// The back chevron this frame draws: the host's, unless the screen
    /// is a terminal state, which §4.14 gives no way back.
    fn back(&self, s: &Frame<'_>) -> Option<Id> {
        if s.no_back { None } else { self.back }
    }
}

/// The parts of a screen frame that every screen fills in the same way.
struct Frame<'a> {
    title: &'a str,
    trailing: Option<Node>,
    body: Node,
    footer: Vec<Node>,
    dimmed: bool,
    no_back: bool,
    /// Whether the title is read character by character (§16.126).
    mono_title: bool,
}

impl<'a> Frame<'a> {
    /// A frame with a title and a body, and nothing else yet.
    fn new(title: &'a str, body: Node) -> Self {
        Frame {
            title,
            trailing: None,
            body,
            footer: Vec::new(),
            dimmed: false,
            no_back: false,
            mono_title: false,
        }
    }

    /// Titles the screen in the mono face: a fingerprint is eight hex
    /// characters read one by one, not a word (§16.126).
    fn mono_title(mut self) -> Self {
        self.mono_title = true;
        self
    }

    /// The app bar's trailing action: the eye on a secret screen,
    /// nothing else (§4.1).
    fn trailing(mut self, node: Option<Node>) -> Self {
        self.trailing = node;
        self
    }

    /// The body, where the frame was built before it.
    fn body(mut self, body: Node) -> Self {
        self.body = body;
        self
    }

    /// The bottom action row.
    fn footer(mut self, footer: Vec<Node>) -> Self {
        self.footer = footer;
        self
    }

    /// Draws the `wide` sidebar inert: a wizard, a secret, the scanner, a
    /// hold step or a terminal screen owns the pane (§4.1).
    fn dimmed(mut self) -> Self {
        self.dimmed = true;
        self
    }

    /// §4.14 Terminal state: "Result layout with no action and no back
    /// chevron." The sidebar goes inert with it.
    fn terminal(mut self) -> Self {
        self.no_back = true;
        self.dimmed = true;
        self
    }
}

/// One bottom action: a label, whether it is live, and the id it reports.
pub struct Action {
    /// Application id.
    pub id: Id,
    /// The verb on the button.
    pub label: String,
    /// False while the step it confirms is not available; the button
    /// keeps its place and its size either way (§4.13).
    pub enabled: bool,
    /// Whether the button is held rather than tapped (§2.7), which a
    /// screen asks for when what it carries is a danger warning.
    pub hold: bool,
    /// §4.11's reason, two or three words, drawn above the button while
    /// it is dead: the condition of this state the person could change.
    /// A live action's reason is not drawn.
    pub reason: Option<String>,
}

impl Action {
    /// A live action.
    pub fn new(id: Id, label: impl Into<String>) -> Self {
        Action {
            id,
            label: label.into(),
            enabled: true,
            hold: false,
            reason: None,
        }
    }

    /// The same action held rather than tapped: the primary slot takes
    /// [`components::hold`] instead of [`components::primary`].
    pub fn holding(id: Id, label: impl Into<String>) -> Self {
        Action {
            hold: true,
            ..Action::new(id, label)
        }
    }

    /// An action that is live only when `enabled`.
    pub fn when(id: Id, label: impl Into<String>, enabled: bool) -> Self {
        Action {
            enabled,
            ..Action::new(id, label)
        }
    }

    /// Why this action is dead, in two or three words (§4.11). It is
    /// drawn only while the action is dead, and only on a primary.
    pub fn reason(mut self, text: impl Into<String>) -> Self {
        self.reason = Some(text.into());
        self
    }
}

/// §4.13: the bottom action row. The last action is the primary one.
///
/// "Full width on `small` and `mobile`" — and where there are two, "the
/// two share the width equally", so Words' "Show numbers" and Continue
/// are the same size. On `wide` both are their natural width, at least
/// [`tokens::BUTTON_MIN_WIDTH`] and at most [`tokens::BUTTON_MAX_WIDTH`],
/// at the pane's bottom-right with the secondary to the primary's left.
fn action_row(class: SizeClass, width_dp: f32, actions: Vec<Action>) -> Vec<Node> {
    if actions.is_empty() {
        return Vec::new();
    }
    let last = actions.len() - 1;
    // §4.11's reason belongs to the primary alone, and only while it is
    // dead. The button keeps its place and its size (§4.13); the footer
    // is one caption line taller, which the scrolling body absorbs.
    let reason = actions[last]
        .reason
        .clone()
        .filter(|_| !actions[last].enabled);
    let buttons = actions.into_iter().enumerate().map(|(i, a)| {
        if i == last {
            if a.hold {
                components::hold(a.id, a.label, false, a.enabled)
            } else {
                components::primary(a.id, a.label, a.enabled)
            }
        } else {
            components::secondary(a.id, a.label)
        }
    });
    let row = buttons_row(class, width_dp, buttons.collect());
    match reason {
        Some(text) => alloc::vec![
            Node::column().gap(tokens::GAP_SMALL).children(
                core::iter::once(
                    Node::row()
                        .id(REASON)
                        .justify(Justify::Center)
                        .child(components::reason(text)),
                )
                .chain(row),
            )
        ],
        None => row,
    }
}

/// The same row for a footer whose one control is not a button: the
/// hold of §4.13, which "is placed as a primary".
///
/// On the touch classes the buttons are given the width the row divides
/// into rather than a weight, because a weight grows each button from
/// its own label and two labels of different lengths would end up two
/// different sizes.
fn buttons_row(class: SizeClass, width_dp: f32, buttons: Vec<Node>) -> Vec<Node> {
    if buttons.is_empty() {
        return Vec::new();
    }
    let row = if class == SizeClass::Wide {
        let mut row = Node::row().gap(tokens::GAP).justify(Justify::End);
        for b in buttons {
            row = row.child(
                b.min_width(tokens::BUTTON_MIN_WIDTH)
                    .max_width(tokens::BUTTON_MAX_WIDTH),
            );
        }
        row
    } else {
        let n = buttons.len() as f32;
        let each = ((width_dp - 2.0 * tokens::PAD - tokens::GAP * (n - 1.0)) / n).max(0.0);
        let mut row = Node::row().gap(tokens::GAP);
        for b in buttons {
            row = row.child(b.width(each));
        }
        row
    };
    alloc::vec![row]
}

/// §4.1 Pager: "`‹ label ›` in the bottom action slot. For runs of
/// same-kind pages only."
pub struct Pager {
    /// The previous page.
    pub prev: Id,
    /// The next page.
    pub next: Id,
    /// The page's name: "Receive 2", "Words 1 to 6".
    pub label: String,
    /// The first page of the run: the left arrow is dead.
    pub at_start: bool,
    /// The last page of the run: the right arrow is dead.
    pub at_end: bool,
}

impl Pager {
    /// The pager row. `mono` sets the page's name in the mono face,
    /// where that name is read character by character (§16.126).
    pub(super) fn node(self, class: SizeClass, mono: bool) -> Node {
        let row = if mono {
            components::pager_mono
        } else {
            components::pager
        };
        row(
            class,
            self.prev,
            self.next,
            self.label,
            self.at_start,
            self.at_end,
        )
    }
}

/// §4.4 Key context: "A row at the top of a body: the label 'Key', the
/// fingerprint chip(s), a caret."
///
/// Only on the Sign review and in Explore. Never on an Entry screen:
/// typing words or a passphrase is not about a key yet (§4.3).
pub struct KeyContext {
    /// Application id, where the row opens something; `None` where it
    /// only names the keys, and then the row carries no chevron.
    pub id: Option<Id>,
    /// The word in front of the chips, which §4.4 makes "Key".
    pub label: String,
    /// The fingerprints this screen acts with.
    pub keys: Vec<String>,
}

impl KeyContext {
    /// The row: on a surface among menu rows, flat inside a Record's
    /// table (§4.11: every row there looks like a table row).
    fn node(&self, class: SizeClass, flat: bool) -> Node {
        if flat {
            components::flat_key_context(class, self.id, self.label.clone(), &self.keys)
        } else {
            components::key_context(class, self.id, self.label.clone(), &self.keys)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::rc::Rc;
    use alloc::vec;

    use osk_codec::qr::{Ecc, Payload, encode};

    use crate::components::{Entries, Network, Tile};
    use crate::geom::{Rect, Scale};
    use crate::layout::{Layout, LayoutCtx, solve};
    use crate::widgets::keyboard::KeyboardKind;
    use crate::widgets::{HitTarget, Icon, Tone, WarningLevel};

    /// The four sizes `just snapshots` renders: the smallest supported
    /// panel, the 2.8" reference panel, a phone, and the desktop window.
    const REFERENCE: [(Rect, u16, SizeClass); 4] = [
        (Rect::new(0, 0, 240, 320), 143, SizeClass::Small),
        (Rect::new(0, 0, 480, 640), 286, SizeClass::Small),
        (Rect::new(0, 0, 1080, 2340), 420, SizeClass::Mobile),
        (Rect::new(0, 0, 960, 640), 160, SizeClass::Wide),
    ];

    const ADDRESS: &str = "bc1pxwww0ct9ue7e8tdnlmug5m2tamfn7q06sahstg39ys4c9f3340qqxrdu9k";
    const XPUB: &str = "xpub6CUGRUonZSQ4TWtTMmzXdrXDtypWKiKrhko4egpiMZbpiaQL2jkwSB1icqYh2cfDfVxdx4df189oLKnC5fSwqPfgyP3hooxujYzAu3fDVmz";
    const FINGERPRINT: &str = "73c5da0a";
    const SEED: [&str; 24] = [
        "abandon", "ability", "able", "about", "above", "absent", "absorb", "abstract", "absurd",
        "abuse", "access", "accident", "account", "accuse", "achieve", "acid", "acoustic",
        "acquire", "across", "act", "action", "actor", "actress", "art",
    ];
    const ROLLS: [&str; 8] = ["3", "6", "1", "1", "4", "2", "5", "3"];
    /// The length of the secret the Entry samples are typing.
    const WORDS_IN_A_SECRET: usize = 12;

    /// The ids the samples use. Nothing here collides with the
    /// rectangles the screens reserve at the top of the id range.
    const A: Id = Id(1);
    const B: Id = Id(2);
    const C: Id = Id(3);

    fn ctx_of(area: Rect, dpi: u16, class: SizeClass) -> (LayoutCtx, Metrics) {
        let ctx = LayoutCtx::new(Scale::for_class(dpi, class), class);
        let factor = ctx.scale.factor();
        let m = Metrics::new(area.w as f32 / factor, area.h as f32 / factor, class);
        (ctx, m)
    }

    /// §4.1: "there is no Home row, Keys is the first area."
    fn nav() -> Vec<SidebarItem> {
        [
            "Scan", "Keys", "Sign", "Verify", "Explore", "Learn", "Settings",
        ]
        .iter()
        .enumerate()
        .map(|(i, label)| SidebarItem {
            id: Some(Id(90 + i as u32)),
            icon: Icon::Keys,
            label: String::from(*label),
            selected: i == 1,
        })
        .collect()
    }

    fn chrome<'a>(m: &'a Metrics, nav: &'a [SidebarItem]) -> Chrome<'a> {
        Chrome {
            m,
            nav,
            back: Some(A),
            dimmed: false,
            info: None,
        }
    }

    fn code() -> Rc<osk_codec::qr::QrMatrix> {
        Rc::new(encode(Payload::Bytes(ADDRESS.as_bytes()), Ecc::Low).expect("fits"))
    }

    fn words_of(n: usize) -> Vec<String> {
        SEED.iter().take(n).map(|w| String::from(*w)).collect()
    }

    fn rolls() -> Vec<String> {
        ROLLS.iter().map(|r| String::from(*r)).collect()
    }

    fn sign_rows() -> Vec<components::Record> {
        vec![
            components::Record::mono("Amount", "60 000 sats"),
            components::Record::mono("Fee", "1 000 sats"),
            components::Record::mono("Rate", "7.1 sat/vB"),
            components::Record::reference(Id(30), "To", ADDRESS),
        ]
    }

    fn key_context() -> KeyContext {
        KeyContext {
            id: Some(B),
            label: String::from("Key"),
            keys: vec![String::from(FINGERPRINT)],
        }
    }

    /// Every screen with the content a flow would give it, named, and
    /// whether it is one of the §5 screens allowed to scroll.
    fn every_screen(c: &Chrome<'_>) -> Vec<Sample> {
        let class = c.class();
        let room = crate::components::secrets::panel_room(c.column().width_dp);
        let width = crate::components::secrets::WordWidth::LATIN;
        let pages = words::pages(class, room, width, false, false, &words_of(12));
        let (first, page) = pages.first().cloned().expect("a page");
        vec![
            Sample::list("Hub", sample_hub(c)).scrolls().bottom(Id(23)),
            Sample::list("Menu", sample_menu(c)).scrolls(),
            Sample::block("Choice", sample_choice(c))
                .scrolls()
                .bottom(Id(12)),
            // Entry's bottom group is the keyboard, which is full-bleed
            // and reaches the bottom edge itself (§4.3).
            Sample::block("Entry", sample_entry(c)),
            Sample::block(
                "Pad",
                sample_pad(
                    c,
                    PadKind::Pin {
                        scramble: None,
                        done: true,
                    },
                    false,
                ),
            )
            .bottom(GROUP),
            Sample::block("Words", sample_words(c, first, &page, false, true)).bottom(Id(15)),
            Sample::block("Secret", sample_secret(c)),
            Sample::block("Compare", sample_compare(c)).bottom(Id(11)),
            Sample::list("Addresses", sample_addresses(c)).scrolls(),
            Sample::block("Address", sample_address(c)),
            Sample::block("QR", sample_qr(c, false)).bottom(GROUP),
            Sample::block("Record", sample_record(c)).bottom(Id(11)),
            Sample::block("Result", sample_result(c, true)).bottom(Id(11)),
            Sample::block("Hold", sample_hold(c)).bottom(C),
            Sample::block("Scanner", sample_scanner(c)).bottom(GROUP),
            Sample::list("Document", sample_document(c)).scrolls(),
        ]
    }

    /// One of the sixteen screens with the content a flow would give it:
    /// its name, how §2.6 places it, whether §5 lets it scroll, and the
    /// bottom-most control the thumb works, where it has one.
    struct Sample {
        name: &'static str,
        node: Node,
        /// One of the §5 screens that hold content of unknown length.
        scrolls: bool,
        /// The last thing the thumb works: the action row's primary, or
        /// the bottom group where the screen has no action.
        bottom: Option<Id>,
    }

    impl Sample {
        fn block(name: &'static str, node: Node) -> Self {
            Sample {
                name,
                node,
                scrolls: false,
                bottom: None,
            }
        }

        fn list(name: &'static str, node: Node) -> Self {
            Sample {
                ..Sample::block(name, node)
            }
        }

        fn scrolls(mut self) -> Self {
            self.scrolls = true;
            self
        }

        fn bottom(mut self, id: Id) -> Self {
            self.bottom = Some(id);
            self
        }
    }

    /// The three areas §4.1 puts in Home's band.
    fn hub_tiles() -> Vec<Tile> {
        ["Wallets", "Keys", "Scan", "Tools", "Learn", "Settings"]
            .iter()
            .enumerate()
            .map(|(i, label)| Tile {
                id: Id(60 + i as u32),
                icon: Icon::Tools,
                label: String::from(*label),
                enabled: true,
                badge: None,
            })
            .collect()
    }

    fn sample_hub(c: &Chrome<'_>) -> Node {
        let status = Status {
            tier: String::from("Tier C"),
            tier_id: Some(C),
            network: Network::Signet,
            lock: Some(Id(11)),
            session: None,
            notice: None,
        };
        launcher(c, &status, hub_tiles())
    }

    fn sample_menu(c: &Chrome<'_>) -> Node {
        menu(
            c,
            "Key 73c5da0a",
            Some(key_context()),
            vec![
                Row::Menu {
                    id: C,
                    icon: Some(Icon::Shield),
                    label: String::from("Backup"),
                    value: Some(String::from("not verified")),
                    tone: Tone::Caution,
                },
                Row::Value {
                    id: Id(11),
                    label: String::from("Script type"),
                    value: String::from("SegWit"),
                    tone: Tone::Text,
                },
                Row::Path {
                    id: Id(17),
                    label: String::from("Path"),
                    value: String::from("m/84h/0h/0h"),
                },
                Row::Reference {
                    id: Id(12),
                    label: String::from("Account key"),
                    value: String::from(XPUB),
                },
                Row::Secret {
                    id: Id(13),
                    label: String::from("Master private key"),
                    value: None,
                },
                Row::Dimmed {
                    icon: Some(Icon::Scan),
                    label: String::from("Scan a QR"),
                    reason: Some(String::from("no camera")),
                },
            ],
            vec![],
        )
    }

    fn sample_choice(c: &Chrome<'_>) -> Node {
        choice(
            c,
            "Word count",
            vec![
                Item::chosen(C, "12 words", true),
                Item::new(Id(11), "24 words"),
                Item::dimmed("From a card", "no reader"),
            ],
            Action::new(Id(12), "Continue"),
        )
    }

    fn sample_entry(c: &Chrome<'_>) -> Node {
        sample_entry_with(c, 7)
    }

    /// The Entry screen part-way through a twelve-word secret, with
    /// `accepted` words already in (§4.3).
    fn sample_entry_with(c: &Chrome<'_>, accepted: usize) -> Node {
        let words = words_of(accepted);
        entry(
            c,
            Entry {
                title: "Word 7 of 12",
                value: String::from("ab"),
                mono: false,
                words: (accepted > 0).then(|| WordsSoFar {
                    panel: Id(20),
                    words: &words,
                    total: WORDS_IN_A_SECRET,
                    revealed: false,
                    width: crate::components::secrets::WordWidth::LATIN,
                }),
                eye: (accepted > 0).then_some((Id(21), None)),
                above: Above::Nothing,
                candidates: Some(Candidates {
                    first: C,
                    second: Id(11),
                    words: vec![
                        String::from("abandon"),
                        String::from("ability"),
                        String::from("able"),
                    ],
                    accepting: false,
                    more: false,
                    one_char: false,
                    selected: None,
                }),
                keyboard: (Id(12), KeyboardKind::Bip39),
                enabled: crate::widgets::keyboard::ALL_KEYS,
                error: None,
            },
        )
    }

    fn sample_pad(c: &Chrome<'_>, kind: PadKind, action: bool) -> Node {
        let dice = !matches!(kind, PadKind::Pin { .. });
        let values = rolls();
        pad(
            c,
            Pad {
                words: None,
                eye: None,
                title: String::from("Enter your PIN"),
                id: C,
                kind,
                field: if dice { Field::Nothing } else { Field::Dots(2) },
                progress: dice.then(|| (String::from("23 of 50 · 59 bits"), 0.5)),
                entries: dice.then(|| Entries {
                    values: &values,
                    revealed: false,
                    flash: true,
                }),
                caption: None,
                error: None,
                action: action.then(|| Action::new(Id(11), "Continue")),
            },
        )
    }

    fn sample_words(
        c: &Chrome<'_>,
        first: usize,
        page: &[String],
        revealed: bool,
        numbers: bool,
    ) -> Node {
        let last = first + page.len() - 1;
        let places: Vec<u16> = (0..page.len() as u16).collect();
        words(
            c,
            Words {
                steel: false,
                extra: None,
                title: "Your words",
                words: page,
                first,
                indices: numbers.then_some(&places),
                width: crate::components::secrets::WordWidth::LATIN,
                revealed,
                panel: C,
                eye: Some(Id(11)),
                remaining: revealed.then_some(0.5),
                pager: (words::pages(
                    c.class(),
                    crate::components::secrets::panel_room(c.column().width_dp),
                    crate::components::secrets::WordWidth::LATIN,
                    numbers,
                    false,
                    &words_of(12),
                )
                .len()
                    > 1)
                .then(|| Pager {
                    prev: Id(12),
                    next: Id(13),
                    label: alloc::format!("Words {first} to {last}"),
                    at_start: true,
                    at_end: false,
                }),
                numbers_action: Some(Action::new(Id(14), "Numbers")),
                action: Action::new(Id(15), "Continue"),
            },
        )
    }

    fn sample_secret(c: &Chrome<'_>) -> Node {
        secret(
            c,
            Secret {
                pager: None,
                rows: Vec::new(),
                action: None,
                secondary: None,
                title: "Master private key",
                value: Value::Text(XPUB),
                revealed: true,
                panel: C,
                eye: Some(Id(11)),
                remaining: Some(0.5),
            },
        )
    }

    fn sample_compare(c: &Chrome<'_>) -> Node {
        compare(
            c,
            Compare {
                title: "Signature",
                label: Some("#0 · 73c5da0a"),
                value: XPUB,
                id: C,
                done: Action::new(Id(11), "Done"),
                key: None,
                copy: None,
                caption: None,
            },
        )
    }

    fn sample_addresses(c: &Chrome<'_>) -> Node {
        addresses(
            c,
            Addresses {
                title: "Addresses",
                script: (Some(C), "Script type", "Taproot"),
                chain: (Id(11), "Receive", Id(12), "Change", false),
                rows: (0..4)
                    .map(|i| AddressRow {
                        id: Id(20 + i),
                        label: alloc::format!("Receive {}", i + 1),
                        address: String::from(ADDRESS),
                    })
                    .collect(),
                more: Some((Id(30), String::from("More"))),
            },
        )
    }

    fn sample_address(c: &Chrome<'_>) -> Node {
        address(
            c,
            Address {
                format: None,
                title: "Receive 2",
                label: "Receive 2",
                address: (Id(12), ADDRESS),
                code: code(),
                copy: None,
                save: None,
                caption: None,
            },
        )
    }

    fn sample_qr(c: &Chrome<'_>, animated: bool) -> Node {
        qr(
            c,
            Qr {
                title: "Wallet export",
                matrix: code(),
                label: "Wallet export",
                toggle: Some((C, "Animated")),
                animated,
                forced: animated.then(|| String::from("too dense")),
                progress: animated.then(|| (0.5, String::from("2 of 5"))),
                save: None,
                caption: None,
            },
        )
    }

    fn sample_record(c: &Chrome<'_>) -> Node {
        record(
            c,
            Record {
                pager: None,
                title: "Output 1 of 2",
                key: None,
                network: Network::Signet,
                rows: sign_rows(),
                warnings: vec![(
                    WarningLevel::Caution,
                    String::from("Fee"),
                    String::from("12 % of the amount"),
                )],
                action: Some(Action::new(Id(11), "Continue")),
            },
        )
    }

    fn sample_result(c: &Chrome<'_>, passed: bool) -> Node {
        result(
            c,
            Result {
                caption: None,
                title: "Sign",
                icon: if passed { Icon::Check } else { Icon::Error },
                tone: if passed { Tone::Success } else { Tone::Danger },
                result: if passed { "Signed" } else { "Not found" },
                rows: vec![components::Record::mono("Key", FINGERPRINT)],
                actions: if passed {
                    vec![Action::new(C, "Show as QR"), Action::new(Id(11), "Done")]
                } else {
                    Vec::new()
                },
            },
        )
    }

    fn sample_hold(c: &Chrome<'_>) -> Node {
        hold(
            c,
            Hold {
                warnings: Vec::new(),
                title: "Sign",
                rows: sign_rows(),
                sign_with: Some((
                    String::from("Sign with"),
                    vec![(Id(21), String::from(FINGERPRINT), true)],
                )),
                then_with: None,
                id: C,
                label: "Hold to sign",
                danger: false,
                enabled: true,
                secondary: None,
            },
        )
    }

    fn sample_scanner(c: &Chrome<'_>) -> Node {
        scanner(
            c,
            Scanner {
                title: "Scan a transaction",
                state: "3 of 8",
                parts: Some(0.5),
                ways: vec![crate::organisms::TileSpec {
                    id: C,
                    icon: Icon::File,
                    label: String::from("Read a file"),
                    enabled: true,
                    badge: None,
                }],
                preview: None,
                action: None,
            },
        )
    }

    fn sample_document(c: &Chrome<'_>) -> Node {
        document(
            c,
            "Tiers",
            vec![Section {
                id: None,
                heading: String::from("Tier C"),
                paragraphs: vec![String::from(
                    "A tier states what the build was compiled from and what the host can \
                     hold against it.",
                )],
            }],
        )
    }

    /// Everything the layout placed outside the region it is drawn in,
    /// leaving out what a scroll region can be scrolled to.
    fn outside(l: &Layout) -> Vec<Rect> {
        let scrolls: Vec<Rect> = l
            .items
            .iter()
            .filter(|p| p.scroll.is_some())
            .map(|p| p.rect)
            .collect();
        l.items
            .iter()
            .filter(|p| !p.rect.is_empty())
            .filter(|p| {
                p.rect.bottom() > p.clip.bottom()
                    || p.rect.right() > p.clip.right()
                    || p.rect.x < p.clip.x
                    || p.rect.y < p.clip.y
            })
            .filter(|p| {
                !scrolls
                    .iter()
                    .any(|s| p.rect.x >= s.x && p.rect.right() <= s.right())
            })
            .map(|p| p.rect)
            .collect()
    }

    /// Everything the layout placed below the bottom of the region it is
    /// drawn in, a scroll region's own content included: what a person
    /// would have to scroll to reach.
    fn below_the_fold(l: &Layout) -> Vec<Rect> {
        l.items
            .iter()
            .filter(|p| !p.rect.is_empty() && p.rect.bottom() > p.clip.bottom())
            .map(|p| p.rect)
            .collect()
    }

    /// §4.3: "On `mobile` and `wide` the space above the entry group
    /// holds, for word entry, a masked words panel of the words accepted
    /// so far, centred in that space; on `small` the screen shows the
    /// word being typed and nothing else."
    ///
    /// The panel keeps a row for every word of the secret, so it is the
    /// same rectangle at the seventh word as at the twelfth and nothing
    /// moves as the entry goes on. What it draws in those rows —
    /// bullets, then empty rows — is
    /// [`components::secrets`]' own test.
    #[test]
    fn the_entry_panel_is_the_whole_secret_and_off_the_small_panel() {
        for (area, dpi, class) in REFERENCE {
            let (ctx, m) = ctx_of(area, dpi, class);
            let items = nav();
            let c = chrome(&m, &items);
            let panel = |accepted: usize| {
                let l = solve(&sample_entry_with(&c, accepted), area, &ctx);
                l.items
                    .iter()
                    .find(|p| p.hit == Some(HitTarget::Reveal(Id(20))))
                    .map(|p| p.rect)
            };
            if class == SizeClass::Small {
                assert!(
                    panel(7).is_none(),
                    "a 268 dp panel shows the words so far at {}x{}",
                    area.w,
                    area.h
                );
                continue;
            }
            let part = panel(7).expect("the panel");
            let whole = panel(WORDS_IN_A_SECRET).expect("the panel");
            assert_eq!(
                part, whole,
                "the panel moves between the seventh word and the twelfth at {}x{}",
                area.w, area.h
            );
        }
    }

    /// The Simplified list's `yi4`, whose twenty characters are the
    /// largest tone group either Chinese list has (§16.44).
    const YI4: &str = "一以已亿义艺忆议亦异译易益意毅溢邑翼裔逸";

    fn sample_hanzi_entry(c: &Chrome<'_>, chars: &str) -> Node {
        entry(
            c,
            Entry {
                title: "Word 7 of 12",
                value: String::from("yi4"),
                mono: false,
                words: None,
                eye: None,
                above: Above::Nothing,
                candidates: Some(Candidates {
                    first: C,
                    second: Id(11),
                    words: chars.chars().map(String::from).collect(),
                    accepting: false,
                    more: false,
                    one_char: true,
                    selected: None,
                }),
                keyboard: (Id(12), KeyboardKind::Pinyin),
                enabled: crate::widgets::keyboard::ALL_KEYS,
                error: None,
            },
        )
    }

    /// §4.3: a list whose words are one character shows ten a row and two
    /// rows, and a cell is exactly as wide as a key of the keyboard under
    /// it — so a character is the target a key is, and twenty of them are
    /// on the screen at once.
    #[test]
    fn a_character_is_a_key_wide_and_the_strip_is_two_rows_of_ten() {
        for (area, dpi, class) in REFERENCE {
            let (ctx, m) = ctx_of(area, dpi, class);
            let items = nav();
            let c = chrome(&m, &items);
            let l = solve(&sample_hanzi_entry(&c, YI4), area, &ctx);
            let rows: Vec<Rect> = [C, Id(11)]
                .iter()
                .map(|id| l.rect(*id).expect("a candidate row"))
                .collect();
            assert_eq!(rows[1].y, rows[0].bottom(), "at {}x{}", area.w, area.h);
            assert_eq!(rows[0].w, rows[1].w, "at {}x{}", area.w, area.h);
            let cells = |id: Id| match l.placed(id).and_then(|p| p.hit) {
                Some(HitTarget::Candidates { count, cells, .. }) => (count, cells),
                other => panic!("{other:?} at {}x{}", area.w, area.h),
            };
            assert_eq!(cells(C), (10, 10), "at {}x{}", area.w, area.h);
            assert_eq!(cells(Id(11)), (10, 10), "at {}x{}", area.w, area.h);
            let keys = crate::widgets::keyboard::keys(
                KeyboardKind::Pinyin,
                l.rect(Id(12)).expect("the keyboard"),
                &ctx,
                crate::widgets::keyboard::ALL_KEYS,
                None,
                Default::default(),
            );
            let key = keys[0].rect.w;
            let cell = rows[0].w / 10;
            assert!(
                (cell - key).abs() <= 1,
                "a cell is {cell} px and a key {key} px at {}x{}",
                area.w,
                area.h
            );
        }
    }

    /// §4.3: the strip keeps its rectangle as a reading is typed, so the
    /// keys under it do not move when the characters left thin out.
    #[test]
    fn a_thinning_group_leaves_the_strip_and_the_keys_where_they_were() {
        for (area, dpi, class) in REFERENCE {
            let (ctx, m) = ctx_of(area, dpi, class);
            let items = nav();
            let c = chrome(&m, &items);
            let geometry = |chars: &str| {
                let l = solve(&sample_hanzi_entry(&c, chars), area, &ctx);
                (
                    l.rect(C).expect("a candidate row"),
                    l.rect(Id(11)).expect("a second row"),
                    l.rect(Id(12)).expect("the keyboard"),
                )
            };
            let full = geometry(YI4);
            for chars in ["", "一", "一以已亿义艺忆议亦异译"] {
                assert_eq!(geometry(chars), full, "at {}x{}", area.w, area.h);
            }
        }
    }

    /// §2.6 and §5: a screen either fits the panel it was designed for or
    /// it is one of the ones that hold a list of unknown length — Menu,
    /// Choice, Addresses, Document — and scrolls.
    #[test]
    fn every_screen_fits_or_scrolls() {
        for (area, dpi, class) in REFERENCE {
            let (ctx, m) = ctx_of(area, dpi, class);
            let items = nav();
            let c = chrome(&m, &items);
            for s in every_screen(&c) {
                let name = s.name;
                let l = solve(&s.node, area, &ctx);
                assert_eq!(
                    outside(&l),
                    Vec::new(),
                    "{name} is drawn outside its region at {}x{}",
                    area.w,
                    area.h
                );
                if !s.scrolls {
                    assert_eq!(
                        below_the_fold(&l),
                        Vec::new(),
                        "{name} puts content below the fold at {}x{}",
                        area.w,
                        area.h
                    );
                }
            }
        }
    }

    /// §2.6 "Small first": the screens that state a result are one
    /// screenful on the 268 × 358 dp panel, with nothing below the fold.
    #[test]
    fn the_result_screens_are_one_screenful_on_small() {
        let (area, dpi, class) = REFERENCE[1];
        let (ctx, m) = ctx_of(area, dpi, class);
        let items = nav();
        let c = chrome(&m, &items);
        let screens: [(&str, Node); 5] = [
            ("Address", sample_address(&c)),
            ("QR", sample_qr(&c, false)),
            ("Record", sample_record(&c)),
            ("Result", sample_result(&c, true)),
            ("Hold", sample_hold(&c)),
        ];
        for (name, screen) in screens {
            let l = solve(&screen, area, &ctx);
            assert_eq!(
                below_the_fold(&l),
                Vec::new(),
                "{name} runs below the fold on small"
            );
            assert_eq!(outside(&l), Vec::new(), "{name} on small");
        }
    }

    /// §4.11 and §2.6: the table, its badge and its warnings are one
    /// block centred in the space above the action, whatever the table's
    /// length; a table longer than the space starts at the top and
    /// scrolls, so the way on stays where it is.
    #[test]
    fn a_long_record_starts_at_the_top_and_leaves_the_action_alone() {
        let (area, dpi, class) = REFERENCE[2];
        let (ctx, m) = ctx_of(area, dpi, class);
        let items = nav();
        let c = chrome(&m, &items);
        // Every row of a Sign review, a key context, a network badge and
        // a warning card together outgrow the space.
        let long = record(
            &c,
            Record {
                pager: None,
                title: "Sign",
                key: Some(key_context()),
                network: Network::Signet,
                rows: (0..20)
                    .map(|i| components::Record::reference(Id(40 + i), "To", ADDRESS))
                    .collect(),
                warnings: vec![(
                    WarningLevel::Caution,
                    String::from("Fee"),
                    String::from("12 % of the amount"),
                )],
                action: Some(Action::new(Id(11), "Continue")),
            },
        );
        let l = solve(&long, area, &ctx);
        let block = l.rect(BLOCK).expect("the table");
        let region = l.rect(SCROLL).expect("the region");
        assert!(
            block.h > region.h,
            "the long record fits the space: {} px of {}",
            block.h,
            region.h
        );
        assert_eq!(block.y, region.y, "a long record does not start at the top");
        let action = l.rect(Id(11)).expect("Continue");
        assert_eq!(
            action.bottom(),
            area.bottom() - ctx.px(tokens::action_bottom(ctx.inset_bottom_dp)),
            "the action moved with the table"
        );
    }
    /// §4.11's reason on a dead primary: a person who loaded a
    /// transaction before a key sees why Continue does nothing, in two
    /// or three words above the button, and the button does not move.
    #[test]
    fn a_dead_primary_says_why_and_a_live_one_says_nothing() {
        const REASON_TEXT: &str = "No key loaded";
        for (area, dpi, class) in REFERENCE {
            let (ctx, m) = ctx_of(area, dpi, class);
            let items = nav();
            let c = chrome(&m, &items);
            let screen = |action: Action| {
                record(
                    &c,
                    Record {
                        pager: None,
                        title: "Sign",
                        key: None,
                        network: Network::Signet,
                        rows: sign_rows(),
                        warnings: Vec::new(),
                        action: Some(action),
                    },
                )
            };
            let dead = screen(Action::when(B, "Continue", false));
            let said = screen(Action::when(B, "Continue", false).reason(REASON_TEXT));
            let live = screen(Action::when(B, "Continue", true).reason(REASON_TEXT));

            assert!(
                texts_of(&said).iter().any(|t| t == REASON_TEXT),
                "{class:?}"
            );
            assert!(!texts_of(&dead).iter().any(|t| t == REASON_TEXT));
            assert!(
                !texts_of(&live).iter().any(|t| t == REASON_TEXT),
                "a live action's reason is not drawn"
            );

            let said = solve(&said, area, &ctx);
            let caption = said.rect(REASON).expect("the caption");
            let button = said.rect(B).expect("Continue");
            assert!(
                caption.bottom() <= button.y,
                "the caption is not above the button on {class:?}"
            );
            assert_eq!(
                solve(&dead, area, &ctx).rect(B),
                Some(button),
                "the button moved or changed size on {class:?}"
            );
            assert_eq!(
                outside(&said),
                Vec::new(),
                "the reason pushed something off"
            );
        }
    }

    /// Every string a node tree draws, in tree order.
    fn texts_of(node: &Node) -> Vec<String> {
        fn walk(node: &Node, out: &mut Vec<String>) {
            if let Some(crate::widgets::Widget::Text { text, .. }) = node.as_widget() {
                out.push(text.clone());
            }
            for child in &node.children {
                walk(child, out);
            }
        }
        let mut out = Vec::new();
        walk(node, &mut out);
        out
    }
}
