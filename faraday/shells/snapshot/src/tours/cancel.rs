//! The cancel tour.

use crate::tour::{Tour, type_text};
use faraday_core::testkit;
use faraday_core::{Action, Screen};

/// Cancel at the top of a flow (`docs/FLOWS.md`, Flows in progress,
/// Cancel; F51): Make a new wallet past Kind with Cancel beside the
/// switch, the sheet it brings up, the sheet over its backup's Places,
/// and New key's Cancel back to Make a new wallet.
pub(crate) fn cancel_tour(t: &mut Tour) -> Result<(), String> {
    use faraday_core::cstep;
    for n in 0..3 {
        t.press(Action::Entry(None));
        type_text(t, &testkit::test_words(testkit::TEST_SEEDS[n].0));
        t.press(Action::EntryAdd);
    }
    let multi = faraday_core::create::NewKind::ALL
        .iter()
        .position(|k| *k == faraday_core::create::NewKind::Multi)
        .ok_or("no multisig")? as u8;
    t.press(Action::Nav(Screen::WalletsStart));
    t.press(Action::CreateWallet);
    t.press(Action::CKind(multi));
    t.press(Action::CNext(cstep::KIND));
    t.shot("cancel-1-create")?;
    // The info button after BIP-48 on the path row, its tooltip open.
    t.press(Action::CPathAbout);
    t.shot("cancel-1b-path-tip")?;
    faraday_shell_api::App::event(
        &mut t.app,
        faraday_shell_api::Event::Key(faraday_shell_api::Key::Escape),
    );
    // The sheet comes once an answer past Kind is given; before it Cancel
    // ends the flow at once.
    t.press(Action::CNext(cstep::QUORUM));
    t.press(Action::CancelFlow);
    t.shot("cancel-2-sheet")?;
    t.press(Action::Cancel);
    for slot in 0..3u8 {
        let fp = t.app.session.keys[usize::from(slot)].fingerprint.0;
        t.press(Action::CSlotHere(slot, fp));
    }
    t.press(Action::CNext(cstep::KEYS));
    t.press(Action::CNext(cstep::CHECK));
    t.press(Action::CBackup);
    t.backup_as(faraday_core::plan::Preset::Paper);
    t.shot("cancel-3-places")?;
    t.press(Action::CancelFlow);
    t.shot("cancel-4-places-sheet")?;
    t.press(Action::CancelFlowNow);
    t.shot("cancel-5-wallets")?;
    // New key opened from Make a new wallet's Keys has Cancel too; with
    // rolls entered it asks, and Discard returns to Keys.
    t.press(Action::CreateWallet);
    t.press(Action::CKind(0));
    t.press(Action::CBackupForm(
        faraday_core::create::BackupForm::Words.index(),
    ));
    t.press(Action::KeyGen(Some(0)));
    t.press(Action::KStart(faraday_core::keygen::Start::Make.index()));
    t.press(Action::KWords(12));
    t.press(Action::KWay(faraday_core::keygen::Way::DiceHashed.index()));
    t.press(Action::KNext);
    for i in 0..5u8 {
        t.press(Action::KRoll(i % 6 + 1));
    }
    t.shot("cancel-6-new-key")?;
    t.press(Action::CancelFlow);
    t.shot("cancel-7-key-dialog")?;
    t.press(Action::CancelFlowNow);
    t.shot("cancel-8-keys")?;
    // New key opened on its own has Cancel once rolls are entered, and
    // Discard returns to where it was opened.
    t.press(Action::Nav(faraday_core::Screen::Home));
    t.press(Action::KeyGen(None));
    t.press(Action::KStart(faraday_core::keygen::Start::Make.index()));
    t.press(Action::KWords(12));
    t.press(Action::KWay(faraday_core::keygen::Way::DiceHashed.index()));
    t.press(Action::KNext);
    for i in 0..5u8 {
        t.press(Action::KRoll(i % 6 + 1));
    }
    t.press(Action::CancelFlow);
    t.shot("cancel-9-lone-key-dialog")?;
    t.press(Action::CancelFlowNow);
    t.shot("cancel-10-home")
}
