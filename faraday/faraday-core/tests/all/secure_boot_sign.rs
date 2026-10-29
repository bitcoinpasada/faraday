//! Faraday signs a boot program only when it is a release's own
//! (`docs/decisions/F134.md`): the release's `SHA256SUMS` and
//! `SHA256SUMS.asc` were copied in with it, the signature verifies with a
//! release key, and the sums list the file. Anything else is refused by
//! name on the signing page and at the hold, and nothing reaches the
//! Outbox. The release is the GnuPG-made fixture in
//! `tools/vectors/release-sums/`, whose key the tests take as a release
//! key (the test kit's; never in a shipped binary).

use crate::common::type_text;

use faraday_core::backup_flow::Kind;
use faraday_core::testkit;
use faraday_core::vault::VaultAction as V;
use faraday_core::{Action, Faraday, Screen, StickInfo, StorageCommand, StorageEvent};
use faraday_shell_api::{App, BootState, DisplayInfo, EntropyBytes, Event, SecureHardware};

/// The clock the tests start at: October 2026.
const CLOCK: u64 = 1_791_482_652;

const BOOT: &str = "faraday-0.9.0-BOOTX64.EFI";

fn fixture(name: &str) -> Vec<u8> {
    let dir =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools/vectors/release-sums");
    std::fs::read(dir.join(name)).unwrap_or_else(|e| panic!("{name}: {e}"))
}

/// The release's files as a person downloads them onto a stick.
fn release() -> Vec<(String, Vec<u8>)> {
    vec![
        (BOOT.into(), fixture("BOOTX64.EFI")),
        ("SHA256SUMS".into(), fixture("SHA256SUMS")),
        ("SHA256SUMS.asc".into(), fixture("SHA256SUMS.asc")),
    ]
}

fn settle(app: &mut Faraday, from: u64) {
    for t in from..from + 20 {
        let _ = app.frame();
        app.event(Event::Tick { now_ms: t * 1000 });
    }
}

fn display(app: &mut Faraday) {
    app.event(Event::Display(DisplayInfo {
        width: 1366,
        height: 768,
        dpi: 160,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
    for i in 0..8u8 {
        app.event(Event::Entropy(EntropyBytes::new([0x60 + i; 32])));
    }
}

/// A stick with `files`, pressed through Copy: every file read in.
fn visit(app: &mut Faraday, files: &[(String, Vec<u8>)]) {
    app.storage(StorageEvent::Sticks(vec![StickInfo {
        serial: None,
        id: "S".into(),
        label: "FARADAY".into(),
        boot: false,
        files: files
            .iter()
            .map(|(n, b)| (n.clone(), b.len() as u64))
            .collect(),
    }]));
    app.press(Action::VisitCopy);
    while let Some(c) = app.poll_storage() {
        if let StorageCommand::Read { stick, name } = c {
            let bytes = files.iter().find(|(n, _)| *n == name).unwrap().1.clone();
            app.storage(StorageEvent::Read { stick, name, bytes });
        }
    }
    app.storage(StorageEvent::Sticks(Vec::new()));
    let _ = app.frame();
}

/// The test vault unlocked with Secure Boot keys made in it, after a
/// stick visit copied `files` in; its images shown.
fn with_keys(files: &[(String, Vec<u8>)]) -> Faraday {
    let mut app = Faraday::new();
    display(&mut app);
    let vault = testkit::files()
        .unwrap()
        .into_iter()
        .find(|(n, _)| n == "vault.fvlt")
        .unwrap();
    app.storage(StorageEvent::Restored {
        inbox: vec![vault],
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    app.storage(StorageEvent::Clock { unix_secs: CLOCK });
    testkit::vault_speed(&mut app, Some(180));
    visit(&mut app, files);
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Open(0)));
    type_text(&mut app, testkit::VAULT_PASSPHRASES[0]);
    app.press(Action::Vault(V::Unlock));
    settle(&mut app, 1);
    app.press(Action::Vault(V::LoadTicked));
    assert_eq!(
        app.vaults().open.len(),
        1,
        "{:?}",
        app.vaults().unlock_error
    );
    app.press(Action::OpenSetTime);
    app.press(Action::SetTimeKeep);
    crate::common::make_released(&mut app, Kind::SecureBoot, &["Test"]);
    app.outbox.clear();
    app.press(Action::Vault(V::SbImages));
    app
}

/// Picks the boot program and holds to sign; returns what the Outbox
/// holds under its name.
fn pick_and_hold(app: &mut Faraday) -> Option<Vec<u8>> {
    let k = app
        .inbox()
        .iter()
        .position(|i| i.name == BOOT)
        .expect("the boot program was copied in");
    app.press(Action::Vault(V::SbPick(k)));
    let _ = app.frame();
    app.press(Action::Vault(V::SbHoldSign));
    app.outbox
        .iter()
        .find(|i| i.name == BOOT)
        .map(|i| i.bytes.clone())
}

fn db_cert(app: &Faraday) -> Vec<u8> {
    app.vaults()
        .records(app.vaults().open[0].id)
        .of(8)
        .find_map(|(_, r)| r.public(faraday_keys::vault::sb_field::DB_CERT))
        .expect("the keys read back")
        .to_vec()
}

#[test]
fn a_release_boot_file_with_genuine_signed_sums_is_signed() {
    let mut app = with_keys(&release());
    let signed = pick_and_hold(&mut app).expect("the signed boot program in the Outbox");
    assert_eq!(
        faraday_sb::pe::check(&signed, &db_cert(&app)),
        Ok(faraday_sb::Check::Valid)
    );
}

/// Each refusal: the page names it before the hold, and the hold signs
/// nothing.
fn refused(files: &[(String, Vec<u8>)], why: &str) {
    let mut app = with_keys(files);
    let k = app.inbox().iter().position(|i| i.name == BOOT).unwrap();
    app.press(Action::Vault(V::SbPick(k)));
    crate::common::expect_wrapped(&mut app, why);
    assert!(pick_and_hold(&mut app).is_none(), "{why}: signed");
    assert!(
        !app.outbox.iter().any(|i| i.name == BOOT),
        "{why}: something reached the Outbox"
    );
}

#[test]
fn without_the_release_sums_nothing_is_signed() {
    refused(
        &release()[..1],
        "No signed sums from a Faraday release are beside this file",
    );
    refused(
        &release()[..2],
        "No signed sums from a Faraday release are beside this file",
    );
}

#[test]
fn sums_signed_by_another_key_sign_nothing() {
    let mut files = release();
    files[2].1 = fixture("SHA256SUMS.other.asc");
    refused(
        &files,
        "The release's sums are not signed by Faraday's release key",
    );
}

#[test]
fn tampered_sums_sign_nothing() {
    let mut files = release();
    files[1]
        .1
        .extend_from_slice(format!("{}  {BOOT}\n", "00".repeat(32)).as_bytes());
    refused(
        &files,
        "The release's sums are not signed by Faraday's release key",
    );
}

#[test]
fn a_boot_file_the_sums_do_not_list_signs_nothing() {
    let mut files = release();
    files[0].1[0x200] = 0x90;
    refused(&files, "This file is not the boot program of release 0.9.0");
}

#[test]
fn a_genuine_release_older_than_this_faraday_signs_nothing() {
    let mut files = release();
    files[1].1 = fixture("variants/older.sums");
    files[2].1 = fixture("variants/older.sums.asc");
    refused(
        &files,
        &format!(
            "Release 0.1.0 is older than this Faraday ({}); sign a release at least as new",
            env!("CARGO_PKG_VERSION")
        ),
    );
}

#[test]
fn a_file_that_is_not_an_efi_image_signs_nothing() {
    let mut files = release();
    files[0].1 = b"not a boot program".to_vec();
    refused(&files, "This file is not an EFI image");
}

#[test]
fn the_test_kits_boot_program_is_the_fixtures() {
    assert_eq!(testkit::test_efi(), fixture("BOOTX64.EFI"));
}

#[test]
fn the_signed_boot_program_is_said_to_wait_for_the_stick() {
    let mut app = with_keys(&release());
    pick_and_hold(&mut app).expect("the signed boot program in the Outbox");
    assert_eq!(
        app.toast_text(),
        Some(format!("{BOOT}, signed, waits for the stick").as_str())
    );
}

#[test]
fn the_enrolment_files_are_counted_in_a_sentence() {
    let mut app = with_keys(&release());
    app.outbox.clear();
    app.press(Action::Vault(V::SbEnrol));
    let n = app.outbox.len();
    assert!(n > 1, "{:?}", app.toast_text());
    assert_eq!(
        app.toast_text(),
        Some(format!("{n} enrolment files wait for the stick").as_str())
    );
}
