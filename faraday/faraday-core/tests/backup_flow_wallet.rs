//! The Bitcoin wallet's part of the guided backup flow
//! (`docs/BACKUP-FLOW.md` §10, §13): the vault part of the plan decided
//! before the keys, the sticks brought in and the vaults opened, the keys
//! made straight into their vaults, the checklist's **Write to sticks**,
//! and every stick written inside the flow.

mod common;

use common::*;
use faraday_core::backup_flow::{BackupFlow, Step, Target};
use faraday_core::backup_flow_steps::FlowAction as F;
use faraday_core::create::NewKind;
use faraday_core::plan::{
    self, Answers, At, Item, Preset, Shape, Spot, Tag, What, form, seeds, wallet,
};
use faraday_core::testkit;
use faraday_core::{Action, BStage, Faraday, Screen, Sheet, bstep, cstep, qrow};
use faraday_vault::records::kind;
use osk_shell_api::{App, Event, Key};

const PHRASE: &str = "first vault phrase";
const PHRASE_2: &str = "second vault phrase";
const PHRASE_3: &str = "third vault phrase";

fn answer(shell: &mut Shell, app: &mut Faraday, list: u8, row: usize) {
    shell.flow(app, F::Answer(list, row as u8));
}

fn answer_backup(app: &mut Faraday, list: u8, row: usize) {
    app.press(Action::BAnswer(list, row as u8));
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

/// Make a new wallet, asked where its keys will be kept; `multi` for the
/// 2-of-3.
fn create_in_vaults(app: &mut Faraday, multi: bool) {
    app.press(Action::CreateWallet);
    if multi {
        app.press(Action::CKind(multi_index()));
    }
    app.press(Action::CForm(seeds::VAULT as u8));
    app.press(Action::CNext(cstep::WHERE));
    assert_eq!(app.screen, Screen::BackupFlow);
    assert_eq!(step(app), Step::Where);
}

/// Steps 2 for the plan's sticks, in order.
fn bring_in_all(shell: &mut Shell, app: &mut Faraday, disks: &[&str]) {
    shell.flow(app, F::Continue);
    for d in disks {
        shell.plug(app, d);
        shell.flow(app, F::Tap);
        shell.pull(app, d);
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
    create_in_vaults(&mut app, false);
    // The seed in Vault 1 only, its stick at both places, the wallet
    // description as files.
    answer(shell, &mut app, qrow::SEEDS, seeds::WORDS);
    answer(shell, &mut app, qrow::STICKS, 0);
    answer(shell, &mut app, qrow::WALLET, wallet::PAPER);
    answer(shell, &mut app, qrow::WALLET, wallet::FILES);
    if !description_in_vault {
        answer(shell, &mut app, qrow::WALLET, wallet::VAULT);
    }
    answer(shell, &mut app, qrow::FORM, form::QR);
    answer(shell, &mut app, qrow::FORM, form::TEXT);
    {
        let f = app.flow().unwrap();
        assert_eq!(f.plan.places, vec![vec![0], vec![0]]);
        assert!(f.plan.files);
        let targets: Vec<Target> = f.slots.iter().map(|s| s.target).collect();
        assert_eq!(
            targets,
            vec![Target::Place(0), Target::Place(1), Target::Files]
        );
    }
    bring_in_all(shell, &mut app, &["S1", "S2"]);
    assert_eq!(step(&app), Step::Vaults);
    create_vault(shell, &mut app, 0, PHRASE);
    // Steps 1 to 3 are done: on to the keys, in Make a new wallet.
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
        1,
        "the key is in Vault 1 as soon as it is chosen"
    );
    if app.create.as_ref().unwrap().built.is_none() {
        app.press(Action::CNext(cstep::KEYS));
    }
    let w = app.create.as_ref().unwrap().built.expect("wallet made");
    assert_eq!(
        app.vaults.open[0].contents.of(kind::WALLET).count(),
        usize::from(description_in_vault),
        "the wallet description is in the vault where the plan puts it"
    );
    app.press(Action::CNext(cstep::CHECK));
    app.press(Action::CBackup(1));
    assert_eq!(app.screen, Screen::Backup);
    // The preset did not undo what the flow decided.
    {
        let b = app.backup.as_ref().unwrap();
        assert!(b.answers.seeds[seeds::VAULT]);
        assert!(b.answers.stick_at(0, 0) && b.answers.stick_at(0, 1));
    }
    answer_backup(&mut app, qrow::WALLET, wallet::PAPER);
    answer_backup(&mut app, qrow::WALLET, wallet::FILES);
    answer_backup(&mut app, qrow::FORM, form::QR);
    answer_backup(&mut app, qrow::FORM, form::TEXT);
    app.press(Action::BChecklist);
    assert_eq!(app.backup.as_ref().unwrap().stage, BStage::Checklist);
    assert_eq!(
        app.backup_items(),
        vec![bstep::VAULT, bstep::PUBLIC, bstep::ENVELOPE, bstep::STICKS]
    );
    assert!(app.backup_item_done(Item::Vault(0)), "made straight in");
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
    sticks(&mut shell, 1);
    let mut app = process(Vec::new(), Vec::new(), Vec::new());
    keep_time(&mut app);
    create_in_vaults(&mut app, false);
    for _ in 0..6 {
        shell.flow(&mut app, F::AddPlace);
    }
    let f = app.flow().unwrap();
    assert_eq!(f.wallet.as_ref().unwrap().answers.places, 8);
    assert_eq!(f.plan.places.len(), 8);
}

/// A 2-of-3 with Vault 1 an existing vault on S1 (beside another vault),
/// Vaults 2 and 3 new on S2 and S3, one key in each; stopped on the
/// checklist's Write to sticks. `software` chooses Sparrow's files too.
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
    create_in_vaults(&mut app, true);
    shell.flow(&mut app, F::VaultExisting(0));
    {
        let f = app.flow().unwrap();
        assert_eq!(f.plan.places, vec![vec![0], vec![1], vec![2]]);
    }
    shell.flow(&mut app, F::Continue);
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
    // One key in each vault.
    assert_eq!(app.vaults.open.len(), 3);
    for o in &app.vaults.open {
        assert_eq!(o.contents.of(kind::KEY).count(), 1);
    }
    app.press(Action::CNext(cstep::CHECK));
    app.press(Action::CBackup(if software { 2 } else { 1 }));
    answer_backup(&mut app, qrow::SEEDS, seeds::WORDS);
    if software {
        answer_backup(&mut app, qrow::SOFTWARE, plan::software::SPARROW);
        answer_backup(&mut app, qrow::SOFTWARE, plan::software::NOT_SURE);
        answer_backup(&mut app, qrow::FORM, form::TEXT);
        answer_backup(&mut app, qrow::FORM, form::QR);
    }
    app.press(Action::BChecklist);
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
    create_in_vaults(&mut app, true);
    // Key 3 is not in a vault: Vault 1 and Vault 2 only.
    answer(&mut shell, &mut app, qrow::VAULTS + 2, 2);
    {
        let f = app.flow().unwrap();
        assert_eq!(f.plan.used_vaults(), vec![0, 1]);
    }
    bring_in_all(&mut shell, &mut app, &["S1", "S2"]);
    create_vault(&mut shell, &mut app, 0, PHRASE);
    create_vault(&mut shell, &mut app, 1, PHRASE_2);
    assert_eq!(app.screen, Screen::Create, "{:?}", app.vaults.unlock_error);
    app.press(Action::Nav(Screen::Create));
    let fps: Vec<[u8; 4]> = (0..3u8).map(|s| new_key_at(&mut app, s)).collect();
    app.press(Action::CNext(cstep::KEYS));
    assert!(app.create.as_ref().unwrap().built.is_some(), "wallet made");
    // Keys 1 and 2 went into their vaults; Key 3 into none.
    assert_eq!(app.vaults.open.len(), 2);
    for o in &app.vaults.open {
        assert_eq!(o.contents.of(kind::KEY).count(), 1);
    }
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
fn the_lines_are_drawn_and_add_a_passphrase_marks_the_key_before_it_exists() {
    let mut shell = Shell::new();
    sticks(&mut shell, 1);
    let mut app = process(Vec::new(), Vec::new(), Vec::new());
    keep_time(&mut app);
    create_in_vaults(&mut app, false);
    // A single key, no passphrase: both lines, under the checks.
    expect_part(
        &mut app,
        "A passphrase kept at another place than the seed is recommended",
    );
    assert!(offered(&mut app, Action::PlanAddPass));
    shell.press(&mut app, Action::PlanAddPass);
    assert!(app.create.as_ref().unwrap().pass_marked);
    let a = &app.flow().unwrap().wallet.as_ref().unwrap().answers;
    assert!(
        a.pass.first().is_some_and(|r| !r.is_empty()),
        "a row for the passphrase"
    );
    let place = (0..a.places).find(|&p| a.pass_at(0, p)).expect("its place");
    assert!(
        !a.seed_at(
            &app.flow().unwrap().wallet.as_ref().unwrap().shape,
            0,
            place
        ),
        "apart from the seed"
    );
    assert!(
        !has_part(&mut app, "A passphrase kept at another place"),
        "marked"
    );
    // Nothing is blocked: Continue is offered.
    assert!(offered(&mut app, Action::Flow(F::Continue)));
}

#[test]
fn a_one_place_plan_shows_the_warning_and_blocks_nothing() {
    let mut shell = Shell::new();
    sticks(&mut shell, 1);
    let mut app = process(Vec::new(), Vec::new(), Vec::new());
    keep_time(&mut app);
    create_in_vaults(&mut app, false);
    answer(&mut shell, &mut app, qrow::PLACES, 1);
    assert_eq!(app.flow().unwrap().plan.places.len(), 1);
    expect_part(&mut app, "Every secret of this wallet is at Place 1");
    expect_part(&mut app, "Separate places are strongly");
    assert!(
        offered(&mut app, Action::Flow(F::Continue)),
        "nothing is blocked"
    );
}

#[test]
fn restore_takes_the_same_steps_and_a_restored_key_signs_at_once() {
    let mut shell = Shell::new();
    sticks(&mut shell, 2);
    let mut app = process(Vec::new(), Vec::new(), Vec::new());
    keep_time(&mut app);
    app.press(Action::RestoreWallet);
    app.press(Action::RForm(seeds::VAULT as u8));
    app.press(Action::RNext(faraday_core::rstep::WHERE));
    assert_eq!(app.screen, Screen::BackupFlow);
    answer(&mut shell, &mut app, qrow::SEEDS, seeds::WORDS);
    bring_in_all(&mut shell, &mut app, &["S1"]);
    create_vault(&mut shell, &mut app, 0, PHRASE);
    // Steps 1 to 3 done: back in Restore, on its seeds.
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
    // The seed typed went into the vault the plan names.
    assert_eq!(app.vaults.open[0].contents.of(kind::KEY).count(), 1);
    assert_eq!(app.vaults.open[0].contents.of(kind::WALLET).count(), 1);
    app.press(Action::Backup(w));
    app.press(Action::BPreset(1));
    app.press(Action::BChecklist);
    app.press(Action::BOut(3));
    app.press(Action::BNext(bstep::ENVELOPE));
    assert!(app.held_keys().is_empty(), "a restored key signs at once");
    shell.flow(&mut app, F::WalletLock);
    let mut app = restart(&mut shell, &mut app);
    assert_eq!(step(&app), Step::Write);
    shell.plug(&mut app, "S1");
    shell.pull(&mut app, "S1");
    shell.plug(&mut app, "S2");
    shell.flow(&mut app, F::Tap);
    shell.pull(&mut app, "S2");
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
    answer(&mut shell, &mut app, qrow::VAULTS + 1, 0);
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
    assert_eq!(step(&app), Step::Done);
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
    for (disk, tap) in [("S1", false), ("S2", false), ("S3", true)] {
        shell.plug(&mut app, disk);
        if tap {
            shell.flow(&mut app, F::Tap);
        } else if disk == "S2" {
            shell.flow(&mut app, F::Hold);
        }
        shell.pull(&mut app, disk);
    }
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
    create_in_vaults(&mut app, false);
    // The seed in Vault 1 and as a file; nothing else.
    answer(&mut shell, &mut app, qrow::SEEDS, seeds::WORDS);
    answer(&mut shell, &mut app, qrow::SEEDS, seeds::FILE);
    answer(&mut shell, &mut app, qrow::WALLET, wallet::PAPER);
    {
        let f = app.flow().unwrap();
        assert!(f.plan.files, "the seed's file is on a stick of files");
        let t: Vec<Target> = f.slots.iter().map(|s| s.target).collect();
        assert_eq!(t, vec![Target::Place(1), Target::Files]);
    }
    bring_in_all(&mut shell, &mut app, &["S1"]);
    create_vault(&mut shell, &mut app, 0, PHRASE);
    let fp = key_in(&mut app, 1);
    app.press(Action::Nav(Screen::Create));
    app.press(Action::CSlotHere(0, fp));
    app.press(Action::CNext(cstep::KEYS));
    app.press(Action::CNext(cstep::CHECK));
    app.press(Action::CBackup(1));
    answer_backup(&mut app, qrow::WALLET, wallet::PAPER);
    app.press(Action::BChecklist);
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
    app.press(Action::BAnswer(qrow::PLACES, 3));
    app.press(Action::BAnswer(qrow::STICKS, 2));
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
    assert!(ofv(&shell, "S3").is_some(), "the new place was written");
    no_visit(&shell);
}

#[test]
fn a_wallet_left_with_its_vaults_open_does_not_take_the_next_wallets_keys() {
    let mut shell = Shell::new();
    sticks(&mut shell, 2);
    let mut app = process(Vec::new(), Vec::new(), Vec::new());
    keep_time(&mut app);
    // Wallet A: its vault made and open, then left.
    create_in_vaults(&mut app, false);
    answer(&mut shell, &mut app, qrow::SEEDS, seeds::WORDS);
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
    for (list, row) in [
        (qrow::SEEDS, seeds::VAULT),
        (qrow::SEEDS, seeds::WORDS),
        (qrow::SEEDS, seeds::FILE),
        (qrow::WALLET, wallet::PAPER),
        (qrow::WALLET, wallet::VAULT),
    ] {
        answer_backup(&mut app, list, row);
    }
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
    create_in_vaults(&mut app, true);
    shell.flow(&mut app, F::Continue);
    for d in ["S1", "S2", "S3"] {
        shell.plug(&mut app, d);
        shell.flow(&mut app, F::Tap);
        shell.pull(&mut app, d);
    }
    create_vault(&mut shell, &mut app, 0, PHRASE);
    create_vault(&mut shell, &mut app, 1, PHRASE_2);
    create_vault(&mut shell, &mut app, 2, PHRASE_3);
    assert_eq!(app.screen, Screen::Create);
    let salts: Vec<[u8; 32]> = app
        .flow()
        .map(|f| f.plan.vaults.iter().map(|v| v.salt.unwrap()).collect())
        .unwrap_or_else(|| {
            app.backup_flows[0]
                .plan
                .vaults
                .iter()
                .map(|v| v.salt.unwrap())
                .collect()
        });
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
    // The plan's vaults follow their rows: the third key's vault is the
    // second now, with its own salt.
    let f = app
        .backup_flows
        .iter()
        .find(|f| f.wallet.is_some())
        .unwrap();
    let now: Vec<Option<[u8; 32]>> = f.plan.vaults.iter().map(|v| v.salt).collect();
    assert_eq!(now[0], Some(salts[0]));
    assert_eq!(now[1], Some(salts[2]), "Key 3's vault kept its salt");
    // And Key 3 is in the vault with that salt.
    let o = app
        .vaults
        .open
        .iter()
        .find(|o| o.header().salt == salts[2])
        .unwrap();
    assert_eq!(o.contents.of(kind::KEY).count(), 1);
    let _ = (salts[1], PHRASE_3);
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
    answer(&mut shell, &mut app, qrow::VAULTS + 1, 0);
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
    create_in_vaults(&mut app, true);
    shell.flow(&mut app, F::VaultExisting(0));
    shell.flow(&mut app, F::Continue);
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
    answer_backup(&mut app, qrow::SEEDS, seeds::WORDS);
    app.press(Action::BChecklist);
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
    assert!(!app.create_in_vaults());
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(faraday_core::vaults::VaultAction::Open(0)));
    app.vaults.ms_per_unit = Some(180);
    unlock(&mut app, PHRASE);
    app.press(Action::Nav(Screen::Create));
    // Paper and vault, chosen after the keys: the vault is the open one,
    // the key goes into it, its stick is checked at its write.
    app.press(Action::CBackup(1));
    assert!(app.create_in_vaults(), "Where reads what was chosen");
    answer_backup(&mut app, qrow::SEEDS, seeds::WORDS);
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
    assert!(app.create_in_vaults(), "Where reads what was decided");
    for k in [cstep::WHERE, cstep::STICKS, cstep::VAULTS] {
        assert!(app.create_card_done(k), "card {k} reads done");
    }
    // Check's Continue opens Back up's card, not Where.
    app.press(Action::CNext(cstep::CHECK));
    assert_eq!(app.create.as_ref().unwrap().open, Some(cstep::BACKUP));
}

#[test]
fn a_key_that_leaves_its_slot_comes_out_of_the_vault_and_a_passphrase_added_later_leaves_one_record()
 {
    let mut shell = Shell::new();
    sticks(&mut shell, 1);
    let mut app = process(Vec::new(), Vec::new(), Vec::new());
    keep_time(&mut app);
    create_in_vaults(&mut app, false);
    answer(&mut shell, &mut app, qrow::SEEDS, seeds::WORDS);
    bring_in_all(&mut shell, &mut app, &["S1"]);
    create_vault(&mut shell, &mut app, 0, PHRASE);
    let a = key_in(&mut app, 0);
    let b = key_in(&mut app, 1);
    app.press(Action::Nav(Screen::Create));
    app.press(Action::CSlotHere(0, a));
    assert_eq!(app.vaults.open[0].contents.of(kind::KEY).count(), 1);
    // Cleared: it comes out.
    app.press(Action::CSlotClear(0));
    assert_eq!(app.vaults.open[0].contents.of(kind::KEY).count(), 0);
    // Replaced by another key: only that one is in.
    app.press(Action::CSlotHere(0, a));
    app.press(Action::CSlotHere(0, b));
    assert_eq!(app.vaults.open[0].contents.of(kind::KEY).count(), 1);
    // A passphrase added later: one record of the seed, not two.
    app.press(Action::CPassOpen(0));
    for f in [0u8, 1] {
        app.press(Action::CPassField(f));
        type_text(&mut app, "later pass");
    }
    app.press(Action::CPassLock);
    assert_eq!(
        app.vaults.open[0].contents.of(kind::KEY).count(),
        1,
        "one record of the seed"
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
    let w = app.flow().unwrap().wallet.as_ref().unwrap().answers.wallet;
    answer(&mut shell, &mut app, qrow::WALLET, wallet::VAULT);
    let w2 = app.flow().unwrap().wallet.as_ref().unwrap().answers.wallet;
    assert_ne!(
        w[wallet::VAULT],
        w2[wallet::VAULT],
        "{w:?} {w2:?} {:?}",
        app.toast_text()
    );
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
    create_in_vaults(&mut app, false);
    shell.flow(&mut app, F::Back);
    assert_eq!(app.screen, Screen::Create);
    // A stick written stays: taking it off the plan says why.
    let (mut app, _, _) = single_key_ready(&mut shell);
    shell.flow(&mut app, F::WalletLock);
    let mut app = restart(&mut shell, &mut app);
    write_sticks(&mut shell, &mut app, &["S1"]);
    shell.flow(&mut app, F::ChangePlan);
    answer(&mut shell, &mut app, qrow::STICKS, 0);
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
    // Into vaults ticked off: the key would be kept on no stick.
    answer(&mut shell, &mut app, qrow::SEEDS, seeds::VAULT);
    assert_eq!(
        app.toast_text(),
        Some("A key not yet written stays kept on a stick")
    );
    assert!(app.flow().unwrap().wallet.as_ref().unwrap().answers.seeds[seeds::VAULT]);
    assert_eq!(app.flow().unwrap().slots.len(), slots);
    // Vault 1's stick at the first place goes; the one at the second stays.
    answer(&mut shell, &mut app, qrow::STICKS, 0);
    assert_eq!(app.flow().unwrap().slots.len(), slots - 1);
    // The last stick of the vault is refused, with its toast.
    answer(&mut shell, &mut app, qrow::STICKS, 1);
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
    create_in_vaults(&mut app, false);
    answer(&mut shell, &mut app, qrow::SEEDS, seeds::WORDS);
    answer(&mut shell, &mut app, qrow::SEEDS, seeds::FILE);
    answer(&mut shell, &mut app, qrow::WALLET, wallet::PAPER);
    bring_in_all(&mut shell, &mut app, &["S1"]);
    create_vault(&mut shell, &mut app, 0, PHRASE);
    app.press(Action::Nav(Screen::Create));
    let fp = new_key_here(&mut app);
    app.press(Action::CNext(cstep::CHECK));
    app.press(Action::CBackup(1));
    answer_backup(&mut app, qrow::WALLET, wallet::PAPER);
    app.press(Action::BChecklist);
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
        let places = wf.answers.places;
        wf.answers.pass[0] = (0..=places).map(|p| p == places).collect();
        wf.answers.seeds[seeds::FILE] = true;
        assert!(!wf.held.is_empty());
    }
    shell.flow(&mut app, F::ChangePlan);
    answer(&mut shell, &mut app, qrow::SEEDS, seeds::VAULT);
    assert_eq!(
        app.toast_text(),
        Some("A key not yet written stays kept on a stick")
    );
    assert!(app.flow().unwrap().wallet.as_ref().unwrap().answers.seeds[seeds::VAULT]);
}

#[test]
fn two_planned_vaults_holding_the_same_seed_are_never_one_open_vault() {
    let mut shell = Shell::new();
    let mut app = two_of_three_ready_with(&mut shell, false);
    // Vault 2 holds Key 1 too, in place of Key 2.
    app.press(Action::BPlan);
    answer_backup(&mut app, qrow::VAULTS + 1, 0);
    answer_backup(&mut app, qrow::VAULTS + 1, 1);
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
    create_in_vaults(&mut app, false);
    answer(&mut shell, &mut app, qrow::SEEDS, seeds::WORDS);
    answer(&mut shell, &mut app, qrow::SEEDS, seeds::FILE);
    answer(&mut shell, &mut app, qrow::WALLET, wallet::PAPER);
    bring_in_all(&mut shell, &mut app, &["S1"]);
    create_vault(&mut shell, &mut app, 0, PHRASE);
    let fp = key_in(&mut app, 1);
    app.press(Action::Nav(Screen::Create));
    app.press(Action::CSlotHere(0, fp));
    app.press(Action::CNext(cstep::KEYS));
    app.press(Action::CNext(cstep::CHECK));
    app.press(Action::CBackup(1));
    answer_backup(&mut app, qrow::WALLET, wallet::PAPER);
    app.press(Action::BChecklist);
    // The seed's file is let out on the secret sheet.
    app.press(Action::BFile);
    app.press(Action::SecretAck);
    app.press(Action::SecretUnprotected);
    assert!(app.outbox.iter().any(|i| i.secret));
    // Then the plan drops seed files and keeps the wallet as files.
    answer_backup(&mut app, qrow::SEEDS, seeds::FILE);
    answer_backup(&mut app, qrow::WALLET, wallet::FILES);
    answer_backup(&mut app, qrow::FORM, form::QR);
    answer_backup(&mut app, qrow::FORM, form::TEXT);
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
fn a_record_the_vault_held_before_the_flow_is_never_taken_out_again() {
    let mut shell = Shell::new();
    sticks(&mut shell, 2);
    // Main holds the key of Key 2 (TEST_SEEDS[1]) already.
    let main = vault_with_key("Main", PHRASE, 0x41, 1);
    shell.disk_mut("S1").files = vec![("main.ofv".into(), main)];
    let mut app = process(Vec::new(), Vec::new(), Vec::new());
    keep_time(&mut app);
    create_in_vaults(&mut app, false);
    answer(&mut shell, &mut app, qrow::SEEDS, seeds::WORDS);
    shell.flow(&mut app, F::VaultExisting(0));
    shell.flow(&mut app, F::Continue);
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
    shell.plug(&mut app, "S2");
    shell.flow(&mut app, F::Tap);
    shell.pull(&mut app, "S2");
    unlock(&mut app, PHRASE);
    assert_eq!(app.screen, Screen::Create, "{:?}", app.vaults.unlock_error);
    let keys = |app: &Faraday| app.vaults.open[0].contents.of(kind::KEY).count();
    assert_eq!(keys(&app), 1, "the vault held the key already");
    let a = key_in(&mut app, 1);
    let b = key_in(&mut app, 2);
    app.press(Action::Nav(Screen::Create));
    // The key it held is chosen, then cleared: still there.
    app.press(Action::CSlotHere(0, a));
    assert_eq!(keys(&app), 1);
    app.press(Action::CSlotClear(0));
    assert_eq!(keys(&app), 1, "cleared: the record that was there stays");
    // Swapped for another: the other is the flow's, the first stays.
    app.press(Action::CSlotHere(0, a));
    app.press(Action::CSlotHere(0, b));
    assert_eq!(keys(&app), 2, "swapped: the old one stays, the new is in");
    app.press(Action::CSlotClear(0));
    assert_eq!(keys(&app), 1, "and what the flow put in goes");
}

#[test]
fn a_vault_row_left_with_no_seed_is_dropped_and_the_vaults_follow_when_the_backup_opens() {
    let mut shell = Shell::new();
    sticks(&mut shell, 3);
    let mut app = process(Vec::new(), Vec::new(), Vec::new());
    keep_time(&mut app);
    create_in_vaults(&mut app, true);
    shell.flow(&mut app, F::Continue);
    for d in ["S1", "S2", "S3"] {
        shell.plug(&mut app, d);
        shell.flow(&mut app, F::Tap);
        shell.pull(&mut app, d);
    }
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
    // Key 2's words go: its vault has no seed here when the plan is read.
    app.session.keys.retain(|k| k.master.fingerprint().0 != b);
    app.press(Action::CNext(cstep::CHECK));
    app.press(Action::Backup(w));
    let f = app
        .backup_flows
        .iter()
        .find(|f| f.wallet.is_some())
        .unwrap();
    let now: Vec<Option<[u8; 32]>> = f.plan.vaults.iter().map(|v| v.salt).collect();
    assert_eq!(now.len(), 2, "the empty row is gone");
    assert_eq!(now[0], salts[0]);
    assert_eq!(now[1], salts[2], "Key 3's vault kept its salt");
}

#[test]
fn the_lock_keeps_the_flows_own_vault_when_another_open_vault_holds_the_same_key() {
    let mut shell = Shell::new();
    sticks(&mut shell, 2);
    let other = vault_with_key("Other", PHRASE_2, 0x42, 1);
    let mut app = process(vec![("other.ofv".into(), other)], Vec::new(), Vec::new());
    keep_time(&mut app);
    create_in_vaults(&mut app, false);
    answer(&mut shell, &mut app, qrow::SEEDS, seeds::WORDS);
    answer(&mut shell, &mut app, qrow::WALLET, wallet::PAPER);
    bring_in_all(&mut shell, &mut app, &["S1"]);
    create_vault(&mut shell, &mut app, 0, PHRASE);
    let own = app.backup_flows[0].plan.vaults[0].salt.unwrap();
    let fp = key_in(&mut app, 1);
    app.press(Action::Nav(Screen::Create));
    app.press(Action::CSlotHere(0, fp));
    app.press(Action::CNext(cstep::KEYS));
    app.press(Action::CNext(cstep::CHECK));
    app.press(Action::CBackup(1));
    answer_backup(&mut app, qrow::WALLET, wallet::PAPER);
    app.press(Action::BChecklist);
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
    use faraday_core::backup_flow::Step as S;
    let mut shell = Shell::new();
    sticks(&mut shell, 2);
    let vb = vault("Second", &[PHRASE_2], 0x43);
    let mut app = process(vec![("second.ofv".into(), vb)], Vec::new(), Vec::new());
    keep_time(&mut app);
    // Wallet A: its vault made and open.
    create_in_vaults(&mut app, false);
    answer(&mut shell, &mut app, qrow::SEEDS, seeds::WORDS);
    bring_in_all(&mut shell, &mut app, &["S1"]);
    create_vault(&mut shell, &mut app, 0, PHRASE);
    app.press(Action::Nav(Screen::Home));
    // Wallet B in vaults: its flow, and a vault of its own.
    app.press(Action::CreateWallet);
    app.press(Action::CForm(seeds::VAULT as u8));
    app.press(Action::CNext(cstep::WHERE));
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
    app.backup_flows[1].step = S::Make;
    app.backup_flows[1].plan.vaults[0].salt = Some(sb);
    app.backup_flows[1].made_in = vec![0];
    let fp = key_in(&mut app, 1);
    app.press(Action::Nav(Screen::Create));
    app.press(Action::CSlotHere(0, fp));
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
    answer(&mut shell, &mut app, qrow::VAULTS + 1, 0);
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
    let before = app.flow().unwrap().wallet.as_ref().unwrap().answers.pass[0].clone();
    assert!(!before[places], "kept apart at the lock");
    answer(&mut shell, &mut app, qrow::PASS, places);
    assert!(
        app.flow().unwrap().wallet.as_ref().unwrap().answers.pass[0][places],
        "{:?}",
        app.toast_text()
    );
    assert!(app.flow().unwrap().adding, "its vault must hold it now");
}

fn forms(app: &Faraday) -> [bool; 4] {
    app.create.as_ref().unwrap().forms.0
}

fn flow_names(app: &Faraday) -> Vec<String> {
    (1..=app.flow().unwrap().slots.len())
        .map(|k| format!("S{k}"))
        .collect()
}

#[test]
fn words_and_vaults_ticked_together_keep_both() {
    let mut shell = Shell::new();
    sticks(&mut shell, 3);
    let mut app = process(Vec::new(), Vec::new(), Vec::new());
    keep_time(&mut app);
    app.press(Action::CreateWallet);
    // Words on paper is ticked to begin with; vaults is ticked with it.
    assert_eq!(forms(&app), [true, false, false, false]);
    app.press(Action::CForm(seeds::VAULT as u8));
    assert_eq!(forms(&app), [true, false, true, false]);
    app.press(Action::CNext(cstep::WHERE));
    assert_eq!(app.screen, Screen::BackupFlow);
    assert_eq!(
        app.flow().unwrap().wallet.as_ref().unwrap().answers.seeds,
        [true, false, true, false]
    );
    // The sticks and the vaults come before the keys.
    assert!(!app.create_card_done(cstep::STICKS) && !app.create_card_done(cstep::VAULTS));
    let names = flow_names(&app);
    let names: Vec<&str> = names.iter().map(String::as_str).collect();
    bring_in_all(&mut shell, &mut app, &names);
    create_vault(&mut shell, &mut app, 0, PHRASE);
    assert_eq!(app.screen, Screen::Create, "{:?}", app.vaults.unlock_error);
    assert!(app.create_card_done(cstep::STICKS) && app.create_card_done(cstep::VAULTS));
    assert_eq!(forms(&app), [true, false, true, false]);
    app.press(Action::Nav(Screen::Create));
    new_key(&mut app);
    if app.create.as_ref().unwrap().built.is_none() {
        app.press(Action::CNext(cstep::KEYS));
    }
    assert!(app.create.as_ref().unwrap().built.is_some());
    app.press(Action::CNext(cstep::CHECK));
    app.press(Action::CBackup(0));
    assert_eq!(app.screen, Screen::Backup);
    // Back up shows the forms chosen on the card; it does not ask again.
    let b = app.backup.as_ref().unwrap();
    assert_eq!(b.answers.seeds, [true, false, true, false]);
    assert!(b.answers.paper_seeds());
}

#[test]
fn forms_other_than_vaults_need_no_flow() {
    let mut app = process(Vec::new(), Vec::new(), Vec::new());
    keep_time(&mut app);
    app.press(Action::CreateWallet);
    app.press(Action::CForm(seeds::WORDS as u8));
    app.press(Action::CForm(seeds::SEEDQR as u8));
    app.press(Action::CForm(seeds::FILE as u8));
    assert_eq!(forms(&app), [false, true, false, true]);
    app.press(Action::CNext(cstep::WHERE));
    assert!(app.backup_flows.is_empty());
    assert_eq!(app.screen, Screen::Create);
}

#[test]
fn unticking_every_form_stops_continue() {
    let mut app = process(Vec::new(), Vec::new(), Vec::new());
    keep_time(&mut app);
    app.press(Action::CreateWallet);
    app.press(Action::CForm(seeds::VAULT as u8));
    app.press(Action::CForm(seeds::WORDS as u8));
    app.press(Action::CForm(seeds::VAULT as u8));
    assert_eq!(forms(&app), [false; 4]);
    let open = app.create.as_ref().unwrap().open;
    app.press(Action::CNext(cstep::WHERE));
    assert_eq!(app.create.as_ref().unwrap().open, open);
    assert!(app.backup_flows.is_empty());
    assert!(!app.create.as_ref().unwrap().done[cstep::WHERE as usize]);
    // The same on Restore.
    app.press(Action::RestoreWallet);
    app.press(Action::RForm(seeds::WORDS as u8));
    assert_eq!(app.restore.as_ref().unwrap().forms.0, [false; 4]);
    let open = app.restore.as_ref().unwrap().open;
    app.press(Action::RNext(faraday_core::rstep::WHERE));
    assert_eq!(app.restore.as_ref().unwrap().open, open);
    assert!(app.backup_flows.is_empty());
}

#[test]
fn restore_takes_every_form_ticked_to_back_up() {
    let mut shell = Shell::new();
    sticks(&mut shell, 3);
    let mut app = process(Vec::new(), Vec::new(), Vec::new());
    keep_time(&mut app);
    app.press(Action::RestoreWallet);
    app.press(Action::RForm(seeds::VAULT as u8));
    app.press(Action::RForm(seeds::FILE as u8));
    assert_eq!(
        app.restore.as_ref().unwrap().forms.0,
        [true, false, true, true]
    );
    app.press(Action::RNext(faraday_core::rstep::WHERE));
    assert_eq!(app.screen, Screen::BackupFlow);
    assert_eq!(
        app.flow().unwrap().wallet.as_ref().unwrap().answers.seeds,
        [true, false, true, true]
    );
    let names = flow_names(&app);
    let names: Vec<&str> = names.iter().map(String::as_str).collect();
    bring_in_all(&mut shell, &mut app, &names);
    create_vault(&mut shell, &mut app, 0, PHRASE);
    assert_eq!(app.screen, Screen::Restore, "{:?}", app.vaults.unlock_error);
    app.press(Action::RSlotWords);
    type_text(&mut app, &testkit::test_words(testkit::TEST_SEEDS[1].0));
    app.press(Action::EntryAdd);
    app.press(Action::Seeds(faraday_core::seeds::SeedsAction::Make));
    let w = app.restore.as_ref().unwrap().wallet.expect("wallet made");
    app.press(Action::Backup(w));
    app.press(Action::BPreset(0));
    assert_eq!(
        app.backup.as_ref().unwrap().answers.seeds,
        [true, false, true, true]
    );
}

#[test]
fn where_is_never_done_before_the_person_has_continued_on_it() {
    let mut app = process(Vec::new(), Vec::new(), Vec::new());
    keep_time(&mut app);
    app.press(Action::CreateWallet);
    assert!(!app.create_card_done(cstep::WHERE));
    // Kind's Continue opens Where, and it is not done.
    app.press(Action::CNext(cstep::KIND));
    assert_eq!(app.create.as_ref().unwrap().open, Some(cstep::WHERE));
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
    assert!(app.restore.as_ref().unwrap().done[faraday_core::rstep::WHERE as usize]);
}
