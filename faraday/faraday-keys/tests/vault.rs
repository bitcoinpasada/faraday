//! A vault opened and sealed again, and the keys it holds loaded: a key
//! record of the "abandon" words loads as the fingerprint BIP-39 and
//! BIP-32 publish for them, a vault sealed after a change opens with the
//! change, and a wrong passphrase opens nothing.

use faraday_bip::bip39::{Language, Mnemonic};
use faraday_bip::keys::Network;
use faraday_crypto::work::Compute;
use faraday_keys::Keys;
use faraday_keys::vault::{self, OpenVaults, gpg_field, is_secret};
use faraday_keys::{Part, Reveal};
use faraday_vault::records::{field, kind};
use faraday_vault::{Contents, Cost, Record};

const ABANDON: &str =
    "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

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
    let mut store = OpenVaults::default();
    assert!(
        store.unlock(&c, file.clone(), b"another one").is_err(),
        "a wrong passphrase opens nothing"
    );
    let (id, _) = store.unlock(&c, file, b"first passphrase").expect("opens");
    let mut keys = Keys::on(Network::Mainnet);
    let all = (0..store.records(id).len()).collect();
    let loaded = store.load_from_vault(&c, &mut keys, id, &all);
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].0, [0x73, 0xc5, 0xda, 0x0a]);
    assert_eq!(keys.len(), 1);
}

#[test]
fn a_vault_sealed_after_a_change_opens_with_it() {
    let c = compute();
    let file = made(&c, b"first passphrase");
    let mut store = OpenVaults::default();
    let (id, _) = store.unlock(&c, file, b"first passphrase").expect("opens");
    store.insert(id, Record::new(kind::NOTE).with(field::NOTE, b"kept"));
    let sealed = store.sealed(id, &[9u8; 24]).expect("seals");
    store.close(id);
    let (again, _) = store
        .unlock(&c, sealed, b"first passphrase")
        .expect("the sealed vault opens");
    assert_eq!(store.records(again).of(kind::NOTE).count(), 1);
    assert_eq!(store.records(again).of(kind::KEY).count(), 1);
}

/// A vault of the smallest slot holding `records`, under `passphrase`.
fn made_with(c: &Compute, passphrase: &[u8], records: Vec<Record>) -> Vec<u8> {
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
        &[Contents { records }],
        &[7u8; 32],
    )
    .expect("a vault")
}

fn entry() -> Record {
    Record::new(kind::ENTRY)
        .with(field::TITLE, b"Mail")
        .with(field::USERNAME, b"me")
        .with(field::PASSWORD, b"hunter2")
        .with(field::NOTES, b"recovery code 1234")
}

#[test]
fn a_record_shows_its_public_fields_and_only_the_length_of_a_secret_one() {
    let c = compute();
    let file = made_with(&c, b"pass", vec![entry()]);
    let mut store = OpenVaults::default();
    let (id, _) = store.unlock(&c, file, b"pass").expect("opens");
    let r = store.records(id).get(0).expect("the entry");
    assert_eq!(r.public_text(field::TITLE), Some("Mail"));
    assert_eq!(r.public_text(field::USERNAME), Some("me"));
    assert_eq!(r.public(field::PASSWORD), None);
    assert!(r.has(field::PASSWORD));
    assert_eq!(r.len(field::PASSWORD), Some(7));
    assert_eq!(r.public_all(field::NOTES).count(), 0);
    assert!(!r.has(field::TOTP));
    assert!(is_secret(kind::ENTRY, field::PASSWORD));
    assert!(!is_secret(kind::ENTRY, field::TITLE));
}

#[test]
fn a_secret_leaves_a_record_only_through_a_reveal_that_names_it() {
    let c = compute();
    let file = made_with(&c, b"pass", vec![entry()]);
    let mut store = OpenVaults::default();
    let (id, _) = store.unlock(&c, file, b"pass").expect("opens");
    let shown = |why| store.reveal(&c, id, 0, field::PASSWORD, why).to_string();
    assert_eq!(shown(Reveal::VaultEntry(Part::Password)), "hunter2");
    assert_eq!(
        shown(Reveal::VaultEntry(Part::EntryNotes)),
        "",
        "another part is not the password"
    );
    assert_eq!(
        &*store.reveal(
            &c,
            id,
            0,
            field::NOTES,
            Reveal::VaultEntry(Part::EntryNotes)
        ),
        "recovery code 1234"
    );
    assert_eq!(
        &*store.reveal(&c, id, 0, field::TITLE, Reveal::VaultEntry(Part::Password)),
        "",
        "a public field is read as public"
    );
}

/// A GPG key made at 1,780,000,000 with no revocation kept yet, as a key
/// made before revocations were kept.
fn gpg_key() -> Record {
    let at = 1_780_000_000u32.to_le_bytes();
    Record::new(kind::GPG)
        .with(gpg_field::PRIMARY, &[1; 32])
        .with(gpg_field::CREATED, &at)
        .with(gpg_field::SUBKEY, &[2; 32])
        .with(gpg_field::SUBKEY_CREATED, &at)
        .with(gpg_field::USER_ID, b"Faraday test")
}

/// Whoever has a key's revocation certificate can revoke it (F32): the
/// kept one is read only as its date, and leaves only as a `.rev` file
/// asked for by name.
#[test]
fn a_kept_revocation_leaves_only_as_a_rev_file_asked_for_by_name() {
    let c = compute();
    let file = made_with(&c, b"pass", vec![gpg_key()]);
    let mut store = OpenVaults::default();
    let (id, _) = store.unlock(&c, file, b"pass").expect("opens");
    let rev = Reveal::OutboxFile(Part::Revocation);
    assert!(
        store.gpg_revocation_file(id, 0, rev).is_none(),
        "none kept yet"
    );
    assert!(store.gpg_keep_revocation(id, 0, 1_780_000_100));
    assert!(
        !store.gpg_keep_revocation(id, 0, 1_780_000_200),
        "one is kept"
    );
    let r = store.records(id).get(0).expect("the key");
    assert!(r.has(gpg_field::REVOCATION));
    assert_eq!(r.public(gpg_field::REVOCATION), None);
    assert_eq!(vault::gpg_revocation_kept_at(r), Some(1_780_000_100));
    assert!(
        store
            .gpg_revocation_file(id, 0, Reveal::OutboxFile(Part::Words))
            .is_none(),
        "another part is not the revocation"
    );
    let kept = store.gpg_revocation_file(id, 0, rev).expect("the kept one");
    assert!(kept.starts_with("This is a revocation certificate for the OpenPGP key:"));
    let new = store
        .gpg_new_revocation_file(
            id,
            0,
            1_780_000_300,
            faraday_pgp::Reason::Superseded,
            "",
            rev,
        )
        .expect("a new one");
    assert_ne!(**new, **kept);
    assert_eq!(
        vault::gpg_revocation_kept_at(store.records(id).get(0).expect("the key")),
        Some(1_780_000_100),
        "a new one does not replace the kept one"
    );
}

#[test]
fn a_secret_field_is_written_as_secret_and_a_public_one_as_public() {
    let c = compute();
    let file = made_with(&c, b"pass", vec![entry()]);
    let mut store = OpenVaults::default();
    let (id, _) = store.unlock(&c, file, b"pass").expect("opens");
    assert!(!store.set_public(id, 0, field::PASSWORD, b"x"));
    assert!(!store.set_secret(id, 0, field::TITLE, &zeroize::Zeroizing::new(b"x".to_vec())));
    assert!(store.set_public(id, 0, field::TITLE, b"Work"));
    assert!(store.set_secret(
        id,
        0,
        field::PASSWORD,
        &zeroize::Zeroizing::new(b"correct horse".to_vec())
    ));
    assert_eq!(
        store.records(id).get(0).unwrap().public_text(field::TITLE),
        Some("Work")
    );
    assert_eq!(
        &*store.reveal(
            &c,
            id,
            0,
            field::PASSWORD,
            Reveal::VaultEntry(Part::Password)
        ),
        "correct horse"
    );
}

#[test]
fn every_edit_moves_the_version_and_a_read_does_not() {
    let c = compute();
    let file = made_with(&c, b"pass", vec![entry()]);
    let mut store = OpenVaults::default();
    let (id, _) = store.unlock(&c, file, b"pass").expect("opens");
    let v0 = store.version(id);
    let _ = store.records(id).iter().count();
    let _ = store.reveal(
        &c,
        id,
        0,
        field::PASSWORD,
        Reveal::VaultEntry(Part::Password),
    );
    assert_eq!(store.version(id), v0, "reading changes nothing");
    store.set_public(id, 0, field::TITLE, b"Work");
    let v1 = store.version(id);
    assert!(v1 > v0);
    store.insert(id, Record::new(kind::NOTE).with(field::NOTE, b"n"));
    let v2 = store.version(id);
    assert!(v2 > v1);
    store.remove(id, 1);
    assert!(store.version(id) > v2);
}

#[test]
fn what_is_read_of_the_words_follows_the_record_it_was_read_from() {
    let c = compute();
    let m12 = Mnemonic::parse(&c, Language::English, ABANDON).expect("12 words");
    let zoo24 = format!("{} vote", ["zoo"; 23].join(" "));
    let m24 = Mnemonic::parse(&c, Language::English, &zoo24).expect("24 words");
    let key = |m: &Mnemonic| {
        Record::new(kind::KEY).with(field::KEY, &faraday_vault::records::words_payload(m))
    };
    let file = made_with(&c, b"pass", vec![key(&m12)]);
    let mut store = OpenVaults::default();
    let (id, _) = store.unlock(&c, file, b"pass").expect("opens");
    assert_eq!(store.word_count(&c, id, 0), Some(12));
    let before = store.words_id(&c, id, 0).copied();
    store.replace(id, 0, key(&m24));
    assert_eq!(
        store.word_count(&c, id, 0),
        Some(24),
        "the new record's words"
    );
    assert_ne!(store.words_id(&c, id, 0).copied(), before);
}

#[test]
fn a_key_record_copied_between_vaults_is_the_same_key_there() {
    let c = compute();
    let from = made(&c, b"first");
    let to = made_with(&c, b"second", Vec::new());
    let mut store = OpenVaults::default();
    let (a, _) = store.unlock(&c, from, b"first").expect("opens");
    let (b, _) = store.unlock(&c, to, b"second").expect("opens");
    assert_eq!(store.records(b).len(), 0);
    store.copy_record(a, 0, b, None).expect("copied");
    assert_eq!(store.records(b).len(), 1);
    let mut keys = Keys::on(Network::Mainnet);
    let loaded = store.load_from_vault(&c, &mut keys, b, &[0].into_iter().collect());
    assert_eq!(loaded[0].0, [0x73, 0xc5, 0xda, 0x0a]);
    let bare = store
        .copy_out(a, 0, Some(field::KEY_LABEL))
        .expect("a copy");
    assert!(
        !bare.view().has(field::KEY_LABEL),
        "lacking the field asked"
    );
}
