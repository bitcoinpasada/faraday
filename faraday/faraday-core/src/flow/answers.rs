//! Which rows of Your choices take a press (`docs/FLOW-CONTAINER.md` §2).

use super::Row;
use crate::trail::Steps;

/// Whether a press on `row` opens its step: a step shown, not the one
/// open (a step is always open), after the first Back may open, and none
/// while the step is running.
pub(crate) fn takes_press(steps: &Steps, row: &Row) -> bool {
    if steps.running || row.current {
        return false;
    }
    let place = |k: u8| steps.order.iter().position(|&s| s == k);
    let Some(at) = place(row.opens) else {
        return false;
    };
    at >= steps.first.and_then(place).unwrap_or(0)
}
