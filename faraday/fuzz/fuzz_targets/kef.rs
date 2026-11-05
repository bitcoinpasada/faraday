//! Krux's encrypted code: `recognize` on any scanned or read bytes,
//! `unwrap` on any envelope, and decryption of what unwraps under a fixed
//! key with a count of 1 (`open_with_key`, so no PBKDF2 runs). The same
//! bytes are also sealed under that key as version 15 and 20 and read as
//! the compressed version 16 and 21 (same layout, same checks), so a hostile
//! deflate stream reaches inflate. Nothing panics, and nothing a compressed
//! version opens is longer than the inflate limit.

#![no_main]

use faraday_kef::{INFLATE_LIMIT, open_with_key, recognize, seal_with_key, unwrap};
use libfuzzer_sys::fuzz_target;

const KEY: [u8; 32] = [0x5a; 32];
/// The versions that compress.
const COMPRESSED: [u8; 4] = [7, 12, 16, 21];

fuzz_target!(|data: &[u8]| {
    let _ = recognize(data);
    if let Ok(envelope) = unwrap(data) {
        if let Ok(opened) = open_with_key(&KEY, &envelope) {
            if COMPRESSED.contains(&envelope.version) {
                assert!(opened.plaintext.len() <= INFLATE_LIMIT);
            }
        }
    }
    // The data as a deflate stream: sealed uncompressed, read compressed.
    for (plain, compressed, iv) in [(15u8, 16u8, 12usize), (20, 21, 12)] {
        if let Ok(mut code) = seal_with_key(&KEY, data, plain, b"id", 1, &vec![0; iv]) {
            let at = 1 + code[0] as usize;
            code[at] = compressed;
            if let Ok(envelope) = unwrap(&code) {
                if let Ok(opened) = open_with_key(&KEY, &envelope) {
                    assert!(opened.plaintext.len() <= INFLATE_LIMIT);
                }
            }
        }
    }
});
