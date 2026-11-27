//! A valid code of a large version is read, whatever its data holds: a
//! version-30 alphanumeric code (137 modules a side, 1429 characters)
//! whose data holds a pattern that looks like a corner square of a code.
//! The lookalike exists because the encoder picks mask 4 for this payload.

use faraday_codec::decode_luma;
use faraday_codec::qr::{self, Ecc, Payload, QUIET_ZONE};
use faraday_crypto::work::Compute;

#[test]
fn a_version_30_code_with_a_corner_lookalike_in_its_data_is_read() {
    let c = Compute::unchecked();
    let data = include_bytes!("large_code_payload.txt");
    let code = qr::encode(&c, Payload::Alphanumeric(data), Ecc::Quartile).unwrap();
    assert_eq!(code.version(), 30);
    let (w, h, luma) = code.to_luma(4, QUIET_ZONE);
    let read = decode_luma(&c, w, h, &luma);
    assert_eq!(read.len(), 1);
    assert_eq!(read[0].bytes.as_slice(), data.as_slice());
}
