//! The Decode screen's reading of a raw transaction: the input is a
//! transaction (`faraday_psbt::transaction::decode`), and what it is
//! read as, with no prevouts and with the test session's wallets,
//! `decode::decode`, never panics.

#![no_main]

use faraday_core::{decode, testkit};
use faraday_crypto::work::Compute;
use faraday_psbt::transaction;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Some(tx) = transaction::decode(data) else {
        return;
    };
    let c = Compute::unchecked();
    let session = testkit::session();
    let none = vec![None; tx.input.len()];
    let _ = decode::decode(&c, &tx, &none, &session, "fuzz");
});
