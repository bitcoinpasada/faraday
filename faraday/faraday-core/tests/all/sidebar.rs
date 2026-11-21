//! The sidebar lists the tabs and lists neither keys nor wallets (they
//! are on Home). Its Files row names the stage of the session. Under its tab, each flow in progress has a
//! row, its name and open step, which returns to it as it was left.

#![expect(
    clippy::disallowed_methods,
    reason = "a test or example builds keys and seeds directly and is not shipped (docs/KEYS.md)"
)]

use faraday_bip::bip39::{Language, Mnemonic};
use faraday_core::testkit::KindText;
use faraday_core::testkit::Plain;
use faraday_core::{Action, Faraday, Screen, Sheet};
use faraday_shell_api::{App, BootState, DisplayInfo, Event, SecureHardware};

/// `seeds` seeds and `wallets` single-key wallets over the first of them,
/// on Files, in a 1366 × 800 window.
fn with(seeds: u8, wallets: usize) -> Faraday {
    with_height(seeds, wallets, 800)
}

/// `with`, at a chosen panel height, for a test that wants more room
/// than 800 to fit its counts.
fn with_height(seeds: u8, wallets: usize, height: u16) -> Faraday {
    let c = testkit::compute();
    let mut app = faraday_core::testkit::new_on_test_net();
    app.event(Event::Display(DisplayInfo {
        width: 1366,
        height,
        dpi: 160,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
    for i in 0..seeds {
        let m = Mnemonic::from_entropy(Language::English, &[i + 1; 16]).unwrap();
        let words: Vec<&str> = m
            .indices()
            .iter()
            .map(|&w| Language::English.words()[usize::from(w)])
            .collect();
        app.session
            .keys
            .add_words(&c, &words.join(" "), &format!("Seed {i}"), None)
            .unwrap();
    }
    for a in 0..wallets {
        let text = faraday_core::create::NewKind::NativeSegwit
            .key_text_at(&c, app.session.keys.master_for_tests(0), a as u32)
            .unwrap();
        let d = format!("wpkh({}/<0;1>/*)", &*text);
        app.session
            .add_wallet_confirmed(
                &c,
                &format!("Wallet {a}"),
                &d,
                "test",
                faraday_core::wallet::Origin::Named,
            )
            .unwrap_or_else(|e| panic!("{d}: {}", &*e.text()));
    }
    app.press(Action::Nav(Screen::Files));
    let _ = app.frame();
    app
}

fn fp(app: &Faraday, k: usize) -> [u8; 4] {
    app.session.keys[k].fingerprint.0
}

#[test]
fn the_sidebar_offers_guide_and_opens_the_guide_sheet() {
    let mut app = with(0, 0);
    assert!(app.offers(Action::Guide));
    app.press(Action::Guide);
    assert_eq!(app.sheet, Some(Sheet::Guide));
}

/// Spend left the sidebar: the Wallets empty
/// state and the guide sheet are its ways in now.
#[test]
fn the_sidebar_has_no_spend_row() {
    let app = with(0, 0);
    assert!(!app.offers(Action::Nav(Screen::SpendBackup)));
}

/// The Files tab carries one number, its badge (what the Inbox holds):
/// no "to write" beside it, for files brought in from the stick or for
/// those waiting to be written to it.
#[test]
fn the_files_tab_says_no_to_write_beside_its_badge() {
    let mut app = with(1, 0);
    app.storage(faraday_core::StorageEvent::Restored {
        inbox: faraday_core::files_from(vec![(
            faraday_text::Text::of("wallet.txt"),
            b"wpkh".to_vec(),
        )]),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app.press(Action::Nav(Screen::Home));
    assert!(!crate::common::has(&mut app, "1 to write"));
    app.outbox.push(faraday_core::Item {
        name: faraday_text::Text::of("a.psbt"),
        bytes: (Vec::new()).into(),
        kind: faraday_core::wallet::FileKind::Psbt,
        secret: false,
        picture: None,
        sha: None,
        facts: None,
        wallet: None,
        psbt: None,
        outpoints: Vec::new(),
        unsigned_txid: None,
    });
    assert!(!crate::common::has(&mut app, "1 to write"));
    assert!(app.offers(Action::Nav(Screen::Files)));
}

/// The sidebar lists the places in one order, Files once, and Power off
/// is named.
#[test]
fn the_sidebar_lists_the_places_in_order_with_files_once() {
    let mut app = with(1, 0);
    let _ = app.frame();
    let order = [
        Screen::Home,
        Screen::Check,
        Screen::WalletsStart,
        Screen::Files,
        Screen::Vaults,
        Screen::Catalog,
        Screen::Settings,
    ];
    let ys: Vec<u16> = order
        .iter()
        .map(|s| {
            app.where_offered(Action::Nav(*s))
                .unwrap_or_else(|| panic!("{s:?} is not in the sidebar"))
                .1
        })
        .collect();
    assert!(ys.windows(2).all(|w| w[0] < w[1]), "the order: {ys:?}");
    let files = app
        .offered_actions()
        .into_iter()
        .filter(|a| *a == Action::Nav(Screen::Files))
        .count();
    assert_eq!(files, 1, "Files is offered once");
    assert!(
        crate::common::has(&mut app, "Power off"),
        "Power off is named"
    );
}

/// With files waiting for the stick, the Files tab does not count them
/// a second time.
#[test]
fn the_files_tab_says_nothing_of_how_many_wait_to_be_written() {
    let mut app = with(1, 0);
    assert!(!crate::common::has(&mut app, "1 to write"));
    app.outbox.push(faraday_core::Item {
        name: faraday_text::Text::of("a.psbt"),
        bytes: (Vec::new()).into(),
        kind: faraday_core::wallet::FileKind::Psbt,
        secret: false,
        picture: None,
        sha: None,
        facts: None,
        wallet: None,
        psbt: None,
        outpoints: Vec::new(),
        unsigned_txid: None,
    });
    assert!(!crate::common::has(&mut app, "1 to write"));
}

// ---- flows in progress (`docs/FLOWS.md`, Flows in progress; F41) ----

use faraday_core::backup_flow::steps::FlowAction as F;
use faraday_core::backup_flow::{Kind, Step};
use faraday_core::in_progress::Going;
use faraday_core::{cstep, qstep, rstep, testkit};
use faraday_shell_api::Key;

/// Test seed `n` typed in as a key.
fn key_in(app: &mut Faraday, n: usize) {
    app.press(Action::Entry(None));
    type_in(app, &testkit::test_words(testkit::TEST_SEEDS[n].0));
    app.press(Action::EntryAdd);
    faraday_core::testkit::keep_published(app);
}

fn type_in(app: &mut Faraday, text: &str) {
    for c in text.chars() {
        app.event(Event::Key(Key::Char(c)));
    }
}

fn multi_index() -> u8 {
    faraday_core::create::NewKind::ALL
        .iter()
        .position(|k| *k == faraday_core::create::NewKind::Multi)
        .unwrap() as u8
}

/// A 2-of-3 made in Make a new wallet over three keys typed here, taken
/// on to its backup's Places.
fn create_at_places() -> Faraday {
    let mut app = testkit::started();
    for n in 0..3 {
        key_in(&mut app, n);
    }
    app.press(Action::CreateWallet);
    crate::common::ckind(&mut app, multi_index());
    for slot in 0..3u8 {
        let fp = app.session.keys[usize::from(slot)].fingerprint.0;
        app.press(Action::CSlotHere(slot, fp));
    }
    app.press(Action::CNext(cstep::KEYS));
    app.press(Action::CNext(cstep::CHECK));
    app.press(Action::CBackup);
    faraday_core::testkit::backup_as(&mut app, faraday_core::plan::Preset::Paper);
    assert_eq!(app.screen, Screen::Backup);
    assert_eq!(app.backup().unwrap().q, Some(qstep::BOARD));
    app
}

/// Whether the screen drawn now offers `action`.
fn offered(app: &mut Faraday, action: Action) -> bool {
    let _ = app.frame();
    app.offers(action)
}

/// The texts drawn in the sidebar, top to bottom.
fn sidebar_texts(app: &mut Faraday) -> Vec<String> {
    let _ = app.frame();
    // The Home row spans the sidebar but for its margins: twice its
    // middle is the sidebar's edge.
    let edge = i32::from(app.where_offered(Action::Nav(Screen::Home)).unwrap().0) * 2;
    let mut v: Vec<(String, i32, i32)> = app
        .drawn_text_boxes()
        .plain()
        .into_iter()
        .filter(|(_, (x, _, _, _))| *x < edge)
        .map(|(t, (x, y, _, _))| (t, y, x))
        .collect();
    v.sort_by_key(|&(_, y, x)| (y, x));
    v.into_iter().map(|(t, _, _)| t).collect()
}

/// The sidebar's row named `label` is drawn lit: just right of its name,
/// the row is not the sidebar's own colour.
fn lit(app: &mut Faraday, label: &str) -> bool {
    while app.poll_command().is_some() {}
    // The screen at rest, not on its way from the one before.
    app.settle();
    let _ = app.frame();
    let edge = i32::from(app.where_offered(Action::Nav(Screen::Home)).unwrap().0) * 2;
    let (x, y, w, h) = app
        .drawn_text_boxes()
        .into_iter()
        .find(|(t, (x, _, _, _))| t == label && *x < edge)
        .unwrap_or_else(|| panic!("no {label} in the sidebar"))
        .1;
    let frame = app.frame();
    let fw = usize::from(frame.width);
    let my = (y + h / 2) as usize;
    let at = |x: usize| {
        let i = (my * fw + x) * 4;
        frame.rgba[i..i + 4].to_vec()
    };
    let bg = at(1);
    let from = (x + w + 2) as usize;
    (from..from + 4).all(|x| at(x) != bg)
}

#[test]
fn make_a_new_wallet_left_on_places_for_settings_comes_back_as_it_was_left() {
    let mut app = create_at_places();
    let answers = app.backup().unwrap().answers.clone();
    app.press(Action::Nav(Screen::Settings));
    let row = Action::Resume(Going::Create);
    assert!(offered(&mut app, row));
    let names = sidebar_texts(&mut app);
    let at = names
        .iter()
        .position(|t| t == "Make a new wallet")
        .unwrap_or_else(|| panic!("no row: {names:?}"));
    assert_eq!(
        names.get(at + 1).map(String::as_str),
        Some("Places"),
        "{names:?}"
    );
    // Under Wallets, before Tools.
    let wallets = names.iter().position(|t| t == "Wallets").unwrap();
    let tools = names.iter().position(|t| t == "Tools").unwrap();
    assert!(wallets < at && at < tools, "{names:?}");

    app.press(row);
    assert_eq!(app.screen, Screen::Backup);
    let b = app.backup().unwrap();
    assert_eq!(b.q, Some(qstep::BOARD), "on Places");
    assert_eq!(b.answers, answers, "the board as it was left");
}

#[test]
fn no_row_before_a_step_is_done() {
    let mut app = testkit::started();
    app.press(Action::CreateWallet);
    app.press(Action::Nav(Screen::Settings));
    assert!(!offered(&mut app, Action::Resume(Going::Create)));
    app.press(Action::RestoreWallet);
    app.press(Action::Nav(Screen::Settings));
    assert!(!offered(&mut app, Action::Resume(Going::Restore)));
    // Its first step done: the row.
    app.press(Action::RestoreWallet);
    app.press(Action::RNext(rstep::KIND));
    app.press(Action::Nav(Screen::Settings));
    assert!(offered(&mut app, Action::Resume(Going::Restore)));
}

#[test]
fn the_row_goes_when_the_flow_is_started_over() {
    let mut app = testkit::started();
    app.press(Action::CreateWallet);
    crate::common::ckind(&mut app, 0);
    app.press(Action::Nav(Screen::Settings));
    assert!(offered(&mut app, Action::Resume(Going::Create)));
    app.press(Action::Resume(Going::Create));
    assert_eq!(app.screen, Screen::Create);
    app.press(Action::CreateOver);
    assert!(!offered(&mut app, Action::Resume(Going::Create)));
}

#[test]
fn the_row_goes_when_the_flow_is_finished() {
    let mut app = testkit::started();
    testkit::move_to(&mut app.session, testkit::NET).unwrap();
    app.press(Action::RestoreWallet);
    app.press(Action::RNext(rstep::KIND));
    app.press(Action::RSeeds);
    app.press(Action::Nav(Screen::Settings));
    assert!(offered(&mut app, Action::Resume(Going::Restore)));
    let names = sidebar_texts(&mut app);
    let at = names
        .iter()
        .position(|t| t.starts_with("Restore an existing"))
        .unwrap();
    assert_eq!(
        names.get(at + 1).map(String::as_str),
        Some("Keys"),
        "{names:?}"
    );
    app.press(Action::Resume(Going::Restore));
    assert_eq!(app.screen, Screen::Restore);
    app.press(Action::RSlotWords);
    type_in(&mut app, &testkit::test_words("bacon"));
    app.press(Action::EntryAdd);
    faraday_core::testkit::keep_published(&mut app);
    assert_eq!(app.screen, Screen::Restore);
    app.press(Action::Seeds(faraday_core::seeds::SeedsAction::Make));
    app.press(Action::RNext(rstep::CHECK));
    assert_eq!(app.restore().unwrap().open, Some(rstep::DONE));
    assert!(
        !offered(&mut app, Action::Resume(Going::Restore)),
        "restored: no row"
    );
}

#[test]
fn a_gpg_keys_flow_has_its_row_under_tools() {
    let mut app = crate::common::process(Vec::new(), Vec::new(), Vec::new());
    crate::common::start(&mut app, Kind::Gpg);
    assert_eq!(app.screen, Screen::BackupFlow);
    app.press(Action::Nav(Screen::Settings));
    assert!(
        !offered(&mut app, Action::Resume(Going::Flow(0))),
        "nothing done yet"
    );
    crate::common::start(&mut app, Kind::Gpg);
    app.press(Action::Flow(F::Continue));
    assert_eq!(app.flow().unwrap().step, Step::Sticks);
    app.press(Action::Nav(Screen::Settings));
    let names = sidebar_texts(&mut app);
    let tools = names.iter().position(|t| t == "Tools").unwrap();
    let learn = names.iter().position(|t| t == "Guide").unwrap();
    let row = names
        .iter()
        .position(|t| t == "GPG key")
        .unwrap_or_else(|| panic!("no row: {names:?}"));
    assert!(tools < row && row < learn, "under Tools: {names:?}");
    app.press(Action::Resume(Going::Flow(0)));
    assert_eq!(app.screen, Screen::BackupFlow);
    assert_eq!(app.flow().unwrap().step, Step::Sticks);
}

#[test]
fn the_row_is_lit_on_its_flows_screen_and_its_tab_is_not() {
    let mut app = create_at_places();
    assert!(lit(&mut app, "Make a new wallet"), "the row");
    assert!(!lit(&mut app, "Wallets"), "not Wallets");
    app.press(Action::Nav(Screen::Settings));
    assert!(!lit(&mut app, "Make a new wallet"));
    assert!(lit(&mut app, "Settings"), "Settings is");
    // On any other screen of the tab, the tab is lit as before.
    app.press(Action::Nav(Screen::WalletsStart));
    assert!(lit(&mut app, "Wallets"));
    assert!(!lit(&mut app, "Make a new wallet"));
}

#[test]
fn a_lock_wipes_what_it_wipes_and_a_kept_flow_keeps_its_row() {
    let mut shell = crate::common::Shell::new();
    let mut app = crate::common::process(Vec::new(), Vec::new(), Vec::new());
    crate::common::start(&mut app, Kind::Gpg);
    app.press(Action::Flow(F::Continue));
    app.press(Action::CreateWallet);
    crate::common::ckind(&mut app, 0);
    assert!(offered(&mut app, Action::Resume(Going::Create)));
    let mut app = shell.lock(&mut app);
    assert!(
        !offered(&mut app, Action::Resume(Going::Create)),
        "Make a new wallet is wiped"
    );
    assert!(
        offered(&mut app, Action::Resume(Going::Flow(0))),
        "the GPG key's flow is kept"
    );
}

// ---- Cancel: leaving a flow for good (`docs/FLOWS.md`, Cancel; F51) ----

/// Whether the flow's row is in the sidebar, on Settings.
fn row_on_settings(app: &mut Faraday, g: Going) -> bool {
    app.press(Action::Nav(Screen::Settings));
    offered(app, Action::Resume(g))
}

/// The texts drawn now, the sheet's among them.
fn texts(app: &mut Faraday) -> Vec<String> {
    let _ = app.frame();
    app.drawn_texts().plain()
}

/// [`create_at_places`], Make a new wallet opened from Wallets.
fn create_at_places_from_wallets() -> Faraday {
    let mut app = testkit::started();
    for n in 0..3 {
        key_in(&mut app, n);
    }
    app.press(Action::Nav(Screen::WalletsStart));
    app.press(Action::CreateWallet);
    crate::common::ckind(&mut app, multi_index());
    for slot in 0..3u8 {
        let fp = app.session.keys[usize::from(slot)].fingerprint.0;
        app.press(Action::CSlotHere(slot, fp));
    }
    app.press(Action::CNext(cstep::KEYS));
    app.press(Action::CNext(cstep::CHECK));
    app.press(Action::CBackup);
    faraday_core::testkit::backup_as(&mut app, faraday_core::plan::Preset::Paper);
    assert_eq!(app.screen, Screen::Backup);
    app
}

#[test]
fn cancel_on_places_asks_then_ends_the_flow_on_wallets_with_the_keys_still_loaded() {
    let mut app = create_at_places_from_wallets();
    let keys: Vec<[u8; 4]> = (0..3).map(|k| fp(&app, k)).collect();
    let wallets = app.session.wallets.len();
    assert!(offered(&mut app, Action::CancelFlow), "Cancel at the top");
    app.press(Action::CancelFlow);
    assert_eq!(app.sheet, Some(Sheet::CancelFlow(Going::Create)));
    let t = texts(&mut app);
    for want in [
        "Cancel making this wallet?",
        "The plan and its places are lost.",
        "Cancel the flow",
        "Keep going",
    ] {
        assert!(t.iter().any(|x| x == want), "no {want:?}: {t:?}");
    }
    app.press(Action::CancelFlowNow);
    assert_eq!(app.sheet, None);
    assert_eq!(app.screen, Screen::WalletsStart, "Wallets shown");
    let now: Vec<[u8; 4]> = (0..3).map(|k| fp(&app, k)).collect();
    assert_eq!(now, keys, "the keys stay loaded");
    assert_eq!(app.session.wallets.len(), wallets, "the wallet made stays");
    assert!(!row_on_settings(&mut app, Going::Create), "no row");
}

#[test]
fn keep_going_leaves_the_flow_as_it_was() {
    let mut app = create_at_places_from_wallets();
    let answers = app.backup().unwrap().answers.clone();
    app.press(Action::CancelFlow);
    assert!(app.sheet.is_some());
    assert!(offered(&mut app, Action::Cancel), "Keep going");
    app.press(Action::Cancel);
    assert_eq!(app.sheet, None);
    assert_eq!(app.screen, Screen::Backup);
    let b = app.backup().unwrap();
    assert_eq!(b.q, Some(qstep::BOARD), "still on Places");
    assert_eq!(b.answers, answers, "the board as it was");
    assert!(row_on_settings(&mut app, Going::Create), "the row stays");
}

#[test]
fn a_flow_on_its_first_card_ends_without_the_sheet() {
    let mut app = testkit::started();
    app.press(Action::Nav(Screen::WalletsStart));
    app.press(Action::CreateWallet);
    app.press(Action::CKind(0));
    assert!(row_on_settings(&mut app, Going::Create), "Kind done: a row");
    app.press(Action::Resume(Going::Create));
    assert_eq!(app.screen, Screen::Create);
    app.press(Action::CancelFlow);
    assert_eq!(app.sheet, None, "nothing past the first card: no sheet");
    // Resumed from Settings: the Cancel lands there (F77).
    assert_eq!(app.screen, Screen::Settings);
    assert!(!row_on_settings(&mut app, Going::Create));
    // Make a new wallet opens afresh, past Kind closed on its default.
    app.press(Action::CreateWallet);
    crate::common::ckind(&mut app, 0);
    assert_eq!(app.create().unwrap().open, Some(cstep::KEYS));
}

#[test]
fn start_over_still_begins_make_a_new_wallet_again() {
    let mut app = testkit::started();
    app.press(Action::CreateWallet);
    crate::common::ckind(&mut app, 0);
    app.press(Action::CreateOver);
    assert_eq!(app.screen, Screen::Create, "Start over stays on the flow");
    assert!(offered(&mut app, Action::CancelFlow));
}

#[test]
fn the_gpg_flows_cancel_removes_its_tools_row() {
    let mut app = crate::common::process(Vec::new(), Vec::new(), Vec::new());
    crate::common::start(&mut app, Kind::Gpg);
    app.press(Action::Flow(F::Continue));
    assert!(row_on_settings(&mut app, Going::Flow(0)));
    app.press(Action::Resume(Going::Flow(0)));
    assert!(offered(&mut app, Action::CancelFlow));
    app.press(Action::CancelFlow);
    assert_eq!(app.sheet, Some(Sheet::CancelFlow(Going::Flow(0))));
    assert!(
        texts(&mut app)
            .iter()
            .any(|t| t == "Cancel making this key?")
    );
    app.press(Action::CancelFlowNow);
    assert_eq!(
        app.screen,
        Screen::Settings,
        "Settings, where it was resumed from (F77)"
    );
    assert!(!row_on_settings(&mut app, Going::Flow(0)), "no row");
    let names = sidebar_texts(&mut app);
    assert!(!names.iter().any(|t| t == "GPG key"), "{names:?}");
}

#[test]
fn restores_cancel_ends_it_and_removes_its_row() {
    let mut app = testkit::started();
    testkit::move_to(&mut app.session, testkit::NET).unwrap();
    app.press(Action::Nav(Screen::WalletsStart));
    app.press(Action::RestoreWallet);
    app.press(Action::RNext(rstep::KIND));
    app.press(Action::RSeeds);
    assert!(row_on_settings(&mut app, Going::Restore));
    app.press(Action::Resume(Going::Restore));
    app.press(Action::CancelFlow);
    assert_eq!(app.sheet, Some(Sheet::CancelFlow(Going::Restore)));
    app.press(Action::CancelFlowNow);
    // Resumed from Settings: the Cancel lands there (F77).
    assert_eq!(app.screen, Screen::Settings);
    assert!(!row_on_settings(&mut app, Going::Restore));
}

#[test]
fn back_ups_cancel_ends_it_on_the_wallet_and_removes_its_row() {
    let mut app = with(1, 1);
    app.press(Action::Backup(0));
    faraday_core::testkit::backup_as(&mut app, faraday_core::plan::Preset::Paper);
    assert!(row_on_settings(&mut app, Going::Backup));
    app.press(Action::Resume(Going::Backup));
    app.press(Action::CancelFlow);
    assert_eq!(app.sheet, Some(Sheet::CancelFlow(Going::Backup)));
    app.press(Action::CancelFlowNow);
    assert_ne!(app.screen, Screen::Backup);
    assert_eq!(app.session.wallets.len(), 1, "the wallet stays");
    assert!(!row_on_settings(&mut app, Going::Backup));
}

#[test]
fn a_spends_cancel_ends_it_and_removes_its_row() {
    use faraday_core::spend_backup::SpendBackupAction;
    let mut app = with(1, 1);
    app.press(Action::SpendBackup(SpendBackupAction::SpendFrom(0)));
    assert_eq!(app.screen, Screen::SpendBackup);
    assert!(row_on_settings(&mut app, Going::Spend));
    app.press(Action::Resume(Going::Spend));
    app.press(Action::CancelFlow);
    assert_eq!(app.sheet, None, "no transaction yet: no sheet");
    assert_ne!(app.screen, Screen::SpendBackup);
    assert!(!row_on_settings(&mut app, Going::Spend));
}

#[test]
fn sign_a_messages_cancel_ends_it_and_removes_its_row() {
    use faraday_core::mstep;
    let mut app = with(1, 1);
    app.press(Action::Nav(Screen::WalletsStart));
    app.press(Action::SignMessage);
    app.press(Action::MNext(mstep::ADDRESS));
    app.press(Action::MNext(mstep::TEXT));
    assert!(row_on_settings(&mut app, Going::Message));
    app.press(Action::Resume(Going::Message));
    app.press(Action::CancelFlow);
    assert_eq!(app.sheet, Some(Sheet::CancelFlow(Going::Message)));
    app.press(Action::CancelFlowNow);
    // Resumed from Settings: the Cancel lands there (F77).
    assert_eq!(app.screen, Screen::Settings);
    assert!(!row_on_settings(&mut app, Going::Message));
}

#[test]
fn create_a_vaults_cancel_ends_it_where_it_was_resumed_and_removes_its_row() {
    use faraday_core::vault::{VaultAction as V, vstep};
    let mut app = with(0, 0);
    app.storage(faraday_core::StorageEvent::Memory {
        available_mib: 15_000,
    });
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Create));
    app.press(Action::Vault(V::CCustomise));
    app.press(Action::Vault(V::CNext(vstep::WHERE)));
    app.press(Action::Vault(V::CNext(vstep::COST)));
    assert!(row_on_settings(&mut app, Going::Vault));
    app.press(Action::Resume(Going::Vault));
    app.press(Action::CancelFlow);
    assert_eq!(app.sheet, Some(Sheet::CancelFlow(Going::Vault)));
    app.press(Action::CancelFlowNow);
    // Resumed from Settings: the Cancel lands there (F77).
    assert_eq!(app.screen, Screen::Settings);
    assert!(!row_on_settings(&mut app, Going::Vault));
}

#[test]
fn a_backup_flow_with_its_key_made_offers_no_cancel() {
    use faraday_core::vault::VaultAction as V;
    let mut app = with(0, 0);
    app.storage(faraday_core::StorageEvent::Restored {
        inbox: faraday_core::files_from(vec![(
            faraday_text::Text::of("vault.kdbx"),
            faraday_core::testkit::test_vault().unwrap(),
        )]),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app.storage(faraday_core::StorageEvent::Memory {
        available_mib: 15_000,
    });
    faraday_core::testkit::vault_speed(&mut app, Some(180));
    app.press(Action::Vault(V::Open(0)));
    type_in(&mut app, faraday_core::testkit::VAULT_PASSPHRASE);
    app.press(Action::Vault(V::Unlock));
    for t in 1..60u64 {
        if !app.vaults().open.is_empty() {
            break;
        }
        let _ = app.frame();
        app.event(Event::Tick { now_ms: t * 1000 });
    }
    assert_eq!(app.vaults().open.len(), 1, "the test vault did not unlock");
    app.press(Action::Vault(V::LoadTicked));
    faraday_core::testkit::keep_published(&mut app);
    // The randomness and the clock a shell gives.
    for i in 0..8u8 {
        app.event(Event::Entropy(faraday_shell_api::EntropyBytes::new(
            [0x60 + i; 32],
        )));
    }
    app.storage(faraday_core::StorageEvent::Clock {
        unix_secs: 1_791_000_000,
    });
    crate::common::keep_time(&mut app);
    crate::common::at_make(&mut app, Kind::Gpg);
    let toast = crate::common::make_in_form(&mut app, &["Alice", ""]);
    let i = app.backup_flows().len() - 1;
    assert!(
        app.backup_flows()[i].made,
        "the key was not made: {toast:?}"
    );
    assert_eq!(app.screen, Screen::BackupFlow);
    assert!(!offered(&mut app, Action::CancelFlow), "no Cancel on it");
    app.press(Action::CancelFlow);
    assert_eq!(app.sheet, None);
    assert_eq!(app.screen, Screen::BackupFlow, "the flow stays on screen");
    assert!(app.backup_flows()[i].made, "the flow is kept");
    assert!(row_on_settings(&mut app, Going::Flow(i)), "its row stays");
}

/// With keys and wallets loaded the sidebar has no row for any of them (they
/// are on Home), and a flow in progress still has its row.
#[test]
fn the_sidebar_shows_no_key_or_wallet_rows_and_a_flow_in_progress_keeps_its_row() {
    let mut app = with_height(2, 2, 1200);
    for k in 0..2 {
        assert!(!app.offers(Action::ExploreKey(fp(&app, k))), "key {k}");
    }
    for w in 0..2 {
        assert!(!app.offers(Action::OpenWallet(w)), "wallet {w}");
    }
    assert!(!crate::common::has(&mut app, "Keys"));
    assert!(app.offers(Action::Nav(Screen::Home)), "the tabs stay");
    assert!(app.offers(Action::PowerAsk), "the foot stays");

    let mut app = create_at_places();
    app.press(Action::Nav(Screen::Settings));
    let _ = app.frame();
    for k in 0..3 {
        assert!(!app.offers(Action::ExploreKey(fp(&app, k))), "key {k}");
    }
    assert!(offered(&mut app, Action::Resume(Going::Create)));
}
