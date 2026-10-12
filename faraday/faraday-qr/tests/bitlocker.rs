//! The BitLocker recovery key's text (`docs/HUB.md` §6.2,
//! `docs/COMPANION.md` §4.6): what the hub app's QR code carries is what
//! the Companion reads back, and nothing else reads as a key.

use faraday_qr::bitlocker::{KeyText, RecoveryKey};

const KEY: &str = "205568-492019-108251-336446-650375-210331-584155-047520";

fn text() -> KeyText {
    KeyText {
        computer: "DESKTOP-4K2Q".into(),
        drive: "C:".into(),
        key: RecoveryKey::parse(KEY).expect("a key"),
    }
}

#[test]
fn the_text_names_the_computer_and_the_drive_and_reads_back() {
    let t = text();
    let written = t.write().expect("written");
    assert_eq!(
        written.as_str(),
        format!("BitLocker recovery key for DESKTOP-4K2Q, drive C:\n{KEY}")
    );
    assert_eq!(KeyText::read(&written), Some(t.clone()));
    // As a phone's notes may keep it.
    let crlf = written.replace('\n', "\r\n") + "\r\n";
    assert_eq!(KeyText::read(&crlf), Some(t));
}

#[test]
fn the_key_is_eight_groups_of_six_digits() {
    let k = RecoveryKey::parse(KEY).expect("a key");
    assert_eq!(k.group(0), "205568");
    assert_eq!(k.group(7), "047520");
    for bad in [
        "",
        "205568-492019-108251-336446-650375-210331-584155",
        "205568-492019-108251-336446-650375-210331-584155-04752",
        "205568-492019-108251-336446-650375-210331-584155-04752x",
        "205568 492019 108251 336446 650375 210331 584155 047520",
        "205568-492019-108251-336446-650375-210331-584155-047520-1",
    ] {
        assert!(RecoveryKey::parse(bad).is_none(), "{bad:?}");
    }
    assert_eq!(format!("{k:?}"), "RecoveryKey(..)");
}

#[test]
fn other_text_does_not_read_as_a_key() {
    for t in [
        KEY.to_string(),
        format!("BitLocker recovery key for X, drive C:\n{KEY}\nmore"),
        format!("recovery key for X, drive C:\n{KEY}"),
        format!("BitLocker recovery key for X drive C:\n{KEY}"),
        format!("BitLocker recovery key for , drive C:\n{KEY}"),
    ] {
        assert!(KeyText::read(&t).is_none(), "{t:?}");
    }
    let mut odd = text();
    odd.computer = "A, drive B".into();
    assert!(odd.write().is_none());
}
