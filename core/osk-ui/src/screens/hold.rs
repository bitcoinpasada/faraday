//! §5 Hold: "a record of what will happen, 'Sign with' chips where
//! several keys could sign, the hold". Used by Sign confirm, Forget and
//! Wipe.

use alloc::string::String;
use alloc::vec::Vec;

use crate::components;
use crate::layout::{Id, Node};
use crate::widgets::WarningLevel;

/// One key that could sign, its chip's id and whether it is on (§4.4).
pub type SigningKey = (Id, String, bool);

use super::{Action, Chrome, Frame};

/// What a [`hold`] screen confirms.
pub struct Hold<'a> {
    /// The app bar's title.
    pub title: &'a str,
    /// What will happen, one fact per row (§4.11). This is the whole
    /// explanation: §4.13 puts no sentence above the button.
    pub rows: Vec<components::Record>,
    /// The ranked warning cards under the table: level, label, value.
    /// What the person should weigh before they hold, said as §4.11 says
    /// it — a label and a value, never a sentence above the button.
    pub warnings: Vec<(WarningLevel, String, String)>,
    /// §4.4 Sign with: the caption and one chip per key that could sign,
    /// each on or off. Absent where only one key can (`None`), because a
    /// row of one chip offers nothing.
    pub sign_with: Option<(String, Vec<SigningKey>)>,
    /// A second chip row under the first, with its own caption: the
    /// other shares of a threshold wallet that sign at a later location
    /// (`docs/PLANNING.md` §16.103). Absent everywhere else.
    pub then_with: Option<(String, Vec<SigningKey>)>,
    /// The button's application id.
    pub id: Id,
    /// The verb: "Hold to sign", "Hold to wipe".
    pub label: &'a str,
    /// A step that removes something, which colours the fill.
    pub danger: bool,
    /// False while the step is not available yet — a danger warning is
    /// unacknowledged, a key is not chosen. The button keeps its place
    /// and its size either way.
    pub enabled: bool,
    /// The one plain action that declines the step, beside the hold in
    /// the same slot and sharing its width (§4.13). Most holds have
    /// none: the step is reached by a tap and the chevron goes back.
    pub secondary: Option<Action>,
}

/// §5 Hold: "a record of what will happen, 'Sign with' chips where
/// several keys could sign, the hold."
///
/// §2.7: "A hold confirms an action the app cannot undo on its own:
/// signing, forgetting a key, wiping. No other confirm dialog." §4.13:
/// "The label is the verb. No sentence above it" — what will happen is
/// the table, not a line attached to the control. The `wide` sidebar is
/// inert here, so navigation never shares a screen with a destructive
/// step.
///
/// §2.6 places the two apart: the table is read, so it is the block
/// centred in the space under the app bar; the "Sign with" chips are
/// touched, so they sit in the bottom group directly above the hold.
///
/// A hold may carry one secondary plain action, the step that declines
/// it: [`components::secondary`] to the hold's left, the two sharing the
/// row's width as §4.13's pair does.
///
/// Made of [`components::record`] and [`components::warning_card`]
/// (§4.11), [`components::sign_with`] (§4.4) and
/// [`components::hold`] (§4.13).
pub fn hold(c: &Chrome, h: Hold<'_>) -> Node {
    let m = c.column();
    let mut block = Node::column()
        .gap(c.body_gap())
        .child(components::record(&m, h.rows));
    for (level, label, value) in h.warnings {
        block = block.child(components::warning_card(&m, level, label, value));
    }
    let rows: Vec<Node> = [h.sign_with, h.then_with]
        .into_iter()
        .flatten()
        .map(|(caption, keys)| components::sign_with(caption, &keys))
        .collect();
    let chips = match rows.len() {
        0 => None,
        1 => rows.into_iter().next(),
        _ => Some(
            rows.into_iter()
                .fold(Node::column().gap(c.body_gap()), Node::child),
        ),
    };
    let mut footer = Vec::new();
    if let Some(a) = h.secondary {
        footer.push(components::secondary(a.id, a.label));
    }
    footer.push(components::hold(h.id, h.label, h.danger, h.enabled));
    c.scrolling(
        Frame::new(h.title, c.place(block, chips))
            .footer(c.buttons(footer))
            .dimmed(),
    )
}
