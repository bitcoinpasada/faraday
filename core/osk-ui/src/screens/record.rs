//! §5 Record: "optional key context, optional network badge, the table
//! with reference rows for strings, optional warning cards, one action".
//! Used by the Sign overview, an output, the inputs, the statistics, the
//! tiers and About.

use alloc::string::String;
use alloc::vec::Vec;

use crate::components::{self, Network};
use crate::layout::{Align, Node};
use crate::widgets::WarningLevel;

use super::{Action, Chrome, Frame, KeyContext, Pager};

/// What a [`record`] screen shows.
pub struct Record<'a> {
    /// The app bar's title.
    pub title: &'a str,
    /// The key this screen is about (§4.4).
    pub key: Option<KeyContext>,
    /// The chain, as the caution pill §4.8 puts on every Sign review
    /// screen. Mainnet carries none.
    pub network: Network,
    /// The facts, one per row (§4.11).
    pub rows: Vec<components::Record>,
    /// The ranked warning cards under the table: level, label, value.
    pub warnings: Vec<(WarningLevel, String, String)>,
    /// §4.1 Pager in the bottom action slot, where the record is one of
    /// a run of same-kind records a person walks through.
    pub pager: Option<Pager>,
    /// The one way on.
    pub action: Option<Action>,
}

/// §5 Record: "optional key context, optional network badge, the table
/// with reference rows for strings, optional warning cards, a pager where
/// the record is one of a run, one action."
///
/// §4.11: "A table: label column left, value column right, on every
/// class. A long string is a reference row drawn flat inside the table.
/// One fact per row. The table, with its badge and warnings, is one
/// block centred above the action." §2.6 centres that block in the space
/// between the app bar and the action on every class; a transaction with
/// more outputs than the screen holds starts at the top and scrolls.
///
/// Made of [`components::record`] and [`components::warning_card`]
/// (§4.11), [`components::key_context`] (§4.4),
/// [`components::network_badge`] (§4.8) and
/// [`components::primary`] (§4.13).
pub fn record(c: &Chrome, r: Record<'_>) -> Node {
    record_at(c, r, false)
}

/// The same record titled by a string that is read character by
/// character rather than as words, which is a word from a list on the
/// word list tool's page: the app bar and the pager name the same
/// string, so both take the mono face (§16.126).
pub fn record_mono_title(c: &Chrome, r: Record<'_>) -> Node {
    record_at(c, r, true)
}

fn record_at(c: &Chrome, r: Record<'_>, mono_title: bool) -> Node {
    let class = c.class();
    let m = c.column();
    let mut block = Node::column().gap(c.body_gap());
    if let Some(badge) = components::network_badge(r.network) {
        block = block.child(Node::row().align(Align::Center).child(badge));
    }
    if let Some(k) = &r.key {
        block = block.child(k.node(class, true));
    }
    block = block.child(components::record(&m, r.rows));
    for (level, label, value) in r.warnings {
        block = block.child(components::warning_card(&m, level, label, value));
    }
    let mut footer: Vec<Node> = Vec::new();
    if let Some(p) = r.pager {
        footer.push(p.node(class, mono_title));
    }
    if let Some(a) = r.action {
        footer.extend(c.actions(alloc::vec![a]));
    }
    let frame = Frame::new(r.title, c.place(block, None)).footer(footer);
    c.scrolling(if mono_title {
        frame.mono_title()
    } else {
        frame
    })
}
