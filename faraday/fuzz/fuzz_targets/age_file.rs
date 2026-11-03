//! An age passphrase file, as the vault flow is handed one from a stick:
//! `recognise` reads the header text with no key, and `open` parses the
//! header, the scrypt stanza and the payload's chunks. The work factor is
//! limited to 10 so a file that states more is refused before any work.

#![no_main]

use faraday_crypto::work::Compute;
use faraday_vault::age;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = age::recognise(data);
    let c = Compute::unchecked();
    let _ = age::open(&c, b"password", data, 10);
});
