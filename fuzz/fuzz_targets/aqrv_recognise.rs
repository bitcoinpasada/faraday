//! `aqrv::recognise`, `recognise_code` and `recognise_file` (the Argon QR
//! Vault, `docs/AQRV.md` §9, §9.1) on the input as a QR payload and as a
//! file: none panics; a recognised header is the one the length and the
//! first byte give and the file's reading is the bytes'; a payload is an
//! envelope only as §9 says (53 or 62 bytes of binary, or 47 characters of
//! text) and gives the bytes of its text form back.

#![no_main]

use aqrv::{Size, Step, from_text, recognise, recognise_code, recognise_file, to_text};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let header = recognise(data);
    assert_eq!(header, recognise_file(data));
    if let Some(h) = header {
        assert_eq!(Some(h.size), Size::of_len(data.len()));
        assert_eq!(h.step.header(), data[0]);
        assert_eq!(Step::of_header(data[0]), Some(h.step));
    }
    match recognise_code(data) {
        None => assert!(data.len() == 32 || header.is_none()),
        Some(envelope) => {
            assert!(recognise(&envelope).is_some());
            if data.len() == 32 || header.is_none() {
                // Text: 47 characters that read back to the same bytes.
                let text = std::str::from_utf8(data).expect("only text is read as text");
                let bytes: [u8; 32] = envelope.as_slice().try_into().expect("32 bytes");
                assert_eq!(from_text(text), Ok(bytes));
                assert_eq!(to_text(&bytes).as_bytes(), data);
            } else {
                assert_eq!(envelope, data);
            }
        }
    }
});
