//! `faraday_sb::release::check` (F134): a release's `SHA256SUMS` and
//! `SHA256SUMS.asc` beside a boot program, from the input, against the
//! GnuPG-made test release key and boot program in
//! `tools/vectors/release-sums/`. The input's first two bytes split it:
//! that many bytes are the sums, the rest the signature. Each half is
//! also tried with the fixture's other half. It never panics, and never
//! takes a boot program without a valid signature over the sums: what it
//! takes, `faraday_pgp::verify_detached` takes too, and the fixture's
//! signature is taken over the fixture's sums alone.

#![no_main]

use faraday_pgp::{Certificate, verify_detached};
use faraday_sb::release::{self, ReleaseKey};
use libfuzzer_sys::fuzz_target;

const KEY: &[u8] = include_bytes!("../../../tools/vectors/release-sums/release-key.asc");
const SUMS: &[u8] = include_bytes!("../../../tools/vectors/release-sums/SHA256SUMS");
const ASC: &[u8] = include_bytes!("../../../tools/vectors/release-sums/SHA256SUMS.asc");
/// The fixture's boot program's SHA-256 (`sha256sum BOOTX64.EFI`).
const BOOT: [u8; 32] = [
    0x27, 0xbb, 0x0b, 0xf5, 0x34, 0x16, 0xa9, 0xab, 0x76, 0x9f, 0xb4, 0xa9, 0xab, 0xda, 0x20, 0x01,
    0xb1, 0x5e, 0xa4, 0xbc, 0x49, 0x8b, 0x87, 0xb2, 0xc6, 0x92, 0xf5, 0xc2, 0xd4, 0x83, 0xa6, 0xe4,
];

fn key() -> ReleaseKey<'static> {
    let cert = Certificate::read(KEY).expect("the fixture's key");
    ReleaseKey {
        certificate: KEY,
        fingerprint: cert.fingerprint(),
    }
}

/// The check, and what it must agree with when it takes the file.
fn checked(sums: &[u8], asc: &[u8]) {
    let k = key();
    if let Ok(r) = release::check(&BOOT, Some(sums), Some(asc), &[k], "0.1.7") {
        let cert = Certificate::read(KEY).expect("the fixture's key");
        assert!(verify_detached(&cert, sums, asc).is_ok());
        assert_eq!(r.sha256(), &BOOT);
        if asc == ASC {
            assert_eq!(sums, SUMS);
        }
    }
}

fuzz_target!(|data: &[u8]| {
    let Some((cut, rest)) = data.split_first_chunk::<2>() else {
        return;
    };
    let cut = usize::from(u16::from_be_bytes(*cut)).min(rest.len());
    let (sums, asc) = rest.split_at(cut);
    checked(sums, asc);
    checked(sums, ASC);
    checked(SUMS, asc);
    let _ = release::check(&BOOT, Some(sums), None, &[key()], "0.1.7");
});
