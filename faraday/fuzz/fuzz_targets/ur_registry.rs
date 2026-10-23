//! The UR registry types a scanned code carries once its parts are put
//! together: the first byte picks the UR type (`psbt`, `crypto-hdkey`,
//! `crypto-account`, `crypto-output`, a seed and an unknown one) and the
//! rest is the CBOR given to `registry::read`; the CBOR alone goes to
//! `cbor::Value::decode`. The fountain decoder under it is `ur_receive`
//! in the root `fuzz/`.

#![no_main]

use faraday_qr::cbor::Value;
use faraday_qr::registry;
use libfuzzer_sys::fuzz_target;

const TYPES: [&str; 8] = [
    "psbt",
    "crypto-hdkey",
    "crypto-account",
    "crypto-output",
    "hdkey",
    "account-descriptor",
    "output-descriptor",
    "crypto-seed",
];

fuzz_target!(|data: &[u8]| {
    let Some((&first, cbor)) = data.split_first() else {
        return;
    };
    let _ = registry::read(TYPES[usize::from(first) % TYPES.len()], cbor);
    let _ = Value::decode(cbor);
});
