//! `faraday_bip::keys::MasterKey::decode`: an `xprv` typed, pasted or
//! scanned by a person, by the strict rules of `faraday_bip::xkey`.

#![no_main]

use faraday_bip::keys::MasterKey;
use faraday_crypto::work::Compute;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(text) = core::str::from_utf8(data) else {
        return;
    };
    let _ = MasterKey::decode(&Compute::unchecked(), text);
});
