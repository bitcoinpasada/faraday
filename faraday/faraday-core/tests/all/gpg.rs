//! A GPG key signs any file a stick visit copied in: a release's
//! `SHA256SUMS` with no extension, a tag's text and a binary Faraday reads
//! as nothing else, each signature to the Outbox beside the file's name.
//! What it does follows GnuPG (F32): the revocation certificate is kept
//! with the key, a key that expired, was revoked or is not yet made signs
//! nothing, a renewal counts its term from the clock, and what is signed
//! is what the page shows.

use crate::common::type_text;
use std::path::{Path, PathBuf};
use std::process::Command;

use faraday_core::backup_flow::Kind;
use faraday_core::backup_flow::steps::FlowAction as F;
use faraday_core::gpg;
use faraday_core::testkit;
use faraday_core::vault::VaultAction as V;
use faraday_core::wallet::{FileKind, classify};
use faraday_core::{Action, Faraday, Item, Screen, Sheet, StickInfo, StorageCommand, StorageEvent};
use faraday_pgp::{Armor, Hash, armor, armor_with};
use faraday_shell_api::{App, BootState, DisplayInfo, EntropyBytes, Event, Key, SecureHardware};
use faraday_vault::records::kind;

/// The clock the tests start at: October 2026.
const CLOCK: u64 = 1_791_482_652;
/// GnuPG's year, 365 days.
const YEAR: u32 = 31_536_000;

fn settle(app: &mut Faraday, from: u64) {
    for t in from..from + 20 {
        let _ = app.frame();
        app.event(Event::Tick { now_ms: t * 1000 });
    }
}

/// What a release is signed from, and a file of no kind Faraday knows.
fn release_files() -> Vec<(String, Vec<u8>)> {
    vec![
        (
            "SHA256SUMS".into(),
            b"# Faraday 0.1.0, commit 7fcc43a\n3d6f  faraday-0.1.0-linux-x86_64\n".to_vec(),
        ),
        (
            "v0.1.0.tag".into(),
            b"object 7fcc43a\ntype commit\ntag v0.1.0\n\nFaraday 0.1.0\n".to_vec(),
        ),
        (
            "build.tar.gz".into(),
            vec![0x1f, 0x8b, 0x08, 0x00, 0xff, 0x00],
        ),
    ]
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
    // The answers a shell gives to the requests for randomness.
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
    // Every file comes in ticked.
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

/// The app with the test vault in the Inbox and the release files copied
/// in on a stick visit, the stick then pulled.
fn copied_in() -> Faraday {
    let mut app = Faraday::new();
    display(&mut app);
    let vault = testkit::files()
        .unwrap()
        .into_iter()
        .find(|(n, _)| n == "vault.ofv")
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
    faraday_core::testkit::vault_speed(&mut app, Some(180));
    visit(&mut app, &release_files());
    app
}

#[test]
fn a_stick_visit_copies_in_files_of_no_kind_faraday_knows() {
    let app = copied_in();
    assert!(
        app.visit().log.iter().all(|(_, ok)| *ok),
        "{:?}",
        app.visit().log
    );
    for (name, bytes) in release_files() {
        let item = app.inbox().iter().find(|i| i.name == name);
        assert_eq!(item.map(|i| &i.bytes[..]), Some(&bytes[..]), "{name}");
    }
}

/// The app with the test vault unlocked on its GPG keys, after the stick
/// visit.
fn unlocked() -> Faraday {
    let mut app = copied_in();
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
    app.press(Action::Nav(Screen::VaultContents));
    app.press(Action::Vault(V::Category(4)));
    app
}

/// Settings' Set the time, then Keep this time: the clock checked, as a
/// person on the stick does before making or signing.
fn keep_the_time(app: &mut Faraday) {
    app.press(Action::OpenSetTime);
    app.press(Action::SetTimeKeep);
}

/// A GPG key made in the open vault by the flow's Make (the flow itself is
/// tested in `backup_flow.rs`), and released as a key unlocked from a
/// stick is: it signs at once.
fn make_key(app: &mut Faraday, name: &str) {
    crate::common::make_released(app, Kind::Gpg, &[name, ""]);
}

fn gpg_keys(app: &Faraday) -> usize {
    app.vaults()
        .store()
        .contents_for_tests(app.vaults().open[app.vaults().current].id)
        .of(kind::GPG)
        .count()
}

/// The record of the GPG key last made.
fn record(app: &Faraday) -> faraday_vault::Record {
    app.vaults()
        .store()
        .contents_for_tests(app.vaults().open[app.vaults().current].id)
        .of(kind::GPG)
        .last()
        .expect("a GPG key")
        .1
        .clone()
}

fn key_of(app: &Faraday) -> faraday_pgp::Key {
    crate::common::gpg_key_of(&record(app)).expect("a readable key")
}

/// The app with the test vault unlocked, a GPG key made, and the Sign a
/// file page open on the three files the stick visit copied in.
fn signing() -> Faraday {
    let mut app = unlocked();
    keep_the_time(&mut app);
    make_key(&mut app, "Release");
    app.press(Action::Vault(V::GpgSignPick));
    app
}

/// The listed position of Inbox file `name` on the signing page.
fn listed(app: &Faraday, name: &str) -> usize {
    app.inbox()
        .iter()
        .filter(|i| !i.name.ends_with(".asc"))
        .position(|i| i.name == name)
        .unwrap_or_else(|| panic!("no {name}"))
}

fn outbox_names(app: &Faraday) -> Vec<String> {
    app.outbox.iter().map(|i| i.name.clone()).collect()
}

/// Sign on the page, then the hold on Sign with {key}: the one way a
/// ticked file is signed (F63).
fn sign(app: &mut Faraday) {
    app.press(Action::Vault(V::GpgSign));
    assert_eq!(app.sheet, Some(Sheet::GpgSign), "Sign opens its sheet");
    app.press(Action::Vault(V::GpgHoldSign));
}

/// Everything drawn, as one string without spaces.
fn drawn(app: &mut Faraday) -> String {
    let _ = app.frame();
    app.drawn_texts().join("").replace(' ', "")
}

/// On the stick, with the clock not checked, Make a key opens Set the
/// time and makes nothing: a GPG key's creation time is part of its
/// fingerprint. After Keep this time, Save again makes it. The desktop
/// app, whose clock is the computer's, never asks.
#[test]
fn on_the_stick_make_a_key_asks_for_the_time_first() {
    let mut app = unlocked();
    let before = gpg_keys(&app);
    make_key(&mut app, "Release");
    assert_eq!(app.screen, Screen::SetTime);
    assert_eq!(gpg_keys(&app), before, "nothing made");
    app.press(Action::SetTimeKeep);
    assert_eq!(app.screen, Screen::BackupFlow, "back on the same page");
    app.press(Action::Flow(F::Make));
    crate::common::settle_n(&mut app, 300, 2);
    assert_eq!(gpg_keys(&app), before + 1);

    let mut app = unlocked();
    app.online = true;
    make_key(&mut app, "Release");
    assert_eq!(
        app.screen,
        Screen::VaultContents,
        "the desktop app never asks"
    );
    assert_eq!(gpg_keys(&app), before + 1);
}

/// On the stick, with the clock not checked, Sign opens Sign with {key}
/// and asks nothing: the sheet lists the file chosen and the time the
/// signature will carry, and nothing is signed until its hold (F63).
#[test]
fn on_the_stick_sign_opens_the_sheet_with_the_time_and_signs_nothing() {
    let mut app = unlocked();
    app.online = true;
    make_key(&mut app, "Release");
    app.online = false;
    app.press(Action::Vault(V::GpgSignPick));
    app.press(Action::Vault(V::GpgTick(listed(&app, "SHA256SUMS"))));
    let files = app.outbox.len();
    app.press(Action::Vault(V::GpgSign));
    assert_eq!(app.screen, Screen::VaultContents, "no Set the time");
    assert_eq!(app.sheet, Some(Sheet::GpgSign));
    assert_eq!(app.outbox.len(), files, "nothing signed");
    let page = drawn(&mut app);
    assert!(page.contains("SignwithRelease"), "{page}");
    assert!(page.contains("SHA256SUMS·"), "the file chosen: {page}");
    assert!(!page.contains("v0.1.0.tag·"), "a file not chosen: {page}");
    assert!(page.contains("Signaturetime"), "{page}");
    let now = app.now_local_text().replace(' ', "");
    assert!(page.contains(&now), "the clock's time: {page}");
    assert!(page.contains("Holdtosign1file"), "{page}");
    app.press(Action::Vault(V::GpgHoldSign));
    assert_eq!(app.sheet, None, "the sheet closes");
    assert_eq!(app.outbox.len(), files + 1, "one signature");
    assert!(app.outbox.iter().any(|i| i.name == "SHA256SUMS.asc"));
    assert_eq!(app.toast_text(), Some("1 signature waits for the stick"));
}

/// Set the time from the sheet returns to it, the files still chosen, and
/// signs nothing by itself; the hold then signs once, at the time set
/// (F63).
#[test]
fn set_the_time_from_the_sheet_returns_to_it_and_signs_nothing() {
    let mut app = signing();
    for name in ["SHA256SUMS", "v0.1.0.tag"] {
        app.press(Action::Vault(V::GpgTick(listed(&app, name))));
    }
    let files = app.outbox.len();
    app.press(Action::Vault(V::GpgSign));
    app.press(Action::OpenSetTime);
    assert_eq!(app.screen, Screen::SetTime);
    assert_eq!(app.sheet, None, "Set the time is not under the sheet");
    app.event(Event::Key(Key::Tab));
    for _ in 0..10 {
        app.event(Event::Key(Key::Backspace));
    }
    type_text(&mut app, "2026-10-21");
    app.event(Event::Key(Key::Tab));
    type_text(&mut app, "09:30");
    app.press(Action::SetTimeSet);
    assert_eq!(app.screen, Screen::VaultContents);
    assert_eq!(app.sheet, Some(Sheet::GpgSign), "back on the sheet");
    assert_eq!(app.outbox.len(), files, "returning signs nothing");
    assert_eq!(app.vaults().sign_ticked.len(), 2, "the files still chosen");
    // Back and Keep this time return to it too, and sign nothing.
    app.press(Action::OpenSetTime);
    app.press(Action::SetTimeBack);
    assert_eq!(app.sheet, Some(Sheet::GpgSign));
    app.press(Action::OpenSetTime);
    app.press(Action::SetTimeKeep);
    assert_eq!(app.sheet, Some(Sheet::GpgSign));
    assert_eq!(app.outbox.len(), files, "nothing signed");
    app.press(Action::Vault(V::GpgHoldSign));
    assert_eq!(app.outbox.len(), files + 2, "one signature per file");
    // The time set is the one signed at.
    let at = u32::try_from(app.clock_now().unwrap()).unwrap();
    let key = key_of(&app);
    let want = armor(
        Armor::Signature,
        &key.sign(&release_files()[0].1, at, Hash::Sha512),
    );
    let sig = app
        .outbox
        .iter()
        .find(|i| i.name == "SHA256SUMS.asc")
        .unwrap();
    assert_eq!(String::from_utf8_lossy(&sig.bytes).trim(), want.trim());
    // A second hold has no sheet to act on.
    app.press(Action::Vault(V::GpgHoldSign));
    assert_eq!(app.outbox.len(), files + 2, "signed once");
}

/// With the clock not known (the Pi before its time is set), the sheet
/// says so and its hold signs nothing until the time is set.
#[test]
fn with_the_clock_not_known_the_sheet_says_so_and_the_hold_signs_nothing() {
    let mut app = signing();
    app.press(Action::Vault(V::GpgTick(listed(&app, "SHA256SUMS"))));
    // The Pi's clock at power-on: 1970.
    app.storage(StorageEvent::Clock { unix_secs: 10 });
    assert_eq!(app.clock_now(), None);
    let files = app.outbox.len();
    app.press(Action::Vault(V::GpgSign));
    assert_eq!(app.sheet, Some(Sheet::GpgSign));
    let page = drawn(&mut app);
    assert!(page.contains("Signaturetime"), "{page}");
    assert!(page.contains("Notknown"), "{page}");
    app.press(Action::Vault(V::GpgHoldSign));
    assert_eq!(app.outbox.len(), files, "nothing signed");
    assert_eq!(app.sheet, Some(Sheet::GpgSign), "the sheet stays");
    app.press(Action::OpenSetTime);
    // The date is empty, the time field focused: Tab to the date.
    app.event(Event::Key(Key::Tab));
    type_text(&mut app, "2026-10-21");
    app.event(Event::Key(Key::Tab));
    type_text(&mut app, "09:30");
    app.press(Action::SetTimeSet);
    assert_eq!(app.sheet, Some(Sheet::GpgSign));
    assert_eq!(app.outbox.len(), files, "setting the time signs nothing");
    app.press(Action::Vault(V::GpgHoldSign));
    assert_eq!(app.outbox.len(), files + 1);
}

/// Cancel closes the sheet and signs nothing; the file stays chosen. The
/// hold signs nothing with no sheet open, and Sign opens none with no
/// file chosen.
#[test]
fn cancel_signs_nothing_and_only_the_sheets_hold_signs() {
    let mut app = signing();
    let files = app.outbox.len();
    app.press(Action::Vault(V::GpgSign));
    assert_eq!(app.sheet, None, "no file chosen");
    app.press(Action::Vault(V::GpgTick(listed(&app, "SHA256SUMS"))));
    app.press(Action::Vault(V::GpgHoldSign));
    assert_eq!(app.outbox.len(), files, "no sheet, nothing signed");
    app.press(Action::Vault(V::GpgSign));
    assert_eq!(app.sheet, Some(Sheet::GpgSign));
    app.press(Action::Cancel);
    assert_eq!(app.sheet, None);
    assert_eq!(app.outbox.len(), files, "nothing signed");
    assert_eq!(app.vaults().sign_ticked.len(), 1, "still chosen");
    assert!(app.vaults().signing, "the page stays");
}

/// A new key's term is GnuPG's default for `--quick-generate-key`, three
/// years of 365 days, and the revocation certificate is kept with the key:
/// it is not in the Outbox, in a file or in any other form.
#[test]
fn making_a_key_keeps_its_revocation_with_it_and_not_in_the_outbox() {
    let mut app = unlocked();
    keep_the_time(&mut app);
    make_key(&mut app, "Release");
    assert_eq!(app.vaults().gpg_years, 3);
    let key = key_of(&app);
    assert_eq!(key.expiry, 3 * YEAR);
    let r = record(&app);
    let rev = r.field(8).expect("a kept revocation").to_vec();
    assert_eq!(
        rev,
        key.revocation(key.created, faraday_pgp::Reason::Unspecified, "")
    );
    // The Outbox has the certificate and nothing else.
    let names = outbox_names(&app);
    assert_eq!(names.len(), 1, "{names:?}");
    assert!(names[0].ends_with(".asc") && !names[0].contains("revocation"));
    let rev_text = armor(Armor::PublicKey, &rev);
    for item in &app.outbox {
        assert_ne!(item.bytes, rev_text.as_bytes());
        assert!(
            !String::from_utf8_lossy(&item.bytes).contains("revocation certificate"),
            "{}",
            item.name
        );
    }
    // The result says where it is kept.
    assert!(
        app.toast_text()
            .is_some_and(|t| t.contains("kept with the key")),
        "{:?}",
        app.toast_text()
    );
}

/// A key made before revocations were kept gets its one on first use of
/// its page, with the clock as it is then.
#[test]
fn a_key_with_no_kept_revocation_makes_one_when_its_page_is_opened() {
    let mut app = unlocked();
    keep_the_time(&mut app);
    make_key(&mut app, "Release");
    let at = app
        .vaults()
        .store()
        .contents_for_tests(app.vaults().open[app.vaults().current].id)
        .of(kind::GPG)
        .last()
        .unwrap()
        .0;
    let id = app.vaults().open[app.vaults().current].id;
    faraday_core::testkit::vault_edit(&mut app, id, |s| {
        s.set_secret(id, at, 8, &zeroize::Zeroizing::new(Vec::new()))
    });
    assert!(record(&app).field(8).is_none());
    app.press(Action::Vault(V::Item(0)));
    assert!(record(&app).field(8).is_some());
}

/// A scratch GnuPG home; GnuPG 2.4 is on the build machine, and a machine
/// without it fails the test unless `FARADAY_SKIP_GNUPG=1`.
struct Home(PathBuf);

impl Drop for Home {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn home(name: &str) -> Home {
    let dir = std::env::temp_dir().join(format!("faraday-core-gpg-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700)).unwrap();
    }
    Home(dir)
}

fn gpg_at(home: &Path, now: u64, args: &[&str]) -> Option<(bool, String)> {
    let out = Command::new("gpg")
        .arg("--homedir")
        .arg(home)
        .args(["--batch", "--no-tty", "--status-fd", "1"])
        .arg("--faked-system-time")
        .arg(format!("{now}!"))
        .args(args)
        .output();
    match out {
        Ok(o) => Some((
            o.status.success(),
            String::from_utf8_lossy(&o.stdout).into_owned() + &String::from_utf8_lossy(&o.stderr),
        )),
        Err(_) if std::env::var("FARADAY_SKIP_GNUPG").is_ok_and(|v| v == "1") => None,
        Err(e) => panic!("gpg is not installed ({e}); set FARADAY_SKIP_GNUPG=1 to skip"),
    }
}

/// The revocation certificate leaves only through the secret sheet, as
/// `{name}-{FPR}.rev` in GnuPG's file layout, with the colon that keeps
/// `gpg --import` from taking it until it is removed by hand.
#[test]
fn the_revocation_is_exported_through_the_secret_sheet_as_a_rev_file() {
    let c = testkit::compute();
    let mut app = unlocked();
    keep_the_time(&mut app);
    make_key(&mut app, "Release");
    let key = key_of(&app);
    let fpr: String = key
        .fingerprint()
        .iter()
        .map(|b| format!("{b:02X}"))
        .collect();
    let cert = app.outbox[0].bytes.clone();
    app.press(Action::Vault(V::GpgExportRevocation));
    assert_eq!(app.sheet, Some(Sheet::SecretOut));
    assert_eq!(
        testkit::secret_out(&app).map(|o| o.name.clone()),
        Some(format!("release-{fpr}.rev"))
    );
    // Nothing is in the Outbox until the person says they understand.
    assert_eq!(app.outbox.len(), 1);
    app.press(Action::SecretUnprotected);
    app.press(Action::SecretHold);
    let item = app
        .outbox
        .iter()
        .find(|i| i.name == format!("release-{fpr}.rev"))
        .expect("the .rev file");
    assert!(item.secret, "listed with the unprotected secrets");
    let text = String::from_utf8(item.bytes.clone()).unwrap();
    assert!(text.starts_with("This is a revocation certificate for the OpenPGP key:"));
    assert!(text.contains(
        "\n:-----BEGIN PGP PUBLIC KEY BLOCK-----\nComment: This is a revocation certificate\n"
    ));
    // Never public: not a file Select all takes, and no code or picture.
    assert_ne!(classify(&c, "x.rev", text.as_bytes()), FileKind::Gpg);
    assert!(item.exposure() == faraday_core::secrets::Exposure::Secret);

    // GnuPG: refuses until the colon is removed, accepts after.
    let h = home("rev");
    let created = u64::from(key.created);
    let c = h.0.join("cert.asc");
    std::fs::write(&c, &cert).unwrap();
    let Some((ok, said)) = gpg_at(&h.0, created + 3600, &["--import", c.to_str().unwrap()]) else {
        return;
    };
    assert!(ok, "{said}");
    let rev = h.0.join("key.rev");
    std::fs::write(&rev, &text).unwrap();
    let (_, said) = gpg_at(&h.0, created + 3600, &["--import", rev.to_str().unwrap()]).unwrap();
    assert!(!said.contains("revocation certificate imported"), "{said}");
    std::fs::write(&rev, text.replacen("\n:-----BEGIN", "\n-----BEGIN", 1)).unwrap();
    let (ok, said) = gpg_at(&h.0, created + 3600, &["--import", rev.to_str().unwrap()]).unwrap();
    assert!(
        ok && said.contains("revocation certificate imported"),
        "{said}"
    );
}

/// A new revocation asks the reason as GnuPG does, codes 0 to 3 and an
/// optional text, and goes out the same way; the one kept stays as it was.
#[test]
fn a_new_revocation_asks_the_reason_as_gnupg_does() {
    let mut app = unlocked();
    keep_the_time(&mut app);
    make_key(&mut app, "Release");
    let kept = record(&app).field(8).unwrap().to_vec();
    app.press(Action::Vault(V::GpgRevocationPage));
    assert!(app.vaults().form.is_some());
    // GnuPG's menu: 1 is "compromised", which the packet codes 2.
    app.press(Action::Vault(V::GpgReason(1)));
    app.press(Action::Vault(V::FocusField(0)));
    type_text(&mut app, "My key was stolen");
    app.press(Action::Vault(V::FormSave));
    assert_eq!(app.sheet, Some(Sheet::SecretOut));
    let out = testkit::secret_out(&app).unwrap();
    let text = String::from_utf8(out.bytes.to_vec()).unwrap();
    assert!(text.contains(":-----BEGIN PGP PUBLIC KEY BLOCK-----"));
    assert_eq!(
        record(&app).field(8).unwrap(),
        &kept[..],
        "the kept one stays"
    );
    let want = key_of(&app).revocation(
        u32::try_from(app.clock_now().unwrap()).unwrap(),
        faraday_pgp::Reason::Compromised,
        "My key was stolen",
    );
    let want = armor_with(
        Armor::PublicKey,
        &want,
        Some("This is a revocation certificate"),
    );
    let armoured = text.split_once("\n:").unwrap().1;
    assert_eq!(armoured.trim(), want.trim());
}

/// Nothing secret reaches the Outbox through any of the key's actions.
#[test]
fn no_seed_and_no_paperkey_reaches_the_outbox() {
    let mut app = signing();
    app.press(Action::Vault(V::GpgTickAll));
    sign(&mut app);
    app.press(Action::Vault(V::GpgExport));
    app.press(Action::Vault(V::GpgRenewTerm(2)));
    app.storage(StorageEvent::Clock {
        unix_secs: CLOCK + 86_400,
    });
    app.press(Action::Vault(V::GpgRenew));
    let key = key_of(&app);
    let paper = key.paperkey();
    let first = paper.lines().next().unwrap().trim().to_string();
    assert!(app.outbox.len() >= 4, "{:?}", outbox_names(&app));
    for item in &app.outbox {
        let text = String::from_utf8_lossy(&item.bytes).to_string();
        for seed in [key.primary_seed, key.subkey_seed] {
            let hexed: String = seed.iter().map(|b| format!("{b:02x}")).collect();
            assert!(!item.bytes.windows(32).any(|w| w == seed), "{}", item.name);
            assert!(!text.to_lowercase().contains(&hexed), "{}", item.name);
        }
        assert!(!text.contains(&first), "{}", item.name);
    }
}

/// Sign a file is a checkbox list: ticking two files and holding Sign 2
/// files puts two signatures in the Outbox in one go, each named
/// `{file}.asc` as `gpg --armor --detach-sign` names it, and the page
/// stays open.
#[test]
fn ticking_two_files_and_signing_puts_two_signatures_in_the_outbox() {
    let mut app = signing();
    let names: Vec<String> = release_files().into_iter().map(|(n, _)| n).collect();
    for n in names.iter().take(2) {
        let k = listed(&app, n);
        app.press(Action::Vault(V::GpgTick(k)));
    }
    assert!(app.screen != Screen::Home, "still on the vault");
    sign(&mut app);
    assert!(app.vaults().signing, "the page stays open");
    for name in names.iter().take(2) {
        let sig = app
            .outbox
            .iter()
            .find(|i| i.name == format!("{name}.asc"))
            .unwrap_or_else(|| panic!("no signature for {name}"));
        assert!(
            sig.bytes.starts_with(b"-----BEGIN PGP SIGNATURE-----"),
            "{name}"
        );
    }
    assert!(
        !app.outbox
            .iter()
            .any(|i| i.name == format!("{}.asc", names[2])),
        "the third file was never ticked"
    );
    assert!(app.vaults().sign_ticked.is_empty(), "the ticks clear");
    assert_eq!(app.toast_text(), Some("2 signatures wait for the stick"));
}

/// The signature is GnuPG-checkable: it verifies against its file with
/// the key's certificate.
#[test]
fn a_signature_verifies_in_gnupg() {
    let mut app = signing();
    let k = listed(&app, "SHA256SUMS");
    app.press(Action::Vault(V::GpgTick(k)));
    sign(&mut app);
    let sig = app
        .outbox
        .iter()
        .find(|i| i.name == "SHA256SUMS.asc")
        .unwrap()
        .bytes
        .clone();
    let cert = app
        .outbox
        .iter()
        .find(|i| i.name != "SHA256SUMS.asc")
        .unwrap();
    let h = home("verify");
    let now = u64::from(key_of(&app).created) + 600;
    std::fs::write(h.0.join("cert.asc"), &cert.bytes).unwrap();
    std::fs::write(h.0.join("SHA256SUMS"), &release_files()[0].1).unwrap();
    std::fs::write(h.0.join("SHA256SUMS.asc"), &sig).unwrap();
    let Some((ok, said)) = gpg_at(
        &h.0,
        now,
        &["--import", h.0.join("cert.asc").to_str().unwrap()],
    ) else {
        return;
    };
    assert!(ok, "{said}");
    let (ok, said) = gpg_at(
        &h.0,
        now,
        &[
            "--verify",
            h.0.join("SHA256SUMS.asc").to_str().unwrap(),
            h.0.join("SHA256SUMS").to_str().unwrap(),
        ],
    )
    .unwrap();
    assert!(
        ok && said.contains("GOODSIG") && said.contains("VALIDSIG"),
        "{said}"
    );
}

/// What is signed is what the page shows: each file's name, size and
/// SHA-256, and a small text file's text; the key named by user ID, key
/// IDs and creation date, and its signing subkey.
#[test]
fn the_page_shows_the_bytes_and_the_key_it_signs_with() {
    let c = faraday_core::testkit::compute();
    let mut app = signing();
    let page = drawn(&mut app);
    for (name, bytes) in release_files() {
        let hash = gpg::sha256_hex(&c, &bytes);
        assert!(page.contains(&hash), "{name}'s SHA-256 on the page");
        assert!(page.contains(&name.replace(' ', "")), "{name}");
    }
    // A small text file's text, and not a binary's.
    assert!(page.contains("object7fcc43a"), "the tag's text");
    assert!(
        page.contains("faraday-0.1.0-linux-x86_64"),
        "the sums' text"
    );
    let key = key_of(&app);
    let sub = gpg::key_id(&key.subkey_fingerprint());
    let main = gpg::key_id(&key.fingerprint());
    assert!(page.contains("\"Release\""));
    assert!(page.contains(&format!("subkeyID{sub}")), "{page}");
    assert!(page.contains(&format!("mainkeyID{main}")));
    assert!(page.contains("created"));
}

/// Signing takes a hold: a press on the button and a release before the
/// hold is up signs nothing.
#[test]
fn signing_takes_a_hold() {
    assert!(faraday_core::vault::is_hold(Action::Vault(V::GpgHoldSign)));
    assert!(faraday_core::vault::is_hold(Action::Vault(
        V::GpgHoldRevoke
    )));
}

/// The ticks are keyed by name and hash: a stick visit that replaces a
/// ticked file's bytes while Sign with {key} is open leaves it unticked,
/// and the hold signs nothing.
#[test]
fn a_stick_visit_that_replaces_a_ticked_file_drops_its_tick() {
    let c = faraday_core::testkit::compute();
    let mut app = signing();
    let k = listed(&app, "SHA256SUMS");
    app.press(Action::Vault(V::GpgTick(k)));
    assert_eq!(app.vaults().sign_ticked.len(), 1);
    app.press(Action::Vault(V::GpgSign));
    assert_eq!(app.sheet, Some(Sheet::GpgSign));
    let changed = (
        "SHA256SUMS".to_string(),
        b"# Faraday 0.1.0, commit 7fcc43a\n0000  faraday-0.1.0-linux-x86_64\n".to_vec(),
    );
    // What a stick visit's Copy answers with: the file read from a stick.
    app.storage(StorageEvent::Read {
        stick: "S".into(),
        name: changed.0.clone(),
        bytes: changed.1.clone(),
    });
    let item = app.inbox().iter().find(|i| i.name == "SHA256SUMS").unwrap();
    assert_eq!(
        item.bytes, changed.1,
        "the stick's file is the one in the Inbox"
    );
    // The sheet lists no file to sign.
    let sheet = drawn(&mut app);
    assert!(!sheet.contains("Holdtosign1file"), "{sheet}");
    let before = app.outbox.len();
    app.press(Action::Vault(V::GpgHoldSign));
    assert_eq!(app.outbox.len(), before, "nothing signed");
    assert!(
        app.toast_text().is_some_and(|t| t.contains("not signed")),
        "{:?}",
        app.toast_text()
    );
    // The page shows it unticked, with the new hash.
    let page = drawn(&mut app);
    assert!(page.contains(&gpg::sha256_hex(&c, &changed.1)), "{page}");
    // Nothing is ticked now: Sign opens no sheet.
    app.press(Action::Vault(V::GpgSign));
    assert_eq!(app.sheet, None);
}

/// Changing the key selected, the category or the vault clears the
/// signing page and its ticks.
#[test]
fn changing_the_key_the_category_or_the_vault_clears_the_signing_page() {
    let mut app = unlocked();
    keep_the_time(&mut app);
    make_key(&mut app, "Release");
    make_key(&mut app, "Second");
    for change in [
        Action::Vault(V::Item(0)),
        Action::Vault(V::Category(2)),
        Action::Vault(V::Show(0)),
    ] {
        app.press(Action::Vault(V::Category(4)));
        app.press(Action::Vault(V::Item(1)));
        app.press(Action::Vault(V::GpgSignPick));
        app.press(Action::Vault(V::GpgTick(0)));
        assert!(app.vaults().signing && !app.vaults().sign_ticked.is_empty());
        app.press(change);
        assert!(!app.vaults().signing, "{change:?}");
        assert!(app.vaults().sign_ticked.is_empty(), "{change:?}");
    }
}

/// The signature already made is marked; an `.asc` copied back in is not
/// listed to sign again.
#[test]
fn a_signed_file_is_marked_and_an_asc_in_the_inbox_is_not_listed() {
    let mut app = signing();
    let name = &release_files()[0].0;
    let k = listed(&app, name);
    app.press(Action::Vault(V::GpgTick(k)));
    sign(&mut app);
    let _ = app.frame();
    let texts = app.drawn_texts();
    assert!(
        texts.iter().any(|t| t == "Signed"),
        "the signed row is marked: {texts:?}"
    );
    testkit::inbox_push(
        &mut app,
        Item {
            name: format!("{name}.asc"),
            bytes: b"-----BEGIN PGP SIGNATURE-----\n...".to_vec(),
            kind: FileKind::Other,
            secret: false,
            picture: None,
            sha: None,
            wallet: None,
            psbt: None,
        },
    );
    assert!(
        app.inbox()
            .iter()
            .filter(|i| !i.name.ends_with(".asc"))
            .all(|x| x.name != format!("{name}.asc")),
        "an .asc file is not itself signable"
    );
}

/// With one file only in the Inbox, it starts ticked and the hold signs
/// it.
#[test]
fn with_one_file_it_starts_ticked() {
    let c = faraday_core::testkit::compute();
    let mut app = Faraday::new();
    display(&mut app);
    let vault = testkit::files()
        .unwrap()
        .into_iter()
        .find(|(n, _)| n == "vault.ofv")
        .unwrap();
    app.storage(StorageEvent::Restored {
        inbox: vec![vault, ("SHA256SUMS".into(), b"hello".to_vec())],
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    app.storage(StorageEvent::Clock { unix_secs: CLOCK });
    faraday_core::testkit::vault_speed(&mut app, Some(180));
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Open(0)));
    type_text(&mut app, testkit::VAULT_PASSPHRASES[0]);
    app.press(Action::Vault(V::Unlock));
    settle(&mut app, 1);
    app.press(Action::Vault(V::LoadTicked));
    app.press(Action::Nav(Screen::VaultContents));
    app.press(Action::Vault(V::Category(4)));
    keep_the_time(&mut app);
    make_key(&mut app, "Release");
    // The vault's own file stays in the Inbox once opened; drop it so
    // SHA256SUMS is the one signable file.
    testkit::inbox_retain(&mut app, |i| i.kind != FileKind::Vault);
    app.press(Action::Vault(V::GpgSignPick));
    assert_eq!(
        app.vaults().sign_ticked,
        vec![("SHA256SUMS".to_string(), gpg::sha256(&c, b"hello"))]
    );
    sign(&mut app);
    assert!(
        app.outbox.iter().any(|i| i.name == "SHA256SUMS.asc"),
        "the hold signed the one ticked file"
    );
}

/// A signature is not written over a file of its name: a file named like
/// the certificate would have its signature replace the certificate, and
/// GnuPG's batch mode does not overwrite either. Nothing signed is not
/// reported as a signature waiting.
#[test]
fn a_signature_never_overwrites_the_certificate() {
    let mut app = signing();
    let cert = app.outbox[0].clone();
    let stem = cert.name.trim_end_matches(".asc").to_string();
    testkit::inbox_push(
        &mut app,
        Item {
            name: stem.clone(),
            bytes: b"the file named like the certificate".to_vec(),
            kind: FileKind::Text,
            secret: false,
            picture: None,
            sha: None,
            wallet: None,
            psbt: None,
        },
    );
    app.press(Action::Vault(V::GpgSignPick));
    app.press(Action::Vault(V::GpgSignPick));
    let k = listed(&app, &stem);
    app.press(Action::Vault(V::GpgTick(k)));
    sign(&mut app);
    let now = app.outbox.iter().find(|i| i.name == cert.name).unwrap();
    assert_eq!(now.bytes, cert.bytes, "the certificate is as it was");
    let toast = app.toast_text().unwrap().to_string();
    assert!(toast.starts_with("File exists"), "{toast}");
    assert!(!toast.contains("signature"), "{toast}");
}

/// A key that expired, was revoked, or was made after the clock signs
/// nothing; GnuPG's "Unusable secret key", and for a clock before the
/// key's creation its own words.
#[test]
fn an_expired_a_revoked_and_a_future_key_sign_nothing() {
    // Expired: the clock past the key's term.
    let mut app = signing();
    let key = key_of(&app);
    app.press(Action::Vault(V::GpgTick(listed(&app, "SHA256SUMS"))));
    app.storage(StorageEvent::Clock {
        unix_secs: u64::from(key.created) + u64::from(key.expiry) + 1,
    });
    let before = app.outbox.len();
    sign(&mut app);
    assert_eq!(app.outbox.len(), before);
    let t = app.toast_text().unwrap();
    assert!(t.starts_with("Unusable secret key: it expired on "), "{t}");

    // Revoked.
    let mut app = signing();
    app.press(Action::Vault(V::GpgHoldRevoke));
    app.press(Action::Vault(V::GpgTick(listed(&app, "SHA256SUMS"))));
    let before = app.outbox.len();
    sign(&mut app);
    assert_eq!(app.outbox.len(), before);
    let t = app.toast_text().unwrap();
    assert!(
        t.starts_with("Unusable secret key: it was revoked on "),
        "{t}"
    );

    // Made in the future: the clock a day and more before the key.
    let mut app = signing();
    app.press(Action::Vault(V::GpgTick(listed(&app, "SHA256SUMS"))));
    app.storage(StorageEvent::Clock {
        unix_secs: CLOCK - 3 * 86_400,
    });
    let before = app.outbox.len();
    sign(&mut app);
    assert_eq!(app.outbox.len(), before);
    let t = app.toast_text().unwrap();
    assert!(
        t.contains("was created 3 days in the future (time warp or clock problem)"),
        "{t}"
    );
}

/// The signature's code and picture follow the file by its hash: a file
/// that arrives in the Inbox ahead of it does not change which file the
/// button signs.
#[test]
fn the_signature_code_follows_the_file_not_its_place() {
    let c = faraday_core::testkit::compute();
    let mut app = signing();
    let code = app.signature_code(&c, "SHA256SUMS").unwrap();
    testkit::inbox_insert(
        &mut app,
        0,
        Item {
            name: "another".into(),
            bytes: b"another file".to_vec(),
            kind: FileKind::Text,
            secret: false,
            picture: None,
            sha: None,
            wallet: None,
            psbt: None,
        },
    );
    app.press(Action::ShowCode(code));
    let q = app.qr().expect("the QR sheet");
    assert!(q.title.contains("SHA256SUMS"), "{}", q.title);
    let key = key_of(&app);
    let now = u32::try_from(app.clock_now().unwrap()).unwrap();
    let want = armor(
        Armor::Signature,
        &key.sign(&release_files()[0].1, now, Hash::Sha512),
    );
    let png = faraday_codec::png::qr_png(&q.frames[0], 4);
    let payloads = faraday_files::qr_in_png(&c, &png).unwrap();
    assert_eq!(
        String::from_utf8(payloads[0].clone()).unwrap().trim(),
        want.trim()
    );
    // And once the file is gone, the code is not another file's.
    app.press(Action::Cancel);
    testkit::inbox_retain(&mut app, |i| i.name != "SHA256SUMS");
    testkit::qr_close(&mut app);
    app.press(Action::ShowCode(code));
    assert!(
        app.qr().is_none(),
        "no signature over a file that is not there"
    );
}

/// Renew asks for the term: nothing is renewed until one is chosen, and
/// the term is counted from the clock, in GnuPG's years of 365 days. The
/// record's certificate and the Export file change together.
#[test]
fn renewing_asks_the_term_and_updates_the_record_and_the_export_together() {
    let mut app = unlocked();
    keep_the_time(&mut app);
    make_key(&mut app, "Release");
    let before = record(&app);
    app.press(Action::Vault(V::GpgRenew));
    assert_eq!(app.toast_text(), Some("Choose the new term first"));
    assert_eq!(record(&app).field(7), before.field(7), "nothing renewed");

    // A year on, renewed for two years from then.
    let later = CLOCK + u64::from(YEAR);
    app.storage(StorageEvent::Clock { unix_secs: later });
    app.press(Action::Vault(V::GpgRenewTerm(2)));
    app.press(Action::Vault(V::GpgRenew));
    let key = key_of(&app);
    assert_eq!(
        u64::from(key.expiry),
        later - u64::from(key.created) + 2 * u64::from(YEAR)
    );
    let r = record(&app);
    let cert = armor(Armor::PublicKey, r.field(7).unwrap());
    assert_ne!(r.field(7), before.field(7));
    assert_eq!(r.field(8), before.field(8), "the kept revocation stays");
    app.outbox.clear();
    app.press(Action::Vault(V::GpgExport));
    assert_eq!(app.outbox.len(), 1);
    assert_eq!(
        app.outbox[0].bytes,
        cert.as_bytes(),
        "Export is the record's"
    );
    // The choice is spent.
    assert_eq!(app.vaults().renew_years, None);

    // Never is a choice too, and is only made by choosing it.
    app.storage(StorageEvent::Clock {
        unix_secs: later + 86_400,
    });
    app.press(Action::Vault(V::GpgRenewTerm(0)));
    app.press(Action::Vault(V::GpgRenew));
    assert_eq!(key_of(&app).expiry, 0);
}

/// A renewal at a clock before the key's last self-signature is
/// refused ("Time conflict"), as GnuPG refuses it, and one within four
/// seconds is signed one second after it.
#[test]
fn renewing_before_the_last_self_signature_is_refused() {
    let mut app = unlocked();
    keep_the_time(&mut app);
    make_key(&mut app, "Release");
    let signed_at = u64::from(key_of(&app).created);
    // Ten seconds before the key's own signatures.
    app.storage(StorageEvent::Clock {
        unix_secs: signed_at - 10,
    });
    let before = record(&app);
    app.press(Action::Vault(V::GpgRenewTerm(1)));
    app.press(Action::Vault(V::GpgRenew));
    assert!(
        app.toast_text()
            .is_some_and(|t| t.starts_with("Time conflict")),
        "{:?}",
        app.toast_text()
    );
    assert_eq!(record(&app).field(7), before.field(7));
    // Two seconds before: signed one second after the last.
    app.storage(StorageEvent::Clock {
        unix_secs: signed_at - 2,
    });
    app.press(Action::Vault(V::GpgRenew));
    let r = record(&app);
    assert_ne!(r.field(7), before.field(7));
    assert_eq!(
        gpg::certified_at(faraday_keys::vault::RecordView::of(&r)),
        Some(u32::try_from(signed_at).unwrap() + 1)
    );
}

/// A renewed key is valid in GnuPG again.
#[test]
fn a_renewed_key_is_valid_in_gnupg_again() {
    let mut app = unlocked();
    keep_the_time(&mut app);
    make_key(&mut app, "Release");
    let key = key_of(&app);
    let end = u64::from(key.created) + u64::from(key.expiry);
    let h = home("renew");
    let first = app.outbox[0].bytes.clone();
    std::fs::write(h.0.join("a.asc"), &first).unwrap();
    let Some((ok, said)) = gpg_at(
        &h.0,
        end - 100,
        &["--import", h.0.join("a.asc").to_str().unwrap()],
    ) else {
        return;
    };
    assert!(ok, "{said}");
    let state = |at: u64| -> String {
        let (_, l) = gpg_at(&h.0, at, &["--with-colons", "--list-keys"]).unwrap();
        l.lines()
            .find(|l| l.starts_with("pub:"))
            .unwrap()
            .to_string()
    };
    assert!(
        state(end + 100).starts_with("pub:e:"),
        "{}",
        state(end + 100)
    );
    app.storage(StorageEvent::Clock {
        unix_secs: end + 100,
    });
    app.press(Action::Vault(V::GpgRenewTerm(1)));
    app.press(Action::Vault(V::GpgRenew));
    app.outbox.clear();
    app.press(Action::Vault(V::GpgExport));
    std::fs::write(h.0.join("b.asc"), &app.outbox[0].bytes).unwrap();
    let (ok, said) = gpg_at(
        &h.0,
        end + 100,
        &["--import", h.0.join("b.asc").to_str().unwrap()],
    )
    .unwrap();
    assert!(ok, "{said}");
    assert!(
        state(end + 110).starts_with("pub:-:"),
        "{}",
        state(end + 110)
    );
}

/// The classifier: a `.rev` file, a block with the colon guard and a
/// block whose Comment holds a seed are not public (F22, F32).
#[test]
fn a_rev_file_a_guarded_block_and_a_comment_with_a_seed_are_not_public() {
    let c = testkit::compute();
    let k = faraday_pgp::Key {
        primary_seed: [0x11; 32],
        created: 1_780_000_000,
        subkey_seed: [0x22; 32],
        subkey_created: 1_780_000_000,
        user_ids: vec!["Test Person <test@example.com>".into()],
        expiry: 0,
    };
    let cert = k.certificate(1_780_000_000);
    let public = armor(Armor::PublicKey, &cert);
    assert_eq!(classify(&c, "k.asc", public.as_bytes()), FileKind::Gpg);
    let rev = k.revocation_file(&k.revocation(1_780_000_100, faraday_pgp::Reason::Unspecified, ""));
    assert_ne!(classify(&c, "k.rev", rev.as_bytes()), FileKind::Gpg);
    assert_ne!(
        classify(&c, "k.asc", format!(":{public}").as_bytes()),
        FileKind::Gpg
    );
    let words = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
    let with_words = armor_with(Armor::PublicKey, &cert, Some(words));
    assert_ne!(classify(&c, "k.asc", with_words.as_bytes()), FileKind::Gpg);
    let with_xprv = armor_with(
        Armor::PublicKey,
        &cert,
        Some(
            "xprv9s21ZrQH143K3QTDL4LXw2F7HEK3wJUD2nW2nRk4stbPy6cq3jPPqjiChkVvvNKmPGJxWUtg6LnF5kejMRNNU3TGtRBeJgk33yuGBxrMPHi",
        ),
    );
    assert_ne!(classify(&c, "k.asc", with_xprv.as_bytes()), FileKind::Gpg);
    // A user ID with a seed.
    let mut k2 = k;
    k2.user_ids = vec![words.to_string()];
    let seeded = armor(Armor::PublicKey, &k2.certificate(1_780_000_000));
    assert_ne!(classify(&c, "k.asc", seeded.as_bytes()), FileKind::Gpg);
}

/// What the app last asked the shell to keep across a lock.
fn boxes_kept(app: &mut Faraday) -> Option<StorageEvent> {
    let mut last = None;
    while let Some(c) = app.poll_storage() {
        if let StorageCommand::SaveBoxes {
            inbox,
            outbox,
            kept,
        } = c
        {
            last = Some(StorageEvent::Restored {
                inbox,
                outbox,
                kept,
            });
        }
    }
    last
}

/// The owner's release signing of 2026-10-20: booted from the stick that
/// holds the vault, its vault unlocked on the import sheet; a second stick
/// with `SHA256SUMS` and `v0.1.5.tag` asks for a lock first; after the
/// lock the visit copies both in, the stick is pulled, the vault unlocked
/// again, and the GPG key's Sign a file lists both.
#[test]
#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
fn files_from_a_second_stick_after_the_boot_stick_are_offered_for_signing() {
    let c = faraday_core::testkit::compute();
    let mut app = Faraday::new();
    display(&mut app);
    app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    app.storage(StorageEvent::Clock { unix_secs: CLOCK });
    faraday_core::testkit::vault_speed(&mut app, Some(180));
    let boot = [("vault.ofv".to_string(), testkit::test_vault().unwrap())];
    let listing = |files: &[(String, Vec<u8>)]| -> Vec<(String, u64)> {
        files
            .iter()
            .map(|(n, b)| (n.clone(), b.len() as u64))
            .collect()
    };
    app.storage(StorageEvent::Sticks(vec![StickInfo {
        serial: None,
        id: "B".into(),
        label: "FARADAY".into(),
        boot: true,
        files: listing(&boot),
    }]));
    while let Some(c) = app.poll_storage() {
        if let StorageCommand::Read { stick, name } = c {
            let bytes = boot.iter().find(|(n, _)| *n == name).unwrap().1.clone();
            app.storage(StorageEvent::Read { stick, name, bytes });
        }
    }
    app.storage(StorageEvent::Sticks(Vec::new()));
    let _ = app.frame();
    assert_eq!(app.sheet, Some(Sheet::Import));
    app.press(Action::Import(faraday_core::boot_import::ImportAction::Go));
    type_text(&mut app, testkit::VAULT_PASSPHRASES[0]);
    app.press(Action::Import(
        faraday_core::boot_import::ImportAction::Submit,
    ));
    settle(&mut app, 1);
    assert_eq!(
        app.vaults().open.len(),
        1,
        "{:?}",
        app.vaults().unlock_error
    );

    let files = vec![
        (
            "SHA256SUMS".to_string(),
            b"# Faraday 0.1.5, commit 70193d94\n3bdf73f7  faraday-stick-0.1.5-x86_64-uefi.img.xz\n"
                .to_vec(),
        ),
        (
            "v0.1.5.tag".to_string(),
            b"object 70193d94\ntype commit\ntag v0.1.5\n\nFaraday 0.1.5\n".to_vec(),
        ),
    ];
    let second = StickInfo {
        serial: None,
        id: "S".into(),
        label: "FILES".into(),
        boot: false,
        files: listing(&files),
    };
    app.storage(StorageEvent::Sticks(vec![second.clone()]));
    assert_eq!(app.sheet, Some(Sheet::Lock));
    let _ = boxes_kept(&mut app);
    app.press(Action::Lock);
    let restored = boxes_kept(&mut app).expect("the boxes kept at the lock");

    // The process after the lock, the second stick still in.
    let mut app = Faraday::new();
    display(&mut app);
    app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    app.storage(restored);
    app.storage(StorageEvent::Clock { unix_secs: CLOCK });
    faraday_core::testkit::vault_speed(&mut app, Some(180));
    app.storage(StorageEvent::Sticks(vec![second]));
    assert_eq!(app.screen, Screen::Visit);
    app.press(Action::VisitCopy);
    while let Some(c) = app.poll_storage() {
        if let StorageCommand::Read { stick, name } = c {
            let bytes = files.iter().find(|(n, _)| *n == name).unwrap().1.clone();
            app.storage(StorageEvent::Read { stick, name, bytes });
        }
    }
    app.storage(StorageEvent::Sticks(Vec::new()));
    let _ = app.frame();
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Open(0)));
    type_text(&mut app, testkit::VAULT_PASSPHRASES[0]);
    app.press(Action::Vault(V::Unlock));
    settle(&mut app, 30);
    app.press(Action::Vault(V::LoadTicked));
    assert_eq!(
        app.vaults().open.len(),
        1,
        "{:?}",
        app.vaults().unlock_error
    );
    app.press(Action::Nav(Screen::VaultContents));
    app.press(Action::Vault(V::Category(4)));
    keep_the_time(&mut app);
    make_key(&mut app, "Release");
    app.press(Action::Vault(V::GpgSignPick));
    let page = drawn(&mut app);
    for (name, bytes) in &files {
        assert!(
            page.contains(&format!("{name}·{}B", bytes.len())),
            "{name} is not offered: {page}"
        );
        assert!(app.signature_code(&c, name).is_some(), "{name}");
    }
    signs_both(&mut app, &files);
}

/// The same files on the boot stick beside its vault: the boot sheet lists
/// them ticked, Import brings them into Files with the vault, the vault
/// unlocked on the sheet, and the GPG key's Sign a file lists both (F60).
#[test]
fn files_imported_from_the_boot_sheet_are_offered_for_signing() {
    let mut app = Faraday::new();
    display(&mut app);
    app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    app.storage(StorageEvent::Clock { unix_secs: CLOCK });
    faraday_core::testkit::vault_speed(&mut app, Some(180));
    let files = vec![
        (
            "SHA256SUMS".to_string(),
            b"# Faraday 0.1.5, commit 70193d94\n3bdf73f7  faraday-stick-0.1.5-x86_64-uefi.img.xz\n"
                .to_vec(),
        ),
        (
            "v0.1.5.tag".to_string(),
            b"object 70193d94\ntype commit\ntag v0.1.5\n\nFaraday 0.1.5\n".to_vec(),
        ),
    ];
    let mut boot = files.clone();
    boot.push(("vault.ofv".to_string(), testkit::test_vault().unwrap()));
    app.storage(StorageEvent::Sticks(vec![StickInfo {
        serial: None,
        id: "B".into(),
        label: "FARADAY".into(),
        boot: true,
        files: boot
            .iter()
            .map(|(n, b)| (n.clone(), b.len() as u64))
            .collect(),
    }]));
    while let Some(c) = app.poll_storage() {
        if let StorageCommand::Read { stick, name } = c {
            let bytes = boot.iter().find(|(n, _)| *n == name).unwrap().1.clone();
            app.storage(StorageEvent::Read { stick, name, bytes });
        }
    }
    assert_eq!(app.sheet, Some(Sheet::Import));
    let view = app.import_view().unwrap();
    assert!(view.files.iter().all(|f| f.chosen), "{view:?}");
    app.press(Action::Import(faraday_core::boot_import::ImportAction::Go));
    for (name, bytes) in &files {
        let item = app.inbox().iter().find(|i| i.name == *name);
        assert_eq!(item.map(|i| &i.bytes[..]), Some(&bytes[..]), "{name}");
    }
    app.storage(StorageEvent::Sticks(Vec::new()));
    let _ = app.frame();
    type_text(&mut app, testkit::VAULT_PASSPHRASES[0]);
    app.press(Action::Import(
        faraday_core::boot_import::ImportAction::Submit,
    ));
    settle(&mut app, 1);
    assert_eq!(
        app.vaults().open.len(),
        1,
        "{:?}",
        app.vaults().unlock_error
    );
    app.press(Action::Vault(V::LoadTicked));
    app.press(Action::Nav(Screen::VaultContents));
    app.press(Action::Vault(V::Category(4)));
    keep_the_time(&mut app);
    make_key(&mut app, "Release");
    app.press(Action::Vault(V::GpgSignPick));
    let page = drawn(&mut app);
    for (name, bytes) in &files {
        assert!(
            page.contains(&format!("{name}·{}B", bytes.len())),
            "{name} is not offered: {page}"
        );
    }
    signs_both(&mut app, &files);
}

/// Both files ticked, Sign: the sheet lists both and signs nothing; its
/// hold signs each once (F63).
fn signs_both(app: &mut Faraday, files: &[(String, Vec<u8>)]) {
    for (name, _) in files {
        let k = listed(app, name);
        if !app.vaults().sign_ticked.iter().any(|(n, _)| n == name) {
            app.press(Action::Vault(V::GpgTick(k)));
        }
    }
    let before = app.outbox.len();
    app.press(Action::Vault(V::GpgSign));
    assert_eq!(app.sheet, Some(Sheet::GpgSign));
    assert_eq!(app.outbox.len(), before, "Sign signs nothing");
    let sheet = drawn(app);
    for (name, bytes) in files {
        assert!(
            sheet.contains(&format!("{name}·{}B", bytes.len())),
            "{name} on the sheet: {sheet}"
        );
    }
    assert!(sheet.contains("Holdtosign2files"), "{sheet}");
    app.press(Action::Vault(V::GpgHoldSign));
    for (name, _) in files {
        let n = app
            .outbox
            .iter()
            .filter(|i| i.name == format!("{name}.asc"))
            .count();
        assert_eq!(n, 1, "{name} signed once");
    }
    assert_eq!(app.outbox.len(), before + files.len());
}
