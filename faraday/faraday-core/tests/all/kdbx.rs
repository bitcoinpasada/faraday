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
    for i in 0..12u8 {
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
        outbox: Vec::new(),
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
    assert_eq!(opened.facts().size, Some(65_536));
    let cost = opened.facts().header.cost;
    assert_eq!((cost.memory_kib, cost.passes, cost.lanes), (65_536, 3, 4));
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
