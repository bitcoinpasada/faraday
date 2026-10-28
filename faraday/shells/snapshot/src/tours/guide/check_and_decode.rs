//! The pictures of `docs/guide/walk/check-and-decode.md`. Its Decode a
//! transaction is drawn where the guide's spend finishes
//! (`spend::spend_from_stick`).

use super::key_session;
use crate::tour::{Tour, type_text};
use faraday_core::{Action, Screen};

/// Costs what the tool screens cost. A fresh stick session with the
/// guide's key typed in: Tools, and the calculators on Hashes.
pub(super) fn run(t: &mut Tour) -> Result<(), String> {
    key_session(t);
    t.press(Action::Nav(Screen::Catalog));
    t.shot("check-and-decode-tools")?;
    hashes(t, "check-and-decode-calculators")
}

/// Costs three hashes. Tools' Hashes calculator opened from Tools, `abc`
/// typed, the shot named `name`.
pub(crate) fn hashes(t: &mut Tour, name: &str) -> Result<(), String> {
    t.press(Action::Nav(Screen::Catalog));
    t.press(Action::Tools);
    t.press(Action::TTool(0));
    type_text(t, "abc");
    t.shot(name)
}
