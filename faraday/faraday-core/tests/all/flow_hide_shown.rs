//! A flow on the container hides every secret shown on a step when the step
//! is left, by any way out, so the step reopens with it hidden
//! (`docs/FLOW-CONTAINER.md` §3, F104): the eye is off and no secret text
//! is drawn.

use crate::common::type_text;
use faraday_core::testkit::{self, Plain};
use faraday_core::vault::{VaultAction as V, vstep};
use faraday_core::{Action, ChoicesPress, Faraday, Screen};
use faraday_shell_api::{App, BootState, DisplayInfo, EntropyBytes, Event, SecureHardware};

const SECRET: &str = "hidepassword";

fn window(app: &mut Faraday, width: u16, height: u16) {
    app.event(Event::Display(DisplayInfo {
        width,
        height,
        dpi: 160,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
}

/// Frames and ticks a step change's live fade takes to finish.
fn at_rest(app: &mut Faraday) {
    let mut now = app.now_ms();
    for _ in 0..30 {
        now += 30;
        app.event(Event::Tick { now_ms: now });
        let _ = app.frame();
    }
}

/// Whether the screen draws `want`, whatever the spaces between groups.
fn draws(app: &mut Faraday, want: &str) -> bool {
    let _ = app.frame();
    let all: String = app
        .drawn_text_boxes()
        .plain()
        .into_iter()
        .map(|(t, _)| t)
        .collect::<String>()
        .split_whitespace()
        .collect();
    all.contains(want)
}

fn on_make_a_vault(width: u16, height: u16) -> Faraday {
    let mut app = Faraday::new();
    window(&mut app, width, height);
    for i in 0..8u8 {
        app.event(Event::Entropy(EntropyBytes::new([0x30 + i; 32])));
    }
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Create));
    let _ = app.frame();
    testkit::settle(&mut app);
    app.press(Action::Vault(V::CFocus(0, false)));
    type_text(&mut app, SECRET);
    app.press(Action::Vault(V::CShow));
    assert!(draws(&mut app, SECRET), "shown");
    app
}

/// Make a new vault: Next, the pane's Opens on row and Back.
#[test]
fn make_a_new_vault_hides_the_passphrase_by_every_way_out() {
    for way in ["back then forward", "pane row", "sheet row"] {
        let narrow = way == "sheet row";
        let mut app = if narrow {
            on_make_a_vault(480, 640)
        } else {
            on_make_a_vault(1366, 768)
        };
        match way {
            "back then forward" => {
                let _ = app.frame();
                app.event(Event::Back);
                at_rest(&mut app);
                app.event(Event::Forward);
            }
            "pane row" => {
                app.press(Action::Vault(V::CStep(vstep::PRESET)));
                at_rest(&mut app);
                app.press(Action::Vault(V::CStep(vstep::PHRASES)));
            }
            _ => {
                app.press(Action::Choices(ChoicesPress::Open));
                let _ = app.frame();
                app.press(Action::Choices(ChoicesPress::Row(vstep::PRESET)));
                at_rest(&mut app);
                app.press(Action::Choices(ChoicesPress::Open));
                let _ = app.frame();
                app.press(Action::Choices(ChoicesPress::Row(vstep::PHRASES)));
            }
        }
        at_rest(&mut app);
        assert_eq!(
            app.vaults().create.as_ref().and_then(|c| c.open),
            Some(vstep::PHRASES),
            "{way}: back on the passphrases"
        );
        assert!(
            !app.vaults().create.as_ref().unwrap().shown,
            "{way}: eye off"
        );
        assert!(!draws(&mut app, SECRET), "{way}: no secret text drawn");
    }
}

const ABOUT: &str =
    "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

/// My seed makes my wallet: the passphrase's eye, left by Back and opened
/// again, and left by a row of Your choices.
#[test]
fn my_seed_makes_my_wallet_hides_the_passphrase_by_every_way_out() {
    use faraday_core::seed_check::SeedAction as S;
    for way in ["back", "sheet row"] {
        let mut app = Faraday::new();
        window(&mut app, 480, 640);
        testkit::move_to(&mut app.session, faraday_bip::keys::Network::Mainnet).unwrap();
        app.press(Action::Nav(Screen::Catalog));
        let tile = faraday_core::catalog::TILES
            .iter()
            .position(|t| t.name == "My seed makes my wallet")
            .unwrap();
        app.press(Action::Catalog(tile as u8));
        app.press(Action::SeedCheck(S::Type));
        testkit::type_words(&mut app, ABOUT);
        app.press(Action::SeedCheck(S::Words));
        app.press(Action::SeedCheck(S::AddPass));
        app.press(Action::SeedCheck(S::PassField(0)));
        type_text(&mut app, SECRET);
        app.press(Action::PassEye);
        assert!(draws(&mut app, SECRET), "{way}: shown");
        if way == "back" {
            let _ = app.frame();
            app.event(Event::Back);
            let _ = app.frame();
        } else {
            app.press(Action::Choices(ChoicesPress::Open));
            let _ = app.frame();
            app.press(Action::Choices(ChoicesPress::Row(0)));
            at_rest(&mut app);
            app.event(Event::Forward);
        }
        at_rest(&mut app);
        assert!(!draws(&mut app, SECRET), "{way}: no secret text drawn");
    }
}

/// BIP-85: the child shown on the result is hidden once the result is left
/// by Back or a row of Your choices, and stays hidden on return.
#[test]
fn bip85_hides_the_child_by_every_way_out() {
    for way in ["back", "sheet row"] {
        let mut app = Faraday::new();
        window(&mut app, 480, 640);
        testkit::move_to(&mut app.session, testkit::NET).unwrap();
        app.press(Action::Entry(None));
        type_text(&mut app, &testkit::test_words(testkit::TEST_SEEDS[0].0));
        app.press(Action::EntryAdd);
        testkit::keep_published(&mut app);
        app.press(Action::Bip85);
        app.press(Action::PApp(4));
        app.press(Action::PNext);
        app.press(Action::PNext);
        app.press(Action::PShow);
        let _ = app.frame();
        assert!(app.bip85().is_some_and(|b| b.shown), "{way}: shown");
        if way == "back" {
            app.event(Event::Back);
            at_rest(&mut app);
            app.event(Event::Forward);
        } else {
            app.press(Action::Choices(ChoicesPress::Open));
            let _ = app.frame();
            app.press(Action::Choices(ChoicesPress::Row(0)));
            at_rest(&mut app);
            app.press(Action::PStep(3));
        }
        at_rest(&mut app);
        assert!(!app.bip85().is_some_and(|b| b.shown), "{way}: eye off");
    }
}

/// Read a Bitcoin Core wallet: the passphrase shown with the eye is hidden
/// once its step is left by Back.
#[test]
fn read_a_core_wallet_hides_the_passphrase_by_back() {
    use faraday_core::core_wallet::tool::CoreWalletAction as W;
    let file = std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tools/vectors/core-wallet/encrypted.dat"
    ))
    .unwrap();
    let mut app = testkit::started();
    app.session = faraday_core::wallet::Session::on(faraday_bip::keys::Network::Testnet);
    app.storage(faraday_core::StorageEvent::Restored {
        inbox: faraday_core::files_from(vec![("wallet.dat".to_string(), file)]),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app.press(Action::Nav(Screen::Catalog));
    let tile = faraday_core::catalog::TILES
        .iter()
        .position(|t| t.name == "Read a Bitcoin Core wallet")
        .unwrap();
    app.press(Action::Catalog(tile as u8));
    app.press(Action::CoreWallet(W::Focus));
    type_text(&mut app, SECRET);
    app.press(Action::CoreWallet(W::Eye));
    assert!(draws(&mut app, SECRET), "shown");
    let _ = app.frame();
    app.event(Event::Back);
    at_rest(&mut app);
    app.event(Event::Forward);
    at_rest(&mut app);
    assert!(!draws(&mut app, SECRET), "no secret text drawn");
}

/// Convert a seed: the value shown with the eye is hidden once its step is
/// left by Back.
#[test]
fn convert_a_seed_hides_the_value_by_back() {
    use faraday_core::convert::{self, ROWS};
    let mut app = testkit::started();
    testkit::move_to(&mut app.session, testkit::NET).unwrap();
    let c = testkit::compute();
    app.session.keys.add_words(&c, ABOUT, "Test", None).unwrap();
    testkit::keep_published(&mut app);
    app.press(Action::Nav(Screen::Catalog));
    let tile = faraday_core::catalog::TILES
        .iter()
        .position(|t| t.name == "Convert a seed")
        .unwrap();
    app.press(Action::Catalog(tile as u8));
    let row = ROWS.iter().position(|r| r.name == "xprv").unwrap() as u8;
    app.press(Action::Convert(faraday_core::convert::ConvertAction::To(
        row,
    )));
    let _ = app.frame();
    let now = app.now_ms() + 16;
    app.event(Event::Tick { now_ms: now });
    testkit::settle(&mut app);
    app.press(Action::Convert(faraday_core::convert::ConvertAction::Show));
    assert!(convert::shown(&app).is_some(), "shown");
    let _ = app.frame();
    app.event(Event::Back);
    at_rest(&mut app);
    app.event(Event::Forward);
    at_rest(&mut app);
    assert!(convert::shown(&app).is_none(), "hidden again");
}
