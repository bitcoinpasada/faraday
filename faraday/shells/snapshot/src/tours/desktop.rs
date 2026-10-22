//! The desktop tours.

use crate::tour::{Tour, type_text};
use faraday_core::testkit;
use faraday_core::{Action, Screen, StickInfo, StorageEvent};

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

/// The demo's dimmed actions (`docs/DEMO.md` §7): BIP-85's Value card
/// and the Lightning node key, on a sample.
pub(crate) fn demo_tour(t: &mut Tour) -> Result<(), String> {
    t.app.online = true;
    t.app.session = faraday_core::wallet::Session::on(testkit::NET);
    t.app
        .session
        .set_demo(faraday_core::testkit::demo_samples());
    t.press(Action::Entry(None));
    type_text(t, &faraday_core::testkit::test_words("bacon"));
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

/// The mode pill and sheet (`docs/MODES.md`
/// §3): Home and the sheet in Demo and on Testnet, then Settings' Mode
/// row on Mainnet, where no pill shows.
pub(crate) fn modes_tour(t: &mut Tour) -> Result<(), String> {
    t.app.online = true;
    t.app.session = faraday_core::wallet::Session::on(testkit::NET);
    t.app
        .session
        .set_demo(faraday_core::testkit::demo_samples());
    t.app.set_modes(faraday_core::DESKTOP_MODES);
    t.press(Action::Nav(Screen::Home));
    t.shot("modes-demo-home")?;
    t.press(Action::ModeSheet);
    t.shot("modes-demo-sheet")?;
    t.press(Action::Cancel);
    // Where a seed is typed, online, in Demo and on testnet: one line
    // under the field.
    t.press(Action::Entry(None));
    type_text(t, "abandon abandon abandon");
    t.shot("modes-demo-add-key")?;
    crate::tour::scroll(t, 700);
    t.shot("modes-demo-add-key-foot")?;
    t.app.session = faraday_core::wallet::Session::on(testkit::NET);
    t.app.set_modes(faraday_core::DESKTOP_MODES);
    t.press(Action::Nav(Screen::Home));
    t.shot("modes-testnet-home")?;
    t.press(Action::ModeSheet);
    t.shot("modes-testnet-sheet")?;
    t.press(Action::Cancel);
    t.press(Action::Entry(None));
    t.shot("modes-testnet-add-key")?;
    t.press(Action::Nav(Screen::Tools));
    t.shot("modes-testnet-tools")?;
    t.app.session = faraday_core::wallet::Session::on(faraday_bip::keys::Network::Mainnet);
    t.app.set_modes(faraday_core::STICK_MODES);
    t.press(Action::Nav(Screen::Settings));
    t.shot("modes-mainnet-settings")
}
