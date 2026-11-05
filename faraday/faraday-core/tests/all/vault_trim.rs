//! Removing a seed's codex32 set or BIP-85 origin from its entry in an open
//! vault, as a person does it from the seed's page: the button while the
//! entry has the data, the dialog, and what the file holds after a seal.

use crate::common::type_text;
use faraday_bip::bip39::{Language, Mnemonic};
use faraday_core::strings::EN;
use faraday_core::vault::VaultAction as V;
use faraday_core::{Action, Faraday, Screen, StorageCommand, StorageEvent, testkit};
use faraday_shell_api::{App, BootState, DisplayInfo, EntropyBytes, Event, SecureHardware};
use faraday_vault::Record;
use faraday_vault::child::{ChildOf, payload as child_payload};
use faraday_vault::codex32::{SetFacts, State, set_payload};
use faraday_vault::records::{field, kind};

type Files = Vec<(String, Vec<u8>)>;

const PHRASE: &str = "trim phrase";
const KEYS: usize = 0;

fn process(inbox: Files) -> Faraday {
    let mut app = Faraday::new();
    app.event(Event::Display(DisplayInfo {
        width: 480,
        height: 1600,
        dpi: 286,
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
    app.storage(StorageEvent::Clock {
        unix_secs: 1_791_000_000,
    });
    testkit::vault_speed(&mut app, Some(100));
    app.storage(StorageEvent::Restored {
        inbox,
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    for i in 0..8u8 {
        app.event(Event::Entropy(EntropyBytes::new([0x40 + i; 32])));
    }
    app.press(Action::ReduceMotion(true));
    app
}

fn unlock(app: &mut Faraday, at: usize) {
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Open(at)));
    let _ = app.frame();
    app.press(Action::Vault(V::FocusPassphrase));
    type_text(app, PHRASE);
    app.press(Action::Vault(V::Unlock));
    for t in 1..60u64 {
        if !app.vaults().open.is_empty() || app.sheet.is_some() {
            break;
        }
        let _ = app.frame();
        app.event(Event::Tick { now_ms: t * 1000 });
    }
    assert_eq!(app.vaults().open.len(), 1, "the vault did not open");
    // The Load sheet an unlock shows: not now.
    app.press(Action::Vault(V::LoadNotNow));
}

fn opened(file: Vec<u8>) -> Faraday {
    let mut app = process(vec![("a.kdbx".into(), file)]);
    unlock(&mut app, 0);
    app
}

fn words_payload() -> Vec<u8> {
    let c = testkit::compute();
    let words = Mnemonic::parse(&c, Language::English, &testkit::test_words("summer")).unwrap();
    faraday_vault::records::words_payload(&words).to_vec()
}

fn set() -> Vec<u8> {
    set_payload(&SetFacts {
        threshold: 2,
        identifier: *b"name",
        length: 48,
        unknown: 1,
        strings: vec![(b'a', State::Placed), (b'c', State::Made)],
    })
    .unwrap()
}

fn origin() -> Vec<u8> {
    child_payload(&ChildOf {
        parent: [1, 2, 3, 4],
        language: 0,
        words: 24,
        index: 7,
    })
    .unwrap()
}

/// A seed with a name, a passphrase, and the set and the origin as asked.
fn seed(with_set: bool, with_origin: bool) -> Record {
    let mut r = Record::new(kind::KEY)
        .with(field::KEY, &words_payload())
        .with(field::KEY_LABEL, b"Savings");
    if with_set {
        r.push(field::KEY_CODEX32, &set());
    }
    if with_origin {
        r.push(field::KEY_CHILD, &origin());
    }
    r
}

fn vault(records: Vec<Record>) -> Vec<u8> {
    testkit::kdbx_vault("Alpha", PHRASE, 1, records).unwrap()
}

/// The fields of open vault 0's key record, by number.
fn fields(app: &Faraday) -> Vec<(u8, Vec<u8>)> {
    let id = app.vaults().open[0].id;
    app.vaults()
        .store()
        .contents_for_tests(id)
        .iter()
        .find(|r| r.kind == kind::KEY)
        .expect("a key record")
        .fields
        .iter()
        .map(|f| (f.number, f.bytes.to_vec()))
        .collect()
}

fn has(app: &Faraday, number: u8) -> bool {
    fields(app).iter().any(|(n, _)| *n == number)
}

fn page(app: &mut Faraday) -> String {
    let _ = app.frame();
    app.drawn_texts().join(" ")
}

/// The seed's page.
fn seed_page(app: &mut Faraday) -> String {
    app.press(Action::Nav(Screen::VaultContents));
    app.press(Action::Vault(V::Category(KEYS)));
    app.press(Action::Vault(V::Item(0)));
    page(app)
}

/// The files written by a lock.
fn lock(app: &mut Faraday) -> Files {
    crate::common::top_up(app);
    app.press(Action::Lock);
    testkit::settle(app);
    let mut written = Vec::new();
    while let Some(c) = app.poll_storage() {
        if let StorageCommand::SaveBoxes { outbox, .. } = c {
            written = outbox;
        }
    }
    written
}

/// The same vault after a seal, a lock and an unlock.
fn resealed(app: &mut Faraday) -> Faraday {
    let written = lock(app);
    assert_eq!(written.len(), 1, "the vault was sealed");
    let mut app = process(written);
    unlock(&mut app, 0);
    app
}

fn remove(app: &mut Faraday, number: u8) {
    app.press(Action::Vault(V::TrimAsk(number)));
    app.press(Action::Vault(V::TrimGo));
}

#[test]
fn removing_the_set_keeps_the_words_the_origin_and_the_name_through_a_reopen() {
    let mut app = opened(vault(vec![seed(true, true)]));
    let before = fields(&app);
    seed_page(&mut app);
    remove(&mut app, field::KEY_CODEX32);
    let mut after = resealed(&mut app);
    let want: Vec<_> = before
        .into_iter()
        .filter(|(n, _)| *n != field::KEY_CODEX32)
        .collect();
    assert_eq!(fields(&after), want);
    let texts = seed_page(&mut after);
    assert!(!texts.contains(EN.vault_trim_codex32), "{texts}");
    assert!(texts.contains(EN.vault_trim_child), "{texts}");
}

#[test]
fn removing_the_origin_keeps_the_set_and_the_rest_through_a_reopen() {
    let mut app = opened(vault(vec![seed(true, true)]));
    let before = fields(&app);
    seed_page(&mut app);
    remove(&mut app, field::KEY_CHILD);
    let after = resealed(&mut app);
    let want: Vec<_> = before
        .into_iter()
        .filter(|(n, _)| *n != field::KEY_CHILD)
        .collect();
    assert_eq!(fields(&after), want);
}

#[test]
fn the_sealed_file_differs_from_the_one_opened() {
    let file = vault(vec![seed(true, false)]);
    let mut app = opened(file.clone());
    seed_page(&mut app);
    remove(&mut app, field::KEY_CODEX32);
    let written = lock(&mut app);
    assert_eq!(written.len(), 1);
    assert_ne!(written[0].1, file);
    let mut fresh = process(written);
    unlock(&mut fresh, 0);
    assert!(!has(&fresh, field::KEY_CODEX32));
}

#[test]
fn the_dialog_names_the_vault_and_says_older_copies_still_hold_it() {
    let mut app = opened(vault(vec![seed(true, true)]));
    let texts = seed_page(&mut app);
    assert!(texts.contains(EN.vault_trim_codex32), "{texts}");
    assert!(texts.contains(EN.vault_trim_child), "{texts}");
    app.press(Action::Vault(V::TrimAsk(field::KEY_CODEX32)));
    let texts = page(&mut app);
    assert!(
        texts.contains("Copies of Alpha sealed before still hold the set"),
        "{texts}"
    );
}

#[test]
fn cancel_changes_nothing() {
    let mut app = opened(vault(vec![seed(true, true)]));
    let before = fields(&app);
    seed_page(&mut app);
    app.press(Action::Vault(V::TrimAsk(field::KEY_CHILD)));
    app.press(Action::Vault(V::TrimCancel));
    assert_eq!(fields(&app), before);
    // A press of Remove with no dialog up does nothing either.
    app.press(Action::Vault(V::TrimGo));
    assert_eq!(fields(&app), before);
}

#[test]
fn a_record_with_neither_offers_neither() {
    let mut app = opened(vault(vec![seed(false, false)]));
    let texts = seed_page(&mut app);
    assert!(!texts.contains(EN.vault_trim_codex32), "{texts}");
    assert!(!texts.contains(EN.vault_trim_child), "{texts}");
    let before = fields(&app);
    remove(&mut app, field::KEY_CODEX32);
    assert_eq!(fields(&app), before);
}

#[test]
fn a_record_with_only_the_origin_offers_only_that() {
    let mut app = opened(vault(vec![seed(false, true)]));
    let texts = seed_page(&mut app);
    assert!(!texts.contains(EN.vault_trim_codex32), "{texts}");
    assert!(texts.contains(EN.vault_trim_child), "{texts}");
}

#[test]
fn a_read_only_vault_offers_neither_and_removes_nothing() {
    let file = testkit::kdbx_edited(
        "Odd",
        PHRASE,
        9,
        |doc| {
            let c = testkit::compute();
            faraday_vault::kdbx::items::put(
                &c,
                doc,
                &faraday_vault::kdbx::items::Place {
                    uuid: [7; 16],
                    group: EN.vault_group_keys,
                    group_uuid: [8; 16],
                    title: "Savings",
                    now: None,
                },
                &faraday_vault::kdbx::items::Body::Record(seed(true, true)),
            )
            .unwrap();
        },
        |xml| {
            let no_bin = "<RecycleBinUUID>AAAAAAAAAAAAAAAAAAAAAA==</RecycleBinUUID>";
            xml.replace(no_bin, &format!("{no_bin}{no_bin}"))
        },
    )
    .unwrap();
    let mut app = opened(file);
    // What a read-only file holds of Faraday's own data reads as damaged,
    // so no seed page is reached; a press of the action still asks nothing.
    let texts = seed_page(&mut app);
    assert!(!texts.contains(EN.vault_trim_codex32), "{texts}");
    assert!(!texts.contains(EN.vault_trim_child), "{texts}");
    app.press(Action::Vault(V::TrimAsk(field::KEY_CODEX32)));
    assert_eq!(app.sheet, None);
}

#[test]
fn the_sessions_own_set_still_reads_after_the_removal() {
    let mut app = opened(vault(vec![seed(true, true)]));
    app.press(Action::Vault(V::LoadAll(0)));
    let k = app.session.keys.len() - 1;
    assert!(app.session.keys.info(k).unwrap().codex32.is_some());
    seed_page(&mut app);
    remove(&mut app, field::KEY_CODEX32);
    assert!(!has(&app, field::KEY_CODEX32));
    assert!(app.session.keys.info(k).unwrap().codex32.is_some());
}

#[test]
fn a_change_while_the_dialog_is_up_makes_the_press_do_nothing() {
    let mut app = opened(vault(vec![seed(true, true)]));
    let before = fields(&app);
    seed_page(&mut app);
    app.press(Action::Vault(V::TrimAsk(field::KEY_CODEX32)));
    let id = app.vaults().open[0].id;
    let note = Record::new(kind::ENTRY).with(field::TITLE, b"Mail");
    testkit::vault_edit(&mut app, id, |s| s.insert(id, note));
    app.press(Action::Vault(V::TrimGo));
    assert_eq!(fields(&app), before);
}
