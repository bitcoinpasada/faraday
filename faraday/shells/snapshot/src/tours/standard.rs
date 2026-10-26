//! The standard tour.

use crate::tour::{Tour, type_text};
use faraday_core::Action;
use faraday_core::testkit;

/// The standard way on the board (`docs/BACKUP-FLOW.md` §17.6, §17.3,
/// §19.5, §19.21): Make a new wallet opening on Kind; its Check with
/// Back up wallet; a single key's Back up open on the board
/// read-only; a 2-of-3's the same, then with Add watch-only software
/// pressed; the 2-of-3 after
/// Customize backup with its form toggles; and the sheet over a drop that
/// lets a place see the balance.
pub(crate) fn standard_tour(t: &mut Tour) -> Result<(), String> {
    use faraday_core::board::{BoardPress as B, Chip, Thing, Zone};
    use faraday_core::cstep;
    t.press(Action::Entry(None));
    type_text(t, &testkit::test_words(testkit::TEST_SEEDS[0].0));
    t.press(Action::EntryAdd);
    t.press(Action::CreateWallet);
    t.shot("std-1-create-kind")?;
    // A single key from the key loaded.
    t.press(Action::CKind(0));
    let fp = t.app.session.keys[0].fingerprint.0;
    t.press(Action::CSlotHere(0, fp));
    t.press(Action::CNext(cstep::KEYS));
    // The vault question with its line (§19.21 item 5), then Check's
    // It matches, leaving Back up wallet at its foot (item 2).
    t.press(Action::CStep(cstep::WHERE));
    t.shot("std-2a-vault-question")?;
    t.press(Action::CNext(cstep::CHECK));
    t.shot("std-2b-create-back-up")?;
    t.press(Action::CBackup);
    t.shot("std-2-single-key")?;
    for n in 1..3 {
        t.press(Action::Entry(None));
        type_text(t, &testkit::test_words(testkit::TEST_SEEDS[n].0));
        t.press(Action::EntryAdd);
    }
    let multi = faraday_core::create::NewKind::ALL
        .iter()
        .position(|k| *k == faraday_core::create::NewKind::Multi)
        .ok_or("no multisig")? as u8;
    t.press(Action::CreateWallet);
    t.press(Action::CKind(multi));
    for slot in 0..3u8 {
        let fp = t.app.session.keys[usize::from(slot)].fingerprint.0;
        t.press(Action::CSlotHere(slot, fp));
    }
    t.press(Action::CNext(cstep::KEYS));
    let three = t
        .app
        .session
        .wallets
        .len()
        .checked_sub(1)
        .ok_or("no wallet")?;
    t.press(Action::Backup(three));
    t.shot("std-3-two-of-three")?;
    // Add watch-only software on the read-only board (§19.21 item 3).
    t.press(Action::Board(B::Software));
    t.shot("std-3a-watch-only")?;
    t.press(Action::Board(B::Software));
    t.press(Action::Board(B::Customize));
    t.shot("std-4-customize")?;
    // Whole off Place 3, Share 1 dropped there beside seed 3: the sheet.
    t.press(Action::Board(B::Off(Zone::Paper(2), Thing::Whole)));
    t.app
        .press(Action::Board(B::Chip(Chip::List(Thing::Share(0)))));
    t.app.press(Action::Board(B::Zone(Zone::Paper(2))));
    t.pump();
    t.shot("std-5-sees-sheet")
}
