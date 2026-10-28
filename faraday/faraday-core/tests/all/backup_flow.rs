//! The shared backup flow for GPG keys and Secure Boot keys
//! (`docs/BACKUP-FLOW.md` §13): where a key will be kept, each stick
//! brought in with nothing secret open, the vaults unlocked or made inside
//! the flow, the key made into every vault, one lock, and each stick
//! written by place.

use crate::common::*;
use faraday_bip::bitcoin::hashes::{Hash, sha256};
use faraday_core::backup_flow::steps::FlowAction as F;
use faraday_core::backup_flow::{Kind, Problem, Step, Target};
use faraday_core::vault::VaultAction as V;
use faraday_core::{Action, Faraday, Screen, Sheet};
use faraday_shell_api::App;
use faraday_vault::records::kind;

const PHRASE: &str = "first vault phrase";
const PHRASE_2: &str = "second vault phrase";

fn step(app: &Faraday) -> Step {
    app.flow().expect("a flow").step
}

fn sha(b: &[u8]) -> [u8; 32] {
    sha256::Hash::hash(b).to_byte_array()
}

/// Where it will be kept, left as it is, then on.
fn through_where(shell: &mut Shell, app: &mut Faraday) {
    shell.flow(app, F::Continue);
    assert_eq!(step(app), Step::Sticks);
}

/// Insert the stick, tap or hold, pull it.
fn bring_in(shell: &mut Shell, app: &mut Faraday, disk: &str, hold: bool) {
    shell.plug(app, disk);
    shell.flow(app, if hold { F::Hold } else { F::Tap });
    shell.pull(app, disk);
}

fn empties(shell: &mut Shell, n: usize) {
    for k in 1..=n {
        shell.add(Disk::empty(&format!("S{k}"), "KEYSTICK"));
    }
}

/// A fresh machine with the clock kept, the flow of `kind` open on Where.
fn started(kind: Kind) -> Faraday {
    let mut app = process(Vec::new(), Vec::new(), Vec::new());
    keep_time(&mut app);
    start(&mut app, kind);
    assert_eq!(app.screen, Screen::BackupFlow);
    assert_eq!(step(&app), Step::Where);
    app
}

/// A key made through the flow from a clean machine: two empty sticks of
/// one label and serial, one new vault.
fn to_make(shell: &mut Shell, kind: Kind) -> Faraday {
    empties(shell, 2);
    let mut app = started(kind);
    through_where(shell, &mut app);
    bring_in(shell, &mut app, "S1", false);
    bring_in(shell, &mut app, "S2", true);
    assert_eq!(step(&app), Step::Vaults);
    create_vault(shell, &mut app, 0, PHRASE);
    assert_eq!(step(&app), Step::Make, "{:?}", unlock_error(&app));
    app
}

const GPG_FIELDS: [&str; 2] = ["Release", "release@example.org"];

/// A GPG key made and locked: the machine restarted, on step 6.
fn gpg_locked(shell: &mut Shell) -> Faraday {
    let mut app = to_make(shell, Kind::Gpg);
    make_key(shell, &mut app, &GPG_FIELDS);
    assert!(app.flow().unwrap().made, "{:?}", app.toast_text());
    shell.flow(&mut app, F::MadeContinue);
    assert_eq!(step(&app), Step::Lock);
    let app = shell.lock(&mut app);
    assert_eq!(step(&app), Step::Write);
    app
}

/// Write each stick of `order` (the hold, where one is needed, pressed).
fn write_all(shell: &mut Shell, app: &mut Faraday, order: &[(&str, bool)]) {
    for (disk, hold) in order {
        shell.plug(app, disk);
        if *hold {
            shell.flow(app, F::Hold);
        }
        shell.pull(app, disk);
    }
}

/// Step 3's card, once done, counts its vault in the singular.
#[test]
fn one_vault_is_counted_as_one_vault() {
    let mut shell = Shell::new();
    let mut app = to_make(&mut shell, Kind::Gpg);
    let texts = app.drawn_texts();
    assert!(texts.iter().any(|t| t == "1 vault"), "{texts:?}");
    assert!(!texts.iter().any(|t| t == "1 vaults"), "{texts:?}");
}

#[test]
fn every_make_starts_on_where_it_will_be_kept() {
    for kind in [Kind::Gpg, Kind::SecureBoot] {
        let mut app = started(kind);
        assert!(offered(&mut app, Action::Flow(F::Continue)));
        assert!(!offered(&mut app, Action::Flow(F::Make)), "{kind:?}");
        assert!(app.vaults().open.is_empty());
    }
}

#[test]
fn make_the_key_is_offered_only_once_the_sticks_are_read_and_the_vaults_open() {
    let mut shell = Shell::new();
    empties(&mut shell, 2);
    let mut app = started(Kind::Gpg);
    through_where(&mut shell, &mut app);
    // Nothing is made before the sticks are read.
    assert!(!offered(&mut app, Action::Flow(F::Make)));
    bring_in(&mut shell, &mut app, "S1", false);
    assert!(!offered(&mut app, Action::Flow(F::Make)));
    bring_in(&mut shell, &mut app, "S2", true);
    // Nor before the vault is made and open.
    assert_eq!(step(&app), Step::Vaults);
    assert!(!offered(&mut app, Action::Flow(F::Make)));
    assert!(app.vaults().open.is_empty());
    create_vault(&mut shell, &mut app, 0, PHRASE);
    assert!(offered(&mut app, Action::Flow(F::Make)));
}

#[test]
fn with_a_vault_open_step_two_locks_first() {
    let mut shell = Shell::new();
    empties(&mut shell, 2);
    let file = vault("Main", &[PHRASE], 0x31);
    let mut app = process(vec![("main.ofv".into(), file)], Vec::new(), Vec::new());
    keep_time(&mut app);
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Open(0)));
    unlock(&mut app, PHRASE);
    assert_eq!(app.vaults().open.len(), 1);
    app.press(Action::Vault(V::Category(4)));
    app.press(Action::Vault(V::Add));
    assert_eq!(app.screen, Screen::BackupFlow);
    through_where(&mut shell, &mut app);
    // A stick is not read by a process that holds a vault.
    shell.plug(&mut app, "S1");
    assert!(app.flow().unwrap().live.reading.is_none());
    assert!(offered(&mut app, Action::Flow(F::LockBring)));
    expect(&mut app, "Lock and bring in the sticks");
    shell.pull(&mut app, "S1");
    shell.flow(&mut app, F::LockBring);
    let mut app = shell.restart();
    assert_eq!(app.screen, Screen::BackupFlow);
    assert_eq!(step(&app), Step::Sticks);
    shell.plug(&mut app, "S1");
    assert!(app.flow().unwrap().live.reading.is_some());
}

#[test]
fn an_empty_stick_is_bound_to_the_new_vault_with_no_question() {
    let mut shell = Shell::new();
    empties(&mut shell, 2);
    let mut app = started(Kind::Gpg);
    through_where(&mut shell, &mut app);
    shell.plug(&mut app, "S1");
    expect(&mut app, "This is the stick for Vault 1 (Place 1)");
    assert!(!has_part(&mut app, "Which vault"));
    assert!(!has(&mut app, "A new vault beside them"));
    shell.flow(&mut app, F::Tap);
    let b = app.flow().unwrap().slots[0].bound.clone().expect("bound");
    assert_eq!(b.label, "KEYSTICK");
    assert!(b.vaults.is_empty());
    // Nothing was copied in.
    assert!(app.inbox().is_empty());
}

#[test]
fn a_stick_with_vaults_asks_which_and_copies_in_only_that_one() {
    let mut shell = Shell::new();
    let mut d = Disk::empty("S1", "WORKSTICK");
    d.serial = Some(1);
    let main = vault("Main", &[PHRASE], 0x41);
    let other = vault("Other", &[PHRASE_2], 0x42);
    d.files = vec![
        ("main.ofv".into(), main.clone()),
        ("main-2.ofv".into(), other.clone()),
    ];
    shell.add(d);
    let mut d2 = Disk::empty("S2", "SPARE");
    d2.serial = Some(2);
    shell.add(d2);
    let mut app = started(Kind::Gpg);
    // The plan gives an existing vault.
    shell.flow(&mut app, F::VaultExisting(0));
    through_where(&mut shell, &mut app);
    shell.plug(&mut app, "S1");
    expect(&mut app, "Which vault is Vault 1?");
    assert!(!has(&mut app, "A new vault beside them"));
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
    expect(&mut app, "This is the stick for Vault 1 (Place 1)");
    shell.flow(&mut app, F::Tap);
    shell.pull(&mut app, "S1");
    // Only the chosen file is in Files.
    let vaults: Vec<&str> = app
        .inbox()
        .iter()
        .filter(|i| i.name.ends_with(".ofv"))
        .map(|i| i.name.as_str())
        .collect();
    assert_eq!(vaults, ["main.ofv"]);
    bring_in(&mut shell, &mut app, "S2", false);
    assert_eq!(step(&app), Step::Vaults);
    unlock(&mut app, PHRASE);
    assert_eq!(step(&app), Step::Make);
    make_key(&mut shell, &mut app, &GPG_FIELDS);
    shell.flow(&mut app, F::MadeContinue);
    let mut app = shell.lock(&mut app);
    write_all(&mut shell, &mut app, &[("S1", false), ("S2", true)]);
    // The other vault on the stick is byte for byte what it was.
    assert_eq!(shell.disk("S1").file("main-2.ofv"), Some(&other));
    assert_ne!(shell.disk("S1").file("main.ofv"), Some(&main), "replaced");
    assert_eq!(
        shell.disk("S1").files.len(),
        3,
        "two vaults and the public key"
    );
    assert_eq!(
        shell.disk("S1").file("main.ofv"),
        shell.disk("S2").file("main.ofv")
    );
}

#[test]
fn at_a_place_given_a_new_vault_a_stick_with_vaults_offers_a_new_one_beside_them() {
    let mut shell = Shell::new();
    let mut d = Disk::empty("S1", "WORKSTICK");
    let main = vault("Main", &[PHRASE], 0x43);
    d.files = vec![("vault-1.ofv".into(), main.clone())];
    shell.add(d);
    shell.add(Disk::empty("S2", "SPARE"));
    shell.disks[1].serial = Some(9);
    let mut app = started(Kind::Gpg);
    through_where(&mut shell, &mut app);
    shell.plug(&mut app, "S1");
    expect(&mut app, "Which vault is Vault 1?");
    expect(&mut app, "A new vault beside them");
    shell.flow(&mut app, F::PickBeside(0));
    shell.flow(&mut app, F::Tap);
    shell.pull(&mut app, "S1");
    bring_in(&mut shell, &mut app, "S2", false);
    create_vault(&mut shell, &mut app, 0, PHRASE_2);
    make_key(&mut shell, &mut app, &GPG_FIELDS);
    shell.flow(&mut app, F::MadeContinue);
    let mut app = shell.lock(&mut app);
    write_all(&mut shell, &mut app, &[("S1", false), ("S2", true)]);
    // Written beside: the vault already there is untouched.
    assert_eq!(
        shell.disk("S1").files.len(),
        3,
        "two vaults and the public key"
    );
    assert_eq!(shell.disk("S1").file("vault-1.ofv"), Some(&main));
}

#[test]
fn a_stick_bound_to_place_1_inserted_for_place_2_is_refused() {
    let mut shell = Shell::new();
    let mut d = Disk::empty("S1", "WORKSTICK");
    d.files = vec![("notes.txt".into(), b"a file to tell it by".to_vec())];
    shell.add(d);
    shell.disks[0].serial = Some(5);
    let mut app = started(Kind::Gpg);
    through_where(&mut shell, &mut app);
    bring_in(&mut shell, &mut app, "S1", false);
    shell.plug(&mut app, "S1");
    assert!(
        has(&mut app, "This is the stick for Vault 1 (Place 1)"),
        "{:?}",
        texts(&mut app)
    );
    assert!(matches!(
        app.flow().unwrap().live.reading.as_ref().unwrap().problem,
        Some(Problem::BoundTo(Target::Place(0)))
    ));
    assert!(!offered(&mut app, Action::Flow(F::Tap)));
    assert!(app.flow().unwrap().slots[1].bound.is_none());
}

#[test]
fn a_second_empty_stick_of_the_same_label_and_serial_needs_the_hold() {
    let mut shell = Shell::new();
    empties(&mut shell, 2);
    let mut app = started(Kind::Gpg);
    through_where(&mut shell, &mut app);
    bring_in(&mut shell, &mut app, "S1", false);
    shell.plug(&mut app, "S2");
    // A tap does not bind it; only the hold does.
    expect(&mut app, "Hold: this is the stick for Vault 1 (Place 2)");
    shell.flow(&mut app, F::Tap);
    assert!(app.flow().unwrap().slots[1].bound.is_none());
    shell.flow(&mut app, F::Hold);
    assert!(app.flow().unwrap().slots[1].bound.is_some());
}

#[test]
fn an_unusable_stick_says_why_and_binds_nothing() {
    type Spoil = fn(&mut Disk);
    let cases: [(&str, Spoil, &str); 4] = [
        (
            "not FAT",
            |d| d.not_fat = true,
            "Not a FAT32 stick. Format it as FAT32, or use another stick",
        ),
        (
            "read-only",
            |d| d.read_only = true,
            "Cannot be written. Use another stick",
        ),
        ("full", |d| d.free = 1000, "Not enough room"),
        (
            "boot",
            |d| d.label = "FARADAYBOOT".into(),
            "This is a boot partition. Use another stick",
        ),
    ];
    for (name, make, want) in cases {
        let mut shell = Shell::new();
        let mut d = Disk::empty("S1", "KEYSTICK");
        make(&mut d);
        shell.add(d);
        let mut app = started(Kind::Gpg);
        through_where(&mut shell, &mut app);
        shell.plug(&mut app, "S1");
        assert!(has_part(&mut app, want), "{name}: {:?}", texts(&mut app));
        assert!(!offered(&mut app, Action::Flow(F::Tap)), "{name}");
        assert!(app.flow().unwrap().slots[0].bound.is_none(), "{name}");
    }
}

#[test]
fn new_vaults_have_their_own_passphrases_and_the_key_is_in_every_one() {
    let mut shell = Shell::new();
    shell.add(Disk::empty("S1", "ONE"));
    shell.add(Disk::empty("S2", "TWO"));
    shell.disks[1].serial = Some(2);
    let mut app = started(Kind::Gpg);
    // Place 2 keeps a vault of its own.
    shell.flow(&mut app, F::PlaceVault(1, 1));
    assert_eq!(app.flow().unwrap().plan.vaults.len(), 2);
    through_where(&mut shell, &mut app);
    bring_in(&mut shell, &mut app, "S1", false);
    bring_in(&mut shell, &mut app, "S2", false);
    // Every stick is read before any vault exists.
    assert_eq!(step(&app), Step::Vaults);
    assert!(app.vault_files().is_empty());
    create_vault(&mut shell, &mut app, 0, PHRASE);
    assert_eq!(app.vaults().open.len(), 1);
    assert_eq!(step(&app), Step::Vaults);
    assert_eq!(app.screen, Screen::BackupFlow);
    create_vault(&mut shell, &mut app, 1, PHRASE_2);
    assert_eq!(app.vaults().open.len(), 2);
    assert_eq!(step(&app), Step::Make);
    make_key(&mut shell, &mut app, &GPG_FIELDS);
    let ids: Vec<Vec<u8>> = app
        .vaults()
        .open
        .iter()
        .map(|o| {
            let keys: Vec<_> = app
                .vaults()
                .store()
                .contents_for_tests(o.id)
                .of(kind::GPG)
                .collect();
            assert_eq!(keys.len(), 1);
            keys[0].1.field(1).unwrap_or(&[]).to_vec()
        })
        .collect();
    assert_eq!(ids[0], ids[1], "one key, made once");
    assert_eq!(app.flow().unwrap().made_in, [0, 1]);
    // Two sticks, two vaults: each its own file.
    shell.flow(&mut app, F::MadeContinue);
    let mut app = shell.lock(&mut app);
    write_all(&mut shell, &mut app, &[("S1", false), ("S2", true)]);
    assert_ne!(shell.disk("S1").files[0].1, shell.disk("S2").files[0].1);
}

#[test]
fn i_do_not_have_its_passphrase_offers_a_new_vault_beside_it() {
    let mut shell = Shell::new();
    let mut d = Disk::empty("S1", "WORKSTICK");
    d.files = vec![("main.ofv".into(), vault("Main", &[PHRASE], 0x44))];
    shell.add(d);
    shell.add(Disk::empty("S2", "SPARE"));
    shell.disks[1].serial = Some(7);
    let mut app = started(Kind::Gpg);
    shell.flow(&mut app, F::VaultExisting(0));
    through_where(&mut shell, &mut app);
    shell.plug(&mut app, "S1");
    shell.flow(&mut app, F::PickFile(0, 0));
    shell.flow(&mut app, F::Tap);
    shell.pull(&mut app, "S1");
    bring_in(&mut shell, &mut app, "S2", false);
    assert_eq!(step(&app), Step::Vaults);
    shell.flow(&mut app, F::UnlockVault(0));
    expect(&mut app, "Unlock Vault 1");
    expect(&mut app, "I do not have its passphrase");
    shell.flow(&mut app, F::NoPass(0));
    expect(&mut app, "A new vault beside it");
    expect(&mut app, "Use another stick");
    shell.flow(&mut app, F::BesideIt(0));
    let v = &app.flow().unwrap().plan.vaults[0];
    assert!(!v.existing && v.salt.is_none(), "the plan's vault is new");
    expect(&mut app, "Create Vault 1");
}

#[test]
fn example_a_writes_both_sticks_and_the_key_page_reads_every_place_written() {
    let mut shell = Shell::new();
    let mut app = gpg_locked(&mut shell);
    // The first stick, then the second with the hold.
    write_all(&mut shell, &mut app, &[("S1", false), ("S2", true)]);
    assert_eq!(step(&app), Step::Done);
    let names: Vec<String> = shell.disk("S1").files.iter().map(|f| f.0.clone()).collect();
    let a = shell
        .disk("S1")
        .files
        .first()
        .unwrap_or_else(|| panic!("{names:?}"))
        .1
        .clone();
    let b = shell.disk("S2").files.first().expect("S2").1.clone();
    assert_eq!(sha(&a), sha(&b), "the same sealed bytes");
    let (n, m) = app.flow().unwrap().written_of();
    assert_eq!((n, m), (2, 2));
    // Unlock, and the key's page reads every place written.
    shell.flow(&mut app, F::UnlockDone(0));
    unlock(&mut app, PHRASE);
    assert_eq!(app.screen, Screen::VaultContents);
    expect(&mut app, "Written to every place");
    assert!(has(&mut app, "Release") || has_part(&mut app, "Release"));
}

#[test]
fn a_stick_put_back_for_place_2_or_unlike_the_one_read_is_refused_and_nothing_is_written_before_the_hold()
 {
    let mut shell = Shell::new();
    let mut app = gpg_locked(&mut shell);
    shell.add(Disk::empty("S3", "OTHER"));
    shell.disk_mut("S3").serial = Some(77);
    shell.plug(&mut app, "S1");
    shell.pull(&mut app, "S1");
    assert_eq!(shell.vault_writes(), 1);
    // The first stick put back for Place 2.
    shell.plug(&mut app, "S1");
    expect(&mut app, "This stick has the first copy");
    assert!(!offered(&mut app, Action::Flow(F::Hold)));
    assert_eq!(shell.vault_writes(), 1);
    shell.pull(&mut app, "S1");
    // A stick unlike the one read for Place 2.
    shell.plug(&mut app, "S3");
    expect(&mut app, "Not Stick 3, read for Vault 1 (Place 2)");
    assert_eq!(shell.vault_writes(), 1);
    shell.pull(&mut app, "S3");
    // The right stick: nothing is written before the hold.
    shell.plug(&mut app, "S2");
    expect(&mut app, "Hold: this is a second stick");
    assert_eq!(shell.vault_writes(), 1);
    shell.flow(&mut app, F::Hold);
    assert_eq!(shell.vault_writes(), 2);
}

#[test]
fn the_stick_visit_never_shows_and_no_lock_or_write_out_sheet_comes_up() {
    let mut shell = Shell::new();
    let mut app = to_make(&mut shell, Kind::Gpg);
    make_key(&mut shell, &mut app, &GPG_FIELDS);
    shell.flow(&mut app, F::MadeContinue);
    let mut app = shell.lock(&mut app);
    // A stick comes and goes at every step: the screen stays the flow's.
    write_all(&mut shell, &mut app, &[("S1", false), ("S2", true)]);
    assert!(
        shell.screens.iter().all(|s| *s != Screen::Visit),
        "{:?}",
        shell.screens
    );
    assert!(
        shell
            .sheets
            .iter()
            .all(|s| !matches!(s, Sheet::Lock | Sheet::WriteOut)),
        "{:?}",
        shell.sheets
    );
}

#[test]
fn a_stick_arriving_at_the_vaults_card_brings_the_pull_sheet_not_the_visit() {
    let mut shell = Shell::new();
    empties(&mut shell, 3);
    let mut app = started(Kind::Gpg);
    through_where(&mut shell, &mut app);
    bring_in(&mut shell, &mut app, "S1", false);
    bring_in(&mut shell, &mut app, "S2", true);
    assert_eq!(step(&app), Step::Vaults);
    shell.plug(&mut app, "S3");
    assert_eq!(app.screen, Screen::BackupFlow);
    assert_eq!(app.sheet, Some(Sheet::Pull));
    shell.pull(&mut app, "S3");
    assert_eq!(app.sheet, None);
}

#[test]
fn a_lock_resumes_on_the_right_card() {
    // On Where.
    let mut shell = Shell::new();
    let mut app = started(Kind::Gpg);
    let app2 = shell.lock(&mut app);
    assert_eq!(app2.screen, Screen::BackupFlow);
    assert_eq!(step(&app2), Step::Where);

    // On Vaults, with the sticks read.
    let mut shell = Shell::new();
    empties(&mut shell, 2);
    let mut app = started(Kind::Gpg);
    through_where(&mut shell, &mut app);
    bring_in(&mut shell, &mut app, "S1", false);
    bring_in(&mut shell, &mut app, "S2", true);
    assert_eq!(step(&app), Step::Vaults);
    let app2 = shell.lock(&mut app);
    assert_eq!(step(&app2), Step::Vaults);
    assert_eq!(
        app2.flow()
            .unwrap()
            .slots
            .iter()
            .filter(|s| s.bound.is_some())
            .count(),
        2
    );

    // On Make with the key made: the vaults sealed, on to the writes.
    let mut shell = Shell::new();
    let mut app = to_make(&mut shell, Kind::Gpg);
    make_key(&mut shell, &mut app, &GPG_FIELDS);
    let app2 = shell.lock(&mut app);
    assert_eq!(step(&app2), Step::Write);
    assert!(app2.flow().unwrap().made);
}

#[test]
fn the_kept_state_read_back_after_the_first_stick_asks_for_the_next() {
    let mut shell = Shell::new();
    let mut app = gpg_locked(&mut shell);
    write_all(&mut shell, &mut app, &[("S1", false)]);
    assert_eq!(app.flow().unwrap().written_of(), (1, 2));
    // A lock, then the kept state read back.
    let mut app2 = shell.lock(&mut app);
    assert_eq!(app2.screen, Screen::BackupFlow);
    assert_eq!(step(&app2), Step::Write);
    assert_eq!(app2.flow().unwrap().written_of(), (1, 2));
    assert!(has(&mut app2, "Insert Stick 3: Vault 1 (Place 2)"));
    write_all(&mut shell, &mut app2, &[("S2", true)]);
    assert_eq!(step(&app2), Step::Done);
}

#[test]
fn a_power_off_between_writes_leaves_the_copy_and_the_key_signs_at_once_from_it() {
    let mut shell = Shell::new();
    let mut app = gpg_locked(&mut shell);
    write_all(&mut shell, &mut app, &[("S1", false)]);
    // Power-off: nothing of the flow is kept.
    let (inbox, outbox, _) = shell.boxes.clone();
    let mut fresh = process(inbox, outbox, Vec::new());
    keep_time(&mut fresh);
    assert!(fresh.backup_flows().is_empty());
    assert!(fresh.backup_pending().is_none());
    // The stick's vault is in Files, the key in it signs at once.
    let on_stick = shell.disk("S1").files[0].1.clone();
    faraday_core::testkit::inbox_retain(&mut fresh, |i| !i.name.ends_with(".ofv"));
    fresh.press(Action::Nav(Screen::Vaults));
    let _ = on_stick;
    fresh.press(Action::Vault(V::Open(0)));
    unlock(&mut fresh, PHRASE);
    assert_eq!(fresh.vaults().open.len(), 1);
    fresh.press(Action::Nav(Screen::VaultContents));
    fresh.press(Action::Vault(V::Category(4)));
    fresh.press(Action::Vault(V::Item(0)));
    assert!(fresh.backup_row(Kind::Gpg, &[0; 20]).is_none());
    assert!(!has(&mut fresh, "Back up first"));
    // Its actions are under the renew terms and the Backup row: the page
    // scrolled down to them.
    for _ in 0..6 {
        fresh.event(faraday_shell_api::Event::Scroll {
            x: 1000,
            y: 400,
            dy: 120,
        });
    }
    assert!(has(&mut fresh, "Sign a file"));
    fresh.press(Action::Vault(V::GpgSignPick));
    assert!(fresh.vaults().signing, "signing opens");
}

#[test]
fn signing_waits_until_every_place_is_written_then_works_after_the_unlock() {
    let mut shell = Shell::new();
    let mut app = to_make(&mut shell, Kind::Gpg);
    make_key(&mut shell, &mut app, &GPG_FIELDS);
    // The key's page, while it waits.
    app.press(Action::Nav(Screen::VaultContents));
    app.press(Action::Vault(V::Category(4)));
    app.press(Action::Vault(V::Item(0)));
    expect(&mut app, "Back up first");
    expect(&mut app, "Not written");
    assert!(!has(&mut app, "Sign a file"));
    app.press(Action::Vault(V::GpgSignPick));
    assert!(!app.vaults().signing);
    assert_eq!(
        app.screen,
        Screen::BackupFlow,
        "Back up first opens the flow"
    );
    app.press(Action::Vault(V::GpgRenew));
    assert!(offered(&mut app, Action::Flow(F::MadeContinue)));
    shell.flow(&mut app, F::MadeContinue);
    let mut app = shell.lock(&mut app);
    write_all(&mut shell, &mut app, &[("S1", false)]);
    assert!(app.backup_pending().is_some());
    write_all(&mut shell, &mut app, &[("S2", true)]);
    assert!(app.backup_pending().is_none());
    shell.flow(&mut app, F::UnlockDone(0));
    unlock(&mut app, PHRASE);
    assert!(!has(&mut app, "Back up first"));
    expect(&mut app, "Written to every place");
    app.press(Action::Vault(V::GpgSignPick));
    assert!(app.vaults().signing);
}

#[test]
fn secure_boot_keys_wait_too() {
    let mut shell = Shell::new();
    let mut app = to_make(&mut shell, Kind::SecureBoot);
    make_key(&mut shell, &mut app, &["Owner"]);
    assert!(app.flow().unwrap().made, "{:?}", app.toast_text());
    app.press(Action::Nav(Screen::VaultContents));
    app.press(Action::Vault(V::Category(5)));
    app.press(Action::Vault(V::Item(0)));
    expect(&mut app, "Back up first");
    assert!(!has(&mut app, "Enrolment files"));
    let files = app.outbox.len();
    app.press(Action::Vault(V::SbEnrol));
    assert_eq!(app.outbox.len(), files, "nothing signed");
    assert_eq!(app.screen, Screen::BackupFlow);
}

#[test]
fn a_vault_kept_at_four_places_asks_for_four_sticks_the_later_ones_by_another_hold() {
    let mut shell = Shell::new();
    empties(&mut shell, 4);
    let mut app = started(Kind::Gpg);
    shell.flow(&mut app, F::AddPlace);
    shell.flow(&mut app, F::AddPlace);
    assert_eq!(app.flow().unwrap().plan.places.len(), 4);
    through_where(&mut shell, &mut app);
    bring_in(&mut shell, &mut app, "S1", false);
    for k in 2..=4 {
        bring_in(&mut shell, &mut app, &format!("S{k}"), true);
    }
    create_vault(&mut shell, &mut app, 0, PHRASE);
    make_key(&mut shell, &mut app, &GPG_FIELDS);
    shell.flow(&mut app, F::MadeContinue);
    let mut app = shell.lock(&mut app);
    write_all(&mut shell, &mut app, &[("S1", false)]);
    shell.plug(&mut app, "S2");
    expect(&mut app, "Hold: this is a second stick");
    shell.flow(&mut app, F::Hold);
    shell.pull(&mut app, "S2");
    shell.plug(&mut app, "S3");
    expect(&mut app, "Hold: this is another stick");
    shell.flow(&mut app, F::Hold);
    shell.pull(&mut app, "S3");
    shell.plug(&mut app, "S4");
    expect(&mut app, "Hold: this is another stick");
    shell.flow(&mut app, F::Hold);
    shell.pull(&mut app, "S4");
    assert_eq!(step(&app), Step::Done);
    assert_eq!(shell.vault_writes(), 4);
}

#[test]
fn a_key_that_cannot_be_made_again_is_planned_at_two_places_or_more() {
    for kind in [Kind::Gpg, Kind::SecureBoot] {
        let mut app = started(kind);
        let before = app.flow().unwrap().plan.clone();
        assert_eq!(before.stick_places().len(), 2);
        // The control to remove a place is not drawn at two.
        assert!(
            !offered(&mut app, Action::Flow(F::RemovePlace(0))),
            "{kind:?}"
        );
        app.press(Action::Flow(F::RemovePlace(0)));
        assert_eq!(app.flow().unwrap().plan, before);
        // Add one, and removing is offered again.
        app.press(Action::Flow(F::AddPlace));
        assert_eq!(app.flow().unwrap().plan.stick_places().len(), 3);
        assert!(offered(&mut app, Action::Flow(F::RemovePlace(2))));
        app.press(Action::Flow(F::RemovePlace(2)));
        assert_eq!(app.flow().unwrap().plan.stick_places().len(), 2);
    }
}

#[test]
fn a_plan_of_empty_sticks_makes_every_vault_at_step_three_inside_the_flow() {
    let mut shell = Shell::new();
    shell.add(Disk::empty("S1", "ONE"));
    shell.add(Disk::empty("S2", "TWO"));
    shell.disks[1].serial = Some(2);
    let mut app = started(Kind::Gpg);
    shell.flow(&mut app, F::PlaceVault(1, 1));
    through_where(&mut shell, &mut app);
    let from = shell.screens.len();
    bring_in(&mut shell, &mut app, "S1", false);
    bring_in(&mut shell, &mut app, "S2", false);
    assert!(
        app.vault_files().is_empty(),
        "no vault before every stick is read"
    );
    create_vault(&mut shell, &mut app, 0, PHRASE);
    create_vault(&mut shell, &mut app, 1, PHRASE_2);
    assert_eq!(app.vault_files().len(), 2);
    assert_eq!(app.vaults().open.len(), 2);
    assert!(
        shell.screens[from..]
            .iter()
            .all(|s| *s == Screen::BackupFlow),
        "{:?}",
        &shell.screens[from..]
    );
    assert_eq!(app.screen, Screen::BackupFlow);
}

#[test]
fn a_mix_of_existing_and_new_vaults_offers_open_and_create_once_every_stick_is_read() {
    let mut shell = Shell::new();
    let mut d = Disk::empty("S1", "WORKSTICK");
    d.files = vec![("main.ofv".into(), vault("Main", &[PHRASE], 0x45))];
    shell.add(d);
    shell.add(Disk::empty("S2", "SPARE"));
    shell.disks[1].serial = Some(3);
    let mut app = started(Kind::Gpg);
    shell.flow(&mut app, F::PlaceVault(1, 1));
    shell.flow(&mut app, F::VaultExisting(0));
    through_where(&mut shell, &mut app);
    shell.plug(&mut app, "S1");
    shell.flow(&mut app, F::PickFile(0, 0));
    shell.flow(&mut app, F::Tap);
    shell.pull(&mut app, "S1");
    // A vault found on the first stick: nothing is unlocked or offered to
    // open until every stick is read.
    assert_eq!(step(&app), Step::Sticks);
    assert!(!offered(&mut app, Action::Flow(F::UnlockVault(0))));
    assert!(!has(&mut app, "Unlock Vault 1"));
    bring_in(&mut shell, &mut app, "S2", false);
    assert_eq!(step(&app), Step::Vaults);
    // Then the step offers both: Open the vault found, Create the new
    // one; nothing is unlocked or opened by itself.
    expect(&mut app, "Open Vault 1");
    expect(&mut app, "Create Vault 2");
    assert!(offered(&mut app, Action::Flow(F::UnlockVault(0))));
    assert!(offered(&mut app, Action::Flow(F::CreateVault(1))));
    assert!(app.vaults().open.is_empty());
    assert_eq!(app.flow().unwrap().live.unlocking, None);
    assert_eq!(app.flow().unwrap().live.creating, None);
    shell.flow(&mut app, F::UnlockVault(0));
    expect(&mut app, "Unlock Vault 1");
    unlock(&mut app, PHRASE);
    assert_eq!(step(&app), Step::Vaults);
    assert_eq!(app.vaults().open.len(), 1);
    assert!(offered(&mut app, Action::Flow(F::CreateVault(1))));
    create_vault(&mut shell, &mut app, 1, PHRASE_2);
    assert_eq!(step(&app), Step::Make);
    assert_eq!(app.vaults().open.len(), 2);
    assert_eq!(app.screen, Screen::BackupFlow);
}

#[test]
fn a_stick_pulled_while_writing_counts_nothing() {
    let mut shell = Shell::new();
    let mut app = gpg_locked(&mut shell);
    shell.hold_writes = true;
    shell.plug(&mut app, "S1");
    assert!(!shell.stashed.is_empty(), "a write is under way");
    shell.pull(&mut app, "S1");
    assert_eq!(app.flow().unwrap().written_of(), (0, 2));
    assert!(
        has(
            &mut app,
            "Pulled before the copy was checked. Nothing counted"
        ) || app.flow().unwrap().live.problem == Some(Problem::Pulled)
    );
    assert!(shell.writes.is_empty());
}

#[test]
fn with_no_flow_in_the_kept_state_none_is_offered() {
    let app = process(Vec::new(), Vec::new(), Vec::new());
    assert!(app.backup_flows().is_empty());
    assert!(app.flow().is_none());
}

#[test]
fn home_leads_with_the_backup_not_finished_and_the_power_sheet_lists_it() {
    let mut shell = Shell::new();
    let mut app = gpg_locked(&mut shell);
    write_all(&mut shell, &mut app, &[("S1", false)]);
    app.press(Action::Flow(F::Back));
    assert_eq!(app.screen, Screen::Home);
    expect(&mut app, "Continue the backup of Release");
    app.press(Action::PowerAsk);
    expect_part(&mut app, "Backup of Release: written to 1 of 2 places");
    app.press(Action::Cancel);
    // The lead resumes the flow on its step.
    let k = app.backup_pending().map(|(i, _)| i).unwrap();
    app.press(Action::Flow(F::Show(k)));
    assert_eq!(app.screen, Screen::BackupFlow);
    assert_eq!(step(&app), Step::Write);
}

#[test]
fn a_flow_left_hands_a_stick_to_the_visit_with_its_files_in_the_backup() {
    let mut shell = Shell::new();
    let mut app = gpg_locked(&mut shell);
    app.press(Action::Flow(F::Back));
    shell.plug(&mut app, "S1");
    assert_eq!(app.screen, Screen::Visit);
    let sealed = app.backup_flows()[0].sealing[0].name.clone();
    assert!(app.in_the_backup(&sealed));
    expect(&mut app, "In the backup");
}

#[test]
fn adding_a_copy_of_a_planned_vault_adds_one_stick_to_step_six() {
    let mut shell = Shell::new();
    empties(&mut shell, 3);
    shell.disks[2].serial = Some(333);
    let mut app = to_make(&mut shell, Kind::Gpg);
    make_key(&mut shell, &mut app, &GPG_FIELDS);
    shell.flow(&mut app, F::MadeContinue);
    // Change the plan: a third place for Vault 1.
    shell.flow(&mut app, F::ChangePlan);
    shell.flow(&mut app, F::AddPlace);
    shell.flow(&mut app, F::Continue);
    assert!(!app.flow().unwrap().adding, "no extra round");
    assert!(!app.restart_requested());
    let mut app = shell.lock(&mut app);
    assert_eq!(step(&app), Step::Write);
    assert_eq!(app.flow().unwrap().slots.len(), 3);
    write_all(&mut shell, &mut app, &[("S1", false), ("S2", true)]);
    // The third stick was named after step two: checked now, then the tap.
    shell.plug(&mut app, "S3");
    expect(&mut app, "This is the stick for Vault 1 (Place 3)");
    shell.flow(&mut app, F::Tap);
    expect(&mut app, "Hold: this is another stick");
    shell.flow(&mut app, F::Hold);
    shell.pull(&mut app, "S3");
    assert_eq!(step(&app), Step::Done);
    assert_eq!(shell.vault_writes(), 3);
}

#[test]
fn adding_a_new_vault_on_an_empty_stick_after_the_key_is_made_asks_for_no_lock() {
    let mut shell = Shell::new();
    empties(&mut shell, 3);
    shell.disks[2].serial = Some(33);
    let mut app = to_make(&mut shell, Kind::Gpg);
    make_key(&mut shell, &mut app, &GPG_FIELDS);
    shell.flow(&mut app, F::MadeContinue);
    shell.flow(&mut app, F::ChangePlan);
    shell.flow(&mut app, F::AddPlace);
    shell.flow(&mut app, F::PlaceVault(2, 1));
    shell.flow(&mut app, F::Continue);
    // The new vault is made in memory, the key saved into it.
    assert_eq!(step(&app), Step::Vaults);
    assert!(!app.restart_requested());
    create_vault(&mut shell, &mut app, 1, PHRASE_2);
    assert_eq!(step(&app), Step::Make);
    shell.flow(&mut app, F::CopyKey);
    assert_eq!(app.flow().unwrap().made_in, [0, 1]);
    assert!(!app.restart_requested());
    shell.flow(&mut app, F::MadeContinue);
    let mut app = shell.lock(&mut app);
    assert_eq!(step(&app), Step::Write);
    assert_eq!(app.flow().unwrap().slots.len(), 3);
    write_all(&mut shell, &mut app, &[("S1", false), ("S2", true)]);
    shell.plug(&mut app, "S3");
    shell.flow(&mut app, F::Tap);
    shell.pull(&mut app, "S3");
    assert_eq!(step(&app), Step::Done);
    assert!(shell.screens.iter().all(|s| *s != Screen::Visit));
}

#[test]
fn adding_a_stick_with_an_existing_vault_runs_the_extra_round_inside_the_flow() {
    let mut shell = Shell::new();
    empties(&mut shell, 2);
    let mut d = Disk::empty("S3", "WORKSTICK");
    d.serial = Some(44);
    d.files = vec![("main.ofv".into(), vault("Main", &[PHRASE_2], 0x46))];
    shell.add(d);
    let mut app = to_make(&mut shell, Kind::Gpg);
    make_key(&mut shell, &mut app, &GPG_FIELDS);
    shell.flow(&mut app, F::MadeContinue);
    shell.flow(&mut app, F::ChangePlan);
    shell.flow(&mut app, F::AddPlace);
    shell.flow(&mut app, F::PlaceVault(2, 1));
    shell.flow(&mut app, F::VaultExisting(1));
    shell.flow(&mut app, F::Continue);
    assert!(app.flow().unwrap().adding);
    assert_eq!(step(&app), Step::Sticks);
    // Secrets are open: lock first, the key's vault sealed for the stick.
    assert!(offered(&mut app, Action::Flow(F::LockBring)));
    shell.flow(&mut app, F::LockBring);
    // The lock asks first, as every lock with a vault open does.
    assert_eq!(app.sheet, Some(faraday_core::Sheet::LockAsk));
    shell.press(&mut app, Action::Lock);
    let mut app = shell.restart();
    assert_eq!(app.screen, Screen::BackupFlow);
    assert_eq!(step(&app), Step::Sticks);
    shell.plug(&mut app, "S3");
    expect(&mut app, "Which vault is Vault 2?");
    shell.flow(&mut app, F::PickFile(1, 0));
    shell.flow(&mut app, F::Tap);
    shell.pull(&mut app, "S3");
    assert_eq!(step(&app), Step::Vaults);
    // The added vault is opened, then one that holds the key.
    open_vault(&mut shell, &mut app, 1, PHRASE_2);
    open_vault(&mut shell, &mut app, 0, PHRASE);
    assert_eq!(app.vaults().open.len(), 2, "{:?}", unlock_error(&app));
    assert_eq!(step(&app), Step::Make);
    shell.flow(&mut app, F::CopyKey);
    for o in &app.vaults().open {
        assert_eq!(
            app.vaults()
                .store()
                .contents_for_tests(o.id)
                .of(kind::GPG)
                .count(),
            1
        );
    }
    shell.flow(&mut app, F::MadeContinue);
    let mut app = shell.lock(&mut app);
    assert_eq!(step(&app), Step::Write);
    write_all(
        &mut shell,
        &mut app,
        &[("S1", false), ("S2", true), ("S3", false)],
    );
    assert_eq!(step(&app), Step::Done);
    assert!(shell.screens.iter().all(|s| *s != Screen::Visit));
}

#[test]
fn removing_a_place_before_its_write_drops_a_new_vault_holding_only_this_flows_key() {
    let mut shell = Shell::new();
    empties(&mut shell, 3);
    shell.disks[1].serial = Some(2);
    shell.disks[2].serial = Some(3);
    let mut app = started(Kind::Gpg);
    shell.flow(&mut app, F::AddPlace);
    shell.flow(&mut app, F::PlaceVault(1, 1));
    assert_eq!(app.flow().unwrap().plan.vaults.len(), 2);
    shell.flow(&mut app, F::RemovePlace(1));
    let plan = &app.flow().unwrap().plan;
    assert_eq!(
        plan.vaults.len(),
        1,
        "the new vault is dropped with its place"
    );
    assert_eq!(plan.stick_places().len(), 2);
}

#[test]
fn removing_a_written_place_keeps_it_on_the_map_until_destroyed() {
    let mut shell = Shell::new();
    empties(&mut shell, 3);
    let mut app = to_make(&mut shell, Kind::Gpg);
    make_key(&mut shell, &mut app, &GPG_FIELDS);
    shell.flow(&mut app, F::MadeContinue);
    shell.flow(&mut app, F::ChangePlan);
    shell.flow(&mut app, F::AddPlace);
    shell.flow(&mut app, F::Continue);
    let mut app = shell.lock(&mut app);
    write_all(&mut shell, &mut app, &[("S1", false)]);
    shell.flow(&mut app, F::ChangePlan);
    let before = app.flow().unwrap().plan.clone();
    app.press(Action::Flow(F::RemovePlace(0)));
    assert_eq!(app.flow().unwrap().plan, before, "stays on the map");
    assert!(offered(&mut app, Action::Flow(F::Destroyed(0))));
    app.press(Action::Flow(F::Destroyed(0)));
    assert_eq!(app.flow().unwrap().plan.stick_places().len(), 2);
}

#[test]
fn two_files_of_one_vault_on_one_stick_cannot_serve_that_vault() {
    let mut shell = Shell::new();
    let main = vault("Main", &[PHRASE], 0x47);
    let mut d = Disk::empty("S1", "WORKSTICK");
    d.files = vec![
        ("a.ofv".into(), main.clone()),
        ("b.ofv".into(), main.clone()),
    ];
    shell.add(d);
    let mut app = started(Kind::Gpg);
    shell.flow(&mut app, F::VaultExisting(0));
    through_where(&mut shell, &mut app);
    shell.plug(&mut app, "S1");
    shell.flow(&mut app, F::PickFile(0, 0));
    expect(
        &mut app,
        "WORKSTICK has two copies of Vault 1: a.ofv and b.ofv",
    );
    assert!(!offered(&mut app, Action::Flow(F::Tap)));
    assert!(app.flow().unwrap().slots[0].bound.is_none());
}

#[test]
fn a_copy_that_differs_from_one_already_read_lets_the_person_pick_which_to_unlock() {
    let mut shell = Shell::new();
    let main = vault("Main", &[PHRASE], 0x48);
    let mut older = main.clone();
    let last = older.len() - 1;
    older[last] ^= 1;
    let mut a = Disk::empty("S1", "FIRST");
    a.serial = Some(1);
    a.files = vec![("main.ofv".into(), main.clone())];
    let mut b = Disk::empty("S2", "SECOND");
    b.serial = Some(2);
    b.files = vec![("main.ofv".into(), older.clone())];
    shell.add(a);
    shell.add(b);
    let mut app = started(Kind::Gpg);
    shell.flow(&mut app, F::VaultExisting(0));
    through_where(&mut shell, &mut app);
    shell.plug(&mut app, "S1");
    shell.flow(&mut app, F::PickFile(0, 0));
    shell.flow(&mut app, F::Tap);
    shell.pull(&mut app, "S1");
    shell.plug(&mut app, "S2");
    expect(&mut app, "SECOND's copy of Vault 1 differs from FIRST's");
    assert!(!offered(&mut app, Action::Flow(F::Tap)));
    // The first copy read stays the one to unlock.
    shell.flow(&mut app, F::UseCopy(false));
    assert!(offered(&mut app, Action::Flow(F::Tap)));
    shell.flow(&mut app, F::Tap);
    let first = app.inbox().iter().find(|i| i.name == "main.ofv").unwrap();
    assert_eq!(first.bytes, main, "the first copy is in Files");
    assert!(app.flow().unwrap().slots[1].bound.is_some());
}

#[test]
fn the_stick_the_machine_started_from_is_offered_as_already_read_and_one_press_takes_it() {
    let mut shell = Shell::new();
    empties(&mut shell, 1);
    let mut app = started(Kind::Gpg);
    through_where(&mut shell, &mut app);
    // The boot stick is in the machine all the time: offered, not asked for.
    shell.add(Disk::empty("BOOT", "FARADAY"));
    shell.present.push("BOOT".into());
    let mut info = shell.disk("BOOT").info();
    info.boot = true;
    app.storage(faraday_core::StorageEvent::Sticks(vec![info]));
    shell.pump(&mut app);
    expect_part(&mut app, "Use Boot stick, read at");
    let k = app.sticks().iter().position(|s| s.boot).unwrap();
    shell.flow(&mut app, F::UseBoot(k));
    // The press names the stick: it is bound with no second tap.
    assert!(app.flow().unwrap().slots[0].bound.is_some());
    assert!(!has_part(&mut app, "This is the stick for"));
    assert!(!offered(&mut app, Action::Flow(F::Tap)));
}

#[test]
fn the_tools_tile_starts_the_flow_with_no_vault_and_reads_continue_once_one_waits() {
    for kind in [Kind::Gpg, Kind::SecureBoot] {
        let mut app = process(Vec::new(), Vec::new(), Vec::new());
        let go = faraday_core::catalog::TILES[usize::from(tile_of(kind))].go;
        // No "Make a vault first": the flow decides the vaults.
        assert_eq!(app.tile_need(go), None, "{kind:?}");
        start(&mut app, kind);
        assert_eq!(app.screen, Screen::BackupFlow, "{kind:?}");
        app.press(Action::Flow(F::Back));
        assert_eq!(app.screen, Screen::Home);
        assert_eq!(app.tile_need(go), Some("Continue"), "{kind:?}");
        start(&mut app, kind);
        assert_eq!(app.screen, Screen::BackupFlow);
        assert_eq!(
            app.backup_flows().len(),
            1,
            "it resumes, it does not start another"
        );
    }
}

#[test]
fn a_written_copy_that_is_deleted_opens_its_card_again() {
    let mut shell = Shell::new();
    let mut app = gpg_locked(&mut shell);
    write_all(&mut shell, &mut app, &[("S1", false), ("S2", true)]);
    assert_eq!(step(&app), Step::Done);
    let name = shell.disk("S1").files[0].0.clone();
    // The person leaves the flow and deletes the file in the stick visit.
    app.press(Action::Flow(F::Back));
    shell.plug(&mut app, "S1");
    app.storage(faraday_core::StorageEvent::Wiped {
        stick: "S1".into(),
        name,
    });
    app.press(Action::Flow(F::Show(0)));
    assert_eq!(step(&app), Step::Write);
    assert_eq!(app.flow().unwrap().written_of(), (1, 2));
    assert!(app.backup_pending().is_some());
}

#[test]
fn back_does_not_leave_while_a_write_is_under_way() {
    let mut shell = Shell::new();
    let mut app = gpg_locked(&mut shell);
    shell.hold_writes = true;
    shell.plug(&mut app, "S1");
    app.press(Action::Flow(F::Back));
    assert_eq!(app.screen, Screen::BackupFlow);
    shell.pull(&mut app, "S1");
    app.press(Action::Flow(F::Back));
    assert_eq!(app.screen, Screen::Home);
}

#[test]
fn coming_back_to_the_writes_with_a_vault_open_again_is_on_lock_and_written_sticks_stay_written() {
    let mut shell = Shell::new();
    let mut app = gpg_locked(&mut shell);
    write_all(&mut shell, &mut app, &[("S1", false)]);
    // The person leaves, unlocks the vault that is in Files, and comes back.
    app.press(Action::Flow(F::Back));
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Open(0)));
    unlock(&mut app, PHRASE);
    assert_eq!(app.vaults().open.len(), 1);
    let i = app.backup_pending().map(|(i, _)| i).unwrap();
    app.press(Action::Flow(F::Show(i)));
    assert_eq!(step(&app), Step::Lock);
    assert_eq!(app.flow().unwrap().written_of(), (1, 2));
    // Lock and write: the next stick is the second.
    let mut app = shell.lock(&mut app);
    assert_eq!(step(&app), Step::Write);
    expect(&mut app, "Insert Stick 3: Vault 1 (Place 2)");
}

#[test]
fn enter_in_the_make_form_makes_the_key_and_adds_no_stray_record() {
    let mut shell = Shell::new();
    let mut app = to_make(&mut shell, Kind::Gpg);
    app.press(Action::Vault(V::FocusField(0)));
    type_text(&mut app, "Release");
    app.press(Action::Vault(V::FocusField(1)));
    type_text(&mut app, "release@example.org");
    app.event(faraday_shell_api::Event::Key(faraday_shell_api::Key::Enter));
    assert!(app.flow().unwrap().made, "{:?}", app.toast_text());
    let n: usize = app
        .vaults()
        .open
        .iter()
        .map(|o| {
            app.vaults()
                .store()
                .contents_for_tests(o.id)
                .of(kind::GPG)
                .count()
        })
        .sum();
    assert_eq!(
        n,
        app.vaults().open.len(),
        "one key in each vault, no stray"
    );
}

// ---- the review's fixes ----

/// The key made in a flow of `kind`, locked: step 6 on a fresh process.
fn locked_of(shell: &mut Shell, kind: Kind, fields: &[&str], public_at: Option<usize>) -> Faraday {
    empties(shell, 2);
    // Two sticks that do not read alike, so no hold is asked.
    shell.disks[1].serial = Some(2);
    shell.disks[1].label = "SECOND".into();
    let mut app = started(kind);
    if let Some(p) = public_at {
        shell.flow(&mut app, F::PublicAt(p));
    }
    through_where(shell, &mut app);
    bring_in(shell, &mut app, "S1", false);
    bring_in(shell, &mut app, "S2", false);
    create_vault(shell, &mut app, 0, PHRASE);
    make_key(shell, &mut app, fields);
    assert!(app.flow().unwrap().made, "{:?}", app.toast_text());
    shell.flow(&mut app, F::MadeContinue);
    let app = shell.lock(&mut app);
    assert_eq!(step(&app), Step::Write);
    app
}

fn boot_disk(shell: &mut Shell) {
    let mut d = Disk::empty("BOOT", "FARADAY");
    d.boot = true;
    d.serial = Some(900);
    shell.add(d);
}

#[test]
fn the_boot_stick_ends_at_the_bind_and_is_written_at_step_six_like_any_stick() {
    let mut shell = Shell::new();
    empties(&mut shell, 1);
    boot_disk(&mut shell);
    let mut app = started(Kind::Gpg);
    through_where(&mut shell, &mut app);
    shell.plug(&mut app, "BOOT");
    let k = app.sticks().iter().position(|s| s.boot).unwrap();
    shell.flow(&mut app, F::UseBoot(k));
    // It stays in: nothing waits for it to be pulled, the next is asked for.
    assert!(app.flow().unwrap().live.reading.is_none());
    assert!(app.flow().unwrap().slots[0].bound.is_some());
    assert_eq!(app.sheet, None);
    bring_in(&mut shell, &mut app, "S1", false);
    assert_eq!(step(&app), Step::Vaults);
    // Keys are made with no stick in, the boot stick too.
    shell.pull(&mut app, "BOOT");
    create_vault(&mut shell, &mut app, 0, PHRASE);
    make_key(&mut shell, &mut app, &GPG_FIELDS);
    shell.flow(&mut app, F::MadeContinue);
    let mut app = shell.lock(&mut app);
    assert_eq!(step(&app), Step::Write);
    // The stick it started from is offered for its place, and written.
    shell.plug(&mut app, "BOOT");
    let k = app.sticks().iter().position(|s| s.boot).unwrap();
    assert!(offered(&mut app, Action::Flow(F::UseBoot(k))));
    shell.flow(&mut app, F::UseBoot(k));
    assert_eq!(app.flow().unwrap().written_of(), (1, 2));
    assert!(
        shell
            .disk("BOOT")
            .files
            .iter()
            .any(|f| f.0.ends_with(".ofv"))
    );
    write_all(&mut shell, &mut app, &[("S1", true)]);
    assert_eq!(step(&app), Step::Done);
}

#[test]
fn a_stick_changed_by_a_write_that_was_cut_short_counts_when_it_is_put_back() {
    let mut shell = Shell::new();
    let mut app = gpg_locked(&mut shell);
    shell.hold_writes = true;
    shell.plug(&mut app, "S1");
    assert!(!shell.stashed.is_empty());
    // The write landed, the answer never came, the stick was pulled.
    shell.land_stashed(|_| true);
    shell.pull(&mut app, "S1");
    assert_eq!(app.flow().unwrap().written_of(), (0, 2));
    shell.hold_writes = false;
    shell.plug(&mut app, "S1");
    assert_eq!(
        app.flow().unwrap().written_of(),
        (1, 2),
        "{:?}",
        texts(&mut app)
    );
    let n = shell.disk("S1").files.len();
    shell.pull(&mut app, "S1");
    assert_eq!(shell.disk("S1").files.len(), n, "nothing was written twice");
    write_all(&mut shell, &mut app, &[("S2", true)]);
    assert_eq!(step(&app), Step::Done);
}

#[test]
fn use_another_stick_at_step_six_gives_the_slot_to_the_next_stick() {
    let mut shell = Shell::new();
    let mut app = gpg_locked(&mut shell);
    shell.add(Disk::empty("S3", "SPARE"));
    shell.disk_mut("S3").serial = Some(77);
    shell.add(Disk::empty("S4", "LATER"));
    shell.disk_mut("S4").serial = Some(78);
    shell.plug(&mut app, "S3");
    expect(&mut app, "Not Stick 2, read for Vault 1 (Place 1)");
    assert!(offered(&mut app, Action::Flow(F::UseAnother)));
    shell.flow(&mut app, F::UseAnother);
    assert!(app.flow().unwrap().slots[0].bound.is_none(), "rebound");
    shell.pull(&mut app, "S3");
    // The next stick is named later: tapped, then written.
    shell.plug(&mut app, "S4");
    expect(&mut app, "This is the stick for Vault 1 (Place 1)");
    shell.flow(&mut app, F::Tap);
    assert_eq!(app.flow().unwrap().written_of(), (1, 2));
    assert!(shell.disk("S4").files.iter().any(|f| f.0.ends_with(".ofv")));
    shell.pull(&mut app, "S4");
    write_all(&mut shell, &mut app, &[("S2", true)]);
    assert_eq!(step(&app), Step::Done);
}

#[test]
fn a_wipe_or_delete_on_an_unrelated_stick_reopens_nothing_before_or_after_a_lock() {
    let mut shell = Shell::new();
    let mut app = locked_of(&mut shell, Kind::Gpg, &GPG_FIELDS, None);
    write_all(&mut shell, &mut app, &[("S1", false), ("S2", true)]);
    assert_eq!(step(&app), Step::Done);
    let name = shell.disk("S1").files[0].0.clone();
    let mut u = Disk::empty("U", "ELSEWHERE");
    u.serial = Some(5150);
    u.files = vec![(name.clone(), vec![1, 2, 3])];
    shell.add(u);
    let wipe = |app: &mut Faraday, id: &str| {
        app.storage(faraday_core::StorageEvent::Wiped {
            stick: id.into(),
            name: name.clone(),
        });
    };
    shell.plug(&mut app, "U");
    wipe(&mut app, "U");
    assert_eq!(app.flow().unwrap().written_of(), (2, 2));
    shell.pull(&mut app, "U");
    // The same after a lock, which keeps only labels, serials and hashes.
    let mut app = shell.lock(&mut app);
    assert_eq!(app.flow().unwrap().written_of(), (2, 2));
    shell.plug(&mut app, "U");
    wipe(&mut app, "U");
    assert_eq!(app.flow().unwrap().written_of(), (2, 2));
    shell.pull(&mut app, "U");
    // The stick that was written still reopens its card.
    shell.plug(&mut app, "S1");
    wipe(&mut app, "S1");
    assert_eq!(app.flow().unwrap().written_of(), (1, 2));
}

#[test]
fn the_copy_not_picked_is_written_over_only_after_a_hold() {
    let mut shell = Shell::new();
    // Two copies of one vault, the same vault, the later one edited.
    let first = vault("Main", &[PHRASE], 0x49);
    let later = vault("Main edited", &[PHRASE], 0x49);
    assert_ne!(first, later);
    let mut a = Disk::empty("S1", "FIRST");
    a.serial = Some(1);
    a.files = vec![("main.ofv".into(), first.clone())];
    let mut b = Disk::empty("S2", "SECOND");
    b.serial = Some(2);
    b.files = vec![("main.ofv".into(), later.clone())];
    shell.add(a);
    shell.add(b);
    let mut app = started(Kind::Gpg);
    shell.flow(&mut app, F::VaultExisting(0));
    through_where(&mut shell, &mut app);
    shell.plug(&mut app, "S1");
    shell.flow(&mut app, F::PickFile(0, 0));
    shell.flow(&mut app, F::Tap);
    shell.pull(&mut app, "S1");
    shell.plug(&mut app, "S2");
    expect(&mut app, "SECOND's copy of Vault 1 differs from FIRST's");
    // The later copy is the one to unlock; the first stick's is declined.
    shell.flow(&mut app, F::UseCopy(true));
    shell.flow(&mut app, F::Tap);
    shell.pull(&mut app, "S2");
    assert_eq!(step(&app), Step::Vaults);
    let kept = app.inbox().iter().find(|i| i.name == "main.ofv").unwrap();
    assert_eq!(kept.bytes, later, "the later copy is in Files");
    unlock(&mut app, PHRASE);
    assert_eq!(step(&app), Step::Make, "{:?}", unlock_error(&app));
    make_key(&mut shell, &mut app, &GPG_FIELDS);
    shell.flow(&mut app, F::MadeContinue);
    let mut app = shell.lock(&mut app);
    // The stick whose copy was declined is not written over without a hold.
    shell.plug(&mut app, "S1");
    assert_eq!(shell.vault_writes(), 0);
    assert_eq!(shell.disk("S1").file("main.ofv"), Some(&first));
    assert!(offered(&mut app, Action::Flow(F::Hold)));
    shell.flow(&mut app, F::Hold);
    assert_eq!(shell.vault_writes(), 1);
    assert_ne!(shell.disk("S1").file("main.ofv"), Some(&first));
}

#[test]
fn a_first_stick_with_a_differing_copy_of_a_vault_in_files_asks_which_to_unlock() {
    let mut shell = Shell::new();
    let first = vault("Main", &[PHRASE], 0x4a);
    let other = vault("Main elsewhere", &[PHRASE], 0x4a);
    let mut d = Disk::empty("S1", "FIRST");
    d.serial = Some(1);
    d.files = vec![("main.ofv".into(), first.clone())];
    shell.add(d);
    // Files already holds another copy of the same vault.
    let mut app = process(
        vec![("main.ofv".into(), other.clone())],
        Vec::new(),
        Vec::new(),
    );
    keep_time(&mut app);
    start(&mut app, Kind::Gpg);
    shell.flow(&mut app, F::VaultExisting(0));
    through_where(&mut shell, &mut app);
    shell.plug(&mut app, "S1");
    shell.flow(&mut app, F::PickFile(0, 0));
    expect(&mut app, "FIRST's copy of Vault 1 differs from main.ofv's");
}

#[test]
fn a_lock_before_copy_the_key_in_the_added_vault_round_resumes_on_the_vaults() {
    let mut shell = Shell::new();
    empties(&mut shell, 3);
    shell.disks[2].serial = Some(33);
    let mut app = to_make(&mut shell, Kind::Gpg);
    make_key(&mut shell, &mut app, &GPG_FIELDS);
    shell.flow(&mut app, F::MadeContinue);
    shell.flow(&mut app, F::ChangePlan);
    shell.flow(&mut app, F::AddPlace);
    shell.flow(&mut app, F::PlaceVault(2, 1));
    shell.flow(&mut app, F::Continue);
    create_vault(&mut shell, &mut app, 1, PHRASE_2);
    assert_eq!(step(&app), Step::Make);
    assert!(app.flow().unwrap().adding);
    // Locked before the key was copied into the new vault.
    let app = shell.lock(&mut app);
    assert_eq!(app.screen, Screen::BackupFlow);
    assert_eq!(step(&app), Step::Vaults);
}

#[test]
fn a_kept_state_that_does_not_read_is_dropped_with_a_toast() {
    let mut shell = Shell::new();
    let app = gpg_locked(&mut shell);
    let good = faraday_core::backup_flow::encode(app.backup_flows());
    let text = String::from_utf8(good.clone()).unwrap();
    assert!(text.starts_with("version\t3\n"));
    let cases: Vec<(&str, Vec<u8>)> = vec![
        ("noise", vec![0xff, 0x00, 0x13, 0x37, 0xfe]),
        (
            "another version",
            text.replacen("version\t3", "version\t9", 1).into_bytes(),
        ),
        (
            "a vault out of range",
            text.replacen("place\t1\t0", "place\t1\t7", 1).into_bytes(),
        ),
        (
            "a place out of range",
            text.replacen("slot\t1\tp1", "slot\t1\tp9", 1).into_bytes(),
        ),
        (
            "a step too far",
            text.replacen("flow\tgpg\t5", "flow\tgpg\t99", 1)
                .into_bytes(),
        ),
        ("cut short", good[..good.len() / 2].to_vec()),
    ];
    for (name, bytes) in cases {
        let mut app = process(Vec::new(), Vec::new(), vec![("backup-flow".into(), bytes)]);
        assert!(
            app.flow().is_none(),
            "{name}: a flow was kept: {:?}",
            app.toast_text()
        );
        assert_eq!(
            app.toast_text(),
            Some("A kept backup did not read and was dropped"),
            "{name}"
        );
        // And the app goes on.
        start(&mut app, Kind::Gpg);
        assert_eq!(app.screen, Screen::BackupFlow, "{name}");
    }
}

#[test]
fn the_plan_cannot_be_changed_while_a_stick_is_being_written() {
    let mut shell = Shell::new();
    let mut app = gpg_locked(&mut shell);
    shell.hold_writes = true;
    shell.plug(&mut app, "S1");
    assert!(!shell.stashed.is_empty(), "a write is under way");
    let plan = app.flow().unwrap().plan.clone();
    assert!(!offered(&mut app, Action::Flow(F::ChangePlan)));
    app.press(Action::Flow(F::ChangePlan));
    app.press(Action::Flow(F::AddPlace));
    app.press(Action::Flow(F::RemovePlace(1)));
    app.press(Action::Flow(F::PlaceVault(0, 1)));
    assert_eq!(app.flow().unwrap().plan, plan);
    assert_eq!(step(&app), Step::Write);
}

#[test]
fn setting_the_time_at_make_and_locking_from_there_keeps_the_flow() {
    let mut shell = Shell::new();
    empties(&mut shell, 2);
    // A clock nobody has checked: Make asks for the time first.
    let mut app = process(Vec::new(), Vec::new(), Vec::new());
    start(&mut app, Kind::Gpg);
    through_where(&mut shell, &mut app);
    bring_in(&mut shell, &mut app, "S1", false);
    bring_in(&mut shell, &mut app, "S2", true);
    create_vault(&mut shell, &mut app, 0, PHRASE);
    assert_eq!(step(&app), Step::Make);
    make_key(&mut shell, &mut app, &GPG_FIELDS);
    assert!(!app.flow().unwrap().made);
    assert_ne!(app.screen, Screen::BackupFlow, "the time is asked for");
    let app = shell.lock(&mut app);
    assert!(app.flow().is_some(), "the flow is not lost");
    assert_eq!(app.screen, Screen::BackupFlow);
}

#[test]
fn a_new_vault_that_holds_other_entries_is_not_dropped_with_its_place() {
    let mut shell = Shell::new();
    empties(&mut shell, 3);
    shell.disks[2].serial = Some(33);
    let mut app = to_make(&mut shell, Kind::Gpg);
    make_key(&mut shell, &mut app, &GPG_FIELDS);
    shell.flow(&mut app, F::MadeContinue);
    shell.flow(&mut app, F::ChangePlan);
    shell.flow(&mut app, F::AddPlace);
    shell.flow(&mut app, F::PlaceVault(2, 1));
    shell.flow(&mut app, F::Continue);
    create_vault(&mut shell, &mut app, 1, PHRASE_2);
    shell.flow(&mut app, F::CopyKey);
    // The person put something else in the new vault.
    let id = app.vaults().open[1].id;
    faraday_core::testkit::vault_edit(&mut app, id, |s| {
        s.insert(id, faraday_vault::Record::new(kind::NOTE))
    });
    shell.flow(&mut app, F::ChangePlan);
    shell.flow(&mut app, F::RemovePlace(2));
    assert_eq!(app.vaults().open.len(), 2, "the vault stays open");
    assert!(
        app.vaults().open.iter().any(|o| app
            .vaults()
            .store()
            .contents_for_tests(o.id)
            .of(kind::NOTE)
            .count()
            == 1),
        "with what it holds"
    );
    assert_eq!(
        app.vaults()
            .open
            .iter()
            .map(|o| app
                .vaults()
                .store()
                .contents_for_tests(o.id)
                .of(kind::GPG)
                .count())
            .sum::<usize>(),
        1,
        "only this flow's key was taken out of it"
    );
}

#[test]
fn the_last_vault_holding_the_key_is_never_taken_out_of_the_plan() {
    let mut shell = Shell::new();
    let mut app = to_make(&mut shell, Kind::Gpg);
    make_key(&mut shell, &mut app, &GPG_FIELDS);
    shell.flow(&mut app, F::MadeContinue);
    shell.flow(&mut app, F::ChangePlan);
    // Place 2 moves to a new vault, which does not hold the key yet.
    shell.flow(&mut app, F::PlaceVault(1, 1));
    let plan = app.flow().unwrap().plan.clone();
    shell.flow(&mut app, F::RemovePlace(0));
    let f = app.flow().unwrap();
    assert!(
        f.plan == plan || f.made_in.iter().any(|v| f.plan.used_vaults().contains(v)),
        "a vault that holds the key stays in the plan"
    );
    let holding: usize = app
        .vaults()
        .open
        .iter()
        .map(|o| {
            app.vaults()
                .store()
                .contents_for_tests(o.id)
                .of(kind::GPG)
                .count()
        })
        .sum();
    assert_eq!(holding, 1, "the key is still in a vault");
}

#[test]
fn nothing_is_written_when_a_vault_has_lost_its_outbox_copy() {
    let mut shell = Shell::new();
    let app = gpg_locked(&mut shell);
    drop(app);
    shell.boxes.1.retain(|(n, _)| !n.ends_with(".ofv"));
    let mut app = shell.restart();
    shell.plug(&mut app, "S1");
    shell.pull(&mut app, "S1");
    shell.plug(&mut app, "S2");
    shell.flow(&mut app, F::Hold);
    assert!(shell.writes.is_empty(), "{:?}", shell.writes);
    assert_eq!(app.flow().map_or(0, |f| f.written_of().0), 0);
}

#[test]
fn a_key_deleted_from_a_vault_before_the_lock_is_not_sealed_and_not_written() {
    let mut shell = Shell::new();
    let mut app = to_make(&mut shell, Kind::Gpg);
    make_key(&mut shell, &mut app, &GPG_FIELDS);
    shell.flow(&mut app, F::MadeContinue);
    for k in 0..app.vaults().open.len() {
        let id = app.vaults().open[k].id;
        faraday_core::testkit::vault_edit(&mut app, id, |s| {
            s.remove_where(id, |r| r.kind() == kind::GPG)
        });
        faraday_core::testkit::vault_changed(&mut app, id);
    }
    let mut app = shell.lock(&mut app);
    assert!(app.flow().is_none_or(|f| f.sealing.is_empty()));
    shell.plug(&mut app, "S1");
    shell.pull(&mut app, "S1");
    shell.plug(&mut app, "S2");
    shell.flow(&mut app, F::Hold);
    assert!(shell.writes.is_empty(), "{:?}", shell.writes);
}

#[test]
fn the_code_of_a_signature_waits_for_the_backup_like_the_signing() {
    let mut shell = Shell::new();
    let mut app = to_make(&mut shell, Kind::Gpg);
    make_key(&mut shell, &mut app, &GPG_FIELDS);
    app.press(Action::Nav(Screen::VaultContents));
    app.press(Action::Vault(V::Category(4)));
    app.press(Action::Vault(V::Item(0)));
    expect(&mut app, "Back up first");
    app.press(Action::ShowCode(faraday_core::Code::GpgSignature([0; 8])));
    assert!(
        app.toast_text()
            .is_some_and(|t| t.contains("Back up first"))
    );
    assert_ne!(app.screen, Screen::Codes);
}

#[test]
fn a_stick_is_read_and_written_only_by_a_clean_process() {
    let mut shell = Shell::new();
    let mut app = gpg_locked(&mut shell);
    // Something secret is typed: the process is not clean.
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Open(0)));
    app.press(Action::Vault(V::FocusPassphrase));
    type_text(&mut app, PHRASE);
    let i = app.backup_pending().map(|(i, _)| i).unwrap();
    app.press(Action::Flow(F::Show(i)));
    shell.plug(&mut app, "S1");
    shell.flow(&mut app, F::Hold);
    shell.flow(&mut app, F::Tap);
    assert!(shell.writes.is_empty(), "{:?}", shell.writes);
    assert!(shell.unclean_io.is_empty(), "{:?}", shell.unclean_io);
    // A whole flow on a clean process asks for no stick unclean.
    let mut shell = Shell::new();
    let mut app = gpg_locked(&mut shell);
    write_all(&mut shell, &mut app, &[("S1", false), ("S2", true)]);
    assert_eq!(step(&app), Step::Done);
    assert!(shell.unclean_io.is_empty(), "{:?}", shell.unclean_io);
}

#[test]
fn a_write_that_failed_or_read_back_wrong_is_not_counted() {
    let mut shell = Shell::new();
    let mut app = gpg_locked(&mut shell);
    shell.fail_writes = true;
    shell.plug(&mut app, "S1");
    assert_eq!(app.flow().unwrap().written_of(), (0, 2));
    shell.pull(&mut app, "S1");
    shell.fail_writes = false;
    shell.mismatch_writes = true;
    shell.plug(&mut app, "S1");
    assert_eq!(app.flow().unwrap().written_of(), (0, 2));
    shell.pull(&mut app, "S1");
}

/// The kept state names only what a stick shows and public values.
fn nothing_secret_kept(app: &Faraday, secret: &[&str]) {
    let bytes = faraday_core::backup_flow::encode(app.backup_flows());
    let text = String::from_utf8(bytes).expect("text");
    let lines = [
        "version", "flow", "plan", "pub", "place", "vault", "slot", "sfile", "bound", "list",
        "vread", "pick", "wrote", "wfile", "seal",
    ];
    for l in text.lines() {
        let head = l.split('\t').next().unwrap();
        assert!(lines.contains(&head), "{l}");
        assert!(l.len() < 300, "a long line: {}", &l[..60]);
    }
    for s in secret {
        assert!(!text.contains(s), "{s} is in the kept state");
    }
    assert!(!text.contains("PRIVATE"));
}

#[test]
fn the_kept_state_of_a_gpg_or_secure_boot_flow_holds_nothing_secret() {
    let mut shell = Shell::new();
    let app = locked_of(&mut shell, Kind::Gpg, &GPG_FIELDS, None);
    nothing_secret_kept(&app, &[PHRASE]);
    let mut shell = Shell::new();
    let app = locked_of(&mut shell, Kind::SecureBoot, &["Owner"], None);
    nothing_secret_kept(&app, &[PHRASE]);
}

#[test]
fn the_public_files_are_written_to_the_stick_of_the_place_the_plan_names() {
    for (kind, fields) in [
        (Kind::Gpg, GPG_FIELDS.to_vec()),
        (Kind::SecureBoot, vec!["Owner"]),
    ] {
        for at in [0usize, 1] {
            let mut shell = Shell::new();
            let mut app = locked_of(&mut shell, kind, &fields, Some(at));
            let names = app.flow().unwrap().public.clone();
            assert!(!names.is_empty(), "{kind:?}: public files were made");
            write_all(&mut shell, &mut app, &[("S1", false), ("S2", true)]);
            assert_eq!(step(&app), Step::Done, "{kind:?} {at}");
            // A Secure Boot key's enrolment files go in the last write.
            let last = kind == Kind::SecureBoot;
            let carries = if at == 0 && !last { "S1" } else { "S2" };
            let other = if at == 0 && !last { "S2" } else { "S1" };
            assert_eq!(
                shell.disk(carries).files.len(),
                1 + names.len(),
                "{kind:?} {at}"
            );
            assert_eq!(shell.disk(other).files.len(), 1, "{kind:?} {at}");
            // Nothing the flow made waits For the stick.
            assert!(app.outbox.is_empty(), "{kind:?} {at}");
        }
    }
}

#[test]
fn a_place_may_keep_a_stick_for_each_vault_or_one_stick_for_both() {
    let mut shell = Shell::new();
    empties(&mut shell, 3);
    shell.disks[1].serial = Some(2);
    shell.disks[2].serial = Some(3);
    let mut app = started(Kind::Gpg);
    // Place 2 keeps a new Vault 2; Place 1 keeps both.
    shell.flow(&mut app, F::PlaceVault(1, 1));
    shell.flow(&mut app, F::AlsoVault(0, 1));
    assert_eq!(app.flow().unwrap().slots.len(), 3, "a stick for each vault");
    assert_eq!(app.flow().unwrap().plan.places[0], [0, 1]);
    shell.flow(&mut app, F::Together(0));
    assert_eq!(app.flow().unwrap().slots.len(), 2, "one stick for both");
    shell.flow(&mut app, F::Together(0));
    assert_eq!(app.flow().unwrap().slots.len(), 3);
    through_where(&mut shell, &mut app);
    for k in 1..=3 {
        bring_in(&mut shell, &mut app, &format!("S{k}"), false);
    }
    create_vault(&mut shell, &mut app, 0, PHRASE);
    create_vault(&mut shell, &mut app, 1, PHRASE_2);
    assert_eq!(step(&app), Step::Make, "{:?}", unlock_error(&app));
    make_key(&mut shell, &mut app, &GPG_FIELDS);
    shell.flow(&mut app, F::MadeContinue);
    let mut app = shell.lock(&mut app);
    write_all(
        &mut shell,
        &mut app,
        &[("S1", false), ("S2", true), ("S3", true)],
    );
    assert_eq!(step(&app), Step::Done, "{:?}", texts(&mut app));
    assert_eq!(shell.vault_writes(), 3, "Vault 1 once, Vault 2 twice");
}

// ---- the verification review ----

fn subject_bytes(app: &Faraday) -> Vec<u8> {
    let id = app.flow().unwrap().subject.clone().unwrap().id;
    (0..id.len() / 2)
        .map(|i| u8::from_str_radix(&id[2 * i..2 * i + 2], 16).unwrap())
        .collect()
}

#[test]
fn a_plan_changed_at_step_six_and_locked_without_continue_keeps_the_key_waiting() {
    let mut shell = Shell::new();
    let mut app = gpg_locked(&mut shell);
    let id = subject_bytes(&app);
    assert!(app.backup_waits(Kind::Gpg, &id).is_some());
    // A new vault for Place 2, with no Continue; then a lock.
    shell.flow(&mut app, F::ChangePlan);
    shell.flow(&mut app, F::PlaceVault(1, 1));
    let app = shell.lock(&mut app);
    assert!(
        app.backup_waits(Kind::Gpg, &id).is_some(),
        "the gate stays closed"
    );
    assert!(app.flow().is_some_and(|f| f.made));
}

#[test]
fn a_kept_state_that_does_not_read_leaves_the_key_waiting() {
    let mut shell = Shell::new();
    let app = gpg_locked(&mut shell);
    let id = subject_bytes(&app);
    // The kept flows are unreadable; the keys that wait are kept apart.
    let mut kept = shell.boxes.2.clone();
    assert!(kept.iter().any(|(n, _)| n == "backup-gate"));
    for (n, b) in kept.iter_mut() {
        if n == "backup-flow" {
            *b = vec![0xff, 0x13, 0x37];
        }
    }
    let app = process(shell.boxes.0.clone(), shell.boxes.1.clone(), kept);
    assert!(app.backup_waits(Kind::Gpg, &id).is_some());
    // And with the state of the flows gone altogether.
    let mut kept = shell.boxes.2.clone();
    kept.retain(|(n, _)| n != "backup-flow");
    let app = process(shell.boxes.0.clone(), shell.boxes.1.clone(), kept);
    assert!(app.backup_waits(Kind::Gpg, &id).is_some());
    // A key whose backup is done is not kept waiting.
    let mut shell = Shell::new();
    let mut app = gpg_locked(&mut shell);
    write_all(&mut shell, &mut app, &[("S1", false), ("S2", true)]);
    assert!(shell.boxes.2.iter().all(|(n, _)| n != "backup-gate"));
}

#[test]
fn a_flow_with_nothing_sealed_past_the_lock_is_repaired_not_dropped() {
    let mut shell = Shell::new();
    let app = gpg_locked(&mut shell);
    let id = subject_bytes(&app);
    let text = String::from_utf8(faraday_core::backup_flow::encode(app.backup_flows())).unwrap();
    let unsealed: String = text
        .lines()
        .filter(|l| !l.starts_with("seal\t"))
        .map(|l| format!("{l}\n"))
        .collect();
    let app = process(
        Vec::new(),
        Vec::new(),
        vec![("backup-flow".into(), unsealed.into_bytes())],
    );
    assert!(app.backup_waits(Kind::Gpg, &id).is_some());
    assert_ne!(step(&app), Step::Write, "back where a copy is sealed");
}

/// An existing vault on two sticks, the key made into it, locked.
fn existing_locked(shell: &mut Shell) -> Faraday {
    let main = vault("Main", &[PHRASE], 0x55);
    for (id, label, serial) in [("S1", "FIRST", 1), ("S2", "SECOND", 2)] {
        let mut d = Disk::empty(id, label);
        d.serial = Some(serial);
        d.files = vec![("main.ofv".into(), main.clone())];
        shell.add(d);
    }
    let mut app = started(Kind::Gpg);
    shell.flow(&mut app, F::VaultExisting(0));
    through_where(shell, &mut app);
    shell.plug(&mut app, "S1");
    shell.flow(&mut app, F::PickFile(0, 0));
    shell.flow(&mut app, F::Tap);
    shell.pull(&mut app, "S1");
    shell.plug(&mut app, "S2");
    shell.flow(&mut app, F::Tap);
    shell.pull(&mut app, "S2");
    assert_eq!(step(&app), Step::Vaults, "{:?}", texts(&mut app));
    unlock(&mut app, PHRASE);
    assert_eq!(step(&app), Step::Make);
    make_key(shell, &mut app, &GPG_FIELDS);
    shell.flow(&mut app, F::MadeContinue);
    let app = shell.lock(&mut app);
    assert_eq!(step(&app), Step::Write);
    app
}

#[test]
fn a_stick_named_at_step_six_with_another_copy_of_the_vault_is_written_over_only_after_a_hold() {
    let mut shell = Shell::new();
    let mut app = existing_locked(&mut shell);
    let edited = vault("Main edited", &[PHRASE], 0x55);
    let mut u = Disk::empty("U", "OTHER");
    u.serial = Some(9);
    u.files = vec![("main.ofv".into(), edited.clone())];
    shell.add(u);
    shell.plug(&mut app, "U");
    expect(&mut app, "Not Stick 2, read for Vault 1 (Place 1)");
    shell.flow(&mut app, F::UseAnother);
    shell.pull(&mut app, "U");
    shell.plug(&mut app, "U");
    expect(&mut app, "This is the stick for Vault 1 (Place 1)");
    // Tapped: nothing is written over the other copy yet.
    shell.flow(&mut app, F::Tap);
    assert_eq!(shell.vault_writes(), 0);
    assert_eq!(shell.disk("U").file("main.ofv"), Some(&edited));
    assert!(offered(&mut app, Action::Flow(F::Hold)));
    // The copy is not asked about either: the key is made, none is unlocked.
    assert!(!offered(&mut app, Action::Flow(F::UseCopy(true))));
    shell.flow(&mut app, F::Hold);
    assert_eq!(shell.vault_writes(), 1);
    assert_ne!(shell.disk("U").file("main.ofv"), Some(&edited));
}

#[test]
fn an_ordinary_visit_after_done_does_not_write_the_flows_vault_over_another_copy() {
    let mut shell = Shell::new();
    let mut app = existing_locked(&mut shell);
    write_all(&mut shell, &mut app, &[("S1", false), ("S2", true)]);
    assert_eq!(step(&app), Step::Done);
    let mut app = shell.lock(&mut app);
    // Another stick holds a copy of the same vault, edited elsewhere.
    let edited = vault("Main edited", &[PHRASE], 0x55);
    let mut t = Disk::empty("T", "OTHER");
    t.serial = Some(7);
    t.files = vec![("elsewhere.ofv".into(), edited.clone())];
    shell.add(t);
    if app.screen == Screen::BackupFlow {
        app.press(Action::Flow(F::Back));
    }
    shell.plug(&mut app, "T");
    shell.writes.clear();
    app.press(Action::Nav(Screen::Visit));
    let k = app
        .inbox()
        .iter()
        .position(|i| i.name == "main.ofv")
        .unwrap();
    app.press(Action::VisitInbox(k));
    shell.press(&mut app, Action::VisitWrite);
    if app.sheet.is_some() {
        shell.press(&mut app, Action::VisitReplace);
    }
    assert_eq!(
        shell.disk("T").file("elsewhere.ofv"),
        Some(&edited),
        "the other copy is not written over"
    );
    assert_eq!(
        shell.disk("T").files.len(),
        2,
        "the visit wrote, beside it: {:?}",
        shell.writes
    );
}

#[test]
fn a_copy_deleted_after_done_is_written_again_and_the_key_signs() {
    let mut shell = Shell::new();
    let mut app = gpg_locked(&mut shell);
    write_all(&mut shell, &mut app, &[("S1", false), ("S2", true)]);
    assert_eq!(step(&app), Step::Done);
    let names: Vec<String> = shell.disk("S1").files.iter().map(|f| f.0.clone()).collect();
    assert_eq!(names.len(), 2, "the copy and the public key");
    app.press(Action::Flow(F::Back));
    shell.plug(&mut app, "S1");
    for n in &names {
        app.storage(faraday_core::StorageEvent::Wiped {
            stick: "S1".into(),
            name: n.clone(),
        });
    }
    shell.disk_mut("S1").files.clear();
    app.press(Action::Flow(F::Show(0)));
    assert_eq!(step(&app), Step::Write);
    assert_eq!(app.flow().unwrap().written_of(), (1, 2));
    // The stick is put back: the copy and the public key go on again.
    shell.pull(&mut app, "S1");
    shell.plug(&mut app, "S1");
    assert_eq!(step(&app), Step::Done, "{:?}", texts(&mut app));
    assert_eq!(shell.disk("S1").files.len(), 2);
    assert!(app.outbox.is_empty());
    // And the key signs once its vault is unlocked.
    shell.flow(&mut app, F::UnlockDone(0));
    unlock(&mut app, PHRASE);
    app.press(Action::Vault(V::GpgSignPick));
    assert!(app.vaults().signing);
}

#[test]
fn stopping_a_whole_wipe_after_the_partition_was_emptied_reopens_the_copies() {
    let mut shell = Shell::new();
    let mut app = gpg_locked(&mut shell);
    write_all(&mut shell, &mut app, &[("S1", false), ("S2", true)]);
    app.press(Action::Flow(F::Back));
    shell.plug(&mut app, "S1");
    shell.press(&mut app, Action::Nav(Screen::Visit));
    shell.press(&mut app, Action::DeleteFiles);
    app.press(Action::DeleteWhat(2));
    app.press(Action::DeletePasses(1));
    shell.hold(
        &mut app,
        Action::Vault(faraday_core::vault::VaultAction::HoldWipe),
    );
    assert_eq!(
        app.backup_flows()[0].written_of(),
        (2, 2),
        "nothing emptied yet"
    );
    // The partition is made empty; the person stops the wipe after that.
    app.press(Action::ZeroStop);
    app.storage(faraday_core::StorageEvent::Zeroing {
        stick: "S1".into(),
        next: 2,
        total: 0,
        done: false,
    });
    assert!(app.zeroing().is_none(), "the wipe stopped");
    assert_eq!(
        app.backup_flows()[0].written_of(),
        (1, 2),
        "the stick was made empty, so its copy is gone"
    );
}

#[test]
fn a_stick_the_space_of_which_cannot_be_asked_is_unusable_and_not_waited_on() {
    let mut shell = Shell::new();
    let mut d = Disk::empty("S1", "KEYSTICK");
    d.unreadable = true;
    shell.add(d);
    let mut app = started(Kind::Gpg);
    through_where(&mut shell, &mut app);
    shell.plug(&mut app, "S1");
    expect_part(&mut app, "cannot be read");
    assert!(offered(&mut app, Action::Flow(F::UseAnother)));
    assert!(!offered(&mut app, Action::Flow(F::Tap)));
    assert!(app.flow().unwrap().slots[0].bound.is_none());
}

#[test]
fn secure_boot_enrolment_files_go_with_the_last_write_not_the_first() {
    let mut shell = Shell::new();
    let mut app = locked_of(&mut shell, Kind::SecureBoot, &["Owner"], None);
    let names = app.flow().unwrap().public.clone();
    assert!(!names.is_empty());
    write_all(&mut shell, &mut app, &[("S1", false)]);
    assert_eq!(shell.disk("S1").files.len(), 1, "only the vault copy");
    write_all(&mut shell, &mut app, &[("S2", true)]);
    assert_eq!(step(&app), Step::Done);
    assert_eq!(shell.disk("S2").files.len(), 1 + names.len());
    assert!(app.outbox.is_empty());
}

// ---- the re-check ----

#[test]
fn the_enrolment_files_of_a_waiting_secure_boot_key_are_in_the_backup_not_tickable() {
    let mut shell = Shell::new();
    let mut app = locked_of(&mut shell, Kind::SecureBoot, &["Owner"], None);
    let names = app.flow().unwrap().public.clone();
    assert!(!names.is_empty());
    write_all(&mut shell, &mut app, &[("S1", false)]);
    // The flow is left before the last write; an ordinary visit follows.
    app.press(Action::Flow(F::Back));
    shell.plug(&mut app, "S2");
    app.press(Action::Nav(Screen::Visit));
    for n in &names {
        assert!(app.in_the_backup(n), "{n}");
        assert!(!app.visit().out.contains(n), "{n} is not ticked");
        let k = app.outbox.iter().position(|i| &i.name == n).unwrap();
        app.press(Action::VisitOut(k));
        assert!(!app.visit().out.contains(n), "{n} cannot be ticked");
    }
    app.press(Action::VisitOutAll);
    assert!(names.iter().all(|n| !app.visit().out.contains(n)));
}

#[test]
fn a_stick_not_handed_out_yet_shows_no_problem_once_it_is() {
    let mut shell = Shell::new();
    let mut d = Disk::empty("S1", "KEYSTICK");
    d.unreadable = true;
    shell.add(d);
    let mut app = started(Kind::Gpg);
    through_where(&mut shell, &mut app);
    shell.plug(&mut app, "S1");
    expect_part(&mut app, "cannot be read");
    assert!(!has_part(&mut app, "Format it"));
    // The grant hands it out: the next listing, the same stick.
    shell.disk_mut("S1").unreadable = false;
    app.storage(faraday_core::StorageEvent::Sticks(shell.infos()));
    shell.pump(&mut app);
    assert!(!has_part(&mut app, "cannot be read"));
    assert!(offered(&mut app, Action::Flow(F::Tap)));
    // A real non-FAT stick that is put right, the same way.
    shell.pull(&mut app, "S1");
    shell.disk_mut("S1").not_fat = true;
    shell.plug(&mut app, "S1");
    expect_part(&mut app, "Format it as FAT32");
    shell.disk_mut("S1").not_fat = false;
    app.storage(faraday_core::StorageEvent::Sticks(shell.infos()));
    shell.pump(&mut app);
    assert!(!has_part(&mut app, "Format it"));
    assert!(offered(&mut app, Action::Flow(F::Tap)));
}

#[test]
fn a_lock_that_cannot_keep_a_waiting_backup_does_not_go_ahead() {
    let mut shell = Shell::new();
    let mut app = to_make(&mut shell, Kind::Gpg);
    make_key(&mut shell, &mut app, &GPG_FIELDS);
    shell.flow(&mut app, F::MadeContinue);
    shell.fail_save = true;
    app.press(Action::Lock);
    shell.pump(&mut app);
    assert!(!app.restart_requested(), "the lock was refused");
    assert!(
        app.toast_text()
            .is_some_and(|t| t.contains("Sealed, not restarted"))
    );
    // With the state kept again, it goes.
    shell.fail_save = false;
    let app = shell.lock(&mut app);
    assert_eq!(step(&app), Step::Write);
}

#[test]
fn a_whole_wipe_that_fails_or_is_cut_off_before_its_first_answer_reopens_the_copies() {
    for cut in ["fails", "pulled"] {
        let mut shell = Shell::new();
        let mut app = gpg_locked(&mut shell);
        write_all(&mut shell, &mut app, &[("S1", false), ("S2", true)]);
        app.press(Action::Flow(F::Back));
        shell.plug(&mut app, "S1");
        shell.press(&mut app, Action::Nav(Screen::Visit));
        shell.press(&mut app, Action::DeleteFiles);
        app.press(Action::DeleteWhat(2));
        app.press(Action::DeletePasses(1));
        shell.hold(
            &mut app,
            Action::Vault(faraday_core::vault::VaultAction::HoldWipe),
        );
        assert_eq!(app.backup_flows()[0].written_of(), (2, 2), "{cut}");
        if cut == "fails" {
            app.storage(faraday_core::StorageEvent::ZeroFailed {
                stick: "S1".into(),
                reason: "cannot make it empty".into(),
            });
        } else {
            shell.pull(&mut app, "S1");
        }
        assert_eq!(
            app.backup_flows()[0].written_of(),
            (1, 2),
            "{cut}: the FATs may be gone"
        );
    }
}

/// A lock without sealing, past a vault of the plan that cannot be
/// sealed, leaves that vault with no sealed copy: the flow does not go on
/// to its writes but back to the round that makes one.
#[test]
fn a_flow_whose_vault_was_not_sealed_at_the_lock_goes_back_to_its_vaults() {
    use faraday_vault::records::{Record, field, kind};
    let mut shell = Shell::new();
    let mut app = to_make(&mut shell, Kind::Gpg);
    make_key(&mut shell, &mut app, &GPG_FIELDS);
    shell.flow(&mut app, F::MadeContinue);
    assert_eq!(step(&app), Step::Lock);
    // The plan's vault filled past its slot: it cannot be sealed.
    let id = app.vaults().open[0].id;
    let slot = app.vaults().open[0].header().slot_len as usize;
    let filler = Record::new(kind::ENTRY)
        .with(field::TITLE, b"filler")
        .with(field::NOTES, "x".repeat(slot).as_bytes());
    faraday_core::testkit::vault_edit(&mut app, id, |s| s.insert(id, filler));
    faraday_core::testkit::vault_changed(&mut app, id);
    app.press(Action::Lock);
    assert_eq!(app.sheet, Some(faraday_core::Sheet::LockAsk));
    app.press(Action::LockUnsealed);
    shell.pump(&mut app);
    assert!(app.restart_requested(), "locked");
    let app = shell.restart();
    assert_eq!(step(&app), Step::Vaults, "on to the writes unsealed");
}
