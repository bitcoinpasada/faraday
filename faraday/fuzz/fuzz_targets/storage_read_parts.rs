//! A file read in pieces from the disk process, answers of anyone's making
//! (`faraday_storage::read_pieces`, the loop the app's file read runs): a
//! piece of the wrong length, a total that changes between pieces, a
//! total short or over the read limit, an empty piece, a failure part way,
//! another kind of answer. It must not panic or reserve more than the read
//! limit, each piece is asked for where the last ended, and a read that
//! succeeds is exactly the pieces' bytes, as long as the total said.

#![no_main]

use faraday_files::MAX_VAULT_READ;
use faraday_files::proto::{PIECE, Response};
use faraday_storage::read_pieces;
use libfuzzer_sys::fuzz_target;

/// A length or a total from one byte: small, around a piece, around the
/// read limit, or absurd.
fn size(sel: u8, lit: u8) -> u64 {
    let piece = PIECE as u64;
    match sel % 12 {
        0 => 0,
        1..=4 => u64::from(lit),
        5 => piece - 1,
        6 => piece,
        7 => piece + 1,
        8 => piece * 2,
        9 => MAX_VAULT_READ,
        10 => MAX_VAULT_READ + 1,
        _ => u64::MAX - u64::from(lit),
    }
}

/// A piece's bytes, made from its length and a seed so the input stays
/// small; the last byte differs so a piece cut short is noticed.
fn piece(len: u64, seed: u8) -> Vec<u8> {
    let mut b = vec![seed; len as usize];
    if let Some(last) = b.last_mut() {
        *last = !seed;
    }
    b
}

fuzz_target!(|data: &[u8]| {
    let mut at = 0;
    let mut handed: Vec<u8> = Vec::new();
    let mut first_total = None;
    let result = read_pieces(|from, len| {
        assert_eq!(
            from,
            handed.len() as u64,
            "a piece is asked where the last ended"
        );
        assert_eq!(len as usize, PIECE);
        let Some(c) = data.get(at..at + 5) else {
            return Ok(Response::Failed(faraday_text::Text::of("input over")));
        };
        at += 5;
        match c[0] % 8 {
            0..=5 => {
                let total = size(c[1], c[4]);
                // Pieces that could not be held anyway stay small.
                let n = size(c[2], c[4]).min(PIECE as u64 + 1);
                let bytes = piece(n, c[3]);
                first_total.get_or_insert(total);
                // Only a piece taken is kept: copies of what the read
                // refused are not needed, and a read refusing it ends.
                handed.extend_from_slice(&bytes);
                Ok(Response::Part { total, bytes })
            }
            6 => Ok(Response::Failed(faraday_text::Text::of("failed"))),
            _ if c[1] % 2 == 0 => Ok(Response::Held(u64::from(c[2]))),
            _ => Err("no answer".into()),
        }
    });
    if let Ok(out) = result {
        let total = first_total.expect("a read that succeeded had a piece");
        assert!(total <= MAX_VAULT_READ);
        assert_eq!(out.len() as u64, total);
        assert!(out == handed, "the bytes are the pieces' bytes");
    }
});
