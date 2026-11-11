//! The key file Make a key file writes (`docs/VAULT.md` §3.2, F208): KeePass's
//! XML key file version 2.0, laid out as KeePassXC 2.7.10 writes it, and a
//! vault made by pykeepass (`tools/vectors/kdbx/keyfile_write.py`) under a
//! passphrase and that file opening here.

use std::path::{Path, PathBuf};

use faraday_crypto::work::{Compute, Work};
use faraday_vault::file::{self, Error};
use faraday_vault::kdbx::items::Body;
use faraday_vault::kdbx::keyfile;
use faraday_vault::kdbx::{self, Components};
use faraday_vault::records::{field, kind};

const PASSPHRASE: &str = "key file writer passphrase";

#[expect(
    clippy::disallowed_methods,
    reason = "a mint: faraday-vault's tests, which cannot reach faraday-core's testkit"
)]
fn compute() -> Compute {
    Compute::new(|_: Work| {})
}

fn vector(name: &str) -> Vec<u8> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let path: PathBuf = dir.join("../../tools/vectors/kdbx").join(name);
    std::fs::read(path).unwrap()
}

/// The bytes the vector's key file holds.
fn key() -> [u8; 32] {
    std::array::from_fn(|i| (i * 5 + 3) as u8)
}

#[test]
fn the_key_file_is_the_one_keepassxc_would_write() {
    let written = keyfile::write(&key());
    assert_eq!(*written, vector("faraday-key-file.keyx"));
}

#[test]
fn a_key_file_keepassxc_wrote_is_written_back_unchanged() {
    let theirs = vector("keepassxc-key-file.keyx");
    let key = keyfile::read(&theirs).unwrap();
    assert_eq!(*keyfile::write(&key), theirs);
}

#[test]
fn the_key_file_reads_back_as_the_key() {
    assert_eq!(*keyfile::read(&keyfile::write(&key())).unwrap(), key());
}

#[test]
fn a_vault_made_with_the_key_file_opens_with_it_and_shows_its_entry() {
    let bytes = vector("pykeepass-key-file.kdbx");
    let components = Components::with_key_file(keyfile::read(&keyfile::write(&key())).unwrap());
    let opened = file::open_with(
        &compute(),
        &bytes,
        PASSPHRASE,
        components,
        &mut |_| Ok(()),
        &mut |_, _| Ok(None),
        None,
    )
    .expect("opens");
    let items = &opened.read().items;
    assert_eq!(items.len(), 1);
    let Body::Record(r) = &items[0].body else {
        panic!("not an entry")
    };
    assert_eq!(r.kind, kind::ENTRY);
    assert_eq!(r.text(field::TITLE), Some("entry-a"));
    assert_eq!(r.text(field::USERNAME), Some("user-a"));
    assert_eq!(r.text(field::PASSWORD), Some("pass-a"));
}

#[test]
fn the_vault_is_refused_with_a_key_file_of_other_bytes() {
    let bytes = vector("pykeepass-key-file.kdbx");
    let other = std::array::from_fn(|i| i as u8);
    for components in [
        Components::with_key_file(keyfile::read(&keyfile::write(&other)).unwrap()),
        Components::none(),
    ] {
        let got = file::open_with(
            &compute(),
            &bytes,
            PASSPHRASE,
            components,
            &mut |_| Ok(()),
            &mut |_, _| Ok(None),
            None,
        );
        assert!(matches!(got, Err(Error::Kdbx(kdbx::Error::Passphrase))));
    }
}
