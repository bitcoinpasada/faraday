//! What Faraday's Secure Boot files are, judged by other tools
//! (`docs/history/PLAN.md` §8): `openssl` reads the certificates and verifies the
//! PKCS #7 signature in each `.auth` update; `sbverify` (sbsigntool)
//! verifies a signed image against the db certificate; and Faraday's own
//! check tells a signed image from an altered one and from one signed by
//! another key. A tool not on PATH (`sbverify` also as `FARADAY_SBVERIFY`)
//! fails the test: nothing here is skipped (docs/DEVELOPING.md names the
//! packages).

use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::process::Command;

use faraday_sb::rng::Stream;
use faraday_sb::{Check, Keys, Policy, efi, enrolment, make, pe};
use sha2::{Digest, Sha256};

const NOW: u64 = 1_791_000_000;

fn keys() -> Keys {
    make(&[0x42; 32], "Test", NOW).unwrap()
}

/// The stream that blinds a test's signing.
fn blind() -> Stream {
    Stream::new(&[0x43; 32], 0)
}

/// A test's own scratch tree; removed when it drops.
struct Scratch(PathBuf);

impl Deref for Scratch {
    type Target = Path;
    fn deref(&self) -> &Path {
        &self.0
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn scratch(name: &str) -> Scratch {
    let dir = std::env::temp_dir().join(format!("faraday-sb-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    Scratch(dir)
}

fn run(cmd: &str, args: &[&str]) -> Option<(bool, String)> {
    let out = Command::new(cmd).args(args).output().unwrap_or_else(|e| {
        panic!("{cmd} is not installed ({e}); these tests need openssl and sbverify, see docs/DEVELOPING.md")
    });
    Some((
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).into_owned() + &String::from_utf8_lossy(&out.stderr),
    ))
}

fn pem(dir: &Path, name: &str, der: &[u8]) -> Option<String> {
    let d = dir.join(format!("{name}.der"));
    let p = dir.join(format!("{name}.pem"));
    std::fs::write(&d, der).unwrap();
    let (ok, _) = run(
        "openssl",
        &[
            "x509",
            "-inform",
            "DER",
            "-in",
            d.to_str()?,
            "-out",
            p.to_str()?,
        ],
    )?;
    ok.then(|| p.to_str().unwrap().to_string())
}

#[test]
fn microsoft_certificates_are_what_was_recorded() {
    for m in faraday_sb::MICROSOFT {
        let got: String = Sha256::digest(m.der)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        assert_eq!(got, m.sha256, "{}", m.name);
        assert_eq!(faraday_sb::common_name(m.der).as_deref(), Some(m.name));
    }
}

#[test]
fn openssl_verifies_the_certificates_and_the_auth_signatures() {
    let dir = scratch("openssl");
    let k = keys();
    let pk = pem(&dir, "pk", &k.pk.cert).expect("openssl reads the PK certificate");
    let kek = pem(&dir, "kek", &k.kek.cert).unwrap();
    for c in [&pk, &kek] {
        let (ok, said) = run("openssl", &["verify", "-check_ss_sig", "-CAfile", c, c]).unwrap();
        assert!(ok, "{said}");
    }
    let files = enrolment(&mut blind(), &k, Policy::Windows, NOW).unwrap();
    for (name, vendor, signer) in [
        ("PK", efi::GLOBAL_VARIABLE, &pk),
        ("KEK", efi::GLOBAL_VARIABLE, &pk),
        ("db", efi::IMAGE_SECURITY_DATABASE, &kek),
    ] {
        let auth = &files
            .iter()
            .find(|(n, _)| *n == format!("{name}.auth"))
            .unwrap()
            .1;
        let esl = &files
            .iter()
            .find(|(n, _)| *n == format!("{name}.esl"))
            .unwrap()
            .1;
        let len = u32::from_le_bytes(auth[16..20].try_into().unwrap()) as usize;
        let signed_data = &auth[16 + 24..16 + len];
        assert_eq!(&auth[16 + len..], esl.as_slice());
        // What the firmware hashes: name, vendor, attributes, time, data.
        let mut content: Vec<u8> = name.encode_utf16().flat_map(u16::to_le_bytes).collect();
        content.extend_from_slice(&vendor);
        content.extend_from_slice(&0x27u32.to_le_bytes());
        content.extend_from_slice(&auth[..16]);
        content.extend_from_slice(esl);
        // openssl reads a ContentInfo around the SignedData.
        let wrapped = faraday_sb::der::seq(&[
            &faraday_sb::der::oid(&[1, 2, 840, 113_549, 1, 7, 2]),
            &faraday_sb::der::explicit(0, signed_data),
        ]);
        let sig = dir.join(format!("{name}.p7"));
        let data = dir.join(format!("{name}.bin"));
        std::fs::write(&sig, wrapped).unwrap();
        std::fs::write(&data, &content).unwrap();
        let out = dir.join("out");
        let (ok, said) = run(
            "openssl",
            &[
                "cms",
                "-verify",
                "-binary",
                "-inform",
                "DER",
                "-in",
                sig.to_str().unwrap(),
                "-content",
                data.to_str().unwrap(),
                "-CAfile",
                signer,
                "-purpose",
                "any",
                "-out",
                out.to_str().unwrap(),
            ],
        )
        .unwrap();
        assert!(ok, "{name}.auth:\n{said}");
    }
}

/// A minimal PE32+ EFI application: headers and one section of code.
fn tiny_pe() -> Vec<u8> {
    let mut pe = vec![0u8; 0x400];
    let put16 =
        |pe: &mut Vec<u8>, at: usize, v: u16| pe[at..at + 2].copy_from_slice(&v.to_le_bytes());
    let put32 =
        |pe: &mut Vec<u8>, at: usize, v: u32| pe[at..at + 4].copy_from_slice(&v.to_le_bytes());
    pe[0..2].copy_from_slice(b"MZ");
    put32(&mut pe, 0x3c, 0x40);
    pe[0x40..0x44].copy_from_slice(b"PE\0\0");
    let coff = 0x44;
    put16(&mut pe, coff, 0x8664);
    put16(&mut pe, coff + 2, 1);
    put16(&mut pe, coff + 16, 240);
    put16(&mut pe, coff + 18, 0x22);
    let opt = coff + 20;
    put16(&mut pe, opt, 0x20b);
    put32(&mut pe, opt + 16, 0x1000); // AddressOfEntryPoint
    put32(&mut pe, opt + 32, 0x1000); // SectionAlignment
    put32(&mut pe, opt + 36, 0x200); // FileAlignment
    put32(&mut pe, opt + 56, 0x2000); // SizeOfImage
    put32(&mut pe, opt + 60, 0x200); // SizeOfHeaders
    put16(&mut pe, opt + 68, 10); // an EFI application
    put32(&mut pe, opt + 108, 16); // NumberOfRvaAndSizes
    let sec = opt + 240;
    pe[sec..sec + 5].copy_from_slice(b".text");
    put32(&mut pe, sec + 8, 0x10);
    put32(&mut pe, sec + 12, 0x1000);
    put32(&mut pe, sec + 16, 0x200);
    put32(&mut pe, sec + 20, 0x200);
    put32(&mut pe, sec + 36, 0x6000_0020);
    pe[0x200] = 0xc3;
    pe
}

#[test]
fn a_signed_image_checks_and_an_altered_one_does_not() {
    let dir = scratch("image");
    let k = keys();
    let mut images = vec![("tiny", tiny_pe())];
    if let Ok(path) = std::env::var("FARADAY_EFI") {
        images.push(("efi", std::fs::read(path).unwrap()));
    }
    let sbverify = std::env::var("FARADAY_SBVERIFY").unwrap_or_else(|_| "sbverify".to_string());
    let db = pem(&dir, "db", &k.db.cert).expect("openssl reads the db certificate");
    for (name, image) in images {
        assert_eq!(pe::check(&image, &k.db.cert).unwrap(), Check::Unsigned);
        let signed = pe::sign(&mut blind(), &image, &k.db).unwrap();
        assert_eq!(
            pe::check(&signed, &k.db.cert).unwrap(),
            Check::Valid,
            "{name}"
        );
        assert_eq!(
            pe::check(&signed, &k.kek.cert).unwrap(),
            Check::OtherSigner,
            "{name}"
        );
        // Signing again replaces the signature rather than adding one.
        let again = pe::sign(&mut blind(), &signed, &k.db).unwrap();
        assert_eq!(again.len(), signed.len(), "{name}");
        let mut altered = signed.clone();
        altered[0x200] ^= 1;
        assert_eq!(
            pe::check(&altered, &k.db.cert).unwrap(),
            Check::Altered,
            "{name}"
        );
        {
            let path = dir.join(format!("{name}.signed.efi"));
            std::fs::write(&path, &signed).unwrap();
            let (ok, said) = run(&sbverify, &["--cert", &db, path.to_str().unwrap()]).unwrap();
            assert!(ok, "sbverify {name}:\n{said}");
            let bad = dir.join(format!("{name}.altered.efi"));
            std::fs::write(&bad, &altered).unwrap();
            let (ok, _) = run(&sbverify, &["--cert", &db, bad.to_str().unwrap()]).unwrap();
            assert!(!ok, "sbverify took an altered {name}");
        }
    }
}

/// A signed image whose certificate table says it is shorter than its own
/// 8-byte header is not read as a signature, and checking it does not
/// stop the program (a stick's file is anyone's to damage).
#[test]
fn an_image_whose_signature_table_is_shorter_than_its_header_reads_as_unreadable() {
    let k = keys();
    let mut image = pe::sign(&mut blind(), &tiny_pe(), &k.db).unwrap();
    // The security data directory is entry 4 of the optional header's 16:
    // its file offset is where the certificate table starts.
    let entry = 0x44 + 20 + 112 + 4 * 8;
    let table = u32::from_le_bytes(image[entry..entry + 4].try_into().unwrap()) as usize;
    assert_ne!(table, 0, "the signed image has a certificate table");
    for length in [0u32, 4, 7] {
        image[table..table + 4].copy_from_slice(&length.to_le_bytes());
        assert_eq!(
            pe::check(&image, &k.db.cert).unwrap(),
            Check::Unreadable,
            "length {length}"
        );
    }
}

/// An image whose certificate table is said to start inside its own
/// headers is not an image: it is not hashed, checked or signed, and
/// signing it does not stop the program (the release fuzz run's pe_image
/// crash: signing cut the image there and then wrote the directory entry
/// past its end).
#[test]
fn an_image_whose_signature_table_starts_inside_its_headers_is_refused() {
    let k = keys();
    let entry = 0x44 + 20 + 112 + 4 * 8;
    for image in [
        tiny_pe(),
        pe::sign(&mut blind(), &tiny_pe(), &k.db).unwrap(),
    ] {
        for at in [0x4du32, 0x40, entry as u32 + 8, 0x1ff] {
            let mut image = image.clone();
            image[entry..entry + 4].copy_from_slice(&at.to_le_bytes());
            image[entry + 4..entry + 8].copy_from_slice(&16u32.to_le_bytes());
            assert!(pe::image_hash(&image).is_err(), "at {at:#x}");
            assert!(pe::check(&image, &k.db.cert).is_err(), "at {at:#x}");
            assert!(pe::sign(&mut blind(), &image, &k.db).is_err(), "at {at:#x}");
        }
    }
}

/// Blinding changes how the key's operation runs, never its result:
/// PKCS #1 v1.5 signatures are deterministic, so a signature made under
/// any blinding stream is the very bytes RustCrypto's unblinded signer
/// gives, and `openssl` gives, for the same key and message; and a signed
/// image is the same file whatever stream blinded it.
#[test]
fn blinding_leaves_the_signature_as_it_was() {
    use rsa::pkcs1v15::SigningKey;
    use rsa::pkcs8::DecodePrivateKey;
    use rsa::signature::{SignatureEncoding, Signer};

    let k = keys();
    let message = b"Faraday signs this with its db key";
    let a = faraday_sb::x509::sign(&mut Stream::new(&[1; 32], 0), &k.db.key, message).unwrap();
    let b = faraday_sb::x509::sign(&mut Stream::new(&[2; 32], 7), &k.db.key, message).unwrap();
    let plain = SigningKey::<Sha256>::new(rsa::RsaPrivateKey::from_pkcs8_der(&k.db.key).unwrap())
        .sign(message)
        .to_vec();
    assert_eq!(a.len(), 256);
    assert_eq!(a, plain);
    assert_eq!(b, plain);

    let dir = scratch("blinding");
    let key = dir.join("db.key.der");
    let msg = dir.join("message");
    std::fs::write(&key, &k.db.key[..]).unwrap();
    std::fs::write(&msg, message).unwrap();
    let out = Command::new("openssl")
        .args([
            "dgst",
            "-sha256",
            "-keyform",
            "DER",
            "-sign",
            key.to_str().unwrap(),
            msg.to_str().unwrap(),
        ])
        .output();
    match out {
        Ok(o) if o.status.success() => assert_eq!(o.stdout, plain),
        Ok(o) => panic!("openssl: {}", String::from_utf8_lossy(&o.stderr)),
        Err(e) => panic!("openssl is not installed ({e}); see docs/DEVELOPING.md"),
    }

    let image = tiny_pe();
    let one = pe::sign(&mut Stream::new(&[3; 32], 0), &image, &k.db).unwrap();
    let two = pe::sign(&mut Stream::new(&[4; 32], 0), &image, &k.db).unwrap();
    assert_eq!(one, two);
}

/// What the Starter reads from a boot file a person signed
/// (`docs/STARTER.md` §4.5): the image hash it had before it was signed,
/// whatever its length, and the certificate whose key signed it, read
/// from the signature itself.
#[test]
fn a_signed_image_names_its_signer_and_keeps_the_image_hash_it_had_unsigned() {
    let k = keys();
    // One whose length is a multiple of eight, and one signing pads.
    let mut odd = tiny_pe();
    odd.extend_from_slice(&[0x11, 0x22, 0x33]);
    for image in [tiny_pe(), odd] {
        let signed = pe::sign(&mut blind(), &image, &k.db).unwrap();
        assert_eq!(
            pe::image_hash(&signed).unwrap(),
            pe::hash_as_signed(&image).unwrap()
        );
        assert_eq!(
            pe::signer(&signed).unwrap(),
            pe::Signer::By(k.db.cert.clone())
        );
        // Signed again with another key: the same program, another signer.
        let other = pe::sign(&mut blind(), &signed, &k.kek).unwrap();
        assert_eq!(
            pe::image_hash(&other).unwrap(),
            pe::hash_as_signed(&image).unwrap()
        );
        assert_eq!(
            pe::signer(&other).unwrap(),
            pe::Signer::By(k.kek.cert.clone())
        );
        assert_eq!(pe::signer(&image).unwrap(), pe::Signer::Unsigned);
        // A byte of its code changed after signing: the signature no
        // longer holds, and the image hash is another program's.
        let mut altered = signed.clone();
        altered[0x200] ^= 1;
        assert_eq!(pe::signer(&altered).unwrap(), pe::Signer::Broken);
        assert_ne!(
            pe::image_hash(&altered).unwrap(),
            pe::hash_as_signed(&image).unwrap()
        );
    }
    assert_eq!(pe::signer(b"not an image"), Err(faraday_sb::Error::Image));
}

/// The security directory entry: its file offset, then its size.
const CERT_DIR: usize = 0x44 + 20 + 112 + 4 * 8;

fn set_dir(image: &mut [u8], at: usize, size: usize) {
    image[CERT_DIR..CERT_DIR + 4].copy_from_slice(&(at as u32).to_le_bytes());
    image[CERT_DIR + 4..CERT_DIR + 8].copy_from_slice(&(size as u32).to_le_bytes());
}

fn dir(image: &[u8]) -> (usize, usize) {
    let n = |at: usize| u32::from_le_bytes(image[at..at + 4].try_into().unwrap()) as usize;
    (n(CERT_DIR), n(CERT_DIR + 4))
}

/// The release's image with a signed copy's signature on it, built from
/// the release's bytes: taken only when the copy is the same program
/// with one signature alone at its end.
#[test]
fn a_signature_is_moved_onto_the_releases_bytes_only_when_it_is_alone_at_the_end() {
    let k = keys();
    let release = tiny_pe();
    let signed = pe::sign(&mut blind(), &release, &k.db).unwrap();
    let built = pe::transplant(&release, &signed).unwrap();
    assert_eq!(built, signed);
    // sbverify takes the moved signature on the release's bytes, and not
    // on the release's bytes with one code byte changed.
    let tmp = scratch("transplant");
    let sbverify = std::env::var("FARADAY_SBVERIFY").unwrap_or_else(|_| "sbverify".to_string());
    let db = pem(&tmp, "db", &k.db.cert).expect("openssl reads the db certificate");
    let path = tmp.join("moved.efi");
    std::fs::write(&path, &built).unwrap();
    let (ok, said) = run(&sbverify, &["--cert", &db, path.to_str().unwrap()]).unwrap();
    assert!(ok, "sbverify the moved signature:\n{said}");
    let mut changed = built.clone();
    changed[0x200] ^= 1;
    std::fs::write(&path, &changed).unwrap();
    let (ok, _) = run(&sbverify, &["--cert", &db, path.to_str().unwrap()]).unwrap();
    assert!(
        !ok,
        "sbverify took the moved signature on a changed program"
    );
    assert_eq!(
        pe::signer(&built).unwrap(),
        pe::Signer::By(k.db.cert.clone())
    );
    assert_eq!(
        pe::transplant(&release, &release),
        Err(pe::Transplant::Unsigned)
    );
    let mut other = release.clone();
    other[0x200] = 0x90;
    let other = pe::sign(&mut blind(), &other, &k.db).unwrap();
    assert_eq!(pe::transplant(&release, &other), Err(pe::Transplant::Other));
    assert_eq!(
        pe::transplant(&release, b"not an image"),
        Err(pe::Transplant::Other)
    );

    // Bytes appended after the table.
    let mut appended = signed.clone();
    appended.extend_from_slice(&[0u8; 16]);
    assert_eq!(
        pe::transplant(&release, &appended),
        Err(pe::Transplant::Layout)
    );
    // Two signatures in the table: the test db key's and another key's.
    let (at, size) = dir(&signed);
    let second = pe::sign(&mut blind(), &release, &k.kek).unwrap();
    let (at2, size2) = dir(&second);
    let mut two = signed.clone();
    two.extend_from_slice(&second[at2..at2 + size2]);
    set_dir(&mut two, at, size + size2);
    assert_eq!(pe::transplant(&release, &two), Err(pe::Transplant::Layout));
    // The table not where the program ends: bytes put before it.
    let mut moved = signed[..at].to_vec();
    moved.extend_from_slice(&[0u8; 8]);
    moved.extend_from_slice(&signed[at..]);
    set_dir(&mut moved, at + 8, size);
    assert!(pe::transplant(&release, &moved).is_err());
    // A table that does not end the file, its entry pointing short.
    let mut short = signed.clone();
    set_dir(&mut short, at, size - 8);
    assert!(pe::transplant(&release, &short).is_err());
}

/// Images signed by other implementations (`tests/vectors/make.sh`:
/// sbsigntool's sbsign and osslsigncode, under a test key whose
/// certificate is `cert.der`) are read as theirs: the image hash Faraday
/// gives the unsigned program, the signer named, the signature checked,
/// and the signature moved onto the unsigned program's bytes gives the
/// same file.
#[test]
fn images_signed_by_sbsign_and_osslsigncode_are_read_and_their_signer_named() {
    let tiny = include_bytes!("vectors/tiny.efi");
    let cert = include_bytes!("vectors/cert.der");
    assert_eq!(&tiny[..], tiny_pe().as_slice());
    assert_eq!(
        faraday_sb::common_name(cert).as_deref(),
        Some("Test sbsign db")
    );
    for (tool, signed) in [
        ("sbsign", &include_bytes!("vectors/tiny.sbsign.efi")[..]),
        (
            "osslsigncode",
            &include_bytes!("vectors/tiny.osslsigncode.efi")[..],
        ),
    ] {
        assert_eq!(
            pe::image_hash(signed).unwrap(),
            pe::hash_as_signed(tiny).unwrap(),
            "{tool}"
        );
        assert_eq!(pe::check(signed, cert).unwrap(), Check::Valid, "{tool}");
        assert_eq!(
            pe::signer(signed).unwrap(),
            pe::Signer::By(cert.to_vec()),
            "{tool}"
        );
        assert_eq!(pe::transplant(tiny, signed).unwrap(), signed, "{tool}");
        let mut altered = signed.to_vec();
        altered[0x200] ^= 1;
        assert_eq!(pe::signer(&altered).unwrap(), pe::Signer::Broken, "{tool}");
    }
}

fn unhex(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}

/// The generator key generation draws from is ChaCha20 as RFC 8439
/// defines it, block counter from 0 and the purpose byte first in the
/// nonce (F158). The all-zero stream is RFC 8439 Appendix A.1's test
/// vectors #1 and #2 (blocks 0 and 1 of a zero key and nonce); the
/// others were computed with Python `cryptography`'s ChaCha20 (OpenSSL)
/// over the nonce `what ‖ 0^11` and a counter of 0.
#[test]
fn the_generator_is_rfc_8439_chacha20_keyed_by_the_seed() {
    use rand_core::TryRng;
    let rfc = unhex(concat!(
        "76b8e0ada0f13d90405d6ae55386bd28bdd219b8a08ded1aa836efcc8b770dc7",
        "da41597c5157488d7724e03fb8d84a376a43b8f41518a11cc387b669b2ee6586",
        "9f07e7be5551387a98ba977c732d080dcb0f29a048e3656912c6533e32ee7aed",
        "29b721769ce64e43d57133b074d839d531ed1f28510afb45ace10a1f4b794d6f",
    ));
    let mut zero = vec![0xffu8; 128];
    Stream::new(&[0; 32], 0).fill(&mut zero);
    assert_eq!(zero, rfc);
    // The same stream read as numbers: little-endian, in order.
    let mut s = Stream::new(&[0; 32], 0);
    assert_eq!(s.try_next_u32().unwrap(), 0xade0_b876);
    assert_eq!(s.try_next_u64().unwrap(), 0xe56a_5d40_903d_f1a0);
    let mut rest = [0u8; 4];
    s.try_fill_bytes(&mut rest).unwrap();
    assert_eq!(rest[..], rfc[12..16]);

    let mut key_gen = [0u8; 64];
    Stream::new(&[0x42; 32], 1).fill(&mut key_gen);
    assert_eq!(
        key_gen[..],
        unhex(concat!(
            "8d764e82caa66707597fcb8c3ec64a05d5fd7d8b2d3058cbaab036827b63f3c6",
            "826eb9c84485464440cd34e36a55e72beb7853e5df7b2238db815ae7c1c9e424",
        ))[..]
    );
    let seed: [u8; 32] = core::array::from_fn(|i| i as u8);
    let mut blinding = [0u8; 16];
    Stream::new(&seed, 2).fill(&mut blinding);
    assert_eq!(blinding[..], unhex("2810192032f347083ad97a7ab7d70d64")[..]);
}

/// Every private key made is one `openssl pkey -check` accepts: p and q
/// prime, n = pq, and d the inverse of e.
#[test]
fn openssl_checks_every_private_key_made() {
    let dir = scratch("pkey");
    let k = keys();
    for (name, pair) in [("pk", &k.pk), ("kek", &k.kek), ("db", &k.db)] {
        let path = dir.join(format!("{name}.key.der"));
        std::fs::write(&path, &pair.key[..]).unwrap();
        let (ok, said) = run(
            "openssl",
            &[
                "pkey",
                "-inform",
                "DER",
                "-in",
                path.to_str().unwrap(),
                "-check",
                "-noout",
            ],
        )
        .unwrap();
        assert!(ok, "{name}: {said}");
        assert!(said.contains("valid"), "{name}: {said}");
    }
}
