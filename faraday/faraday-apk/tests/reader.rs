//! What the reader makes of files it is not meant to trust
//! (`docs/APK-SIGNING.md` §5): the fixtures read as what they are, and no
//! change to a byte of one, nor cutting it short, is more than a refusal.

use std::path::Path;

use faraday_apk::zip::{Apk, Refusal, Slice, names_manifest};

fn fixture(name: &str) -> Vec<u8> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tools/vectors/apk")
        .join(name);
    std::fs::read(&path).unwrap_or_else(|e| panic!("{path:?}: {e}"))
}

#[test]
fn the_small_fixture_reads_as_what_it_states() {
    let bytes = fixture("small-unsigned.apk");
    assert!(names_manifest(&bytes));
    let apk = Apk::read(&mut Slice(&bytes)).unwrap();
    assert_eq!(apk.info.package, "app.faradayoffline.fixture");
    assert_eq!(apk.info.version_name.as_deref(), Some("1.0"));
    assert_eq!((apk.info.version_code, apk.info.min_sdk), (1, 26));
    assert_eq!(apk.size, bytes.len() as u64);
}

#[test]
fn a_file_is_an_apk_by_its_central_directory_and_not_by_its_name_or_its_start() {
    assert!(!names_manifest(b""));
    assert!(!names_manifest(b"PK"));
    assert!(!names_manifest(&[0u8; 1000]));
    assert!(!names_manifest(b"\x89PNG\r\n\x1a\n not a zip"));
    // Every refused fixture still names its manifest: it is an APK, and
    // `Apk::read` says why it cannot be signed.
    for name in [
        "old-sdk-unsigned.apk",
        "unaligned-unsigned.apk",
        "gap-unsigned.apk",
    ] {
        assert!(names_manifest(&fixture(name)), "{name}");
    }
}

#[test]
fn a_changed_or_missing_byte_is_a_refusal_and_never_a_panic() {
    let bytes = fixture("small-unsigned.apk");
    let mut refused = 0usize;
    for at in (0..bytes.len()).step_by(3) {
        let mut b = bytes.clone();
        b[at] ^= 0xa5;
        if Apk::read(&mut Slice(&b)).is_err() {
            refused += 1;
        }
        let _ = names_manifest(&b);
    }
    // The headers, names, sizes and the manifest are all checked; the
    // padding and the library's data are not, and read as before.
    assert!(refused > 500, "{refused} of {}", bytes.len() / 3);
    for len in (0..bytes.len()).step_by(7) {
        assert!(Apk::read(&mut Slice(&bytes[..len])).is_err(), "{len}");
    }
}

#[test]
fn trailing_bytes_after_the_end_record_are_refused() {
    let mut bytes = fixture("small-unsigned.apk");
    bytes.extend_from_slice(b"extra");
    assert_eq!(Apk::read(&mut Slice(&bytes)).unwrap_err(), Refusal::NotApk);
}
