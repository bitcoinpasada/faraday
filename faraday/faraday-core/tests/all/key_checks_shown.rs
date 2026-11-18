//! A wallet of several keys says on its receive addresses and on the
//! spend's review when its keys are not checked.

use crate::common::{add_key, texts};
use faraday_core::testkit::{self, Kit};
use faraday_core::wallet::{KeyBlockOf, step};
use faraday_core::{Action, Code, Faraday, StorageEvent};
use faraday_shell_api::{App, BootState, DisplayInfo, Event, SecureHardware};

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
        app.press(Action::ShowCode(Code::Receive(0, 0)));
        assert!(app.qr().is_some());
        let s = shown(&mut app);
        assert!(s.contains("keys of this wallet are not checked"), "{s}");
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
