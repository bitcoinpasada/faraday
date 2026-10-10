//! The backup's checklist, past the copy by hand, offers the seed into
//! the open vault and as a file where the plan puts it there. Into the vault it is the key record
//! Vaults saves, which loads at the next unlock; from the secret sheet
//! too it goes in as a key, never as a note. As a file it goes to the
//! Outbox only once the warning is ticked: the words, or the SeedQR as a
//! labelled picture, Standard or Compact. Each reads back as the same
//! key, the words from a stick, the picture through Add a key's scanner.
//! No file holds the BIP-39 passphrase, and a stick visit does not tick
//! the file for writing. The backup's panel says where the wallet and
//! each of its seeds are kept.

use faraday_core::backup::{Kept, Tone};
use faraday_core::seeds::SeedsAction as S;
use faraday_core::testkit;
use faraday_core::vaults::VaultAction as V;
use faraday_core::wallet::FileKind;
use faraday_core::{Action, Faraday, Screen, Sheet, StickInfo, StorageEvent, bstep, plan, qrow};
use faraday_vault::records::kind;
use osk_bip::keys::Fingerprint;
use osk_shell_api::{App, BootState, DisplayInfo, Event, Key, SecureHardware};

/// Test key 2: the test vault holds test key 1 only.
const SEED: usize = 1;
const PASSPHRASE: &str = "lantern-orchid-41";

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
    let _ = app.frame();
    app
}

fn type_text(app: &mut Faraday, text: &str) {
    for c in text.chars() {
        app.event(Event::Key(Key::Char(c)));
    }
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
    app.press(Action::Nav(Screen::Backup));
}

/// Test key 2 typed into Add a key, with the passphrase when `with`, a
/// one-key wallet made from it, and its backup's plan made: paper, the
/// vault and a file; the seed's copy open.
/// Returns the key's fingerprint.
fn backing_up(app: &mut Faraday, with: bool) -> Fingerprint {
    app.press(Action::Entry(None));
    type_text(app, &testkit::test_words(testkit::TEST_SEEDS[SEED].0));
    if with {
        app.press(Action::EntryPassphrase);
        type_text(app, PASSPHRASE);
    }
    app.press(Action::EntryAdd);
    let key = app.session.keys.last().expect("the key was not added");
    assert_eq!(key.passphrase.is_some(), with);
    let fp = key.master.fingerprint();
    app.press(Action::KeyWallet(fp.0, 1));
    app.press(Action::Seeds(S::Make));
    let w = app.session.wallets.len() - 1;
    app.press(Action::Backup(w));
    app.press(Action::BPreset(1));
    app.press(Action::BAnswer(qrow::SEEDS, plan::seeds::FILE as u8));
    app.press(Action::BChecklist);
    app.press(Action::BStep(bstep::COPY));
    app.press(Action::BReveal);
    let _ = app.frame();
    fp
}

fn keys_in_vault(app: &Faraday) -> usize {
    app.vaults.open[0].contents.of(kind::KEY).count()
}

fn notes_in_vault(app: &Faraday) -> usize {
    app.vaults.open[0].contents.of(kind::NOTE).count()
}

/// Locks `app`, unlocks the vault it sealed on a device of its own and
/// loads what that vault marks to load: the fingerprints of the keys
/// loaded.
fn reopened(app: &mut Faraday) -> Vec<Fingerprint> {
    app.press(Action::Lock);
    let vault = app
        .outbox
        .iter()
        .find(|i| i.name == "vault.ofv")
        .map(|i| (i.name.clone(), i.bytes.clone()))
        .expect("the lock sealed the vault into the Outbox");
    let mut next = device(vec![vault]);
    unlock(&mut next);
    next.press(Action::Vault(V::LoadChosen(0)));
    next.session
        .keys
        .iter()
        .map(|k| k.master.fingerprint())
        .collect()
}

/// The seed as a file in the Outbox, in form `form` (0 the words, 1 the
/// SeedQR), past the warning. Returns the file's name and bytes.
fn seed_file(app: &mut Faraday, form: u8) -> (String, Vec<u8>) {
    app.press(Action::BFile);
    assert_eq!(app.sheet, Some(Sheet::SecretOut));
    app.press(Action::SecretForm(form));
    app.press(Action::SecretAck);
    app.press(Action::SecretUnprotected);
    assert_eq!(app.sheet, None);
    let item = app.outbox.last().expect("nothing reached the Outbox");
    assert!(item.secret, "{} is not listed as a secret", item.name);
    (item.name.clone(), item.bytes.clone())
}

/// The fingerprint Add a key's scanner loads from a SeedQR picture.
fn scanned(png: &[u8]) -> Fingerprint {
    let payloads = faraday_files::qr_in_png(png).expect("a PNG");
    assert_eq!(payloads.len(), 1, "one code in the picture");
    let mut app = device(Vec::new());
    app.press(Action::ScanSeed);
    assert_eq!(app.sheet, Some(Sheet::Scan));
    app.event(Event::Scanned {
        bytes: payloads[0].clone(),
    });
    let _ = app.frame();
    assert_eq!(app.session.keys.len(), 1, "the scanner loaded no key");
    app.session.keys[0].master.fingerprint()
}

const STICK: &str = "S1";

/// The secrets For the stick: the checklist's public files go there on
/// their own, a seed only past the secret sheet.
fn secrets_out(app: &Faraday) -> usize {
    app.outbox.iter().filter(|i| i.secret).count()
}

#[test]
fn saved_into_the_vault_from_the_seeds_step_the_key_loads_at_the_next_unlock() {
    let mut app = device(vec![kit_file("vault.ofv")]);
    let fp = backing_up(&mut app, false);
    unlock(&mut app);
    let before = keys_in_vault(&app);
    app.press(Action::BVault(false));
    assert_eq!(keys_in_vault(&app), before + 1);
    // Saved once: pressed again, it is not saved twice.
    app.press(Action::BVault(false));
    assert_eq!(keys_in_vault(&app), before + 1);
    assert!(reopened(&mut app).contains(&fp));
}

#[test]
fn with_its_passphrase_the_vault_keeps_the_key_as_it_was_loaded() {
    let mut app = device(vec![kit_file("vault.ofv")]);
    let fp = backing_up(&mut app, true);
    unlock(&mut app);
    app.press(Action::BVault(true));
    assert!(reopened(&mut app).contains(&fp));
}

#[test]
fn the_secret_sheet_saves_a_seed_as_a_key_and_not_a_note() {
    let mut app = device(vec![kit_file("vault.ofv")]);
    let fp = backing_up(&mut app, false);
    unlock(&mut app);
    let (keys, notes) = (keys_in_vault(&app), notes_in_vault(&app));
    app.press(Action::BFile);
    assert_eq!(app.sheet, Some(Sheet::SecretOut));
    app.press(Action::SecretVault);
    assert_eq!(app.sheet, None);
    assert_eq!(keys_in_vault(&app), keys + 1);
    assert_eq!(notes_in_vault(&app), notes, "the seed went in as a note");
    assert!(app.outbox.iter().all(|i| !i.secret), "a file went out too");
    assert!(reopened(&mut app).contains(&fp));
}

#[test]
fn the_unprotected_button_does_nothing_until_the_warning_is_ticked() {
    let mut app = device(Vec::new());
    backing_up(&mut app, false);
    app.press(Action::BFile);
    app.press(Action::SecretUnprotected);
    assert_eq!(secrets_out(&app), 0, "out before the warning was ticked");
    assert_eq!(app.sheet, Some(Sheet::SecretOut));
    // Ticked and unticked again, still nothing.
    app.press(Action::SecretAck);
    app.press(Action::SecretAck);
    app.press(Action::SecretUnprotected);
    assert_eq!(secrets_out(&app), 0);
    app.press(Action::SecretAck);
    app.press(Action::SecretUnprotected);
    assert_eq!(secrets_out(&app), 1);
}

#[test]
fn cancelling_the_sheet_lets_nothing_out() {
    let mut app = device(Vec::new());
    backing_up(&mut app, false);
    app.press(Action::BFile);
    app.press(Action::SecretAck);
    app.press(Action::Cancel);
    assert!(app.secret_out.is_none(), "the secret is still held");
    assert_eq!(secrets_out(&app), 0);
    // Opened again, the warning is unticked.
    app.press(Action::BFile);
    app.press(Action::SecretUnprotected);
    assert_eq!(secrets_out(&app), 0);
}

#[test]
fn the_words_file_read_from_a_stick_loads_the_same_key() {
    let mut app = device(Vec::new());
    let fp = backing_up(&mut app, false);
    let (name, bytes) = seed_file(&mut app, 0);
    assert!(name.ends_with("-words.txt"), "{name}");

    let mut next = testkit::started();
    next.storage(StorageEvent::Sticks(vec![StickInfo {
        id: STICK.to_string(),
        label: "TESTSTICK".to_string(),
        boot: false,
        files: vec![(name.clone(), bytes.len() as u64)],
    }]));
    next.press(Action::Nav(Screen::Visit));
    next.press(Action::VisitIn(0));
    next.press(Action::VisitCopy);
    next.storage(StorageEvent::Read {
        stick: STICK.to_string(),
        name: name.clone(),
        bytes,
    });
    next.storage(StorageEvent::Sticks(Vec::new()));
    let k = next
        .inbox
        .iter()
        .position(|i| i.name == name)
        .expect("not copied in");
    next.press(Action::LoadKey(k));
    assert_eq!(next.session.keys.len(), 1, "the key did not load");
    assert_eq!(next.session.keys[0].master.fingerprint(), fp);
}

#[test]
fn the_seedqr_picture_scans_back_as_the_same_key_standard_and_compact() {
    for compact in [false, true] {
        let mut app = device(Vec::new());
        let fp = backing_up(&mut app, false);
        app.press(Action::BCompact(compact));
        let (name, png) = seed_file(&mut app, 1);
        let want = if compact {
            "-compactseedqr.png"
        } else {
            "-seedqr.png"
        };
        assert!(name.ends_with(want), "{name}");
        assert_eq!(scanned(&png), fp, "{name}");
    }
}

#[test]
fn no_file_holds_the_passphrase() {
    let mut app = device(Vec::new());
    let fp = backing_up(&mut app, true);
    let mut files = vec![seed_file(&mut app, 0), seed_file(&mut app, 1)];
    app.press(Action::BCompact(true));
    files.push(seed_file(&mut app, 1));
    for (name, bytes) in &files {
        let mut read = vec![bytes.clone()];
        if name.ends_with(".png") {
            read.extend(faraday_files::qr_in_png(bytes).expect("a PNG"));
        }
        for b in read {
            assert!(
                !b.windows(PASSPHRASE.len())
                    .any(|w| w == PASSPHRASE.as_bytes()),
                "{name} holds the passphrase"
            );
        }
    }
    // The words alone are the key without its passphrase.
    let words = String::from_utf8(files[0].1.clone()).unwrap();
    assert_eq!(
        words.trim(),
        testkit::test_words(testkit::TEST_SEEDS[SEED].0)
    );
    assert_ne!(scanned(&files[1].1), fp, "the passphrase is in the code");
}

#[test]
fn a_stick_visit_leaves_the_seed_file_unticked() {
    for form in [0u8, 1] {
        let mut app = device(Vec::new());
        backing_up(&mut app, false);
        let (name, _) = seed_file(&mut app, form);
        // A public file beside it, which the visit does tick.
        app.outbox.push(faraday_core::Item {
            name: "wallet-descriptor.txt".into(),
            bytes: b"wpkh([00000000/84h/1h/0h]tpub)".to_vec(),
            kind: FileKind::Text,
            secret: false,
            picture: None,
        });
        app.storage(StorageEvent::Sticks(vec![StickInfo {
            id: STICK.to_string(),
            label: "TESTSTICK".to_string(),
            boot: false,
            files: Vec::new(),
        }]));
        app.press(Action::Nav(Screen::Visit));
        let _ = app.frame();
        assert!(app.visit.out.contains("wallet-descriptor.txt"));
        assert!(
            !app.visit.out.contains(&name),
            "{name} is ticked for writing"
        );
    }
}

fn kept(app: &Faraday) -> Kept {
    app.backup_kept().expect("no backup open")
}

fn line(text: &str, tone: Tone) -> (String, Tone) {
    (text.to_string(), tone)
}

/// Test key `n` typed into Add a key. Returns its index in the session.
fn add_key(app: &mut Faraday, n: usize) -> usize {
    app.press(Action::Entry(None));
    type_text(app, &testkit::test_words(testkit::TEST_SEEDS[n].0));
    app.press(Action::EntryAdd);
    app.session.keys.len() - 1
}

/// The test 2-of-3 over the test seeds, its backup open.
fn savings_backup(app: &mut Faraday) {
    app.session
        .add_wallet("Savings", &testkit::savings(), "test")
        .unwrap();
    app.press(Action::Backup(app.session.wallets.len() - 1));
    let _ = app.frame();
}

#[test]
fn the_panel_says_which_vault_holds_the_seed_and_the_wallet() {
    let mut app = device(vec![kit_file("vault.ofv")]);
    backing_up(&mut app, false);
    let locked = line("Not checked: no vault unlocked", Tone::Dim);
    assert_eq!(kept(&app).wallet, locked);
    assert_eq!(kept(&app).seeds[0].lines[0], locked);
    unlock(&mut app);
    assert_eq!(kept(&app).wallet, line("Wallet not in a vault", Tone::Warn));
    assert_eq!(
        kept(&app).seeds[0].lines[0],
        line("Not in a vault", Tone::Warn)
    );
    app.press(Action::BVault(false));
    assert_eq!(kept(&app).seeds[0].lines[0], line("In vault.ofv", Tone::Ok));
    let w = app.backup.as_ref().unwrap().wallet;
    app.press(Action::Vault(V::SaveWallet(w)));
    assert_eq!(kept(&app).wallet, line("Wallet in vault.ofv", Tone::Ok));
}

#[test]
fn saved_with_its_passphrase_the_panel_says_so() {
    let mut app = device(vec![kit_file("vault.ofv")]);
    backing_up(&mut app, true);
    unlock(&mut app);
    app.press(Action::BVault(true));
    assert_eq!(
        kept(&app).seeds[0].lines[0],
        line("In vault.ofv with its passphrase", Tone::Ok)
    );
}

#[test]
fn with_no_vault_the_seed_is_not_in_a_vault() {
    let mut app = device(Vec::new());
    backing_up(&mut app, false);
    let k = kept(&app);
    assert_eq!(k.wallet, line("Wallet not in a vault", Tone::Warn));
    assert_eq!(
        k.seeds[0].lines,
        vec![
            line("Not in a vault", Tone::Warn),
            line("Paper: not checked", Tone::Warn)
        ]
    );
}

#[test]
fn a_seed_file_for_the_stick_is_listed_as_unprotected() {
    let mut app = device(Vec::new());
    backing_up(&mut app, false);
    seed_file(&mut app, 0);
    assert!(
        kept(&app).seeds[0]
            .lines
            .contains(&line("File for the stick, unprotected", Tone::Err))
    );
}

#[test]
fn a_matched_copy_check_marks_that_seed_only() {
    use osk_bip::bip39::{Language, Mnemonic};
    let mut app = device(Vec::new());
    add_key(&mut app, 0);
    let k2 = add_key(&mut app, 1);
    savings_backup(&mut app);
    // The copy of the seed of test key 2, by its place in the wallet.
    let i = app
        .backup_seed_list(app.backup.as_ref().unwrap().wallet)
        .iter()
        .position(|(_, k)| *k == Some(k2))
        .expect("test key 2 is a seed of Savings") as u8;
    app.press(Action::BPreset(0));
    app.press(Action::BChecklist);
    app.press(Action::BStep(bstep::COPY + i));
    assert_eq!(app.backup.as_ref().unwrap().key, k2);
    app.press(Action::BReveal);
    let words = testkit::test_words(testkit::TEST_SEEDS[1].0);
    let digits = osk_codec::seedqr::to_digits(&Mnemonic::parse(Language::English, &words).unwrap());
    app.press(Action::BCheck);
    type_text(
        &mut app,
        std::str::from_utf8(digits.expose().as_bytes()).unwrap(),
    );
    let k = kept(&app);
    let checked = line("Paper copy checked", Tone::Ok);
    assert!(k.seeds[1].lines.contains(&checked), "{:?}", k.seeds[1]);
    assert!(!k.seeds[0].lines.contains(&checked), "{:?}", k.seeds[0]);
    assert!(
        k.seeds[0]
            .lines
            .contains(&line("Paper: not checked", Tone::Warn))
    );
}

#[test]
fn a_two_of_three_with_one_seed_here_lists_the_other_two_as_not_here() {
    let mut app = device(Vec::new());
    let k = add_key(&mut app, 1);
    let fp = app.session.keys[k].master.fingerprint();
    savings_backup(&mut app);
    let seeds = kept(&app).seeds;
    assert_eq!(seeds.len(), 3);
    let away = vec![line("Not here: backed up on its own device", Tone::Dim)];
    assert_eq!(seeds[0].lines, away);
    assert_eq!(seeds[2].lines, away);
    assert_eq!(seeds[1].name, faraday_core::wallet::fp_text(fp));
    assert_ne!(seeds[1].lines, away);
}
