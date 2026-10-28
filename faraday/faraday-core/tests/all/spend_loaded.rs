//! The Spend tab when something is loaded already: it opens on what is
//! loaded and the next step for each wallet instead of the walk-through,
//! which stays one press away and is what someone with nothing loaded
//! still sees first.

use faraday_core::family::{FamilyAction as F, Open, page};
use faraday_core::testkit;
use faraday_core::wallet::step;
use faraday_core::{Action, Faraday, Screen, StorageEvent};
use faraday_shell_api::{App, BootState, DisplayInfo, Event, Key, SecureHardware};

fn kit(id: &str) -> testkit::Kit {
    testkit::kits().into_iter().find(|k| k.id == id).unwrap()
}

fn with_files(files: Vec<(String, Vec<u8>)>) -> Faraday {
    let mut app = Faraday::new();
    faraday_core::testkit::move_to(&mut app.session, faraday_core::testkit::NET).unwrap();
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
        inbox: files,
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app
}

fn add_key(app: &mut Faraday, n: usize) {
    app.press(Action::Entry(None));
    for c in testkit::test_words(testkit::TEST_SEEDS[n].0).chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    app.press(Action::EntryAdd);
}

fn savings() -> (String, Vec<u8>) {
    (
        "savings-wallet.txt".to_string(),
        kit("savings").descriptor.into_bytes(),
    )
}

fn savings_psbt() -> (String, Vec<u8>) {
    let (u, _) = testkit::spend_of(&kit("savings")).unwrap();
    ("savings-unsigned.psbt".to_string(), u.to_bytes())
}

#[test]
fn with_nothing_loaded_the_tab_is_the_walk_through() {
    let mut app = with_files(Vec::new());
    app.press(Action::Nav(Screen::Family));
    assert!(!app.family_overview());
}

#[test]
fn a_ready_wallet_goes_straight_to_its_next_page() {
    let mut app = with_files(vec![savings()]);
    add_key(&mut app, 0);
    add_key(&mut app, 1);
    app.press(Action::LoadWallet(0));
    app.press(Action::Nav(Screen::Family));
    assert!(app.family_overview(), "the tab opens on what is loaded");
    let _ = app.frame();
    assert!(app.offers(Action::Family(F::SpendFrom(0))));
    app.press(Action::Family(F::SpendFrom(0)));
    assert!(!app.family_overview());
    assert_eq!(app.family_wallet(), Some(0));
    assert_eq!(
        app.family().open,
        Some(Open::Page(page::CHECK)),
        "the map, the stick and the wallet are behind it"
    );
}

#[test]
fn a_ready_wallet_with_its_transaction_in_files_goes_to_signing() {
    let mut app = with_files(vec![savings(), savings_psbt()]);
    add_key(&mut app, 0);
    add_key(&mut app, 1);
    app.press(Action::LoadWallet(0));
    app.press(Action::Nav(Screen::Family));
    app.press(Action::Family(F::SpendFrom(0)));
    assert!(
        app.spend().is_some(),
        "the PSBT in Files is the one to sign"
    );
    assert_eq!(app.screen, Screen::Family);
    assert_eq!(app.family().open, Some(Open::Spend));
    assert!(app.spend().unwrap().steps.contains(&step::SIGN));
}

/// A wallet made here with its keys loaded asks for no seeds: its
/// signers' card is Who signs, not Add the seeds that can sign.
#[test]
fn a_wallet_made_here_with_its_keys_loaded_asks_for_no_seeds() {
    use faraday_core::strings::EN;
    let c = testkit::compute();
    let mut app = with_files(vec![savings_psbt()]);
    add_key(&mut app, 0);
    add_key(&mut app, 1);
    app.session
        .add_wallet(
            &c,
            "Savings",
            &kit("savings").descriptor,
            EN.create_make_made_here,
            faraday_core::wallet::Origin::Statement,
        )
        .unwrap();
    app.press(Action::Nav(Screen::Family));
    app.press(Action::Family(F::SpendFrom(0)));
    let _ = app.frame();
    let texts = app.drawn_texts();
    assert!(texts.iter().any(|t| t == EN.family_who_signs), "{texts:?}");
    assert!(
        !texts.iter().any(|t| t == EN.family_add_the_seeds),
        "{texts:?}"
    );
}

#[test]
fn a_wallet_short_of_keys_is_opened_rather_than_spent() {
    let mut app = with_files(vec![savings()]);
    add_key(&mut app, 0);
    app.press(Action::LoadWallet(0));
    app.press(Action::Nav(Screen::Family));
    let _ = app.frame();
    assert!(!app.offers(Action::Family(F::SpendFrom(0))));
    assert!(app.offers(Action::OpenWallet(0)));
}

#[test]
fn a_seed_with_no_wallet_spends_as_a_single_key_wallet() {
    let mut app = with_files(Vec::new());
    add_key(&mut app, 0);
    app.press(Action::Nav(Screen::Family));
    assert!(app.family_overview());
    let _ = app.frame();
    assert!(app.offers(Action::Family(F::SeedWallet)));
    app.press(Action::Family(F::SeedWallet));
    assert_eq!(app.session.wallets.len(), 1);
    assert_eq!(app.family_wallet(), Some(0));
}

/// The Spending wallet's spend in the signing review, with its first
/// output stating, or not, an origin under a key that is not loaded.
fn spending_review(claimed: bool) -> Faraday {
    let kit = kit("spending");
    let mut psbt = testkit::unsigned(&kit).unwrap().inner().clone();
    if claimed {
        let key = psbt.outputs[1]
            .bip32_derivation
            .keys()
            .next()
            .copied()
            .unwrap();
        let path = psbt.outputs[1].bip32_derivation[&key].1.clone();
        psbt.outputs[0].bip32_derivation.insert(
            key,
            (
                faraday_bip::bitcoin::bip32::Fingerprint::from([0xde, 0xad, 0xbe, 0xef]),
                path,
            ),
        );
    }
    let psbt = faraday_psbt::Psbt::from(psbt);
    let mut app = testkit::started();
    app.storage(StorageEvent::Restored {
        inbox: vec![
            ("spend.psbt".to_string(), psbt.to_bytes()),
            (
                "spending-wallet.txt".to_string(),
                kit.descriptor.clone().into_bytes(),
            ),
        ],
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app.press(Action::LoadWallet(1));
    add_key(&mut app, 0);
    app.press(Action::StartSpend(0));
    app
}

/// The side panel's Change line counts verified change only: an output
/// that is only claimed as change is among what it sends (F136), so the
/// panel reads Verified for the change that is, and never Unverified.
#[test]
fn the_panel_counts_only_verified_change() {
    // The value beside the panel's Change label.
    let panel_change = |claimed: bool| {
        let mut app = spending_review(claimed);
        let shown = crate::common::texts(&mut app);
        let panel = shown
            .iter()
            .position(|t| t == "This transaction")
            .unwrap_or_else(|| panic!("{shown:?}"));
        let label = shown[panel..]
            .iter()
            .position(|t| t == "Change")
            .unwrap_or_else(|| panic!("{shown:?}"));
        shown[panel + label + 1].clone()
    };
    assert_eq!(panel_change(false), "Verified");
    assert_eq!(panel_change(true), "Verified");
}

#[test]
fn the_walk_through_is_one_press_away() {
    let mut app = with_files(Vec::new());
    add_key(&mut app, 0);
    app.press(Action::Nav(Screen::Family));
    app.press(Action::Family(F::Walkthrough));
    assert!(!app.family_overview());
    assert_eq!(app.family().open, Some(Open::Page(page::MAP)));
}

/// The ink boxes of a loaded wallet's name, shape and keys' line on the
/// Pi's panel at `size`, with Savings loaded and one of its keys.
fn card_lines_at(size: faraday_core::ui::TextSize) -> Vec<(String, (i32, i32, i32, i32))> {
    let mut app = with_files(vec![savings()]);
    app.event(Event::Display(DisplayInfo {
        width: 480,
        height: 640,
        dpi: 286,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
    add_key(&mut app, 0);
    app.press(Action::LoadWallet(0));
    app.press(Action::TextSize(size));
    app.press(Action::Nav(Screen::Family));
    let lines: Vec<_> = app
        .drawn_text_boxes()
        .into_iter()
        .filter(|(t, _)| t == "Savings" || t.starts_with("2 of 3") || t.starts_with("1 key"))
        .collect();
    // The card's own name, the one just over its shape: the bar at the
    // top names the wallet too.
    let shape = lines
        .iter()
        .find(|(t, _)| t.starts_with("2 of 3"))
        .map(|(_, b)| b.1);
    let shape = shape.expect("the wallet's shape is shown");
    let card: Vec<_> = lines
        .into_iter()
        .filter(|(t, b)| t != "Savings" || (b.1 < shape && shape - b.1 < 2 * b.3))
        .collect();
    assert_eq!(card.len(), 3, "{card:?}");
    card
}

/// On a one-column page a loaded wallet's card grows with larger text:
/// its name, its shape and its keys' line stand one under another, none
/// over the next, at every text size.
#[test]
fn a_loaded_wallets_card_keeps_its_lines_apart_at_every_text_size() {
    use faraday_core::ui::TextSize;
    for size in TextSize::ALL {
        let lines = card_lines_at(size);
        let mut tops: Vec<_> = lines.iter().map(|(_, b)| *b).collect();
        tops.sort_by_key(|b| b.1);
        for pair in tops.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            assert!(a.1 + a.3 <= b.1 + 1, "{size:?}: {lines:?}");
        }
    }
}
