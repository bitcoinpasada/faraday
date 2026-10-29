//! The how tour (`docs/CRYPTO-LEARN.md` §8): the top app bar's How
//! Faraday does it beside the ?, the Learn sheet on its first tab with a
//! block, and the tab scrolled down, on Spend, which offers nine actions.

use crate::tour::{Tour, scroll};
use faraday_core::{Action, Screen};

pub(crate) fn how_tour(t: &mut Tour) -> Result<(), String> {
    t.press(Action::Nav(Screen::Spend));
    t.shot("how-1-bar")?;
    t.press(Action::How);
    t.shot("how-2-tab")?;
    for n in 3..6 {
        scroll(t, 240);
        t.shot(&format!("how-{n}-scrolled"))?;
    }
    t.press(Action::LearnPage(0));
    t.shot("how-6-page")?;
    // Settings' tabs, Cryptography, Key storage and Glossary, each whole;
    // where Learn is a pane it is pinned, and stays open on the Spend
    // tab, the one screen whose Learn offers Spending, step by step.
    t.press(Action::Nav(Screen::Settings));
    t.press(Action::Learn);
    t.shot("how-7-settings")?;
    t.press(Action::LearnPin);
    t.press(Action::Nav(Screen::Family));
    t.shot("how-8-spend-tab")
}
