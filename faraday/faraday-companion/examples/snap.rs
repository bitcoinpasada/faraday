//! Renders each of the Companion's screens in its states to PNG, for
//! review (`just companion-snap WxH`).
//!
//! ```text
//! cargo run -p faraday-companion --example snap -- WxH[@DPI] OUT_DIR [INSET]
//! ```
//!
//! `INSET` is the bottom strip in pixels a phone's gesture bar takes (63
//! at 1080x2340, as `just snapshots` renders a phone). What the shell
//! would report is made up here: the clock, a PSBT and a description
//! from `faraday-core`'s test kit, the codes the device shows. Each
//! picture is `OUT_DIR/<name>.png`, named for the screen and its state.

use std::path::Path;
use std::process::ExitCode;

use faraday_companion::{Act, Closed, Companion, CompanionEvent};
use faraday_core::testkit;
use faraday_ui::keyboard::OskPress;
use osk_shell_api::{App, BootState, DisplayInfo, Event, SecureHardware, TouchPhase};

const WORDS: &str =
    "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
const ADDRESS: &str = "bc1qcr8te4kr609gcawutmrza0j4xv80jy8z306fyu";
/// BIP-84's account 0 of "abandon … about", as Jade writes it.
const JADE: &str =
    include_str!("../../faraday-core/tests/vectors/interop/sparrow/jade-keystore.txt");
const BITLOCKER: &str = "BitLocker recovery key for DESKTOP-4K2Q, drive C:\n205568-492019-108251-336446-650375-210331-584155-047520";

struct Snap<'a> {
    out: &'a Path,
    size: (u16, u16),
    dpi: u16,
    inset: u16,
}

fn kit(id: &str) -> testkit::Kit {
    testkit::kits()
        .into_iter()
        .find(|k| k.id == id)
        .expect("a test kit")
}

fn psbt() -> Vec<u8> {
    testkit::unsigned(&kit("spending"))
        .expect("a PSBT")
        .to_bytes()
}

fn scanned(c: &mut Companion, text: &str) {
    c.event(Event::Scanned {
        bytes: text.as_bytes().to_vec(),
    });
}

impl Snap<'_> {
    fn app(&self) -> Companion {
        let mut c = Companion::new();
        c.event(Event::Display(DisplayInfo {
            width: self.size.0,
            height: self.size.1,
            dpi: self.dpi,
            inset_bottom: self.inset,
            inset_top: 0,
            buttons: 0,
            camera_fixed: true,
            secure: SecureHardware::None,
            boot: BootState::Unknown,
            memory_mib: None,
        }));
        c.companion_event(CompanionEvent::Time {
            unix_s: 1_791_200_000,
            offset_s: 7200,
        });
        c
    }

    /// A Companion holding a PSBT, a description and a file, kept from
    /// the device's codes.
    fn holding(&self) -> Companion {
        let mut c = self.app();
        c.press(Act::Receive);
        scanned(&mut c, &osk_codec::ur::encode_psbt(&psbt()));
        c.press(Act::ScanAgain);
        scanned(&mut c, &kit("spending").descriptor);
        c.press(Act::ScanAgain);
        let env = faraday_qr::envelope::pack("notes.txt", b"Meet at noon.\n", "file")
            .expect("an envelope");
        scanned(&mut c, std::str::from_utf8(&env).expect("text"));
        c.press(Act::Back);
        c
    }

    fn shot(&self, c: &mut Companion, name: &str) -> Result<(), String> {
        while c.poll_command().is_some() {}
        while c.poll_companion().is_some() {}
        let frame = c.frame();
        let path = self.out.join(format!("{name}.png"));
        let file = std::fs::File::create(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        let mut enc = png::Encoder::new(
            std::io::BufWriter::new(file),
            u32::from(frame.width),
            u32::from(frame.height),
        );
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Eight);
        let mut w = enc.write_header().map_err(|e| e.to_string())?;
        w.write_image_data(frame.rgba).map_err(|e| e.to_string())?;
        println!("{}", path.display());
        Ok(())
    }
}

fn run(s: &Snap) -> Result<(), String> {
    // Home.
    let mut c = s.app();
    s.shot(&mut c, "home-empty")?;
    let mut c = s.holding();
    s.shot(&mut c, "home")?;

    // Scan a Faraday QR code.
    let mut c = s.app();
    c.press(Act::Receive);
    s.shot(&mut c, "receive-waiting")?;
    // A frame with no code in it: the preview, grey.
    let (w, h) = (640u16, 480u16);
    c.event(Event::CameraFrame {
        width: w,
        height: h,
        luma: (0..usize::from(w) * usize::from(h))
            .map(|i| (60 + (i % usize::from(w)) * 120 / usize::from(w)) as u8)
            .collect(),
        chroma: None,
    });
    s.shot(&mut c, "receive-looking")?;
    // The shell's scanner finds a code it has not read yet: outlined.
    c.companion_event(CompanionEvent::Seen {
        width: w,
        height: h,
        corners: [(220, 140), (420, 150), (410, 350), (210, 340)],
        read: false,
    });
    s.shot(&mut c, "receive-found")?;
    let parts = faraday_qr::bbqr::encode('P', &psbt(), 60).ok_or("BBQr")?;
    scanned(&mut c, &parts[1]);
    s.shot(&mut c, "receive-bbqr-parts")?;
    let mut c = s.app();
    c.press(Act::Receive);
    let mut enc = osk_codec::ur::Encoder::psbt(&psbt(), 60).map_err(|e| format!("{e:?}"))?;
    scanned(&mut c, &enc.next_part());
    scanned(&mut c, &enc.next_part());
    s.shot(&mut c, "receive-ur-parts")?;
    c.press(Act::Paste);
    c.event(Event::ClipboardUnavailable {
        kind: osk_shell_api::FileKind::Text,
    });
    s.shot(&mut c, "receive-nothing-to-paste")?;
    let mut c = s.app();
    c.press(Act::Receive);
    c.event(Event::CameraUnavailable);
    s.shot(&mut c, "receive-no-camera")?;

    // What arrives (§5).
    let mut c = s.app();
    c.press(Act::Receive);
    scanned(&mut c, &osk_codec::ur::encode_psbt(&psbt()));
    s.shot(&mut c, "arrived-psbt")?;
    c.press(Act::ScanAgain);
    scanned(&mut c, &osk_codec::ur::encode_psbt(&psbt()));
    s.shot(&mut c, "arrived-again")?;
    c.press(Act::ScanAgain);
    scanned(&mut c, &kit("spending").descriptor);
    s.shot(&mut c, "arrived-description")?;
    c.press(Act::ScanAgain);
    scanned(&mut c, WORDS);
    s.shot(&mut c, "refused-words")?;
    c.press(Act::ScanAgain);
    scanned(
        &mut c,
        "otpauth://totp/Example:alice?secret=JBSWY3DPEHPK3PXP&issuer=Example",
    );
    s.shot(&mut c, "refused-totp")?;
    c.press(Act::ScanAgain);
    c.event(Event::Scanned {
        bytes: b"OSKB\x03".iter().copied().chain([0u8; 60]).collect(),
    });
    s.shot(&mut c, "refused-backup")?;
    c.press(Act::ScanAgain);
    scanned(&mut c, "Meet at the north gate at noon.\nBring the stick.");
    s.shot(&mut c, "shown-text")?;
    c.press(Act::Keep);
    s.shot(&mut c, "shown-text-kept")?;
    c.press(Act::ScanAgain);
    scanned(&mut c, ADDRESS);
    s.shot(&mut c, "shown-address")?;
    c.press(Act::ScanAgain);
    scanned(&mut c, "https://example.com/pay?to=me");
    s.shot(&mut c, "shown-web")?;
    // A guide's address opens the Companion's own page (§4.7).
    c.press(Act::ScanAgain);
    scanned(&mut c, &faraday_learn::guide_address("windows"));
    s.shot(&mut c, "guide-opened")?;

    // What another app shares (§6.3).
    let mut c = s.app();
    c.companion_event(CompanionEvent::SharedIn {
        name: Some("spend.psbt".into()),
        bytes: psbt(),
    });
    s.shot(&mut c, "shared-psbt")?;
    let mut c = s.app();
    c.companion_event(CompanionEvent::SharedIn {
        name: None,
        bytes: WORDS.as_bytes().to_vec(),
    });
    s.shot(&mut c, "shared-refused")?;

    // The unlock (§7).
    let mut c = s.app();
    c.companion_event(CompanionEvent::KeptCounts {
        descriptions: 2,
        keys: 1,
    });
    s.shot(&mut c, "home-locked")?;
    c.press(Act::Descriptions);
    s.shot(&mut c, "unlocking")?;
    c.companion_event(CompanionEvent::KeptClosed {
        why: Closed::NoScreenLock,
    });
    s.shot(&mut c, "closed-no-screen-lock")?;
    c.press(Act::Done);
    c.press(Act::Keys);
    c.companion_event(CompanionEvent::KeptClosed {
        why: Closed::Cancelled,
    });
    s.shot(&mut c, "closed-cancelled")?;
    c.press(Act::Done);
    c.press(Act::Descriptions);
    c.companion_event(CompanionEvent::Kept {
        bytes: b"FCKS\x09".to_vec(),
    });
    s.shot(&mut c, "closed-unreadable")?;

    // Wallet descriptions (§4.5).
    let mut c = s.app();
    c.press(Act::Descriptions);
    c.companion_event(CompanionEvent::Kept { bytes: Vec::new() });
    s.shot(&mut c, "descriptions-empty")?;
    c.press(Act::AddDescription);
    s.shot(&mut c, "descriptions-scanner")?;
    scanned(&mut c, &kit("spending").descriptor);
    s.shot(&mut c, "description")?;
    c.press(Act::Rename);
    for _ in 0..8 {
        c.press(Act::Osk(OskPress::Back));
    }
    c.press(Act::Osk(OskPress::Shift));
    for ch in "Savings".chars() {
        c.press(Act::Osk(OskPress::Char(ch)));
    }
    s.shot(&mut c, "description-rename")?;
    c.press(Act::SaveName);
    c.press(Act::Back);
    c.press(Act::AddDescription);
    scanned(&mut c, JADE);
    c.press(Act::Back);
    s.shot(&mut c, "descriptions")?;
    c.press(Act::DescriptionRecord(1));
    c.press(Act::Delete);
    s.shot(&mut c, "description-delete")?;
    c.press(Act::KeepIt);
    c.press(Act::Back);
    c.press(Act::AddDescription);
    scanned(&mut c, "wpkh(xpub-not-a-key/<0;1>/*)");
    s.shot(&mut c, "description-not-read")?;
    // A description in Received, with Keep in Wallet descriptions.
    let mut c = s.holding();
    c.press(Act::Received);
    c.press(Act::Record(1));
    s.shot(&mut c, "record-description")?;

    // BitLocker keys (§4.6).
    let mut c = s.app();
    c.press(Act::Receive);
    scanned(&mut c, BITLOCKER);
    s.shot(&mut c, "key-unlocking")?;
    c.companion_event(CompanionEvent::Kept { bytes: Vec::new() });
    s.shot(&mut c, "key")?;
    let (x, y) = c.where_offered(Act::Hold).ok_or("Hold to show")?;
    c.event(Event::Touch {
        x,
        y,
        phase: TouchPhase::Down,
    });
    s.shot(&mut c, "key-shown")?;
    c.event(Event::Touch {
        x,
        y,
        phase: TouchPhase::Up,
    });
    c.press(Act::BackInWindows);
    s.shot(&mut c, "key-back-in-windows")?;
    c.press(Act::KeepIt);
    c.press(Act::Delete);
    s.shot(&mut c, "key-delete")?;
    c.press(Act::KeepIt);
    c.press(Act::Back);
    s.shot(&mut c, "home-with-key")?;
    c.press(Act::Keys);
    s.shot(&mut c, "keys")?;

    // Send to Faraday (§4.3).
    let mut c = s.app();
    c.press(Act::Send);
    s.shot(&mut c, "send-empty")?;
    let mut c = s.holding();
    c.press(Act::Send);
    s.shot(&mut c, "send-list")?;
    // Newest first: the file, the description, the PSBT.
    c.press(Act::SendKept(2));
    s.shot(&mut c, "send-psbt")?;
    c.press(Act::Animated);
    c.event(Event::Tick { now_ms: 1000 });
    s.shot(&mut c, "send-psbt-animated")?;
    c.press(Act::Format);
    s.shot(&mut c, "send-psbt-bbqr")?;
    c.press(Act::Rate);
    s.shot(&mut c, "send-psbt-bbqr-rate")?;
    c.press(Act::Back);
    c.press(Act::SendKept(1));
    s.shot(&mut c, "send-description")?;
    c.press(Act::Format);
    c.press(Act::Format);
    s.shot(&mut c, "send-description-parts")?;
    c.press(Act::Back);
    c.press(Act::SendKept(0));
    s.shot(&mut c, "send-file")?;
    c.press(Act::Back);
    c.press(Act::Choose);
    c.event(Event::File {
        kind: osk_shell_api::FileKind::Any,
        bytes: vec![7; 300 * 1024],
    });
    s.shot(&mut c, "send-too-large")?;
    c.press(Act::Done);
    c.press(Act::Choose);
    c.event(Event::File {
        kind: osk_shell_api::FileKind::Any,
        bytes: WORDS.as_bytes().to_vec(),
    });
    s.shot(&mut c, "send-refused")?;
    c.press(Act::Done);
    c.press(Act::Choose);
    c.event(Event::File {
        kind: osk_shell_api::FileKind::Any,
        bytes: (0..40_000u32).map(|i| (i * 7 % 251) as u8).collect(),
    });
    c.event(Event::Tick { now_ms: 2000 });
    s.shot(&mut c, "send-picked-large")?;

    // Received (§4.4).
    let mut c = s.app();
    c.press(Act::Received);
    s.shot(&mut c, "received-empty")?;
    let mut c = s.holding();
    c.press(Act::Received);
    s.shot(&mut c, "received")?;
    c.press(Act::Record(2));
    s.shot(&mut c, "record")?;
    c.press(Act::Save);
    c.event(Event::FileWritten {
        kind: osk_shell_api::FileKind::Psbt,
    });
    s.shot(&mut c, "record-saved")?;
    c.press(Act::Delete);
    s.shot(&mut c, "delete-one")?;
    c.press(Act::KeepIt);
    c.press(Act::Back);
    c.press(Act::DeleteAll);
    s.shot(&mut c, "delete-all")?;
    let mut c = s.app();
    c.companion_event(CompanionEvent::Received {
        bytes: b"FCRS\x09".to_vec(),
    });
    c.press(Act::Received);
    s.shot(&mut c, "received-unreadable")?;
    c.press(Act::DeleteStore);
    s.shot(&mut c, "delete-store")?;

    // Learn and Start from the stick (§4.7).
    let mut c = s.app();
    c.press(Act::Learn);
    s.shot(&mut c, "learn")?;
    c.press(Act::LearnPage(0));
    s.shot(&mut c, "learn-page")?;
    let mut c = s.app();
    c.press(Act::Start);
    s.shot(&mut c, "start")?;
    c.press(Act::StartPage(0));
    s.shot(&mut c, "start-windows")?;
    // A maker's page with its firmware sketches (DECISIONS F18).
    let mut c = s.app();
    c.press(Act::Start);
    c.press(Act::StartPage(faraday_guides::BOOT.len()));
    s.shot(&mut c, "start-maker-dell")?;

    // About (§4.8).
    let mut c = s.app();
    c.press(Act::About);
    s.shot(&mut c, "about-unreported")?;
    c.companion_event(CompanionEvent::System {
        build: "0.1.1 (3)".into(),
        certificate_sha256: [0x5a; 32],
        permissions: vec![
            ("android.permission.CAMERA".into(), true),
            ("android.permission.USE_BIOMETRIC".into(), true),
        ],
    });
    s.shot(&mut c, "about")?;
    Ok(())
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    if !(3..=4).contains(&args.len()) {
        eprintln!("usage: snap WxH[@DPI] OUT_DIR [INSET]");
        return ExitCode::from(2);
    }
    let (size, dpi) = match args[1].split_once('@') {
        Some((s, d)) => (s, d.parse().unwrap_or(160)),
        None => (args[1].as_str(), 160),
    };
    let Some(size) = size
        .split_once('x')
        .and_then(|(w, h)| Some((w.parse().ok()?, h.parse().ok()?)))
    else {
        eprintln!("size must be WxH[@DPI]");
        return ExitCode::from(2);
    };
    let inset = args.get(3).and_then(|i| i.parse().ok()).unwrap_or(0);
    let out = Path::new(&args[2]);
    if let Err(e) = std::fs::create_dir_all(out) {
        eprintln!("{}: {e}", out.display());
        return ExitCode::FAILURE;
    }
    match run(&Snap {
        out,
        size,
        dpi,
        inset,
    }) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}
