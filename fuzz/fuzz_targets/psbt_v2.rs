//! `faraday_psbt::v2::PsbtV2::parse_bytes` / `parse_base64`: a version 2
//! PSBT from a coordinator. One that is read writes back to bytes that
//! read to the same PSBT, and converts to version 0 or is refused.

#![no_main]

use faraday_crypto::work::Compute;
use faraday_psbt::v2::PsbtV2;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let c = Compute::unchecked();
    if let Ok(psbt) = PsbtV2::parse_bytes(&c, data) {
        assert_eq!(PsbtV2::parse_bytes(&c, &psbt.to_bytes()).as_ref(), Ok(&psbt));
        let _ = psbt.to_v0(&c);
    }
    if let Ok(text) = core::str::from_utf8(data) {
        let _ = PsbtV2::parse_base64(&c, text);
    }
});
