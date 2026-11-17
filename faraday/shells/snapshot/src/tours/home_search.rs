//! The home search tour.

use crate::tour::{Tour, type_text};
use faraday_core::{Action, Screen};

/// Home's search bar with nothing typed, with "new key" typed, and with a
/// word no tool has.
pub(crate) fn home_search_tour(t: &mut Tour) -> Result<(), String> {
    t.press(Action::Nav(Screen::Home));
    t.shot("home-search-empty")?;
    type_text(t, "new key");
    t.shot("home-search-key")?;
    type_text(t, "zzzz");
    t.shot("home-search-none")?;
    Ok(())
}
