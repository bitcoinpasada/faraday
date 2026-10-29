//! A raw transaction in hex or bytes, as a stick file or a scanned code
//! holds one: `transaction::read_raw`, `transaction::decode` (the consensus
//! bytes it reads through), and `Spending::read`, which reads a
//! transaction or a PSBT for the previous outputs it states.

#![no_main]

use faraday_psbt::transaction::{Spending, decode, read_raw};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = read_raw(data);
    let _ = decode(data);
    let _ = Spending::read(data);
});
