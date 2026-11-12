//! A vault is sealed only once the system has given the session its
//! randomness: the seal's nonce is drawn from it. Before that, a lock with
//! a changed vault open seals nothing, writes nothing, keeps the vault open
//! with its changes and says why; the same lock seals once the randomness
//! arrives. Two sessions given different randomness seal the same vault
//! into different bytes.

use crate::common::type_text;
use faraday_core::strings::EN;
use faraday_core::vault::VaultAction as V;
use faraday_core::wallet::FileKind;
use faraday_core::{Action, Faraday, Screen, Sheet, StorageEvent, testkit};
use faraday_shell_api::{App, EntropyBytes, Event};
use faraday_vault::records::kind;

/// A vault the first test passphrase opens, at the smallest cost: nothing
/// here is about the cost, and the nonces do not depend on it.
fn cheap_vault() -> Vec<u8> {
    crate::common::vault("Test", testkit::VAULT_PASSPHRASE, 7)
}

/// A device whose shell has not answered a request for randomness, the
/// test vault in its Inbox.
pub(crate) fn device(vault: Vec<u8>) -> Faraday {
    let mut app = testkit::started();
    app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    faraday_core::testkit::vault_speed(&mut app, Some(100));
    app.storage(StorageEvent::Restored {
        inbox: vec![("vault.kdbx".into(), vault)],
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app
}

fn randomness(app: &mut Faraday, byte: u8) {
    app.event(Event::Entropy(EntropyBytes::new([byte; 32])));
}

/// Opens the vault with its first passphrase: the keys it holds.
fn open(app: &mut Faraday) -> usize {
    app.press(Action::Vault(V::Open(0)));
    type_text(app, testkit::VAULT_PASSPHRASE);
    app.press(Action::Vault(V::Unlock));
    for t in 1..60u64 {
        let _ = app.frame();
        app.event(Event::Tick { now_ms: t * 1000 });
        if !app.vaults().open.is_empty() {
            break;
        }
    }
    assert_eq!(app.vaults().open.len(), 1, "the vault did not open");
    app.vaults()
        .records(app.vaults().open[0].id)
        .of(kind::KEY)
        .count()
}

/// Opens the vault and saves test key 2 into it: the keys it held.
pub(crate) fn open_and_change(app: &mut Faraday) -> usize {
    let held = open(app);
    app.press(Action::Entry(None));
    type_text(app, &testkit::test_words(testkit::TEST_SEEDS[1].0));
    app.press(Action::EntryAdd);
    let k = app.session.keys.len() - 1;
    app.press(Action::Vault(V::SaveKey(k)));
    assert!(
        app.vaults().open[0].changes > 0,
        "the vault was not changed"
    );
    held
}

pub(crate) fn sealed(app: &Faraday) -> Option<Vec<u8>> {
    app.outbox
        .iter()
        .find(|i| i.kind == FileKind::Vault)
        .map(|i| i.bytes.to_vec())
}

/// A lock, the seals run on the worker and taken.
fn lock(app: &mut Faraday) {
    app.press(Action::Lock);
    testkit::settle(app);
}

#[test]
fn a_lock_pressed_while_one_vault_is_sealed_locks_when_the_seal_ends() {
    let mut app = device(cheap_vault());
    open_and_change(&mut app);
    randomness(&mut app, 0x21);
    crate::common::top_up(&mut app);
    testkit::hold_worker();
    assert!(
        !testkit::vault_lock_one(&mut app, 0),
        "the seal is under way"
    );
    testkit::press_on_worker(&mut app, Action::Lock);
    testkit::release_worker();
    testkit::settle(&mut app);
    assert!(sealed(&app).is_some(), "the vault was not sealed");
    assert_eq!(app.vaults().open.len(), 0);
    assert!(app.restart_requested(), "the device did not lock");
}

#[test]
fn a_lock_pressed_while_one_vault_is_sealed_closes_the_code_on_show_at_once() {
    let mut app = device(cheap_vault());
    open_and_change(&mut app);
    let c = testkit::compute();
    testkit::move_to(&mut app.session, testkit::net()).expect("the test network");
    let w = app
        .session
        .add_wallet(
            &c,
            "Savings",
            &testkit::savings(),
            "test",
            faraday_core::wallet::Origin::Named,
        )
        .expect("a wallet");
    app.press(Action::ShowCode(faraday_core::Code::Descriptor(w)));
    assert!(app.qr().is_some(), "no code on show");
    randomness(&mut app, 0x21);
    crate::common::top_up(&mut app);
    testkit::hold_worker();
    assert!(
        !testkit::vault_lock_one(&mut app, 0),
        "the seal is under way"
    );
    testkit::press_on_worker(&mut app, Action::Lock);
    assert!(app.qr().is_none(), "the code stays on show during the seal");
    assert!(!app.restart_requested(), "the lock did not wait");
    testkit::release_worker();
    testkit::settle(&mut app);
    assert!(sealed(&app).is_some(), "the vault was not sealed");
    assert!(app.restart_requested(), "the device did not lock");
}

/// A seal of the open vault under way, held on the worker, and Lock
/// pressed meanwhile: the lock waits for it.
fn lock_during_seal(app: &mut Faraday) {
    randomness(app, 0x21);
    crate::common::top_up(app);
    testkit::hold_worker();
    assert!(!testkit::vault_lock_one(app, 0), "the seal is under way");
    testkit::press_on_worker(app, Action::Lock);
    assert!(!app.restart_requested(), "the lock did not wait");
}

/// The seal ends: the vault is sealed and the device locks.
fn seal_ends_and_locks(app: &mut Faraday) {
    testkit::release_worker();
    testkit::settle(app);
    assert!(sealed(app).is_some(), "the vault was not sealed");
    assert!(app.restart_requested(), "the device did not lock");
}

/// Whether the frame drawn now has a text holding `part`.
fn text_on_screen(app: &mut Faraday, part: &str) -> bool {
    app.settle();
    let _ = app.frame();
    app.drawn_texts().iter().any(|t| t.contains(part))
}

/// The codes a camera reads off the frame drawn now.
fn codes_on_screen(app: &mut Faraday) -> usize {
    let c = testkit::compute();
    app.settle();
    let f = app.frame();
    let luma: Vec<u8> = f
        .rgba
        .chunks_exact(4)
        .map(|p| ((u32::from(p[0]) * 3 + u32::from(p[1]) * 6 + u32::from(p[2])) / 10) as u8)
        .collect();
    faraday_codec::decode_luma(&c, usize::from(f.width), usize::from(f.height), &luma).len()
}

/// The vault open and changed, on its first key's page.
fn on_key_page() -> Faraday {
    let mut app = device(cheap_vault());
    open_and_change(&mut app);
    app.press(Action::Nav(Screen::VaultContents));
    app.press(Action::Vault(V::Category(0)));
    app.press(Action::Vault(V::Item(0)));
    let _ = app.frame();
    app
}

#[test]
fn a_lock_pressed_while_one_vault_is_sealed_hides_an_entry_s_revealed_fields_at_once() {
    let mut app = device(cheap_vault());
    open_and_change(&mut app);
    app.press(Action::Nav(Screen::VaultContents));
    app.press(Action::Vault(V::Category(2)));
    app.press(Action::Vault(V::Add));
    app.press(Action::Vault(V::FocusField(0)));
    type_text(&mut app, "Locker");
    app.press(Action::Vault(V::FocusField(4)));
    type_text(&mut app, "combination under the mat");
    app.press(Action::Vault(V::FormSave));
    app.press(Action::Vault(V::Reveal(0)));
    assert!(text_on_screen(&mut app, "under the mat"), "not revealed");
    lock_during_seal(&mut app);
    assert!(
        !text_on_screen(&mut app, "under the mat"),
        "the notes stay on show during the seal"
    );
    seal_ends_and_locks(&mut app);
}

#[test]
fn a_lock_pressed_while_one_vault_is_sealed_hides_a_key_s_seedqr_at_once() {
    let mut app = on_key_page();
    assert_eq!(codes_on_screen(&mut app), 0, "a code before the press");
    app.press(Action::Vault(V::ShowSeedQr));
    assert_eq!(codes_on_screen(&mut app), 1, "no SeedQR shown");
    lock_during_seal(&mut app);
    assert_eq!(
        codes_on_screen(&mut app),
        0,
        "the SeedQR stays on show during the seal"
    );
    seal_ends_and_locks(&mut app);
}

#[test]
fn a_lock_pressed_while_one_vault_is_sealed_hides_a_key_s_words_at_once() {
    let mut app = on_key_page();
    let words = testkit::test_words(testkit::TEST_SEEDS[1].0);
    let two = words.split(' ').take(2).collect::<Vec<_>>().join(" ");
    app.press(Action::Vault(V::Reveal(0)));
    assert!(text_on_screen(&mut app, &two), "the words are not shown");
    lock_during_seal(&mut app);
    assert!(
        !text_on_screen(&mut app, &two),
        "the words stay on show during the seal"
    );
    seal_ends_and_locks(&mut app);
}

#[test]
fn a_lock_pressed_while_one_vault_is_sealed_hides_a_codex32_share_s_qr_at_once() {
    use faraday_core::codex32::{C32, Card, SPLITS, Split, WAYS, Way};
    use faraday_core::keygen::Start;
    use faraday_ui::keyboard::OskPress;
    let mut app = device(cheap_vault());
    open_and_change(&mut app);
    app.press(Action::Nav(Screen::WalletsStart));
    app.press(Action::KeyGen(None));
    app.press(Action::KStart(Start::MakeCodex32.index()));
    let c32 = |app: &mut Faraday, p: C32| app.press(Action::Codex32(p));
    let split = SPLITS.iter().position(|s| *s == Split::TwoOfThree).unwrap();
    c32(&mut app, C32::Split(split as u8));
    c32(&mut app, C32::Next);
    for ch in "name".chars() {
        c32(&mut app, C32::Key(OskPress::Char(ch)));
    }
    c32(&mut app, C32::Next);
    let way = WAYS.iter().position(|w| *w == Way::Typed).unwrap();
    c32(&mut app, C32::Way(way as u8));
    c32(&mut app, C32::Next);
    // Vector 2 of BIP 93: the strings A and C, typed from their 10th
    // character, make the set.
    for s in [
        "ms12namea320zyxwvutsrqpnmlkjhgfedcaxrpp870hkkqrm",
        "ms12namecacdefghjklmnpqrstuvwxyz023ftr2gdzmpy6pn",
    ] {
        for ch in s[9..35].chars() {
            c32(&mut app, C32::Key(OskPress::Char(ch)));
            let _ = app.frame();
        }
    }
    c32(&mut app, C32::Next);
    c32(&mut app, C32::Next);
    for s in [
        "ms12namea320zyxwvutsrqpnmlkjhgfedcaxrpp870hkkqrm",
        "ms12namecacdefghjklmnpqrstuvwxyz023ftr2gdzmpy6pn",
        "ms12namedll4f8jlh4e5vdvuldlfxu2jhdnlsm97xvenrxeg",
    ] {
        for ch in s[3..].chars() {
            c32(&mut app, C32::Key(OskPress::Char(ch)));
        }
        let _ = app.frame();
        c32(&mut app, C32::Next);
    }
    let m = app.keygen().unwrap().codex32.as_ref().unwrap();
    assert_eq!(m.open(), Some(Card::Made));
    let _ = app.frame();
    c32(&mut app, C32::Qr);
    let _ = app.frame();
    let shown = |app: &Faraday| {
        app.keygen()
            .unwrap()
            .codex32
            .as_ref()
            .unwrap()
            .qr()
            .is_some()
    };
    assert!(shown(&app), "no Codex32QR shown");
    assert!(
        !app.keygen().unwrap().codex32.as_ref().unwrap().masked(),
        "the strings are masked before the press"
    );
    lock_during_seal(&mut app);
    let _ = app.frame();
    assert!(!shown(&app), "the QR stays on show during the seal");
    assert!(
        app.keygen().unwrap().codex32.as_ref().unwrap().masked(),
        "the strings stay on show during the seal"
    );
    seal_ends_and_locks(&mut app);
}

#[test]
fn a_lock_pressed_while_one_vault_is_sealed_takes_an_entry_s_code_off_the_page_at_once() {
    use faraday_vault::Record;
    use faraday_vault::records::field;
    const URI: &str = "otpauth://totp/Example:you?secret=JBSWY3DPEHPK3PXP";
    let vault = crate::common::vault_with(
        "Test",
        testkit::VAULT_PASSPHRASE,
        7,
        vec![
            Record::new(kind::ENTRY)
                .with(field::TITLE, b"Mail")
                .with(field::TOTP, URI.as_bytes()),
        ],
    );
    let mut app = device(vault);
    app.online = true;
    app.storage(StorageEvent::Clock {
        unix_secs: 2_000_000_000,
    });
    open_and_change(&mut app);
    app.press(Action::Nav(Screen::VaultContents));
    app.press(Action::Vault(V::Category(2)));
    app.press(Action::Vault(V::Item(0)));
    let code = faraday_vault::totp::grouped(
        &faraday_vault::totp::parse(URI)
            .unwrap()
            .code(app.clock_now().expect("a clock")),
    );
    assert!(text_on_screen(&mut app, &code), "no code on the page");
    lock_during_seal(&mut app);
    assert!(
        !text_on_screen(&mut app, &code),
        "the code stays on the page during the seal"
    );
    seal_ends_and_locks(&mut app);
}

#[test]
fn a_lock_pressed_while_one_vault_is_sealed_hides_a_child_seed_at_once() {
    let mut app = device(cheap_vault());
    open_and_change(&mut app);
    app.press(Action::Bip85);
    app.press(Action::PApp(0));
    app.press(Action::PNext);
    app.press(Action::PNext);
    app.press(Action::PShow);
    assert!(app.bip85().is_some_and(|b| b.shown), "the child is hidden");
    lock_during_seal(&mut app);
    let _ = app.frame();
    assert!(
        app.bip85().is_some_and(|b| !b.shown),
        "the child stays on show during the seal"
    );
    seal_ends_and_locks(&mut app);
}

#[test]
fn a_lock_pressed_while_one_vault_is_sealed_hides_the_backup_checklist_s_words_at_once() {
    use faraday_core::bstep;
    use faraday_core::seeds::SeedsAction as S;
    let mut app = device(cheap_vault());
    open_and_change(&mut app);
    let fp = app.session.keys.last().expect("a key").fingerprint.0;
    app.press(Action::KeyWallet(fp, 1));
    app.press(Action::Seeds(S::Make));
    assert_eq!(app.session.wallets.len(), 1, "no wallet was made");
    app.press(Action::Backup(0));
    let w = app.backup().unwrap().wallet;
    let shape = app.plan_shape_now(w);
    let base = if shape.multi() { shape.seeds.len() } else { 2 };
    let preset = faraday_core::plan::Preset::ALL[2];
    testkit::set_backup_answers(
        &mut app,
        faraday_core::plan::Answers::preset_at(&shape, preset, base),
    );
    app.press(Action::BChecklist);
    if app.backup().and_then(|b| b.open) != Some(bstep::COPY) {
        app.press(Action::BStep(bstep::COPY));
    }
    app.press(Action::BReveal);
    assert!(app.backup().unwrap().reveal, "the words are not revealed");
    lock_during_seal(&mut app);
    let _ = app.frame();
    assert!(
        !app.backup().unwrap().reveal,
        "the words stay revealed during the seal"
    );
    seal_ends_and_locks(&mut app);
}

#[test]
fn a_lock_pressed_with_no_seal_running_closes_the_secrets_on_show_and_locks() {
    let mut app = on_key_page();
    let words = testkit::test_words(testkit::TEST_SEEDS[1].0);
    let two = words.split(' ').take(2).collect::<Vec<_>>().join(" ");
    app.press(Action::Vault(V::Reveal(0)));
    app.press(Action::Vault(V::ShowSeedQr));
    assert!(text_on_screen(&mut app, &two), "the words are not shown");
    randomness(&mut app, 0x21);
    crate::common::top_up(&mut app);
    lock(&mut app);
    assert!(sealed(&app).is_some(), "the vault was not sealed");
    assert!(app.restart_requested(), "the device did not lock");
    assert!(!text_on_screen(&mut app, &two), "the words are on show");
    assert_eq!(codes_on_screen(&mut app), 0, "the SeedQR is on show");
}

#[test]
fn a_lock_pressed_while_one_vault_fails_to_seal_still_locks() {
    let mut app = device(cheap_vault());
    open_and_change(&mut app);
    randomness(&mut app, 0x21);
    crate::common::top_up(&mut app);
    let have = app.vaults().open[0].facts().header.cost.memory_kib;
    let pick = if have == faraday_vault::file::STEPS_KIB[1] {
        2
    } else {
        1
    };
    app.press(Action::Vault(V::MemoryAsk));
    let _ = app.frame();
    app.press(Action::Vault(V::MemoryStep(pick)));
    testkit::fail_next_step_seal();
    testkit::hold_worker();
    assert!(
        !testkit::vault_lock_one(&mut app, 0),
        "the seal is under way"
    );
    testkit::press_on_worker(&mut app, Action::Lock);
    testkit::release_worker();
    testkit::settle(&mut app);
    assert!(
        sealed(&app).is_some(),
        "the Lock did not seal the vault: {:?}",
        app.toast_text()
    );
    assert!(app.restart_requested(), "the device did not lock");
}

#[test]
fn with_no_randomness_a_lock_seals_nothing_until_it_arrives() {
    let mut app = device(cheap_vault());
    let held = open_and_change(&mut app);
    app.event(Event::Tick { now_ms: 1_000 });
    app.event(Event::Tick { now_ms: 120_000 });

    app.press(Action::Lock);
    // Nothing sealed, nothing written, nothing wiped: the vault is open
    // with its change, and the sheet says why it cannot be sealed.
    assert_eq!(sealed(&app), None);
    assert!(!app.restart_requested());
    assert_eq!(app.vaults().open.len(), 1);
    assert!(app.vaults().open[0].changes > 0);
    assert_eq!(app.sheet, Some(Sheet::LockAsk));
    let texts = app.drawn_texts();
    assert!(texts.iter().any(|t| t == EN.no_randomness), "{texts:?}");
    // The vault's changes are listed as wiped from memory.
    let wiped = texts.iter().position(|t| t == "Wiped from memory");
    let lost = texts
        .iter()
        .position(|t| t.ends_with(EN.vaults_changes_not_sealed));
    assert!(wiped.is_some() && lost > wiped, "{texts:?}");
    assert!(
        texts.iter().any(|t| t == EN.vault_lock_without_sealing),
        "{texts:?}"
    );
    // Home says nothing of it.
    app.press(Action::Cancel);
    app.press(Action::Nav(Screen::Home));
    assert!(!app.drawn_texts().iter().any(|t| t == EN.no_randomness));

    // The randomness arrives: the same lock seals, and the file opens with
    // the passphrase, holding the key saved into it.
    randomness(&mut app, 0x21);
    // A seal waits for five answers: the seed and four more are short.
    for b in 0..4 {
        randomness(&mut app, 0x30 + b);
    }
    lock(&mut app);
    assert_eq!(sealed(&app), None);
    assert!(!app.restart_requested());
    assert_eq!(app.vaults().open.len(), 1);
    crate::common::top_up(&mut app);
    lock(&mut app);
    assert!(app.restart_requested());
    let file = sealed(&app).expect("the vault was not sealed");
    let mut next = device(file);
    assert_eq!(open(&mut next), held + 1);
}

#[test]
fn a_new_keyboard_and_a_lock_wait_for_the_same_randomness() {
    // Before the system's randomness: no code for a new keyboard, and no
    // seal.
    let mut app = device(cheap_vault());
    app.storage(StorageEvent::NewInput {
        id: 4,
        name: "USB Keyboard".into(),
        keyboard: true,
        pointer: false,
        touch: false,
    });
    assert_eq!(app.new_inputs()[0].code, None);
    app.press(Action::InputIgnore(4));
    // After it: both.
    randomness(&mut app, 0x22);
    app.storage(StorageEvent::NewInput {
        id: 5,
        name: "USB Keyboard".into(),
        keyboard: true,
        pointer: false,
        touch: false,
    });
    assert!(app.new_inputs()[0].code.is_some());
    app.press(Action::InputIgnore(5));
    open_and_change(&mut app);
    crate::common::top_up(&mut app);
    lock(&mut app);
    assert!(sealed(&app).is_some());
}

#[test]
fn locking_without_sealing_writes_nothing() {
    let mut app = device(cheap_vault());
    open_and_change(&mut app);
    app.press(Action::Lock);
    assert_eq!(app.sheet, Some(Sheet::LockAsk));
    app.press(Action::LockUnsealed);
    assert!(app.restart_requested());
    assert!(app.vaults().open.is_empty());
    assert_eq!(sealed(&app), None);
}

/// A seal draws its randomness from the answers the system gave, so a
/// session brought back with the same answers (a virtual machine restored
/// from a snapshot) seals the same bytes, and one given other answers
/// seals other bytes. With none waiting, a lock seals nothing.
#[test]
fn a_seal_takes_the_answers_the_system_gave() {
    let original = cheap_vault();
    let lock_with = |first: u8, answers: u8| {
        let mut app = device(original.clone());
        randomness(&mut app, first);
        for i in 0..10 {
            randomness(&mut app, answers.wrapping_add(i));
        }
        open_and_change(&mut app);
        lock(&mut app);
        sealed(&app).expect("the vault was not sealed")
    };
    let one = lock_with(0x31, 0x41);
    assert_eq!(
        one,
        lock_with(0x31, 0x41),
        "the same answers seal the same file"
    );
    assert_ne!(
        one,
        lock_with(0x31, 0x51),
        "other answers seal another file"
    );
    assert_ne!(one, original);
}

#[test]
fn two_sessions_with_no_randomness_write_nothing() {
    for _ in 0..2 {
        let mut app = device(cheap_vault());
        open_and_change(&mut app);
        lock(&mut app);
        assert_eq!(sealed(&app), None);
        assert!(!app.restart_requested());
    }
}

/// With a vault open and a key in no vault, Lock asks first with the
/// sheet every lock and power-off has (decision 28): Kept, Wiped from
/// memory with the key, Then, and Back to the vault beside Lock.
#[test]
fn a_lock_with_a_vault_open_lists_kept_wiped_and_then() {
    let mut app = device(cheap_vault());
    randomness(&mut app, 0x21);
    open(&mut app);
    app.press(Action::Entry(None));
    type_text(&mut app, &testkit::test_words(testkit::TEST_SEEDS[1].0));
    app.press(Action::EntryAdd);
    app.press(Action::LockAsk);
    assert_eq!(app.sheet, Some(Sheet::LockAsk));
    let texts = app.drawn_texts();
    let at = |s: &str| texts.iter().position(|t| t == s);
    let (kept, wiped, then) = (at("Kept"), at("Wiped from memory"), at("Then"));
    assert!(kept.is_some() && kept < wiped && wiped < then, "{texts:?}");
    assert!(then < at("Faraday restarts on Unlock"), "{texts:?}");
    let nowhere = texts.iter().position(|t| t.contains("nowhere else"));
    assert!(wiped < nowhere && nowhere < then, "the key: {texts:?}");
    assert!(app.offers(Action::Cancel));
    assert!(texts.iter().any(|t| t == "Back to the vault"), "{texts:?}");
    assert!(app.offers(Action::Lock));
    app.press(Action::Lock);
    assert!(app.restart_requested());
}

/// With no vault open, a wallet being made, the sheet's way out is
/// Cancel: there is no vault to go back to.
#[test]
fn a_lock_with_no_vault_open_is_cancelled_not_back_to_a_vault() {
    let mut app = testkit::started();
    app.press(Action::Entry(None));
    type_text(&mut app, &testkit::test_words(testkit::TEST_SEEDS[1].0));
    app.press(Action::EntryAdd);
    app.press(Action::CreateWallet);
    crate::common::ckind(&mut app, 0);
    app.press(Action::LockAsk);
    assert_eq!(app.sheet, Some(Sheet::LockAsk));
    let texts = app.drawn_texts();
    assert!(texts.iter().any(|t| t == "Cancel"), "{texts:?}");
    assert!(!texts.iter().any(|t| t == "Back to the vault"), "{texts:?}");
    app.press(Action::Cancel);
    assert_eq!(app.sheet, None);
    assert!(app.holds_secret(), "nothing wiped");
}

/// On that sheet, a key in no vault has its Save: pressed, the key is in
/// the open vault, the sheet stays and lists it there, with no Save.
#[test]
fn the_lock_sheet_saves_a_key_in_no_vault_into_the_open_vault() {
    let mut app = device(cheap_vault());
    randomness(&mut app, 0x21);
    open(&mut app);
    app.press(Action::Entry(None));
    type_text(&mut app, &testkit::test_words(testkit::TEST_SEEDS[1].0));
    app.press(Action::EntryAdd);
    app.press(Action::LockAsk);
    let k = app.session.keys.all().count() - 1;
    let save = Action::Vault(V::SaveKey(k));
    let _ = app.drawn_texts();
    assert!(app.offers(save), "Save beside the key");
    app.press(save);
    assert_eq!(app.sheet, Some(Sheet::LockAsk), "the sheet stays");
    let texts = app.drawn_texts();
    assert!(
        !texts.iter().any(|t| t.contains("nowhere else")),
        "{texts:?}"
    );
    assert!(!app.offers(save), "{texts:?}");
}
