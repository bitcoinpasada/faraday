//! A vault that needs a key file or a YubiKey's secret (`docs/VAULT.md`
//! §3.2, F208) keeps them while it is open: every seal writes a file that
//! opens with the same ones, taking one out is a change the next seal
//! writes, and a seal that fails changes nothing.
#![allow(clippy::disallowed_methods)]

use faraday_crypto::work::Compute;
use faraday_intake::secret_buf::SecretText;
use faraday_keys::job::StretchJob;
use faraday_keys::vault::{self, Names, OpenVaults, SEAL_RANDOM_LEN};
use faraday_keys::{Part, Reveal};
use faraday_vault::Cost;
use faraday_vault::kdbx::Components;
use faraday_vault::kdbx::keyfile;
use zeroize::Zeroizing;

const COST: Cost = Cost {
    memory_kib: 8192,
    passes: 1,
    lanes: 1,
};

const PASS: &str = "keyed pass";

#[expect(
    clippy::disallowed_methods,
    reason = "a mint: faraday-keys' tests, which cannot reach faraday-core's testkit"
)]
fn compute() -> Compute {
    Compute::new(|_| {})
}

fn store() -> OpenVaults {
    let mut s = OpenVaults::default();
    let t = |x: &str| x.to_string();
    s.set_names(Names {
        group_keys: t("Seeds"),
        group_wallets: t("Wallets"),
        group_notes: t("Notes"),
        group_sheets: t("Sheets"),
        group_gpg: t("GPG keys"),
        group_secure_boot: t("Secure Boot"),
        group_wallet_shares: t("Wallet shares"),
        group_passphrases: t("Passphrases"),
        group_shares: t("Codex32 shares"),
        title_key: t("Key {fingerprint}"),
        title_note: t("Note"),
        title_passphrase: t("Passphrase for {for}"),
        title_share: t("Codex32 share for {for}"),
        title_secure_boot: t("Secure Boot keys"),
    });
    s
}

fn key(n: u8) -> Zeroizing<[u8; 32]> {
    Zeroizing::new(std::array::from_fn(|i| (i as u8).wrapping_add(n)))
}

fn secret() -> Zeroizing<[u8; 20]> {
    Zeroizing::new(std::array::from_fn(|i| 0x40 + i as u8))
}

fn both() -> Components {
    Components::new(Some(key(1)), Some(secret()))
}

/// A vault under `PASS` and `components`, made by `create_holding`.
fn made(c: &Compute, components: Components) -> Vec<u8> {
    faraday_vault::file::create_holding(
        c,
        [7u8; 16],
        "Test",
        COST,
        PASS,
        &components,
        &[7u8; SEAL_RANDOM_LEN],
        None,
        &[],
    )
    .expect("a vault")
}

/// Opens `file` through a stretch job and keeps it.
fn open(
    store: &mut OpenVaults,
    c: &Compute,
    file: &[u8],
    components: Components,
) -> Option<vault::VaultId> {
    let stretched = StretchJob::open(file.to_vec(), SecretText::of(PASS))
        .keyed(components)
        .expect("an open takes any")
        .run(c);
    store.adopt(stretched).ok().map(|(id, _)| id)
}

fn seal(store: &mut OpenVaults, c: &Compute, id: vault::VaultId) -> Vec<u8> {
    let n = store.uuid_bytes(id).expect("held");
    let uuids: Vec<u8> = (0..n).map(|i| 0x10 + (i / 16) as u8).collect();
    let file = store
        .seal_job(id, &[9u8; SEAL_RANDOM_LEN], &uuids, None)
        .expect("a job")
        .run(c)
        .into_file()
        .expect("sealed");
    store.seal_ended(id, true);
    file
}

#[test]
fn a_vault_opened_with_a_key_file_and_a_secret_seals_with_both() {
    let c = compute();
    let mut store = store();
    let file = made(&c, both());
    assert!(
        open(&mut store, &c, &file, Components::none()).is_none(),
        "the passphrase alone does not open it"
    );
    let id = open(&mut store, &c, &file, both()).expect("opens with all three");
    assert_eq!(store.needs(id), Some((true, true)));
    assert!(!store.rekeyed(id));
    let sealed = seal(&mut store, &c, id);
    assert!(open(&mut store, &c, &sealed, Components::none()).is_none());
    assert!(open(&mut store, &c, &sealed, Components::with_key_file(key(1))).is_none());
    assert!(open(&mut store, &c, &sealed, both()).is_some());
}

#[test]
fn taking_the_key_file_out_is_written_at_the_next_seal() {
    let c = compute();
    let mut store = store();
    let id = open(&mut store, &c, &made(&c, both()), both()).expect("opens");
    assert!(store.set_key_file(id, None));
    assert!(store.rekeyed(id));
    assert_eq!(store.needs(id), Some((false, true)));
    let sealed = seal(&mut store, &c, id);
    assert!(open(&mut store, &c, &sealed, both()).is_none());
    let with = Components::with_yubikey(secret());
    assert!(open(&mut store, &c, &sealed, with).is_some());
}

#[test]
fn removing_the_yubikey_secret_is_written_at_the_next_seal() {
    let c = compute();
    let mut store = store();
    let id = open(&mut store, &c, &made(&c, both()), both()).expect("opens");
    assert!(store.remove_yubikey(id));
    assert!(store.rekeyed(id));
    assert_eq!(store.needs(id), Some((true, false)));
    let sealed = seal(&mut store, &c, id);
    assert!(open(&mut store, &c, &sealed, both()).is_none());
    let with = Components::with_key_file(key(1));
    assert!(open(&mut store, &c, &sealed, with).is_some());
}

#[test]
fn a_removal_of_the_secret_taken_back_keeps_it() {
    let c = compute();
    let mut store = store();
    let id = open(&mut store, &c, &made(&c, both()), both()).expect("opens");
    assert!(!store.keep_yubikey(id), "nothing to take back");
    assert!(store.remove_yubikey(id));
    assert!(store.yubikey_removal_pending(id));
    assert!(store.keep_yubikey(id));
    assert!(!store.yubikey_removal_pending(id));
    assert_eq!(store.needs(id), Some((true, true)));
    let sealed = seal(&mut store, &c, id);
    assert!(open(&mut store, &c, &sealed, Components::with_key_file(key(1))).is_none());
    assert!(open(&mut store, &c, &sealed, both()).is_some());
}

#[test]
fn keeping_the_secret_is_refused_where_removing_it_is() {
    let c = compute();
    let mut store = store();
    let none = open(
        &mut store,
        &c,
        &made(&c, Components::none()),
        Components::none(),
    )
    .expect("opens");
    assert!(!store.keep_yubikey(none), "no secret held");
    let id = open(&mut store, &c, &made(&c, both()), both()).expect("opens");
    assert!(store.remove_yubikey(id));
    // A job in flight: sealing refuses it.
    let n = store.uuid_bytes(id).expect("held");
    let uuids = vec![3u8; n];
    let _job = store
        .seal_job(id, &[9u8; SEAL_RANDOM_LEN], &uuids, None)
        .expect("a job");
    assert!(!store.keep_yubikey(id));
    assert!(store.yubikey_removal_pending(id));
}

const OUTBOX: Reveal = Reveal::OutboxFile(Part::KeyFile);

#[test]
fn a_made_key_file_document_is_handed_out_once() {
    let c = compute();
    let mut store = store();
    let id = open(
        &mut store,
        &c,
        &made(&c, Components::none()),
        Components::none(),
    )
    .expect("opens");
    assert!(store.make_key_file(id, &key(9)));
    assert!(store.key_file_document(id, OUTBOX).is_some());
    assert!(store.key_file_document(id, OUTBOX).is_none(), "taken");
    // Another key made is another document.
    assert!(store.make_key_file(id, &key(8)));
    assert!(store.key_file_document(id, OUTBOX).is_some());
}

#[test]
fn a_made_key_file_gives_no_document_while_the_vault_seals() {
    let c = compute();
    let mut store = store();
    let id = open(
        &mut store,
        &c,
        &made(&c, Components::none()),
        Components::none(),
    )
    .expect("opens");
    assert!(store.make_key_file(id, &key(9)));
    let n = store.uuid_bytes(id).expect("held");
    let uuids = vec![3u8; n];
    let _job = store
        .seal_job(id, &[9u8; SEAL_RANDOM_LEN], &uuids, None)
        .expect("a job");
    assert!(store.key_file_document(id, OUTBOX).is_none());
}

#[test]
fn a_made_key_file_that_could_not_be_kept_is_taken_back() {
    let c = compute();
    let mut store = store();
    let id = open(&mut store, &c, &made(&c, both()), both()).expect("opens");
    assert!(store.set_key_file(id, Some(key(2))));
    assert!(!store.unmake_key_file(id), "a chosen key is not a made one");
    assert!(store.make_key_file(id, &key(9)));
    assert!(store.key_file_document(id, OUTBOX).is_some());
    assert!(store.unmake_key_file(id));
    assert!(store.key_file_document(id, OUTBOX).is_none());
    // The vault keeps the key file it has.
    assert_eq!(store.needs(id), Some((true, true)));
    assert!(!store.rekeyed(id));
    // A seal writes the key file it had, and not the one made.
    let sealed = seal(&mut store, &c, id);
    assert!(open(&mut store, &c, &sealed, both()).is_some());
    let made_key = Components::new(Some(key(9)), Some(secret()));
    assert!(open(&mut store, &c, &sealed, made_key).is_none());
}

#[test]
fn a_made_key_file_through_a_failed_seal_is_not_handed_out_twice_and_a_later_seal_writes_it() {
    use faraday_vault::Record;
    use faraday_vault::records::kind;
    let c = compute();
    let mut store = store();
    let id = open(&mut store, &c, &made(&c, both()), both()).expect("opens");
    assert!(store.make_key_file(id, &key(9)));
    let document = store.key_file_document(id, OUTBOX).expect("the document");
    let read = keyfile::read(&document).expect("reads back");
    let at = store.insert(id, Record::new(kind::ROUND)).expect("kept");
    let n = store.uuid_bytes(id).expect("held");
    let uuids = vec![3u8; n];
    let err = store
        .seal_job(id, &[9u8; SEAL_RANDOM_LEN], &uuids, None)
        .expect("a job")
        .run(&c)
        .error();
    assert!(err.is_some(), "the seal failed");
    store.seal_ended(id, false);
    assert!(store.key_file_document(id, OUTBOX).is_none());
    assert!(store.rekeyed(id));
    assert!(store.remove(id, at));
    let sealed = seal(&mut store, &c, id);
    let new = Components::new(Some(read), Some(secret()));
    assert!(open(&mut store, &c, &sealed, new).is_some());
    assert!(open(&mut store, &c, &sealed, both()).is_none());
}

#[test]
fn a_key_file_made_is_written_as_a_document_and_opens_the_vault() {
    let c = compute();
    let mut store = store();
    let id = open(
        &mut store,
        &c,
        &made(&c, Components::none()),
        Components::none(),
    )
    .expect("opens");
    assert_eq!(store.needs(id), Some((false, false)));
    assert!(store.key_file_document(id, OUTBOX).is_none());
    assert!(store.make_key_file(id, &key(9)));
    assert_eq!(store.needs(id), Some((false, false)), "not taken yet");
    assert!(store.key_file_document(id, Reveal::Words).is_none());
    assert_eq!(store.needs(id), Some((false, false)), "wrong reason");
    let document = store.key_file_document(id, OUTBOX).expect("the document");
    assert_eq!(store.needs(id), Some((true, false)));
    let read = keyfile::read(&document).expect("reads back");
    assert_eq!(*read, *key(9));
    let sealed = seal(&mut store, &c, id);
    assert!(open(&mut store, &c, &sealed, Components::none()).is_none());
    assert!(open(&mut store, &c, &sealed, Components::with_key_file(read)).is_some());
}

#[test]
fn a_key_file_chosen_or_opened_with_gives_no_document() {
    let c = compute();
    let mut store = store();
    let id = open(&mut store, &c, &made(&c, both()), both()).expect("opens");
    assert!(store.key_file_document(id, OUTBOX).is_none(), "opened with");
    assert!(store.set_key_file(id, Some(key(2))));
    assert!(store.key_file_document(id, OUTBOX).is_none(), "chosen");
    // A chosen key after a made one takes the document away.
    assert!(store.make_key_file(id, &key(3)));
    assert!(store.key_file_document(id, OUTBOX).is_some());
    assert!(store.set_key_file(id, Some(key(4))));
    assert!(store.key_file_document(id, OUTBOX).is_none());
}

#[test]
fn a_make_draws_a_key_file_and_gives_its_document_but_refuses_a_secret() {
    let c = compute();
    let make = || {
        StretchJob::make(
            [7u8; 16],
            "Test".into(),
            COST,
            SecretText::of(PASS),
            Zeroizing::new([7u8; SEAL_RANDOM_LEN]),
            None,
        )
    };
    assert!(
        make().keyed(both()).is_err(),
        "no vault that needs a secret"
    );
    assert!(make().keyed(Components::with_yubikey(secret())).is_err());
    let stretched = make().making_key_file(&key(5)).run(&c);
    let document = stretched.key_file_document(OUTBOX).expect("the document");
    assert!(stretched.key_file_document(Reveal::Words).is_none());
    let read = keyfile::read(&document).expect("reads back");
    let file = stretched.into_made().expect("a vault");
    let mut store = store();
    assert!(open(&mut store, &c, &file, Components::none()).is_none());
    assert!(open(&mut store, &c, &file, Components::with_key_file(read)).is_some());
    // A chosen key file at a make has no document.
    let chosen = make()
        .keyed(Components::with_key_file(key(6)))
        .expect("a key file is taken")
        .run(&c);
    assert!(chosen.key_file_document(OUTBOX).is_none());
}

#[test]
fn a_key_file_replaced_is_written_and_the_old_one_no_longer_opens() {
    let c = compute();
    let mut store = store();
    let id = open(&mut store, &c, &made(&c, both()), both()).expect("opens");
    assert!(store.set_key_file(id, Some(key(2))));
    let sealed = seal(&mut store, &c, id);
    let with = |k| Components::new(Some(k), Some(secret()));
    assert!(open(&mut store, &c, &sealed, with(key(2))).is_some());
    assert!(open(&mut store, &c, &sealed, with(key(1))).is_none());
}

/// A vault that Faraday opens read-only (a newer format), under `both()`.
fn read_only_file(c: &Compute) -> Vec<u8> {
    use faraday_vault::kdbx::xml::Document;
    use faraday_vault::kdbx::{self, Binaries, Contents, items};
    let mut doc = Document::new([7u8; 16], "Test");
    items::stamp(&mut doc, 1, None).expect("stamped");
    doc.custom_mut().set("faraday.format", "2").expect("set");
    let random = [5u8; kdbx::RANDOM_LEN];
    let xml = doc.write(kdbx::inner_key_of(&random));
    let contents = Contents {
        document: &xml,
        binaries: Binaries::all(&[]),
        public_custom_data: None,
    };
    kdbx::seal_with(c, PASS, &both(), COST, &random, &contents, 65_536).expect("sealed")
}

#[test]
fn a_read_only_vault_refuses_a_change_of_its_components() {
    let c = compute();
    let mut store = store();
    let id = open(&mut store, &c, &read_only_file(&c), both()).expect("opens");
    assert!(store.read_only(id).is_some());
    assert!(!store.set_key_file(id, Some(key(2))));
    assert!(!store.set_key_file(id, None));
    assert!(!store.make_key_file(id, &key(3)));
    assert!(!store.remove_yubikey(id));
    assert!(!store.rekeyed(id));
    assert_eq!(store.needs(id), Some((true, true)));
}

#[test]
fn a_vault_without_a_secret_cannot_be_given_one_or_lose_one() {
    let c = compute();
    let mut store = store();
    let id = open(
        &mut store,
        &c,
        &made(&c, Components::none()),
        Components::none(),
    )
    .expect("opens");
    assert!(!store.remove_yubikey(id));
    assert!(!store.set_key_file(id, None), "no key file to take out");
    assert!(!store.rekeyed(id));
}

#[test]
fn a_seal_that_fails_keeps_the_components_and_the_change() {
    use faraday_vault::Record;
    use faraday_vault::records::kind;
    let c = compute();
    let mut store = store();
    let id = open(&mut store, &c, &made(&c, both()), both()).expect("opens");
    assert!(store.remove_yubikey(id));
    // A record the seal cannot write makes the seal fail.
    let at = store.insert(id, Record::new(kind::ROUND)).expect("kept");
    let n = store.uuid_bytes(id).expect("held");
    let uuids = vec![3u8; n];
    let err = store
        .seal_job(id, &[9u8; SEAL_RANDOM_LEN], &uuids, None)
        .expect("a job")
        .run(&c)
        .error();
    assert!(err.is_some(), "the seal failed");
    store.seal_ended(id, false);
    assert!(store.rekeyed(id));
    assert_eq!(store.needs(id), Some((true, false)));
    assert!(store.remove(id, at));
    let sealed = seal(&mut store, &c, id);
    let with = Components::with_key_file(key(1));
    assert!(open(&mut store, &c, &sealed, with).is_some());
    assert!(open(&mut store, &c, &sealed, both()).is_none());
}

#[test]
fn a_vault_being_sealed_refuses_a_change_of_its_components() {
    let c = compute();
    let mut store = store();
    let id = open(&mut store, &c, &made(&c, both()), both()).expect("opens");
    let n = store.uuid_bytes(id).expect("held");
    assert!(store.make_key_file(id, &key(5)));
    let _job = store.seal_job(id, &[9u8; SEAL_RANDOM_LEN], &vec![3u8; n], None);
    assert!(store.key_file_document(id, OUTBOX).is_none());
    assert!(!store.set_key_file(id, Some(key(2))));
    assert!(!store.remove_yubikey(id));
    assert!(!store.rekeyed(id));
}

#[test]
fn a_key_drawn_is_written_only_after_its_document_is_taken() {
    let c = compute();
    let mut store = store();
    let id = open(
        &mut store,
        &c,
        &made(&c, Components::none()),
        Components::none(),
    )
    .expect("opens");
    assert!(store.make_key_file(id, &key(9)));
    // A lock before the document is taken has nothing to seal.
    assert!(!store.rekeyed(id));
    let sealed = seal(&mut store, &c, id);
    assert!(open(&mut store, &c, &sealed, Components::none()).is_some());
    assert!(open(&mut store, &c, &sealed, Components::with_key_file(key(9))).is_none());
    // After it is taken, the seal writes the new key.
    let id = open(&mut store, &c, &sealed, Components::none()).expect("opens");
    assert!(store.make_key_file(id, &key(9)));
    assert!(store.key_file_document(id, OUTBOX).is_some());
    let sealed = seal(&mut store, &c, id);
    assert!(open(&mut store, &c, &sealed, Components::none()).is_none());
    assert!(open(&mut store, &c, &sealed, Components::with_key_file(key(9))).is_some());
}

#[test]
fn a_second_draw_before_its_document_leaves_the_file_under_the_key_it_had() {
    for had in [None, Some(key(1))] {
        let c = compute();
        let mut store = store();
        let components = Components::new(had.clone(), None);
        let id = open(
            &mut store,
            &c,
            &made(&c, components),
            Components::new(had.clone(), None),
        )
        .expect("opens");
        assert!(store.make_key_file(id, &key(8)));
        assert!(store.key_file_document(id, OUTBOX).is_some());
        assert!(store.make_key_file(id, &key(9)));
        let sealed = seal(&mut store, &c, id);
        assert!(open(&mut store, &c, &sealed, Components::new(had.clone(), None)).is_some());
        assert!(open(&mut store, &c, &sealed, Components::with_key_file(key(9))).is_none());
        assert!(open(&mut store, &c, &sealed, Components::with_key_file(key(8))).is_none());
    }
}

#[test]
fn a_make_gives_a_key_file_document_only_for_the_draw_that_made_the_key() {
    let c = compute();
    let make = || {
        StretchJob::make(
            [7u8; 16],
            "Test".into(),
            COST,
            SecretText::of(PASS),
            Zeroizing::new([7u8; SEAL_RANDOM_LEN]),
            None,
        )
    };
    let none = make()
        .making_key_file(&key(5))
        .keyed(Components::none())
        .expect("taken")
        .run(&c);
    assert!(none.key_file_document(OUTBOX).is_none());
    let chosen = make()
        .making_key_file(&key(5))
        .keyed(Components::with_key_file(key(6)))
        .expect("taken")
        .run(&c);
    assert!(chosen.key_file_document(OUTBOX).is_none());
    // A make that fails gives no document.
    let failed = StretchJob::make(
        [7u8; 16],
        "Test".into(),
        COST,
        SecretText::of(""),
        Zeroizing::new([7u8; SEAL_RANDOM_LEN]),
        None,
    )
    .making_key_file(&key(5))
    .run(&c);
    assert!(failed.error().is_some(), "an empty passphrase fails");
    assert!(failed.key_file_document(OUTBOX).is_none());
}
