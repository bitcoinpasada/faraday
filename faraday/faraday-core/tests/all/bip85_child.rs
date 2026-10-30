//! A BIP-85 child seed is recorded as one only inside a vault, and only
//! when the person says so (`docs/decisions/F166.md`): **Record it as a
//! child seed?** asks before child words go into a vault or load as a key;
//! recorded, the vault keeps the parent's fingerprint and the index across
//! a seal and an unlock and Keys shows it; not recorded, nothing in the
//! vault ties the seed to its parent. Nothing written for a child seed's
//! backup names its parent, and a wallet made with a recorded child seed
//! says so once.

use crate::common::type_text;
use faraday_core::vault::VaultAction as V;
use faraday_core::{Action, Faraday, Screen, Sheet, StorageCommand, StorageEvent, cstep, testkit};
use faraday_shell_api::{App, EntropyBytes, Event};
use faraday_vault::records::{field, kind};

type Files = Vec<(String, Vec<u8>)>;

const PHRASE: &str = "child phrase";

/// A device with these files restored and test key 1 typed in.
fn process(inbox: Files, outbox: Files, kept: Files) -> Faraday {
    let mut app = testkit::started();
    app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    testkit::vault_speed(&mut app, Some(100));
    app.storage(StorageEvent::Restored {
        inbox,
        outbox,
        kept,
    });
    for i in 0..8u8 {
        app.event(Event::Entropy(EntropyBytes::new([0x50 + i; 32])));
    }
    app
}

fn add_parent(app: &mut Faraday) -> String {
    app.press(Action::Entry(None));
    type_text(app, &testkit::test_words(testkit::TEST_SEEDS[0].0));
    app.press(Action::EntryAdd);
    faraday_core::wallet::fp_text(app.session.keys[0].fingerprint).to_string()
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

fn unlock(app: &mut Faraday) {
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Open(0)));
    type_text(app, PHRASE);
    app.press(Action::Vault(V::Unlock));
    settle(app, |a| !a.vaults().open.is_empty());
    assert_eq!(app.vaults().open.len(), 1, "the vault did not open");
}

/// BIP-85 on the parent at child words, 24 of them, index 0, and `press`
/// on the value: the dialog it stops at.
fn child_words(app: &mut Faraday, press: Action) {
    app.press(Action::Bip85);
    app.press(Action::PApp(0));
    app.press(Action::PNext);
    app.press(Action::PNext);
    app.press(press);
    assert_eq!(app.sheet, Some(Sheet::ChildRecord), "no question asked");
    let _ = app.frame();
    let texts = app.drawn_texts();
    for t in ["Record it as a child seed?", "Don't record", "Record"] {
        assert!(texts.iter().any(|s| s == t), "{t}: {texts:?}");
    }
}

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

/// Child words put into a fresh vault, answered `record`, the vault
/// locked; the next power-on's device with the sealed copy unlocked and
/// its keys loaded. Returns it, the parent's fingerprint, and the sealed
/// file.
fn sealed_and_opened(record: bool) -> (Faraday, String, Vec<u8>) {
    let file = crate::common::vault("Child", &[PHRASE], 3);
    let mut app = process(vec![("child.fvlt".into(), file)], Vec::new(), Vec::new());
    let parent = add_parent(&mut app);
    unlock(&mut app);
    child_words(&mut app, Action::PVault);
    app.press(Action::PRecord(record));
    assert_eq!(app.sheet, None);
    let id = app.vaults().open[0].id;
    assert_eq!(app.vaults().records(id).of(kind::KEY).count(), 1);
    app.press(Action::Lock);
    let (inbox, outbox, kept) = saved(&mut app);
    let sealed = inbox
        .iter()
        .find(|(n, _)| n.ends_with(".fvlt"))
        .map(|(_, b)| b.clone())
        .expect("the sealed vault");
    let mut app = process(inbox, outbox, kept);
    unlock(&mut app);
    if app.sheet == Some(Sheet::UnlockLoad) {
        app.press(Action::Vault(V::LoadTicked));
    }
    (app, parent, sealed)
}

#[test]
fn a_child_seed_recorded_in_a_vault_keeps_its_mark_across_a_seal_and_shows_on_keys() {
    let (mut app, parent, _) = sealed_and_opened(true);
    let key = app
        .session
        .keys
        .all()
        .find(|k| k.child_of.is_some())
        .expect("the child seed loaded, recorded as one");
    let of = key.child_of.unwrap();
    assert_eq!(
        faraday_core::wallet::fp_text(faraday_bip::keys::Fingerprint(of.parent)).to_string(),
        parent
    );
    assert_eq!((of.index, of.words), (0, 24));
    // Its label is its own fingerprint; Keys shows the mark.
    let child_fp = faraday_core::wallet::fp_text(key.fingerprint).to_string();
    assert!(key.label.eq_ignore_ascii_case(&child_fp), "{}", key.label);
    let mark = format!("BIP-85 Words · {parent} · index 0");
    assert_eq!(faraday_core::bip85::key_name(key), mark);
    app.press(Action::Nav(Screen::VaultContents));
    let _ = app.frame();
    let texts = app.drawn_texts();
    assert!(texts.iter().any(|t| t.contains(&mark)), "{texts:?}");
}

#[test]
fn a_child_seed_not_recorded_leaves_no_trace_of_its_parent_in_the_vault() {
    let (app, parent, sealed) = sealed_and_opened(false);
    assert!(app.session.keys.all().all(|k| k.child_of.is_none()));
    let parent_bytes = app.session.keys[0].fingerprint.0;
    let c = testkit::compute();
    let (_, contents) = faraday_vault::open(&c, &sealed, PHRASE).unwrap();
    for r in &contents.records {
        assert!(r.field(field::KEY_CHILD).is_none(), "a child record");
        for f in &r.fields {
            let text = String::from_utf8_lossy(&f.bytes).to_lowercase();
            for gone in ["bip-85", "bip85", "83696968", "index", &parent] {
                assert!(!text.contains(gone), "{gone} in field {}", f.number);
            }
            assert!(
                !f.bytes.windows(4).any(|w| w == parent_bytes),
                "the parent's fingerprint in field {}",
                f.number
            );
        }
    }
}

#[test]
fn a_child_seed_loaded_as_a_key_is_ordinary_unless_recorded() {
    for record in [false, true] {
        let mut app = process(Vec::new(), Vec::new(), Vec::new());
        let parent = add_parent(&mut app);
        child_words(&mut app, Action::PLoad);
        app.press(Action::PRecord(record));
        assert_eq!(app.session.keys.len(), 2);
        let k = &app.session.keys[1];
        assert_eq!(k.child_of.is_some(), record);
        let name = faraday_core::bip85::key_name(k);
        assert_eq!(name.contains(&parent), record, "{name}");
    }
}

#[test]
fn the_question_put_away_puts_nothing_in() {
    let mut app = process(Vec::new(), Vec::new(), Vec::new());
    add_parent(&mut app);
    child_words(&mut app, Action::PLoad);
    app.press(Action::Cancel);
    assert_eq!(app.sheet, None);
    assert_eq!(app.session.keys.len(), 1);
}

/// A single-key wallet made in Create over session key `k`, stopped on
/// Check.
fn make_wallet(app: &mut Faraday, k: usize) -> usize {
    app.press(Action::CreateWallet);
    app.press(Action::CKind(0));
    app.press(Action::CNext(cstep::KIND));
    app.press(Action::CVaults(false));
    app.press(Action::CNext(cstep::WHERE));
    let fp = app.session.keys[k].fingerprint.0;
    app.press(Action::CSlotHere(0, fp));
    app.press(Action::CNext(cstep::KEYS));
    app.create().and_then(|c| c.built).expect("the wallet made")
}

#[test]
fn a_wallet_made_with_a_recorded_child_seed_says_so_once_and_an_ordinary_one_does_not() {
    let mut app = process(Vec::new(), Vec::new(), Vec::new());
    add_parent(&mut app);
    make_wallet(&mut app, 0);
    assert_ne!(app.sheet, Some(Sheet::ChildNotice), "an ordinary seed");

    let mut app = process(Vec::new(), Vec::new(), Vec::new());
    add_parent(&mut app);
    child_words(&mut app, Action::PLoad);
    app.press(Action::PRecord(true));
    make_wallet(&mut app, 1);
    assert_eq!(app.sheet, Some(Sheet::ChildNotice));
    let _ = app.frame();
    let texts = app.drawn_texts().join(" ");
    let child = faraday_core::wallet::fp_text(app.session.keys[1].fingerprint).to_string();
    assert!(texts.contains("A BIP-85 child seed"), "{texts}");
    assert!(texts.contains(&child), "{texts}");
    app.press(Action::Cancel);
    assert_eq!(app.sheet, None);
    // Check's own Continue does not bring it back.
    app.press(Action::CNext(cstep::CHECK));
    assert_ne!(app.sheet, Some(Sheet::ChildNotice));

    // Loaded unrecorded, the same wallet says nothing.
    let mut app = process(Vec::new(), Vec::new(), Vec::new());
    add_parent(&mut app);
    child_words(&mut app, Action::PLoad);
    app.press(Action::PRecord(false));
    make_wallet(&mut app, 1);
    assert_ne!(app.sheet, Some(Sheet::ChildNotice));
}

/// Whether `text` names the child's parent or where it came from.
fn names_parent(text: &str, parent: &str) -> bool {
    let t = text.to_lowercase();
    ["bip-85", "bip85", "83696968", "index ", parent]
        .iter()
        .any(|gone| t.contains(gone))
}

#[test]
fn nothing_written_for_a_recorded_child_seeds_backup_names_its_parent() {
    let mut app = process(Vec::new(), Vec::new(), Vec::new());
    let parent = add_parent(&mut app);
    child_words(&mut app, Action::PLoad);
    app.press(Action::PRecord(true));
    // Its words out unprotected: a seed's words file, named by its own
    // fingerprint, holding the words alone.
    app.press(Action::POut);
    assert_eq!(app.sheet, Some(Sheet::SecretOut));
    app.press(Action::SecretUnprotected);
    app.press(Action::SecretHold);
    let child = faraday_core::wallet::fp_text(app.session.keys[1].fingerprint).to_string();
    let item = app.outbox.last().expect("the file");
    assert_eq!(item.name, format!("{}-words.txt", child.to_lowercase()));
    let body = String::from_utf8_lossy(&item.bytes).to_string();
    assert!(!names_parent(&body, &parent), "{body}");
    assert_eq!(body.split_whitespace().count(), 24, "{body}");

    // A wallet over it, and its backup's paper: the words, Seed XOR's
    // parts, and the screen they are written from.
    let w = make_wallet(&mut app, 1);
    app.press(Action::Cancel);
    app.press(Action::Backup(w));
    let _ = app.frame();
    let mut texts = app.drawn_texts();
    app.press(Action::BXor(2));
    let _ = app.frame();
    texts.extend(app.drawn_texts());
    assert!(
        texts.iter().any(|t| t.contains(&child)),
        "not the child's backup: {texts:?}"
    );
    let leaks: Vec<&String> = texts.iter().filter(|t| names_parent(t, &parent)).collect();
    assert!(leaks.is_empty(), "{leaks:?}");
    for it in app.outbox.iter().chain(app.inbox().iter()) {
        assert!(!names_parent(&it.name, &parent), "{}", it.name);
    }
    assert!(!names_parent(&app.session.wallets[w].name, &parent));
}
