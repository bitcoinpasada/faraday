//! The My seed makes my wallet pick tour.

use crate::tour::Tour;
use faraday_core::seed_check::SeedAction as S;
use faraday_core::testkit;
use faraday_core::{Action, Screen};

/// My seed makes my wallet's Seed card with no key loaded, with two keys
/// loaded, and with Other forms opened.
pub(crate) fn seed_pick_tour(t: &mut Tour) -> Result<(), String> {
    let tile = faraday_core::catalog::TILES
        .iter()
        .position(|tile| tile.name == "My seed makes my wallet")
        .ok_or("no My seed makes my wallet tile")?;
    t.press(Action::Nav(Screen::Catalog));
    t.press(Action::Catalog(tile as u8));
    t.shot("seed-pick-no-key")?;
    let c = testkit::compute();
    for (name, words) in [
        (
            "Savings",
            "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about",
        ),
        (
            "Spare",
            "legal winner thank year wave sausage worth useful legal winner thank yellow",
        ),
    ] {
        t.app
            .session
            .keys
            .add_words(&c, words, name, None)
            .map_err(|e| format!("{e:?}"))?;
    }
    t.press(Action::Nav(Screen::Catalog));
    t.press(Action::Catalog(tile as u8));
    t.shot("seed-pick-keys")?;
    t.press(Action::EntryOtherForms(
        faraday_core::Opening::SeedMakesWallet,
    ));
    t.shot("seed-pick-other-forms")?;
    t.press(Action::SeedCheck(S::Type));
    t.shot("seed-pick-typing")?;
    Ok(())
}
