//! A GPG key signs any file a stick visit copied in: a release's
//! `SHA256SUMS` with no extension, a tag's text and a binary Faraday reads
//! as nothing else, each signature to the Outbox beside the file's name.

use faraday_core::testkit;
use faraday_core::vaults::VaultAction as V;
use faraday_core::{Action, Faraday, Screen, StickInfo, StorageCommand, StorageEvent};
use osk_shell_api::{App, BootState, DisplayInfo, EntropyBytes, Event, Key, SecureHardware};

fn type_text(app: &mut Faraday, text: &str) {
    for c in text.chars() {
        app.event(Event::Key(Key::Char(c)));
    }
}

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

/// The app with the test vault in the Inbox and the release files copied
/// in on a stick visit, the stick then pulled.
fn copied_in() -> Faraday {
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
    // The answers a shell gives to the requests for randomness.
    for i in 0..6u8 {
        app.event(Event::Entropy(EntropyBytes::new([0x60 + i; 32])));
    }
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
    app.storage(StorageEvent::Clock {
        unix_secs: 1_791_482_652,
    });
    app.vaults.ms_per_unit = Some(180);
    let files = release_files();
    app.storage(StorageEvent::Sticks(vec![StickInfo {
        id: "S".into(),
        label: "FARADAY".into(),
        boot: false,
        files: files
            .iter()
            .map(|(n, b)| (n.clone(), b.len() as u64))
            .collect(),
    }]));
    assert_eq!(app.screen, Screen::Visit);
    app.press(Action::VisitInAll);
    app.press(Action::VisitCopy);
    while let Some(c) = app.poll_storage() {
        if let StorageCommand::Read { stick, name } = c {
            let bytes = files.iter().find(|(n, _)| *n == name).unwrap().1.clone();
            app.storage(StorageEvent::Read { stick, name, bytes });
        }
    }
    app.storage(StorageEvent::Sticks(Vec::new()));
    let _ = app.frame();
    app
}

#[test]
fn a_stick_visit_copies_in_files_of_no_kind_faraday_knows() {
    let app = copied_in();
    assert!(
        app.visit.log.iter().all(|(_, ok)| *ok),
        "{:?}",
        app.visit.log
    );
    for (name, bytes) in release_files() {
        let item = app.inbox.iter().find(|i| i.name == name);
        assert_eq!(item.map(|i| &i.bytes[..]), Some(&bytes[..]), "{name}");
    }
}

#[test]
fn a_gpg_key_signs_each_file_into_the_outbox() {
    let mut app = copied_in();
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Open(0)));
    type_text(&mut app, testkit::VAULT_PASSPHRASES[0]);
    app.press(Action::Vault(V::Unlock));
    settle(&mut app, 1);
    assert_eq!(app.vaults.open.len(), 1, "{:?}", app.vaults.unlock_error);
    app.press(Action::Nav(Screen::VaultContents));
    app.press(Action::Vault(V::Category(4)));
    app.press(Action::Vault(V::Add));
    app.press(Action::Vault(V::FocusField(0)));
    type_text(&mut app, "Release");
    app.press(Action::Vault(V::FormSave));
    app.press(Action::Vault(V::GpgSignPick));
    for (name, _) in release_files() {
        let k = app.inbox.iter().position(|i| i.name == name).unwrap();
        app.press(Action::Vault(V::GpgSign(k)));
        let sig = app
            .outbox
            .iter()
            .find(|i| i.name == format!("{name}.asc"))
            .unwrap_or_else(|| panic!("no signature for {name}"));
        assert!(
            sig.bytes.starts_with(b"-----BEGIN PGP SIGNATURE-----"),
            "{name}"
        );
        app.press(Action::Vault(V::GpgSignPick));
    }
}
