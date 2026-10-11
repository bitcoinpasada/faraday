//! What Faraday's Secure Boot files are, judged by other tools
//! (`PLAN.md` §8): `openssl` reads the certificates and verifies the
//! PKCS #7 signature in each `.auth` update; `sbverify` (sbsigntool)
//! verifies a signed image against the db certificate; and Faraday's own
//! check tells a signed image from an altered one and from one signed by
//! another key. A tool not on PATH (`sbverify` also as `FARADAY_SBVERIFY`)
//! is said and its check skipped.

use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::process::Command;

use faraday_sb::{Check, Keys, Policy, efi, enrolment, make, pe};
use sha2::{Digest, Sha256};

const NOW: u64 = 1_791_000_000;

fn keys() -> Keys {
    make(&[0x42; 32], "Test", NOW).unwrap()
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
    let out = Command::new(cmd).args(args).output().ok()?;
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
    let Some(pk) = pem(&dir, "pk", &k.pk.cert) else {
        eprintln!("openssl is not installed: skipped");
        return;
    };
    let kek = pem(&dir, "kek", &k.kek.cert).unwrap();
    for c in [&pk, &kek] {
        let (ok, said) = run("openssl", &["verify", "-check_ss_sig", "-CAfile", c, c]).unwrap();
        assert!(ok, "{said}");
    }
    let files = enrolment(&k, Policy::Windows, NOW).unwrap();
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
    let db = pem(&dir, "db", &k.db.cert);
    for (name, image) in images {
        assert_eq!(pe::check(&image, &k.db.cert).unwrap(), Check::Unsigned);
        let signed = pe::sign(&image, &k.db).unwrap();
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
        let again = pe::sign(&signed, &k.db).unwrap();
        assert_eq!(again.len(), signed.len(), "{name}");
        let mut altered = signed.clone();
        altered[0x200] ^= 1;
        assert_eq!(
            pe::check(&altered, &k.db.cert).unwrap(),
            Check::Altered,
            "{name}"
        );
        if let Some(db) = &db {
            let path = dir.join(format!("{name}.signed.efi"));
            std::fs::write(&path, &signed).unwrap();
            match run(&sbverify, &["--cert", db, path.to_str().unwrap()]) {
                Some((ok, said)) => assert!(ok, "sbverify {name}:\n{said}"),
                None => eprintln!("sbverify is not installed: skipped"),
            }
            let bad = dir.join(format!("{name}.altered.efi"));
            std::fs::write(&bad, &altered).unwrap();
            if let Some((ok, _)) = run(&sbverify, &["--cert", db, bad.to_str().unwrap()]) {
                assert!(!ok, "sbverify took an altered {name}");
            }
        }
    }
}
