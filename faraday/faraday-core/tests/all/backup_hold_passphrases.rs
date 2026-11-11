//! The backup hold (`docs/BACKUP-FLOW.md` §10.2): backing up a key with one
//! passphrase of some words releases that key, and the words alone, and no
//! other passphrase of the same words.

use crate::common::type_text;
use faraday_bip::keys::Fingerprint;
use faraday_core::keygen::Way;
use faraday_core::plan::Preset;
use faraday_core::testkit::{self, backup_as};
use faraday_core::{Action, Faraday, bstep, cstep};

/// Words made here, locked in as a key with no passphrase, then given the
/// passphrase `one` and `two` in Create's slot, each a key made here. Create
/// is left on the wallet of the `two` key. Returns the fingerprints of the
/// words alone, of `one` and of `two`.
fn words_with_two_passphrases(app: &mut Faraday) -> [faraday_bip::keys::Fingerprint; 3] {
    app.press(Action::KeyGen(None));
    app.press(Action::KStart(faraday_core::keygen::Start::Make.index()));
    app.press(Action::KWords(12));
    app.press(Action::KWay(Way::Coins.index()));
    app.press(Action::KNext);
    for i in 0..128 {
        app.press(Action::KFlip((i * 5 + i / 3) % 2 == 0));
    }
    app.press(Action::KNext);
    app.press(Action::KLock);
    let bare = app.session.keys.last().unwrap().fingerprint;
    app.press(Action::CreateWallet);
    crate::common::ckind(app, 0);
    app.press(Action::CVaults(false));
    app.press(Action::CNext(cstep::WHERE));
    let mut with = Vec::new();
    for pass in ["one", "two"] {
        app.press(Action::CSlotHere(0, bare.0));
        app.press(Action::CPassOpen(0));
        type_text(app, pass);
        app.press(Action::CPassField(1));
        type_text(app, pass);
        app.press(Action::CPassLock);
        with.push(app.session.keys.last().unwrap().fingerprint);
    }
    [bare, with[0], with[1]]
}

#[test]
fn backing_up_one_passphrase_of_some_words_holds_the_other_passphrase() {
    let c = testkit::compute();
    let mut app = testkit::started();
    let [bare, one, two] = words_with_two_passphrases(&mut app);
    assert!(app.key_held(&c, one) && app.key_held(&c, two));
    app.press(Action::CNext(cstep::KEYS));
    app.press(Action::CNext(cstep::CHECK));
    app.press(Action::CBackup);
    backup_as(&mut app, Preset::Paper);
    app.press(Action::BChecklist);
    if app.backup_items_now(&c).contains(&bstep::BLANK) {
        app.press(Action::BNext(bstep::BLANK));
    }
    testkit::type_copy(&mut app);
    testkit::type_passphrases(&mut app);
    if app.backup().unwrap().open != Some(bstep::ENVELOPE) {
        app.press(Action::BStep(bstep::ENVELOPE));
    }
    app.press(Action::BNext(bstep::ENVELOPE));
    assert!(!app.key_held(&c, two), "the key backed up is released");
    assert!(!app.key_held(&c, bare), "the words alone are released");
    assert!(
        app.key_held(&c, one),
        "the other passphrase is written nowhere"
    );
}

#[test]
fn a_passphrase_backed_up_is_released_while_another_passphrase_of_its_words_waits_in_an_open_vault()
{
    use crate::backup_flow_wallet::PHRASE;
    use faraday_vault::Record;
    use faraday_vault::records::{self, field, kind};
    let c = testkit::compute();
    let words = testkit::test_words(testkit::TEST_SEEDS[1].0);
    let m = faraday_bip::bip39::Mnemonic::parse(&c, faraday_bip::bip39::Language::English, &words)
        .unwrap();
    // The words kept in a vault with the passphrase `one`, its backup
    // pending.
    let waiting = Record::new(kind::KEY)
        .with(field::KEY, &records::words_payload(&m))
        .with(field::KEY_PASSPHRASE, b"one")
        .with(field::KEY_FLAGS, &[records::BACKUP_PENDING]);
    let file = crate::common::vault_of("Main", PHRASE, 0x41, vec![waiting]);
    let mut app = crate::common::process(vec![("main.kdbx".into(), file)], Vec::new(), Vec::new());
    crate::common::keep_time(&mut app);
    crate::backup_flow_wallet::load_first_vault(&mut app, PHRASE);
    assert_eq!(app.vaults().open.len(), 1);
    let one = app
        .session
        .keys
        .all()
        .find(|k| k.has_passphrase)
        .map(|k| k.fingerprint)
        .expect("the key of the record loaded");
    let bare = crate::backup_flow_wallet::key_in(&mut app, 1);
    app.press(Action::CreateWallet);
    crate::common::ckind(&mut app, 0);
    app.press(Action::CVaults(false));
    app.press(Action::CNext(cstep::WHERE));
    app.press(Action::CSlotHere(0, bare));
    app.press(Action::CPassOpen(0));
    type_text(&mut app, "two");
    app.press(Action::CPassField(1));
    type_text(&mut app, "two");
    app.press(Action::CPassLock);
    let two = app.session.keys.last().unwrap().fingerprint;
    assert_ne!(one, two);
    assert!(app.key_held(&c, two) && app.key_held(&c, one));
    app.press(Action::CNext(cstep::KEYS));
    app.press(Action::CNext(cstep::CHECK));
    app.press(Action::CBackup);
    backup_as(&mut app, Preset::Paper);
    app.press(Action::BChecklist);
    if app.backup_items_now(&c).contains(&bstep::BLANK) {
        app.press(Action::BNext(bstep::BLANK));
    }
    testkit::type_copy(&mut app);
    testkit::type_passphrases(&mut app);
    if app.backup().unwrap().open != Some(bstep::ENVELOPE) {
        app.press(Action::BStep(bstep::ENVELOPE));
    }
    app.press(Action::BNext(bstep::ENVELOPE));
    let name = faraday_bip::keys::Fingerprint::to_string(&two);
    assert!(!app.key_held(&c, two), "the key backed up is released");
    assert!(app.toast_text().is_some_and(|t| t.contains(&name)));
    assert!(app.key_held(&c, one), "the other passphrase still waits");
    testkit::vault_lock_one(&mut app, 0);
    assert!(!app.key_held(&c, two), "released after the lock too");
    assert!(app.key_held(&c, one));
}

#[test]
fn a_passphrase_typed_again_on_words_saved_without_it_stays_held_after_a_reload() {
    use crate::backup_flow_wallet::PHRASE;
    use faraday_vault::Record;
    use faraday_vault::records::{self, field, kind};
    let c = testkit::compute();
    let words = testkit::test_words(testkit::TEST_SEEDS[1].0);
    let m = faraday_bip::bip39::Mnemonic::parse(&c, faraday_bip::bip39::Language::English, &words)
        .unwrap();
    // The words of a key made here with a passphrase, saved on Vaults
    // without it: the record says the backup is pending, as the lock left it.
    let saved = Record::new(kind::KEY)
        .with(field::KEY, &records::words_payload(&m))
        .with(field::KEY_FLAGS, &[records::BACKUP_PENDING]);
    let file = crate::common::vault_of("Main", PHRASE, 0x41, vec![saved]);
    let mut app = crate::common::process(vec![("main.kdbx".into(), file)], Vec::new(), Vec::new());
    crate::common::keep_time(&mut app);
    crate::backup_flow_wallet::load_first_vault(&mut app, PHRASE);
    assert_eq!(app.vaults().open.len(), 1);
    // The passphrase typed again on the loaded words.
    app.press(Action::Entry(None));
    type_text(&mut app, &words);
    crate::common::bip39_passphrase(&mut app, "x");
    app.press(Action::EntryAdd);
    let with = app
        .session
        .keys
        .all()
        .find(|k| k.has_passphrase)
        .map(|k| k.fingerprint)
        .expect("the key with its passphrase");
    assert!(
        app.key_held(&c, with),
        "no copy holds its passphrase: it does not sign"
    );
}

/// The words of the second test seed, kept in a vault with `pass` or with
/// none, the record saying its backup is pending, as a lock leaves it. The
/// vault is unlocked and its key loaded.
fn vault_pending(pass: Option<&str>) -> Faraday {
    vault_of_words(pass, true)
}

/// The words of the second test seed, kept in a vault with `pass` or with
/// none, the record saying its backup is pending when `pending`. The vault
/// is unlocked and its key loaded.
fn vault_of_words(pass: Option<&str>, pending: bool) -> Faraday {
    use crate::backup_flow_wallet::PHRASE;
    use faraday_vault::Record;
    use faraday_vault::records::{self, field, kind};
    let c = testkit::compute();
    let words = testkit::test_words(testkit::TEST_SEEDS[1].0);
    let m = faraday_bip::bip39::Mnemonic::parse(&c, faraday_bip::bip39::Language::English, &words)
        .unwrap();
    let mut r = Record::new(kind::KEY).with(field::KEY, &records::words_payload(&m));
    if let Some(p) = pass {
        r = r.with(field::KEY_PASSPHRASE, p.as_bytes());
    }
    if pending {
        r = r.with(field::KEY_FLAGS, &[records::BACKUP_PENDING]);
    }
    let file = crate::common::vault_of("Main", PHRASE, 0x41, vec![r]);
    let mut app = crate::common::process(vec![("main.kdbx".into(), file)], Vec::new(), Vec::new());
    crate::common::keep_time(&mut app);
    crate::backup_flow_wallet::load_first_vault(&mut app, PHRASE);
    assert_eq!(app.vaults().open.len(), 1);
    app
}

/// Make a new wallet of key `fp`, with passphrase `pass` given it in
/// Create's slot when there is one (a key made here), and leaves Create on
/// it. Returns the wallet's key.
fn wallet_of(app: &mut Faraday, fp: Fingerprint, pass: Option<&str>) -> Fingerprint {
    app.press(Action::CreateWallet);
    crate::common::ckind(app, 0);
    app.press(Action::CVaults(false));
    app.press(Action::CNext(cstep::WHERE));
    app.press(Action::CSlotHere(0, fp.0));
    match pass {
        Some(pass) => pass_in_slot(app, fp, pass),
        None => fp,
    }
}

/// Passphrase `pass` given to key `fp` in Create's slot: a key made here,
/// the wallet's key from now. Returns it.
pub(crate) fn pass_in_slot(app: &mut Faraday, fp: Fingerprint, pass: &str) -> Fingerprint {
    app.press(Action::CSlotHere(0, fp.0));
    app.press(Action::CPassOpen(0));
    type_text(app, pass);
    app.press(Action::CPassField(1));
    type_text(app, pass);
    app.press(Action::CPassLock);
    app.session.keys.last().unwrap().fingerprint
}

/// Whether a key record in open vault `o` says its backup is pending.
pub(crate) fn vault_says_pending(app: &Faraday, o: usize) -> bool {
    use crate::common::RecordsOf;
    use faraday_vault::records::{self, field, kind};
    app.vaults()
        .store()
        .contents_for_tests(app.vaults().open[o].id)
        .of(kind::KEY)
        .any(|(_, r)| {
            r.field(field::KEY_FLAGS)
                .and_then(|f| f.first())
                .is_some_and(|b| b & records::BACKUP_PENDING != 0)
        })
}

/// Backs up the wallet Create is on, on paper: every checklist item done.
pub(crate) fn back_up_on_paper(app: &mut Faraday) {
    let c = testkit::compute();
    app.press(Action::CNext(cstep::KEYS));
    app.press(Action::CNext(cstep::CHECK));
    app.press(Action::CBackup);
    backup_as(app, Preset::Paper);
    app.press(Action::BChecklist);
    if app.backup_items_now(&c).contains(&bstep::BLANK) {
        app.press(Action::BNext(bstep::BLANK));
    }
    testkit::type_copy(app);
    testkit::type_passphrases(app);
    if app.backup().unwrap().open != Some(bstep::ENVELOPE) {
        app.press(Action::BStep(bstep::ENVELOPE));
    }
    app.press(Action::BNext(bstep::ENVELOPE));
}

/// Whether the last toast names key `fp` as signing from now.
pub(crate) fn toast_names(app: &Faraday, fp: Fingerprint) -> bool {
    app.toast_text()
        .is_some_and(|t| t.contains(&Fingerprint::to_string(&fp)))
}

#[test]
fn the_words_alone_of_a_passphrase_waiting_in_a_vault_are_held_until_a_copy_of_them_is_written() {
    let c = testkit::compute();
    let mut app = vault_pending(Some("one"));
    let one = app
        .session
        .keys
        .all()
        .find(|k| k.has_passphrase)
        .map(|k| k.fingerprint)
        .expect("the key of the record loaded");
    let bare = Fingerprint(crate::backup_flow_wallet::key_in(&mut app, 1));
    assert!(
        app.key_held(&c, bare),
        "the words alone are written nowhere either"
    );
    assert!(app.key_held(&c, one));
    wallet_of(&mut app, bare, None);
    back_up_on_paper(&mut app);
    assert!(!app.key_held(&c, bare), "a copy of the words is written");
    assert!(toast_names(&app, bare));
    assert!(app.key_held(&c, one), "its passphrase is written nowhere");
    testkit::vault_lock_one(&mut app, 0);
    assert!(!app.key_held(&c, bare), "released after the lock too");
    assert!(app.key_held(&c, one));
}

#[test]
fn a_passphrase_backed_up_on_words_saved_without_one_is_released_and_the_last_backup_clears_the_mark()
 {
    let c = testkit::compute();
    let mut app = vault_pending(None);
    let bare = app.session.keys.last().unwrap().fingerprint;
    assert!(app.key_held(&c, bare));
    let one = wallet_of(&mut app, bare, Some("one"));
    let two = pass_in_slot(&mut app, bare, "two");
    assert!(app.key_held(&c, one) && app.key_held(&c, two));
    back_up_on_paper(&mut app);
    assert!(!app.key_held(&c, two), "the key backed up signs");
    assert!(toast_names(&app, two));
    assert!(!app.key_held(&c, bare), "a copy of the words is written");
    assert!(app.key_held(&c, one), "the other passphrase waits");
    assert!(vault_says_pending(&app, 0), "kept for the other passphrase");
    // The vault locked and unlocked again.
    testkit::vault_lock_one(&mut app, 0);
    testkit::settle(&mut app);
    assert!(app.vaults().open.is_empty());
    assert!(!app.key_held(&c, two) && app.key_held(&c, one));
    crate::backup_flow_wallet::load_first_vault(&mut app, crate::backup_flow_wallet::PHRASE);
    assert_eq!(app.vaults().open.len(), 1);
    assert!(!app.key_held(&c, two), "not held again by the vault");
    assert!(!app.key_held(&c, bare));
    assert!(app.key_held(&c, one));
    wallet_of(&mut app, one, None);
    back_up_on_paper(&mut app);
    assert!(toast_names(&app, one));
    assert!(app.held_keys_now(&c).is_empty(), "each key has a copy");
    assert!(!vault_says_pending(&app, 0), "a reload holds none of them");
}

#[test]
fn another_passphrase_typed_on_words_waiting_in_a_vault_with_one_is_held_until_its_copy_is_written()
{
    let c = testkit::compute();
    let mut app = vault_pending(Some("one"));
    let one = app
        .session
        .keys
        .all()
        .find(|k| k.has_passphrase)
        .map(|k| k.fingerprint)
        .expect("the key of the record loaded");
    app.press(Action::Entry(None));
    type_text(&mut app, &testkit::test_words(testkit::TEST_SEEDS[1].0));
    crate::common::bip39_passphrase(&mut app, "two");
    app.press(Action::EntryAdd);
    let two = app.session.keys.last().unwrap().fingerprint;
    assert_ne!(one, two);
    assert!(
        app.key_held(&c, two),
        "the vault's words wait for a copy of every passphrase of them"
    );
    wallet_of(&mut app, two, None);
    back_up_on_paper(&mut app);
    assert!(!app.key_held(&c, two), "its copy is written");
    assert!(toast_names(&app, two));
    assert!(
        app.key_held(&c, one),
        "the vault's own passphrase still waits"
    );
    assert!(vault_says_pending(&app, 0));
}

#[test]
fn a_copy_of_the_passphrase_waiting_in_a_vault_releases_the_words_alone_too() {
    let c = testkit::compute();
    let mut app = vault_pending(Some("one"));
    let one = app
        .session
        .keys
        .all()
        .find(|k| k.has_passphrase)
        .map(|k| k.fingerprint)
        .expect("the key of the record loaded");
    let bare = Fingerprint(crate::backup_flow_wallet::key_in(&mut app, 1));
    assert!(app.key_held(&c, bare) && app.key_held(&c, one));
    wallet_of(&mut app, one, None);
    back_up_on_paper(&mut app);
    assert!(toast_names(&app, one));
    assert!(!app.key_held(&c, one), "its copy is written");
    assert!(!app.key_held(&c, bare), "the copy holds the words");
    assert!(!vault_says_pending(&app, 0), "a reload holds neither");
}

#[test]
fn a_vault_locked_while_the_last_passphrase_of_its_words_is_backed_up_holds_none_of_them_after_a_restart()
 {
    use crate::backup_flow_wallet::{PHRASE, load_first_vault};
    let c = testkit::compute();
    let mut app = vault_of_words(None, false);
    let bare = app.session.keys.last().unwrap().fingerprint;
    let one = wallet_of(&mut app, bare, Some("one"));
    let two = pass_in_slot(&mut app, bare, "two");
    back_up_on_paper(&mut app);
    assert!(app.key_held(&c, one), "the other passphrase waits");
    // The vault locked: its words wait for `one`.
    testkit::vault_lock_one(&mut app, 0);
    testkit::settle(&mut app);
    load_first_vault(&mut app, PHRASE);
    assert!(vault_says_pending(&app, 0), "kept for the other passphrase");
    // Locked again, then the last passphrase backed up.
    testkit::vault_lock_one(&mut app, 0);
    testkit::settle(&mut app);
    assert!(app.vaults().open.is_empty());
    wallet_of(&mut app, one, None);
    back_up_on_paper(&mut app);
    assert!(toast_names(&app, one));
    assert!(app.held_keys_now(&c).is_empty(), "each key has a copy");
    // Unlocked again in the same session: its mark goes.
    load_first_vault(&mut app, PHRASE);
    assert_eq!(app.vaults().open.len(), 1);
    assert!(app.held_keys_now(&c).is_empty());
    assert!(
        !vault_says_pending(&app, 0),
        "each key of its words has a copy"
    );
    // Locked and started again: the words loaded from it sign.
    let mut shell = crate::common::Shell::new();
    let mut app = shell.lock(&mut app);
    load_first_vault(&mut app, PHRASE);
    assert_eq!(app.vaults().open.len(), 1);
    assert!(app.session.keys.all().any(|k| k.fingerprint == bare));
    assert!(!app.key_held(&c, bare), "not held again after a restart");
    assert!(!app.key_held(&c, one) && !app.key_held(&c, two));
}

#[test]
fn another_passphrase_made_on_words_kept_in_a_vault_with_one_is_held_after_a_restart() {
    use crate::backup_flow_wallet::{PHRASE, load_first_vault};
    let c = testkit::compute();
    // The words kept with the passphrase `one`, nothing pending.
    let mut app = vault_of_words(Some("one"), false);
    let one = app.session.keys.last().unwrap().fingerprint;
    assert!(!app.key_held(&c, one));
    let bare = Fingerprint(crate::backup_flow_wallet::key_in(&mut app, 1));
    let two = wallet_of(&mut app, bare, Some("two"));
    assert!(app.key_held(&c, two), "made here");
    // Locked and started again, the vault unlocked and `two` typed again.
    let mut shell = crate::common::Shell::new();
    let mut app = shell.lock(&mut app);
    load_first_vault(&mut app, PHRASE);
    assert_eq!(app.vaults().open.len(), 1);
    assert!(vault_says_pending(&app, 0), "the lock kept the hold");
    app.press(Action::Entry(None));
    type_text(&mut app, &testkit::test_words(testkit::TEST_SEEDS[1].0));
    crate::common::bip39_passphrase(&mut app, "two");
    app.press(Action::EntryAdd);
    let again = app.session.keys.last().unwrap().fingerprint;
    assert_eq!(again, two);
    assert!(app.key_held(&c, two), "its passphrase is written nowhere");
    assert!(app.key_held(&c, one), "held again until a copy of it");
    // A copy of `two` releases it; the mark stays for `one`.
    wallet_of(&mut app, two, None);
    back_up_on_paper(&mut app);
    assert!(toast_names(&app, two));
    assert!(!app.key_held(&c, two), "its copy is written");
    assert!(app.key_held(&c, one));
    assert!(vault_says_pending(&app, 0));
    // A copy of `one`, the passphrase the vault stores, clears it.
    wallet_of(&mut app, one, None);
    back_up_on_paper(&mut app);
    assert!(toast_names(&app, one));
    assert!(app.held_keys_now(&c).is_empty(), "each key has a copy");
    assert!(!vault_says_pending(&app, 0));
}

#[test]
fn a_vault_marked_in_an_earlier_session_keeps_its_hold_when_another_passphrase_is_backed_up() {
    use crate::backup_flow_wallet::{PHRASE, key_in, load_first_vault};
    let c = testkit::compute();
    // The words kept with no passphrase; `one` made on them here.
    let mut app = vault_of_words(None, false);
    let bare = app.session.keys.last().unwrap().fingerprint;
    let one = wallet_of(&mut app, bare, Some("one"));
    assert!(app.key_held(&c, one));
    // Locked and started again: the vault's words wait for `one`.
    let mut shell = crate::common::Shell::new();
    let mut app = shell.lock(&mut app);
    // `two` made on the words typed again and backed up, the vault not open.
    let typed = Fingerprint(key_in(&mut app, 1));
    assert_eq!(typed, bare);
    let two = wallet_of(&mut app, bare, Some("two"));
    back_up_on_paper(&mut app);
    assert!(toast_names(&app, two));
    assert!(app.held_keys_now(&c).is_empty(), "each key here has a copy");
    load_first_vault(&mut app, PHRASE);
    assert_eq!(app.vaults().open.len(), 1);
    assert!(
        vault_says_pending(&app, 0),
        "kept for `one`, which is not here"
    );
    // `one` typed again: written nowhere, it does not sign.
    app.press(Action::Entry(None));
    type_text(&mut app, &testkit::test_words(testkit::TEST_SEEDS[1].0));
    crate::common::bip39_passphrase(&mut app, "one");
    app.press(Action::EntryAdd);
    assert_eq!(app.session.keys.last().unwrap().fingerprint, one);
    assert!(app.key_held(&c, one));
}

#[test]
fn an_older_copy_marked_in_an_earlier_session_keeps_its_hold_when_the_newer_copy_is_cleared() {
    use crate::backup_flow_wallet::{PHRASE, key_in, load_first_vault};
    use faraday_core::vault::VaultAction as V;
    let c = testkit::compute();
    // The words kept with the passphrase `x`; `z` made on them here.
    let mut app = vault_of_words(Some("x"), false);
    let x = app.session.keys.last().unwrap().fingerprint;
    let bare = Fingerprint(key_in(&mut app, 1));
    let z = wallet_of(&mut app, bare, Some("z"));
    assert!(app.key_held(&c, z), "made here");
    // Locked: the vault's words wait for `z`. That file is kept as `old.kdbx`.
    let mut shell = crate::common::Shell::new();
    let mut app = shell.lock(&mut app);
    let older = shell.boxes.1.iter().find(|f| f.0 == "main.kdbx").unwrap();
    let older = ("old.kdbx".to_string(), older.1.clone());
    // Another key saved in it and locked again: the newer copy, still marked.
    load_first_vault(&mut app, PHRASE);
    assert!(vault_says_pending(&app, 0));
    key_in(&mut app, 2);
    app.press(Action::Vault(V::SaveKey(app.session.keys.len() - 1)));
    shell.lock(&mut app);
    assert_ne!(shell.boxes.1[0].1, older.1, "sealed again");
    // Started again with the older copy in Files beside the newer one, on a
    // computer that keeps nothing of the earlier sessions: the two copies
    // are told apart only once each is opened.
    shell.boxes.0.push(older);
    shell.boxes.2.clear();
    let mut app = shell.restart_unseen();
    // The newer copy opened: `x`'s copy, written while it is open, clears it.
    load_first_vault(&mut app, PHRASE);
    assert_eq!(app.vaults().open.len(), 1);
    assert!(vault_says_pending(&app, 0));
    assert!(app.session.keys.all().any(|k| k.fingerprint == x));
    wallet_of(&mut app, x, None);
    back_up_on_paper(&mut app);
    assert!(toast_names(&app, x));
    assert!(!vault_says_pending(&app, 0), "its passphrase's copy");
    // `y` made on the words, the vault locked (marked for `y`), `y` copied.
    let typed = Fingerprint(key_in(&mut app, 1));
    assert_eq!(typed, bare);
    let y = wallet_of(&mut app, bare, Some("y"));
    testkit::vault_lock_one(&mut app, 0);
    testkit::settle(&mut app);
    assert!(app.vaults().open.is_empty());
    back_up_on_paper(&mut app);
    assert!(toast_names(&app, y));
    assert!(app.held_keys_now(&c).is_empty(), "each key here has a copy");
    // The older copy unlocked: its mark, from the first session, is for `z`.
    app.press(Action::Nav(faraday_core::Screen::Vaults));
    let row = app
        .vault_files()
        .iter()
        .position(|f| f.name == "old.kdbx")
        .expect("the older copy has its own row");
    app.press(Action::Vault(V::Open(row)));
    testkit::vault_speed(&mut app, Some(180));
    crate::common::unlock(&mut app, PHRASE);
    if app.sheet == Some(faraday_core::Sheet::OlderCopy) {
        app.press(Action::Vault(V::OlderOpen));
    }
    if app.sheet == Some(faraday_core::Sheet::UnlockLoad) {
        app.press(Action::Vault(V::LoadTicked));
    }
    assert_eq!(app.vaults().open.len(), 1);
    assert!(
        vault_says_pending(&app, 0),
        "kept for `z`, which is not here"
    );
    // `z` typed again: written nowhere, it does not sign.
    app.press(Action::Entry(None));
    type_text(&mut app, &testkit::test_words(testkit::TEST_SEEDS[1].0));
    crate::common::bip39_passphrase(&mut app, "z");
    app.press(Action::EntryAdd);
    assert_eq!(app.session.keys.last().unwrap().fingerprint, z);
    assert!(app.key_held(&c, z));
}
