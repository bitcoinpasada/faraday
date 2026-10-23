//! A PNG on a stick, looked at for QR codes: `faraday_files::qr_in_png`
//! over any bytes. The disk process reads this for every `.png` Files
//! lists, so it takes a hostile image: the `png` crate's decoder, the
//! 4096 by 4096 limit and the QR decoder over what it makes.

#![no_main]

use faraday_crypto::work::Compute;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let c = Compute::unchecked();
    let _ = faraday_files::qr_in_png(&c, data);
});
