//! What Faraday writes, judged by Sequoia's `sq` (ESTABLISHED §9.7, batch
//! X24): `sq` inspects the certificate with both keys, the user ID and
//! the expiry, also once renewed, takes the revocation certificate,
//! verifies detached signatures over SHA-256 and SHA-512 and refuses one
//! over other data.
//!
//! `sq` is found on PATH and must be built over Nettle (`sq version` says
//! "cryptographic backend Nettle"), so that the judge is not Faraday's
//! own backend by another name; a machine without it, or with another
//! backend, fails the test and nothing skips. Every call sets its clock
//! (`--time`), so no date in a test depends on the day it runs, and
//! reads no certificate store or key store.

use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::process::Command;

use faraday_pgp::{Armor, Hash, Key, Reason, armor};

/// May 2026: before any clock this runs on.
const MADE: u32 = 1_780_000_000;
const YEAR: u32 = 31_536_000;

fn key() -> Key {
    Key {
        primary_seed: [0x11; 32],
        created: MADE,
        subkey_seed: [0x22; 32],
        subkey_created: MADE,
        user_ids: vec!["Test Person <test@example.com>".to_string()],
        expiry: 2 * YEAR,
    }
}

fn hex(fp: &[u8; 20]) -> String {
    fp.iter().map(|b| format!("{b:02X}")).collect()
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
    let dir = std::env::temp_dir().join(format!("faraday-pgp-sq-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    Scratch(dir)
}

fn write(path: &Path, text: &str) {
    std::fs::write(path, text).unwrap();
}

/// `sq` with its clock at `now`, reading no stores. Fails the test when
/// `sq` is not there or is not built over Nettle.
fn sq_at(home: &Path, now: u32, args: &[&str]) -> (bool, String) {
    backend();
    run(home, Some(now), args)
}

fn run(home: &Path, now: Option<u32>, args: &[&str]) -> (bool, String) {
    let mut cmd = Command::new("sq");
    cmd.arg("--home")
        .arg(home)
        .args(["--cert-store=none", "--key-store=none"]);
    if let Some(now) = now {
        cmd.arg("--time").arg(iso(now));
    }
    let out = match cmd.args(args).output() {
        Ok(o) => o,
        Err(e) => panic!("sq is not installed ({e}); see docs/DEVELOPING.md"),
    };
    let text =
        String::from_utf8_lossy(&out.stdout).into_owned() + &String::from_utf8_lossy(&out.stderr);
    (out.status.success(), text)
}

/// `now` as `YYYY-MM-DDThh:mm:ssZ`, the form `--time` takes.
fn iso(now: u32) -> String {
    let (days, secs) = (i64::from(now / 86_400), now % 86_400);
    // Days since 1970-01-01 to a calendar date (Hinnant's civil_from_days).
    let z = days + 719_468;
    let era = z / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        secs / 3600,
        secs % 3600 / 60,
        secs % 60
    )
}

/// The judge is Sequoia over Nettle, the build `docs/ESTABLISHED.md`
/// names.
fn backend() {
    let home = scratch("version");
    let (ok, said) = run(&home, None, &["version"]);
    assert!(ok, "{said}");
    assert!(
        said.contains("cryptographic backend Nettle"),
        "sq must be built over Nettle:\n{said}"
    );
}

fn inspect(home: &Path, now: u32, file: &Path) -> String {
    let (ok, said) = sq_at(home, now, &["inspect", file.to_str().unwrap()]);
    assert!(ok, "sq inspect refused {}:\n{said}", file.display());
    said
}

/// How `sq inspect` writes a time.
fn shown(t: u32) -> String {
    iso(t).replace('T', " ").replace('Z', " UTC")
}

/// `a` and `b` merged into one armoured certificate, as `sq keyring
/// merge` does, written to `out`.
fn merge(home: &Path, a: &Path, b: &Path, out: &Path) {
    let (ok, said) = sq_at(
        home,
        MADE + 3600,
        &[
            "keyring",
            "merge",
            "--output",
            out.to_str().unwrap(),
            a.to_str().unwrap(),
            b.to_str().unwrap(),
        ],
    );
    assert!(ok, "sq keyring merge:\n{said}");
}

fn verify(home: &Path, now: u32, cert: &Path, sig: &Path, data: &Path) -> (bool, String) {
    sq_at(
        home,
        now,
        &[
            "verify",
            "--signer-file",
            cert.to_str().unwrap(),
            "--signature-file",
            sig.to_str().unwrap(),
            data.to_str().unwrap(),
        ],
    )
}

#[test]
fn sequoia_reads_the_certificate() {
    let home = scratch("cert");
    let k = key();
    let cert = home.join("cert.asc");
    write(&cert, &armor(Armor::PublicKey, &k.certificate(MADE)));
    let said = inspect(&home, MADE + 3600, &cert);
    assert!(said.contains("OpenPGP Certificate"), "{said}");
    for fingerprint in [k.fingerprint(), k.subkey_fingerprint()] {
        assert!(said.contains(&hex(&fingerprint)), "{said}");
    }
    assert!(said.contains("Key flags: certification"), "{said}");
    assert!(said.contains("Key flags: signing"), "{said}");
    assert!(
        said.contains("UserID: Test Person <test@example.com>"),
        "{said}"
    );
    // Both keys end with the term, two years after they were made.
    let end = shown(MADE + 2 * YEAR);
    assert_eq!(
        said.matches(&format!("Expiration time: {end}")).count(),
        2,
        "{said}"
    );
    assert!(!said.contains("Revoked"), "{said}");
}

#[test]
fn sequoia_takes_the_revocation_certificate() {
    let home = scratch("revoke");
    let k = key();
    let cert = home.join("cert.asc");
    write(&cert, &armor(Armor::PublicKey, &k.certificate(MADE)));
    let rev = home.join("revoke.asc");
    write(
        &rev,
        &armor(
            Armor::PublicKey,
            &k.revocation(MADE + 120, Reason::Retired, "Not used"),
        ),
    );
    let said = inspect(&home, MADE + 3600, &rev);
    assert!(said.contains("Revocation Certificate"), "{said}");
    assert!(said.contains(&hex(&k.fingerprint())), "{said}");
    assert!(said.contains("Key is retired and no longer used"), "{said}");
    assert!(said.contains("Message: Not used"), "{said}");

    // Merged into the certificate, it revokes the key from its date on.
    let merged = home.join("merged.asc");
    merge(&home, &cert, &rev, &merged);
    let said = inspect(&home, MADE + 3600, &merged);
    assert!(said.contains("Revoked"), "{said}");
}

#[test]
fn sequoia_verifies_the_signatures_and_refuses_other_data() {
    let home = scratch("sign");
    let k = key();
    let cert = home.join("cert.asc");
    write(&cert, &armor(Armor::PublicKey, &k.certificate(MADE)));
    let data = home.join("data.txt");
    std::fs::write(&data, b"A file Faraday signs.\n").unwrap();
    for hash in [Hash::Sha256, Hash::Sha512] {
        let sig = home.join("data.txt.asc");
        write(
            &sig,
            &armor(
                Armor::Signature,
                &k.sign(b"A file Faraday signs.\n", MADE + 60, hash),
            ),
        );
        let (ok, said) = verify(&home, MADE + 3600, &cert, &sig, &data);
        assert!(ok, "{hash:?}:\n{said}");
        assert!(
            said.contains(&format!("signature made by {}", hex(&k.fingerprint()))),
            "{hash:?}:\n{said}"
        );
        std::fs::write(&data, b"Other data.\n").unwrap();
        let (ok, said) = verify(&home, MADE + 3600, &cert, &sig, &data);
        assert!(!ok, "{hash:?} over other data:\n{said}");
        std::fs::write(&data, b"A file Faraday signs.\n").unwrap();
    }
}

/// A key past its term does not verify; a certificate renewed later, with
/// the term counted from then, makes it verify again.
#[test]
fn sequoia_sees_a_key_expire_and_be_renewed() {
    let home = scratch("expiry");
    let k = key();
    let end = MADE + 2 * YEAR;
    let cert = home.join("cert.asc");
    write(&cert, &armor(Armor::PublicKey, &k.certificate(MADE)));
    let data = home.join("data.txt");
    std::fs::write(&data, b"A file Faraday signs.\n").unwrap();
    // A signature is judged by the key as it stood when it was made.
    let sign = |k: &Key, at: u32| {
        let sig = home.join(format!("{at}.asc"));
        write(
            &sig,
            &armor(
                Armor::Signature,
                &k.sign(b"A file Faraday signs.\n", at, Hash::Sha512),
            ),
        );
        sig
    };
    let (ok, said) = verify(&home, end, &cert, &sign(&k, end - 100), &data);
    assert!(ok, "{said}");
    let (ok, said) = verify(&home, end + 200, &cert, &sign(&k, end + 50), &data);
    assert!(!ok, "made past its term:\n{said}");

    // Renewed at end + 100 for a year from then.
    let now = end + 100;
    let mut renewed = key();
    renewed.expiry = now - MADE + YEAR;
    let cert2 = home.join("cert2.asc");
    write(&cert2, &armor(Armor::PublicKey, &renewed.certificate(now)));
    let merged = home.join("merged.asc");
    merge(&home, &cert, &cert2, &merged);
    let said = inspect(&home, now + 10, &merged);
    assert!(
        said.contains(&format!("Expiration time: {}", shown(now + YEAR))),
        "{said}"
    );
    let (ok, said) = verify(&home, now + 20, &merged, &sign(&renewed, now + 10), &data);
    assert!(ok, "valid again:\n{said}");
    let late = sign(&renewed, now + YEAR + 10);
    let (ok, said) = verify(&home, now + YEAR + 20, &merged, &late, &data);
    assert!(!ok, "past the renewed term:\n{said}");
}
