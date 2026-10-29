//! A GPG key restored from its paperkey and its public certificate
//! (`docs/GPG-PAPERKEY.md` §8, tests 19 to 21, and the Pi's panel): the
//! public key in Files offers the restore, the flow's plan, sticks and
//! vaults are Make's, the lines are typed as key presses and each is
//! checked as it ends, the key goes into every vault of the plan and
//! signs at once, a lock forgets the lines and keeps the certificate, and
//! a key already in an open vault is not saved again.
//!
//! The key is the one `tools/vectors/paperkey/faraday-key.txt` is for:
//! its certificate as Make writes it (`faraday_keys::gpg::create`), and
//! its paperkey as paperkey 1.6 printed it from the key's secret key,
//! comment header and all.

use crate::common::*;
use faraday_core::backup_flow::steps::{FlowAction as F, RestorePress as R};
use faraday_core::backup_flow::{BackupFlow, Kind, Plan, RestoreFrom, Step};
use faraday_core::file_menu::{FilePress, Row};
use faraday_core::vault::VaultAction as V;
use faraday_core::{Action, Faraday, Screen, Sheet, StorageEvent};
use faraday_pgp::{Armor, Key, armor, fingerprint_text};
use faraday_shell_api::{App, BootState, DisplayInfo, Event, Key as KeyIn, SecureHardware};
use faraday_vault::records::kind;

const PHRASE: &str = "first vault phrase";
const CERT: &str = "test-person.asc";
const YEAR: u32 = 31_536_000;
/// paperkey 1.6's `--secret-key` output for the key of [`paper_key`]
/// (`tools/vectors/paperkey/README.md`).
const PRINTED: &str = include_str!("../../../../tools/vectors/paperkey/faraday-key.txt");

/// The key paperkey printed `PRINTED` from (`faraday-pgp`'s example
/// `paperkey_inputs`): seeds with leading zero bytes, made at
/// 1,780,000,000 with one user ID and a two-year term.
fn paper_key() -> Key {
    let mut k = Key {
        primary_seed: [0x11; 32],
        created: 1_780_000_000,
        subkey_seed: [0x22; 32],
        subkey_created: 1_780_000_000,
        user_ids: vec!["Test Person <test@example.com>".to_string()],
        expiry: 2 * YEAR,
    };
    k.primary_seed[0] = 0;
    k.subkey_seed[..2].fill(0);
    k
}

/// The key as Make makes it from those seeds: its record and certificate.
fn made_by_make(k: &Key) -> faraday_keys::gpg::Made {
    faraday_keys::gpg::create(
        &zeroize::Zeroizing::new(k.primary_seed),
        &zeroize::Zeroizing::new(k.subkey_seed),
        k.created,
        k.user_ids[0].clone(),
        k.expiry,
    )
}

/// Its certificate as Make wrote it, armoured, as a stick file.
fn cert_file(k: &Key) -> (String, Vec<u8>) {
    let text = armor(Armor::PublicKey, &made_by_make(k).certificate);
    (CERT.to_string(), text.into_bytes())
}

/// Its paperkey's lines as paperkey printed them, the comments left out.
fn printed() -> Vec<String> {
    PRINTED
        .lines()
        .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
        .map(str::to_string)
        .collect()
}

/// A key's paperkey lines, as its page shows them.
fn lines(k: &Key) -> Vec<String> {
    k.paperkey().lines().map(str::to_string).collect()
}

/// A line typed as key presses: a whole line ends by itself; Enter ends
/// one that did not.
fn type_line(app: &mut Faraday, line: &str) {
    for c in line.chars() {
        app.event(Event::Key(KeyIn::Char(c)));
    }
    app.event(Event::Key(KeyIn::Enter));
}

fn step(app: &Faraday) -> Step {
    app.flow().expect("a flow").step
}

/// A fresh machine with the certificate and a text file in Files, two
/// empty sticks, and the clock kept.
fn machine(shell: &mut Shell, k: &Key) -> Faraday {
    for n in ["S1", "S2"] {
        shell.add(Disk::empty(n, "KEYSTICK"));
    }
    let notes = (
        "notes.txt".to_string(),
        b"Signed after a restore.\n".to_vec(),
    );
    let mut app = process(vec![cert_file(k), notes], Vec::new(), Vec::new());
    keep_time(&mut app);
    app
}

/// Files' ⋮ on the certificate offers **Restore from its paperkey**; it
/// opens the flow on its head card.
fn from_files(app: &mut Faraday) {
    app.press(Action::Nav(Screen::Files));
    let at = app.inbox().iter().position(|i| i.name == CERT).unwrap();
    let _ = app.frame();
    app.press(Action::File(FilePress::Menu(Row::In(at))));
    assert_eq!(app.sheet, Some(Sheet::FileMenu));
    let restore = Action::Flow(F::Restore(R::From(at)));
    let offered = app
        .files_menu()
        .menu
        .as_ref()
        .expect("its menu")
        .lines
        .iter();
    assert!(
        offered
            .into_iter()
            .any(|(l, a)| l == "Restore from its paperkey" && *a == restore),
        "the public key's ⋮ offers the restore"
    );
    app.press(restore);
    assert_eq!(app.screen, Screen::BackupFlow);
}

/// From Files to step 4: the head card's Continue, the plan as it is,
/// both sticks brought in, the vault made.
fn to_lines(shell: &mut Shell, app: &mut Faraday) {
    from_files(app);
    shell.flow(app, F::Restore(R::HeadContinue));
    shell.flow(app, F::Continue);
    assert_eq!(step(app), Step::Sticks);
    for (disk, hold) in [("S1", false), ("S2", true)] {
        shell.plug(app, disk);
        shell.flow(app, if hold { F::Hold } else { F::Tap });
        shell.pull(app, disk);
    }
    assert_eq!(step(app), Step::Vaults);
    create_vault(shell, app, 0, PHRASE);
    assert_eq!(step(app), Step::Make, "{:?}", unlock_error(app));
}

fn gpg_records(app: &Faraday) -> Vec<faraday_vault::Record> {
    (0..app.vaults().open.len())
        .flat_map(|o| {
            contents(app, o)
                .of(kind::GPG)
                .map(|(_, r)| r.clone())
                .collect::<Vec<_>>()
        })
        .collect()
}

/// Test 19: the public key in Files, Restore from its paperkey, the plan,
/// the sticks, the vault, the lines typed as key presses: a mistyped line
/// says why and does not advance; once fixed, the flow goes on; **Restore
/// the key** saves it into the vault of the plan, and it signs at once.
#[test]
fn a_gpg_key_is_restored_from_its_paperkey_into_the_vaults_of_the_plan() {
    let k = paper_key();
    let mut shell = Shell::new();
    let mut app = machine(&mut shell, &k);
    from_files(&mut app);
    // The head card: the public key, and the bytes line 1 must read.
    assert!(has_part(&mut app, "Test Person <test@example.com>"));
    assert!(has(&mut app, "Line 1 of its paperkey"));
    assert!(has_part(&mut app, "00 04"));
    // Left and opened again: the same flow, not a second one.
    app.press(Action::Nav(Screen::Home));
    to_lines(&mut shell, &mut app);
    assert_eq!(app.backup_flows().len(), 1);
    let ls = printed();
    assert_eq!(ls.len(), 7, "paperkey's seven lines");
    type_line(&mut app, &ls[0]);
    assert!(has(&mut app, "Line 1 · checked"));
    // Line 2 with a digit changed.
    let wrong = ls[1].replacen(": 00 ", ": 01 ", 1);
    type_line(&mut app, &wrong);
    assert!(
        has(&mut app, "CRC on line 2 does not match"),
        "{:?}",
        texts(&mut app)
    );
    assert!(!has(&mut app, "Line 2 · checked"));
    for _ in 0..wrong.len() {
        app.event(Event::Key(KeyIn::Backspace));
    }
    for l in &ls[1..] {
        type_line(&mut app, l);
    }
    assert!(has(&mut app, "Line 7 · checked"), "{:?}", texts(&mut app));
    assert!(offered(&mut app, Action::Flow(F::Restore(R::Restore))));
    shell.flow(&mut app, F::Restore(R::Restore));
    let made = app.flow().is_some_and(|f| f.made);
    let toast = app.toast_text().map(str::to_string);
    assert!(made, "{toast:?} {:?}", texts(&mut app));
    assert_eq!(app.flow().unwrap().made_in, [0]);
    assert!(
        has(&mut app, "Restored into Vault 1"),
        "{:?}",
        texts(&mut app)
    );
    let records = gpg_records(&app);
    assert_eq!(records.len(), 1);
    let kept = gpg_key_of(&records[0]).expect("a key");
    assert_eq!(kept.fingerprint(), k.fingerprint());
    assert_eq!(kept.primary_seed, k.primary_seed);
    assert_eq!(kept.subkey_seed, k.subkey_seed);
    assert_eq!(kept.expiry, k.expiry);
    assert_eq!(kept.user_ids, k.user_ids);
    assert_eq!(
        records[0].field(7),
        Some(&made_by_make(&k).certificate[..]),
        "the certificate Make wrote"
    );
    // It signs at once (Q2): nothing waits for the writes.
    assert_eq!(app.backup_waits(Kind::Gpg, &k.fingerprint()), None);
    app.press(Action::Nav(Screen::VaultContents));
    app.press(Action::Vault(V::Category(Kind::Gpg.category())));
    assert!(has_part(&mut app, &fingerprint_text(&k.fingerprint())));
    // The one file to sign starts ticked.
    app.press(Action::Vault(V::GpgSignPick));
    app.press(Action::Vault(V::GpgSign));
    let toast = app.toast_text().map(str::to_string);
    let sheet = app.sheet;
    assert_eq!(
        sheet,
        Some(Sheet::GpgSign),
        "{toast:?} {:?}",
        texts(&mut app)
    );
    app.press(Action::Vault(V::GpgHoldSign));
    let sig = app.outbox.iter().find(|i| i.name == "notes.txt.asc");
    assert!(sig.is_some(), "signed at once: {:?}", app.toast_text());
}

/// Test 20: a lock at step 4 wipes the lines typed; the flow comes back
/// with the certificate, and once the vault is open again asks for line
/// 1 again.
#[test]
fn a_lock_at_step_four_forgets_the_lines_and_keeps_the_public_key() {
    let k = paper_key();
    let mut shell = Shell::new();
    let mut app = machine(&mut shell, &k);
    to_lines(&mut shell, &mut app);
    let ls = printed();
    type_line(&mut app, &ls[0]);
    type_line(&mut app, &ls[1]);
    assert!(has(&mut app, "Line 2 · checked"));
    let mut app = shell.lock(&mut app);
    let f = app.flow().expect("the flow comes back");
    assert_eq!(
        f.restore.as_ref().map(|r| r.fingerprint),
        Some(k.fingerprint())
    );
    assert!(!f.made);
    if step(&app) == Step::Vaults {
        open_vault(&mut shell, &mut app, 0, PHRASE);
    }
    assert_eq!(step(&app), Step::Make, "{:?}", unlock_error(&app));
    assert!(!has(&mut app, "Line 1 · checked"), "{:?}", texts(&mut app));
    assert!(has(&mut app, "Line 1"), "{:?}", texts(&mut app));
}

/// The test vault unlocked from Files.
fn with_test_vault() -> Faraday {
    let vault = faraday_core::testkit::files()
        .unwrap()
        .into_iter()
        .find(|(n, _)| n == "vault.fvlt")
        .unwrap();
    let mut app = process(vec![vault], Vec::new(), Vec::new());
    keep_time(&mut app);
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Open(0)));
    type_text(&mut app, faraday_core::testkit::VAULT_PASSPHRASES[0]);
    app.press(Action::Vault(V::Unlock));
    settle(&mut app, 1);
    app.press(Action::Vault(V::LoadTicked));
    assert_eq!(app.vaults().open.len(), 1);
    app
}

/// A restore of `from` at step 4, its plan the open vault.
fn restore_at_make(app: &mut Faraday, from: RestoreFrom) {
    let c = faraday_core::testkit::compute();
    let (salt, name) = {
        let o = &app.vaults().open[app.vaults().current];
        (o.header().salt, o.name.clone())
    };
    let mut plan = Plan::key_default();
    plan.vaults[0].salt = Some(salt);
    plan.vaults[0].file = Some(name);
    plan.vaults[0].existing = true;
    let mut f = BackupFlow::restoring(&c, from);
    f.plan = plan;
    f.step = Step::Make;
    let i = faraday_core::testkit::flow_push(app, f);
    app.press(Action::Flow(F::Show(i)));
}

/// Test 21: the key already in an open vault: **Restore the key** says
/// which and saves nothing.
#[test]
fn a_key_already_in_an_open_vault_is_not_saved_again() {
    let mut app = with_test_vault();
    make_released(&mut app, Kind::Gpg, &["Kept", ""]);
    let before = gpg_records(&app);
    let made = gpg_key_of(before.last().expect("the key made")).expect("a key");
    let cert = armor(Armor::PublicKey, &made.certificate(made.created));
    let from = RestoreFrom::read(cert.as_bytes()).expect("Make's certificate reads");
    restore_at_make(&mut app, from);
    for l in lines(&made) {
        type_line(&mut app, &l);
    }
    app.press(Action::Flow(F::Restore(R::Restore)));
    let name = app.vaults().open[0].name.clone();
    assert!(
        has(&mut app, &format!("This key is in {name}")),
        "{:?}",
        texts(&mut app)
    );
    assert!(!app.flow().unwrap().made);
    assert_eq!(gpg_records(&app).len(), before.len(), "nothing saved");
}

/// What the page drawn now has cut off at its sides, from its top to its
/// foot.
fn cut_off(app: &mut Faraday, what: &str) -> Vec<String> {
    let mut found = Vec::new();
    for _ in 0..30 {
        let _ = app.frame();
        found.extend(
            app.drawn_past_sides()
                .into_iter()
                .map(|p| format!("{what}: {p}")),
        );
        app.event(Event::Scroll {
            x: 240,
            y: 320,
            dy: 60,
        });
        app.settle();
    }
    found.dedup();
    found
}

/// The Pi's panel: the sheet, the head card with line 1, step 4 with a
/// line checked, one typed and one in error: nothing cut off at its
/// sides.
#[test]
fn nothing_of_a_restore_is_cut_off_on_the_pi_panel() {
    let k = paper_key();
    let mut app = with_test_vault();
    app.event(Event::Display(DisplayInfo {
        width: 480,
        height: 640,
        dpi: 286,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
    app.storage(StorageEvent::Restored {
        inbox: vec![cert_file(&k)],
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    let mut found = Vec::new();
    app.press(Action::Flow(F::Restore(R::Sheet)));
    assert_eq!(app.sheet, Some(Sheet::GpgRestore));
    found.extend(cut_off(&mut app, "the sheet"));
    app.press(Action::Cancel);
    let from = RestoreFrom::read(&cert_file(&k).1).unwrap();
    restore_at_make(&mut app, from);
    app.press(Action::Flow(F::Restore(R::Head)));
    found.extend(cut_off(&mut app, "the head card"));
    app.press(Action::Flow(F::Restore(R::Head)));
    let ls = printed();
    type_line(&mut app, &ls[0]);
    type_line(&mut app, &ls[1].replacen(": 00 ", ": 01 ", 1));
    found.extend(cut_off(&mut app, "step 4"));
    assert!(
        found.is_empty(),
        "cut off on the Pi's panel:\n{}",
        found.join("\n")
    );
}
