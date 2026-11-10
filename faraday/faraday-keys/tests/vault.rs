//! A vault opened and sealed again, and the keys it holds loaded: a key
//! record of the "abandon" words loads as the fingerprint BIP-39 and
//! BIP-32 publish for them, a vault sealed after a change opens with the
//! change, and a wrong passphrase opens nothing.

use faraday_bip::bip39::{Language, Mnemonic};
use faraday_bip::keys::Network;
use faraday_crypto::work::Compute;
use faraday_intake::secret_buf::SecretText;
use faraday_keys::Keys;
use faraday_keys::vault::{self, Names, OpenVaults, SEAL_RANDOM_LEN, gpg_field, is_secret};
use faraday_keys::{Part, Reveal};
use faraday_vault::file::STEPS_KIB;

/// The plaintext the tests that seal by hand use.
const SMALL: usize = 65_536;
use faraday_vault::kdbx::items::{self, Body, Place, Related, RelatedKind};
use faraday_vault::kdbx::xml::Document;
use faraday_vault::kdbx::{self};
use faraday_vault::records::{field, kind};
use faraday_vault::{Cost, Record};

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

/// The names the app writes, as the strings hold them.
fn names() -> Names {
    let s = |t: &str| t.to_string();
    Names {
        group_keys: s("Seeds"),
        group_wallets: s("Wallets"),
        group_notes: s("Notes"),
        group_sheets: s("Sheets"),
        group_gpg: s("GPG keys"),
        group_secure_boot: s("Secure Boot"),
        group_wallet_shares: s("Wallet shares"),
        group_passphrases: s("Passphrases"),
        group_shares: s("Codex32 shares"),
        title_key: s("Key {fingerprint}"),
        title_note: s("Note"),
        title_passphrase: s("Passphrase for {for}"),
        title_share: s("Codex32 share for {for}"),
        title_secure_boot: s("Secure Boot keys"),
    }
}

/// A store that writes `names()`.
fn store() -> OpenVaults {
    let mut s = OpenVaults::default();
    s.set_names(names());
    s
}

/// The cheapest cost the tests create a vault at: 8 MiB, one pass, one
/// lane.
const COST: Cost = Cost {
    memory_kib: 8192,
    passes: 1,
    lanes: 1,
};

/// A vault with no record, of the smallest size, under `passphrase`.
fn empty(c: &Compute, passphrase: &str) -> Vec<u8> {
    vault::create(
        c,
        [7u8; 16],
        "Test",
        COST,
        passphrase,
        &[7u8; SEAL_RANDOM_LEN],
        None,
    )
    .expect("a vault")
}

/// The file open vault `id` seals to, with the vault closed by it, its
/// new entries' UUIDs from `salt`. Costs the seal's own Argon2id run.
fn seal_with(store: &mut OpenVaults, c: &Compute, id: vault::VaultId, salt: u8) -> Vec<u8> {
    let uuids = uuids_of(store, id, salt);
    let file = store
        .seal_job(id, &[9u8; SEAL_RANDOM_LEN], &uuids, Some(1_780_000_000))
        .expect("a job")
        .run(c)
        .into_file()
        .expect("sealed");
    store.seal_ended(id, true);
    file
}

#[test]
fn a_seal_that_fails_leaves_the_vault_open_with_its_changes() {
    let c = compute();
    let note = |n: u8| Record::new(kind::NOTE).with(field::NOTE, &[n]);
    let mut store = store();
    let (id, _) = store.unlock(&c, empty(&c, "pass"), "pass").expect("opens");
    store.insert(id, note(1)).expect("kept");
    let len = store.records(id).len();
    let uuids = uuids_of(&store, id, 0x10);
    let job = store
        .seal_job(id, &[9u8; SEAL_RANDOM_LEN], &uuids, None)
        .expect("a job");
    assert!(
        store.insert(id, note(2)).is_none(),
        "edits refuse while sealing"
    );
    assert!(
        store
            .seal_job(id, &[9u8; SEAL_RANDOM_LEN], &uuids, None)
            .is_none()
    );
    drop(job);
    store.seal_ended(id, false);
    assert_eq!(store.records(id).len(), len, "the changes are kept");
    store.insert(id, note(2)).expect("edits work again");
    let file = seal(&mut store, &c, id);
    let (id, _) = store.unlock(&c, file, "pass").expect("opens");
    assert_eq!(store.records(id).len(), len + 1);
}

fn seal(store: &mut OpenVaults, c: &Compute, id: vault::VaultId) -> Vec<u8> {
    seal_with(store, c, id, 0x10)
}

/// `uuids` for a seal of `id`, each of its 16-byte chunks different.
fn uuids_of(store: &OpenVaults, id: vault::VaultId, salt: u8) -> Vec<u8> {
    let n = store.uuid_bytes(id).expect("held");
    (0..n).map(|i| salt + (i / 16) as u8).collect()
}

/// A vault of the smallest size holding `records`, under `passphrase`.
fn made_with(c: &Compute, passphrase: &str, records: Vec<Record>) -> Vec<u8> {
    let file = empty(c, passphrase);
    if records.is_empty() {
        return file;
    }
    let mut store = store();
    let (id, _) = store.unlock(c, file, passphrase).expect("opens");
    for r in records {
        store.insert(id, r).expect("kept");
    }
    seal_with(&mut store, c, id, 0x80)
}

/// A vault holding one key record of the "abandon" words under
/// `passphrase`.
fn made(c: &Compute, passphrase: &str) -> Vec<u8> {
    let m = Mnemonic::parse(c, Language::English, ABANDON).expect("the words");
    made_with(
        c,
        passphrase,
        vec![
            Record::new(kind::KEY)
                .with(field::KEY, &faraday_vault::records::words_payload(&m))
                .with(field::KEY_LABEL, b"Test"),
        ],
    )
}

/// The place of the first record of kind `k` in open vault `id`.
fn first(store: &OpenVaults, id: vault::VaultId, k: u8) -> usize {
    store.records(id).of(k).next().expect("a record of it").0
}

#[test]
fn a_key_in_a_vault_loads_as_the_key_the_bips_publish() {
    let c = compute();
    let file = made(&c, "first passphrase");
    let mut store = store();
    assert!(
        store.unlock(&c, file.clone(), "another one").is_err(),
        "a wrong passphrase opens nothing"
    );
    let (id, _) = store.unlock(&c, file, "first passphrase").expect("opens");
    let mut keys = Keys::on(Network::Mainnet);
    let all = store.records(id).of(kind::KEY).map(|(i, _)| i).collect();
    let loaded = store.load_from_vault(&c, &mut keys, id, &all, &Default::default());
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].0, [0x73, 0xc5, 0xda, 0x0a]);
    assert_eq!(keys.len(), 1);
}

#[test]
fn a_vault_sealed_after_a_change_opens_with_it() {
    let c = compute();
    let file = made(&c, "first passphrase");
    let mut store = store();
    let (id, _) = store.unlock(&c, file, "first passphrase").expect("opens");
    store.insert(id, Record::new(kind::NOTE).with(field::NOTE, b"kept"));
    let sealed = seal(&mut store, &c, id);
    assert!(store.records(id).is_empty(), "the seal took the vault out");
    let (again, _) = store
        .unlock(&c, sealed, "first passphrase")
        .expect("the sealed vault opens");
    let kinds: Vec<u8> = store.records(again).iter().map(|(_, r)| r.kind()).collect();
    assert_eq!(
        store.records(again).of(kind::NOTE).count(),
        1,
        "{kinds:?} {:?}",
        store.unreadable(again).len()
    );
    assert_eq!(
        store.records(again).of(kind::KEY).count(),
        1,
        "{kinds:?} {:?}",
        store.unreadable(again)
    );
}

/// Where the first record after the vault's label is, in a vault opened
/// from a file with one entry or key or a store's first insert.
const AT: usize = 1;

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
    let file = made_with(&c, "pass", vec![entry()]);
    let mut store = store();
    let (id, _) = store.unlock(&c, file, "pass").expect("opens");
    let r = store.records(id).get(AT).expect("the entry");
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
    let file = made_with(&c, "pass", vec![entry()]);
    let mut store = store();
    let (id, _) = store.unlock(&c, file, "pass").expect("opens");
    let shown = |why| store.reveal(&c, id, AT, field::PASSWORD, why).to_string();
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
            AT,
            field::NOTES,
            Reveal::VaultEntry(Part::EntryNotes)
        ),
        "recovery code 1234"
    );
    assert_eq!(
        &*store.reveal(&c, id, AT, field::TITLE, Reveal::VaultEntry(Part::Password)),
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
    let file = made_with(&c, "pass", vec![gpg_key()]);
    let mut store = store();
    let (id, _) = store.unlock(&c, file, "pass").expect("opens");
    let rev = Reveal::OutboxFile(Part::Revocation);
    assert!(
        store.gpg_revocation_file(id, AT, rev).is_none(),
        "none kept yet"
    );
    assert_eq!(
        store.gpg_keep_revocation(&c, id, AT, 1_780_000_100),
        Ok(true)
    );
    assert_eq!(
        store.gpg_keep_revocation(&c, id, AT, 1_780_000_200),
        Ok(false),
        "one is kept"
    );
    let r = store.records(id).get(AT).expect("the key");
    assert!(r.has(gpg_field::REVOCATION));
    assert_eq!(r.public(gpg_field::REVOCATION), None);
    assert_eq!(vault::gpg_revocation_kept_at(r), Some(1_780_000_100));
    assert!(
        store
            .gpg_revocation_file(id, AT, Reveal::OutboxFile(Part::Words))
            .is_none(),
        "another part is not the revocation"
    );
    let kept = store
        .gpg_revocation_file(id, AT, rev)
        .expect("the kept one");
    assert!(kept.starts_with("This is a revocation certificate for the OpenPGP key:"));
    let new = store
        .gpg_new_revocation_file(
            id,
            AT,
            1_780_000_300,
            faraday_pgp::Reason::Superseded,
            "",
            rev,
        )
        .expect("a new one");
    assert_ne!(**new, **kept);
    assert_eq!(
        vault::gpg_revocation_kept_at(store.records(id).get(AT).expect("the key")),
        Some(1_780_000_100),
        "a new one does not replace the kept one"
    );
}

#[test]
fn a_secret_field_is_written_as_secret_and_a_public_one_as_public() {
    let c = compute();
    let file = made_with(&c, "pass", vec![entry()]);
    let mut store = store();
    let (id, _) = store.unlock(&c, file, "pass").expect("opens");
    assert!(!store.set_public(id, AT, field::PASSWORD, b"x"));
    assert!(!store.set_secret(
        id,
        AT,
        field::TITLE,
        &zeroize::Zeroizing::new(b"x".to_vec())
    ));
    assert!(store.set_public(id, AT, field::TITLE, b"Work"));
    assert!(store.set_secret(
        id,
        AT,
        field::PASSWORD,
        &zeroize::Zeroizing::new(b"correct horse".to_vec())
    ));
    assert_eq!(
        store.records(id).get(AT).unwrap().public_text(field::TITLE),
        Some("Work")
    );
    assert_eq!(
        &*store.reveal(
            &c,
            id,
            AT,
            field::PASSWORD,
            Reveal::VaultEntry(Part::Password)
        ),
        "correct horse"
    );
}

#[test]
fn every_edit_moves_the_version_and_a_read_does_not() {
    let c = compute();
    let file = made_with(&c, "pass", vec![entry()]);
    let mut store = store();
    let (id, _) = store.unlock(&c, file, "pass").expect("opens");
    let v0 = store.version(id);
    let _ = store.records(id).iter().count();
    let _ = store.reveal(
        &c,
        id,
        AT,
        field::PASSWORD,
        Reveal::VaultEntry(Part::Password),
    );
    assert_eq!(store.version(id), v0, "reading changes nothing");
    store.set_public(id, AT, field::TITLE, b"Work");
    let v1 = store.version(id);
    assert!(v1 > v0);
    store.insert(id, Record::new(kind::NOTE).with(field::NOTE, b"n"));
    let v2 = store.version(id);
    assert!(v2 > v1);
    store.remove(id, AT + 1);
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
    let file = made_with(&c, "pass", vec![key(&m12)]);
    let mut store = store();
    let (id, _) = store.unlock(&c, file, "pass").expect("opens");
    assert_eq!(store.word_count(&c, id, AT), Some(12));
    let before = store.words_id(&c, id, AT).copied();
    store.replace(id, AT, key(&m24));
    assert_eq!(
        store.word_count(&c, id, AT),
        Some(24),
        "the new record's words"
    );
    assert_ne!(store.words_id(&c, id, AT).copied(), before);
}

#[test]
fn the_words_of_an_edited_key_are_judged_by_the_new_value_and_a_shifted_place_keeps_its_own() {
    let c = compute();
    let parse = |t: &str| Mnemonic::parse(&c, Language::English, t).expect("words");
    let legal = "legal winner thank year wave sausage worth useful legal winner thank yellow";
    let zoo24 = format!("{} vote", ["zoo"; 23].join(" "));
    let key = |t: &str| {
        Record::new(kind::KEY).with(
            field::KEY,
            &faraday_vault::records::words_payload(&parse(t)),
        )
    };
    let file = made_with(&c, "pass", vec![key(ABANDON), key(&zoo24), key(legal)]);
    let mut store = store();
    let (id, _) = store.unlock(&c, file, "pass").expect("opens");
    let (a, b, k) = (AT, AT + 1, AT + 2);
    store.warm(&c, id);
    let words = |s: &OpenVaults, i: usize| s.word_count(&c, id, i);
    let tell = |s: &OpenVaults, i: usize| s.words_id(&c, id, i).copied();
    let (id_abandon, id_zoo, id_legal) = (tell(&store, a), tell(&store, b), tell(&store, k));
    assert_eq!(
        [words(&store, a), words(&store, b), words(&store, k)],
        [Some(12), Some(24), Some(12)]
    );
    assert!(id_abandon != id_zoo && id_zoo != id_legal && id_abandon != id_legal);

    // The 24 words are replaced by 12: the count and the id are the new
    // words', the neighbours' are as they were.
    store.replace(id, b, key(ABANDON));
    assert_eq!(words(&store, b), Some(12));
    assert_eq!(tell(&store, b), id_abandon);
    assert_eq!((words(&store, a), tell(&store, a)), (Some(12), id_abandon));
    assert_eq!((words(&store, k), tell(&store, k)), (Some(12), id_legal));

    // Edited again, and then another field of it changed.
    store.replace(id, b, key(&zoo24));
    assert_eq!((words(&store, b), tell(&store, b)), (Some(24), id_zoo));
    store.set_public(id, b, field::TITLE, b"Work");
    assert_eq!((words(&store, b), tell(&store, b)), (Some(24), id_zoo));

    // A record put in before them: each place answers for its own record.
    assert!(store.insert_at(id, AT, Record::new(kind::NOTE).with(field::NOTE, b"n")));
    assert_eq!(words(&store, AT), None);
    assert_eq!(
        (words(&store, a + 1), tell(&store, a + 1)),
        (Some(12), id_abandon)
    );
    assert_eq!(
        (words(&store, b + 1), tell(&store, b + 1)),
        (Some(24), id_zoo)
    );
    assert_eq!(
        (words(&store, k + 1), tell(&store, k + 1)),
        (Some(12), id_legal)
    );

    // A record taken out before them, the note and then the first key.
    assert!(store.remove(id, AT));
    assert!(store.remove(id, a));
    assert_eq!((words(&store, a), tell(&store, a)), (Some(24), id_zoo));
    assert_eq!((words(&store, b), tell(&store, b)), (Some(12), id_legal));
    assert_eq!(words(&store, k), None);
}

#[test]
fn a_key_record_copied_between_vaults_is_the_same_key_there() {
    let c = compute();
    let from = made(&c, "first");
    let to = made_with(&c, "second", Vec::new());
    let mut store = store();
    let (a, _) = store.unlock(&c, from, "first").expect("opens");
    let (b, _) = store.unlock(&c, to, "second").expect("opens");
    assert_eq!(store.records(b).len(), AT, "only the label");
    store.copy_record(a, AT, b, &[]).expect("copied");
    assert_eq!(store.records(b).len(), AT + 1);
    let mut keys = Keys::on(Network::Mainnet);
    let loaded = store.load_from_vault(
        &c,
        &mut keys,
        b,
        &[AT].into_iter().collect(),
        &Default::default(),
    );
    assert_eq!(loaded[0].0, [0x73, 0xc5, 0xda, 0x0a]);
    let bare = store.copy_out(a, AT, &[field::KEY_LABEL]).expect("a copy");
    assert!(
        !bare.view().has(field::KEY_LABEL),
        "lacking the field asked"
    );
}

// ---- A GPG key restored from its paperkey (docs/GPG-PAPERKEY.md §8) ----

const MADE: u32 = 1_780_000_000;
const YEAR: u32 = 31_536_000;

/// A key as Make makes one, with seeds that start with zero bytes (a
/// shorter secret MPI).
fn gpg_made() -> faraday_keys::gpg::Made {
    let mut primary = zeroize::Zeroizing::new([0x31u8; 32]);
    primary[0] = 0;
    let mut subkey = zeroize::Zeroizing::new([0x42u8; 32]);
    subkey[..2].fill(0);
    faraday_keys::gpg::create(
        &primary,
        &subkey,
        MADE,
        "Test Person <test@example.com>".to_string(),
        2 * YEAR,
    )
}

/// The certificate armoured, read for a restore.
fn read_cert(packets: &[u8]) -> faraday_pgp::Certificate {
    let text = faraday_pgp::armor(faraday_pgp::Armor::PublicKey, packets);
    faraday_pgp::Certificate::read(text.as_bytes()).expect("a certificate of Faraday's shape")
}

/// Two vaults opened from one empty file, Make's key in the first: the
/// store, their ids, and the key's paperkey as its page shows it.
fn with_made(c: &Compute, made: &faraday_keys::gpg::Made) -> (OpenVaults, [vault::VaultId; 2]) {
    let file = made_with(c, "pass", Vec::new());
    let mut store = store();
    let (a, _) = store.unlock(c, file.clone(), "pass").expect("opens");
    let (b, _) = store.unlock(c, file, "pass").expect("opens");
    store.insert(a, &made.key).expect("kept");
    (store, [a, b])
}

/// The lines typed back, one at a time, as printed: number, colon, bytes
/// and CRC.
fn typed_back(restore: &mut faraday_keys::gpg::Restore, paperkey: &str) {
    for (i, l) in paperkey.lines().enumerate() {
        assert_eq!(restore.next(), i as u32 + 1);
        restore.line(i as u32 + 1, l).expect("each line is taken");
    }
    assert!(restore.complete());
}

/// Test 16: a key made by Make, its paperkey revealed and typed back
/// against its certificate: the record restored is Make's, field for
/// field and byte for byte, the kept revocation included.
#[test]
fn a_key_restored_from_its_paperkey_is_the_record_make_wrote() {
    let c = compute();
    let made = gpg_made();
    let (mut store, [a, b]) = with_made(&c, &made);
    let shown = store
        .gpg_paperkey(store.records(a).get(AT).unwrap(), Reveal::GpgPaperkey)
        .expect("the paperkey");
    let mut restore = faraday_keys::gpg::Restore::new(read_cert(&made.certificate));
    assert_eq!(restore.lines(), None, "the count waits for the lengths");
    typed_back(&mut restore, &shown);
    assert_eq!(restore.lines(), Some(7));
    assert_eq!(
        restore.fingerprints(),
        &[made.facts.fingerprint, made.facts.subkey_fingerprint]
    );
    let restored = restore.finish(&c).expect("restored");
    assert_eq!(restored.set_aside, 0);
    assert_eq!(restored.revoked, None);
    assert_eq!(restored.made.certificate, made.certificate);
    assert_eq!(restored.made.facts.fingerprint, made.facts.fingerprint);
    assert_eq!(restored.made.facts.expiry, made.facts.expiry);
    store.insert(b, &restored.made.key).expect("kept");
    let (mine, theirs) = (
        store.records(a).get(AT).unwrap(),
        store.records(b).get(AT).unwrap(),
    );
    for n in 1..=9 {
        assert_eq!(mine.has(n), theirs.has(n), "field {n}");
        assert_eq!(mine.public(n), theirs.public(n), "field {n}");
    }
    let rev = Reveal::OutboxFile(Part::Revocation);
    assert_eq!(
        **store.gpg_revocation_file(a, AT, rev).unwrap(),
        **store.gpg_revocation_file(b, AT, rev).unwrap(),
        "the kept revocation"
    );
    // The two vaults, sealed under one random, are the same bytes: every
    // field of the two records, secret ones included, is the same.
    assert_eq!(seal(&mut store, &c, a), seal(&mut store, &c, b));
}

/// A line mistyped is refused and nothing of it kept; once typed right,
/// the restore goes on; a line typed again clears it and those after.
#[test]
fn a_mistyped_line_is_refused_and_typed_again() {
    let c = compute();
    let made = gpg_made();
    let (store, [a, _]) = with_made(&c, &made);
    let shown = store
        .gpg_paperkey(store.records(a).get(AT).unwrap(), Reveal::GpgPaperkey)
        .expect("the paperkey");
    let lines: Vec<&str> = shown.lines().collect();
    let mut restore = faraday_keys::gpg::Restore::new(read_cert(&made.certificate));
    restore.line(1, lines[0]).expect("line 1");
    let wrong = lines[1].replacen(": 00 ", ": 01 ", 1);
    assert_eq!(
        restore.line(2, &wrong),
        Err(faraday_keys::gpg::LineRefused::Line(
            faraday_pgp::paperkey::LineError::Crc
        ))
    );
    assert_eq!(restore.next(), 2, "nothing of it is kept");
    assert_eq!(
        restore.line(3, lines[2]),
        Err(faraday_keys::gpg::LineRefused::Line(
            faraday_pgp::paperkey::LineError::Number { typed: 3 }
        ))
    );
    assert_eq!(
        restore.line(2, &lines[1].replacen("  2:", "  4:", 1)),
        Err(faraday_keys::gpg::LineRefused::Line(
            faraday_pgp::paperkey::LineError::Number { typed: 4 }
        ))
    );
    for (i, l) in lines.iter().enumerate().skip(1) {
        restore.line(i as u32 + 1, l).expect("taken");
    }
    assert!(restore.complete());
    restore.again(3);
    assert!(!restore.complete());
    assert_eq!(restore.next(), 3);
    for (i, l) in lines.iter().enumerate().skip(2) {
        restore.line(i as u32 + 1, l).expect("taken again");
    }
    assert!(restore.finish(&c).is_ok());
    // A lock: every line forgotten, the certificate kept.
    restore.wipe();
    assert_eq!(restore.next(), 1);
    assert_eq!(restore.checked(), 0);
    assert_eq!(restore.certificate().fingerprint(), made.facts.fingerprint);
    assert!(restore.finish(&c).is_err(), "nothing to restore from");
}

/// `gpg --verify` of `sig` over `data` with `cert` imported, at `now`.
/// Fails when gpg is missing.
fn gnupg_verifies(cert: &[u8], data: &[u8], sig: &str, now: u32) -> Option<String> {
    let dir = std::env::temp_dir().join(format!("faraday-keys-restore-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700)).unwrap();
    }
    let gpg = |args: &[&str]| {
        std::process::Command::new("gpg")
            .arg("--homedir")
            .arg(&dir)
            .args(["--batch", "--no-tty", "--status-fd", "1"])
            .arg("--faked-system-time")
            .arg(format!("{now}!"))
            .args(args)
            .output()
    };
    let files = [
        ("cert.asc", cert),
        ("data.txt", data),
        ("data.txt.asc", sig.as_bytes()),
    ];
    for (name, bytes) in files {
        std::fs::write(dir.join(name), bytes).unwrap();
    }
    let path = |n: &str| dir.join(n).to_str().unwrap().to_string();
    let out = match gpg(&["--import", &path("cert.asc")]) {
        Ok(o) => o,
        Err(e) => panic!("gpg is not installed ({e}); see docs/DEVELOPING.md"),
    };
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let out = gpg(&["--verify", &path("data.txt.asc"), &path("data.txt")]).unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    Some(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// Tests 17 and 18: the restored key signs a file with the same bytes as
/// Make's at the same time, which `gpg --verify` takes; Renew and Make a
/// revocation on it give what they give on Make's.
#[test]
fn a_restored_key_signs_renews_and_revokes_as_makes_does() {
    let c = compute();
    let made = gpg_made();
    let (mut store, [a, b]) = with_made(&c, &made);
    let shown = store
        .gpg_paperkey(store.records(a).get(AT).unwrap(), Reveal::GpgPaperkey)
        .expect("the paperkey");
    let mut restore = faraday_keys::gpg::Restore::new(read_cert(&made.certificate));
    typed_back(&mut restore, &shown);
    let restored = restore.finish(&c).expect("restored");
    store.insert(b, &restored.made.key).expect("kept");

    let data = b"A file signed after a restore.\n";
    let at = MADE + 600;
    let mine = store.gpg_sign_armored(a, AT, data, at).expect("signs");
    let theirs = store.gpg_sign_armored(b, AT, data, at).expect("signs");
    assert_eq!(mine, theirs);
    let cert = faraday_pgp::armor(faraday_pgp::Armor::PublicKey, &restored.made.certificate);
    if let Some(said) = gnupg_verifies(cert.as_bytes(), data, &theirs, at + 60) {
        assert!(
            said.contains("GOODSIG") && said.contains("VALIDSIG"),
            "{said}"
        );
    }

    let renewed = MADE + YEAR;
    let expiry = Some(renewed - MADE + 2 * YEAR);
    assert_eq!(
        store.gpg_certificate(a, AT, renewed, expiry),
        store.gpg_certificate(b, AT, renewed, expiry)
    );
    let rev = Reveal::OutboxFile(Part::Revocation);
    let (mine, theirs) = (
        store
            .gpg_new_revocation_file(a, AT, renewed, faraday_pgp::Reason::Retired, "Gone", rev)
            .expect("a revocation"),
        store
            .gpg_new_revocation_file(b, AT, renewed, faraday_pgp::Reason::Retired, "Gone", rev)
            .expect("a revocation"),
    );
    assert_eq!(**mine, **theirs);
}

/// A certificate with the key's revocation in it restores marked revoked
/// at the revocation's time, as Make's record is once marked.
#[test]
fn a_revoked_certificate_restores_a_record_marked_revoked() {
    let c = compute();
    let made = gpg_made();
    let (store, [a, _]) = with_made(&c, &made);
    let shown = store
        .gpg_paperkey(store.records(a).get(AT).unwrap(), Reveal::GpgPaperkey)
        .expect("the paperkey");
    let when = MADE + 900;
    let rev = store
        .gpg_new_revocation_file(
            a,
            AT,
            when,
            faraday_pgp::Reason::Compromised,
            "",
            Reveal::OutboxFile(Part::Revocation),
        )
        .expect("a revocation");
    // The revocation's packets, from the armour after the file's colon.
    let armoured = rev.split_once("\n:").expect("the colon").1;
    // Where OpenPGP puts it: straight after the primary key's packet.
    let cert = &made.certificate;
    let primary_end = 2 + usize::from(cert[1]);
    let mut both = cert[..primary_end].to_vec();
    both.extend(packets_of(armoured));
    both.extend_from_slice(&cert[primary_end..]);
    let mut restore = faraday_keys::gpg::Restore::new(read_cert(&both));
    typed_back(&mut restore, &shown);
    let restored = restore.finish(&c).expect("restored");
    assert_eq!(restored.revoked, Some(when));
    assert_eq!(
        restored.made.key.view().public(gpg_field::REVOKED),
        Some(&when.to_le_bytes()[..])
    );
}

/// The packets of one armoured block.
fn packets_of(armoured: &str) -> Vec<u8> {
    let body: String = armoured
        .lines()
        .skip_while(|l| !l.is_empty())
        .skip(1)
        .take_while(|l| !l.starts_with('='))
        .collect();
    let mut out = Vec::new();
    let mut acc = 0u32;
    let mut bits = 0;
    for ch in body.bytes() {
        let v = match ch {
            b'A'..=b'Z' => ch - b'A',
            b'a'..=b'z' => ch - b'a' + 26,
            b'0'..=b'9' => ch - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            _ => continue,
        };
        acc = acc << 6 | u32::from(v);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
            acc &= (1 << bits) - 1;
        }
    }
    out
}

/// paperkey 1.6's output for the key of [`paper_made`]: its base16 lines,
/// comment header and all, and its raw form
/// (`tools/vectors/paperkey/README.md`).
const PRINTED: &str = include_str!("../../../tools/vectors/paperkey/faraday-key.txt");
const RAW: &[u8] = include_bytes!("../../../tools/vectors/paperkey/faraday-key.raw");

/// The key paperkey printed [`PRINTED`] and [`RAW`] from, as Make makes it
/// (`faraday-pgp`'s example `paperkey_inputs`).
fn paper_made() -> faraday_keys::gpg::Made {
    let mut primary = zeroize::Zeroizing::new([0x11u8; 32]);
    primary[0] = 0;
    let mut subkey = zeroize::Zeroizing::new([0x22u8; 32]);
    subkey[..2].fill(0);
    faraday_keys::gpg::create(
        &primary,
        &subkey,
        MADE,
        "Test Person <test@example.com>".to_string(),
        2 * YEAR,
    )
}

/// The base16 lines of [`PRINTED`], its comments left out.
fn printed_lines() -> Vec<&'static str> {
    PRINTED
        .lines()
        .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
        .collect()
}

/// A paperkey read from a code: paperkey 1.6's base16 lines, comments and
/// all, and its raw form each fill every line, checked, and restore the
/// record that typing the lines restores.
#[test]
fn a_paperkey_code_in_either_of_paperkeys_forms_restores_the_key() {
    let c = compute();
    let made = paper_made();
    let typed = {
        let mut r = faraday_keys::gpg::Restore::new(read_cert(&made.certificate));
        for (i, l) in printed_lines().into_iter().enumerate() {
            r.line(i as u32 + 1, l).expect("each line is taken");
        }
        r.finish(&c).expect("restored from the lines")
    };
    for code in [PRINTED.as_bytes(), RAW] {
        let mut r = faraday_keys::gpg::Restore::new(read_cert(&made.certificate));
        r.paperkey_code(code).expect("the code is taken");
        assert!(r.complete());
        assert_eq!(r.checked(), 7, "every line, the CRC line included");
        assert_eq!(
            r.fingerprints(),
            &[made.facts.fingerprint, made.facts.subkey_fingerprint]
        );
        let restored = r.finish(&c).expect("restored from the code");
        assert_eq!(restored.made.certificate, made.certificate);
        assert_eq!(restored.made.certificate, typed.made.certificate);
        assert_eq!(restored.made.facts.fingerprint, made.facts.fingerprint);
    }
}

/// A paperkey code that does not hold is refused where typing it would
/// be, and the lines typed before it are left as they were.
#[test]
fn a_paperkey_code_that_does_not_hold_is_refused_and_keeps_the_lines_typed() {
    use faraday_keys::gpg::{CodeRefused, LineRefused};
    use faraday_pgp::Refusal;
    use faraday_pgp::paperkey::LineError;
    let made = paper_made();
    let mut r = faraday_keys::gpg::Restore::new(read_cert(&made.certificate));
    r.line(1, printed_lines()[0]).expect("line 1");
    // A digit changed on line 3: refused at line 3, as paperkey refuses it.
    let wrong = PRINTED.replacen("  3: 11", "  3: 12", 1);
    assert_eq!(
        r.paperkey_code(wrong.as_bytes()),
        Err(CodeRefused::Line(3, LineRefused::Line(LineError::Crc)))
    );
    // A byte of the raw form changed: its CRC does not match.
    let mut raw = RAW.to_vec();
    raw[30] ^= 1;
    assert_eq!(
        r.paperkey_code(&raw),
        Err(CodeRefused::Paperkey(Refusal::Crc))
    );
    // The lines without the whole data's CRC.
    let cut: String = printed_lines()[..6]
        .iter()
        .map(|l| format!("{l}\n"))
        .collect();
    assert_eq!(r.paperkey_code(cut.as_bytes()), Err(CodeRefused::Short));
    // Something after the whole data's CRC.
    let more = format!("{PRINTED}  8: 00 000000\n");
    assert_eq!(r.paperkey_code(more.as_bytes()), Err(CodeRefused::After(8)));
    // Something else entirely.
    assert_eq!(
        r.paperkey_code(b"cHNidP8BAH0CAAAAAQ"),
        Err(CodeRefused::NotPaperkey)
    );
    // Another key's paperkey: refused at line 1.
    let other = gpg_made();
    let mut theirs = faraday_keys::gpg::Restore::new(read_cert(&other.certificate));
    assert!(matches!(
        theirs.paperkey_code(PRINTED.as_bytes()),
        Err(CodeRefused::Line(
            1,
            LineRefused::Paperkey(Refusal::OtherKey(_))
        ))
    ));
    assert!(matches!(
        theirs.paperkey_code(RAW),
        Err(CodeRefused::Paperkey(Refusal::OtherKey(_)))
    ));
    // Line 1 typed before is still there.
    assert_eq!(r.checked(), 1);
    assert_eq!(r.next(), 2);
}

// ---- Files built here, and what the store does to them ----

/// A document of an empty vault, as a seal leaves one.
fn fresh_doc() -> Document {
    let mut doc = Document::new([7u8; 16], "Test");
    items::stamp(&mut doc, 1, None).expect("stamped");
    doc
}

/// The file of `doc` at the smallest size, at the tests' cost, under
/// "pass".
fn file_of(c: &Compute, doc: &Document) -> Vec<u8> {
    let random = [5u8; kdbx::RANDOM_LEN];
    let xml = doc.write(kdbx::inner_key_of(&random));
    let contents = kdbx::Contents {
        document: &xml,
        binaries: faraday_vault::kdbx::Binaries::all(&[]),
        public_custom_data: None,
    };
    kdbx::seal(c, "pass", COST, &random, &contents, SMALL).expect("sealed")
}

/// Puts `body` in `doc` as the entry `[n; 16]`, in group `group`.
fn put(c: &Compute, doc: &mut Document, n: u8, group: &str, body: &Body) {
    let place = Place {
        uuid: [n; 16],
        group,
        group_uuid: [0xa0; 16],
        title: "Title",
        now: None,
    };
    items::put(c, doc, &place, body).expect("put");
}

fn titled(title: &str) -> Body {
    Body::Record(Record::new(kind::ENTRY).with(field::TITLE, title.as_bytes()))
}

fn sealed_open(file: &[u8]) -> faraday_vault::file::Opened {
    faraday_vault::file::open(&compute(), file, "pass").expect("opens")
}

#[test]
fn a_change_by_place_is_a_change_of_that_entry_alone() {
    let c = compute();
    let mut doc = fresh_doc();
    put(&c, &mut doc, 1, "", &titled("Mail"));
    put(&c, &mut doc, 2, "", &titled("Bank"));
    let mut store = store();
    let (id, _) = store.unlock(&c, file_of(&c, &doc), "pass").expect("opens");
    let bank = store
        .records(id)
        .iter()
        .find(|(_, r)| r.public_text(field::TITLE) == Some("Bank"))
        .expect("the bank")
        .0;
    assert!(store.set_public(id, bank, field::TITLE, b"Work"));
    let sealed = seal(&mut store, &c, id);
    let (again, _) = store.unlock(&c, sealed.clone(), "pass").expect("opens");
    let mut titles: Vec<String> = store
        .records(again)
        .of(kind::ENTRY)
        .map(|(_, r)| r.public_text(field::TITLE).unwrap_or("").to_string())
        .collect();
    titles.sort();
    assert_eq!(titles, ["Mail", "Work"]);
    let mut uuids: Vec<[u8; 16]> = sealed_open(&sealed)
        .read()
        .items
        .iter()
        .map(|i| i.uuid)
        .collect();
    uuids.sort();
    assert_eq!(uuids, [[1; 16], [2; 16]], "the same two entries");
}

#[test]
fn a_new_plans_number_goes_past_one_that_does_not_read() {
    let c = compute();
    let mut doc = fresh_doc();
    doc.custom_mut()
        .set("faraday.plan.5", "not a plan")
        .expect("set");
    let mut store = store();
    let (id, _) = store.unlock(&c, file_of(&c, &doc), "pass").expect("opens");
    assert_eq!(store.unreadable(id).len(), 1, "the plan that does not read");
    store.insert(
        id,
        Record::new(kind::PLAN)
            .with(field::PLAN_WALLET, b"Wallet")
            .with(field::PLAN_PLACE, b"Home"),
    );
    let sealed = seal(&mut store, &c, id);
    let opened = sealed_open(&sealed);
    let keys: Vec<&str> = opened
        .read()
        .custom
        .iter()
        .map(|k| k.key.as_str())
        .collect();
    assert_eq!(keys, ["faraday.plan.6"]);
    assert_eq!(opened.read().unreadable.len(), 1, "kept as it was");
}

/// Whether `fits` says what a seal then takes, to the byte: `extra` records
/// and a note of `n` bytes are put in an empty vault. The plaintext a
/// seal takes is `used`; the store's limit is lowered to it, and then the note fits and one byte more does not.
fn fits_is_exact(c: &Compute, extra: &[Record]) {
    const N: usize = 30_000;
    let note = |n: usize| Record::new(kind::NOTE).with(field::NOTE, &vec![b'x'; n]);
    let file = empty(c, "pass");
    let plain = |n: usize| {
        let mut store = store();
        let (id, _) = store.unlock(c, file.clone(), "pass").expect("opens");
        for r in extra {
            store.insert(id, r.clone()).expect("kept");
        }
        store.insert(id, note(n)).expect("kept");
        // Past a limit of nothing the size is counted, not bounded.
        store.lower_limit(0);
        store.records(id).used(c)
    };
    let limit = plain(N);
    // A note is written Base64 (4 bytes for 3), so three bytes more is at
    // least four over.
    assert!(plain(N + 3) > limit);
    let fits = |n: usize| {
        let mut store = store();
        store.lower_limit(limit);
        let (id, _) = store.unlock(c, file.clone(), "pass").expect("opens");
        let filler = note(n);
        let mut add: Vec<&Record> = extra.iter().collect();
        add.push(&filler);
        store.fits(c, id, &[], &add)
    };
    assert!(fits(N - 1) && fits(N), "{N} bytes of note take {limit}");
    assert!(!fits(N + 3), "three bytes more are over");
}

/// A vault past its ceiling that is asked whether it has room, and then how
/// full it is, counts its plaintext once: the second asking makes no
/// document.
#[test]
fn a_vault_past_its_ceiling_is_counted_once() {
    let c = compute();
    let mut store = store();
    store.lower_limit(0);
    let (id, _) = store.unlock(&c, empty(&c, "pass"), "pass").expect("opens");
    let none: [&Record; 0] = [];
    let before = faraday_vault::kdbx::xml::writes();
    assert!(!store.fits(&c, id, &[], &none), "past a limit of nothing");
    let counted = faraday_vault::kdbx::xml::writes();
    assert!(counted > before, "the room was counted from a document");
    let used = store.records(id).counted(&c).expect("counted");
    assert!(used > 0);
    assert_eq!(
        faraday_vault::kdbx::xml::writes(),
        counted,
        "the count was made again"
    );
}

/// A person's edit of a vault that is nowhere near its ceiling writes no
/// document, and what was edited is sealed all the same.
#[test]
fn an_edit_under_the_ceiling_writes_no_document() {
    let c = compute();
    let mut store = store();
    let (id, _) = store.unlock(&c, empty(&c, "pass"), "pass").expect("opens");
    fn note(text: &[u8]) -> [(u8, &[u8]); 1] {
        [(field::NOTE, text)]
    }
    let before = faraday_vault::kdbx::xml::writes();
    let first = store
        .set_fields(&c, id, None, kind::NOTE, &note(b"first"))
        .expect("added");
    store
        .set_fields(&c, id, Some(first), kind::NOTE, &note(b"first, edited"))
        .expect("edited");
    store
        .set_fields(&c, id, None, kind::NOTE, &note(b"second"))
        .expect("added");
    let extra = Record::new(kind::NOTE).with(field::NOTE, b"more");
    assert!(store.fits(&c, id, &[], &[&extra]));
    assert_eq!(
        faraday_vault::kdbx::xml::writes(),
        before,
        "the edits wrote a document"
    );

    let file = seal_with(&mut store, &c, id, 1);
    let file_again = file.clone();
    let (back, _) = store.unlock(&c, file, "pass").expect("opens again");
    assert_eq!(store.records(back).of(kind::NOTE).count(), 2);
    // The texts the file holds: the edited one and the second, and not
    // the first as it was.
    let opened = kdbx::open(&c, &file_again, "pass").expect("opens");
    let doc = Document::read(&opened.document, &opened.inner_key).expect("reads");
    fn values(g: &faraday_vault::kdbx::xml::Group, out: &mut Vec<String>) {
        for e in g.entries() {
            out.extend(e.fields().map(|p| p.value().to_string()));
        }
        for s in g.groups() {
            values(s, out);
        }
    }
    let mut texts = Vec::new();
    values(doc.root(), &mut texts);
    texts.retain(|v| v.starts_with("first") || v == "second");
    texts.sort();
    assert_eq!(texts, ["first, edited", "second"]);
}

#[test]
fn fits_says_what_a_seal_takes_to_the_byte() {
    fits_is_exact(&compute(), &[]);
}

/// A key with no label has its title worked out where it is sealed; what
/// `fits` counts for it is the same length.
#[test]
fn fits_is_exact_for_a_key_with_no_label_too() {
    let c = compute();
    let m = Mnemonic::parse(&c, Language::English, ABANDON).expect("the words");
    let key = Record::new(kind::KEY).with(field::KEY, &faraday_vault::records::words_payload(&m));
    fits_is_exact(&c, &[key]);
}

#[test]
fn every_edit_is_refused_on_a_read_only_vault_for_either_reason() {
    let c = compute();
    let mut newer = fresh_doc();
    put(&c, &mut newer, 1, "", &titled("Mail"));
    newer.custom_mut().set("faraday.format", "2").expect("set");
    let mut one = fresh_doc();
    put(&c, &mut one, 1, "", &titled("Mail"));
    let mut text = String::from_utf8(one.write(&[0; 64]).to_vec()).expect("text");
    let at = text.find("</DatabaseName>").expect("a name") + "</DatabaseName>".len();
    text.insert_str(at, "<DatabaseName>second</DatabaseName>");
    let ambiguous = Document::read(text.as_bytes(), &[0; 64]).expect("reads");

    // What the same edits do to a vault that is not read-only.
    let mut open = store();
    let (w, _) = open.unlock(&c, file_of(&c, &one), "pass").expect("opens");
    assert!(open.read_only(w).is_none());

    for doc in [&newer, &ambiguous] {
        let (id, _) = open.unlock(&c, file_of(&c, doc), "pass").expect("opens");
        assert!(open.read_only(id).is_some());
        let (len, version) = (open.records(id).len(), open.version(id));
        let note = Record::new(kind::NOTE).with(field::NOTE, b"n");
        for i in 0..len + 1 {
            assert!(!open.set_public(id, i, field::TITLE, b"x"));
            assert!(!open.set_secret(
                id,
                i,
                field::PASSWORD,
                &zeroize::Zeroizing::new(b"x".to_vec())
            ));
            assert!(!open.set_loads_at_unlock(id, i, true));
            assert!(!open.replace(id, i, note.clone()));
            assert!(!open.remove(id, i));
            assert!(!open.insert_at(id, i, note.clone()));
            assert!(
                open.set_fields(&c, id, Some(i), kind::ENTRY, &[(field::TITLE, b"x")])
                    .is_err()
            );
            assert!(
                open.set_public_all_in(&c, id, &[i], field::TITLE, &[b"x"])
                    .is_err()
            );
        }
        assert!(open.insert(id, note.clone()).is_none());
        assert!(
            open.set_fields(&c, id, None, kind::NOTE, &[(field::NOTE, b"n")])
                .is_err()
        );
        assert_eq!(open.remove_where(id, |_| true), 0);
        assert!(open.copy_record(w, AT, id, &[]).is_none());
        assert!(!open.fits(&c, id, &[], &[] as &[&Record]));
        assert!(
            open.seal_job(id, &[9u8; SEAL_RANDOM_LEN], &[0u8; 64], None)
                .is_none()
        );
        assert_eq!(open.records(id).len(), len);
        assert_eq!(open.version(id), version, "nothing was edited");
    }
    assert!(open.set_public(w, AT, field::TITLE, b"x"), "a control");
}

/// The entry of a person's other app as that app wrote it.
fn entry_xml(n: u8, fields: &[(&str, &str)]) -> String {
    const A: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let u = [n; 16];
    let mut id = String::new();
    for ch in u.chunks(3) {
        let v = (u32::from(ch[0]) << 16)
            | (u32::from(*ch.get(1).unwrap_or(&0)) << 8)
            | u32::from(*ch.get(2).unwrap_or(&0));
        for k in 0..4 {
            if k <= ch.len() {
                id.push(A[((v >> (18 - 6 * k)) & 63) as usize] as char);
            } else {
                id.push('=');
            }
        }
    }
    let f: String = fields
        .iter()
        .map(|(k, v)| format!("<String><Key>{k}</Key><Value>{v}</Value></String>"))
        .collect();
    format!("<Entry><UUID>{id}</UUID>{f}</Entry>")
}

fn with_entries(xml: &str) -> Document {
    let mut text = String::from_utf8(fresh_doc().write(&[0; 64]).to_vec()).expect("text");
    let at = text.rfind("</Group></Root>").expect("the root's end");
    text.insert_str(at, xml);
    Document::read(text.as_bytes(), &[0; 64]).expect("reads")
}

#[test]
fn a_persons_entry_that_was_not_changed_keeps_its_own_fields_and_its_totp() {
    let c = compute();
    let b32 = "GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ";
    let doc = with_entries(&entry_xml(
        9,
        &[
            ("Title", "Mail"),
            ("UserName", "me"),
            ("Custom Thing", "kept"),
            ("TimeOtp-Secret-Base32", b32),
        ],
    ));
    let mut store = store();
    let (id, _) = store.unlock(&c, file_of(&c, &doc), "pass").expect("opens");
    let code = |store: &OpenVaults, id| {
        let at = first(store, id, kind::ENTRY);
        let t = store
            .totp(id, at, Some(59), Reveal::VaultEntry(Part::Totp))
            .expect("a TOTP entry");
        t.code
            .expect("reads")
            .expect("a code")
            .grouped
            .replace(' ', "")
    };
    assert_eq!(
        code(&store, id),
        "287082",
        "RFC 6238's SHA-1 vector at 59 s"
    );
    store.insert(
        id,
        Record::new(kind::NOTE).with(field::NOTE, b"another change"),
    );
    let sealed = seal(&mut store, &c, id);
    let (again, _) = store.unlock(&c, sealed.clone(), "pass").expect("opens");
    assert_eq!(code(&store, again), "287082");
    let opened = sealed_open(&sealed);
    let item = opened
        .read()
        .items
        .iter()
        .find(|i| i.uuid == [9; 16])
        .expect("the entry");
    let names: Vec<&str> = item.extra.iter().map(|e| e.name.as_str()).collect();
    assert!(names.contains(&"Custom Thing"), "{names:?}");
    assert!(names.contains(&"TimeOtp-Secret-Base32"), "{names:?}");
    assert!(!names.contains(&"otp"), "not turned into another form");
}

#[test]
fn a_gpg_key_that_is_not_faradays_own_is_unreadable_and_written_back_as_read() {
    use faraday_vault::kdbx::items::GpgKey;
    use zeroize::Zeroizing;
    let c = compute();
    let key = |seed: u8| faraday_pgp::Key {
        primary_seed: [seed; 32],
        created: MADE,
        subkey_seed: [seed + 1; 32],
        subkey_created: MADE,
        user_ids: vec!["Test Person <test@example.com>".to_string()],
        expiry: YEAR,
    };
    let k = key(0x11);
    let foreign = GpgKey {
        title: "Foreign".to_string(),
        key: Zeroizing::new(
            String::from_utf8(faraday_pgp::armor_secret(&k, MADE).to_vec()).expect("text"),
        ),
        // Another key's certificate: not the key's own.
        public: Some(faraday_pgp::armor(
            faraday_pgp::Armor::PublicKey,
            &key(0x33).certificate(MADE),
        )),
        revocation: None,
        revoked: None,
    };
    let mut doc = fresh_doc();
    put(&c, &mut doc, 4, "GPG keys", &Body::Gpg(foreign));
    let mut store = store();
    let (id, _) = store.unlock(&c, file_of(&c, &doc), "pass").expect("opens");
    assert_eq!(
        store.records(id).of(kind::GPG).count(),
        0,
        "no record of it"
    );
    let lost = store.unreadable(id);
    assert_eq!(lost.len(), 1);
    assert_eq!(lost[0].title, "Foreign");
    assert_eq!(lost[0].uuid, Some([4; 16]));
    store.insert(
        id,
        Record::new(kind::NOTE).with(field::NOTE, b"another change"),
    );
    let sealed = seal(&mut store, &c, id);
    let before = sealed_open(&file_of(&c, &doc));
    let after = sealed_open(&sealed);
    let gpg = |o: &faraday_vault::file::Opened| {
        o.read()
            .items
            .iter()
            .find(|i| i.uuid == [4; 16])
            .and_then(|i| match &i.body {
                Body::Gpg(g) => Some((g.title.clone(), g.key.to_string(), g.public.clone())),
                Body::Record(_) => None,
            })
            .expect("the key")
    };
    assert_eq!(gpg(&after), gpg(&before), "written back as it was read");
}

#[test]
fn a_key_with_two_passphrases_gives_both_each_with_its_place() {
    let c = compute();
    let passphrase = |key: [u8; 4], text: &[u8]| {
        Body::Record(
            Record::new(kind::PASSPHRASE)
                .with(field::PASSPHRASE_KEY, &key)
                .with(field::PASSPHRASE, text),
        )
    };
    let mut doc = fresh_doc();
    put(&c, &mut doc, 1, "Passphrases", &passphrase([1; 4], b"one"));
    put(&c, &mut doc, 2, "Passphrases", &passphrase([1; 4], b"two"));
    put(
        &c,
        &mut doc,
        3,
        "Passphrases",
        &passphrase([2; 4], b"other"),
    );
    let mut store = store();
    let (id, _) = store.unlock(&c, file_of(&c, &doc), "pass").expect("opens");
    assert!(store.unreadable(id).is_empty());
    let places: Vec<usize> = store
        .records(id)
        .of(kind::PASSPHRASE)
        .map(|(i, _)| i)
        .collect();
    assert_eq!(places.len(), 3, "each is kept");
    let of_key = |k: u8| -> Vec<usize> {
        places
            .iter()
            .copied()
            .filter(|&i| {
                store
                    .records(id)
                    .get(i)
                    .and_then(|r| r.public(field::PASSPHRASE_KEY))
                    == Some(&[k; 4][..])
            })
            .collect()
    };
    let (two, one) = (of_key(1), of_key(2));
    assert_eq!(two.len(), 2);
    for (n, &i) in two.iter().enumerate() {
        assert_eq!(
            store.related(id, i),
            Some(Related {
                kind: RelatedKind::Passphrases,
                place: n + 1,
                of: 2
            })
        );
    }
    assert_eq!(store.related(id, one[0]), None, "a passphrase alone");
    let read = sealed_open(&file_of(&c, &doc));
    let labelled = read
        .read()
        .items
        .iter()
        .filter(|i| i.related.is_some())
        .count();
    assert_eq!(labelled, 2, "the file's own reading says it too");
}

#[test]
fn an_edited_transactions_amounts_leave_no_trace_of_the_old_transaction() {
    let c = compute();
    let amounts = |tx: u8| {
        Record::new(kind::AMOUNTS)
            .with(field::AMOUNTS_TX, &[tx; 32])
            .with(field::AMOUNTS_INPUT, &5u64.to_le_bytes())
    };
    let mut doc = fresh_doc();
    items::put_signed(&mut doc, &amounts(1)).expect("put");
    let mut store = store();
    let (id, _) = store.unlock(&c, file_of(&c, &doc), "pass").expect("opens");
    let at = first(&store, id, kind::AMOUNTS);
    assert!(store.replace(id, at, amounts(2)));
    let sealed = seal(&mut store, &c, id);
    let opened = sealed_open(&sealed);
    let keys: Vec<&str> = opened
        .read()
        .custom
        .iter()
        .map(|k| k.key.as_str())
        .collect();
    assert_eq!(keys, [format!("faraday.signed.{}", "02".repeat(32))]);
}

fn amounts_of(tx: u8) -> Record {
    Record::new(kind::AMOUNTS)
        .with(field::AMOUNTS_TX, &[tx; 32])
        .with(field::AMOUNTS_INPUT, &5u64.to_le_bytes())
}

fn custom_keys(file: &[u8]) -> Vec<String> {
    sealed_open(file)
        .read()
        .custom
        .iter()
        .map(|k| k.key.clone())
        .collect()
}

#[test]
fn amounts_removed_and_added_again_are_still_there_after_a_seal() {
    let c = compute();
    let mut doc = fresh_doc();
    items::put_signed(&mut doc, &amounts_of(1)).expect("put");
    let mut store = store();
    let (id, _) = store.unlock(&c, file_of(&c, &doc), "pass").expect("opens");
    let at = first(&store, id, kind::AMOUNTS);
    assert!(store.remove(id, at));
    store.insert(id, amounts_of(1)).expect("added");
    let sealed = seal(&mut store, &c, id);
    assert_eq!(
        custom_keys(&sealed),
        [format!("faraday.signed.{}", "01".repeat(32))]
    );
}

/// Seals open vault `id` and says why it gave no file; the vault stays
/// open with its changes.
fn refused(store: &mut OpenVaults, c: &Compute, id: vault::VaultId) -> faraday_vault::file::Error {
    let uuids = uuids_of(store, id, 0x10);
    let err = store
        .seal_job(id, &[9u8; SEAL_RANDOM_LEN], &uuids, None)
        .expect("a job")
        .run(c)
        .error()
        .expect("refused");
    store.seal_ended(id, false);
    err
}

#[test]
fn a_change_the_seal_cannot_write_refuses_the_seal_and_keeps_the_vault_open() {
    let c = compute();
    let mut store = store();
    let (id, _) = store.unlock(&c, empty(&c, "pass"), "pass").expect("opens");
    let before = store.records(id).len();
    let round = Record::new(kind::ROUND);
    assert!(!store.fits(&c, id, &[], &[&round]), "fits agrees");
    store.insert(id, round).expect("kept");
    assert!(matches!(
        refused(&mut store, &c, id),
        faraday_vault::file::Error::Unwritable(_)
    ));
    assert_eq!(store.records(id).len(), before + 1, "the changes are kept");
}

#[test]
fn a_new_plan_with_no_number_left_refuses_the_seal_but_other_changes_seal() {
    let c = compute();
    let plan = Record::new(kind::PLAN)
        .with(field::PLAN_WALLET, b"Wallet")
        .with(field::PLAN_PLACE, b"Home");
    let mut doc = fresh_doc();
    items::put_plan(&mut doc, u32::MAX, &plan).expect("put");
    let mut store = store();
    let (id, _) = store.unlock(&c, file_of(&c, &doc), "pass").expect("opens");
    let note = Record::new(kind::NOTE).with(field::NOTE, b"n");
    assert!(store.fits(&c, id, &[], &[&note]), "no new plan, no number");
    assert!(!store.fits(&c, id, &[], &[&plan]));
    store.insert(id, plan.clone()).expect("kept");
    assert!(matches!(
        refused(&mut store, &c, id),
        faraday_vault::file::Error::Unwritable(_)
    ));
    let at = store.records(id).len() - 1;
    assert!(store.remove(id, at));
    store.insert(id, note).expect("kept");
    let sealed = seal(&mut store, &c, id);
    assert_eq!(custom_keys(&sealed), ["faraday.plan.4294967295"]);
}

/// Two new plans where the file has one number left: the second is
/// refused when it is added, as the seal would refuse it.
#[test]
fn a_second_new_plan_with_one_number_left_is_refused_where_the_first_fits() {
    let c = compute();
    let plan = Record::new(kind::PLAN)
        .with(field::PLAN_WALLET, b"Wallet")
        .with(field::PLAN_PLACE, b"Home");
    let mut doc = fresh_doc();
    items::put_plan(&mut doc, u32::MAX - 1, &plan).expect("put");
    let mut store = store();
    let (id, _) = store.unlock(&c, file_of(&c, &doc), "pass").expect("opens");
    assert!(store.fits(&c, id, &[], &[&plan]), "the last number");
    store.insert(id, plan.clone()).expect("kept");
    assert!(!store.fits(&c, id, &[], &[&plan]), "none left");
}

/// A record put with no check that the seal refuses is found by the next
/// check, once, and refuses it.
#[test]
fn a_record_put_unchecked_that_the_seal_refuses_makes_the_next_edit_refused() {
    let c = compute();
    let mut store = store();
    let (id, _) = store.unlock(&c, empty(&c, "pass"), "pass").expect("opens");
    let note = Record::new(kind::NOTE).with(field::NOTE, b"n");
    assert!(store.fits(&c, id, &[], &[&note]));
    let junk = Record::new(kind::SHEET).with(field::SHEET, b"junk");
    let at = store.insert(id, junk).expect("kept");
    assert!(!store.fits(&c, id, &[], &[&note]));
    assert!(matches!(
        refused(&mut store, &c, id),
        faraday_vault::file::Error::Unwritable(_)
    ));
    assert!(store.remove(id, at));
    assert!(store.fits(&c, id, &[], &[&note]));
}

/// Asking whether a vault fits with a record taken out does not take it
/// out: the record the seal refuses is still found afterwards.
#[test]
fn asking_to_fit_without_a_record_does_not_clear_it_of_the_seals_refusal() {
    let c = compute();
    let mut store = store();
    let (id, _) = store.unlock(&c, empty(&c, "pass"), "pass").expect("opens");
    let note = Record::new(kind::NOTE).with(field::NOTE, b"n");
    let junk = Record::new(kind::SHEET).with(field::SHEET, b"junk");
    let at = store.insert(id, junk).expect("kept");
    assert!(store.fits(&c, id, &[at], &[&note]), "without it, it fits");
    assert!(!store.fits(&c, id, &[], &[&note]), "but it is still there");
    assert!(matches!(
        refused(&mut store, &c, id),
        faraday_vault::file::Error::Unwritable(_)
    ));
}

/// A file whose entry has the UUID the checks count from does not make
/// every new record refused.
#[test]
fn an_entry_with_a_counting_uuid_does_not_make_every_new_record_refused() {
    let c = compute();
    let entry = "<Entry><UUID>AQAAAAAAAAAAAAAAAAAAAA==</UUID>\
        <String><Key>Title</Key><Value>a</Value></String>\
        <String><Key>Title</Key><Value>b</Value></String></Entry>";
    let doc = with_entries(entry);
    let mut store = store();
    let (id, _) = store.unlock(&c, file_of(&c, &doc), "pass").expect("opens");
    let note = Record::new(kind::NOTE).with(field::NOTE, b"n");
    assert!(store.fits(&c, id, &[], &[&note]));
}

#[test]
fn a_note_and_an_entry_with_nothing_typed_stay_editable_and_seal() {
    let c = compute();
    let mut store = store();
    let (id, _) = store.unlock(&c, empty(&c, "pass"), "pass").expect("opens");
    let n = store
        .set_fields(&c, id, None, kind::NOTE, &[(field::NOTE, b"text")])
        .expect("a note");
    store
        .set_fields(&c, id, Some(n), kind::NOTE, &[(field::NOTE, b"")])
        .expect("the text cleared is not a full vault");
    let e = store
        .set_fields(&c, id, None, kind::ENTRY, &[(1, b"Mail"), (2, b"me")])
        .expect("an entry");
    assert!(store.set_public(id, e, 1, b""), "the title cleared");
    let sealed = seal(&mut store, &c, id);
    let (id, _) = store.unlock(&c, sealed, "pass").expect("opens");
    assert_eq!(store.records(id).of(kind::NOTE).count(), 1);
    let (_, entry) = store.records(id).of(kind::ENTRY).next().expect("the entry");
    assert_eq!(entry.public(2), Some(&b"me"[..]), "its other fields stay");
}

#[test]
fn an_edit_of_an_entry_keeps_its_totp_code() {
    let c = compute();
    let doc = with_entries(&entry_xml(
        9,
        &[
            ("Title", "Mail"),
            ("UserName", "me"),
            ("TimeOtp-Secret-Base32", "GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ"),
        ],
    ));
    let mut store = store();
    let (id, _) = store.unlock(&c, file_of(&c, &doc), "pass").expect("opens");
    let at = first(&store, id, kind::ENTRY);
    assert!(store.set_public(id, at, 2, b"you"));
    let t = store
        .totp(id, at, Some(59), Reveal::VaultEntry(Part::Totp))
        .expect("still a TOTP entry");
    assert_eq!(
        t.code
            .expect("reads")
            .expect("a code")
            .grouped
            .replace(' ', ""),
        "287082"
    );
}

#[test]
fn an_otp_field_cleared_shows_no_code() {
    let c = compute();
    let doc = with_entries(&entry_xml(
        9,
        &[
            ("Title", "Mail"),
            ("UserName", "me"),
            (
                "otp",
                "otpauth://totp/Mail:me?secret=GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ",
            ),
        ],
    ));
    let mut store = store();
    let (id, _) = store.unlock(&c, file_of(&c, &doc), "pass").expect("opens");
    let at = first(&store, id, kind::ENTRY);
    let reason = Reveal::VaultEntry(Part::Totp);
    assert!(store.totp(id, at, Some(59), reason).is_some(), "a code");
    assert!(store.set_secret(id, at, field::TOTP, &zeroize::Zeroizing::new(Vec::new())));
    assert!(
        store.totp(id, at, Some(59), reason).is_none(),
        "no code once the field is cleared"
    );
}

#[test]
fn a_vault_name_taken_out_is_gone_after_a_seal() {
    let c = compute();
    let mut store = store();
    let (id, _) = store.unlock(&c, empty(&c, "pass"), "pass").expect("opens");
    let at = first(&store, id, kind::SLOT_LABEL);
    assert!(store.remove(id, at));
    let sealed = seal(&mut store, &c, id);
    assert_eq!(sealed_open(&sealed).read().label, "");
}

#[test]
fn two_strings_of_one_index_do_not_stop_the_key_loading() {
    let c = compute();
    let seed: Vec<u8> = (1..=16).collect();
    let mut owner = Keys::on(Network::Mainnet);
    owner.add_seed(&c, &seed, "").expect("a key");
    let fp = owner.info(0).expect("info").fingerprint.0;
    // The same four bytes after a share's own draw the same identifier:
    // two sets of one header, their strings differing.
    let need = faraday_bip::codex32::symbols_for(16);
    let split = |random: u8| -> Vec<String> {
        let mut bytes = vec![random; need + 4];
        bytes[need..].fill(7);
        owner
            .codex32_shares(0, 2, 3, &bytes, Reveal::PaperForm)
            .expect("shares")
            .iter()
            .map(|s| s.as_str().to_string())
            .collect()
    };
    let (right, other) = (split(1), split(2));
    let ident = |s: &str| {
        faraday_bip::codex32::Codex32::parse(s)
            .expect("parses")
            .identifier()
    };
    assert_eq!(ident(&right[0]), ident(&other[0]), "one set header");
    assert_ne!(right[0], other[0]);
    let mut store = store();
    let (id, _) = store.unlock(&c, empty(&c, "pass"), "pass").expect("opens");
    // The wrong string of an index comes first.
    for s in [&other[0], &right[0], &right[1]] {
        store
            .insert(
                id,
                Record::new(kind::CODEX32_SHARE)
                    .with(field::CODEX32_STRING, s.as_bytes())
                    .with(field::CODEX32_KEY, &fp),
            )
            .expect("kept");
    }
    assert_eq!(
        store.records(id).of(kind::CODEX32_SHARE).count(),
        3,
        "all kept"
    );
    let mut keys = Keys::on(Network::Mainnet);
    assert_eq!(
        store.gather_codex32(&c, &mut keys).len(),
        1,
        "the key loads"
    );
}

#[test]
fn a_typed_share_finds_its_seed_where_the_vault_holds_two_strings_of_an_index() {
    let c = compute();
    let seed: Vec<u8> = (1..=16).collect();
    let mut owner = Keys::on(Network::Mainnet);
    owner.add_seed(&c, &seed, "").expect("a key");
    let fp = owner.info(0).expect("info").fingerprint.0;
    // The same four bytes after a share's own draw the same identifier:
    // two sets of one header, their strings differing.
    let need = faraday_bip::codex32::symbols_for(16);
    let split = |random: u8| -> Vec<String> {
        let mut bytes = vec![random; need + 4];
        bytes[need..].fill(7);
        owner
            .codex32_shares(0, 2, 3, &bytes, Reveal::PaperForm)
            .expect("shares")
            .iter()
            .map(|s| s.as_str().to_string())
            .collect()
    };
    let (right, other) = (split(1), split(2));
    let mut store = store();
    let (id, _) = store.unlock(&c, empty(&c, "pass"), "pass").expect("opens");
    // The wrong string of index 0 comes first.
    for s in [&other[0], &right[0]] {
        store
            .insert(
                id,
                Record::new(kind::CODEX32_SHARE)
                    .with(field::CODEX32_STRING, s.as_bytes())
                    .with(field::CODEX32_KEY, &fp),
            )
            .expect("kept");
    }
    let typed = [faraday_bip::codex32::Codex32::parse(&right[1]).expect("parses")];
    assert_eq!(store.codex32_count(&typed), 1, "one more index is here");
    let mut keys = Keys::on(Network::Mainnet);
    let got = store.codex32_with_typed(&c, &mut keys, &typed, "typed");
    assert_eq!(
        got.expect("enough are in").expect("the seed comes back"),
        owner.info(0).expect("info").fingerprint
    );
}

#[test]
fn a_new_passphrase_and_step_are_sealed_and_a_failed_seal_keeps_them() {
    let c = compute();
    let mut store = store();
    let (id, _) = store.unlock(&c, empty(&c, "old"), "old").expect("opens");
    let v0 = store.version(id);
    assert!(!store.rekeyed(id));
    assert!(
        !store.set_passphrase(id, SecretText::new()),
        "an empty one is refused"
    );
    assert!(
        store.set_step(id, 100_000).is_none(),
        "a step not offered is refused"
    );
    assert_eq!(store.version(id), v0);
    assert!(store.set_passphrase(id, SecretText::of("new")));
    assert_eq!(store.set_step(id, STEPS_KIB[1]), Some(true));
    assert!(store.rekeyed(id));
    assert!(store.version(id) > v0, "it counts as a change");
    assert_eq!(
        store.next_cost(id).map(|k| k.memory_kib),
        Some(STEPS_KIB[1])
    );
    // A seal that fails is retried with the same change.
    let uuids = uuids_of(&store, id, 0x10);
    drop(store.seal_job(id, &[9u8; SEAL_RANDOM_LEN], &uuids, None));
    store.seal_ended(id, false);
    assert!(store.rekeyed(id));
    let file = seal(&mut store, &c, id);
    assert!(store.unlock(&c, file.clone(), "old").is_err());
    let (id, facts) = store.unlock(&c, file, "new").expect("opens under the new");
    assert_eq!(facts.header.cost.memory_kib, STEPS_KIB[1]);
    assert_eq!(store.seal_state(id).map(|(_, s)| s.count), Some(2));
}

#[test]
fn a_vault_being_sealed_refuses_a_new_passphrase_and_step() {
    let c = compute();
    let mut store = store();
    let (id, _) = store.unlock(&c, empty(&c, "old"), "old").expect("opens");
    let uuids = uuids_of(&store, id, 1);
    let _job = store
        .seal_job(id, &[9u8; SEAL_RANDOM_LEN], &uuids, None)
        .expect("a job");
    assert!(!store.set_passphrase(id, SecretText::of("new")));
    assert_eq!(store.set_step(id, STEPS_KIB[1]), None);
    assert!(!store.rekeyed(id));
}

#[test]
fn a_key_record_gets_no_keepass_export() {
    let c = compute();
    let mut store = store();
    let (id, _) = store.unlock(&c, made(&c, "first"), "first").expect("opens");
    let at = first(&store, id, kind::KEY);
    let job = store.kdbx_job(
        (id, at),
        "Seed",
        ([1; 16], "Export", None),
        SecretText::of("export"),
        COST,
        zeroize::Zeroizing::new([3u8; SEAL_RANDOM_LEN]),
        [5; 32],
    );
    assert!(job.is_none(), "a key stays in the vault");
}

fn note_of(text: &[u8]) -> [(u8, &[u8]); 1] {
    [(field::NOTE, text)]
}

/// A note that would take the vault past what a seal may write is refused,
/// whether it is added or an entry already there is made longer, and the
/// vault keeps what it had.
#[test]
fn a_note_past_what_a_seal_may_take_is_refused_and_the_vault_keeps_its_notes() {
    let c = compute();
    let file = empty(&c, "pass");
    let filler = vec![b'x'; 30_000];
    let used = {
        let mut store = store();
        let (id, _) = store.unlock(&c, file.clone(), "pass").expect("opens");
        store
            .set_fields(&c, id, None, kind::NOTE, &note_of(&filler))
            .expect("added");
        // Past a limit of nothing the size is counted, not bounded.
        store.lower_limit(0);
        store.records(id).used(&c)
    };
    let mut store = store();
    store.lower_limit(used);
    let (id, _) = store.unlock(&c, file, "pass").expect("opens");
    let first = store
        .set_fields(&c, id, None, kind::NOTE, &note_of(&filler))
        .expect("the note that fits is added");
    let longer = vec![b'x'; 30_100];
    assert_eq!(
        store.set_fields(&c, id, None, kind::NOTE, &note_of(&longer)),
        Err(vault::NotSet::Full)
    );
    assert_eq!(
        store.set_fields(&c, id, Some(first), kind::NOTE, &note_of(&longer)),
        Err(vault::NotSet::Full)
    );
    assert_eq!(store.records(id).of(kind::NOTE).count(), 1);
    let shorter = vec![b'y'; 100];
    store
        .set_fields(&c, id, Some(first), kind::NOTE, &note_of(&shorter))
        .expect("a shorter note still fits");
}

/// However many edits came before, one more is checked as the one change it
/// makes.
#[test]
fn an_edit_is_checked_as_one_change_however_many_came_before() {
    let c = compute();
    let mut store = store();
    let (id, _) = store.unlock(&c, empty(&c, "pass"), "pass").expect("opens");
    let mut last = 0;
    for n in 0..40u32 {
        let text = format!("note {n}");
        last = store
            .set_fields(&c, id, None, kind::NOTE, &note_of(text.as_bytes()))
            .expect("added");
    }
    let before = faraday_vault::file::checked();
    store
        .set_fields(&c, id, Some(last), kind::NOTE, &note_of(b"changed"))
        .expect("edited");
    assert_eq!(faraday_vault::file::checked() - before, 1);
}

/// Entries added, moved and taken out are each the entry a person meant
/// when it is edited after, and the file keeps exactly what is left.
#[test]
fn an_entry_edited_after_adds_moves_and_deletes_is_the_one_chosen() {
    let c = compute();
    let mut store = store();
    let (id, _) = store.unlock(&c, empty(&c, "pass"), "pass").expect("opens");
    let note = |t: &str| Record::new(kind::NOTE).with(field::NOTE, t.as_bytes());
    let places: Vec<usize> = ["a", "b", "c", "d"]
        .iter()
        .map(|t| store.insert(id, note(t)).expect("kept"))
        .collect();
    // "b" is taken out, so the places after it move down; "e" goes in front
    // of everything, so every place moves up.
    assert!(store.remove(id, places[1]));
    assert!(store.insert_at(id, 0, note("e")));
    let at = |t: &str| {
        let sought = Record::new(kind::NOTE).with(field::NOTE, t.as_bytes());
        store
            .records(id)
            .of(kind::NOTE)
            .find(|(_, r)| r.same_fields(&sought))
            .map(|(i, _)| i)
            .expect("there")
    };
    let (c_at, e_at) = (at("c"), at("e"));
    store
        .set_fields(&c, id, Some(c_at), kind::NOTE, &note_of(b"c, edited"))
        .expect("edited");
    store
        .set_fields(&c, id, Some(e_at), kind::NOTE, &note_of(b"e, edited"))
        .expect("edited");
    let file = seal_with(&mut store, &c, id, 1);
    let opened = kdbx::open(&c, &file, "pass").expect("opens");
    let doc = Document::read(&opened.document, &opened.inner_key).expect("reads");
    fn values(g: &faraday_vault::kdbx::xml::Group, out: &mut Vec<String>) {
        for e in g.entries() {
            out.extend(e.fields().map(|p| p.value().to_string()));
        }
        for s in g.groups() {
            values(s, out);
        }
    }
    let mut texts = Vec::new();
    values(doc.root(), &mut texts);
    texts.retain(|v| ["a", "c, edited", "d", "e, edited", "b", "c", "e"].contains(&v.as_str()));
    texts.sort();
    assert_eq!(texts, ["a", "c, edited", "d", "e, edited"]);
}
