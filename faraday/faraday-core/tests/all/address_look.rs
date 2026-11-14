//! The address check looks past the first hundred (`docs/QR.md` §2): an
//! address beyond them is offered **Look further** to a thousand and ten
//! thousand of each chain in every loaded wallet, found at the depth that
//! holds it, a miss names how far it looked and in how many wallets, and
//! **Stop** stops.

use faraday_bip::policy::WalletPolicy;
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

/// What the read card says.
fn said(app: &Faraday) -> String {
    app.scan()
        .and_then(|s| s.card.as_ref())
        .map(|k| {
            k.facts
                .iter()
                .map(|f| String::from(f.as_str()))
                .collect::<Vec<_>>()
                .join("\n")
        })
        .unwrap_or_default()
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
