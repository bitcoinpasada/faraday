//! What other implementations write, read here: a ten-part BBQr PSBT from
//! the QR test kit, checked against the base64 the kit lists for it, and
//! Faraday OS's file envelope over BBQr, written by Faraday OS's own code (`tests/vectors/README.md`).

use std::path::Path;

use faraday_qr::{Arrived, Assembler, Step, envelope};

fn vector(name: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/vectors")
        .join(name)
}

/// The one code in a PNG, as text.
fn code(png: &str) -> String {
    let bytes = std::fs::read(vector(png)).unwrap();
    let mut codes = faraday_files::qr_in_png(&bytes).unwrap();
    assert_eq!(codes.len(), 1, "{png}");
    String::from_utf8(codes.remove(0)).unwrap()
}

#[test]
fn ten_bbqr_parts_are_the_psbt_in_any_order() {
    // The base64 under "3-OF-5, FIVE INPUTS" in the kit's list.
    let list = std::fs::read_to_string(vector("wallets/psbts.txt")).unwrap();
    let mut lines = list.lines();
    lines
        .find(|l| l.starts_with("3-OF-5, FIVE INPUTS"))
        .unwrap();
    let whole = osk_bip::base64::decode(lines.next().unwrap().trim()).unwrap();
    let parts: Vec<String> = (1..=10)
        .map(|n| code(&format!("wallets/multisig-6-psbt-bbqr-part-{n}-of-10.png")))
        .collect();
    let mut a = Assembler::default();
    // Out of order, with one part seen twice, as a camera reads them.
    let order = [3, 0, 7, 7, 9, 1, 2, 4, 5, 6, 8];
    let mut last = Step::NotMine;
    for (k, i) in order.iter().enumerate() {
        last = a.feed(&parts[*i]);
        if k == 2 {
            assert_eq!(
                last,
                Step::Part {
                    what: "BBQr",
                    have: 3,
                    total: 10,
                    missing: vec![2, 3, 5, 6, 7, 9, 10],
                }
            );
        }
    }
    assert_eq!(last, Step::Done(Arrived::Psbt(whole)));
    assert!(!a.busy());
}

#[test]
fn faraday_oss_file_transfers_arrive_whole() {
    for (frames, json, name, kind) in [
        (
            "faraday-os-bbqr-z.txt",
            "faraday-os-envelope.json",
            "notes.txt",
            "file",
        ),
        (
            "faraday-os-bbqr-z-key.txt",
            "faraday-os-envelope-2.json",
            "key.asc",
            "public-key",
        ),
    ] {
        let env = std::fs::read(vector(json)).unwrap();
        let (_, data, _) = envelope::unpack(&env).unwrap();
        // Faraday writes the same envelope bytes for the same file.
        assert_eq!(envelope::pack(name, &data, kind).unwrap(), env);
        let mut a = Assembler::default();
        let mut last = Step::NotMine;
        for line in std::fs::read_to_string(vector(frames)).unwrap().lines() {
            last = a.feed(line);
        }
        assert_eq!(
            last,
            Step::Done(Arrived::File(name.into(), data, kind.into())),
            "{frames}"
        );
    }
}

#[test]
fn an_envelope_that_does_not_match_its_file_is_refused() {
    let env = std::fs::read(vector("faraday-os-envelope.json")).unwrap();
    let text = String::from_utf8(env).unwrap();
    // One byte of the file changed: the hash no longer matches.
    let tampered = text.replacen("\"data\":\"R", "\"data\":\"S", 1);
    assert!(envelope::unpack(tampered.as_bytes()).is_err());
    // A field added.
    let extra = text.replacen('{', "{\"x\":1,", 1);
    assert!(envelope::unpack(extra.as_bytes()).is_err());
}
