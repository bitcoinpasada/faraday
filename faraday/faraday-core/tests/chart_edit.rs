//! Changing the backup on the wallet's chart (`docs/NEW-WALLET.md`
//! §9.5–§9.7): a move, another copy, a thing destroyed or a place removed
//! shows the check's three lines before and after and is made only once
//! confirmed; from then the map is the plan, kept in the vault, and it
//! loads back the same on another power-on. Names, dates and marks are
//! kept in the vault alone, never in what the device keeps across a lock.
//! A thing marked lost or exposed says what follows, and a key not held
//! here can be given its holder's name and the date its backup was
//! confirmed.

use faraday_core::glance::{Backup, BackupNode, Glance, Press};
use faraday_core::glance_sheet::{ChartAction as C, Step, Target};
use faraday_core::plan::{At, Edit, Found, Lost, Mark, What};
use faraday_core::vaults::VaultAction as V;
use faraday_core::{Action, BStage, Faraday, Screen, Sheet, StorageCommand, StorageEvent, testkit};
use osk_shell_api::{App, BootState, DisplayInfo, Event, Key, SecureHardware};

type Files = Vec<(String, Vec<u8>)>;

/// A tall desktop, so a sheet's rows are all on screen.
fn device(inbox: Files, kept: Files) -> Faraday {
    let mut app = Faraday::new();
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
        inbox,
        outbox: Vec::new(),
        kept,
    });
    app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    app.storage(StorageEvent::Clock {
        unix_secs: 1_791_000_000,
    });
    app.vaults.ms_per_unit = Some(180);
    let _ = app.frame();
    app
}

fn desktop() -> Faraday {
    device(Vec::new(), Vec::new())
}

fn type_text(app: &mut Faraday, text: &str) {
    for c in text.chars() {
        app.event(Event::Key(Key::Char(c)));
    }
}

fn add_key(app: &mut Faraday, n: usize) {
    app.press(Action::Entry(None));
    type_text(app, &testkit::test_words(testkit::TEST_SEEDS[n].0));
    app.press(Action::EntryAdd);
}

/// Test key 1 typed in and a one-key wallet over it.
fn one_key(app: &mut Faraday) -> usize {
    add_key(app, 0);
    let text = faraday_core::create::NewKind::NativeSegwit
        .key_text(&app.session.keys[0].master)
        .unwrap();
    app.session
        .add_wallet("Spending", &format!("wpkh({text}/<0;1>/*)"), "test")
        .unwrap()
}

/// Test keys 1 and 2 typed in and the 2-of-3 over the three test keys:
/// the third is a cosigner's.
fn two_of_three(app: &mut Faraday) -> usize {
    add_key(app, 0);
    add_key(app, 1);
    app.session
        .add_wallet("Savings", &testkit::savings(), "test")
        .unwrap()
}

fn settle(app: &mut Faraday, until: impl Fn(&Faraday) -> bool) {
    for t in 1..60u64 {
        if until(app) {
            break;
        }
        let _ = app.frame();
        app.event(Event::Tick { now_ms: t * 1000 });
    }
}

/// A vault made and unlocked, with the passphrase "test phrase".
fn fresh_vault(app: &mut Faraday) {
    for i in 0..8u8 {
        app.event(Event::Entropy(osk_shell_api::EntropyBytes::new(
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
    settle(app, |a| !a.vaults.open.is_empty());
    assert_eq!(app.vaults.open.len(), 1, "the vault did not open");
}

/// The backup of wallet `w` planned with preset `preset` and its
/// checklist made; then the wallet's card.
fn planned(app: &mut Faraday, w: usize, preset: u8) {
    app.press(Action::Backup(w));
    app.press(Action::BPreset(preset));
    app.press(Action::BChecklist);
    app.press(Action::OpenWallet(w));
    let _ = app.frame();
}

fn chart(app: &Faraday, w: usize) -> Glance {
    faraday_core::glance::of(app, w).expect("the wallet has a chart")
}

fn nodes(g: &Glance) -> Vec<BackupNode> {
    match &g.backup {
        Backup::Plan { nodes, .. } => nodes.clone(),
        other => panic!("no plan on the chart: {other:?}"),
    }
}

/// Backup node `name` on wallet `w`'s chart: its place in the row.
fn node_of(app: &Faraday, w: usize, name: &str) -> (u8, BackupNode) {
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

fn open(app: &mut Faraday, press: Press, t: Target) {
    app.press(Action::Chart(C::Open(press, t)));
    let _ = app.frame();
    assert_eq!(app.sheet, Some(Sheet::Chart), "{t:?} opened no sheet");
}

fn shown(app: &mut Faraday, want: &str) -> bool {
    app.drawn_texts().iter().any(|t| t == want)
}

/// Asks for `edit` as its row does, and confirms it once the check
/// before and after is on the sheet.
fn edit_and_confirm(app: &mut Faraday, press: Press, edit: Edit) {
    app.press(Action::Chart(C::Edit(press, edit)));
    let _ = app.frame();
    assert_eq!(
        app.chart,
        Some((press, Target::Confirm)),
        "{edit:?} asked nothing"
    );
    let confirm = Action::Chart(C::Confirm(press));
    assert!(app.offers(confirm), "{edit:?} offers no Confirm");
    app.press(confirm);
    let _ = app.frame();
}

/// What the lock saved: For the stick, and the kept state.
fn saved_at_lock(app: &mut Faraday) -> (Files, Files) {
    app.press(Action::Lock);
    let mut last = (Vec::new(), Vec::new());
    while let Some(c) = app.poll_storage() {
        if let StorageCommand::SaveBoxes { outbox, kept, .. } = c {
            last = (outbox, kept);
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
fn single_key_with_vault() -> (Faraday, usize) {
    let mut app = desktop();
    let w = one_key(&mut app);
    fresh_vault(&mut app);
    planned(&mut app, w, 1);
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
    let ask = app.chart_work.ask.as_ref().expect("the move is asked");
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
    let _ = app.frame();
    let ask = app.chart_work.ask.as_ref().expect("asked");
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
        .filter(|(n, _)| n.ends_with(".ofv"))
        .collect();
    assert_eq!(vault.len(), 1, "the vault was not put For the stick");
    let mut app = device(vault, Vec::new());
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Open(0)));
    type_text(&mut app, "test phrase");
    app.press(Action::Vault(V::Unlock));
    settle(&mut app, |a| !a.vaults.open.is_empty());
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
fn changing_the_plan_after_an_edit_says_the_changes_are_replaced() {
    let (mut app, w) = single_key_with_vault();
    let press = Press::Loaded(w);
    edit_and_confirm(
        &mut app,
        press,
        Edit::Move(At::Place(0), What::Words(0), At::Place(2)),
    );
    open(&mut app, press, Target::Wallet);
    let replan = Action::Chart(C::Open(press, Target::Replan));
    assert!(app.offers(replan), "Change the plan does not ask first");
    // From the checklist too.
    app.press(Action::Backup(w));
    app.press(Action::BPlan);
    let _ = app.frame();
    assert_eq!(app.chart, Some((press, Target::Replan)));
    assert!(shown(&mut app, "Your changes on the chart are replaced"));
    app.press(Action::Chart(C::Replan(press)));
    assert_eq!(app.screen, Screen::Backup);
    let b = app.backup.as_ref().unwrap();
    assert_eq!(b.stage, BStage::Plan);
    assert!(b.answers.map.is_none());
}

#[test]
fn a_cosigners_key_takes_its_holders_name_and_a_date_kept_in_the_vault_alone() {
    let mut app = desktop();
    let w = two_of_three(&mut app);
    planned(&mut app, w, 0);
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
    assert_eq!(app.chart, Some((press, Target::Marked(n, None))));
    assert!(shown(&mut app, "Someone can now spend"));
    // Marked, the place's lines are struck through and stay on the chart.
    let (_, node) = node_of(&app, w, "Place 1");
    assert!(node.marks.iter().all(|m| *m == Some(Mark::Exposed)));
    let go = Action::Chart(C::MoveMoney(press));
    assert!(app.offers(go));
    app.press(go);
    assert_eq!(app.screen, Screen::Create);
    assert!(app.chart_work.moving.is_some());
    // The marked lines still on the card, marked.
    app.press(Action::OpenWallet(w));
    let texts = app.drawn_texts();
    assert!(texts.iter().any(|t| t.contains("exposed")), "{texts:?}");
}

#[test]
fn a_copy_marked_lost_says_whether_anything_is_lost_that_is_not_elsewhere() {
    let (mut app, w) = single_key_with_vault();
    let press = Press::Loaded(w);
    // The words at Place 1 lost: they are at Place 2 too.
    let (n, j) = line_of(&app, w, "Place 1", "Key 1 words");
    app.press(Action::Chart(C::Mark(press, n, Some(j), Mark::Lost)));
    let _ = app.frame();
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
    assert!(shown(
        &mut app,
        "The wallet can no longer be rebuilt from what is left"
    ));
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
    planned(&mut app, w, 1);
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
    assert!(shown(&mut app, "Checked 2026-10-03"));
}

#[test]
fn the_wallet_node_checks_its_addresses_and_a_cosigners_key_shows_its_xpub() {
    let mut app = desktop();
    let w = two_of_three(&mut app);
    planned(&mut app, w, 0);
    let press = Press::Loaded(w);
    open(&mut app, press, Target::Wallet);
    let check = Action::Chart(C::Do(press, Step::Addresses));
    assert!(app.offers(check));
    app.press(check);
    let _ = app.frame();
    assert_eq!(app.chart, Some((press, Target::Addresses)));
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
    planned(&mut app, w, 0);
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
    use faraday_core::plan::{Answers, Seed, Shape};
    let shape = Shape {
        m: 1,
        keys: 1,
        seeds: vec![Seed {
            name: "9A6A2580".to_string(),
            here: true,
            passphrase: false,
        }],
        splits: false,
    };
    let text = "seeds 1010\nplaces 2\nsplit 0\nwallet 1100\nsoftware 00001\nform 10\nomit 0\nvault 0 1\nsticks 0 01\n";
    let a = Answers::from_text(&shape, text).expect("the plan loads");
    assert!(a.map.is_none());
    assert_eq!(
        faraday_core::plan::check(&shape, &a).spend,
        Found::Yes,
        "a place with the words can spend"
    );
}

#[test]
fn in_an_open_vaults_view_a_change_is_kept_in_that_vault() {
    let (mut app, w) = single_key_with_vault();
    app.press(Action::Nav(Screen::VaultContents));
    app.press(Action::Vault(V::Category(1)));
    app.press(Action::Vault(V::Item(0)));
    let _ = app.frame();
    let r = app.vault_selected_index().expect("a wallet is chosen");
    let press = Press::Vault(0, r);
    let vault_chart = |app: &Faraday| faraday_core::glance::of_vault(app, 0, r).unwrap();
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
