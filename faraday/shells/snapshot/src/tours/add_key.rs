//! The add_key tour.

use crate::tour::{Tour, type_text};
use faraday_core::testkit;
use faraday_core::{Action, Screen};
use faraday_shell_api::App;

/// Add a key, which is New key opening on Start:
/// the Start card, its Other forms, the Words card typed, the Key
/// card for words typed, locked in; then the codex32 tile's Words card;
/// then a PSBT read at Add a key's Scan a SeedQR, on the read card
/// (`docs/SCANNING.md` §5).
pub(crate) fn add_key_tour(t: &mut Tour) -> Result<(), String> {
    use faraday_core::keygen::Start;
    t.press(Action::Nav(Screen::WalletsStart));
    t.press(Action::KeyGen(None));
    // Start opens with nothing ticked.
    t.shot("addkey-start")?;
    t.press(Action::KOthers);
    t.shot("addkey-start-others")?;
    t.press(Action::KOthers);
    t.press(Action::KStart(Start::Words.index()));
    t.shot("addkey-words")?;
    let words = testkit::test_words(testkit::TEST_SEEDS[0].0);
    let cut = words.len() / 2;
    type_text(t, &words[..cut]);
    t.shot("addkey-words-typing")?;
    type_text(t, &words[cut..]);
    t.shot("addkey-words-typed")?;
    t.press(Action::KNext);
    t.shot("addkey-key")?;
    t.press(Action::KLock);
    // Lock in leaves the flow by itself.
    t.shot("addkey-key-locked")?;
    // Opened from Tools, a key typed in ends on its result card.
    t.press(Action::Nav(Screen::Catalog));
    t.press(Action::KeyGen(None));
    t.press(Action::KStart(Start::Words.index()));
    let other = testkit::test_words(testkit::TEST_SEEDS[1].0);
    type_text(t, &other);
    t.press(Action::KNext);
    t.press(Action::KLock);
    t.shot("addkey-tools-result")?;
    t.press(Action::KMadeDone);
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
