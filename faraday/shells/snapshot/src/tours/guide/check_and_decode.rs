//! The pictures of `docs/guide/walk/check-and-decode.md`.

use super::key_session;
use crate::tour::Tour;
use faraday_core::{Action, Screen};

/// Costs what the tool screens cost to draw. A fresh stick session with
/// the guide's key typed in: Tools, the calculators and Decode a
/// transaction.
pub(super) fn run(t: &mut Tour) -> Result<(), String> {
    key_session(t);
    for (screen, name) in [
        (Screen::Catalog, "check-and-decode-tools"),
        (Screen::Tools, "check-and-decode-calculators"),
        (Screen::Decode, "check-and-decode-decode"),
    ] {
        t.press(Action::Nav(screen));
        t.shot(name)?;
    }
    Ok(())
}
