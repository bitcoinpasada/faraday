//! A UR message taken out of the decoder: seed words sent as `ur:bytes`
//! and a `ur:crypto-seed` the registry refuses, each read to the end.

use crate::forms::{PHRASE, seed_forms};
use crate::{arm, finish};
use faraday_codec::ur::{Decoder, Message, encode_bytes, encode_single};
use faraday_leak_scan::Form;

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
