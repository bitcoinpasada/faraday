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
    let mut offered = false;
    for _ in 0..20 {
        let _ = app.frame();
        offered |= app.offers(Action::Vault(V::AddFile(k)));
        app.event(Event::Key(Key::Down));
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
