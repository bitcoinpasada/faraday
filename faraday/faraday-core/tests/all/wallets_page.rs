//! The Wallets page with wallets loaded: the
//! start page is the list and Add a key; every other job (Sign a
//! transaction, Back up, Show wallet QR, Sign a message) is reached
//! from the wallet card.

use faraday_core::strings::EN;
use faraday_core::testkit::KindText;
use faraday_core::{Action, Faraday, Screen};
use faraday_shell_api::{App, BootState, DisplayInfo, Event, SecureHardware};

const WORDS: &str = "abandon abandon abandon abandon abandon abandon abandon abandon abandon \
                      abandon abandon about";

fn shown() -> Faraday {
    let mut app = faraday_core::testkit::new_on_test_net();
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
    app
}

/// A single-key wallet, its one key loaded here.
fn with_single_key_wallet() -> Faraday {
    let c = faraday_core::testkit::compute();
    let mut app = shown();
    app.session.keys.add_words(&c, WORDS, "Seed", None).unwrap();
    faraday_core::testkit::keep_published(&mut app);
    let text = faraday_core::create::NewKind::NativeSegwit
        .key_text(&c, app.session.keys.master_for_tests(0))
        .unwrap();
    let d = format!("wpkh({}/<0;1>/*)", &*text);
    app.session
        .add_wallet_confirmed(
            &c,
            "Wallet",
            &d,
            "test",
            faraday_core::wallet::Origin::Named,
        )
        .unwrap();
    app.press(Action::Nav(Screen::WalletsStart));
    let _ = app.frame();
    app
}

/// A watch-only single-key wallet: the key is not held here, so it
/// cannot sign a message.
fn with_watch_only_wallet() -> Faraday {
    let c = faraday_core::testkit::compute();
    let mut app = shown();
    let mut elsewhere = faraday_core::wallet::Session::default();
    elsewhere.keys.add_words(&c, WORDS, "Seed", None).unwrap();
    let text = faraday_core::create::NewKind::NativeSegwit
        .key_text(&c, elsewhere.keys.master_for_tests(0))
        .unwrap();
    let d = format!("wpkh({}/<0;1>/*)", &*text);
    app.session
        .add_wallet_confirmed(
            &c,
            "Watch-only",
            &d,
            "test",
            faraday_core::wallet::Origin::Named,
        )
        .unwrap();
    app.press(Action::Nav(Screen::WalletsStart));
    let _ = app.frame();
    app
}

#[test]
fn wallets_start_page_offers_the_row_and_the_four_ways() {
    let app = with_single_key_wallet();
    assert!(app.offers(Action::OpenWallet(0)), "the wallet row");
    assert!(app.offers(Action::CreateWallet), "Make a new wallet");
    assert!(
        app.offers(Action::RestoreWallet),
        "Restore an existing wallet"
    );
    assert!(
        app.offers(Action::Nav(Screen::SpendBackup)),
        "Spend from a backup"
    );
    assert!(app.offers(Action::Entry(None)), "Add a key");
    // The tile lists the wallets and the four ways in: Make a new key is
    // Add a key's, and the keys are in the session pane.
    assert!(!app.offers(Action::KeyGen(None)), "no Make a new key");
}

const OTHER_WORDS: [&str; 3] = [
    "legal winner thank year wave sausage worth useful legal winner thank yellow",
    "letter advice cage absurd amount doctor acoustic avoid letter advice cage above",
    "zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo wrong",
];

/// A window `w` by `h` with the wallets named "Wallet 0" and on, one for
/// each of the `words`, each a single-key wallet from its seed.
fn with_wallets(w: u16, h: u16, words: &[&str], count: usize) -> Faraday {
    let c = faraday_core::testkit::compute();
    let mut app = shown();
    app.event(Event::Display(DisplayInfo {
        width: w,
        height: h,
        dpi: 160,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
    for i in 0..count {
        // Past the word sets, the same words under a passphrase: a new key.
        let pass = if i < words.len() {
            String::new()
        } else {
            format!("p{i}")
        };
        app.session
            .keys
            .add_words_with(&c, words[i % words.len()], &pass, "Seed", None)
            .unwrap();
        let text = faraday_core::create::NewKind::NativeSegwit
            .key_text(&c, app.session.keys.master_for_tests(i))
            .unwrap();
        let d = format!("wpkh({}/<0;1>/*)", &*text);
        app.session
            .add_wallet_confirmed(
                &c,
                &format!("Wallet {i}"),
                &d,
                "test",
                faraday_core::wallet::Origin::Named,
            )
            .unwrap();
    }
    app.press(Action::Nav(Screen::WalletsStart));
    let _ = app.frame();
    app
}

/// Where the line `text` is drawn: its top.
fn top_of(app: &mut Faraday, text: &str) -> i32 {
    let _ = app.frame();
    app.drawn_text_boxes()
        .into_iter()
        .find(|(t, _)| t == text)
        .unwrap_or_else(|| panic!("{text} not drawn"))
        .1
        .1
}

/// The four ways' names, in Home's order.
fn way_names() -> [&'static str; 4] {
    [
        EN.start_make_a_new_wallet,
        EN.start_restore_an_existing,
        EN.catalog_spend_backup,
        EN.start_add_a_key,
    ]
}

/// The pair stands only where the pills do, so the tile leaves out the three
/// rows they repeat and Add a key is alone.
#[test]
fn under_a_short_list_the_tile_ends_in_add_a_key_alone_where_the_pills_stand() {
    let mut app = with_wallets(1366, 768, &[WORDS], 1);
    let last = top_of(&mut app, "Wallet 0");
    let add = top_of(&mut app, EN.start_add_a_key);
    assert!(add > last && add - last < 200, "under the one wallet");
    let texts = app.drawn_texts();
    for n in &way_names()[..3] {
        assert!(
            !texts.iter().any(|t| t == n),
            "{n} is a pill's, not a row's"
        );
    }
    for a in [
        Action::CreateWallet,
        Action::RestoreWallet,
        Action::Nav(Screen::SpendBackup),
        Action::Entry(None),
    ] {
        assert!(app.offers(a), "{a:?} stays offered");
    }
    // Home's pills put the same three first, in the same order.
    let mut home = with_wallets(1920, 1080, &[WORDS], 1);
    home.press(Action::Nav(Screen::Home));
    let _ = home.frame();
    let at = |a: Action| home.where_offered(a).unwrap().0;
    assert!(
        at(Action::CreateWallet) < at(Action::RestoreWallet)
            && at(Action::RestoreWallet) < at(Action::Nav(Screen::SpendBackup)),
        "Home's order"
    );
}

#[test]
fn each_way_goes_where_the_same_words_go_elsewhere() {
    for (action, screen) in [
        (Action::CreateWallet, Screen::Create),
        (Action::RestoreWallet, Screen::Restore),
        (Action::Nav(Screen::SpendBackup), Screen::SpendBackup),
        (Action::Entry(None), Screen::Entry),
    ] {
        let mut app = with_wallets(1366, 768, &[WORDS], 1);
        app.press(action);
        let _ = app.frame();
        assert_eq!(app.screen, screen, "{action:?}");
    }
}

#[test]
fn add_a_keys_row_is_neutral_with_no_fill() {
    for count in [0, 1, 3] {
        let mut app = with_wallets(1366, 768, &[WORDS, OTHER_WORDS[0], OTHER_WORDS[1]], count);
        let a = Action::Entry(None);
        let _ = app.frame();
        let (x, y, _, h) = app.hit_box(a).unwrap();
        let f = app.frame();
        let px = |x: u16, y: u16| {
            let i = (usize::from(y) * usize::from(f.width) + usize::from(x)) * 4;
            [f.rgba[i], f.rgba[i + 1], f.rgba[i + 2]]
        };
        // Inside the row, left of its icon, and in the tile just above
        // it (the rows touch: the row above is neutral too).
        let inside = px(x + 6, y + h / 2);
        let tile = px(x + 6, y.saturating_sub(4));
        assert_eq!(inside, tile, "{a:?} with {count} wallets is not filled");
    }
}

#[test]
fn the_page_with_no_wallet_says_none_is_loaded_and_offers_add_a_key() {
    let mut app = with_wallets(1366, 768, &[WORDS], 0);
    top_of(&mut app, EN.start_add_a_key);
    top_of(&mut app, EN.wallets_no_wallet_loaded);
}

#[test]
fn the_tile_is_headed_wallets_and_a_wallet_row_has_a_chevron() {
    let mut app = with_wallets(1366, 768, &[WORDS], 1);
    let _ = app.frame();
    // The heading at the tile, not the rail's own Wallets.
    let head = app
        .drawn_text_boxes()
        .into_iter()
        .filter(|(t, b)| t == EN.start_wallets && b.0 > 300)
        .map(|(_, b)| b.1)
        .next()
        .expect("the tile's heading");
    assert!(head < top_of(&mut app, "Wallet 0"), "the heading leads");
    let (x, y, w, h) = app.hit_box(Action::OpenWallet(0)).unwrap();
    // The chevron is drawn at the row's end: ink there, none at the same
    // place in a row of the plain list.
    let f = app.frame();
    let end = usize::from(x + w) - 16 - 10;
    let mid = usize::from(y + h / 2);
    let tile = (mid * usize::from(f.width) + usize::from(x) + 2) * 4;
    let ink = (0..20usize).any(|dx| {
        let i = (mid * usize::from(f.width) + end + dx - 4) * 4;
        f.rgba[i..i + 3] != f.rgba[tile..tile + 3]
    });
    assert!(ink, "a chevron at the row's end");
}

/// The first and the last row of the pane's left padding, a few dp left
/// of its Keys heading, that differ from the window's foot: where the pane
/// stands.
fn pane_span(app: &mut Faraday) -> (usize, usize) {
    let _ = app.frame();
    let keys = app
        .drawn_text_boxes()
        .into_iter()
        .find(|(t, _)| t == EN.session_group_keys)
        .unwrap()
        .1;
    let f = app.frame();
    let col = (keys.0 - 6) as usize;
    let at = |y: usize| {
        let i = (y * usize::from(f.width) + col) * 4;
        [f.rgba[i], f.rgba[i + 1], f.rgba[i + 2]]
    };
    let ground = at(usize::from(f.height) - 1);
    let rows: Vec<usize> = (0..usize::from(f.height))
        .filter(|&y| at(y) != ground)
        .collect();
    (rows[0], *rows.last().unwrap())
}

#[test]
fn add_a_key_is_pinned_at_the_foot_where_the_list_scrolls() {
    // 600 is the lowest window that has the pair; twelve wallets run past it.
    let mut app = with_wallets(1366, 600, &[WORDS, OTHER_WORDS[0]], 12);
    let row = top_of(&mut app, EN.start_add_a_key);
    assert!(row > 600 - 100, "at the tile's foot: {row}");
    let first = top_of(&mut app, "Wallet 0");
    // The wheel over the tile moves the list under the row, not the row.
    app.event(Event::Wheel {
        x: 1000,
        y: 200,
        dy: 120,
    });
    app.settle();
    assert_eq!(top_of(&mut app, EN.start_add_a_key), row);
    assert!(
        app.drawn_text_boxes().iter().all(|(t, _)| t != "Wallet 0")
            || top_of(&mut app, "Wallet 0") < first,
        "the list moved"
    );
}

#[test]
fn the_pane_stands_still_between_the_tile_and_a_wallets_page() {
    // Twelve wallets fill the tile; the page fills its place too.
    let mut app = with_wallets(1366, 600, &[WORDS, OTHER_WORDS[0]], 12);
    app.settle();
    let keys = top_of(&mut app, EN.session_group_keys);
    let tile = pane_span(&mut app);
    app.press(Action::OpenWallet(0));
    app.settle();
    assert_eq!(app.screen, Screen::Wallets);
    assert_eq!(top_of(&mut app, EN.session_group_keys), keys, "its heading");
    // The foot (the top row of the column is the page's own bar).
    assert_eq!(pane_span(&mut app).1, tile.1, "its foot");
}

#[test]
fn a_key_in_no_wallet_stands_in_the_panes_keys_not_the_tile() {
    let c = faraday_core::testkit::compute();
    let mut app = shown();
    app.session.keys.add_words(&c, WORDS, "Seed", None).unwrap();
    app.press(Action::Nav(Screen::WalletsStart));
    let _ = app.frame();
    let texts = app.drawn_texts();
    assert!(texts.iter().any(|t| t == EN.session_group_keys), "Keys");
    assert!(
        !texts.iter().any(|t| t == EN.wallets_keys_without),
        "no Keys without a wallet in the tile"
    );
}

#[test]
fn a_waiting_psbt_stands_in_waiting_with_sign() {
    let kit = faraday_core::testkit::kits()
        .into_iter()
        .find(|k| k.id == "spending")
        .unwrap();
    let psbt = faraday_core::testkit::unsigned(&kit).unwrap().to_bytes();
    let mut app = with_single_key_wallet();
    app.storage(faraday_core::StorageEvent::Restored {
        inbox: faraday_core::files_from(vec![(faraday_text::Text::of("spend.psbt"), psbt)]),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    let _ = app.frame();
    assert!(
        app.drawn_texts()
            .iter()
            .any(|t| t == EN.session_group_waiting),
        "Waiting"
    );
    assert!(app.offers(Action::StartSpend(0)), "Sign");
}

/// The wheel turned over the wallet card until `action` is drawn: a row
/// of "What you can do" below the window is reached by scrolling to it.
fn scroll_card_to(app: &mut Faraday, action: Action) {
    for _ in 0..20 {
        let _ = app.frame();
        if app.offers(action) {
            return;
        }
        app.event(Event::Wheel {
            x: 1000,
            y: 400,
            dy: 120,
        });
        app.settle();
    }
    let _ = app.frame();
}

#[test]
fn the_wallet_card_offers_its_jobs() {
    let mut app = with_single_key_wallet();
    app.press(Action::OpenWallet(0));
    let _ = app.frame();
    assert_eq!(app.screen, Screen::Wallets);
    scroll_card_to(&mut app, Action::QrWallet(0));
    assert!(app.offers(Action::Backup(0)), "Back up");
    assert!(app.offers(Action::QrWallet(0)), "Show wallet QR");
}

#[test]
fn another_wallets_psbt_in_files_leaves_the_card_on_spend_from_this_wallet() {
    // The test kit's PSBT spends from a test-network wallet, not this one
    // (only its own PSBT turns the button).
    let kit = faraday_core::testkit::kits()
        .into_iter()
        .find(|k| k.id == "spending")
        .unwrap();
    let psbt = faraday_core::testkit::unsigned(&kit).unwrap().to_bytes();
    let mut app = with_single_key_wallet();
    app.storage(faraday_core::StorageEvent::Restored {
        inbox: faraday_core::files_from(vec![(faraday_text::Text::of("spend.psbt"), psbt)]),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app.press(Action::OpenWallet(0));
    let _ = app.frame();
    assert_eq!(app.screen, Screen::Wallets);
    scroll_card_to(
        &mut app,
        Action::SpendBackup(faraday_core::spend_backup::SpendBackupAction::SpendFrom(0)),
    );
    for _ in 0..20 {
        if app
            .drawn_texts()
            .iter()
            .any(|t| t == "Spend from this wallet")
        {
            break;
        }
        app.event(Event::Wheel {
            x: 1000,
            y: 400,
            dy: 120,
        });
        app.settle();
    }
    let texts = app.drawn_texts();
    assert!(
        texts.iter().any(|t| t == "Spend from this wallet"),
        "{texts:?}"
    );
    assert!(app.offers(Action::SpendBackup(
        faraday_core::spend_backup::SpendBackupAction::SpendFrom(0)
    )));
}

#[test]
fn the_card_offers_sign_a_message_for_a_single_key_wallet_with_its_key_here() {
    let mut app = with_single_key_wallet();
    app.press(Action::OpenWallet(0));
    let _ = app.frame();
    scroll_card_to(&mut app, Action::SignMessageWith(0));
    assert!(app.offers(Action::SignMessageWith(0)));
}

#[test]
fn the_card_does_not_offer_sign_a_message_without_the_key_here() {
    let mut app = with_watch_only_wallet();
    app.press(Action::OpenWallet(0));
    let _ = app.frame();
    assert!(!app.offers(Action::SignMessageWith(0)));
}

#[test]
fn the_card_offers_check_an_address_and_it_opens() {
    let mut app = with_single_key_wallet();
    app.press(Action::OpenWallet(0));
    let _ = app.frame();
    let check = Action::AddressCheck(faraday_core::address_check::AddressPress::Another);
    scroll_card_to(&mut app, check);
    assert!(app.offers(check), "Check an address");
    app.press(check);
    assert_eq!(app.screen, Screen::CheckAddress);
}

#[test]
fn a_wallet_needing_a_key_lists_add_a_key_first_and_it_opens_for_that_key() {
    let mut app = with_watch_only_wallet();
    app.press(Action::OpenWallet(0));
    let _ = app.frame();
    // The one empty slot's key: the published words' fingerprint.
    let add = Action::Entry(Some([0x73, 0xc5, 0xda, 0x0a]));
    let (_, add_y) = app.where_offered(add).expect("Add a key");
    let (_, spend) = app.spend_button(&faraday_core::testkit::compute(), 0);
    let (_, spend_y) = app.where_offered(spend).expect("the spend");
    assert!(add_y < spend_y, "Add a key leads: {add_y} {spend_y}");
    app.press(add);
    let _ = app.frame();
    assert_eq!(app.screen, Screen::Entry);
    assert!(
        app.drawn_texts()
            .iter()
            .any(|t| t.contains("Add a key") && t.contains("73c5da0a")),
        "{:?}",
        app.drawn_texts()
    );
}

#[test]
fn a_complete_wallets_card_has_no_add_a_key_in_what_you_can_do() {
    let mut app = with_single_key_wallet();
    app.press(Action::OpenWallet(0));
    let _ = app.frame();
    let texts = app.drawn_texts();
    let at = texts
        .iter()
        .position(|t| t == EN.wallets_what_you_can_do)
        .expect("the heading");
    assert_ne!(texts[at + 1], EN.wallets_add_a_key, "{texts:?}");
    assert!(!app.offers(Action::Entry(Some([0x73, 0xc5, 0xda, 0x0a]))));
}
