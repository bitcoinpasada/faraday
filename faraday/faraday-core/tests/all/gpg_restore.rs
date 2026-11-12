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

use crate::common::RecordsOf as _;
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

/// A paperkey line typed in full with a digit wrong stays; the caret
/// moved back to the digit and the digit typed again there, the line is
/// right and goes on to the next at once, with no Enter
/// (`docs/TEXT-CURSOR.md`).
#[test]
fn a_line_fixed_in_the_middle_is_checked_the_moment_it_is_right() {
    let k = paper_key();
    let mut shell = Shell::new();
    let mut app = machine(&mut shell, &k);
    to_lines(&mut shell, &mut app);
    let ls = printed();
    type_line(&mut app, &ls[0]);
    let wrong = ls[1].replacen(": 00 ", ": 01 ", 1);
    for c in wrong.chars() {
        app.event(Event::Key(KeyIn::Char(c)));
    }
    assert!(!has(&mut app, "Line 2 · checked"), "a wrong line stays");
    let digit = wrong.find(": 01 ").expect("the digit") + 3;
    app.event(Event::Key(KeyIn::Home));
    for _ in 0..digit {
        app.event(Event::Key(KeyIn::Right));
    }
    app.event(Event::Key(KeyIn::Delete));
    assert!(!has(&mut app, "Line 2 · checked"), "a line short stays");
    app.event(Event::Key(KeyIn::Char('0')));
    assert!(has(&mut app, "Line 2 · checked"), "{:?}", texts(&mut app));
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

/// The scanner's note, while it is open.
fn scan_note(app: &Faraday) -> Option<String> {
    app.scan().and_then(|s| s.note.clone())
}

/// A code read by the camera on screen.
fn scan(app: &mut Faraday, bytes: &[u8]) {
    app.event(Event::Scanned {
        bytes: bytes.to_vec(),
    });
    let _ = app.frame();
}

/// The public key scanned on the sheet's **Scan the public key** (P4):
/// with no public key in Files the sheet says so and offers the scanner;
/// a code that is not a public key is named on the scanner and filed
/// nowhere; the certificate's code is filed in Files under
/// `{name}-{fp8}.asc` and the restore opens on it.
#[test]
fn the_public_key_scanned_on_the_sheet_is_filed_and_restored_from() {
    let k = paper_key();
    let mut app = with_test_vault();
    app.press(Action::Flow(F::Restore(R::Sheet)));
    assert_eq!(app.sheet, Some(Sheet::GpgRestore));
    assert!(has(
        &mut app,
        "Copy its public key in on a stick visit first."
    ));
    let scan_press = Action::Flow(F::Restore(R::Scan));
    assert!(offered(&mut app, scan_press));
    app.press(scan_press);
    assert_eq!(app.sheet, Some(Sheet::Scan));
    assert!(has(&mut app, "Scan the public key"));
    let files = app.inbox().len();
    scan(&mut app, b"Signed after a restore.");
    assert_eq!(
        scan_note(&app).as_deref(),
        Some("Not a public key Faraday reads")
    );
    assert_eq!(app.inbox().len(), files, "nothing filed");
    assert_eq!(app.sheet, Some(Sheet::Scan), "the camera stays on");
    scan(&mut app, &cert_file(&k).1);
    let hex: String = k.fingerprint().iter().map(|b| format!("{b:02X}")).collect();
    let name = format!("test-person-{}.asc", &hex[32..]);
    assert!(
        app.inbox().iter().any(|i| i.name == name),
        "filed as {name}: {:?}",
        app.inbox().iter().map(|i| &i.name).collect::<Vec<_>>()
    );
    assert_eq!(app.screen, Screen::BackupFlow);
    let f = app.flow().expect("the restore");
    assert_eq!(
        f.restore.as_ref().map(|r| r.fingerprint),
        Some(k.fingerprint())
    );
    assert!(has(&mut app, "Line 1 of its paperkey"));
}

/// A GPG public key scanned at Home's Scan is filed as the Restore sheet's
/// own scan files it, `{name}-{fp8}.asc`; the toast names it and Restore a
/// GPG key lists it. A file sent in the Faraday file envelope keeps the
/// name it was sent under.
#[test]
fn a_public_key_scanned_at_home_keeps_its_real_name() {
    let k = paper_key();
    let mut app = with_test_vault();
    app.press(Action::Nav(Screen::Home));
    app.press(Action::Scan);
    assert_eq!(app.sheet, Some(Sheet::Scan));
    scan(&mut app, &cert_file(&k).1);
    let hex: String = k.fingerprint().iter().map(|b| format!("{b:02X}")).collect();
    let name = format!("test-person-{}.asc", &hex[32..]);
    let names = |app: &Faraday| {
        app.inbox()
            .iter()
            .map(|i| i.name.clone())
            .collect::<Vec<_>>()
    };
    assert!(names(&app).contains(&name), "{:?}", names(&app));
    assert_eq!(
        app.toast_text(),
        Some(format!("{name} is in Files").as_str())
    );
    let files = app.inbox().len();
    app.press(Action::Nav(Screen::Home));
    app.press(Action::Scan);
    scan(&mut app, &cert_file(&k).1);
    assert_eq!(
        app.inbox().len(),
        files,
        "the same file is the one in Files"
    );
    app.press(Action::Flow(F::Restore(R::Sheet)));
    assert!(
        has(&mut app, "Test Person <test@example.com>"),
        "{:?}",
        texts(&mut app)
    );

    let env = faraday_qr::envelope::pack("sent-key.asc", &cert_file(&k).1, "file").unwrap();
    app.press(Action::Nav(Screen::Home));
    app.press(Action::Scan);
    scan(&mut app, &env);
    assert!(
        names(&app).contains(&"sent-key.asc".to_string()),
        "{:?}",
        names(&app)
    );
}

/// A public key scanned that is not restored here is still filed, and the
/// sheet comes back saying why.
#[test]
fn a_public_key_scanned_that_is_not_restored_here_is_filed_and_says_why() {
    let rsa = include_bytes!("../../../../tools/vectors/paperkey/gpg-rsa3072.asc");
    let mut app = with_test_vault();
    app.press(Action::Flow(F::Restore(R::Sheet)));
    app.press(Action::Flow(F::Restore(R::Scan)));
    scan(&mut app, rsa);
    let filed = app
        .inbox()
        .iter()
        .any(|i| i.name.ends_with(".asc") && rsa.starts_with(&i.bytes));
    assert!(
        filed,
        "{:?} {:?}",
        scan_note(&app),
        app.inbox().iter().map(|i| &i.name).collect::<Vec<_>>()
    );
    assert_eq!(app.sheet, Some(Sheet::GpgRestore));
    assert!(
        has(
            &mut app,
            "rsa3072 keys are not restored here: Faraday's are Ed25519"
        ),
        "{:?}",
        texts(&mut app)
    );
}

/// The test vault unlocked from Files.
fn with_test_vault() -> Faraday {
    let vault = faraday_core::testkit::files()
        .unwrap()
        .into_iter()
        .find(|(n, _)| n == "vault.kdbx")
        .unwrap();
    let mut app = process(vec![vault], Vec::new(), Vec::new());
    keep_time(&mut app);
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Open(0)));
    type_text(&mut app, faraday_core::testkit::VAULT_PASSPHRASE);
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
        (o.vault_id(), o.name.clone())
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

/// Learn on a restore opens on the GPG page, which says how a paperkey's
/// lines are typed.
#[test]
fn learn_on_a_restore_opens_the_gpg_page() {
    let k = paper_key();
    let mut app = with_test_vault();
    let from = RestoreFrom::read(&cert_file(&k).1).unwrap();
    restore_at_make(&mut app, from);
    assert_eq!(app.screen, Screen::BackupFlow);
    app.press(Action::Learn);
    assert!(app.learn().shown.starts_with("backup"));
}

/// paperkey 1.6's raw form of the same key (`--output-type raw`).
const RAW: &[u8] = include_bytes!("../../../../tools/vectors/paperkey/faraday-key.raw");

/// A paperkey's code scanned at step 4 (P5): offered while no line is
/// checked; a code with a mistyped line is refused on the scanner at that
/// line and the camera stays on; paperkey's printed lines, or its raw
/// form, fill every line, checked, and the key restores from them.
#[test]
fn a_paperkey_scanned_at_step_four_fills_every_line() {
    let k = paper_key();
    for good in [PRINTED.as_bytes(), RAW] {
        let mut app = with_test_vault();
        let from = RestoreFrom::read(&cert_file(&k).1).unwrap();
        restore_at_make(&mut app, from);
        let press = Action::Flow(F::Restore(R::ScanPaperkey));
        assert!(offered(&mut app, press), "{:?}", texts(&mut app));
        app.press(press);
        assert_eq!(app.sheet, Some(Sheet::Scan));
        assert!(has(&mut app, "Scan the paperkey"));
        let wrong = PRINTED.replacen("  3: 11", "  3: 12", 1);
        scan(&mut app, wrong.as_bytes());
        assert_eq!(
            scan_note(&app).as_deref(),
            Some("CRC on line 3 does not match")
        );
        assert_eq!(app.sheet, Some(Sheet::Scan), "the camera stays on");
        scan(&mut app, good);
        assert_eq!(app.sheet, None, "{:?}", scan_note(&app));
        assert!(has(&mut app, "Line 7 · checked"), "{:?}", texts(&mut app));
        assert!(!offered(&mut app, press), "no second code over the lines");
        app.press(Action::Flow(F::Restore(R::Restore)));
        assert!(app.flow().unwrap().made, "{:?}", texts(&mut app));
        let records = gpg_records(&app);
        let kept = gpg_key_of(records.last().unwrap()).expect("a key");
        assert_eq!(kept.fingerprint(), k.fingerprint());
    }
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

/// The key of [`paper_key`] restored into the test vault, its page open
/// on a panel of `width` × `height` at `dpi`.
fn key_page(width: u16, height: u16, dpi: u16) -> Faraday {
    let k = paper_key();
    let mut app = with_test_vault();
    app.event(Event::Display(DisplayInfo {
        width,
        height,
        dpi,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
    restore_at_make(&mut app, RestoreFrom::read(&cert_file(&k).1).unwrap());
    app.press(Action::Flow(F::Restore(R::ScanPaperkey)));
    scan(&mut app, PRINTED.as_bytes());
    app.press(Action::Flow(F::Restore(R::Restore)));
    assert!(app.flow().unwrap().made, "{:?}", texts(&mut app));
    app.press(Action::Nav(Screen::VaultContents));
    app.press(Action::Vault(V::Category(Kind::Gpg.category())));
    app.press(Action::Vault(V::Item(0)));
    let _ = app.frame();
    assert!(has_part(&mut app, "Test Person"), "{:?}", texts(&mut app));
    app
}

/// The key's page offers Show as QR under the masked paperkey; one press,
/// with no hold, opens the sheet marked Secret on the paperkey's lines,
/// with no picture, no Outbox file and no choice of format, speed or
/// size; Done and a lock each take it away (F215).
#[test]
fn a_gpg_keys_paperkey_shows_as_one_secret_code_and_is_gone_after_done_and_a_lock() {
    let mut app = key_page(480, 640, 160);
    let show = Action::Vault(V::PaperkeyQr);
    assert!(offered(&mut app, show), "{:?}", texts(&mut app));
    for end in [Action::Cancel, Action::Lock] {
        app.press(show);
        let q = app.qr().expect("a press opens the sheet");
        assert!(q.secret && q.one_code);
        assert!(!q.offers_png());
        assert!(
            has(&mut app, "Secret · whoever scans this can use it"),
            "{:?}",
            texts(&mut app)
        );
        assert!(has_part(&mut app, "Test Person"));
        assert!(has_part(&mut app, "· Paperkey"));
        for no in ["UR", "BBQr", "Parts", "Small", "Medium", "Large"] {
            assert!(!has(&mut app, no), "{no} is offered");
        }
        assert!(!app.drawn_actions().iter().any(|a| matches!(
            a,
            Action::QrFormat(_) | Action::QrPartSize(_) | Action::QrPng
        )));
        // A paperkey always fits one code, so a size taken would show only
        // in what the sheet holds, not in its frames.
        let (format, part) = {
            let q = app.qr().unwrap();
            (q.format, q.part)
        };
        let size = faraday_core::QR_PARTS
            .into_iter()
            .find(|n| *n != part)
            .expect("another size");
        app.press(Action::QrFormat(1));
        app.press(Action::QrPartSize(size));
        app.press(Action::QrPng);
        let q = app.qr().unwrap();
        assert_eq!(q.frames.len(), 1);
        assert_eq!((q.format, q.part), (format, part), "a choice was taken");
        assert!(app.outbox.is_empty(), "the paperkey went to the Outbox");
        app.press(end);
        assert!(app.qr().is_none(), "{end:?} left the code on screen");
        if end == Action::Lock {
            break;
        }
    }
}

/// The key's page open, a KVM plugged in, its sheet closed and the KVM
/// still there.
fn key_page_with_kvm() -> Faraday {
    let mut app = key_page(480, 640, 160);
    attach_kvm(&mut app);
    app
}

/// A KVM plugged in with the key's page open, its sheet closed and the
/// KVM still there: the press waits under the sheet and no code shows
/// until the KVM is pulled.
#[test]
fn a_paperkeys_code_waits_while_a_kvm_is_attached() {
    let mut app = key_page_with_kvm();
    app.press(Action::Vault(V::PaperkeyQr));
    assert_eq!(app.sheet, Some(Sheet::Devices));
    assert!(app.qr().is_none(), "the code showed with a KVM attached");
    app.usb_devices(&[]);
    app.press(Action::DevicesCheck);
    // This machine passed no EDID, so the reminder about a TV is due too.
    assert!(app.qr().is_none(), "the code showed over the reminder");
    app.press(Action::DevicesUnderstood);
    assert!(app.qr().is_some(), "the press did not carry on");
}

/// The paperkey's code on show when a KVM arrives goes off the sheet at
/// once, and shows again once the KVM is gone and checked, with no press.
#[test]
fn a_paperkeys_code_on_show_hides_for_a_kvm_and_comes_back_when_it_is_gone() {
    let mut app = key_page(480, 640, 160);
    crate::common::shown_on_a_read_machine(&mut app, Action::Vault(V::PaperkeyQr));
    assert!(app.qr().is_some(), "the code did not show");
    crate::common::kvm_arrives(&mut app);
    assert!(app.qr().is_none(), "the code stayed with a KVM arriving");
    crate::common::kvm_gone(&mut app);
    assert!(app.qr().is_some(), "the code did not come back");
}

/// A press that waited under the device sheet is dropped when a stick
/// arriving in an unclean session puts up the lock sheet: the sheet
/// closed later, and a new device checked again, show no code.
#[test]
fn a_press_that_waited_does_not_run_after_the_lock_sheet_replaced_the_sheet() {
    let mut app = key_page_with_kvm();
    app.press(Action::Vault(V::PaperkeyQr));
    assert_eq!(app.sheet, Some(Sheet::Devices));
    let stick = |on: bool| {
        if on {
            vec![faraday_core::StickInfo {
                make: Default::default(),
                serial: None,
                id: "a".into(),
                label: "STICK".into(),
                boot: false,
                files: Vec::new(),
                vaults: Vec::new(),
            }]
        } else {
            Vec::new()
        }
    };
    app.storage(StorageEvent::Sticks(stick(true)));
    assert_eq!(app.sheet, Some(Sheet::Lock));
    app.storage(StorageEvent::Sticks(stick(false)));
    assert_eq!(app.sheet, None);
    let mut other = kvm();
    other.port = "1-3".into();
    app.usb_devices(&[other]);
    assert_eq!(app.sheet, Some(Sheet::Devices));
    kvm_gone(&mut app);
    assert!(app.qr().is_none(), "the old press ran with no press");
}

/// The code's text, scanned at Restore's step 4 on another machine, fills
/// every line and restores the same key, the record equal to Make's; on
/// Home's Scan it is named and filed nowhere.
#[test]
fn the_shown_paperkey_restores_the_key_on_another_machine() {
    let k = paper_key();
    let mut old = key_page(480, 640, 160);
    old.press(Action::Vault(V::PaperkeyQr));
    let text = crate::common::qr_text(&old);
    let mut home = faraday_core::testkit::started();
    home.press(Action::Nav(Screen::Home));
    home.press(Action::Scan);
    scan(&mut home, text.as_bytes());
    assert!(
        scan_note(&home).is_some_and(|n| n.contains("paperkey")),
        "{:?}",
        scan_note(&home)
    );
    assert!(home.inbox().is_empty(), "the paperkey was filed");
    let mut shell = Shell::new();
    let mut app = machine(&mut shell, &k);
    to_lines(&mut shell, &mut app);
    app.press(Action::Flow(F::Restore(R::ScanPaperkey)));
    scan(&mut app, text.as_bytes());
    assert_eq!(app.sheet, None, "{:?}", scan_note(&app));
    assert!(has(&mut app, "Line 7 · checked"), "{:?}", texts(&mut app));
    shell.flow(&mut app, F::Restore(R::Restore));
    assert!(app.flow().unwrap().made, "{:?}", texts(&mut app));
    let records = gpg_records(&app);
    assert_eq!(records.len(), 1);
    let kept = gpg_key_of(&records[0]).expect("a key");
    assert_eq!(kept.fingerprint(), k.fingerprint());
    assert_eq!(kept.primary_seed, k.primary_seed);
    assert_eq!(kept.subkey_seed, k.subkey_seed);
    assert_eq!(records[0].field(7), Some(&made_by_make(&k).certificate[..]));
}

/// The module size, pixels, of the code the frame draws: the top row of
/// the first finder pattern is seven modules.
fn module_px(app: &mut Faraday) -> usize {
    app.settle();
    let f = app.frame();
    let dark: Vec<bool> = f
        .rgba
        .chunks_exact(4)
        .map(|p| p[..3].iter().all(|v| *v < 16))
        .collect();
    let first = dark.iter().position(|d| *d).expect("a code is drawn");
    let run = dark[first..].iter().take_while(|d| **d).count();
    run / 7
}

/// The code takes the sheet's whole width: two pixels a module at
/// 240 × 320, four at 480 × 640.
#[test]
fn the_paperkey_code_draws_at_two_pixels_a_module_on_the_smallest_panel() {
    for (w, h, dpi, want) in [(240, 320, 143, 2), (480, 640, 286, 4)] {
        let mut app = key_page(w, h, dpi);
        app.press(Action::Vault(V::PaperkeyQr));
        assert_eq!(module_px(&mut app), want, "{w}x{h}");
    }
}

/// A paperkey code too fine for the camera says to bring it closer or
/// type the paperkey, not to use smaller parts.
#[test]
fn a_paperkey_code_too_fine_for_the_camera_says_to_type_it() {
    let k = paper_key();
    let mut app = with_test_vault();
    restore_at_make(&mut app, RestoreFrom::read(&cert_file(&k).1).unwrap());
    app.press(Action::Flow(F::Restore(R::ScanPaperkey)));
    for _ in 0..3 {
        app.storage(StorageEvent::QrSeen {
            width: 640,
            height: 480,
            corners: [(100, 100), (300, 100), (300, 300), (100, 300)],
            module_tenths: 14,
            read: false,
            whole: true,
        });
    }
    assert_eq!(
        scan_note(&app).as_deref(),
        Some("Too fine for this camera: bring it closer, or type the paperkey")
    );
}

/// The line being typed, with its label, is above the docked keyboard.
fn line_clear(app: &mut Faraday, what: &str) {
    for _ in 0..3 {
        let _ = app.frame();
    }
    let top = app.keyboard_top().expect("the keyboard is docked");
    let (_, y, _, h) = app.focused_field().expect("the line has focus");
    assert!(
        y >= 0 && y + h <= i32::from(top),
        "{what}: the line ({y} to {}) is above the keyboard's top ({top})",
        y + h
    );
}

/// Step 4 on the Pi's panel, a line typed with the keyboard up: the line
/// stays above the keyboard as its digits wrap and grow it, and after
/// Next line moves on.
#[test]
fn the_line_typed_stays_above_the_docked_keyboard_on_the_pi_panel() {
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
    let from = RestoreFrom::read(&cert_file(&k).1).unwrap();
    restore_at_make(&mut app, from);
    app.press(Action::Flow(F::Restore(R::Field)));
    line_clear(&mut app, "an empty line");
    let ls = printed();
    let mut at = 0;
    for c in ls[0].chars() {
        app.event(Event::Key(KeyIn::Char(c)));
        at += 1;
        if at % 12 == 0 {
            line_clear(&mut app, &format!("{at} characters typed"));
        }
    }
    app.press(Action::Flow(F::Restore(R::Next)));
    line_clear(&mut app, "the next line");
}

/// Step 4 on the Pi's panel with nothing typed: the keyboard is down, so
/// Scan the paperkey and Next line show; pressing the line brings the
/// keyboard up and digits type.
#[test]
fn step_four_opens_with_the_keyboard_down_and_the_line_brings_it_up() {
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
    restore_at_make(&mut app, RestoreFrom::read(&cert_file(&k).1).unwrap());
    for _ in 0..3 {
        let _ = app.frame();
    }
    assert!(app.keyboard_top().is_none(), "the keyboard is down");
    assert!(has(&mut app, "Scan the paperkey"), "{:?}", texts(&mut app));
    assert!(has(&mut app, "Next line"), "{:?}", texts(&mut app));
    app.press(Action::Flow(F::Restore(R::Field)));
    for _ in 0..3 {
        let _ = app.frame();
    }
    assert!(app.keyboard_top().is_some(), "the keyboard is up");
    let ls = printed();
    for c in ls[0].chars() {
        app.event(Event::Key(KeyIn::Char(c)));
    }
    assert!(has(&mut app, "Line 2"), "{:?}", texts(&mut app));
}

/// With a computer's keyboard at the wide layout, digits typed on arrival
/// land in the line without a press first.
#[test]
fn digits_typed_on_arrival_at_step_four_land_in_the_line() {
    let k = paper_key();
    let mut app = with_test_vault();
    restore_at_make(&mut app, RestoreFrom::read(&cert_file(&k).1).unwrap());
    let ls = printed();
    for c in ls[0].chars() {
        app.event(Event::Key(KeyIn::Char(c)));
    }
    assert!(has(&mut app, "Line 1 · checked"), "{:?}", texts(&mut app));
}
