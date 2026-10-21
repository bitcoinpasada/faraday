//! The sidebar names what is loaded in two groups, keys and wallets,
//! as far up as the last tab. A group that does not fit shows its first
//! rows and "+ N more", which opens the whole list; each row opens what
//! it names. Under its tab, each flow in progress has a row, its name
//! and open step, which returns to it as it was left.

use faraday_core::{Action, Faraday, Screen, Sheet};
use osk_bip::bip39::{Language, Mnemonic};
use osk_shell_api::{App, BootState, DisplayInfo, Event, SecureHardware};

/// `seeds` seeds and `wallets` single-key wallets over the first of them,
/// on Files.
fn with(seeds: u8, wallets: usize) -> Faraday {
    with_height(seeds, wallets, 768)
}

/// `with`, at a chosen panel height, for a test that wants more room
/// than the panel's own 768 to fit its counts.
fn with_height(seeds: u8, wallets: usize, height: u16) -> Faraday {
    let mut app = Faraday::new();
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
            .add_words(&words.join(" "), &format!("Seed {i}"), None)
            .unwrap();
    }
    let text = (wallets > 0).then(|| {
        faraday_core::create::NewKind::NativeSegwit
            .key_text(app.session.keys[0].master())
            .unwrap()
    });
    for a in 0..wallets {
        let text = text.as_deref().unwrap();
        let text = text.replace("/0h]", &format!("/{a}h]"));
        let d = format!("wpkh({text}/<0;1>/*)");
        // The account number differs only in the origin: enough for a
        // wallet of its own in the list.
        app.session
            .add_wallet(
                &format!("Wallet {a}"),
                &d,
                "test",
                faraday_core::wallet::Origin::Named,
            )
            .unwrap_or_else(|e| panic!("{d}: {}", e.text()));
    }
    app.press(Action::Nav(Screen::Files));
    let _ = app.frame();
    app
}

fn fp(app: &Faraday, k: usize) -> [u8; 4] {
    app.session.keys[k].fingerprint().0
}

#[test]
fn the_sidebar_offers_learn_and_opens_the_learn_sheet() {
    let mut app = with(0, 0);
    assert!(app.offers(Action::Learn));
    app.press(Action::Learn);
    assert_eq!(app.sheet, Some(Sheet::Learn));
}

/// Spend left the sidebar (`docs/SIMPLIFY.md` §6.2): the Wallets empty
/// state and the Learn sheet are its ways in now.
#[test]
fn the_sidebar_has_no_spend_row() {
    let app = with(0, 0);
    assert!(!app.offers(Action::Nav(Screen::Family)));
}

/// Each of the four stages of the session strip in turn.
#[test]
fn the_session_strip_names_the_current_stage() {
    let mut app = with(0, 0);
    assert_eq!(app.session_stage(), "Open");
    app.session
        .add_words("abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about", "k", None)
        .unwrap();
    assert_eq!(app.session_stage(), "Work");
    app.outbox.clear();
    app.session.keys.clear();
    app.outbox.push(faraday_core::Item {
        name: "a.psbt".into(),
        bytes: Vec::new(),
        kind: faraday_core::wallet::FileKind::Psbt,
        secret: false,
        picture: None,
        sha: None,
        wallet: None,
        psbt: None,
    });
    assert_eq!(app.session_stage(), "Write out");
    app.storage(faraday_core::StorageEvent::Sticks(vec![
        faraday_core::StickInfo {
            serial: None,
            id: "a".into(),
            label: "STICK".into(),
            boot: false,
            files: Vec::new(),
        },
    ]));
    assert_eq!(app.session_stage(), "Bring in");
}

/// Write out also when nothing waits For the stick but a vault has
/// changed since it was written (§3.5): here the test vault, copied in,
/// opened, and a key saved into it.
#[test]
fn the_session_strip_reads_write_out_for_a_vault_changed_since_written() {
    use faraday_core::vaults::VaultAction as V;
    let mut app = with(1, 0);
    app.storage(faraday_core::StorageEvent::Restored {
        inbox: vec![(
            "vault.ofv".to_string(),
            faraday_core::testkit::test_vault().unwrap(),
        )],
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app.storage(faraday_core::StorageEvent::Memory {
        available_mib: 15_000,
    });
    app.vaults.ms_per_unit = Some(180);
    app.press(Action::Vault(V::Open(0)));
    for c in faraday_core::testkit::VAULT_PASSPHRASES[0].chars() {
        app.event(Event::Key(osk_shell_api::Key::Char(c)));
    }
    app.press(Action::Vault(V::Unlock));
    for t in 1..60u64 {
        if !app.vaults.open.is_empty() {
            break;
        }
        let _ = app.frame();
        app.event(Event::Tick { now_ms: t * 1000 });
    }
    assert_eq!(app.vaults.open.len(), 1, "the test vault did not unlock");
    assert_eq!(app.session_stage(), "Work", "open and unchanged");
    app.press(Action::Vault(V::AddKind(0)));
    app.press(Action::Vault(V::SaveKey(0)));
    assert!(app.outbox.is_empty());
    assert_eq!(app.session_stage(), "Write out");
}

#[test]
fn a_few_are_each_named_and_open_what_they_name() {
    let mut app = with(3, 2);
    for k in 0..3 {
        assert!(
            app.offers(Action::ExploreKey(fp(&app, k))),
            "seed {k} not listed"
        );
    }
    for w in 0..2 {
        assert!(app.offers(Action::OpenWallet(w)), "wallet {w} not listed");
    }
    assert!(
        !app.offers(Action::Explore),
        "a '+ more' with room to spare"
    );
    let first = fp(&app, 0);
    app.press(Action::ExploreKey(first));
    assert_eq!(app.screen, Screen::Explore);
    assert_eq!(app.explore.as_ref().and_then(|e| e.key), Some(first));
}

#[test]
fn many_show_their_first_rows_and_how_many_more() {
    let app = with(30, 12);
    assert!(app.offers(Action::ExploreKey(fp(&app, 0))));
    assert!(
        !app.offers(Action::ExploreKey(fp(&app, 29))),
        "every seed is listed: the corner would cover the tabs"
    );
    assert!(app.offers(Action::Explore), "no '+ more' for seeds");
    assert!(app.offers(Action::OpenWallet(0)));
    assert!(
        app.offers(Action::Nav(Screen::Wallets)),
        "no '+ more' for wallets"
    );
}

#[test]
fn a_wallet_the_seeds_here_sign_for_stays_in_view_among_many() {
    let mut app = with(2, 0);
    let text_of = |app: &Faraday, k: usize, a: usize| {
        faraday_core::create::NewKind::NativeSegwit
            .key_text(app.session.keys[k].master())
            .unwrap()
            .replace("/0h]", &format!("/{a}h]"))
    };
    // Twelve wallets over the second seed, then one over the first.
    for a in 0..12 {
        let d = format!("wpkh({}/<0;1>/*)", text_of(&app, 1, a));
        app.session
            .add_wallet(
                &format!("Watched {a}"),
                &d,
                "test",
                faraday_core::wallet::Origin::Named,
            )
            .unwrap();
    }
    let d = format!("wpkh({}/<0;1>/*)", text_of(&app, 0, 0));
    app.session
        .add_wallet("Ready", &d, "test", faraday_core::wallet::Origin::Named)
        .unwrap();
    // The second seed leaves: its twelve wallets are watch-only now.
    app.session.keys.remove(1);
    app.press(Action::Nav(Screen::Files));
    let _ = app.frame();
    assert!(
        app.offers(Action::OpenWallet(12)),
        "the wallet that can sign is cut off"
    );
    assert!(app.offers(Action::Nav(Screen::Wallets)), "no '+ more'");
}

/// A key loaded with a passphrase says so on its row: the same words
/// without it are another key, listed apart.
#[test]
fn a_key_with_a_passphrase_says_so() {
    let mut app = with(1, 0);
    let words = app.session.keys[0].words.as_deref().unwrap().to_string();
    app.session
        .add_words_with(&words, "TREZOR", "Seed 0", None)
        .unwrap();
    let _ = app.frame();
    let texts = app.drawn_texts();
    assert!(texts.iter().any(|t| t == "Keys"), "{texts:?}");
    assert_eq!(
        texts.iter().filter(|t| *t == "passphrase").count(),
        1,
        "only the passphrase key is tagged: {texts:?}"
    );
}

// ---- flows in progress (`docs/FLOWS.md`, Flows in progress; F41) ----

mod common;

use faraday_core::backup_flow::{Kind, Step};
use faraday_core::backup_flow_steps::FlowAction as F;
use faraday_core::in_progress::Going;
use faraday_core::{cstep, qstep, rstep, testkit};
use osk_shell_api::Key;

/// Test seed `n` typed in as a key.
fn key_in(app: &mut Faraday, n: usize) {
    app.press(Action::Entry(None));
    type_in(app, &testkit::test_words(testkit::TEST_SEEDS[n].0));
    app.press(Action::EntryAdd);
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
    app.press(Action::CKind(multi_index()));
    for slot in 0..3u8 {
        let fp = app.session.keys[usize::from(slot)].fingerprint().0;
        app.press(Action::CSlotHere(slot, fp));
    }
    app.press(Action::CNext(cstep::KEYS));
    app.press(Action::CNext(cstep::CHECK));
    app.press(Action::CBackup);
    faraday_core::testkit::backup_as(&mut app, faraday_core::plan::Preset::Paper);
    assert_eq!(app.screen, Screen::Backup);
    assert_eq!(app.backup.as_ref().unwrap().q, Some(qstep::BOARD));
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
    let answers = app.backup.as_ref().unwrap().answers.clone();
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
    let b = app.backup.as_ref().unwrap();
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
    app.press(Action::CNext(cstep::KIND));
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
    app.press(Action::Network(testkit::NET));
    app.press(Action::RestoreWallet);
    app.press(Action::RNext(rstep::KIND));
    app.press(Action::RSeeds);
    app.press(Action::RVaults(false));
    app.press(Action::RNext(rstep::WHERE));
    app.press(Action::Nav(Screen::Settings));
    assert!(offered(&mut app, Action::Resume(Going::Restore)));
    let names = sidebar_texts(&mut app);
    let at = names.iter().position(|t| t == "Restore a wallet").unwrap();
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
    assert_eq!(app.screen, Screen::Restore);
    app.press(Action::Seeds(faraday_core::seeds::SeedsAction::Make));
    app.press(Action::RNext(rstep::CHECK));
    assert_eq!(app.restore.as_ref().unwrap().open, Some(rstep::DONE));
    assert!(
        !offered(&mut app, Action::Resume(Going::Restore)),
        "restored: no row"
    );
}

#[test]
fn a_gpg_keys_flow_has_its_row_under_tools() {
    let mut app = common::process(Vec::new(), Vec::new(), Vec::new());
    common::start(&mut app, Kind::Gpg);
    assert_eq!(app.screen, Screen::BackupFlow);
    app.press(Action::Nav(Screen::Settings));
    assert!(
        !offered(&mut app, Action::Resume(Going::Flow(0))),
        "nothing done yet"
    );
    common::start(&mut app, Kind::Gpg);
    app.press(Action::Flow(F::Continue));
    assert_eq!(app.flow().unwrap().step, Step::Sticks);
    app.press(Action::Nav(Screen::Settings));
    let names = sidebar_texts(&mut app);
    let tools = names.iter().position(|t| t == "Tools").unwrap();
    let learn = names.iter().position(|t| t == "Learn").unwrap();
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
    app.press(Action::Nav(Screen::Start));
    assert!(lit(&mut app, "Wallets"));
    assert!(!lit(&mut app, "Make a new wallet"));
}

#[test]
fn a_lock_wipes_what_it_wipes_and_a_kept_flow_keeps_its_row() {
    let mut shell = common::Shell::new();
    let mut app = common::process(Vec::new(), Vec::new(), Vec::new());
    common::start(&mut app, Kind::Gpg);
    app.press(Action::Flow(F::Continue));
    app.press(Action::CreateWallet);
    app.press(Action::CNext(cstep::KIND));
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
    app.drawn_texts()
}

/// [`create_at_places`], Make a new wallet opened from Wallets.
fn create_at_places_from_wallets() -> Faraday {
    let mut app = testkit::started();
    for n in 0..3 {
        key_in(&mut app, n);
    }
    app.press(Action::Nav(Screen::Start));
    app.press(Action::CreateWallet);
    app.press(Action::CKind(multi_index()));
    for slot in 0..3u8 {
        let fp = app.session.keys[usize::from(slot)].fingerprint().0;
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
    assert_eq!(app.screen, Screen::Start, "Wallets shown");
    let now: Vec<[u8; 4]> = (0..3).map(|k| fp(&app, k)).collect();
    assert_eq!(now, keys, "the keys stay loaded");
    assert_eq!(app.session.wallets.len(), wallets, "the wallet made stays");
    assert!(!row_on_settings(&mut app, Going::Create), "no row");
}

#[test]
fn keep_going_leaves_the_flow_as_it_was() {
    let mut app = create_at_places_from_wallets();
    let answers = app.backup.as_ref().unwrap().answers.clone();
    app.press(Action::CancelFlow);
    assert!(app.sheet.is_some());
    assert!(offered(&mut app, Action::Cancel), "Keep going");
    app.press(Action::Cancel);
    assert_eq!(app.sheet, None);
    assert_eq!(app.screen, Screen::Backup);
    let b = app.backup.as_ref().unwrap();
    assert_eq!(b.q, Some(qstep::BOARD), "still on Places");
    assert_eq!(b.answers, answers, "the board as it was");
    assert!(row_on_settings(&mut app, Going::Create), "the row stays");
}

#[test]
fn a_flow_on_its_first_card_ends_without_the_sheet() {
    let mut app = testkit::started();
    app.press(Action::Nav(Screen::Start));
    app.press(Action::CreateWallet);
    app.press(Action::CNext(cstep::KIND));
    assert!(row_on_settings(&mut app, Going::Create), "Kind done: a row");
    app.press(Action::Resume(Going::Create));
    assert_eq!(app.screen, Screen::Create);
    app.press(Action::CancelFlow);
    assert_eq!(app.sheet, None, "nothing past the first card: no sheet");
    assert_eq!(app.screen, Screen::Start);
    assert!(!row_on_settings(&mut app, Going::Create));
    // Make a new wallet opens afresh, on Kind.
    app.press(Action::CreateWallet);
    assert_eq!(app.create.as_ref().unwrap().open, Some(cstep::KIND));
}

#[test]
fn start_over_still_begins_make_a_new_wallet_again() {
    let mut app = testkit::started();
    app.press(Action::CreateWallet);
    app.press(Action::CNext(cstep::KIND));
    app.press(Action::CreateOver);
    assert_eq!(app.screen, Screen::Create, "Start over stays on the flow");
    assert!(offered(&mut app, Action::CancelFlow));
}

#[test]
fn the_gpg_flows_cancel_removes_its_tools_row() {
    let mut app = common::process(Vec::new(), Vec::new(), Vec::new());
    common::start(&mut app, Kind::Gpg);
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
    assert_eq!(app.screen, Screen::Catalog, "Tools shown");
    assert!(!row_on_settings(&mut app, Going::Flow(0)), "no row");
    let names = sidebar_texts(&mut app);
    assert!(!names.iter().any(|t| t == "GPG key"), "{names:?}");
}

#[test]
fn restores_cancel_ends_it_and_removes_its_row() {
    let mut app = testkit::started();
    app.press(Action::Network(testkit::NET));
    app.press(Action::Nav(Screen::Start));
    app.press(Action::RestoreWallet);
    app.press(Action::RNext(rstep::KIND));
    app.press(Action::RSeeds);
    app.press(Action::RVaults(false));
    app.press(Action::RNext(rstep::WHERE));
    assert!(row_on_settings(&mut app, Going::Restore));
    app.press(Action::Resume(Going::Restore));
    app.press(Action::CancelFlow);
    assert_eq!(app.sheet, Some(Sheet::CancelFlow(Going::Restore)));
    app.press(Action::CancelFlowNow);
    assert_eq!(app.screen, Screen::Start);
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
    use faraday_core::family::FamilyAction;
    let mut app = with(1, 1);
    app.press(Action::Family(FamilyAction::SpendFrom(0)));
    assert_eq!(app.screen, Screen::Family);
    assert!(row_on_settings(&mut app, Going::Spend));
    app.press(Action::Resume(Going::Spend));
    app.press(Action::CancelFlow);
    assert_eq!(app.sheet, None, "no transaction yet: no sheet");
    assert_ne!(app.screen, Screen::Family);
    assert!(!row_on_settings(&mut app, Going::Spend));
}

#[test]
fn sign_a_messages_cancel_ends_it_and_removes_its_row() {
    use faraday_core::mstep;
    let mut app = with(1, 1);
    app.press(Action::Nav(Screen::Start));
    app.press(Action::SignMessage);
    app.press(Action::MNext(mstep::ADDRESS));
    app.press(Action::MNext(mstep::TEXT));
    assert!(row_on_settings(&mut app, Going::Message));
    app.press(Action::Resume(Going::Message));
    app.press(Action::CancelFlow);
    assert_eq!(app.sheet, Some(Sheet::CancelFlow(Going::Message)));
    app.press(Action::CancelFlowNow);
    assert_eq!(app.screen, Screen::Start);
    assert!(!row_on_settings(&mut app, Going::Message));
}

#[test]
fn create_a_vaults_cancel_ends_it_on_vaults_and_removes_its_row() {
    use faraday_core::vaults::{VaultAction as V, vstep};
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
    assert_eq!(app.screen, Screen::Vaults);
    assert!(!row_on_settings(&mut app, Going::Vault));
}

#[test]
fn a_backup_flow_with_its_key_made_offers_no_cancel() {
    use faraday_core::vaults::VaultAction as V;
    let mut app = with(0, 0);
    app.storage(faraday_core::StorageEvent::Restored {
        inbox: vec![(
            "vault.ofv".to_string(),
            faraday_core::testkit::test_vault().unwrap(),
        )],
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app.storage(faraday_core::StorageEvent::Memory {
        available_mib: 15_000,
    });
    app.vaults.ms_per_unit = Some(180);
    app.press(Action::Vault(V::Open(0)));
    type_in(&mut app, faraday_core::testkit::VAULT_PASSPHRASES[0]);
    app.press(Action::Vault(V::Unlock));
    for t in 1..60u64 {
        if !app.vaults.open.is_empty() {
            break;
        }
        let _ = app.frame();
        app.event(Event::Tick { now_ms: t * 1000 });
    }
    assert_eq!(app.vaults.open.len(), 1, "the test vault did not unlock");
    app.press(Action::Vault(V::LoadTicked));
    // The randomness and the clock a shell gives.
    for i in 0..8u8 {
        app.event(Event::Entropy(osk_shell_api::EntropyBytes::new(
            [0x60 + i; 32],
        )));
    }
    app.storage(faraday_core::StorageEvent::Clock {
        unix_secs: 1_791_000_000,
    });
    common::keep_time(&mut app);
    common::at_make(&mut app, Kind::Gpg);
    let toast = common::make_in_form(&mut app, &["Alice", ""]);
    let i = app.backup_flows.len() - 1;
    assert!(app.backup_flows[i].made, "the key was not made: {toast:?}");
    assert_eq!(app.screen, Screen::BackupFlow);
    assert!(!offered(&mut app, Action::CancelFlow), "no Cancel on it");
    app.press(Action::CancelFlow);
    assert_eq!(app.sheet, None);
    assert_eq!(app.screen, Screen::BackupFlow, "the flow stays on screen");
    assert!(app.backup_flows[i].made, "the flow is kept");
    assert!(row_on_settings(&mut app, Going::Flow(i)), "its row stays");
}
