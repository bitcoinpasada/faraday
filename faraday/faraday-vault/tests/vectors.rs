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
    let c = vectors::compute();
    let three = committed(vectors::THREE.0);
    for (p, label) in vectors::THREE.1.iter().zip(vectors::LABELS) {
        let (_, got) = faraday_vault::open(&c, &three, p.as_bytes()).expect("opens");
        assert_eq!(got.records.len(), 12);
        assert_eq!(got.records[0].text(field::LABEL), Some(label));
    }
    let one = committed(vectors::ONE.0);
    assert!(faraday_vault::open(&c, &one, vectors::ONE.1[0].as_bytes()).is_ok());
    assert!(faraday_vault::open(&c, &one, b"first passphrase").is_err());
}

fn unhex(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}

/// `slots.json`: slot plaintexts written by hand from §7, each read as
/// the file says: the required fields of a GPG key (type 7) and a
/// signing round (type 9), a sheet's 16 KiB cap, one passphrase record
/// per key (type 14) and one signed amounts record per transaction
/// (type 10).
#[test]
fn each_slot_vector_is_read_as_its_file_says() {
    let text = String::from_utf8(committed("slots.json")).unwrap();
    let v: serde_json::Value = serde_json::from_str(&text).unwrap();
    let cases = v["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 12);
    for case in cases {
        let name = case["case"].as_str().unwrap();
        let got = faraday_vault::Contents::decode(&unhex(case["plain"].as_str().unwrap()));
        let got = match got {
            Ok(_) => "ok".to_string(),
            Err(fault) => format!("{fault:?}"),
        };
        assert_eq!(got, case["expect"].as_str().unwrap(), "{name}");
    }
}

/// The writer refuses what the reader refuses: a second passphrase
/// record for a key, a second signed amounts record for a transaction.
#[test]
fn a_duplicate_record_is_not_written() {
    use faraday_vault::records::{Fault, kind};
    use faraday_vault::{Contents, Error, Record};
    let pass = |fp: [u8; 4]| {
        Record::new(kind::PASSPHRASE)
            .with(field::PASSPHRASE_KEY, &fp)
            .with(field::PASSPHRASE, b"TREZOR")
    };
    let amounts = |tx: u8| {
        Record::new(kind::AMOUNTS)
            .with(field::AMOUNTS_TX, &[tx; 32])
            .with(field::AMOUNTS_INPUT, &1u64.to_le_bytes())
    };
    let fp = [0x9a, 0x6a, 0x25, 0x80];
    for (records, fault) in [
        (
            vec![pass(fp), pass(fp)],
            Some(Fault::Duplicate {
                kind: kind::PASSPHRASE,
                of: fp,
            }),
        ),
        (vec![pass(fp), pass([1, 2, 3, 4])], None),
        (
            vec![amounts(7), amounts(7)],
            Some(Fault::Duplicate {
                kind: kind::AMOUNTS,
                of: [7; 4],
            }),
        ),
        (vec![amounts(7), amounts(8)], None),
    ] {
        let got = Contents { records }.encode(65_536).err();
        assert_eq!(got, fault.map(Error::Contents));
    }
}

/// §8: a vault is not made with an empty passphrase, alone or among
/// others.
#[test]
fn an_empty_passphrase_is_refused_at_create() {
    let c = vectors::compute();
    let size = faraday_vault::SLOT_SIZES[0];
    for passphrases in [vec![&b""[..]], vec![&b"one"[..], &b""[..]]] {
        let contents = vec![faraday_vault::Contents::default(); passphrases.len()];
        let got = faraday_vault::create(&c, size, vectors::COST, &passphrases, &contents, &[1; 32]);
        assert_eq!(got.err(), Some(faraday_vault::Error::Passphrases));
    }
}
