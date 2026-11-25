//! Make a new wallet's row for a vault already on a stick, on Keys,
//! before any key exists: it brings in only that stick and unlocks that
//! vault; a new vault is not made there but on the backup plan.

use crate::backup_flow_wallet::{PHRASE, step};
use crate::common::*;
use faraday_core::backup_flow::Step;
use faraday_core::backup_flow::steps::FlowAction as F;
use faraday_core::{Action, Faraday, Screen, Sheet, cstep};

/// A process on the stick, its clock kept, with Keys' row for a vault
/// already on a stick turned on: the one stick is asked for, no key
/// exists.
fn on_the_sticks() -> Faraday {
    let mut app = process(Vec::new(), Vec::new(), Vec::new());
    keep_time(&mut app);
    create_single_key(&mut app);
    assert_eq!(app.create().and_then(|c| c.open), Some(cstep::KEYS));
    assert!(
        !app.create().unwrap().uses_existing_vault(),
        "off by default"
    );
    app.press(Action::CExistingVault(true));
    assert_eq!(app.screen, Screen::BackupFlow);
    assert_eq!(step(&app), Step::Sticks);
    app
}

/// A stick holding no vault is not made into one here: the flow never
/// opens Make a new vault, and says a new vault is added on the plan.
#[test]
fn a_stick_with_no_vault_is_not_made_into_one_before_the_keys() {
    let mut shell = Shell::new();
    shell.add(Disk::empty("S1", "KEYSTICK"));
    let mut app = on_the_sticks();
    shell.plug(&mut app, "S1");
    shell.flow(&mut app, F::Tap);
    shell.pull(&mut app, "S1");
    assert_eq!(step(&app), Step::Vaults);
    assert_eq!(app.sheet, None::<Sheet>);
    assert_eq!(app.flow().unwrap().live.creating, None);
    expect_wrapped(
        &mut app,
        "No vault on this stick. A new vault is added on the backup plan",
    );
}

/// The setup has no counts and no Check it later: a waiting stick is asked
/// for and nothing is put off, and a stick read says how many are in.
#[test]
fn the_setup_shows_no_counts_and_offers_no_check_it_later() {
    let mut shell = Shell::new();
    shell.add(Disk::empty("S1", "KEYSTICK"));
    let mut app = on_the_sticks();
    app.press(Action::Flow(F::LaterAsk(0)));
    assert_eq!(app.sheet, None::<Sheet>, "nothing is put off");
    let shown = texts(&mut app).join("\n");
    assert!(!shown.contains("Check it later"), "{shown}");
    assert!(!shown.contains(" of 1 "), "{shown}");
    shell.plug(&mut app, "S1");
    shell.flow(&mut app, F::Tap);
    let shown = texts(&mut app).join("\n");
    assert!(!shown.contains(" of 1 "), "{shown}");
}

/// A stick that is not FAT32 shows its problem on its figure and is not
/// bound.
#[test]
fn a_stick_that_is_not_fat32_shows_its_problem_and_is_not_bound() {
    let mut shell = Shell::new();
    let mut bad = Disk::empty("S1", "KEYSTICK");
    bad.not_fat = true;
    shell.add(bad);
    let mut app = on_the_sticks();
    shell.plug(&mut app, "S1");
    expect_wrapped(
        &mut app,
        "Not a FAT32 stick. Format it as FAT32, or use another stick",
    );
    assert!(app.flow().unwrap().slots.iter().all(|s| s.bound.is_none()));
}

/// A wallet whose vault is a new one added on the plan: no stick is read
/// before the keys, and each stick (the vault's and the public files') is inserted once, at the write.
#[test]
fn a_new_vault_added_on_the_plan_is_written_to_its_stick_inserted_once_at_the_write() {
    use crate::backup_flow_wallet::{
        make_plan_vault, new_key, restart, save_vaults, sticks, write_sticks,
    };
    use faraday_core::plan::{Item, Preset};
    let mut shell = Shell::new();
    sticks(&mut shell, 2);
    let mut app = process(Vec::new(), Vec::new(), Vec::new());
    keep_time(&mut app);
    create_single_key(&mut app);
    assert_eq!(app.screen, Screen::Create, "no stick is asked for first");
    new_key(&mut app);
    if app.create().unwrap().built.is_none() {
        app.press(Action::CNext(cstep::KEYS));
    }
    app.press(Action::CNext(cstep::CHECK));
    app.press(Action::CBackup);
    faraday_core::testkit::backup_as(&mut app, Preset::PaperVault);
    no_words(&mut app);
    crate::common::stick_also_at(&mut app, 0, 0);
    app.press(Action::BChecklist);
    make_plan_vault(&mut app, 0, PHRASE);
    save_vaults(&mut app);
    app.press(Action::BFilesMake);
    app.press(Action::BNext(Item::Envelopes));
    crate::common::top_up(&mut app);
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
    assert!(
        crate::backup_flow_wallet::kdbx(&shell, "S1").is_some(),
        "the vault is on the stick"
    );
}

/// A vault already on a stick is brought in first, before any key, and
/// its row is refused once a key exists.
#[test]
fn a_vault_brought_in_first_is_open_for_the_keys_and_the_row_is_refused_once_a_key_exists() {
    use crate::backup_flow_wallet::{bring_in_existing, key_in, sticks};
    let mut shell = Shell::new();
    sticks(&mut shell, 1);
    let mut app = process(Vec::new(), Vec::new(), Vec::new());
    keep_time(&mut app);
    create_single_key(&mut app);
    bring_in_existing(&mut shell, &mut app);
    assert_eq!(
        app.vaults().open.len(),
        1,
        "the vault is open before any key"
    );
    assert!(app.create().unwrap().uses_existing_vault());
    // With a key held, the row is not offered and pressing it does nothing.
    let mut app = process(Vec::new(), Vec::new(), Vec::new());
    keep_time(&mut app);
    create_single_key(&mut app);
    let fp = key_in(&mut app, 1);
    app.press(Action::Nav(Screen::Create));
    app.press(Action::CSlotHere(0, fp));
    app.press(Action::CExistingVault(true));
    assert_ne!(
        app.screen,
        Screen::BackupFlow,
        "no stick is read with a key held"
    );
    assert!(!app.create().unwrap().uses_existing_vault());
}

/// The wallet made on Keys with a vault already on a stick, taken out of
/// the list: Keys' Continue goes on and makes it again, the vault choice
/// and its flow standing as before or both gone.
#[test]
fn a_wallet_made_with_a_vault_on_a_stick_and_taken_out_makes_again_on_continue_at_the_same_step() {
    use crate::backup_flow_wallet::{bring_in_existing, new_key, sticks};
    let mut shell = Shell::new();
    sticks(&mut shell, 1);
    let mut app = process(Vec::new(), Vec::new(), Vec::new());
    keep_time(&mut app);
    create_single_key(&mut app);
    bring_in_existing(&mut shell, &mut app);
    new_key(&mut app);
    if app.create().unwrap().built.is_none() {
        app.press(Action::CNext(cstep::KEYS));
    }
    let w = app.create().and_then(|c| c.built).expect("made");
    app.press(Action::RemoveWallet(w));
    assert!(app.create().is_some_and(|c| c.built.is_none()));
    app.press(Action::CNext(cstep::KEYS));
    assert!(
        app.create().is_some_and(|c| c.built.is_some()),
        "Keys' Continue did not make the wallet again"
    );
}

/// The same, with the flow on its Sticks step (as a plan changed later
/// leaves it) before the wallet is taken out: Keys' Continue
/// still makes the wallet again.
#[test]
fn a_wallet_made_with_a_vault_on_a_stick_makes_again_after_its_flow_moved_on() {
    use crate::backup_flow_wallet::{bring_in_existing, new_key, sticks};
    let mut shell = Shell::new();
    sticks(&mut shell, 1);
    let mut app = process(Vec::new(), Vec::new(), Vec::new());
    keep_time(&mut app);
    create_single_key(&mut app);
    bring_in_existing(&mut shell, &mut app);
    new_key(&mut app);
    if app.create().unwrap().built.is_none() {
        app.press(Action::CNext(cstep::KEYS));
    }
    let w = app.create().and_then(|c| c.built).expect("made");
    // The flow, put on its Sticks step as a plan changed later puts it.
    faraday_core::testkit::change_flows(&mut app, |all| {
        all.last_mut().unwrap().step = Step::Sticks;
    });
    app.press(Action::RemoveWallet(w));
    app.press(Action::Nav(Screen::Create));
    app.press(Action::CNext(cstep::KEYS));
    assert!(
        app.create().is_some_and(|c| c.built.is_some()),
        "Keys' Continue did not make the wallet again"
    );
}
