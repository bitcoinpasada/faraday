//! The QR code of one vector at each size (`docs/AQRV.md` §3, §11): the
//! pictures decode to the envelope's bytes, and the encoder makes the
//! version and level the format names. The matrix is one encoder's
//! output and its mask that encoder's choice, so the modules are compared
//! only where both encoders chose the same mask.

use faraday_codec::qr::{Ecc, Payload, QrMatrix, encode};
use faraday_crypto::work::Compute;
use faraday_files::qr_in_png;
use serde_json::Value;

fn vectors() -> Value {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tools/vectors/aqrv/aqrv.json"
    );
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

fn unhex(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}

/// The mask a symbol was drawn with, from its format information
/// (ISO/IEC 18004: 15 bits, first copy, masked with 0x5412).
fn mask_of(module: impl Fn(usize, usize) -> bool) -> u8 {
    let mut bits = 0u32;
    for i in 0..15 {
        let (x, y) = match i {
            0..=5 => (8, i),
            6 => (8, 7),
            7 => (8, 8),
            8 => (7, 8),
            _ => (14 - i, 8),
        };
        bits |= u32::from(module(x, y)) << i;
    }
    (((bits ^ 0x5412) >> 10) & 7) as u8
}

#[test]
fn a_qr_picture_decodes_to_the_envelope_at_each_size() {
    let c = Compute::unchecked();
    for entry in vectors()["qr"].as_array().unwrap() {
        let envelope = unhex(entry["envelope"].as_str().unwrap());
        let file = entry["png"].as_str().unwrap();
        let path = format!(
            "{}/../../tools/vectors/aqrv/{file}",
            env!("CARGO_MANIFEST_DIR")
        );
        let picture = std::fs::read(path).unwrap();
        let codes = qr_in_png(&c, &picture).unwrap();
        assert_eq!(codes.to_vec(), vec![envelope], "{file}");
    }
}

#[test]
fn the_envelope_is_version_3_at_53_bytes_and_version_4_at_62() {
    let c = Compute::unchecked();
    for entry in vectors()["qr"].as_array().unwrap() {
        let envelope = unhex(entry["envelope"].as_str().unwrap());
        let (version, ecc) = match envelope.len() {
            53 => (3, Ecc::Low),
            62 => (4, Ecc::Medium),
            n => panic!("an envelope of {n} bytes"),
        };
        let qr: QrMatrix = encode(&c, Payload::Bytes(&envelope), ecc).unwrap();
        assert_eq!(qr.version(), version);
        assert_eq!(qr.size() as u64, entry["modules"].as_u64().unwrap());

        // The same module for module, where the masks agree.
        let rows: Vec<&str> = entry["matrix"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| r.as_str().unwrap())
            .collect();
        let theirs = |x: usize, y: usize| rows[y].as_bytes()[x] == b'1';
        if mask_of(|x, y| qr.module(x, y)) == mask_of(theirs) {
            for y in 0..qr.size() {
                for x in 0..qr.size() {
                    assert_eq!(qr.module(x, y), theirs(x, y), "module {x},{y}");
                }
            }
        }
    }
}
