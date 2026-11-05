//! KEF's text forms (`docs/KEF.md` §5): what is written reads back, leading
//! zero bytes included, and text that is not canonical is not read.

use faraday_codec::kef::{
    Form, base32_decode, base43_decode, base43_encode, base64_decode, encode, recognize,
};

#[test]
fn base43_keeps_leading_zero_bytes() {
    for bytes in [
        &b""[..],
        &[0],
        &[0, 0, 1],
        &[0, 0, 0xff, 0xfe, 0xfd],
        &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12],
        &[0xff; 40],
    ] {
        assert_eq!(base43_decode(&base43_encode(bytes)).unwrap(), bytes);
    }
    // Electrum's base43: 43 is "10", and 2 leading zeros are 2 leading "0".
    assert_eq!(base43_encode(&[43]), "10");
    assert_eq!(base43_encode(&[0, 0, 5]), "005");
    assert!(base43_decode("lower").is_none());
}

#[test]
fn the_other_text_forms_read_back() {
    let bytes: Vec<u8> = (0..=40).collect();
    let base32 = encode(Form::Base32, &bytes).unwrap();
    assert_eq!(base32_decode(&base32).unwrap(), bytes);
    let base64 = encode(Form::Base64, &bytes).unwrap();
    assert_eq!(base64_decode(&base64).unwrap(), bytes);
    assert_eq!(encode(Form::Hex, &[0xab, 0x01]).unwrap(), "AB01");
    assert_eq!(encode(Form::Base64, b"Ma").unwrap(), "TWE=");
    assert_eq!(encode(Form::Base32, b"foobar").unwrap(), "MZXW6YTBOI");
    assert!(base64_decode("TWE").is_none());
    assert!(recognize(b"not a code").is_none());
}

#[test]
fn a_very_long_base43_text_is_at_once_not_a_code() {
    // Longer than any envelope: not decoded, so it comes back at once.
    let long = "1".repeat(400_000);
    let started = std::time::Instant::now();
    assert!(recognize(long.as_bytes()).is_none());
    assert!(started.elapsed().as_secs() < 2);
}
