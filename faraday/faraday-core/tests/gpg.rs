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

/// The app with the test vault unlocked on its GPG keys, after the stick
/// visit.
fn unlocked() -> Faraday {
    let mut app = copied_in();
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Open(0)));
    type_text(&mut app, testkit::VAULT_PASSPHRASES[0]);
    app.press(Action::Vault(V::Unlock));
    settle(&mut app, 1);
    assert_eq!(app.vaults.open.len(), 1, "{:?}", app.vaults.unlock_error);
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

/// Add, a name, Save: a GPG key made, as a person makes one.
fn make_key(app: &mut Faraday, name: &str) {
    app.press(Action::Vault(V::Add));
    app.press(Action::Vault(V::FocusField(0)));
    type_text(app, name);
    app.press(Action::Vault(V::FormSave));
}

fn gpg_keys(app: &Faraday) -> usize {
    app.vaults.open[app.vaults.current]
        .contents
        .of(faraday_vault::records::kind::GPG)
        .count()
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
    assert_eq!(app.screen, Screen::VaultContents, "back on the same page");
    app.press(Action::Vault(V::FormSave));
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

/// Signing on the stick asks for the time first too, and signs nothing
/// until it is checked.
#[test]
fn on_the_stick_signing_asks_for_the_time_first() {
    let mut app = unlocked();
    app.online = true;
    make_key(&mut app, "Release");
    app.online = false;
    app.press(Action::Vault(V::GpgSignPick));
    app.press(Action::Vault(V::GpgTick(0)));
    let files = app.outbox.len();
    app.press(Action::Vault(V::GpgSignTicked));
    assert_eq!(app.screen, Screen::SetTime);
    assert_eq!(app.outbox.len(), files, "nothing signed");
    app.press(Action::SetTimeKeep);
    app.press(Action::Vault(V::GpgSignTicked));
    assert_eq!(app.screen, Screen::VaultContents);
    assert_eq!(app.toast_text(), Some("1 signature waits for the stick"));
}

/// Sign a file is a checkbox list: ticking two files and pressing Sign 2
/// files puts two signatures in the Outbox in one go, each verifying
/// against its own file with the key's certificate, and the page stays
/// open with the signed rows marked.
#[test]
fn ticking_two_files_and_signing_puts_two_signatures_in_the_outbox() {
    let mut app = signing();
    let names: Vec<String> = release_files().into_iter().map(|(n, _)| n).collect();
    let signable = |app: &Faraday| -> Vec<String> {
        app.inbox
            .iter()
            .filter(|i| !i.name.ends_with(".asc"))
            .map(|i| i.name.clone())
            .collect()
    };
    let ticks: Vec<usize> = names
        .iter()
        .take(2)
        .map(|n| signable(&app).iter().position(|x| x == n).unwrap())
        .collect();
    for k in ticks {
        app.press(Action::Vault(V::GpgTick(k)));
    }
    assert!(app.screen != Screen::Home, "still on the vault");
    app.press(Action::Vault(V::GpgSignTicked));
    assert!(app.vaults.signing, "the page stays open");
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
    assert!(app.vaults.sign_ticked.is_empty(), "the ticks clear");
}

/// A signature already made (`NAME.asc` in the Outbox) marks its file's
/// row Signed, and an `.asc` file already in the Inbox (say, copied back
/// in from another device) is not itself listed to sign again.
#[test]
fn a_signed_file_is_marked_and_an_asc_in_the_inbox_is_not_listed() {
    let mut app = signing();
    let name = &release_files()[0].0;
    let signable = |app: &Faraday| -> Vec<String> {
        app.inbox
            .iter()
            .filter(|i| !i.name.ends_with(".asc"))
            .map(|i| i.name.clone())
            .collect()
    };
    let k = signable(&app).iter().position(|x| x == name).unwrap();
    app.press(Action::Vault(V::GpgTick(k)));
    app.press(Action::Vault(V::GpgSignTicked));
    let _ = app.frame();
    let texts = app.drawn_texts();
    assert!(
        texts.iter().any(|t| t == "Signed"),
        "the signed row is marked: {texts:?}"
    );
    // An .asc file that lands in the Inbox is a signature, not something
    // to sign.
    app.inbox.push(faraday_core::Item {
        name: format!("{name}.asc"),
        bytes: b"-----BEGIN PGP SIGNATURE-----\n...".to_vec(),
        kind: faraday_core::wallet::FileKind::Other,
        secret: false,
        picture: None,
    });
    assert!(
        signable(&app).iter().all(|x| x != &format!("{name}.asc")),
        "an .asc file is not itself signable"
    );
}

/// With one file only in the Inbox, it starts ticked and Sign 1 file
/// signs it.
#[test]
fn with_one_file_it_starts_ticked() {
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
    for i in 0..6u8 {
        app.event(Event::Entropy(EntropyBytes::new([0x60 + i; 32])));
    }
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
    app.storage(StorageEvent::Clock {
        unix_secs: 1_791_482_652,
    });
    app.vaults.ms_per_unit = Some(180);
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Open(0)));
    type_text(&mut app, testkit::VAULT_PASSPHRASES[0]);
    app.press(Action::Vault(V::Unlock));
    settle(&mut app, 1);
    app.press(Action::Nav(Screen::VaultContents));
    app.press(Action::Vault(V::Category(4)));
    keep_the_time(&mut app);
    make_key(&mut app, "Release");
    // The vault's own file stays in the Inbox once opened; drop it so
    // SHA256SUMS is the one signable file, as a stick visit that only
    // ever copied in the release would leave it.
    app.inbox
        .retain(|i| i.kind != faraday_core::wallet::FileKind::Vault);
    app.press(Action::Vault(V::GpgSignPick));
    assert_eq!(app.vaults.sign_ticked, vec!["SHA256SUMS".to_string()]);
    app.press(Action::Vault(V::GpgSignTicked));
    assert!(
        app.outbox.iter().any(|i| i.name == "SHA256SUMS.asc"),
        "Sign 1 file signed the one ticked file"
    );
}
