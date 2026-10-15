//! What Faraday signs, judged by Android's own tool (`docs/APK-SIGNING.md`
//! §8): `apksigner verify` accepts each signed fixture and prints the
//! fingerprint Faraday shows, and `apksigner sign` with the same key makes
//! the same file, byte for byte. `apksigner` is found as
//! `FARADAY_APKSIGNER` or under `$ANDROID_HOME/build-tools/36.0.0/`; with
//! none, and with the large fixture's tools, the tests fail unless
//! `FARADAY_SKIP_ANDROID=1` says the machine has no Android SDK.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use faraday_apk::sign::{assemble, sign, strip};
use faraday_apk::zip::{Apk, Refusal, Slice};
use faraday_apk::{key, pem};
use faraday_sb::Pair;
use faraday_sb::rng::Stream;

const NOW: u64 = 1_791_000_000;
const PINNED: &str = "36.0.0";

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tools/vectors/apk")
        .join(name)
}

/// The key every test signs with: made by Faraday's own code from a fixed
/// seed. No key is committed.
fn pair() -> &'static Pair {
    static PAIR: OnceLock<Pair> = OnceLock::new();
    PAIR.get_or_init(|| key::make(&[0x42; 32], "Faraday Test", NOW).unwrap())
}

/// `FARADAY_SKIP_ANDROID=1` says on purpose that this machine has no
/// Android tools; without it a missing `apksigner` fails the test.
fn skipping() -> bool {
    std::env::var_os("FARADAY_SKIP_ANDROID").is_some_and(|v| v == "1")
}

fn apksigner() -> Option<PathBuf> {
    let found = std::env::var_os("FARADAY_APKSIGNER")
        .map(PathBuf::from)
        .or_else(|| {
            let home = std::env::var_os("ANDROID_HOME")?;
            let p = Path::new(&home)
                .join("build-tools")
                .join(PINNED)
                .join("apksigner");
            p.exists().then_some(p)
        });
    assert!(
        found.is_some() || skipping(),
        "apksigner {PINNED} is not found: set FARADAY_APKSIGNER or ANDROID_HOME, or FARADAY_SKIP_ANDROID=1"
    );
    found
}

/// A test's own scratch tree; removed when it drops.
struct Scratch(PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn scratch(name: &str) -> Scratch {
    let dir = std::env::temp_dir().join(format!("faraday-apk-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    Scratch(dir)
}

/// Faraday's signature over a fixture's bytes.
fn faraday_signs(unsigned: &[u8]) -> Vec<u8> {
    let mut src = Slice(unsigned);
    let apk = Apk::read(&mut src).expect("the fixture reads");
    let p = pair();
    let signed = sign(
        &mut Stream::new(&[0x43; 32], 0),
        &mut src,
        &apk,
        &p.key,
        &p.cert,
    )
    .unwrap();
    assemble(&mut src, &apk, &signed).unwrap()
}

fn run(tool: &Path, args: &[&str]) -> (bool, String) {
    let out = Command::new(tool)
        .args(args)
        .output()
        .expect("apksigner runs");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).into_owned() + &String::from_utf8_lossy(&out.stderr),
    )
}

/// The large fixture, made by the script into `dir`; `None` only where
/// `FARADAY_SKIP_ANDROID=1` says the Android tools are not there.
fn large(dir: &Path) -> Option<PathBuf> {
    if skipping() {
        return None;
    }
    let script = fixture("make.sh");
    let ok = Command::new("bash")
        .arg(script)
        .arg("--large")
        .arg(dir)
        .status()
        .is_ok_and(|s| s.success());
    assert!(ok, "make.sh --large failed: the Android tools are needed");
    Some(dir.join("large-unsigned.apk"))
}

/// The ID-value pairs of a signed APK's signing block: ID and value length.
fn pairs(signed: &[u8]) -> Vec<(u32, usize)> {
    let n = signed.len();
    let tail = &signed[n.saturating_sub(22 + 65_535)..];
    let i = (0..=tail.len() - 22)
        .rev()
        .find(|&i| tail[i..i + 4] == [0x50, 0x4b, 5, 6])
        .unwrap();
    let eocd = n - tail.len() + i;
    let cd = u32::from_le_bytes(signed[eocd + 16..eocd + 20].try_into().unwrap()) as usize;
    let size = u64::from_le_bytes(signed[cd - 24..cd - 16].try_into().unwrap()) as usize;
    let (mut at, end) = (cd - size, cd - 24);
    let mut out = Vec::new();
    while at < end {
        let len = u64::from_le_bytes(signed[at..at + 8].try_into().unwrap()) as usize;
        out.push((
            u32::from_le_bytes(signed[at + 8..at + 12].try_into().unwrap()),
            len - 4,
        ));
        at += 8 + len;
    }
    out
}

/// The v3 signer's SDK range in a signed APK.
fn v3_range(signed: &[u8]) -> (u32, u32) {
    let n = signed.len();
    let tail = &signed[n.saturating_sub(22 + 65_535)..];
    let i = (0..=tail.len() - 22)
        .rev()
        .find(|&i| tail[i..i + 4] == [0x50, 0x4b, 5, 6])
        .unwrap();
    let eocd = n - tail.len() + i;
    let cd = u32::from_le_bytes(signed[eocd + 16..eocd + 20].try_into().unwrap()) as usize;
    let size = u64::from_le_bytes(signed[cd - 24..cd - 16].try_into().unwrap()) as usize;
    let mut at = cd - size - 8 + 8;
    let end = cd - 24;
    while at < end {
        let len = u64::from_le_bytes(signed[at..at + 8].try_into().unwrap()) as usize;
        let id = u32::from_le_bytes(signed[at + 8..at + 12].try_into().unwrap());
        if id == 0xf053_68c0 {
            // signers(4) signer(4) signed_data(4 + n) then min and max.
            let v = at + 12;
            let signed_len = u32::from_le_bytes(signed[v + 8..v + 12].try_into().unwrap()) as usize;
            let r = v + 12 + signed_len;
            return (
                u32::from_le_bytes(signed[r..r + 4].try_into().unwrap()),
                u32::from_le_bytes(signed[r + 4..r + 8].try_into().unwrap()),
            );
        }
        at += 8 + len;
    }
    panic!("no v3 block");
}

#[test]
fn apksigner_accepts_each_signed_fixture_and_prints_the_fingerprint_faraday_shows() {
    let Some(tool) = apksigner() else {
        eprintln!("apksigner is not installed: skipped");
        return;
    };
    let dir = scratch("verify");
    let mut files = vec![fixture("small-unsigned.apk"), fixture("sdk33-unsigned.apk")];
    match large(&dir.0) {
        Some(f) => files.push(f),
        None => eprintln!("the Android tools that make the large fixture are not here: skipped"),
    }
    let shown = faraday_apk::hex(&key::fingerprint(&pair().cert));
    for f in files {
        let signed = faraday_signs(&std::fs::read(&f).unwrap());
        let out = dir.0.join("signed.apk");
        std::fs::write(&out, &signed).unwrap();
        let (ok, said) = run(
            &tool,
            &[
                "verify",
                "--verbose",
                "--print-certs",
                out.to_str().unwrap(),
            ],
        );
        assert!(ok, "{f:?}: {said}");
        assert!(
            said.contains("Verified using v1 scheme (JAR signing): false"),
            "{said}"
        );
        // `apksigner` leaves v2 unchecked where every supported Android
        // reads v3 (a minimum of 28 or more); the block is there either way.
        let v2 = !f.to_string_lossy().contains("sdk33");
        assert!(
            said.contains(&format!(
                "Verified using v2 scheme (APK Signature Scheme v2): {v2}"
            )),
            "{f:?}: {said}"
        );
        assert!(
            said.contains("Verified using v3 scheme (APK Signature Scheme v3): true"),
            "{said}"
        );
        assert!(said.contains("Number of signers: 1"), "{said}");
        assert!(
            said.contains(&format!("Signer #1 certificate SHA-256 digest: {shown}")),
            "{said}"
        );
    }
}

#[test]
fn apksigner_with_the_same_key_makes_the_same_file_byte_for_byte() {
    let Some(tool) = apksigner() else {
        eprintln!("apksigner is not installed: skipped");
        return;
    };
    let dir = scratch("sign");
    std::fs::write(dir.0.join("key.pk8"), &*pair().key).unwrap();
    std::fs::write(dir.0.join("cert.der"), &pair().cert).unwrap();
    let mut files = vec![fixture("small-unsigned.apk"), fixture("sdk33-unsigned.apk")];
    match large(&dir.0) {
        Some(f) => files.push(f),
        None => eprintln!("the Android tools that make the large fixture are not here: skipped"),
    }
    for f in files {
        let theirs = dir.0.join("theirs.apk");
        let (ok, said) = run(
            &tool,
            &[
                "sign",
                "--key",
                dir.0.join("key.pk8").to_str().unwrap(),
                "--cert",
                dir.0.join("cert.der").to_str().unwrap(),
                "--v1-signing-enabled",
                "false",
                "--v2-signing-enabled",
                "true",
                "--v3-signing-enabled",
                "true",
                "--v4-signing-enabled",
                "false",
                "--alignment-preserved",
                "true",
                "--out",
                theirs.to_str().unwrap(),
                f.to_str().unwrap(),
            ],
        );
        assert!(ok, "{f:?}: {said}");
        let unsigned = std::fs::read(&f).unwrap();
        let ours = faraday_signs(&unsigned);
        let theirs = std::fs::read(&theirs).unwrap();
        let first = ours.iter().zip(&theirs).position(|(a, b)| a != b);
        assert!(
            ours == theirs,
            "{f:?}: the files differ ({} and {} bytes), first at {first:?}: {:x?} and {:x?}",
            ours.len(),
            theirs.len(),
            pairs(&ours),
            pairs(&theirs)
        );
        assert_eq!(v3_range(&ours), v3_range(&theirs));
        // What `apksigner` signed is signed, to this reader too.
        assert_eq!(
            Apk::read(&mut Slice(&theirs)).unwrap_err(),
            Refusal::AlreadySigned
        );
    }
}

#[test]
fn an_apk_with_a_v1_signature_reads_as_already_signed() {
    let Some(tool) = apksigner() else {
        return;
    };
    let dir = scratch("v1");
    std::fs::write(dir.0.join("key.pk8"), &*pair().key).unwrap();
    std::fs::write(dir.0.join("cert.der"), &pair().cert).unwrap();
    for (fixture_name, v2) in [
        ("old-sdk-unsigned.apk", "false"),
        ("small-unsigned.apk", "false"),
    ] {
        let out = dir.0.join("v1.apk");
        let (ok, said) = run(
            &tool,
            &[
                "sign",
                "--key",
                dir.0.join("key.pk8").to_str().unwrap(),
                "--cert",
                dir.0.join("cert.der").to_str().unwrap(),
                "--v1-signing-enabled",
                "true",
                "--v2-signing-enabled",
                v2,
                "--v3-signing-enabled",
                "false",
                "--v4-signing-enabled",
                "false",
                "--alignment-preserved",
                "true",
                "--out",
                out.to_str().unwrap(),
                fixture(fixture_name).to_str().unwrap(),
            ],
        );
        assert!(ok, "{said}");
        let bytes = std::fs::read(&out).unwrap();
        assert_eq!(
            Apk::read(&mut Slice(&bytes)).unwrap_err(),
            Refusal::AlreadySigned,
            "{fixture_name}"
        );
    }
}

#[test]
fn removing_the_block_and_restoring_the_offset_gives_back_the_unsigned_file() {
    let unsigned = std::fs::read(fixture("small-unsigned.apk")).unwrap();
    let signed = faraday_signs(&unsigned);
    let entries = Apk::read(&mut Slice(&unsigned)).unwrap().cd_offset as usize;
    assert_eq!(strip(&signed, entries).as_deref(), Some(&unsigned[..]));
    assert_eq!(
        Apk::read(&mut Slice(&signed)).unwrap_err(),
        Refusal::AlreadySigned
    );
}

#[test]
fn the_signature_does_not_depend_on_the_blinding() {
    let unsigned = std::fs::read(fixture("small-unsigned.apk")).unwrap();
    let mut src = Slice(&unsigned);
    let apk = Apk::read(&mut src).unwrap();
    let p = pair();
    let a = sign(
        &mut Stream::new(&[1; 32], 0),
        &mut src,
        &apk,
        &p.key,
        &p.cert,
    )
    .unwrap();
    let b = sign(
        &mut Stream::new(&[2; 32], 9),
        &mut src,
        &apk,
        &p.key,
        &p.cert,
    )
    .unwrap();
    assert_eq!(a, b);
}

#[test]
fn the_certificate_is_what_apksigner_and_openssl_read() {
    let k = pair();
    let info = key::describe(&k.cert).unwrap();
    assert_eq!(info.name, "Faraday Test");
    assert_eq!(info.made, NOW - NOW % 86_400);
    let dir = scratch("cert");
    let path = dir.0.join("cert.pem");
    std::fs::write(&path, pem::encode(&k.cert)).unwrap();
    assert_eq!(
        pem::read(&std::fs::read(&path).unwrap()).as_deref(),
        Some(&k.cert[..])
    );
    let Ok(out) = Command::new("openssl")
        .args([
            "x509",
            "-in",
            path.to_str().unwrap(),
            "-noout",
            "-subject",
            "-enddate",
            "-startdate",
        ])
        .output()
    else {
        eprintln!("openssl is not installed: skipped");
        return;
    };
    let said = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "{said}");
    assert!(said.contains("CN"), "{said}");
    assert!(said.contains("Faraday Test"), "{said}");
    assert!(said.contains("notAfter=Dec 31 23:59:59 9999 GMT"), "{said}");
    assert!(said.contains("00:00:00"), "{said}");
}
