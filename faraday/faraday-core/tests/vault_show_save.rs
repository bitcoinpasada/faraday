//! A vault's key and wallet, shown and saved (`docs/BACKUP-FLOW.md`
//! §19.7): a key's SeedQR shows only while it is held and reads, as a
//! camera reads the screen, as the seed's SeedQR; Save to a stick says
//! what the stick gives whoever finds it before anything else, and writes
//! the forms ticked, under the names Back up gives them on a stick; a
//! wallet's Show as QR is the Wallets page's code of its descriptor, and
//! its Save writes the descriptor's text, its picture and the software's
//! files. Nothing new goes into the vault, and a wallet shown or saved
//! from the vault is not loaded.

use faraday_core::vaults::VaultAction as V;
use faraday_core::{Action, Faraday, Screen, Sheet, StorageEvent, testkit};
use faraday_vault::records::{field, kind};
use osk_bip::bip39::{Language, Mnemonic};
use osk_shell_api::{App, BootState, DisplayInfo, Event, Key, SecureHardware, TouchPhase};

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
    app.vaults.ms_per_unit = Some(180);
    app.press(Action::Vault(V::OpenFrom(0, Screen::Files)));
    for c in testkit::VAULT_PASSPHRASES[0].chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    app.press(Action::Vault(V::Unlock));
    for t in 1..60u64 {
        if !app.vaults.open.is_empty() {
            break;
        }
        let _ = app.frame();
        app.event(Event::Tick { now_ms: t * 1000 });
    }
    assert_eq!(app.vaults.open.len(), 1, "the test vault did not unlock");
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
    let k = app.vaults.open[0]
        .contents
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
    let mx = osk_codec::seedqr::encode_seedqr(&c, &m).unwrap();
    let (w, h, px) = mx.to_luma(4, osk_codec::qr::QUIET_ZONE);
    let found = osk_codec::decode_luma(&c, w, h, &px);
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
    osk_codec::decode_luma(&c, usize::from(f.width), usize::from(f.height), &luma)
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
    let v = &app.vaults.open[0];
    (v.contents.records.len(), v.changes)
}

#[test]
fn a_key_s_seedqr_shows_only_while_held_and_reads_as_its_seed() {
    let mut app = on_key();
    let show = Action::Vault(V::ShowSeedQr);
    let at = app.where_offered(show).expect("Show SeedQR is offered");
    assert!(on_screen(&mut app).is_empty(), "a code before the hold");
    touch(&mut app, at, TouchPhase::Down);
    let codes = on_screen(&mut app);
    assert_eq!(codes, vec![seedqr_digits()], "the held code is the seed's");
    touch(&mut app, at, TouchPhase::Up);
    assert!(on_screen(&mut app).is_empty(), "a code after the lift");
    assert!(app.session.keys.is_empty(), "showing it loads the key");
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

#[test]
fn save_to_a_stick_writes_only_the_form_left_ticked() {
    let mut app = on_key();
    app.press(Action::Vault(V::SaveToStick));
    // Words untouched, SeedQR unticked.
    app.press(Action::Vault(V::SaveTick(1)));
    app.press(Action::Vault(V::SaveToStickGo));
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
    let q = app.qr.as_ref().expect("the QR view");
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
    let files = &app.vaults.stick_save.as_ref().unwrap().files;
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
