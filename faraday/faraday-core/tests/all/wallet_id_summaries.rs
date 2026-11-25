//! A locked vault's summary names its wallets by id, so a wallet that
//! shares the checksum of one the vault holds is not told backed up
//! (`docs/VAULT.md` §11, `docs/WALLETS.md` §4).

use crate::chart_edit::{desktop, line_holding, planned, saved_at_lock, settle};
use crate::common::*;
use faraday_core::backups::Entry;
use faraday_core::glance::{self, Backup};
use faraday_core::plan::{Answers, At, Item, Preset, What};
use faraday_core::testkit::{self, WALLET_ID_FIRST as FIRST, WALLET_ID_TWIN as TWIN};
use faraday_core::vault::{VaultAction as V, VaultSummary};
use faraday_core::wallet::Origin;
use faraday_core::wallet_backup::Tone;
use faraday_core::{Action, Faraday, Screen, Sheet};
use faraday_shell_api::{App, EntropyBytes, Event};
use faraday_text::Text;
use faraday_vault::Record;
use faraday_vault::records::{field, kind};

const NOT_BACKED: &str = "Not backed up";
const NO_PLAN: &str = "Backed up · no plan kept";

/// A (`FIRST`) and B (`TWIN`) loaded: one checksum, two wallets.
fn both_loaded() -> Faraday {
    let c = testkit::compute();
    let mut app = Faraday::new();
    testkit::move_to(&mut app.session, testkit::NET).unwrap();
    for (name, d) in [("First", FIRST), ("Twin", TWIN)] {
        app.session
            .add_wallet_confirmed(&c, name, d, "kit", Origin::Named)
            .expect("the wallet reads");
    }
    app
}

fn id_of(app: &Faraday, w: usize) -> String {
    app.session.wallets[w].id().to_hex().to_string()
}

/// A locked vault V that holds the wallet `w` of `app` (the wallet is
/// taken out after, where asked), with the plan for it kept in V when
/// `plan`; `ids` false is a summary kept without ids.
fn locked_vault_holding(app: &mut Faraday, w: usize, plan: bool, ids: bool) {
    let wallet = &app.session.wallets[w];
    let sum = Text::of(wallet.checksum());
    let id = Text::of(&id_of(app, w));
    let name = Text::of(&app.session.wallets[w].name);
    testkit::change_vault_summaries(app, |all| {
        all.push(VaultSummary {
            file: Text::of("v.kdbx"),
            id: [b'v'; 32],
            copies: vec![[1; 32]],
            name: Text::of("V"),
            wallets: vec![(name, Text::of("2 of 3"))],
            sums: vec![sum.clone()],
            wallet_ids: if ids { vec![id.clone()] } else { Vec::new() },
            planned: if plan { vec![sum] } else { Vec::new() },
            planned_ids: if plan && ids { vec![id] } else { Vec::new() },
            ..Default::default()
        });
    });
}

fn entry<'a>(all: &'a [Entry], id: &str) -> &'a Entry {
    all.iter().find(|e| e.id == id).expect("an entry")
}

fn states(e: &Entry) -> Vec<String> {
    e.lines
        .iter()
        .flatten()
        .map(|l| l.state.to_string())
        .collect()
}

/// B2, sequence 1: A is saved into locked V, B nowhere. B's card, its
/// entry in Backups, Home's count and the chart agree that B has no
/// backup; A's say V holds it.
#[test]
fn a_wallet_of_the_same_checksum_is_not_backed_up_by_the_locked_vault_holding_the_other() {
    for plan in [false, true] {
        let mut app = both_loaded();
        locked_vault_holding(&mut app, 0, plan, true);
        let c = testkit::compute();
        let (a, b) = (id_of(&app, 0), id_of(&app, 1));
        if !plan {
            assert_eq!(app.backup_line(&c, 0).as_str(), NO_PLAN);
        }
        assert_eq!(app.backup_line(&c, 1).as_str(), NOT_BACKED, "plan {plan}");
        let all = app.backups_now(&c);
        assert!(entry(&all, &b).lines.is_none(), "B's entry, plan {plan}");
        assert!(entry(&all, &a).lines.is_some(), "A's entry, plan {plan}");
        let (listed, without) = app.backup_counts_now(&c);
        assert_eq!((listed, without), (2, 1), "Home's tile, plan {plan}");
        let g = glance::of_now(&c, &app, 1).expect("a chart");
        assert!(matches!(g.backup, Backup::None), "B's chart, plan {plan}");
    }
}

/// B2, sequence 2: A in locked V and not loaded, B loaded alone: the
/// same, and A is listed with its Unlock.
#[test]
fn a_wallet_loaded_alone_is_not_backed_up_by_the_locked_vault_holding_its_twin() {
    for plan in [false, true] {
        let mut app = both_loaded();
        locked_vault_holding(&mut app, 0, plan, true);
        let a = id_of(&app, 0);
        let b = id_of(&app, 1);
        app.session.wallets.remove(0);
        let c = testkit::compute();
        assert_eq!(app.backup_line(&c, 0).as_str(), NOT_BACKED, "plan {plan}");
        let all = app.backups_now(&c);
        assert!(entry(&all, &b).lines.is_none(), "B's entry, plan {plan}");
        let listed = entry(&all, &a);
        assert!(listed.unlock.is_some(), "A is listed locked, plan {plan}");
        assert_eq!(app.backup_counts_now(&c), (2, 1), "Home's tile");
        let g = glance::of_now(&c, &app, 0).expect("a chart");
        assert!(matches!(g.backup, Backup::None), "B's chart, plan {plan}");
    }
}

/// A summary kept without ids says nothing for any wallet.
#[test]
fn a_summary_kept_without_ids_names_no_wallet() {
    let mut app = both_loaded();
    locked_vault_holding(&mut app, 0, true, false);
    let c = testkit::compute();
    assert_eq!(app.backup_line(&c, 0).as_str(), NOT_BACKED);
    let g = glance::of_now(&c, &app, 0).expect("a chart");
    assert!(matches!(g.backup, Backup::None));
}

/// B3: A's record (checked copies) is not shown under B's name: A is
/// taken out, locked V holds B.
#[test]
fn a_locked_vault_entry_shows_its_own_wallets_record_and_not_its_twins() {
    let mut app = both_loaded();
    let (a, b) = (id_of(&app, 0), id_of(&app, 1));
    let sum = app.session.wallets[0].checksum().to_string();
    let record = format!("backup\t{sum}\tfirst\t1\nwid\t{a}\n");
    testkit::restore_backups(&mut app, record.as_bytes());
    locked_vault_holding(&mut app, 1, false, true);
    app.session.wallets.remove(1);
    app.session.wallets.remove(0);
    let c = testkit::compute();
    let all = app.backups_now(&c);
    let listed = entry(&all, &b);
    assert!(listed.unlock.is_some(), "B is listed locked");
    assert_eq!(
        states(listed),
        vec!["Not kept".to_string()],
        "A's record shown under B"
    );
}

/// The summary an unlocked vault leaves names its wallet by id: with A in
/// the vault and A and B loaded, A reads "no plan kept" and B not backed
/// up.
#[test]
fn an_unlocked_vault_s_summary_keeps_its_wallets_ids() {
    let c = testkit::compute();
    let a = faraday_bip::policy::WalletPolicy::parse_any(&c, FIRST)
        .unwrap()
        .id()
        .to_hex()
        .to_string();
    let records = vec![
        Record::new(kind::WALLET)
            .with(field::WALLET, FIRST.as_bytes())
            .with(field::WALLET_NAME, b"First"),
    ];
    let file = vault_of("V", "first vault phrase", 9, records);
    let mut app = process(vec![("v.kdbx".into(), file)], Vec::new(), Vec::new());
    keep_time(&mut app);
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(faraday_core::vault::VaultAction::Open(0)));
    unlock(&mut app, "first vault phrase");
    let s = &app.vaults().summaries[0];
    assert_eq!(
        s.wallet_ids
            .iter()
            .map(|t| t.to_string())
            .collect::<Vec<_>>(),
        vec![a]
    );
    let app = both_loaded_in(app);
    assert_eq!(app.backup_line(&testkit::compute(), 0).as_str(), NO_PLAN);
    assert_eq!(app.backup_line(&testkit::compute(), 1).as_str(), NOT_BACKED);
}

fn both_loaded_in(mut app: Faraday) -> Faraday {
    let c = testkit::compute();
    for (name, d) in [("First", FIRST), ("Twin", TWIN)] {
        let _ = app
            .session
            .add_wallet_confirmed(&c, name, d, "kit", Origin::Named);
    }
    assert_eq!(app.session.wallets.len(), 2);
    app
}

const PHRASE: &str = "test phrase";

/// Test key 1, which both wallets name as their first key, typed in.
fn own_seed(app: &mut Faraday) {
    app.press(Action::Entry(None));
    type_text(app, &testkit::test_words(testkit::TEST_SEEDS[0].0));
    app.press(Action::EntryAdd);
}

fn load_wallet(app: &mut Faraday, name: &str, descriptor: &str) -> usize {
    app.session
        .add_wallet_confirmed(&testkit::compute(), name, descriptor, "kit", Origin::Named)
        .expect("the wallet reads")
}

/// `chart_edit::fresh_vault` with randomness of its own (`salt`), so that
/// two vaults made in two runs have two identifiers.
fn new_vault(app: &mut Faraday, salt: u8) {
    for i in 0..8u8 {
        app.event(Event::Entropy(EntropyBytes::new([salt + i; 32])));
    }
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Create));
    for second in [false, true] {
        app.press(Action::Vault(V::CFocus(0, second)));
        type_text(app, PHRASE);
    }
    app.press(Action::Vault(V::CGo));
    settle(app, |a| a.screen == Screen::Unlock);
    type_text(app, PHRASE);
    app.press(Action::Vault(V::Unlock));
    settle(app, |a| !a.vaults().open.is_empty());
    assert_eq!(app.vaults().open.len(), 1, "the vault did not open");
}

/// A vault file made in the app: the wallet's own seed, the wallet and its
/// Paper and vault plan saved into it, then locked. `salt` keeps the
/// identifiers of two such vaults apart.
fn vault_with_plan(descriptor: &str, salt: u8) -> Vec<u8> {
    let mut app = desktop();
    own_seed(&mut app);
    let w = load_wallet(&mut app, "Spending", descriptor);
    new_vault(&mut app, salt);
    planned(&mut app, w, 2);
    app.press(Action::Backup(w));
    app.press(Action::BChecklist);
    app.press(Action::BVaultSave(0));
    let c = testkit::compute();
    assert!(app.backup_item_done_now(&c, Item::Vault(0)), "saved");
    let (outbox, _) = saved_at_lock(&mut app);
    outbox
        .into_iter()
        .find(|(n, _)| n.ends_with(".kdbx"))
        .expect("the lock saved the vault")
        .1
}

/// The vault at place `at` of the list unlocked, until `open` vaults are
/// open or the older-copy dialog stands.
fn open_at(app: &mut Faraday, at: usize, open: usize) {
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Open(at)));
    type_text(app, PHRASE);
    app.press(Action::Vault(V::Unlock));
    settle(app, |a| {
        a.vaults().open.len() == open || a.sheet == Some(Sheet::OlderCopy)
    });
}

/// The lines of field `number` in the plan records of the vault file
/// `file`, read on its own, outside any app.
fn plan_lines(file: &[u8], number: u8) -> Vec<String> {
    let c = testkit::compute();
    let mut store = faraday_keys::vault::OpenVaults::default();
    let (vid, _) = store.unlock(&c, file.to_vec(), PHRASE).unwrap();
    store
        .contents_for_tests(vid)
        .iter()
        .filter(|r| r.kind == kind::PLAN)
        .flat_map(|r| r.fields.iter())
        .filter(|f| f.number == number)
        .flat_map(|f| {
            String::from_utf8_lossy(&f.bytes)
                .lines()
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .collect()
}

/// The plan notes that date a check, in open vault `o`'s plan records.
fn checked_notes(app: &Faraday, o: usize) -> Vec<String> {
    contents(app, o)
        .iter()
        .filter(|r| r.kind == kind::PLAN)
        .flat_map(|r| r.fields.iter())
        .filter(|f| f.number == field::PLAN_NOTE)
        .map(|f| String::from_utf8_lossy(&f.bytes).into_owned())
        .filter(|n| n.starts_with("checked "))
        .collect()
}

/// The wallet descriptors open vault `o` plans for.
fn plans_of(app: &Faraday, o: usize) -> Vec<String> {
    contents(app, o)
        .iter()
        .filter(|r| r.kind == kind::PLAN)
        .flat_map(|r| r.fields.iter())
        .filter(|f| f.number == field::PLAN_WALLET)
        .map(|f| {
            let c = testkit::compute();
            let text = String::from_utf8_lossy(&f.bytes);
            faraday_bip::policy::WalletPolicy::parse_any(&c, &text)
                .unwrap()
                .id()
                .to_hex()
                .to_string()
        })
        .collect()
}

fn mark_of(app: &Faraday, w: usize) -> Option<String> {
    let (node, _) = line_holding(app, w, What::VaultStick(0));
    node.state.map(|(t, _)| t.to_string())
}

/// B1: B's plan is in the open vault W, and V, which holds the same seed
/// and A, is unlocked from its stick. Nothing of B was read: W's record of
/// B takes no date and keeps its "Older copy" mark (set by an older copy
/// of W read from its stick). A's own record in V is dated.
#[test]
fn a_stick_visit_of_the_vault_holding_a_twin_dates_nothing_of_the_other_wallet() {
    let v = vault_with_plan(FIRST, 0x11);
    let w_old = vault_with_plan(TWIN, 0x22);
    let w_new = testkit::kdbx_reseal(&w_old, PHRASE, 9).unwrap();
    // No file dates a check: a date seen later was written by a visit here.
    assert!(
        !plan_lines(&v, field::PLAN_NOTE)
            .iter()
            .any(|n| n.starts_with("checked "))
    );
    let mut app = process(
        vec![
            ("a-w-new.kdbx".into(), w_new),
            ("b-w-old.kdbx".into(), w_old),
            ("c-v.kdbx".into(), v),
        ],
        Vec::new(),
        Vec::new(),
    );
    keep_time(&mut app);
    testkit::vault_read_from_stick(&mut app, "b-w-old.kdbx", "Stick B");
    testkit::vault_read_from_stick(&mut app, "c-v.kdbx", "Stick A");
    own_seed(&mut app);
    load_wallet(&mut app, "First", FIRST);
    load_wallet(&mut app, "Twin", TWIN);
    let (a, b) = (id_of(&app, 0), id_of(&app, 1));
    // W's newer copy is seen, then its older one is read from its stick.
    open_at(&mut app, 0, 1);
    assert_eq!(plans_of(&app, 0), vec![b.clone()], "W plans B");
    assert!(testkit::vault_lock_one(&mut app, 0));
    open_at(&mut app, 1, 1);
    assert_eq!(app.sheet, Some(Sheet::OlderCopy));
    app.press(Action::Vault(V::OlderOpen));
    settle(&mut app, |a| !a.vaults().open.is_empty());
    assert!(testkit::vault_lock_one(&mut app, 0));
    open_at(&mut app, 0, 1);
    assert_eq!(app.sheet, None, "the newer copy opens as it is");
    assert_eq!(mark_of(&app, 1).as_deref(), Some("Older copy"), "set up");
    let of_b = checked_notes(&app, 0);
    // The two copies of W are one row once both were opened, so V is
    // found by its file name.
    let files = app.vault_files();
    let at = files
        .iter()
        .position(|f| &*f.name == "c-v.kdbx")
        .expect("V is listed");
    open_at(&mut app, at, 2);
    let _ = app.frame();
    assert_eq!(app.vaults().open.len(), 2);
    assert_eq!(plans_of(&app, 1), vec![a], "V plans A");
    assert_eq!(checked_notes(&app, 0), of_b, "B's record in W took a date");
    assert_eq!(
        mark_of(&app, 1).as_deref(),
        Some("Older copy"),
        "B's mark was cleared"
    );
    assert!(
        checked_notes(&app, 1)
            .iter()
            .any(|n| n.starts_with("checked place1 stick0 ")),
        "A's own record is dated: {:?}",
        checked_notes(&app, 1)
    );
}

/// A vault file holding the wallet `descriptor` only, unlocked from its
/// stick and locked again.
fn locked_after_a_visit(descriptor: &str) -> Faraday {
    let records = vec![
        Record::new(kind::WALLET)
            .with(field::WALLET, descriptor.as_bytes())
            .with(field::WALLET_NAME, b"Kept"),
    ];
    let file = vault_of("V", PHRASE, 9, records);
    let mut app = process(vec![("v.kdbx".into(), file)], Vec::new(), Vec::new());
    keep_time(&mut app);
    testkit::vault_read_from_stick(&mut app, "v.kdbx", "Stick A");
    open_at(&mut app, 0, 1);
    assert!(testkit::vault_lock_one(&mut app, 0));
    settle(&mut app, |a| a.vaults().open.is_empty());
    app.session.wallets.clear();
    app
}

/// B4: B's backup record keeps its descriptor in a vault. A locked vault
/// that holds A does not read as that place; one that holds B does.
#[test]
fn a_wallet_only_vault_place_is_not_kept_by_a_locked_vault_holding_the_twin() {
    for (held, kept) in [(FIRST, false), (TWIN, true)] {
        let mut app = locked_after_a_visit(held);
        load_wallet(&mut app, "Twin", TWIN);
        let b = id_of(&app, 0);
        let sum = app.session.wallets[0].checksum().to_string();
        let record =
            format!("backup\t{sum}\ttwin\t1\nwid\t{b}\nspot\tvault\tpublic\tDescriptor\t\t\t\n");
        testkit::restore_backups(&mut app, record.as_bytes());
        let all = app.backups_now(&testkit::compute());
        let lines = entry(&all, &b).lines.as_ref().expect("B has its record");
        assert_eq!(lines.len(), 1, "{lines:?}");
        let (state, tone) = (lines[0].state.to_string(), lines[0].tone);
        if kept {
            assert_eq!(tone, Tone::Ok, "{state}");
            assert!(state.contains("v.kdbx"), "{state}");
        } else {
            assert_eq!(state, "Not saved yet");
            assert_ne!(tone, Tone::Ok);
        }
    }
}

/// A process with the vault file `file`, unlocked and locked again, so
/// that it is known by its summary only.
fn knows_locked(files: Vec<(String, Vec<u8>)>) -> Faraday {
    let mut app = process(files, Vec::new(), Vec::new());
    keep_time(&mut app);
    open_at(&mut app, 0, 1);
    assert!(testkit::vault_lock_one(&mut app, 0));
    settle(&mut app, |a| a.vaults().open.is_empty());
    app.session.wallets.clear();
    app
}

/// N0: B's checklist vault item is not done by a locked vault that holds
/// the same seed and A; it is by one that holds B.
#[test]
fn a_checklist_vault_item_is_not_done_by_a_locked_vault_holding_the_twin() {
    for (held, done) in [(FIRST, false), (TWIN, true)] {
        let v = vault_with_plan(held, 0x11);
        let mut app = knows_locked(vec![("v.kdbx".into(), v)]);
        own_seed(&mut app);
        let w = load_wallet(&mut app, "Twin", TWIN);
        // With no vault open nothing keeps the plan: it is set and read
        // without leaving it. Its vault 1 holds the seed and the wallet.
        app.press(Action::Backup(w));
        let shape = app.plan_shape_now(w);
        testkit::set_backup_answers(&mut app, Answers::preset(&shape, Preset::PaperVault));
        app.press(Action::BChecklist);
        assert!(
            backup_held(&app, At::Vault(0)).contains(&What::Wallet),
            "the plan keeps the wallet in vault 1: {:?}",
            backup_map(&app)
        );
        assert!(app.vaults().open.is_empty(), "no vault is open");
        let c = testkit::compute();
        assert_eq!(
            app.backup_item_done_now(&c, Item::Vault(0)),
            done,
            "the locked vault holds {}",
            if done { "B" } else { "A" }
        );
    }
}

/// N2: sealing W, which plans B, records V's seal in B's plan only when V
/// holds B.
#[test]
fn sealing_records_no_seal_of_a_vault_holding_the_twin() {
    for (held, recorded) in [(FIRST, false), (TWIN, true)] {
        let v = vault_with_plan(held, 0x11);
        let w = vault_with_plan(TWIN, 0x22);
        let mut app = knows_locked(vec![("a-v.kdbx".into(), v), ("b-w.kdbx".into(), w)]);
        let of_v = app.vaults().summaries[0].id;
        open_at(&mut app, 1, 1);
        let id = app.vaults().open[0].id;
        assert_eq!(plans_of(&app, 0).len(), 1, "W plans B");
        // W counts a change, so the lock seals it either way.
        testkit::vault_changed(&mut app, id);
        let (outbox, _) = saved_at_lock(&mut app);
        let sealed = outbox
            .iter()
            .find(|(n, _)| n.contains("b-w"))
            .unwrap_or_else(|| panic!("W was sealed: {:?}", names(&outbox)));
        // A `seals` line spells the identifier, which is ASCII, as it is.
        let of_v = String::from_utf8_lossy(&of_v).into_owned();
        let lines: Vec<String> = plan_lines(&sealed.1, field::PLAN_ANSWERS)
            .into_iter()
            .filter(|l| l.starts_with(&format!("seals {of_v} ")))
            .collect();
        assert_eq!(lines.len(), usize::from(recorded), "{lines:?}");
    }
}

fn names(files: &[(String, Vec<u8>)]) -> Vec<&str> {
    files.iter().map(|(n, _)| n.as_str()).collect()
}
