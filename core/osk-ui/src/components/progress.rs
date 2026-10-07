//! §4.14 Progress and state: the bar with its count, the countdown, the
//! empty state and the terminal state.

use alloc::string::String;
use alloc::vec::Vec;

use crate::geom::{Metrics, SizeClass};
use crate::layout::{Id, Node};
use crate::tokens;
use crate::widgets::{Icon, Tone};

use super::records::{Record, record, result};

/// §4.14 Progress: "A bar with a count '2 of 5' beside or above it.
/// Entropy entry, scanning parts, animated codes."
pub fn progress(fraction: f32, count: impl Into<String>) -> Node {
    super::codes::progress_bar(fraction, Some(count.into()))
}

/// The same bar with no count above it, for the one row that has to be
/// one line: the QR screen's progress on `small`, whose count is in the
/// title instead (§3, §4.9).
pub fn progress_of(fraction: f32) -> Node {
    super::codes::progress_bar(fraction, None)
}

/// §4.14 Countdown: "Only the eye's ring (§4.10). The status line has
/// none."
///
/// The share of the eye's 30 s reveal still to run, which
/// [`super::nav::bar_eye`] draws as a ring. There is no other countdown
/// and no countdown text anywhere.
pub fn reveal_remaining(seconds_left: u32) -> f32 {
    (seconds_left as f32 / tokens::EYE_REVEAL_S as f32).clamp(0.0, 1.0)
}

/// §4.14 Empty state: "A row that starts the flow: 'Load a key', 'Create
/// a key'. Never a sentence about emptiness."
pub fn empty_row(class: SizeClass, id: Id, icon: Icon, label: impl Into<String>) -> Node {
    super::actions::row_action(class, id, icon, label)
}

/// §4.14 Terminal state: "Result layout with no action and no back
/// chevron: 'Session ended', 'Self-test failed · vector 3'."
pub fn terminal(
    m: &Metrics,
    icon: Icon,
    tone: Tone,
    title: impl Into<String>,
    rows: Vec<Record>,
) -> Node {
    let mut column = Node::column()
        .gap(tokens::GAP)
        .child(result(icon, tone, title));
    if !rows.is_empty() {
        column = column.child(record(m, rows));
    }
    column
}

/// The line a screen shows instead of a value it has nothing for: the
/// reason token, so it reads as an aside and not as the value.
pub fn no_value(text: impl Into<String>) -> Node {
    super::text::reason(text)
}

/// A label for a run: "2 of 5", built where the count is, so a bar, a
/// pager and a title all say the same thing.
pub fn count_of(index: usize, total: usize, of: &str) -> String {
    alloc::format!("{index} {of} {total}")
}
