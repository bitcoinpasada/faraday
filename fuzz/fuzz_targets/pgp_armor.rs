//! `faraday_pgp::read_public_armor`: a stick file that looks like an
//! armoured OpenPGP block, in its armour and packet headers.

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = faraday_pgp::read_public_armor(data);
});
