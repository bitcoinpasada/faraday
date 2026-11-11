//! The QR vault tool's secrets: a note, a key's words and a file vault's
//! item sealed as a QR vault and opened back to the same item, what does
//! not fit said as the contents count it, one code held at a time and
//! wiped at a lock, and committed codes opened or refused.

use faraday_bip::bip39::{Language, Mnemonic};
use faraday_bip::keys::{Fingerprint, Network};
use faraday_crypto::work::Compute;
use faraday_keys::vault::{self, Names, OpenVaults, SEAL_RANDOM_LEN, VaultId};
use faraday_keys::{
    Keys, QR_SEAL_EXPONENTS, QrError, QrFit, QrHolds, QrOpenJob, QrPassOffer, QrUnsupported,
    Reveal, qr_cost, qr_item_offered, qr_note_over, qr_seal_note_job,
};
use faraday_vault::Record;
use faraday_vault::contents::write_qr;
use faraday_vault::records::{field, kind, words_payload};
use zeroize::Zeroizing;

const ABANDON: &str =
    "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
const ZOO: &str = "zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo vote";
const SHARE: &str = "ms12namecacdefghjklmnpqrstuvwxyz023ftr2gdzmpy6pn";
const SHARE_KEY: [u8; 4] = [0xfa, 0xb6, 0x86, 0x8a];
const ABANDON_KEY: [u8; 4] = [0x73, 0xc5, 0xda, 0x0a];
/// The key of the 12-word phrase with the passphrase `TREZOR`.
const TREZOR_KEY: [u8; 4] = [0xb4, 0xe3, 0xf5, 0xed];
const PASS: &str = "correct horse battery staple";
const EXPONENT: u8 = 16;
const RANDOM: [u8; 95] = [7u8; 95];

#[expect(
    clippy::disallowed_methods,
    reason = "a mint: faraday-keys' tests, which cannot reach faraday-core's testkit"
)]
fn compute() -> Compute {
    Compute::new(|_| {})
}

fn committed(name: &str) -> Vec<u8> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tools/vectors/vault/qr")
        .join(name);
    std::fs::read(path).expect("the committed vector")
}

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

/// An open file vault holding `records`.
fn vault_with(c: &Compute, records: Vec<Record>) -> (OpenVaults, VaultId, Vec<usize>) {
    let file = vault::create(
        c,
        [7u8; 16],
        "Test",
        faraday_vault::Cost {
            memory_kib: 8192,
            passes: 1,
            lanes: 1,
        },
        "pass",
        &[7u8; SEAL_RANDOM_LEN],
        None,
    )
    .expect("a vault");
    let mut store = OpenVaults::default();
    store.set_names(names());
    let (id, _) = store.unlock(c, file, "pass").expect("opens");
    let places = records
        .into_iter()
        .map(|r| store.insert(id, r).expect("kept"))
        .collect();
    (store, id, places)
}

fn seed_record(c: &Compute, phrase: &str) -> Record {
    let m = Mnemonic::parse(c, Language::English, phrase).expect("words");
    Record::new(kind::KEY)
        .with(field::KEY, &words_payload(&m))
        .with(field::KEY_LABEL, b"Mine")
}

fn note(text: &str) -> Record {
    Record::new(kind::NOTE).with(field::NOTE, text.as_bytes())
}

/// A code opened under `pass` and held. Forgetting what was held before a
/// new open is the tool's job, not this store's.
fn open_code(keys: &mut Keys, c: &Compute, pass: &str, code: &[u8]) -> Result<QrHolds, QrError> {
    let opened = QrOpenJob::new(code, pass).run(c)?;
    keys.qr_hold(opened)
}

#[test]
fn a_note_is_sealed_and_opens_to_the_same_note() {
    let c = compute();
    let code = qr_seal_note_job(
        "The safe is in the drawer.",
        PASS,
        EXPONENT,
        Zeroizing::new(RANDOM),
    )
    .run(&c)
    .unwrap();
    assert_eq!(code.len(), 396);
    let mut keys = Keys::on(Network::Mainnet);
    assert_eq!(open_code(&mut keys, &c, PASS, &code), Ok(QrHolds::Note));
    assert_eq!(
        &*keys.qr_note(Reveal::QrVault),
        "The safe is in the drawer."
    );
    assert_eq!(
        &*keys.qr_note(Reveal::Words),
        "",
        "another reason lends nothing"
    );
}

#[test]
fn an_empty_note_seals_the_empty_vault() {
    let c = compute();
    let code = qr_seal_note_job("", PASS, EXPONENT, Zeroizing::new(RANDOM))
        .run(&c)
        .unwrap();
    let mut keys = Keys::on(Network::Mainnet);
    assert_eq!(open_code(&mut keys, &c, PASS, &code), Ok(QrHolds::Empty));
}

#[test]
fn a_loaded_key_of_12_or_24_words_is_sealed_and_loads_back() {
    let c = compute();
    for (phrase, count) in [(ABANDON, 12u8), (ZOO, 24)] {
        let mut keys = Keys::on(Network::Mainnet);
        keys.add_words(&c, phrase, "", None).unwrap();
        let code = keys
            .qr_seal_key_job(&c, 0, PASS, EXPONENT, Zeroizing::new(RANDOM))
            .and_then(|j| j.run(&c))
            .unwrap();
        let mut other = Keys::on(Network::Mainnet);
        assert_eq!(
            open_code(&mut other, &c, PASS, &code),
            Ok(QrHolds::Seed { words: count })
        );
        assert_eq!(&*other.qr_words(Reveal::QrVault), phrase);
    }
}

#[test]
fn a_key_of_18_words_or_another_list_is_refused() {
    let c = compute();
    let mut keys = Keys::on(Network::Mainnet);
    let eighteen = Mnemonic::from_entropy(Language::English, &[3u8; 24]).unwrap();
    keys.add_mnemonic(&c, &eighteen, "", "", None).unwrap();
    let other = Language::ALL
        .into_iter()
        .find(|l| *l != Language::English)
        .unwrap();
    let foreign = Mnemonic::from_entropy(other, &[5u8; 16]).unwrap();
    keys.add_mnemonic(&c, &foreign, "", "", None).unwrap();
    for i in 0..2 {
        assert!(!keys.qr_key_offered(i));
        assert_eq!(
            keys.qr_seal_key_job(&c, i, PASS, EXPONENT, Zeroizing::new(RANDOM))
                .and_then(|j| j.run(&c)),
            Err(QrError::NotFit(QrFit::SeedNotQr))
        );
    }
    assert_eq!(
        keys.qr_seal_key_job(&c, 5, PASS, EXPONENT, Zeroizing::new(RANDOM))
            .and_then(|j| j.run(&c)),
        Err(QrError::NoItem)
    );
}

#[test]
fn a_vault_items_seed_share_passphrase_and_note_are_sealed_as_that_item() {
    let c = compute();
    let (store, id, at) = vault_with(
        &c,
        vec![
            seed_record(&c, ZOO),
            Record::new(kind::CODEX32_SHARE)
                .with(field::CODEX32_STRING, SHARE.as_bytes())
                .with(field::CODEX32_KEY, &SHARE_KEY),
            Record::new(kind::PASSPHRASE)
                .with(field::PASSPHRASE_KEY, &TREZOR_KEY)
                .with(field::PASSPHRASE, b"TREZOR"),
            note("A note\nof two lines"),
        ],
    );
    let mut keys = Keys::on(Network::Mainnet);
    for (n, &i) in at.iter().enumerate() {
        assert_eq!(store.qr_fit(&c, id, i), QrFit::Fits);
        let code = store
            .qr_seal_item_job(&c, id, i, PASS, EXPONENT, Zeroizing::new(RANDOM))
            .unwrap()
            .run(&c)
            .unwrap();
        let held = open_code(&mut keys, &c, PASS, &code).unwrap();
        match n {
            0 => {
                assert_eq!(held, QrHolds::Seed { words: 24 });
                assert_eq!(&*keys.qr_words(Reveal::QrVault), ZOO);
            }
            1 => {
                assert_eq!(
                    held,
                    QrHolds::Share {
                        key: Fingerprint(SHARE_KEY)
                    }
                );
                assert_eq!(&*keys.qr_share(Reveal::QrVault), SHARE);
            }
            2 => {
                assert_eq!(
                    held,
                    QrHolds::Passphrase {
                        key: Fingerprint(TREZOR_KEY)
                    }
                );
                assert_eq!(&*keys.qr_passphrase(Reveal::QrVault), "TREZOR");
            }
            _ => {
                assert_eq!(held, QrHolds::Note);
                assert_eq!(&*keys.qr_note(Reveal::QrVault), "A note\nof two lines");
            }
        }
    }
}

#[test]
fn a_password_entry_a_wallet_and_a_gpg_key_get_no_button() {
    let c = compute();
    let (store, id, at) = vault_with(
        &c,
        vec![
            Record::new(kind::ENTRY)
                .with(field::TITLE, b"Bank")
                .with(field::PASSWORD, b"hunter2"),
            Record::new(kind::WALLET).with(field::WALLET, b"wsh(...)"),
            Record::new(kind::GPG),
        ],
    );
    for k in [
        kind::ENTRY,
        kind::WALLET,
        kind::GPG,
        kind::SHEET,
        kind::SECURE_BOOT,
        kind::WALLET_SHARE,
    ] {
        assert!(!qr_item_offered(k));
    }
    for k in [kind::KEY, kind::CODEX32_SHARE, kind::PASSPHRASE, kind::NOTE] {
        assert!(qr_item_offered(k));
    }
    for &i in &at {
        assert_eq!(store.qr_fit(&c, id, i), QrFit::NotOffered);
        assert_eq!(
            store
                .qr_seal_item_job(&c, id, i, PASS, EXPONENT, Zeroizing::new(RANDOM))
                .map(|_| ()),
            Err(QrError::NotFit(QrFit::NotOffered))
        );
    }
    assert_eq!(
        store
            .qr_seal_item_job(&c, id, 9, PASS, EXPONENT, Zeroizing::new(RANDOM))
            .map(|_| ()),
        Err(QrError::NoItem)
    );
}

#[test]
fn what_does_not_fit_says_why() {
    let c = compute();
    let eighteen = Mnemonic::from_entropy(Language::English, &[3u8; 24]).unwrap();
    let (store, id, at) = vault_with(
        &c,
        vec![
            Record::new(kind::KEY).with(field::KEY, &words_payload(&eighteen)),
            Record::new(kind::PASSPHRASE)
                .with(field::PASSPHRASE_KEY, &TREZOR_KEY)
                .with(field::PASSPHRASE, "p".repeat(193).as_bytes()),
            Record::new(kind::PASSPHRASE)
                .with(field::PASSPHRASE_KEY, &TREZOR_KEY)
                .with(field::PASSPHRASE, "p".repeat(194).as_bytes()),
            note(&"a".repeat(218)),
        ],
    );
    assert_eq!(store.qr_fit(&c, id, at[0]), QrFit::SeedNotQr);
    assert_eq!(store.qr_fit(&c, id, at[1]), QrFit::Fits);
    assert_eq!(store.qr_fit(&c, id, at[2]), QrFit::PassphraseTooLong);
    assert_eq!(store.qr_fit(&c, id, at[3]), QrFit::NoteTooLong(2));
    assert_eq!(
        store
            .qr_seal_item_job(&c, id, at[3], PASS, EXPONENT, Zeroizing::new(RANDOM))
            .map(|_| ()),
        Err(QrError::NotFit(QrFit::NoteTooLong(2)))
    );
}

#[test]
fn the_note_count_is_in_characters_with_escapes_and_utf8_counted() {
    let a = |n: usize| "a".repeat(n);
    assert_eq!(qr_note_over(""), 0);
    assert_eq!(qr_note_over(&a(216)), 0, "exactly at the limit fits");
    assert_eq!(qr_note_over(&a(217)), 1);
    assert_eq!(
        qr_note_over(&format!("{}\u{1f600}", a(216))),
        1,
        "an emoji is one character"
    );
    assert_eq!(
        qr_note_over(&format!("{}\"", a(215))),
        1,
        "a quote takes two bytes"
    );
    assert_eq!(qr_note_over(&format!("{}\"", a(214))), 0);
    assert_eq!(
        qr_note_over(&format!("{}\\", a(215))),
        1,
        "so does a backslash"
    );
    assert_eq!(
        qr_note_over(&format!("{}\n", a(215))),
        1,
        "and a line break"
    );
    assert_eq!(
        qr_note_over(&"\u{e9}".repeat(108)),
        0,
        "an accented letter is two bytes"
    );
    assert_eq!(qr_note_over(&"\u{e9}".repeat(109)), 1);
    assert_eq!(qr_note_over(&"\u{e9}".repeat(110)), 2);
    assert_eq!(qr_note_over(&"\"".repeat(109)), 1);
    // The count is what the contents themselves refuse and accept.
    for text in [
        format!("{}\u{1f600}\u{1f600}", a(214)),
        "\"\\\n\u{e9}".repeat(40),
        format!("{}\u{e9}", a(215)),
    ] {
        let over = qr_note_over(&text);
        let chars: Vec<char> = text.chars().collect();
        let kept: String = chars[..chars.len() - over].iter().collect();
        assert!(write_qr(&note(&kept)).is_ok(), "{over} off fits");
        if over > 0 {
            let one_more: String = chars[..chars.len() - over + 1].iter().collect();
            assert!(
                write_qr(&note(&one_more)).is_err(),
                "one fewer off does not"
            );
        }
    }
}

#[test]
fn a_wrong_passphrase_reads_as_wrong_and_nothing_is_held() {
    let c = compute();
    let code = qr_seal_note_job("hello", PASS, EXPONENT, Zeroizing::new(RANDOM))
        .run(&c)
        .unwrap();
    let mut keys = Keys::on(Network::Mainnet);
    assert_eq!(
        open_code(&mut keys, &c, "not it", &code),
        Err(QrError::WrongPassphrase)
    );
    assert_eq!(keys.qr_holds(), None);
    assert_eq!(
        open_code(&mut keys, &c, "", &code),
        Err(QrError::EmptyPassphrase)
    );
    assert_eq!(&*keys.qr_note(Reveal::QrVault), "");
}

#[test]
fn a_new_open_keeps_only_the_second_and_a_lock_wipes_it() {
    let c = compute();
    let first = qr_seal_note_job("first", PASS, EXPONENT, Zeroizing::new(RANDOM))
        .run(&c)
        .unwrap();
    let second = qr_seal_note_job("second", "other", EXPONENT, Zeroizing::new(RANDOM))
        .run(&c)
        .unwrap();
    let mut keys = Keys::on(Network::Mainnet);
    open_code(&mut keys, &c, PASS, &first).unwrap();
    assert!(keys.holds_secret());
    open_code(&mut keys, &c, "other", &second).unwrap();
    assert_eq!(&*keys.qr_note(Reveal::QrVault), "second");
    keys.clear();
    assert_eq!(keys.qr_holds(), None);
    assert_eq!(&*keys.qr_note(Reveal::QrVault), "");
    assert!(!keys.holds_secret());
    open_code(&mut keys, &c, PASS, &first).unwrap();
    keys.qr_forget();
    assert_eq!(keys.qr_holds(), None);
}

#[test]
fn offers_refuse_the_wrong_item() {
    let c = compute();
    let code = qr_seal_note_job("text", PASS, EXPONENT, Zeroizing::new(RANDOM))
        .run(&c)
        .unwrap();
    let mut keys = Keys::on(Network::Mainnet);
    open_code(&mut keys, &c, PASS, &code).unwrap();
    assert_eq!(keys.qr_add_passphrase(&c, ""), Err(QrError::NotHeld));
    assert_eq!(keys.qr_passphrase_offer(&c), None);
    assert_eq!(&*keys.qr_share(Reveal::QrVault), "");
}

#[test]
fn a_held_passphrase_is_added_to_the_key_its_words_make_with_it() {
    let c = compute();
    let mut keys = Keys::on(Network::Mainnet);
    open_code(&mut keys, &c, PASS, &committed("qr-passphrase.pgp")).unwrap();
    assert_eq!(keys.qr_passphrase_offer(&c), Some(QrPassOffer::NotLoaded));
    assert_eq!(keys.qr_add_passphrase(&c, ""), Err(QrError::NotLoaded));
    keys.add_words(&c, ABANDON, "", None).unwrap();
    assert_eq!(keys.info(0).unwrap().fingerprint, Fingerprint(ABANDON_KEY));
    assert_eq!(
        keys.qr_passphrase_offer(&c),
        Some(QrPassOffer::Add(Fingerprint(ABANDON_KEY)))
    );
    let fp = keys.qr_add_passphrase(&c, "With passphrase").unwrap();
    assert_eq!(keys.qr_passphrase_offer(&c), Some(QrPassOffer::Loaded));
    assert_eq!(fp, Fingerprint(TREZOR_KEY));
    assert_eq!(keys.len(), 2);
    assert!(keys.info(1).unwrap().has_passphrase);
    assert!(matches!(
        keys.qr_add_passphrase(&c, "Again"),
        Err(QrError::Key(faraday_intake::wallet::Refusal::Duplicate(f))) if f == fp
    ));
    assert_eq!(keys.len(), 2);
}

#[test]
fn a_passphrase_naming_a_plain_loaded_key_is_not_already_here() {
    let c = compute();
    let (store, id, at) = vault_with(
        &c,
        vec![
            Record::new(kind::PASSPHRASE)
                .with(field::PASSPHRASE_KEY, &ABANDON_KEY)
                .with(field::PASSPHRASE, b"TREZOR"),
        ],
    );
    let code = store
        .qr_seal_item_job(&c, id, at[0], PASS, EXPONENT, Zeroizing::new(RANDOM))
        .unwrap()
        .run(&c)
        .unwrap();
    let mut keys = Keys::on(Network::Mainnet);
    keys.add_words(&c, ABANDON, "", None).unwrap();
    open_code(&mut keys, &c, PASS, &code).unwrap();
    assert_eq!(keys.qr_passphrase_offer(&c), Some(QrPassOffer::NotLoaded));
}

#[test]
fn a_garbage_or_damaged_code_is_refused_and_nothing_is_held() {
    let c = compute();
    let mut keys = Keys::on(Network::Mainnet);
    assert!(open_code(&mut keys, &c, PASS, &[0x5a; 396]).is_err());
    assert_eq!(&*keys.qr_note(Reveal::QrVault), "");
    let mut bytes = committed("qr-note.pgp");
    let mid = bytes.len() / 2;
    bytes[mid] ^= 1;
    assert!(open_code(&mut keys, &c, PASS, &bytes).is_err());
    assert_eq!(&*keys.qr_note(Reveal::QrVault), "");
    assert_eq!(keys.qr_add_passphrase(&c, ""), Err(QrError::NotHeld));
    assert!(!keys.holds_secret());
}

#[test]
fn a_held_note_is_saved_as_a_new_note_in_an_open_vault() {
    let c = compute();
    let code = qr_seal_note_job("Kept", PASS, EXPONENT, Zeroizing::new(RANDOM))
        .run(&c)
        .unwrap();
    let (mut store, id, _) = vault_with(&c, vec![]);
    let mut keys = Keys::on(Network::Mainnet);
    assert_eq!(store.qr_save_note(&c, &keys, id), Err(QrError::NotHeld));
    open_code(&mut keys, &c, PASS, &code).unwrap();
    let at = store.qr_save_note(&c, &keys, id).unwrap();
    assert_eq!(
        &*store.reveal(
            &c,
            id,
            at,
            field::NOTE,
            Reveal::VaultEntry(faraday_keys::Part::Note)
        ),
        "Kept"
    );
}

#[test]
fn a_committed_code_opens_and_its_cost_reads_without_a_passphrase() {
    let c = compute();
    let bytes = committed("qr-note.pgp");
    let cost = qr_cost(&bytes).unwrap();
    assert_eq!((cost.exponent, cost.passes, cost.lanes), (16, 3, 4));
    assert!(QR_SEAL_EXPONENTS.contains(&cost.exponent));
    let mut keys = Keys::on(Network::Mainnet);
    assert_eq!(open_code(&mut keys, &c, PASS, &bytes), Ok(QrHolds::Note));
    assert_eq!(
        &*keys.qr_note(Reveal::QrVault),
        "The safe's code is in the drawer."
    );
    let share = committed("qr-codex32.pgp");
    assert_eq!(
        open_code(&mut keys, &c, PASS, &share),
        Ok(QrHolds::Share {
            key: Fingerprint(SHARE_KEY)
        })
    );
    assert_eq!(&*keys.qr_share(Reveal::QrVault), SHARE);
}

#[test]
fn what_is_not_a_qr_vault_is_named() {
    let c = compute();
    let mut keys = Keys::on(Network::Mainnet);
    let refused = committed("refused-compressed.pgp");
    assert_eq!(
        qr_cost(&refused),
        Err(QrError::NotQrVault(QrUnsupported::Compressed))
    );
    assert_eq!(
        open_code(&mut keys, &c, PASS, &refused),
        Err(QrError::NotQrVault(QrUnsupported::Compressed))
    );
    assert_eq!(qr_cost(b"not a message at all"), Err(QrError::NotAMessage));
}

#[test]
fn a_memory_step_outside_the_seal_steps_is_refused() {
    let c = compute();
    assert_eq!(
        qr_seal_note_job("x", PASS, 12, Zeroizing::new(RANDOM)).run(&c),
        Err(QrError::Cost)
    );
}

#[test]
fn a_code_is_not_held_while_a_stick_is_attached() {
    let c = compute();
    let code = qr_seal_note_job("a note", PASS, EXPONENT, Zeroizing::new(RANDOM))
        .run(&c)
        .unwrap();
    let mut keys = Keys::on(Network::Mainnet);
    keys.set_stick_attached(Some("stick"));
    assert_eq!(
        open_code(&mut keys, &c, PASS, &code),
        Err(QrError::Stick("stick"))
    );
    assert_eq!(keys.qr_holds(), None);
    keys.set_stick_attached(None);
    assert_eq!(open_code(&mut keys, &c, PASS, &code), Ok(QrHolds::Note));
}
