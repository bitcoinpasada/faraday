//! The QR vault's committed vectors (`tools/vectors/vault/qr/`,
//! `docs/VAULT.md` §10) are what this build writes; each opens under its
//! passphrase to what it holds, and each refused file is refused under its
//! name.
#![allow(clippy::disallowed_methods)]

#[path = "../examples/qr_vectors.rs"]
mod vectors;

use faraday_pgp::message;
use faraday_vault::contents::Why;
use faraday_vault::qr::{self, Error};
use faraday_vault::records::field;

fn committed(name: &str) -> Vec<u8> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tools/vectors/vault/qr")
        .join(name);
    std::fs::read(path).expect("the committed vector")
}

fn fields(r: &faraday_vault::Record) -> Vec<(u8, Vec<u8>)> {
    r.fields
        .iter()
        .map(|f| (f.number, f.bytes.to_vec()))
        .collect()
}

#[test]
fn the_committed_vectors_are_what_this_build_writes() {
    for (name, bytes) in vectors::files() {
        assert_eq!(bytes.len(), vectors::LEN, "{name}");
        assert!(
            committed(name) == bytes,
            "{name} differs from what this build writes"
        );
    }
}

#[test]
fn each_vault_opens_to_the_item_it_was_made_from() {
    let c = vectors::compute();
    for (name, item) in vectors::items() {
        let passphrase = if name == vectors::NFD {
            vectors::NFD_PASSPHRASE
        } else {
            vectors::PASSPHRASE
        };
        let got = match qr::open(&c, passphrase, &committed(name)) {
            Ok(r) => r,
            Err(e) => panic!("{name} does not open: {e:?}"),
        };
        assert_eq!(got.kind, item.kind, "{name}");
        assert_eq!(fields(&got), fields(&item), "{name}");
    }
}

#[test]
fn the_empty_vault_is_an_empty_note() {
    let got = qr::open(
        &vectors::compute(),
        vectors::PASSPHRASE,
        &committed(vectors::EMPTY),
    )
    .unwrap();
    assert_eq!(got.field(field::NOTE).unwrap_or(&[]), b"");
}

#[test]
fn the_nfd_vault_opens_under_either_form() {
    let c = vectors::compute();
    for p in [vectors::NFD_PASSPHRASE, vectors::NFD_AS_NFC] {
        let got = qr::open(&c, p, &committed(vectors::NFD)).unwrap();
        assert_eq!(got.field(field::NOTE).unwrap(), "caf\u{e9}".as_bytes());
    }
}

#[test]
fn a_wrong_passphrase_is_refused() {
    let err = qr::open(&vectors::compute(), "not it", &committed(vectors::NOTE)).unwrap_err();
    assert_eq!(err, Error::Message(message::Error::WrongPassphrase));
}

#[test]
fn the_refused_files_are_refused_with_their_refusal() {
    let c = vectors::compute();
    let refused = vectors::refused();
    assert_eq!(refused.len(), 4);
    for (name, _, why) in refused {
        let bytes = committed(name);
        assert_eq!(bytes.len(), vectors::LEN, "{name}");
        assert_eq!(
            qr::open(&c, vectors::PASSPHRASE, &bytes).unwrap_err(),
            Error::Message(message::Error::Unsupported(why)),
            "{name}"
        );
    }
}

#[test]
fn a_message_that_is_not_a_qr_toml_is_refused_as_contents() {
    // A valid message whose plaintext is not a QR vault's 226 bytes.
    let c = vectors::compute();
    let bytes = message::seal(
        &c,
        vectors::PASSPHRASE.as_bytes(),
        vectors::EXPONENT,
        &vectors::random(9),
        b"note = \"short\"\n",
    )
    .unwrap();
    match qr::open(&c, vectors::PASSPHRASE, &bytes).unwrap_err() {
        Error::Contents(r) => assert_eq!(r.why, Why::NotQr),
        e => panic!("{e:?}"),
    }
}

#[test]
fn an_empty_passphrase_seals_and_opens_nothing() {
    let c = vectors::compute();
    let record = vectors::items().remove(0).1;
    assert_eq!(
        qr::seal(&c, "", vectors::EXPONENT, &vectors::random(1), &record).unwrap_err(),
        Error::EmptyPassphrase
    );
    assert_eq!(
        qr::open(&c, "", &committed(vectors::NOTE)).unwrap_err(),
        Error::EmptyPassphrase
    );
}
