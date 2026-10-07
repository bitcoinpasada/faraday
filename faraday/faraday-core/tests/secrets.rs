//! A secret leaves this device sealed in a vault unless the person says
//! otherwise after a warning. The FROST carry, the secret nonce the next
//! share signs with, goes into the open vault with its public PSBT to the
//! Outbox; the next device, with the vault unlocked, signs that PSBT with
//! the nonce from the vault, and the round leaves the vault once used.
//! With no vault, the carry reaches the Outbox only after the warning is
//! acknowledged, and is listed as an unprotected secret.

use faraday_core::secrets::Exposure;
use faraday_core::testkit;
use faraday_core::vaults::VaultAction as V;
use faraday_core::{Action, Faraday, Screen, Sheet, StorageCommand, StorageEvent};
use osk_shell_api::{App, BootState, DisplayInfo, Event, Key, SecureHardware};

fn kit_file(name: &str) -> (String, Vec<u8>) {
    testkit::files()
        .unwrap()
        .into_iter()
        .find(|(n, _)| n == name)
        .unwrap()
}

fn device(inbox: Vec<(String, Vec<u8>)>) -> Faraday {
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
        inbox,
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    app
}

fn type_text(app: &mut Faraday, text: &str) {
    for c in text.chars() {
        app.event(Event::Key(Key::Char(c)));
    }
}

fn find(app: &Faraday, name: &str) -> usize {
    app.inbox.iter().position(|i| i.name == name).unwrap()
}

fn add_key(app: &mut Faraday, n: usize) {
    app.press(Action::Entry(None));
    type_text(app, &testkit::test_words(testkit::TEST_SEEDS[n].0));
    app.press(Action::EntryAdd);
}

fn unlock(app: &mut Faraday) {
    app.vaults.ms_per_unit = Some(180);
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Open(0)));
    type_text(app, testkit::VAULT_PASSPHRASES[0]);
    app.press(Action::Vault(V::Unlock));
    for t in 1..60u64 {
        if !app.vaults.open.is_empty() {
            break;
        }
        let _ = app.frame();
        app.event(Event::Tick { now_ms: t * 1000 });
    }
    assert_eq!(app.vaults.open.len(), 1, "the test vault did not unlock");
}

/// Device A: test key 1 signs as share 1 for shares 1 and 2, leaving the
/// carry on its way out.
fn first_share(app: &mut Faraday) {
    add_key(app, 0);
    let w = find(app, "threshold-wallet.txt");
    app.press(Action::LoadWallet(w));
    let p = find(app, "threshold-unsigned.psbt");
    app.press(Action::StartSpend(p));
    for n in [1u8, 2, 3] {
        app.press(Action::StepNext(n));
    }
    app.press(Action::TOther(1));
    app.press(Action::StepNext(4));
    app.press(Action::SignHere);
    assert!(app.spend.as_ref().unwrap().carry_out.is_some());
    app.press(Action::CarryToOutbox);
    assert_eq!(app.sheet, Some(Sheet::SecretOut));
    assert!(app.outbox.is_empty(), "nothing goes out before a choice");
}

#[test]
fn without_a_vault_the_carry_goes_out_only_after_the_warning() {
    let mut app = device(vec![
        kit_file("threshold-wallet.txt"),
        kit_file("threshold-unsigned.psbt"),
    ]);
    first_share(&mut app);
    app.press(Action::SecretUnprotected);
    assert!(app.outbox.is_empty(), "not acknowledged yet");
    app.press(Action::SecretAck);
    app.press(Action::SecretUnprotected);
    assert_eq!(app.outbox.len(), 1);
    assert_eq!(app.outbox[0].exposure(), Exposure::Secret);
    // And a visit does not write it unless ticked there.
    assert_eq!(app.sheet, None);
}

#[test]
fn cancelling_keeps_the_carry_for_later() {
    let mut app = device(vec![
        kit_file("threshold-wallet.txt"),
        kit_file("threshold-unsigned.psbt"),
    ]);
    first_share(&mut app);
    app.press(Action::Cancel);
    assert!(app.outbox.is_empty());
    assert!(app.spend.as_ref().unwrap().carry_out.is_some());
}

#[test]
fn the_carry_goes_into_the_vault_and_the_next_device_signs_with_it() {
    let mut a = device(vec![
        kit_file("threshold-wallet.txt"),
        kit_file("threshold-unsigned.psbt"),
        kit_file("vault.ofv"),
    ]);
    unlock(&mut a);
    first_share(&mut a);
    a.press(Action::SecretVault);
    assert_eq!(a.sheet, None);
    let names: Vec<(&str, Exposure)> = a
        .outbox
        .iter()
        .map(|i| (i.name.as_str(), i.kind.exposure()))
        .collect();
    assert_eq!(names, vec![("threshold-part.psbt", Exposure::Public)]);
    // Locking seals the vault, round and all, into the Outbox.
    a.press(Action::Lock);
    let out: Vec<(String, Vec<u8>)> = a
        .outbox
        .iter()
        .map(|i| (i.name.clone(), i.bytes.clone()))
        .collect();
    assert!(out.iter().any(|(n, _)| n == "vault.ofv"));
    assert!(!out.iter().any(|(n, _)| n.ends_with(".osk")));

    // Device B: test key 2, the vault, and the public PSBT.
    let mut inbox = vec![kit_file("threshold-wallet.txt")];
    inbox.extend(out);
    let mut b = device(inbox);
    unlock(&mut b);
    add_key(&mut b, 1);
    let w = find(&b, "threshold-wallet.txt");
    b.press(Action::LoadWallet(w));
    let p = find(&b, "threshold-part.psbt");
    b.press(Action::StartSpend(p));
    assert!(
        b.spend.as_ref().unwrap().carry.is_some(),
        "the vault's round was not found for this PSBT"
    );
    for n in [1u8, 2, 3, 4] {
        b.press(Action::StepNext(n));
    }
    b.press(Action::SignHere);
    let s = b.spend.as_ref().unwrap();
    assert!(s.complete, "{:?}", s.error);
    // The round has been used: it leaves the vault.
    let v = &b.vaults.open[0];
    assert!(v.changes > 0);
    assert!(
        v.contents
            .of(faraday_vault::records::kind::ROUND)
            .next()
            .is_none()
    );
}

/// What the last box save before a lock handed the next process.
type Boxes = (
    Vec<(String, Vec<u8>)>,
    Vec<(String, Vec<u8>)>,
    Vec<(String, Vec<u8>)>,
);

fn locked(app: &mut Faraday) -> Boxes {
    while app.poll_storage().is_some() {}
    app.press(Action::Lock);
    let mut last = None;
    while let Some(c) = app.poll_storage() {
        if let StorageCommand::SaveBoxes {
            inbox,
            outbox,
            kept,
        } = c
        {
            last = Some((inbox, outbox, kept));
        }
    }
    last.expect("the lock saved the boxes")
}

#[test]
fn a_secret_let_out_stays_a_secret_across_a_lock() {
    // A Lightning node key reads as plain text; it is still a secret.
    let mut app = device(Vec::new());
    add_key(&mut app, 0);
    app.press(Action::Lightning);
    let fp = app.session.keys[0].master.fingerprint().0;
    app.press(Action::LKey(fp));
    app.press(Action::LOut);
    app.press(Action::SecretAck);
    app.press(Action::SecretUnprotected);
    assert_eq!(app.outbox.len(), 1);
    assert_eq!(app.outbox[0].exposure(), Exposure::Secret);

    let (inbox, outbox, kept) = locked(&mut app);
    let mut next = device(Vec::new());
    next.storage(StorageEvent::Restored {
        inbox,
        outbox,
        kept,
    });
    assert_eq!(next.outbox.len(), 1);
    assert_eq!(
        next.outbox[0].exposure(),
        Exposure::Secret,
        "after the lock it would be written as a public file"
    );
}

#[test]
fn a_lock_leaves_no_secret_file_from_a_stick_in_the_inbox() {
    let words = format!("{}\n", testkit::test_words(testkit::TEST_SEEDS[0].0));
    let mut app = device(vec![
        ("seed-words.txt".to_string(), words.into_bytes()),
        (
            "recovery-codes.txt".to_string(),
            b"mail recovery\n1234 5678\n".to_vec(),
        ),
        kit_file("threshold-wallet.txt"),
        kit_file("vault.ofv"),
    ]);
    let k = find(&app, "seed-words.txt");
    app.press(Action::LoadKey(k));
    assert_eq!(app.session.keys.len(), 1);

    let (inbox, _, _) = locked(&mut app);
    let names: Vec<&str> = inbox.iter().map(|(n, _)| n.as_str()).collect();
    assert_eq!(
        names,
        ["threshold-wallet.txt", "vault.ofv"],
        "the words and the text are wiped with the session; the public wallet and the sealed vault stay"
    );
}
