//! What Files offers for each kind of file copied in: a text file, a
//! seed's words or a descriptor kept in the open vault with one press,
//! and the shares of a split backup put back together into the wallet
//! from Files itself.

use faraday_core::testkit;
use faraday_core::vaults::VaultAction as V;
use faraday_core::wallet::FileKind;
use faraday_core::{Action, Faraday, Screen, StorageEvent};
use faraday_vault::records::{field, kind};
use osk_shell_api::{App, BootState, DisplayInfo, Event, Key, SecureHardware};

const CODES: &str = "Gmail backup codes\n1234 5678\n2345 6789\n";

fn with_files(files: Vec<(String, Vec<u8>)>) -> Faraday {
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
    app.storage(StorageEvent::Restored {
        inbox: files,
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app
}

fn unlocked(mut files: Vec<(String, Vec<u8>)>) -> Faraday {
    files.insert(0, ("vault.ofv".to_string(), testkit::test_vault().unwrap()));
    let mut app = with_files(files);
    app.press(Action::Vault(V::Open(0)));
    for c in testkit::VAULT_PASSPHRASES[0].chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    app.press(Action::Vault(V::Unlock));
    for t in 1..10u64 {
        let _ = app.frame();
        app.event(Event::Tick { now_ms: t * 1000 });
    }
    assert_eq!(app.vaults.open.len(), 1);
    app
}

fn at(app: &Faraday, name: &str) -> usize {
    app.inbox.iter().position(|i| i.name == name).unwrap()
}

#[test]
fn a_text_file_is_kept_and_offered_to_the_vault_as_a_note() {
    let mut app = unlocked(vec![("gmail-codes.txt".into(), CODES.into())]);
    let k = at(&app, "gmail-codes.txt");
    assert_eq!(app.inbox[k].kind, FileKind::Text);
    app.press(Action::Nav(Screen::Files));
    // The open vault's panel stands above the Inbox: scroll down to it.
    // A key glides the page, so the frames between presses are ticked
    // as a shell ticks them.
    let mut offered = false;
    let mut now = 20_000;
    for _ in 0..20 {
        let _ = app.frame();
        offered |= app.offers(Action::Vault(V::AddFile(k)));
        app.event(Event::Key(Key::Down));
        for _ in 0..12 {
            now += 16;
            app.event(Event::Tick { now_ms: now });
        }
    }
    assert!(offered, "Files does not offer Add to vault");
    app.press(Action::Vault(V::AddFile(k)));
    let v = &app.vaults.open[0];
    let note = v
        .contents
        .of(kind::NOTE)
        .filter_map(|(_, r)| r.text(field::NOTE))
        .find(|t| t.contains("2345 6789"))
        .expect("no note holds the codes");
    assert!(
        note.starts_with("gmail-codes\n"),
        "the note's title is not the file's name"
    );
    assert!(v.changes > 0, "the vault was not marked changed");
}

#[test]
fn a_seed_words_file_goes_into_the_vault_as_a_key() {
    let (name, bytes) = testkit::files()
        .unwrap()
        .into_iter()
        .find(|(n, _)| n.starts_with("seed-2") && n.ends_with("-words.txt"))
        .unwrap();
    let mut app = unlocked(vec![(name.clone(), bytes)]);
    let keys = app.vaults.open[0].contents.of(kind::KEY).count();
    app.press(Action::Vault(V::AddFile(at(&app, &name))));
    assert_eq!(app.vaults.open[0].contents.of(kind::KEY).count(), keys + 1);
}

#[test]
fn without_an_open_vault_nothing_is_added() {
    let mut app = with_files(vec![("gmail-codes.txt".into(), CODES.into())]);
    app.press(Action::Vault(V::AddFile(0)));
    assert!(app.vaults.open.is_empty());
    assert!(app.toast_text().is_some_and(|t| t.contains("Unlock")));
}

#[test]
fn files_restores_a_wallet_from_its_shares() {
    let shares: Vec<(String, Vec<u8>)> = testkit::files()
        .unwrap()
        .into_iter()
        .filter(|(n, _)| n.starts_with("savings-share-"))
        .take(2)
        .collect();
    let mut app = with_files(shares);
    app.press(Action::Nav(Screen::Files));
    let _ = app.frame();
    assert!(
        app.offers(Action::RestoreShares),
        "Files does not offer the restore"
    );
    app.press(Action::RestoreShares);
    assert_eq!(app.screen, Screen::Wallets);
    assert_eq!(app.session.wallets.len(), 1);
    assert_eq!(app.session.address(&app.session.wallets[0], false, 0), {
        let mut s = faraday_core::wallet::Session::on(testkit::NET);
        s.add_wallet("Savings", &testkit::savings(), "test")
            .unwrap();
        s.address(&s.wallets[0], false, 0)
    });
}

// ---------------------------------------------------------------------
// §4.1 (`docs/SIMPLIFY.md`): the two halves of Files are named by
// direction, From the stick and For the stick, wherever a person reads
// them.
// ---------------------------------------------------------------------

use faraday_core::{Screen as S, Sheet, StickInfo};

const SCREENS: [S; 29] = [
    S::Home,
    S::Start,
    S::Wallets,
    S::Spend,
    S::Files,
    S::Visit,
    S::Entry,
    S::Backup,
    S::Message,
    S::CheckMessage,
    S::Create,
    S::Restore,
    S::Settings,
    S::Vaults,
    S::CreateVault,
    S::KeyGen,
    S::Bip85,
    S::Silent,
    S::Explore,
    S::Lightning,
    S::Tools,
    S::Unlock,
    S::VaultContents,
    S::Family,
    S::Vanity,
    S::Decode,
    S::Catalog,
    S::Transfer,
    S::Upgrade,
];

/// What `app` draws, each string tagged with where it was drawn.
fn texts_at(app: &mut Faraday, at: &str, all: &mut Vec<(String, String)>) {
    all.extend(app.drawn_texts().into_iter().map(|t| (at.to_string(), t)));
}

/// A boot stick holding `files`, every read it asks for answered.
fn boot_stick(app: &mut Faraday, files: &[(String, Vec<u8>)]) {
    app.storage(StorageEvent::Sticks(vec![StickInfo {
        id: "boot".into(),
        label: "FARADAY".into(),
        boot: true,
        files: files
            .iter()
            .map(|(n, b)| (n.clone(), b.len() as u64))
            .collect(),
    }]));
    while let Some(c) = app.poll_storage() {
        if let faraday_core::StorageCommand::Read { stick, name } = c
            && let Some((_, bytes)) = files.iter().find(|(n, _)| *n == name)
        {
            app.storage(StorageEvent::Read {
                stick,
                name,
                bytes: bytes.clone(),
            });
        }
    }
    let _ = app.frame();
}

/// Every screen, and every sheet that names the two halves, at a
/// display of `width` x `height` at `dpi`, with files in both: what is
/// drawn, tagged with where.
fn every_text(width: u16, height: u16, dpi: u16) -> Vec<(String, String)> {
    let kit = |name: &str| {
        testkit::files()
            .unwrap()
            .into_iter()
            .find(|(n, _)| n == name)
            .unwrap()
    };
    let mut app = Faraday::new();
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
    app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    app.storage(StorageEvent::Restored {
        inbox: vec![
            kit("vault.ofv"),
            kit("spending-wallet.txt"),
            ("gmail-codes.txt".into(), CODES.into()),
        ],
        outbox: vec![kit("savings-wallet.txt")],
        kept: Vec::new(),
    });
    let mut all = Vec::new();
    // The boot import sheet, then a stick visit with nothing held.
    boot_stick(&mut app, &[("notes.txt".to_string(), b"a note".to_vec())]);
    if app.import.is_some() {
        app.sheet = Some(Sheet::Import);
        texts_at(&mut app, "boot import sheet", &mut all);
    }
    app.storage(StorageEvent::Sticks(vec![StickInfo {
        id: "S".into(),
        label: "STICK".into(),
        boot: false,
        files: vec![("spend.psbt".into(), 300)],
    }]));
    app.sheet = None;
    app.press(Action::Nav(S::Visit));
    texts_at(&mut app, "visit", &mut all);
    app.press(Action::Nav(S::Files));
    texts_at(&mut app, "Files with a stick", &mut all);
    app.storage(StorageEvent::Sticks(Vec::new()));
    app.sheet = None;
    // A key loaded, and a wallet: every screen, then the sheets.
    app.press(Action::Entry(None));
    for c in testkit::test_words(testkit::TEST_SEEDS[0].0).chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    app.press(Action::EntryAdd);
    let w = app
        .inbox
        .iter()
        .position(|i| i.name == "spending-wallet.txt")
        .unwrap();
    app.press(Action::LoadWallet(w));
    for screen in SCREENS {
        app.screen = screen;
        texts_at(&mut app, &format!("{screen:?}"), &mut all);
    }
    app.screen = S::Files;
    for sheet in [
        Sheet::Lock,
        Sheet::LockAsk,
        Sheet::Power,
        Sheet::IdleWarn,
        Sheet::Locked,
        Sheet::WriteOut,
    ] {
        app.sheet = Some(sheet);
        texts_at(&mut app, &format!("{sheet:?}"), &mut all);
    }
    app.sheet = None;
    // The secret sheet: a BIP-85 child seed let out.
    app.press(Action::Bip85);
    app.press(Action::PApp(4));
    app.press(Action::PNext);
    app.press(Action::PNext);
    app.press(Action::POut);
    assert_eq!(app.sheet, Some(Sheet::SecretOut));
    texts_at(&mut app, "secret sheet", &mut all);
    all
}

#[test]
fn no_screen_or_sheet_calls_either_half_inbox_or_outbox() {
    for (w, h, dpi) in [(1366, 768, 160), (480, 640, 160)] {
        let all = every_text(w, h, dpi);
        assert!(all.len() > 500, "{w}x{h}: too little drawn to judge");
        let bad: Vec<&(String, String)> = all
            .iter()
            .filter(|(_, t)| {
                let t = t.to_lowercase();
                t.contains("inbox") || t.contains("outbox")
            })
            .collect();
        assert!(bad.is_empty(), "{w}x{h}: {bad:?}");
        assert!(
            all.iter().any(|(_, t)| t == "From the stick")
                && all.iter().any(|(_, t)| t == "For the stick"),
            "{w}x{h}: Files does not name its halves by direction"
        );
    }
}
