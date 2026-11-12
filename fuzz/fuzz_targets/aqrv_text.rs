//! `aqrv::from_text` (the Argon QR Vault's 32-byte size as 47 characters
//! of QR's alphanumeric table, `https://github.com/bitcoinpasada/aqrv`, `AQRV.md` §3, §9) on the input as
//! text: it never panics; a text it reads is exactly 47 characters and is
//! the one `to_text` writes for the bytes it gives. The input's first 32
//! bytes also go to `to_text`, whose text is read back to those bytes
//! unless its first character is not `A`.

#![no_main]

use aqrv::{NotText, from_text, to_text};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(text) = std::str::from_utf8(data) {
        if let Ok(bytes) = from_text(text) {
            assert_eq!(text.chars().count(), 47);
            assert_eq!(to_text(&bytes), text);
        }
    }
    if let Ok(bytes) = <[u8; 32]>::try_from(data.get(..32).unwrap_or(&[])) {
        let text = to_text(&bytes);
        assert_eq!(text.len(), 47);
        match from_text(&text) {
            Ok(back) => assert_eq!(back, bytes),
            Err(why) => assert_eq!(why, NotText::First),
        }
    }
});
