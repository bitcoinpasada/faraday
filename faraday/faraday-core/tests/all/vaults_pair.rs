//! Vaults on the wide layout: this session's keys at the left, the vault
//! files and the row Make a new vault in the tile at the right.

use crate::common::type_text;
use faraday_core::strings::EN;
use faraday_core::vault::VaultAction as V;
use faraday_core::{Action, Faraday, Screen, StorageEvent, testkit};
use faraday_shell_api::{App, BootState, DisplayInfo, EntropyBytes, Event, SecureHardware};
use faraday_vault::records::kind;

const WORDS: &str = "abandon abandon abandon abandon abandon abandon abandon abandon abandon \
                      abandon abandon about";

fn shown(width: u16, height: u16) -> Faraday {
    let mut app = faraday_core::testkit::new_on_test_net();
    app.event(Event::Display(DisplayInfo {
        width,
        height,
        dpi: 160,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
    app
}

/// A 1366 by 768 window holding one locked vault file.
fn with_a_vault() -> Faraday {
    let mut app = shown(1366, 768);
    let file = testkit::kdbx_vault("Main", "pair phrase", 1, Vec::new()).unwrap();
    app.storage(StorageEvent::Restored {
        inbox: faraday_core::files_from(vec![("main.kdbx".to_string(), file)]),
        outbox: faraday_core::files_from(Vec::<(String, Vec<u8>)>::new()),
        kept: faraday_core::kept_from(Vec::<(String, Vec<u8>)>::new()),
    });
    app
}

#[test]
fn the_pane_lists_the_sessions_keys_and_the_tile_ends_in_make_a_new_vault() {
    let c = faraday_core::testkit::compute();
    let mut app = with_a_vault();
    app.session.keys.add_words(&c, WORDS, "Seed", None).unwrap();
    app.press(Action::Nav(Screen::Vaults));
    let _ = app.frame();
    let texts = app.drawn_texts();
    assert!(texts.iter().any(|t| t == EN.session_group_keys), "Keys");
    assert!(
        texts.iter().any(|t| t == EN.vaults_make_a_vault),
        "Make row"
    );
    assert!(
        texts.iter().any(|t| t == EN.vault_unlock),
        "the file's Unlock"
    );
}

#[test]
fn make_a_new_vault_in_the_tile_opens_the_wizard_and_the_bar_has_no_button() {
    let mut app = with_a_vault();
    app.press(Action::Nav(Screen::Vaults));
    let _ = app.frame();
    assert!(app.offers(Action::Vault(V::Create)));
    // One press target for it: the row, not the row and a bar button.
    let n = app
        .drawn_texts()
        .iter()
        .filter(|t| *t == EN.vaults_make_a_vault)
        .count();
    assert_eq!(n, 1, "the row only");
    app.press(Action::Vault(V::Create));
    assert_eq!(app.screen, Screen::CreateVault);
}

const PHRASE: &str = "pair phrase";

/// A 1366 by 768 window with `files` as vault files, the first `n` of them
/// unlocked, on the Vaults page with a key (and a wallet when asked) held.
fn marked(files: Vec<Vec<u8>>, n: usize, wallet: bool) -> Faraday {
    let c = faraday_core::testkit::compute();
    let mut app = shown(1366, 768);
    app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    testkit::vault_speed(&mut app, Some(100));
    let inbox: Vec<(String, Vec<u8>)> = files
        .into_iter()
        .enumerate()
        .map(|(i, f)| (format!("v{i}.kdbx"), f))
        .collect();
    app.storage(StorageEvent::Restored {
        inbox: faraday_core::files_from(inbox),
        outbox: faraday_core::files_from(Vec::<(String, Vec<u8>)>::new()),
        kept: faraday_core::kept_from(Vec::<(String, Vec<u8>)>::new()),
    });
    app.event(Event::Entropy(EntropyBytes::new([7; 32])));
    app.session.keys.add_words(&c, WORDS, "Seed", None).unwrap();
    if wallet {
        let kit = testkit::kits().into_iter().next().unwrap();
        app.session
            .add_wallet_confirmed(
                &c,
                kit.name,
                &kit.descriptor,
                "Test wallet",
                faraday_core::wallet::Origin::Named,
            )
            .unwrap();
    }
    app.press(Action::Nav(Screen::Vaults));
    for i in 0..n {
        unlock(&mut app, i);
        assert_eq!(app.vaults().open.len(), i + 1, "vault {i} did not open");
    }
    app.press(Action::Nav(Screen::Vaults));
    let _ = app.frame();
    app
}

/// Unlocks vault file `i` with [`PHRASE`], ticking until it is open.
fn unlock(app: &mut Faraday, i: usize) {
    let had = app.vaults().open.len();
    app.press(Action::Vault(V::Open(i)));
    let _ = app.frame();
    type_text(app, PHRASE);
    app.press(Action::Vault(V::Unlock));
    for t in 1..60u64 {
        if app.vaults().open.len() > had {
            break;
        }
        let _ = app.frame();
        app.event(Event::Tick { now_ms: t * 1000 });
    }
}

fn one(id: u8) -> Vec<u8> {
    testkit::kdbx_vault("Main", PHRASE, id, Vec::new()).unwrap()
}

fn says(app: &mut Faraday, text: &str) -> bool {
    let _ = app.frame();
    app.drawn_texts().iter().any(|t| t == text)
}

fn keys_in_vault(app: &Faraday) -> usize {
    let id = app.vaults().open[0].id;
    app.vaults().records(id).of(kind::KEY).count()
}

#[test]
fn a_key_saved_from_the_pane_is_in_the_vault_and_marked() {
    let mut app = marked(vec![one(1)], 1, false);
    assert!(says(&mut app, EN.session_mark_not_saved));
    assert!(app.offers(Action::Vault(V::SaveKey(0))));
    app.press(Action::Vault(V::SaveKey(0)));
    testkit::settle(&mut app);
    assert_eq!(keys_in_vault(&app), 1);
    assert!(says(&mut app, EN.session_mark_in_a_vault));
    assert!(!says(&mut app, EN.session_mark_not_saved));
    assert!(!app.offers(Action::Vault(V::SaveKey(0))));
}

#[test]
fn a_wallet_saved_from_the_pane_is_in_the_vault_and_marked() {
    let mut app = marked(vec![one(1)], 1, true);
    assert!(says(&mut app, EN.session_mark_not_saved));
    assert!(app.offers(Action::Vault(V::SaveWallet(0))));
    app.press(Action::Vault(V::SaveWallet(0)));
    testkit::settle(&mut app);
    let id = app.vaults().open[0].id;
    assert_eq!(app.vaults().records(id).of(kind::WALLET).count(), 1);
    assert!(says(&mut app, EN.session_mark_in_a_vault));
    assert!(!app.offers(Action::Vault(V::SaveWallet(0))));
}

#[test]
fn two_open_vaults_draw_no_mark_and_no_save() {
    let mut app = marked(vec![one(1), one(2)], 2, true);
    assert!(!says(&mut app, EN.session_mark_not_saved));
    assert!(!says(&mut app, EN.session_mark_in_a_vault));
    assert!(!app.offers(Action::Vault(V::SaveKey(0))));
    assert!(!app.offers(Action::Vault(V::SaveWallet(0))));
}

#[test]
fn a_locked_vault_draws_no_mark_and_no_save() {
    let mut app = marked(vec![one(1)], 1, true);
    assert!(says(&mut app, EN.session_mark_not_saved));
    app.press(Action::Lock);
    testkit::settle(&mut app);
    // A lock wipes the session: a key loaded after it is the one in the pane.
    let c = faraday_core::testkit::compute();
    app.session.keys.add_words(&c, WORDS, "Seed", None).unwrap();
    app.press(Action::Nav(Screen::Vaults));
    assert!(app.vaults().open.is_empty());
    assert!(says(&mut app, EN.session_group_keys), "the pane is there");
    assert!(!says(&mut app, EN.session_mark_not_saved));
    assert!(!app.offers(Action::Vault(V::SaveKey(0))));
}

#[test]
fn a_read_only_vault_marks_but_offers_no_save() {
    let newer = testkit::kdbx_edited(
        "Newer",
        PHRASE,
        9,
        |_| {},
        |xml| {
            let at = xml.find("faraday.format").unwrap();
            let v = at + xml[at..].find("<Value>1</Value>").unwrap();
            format!(
                "{}<Value>99</Value>{}",
                &xml[..v],
                &xml[v + "<Value>1</Value>".len()..]
            )
        },
    )
    .unwrap();
    let mut app = marked(vec![newer], 1, true);
    assert!(says(&mut app, EN.session_mark_not_saved));
    assert!(!app.offers(Action::Vault(V::SaveKey(0))));
    assert!(!app.offers(Action::Vault(V::SaveWallet(0))));
}

#[test]
fn the_mark_follows_the_vault_when_its_record_goes() {
    let mut app = marked(vec![one(1)], 1, false);
    app.press(Action::Vault(V::SaveKey(0)));
    testkit::settle(&mut app);
    assert!(says(&mut app, EN.session_mark_in_a_vault));
    let id = app.vaults().open[0].id;
    let removed = testkit::vault_edit(&mut app, id, |s| {
        s.remove_where(id, |r| r.kind() == kind::KEY)
    });
    testkit::vault_changed(&mut app, id);
    assert_eq!(removed, 1);
    assert_eq!(keys_in_vault(&app), 0, "the record is out");
    assert!(says(&mut app, EN.session_mark_not_saved));
    assert!(app.offers(Action::Vault(V::SaveKey(0))));
}

fn count(app: &mut Faraday, text: &str) -> usize {
    let _ = app.frame();
    app.drawn_texts().iter().filter(|t| *t == text).count()
}

/// Key 0 (words only) and key 1 (the same words with a passphrase).
fn with_passphrase_key() -> Faraday {
    let mut app = marked(vec![one(1)], 1, false);
    let c = faraday_core::testkit::compute();
    app.session
        .keys
        .add_with_passphrase(&c, 0, "pw one", "Seed pw")
        .unwrap();
    app
}

#[test]
fn a_key_with_a_passphrase_is_not_in_a_vault_until_its_passphrase_is() {
    let mut app = with_passphrase_key();
    assert_eq!(count(&mut app, EN.session_mark_not_saved), 2);
    assert!(app.offers(Action::Vault(V::SaveKey(1))));
    // Its words alone: key 0 is in the vault, key 1 holds only words there.
    app.press(Action::Vault(V::SaveKey(0)));
    testkit::settle(&mut app);
    assert_eq!(count(&mut app, EN.session_mark_in_a_vault), 1);
    assert_eq!(count(&mut app, EN.session_mark_words_in_a_vault), 1);
    assert_eq!(count(&mut app, EN.session_mark_not_saved), 0);
    assert!(!app.offers(Action::Vault(V::SaveKey(1))), "no Save for it");
    // With the passphrase it is in a vault.
    app.press(Action::Vault(V::SaveKeyWithPassphrase(1)));
    testkit::settle(&mut app);
    assert_eq!(count(&mut app, EN.session_mark_in_a_vault), 2);
    assert_eq!(count(&mut app, EN.session_mark_words_in_a_vault), 0);
}

#[test]
fn a_key_with_a_passphrase_saved_from_the_pane_has_its_words_in_a_vault() {
    let mut app = with_passphrase_key();
    app.press(Action::Vault(V::SaveKey(1)));
    testkit::settle(&mut app);
    assert_eq!(keys_in_vault(&app), 1);
    assert_eq!(count(&mut app, EN.session_mark_words_in_a_vault), 1);
    assert_eq!(
        count(&mut app, EN.session_mark_in_a_vault),
        1,
        "key 0: same words"
    );
    assert!(!app.offers(Action::Vault(V::SaveKey(1))));
}

#[test]
fn a_passphrase_stored_with_other_words_does_not_make_the_key_held() {
    let mut app = with_passphrase_key();
    let c = faraday_core::testkit::compute();
    app.session
        .keys
        .add_with_passphrase(&c, 0, "pw two", "Seed pw2")
        .unwrap();
    app.press(Action::Vault(V::SaveKeyWithPassphrase(1)));
    testkit::settle(&mut app);
    // Key 1 stored with its own passphrase; key 2 has another: words only.
    assert_eq!(count(&mut app, EN.session_mark_words_in_a_vault), 1);
    assert_eq!(count(&mut app, EN.session_mark_in_a_vault), 2);
}

#[test]
fn a_key_loaded_from_a_master_seed_is_marked_and_saved_from_the_pane() {
    let mut app = marked(vec![one(1)], 1, false);
    let c = faraday_core::testkit::compute();
    app.session.keys.add_seed(&c, &[3u8; 32], "Master").unwrap();
    // Put in beside the app, with no event asking for a redraw: draw it.
    let _ = app.drawn_text_boxes();
    assert!(app.offers(Action::Vault(V::SaveKey(1))));
    app.press(Action::Vault(V::SaveKey(1)));
    testkit::settle(&mut app);
    assert_eq!(keys_in_vault(&app), 1);
    assert_eq!(count(&mut app, EN.session_mark_in_a_vault), 1);
    assert_eq!(count(&mut app, EN.session_mark_not_saved), 1, "key 0");
    assert!(!app.offers(Action::Vault(V::SaveKey(1))));
}

#[test]
fn a_press_on_save_duplicate_is_refused_as_ever() {
    let mut app = marked(vec![one(1)], 1, false);
    app.press(Action::Vault(V::SaveKey(0)));
    testkit::settle(&mut app);
    app.press(Action::Vault(V::SaveKey(0)));
    testkit::settle(&mut app);
    assert_eq!(keys_in_vault(&app), 1, "one record, not two");
}

/// A vault with a change, its seal held on the worker.
fn sealing() -> (Faraday, faraday_keys::vault::VaultId) {
    let mut app = marked(vec![one(1)], 1, false);
    let id = app.vaults().open[0].id;
    testkit::vault_edit(&mut app, id, |s| {
        s.insert(
            id,
            faraday_vault::Record::new(kind::ENTRY)
                .with(faraday_vault::records::field::TITLE, b"e"),
        );
    });
    testkit::vault_changed(&mut app, id);
    for n in 0..26u8 {
        app.event(Event::Entropy(EntropyBytes::new([n + 20; 32])));
    }
    crate::common::top_up(&mut app);
    testkit::hold_worker();
    testkit::press_on_worker(&mut app, Action::Lock);
    assert!(app.busy().is_some(), "the vault is sealing");
    (app, id)
}

fn seal_ends(mut app: Faraday) {
    testkit::release_worker();
    testkit::settle(&mut app);
    assert!(app.restart_requested() || app.vaults().open.is_empty());
}

/// The press is refused twice over: by the app while a job runs, and by
/// the vault, which the next test asks alone.
#[test]
fn a_press_on_save_while_the_vault_seals_is_not_taken() {
    let (mut app, id) = sealing();
    app.press(Action::Vault(V::SaveKey(0)));
    assert_eq!(
        app.vaults().records(id).of(kind::KEY).count(),
        0,
        "nothing added while it seals"
    );
    seal_ends(app);
}

#[test]
fn a_vault_takes_no_record_while_it_seals() {
    let (mut app, id) = sealing();
    let at = testkit::vault_edit(&mut app, id, |s| {
        s.insert(
            id,
            faraday_vault::Record::new(kind::ENTRY)
                .with(faraday_vault::records::field::TITLE, b"late"),
        )
    });
    assert!(at.is_none(), "refused");
    assert_eq!(app.vaults().records(id).of(kind::ENTRY).count(), 1);
    seal_ends(app);
}

#[test]
fn the_tile_has_no_make_row_while_another_flow_waits_for_an_unlock() {
    let mut app = with_a_vault();
    app.press(Action::Nav(Screen::Vaults));
    let _ = app.frame();
    assert!(app.offers(Action::Vault(V::Create)), "plain: the row");
    app.press(Action::Vault(V::ListFrom(Screen::Catalog)));
    let _ = app.frame();
    assert_eq!(app.screen, Screen::Vaults);
    assert_eq!(app.vaults().back_to, Some(Screen::Catalog));
    assert!(!app.offers(Action::Vault(V::Create)), "back_to: no row");
    assert!(
        !says(&mut app, EN.vaults_make_a_vault),
        "back_to: no row drawn"
    );
}

/// Random bytes for the seal a lock or a close makes of a changed vault.
fn feed_the_seal(app: &mut Faraday) {
    for n in 0..26u8 {
        app.event(Event::Entropy(EntropyBytes::new([n + 20; 32])));
    }
    crate::common::top_up(app);
}

/// Where the vault's key record stands among its records.
fn key_at(app: &Faraday, id: faraday_keys::vault::VaultId) -> usize {
    app.vaults()
        .records(id)
        .of(kind::KEY)
        .next()
        .expect("a key record")
        .0
}

#[test]
fn a_lock_empties_the_ids_kept_beside_a_key_and_a_vaults_record() {
    let mut app = marked(vec![one(1)], 1, false);
    let c = faraday_core::testkit::compute();
    app.session.keys.add_seed(&c, &[3u8; 32], "Master").unwrap();
    app.press(Action::Vault(V::SaveKey(0)));
    testkit::settle(&mut app);
    let id = app.vaults().open[0].id;
    let at = key_at(&app, id);
    assert!(app.session.keys.seed_record_id_kept(1).is_some());
    assert!(app.vaults().store().record_id(&c, id, at).is_some());
    feed_the_seal(&mut app);
    app.press(Action::Lock);
    testkit::settle(&mut app);
    assert!(app.session.keys.seed_record_id_kept(1).is_none());
    assert!(app.vaults().store().record_id(&c, id, at).is_none());
}

#[test]
fn a_vaults_close_empties_the_ids_kept_with_its_records() {
    let mut app = marked(vec![one(1)], 1, false);
    let c = faraday_core::testkit::compute();
    app.press(Action::Vault(V::SaveKey(0)));
    testkit::settle(&mut app);
    let id = app.vaults().open[0].id;
    let at = key_at(&app, id);
    assert!(app.vaults().store().record_id(&c, id, at).is_some());
    feed_the_seal(&mut app);
    // False while the seal is on the worker; closed when it is done.
    testkit::vault_lock_one(&mut app, 0);
    testkit::settle(&mut app);
    assert!(app.vaults().open.is_empty());
    assert!(app.vaults().store().record_id(&c, id, at).is_none());
}

/// With nothing to seal, the close alone lets the records go.
#[test]
fn an_unchanged_vaults_close_empties_the_ids_kept_with_its_records() {
    let mut app = marked(vec![one(1)], 1, false);
    let c = faraday_core::testkit::compute();
    app.press(Action::Vault(V::SaveKey(0)));
    testkit::settle(&mut app);
    feed_the_seal(&mut app);
    testkit::vault_lock_one(&mut app, 0);
    testkit::settle(&mut app);
    assert!(app.vaults().open.is_empty());
    // The file the seal wrote, opened again and left as it is.
    app.press(Action::Nav(Screen::Vaults));
    let _ = app.frame();
    unlock(&mut app, 0);
    let id = app.vaults().open[0].id;
    let at = key_at(&app, id);
    assert!(app.vaults().store().record_id(&c, id, at).is_some());
    assert!(testkit::vault_lock_one(&mut app, 0), "closed at once");
    assert!(app.vaults().open.is_empty());
    assert!(app.vaults().store().record_id(&c, id, at).is_none());
}

#[test]
fn a_waiting_psbt_has_sign_everywhere_and_opens_its_file_everywhere_but_on_vaults() {
    use faraday_core::Row;
    let mut app = with_a_vault();
    crate::common::add_key(&mut app, faraday_core::testkit::TEST_SEEDS[0].0);
    let kit = testkit::kits()
        .into_iter()
        .find(|k| k.id == "spending")
        .unwrap();
    let psbt = testkit::unsigned(&kit).unwrap().to_bytes();
    testkit::inbox_file_at(&mut app, 1, "spend.psbt", psbt);
    app.press(Action::Nav(Screen::Vaults));
    let _ = app.drawn_text_boxes();
    assert!(app.offers(Action::StartSpend(1)), "Vaults: Sign");
    assert!(
        !app.offers(Action::RowOpen(Row::File(1))),
        "Vaults: the file's page is no page of Vaults"
    );
    for page in [Screen::Wallets, Screen::Catalog, Screen::Check] {
        app.press(Action::Nav(page));
        let _ = app.drawn_text_boxes();
        assert!(app.offers(Action::StartSpend(1)), "{page:?}: Sign");
        assert!(
            app.offers(Action::RowOpen(Row::File(1))),
            "{page:?}: the row opens"
        );
    }
}
