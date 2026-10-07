//! The committed vectors (`tools/vectors/vault/`) are what this build
//! writes, and each passphrase opens the slot it was given.

#[path = "../examples/vectors.rs"]
mod vectors;

use faraday_vault::records::field;

fn committed(name: &str) -> Vec<u8> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tools/vectors/vault")
        .join(name);
    std::fs::read(path).expect("the committed vector")
}

#[test]
fn the_committed_vectors_are_what_this_build_writes() {
    for (name, bytes) in vectors::vectors() {
        assert!(
            committed(name) == bytes,
            "{name} differs from what this build writes"
        );
    }
}

#[test]
fn each_passphrase_opens_its_slot_of_the_vectors() {
    let three = committed(vectors::THREE.0);
    for (p, label) in vectors::THREE.1.iter().zip(vectors::LABELS) {
        let (_, c) = faraday_vault::open(&three, p.as_bytes()).expect("opens");
        assert_eq!(c.records.len(), 10);
        assert_eq!(c.records[0].text(field::LABEL), Some(label));
    }
    let one = committed(vectors::ONE.0);
    assert!(faraday_vault::open(&one, vectors::ONE.1[0].as_bytes()).is_ok());
    assert!(faraday_vault::open(&one, b"first passphrase").is_err());
}
