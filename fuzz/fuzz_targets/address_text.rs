//! `faraday_codec::address_text::find`: the addresses in a text file.
//! Anything may be found or not; nothing may panic, at most 100 addresses
//! are kept, none twice by what it parses to, each reads as an address, and the same text gives
//! the same addresses twice.

#![no_main]

use faraday_codec::address_text::{MAX_ADDRESSES, find};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(text) = core::str::from_utf8(data) else {
        return;
    };
    let found = find(text);
    assert!(found.addresses.len() <= MAX_ADDRESSES);
    assert!(found.total >= found.addresses.len());
    for (i, a) in found.addresses.iter().enumerate() {
        // Bech32 is one address in either case; base58 is as written.
        let key = |a: &str| {
            let lower = a.to_ascii_lowercase();
            if faraday_bip::address::parses(&lower) {
                lower
            } else {
                a.to_string()
            }
        };
        assert!(found.addresses[..i].iter().all(|b| key(b) != key(a)));
        assert!(faraday_bip::address::parses(a));
    }
    assert_eq!(find(text), found);
});
