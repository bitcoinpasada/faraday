//! `faraday_bip::policy::WalletPolicy::parse_any` and `parse_script_descriptor`: a wallet registration as
//! it is scanned, in either the BIP-388 two-part form or as a plain
//! descriptor.

#![no_main]

use libfuzzer_sys::fuzz_target;
use faraday_bip::miniscript::descriptor::DescriptorPublicKey;
use faraday_bip::policy::{WalletPolicy, parse_script_descriptor};
use faraday_crypto::work::Compute;

fuzz_target!(|data: &[u8]| {
    let Ok(text) = core::str::from_utf8(data) else {
        return;
    };
    let c = Compute::unchecked();
    let _ = WalletPolicy::parse_any(&c, text);
    let _ = parse_script_descriptor::<DescriptorPublicKey>(&c, text);
});
