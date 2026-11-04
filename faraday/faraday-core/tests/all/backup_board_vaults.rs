//! Several vaults on the sticks of one place: a second vault joined to a
//! stick is written to that stick only, and both of a stick's vaults open
//! from it.

use crate::backup_board::*;
use crate::common::*;
use faraday_core::backup_flow::steps::FlowAction as F;
use faraday_core::backup_flow::{Step, Target};
use faraday_core::board::{Thing, Zone};
use faraday_core::plan::{self, At, What};
use faraday_core::testkit;
use faraday_core::vault::VaultAction as V;
use faraday_core::{Action, BStage, Screen, bstep};

/// Two vaults on one stick (`docs/STICK-SETUP.md` §6.3): Vault 2 dropped on
/// the stick that carries Vault 1 joins it there, and the write step
/// writes both vault files in one visit.
#[test]
fn two_vaults_on_one_stick_survive_a_save_and_are_both_written_to_it_each_its_own_file() {
    let mut shell = Shell::new();
    sticks(&mut shell, 2);
    let (mut app, _) = three_with_vaults(&mut shell, 2);
    vault_at(&mut app, 0, 0, &[0]);
    vault_at(&mut app, 0, 1, &[1, 2]);
    move_to(
        &mut app,
        Zone::Files(0, 1),
        Thing::Vault(1),
        Zone::Files(0, 0),
    );
    let m = the_map(&app);
    assert_eq!(plan::sticks_at(&m, 0), 1, "one stick");
    assert_eq!(
        held(&m, At::Stick(0, 0)),
        vec![What::VaultStick(0), What::VaultStick(1)]
    );
    // The saved plan reads back with both vaults on the stick.
    let shape = backup_shape(&app);
    let text = app.backup().unwrap().answers.to_text();
    let back = faraday_core::plan::Answers::from_text(&shape, &text).expect("the plan loads");
    let m = plan::map(&testkit::compute(), &shape, &back);
    assert_eq!(plan::stick_vaults(&m, 0, 0), vec![0, 1]);
    app.press(Action::BChecklist);
    assert_eq!(app.backup().unwrap().stage, BStage::Checklist);
    save_vaults(&mut app);
    app.press(Action::BNext(bstep::ENVELOPE));
    shell.flow(&mut app, F::WalletLock);
    let_seed_files_out(&mut app);
    let mut app = restart(&mut shell, &mut app);
    assert_eq!(step(&app), Step::Write);
    let targets: Vec<Target> = app.flow().unwrap().slots.iter().map(|s| s.target).collect();
    assert_eq!(targets, vec![Target::Place(0)], "one stick");
    write_stick(&mut shell, &mut app, "S1");
    assert_eq!(
        step(&app),
        Step::Done,
        "{:?}",
        app.flow().unwrap().live.problem
    );
    let vaults: Vec<String> = names(&shell, "S1")
        .into_iter()
        .filter(|n| n.ends_with(".kdbx"))
        .collect();
    assert_eq!(vaults.len(), 2, "{vaults:?}");
    assert_ne!(vaults[0], vaults[1]);
    // Each vault on the stick opens with its own passphrase.
    let files: Vec<(String, Vec<u8>)> = shell
        .disk("S1")
        .files
        .iter()
        .filter(|(n, _)| n.ends_with(".kdbx"))
        .cloned()
        .collect();
    for phrase in ["first vault phrase", "second vault phrase"] {
        let opens = files
            .iter()
            .filter(|f| {
                let mut fresh = process(vec![(*f).clone()], Vec::new(), Vec::new());
                keep_time(&mut fresh);
                fresh.press(Action::Nav(Screen::Vaults));
                fresh.press(Action::Vault(V::Open(0)));
                unlock(&mut fresh, phrase);
                fresh.vaults().open.len() == 1
            })
            .count();
        assert_eq!(opens, 1, "{phrase}: opens exactly one vault of the stick");
    }
}

/// Two copies of Vault 1 at a place and Vault 2 put on the second stick
/// only: the write step puts Vault 2's file on the second stick and not on
/// the first.
#[test]
fn a_vault_put_on_the_second_of_two_copies_sticks_is_written_to_that_stick_only() {
    let mut shell = Shell::new();
    sticks(&mut shell, 3);
    let (mut app, _) = three_with_vaults(&mut shell, 2);
    vault_at(&mut app, 0, 0, &[0]);
    put(&mut app, Thing::Vault(0), Zone::Card(0));
    vault_at(&mut app, 0, 1, &[1, 2]);
    move_to(
        &mut app,
        Zone::Files(0, 2),
        Thing::Vault(1),
        Zone::Files(0, 1),
    );
    let m = the_map(&app);
    assert_eq!(plan::sticks_at(&m, 0), 2, "two sticks");
    assert_eq!(held(&m, At::Stick(0, 0)), vec![What::VaultStick(0)]);
    assert_eq!(
        held(&m, At::Stick(0, 1)),
        vec![What::VaultStick(0), What::VaultStick(1)]
    );
    app.press(Action::BChecklist);
    save_vaults(&mut app);
    app.press(Action::BNext(bstep::ENVELOPE));
    shell.flow(&mut app, F::WalletLock);
    let_seed_files_out(&mut app);
    let mut app = restart(&mut shell, &mut app);
    assert_eq!(step(&app), Step::Write);
    write_stick(&mut shell, &mut app, "S1");
    write_stick(&mut shell, &mut app, "S2");
    assert_eq!(
        step(&app),
        Step::Done,
        "{:?}",
        app.flow().unwrap().live.problem
    );
    let on = |d: &str| {
        names(&shell, d)
            .iter()
            .filter(|n| n.ends_with(".kdbx"))
            .count()
    };
    assert_eq!(on("S1"), 1, "the first stick holds Vault 1 only");
    assert_eq!(on("S2"), 2, "the second holds both");
}
