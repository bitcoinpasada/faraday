//! The pictures of `docs/guide/walk/spend-from-a-backup.md`.

use crate::tour::Tour;
use faraday_core::{Action, Screen};

/// The Spend tab's map in a fresh stick session with nothing loaded.
pub(super) fn run(t: &mut Tour) -> Result<(), String> {
    crate::tours::screens::stick_session(t);
    t.press(Action::Nav(Screen::Family));
    t.shot("spend-from-a-backup-map")
}
