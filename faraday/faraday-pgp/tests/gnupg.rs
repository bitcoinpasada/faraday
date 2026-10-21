//! What Faraday writes, judged by GnuPG and paperkey (`docs/history/PLAN.md` §7):
//! `gpg` imports the certificate with both keys and the user ID, verifies
//! detached signatures over SHA-256 and SHA-512 and rejects one over
//! other data, takes the revocation certificate, sees a key as expired
//! and valid again once renewed, and refuses a `.rev` file until its
//! colon is taken out; `paperkey`, given the secret key, prints the same
//! lines as `Key::paperkey`.
//!
//! The tools are found on PATH (`paperkey` also as `FARADAY_PAPERKEY`). A
//! machine without one fails the test, so that a check never passes by
//! not running; `FARADAY_SKIP_GNUPG=1` skips them on purpose. Every call
//! to `gpg` sets its clock (`--faked-system-time`), so no date in a test
//! depends on the day it runs.

use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::process::Command;

use faraday_pgp::{Armor, Hash, Key, Reason, armor, read_public_armor};

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
    let dir = std::env::temp_dir().join(format!("faraday-pgp-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700)).unwrap();
    }
    Scratch(dir)
}

/// Whether the tools are to be skipped on purpose.
fn skipped() -> bool {
    std::env::var("FARADAY_SKIP_GNUPG").is_ok_and(|v| v == "1")
}

/// `gpg` with its clock at `now`, in `home`. Fails the test when `gpg`
/// is not there, unless the check is skipped on purpose.
fn gpg_at(home: &Path, now: u32, args: &[&str]) -> Option<(bool, String)> {
    let out = Command::new("gpg")
        .arg("--homedir")
        .arg(home)
        .args(["--batch", "--no-tty", "--status-fd", "1"])
        .arg("--faked-system-time")
        .arg(format!("{now}!"))
        .args(args)
        .output();
    let out = match out {
        Ok(o) => o,
        Err(_) if skipped() => {
            eprintln!("gpg is not installed: skipped, FARADAY_SKIP_GNUPG=1");
            return None;
        }
        Err(e) => panic!("gpg is not installed ({e}); set FARADAY_SKIP_GNUPG=1 to skip"),
    };
    let text =
        String::from_utf8_lossy(&out.stdout).into_owned() + &String::from_utf8_lossy(&out.stderr);
    Some((out.status.success(), text))
}

fn gpg(home: &Path, args: &[&str]) -> Option<(bool, String)> {
    gpg_at(home, MADE + 3600, args)
}

fn write(path: &Path, text: &str) {
    std::fs::write(path, text).unwrap();
}

/// The `pub:` line of `gpg --list-keys` at `now`.
fn pub_line(home: &Path, now: u32) -> String {
    let (_, listing) = gpg_at(home, now, &["--with-colons", "--list-keys"]).unwrap();
    listing
        .lines()
        .find(|l| l.starts_with("pub:"))
        .unwrap_or_else(|| panic!("no pub line:\n{listing}"))
        .to_string()
}

#[test]
fn gnupg_takes_the_certificate_signatures_and_revocation() {
    let home = scratch("gpg");
    let k = key();
    let cert = home.join("cert.asc");
    write(&cert, &armor(Armor::PublicKey, &k.certificate(MADE)));
    let Some((ok, said)) = gpg(&home, &["--import", cert.to_str().unwrap()]) else {
        return;
    };
    assert!(ok, "gpg --import refused the certificate:\n{said}");
    let (_, listing) = gpg(&home, &["--with-colons", "--list-keys"]).unwrap();
    assert!(
        listing.contains(&format!("fpr:::::::::{}:", hex(&k.fingerprint()))),
        "{listing}"
    );
    assert!(
        listing.contains(&format!("fpr:::::::::{}:", hex(&k.subkey_fingerprint()))),
        "{listing}"
    );
    assert!(
        listing.contains("Test Person <test@example.com>"),
        "{listing}"
    );

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
        let (ok, said) = gpg(
            &home,
            &["--verify", sig.to_str().unwrap(), data.to_str().unwrap()],
        )
        .unwrap();
        assert!(
            ok && said.contains("GOODSIG") && said.contains("VALIDSIG"),
            "{hash:?}:\n{said}"
        );
        std::fs::write(&data, b"Other data.\n").unwrap();
        let (ok, said) = gpg(
            &home,
            &["--verify", sig.to_str().unwrap(), data.to_str().unwrap()],
        )
        .unwrap();
        assert!(
            !ok && said.contains("BADSIG"),
            "{hash:?} over other data:\n{said}"
        );
        std::fs::write(&data, b"A file Faraday signs.\n").unwrap();
    }

    let rev = home.join("revoke.asc");
    write(
        &rev,
        &armor(
            Armor::PublicKey,
            &k.revocation(MADE + 120, Reason::Retired, "Not used"),
        ),
    );
    let (ok, said) = gpg(&home, &["--import", rev.to_str().unwrap()]).unwrap();
    assert!(ok, "gpg refused the revocation:\n{said}");
    assert!(pub_line(&home, MADE + 3600).starts_with("pub:r:"));
}

/// Signatures whose r or s begins with a zero byte are written with a
/// shorter MPI: GnuPG must still verify them.
#[test]
fn a_signature_whose_r_or_s_starts_with_a_zero_byte_verifies() {
    let k = key();
    let data = b"A file Faraday signs.\n";
    // The two MPIs at the end of a version-4 signature packet's body.
    let mpis = |packet: &[u8]| -> (usize, usize) {
        let body = &packet[2..];
        let hashed = u16::from_be_bytes([body[4], body[5]]) as usize;
        let at = 6 + hashed;
        let unhashed = u16::from_be_bytes([body[at], body[at + 1]]) as usize;
        let at = at + 2 + unhashed + 2;
        let r = (u16::from_be_bytes([body[at], body[at + 1]]) as usize).div_ceil(8);
        let s_at = at + 2 + r;
        let s = (u16::from_be_bytes([body[s_at], body[s_at + 1]]) as usize).div_ceil(8);
        (r, s)
    };
    let (at, sig) = (MADE..MADE + 100_000)
        .map(|t| (t, k.sign(data, t, Hash::Sha512)))
        .find(|(_, sig)| {
            let (r, s) = mpis(sig);
            r < 32 || s < 32
        })
        .expect("a signature with a short r or s in 100,000 tries");
    let home = scratch("short-sig");
    let cert = home.join("cert.asc");
    write(&cert, &armor(Armor::PublicKey, &k.certificate(MADE)));
    let Some((ok, said)) = gpg_at(&home, at + 60, &["--import", cert.to_str().unwrap()]) else {
        return;
    };
    assert!(ok, "{said}");
    let sigfile = home.join("data.asc");
    write(&sigfile, &armor(Armor::Signature, &sig));
    let datafile = home.join("data.txt");
    std::fs::write(&datafile, data).unwrap();
    let (ok, said) = gpg_at(
        &home,
        at + 60,
        &[
            "--verify",
            sigfile.to_str().unwrap(),
            datafile.to_str().unwrap(),
        ],
    )
    .unwrap();
    assert!(
        ok && said.contains("GOODSIG"),
        "signature at {at} with a short MPI:\n{said}"
    );
}

/// A key past its term is expired to GnuPG; a certificate renewed later,
/// with the term counted from then, makes it valid again, as `gpg
/// --quick-set-expire` does.
#[test]
fn gnupg_sees_a_key_expire_and_be_renewed() {
    let home = scratch("expiry");
    let k = key();
    let end = MADE + 2 * YEAR;
    let cert = home.join("cert.asc");
    write(&cert, &armor(Armor::PublicKey, &k.certificate(MADE)));
    let Some((ok, said)) = gpg_at(&home, MADE + 3600, &["--import", cert.to_str().unwrap()]) else {
        return;
    };
    assert!(ok, "{said}");
    let line = pub_line(&home, MADE + 3600);
    assert!(line.starts_with("pub:-:"), "{line}");
    assert!(
        line.contains(&format!(":{end}:")),
        "expires at {end}: {line}"
    );
    let line = pub_line(&home, end + 100);
    assert!(line.starts_with("pub:e:"), "expired: {line}");

    // Renewed at end + 100 for a year from then.
    let now = end + 100;
    let mut renewed = key();
    renewed.expiry = now - MADE + YEAR;
    let cert2 = home.join("cert2.asc");
    write(&cert2, &armor(Armor::PublicKey, &renewed.certificate(now)));
    let (ok, said) = gpg_at(&home, now, &["--import", cert2.to_str().unwrap()]).unwrap();
    assert!(ok, "{said}");
    let line = pub_line(&home, now + 10);
    assert!(line.starts_with("pub:-:"), "valid again: {line}");
    assert!(
        line.contains(&format!(":{}:", now + YEAR)),
        "expires a year from the renewal: {line}"
    );
    // The signing subkey follows.
    let (_, listing) = gpg_at(&home, now + 10, &["--with-colons", "--list-keys"]).unwrap();
    assert!(
        listing.lines().any(|l| l.starts_with("sub:-:")),
        "{listing}"
    );
}

/// The file layout GnuPG 2.4 writes for `openpgp-revocs.d/<FPR>.rev`: its
/// colon keeps `gpg --import` from taking it until a person removes it.
#[test]
fn gnupg_refuses_a_rev_file_until_its_colon_is_removed() {
    let home = scratch("rev");
    let k = key();
    let cert = home.join("cert.asc");
    write(&cert, &armor(Armor::PublicKey, &k.certificate(MADE)));
    let Some((ok, said)) = gpg(&home, &["--import", cert.to_str().unwrap()]) else {
        return;
    };
    assert!(ok, "{said}");
    let file = k.revocation_file(&k.revocation(MADE + 120, Reason::Superseded, ""));
    assert!(file.contains("\n:-----BEGIN PGP PUBLIC KEY BLOCK-----\n"));
    assert!(file.starts_with("This is a revocation certificate for the OpenPGP key:"));
    let rev = home.join("key.rev");
    write(&rev, &file);
    let (ok, said) = gpg(&home, &["--import", rev.to_str().unwrap()]).unwrap();
    assert!(
        !ok || !said.contains("revocation certificate imported"),
        "{said}"
    );
    assert!(pub_line(&home, MADE + 3600).starts_with("pub:-:"));
    write(&rev, &file.replacen("\n:-----BEGIN", "\n-----BEGIN", 1));
    let (ok, said) = gpg(&home, &["--import", rev.to_str().unwrap()]).unwrap();
    assert!(ok, "{said}");
    assert!(pub_line(&home, MADE + 3600).starts_with("pub:r:"));
}

/// A key made by Faraday and one made by GnuPG list the same way: the
/// text before the colon, lines for the key and each user ID.
#[test]
fn the_rev_file_lists_the_key_as_gnupg_does() {
    let k = key();
    let file = k.revocation_file(&k.revocation(MADE, Reason::Unspecified, ""));
    let want = format!(
        "pub   ed25519 2026-05-28 [C] [expires: 2028-05-27]\n      {}\nuid          Test Person <test@example.com>\n",
        hex(&k.fingerprint())
    );
    assert!(file.contains(&want), "{file}");
}

#[test]
fn paperkey_prints_the_same_lines() {
    let paperkey = std::env::var("FARADAY_PAPERKEY").unwrap_or_else(|_| "paperkey".to_string());
    // A seed with a leading zero byte has a shorter secret MPI.
    let mut k = key();
    k.primary_seed[0] = 0;
    k.subkey_seed[0] = 0;
    k.subkey_seed[1] = 0;
    for k in [key(), k] {
        let dir = scratch("paperkey");
        let secret = dir.join("secret.gpg");
        std::fs::write(&secret, &*k.secret_key(MADE)).unwrap();
        let out = match Command::new(&paperkey)
            .arg("--secret-key")
            .arg(&secret)
            .output()
        {
            Ok(o) => o,
            Err(_) if skipped() => {
                eprintln!("paperkey is not installed: skipped, FARADAY_SKIP_GNUPG=1");
                return;
            }
            Err(e) => panic!("paperkey is not installed ({e}); set FARADAY_SKIP_GNUPG=1 to skip"),
        };
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        let theirs: String = String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
            .map(|l| format!("{}\n", l.trim_end()))
            .collect();
        let ours: String = k
            .paperkey()
            .lines()
            .map(|l| format!("{}\n", l.trim_end()))
            .collect();
        assert_eq!(ours, theirs);
    }
}

#[test]
fn what_gnupg_writes_reads_as_public_and_its_secret_key_does_not() {
    let home = scratch("gpg-armor");
    let Some((ok, said)) = gpg(
        &home,
        &[
            "--passphrase",
            "",
            "--pinentry-mode",
            "loopback",
            "--quick-gen-key",
            "Other Person <other@example.com>",
            "ed25519",
            "sign",
            "never",
        ],
    ) else {
        return;
    };
    assert!(ok, "gpg could not make a key:\n{said}");
    let export = |args: &[&str]| -> Vec<u8> {
        let out = Command::new("gpg")
            .arg("--homedir")
            .arg(&*home)
            .args(["--batch", "--no-tty", "--pinentry-mode", "loopback"])
            .args(["--passphrase", ""])
            .args(args)
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        out.stdout
    };
    let public = export(&["--armor", "--export"]);
    assert!(!public.is_empty());
    assert_eq!(read_public_armor(&public), Some(Armor::PublicKey));

    let data = home.join("data.txt");
    std::fs::write(&data, b"Signed by GnuPG.\n").unwrap();
    let sig = export(&[
        "--armor",
        "--detach-sign",
        "--output",
        "-",
        data.to_str().unwrap(),
    ]);
    assert!(!sig.is_empty());
    assert_eq!(read_public_armor(&sig), Some(Armor::Signature));

    let secret = export(&["--armor", "--export-secret-keys"]);
    assert!(!secret.is_empty());
    assert_eq!(read_public_armor(&secret), None);

    // GnuPG's own revocation file is not public.
    let revs: Vec<_> = std::fs::read_dir(home.join("openpgp-revocs.d"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    assert_eq!(revs.len(), 1);
    let rev = std::fs::read(&revs[0]).unwrap();
    assert_eq!(read_public_armor(&rev), None);
}
