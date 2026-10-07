//! What a person scanning codes into Files sees: a transfer in parts says
//! which parts are still missing and lands in the Inbox whole; a file sent
//! in Faraday's envelope keeps its name; an address is checked against the
//! loaded wallets; and a seed's words are named and kept out.

use faraday_core::testkit;
use faraday_core::wallet::FileKind;
use faraday_core::{Action, Faraday, StorageEvent};
use osk_shell_api::{App, Event};

fn kit(id: &str) -> testkit::Kit {
    testkit::kits().into_iter().find(|k| k.id == id).unwrap()
}

fn scanning(inbox: Vec<(&str, Vec<u8>)>) -> Faraday {
    let mut app = Faraday::new();
    app.storage(StorageEvent::Restored {
        inbox: inbox.into_iter().map(|(n, b)| (n.to_string(), b)).collect(),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app.press(Action::Scan);
    app
}

fn scan(app: &mut Faraday, text: &str) {
    app.event(Event::Scanned {
        bytes: text.as_bytes().to_vec(),
    });
}

fn note(app: &Faraday) -> String {
    app.scan
        .as_ref()
        .and_then(|s| s.note.clone())
        .unwrap_or_default()
}

#[test]
fn a_psbt_in_bbqr_parts_says_what_is_missing_and_arrives_whole() {
    let psbt = testkit::unsigned(&kit("spending")).unwrap().to_bytes();
    let parts = faraday_qr::bbqr::encode('P', &psbt, 60).unwrap();
    assert!(parts.len() >= 3, "{} parts", parts.len());
    let mut app = scanning(Vec::new());
    scan(&mut app, &parts[1]);
    let first = note(&app);
    assert!(first.contains(&format!("1 of {}", parts.len())), "{first}");
    assert!(first.contains("missing 1, 3"), "{first}");
    for p in parts.iter().rev() {
        scan(&mut app, p);
    }
    let item = app.inbox.last().expect("nothing reached the Inbox");
    assert_eq!(item.kind, FileKind::Psbt);
    assert_eq!(item.bytes, psbt);
}

#[test]
fn a_file_in_the_envelope_arrives_under_its_own_name() {
    let wallet = kit("spending").descriptor;
    let env = faraday_qr::envelope::pack("spending-wallet.txt", wallet.as_bytes(), "file").unwrap();
    let mut app = scanning(Vec::new());
    for p in faraday_qr::bbqr::encode('J', &env, 100).unwrap() {
        scan(&mut app, &p);
    }
    let item = app.inbox.last().expect("nothing reached the Inbox");
    assert_eq!(item.name, "spending-wallet.txt");
    assert_eq!(item.bytes, wallet.as_bytes());
}

#[test]
fn a_scanned_address_is_named_as_a_loaded_wallets_own() {
    let mut app = scanning(vec![(
        "spending-wallet.txt",
        kit("spending").descriptor.into_bytes(),
    )]);
    app.press(Action::LoadWallet(0));
    let ours = app.session.address(&app.session.wallets[0], false, 3);
    app.press(Action::Scan);
    scan(&mut app, &format!("bitcoin:{ours}?amount=0.001"));
    let said = note(&app);
    assert!(said.contains("receive address 3"), "{said}");
    assert_eq!(app.inbox.len(), 1, "the address was filed");
}

#[test]
fn a_seeds_words_scanned_into_files_are_named_and_not_filed() {
    let mut app = scanning(Vec::new());
    scan(
        &mut app,
        "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about",
    );
    assert!(note(&app).contains("seed's words"), "{}", note(&app));
    assert!(app.inbox.is_empty());
}

#[test]
fn closing_the_camera_mid_transfer_keeps_the_parts_read() {
    let psbt = testkit::unsigned(&kit("spending")).unwrap().to_bytes();
    let parts = faraday_qr::bbqr::encode('P', &psbt, 60).unwrap();
    let mut app = scanning(Vec::new());
    scan(&mut app, &parts[0]);
    app.press(Action::Cancel);
    app.press(Action::Scan);
    for p in &parts[1..] {
        scan(&mut app, p);
    }
    let item = app.inbox.last().expect("the transfer did not complete");
    assert_eq!(item.bytes, psbt);
}

#[test]
fn a_code_says_whether_it_is_safe_to_scan() {
    let mut app = Faraday::new();
    app.storage(StorageEvent::Restored {
        inbox: Vec::new(),
        outbox: vec![
            (
                "spending-wallet.txt".to_string(),
                kit("spending").descriptor.into_bytes(),
            ),
            (
                "threshold-partly-signed.osk".to_string(),
                testkit::threshold_carry().unwrap(),
            ),
        ],
        kept: Vec::new(),
    });
    app.press(Action::QrOutbox(0));
    assert!(!app.qr.as_ref().expect("no QR").secret);
    app.press(Action::Cancel);
    app.press(Action::QrOutbox(1));
    assert!(app.qr.as_ref().expect("no QR").secret);
}

#[test]
fn a_cosigners_key_scanned_while_making_a_multisig_fills_the_waiting_slot() {
    let mut app = Faraday::new();
    app.press(Action::CreateWallet);
    app.press(Action::CKind(4));
    app.press(Action::CNext(0));
    app.press(Action::CNext(1));
    app.press(Action::CSlotLater(1));
    app.press(Action::Scan);
    let key = testkit::key(2, "48h/1h/0h/2h");
    scan(&mut app, &key);
    assert_eq!(app.screen, faraday_core::Screen::Create);
    let c = app.create.as_ref().unwrap();
    assert!(
        matches!(&c.slots[1], faraday_core::create::Source::Cosigner(k) if k.replace('\'', "h") == key.replace('\'', "h")),
        "{:?}",
        c.slots
    );
}

fn seen(app: &mut Faraday, module_tenths: u16, read: bool) {
    app.storage(StorageEvent::QrSeen {
        width: 640,
        height: 480,
        corners: [(100, 100), (300, 100), (300, 300), (100, 300)],
        module_tenths,
        read,
    });
}

#[test]
fn a_code_too_fine_for_the_camera_is_said_to_be() {
    let mut app = scanning(Vec::new());
    seen(&mut app, 14, false);
    seen(&mut app, 14, false);
    assert!(!note(&app).contains("Too fine"));
    seen(&mut app, 14, false);
    assert!(note(&app).contains("Too fine"), "{}", note(&app));
    // The outline is there for the sheet to draw.
    assert!(app.scan.as_ref().unwrap().seen.is_some());
    // A code with modules large enough does not count.
    let mut app = scanning(Vec::new());
    for _ in 0..5 {
        seen(&mut app, 60, false);
    }
    assert!(!note(&app).contains("Too fine"));
}

#[test]
fn a_camera_chosen_on_the_sheet_goes_to_the_shell() {
    let mut app = scanning(Vec::new());
    app.storage(StorageEvent::Cameras(vec![
        ("/dev/video0".into(), "Integrated Camera".into()),
        ("/dev/video2".into(), "USB Camera".into()),
    ]));
    assert_eq!(app.take_camera(), None);
    app.press(Action::ScanCamera(1));
    assert_eq!(app.take_camera().as_deref(), Some("/dev/video2"));
    assert_eq!(app.take_camera(), None);
    assert_eq!(app.camera.as_deref(), Some("/dev/video2"));
}

/// Pixels of the drawn frame that are strongly blue.
fn blue_pixels(app: &mut Faraday) -> usize {
    let f = app.frame();
    f.rgba
        .chunks_exact(4)
        .filter(|p| i32::from(p[2]) - i32::from(p[0]) > 80)
        .count()
}

#[test]
fn the_camera_preview_is_in_colour_when_the_camera_gives_colour() {
    let mut app = Faraday::new();
    app.event(Event::Display(osk_shell_api::DisplayInfo {
        width: 1366,
        height: 768,
        dpi: 160,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: osk_shell_api::SecureHardware::None,
        boot: osk_shell_api::BootState::Unknown,
        memory_mib: None,
    }));
    app.press(Action::Scan);
    let (w, h) = (320u16, 240u16);
    let luma = vec![110u8; usize::from(w) * usize::from(h)];
    app.event(Event::CameraFrame {
        width: w,
        height: h,
        luma: luma.clone(),
        chroma: None,
    });
    let grey = blue_pixels(&mut app);
    // A blue scene: U high, V low, in every 2 × 2 block.
    let chroma: Vec<u8> = [255u8, 0].repeat(usize::from(w / 2) * usize::from(h / 2));
    app.event(Event::CameraFrame {
        width: w,
        height: h,
        luma,
        chroma: Some(chroma),
    });
    let colour = blue_pixels(&mut app);
    assert!(
        colour > grey + 10_000,
        "the preview is not in colour: {grey} then {colour}"
    );
}
