//! Two PSBTs compared in Check a signer (`docs/SIGNER-CHECK.md` §11): the
//! Tools tile opens it; Files' ⋮ Check a signer on the replacement opens
//! it with the original as Sent and the result shown; a scanned PSBT or
//! transaction reaches its card and not Files; loading the "abandon" key
//! turns Cannot tell into Only the fee is raised; the signer's verdict is
//! given for two PSBTs and a finished transaction on a card gets "The
//! signer check needs two PSBTs"; Tools and Files have no Compare.
//!
//! The pair is Bitcoin Core's own (`tools/vectors/psbt/bump-core-*`).

use faraday_bip::keys::Network;
use faraday_core::catalog::{Go, TILES};
use faraday_core::decode::check::{SignerPress, Slot};
use faraday_core::file_menu::{FilePress, Row};
use faraday_core::strings::EN;
use faraday_core::testkit;
use faraday_core::testkit::Plain;
use faraday_core::{Action, Faraday, Screen, Sheet, StorageEvent};
use faraday_shell_api::{App, Event};

const ABANDON: &str = "abandon abandon abandon abandon abandon abandon abandon abandon abandon \
                       abandon abandon about";

fn vector(name: &str) -> Vec<u8> {
    let path = format!(
        "{}/../../tools/vectors/psbt/{name}",
        env!("CARGO_MANIFEST_DIR")
    );
    std::fs::read(&path).unwrap_or_else(|e| panic!("{path}: {e}"))
}

fn original() -> Vec<u8> {
    vector("bump-core-original.psbt")
}

fn replacement() -> Vec<u8> {
    vector("bump-core-replacement.psbt")
}

/// The Core replacement whose inputs carry a previous transaction that is
/// not the one they spend (another lock time, so another txid), with the
/// `witness_utxo` beside it kept or dropped.
fn false_previous(keep_witness_utxo: bool) -> Vec<u8> {
    let c = testkit::compute();
    let text = String::from_utf8(vector("bump-core-replacement.psbt")).unwrap();
    let mut psbt = faraday_psbt::Psbt::parse_base64(&c, text.trim()).unwrap();
    for input in &mut psbt.inner_mut().inputs {
        let tx = input
            .non_witness_utxo
            .as_mut()
            .expect("Core's replacement holds the previous transaction");
        tx.lock_time = faraday_bip::bitcoin::absolute::LockTime::from_consensus(7);
        if !keep_witness_utxo {
            input.witness_utxo = None;
        }
    }
    psbt.to_base64().into_bytes()
}

fn with_files(files: Vec<(&str, Vec<u8>)>) -> Faraday {
    let mut app = testkit::started();
    app.storage(StorageEvent::Restored {
        inbox: faraday_core::files_from(
            files.into_iter().map(|(n, b)| (n.to_string(), b)).collect(),
        ),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app
}

fn open(app: &mut Faraday) {
    let tile = TILES
        .iter()
        .position(|t| t.go == Go::SignerCheck)
        .expect("the tile is in Tools") as u8;
    app.press(Action::Nav(Screen::Catalog));
    app.press(Action::Catalog(tile));
}

fn texts(app: &mut Faraday) -> Vec<String> {
    let _ = app.frame();
    app.drawn_texts().plain()
}

fn shows(app: &mut Faraday, line: &str) -> bool {
    texts(app).iter().any(|t| t == line)
}

fn holding(app: &mut Faraday, part: &str) -> Vec<String> {
    texts(app)
        .into_iter()
        .filter(|t| t.contains(part))
        .collect()
}

fn read(app: &mut Faraday, bytes: &[u8]) {
    app.event(Event::Scanned {
        bytes: bytes.to_vec(),
    });
    let _ = app.frame();
}

fn choose_check(app: &mut Faraday, row: Row) {
    let _ = app.frame();
    app.press(Action::File(FilePress::Menu(row)));
    assert_eq!(app.sheet, Some(Sheet::FileMenu));
    let lines = app
        .files_menu()
        .menu
        .as_ref()
        .expect("its menu")
        .lines
        .clone();
    let (_, a) = lines
        .iter()
        .find(|(l, _)| l == EN.catalog_check_a_signer)
        .unwrap_or_else(|| panic!("Check a signer is in {lines:?}"));
    let _ = app.frame();
    app.press(*a);
}

/// Both files in Files, Check a signer chosen on the replacement.
fn pair() -> Faraday {
    let mut app = with_files(vec![
        ("original.psbt", original()),
        ("replacement.psbt", replacement()),
    ]);
    app.press(Action::Nav(Screen::Files));
    choose_check(&mut app, Row::In(1));
    app
}

#[test]
fn the_tile_opens_the_flow() {
    let mut app = testkit::started();
    open(&mut app);
    assert_eq!(app.screen, Screen::SignerCheck);
    assert!(holding(&mut app, EN.compare_fee_bump_label).is_empty());
}

#[test]
fn check_on_the_replacement_opens_with_the_original_sent_and_the_result_shown() {
    let mut app = pair();
    assert_eq!(app.screen, Screen::SignerCheck);
    assert!(shows(&mut app, EN.signer_result));
    assert!(shows(&mut app, "original.psbt"), "{:?}", texts(&mut app));
    // No key is loaded: the change cannot be read.
    assert!(
        shows(&mut app, EN.bump_cannot_tell),
        "{:?}",
        texts(&mut app)
    );
}

#[test]
fn swap_makes_the_comparison_again() {
    let mut app = pair();
    let before = texts(&mut app);
    let at = |t: &[String], name: &str| t.iter().position(|l| l == name);
    let (first, second) = (
        at(&before, "original.psbt").expect("First"),
        at(&before, "replacement.psbt").expect("Second"),
    );
    assert!(first < second, "{before:?}");
    app.press(Action::SignerCheck(SignerPress::Swap));
    assert_eq!(app.screen, Screen::SignerCheck);
    let after = texts(&mut app);
    let (first, second) = (
        at(&after, "original.psbt").expect("Second"),
        at(&after, "replacement.psbt").expect("First"),
    );
    assert!(second < first, "{after:?}");
    // The swapped pair is compared again: the fee falls, so more than the
    // fee changed (the pair as it came was Cannot tell).
    assert!(after.iter().any(|l| l == EN.bump_more_changed), "{after:?}");
    assert_ne!(before, after);
}

#[test]
fn change_opens_a_card_and_choosing_a_file_compares_again() {
    let mut app = with_files(vec![
        ("original.psbt", original()),
        ("replacement.psbt", replacement()),
        ("added.psbt", vector("bump-core-added.psbt")),
    ]);
    app.press(Action::Nav(Screen::Files));
    choose_check(&mut app, Row::In(1));
    // Two files spend the same inputs: nothing is chosen for First.
    app.press(Action::SignerCheck(SignerPress::Pick(Slot::Sent, 0)));
    assert!(shows(&mut app, "original.psbt"));
    assert!(shows(&mut app, EN.bump_cannot_tell));
    // Change on First opens its card, which lists the third file.
    assert!(!shows(&mut app, "added.psbt"));
    app.press(Action::SignerCheck(SignerPress::Card(0)));
    assert!(shows(&mut app, "added.psbt"), "{:?}", texts(&mut app));
    // The third file against the replacement: a comparison of its own.
    let _ = app.frame();
    let pick = app
        .drawn_actions()
        .into_iter()
        .find(|a| matches!(a, Action::SignerCheck(SignerPress::Pick(Slot::Sent, 2))))
        .expect("the First card draws the third file");
    app.press(pick);
    assert_eq!(app.screen, Screen::SignerCheck);
    let t = texts(&mut app);
    assert!(t.iter().any(|l| l == "added.psbt"), "{t:?}");
    assert!(t.iter().any(|l| l == "replacement.psbt"), "{t:?}");
    assert!(!t.iter().any(|l| l == "original.psbt"), "{t:?}");
    assert!(t.iter().any(|l| l == EN.signer_result), "{t:?}");
    assert!(t.iter().any(|l| l == EN.bump_more_changed), "{t:?}");
}

#[test]
fn a_scanned_psbt_reaches_its_card_and_not_files() {
    let mut app = testkit::started();
    open(&mut app);
    app.press(Action::SignerCheck(SignerPress::Scan(Slot::Sent)));
    assert_eq!(app.sheet, Some(Sheet::Scan));
    read(&mut app, &original());
    assert_eq!(app.sheet, None);
    assert_eq!(app.screen, Screen::SignerCheck);
    assert!(app.inbox().is_empty(), "nothing was filed");
    assert!(shows(&mut app, EN.signer_scanned));
    app.press(Action::SignerCheck(SignerPress::Scan(Slot::Returned)));
    read(&mut app, &replacement());
    assert_eq!(app.screen, Screen::SignerCheck);
    assert!(app.inbox().is_empty());
    // Both scans were compared: the replacement is judged against the original.
    assert!(
        shows(&mut app, EN.bump_cannot_tell),
        "{:?}",
        texts(&mut app)
    );
}

#[test]
fn a_scanned_transaction_reaches_its_card() {
    let mut app = testkit::started();
    open(&mut app);
    app.press(Action::SignerCheck(SignerPress::Scan(Slot::Sent)));
    read(&mut app, &vector("bump-core-original.tx"));
    assert_eq!(app.screen, Screen::SignerCheck);
    let sheet = app.sheet;
    assert_eq!(sheet, None, "{:?}", texts(&mut app));
    assert!(app.inbox().is_empty());
}

#[test]
fn something_else_scanned_is_not_a_transaction() {
    let mut app = testkit::started();
    open(&mut app);
    app.press(Action::SignerCheck(SignerPress::Scan(Slot::Sent)));
    read(&mut app, b"not a transaction");
    assert_eq!(app.sheet, Some(Sheet::Scan));
    assert!(app.inbox().is_empty());
}

#[test]
fn loading_the_key_turns_cannot_tell_into_only_the_fee_is_raised() {
    let mut app = pair();
    assert!(shows(&mut app, EN.bump_cannot_tell));
    // The pair is Bitcoin Core's regtest: the session follows it.
    app.session.follow(Some(Network::Regtest)).unwrap();
    let c = testkit::compute();
    app.session.keys.add_words(&c, ABANDON, "", None).unwrap();
    app.press(Action::SignerCheck(SignerPress::Card(2)));
    assert!(shows(&mut app, EN.bump_only_fee), "{:?}", texts(&mut app));
    assert!(!shows(&mut app, EN.bump_cannot_tell));
    app.session.keys.remove(0);
    app.press(Action::SignerCheck(SignerPress::Card(2)));
    assert!(shows(&mut app, EN.bump_cannot_tell));
}

#[test]
fn one_transaction_in_two_psbts_is_checked_on_the_same_result() {
    let mut app = with_files(vec![
        ("sent.psbt", vector("signer-core-sent.psbt")),
        ("signed.psbt", vector("signer-core-signed.psbt")),
    ]);
    app.press(Action::Nav(Screen::Files));
    // A file of the same unsigned transaction is not another one: the
    // flow opens with nothing chosen.
    choose_check(&mut app, Row::In(1));
    assert_eq!(app.screen, Screen::SignerCheck);
    app.press(Action::SignerCheck(SignerPress::Pick(Slot::Sent, 0)));
    app.press(Action::SignerCheck(SignerPress::Pick(Slot::Returned, 1)));
    assert_eq!(app.screen, Screen::SignerCheck);
    assert!(shows(&mut app, EN.signer_only_signatures));
    app.press(Action::SignerCheck(SignerPress::Back));
}

#[test]
fn two_transactions_show_the_fee_bump_and_the_signer_check_together() {
    let mut app = pair();
    assert!(
        shows(&mut app, EN.compare_fee_bump_label),
        "{:?}",
        texts(&mut app)
    );
    assert!(
        holding(&mut app, EN.signer_needs_two_psbts).is_empty(),
        "both are PSBTs: the check runs"
    );
    let many = EN.signer_findings.replace("{}", "");
    assert!(
        texts(&mut app)
            .iter()
            .any(|t| t == EN.signer_one_finding || t.ends_with(many.trim_start())),
        "the verdict is on the same Result: {:?}",
        texts(&mut app)
    );
}

#[test]
fn a_finished_transaction_is_compared_and_not_signer_checked() {
    let mut app = testkit::started();
    open(&mut app);
    app.press(Action::SignerCheck(SignerPress::Scan(Slot::Sent)));
    read(&mut app, &original());
    app.press(Action::SignerCheck(SignerPress::Scan(Slot::Returned)));
    read(&mut app, &vector("bump-core-original.tx"));
    assert_eq!(app.screen, Screen::SignerCheck);
    assert!(
        shows(&mut app, EN.signer_needs_two_psbts),
        "{:?}",
        texts(&mut app)
    );
    for verdict in [
        EN.signer_only_signatures,
        EN.signer_only_nonces,
        EN.signer_not_the_same,
        EN.signer_one_finding,
    ] {
        assert!(!shows(&mut app, verdict), "no verdict: {verdict}");
    }
}

#[test]
fn tools_and_files_have_no_compare() {
    assert!(!TILES.iter().any(|t| t.name == "Compare"));
    let mut app = with_files(vec![("original.psbt", original())]);
    app.press(Action::Nav(Screen::Files));
    let _ = app.frame();
    app.press(Action::File(FilePress::Menu(Row::In(0))));
    let lines = app
        .files_menu()
        .menu
        .as_ref()
        .expect("its menu")
        .lines
        .clone();
    assert!(lines.iter().any(|(l, _)| l == EN.catalog_check_a_signer));
    assert!(!lines.iter().any(|(l, _)| l == "Compare"), "{lines:?}");
}

#[test]
fn done_returns_where_the_flow_was_opened() {
    let mut app = pair();
    app.press(Action::SignerCheck(SignerPress::Back));
    assert_eq!(app.screen, Screen::Files);
    let mut app = testkit::started();
    open(&mut app);
    app.press(Action::SignerCheck(SignerPress::Back));
    assert_eq!(app.screen, Screen::Catalog);
}

/// Back, as the arrow or the mouse's back button takes it.
fn back(app: &mut Faraday, mouse: bool) {
    if mouse {
        app.event(Event::Back);
    } else {
        app.press(Action::Back);
    }
    let _ = app.frame();
}

fn back_to(name: &str) -> String {
    faraday_core::strings::fill(EN.bar_back_to, &[name])
}

#[test]
fn back_in_the_tile_opens_the_card_before_and_from_the_first_leaves() {
    for mouse in [false, true] {
        let mut app = testkit::started();
        open(&mut app);
        assert_eq!(app.screen, Screen::SignerCheck);
        app.press(Action::SignerCheck(SignerPress::Card(1)));
        assert!(shows(&mut app, &back_to(EN.signer_sent)));
        back(&mut app, mouse);
        assert_eq!(app.screen, Screen::SignerCheck, "the first card, not Tools");
        assert!(shows(&mut app, &back_to(EN.catalog_tools)));
        app.event(Event::Forward);
        let _ = app.frame();
        assert_eq!(app.screen, Screen::SignerCheck);
        assert!(
            shows(&mut app, &back_to(EN.signer_sent)),
            "Forward opens the card Back left"
        );
        back(&mut app, mouse);
        back(&mut app, mouse);
        assert_ne!(
            app.screen,
            Screen::SignerCheck,
            "from the first card it leaves"
        );
    }
}

#[test]
fn back_in_check_a_signer_opens_the_card_before_and_from_the_first_leaves() {
    for mouse in [false, true] {
        let mut app = with_files(vec![
            ("sent.psbt", vector("signer-core-sent.psbt")),
            ("signed.psbt", vector("signer-core-signed.psbt")),
        ]);
        app.press(Action::Nav(Screen::Files));
        choose_check(&mut app, Row::In(1));
        app.press(Action::SignerCheck(SignerPress::Pick(Slot::Sent, 0)));
        app.press(Action::SignerCheck(SignerPress::Pick(Slot::Returned, 1)));
        assert_eq!(app.screen, Screen::SignerCheck);
        // The Result is open: Back opens Returned, named after Sent.
        assert!(shows(&mut app, &back_to(EN.signer_returned)));
        back(&mut app, mouse);
        assert_eq!(app.screen, Screen::SignerCheck);
        assert!(shows(&mut app, &back_to(EN.signer_sent)));
        app.event(Event::Forward);
        let _ = app.frame();
        assert!(shows(&mut app, &back_to(EN.signer_returned)));
        // Result, Returned, Sent: from Sent it leaves.
        back(&mut app, mouse);
        back(&mut app, mouse);
        assert_eq!(app.screen, Screen::SignerCheck);
        back(&mut app, mouse);
        assert_ne!(app.screen, Screen::SignerCheck, "from the first it leaves");
    }
}

#[test]
fn a_false_previous_transaction_gives_a_finished_transaction_no_fee() {
    for keep in [true, false] {
        let mut app = testkit::started();
        open(&mut app);
        app.press(Action::SignerCheck(SignerPress::Scan(Slot::Sent)));
        read(&mut app, &vector("bump-core-original.tx"));
        app.press(Action::SignerCheck(SignerPress::Scan(Slot::Returned)));
        read(&mut app, &false_previous(keep));
        // The pair is Bitcoin Core's regtest: with its key loaded the
        // verdict would be "only the fee is raised" if the amounts held.
        app.session.follow(Some(Network::Regtest)).unwrap();
        let c = testkit::compute();
        app.session.keys.add_words(&c, ABANDON, "", None).unwrap();
        app.press(Action::SignerCheck(SignerPress::Card(2)));
        assert!(
            shows(&mut app, EN.bump_cannot_tell),
            "{:?}",
            texts(&mut app)
        );
        assert!(!shows(&mut app, EN.bump_only_fee), "{:?}", texts(&mut app));
        assert!(holding(&mut app, "in the original").is_empty());
    }
}

/// Resizes to `width` x `height`, then whether `address` is on screen whole
/// in the Result: every character of it drawn on one screenful, in the
/// groups the card wraps it into, the card scrolled down until it does.
fn whole_on_screen(app: &mut Faraday, (width, height): (u16, u16), address: &str) -> bool {
    resize(app, width, height);
    let mut last: Vec<String> = Vec::new();
    for _ in 0..60 {
        let frame = texts(app);
        if frame.concat().replace(' ', "").contains(address) {
            return true;
        }
        if frame == last {
            return false;
        }
        last = frame;
        app.event(Event::Wheel {
            x: width / 2,
            y: height / 2,
            dy: 120,
        });
        app.settle();
    }
    false
}

const SIZES: [(u16, u16); 3] = [(1366, 768), (480, 640), (320, 480)];

/// The resize the helper needs.
fn resize(app: &mut Faraday, width: u16, height: u16) {
    app.event(Event::Display(faraday_shell_api::DisplayInfo {
        width,
        height,
        dpi: 160,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: faraday_shell_api::SecureHardware::None,
        boot: faraday_shell_api::BootState::Unknown,
        memory_mib: None,
    }));
    let _ = app.frame();
}

#[test]
fn a_line_that_names_an_output_gives_its_address_whole() {
    use faraday_bip::bitcoin::Address;
    let net = faraday_bip::bitcoin::Network::from(testkit::NET);
    let c = testkit::compute();
    let text = String::from_utf8(replacement()).expect("a text file");
    let psbt = faraday_psbt::Psbt::parse_base64(&c, text.trim()).expect("a PSBT");
    let addresses: Vec<String> = psbt
        .inner()
        .unsigned_tx
        .output
        .iter()
        .map(|o| {
            Address::from_script(&o.script_pubkey, net)
                .unwrap()
                .to_string()
        })
        .collect();
    for size in SIZES {
        let mut app = pair();
        let named: Vec<&String> = addresses
            .iter()
            .filter(|a| whole_on_screen(&mut app, size, a))
            .collect();
        assert!(!named.is_empty(), "{size:?}: {:?}", texts(&mut app));
    }
}
