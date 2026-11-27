//! What the Starter's writer sees from a payload file, and what `--image`
//! sees from a developer's own image (`docs/STARTER.md` §1.1, §1.3,
//! §4.1). The `.xz` files in `vectors/` were made by the xz program;
//! `vectors/README.md` says how, and what `sha256sum` gives for each.

use std::io::Cursor;

use faraday_payload::{
    ImageKind, ImageSum, PayloadError, PayloadFile, Sha256Sum, hash_image, write_image,
    write_payload,
};

const FIXTURE_XZ: &[u8] = include_bytes!("vectors/fixture.img.xz");
const FIXTURE_XZ_SHA256: &str = "d7e055e6f9f0d8a3c019d1e6b272b793dd607c3929a2a8f922cee3a5eae71c4f";
const FIXTURE_SHA256: &str = "c3a69d3c8e5599d5dc08dc6780f76b30a8e4d53082a85d40c6cd2effd28dfb0c";
const FIXTURE_SIZE: u64 = 39936;

const ZEROS_XZ: &[u8] = include_bytes!("vectors/zeros.img.xz");
const ZEROS_XZ_SHA256: &str = "236e07f110fe513771bfb64dc3670d1051209787b11ca55f60167cd0df9a64ee";
const ZEROS_SHA256: &str = "bb9f8df61474d25e71fa00722318cd387396ca1736605e1248821cc0de3d3af8";
const ZEROS_SIZE: u64 = 4 * 1024 * 1024;

/// The image inside `fixture.img.xz`, made as `vectors/README.md` makes it.
fn fixture_image() -> Vec<u8> {
    let mut out = String::new();
    for i in 0u64..1024 {
        let mixed = (i * 2_654_435_761) % (1 << 32);
        out.push_str(&format!("{i:05} {mixed:08x} faraday payload fixture\n"));
    }
    out.into_bytes()
}

fn sum(hex: &str) -> Sha256Sum {
    Sha256Sum::from_hex(hex).unwrap()
}

fn sha256(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// A payload file's entry, as the list names it: a stick image's carries
/// its boot partition's sum, any other none.
fn entry(name: &str, xz_sum: &str, inner: &str, inner_sum: &str, size: u64) -> PayloadFile {
    PayloadFile {
        name: name.to_owned(),
        sha256: sum(xz_sum),
        inner_name: inner.to_owned(),
        inner_sha256: sum(inner_sum),
        inner_size: size,
        build_id: "6.6.84-faraday-0.2.0".to_owned(),
        boot_sha256: name
            .starts_with("faraday-stick-")
            .then(|| sum(FIXTURE_SHA256)),
    }
}

fn fixture_entry() -> PayloadFile {
    entry(
        "fixture.img.xz",
        FIXTURE_XZ_SHA256,
        "fixture.img",
        FIXTURE_SHA256,
        FIXTURE_SIZE,
    )
}

/// The outcome of a write, what the writer received and the last
/// progress report.
type Written = (Result<(), PayloadError>, Vec<u8>, Option<(u64, u64)>);

/// Writes a payload into a Vec.
fn write(xz: &[u8], entry: &PayloadFile) -> Written {
    let mut out = Vec::new();
    let mut last = None;
    let r = write_payload(Cursor::new(xz), entry, &mut out, |done, total| {
        last = Some((done, total));
    });
    (r, out, last)
}

#[test]
fn a_payload_made_by_xz_decodes_to_exactly_the_original_bytes() {
    let (r, out, last) = write(FIXTURE_XZ, &fixture_entry());

    r.unwrap();
    assert_eq!(out, fixture_image());
    assert_eq!(last, Some((FIXTURE_SIZE, FIXTURE_SIZE)));
}

#[test]
fn an_image_larger_than_one_write_decodes_whole() {
    let entry = entry(
        "zeros.img.xz",
        ZEROS_XZ_SHA256,
        "zeros.img",
        ZEROS_SHA256,
        ZEROS_SIZE,
    );
    let mut reports = 0;
    let mut out = Vec::new();
    write_payload(Cursor::new(ZEROS_XZ), &entry, &mut out, |_, _| reports += 1).unwrap();
    assert_eq!(out.len() as u64, ZEROS_SIZE);
    assert!(out.iter().all(|&b| b == 0));
    assert!(reports > 1, "progress was reported {reports} time(s)");
}

#[test]
fn a_payload_whose_xz_sum_does_not_match_is_refused_before_anything_is_written() {
    // One byte of the .xz changed: the list's sum no longer matches.
    let mut damaged = FIXTURE_XZ.to_vec();
    damaged[FIXTURE_XZ.len() / 2] ^= 0x01;
    let (r, out, last) = write(&damaged, &fixture_entry());
    assert!(matches!(r, Err(PayloadError::PayloadSum { .. })), "{r:?}");
    assert!(out.is_empty(), "the writer received {} bytes", out.len());
    assert_eq!(last, None);

    // The right file against a line in the list with another sum.
    let mut entry = fixture_entry();
    entry.sha256 = sum(ZEROS_XZ_SHA256);
    let (r, out, _) = write(FIXTURE_XZ, &entry);
    assert!(matches!(r, Err(PayloadError::PayloadSum { .. })), "{r:?}");
    assert!(out.is_empty(), "the writer received {} bytes", out.len());
}

#[test]
fn decompressed_bytes_that_do_not_match_the_list_are_a_failure() {
    // Another SHA-256 for what is inside.
    let mut entry = fixture_entry();
    entry.inner_sha256 = sum(ZEROS_SHA256);
    let (r, _, _) = write(FIXTURE_XZ, &entry);
    assert!(matches!(r, Err(PayloadError::Sum { .. })), "{r:?}");

    // A size larger than what is inside.
    let mut entry = fixture_entry();
    entry.inner_size = FIXTURE_SIZE + 1;
    let (r, _, _) = write(FIXTURE_XZ, &entry);
    assert!(
        matches!(r, Err(PayloadError::Shorter { expected, actual }) if expected == FIXTURE_SIZE + 1 && actual == FIXTURE_SIZE),
        "{r:?}"
    );

    // A size smaller than what is inside: nothing past it is written.
    let mut entry = fixture_entry();
    entry.inner_size = 1000;
    let (r, out, _) = write(FIXTURE_XZ, &entry);
    assert!(
        matches!(r, Err(PayloadError::Longer { expected: 1000 })),
        "{r:?}"
    );
    assert!(out.len() <= 1000, "the writer received {} bytes", out.len());
}

#[test]
fn a_truncated_xz_is_refused() {
    for keep in [0, 6, 100, FIXTURE_XZ.len() / 2, FIXTURE_XZ.len() - 1] {
        let cut = &FIXTURE_XZ[..keep];
        // The list describes the cut file, so the .xz sum check
        // passes and the decoder itself has to notice.
        let mut entry = fixture_entry();
        entry.sha256 = sum(&sha256(cut));
        let (r, _, _) = write(cut, &entry);
        assert!(r.is_err(), "kept {keep} bytes: {r:?}");

        let r = hash_image(cut, ImageKind::Xz);
        assert!(
            matches!(r, Err(PayloadError::Damaged(_))),
            "kept {keep} bytes: {r:?}"
        );
    }
}

#[test]
fn a_damaged_xz_with_no_list_is_refused_by_its_own_checks() {
    // A changed byte inside the compressed data, and one in the block's
    // CRC-64, its last 8 bytes (the block starts 12 bytes in, and
    // `xz -lvv` gives its total size as 6028).
    for at in [FIXTURE_XZ.len() / 2, 12 + 6028 - 1] {
        let mut damaged = FIXTURE_XZ.to_vec();
        damaged[at] ^= 0x01;
        let r = hash_image(&damaged[..], ImageKind::Xz);
        assert!(
            matches!(r, Err(PayloadError::Damaged(_))),
            "byte {at}: {r:?}"
        );
    }

    // Something after the stream that is not another stream.
    let mut trailing = FIXTURE_XZ.to_vec();
    trailing.extend_from_slice(b"not xz");
    let r = hash_image(&trailing[..], ImageKind::Xz);
    assert!(matches!(r, Err(PayloadError::Damaged(_))), "{r:?}");
}

#[test]
fn a_developers_image_has_the_sum_sha256sum_gives_plain_or_xz() {
    let image = fixture_image();
    let expected = ImageSum {
        sha256: sum(FIXTURE_SHA256),
        size: FIXTURE_SIZE,
    };
    assert_eq!(hash_image(&image[..], ImageKind::Plain).unwrap(), expected);
    assert_eq!(hash_image(FIXTURE_XZ, ImageKind::Xz).unwrap(), expected);
    assert_eq!(
        ImageKind::of_name("faraday-x86_64-uefi-dev.img.xz"),
        ImageKind::Xz
    );
    assert_eq!(
        ImageKind::of_name("faraday-x86_64-uefi-dev.img"),
        ImageKind::Plain
    );
    assert_eq!(
        ImageKind::of_name("faraday-x86_64-uefi-dev-boot.vfat"),
        ImageKind::Plain
    );

    for (src, kind) in [(&image[..], ImageKind::Plain), (FIXTURE_XZ, ImageKind::Xz)] {
        let mut out = Vec::new();
        let mut last = None;
        write_image(src, kind, &expected, &mut out, |d, t| last = Some((d, t))).unwrap();
        assert_eq!(out, image, "{kind:?}");
        assert_eq!(last, Some((FIXTURE_SIZE, FIXTURE_SIZE)), "{kind:?}");
    }
}

#[test]
fn a_developers_image_that_changed_after_it_was_confirmed_is_a_failure() {
    let confirmed = hash_image(&fixture_image()[..], ImageKind::Plain).unwrap();
    let mut changed = fixture_image();
    changed[0] = b'9';
    let r = write_image(
        &changed[..],
        ImageKind::Plain,
        &confirmed,
        &mut Vec::new(),
        |_, _| {},
    );
    assert!(matches!(r, Err(PayloadError::Sum { .. })), "{r:?}");
}
