//! `faraday_qr::specter::parse`: the `pNofM text` header of a Specter
//! animated QR part.

#![no_main]

use faraday_qr::specter;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    let _ = specter::parse(text);
});
