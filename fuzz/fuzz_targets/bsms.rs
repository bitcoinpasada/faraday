//! `faraday_bip::bsms::SignerRecord::parse` and `DescriptorRecord::parse`:
//! a BSMS key record or descriptor record from a coordinator, as a file
//! or a scan.

#![no_main]

use faraday_bip::bsms::{DescriptorRecord, SignerRecord};
use faraday_crypto::work::Compute;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(text) = core::str::from_utf8(data) else {
        return;
    };
    let _ = SignerRecord::parse(text);
    let _ = DescriptorRecord::parse(&Compute::unchecked(), text);
});
