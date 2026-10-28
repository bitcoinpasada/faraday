//! The pictures of `docs/guide/walk/multisig-with-others.md`.

use super::{DICE_WORDS, seedqr};
use crate::tour::Tour;
use faraday_core::{Action, Screen};
use faraday_crypto::work::Compute;
use faraday_shell_api::{App, Event};

/// Costs two mnemonics' checksums. §5.6: Remove every key; the 2-of-3's
/// card short of keys; Scan a key; test key 2's SeedQR, then the dice
/// key's.
pub(super) fn scan_key(c: &Compute, t: &mut Tour) -> Result<(), String> {
    let w = t
        .app
        .session
        .wallets
        .len()
        .checked_sub(1)
        .ok_or("no wallet")?;
    t.press(Action::Nav(Screen::Settings));
    t.shot_marking("5-6-settings", Action::RemoveKeys)?;
    t.press(Action::RemoveKeys);
    t.shot_marking(
        "multisig-with-others-keys-removed",
        Action::Nav(Screen::Wallets),
    )?;
    t.press(Action::Nav(Screen::Wallets));
    t.press(Action::OpenWallet(w));
    t.shot_marking("5-6-card-short", Action::ScanKey(w))?;
    t.press(Action::ScanKey(w));
    crate::tours::site_tour::camera(t);
    t.shot("5-6-camera")?;
    let zebra = faraday_core::testkit::test_words(faraday_core::testkit::TEST_SEEDS[1].0);
    t.app.event(Event::Scanned {
        bytes: seedqr(c, &zebra)?,
    });
    t.pump();
    t.shot_after("5-6-one-more", 0)?;
    t.app.event(Event::Scanned {
        bytes: seedqr(c, DICE_WORDS)?,
    });
    t.pump();
    t.shot_after("5-6-can-spend", 0)?;
    t.shot("multisig-with-others-card-can-spend")
}
