//! `faraday_bip::xkey::decode_xpriv` / `decode_xpub`,
//! `faraday_bip::slip132::decode_xpub` and `faraday_bip::wif`'s
//! `SingleKey::decode`: a key as it is scanned or typed, an extended key
//! in the BIP-32 and SLIP-132 prefixes or a single private key as WIF.

#![no_main]

use faraday_bip::{slip132, wif, xkey};
use faraday_crypto::work::Compute;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(text) = core::str::from_utf8(data) else {
        return;
    };
    let _ = xkey::decode_xpriv(text);
    let _ = xkey::decode_xpub(text);
    let _ = slip132::decode_xpub(text);
    let _ = wif::SingleKey::decode(&Compute::unchecked(), text);
});
