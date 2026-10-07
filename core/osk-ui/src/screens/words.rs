//! §5 Words: "the words panel, pager when it pages, 'Show numbers' +
//! primary as an equal pair, eye". Used by Backup › words, Create ›
//! words, Load › check your words, the quiz's reveal and Explore ›
//! words.

use alloc::string::String;
use alloc::vec::Vec;

use crate::components::{self, Secret, secrets::WordWidth};
use crate::geom::SizeClass;
use crate::layout::{Id, Node};
use crate::tokens;

use super::{Action, Chrome, Frame, Pager};

/// What a [`words`] screen shows.
pub struct Words<'a> {
    /// The app bar's title.
    pub title: &'a str,
    /// This page's words. [`pages`] splits a 12- or 24-word secret into
    /// the pages a panel holds on this class.
    pub words: &'a [String],
    /// The number of `words[0]`, so a paged panel keeps counting.
    pub first: usize,
    /// Each word's place in the wordlist, 0-based, when "Numbers" is
    /// on. It masks with the word it belongs to (§4.10).
    pub indices: Option<&'a [u16]>,
    /// How wide a word of this wordlist is, which is what the panel pads
    /// its rows to and how many columns it draws.
    pub width: WordWidth,
    /// Whether the words are shown. A finger on the panel or the app
    /// bar's eye sets this; the panel's rectangle is the same either way
    /// (§4.10).
    pub revealed: bool,
    /// The panel's application id, which is also its touch surface.
    pub panel: Id,
    /// The app bar's eye, which shows the words for 30 s (§4.10).
    pub eye: Option<Id>,
    /// The share of the eye's reveal still to run, which draws the eye
    /// as a ring that empties clockwise. `None` while the eye is not
    /// running: there is no countdown text anywhere (§4.14).
    pub remaining: Option<f32>,
    /// The pager under the panel, on a class that pages.
    pub pager: Option<Pager>,
    /// Whether the rows are steel rows: the wordlist number, the four
    /// punched letters and the word (`docs/PLANNING.md` §8.2 item 6).
    pub steel: bool,
    /// "Show numbers", the secondary action. `None` on a panel whose
    /// rows already carry the numbers: the steel rows.
    pub numbers_action: Option<Action>,
    /// An action between "Show numbers" and the primary, where a class
    /// has one: "Print template" on `wide`.
    pub extra: Option<Action>,
    /// The primary action.
    pub action: Action,
}

/// Words on one page of the panel: the rows the class holds, in as many
/// columns as `room_dp` has width for.
pub fn per_page(
    class: SizeClass,
    room_dp: f32,
    width: WordWidth,
    indexed: bool,
    steel: bool,
) -> usize {
    components::secrets::words_per_page(
        class,
        components::secrets::word_columns(class, room_dp, width, indexed, steel),
    )
}

/// The pages a panel shows a secret in on this class: the first word's
/// number and the words of each page (§4.10).
pub fn pages(
    class: SizeClass,
    room_dp: f32,
    width: WordWidth,
    indexed: bool,
    steel: bool,
    words: &[String],
) -> Vec<(usize, Vec<String>)> {
    components::secrets::word_pages(
        class,
        components::secrets::word_columns(class, room_dp, width, indexed, steel),
        words,
    )
}

/// §5 Words: "the words panel, pager when it pages, 'Show numbers' +
/// primary as an equal pair, eye."
///
/// §4.10: "An orange-outlined panel sized to its content ... six per
/// page at 24 dp rows on `small`; twelve at 40 dp rows and mono 20 on
/// `mobile` and `wide`, in two columns ... No caption inside it, in
/// either state. Hold anywhere on it to show; release masks. The eye in
/// the app bar shows it for 30 s; while it does, the eye is drawn as a
/// ring that empties clockwise." The panel keeps one rectangle masked and
/// revealed and with the wordlist numbers on and off, so nothing moves
/// under the finger that reveals it; the pager sits under it on a class
/// that pages, and the two actions share the width equally (§4.13).
///
/// §2.6 places the panel and its pager as one block, centred in the
/// space between the app bar and the two actions, on every class: a
/// secret is read, not touched, and the pager belongs to the panel it
/// pages.
///
/// Made of [`components::secret_panel`] (§4.10),
/// [`components::pager`] (§4.1), [`components::bar_eye`] (§4.1, §4.10)
/// and [`components::primary`] / [`components::secondary`] (§4.13).
pub fn words(c: &Chrome, w: Words<'_>) -> Node {
    let class = c.class();
    let panel = components::secret_panel(
        class,
        w.panel,
        Secret::Words {
            words: w.words,
            first: w.first,
            indices: w.indices,
            width: w.width,
            steel: w.steel,
        },
        w.revealed,
        // The panel is drawn inside the column cap rather than across
        // the pane (§3), so that is the width its columns are worked out
        // against.
        components::secrets::panel_room(c.column().width_dp),
    );
    let mut block = Node::column().gap(tokens::GAP).child(panel);
    if let Some(p) = w.pager {
        block = block.child(p.node(class, false));
    }
    let trailing = w.eye.map(|id| components::bar_eye(id, w.remaining));
    let mut actions = Vec::new();
    if let Some(numbers) = w.numbers_action {
        actions.push(numbers);
    }
    if let Some(extra) = w.extra {
        actions.push(extra);
    }
    actions.push(w.action);
    c.fixed(
        Frame::new(w.title, c.place(block, None))
            .trailing(trailing)
            .footer(c.actions(actions))
            .dimmed(),
    )
}
