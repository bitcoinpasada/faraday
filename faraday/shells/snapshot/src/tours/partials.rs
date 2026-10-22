//! The partials tour.

use crate::tour::{Tour, scroll, type_text};
use faraday_core::Action;
use faraday_core::testkit;

/// Partial descriptions by one button (`docs/BACKUP-FLOW.md` §17.19): a
/// 2-of-3's board customised; the sheet after adding them; the board with
/// a share at each place and the whole taken off every place; the board
/// after Remove partial descriptions, its warning under it.
pub(crate) fn partials_tour(t: &mut Tour) -> Result<(), String> {
    use faraday_core::board::BoardPress as B;
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
    t.press(Action::CreateWallet);
    t.press(Action::CKind(multi));
    for slot in 0..3u8 {
        let fp = t.app.session.keys[usize::from(slot)].fingerprint.0;
        t.press(Action::CSlotHere(slot, fp));
    }
    t.press(Action::CNext(cstep::KEYS));
    let w = t
        .app
        .session
        .wallets
        .len()
        .checked_sub(1)
        .ok_or("no wallet")?;
    t.press(Action::Backup(w));
    t.press(Action::Board(B::Customize));
    t.app.press(Action::Board(B::Partials));
    t.pump();
    t.shot("parts-1-sheet")?;
    t.press(Action::Board(B::PartialsWholeOff));
    t.shot("parts-2-shares")?;
    t.press(Action::Board(B::Partials));
    scroll(t, 400);
    t.shot("parts-3-removed")
}
