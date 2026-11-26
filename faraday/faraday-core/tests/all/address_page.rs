//! Check an address, on the flow's one card or the compact panel's page: an address typed into its
//! field or pasted is checked, a `bitcoin:` link read to its address, a
//! refusal named under the field, a secret refused and the field
//! emptied, and a scanned address lands on the page.

use crate::common::{has, offered, through_fade};
use faraday_bip::policy::WalletPolicy;
use faraday_core::address_check::AddressPress;
use faraday_core::edit::Here;
use faraday_core::strings::EN;
use faraday_core::{Action, Ask, Faraday, Screen, StorageEvent, testkit};
use faraday_shell_api::{App, BootState, DisplayInfo, Event, FileKind, Key, SecureHardware};

const ABANDON: &str =
    "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

fn descriptor() -> String {
    format!("wpkh({}/<0;1>/*)", testkit::key(0, "m/84h/1h/0h"))
}

/// The wallet's address at `index`.
pub(crate) fn address(index: u32) -> String {
    let c = testkit::compute();
    WalletPolicy::parse_any(&c, &descriptor())
        .unwrap()
        .address_at(&c, testkit::net(), false, index)
        .unwrap()
        .to_string()
}

/// Faraday with the wallet loaded and the page open.
pub(crate) fn on_page() -> Faraday {
    on_page_with(Vec::new())
}

/// The same with `files` in Files after the wallet's own, at places 1 on.
pub(crate) fn on_page_with(files: Vec<(&str, Vec<u8>)>) -> Faraday {
    let mut app = testkit::started();
    let mut inbox = vec![(
        faraday_text::Text::of("wallet.txt"),
        descriptor().into_bytes(),
    )];
    inbox.extend(
        files
            .into_iter()
            .map(|(n, b)| (faraday_text::Text::of(n), b)),
    );
    app.storage(StorageEvent::Restored {
        inbox: faraday_core::files_from(inbox),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app.press(Action::LoadWallet(0));
    app.press(Action::Nav(Screen::Catalog));
    let i = faraday_core::catalog::TILES
        .iter()
        .position(|t| t.name == "Check an address")
        .unwrap();
    app.press(Action::Catalog(i as u8));
    assert_eq!(app.screen, Screen::CheckAddress);
    app
}

fn type_in(app: &mut Faraday, text: &str) {
    for c in text.chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    app.event(Event::Key(Key::Enter));
    let _ = app.frame();
}

pub(crate) fn said(app: &Faraday) -> String {
    let page = app.address_check_state().unwrap();
    let mut out: Vec<String> = page
        .result_lines()
        .iter()
        .map(|l| String::from(l.as_str()))
        .collect();
    out.extend(page.list_lines().iter().map(|l| String::from(l.as_str())));
    out.extend(page.refusal().map(|r| String::from(r.as_str())));
    out.join("\n")
}

#[test]
fn a_typed_address_is_checked_on_enter_and_found() {
    let mut app = on_page();
    let a = address(3);
    for c in a.chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    assert!(said(&app).is_empty(), "nothing is checked before Enter");
    app.event(Event::Key(Key::Enter));
    assert!(said(&app).contains("Receive · index 3"), "{}", said(&app));
}

#[test]
fn a_payment_link_is_read_to_its_address() {
    let mut app = on_page();
    type_in(&mut app, &format!("bitcoin:{}?amount=0.1", address(2)));
    assert!(said(&app).contains("Receive · index 2"), "{}", said(&app));
    assert_eq!(
        app.address_check_state().unwrap().typed().as_str(),
        address(2),
        "the field shows the address"
    );
}

#[test]
fn another_networks_address_is_refused_naming_both() {
    let mut app = on_page();
    type_in(&mut app, "bc1qcr8te4kr609gcawutmrza0j4xv80jy8z306fyu");
    let s = said(&app);
    assert!(s.contains("this wallet is"), "{s}");
    assert!(!s.contains("Receive"), "{s}");
}

#[test]
fn a_bad_link_and_a_non_address_are_refused() {
    let mut app = on_page();
    type_in(&mut app, "bitcoin:nonsense!!");
    assert_eq!(said(&app), "That payment link is not well formed");
    let mut app = on_page();
    type_in(&mut app, "hello");
    assert_eq!(said(&app), "Not a Bitcoin address");
    let mut app = on_page();
    type_in(&mut app, &format!("{}x", &address(0)[..30]));
    assert_eq!(
        said(&app),
        "That address fails its checksum: a character in it is wrong"
    );
}

#[test]
fn a_pasted_secret_is_refused_and_the_field_emptied() {
    let mut app = on_page();
    app.clipboard = true;
    app.press(Action::AddressCheck(AddressPress::Paste));
    app.event(Event::Clipboard {
        kind: FileKind::Text,
        text: faraday_text::Text::of(&address(1)),
    });
    assert!(said(&app).contains("Receive · index 1"), "{}", said(&app));
    app.press(Action::AddressCheck(AddressPress::Paste));
    app.event(Event::Clipboard {
        kind: FileKind::Text,
        text: faraday_text::Text::of(ABANDON),
    });
    let page = app.address_check_state().unwrap();
    assert!(page.typed().is_empty(), "the field is emptied");
    assert!(page.refusal().is_some(), "the words are refused and named");
    assert!(page.result_lines().is_empty());
}

#[test]
fn a_scanned_address_opens_the_page_with_its_result() {
    let mut app = on_page();
    app.press(Action::Nav(Screen::Catalog));
    app.press(Action::ScanFor(Ask::Address));
    app.event(Event::Scanned {
        bytes: address(4).into_bytes(),
    });
    let _ = app.frame();
    assert_eq!(app.screen, Screen::CheckAddress);
    assert!(said(&app).contains("Receive · index 4"), "{}", said(&app));
}

const OTHER_NETWORK: &str = "bc1qcr8te4kr609gcawutmrza0j4xv80jy8z306fyu";

fn read_file(app: &mut Faraday, at: u16) {
    app.press(Action::AddressCheck(AddressPress::File(at)));
    let _ = app.frame();
}

#[test]
fn a_file_of_several_addresses_lists_each_with_its_answer() {
    // Past the first 100, so not found.
    let stranger = &address(150);
    let text = format!(
        "pay to {}\nand {}, or {}\n{}\n",
        address(5),
        address(1),
        stranger,
        OTHER_NETWORK
    );
    let mut app = on_page_with(vec![("list.txt", text.into_bytes())]);
    read_file(&mut app, 1);
    let said = said(&app);
    assert!(said.contains("4 of 4 addresses read"), "{said}");
    let line = |a: &str| said.lines().find(|l| l.starts_with(a)).unwrap().to_string();
    assert!(line(&address(5)).contains("Receive · index 5"), "{said}");
    assert!(line(&address(1)).contains("Receive · index 1"), "{said}");
    assert!(line(stranger).contains("Not found"), "{said}");
    assert!(line(OTHER_NETWORK).contains("this wallet is"), "{said}");
}

#[test]
fn a_press_on_a_listed_address_shows_its_answer_and_back_returns_to_the_list() {
    let text = format!("{}\n{}\n", address(2), address(7));
    let mut app = on_page_with(vec![("list.txt", text.into_bytes())]);
    read_file(&mut app, 1);
    app.press(Action::AddressCheck(AddressPress::Row(1)));
    assert!(said(&app).contains("Receive · index 7"), "{}", said(&app));
    assert!(!said(&app).contains("2 of 2"), "the list is not shown");
    app.press(Action::AddressCheck(AddressPress::Back));
    assert!(
        said(&app).contains("2 of 2 addresses read"),
        "{}",
        said(&app)
    );
    assert_eq!(app.screen, Screen::CheckAddress);
}

#[test]
fn a_file_of_one_address_is_checked_as_typed() {
    let mut app = on_page_with(vec![(
        "one.txt",
        format!("my address: {}\n", address(3)).into_bytes(),
    )]);
    read_file(&mut app, 1);
    assert!(said(&app).contains("Receive · index 3"), "{}", said(&app));
    assert!(!said(&app).contains("addresses read"));
}

#[test]
fn a_file_over_64_kib_or_not_text_is_refused() {
    let mut big = format!("{}\n", address(0)).into_bytes();
    big.resize(64 * 1024 + 1, b' ');
    let mut app = on_page_with(vec![
        ("big.txt", big),
        ("binary.bin", vec![0xff, 0xfe, 0x00, 0x01]),
        ("empty.txt", Vec::new()),
    ]);
    for at in 1..=3 {
        read_file(&mut app, at);
        assert_eq!(said(&app), "Not a text file", "file {at}");
    }
}

#[test]
fn a_file_of_exactly_64_kib_is_taken() {
    let mut exact = format!("{}\n", address(0)).into_bytes();
    exact.resize(64 * 1024, b' ');
    let mut app = on_page_with(vec![("exact.txt", exact)]);
    read_file(&mut app, 1);
    assert!(said(&app).contains("Receive · index 0"), "{}", said(&app));
}

#[test]
fn a_descriptor_file_is_named_and_not_read_as_addresses() {
    let mut app = on_page();
    read_file(&mut app, 0);
    assert_eq!(said(&app), "A descriptor, not an address");
}

/// Where the frame last drawn offers `action`: its left and top, pixels.
fn rect(app: &Faraday, action: Action) -> (u16, u16) {
    let (x, y, ..) = app.hit_box(action).expect("the action is offered");
    (x, y)
}

/// On a wide window the ways in are the pick at the left of the card, and
/// they stay beside the result.
#[test]
fn a_wide_window_has_the_ways_in_at_the_left_and_the_result_on_the_card() {
    let mut app = on_page();
    assert!(has(&mut app, EN.invoice_way_type));
    assert!(offered(&mut app, Action::AddressCheck(AddressPress::Scan)));
    type_in(&mut app, &address(3));
    through_fade(&mut app);
    assert!(said(&app).contains("Receive · index 3"), "{}", said(&app));
    assert!(has(&mut app, EN.decode_card_result));
    let scan = rect(&app, Action::AddressCheck(AddressPress::Scan));
    let again = rect(&app, Action::AddressCheck(AddressPress::Another));
    assert!(scan.0 < again.0, "the result stands on the card");
}

/// The compact panel keeps the page: no pick, the ways in under the field.
#[test]
fn a_compact_panel_keeps_the_page() {
    let mut app = on_page();
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
    through_fade(&mut app);
    assert!(!has(&mut app, EN.invoice_way_type));
    let field = rect(&app, Action::TypeHere(Here::CheckAddress));
    let scan = rect(&app, Action::AddressCheck(AddressPress::Scan));
    assert!(scan.1 > field.1, "the ways in stand under the field");
}
