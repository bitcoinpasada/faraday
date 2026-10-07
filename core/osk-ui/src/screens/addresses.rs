//! §5 Addresses: "script type value row, Receive | Change pair, then one
//! reference row per address". Used by Addresses and Explore ›
//! addresses.

use alloc::string::String;
use alloc::vec::Vec;

use crate::components;
use crate::layout::{Id, Node};
use crate::tokens;
use crate::widgets::{Icon, Tone};

use super::{Chrome, Frame};

/// One address in the list: its name and the address itself (§4.5).
pub struct Row {
    /// Application id: the whole row opens the [`super::address`]
    /// screen for it.
    pub id: Id,
    /// What the address is: "Receive 2".
    pub label: String,
    /// The address; the row shows its head and tail.
    pub address: String,
}

/// What an [`addresses`] screen shows.
pub struct Addresses<'a> {
    /// The app bar's title.
    pub title: &'a str,
    /// The script type: its label and value, and the id of the Choice
    /// it opens (§4.6). `None` where the script type is a fact of what
    /// is being listed and no choice of this person's changes it, which
    /// §4.12 draws with no chevron and no hit target.
    pub script: (Option<Id>, &'a str, &'a str),
    /// The two chains as a pair of equal buttons: the two ids, the two
    /// words, and whether the right one is the chosen one (§4.2).
    pub chain: (Id, &'a str, Id, &'a str, bool),
    /// One reference row per address.
    pub rows: Vec<Row>,
    /// The row under the list that extends it: its id and its label.
    /// `None` where the run has no more addresses to show.
    pub more: Option<(Id, String)>,
}

/// §5 Addresses: "script type value row, Receive | Change pair, then one
/// reference row per address ('Receive 2' above, the elided address
/// below)."
///
/// The list is the screen: §4.5 makes every long string a reference row
/// wherever it appears inside another screen, and a run of addresses is
/// a list of unknown length, so it scrolls rather than paging. Tapping a
/// row opens [`super::address`], which is the one address with its QR.
///
/// Made of [`components::script_type_row`] (§4.6),
/// [`components::pair`] (§4.2) and [`components::reference_row`] (§4.5).
pub fn addresses(c: &Chrome, a: Addresses<'_>) -> Node {
    let class = c.class();
    let (left_id, left, right_id, right, right_chosen) = a.chain;
    let script = match a.script.0 {
        Some(id) => components::script_type_row(class, id, a.script.1, a.script.2),
        None => components::fact_row(class, a.script.1, a.script.2, false, Tone::Text),
    };
    let mut body = Node::column()
        .gap(tokens::menu_row_gap(class))
        .child(script)
        .child(components::pair(
            (left_id, left),
            (right_id, right),
            right_chosen,
        ));
    for row in a.rows {
        body = body.child(components::reference_row(
            class,
            row.id,
            row.label,
            &row.address,
        ));
    }
    // §4.1 forbids a pager between two views of one thing, and a run of
    // addresses has no end: the list grows from its own last row.
    if let Some((id, label)) = a.more {
        body = body.child(components::menu_row(
            class,
            id,
            Some(Icon::Ellipsis),
            label,
            None,
            Tone::Text,
        ));
    }
    c.scrolling(Frame::new(a.title, body))
}
