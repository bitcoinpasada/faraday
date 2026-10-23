use alloc::vec;
use alloc::vec::Vec;

use super::records::{Contents, Fault, Record, field, kind};
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
        &[b"first", b"second", b"third"],
        &[entry("Email", "a"), entry("Bank", "b"), entry("Decoy", "c")],
        &[7; 32],
    )
    .unwrap();
    assert_eq!(file.len(), 262_357);
    for (p, t) in [("first", "Email"), ("second", "Bank"), ("third", "Decoy")] {
        let (_, got) = open(&c, &file, p.as_bytes()).unwrap();
        assert_eq!(title_of(&got), t);
    }
    assert_eq!(open(&c, &file, b"fourth").err(), Some(Error::Passphrase));
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
        &[b"first", b"second"],
        &[entry("Email", "a"), entry("Bank", "b")],
        &[9; 32],
    )
    .unwrap();
    let (opened, _) = open(&c, &file, b"first").unwrap();
    let sealed = opened
        .seal(&file, &entry("Email, changed", "a2"), &[3; NONCE_LEN])
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
        title_of(&open(&c, &sealed, b"first").unwrap().1),
        "Email, changed"
    );
    assert_eq!(title_of(&open(&c, &sealed, b"second").unwrap().1), "Bank");
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
        &[b"only"],
        &[entry("Email", "a")],
        &[1; 32],
    )
    .unwrap();
    file[30] ^= 1;
    assert_eq!(open(&c, &file, b"only").err(), Some(Error::Passphrase));
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
        &[b"only"],
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

/// Two equal passphrases are refused, as is contents too large for the
/// slot.
#[test]
fn create_refuses_what_it_cannot_keep() {
    let c = compute();
    assert_eq!(
        create(
            &c,
            SMALL,
            CHEAP,
            &[b"same", b"same"],
            &[entry("a", "a"), entry("b", "b")],
            &[0; 32]
        )
        .err(),
        Some(Error::SamePassphrase)
    );
    let note = Record::new(kind::NOTE).with(field::NOTE, &[b'x'; 60_000]);
    let full = Contents {
        records: vec![note.clone(), note],
    };
    assert_eq!(
        create(&c, SMALL, CHEAP, &[b"p"], &[full], &[0; 32]).err(),
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
            Record::new(kind::APK_KEY)
                .with(field::APK_KEY, &[7; 20])
                .with(field::APK_CERT, &[8; 10])
                .with(field::APK_PACKAGE, b"app.example.signer"),
        ],
    };
    let plain = c.encode(SMALL).unwrap();
    let back = Contents::decode(&plain).unwrap();
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
    assert_eq!(
        back.records[10].text(field::APK_PACKAGE),
        Some("app.example.signer")
    );
    let no_package = Contents {
        records: vec![
            Record::new(kind::APK_KEY)
                .with(field::APK_KEY, &[1])
                .with(field::APK_CERT, &[2]),
        ],
    };
    assert_eq!(
        no_package.encode(SMALL).err(),
        Some(Error::Contents(Fault::Missing(kind::APK_KEY, 3)))
    );
    let no_title = Contents {
        records: vec![Record::new(kind::ENTRY).with(field::USERNAME, b"me")],
    };
    assert_eq!(
        no_title.encode(SMALL).err(),
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
    let file = create(&c, SMALL, CHEAP, &[b"shares"], &[contents], &[5; 32]).unwrap();
    let (_, back) = open(&c, &file, b"shares").unwrap();
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
        no_text.encode(SMALL).err(),
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
    let file = create(&c, SMALL, CHEAP, &[b"apart"], &[contents], &[6; 32]).unwrap();
    let (_, back) = open(&c, &file, b"apart").unwrap();
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
        none.encode(SMALL).err(),
        Some(Error::Contents(Fault::Missing(kind::PASSPHRASE, 2)))
    );
    let short = Contents {
        records: vec![
            Record::new(kind::PASSPHRASE)
                .with(field::PASSPHRASE_KEY, &[1, 2, 3])
                .with(field::PASSPHRASE, b"x"),
        ],
    };
    assert!(short.encode(SMALL).is_err());
}
