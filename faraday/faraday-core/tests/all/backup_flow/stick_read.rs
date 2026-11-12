//! A stick read before any vault is unlocked: what a file on it is known
//! as, and what a flow keeps of the files it did not choose
//! (`docs/BACKUP-FLOW.md` §4.2).

use super::*;

/// The question's row for `name`, pressed when the stick asks which vault.
fn pick_if_asked(shell: &mut Shell, app: &mut Faraday, name: &str) {
    if !has(app, "Which vault is Vault 1?") {
        return;
    }
    let k = app
        .flow()
        .unwrap()
        .live
        .reading
        .as_ref()
        .unwrap()
        .files
        .iter()
        .position(|(f, _)| f.name == name)
        .expect("the file is on the stick");
    shell.flow(app, F::PickFile(0, k));
}

#[test]
fn a_vault_opened_from_files_is_not_the_copy_of_another_vault_sharing_its_name() {
    let mut shell = Shell::new();
    let a = vault("Alpha", PHRASE, 0x61);
    let b = vault("Beta", PHRASE_2, 0x62);
    let mut s1 = Disk::empty("S1", "ONE");
    s1.serial = Some(1);
    s1.files = vec![("vault.kdbx".into(), b.clone())];
    let mut s2 = Disk::empty("S2", "TWO");
    s2.serial = Some(2);
    // The stick holds Alpha's sealed copy, which Files also has.
    s2.files = vec![("vault.kdbx".into(), a.clone())];
    shell.add(s1);
    shell.add(s2);
    let mut app = process(
        Vec::new(),
        vec![("vault.kdbx".into(), a.clone())],
        Vec::new(),
    );
    keep_time(&mut app);
    start(&mut app, Kind::Gpg);
    shell.flow(&mut app, F::VaultExisting(0));
    through_where(&mut shell, &mut app);
    shell.plug_known(&mut app, "S1");
    pick_if_asked(&mut shell, &mut app, "vault.kdbx");
    shell.flow(&mut app, F::Tap);
    shell.pull(&mut app, "S1");
    shell.plug_known(&mut app, "S2");
    shell.flow(&mut app, F::Tap);
    shell.pull(&mut app, "S2");
    assert_eq!(step(&app), Step::Vaults);
    shell.flow(&mut app, F::UnlockVault(0));
    unlock(&mut app, PHRASE_2);
    assert_eq!(step(&app), Step::Make, "{:?}", unlock_error(&app));
    make_key(&mut shell, &mut app, &GPG_FIELDS);
    shell.flow(&mut app, F::MadeContinue);
    let mut app = shell.lock(&mut app);
    write_all(&mut shell, &mut app, &[("S1", false)]);
    shell.plug(&mut app, "S2");
    shell.flow(&mut app, F::Hold);
    // Alpha's copy is not one of Beta's: Beta's seal went beside it in the
    // Outbox, so it is written to the stick beside Alpha's too.
    assert_eq!(app.sheet, None);
    shell.pull(&mut app, "S2");
    assert_eq!(step(&app), Step::Done);
    // Beta was written to both sticks; Alpha's copy stays as it was.
    assert_eq!(shell.disk("S2").file("vault.kdbx"), Some(&a));
    assert_eq!(
        shell
            .disk("S2")
            .files
            .iter()
            .filter(|f| f.0.ends_with(".kdbx"))
            .count(),
        2,
        "Beta's copy is beside Alpha's"
    );
    assert!(
        shell
            .disk("S2")
            .file("vault-2.kdbx")
            .is_some_and(|f| f != &a),
        "the file beside Alpha's is not Alpha's"
    );
}

#[test]
fn a_stick_whose_vault_file_the_free_memory_does_not_hold_is_not_read_and_says_what_it_needs() {
    let mut shell = Shell::new();
    let a = vault("Alpha", PHRASE, 0x61);
    let mut s1 = Disk::empty("S1", "ONE");
    s1.serial = Some(1);
    s1.files = vec![("vault.kdbx".into(), a.clone())];
    shell.add(s1);
    let mut app = process(Vec::new(), vec![("vault.kdbx".into(), a)], Vec::new());
    keep_time(&mut app);
    start(&mut app, Kind::Gpg);
    shell.flow(&mut app, F::VaultExisting(0));
    through_where(&mut shell, &mut app);
    app.storage(faraday_core::StorageEvent::Memory { available_mib: 0 });
    shell.plug_known(&mut app, "S1");
    assert!(
        has(&mut app, "Needs 1 MiB. This computer has 0 MiB free."),
        "{:?}",
        app.drawn_texts()
    );
    assert!(
        app.flow()
            .unwrap()
            .live
            .reading
            .as_ref()
            .unwrap()
            .files
            .is_empty()
    );
}

#[test]
fn two_vaults_never_opened_stay_in_files_when_one_is_picked_from_a_stick() {
    let mut shell = Shell::new();
    let x = vault("Xray", PHRASE, 0x63);
    let y = vault("Yankee", PHRASE, 0x64);
    let w = vault("Whiskey", PHRASE_2, 0x65);
    let mut s1 = Disk::empty("S1", "ONE");
    s1.serial = Some(1);
    s1.files = vec![("main.kdbx".into(), w)];
    shell.add(s1);
    let mut app = process_unseen(
        vec![("x.kdbx".into(), x), ("y.kdbx".into(), y)],
        Vec::new(),
        Vec::new(),
    );
    keep_time(&mut app);
    start(&mut app, Kind::Gpg);
    shell.flow(&mut app, F::VaultExisting(0));
    through_where(&mut shell, &mut app);
    shell.plug(&mut app, "S1");
    pick_if_asked(&mut shell, &mut app, "main.kdbx");
    shell.flow(&mut app, F::Tap);
    shell.pull(&mut app, "S1");
    let mut names: Vec<&str> = app
        .inbox()
        .iter()
        .filter(|i| i.name.ends_with(".kdbx"))
        .map(|i| i.name.as_str())
        .collect();
    names.sort_unstable();
    assert_eq!(names, ["main.kdbx", "x.kdbx", "y.kdbx"]);
}

#[test]
fn a_kdbx_that_is_not_a_vault_is_no_vault_for_the_place() {
    let mut shell = Shell::new();
    let mut s1 = Disk::empty("S1", "ONE");
    s1.serial = Some(1);
    s1.files = vec![("notes.kdbx".into(), b"not a database at all".to_vec())];
    shell.add(s1);
    let mut app = started(Kind::Gpg);
    shell.flow(&mut app, F::VaultExisting(0));
    through_where(&mut shell, &mut app);
    shell.plug(&mut app, "S1");
    expect(&mut app, "No vault for Vault 1 on this stick");
}

/// The flow of `kind`, started on a machine that has read nothing yet.
fn fresh(shell: &mut Shell, kind: Kind) -> Faraday {
    let mut app = shell.power_off();
    start(&mut app, kind);
    assert_eq!(step(&app), Step::Where);
    app
}

/// Make the key from a vault opened with `phrase`, lock, and write.
fn finish(shell: &mut Shell, app: &mut Faraday, phrase: &str, sticks: &[(&str, bool)]) -> Faraday {
    shell.flow(app, F::UnlockVault(0));
    unlock(app, phrase);
    assert_eq!(step(app), Step::Make, "{:?}", unlock_error(app));
    make_key(shell, app, &GPG_FIELDS);
    shell.flow(app, F::MadeContinue);
    let mut app = shell.lock(app);
    write_all(shell, &mut app, sticks);
    assert_eq!(step(&app), Step::Done);
    app
}

#[test]
fn an_existing_place_takes_the_one_vault_on_a_stick_nobody_has_opened() {
    let mut shell = Shell::new();
    let main = vault("Main", PHRASE, 0x71);
    let mut s1 = Disk::empty("S1", "ONE");
    s1.serial = Some(1);
    s1.files = vec![("vault.kdbx".into(), main.clone())];
    let mut s2 = Disk::empty("S2", "TWO");
    s2.serial = Some(2);
    shell.add(s1);
    shell.add(s2);
    let mut app = fresh(&mut shell, Kind::Gpg);
    shell.flow(&mut app, F::VaultExisting(0));
    through_where(&mut shell, &mut app);
    shell.plug(&mut app, "S1");
    assert!(
        !has(&mut app, "No vault for Vault 1"),
        "{:?}",
        texts(&mut app)
    );
    assert!(!has(&mut app, "Which vault is Vault 1?"));
    shell.flow(&mut app, F::Tap);
    shell.pull(&mut app, "S1");
    bring_in(&mut shell, &mut app, "S2", false);
    let app = finish(&mut shell, &mut app, PHRASE, &[("S1", false), ("S2", true)]);
    let written = shell.disk("S1").file("vault.kdbx").cloned().unwrap();
    assert_ne!(written, main, "replaced");
    assert!(shell.disk("S1").file("vault-2.kdbx").is_none());
    assert_eq!(shell.disk("S2").file("vault.kdbx"), Some(&written));
    drop(app);
}

#[test]
fn two_vaults_on_a_stick_nobody_has_opened_ask_and_leave_the_other_as_it_was() {
    let mut shell = Shell::new();
    let main = vault("Main", PHRASE, 0x72);
    let other = vault("Other", PHRASE_2, 0x73);
    let mut s1 = Disk::empty("S1", "ONE");
    s1.serial = Some(1);
    s1.files = vec![
        ("main.kdbx".into(), main.clone()),
        ("other.kdbx".into(), other.clone()),
    ];
    let mut s2 = Disk::empty("S2", "TWO");
    s2.serial = Some(2);
    shell.add(s1);
    shell.add(s2);
    let mut app = fresh(&mut shell, Kind::Gpg);
    shell.flow(&mut app, F::VaultExisting(0));
    through_where(&mut shell, &mut app);
    shell.plug(&mut app, "S1");
    expect(&mut app, "Which vault is Vault 1?");
    pick_if_asked(&mut shell, &mut app, "main.kdbx");
    shell.flow(&mut app, F::Tap);
    shell.pull(&mut app, "S1");
    bring_in(&mut shell, &mut app, "S2", false);
    let vaults: Vec<&str> = app
        .inbox()
        .iter()
        .filter(|i| i.name.ends_with(".kdbx"))
        .map(|i| i.name.as_str())
        .collect();
    assert_eq!(vaults, ["main.kdbx"], "only the chosen one is taken in");
    let _app = finish(&mut shell, &mut app, PHRASE, &[("S1", false), ("S2", true)]);
    assert_eq!(shell.disk("S1").file("other.kdbx"), Some(&other));
    assert_ne!(shell.disk("S1").file("main.kdbx"), Some(&main));
}

#[test]
fn a_new_vault_place_offers_a_vault_beside_a_stick_s_vault_and_a_row_makes_it_existing() {
    let mut shell = Shell::new();
    let main = vault("Main", PHRASE, 0x74);
    let mut s1 = Disk::empty("S1", "ONE");
    s1.serial = Some(1);
    s1.files = vec![("vault-1.kdbx".into(), main.clone())];
    let mut s2 = Disk::empty("S2", "TWO");
    s2.serial = Some(2);
    shell.add(s1);
    shell.add(s2);
    let mut app = fresh(&mut shell, Kind::Gpg);
    through_where(&mut shell, &mut app);
    shell.plug(&mut app, "S1");
    expect(&mut app, "Which vault is Vault 1?");
    expect(&mut app, "A new vault beside them");
    shell.flow(&mut app, F::PickBeside(0));
    shell.flow(&mut app, F::Tap);
    shell.pull(&mut app, "S1");
    assert!(!app.flow().unwrap().plan.vaults[0].existing);
    assert!(app.inbox().iter().all(|i| !i.name.ends_with(".kdbx")));
    bring_in(&mut shell, &mut app, "S2", false);
    create_vault(&mut shell, &mut app, 0, PHRASE_2);
    make_key(&mut shell, &mut app, &GPG_FIELDS);
    shell.flow(&mut app, F::MadeContinue);
    let mut app = shell.lock(&mut app);
    write_all(&mut shell, &mut app, &[("S1", false), ("S2", true)]);
    assert_eq!(shell.disk("S1").file("vault-1.kdbx"), Some(&main));
    // The row instead: the vault is existing and the file is Files'.
    let mut shell = Shell::new();
    let mut s1 = Disk::empty("S1", "ONE");
    s1.serial = Some(1);
    s1.files = vec![("vault-1.kdbx".into(), main.clone())];
    shell.add(s1);
    let mut app = fresh(&mut shell, Kind::Gpg);
    through_where(&mut shell, &mut app);
    shell.plug(&mut app, "S1");
    shell.flow(&mut app, F::PickFile(0, 0));
    shell.flow(&mut app, F::Tap);
    let vp = &app.flow().unwrap().plan.vaults[0];
    assert!(vp.existing);
    assert_eq!(vp.file.as_deref(), Some("vault-1.kdbx"));
}

#[test]
fn two_sticks_with_the_same_vault_bytes_ask_nothing_of_the_second_and_both_are_replaced() {
    let mut shell = Shell::new();
    let main = vault("Main", PHRASE, 0x75);
    for (id, label, n) in [("S1", "ONE", 1), ("S2", "TWO", 2)] {
        let mut d = Disk::empty(id, label);
        d.serial = Some(n);
        d.files = vec![("vault.kdbx".into(), main.clone())];
        shell.add(d);
    }
    let mut app = fresh(&mut shell, Kind::Gpg);
    shell.flow(&mut app, F::VaultExisting(0));
    through_where(&mut shell, &mut app);
    shell.plug(&mut app, "S1");
    shell.flow(&mut app, F::Tap);
    shell.pull(&mut app, "S1");
    shell.plug(&mut app, "S2");
    assert!(
        !has(&mut app, "Which vault is Vault 1?"),
        "{:?}",
        texts(&mut app)
    );
    assert!(offered(&mut app, Action::Flow(F::Tap)));
    shell.flow(&mut app, F::Tap);
    shell.pull(&mut app, "S2");
    let _app = finish(&mut shell, &mut app, PHRASE, &[("S1", false), ("S2", true)]);
    let a = shell.disk("S1").file("vault.kdbx").cloned().unwrap();
    assert_ne!(a, main);
    assert_eq!(shell.disk("S2").file("vault.kdbx"), Some(&a));
}

#[test]
fn a_vault_chosen_from_an_unread_stick_holds_the_key_and_its_neighbour_stays_out() {
    let mut shell = Shell::new();
    let app = crate::backup_flow_wallet::two_of_three_ready(&mut shell, false);
    // Each of the three vaults holds the one key the plan puts in it.
    let held: Vec<usize> = app
        .vaults()
        .open
        .iter()
        .map(|o| {
            app.vaults()
                .store()
                .contents_for_tests(o.id)
                .of(faraday_vault::records::kind::KEY)
                .count()
        })
        .collect();
    assert_eq!(held, [1, 1, 1]);
    // The other vault on the stick was never taken into Files.
    assert!(app.inbox().iter().all(|i| i.name != "main-2.kdbx"));
}

fn kdbx_count(shell: &Shell, disk: &str) -> usize {
    shell
        .disk(disk)
        .files
        .iter()
        .filter(|f| f.0.ends_with(".kdbx"))
        .count()
}

/// A vault `Main` that `phrase` opens, holding one record so that its
/// bytes differ from the same vault made without.
fn main_with_note(phrase: &str, id: u8) -> Vec<u8> {
    use faraday_vault::records::{field, kind};
    vault_with(
        "Main",
        phrase,
        id,
        vec![faraday_vault::Record::new(kind::WALLET).with(field::WALLET, b"a description")],
    )
}

/// Vault 1 known from S1 (seen this session); S2 holds `other_file`,
/// which nobody has opened; the person presses it for Vault 1.
fn press_unseen_file(shell: &mut Shell, other_file: &[u8]) -> Faraday {
    let main = vault("Main", PHRASE, 0x81);
    let mut s1 = Disk::empty("S1", "ONE");
    s1.serial = Some(1);
    s1.files = vec![("a.kdbx".into(), main)];
    let mut s2 = Disk::empty("S2", "TWO");
    s2.serial = Some(2);
    s2.files = vec![("b.kdbx".into(), other_file.to_vec())];
    shell.add(s1);
    shell.add(s2);
    let mut app = fresh(shell, Kind::Gpg);
    shell.flow(&mut app, F::VaultExisting(0));
    through_where(shell, &mut app);
    shell.plug_known(&mut app, "S1");
    shell.flow(&mut app, F::Tap);
    shell.pull(&mut app, "S1");
    shell.plug(&mut app, "S2");
    assert!(offered(&mut app, Action::Flow(F::Tap)));
    let k = app
        .flow()
        .unwrap()
        .live
        .reading
        .as_ref()
        .unwrap()
        .files
        .iter()
        .position(|(f, _)| f.name == "b.kdbx")
        .expect("the file is listed");
    shell.flow(&mut app, F::PickFile(0, k));
    shell.flow(&mut app, F::Tap);
    shell.pull(&mut app, "S2");
    assert_eq!(step(&app), Step::Vaults);
    app
}

#[test]
fn a_file_pressed_for_a_known_vault_that_is_another_vault_says_so_and_stays_as_it_was() {
    let mut shell = Shell::new();
    let other = vault("Other", PHRASE_2, 0x82);
    let mut app = press_unseen_file(&mut shell, &other);
    shell.flow(&mut app, F::UnlockVault(0));
    unlock(&mut app, PHRASE_2);
    assert_eq!(step(&app), Step::Vaults);
    expect(&mut app, "b.kdbx is not Vault 1");
    assert!(app.vaults().open.is_empty(), "the other vault is closed");
    // The known copy opens as before.
    shell.flow(&mut app, F::UnlockVault(0));
    unlock(&mut app, PHRASE);
    assert_eq!(step(&app), Step::Make, "{:?}", unlock_error(&app));
    make_key(&mut shell, &mut app, &GPG_FIELDS);
    shell.flow(&mut app, F::MadeContinue);
    let mut app = shell.lock(&mut app);
    write_all(&mut shell, &mut app, &[("S1", false), ("S2", true)]);
    assert_eq!(step(&app), Step::Done);
    assert_eq!(shell.disk("S2").file("b.kdbx"), Some(&other));
    assert_eq!(kdbx_count(&shell, "S2"), 2, "Vault 1 is beside it");
}

#[test]
fn a_vault_closed_for_not_being_the_one_asked_for_leaves_the_nonce_history_at_once() {
    let mut shell = Shell::new();
    let other = vault("Other", PHRASE_2, 0x82);
    let mut app = press_unseen_file(&mut shell, &other);
    faraday_core::testkit::nonce_history_on(&mut app, true);
    assert!(app.nonce_history_kept());
    shell.flow(&mut app, F::UnlockVault(0));
    // The device's path: the unlock runs on the worker and its result
    // arrives on a tick, with no press after it.
    app.press(Action::Vault(V::FocusPassphrase));
    type_text(&mut app, PHRASE_2);
    faraday_core::testkit::press_on_worker(&mut app, Action::Vault(V::Unlock));
    let mut closed = false;
    for t in 100..140u64 {
        let _ = app.frame();
        app.event(faraday_shell_api::Event::Tick { now_ms: t * 1000 });
        // Waits for the worker's answer and takes it as a tick does,
        // without the settling a press brings.
        faraday_core::testkit::settle(&mut app);
        if has_part(&mut app, "b.kdbx is not Vault 1") {
            closed = true;
            break;
        }
    }
    assert!(closed, "the other vault is refused");
    assert!(app.vaults().open.is_empty(), "the other vault is closed");
    assert_eq!(faraday_core::testkit::nonce_history_vaults(&app), None);
}

#[test]
fn a_file_pressed_for_a_known_vault_that_is_an_older_copy_of_it_opens_and_is_replaced() {
    let mut shell = Shell::new();
    let older = main_with_note(PHRASE, 0x81);
    let mut app = press_unseen_file(&mut shell, &older);
    shell.flow(&mut app, F::UnlockVault(0));
    unlock(&mut app, PHRASE);
    assert_eq!(step(&app), Step::Make, "{:?}", unlock_error(&app));
    assert!(!has(&mut app, "b.kdbx is not Vault 1"));
    make_key(&mut shell, &mut app, &GPG_FIELDS);
    shell.flow(&mut app, F::MadeContinue);
    let mut app = shell.lock(&mut app);
    write_all(&mut shell, &mut app, &[("S1", false), ("S2", true)]);
    assert_eq!(step(&app), Step::Done);
    assert_ne!(shell.disk("S2").file("b.kdbx"), Some(&older), "replaced");
    assert_eq!(kdbx_count(&shell, "S2"), 1);
}

/// Two vaults, both existing: Vault 1 on S1 and S2, Vault 2 on S3, whose
/// file is another copy of Vault 1's, nobody having opened either.
fn two_vaults_one_file_each() -> (Shell, Faraday) {
    let mut shell = Shell::new();
    let a = vault("Main", PHRASE, 0x83);
    let b = main_with_note(PHRASE, 0x83);
    for (id, label, n, name, bytes) in [
        ("S1", "ONE", 1, "a.kdbx", &a),
        ("S2", "TWO", 2, "a.kdbx", &a),
        ("S3", "THREE", 3, "b.kdbx", &b),
    ] {
        let mut d = Disk::empty(id, label);
        d.serial = Some(n);
        d.files = vec![(name.into(), bytes.clone())];
        shell.add(d);
    }
    let mut app = fresh(&mut shell, Kind::Gpg);
    shell.flow(&mut app, F::VaultExisting(0));
    shell.flow(&mut app, F::AddPlace);
    shell.flow(&mut app, F::PlaceVault(2, 1));
    shell.flow(&mut app, F::VaultExisting(1));
    through_where(&mut shell, &mut app);
    for d in ["S1", "S2", "S3"] {
        bring_in(&mut shell, &mut app, d, false);
    }
    assert_eq!(step(&app), Step::Vaults);
    (shell, app)
}

#[test]
fn two_vaults_chosen_from_two_copies_of_one_vault_refuse_the_second_and_write_nothing() {
    let (mut shell, mut app) = two_vaults_one_file_each();
    shell.flow(&mut app, F::UnlockVault(0));
    unlock(&mut app, PHRASE);
    assert_eq!(step(&app), Step::Vaults);
    shell.flow(&mut app, F::UnlockVault(1));
    unlock(&mut app, PHRASE);
    assert_eq!(step(&app), Step::Vaults, "stays on step 3");
    assert_eq!(app.vaults().open.len(), 1);
    assert!(unlock_error(&app).is_some(), "the row says why");
    assert_eq!(shell.vault_writes(), 0);
}

#[test]
fn a_lock_between_two_copies_of_one_vault_makes_the_second_say_it_is_not_that_vault() {
    let (mut shell, mut app) = two_vaults_one_file_each();
    shell.flow(&mut app, F::UnlockVault(0));
    unlock(&mut app, PHRASE);
    // The new process has seen only what it opened: b.kdbx is a copy
    // nobody has opened.
    let mut app = shell.lock_unseen(&mut app);
    assert_eq!(step(&app), Step::Vaults);
    shell.flow(&mut app, F::UnlockVault(1));
    unlock(&mut app, PHRASE);
    assert_eq!(step(&app), Step::Vaults, "stays on step 3");
    expect(&mut app, "b.kdbx is not Vault 2");
    assert_eq!(app.flow().unwrap().plan.vaults[1].salt, None, "not bound");
    assert!(app.vaults().open.is_empty(), "the file chosen is closed");
    // The panel offers both answers.
    expect(&mut app, "A new vault beside it");
    assert_eq!(shell.vault_writes(), 0);
}

#[test]
fn a_restart_between_the_sticks_and_the_unlock_keeps_the_open_row_and_the_vault_opens() {
    let mut shell = Shell::new();
    let main = vault("Main", PHRASE, 0x84);
    let mut s1 = Disk::empty("S1", "ONE");
    s1.serial = Some(1);
    s1.files = vec![("vault.kdbx".into(), main)];
    shell.add(s1);
    shell.add(Disk::empty("S2", "TWO"));
    let mut app = fresh(&mut shell, Kind::Gpg);
    shell.flow(&mut app, F::VaultExisting(0));
    through_where(&mut shell, &mut app);
    bring_in(&mut shell, &mut app, "S1", false);
    bring_in(&mut shell, &mut app, "S2", false);
    assert_eq!(step(&app), Step::Vaults);
    let mut app = shell.lock(&mut app);
    assert_eq!(step(&app), Step::Vaults, "{:?}", texts(&mut app));
    expect(&mut app, "Open Vault 1");
    shell.flow(&mut app, F::UnlockVault(0));
    unlock(&mut app, PHRASE);
    assert_eq!(step(&app), Step::Make, "{:?}", unlock_error(&app));
    assert!(app.flow().unwrap().plan.vaults[0].salt.is_some());
}

#[test]
fn without_its_passphrase_a_chosen_vault_can_be_made_new_beside_it_or_moved_to_another_stick() {
    for beside in [true, false] {
        let mut shell = Shell::new();
        let main = vault("Main", PHRASE, 0x85);
        let mut s1 = Disk::empty("S1", "ONE");
        s1.serial = Some(1);
        s1.files = vec![("vault.kdbx".into(), main)];
        shell.add(s1);
        shell.add(Disk::empty("S2", "TWO"));
        let mut app = fresh(&mut shell, Kind::Gpg);
        shell.flow(&mut app, F::VaultExisting(0));
        through_where(&mut shell, &mut app);
        bring_in(&mut shell, &mut app, "S1", false);
        bring_in(&mut shell, &mut app, "S2", false);
        assert_eq!(step(&app), Step::Vaults);
        shell.flow(&mut app, F::UnlockVault(0));
        shell.flow(&mut app, F::NoPass(0));
        expect(&mut app, "A new vault beside it");
        expect(&mut app, "Use another stick");
        if beside {
            shell.flow(&mut app, F::BesideIt(0));
            assert!(has(&mut app, "Create Vault 1"), "{:?}", texts(&mut app));
            assert!(!app.flow().unwrap().plan.vaults[0].found());
        } else {
            shell.flow(&mut app, F::AnotherStick(0));
            assert_eq!(step(&app), Step::Sticks);
            assert!(!app.flow().unwrap().plan.vaults[0].found());
        }
    }
}

/// Vault 1 known from S1 (seen this session), its place on S2 too; S2
/// holds `other` as `b.kdbx`, which nobody has opened. S1 is read and
/// S2 is plugged, not yet tapped.
fn known_with_unseen_second(shell: &mut Shell, other: &[u8], seen: bool) -> Faraday {
    let main = vault("Main", PHRASE, 0x81);
    let mut s1 = Disk::empty("S1", "ONE");
    s1.serial = Some(1);
    s1.files = vec![("a.kdbx".into(), main)];
    let mut s2 = Disk::empty("S2", "TWO");
    s2.serial = Some(2);
    s2.files = vec![("b.kdbx".into(), other.to_vec())];
    shell.add(s1);
    shell.add(s2);
    let mut app = fresh(shell, Kind::Gpg);
    shell.flow(&mut app, F::VaultExisting(0));
    through_where(shell, &mut app);
    if seen {
        shell.plug_known(&mut app, "S1");
    } else {
        shell.plug(&mut app, "S1");
    }
    shell.flow(&mut app, F::Tap);
    shell.pull(&mut app, "S1");
    shell.plug(&mut app, "S2");
    app
}

#[test]
fn a_known_vault_and_a_file_nobody_has_opened_goes_beside_it_and_says_so() {
    let mut shell = Shell::new();
    let other = vault("Other", PHRASE_2, 0x82);
    let mut app = known_with_unseen_second(&mut shell, &other, true);
    assert!(offered(&mut app, Action::Flow(F::Tap)), "at once");
    expect(&mut app, "Vault 1 goes beside these");
    assert!(has_part(&mut app, "b.kdbx"));
    assert!(has(&mut app, "Use b.kdbx for Vault 1"));
    shell.flow(&mut app, F::Tap);
    shell.pull(&mut app, "S2");
    assert_eq!(step(&app), Step::Vaults);
    finish(&mut shell, &mut app, PHRASE, &[("S1", false), ("S2", true)]);
    assert_eq!(shell.disk("S2").file("b.kdbx"), Some(&other), "untouched");
    assert_eq!(kdbx_count(&shell, "S2"), 2, "Vault 1 is beside it");
}

#[test]
fn a_second_stick_with_other_bytes_goes_beside_and_its_file_is_untouched() {
    let mut shell = Shell::new();
    let older = main_with_note(PHRASE, 0x81);
    let mut app = known_with_unseen_second(&mut shell, &older, false);
    expect(&mut app, "Vault 1 goes beside these");
    shell.flow(&mut app, F::Tap);
    shell.pull(&mut app, "S2");
    finish(&mut shell, &mut app, PHRASE, &[("S1", false), ("S2", true)]);
    assert_eq!(shell.disk("S2").file("b.kdbx"), Some(&older));
    assert_eq!(kdbx_count(&shell, "S2"), 2);
}

#[test]
fn pressing_the_second_stick_s_file_makes_the_first_stick_ask_a_hold_and_go_beside() {
    let mut shell = Shell::new();
    let older = main_with_note(PHRASE, 0x81);
    let mut app = known_with_unseen_second(&mut shell, &older, false);
    let k = app
        .flow()
        .unwrap()
        .live
        .reading
        .as_ref()
        .unwrap()
        .files
        .iter()
        .position(|(f, _)| f.name == "b.kdbx")
        .unwrap();
    shell.flow(&mut app, F::PickFile(0, k));
    shell.flow(&mut app, F::Tap);
    shell.pull(&mut app, "S2");
    let main = shell.disk("S1").file("a.kdbx").cloned().unwrap();
    shell.flow(&mut app, F::UnlockVault(0));
    unlock(&mut app, PHRASE);
    assert_eq!(step(&app), Step::Make, "{:?}", unlock_error(&app));
    make_key(&mut shell, &mut app, &GPG_FIELDS);
    shell.flow(&mut app, F::MadeContinue);
    let mut app = shell.lock(&mut app);
    shell.plug(&mut app, "S1");
    assert!(offered(&mut app, Action::Flow(F::Hold)), "S1 asks a hold");
    shell.flow(&mut app, F::Hold);
    shell.pull(&mut app, "S1");
    assert_eq!(shell.disk("S1").file("a.kdbx"), Some(&main));
    assert_eq!(kdbx_count(&shell, "S1"), 2, "beside");
    write_all(&mut shell, &mut app, &[("S2", true)]);
    assert_ne!(shell.disk("S2").file("b.kdbx"), Some(&older), "replaced");
}

/// An existing vault on S1 and S2 (one file each), read and locked with
/// the key made; on step 6.
fn existing_two_locked(shell: &mut Shell) -> Faraday {
    let main = vault("Main", PHRASE, 0x71);
    for (id, label, n) in [("S1", "ONE", 1), ("S2", "TWO", 2)] {
        let mut d = Disk::empty(id, label);
        d.serial = Some(n);
        d.files = vec![("vault.kdbx".into(), main.clone())];
        shell.add(d);
    }
    let mut app = fresh(shell, Kind::Gpg);
    shell.flow(&mut app, F::VaultExisting(0));
    through_where(shell, &mut app);
    bring_in(shell, &mut app, "S1", false);
    bring_in(shell, &mut app, "S2", false);
    shell.flow(&mut app, F::UnlockVault(0));
    unlock(&mut app, PHRASE);
    assert_eq!(step(&app), Step::Make, "{:?}", unlock_error(&app));
    make_key(shell, &mut app, &GPG_FIELDS);
    shell.flow(&mut app, F::MadeContinue);
    shell.lock(&mut app)
}

#[test]
fn a_stick_changed_after_it_was_read_is_refused_whatever_replaced_its_vault_file() {
    for replacement in [vault("Other", PHRASE_2, 0x72), b"not a vault".to_vec()] {
        let mut shell = Shell::new();
        let mut app = existing_two_locked(&mut shell);
        shell.disk_mut("S1").files = vec![("vault.kdbx".into(), replacement)];
        shell.plug(&mut app, "S1");
        let _ = app.frame();
        assert!(
            has_part(&mut app, "Not Stick 2, read for"),
            "{:?}",
            texts(&mut app)
        );
        assert_eq!(shell.vault_writes(), 0);
    }
}

#[test]
fn a_stick_left_as_it_was_after_the_read_is_written_without_a_refusal() {
    let mut shell = Shell::new();
    let mut app = existing_two_locked(&mut shell);
    write_all(&mut shell, &mut app, &[("S1", false), ("S2", true)]);
    assert_eq!(step(&app), Step::Done);
    assert_eq!(kdbx_count(&shell, "S1"), 1);
}

#[test]
fn a_stick_named_at_step_six_with_a_file_nobody_opened_asks_nothing_and_goes_beside() {
    let mut shell = Shell::new();
    let mut app = existing_two_locked(&mut shell);
    let other = vault("Other", PHRASE_2, 0x73);
    let mut u = Disk::empty("U", "OTHER");
    u.serial = Some(9);
    u.files = vec![("other.kdbx".into(), other.clone())];
    shell.add(u);
    shell.plug(&mut app, "U");
    shell.flow(&mut app, F::UseAnother);
    shell.pull(&mut app, "U");
    shell.plug(&mut app, "U");
    assert!(!offered(&mut app, Action::Flow(F::Hold)), "no hold");
    assert!(!has_part(&mut app, "Use other.kdbx"), "no press");
    assert!(!has(&mut app, "Which vault is Vault 1?"));
    shell.flow(&mut app, F::Tap);
    assert_eq!(shell.disk("U").file("other.kdbx"), Some(&other));
    assert_eq!(kdbx_count(&shell, "U"), 2, "beside");
}

// ---- a file tried for a known vault, and the files beside it ----

use faraday_core::backup_flow::Pick;

#[test]
fn opening_vault_one_after_a_line_under_vault_two_leaves_vault_one_open() {
    let (mut shell, mut app) = two_vaults_one_file_each();
    shell.flow(&mut app, F::UnlockVault(0));
    unlock(&mut app, PHRASE);
    let mut app = shell.lock(&mut app);
    shell.flow(&mut app, F::UnlockVault(1));
    unlock(&mut app, PHRASE);
    expect(&mut app, "b.kdbx is not Vault 2");
    // Vault 1 is opened again while the line stands.
    shell.flow(&mut app, F::UnlockVault(0));
    unlock(&mut app, PHRASE);
    assert_eq!(app.vaults().open.len(), 1, "Vault 1 stays open");
}

#[test]
fn a_failed_press_gives_up_only_the_pressed_sticks_pick() {
    let mut shell = Shell::new();
    let other = vault("Other", PHRASE_2, 0x82);
    let mut app = press_unseen_file(&mut shell, &other);
    shell.flow(&mut app, F::UnlockVault(0));
    unlock(&mut app, PHRASE_2);
    expect(&mut app, "b.kdbx is not Vault 1");
    let f = app.flow().unwrap();
    let picks = |s: usize| f.slots[s].bound.as_ref().unwrap().picks.clone();
    assert!(
        picks(0)
            .iter()
            .any(|(v, p)| *v == 0 && *p == Pick::Over("a.kdbx".into())),
        "Stick 1 still replaces its copy: {:?}",
        picks(0)
    );
    assert!(
        picks(1)
            .iter()
            .all(|(v, p)| *v != 0 || matches!(p, Pick::New)),
        "Stick 2 goes beside: {:?}",
        picks(1)
    );
}

/// Vault 1 opened and sealed in a first process (so that its seal count is
/// known), S1 holding its first copy, S2 an older copy under `b.kdbx`;
/// the person presses `b.kdbx` for Vault 1 and opens it: the older-copy
/// dialog shows.
fn older_copy_pressed() -> (Shell, Faraday) {
    let mut shell = Shell::new();
    let main = vault("Main", PHRASE, 0x91);
    let older = main_with_note(PHRASE, 0x91);
    for (id, label, n, name, bytes) in [
        ("S1", "ONE", 1, "a.kdbx", &main),
        ("S2", "TWO", 2, "b.kdbx", &older),
    ] {
        let mut d = Disk::empty(id, label);
        d.serial = Some(n);
        d.files = vec![(name.into(), bytes.clone())];
        shell.add(d);
    }
    shell.boxes.0 = vec![("main.kdbx".into(), main)];
    let mut app = shell.restart();
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Open(0)));
    unlock(&mut app, PHRASE);
    assert_eq!(app.vaults().open.len(), 1);
    // A change saved, so that the lock seals it once more.
    app.press(Action::Entry(None));
    type_text(
        &mut app,
        &faraday_core::testkit::test_words(faraday_core::testkit::TEST_SEEDS[1].0),
    );
    app.press(Action::EntryAdd);
    app.press(Action::Vault(V::SaveKey(0)));
    top_up(&mut app);
    let _ = shell.lock(&mut app);
    // What the process kept is the seal count; no vault file is left in
    // Files, so that S1's copy and S2's are the files the flow knows.
    shell.boxes.0.retain(|f| !f.0.ends_with(".kdbx"));
    shell.boxes.1.retain(|f| !f.0.ends_with(".kdbx"));
    let mut app = shell.restart();
    start(&mut app, Kind::Gpg);
    shell.flow(&mut app, F::VaultExisting(0));
    through_where(&mut shell, &mut app);
    shell.plug_known(&mut app, "S1");
    shell.flow(&mut app, F::Tap);
    shell.pull(&mut app, "S1");
    assert!(
        app.flow().unwrap().plan.vaults[0].salt.is_some(),
        "known: {:?} {:?}",
        app.vaults()
            .summaries
            .iter()
            .map(|s| (s.file.clone(), s.copies.len(), s.name.clone()))
            .collect::<Vec<_>>(),
        app.flow().unwrap().live.reading.as_ref().map(|r| (
            r.picks.clone(),
            r.differs.clone(),
            r.problem.clone()
        )),
    );
    shell.plug(&mut app, "S2");
    let k = app
        .flow()
        .unwrap()
        .live
        .reading
        .as_ref()
        .unwrap()
        .files
        .iter()
        .position(|(f, _)| f.name == "b.kdbx")
        .expect("the file is listed");
    shell.flow(&mut app, F::PickFile(0, k));
    shell.flow(&mut app, F::Tap);
    shell.pull(&mut app, "S2");
    assert_eq!(step(&app), Step::Vaults);
    shell.flow(&mut app, F::UnlockVault(0));
    unlock(&mut app, PHRASE);
    (shell, app)
}

#[test]
fn an_older_copy_pressed_for_a_known_vault_asks_and_keep_locked_binds_nothing() {
    let (_shell, mut app) = older_copy_pressed();
    if app.sheet != Some(Sheet::OlderCopy) {
        panic!("no dialog: {:?}", texts(&mut app));
    }
    expect(&mut app, "Open this copy?");
    app.press(Action::Vault(V::OlderBack));
    let vp = app.flow().unwrap().plan.vaults[0].clone();
    assert_ne!(vp.file.as_deref(), Some("b.kdbx"), "not bound");
    assert!(vp.salt.is_some(), "the vault stays known");
    assert_eq!(vp.trying, None, "the file tried is given up");
    assert!(app.vaults().open.is_empty());
}

#[test]
fn an_older_copy_pressed_for_a_known_vault_is_bound_on_open() {
    let (_shell, mut app) = older_copy_pressed();
    assert_eq!(app.sheet, Some(Sheet::OlderCopy));
    app.press(Action::Vault(V::OlderOpen));
    assert_eq!(step(&app), Step::Make, "{:?}", unlock_error(&app));
    let vp = app.flow().unwrap().plan.vaults[0].clone();
    assert_eq!(vp.file.as_deref(), Some("b.kdbx"));
    assert_eq!(vp.trying, None);
}

#[test]
fn without_its_passphrase_a_tried_file_is_given_up_and_the_plan_vault_stays() {
    let mut shell = Shell::new();
    let older = main_with_note(PHRASE, 0x81);
    let mut app = press_unseen_file(&mut shell, &older);
    assert!(app.flow().unwrap().plan.vaults[0].trying.is_some());
    assert!(offered(&mut app, Action::Flow(F::UnlockVault(0))));
    shell.flow(&mut app, F::UnlockVault(0));
    assert!(offered(&mut app, Action::Flow(F::NoPass(0))));
    shell.flow(&mut app, F::NoPass(0));
    let vp = app.flow().unwrap().plan.vaults[0].clone();
    assert_eq!(vp.trying, None);
    assert!(vp.salt.is_some() && vp.file.is_some(), "still the plan's");
    // Only what is drawn is pressed: the row offers Open again.
    assert!(offered(&mut app, Action::Flow(F::UnlockVault(0))));
    shell.flow(&mut app, F::UnlockVault(0));
    unlock(&mut app, PHRASE);
    assert_eq!(step(&app), Step::Make, "{:?}", unlock_error(&app));
    assert_eq!(
        app.flow().unwrap().plan.vaults[0].file.as_deref(),
        Some("a.kdbx")
    );
}

#[test]
fn without_its_passphrase_the_pressed_stick_goes_back_to_a_new_vault_beside_its_file() {
    let mut shell = Shell::new();
    let older = main_with_note(PHRASE, 0x81);
    let mut app = press_unseen_file(&mut shell, &older);
    shell.flow(&mut app, F::UnlockVault(0));
    shell.flow(&mut app, F::NoPass(0));
    let f = app.flow().unwrap();
    let picks = f.slots[1].bound.as_ref().unwrap().picks.clone();
    assert!(
        picks.iter().all(|(v, p)| *v != 0 || matches!(p, Pick::New)),
        "Stick 2 goes beside: {picks:?}"
    );
}

#[test]
fn a_file_known_as_a_vault_outside_the_plan_gets_no_press_and_its_newer_copy_stays() {
    let mut shell = Shell::new();
    let main = vault("Main", PHRASE, 0x81);
    let w_new = main_with_note(PHRASE_2, 0x95);
    let w_old = vault("Other", PHRASE_2, 0x95);
    let mut s1 = Disk::empty("S1", "ONE");
    s1.serial = Some(1);
    s1.files = vec![("a.kdbx".into(), main)];
    let mut s2 = Disk::empty("S2", "TWO");
    s2.serial = Some(2);
    s2.files = vec![("w2.kdbx".into(), w_old)];
    shell.add(s1);
    shell.add(s2);
    let mut app = process(
        vec![("w.kdbx".into(), w_new.clone())],
        Vec::new(),
        Vec::new(),
    );
    keep_time(&mut app);
    start(&mut app, Kind::Gpg);
    shell.flow(&mut app, F::VaultExisting(0));
    through_where(&mut shell, &mut app);
    shell.plug_known(&mut app, "S1");
    shell.flow(&mut app, F::Tap);
    shell.pull(&mut app, "S1");
    shell.plug_known(&mut app, "S2");
    assert!(!has_part(&mut app, "Use w2.kdbx"), "no press");
    // A press that is made anyway is not taken.
    let k = app
        .flow()
        .unwrap()
        .live
        .reading
        .as_ref()
        .unwrap()
        .files
        .iter()
        .position(|(f, _)| f.name == "w2.kdbx")
        .unwrap();
    shell.flow(&mut app, F::PickFile(0, k));
    shell.flow(&mut app, F::Tap);
    shell.pull(&mut app, "S2");
    let names: Vec<_> = app.inbox().iter().map(|i| i.name.clone()).collect();
    assert!(
        app.inbox()
            .iter()
            .any(|i| i.name == "w.kdbx" && i.bytes == w_new),
        "{names:?}"
    );
}

#[test]
fn a_stick_named_by_its_own_press_with_an_unrelated_file_shows_the_line_beside() {
    let mut shell = Shell::new();
    let other = vault("Other", PHRASE_2, 0x82);
    let main = vault("Main", PHRASE, 0x81);
    let mut s1 = Disk::empty("S1", "ONE");
    s1.serial = Some(1);
    s1.files = vec![("a.kdbx".into(), main)];
    shell.add(s1);
    boot_disk(&mut shell);
    shell.disk_mut("BOOT").files = vec![("other.kdbx".into(), other)];
    let mut app = fresh(&mut shell, Kind::Gpg);
    shell.flow(&mut app, F::VaultExisting(0));
    through_where(&mut shell, &mut app);
    shell.plug_known(&mut app, "S1");
    shell.flow(&mut app, F::Tap);
    shell.pull(&mut app, "S1");
    shell.plug(&mut app, "BOOT");
    let k = app.sticks().iter().position(|s| s.boot).unwrap();
    shell.flow(&mut app, F::UseBoot(k));
    expect(&mut app, "Vault 1 goes beside these");
}

#[test]
fn two_copies_learned_across_two_locks_say_the_second_is_not_that_vault_with_nothing_pressed() {
    let (mut shell, mut app) = two_vaults_one_file_each();
    shell.flow(&mut app, F::UnlockVault(0));
    unlock(&mut app, PHRASE);
    let mut app = shell.lock_unseen(&mut app);
    shell.flow(&mut app, F::UnlockVault(1));
    unlock(&mut app, PHRASE);
    expect(&mut app, "b.kdbx is not Vault 2");
    shell.flow(&mut app, F::UnlockVault(0));
    unlock(&mut app, PHRASE);
    // What the lock kept knows both files as one vault: one row in Files.
    let mut app = shell.lock_unseen(&mut app);
    assert_eq!(app.vault_files().len(), 1);
    expect(&mut app, "b.kdbx is not Vault 2");
    expect(&mut app, "A new vault beside it");
    assert_eq!(shell.vault_writes(), 0);
}
