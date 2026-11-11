//! The `aqrv` binary as a person runs it: a code made and opened again,
//! a wrong passphrase, a file that is not a code, a code inspected with
//! no passphrase, and the test vectors.

use std::io::Write;
use std::process::{Command, Output, Stdio};

fn aqrv(args: &[&str], stdin: &str) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_aqrv"))
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(stdin.as_bytes())
        .unwrap();
    child.wait_with_output().unwrap()
}

fn scratch(name: &str) -> String {
    let dir = std::env::temp_dir().join(format!("aqrv-cli-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir.join(name).to_str().unwrap().to_owned()
}

fn make_note(file: &str) {
    let out = aqrv(
        &[
            "make",
            "--kind",
            "note",
            "--size",
            "53",
            "--memory",
            "64",
            "--out",
            file,
            "--passphrase-stdin",
        ],
        "meet at noon\ncorrect horse\n",
    );
    assert_eq!(out.status.code(), Some(0), "{:?}", out.stderr);
}

#[test]
fn a_code_made_opens_with_its_passphrase_and_not_with_another() {
    let file = scratch("note.aqrv");
    make_note(&file);
    assert_eq!(std::fs::metadata(&file).unwrap().len(), 53);

    let ok = aqrv(&["open", &file, "--passphrase-stdin"], "correct horse\n");
    assert_eq!(ok.status.code(), Some(0), "{:?}", ok.stderr);
    assert!(String::from_utf8_lossy(&ok.stdout).contains("meet at noon"));

    let wrong = aqrv(&["open", &file, "--passphrase-stdin"], "wrong horse\n");
    assert_eq!(wrong.status.code(), Some(1));
    assert!(!String::from_utf8_lossy(&wrong.stdout).contains("meet at noon"));
}

#[test]
fn a_file_of_40_bytes_is_not_recognised() {
    let file = scratch("short.aqrv");
    std::fs::write(&file, [0xFAu8; 40]).unwrap();
    let out = aqrv(&["open", &file, "--passphrase-stdin"], "anything\n");
    assert_eq!(out.status.code(), Some(2));
}

#[test]
fn a_qr_picture_of_32_binary_bytes_is_not_recognised() {
    use faraday_codec::png::qr_png;
    use faraday_codec::qr::{Ecc, Payload, encode};
    use faraday_crypto::work::Compute;
    let c = Compute::new(|_| {});
    let mut payload = [0x11u8; 32];
    payload[0] = 0xF8;
    let matrix = encode(&c, Payload::Bytes(&payload), Ecc::Low).unwrap();
    let file = scratch("binary32.png");
    std::fs::write(&file, qr_png(&matrix, 4)).unwrap();
    let out = aqrv(&["open", &file, "--passphrase-stdin"], "anything\n");
    assert_eq!(out.status.code(), Some(2), "{:?}", out.stderr);
}

#[test]
fn inspect_shows_the_code_and_asks_for_no_passphrase() {
    let file = scratch("inspect.aqrv");
    make_note(&file);
    let out = aqrv(&["inspect", &file], "");
    assert_eq!(out.status.code(), Some(0), "{:?}", out.stderr);
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("53 bytes"));
    assert!(text.contains("64 MiB"));

    let sheet = aqrv(&["inspect", &file, "--json"], "");
    assert!(String::from_utf8_lossy(&sheet.stdout).contains("\"format\":\"AQRV\""));
}

#[test]
fn the_vectors_agree_without_the_large_cases() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tools/vectors/aqrv/aqrv.json"
    );
    let out = aqrv(&["vectors", path, "--skip-large"], "");
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&out.stdout)
    );
}

#[test]
fn a_32_byte_code_prints_its_text_and_opens_from_the_text_or_a_file_of_it() {
    let file = scratch("tiny.txt");
    let made = aqrv(
        &[
            "make",
            "--kind",
            "note",
            "--size",
            "32",
            "--memory",
            "64",
            "--passphrase-stdin",
        ],
        "meet at noon\ncorrect horse\n",
    );
    assert_eq!(made.status.code(), Some(0), "{:?}", made.stderr);
    let stdout = String::from_utf8(made.stdout).unwrap();
    let text = stdout.strip_suffix('\n').unwrap();
    assert_eq!(text.chars().count(), 47);

    let by_argument = aqrv(
        &["open", "--text", text, "--passphrase-stdin"],
        "correct horse\n",
    );
    assert_eq!(
        by_argument.status.code(),
        Some(0),
        "{:?}",
        by_argument.stderr
    );
    assert!(String::from_utf8_lossy(&by_argument.stdout).contains("note meet at noon"));

    std::fs::write(&file, &stdout).unwrap();
    let by_file = aqrv(&["open", &file, "--passphrase-stdin"], "correct horse\n");
    assert_eq!(by_file.status.code(), Some(0), "{:?}", by_file.stderr);

    let lower = aqrv(
        &["open", "--text", &text.to_lowercase(), "--passphrase-stdin"],
        "x\n",
    );
    assert_eq!(lower.status.code(), Some(2));
}
