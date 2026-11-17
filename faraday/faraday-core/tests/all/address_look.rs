//! The address check looks past the first hundred (`docs/QR.md` §2): an
//! address beyond them is offered **Look further** to a thousand and ten
//! thousand of each chain in every loaded wallet, found at the depth that
//! holds it, a miss names how far it looked and in how many wallets, and
//! **Stop** stops.

use faraday_bip::policy::WalletPolicy;
use faraday_core::address_check::AddressPress;
use faraday_core::create::NewKind;
use faraday_core::explore::XAction;
use faraday_core::{Action, Ask, Faraday, Screen, Sheet, StorageEvent, testkit};
use faraday_shell_api::{App, Event};

/// A single-key native SegWit wallet over test seed `n`.
fn single(n: usize) -> String {
    format!("wpkh({}/<0;1>/*)", testkit::key(n, "m/84h/1h/0h"))
}

/// Faraday with `descriptors` loaded as wallets, the address check's
/// scanner open.
fn checking(descriptors: &[String]) -> Faraday {
    let mut app = testkit::started();
    for (i, d) in descriptors.iter().enumerate() {
        app.storage(StorageEvent::Restored {
            inbox: faraday_core::files_from(vec![(
                faraday_text::Text::of(&format!("wallet{i}.txt")),
                d.as_bytes().to_vec(),
            )]),
            outbox: Vec::new(),
            kept: Vec::new(),
        });
        testkit::load_wallet(&mut app, 0);
    }
    assert_eq!(app.session.wallets.len(), descriptors.len());
    app.press(Action::ScanFor(Ask::Address));
    app
}

/// The address of `descriptor` at `change`/`index`.
fn address(descriptor: &str, change: bool, index: u32) -> String {
    let c = testkit::compute();
    let policy = WalletPolicy::parse_any(&c, descriptor).unwrap();
    policy
        .address_at(&c, testkit::net(), change, index)
        .unwrap()
        .to_string()
}

fn read(app: &mut Faraday, text: &str) {
    // A read closes the scanner: the next one opens it again.
    if app.scan().is_none() {
        app.press(Action::ScanFor(Ask::Address));
    }
    app.event(Event::Scanned {
        bytes: text.as_bytes().to_vec(),
    });
    let _ = app.frame();
}

/// What the address check's page says, or the scanner's note while the
/// page is not open.
fn said(app: &Faraday) -> String {
    if let Some(page) = app.address_check_state() {
        let mut out: Vec<String> = page
            .result_lines()
            .iter()
            .map(|l| String::from(l.as_str()))
            .collect();
        out.extend(page.refusal().map(|r| String::from(r.as_str())));
        if page.load_first() {
            out.push("Load a key or a wallet first".to_string());
        }
        return out.join("\n");
    }
    app.scan()
        .and_then(|s| s.note.as_ref())
        .map(|n| String::from(n.as_str()))
        .unwrap_or_default()
}

/// The labels of the ways over the page's result offers.
fn ways(app: &Faraday) -> Vec<String> {
    app.address_check_state()
        .map(|p| {
            p.ways()
                .iter()
                .map(|w| String::from(w.label().as_str()))
                .collect()
        })
        .unwrap_or_default()
}

/// A press on the page's way over at `i`.
fn way(i: u8) -> Action {
    Action::AddressCheck(AddressPress::Way(i))
}

/// Ticks a millisecond apart until nothing looks any more.
fn look_through(app: &mut Faraday) {
    for t in 1..100_000u64 {
        app.event(Event::Tick { now_ms: t });
        if !said(app).contains("Looking") {
            return;
        }
    }
    panic!("the look did not end: {}", said(app));
}

#[test]
fn an_address_at_150_is_found_by_looking_to_a_thousand() {
    let d = single(0);
    let mut app = checking(std::slice::from_ref(&d));
    read(&mut app, &address(&d, false, 150));
    assert!(said(&app).contains("first 100"), "{}", said(&app));
    app.press(way(0));
    look_through(&mut app);
    assert!(said(&app).contains("Receive · index 150"), "{}", said(&app));
}

#[test]
fn a_change_address_at_5000_is_found_by_looking_to_ten_thousand() {
    let d = single(0);
    let mut app = checking(std::slice::from_ref(&d));
    read(&mut app, &address(&d, true, 5_000));
    app.press(way(1));
    look_through(&mut app);
    assert!(said(&app).contains("Change · index 5000"), "{}", said(&app));
}

#[test]
fn a_multisig_address_past_100_is_found() {
    let d = testkit::savings();
    let mut app = checking(std::slice::from_ref(&d));
    read(&mut app, &address(&d, false, 120));
    app.press(way(0));
    look_through(&mut app);
    assert!(said(&app).contains("Receive · index 120"), "{}", said(&app));
}

#[test]
fn a_miss_says_how_far_it_looked_and_in_how_many_wallets() {
    let mut app = checking(&[single(0), testkit::savings()]);
    read(&mut app, &address(&single(1), false, 7));
    assert!(said(&app).contains("first 100"), "{}", said(&app));
    assert!(said(&app).contains("2 wallets"), "{}", said(&app));
    app.press(way(0));
    look_through(&mut app);
    assert!(said(&app).contains("first 1000"), "{}", said(&app));
    assert!(said(&app).contains("2 wallets"), "{}", said(&app));
}

#[test]
fn stop_ends_the_look() {
    let mut app = checking(&[single(0)]);
    read(&mut app, &address(&single(1), false, 7));
    app.press(way(1));
    app.event(Event::Tick { now_ms: 1 });
    assert!(said(&app).contains("Looking"), "{}", said(&app));
    // Stop is the card's only way while it looks.
    app.press(way(0));
    let stopped = said(&app);
    assert!(stopped.contains("Not found"), "{stopped}");
    for t in 2..50 {
        app.event(Event::Tick { now_ms: t });
    }
    assert_eq!(said(&app), stopped);
}

#[test]
fn a_look_ends_when_the_loaded_wallets_change_and_does_not_claim_the_new_one() {
    let mut app = checking(&[single(0), single(1)]);
    read(&mut app, &address(&single(1), false, 5_000));
    app.press(way(1));
    app.event(Event::Tick { now_ms: 1 });
    assert!(said(&app).contains("Looking"), "{}", said(&app));
    // The first wallet goes and the second takes its place.
    app.session.wallets.remove(0);
    app.event(Event::Tick { now_ms: 2 });
    let line = said(&app);
    assert!(!line.contains("Looking"), "{line}");
    for t in 3..50 {
        app.event(Event::Tick { now_ms: t });
    }
    assert_eq!(said(&app), line);
    assert!(ways(&app).is_empty());
}

#[test]
fn a_find_keeps_the_name_of_its_wallet_when_the_wallets_change() {
    let mut app = checking(&[single(0), single(1)]);
    read(&mut app, &address(&single(1), false, 150));
    app.press(way(0));
    look_through(&mut app);
    let found = said(&app);
    let name = app.session.wallets[1].name.clone();
    assert!(found.contains(name.as_str()), "{found}");
    app.session.wallets.remove(0);
    app.press(way(0));
    assert_eq!(said(&app), found);
}

/// The best-known public test mnemonic, whose addresses every BIP's
/// vectors give.
const OTHER: &str = "abandon abandon abandon abandon abandon abandon \
                     abandon abandon abandon abandon abandon about";

/// The single-key wallet over `OTHER`'s account 0 of `kind`, in the
/// session's network.
fn account_of_other(app: &Faraday, kind: NewKind) -> String {
    let c = testkit::compute();
    let spec = kind.spec(app.session.network(), 0).unwrap();
    let key = app.session.keys.key_text(&c, 0, &spec).unwrap().unwrap();
    String::from(&*kind.descriptor(1, &[key.as_str()]))
}

/// The address of `descriptor` at `change`/`index` on the network the
/// session is on.
fn address_in(app: &Faraday, descriptor: &str, change: bool, index: u32) -> String {
    let c = testkit::compute();
    let policy = WalletPolicy::parse_any(&c, descriptor).unwrap();
    policy
        .address_at(&c, app.session.network(), change, index)
        .unwrap()
        .to_string()
}

/// Faraday with `OTHER` loaded and no wallet, the address check's scanner
/// open.
fn with_key() -> Faraday {
    let mut app = testkit::started();
    let c = testkit::compute();
    app.session.keys.add_words(&c, OTHER, "", None).unwrap();
    app.press(Action::ScanFor(Ask::Address));
    app
}

#[test]
fn a_loaded_key_with_no_wallet_has_its_standard_accounts_searched() {
    // Account 0 of `OTHER` on Testnet (the sample seed is refused on
    // Mainnet): the addresses of BIP-84's and BIP-86's own Mainnet vectors
    // with the coin type 1 and the `tb` prefix, worked out with embit
    // 0.8.0, which reproduces BIP-84's and BIP-86's published Mainnet
    // vectors; BIP-44 publishes no addresses, so its rows rest on embit.
    for (address, line) in [
        // BIP-84: m/84'/1'/0'/0/0
        (
            "tb1q6rz28mcfaxtmd6v789l9rrlrusdprr9pqcpvkl",
            "Receive · index 0",
        ),
        // BIP-84: m/84'/1'/0'/1/0
        (
            "tb1q9u62588spffmq4dzjxsr5l297znf3z6j5p2688",
            "Change · index 0",
        ),
        // BIP-86: m/86'/1'/0'/0/0
        (
            "tb1p8wpt9v4frpf3tkn0srd97pksgsxc5hs52lafxwru9kgeephvs7rqlqt9zj",
            "Receive · index 0",
        ),
        // BIP-44: m/44'/1'/0'/0/0, worked out with embit 0.8.0
        ("mkpZhYtJu2r87Js3pDiWJDmPte2NRZ8bJV", "Receive · index 0"),
    ] {
        let mut app = with_key();
        read(&mut app, address);
        assert!(said(&app).contains(line), "{address}: {}", said(&app));
        assert!(
            said(&app).contains("Found in key "),
            "{address}: {}",
            said(&app)
        );
    }
    let mut app = with_key();
    read(
        &mut app,
        "tb1p8wpt9v4frpf3tkn0srd97pksgsxc5hs52lafxwru9kgeephvs7rqlqt9zj",
    );
    assert!(said(&app).contains("m/86h/1h/0h"), "{}", said(&app));
}

#[test]
fn a_loaded_key_has_its_nested_account_searched_on_testnet() {
    let mut app = testkit::started();
    testkit::move_to(&mut app.session, faraday_core::testkit::NET).unwrap();
    let c = testkit::compute();
    app.session.keys.add_words(&c, OTHER, "", None).unwrap();
    app.press(Action::ScanFor(Ask::Address));
    // BIP-49's test vector, m/49'/1'/0'/0/0 of `OTHER` on Testnet
    // (https://github.com/bitcoin/bips/blob/master/bip-0049.mediawiki).
    read(&mut app, "2Mww8dCYPUpKHofjgcXcBCEGmniw9CoaiD2");
    assert!(said(&app).contains("Receive · index 0"), "{}", said(&app));
    assert!(said(&app).contains("m/49h/1h/0h"), "{}", said(&app));
}

#[test]
fn a_standard_account_is_looked_further_in() {
    let mut app = with_key();
    // BIP-44 m/44'/1'/0'/0/150 of `OTHER`, worked out with embit 0.8.0; its index 0 is the address above.
    read(&mut app, "mhLeXQvZ8zzN2idymESqznYm8EcZupZP4W");
    assert!(said(&app).contains("first 100"), "{}", said(&app));
    app.press(way(0));
    look_through(&mut app);
    assert!(said(&app).contains("Receive · index 150"), "{}", said(&app));
    assert!(said(&app).contains("m/44h/1h/0h"), "{}", said(&app));
    assert!(ways(&app).contains(&"Make a wallet".to_string()));
}

#[test]
fn an_account_with_a_wallet_is_not_searched_twice() {
    let mut app = with_key();
    let d = account_of_other(&app, NewKind::NativeSegwit);
    app.storage(StorageEvent::Restored {
        inbox: faraday_core::files_from(vec![(
            faraday_text::Text::of("wallet.txt"),
            d.as_bytes().to_vec(),
        )]),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    testkit::load_wallet(&mut app, 0);
    assert_eq!(app.session.wallets.len(), 1);
    // BIP-84: m/84'/1'/0'/0/0
    read(&mut app, "tb1q6rz28mcfaxtmd6v789l9rrlrusdprr9pqcpvkl");
    // The wallet's own line, not the account's.
    assert!(!said(&app).contains("Found in key "), "{}", said(&app));
    assert!(said(&app).contains("Receive · index 0"), "{}", said(&app));
    assert!(!ways(&app).contains(&"Make a wallet".to_string()));
    // A miss counts the wallet and the three accounts with none.
    let a = address_in(&app, &single(1), false, 7);
    read(&mut app, &a);
    assert!(
        said(&app).contains("1 wallet and 3 key accounts"),
        "{}",
        said(&app)
    );
}

/// Loads `descriptor` as a wallet from a file.
fn load_from_file(app: &mut Faraday, descriptor: &str) {
    app.storage(StorageEvent::Restored {
        inbox: faraday_core::files_from(vec![(
            faraday_text::Text::of("wallet.txt"),
            descriptor.as_bytes().to_vec(),
        )]),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    testkit::load_wallet(app, 0);
    assert_eq!(app.session.wallets.len(), 1);
}

#[test]
fn a_wallet_written_with_h_is_the_wallet_a_find_names() {
    let mut app = with_key();
    let d = account_of_other(&app, NewKind::NativeSegwit).replace('\'', "h");
    assert!(d.contains('h') && !d.contains('\''), "{d}");
    load_from_file(&mut app, &d);
    read(&mut app, "tb1q6rz28mcfaxtmd6v789l9rrlrusdprr9pqcpvkl");
    assert!(!said(&app).contains("Found in key "), "{}", said(&app));
    assert!(said(&app).contains("Receive · index 0"), "{}", said(&app));
}

#[test]
fn a_wallet_of_another_script_leaves_the_key_searched_on_its_own() {
    let mut app = with_key();
    let d = account_of_other(&app, NewKind::Taproot).replace('\'', "h");
    load_from_file(&mut app, &d);
    // BIP-84 m/84'/1'/0'/0/0: the key's own account, no wallet for it.
    read(&mut app, "tb1q6rz28mcfaxtmd6v789l9rrlrusdprr9pqcpvkl");
    assert!(said(&app).contains("Found in key "), "{}", said(&app));
}

#[test]
fn a_find_in_a_single_key_wallet_shows_the_wallet_place_path_and_whole_address() {
    let d = single(0);
    let mut app = checking(std::slice::from_ref(&d));
    let a = address(&d, false, 12);
    read(&mut app, &a);
    let card = said(&app);
    let name = app.session.wallets[0].name.clone();
    assert!(
        card.contains(&format!("Found in {}", name.as_str())),
        "{card}"
    );
    assert!(
        card.contains(&format!("Address: {}", faraday_ui::grouped(&a).as_str())),
        "{card}"
    );
    assert!(card.contains("Place: Receive · index 12"), "{card}");
    assert!(
        card.contains("Wallet: Single key · native SegWit"),
        "{card}"
    );
    assert!(card.contains("Path: "), "{card}");
    assert!(card.contains("m/84h/1h/0h/0/12"), "{card}");
}

#[test]
fn a_find_in_a_multisig_wallet_lists_each_cosigner() {
    let d = testkit::savings();
    let mut app = checking(std::slice::from_ref(&d));
    read(&mut app, &address(&d, true, 3));
    let card = said(&app);
    let keys = app.session.wallets[0].policy().keys().len();
    assert!(keys >= 2);
    assert!(card.contains("Keys: "), "{card}");
    assert!(card.contains("Place: Change · index 3"), "{card}");
    assert_eq!(card.matches("/1/3").count(), keys, "{card}");
}

#[test]
fn a_find_in_a_keys_account_offers_to_make_its_wallet_and_then_to_open_it() {
    let mut app = with_key();
    testkit::keep_published(&mut app);
    read(&mut app, "tb1q6rz28mcfaxtmd6v789l9rrlrusdprr9pqcpvkl");
    let card = said(&app);
    assert!(
        card.contains("Account: Native SegWit · account 0"),
        "{card}"
    );
    assert!(card.contains("Path: "), "{card}");
    let sign = "Sign a message with this address";
    assert_eq!(ways(&app), [sign, "Make a wallet"]);
    app.press(way(1));
    assert_eq!(app.session.wallets.len(), 1);
    assert_eq!(app.sheet, Some(Sheet::MadeWallet));
    assert_eq!(ways(&app), [sign, "Open the wallet"]);
    // Done leaves the page as it was, with the wallet loaded.
    app.press(Action::X(XAction::MadeDone));
    assert_eq!(app.sheet, None);
    assert_eq!(app.screen, Screen::CheckAddress);
    assert_eq!(app.session.wallets.len(), 1);
}

#[test]
fn the_made_wallets_dialog_opens_the_wallet() {
    let mut app = with_key();
    testkit::keep_published(&mut app);
    read(&mut app, "tb1q6rz28mcfaxtmd6v789l9rrlrusdprr9pqcpvkl");
    app.press(way(1));
    assert_eq!(app.sheet, Some(Sheet::MadeWallet));
    let _ = app.frame();
    app.press(Action::X(XAction::MadeOpen));
    assert_eq!(app.sheet, None);
    assert_eq!(app.screen, Screen::Wallets);
    assert_eq!(app.wallet_index(), 0);
}

#[test]
fn back_from_the_opened_wallet_returns_to_the_address_check() {
    let mut app = with_key();
    testkit::keep_published(&mut app);
    read(&mut app, "tb1q6rz28mcfaxtmd6v789l9rrlrusdprr9pqcpvkl");
    let card = said(&app);
    app.press(way(1));
    let _ = app.frame();
    app.press(Action::X(XAction::MadeOpen));
    assert_eq!(app.screen, Screen::Wallets);
    let _ = app.frame();
    app.press(Action::Back);
    assert_eq!(app.screen, Screen::CheckAddress);
    assert_eq!(said(&app), card);
}

#[test]
fn a_find_among_a_files_several_addresses_offers_no_wallet() {
    let mut app = with_key();
    testkit::keep_published(&mut app);
    app.press(Action::Nav(Screen::Catalog));
    let tile = faraday_core::catalog::TILES
        .iter()
        .position(|t| t.name == "Check an address")
        .unwrap();
    app.press(Action::Catalog(tile as u8));
    assert_eq!(app.screen, Screen::CheckAddress);
    let text = "tb1q6rz28mcfaxtmd6v789l9rrlrusdprr9pqcpvkl\nmhLeXQvZ8zzN2idymESqznYm8EcZupZP4W\n";
    app.storage(StorageEvent::Restored {
        inbox: faraday_core::files_from(vec![(
            faraday_text::Text::of("list.txt"),
            text.as_bytes().to_vec(),
        )]),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app.press(Action::AddressCheck(AddressPress::File(0)));
    let _ = app.frame();
    app.press(Action::AddressCheck(AddressPress::Row(0)));
    assert!(said(&app).contains("Found in key"), "{}", said(&app));
    assert!(
        !ways(&app).contains(&"Make a wallet".to_string()),
        "{:?}",
        ways(&app)
    );
}

#[test]
fn a_wallet_made_from_a_child_seeds_account_shows_its_notice_once_after_the_dialog() {
    let mut app = with_key();
    testkit::keep_published(&mut app);
    let of = faraday_vault::child::ChildOf {
        parent: [1, 2, 3, 4],
        language: 0,
        words: 12,
        index: 7,
    };
    assert!(app.session.keys.set_child_of(0, Some(of)));
    read(&mut app, "tb1q6rz28mcfaxtmd6v789l9rrlrusdprr9pqcpvkl");
    app.press(way(1));
    assert_eq!(app.sheet, Some(Sheet::MadeWallet));
    app.press(Action::X(XAction::MadeDone));
    assert_eq!(app.sheet, Some(Sheet::ChildNotice));
    app.press(Action::Cancel);
    assert_eq!(app.sheet, None);
}

#[test]
fn a_miss_offers_look_further_and_names_what_it_searched() {
    let d = single(0);
    let mut app = checking(std::slice::from_ref(&d));
    read(&mut app, &address(&single(1), false, 7));
    let card = said(&app);
    let name = app.session.wallets[0].name.clone();
    assert!(card.starts_with("Not found"), "{card}");
    assert!(
        card.contains(&format!(
            "Searched: The first 100 receive and change addresses of {}",
            name.as_str()
        )),
        "{card}"
    );
    let ways = ways(&app);
    assert_eq!(ways.len(), 2);
    assert!(ways[0].starts_with("Look further"));
}

#[test]
fn homes_scan_of_an_address_opens_the_address_check_with_it_checked() {
    let d = single(0);
    let mut app = checking(std::slice::from_ref(&d));
    app.press(Action::Cancel);
    app.press(Action::Scan);
    read(&mut app, &address(&d, false, 4));
    let card = said(&app);
    assert!(card.contains("Place: Receive · index 4"), "{card}");
}

#[test]
fn with_nothing_loaded_the_address_check_says_to_load_a_key_or_a_wallet() {
    let mut app = testkit::started();
    app.press(Action::ScanFor(Ask::Address));
    read(&mut app, "tb1q6rz28mcfaxtmd6v789l9rrlrusdprr9pqcpvkl");
    assert_eq!(said(&app), "Load a key or a wallet first");
}

#[test]
fn another_scanners_read_card_of_an_address_leads_to_the_result() {
    let d = single(0);
    let mut app = checking(std::slice::from_ref(&d));
    app.press(Action::Cancel);
    app.press(Action::ScanSeed);
    read(&mut app, &address(&d, false, 4));
    let ways = &app.scan().unwrap().card.as_ref().unwrap().ways;
    assert_eq!(ways.len(), 1, "{}", said(&app));
    assert_eq!(ways[0].label().as_str(), "Check the address");
    app.press(Action::ReadGo(0));
    assert!(
        said(&app).contains("Place: Receive · index 4"),
        "{}",
        said(&app)
    );
}

#[test]
fn an_address_found_in_a_wallet_offers_to_open_the_wallet() {
    let (a, b) = (single(0), single(1));
    let mut app = checking(&[a, b.clone()]);
    read(&mut app, &address(&b, false, 5));
    let at = ways(&app)
        .iter()
        .position(|w| w == "Open the wallet")
        .unwrap_or_else(|| panic!("{:?}", ways(&app)));
    app.press(way(u8::try_from(at).unwrap()));
    assert_eq!(app.screen, Screen::Wallets);
    let _ = app.frame();
    assert!(
        app.offers(Action::Backup(1)),
        "the card of the wallet found"
    );
}
