//! `faraday_crypto::hex`: `decode_vec`, `decode_into` and `decode_exact` on
//! text from the online side. They agree, and what decodes encodes back to
//! the text in lower case.

#![no_main]

use faraday_crypto::hex::{decode_exact, decode_into, decode_vec, encode_lower};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(text) = core::str::from_utf8(data) else {
        return;
    };
    let vec = decode_vec(text);

    let mut buf = vec![0u8; text.len() / 2 + 1];
    let into = decode_into(text, &mut buf);
    assert_eq!(into, vec.as_ref().map(Vec::len));
    if let (Some(n), Some(bytes)) = (into, &vec) {
        assert_eq!(&buf[..n], bytes.as_slice());
    }

    let exact32 = decode_exact::<32>(text);
    let exact1 = decode_exact::<1>(text);
    match &vec {
        Some(bytes) => {
            assert_eq!(
                exact32.map(|a| a.to_vec()),
                (bytes.len() == 32).then(|| bytes.clone())
            );
            assert_eq!(
                exact1.map(|a| a.to_vec()),
                (bytes.len() == 1).then(|| bytes.clone())
            );
            let mut out = vec![0u8; bytes.len() * 2];
            assert_eq!(encode_lower(bytes, &mut out), text.to_ascii_lowercase());
        }
        None => assert!(exact32.is_none() && exact1.is_none()),
    }
});
