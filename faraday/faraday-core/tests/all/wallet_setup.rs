//! Make a new wallet's setup of its vaults, before the keys
//! (`docs/BACKUP-FLOW.md` §4.2, §4.3, §10): one press takes a stick for
//! its vault, Create a vault opens by itself once the last stick is
//! bound, and Guided says why the sticks come first.

use crate::backup_flow_wallet::{step, sticks};
use crate::common::*;
use faraday_core::backup_flow::Step;
use faraday_core::backup_flow::steps::FlowAction as F;
use faraday_core::board::{BoardPress as BP, Thing, Zone};
use faraday_core::create::NewKind;
use faraday_core::plan::{self, At, What};
use faraday_core::testkit;
use faraday_core::{Action, Faraday, Screen, Sheet, cstep, qstep};
use faraday_shell_api::{App, Event, Key};

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
    expect_part(&mut app, "Use the boot stick");
    shell.flow(&mut app, F::UseBootAt(0));
    assert!(app.flow().unwrap().slots[0].bound.is_some(), "one press");
    assert_eq!(step(&app), Step::Sticks, "Continue goes on");
    shell.flow(&mut app, F::SticksOn);
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

/// Make a new wallet with three vaults on three sticks, at Bring in the
/// sticks.
fn on_three_sticks() -> Faraday {
    let mut app = on_the_card();
    app.press(Action::CVaultN(3));
    app.press(Action::CSticks(3));
    app.press(Action::CNext(cstep::HOW_MANY));
    assert_eq!(step(&app), Step::Sticks);
    app
}

/// A stick put in goes to the figure the person pressed last, else to the
/// first one waiting.
#[test]
fn a_stick_put_in_goes_to_the_figure_pressed_last_else_the_first_waiting() {
    let mut shell = Shell::new();
    shell.add(Disk::empty("S1", "KEYSTICK"));
    let mut app = on_three_sticks();
    shell.flow(&mut app, F::Want(1));
    shell.plug(&mut app, "S1");
    shell.flow(&mut app, F::Tap);
    let f = app.flow().unwrap();
    assert!(f.slots[1].bound.is_some() && f.slots[0].bound.is_none());

    let mut app = on_three_sticks();
    shell.plug(&mut app, "S1");
    shell.flow(&mut app, F::Tap);
    let f = app.flow().unwrap();
    assert!(f.slots[0].bound.is_some() && f.slots[1].bound.is_none());
}

/// The boot stick chosen for the first stick: it is read from where it
/// is, the figures read Boot stick, Stick 2 and Stick 3, and Continue
/// waits for the other two.
#[test]
fn the_boot_stick_chosen_for_the_first_stick_numbers_the_others_from_2() {
    let mut shell = Shell::new();
    let mut boot = Disk::empty("BOOT", "FARADAY");
    boot.boot = true;
    shell.add(boot);
    let mut app = on_three_sticks();
    shell.plug(&mut app, "BOOT");
    shell.flow(&mut app, F::UseBootAt(0));
    assert!(app.flow().unwrap().slots[0].bound.as_ref().unwrap().boot);
    assert_eq!(step(&app), Step::Sticks, "two sticks to come");
    for name in ["Boot stick", "Stick 2", "Stick 3"] {
        expect_part(&mut app, name);
    }
    assert!(!texts(&mut app).iter().any(|t| t == "Stick 4"));
}

/// A stick that is not FAT32 shows its problem on its figure and is not
/// bound.
#[test]
fn a_stick_that_is_not_fat32_shows_its_problem_and_is_not_bound() {
    let mut shell = Shell::new();
    let mut bad = Disk::empty("S1", "KEYSTICK");
    bad.not_fat = true;
    shell.add(bad);
    let mut app = on_three_sticks();
    shell.plug(&mut app, "S1");
    expect_wrapped(
        &mut app,
        "Not a FAT32 stick. Format it as FAT32, or use another stick",
    );
    assert!(app.flow().unwrap().slots.iter().all(|s| s.bound.is_none()));
}

/// A key typed in as test seed `n`.
fn typed_key(app: &mut Faraday, n: usize) -> [u8; 4] {
    app.press(Action::Entry(None));
    for c in testkit::test_words(testkit::TEST_SEEDS[n].0).chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    app.press(Action::EntryAdd);
    app.session.keys.last().unwrap().fingerprint.0
}

/// Make a new wallet (a 2-of-3 if `multi`) with `vaults` new vaults on
/// `sticks_n` sticks, the sticks brought in, the vaults and the keys made:
/// Back up's plan on the board.
fn plan_after_setup(multi: bool, vaults: usize, sticks_n: usize) -> Faraday {
    let mut shell = Shell::new();
    sticks(&mut shell, sticks_n);
    let mut app = on_the_card();
    if multi {
        let k = NewKind::ALL
            .iter()
            .position(|k| *k == NewKind::Multi)
            .unwrap() as u8;
        app.press(Action::CKind(k));
    }
    app.press(Action::CVaultN(vaults as u16));
    app.press(Action::CSticks(sticks_n as u16));
    app.press(Action::CNext(cstep::HOW_MANY));
    assert_eq!(step(&app), Step::Sticks);
    for k in 1..=sticks_n {
        let d = format!("S{k}");
        shell.plug(&mut app, &d);
        shell.flow(&mut app, F::Tap);
        shell.pull(&mut app, &d);
    }
    for v in 0..vaults {
        create_vault(&mut shell, &mut app, v, &format!("vault phrase {v}"));
    }
    let keys = if multi { 3 } else { 1 };
    let fps: Vec<[u8; 4]> = (0..keys).map(|n| typed_key(&mut app, n)).collect();
    app.press(Action::Nav(Screen::Create));
    for (i, fp) in fps.iter().enumerate() {
        app.press(Action::CSlotHere(i as u8, *fp));
    }
    app.press(Action::CNext(cstep::KEYS));
    app.press(Action::CNext(cstep::CHECK));
    app.press(Action::CBackup);
    let _ = app.frame();
    app
}

/// Whether the board's Continue is offered, anywhere down the page.
fn continue_is_live(app: &mut Faraday) -> bool {
    for _ in 0..30 {
        let _ = app.frame();
        if app.offers(Action::BQNext(qstep::BOARD)) {
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

/// What place `p` keeps on paper, by key.
fn words_at(app: &Faraday, p: usize) -> Vec<usize> {
    backup_held(app, At::Place(p))
        .into_iter()
        .filter_map(|w| match w {
            What::Words(i) => Some(i),
            _ => None,
        })
        .collect()
}

/// A 2-of-3 set up with three vaults on three sticks opens on the board
/// with three places, each keeping one key's paper and the stick of that
/// key's vault, and Continue live with nothing moved; after Customize
/// backup, a stick moved to another place and a seed added to a vault are
/// kept in the plan.
#[test]
fn three_vaults_on_three_sticks_open_the_board_with_each_stick_at_its_keys_place() {
    let mut app = plan_after_setup(true, 3, 3);
    assert_eq!(board_places(&app), 3);
    for p in 0..3 {
        assert_eq!(words_at(&app, p), vec![p], "Place {} paper", p + 1);
        assert!(vault_stick(&app, p, p).is_some(), "Place {} stick", p + 1);
        assert!(vault_stick(&app, p, (p + 1) % 3).is_none());
    }
    assert!(continue_is_live(&mut app), "Continue is live");
    assert!(app.offers(Action::Board(BP::Customize)));
    assert!(!app.offers(Action::Board(BP::AddPlace)), "read-only");
    // Customize backup: everything is the person's to change.
    app.press(Action::Board(BP::Customize));
    let s = vault_stick(&app, 0, 0).unwrap();
    move_to(&mut app, Zone::Files(0, s), Thing::Vault(0), Zone::Card(1));
    assert!(
        vault_stick(&app, 1, 0).is_some(),
        "Stick 2 moved to Place 2"
    );
    assert!(vault_stick(&app, 0, 0).is_none());
    let s = vault_stick(&app, 2, 2).unwrap();
    put(&mut app, Thing::Seed(0), Zone::Vault(2, s, 2));
    assert!(backup_held(&app, At::Vault(2)).contains(&What::Seed(0)));
    let _ = app.frame();
    assert!(vault_stick(&app, 1, 0).is_some());
}

/// A single key with two vaults on two sticks: the first stick at the
/// key's place, the second, holding a copy, at a place added after.
#[test]
fn a_single_keys_second_vault_goes_to_an_added_place() {
    let mut app = plan_after_setup(false, 2, 2);
    assert_eq!(board_places(&app), 2);
    assert!(vault_stick(&app, 0, 0).is_some());
    assert!(vault_stick(&app, 1, 1).is_some());
    assert_eq!(words_at(&app, 0), vec![0]);
    assert!(continue_is_live(&mut app), "Continue is live");
}

/// Three vaults on two sticks: the two vaults of the second stick stay on
/// it.
#[test]
fn vaults_on_one_stick_stay_on_it() {
    let app = plan_after_setup(true, 3, 2);
    let m = backup_map(&app);
    assert_eq!(plan::stick_vaults(&m, 0, 0), vec![0]);
    assert_eq!(plan::stick_vaults(&m, 1, 0), vec![1, 2]);
    assert_eq!(plan::sticks_at(&m, 1), 1);
}

/// A multisig whose setup put two keys' vaults on one stick: the board's
/// check says two keys are at one place.
#[test]
fn two_keys_vaults_on_one_stick_are_said_to_be_at_one_place() {
    let mut app = plan_after_setup(true, 3, 1);
    let m = backup_map(&app);
    assert_eq!(plan::stick_vaults(&m, 0, 0), vec![0, 1, 2]);
    let t = texts(&mut app);
    assert!(
        t.iter()
            .any(|s| s == "Can spend alone with the vault's passphrase"),
        "{t:?}"
    );
}
