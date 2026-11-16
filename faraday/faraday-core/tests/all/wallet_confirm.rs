//! A wallet of several keys is added only after the person holds the
//! confirmation sheet that shows its keys (`docs/decisions/F254.md`),
//! however it comes in; a wallet made only of keys loaded here, and a
//! wallet of one key, load with no sheet.

use faraday_core::testkit::{self, Kit};
use faraday_core::{Action, Faraday, Screen, Sheet, StorageEvent};
use faraday_shell_api::{App, BootState, DisplayInfo, Event, Key, SecureHardware};

use crate::common::texts;

fn multisig(account: u32) -> Kit {
    let p = format!("m/48'/1'/0'/{account}'");
    Kit {
        id: "confirm",
        name: "Confirmed",
        descriptor: format!(
            "wsh(sortedmulti(2,{}/<0;1>/*,{}/<0;1>/*,{}/<0;1>/*))",
            testkit::key(0, &p),
            testkit::key(1, &p),
            testkit::key(2, &p)
        ),
        cosigner: None,
        fee: 1_820,
    }
}

fn with_files(files: Vec<(&str, String)>) -> Faraday {
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
    app.storage(StorageEvent::Restored {
        inbox: faraday_core::files_from(
            files
                .into_iter()
                .map(|(n, t)| (n.to_string(), t.into_bytes()))
                .collect(),
        ),
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

fn screen(app: &mut Faraday) -> String {
    texts(app).join(" ")
}

#[test]
fn a_multisig_file_is_added_only_after_the_hold_and_cancel_adds_nothing() {
    let mut app = with_files(vec![("a-wallet.txt", multisig(0).descriptor)]);
    app.press(Action::LoadWallet(0));
    assert_eq!(app.sheet, Some(Sheet::WalletConfirm));
    assert!(app.session.wallets.is_empty());
    let shown = screen(&mut app);
    assert!(shown.contains("Hold to add this wallet"), "{shown}");
    assert!(shown.contains("Descriptor checksum"), "{shown}");

    app.press(Action::WalletCancel);
    assert_eq!(app.sheet, None);
    assert!(app.session.wallets.is_empty());

    app.press(Action::LoadWallet(0));
    app.press(Action::WalletHold);
    assert_eq!(app.sheet, None);
    assert_eq!(app.session.wallets.len(), 1);
}

#[test]
fn the_sheet_shows_each_cosigners_whole_key() {
    let kit = multisig(0);
    let mut app = with_files(vec![("a-wallet.txt", kit.descriptor.clone())]);
    app.press(Action::LoadWallet(0));
    let shown = screen(&mut app);
    for i in 0..3 {
        let xpub = testkit::key(i, "m/48'/1'/0'/0'");
        let key = xpub.rsplit(']').next().unwrap().split('/').next().unwrap();
        let first: String = key.chars().take(4).collect();
        let last: String = key.chars().skip(key.chars().count() - 3).collect();
        assert!(
            shown.contains(&first) && shown.contains(&last),
            "key {i} is shown: {shown}"
        );
    }
    assert!(shown.contains("Key 3"), "{shown}");
}

#[test]
fn a_key_loaded_here_is_yours_and_the_wallet_is_still_asked_about() {
    let mut app = with_files(vec![("a-wallet.txt", multisig(0).descriptor)]);
    add_key(&mut app, 0);
    app.press(Action::LoadWallet(0));
    assert_eq!(app.sheet, Some(Sheet::WalletConfirm));
    let shown = screen(&mut app);
    assert!(shown.contains("Yours"), "{shown}");
}

#[test]
fn a_wallet_of_loaded_keys_only_loads_with_no_sheet() {
    let mut app = with_files(vec![("a-wallet.txt", multisig(0).descriptor)]);
    for n in 0..3 {
        add_key(&mut app, n);
    }
    app.press(Action::LoadWallet(0));
    assert_eq!(app.sheet, None);
    assert_eq!(app.session.wallets.len(), 1);
}

#[test]
fn the_same_wallet_is_not_asked_about_again() {
    let mut app = with_files(vec![("a-wallet.txt", multisig(0).descriptor)]);
    app.press(Action::LoadWallet(0));
    app.press(Action::WalletHold);
    app.press(Action::Nav(Screen::Files));
    app.press(Action::LoadWallet(0));
    assert_eq!(app.sheet, None);
    assert_eq!(app.session.wallets.len(), 1);
}

#[test]
fn a_single_key_wallet_loads_with_no_sheet() {
    let one = format!("wpkh({}/<0;1>/*)", testkit::key(0, "m/84'/1'/0'"));
    let mut app = with_files(vec![("one-wallet.txt", one)]);
    app.press(Action::LoadWallet(0));
    assert_eq!(app.sheet, None);
    assert_eq!(app.session.wallets.len(), 1);
}

#[test]
fn two_wallets_in_one_load_are_asked_one_after_the_other() {
    let mut app = with_files(vec![
        ("a-wallet.txt", multisig(0).descriptor),
        ("b-wallet.txt", multisig(1).descriptor),
    ]);
    app.press(Action::InboxLoad);
    assert_eq!(app.sheet, Some(Sheet::WalletConfirm));
    assert!(screen(&mut app).contains("1 of 2"));

    // Cancel drops only the wallet on the sheet, and the next is asked.
    app.press(Action::WalletCancel);
    assert_eq!(app.sheet, Some(Sheet::WalletConfirm));
    assert!(app.session.wallets.is_empty());
    assert!(screen(&mut app).contains("Add this wallet"));
    app.press(Action::WalletHold);
    assert_eq!(app.sheet, None);
    assert_eq!(app.session.wallets.len(), 1);
}

#[test]
fn escape_on_the_sheet_drops_that_wallet_and_asks_the_next() {
    let mut app = with_files(vec![
        ("a-wallet.txt", multisig(0).descriptor),
        ("b-wallet.txt", multisig(1).descriptor),
    ]);
    app.press(Action::InboxLoad);
    assert!(screen(&mut app).contains("1 of 2"));

    app.event(Event::Key(Key::Escape));
    assert_eq!(app.sheet, Some(Sheet::WalletConfirm));
    assert!(app.session.wallets.is_empty());
    assert!(screen(&mut app).contains("Add this wallet"));

    app.event(Event::Key(Key::Escape));
    assert_eq!(app.sheet, None);
    assert!(app.session.wallets.is_empty());
}

#[test]
fn both_wallets_of_one_load_are_added_by_two_holds() {
    let mut app = with_files(vec![
        ("a-wallet.txt", multisig(0).descriptor),
        ("b-wallet.txt", multisig(1).descriptor),
    ]);
    app.press(Action::InboxLoad);
    app.press(Action::WalletHold);
    assert_eq!(app.sheet, Some(Sheet::WalletConfirm));
    app.press(Action::WalletHold);
    assert_eq!(app.sheet, None);
    assert_eq!(app.session.wallets.len(), 2);
}

#[test]
fn a_load_all_adds_the_single_key_wallet_at_once_and_asks_about_the_rest() {
    let single = format!("wpkh({}/<0;1>/*)", testkit::key(0, "m/84'/1'/0'"));
    let mut app = with_files(vec![
        ("one-wallet.txt", single),
        ("a-wallet.txt", multisig(0).descriptor),
        ("b-wallet.txt", multisig(1).descriptor),
    ]);
    app.press(Action::InboxLoad);
    assert_eq!(app.session.wallets.len(), 1);
    assert_eq!(app.sheet, Some(Sheet::WalletConfirm));
    assert!(screen(&mut app).contains("1 of 2"));
    app.press(Action::WalletHold);
    assert!(screen(&mut app).contains("2 of 2"));
    app.press(Action::WalletHold);
    assert_eq!(app.sheet, None);
    assert_eq!(app.session.wallets.len(), 3);
}
