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

/// Whether the screen on show draws `text`, a line that wraps read
/// across its lines.
pub fn shows(c: &mut Companion, text: &str) -> bool {
    c.texts().join(" ").contains(text)
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

/// The camera seeing one code: the frame of luma for the preview, and
/// what the shell's scanner reads in it, as the shell hands both over.
pub fn show(c: &mut Companion, code: &QrMatrix) {
    let (w, h, luma) = code.to_luma(3, osk_codec::qr::QUIET_ZONE);
    let found = osk_codec::decode_luma(w, h, &luma);
    c.event(Event::CameraFrame {
        width: w as u16,
        height: h as u16,
        luma,
        chroma: None,
    });
    for d in found {
        c.event(Event::Scanned { bytes: d.bytes });
    }
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

/// The phone on Scan a Faraday QR code.
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

/// Whether the core asked the shell for the phone's unlock.
pub fn asked_unlock(cmds: &[CompanionCommand]) -> bool {
    cmds.iter()
        .any(|c| matches!(c, CompanionCommand::UnlockKept { .. }))
}

/// The last kept store the core handed the shell, with its two counts.
pub fn kept_kept(cmds: &[CompanionCommand]) -> Option<(Vec<u8>, u32, u32)> {
    cmds.iter().rev().find_map(|c| match c {
        CompanionCommand::KeepKept {
            bytes,
            descriptions,
            keys,
        } => Some((bytes.clone(), *descriptions, *keys)),
        _ => None,
    })
}

/// The person gives the phone's unlock the core asked for, and the
/// shell hands over the kept store `bytes` (none the first time).
/// Returns what the core asked for before the answer.
pub fn unlock(c: &mut Companion, bytes: &[u8]) -> Vec<CompanionCommand> {
    let (_, ours) = drain(c);
    assert!(asked_unlock(&ours), "no unlock asked: {ours:?}");
    c.companion_event(CompanionEvent::Kept {
        bytes: bytes.to_vec(),
    });
    ours
}

/// The phone on Wallet descriptions, the kept store opened with `bytes`.
pub fn on_descriptions(bytes: &[u8]) -> Companion {
    let mut c = phone();
    c.press(Act::Descriptions);
    unlock(&mut c, bytes);
    assert_eq!(c.page, Page::Descriptions);
    drain(&mut c);
    c
}

/// Every string on the screen run together without spaces, so a value
/// drawn in groups or wrapped across lines reads whole.
pub fn screen_joined(c: &mut Companion) -> String {
    c.texts().concat().replace(' ', "")
}

/// A file under `faraday-core`'s copies of other wallets' files
/// (`docs/INTEROP.md`).
pub fn interop(path: &str) -> String {
    String::from_utf8(repo(&format!(
        "faraday/faraday-core/tests/vectors/interop/{path}"
    )))
    .expect("text")
}

/// The hub app's text for a BitLocker recovery key.
pub fn hub_key() -> faraday_qr::bitlocker::KeyText {
    faraday_qr::bitlocker::KeyText {
        computer: "DESKTOP-4K2Q".into(),
        drive: "C:".into(),
        key: faraday_qr::bitlocker::RecoveryKey::parse(
            "205568-492019-108251-336446-650375-210331-584155-047520",
        )
        .expect("a key"),
    }
}
