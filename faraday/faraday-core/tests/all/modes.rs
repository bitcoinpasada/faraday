//! Modes (F81, `docs/MODES.md`): Practice, Mainnet, Testnet, Signet and Regtest.
//! A mode change is a lock that leaves a fresh session in the mode
//! chosen, and a lock keeps the mode it was in. Practice is the public seeds
//! on testnet: its addresses are real testnet addresses, and its dimmed
//! actions are live on a test network.

use faraday_bip::keys::Network;
use faraday_core::seeds::SeedsAction as S;
use faraday_core::vault::VaultAction as V;
use faraday_core::wallet::Session;
use faraday_core::{
    Action, DESKTOP_MODES, Faraday, Mode, STICK_MODES, Screen, Sheet, StorageEvent, testkit,
};
use faraday_shell_api::{App, Event};

const ABANDON: &str = "abandon abandon abandon abandon abandon abandon abandon abandon abandon \
                       abandon abandon about";

/// The desktop's app: online, in Practice.
fn demo_app() -> Faraday {
    let mut app = testkit::started();
    app.online = true;
    app.session = Session::on(Network::Testnet);
    app.session.set_demo(faraday_core::testkit::demo_samples());
    app.set_modes(DESKTOP_MODES);
    app
}

#[test]
fn choosing_a_mode_with_keys_and_wallets_loaded_leaves_a_fresh_session_in_it() {
    let c = testkit::compute();
    let mut app = testkit::started();
    app.session.keys.add_words(&c, ABANDON, "", None).unwrap();
    faraday_core::testkit::keep_published(&mut app);
    let kit = testkit::kits().into_iter().next().unwrap();
    testkit::move_to(&mut app.session, Network::Testnet).unwrap();
    app.session
        .add_wallet_confirmed(
            &c,
            "w",
            &kit.descriptor,
            "t",
            faraday_core::wallet::Origin::Named,
        )
        .unwrap();
    app.press(Action::Nav(Screen::Settings));
    app.press(Action::Mode(Mode::Signet));
    assert_eq!(app.sheet, Some(Sheet::LockAsk), "the one sheet asks");
    assert!(
        !app.restart_requested(),
        "nothing is wiped before the answer"
    );
    app.press(Action::Lock);
    assert!(app.restart_requested());
    assert!(app.session.keys.is_empty(), "the keys are gone");
    assert!(app.session.wallets.is_empty(), "the wallets are gone");
    assert_eq!(app.mode_after_restart(), Mode::Signet);
}

#[test]
fn a_lock_keeps_the_mode() {
    for mode in [Mode::Mainnet, Mode::Testnet, Mode::Regtest] {
        let mut app = testkit::started();
        testkit::move_to(&mut app.session, mode.network()).unwrap();
        app.press(Action::Lock);
        assert!(app.restart_requested(), "{mode:?}");
        assert_eq!(app.mode_after_restart(), mode);
    }
    let mut app = demo_app();
    app.press(Action::Lock);
    assert!(app.restart_requested());
    assert_eq!(app.mode_after_restart(), Mode::Demo);
}

#[test]
fn the_mode_in_use_is_read_from_the_session() {
    let mut app = testkit::started();
    testkit::move_to(&mut app.session, Network::Mainnet).unwrap();
    assert_eq!(app.mode(), Mode::Mainnet);
    testkit::move_to(&mut app.session, Network::Testnet).unwrap();
    assert_eq!(app.mode(), Mode::Testnet);
    assert_eq!(app.modes(), STICK_MODES);
    assert_eq!(demo_app().mode(), Mode::Demo);
    assert_eq!(demo_app().mode().network(), Network::Testnet);
}

#[test]
fn choosing_the_mode_in_use_or_one_not_offered_does_nothing() {
    let mut app = testkit::started();
    testkit::move_to(&mut app.session, Network::Mainnet).unwrap();
    app.press(Action::Mode(Mode::Mainnet));
    assert!(!app.restart_requested());
    // The stick has no Practice.
    app.press(Action::Mode(Mode::Demo));
    assert!(!app.restart_requested());
    assert_eq!(app.mode_after_restart(), Mode::Mainnet);
    // The desktop has no Mainnet.
    let mut app = demo_app();
    app.press(Action::Mode(Mode::Mainnet));
    assert!(!app.restart_requested());
    assert_eq!(app.mode_after_restart(), Mode::Demo);
}

/// A device with the test vault open and a change in it that has not
/// been sealed: a lock asks first.
fn with_unsealed_change() -> Faraday {
    // On Testnet, where the test seed added below (a published sample) is taken.
    let mut app = testkit::started();
    app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    faraday_core::testkit::vault_speed(&mut app, Some(100));
    app.storage(StorageEvent::Restored {
        inbox: faraday_core::files_from(vec![(
            faraday_text::Text::of("vault.kdbx"),
            testkit::test_vault().unwrap(),
        )]),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app.press(Action::Vault(V::Open(0)));
    crate::common::type_text(&mut app, testkit::VAULT_PASSPHRASE);
    app.press(Action::Vault(V::Unlock));
    for t in 1..60u64 {
        if !app.vaults().open.is_empty() {
            break;
        }
        let _ = app.frame();
        app.event(Event::Tick { now_ms: t * 1000 });
    }
    assert_eq!(app.vaults().open.len(), 1, "the vault did not open");
    app.press(Action::Entry(None));
    crate::common::type_text(&mut app, &testkit::test_words(testkit::TEST_SEEDS[1].0));
    app.press(Action::EntryAdd);
    faraday_core::testkit::keep_published(&mut app);
    let k = app.session.keys.len() - 1;
    app.press(Action::Vault(V::SaveKey(k)));
    assert!(
        app.vaults().open[0].changes > 0,
        "the vault was not changed"
    );
    app.event(Event::Tick { now_ms: 120_000 });
    app
}

#[test]
fn choosing_a_mode_where_a_lock_asks_shows_the_lock_sheet_and_not_now_keeps_the_mode() {
    let mut app = with_unsealed_change();
    app.press(Action::Mode(Mode::Mainnet));
    assert_eq!(app.sheet, Some(Sheet::LockAsk));
    assert!(!app.restart_requested(), "nothing is wiped yet");
    assert_eq!(app.vaults().open.len(), 1);
    assert_eq!(app.mode_after_restart(), Mode::Mainnet);
    // Not now: the person stays where they were, in the mode in use.
    app.press(Action::Cancel);
    assert_eq!(app.sheet, None);
    assert_eq!(app.mode_after_restart(), Mode::Testnet);
    // Locking anyway ends in the mode chosen.
    app.press(Action::Mode(Mode::Mainnet));
    app.press(Action::LockUnsealed);
    assert!(app.restart_requested());
    assert_eq!(app.mode_after_restart(), Mode::Mainnet);
}

#[test]
fn demo_addresses_are_testnet_addresses_a_testnet_wallet_sits_beside() {
    let c = testkit::compute();
    let mut app = demo_app();
    app.press(Action::Entry(None));
    crate::common::type_text(&mut app, &faraday_core::testkit::test_words("bacon"));
    app.press(Action::EntryAdd);
    faraday_core::testkit::keep_published(&mut app);
    let fp = app.session.keys.first().unwrap().fingerprint.0;
    app.press(Action::KeyWallet(fp, 1));
    app.press(Action::Seeds(S::Make));
    let ours = &app.session.wallets[app.session.wallets.len() - 1];
    let address = String::from(&*app.session.address(&c, ours, false, 0));
    assert!(address.starts_with("tb1q"), "{address}");
    // A real testnet wallet from elsewhere loads in the same session.
    let kit = testkit::kits().into_iter().next().unwrap();
    app.session
        .add_wallet_confirmed(
            &c,
            "t",
            &kit.descriptor,
            "t",
            faraday_core::wallet::Origin::Named,
        )
        .unwrap();
    assert_eq!(app.mode(), Mode::Demo);
    assert_eq!(app.session.wallets.len(), 2);
}

#[test]
fn demo_refuses_a_key_that_is_not_public_and_takes_a_public_one() {
    let c = testkit::compute();
    let mut app = demo_app();
    assert!(app.session.keys.add_words(&c, ABANDON, "", None).is_err());
    assert!(
        app.session
            .keys
            .add_words(&c, &faraday_core::testkit::test_words("gas"), "", None)
            .is_ok()
    );
    // A passphrase that does not make a public key: refused.
    assert!(
        app.session
            .keys
            .add_words_with(
                &c,
                &faraday_core::testkit::test_words("trap"),
                "not public",
                "",
                None
            )
            .is_err()
    );
}

#[test]
fn a_testnet_wallet_moves_an_empty_stick_session_to_testnet_and_a_loaded_key_stops_it() {
    let c = testkit::compute();
    let kit = testkit::kits().into_iter().next().unwrap();
    let mut empty = testkit::started();
    testkit::move_to(&mut empty.session, Network::Mainnet).unwrap();
    empty
        .session
        .add_wallet_confirmed(
            &c,
            "t",
            &kit.descriptor,
            "t",
            faraday_core::wallet::Origin::Named,
        )
        .unwrap();
    assert_eq!(empty.mode(), Mode::Testnet);

    let mut keyed = testkit::started();
    testkit::move_to(&mut keyed.session, Network::Mainnet).unwrap();
    // BIP-39's second vector: published but no sample, which Mainnet takes.
    keyed
        .session
        .keys
        .add_words(
            &c,
            "legal winner thank year wave sausage worth useful legal winner thank yellow",
            "",
            None,
        )
        .unwrap();
    faraday_core::testkit::keep_published(&mut keyed);
    let refused = keyed.session.add_wallet_confirmed(
        &c,
        "t",
        &kit.descriptor,
        "t",
        faraday_core::wallet::Origin::Named,
    );
    assert!(refused.is_err());
    assert_eq!(keyed.mode(), Mode::Mainnet);
}

#[test]
fn the_actions_the_demo_dims_are_live_on_testnet() {
    let demo = demo_app();
    let testnet = testkit::started();
    for a in [
        Action::BXor(2),
        Action::PShow,
        Action::LShow,
        Action::SScanVault,
    ] {
        assert!(demo.demo_refuses(a), "{a:?} is open in Practice");
        assert!(!testnet.demo_refuses(a), "{a:?} is dimmed on Testnet");
    }
    assert!(!demo.makes_secrets());
    assert!(testnet.makes_secrets());
}

#[test]
fn demo_hands_out_a_sample_for_new_key_and_testnet_does_not() {
    let demo = demo_app();
    assert!(demo.session.keys.next_handout(24).is_some());
    let testnet = testkit::started();
    assert!(testnet.session.keys.next_handout(24).is_none());
}

/// `app` shown on a window: wide (1366 x 768) or the Pi's compact panel
/// (480 x 640).
fn shown_on(app: &mut Faraday, compact: bool) {
    let (width, height, dpi) = if compact {
        (480, 640, 286)
    } else {
        (1366, 768, 160)
    };
    app.event(Event::Display(faraday_shell_api::DisplayInfo {
        width,
        height,
        dpi,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: faraday_shell_api::SecureHardware::None,
        boot: faraday_shell_api::BootState::Unknown,
        memory_mib: None,
    }));
}

const PAGES: [Screen; 5] = [
    Screen::Home,
    Screen::Wallets,
    Screen::Files,
    Screen::Vaults,
    Screen::Settings,
];

#[test]
fn off_mainnet_every_page_offers_the_pill_and_it_opens_the_sheet() {
    for compact in [false, true] {
        for demo in [true, false] {
            let mut app = if demo { demo_app() } else { testkit::started() };
            if !demo {
                testkit::move_to(&mut app.session, Network::Signet).unwrap();
            }
            shown_on(&mut app, compact);
            for page in PAGES {
                app.press(Action::Nav(page));
                assert!(
                    crate::common::offered(&mut app, Action::ModeSheet),
                    "{page:?} compact {compact} demo {demo}"
                );
            }
            app.press(Action::Nav(Screen::Home));
            app.press(Action::ModeSheet);
            assert_eq!(app.sheet, Some(Sheet::Mode));
        }
    }
}

#[test]
fn the_pill_names_the_mode_and_in_demo_its_network() {
    let mut app = demo_app();
    shown_on(&mut app, false);
    assert!(crate::common::has(&mut app, "Practice · Testnet"));
    let mut app = testkit::started();
    testkit::move_to(&mut app.session, Network::Regtest).unwrap();
    shown_on(&mut app, true);
    assert!(crate::common::has(&mut app, "Regtest"));
}

#[test]
fn on_mainnet_no_pill_shows_and_settings_mode_row_opens_the_sheet_on_the_small_panel() {
    for compact in [false, true] {
        let mut app = testkit::started();
        testkit::move_to(&mut app.session, Network::Mainnet).unwrap();
        shown_on(&mut app, compact);
        for page in [Screen::Home, Screen::Wallets, Screen::Files] {
            app.press(Action::Nav(page));
            assert!(
                !crate::common::offered(&mut app, Action::ModeSheet),
                "{page:?} compact {compact}"
            );
        }
        app.press(Action::Nav(Screen::Settings));
        if compact {
            assert!(crate::common::offered(&mut app, Action::ModeSheet));
            app.press(Action::ModeSheet);
            assert_eq!(app.sheet, Some(Sheet::Mode));
        } else {
            // A wide window shows the modes in the card itself.
            assert!(!crate::common::offered(&mut app, Action::ModeSheet));
            assert!(crate::common::offered(
                &mut app,
                Action::Mode(Mode::Testnet)
            ));
        }
    }
}

#[test]
fn the_sheet_offers_each_mode_marks_the_one_in_use_and_not_now_closes_it() {
    for compact in [false, true] {
        let mut app = demo_app();
        shown_on(&mut app, compact);
        app.press(Action::ModeSheet);
        for &m in DESKTOP_MODES {
            assert!(crate::common::offered(&mut app, Action::Mode(m)), "{m:?}");
        }
        assert!(!crate::common::offered(
            &mut app,
            Action::Mode(Mode::Mainnet)
        ));
        let names = crate::common::texts(&mut app);
        for want in [
            "Mode", "Practice", "Testnet", "Signet", "Regtest", "Not now",
        ] {
            assert!(names.iter().any(|t| t == want), "{want}: {names:?}");
        }
        assert!(!names.iter().any(|t| t == "Mainnet"));
        assert!(crate::common::offered(&mut app, Action::Cancel));
        app.press(Action::Cancel);
        assert_eq!(app.sheet, None);
        assert_eq!(app.mode(), Mode::Demo);
    }
}

#[test]
fn escape_closes_the_sheet_and_choosing_a_row_locks_into_its_mode() {
    let mut app = demo_app();
    shown_on(&mut app, false);
    app.press(Action::ModeSheet);
    app.event(Event::Key(faraday_shell_api::Key::Escape));
    assert_eq!(app.sheet, None);
    app.press(Action::ModeSheet);
    app.press(Action::Mode(Mode::Regtest));
    // The sheet closed for the one question, which is asked once.
    assert_eq!(app.sheet, Some(Sheet::LockAsk));
    assert!(!app.restart_requested());
    app.press(Action::Lock);
    assert!(app.restart_requested());
    assert_eq!(app.mode_after_restart(), Mode::Regtest);
}

#[test]
fn settings_mode_card_on_a_wide_window_shows_every_mode_and_presses_the_ones_offered() {
    let mut app = demo_app();
    shown_on(&mut app, false);
    app.press(Action::Nav(Screen::Settings));
    let names = crate::common::texts(&mut app);
    for want in ["Mode", "Practice", "Testnet", "Signet", "Regtest"] {
        assert!(names.iter().any(|t| t == want), "{want}: {names:?}");
    }
    // The desktop has no Mainnet, as the sheet lists none.
    assert!(!names.iter().any(|t| t == "Mainnet"), "{names:?}");
    for m in [Mode::Testnet, Mode::Signet, Mode::Regtest] {
        assert!(crate::common::offered(&mut app, Action::Mode(m)), "{m:?}");
    }
    assert!(!crate::common::offered(
        &mut app,
        Action::Mode(Mode::Mainnet)
    ));
}

#[test]
fn pressing_another_mode_in_settings_asks_in_one_sheet_and_not_now_changes_nothing() {
    let mut app = demo_app();
    shown_on(&mut app, false);
    app.press(Action::Nav(Screen::Settings));
    // The mode in use asks nothing.
    app.press(Action::Mode(Mode::Demo));
    assert_eq!(app.sheet, None);
    app.press(Action::Mode(Mode::Testnet));
    assert_eq!(app.sheet, Some(Sheet::LockAsk));
    assert!(crate::common::has(&mut app, "Change to Testnet?"));
    assert!(crate::common::has(&mut app, "Not now"));
    assert!(crate::common::has(&mut app, "Lock and use Testnet"));
    assert!(crate::common::offered(&mut app, Action::Cancel));
    assert!(crate::common::offered(&mut app, Action::Lock));
    app.press(Action::Cancel);
    assert_eq!(app.sheet, None);
    assert!(!app.restart_requested());
    assert_eq!(app.mode(), Mode::Demo);
    assert_eq!(app.mode_after_restart(), Mode::Demo);
}

#[test]
fn lock_and_use_from_settings_ends_in_the_mode_with_the_session_a_lock_leaves() {
    let c = testkit::compute();
    let mut app = testkit::started();
    shown_on(&mut app, false);
    app.session.keys.add_words(&c, ABANDON, "", None).unwrap();
    faraday_core::testkit::keep_published(&mut app);
    app.press(Action::Nav(Screen::Settings));
    app.press(Action::Mode(Mode::Regtest));
    assert!(crate::common::has(&mut app, "Change to Regtest?"));
    app.press(Action::Lock);
    assert!(app.restart_requested());
    assert_eq!(app.mode_after_restart(), Mode::Regtest);
    assert!(app.session.keys.is_empty());
}

#[test]
fn a_session_holding_something_unkept_asks_in_the_same_sheet_titled_by_the_mode() {
    let mut app = with_unsealed_change();
    app.press(Action::Mode(Mode::Mainnet));
    assert_eq!(app.sheet, Some(Sheet::LockAsk));
    assert!(crate::common::has(&mut app, "Change to Mainnet?"));
    assert!(crate::common::has(&mut app, "Not now"));
    assert!(!app.restart_requested());
}

/// Nothing is pending after the change sheet ended some way other than
/// Lock and use: a later plain Lock keeps the mode in use and asks
/// nothing about a change.
fn no_change_is_pending(app: &mut Faraday, in_use: Mode) {
    assert_eq!(app.sheet, None);
    assert_eq!(app.mode_after_restart(), in_use);
    app.press(Action::LockAsk);
    assert!(!crate::common::has_part(app, "Change to"));
    assert!(!crate::common::has_part(app, "Lock and use"));
    app.press(Action::Lock);
    assert!(app.restart_requested());
    assert_eq!(app.mode_after_restart(), in_use);
}

#[test]
fn escape_on_the_change_sheet_leaves_no_mode_pending() {
    let mut app = demo_app();
    app.press(Action::Mode(Mode::Testnet));
    assert_eq!(app.sheet, Some(Sheet::LockAsk));
    app.event(Event::Key(faraday_shell_api::Key::Escape));
    no_change_is_pending(&mut app, Mode::Demo);
}

fn a_stick() -> StorageEvent {
    StorageEvent::Sticks(vec![faraday_core::StickInfo {
        make: Default::default(),
        serial: None,
        id: faraday_text::Text::of("stick"),
        label: faraday_text::Text::of("STICK"),
        boot: false,
        files: Vec::new(),
        vaults: Vec::new(),
    }])
}

/// No change is pending, with a session that holds something: the mode a
/// restart would start in is the one in use, and a plain Lock asks no
/// question about a change.
fn nothing_pending_while_holding(app: &mut Faraday, in_use: Mode) {
    assert_eq!(app.mode_after_restart(), in_use);
    app.press(Action::LockAsk);
    assert!(!crate::common::has_part(app, "Change to"));
    assert!(!crate::common::has_part(app, "Lock and use Testnet"));
    assert_eq!(app.mode_after_restart(), in_use);
}

#[test]
fn a_stick_pulled_under_the_change_sheet_leaves_no_mode_pending() {
    let mut app = with_unsealed_change();
    app.press(Action::Mode(Mode::Mainnet));
    assert_eq!(app.sheet, Some(Sheet::LockAsk));
    app.storage(a_stick());
    let _ = app.frame();
    assert_eq!(app.sheet, Some(Sheet::Lock), "the stick's Lock sheet");
    assert_eq!(app.mode_after_restart(), Mode::Testnet, "no change named");
    app.storage(StorageEvent::Sticks(Vec::new()));
    let _ = app.frame();
    assert_eq!(app.sheet, None);
    nothing_pending_while_holding(&mut app, Mode::Testnet);
}

/// A device with a vault open and a change in it that can be sealed.
fn with_sealable_change() -> Faraday {
    let mut app = with_unsealed_change();
    app.event(Event::Entropy(faraday_shell_api::EntropyBytes::new(
        [0x21; 32],
    )));
    crate::common::top_up(&mut app);
    app
}

#[test]
fn lock_and_use_the_stick_after_a_stick_arrived_under_the_change_sheet_keeps_the_mode() {
    let mut app = with_sealable_change();
    app.press(Action::Mode(Mode::Mainnet));
    assert_eq!(app.sheet, Some(Sheet::LockAsk));
    app.storage(a_stick());
    let _ = app.frame();
    assert_eq!(app.sheet, Some(Sheet::Lock));
    app.press(Action::Lock);
    testkit::settle(&mut app);
    assert!(app.restart_requested());
    assert_eq!(app.mode_after_restart(), Mode::Testnet);
}

#[test]
fn add_randomness_then_home_leaves_no_mode_pending() {
    let mut app = with_unsealed_change();
    app.press(Action::Mode(Mode::Mainnet));
    assert_eq!(app.sheet, Some(Sheet::LockAsk));
    app.press(Action::Own(faraday_core::fresh::page::Act::Open));
    assert_eq!(app.screen, Screen::Randomness);
    app.press(Action::Nav(Screen::Home));
    assert_eq!(app.sheet, None);
    nothing_pending_while_holding(&mut app, Mode::Testnet);
}

#[test]
fn save_then_cancel_in_the_change_sheets_list_leaves_no_mode_pending() {
    for save in [false, true] {
        let mut app = with_unsealed_change();
        app.press(Action::Entry(None));
        crate::common::type_text(&mut app, &testkit::test_words(testkit::TEST_SEEDS[2].0));
        app.press(Action::EntryAdd);
        faraday_core::testkit::keep_published(&mut app);
        let k = app.session.keys.len() - 1;
        let of = faraday_vault::child::ChildOf {
            parent: [1, 2, 3, 4],
            language: 0,
            words: 24,
            index: 7,
        };
        assert!(app.session.keys.set_child_of(k, Some(of)));
        app.press(Action::Mode(Mode::Mainnet));
        assert_eq!(app.sheet, Some(Sheet::LockAsk));
        app.press(Action::Vault(V::SaveKey(k)));
        assert_eq!(app.sheet, Some(Sheet::SaveKeeps));
        app.press(Action::Vault(if save {
            V::KeepSave
        } else {
            V::KeepCancel
        }));
        assert_eq!(app.sheet, None);
        nothing_pending_while_holding(&mut app, Mode::Testnet);
    }
}

#[test]
fn a_lock_agreed_on_the_change_sheet_ends_in_the_mode_even_over_many_frames() {
    let mut app = with_sealable_change();
    app.press(Action::Mode(Mode::Mainnet));
    assert_eq!(app.sheet, Some(Sheet::LockAsk));
    testkit::hold_worker();
    testkit::press_on_worker(&mut app, Action::Lock);
    assert!(!app.restart_requested(), "the seal is under way");
    // Presses while the seal runs do not undo the change.
    testkit::press_on_worker(&mut app, Action::Nav(Screen::Settings));
    let _ = app.frame();
    testkit::press_on_worker(&mut app, Action::Nav(Screen::Home));
    // A stick that comes meanwhile does not undo it.
    app.storage(a_stick());
    let _ = app.frame();
    testkit::release_worker();
    testkit::settle(&mut app);
    assert!(app.restart_requested());
    assert_eq!(app.mode_after_restart(), Mode::Mainnet);
}

#[test]
fn a_stick_that_comes_while_the_seal_runs_leaves_the_agreed_mode() {
    let mut app = with_sealable_change();
    app.press(Action::Mode(Mode::Mainnet));
    assert_eq!(app.sheet, Some(Sheet::LockAsk));
    testkit::hold_worker();
    testkit::press_on_worker(&mut app, Action::Lock);
    assert!(!app.restart_requested(), "the seal is under way");
    app.storage(a_stick());
    let _ = app.frame();
    testkit::release_worker();
    testkit::settle(&mut app);
    assert!(app.restart_requested());
    assert_eq!(app.mode_after_restart(), Mode::Mainnet);
}

#[test]
fn a_stick_that_comes_after_the_lock_is_agreed_leaves_the_agreed_mode() {
    let mut app = with_sealable_change();
    app.press(Action::Mode(Mode::Mainnet));
    app.press(Action::Lock);
    testkit::settle(&mut app);
    assert!(app.restart_requested());
    app.storage(a_stick());
    let _ = app.frame();
    assert_eq!(app.mode_after_restart(), Mode::Mainnet);
}

#[test]
fn a_waiting_lock_is_dropped_when_a_stick_comes() {
    let mut app = with_unsealed_change();
    app.press(Action::Mode(Mode::Mainnet));
    app.press(Action::Lock);
    assert!(!app.restart_requested(), "the lock waits for randomness");
    app.storage(a_stick());
    let _ = app.frame();
    assert_eq!(app.sheet, Some(Sheet::Lock));
    app.event(Event::Entropy(faraday_shell_api::EntropyBytes::new(
        [0x21; 32],
    )));
    crate::common::top_up(&mut app);
    testkit::settle(&mut app);
    assert!(!app.restart_requested(), "the stick's sheet is unanswered");
}

#[test]
fn the_boot_sticks_list_over_the_change_sheet_leaves_no_mode_pending() {
    let mut app = demo_app();
    app.storage(a_stick());
    app.press(Action::Mode(Mode::Testnet));
    assert_eq!(app.sheet, Some(Sheet::LockAsk));
    let mut boot = match a_stick() {
        StorageEvent::Sticks(mut v) => v.remove(0),
        _ => unreachable!(),
    };
    boot.boot = true;
    boot.id = faraday_text::Text::of("boot");
    boot.files = vec![(faraday_text::Text::of("vault.kdbx"), 4)];
    app.storage(StorageEvent::Sticks(vec![boot]));
    let _ = app.frame();
    assert_ne!(app.sheet, Some(Sheet::LockAsk), "the list took the sheet");
    assert!(!crate::common::has_part(&mut app, "Change to"));
    app.press(Action::LockAsk);
    assert!(!crate::common::has_part(&mut app, "Change to"));
    assert!(!crate::common::has_part(&mut app, "Lock and use Testnet"));
}

#[test]
fn the_idle_lock_under_the_change_sheet_keeps_the_mode_in_use() {
    let mut app = with_unsealed_change();
    app.press(Action::Mode(Mode::Mainnet));
    assert_eq!(app.sheet, Some(Sheet::LockAsk));
    app.event(Event::Tick {
        now_ms: 120_000 + 31 * 60_000,
    });
    assert!(app.restart_requested());
    assert_eq!(app.mode_after_restart(), Mode::Testnet);
}

#[test]
fn with_a_stick_attached_a_press_on_a_mode_shows_the_one_change_sheet() {
    let mut app = demo_app();
    app.storage(a_stick());
    app.press(Action::Mode(Mode::Testnet));
    assert_eq!(app.sheet, Some(Sheet::LockAsk));
    assert!(crate::common::has(&mut app, "Change to Testnet?"));
    assert!(crate::common::has(&mut app, "Lock and use Testnet"));
}
