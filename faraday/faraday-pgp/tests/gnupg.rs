//! What Faraday writes, judged by GnuPG and paperkey (`PLAN.md` §7):
//! `gpg` imports the certificate with both keys and the user ID, verifies
//! detached signatures over SHA-256 and SHA-512 and rejects one over
//! other data, and takes the revocation certificate; `paperkey`, given the
//! secret key, prints the same lines as `Key::paperkey`.
//!
//! The tools are found on PATH (`paperkey` also as `FARADAY_PAPERKEY`); a
//! machine without one says so and skips that check.

use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::process::Command;

use faraday_pgp::{Armor, Hash, Key, Reason, armor};

/// May 2026: before any clock this runs on.
const MADE: u32 = 1_780_000_000;

fn key() -> Key {
    Key {
        primary_seed: [0x11; 32],
        created: MADE,
        subkey_seed: [0x22; 32],
        subkey_created: MADE,
        user_ids: vec!["Test Person <test@example.com>".to_string()],
        expiry: 2 * 365 * 24 * 3600,
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

fn gpg(home: &Path, args: &[&str]) -> Option<(bool, String)> {
    let out = Command::new("gpg")
        .arg("--homedir")
        .arg(home)
        .args(["--batch", "--no-tty", "--status-fd", "1"])
        .args(args)
        .output()
        .ok()?;
    let text =
        String::from_utf8_lossy(&out.stdout).into_owned() + &String::from_utf8_lossy(&out.stderr);
    Some((out.status.success(), text))
}

#[test]
fn gnupg_takes_the_certificate_signatures_and_revocation() {
    let home = scratch("gpg");
    let k = key();
    let cert = home.join("cert.asc");
    std::fs::write(&cert, armor(Armor::PublicKey, &k.certificate(MADE))).unwrap();
    let Some((ok, said)) = gpg(&home, &["--import", cert.to_str().unwrap()]) else {
        eprintln!("gpg is not installed: the GnuPG check is skipped");
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
        std::fs::write(
            &sig,
            armor(
                Armor::Signature,
                &k.sign(b"A file Faraday signs.\n", MADE + 60, hash),
            ),
        )
        .unwrap();
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
    std::fs::write(
        &rev,
        armor(
            Armor::PublicKey,
            &k.revocation(MADE + 120, Reason::Retired, "Not used"),
        ),
    )
    .unwrap();
    let (ok, said) = gpg(&home, &["--import", rev.to_str().unwrap()]).unwrap();
    assert!(ok, "gpg refused the revocation:\n{said}");
    let (_, listing) = gpg(&home, &["--with-colons", "--list-keys"]).unwrap();
    assert!(
        listing.lines().any(|l| l.starts_with("pub:r:")),
        "not revoked:\n{listing}"
    );
}

#[test]
fn paperkey_prints_the_same_lines() {
    let paperkey = std::env::var("FARADAY_PAPERKEY").unwrap_or_else(|_| "paperkey".to_string());
    let dir = scratch("paperkey");
    let k = key();
    let secret = dir.join("secret.gpg");
    std::fs::write(&secret, k.secret_key(MADE)).unwrap();
    let Ok(out) = Command::new(&paperkey)
        .arg("--secret-key")
        .arg(&secret)
        .output()
    else {
        eprintln!("paperkey is not installed: the paperkey check is skipped");
        return;
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
