//! A stick's pictures are read for their QR codes off the shell's
//! thread, one at a time (`docs/SPEED.md` §9): a pass of the shell's
//! storage returns while a photograph is still being read, the boot
//! stick's sheet, Import pressed, is drawn meanwhile and says the
//! pictures are being read,
//! and each picture's codes come in on a later pass.

use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use faraday_codec::qr::{self, Ecc, Payload, QUIET_ZONE};
use faraday_core::boot_import::ImportAction;
use faraday_core::{Action, Faraday, Sheet, StorageEvent, testkit};
use faraday_shell_api::App;
use faraday_storage::{Boxes, Dirs, serve_with, stick_info};

/// A test's own folder standing for the boot stick; removed when it drops.
struct Stick(PathBuf);

impl Drop for Stick {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// A grey picture of `w` × `h` pixels of noise, the same each run: a
/// photograph with no code in it.
fn photo(w: usize, h: usize) -> Vec<u8> {
    let mut x: u32 = 0x9e37_79b9;
    let grey: Vec<u8> = (0..w * h)
        .map(|i| {
            x ^= x << 13;
            x ^= x >> 17;
            x ^= x << 5;
            ((i % w) * 160 / w + (x % 64) as usize) as u8
        })
        .collect();
    faraday_codec::png::grey_png(w, h, &grey)
}

/// A picture of one code holding `text`.
fn code(text: &str) -> Vec<u8> {
    let c = testkit::compute();
    let m = qr::encode(&c, Payload::Bytes(text.as_bytes()), Ecc::Low).expect("a code");
    let (w, h, px) = m.to_luma(4, QUIET_ZONE);
    faraday_codec::png::grey_png(w, h, &px)
}

#[test]
fn a_sticks_pictures_come_in_while_its_sheet_is_drawn() {
    let stick =
        Stick(std::env::temp_dir().join(format!("faraday-pictures-{}", std::process::id())));
    let _ = fs::remove_dir_all(&stick.0);
    fs::create_dir_all(&stick.0).unwrap();
    // A photograph first, then a wallet's descriptor as a code.
    fs::write(stick.0.join("a-photo.png"), photo(3000, 2000)).unwrap();
    let savings = testkit::kits()
        .into_iter()
        .find(|k| k.id == "savings")
        .expect("the Savings test wallet");
    fs::write(stick.0.join("b-wallet.png"), code(&savings.descriptor)).unwrap();

    let mut app: Faraday = testkit::started();
    app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    let _ = app.frame();
    app.storage(StorageEvent::Sticks(vec![stick_info(
        &stick.0, "BOOT", true,
    )]));
    let mut boxes = Boxes::Memory {
        inbox: Vec::new(),
        outbox: Vec::new(),
        kept: Vec::new(),
    };
    let mut dirs = Dirs::default();
    serve_with(&mut app, &mut boxes, None, &mut dirs);
    let _ = app.frame();
    assert_eq!(
        app.sheet,
        Some(Sheet::Import),
        "the boot stick's sheet is up"
    );
    // Import reads them (F130).
    app.press(Action::Import(ImportAction::Go));
    serve_with(&mut app, &mut boxes, None, &mut dirs);
    let _ = app.frame();
    let count = app.import_count().expect("the sheet counts");
    assert!(
        count.reading >= 1,
        "the pass returned with the photograph still being read: {count:?}"
    );
    assert!(
        app.drawn_texts().iter().any(|t| t.contains("files read")),
        "the sheet does not say the files are being read"
    );

    // Frames go on being drawn while the pictures are read.
    let until = Instant::now() + Duration::from_secs(60);
    let mut frames = 0;
    while app.import_count().is_some_and(|c| c.reading > 0) {
        assert!(Instant::now() < until, "the pictures were never read");
        std::thread::sleep(Duration::from_millis(5));
        serve_with(&mut app, &mut boxes, None, &mut dirs);
        let _ = app.frame();
        frames += 1;
    }
    assert!(frames > 1, "frames were drawn while the pictures were read");
    // The code's wallet came in; the photograph held none.
    assert_eq!(app.session.wallets.len(), 1, "the code's wallet is loaded");
    assert!(app.import().is_none(), "the import is not over");
}
