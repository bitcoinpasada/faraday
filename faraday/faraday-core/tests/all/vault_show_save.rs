//! A vault's key and wallet, shown and saved (`docs/BACKUP-FLOW.md`
//! §17.7): a key's SeedQR shows behind its eye and reads, as a
//! camera reads the screen, as the seed's SeedQR; Save to a stick says
//! what the stick gives whoever finds it before anything else, and writes
//! the forms ticked, under the names Back up gives them on a stick; a
//! wallet's Show as QR is the Wallets page's code of its descriptor, and
//! its Save writes the descriptor's text, its picture and the software's
//! files. Nothing new goes into the vault, and a wallet shown or saved
//! from the vault is not loaded. A secret shows at a press on its eye,
//! hides at the next and once its page is left. + Bitcoin key saves any
//! key loaded, a codex32 secret's among them, as the key record it reads
//! back from.

use faraday_bip::bip39::{Language, Mnemonic};
use faraday_core::vault::VaultAction as V;
use faraday_core::{Action, Faraday, Screen, Sheet, StorageEvent, testkit};
use faraday_shell_api::{App, BootState, DisplayInfo, Event, Key, SecureHardware, TouchPhase};
use faraday_vault::records::{field, kind};

fn kit_file(name: &str) -> (String, Vec<u8>) {
    testkit::files()
        .unwrap()
        .into_iter()
        .find(|(n, _)| n == name)
        .unwrap()
}

/// The test vault unlocked from a flow's way in, so that nothing in it
/// is loaded, on its contents.
fn opened() -> Faraday {
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
        inbox: vec![kit_file("vault.ofv")],
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    faraday_core::testkit::vault_speed(&mut app, Some(180));
    app.press(Action::Vault(V::OpenFrom(0, Screen::Files)));
    for c in testkit::VAULT_PASSPHRASES[0].chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    app.press(Action::Vault(V::Unlock));
    for t in 1..60u64 {
        if !app.vaults().open.is_empty() {
            break;
        }
        let _ = app.frame();
        app.event(Event::Tick { now_ms: t * 1000 });
    }
    assert_eq!(app.vaults().open.len(), 1, "the test vault did not unlock");
    app.press(Action::Nav(Screen::VaultContents));
    app
}

/// The vault's key record shown.
fn on_key() -> Faraday {
    let mut app = opened();
    app.press(Action::Vault(V::Category(0)));
    app.press(Action::Vault(V::Item(0)));
    let _ = app.frame();
    app
}

/// The vault's wallet record named `name` shown.
fn on_wallet(name: &str) -> Faraday {
    let mut app = opened();
    app.press(Action::Vault(V::Category(1)));
    let k = app
        .vaults()
        .store()
        .contents_for_tests(app.vaults().open[0].id)
        .records
        .iter()
        .filter(|r| [kind::WALLET, kind::WALLET_SHARE].contains(&r.kind))
        .position(|r| r.text(field::WALLET_NAME) == Some(name))
        .expect("the wallet is in the test vault");
    app.press(Action::Vault(V::Item(k)));
    let _ = app.frame();
    app
}

/// Test key 1's words, as typed.
fn words() -> String {
    testkit::test_words(testkit::TEST_SEEDS[0].0)
}

/// Test key 1's fingerprint in lower case, as the kit's own files name it.
fn fp() -> String {
    let name = testkit::files()
        .unwrap()
        .into_iter()
        .map(|(n, _)| n)
        .find(|n| n.starts_with("seed-1-") && n.ends_with("-words.txt"))
        .unwrap();
    name["seed-1-".len()..name.len() - "-words.txt".len()].to_lowercase()
}

/// What SeedQR's own encoder writes for test key 1, as a camera reads it.
fn seedqr_digits() -> Vec<u8> {
    let c = testkit::compute();
    let m = Mnemonic::parse(&c, Language::English, &words()).unwrap();
    let mx = faraday_codec::seedqr::encode_seedqr(&c, &m).unwrap();
    let (w, h, px) = mx.to_luma(4, faraday_codec::qr::QUIET_ZONE);
    let found = faraday_codec::decode_luma(&c, w, h, &px);
    assert_eq!(found.len(), 1);
    found[0].bytes.clone()
}

/// The codes a camera reads off the screen drawn now.
fn on_screen(app: &mut Faraday) -> Vec<Vec<u8>> {
    let c = faraday_core::testkit::compute();
    // Drawn afresh, as the shell draws after an event.
    app.settle();
    let f = app.frame();
    let luma: Vec<u8> = f
        .rgba
        .chunks_exact(4)
        .map(|p| ((u32::from(p[0]) * 3 + u32::from(p[1]) * 6 + u32::from(p[2])) / 10) as u8)
        .collect();
    faraday_codec::decode_luma(&c, usize::from(f.width), usize::from(f.height), &luma)
        .into_iter()
        .map(|d| d.bytes)
        .collect()
}

fn touch(app: &mut Faraday, (x, y): (u16, u16), phase: TouchPhase) {
    app.event(Event::Touch { x, y, phase });
}

/// The one code in a PNG.
fn read_png(png: &[u8]) -> Vec<u8> {
    let c = faraday_core::testkit::compute();
    let payloads = faraday_files::qr_in_png(&c, png).expect("a PNG");
    assert_eq!(payloads.len(), 1, "one code");
    payloads[0].clone()
}

fn outbox<'a>(app: &'a Faraday, name: &str) -> &'a faraday_core::Item {
    app.outbox
        .iter()
        .find(|i| i.name == name)
        .unwrap_or_else(|| {
            panic!(
                "no {name} in {:?}",
                app.outbox.iter().map(|i| &i.name).collect::<Vec<_>>()
            )
        })
}

fn outbox_names(app: &Faraday) -> Vec<String> {
    app.outbox.iter().map(|i| i.name.clone()).collect()
}

/// The open vault's records and changes, to say nothing went in.
fn vault_now(app: &Faraday) -> (usize, usize) {
    let v = &app.vaults().open[0];
    (
        app.vaults().store().contents_for_tests(v.id).records.len(),
        v.changes,
    )
}

/// A tap at `at`: down and up in one place.
fn tap(app: &mut Faraday, at: (u16, u16)) {
    touch(app, at, TouchPhase::Down);
    touch(app, at, TouchPhase::Up);
}

/// Whether the frame drawn now shows test key 1's words.
fn words_on_screen(app: &mut Faraday) -> bool {
    app.settle();
    let _ = app.frame();
    let w = words();
    let two = w.split(' ').take(2).collect::<Vec<_>>().join(" ");
    app.drawn_texts().iter().any(|t| t.contains(&two))
}

#[test]
fn a_key_s_seedqr_shows_at_a_tap_on_its_eye_and_hides_at_the_next() {
    let mut app = on_key();
    let show = Action::Vault(V::ShowSeedQr);
    let at = app
        .where_offered(show)
        .expect("Show SeedQR's eye is offered");
    assert!(on_screen(&mut app).is_empty(), "a code before the tap");
    // A press held shows nothing: only the tap does.
    touch(&mut app, at, TouchPhase::Down);
    assert!(on_screen(&mut app).is_empty(), "a code while pressed");
    touch(&mut app, at, TouchPhase::Up);
    let codes = on_screen(&mut app);
    assert_eq!(codes, vec![seedqr_digits()], "the code shown is the seed's");
    // It stays after the lift, through other frames.
    assert_eq!(on_screen(&mut app), vec![seedqr_digits()]);
    let at = app.where_offered(show).expect("the eye that hides it");
    tap(&mut app, at);
    assert!(
        on_screen(&mut app).is_empty(),
        "a code after the second tap"
    );
    assert!(app.session.keys.is_empty(), "showing it loads the key");
}

#[test]
fn a_key_s_words_show_at_a_press_on_their_eye_and_hide_at_the_next() {
    let mut app = on_key();
    let eye = Action::Vault(V::Reveal(0));
    assert!(app.offers(eye), "the words' eye");
    assert!(!words_on_screen(&mut app), "the words before the press");
    app.press(eye);
    assert!(words_on_screen(&mut app), "the words after the press");
    assert!(words_on_screen(&mut app), "the words on the next frame");
    app.press(eye);
    assert!(
        !words_on_screen(&mut app),
        "the words after the second press"
    );
    assert!(app.session.keys.is_empty(), "showing them loads the key");
}

#[test]
fn a_shown_secret_is_hidden_again_after_leaving_and_coming_back() {
    let mut app = on_key();
    app.press(Action::Vault(V::Reveal(0)));
    app.press(Action::Vault(V::ShowSeedQr));
    assert!(words_on_screen(&mut app));
    assert_eq!(on_screen(&mut app), vec![seedqr_digits()]);
    // Another screen, and back to the same record.
    app.press(Action::Nav(Screen::Home));
    app.press(Action::Nav(Screen::VaultContents));
    let _ = app.frame();
    assert!(app.offers(Action::Vault(V::Reveal(0))), "back on the key");
    assert!(!words_on_screen(&mut app), "the words after coming back");
    assert!(on_screen(&mut app).is_empty(), "the code after coming back");
    // Another kind, and back to the same record.
    app.press(Action::Vault(V::Reveal(0)));
    assert!(words_on_screen(&mut app));
    app.press(Action::Vault(V::Category(1)));
    app.press(Action::Vault(V::Category(0)));
    app.press(Action::Vault(V::Item(0)));
    assert!(!words_on_screen(&mut app), "the words after another kind");
    // The vault locked with the key's words shown.
    app.press(Action::Vault(V::Reveal(0)));
    assert!(words_on_screen(&mut app));
    app.press(Action::LockUnsealed);
    assert!(!words_on_screen(&mut app), "the words after the lock");
}

#[test]
fn save_to_a_stick_says_the_seed_line_first_and_writes_the_ticked_forms() {
    let mut app = on_key();
    let before = vault_now(&app);
    assert!(app.offers(Action::Vault(V::SaveToStick)));
    app.press(Action::Vault(V::SaveToStick));
    assert_eq!(app.sheet, Some(Sheet::VaultSave));
    assert!(app.outbox.is_empty(), "nothing is written before Save");
    let _ = app.frame();
    let texts = app.drawn_texts();
    let line = texts
        .iter()
        .position(|t| t == "Anyone who finds this stick has this seed")
        .expect("the seed line");
    let form = texts.iter().position(|t| t == "Words").expect("Words");
    assert!(line < form, "the seed line comes first: {texts:?}");
    let fp = fp();
    let (txt, png) = (format!("seed-{fp}.txt"), format!("seedqr-{fp}.png"));
    assert!(texts.contains(&txt) && texts.contains(&png), "{texts:?}");
    app.press(Action::Vault(V::SaveToStickGo));
    // A key's words and SeedQR as plain files ask for the hold (F129).
    assert_eq!(app.sheet, Some(Sheet::SecretOut));
    assert!(app.outbox.is_empty(), "nothing is written before the hold");
    app.press(Action::SecretHold);
    assert_eq!(app.sheet, None);
    let words_file = outbox(&app, &txt);
    assert!(words_file.secret);
    assert_eq!(words_file.bytes, format!("{}\n", words()).into_bytes());
    let picture = outbox(&app, &png);
    assert!(picture.secret);
    assert_eq!(read_png(&picture.bytes), seedqr_digits());
    assert_eq!(app.outbox.len(), 2);
    assert_eq!(vault_now(&app), before, "the vault holds something new");
}

/// A Japanese seed kept in a vault, saved to a stick as its words: the
/// file is the words as Add a key reads them, the longest list's words
/// whole.
#[test]
fn a_japanese_seed_saved_to_a_stick_is_its_words() {
    use faraday_vault::records;
    let c = testkit::compute();
    let m = Mnemonic::from_entropy(Language::Japanese, &[0xb7; 32]).unwrap();
    let record = faraday_vault::Record::new(kind::KEY)
        .with(field::KEY, &records::words_payload(&m))
        .with(field::KEY_LABEL, b"Japanese");
    let vault = faraday_vault::create(
        &c,
        faraday_vault::SLOT_SIZES[0],
        faraday_vault::Cost {
            memory_kib: 64 * 1024,
            passes: 1,
            lanes: 1,
        },
        &[b"japanese phrase"],
        &[faraday_vault::Contents {
            records: vec![record],
        }],
        &[0x6a; 32],
    )
    .unwrap();
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
        inbox: vec![("japanese.ofv".into(), vault)],
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    faraday_core::testkit::vault_speed(&mut app, Some(180));
    app.press(Action::Vault(V::OpenFrom(0, Screen::Files)));
    crate::common::type_text(&mut app, "japanese phrase");
    app.press(Action::Vault(V::Unlock));
    for t in 1..60u64 {
        if !app.vaults().open.is_empty() {
            break;
        }
        let _ = app.frame();
        app.event(Event::Tick { now_ms: t * 1000 });
    }
    assert_eq!(app.vaults().open.len(), 1);
    app.press(Action::Nav(Screen::VaultContents));
    app.press(Action::Vault(V::Category(0)));
    app.press(Action::Vault(V::Item(0)));
    app.press(Action::Vault(V::SaveToStick));
    app.press(Action::Vault(V::SaveTick(1)));
    app.press(Action::Vault(V::SaveToStickGo));
    app.press(Action::SecretHold);
    let file = app
        .outbox
        .iter()
        .find(|i| i.name.starts_with("seed-") && i.name.ends_with(".txt"))
        .expect("the words file");
    let want: Vec<&str> = m
        .indices()
        .iter()
        .map(|&i| Language::Japanese.word(i))
        .collect();
    assert_eq!(file.bytes, format!("{}\n", want.join(" ")).into_bytes());
}

#[test]
fn save_to_a_stick_writes_only_the_form_left_ticked() {
    let mut app = on_key();
    app.press(Action::Vault(V::SaveToStick));
    // Words untouched, SeedQR unticked.
    app.press(Action::Vault(V::SaveTick(1)));
    app.press(Action::Vault(V::SaveToStickGo));
    app.press(Action::SecretHold);
    assert_eq!(outbox_names(&app), vec![format!("seed-{}.txt", fp())]);
}

#[test]
fn cancel_writes_nothing() {
    let mut app = on_key();
    app.press(Action::Vault(V::SaveToStick));
    app.press(Action::Cancel);
    assert_eq!(app.sheet, None);
    assert!(app.outbox.is_empty());
}

#[test]
fn a_wallet_s_show_as_qr_is_its_descriptor_and_loads_nothing() {
    let mut app = on_wallet("Savings");
    assert!(app.offers(Action::Vault(V::WalletQr)));
    app.press(Action::Vault(V::WalletQr));
    assert_eq!(app.sheet, Some(Sheet::Qr));
    let q = app.qr().expect("the QR view");
    let kit = testkit::kits()
        .into_iter()
        .find(|k| k.id == "savings")
        .unwrap();
    let whole = q.whole.clone().expect("the descriptor under the code");
    assert_eq!(
        whole.split('#').next(),
        kit.descriptor.split('#').next(),
        "the code is not the vault's descriptor"
    );
    assert!(!q.frames.is_empty());
    assert!(
        app.session.wallets.is_empty(),
        "showing it loads the wallet"
    );
}

#[test]
fn a_wallet_s_save_writes_its_descriptor_text_qr_and_software_files() {
    let mut app = on_wallet("Spending");
    let before = vault_now(&app);
    app.press(Action::Vault(V::SaveToStick));
    assert_eq!(app.sheet, Some(Sheet::VaultSave));
    let _ = app.frame();
    let texts = app.drawn_texts();
    assert!(
        !texts.contains(&"Anyone who finds this stick has this seed".to_string()),
        "a wallet's files are public"
    );
    let files = &app.vaults().stick_save.as_ref().unwrap().files;
    // The descriptor ticked to begin with; the QR picture and Sparrow's
    // file not.
    let qr = files
        .iter()
        .position(|f| f.label == "QR picture")
        .expect("QR");
    let sparrow = files
        .iter()
        .position(|f| f.label == "Sparrow file")
        .expect("Sparrow");
    assert!(files[0].ticked && !files[qr].ticked && !files[sparrow].ticked);
    app.press(Action::Vault(V::SaveTick(qr as u8)));
    app.press(Action::Vault(V::SaveTick(sparrow as u8)));
    app.press(Action::Vault(V::SaveToStickGo));
    let kit = testkit::kits()
        .into_iter()
        .find(|k| k.id == "spending")
        .unwrap();
    let text = String::from_utf8(outbox(&app, "spending-descriptor.txt").bytes.clone()).unwrap();
    assert_eq!(
        text.trim_end().split('#').next(),
        kit.descriptor.split('#').next()
    );
    let picture = outbox(&app, "spending-descriptor.png");
    assert!(!picture.secret);
    assert_eq!(
        String::from_utf8(read_png(&picture.bytes)).unwrap(),
        text.trim_end()
    );
    let json = String::from_utf8(outbox(&app, "spending-wallet.json").bytes.clone()).unwrap();
    assert!(json.contains("\"descriptor\""), "{json}");
    assert_eq!(app.outbox.len(), 3, "{:?}", outbox_names(&app));
    assert!(app.session.wallets.is_empty(), "saving it loads the wallet");
    assert_eq!(vault_now(&app), before, "the vault holds something new");
}

/// BIP 93's vector 1: a codex32 secret alone (`tools/vectors/bip93`).
const CODEX32_SECRET: &str = "ms10testsxxxxxxxxxxxxxxxxxxxxxxxxxx4nzvca9cmczlw";

#[test]
fn a_codex32_key_saved_by_bitcoin_key_reads_back_from_the_vault_as_itself() {
    let c = testkit::compute();
    let mut app = opened();
    // The secret typed in on Add a key's codex32 form.
    app.press(Action::Entry(None));
    app.press(Action::EntryForm(2));
    crate::common::type_text(&mut app, CODEX32_SECRET);
    app.event(Event::Key(Key::Enter));
    if app.session.keys.is_empty() {
        app.event(Event::Key(Key::Enter));
    }
    assert_eq!(
        app.session.keys.len(),
        1,
        "{:?}",
        testkit::entry(&app).error
    );
    let fp = app.session.keys[0].fingerprint;
    let facts = app.session.keys[0].codex32.clone();
    assert!(!app.session.keys[0].has_words(), "a key of a master seed");
    let secret = faraday_bip::codex32::Codex32::parse(CODEX32_SECRET).unwrap();
    assert_eq!(
        app.session
            .keys
            .codex32_rebuilds(&c, 0, std::slice::from_ref(&secret)),
        Some(true)
    );
    // + Bitcoin key offers it, and Save keeps it in the vault.
    app.press(Action::Nav(Screen::VaultContents));
    app.press(Action::Vault(V::Category(0)));
    app.press(Action::Vault(V::Add));
    let _ = app.frame();
    let save = Action::Vault(V::SaveKey(0));
    assert!(app.offers(save), "+ Bitcoin key offers the codex32 key");
    // The Vaults page's row for the key offers Save while it is in no vault.
    let vaults_page_save = |app: &Faraday| {
        let c = testkit::compute();
        app.kept_wiped_now(&c, false)
            .wiped
            .iter()
            .any(|(_, _, a)| *a == Some(save))
    };
    assert!(vaults_page_save(&app), "the Vaults page offers Save");
    let (records, _) = vault_now(&app);
    app.press(save);
    assert_eq!(vault_now(&app).0, records + 1, "one record more");
    assert!(
        !vaults_page_save(&app),
        "the Vaults page offers Save for a key already in the vault"
    );
    // Out of the session, and loaded again from the vault's record.
    app.session.keys.remove(0);
    assert!(app.session.keys.is_empty());
    // The record saved is the kind's last.
    let id = app.vaults().open[0].id;
    let last = app
        .vaults()
        .store()
        .contents_for_tests(id)
        .records
        .iter()
        .filter(|r| [kind::KEY, kind::PASSPHRASE, kind::CODEX32_SHARE].contains(&r.kind))
        .count()
        - 1;
    let saved = &app.vaults().store().contents_for_tests(id).records;
    assert_eq!(
        saved.last().map(|r| r.kind),
        Some(kind::KEY),
        "a key record"
    );
    app.press(Action::Vault(V::Item(last)));
    app.press(Action::Vault(V::Load));
    let k = app
        .session
        .keys
        .position_of(fp.0)
        .expect("the vault's record loads the key");
    assert_eq!(app.session.keys[k].fingerprint, fp, "the same fingerprint");
    assert_eq!(app.session.keys[k].codex32, facts, "the same codex32 set");
    assert_eq!(
        app.session.keys.codex32_rebuilds(&c, k, &[secret]),
        Some(true),
        "the secret string rebuilds the key read back"
    );
    // Saved, it is no longer offered.
    app.press(Action::Vault(V::Add));
    if !app.vaults().saving {
        app.press(Action::Vault(V::Add));
    }
    let _ = app.frame();
    assert!(!app.offers(Action::Vault(V::SaveKey(k))), "offered again");
}

#[test]
fn an_entry_s_notes_show_at_a_press_on_their_eye_and_hide_on_leaving() {
    let mut app = opened();
    // An entry with a title and notes, added on Entries.
    app.press(Action::Vault(V::Category(2)));
    app.press(Action::Vault(V::Add));
    app.press(Action::Vault(V::FocusField(0)));
    crate::common::type_text(&mut app, "Locker");
    app.press(Action::Vault(V::FocusField(4)));
    crate::common::type_text(&mut app, "combination under the mat");
    app.press(Action::Vault(V::FormSave));
    let notes_shown = |app: &mut Faraday| {
        app.settle();
        let _ = app.frame();
        let t = app.drawn_texts();
        assert!(t.iter().any(|x| x == "Locker"), "the entry: {t:?}");
        t.iter().any(|x| x.contains("under the mat"))
    };
    let eye = Action::Vault(V::Reveal(0));
    assert!(!notes_shown(&mut app), "the notes before the press");
    assert!(app.offers(eye), "the notes' eye");
    app.press(eye);
    assert!(notes_shown(&mut app), "the notes after the press");
    app.press(eye);
    assert!(!notes_shown(&mut app), "the notes after the second press");
    app.press(eye);
    app.press(Action::Nav(Screen::Home));
    app.press(Action::Nav(Screen::VaultContents));
    assert!(!notes_shown(&mut app), "the notes after coming back");
}
