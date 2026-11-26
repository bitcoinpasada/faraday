//! The sidebar lists the tabs and lists neither keys nor wallets (they
//! are on Home). Its Files row names the stage of the session. A tab with flows in progress carries
//! their number, a second badge; the flows are the session pane's rows.

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

/// Spend left the sidebar. The top bar's Spend from backup pill stands
/// beside it on every menu page (`docs/WORKSPACE.md` §4), so what is
/// offered here is that pill alone, right of the sidebar.
#[test]
fn the_sidebar_has_no_spend_row() {
    let app = with(0, 0);
    let spend = Action::Nav(Screen::SpendBackup);
    let offered = app.offered_actions();
    assert!(offered.iter().filter(|&&a| a == spend).count() <= 1);
    // 240 pixels at 160 dpi: the sidebar's width.
    assert!(app.hit_box(spend).is_none_or(|(x, ..)| x >= 240));
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
        !crate::common::has(&mut app, "Power off"),
        "Power off is the top app bar's power button's"
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

/// The numbers drawn in the sidebar on the tab named `label`'s row
/// (its icon's corner), top to bottom.
fn marks(app: &mut Faraday, label: &str) -> Vec<String> {
    let _ = app.frame();
    let edge = i32::from(app.where_offered(Action::Nav(Screen::Home)).unwrap().0) * 2;
    let boxes = app.drawn_text_boxes().plain();
    let (lx, ly, _, lh) = boxes
        .iter()
        .find(|(t, (x, _, _, _))| t == label && *x < edge)
        .unwrap_or_else(|| panic!("no {label} in the sidebar"))
        .1;
    let mut v: Vec<(i32, String)> = boxes
        .iter()
        .filter(|(t, (x, y, _, _))| {
            t.chars().all(|c| c.is_ascii_digit()) && *x < lx && (ly - lh..ly + lh * 2).contains(y)
        })
        .map(|(t, (_, y, _, _))| (*y, t.clone()))
        .collect();
    v.sort();
    v.into_iter().map(|(_, t)| t).collect()
}

/// A flow in progress is the number on its tab, not a row under it: the
/// number counts the flows there, none for none, and no row names the flow
/// or its step in the sidebar.
#[test]
fn a_flow_in_progress_is_a_number_on_its_tab_and_no_row() {
    let mut app = testkit::started();
    app.press(Action::Nav(Screen::Settings));
    assert!(marks(&mut app, "Wallets").is_empty(), "no flow, no number");
    app.press(Action::CreateWallet);
    crate::common::ckind(&mut app, 0);
    app.press(Action::Nav(Screen::Settings));
    assert_eq!(marks(&mut app, "Wallets"), ["1"]);
    app.press(Action::RestoreWallet);
    app.press(Action::RNext(rstep::KIND));
    app.press(Action::Nav(Screen::Settings));
    assert_eq!(marks(&mut app, "Wallets"), ["2"]);
    let names = sidebar_texts(&mut app);
    assert!(
        !names
            .iter()
            .any(|t| t == "Make a new wallet" || t == "Restore a wallet"),
        "no row under the tab: {names:?}"
    );
    assert!(marks(&mut app, "Tools").is_empty());
    // The press on the tab is the tab's, as for every tab.
    app.press(Action::Nav(Screen::WalletsStart));
    assert_eq!(app.screen, Screen::WalletsStart);
}

/// A tab with its own badge keeps it, and the flows' number stands beside
/// it, under it: Vaults with a vault open (its badge) and Create a vault
/// in progress.
#[test]
fn the_flow_number_stands_beside_a_tabs_own_badge() {
    use faraday_core::vault::{VaultAction as V, vstep};
    let mut app = with_a_vault_open();
    app.press(Action::Nav(Screen::Settings));
    assert_eq!(marks(&mut app, "Vaults"), ["1"], "its own badge alone");
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Create));
    app.press(Action::Vault(V::CCustomise));
    app.press(Action::Vault(V::CNext(vstep::WHERE)));
    app.press(Action::Vault(V::CNext(vstep::COST)));
    app.press(Action::Nav(Screen::Settings));
    assert_eq!(
        marks(&mut app, "Vaults"),
        ["1", "1"],
        "its own badge and the flow's, one under the other"
    );
}

/// The test vault unlocked with its ticked items loaded: Vaults' own badge.
fn with_a_vault_open() -> Faraday {
    use faraday_core::vault::VaultAction as V;
    let vault = testkit::test_vault().unwrap();
    let mut app = with(0, 0);
    app.storage(faraday_core::StorageEvent::Restored {
        inbox: faraday_core::files_from(vec![(faraday_text::Text::of("vault.kdbx"), vault)]),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    let _ = app.frame();
    app.storage(faraday_core::StorageEvent::Memory {
        available_mib: 15_000,
    });
    app.press(Action::Vault(V::Open(0)));
    for c in testkit::VAULT_PASSPHRASE.chars() {
        app.event(Event::Key(Key::Char(c)));
    }
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
    app
}

/// On the phone's bar too a tab shows its own badge's number and the
/// number of flows in progress there.
#[test]
fn the_phone_bar_shows_the_flow_number_beside_a_tabs_own_badge() {
    use faraday_core::vault::{VaultAction as V, vstep};
    let mut app = with_a_vault_open();
    phone(&mut app);
    app.press(Action::Nav(Screen::Settings));
    assert_eq!(bar_marks(&mut app, "Vaults"), ["1"], "its own badge alone");
    assert!(bar_marks(&mut app, "Tools").is_empty());
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Create));
    app.press(Action::Vault(V::CCustomise));
    app.press(Action::Vault(V::CNext(vstep::WHERE)));
    app.press(Action::Vault(V::CNext(vstep::COST)));
    app.press(Action::Nav(Screen::Settings));
    assert_eq!(bar_marks(&mut app, "Vaults"), ["1", "1"]);
}

fn phone(app: &mut Faraday) {
    app.event(Event::Display(DisplayInfo {
        width: 480,
        height: 640,
        dpi: 160,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
}

/// The numbers drawn on the icon of the phone bar's destination `label`,
/// left to right.
fn bar_marks(app: &mut Faraday, label: &str) -> Vec<String> {
    app.settle();
    let _ = app.frame();
    let boxes = app.drawn_text_boxes().plain();
    // The bar is at the window's foot: the lowest such text.
    let (lx, ly, lw, _) = boxes
        .iter()
        .filter(|(t, _)| t == label)
        .max_by_key(|(_, b)| b.1)
        .unwrap_or_else(|| panic!("no {label} on the bar"))
        .1;
    // A fifth of the window is the destination's own width.
    let middle = lx + lw / 2;
    let mut v: Vec<(i32, String)> = boxes
        .iter()
        .filter(|(t, (x, y, w, _))| {
            t.chars().all(|c| c.is_ascii_digit())
                && (*x + *w / 2 - middle).abs() < 480 / 10
                && (ly - 48..ly).contains(y)
        })
        .map(|(t, (x, ..))| (*x, t.clone()))
        .collect();
    v.sort();
    v.into_iter().map(|(_, t)| t).collect()
}

/// A GPG keys flow is the number on Tools, which stands lit on its page.
#[test]
fn a_gpg_keys_flow_is_a_number_on_tools() {
    let mut app = crate::common::process(Vec::new(), Vec::new(), Vec::new());
    crate::common::start(&mut app, Kind::Gpg);
    assert_eq!(app.screen, Screen::BackupFlow);
    assert!(marks(&mut app, "Tools").is_empty(), "nothing done yet");
    crate::common::start(&mut app, Kind::Gpg);
    app.press(Action::Flow(F::Continue));
    assert_eq!(app.flow().unwrap().step, Step::Sticks);
    assert_eq!(marks(&mut app, "Tools"), ["1"]);
    assert!(lit(&mut app, "Tools"), "the flow's tab is lit on its page");
    let names = sidebar_texts(&mut app);
    assert!(!names.iter().any(|t| t == "GPG key"), "no row: {names:?}");
    app.press(Action::Nav(Screen::Settings));
    app.press(Action::Resume(Going::Flow(0)));
    assert_eq!(app.screen, Screen::BackupFlow);
    assert_eq!(app.flow().unwrap().step, Step::Sticks);
}

/// Presses tab `s` as the frame draws it: a tab with a flow in progress.
fn press_flow_tab(app: &mut Faraday, s: Screen) {
    assert!(offered(app, Action::Tab(s)), "no flow's tab for {s:?}");
    app.press(Action::Tab(s));
}

#[test]
fn the_tab_of_the_flow_on_screen_opens_its_menu_and_once_more_returns_to_it() {
    let mut app = create_at_places_from_wallets();
    press_flow_tab(&mut app, Screen::WalletsStart);
    assert_eq!(app.screen, Screen::WalletsStart, "the tab's menu");
    assert!(counted(&mut app, Going::Create), "the mark stays");
    press_flow_tab(&mut app, Screen::WalletsStart);
    assert_eq!(app.screen, Screen::Backup, "back to the flow, as left");
}

#[test]
fn a_tab_goes_to_its_flow_and_with_several_to_the_one_last_left() {
    let mut app = create_at_places_from_wallets();
    app.press(Action::Nav(Screen::Home));
    app.press(Action::SignMessage);
    app.press(Action::MNext(faraday_core::mstep::ADDRESS));
    app.press(Action::Nav(Screen::Settings));
    press_flow_tab(&mut app, Screen::WalletsStart);
    assert_eq!(app.screen, Screen::Message, "the one last left");
    // The one begun first, once it is the one last left.
    app.press(Action::Nav(Screen::Settings));
    app.press(Action::Resume(Going::Create));
    // A flow of another tab on show: its own tab's menu is not meant.
    app.press(Action::Nav(Screen::Catalog));
    crate::common::start(&mut app, Kind::Gpg);
    app.press(Action::Flow(F::Continue));
    press_flow_tab(&mut app, Screen::WalletsStart);
    assert_eq!(app.screen, Screen::Backup);
    press_flow_tab(&mut app, Screen::Catalog);
    assert_eq!(app.screen, Screen::BackupFlow);
}

#[test]
fn a_tab_with_no_flow_in_progress_opens_its_page() {
    let mut app = create_at_places_from_wallets();
    for s in [Screen::Files, Screen::Catalog, Screen::Home] {
        assert!(!offered(&mut app, Action::Tab(s)), "{s:?}");
        assert!(offered(&mut app, Action::Nav(s)), "{s:?}");
        app.press(Action::Nav(s));
        assert_eq!(app.screen, s);
    }
}

#[test]
fn no_row_before_a_step_is_done() {
    let mut app = testkit::started();
    app.press(Action::CreateWallet);
    app.press(Action::Nav(Screen::Settings));
    assert!(!counted(&mut app, Going::Create));
    app.press(Action::RestoreWallet);
    app.press(Action::Nav(Screen::Settings));
    assert!(!counted(&mut app, Going::Restore));
    // Its first step done: the row.
    app.press(Action::RestoreWallet);
    app.press(Action::RNext(rstep::KIND));
    app.press(Action::Nav(Screen::Settings));
    assert!(counted(&mut app, Going::Restore));
}

/// A flow in progress is a row of Home's box that resumes it.
#[test]
fn a_flow_in_progress_is_a_row_of_homes_box() {
    let mut app = testkit::started();
    app.press(Action::RestoreWallet);
    app.press(Action::RNext(rstep::KIND));
    app.press(Action::Nav(Screen::Home));
    assert!(offered(&mut app, Action::Resume(Going::Restore)));
}

#[test]
fn the_row_goes_when_the_flow_is_started_over() {
    let mut app = testkit::started();
    app.press(Action::CreateWallet);
    crate::common::ckind(&mut app, 0);
    app.press(Action::Nav(Screen::Settings));
    assert!(counted(&mut app, Going::Create));
    app.press(Action::Resume(Going::Create));
    assert_eq!(app.screen, Screen::Create);
    app.press(Action::CreateOver);
    assert!(!counted(&mut app, Going::Create));
}

#[test]
fn the_row_goes_when_the_flow_is_finished() {
    let mut app = testkit::started();
    testkit::move_to(&mut app.session, testkit::NET).unwrap();
    app.press(Action::RestoreWallet);
    app.press(Action::RNext(rstep::KIND));
    app.press(Action::RSeeds);
    app.press(Action::Nav(Screen::Settings));
    assert!(counted(&mut app, Going::Restore));
    let names = sidebar_texts(&mut app);
    assert!(
        !names.iter().any(|t| t.starts_with("Restore an existing")),
        "no row: {names:?}"
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
    assert!(!counted(&mut app, Going::Restore), "restored: not counted");
}

#[test]
fn the_flows_tab_is_lit_on_its_flows_screen() {
    let mut app = create_at_places();
    assert!(lit(&mut app, "Wallets"), "its tab, no row");
    app.press(Action::Nav(Screen::Settings));
    assert!(!lit(&mut app, "Wallets"));
    assert!(lit(&mut app, "Settings"), "Settings is");
    app.press(Action::Nav(Screen::WalletsStart));
    assert!(lit(&mut app, "Wallets"));
}

#[test]
fn a_lock_wipes_what_it_wipes_and_a_kept_flow_keeps_its_row() {
    let mut shell = crate::common::Shell::new();
    let mut app = crate::common::process(Vec::new(), Vec::new(), Vec::new());
    crate::common::start(&mut app, Kind::Gpg);
    app.press(Action::Flow(F::Continue));
    app.press(Action::CreateWallet);
    crate::common::ckind(&mut app, 0);
    assert!(counted(&mut app, Going::Create));
    let mut app = shell.lock(&mut app);
    assert!(
        !counted(&mut app, Going::Create),
        "Make a new wallet is wiped"
    );
    assert!(
        counted(&mut app, Going::Flow(0)),
        "the GPG key's flow is kept"
    );
}

// ---- Cancel: leaving a flow for good (`docs/FLOWS.md`, Cancel; F51) ----

/// The sidebar tab the flow `g` is counted on.
fn tab_of(g: Going) -> &'static str {
    match g {
        Going::Vault => "Vaults",
        Going::Flow(_) => "Tools",
        _ => "Wallets",
    }
}

/// Whether the flow `g` is counted on its tab in the sidebar now: the tab
/// carries a number (its own badge, if any, stands above the flows').
fn counted(app: &mut Faraday, g: Going) -> bool {
    !marks(app, tab_of(g)).is_empty()
}

/// Whether the flow `g` is counted on its tab, seen from Settings.
fn row_on_settings(app: &mut Faraday, g: Going) -> bool {
    app.press(Action::Nav(Screen::Settings));
    counted(app, g)
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
    // Resumed from Settings: the Cancel lands where it was begun (F77).
    assert_eq!(app.screen, Screen::WalletsStart);
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
    app.press(Action::Nav(Screen::Catalog));
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
        Screen::Catalog,
        "Tools, where it was first begun from (F77)"
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
    // Resumed from Settings: the Cancel lands where it was begun (F77).
    assert_eq!(app.screen, Screen::WalletsStart);
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
    // Resumed from Settings: the Cancel lands where it was begun (F77).
    assert_eq!(app.screen, Screen::WalletsStart);
    assert!(!row_on_settings(&mut app, Going::Message));
}

#[test]
fn create_a_vaults_cancel_ends_it_where_it_was_begun_and_removes_its_row() {
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
    // Resumed from Settings: the Cancel lands where it was begun (F77),
    // Files, where Vaults was pressed with no vault to list.
    assert_eq!(app.screen, Screen::Files);
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
    assert!(!app.offers(Action::PowerAsk), "no Power off in the sidebar");

    let mut app = create_at_places();
    app.press(Action::Nav(Screen::Settings));
    let _ = app.frame();
    for k in 0..3 {
        assert!(!app.offers(Action::ExploreKey(fp(&app, k))), "key {k}");
    }
    assert!(counted(&mut app, Going::Create));
}
