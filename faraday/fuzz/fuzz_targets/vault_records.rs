//! A slot's plaintext, read once a passphrase opens it:
//! `records::Contents::decode` over any bytes, then the records written
//! out again. The plaintext is authenticated, so only a vault written by
//! a buggy or hostile program reaches this; it still must not panic.

#![no_main]

use faraday_vault::records::Contents;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok((contents, seals)) = Contents::decode(data) {
        let _ = contents.used();
        let _ = contents.encode(data.len(), seals);
    }
});
