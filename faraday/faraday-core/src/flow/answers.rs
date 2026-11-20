//! Which rows of Your choices take a press (`docs/FLOW-CONTAINER.md` §1, §2).

use super::Row;
use crate::trail::Steps;

/// Whether a press on `row` opens its step: a step shown, not the one
/// open, never while a step is running. A step before the open one opens
/// from the first Back may open; one after it only while its answer and
/// every answer between hold, so a row with nothing reached takes none.
pub(crate) fn takes_press(steps: &Steps, row: &Row) -> bool {
    if steps.running || row.current {
        return false;
    }
    let place = |k: u8| steps.order.iter().position(|&s| s == k);
    let Some(at) = place(row.opens) else {
        return false;
    };
    let Some(open) = steps.open.and_then(place) else {
        return false;
    };
    if at < open {
        return at >= steps.first.and_then(place).unwrap_or(0);
    }
    steps
        .order
        .iter()
        .skip(open.saturating_add(1))
        .take(at.saturating_sub(open))
        .all(|k| steps.answered.contains(k))
}
