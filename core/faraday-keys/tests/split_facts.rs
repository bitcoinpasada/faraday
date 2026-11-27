//! A key keeps the public facts of one Seed XOR set and one SLIP-39 set
//! its seed was split into (`docs/VAULT.md` §7): never a part or a share;
//! one set of each kind per key; they go into its vault records, come
//! back when the key loads from one, and a record without them loads as
//! it always did.
#![allow(clippy::disallowed_methods)]

use std::collections::BTreeSet;

use faraday_bip::keys::Network;
use faraday_crypto::work::Compute;
use faraday_keys::vault::{self, OpenVaults};
use faraday_keys::{Keys, Part, Reveal};
use faraday_vault::Cost;
use faraday_vault::records::field;
use faraday_vault::slip39::{Group, Slip39Facts};
use faraday_vault::xor::XorFacts;

#[expect(
    clippy::disallowed_methods,
    reason = "a mint: faraday-keys' tests, which cannot reach faraday-core's testkit"
)]
fn compute() -> Compute {
    Compute::new(faraday_crypto::work::Hooks {
        count: ignore,
        stop: || false,
        progress: |_, _| {},
    })
}

fn ignore(_: faraday_crypto::work::Work) {}

fn halves() -> XorFacts {
    XorFacts {
        parts: vec![[1, 2, 3, 4], [5, 6, 7, 8]],
    }
}

fn quorum() -> Slip39Facts {
    Slip39Facts {
        identifier: 0x1234,
        exponent: 1,
        threshold: 1,
        groups: vec![Group {
            threshold: 2,
            count: 3,
            made: 0b011,
        }],
    }
}

fn with_key(c: &Compute) -> Keys {
    let mut keys = Keys::on(Network::Mainnet);
    keys.add_seed(c, &[7u8; 16], "Savings").expect("a key");
    keys
}

fn record_of(
    c: &Compute,
    keys: &Keys,
    without: &[u8],
) -> faraday_keys::Held<faraday_vault::Record> {
    keys.key_record(c, 0, None, false, without, Reveal::VaultEntry(Part::Words))
        .expect("a seed key has a record")
}

#[test]
fn a_key_with_no_set_has_none_and_its_record_has_no_field_for_one() {
    let c = compute();
    let keys = with_key(&c);
    assert!(keys[0].xor.is_none() && keys[0].slip39.is_none());
    let r = record_of(&c, &keys, &[]);
    assert!(r.field(field::KEY_XOR).is_none() && r.field(field::KEY_SLIP39).is_none());
}

#[test]
fn a_key_keeps_one_set_of_each_kind_and_a_second_is_refused() {
    let c = compute();
    let mut keys = with_key(&c);
    assert!(keys.set_xor(0, Some(halves())));
    assert!(keys.set_slip39(0, Some(quorum())));
    assert!(!keys.set_xor(
        0,
        Some(XorFacts {
            parts: vec![[9; 4]; 3]
        })
    ));
    assert!(!keys.set_slip39(
        0,
        Some(Slip39Facts {
            exponent: 2,
            ..quorum()
        })
    ));
    assert_eq!(keys[0].xor, Some(halves()));
    assert_eq!(keys[0].slip39, Some(quorum()));
    // Facts that are not a set, and a key that is not there.
    assert!(!with_key(&c).set_xor(
        0,
        Some(XorFacts {
            parts: vec![[1; 4]]
        })
    ));
    assert!(!keys.set_xor(5, Some(halves())));
    // Taking the set off leaves room for another.
    assert!(keys.set_xor(0, None));
    assert!(keys.set_xor(
        0,
        Some(XorFacts {
            parts: vec![[9; 4]; 3]
        })
    ));
}

#[test]
fn the_vault_record_keeps_the_sets_and_they_load_back_with_the_key() {
    let c = compute();
    let mut keys = with_key(&c);
    keys.set_xor(0, Some(halves()));
    keys.set_slip39(0, Some(quorum()));
    let record = record_of(&c, &keys, &[]);
    assert!(record.field(field::KEY_XOR).is_some());
    // Left out when the person leaves the field out.
    let bare = record_of(&c, &keys, &[field::KEY_XOR, field::KEY_SLIP39]);
    assert!(bare.field(field::KEY_XOR).is_none() && bare.field(field::KEY_SLIP39).is_none());
    assert_eq!(keys[0].xor, Some(halves()), "the key keeps them");
    let file = vault::create(
        &c,
        [7u8; 16],
        "Test",
        Cost {
            memory_kib: 8192,
            passes: 1,
            lanes: 1,
        },
        "pass",
        &[7u8; vault::SEAL_RANDOM_LEN],
        None,
    )
    .unwrap();
    let mut store = OpenVaults::default();
    let (id, _) = store.unlock(&c, file, "pass").expect("opens");
    let at = store.insert(id, record).unwrap();
    let r = store.records(id).get(at).unwrap();
    // Public facts: readable without a reveal, and no part in them.
    assert!(r.public(field::KEY_XOR).is_some());
    assert!(r.public(field::KEY_SLIP39).is_some());
    assert!(!vault::is_secret(
        faraday_vault::records::kind::KEY,
        field::KEY_XOR
    ));
    assert!(!vault::is_secret(
        faraday_vault::records::kind::KEY,
        field::KEY_SLIP39
    ));
    let mut again = Keys::on(Network::Mainnet);
    let loaded = store.load_from_vault(
        &c,
        &mut again,
        id,
        &BTreeSet::from([at]),
        &Default::default(),
    );
    assert_eq!(loaded, vec![keys[0].fingerprint]);
    assert_eq!(again[0].xor, Some(halves()));
    assert_eq!(again[0].slip39, Some(quorum()));
}

#[test]
fn make_new_parts_replaces_a_keys_seed_xor_set() {
    let c = compute();
    let thirds = XorFacts {
        parts: vec![[9; 4], [10; 4], [11; 4]],
    };
    let mut keys = with_key(&c);
    // A key with no set has nothing to replace; no such key either.
    assert!(!keys.replace_xor(0, thirds.clone()));
    assert!(keys.set_xor(0, Some(halves())));
    assert!(!keys.replace_xor(5, thirds.clone()));
    assert!(!keys.replace_xor(
        0,
        XorFacts {
            parts: vec![[1; 4]]
        }
    ));
    assert_eq!(keys[0].xor, Some(halves()));
    assert!(keys.replace_xor(0, thirds.clone()));
    assert_eq!(keys[0].xor, Some(thirds));
    // The record keeps the new set, in the one field it always had.
    let r = record_of(&c, &keys, &[]);
    assert!(r.field(field::KEY_XOR).is_some());
}

#[test]
fn a_second_set_xor_is_still_not_recorded() {
    let c = compute();
    let mut keys = with_key(&c);
    assert!(keys.set_xor(0, Some(halves())));
    assert!(!keys.set_xor(
        0,
        Some(XorFacts {
            parts: vec![[9; 4]; 3]
        })
    ));
    assert_eq!(keys[0].xor, Some(halves()));
}
