//! **Save as a QR vault** on the page of an item in an open file vault
//! (`docs/VAULT.md` §13): the button for a seed, a codex32 share, a BIP-39
//! passphrase and a note and for nothing else, the tool opened at its
//! Passphrase step with the item as its contents, a code that opens back to
//! the item, and a dialog for an item too long. The item stays in the vault.

use crate::common::type_text;
use faraday_bip::bip39::{Language, Mnemonic};
use faraday_core::qr_tool::{self, Pick, QrToolAction as Q};
use faraday_core::strings::{EN, fill};
use faraday_core::vault::VaultAction as V;
use faraday_core::{Action, Faraday, Screen, Sheet, StorageEvent, testkit};
use faraday_crypto::work::Compute;
use faraday_keys::{Keys, QrHolds, QrOpenJob, Reveal};
use faraday_shell_api::{App, BootState, DisplayInfo, EntropyBytes, Event, SecureHardware};
use faraday_vault::Record;
use faraday_vault::records::{field, kind, words_payload};

const PHRASE: &str = "save phrase";
const PASS: &str = "correct horse";
const ZOO: &str = "zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo vote";
const SHARE: &str = "ms12namecacdefghjklmnpqrstuvwxyz023ftr2gdzmpy6pn";
const SHARE_KEY: [u8; 4] = [0xfa, 0xb6, 0x86, 0x8a];
const TREZOR_KEY: [u8; 4] = [0xb4, 0xe3, 0xf5, 0xed];
const WALLET: &str = "wpkh([73c5da0a/84h/0h/0h]xpub6CatWdiZiodmUeTDp8LT5or8nmbKNcuyvz7WyksVFkKB4RHwCD3XyuvPEbvqAQY3rAPshWcMLoP2fMFMKHPJ4ZeZXYVUhLv1VMrjPC7PW6V/<0;1>/*)";
const KEYS: usize = 0;
const WALLETS: usize = 1;
const ENTRIES: usize = 2;
const NOTES: usize = 3;

fn process(inbox: Vec<(String, Vec<u8>)>) -> Faraday {
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

/// A process with the vault holding `records` unlocked.
fn opened(records: Vec<Record>) -> Faraday {
    let file = testkit::kdbx_vault("Alpha", PHRASE, 1, records).unwrap();
    let mut app = process(vec![("a.kdbx".into(), file)]);
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Open(0)));
    let _ = app.frame();
    app.press(Action::Vault(V::FocusPassphrase));
    type_text(&mut app, PHRASE);
    app.press(Action::Vault(V::Unlock));
    for t in 1..60u64 {
        if !app.vaults().open.is_empty() || app.sheet.is_some() {
            break;
        }
        let _ = app.frame();
        app.event(Event::Tick { now_ms: t * 1000 });
    }
    assert_eq!(app.vaults().open.len(), 1, "the vault did not open");
    app.press(Action::Vault(V::LoadNotNow));
    app
}

fn page(app: &mut Faraday) -> String {
    let _ = app.frame();
    app.drawn_texts().join(" ")
}

/// The page of the first item of a category.
fn item_page(app: &mut Faraday, category: usize) -> String {
    app.press(Action::Nav(Screen::VaultContents));
    app.press(Action::Vault(V::Category(category)));
    app.press(Action::Vault(V::Item(0)));
    page(app)
}

fn seed(phrase: &str) -> Record {
    let c = testkit::compute();
    let m = Mnemonic::parse(&c, Language::English, phrase).unwrap();
    Record::new(kind::KEY)
        .with(field::KEY, &words_payload(&m))
        .with(field::KEY_LABEL, b"Savings")
}

fn note(text: &str) -> Record {
    Record::new(kind::NOTE).with(field::NOTE, text.as_bytes())
}

fn share() -> Record {
    Record::new(kind::CODEX32_SHARE)
        .with(field::CODEX32_STRING, SHARE.as_bytes())
        .with(field::CODEX32_KEY, &SHARE_KEY)
}

fn passphrase_item(text: &str) -> Record {
    Record::new(kind::PASSPHRASE)
        .with(field::PASSPHRASE_KEY, &TREZOR_KEY)
        .with(field::PASSPHRASE, text.as_bytes())
}

/// A record's type and fields.
type Shape = (u8, Vec<(u8, Vec<u8>)>);

/// The vault's records, as they are.
fn records(app: &Faraday) -> Vec<Shape> {
    let id = app.vaults().open[0].id;
    app.vaults()
        .store()
        .contents_for_tests(id)
        .iter()
        .map(|r| {
            let fields = r.fields.iter().map(|f| (f.number, f.bytes.to_vec()));
            (r.kind, fields.collect())
        })
        .collect()
}

fn type_pass(app: &mut Faraday) {
    app.press(Action::QrTool(Q::Focus(qr_tool::Field::Pass)));
    type_text(app, PASS);
    app.press(Action::QrTool(Q::Focus(qr_tool::Field::Again)));
    type_text(app, PASS);
}

/// The button pressed, a passphrase typed and the code sealed.
fn save(app: &mut Faraday) {
    app.press(Action::Vault(V::QrSave));
    assert_eq!(app.screen, Screen::QrTool);
    type_pass(app);
    app.press(Action::QrTool(Q::Make));
    testkit::settle(app);
}

/// The code made, opened under the passphrase and held.
fn opened_code(app: &mut Faraday) -> (Keys, QrHolds) {
    app.press(Action::QrTool(Q::File));
    let item = app.outbox.first().expect("the code is in the Outbox");
    let c: Compute = testkit::compute();
    let mut keys = Keys::default();
    let opened = QrOpenJob::new(&item.bytes, PASS).run(&c).unwrap();
    let holds = keys.qr_hold(opened).unwrap();
    (keys, holds)
}

#[test]
fn a_note_is_saved_as_a_qr_vault_that_opens_back_to_it_and_stays_in_the_vault() {
    let mut app = opened(vec![note("meet at the old oak")]);
    let before = records(&app);
    let texts = item_page(&mut app, NOTES);
    assert!(texts.contains(EN.vault_qr_save), "{texts}");
    app.press(Action::Vault(V::QrSave));
    assert_eq!(app.screen, Screen::QrTool);
    let s = qr_tool::state(&app).unwrap();
    assert_eq!(s.pick(), Some(Pick::Item));
    let texts = page(&mut app);
    assert!(
        texts.contains(EN.qr_tool_pass_again),
        "the Passphrase step is open"
    );
    app.press(Action::Back);
    let texts = page(&mut app);
    assert!(
        texts.contains(&fill(EN.qr_tool_item_note, &["Alpha"])),
        "{texts}"
    );
    app.press(Action::QrTool(Q::Step(1)));
    let _ = page(&mut app);
    type_pass(&mut app);
    app.press(Action::QrTool(Q::Make));
    testkit::settle(&mut app);
    assert!(qr_tool::state(&app).unwrap().is_made());
    let (keys, holds) = opened_code(&mut app);
    assert_eq!(holds, QrHolds::Note);
    assert_eq!(&*keys.qr_note(Reveal::QrVault), "meet at the old oak");
    assert_eq!(records(&app), before);
}

#[test]
fn a_seed_is_saved_and_its_kept_passphrase_is_said_to_be_left_out() {
    let mut app = opened(vec![seed(ZOO).with(field::KEY_PASSPHRASE, b"kept")]);
    let before = records(&app);
    item_page(&mut app, KEYS);
    app.press(Action::Vault(V::QrSave));
    app.press(Action::Back);
    let texts = page(&mut app);
    assert!(
        texts.contains(&fill(EN.qr_tool_item_seed, &["Alpha"])),
        "{texts}"
    );
    assert!(texts.contains(EN.qr_tool_item_pass_kept), "{texts}");
    app.press(Action::QrTool(Q::Step(1)));
    let _ = page(&mut app);
    type_pass(&mut app);
    app.press(Action::QrTool(Q::Make));
    testkit::settle(&mut app);
    let (keys, holds) = opened_code(&mut app);
    assert_eq!(holds, QrHolds::Seed { words: 24 });
    assert_eq!(&*keys.qr_words(Reveal::QrVault), ZOO);
    assert_eq!(records(&app), before);
}

#[test]
fn a_codex32_share_and_a_passphrase_are_saved_and_open_back() {
    let mut app = opened(vec![share()]);
    item_page(&mut app, KEYS);
    save(&mut app);
    let (keys, holds) = opened_code(&mut app);
    assert!(matches!(holds, QrHolds::Share { .. }));
    assert_eq!(&*keys.qr_share(Reveal::QrVault), SHARE);

    let mut app = opened(vec![passphrase_item("TREZOR")]);
    item_page(&mut app, KEYS);
    save(&mut app);
    let (keys, holds) = opened_code(&mut app);
    assert!(matches!(holds, QrHolds::Passphrase { .. }));
    assert_eq!(&*keys.qr_passphrase(Reveal::QrVault), "TREZOR");
}

#[test]
fn a_person_entry_and_a_wallet_have_no_button() {
    let mut app = opened(vec![
        Record::new(kind::ENTRY)
            .with(field::TITLE, b"Mail")
            .with(field::PASSWORD, b"secret"),
    ]);
    let texts = item_page(&mut app, ENTRIES);
    assert!(texts.contains(EN.vault_copy_to), "{texts}");
    assert!(!texts.contains(EN.vault_qr_save), "{texts}");

    let mut app = opened(vec![
        Record::new(kind::WALLET).with(field::WALLET, WALLET.as_bytes()),
    ]);
    let texts = item_page(&mut app, WALLETS);
    assert!(texts.contains(EN.vault_copy_to), "{texts}");
    assert!(!texts.contains(EN.vault_qr_save), "{texts}");
}

/// The dialog an item that does not fit gives, and nothing opened.
fn dialog(app: &mut Faraday, category: usize) -> String {
    item_page(app, category);
    app.press(Action::Vault(V::QrSave));
    assert_eq!(app.screen, Screen::VaultContents, "the tool did not open");
    assert_eq!(app.sheet, Some(Sheet::QrNoFit));
    page(app)
}

#[test]
fn a_note_too_long_says_by_how_many_characters() {
    let mut app = opened(vec![note(&"a".repeat(218))]);
    let texts = dialog(&mut app, NOTES);
    assert!(
        texts.contains(&fill(EN.vault_qr_note_no, &["2", "2"])),
        "{texts}"
    );
    app.press(Action::Vault(V::QrNoClose));
    assert!(app.sheet.is_none());
    assert!(qr_tool::state(&app).is_none());
}

#[test]
fn an_18_word_seed_a_seed_of_another_list_and_a_long_passphrase_say_why() {
    let eighteen = Mnemonic::from_entropy(Language::English, &[3u8; 24]).unwrap();
    let spanish = Mnemonic::from_entropy(Language::Spanish, &[3u8; 16]).unwrap();
    for m in [eighteen, spanish] {
        let mut app = opened(vec![
            Record::new(kind::KEY).with(field::KEY, &words_payload(&m)),
        ]);
        let texts = dialog(&mut app, KEYS);
        assert!(texts.contains(EN.vault_qr_seed_no), "{texts}");
    }
    let mut app = opened(vec![passphrase_item(&"p".repeat(194))]);
    let texts = dialog(&mut app, KEYS);
    assert!(texts.contains(EN.vault_qr_pass_no), "{texts}");
}

#[test]
fn cancel_or_a_lock_while_the_item_is_sealed_makes_no_code() {
    let mut app = opened(vec![note("keep me")]);
    let before = records(&app);
    item_page(&mut app, NOTES);
    app.press(Action::Vault(V::QrSave));
    type_pass(&mut app);
    testkit::hold_worker();
    testkit::press_on_worker(&mut app, Action::QrTool(Q::Make));
    app.press(Action::Cancel);
    testkit::release_worker();
    testkit::settle_stale(&mut app);
    assert!(!qr_tool::state(&app).unwrap().is_made());
    assert!(app.outbox.is_empty());
    assert_eq!(records(&app), before, "the item is still in the vault");

    type_pass(&mut app);
    testkit::hold_worker();
    testkit::press_on_worker(&mut app, Action::QrTool(Q::Make));
    app.press(Action::Lock);
    testkit::release_worker();
    testkit::settle_stale(&mut app);
    assert!(qr_tool::state(&app).is_none());
    assert!(app.outbox.is_empty());
}

#[test]
fn back_from_the_tool_returns_to_the_items_page() {
    let mut app = opened(vec![note("page")]);
    item_page(&mut app, NOTES);
    app.press(Action::Vault(V::QrSave));
    app.press(Action::Back);
    app.press(Action::Back);
    assert_eq!(app.screen, Screen::VaultContents);
    assert!(qr_tool::state(&app).is_none());
    let texts = page(&mut app);
    assert!(texts.contains(EN.vault_qr_save), "{texts}");
}

#[test]
fn the_second_of_two_notes_seals_that_note_and_not_the_first() {
    let mut app = opened(vec![note("a"), seed(ZOO), note("b")]);
    app.press(Action::Nav(Screen::VaultContents));
    app.press(Action::Vault(V::Category(NOTES)));
    app.press(Action::Vault(V::Item(1)));
    let _ = page(&mut app);
    save(&mut app);
    let (keys, holds) = opened_code(&mut app);
    assert_eq!(holds, QrHolds::Note);
    assert_eq!(&*keys.qr_note(Reveal::QrVault), "b");
}

#[test]
fn a_vault_changed_before_or_while_the_seal_runs_makes_no_code() {
    // Changed before Make.
    let mut app = opened(vec![note("keep me")]);
    item_page(&mut app, NOTES);
    app.press(Action::Vault(V::QrSave));
    type_pass(&mut app);
    let id = app.vaults().open[0].id;
    testkit::vault_edit(&mut app, id, |s| s.insert(id, note("added")));
    app.press(Action::QrTool(Q::Make));
    app.event(Event::Key(faraday_shell_api::Key::Enter));
    let texts = page(&mut app);
    assert!(texts.contains(gone_start()), "{texts}");
    assert!(!qr_tool::state(&app).unwrap().is_made());
    assert!(app.outbox.is_empty());

    // Changed while the seal runs.
    let mut app = opened(vec![note("keep me")]);
    item_page(&mut app, NOTES);
    app.press(Action::Vault(V::QrSave));
    type_pass(&mut app);
    let id = app.vaults().open[0].id;
    testkit::hold_worker();
    testkit::press_on_worker(&mut app, Action::QrTool(Q::Make));
    testkit::vault_edit(&mut app, id, |s| s.insert(id, note("added")));
    testkit::release_worker();
    testkit::settle(&mut app);
    app.event(Event::Key(faraday_shell_api::Key::Enter));
    let texts = page(&mut app);
    assert!(texts.contains(gone_start()), "{texts}");
    assert!(!qr_tool::state(&app).unwrap().is_made());
    assert!(app.outbox.is_empty());
}

#[test]
fn an_empty_note_seals_nothing() {
    let mut app = opened(vec![note("")]);
    item_page(&mut app, NOTES);
    app.press(Action::Vault(V::QrSave));
    assert_eq!(app.screen, Screen::VaultContents, "the tool did not open");
    assert!(app.sheet.is_none());
    assert!(qr_tool::state(&app).is_none());
    assert!(app.outbox.is_empty());
}

/// The refusal's first words; the line wraps on the screen.
fn gone_start() -> &'static str {
    EN.qr_tool_item_gone.split_once(". ").unwrap().0
}
