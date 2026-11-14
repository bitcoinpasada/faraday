//! `faraday_codec::bip21::parse`: a `bitcoin:` URI scanned or pasted.
//! Anything may be refused; nothing may panic, a URI read has no `?` in its
//! address and an amount no larger than 21 million bitcoin, and it reads the
//! same twice.

#![no_main]

use faraday_codec::bip21::parse;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(text) = core::str::from_utf8(data) else {
        return;
    };
    if let Ok(uri) = parse(text) {
        assert!(!uri.address.contains('?'));
        assert!(uri.amount.is_none_or(|a| a <= 21_000_000 * 100_000_000));
        assert_eq!(parse(text), Ok(uri));
    }
});
