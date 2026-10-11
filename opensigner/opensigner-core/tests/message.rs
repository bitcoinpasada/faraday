//! Signing a message and checking a signed one through the shell
//! contract (UX.md F7 and G4): what the screens show, what the signature
//! is checked against, and what the saved file holds.

mod common;

use common::{ABANDON, Harness, PANEL};
use opensigner_core::message::{self, Stage};
use opensigner_core::{ScreenKind, ids};
use osk_bip::bip39::{Language, Mnemonic};
use osk_bip::keys::{MasterKey, Network, ScriptType};
use osk_psbt::bitcoin::address::{Address, NetworkUnchecked};
use osk_psbt::message::{Checked, Format};
use osk_shell_api::{App, Command, Event, FileKind, TouchPhase};

/// A message with a blank line and an indent in it.
const MESSAGE: &str = include_str!("../../../tools/vectors/message.txt");
/// One this device signed with the test seed.
const SIGNED: &str = include_str!("../../../tools/vectors/signed-message.txt");

fn master() -> MasterKey {
    let m = Mnemonic::parse(Language::English, &ABANDON.join(" ")).expect("test mnemonic");
    MasterKey::from_seed(&m.to_seed(b"").expect("ascii"), Network::Mainnet)
}

/// The first receive address of the test seed for `script`.
fn first_address(script: ScriptType) -> String {
    master()
        .account_xpub(script, 0)
        .expect("account")
        .address(false, 0)
        .expect("address")
        .to_string()
}

fn checked(address: &str, message: &str, signature: &str) -> Result<Checked, ()> {
    let address = address
        .parse::<Address<NetworkUnchecked>>()
        .expect("an address")
        .require_network(Network::Mainnet.into())
        .expect("mainnet");
    osk_psbt::message::verify(&address, message, signature).map_err(|_| ())
}

/// A device with the test seed loaded, on the message screen over
/// `text`, which arrived through the scanner's file channel.
fn reading(text: &str) -> Harness {
    let mut h = Harness::new(PANEL);
    h.start_load(&ABANDON);
    h.finish_load(None);
    h.open_single_sig(0);
    h.tap(ids::WALLET_SIGN_MESSAGE);
    h.tap(ids::SCAN_FILE);
    h.send(Event::File {
        kind: FileKind::Any,
        bytes: text.as_bytes().to_vec(),
    });
    h
}

/// The bytes "Save to file" handed the shell. The harness's own tap
/// drops the commands it caused, so this one keeps them.
fn saved(h: &mut Harness) -> (FileKind, String, Vec<u8>) {
    let (x, y) = h.center(ids::MSG_SAVE);
    for phase in [TouchPhase::Down, TouchPhase::Up] {
        h.app.event(Event::Touch { x, y, phase });
    }
    h.drain()
        .into_iter()
        .find_map(|c| match c {
            Command::WriteFile {
                kind,
                name_hint,
                bytes,
            } => Some((kind, name_hint, bytes)),
            _ => None,
        })
        .expect("a write")
}

#[test]
fn a_message_read_from_a_file_is_shown_whole() {
    let h = reading(MESSAGE);
    assert_eq!(h.app.screen(), ScreenKind::SignMessage);
    let texts = h.app.texts();
    for line in MESSAGE.lines().filter(|l| !l.trim().is_empty()) {
        assert!(
            texts.iter().any(|t| t == line),
            "the message is missing {line:?}: {texts:?}"
        );
    }
    // The address it will be signed for is on the screen, and it is the
    // first receive address of the script type the row names.
    let address = first_address(ScriptType::NativeSegwit);
    assert_eq!(h.app.message_address_shown().as_deref(), Some(&*address));
}

#[test]
fn the_signature_verifies_for_the_address_and_format_the_screen_named() {
    for (script, row, format) in [
        (ScriptType::NativeSegwit, 2usize, Format::Bip137),
        (ScriptType::Taproot, 3, Format::Bip322),
        (ScriptType::Legacy, 0, Format::Bip137),
    ] {
        let mut h = reading(MESSAGE);
        h.tap(ids::MSG_SCRIPT);
        h.choose(ids::at(ids::PICK_BASE, row), ids::PICK_CONTINUE);
        h.tap(ids::MSG_CONTINUE);
        assert_eq!(
            h.app.signing_message().map(message::SignMessage::stage),
            Some(Stage::Confirm)
        );
        h.hold(ids::MSG_HOLD);
        let flow = h.app.signing_message().expect("the flow");
        assert_eq!(flow.stage(), Stage::Result);
        let out = flow.outcome().expect("a signature");
        assert_eq!(out.address, first_address(script), "{script}");
        assert_eq!(out.format, format, "{script}");
        assert_eq!(
            checked(&out.address, flow.text(), &out.signature),
            Ok(Checked { format, script }),
            "{script}"
        );
    }
}

#[test]
fn a_taproot_address_offers_no_format_choice() {
    let mut h = reading(MESSAGE);
    assert!(
        h.app.rect_of(ids::MSG_FORMAT).is_some(),
        "a native segwit address has two formats to choose between"
    );
    h.tap(ids::MSG_SCRIPT);
    h.choose(ids::at(ids::PICK_BASE, 3), ids::PICK_CONTINUE);
    assert!(
        h.app.rect_of(ids::MSG_FORMAT).is_none(),
        "taproot has one form, so there is nothing to choose"
    );
    h.tap(ids::MSG_CONTINUE);
    h.hold(ids::MSG_HOLD);
    assert_eq!(
        h.app
            .signing_message()
            .and_then(message::SignMessage::outcome)
            .map(|o| o.format),
        Some(Format::Bip322)
    );
}

#[test]
fn the_saved_file_is_the_three_line_form() {
    let mut h = reading(MESSAGE);
    h.tap(ids::MSG_CONTINUE);
    h.hold(ids::MSG_HOLD);
    let (kind, name_hint, bytes) = saved(&mut h);
    assert_eq!(kind, FileKind::Any);
    assert_eq!(name_hint, "signature.txt");
    let text = String::from_utf8(bytes).expect("text");
    let signed = osk_psbt::message::parse_signed(&text).expect("three lines");
    let out = h
        .app
        .signing_message()
        .and_then(message::SignMessage::outcome)
        .expect("a signature");
    assert_eq!(signed.address, out.address);
    assert_eq!(signed.signature, out.signature);
    assert_eq!(signed.message, MESSAGE.trim());
}

#[test]
fn a_message_with_newlines_survives_the_round_trip() {
    let text = "Line one\n\n  line three, indented\nline four";
    let mut h = reading(text);
    h.tap(ids::MSG_CONTINUE);
    h.hold(ids::MSG_HOLD);
    let (_, _, bytes) = saved(&mut h);
    let file = String::from_utf8(bytes).expect("text");
    let signed = osk_psbt::message::parse_signed(&file).expect("three lines");
    assert_eq!(signed.message, text);
    assert_eq!(
        checked(&signed.address, &signed.message, &signed.signature),
        Ok(Checked {
            format: Format::Bip137,
            script: ScriptType::NativeSegwit,
        })
    );

    // And the device reads its own file back as valid.
    let mut h = Harness::new(PANEL);
    h.tap(ids::at(ids::HOME_TILE_BASE, 3));
    h.tap(ids::MSG_CHECK_ROW);
    h.tap(ids::SCAN_FILE);
    h.send(Event::File {
        kind: FileKind::Any,
        bytes: file.into_bytes(),
    });
    let check = h.app.checked_message().expect("an answer");
    assert!(check.checked().is_some(), "the device's own file is valid");
    assert_eq!(check.message(), text);
}

#[test]
fn a_signed_message_is_valid_and_one_changed_character_is_not() {
    let mut h = Harness::new(PANEL);
    // Checking is keyless: nothing is loaded here.
    h.tap(ids::at(ids::HOME_TILE_BASE, 3));
    h.tap(ids::MSG_CHECK_ROW);
    h.tap(ids::SCAN_FILE);
    h.send(Event::File {
        kind: FileKind::Any,
        bytes: SIGNED.as_bytes().to_vec(),
    });
    assert_eq!(h.app.screen(), ScreenKind::CheckedMessage);
    let check = h.app.checked_message().expect("an answer");
    assert_eq!(
        check.checked(),
        Some(Checked {
            format: Format::Bip137,
            script: ScriptType::NativeSegwit,
        })
    );
    assert_eq!(check.address(), first_address(ScriptType::NativeSegwit));
    // The message is behind the row that states its length.
    let message = String::from(check.message());
    h.tap(ids::MSG_READ);
    let texts = h.app.texts();
    let first = message.lines().next().expect("a line");
    assert!(texts.iter().any(|t| t == first), "{texts:?}");

    let tampered = SIGNED.replacen("I hold", "I held", 1);
    assert_ne!(tampered, SIGNED, "the fixture holds the text that changed");
    let mut h = Harness::new(PANEL);
    h.tap(ids::at(ids::HOME_TILE_BASE, 3));
    h.tap(ids::MSG_CHECK_ROW);
    h.tap(ids::SCAN_FILE);
    h.send(Event::File {
        kind: FileKind::Any,
        bytes: tampered.into_bytes(),
    });
    assert_eq!(
        h.app.checked_message().and_then(|c| c.checked()),
        None,
        "a message with one character changed is not signed"
    );
}

#[test]
fn a_text_that_is_not_three_lines_is_the_scanners_error() {
    let mut h = Harness::new(PANEL);
    h.tap(ids::at(ids::HOME_TILE_BASE, 3));
    h.tap(ids::MSG_CHECK_ROW);
    h.tap(ids::SCAN_FILE);
    h.send(Event::File {
        kind: FileKind::Any,
        bytes: b"just an address\n".to_vec(),
    });
    assert_eq!(h.app.screen(), ScreenKind::Scan);
    assert!(h.app.scan_error().is_some(), "the scanner says why");
    assert!(h.app.checked_message().is_none());
}

/// A wipe or a lock takes the message with it, as it takes a PSBT
/// (UX.md I1, C8).
#[test]
fn a_lock_drops_the_message() {
    let mut h = reading(MESSAGE);
    h.tap(ids::MSG_CONTINUE);
    h.hold(ids::MSG_HOLD);
    assert!(h.app.signing_message().is_some());
    // The auto-lock, since the status line's lock is on Home.
    h.tick(h.app.lock_after_ms() + 1_000);
    assert!(h.app.is_locked());
    h.unlock(common::PIN);
    assert!(
        h.app.signing_message().is_none(),
        "the message outlived the lock"
    );
    assert_eq!(h.app.screen(), ScreenKind::Home);
}

/// A message signed under either Nonce setting verifies for the address
/// the screen named, and under Low R the BIP-137 signature it produces
/// is the shortest form.
#[test]
fn a_message_is_signed_under_either_nonce_setting() {
    for choice in [0usize, 1] {
        let mut h = Harness::new(PANEL);
        h.open_settings();
        h.tap(ids::SETTINGS_NONCE_ROW);
        h.tap(ids::at(ids::SETTINGS_NONCE_BASE, choice));
        h.tap(ids::BACK);
        h.tap(ids::BACK);
        h.start_load(&ABANDON);
        h.finish_load(None);
        h.open_single_sig(0);
        h.tap(ids::WALLET_SIGN_MESSAGE);
        h.tap(ids::SCAN_FILE);
        h.send(Event::File {
            kind: FileKind::Any,
            bytes: MESSAGE.as_bytes().to_vec(),
        });
        h.tap(ids::MSG_CONTINUE);
        h.hold(ids::MSG_HOLD);
        let flow = h.app.signing_message().expect("the flow");
        let out = flow.outcome().expect("a signature");
        assert_eq!(
            checked(&out.address, flow.text(), &out.signature),
            Ok(Checked {
                format: Format::Bip137,
                script: ScriptType::NativeSegwit,
            }),
            "choice {choice}"
        );
        let bytes = osk_psbt::base64::decode(&out.signature).expect("base64");
        if choice == 0 {
            // A compact signature is header, r, s; a low R keeps r below
            // 2^255, so its first byte is under 0x80.
            assert!(bytes[1] < 0x80, "Low R gave a high R");
        }
    }
}
