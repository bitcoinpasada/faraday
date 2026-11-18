//! The home search tour.

use crate::tour::{Tour, scroll, scroll, type_text};
use faraday_core::edit::Here;
use faraday_core::{Action, Screen};
use faraday_shell_api::{App, Event};

/// Home's search bar with nothing typed, with "send" typed, with "theme" typed
/// (a setting shows), with "quorum" typed (a glossary word shows) and with a word nothing has.
pub(crate) fn home_search_tour(t: &mut Tour) -> Result<(), String> {
    t.press(Action::Nav(Screen::Home));
    let find = Action::TypeHere(Here::HomeFind);
    // A phone-width Home has the bar below the fold: scroll to it as a
    // person does. The Pi's small panel has no bar (its Home is the menu).
    let mut scrolled = 0;
    while scrolled < 3600 && t.app.hit_box(find).is_none() {
        scroll(t, 120);
        t.app.settle();
        let _ = t.app.frame();
        scrolled += 120;
    }
    let Some((x, y, w, h)) = t.app.hit_box(find) else {
        scroll(t, -3600);
        t.app.settle();
        t.shot("home-search-empty")?;
        return Ok(());
    };
    t.shot("home-search-empty")?;
    t.app.event(Event::Hover {
        x: x + w / 2,
        y: y + h / 2,
    });
    t.tick();
    t.shot("home-search-hovered")?;
    t.app.event(Event::HoverEnd);
    t.press(Action::TypeHere(Here::HomeFind));
    t.shot("home-search-focused")?;
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
