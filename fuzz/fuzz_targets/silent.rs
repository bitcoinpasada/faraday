//! Silent payments' strings as they arrive: `faraday_bip::silent::decode_address`
//! and `decode_scan_key` (BIP-352's address, BIP-392's `spscan1q…`),
//! `faraday_bip::silent_wallet::parse_descriptor` (BIP-392's `sp()` or its
//! key alone) and the record `SilentWallet::parse` keeps. A record read is
//! written back and must read as itself.

#![no_main]

use faraday_bip::silent;
use faraday_bip::silent_wallet::{self, SilentWallet};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(text) = core::str::from_utf8(data) else {
        return;
    };
    let c = faraday_crypto::work::Compute::unchecked();
    let _ = silent::decode_address(text);
    let _ = silent::decode_scan_key(&c, text);
    if let Ok(w) = silent_wallet::parse_descriptor(&c, text) {
        assert_eq!(SilentWallet::parse(&w.to_text()), Some(w));
    }
    if let Some(w) = SilentWallet::parse(text) {
        assert_eq!(SilentWallet::parse(&w.to_text()), Some(w));
    }
});
