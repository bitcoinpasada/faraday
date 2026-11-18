//! The add_key tour.

use crate::tour::{Tour, type_text};
use faraday_core::testkit;
use faraday_core::{Action, Screen};
use faraday_shell_api::App;

/// New key opened from Wallets, which lists only the ways of making a
/// key: its Start card and its Other forms. Then Add a key, where a key
/// held is brought in: the words typed, added; from Tools the same, then
/// the codex32 tile's Words card; then a PSBT read at Add a key's Scan a
/// SeedQR, on the read card (`docs/SCANNING.md` §5).
pub(crate) fn add_key_tour(t: &mut Tour) -> Result<(), String> {
    t.press(Action::Nav(Screen::WalletsStart));
    t.press(Action::KeyGen(None));
    // Start opens with nothing ticked.
    t.shot("addkey-start")?;
    t.press(Action::KOthers);
    t.shot("addkey-start-others")?;
    t.press(Action::Back);
    t.press(Action::Nav(Screen::WalletsStart));
    t.press(Action::Entry(None));
    t.shot("addkey-words")?;
    let words = testkit::test_words(testkit::TEST_SEEDS[0].0);
    let cut = words.len() / 2;
    type_text(t, &words[..cut]);
    t.shot("addkey-words-typing")?;
    type_text(t, &words[cut..]);
    t.shot("addkey-words-typed")?;
    t.shot("addkey-key")?;
    t.press(Action::EntryAdd);
    t.shot("addkey-key-locked")?;
    // From Tools the same Add a key takes a key held.
    t.press(Action::Nav(Screen::Catalog));
    t.press(Action::Entry(None));
    let other = testkit::test_words(testkit::TEST_SEEDS[1].0);
    type_text(t, &other);
    t.press(Action::EntryAdd);
    t.shot("addkey-tools-result")?;
    // Find a tool also finds the rows of Settings.
    t.press(Action::Nav(Screen::Catalog));
    type_text(t, "theme");
    t.shot("addkey-tools-find-setting")?;
    t.app.event(faraday_shell_api::Event::Key(
        faraday_shell_api::Key::Escape,
    ));
    let codex32 = faraday_core::catalog::TILES
        .iter()
        .position(|tile| tile.name == "Restore from codex32 shares")
        .ok_or("no codex32 tile")?;
    t.press(Action::Catalog(codex32 as u8));
    t.shot("addkey-codex32")?;
    t.press(Action::Entry(None));
    t.press(Action::ScanSeed);
    let kit = testkit::kits()
        .into_iter()
        .find(|k| k.id == "spending")
        .ok_or("no spending kit")?;
    let psbt = testkit::unsigned(&kit)
        .map_err(|e| String::from(&*e))?
        .to_bytes();
    t.app
        .event(faraday_shell_api::Event::Scanned { bytes: psbt });
    t.pump();
    t.shot("addkey-scan-card")
}
