//! What the Starter's writer and Open Faraday Demo see from a payload
//! file, and what `--image` sees from a developer's own image
//! (`docs/STARTER.md` §1.1, §1.3, §4.1). The `.xz` files in `vectors/` were
//! made by the xz program; `vectors/README.md` says how, and what
//! `sha256sum` gives for each.

use std::io::Cursor;

use faraday_payload::{
    ImageKind, ImageSum, Manifest, ManifestFault, PayloadError, PayloadFile, Sha256Sum, hash_image,
    write_image, write_payload,
};

const FIXTURE_XZ: &[u8] = include_bytes!("vectors/fixture.img.xz");
const FIXTURE_XZ_SHA256: &str = "d7e055e6f9f0d8a3c019d1e6b272b793dd607c3929a2a8f922cee3a5eae71c4f";
const FIXTURE_SHA256: &str = "c3a69d3c8e5599d5dc08dc6780f76b30a8e4d53082a85d40c6cd2effd28dfb0c";
const FIXTURE_SIZE: u64 = 39936;

const ZEROS_XZ: &[u8] = include_bytes!("vectors/zeros.img.xz");
const ZEROS_XZ_SHA256: &str = "236e07f110fe513771bfb64dc3670d1051209787b11ca55f60167cd0df9a64ee";
const ZEROS_SHA256: &str = "bb9f8df61474d25e71fa00722318cd387396ca1736605e1248821cc0de3d3af8";
const ZEROS_SIZE: u64 = 4 * 1024 * 1024;

const COMMIT: &str = "0123456789abcdef0123456789abcdef01234567";

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

fn line(name: &str, xz_sum: &str, inner: &str, inner_sum: &str, size: u64) -> String {
    format!("{name}\t{xz_sum}\t{inner}\t{inner_sum}\t{size}\t6.6.84-faraday-0.2.0")
}

fn manifest(lines: &[String]) -> Vec<u8> {
    let mut text = format!("# Faraday Starter payload 0.2.0, commit {COMMIT}\n");
    for l in lines {
        text.push_str(l);
        text.push('\n');
    }
    text.into_bytes()
}

fn fixture_entry() -> PayloadFile {
    PayloadFile::parse_line(&line(
        "fixture.img.xz",
        FIXTURE_XZ_SHA256,
        "fixture.img",
        FIXTURE_SHA256,
        FIXTURE_SIZE,
    ))
    .unwrap()
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
    let manifest = Manifest::parse(&manifest(&[line(
        "fixture.img.xz",
        FIXTURE_XZ_SHA256,
        "fixture.img",
        FIXTURE_SHA256,
        FIXTURE_SIZE,
    )]))
    .unwrap();
    let entry = manifest.file("fixture.img.xz").unwrap();

    let (r, out, last) = write(FIXTURE_XZ, entry);

    r.unwrap();
    assert_eq!(out, fixture_image());
    assert_eq!(last, Some((FIXTURE_SIZE, FIXTURE_SIZE)));
}

#[test]
fn an_image_larger_than_one_write_decodes_whole() {
    let entry = PayloadFile::parse_line(&line(
        "zeros.img.xz",
        ZEROS_XZ_SHA256,
        "zeros.img",
        ZEROS_SHA256,
        ZEROS_SIZE,
    ))
    .unwrap();
    let mut reports = 0;
    let mut out = Vec::new();
    write_payload(Cursor::new(ZEROS_XZ), &entry, &mut out, |_, _| reports += 1).unwrap();
    assert_eq!(out.len() as u64, ZEROS_SIZE);
    assert!(out.iter().all(|&b| b == 0));
    assert!(reports > 1, "progress was reported {reports} time(s)");
}

#[test]
fn a_payload_whose_xz_sum_does_not_match_is_refused_before_anything_is_written() {
    // One byte of the .xz changed: the manifest's sum no longer matches.
    let mut damaged = FIXTURE_XZ.to_vec();
    damaged[FIXTURE_XZ.len() / 2] ^= 0x01;
    let (r, out, last) = write(&damaged, &fixture_entry());
    assert!(matches!(r, Err(PayloadError::PayloadSum { .. })), "{r:?}");
    assert!(out.is_empty(), "the writer received {} bytes", out.len());
    assert_eq!(last, None);

    // The right file against a manifest line with another sum.
    let mut entry = fixture_entry();
    entry.sha256 = sum(ZEROS_XZ_SHA256);
    let (r, out, _) = write(FIXTURE_XZ, &entry);
    assert!(matches!(r, Err(PayloadError::PayloadSum { .. })), "{r:?}");
    assert!(out.is_empty(), "the writer received {} bytes", out.len());
}

#[test]
fn decompressed_bytes_that_do_not_match_the_manifest_are_a_failure() {
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
        // The manifest describes the cut file, so the .xz sum check
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
fn a_damaged_xz_with_no_manifest_is_refused_by_its_own_checks() {
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

#[test]
fn a_manifest_parses_into_its_files() {
    let text = manifest(&[
        line(
            "a.img.xz",
            FIXTURE_XZ_SHA256,
            "a.img",
            FIXTURE_SHA256,
            FIXTURE_SIZE,
        ),
        line(
            "b-linux-x86_64.xz",
            ZEROS_XZ_SHA256,
            "b-linux-x86_64",
            ZEROS_SHA256,
            ZEROS_SIZE,
        ),
    ]);
    let m = Manifest::parse(&text).unwrap();
    assert_eq!(m.version(), "0.2.0");
    assert_eq!(m.commit(), COMMIT);
    assert_eq!(m.files().len(), 2);
    let b = m.file("b-linux-x86_64.xz").unwrap();
    assert_eq!(b.inner_name, "b-linux-x86_64");
    assert_eq!(b.inner_sha256, sum(ZEROS_SHA256));
    assert_eq!(b.inner_size, ZEROS_SIZE);
    assert_eq!(b.build_id, "6.6.84-faraday-0.2.0");
    assert!(m.file("c.img.xz").is_none());

    // Without its final newline too.
    let mut bare = text.clone();
    bare.pop();
    assert_eq!(Manifest::parse(&bare).unwrap(), m);
}

/// A payload carries the images and Faraday Demo for its system
/// (F83): a manifest naming the desktop app lists it like any file.
#[test]
fn a_payload_may_list_the_desktop_app_beside_the_images() {
    let stick = line(
        "faraday-stick-0.2.0-x86_64-uefi.img.xz",
        FIXTURE_XZ_SHA256,
        "faraday-stick-0.2.0-x86_64-uefi.img",
        FIXTURE_SHA256,
        FIXTURE_SIZE,
    );
    let desktop = line(
        "faraday-demo-0.2.0-linux-x86_64.xz",
        ZEROS_XZ_SHA256,
        "faraday-demo-0.2.0-linux-x86_64",
        ZEROS_SHA256,
        ZEROS_SIZE,
    );
    let m = Manifest::parse(&manifest(&[stick, desktop])).unwrap();
    let names: Vec<&str> = m.files().iter().map(|f| f.name.as_str()).collect();
    assert_eq!(
        names,
        [
            "faraday-stick-0.2.0-x86_64-uefi.img.xz",
            "faraday-demo-0.2.0-linux-x86_64.xz"
        ]
    );
}

#[test]
#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
fn a_malformed_manifest_is_refused_with_its_line_and_reason() {
    let good = line(
        "a.img.xz",
        FIXTURE_XZ_SHA256,
        "a.img",
        FIXTURE_SHA256,
        FIXTURE_SIZE,
    );
    let header = format!("# Faraday Starter payload 0.2.0, commit {COMMIT}");
    let refused = |text: String, line: usize, fault: ManifestFault| {
        let e = Manifest::parse(text.as_bytes()).unwrap_err();
        assert_eq!((e.line, e.fault), (line, fault), "{text:?}");
    };

    // Headers.
    for bad in [
        String::new(),
        "# Faraday payload 0.2.0, commit 0123".to_string(),
        format!("# Faraday Starter payload , commit {COMMIT}"),
        format!("# Faraday Starter payload 0.2.0 commit {COMMIT}"),
        "# Faraday Starter payload 0.2.0, commit 0123456".to_string(),
        format!(
            "# Faraday Starter payload 0.2.0, commit {}",
            COMMIT.to_uppercase()
        ),
    ] {
        refused(format!("{bad}\n{good}\n"), 1, ManifestFault::Header);
    }

    // No files, an empty line, a field too many or too few.
    refused(format!("{header}\n"), 2, ManifestFault::NoFiles);
    refused(
        format!("{header}\n{good}\n\n{good}\n"),
        3,
        ManifestFault::EmptyLine,
    );
    refused(
        format!("{header}\n{good}\textra\n"),
        2,
        ManifestFault::FieldCount(7),
    );
    refused(
        format!("{header}\n{}\n", good.replace('\t', " ")),
        2,
        ManifestFault::FieldCount(1),
    );
    refused(
        format!("{header}\n{good}\n{good}\n"),
        3,
        ManifestFault::Duplicate,
    );

    // Each field.
    let cases: [(String, ManifestFault); 12] = [
        (
            line("a.img", FIXTURE_XZ_SHA256, "a.img", FIXTURE_SHA256, 1),
            ManifestFault::Name,
        ),
        (
            line(".xz", FIXTURE_XZ_SHA256, "a.img", FIXTURE_SHA256, 1),
            ManifestFault::Name,
        ),
        (
            line("../a.img.xz", FIXTURE_XZ_SHA256, "a.img", FIXTURE_SHA256, 1),
            ManifestFault::Name,
        ),
        (
            line(
                "a.img.xz",
                FIXTURE_XZ_SHA256,
                "dir/a.img",
                FIXTURE_SHA256,
                1,
            ),
            ManifestFault::Name,
        ),
        (
            line("a.img.xz", FIXTURE_XZ_SHA256, "", FIXTURE_SHA256, 1),
            ManifestFault::Name,
        ),
        (
            line(
                "a.img.xz",
                &FIXTURE_XZ_SHA256.to_uppercase(),
                "a.img",
                FIXTURE_SHA256,
                1,
            ),
            ManifestFault::Sum,
        ),
        (
            line(
                "a.img.xz",
                FIXTURE_XZ_SHA256,
                "a.img",
                &FIXTURE_SHA256[1..],
                1,
            ),
            ManifestFault::Sum,
        ),
        (
            line("a.img.xz", FIXTURE_XZ_SHA256, "a.img", FIXTURE_SHA256, 0),
            ManifestFault::Size,
        ),
        (good.replace("\t39936\t", "\t039936\t"), ManifestFault::Size),
        (good.replace("\t39936\t", "\t-39936\t"), ManifestFault::Size),
        (
            good.replace("\t39936\t", "\t99999999999999999999\t"),
            ManifestFault::Size,
        ),
        (
            good.replace("6.6.84-faraday-0.2.0", ""),
            ManifestFault::BuildId,
        ),
    ];
    for (bad, fault) in cases {
        refused(format!("{header}\n{good}\n{bad}\n"), 3, fault);
    }

    // Windows line ends leave a carriage return in the build ID.
    refused(format!("{header}\r\n{good}\r\n"), 1, ManifestFault::Header);
    refused(format!("{header}\n{good}\r\n"), 2, ManifestFault::BuildId);

    // Not text.
    let e = Manifest::parse(b"# Faraday Starter payload \xff").unwrap_err();
    assert_eq!(e.fault, ManifestFault::NotText);
}
