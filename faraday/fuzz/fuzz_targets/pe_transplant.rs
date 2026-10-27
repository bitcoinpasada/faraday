//! A boot file the person signed, against the release's (`docs/STARTER.md`
//! §4.5, F133): the input is the signed copy, the release a fixed PE
//! (`faraday-sb`'s tests' `tiny_pe`, `tests/vectors/tiny.efi`).
//! `pe::transplant` and `pe::signer` run on it and never panic; whenever
//! `transplant` takes it, what it builds has the release's image hash and
//! is the release's bytes but for the checksum, the certificate-table
//! entry and the table after them.

#![no_main]

use faraday_sb::pe;
use libfuzzer_sys::fuzz_target;

/// The release: 0x400 bytes, already a multiple of eight.
const RELEASE: &[u8] = include_bytes!("../../faraday-sb/tests/vectors/tiny.efi");
/// Where its optional header's checksum is.
const CHECKSUM: usize = 0x44 + 20 + 64;
/// Where its certificate-table directory entry is.
const CERT_DIR: usize = 0x44 + 20 + 112 + 4 * 8;

fuzz_target!(|signed: &[u8]| {
    let _ = pe::signer(signed);
    let Ok(built) = pe::transplant(RELEASE, signed) else {
        return;
    };
    let _ = pe::signer(&built);
    assert_eq!(
        pe::image_hash(&built).ok(),
        pe::hash_as_signed(RELEASE).ok()
    );
    assert!(built.len() > RELEASE.len());
    for (i, (b, r)) in built.iter().zip(RELEASE).enumerate() {
        let carried = (CHECKSUM..CHECKSUM + 4).contains(&i) || (CERT_DIR..CERT_DIR + 8).contains(&i);
        assert!(carried || b == r, "byte {i} is not the release's");
    }
});
