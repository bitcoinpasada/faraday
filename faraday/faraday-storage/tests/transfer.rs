//! Transfer, the desktop app as the device's QR link: a file in Downloads
//! sent as codes reads back on the device's scanner as the same bytes, a
//! file over 256 KiB is refused, and what the device sends as codes is
//! saved into Downloads once, under its own name or `received-N`, never
//! over a file already there, and never into the Inbox. The device's app
//! has no Transfer.
//!
//! This crate's `serve` is the desktop shell's storage, with a folder in
//! the system's temporary directory standing for Downloads.

use std::fs;
use std::ops::Deref;
use std::path::{Path, PathBuf};

use faraday_core::wallet::{FileKind, Session};
use faraday_core::{Action, Faraday, Screen, StickInfo, StorageCommand, StorageEvent, testkit};
use faraday_storage::{Boxes, Host, serve};
use osk_shell_api::{App, BootState, DisplayInfo, Event, SecureHardware};

/// A test's own folder standing for Downloads; removed when it drops.
struct Downloads(PathBuf);

impl Deref for Downloads {
    type Target = Path;
    fn deref(&self) -> &Path {
        &self.0
    }
}

impl Drop for Downloads {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// A fresh folder standing for Downloads.
fn downloads(name: &str) -> Downloads {
    let dir = std::env::temp_dir().join(format!("faraday-transfer-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    Downloads(dir)
}

fn host(dir: &std::path::Path) -> Host {
    Host {
        print: dir.join("print"),
        downloads: dir.to_path_buf(),
        opens: false,
    }
}

fn boxes() -> Boxes {
    Boxes::Memory {
        inbox: Vec::new(),
        outbox: Vec::new(),
        kept: Vec::new(),
    }
}

fn display(app: &mut Faraday, width: u16, height: u16, dpi: u16) {
    app.event(Event::Display(DisplayInfo {
        width,
        height,
        dpi,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
}

/// The desktop's app: online, starting on testnet.
fn desktop() -> Faraday {
    let mut app = Faraday::new();
    app.online = true;
    app.session = Session::on(testkit::NET);
    app
}

fn kit(id: &str) -> testkit::Kit {
    testkit::kits().into_iter().find(|k| k.id == id).unwrap()
}

/// Bytes no compression shortens, the same each run.
fn noise(len: usize) -> Vec<u8> {
    let mut x: u32 = 0x9e37_79b9;
    (0..len)
        .map(|_| {
            x ^= x << 13;
            x ^= x >> 17;
            x ^= x << 5;
            x as u8
        })
        .collect()
}

/// What a camera reads off one code.
fn read(m: &osk_codec::qr::QrMatrix) -> Vec<u8> {
    let png = osk_codec::png::qr_png(m, 3);
    let mut read = faraday_files::qr_in_png(&png).expect("a picture");
    assert_eq!(read.len(), 1, "one code a frame");
    read.remove(0)
}

/// What a camera reads off each code the QR sheet shows, in turn.
fn codes(app: &Faraday) -> Vec<Vec<u8>> {
    let q = app.qr.as_ref().expect("no codes shown");
    q.frames.iter().map(read).collect()
}

/// The file Downloads lists under `name`, sent from Transfer.
fn send(app: &mut Faraday, boxes: &mut Boxes, host: &Host, name: &str) {
    app.storage(host.files());
    let i = app
        .transfer
        .files
        .iter()
        .position(|(n, _)| n == name)
        .unwrap_or_else(|| panic!("{name} not listed: {:?}", app.transfer.files));
    app.press(Action::TransferSend(i));
    serve(app, boxes, Some(host));
}

/// The device's Files scanner reading the codes `app` shows, in turn,
/// until something lands in its Inbox.
fn device_reads(app: &Faraday) -> faraday_core::Item {
    let q = app.qr.as_ref().expect("no codes shown");
    let mut device = Faraday::new();
    device.press(Action::Scan);
    for m in &q.frames {
        device.event(Event::Scanned { bytes: read(m) });
        if let Some(item) = device.inbox.last() {
            return item.clone();
        }
    }
    panic!(
        "the device read nothing whole: {:?}",
        device.scan.as_ref().and_then(|s| s.note.clone())
    );
}

#[test]
fn a_psbt_a_descriptor_and_a_100_kib_file_read_back_on_the_device_as_the_same_bytes() {
    let dir = downloads("send");
    let psbt = testkit::unsigned(&kit("spending")).unwrap().to_bytes();
    let descriptor = kit("spending").descriptor;
    let binary = noise(100 * 1024);
    fs::write(dir.join("spending-unsigned.psbt"), &psbt).unwrap();
    fs::write(dir.join("spending-wallet.txt"), &descriptor).unwrap();
    fs::write(dir.join("notes.bin"), &binary).unwrap();
    let host = host(&dir);
    let mut boxes = boxes();
    let mut app = desktop();
    app.press(Action::Nav(Screen::Transfer));
    assert_eq!(app.screen, Screen::Transfer);

    send(&mut app, &mut boxes, &host, "spending-unsigned.psbt");
    let item = device_reads(&app);
    assert_eq!(item.kind, FileKind::Psbt);
    assert_eq!(item.bytes, psbt);
    app.press(Action::Cancel);

    send(&mut app, &mut boxes, &host, "spending-wallet.txt");
    let item = device_reads(&app);
    assert_eq!(item.kind, FileKind::Wallet);
    assert_eq!(item.bytes, descriptor.as_bytes());
    app.press(Action::Cancel);

    // In the file envelope, under its own name.
    send(&mut app, &mut boxes, &host, "notes.bin");
    let item = device_reads(&app);
    assert_eq!(item.name, "notes.bin");
    assert_eq!(item.bytes, binary);
}

#[test]
fn a_file_over_256_kib_is_refused_unread() {
    let dir = downloads("large");
    fs::write(dir.join("photo.jpg"), noise(300 * 1024)).unwrap();
    let host = host(&dir);
    let mut app = desktop();
    app.press(Action::Nav(Screen::Transfer));
    app.storage(host.files());
    app.press(Action::TransferSend(0));
    assert_eq!(
        app.transfer.refused.as_deref(),
        Some("Larger than 256 KiB: carry it on a stick")
    );
    assert!(app.qr.is_none());
    assert!(
        !std::iter::from_fn(|| app.poll_storage())
            .any(|c| matches!(c, StorageCommand::ReadHost { .. })),
        "the file was read"
    );
}

/// What the device's Files shows as codes for each of `files`.
fn device_codes(files: &[(&str, Vec<u8>)]) -> Vec<Vec<Vec<u8>>> {
    let mut device = Faraday::new();
    device.storage(StorageEvent::Restored {
        inbox: Vec::new(),
        outbox: files
            .iter()
            .map(|(n, b)| (n.to_string(), b.clone()))
            .collect(),
        kept: Vec::new(),
    });
    (0..files.len())
        .map(|i| {
            device.press(Action::QrOutbox(i));
            let c = codes(&device);
            device.press(Action::Cancel);
            c
        })
        .collect()
}

/// Feeds `codes` to Receive, the shell answering after each.
fn receive(app: &mut Faraday, boxes: &mut Boxes, host: &Host, codes: &[Vec<u8>]) {
    for c in codes {
        app.event(Event::Scanned { bytes: c.clone() });
        serve(app, boxes, Some(host));
    }
}

fn listed(dir: &std::path::Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(dir)
        .unwrap()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

#[test]
fn each_file_received_is_saved_once_and_a_second_of_the_same_name_gets_2() {
    let dir = downloads("receive");
    let psbt = testkit::unsigned(&kit("spending")).unwrap().to_bytes();
    let descriptor = kit("spending").descriptor;
    let binary = noise(2048);
    let sent = device_codes(&[
        ("spending-signed.psbt", psbt.clone()),
        ("spending-wallet.txt", descriptor.clone().into_bytes()),
        ("notes.bin", binary.clone()),
    ]);
    let host = host(&dir);
    let mut boxes = boxes();
    let mut app = desktop();
    app.press(Action::Nav(Screen::Transfer));
    app.press(Action::TransferReceive);
    for codes in &sent {
        // The device's animated code comes round twice before the camera
        // is taken away: one file each.
        receive(&mut app, &mut boxes, &host, codes);
        receive(&mut app, &mut boxes, &host, codes);
    }
    assert_eq!(
        listed(&dir),
        ["notes.bin", "received-1.psbt", "received-2.txt"]
    );
    assert_eq!(fs::read(dir.join("received-1.psbt")).unwrap(), psbt);
    assert_eq!(
        fs::read(dir.join("received-2.txt")).unwrap(),
        descriptor.as_bytes()
    );
    assert_eq!(fs::read(dir.join("notes.bin")).unwrap(), binary);
    let saved: Vec<&str> = app.transfer.saved.iter().map(|(l, _)| l.as_str()).collect();
    let path = |n: &str| format!("Saved {}", dir.join(n).display());
    assert_eq!(
        saved,
        [
            path("received-1.psbt"),
            path("received-2.txt"),
            path("notes.bin")
        ]
    );
    // The camera is still on for the next.
    assert_eq!(app.sheet, Some(faraday_core::Sheet::Scan));

    // Received again: a new file beside the first.
    app.press(Action::Cancel);
    app.press(Action::TransferReceive);
    receive(&mut app, &mut boxes, &host, &sent[2]);
    assert_eq!(fs::read(dir.join("notes (2).bin")).unwrap(), binary);
    assert_eq!(fs::read(dir.join("notes.bin")).unwrap(), binary);
}

#[test]
fn a_seeds_words_received_are_saved_and_said_to_hold_a_secret() {
    let dir = downloads("secret");
    let words = testkit::test_words(testkit::TEST_SEEDS[0].0);
    let env = faraday_qr::envelope::pack("seed-words.txt", words.as_bytes(), "file").unwrap();
    let parts: Vec<Vec<u8>> = faraday_qr::bbqr::encode('J', &env, 160)
        .unwrap()
        .into_iter()
        .map(String::into_bytes)
        .collect();
    let host = host(&dir);
    let mut boxes = boxes();
    let mut app = desktop();
    app.press(Action::Nav(Screen::Transfer));
    app.press(Action::TransferReceive);
    receive(&mut app, &mut boxes, &host, &parts);
    let path = dir.join("seed-words.txt");
    assert_eq!(fs::read_to_string(&path).unwrap(), words);
    let said = format!("Saved {}: this file holds a secret", path.display());
    assert_eq!(
        app.scan.as_ref().and_then(|s| s.note.clone()),
        Some(said.clone())
    );
    assert_eq!(app.transfer.saved.last(), Some(&(said, true)));
}

#[test]
fn receive_puts_nothing_in_the_inbox() {
    let dir = downloads("inbox");
    let psbt = testkit::unsigned(&kit("spending")).unwrap().to_bytes();
    let sent = device_codes(&[("spending-signed.psbt", psbt)]);
    let host = host(&dir);
    let mut boxes = boxes();
    let mut app = desktop();
    app.press(Action::Nav(Screen::Transfer));
    app.press(Action::TransferReceive);
    receive(&mut app, &mut boxes, &host, &sent[0]);
    app.press(Action::Cancel);
    assert_eq!(listed(&dir), ["received-1.psbt"]);
    assert!(app.inbox.is_empty());
    let Boxes::Memory { inbox, .. } = &boxes else {
        unreachable!()
    };
    assert!(inbox.is_empty());
}

#[test]
fn a_file_dropped_on_the_window_is_sent() {
    let dir = downloads("drop");
    let path = dir.join("elsewhere.psbt");
    let psbt = testkit::unsigned(&kit("spending")).unwrap().to_bytes();
    fs::write(&path, &psbt).unwrap();
    let target = downloads("drop-downloads");
    let host = host(&target);
    let mut boxes = boxes();
    let mut app = desktop();
    app.storage(StorageEvent::Dropped {
        path: path.display().to_string(),
        size: psbt.len() as u64,
    });
    serve(&mut app, &mut boxes, Some(&host));
    assert_eq!(app.screen, Screen::Transfer);
    assert_eq!(device_reads(&app).bytes, psbt);
}

#[test]
fn the_camera_opens_for_receive_with_a_stick_attached() {
    let mut app = desktop();
    display(&mut app, 1280, 800, 160);
    app.storage(StorageEvent::Sticks(vec![StickInfo {
        id: "/sticks/BLANK".into(),
        label: "BLANK".into(),
        boot: false,
        files: Vec::new(),
    }]));
    app.press(Action::Nav(Screen::Transfer));
    app.press(Action::TransferReceive);
    assert_eq!(app.sheet, Some(faraday_core::Sheet::Scan));
}

#[test]
fn transfer_is_offered_only_in_the_online_app() {
    let dir = downloads("offline");
    let path = dir.join("a.psbt");
    fs::write(&path, b"x").unwrap();
    // A small panel tall enough that Home's last tile is on screen.
    for (w, h, dpi) in [(1280, 800, 160), (480, 1600, 286)] {
        let mut app = desktop();
        display(&mut app, w, h, dpi);
        let _ = app.frame();
        assert!(app.offers(Action::Nav(Screen::Transfer)), "{w}x{h}");

        let mut device = Faraday::new();
        display(&mut device, w, h, dpi);
        let _ = device.frame();
        assert!(!device.offers(Action::Nav(Screen::Transfer)), "{w}x{h}");
        device.press(Action::Nav(Screen::Transfer));
        assert_ne!(device.screen, Screen::Transfer);
        device.press(Action::TransferReceive);
        assert!(device.scan.is_none() && device.sheet.is_none());
        device.storage(StorageEvent::Dropped {
            path: path.display().to_string(),
            size: 1,
        });
        assert_ne!(device.screen, Screen::Transfer);
        assert!(
            !std::iter::from_fn(|| device.poll_storage())
                .any(|c| matches!(c, StorageCommand::ReadHost { .. })),
            "the device read a file of the computer's"
        );
    }
}
