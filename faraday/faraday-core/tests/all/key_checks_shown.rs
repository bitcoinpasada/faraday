//! A wallet of several keys says on its receive addresses and on the
//! spend's review when its keys are not checked.

use crate::common::{add_key, offered, texts};
use faraday_core::spend_backup::{Open, SpendBackupAction as F, page};
use faraday_core::testkit::{self, Kit};
use faraday_core::wallet::{KeyBlockOf, step};
use faraday_core::{Action, Code, Faraday, Screen, StorageEvent};
use faraday_shell_api::{App, BootState, DisplayInfo, Event, Key, SecureHardware};

fn multisig() -> Kit {
    let p = "m/48'/1'/0'/0'";
    Kit {
        id: "shown",
        name: "Shown",
        descriptor: format!(
            "wsh(sortedmulti(2,{}/<0;1>/*,{}/<0;1>/*,{}/<0;1>/*))",
            testkit::key(0, p),
            testkit::key(1, p),
            testkit::key(2, p)
        ),
        cosigner: None,
        fee: 1_820,
    }
}

fn single() -> Kit {
    testkit::kits()
        .into_iter()
        .find(|k| k.id == "spending")
        .unwrap()
}

/// `kit`'s wallet added, a spend from it open on the review.
fn reviewing(kit: &Kit, size: (u16, u16, u16)) -> Faraday {
    let psbt = testkit::unsigned(kit).unwrap();
    let mut app = testkit::started();
    app.event(Event::Display(DisplayInfo {
        width: size.0,
        height: size.1,
        dpi: size.2,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
    app.storage(StorageEvent::Restored {
        inbox: faraday_core::files_from(vec![
            (
                faraday_text::Text::of("a-wallet.txt"),
                kit.descriptor.clone().into_bytes(),
            ),
            (faraday_text::Text::of("spend.psbt"), psbt.to_bytes()),
        ]),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    testkit::load_wallet(&mut app, 0);
    add_key(&mut app, "bacon");
    let at = app.inbox().iter().position(|i| i.name == "spend.psbt");
    app.press(Action::StartSpend(at.unwrap()));
    if app.spend().unwrap().open != Some(step::TRANSACTION) {
        app.press(Action::Step(step::TRANSACTION));
    }
    app
}

fn shown(app: &mut Faraday) -> String {
    texts(app).join(" ")
}

/// Ticks every key of the wallet the person can.
fn tick_all(app: &mut Faraday) {
    for k in 0..3 {
        app.press(Action::KeyTick(KeyBlockOf::Wallet(0), k));
    }
}

// Tall, so that nothing of a long card is below the fold.
const DESKTOP: (u16, u16, u16) = (1280, 3200, 160);
const PI: (u16, u16, u16) = (480, 3200, 286);

#[test]
fn receive_cautions_while_keys_are_not_checked() {
    for size in [DESKTOP, PI] {
        let mut app = reviewing(&multisig(), size);
        assert!(app.session.wallets[0].keys_unchecked() > 0);
        app.press(Action::OpenWallet(0));
        let s = shown(&mut app);
        assert!(s.contains("keys of this wallet are not checked"), "{s}");
        assert!(s.contains("tb1"), "the address still shows: {s}");
        let behind = cautions(&mut app);
        assert_eq!(behind, 1, "the wallet card's own line");
        app.press(Action::ShowCode(Code::Receive(0, 0)));
        assert!(app.qr().is_some());
        // One line for the sheet, and one more for the card where the
        // card is still drawn behind it.
        let card_behind = texts(&mut app)
            .iter()
            .any(|t| t == "The wallet's first addresses");
        assert_eq!(
            cautions(&mut app),
            1 + usize::from(card_behind),
            "the sheet draws its own line, card behind: {card_behind}"
        );
    }
}

#[test]
fn receive_says_nothing_once_every_key_is_checked() {
    let mut app = reviewing(&multisig(), DESKTOP);
    tick_all(&mut app);
    assert_eq!(app.session.wallets[0].keys_unchecked(), 0);
    app.press(Action::OpenWallet(0));
    let s = shown(&mut app);
    assert!(!s.contains("of this wallet are not checked"), "{s}");
    assert!(s.contains("tb1"), "{s}");
    app.press(Action::ShowCode(Code::Receive(0, 0)));
    let s = shown(&mut app);
    assert!(!s.contains("of this wallet are not checked"), "{s}");
}

#[test]
fn the_review_shows_the_keys_until_they_are_checked() {
    for size in [DESKTOP, PI] {
        let mut app = reviewing(&multisig(), size);
        let s = shown(&mut app);
        assert!(s.contains("Not checked"), "{s}");
        assert!(s.contains("not checked · compare"), "{s}");
        assert!(!s.contains("Every key checked"), "{s}");
        tick_all(&mut app);
        let s = shown(&mut app);
        assert!(s.contains("Every key checked"), "{s}");
        assert!(!s.contains("Not checked"), "{s}");
    }
}

#[test]
fn a_wallet_of_one_key_shows_neither() {
    let mut app = reviewing(&single(), DESKTOP);
    let s = shown(&mut app);
    assert!(!s.contains("Every key checked"), "{s}");
    assert!(!s.contains("not checked"), "{s}");
    app.press(Action::OpenWallet(0));
    let s = shown(&mut app);
    assert!(!s.contains("not checked"), "{s}");
}

/// The wallet `kit` with a single-key wallet before it, so that the
/// spend's wallet is not at place 0, a spend from it open on the review.
fn reviewing_second(kit: &Kit) -> (Faraday, usize) {
    let psbt = testkit::unsigned(kit).unwrap();
    let mut app = testkit::started();
    app.event(Event::Display(DisplayInfo {
        width: DESKTOP.0,
        height: DESKTOP.1,
        dpi: DESKTOP.2,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
    app.storage(StorageEvent::Restored {
        inbox: faraday_core::files_from(vec![
            (
                faraday_text::Text::of("a-first.txt"),
                single().descriptor.into_bytes(),
            ),
            (
                faraday_text::Text::of("b-second.txt"),
                kit.descriptor.clone().into_bytes(),
            ),
            (faraday_text::Text::of("spend.psbt"), psbt.to_bytes()),
        ]),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    for name in ["a-first.txt", "b-second.txt"] {
        let at = app.inbox().iter().position(|i| i.name == name).unwrap();
        testkit::load_wallet(&mut app, at);
    }
    add_key(&mut app, "bacon");
    let at = app.inbox().iter().position(|i| i.name == "spend.psbt");
    app.press(Action::StartSpend(at.unwrap()));
    if app.spend().unwrap().open != Some(step::TRANSACTION) {
        app.press(Action::Step(step::TRANSACTION));
    }
    let wi = app.spend().unwrap().wallet.expect("the spend's wallet");
    (app, wi)
}

#[test]
fn the_reviews_tick_is_offered_and_ticks_the_spends_wallet_only() {
    let (mut app, wi) = reviewing_second(&multisig());
    assert_ne!(wi, 0, "the spend's wallet is not the first");
    let of = KeyBlockOf::Wallet(wi as u16);
    let k = (0..3u8)
        .find(|&k| offered(&mut app, Action::KeyTick(of, k)))
        .expect("the review offers a key's tick");
    let others = |app: &Faraday| -> Vec<(usize, Vec<bool>)> {
        (0..app.session.wallets.len())
            .filter(|&i| i != wi)
            .map(|i| {
                (
                    app.session.wallets[i].keys_unchecked(),
                    app.session.wallets[i].key_ticks(),
                )
            })
            .collect()
    };
    let (before, others_before) = (app.session.wallets[wi].key_ticks(), others(&app));
    let unchecked = app.session.wallets[wi].keys_unchecked();
    app.press(Action::KeyTick(of, k));
    assert_ne!(app.session.wallets[wi].key_ticks(), before);
    assert_eq!(app.session.wallets[wi].keys_unchecked(), unchecked - 1);
    assert_eq!(others(&app), others_before);
}

fn add_seed(app: &mut Faraday, n: usize) {
    app.press(Action::Entry(None));
    for c in testkit::test_words(testkit::TEST_SEEDS[n].0).chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    app.press(Action::EntryAdd);
}

/// The 2-of-3 savings wallet with two of its keys loaded, as the Spend
/// tab stages it, its transaction in Files when `with_psbt`.
fn savings_staged(with_psbt: bool) -> Faraday {
    let kit = testkit::kits()
        .into_iter()
        .find(|k| k.id == "savings")
        .unwrap();
    let mut files = vec![(
        faraday_text::Text::of("savings-wallet.txt"),
        kit.descriptor.clone().into_bytes(),
    )];
    if with_psbt {
        let (u, _) = testkit::spend_of(&kit).unwrap();
        files.push((faraday_text::Text::of("savings.psbt"), u.to_bytes()));
    }
    let mut app = testkit::started();
    app.event(Event::Display(DisplayInfo {
        width: DESKTOP.0,
        height: DESKTOP.1,
        dpi: DESKTOP.2,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
    app.storage(StorageEvent::Restored {
        inbox: faraday_core::files_from(files),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    add_seed(&mut app, 0);
    add_seed(&mut app, 1);
    testkit::load_wallet(&mut app, 0);
    app
}

fn cautions(app: &mut Faraday) -> usize {
    texts(app)
        .iter()
        .filter(|t| t.contains(" of this wallet "))
        .count()
}

fn tick_every_key(app: &mut Faraday) {
    for k in 0..app.session.wallets[0].key_checks().len() {
        app.press(Action::KeyTick(KeyBlockOf::Wallet(0), k as u8));
    }
}

#[test]
fn the_spends_address_step_cautions_while_a_key_is_not_checked() {
    let mut app = reviewing(&multisig(), DESKTOP);
    assert!(app.session.wallets[0].keys_unchecked() > 0);
    app.press(Action::Step(step::CHECK));
    let s = shown(&mut app);
    assert!(s.contains("tb1"), "{s}");
    assert_eq!(cautions(&mut app), 1, "{s}");
    tick_all(&mut app);
    assert_eq!(app.session.wallets[0].keys_unchecked(), 0);
    assert_eq!(cautions(&mut app), 0);
}

#[test]
fn spend_from_a_backup_cautions_over_its_first_address_while_a_key_is_not_checked() {
    let mut app = savings_staged(true);
    app.press(Action::Nav(Screen::SpendBackup));
    app.press(Action::SpendBackup(F::SpendFrom(0)));
    app.press(Action::Step(step::CHECK));
    let s = shown(&mut app);
    assert!(s.contains("Receive address 0"), "{s}");
    assert_eq!(cautions(&mut app), 1, "{s}");
    tick_every_key(&mut app);
    assert_eq!(cautions(&mut app), 0);
}

#[test]
fn spend_from_a_backups_address_lists_caution_once_each_while_a_key_is_not_checked() {
    let mut app = savings_staged(false);
    app.press(Action::Nav(Screen::SpendBackup));
    app.press(Action::SpendBackup(F::Walkthrough));
    app.press(Action::SpendBackup(F::Choose(0)));
    app.press(Action::SpendBackup(F::Card(page::CHECK)));
    assert_eq!(app.spend_backup().open, Some(Open::Page(page::CHECK)));
    let s = shown(&mut app);
    assert!(s.contains("Receive") && s.contains("Change"), "{s}");
    assert_eq!(cautions(&mut app), 2, "{s}");
    tick_every_key(&mut app);
    assert_eq!(cautions(&mut app), 0);
}
