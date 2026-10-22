//! The in_progress tour.

use crate::tour::{Tour, type_text};
use faraday_core::testkit;
use faraday_core::{Action, Screen, StorageEvent};

/// The backup flow for a GPG key kept in two vaults at three places
/// (`docs/BACKUP-FLOW.md`): the screens a person sees from Where to Done.
/// Two flows in progress (`docs/FLOWS.md`, Flows in progress): a GPG
/// key's flow at its sticks, and Make a new wallet left on its backup's
/// Places. Each row is lit on its own screen; Settings shows both under
/// their tabs, and the small Home leads with a tile for each.
pub(crate) fn in_progress_tour(t: &mut Tour) -> Result<(), String> {
    use faraday_core::backup_flow::Kind;
    use faraday_core::backup_flow::steps::FlowAction as F;
    use faraday_core::cstep;
    t.app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    t.app.storage(StorageEvent::Clock {
        unix_secs: 1_791_000_000,
    });
    t.press(Action::OpenSetTime);
    t.press(Action::SetTimeKeep);
    let tile = faraday_core::catalog::TILES
        .iter()
        .position(|x| x.go == faraday_core::catalog::Go::VaultCategory(Kind::Gpg.category()))
        .ok_or("no GPG tile")? as u8;
    t.press(Action::Nav(Screen::Catalog));
    t.press(Action::Catalog(tile));
    t.press(Action::Flow(F::Continue));
    t.shot("inprogress-gpg")?;
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
    t.press(Action::CNext(cstep::CHECK));
    t.press(Action::CBackup);
    t.backup_as(faraday_core::plan::Preset::Paper);
    t.shot("inprogress-places")?;
    t.press(Action::Nav(Screen::Settings));
    t.shot("inprogress-settings")?;
    t.press(Action::Nav(Screen::Home));
    t.shot("inprogress-home")
}
