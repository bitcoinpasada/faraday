//! `osk_bip::policy::WalletPolicy::parse_any`: a wallet registration as
//! it is scanned, in either the BIP-388 two-part form or as a plain
//! descriptor.

#![no_main]

use libfuzzer_sys::fuzz_target;
use osk_bip::policy::WalletPolicy;

fuzz_target!(|data: &[u8]| {
    let Ok(text) = core::str::from_utf8(data) else {
        return;
    };
    let _ = WalletPolicy::parse_any(text);
});
