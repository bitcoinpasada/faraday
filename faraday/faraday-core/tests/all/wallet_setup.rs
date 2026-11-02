//! Make a new wallet's setup of its vaults, before the keys
//! (`docs/BACKUP-FLOW.md` §4.2, §4.3, §10): one press takes a stick for
//! its vault, Create a vault opens by itself once the last stick is
//! bound, and Guided says why the sticks come first.

use crate::backup_flow_wallet::{step, sticks};
use crate::common::*;
use faraday_core::backup_flow::Step;
use faraday_core::backup_flow::steps::FlowAction as F;
use faraday_core::{Action, Faraday, Screen, Sheet, cstep};

/// A process on the stick, its clock kept, on Make a new wallet's Bring
/// in the sticks for one vault of a single key.
fn on_the_sticks() -> Faraday {
    let mut app = process(Vec::new(), Vec::new(), Vec::new());
    keep_time(&mut app);
    create_single_key(&mut app);
    app.press(Action::CVaults(true));
    app.press(Action::CNext(cstep::WHERE));
    app.press(Action::CVaultN(1));
    app.press(Action::CNext(cstep::HOW_MANY));
    assert_eq!(app.screen, Screen::BackupFlow);
    assert_eq!(step(&app), Step::Sticks);
    app
}

/// With the boot stick in, as Faraday Demo has it: one press, Use
/// Boot stick, takes it for Vault 1; the Pull sheet then asks for it out
/// to create the vault, then the key; once it is out, Create a vault is
/// open, with no Create Vault 1 pressed.
#[test]
fn use_boot_stick_takes_it_for_vault_1_and_its_pull_opens_create_a_vault() {
    let mut shell = Shell::new();
    let mut boot = Disk::empty("BOOT", "FARADAY");
    boot.boot = true;
    boot.serial = Some(900);
    shell.add(boot);
    let mut app = on_the_sticks();
    shell.plug(&mut app, "BOOT");
    expect_part(&mut app, "Use Boot stick, read at");
    let k = app.sticks().iter().position(|s| s.boot).unwrap();
    shell.flow(&mut app, F::UseBoot(k));
    assert!(app.flow().unwrap().slots[0].bound.is_some(), "one press");
    assert_eq!(step(&app), Step::Vaults);
    assert_eq!(app.sheet, Some(Sheet::Pull));
    expect_wrapped(
        &mut app,
        "Pull the stick to create the vault, then your key",
    );
    shell.pull(&mut app, "BOOT");
    assert_eq!(app.sheet, None);
    assert_eq!(app.screen, Screen::BackupFlow);
    assert_eq!(
        app.flow().unwrap().live.creating,
        Some(0),
        "Create a vault is open for Vault 1"
    );
}

/// An inserted stick confirmed with its one tap and pulled: Create a
/// vault opens for the new vault, with no Create Vault 1 pressed.
#[test]
fn the_last_stick_pulled_opens_create_a_vault_for_a_new_vault() {
    let mut shell = Shell::new();
    shell.add(Disk::empty("S1", "KEYSTICK"));
    let mut app = on_the_sticks();
    shell.plug(&mut app, "S1");
    shell.flow(&mut app, F::Tap);
    shell.pull(&mut app, "S1");
    assert_eq!(step(&app), Step::Vaults);
    assert_eq!(app.sheet, None);
    assert_eq!(app.flow().unwrap().live.creating, Some(0));
}

/// Guided on, Bring in the sticks says why the sticks are read before the
/// keys; Steps only, it does not.
#[test]
fn guided_bring_in_the_sticks_says_why_the_sticks_come_first() {
    let mut app = on_the_sticks();
    app.press(Action::Guided(true));
    let why = |app: &mut Faraday| {
        texts(app)
            .iter()
            .any(|t| t.starts_with("Each vault's stick is read here"))
    };
    assert!(why(&mut app), "{:?}", texts(&mut app));
    app.press(Action::Guided(false));
    assert!(!why(&mut app));
}

/// Make a new wallet on the Sticks and vaults card, Use vaults answered
/// Yes, for a single key.
fn on_the_card() -> Faraday {
    let mut app = process(Vec::new(), Vec::new(), Vec::new());
    keep_time(&mut app);
    create_single_key(&mut app);
    app.press(Action::CVaults(true));
    app.press(Action::CNext(cstep::WHERE));
    app
}

/// Three vaults on two sticks: Vault 1 on Stick 2 and Vaults 2 and 3 on
/// Stick 3, which the card says they share; Continue is live and the
/// setup asks for those two sticks.
#[test]
fn three_vaults_on_two_sticks_share_the_last_and_continue_is_live() {
    let mut app = on_the_card();
    app.press(Action::CVaultN(3));
    app.press(Action::CSticks(2));
    expect_wrapped(
        &mut app,
        "Vault 2 and Vault 3 share Stick 3: losing it loses both",
    );
    app.press(Action::CNext(cstep::HOW_MANY));
    assert_eq!(app.screen, Screen::BackupFlow);
    assert_eq!(app.flow().unwrap().plan.places, vec![vec![0], vec![1, 2]]);
    assert_eq!(app.flow().unwrap().slots.len(), 2);
}

/// A vault taken off every stick holds Continue, and the card names it.
#[test]
fn a_vault_on_no_stick_holds_continue_and_is_named() {
    use faraday_core::board::{BoardPress as BP, Thing, Zone};
    let mut app = on_the_card();
    app.press(Action::CVaultN(2));
    app.press(Action::Board(BP::Off(Zone::Card(1), Thing::Vault(1))));
    expect_wrapped(&mut app, "Vault 2 is on no stick");
    app.press(Action::CNext(cstep::HOW_MANY));
    assert_eq!(app.screen, Screen::Create, "Continue is not live");
}

/// A stick read for the setup is not kept when the card then changes what
/// the sticks hold: it is asked for again, for the vaults it now carries.
#[test]
fn changing_the_card_after_a_stick_is_read_asks_for_the_stick_again() {
    let mut shell = Shell::new();
    sticks(&mut shell, 2);
    let mut app = on_the_card();
    app.press(Action::CVaultN(2));
    app.press(Action::CNext(cstep::HOW_MANY));
    expect(&mut app, "Insert Stick 2: Vault 1");
    shell.plug(&mut app, "S1");
    shell.flow(&mut app, F::Tap);
    shell.pull(&mut app, "S1");
    expect(&mut app, "Insert Stick 3: Vault 2");
    app.press(Action::Back);
    assert_eq!(app.screen, Screen::Create);
    app.press(Action::CSticks(1));
    app.press(Action::CNext(cstep::HOW_MANY));
    expect(&mut app, "Insert Stick 2: Vault 1, Vault 2");
    assert!(app.flow().unwrap().slots.iter().all(|s| s.bound.is_none()));
}

/// On the Pi the Sticks and vaults card says SD card where the PC says
/// stick: the count's label, the segment, the stick boxes and a check.
#[test]
fn the_card_names_an_sd_card_on_the_pi() {
    use faraday_core::Medium;
    use faraday_core::board::{BoardPress as BP, Thing, Zone};
    let mut app = process(Vec::new(), Vec::new(), Vec::new());
    app.medium = Medium::SdCard;
    keep_time(&mut app);
    create_single_key(&mut app);
    app.press(Action::CVaults(true));
    app.press(Action::CNext(cstep::WHERE));
    app.press(Action::CVaultN(2));
    app.press(Action::Board(BP::Off(Zone::Card(1), Thing::Vault(1))));
    expect_wrapped(&mut app, "SD cards and vaults");
    expect_wrapped(&mut app, "Already on an SD card");
    expect_wrapped(&mut app, "SD card 2");
    expect_wrapped(&mut app, "Vault 2 is on no SD card");
    expect_wrapped(&mut app, "SD card 3 holds no vault");
}
