//! `faraday_codec::bolt11::decode`: an invoice scanned or pasted. Anything
//! may be refused; nothing may panic, an invoice read must carry a payee key
//! in compressed form, and its fallback addresses must come out as text.

#![no_main]

use faraday_codec::bolt11::decode;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(text) = core::str::from_utf8(data) else {
        return;
    };
    if let Ok(invoice) = decode(text) {
        assert!(matches!(invoice.payee[0], 2 | 3));
        for f in &invoice.fallbacks {
            assert!(!f.address(invoice.network).is_empty());
        }
    }
});
