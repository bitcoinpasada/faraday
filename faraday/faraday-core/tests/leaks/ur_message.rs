//! A UR message taken out of the decoder: seed words sent as `ur:bytes`
//! and a `ur:crypto-seed` the registry refuses, each read to the end,
//! and the same taken by Receive and by the scanned key reader.

use crate::forms::{PHRASE, seed_forms};
use crate::{arm, finish};
use faraday_codec::ur::{Decoder, Message, encode_bytes, encode_single};
use faraday_core::{Action, Faraday, Screen, Sheet, testkit};
use faraday_leak_scan::Form;
use faraday_shell_api::{App, Event};

fn take(text: &str) -> Message {
    let mut decoder = Decoder::new();
    assert_eq!(decoder.receive(text), Ok(true), "a whole message");
    decoder.into_message().expect("a message")
}

#[test]
fn seed_words_in_a_ur_bytes_leave_no_freed_copy() {
    let forms = seed_forms("");
    let text = encode_bytes(PHRASE.as_bytes());
    let armed = arm(&forms);
    let Message::Bytes(bytes) = take(&text) else {
        panic!("not a byte message");
    };
    assert!(std::str::from_utf8(&bytes).is_ok_and(|t| t == PHRASE));
    drop(bytes);
    finish("seed words as ur:bytes", armed, false);
}

#[test]
fn a_ur_crypto_seed_refused_leaves_no_freed_copy() {
    let seed: Vec<u8> = (0..32u8)
        .map(|i| i.wrapping_mul(37).wrapping_add(11))
        .collect();
    let cbor: Vec<u8> = [0xa1, 0x01, 0x58, 0x20]
        .iter()
        .chain(&seed)
        .copied()
        .collect();
    let forms = vec![
        Form::new("crypto-seed entropy", seed),
        Form::new("crypto-seed cbor", cbor.clone()),
    ];
    let text = encode_single("crypto-seed", &cbor);
    drop(cbor);
    let armed = arm(&forms);
    let Message::Other { ur_type, cbor } = take(&text) else {
        panic!("not another type");
    };
    let read = faraday_qr::registry::read(&ur_type, &cbor);
    assert!(matches!(read, Err(faraday_qr::registry::Error::Seed)));
    drop(read);
    drop(cbor);
    finish("crypto-seed refused", armed, false);
}

fn crypto_seed() -> (Vec<Form>, Vec<u8>) {
    let seed: Vec<u8> = (0..32u8)
        .map(|i| i.wrapping_mul(37).wrapping_add(11))
        .collect();
    let cbor: Vec<u8> = [0xa1, 0x01, 0x58, 0x20]
        .iter()
        .chain(&seed)
        .copied()
        .collect();
    let forms = vec![
        Form::new("crypto-seed entropy", seed),
        Form::new("crypto-seed cbor", cbor.clone()),
    ];
    (
        forms,
        encode_single("crypto-seed", &cbor)
            .as_str()
            .as_bytes()
            .to_vec(),
    )
}

fn scan_in(app: &mut Faraday, code: Vec<u8>) {
    app.event(Event::Scanned { bytes: code });
}

#[test]
fn seed_words_in_a_ur_bytes_received_and_saved_leave_no_freed_copy() {
    let forms = seed_forms("");
    let text = encode_bytes(PHRASE.as_bytes()).as_str().as_bytes().to_vec();
    let armed = arm(&forms);
    let mut app = testkit::started();
    app.online = true;
    app.press(Action::Nav(Screen::Transfer));
    app.press(Action::TransferReceive);
    assert_eq!(app.sheet, Some(Sheet::Scan));
    scan_in(&mut app, text);
    assert_eq!(app.sheet, Some(Sheet::SecretOut), "the hold was not asked");
    app.press(Action::SecretHold);
    let Some(faraday_core::StorageCommand::SaveDownload { bytes, .. }) = app.poll_storage() else {
        panic!("nothing was saved");
    };
    assert_eq!(bytes, PHRASE.as_bytes(), "not what was sent");
    // The command's bytes belong to the shell, which frees them.
    std::mem::forget(bytes);
    app.storage(faraday_core::StorageEvent::Saved {
        path: faraday_text::Text::of("~/Downloads/received.txt"),
    });
    finish("ur:bytes seed received and saved", armed, false);
}

#[test]
fn a_ur_crypto_seed_refused_by_the_key_reader_leaves_no_freed_copy() {
    let (forms, text) = crypto_seed();
    let armed = arm(&forms);
    let mut app = testkit::started();
    app.press(Action::Nav(Screen::Home));
    app.press(Action::Scan);
    assert_eq!(app.sheet, Some(Sheet::Scan));
    scan_in(&mut app, text);
    let _ = app.frame();
    app.press(Action::Cancel);
    finish("crypto-seed refused by the key reader", armed, false);
}

#[test]
fn seed_words_in_a_ur_bytes_read_by_the_key_reader_leave_no_freed_copy() {
    let forms = seed_forms("");
    let text = encode_bytes(PHRASE.as_bytes()).as_str().as_bytes().to_vec();
    let armed = arm(&forms);
    let mut app = testkit::started();
    app.press(Action::Nav(Screen::Home));
    app.press(Action::Scan);
    assert_eq!(app.sheet, Some(Sheet::Scan));
    scan_in(&mut app, text);
    let _ = app.frame();
    app.press(Action::Cancel);
    finish("ur:bytes seed read by the key reader", armed, false);
}
