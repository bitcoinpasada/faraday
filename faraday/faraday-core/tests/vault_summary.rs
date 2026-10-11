//! What a locked vault held is remembered across a lock
//! (`docs/SIMPLIFY.md` §3.4, `docs/VAULT.md` §11): the Vaults list names
//! the wallet and key a vault held when last seen open, in the next
//! process after a lock; after power-off, with no kept state, it says to
//! unlock it to see. Each row reads its currency too (§3.5).

use faraday_core::vaults::VaultAction as V;
use faraday_core::{Action, Faraday, Screen, StorageCommand, StorageEvent, testkit};
use osk_shell_api::{App, BootState, DisplayInfo, EntropyBytes, Event, Key, SecureHardware};

type Files = Vec<(String, Vec<u8>)>;

fn shown(inbox: Files, outbox: Files, kept: Files) -> Faraday {
    let mut app = Faraday::new();
    app.event(Event::Display(DisplayInfo {
        width: 1920,
        height: 1080,
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
        inbox,
        outbox,
        kept,
    });
    app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    app.storage(StorageEvent::Clock {
        unix_secs: 1_791_000_000,
    });
    app.vaults.ms_per_unit = Some(180);
    let _ = app.frame();
    app
}

fn type_text(app: &mut Faraday, text: &str) {
    for c in text.chars() {
        app.event(Event::Key(Key::Char(c)));
    }
}

fn settle(app: &mut Faraday, until: impl Fn(&Faraday) -> bool) {
    for t in 1..60u64 {
        if until(app) {
            break;
        }
        let _ = app.frame();
        app.event(Event::Tick { now_ms: t * 1000 });
    }
}

/// What the lock saved: the Inbox, the Outbox and the kept state.
fn saved(app: &mut Faraday) -> (Files, Files, Files) {
    let mut last = (Vec::new(), Vec::new(), Vec::new());
    while let Some(c) = app.poll_storage() {
        if let StorageCommand::SaveBoxes {
            inbox,
            outbox,
            kept,
        } = c
        {
            last = (inbox, outbox, kept);
        }
    }
    last
}

/// A vault made here holding test key 1 and the Spending wallet, locked:
/// what the lock saved, the wallet's name and the key's fingerprint.
fn locked_with_a_wallet_and_a_key() -> ((Files, Files, Files), String, String) {
    let wallet = testkit::files()
        .unwrap()
        .into_iter()
        .find(|(n, _)| n == "spending-wallet.txt")
        .unwrap();
    let mut app = shown(vec![wallet], Vec::new(), Vec::new());
    for i in 0..8u8 {
        app.event(Event::Entropy(EntropyBytes::new([0x60 + i; 32])));
    }
    app.press(Action::Entry(None));
    type_text(&mut app, &testkit::test_words(testkit::TEST_SEEDS[0].0));
    app.press(Action::EntryAdd);
    app.press(Action::LoadWallet(0));
    let name = app.session.wallets[0].name.clone();
    let fp = faraday_core::wallet::fp_text(app.session.keys[0].master.fingerprint());

    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Create));
    for second in [false, true] {
        app.press(Action::Vault(V::CFocus(0, second)));
        type_text(&mut app, "test phrase");
    }
    app.press(Action::Vault(V::CGo));
    settle(&mut app, |a| a.screen == Screen::Unlock);
    type_text(&mut app, "test phrase");
    app.press(Action::Vault(V::Unlock));
    settle(&mut app, |a| !a.vaults.open.is_empty());
    assert_eq!(app.vaults.open.len(), 1, "the vault did not open");
    app.press(Action::Vault(V::AddKind(0)));
    app.press(Action::Vault(V::SaveKey(0)));
    app.press(Action::Vault(V::AddKind(1)));
    app.press(Action::Vault(V::SaveWallet(0)));
    app.press(Action::Lock);
    (saved(&mut app), name, fp)
}

#[test]
fn after_a_lock_the_vaults_list_names_the_vaults_wallet_and_key() {
    let ((inbox, outbox, kept), wallet, fp) = locked_with_a_wallet_and_a_key();
    let mut app = shown(inbox, outbox, kept);
    app.press(Action::Nav(Screen::Vaults));
    let texts = app.drawn_texts();
    assert!(
        texts
            .iter()
            .any(|t| t.contains(&wallet) && t.contains(&format!("key {fp}"))),
        "the locked vault's row names {wallet} and key {fp}: {texts:?}"
    );
    assert!(
        texts.iter().any(|t| t == "Main"),
        "the row goes by the slot's name: {texts:?}"
    );
}

#[test]
fn after_power_off_the_vaults_list_does_not_name_what_it_held() {
    let ((inbox, outbox, _), _, fp) = locked_with_a_wallet_and_a_key();
    // Power-off: the files come back from the stick; the kept state does
    // not.
    let mut app = shown(inbox, outbox, Vec::new());
    app.press(Action::Nav(Screen::Vaults));
    let texts = app.drawn_texts();
    assert!(
        !texts.iter().any(|t| t.contains(&fp)),
        "nothing of the vault is remembered: {texts:?}"
    );
    assert!(
        texts
            .iter()
            .any(|t| t.contains("Unlock to see what it holds")),
        "the row says how to see it: {texts:?}"
    );
}

/// Answers every write the visit sends as written and matched.
fn answer_writes(app: &mut Faraday) {
    while let Some(c) = app.poll_storage() {
        if let StorageCommand::Write { stick, name, .. } = c {
            app.storage(StorageEvent::Written {
                stick,
                wrote_as: name.clone(),
                name,
            });
        }
    }
}

/// The Vaults list's lines as drawn.
fn vaults_list(app: &mut Faraday) -> Vec<String> {
    app.press(Action::Nav(Screen::Vaults));
    app.drawn_texts()
}

/// A vault's currency (`docs/SIMPLIFY.md` §3.5), on its Vaults list row:
/// never written once made; current on the stick once a visit wrote it;
/// changed once it is open with a change, and still after the lock that
/// seals the change.
#[test]
fn a_vaults_currency_reads_never_written_then_current_then_changed() {
    let mut app = shown(Vec::new(), Vec::new(), Vec::new());
    for i in 0..8u8 {
        app.event(Event::Entropy(EntropyBytes::new([0x70 + i; 32])));
    }
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Create));
    for second in [false, true] {
        app.press(Action::Vault(V::CFocus(0, second)));
        type_text(&mut app, "test phrase");
    }
    app.press(Action::Vault(V::CGo));
    settle(&mut app, |a| a.screen == Screen::Unlock);
    let texts = vaults_list(&mut app);
    assert!(
        texts.iter().any(|t| t.contains("Never written")),
        "made and never written: {texts:?}"
    );

    // The passphrase was typed: the next process writes it.
    app.press(Action::Lock);
    let (inbox, outbox, kept) = saved(&mut app);
    let mut app = shown(inbox, outbox, kept);
    app.storage(StorageEvent::Sticks(vec![faraday_core::StickInfo {
        id: "S".into(),
        label: "VAULTSTICK".into(),
        boot: false,
        files: Vec::new(),
    }]));
    app.press(Action::Nav(Screen::Visit));
    app.press(Action::VisitWrite);
    answer_writes(&mut app);
    app.storage(StorageEvent::Sticks(Vec::new()));
    let texts = vaults_list(&mut app);
    assert!(
        texts.iter().any(|t| t.contains("On VAULTSTICK · current")),
        "written and matched: {texts:?}"
    );

    // Opened with a key saved into it: changed since written.
    for i in 0..8u8 {
        app.event(Event::Entropy(EntropyBytes::new([0x78 + i; 32])));
    }
    app.press(Action::Entry(None));
    type_text(&mut app, &testkit::test_words(testkit::TEST_SEEDS[0].0));
    app.press(Action::EntryAdd);
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Open(0)));
    type_text(&mut app, "test phrase");
    app.press(Action::Vault(V::Unlock));
    settle(&mut app, |a| !a.vaults.open.is_empty());
    assert_eq!(app.vaults.open.len(), 1, "the written vault did not open");
    app.press(Action::Vault(V::AddKind(0)));
    app.press(Action::Vault(V::SaveKey(0)));
    let texts = vaults_list(&mut app);
    assert!(
        texts.iter().any(|t| t.contains("Changed since written")),
        "open with a change: {texts:?}"
    );
    // Sealed at the lock, its bytes are not the ones written.
    app.press(Action::Lock);
    let (inbox, outbox, kept) = saved(&mut app);
    let mut app = shown(inbox, outbox, kept);
    let texts = vaults_list(&mut app);
    assert!(
        texts.iter().any(|t| t.contains("Changed since written")),
        "sealed with the change: {texts:?}"
    );
}

/// A vault copied in from a stick and not changed reads as current on
/// that stick, across the lock that follows; one whose stick is not known
/// reads "Unchanged" (§3.5).
#[test]
fn a_vault_copied_in_reads_current_on_its_stick_or_unchanged() {
    let vault = testkit::test_vault().unwrap();
    let mut app = shown(Vec::new(), Vec::new(), Vec::new());
    app.storage(StorageEvent::Sticks(vec![faraday_core::StickInfo {
        id: "S".into(),
        label: "VAULTSTICK".into(),
        boot: false,
        files: vec![("vault.ofv".into(), vault.len() as u64)],
    }]));
    app.press(Action::Nav(Screen::Visit));
    app.press(Action::VisitCopy);
    while let Some(c) = app.poll_storage() {
        if let StorageCommand::Read { stick, name } = c {
            app.storage(StorageEvent::Read {
                stick,
                name,
                bytes: vault.clone(),
            });
        }
    }
    app.storage(StorageEvent::Sticks(Vec::new()));
    let texts = vaults_list(&mut app);
    assert!(
        texts.iter().any(|t| t.contains("On VAULTSTICK · current")),
        "copied in, unchanged: {texts:?}"
    );
    // The next process remembers where it came from.
    app.press(Action::Lock);
    let (inbox, outbox, kept) = saved(&mut app);
    let mut next = shown(inbox.clone(), outbox.clone(), kept);
    let texts = vaults_list(&mut next);
    assert!(
        texts.iter().any(|t| t.contains("On VAULTSTICK · current")),
        "after the lock: {texts:?}"
    );
    // With nothing kept, the stick is not known.
    let mut fresh = shown(inbox, outbox, Vec::new());
    let texts = vaults_list(&mut fresh);
    assert!(
        texts.iter().any(|t| t.contains("Unchanged")),
        "stick not known: {texts:?}"
    );
}
