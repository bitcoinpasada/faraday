//! A key file chosen from a stick (`docs/VAULT.md` §3.2): any bytes, read
//! as XML version 1 or 2, 32 bytes, hexadecimal or a hash. It never panics,
//! and what it reads once it reads the same again.

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let read = |d: &[u8]| faraday_vault::kdbx::keyfile::read(d).map(|k| *k);
    assert_eq!(read(data), read(data));
});
