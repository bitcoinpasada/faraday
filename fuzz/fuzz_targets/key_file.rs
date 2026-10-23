//! A cosigner's key file, as Create and Restore read it from a stick:
//! a BSMS signer record (`parse`, then `verify`), a Coldcard export's
//! cosigner lines, a SLIP-132 key line and a policy key, all over any
//! text another wallet's writer chose.

#![no_main]

use faraday_bip::bsms::SignerRecord;
use faraday_bip::policy::PolicyKey;
use faraday_bip::{coldcard, slip132};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    if let Ok(record) = SignerRecord::parse(text) {
        let _ = record.verify();
        let _ = record.key_line();
    }
    for line in coldcard::cosigner_keys(text) {
        let _ = slip132::key_line(&line);
        let _ = PolicyKey::parse(&line);
    }
    for line in text.lines().take(256) {
        let _ = slip132::key_line(line);
        let _ = PolicyKey::parse(line);
    }
});
