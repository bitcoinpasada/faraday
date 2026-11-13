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
        inbox: faraday_core::files_from(vec![(faraday_text::Text::of("vault.kdbx"), vault)]),
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

/// New key's Make codex32 shares, on the card that shows the strings made.
fn codex32_made() -> Faraday {
    use faraday_core::codex32::{C32, SPLITS, Split, WAYS, Way};
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
    assert_eq!(m.open(), Some(faraday_core::codex32::Card::Made));
    let _ = app.frame();
    app
}

#[test]
fn a_lock_pressed_while_one_vault_is_sealed_hides_a_codex32_share_at_once() {
    use faraday_core::codex32::{C32, CARDS, Card};
    let mut app = codex32_made();
    // The string is drawn in groups of four on the card that types it back.
    let at = CARDS.iter().position(|c| *c == Card::Strings).unwrap();
    app.press(Action::Codex32(C32::Card(at as u8)));
    assert!(
        text_on_screen(&mut app, "ms12 name"),
        "no string is shown: {:?}",
        app.drawn_texts()
    );
    lock_during_seal(&mut app);
    assert!(
        !text_on_screen(&mut app, "ms12 name"),
        "the string stays on show during the seal"
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
    let child = app
        .bip85_value(&testkit::compute())
        .expect("a child")
        .split(' ')
        .take(2)
        .collect::<Vec<_>>()
        .join(" ");
    assert!(
        text_on_screen(&mut app, &child),
        "the child is hidden: {:?}",
        app.drawn_texts()
    );
    lock_during_seal(&mut app);
    assert!(
        !text_on_screen(&mut app, &child),
        "the child stays on show during the seal"
    );
    seal_ends_and_locks(&mut app);
}

#[test]
fn a_lock_pressed_while_one_vault_is_sealed_hides_the_convert_result_at_once() {
    use faraday_core::convert::{self, ConvertAction as C, ROWS};
    let mut app = device(cheap_vault());
    open_and_change(&mut app);
    app.press(Action::Nav(Screen::Catalog));
    let tile = faraday_core::catalog::TILES
        .iter()
        .position(|t| t.name == "Convert a seed")
        .unwrap();
    app.press(Action::Catalog(tile as u8));
    let row = ROWS.iter().position(|r| r.name == "xprv").unwrap();
    app.press(Action::Convert(C::To(row as u8)));
    let _ = app.frame();
    let now = app.now_ms() + 16;
    app.event(Event::Tick { now_ms: now });
    testkit::settle(&mut app);
    app.press(Action::Convert(C::Show));
    let _ = app.frame();
    let xprv = convert::shown(&app)
        .expect("the result is shown")
        .to_string();
    assert!(
        text_on_screen(&mut app, &xprv[..20]),
        "the result is hidden"
    );
    lock_during_seal(&mut app);
    assert!(
        !text_on_screen(&mut app, &xprv[..20]),
        "the result stays on show during the seal"
    );
    seal_ends_and_locks(&mut app);
}

#[test]
fn a_lock_pressed_while_one_vault_is_sealed_hides_a_krux_code_s_contents_at_once() {
    use faraday_codec::kef::{Form, encode};
    use faraday_core::seeds::kef::KefAct;
    use zeroize::Zeroizing;
    let mut app = device(cheap_vault());
    open_and_change(&mut app);
    app.press(Action::Nav(Screen::Home));
    app.press(Action::Scan);
    for i in 0..8u8 {
        app.event(Event::Entropy(EntropyBytes::new([0x30 + i; 32])));
    }
    app.kef_seal(
        Zeroizing::new(b"note key".to_vec()),
        Zeroizing::new(b"a note about the pantry".to_vec()),
        20,
        faraday_core::worker::kef::KefId::Random(8),
        10_000,
        "Working",
    )
    .expect("fresh bytes");
    testkit::settle(&mut app);
    let raw = app.kef_take_sealed().unwrap().unwrap();
    let code = encode(Form::Base43, &raw).unwrap();
    app.event(Event::Scanned {
        bytes: code.into_bytes(),
    });
    let _ = app.frame();
    let open = app
        .scan()
        .and_then(|s| s.card.as_ref())
        .and_then(|c| c.ways.iter().position(|w| w.label() == "Open"))
        .expect("no Open way");
    app.press(Action::ReadGo(open as u8));
    type_text(&mut app, "note key");
    app.event(Event::Key(faraday_shell_api::Key::Enter));
    for _ in 0..6 {
        testkit::settle(&mut app);
        if app.busy().is_none() {
            break;
        }
    }
    let _ = app.frame();
    app.press(Action::KefRead(KefAct::Show));
    assert!(
        text_on_screen(&mut app, "pantry"),
        "the note is hidden: {:?}",
        app.drawn_texts()
    );
    lock_during_seal(&mut app);
    assert!(
        !text_on_screen(&mut app, "pantry"),
        "the note stays on show during the seal"
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
    if app
        .backup_items_now(&testkit::compute())
        .contains(&bstep::BLANK)
    {
        app.press(Action::BNext(bstep::BLANK));
    }
    if app.backup().and_then(|b| b.open) != Some(bstep::COPY) {
        app.press(Action::BStep(bstep::COPY));
    }
    app.press(Action::BReveal);
    // The checklist draws each word of the seed in a row of its own.
    let word = testkit::TEST_SEEDS[1].0;
    let word_shown = |app: &mut Faraday| {
        app.settle();
        let _ = app.frame();
        app.drawn_texts().iter().any(|t| *t == word)
    };
    assert!(
        word_shown(&mut app) || codes_on_screen(&mut app) > 0,
        "the seed is not shown: {:?}",
        app.drawn_texts()
    );
    lock_during_seal(&mut app);
    assert!(
        !word_shown(&mut app) && codes_on_screen(&mut app) == 0,
        "the seed stays on show during the seal"
    );
    seal_ends_and_locks(&mut app);
}

#[test]
fn a_lock_pressed_with_no_seal_running_takes_the_secrets_on_show_off_the_screen_at_once() {
    let mut app = on_key_page();
    let words = testkit::test_words(testkit::TEST_SEEDS[1].0);
    let two = words.split(' ').take(2).collect::<Vec<_>>().join(" ");
    app.press(Action::Vault(V::Reveal(0)));
    app.press(Action::Vault(V::ShowSeedQr));
    assert!(text_on_screen(&mut app, &two), "the words are not shown");
    randomness(&mut app, 0x21);
    crate::common::top_up(&mut app);
    // The seal is held on the worker: the screen is looked at while the
    // session is still there.
    testkit::hold_worker();
    testkit::press_on_worker(&mut app, Action::Lock);
    assert!(!app.restart_requested(), "the lock did not wait");
    assert!(!text_on_screen(&mut app, &two), "the words are on show");
    assert_eq!(codes_on_screen(&mut app), 0, "the SeedQR is on show");
    seal_ends_and_locks(&mut app);
}

#[test]
fn a_lock_from_the_checklist_s_one_vault_seal_through_the_lock_sheet_locks_when_it_ends() {
    use crate::backup_checklist::{
        device as checklist_device, make_the_vault, press_offered, two_of_three_two_here,
    };
    use faraday_core::bstep;
    let mut app = checklist_device(Vec::new());
    let w = two_of_three_two_here(&mut app);
    app.press(Action::Backup(w));
    testkit::backup_as(&mut app, faraday_core::plan::Preset::PaperVault);
    let c = testkit::compute();
    crate::common::no_words(&mut app);
    press_offered(&mut app, Action::BChecklist);
    if app.backup_items_now(&c).contains(&bstep::BLANK) {
        app.press(Action::BNext(bstep::BLANK));
    }
    press_offered(&mut app, Action::Vault(V::CreateFrom(Screen::Backup)));
    make_the_vault(&mut app);
    press_offered(&mut app, Action::BVaultSave(0));
    press_offered(&mut app, Action::BStep(bstep::VAULT + 1));
    for _ in 0..20 {
        app.settle();
        let _ = app.frame();
        if app.offers(Action::BNewVault(1)) {
            break;
        }
        app.event(Event::Scroll {
            x: 400,
            y: 384,
            dy: 300,
        });
    }
    assert!(app.offers(Action::BNewVault(1)), "no new vault is offered");
    crate::common::top_up(&mut app);
    // The checklist's press starts the seal of the first vault; Lock from
    // the sidebar and the Lock sheet's Lock go on while it runs.
    testkit::hold_worker();
    testkit::press_on_worker(&mut app, Action::BNewVault(1));
    testkit::press_on_worker(&mut app, Action::LockAsk);
    assert_eq!(app.sheet, Some(Sheet::LockAsk));
    testkit::press_on_worker(&mut app, Action::Lock);
    assert!(!app.restart_requested(), "the lock did not wait");
    testkit::release_worker();
    testkit::settle(&mut app);
    assert!(sealed(&app).is_some(), "the vault was not sealed");
    assert!(app.restart_requested(), "the device did not lock");
}

/// A lock that stops in its seal, for want of memory: `lock_now` has taken
/// what is on show off the screen, nothing is sealed or wiped, and the Lock
/// sheet shows.
fn lock_stops(app: &mut Faraday) {
    crate::common::top_up(app);
    app.storage(StorageEvent::Memory { available_mib: 1 });
    app.press(Action::Lock);
    testkit::settle(app);
    assert_eq!(app.sheet, Some(Sheet::LockAsk), "the lock did not stop");
    assert!(!app.restart_requested());
    assert!(sealed(app).is_none(), "the vault was sealed");
}

#[test]
fn a_lock_that_stops_keeps_the_check_being_typed_on_the_checklist() {
    use faraday_core::bstep;
    use faraday_core::seeds::SeedsAction as S;
    let mut app = device(cheap_vault());
    open_and_change(&mut app);
    let fp = app.session.keys.last().expect("a key").fingerprint.0;
    app.press(Action::KeyWallet(fp, 1));
    app.press(Action::Seeds(S::Make));
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
    if app
        .backup_items_now(&testkit::compute())
        .contains(&bstep::BLANK)
    {
        app.press(Action::BNext(bstep::BLANK));
    }
    if app.backup().and_then(|b| b.open) != Some(bstep::COPY) {
        app.press(Action::BStep(bstep::COPY));
    }
    app.press(Action::BReveal);
    app.press(Action::BCheck);
    assert!(app.backup().unwrap().check.is_some(), "no check is open");
    type_text(&mut app, "aban");
    lock_stops(&mut app);
    app.press(Action::Cancel);
    assert!(
        app.backup().is_some_and(|b| b.check.is_some()),
        "the check typed was thrown away"
    );
    assert!(
        text_on_screen(&mut app, "aban"),
        "the check typed is not drawn: {:?}",
        app.drawn_texts()
    );
}

#[test]
fn a_lock_that_stops_keeps_new_key_s_selected_cell() {
    use faraday_core::codex32::{C32, Cell};
    let mut app = codex32_made();
    app.press(Action::Codex32(C32::Cell(Cell::Given { sheet: 0, b: 10 })));
    let done = EN.codex32_cell_box.replace("{}", "10");
    let done = done.as_str();
    assert!(text_on_screen(&mut app, done), "no cell is open");
    lock_stops(&mut app);
    app.press(Action::Cancel);
    // Masked or not, the eye brings the working back.
    if app.keygen().unwrap().codex32.as_ref().unwrap().masked() {
        app.press(Action::Codex32(C32::Eye));
    }
    assert!(
        text_on_screen(&mut app, done),
        "the open cell is not drawn: {:?}",
        app.drawn_texts()
    );
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
        name: faraday_text::Text::of("USB Keyboard"),
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
        name: faraday_text::Text::of("USB Keyboard"),
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

/// The texts drawn now.
fn drawn(app: &mut Faraday) -> Vec<String> {
    app.settle();
    let _ = app.frame();
    app.drawn_texts()
}

/// A screen with a secret behind its eye, the vault open and changed: the
/// texts the eye adds are on show, then a Lock pressed while one vault is
/// sealed takes them off at once.
fn eye_hides_at_lock(app: &mut Faraday, eye: Action) {
    let hidden = drawn(app);
    app.press(eye);
    let shown = drawn(app);
    let added: Vec<String> = shown
        .iter()
        .filter(|t| !hidden.contains(t))
        .cloned()
        .collect();
    assert!(!added.is_empty(), "the eye shows nothing: {shown:?}");
    lock_during_seal(app);
    let now = drawn(app);
    assert!(
        added.iter().all(|t| !now.contains(t)),
        "still on show during the seal: {:?}",
        added.iter().filter(|t| now.contains(t)).collect::<Vec<_>>()
    );
    seal_ends_and_locks(app);
}

#[test]
fn a_lock_pressed_while_one_vault_is_sealed_hides_lightning_s_private_key_at_once() {
    use faraday_shell_api::Key;
    let mut app = device(cheap_vault());
    open_and_change(&mut app);
    app.press(Action::Lightning);
    app.press(Action::LAezeed);
    let cipher = "above judge emerge veteran reform crunch system all snap please \
        shoulder vault hurt city quarter cover enlist swear success suggest drink wagon enrich body";
    for w in cipher.split_whitespace() {
        type_text(&mut app, w);
        app.event(Event::Key(Key::Char(' ')));
    }
    app.event(Event::Key(Key::Enter));
    let _ = app.frame();
    let now = app.now_ms() + 16;
    app.event(Event::Tick { now_ms: now });
    testkit::settle(&mut app);
    assert!(app.lightning().unwrap().node.is_some(), "no node key");
    eye_hides_at_lock(&mut app, Action::LShow);
}

#[test]
fn a_lock_pressed_while_one_vault_is_sealed_hides_vanity_s_passphrase_at_once() {
    use faraday_core::vanity::{VanityAction as VA, vstep};
    let mut app = device(cheap_vault());
    open_and_change(&mut app);
    app.press(Action::Vanity(VA::Open));
    let dial = faraday_core::grind::Dial::ALL
        .iter()
        .position(|d| *d == faraday_core::grind::Dial::Passphrase)
        .unwrap();
    app.press(Action::Vanity(VA::Dial(dial as u8)));
    app.press(Action::Vanity(VA::Next(vstep::DIAL)));
    app.press(Action::Vanity(VA::Next(vstep::SCRIPT)));
    app.event(Event::Key(faraday_shell_api::Key::Char('q')));
    app.press(Action::Vanity(VA::Next(vstep::PREFIX)));
    app.press(Action::Vanity(VA::Start));
    for t in 1..5000u64 {
        app.event(Event::Tick { now_ms: t * 100 });
        if app.vanity().unwrap().grind.find.is_some() {
            break;
        }
    }
    assert!(app.vanity().unwrap().grind.find.is_some(), "no find");
    eye_hides_at_lock(&mut app, Action::Vanity(VA::Show));
}

#[test]
fn a_lock_pressed_while_one_vault_is_sealed_hides_create_a_vault_s_passphrases_at_once() {
    let mut app = device(cheap_vault());
    open_and_change(&mut app);
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Create));
    app.press(Action::Vault(V::CFocus(0, false)));
    type_text(&mut app, "one phrase");
    app.press(Action::Vault(V::CFocus(0, true)));
    type_text(&mut app, "one phrase");
    // Focus off the phrase boxes, so only the eye shows what is typed.
    app.press(Action::Vault(V::CName));
    eye_hides_at_lock(&mut app, Action::Vault(V::CShow));
}

#[test]
fn a_lock_pressed_while_one_vault_is_sealed_hides_the_inbox_passphrase_at_once() {
    let mut app = device(cheap_vault());
    open_and_change(&mut app);
    let words = testkit::test_words("summer");
    app.storage(StorageEvent::Restored {
        inbox: vec![(
            faraday_text::Text::of("summer-words.txt"),
            words.as_bytes().to_vec().into(),
        )],
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    let c = testkit::compute();
    let fp = app.inbox_found_now(&c).potential()[0].fingerprint;
    app.press(Action::PotentialOpen(fp.0));
    assert_eq!(app.sheet, Some(Sheet::Potential));
    app.press(Action::PotentialType);
    type_text(&mut app, "a passphrase");
    eye_hides_at_lock(&mut app, Action::PotentialShow);
}

/// The checklist on the copy step with the words shown and Seed XOR made.
fn checklist_with_parts() -> Faraday {
    use faraday_core::bstep;
    use faraday_core::seeds::SeedsAction as S;
    let mut app = device(cheap_vault());
    open_and_change(&mut app);
    let fp = app.session.keys.last().expect("a key").fingerprint.0;
    app.press(Action::KeyWallet(fp, 1));
    app.press(Action::Seeds(S::Make));
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
    if app
        .backup_items_now(&testkit::compute())
        .contains(&bstep::BLANK)
    {
        app.press(Action::BNext(bstep::BLANK));
    }
    if app.backup().and_then(|b| b.open) != Some(bstep::COPY) {
        app.press(Action::BStep(bstep::COPY));
    }
    app.press(Action::BReveal);
    crate::common::top_up(&mut app);
    app.press(Action::BXor(2));
    app
}

fn parts_shown(app: &mut Faraday) -> bool {
    let line = EN.paper_section_parts.replace("{}", "2");
    text_on_screen(app, &line)
}

#[test]
fn a_lock_pressed_while_one_vault_is_sealed_takes_the_seed_xor_parts_off_the_screen_at_once() {
    let mut app = checklist_with_parts();
    assert!(parts_shown(&mut app), "no parts: {:?}", app.drawn_texts());
    lock_during_seal(&mut app);
    assert!(!parts_shown(&mut app), "the parts stay on show");
    seal_ends_and_locks(&mut app);
}

#[test]
fn the_idle_warning_keeps_the_seed_xor_parts_and_hides_the_words() {
    let mut app = checklist_with_parts();
    let word = testkit::TEST_SEEDS[1].0;
    let word_shown = |app: &mut Faraday| drawn(app).iter().any(|t| t == word);
    assert!(parts_shown(&mut app), "no parts");
    assert!(word_shown(&mut app), "the words are not shown");
    let last = app.now_ms();
    app.event(Event::Tick {
        now_ms: last + 26 * 60_000,
    });
    assert_eq!(app.sheet, Some(Sheet::IdleWarn));
    app.press(Action::Cancel);
    assert!(!word_shown(&mut app), "the idle warning left the words");
    // The parts sit with the words under Show the seed: the eye brings
    // both back, the parts the same.
    app.press(Action::BReveal);
    assert!(
        parts_shown(&mut app),
        "the idle warning dropped the parts: {:?}",
        app.drawn_texts()
    );
}

#[test]
fn the_idle_warning_hides_lightning_s_private_key() {
    use faraday_shell_api::Key;
    let mut app = device(cheap_vault());
    open_and_change(&mut app);
    app.press(Action::Lightning);
    app.press(Action::LAezeed);
    let cipher = "above judge emerge veteran reform crunch system all snap please \
        shoulder vault hurt city quarter cover enlist swear success suggest drink wagon enrich body";
    for w in cipher.split_whitespace() {
        type_text(&mut app, w);
        app.event(Event::Key(Key::Char(' ')));
    }
    app.event(Event::Key(Key::Enter));
    let _ = app.frame();
    let now = app.now_ms() + 16;
    app.event(Event::Tick { now_ms: now });
    testkit::settle(&mut app);
    let hidden = drawn(&mut app);
    app.press(Action::LShow);
    let added: Vec<String> = drawn(&mut app)
        .into_iter()
        .filter(|t| !hidden.contains(t))
        .collect();
    assert!(!added.is_empty(), "the eye shows nothing: {hidden:?}");
    let last = app.now_ms();
    app.event(Event::Tick {
        now_ms: last + 26 * 60_000,
    });
    assert_eq!(app.sheet, Some(Sheet::IdleWarn));
    app.press(Action::Cancel);
    let now = drawn(&mut app);
    assert!(added.iter().all(|t| !now.contains(t)), "still on show");
}

#[test]
fn a_lock_pressed_while_one_vault_is_sealed_hides_the_chart_s_passphrase_at_once() {
    use faraday_bip::bip39::{Language, Mnemonic};
    use faraday_core::create::NewKind;
    use faraday_core::glance::Press;
    use faraday_core::glance::sheet::{ChartAction as C, Target};
    use faraday_core::plan::{At, Edit, What};
    use faraday_core::testkit::KindText;
    const PASS: &str = "lantern-orchid-41";
    let mut app = device(cheap_vault());
    open_and_change(&mut app);
    let c = testkit::compute();
    let m = Mnemonic::parse(
        &c,
        Language::English,
        &testkit::test_words(testkit::TEST_SEEDS[0].0),
    )
    .unwrap();
    app.session
        .keys
        .add_mnemonic(&c, &m, PASS, "Key", None)
        .unwrap();
    let k = app.session.keys.len() - 1;
    let text = NewKind::NativeSegwit
        .key_text(&c, app.session.keys.master_for_tests(k))
        .unwrap();
    let w = app
        .session
        .add_wallet(
            &c,
            "Spending",
            &format!("wpkh({text}/<0;1>/*)"),
            "test",
            faraday_core::wallet::Origin::Named,
        )
        .unwrap();
    app.press(Action::Backup(w));
    let shape = app.plan_shape_now(w);
    testkit::set_backup_answers(
        &mut app,
        faraday_core::plan::Answers::preset_at(&shape, faraday_core::plan::Preset::ALL[1], 2),
    );
    app.press(Action::BChecklist);
    app.press(Action::OpenWallet(w));
    let _ = app.frame();
    let press = Press::Loaded(w);
    app.press(Action::Chart(C::Open(
        press,
        Target::CopyTo(What::Passphrase(0)),
    )));
    crate::common::a_moment(&mut app);
    app.press(Action::Chart(C::Edit(
        press,
        Edit::Add(At::Place(2), What::Passphrase(0)),
    )));
    app.press(Action::Chart(C::Confirm(press)));
    assert!(
        matches!(app.chart(), Some((_, Target::PassShow(..)))),
        "the passphrase is not shown to copy"
    );
    app.press(Action::Chart(C::Reveal));
    assert!(
        text_on_screen(&mut app, PASS),
        "the passphrase is not shown"
    );
    lock_during_seal(&mut app);
    assert!(
        !text_on_screen(&mut app, PASS),
        "the passphrase stays on show during the seal"
    );
    seal_ends_and_locks(&mut app);
}
