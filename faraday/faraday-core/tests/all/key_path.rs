//! A `tr()` wallet with a script tree states who can spend by its key
//! path (`docs/decisions/F132.md`): a key that is not one of the
//! wallet's is named in the danger line, the wallet is added only after
//! a hold that names it, its card keeps the line, and a spend from it
//! repeats it as a caution. A wallet whose key path nobody can spend
//! reads as "2 of 3" with no warning.

use faraday_core::family::FamilyAction as F;
use faraday_core::testkit::{self, Kit};
use faraday_core::wallet::step;
use faraday_core::{Action, Faraday, Screen, Sheet, StorageEvent};
use faraday_shell_api::{App, BootState, DisplayInfo, Event, Key, SecureHardware};

use crate::common::texts;

const NUMS: &str = "50929b74c1a04954b78b4b6035e97a5e078a5a0f28ec96d547bfee9ace803ac0";
/// The generator's x coordinate: a valid key that is not one of the
/// wallet's.
const OTHER: &str = "79be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798";

fn kit(internal: &str) -> Kit {
    let p = "m/48'/1'/0'/3'";
    Kit {
        id: "shared",
        name: "Shared",
        descriptor: format!(
            "tr({internal},multi_a(2,{}/<0;1>/*,{}/<0;1>/*,{}/<0;1>/*))",
            testkit::key(0, p),
            testkit::key(1, p),
            testkit::key(2, p)
        ),
        cosigner: None,
        fee: 1_820,
    }
}

fn with_files(kit: &Kit) -> Faraday {
    with_more(kit, Vec::new())
}

/// The kit's wallet file and unsigned spend in Files, and `more` after them.
fn with_more(kit: &Kit, more: Vec<(String, Vec<u8>)>) -> Faraday {
    let mut app = Faraday::new();
    testkit::move_to(&mut app.session, testkit::NET).unwrap();
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
    let psbt = testkit::unsigned(kit).unwrap();
    app.storage(StorageEvent::Restored {
        inbox: vec![
            (
                "shared-wallet.txt".to_string(),
                kit.descriptor.clone().into_bytes(),
            ),
            ("shared-unsigned.psbt".to_string(), psbt.to_bytes()),
        ]
        .into_iter()
        .chain(more)
        .collect(),
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

/// Everything the frame draws, as one line: a wrapped line reads whole.
fn screen(app: &mut Faraday) -> String {
    texts(app).join(" ")
}

/// The danger line, as the screens write it.
fn danger() -> String {
    faraday_core::wallet::key_path_danger(OTHER)
}

#[test]
fn a_wallet_another_key_spends_alone_is_added_only_after_the_hold() {
    let mut app = with_files(&kit(OTHER));
    app.press(Action::LoadWallet(0));
    assert_eq!(app.sheet, Some(Sheet::KeyPath));
    assert!(app.session.wallets.is_empty(), "not added before the hold");
    let shown = screen(&mut app);
    assert!(
        shown.contains("Shared: this key is not one of its keys"),
        "the sheet names the wallet: {shown}"
    );
    assert!(
        shown.contains("79be 667e") && shown.contains("16f8 1798"),
        "the sheet names the key: {shown}"
    );
    assert!(shown.contains("Hold to add this wallet"), "{shown}");

    // Cancel: nothing is added, and nothing waits.
    app.press(Action::KeyPathCancel);
    assert_eq!(app.sheet, None);
    assert!(app.session.wallets.is_empty());
    assert!(app.session.key_path_asked().is_none());

    // Asked again, the hold adds it, and Files' load goes on as it would
    // have: to the wallet's card.
    app.press(Action::Nav(Screen::Files));
    app.press(Action::LoadWallet(0));
    assert_eq!(app.sheet, Some(Sheet::KeyPath));
    app.press(Action::KeyPathHold);
    assert_eq!(app.sheet, None);
    assert_eq!(app.session.wallets.len(), 1);
    assert_eq!(app.screen, Screen::Wallets);
    assert!(app.session.key_path_asked().is_none());
}

#[test]
fn restore_goes_on_from_the_description_once_the_hold_adds_it() {
    let mut app = with_files(&kit(OTHER));
    app.press(Action::RestoreWallet);
    app.press(Action::RUse(0));
    assert_eq!(app.sheet, Some(Sheet::KeyPath));
    assert_eq!(app.restore().unwrap().wallet, None);
    app.press(Action::KeyPathHold);
    assert_eq!(app.sheet, None);
    let r = app.restore().unwrap();
    assert_eq!(r.wallet, Some(0), "restore has the wallet");
    assert_eq!(r.error, None, "no line is left from the question");
    assert_eq!(app.screen, Screen::Restore);
}

#[test]
fn with_its_keys_here_the_card_counts_the_script_and_never_reads_can_spend() {
    let mut app = with_files(&kit(OTHER));
    add_key(&mut app, 0);
    app.press(Action::LoadWallet(0));
    app.press(Action::KeyPathHold);
    app.press(Action::Nav(Screen::Wallets));
    let shown = screen(&mut app);
    assert!(shown.contains("1 of 2 signatures"), "{shown}");
    add_key(&mut app, 1);
    app.press(Action::Nav(Screen::Wallets));
    let shown = screen(&mut app);
    assert!(shown.contains("2 of 2 signatures"), "{shown}");
    assert!(shown.contains("Another key can spend"), "{shown}");
    assert!(!shown.contains("Can spend"), "{shown}");
}

#[test]
fn the_card_keeps_the_danger_line_and_shows_no_quorum() {
    let mut app = with_files(&kit(OTHER));
    app.press(Action::LoadWallet(0));
    app.press(Action::KeyPathHold);
    app.press(Action::Nav(Screen::Wallets));
    let shown = screen(&mut app);
    assert!(shown.contains(&danger()), "{shown}");
    assert!(!shown.contains("2 of 3"), "{shown}");
}

#[test]
fn a_spend_from_it_repeats_the_line_as_a_caution() {
    let mut app = with_files(&kit(OTHER));
    add_key(&mut app, 0);
    add_key(&mut app, 1);
    app.press(Action::LoadWallet(0));
    app.press(Action::KeyPathHold);
    app.press(Action::Nav(Screen::Family));
    app.press(Action::Family(F::SpendFrom(0)));
    assert!(app.spend().is_some());
    app.press(Action::Step(step::TRANSACTION));
    let shown = screen(&mut app);
    assert!(shown.contains(&danger()), "{shown}");
}

#[test]
fn a_nums_wallet_reads_two_of_three_with_no_warning() {
    let mut app = with_files(&kit(NUMS));
    app.press(Action::LoadWallet(0));
    assert_eq!(app.sheet, None);
    assert_eq!(app.session.wallets.len(), 1);
    app.press(Action::Nav(Screen::Wallets));
    let shown = screen(&mut app);
    assert!(shown.contains("2 of 3"), "{shown}");
    assert!(shown.contains("Key path: none (unspendable)"), "{shown}");
    assert!(!shown.contains("can spend everything alone"), "{shown}");
}

#[test]
fn a_wallet_key_on_the_key_path_is_named_and_not_two_of_three() {
    let own = format!("{}/<0;1>/*", testkit::key(0, "m/48'/1'/0'/3'"));
    let mut app = with_files(&kit(&own));
    app.press(Action::LoadWallet(0));
    assert_eq!(app.sheet, None, "a key of the wallet needs no hold");
    app.press(Action::Nav(Screen::Wallets));
    let shown = screen(&mut app);
    assert!(shown.contains("Key path: "), "{shown}");
    assert!(shown.contains(" alone"), "{shown}");
    assert!(!shown.contains("2 of 3"), "{shown}");
}

#[test]
fn the_timelocked_tree_sample_names_its_key_path_holder() {
    let tree = testkit::kits()
        .into_iter()
        .find(|k| k.id == "tree")
        .unwrap();
    let mut app = with_files(&tree);
    app.press(Action::LoadWallet(0));
    assert_eq!(app.session.wallets.len(), 1);
    app.press(Action::Nav(Screen::Wallets));
    let shown = screen(&mut app);
    assert!(shown.contains("Key path: "), "{shown}");
    assert!(shown.contains(" alone"), "{shown}");
}

/// The names in Files, in order.
fn files(app: &Faraday) -> Vec<String> {
    app.inbox().iter().map(|i| i.name.clone()).collect()
}

#[test]
fn files_load_after_cancel_then_hold_ends_as_a_load_with_no_question() {
    let mut plain = with_files(&kit(NUMS));
    plain.press(Action::Nav(Screen::Files));
    plain.press(Action::LoadWallet(0));
    let mut app = with_files(&kit(OTHER));
    let before = files(&app);
    app.press(Action::Nav(Screen::Files));
    app.press(Action::LoadWallet(0));
    app.press(Action::KeyPathCancel);
    app.press(Action::LoadWallet(0));
    app.press(Action::KeyPathHold);
    assert_eq!(app.session.wallets.len(), 1, "added once");
    assert_eq!(files(&app), before, "no file moved or lost");
    assert_eq!(app.screen, plain.screen);
    assert_eq!(app.wallet_index(), plain.wallet_index());
}

#[test]
fn restore_after_the_hold_stands_where_a_restore_with_no_question_does() {
    let mut plain = with_files(&kit(NUMS));
    plain.press(Action::RestoreWallet);
    plain.press(Action::RUse(0));
    let mut app = with_files(&kit(OTHER));
    let before = files(&app);
    app.press(Action::RestoreWallet);
    app.press(Action::RUse(0));
    app.press(Action::KeyPathHold);
    let (r, p) = (app.restore().unwrap(), plain.restore().unwrap());
    assert_eq!((r.open, r.done, r.wallet), (p.open, p.done, p.wallet));
    assert_eq!(files(&app), before);
    assert_eq!(app.session.wallets.len(), 1);
}

#[test]
fn spend_from_a_backup_after_the_hold_has_the_wallet_as_with_no_question() {
    let mut plain = with_files(&kit(NUMS));
    plain.press(Action::Nav(Screen::Family));
    plain.press(Action::Family(F::UseWallet(0)));
    let mut app = with_files(&kit(OTHER));
    app.press(Action::Nav(Screen::Family));
    app.press(Action::Family(F::UseWallet(0)));
    assert_eq!(app.sheet, Some(Sheet::KeyPath));
    assert_eq!(app.family().wallet, None);
    app.press(Action::KeyPathHold);
    assert_eq!(app.family().wallet, plain.family().wallet);
    assert_eq!(
        app.family().came,
        plain.family().came,
        "each file noted once"
    );
    assert_eq!(app.family().error, None);
    assert_eq!(app.session.wallets.len(), 1);
}

#[test]
fn a_bulk_load_is_not_run_again_and_the_hold_adds_the_one_wallet() {
    let nums = kit(NUMS);
    let mut app = with_more(
        &kit(OTHER),
        vec![("nums-wallet.txt".to_string(), nums.descriptor.into_bytes())],
    );
    let before = files(&app);
    app.press(Action::InboxLoad);
    assert_eq!(app.sheet, Some(Sheet::KeyPath));
    assert_eq!(
        app.session.wallets.len(),
        1,
        "the other wallet loads at once"
    );
    app.press(Action::KeyPathHold);
    assert_eq!(app.session.wallets.len(), 2);
    assert_eq!(files(&app), before);
    assert!(screen(&mut app).contains("Shared added"));
}

#[test]
fn with_files_changed_under_the_sheet_the_hold_adds_the_wallet_itself() {
    let mut app = with_files(&kit(OTHER));
    app.press(Action::LoadWallet(0));
    testkit::inbox_retain(&mut app, |i| i.name != "shared-unsigned.psbt");
    app.press(Action::KeyPathHold);
    assert_eq!(app.session.wallets.len(), 1);
    assert_eq!(app.session.wallets[0].name, "Shared");
}
