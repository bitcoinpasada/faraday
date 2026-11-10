//! A vault entry exported for KeePass: sealed under the passphrase typed
//! for it, listed with the sealed files, and a KDBX 4 file that passphrase
//! opens.

use crate::common::type_text;
use faraday_core::secrets::Exposure;
use faraday_core::testkit;
use faraday_core::vault::VaultAction as V;
use faraday_core::{Action, Faraday, Screen, StorageEvent};
use faraday_shell_api::{App, BootState, DisplayInfo, Event, SecureHardware};

fn settle(app: &mut Faraday, from: u64) {
    for t in from..from + 20 {
        let _ = app.frame();
        app.event(Event::Tick { now_ms: t * 1000 });
    }
}

/// A device with an entry open on its page and the passphrase of its
/// KeePass export typed, the press that writes it still to come.
fn at_the_export_prompt() -> Faraday {
    at_the_export_prompt_with(12)
}

/// The same, with this many answers to the requests for randomness.
fn at_the_export_prompt_with(answers: u8) -> Faraday {
    at_the_export_prompt_beside(answers, Vec::new())
}

/// The same, with these files waiting in the Outbox.
fn at_the_export_prompt_beside(answers: u8, outbox: Vec<(String, Vec<u8>)>) -> Faraday {
    let mut app = Faraday::new();
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
    // The answers a shell gives to the requests for randomness: the
    // session's seed, then eleven pooled.
    for i in 0..answers {
        app.event(Event::Entropy(faraday_shell_api::EntropyBytes::new(
            [0x40 + i; 32],
        )));
    }
    let vault = testkit::files()
        .unwrap()
        .into_iter()
        .find(|(n, _)| n == "vault.kdbx")
        .unwrap();
    app.storage(StorageEvent::Restored {
        inbox: vec![vault],
        outbox,
        kept: Vec::new(),
    });
    app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    faraday_core::testkit::vault_speed(&mut app, Some(180));
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Open(0)));
    type_text(&mut app, testkit::VAULT_PASSPHRASE);
    app.press(Action::Vault(V::Unlock));
    settle(&mut app, 1);
    assert_eq!(app.vaults().open.len(), 1);
    app.press(Action::Vault(V::LoadNotNow));
    app.press(Action::Nav(Screen::VaultContents));
    // An entry of its own, then exported.
    app.press(Action::Vault(V::Category(2)));
    app.press(Action::Vault(V::Add));
    app.press(Action::Vault(V::FocusField(0)));
    type_text(&mut app, "Mail");
    app.press(Action::Vault(V::FocusField(2)));
    type_text(&mut app, "hunter22");
    app.press(Action::Vault(V::FormSave));
    let entries = app
        .vaults()
        .records(app.vaults().open[0].id)
        .of(faraday_vault::records::kind::ENTRY)
        .count();
    app.press(Action::Vault(V::Item(entries - 1)));
    app.press(Action::Vault(V::ExportKdbx));
    type_text(&mut app, "keepass passphrase");
    app
}

#[test]
fn an_entry_exported_for_keepass_is_sealed_under_its_passphrase() {
    let c = testkit::compute();
    let mut app = at_the_export_prompt();
    // A person's press: the file is written on the worker.
    testkit::press_on_worker(&mut app, Action::Vault(V::PromptGo));
    for t in 30..40 {
        if app.busy().is_some() {
            break;
        }
        let _ = app.frame();
        app.event(Event::Tick { now_ms: t * 1000 });
    }
    assert!(
        app.busy().is_some(),
        "the screen is busy while it is sealed"
    );
    testkit::settle(&mut app);
    let file = app
        .outbox
        .iter()
        .find(|i| i.name.ends_with(".kdbx"))
        .expect("no KeePass file");
    assert_eq!(file.kind.exposure(), Exposure::Sealed);
    let opened = faraday_vault::file::open(&c, &file.bytes, "keepass passphrase")
        .expect("the file opens as a vault under its passphrase");
    assert!(faraday_vault::file::open(&c, &file.bytes, "another").is_err());
    // One entry, an ordinary one in the root group, at the smallest size.
    let read = opened.read();
    assert_eq!(read.items.len(), 1);
    assert!(read.items[0].path.is_empty());
    let faraday_vault::kdbx::items::Body::Record(r) = &read.items[0].body else {
        panic!("a person's entry");
    };
    assert_eq!(r.text(faraday_vault::records::field::TITLE), Some("Mail"));
    assert_eq!(
        r.text(faraday_vault::records::field::PASSWORD),
        Some("hunter22")
    );
    assert_eq!(read.seals, 1);
    pykeepass_reads_one_entry(&file.bytes);
    // Its random fields are the session's fresh bytes, an answer for every
    // 32, the latest first, each as SHA-256("faraday fresh" ‖ label ‖
    // answer) (docs/OWN-RANDOMNESS.md row 6; computed with Python's
    // `hashlib`): the vault's identifier from the last answer, 0x4b × 32,
    // under "vault id"; the seal's from the five before it under "vault
    // seal", its master seed the first (0x4a) and its IV the first 12 bytes
    // of the next; the entry's UUID from the one after, under "vault
    // entries". A fixed value in any of them would not match.
    assert_eq!(
        hex(&opened.vault_id()),
        hex(b"3a6ee9af231bd7f7ef962395566a6f5e")
    );
    assert_eq!(
        hex(header_field(&file.bytes, 4)),
        "157d96e797d134db7e0f4b897cdf1e06531c59cdbbcfe0609ee15c98b95c9f39"
    );
    assert_eq!(
        hex(header_field(&file.bytes, 7)),
        "b8f9305623a00cb447e4ab9c"
    );
    assert_eq!(hex(&read.items[0].uuid), "6a40f706f73b5f0b290d485bed110795");
    assert_eq!(opened.facts().size, Some(32_768));
    let cost = opened.facts().header.cost;
    assert_eq!((cost.memory_kib, cost.passes, cost.lanes), (65_536, 3, 4));
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

/// The value of KDBX 4 outer header field `id`: after the two signatures
/// and the version, each field is its id, a four-byte little-endian
/// length and the value.
fn header_field(file: &[u8], id: u8) -> &[u8] {
    let mut at = 12;
    loop {
        let len = u32::from_le_bytes(file[at + 1..at + 5].try_into().unwrap()) as usize;
        if file[at] == id {
            return &file[at + 5..at + 5 + len];
        }
        assert_ne!(file[at], 0, "no field {id}");
        at += 5 + len;
    }
}

#[test]
fn an_export_named_as_a_sealed_vault_waiting_goes_beside_it() {
    let c = testkit::compute();
    let sealed = testkit::test_vault().unwrap();
    let mut app = at_the_export_prompt_beside(12, vec![("mail.kdbx".into(), sealed.clone())]);
    app.press(Action::Vault(V::PromptGo));
    testkit::settle(&mut app);
    let file = |name: &str| app.outbox.iter().find(|i| i.name == name);
    assert_eq!(
        file("mail.kdbx").map(|i| &i.bytes),
        Some(&sealed),
        "the sealed vault is still there, unchanged"
    );
    let export = file("mail-2.kdbx").expect("the export beside it");
    faraday_vault::file::open(&c, &export.bytes, "keepass passphrase")
        .expect("the export opens under its own passphrase");
}

/// The exported file read by pykeepass (tools/vectors/kdbx/reread.py): one
/// entry, in the root group, with its password. Said and skipped where
/// pykeepass is not installed.
fn pykeepass_reads_one_entry(file: &[u8]) {
    let dir = std::env::temp_dir().join(format!("faraday-export-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("export.kdbx");
    std::fs::write(&path, file).unwrap();
    let script =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools/vectors/kdbx/reread.py");
    let out = std::process::Command::new("python3")
        .args(["-I", "-B"])
        .arg(script)
        .arg(&path)
        .arg("keepass passphrase")
        .output();
    let _ = std::fs::remove_dir_all(&dir);
    let Ok(out) = out else {
        return;
    };
    let err = String::from_utf8_lossy(&out.stderr);
    if !out.status.success() && (err.contains("ModuleNotFoundError") || err.contains("ImportError"))
    {
        eprintln!("pykeepass is not installed: skipped");
        return;
    }
    assert!(out.status.success(), "pykeepass: {err}");
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(
        text.starts_with("/Mail\t\thunter22\t\t0\t\nbin: no"),
        "{text}"
    );
}

fn kdbx_files(app: &Faraday) -> usize {
    app.outbox
        .iter()
        .filter(|i| i.name.ends_with(".kdbx"))
        .count()
}

#[test]
fn back_during_a_keepass_export_leaves_no_file() {
    let mut app = at_the_export_prompt();
    testkit::hold_worker();
    testkit::press_on_worker(&mut app, Action::Vault(V::PromptGo));
    for t in 30..40 {
        if app.busy().is_some() {
            break;
        }
        let _ = app.frame();
        app.event(Event::Tick { now_ms: t * 1000 });
    }
    assert!(app.busy().is_some());
    app.press(Action::Back);
    assert!(app.busy().is_none());
    testkit::release_worker();
    testkit::settle_stale(&mut app);
    settle(&mut app, 100);
    assert_eq!(kdbx_files(&app), 0);
}

#[test]
fn a_lock_during_a_keepass_export_leaves_no_file() {
    let mut app = at_the_export_prompt();
    testkit::hold_worker();
    testkit::press_on_worker(&mut app, Action::Vault(V::PromptGo));
    for t in 30..40 {
        if app.busy().is_some() {
            break;
        }
        let _ = app.frame();
        app.event(Event::Tick { now_ms: t * 1000 });
    }
    assert!(app.busy().is_some());
    app.press(Action::LockUnsealed);
    testkit::release_worker();
    testkit::settle_stale(&mut app);
    settle(&mut app, 100);
    assert_eq!(kdbx_files(&app), 0);
}

#[test]
fn exporting_for_keepass_before_the_randomness_arrives_says_why() {
    let mut app = at_the_export_prompt_with(6);
    app.press(Action::Vault(V::PromptGo));
    settle(&mut app, 30);
    let _ = app.frame();
    assert!(
        app.drawn_texts()
            .iter()
            .any(|t| t == faraday_strings::EN.no_randomness),
        "the prompt says why"
    );
    assert!(
        app.outbox.iter().all(|i| !i.name.ends_with(".kdbx")),
        "no file was made"
    );
}
