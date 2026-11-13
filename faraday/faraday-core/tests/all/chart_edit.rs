//! Changing the backup on the wallet's chart (`docs/history/NEW-WALLET.md`
//! §9.5–§9.7): a move, another copy, a thing destroyed or a place removed
//! shows the check's three lines before and after and is made only once
//! confirmed; from then the map is the plan, kept in the vault, and it
//! loads back the same on another power-on. Names, dates and marks are
//! kept in the vault alone, never in what the device keeps across a lock.
//! A thing marked lost or exposed says what follows, and a key not held
//! here can be given its holder's name and the date its backup was
//! confirmed.

use crate::common::{shown, type_text};
use faraday_core::glance::sheet::{ChartAction as C, Step, Target};
use faraday_core::glance::{Backup, BackupNode, Glance, Press};
use faraday_core::plan::{At, Edit, Found, Lost, Mark, What};
use faraday_core::testkit::KindText;
use faraday_core::vault::VaultAction as V;
use faraday_core::{Action, BStage, Faraday, Screen, Sheet, StorageCommand, StorageEvent, testkit};
use faraday_shell_api::{App, BootState, DisplayInfo, Event, Key, SecureHardware};

type Files = Vec<(String, Vec<u8>)>;

/// A tall desktop, so a sheet's rows are all on screen.
fn device(inbox: Files, kept: Files) -> Faraday {
    let mut app = Faraday::new();
    faraday_core::testkit::move_to(&mut app.session, faraday_core::testkit::NET).unwrap();
    app.event(Event::Display(DisplayInfo {
        width: 1366,
        height: 2400,
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
        inbox: faraday_core::files_from(inbox),
        outbox: Vec::new(),
        kept,
    });
    app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    app.storage(StorageEvent::Clock {
        unix_secs: 1_791_000_000,
    });
    faraday_core::testkit::vault_speed(&mut app, Some(180));
    let _ = app.frame();
    app
}

pub(crate) fn desktop() -> Faraday {
    device(Vec::new(), Vec::new())
}

fn add_key(app: &mut Faraday, n: usize) {
    app.press(Action::Entry(None));
    type_text(app, &testkit::test_words(testkit::TEST_SEEDS[n].0));
    app.press(Action::EntryAdd);
}

/// Test key 1 typed in and a one-key wallet over it.
fn one_key(app: &mut Faraday) -> usize {
    one_key_pass(app, "")
}

/// [`one_key`], the key loaded with the BIP-39 passphrase `pass` when
/// there is one.
fn one_key_pass(app: &mut Faraday, pass: &str) -> usize {
    one_key_wait(app, pass, None)
}

/// [`one_key_pass`], the wallet a recovery wallet whose second key waits
/// `blocks` blocks when there is a wait.
pub(crate) fn one_key_wait(app: &mut Faraday, pass: &str, blocks: Option<u32>) -> usize {
    let c = testkit::compute();
    app.press(Action::Entry(None));
    type_text(app, &testkit::test_words(testkit::TEST_SEEDS[0].0));
    if !pass.is_empty() {
        app.press(Action::EntryPassphrase);
        type_text(app, pass);
        app.press(Action::EntryPassphraseAgain);
        type_text(app, pass);
    }
    app.press(Action::EntryAdd);
    let text = faraday_core::create::NewKind::NativeSegwit
        .key_text(&c, app.session.keys.master_for_tests(0))
        .unwrap();
    let descriptor = match blocks {
        None => format!("wpkh({}/<0;1>/*)", &*text),
        Some(n) => {
            let p = "m/48'/1'/0'/2'";
            let (a, b) = (testkit::key(0, p), testkit::key(1, p));
            format!("wsh(or_d(pk({a}/<0;1>/*),and_v(v:pkh({b}/<0;1>/*),older({n}))))")
        }
    };
    app.session
        .add_wallet(
            &c,
            "Spending",
            &descriptor,
            "test",
            faraday_core::wallet::Origin::Named,
        )
        .unwrap()
}

/// Test keys 1 and 2 typed in and the 2-of-3 over the three test keys:
/// the third is a cosigner's.
fn two_of_three(app: &mut Faraday) -> usize {
    let c = faraday_core::testkit::compute();
    add_key(app, 0);
    add_key(app, 1);
    app.session
        .add_wallet(
            &c,
            "Savings",
            &testkit::savings(),
            "test",
            faraday_core::wallet::Origin::Named,
        )
        .unwrap()
}

pub(crate) fn settle(app: &mut Faraday, until: impl Fn(&Faraday) -> bool) {
    for t in 1..60u64 {
        if until(app) {
            break;
        }
        let _ = app.frame();
        app.event(Event::Tick { now_ms: t * 1000 });
    }
}

/// A vault made and unlocked, with the passphrase "test phrase".
pub(crate) fn fresh_vault(app: &mut Faraday) {
    for i in 0..8u8 {
        app.event(Event::Entropy(faraday_shell_api::EntropyBytes::new(
            [0x40 + i; 32],
        )));
    }
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Create));
    for second in [false, true] {
        app.press(Action::Vault(V::CFocus(0, second)));
        type_text(app, "test phrase");
    }
    app.press(Action::Vault(V::CGo));
    settle(app, |a| a.screen == Screen::Unlock);
    type_text(app, "test phrase");
    app.press(Action::Vault(V::Unlock));
    settle(app, |a| !a.vaults().open.is_empty());
    assert_eq!(app.vaults().open.len(), 1, "the vault did not open");
}

/// The backup of wallet `w` planned with preset `preset` and its
/// checklist made; then the wallet's card.
pub(crate) fn planned(app: &mut Faraday, w: usize, preset: u8) {
    app.press(Action::Backup(w));
    preset_saved(app, preset);
    app.press(Action::BChecklist);
    app.press(Action::OpenWallet(w));
    let _ = app.frame();
}

pub(crate) fn chart(app: &Faraday, w: usize) -> Glance {
    let c = testkit::compute();
    faraday_core::glance::of_now(&c, app, w).expect("the wallet has a chart")
}

pub(crate) fn nodes(g: &Glance) -> Vec<BackupNode> {
    match &g.backup {
        Backup::Plan { nodes, .. } => nodes.clone(),
        other => panic!("no plan on the chart: {other:?}"),
    }
}

/// Backup node `name` on wallet `w`'s chart: its place in the row.
pub(crate) fn node_of(app: &Faraday, w: usize, name: &str) -> (u8, BackupNode) {
    let all = nodes(&chart(app, w));
    let i = all
        .iter()
        .position(|n| n.name == name)
        .unwrap_or_else(|| panic!("no {name} on the chart: {all:?}"));
    (i as u8, all[i].clone())
}

/// Line `label` of backup node `name`: the node's and line's places.
fn line_of(app: &Faraday, w: usize, name: &str, label: &str) -> (u8, u8) {
    let (n, node) = node_of(app, w, name);
    let j = node
        .holds
        .iter()
        .position(|h| h == label)
        .unwrap_or_else(|| panic!("no {label} in {name}: {node:?}"));
    (n, j as u8)
}

fn names(g: &Glance) -> Vec<String> {
    nodes(g).iter().map(|n| n.name.clone()).collect()
}

fn holds(app: &Faraday, w: usize, name: &str) -> Vec<String> {
    node_of(app, w, name).1.holds
}

pub(crate) fn open(app: &mut Faraday, press: Press, t: Target) {
    app.press(Action::Chart(C::Open(press, t)));
    crate::common::a_moment(app);
    assert_eq!(app.sheet, Some(Sheet::Chart), "{t:?} opened no sheet");
}

/// Asks for `edit` as its row does, and confirms it once the check
/// before and after is on the sheet.
pub(crate) fn edit_and_confirm(app: &mut Faraday, press: Press, edit: Edit) {
    app.press(Action::Chart(C::Edit(press, edit)));
    crate::common::a_moment(app);
    assert_eq!(
        app.chart(),
        Some((press, Target::Confirm)),
        "{edit:?} asked nothing"
    );
    let confirm = Action::Chart(C::Confirm(press));
    assert!(app.offers(confirm), "{edit:?} offers no Confirm");
    app.press(confirm);
    let _ = app.frame();
}

/// What the lock saved: For the stick, and the kept state.
pub(crate) fn saved_at_lock(app: &mut Faraday) -> (Files, Files) {
    crate::common::top_up(app);
    app.press(Action::Lock);
    testkit::settle(app);
    let mut last = (Vec::new(), Vec::new());
    while let Some(c) = app.poll_storage() {
        if let StorageCommand::SaveBoxes { outbox, kept, .. } = c {
            last = (faraday_core::testkit::plain_files(outbox), kept);
        }
    }
    last
}

fn holds_bytes(files: &Files, needle: &str) -> Option<String> {
    let needle = needle.as_bytes();
    files
        .iter()
        .find(|(_, b)| b.windows(needle.len()).any(|w| w == needle))
        .map(|(n, _)| n.clone())
}

/// A single key, a vault open, the Paper and vault plan made with it, its
/// seed and the wallet saved into the vault; then the wallet's card.
pub(crate) fn single_key_with_vault() -> (Faraday, usize) {
    single_key_with_vault_pass("")
}

/// [`single_key_with_vault`], the key loaded with passphrase `pass`.
pub(crate) fn single_key_with_vault_pass(pass: &str) -> (Faraday, usize) {
    vault_wallet(pass, None)
}

/// [`single_key_with_vault`] over a recovery wallet whose second key waits
/// `blocks` blocks.
pub(crate) fn recovery_with_vault(blocks: u32) -> (Faraday, usize) {
    vault_wallet("", Some(blocks))
}

fn vault_wallet(pass: &str, blocks: Option<u32>) -> (Faraday, usize) {
    let mut app = desktop();
    let w = one_key_wait(&mut app, pass, blocks);
    fresh_vault(&mut app);
    planned(&mut app, w, 2);
    app.press(Action::Backup(w));
    app.press(Action::BChecklist);
    app.press(Action::BVaultSave(0));
    app.press(Action::Nav(Screen::VaultContents));
    app.press(Action::Vault(V::AddKind(1)));
    app.press(Action::Vault(V::SaveWallet(w)));
    app.press(Action::OpenWallet(w));
    let _ = app.frame();
    (app, w)
}

#[test]
fn a_move_shows_the_check_before_and_after_and_is_made_only_once_confirmed() {
    let (mut app, w) = single_key_with_vault();
    let press = Press::Loaded(w);
    // Place 2 keeps the seed's words, the sheet and the vault's stick;
    // Place 1 the words and the sheet.
    let (n, j) = line_of(&app, w, "Place 2", "Vault 1 stick");
    open(&mut app, press, Target::Line(n, j));
    let move_to = Action::Chart(C::Open(press, Target::MoveTo(n, j)));
    assert!(app.offers(move_to), "the stick's line offers no Move");
    app.press(move_to);
    let _ = app.frame();
    assert!(shown(&mut app, "Place 1"));
    assert!(shown(&mut app, "A new place"));
    // Asked: the check before and after, nothing changed yet.
    let edit = Edit::Move(At::Place(1), What::VaultStick(0), At::Place(0));
    app.press(Action::Chart(C::Edit(press, edit)));
    let _ = app.frame();
    let ask = app.chart_work().ask.as_ref().expect("the move is asked");
    assert_eq!((ask.before.lost, ask.after.lost), (Lost::Yes, Lost::Yes));
    assert!(shown(
        &mut app,
        "Any one place lost: the rest rebuild the wallet"
    ));
    assert!(holds(&app, w, "Place 2").contains(&"Vault 1 stick".to_string()));
    // Closed: nothing moves.
    app.press(Action::Cancel);
    assert!(holds(&app, w, "Place 2").contains(&"Vault 1 stick".to_string()));
    // Confirmed: the stick is at Place 1, and the map is the plan.
    edit_and_confirm(&mut app, press, edit);
    assert!(holds(&app, w, "Place 1").contains(&"Vault 1 stick".to_string()));
    assert!(!holds(&app, w, "Place 2").contains(&"Vault 1 stick".to_string()));
    assert!(matches!(
        chart(&app, w).backup,
        Backup::Plan { edited: true, .. }
    ));
    let (_, vault) = node_of(&app, w, "Vault 1");
    assert_eq!(vault.stick.as_deref(), Some("Stick at Place 1"));
}

#[test]
fn a_change_that_makes_a_check_line_worse_says_so_before_it_is_made() {
    let (mut app, w) = single_key_with_vault();
    let press = Press::Loaded(w);
    // The words at Place 1 destroyed: the seed is left at Place 2 and in
    // the vault whose stick is there too.
    app.press(Action::Chart(C::Edit(
        press,
        Edit::Drop(At::Place(0), What::Words(0)),
    )));
    crate::common::a_moment(&mut app);
    let ask = app.chart_work().ask.as_ref().expect("asked");
    assert_eq!(ask.before.lost, Lost::Yes);
    assert_eq!(ask.after.lost, Lost::No);
    assert!(shown(&mut app, "Yes → No"), "{:?}", app.drawn_texts());
    assert!(shown(&mut app, "Destroy what goes off the plan by hand"));
}

#[test]
fn a_plan_edited_on_the_chart_loads_back_the_same_from_the_vault() {
    let (mut app, w) = single_key_with_vault();
    let press = Press::Loaded(w);
    // The words at Place 1 move to a new place, and a copy of the vault's
    // stick goes to Place 1.
    edit_and_confirm(
        &mut app,
        press,
        Edit::Move(At::Place(0), What::Words(0), At::Place(2)),
    );
    edit_and_confirm(
        &mut app,
        press,
        Edit::Add(At::Place(0), What::VaultStick(0)),
    );
    let before = chart(&app, w);
    assert_eq!(names(&before), ["Place 1", "Place 2", "Place 3", "Vault 1"]);
    assert!(holds(&app, w, "Place 3").contains(&"Key 1 words".to_string()));
    assert!(holds(&app, w, "Place 1").contains(&"Vault 1 · copy".to_string()));

    // Another power-on: nothing kept, the vault read off the stick,
    // unlocked, its wallet loaded.
    let (outbox, _) = saved_at_lock(&mut app);
    let vault: Files = outbox
        .into_iter()
        .filter(|(n, _)| n.ends_with(".kdbx"))
        .collect();
    assert_eq!(vault.len(), 1, "the vault was not put For the stick");
    let mut app = device(vault, Vec::new());
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Open(0)));
    type_text(&mut app, "test phrase");
    app.press(Action::Vault(V::Unlock));
    settle(&mut app, |a| !a.vaults().open.is_empty());
    app.press(Action::Vault(V::LoadWithKeys(0)));
    assert_eq!(app.session.wallets.len(), 1, "the wallet did not load");
    let after = chart(&app, 0);
    assert_eq!(names(&after), names(&before));
    for (a, b) in nodes(&after).iter().zip(nodes(&before)) {
        assert_eq!(a.holds, b.holds, "{} changed", a.name);
    }
    let (check_a, check_b) = match (&after.backup, &before.backup) {
        (Backup::Plan { check: a, .. }, Backup::Plan { check: b, .. }) => (*a, *b),
        _ => unreachable!(),
    };
    assert_eq!(check_a, check_b);
}

#[test]
fn a_copy_of_the_vault_on_another_stick_is_its_own_line_and_counts_in_the_check() {
    let (mut app, w) = single_key_with_vault();
    let press = Press::Loaded(w);
    // The words destroyed at both places: the seed is only in the vault,
    // whose one stick is at Place 2.
    edit_and_confirm(&mut app, press, Edit::Drop(At::Place(0), What::Words(0)));
    edit_and_confirm(&mut app, press, Edit::Drop(At::Place(1), What::Words(0)));
    let lost = |app: &Faraday| match chart(app, w).backup {
        Backup::Plan { check, .. } => check.lost,
        _ => unreachable!(),
    };
    assert_eq!(lost(&app), Lost::No);
    // The vault copied to a stick at Place 1, from the vault's node.
    let (n, _) = node_of(&app, w, "Vault 1");
    open(&mut app, press, Target::Node(n));
    let copy = Action::Chart(C::Open(press, Target::CopyTo(What::VaultStick(0))));
    assert!(
        app.offers(copy),
        "the vault offers no copy to another stick"
    );
    app.press(copy);
    let _ = app.frame();
    edit_and_confirm(
        &mut app,
        press,
        Edit::Add(At::Place(0), What::VaultStick(0)),
    );
    assert!(holds(&app, w, "Place 1").contains(&"Vault 1 · copy".to_string()));
    assert_eq!(lost(&app), Lost::WithVault);
}

#[test]
fn removing_a_place_asks_where_each_thing_in_it_goes_first() {
    let (mut app, w) = single_key_with_vault();
    let press = Press::Loaded(w);
    let (n, node) = node_of(&app, w, "Place 1");
    open(&mut app, press, Target::Node(n));
    let remove = Action::Chart(C::Open(press, Target::Remove(n)));
    assert!(app.offers(remove));
    app.press(remove);
    let _ = app.frame();
    let go = Action::Chart(C::Edit(press, Edit::RemovePlace(0)));
    assert!(
        !app.offers(go),
        "the place goes before each thing has a place"
    );
    for j in 0..node.whats.len() as u8 {
        app.press(Action::Chart(C::Dest(j, faraday_core::chart_edit::OFF)));
    }
    let _ = app.frame();
    assert!(app.offers(go));
    edit_and_confirm(&mut app, press, Edit::RemovePlace(0));
    assert_eq!(names(&chart(&app, w)), ["Place 1", "Vault 1"]);
}

#[test]
fn changing_the_plan_after_an_edit_opens_the_cards_with_the_edit_kept() {
    let (mut app, w) = single_key_with_vault();
    let press = Press::Loaded(w);
    edit_and_confirm(
        &mut app,
        press,
        Edit::Move(At::Place(0), What::Words(0), At::Place(2)),
    );
    let edited = app.backup().unwrap().answers.map.clone();
    assert!(edited.is_some());
    // The plan's cards show the map as it is: nothing is replaced, so
    // nothing is asked (`docs/BACKUP-FLOW.md` §15.2).
    // The chart's Change the plan goes straight to the plan.
    open(&mut app, press, Target::Wallet);
    let go = Action::Chart(C::Do(press, Step::Plan));
    assert!(app.offers(go), "Change the plan asks nothing first");
    app.press(Action::Cancel);
    // From the checklist too.
    app.press(Action::Backup(w));
    app.press(Action::BPlan);
    let _ = app.frame();
    assert_eq!(app.chart(), None);
    assert!(!shown(&mut app, "Your changes on the chart are replaced"));
    assert_eq!(app.screen, Screen::Backup);
    let b = app.backup().unwrap();
    assert_eq!(b.stage, BStage::Plan);
    assert_eq!(b.answers.map, edited, "the chart's edit is the plan");
}

#[test]
fn a_cosigners_key_takes_its_holders_name_and_a_date_kept_in_the_vault_alone() {
    let mut app = desktop();
    clock_kept(&mut app);
    let w = two_of_three(&mut app);
    planned(&mut app, w, 1);
    let press = Press::Loaded(w);
    // With no vault open: the way to one.
    open(&mut app, press, Target::Key(2));
    assert!(shown(&mut app, "Name its holder"));
    assert!(shown(&mut app, "Open a vault to keep this"));
    assert!(!app.offers(Action::Chart(C::Open(press, Target::Holder(2)))));
    app.press(Action::Cancel);
    fresh_vault(&mut app);
    app.press(Action::OpenWallet(w));
    let _ = app.frame();
    open(&mut app, press, Target::Key(2));
    let name = Action::Chart(C::Open(press, Target::Holder(2)));
    assert!(app.offers(name));
    app.press(name);
    type_text(&mut app, "Alice's Coldcard");
    app.press(Action::Chart(C::Save(press)));
    app.press(Action::Chart(C::KeyChecked(press, 2)));
    let key = &chart(&app, w).keys[2];
    assert_eq!(key.holder.as_deref(), Some("Alice's Coldcard"));
    assert_eq!(key.checked.as_deref(), Some("2026-10-03"));
    let texts = app.drawn_texts();
    assert!(texts.iter().any(|t| t == "Alice's Coldcard"), "{texts:?}");
    // Nothing of it is kept across a lock outside the vault.
    let (outbox, kept) = saved_at_lock(&mut app);
    assert_eq!(holds_bytes(&kept, "Alice"), None);
    assert_eq!(holds_bytes(&outbox, "Alice"), None);
}

#[test]
fn a_place_marked_exposed_with_the_words_in_it_leads_to_moving_the_money() {
    let (mut app, w) = single_key_with_vault();
    let press = Press::Loaded(w);
    let (n, _) = node_of(&app, w, "Place 1");
    open(&mut app, press, Target::Node(n));
    let mark = Action::Chart(C::Mark(press, n, None, Mark::Exposed));
    assert!(app.offers(mark));
    app.press(mark);
    let _ = app.frame();
    assert_eq!(app.chart(), Some((press, Target::Marked(n, None))));
    assert!(shown(&mut app, "Someone can now spend"));
    // Marked, the place's lines are struck through and stay on the chart.
    let (_, node) = node_of(&app, w, "Place 1");
    assert!(node.marks.iter().all(|m| *m == Some(Mark::Exposed)));
    let go = Action::Chart(C::MoveMoney(press));
    assert!(app.offers(go));
    app.press(go);
    assert_eq!(app.screen, Screen::Create);
    assert!(app.chart_work().moving.is_some());
    // The marked lines still on the card, marked.
    app.press(Action::OpenWallet(w));
    let texts = app.drawn_texts();
    assert!(texts.iter().any(|t| t.contains("exposed")), "{texts:?}");
}

#[test]
fn move_money_opens_kind_with_no_kind_ticked_and_next_waits_for_one() {
    use faraday_core::create::NewKind;
    use faraday_core::cstep;
    let (mut app, w) = single_key_with_vault();
    let press = Press::Loaded(w);
    let (n, _) = node_of(&app, w, "Place 1");
    open(&mut app, press, Target::Node(n));
    app.press(Action::Chart(C::Mark(press, n, None, Mark::Exposed)));
    app.press(Action::Chart(C::MoveMoney(press)));
    assert_eq!(app.screen, Screen::Create);
    let c = app.create().unwrap();
    assert!(c.kind_unchosen, "no kind ticked");
    assert_eq!(c.open, Some(cstep::KIND));
    let texts = app.drawn_texts();
    assert!(
        texts.iter().position(|t| t == "Single key · native SegWit")
            < texts.iter().position(|t| t == "Multisig · native SegWit"),
        "the rows in the same order: {texts:?}"
    );
    app.press(Action::CNext(cstep::KIND));
    assert_eq!(
        app.create().unwrap().open,
        Some(cstep::KIND),
        "Next waits for a kind"
    );
    let multi = NewKind::ALL
        .iter()
        .position(|k| *k == NewKind::Multi)
        .unwrap() as u8;
    app.press(Action::CKind(multi));
    let c = app.create().unwrap();
    assert_eq!((c.kind, c.kind_unchosen), (NewKind::Multi, false));
    assert_eq!(c.open, Some(cstep::QUORUM), "Multisig's Quorum");
    // Make a new wallet on its own opens with none ticked too.
    app.press(Action::CreateOver);
    assert!(app.create().unwrap().kind_unchosen);
}

#[test]
fn create_from_home_ticks_no_kind_until_a_row_is_pressed() {
    use faraday_core::create::NewKind;
    use faraday_core::cstep;
    let mut app = desktop();
    app.press(Action::CreateWallet);
    let c = app.create().unwrap();
    assert!(c.kind_unchosen, "no kind ticked");
    assert_eq!(c.open, Some(cstep::KIND));
    let texts = app.drawn_texts();
    let at = |s: &str| texts.iter().position(|t| t == s);
    assert!(
        at("Single key · native SegWit") < at("Multisig · native SegWit"),
        "the rows in order: {texts:?}"
    );
    assert!(
        at("Single key, backup in shares (codex32)").is_none(),
        "no shares kind: {texts:?}"
    );
    app.press(Action::CKind(0));
    let c = app.create().unwrap();
    assert_eq!((c.kind, c.kind_unchosen), (NewKind::NativeSegwit, false));
    assert_eq!(c.open, Some(cstep::BACKUP_FORM));
}

#[test]
fn a_copy_marked_lost_says_whether_anything_is_lost_that_is_not_elsewhere() {
    let (mut app, w) = single_key_with_vault();
    let press = Press::Loaded(w);
    // The words at Place 1 lost: they are at Place 2 too.
    let (n, j) = line_of(&app, w, "Place 1", "Key 1 words");
    app.press(Action::Chart(C::Mark(press, n, Some(j), Mark::Lost)));
    crate::common::a_moment(&mut app);
    assert!(shown(&mut app, "Nothing lost that is not elsewhere"));
    assert!(app.offers(Action::Chart(C::Open(
        press,
        Target::CopyTo(What::Words(0))
    ))));
    // Place 2 lost as well: the seed is left only in the vault, whose
    // stick was there.
    let (n2, _) = node_of(&app, w, "Place 2");
    app.press(Action::Chart(C::Mark(press, n2, None, Mark::Lost)));
    let _ = app.frame();
    // The line may wrap on the sheet beside the chart.
    let text = app.drawn_texts().join(" ");
    assert!(
        text.contains("The wallet can no longer be rebuilt from what is left"),
        "{text}"
    );
    assert!(shown(&mut app, "Back up again now"));
    // A lost thing stays on the chart until it is taken off the plan.
    let (_, j) = line_of(&app, w, "Place 1", "Key 1 words");
    open(&mut app, press, Target::Line(n, j));
    assert!(shown(&mut app, "Remove from the plan"));
}

#[test]
fn a_seed_saved_into_another_vault_is_a_change_to_the_plan() {
    let mut app = desktop();
    let w = two_of_three(&mut app);
    fresh_vault(&mut app);
    planned(&mut app, w, 2);
    let press = Press::Loaded(w);
    let (n, node) = node_of(&app, w, "Vault 1");
    assert!(!node.holds.contains(&"Key 2 seed".to_string()));
    open(&mut app, press, Target::Node(n));
    assert!(shown(&mut app, "Save more into it"));
    let save = Edit::Add(At::Vault(0), What::Seed(1));
    assert!(app.offers(Action::Chart(C::Edit(press, save))));
    edit_and_confirm(&mut app, press, save);
    // Then the vault's item, which saves it.
    assert_eq!(app.screen, Screen::Backup);
    assert!(holds(&app, w, "Vault 1").contains(&"Key 2 seed".to_string()));
}

#[test]
fn a_vault_is_renamed_on_the_chart_and_the_name_is_kept_in_the_vault() {
    let (mut app, w) = single_key_with_vault();
    let press = Press::Loaded(w);
    let (n, _) = node_of(&app, w, "Vault 1");
    open(&mut app, press, Target::VaultName(n));
    type_text(&mut app, "Cold storage");
    app.event(Event::Key(Key::Enter));
    assert_eq!(app.sheet, None);
    let all = names(&chart(&app, w));
    assert!(all.contains(&"Cold storage".to_string()), "{all:?}");
    assert!(holds(&app, w, "Place 2").contains(&"Cold storage stick".to_string()));
    let (_, kept) = saved_at_lock(&mut app);
    assert_eq!(holds_bytes(&kept, "Cold storage"), None);
}

#[test]
fn a_place_marked_checked_dates_each_thing_in_it() {
    let (mut app, w) = single_key_with_vault();
    clock_kept(&mut app);
    let press = Press::Loaded(w);
    let (n, _) = node_of(&app, w, "Place 1");
    app.press(Action::Chart(C::Checked(press, n)));
    let (_, node) = node_of(&app, w, "Place 1");
    assert!(
        node.checked
            .iter()
            .all(|d| d.as_deref() == Some("2026-10-03")),
        "{node:?}"
    );
    open(&mut app, press, Target::Node(n));
    assert!(shown(&mut app, "2026-10-03 · today"));
}

#[test]
fn the_wallet_node_checks_its_addresses_and_a_cosigners_key_shows_its_xpub() {
    let mut app = desktop();
    let w = two_of_three(&mut app);
    planned(&mut app, w, 1);
    let press = Press::Loaded(w);
    open(&mut app, press, Target::Wallet);
    let check = Action::Chart(C::Do(press, Step::Addresses));
    assert!(app.offers(check));
    app.press(check);
    let _ = app.frame();
    assert_eq!(app.chart(), Some((press, Target::Addresses)));
    assert!(shown(&mut app, "Receive 0/0"));
    app.press(Action::Cancel);
    open(&mut app, press, Target::Key(2));
    let xpub = Action::Chart(C::Do(press, Step::CosignerQr(2)));
    assert!(app.offers(xpub));
    app.press(xpub);
    assert_eq!(app.sheet, Some(Sheet::Qr));
}

#[test]
fn the_envelope_list_goes_for_the_stick_as_a_pdf() {
    let mut app = desktop();
    let w = one_key(&mut app);
    planned(&mut app, w, 1);
    let press = Press::Loaded(w);
    let (n, _) = node_of(&app, w, "Place 1");
    open(&mut app, press, Target::Node(n));
    let pdf = Action::Chart(C::Do(press, Step::Envelopes));
    assert!(app.offers(pdf));
    app.press(pdf);
    assert!(
        app.outbox
            .iter()
            .any(|i| i.name.ends_with("-envelopes.pdf")),
        "{:?}",
        app.outbox.iter().map(|i| &i.name).collect::<Vec<_>>()
    );
}

#[test]
fn a_plan_saved_before_the_chart_could_edit_it_still_loads() {
    let c = faraday_core::testkit::compute();
    use faraday_core::plan::{Answers, Seed, Shape};
    let shape = Shape {
        m: 1,
        keys: 1,
        seeds: vec![Seed {
            name: faraday_text::Text::of("9A6A2580"),
            here: true,
            passphrase: false,
        }],
        splits: false,
        vaults: 0,
    };
    let text = "seeds 1010\nplaces 2\nsplit 0\nwallet 1100\nsoftware 00001\nform 10\nomit 0\nvault 0 1\nsticks 0 01\n";
    let a = Answers::from_text(&shape, text).expect("the plan loads");
    assert!(a.map.is_none());
    assert_eq!(
        faraday_core::plan::check(&c, &shape, &a).spend,
        Found::Yes,
        "a place with the words can spend"
    );
}

#[test]
fn in_an_open_vaults_view_a_change_is_kept_in_that_vault() {
    let c = testkit::compute();
    let (mut app, w) = single_key_with_vault();
    clock_kept(&mut app);
    app.press(Action::Nav(Screen::VaultContents));
    app.press(Action::Vault(V::Category(1)));
    app.press(Action::Vault(V::Item(0)));
    let _ = app.frame();
    let r = app.vault_selected_index().expect("a wallet is chosen");
    let press = Press::Vault(0, r);
    let vault_chart = |app: &Faraday| faraday_core::glance::of_vault_now(&c, app, 0, r).unwrap();
    let at = nodes(&vault_chart(&app))
        .iter()
        .position(|n| n.name == "Place 1")
        .unwrap() as u8;
    app.press(Action::Chart(C::Checked(press, at)));
    edit_and_confirm(
        &mut app,
        press,
        Edit::Move(At::Place(0), What::Words(0), At::Place(2)),
    );
    let g = vault_chart(&app);
    assert_eq!(names(&g), ["Place 1", "Place 2", "Place 3", "Vault 1"]);
    let all = nodes(&g);
    assert!(all[2].holds.contains(&"Key 1 words".to_string()), "{all:?}");
    // The date moved with the words; the sheet still there.
    assert_eq!(all[2].checked, [Some("2026-10-03".to_string())]);
    assert_eq!(names(&chart(&app, w)), names(&g));
}

/// Back up's plan as preset `k` (by `Preset::ALL`) fills it over a place
/// per key, two for a single key: as a plan saved before plans kept at
/// most one place per key has it (`docs/BACKUP-FLOW.md` §17.19).
fn preset_saved(app: &mut Faraday, k: u8) {
    let w = app.backup().unwrap().wallet;
    let shape = app.plan_shape_now(w);
    let base = if shape.multi() { shape.seeds.len() } else { 2 };
    let preset = faraday_core::plan::Preset::ALL[usize::from(k)];
    faraday_core::testkit::set_backup_answers(
        app,
        faraday_core::plan::Answers::preset_at(&shape, preset, base),
    );
}

// ---------------------------------------------------------------------
// Every check of a copy is dated in the vault (`docs/BACKUP-FLOW.md` §18.3)
// ---------------------------------------------------------------------

/// Set the time kept: the clock the desktop reads is then a checked one.
pub(crate) fn clock_kept(app: &mut Faraday) {
    app.press(Action::OpenSetTime);
    app.press(Action::SetTimeKeep);
}

/// The checklist's Check my copy opened on the copy of the seed's words,
/// which are typed back.
fn type_back(app: &mut Faraday, w: usize) {
    type_back_in(app, w, false);
}

/// The checklist's Check my copy on the words, typed back in the backup
/// already under way (the person comes back to it from wherever they were).
fn type_back_here(app: &mut Faraday) {
    app.press(Action::Nav(Screen::Backup));
    if app.backup().and_then(|b| b.open) == Some(faraday_core::bstep::BLANK) {
        app.press(Action::BNext(faraday_core::bstep::BLANK));
    }
    if app.backup().and_then(|b| b.open) != Some(faraday_core::bstep::COPY) {
        app.press(Action::BStep(faraday_core::bstep::COPY));
    }
    app.press(Action::BCheck);
    testkit::type_words(app, &testkit::test_words(testkit::TEST_SEEDS[0].0));
}

/// [`type_back`], under the plan of [`plan_as_kept`] when `kept`.
fn type_back_in(app: &mut Faraday, w: usize, kept: bool) {
    app.press(Action::Backup(w));
    app.press(Action::BChecklist);
    if kept {
        plan_as_kept(app, w);
    }
    if app.backup().and_then(|b| b.open) == Some(faraday_core::bstep::BLANK) {
        app.press(Action::BNext(faraday_core::bstep::BLANK));
    }
    if app.backup().and_then(|b| b.open) != Some(faraday_core::bstep::COPY) {
        app.press(Action::BStep(faraday_core::bstep::COPY));
    }
    app.press(Action::BCheck);
    testkit::type_words(app, &testkit::test_words(testkit::TEST_SEEDS[0].0));
}

/// The backup under way takes the plan the vault of [`single_key_with_vault`]
/// keeps: the words at two places and a stick for the vault.
fn plan_as_kept(app: &mut Faraday, w: usize) {
    let shape = app.plan_shape_now(w);
    let text = "seeds 1010\nplaces 2\nsplit 0\nwallet 1100\nsoftware 00001\nform 10\nomit 0\nvault 0 1\nsticks 0 01\n";
    faraday_core::testkit::set_backup_answers(
        app,
        faraday_core::plan::Answers::from_text(&shape, text).expect("the plan loads"),
    );
}

/// When the words line of `place` was last checked, as the chart shows.
fn words_checked(app: &Faraday, w: usize, place: &str) -> Option<String> {
    let (_, node) = node_of(app, w, place);
    let j = node
        .holds
        .iter()
        .position(|h| h == "Key 1 words")
        .unwrap_or_else(|| panic!("no words in {place}: {node:?}"));
    node.checked[j].clone()
}

#[test]
fn a_copy_matched_in_the_checklist_is_dated_in_the_vault_and_kept_across_a_power_off() {
    let (mut app, w) = single_key_with_vault();
    clock_kept(&mut app);
    type_back(&mut app, w);
    app.press(Action::BWhich(0));
    app.press(Action::OpenWallet(w));
    let _ = app.frame();
    assert_eq!(
        words_checked(&app, w, "Place 1").as_deref(),
        Some("2026-10-03")
    );
    // Another power-on: the vault read off the stick and unlocked.
    let (outbox, _) = saved_at_lock(&mut app);
    let vault: Files = outbox
        .into_iter()
        .filter(|(n, _)| n.ends_with(".kdbx"))
        .collect();
    let mut app = device(vault, Vec::new());
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Open(0)));
    type_text(&mut app, "test phrase");
    app.press(Action::Vault(V::Unlock));
    settle(&mut app, |a| !a.vaults().open.is_empty());
    app.press(Action::Vault(V::LoadWithKeys(0)));
    assert_eq!(
        words_checked(&app, 0, "Place 1").as_deref(),
        Some("2026-10-03")
    );
    assert_eq!(words_checked(&app, 0, "Place 2"), None);
}

#[test]
fn a_seed_with_its_words_at_two_places_asks_which_copy_and_dates_only_that_one() {
    let (mut app, w) = single_key_with_vault();
    clock_kept(&mut app);
    type_back(&mut app, w);
    assert!(shown(&mut app, "Which copy was this?"));
    assert!(app.offers(Action::BWhich(1)));
    app.press(Action::BWhich(1));
    assert!(app.backup().unwrap().which.is_none());
    app.press(Action::OpenWallet(w));
    let _ = app.frame();
    assert_eq!(words_checked(&app, w, "Place 1"), None);
    assert_eq!(
        words_checked(&app, w, "Place 2").as_deref(),
        Some("2026-10-03")
    );
}

#[test]
fn the_charts_type_my_copy_dates_the_place_of_its_line_without_asking() {
    let (mut app, w) = single_key_with_vault();
    clock_kept(&mut app);
    let press = Press::Loaded(w);
    let fp = app.session.keys[0].fingerprint.0;
    let (n, j) = line_of(&app, w, "Place 2", "Key 1 words");
    open(&mut app, press, Target::Line(n, j));
    let step = Step::Type(fp, faraday_core::bstep::COPY, Some(At::Place(1)));
    assert!(app.offers(Action::Chart(C::Do(press, step))));
    app.press(Action::Chart(C::Do(press, step)));
    testkit::type_words(&mut app, &testkit::test_words(testkit::TEST_SEEDS[0].0));
    assert!(!shown(&mut app, "Which copy was this?"));
    app.press(Action::OpenWallet(w));
    let _ = app.frame();
    assert_eq!(words_checked(&app, w, "Place 1"), None);
    assert_eq!(
        words_checked(&app, w, "Place 2").as_deref(),
        Some("2026-10-03")
    );
}

#[test]
fn a_match_with_the_clock_not_checked_has_no_date_until_set_the_time() {
    let (mut app, w) = single_key_with_vault();
    type_back(&mut app, w);
    app.press(Action::BWhich(0));
    assert!(shown(&mut app, "Set the time"));
    assert!(app.offers(Action::OpenSetTime));
    app.press(Action::OpenWallet(w));
    let _ = app.frame();
    assert_eq!(words_checked(&app, w, "Place 1").as_deref(), Some(""));
    let (n, j) = line_of(&app, w, "Place 1", "Key 1 words");
    open(&mut app, Press::Loaded(w), Target::Line(n, j));
    assert!(
        shown(&mut app, "Checked · no date"),
        "{:?}",
        app.drawn_texts()
    );
    app.press(Action::OpenWallet(w));
    // Set the time dates it with that day.
    clock_kept(&mut app);
    app.press(Action::OpenWallet(w));
    let _ = app.frame();
    assert_eq!(
        words_checked(&app, w, "Place 1").as_deref(),
        Some("2026-10-03")
    );
}

#[test]
fn a_match_with_no_vault_open_is_held_in_memory_and_never_kept_outside_a_vault() {
    let mut app = desktop();
    clock_kept(&mut app);
    let w = one_key(&mut app);
    planned(&mut app, w, 2);
    type_back(&mut app, w);
    app.press(Action::BWhich(0));
    let (outbox, kept) = saved_at_lock(&mut app);
    assert_eq!(holds_bytes(&kept, "2026-10-03"), None);
    assert_eq!(holds_bytes(&outbox, "2026-10-03"), None);
}

#[test]
fn a_dated_check_held_in_memory_is_written_into_the_notes_at_the_next_unlock() {
    let (mut app, _w) = single_key_with_vault();
    clock_kept(&mut app);
    let (outbox, _) = saved_at_lock(&mut app);
    let vault: Files = outbox
        .into_iter()
        .filter(|(n, _)| n.ends_with(".kdbx"))
        .collect();
    // Another power-on: the vault is open when the backup starts, then it
    // is locked, and the copy typed back matches and is held.
    let mut app = unlocked_again(vault, &[], 0);
    app.press(Action::Vault(V::LoadWithKeys(0)));
    app.press(Action::Backup(0));
    app.press(Action::BChecklist);
    testkit::vault_lock_one(&mut app, 0);
    type_back_here(&mut app);
    app.press(Action::BWhich(0));
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Open(0)));
    type_text(&mut app, "test phrase");
    app.press(Action::Vault(V::Unlock));
    settle(&mut app, |a| !a.vaults().open.is_empty());
    app.press(Action::Vault(V::LoadWithKeys(0)));
    let held = (0..app.session.wallets.len()).find_map(|i| words_checked(&app, i, "Place 1"));
    assert_eq!(held.as_deref(), Some("2026-10-03"));
}

// ---------------------------------------------------------------------
// A vault's stick and a codex32 string are dated by what was read
// (`docs/BACKUP-FLOW.md` §18.3)
// ---------------------------------------------------------------------

/// The line of the chart holding `what`, as the node's and line's places:
/// the node and what it shows for that line.
pub(crate) fn line_holding(app: &Faraday, w: usize, what: What) -> (BackupNode, usize) {
    for node in nodes(&chart(app, w)) {
        if let Some(j) = node.whats.iter().position(|x| *x == what) {
            return (node, j);
        }
    }
    panic!("nothing holds {what:?}");
}

/// The vault file the lock saved.
pub(crate) fn vault_file(outbox: Files) -> (String, Vec<u8>) {
    outbox
        .into_iter()
        .find(|(n, _)| n.ends_with(".kdbx"))
        .expect("the lock saved the vault")
}

/// Another power-on with the vault files `files` in the Inbox and the clock
/// kept, the wallet's key in memory; the vault file at place `at` unlocked
/// and the wallets in it loaded. The wallet is 0.
pub(crate) fn unlocked_again(files: Files, from_stick: &[&str], at: usize) -> Faraday {
    unlocked_again_clock(files, from_stick, at, true)
}

/// [`unlocked_again`], the clock kept or not.
fn unlocked_again_clock(files: Files, from_stick: &[&str], at: usize, clock: bool) -> Faraday {
    unlocked_again_keys(files, Vec::new(), from_stick, at, clock, Keys::Wallet)
}

/// What is in memory before a vault is unlocked.
#[derive(Clone, Copy, PartialEq)]
enum Keys {
    /// No key: the keys come only from the vault's Load with keys.
    None,
    /// The wallet's key and no wallet.
    SeedOnly,
    /// The wallet's key and the wallet.
    Wallet,
}

/// [`unlocked_again_clock`] with the state the last lock kept (`kept`), the
/// wallet's key in memory before the unlock or not (the keys then come
/// only from the vault's Load with keys).
fn unlocked_again_keys(
    files: Files,
    kept: Files,
    from_stick: &[&str],
    at: usize,
    clock: bool,
    key: Keys,
) -> Faraday {
    let mut app = device(files, kept);
    if clock {
        clock_kept(&mut app);
    }
    for name in from_stick {
        testkit::vault_read_from_stick(&mut app, name, "Stick A");
    }
    match key {
        Keys::None => {}
        Keys::SeedOnly => add_key(&mut app, 0),
        Keys::Wallet => {
            let _ = one_key(&mut app);
        }
    }
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Open(at)));
    type_text(&mut app, "test phrase");
    app.press(Action::Vault(V::Unlock));
    settle(&mut app, |a| {
        !a.vaults().open.is_empty() || a.sheet == Some(Sheet::OlderCopy)
    });
    app
}

fn vault_stick_checked(app: &Faraday) -> Option<String> {
    let (node, j) = line_holding(app, 0, What::VaultStick(0));
    node.checked[j].clone()
}

#[test]
fn a_vault_unlocked_from_a_stick_read_dates_that_sticks_place_line() {
    let (mut app, _) = single_key_with_vault();
    clock_kept(&mut app);
    let (outbox, _) = saved_at_lock(&mut app);
    let (name, bytes) = vault_file(outbox);
    let mut app = unlocked_again(vec![(name.clone(), bytes)], &[&name], 0);
    app.press(Action::Vault(V::LoadWithKeys(0)));
    app.press(Action::OpenWallet(0));
    let _ = app.frame();
    assert_eq!(vault_stick_checked(&app).as_deref(), Some("2026-10-03"));
}

#[test]
fn a_vault_unlocked_not_from_a_stick_dates_nothing() {
    let (mut app, _) = single_key_with_vault();
    clock_kept(&mut app);
    let (outbox, _) = saved_at_lock(&mut app);
    let (name, bytes) = vault_file(outbox);
    let mut app = unlocked_again(vec![(name, bytes)], &[], 0);
    app.press(Action::Vault(V::LoadWithKeys(0)));
    app.press(Action::OpenWallet(0));
    let _ = app.frame();
    assert_eq!(vault_stick_checked(&app), None);
}

#[test]
fn a_stick_read_that_opens_an_older_copy_reads_older_copy_and_dates_nothing() {
    let (mut app, _) = single_key_with_vault();
    clock_kept(&mut app);
    let (outbox, _) = saved_at_lock(&mut app);
    let (name, old) = vault_file(outbox);
    let new = testkit::kdbx_reseal(&old, "test phrase", 9).unwrap();
    let files = vec![("a-new.kdbx".to_string(), new), (name.clone(), old)];
    // The newer copy is opened (not from a stick) and locked again, so the
    // count is known; the older one then stops at its dialog.
    let mut app = unlocked_again(files, &[&name], 0);
    testkit::vault_lock_one(&mut app, 0);
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Open(1)));
    type_text(&mut app, "test phrase");
    app.press(Action::Vault(V::Unlock));
    settle(&mut app, |a| a.sheet == Some(Sheet::OlderCopy));
    assert_eq!(app.sheet, Some(Sheet::OlderCopy));
    app.press(Action::Vault(V::OlderOpen));
    settle(&mut app, |a| !a.vaults().open.is_empty());
    app.press(Action::Vault(V::LoadWithKeys(0)));
    app.press(Action::OpenWallet(0));
    let _ = app.frame();
    let (node, j) = line_holding(&app, 0, What::VaultStick(0));
    assert!(node.older[j], "{node:?}");
    assert_eq!(node.checked[j], None);
    let (n, _) = node_of(&app, 0, &node.name);
    open(&mut app, Press::Loaded(0), Target::Node(n));
    assert!(shown(&mut app, "Older copy"), "{:?}", app.drawn_texts());
}

#[test]
fn a_codex32_string_checked_in_the_checklist_is_dated_at_its_place() {
    use crate::codex32_backup::{c32, checklist_at_codex32, on_unseeded, with_set_on};
    let mut app = on_unseeded();
    fresh_vault(&mut app);
    let mut app = with_set_on(app);
    app.storage(StorageEvent::Clock {
        unix_secs: 1_791_000_000,
    });
    clock_kept(&mut app);
    checklist_at_codex32(&mut app);
    app.press(Action::BVaultSave(0));
    let a = faraday_core::plan::codex32::index_of(b'a').unwrap();
    c32(&mut app, faraday_core::codex32_backup::Press::Open(a as u8));
    for ch in "2namea320zyxwvutsrqpnmlkjhgfedcaxrpp870hkkqrm".chars() {
        c32(
            &mut app,
            faraday_core::codex32_backup::Press::Key(faraday_ui::keyboard::OskPress::Char(ch)),
        );
    }
    c32(&mut app, faraday_core::codex32_backup::Press::Next);
    app.press(Action::OpenWallet(0));
    let _ = app.frame();
    let (node, j) = line_holding(&app, 0, What::Codex32Share(0, a));
    assert_eq!(node.checked[j].as_deref(), Some("2026-10-03"), "{node:?}");
}

/// Two copies of the test vault's file, sealed once more than the first
/// by two other places: the same seal count, other bytes. The first is
/// opened and locked, so its count is known.
fn two_newer_copies() -> (Files, Faraday) {
    let (mut app, _) = single_key_with_vault();
    clock_kept(&mut app);
    let (outbox, _) = saved_at_lock(&mut app);
    let (_, old) = vault_file(outbox);
    let a = testkit::kdbx_reseal(&old, "test phrase", 9).unwrap();
    let b = testkit::kdbx_reseal(&old, "test phrase", 10).unwrap();
    let files = vec![("a-new.kdbx".to_string(), a), ("b-new.kdbx".to_string(), b)];
    let app = unlocked_again(files.clone(), &["b-new.kdbx"], 0);
    (files, app)
}

/// The vault file at place `at` opened from the older-copy dialog.
fn opened_from_dialog(app: &mut Faraday, at: usize) {
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Open(at)));
    type_text(app, "test phrase");
    app.press(Action::Vault(V::Unlock));
    settle(app, |a| a.sheet == Some(Sheet::OlderCopy));
    assert_eq!(app.sheet, Some(Sheet::OlderCopy));
    app.press(Action::Vault(V::OlderOpen));
    settle(app, |a| !a.vaults().open.is_empty());
    app.press(Action::Vault(V::LoadWithKeys(0)));
}

#[test]
fn a_copy_sealed_as_many_times_with_other_bytes_dates_nothing_and_reads_no_older_copy() {
    let (_, mut app) = two_newer_copies();
    testkit::vault_lock_one(&mut app, 0);
    opened_from_dialog(&mut app, 1);
    app.press(Action::OpenWallet(0));
    let _ = app.frame();
    let (node, j) = line_holding(&app, 0, What::VaultStick(0));
    assert_eq!(node.checked[j], None);
    assert!(!node.older[j], "{node:?}");
}

#[test]
fn a_check_held_in_memory_is_not_written_into_an_older_copy_but_into_the_current_one() {
    let (mut app, _) = single_key_with_vault();
    clock_kept(&mut app);
    let (outbox, _) = saved_at_lock(&mut app);
    let (name, old) = vault_file(outbox);
    let new = testkit::kdbx_reseal(&old, "test phrase", 9).unwrap();
    let files = vec![("a-new.kdbx".to_string(), new), (name.clone(), old)];
    let mut app = unlocked_again(files, &[&name], 0);
    testkit::vault_lock_one(&mut app, 0);
    // A copy checked with no vault open is held.
    planned(&mut app, 0, 2);
    type_back_in(&mut app, 0, true);
    app.press(Action::BWhich(0));
    // The older copy opened: it takes nothing.
    opened_from_dialog(&mut app, 1);
    app.press(Action::OpenWallet(0));
    let _ = app.frame();
    assert_eq!(words_checked(&app, 0, "Place 1"), None);
    // The current copy, unlocked, takes it.
    testkit::vault_lock_one(&mut app, 0);
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Open(0)));
    type_text(&mut app, "test phrase");
    app.press(Action::Vault(V::Unlock));
    settle(&mut app, |a| !a.vaults().open.is_empty());
    app.press(Action::Vault(V::LoadWithKeys(0)));
    let held = (0..app.session.wallets.len()).find_map(|i| words_checked(&app, i, "Place 1"));
    assert_eq!(held.as_deref(), Some("2026-10-03"));
}

#[test]
fn marking_a_place_checked_in_an_older_copy_writes_nothing_and_says_so() {
    let (mut app, _) = single_key_with_vault();
    clock_kept(&mut app);
    let (outbox, _) = saved_at_lock(&mut app);
    let (name, old) = vault_file(outbox);
    let new = testkit::kdbx_reseal(&old, "test phrase", 9).unwrap();
    let files = vec![("a-new.kdbx".to_string(), new), (name.clone(), old)];
    let mut app = unlocked_again(files, &[&name], 0);
    testkit::vault_lock_one(&mut app, 0);
    opened_from_dialog(&mut app, 1);
    app.press(Action::OpenWallet(0));
    let _ = app.frame();
    let (node, j) = line_holding(&app, 0, What::VaultStick(0));
    assert!(node.older[j]);
    let (n, _) = node_of(&app, 0, &node.name);
    app.press(Action::Chart(C::Checked(Press::Loaded(0), n)));
    let _ = app.frame();
    assert!(shown(&mut app, faraday_strings::EN.chart_edit_not_current));
    let (node, j) = line_holding(&app, 0, What::VaultStick(0));
    assert!(node.older[j], "{node:?}");
    assert_eq!(node.checked[j], None);
}

#[test]
fn a_scan_cancelled_on_the_chart_leaves_no_place_for_the_next_match() {
    let (mut app, w) = single_key_with_vault();
    clock_kept(&mut app);
    let fp = app.session.keys[0].fingerprint.0;
    let press = Press::Loaded(w);
    // The checklist's Copy item is open, as the person left it.
    app.press(Action::Nav(Screen::Backup));
    app.press(Action::BStep(faraday_core::bstep::COPY));
    app.press(Action::OpenWallet(w));
    let _ = app.frame();
    let (n, j) = line_of(&app, w, "Place 2", "Key 1 words");
    open(&mut app, press, Target::Line(n, j));
    app.press(Action::Chart(C::Do(
        press,
        Step::Scan(fp, Some(At::Place(1))),
    )));
    assert_eq!(app.sheet, Some(Sheet::Scan));
    app.press(Action::Cancel);
    // The words typed back on the checklist: the plan keeps them at two
    // places, and the scan's place is not the answer.
    type_back_here(&mut app);
    assert!(shown(&mut app, "Which copy was this?"));
    app.press(Action::OpenWallet(w));
    let _ = app.frame();
    assert_eq!(words_checked(&app, w, "Place 2"), None);
}

#[test]
fn a_typed_check_closed_with_escape_leaves_no_place_for_the_next_match() {
    let (mut app, w) = single_key_with_vault();
    clock_kept(&mut app);
    let press = Press::Loaded(w);
    let fp = app.session.keys[0].fingerprint.0;
    let (n, j) = line_of(&app, w, "Place 2", "Key 1 words");
    open(&mut app, press, Target::Line(n, j));
    app.press(Action::Chart(C::Do(
        press,
        Step::Type(fp, faraday_core::bstep::COPY, Some(At::Place(1))),
    )));
    app.event(Event::Key(Key::Escape));
    // The words typed back on the checklist: the plan keeps them at two
    // places, and the line pressed before is not the answer.
    type_back_here(&mut app);
    assert!(shown(&mut app, "Which copy was this?"));
}

/// This power-on's plan changed and not yet saved in the vault: `what` is
/// taken off place `at` of its map.
fn plan_without(app: &mut Faraday, w: usize, at: usize, what: What) {
    let c = testkit::compute();
    let shape = app.plan_shape_now(w);
    faraday_core::testkit::change_backup_answers(app, |a| {
        let mut map = faraday_core::plan::Mapped::of(&c, &shape, a).map.clone();
        for s in map.iter_mut().filter(|s| s.at == At::Place(at)) {
            s.holds.retain(|(h, _)| *h != what);
        }
        a.map = Some(map);
    });
}

#[test]
fn a_match_at_a_place_whose_line_the_vaults_plan_does_not_have_dates_no_line() {
    let (mut app, w) = single_key_with_vault();
    clock_kept(&mut app);
    // This power-on's plan is changed, and not yet saved in the vault:
    // Place 1 no longer keeps the sheet.
    app.press(Action::Backup(w));
    app.press(Action::BChecklist);
    plan_without(&mut app, w, 0, What::Sheet);
    type_back_here(&mut app);
    app.press(Action::BWhich(0));
    app.press(Action::OpenWallet(w));
    let _ = app.frame();
    assert_eq!(words_checked(&app, w, "Place 1"), None);
    assert_eq!(words_checked(&app, w, "Place 2"), None);
}

#[test]
fn a_change_elsewhere_in_the_plan_does_not_stop_a_place_being_dated() {
    let (mut app, w) = single_key_with_vault();
    clock_kept(&mut app);
    // Place 2 no longer keeps the sheet; Place 1 is as the vault has it.
    app.press(Action::Backup(w));
    app.press(Action::BChecklist);
    plan_without(&mut app, w, 1, What::Sheet);
    type_back_here(&mut app);
    app.press(Action::BWhich(0));
    app.press(Action::OpenWallet(w));
    let _ = app.frame();
    assert_eq!(
        words_checked(&app, w, "Place 1").as_deref(),
        Some("2026-10-03")
    );
}

#[test]
fn an_answer_to_which_copy_after_the_plan_changed_dates_no_line() {
    let (mut app, w) = single_key_with_vault();
    clock_kept(&mut app);
    type_back(&mut app, w);
    assert!(shown(&mut app, "Which copy was this?"));
    // The plan changes under the question (as the wallet's own flow
    // changes it): Place 2 no longer keeps the words.
    plan_without(&mut app, w, 1, What::Words(0));
    app.press(Action::BWhich(1));
    app.press(Action::OpenWallet(w));
    let _ = app.frame();
    assert_eq!(words_checked(&app, w, "Place 1"), None);
    let (_, place2) = node_of(&app, w, "Place 2");
    assert!(place2.checked.iter().all(Option::is_none), "{place2:?}");
}

#[test]
fn a_plan_kept_in_an_older_line_form_is_still_dated_by_a_match() {
    use faraday_vault::records::{field, kind};
    let (mut app, w) = single_key_with_vault();
    clock_kept(&mut app);
    app.press(Action::Backup(w));
    app.press(Action::BChecklist);
    // The vault's plan has a line a plan saved before the layers were
    // withdrawn has: read past, so the plan is the same.
    let c = testkit::compute();
    let id = app.vaults().open[0].id;
    let at: Vec<usize> = app
        .vaults()
        .described_of(&c, id, kind::PLAN)
        .map(|(i, _, _)| i)
        .collect();
    let text = format!(
        "{}layers 1\n",
        app.vaults()
            .records(id)
            .get(at[0])
            .unwrap()
            .public_text(field::PLAN_ANSWERS)
            .unwrap()
    );
    testkit::vault_edit(&mut app, id, |s| {
        s.set_public_all_in(&c, id, &at, field::PLAN_ANSWERS, &[text.as_bytes()])
    })
    .unwrap();
    type_back_here(&mut app);
    app.press(Action::BWhich(0));
    app.press(Action::OpenWallet(w));
    let _ = app.frame();
    assert_eq!(
        words_checked(&app, w, "Place 1").as_deref(),
        Some("2026-10-03")
    );
}

#[test]
fn a_stick_read_with_the_clock_not_checked_is_dated_at_set_the_time() {
    let (mut app, _) = single_key_with_vault();
    clock_kept(&mut app);
    let (outbox, _) = saved_at_lock(&mut app);
    let (name, bytes) = vault_file(outbox);
    let mut app = unlocked_again_clock(vec![(name.clone(), bytes)], &[&name], 0, false);
    app.press(Action::Vault(V::LoadWithKeys(0)));
    app.press(Action::OpenWallet(0));
    let _ = app.frame();
    // The note is written with no date: the line reads checked, no date.
    assert_eq!(vault_stick_checked(&app), Some(String::new()));
    clock_kept(&mut app);
    app.press(Action::OpenWallet(0));
    let _ = app.frame();
    assert_eq!(vault_stick_checked(&app).as_deref(), Some("2026-10-03"));
}

#[test]
fn a_question_left_unanswered_dates_nothing_and_a_new_match_asks_again() {
    let (mut app, w) = single_key_with_vault();
    clock_kept(&mut app);
    type_back(&mut app, w);
    assert!(shown(&mut app, "Which copy was this?"));
    // Another item opens: the question is gone, nothing is dated.
    app.press(Action::BStep(faraday_core::bstep::BLANK));
    assert!(!shown(&mut app, "Which copy was this?"));
    // An answer to it dates nothing.
    app.press(Action::BWhich(0));
    app.press(Action::OpenWallet(w));
    let _ = app.frame();
    assert_eq!(words_checked(&app, w, "Place 1"), None);
    assert_eq!(words_checked(&app, w, "Place 2"), None);
    // A match again asks again, and its answer is dated.
    type_back_here(&mut app);
    assert!(shown(&mut app, "Which copy was this?"));
    app.press(Action::BWhich(0));
    app.press(Action::OpenWallet(w));
    let _ = app.frame();
    assert_eq!(
        words_checked(&app, w, "Place 1").as_deref(),
        Some("2026-10-03")
    );
    assert_eq!(words_checked(&app, w, "Place 2"), None);
}

#[test]
fn skipping_a_codex32_string_dates_nothing() {
    use crate::codex32_backup::{c32, checklist_at_codex32, on_unseeded, with_set_on};
    let mut app = on_unseeded();
    fresh_vault(&mut app);
    let mut app = with_set_on(app);
    app.storage(StorageEvent::Clock {
        unix_secs: 1_791_000_000,
    });
    clock_kept(&mut app);
    checklist_at_codex32(&mut app);
    app.press(Action::BVaultSave(0));
    let a = faraday_core::plan::codex32::index_of(b'a').unwrap();
    c32(&mut app, faraday_core::codex32_backup::Press::Open(a as u8));
    c32(&mut app, faraday_core::codex32_backup::Press::Skip);
    c32(&mut app, faraday_core::codex32_backup::Press::Skip);
    app.press(Action::OpenWallet(0));
    let _ = app.frame();
    let (node, j) = line_holding(&app, 0, What::Codex32Share(0, a));
    assert_eq!(node.checked[j], None, "{node:?}");
}

/// The vault's plan record in the plan form saved with one vault, which
/// holds every seed here: a `sticks` row alone, no `vault` rows.
fn plan_in_one_vault_form(app: &mut Faraday) {
    use faraday_vault::records::{field, kind};
    let c = testkit::compute();
    let id = app.vaults().open[0].id;
    let at: Vec<usize> = app
        .vaults()
        .described_of(&c, id, kind::PLAN)
        .map(|(i, _, _)| i)
        .collect();
    let old = app
        .vaults()
        .records(id)
        .get(at[0])
        .unwrap()
        .public_text(field::PLAN_ANSWERS)
        .unwrap()
        .to_string();
    let text: String = old
        .lines()
        .filter(|l| !l.starts_with("vault ") && !l.starts_with("sticks "))
        .map(|l| format!("{l}\n"))
        .chain(["sticks 01\n".to_string()])
        .collect();
    assert_ne!(text, old);
    testkit::vault_edit(app, id, |s| {
        s.set_public_all_in(&c, id, &at, field::PLAN_ANSWERS, &[text.as_bytes()])
    })
    .unwrap();
}

#[test]
fn a_stick_read_dates_a_plan_in_the_one_vault_form_before_the_keys_are_loaded() {
    let (mut app, _) = single_key_with_vault();
    clock_kept(&mut app);
    plan_in_one_vault_form(&mut app);
    // The backup's record is kept across the lock, so the seed is known
    // by its master id with no key in memory.
    let (outbox, kept) = saved_at_lock(&mut app);
    let (name, bytes) = vault_file(outbox);
    let mut app = unlocked_again_keys(
        vec![(name.clone(), bytes)],
        kept,
        &[&name],
        0,
        true,
        Keys::None,
    );
    app.press(Action::Vault(V::LoadWithKeys(0)));
    app.press(Action::OpenWallet(0));
    let _ = app.frame();
    assert_eq!(vault_stick_checked(&app).as_deref(), Some("2026-10-03"));
}

/// The held-check setup: a stick read with the clock not checked, its
/// vault really locked (sealed, so the lock's copy is the one in the
/// Outbox) and the time set with no vault open. The files hold the newer
/// copy `a-new.kdbx` from elsewhere beside the stick's. The row of the
/// copy this device's lock sealed, and the row of the newer copy.
fn held_stick_check() -> (Faraday, usize, usize) {
    let (mut app, _) = single_key_with_vault();
    clock_kept(&mut app);
    let (outbox, _) = saved_at_lock(&mut app);
    let (name, old) = vault_file(outbox);
    // Sealed twice elsewhere: past the one seal this device's lock makes.
    let new = testkit::kdbx_reseal(&old, "test phrase", 9).unwrap();
    let new = testkit::kdbx_reseal(&new, "test phrase", 10).unwrap();
    let files = vec![("a-new.kdbx".to_string(), new), (name.clone(), old)];
    // The seed's key is in memory and no wallet is loaded or has a backup
    // record, so the check is held for the clock.
    let mut app = unlocked_again_keys(files, Vec::new(), &[&name], 1, false, Keys::SeedOnly);
    // A lock seals the changed vault (the empty-date note is a change).
    crate::common::top_up(&mut app);
    app.press(Action::Lock);
    testkit::settle(&mut app);
    assert!(app.vaults().open.is_empty());
    clock_kept(&mut app);
    let rows = app.vault_files();
    let sealed = rows
        .iter()
        .position(|f| f.in_outbox)
        .expect("a sealed copy");
    let newer = rows.iter().position(|f| f.name == "a-new.kdbx").unwrap();
    (app, sealed, newer)
}

fn unlock_row(app: &mut Faraday, at: usize) {
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Open(at)));
    type_text(app, "test phrase");
    app.press(Action::Vault(V::Unlock));
    settle(app, |a| !a.vaults().open.is_empty());
    app.press(Action::Vault(V::LoadWithKeys(0)));
    app.press(Action::OpenWallet(0));
    let _ = app.frame();
}

#[test]
fn a_stick_check_held_for_the_clock_is_dated_in_the_copy_this_devices_lock_sealed() {
    let (mut app, sealed, _) = held_stick_check();
    unlock_row(&mut app, sealed);
    assert_eq!(vault_stick_checked(&app).as_deref(), Some("2026-10-03"));
}

#[test]
fn a_stick_check_held_for_the_clock_is_not_dated_in_a_newer_copy_from_elsewhere() {
    let (mut app, _, newer) = held_stick_check();
    unlock_row(&mut app, newer);
    assert!(
        !app.vaults().open[0].is_not_current(),
        "the newer copy is current"
    );
    assert_eq!(vault_stick_checked(&app), None);
}

#[test]
fn a_stick_read_with_no_key_loaded_is_dated_once_the_keys_are_loaded_and_the_time_set() {
    let (mut app, _) = single_key_with_vault();
    let (outbox, kept) = saved_at_lock(&mut app);
    let (name, bytes) = vault_file(outbox);
    let mut app = unlocked_again_keys(
        vec![(name.clone(), bytes)],
        kept,
        &[&name],
        0,
        false,
        Keys::None,
    );
    app.press(Action::Vault(V::LoadWithKeys(0)));
    clock_kept(&mut app);
    app.press(Action::OpenWallet(0));
    let _ = app.frame();
    assert_eq!(vault_stick_checked(&app).as_deref(), Some("2026-10-03"));
}

#[test]
fn a_stick_of_a_vault_holding_seeds_but_not_the_wallet_is_dated_before_the_keys_are_loaded() {
    use faraday_vault::records::{field, kind};
    let (mut app, _) = single_key_with_vault();
    // The plan keeps the wallet description out of the vault: the vault
    // holds the seed, and the description it keeps is no part of the plan.
    let c = testkit::compute();
    let id = app.vaults().open[0].id;
    let at: Vec<usize> = app
        .vaults()
        .described_of(&c, id, kind::PLAN)
        .map(|(i, _, _)| i)
        .collect();
    let text = String::from_utf8_lossy(
        app.vaults()
            .records(id)
            .get(at[0])
            .unwrap()
            .public_text(field::PLAN_ANSWERS)
            .unwrap()
            .as_bytes(),
    )
    .into_owned();
    let edited: Vec<String> = text
        .lines()
        .map(|l| match l.strip_prefix("wallet ") {
            Some(bits) => {
                let mut b: Vec<char> = bits.chars().collect();
                b[1] = '0';
                format!("wallet {}", b.into_iter().collect::<String>())
            }
            None => l.to_string(),
        })
        .collect();
    let edited = format!("{}\n", edited.join("\n"));
    assert_ne!(edited, text, "the plan kept the wallet out of the vault");
    testkit::vault_edit(&mut app, id, |s| {
        s.set_public_all_in(&c, id, &at, field::PLAN_ANSWERS, &[edited.as_bytes()])
    })
    .unwrap();
    let (outbox, kept) = saved_at_lock(&mut app);
    let (name, bytes) = vault_file(outbox);
    let mut app = unlocked_again_keys(
        vec![(name.clone(), bytes)],
        kept,
        &[&name],
        0,
        true,
        Keys::None,
    );
    app.press(Action::Vault(V::LoadWithKeys(0)));
    app.press(Action::OpenWallet(0));
    let _ = app.frame();
    assert_eq!(vault_stick_checked(&app).as_deref(), Some("2026-10-03"));
}

/// Unlocks the vault file `name`, with the others already open kept.
fn unlock_named(app: &mut Faraday, name: &str) {
    let open = app.vaults().open.len();
    let at = app
        .vault_files()
        .iter()
        .position(|f| f.name == name || f.copies.iter().any(|c| c == name))
        .expect("the vault file is listed");
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Open(at)));
    type_text(app, "test phrase");
    app.press(Action::Vault(V::Unlock));
    settle(app, |a| a.vaults().open.len() > open);
}

/// The plan in vault A and the seed and wallet in vault B, both locked and
/// their files and the kept state: the names of A and of B.
fn plan_in_a_wallet_in_b() -> (Files, Files, String, String) {
    use faraday_vault::records::kind;
    let mut app = desktop();
    let w = one_key(&mut app);
    fresh_vault(&mut app);
    // Vault A keeps the plan.
    planned(&mut app, w, 2);
    let a_name = String::from(app.vaults().open[0].name.as_str());
    // Vault B keeps the seed and the wallet.
    for i in 0..8u8 {
        app.event(Event::Entropy(faraday_shell_api::EntropyBytes::new(
            [0x70 + i; 32],
        )));
    }
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Create));
    for second in [false, true] {
        app.press(Action::Vault(V::CFocus(0, second)));
        type_text(&mut app, "test phrase");
    }
    app.press(Action::Vault(V::CGo));
    settle(&mut app, |a| a.screen == Screen::Unlock);
    type_text(&mut app, "test phrase");
    app.press(Action::Vault(V::Unlock));
    settle(&mut app, |a| a.vaults().open.len() == 2);
    assert_eq!(app.vaults().open.len(), 2, "vault B did not open");
    let b_name = String::from(app.vaults().open[1].name.as_str());
    testkit::vault_current(&mut app, 1);
    app.press(Action::Backup(w));
    app.press(Action::BChecklist);
    app.press(Action::BVaultSave(0));
    app.press(Action::Nav(Screen::VaultContents));
    app.press(Action::Vault(V::AddKind(1)));
    app.press(Action::Vault(V::SaveWallet(w)));
    let c = testkit::compute();
    let (a_id, b_id) = (app.vaults().open[0].id, app.vaults().open[1].id);
    assert!(
        app.vaults()
            .described_of(&c, a_id, kind::PLAN)
            .next()
            .is_some()
    );
    assert!(
        app.vaults()
            .described_of(&c, a_id, kind::WALLET)
            .next()
            .is_none()
    );
    assert!(
        app.vaults()
            .described_of(&c, b_id, kind::WALLET)
            .next()
            .is_some()
    );
    let (outbox, kept) = saved_at_lock(&mut app);
    let files: Vec<(String, Vec<u8>)> = outbox
        .into_iter()
        .filter(|(n, _)| n.ends_with(".kdbx"))
        .collect();
    (files, kept, a_name, b_name)
}

/// B's stick read and both vaults unlocked, the keys loaded from B, the
/// wallet's chart open.
fn stick_of_b_read(app: &mut Faraday, a_name: &str, b_name: &str) {
    testkit::vault_read_from_stick(app, b_name, "Stick A");
    unlock_named(app, a_name);
    unlock_named(app, b_name);
    app.press(Action::Vault(V::LoadWithKeys(1)));
    app.press(Action::OpenWallet(0));
    let _ = app.frame();
}

#[test]
fn a_stick_is_dated_when_the_plan_is_in_one_vault_and_the_wallet_in_another() {
    let (files, kept, a_name, b_name) = plan_in_a_wallet_in_b();
    let mut app = device(files, kept);
    clock_kept(&mut app);
    stick_of_b_read(&mut app, &a_name, &b_name);
    assert_eq!(vault_stick_checked(&app).as_deref(), Some("2026-10-03"));
}

#[test]
fn a_stick_read_on_a_power_on_with_no_backup_record_is_dated_in_the_vault_that_keeps_the_plan() {
    let (files, _, a_name, b_name) = plan_in_a_wallet_in_b();
    let mut app = device(files, Vec::new());
    add_key(&mut app, 0);
    clock_kept(&mut app);
    stick_of_b_read(&mut app, &a_name, &b_name);
    assert_eq!(vault_stick_checked(&app).as_deref(), Some("2026-10-03"));
}

#[test]
fn a_stick_read_with_no_record_and_no_clock_is_dated_at_set_the_time() {
    let (files, _, a_name, b_name) = plan_in_a_wallet_in_b();
    let mut app = device(files, Vec::new());
    add_key(&mut app, 0);
    stick_of_b_read(&mut app, &a_name, &b_name);
    clock_kept(&mut app);
    app.press(Action::OpenWallet(0));
    let _ = app.frame();
    assert_eq!(vault_stick_checked(&app).as_deref(), Some("2026-10-03"));
}

#[test]
fn a_stick_check_made_with_a_backup_record_is_not_written_into_a_newer_copy_from_elsewhere() {
    let (mut app, _) = single_key_with_vault();
    clock_kept(&mut app);
    let (outbox, kept) = saved_at_lock(&mut app);
    let (name, old) = vault_file(outbox);
    let new = testkit::kdbx_reseal(&old, "test phrase", 9).unwrap();
    let new = testkit::kdbx_reseal(&new, "test phrase", 10).unwrap();
    let files = vec![("a-new.kdbx".to_string(), new), (name.clone(), old)];
    // The backup's record is kept, and the clock is not checked.
    let mut app = unlocked_again_keys(files, kept, &[&name], 1, false, Keys::SeedOnly);
    assert!(!app.vaults().open.is_empty());
    crate::common::top_up(&mut app);
    testkit::vault_lock_one(&mut app, 0);
    testkit::settle(&mut app);
    clock_kept(&mut app);
    assert!(app.vaults().open.is_empty());
    unlock_named(&mut app, "a-new.kdbx");
    assert!(
        !app.vaults().open[0].is_not_current(),
        "the newer copy is current"
    );
    app.press(Action::Vault(V::LoadWithKeys(0)));
    app.press(Action::OpenWallet(0));
    let _ = app.frame();
    assert_eq!(vault_stick_checked(&app), None);
}

#[test]
fn a_key_confirmed_in_a_copy_that_is_not_current_writes_nothing_and_says_so() {
    let (mut app, _) = single_key_with_vault();
    clock_kept(&mut app);
    let (outbox, _) = saved_at_lock(&mut app);
    let (name, old) = vault_file(outbox);
    let new = testkit::kdbx_reseal(&old, "test phrase", 9).unwrap();
    let files = vec![("a-new.kdbx".to_string(), new), (name.clone(), old)];
    let mut app = unlocked_again(files, &[&name], 0);
    testkit::vault_lock_one(&mut app, 0);
    opened_from_dialog(&mut app, 1);
    app.press(Action::OpenWallet(0));
    let _ = app.frame();
    app.press(Action::Chart(C::KeyChecked(Press::Loaded(0), 0)));
    let _ = app.frame();
    assert!(shown(&mut app, faraday_strings::EN.chart_edit_not_current));
    assert_eq!(chart(&app, 0).keys[0].checked, None);
}

#[test]
fn a_wallet_whose_seed_has_a_passphrase_has_its_paper_and_stick_checks_dated() {
    let (mut app, w) = single_key_with_vault_pass("hunter2");
    clock_kept(&mut app);
    // A paper check, the words typed back in the checklist.
    type_back(&mut app, w);
    if app.backup().is_some_and(|b| b.which.is_some()) {
        app.press(Action::BWhich(0));
    }
    app.press(Action::OpenWallet(w));
    let _ = app.frame();
    assert_eq!(
        words_checked(&app, w, "Place 1").as_deref(),
        Some("2026-10-03")
    );
    // A stick check, the vault read off its stick before any key is loaded.
    let (outbox, kept) = saved_at_lock(&mut app);
    let (name, bytes) = vault_file(outbox);
    let mut app = unlocked_again_keys(
        vec![(name.clone(), bytes)],
        kept,
        &[&name],
        0,
        true,
        Keys::None,
    );
    app.press(Action::Vault(V::LoadWithKeys(0)));
    app.press(Action::OpenWallet(0));
    let _ = app.frame();
    assert_eq!(vault_stick_checked(&app).as_deref(), Some("2026-10-03"));
}

#[test]
fn a_codex32_set_made_in_the_checklist_has_a_string_typed_back_dated() {
    use crate::codex32_backup::{c32, checklist_at_codex32, on_unseeded, with_set_on};
    use faraday_core::codex32_backup::{Press as P, SetPress};
    use faraday_ui::keyboard::OskPress;
    let mut app = on_unseeded();
    fresh_vault(&mut app);
    let mut app = with_set_on(app);
    app.storage(StorageEvent::Clock {
        unix_secs: 1_791_000_000,
    });
    clock_kept(&mut app);
    for i in 0..8u8 {
        app.event(Event::Entropy(faraday_shell_api::EntropyBytes::new(
            [0x60 + i; 32],
        )));
    }
    let ix = |c: char| faraday_core::plan::codex32::index_of(c as u8).unwrap() as u8;
    // The plan is customized: D is put with A at place 0, and the plan is
    // saved at the checklist with D there.
    {
        use crate::common::{board, move_to};
        use faraday_core::board::{BoardPress, Thing, Zone};
        board(&mut app, BoardPress::Customize);
        let d_at = (0..4)
            .find(|&p| {
                crate::common::backup_held(&app, At::Place(p))
                    .contains(&What::Codex32Share(0, ix('d') as usize))
            })
            .expect("D is on paper somewhere");
        if d_at != 0 {
            move_to(
                &mut app,
                Zone::Paper(d_at),
                Thing::String(0, usize::from(ix('d'))),
                Zone::Paper(0),
            );
        }
        assert!(
            crate::common::backup_held(&app, At::Place(0))
                .contains(&What::Codex32Share(0, ix('d') as usize)),
            "D is with A"
        );
    }
    checklist_at_codex32(&mut app);
    // A new set of this seed is made in the checklist, of two strings, so
    // that D leaves its place.
    c32(&mut app, P::Set(SetPress::Rotate));
    c32(&mut app, P::Set(SetPress::Mark(ix('a'), 0)));
    c32(&mut app, P::Set(SetPress::Mark(ix('c'), 1)));
    c32(&mut app, P::Set(SetPress::Counted));
    c32(&mut app, P::Set(SetPress::Destroyed(ix('a'))));
    c32(&mut app, P::Set(SetPress::Destroyed(ix('d'))));
    for ch in "w0za".chars() {
        c32(&mut app, P::Key(OskPress::Char(ch)));
    }
    c32(&mut app, P::Set(SetPress::N(-1)));
    c32(&mut app, P::Set(SetPress::MakeNew));
    // D left its place: the place's line is not what the saved record
    // before the set had.
    assert!(
        !crate::common::backup_held(&app, At::Place(0))
            .contains(&What::Codex32Share(0, ix('d') as usize)),
        "the new set has no D"
    );
    // The first string on paper is written and typed back.
    let facts = app.session.keys[0].codex32.clone().unwrap();
    let j = facts
        .strings
        .first()
        .map(|&(c, _)| faraday_core::plan::codex32::index_of(c).unwrap())
        .expect("a string of the new set");
    let text = app
        .backup()
        .unwrap()
        .c32
        .as_ref()
        .unwrap()
        .made(j)
        .expect("the string was made")
        .to_string();
    c32(&mut app, P::Open(j as u8));
    for ch in text[3..].chars() {
        c32(&mut app, P::Key(OskPress::Char(ch)));
    }
    c32(&mut app, P::Next);
    app.press(Action::OpenWallet(0));
    let _ = app.frame();
    let (node, k) = line_holding(&app, 0, What::Codex32Share(0, j));
    assert_eq!(node.checked[k].as_deref(), Some("2026-10-03"), "{node:?}");
}

/// The checklist at the codex32 item of a seed, its wallet's backup past
/// the lock with the writes not done; with `other`, the backup past its
/// lock is that of a second wallet over the same seed, and the checklist
/// is the first wallet's.
fn checklist_past_a_flows_lock(other: bool) -> Faraday {
    checklist_past_a_lock(other, false)
}

/// As above; with `unloaded`, the flow's wallet is not among the
/// session's wallets, as after a lock.
fn checklist_past_a_lock(other: bool, unloaded: bool) -> Faraday {
    use crate::codex32_backup::{checklist_at_codex32, on_unseeded, with_set_on};
    use faraday_bip::keys::ScriptType;
    use faraday_core::backup_flow::{BackupFlow, Step};
    use faraday_keys::Spec;
    let mut app = on_unseeded();
    fresh_vault(&mut app);
    let mut app = with_set_on(app);
    for i in 0..8u8 {
        app.event(Event::Entropy(faraday_shell_api::EntropyBytes::new(
            [0x60 + i; 32],
        )));
    }
    let mut at = 0;
    if other {
        let c = testkit::compute();
        let text = app
            .session
            .keys
            .key_text(&c, 0, &Spec::Single(ScriptType::NativeSegwit, 1))
            .unwrap()
            .unwrap();
        app.storage(StorageEvent::Restored {
            inbox: faraday_core::files_from(vec![(
                "second.txt".into(),
                format!("wpkh({}/<0;1>/*)", &*text).into_bytes(),
            )]),
            outbox: Vec::new(),
            kept: Vec::new(),
        });
        app.press(Action::LoadWallet(0));
        assert_eq!(app.session.wallets.len(), 2);
        at = 1;
        app.press(Action::Backup(0));
    }
    checklist_at_codex32(&mut app);
    let (shape, answers) = app.board_plan().expect("the plan");
    let mut f = BackupFlow::new_wallet(&testkit::compute(), shape, answers, "w");
    let wf = f.wallet.as_mut().expect("a wallet flow");
    wf.checksum = Some(if unloaded {
        "notloaded".to_string()
    } else {
        app.session.wallets[at].checksum().to_string()
    });
    f.made = true;
    f.step = Step::Lock;
    testkit::flow_push(&mut app, f);
    app
}

const PAST_LOCK: &str = "This backup is past its lock; finish the writes, then Back up again";

#[test]
fn a_codex32_set_is_not_rotated_in_the_checklist_past_a_flows_lock() {
    use crate::codex32_backup::c32;
    use faraday_core::codex32_backup::{Press as P, SetPress, SetStep};
    let mut app = checklist_past_a_flows_lock(false);
    let before = app.session.keys[0].codex32.clone();
    c32(&mut app, P::Set(SetPress::Rotate));
    let st = app.backup().unwrap().c32.as_ref().unwrap();
    // Refused at the press: no string to destroy is asked for.
    assert!(matches!(st.set, SetStep::None), "{:?}", st.set);
    assert_eq!(st.note.as_deref(), Some(PAST_LOCK));
    assert_eq!(app.session.keys[0].codex32, before, "the set is as it was");
}

#[test]
fn a_codex32_share_is_not_added_in_the_checklist_past_a_flows_lock() {
    use crate::codex32_backup::c32;
    use faraday_core::codex32_backup::{Press as P, SetPress, SetStep};
    let mut app = checklist_past_a_flows_lock(false);
    let before = app.session.keys[0].codex32.clone();
    let (_, plan) = app.board_plan().expect("the plan");
    let indices = plan.c32(0).unwrap().indices.clone();
    c32(&mut app, P::Set(SetPress::Add));
    let st = app.backup().unwrap().c32.as_ref().unwrap();
    assert!(matches!(st.set, SetStep::None), "{:?}", st.set);
    assert_eq!(st.note.as_deref(), Some(PAST_LOCK));
    assert_eq!(app.session.keys[0].codex32, before, "no share is added");
    let (_, plan) = app.board_plan().expect("the plan");
    assert_eq!(plan.c32(0).unwrap().indices, indices);
}

#[test]
fn a_codex32_set_is_not_rotated_while_another_wallet_of_its_seed_is_past_its_lock() {
    use crate::codex32_backup::c32;
    use faraday_core::codex32_backup::{Press as P, SetPress, SetStep};
    let mut app = checklist_past_a_flows_lock(true);
    assert_eq!(
        app.backup().unwrap().wallet,
        0,
        "the first wallet's checklist"
    );
    let before = app.session.keys[0].codex32.clone();
    c32(&mut app, P::Set(SetPress::Rotate));
    let st = app.backup().unwrap().c32.as_ref().unwrap();
    assert!(matches!(st.set, SetStep::None), "{:?}", st.set);
    assert_eq!(st.note.as_deref(), Some(PAST_LOCK));
    assert_eq!(app.session.keys[0].codex32, before);
}

#[test]
fn a_codex32_set_is_not_rotated_past_the_lock_of_a_wallet_that_is_not_loaded() {
    use crate::codex32_backup::c32;
    use faraday_core::codex32_backup::{Press as P, SetPress, SetStep};
    let mut app = checklist_past_a_lock(false, true);
    let before = app.session.keys[0].codex32.clone();
    c32(&mut app, P::Set(SetPress::Rotate));
    let st = app.backup().unwrap().c32.as_ref().unwrap();
    assert!(matches!(st.set, SetStep::None), "{:?}", st.set);
    assert_eq!(st.note.as_deref(), Some(PAST_LOCK));
    assert_eq!(app.session.keys[0].codex32, before);
}

#[test]
fn how_many_codex32_strings_exist_is_not_changed_past_a_flows_lock() {
    use crate::codex32_backup::c32;
    use faraday_core::codex32_backup::Press as P;
    let mut app = checklist_past_a_lock(false, true);
    let (_, plan) = app.board_plan().expect("the plan");
    let before = plan.c32(0).unwrap().n();
    c32(&mut app, P::Exist(1));
    let st = app.backup().unwrap().c32.as_ref().unwrap();
    assert_eq!(st.note.as_deref(), Some(PAST_LOCK));
    let (_, plan) = app.board_plan().expect("the plan");
    assert_eq!(plan.c32(0).unwrap().n(), before);
}

#[test]
fn a_place_with_only_its_words_dated_reads_not_checked_on_the_chart_and_on_backups() {
    let (mut app, w) = single_key_with_vault();
    clock_kept(&mut app);
    type_back(&mut app, w);
    app.press(Action::BWhich(0));
    app.press(Action::OpenWallet(w));
    let _ = app.frame();
    // The words are dated, the wallet sheet there never was.
    assert_eq!(
        words_checked(&app, w, "Place 1").as_deref(),
        Some("2026-10-03")
    );
    let (_, node) = node_of(&app, w, "Place 1");
    assert_eq!(
        node.age.as_ref().unwrap().line(),
        ("Never checked".to_string(), true)
    );
    let all = app.backups_now(&testkit::compute());
    let line = all[0]
        .lines
        .clone()
        .unwrap()
        .into_iter()
        .find(|l| l.place == "Place 1")
        .unwrap();
    assert_eq!(
        (line.state, line.tone),
        (
            "Not checked".to_string(),
            faraday_core::wallet_backup::Tone::Warn
        )
    );
}

#[test]
fn a_stick_read_from_an_older_copy_reads_older_copy_on_its_node_and_never_clears_the_strip() {
    let (mut app, w) = single_key_with_vault();
    clock_kept(&mut app);
    // Every copy is checked before the lock: only the stick can make
    // Place 2 due.
    for place in ["Place 1", "Place 2"] {
        let (n, _) = node_of(&app, w, place);
        app.press(Action::Chart(C::Checked(Press::Loaded(w), n)));
    }
    let (outbox, _) = saved_at_lock(&mut app);
    let (name, old) = vault_file(outbox);
    let new = testkit::kdbx_reseal(&old, "test phrase", 9).unwrap();
    let files = vec![("a-new.kdbx".to_string(), new), (name.clone(), old)];
    let mut app = unlocked_again(files, &[&name], 0);
    testkit::vault_lock_one(&mut app, 0);
    opened_from_dialog(&mut app, 1);
    app.press(Action::OpenWallet(0));
    let _ = app.frame();
    let (node, _) = line_holding(&app, 0, What::VaultStick(0));
    assert_eq!(
        node.age.as_ref().unwrap().line(),
        ("Older copy".to_string(), true)
    );
    let texts = app.drawn_texts().join(" ");
    assert!(texts.contains("To check: Place 2"), "{texts}");
    assert!(!texts.contains("Place 1,"), "{texts}");
    assert!(!texts.contains("Every place checked"), "{texts}");
}
