//! The online app (the desktop) opens in Demo, which takes only public
//! seeds (F20, F81). Before a release build leaves Demo for a test
//! network, which takes any seed, it says it is for testing only and that
//! this computer is not air-gapped: the change waits for "I understand",
//! and Cancel leaves it in Demo. The device never shows it. A released
//! Faraday Desktop that opens behind the warning shows it at start,
//! before any key can be added (F61).

use faraday_bip::keys::Network;
use faraday_core::create::NewKind;
use faraday_core::testkit::KindText;
use faraday_core::wallet::Session;
use faraday_core::{
    Action, DESKTOP_MODES, Faraday, Mode, Sheet, StickInfo, StorageCommand, StorageEvent, demo,
    testkit,
};
use faraday_shell_api::App;

/// The words of a key that is not a public seed.
const OTHER: &str = "abandon abandon abandon abandon abandon abandon \
                     abandon abandon abandon abandon abandon about";

/// BIP-84's account 0 for [`OTHER`] on `network`, as a wallet.
fn wallet_on(network: Network) -> String {
    let c = testkit::compute();
    let mut s = Session::on(network);
    s.keys.add_words(&c, OTHER, "", None).unwrap();
    let key = NewKind::NativeSegwit
        .key_text(&c, s.keys.master_for_tests(0))
        .unwrap();
    format!("wpkh({key}/<0;1>/*)")
}

/// The desktop's app: online, in Demo (the public seeds on testnet), a
/// developer's build with no warning.
fn desktop() -> Faraday {
    let c = testkit::compute();
    let mut app = testkit::started();
    app.online = true;
    app.session = Session::on(Network::Testnet);
    app.session.set_demo(&c);
    app.set_modes(DESKTOP_MODES);
    app
}

/// A file put in the Inbox, and loaded as a wallet.
fn load_wallet(app: &mut Faraday, text: &str) {
    app.storage(StorageEvent::Restored {
        inbox: vec![("wallet.txt".to_string(), text.as_bytes().to_vec())],
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app.press(Action::LoadWallet(0));
}

#[test]
fn a_mode_change_on_the_desktop_shows_no_warning() {
    for mode in [Mode::Testnet, Mode::Signet, Mode::Regtest] {
        let mut app = desktop();
        app.press(Action::Mode(mode));
        assert_eq!(app.sheet, None, "{mode:?}");
        let _ = app.frame();
        assert_eq!(app.sheet, None, "{mode:?}");
        assert!(app.restart_requested());
        assert_eq!(app.mode_after_restart(), mode);
    }
}

#[test]
fn a_mode_not_offered_changes_nothing() {
    let mut app = desktop();
    app.press(Action::Mode(Mode::Mainnet));
    assert_eq!(app.sheet, None);
    assert!(!app.restart_requested());
}

#[test]
fn demo_takes_only_public_seeds_and_a_test_mode_takes_any() {
    let c = testkit::compute();
    let mut app = desktop();
    assert!(
        app.session
            .keys
            .add_words(&c, &demo::words("trap"), "", None)
            .is_ok()
    );
    assert!(app.session.keys.add_words(&c, OTHER, "", None).is_err());
    // The same key under a passphrase is another key: not public.
    assert!(
        app.session
            .keys
            .add_words_with(&c, &demo::words("zebra"), "x", "", None)
            .is_err()
    );
    // A testnet session that is not Demo takes any seed.
    let mut other = testkit::started();
    assert!(other.session.keys.add_words(&c, OTHER, "", None).is_ok());
}

#[test]
fn a_mainnet_wallet_is_refused_in_demo_even_where_mainnet_is_offered() {
    let mut app = desktop();
    app.set_modes(faraday_core::STICK_MODES);
    load_wallet(&mut app, &wallet_on(Network::Mainnet));
    assert_eq!(app.mode(), Mode::Demo);
    assert!(app.session.wallets.is_empty());
    assert!(app.toast_text().is_some());
}

#[test]
fn a_mainnet_wallet_is_refused_on_a_test_network_holding_another_key() {
    let c = testkit::compute();
    let mut app = testkit::started();
    app.set_modes(DESKTOP_MODES);
    testkit::move_to(&mut app.session, Network::Testnet).unwrap();
    app.session.keys.add_words(&c, OTHER, "", None).unwrap();
    load_wallet(&mut app, &wallet_on(Network::Mainnet));
    assert_eq!(app.session.network(), Network::Testnet);
    assert!(app.session.wallets.is_empty());
}

#[test]
fn a_fixed_network_stays_whatever_is_chosen_or_loaded() {
    let mut app = desktop();
    app.session.keys.fix_network();
    app.press(Action::Mode(Mode::Signet));
    assert!(!app.restart_requested());
    assert_eq!(app.mode_after_restart(), Mode::Demo);
    assert_eq!(app.sheet, None);
    load_wallet(&mut app, &wallet_on(Network::Mainnet));
    assert!(app.session.wallets.is_empty());
    let _ = app.frame();
    assert_eq!(app.sheet, None);
}

#[test]
fn the_device_changes_mode_without_the_warning() {
    let c = testkit::compute();
    let mut app = testkit::started();
    app.press(Action::Mode(Mode::Signet));
    let _ = app.frame();
    assert_eq!(app.sheet, None);
    assert_eq!(app.mode_after_restart(), Mode::Signet);
    assert!(app.session.keys.add_words(&c, OTHER, "", None).is_ok());
}

#[test]
fn demo_names_its_learn_page_first_and_the_device_never() {
    let title = |app: &Faraday| app.learn_pages().first().map(|p| p.title);
    let app = desktop();
    assert_eq!(title(&app), Some("The demo and its public seeds"));
    let device = testkit::started();
    assert!(
        device
            .learn_pages()
            .iter()
            .all(|p| p.title != "The demo and its public seeds")
    );
}

/// The released desktop's app: online, with the start warning, before its
/// first frame or stick (F61).
fn released_desktop() -> Faraday {
    let mut app = desktop();
    app.with_start_warning();
    app
}

#[test]
fn the_released_desktop_warns_at_start_before_any_key_can_be_added() {
    let c = testkit::compute();
    let mut app = released_desktop();
    let _ = app.frame();
    assert_eq!(app.sheet, Some(Sheet::NotAirgapped));
    assert!(app.offers(Action::AirgapUnderstood));
    assert!(!app.offers(Action::Cancel), "only the acknowledgement");
    assert!(
        !app.offers(Action::Scan),
        "no Scan, or anything else, under it"
    );
    app.press(Action::AirgapUnderstood);
    let _ = app.frame();
    assert_eq!(app.sheet, None);
    assert_eq!(app.mode(), Mode::Demo);
    assert!(app.offers(Action::Scan));
    assert!(
        app.session
            .keys
            .add_words(&c, &demo::words("zebra"), "", None)
            .is_ok()
    );
}

#[test]
fn the_start_warning_is_acknowledged_once_a_session() {
    let mut app = released_desktop();
    app.press(Action::AirgapUnderstood);
    app.press(Action::Mode(Mode::Signet));
    let _ = app.frame();
    assert_eq!(app.mode_after_restart(), Mode::Signet);
    assert_eq!(app.sheet, None);
}

#[test]
fn the_boot_import_waits_for_the_start_warning() {
    let mut app = released_desktop();
    app.storage(StorageEvent::Sticks(vec![StickInfo {
        serial: None,
        id: "boot".into(),
        label: "FARADAY".into(),
        boot: true,
        files: vec![("words.txt".into(), OTHER.len() as u64)],
    }]));
    while let Some(c) = app.poll_storage() {
        if let StorageCommand::Read { stick, name } = c {
            app.storage(StorageEvent::Read {
                stick,
                name,
                bytes: OTHER.as_bytes().to_vec(),
            });
        }
    }
    let _ = app.frame();
    assert_eq!(app.sheet, Some(Sheet::NotAirgapped));
    assert!(app.session.keys.is_empty());
    app.press(Action::AirgapUnderstood);
    assert_eq!(app.sheet, Some(Sheet::Import));
}

#[test]
fn a_developer_desktop_opens_with_no_warning() {
    let mut app = desktop();
    app.without_airgap_warning();
    let _ = app.frame();
    assert_eq!(app.sheet, None);
    assert!(app.offers(Action::Scan));
}
