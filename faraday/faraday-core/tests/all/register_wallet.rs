//! A spend from a wallet of several keys is signed only once the wallet
//! is registered here; a PSBT that carries its cosigners' extended keys
//! (BIP 174's global xpubs) offers **Register this wallet**, a sheet
//! with the wallet's shape, its cosigners, which is yours and its name,
//! and a hold that registers it once its descriptor gives the spend's
//! scripts.

use faraday_bip::policy::WalletPolicy;
use faraday_core::testkit::{self, Kit};
use faraday_core::wallet::step;
use faraday_core::{Action, Faraday, Screen, Sheet, StorageEvent};
use faraday_shell_api::{App, BootState, DisplayInfo, Event, Key, SecureHardware};

use crate::common::{hold, texts};

const FILE: &str = "savings-unsigned.psbt";

fn savings() -> Kit {
    testkit::kits().remove(0)
}

/// Faraday on a desktop panel with `psbt` in Files under [`FILE`] and the
/// first test key loaded, no wallet.
fn with_spend(psbt: &faraday_psbt::Psbt) -> Faraday {
    let mut app = Faraday::new();
    testkit::move_to(&mut app.session, testkit::NET).unwrap();
    app.event(Event::Display(DisplayInfo {
        width: 1366,
        height: 768,
        dpi: 160,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
    app.storage(StorageEvent::Restored {
        inbox: vec![(FILE.to_string(), psbt.to_bytes())],
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app.press(Action::Entry(None));
    for c in testkit::test_words(testkit::TEST_SEEDS[0].0).chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    app.press(Action::EntryAdd);
    assert_eq!(app.session.keys.len(), 1);
    app.press(Action::StartSpend(0));
    assert!(app.spend().is_some());
    open_transaction(&mut app);
    app
}

/// The spend's Transaction card open, where its warnings are.
fn open_transaction(app: &mut Faraday) {
    if app.spend().unwrap().open != Some(step::TRANSACTION) {
        app.press(Action::Step(step::TRANSACTION));
    }
}

fn screen(app: &mut Faraday) -> String {
    texts(app).join(" ")
}

fn blocked(app: &Faraday) -> bool {
    app.spend()
        .unwrap()
        .inspection
        .blocked()
        .contains(&faraday_psbt::WarningKind::WalletNotRegistered)
}

#[test]
fn an_unregistered_multisig_spend_is_blocked_and_signs_nothing() {
    let mut app = with_spend(&testkit::unsigned_with_xpubs(&savings()).unwrap());
    assert!(blocked(&app));
    let shown = screen(&mut app);
    assert!(
        shown.contains(
            "input 0 spends from a wallet of several keys that is not registered here; register the wallet to sign"
        ),
        "{shown}"
    );
    assert!(shown.contains("Register this wallet"), "{shown}");
    let before = app.spend().unwrap().spend.psbt.clone();
    app.press(Action::SignHere);
    assert_eq!(
        app.spend().unwrap().spend.psbt,
        before,
        "nothing is signed for an unregistered wallet"
    );
}

#[test]
fn without_global_xpubs_the_line_says_to_load_the_descriptor() {
    let mut app = with_spend(&testkit::unsigned(&savings()).unwrap());
    assert!(blocked(&app));
    assert!(app.spend().unwrap().inspection.register.is_none());
    let shown = screen(&mut app);
    assert!(!shown.contains("Register this wallet"), "{shown}");
    assert!(
        shown.contains("Load this wallet's descriptor to sign"),
        "{shown}"
    );
    assert!(!app.offers(Action::RegisterOpen));
}

#[test]
fn register_this_wallet_shows_the_wallet_and_the_hold_registers_it_and_it_signs() {
    let kit = savings();
    let mut app = with_spend(&testkit::unsigned_with_xpubs(&kit).unwrap());
    app.press(Action::RegisterOpen);
    assert_eq!(app.sheet, Some(Sheet::Register));
    let shown = screen(&mut app);
    assert!(shown.contains("2 of 3, P2WSH, sorted"), "{shown}");
    let c = testkit::compute();
    let reference = WalletPolicy::parse_any(&c, &kit.descriptor).unwrap();
    for key in reference.keys() {
        let fp = key.fingerprint().unwrap().to_string();
        assert!(
            shown.contains(&fp),
            "the sheet names cosigner {fp}: {shown}"
        );
    }
    assert!(
        shown.contains("48h/1h/0h/2h"),
        "the sheet names the path: {shown}"
    );
    assert!(
        shown.contains("Yours"),
        "the sheet says which key is yours: {shown}"
    );
    assert!(
        shown.contains("Savings Unsigned"),
        "the name from the file: {shown}"
    );
    assert!(shown.contains("Hold to register"), "{shown}");
    assert!(app.session.wallets.is_empty(), "nothing before the hold");

    // Cancel registers nothing.
    app.press(Action::RegisterCancel);
    assert_eq!(app.sheet, None);
    assert!(app.session.wallets.is_empty());

    // The hold held to its end, as a person holds it.
    app.press(Action::RegisterOpen);
    hold(&mut app, Action::RegisterHold);
    assert_eq!(app.sheet, None);
    assert_eq!(app.session.wallets.len(), 1);
    let wallet = &app.session.wallets[0];
    assert_eq!(wallet.name, "Savings Unsigned");
    for change in [false, true] {
        for index in 0..3 {
            assert_eq!(
                wallet
                    .policy()
                    .address_at(&c, testkit::NET, change, index)
                    .unwrap(),
                reference
                    .address_at(&c, testkit::NET, change, index)
                    .unwrap(),
                "the registered wallet's addresses are the descriptor's"
            );
        }
    }
    assert!(
        !blocked(&app),
        "the spend is re-inspected and no longer held"
    );
    assert_eq!(app.spend().unwrap().wallet, Some(0));
    let before = app.spend().unwrap().spend.psbt.clone();
    app.press(Action::SignHere);
    assert_ne!(app.spend().unwrap().spend.psbt, before, "now it signs");
}

#[test]
fn the_name_typed_is_the_wallet_name() {
    let mut app = with_spend(&testkit::unsigned_with_xpubs(&savings()).unwrap());
    app.press(Action::RegisterOpen);
    app.press(Action::RegisterField);
    for _ in 0.."Savings Unsigned".len() {
        app.event(Event::Key(Key::Backspace));
    }
    for c in "Family".chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    // Enter ends the typing and registers nothing: the hold does.
    app.event(Event::Key(Key::Enter));
    assert_eq!(app.sheet, Some(Sheet::Register));
    assert!(app.session.wallets.is_empty());
    app.press(Action::RegisterHold);
    assert_eq!(app.session.wallets.len(), 1);
    assert_eq!(app.session.wallets[0].name, "Family");
}

#[test]
fn global_xpubs_that_do_not_give_the_input_are_refused_by_name() {
    let kit = savings();
    let mut psbt = testkit::unsigned_with_xpubs(&kit).unwrap();
    // The third cosigner's entry carries another account's key under
    // the origin the input states.
    let c = testkit::compute();
    let reference = WalletPolicy::parse_any(&c, &kit.descriptor).unwrap();
    let third = &reference.keys()[2];
    let other = WalletPolicy::parse_any(
        &c,
        &format!("wpkh({}/<0;1>/*)", testkit::key(2, "m/48'/1'/1'/2'")),
    )
    .unwrap();
    let xpubs = &mut psbt.inner_mut().xpub;
    let origin = xpubs.remove(third.xpub()).unwrap();
    xpubs.insert(*other.keys()[0].xpub(), origin);

    let mut app = with_spend(&psbt);
    assert!(app.spend().unwrap().inspection.register.is_some());
    app.press(Action::RegisterOpen);
    app.press(Action::RegisterHold);
    assert!(app.session.wallets.is_empty(), "nothing is registered");
    assert_eq!(app.sheet, Some(Sheet::Register), "the sheet stays with why");
    let shown = screen(&mut app);
    assert!(
        shown.contains("These keys do not give input 0's address"),
        "{shown}"
    );
    assert!(blocked(&app));
}

#[test]
fn a_registered_wallet_spend_opens_on_the_wallet_as_before() {
    let kit = savings();
    let mut app = with_spend(&testkit::unsigned_with_xpubs(&kit).unwrap());
    app.press(Action::Nav(Screen::Wallets));
    let c = testkit::compute();
    app.session
        .add_wallet(
            &c,
            "Savings",
            &kit.descriptor,
            "Test wallet",
            faraday_core::wallet::Origin::Named,
        )
        .unwrap();
    app.press(Action::StartSpend(0));
    open_transaction(&mut app);
    assert!(!blocked(&app));
    let shown = screen(&mut app);
    assert!(!shown.contains("Register this wallet"), "{shown}");
}

/// A coordinator writes a key of its own under the person's fingerprint
/// and path: the wallet gives every script of the spend, but that key
/// is not the person's, so it is never marked theirs, and the hold
/// registers nothing and names the key.
#[test]
fn a_key_under_your_fingerprint_that_is_not_yours_is_refused_by_name() {
    let p = "m/48'/1'/0'/2'";
    let mine = testkit::key(0, p);
    let theirs = testkit::key(2, "m/48'/1'/1'/2'");
    let fake = format!(
        "{}]{}",
        &mine[..mine.find(']').unwrap()],
        &theirs[theirs.find(']').unwrap() + 1..]
    );
    let kit = Kit {
        id: "shared",
        name: "Shared",
        descriptor: format!(
            "wsh(sortedmulti(2,{fake}/<0;1>/*,{}/<0;1>/*,{}/<0;1>/*))",
            testkit::key(1, p),
            testkit::key(2, p)
        ),
        cosigner: None,
        fee: 2_140,
    };
    let mut app = with_spend(&testkit::unsigned_with_xpubs(&kit).unwrap());
    assert!(blocked(&app));
    app.press(Action::RegisterOpen);
    hold(&mut app, Action::RegisterHold);
    assert!(app.session.wallets.is_empty(), "nothing is registered");
    assert_eq!(app.sheet, Some(Sheet::Register));
    let shown = screen(&mut app);
    let fp = &mine[1..9];
    assert!(
        shown.contains(&format!(
            "Key {fp} at 48h/1h/0h/2h has your fingerprint but is not your key"
        )),
        "{shown}"
    );
    assert!(
        !shown.contains("Yours"),
        "the key is never the person's: {shown}"
    );
}

/// A `multi` wallet whose keys sort at the address spent, and a spend
/// with no change to tell the readings apart: the sheet offers both,
/// sorted chosen, and the person's choice is the wallet registered.
#[test]
fn where_the_spend_cannot_tell_sorted_from_listed_the_person_chooses() {
    let c = testkit::compute();
    let p = "m/48'/1'/0'/2'";
    let sorted = WalletPolicy::parse_any(&c, &savings().descriptor).unwrap();
    let orders = [
        [0, 1, 2],
        [0, 2, 1],
        [1, 0, 2],
        [1, 2, 0],
        [2, 0, 1],
        [2, 1, 0],
    ];
    let descriptor = orders
        .iter()
        .map(|o| {
            format!(
                "wsh(multi(2,{}/<0;1>/*,{}/<0;1>/*,{}/<0;1>/*))",
                testkit::key(o[0], p),
                testkit::key(o[1], p),
                testkit::key(o[2], p)
            )
        })
        .find(|d| {
            WalletPolicy::parse_any(&c, d)
                .unwrap()
                .script_at(&c, false, 0)
                .unwrap()
                == sorted.script_at(&c, false, 0).unwrap()
        })
        .expect("one order of the keys is sorted at 0/0");
    let listed = WalletPolicy::parse_any(&c, &descriptor).unwrap();
    let kit = Kit {
        id: "listed",
        name: "Listed",
        descriptor,
        cosigner: None,
        fee: 2_140,
    };
    let mut psbt = testkit::unsigned_with_xpubs(&kit).unwrap();
    // No change: nothing in the spend tells the two readings apart.
    let inner = psbt.inner_mut();
    inner.unsigned_tx.output.truncate(1);
    inner.outputs.truncate(1);
    let mut app = with_spend(&psbt);
    let offer = app.spend().unwrap().inspection.register.clone().unwrap();
    assert_eq!(offer.readings.len(), 2);
    app.press(Action::RegisterOpen);
    let shown = screen(&mut app);
    assert!(shown.contains("Most coordinators sort the keys"), "{shown}");
    assert!(shown.contains("2 of 3, P2WSH, sorted"), "{shown}");
    app.press(Action::RegisterReading(1));
    let shown = screen(&mut app);
    assert!(
        shown.contains("2 of 3, P2WSH, keys in this order"),
        "{shown}"
    );
    hold(&mut app, Action::RegisterHold);
    assert_eq!(app.session.wallets.len(), 1);
    let registered = app.session.wallets[0].policy();
    for i in 0..4 {
        assert_eq!(
            registered.script_at(&c, true, i).unwrap(),
            listed.script_at(&c, true, i).unwrap(),
            "the wallet registered is the one chosen"
        );
    }
}
