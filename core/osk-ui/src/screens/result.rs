//! §5 Result: "icon and coloured title, a record, one or two actions".
//! Used by the Verify result, the Sign result, the quiz result, a
//! checksum, the scan results and a finished wipe.

use alloc::string::String;
use alloc::vec::Vec;

use crate::components;
use crate::layout::Node;
use crate::widgets::{Icon, Tone};

use super::{Action, Chrome, Frame};

/// What a [`result`] screen states.
pub struct Result<'a> {
    /// The app bar's title.
    pub title: &'a str,
    /// The mark over the result.
    pub icon: Icon,
    /// The result's colour: success, danger, caution, muted.
    pub tone: Tone,
    /// The result itself: "Signed", "Yours", "Not found", "Passed",
    /// "Self-test failed".
    pub result: &'a str,
    /// What it is a result about, one fact per row (§4.11).
    pub rows: Vec<components::Record>,
    /// One or two ways on. None of them is §4.14's terminal state:
    /// "Session ended", "Self-test failed · vector 3".
    pub actions: Vec<Action>,
    /// A line under the record that states what has just happened and
    /// then stops: "Copied", "No clipboard" (PLANNING §16.88). Drawn as
    /// §4.12's caption, muted, and reserved no room when there is none.
    pub caption: Option<String>,
}

/// §5 Result: "icon and coloured title, a record, one or two actions."
///
/// §4.11: "Icon and a coloured title at the top of the body, then a
/// record under it. The colour is on the title." §2.6 makes the title
/// and the table one block, centred in the space above the actions. A
/// result with no action is §4.14's terminal state: the same block
/// centred in the whole space, with no back chevron either and the
/// `wide` sidebar inert, because the screen is the end of the flow.
///
/// Made of [`components::result`] and [`components::record`] (§4.11)
/// with [`components::primary`] and [`components::secondary`] (§4.13)
/// under them.
pub fn result(c: &Chrome, v: Result<'_>) -> Node {
    let m = c.column();
    let terminal = v.actions.is_empty();
    let mut block = Node::column()
        .gap(c.body_gap())
        .child(components::result(v.icon, v.tone, v.result));
    if !v.rows.is_empty() {
        block = block.child(components::record(&m, v.rows));
    }
    if let Some(caption) = v.caption {
        block = block.child(components::reason(caption));
    }
    let frame = Frame::new(v.title, c.place(block, None)).footer(c.actions(v.actions));
    c.scrolling(if terminal { frame.terminal() } else { frame })
}
