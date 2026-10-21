//! The online app (the desktop) starts on mainnet as the demo, which
//! takes only public seeds (F20). Before it goes to a test network, which
//! takes any seed, it says it is for testing only and that this computer
//! is not air-gapped: the network waits for "I understand", and Cancel
//! leaves it on mainnet. Reaching a test network another way brings the
//! same warning up with no way past it but the acknowledgement. The
//! device never shows it. A released Faraday Desktop, which opens on
//! testnet, shows it at start, before any key can be added (F61).

use faraday_core::create::NewKind;
use faraday_core::wallet::Session;
use faraday_core::{
    Action, Faraday, Sheet, StickInfo, StorageCommand, StorageEvent, demo, testkit,
};
use osk_bip::keys::Network;
use osk_shell_api::App;

/// The words of a key that is not a public seed.
const OTHER: &str = "abandon abandon abandon abandon abandon abandon \
                     abandon abandon abandon abandon abandon about";

/// BIP-84's account 0 for [`OTHER`] on `network`, as a wallet.
fn wallet_on(network: Network) -> String {
    let c = testkit::compute();
    let mut s = Session::on(network);
    s.add_words(&c, OTHER, "", None).unwrap();
    let key = NewKind::NativeSegwit
        .key_text(&c, s.keys[0].master())
        .unwrap();
    format!("wpkh({key}/<0;1>/*)")
}

/// The desktop's app: online, on mainnet as the demo.
fn desktop() -> Faraday {
    let c = testkit::compute();
    let mut app = testkit::started();
    app.online = true;
    app.session = Session::default();
    app.session.set_demo(&c);
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

/// To testnet, through the warning.
fn to_testnet(app: &mut Faraday) {
    app.press(Action::Network(Network::Testnet));
    app.press(Action::AirgapUnderstood);
    assert_eq!(app.session.network(), Network::Testnet);
}

#[test]
fn a_test_network_waits_for_i_understand_on_the_desktop() {
    let mut app = desktop();
    let _ = app.frame();
    assert_eq!(app.sheet, None, "mainnet asks nothing");
    app.press(Action::Network(Network::Testnet));
    assert_eq!(app.sheet, Some(Sheet::NotAirgapped));
    assert!(app.session.network().is_mainnet());
    let _ = app.frame();
    assert!(app.offers(Action::AirgapUnderstood));
    assert!(app.offers(Action::Cancel));
    app.press(Action::AirgapUnderstood);
    assert_eq!(app.session.network(), Network::Testnet);
    assert_eq!(app.sheet, None);
    // Once a session: back to mainnet and to a test network again asks
    // nothing.
    app.press(Action::Network(Network::Mainnet));
    app.press(Action::Network(Network::Signet));
    assert_eq!(app.session.network(), Network::Signet);
    assert_eq!(app.sheet, None);
}

#[test]
fn cancel_leaves_the_desktop_on_mainnet() {
    let mut app = desktop();
    app.press(Action::Network(Network::Regtest));
    app.press(Action::Cancel);
    assert!(app.session.network().is_mainnet());
    assert_eq!(app.sheet, None);
    let _ = app.frame();
    assert_eq!(app.sheet, None, "and the warning does not come back");
}

#[test]
fn a_test_wallet_loaded_on_the_desktop_brings_the_warning_up() {
    let mut app = desktop();
    load_wallet(&mut app, &wallet_on(Network::Testnet));
    assert!(!app.session.network().is_mainnet());
    let _ = app.frame();
    assert_eq!(app.sheet, Some(Sheet::NotAirgapped));
    let _ = app.frame();
    assert!(app.offers(Action::AirgapUnderstood));
    assert!(!app.offers(Action::Cancel), "only the acknowledgement");
    app.press(Action::AirgapUnderstood);
    let _ = app.frame();
    assert_eq!(app.sheet, None);
}

#[test]
fn mainnet_on_the_desktop_takes_only_public_seeds() {
    let c = testkit::compute();
    let mut app = desktop();
    assert!(
        app.session
            .add_words(&c, &demo::words("trap"), "", None)
            .is_ok()
    );
    assert!(app.session.add_words(&c, OTHER, "", None).is_err());
    // A test network takes any seed, and mainnet is refused while that
    // key is loaded.
    to_testnet(&mut app);
    assert!(app.session.add_words(&c, OTHER, "", None).is_ok());
    app.press(Action::Network(Network::Mainnet));
    assert_eq!(app.session.network(), Network::Testnet);
}

#[test]
fn a_mainnet_wallet_is_refused_on_a_test_network_holding_another_key() {
    let c = testkit::compute();
    let mut app = desktop();
    to_testnet(&mut app);
    app.session.add_words(&c, OTHER, "", None).unwrap();
    load_wallet(&mut app, &wallet_on(Network::Mainnet));
    assert_eq!(app.session.network(), Network::Testnet);
    assert!(app.session.wallets.is_empty());
}

#[test]
fn the_demo_shows_broken_addresses_on_mainnet_only() {
    let mut app = desktop();
    let address = "bc1qcr8te4kr609gcawutmrza0j4xv80jy8z306fyu";
    assert_ne!(app.session.shown(address), address);
    to_testnet(&mut app);
    let address = "tb1q6rz28mcfaxtmd6v789l9rrlrusdprr9pqcpvkl";
    assert_eq!(app.session.shown(address), address);
}

#[test]
fn a_fixed_network_stays_on_mainnet_whatever_is_loaded() {
    let mut app = desktop();
    app.session.fix_network();
    app.press(Action::Network(Network::Testnet));
    assert!(app.session.network().is_mainnet());
    assert_eq!(app.sheet, None);
    load_wallet(&mut app, &wallet_on(Network::Testnet));
    assert!(app.session.network().is_mainnet());
    assert!(app.session.wallets.is_empty());
    let _ = app.frame();
    assert_eq!(app.sheet, None);
}

#[test]
fn the_device_goes_to_a_test_network_without_the_warning() {
    let c = testkit::compute();
    let mut app = testkit::started();
    app.press(Action::Network(Network::Testnet));
    let _ = app.frame();
    assert_eq!(app.session.network(), Network::Testnet);
    assert_eq!(app.sheet, None);
    assert!(app.session.add_words(&c, OTHER, "", None).is_ok());
}

#[test]
fn the_demo_names_its_learn_page_first_and_the_device_never() {
    let title = |app: &Faraday| app.learn_pages().first().map(|p| p.title);
    let mut app = desktop();
    assert_eq!(title(&app), Some("The demo and its public seeds"));
    to_testnet(&mut app);
    assert_eq!(title(&app), Some("The demo and its public seeds"));
    let device = testkit::started();
    assert!(
        device
            .learn_pages()
            .iter()
            .all(|p| p.title != "The demo and its public seeds")
    );
}

/// The released desktop's app: online, opening on testnet, with the start
/// warning, before its first frame or stick (F61).
fn released_desktop() -> Faraday {
    let mut app = desktop();
    app.session.set_network(Network::Testnet).unwrap();
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
    assert_eq!(app.session.network(), Network::Testnet);
    assert!(app.offers(Action::Scan));
    assert!(app.session.add_words(&c, OTHER, "", None).is_ok());
}

#[test]
fn the_start_warning_is_acknowledged_once_a_session() {
    let mut app = released_desktop();
    app.press(Action::AirgapUnderstood);
    app.press(Action::Network(Network::Mainnet));
    let _ = app.frame();
    app.press(Action::Network(Network::Signet));
    let _ = app.frame();
    assert_eq!(app.session.network(), Network::Signet);
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
fn a_developer_desktop_opens_on_testnet_with_no_warning() {
    let mut app = desktop();
    app.session.set_network(Network::Testnet).unwrap();
    app.without_airgap_warning();
    let _ = app.frame();
    assert_eq!(app.sheet, None);
    assert!(app.offers(Action::Scan));
}
