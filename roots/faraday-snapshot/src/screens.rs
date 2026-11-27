//! The secret-leak check of each step's screen (07 §5.3).

use crate::runner::Tour;

/// Looks for a secret the screen must not show; the step's name is for the failure.
pub fn check(_t: &Tour, _step: &str) -> Result<(), String> {
    todo!()
}
