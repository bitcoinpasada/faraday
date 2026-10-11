//! `osk_bip::xkey::decode_xpriv` / `decode_xpub` and
//! `osk_bip::slip132::decode_xpub`: an extended key as it is scanned or
//! typed, in the BIP-32 and SLIP-132 prefixes.

#![no_main]

use libfuzzer_sys::fuzz_target;
use osk_bip::{slip132, xkey};

fuzz_target!(|data: &[u8]| {
    let Ok(text) = core::str::from_utf8(data) else {
        return;
    };
    let _ = xkey::decode_xpriv(text);
    let _ = xkey::decode_xpub(text);
    let _ = slip132::decode_xpub(text);
});
