//! The Argon QR Vault's item layouts (`docs/AQRV.md` §3, §3.1): the room
//! of plaintext `P` a reader finds after the tag matched, read as a kind's
//! item (`aqrv::fuzz_read`). It never panics, and an item it reads is
//! laid out again by the writer as the bytes it was read from: the same
//! length and bytes, a codex32 string's unused low bits apart.
//!
//! The input is a kind (byte 0, an index into the nine), a size (byte 1,
//! 0 to 2) and then the room, zero-filled or cut to the size's length.

#![no_main]

use aqrv::{Item, Kind, Size, fuzz_read};
use libfuzzer_sys::fuzz_target;

const SIZES: [Size; 3] = [Size::Tiny, Size::Small, Size::Large];

fuzz_target!(|data: &[u8]| {
    let [kind, size, rest @ ..] = data else { return };
    let kind = Kind::ALL[usize::from(*kind) % Kind::ALL.len()];
    let size = SIZES[usize::from(*size) % SIZES.len()];
    // A size holds only some kinds, and no envelope has a kind its size
    // does not hold (§3), so a reader never gets here with one.
    if !size.holds(kind) {
        return;
    }
    let mut p = rest.to_vec();
    p.resize(size.room(), 0);
    let Ok((item, laid)) = fuzz_read(kind, size, &p) else {
        return;
    };
    // Entropy's kind byte is its length, so a size that holds the kind
    // holds the item.
    let (laid_kind, bytes, mask) = laid.expect("an item read lays out again");
    assert_eq!(laid_kind, kind);
    assert_eq!(item.kind(), Some(kind));
    assert!(bytes.len() <= p.len());
    let mask = mask.unwrap_or(0);
    let last = bytes.len() - 1;
    assert_eq!(bytes[..last], p[..last]);
    assert_eq!(bytes[last], p[last] & !mask);
    if let Item::Codex32(string) = &item {
        assert!(string.starts_with("ms1") || string.starts_with("MS1"));
    }
});
