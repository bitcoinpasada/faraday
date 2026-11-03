//! The text inside a vault (`docs/VAULT.md` §7, §7.1, §2.1): a vault
//! of every kind of item written and read back, a hand-edited document
//! read as the written one, what a document must not hold, and the QR
//! vault's top-level keys.

use faraday_bip::bip39::{Language, Mnemonic};
use faraday_crypto::work::Compute;
use faraday_vault::child::{ChildOf, payload as child_payload};
use faraday_vault::codex32::{SetFacts, State, set_payload};
use faraday_vault::contents::{
    Document, Form, QR_LEN, QR_NOTE_MAX, QR_PASSPHRASE_MAX, Refusal, Size, Why, form, read,
    read_qr, write, write_qr,
};
use faraday_vault::layouts::{seed_payload, sheet_payload, words_payload};
use faraday_vault::records::{field, kind};
use faraday_vault::{Contents, Record, Seals};

#[expect(
    clippy::disallowed_methods,
    reason = "a mint: faraday-vault's tests, which cannot reach faraday-core's testkit"
)]
fn compute() -> Compute {
    Compute::new(|_| {})
}

const TWELVE: &str =
    "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
const TWENTY_FOUR: &str = "zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo vote";
const XPUB: &str = "[73c5da0a/84h/0h/0h]xpub6CatWdiZiodmUeTDp8LT5or8nmbKNcuyvz7WyksVFkKB4RHwCD3XyuvPEbvqAQY3rAPshWcMLoP2fMFMKHPJ4ZeZXYVUhLv1VMrjPC7PW6V";
const SHARE: &str = "ms12namecacdefghjklmnpqrstuvwxyz023ftr2gdzmpy6pn";

fn words(phrase: &str) -> Vec<u8> {
    let c = compute();
    let m = Mnemonic::parse(&c, Language::English, phrase).unwrap();
    words_payload(&m).to_vec()
}

fn one_of_each() -> Document {
    let facts = SetFacts {
        threshold: 2,
        identifier: *b"name",
        length: 48,
        unknown: 1,
        strings: vec![(b'a', State::Made), (b'c', State::Placed)],
    };
    let child = ChildOf {
        parent: [0x73, 0xc5, 0xda, 0x0a],
        language: 0,
        words: 12,
        index: 7,
    };
    let policy = format!("wpkh(@0/**)\n{XPUB}");
    let records = vec![
        Record::new(kind::SLOT_LABEL).with(field::LABEL, b"Savings"),
        Record::new(kind::KEY)
            .with(field::KEY, &words(TWELVE))
            .with(field::KEY_LABEL, "Cold \"key\"\n\u{e9}".as_bytes())
            .with(field::KEY_FLAGS, &[3])
            .with(field::KEY_PASSPHRASE, b"tr\\ust")
            .with(field::KEY_CODEX32, &set_payload(&facts).unwrap())
            .with(field::KEY_CHILD, &child_payload(&child).unwrap()),
        Record::new(kind::KEY).with(field::KEY, &seed_payload(&[7u8; 16]).unwrap()),
        Record::new(kind::WALLET)
            .with(field::WALLET, policy.as_bytes())
            .with(field::WALLET_NAME, b"Spending")
            .with(field::WALLET_FLAGS, &[1]),
        Record::new(kind::WALLET).with(field::WALLET, format!("wpkh({XPUB}/<0;1>/*)").as_bytes()),
        Record::new(kind::NOTE).with(field::NOTE, b"line one\nline two"),
        Record::new(kind::SHEET).with(
            field::SHEET,
            &sheet_payload(b"wpkh(x)", b"Main", b"keep it").unwrap(),
        ),
        Record::new(kind::ENTRY)
            .with(field::TITLE, b"Mail")
            .with(field::USERNAME, b"me")
            .with(field::PASSWORD, b"correct horse")
            .with(field::URL, b"https://example.org")
            .with(field::NOTES, b"n")
            .with(
                field::TOTP,
                b"otpauth://totp/Mail:me?secret=JBSWY3DPEHPK3PXP&issuer=Mail",
            ),
        Record::new(kind::SECURE_BOOT)
            .with(1, &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16])
            .with(2, &[0x30, 0x82, 1, 2, 3])
            .with(3, &[0x30, 0x82, 9, 9, 9, 9])
            .with(6, &[0x30; 100]),
        Record::new(kind::AMOUNTS)
            .with(field::AMOUNTS_TX, &[0xab; 32])
            .with(field::AMOUNTS_INPUT, &5_000u64.to_le_bytes())
            .with(field::AMOUNTS_INPUT, &21_000_000u64.to_le_bytes()),
        Record::new(kind::PLAN)
            .with(
                field::PLAN_WALLET,
                format!("wpkh({XPUB}/<0;1>/*)").as_bytes(),
            )
            .with(field::PLAN_ANSWERS, b"seeds 1\nplaces 2")
            .with(field::PLAN_PLACE, b"Home")
            .with(field::PLAN_PLACE, b"")
            .with(field::PLAN_HOLDS, b"vault holds seed 73c5da0a")
            .with(field::PLAN_NOTE, b"lost place0 w0"),
        Record::new(kind::WALLET_SHARE)
            .with(field::SHARE, b"wsh(sortedmulti(2,A,B))")
            .with(field::SHARE_WALLET_NAME, b"Family")
            .with(field::SHARE_NUMBER, &[1])
            .with(field::SHARE_OF, &[3])
            .with(field::SHARE_WALLET, b"wsh(sortedmulti(2,A,B,C))"),
        Record::new(kind::PASSPHRASE)
            .with(field::PASSPHRASE_KEY, &[0x73, 0xc5, 0xda, 0x0a])
            .with(field::PASSPHRASE, "p\u{e4}ss".as_bytes()),
        Record::new(kind::CODEX32_SHARE)
            .with(field::CODEX32_STRING, SHARE.as_bytes())
            .with(field::CODEX32_KEY, &[0x73, 0xc5, 0xda, 0x0a]),
    ];
    Document {
        id: [
            0x3f, 0x0c, 0x9a, 0x6e, 0x2b, 0x7d, 0x41, 0xc5, 0x8e, 0x19, 0xa0, 0xb2, 0xc4, 0xd6,
            0xe8, 0xf0,
        ],
        seals: Seals {
            count: 12,
            last: Some(1_793_196_120),
        },
        contents: Contents { records },
    }
}

fn same(a: &Contents, b: &Contents) {
    assert_eq!(a.records.len(), b.records.len());
    for (x, y) in a.records.iter().zip(&b.records) {
        assert_eq!(x.kind, y.kind);
        let fx: Vec<(u8, &[u8])> = x.fields.iter().map(|f| (f.number, &f.bytes[..])).collect();
        let fy: Vec<(u8, &[u8])> = y.fields.iter().map(|f| (f.number, &f.bytes[..])).collect();
        assert_eq!(fx, fy, "record of type {}", x.kind);
    }
}

fn file(body: &str) -> Vec<u8> {
    format!("[vault]\nformat = 1\nid = \"3f0c9a6e2b7d41c58e19a0b2c4d6e8f0\"\nseals = 3\n\n{body}\n")
        .into_bytes()
}

fn refused(body: &str) -> Refusal {
    read(&compute(), &file(body)).expect_err(body)
}

#[test]
fn a_vault_of_every_kind_is_read_back_unchanged() {
    let doc = one_of_each();
    let text = write(&doc, Size::Kib64).unwrap();
    assert_eq!(text.len(), 64 * 1024);
    let back = read(&compute(), &text).unwrap();
    assert_eq!(back.id, doc.id);
    assert_eq!(back.seals, doc.seals);
    same(&back.contents, &doc.contents);
    // One canonical form.
    assert_eq!(write(&back, Size::Kib64).unwrap(), text);
    assert!(text.ends_with(b"\n\n\n"));
    assert_eq!(write(&doc, Size::Mib1).unwrap().len(), 1024 * 1024);
}

#[test]
fn the_text_shows_each_item_in_its_published_form() {
    let text = write(&one_of_each(), Size::Kib64).unwrap();
    let text = String::from_utf8(text.to_vec()).unwrap();
    assert!(text.starts_with("[vault]\nformat = 1\nid = \"3f0c9a6e2b7d41c58e19a0b2c4d6e8f0\"\nname = \"Savings\"\nseals = 12\nsealed = 2026-10-28T"));
    assert!(text.contains(&format!("words = \"{TWELVE}\"\n")));
    assert!(text.contains("hex = \"07070707070707070707070707070707\"\n"));
    assert!(text.contains(concat!("-----BEGIN ", "PRIVATE KEY-----\n")));
    assert!(text.contains("owner = \"04030201-0605-0807-090a-0b0c0d0e0f10\"\n"));
    let order: Vec<usize> = [
        "[[seed]]",
        "[[wallet]]",
        "[[note]]",
        "[[sheet]]",
        "[[entry]]",
        "[[secure_boot]]",
        "[[signed]]",
        "[[plan]]",
        "[[wallet_share]]",
        "[[passphrase]]",
        "[[share]]",
    ]
    .iter()
    .map(|h| text.find(h).unwrap())
    .collect();
    assert!(order.windows(2).all(|w| w[0] < w[1]));
}

#[test]
fn a_hand_edited_document_reads_as_the_written_one() {
    let doc = Document {
        id: one_of_each().id,
        seals: Seals {
            count: 3,
            last: None,
        },
        contents: Contents {
            records: vec![
                Record::new(kind::KEY)
                    .with(field::KEY, &words(TWELVE))
                    .with(field::KEY_LABEL, b"Cold key"),
                Record::new(kind::ENTRY)
                    .with(field::TITLE, b"Mail")
                    .with(field::PASSWORD, b"c:\\temp"),
                Record::new(kind::NOTE).with(field::NOTE, b"a\nb"),
            ],
        },
    };
    let written = read(&compute(), &write(&doc, Size::Kib64).unwrap()).unwrap();
    let edited = format!(
        "# edited by hand\nvault = {{ seals = 3, id = '3f0c9a6e2b7d41c58e19a0b2c4d6e8f0', format = 1 }} # count\n\n\
         seed = [ {{ label = 'Cold key', words = '{TWELVE}' }} ]\n\n\
         [[note]]\ntext = \"\"\"\na\nb\"\"\"\n\n\
         [[entry]]\npassword = 'c:\\temp'\ntitle = \"Mail\"\n\n\n\n"
    );
    let back = read(&compute(), edited.as_bytes()).unwrap();
    assert_eq!(back.id, written.id);
    assert_eq!(back.seals, written.seals);
    same(&back.contents, &written.contents);
}

#[test]
fn a_codex32_set_and_a_child_may_be_written_as_tables() {
    let body = format!(
        "[[seed]]\nwords = \"{TWELVE}\"\n[seed.codex32]\nk = 2\nid = \"name\"\nlengths = [48]\nunknown = 1\n\
         [[seed.codex32.shares]]\nindex = \"a\"\nstate = \"made\"\n\
         [seed.child]\nparent = \"73c5da0a\"\napplication = 39\nlanguage = 0\nwords = 12\nindex = 7\n"
    );
    let doc = read(&compute(), &file(&body)).unwrap();
    assert_eq!(doc.contents.records.len(), 1);
    assert!(doc.contents.records[0].field(field::KEY_CODEX32).is_some());
    assert!(doc.contents.records[0].field(field::KEY_CHILD).is_some());
}

#[test]
fn a_document_may_be_any_length_up_to_four_mib_and_no_more() {
    let doc = one_of_each();
    let mut text = write(&doc, Size::Kib64).unwrap().to_vec();
    text.resize(3_000_001, b'\n');
    assert!(read(&compute(), &text).is_ok());
    text.resize(4 * 1024 * 1024 + 1, b'\n');
    assert_eq!(read(&compute(), &text).unwrap_err().why, Why::TooLong);
    // Padding is blank lines, and white space and comments may follow; a
    // NUL after the document is refused.
    let mut text = write(&doc, Size::Kib64).unwrap().to_vec();
    *text.last_mut().unwrap() = 0;
    assert!(matches!(
        read(&compute(), &text).unwrap_err().why,
        Why::Syntax(_)
    ));
}

#[test]
fn what_section_7_1_refuses_is_refused_naming_what_and_where() {
    let words_bad = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon";
    let cases: Vec<(String, Why, &str, usize, &str)> = vec![
        ("[[nope]]\nx = 1".into(), Why::Unknown, "", 0, "nope"),
        ("[[note]]\ntext = \"a\"\n[[note]]\ntext = \"b\"\ncolour = \"x\"".into(), Why::Unknown, "note", 1, "colour"),
        ("[[note]]\ntext = 5".into(), Why::WrongType, "note", 0, "text"),
        ("[[note]]\ntext = \"a\"\n[[entry]]\ntitle = \"t\"\n[[entry]]\nusername = \"u\"".into(), Why::Missing, "entry", 1, "title"),
        (format!("[[seed]]\nwords = \"{words_bad}\""), Why::BadWords, "seed", 0, "words"),
        (format!("[[seed]]\nwords = \"{TWELVE}\"\nhex = \"00112233445566778899aabbccddeeff\""), Why::Exclusive, "seed", 0, "hex"),
        ("[[seed]]\nhex = \"00112233445566778899AABBCCDDEEFF\"".into(), Why::BadHex, "seed", 0, "hex"),
        ("[[wallet]]\npolicy = \"wpkh(@0/**)\"".into(), Why::BadWallet, "wallet", 0, "policy"),
        ("[[wallet]]\ndescriptor = \"nonsense\"".into(), Why::BadWallet, "wallet", 0, "descriptor"),
        ("[[secure_boot]]\nowner = \"04030201-0605-0807-090a-0b0c0d0e0f10\"\npk_key = \"-----BEGIN CERTIFICATE-----\\nAAAA\\n-----END CERTIFICATE-----\\n\"".into(), Why::BadPem, "secure_boot", 0, "pk_key"),
        ("[[entry]]\ntitle = \"t\"\notp = \"JBSWY3DPEHPK3PXP\"".into(), Why::BadOtp, "entry", 0, "otp"),
        ("[[entry]]\ntitle = \"t\"\notp = \"otpauth://hotp/x?secret=JBSWY3DPEHPK3PXP&counter=1\"".into(), Why::BadOtp, "entry", 0, "otp"),
        ("[[gpg]]\nkey = \"x\"".into(), Why::GpgNotRead, "gpg", 0, ""),
        ("[[passphrase]]\nfor = \"73c5da0a\"\ntext = \"a\"\n[[passphrase]]\nfor = \"73c5da0a\"\ntext = \"b\"".into(), Why::Twice, "passphrase", 1, ""),
        (format!("[[signed]]\ntx = \"{0}\"\namounts = [1]\n[[signed]]\ntx = \"{0}\"\namounts = [2]", "ab".repeat(32)), Why::Twice, "signed", 1, ""),
        (format!("[[share]]\ncodex32 = \"{SHARE}\"\nfor = \"73c5da0a\"\n[[share]]\ncodex32 = \"{SHARE}\"\nfor = \"73c5da0a\""), Why::Twice, "share", 1, ""),
        ("[[share]]\ncodex32 = \"ms10testsxxxxxxxxxxxxxxxxxxxxxxxxxx4nzvca9cmczlw\"\nfor = \"73c5da0a\"".into(), Why::BadCodex32, "share", 0, "codex32"),
        ("[[note]]\ntext = \"a\"\ntext = \"b\"".into(), Why::Syntax(0), "", 0, ""),
        ("[[note]]\ntext = \"\\x41\"".into(), Why::Syntax(0), "", 0, ""),
        ("[[note]]\n\"\\x74ext\" = \"a\"".into(), Why::Syntax(0), "", 0, ""),
        ("[[note]]\n\"te\\xt\" = \"a\"".into(), Why::Syntax(0), "", 0, ""),
        (format!("[[note]]\n{}", (0..65).map(|i| format!("k{i} = 1\n")).collect::<String>()), Why::Syntax(0), "", 0, ""),
        ("[[note]]\nx = {a = 1,\n b = 2}".into(), Why::Syntax(0), "", 0, ""),
        ("[[note]]\nx = {a = 1,}".into(), Why::Syntax(0), "", 0, ""),
        (format!("[[note]]\n{} = 1", ["a"; 9].join(".")), Why::Syntax(0), "", 0, ""),
        (format!("[[note]]\nx = {{{} = 1}}", ["a"; 9].join(".")), Why::Syntax(0), "", 0, ""),
        (format!("[{}]\nx = 1", ["a"; 9].join(".")), Why::Syntax(0), "", 0, ""),
        (format!("[[note]]\n{} = 1", ["a"; 100_000].join(".")), Why::Syntax(0), "", 0, ""),
        ("[[note]]\ntext = \"a\"\n[note]\ntext = \"b\"".into(), Why::Syntax(0), "", 0, ""),
    ];
    for (body, why, item, index, key) in cases {
        let mut r = refused(&body);
        if let Why::Syntax(_) = r.why {
            r.why = Why::Syntax(0);
        }
        assert_eq!(
            (r.why, r.item, r.index, r.key.as_str()),
            (why, item, index, key),
            "{body}"
        );
    }
}

#[test]
fn the_vault_table_is_first_and_complete() {
    let c = compute();
    let id = "id = \"3f0c9a6e2b7d41c58e19a0b2c4d6e8f0\"";
    let get = |t: &str| read(&c, t.as_bytes()).unwrap_err();
    assert_eq!(get("[[note]]\ntext = \"a\"\n").why, Why::NoVault);
    assert_eq!(
        get(&format!(
            "[[note]]\ntext = \"a\"\n[vault]\nformat = 1\n{id}\nseals = 1"
        ))
        .why,
        Why::VaultNotFirst
    );
    assert_eq!(get(&format!("[vault]\n{id}\nseals = 1")).why, Why::Missing);
    assert_eq!(
        get(&format!("[vault]\nformat = 2\n{id}\nseals = 1")).why,
        Why::Format
    );
    assert_eq!(get("[vault]\nformat = 1\nseals = 1").why, Why::Missing);
    assert_eq!(get(&format!("[vault]\nformat = 1\n{id}")).why, Why::Missing);
    assert_eq!(
        get(&format!("[vault]\nformat = 1\n{id}\nseals = 0")).why,
        Why::Seals
    );
    assert_eq!(
        get(&format!(
            "[vault]\nformat = 1\n{id}\nseals = 1\nsealed = 2026-10-28T14:02Z"
        ))
        .why,
        Why::BadValue
    );
    assert_eq!(
        get(&format!("[vault]\nformat = 1\n{id}\nseals = 1\nextra = 1")).key,
        "extra"
    );
    assert_eq!(
        get("[vault]\nformat = 1\nid = \"3F0C9A6E2B7D41C58E19A0B2C4D6E8F0\"\nseals = 1").why,
        Why::BadHex
    );
    // A time that is not ASCII, an offset out of range, a time past the
    // year 9999 (the offset moving it there), and a seal count past i64.
    for sealed in [
        "2026-10-28T14:02:00+0\u{e9}00",
        "2026-10-28T14:02:00+99:99",
        "2026-10-28T14:02:00+24:00",
        "2026-10-28T14:02:00+01:60",
        "9999-12-31T23:59:59-01:00",
    ] {
        assert_eq!(
            get(&format!(
                "[vault]\nformat = 1\n{id}\nseals = 1\nsealed = {sealed}"
            ))
            .why,
            Why::BadValue,
            "{sealed}"
        );
    }
    assert!(
        read(
            &c,
            format!("[vault]\nformat = 1\n{id}\nseals = 1\nsealed = 9999-12-31T23:59:59Z")
                .as_bytes()
        )
        .is_ok()
    );
    assert!(matches!(
        get(&format!(
            "[vault]\nformat = 1\n{id}\nseals = 9223372036854775808"
        ))
        .why,
        Why::Syntax(_)
    ));
}

#[test]
fn what_cannot_be_read_back_is_not_written() {
    let write_with = |seals: Seals| {
        let mut doc = one_of_each();
        doc.seals = seals;
        write(&doc, Size::Kib64)
    };
    let ok = Seals {
        count: 1,
        last: Some(253_402_300_799),
    };
    assert!(write_with(ok).is_ok());
    for bad in [
        Seals {
            count: 0,
            last: None,
        },
        Seals {
            count: i64::MAX as u64 + 1,
            last: None,
        },
        Seals {
            count: 1,
            last: Some(253_402_300_800),
        },
        Seals {
            count: 1,
            last: Some(u64::MAX),
        },
    ] {
        assert!(write_with(bad).is_err(), "{bad:?}");
    }
    let mut doc = one_of_each();
    doc.contents.records.push(
        Record::new(kind::SECURE_BOOT)
            .with(1, &[1; 16])
            .with(2, &[]),
    );
    assert_eq!(write(&doc, Size::Kib64).unwrap_err().why, Why::BadPem);
}

#[test]
fn gpg_keys_are_refused_both_ways() {
    let mut doc = one_of_each();
    doc.contents
        .records
        .push(Record::new(kind::GPG).with(1, &[1; 32]).with(2, &[1; 4]));
    assert_eq!(write(&doc, Size::Kib64).unwrap_err().why, Why::GpgNotRead);
}

#[test]
fn a_vault_too_full_for_its_size_is_refused() {
    let mut doc = one_of_each();
    doc.contents
        .records
        .push(Record::new(kind::NOTE).with(field::NOTE, &vec![b'x'; 60_000]));
    doc.contents
        .records
        .push(Record::new(kind::NOTE).with(field::NOTE, &vec![b'y'; 60_000]));
    assert_eq!(write(&doc, Size::Kib64).unwrap_err().why, Why::TooLong);
    assert!(write(&doc, Size::Kib256).is_ok());
}

fn qr_items() -> Vec<Record> {
    vec![
        Record::new(kind::KEY).with(field::KEY, &words(TWELVE)),
        Record::new(kind::KEY).with(field::KEY, &words(TWENTY_FOUR)),
        Record::new(kind::CODEX32_SHARE)
            .with(field::CODEX32_STRING, SHARE.as_bytes())
            .with(field::CODEX32_KEY, &[0x73, 0xc5, 0xda, 0x0a]),
        Record::new(kind::PASSPHRASE)
            .with(field::PASSPHRASE_KEY, &[0x73, 0xc5, 0xda, 0x0a])
            .with(field::PASSPHRASE, b"a short passphrase"),
    ]
}

#[test]
fn a_qr_vault_holds_one_item_in_one_length() {
    let c = compute();
    for item in qr_items() {
        let text = write_qr(&item).unwrap();
        assert_eq!(text.len(), QR_LEN);
        assert_eq!(form(&text).unwrap(), Form::Qr);
        let back = read_qr(&c, &text).unwrap();
        assert_eq!(back.kind, item.kind);
        let f = |r: &Record| {
            r.fields
                .iter()
                .map(|f| (f.number, f.bytes.to_vec()))
                .collect::<Vec<_>>()
        };
        assert_eq!(f(&back), f(&item));
        // A file vault is not one.
        assert_eq!(read(&c, &text).unwrap_err().why, Why::NoVault);
    }
    assert_eq!(
        form(&write(&one_of_each(), Size::Kib64).unwrap()).unwrap(),
        Form::File
    );
}

#[test]
fn a_twelve_word_seed_and_a_twenty_four_word_one_make_codes_of_one_size() {
    let a = write_qr(&qr_items()[0]).unwrap();
    let b = write_qr(&qr_items()[1]).unwrap();
    assert_eq!(a.len(), b.len());
}

#[test]
fn a_qr_vault_refuses_what_it_cannot_hold() {
    let c = compute();
    // A 15-word seed, a seed with a label, a long passphrase.
    let fifteen = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon address";
    let m = Mnemonic::parse(&c, Language::English, fifteen).unwrap();
    let r = Record::new(kind::KEY).with(field::KEY, &words_payload(&m));
    assert_eq!(write_qr(&r).unwrap_err().why, Why::NotQr);
    let labelled = qr_items()[0].clone().with(field::KEY_LABEL, b"x");
    assert_eq!(write_qr(&labelled).unwrap_err().why, Why::NotQr);
    let long = Record::new(kind::PASSPHRASE)
        .with(field::PASSPHRASE_KEY, &[0; 4])
        .with(field::PASSPHRASE, &[b'p'; 200]);
    assert_eq!(write_qr(&long).unwrap_err().why, Why::TooLong);
    let wallet = Record::new(kind::SECURE_BOOT).with(1, &[1; 16]);
    assert_eq!(write_qr(&wallet).unwrap_err().why, Why::NotQr);
    // Wrong length, two items.
    let mut text = write_qr(&qr_items()[0]).unwrap().to_vec();
    text.push(b'\n');
    assert_eq!(read_qr(&c, &text).unwrap_err().why, Why::NotQr);
    let two = format!("words = \"{TWELVE}\"\nnote = \"a\"");
    let mut two = two.into_bytes();
    two.resize(QR_LEN, b'\n');
    assert_eq!(read_qr(&c, &two).unwrap_err().why, Why::Unknown);
    // Keys the form does not name, beside the ones it does.
    for extra in ["list = \"english\"", "load = false", "held = true"] {
        let mut text = format!("words = \"{TWELVE}\"\n{extra}").into_bytes();
        text.resize(QR_LEN, b'\n');
        let r = read_qr(&c, &text).unwrap_err();
        assert_eq!(
            (r.why, r.key.as_str()),
            (Why::Unknown, extra.split(' ').next().unwrap())
        );
    }
    let mut text =
        format!("codex32 = \"{SHARE}\"\nfor = \"73c5da0a\"\nwords = \"{TWELVE}\"").into_bytes();
    text.resize(QR_LEN, b'\n');
    assert_eq!(read_qr(&c, &text).unwrap_err().why, Why::Unknown);
    let edge = |n: usize| {
        Record::new(kind::PASSPHRASE)
            .with(field::PASSPHRASE_KEY, &[0; 4])
            .with(field::PASSPHRASE, &vec![b'p'; n])
    };
    assert_eq!(write_qr(&edge(QR_PASSPHRASE_MAX)).unwrap().len(), QR_LEN);
    assert_eq!(
        write_qr(&edge(QR_PASSPHRASE_MAX + 1)).unwrap_err().why,
        Why::TooLong
    );
}

#[test]
fn a_qr_vault_may_hold_a_note_or_nothing() {
    let c = compute();
    let note = |t: &[u8]| Record::new(kind::NOTE).with(field::NOTE, t);
    let back = |r: &Record| {
        let text = write_qr(r).unwrap();
        assert_eq!(text.len(), QR_LEN);
        assert_eq!(form(&text).unwrap(), Form::Qr);
        read_qr(&c, &text).unwrap()
    };
    // The most a note holds, in bytes of TOML text.
    let edge = |n: usize| note("a".repeat(n).as_bytes());
    assert_eq!(
        back(&edge(QR_NOTE_MAX)).field(field::NOTE).unwrap().len(),
        QR_NOTE_MAX
    );
    assert_eq!(
        write_qr(&edge(QR_NOTE_MAX + 1)).unwrap_err().why,
        Why::TooLong
    );
    // Quotes and backslashes are two bytes each.
    let esc = note("\"".repeat(QR_NOTE_MAX / 2).as_bytes());
    assert_eq!(
        back(&esc).field(field::NOTE).unwrap().len(),
        QR_NOTE_MAX / 2
    );
    let over = note("\\".repeat(QR_NOTE_MAX / 2 + 1).as_bytes());
    assert_eq!(write_qr(&over).unwrap_err().why, Why::TooLong);
    // Line feeds and other text.
    let text = "line one\nline two\ttab \u{e9}";
    assert_eq!(
        back(&note(text.as_bytes())).field(field::NOTE).unwrap(),
        text.as_bytes()
    );
    // A shorter way of writing the same line holds no more than the limit
    // once written: it is refused on read, naming the key.
    let padded = |t: String| {
        let mut b = t.into_bytes();
        b.resize(QR_LEN, b'\n');
        b
    };
    let long = "a".repeat(QR_NOTE_MAX + 1);
    for line in [format!("note=\"{long}\"\n"), format!("note='{long}'\n")] {
        let r = read_qr(&c, &padded(line)).unwrap_err();
        assert_eq!((r.why, r.key.as_str()), (Why::TooLong, "note"));
    }
    let long = "a".repeat(QR_PASSPHRASE_MAX + 1);
    for line in [
        format!("passphrase=\"{long}\"\nfor=\"73c5da0a\"\n"),
        format!("passphrase='{long}'\nfor=\"73c5da0a\"\n"),
    ] {
        let r = read_qr(&c, &padded(line)).unwrap_err();
        assert_eq!((r.why, r.key.as_str()), (Why::TooLong, "passphrase"));
    }
    // Nothing: the padding alone.
    let empty = write_qr(&note(b"")).unwrap();
    assert!(empty.iter().all(|&b| b == b'\n'));
    assert_eq!(back(&note(b"")).field(field::NOTE).unwrap(), b"");
    // Two forms mixed.
    let mut mixed = format!("words = \"{TWELVE}\"\nnote = \"a\"").into_bytes();
    mixed.resize(QR_LEN, b'\n');
    assert_eq!(read_qr(&c, &mixed).unwrap_err().why, Why::Unknown);
    let mut mixed = String::from("note = \"a\"\nfor = \"73c5da0a\"").into_bytes();
    mixed.resize(QR_LEN, b'\n');
    assert_eq!(read_qr(&c, &mixed).unwrap_err().why, Why::Unknown);
}
