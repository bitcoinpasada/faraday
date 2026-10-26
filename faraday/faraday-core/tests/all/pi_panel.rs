//! The Pi's 2.8-inch panel, 480 × 640 pixels at 286 dpi (268 dp wide):
//! nothing on any page or sheet is cut off at its sides. Every control
//! and every line of text stands inside the panel and inside the card or
//! sheet it is drawn in, at every place the page scrolls to; what runs
//! past the foot of a page that scrolls is not counted.

use crate::common::add_key;
use faraday_core::family::FamilyAction as F;
use faraday_core::testkit;
use faraday_core::wallet::step;
use faraday_core::{Action, Faraday, ScanPurpose, Screen, Sheet, StorageEvent};
use faraday_shell_api::{App, BootState, DisplayInfo, Event, SecureHardware};

const W: u16 = 480;
const H: u16 = 640;

/// Every screen a press on the navigation or a tile can open.
const SCREENS: [Screen; 31] = [
    Screen::Home,
    Screen::WalletsStart,
    Screen::Wallets,
    Screen::Spend,
    Screen::Files,
    Screen::Entry,
    Screen::Backup,
    Screen::Message,
    Screen::CheckMessage,
    Screen::Create,
    Screen::Restore,
    Screen::SetTime,
    Screen::Randomness,
    Screen::Vaults,
    Screen::Backups,
    Screen::Bip85,
    Screen::Silent,
    Screen::Explore,
    Screen::Tools,
    Screen::Codes,
    Screen::Family,
    Screen::Catalog,
    Screen::Visit,
    Screen::Unlock,
    Screen::VaultContents,
    Screen::Lightning,
    Screen::Vanity,
    Screen::Decode,
    Screen::Transfer,
    Screen::Upgrade,
    Screen::Settings,
];

/// The sheets that hold no state of their own beyond the page under
/// them.
const SHEETS: [Sheet; 6] = [
    Sheet::Lock,
    Sheet::LockAsk,
    Sheet::Power,
    Sheet::Devices,
    Sheet::Mode,
    Sheet::NavMenu,
];

/// The app on the Pi's panel with `inbox` in Files.
fn panel(inbox: Vec<(String, Vec<u8>)>) -> Faraday {
    let mut app = Faraday::new();
    testkit::move_to(&mut app.session, testkit::NET).unwrap();
    app.event(Event::Display(DisplayInfo {
        width: W,
        height: H,
        dpi: 286,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
    app.storage(StorageEvent::Restored {
        inbox,
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    let _ = app.frame();
    app
}

fn kit(id: &str) -> testkit::Kit {
    testkit::kits().into_iter().find(|k| k.id == id).unwrap()
}

/// Savings' wallet file and its unsigned spend in Files, with test keys
/// 1 and 2 loaded and the wallet loaded.
fn savings_loaded() -> Faraday {
    let k = kit("savings");
    let (unsigned, _) = testkit::spend_of(&k).unwrap();
    let mut app = panel(vec![
        ("savings-wallet.txt".to_string(), k.descriptor.into_bytes()),
        ("savings-unsigned.psbt".to_string(), unsigned.to_bytes()),
    ]);
    let at = |app: &Faraday, name: &str| app.inbox().iter().position(|i| i.name == name).unwrap();
    app.press(Action::LoadWallet(at(&app, "savings-wallet.txt")));
    add_key(&mut app, testkit::TEST_SEEDS[0].0);
    add_key(&mut app, testkit::TEST_SEEDS[1].0);
    app
}

/// What the page drawn now has cut off at its sides, from its top to its
/// foot, each named with `what`.
fn cut_off(app: &mut Faraday, what: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut last = None;
    for _ in 0..60 {
        for p in app.drawn_past_sides() {
            let line = format!("{what}: {p}");
            if !found.contains(&line) {
                found.push(line);
            }
        }
        let shown = Some(app.drawn_text_boxes());
        if shown == last {
            break;
        }
        last = shown;
        app.event(Event::Scroll {
            x: W / 2,
            y: H / 2,
            dy: 60,
        });
        app.settle();
    }
    for _ in 0..60 {
        app.event(Event::Scroll {
            x: W / 2,
            y: H / 2,
            dy: -60,
        });
    }
    app.settle();
    found
}

/// Each screen, and each sheet over it, as `app` opens them.
fn every_screen(app: &mut Faraday, state: &str) -> Vec<String> {
    let mut found = Vec::new();
    for screen in SCREENS {
        app.press(Action::Nav(screen));
        app.settle();
        let what = format!("{state} {:?}", app.screen);
        found.extend(cut_off(app, &what));
    }
    app.press(Action::Nav(Screen::Home));
    for sheet in SHEETS {
        app.sheet = Some(sheet);
        app.settle();
        found.extend(cut_off(app, &format!("{state} Home under {sheet:?}")));
        app.sheet = None;
    }
    for purpose in [ScanPurpose::Files, ScanPurpose::Seed, ScanPurpose::Intake] {
        testkit::open_scan(app, purpose);
        app.settle();
        found.extend(cut_off(app, &format!("{state} scanner for {purpose:?}")));
        app.press(Action::Cancel);
    }
    found
}

/// Every step card of the spend open in turn, then signed here.
fn every_spend_step(app: &mut Faraday) -> Vec<String> {
    let at = app
        .inbox()
        .iter()
        .position(|i| i.name == "savings-unsigned.psbt")
        .unwrap();
    app.press(Action::StartSpend(at));
    let mut found = Vec::new();
    let steps = [
        step::WALLET,
        step::CHECK,
        step::TRANSACTION,
        step::TXID,
        step::SIGNERS,
        step::SIGN,
    ];
    for s in steps {
        app.press(Action::Step(s));
        app.settle();
        found.extend(cut_off(app, &format!("spend step {s}")));
    }
    app.press(Action::SignHere);
    for s in [step::COLLECT, step::FINISH] {
        app.press(Action::Step(s));
        app.settle();
        found.extend(cut_off(app, &format!("spend signed, step {s}")));
    }
    found
}

/// The Spend tab's pages, from the map to the transaction, by its own
/// Next on each.
fn every_family_page(app: &mut Faraday) -> Vec<String> {
    use faraday_core::family::page;
    app.press(Action::Nav(Screen::Family));
    let mut found = Vec::new();
    for p in [page::MAP, page::CHECK, page::WRITE] {
        found.extend(cut_off(app, &format!("Spend tab page {p}")));
        app.press(Action::Family(F::Next(p)));
        app.settle();
    }
    found.extend(cut_off(app, "Spend tab page after Write"));
    found
}

fn assert_none(found: &[String]) {
    assert!(
        found.is_empty(),
        "cut off on the Pi's panel:\n{}",
        found.join("\n")
    );
}

#[test]
fn nothing_is_cut_off_on_any_screen_with_nothing_loaded() {
    let mut app = panel(Vec::new());
    assert_none(&every_screen(&mut app, "empty"));
}

#[test]
fn nothing_is_cut_off_on_any_screen_with_a_wallet_and_its_keys() {
    let mut app = savings_loaded();
    let mut found = every_screen(&mut app, "savings");
    app.press(Action::Nav(Screen::Wallets));
    app.press(Action::OpenWallet(0));
    found.extend(cut_off(&mut app, "savings, Savings open"));
    assert_none(&found);
}

#[test]
fn nothing_is_cut_off_on_any_step_of_a_spend() {
    let mut app = savings_loaded();
    assert_none(&every_spend_step(&mut app));
}

#[test]
fn nothing_is_cut_off_on_the_spend_tab() {
    let mut app = savings_loaded();
    assert_none(&every_family_page(&mut app));
}

#[test]
fn nothing_is_cut_off_on_any_screen_of_the_online_app() {
    let mut app = savings_loaded();
    app.online = true;
    assert_none(&every_screen(&mut app, "online"));
    assert_none(&every_spend_step(&mut app));
}

/// New key's steps: Length with no count chosen and its Other menu,
/// Randomness, and This device's Generator card with its sources, Skip
/// and Make the words, then with dice of one's own chosen and added.
fn every_new_key_step(app: &mut Faraday) -> Vec<String> {
    use faraday_core::fresh::OwnSource;
    use faraday_core::fresh::page::Act;
    use faraday_core::keygen::Way;
    app.press(Action::Nav(Screen::Entry));
    app.press(Action::KeyGen(None));
    app.settle();
    let mut found = cut_off(app, "New key, Length");
    app.press(Action::KOtherLengths);
    app.settle();
    found.extend(cut_off(app, "New key, Length's Other menu"));
    app.press(Action::Cancel);
    app.press(Action::KWords(12));
    app.settle();
    found.extend(cut_off(app, "New key, Randomness"));
    app.press(Action::KWay(Way::Device.index()));
    app.event(Event::Entropy(faraday_shell_api::EntropyBytes::new(
        [0x6c; 32],
    )));
    app.settle();
    found.extend(cut_off(app, "New key, Generator"));
    app.press(Action::Own(Act::Source(OwnSource::Dice)));
    for f in [3, 1, 4] {
        app.press(Action::Own(Act::Roll(f)));
    }
    app.settle();
    found.extend(cut_off(app, "New key, Generator on Dice"));
    app.press(Action::Own(Act::Add));
    app.settle();
    found.extend(cut_off(app, "New key, Generator with dice added"));
    found
}

#[test]
fn nothing_is_cut_off_on_new_key() {
    let mut app = panel(Vec::new());
    assert_none(&every_new_key_step(&mut app));
    let mut app = panel(Vec::new());
    testkit::into_demo(&mut app);
    assert_none(&every_new_key_step(&mut app));
}

#[test]
fn nothing_is_cut_off_on_any_screen_in_practice() {
    let mut app = panel(Vec::new());
    testkit::into_demo(&mut app);
    assert_none(&every_screen(&mut app, "practice"));
}
