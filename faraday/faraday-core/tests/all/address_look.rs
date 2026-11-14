//! The address check looks past the first hundred (`docs/QR.md` §2): an
//! address beyond them is offered **Look further** to a thousand and ten
//! thousand of each chain in every loaded wallet, found at the depth that
//! holds it, a miss names how far it looked and in how many wallets, and
//! **Stop** stops.

use faraday_bip::policy::WalletPolicy;
use faraday_core::create::NewKind;
use faraday_core::{Action, Ask, Faraday, StorageEvent, testkit};
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
        app.press(Action::LoadWallet(0));
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
    app.event(Event::Scanned {
        bytes: text.as_bytes().to_vec(),
    });
    let _ = app.frame();
}

/// What the read card says, or the scanner's note where an address found
/// at once leaves no card.
fn said(app: &Faraday) -> String {
    let Some(scan) = app.scan() else {
        return String::new();
    };
    match scan.card.as_ref() {
        Some(k) => k
            .facts
            .iter()
            .map(|f| String::from(f.as_str()))
            .collect::<Vec<_>>()
            .join("\n"),
        None => scan
            .note
            .as_ref()
            .map(|n| String::from(n.as_str()))
            .unwrap_or_default(),
    }
}

/// Ticks a millisecond apart until nothing looks any more.
fn look_through(app: &mut Faraday) {
    for t in 1..100_000u64 {
        app.event(Event::Tick { now_ms: t });
        if !said(app).starts_with("Looking") {
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
    app.press(Action::ReadGo(0));
    look_through(&mut app);
    assert!(said(&app).contains("receive address 150"), "{}", said(&app));
}

#[test]
fn a_change_address_at_5000_is_found_by_looking_to_ten_thousand() {
    let d = single(0);
    let mut app = checking(std::slice::from_ref(&d));
    read(&mut app, &address(&d, true, 5_000));
    app.press(Action::ReadGo(1));
    look_through(&mut app);
    assert!(said(&app).contains("change address 5000"), "{}", said(&app));
}

#[test]
fn a_multisig_address_past_100_is_found() {
    let d = testkit::savings();
    let mut app = checking(std::slice::from_ref(&d));
    read(&mut app, &address(&d, false, 120));
    app.press(Action::ReadGo(0));
    look_through(&mut app);
    assert!(said(&app).contains("receive address 120"), "{}", said(&app));
}

#[test]
fn a_miss_says_how_far_it_looked_and_in_how_many_wallets() {
    let mut app = checking(&[single(0), testkit::savings()]);
    read(&mut app, &address(&single(1), false, 7));
    assert!(said(&app).contains("first 100"), "{}", said(&app));
    assert!(said(&app).contains("2 loaded wallets"), "{}", said(&app));
    app.press(Action::ReadGo(0));
    look_through(&mut app);
    assert!(said(&app).contains("first 1000"), "{}", said(&app));
    assert!(said(&app).contains("2 loaded wallets"), "{}", said(&app));
}

#[test]
fn stop_ends_the_look() {
    let mut app = checking(&[single(0)]);
    read(&mut app, &address(&single(1), false, 7));
    app.press(Action::ReadGo(1));
    app.event(Event::Tick { now_ms: 1 });
    assert!(said(&app).starts_with("Looking"), "{}", said(&app));
    // Stop is the card's only way while it looks.
    app.press(Action::ReadGo(0));
    let stopped = said(&app);
    assert!(stopped.contains("not among the first"), "{stopped}");
    for t in 2..50 {
        app.event(Event::Tick { now_ms: t });
    }
    assert_eq!(said(&app), stopped);
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
    // Account 0 of `OTHER` on Mainnet: BIP-84's and BIP-86's own vectors,
    // BIP-44's made with embit 0.8.0 (BIP-44 publishes none).
    for (address, line) in [
        // BIP-84: m/84'/0'/0'/0/0
        (
            "bc1qcr8te4kr609gcawutmrza0j4xv80jy8z306fyu",
            "receive address 0",
        ),
        // BIP-84: m/84'/0'/0'/1/0
        (
            "bc1q8c6fshw2dlwun7ekn9qwf37cu2rn755upcp6el",
            "change address 0",
        ),
        // BIP-86: m/86'/0'/0'/0/0
        (
            "bc1p5cyxnuxmeuwuvkwfem96lqzszd02n6xdcjrs20cac6yqjjwudpxqkedrcr",
            "receive address 0",
        ),
        // BIP-44: m/44'/0'/0'/0/0, made with embit 0.8.0
        ("1LqBGSKuX5yYUonjxT5qGfpUsXKYYWeabA", "receive address 0"),
    ] {
        let mut app = with_key();
        read(&mut app, address);
        assert!(said(&app).contains(line), "{address}: {}", said(&app));
        assert!(said(&app).contains("is key "), "{address}: {}", said(&app));
    }
    let mut app = with_key();
    read(
        &mut app,
        "bc1p5cyxnuxmeuwuvkwfem96lqzszd02n6xdcjrs20cac6yqjjwudpxqkedrcr",
    );
    assert!(said(&app).contains("m/86'/0'/0'"), "{}", said(&app));
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
    assert!(said(&app).contains("receive address 0"), "{}", said(&app));
    assert!(said(&app).contains("m/49'/1'/0'"), "{}", said(&app));
}

#[test]
fn a_standard_account_is_looked_further_in() {
    let mut app = with_key();
    // BIP-44 m/44'/0'/0'/0/150 of `OTHER`, made with embit 0.8.0 (Python): `HDKey.from_seed(bip39.mnemonic_to_seed(words))`, `derive("m/44h/0h/0h/0/150")`, `script.p2pkh(..).address()`; its index 0 is the embit address above.
    read(&mut app, "1B2iaW2V1eutqAyXQxgussmdjeqDz4PPvw");
    assert!(said(&app).contains("first 100"), "{}", said(&app));
    app.press(Action::ReadGo(0));
    look_through(&mut app);
    assert!(said(&app).contains("receive address 150"), "{}", said(&app));
    assert!(said(&app).contains("m/44'/0'/0'"), "{}", said(&app));
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
    app.press(Action::LoadWallet(0));
    assert_eq!(app.session.wallets.len(), 1);
    // BIP-84: m/84'/0'/0'/0/0
    read(&mut app, "bc1qcr8te4kr609gcawutmrza0j4xv80jy8z306fyu");
    // The wallet's own line, not the account's.
    assert!(!said(&app).contains("is key "), "{}", said(&app));
    assert!(said(&app).contains("receive address 0"), "{}", said(&app));
    // A miss counts the wallet and the three accounts with none.
    let a = address_in(&app, &single(1), false, 7);
    read(&mut app, &a);
    assert!(
        said(&app).contains("4 loaded wallets and key accounts"),
        "{}",
        said(&app)
    );
}
