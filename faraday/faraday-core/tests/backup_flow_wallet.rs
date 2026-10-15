//! The Bitcoin wallet's part of the guided backup flow
//! (`docs/BACKUP-FLOW.md` §10, §13): whether vaults are part of the backup
//! and how many, asked before the keys; each vault's stick brought in and
//! the vault opened or made; the keys made into memory; the plan in Back
//! up after them; the checklist's **Write to sticks**, and every stick
//! written inside the flow.

mod common;

use common::*;
use faraday_core::backup_flow::{BackupFlow, Step, Target};
use faraday_core::backup_flow_steps::FlowAction as F;
use faraday_core::create::NewKind;
use faraday_core::plan::{
    self, Answers, At, Form, Item, Preset, Row, Shape, Spot, Tag, What, form, seeds, wallet,
};
use faraday_core::testkit;
use faraday_core::{Action, BStage, Faraday, PlanPress, Screen, Sheet, bstep, cstep, qrow};
use faraday_vault::records::kind;
use osk_shell_api::{App, Event, Key};

const PHRASE: &str = "first vault phrase";
const PHRASE_2: &str = "second vault phrase";
const PHRASE_3: &str = "third vault phrase";

/// A row of the flow's Software card.
#[allow(dead_code)]
fn answer(shell: &mut Shell, app: &mut Faraday, list: u8, row: usize) {
    shell.flow(app, F::Answer(list, row as u8));
}

/// A press on the flow's own plan cards, Back up's cards in the flow.
fn flow_plan(shell: &mut Shell, app: &mut Faraday, press: PlanPress) {
    shell.flow(app, F::Plan(press));
}

/// The first place the flow's plan keeps vault `v`'s stick at.
fn flow_place_of(app: &Faraday, v: usize) -> usize {
    let wf = app.flow().unwrap().wallet.as_ref().unwrap();
    (0..wf.answers.places)
        .find(|&p| wf.answers.row_seeds(&wf.shape, p, Row::Vault(v)).is_some())
        .expect("the vault's stick has a place")
}

/// At place `p` of the flow's plan, row `row`'s sheet given exactly
/// `seeds`, and Done.
fn flow_keep(shell: &mut Shell, app: &mut Faraday, p: usize, row: Row, seeds: &[usize]) {
    flow_plan(shell, app, PlanPress::Row(p as u8, row.code()));
    let ticked = app
        .choosing
        .as_ref()
        .expect("the row's sheet")
        .seeds
        .clone();
    let n = app
        .flow()
        .unwrap()
        .wallet
        .as_ref()
        .unwrap()
        .shape
        .seeds
        .len();
    for i in 0..n {
        if ticked.contains(&i) != seeds.contains(&i) {
            shell.press(app, Action::BPick(i as u8));
        }
    }
    shell.press(app, Action::BPickDone);
}

/// Seed `i` put into vault `v` too, on the vault's sheet in the flow.
fn flow_vault_add(shell: &mut Shell, app: &mut Faraday, v: usize, i: usize) {
    let p = flow_place_of(app, v);
    let mut seeds = {
        let wf = app.flow().unwrap().wallet.as_ref().unwrap();
        wf.answers.vault_seeds(&wf.shape, v)
    };
    seeds.push(i);
    flow_keep(shell, app, p, Row::Vault(v), &seeds);
}

/// A row of Back up's Software card (the software, the form).
fn answer_backup(app: &mut Faraday, list: u8, row: usize) {
    app.press(Action::BAnswer(list, row as u8));
}

fn backup_places(app: &Faraday) -> usize {
    app.backup.as_ref().unwrap().answers.places
}

fn backup_shape(app: &Faraday) -> Shape {
    app.plan_shape(app.backup.as_ref().unwrap().wallet)
}

/// Row `row` unticked on every place's card of Back up's plan.
fn untick_everywhere(app: &mut Faraday, row: Row) {
    for p in 0..backup_places(app) {
        app.press(Action::BRowOff(p as u8, row.code()));
    }
}

/// The description's form `form` unticked at every place that keeps it.
fn form_off_everywhere(app: &mut Faraday, form: Form) {
    for p in 0..backup_places(app) {
        let shape = backup_shape(app);
        if app
            .backup
            .as_ref()
            .unwrap()
            .answers
            .form_at(&shape, p, form)
        {
            app.press(Action::BForm(p as u8, form.code()));
        }
    }
}

/// The description's form `form` ticked at place `p`.
fn form_on(app: &mut Faraday, p: usize, form: Form) {
    let shape = backup_shape(app);
    if !app
        .backup
        .as_ref()
        .unwrap()
        .answers
        .form_at(&shape, p, form)
    {
        app.press(Action::BForm(p as u8, form.code()));
    }
}

/// At place `p`, row `row` given exactly `seeds` on its sheet, and Done.
fn keep_at(app: &mut Faraday, p: usize, row: Row, seeds: &[usize]) {
    app.press(Action::BRow(p as u8, row.code()));
    let ticked = app
        .choosing
        .as_ref()
        .expect("the row's sheet")
        .seeds
        .clone();
    for i in 0..backup_shape(app).seeds.len() {
        if ticked.contains(&i) != seeds.contains(&i) {
            app.press(Action::BPick(i as u8));
        }
    }
    app.press(Action::BPickDone);
}

/// Whether Back up's plan offers `action` anywhere down its column.
fn offered_down(app: &mut Faraday, action: Action) -> bool {
    for _ in 0..20 {
        if offered(app, action) {
            return true;
        }
        app.event(Event::Scroll {
            x: 400,
            y: 384,
            dy: 300,
        });
    }
    false
}

/// The first place that keeps vault `v`'s stick.
fn vault_place(app: &Faraday, v: usize) -> usize {
    let shape = backup_shape(app);
    let a = &app.backup.as_ref().unwrap().answers;
    (0..a.places)
        .find(|&p| a.row_seeds(&shape, p, Row::Vault(v)).is_some())
        .expect("the vault's stick has a place")
}

/// Vault `v`'s stick kept at place `p` too, the vault as it is.
fn stick_also_at(app: &mut Faraday, p: usize, v: usize) {
    app.press(Action::BRow(p as u8, Row::Vault(v).code()));
    app.press(Action::BPickDone);
}

fn step(app: &Faraday) -> Step {
    app.flow().expect("a flow").step
}

/// `n` empty sticks of different serials.
fn sticks(shell: &mut Shell, n: usize) {
    for k in 1..=n {
        let mut d = Disk::empty(&format!("S{k}"), "KEYSTICK");
        d.serial = Some(k as u32);
        shell.add(d);
    }
}

fn key_in(app: &mut Faraday, n: usize) -> [u8; 4] {
    app.press(Action::Entry(None));
    for c in testkit::test_words(testkit::TEST_SEEDS[n].0).chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    app.press(Action::EntryAdd);
    app.session.keys.last().unwrap().master.fingerprint().0
}

fn multi_index() -> u8 {
    NewKind::ALL
        .iter()
        .position(|k| *k == NewKind::Multi)
        .unwrap() as u8
}

/// Make a new wallet, with `count` vaults to set up before the keys;
/// `multi` for the 2-of-3.
fn create_with_vaults(app: &mut Faraday, multi: bool, count: usize) {
    app.press(Action::CreateWallet);
    if multi {
        app.press(Action::CKind(multi_index()));
    }
    app.press(Action::CVaults(true));
    app.press(Action::CNext(cstep::WHERE));
    app.press(Action::CVaultN(count as u16));
    app.press(Action::CNext(cstep::HOW_MANY));
    assert_eq!(app.screen, Screen::BackupFlow);
    assert_eq!(step(app), Step::Sticks);
}

/// The sticks of the setup, in order, each one at a time.
fn bring_in_all(shell: &mut Shell, app: &mut Faraday, disks: &[&str]) {
    for d in disks {
        shell.plug(app, d);
        shell.flow(app, F::Tap);
        shell.pull(app, d);
    }
}

/// Saves into the open vaults what the plan puts in each, by the
/// checklist's vault items.
fn save_vaults(app: &mut Faraday) {
    let vaults: Vec<usize> = app
        .backup_items()
        .into_iter()
        .filter_map(|n| match bstep::item(n) {
            Some(Item::Vault(v)) => Some(v),
            _ => None,
        })
        .collect();
    for v in vaults {
        app.press(Action::BVaultSave(v as u8));
        assert!(
            app.backup_item_done(Item::Vault(v)),
            "Vault {} saved",
            v + 1
        );
    }
}

fn restart(shell: &mut Shell, app: &mut Faraday) -> Faraday {
    shell.pump(app);
    assert!(app.restart_requested(), "locked");
    shell.restart()
}

fn no_visit(shell: &Shell) {
    assert!(
        shell.screens.iter().all(|s| *s != Screen::Visit),
        "the stick visit page is never shown"
    );
    assert!(shell.sheets.iter().all(|s| *s != Sheet::WriteOut));
    assert!(shell.sheets.iter().all(|s| *s != Sheet::Lock));
}

fn ofv(shell: &Shell, d: &str) -> Option<Vec<u8>> {
    shell
        .disk(d)
        .files
        .iter()
        .find(|(n, _)| n.ends_with(".ofv"))
        .map(|(_, b)| b.clone())
}

fn names(shell: &Shell, d: &str) -> Vec<String> {
    shell.disk(d).files.iter().map(|f| f.0.clone()).collect()
}

/// A single key whose Vault 1 is on two sticks (S1, S2) and the
/// descriptor on a stick of files (S3): made in Create, stopped on the
/// checklist's Write to sticks.
fn single_key_ready(shell: &mut Shell) -> (Faraday, usize, [u8; 4]) {
    single_key(shell, false)
}

/// A key made by New key for slot 0 of the Create open: coin flips, its
/// words locked in. The wallet is made with it.
fn new_key(app: &mut Faraday) -> [u8; 4] {
    app.press(Action::KeyGen(Some(0)));
    app.press(Action::KWords(12));
    app.press(Action::KWay(faraday_core::keygen::Way::Coins.index()));
    app.press(Action::KNext);
    for i in 0..128 {
        app.press(Action::KFlip((i * 5 + i / 3) % 2 == 0));
    }
    app.press(Action::KNext);
    app.press(Action::KLock);
    let fp = app.session.keys.last().unwrap().master.fingerprint().0;
    app.press(Action::KNext);
    fp
}

/// [`single_key_ready`], with the key made by New key when `made_here`.
fn single_key(shell: &mut Shell, made_here: bool) -> (Faraday, usize, [u8; 4]) {
    single_key_with(shell, made_here, true)
}

/// [`single_key`], with the wallet description in the vault or not.
fn single_key_with(
    shell: &mut Shell,
    made_here: bool,
    description_in_vault: bool,
) -> (Faraday, usize, [u8; 4]) {
    sticks(shell, 3);
    let mut app = process(Vec::new(), Vec::new(), Vec::new());
    keep_time(&mut app);
    create_with_vaults(&mut app, false, 1);
    bring_in_all(shell, &mut app, &["S1"]);
    assert_eq!(step(&app), Step::Vaults);
    create_vault(shell, &mut app, 0, PHRASE);
    // The vault is set up: on to the keys, in Make a new wallet.
    assert_eq!(app.screen, Screen::Create, "{:?}", app.vaults.unlock_error);
    let fp = if made_here {
        app.press(Action::Nav(Screen::Create));
        new_key(&mut app)
    } else {
        let fp = key_in(&mut app, 1);
        app.press(Action::Nav(Screen::Create));
        app.press(Action::CSlotHere(0, fp));
        fp
    };
    assert_eq!(
        app.vaults.open[0].contents.of(kind::KEY).count(),
        0,
        "no key is in a vault until the plan says which"
    );
    if app.create.as_ref().unwrap().built.is_none() {
        app.press(Action::CNext(cstep::KEYS));
    }
    let w = app.create.as_ref().unwrap().built.expect("wallet made");
    app.press(Action::CNext(cstep::CHECK));
    app.press(Action::CBackup(1));
    assert_eq!(app.screen, Screen::Backup);
    // The seed in Vault 1 only, its stick at both places, the wallet
    // description as files at Place 2.
    untick_everywhere(&mut app, Row::Words);
    stick_also_at(&mut app, 0, 0);
    form_off_everywhere(&mut app, Form::Paper);
    form_on(&mut app, 1, Form::File);
    if !description_in_vault {
        form_off_everywhere(&mut app, Form::Vault(0));
    }
    answer_backup(&mut app, qrow::FORM, form::QR);
    answer_backup(&mut app, qrow::FORM, form::TEXT);
    app.press(Action::BChecklist);
    assert_eq!(app.backup.as_ref().unwrap().stage, BStage::Checklist);
    assert_eq!(
        app.backup_items(),
        vec![bstep::VAULT, bstep::PUBLIC, bstep::ENVELOPE, bstep::STICKS]
    );
    {
        // The flow follows the plan: the stick read for Vault 1 is at its
        // first place, the second place waits for a stick.
        let f = app.backup_flows.last().unwrap();
        assert_eq!(f.plan.places, vec![vec![0], vec![0]]);
        assert_eq!(f.plan.place_files, vec![1], "Place 2's stick of files");
        let targets: Vec<Target> = f.slots.iter().map(|s| s.target).collect();
        assert_eq!(
            targets,
            vec![Target::Place(0), Target::Place(1), Target::PlaceFiles(1)]
        );
        assert!(f.slots[0].bound.is_some(), "the stick read in the setup");
        assert!(
            f.slots[1].bound.is_none(),
            "a new stick for the second copy"
        );
    }
    assert!(!app.backup_item_done(Item::Vault(0)));
    save_vaults(&mut app);
    assert_eq!(
        app.vaults.open[0].contents.of(kind::KEY).count(),
        1,
        "the key is in Vault 1 once the checklist saves it"
    );
    assert_eq!(
        app.vaults.open[0].contents.of(kind::WALLET).count(),
        usize::from(description_in_vault),
        "the wallet description is in the vault where the plan puts it"
    );
    app.press(Action::BFilesMake);
    app.press(Action::BNext(bstep::ENVELOPE));
    assert!(!app.backup_item_done(Item::Sticks));
    (app, w, fp)
}

#[test]
fn a_single_key_with_a_vault_on_two_sticks_and_the_descriptor_on_a_stick_of_files_is_written_inside_the_flow()
 {
    let mut shell = Shell::new();
    let (mut app, _, _) = single_key_ready(&mut shell);
    // One lock, then each stick by place: the visit is never shown.
    shell.flow(&mut app, F::WalletLock);
    let mut app = restart(&mut shell, &mut app);
    assert_eq!(app.screen, Screen::BackupFlow);
    assert_eq!(step(&app), Step::Write);
    assert!(!app.holds_secret(), "a stick meets a clean process");
    // Each stick ticks on its own write.
    shell.plug(&mut app, "S1");
    assert!(app.flow().unwrap().slots[0].written.is_some());
    assert!(app.flow().unwrap().slots[1].written.is_none());
    shell.pull(&mut app, "S1");
    shell.plug(&mut app, "S2");
    // A new stick, checked when it is inserted.
    shell.flow(&mut app, F::Tap);
    assert!(
        app.flow().unwrap().live.reading.as_ref().unwrap().hold,
        "a vault's second copy needs the hold"
    );
    assert!(app.flow().unwrap().slots[1].written.is_none());
    shell.flow(&mut app, F::Hold);
    assert!(app.flow().unwrap().slots[1].written.is_some());
    shell.pull(&mut app, "S2");
    assert_eq!(step(&app), Step::Write);
    assert!(app.flow().unwrap().slots[2].written.is_none());
    shell.plug(&mut app, "S3");
    shell.flow(&mut app, F::Tap);
    shell.pull(&mut app, "S3");
    assert_eq!(step(&app), Step::Done);
    no_visit(&shell);
    assert!(ofv(&shell, "S1").is_some(), "{:?}", names(&shell, "S1"));
    assert_eq!(
        ofv(&shell, "S1"),
        ofv(&shell, "S2"),
        "the same sealed bytes"
    );
    assert!(
        names(&shell, "S3")
            .iter()
            .any(|n| n.ends_with("-descriptor.txt")),
        "the descriptor is on the stick of files: {:?}",
        names(&shell, "S3")
    );
    assert!(
        ofv(&shell, "S3").is_none(),
        "no vault copy on the stick of files"
    );
    // After the checklist nothing waits For the stick.
    assert!(
        app.outbox.is_empty(),
        "{:?}",
        app.outbox.iter().map(|i| &i.name).collect::<Vec<_>>()
    );
}

#[test]
fn the_hold_on_the_key_stays_until_the_last_stick_is_written_and_a_lock_resumes_at_the_next_stick()
{
    let mut shell = Shell::new();
    let (mut app, _, fp) = single_key(&mut shell, true);
    let fp = osk_bip::keys::Fingerprint(fp);
    assert!(app.key_held(fp), "made here, its backup not done");
    shell.flow(&mut app, F::WalletLock);
    let mut app = restart(&mut shell, &mut app);
    // The copies sealed say the backup is done; the flow holds the key.
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(faraday_core::vaults::VaultAction::Open(0)));
    app.vaults.ms_per_unit = Some(180);
    unlock(&mut app, PHRASE);
    app.press(Action::Vault(
        faraday_core::vaults::VaultAction::LoadChosen(0),
    ));
    assert!(
        app.session
            .keys
            .iter()
            .any(|k| k.master.fingerprint() == fp),
        "the vault loaded the key"
    );
    assert!(app.key_held(fp), "held while the sticks are not written");
    let pending = app.vaults.open[0].contents.of(kind::KEY).any(|(_, r)| {
        r.field(faraday_vault::records::field::KEY_FLAGS)
            .and_then(|f| f.first())
            .is_some_and(|b| b & faraday_vault::records::BACKUP_PENDING != 0)
    });
    assert!(!pending, "the sealed copy says the backup is done");
    // A lock mid-sequence resumes at the right stick.
    let mut app = shell.lock(&mut app);
    assert_eq!(step(&app), Step::Write);
    shell.plug(&mut app, "S1");
    shell.pull(&mut app, "S1");
    let mut app = shell.lock(&mut app);
    assert_eq!(step(&app), Step::Write);
    assert!(app.flow().unwrap().slots[0].written.is_some());
    assert!(app.flow().unwrap().slots[1].written.is_none());
    expect(&mut app, "Insert the stick for Vault 1 (Place 2)");
    shell.plug(&mut app, "S2");
    shell.flow(&mut app, F::Tap);
    shell.flow(&mut app, F::Hold);
    shell.pull(&mut app, "S2");
    assert!(app.flow().unwrap().waiting());
    shell.plug(&mut app, "S3");
    shell.flow(&mut app, F::Tap);
    shell.pull(&mut app, "S3");
    assert_eq!(step(&app), Step::Done);
    assert!(app.flow().unwrap().all_written());
    assert!(
        app.held_keys().is_empty(),
        "the hold is lifted once every stick is written"
    );
    no_visit(&shell);
}

fn spots_two_of_three_with_key_3_on_paper() -> (Shape, Answers) {
    let seed = |n: &str| plan::Seed {
        name: n.into(),
        here: true,
        passphrase: false,
    };
    let shape = Shape {
        m: 2,
        keys: 3,
        seeds: vec![seed("a"), seed("b"), seed("c")],
        splits: true,
        vaults: 0,
    };
    let mut a = Answers::preset(&shape, Preset::PaperVault);
    a.set_map(vec![
        Spot {
            at: At::Place(0),
            holds: vec![(What::VaultStick(0), Tag::Sealed)],
        },
        Spot {
            at: At::Place(1),
            holds: vec![(What::VaultStick(1), Tag::Sealed)],
        },
        Spot {
            at: At::Place(2),
            holds: vec![(What::Words(2), Tag::Secret)],
        },
        Spot {
            at: At::Vault(0),
            holds: vec![(What::Seed(0), Tag::Sealed), (What::Wallet, Tag::Sealed)],
        },
        Spot {
            at: At::Vault(1),
            holds: vec![(What::Seed(1), Tag::Sealed), (What::Wallet, Tag::Sealed)],
        },
    ]);
    (shape, a)
}

#[test]
fn a_paper_place_has_no_stick_and_a_plan_with_no_stick_has_no_stick_step() {
    let (shape, a) = spots_two_of_three_with_key_3_on_paper();
    let f = BackupFlow::new_wallet(shape.clone(), a.clone(), "w");
    let targets: Vec<Target> = f.slots.iter().map(|s| s.target).collect();
    assert_eq!(
        targets,
        vec![Target::Place(0), Target::Place(1)],
        "two sticks only"
    );
    assert!(plan::checklist(&shape, &a).contains(&Item::Sticks));
    // Paper only: no vault, no sheet, no files.
    let mut paper = Answers::preset(&shape, Preset::Paper);
    paper.toggle(&shape, plan::Question::Wallet, wallet::PAPER);
    let f = BackupFlow::new_wallet(shape.clone(), paper.clone(), "w");
    assert!(f.slots.is_empty(), "no stick");
    assert!(
        !plan::checklist(&shape, &paper).contains(&Item::Sticks),
        "no lock"
    );
}

fn one_key_shape() -> Shape {
    Shape {
        m: 1,
        keys: 1,
        seeds: vec![plan::Seed {
            name: "a".into(),
            here: true,
            passphrase: false,
        }],
        splits: false,
        vaults: 0,
    }
}

#[test]
fn a_plan_may_have_any_number_of_places_and_one_place_says_so() {
    let shape = one_key_shape();
    let mut a = Answers::preset(&shape, Preset::PaperVault);
    for n in [1usize, 7, 40] {
        a.set_places(&shape, n);
        assert_eq!(a.places, n, "no cap on places");
    }
    // A single place keeps every secret: the plan says so, and that is all.
    a.set_places(&shape, 1);
    assert_eq!(plan::all_at_one_place(&shape, &a), Some(0));
    assert_eq!(plan::check(&shape, &a).one_place, Some(0));
    assert!(plan::check(&shape, &a).passphrase_apart, "no passphrase");
}

#[test]
fn the_flow_follows_any_number_of_places() {
    let mut shell = Shell::new();
    let (mut app, _, _) = single_key_ready(&mut shell);
    // Back up's places card: any number, and the flow follows it.
    app.press(Action::BPlan);
    while backup_places(&app) < 8 {
        app.press(Action::BPlaces(true));
    }
    let f = app.backup_flows.last().unwrap();
    assert_eq!(f.wallet.as_ref().unwrap().answers.places, 8);
    assert_eq!(f.plan.places.len(), 8);
}

/// A 2-of-3 with three vaults set up before the keys: Vault 1 an existing
/// vault on S1 (beside another vault), Vaults 2 and 3 new on S2 and S3;
/// one key in each, by the plan; stopped on the checklist's Write to
/// sticks. `software` chooses Sparrow's files too.
fn two_of_three_ready(shell: &mut Shell, software: bool) -> Faraday {
    sticks(shell, 4);
    let main = vault("Main", &[PHRASE], 0x41);
    let other = vault("Other", &[PHRASE_2], 0x42);
    {
        let d = shell.disk_mut("S1");
        d.files = vec![("main.ofv".into(), main), ("main-2.ofv".into(), other)];
    }
    let mut app = process(Vec::new(), Vec::new(), Vec::new());
    keep_time(&mut app);
    create_with_vaults(&mut app, true, 3);
    {
        let f = app.flow().unwrap();
        assert_eq!(f.plan.places, vec![vec![0], vec![1], vec![2]]);
        assert!(f.is_setup());
    }
    // Vault 1 is on S1 beside another vault: which of them?
    shell.plug(&mut app, "S1");
    expect(&mut app, "Which vault is Vault 1?");
    let k = app
        .flow()
        .unwrap()
        .live
        .reading
        .as_ref()
        .unwrap()
        .files
        .iter()
        .position(|(f, _)| f.name == "main.ofv")
        .unwrap();
    shell.flow(&mut app, F::PickFile(0, k));
    shell.flow(&mut app, F::Tap);
    shell.pull(&mut app, "S1");
    for d in ["S2", "S3"] {
        shell.plug(&mut app, d);
        shell.flow(&mut app, F::Tap);
        shell.pull(&mut app, d);
    }
    assert_eq!(step(&app), Step::Vaults);
    unlock(&mut app, PHRASE);
    create_vault(shell, &mut app, 1, PHRASE_2);
    create_vault(shell, &mut app, 2, PHRASE_3);
    assert_eq!(app.screen, Screen::Create, "{:?}", app.vaults.unlock_error);
    let fps: Vec<[u8; 4]> = (0..3).map(|i| key_in(&mut app, i)).collect();
    app.press(Action::Nav(Screen::Create));
    for (i, fp) in fps.iter().enumerate() {
        app.press(Action::CSlotHere(i as u8, *fp));
    }
    app.press(Action::CNext(cstep::KEYS));
    assert!(app.create.as_ref().unwrap().built.is_some(), "wallet made");
    // Three vaults are open and hold no key yet.
    assert_eq!(app.vaults.open.len(), 3);
    for o in &app.vaults.open {
        assert_eq!(o.contents.of(kind::KEY).count(), 0);
    }
    app.press(Action::CNext(cstep::CHECK));
    app.press(Action::CBackup(if software { 2 } else { 1 }));
    untick_everywhere(&mut app, Row::Words);
    if software {
        answer_backup(&mut app, qrow::SOFTWARE, plan::software::SPARROW);
        answer_backup(&mut app, qrow::SOFTWARE, plan::software::NOT_SURE);
        answer_backup(&mut app, qrow::FORM, form::TEXT);
        answer_backup(&mut app, qrow::FORM, form::QR);
    }
    app.press(Action::BChecklist);
    // One key in each vault, by the plan.
    save_vaults(&mut app);
    for o in &app.vaults.open {
        assert_eq!(o.contents.of(kind::KEY).count(), 1);
    }
    if software {
        app.press(Action::BFilesMake);
    }
    app.press(Action::BOut(3));
    app.press(Action::BNext(bstep::ENVELOPE));
    app
}

#[test]
fn a_two_of_three_with_an_existing_vault_and_two_new_ones_writes_each_stick_by_place() {
    let mut shell = Shell::new();
    let mut app = two_of_three_ready(&mut shell, false);
    let other = shell.disk("S1").file("main-2.ofv").cloned().unwrap();
    let before = shell.disk("S1").file("main.ofv").cloned().unwrap();
    assert_eq!(
        app.backup_items(),
        vec![
            bstep::VAULT,
            bstep::VAULT + 1,
            bstep::VAULT + 2,
            bstep::SHEETS,
            bstep::ENVELOPE,
            bstep::STICKS
        ]
    );
    shell.flow(&mut app, F::WalletLock);
    let mut app = restart(&mut shell, &mut app);
    assert_eq!(step(&app), Step::Write);
    let targets: Vec<Target> = app.flow().unwrap().slots.iter().map(|s| s.target).collect();
    assert_eq!(
        targets,
        vec![
            Target::Place(0),
            Target::Place(1),
            Target::Place(2),
            Target::Public
        ]
    );
    for d in ["S1", "S2", "S3"] {
        shell.plug(&mut app, d);
        shell.pull(&mut app, d);
    }
    shell.plug(&mut app, "S4");
    shell.flow(&mut app, F::Tap);
    shell.pull(&mut app, "S4");
    assert_eq!(step(&app), Step::Done);
    no_visit(&shell);
    // The other vault on S1 is what it was; the chosen one is replaced.
    assert_eq!(shell.disk("S1").file("main-2.ofv"), Some(&other));
    assert_ne!(shell.disk("S1").file("main.ofv"), Some(&before));
    // Three different vaults, one on each; the sheet on the stick of
    // public files, which holds no vault.
    let v1 = shell.disk("S1").file("main.ofv").cloned();
    let v2 = ofv(&shell, "S2");
    let v3 = ofv(&shell, "S3");
    assert!(v1.is_some() && v2.is_some() && v3.is_some());
    assert_ne!(v1, v2);
    assert_ne!(v2, v3);
    assert!(ofv(&shell, "S4").is_none());
    assert!(
        names(&shell, "S4")
            .iter()
            .any(|n| n.ends_with("-backup.pdf")),
        "{:?}",
        names(&shell, "S4")
    );
}

#[test]
fn the_account_keys_and_their_records_are_written_to_the_stick_of_public_files() {
    let mut shell = Shell::new();
    let mut app = two_of_three_ready(&mut shell, true);
    shell.flow(&mut app, F::WalletLock);
    let mut app = restart(&mut shell, &mut app);
    for d in ["S1", "S2", "S3"] {
        shell.plug(&mut app, d);
        shell.pull(&mut app, d);
    }
    shell.plug(&mut app, "S4");
    shell.flow(&mut app, F::Tap);
    shell.pull(&mut app, "S4");
    assert_eq!(step(&app), Step::Done);
    let n = names(&shell, "S4");
    let bsms = n
        .iter()
        .filter(|x| x.starts_with("xpub-") && x.ends_with("-bsms.txt"))
        .count();
    let keys = n
        .iter()
        .filter(|x| x.starts_with("xpub-") && x.ends_with(".txt") && !x.ends_with("-bsms.txt"))
        .count();
    assert!(n.iter().any(|x| x.ends_with("-wallet.json")), "{n:?}");
    assert_eq!(keys, 3, "{n:?}");
    assert_eq!(bsms, 3, "{n:?}");
    assert!(n.iter().any(|x| x.ends_with("-backup.pdf")), "{n:?}");
    for d in ["S1", "S2", "S3"] {
        assert!(
            !names(&shell, d).iter().any(|x| x.starts_with("xpub-")),
            "no account key on a vault's stick"
        );
    }
    assert!(
        ofv(&shell, "S4").is_none(),
        "no vault copy on the public stick"
    );
}

/// Key `slot` of the Create open, made by New key (coin flips that differ
/// from slot to slot).
fn new_key_at(app: &mut Faraday, slot: u8) -> [u8; 4] {
    app.press(Action::KeyGen(Some(slot)));
    app.press(Action::KWords(12));
    app.press(Action::KWay(faraday_core::keygen::Way::Coins.index()));
    app.press(Action::KNext);
    let s = usize::from(slot);
    for i in 0..128usize {
        app.press(Action::KFlip((i * (5 + 2 * s) + i / (3 + s)) % 2 == 0));
    }
    app.press(Action::KNext);
    app.press(Action::KLock);
    let fp = app.session.keys.last().unwrap().master.fingerprint().0;
    app.press(Action::KNext);
    fp
}

#[test]
fn a_two_of_three_with_key_3_on_paper_asks_for_two_sticks_and_holds_key_3_until_its_copy_is_checked()
 {
    let mut shell = Shell::new();
    sticks(&mut shell, 2);
    let mut app = process(Vec::new(), Vec::new(), Vec::new());
    keep_time(&mut app);
    // Two vaults for three keys: Key 3 is kept on paper.
    create_with_vaults(&mut app, true, 2);
    bring_in_all(&mut shell, &mut app, &["S1", "S2"]);
    create_vault(&mut shell, &mut app, 0, PHRASE);
    create_vault(&mut shell, &mut app, 1, PHRASE_2);
    assert_eq!(app.screen, Screen::Create, "{:?}", app.vaults.unlock_error);
    app.press(Action::Nav(Screen::Create));
    let fps: Vec<[u8; 4]> = (0..3u8).map(|s| new_key_at(&mut app, s)).collect();
    app.press(Action::CNext(cstep::KEYS));
    assert!(app.create.as_ref().unwrap().built.is_some(), "wallet made");
    assert_eq!(app.vaults.open.len(), 2);
    app.press(Action::CNext(cstep::CHECK));
    app.press(Action::CBackup(1));
    // Key 3 on paper at Place 3, the vaults' sticks at Places 1 and 2.
    {
        let b = app.backup.as_mut().unwrap();
        let mut a = b.answers.clone();
        let spots = vec![
            Spot {
                at: At::Place(0),
                holds: vec![(What::VaultStick(0), Tag::Sealed)],
            },
            Spot {
                at: At::Place(1),
                holds: vec![(What::VaultStick(1), Tag::Sealed)],
            },
            Spot {
                at: At::Place(2),
                holds: vec![(What::Words(2), Tag::Secret)],
            },
            Spot {
                at: At::Vault(0),
                holds: vec![(What::Seed(0), Tag::Sealed), (What::Wallet, Tag::Sealed)],
            },
            Spot {
                at: At::Vault(1),
                holds: vec![(What::Seed(1), Tag::Sealed), (What::Wallet, Tag::Sealed)],
            },
        ];
        a.set_map(spots);
        b.answers = a;
    }
    app.press(Action::BChecklist);
    assert_eq!(
        app.backup_items(),
        vec![
            bstep::BLANK,
            bstep::COPY + 2,
            bstep::VAULT,
            bstep::VAULT + 1,
            bstep::ENVELOPE,
            bstep::STICKS
        ]
    );
    // Keys 1 and 2 go into the vaults set up for them; Key 3 into none.
    save_vaults(&mut app);
    for o in &app.vaults.open {
        assert_eq!(o.contents.of(kind::KEY).count(), 1);
    }
    app.press(Action::BNext(bstep::BLANK));
    // Key 3 is held until its copy is checked; Keys 1 and 2 until their
    // sticks are written.
    let held = |app: &Faraday, k: usize| app.key_held(osk_bip::keys::Fingerprint(fps[k]));
    assert!(held(&app, 2), "Key 3 waits for its copy");
    let key3 = app
        .session
        .keys
        .iter()
        .find(|k| k.master.fingerprint().0 == fps[2])
        .unwrap();
    let words = key3.words.as_ref().unwrap().to_string();
    let lang = key3.language;
    let digits =
        osk_codec::seedqr::to_digits(&osk_bip::bip39::Mnemonic::parse(lang, &words).unwrap());
    app.press(Action::BStep(bstep::COPY + 2));
    app.press(Action::BCheck);
    type_text(
        &mut app,
        std::str::from_utf8(digits.expose().as_bytes()).unwrap(),
    );
    assert!(
        app.backup_item_done(Item::Copy(2)),
        "the typed check matched"
    );
    app.press(Action::BNext(bstep::COPY + 2));
    assert!(
        held(&app, 0) && held(&app, 1),
        "Keys 1 and 2 wait for their sticks"
    );
    app.press(Action::BNext(bstep::ENVELOPE));
    assert!(!held(&app, 2), "Key 3's copy is checked: it signs from now");
    shell.flow(&mut app, F::WalletLock);
    let mut app = restart(&mut shell, &mut app);
    let targets: Vec<Target> = app.flow().unwrap().slots.iter().map(|s| s.target).collect();
    assert_eq!(
        targets,
        vec![Target::Place(0), Target::Place(1)],
        "two sticks only: Place 3 keeps no stick"
    );
    for d in ["S1", "S2"] {
        shell.plug(&mut app, d);
        shell.pull(&mut app, d);
    }
    assert_eq!(step(&app), Step::Done);
    no_visit(&shell);
    assert!(ofv(&shell, "S1").is_some() && ofv(&shell, "S2").is_some());
    assert_ne!(ofv(&shell, "S1"), ofv(&shell, "S2"));
}

fn shape_with(passphrase: bool, seeds_n: usize) -> Shape {
    Shape {
        m: seeds_n.min(2),
        keys: seeds_n,
        seeds: (0..seeds_n)
            .map(|i| plan::Seed {
                name: format!("k{i}"),
                here: true,
                passphrase: passphrase && i == 0,
            })
            .collect(),
        splits: seeds_n > 1,
        vaults: 0,
    }
}

#[test]
fn the_plan_says_when_every_secret_is_at_one_place() {
    // A single key, kept at one place: it is the place.
    let shape = shape_with(false, 1);
    let mut a = Answers::preset(&shape, Preset::Paper);
    a.set_places(&shape, 1);
    assert_eq!(plan::all_at_one_place(&shape, &a), Some(0));
    // A single key's words at two places: still every secret at Place 1.
    let a = Answers::preset(&shape, Preset::Paper);
    assert_eq!(plan::all_at_one_place(&shape, &a), Some(0));
    // A 2-of-3 with a seed at each place: no place has every secret.
    let shape3 = shape_with(false, 3);
    let a3 = Answers::preset(&shape3, Preset::Paper);
    assert_eq!(plan::all_at_one_place(&shape3, &a3), None);
    // A seed with its passphrase at one place shows it, wherever the other
    // seeds are.
    let shape3p = shape_with(true, 3);
    let mut a = Answers::preset(&shape3p, Preset::Paper);
    assert_eq!(
        plan::all_at_one_place(&shape3p, &a),
        None,
        "apart by default"
    );
    // The passphrase moves to its seed's place.
    a.pass = vec![vec![true, false, false, false], Vec::new(), Vec::new()];
    assert_eq!(plan::all_at_one_place(&shape3p, &a), Some(0));
}

#[test]
fn a_passphrase_has_a_place_that_keeps_no_copy_of_its_seed() {
    let shape = shape_with(true, 1);
    for preset in Preset::ALL {
        let a = Answers::preset(&shape, preset);
        let place = (0..a.places).find(|&p| a.pass_at(0, p)).expect("a place");
        assert!(
            !a.seed_at(&shape, 0, place),
            "{preset:?}: Place {} keeps a copy of the seed",
            place + 1
        );
        assert_eq!(plan::all_at_one_place(&shape, &a), None, "{preset:?}");
    }
    // Paper and the vault: the passphrase is at a third place.
    let a = Answers::preset(&shape, Preset::PaperVault);
    assert_eq!(a.places, 3);
    assert!(a.pass_at(0, 2));
    // The plan reads back with that place.
    assert_eq!(Answers::from_text(&shape, &a.to_text()), Some(a));
}

#[test]
fn the_lines_are_drawn_and_add_a_passphrase_opens_the_key_with_its_fields() {
    let mut shell = Shell::new();
    let (mut app, _, _) = single_key_ready(&mut shell);
    app.press(Action::BPlan);
    // A single key, no passphrase: both lines, under the checks.
    expect_part(&mut app, "A passphrase kept at another place than");
    expect_part(&mut app, "the seed is recommended");
    assert!(offered(&mut app, Action::PlanAddPass));
    shell.press(&mut app, Action::PlanAddPass);
    // Create opens on Keys with the key in its slot and the fields open:
    // the key with a passphrase is another key.
    assert_eq!(app.screen, Screen::Create);
    let c = app.create.as_ref().unwrap();
    assert_eq!(c.open, Some(cstep::KEYS));
    assert_eq!(c.pass_slot, Some(0));
}

#[test]
fn a_one_place_plan_shows_the_warning_and_blocks_nothing() {
    let mut shell = Shell::new();
    let (mut app, _, _) = single_key_ready(&mut shell);
    app.press(Action::BPlan);
    while backup_places(&app) > 1 {
        app.press(Action::BPlaces(false));
    }
    assert_eq!(app.backup_flows.last().unwrap().plan.places.len(), 1);
    expect_part(&mut app, "Every secret of this wallet is at Place 1");
    expect_part(&mut app, "Separate places are strongly");
    app.press(Action::BQ(faraday_core::qstep::PLAN));
    assert!(
        offered_down(&mut app, Action::BChecklist),
        "nothing is blocked"
    );
    app.press(Action::BChecklist);
    assert_eq!(app.backup.as_ref().unwrap().stage, BStage::Checklist);
}

#[test]
fn restore_takes_the_same_steps_and_a_restored_key_signs_at_once() {
    let mut shell = Shell::new();
    sticks(&mut shell, 2);
    let mut app = process(Vec::new(), Vec::new(), Vec::new());
    keep_time(&mut app);
    app.press(Action::RestoreWallet);
    app.press(Action::RVaults(true));
    app.press(Action::RNext(faraday_core::rstep::WHERE));
    app.press(Action::RVaultN(1));
    app.press(Action::RNext(faraday_core::rstep::HOW_MANY));
    assert_eq!(app.screen, Screen::BackupFlow);
    assert_eq!(step(&app), Step::Sticks);
    bring_in_all(&mut shell, &mut app, &["S1"]);
    create_vault(&mut shell, &mut app, 0, PHRASE);
    // The vault is set up: back in Restore, on its seeds.
    assert_eq!(app.screen, Screen::Restore, "{:?}", app.vaults.unlock_error);
    assert_eq!(
        app.restore.as_ref().unwrap().open,
        Some(faraday_core::rstep::SEEDS)
    );
    app.press(Action::RSlotWords);
    type_text(&mut app, &testkit::test_words(testkit::TEST_SEEDS[1].0));
    app.press(Action::EntryAdd);
    assert_eq!(app.screen, Screen::Restore);
    app.press(Action::Seeds(faraday_core::seeds::SeedsAction::Make));
    let w = app.restore.as_ref().unwrap().wallet.expect("wallet made");
    // The seed typed is in no vault until the plan says so.
    assert_eq!(app.vaults.open[0].contents.of(kind::KEY).count(), 0);
    app.press(Action::Backup(w));
    app.press(Action::BPreset(1));
    untick_everywhere(&mut app, Row::Words);
    app.press(Action::BChecklist);
    save_vaults(&mut app);
    assert_eq!(app.vaults.open[0].contents.of(kind::KEY).count(), 1);
    assert_eq!(app.vaults.open[0].contents.of(kind::WALLET).count(), 1);
    app.press(Action::BOut(3));
    app.press(Action::BNext(bstep::ENVELOPE));
    assert!(app.held_keys().is_empty(), "a restored key signs at once");
    shell.flow(&mut app, F::WalletLock);
    let mut app = restart(&mut shell, &mut app);
    assert_eq!(step(&app), Step::Write);
    write_sticks(&mut shell, &mut app, &["S1", "S2"]);
    assert_eq!(step(&app), Step::Done);
    assert!(app.held_keys().is_empty());
    no_visit(&shell);
}

#[test]
fn changing_the_plan_after_the_lock_runs_the_round_and_writes_the_vault_again() {
    let mut shell = Shell::new();
    let mut app = two_of_three_ready(&mut shell, false);
    shell.flow(&mut app, F::WalletLock);
    let mut app = restart(&mut shell, &mut app);
    assert_eq!(step(&app), Step::Write);
    // Seed 1 is to be in Vault 2 as well. Nothing is written yet.
    shell.flow(&mut app, F::ChangePlan);
    flow_vault_add(&mut shell, &mut app, 1, 0);
    shell.flow(&mut app, F::Continue);
    {
        let f = app.flow().unwrap();
        assert!(f.adding, "a vault that takes another seed is a round");
        assert_eq!(f.step, Step::Vaults);
    }
    // The vault that holds the seed and the vault that takes it, unlocked
    // inside the flow, never the visit page.
    assert!(offered(&mut app, Action::Flow(F::UnlockVault(0))) || step(&app) == Step::Vaults);
    unlock(&mut app, PHRASE);
    unlock(&mut app, PHRASE_2);
    assert_eq!(step(&app), Step::Make, "{:?}", app.vaults.unlock_error);
    shell.flow(&mut app, F::CopyKey);
    let n = app
        .vaults
        .open
        .iter()
        .map(|o| o.contents.of(kind::KEY).count())
        .max();
    assert_eq!(
        n,
        Some(2),
        "Vault 2 holds two keys now: {:?} {} keys {} wallets",
        app.toast_text(),
        app.session.keys.len(),
        app.session.wallets.len()
    );
    shell.flow(&mut app, F::MadeContinue);
    assert_eq!(step(&app), Step::Lock);
    shell.flow(&mut app, F::LockWrite);
    let mut app = restart(&mut shell, &mut app);
    assert_eq!(step(&app), Step::Write);
    for d in ["S1", "S2", "S3"] {
        shell.plug(&mut app, d);
        shell.pull(&mut app, d);
    }
    shell.plug(&mut app, "S4");
    shell.flow(&mut app, F::Tap);
    shell.pull(&mut app, "S4");
    assert_eq!(
        step(&app),
        Step::Done,
        "{:?} {:?}",
        app.flow().unwrap().live.problem,
        app.flow()
            .unwrap()
            .slots
            .iter()
            .map(|s| (s.target, s.files.clone(), s.written.is_some()))
            .collect::<Vec<_>>()
    );
    no_visit(&shell);
    // The copy of Vault 2 that went to its stick holds both seeds.
    let bytes = ofv(&shell, "S2").expect("Vault 2 on S2");
    let mut next = process(vec![("vault.ofv".into(), bytes)], Vec::new(), Vec::new());
    next.vaults.ms_per_unit = Some(180);
    next.press(Action::Nav(Screen::Vaults));
    next.press(Action::Vault(faraday_core::vaults::VaultAction::Open(0)));
    unlock(&mut next, PHRASE_2);
    assert_eq!(next.vaults.open[0].contents.of(kind::KEY).count(), 2);
}

#[test]
fn the_first_stick_put_back_for_the_second_place_is_refused_and_nothing_is_written() {
    let mut shell = Shell::new();
    let (mut app, _, _) = single_key_ready(&mut shell);
    shell.flow(&mut app, F::WalletLock);
    let mut app = restart(&mut shell, &mut app);
    shell.plug(&mut app, "S1");
    shell.pull(&mut app, "S1");
    let before = shell.writes.len();
    shell.plug(&mut app, "S1");
    expect(&mut app, "This stick has the first copy");
    assert!(app.flow().unwrap().slots[1].written.is_none());
    assert_eq!(shell.writes.len(), before, "nothing was written");
    // Use another stick: the second copy needs the hold.
    shell.flow(&mut app, F::UseAnother);
    shell.pull(&mut app, "S1");
    shell.plug(&mut app, "S2");
    shell.flow(&mut app, F::Tap);
    shell.flow(&mut app, F::Hold);
    assert!(app.flow().unwrap().slots[1].written.is_some());
}

#[test]
fn the_kept_state_of_a_wallet_flow_holds_nothing_secret_and_a_state_that_does_not_read_leaves_the_key_waiting()
 {
    let mut shell = Shell::new();
    let (mut app, _, fp) = single_key(&mut shell, true);
    let words = app
        .session
        .keys
        .iter()
        .find(|k| k.master.fingerprint().0 == fp)
        .and_then(|k| k.words.as_ref().map(|w| w.to_string()))
        .unwrap();
    shell.flow(&mut app, F::WalletLock);
    let app = restart(&mut shell, &mut app);
    let (_, _, kept) = shell.boxes.clone();
    for (name, bytes) in &kept {
        let text = String::from_utf8_lossy(bytes).to_string();
        for w in words.split_whitespace().take(3) {
            assert!(
                !text.contains(&format!(" {w} ")),
                "{name} holds a word of the seed"
            );
        }
        assert!(
            !text.contains(PHRASE),
            "{name} holds the vault's passphrase"
        );
    }
    assert!(kept.iter().any(|(n, _)| n == "backup-gate"));
    // The flows unreadable: the key still waits, from the gate alone.
    let mut kept2 = shell.boxes.2.clone();
    for (n, b) in kept2.iter_mut() {
        if n == "backup-flow" {
            *b = vec![0xff, 0x13, 0x37];
        }
    }
    let mut next = process(shell.boxes.0.clone(), shell.boxes.1.clone(), kept2);
    assert!(next.backup_flows.iter().any(|f| f.waiting()));
    next.press(Action::Nav(Screen::Vaults));
    next.press(Action::Vault(faraday_core::vaults::VaultAction::Open(0)));
    next.vaults.ms_per_unit = Some(180);
    unlock(&mut next, PHRASE);
    next.press(Action::Vault(
        faraday_core::vaults::VaultAction::LoadChosen(0),
    ));
    assert!(
        next.key_held(osk_bip::keys::Fingerprint(fp)),
        "fails closed: the key is held"
    );
    let _ = app;
}

#[test]
fn the_done_card_unlocks_to_the_checklist_done_and_the_files_read_written_to() {
    let mut shell = Shell::new();
    let (mut app, _, _) = single_key_ready(&mut shell);
    shell.flow(&mut app, F::WalletLock);
    let mut app = restart(&mut shell, &mut app);
    // While the flow is left, a stick visit lists its files In the backup.
    shell.flow(&mut app, F::Back);
    let wrote: Vec<String> = app
        .backup_flows
        .iter()
        .flat_map(|f| f.slots.iter().flat_map(|s| s.files.clone()))
        .collect();
    assert!(!wrote.is_empty());
    for n in &wrote {
        assert!(app.in_the_backup(n), "{n} is in the backup");
    }
    shell.flow(&mut app, F::Show(0));
    write_sticks(&mut shell, &mut app, &["S1", "S2", "S3"]);
    assert_eq!(step(&app), Step::Done);
    for n in &wrote {
        assert!(!app.in_the_backup(n), "{n} is done");
    }
    // Unlock: the wallet's checklist, done, with nothing For the stick.
    shell.flow(&mut app, F::UnlockDone(0));
    app.vaults.ms_per_unit = Some(180);
    unlock(&mut app, PHRASE);
    assert_eq!(app.screen, Screen::Backup, "{:?}", app.toast_text());
    assert!(app.backup_item_done(Item::Sticks));
    assert!(app.outbox.is_empty());
    expect_part(&mut app, "Written to KEYSTICK");
    assert!(!offered(&mut app, Action::WriteAsk), "no Write to a stick");
}

#[test]
fn a_seed_let_out_unprotected_is_written_to_the_stick_of_files_and_to_no_other() {
    let mut shell = Shell::new();
    sticks(&mut shell, 2);
    let mut app = process(Vec::new(), Vec::new(), Vec::new());
    keep_time(&mut app);
    create_with_vaults(&mut app, false, 1);
    bring_in_all(&mut shell, &mut app, &["S1"]);
    create_vault(&mut shell, &mut app, 0, PHRASE);
    let fp = key_in(&mut app, 1);
    app.press(Action::Nav(Screen::Create));
    app.press(Action::CSlotHere(0, fp));
    app.press(Action::CNext(cstep::KEYS));
    app.press(Action::CNext(cstep::CHECK));
    app.press(Action::CBackup(1));
    // The seed in Vault 1 and as a file at Place 2; nothing else.
    untick_everywhere(&mut app, Row::Words);
    keep_at(&mut app, 1, Row::File, &[0]);
    form_off_everywhere(&mut app, Form::Paper);
    {
        let f = app.backup_flows.last().unwrap();
        assert_eq!(
            f.plan.place_files,
            vec![1],
            "the seed's file is on Place 2's stick of files"
        );
        let t: Vec<Target> = f.slots.iter().map(|s| s.target).collect();
        assert_eq!(t, vec![Target::Place(1), Target::PlaceFiles(1)]);
        assert!(f.slots[0].bound.is_some(), "the stick read for Vault 1");
    }
    app.press(Action::BChecklist);
    save_vaults(&mut app);
    assert!(app.backup_items().contains(&bstep::FILES));
    // The seed's file, past the secret sheet.
    app.press(Action::BFile);
    assert_eq!(app.sheet, Some(Sheet::SecretOut));
    app.press(Action::SecretAck);
    app.press(Action::SecretUnprotected);
    assert!(
        app.outbox.iter().any(|i| i.secret),
        "the file waits, unprotected"
    );
    app.press(Action::BNext(bstep::ENVELOPE));
    shell.flow(&mut app, F::WalletLock);
    let mut app = restart(&mut shell, &mut app);
    // The vault's stick takes no secret file.
    shell.plug(&mut app, "S1");
    shell.pull(&mut app, "S1");
    assert!(
        !names(&shell, "S1")
            .iter()
            .any(|n| n.ends_with("-words.txt")),
        "no seed file on a vault's stick: {:?}",
        names(&shell, "S1")
    );
    shell.plug(&mut app, "S2");
    shell.flow(&mut app, F::Tap);
    shell.pull(&mut app, "S2");
    assert_eq!(step(&app), Step::Done);
    assert!(
        names(&shell, "S2")
            .iter()
            .any(|n| n.ends_with("-words.txt")),
        "the seed's file is on the stick of files: {:?}",
        names(&shell, "S2")
    );
    no_visit(&shell);
}

// ---- the review's fixes ----

/// The sticks of a flow written in order, each given what it asks for.
fn write_sticks(shell: &mut Shell, app: &mut Faraday, disks: &[&str]) {
    for d in disks {
        shell.plug(app, d);
        for _ in 0..3 {
            let r = app.flow().and_then(|f| f.live.reading.as_ref());
            let Some(r) = r else { break };
            if r.problem.is_some() || r.bound {
                break;
            }
            if r.hold {
                shell.flow(app, F::Hold);
            } else if r.differs.is_some() {
                shell.flow(app, F::UseCopy(true));
            } else {
                shell.flow(app, F::Tap);
            }
        }
        shell.pull(app, d);
    }
}

#[test]
fn a_second_backup_of_the_same_wallet_in_the_same_power_on_writes_again() {
    let mut shell = Shell::new();
    let (mut app, _, _) = single_key_ready(&mut shell);
    let mut d = Disk::empty("S4", "KEYSTICK");
    d.serial = Some(4);
    shell.add(d);
    shell.flow(&mut app, F::WalletLock);
    let mut app = restart(&mut shell, &mut app);
    write_sticks(&mut shell, &mut app, &["S1", "S2", "S3"]);
    assert_eq!(step(&app), Step::Done);
    // Unlock: the checklist, done. Then the plan gains a place, with a
    // copy of Vault 1 at it.
    shell.flow(&mut app, F::UnlockDone(0));
    app.vaults.ms_per_unit = Some(180);
    unlock(&mut app, PHRASE);
    assert_eq!(app.screen, Screen::Backup);
    assert!(app.backup_item_done(Item::Sticks));
    app.press(Action::BPlan);
    app.press(Action::BPlaces(true));
    stick_also_at(&mut app, 2, 0);
    app.press(Action::BChecklist);
    app.press(Action::BNext(bstep::ENVELOPE));
    assert!(
        !app.backup_item_done(Item::Sticks),
        "a place the plan names and no stick wrote is not done"
    );
    shell.flow(&mut app, F::WalletLock);
    let mut app = restart(&mut shell, &mut app);
    assert_eq!(step(&app), Step::Write);
    assert_eq!(app.flow().unwrap().written_of().0, 0, "a fresh flow");
    write_sticks(&mut shell, &mut app, &["S1", "S2", "S3", "S4"]);
    assert_eq!(
        step(&app),
        Step::Done,
        "{:?}",
        app.flow().unwrap().live.problem
    );
    // S3 is Place 2's stick of files, written in its place's turn; S4
    // the new place's.
    assert!(ofv(&shell, "S4").is_some(), "the new place was written");
    no_visit(&shell);
}

#[test]
fn a_wallet_left_with_its_vaults_open_does_not_take_the_next_wallets_keys() {
    let mut shell = Shell::new();
    sticks(&mut shell, 2);
    let mut app = process(Vec::new(), Vec::new(), Vec::new());
    keep_time(&mut app);
    // Wallet A: its vault made and open, then left.
    create_with_vaults(&mut app, false, 1);
    bring_in_all(&mut shell, &mut app, &["S1"]);
    create_vault(&mut shell, &mut app, 0, PHRASE);
    assert_eq!(app.screen, Screen::Create);
    app.press(Action::Nav(Screen::Home));
    // Wallet B, kept as it is made, not in a vault: its key goes to no
    // vault of A's, and neither does its description.
    app.press(Action::CreateWallet);
    let fp = key_in(&mut app, 1);
    app.press(Action::Nav(Screen::Create));
    app.press(Action::CSlotHere(0, fp));
    app.press(Action::CNext(cstep::KEYS));
    assert!(app.create.as_ref().unwrap().built.is_some());
    assert_eq!(app.vaults.open[0].contents.of(kind::KEY).count(), 0);
    assert_eq!(app.vaults.open[0].contents.of(kind::WALLET).count(), 0);
}

#[test]
fn a_seed_kept_only_as_a_file_is_held_until_its_stick_is_written() {
    let mut shell = Shell::new();
    sticks(&mut shell, 1);
    let mut app = process(Vec::new(), Vec::new(), Vec::new());
    keep_time(&mut app);
    app.press(Action::CreateWallet);
    let fp = new_key_here(&mut app);
    app.press(Action::CBackup(1));
    // No vault, no words, no sheet: the seed as a file at Place 1 alone.
    untick_everywhere(&mut app, Row::Vault(0));
    untick_everywhere(&mut app, Row::Words);
    keep_at(&mut app, 0, Row::File, &[0]);
    form_off_everywhere(&mut app, Form::Paper);
    app.press(Action::BChecklist);
    assert_eq!(
        app.backup_items(),
        vec![bstep::FILES, bstep::ENVELOPE, bstep::STICKS]
    );
    app.press(Action::BFile);
    app.press(Action::SecretAck);
    app.press(Action::SecretUnprotected);
    app.press(Action::BNext(bstep::ENVELOPE));
    assert!(
        app.key_held(osk_bip::keys::Fingerprint(fp)),
        "no vault holds it, and still it waits for its file's stick"
    );
    shell.flow(&mut app, F::WalletLock);
    let mut app = restart(&mut shell, &mut app);
    let held = app.backup_flows[0].wallet.as_ref().unwrap().held.clone();
    assert!(!held.is_empty(), "the flow holds the key back");
    assert!(app.backup_flows[0].waiting());
    write_sticks(&mut shell, &mut app, &["S1"]);
    assert_eq!(step(&app), Step::Done);
    assert!(!app.backup_flows[0].waiting());
}

/// A key made by New key for slot 0 of the Create open, whose wallet is
/// made with it (no flow).
fn new_key_here(app: &mut Faraday) -> [u8; 4] {
    app.press(Action::KeyGen(Some(0)));
    app.press(Action::KWords(12));
    app.press(Action::KWay(faraday_core::keygen::Way::Coins.index()));
    app.press(Action::KNext);
    for i in 0..128 {
        app.press(Action::KFlip((i * 7 + i / 5) % 2 == 0));
    }
    app.press(Action::KNext);
    app.press(Action::KLock);
    let fp = app.session.keys.last().unwrap().master.fingerprint().0;
    app.press(Action::KNext);
    fp
}

#[test]
fn a_kept_state_cut_short_or_missing_its_wallet_part_still_holds_the_key() {
    let mut shell = Shell::new();
    let (mut app, _, fp) = single_key(&mut shell, true);
    shell.flow(&mut app, F::WalletLock);
    let _app = restart(&mut shell, &mut app);
    let kept = shell.boxes.2.clone();
    let flow_text = String::from_utf8(
        kept.iter()
            .find(|(n, _)| n == "backup-flow")
            .map(|(_, b)| b.clone())
            .unwrap(),
    )
    .unwrap();
    let held_after = |kept: Vec<(String, Vec<u8>)>| {
        let mut next = process(shell.boxes.0.clone(), shell.boxes.1.clone(), kept);
        let held_in_flow = next
            .backup_flows
            .iter()
            .filter_map(|f| f.wallet.as_ref())
            .any(|w| !w.held.is_empty());
        next.press(Action::Nav(Screen::Vaults));
        next.press(Action::Vault(faraday_core::vaults::VaultAction::Open(0)));
        next.vaults.ms_per_unit = Some(180);
        unlock(&mut next, PHRASE);
        next.press(Action::Vault(
            faraday_core::vaults::VaultAction::LoadChosen(0),
        ));
        (held_in_flow, next.key_held(osk_bip::keys::Fingerprint(fp)))
    };
    let with = |bytes: Vec<u8>| -> Vec<(String, Vec<u8>)> {
        let mut k = kept.clone();
        for (n, b) in k.iter_mut() {
            if n == "backup-flow" {
                *b = bytes.clone();
            }
        }
        k
    };
    // Cut short.
    let cut = flow_text.as_bytes()[..flow_text.len() / 2].to_vec();
    assert!(held_after(with(cut)).1, "a cut backup-flow holds the key");
    // The wallet's lines gone: the flow does not read, the gate holds.
    let no_wallet: String = flow_text
        .lines()
        .filter(|l| {
            ![
                "wallet\t",
                "wshape\t",
                "wseed\t",
                "wans\t",
                "wsealed\t",
                "wbase\t",
            ]
            .iter()
            .any(|p| l.starts_with(p))
        })
        .map(|l| format!("{l}\n"))
        .collect();
    assert!(held_after(with(no_wallet.into_bytes())).1);
    // Its held list empty: the flow reads, and the gate's list is merged.
    let emptied: String = flow_text
        .lines()
        .map(|l| {
            if l.starts_with("wallet\t") {
                let mut p: Vec<&str> = l.split('\t').collect();
                p[3] = "";
                format!("{}\n", p.join("\t"))
            } else {
                format!("{l}\n")
            }
        })
        .collect();
    let (in_flow, held) = held_after(with(emptied.into_bytes()));
    assert!(in_flow, "the gate's fingerprints were merged into the flow");
    assert!(held);
}

#[test]
fn a_vault_stays_with_its_own_salt_when_a_slot_turns_out_to_be_a_cosigners() {
    let mut shell = Shell::new();
    sticks(&mut shell, 3);
    let mut app = process(Vec::new(), Vec::new(), Vec::new());
    keep_time(&mut app);
    create_with_vaults(&mut app, true, 3);
    bring_in_all(&mut shell, &mut app, &["S1", "S2", "S3"]);
    create_vault(&mut shell, &mut app, 0, PHRASE);
    create_vault(&mut shell, &mut app, 1, PHRASE_2);
    create_vault(&mut shell, &mut app, 2, PHRASE_3);
    assert_eq!(app.screen, Screen::Create);
    let salts: Vec<[u8; 32]> = app
        .backup_flows
        .last()
        .unwrap()
        .plan
        .vaults
        .iter()
        .map(|v| v.salt.unwrap())
        .collect();
    // Keys 1 and 3 are here; Key 2 is a cosigner's account key.
    let a = key_in(&mut app, 0);
    let b = key_in(&mut app, 1);
    let c = key_in(&mut app, 2);
    let cosigner = {
        let kind = NewKind::Multi;
        let k = app
            .session
            .keys
            .iter()
            .find(|k| k.master.fingerprint().0 == b)
            .unwrap();
        kind.key_text(&k.master).unwrap()
    };
    app.session.keys.retain(|k| k.master.fingerprint().0 != b);
    app.press(Action::Nav(Screen::Create));
    app.press(Action::CSlotHere(0, a));
    app.create.as_mut().unwrap().slots[1] = faraday_core::create::Source::Cosigner(cosigner);
    app.press(Action::CSlotHere(2, c));
    app.press(Action::CNext(cstep::KEYS));
    assert!(
        app.create.as_ref().unwrap().built.is_some(),
        "{:?}",
        app.create.as_ref().unwrap().error
    );
    // The vaults set up stay as they are, each with its own salt, however
    // the keys turned out: all three are the plan's.
    let f = app
        .backup_flows
        .iter()
        .find(|f| f.wallet.is_some())
        .unwrap();
    let now: Vec<Option<[u8; 32]>> = f.plan.vaults.iter().map(|v| v.salt).collect();
    assert_eq!(now, salts.iter().map(|s| Some(*s)).collect::<Vec<_>>());
    app.press(Action::CNext(cstep::CHECK));
    app.press(Action::CBackup(1));
    let b = app.backup.as_ref().unwrap();
    assert_eq!(b.answers.vaults.len(), 3, "Back up offers the three vaults");
}

#[test]
fn the_round_keeps_a_passphrase_apart_and_copies_only_this_wallets_records() {
    use faraday_vault::Record;
    use faraday_vault::records::field;
    let mut shell = Shell::new();
    let mut app = two_of_three_ready_with(&mut shell, true);
    shell.flow(&mut app, F::WalletLock);
    let mut app = restart(&mut shell, &mut app);
    shell.flow(&mut app, F::ChangePlan);
    flow_vault_add(&mut shell, &mut app, 1, 0);
    shell.flow(&mut app, F::Continue);
    unlock(&mut app, PHRASE);
    unlock(&mut app, PHRASE_2);
    assert_eq!(step(&app), Step::Make, "{:?}", app.vaults.unlock_error);
    // Another wallet's description and plan sit in the holder vault.
    let foreign = Record::new(kind::WALLET)
        .with(field::WALLET, testkit::savings().as_bytes())
        .with(field::WALLET_NAME, b"Other");
    app.vaults.open[0].contents.records.push(foreign);
    let foreign_plan = Record::new(kind::PLAN)
        .with(field::PLAN_WALLET, testkit::savings().as_bytes())
        .with(field::PLAN_ANSWERS, b"other");
    app.vaults.open[0].contents.records.push(foreign_plan);
    shell.flow(&mut app, F::CopyKey);
    let target = app
        .vaults
        .open
        .iter()
        .find(|o| o.contents.of(kind::KEY).count() == 2)
        .expect("the vault that took the seed");
    // The passphrase stays apart: no record of it in the vault.
    assert!(
        target
            .contents
            .of(kind::KEY)
            .all(|(_, r)| r.field(field::KEY_PASSPHRASE).is_none()),
        "a passphrase kept apart is not put in the vault"
    );
    // Nothing of the other wallet went across.
    let has_foreign = target.contents.of(kind::WALLET).any(|(_, r)| {
        r.text(field::WALLET).is_some_and(|t| {
            t.contains(testkit::savings().split('(').next().unwrap()) && t == testkit::savings()
        })
    });
    assert!(!has_foreign, "another wallet's description was copied");
    let has_foreign_plan = target
        .contents
        .of(kind::PLAN)
        .any(|(_, r)| r.text(field::PLAN_WALLET) == Some(testkit::savings().as_str()));
    assert!(!has_foreign_plan, "another wallet's plan was copied");
}

/// [`two_of_three_ready`] with Key 1 typed in with a passphrase.
fn two_of_three_ready_with(shell: &mut Shell, passphrase: bool) -> Faraday {
    sticks(shell, 4);
    let main = vault("Main", &[PHRASE], 0x41);
    {
        let d = shell.disk_mut("S1");
        d.files = vec![("main.ofv".into(), main)];
    }
    let mut app = process(Vec::new(), Vec::new(), Vec::new());
    keep_time(&mut app);
    create_with_vaults(&mut app, true, 3);
    shell.plug(&mut app, "S1");
    let k = app
        .flow()
        .unwrap()
        .live
        .reading
        .as_ref()
        .unwrap()
        .files
        .iter()
        .position(|(f, _)| f.name == "main.ofv")
        .unwrap();
    shell.flow(&mut app, F::PickFile(0, k));
    shell.flow(&mut app, F::Tap);
    shell.pull(&mut app, "S1");
    for d in ["S2", "S3"] {
        shell.plug(&mut app, d);
        shell.flow(&mut app, F::Tap);
        shell.pull(&mut app, d);
    }
    unlock(&mut app, PHRASE);
    create_vault(shell, &mut app, 1, PHRASE_2);
    create_vault(shell, &mut app, 2, PHRASE_3);
    let mut fps = Vec::new();
    for i in 0..3 {
        app.press(Action::Entry(None));
        for c in testkit::test_words(testkit::TEST_SEEDS[i].0).chars() {
            app.event(Event::Key(Key::Char(c)));
        }
        if i == 0 && passphrase {
            app.press(Action::EntryPassphrase);
            type_text(&mut app, "kept apart");
        }
        app.press(Action::EntryAdd);
        fps.push(app.session.keys.last().unwrap().master.fingerprint().0);
    }
    app.press(Action::Nav(Screen::Create));
    for (i, fp) in fps.iter().enumerate() {
        app.press(Action::CSlotHere(i as u8, *fp));
    }
    app.press(Action::CNext(cstep::KEYS));
    app.press(Action::CNext(cstep::CHECK));
    app.press(Action::CBackup(1));
    untick_everywhere(&mut app, Row::Words);
    app.press(Action::BChecklist);
    save_vaults(&mut app);
    app.press(Action::BOut(3));
    app.press(Action::BNext(bstep::ENVELOPE));
    app
}

#[test]
fn a_vault_preset_chosen_after_the_keys_goes_through_to_the_writes() {
    let mut shell = Shell::new();
    sticks(&mut shell, 2);
    let main = vault("Main", &[PHRASE], 0x41);
    let mut app = process(vec![("main.ofv".into(), main)], Vec::new(), Vec::new());
    keep_time(&mut app);
    app.press(Action::CreateWallet);
    // Not in a vault: the default.
    let fp = key_in(&mut app, 1);
    app.press(Action::Nav(Screen::Create));
    app.press(Action::CSlotHere(0, fp));
    app.press(Action::CNext(cstep::KEYS));
    app.press(Action::CNext(cstep::CHECK));
    assert!(!app.create.as_ref().unwrap().into_vaults());
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(faraday_core::vaults::VaultAction::Open(0)));
    app.vaults.ms_per_unit = Some(180);
    unlock(&mut app, PHRASE);
    app.press(Action::Nav(Screen::Create));
    // Paper and vault, chosen after the keys: the vault is the open one,
    // the key goes into it, its stick is checked at its write.
    app.press(Action::CBackup(1));
    untick_everywhere(&mut app, Row::Words);
    app.press(Action::BChecklist);
    app.press(Action::BVaultSave(0));
    assert!(app.backup_item_done(Item::Vault(0)));
    app.press(Action::BOut(3));
    app.press(Action::BNext(bstep::ENVELOPE));
    shell.flow(&mut app, F::WalletLock);
    let mut app = restart(&mut shell, &mut app);
    assert_eq!(step(&app), Step::Write);
    write_sticks(&mut shell, &mut app, &["S1", "S2"]);
    assert_eq!(
        step(&app),
        Step::Done,
        "{:?}",
        app.flow().unwrap().live.problem
    );
    assert!(ofv(&shell, "S1").is_some() || ofv(&shell, "S2").is_some());
    no_visit(&shell);
}

#[test]
fn where_reads_its_answer_after_the_wallet_is_made_and_check_goes_to_back_up() {
    let mut shell = Shell::new();
    let (mut app, _, _) = single_key(&mut shell, false);
    app.press(Action::CNext(cstep::KIND));
    app.press(Action::CNext(cstep::KEYS));
    assert!(app.create.as_ref().unwrap().built.is_some());
    assert!(
        app.create.as_ref().unwrap().into_vaults(),
        "the question reads what was answered"
    );
    for k in [cstep::WHERE, cstep::HOW_MANY, cstep::STICKS, cstep::VAULTS] {
        assert!(app.create_card_done(k), "card {k} reads done");
    }
    // Check's Continue opens Back up's card, not the vaults.
    app.press(Action::CNext(cstep::CHECK));
    assert_eq!(app.create.as_ref().unwrap().open, Some(cstep::BACKUP));
}

#[test]
fn a_key_in_a_slot_goes_into_no_vault_by_itself_and_a_passphrase_added_later_leaves_one_record() {
    let mut shell = Shell::new();
    sticks(&mut shell, 1);
    let mut app = process(Vec::new(), Vec::new(), Vec::new());
    keep_time(&mut app);
    create_with_vaults(&mut app, false, 1);
    bring_in_all(&mut shell, &mut app, &["S1"]);
    create_vault(&mut shell, &mut app, 0, PHRASE);
    let a = key_in(&mut app, 0);
    let b = key_in(&mut app, 1);
    app.press(Action::Nav(Screen::Create));
    app.press(Action::CSlotHere(0, a));
    assert_eq!(app.vaults.open[0].contents.of(kind::KEY).count(), 0);
    // Replaced by another key, with a passphrase added later.
    app.press(Action::CSlotHere(0, b));
    app.press(Action::CPassOpen(0));
    for f in [0u8, 1] {
        app.press(Action::CPassField(f));
        type_text(&mut app, "later pass");
    }
    app.press(Action::CPassLock);
    assert_eq!(
        app.vaults.open[0].contents.of(kind::KEY).count(),
        0,
        "still in no vault"
    );
    app.press(Action::CNext(cstep::KEYS));
    app.press(Action::CNext(cstep::CHECK));
    app.press(Action::CBackup(1));
    untick_everywhere(&mut app, Row::Words);
    app.press(Action::BChecklist);
    save_vaults(&mut app);
    assert_eq!(
        app.vaults.open[0].contents.of(kind::KEY).count(),
        1,
        "one record of the seed"
    );
    // The record that remains is key b's, not key a's.
    let lang = app.session.keys.last().unwrap().language;
    let b_words =
        osk_bip::bip39::Mnemonic::parse(lang, &testkit::test_words(testkit::TEST_SEEDS[1].0))
            .unwrap();
    let (_, rec) = app.vaults.open[0].contents.of(kind::KEY).next().unwrap();
    assert_eq!(
        faraday_vault::records::words_of(rec).unwrap().indices(),
        b_words.indices(),
        "the record is key b's"
    );
}

#[test]
fn a_description_added_to_an_existing_vault_after_the_lock_starts_a_round() {
    let mut shell = Shell::new();
    let (mut app, _, _) = single_key_with(&mut shell, false, false);
    shell.flow(&mut app, F::WalletLock);
    let mut app = restart(&mut shell, &mut app);
    assert_eq!(step(&app), Step::Write);
    shell.flow(&mut app, F::ChangePlan);
    let w = app
        .flow()
        .unwrap()
        .wallet
        .as_ref()
        .unwrap()
        .answers
        .wallet_in_vault(0);
    let p = flow_place_of(&app, 0);
    flow_plan(
        &mut shell,
        &mut app,
        PlanPress::Form(p as u8, Form::Vault(0).code()),
    );
    let w2 = app
        .flow()
        .unwrap()
        .wallet
        .as_ref()
        .unwrap()
        .answers
        .wallet_in_vault(0);
    assert_ne!(w, w2, "{w:?} {w2:?} {:?}", app.toast_text());
    shell.flow(&mut app, F::Continue);
    assert!(app.flow().unwrap().adding, "the description is a round");
    unlock(&mut app, PHRASE);
    assert_eq!(step(&app), Step::Make, "{:?}", app.vaults.unlock_error);
    shell.flow(&mut app, F::CopyKey);
    assert_eq!(app.vaults.open[0].contents.of(kind::WALLET).count(), 1);
}

#[test]
fn a_one_place_plan_with_a_passphrase_stays_one_place_when_the_wallet_is_made() {
    let shape = one_key_shape();
    let mut a = Answers::preset(&shape, Preset::Paper);
    a.set_places(&shape, 1);
    let mut with = shape.clone();
    with.seeds[0].passphrase = true;
    a.refit(&with);
    assert_eq!(a.places, 1, "refit adds no place");
    assert!(
        a.pass[0].iter().all(|b| !b),
        "and places no passphrase: that is the person's to say"
    );
    // A preset still places one apart.
    let p = Answers::preset(&with, Preset::Paper);
    assert!(p.pass_at(0, p.places - 1));
    // Seeds are renumbered by where they turned out to be.
    let shape3 = Shape {
        m: 2,
        keys: 3,
        seeds: (0..3)
            .map(|i| plan::Seed {
                name: format!("k{i}"),
                here: true,
                passphrase: false,
            })
            .collect(),
        splits: true,
        vaults: 0,
    };
    let mut b = Answers::preset(&shape3, Preset::PaperVault);
    b.vaults[0] = vec![true, false, false];
    b.remap_seeds(&[2, 0, 1], 3);
    assert_eq!(b.vaults[0], vec![false, false, true], "old seed 0 is now 2");
}

#[test]
fn only_this_wallets_seed_files_go_to_the_stick_of_files_and_public_files_have_their_own() {
    // A plan with seed files and the description as files: two sticks.
    let mut shape = one_key_shape();
    shape.seeds[0].name = "9a6a2580".into();
    let mut a = Answers::preset(&shape, Preset::PaperVault);
    a.toggle(&shape, plan::Question::Seeds, seeds::FILE);
    a.toggle(&shape, plan::Question::Wallet, wallet::FILES);
    a.toggle(&shape, plan::Question::Form, form::TEXT);
    let f = BackupFlow::new_wallet(shape.clone(), a, "Savings");
    let t: Vec<Target> = f.slots.iter().map(|s| s.target).collect();
    assert!(
        t.contains(&Target::Files) && t.contains(&Target::Public),
        "{t:?}"
    );
    assert!(f.seed_file_ok("savings-9a6a2580-words.txt"));
    assert!(
        !f.seed_file_ok("savings-deadbeef-words.txt"),
        "another seed's"
    );
    assert!(
        !f.seed_file_ok("other-9a6a2580-words.txt"),
        "another wallet's"
    );
    assert!(!f.seed_file_ok("savings-9a6a2580-words.txt.bak"));
}

#[test]
fn back_on_a_card_before_the_keys_returns_to_make_a_new_wallet_and_a_refusal_says_so() {
    let mut shell = Shell::new();
    sticks(&mut shell, 1);
    let mut app = process(Vec::new(), Vec::new(), Vec::new());
    keep_time(&mut app);
    create_with_vaults(&mut app, false, 1);
    shell.flow(&mut app, F::Back);
    assert_eq!(app.screen, Screen::Create);
    // A stick written stays: taking it off the plan says why.
    let (mut app, _, _) = single_key_ready(&mut shell);
    shell.flow(&mut app, F::WalletLock);
    let mut app = restart(&mut shell, &mut app);
    write_sticks(&mut shell, &mut app, &["S1"]);
    shell.flow(&mut app, F::ChangePlan);
    flow_plan(
        &mut shell,
        &mut app,
        PlanPress::RowOff(0, Row::Vault(0).code()),
    );
    assert_eq!(
        app.toast_text(),
        Some("A stick already written stays on the plan")
    );
}

// ---- the second review's fixes ----

/// A vault file of one passphrase that already holds the key of
/// `TEST_SEEDS[n]`.
fn vault_with_key(label: &str, phrase: &str, seed: u8, n: usize) -> Vec<u8> {
    use faraday_vault::Record;
    use faraday_vault::records::{self, field};
    let cost = faraday_vault::Cost {
        memory_kib: faraday_vault::MIN_MEMORY_KIB,
        passes: 1,
        lanes: 1,
    };
    let words = testkit::test_words(testkit::TEST_SEEDS[n].0);
    let m = osk_bip::bip39::Mnemonic::parse(osk_bip::bip39::Language::English, &words).unwrap();
    let contents = faraday_vault::Contents {
        records: vec![
            Record::new(records::kind::SLOT_LABEL).with(field::LABEL, label.as_bytes()),
            Record::new(kind::KEY).with(field::KEY, &records::words_payload(&m)),
        ],
    };
    faraday_vault::create(
        faraday_vault::SLOT_SIZES[0],
        cost,
        &[phrase.as_bytes()],
        &[contents],
        &[seed; 32],
    )
    .unwrap()
}

/// Loads the keys of the first vault of a fresh process into it.
fn load_first_vault(app: &mut Faraday, phrase: &str) {
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(faraday_core::vaults::VaultAction::Open(0)));
    app.vaults.ms_per_unit = Some(180);
    unlock(app, phrase);
    app.press(Action::Vault(
        faraday_core::vaults::VaultAction::LoadChosen(0),
    ));
}

#[test]
fn a_key_held_for_its_sticks_stays_kept_on_one_and_is_held_until_every_stick_is_written() {
    let mut shell = Shell::new();
    let (mut app, _, fp) = single_key(&mut shell, true);
    shell.flow(&mut app, F::WalletLock);
    let mut app = restart(&mut shell, &mut app);
    assert_eq!(step(&app), Step::Write);
    let slots = app.flow().unwrap().slots.len();
    shell.flow(&mut app, F::ChangePlan);
    // The key taken out of Vault 1 on its sheet: it would be kept on no
    // stick.
    let p = flow_place_of(&app, 0);
    flow_keep(&mut shell, &mut app, p, Row::Vault(0), &[]);
    assert_eq!(
        app.toast_text(),
        Some("A key not yet written stays kept on a stick")
    );
    {
        let wf = app.flow().unwrap().wallet.as_ref().unwrap();
        assert!(wf.answers.vault_seeds(&wf.shape, 0).contains(&0));
    }
    assert_eq!(app.flow().unwrap().slots.len(), slots);
    // Vault 1's stick at the first place goes; the one at the second stays.
    flow_plan(
        &mut shell,
        &mut app,
        PlanPress::RowOff(0, Row::Vault(0).code()),
    );
    assert_eq!(app.flow().unwrap().slots.len(), slots - 1);
    // The last stick of the vault is refused, with its toast.
    flow_plan(
        &mut shell,
        &mut app,
        PlanPress::RowOff(1, Row::Vault(0).code()),
    );
    assert_eq!(
        app.toast_text(),
        Some("A key not yet written stays kept on a stick")
    );
    assert!(
        app.flow()
            .unwrap()
            .wallet
            .as_ref()
            .unwrap()
            .answers
            .stick_at(0, 1),
        "the vault keeps a stick"
    );
    // The vault's stick is written; the stick of files is not: the key
    // is still held, for every stick of the plan.
    write_sticks(&mut shell, &mut app, &["S2"]);
    assert!(held_after_lock(&mut shell, &mut app, fp));
}

/// Locks, loads the first vault's keys in the process that follows, and
/// says whether key `fp` is loaded and held.
fn held_after_lock(shell: &mut Shell, app: &mut Faraday, fp: [u8; 4]) -> bool {
    let mut next = shell.lock(app);
    assert!(next.backup_flows[0].waiting());
    load_first_vault(&mut next, PHRASE);
    assert!(
        next.session
            .keys
            .iter()
            .any(|k| k.master.fingerprint().0 == fp),
        "the key is loaded"
    );
    next.key_held(osk_bip::keys::Fingerprint(fp))
}

#[test]
fn a_key_with_one_of_its_two_vault_sticks_written_is_held() {
    let mut shell = Shell::new();
    let (mut app, _, fp) = single_key(&mut shell, true);
    shell.flow(&mut app, F::WalletLock);
    let mut app = restart(&mut shell, &mut app);
    assert!(app.flow().unwrap().slots.len() >= 3, "two vault sticks");
    write_sticks(&mut shell, &mut app, &["S1"]);
    assert_eq!(app.flow().unwrap().written_of().0, 1);
    assert!(held_after_lock(&mut shell, &mut app, fp));
}

#[test]
fn a_key_with_its_vault_sticks_written_but_not_the_stick_of_the_description_is_held() {
    let mut shell = Shell::new();
    let (mut app, _, fp) = single_key(&mut shell, true);
    shell.flow(&mut app, F::WalletLock);
    let mut app = restart(&mut shell, &mut app);
    write_sticks(&mut shell, &mut app, &["S1", "S2"]);
    assert_eq!(app.flow().unwrap().written_of(), (2, 3), "S3 waits");
    assert!(held_after_lock(&mut shell, &mut app, fp));
}

#[test]
fn a_key_with_its_vault_written_but_not_its_seed_file_is_held() {
    let mut shell = Shell::new();
    sticks(&mut shell, 2);
    let mut app = process(Vec::new(), Vec::new(), Vec::new());
    keep_time(&mut app);
    create_with_vaults(&mut app, false, 1);
    bring_in_all(&mut shell, &mut app, &["S1"]);
    create_vault(&mut shell, &mut app, 0, PHRASE);
    app.press(Action::Nav(Screen::Create));
    let fp = new_key_here(&mut app);
    app.press(Action::CNext(cstep::CHECK));
    app.press(Action::CBackup(1));
    untick_everywhere(&mut app, Row::Words);
    // The seed's file at the place that keeps Vault 1's stick.
    let p = vault_place(&app, 0);
    keep_at(&mut app, p, Row::File, &[0]);
    form_off_everywhere(&mut app, Form::Paper);
    app.press(Action::BChecklist);
    save_vaults(&mut app);
    app.press(Action::BFile);
    app.press(Action::SecretAck);
    app.press(Action::SecretUnprotected);
    app.press(Action::BNext(bstep::ENVELOPE));
    shell.flow(&mut app, F::WalletLock);
    let mut app = restart(&mut shell, &mut app);
    write_sticks(&mut shell, &mut app, &["S1"]);
    assert_eq!(app.flow().unwrap().written_of(), (1, 2), "the file waits");
    assert!(held_after_lock(&mut shell, &mut app, fp));
}

#[test]
fn a_passphrase_kept_only_in_the_vault_is_not_taken_out_while_its_key_is_held() {
    let mut shell = Shell::new();
    let (mut app, _, _) = single_key(&mut shell, true);
    shell.flow(&mut app, F::WalletLock);
    let mut app = restart(&mut shell, &mut app);
    // Seed 1 has a passphrase, kept only in Vault 1, and the seed is also a
    // file.
    {
        let wf = app.backup_flows[0].wallet.as_mut().unwrap();
        wf.shape.seeds[0].passphrase = true;
        let m = wf.answers.map.clone().expect("the map is the plan");
        let m = plan::without(&m, At::Vault(0), What::Seed(0));
        let m = plan::with_added(&m, At::Vault(0), What::SeedPassphrase(0));
        let m = plan::with_added(&m, At::Place(0), What::SeedFile(0));
        wf.answers.set_map(m);
        assert!(!wf.held.is_empty());
    }
    shell.flow(&mut app, F::ChangePlan);
    // The seed out of Vault 1: its file keeps it on a stick, but its
    // passphrase would be kept nowhere.
    let p = flow_place_of(&app, 0);
    flow_keep(&mut shell, &mut app, p, Row::Vault(0), &[]);
    assert_eq!(
        app.toast_text(),
        Some("A key not yet written stays kept on a stick")
    );
    let wf = app.flow().unwrap().wallet.as_ref().unwrap();
    assert!(wf.answers.vault_seeds(&wf.shape, 0).contains(&0));
}

#[test]
fn two_planned_vaults_holding_the_same_seed_are_never_one_open_vault() {
    let mut shell = Shell::new();
    let mut app = two_of_three_ready_with(&mut shell, false);
    // Vault 2 holds Key 1 too, in place of Key 2.
    app.press(Action::BPlan);
    let p = vault_place(&app, 1);
    keep_at(&mut app, p, Row::Vault(1), &[0]);
    app.press(Action::BChecklist);
    app.press(Action::BVaultSave(1));
    app.press(Action::BOut(3));
    app.press(Action::BNext(bstep::ENVELOPE));
    shell.flow(&mut app, F::WalletLock);
    let app = restart(&mut shell, &mut app);
    let salts: Vec<Option<[u8; 32]>> = app.backup_flows[0]
        .plan
        .vaults
        .iter()
        .map(|v| v.salt)
        .collect();
    assert!(salts[0].is_some() && salts[1].is_some());
    assert_ne!(salts[0], salts[1], "each planned vault is its own");
}

#[test]
fn a_seed_file_let_out_and_then_dropped_from_the_plan_is_not_written_beside_the_public_files() {
    let mut shell = Shell::new();
    sticks(&mut shell, 2);
    let mut app = process(Vec::new(), Vec::new(), Vec::new());
    keep_time(&mut app);
    create_with_vaults(&mut app, false, 1);
    bring_in_all(&mut shell, &mut app, &["S1"]);
    create_vault(&mut shell, &mut app, 0, PHRASE);
    let fp = key_in(&mut app, 1);
    app.press(Action::Nav(Screen::Create));
    app.press(Action::CSlotHere(0, fp));
    app.press(Action::CNext(cstep::KEYS));
    app.press(Action::CNext(cstep::CHECK));
    app.press(Action::CBackup(1));
    untick_everywhere(&mut app, Row::Words);
    let p = vault_place(&app, 0);
    keep_at(&mut app, p, Row::File, &[0]);
    form_off_everywhere(&mut app, Form::Paper);
    app.press(Action::BChecklist);
    save_vaults(&mut app);
    // The seed's file is let out on the secret sheet.
    app.press(Action::BFile);
    app.press(Action::SecretAck);
    app.press(Action::SecretUnprotected);
    assert!(app.outbox.iter().any(|i| i.secret));
    // Then the plan drops seed files and keeps the wallet as files.
    app.press(Action::BRowOff(p as u8, Row::File.code()));
    form_on(&mut app, p, Form::File);
    answer_backup(&mut app, qrow::FORM, form::QR);
    answer_backup(&mut app, qrow::FORM, form::TEXT);
    save_vaults(&mut app);
    app.press(Action::BFilesMake);
    app.press(Action::BNext(bstep::ENVELOPE));
    shell.flow(&mut app, F::WalletLock);
    let mut app = restart(&mut shell, &mut app);
    {
        let f = app.flow().unwrap();
        assert!(
            f.slots
                .iter()
                .all(|s| s.files.iter().all(|n| !n.ends_with("-words.txt"))),
            "no stick is given the seed file"
        );
    }
    write_sticks(&mut shell, &mut app, &["S1", "S2"]);
    assert_eq!(
        step(&app),
        Step::Done,
        "{:?}",
        app.flow().unwrap().live.problem
    );
    for d in ["S1", "S2"] {
        assert!(
            names(&shell, d).iter().all(|n| !n.ends_with("-words.txt")),
            "{d}: {:?}",
            names(&shell, d)
        );
    }
    no_visit(&shell);
}

#[test]
fn a_record_the_vault_held_before_the_flow_is_neither_doubled_nor_taken_out() {
    let mut shell = Shell::new();
    sticks(&mut shell, 2);
    // Main holds the key of Key 2 (TEST_SEEDS[1]) already.
    let main = vault_with_key("Main", PHRASE, 0x41, 1);
    shell.disk_mut("S1").files = vec![("main.ofv".into(), main)];
    let mut app = process(Vec::new(), Vec::new(), Vec::new());
    keep_time(&mut app);
    create_with_vaults(&mut app, false, 1);
    shell.plug(&mut app, "S1");
    let k = app
        .flow()
        .unwrap()
        .live
        .reading
        .as_ref()
        .unwrap()
        .files
        .iter()
        .position(|(f, _)| f.name == "main.ofv")
        .unwrap();
    shell.flow(&mut app, F::PickFile(0, k));
    shell.flow(&mut app, F::Tap);
    shell.pull(&mut app, "S1");
    unlock(&mut app, PHRASE);
    assert_eq!(app.screen, Screen::Create, "{:?}", app.vaults.unlock_error);
    let keys = |app: &Faraday| app.vaults.open[0].contents.of(kind::KEY).count();
    assert_eq!(keys(&app), 1, "the vault held the key already");
    let a = key_in(&mut app, 1);
    app.press(Action::Nav(Screen::Create));
    app.press(Action::CSlotHere(0, a));
    app.press(Action::CSlotClear(0));
    assert_eq!(keys(&app), 1, "the record that was there stays");
    app.press(Action::CSlotHere(0, a));
    app.press(Action::CNext(cstep::KEYS));
    app.press(Action::CNext(cstep::CHECK));
    app.press(Action::CBackup(1));
    untick_everywhere(&mut app, Row::Words);
    app.press(Action::BChecklist);
    save_vaults(&mut app);
    assert_eq!(keys(&app), 1, "saved into the vault, not doubled");
    assert_eq!(app.vaults.open[0].contents.of(kind::WALLET).count(), 1);
}

#[test]
fn the_vaults_set_up_stay_the_plans_when_a_key_turns_out_not_to_be_here() {
    let mut shell = Shell::new();
    sticks(&mut shell, 3);
    let mut app = process(Vec::new(), Vec::new(), Vec::new());
    keep_time(&mut app);
    create_with_vaults(&mut app, true, 3);
    bring_in_all(&mut shell, &mut app, &["S1", "S2", "S3"]);
    create_vault(&mut shell, &mut app, 0, PHRASE);
    create_vault(&mut shell, &mut app, 1, PHRASE_2);
    create_vault(&mut shell, &mut app, 2, PHRASE_3);
    let salts: Vec<Option<[u8; 32]>> = app.backup_flows[0]
        .plan
        .vaults
        .iter()
        .map(|v| v.salt)
        .collect();
    assert_eq!(salts.len(), 3);
    let a = key_in(&mut app, 0);
    let b = key_in(&mut app, 1);
    let c = key_in(&mut app, 2);
    app.press(Action::Nav(Screen::Create));
    for (i, fp) in [a, b, c].iter().enumerate() {
        app.press(Action::CSlotHere(i as u8, *fp));
    }
    app.press(Action::CNext(cstep::KEYS));
    let w = app.create.as_ref().unwrap().built.expect("made");
    // Key 2's words go: no vault is dropped for it.
    app.session.keys.retain(|k| k.master.fingerprint().0 != b);
    app.press(Action::CNext(cstep::CHECK));
    app.press(Action::Backup(w));
    let f = app
        .backup_flows
        .iter()
        .find(|f| f.wallet.is_some())
        .unwrap();
    let now: Vec<Option<[u8; 32]>> = f.plan.vaults.iter().map(|v| v.salt).collect();
    assert_eq!(now, salts, "each vault keeps its own salt");
    assert_eq!(app.backup.as_ref().unwrap().answers.vaults.len(), 3);
}

#[test]
fn the_lock_keeps_the_flows_own_vault_when_another_open_vault_holds_the_same_key() {
    let mut shell = Shell::new();
    sticks(&mut shell, 2);
    let other = vault_with_key("Other", PHRASE_2, 0x42, 1);
    let mut app = process(vec![("other.ofv".into(), other)], Vec::new(), Vec::new());
    keep_time(&mut app);
    create_with_vaults(&mut app, false, 1);
    bring_in_all(&mut shell, &mut app, &["S1"]);
    create_vault(&mut shell, &mut app, 0, PHRASE);
    let own = app.backup_flows[0].plan.vaults[0].salt.unwrap();
    let fp = key_in(&mut app, 1);
    app.press(Action::Nav(Screen::Create));
    app.press(Action::CSlotHere(0, fp));
    app.press(Action::CNext(cstep::KEYS));
    app.press(Action::CNext(cstep::CHECK));
    app.press(Action::CBackup(1));
    untick_everywhere(&mut app, Row::Words);
    form_off_everywhere(&mut app, Form::Paper);
    app.press(Action::BChecklist);
    save_vaults(&mut app);
    app.press(Action::BNext(bstep::ENVELOPE));
    // Another vault, which holds the same key, is opened, and is first.
    app.press(Action::Nav(Screen::Vaults));
    for n in 0..4 {
        if app.vaults.open.len() > 1 {
            break;
        }
        app.press(Action::Vault(faraday_core::vaults::VaultAction::Open(n)));
        unlock(&mut app, PHRASE_2);
    }
    assert_eq!(app.vaults.open.len(), 2, "{:?}", app.vaults.unlock_error);
    app.vaults.open.swap(0, 1);
    shell.flow(&mut app, F::WalletLock);
    let app = restart(&mut shell, &mut app);
    assert_eq!(
        app.backup_flows[0].plan.vaults[0].salt,
        Some(own),
        "the flow's vault, not the other"
    );
}

#[test]
fn two_wallets_in_vaults_keep_each_its_own_vaults_and_a_flow_with_no_owner_matches_nothing() {
    let mut shell = Shell::new();
    sticks(&mut shell, 2);
    let vb = vault("Second", &[PHRASE_2], 0x43);
    let mut app = process(vec![("second.ofv".into(), vb)], Vec::new(), Vec::new());
    keep_time(&mut app);
    // Wallet A: its vault made and open.
    create_with_vaults(&mut app, false, 1);
    bring_in_all(&mut shell, &mut app, &["S1"]);
    create_vault(&mut shell, &mut app, 0, PHRASE);
    app.press(Action::Nav(Screen::Home));
    // Wallet B in vaults: its flow, and a vault of its own.
    app.press(Action::CreateWallet);
    app.press(Action::CVaults(true));
    app.press(Action::CNext(cstep::WHERE));
    app.press(Action::CNext(cstep::HOW_MANY));
    assert_eq!(app.backup_flows.len(), 2);
    let owners: Vec<u64> = app
        .backup_flows
        .iter()
        .map(|f| f.wallet.as_ref().unwrap().owner)
        .collect();
    assert!(owners.iter().all(|&o| o != 0) && owners[0] != owners[1]);
    app.press(Action::Nav(Screen::Vaults));
    for n in 0..4 {
        if app.vaults.open.len() > 1 {
            break;
        }
        app.press(Action::Vault(faraday_core::vaults::VaultAction::Open(n)));
        unlock(&mut app, PHRASE_2);
    }
    assert_eq!(app.vaults.open.len(), 2, "{:?}", app.vaults.unlock_error);
    let sb = app.vaults.open[1].header().salt;
    app.backup_flows[1].step = Step::Make;
    app.backup_flows[1].plan.vaults[0].salt = Some(sb);
    let fp = key_in(&mut app, 1);
    app.press(Action::Nav(Screen::Create));
    app.press(Action::CSlotHere(0, fp));
    app.press(Action::CNext(cstep::KEYS));
    app.press(Action::CNext(cstep::CHECK));
    app.press(Action::CBackup(1));
    untick_everywhere(&mut app, Row::Words);
    app.press(Action::BChecklist);
    save_vaults(&mut app);
    let n: Vec<usize> = app
        .vaults
        .open
        .iter()
        .map(|o| o.contents.of(kind::KEY).count())
        .collect();
    assert_eq!(n, vec![0, 1], "B's key is in B's vault and in A's not");
}

#[test]
fn a_done_flow_still_waiting_hands_its_hold_to_the_next_backup() {
    let mut shell = Shell::new();
    let (mut app, _, fp) = single_key(&mut shell, true);
    let mut d = Disk::empty("S4", "KEYSTICK");
    d.serial = Some(4);
    shell.add(d);
    shell.flow(&mut app, F::WalletLock);
    let mut app = restart(&mut shell, &mut app);
    write_sticks(&mut shell, &mut app, &["S1", "S2", "S3"]);
    assert_eq!(step(&app), Step::Done);
    // A copy is gone: the flow is done and waits again.
    app.backup_flows[0].slots[0].written = None;
    assert!(app.backup_flows[0].waiting());
    shell.flow(&mut app, F::UnlockDone(0));
    app.vaults.ms_per_unit = Some(180);
    unlock(&mut app, PHRASE);
    assert_eq!(app.screen, Screen::Backup);
    app.press(Action::BNext(bstep::ENVELOPE));
    shell.flow(&mut app, F::WalletLock);
    let app = restart(&mut shell, &mut app);
    let held = &app.backup_flows[0].wallet.as_ref().unwrap().held;
    let t = fp.iter().map(|b| format!("{b:02X}")).collect::<String>();
    assert!(
        held.iter().any(|h| h.eq_ignore_ascii_case(&t)),
        "the new flow holds what the old one held: {held:?}"
    );
}

#[test]
fn add_a_passphrase_acts_on_the_wallet_shown_not_the_one_a_backup_was_left_on() {
    let mut shell = Shell::new();
    let _ = &mut shell;
    let mut app = process(Vec::new(), Vec::new(), Vec::new());
    keep_time(&mut app);
    let mut fps = Vec::new();
    for n in [1usize, 2] {
        app.press(Action::CreateWallet);
        let fp = key_in(&mut app, n);
        app.press(Action::Nav(Screen::Create));
        app.press(Action::CSlotHere(0, fp));
        app.press(Action::CNext(cstep::KEYS));
        assert!(app.create.as_ref().unwrap().built.is_some());
        fps.push(fp);
    }
    assert_eq!(app.session.wallets.len(), 2);
    // A backup is left open on the second wallet; the first is shown.
    app.press(Action::Backup(1));
    app.press(Action::Nav(Screen::Wallets));
    app.wallet = 0;
    app.press(Action::PlanAddPass);
    let c = app.create.as_ref().unwrap();
    assert_eq!(c.slots, vec![faraday_core::create::Source::Here(fps[0])]);
}

#[test]
fn a_plan_change_after_the_lock_starts_its_round_without_continue() {
    let mut shell = Shell::new();
    let mut app = two_of_three_ready(&mut shell, false);
    shell.flow(&mut app, F::WalletLock);
    let mut app = restart(&mut shell, &mut app);
    shell.flow(&mut app, F::ChangePlan);
    flow_vault_add(&mut shell, &mut app, 1, 0);
    let f = app.flow().unwrap();
    assert!(f.adding, "no Continue was pressed");
    assert_eq!(f.step, Step::Vaults);
}

#[test]
fn a_passphrase_put_into_a_vault_after_the_lock_starts_a_round_too() {
    let mut shell = Shell::new();
    let mut app = two_of_three_ready_with(&mut shell, true);
    shell.flow(&mut app, F::WalletLock);
    let mut app = restart(&mut shell, &mut app);
    assert_eq!(step(&app), Step::Write);
    shell.flow(&mut app, F::ChangePlan);
    let places = app.flow().unwrap().wallet.as_ref().unwrap().answers.places;
    let in_vault = |app: &Faraday| {
        app.flow()
            .unwrap()
            .wallet
            .as_ref()
            .unwrap()
            .answers
            .pass_in_vault(0)
    };
    assert!(!in_vault(&app), "kept apart at the lock");
    let _ = places;
    // Into the vault that holds it, on that vault's sheet.
    let v = {
        let wf = app.flow().unwrap().wallet.as_ref().unwrap();
        (0..wf.answers.vault_count(&wf.shape))
            .find(|&v| wf.answers.vault_seeds(&wf.shape, v).contains(&0))
            .unwrap()
    };
    let p = flow_place_of(&app, v);
    flow_plan(
        &mut shell,
        &mut app,
        PlanPress::Row(p as u8, Row::Vault(v).code()),
    );
    shell.press(&mut app, Action::BPickPass(0));
    shell.press(&mut app, Action::BPickDone);
    assert!(in_vault(&app), "{:?}", app.toast_text());
    assert!(app.flow().unwrap().adding, "its vault must hold it now");
}

/// The sticks the setup of the flow on screen asks for, as disks.
fn flow_names(app: &Faraday) -> Vec<String> {
    (1..=app.flow().unwrap().slots.len())
        .map(|k| format!("S{k}"))
        .collect()
}

#[test]
fn answering_no_goes_to_the_keys_with_no_stick_or_vault_step_and_back_up_asks_the_rest() {
    let mut app = process(Vec::new(), Vec::new(), Vec::new());
    keep_time(&mut app);
    app.press(Action::CreateWallet);
    app.press(Action::CNext(cstep::KIND));
    app.press(Action::CVaults(false));
    app.press(Action::CNext(cstep::WHERE));
    assert_eq!(app.create.as_ref().unwrap().open, Some(cstep::KEYS));
    assert_eq!(app.screen, Screen::Create);
    assert!(app.backup_flows.is_empty(), "no stick or vault step");
    let cards = app.create_cards();
    assert!(
        ![cstep::HOW_MANY, cstep::STICKS, cstep::VAULTS]
            .iter()
            .any(|k| cards.contains(k)),
        "{cards:?}"
    );
    new_key(&mut app);
    if app.create.as_ref().unwrap().built.is_none() {
        app.press(Action::CNext(cstep::KEYS));
    }
    app.press(Action::CNext(cstep::CHECK));
    app.press(Action::CBackup(1));
    assert_eq!(app.screen, Screen::Backup);
    // The choices of the backup are asked in Back up, after the keys:
    // the forms of the seeds, the places, the paper.
    let b = app.backup.as_ref().unwrap();
    assert!(b.answers.seeds[seeds::VAULT] && b.answers.paper_seeds());
    assert!(b.answers.places >= 1);
}

#[test]
fn two_vaults_ask_for_two_sticks_read_before_any_unlock_and_are_open_before_the_keys() {
    let mut shell = Shell::new();
    sticks(&mut shell, 3);
    let main = vault("Main", &[PHRASE], 0x41);
    shell.disk_mut("S1").files = vec![("main.ofv".into(), main)];
    let mut app = process(Vec::new(), Vec::new(), Vec::new());
    keep_time(&mut app);
    create_with_vaults(&mut app, false, 2);
    assert_eq!(app.flow().unwrap().slots.len(), 2, "a stick for each vault");
    expect(&mut app, "Insert the stick for Vault 1");
    // Vault 1: a stick that holds a vault; the person picks which.
    shell.plug(&mut app, "S1");
    expect(&mut app, "Which vault is Vault 1?");
    expect(&mut app, "A new vault beside them");
    let k = app
        .flow()
        .unwrap()
        .live
        .reading
        .as_ref()
        .unwrap()
        .files
        .iter()
        .position(|(f, _)| f.name == "main.ofv")
        .unwrap();
    shell.flow(&mut app, F::PickFile(0, k));
    shell.flow(&mut app, F::Tap);
    shell.pull(&mut app, "S1");
    assert!(app.vaults.open.is_empty(), "no vault is opened yet");
    expect(&mut app, "Insert the stick for Vault 2");
    // Vault 2: an empty stick, a new vault, no question.
    shell.plug(&mut app, "S2");
    shell.flow(&mut app, F::Tap);
    shell.pull(&mut app, "S2");
    assert!(
        app.vaults.open.is_empty(),
        "every stick is read before any vault is unlocked"
    );
    assert_eq!(step(&app), Step::Vaults);
    unlock(&mut app, PHRASE);
    create_vault(&mut shell, &mut app, 1, PHRASE_2);
    // Both vaults are open before the keys, inside the flow.
    assert_eq!(app.vaults.open.len(), 2, "{:?}", app.vaults.unlock_error);
    assert_eq!(app.screen, Screen::Create);
    assert_eq!(app.create.as_ref().unwrap().open, Some(cstep::KEYS));
    no_visit(&shell);
}

#[test]
fn back_ups_plan_offers_the_open_vaults_and_a_vault_at_two_places_writes_both_copies() {
    let mut shell = Shell::new();
    sticks(&mut shell, 3);
    let mut app = process(Vec::new(), Vec::new(), Vec::new());
    keep_time(&mut app);
    create_with_vaults(&mut app, false, 2);
    bring_in_all(&mut shell, &mut app, &["S1", "S2"]);
    create_vault(&mut shell, &mut app, 0, PHRASE);
    create_vault(&mut shell, &mut app, 1, PHRASE_2);
    let fp = key_in(&mut app, 1);
    app.press(Action::Nav(Screen::Create));
    app.press(Action::CSlotHere(0, fp));
    app.press(Action::CNext(cstep::KEYS));
    app.press(Action::CNext(cstep::CHECK));
    app.press(Action::CBackup(1));
    // The plan offers both open vaults, though there is one key: a row
    // each on a place's card.
    assert_eq!(app.backup.as_ref().unwrap().answers.vaults.len(), 2);
    app.press(Action::BQ(faraday_core::qstep::place(0)));
    expect_part(&mut app, "Vault 2");
    // Key 1 in Vault 1, at two places; no paper words, no sheet.
    untick_everywhere(&mut app, Row::Words);
    form_off_everywhere(&mut app, Form::Paper);
    {
        let a = &app.backup.as_ref().unwrap().answers;
        assert!(a.vault_seeds(&app.plan_shape(0), 0).contains(&0));
    }
    let places = app.backup.as_ref().unwrap().answers.places;
    for p in 0..places {
        if !app.backup.as_ref().unwrap().answers.stick_at(0, p) {
            stick_also_at(&mut app, p, 0);
        }
    }
    assert!(places >= 2);
    app.press(Action::BChecklist);
    save_vaults(&mut app);
    app.press(Action::BNext(bstep::ENVELOPE));
    shell.flow(&mut app, F::WalletLock);
    let mut app = restart(&mut shell, &mut app);
    assert_eq!(step(&app), Step::Write);
    let f = app.flow().unwrap();
    assert_eq!(f.slots.len(), places, "a stick for each place");
    // The stick read for Vault 1 is the first; the rest are new sticks,
    // checked when they are inserted.
    assert!(f.slots[0].bound.is_some());
    assert!(f.slots[1..].iter().all(|s| s.bound.is_none()));
    let disks = ["S1", "S3"];
    write_sticks(&mut shell, &mut app, &disks[..places.min(2)]);
    assert_eq!(
        step(&app),
        Step::Done,
        "{:?}",
        app.flow().unwrap().live.problem
    );
    assert!(ofv(&shell, "S1").is_some() && ofv(&shell, "S3").is_some());
    assert_eq!(ofv(&shell, "S1"), ofv(&shell, "S3"), "the same sealed copy");
    assert!(
        ofv(&shell, "S2").is_none(),
        "Vault 2 holds nothing: not written"
    );
    no_visit(&shell);
}

#[test]
fn the_question_of_vaults_is_not_done_before_it_is_answered_and_continued() {
    let mut app = process(Vec::new(), Vec::new(), Vec::new());
    keep_time(&mut app);
    app.press(Action::CreateWallet);
    assert!(!app.create_card_done(cstep::WHERE));
    // Kind's Continue opens the question, and it is not done.
    app.press(Action::CNext(cstep::KIND));
    assert_eq!(app.create.as_ref().unwrap().open, Some(cstep::WHERE));
    assert!(!app.create_card_done(cstep::WHERE));
    // Unanswered, Continue does nothing.
    app.press(Action::CNext(cstep::WHERE));
    assert!(!app.create_card_done(cstep::WHERE));
    assert_eq!(app.create.as_ref().unwrap().open, Some(cstep::WHERE));
    // Answered but not continued: still not done.
    app.press(Action::CVaults(false));
    assert!(!app.create_card_done(cstep::WHERE));
    app.press(Action::CNext(cstep::WHERE));
    assert!(app.create_card_done(cstep::WHERE));
    assert_eq!(app.create.as_ref().unwrap().open, Some(cstep::KEYS));
    // The same on Restore.
    app.press(Action::RestoreWallet);
    assert!(!app.restore.as_ref().unwrap().done[faraday_core::rstep::WHERE as usize]);
    app.press(Action::RNext(faraday_core::rstep::KIND));
    app.press(Action::RNext(faraday_core::rstep::DESCRIPTION));
    assert_eq!(
        app.restore.as_ref().unwrap().open,
        Some(faraday_core::rstep::WHERE)
    );
    app.press(Action::RNext(faraday_core::rstep::WHERE));
    assert!(!app.restore.as_ref().unwrap().done[faraday_core::rstep::WHERE as usize]);
    app.press(Action::RVaults(false));
    app.press(Action::RNext(faraday_core::rstep::WHERE));
    assert!(app.restore.as_ref().unwrap().done[faraday_core::rstep::WHERE as usize]);
}

#[test]
fn the_count_of_vaults_is_one_to_1024_and_nothing_starts_before_its_continue() {
    let mut app = process(Vec::new(), Vec::new(), Vec::new());
    keep_time(&mut app);
    app.press(Action::CreateWallet);
    app.press(Action::CNext(cstep::KIND));
    app.press(Action::CVaults(true));
    app.press(Action::CNext(cstep::WHERE));
    assert_eq!(app.create.as_ref().unwrap().open, Some(cstep::HOW_MANY));
    assert!(!app.create_card_done(cstep::HOW_MANY));
    assert!(
        app.backup_flows.is_empty(),
        "nothing starts before Continue"
    );
    app.press(Action::CVaultN(0));
    assert_eq!(app.create.as_ref().unwrap().vaults.count, 1, "one at least");
    app.press(Action::CVaultN(25));
    assert_eq!(app.create.as_ref().unwrap().vaults.count, 25);
    // A count a saved plan could not read back is refused.
    app.press(Action::CVaultN(5000));
    assert_eq!(app.create.as_ref().unwrap().vaults.count, 1024);
    app.press(Action::CVaultN(25));
    assert!(!app.create_card_done(cstep::HOW_MANY));
    app.press(Action::CNext(cstep::HOW_MANY));
    assert!(app.create_card_done(cstep::HOW_MANY));
    assert_eq!(app.screen, Screen::BackupFlow);
    assert_eq!(app.flow().unwrap().slots.len(), 25);
    assert_eq!(step(&app), Step::Sticks);
    assert!(!app.create_card_done(cstep::STICKS) && !app.create_card_done(cstep::VAULTS));
}

#[test]
fn restore_asks_the_same_question_and_opens_its_vaults_before_the_seeds() {
    let mut shell = Shell::new();
    sticks(&mut shell, 3);
    let mut app = process(Vec::new(), Vec::new(), Vec::new());
    keep_time(&mut app);
    app.press(Action::RestoreWallet);
    app.press(Action::RVaults(true));
    app.press(Action::RNext(faraday_core::rstep::WHERE));
    assert_eq!(
        app.restore.as_ref().unwrap().open,
        Some(faraday_core::rstep::HOW_MANY)
    );
    app.press(Action::RVaultN(2));
    app.press(Action::RNext(faraday_core::rstep::HOW_MANY));
    assert_eq!(app.screen, Screen::BackupFlow);
    let names = flow_names(&app);
    let names: Vec<&str> = names.iter().map(String::as_str).collect();
    assert_eq!(names.len(), 2);
    bring_in_all(&mut shell, &mut app, &names);
    assert!(app.vaults.open.is_empty(), "read before any unlock");
    create_vault(&mut shell, &mut app, 0, PHRASE);
    create_vault(&mut shell, &mut app, 1, PHRASE_2);
    assert_eq!(app.screen, Screen::Restore, "{:?}", app.vaults.unlock_error);
    assert_eq!(app.vaults.open.len(), 2, "open before the seeds are typed");
    app.press(Action::RSlotWords);
    type_text(&mut app, &testkit::test_words(testkit::TEST_SEEDS[1].0));
    app.press(Action::EntryAdd);
    app.press(Action::Seeds(faraday_core::seeds::SeedsAction::Make));
    let w = app.restore.as_ref().unwrap().wallet.expect("wallet made");
    app.press(Action::Backup(w));
    app.press(Action::BPreset(0));
    assert_eq!(app.backup.as_ref().unwrap().answers.vaults.len(), 2);
}

/// A 2-of-3 with `count` vaults set up and its three keys made, on Back
/// up's card with the preset "Paper and vault" chosen.
fn two_of_three_with_setup(shell: &mut Shell, count: usize) -> Faraday {
    sticks(shell, count);
    let mut app = process(Vec::new(), Vec::new(), Vec::new());
    keep_time(&mut app);
    create_with_vaults(&mut app, true, count);
    let disks = ["S1", "S2", "S3"];
    bring_in_all(shell, &mut app, &disks[..count]);
    let phrases = [PHRASE, PHRASE_2, PHRASE_3];
    for (v, phrase) in phrases.iter().enumerate().take(count) {
        create_vault(shell, &mut app, v, phrase);
    }
    let mut fps = Vec::new();
    for i in 0..3 {
        fps.push(key_in(&mut app, i));
    }
    app.press(Action::Nav(Screen::Create));
    for (i, fp) in fps.iter().enumerate() {
        app.press(Action::CSlotHere(i as u8, *fp));
    }
    app.press(Action::CNext(cstep::KEYS));
    app.press(Action::CNext(cstep::CHECK));
    app.press(Action::CBackup(1));
    app
}

#[test]
fn a_plan_vault_with_none_set_up_offers_to_make_one_while_another_is_open() {
    use faraday_core::vaults::VaultAction as V;
    let mut shell = Shell::new();
    let mut app = two_of_three_with_setup(&mut shell, 1);
    untick_everywhere(&mut app, Row::Words);
    app.press(Action::BChecklist);
    let items = app.backup_items();
    assert!(items.contains(&(bstep::VAULT + 1)), "{items:?}");
    assert!(!app.backup_item_done(Item::Vault(1)));
    app.press(Action::BVaultSave(0));
    assert!(app.backup_item_done(Item::Vault(0)));
    app.press(Action::BStep(bstep::VAULT + 1));
    let make = Action::Vault(V::CreateFrom(Screen::Backup));
    let mut found = false;
    for _ in 0..20 {
        app.settle();
        let _ = app.frame();
        if app.offers(make) {
            found = true;
            break;
        }
        app.event(Event::Scroll {
            x: 400,
            y: 384,
            dy: 300,
        });
    }
    let shown = texts(&mut app);
    assert!(
        found,
        "Make a vault is not offered for Vault 2: open {:?} cur {} items {:?} {:?}",
        app.backup.as_ref().unwrap().open,
        app.vaults.current,
        app.backup_items(),
        shown
    );
    app.press(make);
    for second in [false, true] {
        app.press(Action::Vault(V::CFocus(0, second)));
        type_text(&mut app, PHRASE_2);
    }
    app.press(Action::Vault(V::CGo));
    settle(&mut app, 10);
    settle(&mut app, 40);
    if app.screen == Screen::Unlock {
        type_text(&mut app, PHRASE_2);
        app.press(Action::Vault(V::Unlock));
        settle(&mut app, 100);
    }
    assert_eq!(app.vaults.open.len(), 2, "the new vault is open");
    app.press(Action::BVaultSave(1));
    assert!(app.backup_item_done(Item::Vault(1)));
}

#[test]
fn the_vault_set_up_for_a_plan_vault_is_not_locked_away_for_another_one_that_holds_a_key() {
    use faraday_core::vaults::VaultAction as V;
    let mut shell = Shell::new();
    let mut app = two_of_three_with_setup(&mut shell, 2);
    untick_everywhere(&mut app, Row::Words);
    // Vault 1 holds Key 1, Vault 2 Keys 2 and 3; the vault set up for
    // Vault 1 is given Key 2 too.
    let p = vault_place(&app, 0);
    keep_at(&mut app, p, Row::Vault(0), &[0]);
    let p = vault_place(&app, 1);
    keep_at(&mut app, p, Row::Vault(1), &[1, 2]);
    untick_everywhere(&mut app, Row::Vault(2));
    app.press(Action::BChecklist);
    app.vaults.current = 0;
    app.press(Action::Vault(V::SaveKey(1)));
    // Vault 2's vault is the one selected when the button is pressed.
    app.vaults.current = 1;
    let both = app.vaults.open.len();
    app.press(Action::BNewVault(0));
    assert_eq!(app.vaults.open.len(), both, "no vault was locked away");
    assert!(app.toast_text().is_some());
}

#[test]
fn a_setup_survives_the_lock_that_makes_the_process_clean() {
    let mut shell = Shell::new();
    sticks(&mut shell, 2);
    let mut app = process(Vec::new(), Vec::new(), Vec::new());
    keep_time(&mut app);
    app.press(Action::CreateWallet);
    // A key put in a slot before the question was answered.
    key_in(&mut app, 0);
    app.press(Action::Nav(Screen::Create));
    app.press(Action::CKind(multi_index()));
    app.press(Action::CVaults(true));
    app.press(Action::CNext(cstep::WHERE));
    app.press(Action::CVaultN(2));
    app.press(Action::CNext(cstep::HOW_MANY));
    assert_eq!(app.screen, Screen::BackupFlow);
    shell.flow(&mut app, F::LockBring);
    let mut app = restart(&mut shell, &mut app);
    // The wallet being made is there again, past its questions.
    let c = app.create.as_ref().expect("Make a new wallet is kept");
    assert_eq!(c.kind, NewKind::Multi);
    assert_eq!((c.vaults.answer, c.vaults.count), (Some(true), 2));
    assert!(app.create_card_done(cstep::WHERE) && app.create_card_done(cstep::HOW_MANY));
    assert_eq!(app.screen, Screen::BackupFlow);
    // Keys stay closed until the setup is done.
    app.press(Action::Nav(Screen::Create));
    app.press(Action::CStep(cstep::KEYS));
    assert_ne!(app.create.as_ref().unwrap().open, Some(cstep::KEYS));
    let flow = flow_names(&app);
    let names: Vec<&str> = flow.iter().map(String::as_str).collect();
    app.press(Action::Nav(Screen::BackupFlow));
    bring_in_all(&mut shell, &mut app, &names);
    create_vault(&mut shell, &mut app, 0, PHRASE);
    create_vault(&mut shell, &mut app, 1, PHRASE_2);
    assert_eq!(app.screen, Screen::Create);
    assert_eq!(app.create.as_ref().unwrap().open, Some(cstep::KEYS));
}

#[test]
fn a_restore_setup_survives_the_lock_too() {
    let mut shell = Shell::new();
    sticks(&mut shell, 1);
    let mut app = process(Vec::new(), Vec::new(), Vec::new());
    keep_time(&mut app);
    app.press(Action::RestoreWallet);
    key_in(&mut app, 0);
    app.press(Action::Nav(Screen::Restore));
    app.press(Action::RVaults(true));
    app.press(Action::RNext(faraday_core::rstep::WHERE));
    app.press(Action::RVaultN(1));
    app.press(Action::RNext(faraday_core::rstep::HOW_MANY));
    shell.flow(&mut app, F::LockBring);
    let mut app = restart(&mut shell, &mut app);
    let r = app.restore.as_ref().expect("Restore is kept");
    assert_eq!((r.vaults.answer, r.vaults.count), (Some(true), 1));
    app.press(Action::Nav(Screen::Restore));
    app.press(Action::RStep(faraday_core::rstep::SEEDS));
    assert_ne!(
        app.restore.as_ref().unwrap().open,
        Some(faraday_core::rstep::SEEDS),
        "the seeds stay closed until the setup is done"
    );
}

#[test]
fn the_question_and_the_count_are_fixed_once_a_vault_of_the_setup_is_open() {
    let mut shell = Shell::new();
    sticks(&mut shell, 1);
    let mut app = process(Vec::new(), Vec::new(), Vec::new());
    keep_time(&mut app);
    create_with_vaults(&mut app, false, 1);
    bring_in_all(&mut shell, &mut app, &["S1"]);
    create_vault(&mut shell, &mut app, 0, PHRASE);
    assert_eq!(app.vaults.open.len(), 1);
    app.press(Action::Nav(Screen::Create));
    app.press(Action::CVaults(false));
    app.press(Action::CVaultN(3));
    let c = app.create.as_ref().unwrap();
    assert_eq!((c.vaults.answer, c.vaults.count), (Some(true), 1));
    assert!(!app.backup_flows.is_empty(), "the setup stays");
}

#[test]
fn a_wallet_made_without_the_question_continued_does_not_read_it_as_done() {
    let mut app = process(Vec::new(), Vec::new(), Vec::new());
    keep_time(&mut app);
    app.press(Action::CreateWallet);
    let fp = key_in(&mut app, 0);
    app.press(Action::Nav(Screen::Create));
    app.press(Action::CSlotHere(0, fp));
    app.press(Action::CNext(cstep::KEYS));
    assert!(app.create.as_ref().unwrap().built.is_some());
    assert!(!app.create_card_done(cstep::WHERE));
    assert!(!app.create_card_done(cstep::HOW_MANY));
}

// ---- the review of the plan by place (`docs/BACKUP-FLOW.md` §17) ----

/// The guided flow's own plan card, Back up's cards, edits the map the
/// cards made: what they put elsewhere stays.
#[test]
fn the_flows_plan_card_edits_the_map_the_cards_made() {
    let mut shell = Shell::new();
    let (mut app, _, _) = single_key_ready(&mut shell);
    shell.flow(&mut app, F::WalletLock);
    let mut app = restart(&mut shell, &mut app);
    assert_eq!(step(&app), Step::Write);
    shell.flow(&mut app, F::ChangePlan);
    // The seed out of Vault 1, on its sheet.
    let p = flow_place_of(&app, 0);
    flow_keep(&mut shell, &mut app, p, Row::Vault(0), &[]);
    let wf = app.flow().unwrap().wallet.as_ref().unwrap();
    let a = &wf.answers;
    assert!(a.map.is_some(), "the map stays the plan");
    let m = plan::map(&wf.shape, a);
    let at = |at: At| -> Vec<What> {
        m.iter()
            .filter(|s| s.at == at)
            .flat_map(|s| s.holds.iter().map(|(w, _)| *w))
            .collect()
    };
    assert!(a.vault_seeds(&wf.shape, 0).is_empty(), "no seed in a vault");
    assert!(
        at(At::Place(1)).contains(&What::WalletFile),
        "the description's files stay at Place 2"
    );
    assert!(
        !m.iter()
            .any(|s| s.holds.iter().any(|(w, _)| matches!(w, What::Words(_)))),
        "no words came back"
    );
}

/// A key kept only as a file at a place is held until that place's stick
/// of files is written, and released once it is.
#[test]
fn a_seed_kept_as_a_file_at_a_place_is_released_once_that_stick_is_written() {
    let mut shell = Shell::new();
    sticks(&mut shell, 1);
    let mut app = process(Vec::new(), Vec::new(), Vec::new());
    keep_time(&mut app);
    app.press(Action::CreateWallet);
    let fp = new_key_here(&mut app);
    app.press(Action::CBackup(1));
    untick_everywhere(&mut app, Row::Vault(0));
    untick_everywhere(&mut app, Row::Words);
    keep_at(&mut app, 1, Row::File, &[0]);
    form_off_everywhere(&mut app, Form::Paper);
    app.press(Action::BChecklist);
    app.press(Action::BFile);
    app.press(Action::SecretAck);
    app.press(Action::SecretUnprotected);
    app.press(Action::BNext(bstep::ENVELOPE));
    let fp = osk_bip::keys::Fingerprint(fp);
    assert!(app.key_held(fp), "held for Place 2's stick of files");
    shell.flow(&mut app, F::WalletLock);
    let mut app = restart(&mut shell, &mut app);
    assert_eq!(app.flow().unwrap().slots[0].target, Target::PlaceFiles(1));
    write_sticks(&mut shell, &mut app, &["S1"]);
    assert_eq!(step(&app), Step::Done);
    assert!(!app.key_held(fp), "released once its stick is written");
}

/// Each seed's file at a place of its own, all three let out: each
/// place's stick of files is written with that place's seed file alone.
#[test]
fn each_places_stick_of_files_takes_only_that_places_seed_file() {
    let mut shell = Shell::new();
    sticks(&mut shell, 3);
    let mut app = process(Vec::new(), Vec::new(), Vec::new());
    keep_time(&mut app);
    app.press(Action::CreateWallet);
    app.press(Action::CKind(multi_index()));
    let fps: Vec<[u8; 4]> = (0..3).map(|i| key_in(&mut app, i)).collect();
    app.press(Action::Nav(Screen::Create));
    for (i, fp) in fps.iter().enumerate() {
        app.press(Action::CSlotHere(i as u8, *fp));
    }
    app.press(Action::CNext(cstep::KEYS));
    app.press(Action::CNext(cstep::CHECK));
    app.press(Action::CBackup(0));
    untick_everywhere(&mut app, Row::Words);
    form_off_everywhere(&mut app, Form::Paper);
    for p in 0..3 {
        keep_at(&mut app, p, Row::File, &[p]);
    }
    let seed_names: Vec<String> = backup_shape(&app)
        .seeds
        .iter()
        .map(|s| s.name.to_lowercase())
        .collect();
    app.press(Action::BChecklist);
    assert_eq!(
        app.backup_items(),
        vec![bstep::FILES, bstep::ENVELOPE, bstep::STICKS]
    );
    for _ in 0..3 {
        app.press(Action::BFile);
        app.press(Action::SecretAck);
        app.press(Action::SecretUnprotected);
    }
    assert_eq!(app.outbox.iter().filter(|i| i.secret).count(), 3);
    app.press(Action::BNext(bstep::ENVELOPE));
    shell.flow(&mut app, F::WalletLock);
    let mut app = restart(&mut shell, &mut app);
    let targets: Vec<Target> = app.flow().unwrap().slots.iter().map(|s| s.target).collect();
    assert_eq!(
        targets,
        vec![
            Target::PlaceFiles(0),
            Target::PlaceFiles(1),
            Target::PlaceFiles(2)
        ]
    );
    write_sticks(&mut shell, &mut app, &["S1", "S2", "S3"]);
    assert_eq!(
        step(&app),
        Step::Done,
        "{:?}",
        app.flow().unwrap().live.problem
    );
    for (p, d) in ["S1", "S2", "S3"].iter().enumerate() {
        let words: Vec<String> = names(&shell, d)
            .into_iter()
            .filter(|n| n.ends_with("-words.txt"))
            .collect();
        assert_eq!(words.len(), 1, "{d}: {words:?}");
        assert!(words[0].contains(&seed_names[p]), "{d}: {words:?}");
    }
    no_visit(&shell);
}

/// A share kept in a vault (`docs/BACKUP-FLOW.md` §17.6): Save into the
/// vault puts it there as a type 13 record, the item is done only then,
/// the sealed vault holds it after the lock, and Restore with that vault
/// open and another share in the Inbox rebuilds the wallet.
#[test]
fn a_share_kept_in_a_vault_is_saved_sealed_and_rebuilds_with_another() {
    use faraday_vault::records::field;
    let mut shell = Shell::new();
    sticks(&mut shell, 1);
    let mut app = process(Vec::new(), Vec::new(), Vec::new());
    keep_time(&mut app);
    create_with_vaults(&mut app, true, 1);
    bring_in_all(&mut shell, &mut app, &["S1"]);
    create_vault(&mut shell, &mut app, 0, PHRASE);
    let fps: Vec<[u8; 4]> = (0..3).map(|i| key_in(&mut app, i)).collect();
    app.press(Action::Nav(Screen::Create));
    for (i, fp) in fps.iter().enumerate() {
        app.press(Action::CSlotHere(i as u8, *fp));
    }
    app.press(Action::CNext(cstep::KEYS));
    let w = app.create.as_ref().unwrap().built.expect("wallet made");
    app.press(Action::CNext(cstep::CHECK));
    app.press(Action::CBackup(1));
    // In shares; nothing on paper; Vault 1 alone, holding Share 2.
    app.press(Action::BSplit(true));
    untick_everywhere(&mut app, Row::Words);
    let vaults = {
        let shape = backup_shape(&app);
        app.backup.as_ref().unwrap().answers.vault_count(&shape)
    };
    for v in 1..vaults {
        untick_everywhere(&mut app, Row::Vault(v));
    }
    form_off_everywhere(&mut app, Form::Vault(0));
    form_off_everywhere(&mut app, Form::Paper);
    form_off_everywhere(&mut app, Form::File);
    let p = vault_place(&app, 0);
    app.press(Action::BShare(p as u8, 1));
    app.press(Action::BForm(p as u8, Form::Vault(0).code()));
    form_off_everywhere(&mut app, Form::Paper);
    {
        let shape = backup_shape(&app);
        let a = &app.backup.as_ref().unwrap().answers;
        assert_eq!(a.vault_shares(&shape, 0), vec![1], "Share 2 in Vault 1");
    }
    let omit = app.backup.as_ref().unwrap().answers.omit;
    let (m, n) = faraday_core::wallet::Session::quorum(&app.session.wallets[w]);
    let split = faraday_core::backup::split_plan(n, m, omit);
    let texts: Vec<String> = split
        .iter()
        .map(|row| faraday_core::backup::split_share(&app.session.wallets[w], row).unwrap())
        .collect();
    let share = |j: usize| texts[j].clone();
    let made = app.session.wallets[w].policy.to_descriptor_checksummed();
    let stem = faraday_core::file_stem(&app.session.wallets[w].name);
    app.press(Action::BChecklist);
    assert_eq!(app.backup.as_ref().unwrap().stage, BStage::Checklist);
    assert!(!app.backup_item_done(Item::Vault(0)));
    app.press(Action::BVaultSave(0));
    let shares = |app: &Faraday| -> Vec<(String, Vec<u8>)> {
        app.vaults.open[0]
            .contents
            .of(kind::WALLET_SHARE)
            .map(|(_, r)| {
                (
                    r.text(field::SHARE).unwrap_or("").to_string(),
                    r.field(field::SHARE_NUMBER).unwrap_or(&[]).to_vec(),
                )
            })
            .collect()
    };
    assert_eq!(shares(&app), vec![(share(1), vec![2])]);
    assert!(app.backup_item_done(Item::Vault(0)));
    // Done only while the vault holds it; saved again, kept once.
    app.vaults.open[0]
        .contents
        .records
        .retain(|r| r.kind != kind::WALLET_SHARE);
    assert!(
        !app.backup_item_done(Item::Vault(0)),
        "the share is part of the item"
    );
    app.press(Action::BVaultSave(0));
    app.press(Action::BVaultSave(0));
    assert_eq!(shares(&app).len(), 1);
    assert!(app.backup_item_done(Item::Vault(0)));
    app.press(Action::BNext(bstep::ENVELOPE));
    shell.flow(&mut app, F::WalletLock);
    let mut app = restart(&mut shell, &mut app);
    write_sticks(&mut shell, &mut app, &["S1"]);
    assert_eq!(
        step(&app),
        Step::Done,
        "{:?}",
        app.flow().unwrap().live.problem
    );
    let sealed = ofv(&shell, "S1").expect("the vault's copy");
    let (_, c) = faraday_vault::open(&sealed, PHRASE.as_bytes()).unwrap();
    assert_eq!(c.of(kind::WALLET_SHARE).count(), 1, "sealed with the share");
    // Restore: that vault open, Share 1 in the Inbox.
    let name = format!("{stem}-share-1-of-{n}.txt");
    let mut r = process(
        vec![
            ("vault-1.ofv".into(), sealed),
            (name.clone(), share(0).into_bytes()),
        ],
        Vec::new(),
        Vec::new(),
    );
    keep_time(&mut r);
    r.press(Action::Nav(Screen::Vaults));
    r.press(Action::Vault(faraday_core::vaults::VaultAction::Open(0)));
    unlock(&mut r, PHRASE);
    assert_eq!(r.vaults.open.len(), 1);
    r.press(Action::RestoreWallet);
    let i = r.inbox.iter().position(|f| f.name == name).unwrap();
    r.press(Action::RShare(i));
    r.press(Action::RRebuild);
    let rw = r
        .restore
        .as_ref()
        .and_then(|x| x.wallet)
        .unwrap_or_else(|| panic!("{:?}", r.restore.as_ref().and_then(|x| x.error.clone())));
    assert_eq!(
        r.session.wallets[rw].policy.to_descriptor_checksummed(),
        made,
        "the same wallet, the same checksum"
    );
}

/// A share put into a vault after the lock, from Change the plan, changes
/// what the vault holds (`docs/BACKUP-FLOW.md` §10.2, §17.6): the vault
/// goes back to be filled and written, and the flow does not finish with
/// the share missing from its sealed copy.
#[test]
fn a_share_put_into_a_vault_after_the_lock_starts_the_round_and_is_sealed() {
    use faraday_vault::records::field;
    let mut shell = Shell::new();
    sticks(&mut shell, 1);
    let mut app = process(Vec::new(), Vec::new(), Vec::new());
    keep_time(&mut app);
    create_with_vaults(&mut app, true, 1);
    bring_in_all(&mut shell, &mut app, &["S1"]);
    create_vault(&mut shell, &mut app, 0, PHRASE);
    let fps: Vec<[u8; 4]> = (0..3).map(|i| key_in(&mut app, i)).collect();
    app.press(Action::Nav(Screen::Create));
    for (i, fp) in fps.iter().enumerate() {
        app.press(Action::CSlotHere(i as u8, *fp));
    }
    app.press(Action::CNext(cstep::KEYS));
    app.press(Action::CNext(cstep::CHECK));
    app.press(Action::CBackup(1));
    // Vault 1 alone, with its seed and the whole description; nothing on
    // paper.
    untick_everywhere(&mut app, Row::Words);
    let vaults = {
        let shape = backup_shape(&app);
        app.backup.as_ref().unwrap().answers.vault_count(&shape)
    };
    for v in 1..vaults {
        untick_everywhere(&mut app, Row::Vault(v));
    }
    form_off_everywhere(&mut app, Form::Paper);
    app.press(Action::BChecklist);
    save_vaults(&mut app);
    app.press(Action::BNext(bstep::ENVELOPE));
    shell.flow(&mut app, F::WalletLock);
    let mut app = restart(&mut shell, &mut app);
    assert_eq!(step(&app), Step::Write);
    // Change the plan: in shares, Share 2 in Vault 1.
    shell.flow(&mut app, F::ChangePlan);
    flow_plan(&mut shell, &mut app, PlanPress::Split(true));
    let p = flow_place_of(&app, 0);
    let pc = p as u8;
    let vault = Form::Vault(0).code();
    let in_vault = |app: &Faraday| {
        let wf = app.flow().unwrap().wallet.as_ref().unwrap();
        wf.answers.vault_shares(&wf.shape, 0)
    };
    let _ = vault;
    // The vault's shares only (§17.8): its form is the vault, so each
    // share ticked or unticked here goes into or out of it alone.
    for j in [1u8, 0, 2] {
        let here = {
            let wf = app.flow().unwrap().wallet.as_ref().unwrap();
            wf.answers
                .shares_here(&wf.shape, p)
                .contains(&usize::from(j))
        };
        if here != (j == 1) {
            flow_plan(&mut shell, &mut app, PlanPress::Share(pc, j));
        }
    }
    assert_eq!(in_vault(&app), vec![1], "Share 2 in Vault 1");
    assert_eq!(app.flow().unwrap().slots.len(), 1, "Vault 1's stick alone");
    assert!(app.flow().unwrap().adding, "a share added is a round");
    assert_eq!(step(&app), Step::Vaults);
    unlock(&mut app, PHRASE);
    assert_eq!(step(&app), Step::Make, "{:?}", app.vaults.unlock_error);
    shell.flow(&mut app, F::CopyKey);
    let numbers: Vec<Vec<u8>> = app.vaults.open[0]
        .contents
        .of(kind::WALLET_SHARE)
        .map(|(_, r)| r.field(field::SHARE_NUMBER).unwrap_or(&[]).to_vec())
        .collect();
    assert_eq!(numbers, vec![vec![2]], "{:?}", app.toast_text());
    shell.flow(&mut app, F::MadeContinue);
    assert_eq!(step(&app), Step::Lock);
    shell.flow(&mut app, F::LockWrite);
    let mut app = restart(&mut shell, &mut app);
    write_sticks(&mut shell, &mut app, &["S1"]);
    assert_eq!(
        step(&app),
        Step::Done,
        "{:?}",
        app.flow().unwrap().live.problem
    );
    let sealed = ofv(&shell, "S1").expect("Vault 1 on S1");
    let (_, c) = faraday_vault::open(&sealed, PHRASE.as_bytes()).unwrap();
    assert_eq!(
        c.of(kind::WALLET_SHARE).count(),
        1,
        "the sealed copy holds Share 2"
    );
    no_visit(&shell);
}

// ---- the second review (`docs/BACKUP-FLOW.md` §17.6, §17.7) ----

/// A single key whose Vault 1 is on one stick, at Place 2, with the
/// description's files at Place 2 too: made, saved, stopped on the
/// checklist's Write to sticks.
fn single_key_one_stick(shell: &mut Shell) -> Faraday {
    sticks(shell, 3);
    let mut app = process(Vec::new(), Vec::new(), Vec::new());
    keep_time(&mut app);
    create_with_vaults(&mut app, false, 1);
    bring_in_all(shell, &mut app, &["S1"]);
    create_vault(shell, &mut app, 0, PHRASE);
    let fp = key_in(&mut app, 1);
    app.press(Action::Nav(Screen::Create));
    app.press(Action::CSlotHere(0, fp));
    if app.create.as_ref().unwrap().built.is_none() {
        app.press(Action::CNext(cstep::KEYS));
    }
    app.press(Action::CNext(cstep::CHECK));
    app.press(Action::CBackup(1));
    untick_everywhere(&mut app, Row::Words);
    form_off_everywhere(&mut app, Form::Paper);
    form_on(&mut app, 1, Form::File);
    assert_eq!(vault_place(&app, 0), 1, "Vault 1's stick at Place 2 alone");
    app.press(Action::BChecklist);
    save_vaults(&mut app);
    app.press(Action::BFilesMake);
    app.press(Action::BNext(bstep::ENVELOPE));
    app
}

/// The row sheet's Done while a stick is written changes nothing of the
/// plan, and the write counts for the stick it was for.
#[test]
fn done_on_a_row_sheet_during_a_write_changes_nothing_and_the_write_counts_for_its_stick() {
    let mut shell = Shell::new();
    let mut app = single_key_one_stick(&mut shell);
    shell.flow(&mut app, F::WalletLock);
    let mut app = restart(&mut shell, &mut app);
    let targets = |app: &Faraday| -> Vec<Target> {
        app.flow().unwrap().slots.iter().map(|s| s.target).collect()
    };
    assert_eq!(targets(&app), vec![Target::Place(1), Target::PlaceFiles(1)]);
    shell.flow(&mut app, F::ChangePlan);
    // "Vault 1 · Place 1" open, then S1 in: its write starts.
    flow_plan(
        &mut shell,
        &mut app,
        PlanPress::Row(0, Row::Vault(0).code()),
    );
    assert!(app.choosing.is_some());
    shell.hold_writes = true;
    shell.plug(&mut app, "S1");
    assert!(
        app.flow().unwrap().live.writing.is_some(),
        "the write is under way"
    );
    let before = app.flow().unwrap().wallet.as_ref().unwrap().answers.clone();
    shell.press(&mut app, Action::BPickDone);
    assert_eq!(
        app.flow().unwrap().wallet.as_ref().unwrap().answers,
        before,
        "the plan does not change under a write"
    );
    assert_eq!(targets(&app), vec![Target::Place(1), Target::PlaceFiles(1)]);
    shell.release_writes(&mut app);
    let f = app.flow().unwrap();
    assert!(
        f.slots[0].written.is_some(),
        "S1 counts for Vault 1 at Place 2"
    );
    assert!(f.slots[1].written.is_none());
}

/// Two wallets of one name, each with its Share 2 in one vault: both
/// shares stay, and each wallet's vault item sees only its own.
#[test]
fn two_wallets_of_one_name_keep_each_its_own_share_in_one_vault() {
    use faraday_vault::records::field;
    let mut shell = Shell::new();
    sticks(&mut shell, 1);
    let mut app = process(Vec::new(), Vec::new(), Vec::new());
    keep_time(&mut app);
    create_with_vaults(&mut app, true, 1);
    bring_in_all(&mut shell, &mut app, &["S1"]);
    create_vault(&mut shell, &mut app, 0, PHRASE);
    let fps: Vec<[u8; 4]> = (0..3).map(|i| key_in(&mut app, i)).collect();
    app.press(Action::Nav(Screen::Create));
    for (i, fp) in fps.iter().enumerate() {
        app.press(Action::CSlotHere(i as u8, *fp));
    }
    app.press(Action::CNext(cstep::KEYS));
    let a = app.create.as_ref().unwrap().built.expect("wallet made");
    app.press(Action::CNext(cstep::CHECK));
    app.press(Action::CBackup(1));
    // Share 2 in Vault 1, as each of the two wallets plans it.
    let share_two_in_vault = |app: &mut Faraday| {
        app.press(Action::BSplit(true));
        untick_everywhere(app, Row::Words);
        let vaults = {
            let shape = backup_shape(app);
            app.backup.as_ref().unwrap().answers.vault_count(&shape)
        };
        for v in 1..vaults {
            untick_everywhere(app, Row::Vault(v));
        }
        let p = vault_place(app, 0);
        form_off_everywhere(app, Form::Vault(0));
        form_off_everywhere(app, Form::Paper);
        form_off_everywhere(app, Form::File);
        app.press(Action::BShare(p as u8, 1));
        app.press(Action::BForm(p as u8, Form::Vault(0).code()));
        form_off_everywhere(app, Form::Paper);
        app.press(Action::BChecklist);
        app.press(Action::BVaultSave(0));
    };
    share_two_in_vault(&mut app);
    assert!(app.backup_item_done(Item::Vault(0)));
    // Another wallet, of the same name.
    let name = app.session.wallets[a].name.clone();
    let keys: Vec<String> = (0..3)
        .map(|k| {
            NewKind::TapMulti
                .key_text(&app.session.keys[k].master)
                .unwrap()
        })
        .collect();
    let b = app
        .session
        .add_wallet(
            &name,
            &NewKind::TapMulti.descriptor(2, &keys),
            "test",
            faraday_core::wallet::Origin::Named,
        )
        .unwrap();
    assert_ne!(b, a);
    app.press(Action::Backup(b));
    app.press(Action::BPreset(1));
    share_two_in_vault(&mut app);
    assert!(app.backup_item_done(Item::Vault(0)), "B's share saved");
    let shares = |app: &Faraday| -> Vec<(String, Vec<u8>)> {
        app.vaults.open[0]
            .contents
            .of(kind::WALLET_SHARE)
            .map(|(_, r)| {
                (
                    r.text(field::SHARE_WALLET).unwrap_or("").to_string(),
                    r.field(field::SHARE_NUMBER).unwrap_or(&[]).to_vec(),
                )
            })
            .collect()
    };
    let both = shares(&app);
    assert_eq!(both.len(), 2, "both Share 2s stay");
    assert!(both.iter().all(|(_, n)| n == &vec![2]));
    assert_ne!(both[0].0, both[1].0, "each told by its wallet");
    // A's share taken out: B's item is still done, A's is not.
    let a_sum = app.session.wallets[a].policy.checksum();
    app.vaults.open[0].contents.records.retain(|r| {
        !(r.kind == kind::WALLET_SHARE
            && r.text(field::SHARE_WALLET)
                .and_then(|t| faraday_core::wallet::read_wallet(t).ok())
                .is_some_and(|p| p.checksum() == a_sum))
    });
    assert!(app.backup_item_done(Item::Vault(0)), "B sees its own share");
    app.press(Action::Backup(a));
    assert!(!app.backup_item_done(Item::Vault(0)), "A does not take B's");
    app.press(Action::BVaultSave(0));
    assert!(app.backup_item_done(Item::Vault(0)));
    assert_eq!(shares(&app).len(), 2);
}

/// "With its passphrase" unticked after the lock: a round starts, and
/// the sealed copy on S1 keeps the key without its passphrase.
#[test]
fn a_passphrase_taken_out_of_a_vault_after_the_lock_starts_a_round_and_leaves_it() {
    let mut shell = Shell::new();
    sticks(&mut shell, 1);
    let mut app = process(Vec::new(), Vec::new(), Vec::new());
    keep_time(&mut app);
    create_with_vaults(&mut app, false, 1);
    bring_in_all(&mut shell, &mut app, &["S1"]);
    create_vault(&mut shell, &mut app, 0, PHRASE);
    app.press(Action::Entry(None));
    for c in testkit::test_words(testkit::TEST_SEEDS[1].0).chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    app.press(Action::EntryPassphrase);
    type_text(&mut app, "kept apart");
    app.press(Action::EntryAdd);
    let fp = app.session.keys.last().unwrap().master.fingerprint().0;
    app.press(Action::Nav(Screen::Create));
    app.press(Action::CSlotHere(0, fp));
    if app.create.as_ref().unwrap().built.is_none() {
        app.press(Action::CNext(cstep::KEYS));
    }
    app.press(Action::CNext(cstep::CHECK));
    app.press(Action::CBackup(1));
    // The key in Vault 1 with its passphrase; nothing else.
    untick_everywhere(&mut app, Row::Words);
    untick_everywhere(&mut app, Row::Passphrase);
    form_off_everywhere(&mut app, Form::Paper);
    let p = vault_place(&app, 0);
    app.press(Action::BRow(p as u8, Row::Vault(0).code()));
    app.press(Action::BPickPass(0));
    app.press(Action::BPickDone);
    assert!(app.backup.as_ref().unwrap().answers.pass_in_vault(0));
    app.press(Action::BChecklist);
    save_vaults(&mut app);
    app.press(Action::BNext(bstep::ENVELOPE));
    shell.flow(&mut app, F::WalletLock);
    let mut app = restart(&mut shell, &mut app);
    assert_eq!(step(&app), Step::Write);
    shell.flow(&mut app, F::ChangePlan);
    let p = flow_place_of(&app, 0);
    flow_plan(
        &mut shell,
        &mut app,
        PlanPress::Row(p as u8, Row::Vault(0).code()),
    );
    shell.press(&mut app, Action::BPickPass(0));
    shell.press(&mut app, Action::BPickDone);
    assert!(app.flow().unwrap().adding, "{:?}", app.toast_text());
    unlock(&mut app, PHRASE);
    assert_eq!(step(&app), Step::Make, "{:?}", app.vaults.unlock_error);
    shell.flow(&mut app, F::CopyKey);
    shell.flow(&mut app, F::MadeContinue);
    shell.flow(&mut app, F::LockWrite);
    let mut app = restart(&mut shell, &mut app);
    write_sticks(&mut shell, &mut app, &["S1"]);
    assert_eq!(
        step(&app),
        Step::Done,
        "{:?}",
        app.flow().unwrap().live.problem
    );
    let sealed = ofv(&shell, "S1").expect("Vault 1 on S1");
    let (_, c) = faraday_vault::open(&sealed, PHRASE.as_bytes()).unwrap();
    let keys: Vec<bool> = c
        .of(kind::KEY)
        .map(|(_, r)| {
            r.field(faraday_vault::records::field::KEY_PASSPHRASE)
                .is_some()
        })
        .collect();
    assert_eq!(keys, vec![false], "the key, without its passphrase");
}

/// After the lock, Back up's own cards edit the flow's plan, with its
/// refusals: an allowed edit is the flow's, a refused one changes
/// neither.
#[test]
fn back_ups_cards_after_the_lock_edit_the_flows_plan_with_its_refusals() {
    let mut shell = Shell::new();
    let (mut app, w, fp) = single_key(&mut shell, true);
    let name = app.session.wallets[w].name.clone();
    shell.flow(&mut app, F::WalletLock);
    let mut app = restart(&mut shell, &mut app);
    assert!(app.flow().unwrap().made);
    // A vault unlocked after the lock: the wallet and its key load from
    // it, the key still held and no receive address shown for it; the
    // plan stays the flow's (§17.8).
    load_first_vault(&mut app, PHRASE);
    let w = app
        .session
        .wallets
        .iter()
        .position(|x| x.name == name)
        .expect("the wallet loaded from its vault");
    assert!(
        app.key_held(osk_bip::keys::Fingerprint(fp)),
        "held, from the vault"
    );
    assert!(
        app.receive_held(&app.session.wallets[w]),
        "no receive address"
    );
    app.press(Action::Backup(w));
    assert!(app.backup.is_some());
    let slots = app.flow().unwrap().slots.len();
    // Vault 1's stick at Place 1 off: a vault's stick, allowed, the flow's.
    app.press(Action::BRowOff(0, Row::Vault(0).code()));
    assert_eq!(
        app.flow().unwrap().slots.len(),
        slots - 1,
        "{:?}",
        app.toast_text()
    );
    let flow_answers = app.flow().unwrap().wallet.as_ref().unwrap().answers.clone();
    assert_eq!(app.backup.as_ref().unwrap().answers, flow_answers);
    // Its last stick off: the key made here would be kept on no stick.
    app.press(Action::BRowOff(1, Row::Vault(0).code()));
    assert_eq!(
        app.toast_text(),
        Some("A key not yet written stays kept on a stick")
    );
    assert_eq!(app.backup.as_ref().unwrap().answers, flow_answers);
    // Files stay as made, a vault open or not.
    app.press(Action::BForm(0, Form::File.code()));
    assert_eq!(
        app.toast_text(),
        Some(
            "Files and paper stay as made after the lock; finish the writes, then Back up again to change them"
        )
    );
    assert_eq!(
        app.flow().unwrap().wallet.as_ref().unwrap().answers,
        flow_answers
    );
}

/// The flow's own slider of keys left off each share sets the flow's
/// plan, and the vault that keeps a share goes back to take the share it
/// implies (§17.7).
#[test]
fn the_flows_slider_sets_its_keys_left_off_and_the_vault_takes_the_share_it_implies() {
    use faraday_vault::records::field;
    let mut shell = Shell::new();
    sticks(&mut shell, 1);
    let mut app = process(Vec::new(), Vec::new(), Vec::new());
    keep_time(&mut app);
    create_with_vaults(&mut app, true, 1);
    bring_in_all(&mut shell, &mut app, &["S1"]);
    create_vault(&mut shell, &mut app, 0, PHRASE);
    let fps: Vec<[u8; 4]> = (0..3).map(|i| key_in(&mut app, i)).collect();
    app.press(Action::Nav(Screen::Create));
    for (i, fp) in fps.iter().enumerate() {
        app.press(Action::CSlotHere(i as u8, *fp));
    }
    app.press(Action::CNext(cstep::KEYS));
    let w = app.create.as_ref().unwrap().built.expect("wallet made");
    app.press(Action::CNext(cstep::CHECK));
    app.press(Action::CBackup(1));
    app.press(Action::BSplit(true));
    untick_everywhere(&mut app, Row::Words);
    let vaults = {
        let shape = backup_shape(&app);
        app.backup.as_ref().unwrap().answers.vault_count(&shape)
    };
    for v in 1..vaults {
        untick_everywhere(&mut app, Row::Vault(v));
    }
    let p = vault_place(&app, 0);
    form_off_everywhere(&mut app, Form::Vault(0));
    form_off_everywhere(&mut app, Form::Paper);
    app.press(Action::BShare(p as u8, 1));
    app.press(Action::BForm(p as u8, Form::Vault(0).code()));
    form_off_everywhere(&mut app, Form::Paper);
    assert_eq!(app.backup.as_ref().unwrap().answers.omit, 1);
    let (m, n) = faraday_core::wallet::Session::quorum(&app.session.wallets[w]);
    let none_off = faraday_core::backup::split_share(
        &app.session.wallets[w],
        &faraday_core::backup::split_plan(n, m, 0)[1],
    )
    .unwrap();
    app.press(Action::BChecklist);
    save_vaults(&mut app);
    app.press(Action::BNext(bstep::ENVELOPE));
    shell.flow(&mut app, F::WalletLock);
    let mut app = restart(&mut shell, &mut app);
    shell.flow(&mut app, F::ChangePlan);
    flow_plan(
        &mut shell,
        &mut app,
        PlanPress::Q(faraday_core::qstep::DESCRIPTION),
    );
    let slide = Action::Slide(faraday_core::seeds::SLIDE_FLOW_OMIT, 0);
    assert!(offered_down(&mut app, slide), "the flow draws the slider");
    shell.press(&mut app, slide);
    assert_eq!(app.flow().unwrap().wallet.as_ref().unwrap().answers.omit, 0);
    assert!(app.flow().unwrap().adding, "Vault 1's share is another now");
    unlock(&mut app, PHRASE);
    shell.flow(&mut app, F::CopyKey);
    let texts: Vec<String> = app.vaults.open[0]
        .contents
        .of(kind::WALLET_SHARE)
        .map(|(_, r)| r.text(field::SHARE).unwrap_or("").to_string())
        .collect();
    assert_eq!(texts, vec![none_off], "Share 2 with no key left off");
}

/// Place names typed on the flow's Places card before the lock are in
/// each vault's plan record; after the lock no name field is drawn, and
/// the flow's kept state holds no name (§17.7).
#[test]
fn names_typed_before_the_lock_are_in_each_vaults_plan_and_never_in_the_flow() {
    use faraday_vault::records::field;
    const NAME: &str = "Harbour cellar";
    let mut shell = Shell::new();
    let mut app = two_of_three_with_setup(&mut shell, 2);
    untick_everywhere(&mut app, Row::Words);
    // Two vaults, as set up.
    untick_everywhere(&mut app, Row::Vault(2));
    let i = app.backup_flows.len() - 1;
    shell.flow(&mut app, F::Show(i));
    shell.flow(&mut app, F::ChangePlan);
    flow_plan(
        &mut shell,
        &mut app,
        PlanPress::Q(faraday_core::qstep::PLACES),
    );
    assert!(offered_down(&mut app, Action::BName(0)), "a name field");
    app.press(Action::BName(0));
    type_text(&mut app, NAME);
    app.event(Event::Key(Key::Enter));
    app.press(Action::BChecklist);
    save_vaults(&mut app);
    for o in 0..2 {
        let names: Vec<String> = app.vaults.open[o]
            .contents
            .of(kind::PLAN)
            .flat_map(|(_, r)| {
                r.fields
                    .iter()
                    .filter(|f| f.number == field::PLAN_PLACE)
                    .map(|f| String::from_utf8_lossy(&f.bytes).into_owned())
                    .collect::<Vec<_>>()
            })
            .collect();
        assert!(names.iter().any(|n| n == NAME), "vault {o}: {names:?}");
    }
    app.press(Action::BNext(bstep::ENVELOPE));
    shell.flow(&mut app, F::WalletLock);
    let mut app = restart(&mut shell, &mut app);
    let kept = faraday_core::backup_flow::encode(&app.backup_flows);
    assert!(
        !kept.windows(NAME.len()).any(|w| w == NAME.as_bytes()),
        "the flow keeps no name"
    );
    shell.flow(&mut app, F::ChangePlan);
    flow_plan(
        &mut shell,
        &mut app,
        PlanPress::Q(faraday_core::qstep::PLACES),
    );
    assert!(
        !offered_down(&mut app, Action::BName(0)),
        "no name field after the lock"
    );
}

/// The flow's plan card has one way on: whatever card is open, one
/// Continue on the screen, at the small panel's size and the desktop's.
#[test]
fn the_flows_plan_card_has_one_continue_at_both_sizes() {
    for (w, h, dpi) in [(480u16, 640u16, 286u16), (1280, 800, 160)] {
        let mut shell = Shell::new();
        let (mut app, _, _) = single_key_ready(&mut shell);
        shell.flow(&mut app, F::WalletLock);
        let mut app = restart(&mut shell, &mut app);
        app.event(Event::Display(osk_shell_api::DisplayInfo {
            width: w,
            height: h,
            dpi,
            inset_bottom: 0,
            inset_top: 0,
            buttons: 0,
            camera_fixed: false,
            secure: osk_shell_api::SecureHardware::None,
            boot: osk_shell_api::BootState::Unknown,
            memory_mib: None,
        }));
        shell.flow(&mut app, F::ChangePlan);
        let wf = app.flow().unwrap().wallet.as_ref().unwrap();
        let cards = faraday_core::plan_cards(&wf.shape, &wf.answers);
        for q in cards {
            if app.flow().unwrap().live.plan_q != Some(q) {
                flow_plan(&mut shell, &mut app, PlanPress::Q(q));
            }
            let _ = app.frame();
            let n = app
                .drawn_texts()
                .iter()
                .filter(|t| t.as_str() == "Continue")
                .count();
            assert!(n <= 1, "{w}x{h}, card {q}: {n} Continue");
        }
    }
}

/// One wallet, one checksum (F35): a 2-of-3 made here, split into shares
/// and rebuilt from a quorum of share files, has the descriptor and the
/// checksum it was made with.
#[test]
fn a_wallet_rebuilt_from_its_share_files_has_the_checksum_it_was_made_with() {
    let mut app = process(Vec::new(), Vec::new(), Vec::new());
    keep_time(&mut app);
    app.press(Action::CreateWallet);
    app.press(Action::CKind(multi_index()));
    let fps: Vec<[u8; 4]> = (0..3).map(|i| key_in(&mut app, i)).collect();
    app.press(Action::Nav(Screen::Create));
    for (i, fp) in fps.iter().enumerate() {
        app.press(Action::CSlotHere(i as u8, *fp));
    }
    app.press(Action::CNext(cstep::KEYS));
    let w = app.create.as_ref().unwrap().built.expect("wallet made");
    let wallet = &app.session.wallets[w];
    let made = wallet.policy.to_descriptor_checksummed();
    let (m, n) = faraday_core::wallet::Session::quorum(wallet);
    let split = faraday_core::backup::split_plan(n, m, 1);
    let files: Vec<(String, Vec<u8>)> = (0..2)
        .map(|j| {
            (
                format!("w-share-{}-of-{n}.txt", j + 1),
                faraday_core::backup::split_share(wallet, &split[j])
                    .unwrap()
                    .into_bytes(),
            )
        })
        .collect();
    let mut r = process(files.clone(), Vec::new(), Vec::new());
    keep_time(&mut r);
    r.press(Action::RestoreWallet);
    for (name, _) in &files {
        let i = r.inbox.iter().position(|f| &f.name == name).unwrap();
        r.press(Action::RShare(i));
    }
    r.press(Action::RRebuild);
    let rw = r.restore.as_ref().and_then(|x| x.wallet).expect("rebuilt");
    assert_eq!(
        r.session.wallets[rw].policy.to_descriptor_checksummed(),
        made
    );
}

/// A wallet written with `'` reads with the checksum of its `h` form.
#[test]
fn a_wallet_written_with_apostrophes_has_the_checksum_of_its_h_form() {
    let h = testkit::savings().split('#').next().unwrap().to_string();
    let apostrophe = apostrophes(&h);
    assert_ne!(h, apostrophe);
    let a = faraday_core::wallet::read_wallet(&apostrophe).unwrap();
    let b = faraday_core::wallet::read_wallet(&h).unwrap();
    assert_eq!(a.checksum(), b.checksum());
    assert_eq!(a.to_descriptor_checksummed(), b.to_descriptor_checksummed());
}

/// `text` with each hardened step of its keys' origins written `'`.
fn apostrophes(text: &str) -> String {
    let text = text.split('#').next().unwrap_or(text);
    let mut out = String::new();
    let mut origin = false;
    for c in text.chars() {
        match c {
            '[' => origin = true,
            ']' => origin = false,
            'h' if origin => {
                out.push('\'');
                continue;
            }
            _ => {}
        }
        out.push(c);
    }
    out
}

/// A vault whose wallet (type 3) and plan (type 11) were kept with `'`
/// opens with the wallet loaded and its plan attached.
#[test]
fn a_vault_kept_with_apostrophes_opens_with_its_plan_attached() {
    use faraday_vault::Record;
    use faraday_vault::records::field;
    let desc = apostrophes(&testkit::savings());
    let mut app = process(Vec::new(), Vec::new(), Vec::new());
    keep_time(&mut app);
    let w = app
        .session
        .add_wallet(
            "Savings",
            &desc,
            "test",
            faraday_core::wallet::Origin::Named,
        )
        .unwrap();
    let shape = app.plan_shape(w);
    let mut answers = Answers::preset(&shape, Preset::Paper);
    answers.set_places(&shape, 4);
    let text = answers.to_text();
    let contents = faraday_vault::Contents {
        records: vec![
            Record::new(kind::WALLET)
                .with(field::WALLET, desc.as_bytes())
                .with(field::WALLET_NAME, b"Savings"),
            Record::new(kind::PLAN)
                .with(field::PLAN_WALLET, desc.as_bytes())
                .with(field::PLAN_ANSWERS, text.as_bytes()),
        ],
    };
    let cost = faraday_vault::Cost {
        memory_kib: faraday_vault::MIN_MEMORY_KIB,
        passes: 1,
        lanes: 1,
    };
    let file = faraday_vault::create(
        faraday_vault::SLOT_SIZES[0],
        cost,
        &[PHRASE.as_bytes()],
        &[contents],
        &[9; 32],
    )
    .unwrap();
    let mut next = process(vec![("old.ofv".into(), file)], Vec::new(), Vec::new());
    keep_time(&mut next);
    next.press(Action::Nav(Screen::Vaults));
    next.press(Action::Vault(faraday_core::vaults::VaultAction::Open(0)));
    unlock(&mut next, PHRASE);
    let loaded = next
        .session
        .wallets
        .iter()
        .position(|x| x.policy.checksum() == app.session.wallets[w].policy.checksum());
    let loaded = loaded.unwrap_or_else(|| {
        next.press(Action::Vault(
            faraday_core::vaults::VaultAction::LoadChosen(0),
        ));
        next.session.wallets.len().saturating_sub(1)
    });
    assert_eq!(
        next.session.wallets[loaded]
            .policy
            .to_descriptor_checksummed(),
        app.session.wallets[w].policy.to_descriptor_checksummed()
    );
    next.press(Action::Backup(loaded));
    let b = next.backup.as_ref().unwrap();
    assert_eq!(b.q, None, "the plan is attached");
    assert_eq!(b.answers.places, 4);
}

// ---- the round takes out only what the flow put in (§17) ----

/// After the lock, taking Key 1 out of the existing vault that held it
/// before the flow is refused, and the sealed copy still holds it.
#[test]
fn a_key_an_existing_vault_held_before_the_flow_is_never_taken_out() {
    let mut shell = Shell::new();
    sticks(&mut shell, 1);
    let main = vault_with_key("Main", PHRASE, 0x41, 1);
    shell.disk_mut("S1").files = vec![("main.ofv".into(), main)];
    let mut app = process(Vec::new(), Vec::new(), Vec::new());
    keep_time(&mut app);
    create_with_vaults(&mut app, false, 1);
    shell.plug(&mut app, "S1");
    let k = app
        .flow()
        .unwrap()
        .live
        .reading
        .as_ref()
        .and_then(|r| r.files.iter().position(|(f, _)| f.name == "main.ofv"));
    if let Some(k) = k {
        shell.flow(&mut app, F::PickFile(0, k));
    }
    shell.flow(&mut app, F::Tap);
    shell.pull(&mut app, "S1");
    assert_eq!(step(&app), Step::Vaults);
    unlock(&mut app, PHRASE);
    assert_eq!(app.screen, Screen::Create, "{:?}", app.vaults.unlock_error);
    let fp = key_in(&mut app, 1);
    app.press(Action::Nav(Screen::Create));
    app.press(Action::CSlotHere(0, fp));
    if app.create.as_ref().unwrap().built.is_none() {
        app.press(Action::CNext(cstep::KEYS));
    }
    app.press(Action::CNext(cstep::CHECK));
    app.press(Action::CBackup(1));
    untick_everywhere(&mut app, Row::Words);
    form_off_everywhere(&mut app, Form::Paper);
    app.press(Action::BChecklist);
    save_vaults(&mut app);
    assert_eq!(app.vaults.open[0].contents.of(kind::KEY).count(), 1);
    app.press(Action::BNext(bstep::ENVELOPE));
    shell.flow(&mut app, F::WalletLock);
    let mut app = restart(&mut shell, &mut app);
    shell.flow(&mut app, F::ChangePlan);
    let p = flow_place_of(&app, 0);
    flow_keep(&mut shell, &mut app, p, Row::Vault(0), &[]);
    assert_eq!(
        app.toast_text(),
        Some("Vault 1 held this before; take it out from the vault's contents")
    );
    {
        let wf = app.flow().unwrap().wallet.as_ref().unwrap();
        assert!(
            wf.answers.vault_seeds(&wf.shape, 0).contains(&0),
            "the plan stays"
        );
    }
    write_sticks(&mut shell, &mut app, &["S1"]);
    assert_eq!(
        step(&app),
        Step::Done,
        "{:?}",
        app.flow().unwrap().live.problem
    );
    let sealed = ofv(&shell, "S1").expect("Vault 1 on S1");
    let (_, c) = faraday_vault::open(&sealed, PHRASE.as_bytes()).unwrap();
    assert_eq!(c.of(kind::KEY).count(), 1, "Key 1 stays");
}

/// Two wallets that share a key in one vault: taking the key out for one
/// of them after the lock is refused, and the key stays.
#[test]
fn a_key_another_wallet_in_the_vault_uses_is_never_taken_out() {
    let mut shell = Shell::new();
    let (mut app, a, _) = single_key(&mut shell, false);
    // Another wallet of the same key, saved into the same vault.
    let k = app.session.keys.len() - 1;
    let text = NewKind::Taproot
        .key_text(&app.session.keys[k].master)
        .unwrap();
    let b = app
        .session
        .add_wallet(
            "Other",
            &format!("tr({text}/<0;1>/*)"),
            "test",
            faraday_core::wallet::Origin::Named,
        )
        .unwrap();
    assert_ne!(a, b);
    app.press(Action::Backup(b));
    app.press(Action::BPreset(1));
    untick_everywhere(&mut app, Row::Words);
    form_off_everywhere(&mut app, Form::Paper);
    app.press(Action::BChecklist);
    app.press(Action::BVaultSave(0));
    assert_eq!(app.vaults.open[0].contents.of(kind::WALLET).count(), 2);
    // Back to the first wallet's checklist, and its lock.
    app.press(Action::Backup(a));
    app.press(Action::BChecklist);
    app.press(Action::BNext(bstep::ENVELOPE));
    shell.flow(&mut app, F::WalletLock);
    let mut app = restart(&mut shell, &mut app);
    shell.flow(&mut app, F::ChangePlan);
    let p = flow_place_of(&app, 0);
    flow_keep(&mut shell, &mut app, p, Row::Vault(0), &[]);
    assert_eq!(
        app.toast_text(),
        Some("Another wallet in Vault 1 uses this key; take it out from the vault's contents")
    );
    let wf = app.flow().unwrap().wallet.as_ref().unwrap();
    assert!(
        wf.answers.vault_seeds(&wf.shape, 0).contains(&0),
        "the key stays"
    );
}

// ---- the final review (§17) ----

/// A key held for its sticks never ends on none: after the lock, a seed's
/// file is not added (it is made before the lock), the seed stays in its
/// vault, and the key is released only once the sealed copies hold it.
#[test]
fn a_held_seed_is_never_left_on_no_stick_after_the_lock() {
    let mut shell = Shell::new();
    let (mut app, _, fp) = single_key(&mut shell, true);
    shell.flow(&mut app, F::WalletLock);
    let mut app = restart(&mut shell, &mut app);
    assert_eq!(step(&app), Step::Write);
    shell.flow(&mut app, F::ChangePlan);
    flow_keep(&mut shell, &mut app, 1, Row::File, &[0]);
    assert_eq!(
        app.toast_text(),
        Some(
            "Files and paper stay as made after the lock; finish the writes, then Back up again to change them"
        )
    );
    let p = flow_place_of(&app, 0);
    flow_keep(&mut shell, &mut app, p, Row::Vault(0), &[]);
    assert_eq!(
        app.toast_text(),
        Some("A key not yet written stays kept on a stick")
    );
    {
        let wf = app.flow().unwrap().wallet.as_ref().unwrap();
        assert!(wf.answers.vault_seeds(&wf.shape, 0).contains(&0));
        assert!(
            wf.answers.seeds_as_files(&wf.shape).is_empty(),
            "no seed file on the plan"
        );
    }
    let fp = osk_bip::keys::Fingerprint(fp);
    write_sticks(&mut shell, &mut app, &["S1", "S2", "S3"]);
    assert_eq!(
        step(&app),
        Step::Done,
        "{:?}",
        app.flow().unwrap().live.problem
    );
    for d in ["S1", "S2"] {
        let sealed = ofv(&shell, d).expect("Vault 1's copy");
        let (_, c) = faraday_vault::open(&sealed, PHRASE.as_bytes()).unwrap();
        assert_eq!(c.of(kind::KEY).count(), 1, "{d} holds the key");
    }
    assert!(!app.key_held(fp), "released once its copies are written");
}

/// A vault that holds any record of another wallet keeps its key records:
/// a hidden wallet on the same words with a passphrase kept apart is one
/// the flow cannot tell from the words alone.
#[test]
fn a_key_is_never_taken_out_of_a_vault_another_wallets_record_is_in() {
    use faraday_vault::Record;
    use faraday_vault::records::field;
    let mut shell = Shell::new();
    let (mut app, _, _) = single_key_with(&mut shell, false, true);
    // The hidden wallet: the same words with a passphrase; its
    // description in Vault 1.
    app.press(Action::Entry(None));
    for c in testkit::test_words(testkit::TEST_SEEDS[1].0).chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    app.press(Action::EntryPassphrase);
    type_text(&mut app, "hidden");
    app.press(Action::EntryAdd);
    let k = app.session.keys.len() - 1;
    let text = NewKind::NativeSegwit
        .key_text(&app.session.keys[k].master)
        .unwrap();
    let hidden = format!("wpkh({text}/<0;1>/*)");
    app.vaults.open[0].contents.records.push(
        Record::new(kind::WALLET)
            .with(field::WALLET, hidden.as_bytes())
            .with(field::WALLET_NAME, b"Hidden"),
    );
    shell.flow(&mut app, F::WalletLock);
    let mut app = restart(&mut shell, &mut app);
    shell.flow(&mut app, F::ChangePlan);
    // Words at Place 1, then the seed out of Vault 1.
    flow_keep(&mut shell, &mut app, 0, Row::Words, &[0]);
    let p = flow_place_of(&app, 0);
    flow_keep(&mut shell, &mut app, p, Row::Vault(0), &[]);
    assert_eq!(
        app.toast_text(),
        Some("Another wallet in Vault 1 uses this key; take it out from the vault's contents")
    );
    let wf = app.flow().unwrap().wallet.as_ref().unwrap();
    assert!(
        wf.answers.vault_seeds(&wf.shape, 0).contains(&0),
        "the key stays"
    );
}

/// After the lock, Keys left off each share does not change while a share
/// is on paper or in a file: those were made with it as it is.
#[test]
fn keys_left_off_each_share_stay_after_the_lock_while_a_share_is_in_a_file() {
    let mut shell = Shell::new();
    sticks(&mut shell, 3);
    let mut app = process(Vec::new(), Vec::new(), Vec::new());
    keep_time(&mut app);
    create_with_vaults(&mut app, true, 1);
    bring_in_all(&mut shell, &mut app, &["S1"]);
    create_vault(&mut shell, &mut app, 0, PHRASE);
    let fps: Vec<[u8; 4]> = (0..3).map(|i| key_in(&mut app, i)).collect();
    app.press(Action::Nav(Screen::Create));
    for (i, fp) in fps.iter().enumerate() {
        app.press(Action::CSlotHere(i as u8, *fp));
    }
    app.press(Action::CNext(cstep::KEYS));
    app.press(Action::CNext(cstep::CHECK));
    app.press(Action::CBackup(1));
    app.press(Action::BSplit(true));
    untick_everywhere(&mut app, Row::Words);
    let vaults = {
        let shape = backup_shape(&app);
        app.backup.as_ref().unwrap().answers.vault_count(&shape)
    };
    for v in 1..vaults {
        untick_everywhere(&mut app, Row::Vault(v));
    }
    // Share 2 in Vault 1 and as a file, at the place of Vault 1's stick.
    let p = vault_place(&app, 0);
    form_off_everywhere(&mut app, Form::Vault(0));
    form_off_everywhere(&mut app, Form::Paper);
    app.press(Action::BShare(p as u8, 1));
    app.press(Action::BForm(p as u8, Form::Vault(0).code()));
    app.press(Action::BForm(p as u8, Form::File.code()));
    form_off_everywhere(&mut app, Form::Paper);
    assert_eq!(app.backup.as_ref().unwrap().answers.omit, 1);
    app.press(Action::BChecklist);
    save_vaults(&mut app);
    app.press(Action::BNext(bstep::ENVELOPE));
    shell.flow(&mut app, F::WalletLock);
    let mut app = restart(&mut shell, &mut app);
    shell.flow(&mut app, F::ChangePlan);
    shell.press(
        &mut app,
        Action::Slide(faraday_core::seeds::SLIDE_FLOW_OMIT, 0),
    );
    assert_eq!(
        app.toast_text(),
        Some(
            "Files and paper stay as made after the lock; finish the writes, then Back up again to change them"
        )
    );
    assert_eq!(app.flow().unwrap().wallet.as_ref().unwrap().answers.omit, 1);
}

// ---- the re-check: after the lock, each stick's files from the plan ----

/// A 2-of-3 with no vault: the three keys loaded, Paper only, nothing on
/// paper, at the checklist.
fn two_of_three_paper(shell: &mut Shell, n: usize) -> Faraday {
    sticks(shell, n);
    let mut app = process(Vec::new(), Vec::new(), Vec::new());
    keep_time(&mut app);
    app.press(Action::CreateWallet);
    app.press(Action::CKind(multi_index()));
    let fps: Vec<[u8; 4]> = (0..3).map(|i| key_in(&mut app, i)).collect();
    app.press(Action::Nav(Screen::Create));
    for (i, fp) in fps.iter().enumerate() {
        app.press(Action::CSlotHere(i as u8, *fp));
    }
    app.press(Action::CNext(cstep::KEYS));
    app.press(Action::CNext(cstep::CHECK));
    app.press(Action::CBackup(0));
    untick_everywhere(&mut app, Row::Words);
    form_off_everywhere(&mut app, Form::Paper);
    app
}

/// A seed file taken off a place's stick of files after the lock leaves
/// that stick's list: the stick is written with what the plan keeps
/// there, and nothing stale holds the flow.
#[test]
fn a_seed_file_taken_off_after_the_lock_is_refused_and_its_stick_keeps_its_files() {
    let mut shell = Shell::new();
    let mut app = two_of_three_paper(&mut shell, 2);
    keep_at(&mut app, 0, Row::File, &[0, 1]);
    keep_at(&mut app, 1, Row::File, &[2]);
    let names: Vec<String> = backup_shape(&app)
        .seeds
        .iter()
        .map(|s| s.name.to_lowercase())
        .collect();
    app.press(Action::BChecklist);
    for _ in 0..3 {
        app.press(Action::BFile);
        app.press(Action::SecretAck);
        app.press(Action::SecretUnprotected);
    }
    app.press(Action::BNext(bstep::ENVELOPE));
    shell.flow(&mut app, F::WalletLock);
    let mut app = restart(&mut shell, &mut app);
    shell.flow(&mut app, F::ChangePlan);
    // Files stay as made after the lock (§17.8): refused, and Place 1's
    // stick keeps both files.
    flow_keep(&mut shell, &mut app, 0, Row::File, &[1]);
    assert_eq!(
        app.toast_text(),
        Some(
            "Files and paper stay as made after the lock; finish the writes, then Back up again to change them"
        )
    );
    let f = app.flow().unwrap();
    assert_eq!(f.slots[0].target, Target::PlaceFiles(0));
    assert_eq!(f.slots[0].files.len(), 2, "{:?}", f.slots[0].files);
    write_sticks(&mut shell, &mut app, &["S1", "S2"]);
    assert_eq!(
        step(&app),
        Step::Done,
        "{:?}",
        app.flow().unwrap().live.problem
    );
    let words = |d: &str| -> Vec<String> {
        names_on(&shell, d)
            .into_iter()
            .filter(|n| n.ends_with("-words.txt"))
            .collect()
    };
    assert_eq!(words("S1").len(), 2);
    assert!(words("S1").iter().any(|n| n.contains(&names[0])));
    assert!(words("S1").iter().any(|n| n.contains(&names[1])));
    assert!(words("S2")[0].contains(&names[2]));
}

fn names_on(shell: &Shell, d: &str) -> Vec<String> {
    shell.disk(d).files.iter().map(|f| f.0.clone()).collect()
}

/// Types back the copy of the key in Create's slot 0 on the checklist's
/// copy item.
fn type_back_the_copy(app: &mut Faraday) {
    use osk_bip::bip39::Mnemonic;
    let k = app.session.keys.len() - 1;
    let key = &app.session.keys[k];
    let words = key.words.as_ref().unwrap().to_string();
    let m = Mnemonic::parse(key.language, &words).unwrap();
    let digits = osk_codec::seedqr::to_digits(&m);
    if app.backup.as_ref().unwrap().open != Some(bstep::COPY) {
        app.press(Action::BStep(bstep::COPY));
    }
    app.press(Action::BCheck);
    type_text(
        app,
        std::str::from_utf8(digits.expose().as_bytes()).unwrap(),
    );
    assert!(app.backup_item_done(Item::Copy(0)), "the copy is checked");
}

/// A key made here, in Vault 1, as a file at Place 2 and in words at
/// Place 1: after the lock, with the file off the plan, emptying Vault 1
/// leaves it on no stick and is refused.
#[test]
fn a_held_key_is_not_left_on_a_stale_file_after_the_lock() {
    let mut shell = Shell::new();
    sticks(&mut shell, 2);
    let mut app = process(Vec::new(), Vec::new(), Vec::new());
    keep_time(&mut app);
    create_with_vaults(&mut app, false, 1);
    bring_in_all(&mut shell, &mut app, &["S1"]);
    create_vault(&mut shell, &mut app, 0, PHRASE);
    app.press(Action::Nav(Screen::Create));
    let fp = new_key(&mut app);
    if app.create.as_ref().unwrap().built.is_none() {
        app.press(Action::CNext(cstep::KEYS));
    }
    app.press(Action::CNext(cstep::CHECK));
    app.press(Action::CBackup(1));
    form_off_everywhere(&mut app, Form::Paper);
    for p in 1..backup_places(&app) {
        app.press(Action::BRowOff(p as u8, Row::Words.code()));
    }
    keep_at(&mut app, 1, Row::File, &[0]);
    app.press(Action::BChecklist);
    app.press(Action::BNext(bstep::BLANK));
    type_back_the_copy(&mut app);
    save_vaults(&mut app);
    app.press(Action::BFile);
    app.press(Action::SecretAck);
    app.press(Action::SecretUnprotected);
    app.press(Action::BNext(bstep::ENVELOPE));
    shell.flow(&mut app, F::WalletLock);
    let mut app = restart(&mut shell, &mut app);
    shell.flow(&mut app, F::ChangePlan);
    flow_plan(&mut shell, &mut app, PlanPress::RowOff(1, Row::File.code()));
    assert_eq!(
        app.toast_text(),
        Some(
            "Files and paper stay as made after the lock; finish the writes, then Back up again to change them"
        )
    );
    let p = flow_place_of(&app, 0);
    // The file stays as made, so Vault 1 may be emptied: the key stays
    // held until the stick of files that holds it is written.
    flow_keep(&mut shell, &mut app, p, Row::Vault(0), &[]);
    let f = app.flow().unwrap();
    let wf = f.wallet.as_ref().unwrap();
    assert!(!wf.answers.vault_seeds(&wf.shape, 0).contains(&0));
    assert!(
        f.slots.iter().any(|s| s.target == Target::PlaceFiles(1)
            && s.files.iter().any(|n| n.ends_with("-words.txt"))),
        "kept on the stick of files"
    );
    let held = &app.flow().unwrap().wallet.as_ref().unwrap().held;
    let fp = faraday_core::wallet::fp_text(osk_bip::keys::Fingerprint(fp));
    assert!(
        held.iter().any(|h| h.eq_ignore_ascii_case(&fp)),
        "held until that stick is written"
    );
}

/// A 2-of-3 in shares, Share 2 in Vault 1 and as a file at the place of
/// Vault 1's stick, saved, locked and restarted: (app, that place).
fn shares_locked(shell: &mut Shell) -> (Faraday, usize) {
    sticks(shell, 3);
    let mut app = process(Vec::new(), Vec::new(), Vec::new());
    keep_time(&mut app);
    create_with_vaults(&mut app, true, 1);
    bring_in_all(shell, &mut app, &["S1"]);
    create_vault(shell, &mut app, 0, PHRASE);
    let fps: Vec<[u8; 4]> = (0..3).map(|i| key_in(&mut app, i)).collect();
    app.press(Action::Nav(Screen::Create));
    for (i, fp) in fps.iter().enumerate() {
        app.press(Action::CSlotHere(i as u8, *fp));
    }
    app.press(Action::CNext(cstep::KEYS));
    app.press(Action::CNext(cstep::CHECK));
    app.press(Action::CBackup(1));
    app.press(Action::BSplit(true));
    untick_everywhere(&mut app, Row::Words);
    let vaults = {
        let shape = backup_shape(&app);
        app.backup.as_ref().unwrap().answers.vault_count(&shape)
    };
    for v in 1..vaults {
        untick_everywhere(&mut app, Row::Vault(v));
    }
    let p = vault_place(&app, 0);
    form_off_everywhere(&mut app, Form::Vault(0));
    form_off_everywhere(&mut app, Form::Paper);
    app.press(Action::BShare(p as u8, 1));
    app.press(Action::BForm(p as u8, Form::Vault(0).code()));
    app.press(Action::BForm(p as u8, Form::File.code()));
    form_off_everywhere(&mut app, Form::Paper);
    app.press(Action::BChecklist);
    save_vaults(&mut app);
    app.press(Action::BNext(bstep::ENVELOPE));
    shell.flow(&mut app, F::WalletLock);
    (restart(shell, &mut app), p)
}

/// The sticks of the flow on screen in its order, as disks: the stick
/// read for Vault 1 is S1, the others new sticks after it.
fn disks_in_order(app: &Faraday) -> Vec<&'static str> {
    let mut fresh = ["S2", "S3"].into_iter();
    app.flow()
        .unwrap()
        .slots
        .iter()
        .map(|s| {
            if matches!(s.target, Target::Place(_) | Target::PlaceVault(..)) {
                "S1"
            } else {
                fresh.next().unwrap_or("S3")
            }
        })
        .collect()
}

/// After the lock, a share moved and "As a file" ticked at another place:
/// each stick of files is written with the shares' files the plan puts
/// there, and the vault takes the share now planned.
#[test]
fn shares_files_follow_the_plan_after_the_lock() {
    let mut shell = Shell::new();
    let (mut app, p) = shares_locked(&mut shell);
    let q = (0..3).find(|&q| q != p).unwrap();
    let files_at = |app: &Faraday, at: usize| -> Vec<String> {
        app.flow()
            .unwrap()
            .slots
            .iter()
            .find(|s| s.target == Target::PlaceFiles(at))
            .map(|s| s.files.clone())
            .unwrap_or_default()
    };
    let before = files_at(&app, p);
    assert!(
        before.iter().any(|n| n.contains("-share-2-of-3")),
        "{before:?}"
    );
    // After the lock the shares' files stay as made (§17.8): moving a
    // share, or a file at another place, is refused.
    shell.flow(&mut app, F::ChangePlan);
    for press in [
        PlanPress::Share(p as u8, 2),
        PlanPress::Form(q as u8, Form::File.code()),
    ] {
        flow_plan(&mut shell, &mut app, press);
        assert_eq!(
            app.toast_text(),
            Some(
                "Files and paper stay as made after the lock; finish the writes, then Back up again to change them"
            ),
            "{press:?}"
        );
    }
    assert_eq!(files_at(&app, p), before);
    assert!(
        app.flow()
            .unwrap()
            .slots
            .iter()
            .all(|s| s.target != Target::PlaceFiles(q))
    );
    // The sticks are written with the files made.
    let order = disks_in_order(&app);
    write_sticks(&mut shell, &mut app, &order);
    assert_eq!(
        step(&app),
        Step::Done,
        "{:?}",
        app.flow().unwrap().live.problem
    );
    let k = app
        .flow()
        .unwrap()
        .slots
        .iter()
        .position(|s| s.target == Target::PlaceFiles(p))
        .unwrap();
    let on = names_on(&shell, order[k]);
    assert!(on.iter().any(|n| n.contains("-share-2-of-3")), "{on:?}");
}

/// After the lock, with a share in a file, Whole and back cannot change
/// the split: whole or in shares is fixed, as the keys left off are.
#[test]
fn whole_or_in_shares_is_fixed_after_the_lock_while_a_share_is_in_a_file() {
    let mut shell = Shell::new();
    let (mut app, _) = shares_locked(&mut shell);
    shell.flow(&mut app, F::ChangePlan);
    flow_plan(&mut shell, &mut app, PlanPress::Split(false));
    assert_eq!(
        app.toast_text(),
        Some(
            "Files and paper stay as made after the lock; finish the writes, then Back up again to change them"
        )
    );
    let a = &app.flow().unwrap().wallet.as_ref().unwrap().answers;
    assert!(a.split);
    assert_eq!(a.omit, 1);
}

/// A vault that keeps another wallet's bare record of the words: each
/// round adds this wallet's record with its passphrase once, not again.
#[test]
fn the_round_adds_a_planned_key_record_once_beside_another_of_its_words() {
    use faraday_vault::records::field;
    let mut shell = Shell::new();
    sticks(&mut shell, 1);
    let main = vault_with_key("Main", PHRASE, 0x41, 1);
    shell.disk_mut("S1").files = vec![("main.ofv".into(), main)];
    let mut app = process(Vec::new(), Vec::new(), Vec::new());
    keep_time(&mut app);
    create_with_vaults(&mut app, false, 1);
    shell.plug(&mut app, "S1");
    let k = app
        .flow()
        .unwrap()
        .live
        .reading
        .as_ref()
        .and_then(|r| r.files.iter().position(|(f, _)| f.name == "main.ofv"));
    if let Some(k) = k {
        shell.flow(&mut app, F::PickFile(0, k));
    }
    shell.flow(&mut app, F::Tap);
    shell.pull(&mut app, "S1");
    unlock(&mut app, PHRASE);
    // The same words with a passphrase: this wallet's key.
    app.press(Action::Entry(None));
    for c in testkit::test_words(testkit::TEST_SEEDS[1].0).chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    app.press(Action::EntryPassphrase);
    type_text(&mut app, "kept here");
    app.press(Action::EntryAdd);
    let fp = app.session.keys.last().unwrap().master.fingerprint().0;
    app.press(Action::Nav(Screen::Create));
    app.press(Action::CSlotHere(0, fp));
    if app.create.as_ref().unwrap().built.is_none() {
        app.press(Action::CNext(cstep::KEYS));
    }
    app.press(Action::CNext(cstep::CHECK));
    app.press(Action::CBackup(1));
    untick_everywhere(&mut app, Row::Words);
    untick_everywhere(&mut app, Row::Passphrase);
    form_off_everywhere(&mut app, Form::Paper);
    form_off_everywhere(&mut app, Form::Vault(0));
    let p = vault_place(&app, 0);
    app.press(Action::BRow(p as u8, Row::Vault(0).code()));
    app.press(Action::BPickPass(0));
    app.press(Action::BPickDone);
    app.press(Action::BChecklist);
    save_vaults(&mut app);
    let keys = |app: &Faraday| -> (usize, usize) {
        let all: Vec<bool> = app.vaults.open[0]
            .contents
            .of(kind::KEY)
            .map(|(_, r)| r.field(field::KEY_PASSPHRASE).is_some())
            .collect();
        (all.len(), all.iter().filter(|&&p| p).count())
    };
    assert_eq!(keys(&app), (2, 1), "the bare record and this wallet's");
    app.press(Action::BNext(bstep::ENVELOPE));
    shell.flow(&mut app, F::WalletLock);
    let mut app = restart(&mut shell, &mut app);
    // Two rounds: the description into Vault 1, then out of it.
    for _ in 0..2 {
        shell.flow(&mut app, F::ChangePlan);
        let p = flow_place_of(&app, 0);
        flow_plan(
            &mut shell,
            &mut app,
            PlanPress::Form(p as u8, Form::Vault(0).code()),
        );
        assert!(app.flow().unwrap().adding, "{:?}", app.toast_text());
        if app.vaults.open.is_empty() {
            unlock(&mut app, PHRASE);
        }
        shell.flow(&mut app, F::CopyKey);
        assert_eq!(keys(&app), (2, 1), "{:?}", app.toast_text());
        shell.flow(&mut app, F::MadeContinue);
    }
}

// ---- after the lock only vaults change (§17.8) ----

/// A refused edit after the lock loads nothing: no wallet, no receive
/// address; "As a file" at a place with no vault open is one.
#[test]
fn a_file_added_after_the_lock_is_refused_and_loads_no_wallet() {
    let mut shell = Shell::new();
    let (mut app, _, _) = single_key_ready(&mut shell);
    shell.flow(&mut app, F::WalletLock);
    let mut app = restart(&mut shell, &mut app);
    assert!(app.session.wallets.is_empty());
    shell.flow(&mut app, F::ChangePlan);
    flow_plan(&mut shell, &mut app, PlanPress::Form(0, Form::File.code()));
    assert_eq!(
        app.toast_text(),
        Some(
            "Files and paper stay as made after the lock; finish the writes, then Back up again to change them"
        )
    );
    assert!(app.session.wallets.is_empty(), "no wallet is loaded");
    assert!(
        app.flow()
            .unwrap()
            .slots
            .iter()
            .all(|s| s.target != Target::PlaceFiles(0))
    );
}

/// The Done card's unlock loads the vault's wallet and key, the key no
/// longer held once every stick holding it is written.
#[test]
fn the_done_landing_loads_the_vaults_keys_released() {
    let mut shell = Shell::new();
    let (mut app, _, fp) = single_key(&mut shell, true);
    shell.flow(&mut app, F::WalletLock);
    let mut app = restart(&mut shell, &mut app);
    write_sticks(&mut shell, &mut app, &["S1", "S2", "S3"]);
    assert_eq!(step(&app), Step::Done);
    shell.flow(&mut app, F::UnlockDone(0));
    app.vaults.ms_per_unit = Some(180);
    unlock(&mut app, PHRASE);
    let fp = osk_bip::keys::Fingerprint(fp);
    assert!(
        app.session
            .keys
            .iter()
            .any(|k| k.master.fingerprint() == fp),
        "the key loaded from the vault"
    );
    assert!(!app.session.wallets.is_empty(), "its wallet too");
    assert!(!app.key_held(fp), "released");
}

/// An edit after the lock that only moves a vault's sticks leaves every
/// stick of files' list as it was made, the account key files with it.
#[test]
fn a_stick_of_files_keeps_its_list_after_a_vault_edit() {
    let mut shell = Shell::new();
    let (mut app, _, _) = single_key_ready(&mut shell);
    shell.flow(&mut app, F::WalletLock);
    let mut app = restart(&mut shell, &mut app);
    let files = |app: &Faraday| -> Vec<String> {
        app.flow()
            .unwrap()
            .slots
            .iter()
            .find(|s| s.target == Target::PlaceFiles(1))
            .map(|s| s.files.clone())
            .unwrap()
    };
    let before = files(&app);
    assert!(!before.is_empty());
    shell.flow(&mut app, F::ChangePlan);
    flow_plan(&mut shell, &mut app, PlanPress::Places(true));
    flow_plan(
        &mut shell,
        &mut app,
        PlanPress::Row(2, Row::Vault(0).code()),
    );
    shell.press(&mut app, Action::BPickDone);
    assert!(
        app.flow()
            .unwrap()
            .slots
            .iter()
            .any(|s| s.target == Target::Place(2)),
        "Vault 1's stick at Place 3"
    );
    assert_eq!(files(&app), before, "the list stays as made");
}

/// A written stick of files is not sent back after the lock; with the
/// vault unlocked again, Back up's change of its files makes it unwritten,
/// to be written again, while a vault's stick that did not change stays
/// written.
#[test]
fn a_written_stick_of_files_is_never_sent_back_after_the_lock() {
    let mut shell = Shell::new();
    sticks(&mut shell, 3);
    let mut app = process(Vec::new(), Vec::new(), Vec::new());
    keep_time(&mut app);
    create_with_vaults(&mut app, false, 1);
    bring_in_all(&mut shell, &mut app, &["S1"]);
    create_vault(&mut shell, &mut app, 0, PHRASE);
    let fp = key_in(&mut app, 1);
    app.press(Action::Nav(Screen::Create));
    app.press(Action::CSlotHere(0, fp));
    if app.create.as_ref().unwrap().built.is_none() {
        app.press(Action::CNext(cstep::KEYS));
    }
    let w = app.create.as_ref().unwrap().built.unwrap();
    let name = app.session.wallets[w].name.clone();
    app.press(Action::CNext(cstep::CHECK));
    app.press(Action::CBackup(1));
    // Vault 1 at both places, the description in it and as files at
    // Place 1, nothing else.
    untick_everywhere(&mut app, Row::Words);
    stick_also_at(&mut app, 0, 0);
    form_off_everywhere(&mut app, Form::Paper);
    form_on(&mut app, 0, Form::File);
    app.press(Action::BChecklist);
    save_vaults(&mut app);
    app.press(Action::BFilesMake);
    app.press(Action::BNext(bstep::ENVELOPE));
    shell.flow(&mut app, F::WalletLock);
    let mut app = restart(&mut shell, &mut app);
    let targets: Vec<Target> = app.flow().unwrap().slots.iter().map(|s| s.target).collect();
    assert_eq!(
        targets,
        vec![Target::Place(0), Target::PlaceFiles(0), Target::Place(1)]
    );
    write_sticks(&mut shell, &mut app, &["S1", "S2"]);
    assert!(
        app.flow().unwrap().slots[1].written.is_some(),
        "the files written"
    );
    // Locked: its files stay.
    shell.flow(&mut app, F::ChangePlan);
    flow_plan(&mut shell, &mut app, PlanPress::Form(0, Form::File.code()));
    assert_eq!(
        app.toast_text(),
        Some(
            "Files and paper stay as made after the lock; finish the writes, then Back up again to change them"
        )
    );
    assert!(app.flow().unwrap().slots[1].written.is_some());
    // A vault unlocked again changes nothing of it: the text form on
    // Back up, the checklist and Lock and write are refused, and every
    // stick keeps its written mark.
    load_first_vault(&mut app, PHRASE);
    let w = app
        .session
        .wallets
        .iter()
        .position(|x| x.name == name)
        .expect("the wallet, from the vault");
    app.press(Action::Backup(w));
    let before: Vec<bool> = app
        .flow()
        .unwrap()
        .slots
        .iter()
        .map(|s| s.written.is_some())
        .collect();
    app.press(Action::BAnswer(qrow::FORM, form::TEXT as u8));
    assert_eq!(
        app.toast_text(),
        Some(
            "Files and paper stay as made after the lock; finish the writes, then Back up again to change them"
        )
    );
    app.press(Action::BChecklist);
    assert_eq!(
        app.toast_text(),
        Some("This backup is past its lock; finish the writes, then Back up again")
    );
    shell.flow(&mut app, F::WalletLock);
    let after: Vec<bool> = app
        .flow()
        .unwrap()
        .slots
        .iter()
        .map(|s| s.written.is_some())
        .collect();
    assert_eq!(after, before, "written marks unchanged");
    assert!(app.flow().unwrap().slots[1].written.is_some());
}

/// A vault that held the bare words before the backup: this wallet's key
/// with its passphrase, taken out of its passphrase after the lock, leaves
/// the sealed copy with the earlier bare record and no record with the
/// passphrase.
#[test]
fn a_passphrase_taken_out_leaves_the_earlier_bare_record_alone() {
    use faraday_vault::records::field;
    let mut shell = Shell::new();
    sticks(&mut shell, 1);
    let main = vault_with_key("Main", PHRASE, 0x41, 1);
    shell.disk_mut("S1").files = vec![("main.ofv".into(), main)];
    let mut app = process(Vec::new(), Vec::new(), Vec::new());
    keep_time(&mut app);
    create_with_vaults(&mut app, false, 1);
    shell.plug(&mut app, "S1");
    let k = app
        .flow()
        .unwrap()
        .live
        .reading
        .as_ref()
        .and_then(|r| r.files.iter().position(|(f, _)| f.name == "main.ofv"));
    if let Some(k) = k {
        shell.flow(&mut app, F::PickFile(0, k));
    }
    shell.flow(&mut app, F::Tap);
    shell.pull(&mut app, "S1");
    unlock(&mut app, PHRASE);
    app.press(Action::Entry(None));
    for c in testkit::test_words(testkit::TEST_SEEDS[1].0).chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    app.press(Action::EntryPassphrase);
    type_text(&mut app, "kept here");
    app.press(Action::EntryAdd);
    let fp = app.session.keys.last().unwrap().master.fingerprint().0;
    app.press(Action::Nav(Screen::Create));
    app.press(Action::CSlotHere(0, fp));
    if app.create.as_ref().unwrap().built.is_none() {
        app.press(Action::CNext(cstep::KEYS));
    }
    app.press(Action::CNext(cstep::CHECK));
    app.press(Action::CBackup(1));
    untick_everywhere(&mut app, Row::Words);
    untick_everywhere(&mut app, Row::Passphrase);
    form_off_everywhere(&mut app, Form::Paper);
    let p = vault_place(&app, 0);
    app.press(Action::BRow(p as u8, Row::Vault(0).code()));
    app.press(Action::BPickPass(0));
    app.press(Action::BPickDone);
    app.press(Action::BChecklist);
    save_vaults(&mut app);
    app.press(Action::BNext(bstep::ENVELOPE));
    shell.flow(&mut app, F::WalletLock);
    let mut app = restart(&mut shell, &mut app);
    shell.flow(&mut app, F::ChangePlan);
    let p = flow_place_of(&app, 0);
    flow_plan(
        &mut shell,
        &mut app,
        PlanPress::Row(p as u8, Row::Vault(0).code()),
    );
    shell.press(&mut app, Action::BPickPass(0));
    shell.press(&mut app, Action::BPickDone);
    assert!(app.flow().unwrap().adding, "{:?}", app.toast_text());
    unlock(&mut app, PHRASE);
    shell.flow(&mut app, F::CopyKey);
    shell.flow(&mut app, F::MadeContinue);
    shell.flow(&mut app, F::LockWrite);
    let mut app = restart(&mut shell, &mut app);
    write_sticks(&mut shell, &mut app, &["S1"]);
    assert_eq!(
        step(&app),
        Step::Done,
        "{:?}",
        app.flow().unwrap().live.problem
    );
    let sealed = ofv(&shell, "S1").expect("Vault 1 on S1");
    let (_, c) = faraday_vault::open(&sealed, PHRASE.as_bytes()).unwrap();
    let keys: Vec<bool> = c
        .of(kind::KEY)
        .map(|(_, r)| r.field(field::KEY_PASSPHRASE).is_some())
        .collect();
    assert_eq!(keys, vec![false], "the earlier bare record alone");
}

// ---- after the lock, files and paper never change in that backup ----

/// F1: with Vault 1 unlocked after the lock, "As a file" and words by hand
/// on the flow's plan card are refused, and no stick is added.
#[test]
fn files_and_words_stay_as_made_after_the_lock_with_a_vault_unlocked() {
    let mut shell = Shell::new();
    let (mut app, _, _) = single_key_ready(&mut shell);
    shell.flow(&mut app, F::WalletLock);
    let mut app = restart(&mut shell, &mut app);
    load_first_vault(&mut app, PHRASE);
    assert!(!app.vaults.open.is_empty(), "Vault 1 is unlocked");
    let slots = app.flow().unwrap().slots.len();
    shell.flow(&mut app, F::ChangePlan);
    flow_plan(&mut shell, &mut app, PlanPress::Form(0, Form::File.code()));
    assert_eq!(
        app.toast_text(),
        Some(
            "Files and paper stay as made after the lock; finish the writes, then Back up again to change them"
        )
    );
    assert_eq!(app.flow().unwrap().slots.len(), slots, "no new stick");
    // Words by hand at Place 1: refused too.
    flow_keep(&mut shell, &mut app, 0, Row::Words, &[0]);
    assert_eq!(
        app.toast_text(),
        Some(
            "Files and paper stay as made after the lock; finish the writes, then Back up again to change them"
        )
    );
    let wf = app.flow().unwrap().wallet.as_ref().unwrap();
    assert_eq!(wf.answers.row_seeds(&wf.shape, 0, Row::Words), None);
}

/// After the lock, another vault holding the same key and wallet,
/// unlocked, with a chart edit, the checklist and Lock and write: the
/// flow's plan, its vaults and its written marks stay as they were.
#[test]
fn another_open_vault_never_replaces_a_backup_past_its_lock() {
    use faraday_core::glance::Press;
    use faraday_core::glance_sheet::ChartAction as C;
    use faraday_vault::Record;
    use faraday_vault::records::{self, field};
    let mut shell = Shell::new();
    let (mut app, w, _) = single_key_ready(&mut shell);
    let desc = app.session.wallets[w].policy.to_descriptor_checksummed();
    let name = app.session.wallets[w].name.clone();
    shell.flow(&mut app, F::WalletLock);
    // A second vault, of another passphrase, with the same key and wallet.
    let words = testkit::test_words(testkit::TEST_SEEDS[1].0);
    let m = osk_bip::bip39::Mnemonic::parse(osk_bip::bip39::Language::English, &words).unwrap();
    let contents = faraday_vault::Contents {
        records: vec![
            Record::new(records::kind::SLOT_LABEL).with(field::LABEL, b"Second"),
            Record::new(kind::KEY)
                .with(field::KEY, &records::words_payload(&m))
                .with(field::KEY_FLAGS, &[records::LOAD_AT_UNLOCK]),
            Record::new(kind::WALLET)
                .with(field::WALLET, desc.as_bytes())
                .with(field::WALLET_NAME, name.as_bytes()),
        ],
    };
    let cost = faraday_vault::Cost {
        memory_kib: faraday_vault::MIN_MEMORY_KIB,
        passes: 1,
        lanes: 1,
    };
    let second = faraday_vault::create(
        faraday_vault::SLOT_SIZES[0],
        cost,
        &[PHRASE_2.as_bytes()],
        &[contents],
        &[0x77; 32],
    )
    .unwrap();
    shell.boxes.0.push(("second.ofv".into(), second));
    let mut app = restart(&mut shell, &mut app);
    let flow_before = app.flow().unwrap().clone();
    // The second vault unlocked.
    app.press(Action::Nav(Screen::Vaults));
    for n in 0..4 {
        if !app.vaults.open.is_empty() {
            break;
        }
        app.press(Action::Vault(faraday_core::vaults::VaultAction::Open(n)));
        app.vaults.ms_per_unit = Some(180);
        unlock(&mut app, PHRASE_2);
    }
    assert_eq!(app.vaults.open.len(), 1);
    app.press(Action::Vault(
        faraday_core::vaults::VaultAction::LoadChosen(0),
    ));
    let w = app
        .session
        .wallets
        .iter()
        .position(|x| x.name == name)
        .expect("the wallet, from the second vault");
    app.press(Action::Backup(w));
    // A chart edit: the description's files at Place 1.
    let press = Press::Loaded(w);
    app.press(Action::Chart(C::Edit(
        press,
        plan::Edit::Add(At::Place(0), What::WalletFile),
    )));
    app.press(Action::Chart(C::Confirm(press)));
    app.press(Action::BChecklist);
    shell.flow(&mut app, F::WalletLock);
    let f = app.flow().unwrap();
    let wf = f.wallet.as_ref().unwrap();
    assert_eq!(
        wf.answers,
        flow_before.wallet.as_ref().unwrap().answers,
        "the plan stays"
    );
    let salts = |f: &BackupFlow| f.plan.vaults.iter().map(|v| v.salt).collect::<Vec<_>>();
    assert_eq!(salts(f), salts(&flow_before), "its vaults stay");
    let marks = |f: &BackupFlow| {
        f.slots
            .iter()
            .map(|s| (s.target, s.written.is_some()))
            .collect::<Vec<_>>()
    };
    assert_eq!(marks(f), marks(&flow_before), "its written marks stay");
}

/// Once a backup is done, Back up again on the wallet is a new backup: a
/// file added at a place is made by its checklist and written by its Lock
/// and write.
#[test]
fn back_up_again_after_done_adds_a_file_and_writes_it() {
    let mut shell = Shell::new();
    let (mut app, _, _) = single_key_ready(&mut shell);
    let mut d = Disk::empty("S4", "KEYSTICK");
    d.serial = Some(4);
    shell.add(d);
    shell.flow(&mut app, F::WalletLock);
    let mut app = restart(&mut shell, &mut app);
    write_sticks(&mut shell, &mut app, &["S1", "S2", "S3"]);
    assert_eq!(step(&app), Step::Done);
    shell.flow(&mut app, F::UnlockDone(0));
    app.vaults.ms_per_unit = Some(180);
    unlock(&mut app, PHRASE);
    assert_eq!(app.screen, Screen::Backup);
    app.press(Action::BPlan);
    form_on(&mut app, 0, Form::File);
    app.press(Action::BChecklist);
    assert_eq!(app.backup.as_ref().unwrap().stage, BStage::Checklist);
    app.press(Action::BFilesMake);
    app.press(Action::BNext(bstep::ENVELOPE));
    shell.flow(&mut app, F::WalletLock);
    let targets: Vec<Target> = app.flow().unwrap().slots.iter().map(|s| s.target).collect();
    assert!(targets.contains(&Target::PlaceFiles(0)), "{targets:?}");
    let mut app = restart(&mut shell, &mut app);
    let k = app
        .flow()
        .unwrap()
        .slots
        .iter()
        .position(|s| s.target == Target::PlaceFiles(0))
        .unwrap();
    let mut order = vec!["S1", "S2", "S3"];
    order.insert(k, "S4");
    write_sticks(&mut shell, &mut app, &order);
    assert_eq!(
        step(&app),
        Step::Done,
        "{:?}",
        app.flow().unwrap().live.problem
    );
    assert!(
        names(&shell, "S4")
            .iter()
            .any(|n| n.ends_with("-descriptor.txt")),
        "{:?}",
        names(&shell, "S4")
    );
}
