//! Compare two PSBTs (`docs/COMPARE.md` §2): the Tools tile opens it;
//! Files' ⋮ Compare on the replacement opens it with the original as
//! First and the result shown; a scanned PSBT or transaction reaches its
//! card and not Files; loading the "abandon" key turns Cannot tell into
//! Only the fee is raised; Check a signer shows only for one unsigned
//! transaction.
//!
//! The pair is Bitcoin Core's own (`tools/vectors/psbt/bump-core-*`).

use faraday_bip::keys::Network;
use faraday_core::catalog::{Go, TILES};
use faraday_core::compare::{ComparePress, Slot};
use faraday_core::decode::check::SignerPress;
use faraday_core::file_menu::{FilePress, Row};
use faraday_core::strings::EN;
use faraday_core::testkit;
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
        .position(|t| t.go == Go::Compare)
        .expect("the tile is in Tools") as u8;
    app.press(Action::Nav(Screen::Catalog));
    app.press(Action::Catalog(tile));
}

fn texts(app: &mut Faraday) -> Vec<String> {
    let _ = app.frame();
    app.drawn_texts()
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

fn choose_compare(app: &mut Faraday, row: Row) {
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
        .find(|(l, _)| l == EN.file_menu_compare)
        .unwrap_or_else(|| panic!("Compare is in {lines:?}"));
    let _ = app.frame();
    app.press(*a);
}

/// Both files in Files, Compare chosen on the replacement.
fn pair() -> Faraday {
    let mut app = with_files(vec![
        ("original.psbt", original()),
        ("replacement.psbt", replacement()),
    ]);
    app.press(Action::Nav(Screen::Files));
    choose_compare(&mut app, Row::In(1));
    app
}

#[test]
fn the_tile_opens_the_flow() {
    let mut app = testkit::started();
    open(&mut app);
    assert_eq!(app.screen, Screen::Compare);
    assert!(holding(&mut app, EN.compare_fee_bump_label).is_empty());
}

#[test]
fn compare_on_the_replacement_opens_with_the_original_first_and_the_result_shown() {
    let mut app = pair();
    assert_eq!(app.screen, Screen::Compare);
    assert!(shows(&mut app, EN.compare_result));
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
    app.press(Action::Compare(ComparePress::Swap));
    assert_eq!(app.screen, Screen::Compare);
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
    choose_compare(&mut app, Row::In(1));
    // Two files spend the same inputs: nothing is chosen for First.
    app.press(Action::Compare(ComparePress::Pick(Slot::First, 0)));
    assert!(shows(&mut app, "original.psbt"));
    assert!(shows(&mut app, EN.bump_cannot_tell));
    // Change on First opens its card, which lists the third file.
    assert!(!shows(&mut app, "added.psbt"));
    app.press(Action::Compare(ComparePress::Card(0)));
    assert!(shows(&mut app, "added.psbt"), "{:?}", texts(&mut app));
    // The third file against the replacement: a comparison of its own.
    let _ = app.frame();
    let pick = app
        .drawn_actions()
        .into_iter()
        .find(|a| matches!(a, Action::Compare(ComparePress::Pick(Slot::First, 2))))
        .expect("the First card draws the third file");
    app.press(pick);
    assert_eq!(app.screen, Screen::Compare);
    let t = texts(&mut app);
    assert!(t.iter().any(|l| l == "added.psbt"), "{t:?}");
    assert!(t.iter().any(|l| l == "replacement.psbt"), "{t:?}");
    assert!(!t.iter().any(|l| l == "original.psbt"), "{t:?}");
    assert!(t.iter().any(|l| l == EN.compare_result), "{t:?}");
    assert!(t.iter().any(|l| l == EN.bump_more_changed), "{t:?}");
}

#[test]
fn a_scanned_psbt_reaches_its_card_and_not_files() {
    let mut app = testkit::started();
    open(&mut app);
    app.press(Action::Compare(ComparePress::Scan(Slot::First)));
    assert_eq!(app.sheet, Some(Sheet::Scan));
    read(&mut app, &original());
    assert_eq!(app.sheet, None);
    assert_eq!(app.screen, Screen::Compare);
    assert!(app.inbox().is_empty(), "nothing was filed");
    assert!(shows(&mut app, EN.signer_scanned));
    app.press(Action::Compare(ComparePress::Scan(Slot::Second)));
    read(&mut app, &replacement());
    assert_eq!(app.screen, Screen::Compare);
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
    app.press(Action::Compare(ComparePress::Scan(Slot::First)));
    read(&mut app, &vector("bump-core-original.tx"));
    assert_eq!(app.screen, Screen::Compare);
    let sheet = app.sheet;
    assert_eq!(sheet, None, "{:?}", texts(&mut app));
    assert!(app.inbox().is_empty());
}

#[test]
fn something_else_scanned_is_not_a_transaction() {
    let mut app = testkit::started();
    open(&mut app);
    app.press(Action::Compare(ComparePress::Scan(Slot::First)));
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
    app.press(Action::Compare(ComparePress::Card(2)));
    assert!(shows(&mut app, EN.bump_only_fee), "{:?}", texts(&mut app));
    assert!(!shows(&mut app, EN.bump_cannot_tell));
    app.session.keys.remove(0);
    app.press(Action::Compare(ComparePress::Card(2)));
    assert!(shows(&mut app, EN.bump_cannot_tell));
}

#[test]
fn check_a_signer_shows_for_one_transaction_and_opens_it_checked() {
    let mut app = with_files(vec![
        ("sent.psbt", vector("signer-core-sent.psbt")),
        ("signed.psbt", vector("signer-core-signed.psbt")),
    ]);
    app.press(Action::Nav(Screen::Files));
    // A file of the same unsigned transaction is not another one: the
    // flow opens with nothing chosen.
    choose_compare(&mut app, Row::In(1));
    assert_eq!(app.screen, Screen::Compare);
    app.press(Action::Compare(ComparePress::Pick(Slot::First, 0)));
    app.press(Action::Compare(ComparePress::Pick(Slot::Second, 1)));
    let t = texts(&mut app);
    assert!(t.iter().any(|l| l == EN.catalog_check_a_signer), "{t:?}");
    app.press(Action::Compare(ComparePress::CheckSigner));
    assert_eq!(app.screen, Screen::SignerCheck);
    assert!(shows(&mut app, EN.signer_only_signatures));
    app.press(Action::SignerCheck(SignerPress::Back));
}

#[test]
fn two_transactions_do_not_offer_check_a_signer() {
    let mut app = pair();
    assert!(holding(&mut app, EN.catalog_check_a_signer).is_empty());
}

#[test]
fn done_returns_where_the_flow_was_opened() {
    let mut app = pair();
    app.press(Action::Compare(ComparePress::Back));
    assert_eq!(app.screen, Screen::Files);
    let mut app = testkit::started();
    open(&mut app);
    app.press(Action::Compare(ComparePress::Back));
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
fn back_in_compare_opens_the_card_before_and_from_the_first_leaves() {
    for mouse in [false, true] {
        let mut app = testkit::started();
        open(&mut app);
        assert_eq!(app.screen, Screen::Compare);
        app.press(Action::Compare(ComparePress::Card(1)));
        assert!(shows(&mut app, &back_to(EN.compare_fact_first)));
        back(&mut app, mouse);
        assert_eq!(app.screen, Screen::Compare, "the first card, not Tools");
        assert!(shows(&mut app, &back_to(EN.catalog_tools)));
        app.event(Event::Forward);
        let _ = app.frame();
        assert_eq!(app.screen, Screen::Compare);
        assert!(
            shows(&mut app, &back_to(EN.compare_fact_first)),
            "Forward opens the card Back left"
        );
        back(&mut app, mouse);
        back(&mut app, mouse);
        assert_ne!(app.screen, Screen::Compare, "from the first card it leaves");
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
        choose_compare(&mut app, Row::In(1));
        app.press(Action::Compare(ComparePress::Pick(Slot::First, 0)));
        app.press(Action::Compare(ComparePress::Pick(Slot::Second, 1)));
        app.press(Action::Compare(ComparePress::CheckSigner));
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
