//! The cancel tour.

use crate::tour::{Tour, type_text};
use faraday_core::testkit;
use faraday_core::{Action, Screen};

/// Cancel at the top of a flow (`docs/FLOWS.md`, Flows in progress,
/// Cancel; F51): Make a new wallet past Kind with Cancel beside the
/// switch, the sheet it brings up, and the sheet over its backup's Places.
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
    t.press(Action::CVaults(false));
    t.press(Action::CNext(cstep::WHERE));
    t.shot("cancel-1-create")?;
    t.press(Action::CancelFlow);
    t.shot("cancel-2-sheet")?;
    t.press(Action::Cancel);
    t.press(Action::CNext(cstep::QUORUM));
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
    t.shot("cancel-5-wallets")
}
