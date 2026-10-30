//! `faraday_bip::silent::decode_address` and
//! `faraday_bip::silent_wallet::SilentWallet::parse`: a BIP-352 address
//! and a silent payments wallet record. A record that parses writes back
//! to a record that parses to the same wallet.

#![no_main]

use faraday_bip::silent::decode_address;
use faraday_bip::silent_wallet::SilentWallet;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(text) = core::str::from_utf8(data) else {
        return;
    };
    let _ = decode_address(text);
    if let Some(wallet) = SilentWallet::parse(text) {
        assert_eq!(SilentWallet::parse(&wallet.to_text()), Some(wallet));
    }
});
