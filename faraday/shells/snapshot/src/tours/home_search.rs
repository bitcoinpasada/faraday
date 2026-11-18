//! The home search tour.

use crate::tour::{Tour, scroll, type_text};
use faraday_core::{Action, Screen};

/// Home's search bar with nothing typed, with "send" typed, with "theme" typed
/// (a setting shows), with "quorum" typed (a glossary word shows) and with a word nothing has.
pub(crate) fn home_search_tour(t: &mut Tour) -> Result<(), String> {
    t.press(Action::Nav(Screen::Home));
    t.shot("home-search-empty")?;
    type_text(t, "send");
    t.shot("home-search-send")?;
    t.press(Action::HomeSearchClear);
    type_text(t, "theme");
    t.shot("home-search-theme")?;
    t.press(Action::HomeSearchClear);
    type_text(t, "quorum");
    t.shot("home-search-word")?;
    t.press(Action::HomeSearchClear);
    type_text(t, "zzzz");
    t.shot("home-search-none")?;
    // A phone window has no search bar on Home: Tools' Find a tool offers the word.
    t.press(Action::Nav(Screen::Catalog));
    type_text(t, "seal");
    t.shot("tools-find-word")?;
    scroll(t, 400);
    t.shot("tools-find-word-scrolled")?;
    Ok(())
}
