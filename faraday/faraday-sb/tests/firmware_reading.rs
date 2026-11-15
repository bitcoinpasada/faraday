//! A signed image is judged as firmware judges it (Authenticode, UEFI
//! 2.10): the certificate table ends the file, the signer is named by
//! the certificate that verifies, the content is SPC_INDIRECT_DATA, and
//! the algorithms are SHA-256 and RSA PKCS #1 v1.5; another algorithm is
//! one this check does not read, not a change to the file. `sbverify`
//! (sbsigntool) is the outside judge: the vectors in `vectors/` were
//! made with `sbsign` and `osslsigncode` by `vectors/make.sh`. A tool
//! not on PATH (`sbverify` also as `FARADAY_SBVERIFY`) fails the test.

use std::path::PathBuf;
use std::process::Command;

use faraday_sb::{Check, Keys, make, pe, rng};

const NOW: u64 = 1_700_000_000;

fn keys() -> Keys {
    make(&[7u8; 32], "Firmware", NOW).unwrap()
}

/// A Faraday-signed tiny image and the certificate that signed it.
fn signed() -> (Vec<u8>, Vec<u8>) {
    let k = keys();
    let tiny = include_bytes!("vectors/tiny.efi");
    let mut blind = rng::stream(&[8u8; 32], 0);
    (pe::sign(&mut blind, tiny, &k.db).unwrap(), k.db.cert)
}

/// Where `needle` last occurs in `hay`.
fn last(hay: &[u8], needle: &[u8]) -> usize {
    hay.windows(needle.len())
        .rposition(|w| w == needle)
        .expect("the bytes are in the image")
}

/// Whether `sbverify` takes `image` signed by the DER certificate `cert`.
fn sbverify_takes(name: &str, image: &[u8], cert: &[u8]) -> bool {
    let dir = std::env::temp_dir().join(format!("faraday-sb-fw-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = |f: &str| -> PathBuf { dir.join(f) };
    std::fs::write(path("c.der"), cert).unwrap();
    std::fs::write(path("i.efi"), image).unwrap();
    let pem = Command::new("openssl")
        .args(["x509", "-inform", "DER", "-in"])
        .arg(path("c.der"))
        .arg("-out")
        .arg(path("c.pem"))
        .output()
        .expect("openssl is installed");
    assert!(pem.status.success());
    let sbverify = std::env::var("FARADAY_SBVERIFY").unwrap_or_else(|_| "sbverify".to_string());
    let ok = Command::new(sbverify)
        .arg("--cert")
        .arg(path("c.pem"))
        .arg(path("i.efi"))
        .output()
        .expect("sbverify is installed (sbsigntool)")
        .status
        .success();
    let _ = std::fs::remove_dir_all(&dir);
    ok
}

#[test]
fn an_image_with_a_byte_after_its_signature_is_not_called_signed() {
    // Firmware hashes the byte after the certificate table; Faraday did not.
    let cert = include_bytes!("vectors/cert.der");
    let sbsign = include_bytes!("vectors/tiny.sbsign.efi").to_vec();
    assert_eq!(pe::check(&sbsign, cert).unwrap(), Check::Valid);
    assert!(sbverify_takes("whole", &sbsign, cert));
    let mut longer = sbsign.clone();
    longer.push(b'x');
    assert_eq!(pe::check(&longer, cert).unwrap(), Check::Unreadable);
    assert!(!sbverify_takes("longer", &longer, cert));
    assert_eq!(pe::signer(&longer).unwrap(), pe::Signer::Broken);

    let (image, cert) = signed();
    let mut longer = image;
    longer.push(0);
    assert_eq!(pe::check(&longer, &cert).unwrap(), Check::Unreadable);
}

#[test]
fn a_signature_naming_another_certificate_is_by_another_signer() {
    let (mut image, cert) = signed();
    assert_eq!(pe::check(&image, &cert).unwrap(), Check::Valid);
    // The signer's name in its SignerInfo, after the certificates.
    let at = last(&image, b"Firmware");
    image[at] ^= 1;
    assert_eq!(pe::check(&image, &cert).unwrap(), Check::OtherSigner);
    assert_eq!(pe::signer(&image).unwrap(), pe::Signer::Broken);
}

#[test]
fn content_that_is_not_authenticodes_is_unreadable() {
    let (mut image, cert) = signed();
    // SPC_INDIRECT_DATA's encoding, first where the content type is.
    let oid = [0x2b, 0x06, 0x01, 0x04, 0x01, 0x82, 0x37, 0x02, 0x01, 0x04];
    let at = image
        .windows(oid.len())
        .position(|w| w == oid)
        .expect("the content type is in the image");
    image[at + oid.len() - 1] = 0x05;
    assert_eq!(pe::check(&image, &cert).unwrap(), Check::Unreadable);
}

#[test]
fn a_digest_or_signature_algorithm_firmware_may_read_is_not_read() {
    let (image, cert) = signed();
    // SHA-256's and rsaEncryption's encodings, last where the SignerInfo
    // names them.
    let sha256 = [0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x01];
    let rsa = [0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x01];
    let mut digest = image.clone();
    let at = last(&digest, &sha256);
    digest[at + sha256.len() - 1] = 0x02; // SHA-384
    assert_eq!(pe::check(&digest, &cert).unwrap(), Check::NotRead);
    assert_eq!(pe::signer(&digest).unwrap(), pe::Signer::NotRead);
    let mut sig = image.clone();
    let at = last(&sig, &rsa);
    sig[at + rsa.len() - 1] = 0x05; // sha1WithRSAEncryption
    assert_eq!(pe::check(&sig, &cert).unwrap(), Check::NotRead);
    // sha256WithRSAEncryption is the same signature.
    let mut same = image;
    let at = last(&same, &rsa);
    same[at + rsa.len() - 1] = 0x0b;
    assert_eq!(pe::check(&same, &cert).unwrap(), Check::Valid);
}

#[test]
fn an_image_signed_with_sha_384_or_sha_1_is_not_read_and_not_changed() {
    let cert = include_bytes!("vectors/cert.sha.der");
    for (name, image) in [
        ("sha384", &include_bytes!("vectors/tiny.sha384.efi")[..]),
        ("sha1", &include_bytes!("vectors/tiny.sha1.efi")[..]),
    ] {
        assert_eq!(pe::check(image, cert).unwrap(), Check::NotRead, "{name}");
        assert_eq!(pe::signer(image).unwrap(), pe::Signer::NotRead, "{name}");
        assert!(!sbverify_takes(name, image, cert), "{name}");
    }
}

#[test]
fn images_signed_by_other_tools_with_sha_256_still_check() {
    let cert = include_bytes!("vectors/cert.der");
    for (name, image) in [
        ("sbsign", &include_bytes!("vectors/tiny.sbsign.efi")[..]),
        (
            "osslsigncode",
            &include_bytes!("vectors/tiny.osslsigncode.efi")[..],
        ),
    ] {
        assert_eq!(pe::check(image, cert).unwrap(), Check::Valid, "{name}");
        assert!(matches!(pe::signer(image).unwrap(), pe::Signer::By(_)));
    }
}

/// Where the certificate table's entry is in the optional header's data
/// directories (PE32+): its address and size fields.
fn cert_dir(image: &[u8]) -> usize {
    let nt = u32::from_le_bytes(image[0x3c..0x40].try_into().unwrap()) as usize;
    assert_eq!(&image[nt + 24..nt + 26], &[0x0b, 0x02], "PE32+");
    nt + 24 + 112 + 4 * 8
}

fn table_of(image: &[u8]) -> (usize, usize) {
    let d = cert_dir(image);
    let at = u32::from_le_bytes(image[d..d + 4].try_into().unwrap()) as usize;
    let size = u32::from_le_bytes(image[d + 4..d + 8].try_into().unwrap()) as usize;
    assert_eq!(at + size, image.len());
    (at, size)
}

fn set_table(image: &mut [u8], at: usize, size: usize) {
    let d = cert_dir(image);
    image[d..d + 4].copy_from_slice(&(at as u32).to_le_bytes());
    image[d + 4..d + 8].copy_from_slice(&(size as u32).to_le_bytes());
}

#[test]
fn a_changed_image_is_changed_whatever_its_signer_says() {
    // Firmware hashes the image with the content's SHA-256 and refuses a
    // different hash before it reads the signer.
    let (image, cert) = signed();
    let (at, _) = table_of(&image);
    let mut changed = image.clone();
    changed[at - 1] ^= 1;
    assert_eq!(pe::check(&changed, &cert).unwrap(), Check::Altered);
    let sha256 = [0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x01];
    let n = last(&changed, &sha256);
    changed[n + sha256.len() - 1] = 0x02; // the signer's digest: SHA-384
    assert_eq!(pe::check(&changed, &cert).unwrap(), Check::Altered);
    assert_eq!(pe::signer(&changed).unwrap(), pe::Signer::Broken);
    assert!(!sbverify_takes("changed", &changed, &cert));
}

#[test]
fn a_table_that_is_not_where_firmware_reads_it_is_unreadable() {
    let (image, cert) = signed();
    let (at, size) = table_of(&image);
    assert_eq!(at % 8, 0);
    // Not on an 8-byte boundary: a byte in front, the table still last.
    let mut shifted = image[..at].to_vec();
    shifted.push(0);
    shifted.extend_from_slice(&image[at..]);
    set_table(&mut shifted, at + 1, size);
    assert!(pe::image_hash(&shifted).is_err());
    assert_eq!(pe::check(&shifted, &cert).unwrap(), Check::Unreadable);
    assert_eq!(pe::signer(&shifted).unwrap(), pe::Signer::Broken);
    // Starting inside the last section.
    let mut inside = image.clone();
    set_table(&mut inside, at - 8, size + 8);
    assert!(pe::image_hash(&inside).is_err());
    assert_eq!(pe::check(&inside, &cert).unwrap(), Check::Unreadable);
    assert_eq!(pe::signer(&inside).unwrap(), pe::Signer::Broken);
}

#[test]
fn a_signer_id_naming_another_serial_is_by_another_signer() {
    let (mut image, cert) = signed();
    // The certificate's serial number (INTEGER after its version), last
    // where the SignerInfo names it.
    let v3 = [0xa0, 0x03, 0x02, 0x01, 0x02];
    let c = cert.windows(v3.len()).position(|w| w == v3).unwrap() + v3.len();
    assert_eq!(cert[c], 0x02);
    let integer = &cert[c..c + 2 + usize::from(cert[c + 1])];
    let end = last(&image, integer) + integer.len() - 1;
    image[end] ^= 1;
    assert_eq!(pe::check(&image, &cert).unwrap(), Check::OtherSigner);
    assert_eq!(pe::signer(&image).unwrap(), pe::Signer::Broken);
    assert!(!sbverify_takes("serial", &image, &cert));
}

#[test]
fn two_content_type_attributes_are_unreadable() {
    let (mut image, cert) = signed();
    // The message digest attribute's name, made a second content type.
    let digest = [0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x09, 0x04];
    let at = last(&image, &digest);
    image[at + digest.len() - 1] = 0x03;
    assert_eq!(pe::check(&image, &cert).unwrap(), Check::Unreadable);
    assert_eq!(pe::signer(&image).unwrap(), pe::Signer::Broken);
    assert!(!sbverify_takes("twotypes", &image, &cert));
}
