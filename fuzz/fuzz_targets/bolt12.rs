//! `faraday_codec::bolt12::decode`: an offer scanned or pasted. Anything may
//! be refused; nothing may panic, an offer read must name an issuer or a path,
//! carry a description when it has an amount, and name a chain it knows.

#![no_main]

use faraday_codec::bolt12::{Chain, decode};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(text) = core::str::from_utf8(data) else {
        return;
    };
    if let Ok(offer) = decode(text) {
        assert!(offer.issuer_id.is_some() || !offer.paths.is_empty());
        assert!(offer.amount.is_none() || offer.description.is_some());
        assert!(offer.features.iter().all(|b| b % 2 == 1));
        assert!(
            offer.chains.is_empty() || offer.chains.iter().any(|c| !matches!(c, Chain::Other(_)))
        );
    }
});
