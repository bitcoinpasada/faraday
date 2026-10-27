//! A boot program is signed only when it is a Faraday release's own
//! (`faraday_sb::release`, `docs/decisions/F134.md`). The release here is
//! the GnuPG-made fixture in `tools/vectors/release-sums/` (its README
//! has the commands): a test release key, the sums of release 0.9.0
//! listing the fixture's `BOOTX64.EFI` as `faraday-0.9.0-BOOTX64.EFI`,
//! signed by that key and by another.
//! Faraday 0.1.7's published sums check the key built into Faraday.

use std::path::{Path, PathBuf};

use faraday_sb::release::{self, KEYS, Refusal, ReleaseKey};
use faraday_sb::rng::Stream;
use faraday_sb::{Check, Error, Keys, make, pe};
use sha2::{Digest, Sha256};

fn dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools/vectors/release-sums")
}

fn read(name: &str) -> Vec<u8> {
    std::fs::read(dir().join(name)).unwrap_or_else(|e| panic!("{name}: {e}"))
}

fn sha(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

/// The fixture's release key, with the fingerprint `gpg --show-keys`
/// prints for it.
fn test_key(certificate: &[u8]) -> ReleaseKey<'_> {
    ReleaseKey {
        certificate,
        fingerprint: [
            0xBA, 0xFE, 0x0C, 0xDD, 0x02, 0x1B, 0x72, 0xA8, 0x43, 0x66, 0x81, 0x83, 0x66, 0x28,
            0x46, 0xB9, 0x76, 0x71, 0x65, 0x84,
        ],
    }
}

/// The Faraday checking, in these tests: 0.1.7, so the test release
/// 0.9.0 is newer and Faraday 0.1.7's own is as new.
const RUNNING: &str = "0.1.7";

fn check(
    file: &[u8; 32],
    sums: Option<&[u8]>,
    asc: Option<&[u8]>,
    keys: &[ReleaseKey<'_>],
) -> Result<release::Release, Refusal> {
    release::check(file, sums, asc, keys, RUNNING)
}

/// A variant of the sums in `variants/`, with its signature by the test
/// release key, checked for the fixture's boot program.
fn variant(name: &str, running: &str) -> Result<release::Release, Refusal> {
    let cert = read("release-key.asc");
    release::check(
        &sha(&read("BOOTX64.EFI")),
        Some(&read(&format!("variants/{name}.sums"))),
        Some(&read(&format!("variants/{name}.sums.asc"))),
        &[test_key(&cert)],
        running,
    )
}

fn keys() -> Keys {
    make(&[0x42; 32], "Test", 1_791_000_000).expect("keys")
}

#[test]
fn a_release_boot_file_with_genuine_signed_sums_is_signed() {
    let cert = read("release-key.asc");
    let boot = read("BOOTX64.EFI");
    let release = check(
        &sha(&boot),
        Some(&read("SHA256SUMS")),
        Some(&read("SHA256SUMS.asc")),
        &[test_key(&cert)],
    )
    .expect("the release's own boot program");
    assert_eq!(release.version(), "0.9.0");
    let k = keys();
    let signed =
        release::sign(&mut Stream::new(&[0x43; 32], 0), &boot, &k.db, &release).expect("signed");
    assert_eq!(pe::check(&signed, &k.db.cert), Ok(Check::Valid));
}

#[test]
fn sums_signed_by_another_key_or_changed_are_refused() {
    let cert = read("release-key.asc");
    let boot = read("BOOTX64.EFI");
    let sums = read("SHA256SUMS");
    let other = check(
        &sha(&boot),
        Some(&sums),
        Some(&read("SHA256SUMS.other.asc")),
        &[test_key(&cert)],
    );
    assert_eq!(other, Err(Refusal::NotSigned));
    // The sums with a line added for another boot program.
    let mut tampered = sums.clone();
    tampered
        .extend_from_slice(format!("{}  faraday-0.9.0-BOOTX64.EFI\n", "00".repeat(32)).as_bytes());
    let changed = check(
        &sha(&boot),
        Some(&tampered),
        Some(&read("SHA256SUMS.asc")),
        &[test_key(&cert)],
    );
    assert_eq!(changed, Err(Refusal::NotSigned));
    // A key whose fingerprint is not the one built in is not taken.
    let mut wrong = test_key(&cert);
    wrong.fingerprint[0] ^= 1;
    let unpinned = check(
        &sha(&boot),
        Some(&sums),
        Some(&read("SHA256SUMS.asc")),
        &[wrong],
    );
    assert_eq!(unpinned, Err(Refusal::NotSigned));
}

#[test]
fn a_boot_file_the_sums_do_not_list_is_refused_naming_the_release() {
    let cert = read("release-key.asc");
    let mut boot = read("BOOTX64.EFI");
    boot[0x200] = 0x90;
    let refused = check(
        &sha(&boot),
        Some(&read("SHA256SUMS")),
        Some(&read("SHA256SUMS.asc")),
        &[test_key(&cert)],
    );
    assert_eq!(
        refused,
        Err(Refusal::NotListed {
            version: "0.9.0".to_string()
        })
    );
}

#[test]
fn without_the_sums_or_their_signature_nothing_is_taken() {
    let cert = read("release-key.asc");
    let boot = sha(&read("BOOTX64.EFI"));
    let sums = read("SHA256SUMS");
    let asc = read("SHA256SUMS.asc");
    for (s, a) in [
        (None, None),
        (Some(&sums[..]), None),
        (None, Some(&asc[..])),
    ] {
        assert_eq!(check(&boot, s, a, &[test_key(&cert)]), Err(Refusal::NoSums));
    }
}

#[test]
fn a_release_signs_only_the_file_it_names() {
    let cert = read("release-key.asc");
    let boot = read("BOOTX64.EFI");
    let release = check(
        &sha(&boot),
        Some(&read("SHA256SUMS")),
        Some(&read("SHA256SUMS.asc")),
        &[test_key(&cert)],
    )
    .expect("the release's own boot program");
    let mut other = boot.clone();
    other[0x200] = 0x90;
    let k = keys();
    assert_eq!(
        release::sign(&mut Stream::new(&[0x43; 32], 0), &other, &k.db, &release),
        Err(Error::NotRelease)
    );
}

#[test]
fn the_built_in_release_key_is_the_repositorys_and_checks_faraday_0_1_7s_sums() {
    let file = std::fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join("../release-key.asc"))
        .expect("faraday/release-key.asc");
    assert_eq!(KEYS.len(), 1);
    assert_eq!(KEYS[0].certificate, &file[..]);
    let cert = faraday_pgp::Certificate::read(KEYS[0].certificate).expect("the release key");
    assert_eq!(
        faraday_pgp::fingerprint_text(&cert.fingerprint()),
        "D1AF A712 ABEF 9010 9A1B 587F F153 6BE0 7E23 4CFD"
    );
    assert_eq!(cert.fingerprint(), KEYS[0].fingerprint);
    // 0.1.7's sums verify with it, and list no boot program of their own.
    let refused = check(
        &sha(&read("BOOTX64.EFI")),
        Some(&read("faraday-0.1.7/SHA256SUMS")),
        Some(&read("faraday-0.1.7/SHA256SUMS.asc")),
        KEYS,
    );
    assert_eq!(
        refused,
        Err(Refusal::NotListed {
            version: "0.1.7".to_string()
        })
    );
    // The test release's sums do not verify with it.
    let test = check(
        &sha(&read("BOOTX64.EFI")),
        Some(&read("SHA256SUMS")),
        Some(&read("SHA256SUMS.asc")),
        KEYS,
    );
    assert_eq!(test, Err(Refusal::NotSigned));
}

#[test]
fn a_release_older_than_the_faraday_signing_is_refused_and_one_as_new_is_not() {
    let older = || Refusal::Older {
        version: "0.1.0".to_string(),
        running: "0.1.7".to_string(),
    };
    assert_eq!(variant("older", "0.1.7"), Err(older()));
    // As new as the Faraday checking, or newer: taken.
    assert_eq!(
        variant("older", "0.1.0").map(|r| r.version().to_string()),
        Ok("0.1.0".into())
    );
    assert!(variant("older", "0.0.9").is_ok());
    // A pre-release of 0.1.0 counts as 0.1.0.
    assert!(variant("older", "0.1.0-rc.1").is_ok());
    // The test release 0.9.0, from a Faraday 1.0.0.
    assert_eq!(
        variant("crlf", "1.0.0"),
        Err(Refusal::Older {
            version: "0.9.0".to_string(),
            running: "1.0.0".to_string(),
        })
    );
    // A running version that does not read as one refuses.
    assert!(matches!(variant("crlf", "dev"), Err(Refusal::Older { .. })));
}

#[test]
fn the_boot_line_twice_or_only_another_versions_is_not_the_release_boot_program() {
    let not_listed = Err(Refusal::NotListed {
        version: "0.9.0".to_string(),
    });
    for name in ["twice-different", "twice-same", "other-version"] {
        assert_eq!(variant(name, RUNNING), not_listed, "{name}");
    }
}

#[test]
fn sums_whose_first_line_does_not_name_the_release_are_not_a_releases() {
    for name in ["no-version", "version-second"] {
        assert_eq!(variant(name, RUNNING), Err(Refusal::NoSums), "{name}");
    }
}

#[test]
fn crlf_sums_and_a_binary_mode_line_are_read_as_sha256sum_reads_them() {
    for name in ["crlf", "binary-mode"] {
        let release = variant(name, RUNNING).unwrap_or_else(|e| panic!("{name}: {e:?}"));
        assert_eq!(release.version(), "0.9.0", "{name}");
    }
}

#[test]
fn a_text_mode_signature_over_the_sums_is_refused() {
    let cert = read("release-key.asc");
    assert_eq!(
        check(
            &sha(&read("BOOTX64.EFI")),
            Some(&read("SHA256SUMS")),
            Some(&read("SHA256SUMS.textmode.asc")),
            &[test_key(&cert)],
        ),
        Err(Refusal::NotSigned)
    );
}
