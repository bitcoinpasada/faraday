//! The how tour (`docs/CRYPTO-LEARN.md` §8): the top app bar with the ?
//! alone, the guide sheet on its Cryptography tab with a block, and the tab scrolled down, on Spend, which offers nine actions.

use crate::tour::{Tour, scroll};
use faraday_core::{Action, Screen};
use faraday_shell_api::App;

pub(crate) fn how_tour(t: &mut Tour) -> Result<(), String> {
    t.press(Action::Nav(Screen::Spend));
    t.shot("how-1-bar")?;
    t.press(Action::Guide);
    t.press(Action::GuideTabHow);
    t.shot("how-2-tab")?;
    for n in 3..6 {
        scroll(t, 240);
        t.shot(&format!("how-{n}-scrolled"))?;
    }
    t.press(Action::GuideTabGuide);
    t.shot("how-6-page")?;
    // The page's Words on this page rows, and the glossary at the first.
    let word = words_in_view(t).ok_or("no Words on this page row on Spend's guide page")?;
    t.shot("how-6-words")?;
    t.press(word);
    t.shot("how-6-glossary")?;
    // Settings' tabs, Cryptography, Key storage and Glossary, each whole;
    // where the guide is a pane it stays open on the Spend tab. Closed
    // first: a pane would still be open, and the ? would put it away.
    t.press(Action::GuideClose);
    t.press(Action::Nav(Screen::Settings));
    t.press(Action::Guide);
    t.shot("how-7-settings")?;
    t.press(Action::Nav(Screen::SpendBackup));
    t.shot("how-8-spend-tab")
}

/// Scrolls the open guide page until one of its word rows is offered,
/// and returns its action.
fn words_in_view(t: &mut Tour) -> Option<Action> {
    for _ in 0..40 {
        let _ = t.app.frame();
        let found = t
            .app
            .offered_actions()
            .into_iter()
            .find(|a| matches!(a, Action::GuideWord(_)));
        if found.is_some() {
            return found;
        }
        scroll(t, 240);
    }
    None
}
