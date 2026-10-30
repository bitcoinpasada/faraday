use alloc::vec;
use alloc::vec::Vec;

use super::records::{Contents, Fault, Record, Seals, field, kind};
use super::*;

/// A token for the tests' key work (`docs/COMPUTE.md` §3), counting
/// nothing: faraday-core's `testkit::compute` is out of this crate's
/// reach. Cheap.
#[expect(
    clippy::disallowed_methods,
    reason = "a mint: faraday-vault's tests, which cannot reach faraday-core's testkit"
)]
fn compute() -> Compute {
    Compute::new(|_| {})
}

/// The cheapest cost a reader accepts: what these tests are about is
/// not the cost.
const CHEAP: Cost = Cost {
    memory_kib: MIN_MEMORY_KIB,
    passes: 1,
    lanes: 1,
};

const SMALL: u32 = SLOT_SIZES[0];
/// What follows a small slot's commitment block.
const BODY: usize = SMALL as usize - COMMIT_LEN;

fn entry(title: &str, password: &str) -> Contents {
    Contents {
        records: vec![
            Record::new(kind::ENTRY)
                .with(field::TITLE, title.as_bytes())
                .with(field::PASSWORD, password.as_bytes()),
        ],
    }
}

fn title_of(c: &Contents) -> &str {
    c.records[0].text(field::TITLE).unwrap()
}

/// RFC 5869 test case 3: SHA-256, a 22-byte key of 0x0b, no salt and no
/// info. The first 32 bytes of its output.
#[test]
fn hkdf_matches_rfc_5869() {
    let okm = hkdf_sha256(&[0x0b; 22], b"");
    let want = [
        0x8d, 0xa4, 0xe7, 0x75, 0xa5, 0x63, 0xc1, 0x8f, 0x71, 0x5f, 0x80, 0x2a, 0x06, 0x3c, 0x5a,
        0x31, 0xb8, 0xa1, 0x1f, 0x5c, 0x5e, 0xe1, 0x87, 0x9e, 0xc3, 0x45, 0x4e, 0x5f, 0x3c, 0x73,
        0x8d, 0x2d,
    ];
    assert_eq!(okm.expose(), &want);
}

/// Three passphrases each open their own slot and nothing else; a fourth
/// passphrase opens nothing; the file is the size §2 gives whatever the
/// number of passphrases.
#[test]
fn each_passphrase_opens_its_own_contents() {
    let c = compute();
    let file = create(
        &c,
        SMALL,
        CHEAP,
        &["first", "second", "third"],
        &[entry("Email", "a"), entry("Bank", "b"), entry("Decoy", "c")],
        &[7; 32],
    )
    .unwrap();
    assert_eq!(file.len(), 262_357);
    for (p, t) in [("first", "Email"), ("second", "Bank"), ("third", "Decoy")] {
        let (_, got) = open(&c, &file, p).unwrap();
        assert_eq!(title_of(&got), t);
    }
    assert_eq!(open(&c, &file, "fourth").err(), Some(Error::Passphrase));
}

/// Sealing re-encrypts the open slot and leaves every other byte of the
/// file as it was, so a slot another passphrase opens survives a session
/// that never knew of it.
#[test]
fn sealing_changes_only_the_open_slot() {
    let c = compute();
    let file = create(
        &c,
        SMALL,
        CHEAP,
        &["first", "second"],
        &[entry("Email", "a"), entry("Bank", "b")],
        &[9; 32],
    )
    .unwrap();
    let (mut opened, _) = open(&c, &file, "first").unwrap();
    let sealed = opened
        .seal(&file, &entry("Email, changed", "a2"), &[3; NONCE_LEN], None)
        .unwrap();
    assert_eq!(sealed.len(), file.len());
    let changed: Vec<usize> = (0..SLOTS)
        .filter(|&i| {
            let at = slot_at(SMALL, i);
            let end = at + SMALL as usize + NONCE_LEN + TAG_LEN;
            file[at..end] != sealed[at..end]
        })
        .collect();
    assert_eq!(changed.len(), 1);
    assert_eq!(file[..HEADER_LEN], sealed[..HEADER_LEN]);
    assert_eq!(
        title_of(&open(&c, &sealed, "first").unwrap().1),
        "Email, changed"
    );
    assert_eq!(title_of(&open(&c, &sealed, "second").unwrap().1), "Bank");
}

/// A changed header fails every slot rather than opening under other
/// parameters.
#[test]
fn a_changed_header_opens_nothing() {
    let c = compute();
    let mut file = create(
        &c,
        SMALL,
        CHEAP,
        &["only"],
        &[entry("Email", "a")],
        &[1; 32],
    )
    .unwrap();
    file[30] ^= 1;
    assert_eq!(open(&c, &file, "only").err(), Some(Error::Passphrase));
}

/// A cost outside §3's limits, or a slot size §2 does not list, is
/// refused from the header alone.
#[test]
fn the_header_is_checked_before_any_work() {
    let c = compute();
    let file = create(
        &c,
        SMALL,
        CHEAP,
        &["only"],
        &[entry("Email", "a")],
        &[1; 32],
    )
    .unwrap();
    let mut big = file.clone();
    big[5..9].copy_from_slice(&(MAX_MEMORY_KIB + 1).to_le_bytes());
    assert_eq!(read_header(&big).err(), Some(Error::Cost));
    let mut lanes = file.clone();
    lanes[13] = 4;
    assert_eq!(read_header(&lanes).err(), Some(Error::Cost));
    let mut odd = file.clone();
    odd[17..21].copy_from_slice(&1000u32.to_le_bytes());
    assert_eq!(read_header(&odd).err(), Some(Error::SlotSize));
    assert_eq!(
        read_header(&file[..file.len() - 1]).err(),
        Some(Error::Length)
    );
}

/// Two equal passphrases are refused, and so are two that differ only in
/// their Unicode form; a no-break space and a space are two passphrases.
/// Contents too large for the slot are refused.
#[test]
fn create_refuses_what_it_cannot_keep() {
    let c = compute();
    let two = |a: &str, b: &str| {
        create(
            &c,
            SMALL,
            CHEAP,
            &[a, b],
            &[entry("a", "a"), entry("b", "b")],
            &[0; 32],
        )
    };
    assert_eq!(two("same", "same").err(), Some(Error::SamePassphrase));
    assert_eq!(
        two("\u{e9}t\u{e9}", "e\u{301}te\u{301}").err(),
        Some(Error::SamePassphrase)
    );
    let file = two("a b", "a\u{a0}b").expect("two passphrases");
    assert_eq!(title_of(&open(&c, &file, "a b").unwrap().1), "a");
    assert_eq!(title_of(&open(&c, &file, "a\u{a0}b").unwrap().1), "b");
    let note = Record::new(kind::NOTE).with(field::NOTE, &[b'x'; 60_000]);
    let full = Contents {
        records: vec![note.clone(), note],
    };
    assert_eq!(
        create(&c, SMALL, CHEAP, &["p"], &[full], &[0; 32]).err(),
        Some(Error::Full)
    );
}

/// A slot's records of every type survive encoding and decoding; an
/// unknown type and a missing required field are refused.
#[test]
fn records_of_every_type_read_back() {
    let c = compute();
    let words = faraday_bip::bip39::Mnemonic::parse(
        &c,
        faraday_bip::bip39::Language::English,
        "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about",
    )
    .unwrap();
    let sheet = crate::layouts::sheet_payload(b"wpkh(...)", b"Savings", b"").unwrap();
    let c = Contents {
        records: vec![
            Record::new(kind::SLOT_LABEL).with(field::LABEL, b"Main"),
            Record::new(kind::KEY)
                .with(field::KEY, &records::words_payload(&words))
                .with(field::KEY_LABEL, b"Test")
                .with(field::KEY_FLAGS, &[records::NO_LOAD_AT_UNLOCK]),
            Record::new(kind::WALLET)
                .with(field::WALLET, b"wpkh(@0/**)")
                .with(field::WALLET_NAME, b"Spending")
                .with(field::WALLET_FLAGS, &[records::NO_LOAD_AT_UNLOCK]),
            Record::new(kind::NOTE).with(field::NOTE, "Recovery codes · ünïcode".as_bytes()),
            Record::new(kind::SHEET).with(field::SHEET, &sheet),
            Record::new(kind::ENTRY)
                .with(field::TITLE, b"Email")
                .with(field::TOTP, b"otpauth://totp/x?secret=AB"),
            Record::new(kind::GPG)
                .with(1, &[1; 32])
                .with(2, &[0; 4])
                .with(5, b"A <a@b>")
                .with(5, b"B <b@c>"),
            Record::new(kind::SECURE_BOOT)
                .with(1, &[2; 16])
                .with(3, &[0x30, 0]),
            Record::new(kind::ROUND)
                .with(1, &[2])
                .with(2, &[3; 32])
                .with(3, &[4; 4])
                .with(4, &[5; 10]),
            Record::new(kind::AMOUNTS)
                .with(1, &[6; 32])
                .with(2, &[0; 8])
                .with(2, &[1; 8]),
        ],
    };
    let plain = c.encode(BODY, Seals::FIRST).unwrap();
    let (back, _) = Contents::decode(&plain).unwrap();
    assert_eq!(back.records.len(), c.records.len());
    assert_eq!(
        records::words_of(&back.records[1]).unwrap().word_count(),
        12
    );
    // A key and a wallet set not to load at unlock read back so; one
    // with no flags loads.
    assert!(!records::loads_at_unlock(&back.records[1]));
    assert!(!records::loads_at_unlock(&back.records[2]));
    let mut plain_wallet = back.records[2].clone();
    records::set_loads_at_unlock(&mut plain_wallet, true);
    assert!(plain_wallet.field(field::WALLET_FLAGS).is_none());
    assert!(records::loads_at_unlock(&plain_wallet));
    assert_eq!(
        back.records[3].text(field::NOTE),
        Some("Recovery codes · ünïcode")
    );
    assert_eq!(
        back.records[6]
            .fields
            .iter()
            .filter(|f| f.number == 5)
            .count(),
        2
    );

    let mut unknown = plain.clone();
    unknown[1] = 42;
    assert_eq!(
        Contents::decode(&unknown).err(),
        Some(Fault::UnknownType(42))
    );
    // Type 12, once an APK signing key, is retired: a slot holding one is
    // refused as an unknown type, whole.
    let mut retired = plain.clone();
    retired[1] = 12;
    assert_eq!(
        Contents::decode(&retired).err(),
        Some(Fault::UnknownType(12))
    );
    assert_eq!(
        Contents {
            records: vec![Record::new(12).with(1, &[1])],
        }
        .encode(BODY, Seals::FIRST)
        .err(),
        Some(Error::Contents(Fault::UnknownType(12)))
    );
    let no_title = Contents {
        records: vec![Record::new(kind::ENTRY).with(field::USERNAME, b"me")],
    };
    assert_eq!(
        no_title.encode(BODY, Seals::FIRST).err(),
        Some(Error::Contents(Fault::Missing(kind::ENTRY, 1)))
    );
}

/// Two wallet shares (type 13) in one slot seal and open with every
/// field; a share without its text refuses the slot
/// (`docs/BACKUP-FLOW.md` §15.6).
#[test]
fn wallet_shares_seal_and_open_and_one_without_its_text_is_refused() {
    let c = compute();
    let share = |n: u8, text: &str| {
        Record::new(kind::WALLET_SHARE)
            .with(field::SHARE, text.as_bytes())
            .with(field::SHARE_WALLET_NAME, b"Savings")
            .with(field::SHARE_NUMBER, &[n])
            .with(field::SHARE_OF, &[3])
            .with(field::SHARE_WALLET, b"wsh(sortedmulti(2,A,B,C))")
    };
    let contents = Contents {
        records: vec![
            share(1, "wsh(sortedmulti(2,A,B))"),
            share(2, "wsh(sortedmulti(2,B,C))"),
        ],
    };
    let file = create(&c, SMALL, CHEAP, &["shares"], &[contents], &[5; 32]).unwrap();
    let (_, back) = open(&c, &file, "shares").unwrap();
    assert_eq!(back.of(kind::WALLET_SHARE).count(), 2);
    for (k, (_, r)) in back.of(kind::WALLET_SHARE).enumerate() {
        assert_eq!(r.text(field::SHARE_WALLET_NAME), Some("Savings"));
        assert_eq!(r.field(field::SHARE_NUMBER), Some(&[k as u8 + 1][..]));
        assert_eq!(r.field(field::SHARE_OF), Some(&[3u8][..]));
        assert_eq!(
            r.text(field::SHARE_WALLET),
            Some("wsh(sortedmulti(2,A,B,C))")
        );
    }
    assert_eq!(
        back.records[1].text(field::SHARE),
        Some("wsh(sortedmulti(2,B,C))")
    );
    let no_text = Contents {
        records: vec![
            Record::new(kind::WALLET_SHARE)
                .with(field::SHARE_NUMBER, &[1])
                .with(field::SHARE_OF, &[3]),
        ],
    };
    assert_eq!(
        no_text.encode(BODY, Seals::FIRST).err(),
        Some(Error::Contents(Fault::Missing(kind::WALLET_SHARE, 1)))
    );
}

/// A passphrase kept on its own (type 14) seals and opens with the
/// fingerprint of its key and the passphrase; one without its passphrase,
/// or with a fingerprint of another length, refuses the slot
/// (`docs/BACKUP-FLOW.md` §17.12 item 4).
#[test]
fn a_passphrase_record_seals_and_opens_and_one_without_its_passphrase_is_refused() {
    let c = compute();
    let contents = Contents {
        records: vec![
            Record::new(kind::PASSPHRASE)
                .with(field::PASSPHRASE_KEY, &[0x9a, 0x6a, 0x25, 0x80])
                .with(field::PASSPHRASE, b"TREZOR"),
        ],
    };
    let file = create(&c, SMALL, CHEAP, &["apart"], &[contents], &[6; 32]).unwrap();
    let (_, back) = open(&c, &file, "apart").unwrap();
    let (_, r) = back.of(kind::PASSPHRASE).next().expect("the record");
    assert_eq!(
        r.field(field::PASSPHRASE_KEY),
        Some(&[0x9a, 0x6a, 0x25, 0x80][..])
    );
    assert_eq!(r.text(field::PASSPHRASE), Some("TREZOR"));
    let none = Contents {
        records: vec![Record::new(kind::PASSPHRASE).with(field::PASSPHRASE_KEY, &[1, 2, 3, 4])],
    };
    assert_eq!(
        none.encode(BODY, Seals::FIRST).err(),
        Some(Error::Contents(Fault::Missing(kind::PASSPHRASE, 2)))
    );
    let short = Contents {
        records: vec![
            Record::new(kind::PASSPHRASE)
                .with(field::PASSPHRASE_KEY, &[1, 2, 3])
                .with(field::PASSPHRASE, b"x"),
        ],
    };
    assert!(short.encode(BODY, Seals::FIRST).is_err());
}

#[test]
fn a_child_seed_record_seals_and_opens_and_one_that_does_not_fit_its_key_is_refused() {
    use crate::child::{self, ChildOf};
    let c = compute();
    let words = faraday_bip::bip39::Mnemonic::parse(
        &c,
        faraday_bip::bip39::Language::English,
        "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about",
    )
    .unwrap();
    let of = ChildOf {
        parent: [0x73, 0xc5, 0xda, 0x0a],
        language: 0,
        words: 12,
        index: 7,
    };
    let key = |bytes: &[u8]| Contents {
        records: vec![
            Record::new(kind::KEY)
                .with(field::KEY, &records::words_payload(&words))
                .with(field::KEY_CHILD, bytes),
        ],
    };
    let file = create(
        &c,
        SMALL,
        CHEAP,
        &["child"],
        &[key(&child::payload(&of).unwrap())],
        &[6; 32],
    )
    .unwrap();
    let (_, back) = open(&c, &file, "child").unwrap();
    let (_, r) = back.of(kind::KEY).next().expect("the key");
    assert_eq!(
        r.field(field::KEY_CHILD).and_then(child::child_of),
        Some(of)
    );
    // 24 words recorded for a key of 12, another application, an index
    // past BIP-85's, a seed in place of words: each refused.
    let mut other = child::payload(&ChildOf { words: 24, ..of }).unwrap();
    let refused = |b: &[u8]| {
        assert_eq!(
            key(b).encode(BODY, Seals::FIRST).err(),
            Some(Error::Contents(Fault::Value(kind::KEY, field::KEY_CHILD)))
        );
    };
    refused(&other);
    other = child::payload(&of).unwrap();
    other[4] = 32;
    refused(&other);
    other = child::payload(&of).unwrap();
    other[10..].copy_from_slice(&0x8000_0000u32.to_le_bytes());
    refused(&other);
    let seed = Contents {
        records: vec![
            Record::new(kind::KEY)
                .with(field::KEY, &crate::layouts::seed_payload(&[1; 32]).unwrap())
                .with(field::KEY_CHILD, &child::payload(&of).unwrap()),
        ],
    };
    assert!(seed.encode(BODY, Seals::FIRST).is_err());
}

/// Seals `plain`, a whole slot plaintext as given, into slot `i` of
/// `file` under `key`: a slot no writer of this format makes.
fn seal_raw(file: &mut [u8], key: &SlotKey, i: usize, plain: &[u8]) {
    let mut buf = plain.to_vec();
    let ad = associated(file, i);
    let cipher = XChaCha20Poly1305::new_from_slice(key.0.expose()).unwrap();
    let nonce = [5u8; NONCE_LEN];
    let tag = cipher
        .encrypt_inout_detached(&XNonce::from(nonce), &ad, InOutBuf::from(&mut buf[..]))
        .unwrap();
    let r = slot_range(SMALL, i);
    file[r.start..r.start + NONCE_LEN].copy_from_slice(&nonce);
    file[r.start + NONCE_LEN..r.end - TAG_LEN].copy_from_slice(&buf);
    file[r.end - TAG_LEN..r.end].copy_from_slice(&tag);
}

/// A seal count record of `count`, as §7 lays it out, with no time.
fn seal_record(count: u64) -> Vec<u8> {
    let mut r = vec![kind::SEALS, 11, 0, 0, 0, field::SEAL_COUNT, 8, 0];
    r.extend_from_slice(&count.to_le_bytes());
    r
}

/// A slot's plaintext after its commitment block: the format byte,
/// `records` as bytes, the end record and zeros.
fn body_of(records: &[u8]) -> Vec<u8> {
    let mut b = vec![records::FORMAT];
    b.extend_from_slice(records);
    b.resize(BODY, 0);
    b
}

/// "café" with its é as one code point (NFC) or as e and U+0301 (NFD):
/// a vault made with either opens with either, the same slot and the
/// same contents.
#[test]
fn a_passphrase_opens_in_either_unicode_form() {
    let c = compute();
    let nfc = "caf\u{e9}";
    let nfd = "cafe\u{301}";
    for (made, other) in [(nfc, nfd), (nfd, nfc)] {
        let file = create(
            &c,
            SMALL,
            CHEAP,
            &[made, "decoy"],
            &[entry("Caf\u{e9}", "x"), entry("Decoy", "y")],
            &[3; 32],
        )
        .unwrap();
        let (a, got_a) = open(&c, &file, made).unwrap();
        let (b, got_b) = open(&c, &file, other).unwrap();
        assert_eq!(a.position(), b.position());
        assert_eq!(title_of(&got_a), "Caf\u{e9}");
        assert_eq!(title_of(&got_b), "Caf\u{e9}");
    }
}

/// `tools/vectors/vault/two-keys.fvlt`: one slot whose tag verifies under
/// the slot keys of two passphrases (Len, Grubbs and Ristenpart's
/// construction, written by `tools/vectors/vault/two_keys.py`). Under
/// each, only the commitment block refuses it, and the file opens under
/// neither.
#[test]
fn a_slot_made_for_two_passphrases_opens_under_neither() {
    const FILE: &[u8] = include_bytes!("../../../tools/vectors/vault/two-keys.fvlt");
    let c = compute();
    let header = read_header(FILE).unwrap();
    for p in ["first of two keys", "second of two keys"] {
        let key = derive(&c, &header, p).unwrap();
        let verified = (0..SLOTS)
            .filter(|&i| matches!(decrypt_slot(FILE, &header, &key, i), Ok(Tried::Uncommitted)))
            .count();
        assert_eq!(verified, 1, "{p}: the slot's tag verifies");
        assert_eq!(open(&c, FILE, p).err(), Some(Error::Commitment), "{p}");
    }
}

/// A slot sealed honestly under its key but with the first byte of its
/// commitment block 1 is refused before its contents are read: the same
/// slot with the block zero is refused for its format byte instead.
#[test]
fn a_slot_whose_commitment_block_is_not_zero_is_refused_unread() {
    let c = compute();
    let mut file = create(
        &c,
        SMALL,
        CHEAP,
        &["only"],
        &[entry("Email", "a")],
        &[2; 32],
    )
    .unwrap();
    let (opened, _) = open(&c, &file, "only").unwrap();
    let key = derive(&c, opened.header(), "only").unwrap();
    let mut plain = vec![0u8; SMALL as usize];
    plain[0] = 1;
    plain[COMMIT_LEN] = 0xff;
    seal_raw(&mut file, &key, opened.position(), &plain);
    assert_eq!(open(&c, &file, "only").err(), Some(Error::Commitment));
    plain[0] = 0;
    seal_raw(&mut file, &key, opened.position(), &plain);
    assert_eq!(
        open(&c, &file, "only").err(),
        Some(Error::Contents(Fault::Format))
    );
}

/// A new vault's slots open with seal count 1 and no time. Each seal of
/// one opening counts one more, with the time it was given, and the
/// sealed file opens with it; the other slot keeps its own count.
#[test]
fn each_seal_of_one_opening_counts_one_more_and_keeps_its_time() {
    let c = compute();
    let file = create(
        &c,
        SMALL,
        CHEAP,
        &["first", "second"],
        &[entry("Email", "a"), entry("Bank", "b")],
        &[4; 32],
    )
    .unwrap();
    let (mut opened, contents) = open(&c, &file, "first").unwrap();
    assert_eq!(opened.seals(), Seals::FIRST);
    let at = Some(1_790_000_000);
    let once = opened.seal(&file, &contents, &[1; NONCE_LEN], at).unwrap();
    assert_eq!(opened.seals(), Seals { count: 2, last: at });
    let twice = opened
        .seal(&file, &contents, &[2; NONCE_LEN], None)
        .unwrap();
    assert_eq!(
        opened.seals(),
        Seals {
            count: 3,
            last: None
        }
    );
    assert_eq!(
        open(&c, &once, "first").unwrap().0.seals(),
        Seals { count: 2, last: at }
    );
    let (again, read) = open(&c, &twice, "first").unwrap();
    assert_eq!(
        again.seals(),
        Seals {
            count: 3,
            last: None
        }
    );
    assert_eq!(title_of(&read), "Email");
    assert_eq!(open(&c, &twice, "second").unwrap().0.seals(), Seals::FIRST);
}

/// A slot with no seal count, with two, or with a count of 0 is refused,
/// read alone or opened in a vault.
#[test]
fn a_slot_without_exactly_one_seal_count_is_refused() {
    let c = compute();
    assert_eq!(Contents::decode(&body_of(&[])).err(), Some(Fault::Seals));
    let two = [seal_record(1), seal_record(2)].concat();
    assert_eq!(Contents::decode(&body_of(&two)).err(), Some(Fault::Seals));
    assert_eq!(
        Contents::decode(&body_of(&seal_record(0))).err(),
        Some(Fault::Value(kind::SEALS, field::SEAL_COUNT))
    );
    let (back, seals) = Contents::decode(&body_of(&seal_record(7))).unwrap();
    assert!(back.records.is_empty());
    assert_eq!(
        seals,
        Seals {
            count: 7,
            last: None
        }
    );

    let mut file = create(
        &c,
        SMALL,
        CHEAP,
        &["only"],
        &[entry("Email", "a")],
        &[8; 32],
    )
    .unwrap();
    let (opened, _) = open(&c, &file, "only").unwrap();
    let key = derive(&c, opened.header(), "only").unwrap();
    let plain = [&[0u8; COMMIT_LEN][..], &body_of(&two)].concat();
    seal_raw(&mut file, &key, opened.position(), &plain);
    assert_eq!(
        open(&c, &file, "only").err(),
        Some(Error::Contents(Fault::Seals))
    );
}

/// The seal count is the vault's to write: contents holding one are
/// refused by `create` and by a seal, which then counts nothing.
#[test]
fn contents_holding_a_seal_count_are_not_written() {
    let c = compute();
    let forged = Contents {
        records: vec![Record::new(kind::SEALS).with(field::SEAL_COUNT, &9u64.to_le_bytes())],
    };
    assert_eq!(
        create(
            &c,
            SMALL,
            CHEAP,
            &["p"],
            core::slice::from_ref(&forged),
            &[0; 32]
        )
        .err(),
        Some(Error::Contents(Fault::Seals))
    );
    let file = create(&c, SMALL, CHEAP, &["p"], &[entry("Email", "a")], &[0; 32]).unwrap();
    let (mut opened, _) = open(&c, &file, "p").unwrap();
    assert_eq!(
        opened.seal(&file, &forged, &[0; NONCE_LEN], None).err(),
        Some(Error::Contents(Fault::Seals))
    );
    assert_eq!(opened.seals(), Seals::FIRST);
}

/// Records whose `used` is the slot's length fit, and seal with a time;
/// one byte more is `Full`, made or sealed.
#[test]
fn a_slot_filled_to_its_room_seals_and_one_byte_more_is_full() {
    let c = compute();
    let note = |n: usize| Contents {
        records: vec![Record::new(kind::NOTE).with(field::NOTE, &vec![b'x'; n])],
    };
    let room = SMALL as usize - note(0).used();
    let fits = note(room);
    assert_eq!(fits.used(), SMALL as usize);
    let file = create(
        &c,
        SMALL,
        CHEAP,
        &["p"],
        core::slice::from_ref(&fits),
        &[1; 32],
    )
    .unwrap();
    let (mut opened, got) = open(&c, &file, "p").unwrap();
    assert_eq!(
        got.records[0].field(field::NOTE).map(<[u8]>::len),
        Some(room)
    );
    let sealed = opened
        .seal(&file, &fits, &[1; NONCE_LEN], Some(u64::MAX))
        .unwrap();
    assert_eq!(
        open(&c, &sealed, "p").unwrap().0.seals().last,
        Some(u64::MAX)
    );
    let over = note(room + 1);
    assert_eq!(
        create(
            &c,
            SMALL,
            CHEAP,
            &["p"],
            core::slice::from_ref(&over),
            &[1; 32]
        )
        .err(),
        Some(Error::Full)
    );
    assert_eq!(
        opened.seal(&file, &over, &[2; NONCE_LEN], None).err(),
        Some(Error::Full)
    );
}
