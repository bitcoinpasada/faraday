//! A GPG key restored from its paperkey and its public certificate
//! (`docs/GPG-PAPERKEY.md` §8, tests 1 to 15), judged by paperkey 1.6 and
//! GnuPG 2.4: what Faraday reads of paperkey's lines and raw form is what
//! paperkey reads, a line paperkey refuses Faraday refuses on the same
//! line, paperkey's restore of Faraday's lines is Faraday's secret key,
//! and the certificates GnuPG makes or merges are restored or refused by
//! what they are.
//!
//! The fixtures in `tools/vectors/paperkey/` were made by the tools
//! (`make.sh` there, its README naming each file). The tests that run the
//! tools find them on PATH (`paperkey` also as `FARADAY_PAPERKEY`) and
//! fail without them. Every call to `gpg` sets its clock
//! (`--faked-system-time`).

use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::process::Command;

use faraday_pgp::paperkey::{self, LineError, ReadError};
use faraday_pgp::{Armor, Certificate, Key, Reason, Refusal, armor, crc24, restore};

/// May 2026: before any clock this runs on.
const MADE: u32 = 1_780_000_000;
const YEAR: u32 = 31_536_000;

fn key_with(primary: u8, subkey: u8) -> Key {
    Key {
        primary_seed: [primary; 32],
        created: MADE,
        subkey_seed: [subkey; 32],
        subkey_created: MADE,
        user_ids: vec!["Test Person <test@example.com>".to_string()],
        expiry: 2 * YEAR,
    }
}

/// The key the fixtures were made from (`examples/paperkey_inputs.rs`):
/// seeds with leading zero bytes, so shorter secret MPIs.
fn key() -> Key {
    let mut k = key_with(0x11, 0x22);
    k.primary_seed[0] = 0;
    k.subkey_seed[0] = 0;
    k.subkey_seed[1] = 0;
    k
}

fn vectors() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools/vectors/paperkey")
}

fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(vectors().join(name)).unwrap_or_else(|e| panic!("{name}: {e}"))
}

fn text(name: &str) -> String {
    String::from_utf8(fixture(name)).expect("text")
}

/// A certificate as Faraday writes it, armoured, read for a restore.
fn cert_of(k: &Key, at: u32) -> Certificate {
    Certificate::read(armor(Armor::PublicKey, &k.certificate(at)).as_bytes()).expect("reads")
}

/// The numbered lines of a paperkey's data (the data lines only), each
/// as `Key::paperkey` writes it.
fn data_lines(k: &Key) -> Vec<String> {
    let all = k.paperkey();
    let lines: Vec<String> = all.lines().map(str::to_string).collect();
    lines[..lines.len() - 1].to_vec()
}

/// A secret part as Faraday writes one: usage 0, the MPI, its checksum.
fn secret_part(seed: &[u8; 32]) -> Vec<u8> {
    let start = seed.iter().position(|&b| b != 0).unwrap_or(32);
    let b = &seed[start..];
    let bits = if b.is_empty() {
        0
    } else {
        (b.len() as u16 - 1) * 8 + (8 - b[0].leading_zeros() as u16)
    };
    let mut m = bits.to_be_bytes().to_vec();
    m.extend_from_slice(b);
    let sum = m.iter().fold(0u16, |s, x| s.wrapping_add(u16::from(*x)));
    let mut out = vec![0];
    out.extend(m);
    out.extend(sum.to_be_bytes());
    out
}

/// paperkey's lines of `data`, as `paperkey --secret-key` prints them
/// without its comments.
fn lines_of(data: &[u8]) -> String {
    let mut out = String::new();
    let mut n = 0;
    for chunk in data.chunks(22) {
        n += 1;
        out.push_str(&format!("{n:3}: "));
        for b in chunk {
            out.push_str(&format!("{b:02X} "));
        }
        out.push_str(&format!("{:06X}\n", crc24(chunk)));
    }
    out.push_str(&format!("{:3}: {:06X}\n", n + 1, crc24(data)));
    out
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
    let dir = std::env::temp_dir().join(format!("faraday-paperkey-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700)).unwrap();
    }
    Scratch(dir)
}

/// A tool's run: whether it succeeded, its stdout, and its stderr.
struct Ran {
    ok: bool,
    out: Vec<u8>,
    err: String,
}

/// Runs `tool`; a failure when it is missing.
fn run(tool: &str, cmd: &mut Command) -> Option<Ran> {
    match cmd.output() {
        Ok(o) => Some(Ran {
            ok: o.status.success(),
            out: o.stdout,
            err: String::from_utf8_lossy(&o.stderr).into_owned(),
        }),
        Err(e) => panic!("{tool} is not installed ({e}); see docs/DEVELOPING.md"),
    }
}

fn paperkey_cmd() -> Command {
    Command::new(std::env::var("FARADAY_PAPERKEY").unwrap_or_else(|_| "paperkey".to_string()))
}

/// paperkey's restore of `secrets` (a file) onto the binary certificate
/// `pubring`.
fn paperkey_restore(pubring: &Path, secrets: &Path, extra: &[&str]) -> Option<Ran> {
    run(
        "paperkey",
        paperkey_cmd()
            .arg("--pubring")
            .arg(pubring)
            .arg("--secrets")
            .arg(secrets)
            .args(extra),
    )
}

/// `gpg` in `home` with its clock at `now`.
fn gpg_at(home: &Path, now: u32, args: &[&str]) -> Option<Ran> {
    run(
        "gpg",
        Command::new("gpg")
            .arg("--homedir")
            .arg(home)
            .args(["--batch", "--no-tty", "--pinentry-mode", "loopback"])
            .arg("--faked-system-time")
            .arg(format!("{now}!"))
            .args(args),
    )
}

fn gpg(home: &Path, args: &[&str]) -> Option<Ran> {
    gpg_at(home, MADE + 3600, args)
}

fn hex(fp: &[u8; 20]) -> String {
    fp.iter().map(|b| format!("{b:02X}")).collect()
}

// ---- Against paperkey ----

/// Test 1: the lines Faraday shows, typed back, give the entries the key
/// was made from, for seeds with leading zero bytes.
#[test]
fn the_lines_faraday_shows_read_back_to_the_key() {
    for k in [key(), key_with(0x33, 0x44)] {
        let shown = k.paperkey();
        let data = paperkey::read_text(&shown).expect("Faraday's lines read");
        let entries = paperkey::entries(&data).expect("its entries");
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].fingerprint, k.fingerprint());
        assert_eq!(entries[0].secret, secret_part(&k.primary_seed));
        assert_eq!(entries[1].fingerprint, k.subkey_fingerprint());
        assert_eq!(entries[1].secret, secret_part(&k.subkey_seed));
        // Each line on its own, as typed: with or without its number, the
        // spaces left out, in lower case.
        for (i, l) in shown.lines().enumerate() {
            let n = i as u32 + 1;
            let read = paperkey::line(n, l).expect("a line");
            let bare = l
                .split_once(": ")
                .unwrap()
                .1
                .replace(' ', "")
                .to_lowercase();
            let again = paperkey::line(n, &bare).expect("the same line, typed bare");
            assert_eq!(*read.bytes, *again.bytes);
            assert_eq!(read.crc, again.crc);
        }
        let r = restore(&cert_of(&k, MADE), &data).expect("restored");
        assert_eq!(r.key.primary_seed, k.primary_seed);
        assert_eq!(r.key.subkey_seed, k.subkey_seed);
    }
}

/// Test 2: paperkey 1.6's text and raw output of the secret key read to
/// the same data as Faraday's own lines.
#[test]
fn paperkeys_text_and_raw_output_read_as_faradays_lines() {
    let k = key();
    let ours = paperkey::read_text(&k.paperkey()).expect("ours");
    let text = text("faraday-key.txt");
    assert!(
        text.starts_with("# Secret portions of key "),
        "the fixture keeps its comments"
    );
    let theirs = paperkey::read_text(&text).expect("paperkey's text");
    assert_eq!(*theirs, *ours);
    let raw = paperkey::read_raw(&fixture("faraday-key.raw")).expect("paperkey's raw form");
    assert_eq!(*raw, *ours);
}

/// Test 3: paperkey's restore of the lines Faraday shows is Faraday's
/// secret key, byte for byte.
#[test]
fn paperkey_restores_faradays_lines_to_faradays_secret_key() {
    let k = key();
    let dir = scratch("restore");
    let cert = dir.join("cert.gpg");
    std::fs::write(&cert, k.certificate(MADE)).unwrap();
    let lines = dir.join("lines.txt");
    std::fs::write(&lines, k.paperkey().as_bytes()).unwrap();
    let Some(ran) = paperkey_restore(&cert, &lines, &[]) else {
        return;
    };
    assert!(ran.ok, "{}", ran.err);
    // paperkey writes every packet's header in the old format, where
    // Faraday writes the new: the packets are the same, tag and body.
    assert_eq!(packets(&ran.out), packets(&k.secret_key(MADE)));
    let data = paperkey::read_text(&k.paperkey()).unwrap();
    let r = restore(&cert_of(&k, MADE), &data).expect("Faraday restores it too");
    assert_eq!(*r.key.secret_key(MADE), *k.secret_key(MADE));
}

/// Each packet's tag and body, whichever header format it has.
fn packets(bytes: &[u8]) -> Vec<(u8, Vec<u8>)> {
    let mut out = Vec::new();
    let mut at = 0;
    while at < bytes.len() {
        let first = bytes[at];
        let b = |n: usize| bytes[at + n] as usize;
        let (tag, head, len) = if first & 0x40 != 0 {
            match b(1) {
                l @ 0..=191 => (first & 0x3f, 2, l),
                l @ 192..=223 => (first & 0x3f, 3, ((l - 192) << 8) + b(2) + 192),
                _ => (first & 0x3f, 6, b(2) << 24 | b(3) << 16 | b(4) << 8 | b(5)),
            }
        } else {
            let tag = (first >> 2) & 0x0f;
            match first & 3 {
                0 => (tag, 2, b(1)),
                1 => (tag, 3, b(1) << 8 | b(2)),
                _ => (tag, 5, b(1) << 24 | b(2) << 16 | b(3) << 8 | b(4)),
            }
        };
        out.push((tag, bytes[at + head..at + head + len].to_vec()));
        at += head + len;
    }
    out
}

/// One hex digit of a line's bytes changed: the first byte's first digit.
fn one_digit_changed(line: &str) -> String {
    let (head, rest) = line.split_once(": ").unwrap();
    let mut rest: Vec<char> = rest.chars().collect();
    rest[0] = if rest[0] == '0' { '1' } else { '0' };
    format!("{head}: {}", rest.into_iter().collect::<String>())
}

/// Test 4: a digit changed on each line in turn is refused on that line,
/// by Faraday and by paperkey; the whole data's CRC changed, and a byte of
/// the raw form changed, are refused by both.
#[test]
fn a_line_paperkey_refuses_faraday_refuses_on_the_same_line() {
    let k = key();
    let dir = scratch("bad-crc");
    let cert = dir.join("cert.gpg");
    std::fs::write(&cert, k.certificate(MADE)).unwrap();
    let shown = k.paperkey();
    let lines: Vec<&str> = shown.lines().collect();
    let last = lines.len();
    for n in 1..last {
        let bad: Vec<String> = lines
            .iter()
            .enumerate()
            .map(|(i, l)| {
                if i + 1 == n {
                    one_digit_changed(l)
                } else {
                    l.to_string()
                }
            })
            .collect();
        assert_eq!(
            paperkey::line(n as u32, &bad[n - 1]).err(),
            Some(LineError::Crc),
            "line {n}"
        );
        let text = bad.join("\n") + "\n";
        assert_eq!(
            paperkey::read_text(&text).err(),
            Some(ReadError::Line(n as u32, LineError::Crc))
        );
        let file = dir.join("bad.txt");
        std::fs::write(&file, &text).unwrap();
        let Some(ran) = paperkey_restore(&cert, &file, &[]) else {
            return;
        };
        assert!(!ran.ok);
        assert!(
            ran.err.contains(&format!("CRC on line {n} does not match")),
            "line {n}: {}",
            ran.err
        );
    }
    // The whole data's CRC.
    let mut whole: Vec<String> = lines.iter().map(|l| l.to_string()).collect();
    whole[last - 1] = format!("{last:3}: 000000");
    let text = whole.join("\n") + "\n";
    assert_eq!(paperkey::read_text(&text).err(), Some(ReadError::Crc));
    let file = dir.join("bad.txt");
    std::fs::write(&file, &text).unwrap();
    let ran = paperkey_restore(&cert, &file, &[]).unwrap();
    assert!(
        !ran.ok && ran.err.contains("CRC of secret does not match"),
        "{}",
        ran.err
    );
    // The raw form with a byte changed, as paperkey's bad-crc-raw.sh does.
    let mut raw = fixture("faraday-key.raw");
    raw[30] ^= 0x55;
    assert_eq!(paperkey::read_raw(&raw).err(), Some(ReadError::Crc));
    let file = dir.join("bad.raw");
    std::fs::write(&file, &raw).unwrap();
    let ran = paperkey_restore(&cert, &file, &["--input-type", "raw"]).unwrap();
    assert!(
        !ran.ok && ran.err.contains("CRC of secret does not match"),
        "{}",
        ran.err
    );
}

/// Test 5: a key of paperkey's own EdDSA check key's shape (an Ed25519
/// primary that certifies and signs, no subkey), made by GnuPG: its
/// paperkey reads as one entry with that key's fingerprint, and the
/// certificate is refused as not of Faraday's shape.
#[test]
fn a_key_with_no_subkey_reads_as_one_entry_and_is_refused_by_its_shape() {
    let text = text("gpg-no-subkey.txt");
    let fpr = text
        .lines()
        .next()
        .and_then(|l| l.strip_prefix("# Secret portions of key "))
        .expect("paperkey's first comment names the key")
        .to_string();
    let data = paperkey::read_text(&text).expect("reads");
    let entries = paperkey::entries(&data).expect("its entries");
    assert_eq!(entries.len(), 1);
    assert_eq!(hex(&entries[0].fingerprint), fpr);
    assert_eq!(
        Certificate::read(&fixture("gpg-no-subkey.asc")).err(),
        Some(Refusal::Shape)
    );
}

// ---- Refusals, with keys made by GnuPG 2.4 ----

/// `gpg --list-keys` of a certificate, in a home of its own.
fn listing(name: &str, cert: &[u8]) -> Option<String> {
    let home = scratch(name);
    let file = home.join("cert.asc");
    std::fs::write(&file, cert).unwrap();
    let ran = gpg(&home, &["--import", file.to_str().unwrap()])?;
    assert!(ran.ok, "{}", ran.err);
    let ran = gpg(&home, &["--list-keys"]).unwrap();
    Some(String::from_utf8_lossy(&ran.out).into_owned())
}

/// Tests 6 and 7: GnuPG's default key (an Ed25519 primary with a cv25519
/// encryption subkey) and an RSA key are refused by their algorithm,
/// named as `gpg --list-keys` names it.
#[test]
fn a_key_of_another_algorithm_is_refused_by_gnupgs_name_for_it() {
    for (file, name, line) in [
        ("gpg-default.asc", "cv25519", "sub   cv25519 "),
        ("gpg-rsa3072.asc", "rsa3072", "pub   rsa3072 "),
    ] {
        let cert = fixture(file);
        assert_eq!(
            Certificate::read(&cert).err(),
            Some(Refusal::Algorithm(name.to_string())),
            "{file}"
        );
        let Some(listed) = listing("algorithm", &cert) else {
            return;
        };
        assert!(listed.contains(line), "{file}:\n{listed}");
    }
}

/// Test 8: a key of Faraday's shape made by GnuPG with a passphrase: its
/// paperkey is refused as protected, at its second line.
#[test]
fn a_paperkey_protected_by_a_passphrase_is_refused() {
    let cert = Certificate::read(&fixture("gpg-protected.asc")).expect("of Faraday's shape");
    let keys = [cert.fingerprint(), cert.subkey_fingerprint()];
    let text = text("gpg-protected.txt");
    let mut prefix = Vec::new();
    let mut lines = text
        .lines()
        .filter(|l| !l.starts_with('#') && !l.is_empty());
    let one = paperkey::line(1, lines.next().unwrap()).unwrap();
    prefix.extend_from_slice(&one.bytes);
    assert!(
        paperkey::check(&prefix, &keys, false).is_ok(),
        "line 1 is the key's"
    );
    let two = paperkey::line(2, lines.next().unwrap()).unwrap();
    prefix.extend_from_slice(&two.bytes);
    assert_eq!(
        paperkey::check(&prefix, &keys, false),
        Err(Refusal::Protected)
    );
    let data = paperkey::read_text(&text).unwrap();
    assert_eq!(restore(&cert, &data).err(), Some(Refusal::Protected));
}

/// Test 9: the same shape with no passphrase: its secrets make its
/// public keys, and its self-signatures are not the ones Faraday makes.
#[test]
fn a_key_of_faradays_shape_made_by_gnupg_is_not_restored() {
    let cert = Certificate::read(&fixture("gpg-unprotected.asc")).expect("of Faraday's shape");
    let data = paperkey::read_text(&text("gpg-unprotected.txt")).unwrap();
    let keys = [cert.fingerprint(), cert.subkey_fingerprint()];
    assert!(paperkey::check(&data, &keys, true).is_ok());
    assert_eq!(restore(&cert, &data).err(), Some(Refusal::NotOurs));
}

/// The line, counting from 1, at which `check` first refuses the lines of
/// `k` against `keys`, and its refusal.
fn refused_at(k: &Key, keys: &[[u8; 20]; 2]) -> Option<(usize, Refusal)> {
    let mut prefix = Vec::new();
    for (i, l) in data_lines(k).iter().enumerate() {
        prefix.extend_from_slice(&paperkey::line(i as u32 + 1, l).unwrap().bytes);
        if let Err(e) = paperkey::check(&prefix, keys, false) {
            return Some((i + 1, e));
        }
    }
    None
}

/// Test 10: the lines of key A with the certificate of key B are refused
/// at line 1; with the certificate of A's primary and another subkey, at
/// the line that completes the subkey's fingerprint.
#[test]
fn a_paperkey_for_another_key_or_subkey_is_refused_at_its_header() {
    let a = key_with(0x55, 0x66);
    let b = key_with(0x77, 0x88);
    let cert_b = cert_of(&b, MADE);
    assert_eq!(
        refused_at(&a, &[cert_b.fingerprint(), cert_b.subkey_fingerprint()]),
        Some((1, Refusal::OtherKey(a.fingerprint())))
    );
    let other_subkey = key_with(0x55, 0x99);
    let cert = cert_of(&other_subkey, MADE);
    assert_eq!(cert.fingerprint(), a.fingerprint(), "the same primary key");
    // The subkey's fingerprint is bytes 62 to 81 of the data: line 4.
    assert_eq!(
        refused_at(&a, &[cert.fingerprint(), cert.subkey_fingerprint()]),
        Some((4, Refusal::OtherSubkey(a.subkey_fingerprint())))
    );
    let data = paperkey::read_text(&a.paperkey()).unwrap();
    assert_eq!(
        restore(&cert, &data).err(),
        Some(Refusal::OtherSubkey(a.subkey_fingerprint()))
    );
}

/// Test 11: a paperkey whose secrets are other seeds, with CRCs made over
/// them: Faraday refuses it as not matching the public key. paperkey
/// takes the same lines and writes a secret key from them, since it
/// checks only the CRCs; `gpg --import` takes that key as the key's
/// secret key, and only signing with it fails.
#[test]
fn a_secret_that_does_not_make_the_public_key_is_refused() {
    let k = key();
    let mut data = vec![0u8];
    for (fp, seed) in [
        (k.fingerprint(), [0x5A; 32]),
        (k.subkey_fingerprint(), [0xA5; 32]),
    ] {
        let secret = secret_part(&seed);
        data.push(4);
        data.extend_from_slice(&fp);
        data.extend_from_slice(&(secret.len() as u16).to_be_bytes());
        data.extend_from_slice(&secret);
    }
    let lines = lines_of(&data);
    let read = paperkey::read_text(&lines).expect("every CRC matches");
    let cert = cert_of(&k, MADE);
    let keys = [cert.fingerprint(), cert.subkey_fingerprint()];
    assert!(
        paperkey::check(&read, &keys, true).is_ok(),
        "every header is the key's"
    );
    assert_eq!(restore(&cert, &read).err(), Some(Refusal::Mismatch));

    let dir = scratch("mismatch");
    let pubring = dir.join("cert.gpg");
    std::fs::write(&pubring, k.certificate(MADE)).unwrap();
    let file = dir.join("lines.txt");
    std::fs::write(&file, &lines).unwrap();
    let Some(ran) = paperkey_restore(&pubring, &file, &[]) else {
        return;
    };
    assert!(ran.ok, "paperkey checks only the CRCs: {}", ran.err);
    assert!(!ran.out.is_empty(), "and writes a secret key");
    let secret = dir.join("secret.gpg");
    std::fs::write(&secret, &ran.out).unwrap();
    let home = scratch("mismatch-gpg");
    let ran = gpg(&home, &["--import", secret.to_str().unwrap()]).unwrap();
    // Recorded: GnuPG 2.4 imports it as the key's secret key, unchecked.
    assert!(
        ran.ok && ran.err.contains("secret key imported"),
        "{}",
        ran.err
    );
    let listed = gpg(&home, &["--with-colons", "--list-secret-keys"]).unwrap();
    let listed = String::from_utf8_lossy(&listed.out).into_owned();
    assert!(listed.contains("sec:"), "{listed}");
    // And refuses to sign with it only when asked to: "Bad secret key".
    let data = dir.join("data.txt");
    std::fs::write(&data, b"Signed after a wrong restore.\n").unwrap();
    let sig = dir.join("data.txt.sig");
    let signed = gpg(
        &home,
        &[
            "--passphrase",
            "",
            "--detach-sign",
            "--output",
            sig.to_str().unwrap(),
            data.to_str().unwrap(),
        ],
    )
    .unwrap();
    assert!(
        !signed.ok && signed.err.contains("signing failed: Bad secret key"),
        "{}",
        signed.err
    );
}

// ---- Certificates as they arrive ----

/// The lines of `k`, read.
fn data_of(k: &Key) -> zeroize::Zeroizing<Vec<u8>> {
    paperkey::read_text(&k.paperkey()).unwrap()
}

/// Test 12: a key renewed, its renewed certificate: the record has the
/// new expiry and that certificate.
#[test]
fn a_renewed_certificate_restores_with_its_new_expiry() {
    let k = key();
    let at = MADE + YEAR;
    let mut renewed = key();
    renewed.expiry = at - MADE + 3 * YEAR;
    let r = restore(&cert_of(&renewed, at), &data_of(&k)).expect("restored");
    assert_eq!(r.key.expiry, renewed.expiry);
    assert_eq!(r.certified_at, at);
    assert_eq!(r.certificate, renewed.certificate(at));
    assert_eq!(*r.revocation, k.revocation(MADE, Reason::Unspecified, ""));
    assert_eq!(r.revoked, None);
}

/// `certs` imported into one GnuPG home in turn, then the key exported
/// as armour.
fn merged(name: &str, now: u32, certs: &[Vec<u8>], fp: &[u8; 20]) -> Option<Vec<u8>> {
    let home = scratch(name);
    for (i, c) in certs.iter().enumerate() {
        let file = home.join(format!("{i}.asc"));
        std::fs::write(&file, c).unwrap();
        let ran = gpg_at(&home, now, &["--import", file.to_str().unwrap()])?;
        assert!(ran.ok, "{}", ran.err);
    }
    let ran = gpg_at(&home, now, &["--armor", "--export", &hex(fp)]).unwrap();
    assert!(ran.ok, "{}", ran.err);
    Some(ran.out)
}

/// Test 13: the certificates before and after a renewal, merged by GnuPG:
/// the newest self-signature gives the record, the older one is made
/// again and equal, and not kept.
#[test]
fn a_certificate_merged_by_gnupg_restores_from_its_newest_self_signature() {
    let k = key();
    let at = MADE + YEAR;
    let mut renewed = key();
    renewed.expiry = at - MADE + 3 * YEAR;
    let first = armor(Armor::PublicKey, &k.certificate(MADE)).into_bytes();
    let second = armor(Armor::PublicKey, &renewed.certificate(at)).into_bytes();
    let Some(both) = merged("merged", at + 60, &[first, second], &k.fingerprint()) else {
        return;
    };
    let cert = Certificate::read(&both).expect("reads");
    assert_eq!(cert.certified_at(), Some(at));
    let r = restore(&cert, &data_of(&k)).expect("restored");
    assert_eq!(r.key.expiry, renewed.expiry);
    assert_eq!(r.certificate, renewed.certificate(at));
    // GnuPG kept the older self-signatures beside the newer ones: two on
    // the user ID and two bindings, each made again and compared.
    let exported = faraday_pgp::read_block(&both).unwrap().0;
    assert_eq!(exported.tags.iter().filter(|&&t| t == 2).count(), 4);
}

/// Test 14: a certificate with the key's revocation (made with Make a
/// revocation), imported into GnuPG and exported: restored, marked
/// revoked at the revocation's time.
#[test]
fn a_revoked_certificate_restores_marked_revoked() {
    let k = key();
    let when = MADE + 100;
    let cert = armor(Armor::PublicKey, &k.certificate(MADE)).into_bytes();
    let rev = armor(
        Armor::PublicKey,
        &k.revocation(when, Reason::Retired, "Not used"),
    )
    .into_bytes();
    let Some(both) = merged("revoked", when + 60, &[cert, rev], &k.fingerprint()) else {
        return;
    };
    let cert = Certificate::read(&both).expect("reads");
    assert_eq!(cert.revoked_at(), Some(when));
    let r = restore(&cert, &data_of(&k)).expect("restored");
    assert_eq!(r.revoked, Some(when));
    assert_eq!(r.certificate, k.certificate(MADE));
}

/// Test 15: a certificate a second GnuPG key has certified: restored, the
/// certification set aside and counted.
#[test]
fn a_certification_by_another_key_is_set_aside_and_counted() {
    let k = key();
    let home = scratch("certified");
    let Some(ran) = gpg(
        &home,
        &[
            "--passphrase",
            "",
            "--quick-generate-key",
            "Other Person <other@example.com>",
            "ed25519",
            "cert",
            "never",
        ],
    ) else {
        return;
    };
    assert!(ran.ok, "{}", ran.err);
    let file = home.join("cert.asc");
    std::fs::write(&file, armor(Armor::PublicKey, &k.certificate(MADE))).unwrap();
    let ran = gpg(&home, &["--import", file.to_str().unwrap()]).unwrap();
    assert!(ran.ok, "{}", ran.err);
    let ran = gpg(
        &home,
        &[
            "--passphrase",
            "",
            "--quick-sign-key",
            &hex(&k.fingerprint()),
        ],
    )
    .unwrap();
    assert!(ran.ok, "{}", ran.err);
    let ran = gpg(&home, &["--armor", "--export", &hex(&k.fingerprint())]).unwrap();
    let cert = Certificate::read(&ran.out).expect("reads");
    assert_eq!(cert.set_aside(), 1);
    let r = restore(&cert, &data_of(&k)).expect("restored");
    assert_eq!(r.set_aside, 1);
    assert_eq!(r.certificate, k.certificate(MADE));
}

/// A certificate Faraday made reads with the facts the cards show.
#[test]
fn a_certificate_reads_with_its_facts() {
    let k = key();
    let cert = cert_of(&k, MADE);
    assert_eq!(cert.fingerprint(), k.fingerprint());
    assert_eq!(cert.subkey_fingerprint(), k.subkey_fingerprint());
    assert_eq!(cert.created(), MADE);
    assert_eq!(cert.user_ids(), &k.user_ids[..]);
    assert_eq!(cert.expiry(), k.expiry);
    assert_eq!(cert.certified_at(), Some(MADE));
    assert_eq!(cert.revoked_at(), None);
    assert_eq!(cert.set_aside(), 0);
    assert_eq!(
        Certificate::read(b"not a key").err(),
        Some(Refusal::NotCertificate)
    );
}
