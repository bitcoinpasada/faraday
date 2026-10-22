//! What a person scanning codes into Files sees: a transfer in parts says
//! which parts are still missing and lands in the Inbox whole; a file sent
//! in Faraday's envelope keeps its name; an address is checked against the
//! loaded wallets; and a seed's words are never filed, nor pasted here
//! (a seed scanner takes them, `paste_a_seed`).

use faraday_core::testkit;
use faraday_core::wallet::FileKind;
use faraday_core::{Action, Faraday, StorageEvent};
use faraday_shell_api::{App, Event};

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
    let c = faraday_core::testkit::compute();
    let mut app = scanning(vec![(
        "spending-wallet.txt",
        kit("spending").descriptor.into_bytes(),
    )]);
    app.press(Action::LoadWallet(0));
    let ours = app.session.address(&c, &app.session.wallets[0], false, 3);
    app.press(Action::Scan);
    scan(&mut app, &format!("bitcoin:{ours}?amount=0.001"));
    let said = note(&app);
    assert!(said.contains("receive address 3"), "{said}");
    assert_eq!(app.inbox.len(), 1, "the address was filed");
}

#[test]
fn a_scanned_address_with_one_character_wrong_is_refused_for_its_checksum() {
    let c = faraday_core::testkit::compute();
    let mut app = scanning(vec![(
        "spending-wallet.txt",
        kit("spending").descriptor.into_bytes(),
    )]);
    app.press(Action::LoadWallet(0));
    let ours = app.session.address(&c, &app.session.wallets[0], false, 0);
    // The same address with its last character changed: a bech32
    // checksum catches any one wrong character.
    let mut wrong = ours.clone();
    let last = wrong.pop().unwrap();
    wrong.push(if last == 'q' { 'p' } else { 'q' });
    app.press(Action::Scan);
    scan(&mut app, &wrong);
    let said = note(&app);
    assert!(said.contains("fails its checksum"), "{said}");
    assert_eq!(app.inbox.len(), 1, "the address was filed");
}

#[test]
fn a_seeds_words_scanned_are_never_filed() {
    let mut app = scanning(Vec::new());
    scan(
        &mut app,
        "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about",
    );
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
    app.event(Event::Display(faraday_shell_api::DisplayInfo {
        width: 1366,
        height: 768,
        dpi: 160,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: faraday_shell_api::SecureHardware::None,
        boot: faraday_shell_api::BootState::Unknown,
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

/// The parts of a PSBT as a UR animation shows them, in order: the
/// fragments first, then mixes of them; and how many fragments there are.
fn ur_frames(psbt: &[u8], n: usize) -> (Vec<String>, usize) {
    let mut e = faraday_codec::ur::Encoder::psbt(psbt, 60).unwrap();
    let count = e.fragment_count();
    ((0..n).map(|_| e.next_part()).collect(), count)
}

fn state(app: &Faraday) -> String {
    app.scan
        .as_ref()
        .map(|s| s.state_line().to_string())
        .unwrap_or_default()
}

#[test]
fn a_second_ur_animation_starts_again_instead_of_sticking() {
    let first = testkit::unsigned(&kit("spending")).unwrap().to_bytes();
    let second = testkit::unsigned(&kit("taproot")).unwrap().to_bytes();
    let (a, a_count) = ur_frames(&first, 40);
    let (b, b_count) = ur_frames(&second, 40);
    assert!(
        a_count >= 4 && b_count >= 4,
        "{a_count} and {b_count} parts"
    );
    let mut app = scanning(Vec::new());
    scan(&mut app, &a[0]);
    scan(&mut app, &a[1]);
    // The second animation comes in the middle of the first.
    scan(&mut app, &b[0]);
    assert_eq!(note(&app), "A new QR started; reading it from the start");
    scan(&mut app, &b[1]);
    assert!(note(&app).starts_with("UR: 2 of"), "{}", note(&app));
    // Shown in turn, each frame starts the other again; held alone, the
    // second arrives whole.
    scan(&mut app, &a[2]);
    scan(&mut app, &b[2]);
    for p in &b {
        scan(&mut app, p);
        if app.scan.is_none() {
            break;
        }
    }
    let item = app
        .inbox
        .last()
        .expect("the second animation did not arrive");
    assert_eq!(item.kind, FileKind::Psbt);
    assert_eq!(item.bytes, second);
    // Then the first, from its start, on the scanner opened again.
    app.press(Action::Scan);
    for p in &a {
        scan(&mut app, p);
        assert!(!note(&app).contains("Mismatch"), "{}", note(&app));
        if app.scan.is_none() {
            break;
        }
    }
    assert_eq!(app.inbox.last().unwrap().bytes, first);
}

#[test]
fn a_damaged_ur_part_leaves_the_animation_being_read() {
    let psbt = testkit::unsigned(&kit("spending")).unwrap().to_bytes();
    let (a, count) = ur_frames(&psbt, 40);
    let mut app = scanning(Vec::new());
    scan(&mut app, &a[0]);
    // The part's last byteword changed: its checksum fails.
    let mut broken = a[1].clone();
    broken.truncate(broken.len() - 4);
    broken.push_str(if a[1].ends_with("able") {
        "acid"
    } else {
        "able"
    });
    scan(&mut app, &broken);
    assert_eq!(note(&app), "That QR is not a readable UR part");
    scan(&mut app, &a[2]);
    assert!(
        note(&app).starts_with(&format!("UR: 2 of {count}")),
        "{}",
        note(&app)
    );
}

#[test]
fn ur_progress_names_the_missing_parts() {
    let psbt = testkit::unsigned(&kit("spending")).unwrap().to_bytes();
    let (a, count) = ur_frames(&psbt, 40);
    assert!(count >= 5, "{count} parts");
    let mut app = scanning(Vec::new());
    scan(&mut app, &a[1]);
    scan(&mut app, &a[3]);
    let said = note(&app);
    assert!(
        said.starts_with(&format!("UR: 2 of {count} · missing 1, 3, 5")),
        "{said}"
    );
    // The bar inside the viewfinder follows the parts read.
    let share = app.scan.as_ref().unwrap().parts_read().unwrap();
    assert!((share - 2.0 / count as f32).abs() < 1e-6, "{share}");
}

fn paste(app: &mut Faraday, text: &str) {
    app.press(Action::ScanPaste);
    let asked = std::iter::from_fn(|| app.poll_command())
        .any(|c| matches!(c, faraday_shell_api::Command::RequestClipboard { .. }));
    assert!(asked, "Paste did not ask the shell for the clipboard");
    app.event(Event::Clipboard {
        kind: faraday_shell_api::FileKind::Text,
        text: text.to_string(),
    });
}

#[test]
fn paste_reads_the_clipboard_as_a_scan_reads_a_qr() {
    let mut app = scanning(Vec::new());
    app.clipboard = true;
    paste(&mut app, &kit("spending").descriptor);
    let item = app.inbox.last().expect("the paste did not arrive");
    assert_eq!(item.bytes, kit("spending").descriptor.as_bytes());
}

#[test]
fn paste_takes_ur_parts_as_the_camera_does() {
    let psbt = testkit::unsigned(&kit("spending")).unwrap().to_bytes();
    let (a, count) = ur_frames(&psbt, 40);
    let mut app = scanning(Vec::new());
    app.clipboard = true;
    for p in &a[..count] {
        paste(&mut app, p);
    }
    assert_eq!(app.inbox.last().expect("nothing arrived").bytes, psbt);
}

#[test]
fn a_secret_pasted_where_it_is_not_taken_is_refused() {
    let words = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
    // Words with a bad checksum are words all the same.
    let unchecked = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon";
    let seedqr = "0000".repeat(11) + "0003";
    let xprv = "xprv9s21ZrQH143K3QTDL4LXw2F7HEK3wJUD2nW2nRk4stbPy6cq3jPPqjiChkVvvNKmPGJxWUtg6LnF5kejMRNNU3TGtRBeJgk33yuGBxrMPHi";
    let wif = "KwDiBf89QgGbjEhKnhXJuH7LrciVrZi3qYjgd9M7rFU73sVHnoWn";
    // BIP-93's first test vector.
    let codex32 = "ms10testsxxxxxxxxxxxxxxxxxxxxxxxxxx4nzvca9cmczlw";
    for (secret, said) in [
        (words, "seed's words"),
        (unchecked, "seed's words"),
        (seedqr.as_str(), "SeedQR"),
        (xprv, "private key"),
        (wif, "private key"),
        (codex32, "share of a key"),
    ] {
        let mut app = scanning(Vec::new());
        app.clipboard = true;
        paste(&mut app, secret);
        assert!(note(&app).contains(said), "{secret}: {}", note(&app));
        assert!(app.inbox.is_empty(), "{secret} was filed");
        assert!(app.session.keys.is_empty(), "{secret} loaded a key");
        assert!(app.scan.is_some(), "{secret} closed the scanner");
    }
}

#[test]
fn paste_is_dimmed_where_the_shell_has_no_clipboard() {
    let mut app = scanning(Vec::new());
    assert!(app.scan_pastes());
    app.press(Action::ScanPaste);
    let asked = std::iter::from_fn(|| app.poll_command())
        .any(|c| matches!(c, faraday_shell_api::Command::RequestClipboard { .. }));
    assert!(!asked, "a shell with no clipboard was asked for it");
}

#[test]
fn a_seed_scanner_pastes_only_where_the_shell_has_a_clipboard() {
    // No dimmed Paste on a seed scanner: the stick and the Pi have none
    // (F75).
    for clipboard in [false, true] {
        let mut app = Faraday::new();
        app.clipboard = clipboard;
        // No key loads before the self-test, which the display starts.
        display(&mut app);
        app.press(Action::ScanSeed);
        assert!(app.scan.is_some(), "the seed scanner did not open");
        assert_eq!(app.scan_pastes(), clipboard);
    }
}

#[test]
fn nothing_to_paste_shows_for_two_seconds() {
    let mut app = scanning(Vec::new());
    app.clipboard = true;
    app.event(Event::Tick { now_ms: 1_000 });
    app.press(Action::ScanPaste);
    app.event(Event::ClipboardUnavailable {
        kind: faraday_shell_api::FileKind::Text,
    });
    assert_eq!(state(&app), "Nothing to paste");
    app.event(Event::Tick { now_ms: 2_500 });
    assert_eq!(state(&app), "Nothing to paste");
    app.event(Event::Tick { now_ms: 3_100 });
    assert_ne!(state(&app), "Nothing to paste");
    // An empty clipboard is the same.
    paste(&mut app, "  ");
    assert_eq!(state(&app), "Nothing to paste");
}

fn frame(app: &mut Faraday) {
    app.event(Event::CameraFrame {
        width: 64,
        height: 48,
        luma: vec![120; 64 * 48],
        chroma: None,
    });
}

#[test]
fn a_camera_that_sends_no_picture_is_said_to() {
    let mut app = scanning(Vec::new());
    app.event(Event::Tick { now_ms: 1_000 });
    assert_eq!(state(&app), "Starting the camera");
    app.event(Event::Tick { now_ms: 5_000 });
    assert_eq!(state(&app), "Starting the camera");
    app.event(Event::Tick { now_ms: 7_100 });
    assert_eq!(state(&app), "The camera sends no picture");
    assert!(app.scan_pastes(), "the scanner closed");
    // A picture after all.
    frame(&mut app);
    assert_eq!(state(&app), "Hold the QR in front of the camera");
}

#[test]
fn a_camera_slow_to_its_first_frame_reaches_the_picture() {
    // A webcam whose first frame comes four and a half seconds after
    // it is turned on.
    let mut app = scanning(Vec::new());
    let mut now_ms = 0;
    while now_ms < 4_500 {
        now_ms += 250;
        app.event(Event::Tick { now_ms });
        assert_eq!(state(&app), "Starting the camera", "at {now_ms} ms");
    }
    frame(&mut app);
    for _ in 0..40 {
        now_ms += 250;
        app.event(Event::Tick { now_ms });
        frame(&mut app);
        assert_eq!(
            state(&app),
            "Hold the QR in front of the camera",
            "at {now_ms} ms"
        );
    }
}

#[test]
fn a_scanner_opened_again_waits_afresh_for_the_camera() {
    let mut app = scanning(Vec::new());
    app.event(Event::Tick { now_ms: 1_000 });
    app.event(Event::Tick { now_ms: 7_100 });
    assert_eq!(state(&app), "The camera sends no picture");
    // Cancel, and the scanner again: the camera has its full wait.
    app.press(Action::Cancel);
    app.press(Action::Scan);
    app.event(Event::Tick { now_ms: 8_000 });
    app.event(Event::Tick { now_ms: 13_000 });
    assert_eq!(state(&app), "Starting the camera");
    frame(&mut app);
    assert_eq!(state(&app), "Hold the QR in front of the camera");
}

#[test]
fn each_camera_failure_is_named_and_the_scanner_stays() {
    use faraday_shell_api::CameraFailure;
    for (event, said) in [
        (
            Event::CameraFailed(CameraFailure::Refused),
            "The camera was refused",
        ),
        (
            Event::CameraFailed(CameraFailure::InUse),
            "The camera is in use by another app",
        ),
        (
            Event::CameraFailed(CameraFailure::NotFound),
            "No camera found",
        ),
        (
            Event::CameraUnavailable,
            "No camera, or the camera was refused",
        ),
    ] {
        let mut app = scanning(Vec::new());
        app.event(event);
        assert_eq!(state(&app), said);
        assert!(app.scan_pastes(), "the scanner closed");
        // Its other ways in still work.
        app.clipboard = true;
        paste(&mut app, &kit("spending").descriptor);
        assert!(!app.inbox.is_empty(), "a paste after {said} did not arrive");
    }
}

#[test]
fn a_qr_seen_large_enough_that_never_reads_gets_a_hint() {
    let hint = "Hold it square-on, in focus, with its white margin in the frame";
    let mut app = scanning(Vec::new());
    frame(&mut app);
    app.event(Event::Tick { now_ms: 1_000 });
    seen(&mut app, 60, false);
    app.event(Event::Tick { now_ms: 3_000 });
    seen(&mut app, 60, false);
    assert_ne!(note(&app), hint);
    app.event(Event::Tick { now_ms: 5_100 });
    seen(&mut app, 60, false);
    assert_eq!(note(&app), hint);
    // A QR that reads starts the wait again.
    let mut app = scanning(Vec::new());
    frame(&mut app);
    app.event(Event::Tick { now_ms: 1_000 });
    seen(&mut app, 60, false);
    app.event(Event::Tick { now_ms: 4_000 });
    seen(&mut app, 60, true);
    app.event(Event::Tick { now_ms: 5_100 });
    seen(&mut app, 60, false);
    assert_ne!(note(&app), hint);
    // A QR too fine is said to be, not this.
    let mut app = scanning(Vec::new());
    frame(&mut app);
    for t in [1_000, 3_000, 5_100, 7_000] {
        app.event(Event::Tick { now_ms: t });
        seen(&mut app, 14, false);
    }
    assert!(note(&app).contains("Too fine"), "{}", note(&app));
}

fn display(app: &mut Faraday) {
    app.event(Event::Display(faraday_shell_api::DisplayInfo {
        width: 1366,
        height: 768,
        dpi: 160,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: faraday_shell_api::SecureHardware::None,
        boot: faraday_shell_api::BootState::Unknown,
        memory_mib: None,
    }));
}

#[test]
fn the_scanner_draws_its_state_over_the_picture() {
    let mut app = Faraday::new();
    display(&mut app);
    app.press(Action::Scan);
    let _ = app.frame();
    frame(&mut app);
    let psbt = testkit::unsigned(&kit("spending")).unwrap().to_bytes();
    let (a, _) = ur_frames(&psbt, 2);
    scan(&mut app, &a[1]);
    seen(&mut app, 60, true);
    let _ = app.frame();
    app.event(Event::CameraFailed(faraday_shell_api::CameraFailure::InUse));
    let _ = app.frame();
}

fn asks_for_clipboard(app: &mut Faraday) -> bool {
    std::iter::from_fn(|| app.poll_command())
        .any(|c| matches!(c, faraday_shell_api::Command::RequestClipboard { .. }))
}

#[test]
fn ctrl_v_on_the_open_scanner_pastes_and_elsewhere_does_nothing() {
    let mut app = scanning(Vec::new());
    app.clipboard = true;
    app.event(Event::Key(faraday_shell_api::Key::Paste));
    assert!(asks_for_clipboard(&mut app), "Ctrl+V did not ask");
    // The same refusal of a secret as the Paste tile.
    app.event(Event::Clipboard {
        kind: faraday_shell_api::FileKind::Text,
        text: "xprv9s21ZrQH143K3QTDL4LXw2F7HEK3wJUD2nW2nRk4stbPy6cq3jPPqjiChkVvvNKmPGJxWUtg6LnF5kejMRNNU3TGtRBeJgk33yuGBxrMPHi".into(),
    });
    assert!(app.inbox.is_empty() && app.session.keys.is_empty());
    // With the clipboard absent, nothing is asked.
    let mut app = scanning(Vec::new());
    app.event(Event::Key(faraday_shell_api::Key::Paste));
    assert!(!asks_for_clipboard(&mut app));
    // With the scanner closed, nothing is asked.
    let mut app = Faraday::new();
    app.clipboard = true;
    app.event(Event::Key(faraday_shell_api::Key::Paste));
    assert!(!asks_for_clipboard(&mut app));
}
