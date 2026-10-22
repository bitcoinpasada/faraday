//! The desktop tours.

use crate::tour::{Tour, type_text};
use faraday_core::testkit;
use faraday_core::{Action, Screen, StickInfo, StorageEvent};
use faraday_crypto::work::Compute;

/// The desktop app's stick pills: the default two, then a wallet's backup
/// with five vaults.
pub(crate) fn pills_tour(t: &mut Tour) -> Result<(), String> {
    t.app.shell_sticks = true;
    t.app.online = true;
    let stick = |name: &str| StickInfo {
        id: name.into(),
        label: name.into(),
        boot: name == "BOOTSTICK",
        serial: None,
        files: Vec::new(),
    };
    t.app.storage(StorageEvent::Sticks(vec![
        stick("BOOTSTICK"),
        stick("STICK-2"),
    ]));
    t.press(Action::Nav(Screen::Home));
    t.shot("pills-default")?;
    let multi = faraday_core::create::NewKind::ALL
        .iter()
        .position(|k| *k == faraday_core::create::NewKind::Multi)
        .unwrap_or(0) as u8;
    t.press(Action::CreateWallet);
    t.press(Action::CKind(multi));
    t.press(Action::CVaults(true));
    t.press(Action::CNext(faraday_core::cstep::WHERE));
    t.press(Action::CVaultN(5));
    t.press(Action::CNext(faraday_core::cstep::HOW_MANY));
    t.shot("pills-five-vaults")
}

/// Costs what `set_demo` costs. A released Faraday Desktop's first
/// screen: testnet, behind the online warning (F61), then Home once it
/// is acknowledged.
pub(crate) fn start_warning_tour(c: &Compute, t: &mut Tour) -> Result<(), String> {
    t.app.online = true;
    t.app.shell_sticks = true;
    t.app.session = faraday_core::wallet::Session::default();
    t.app.session.set_demo(c);
    t.app
        .session
        .set_network(testkit::NET)
        .map_err(|e| e.text())?;
    t.app.with_start_warning();
    t.shot("desktop-start-warning")?;
    t.press(Action::AirgapUnderstood);
    t.shot("desktop-start-home")
}

/// Costs what `set_demo` costs. The demo's dimmed actions
/// (`docs/DEMO.md` §7): BIP-85's Value card and the Lightning node key,
/// on a public seed.
pub(crate) fn demo_tour(c: &Compute, t: &mut Tour) -> Result<(), String> {
    t.app.online = true;
    t.app.session = faraday_core::wallet::Session::default();
    t.app.session.set_demo(c);
    t.press(Action::Entry(None));
    type_text(t, &faraday_core::demo::words("bacon"));
    t.press(Action::EntryAdd);
    let fp = t
        .app
        .session
        .keys
        .first()
        .ok_or("bacon was not loaded")?
        .fingerprint
        .0;
    t.press(Action::Bip85);
    t.press(Action::PKey(fp));
    t.press(Action::PNext);
    t.press(Action::PNext);
    t.shot("demo-bip85-value")?;
    t.press(Action::Lightning);
    t.press(Action::LKey(fp));
    t.shot("demo-lightning")
}
