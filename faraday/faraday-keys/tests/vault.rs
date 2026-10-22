//! A vault opened and sealed again, and the keys it holds loaded: a key
//! record of the "abandon" words loads as the fingerprint BIP-39 and
//! BIP-32 publish for them, a vault sealed after a change opens with the
//! change, and a wrong passphrase opens nothing.

use std::ops::{Deref, DerefMut};

use faraday_bip::bip39::{Language, Mnemonic};
use faraday_bip::keys::Network;
use faraday_crypto::work::Compute;
use faraday_keys::Keys;
use faraday_keys::vault::{self, OpenVaults};
use faraday_vault::records::{field, kind};
use faraday_vault::{Contents, Cost, Record};

const ABANDON: &str =
    "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

/// The records as the tests hold them: just the contents.
struct Plain(Contents);

impl Deref for Plain {
    type Target = Contents;
    fn deref(&self) -> &Contents {
        &self.0
    }
}

impl DerefMut for Plain {
    fn deref_mut(&mut self) -> &mut Contents {
        &mut self.0
    }
}

impl From<Contents> for Plain {
    fn from(c: Contents) -> Plain {
        Plain(c)
    }
}

/// A token for the tests' key work, counting nothing (`docs/COMPUTE.md`
/// §3). Cheap.
#[expect(
    clippy::disallowed_methods,
    reason = "a mint: faraday-keys' tests, which cannot reach faraday-core's testkit"
)]
fn compute() -> Compute {
    Compute::new(|_| {})
}

/// A vault of the smallest slot holding one key record of the "abandon"
/// words under `passphrase`. Costs the cheapest Argon2id `create` allows.
fn made(c: &Compute, passphrase: &[u8]) -> Vec<u8> {
    let m = Mnemonic::parse(c, Language::English, ABANDON).expect("the words");
    let contents = Contents {
        records: vec![
            Record::new(kind::KEY)
                .with(field::KEY, &faraday_vault::records::words_payload(&m))
                .with(field::KEY_LABEL, b"Test"),
        ],
    };
    let cost = Cost {
        memory_kib: faraday_vault::CREATE_MIN_MEMORY_KIB,
        passes: faraday_vault::CREATE_MIN_PASSES,
        lanes: 1,
    };
    vault::create(
        c,
        faraday_vault::SLOT_SIZES[0],
        cost,
        &[passphrase],
        &[contents],
        &[7u8; 32],
    )
    .expect("a vault")
}

#[test]
fn a_key_in_a_vault_loads_as_the_key_the_bips_publish() {
    let c = compute();
    let file = made(&c, b"first passphrase");
    let mut store: OpenVaults<Plain> = OpenVaults::default();
    assert!(
        store.unlock(&c, file.clone(), b"another one").is_err(),
        "a wrong passphrase opens nothing"
    );
    let (id, _) = store.unlock(&c, file, b"first passphrase").expect("opens");
    let mut keys = Keys::on(Network::Mainnet);
    let all = (0..store.vault_records(id).records.len()).collect();
    let loaded = store.load_from_vault(&c, &mut keys, id, &all);
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].0, [0x73, 0xc5, 0xda, 0x0a]);
    assert_eq!(keys.len(), 1);
}

#[test]
fn a_vault_sealed_after_a_change_opens_with_it() {
    let c = compute();
    let file = made(&c, b"first passphrase");
    let mut store: OpenVaults<Plain> = OpenVaults::default();
    let (id, _) = store.unlock(&c, file, b"first passphrase").expect("opens");
    store
        .vault_records_mut(id)
        .records
        .push(Record::new(kind::NOTE).with(field::NOTE, b"kept"));
    let sealed = store.sealed(id, &[9u8; 24]).expect("seals");
    store.close(id);
    let (again, _) = store
        .unlock(&c, sealed, b"first passphrase")
        .expect("the sealed vault opens");
    assert_eq!(store.vault_records(again).of(kind::NOTE).count(), 1);
    assert_eq!(store.vault_records(again).of(kind::KEY).count(), 1);
}
