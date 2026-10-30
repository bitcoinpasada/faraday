//! `faraday_bip::musig`: the BIP-327 values a co-signer sends (public
//! nonce, aggregate nonce, partial signature, secret nonce) and the
//! `musig(...)` descriptor expressions. A value that is read writes back
//! to the bytes it was read from.

#![no_main]

use faraday_bip::musig::{
    AggNonce, PartialSig, PubNonce, SecNonce, parse_musig_expr, parse_tr_musig,
};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(v) = PubNonce::from_bytes(data) {
        assert_eq!(v.serialize(), data);
    }
    if let Ok(v) = AggNonce::from_bytes(data) {
        assert_eq!(v.serialize(), data);
    }
    if let Ok(v) = PartialSig::from_bytes(data) {
        assert_eq!(v.serialize(), data);
    }
    if let Ok(v) = SecNonce::from_bytes(data) {
        assert_eq!(v.serialize(), data);
    }
    if let Ok(text) = core::str::from_utf8(data) {
        let _ = parse_musig_expr(text);
        let _ = parse_tr_musig(text);
    }
});
