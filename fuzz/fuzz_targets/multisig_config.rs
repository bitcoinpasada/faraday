//! `faraday_bip::multisig_config::parse` and `parse_named`: a multisig
//! wallet configuration file as a coordinator writes it.

#![no_main]

use faraday_bip::multisig_config;
use faraday_crypto::work::Compute;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(text) = core::str::from_utf8(data) else {
        return;
    };
    let c = Compute::unchecked();
    let _ = multisig_config::parse(&c, text);
    let _ = multisig_config::parse_named(&c, text);
});
