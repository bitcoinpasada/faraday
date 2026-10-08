//! The backup test stick: what backing up the 2-of-3 Taproot multisig
//! over the test seeds leaves. Its public files restore the wallet, any
//! two of its split sheets rebuild it, and its vault, passphrase `a`,
//! opens from Home once the stick is pulled and holds the three seeds
//! that sign, and its unsigned spend signs and finishes on the Spend tab.

use faraday_core::testkit;
use faraday_core::vaults::VaultAction as V;
use faraday_core::wallet::FileKind;
use faraday_core::{Action, Faraday, Screen, StickInfo, StorageEvent};
use osk_shell_api::{App, BootState, DisplayInfo, Event, Key, SecureHardware};

fn names() -> Vec<String> {
    testkit::backup_files()
        .unwrap()
        .into_iter()
        .map(|(n, _)| n)
        .collect()
}

fn first_address(text: &str) -> String {
    let mut s = testkit::session();
    let i = s.add_wallet("w", text, "test").unwrap();
    s.address(&s.wallets[i], false, 0)
}

fn taproot_multisig() -> String {
    testkit::kits()
        .into_iter()
        .find(|k| k.id == "taproot-multisig")
        .unwrap()
        .descriptor
}

#[test]
fn the_stick_holds_the_backup_and_no_seed_outside_the_vault() {
    let n = names();
    for want in [
        "taproot-multisig-descriptor.txt",
        "taproot-multisig-wallet.json",
        "taproot-multisig-backup.pdf",
        "taproot-multisig-share-1-of-3.txt",
        "taproot-multisig-share-1-of-3.pdf",
        "taproot-multisig-share-3-of-3.txt",
        "vault.ofv",
        "taproot-multisig-unsigned.psbt",
    ] {
        assert!(n.iter().any(|x| x == want), "no {want}");
    }
    for (name, bytes) in testkit::backup_files().unwrap() {
        let kind = faraday_core::wallet::classify(&name, &bytes);
        assert_ne!(kind, FileKind::Words, "{name} holds a seed in the clear");
    }
}

#[test]
fn any_two_taproot_split_sheets_rebuild_the_wallet() {
    let files = testkit::backup_files().unwrap();
    let share = |k: usize| {
        files
            .iter()
            .find(|(n, _)| *n == format!("taproot-multisig-share-{k}-of-3.txt"))
            .map(|(_, b)| String::from_utf8(b.clone()).unwrap())
            .unwrap()
    };
    let want = first_address(&taproot_multisig());
    for pair in [[1, 2], [2, 3], [1, 3]] {
        let m = faraday_core::restore::merge(&[share(pair[0]), share(pair[1])]).unwrap();
        let whole = m
            .whole
            .unwrap_or_else(|| panic!("{pair:?} did not rebuild"));
        assert_eq!(first_address(&whole), want, "{pair:?}");
    }
    let one = faraday_core::restore::merge(&[share(1)]).unwrap();
    assert!(one.whole.is_none(), "one sheet rebuilt the wallet");
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
}

#[test]
fn importing_everything_and_pulling_the_stick_asks_for_the_vault_and_loads_the_seeds() {
    let files = testkit::backup_files().unwrap();
    let mut app = Faraday::new();
    display(&mut app);
    app.storage(StorageEvent::Sticks(vec![StickInfo {
        id: "T".into(),
        label: "TESTSTICK".into(),
        boot: false,
        files: files
            .iter()
            .map(|(n, b)| (n.clone(), b.len() as u64))
            .collect(),
    }]));
    assert_eq!(app.screen, Screen::Visit);
    app.press(Action::VisitInAll);
    app.press(Action::VisitCopy);
    // The shell answers each read.
    while let Some(c) = app.poll_storage() {
        if let faraday_core::StorageCommand::Read { stick, name } = c {
            let bytes = files.iter().find(|(n, _)| *n == name).unwrap().1.clone();
            app.storage(StorageEvent::Read { stick, name, bytes });
        }
    }
    assert!(
        app.visit.log.iter().all(|(_, ok)| *ok),
        "{:?}",
        app.visit.log
    );
    // F4: the stick out, Home, and the vault opened from there.
    app.storage(StorageEvent::Sticks(Vec::new()));
    assert_eq!(app.screen, Screen::Home);
    let _ = app.frame();
    let open = Action::Vault(V::Open(0));
    assert!(app.offers(open), "Home does not offer the vault");
    app.press(open);
    assert_eq!(app.screen, Screen::Unlock);
    for c in testkit::BACKUP_VAULT_PASSPHRASE.chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    app.press(Action::Vault(V::Unlock));
    for t in 1..10u64 {
        let _ = app.frame();
        app.event(Event::Tick { now_ms: t * 1000 });
    }
    assert_eq!(app.vaults.open.len(), 1, "{:?}", app.vaults.unlock_error);
    app.press(Action::Vault(V::LoadAll(0)));
    assert_eq!(app.session.keys.len(), 3, "the three seeds did not load");
    // The descriptor in Files, opened: every key of it signs here.
    let at = app
        .inbox
        .iter()
        .position(|i| i.name == "taproot-multisig-descriptor.txt")
        .unwrap();
    app.press(Action::LoadWallet(at));
    let w = &app.session.wallets[0];
    assert!(app.session.slots(w).iter().all(|s| s.held_by.is_some()));
}

#[test]
fn the_sticks_spend_signs_and_finishes_on_the_spend_tab() {
    use faraday_core::family::{FamilyAction as F, Route};
    let mut app = Faraday::new();
    display(&mut app);
    app.storage(StorageEvent::Restored {
        inbox: testkit::backup_files().unwrap(),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app.press(Action::Nav(Screen::Family));
    app.press(Action::Family(F::Holding(Route::Vault)));
    let v = app
        .vault_files()
        .iter()
        .position(|f| f.name == "vault.ofv")
        .unwrap();
    app.press(Action::Family(F::Pick(v)));
    for c in testkit::BACKUP_VAULT_PASSPHRASE.chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    app.press(Action::Family(F::Unlock));
    for t in 1..10u64 {
        let _ = app.frame();
        app.event(Event::Tick { now_ms: t * 1000 });
    }
    app.press(Action::Vault(V::LoadChosen(0)));
    assert_eq!(app.session.keys.len(), 3);
    // The vault holds the seeds; the wallet opens by itself from its
    // description in Files, which the stick has in four forms.
    let w = app.family_wallet().expect("the description did not open");
    assert_eq!(app.session.wallets.len(), 1);
    assert!(
        app.session
            .slots(&app.session.wallets[w])
            .iter()
            .all(|s| s.held_by.is_some())
    );
    let p = app
        .inbox
        .iter()
        .position(|i| i.name == "taproot-multisig-unsigned.psbt")
        .unwrap();
    app.press(Action::Family(F::UsePsbt(p)));
    assert!(app.spend.as_ref().unwrap().wallet.is_some());
    app.press(Action::SignHere);
    assert!(
        app.spend.as_ref().unwrap().complete,
        "the spend did not complete"
    );
}
