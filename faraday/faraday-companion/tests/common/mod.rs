//! What the tests share: a phone, its camera and its clipboard, and the
//! device's own files from `faraday-core`'s test kit.

#![allow(dead_code)]

use faraday_companion::{Act, Companion, CompanionCommand, CompanionEvent, Page};
use faraday_core::testkit;
use osk_codec::QrMatrix;
use osk_shell_api::{App, BootState, Command, DisplayInfo, Event, FileKind, SecureHardware};

/// A phone as `just snapshots` renders one: 1080 x 2340 at 420 dpi.
pub fn phone() -> Companion {
    let mut c = Companion::new();
    c.event(Event::Display(DisplayInfo {
        width: 1080,
        height: 2340,
        dpi: 420,
        inset_bottom: 63,
        inset_top: 0,
        buttons: 0,
        camera_fixed: true,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
    c.companion_event(CompanionEvent::Time {
        unix_s: 1_791_200_000,
        offset_s: 0,
    });
    drain(&mut c);
    c
}

/// Everything the core has asked of the shell since the last drain.
pub fn drain(c: &mut Companion) -> (Vec<Command>, Vec<CompanionCommand>) {
    let mut a = Vec::new();
    while let Some(x) = c.poll_command() {
        a.push(x);
    }
    let mut b = Vec::new();
    while let Some(x) = c.poll_companion() {
        b.push(x);
    }
    (a, b)
}

/// The last store the core handed the shell to keep, if any.
pub fn kept_store(cmds: &[CompanionCommand]) -> Option<Vec<u8>> {
    cmds.iter().rev().find_map(|c| match c {
        CompanionCommand::KeepReceived { bytes } => Some(bytes.clone()),
        _ => None,
    })
}

/// The camera seeing one code, as a frame of luma.
pub fn show(c: &mut Companion, code: &QrMatrix) {
    let (w, h, luma) = code.to_luma(3, osk_codec::qr::QUIET_ZONE);
    c.event(Event::CameraFrame {
        width: w as u16,
        height: h as u16,
        luma,
        chroma: None,
    });
}

/// A code made of `bytes`, in byte mode.
pub fn code_of(bytes: &[u8]) -> QrMatrix {
    osk_codec::qr::encode(osk_codec::Payload::Bytes(bytes), osk_codec::Ecc::Low).expect("a code")
}

/// Paste: the core asks for the clipboard, and the clipboard holds
/// `text`.
pub fn paste(c: &mut Companion, text: &str) {
    c.press(Act::Paste);
    let (cmds, _) = drain(c);
    assert!(
        cmds.contains(&Command::RequestClipboard {
            kind: FileKind::Text
        }),
        "Paste asked for no clipboard"
    );
    c.event(Event::Clipboard {
        kind: FileKind::Text,
        text: text.to_string(),
    });
}

/// The phone on Receive from Faraday.
pub fn receiving() -> Companion {
    let mut c = phone();
    c.press(Act::Receive);
    assert_eq!(c.page, Page::Receive);
    drain(&mut c);
    c
}

pub fn kit(id: &str) -> testkit::Kit {
    testkit::kits()
        .into_iter()
        .find(|k| k.id == id)
        .expect("a test kit")
}

/// The test kit's unsigned spend from its spending wallet.
pub fn psbt() -> Vec<u8> {
    testkit::unsigned(&kit("spending"))
        .expect("a PSBT")
        .to_bytes()
}

/// The spending wallet's descriptor.
pub fn description() -> String {
    kit("spending").descriptor
}

/// A file under the repository's root.
pub fn repo(path: &str) -> Vec<u8> {
    let at = format!("{}/../../{path}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read(&at).unwrap_or_else(|e| panic!("{at}: {e}"))
}

pub fn json(path: &str) -> serde_json::Value {
    serde_json::from_slice(&repo(path)).expect(path)
}
