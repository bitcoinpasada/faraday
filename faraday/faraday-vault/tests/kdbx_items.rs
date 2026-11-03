//! Faraday's items in a KDBX document (`faraday_vault::kdbx::items`,
//! `docs/VAULT.md` §7): a vault holding one item of each kind is written
//! and read back to the same records; each damaged item opens Unreadable,
//! naming its field and fault, with the others read and its entry written
//! back as it was; pykeepass's file opens with its entry; a person's
//! entry keeps the fields Faraday does not read; an item moved to another
//! group keeps its kind.

use std::path::{Path, PathBuf};

use faraday_crypto::work::{Compute, Work};
use faraday_vault::child::ChildOf;
use faraday_vault::codex32::{SetFacts, State, set_payload};
use faraday_vault::kdbx::items::{
    self, Body, Damage, Extra, Fault, Format, GpgKey, Place, Read, Unwritable,
};
use faraday_vault::kdbx::open;
use faraday_vault::kdbx::xml::{Document, Entry, Group};
use faraday_vault::records::{BACKUP_PENDING, NO_LOAD_AT_UNLOCK, field, kind};
use faraday_vault::totp::NotTotp;
use faraday_vault::{Record, layouts};
use zeroize::Zeroizing;

const KEY: &[u8] = &[9; 64];
const XPUB: &str = "[73c5da0a/84h/0h/0h]xpub6CatWdiZiodmUeTDp8LT5or8nmbKNcuyvz7WyksVFkKB4RHwCD3XyuvPEbvqAQY3rAPshWcMLoP2fMFMKHPJ4ZeZXYVUhLv1VMrjPC7PW6V";
const WORDS: &str =
    "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
const SHARE_A: &str = "ms12namea320zyxwvutsrqpnmlkjhgfedcaxrpp870hkkqrm";
const SHARE_C: &str = "ms12namecacdefghjklmnpqrstuvwxyz023ftr2gdzmpy6pn";
const FP: [u8; 4] = [0x73, 0xc5, 0xda, 0x0a];
const DER: &[u8] = &[0x30, 0x03, 0x02, 0x01, 0x01];
/// An armor's kind, built so that no whole armor line of a secret key is
/// written in the source.
const SECRET: &str = "PRIVATE";

fn gpg_key() -> String {
    format!(
        "-----BEGIN PGP {SECRET} KEY BLOCK-----\n\nxVgEZ0\n-----END PGP {SECRET} KEY BLOCK-----\n"
    )
}
const GPG_PUBLIC: &str =
    "-----BEGIN PGP PUBLIC KEY BLOCK-----\n\nxjME\n-----END PGP PUBLIC KEY BLOCK-----\n";

#[expect(
    clippy::disallowed_methods,
    reason = "a mint: faraday-vault's tests, which cannot reach faraday-core's testkit"
)]
fn compute() -> Compute {
    Compute::new(|_: Work| {})
}

fn repo(path: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(path)
}

fn uuid(n: u8) -> [u8; 16] {
    [n; 16]
}

fn fields(r: &Record) -> Vec<(u8, Vec<u8>)> {
    let mut v: Vec<(u8, Vec<u8>)> = r
        .fields
        .iter()
        .map(|f| (f.number, f.bytes.to_vec()))
        .collect();
    v.sort();
    v
}

fn words_payload() -> Zeroizing<Vec<u8>> {
    let m = faraday_bip::bip39::Mnemonic::parse(
        &compute(),
        faraday_bip::bip39::Language::English,
        WORDS,
    )
    .unwrap();
    layouts::words_payload(&m)
}

fn seed_record() -> Record {
    let facts = SetFacts {
        threshold: 2,
        identifier: *b"name",
        length: 48,
        unknown: 1,
        strings: vec![(b'a', State::Placed), (b'c', State::Made)],
    };
    let child = ChildOf {
        parent: [1, 2, 3, 4],
        language: 0,
        words: 12,
        index: 7,
    };
    Record::new(kind::KEY)
        .with(field::KEY, &words_payload())
        .with(field::KEY_LABEL, b"Savings")
        .with(field::KEY_FLAGS, &[NO_LOAD_AT_UNLOCK | BACKUP_PENDING])
        .with(field::KEY_PASSPHRASE, b"extra words")
        .with(field::KEY_CODEX32, &set_payload(&facts).unwrap())
        .with(
            field::KEY_CHILD,
            &faraday_vault::child::payload(&child).unwrap(),
        )
}

fn records() -> Vec<(Record, &'static str, &'static str)> {
    let policy = format!("wpkh(@0/**)\n{XPUB}");
    let descriptor = format!("wpkh({XPUB}/<0;1>/*)");
    let seed_hex =
        Record::new(kind::KEY).with(field::KEY, &layouts::seed_payload(&[0x5a; 32]).unwrap());
    let sheet = layouts::sheet_payload(descriptor.as_bytes(), b"Spending", b"in the safe").unwrap();
    let mut sb = Record::new(kind::SECURE_BOOT).with(1, &[0x11; 16]);
    for n in 2..=7 {
        sb.push(n, DER);
    }
    vec![
        (seed_record(), "Bitcoin keys", "Key 73c5da0a"),
        (seed_hex, "Bitcoin keys", "Key 01020304"),
        (
            Record::new(kind::WALLET)
                .with(field::WALLET, policy.as_bytes())
                .with(field::WALLET_NAME, b"Spending")
                .with(field::WALLET_FLAGS, &[NO_LOAD_AT_UNLOCK]),
            "Wallets",
            "",
        ),
        (
            Record::new(kind::WALLET).with(field::WALLET, descriptor.as_bytes()),
            "Wallets",
            "",
        ),
        (
            Record::new(kind::NOTE).with(field::NOTE, b"the safe's code is in the drawer"),
            "Notes",
            "Note",
        ),
        (
            Record::new(kind::SHEET).with(field::SHEET, &sheet),
            "Recovery sheets",
            "",
        ),
        (sb, "Secure Boot", "Secure Boot keys"),
        (
            Record::new(kind::WALLET_SHARE)
                .with(field::SHARE, b"share text")
                .with(field::SHARE_WALLET_NAME, b"Family")
                .with(field::SHARE_NUMBER, &[1])
                .with(field::SHARE_OF, &[3])
                .with(field::SHARE_WALLET, descriptor.as_bytes()),
            "Wallet shares",
            "",
        ),
        (
            Record::new(kind::PASSPHRASE)
                .with(field::PASSPHRASE_KEY, &FP)
                .with(field::PASSPHRASE, b"correct horse"),
            "Passphrases",
            "Passphrase for 73c5da0a",
        ),
        (
            Record::new(kind::CODEX32_SHARE)
                .with(field::CODEX32_STRING, SHARE_A.as_bytes())
                .with(field::CODEX32_KEY, &FP),
            "Codex32 shares",
            "Codex32 share for 73c5da0a",
        ),
        (
            Record::new(kind::ENTRY)
                .with(field::TITLE, b"Mail")
                .with(field::USERNAME, b"alice")
                .with(field::PASSWORD, b"hunter2")
                .with(field::URL, b"https://mail.example/")
                .with(field::NOTES, b"two lines\nof notes")
                .with(
                    field::TOTP,
                    b"otpauth://totp/Example:alice?secret=JBSWY3DPEHPK3PXP&issuer=Example",
                ),
            "",
            "",
        ),
    ]
}

fn gpg() -> GpgKey {
    GpgKey {
        title: "Alice <alice@example.org>".into(),
        key: Zeroizing::new(gpg_key()),
        public: Some(GPG_PUBLIC.into()),
        revocation: Some(Zeroizing::new(GPG_PUBLIC.into())),
        revoked: Some("2026-10-28T14:02:00Z".into()),
    }
}

fn place<'a>(n: u8, group: &'a str, title: &'a str) -> Place<'a> {
    Place {
        uuid: uuid(n),
        group,
        group_uuid: [n.wrapping_add(100); 16],
        title,
        now: Some(1_800_000_000),
    }
}

/// A Faraday vault holding one item of each kind (and a plan and signed
/// amounts), the UUIDs from 1.
fn full() -> Document {
    let c = compute();
    let mut doc = Document::new([0xee; 16], "Vault");
    items::stamp(&mut doc, 3, Some("2026-10-28T14:02:00Z")).unwrap();
    for (i, (r, group, title)) in records().into_iter().enumerate() {
        items::put(
            &c,
            &mut doc,
            &place(i as u8 + 1, group, title),
            &Body::Record(r),
        )
        .unwrap();
    }
    items::put(&c, &mut doc, &place(50, "GPG keys", ""), &Body::Gpg(gpg())).unwrap();
    items::put_plan(&mut doc, 1, &plan()).unwrap();
    items::put_signed(&mut doc, &signed()).unwrap();
    doc
}

fn plan() -> Record {
    Record::new(kind::PLAN)
        .with(
            field::PLAN_WALLET,
            format!("wpkh({XPUB}/<0;1>/*)").as_bytes(),
        )
        .with(field::PLAN_ANSWERS, b"seeds 1\nplaces 2")
        .with(field::PLAN_PLACE, b"Home")
}

fn signed() -> Record {
    Record::new(kind::AMOUNTS)
        .with(field::AMOUNTS_TX, &[0xab; 32])
        .with(field::AMOUNTS_INPUT, &5000u64.to_le_bytes())
        .with(field::AMOUNTS_INPUT, &0u64.to_le_bytes())
}

fn again(doc: &Document) -> Document {
    Document::read(&doc.write(KEY), KEY).expect("reads")
}

fn read(doc: &Document) -> Read {
    items::read(&compute(), &again(doc))
}

fn record_of(read: &Read, n: u8) -> &Record {
    match &read
        .items
        .iter()
        .find(|i| i.uuid == uuid(n))
        .expect("the item")
        .body
    {
        Body::Record(r) => r,
        Body::Gpg(_) => panic!("a GPG key"),
    }
}

#[test]
fn a_vault_of_every_kind_reads_back_to_the_same_records() {
    let read = read(&full());
    assert_eq!(read.format, Format::Faraday);
    assert_eq!((read.seals, read.next_seal()), (3, 4));
    assert_eq!(read.sealed.as_deref(), Some("2026-10-28T14:02:00Z"));
    assert_eq!(read.label, "Vault");
    assert!(read.unreadable.is_empty(), "{:?}", read.unreadable);
    assert!(!read.read_only());
    for (i, (r, group, _)) in records().into_iter().enumerate() {
        let n = i as u8 + 1;
        assert_eq!(fields(record_of(&read, n)), fields(&r), "item {n}");
        let item = read.items.iter().find(|i| i.uuid == uuid(n)).unwrap();
        let path: Vec<&str> = item.path.iter().map(String::as_str).collect();
        let want: Vec<&str> = if group.is_empty() {
            vec![]
        } else {
            vec![group]
        };
        assert_eq!(path, want, "item {n}");
    }
    let g = read.items.iter().find(|i| i.uuid == uuid(50)).unwrap();
    let Body::Gpg(k) = &g.body else { panic!() };
    let want = gpg();
    assert_eq!(
        (
            &k.title,
            &k.key[..],
            &k.public,
            k.revocation.as_deref().map(String::as_str),
            &k.revoked
        ),
        (
            &want.title,
            &want.key[..],
            &want.public,
            want.revocation.as_deref().map(String::as_str),
            &want.revoked
        )
    );
    let keys: Vec<String> = read.custom.iter().map(|c| c.key.clone()).collect();
    assert_eq!(
        keys,
        [
            "faraday.plan.1".to_string(),
            format!("faraday.signed.{}", "ab".repeat(32))
        ]
    );
    assert_eq!(fields(&read.custom[0].record), fields(&plan()));
    assert_eq!(fields(&read.custom[1].record), fields(&signed()));
}

#[test]
fn an_item_is_written_in_its_published_form() {
    let doc = again(&full());
    let seed = doc
        .root()
        .groups()
        .find(|g| g.name() == "Bitcoin keys")
        .unwrap()
        .entries()
        .next()
        .unwrap();
    assert_eq!(seed.field("Title").unwrap().value(), "Savings");
    let words = seed.field("Password").unwrap();
    assert_eq!((words.value(), words.protected()), (WORDS, true));
    assert_eq!(seed.custom().get("faraday.kind"), Some("seed"));
    assert_eq!(seed.custom().get("faraday.load"), Some("false"));
    assert_eq!(seed.custom().get("faraday.held"), Some("true"));
    assert_eq!(
        seed.custom().get("faraday.child"),
        Some("{parent = \"01020304\", application = 39, language = 0, words = 12, index = 7}")
    );
    let sb = doc
        .root()
        .groups()
        .find(|g| g.name() == "Secure Boot")
        .unwrap()
        .entries()
        .next()
        .unwrap();
    assert_eq!(
        sb.field("owner").unwrap().value(),
        "11111111-1111-1111-1111-111111111111"
    );
    assert!(
        sb.field("pk_key")
            .unwrap()
            .value()
            .starts_with(&format!("-----BEGIN {SECRET} KEY-----\n"))
    );
    assert!(sb.field("pk_key").unwrap().protected());
    assert!(
        sb.field("pk_cert")
            .unwrap()
            .value()
            .starts_with("-----BEGIN CERTIFICATE-----\n")
    );
    let wallets = doc.root().groups().find(|g| g.name() == "Wallets").unwrap();
    let names: Vec<bool> = wallets
        .entries()
        .map(|e| e.field("policy").is_some())
        .collect();
    assert_eq!(names, [true, false]);
}

// --- Damage ---------------------------------------------------------------

fn bad(n: u8, kind_name: Option<&str>, f: &[(&str, &str, bool)], cd: &[(&str, &str)]) -> Entry {
    let mut e = Entry::new(uuid(n));
    e.set_field("Title", &format!("bad {n}"), false).unwrap();
    for (k, v, p) in f {
        e.set_field(k, v, *p).unwrap();
    }
    if let Some(k) = kind_name {
        e.custom_mut().set("faraday.kind", k).unwrap();
    }
    for (k, v) in cd {
        e.custom_mut().set(k, v).unwrap();
    }
    e
}

fn damaged() -> Vec<(Entry, &'static str, Fault)> {
    let policy = "wpkh(@0/**)\nnot a key";
    vec![
        (
            bad(60, Some("rocket"), &[], &[]),
            "faraday.kind",
            Fault::UnknownKind,
        ),
        (
            bad(
                61,
                Some("note"),
                &[("Notes", "n", false), ("Password", "x", true)],
                &[],
            ),
            "Password",
            Fault::UnknownField,
        ),
        (
            bad(
                62,
                Some("note"),
                &[("Notes", "n", false)],
                &[("faraday.load", "false")],
            ),
            "faraday.load",
            Fault::UnknownField,
        ),
        (
            bad(63, Some("passphrase"), &[("Password", "p", true)], &[]),
            "for",
            Fault::Missing,
        ),
        (
            bad(
                64,
                Some("seed"),
                &[("Password", "abandon abandon", true)],
                &[],
            ),
            "Password",
            Fault::Value,
        ),
        (
            bad(
                65,
                Some("seed"),
                &[("Password", WORDS, true), ("hex", "00", true)],
                &[],
            ),
            "hex",
            Fault::Twice,
        ),
        (
            bad(
                66,
                Some("seed"),
                &[("Password", WORDS, true)],
                &[("faraday.codex32", "{k = 1}")],
            ),
            "faraday.codex32",
            Fault::Value,
        ),
        (
            bad(
                67,
                Some("seed"),
                &[("Password", WORDS, true)],
                &[("faraday.child", "nonsense")],
            ),
            "faraday.child",
            Fault::Value,
        ),
        (
            bad(68, Some("wallet"), &[("policy", policy, false)], &[]),
            "policy",
            Fault::Value,
        ),
        (
            bad(
                69,
                Some("secure_boot"),
                &[
                    ("owner", "11111111-1111-1111-1111-111111111111", false),
                    (
                        "pk_key",
                        "-----BEGIN CERTIFICATE-----\nMAMCAQE=\n-----END CERTIFICATE-----\n",
                        true,
                    ),
                ],
                &[],
            ),
            "pk_key",
            Fault::Value,
        ),
        (
            bad(
                71,
                Some("share"),
                &[
                    ("Password", "ms12nameaxxxx", true),
                    ("for", "73c5da0a", false),
                ],
                &[],
            ),
            "Password",
            Fault::Value,
        ),
        (
            bad(72, Some("gpg"), &[("key", GPG_PUBLIC, true)], &[]),
            "key",
            Fault::Value,
        ),
        (
            bad(
                73,
                Some("passphrase"),
                &[("Password", "one", true), ("for", "0badf00d", false)],
                &[],
            ),
            "for",
            Fault::Twice,
        ),
        (
            bad(
                74,
                Some("passphrase"),
                &[("Password", "two", true), ("for", "0badf00d", false)],
                &[],
            ),
            "for",
            Fault::Twice,
        ),
        (
            bad(
                75,
                Some("share"),
                &[("Password", SHARE_C, true), ("for", "0badf00d", false)],
                &[],
            ),
            "for",
            Fault::Twice,
        ),
        (
            bad(
                76,
                Some("share"),
                &[("Password", SHARE_C, true), ("for", "0badf00d", false)],
                &[],
            ),
            "for",
            Fault::Twice,
        ),
        (
            bad(
                77,
                Some("wallet_share"),
                &[("Password", "s", true), ("index", "01", false)],
                &[],
            ),
            "index",
            Fault::Value,
        ),
    ]
}

/// A vault whose damaged entries come first in a group of their own, then
/// the items of `full`.
fn with_damage() -> (Document, Vec<(u8, &'static str, Fault)>) {
    let mut doc = Document::new([0xee; 16], "Vault");
    items::stamp(&mut doc, 1, None).unwrap();
    let mut g = Group::new([0xdd; 16], "Damaged");
    let mut want = Vec::new();
    for (e, at, fault) in damaged() {
        want.push((e.uuid()[0], at, fault));
        g.push_entry(e).unwrap();
    }
    doc.root_mut().push_group(g).unwrap();
    let c = compute();
    for (i, (r, group, title)) in records().into_iter().enumerate() {
        items::put(
            &c,
            &mut doc,
            &place(i as u8 + 1, group, title),
            &Body::Record(r),
        )
        .unwrap();
    }
    (doc, want)
}

/// The written bytes of the entry with this UUID: from its `<Entry>` to
/// its `</Entry>`.
fn entry_bytes(written: &[u8], n: u8) -> Vec<u8> {
    let tag = format!("<UUID>{}</UUID>", base64(&uuid(n)));
    let at = written
        .windows(tag.len())
        .position(|w| w == tag.as_bytes())
        .expect("the entry");
    let start = written[..at]
        .windows(7)
        .rposition(|w| w == b"<Entry>")
        .unwrap();
    let end = at
        + written[at..]
            .windows(8)
            .position(|w| w == b"</Entry>")
            .unwrap()
        + 8;
    written[start..end].to_vec()
}

fn base64(b: &[u8]) -> String {
    const A: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut s = String::new();
    for ch in b.chunks(3) {
        let n = (u32::from(ch[0]) << 16)
            | (u32::from(*ch.get(1).unwrap_or(&0)) << 8)
            | u32::from(*ch.get(2).unwrap_or(&0));
        for i in 0..4 {
            if i <= ch.len() {
                s.push(A[(n >> (18 - 6 * i) & 63) as usize] as char);
            } else {
                s.push('=');
            }
        }
    }
    s
}

#[test]
fn a_damaged_item_is_unreadable_and_the_others_read() {
    let (doc, want) = with_damage();
    let read = read(&doc);
    assert_eq!(read.items.len(), records().len());
    let mut got: Vec<(u8, String, Fault)> = read
        .unreadable
        .iter()
        .map(|u| (u.uuid.unwrap()[0], u.what.at.clone(), u.what.fault))
        .collect();
    got.sort_by(|a, b| (a.0, &a.1).cmp(&(b.0, &b.1)));
    let mut want: Vec<(u8, String, Fault)> =
        want.into_iter().map(|(n, a, f)| (n, a.into(), f)).collect();
    want.sort_by(|a, b| (a.0, &a.1).cmp(&(b.0, &b.1)));
    assert_eq!(got, want);
    let u = read
        .unreadable
        .iter()
        .find(|u| u.uuid == Some(uuid(60)))
        .unwrap();
    assert_eq!(u.title, "bad 60");
}

#[test]
fn a_damaged_entry_is_written_back_as_it_was() {
    let (doc, want) = with_damage();
    let doc = again(&doc);
    let before = doc.write(KEY);
    let mut changed = again(&doc);
    let mut r = record_of(&items::read(&compute(), &changed), 11).clone();
    r.set(field::PASSWORD, b"a new password");
    items::put(
        &compute(),
        &mut changed,
        &place(11, "", ""),
        &Body::Record(r),
    )
    .unwrap();
    let after = changed.write(KEY);
    assert_ne!(&before[..], &after[..]);
    for (n, _, _) in want {
        assert_eq!(entry_bytes(&before, n), entry_bytes(&after, n), "entry {n}");
    }
}

#[test]
fn entries_sharing_a_uuid_are_unreadable() {
    let mut doc = full();
    doc.root_mut().push_entry(bad(80, None, &[], &[])).unwrap();
    doc.root_mut().push_entry(bad(80, None, &[], &[])).unwrap();
    let read = read(&doc);
    let same: Vec<&Damage> = read.unreadable.iter().map(|u| &u.what).collect();
    assert_eq!(same.len(), 2);
    assert!(same.iter().all(|d| d.fault == Fault::SameUuid));
    let mut doc = again(&doc);
    let r = Record::new(kind::NOTE).with(field::NOTE, b"x");
    assert_eq!(
        items::put(
            &compute(),
            &mut doc,
            &place(80, "Notes", "Note"),
            &Body::Record(r)
        ),
        Err(Unwritable::Unreadable)
    );
}

// --- The vault's own values ------------------------------------------------

#[test]
fn a_newer_format_is_read_only_and_its_items_unreadable() {
    let mut doc = full();
    doc.custom_mut().set("faraday.format", "2").unwrap();
    let read = read(&doc);
    assert_eq!(read.format, Format::Newer("2".into()));
    assert!(read.read_only());
    assert_eq!(read.items.len(), 1, "the person's entry reads");
    assert!(
        read.unreadable
            .iter()
            .all(|u| u.what.fault == Fault::Format)
    );
    let mut doc = again(&doc);
    assert_eq!(items::stamp(&mut doc, 4, None), Err(Unwritable::ReadOnly));
}

#[test]
fn custom_data_that_does_not_read_is_named_and_kept() {
    let mut doc = full();
    let c = doc.custom_mut();
    c.set("faraday.seals", "three").unwrap();
    c.set("faraday.plan.2", "wallet = 3").unwrap();
    c.set("faraday.signed.00", "1").unwrap();
    c.set("faraday.colour", "blue").unwrap();
    let read = read(&doc);
    assert_eq!((read.seals, read.next_seal()), (0, 1));
    let mut named: Vec<(&str, Fault)> = read
        .unreadable
        .iter()
        .map(|u| (u.what.at.as_str(), u.what.fault))
        .collect();
    named.sort_by_key(|n| n.0);
    assert_eq!(
        named,
        [
            ("faraday.colour", Fault::UnknownField),
            ("faraday.plan.2", Fault::Value),
            ("faraday.seals", Fault::Value),
            ("faraday.signed.00", Fault::Value),
        ]
    );
    assert!(read.unreadable.iter().all(|u| u.uuid.is_none()));
    let written = again(&doc);
    assert_eq!(written.custom().get("faraday.plan.2"), Some("wallet = 3"));
    assert_eq!(written.custom().get("faraday.colour"), Some("blue"));
}

// --- KeePass apps ------------------------------------------------------------

#[test]
fn a_file_pykeepass_wrote_opens_with_its_entry() {
    let bytes = std::fs::read(repo(
        "tools/vectors/kdbx/pykeepass-chacha20-argon2id-plain.kdbx",
    ))
    .unwrap();
    let o = open(&compute(), &bytes, "kdbx vector passphrase").expect("opens");
    let doc = Document::read(&o.document, &o.inner_key).unwrap();
    let read = items::read(&compute(), &doc);
    assert_eq!(read.format, Format::Foreign);
    assert_eq!((read.seals, read.next_seal()), (0, 1));
    assert!(read.unreadable.is_empty(), "{:?}", read.unreadable);
    assert_eq!(read.items.len(), 1);
    let Body::Record(r) = &read.items[0].body else {
        panic!()
    };
    assert_eq!(r.kind, kind::ENTRY);
    assert_eq!(r.text(field::TITLE), Some("Example"));
    assert_eq!(r.text(field::USERNAME), Some("alice"));
    assert_eq!(r.text(field::PASSWORD), Some("hunter2"));
    assert_eq!(r.text(field::URL), Some("https://example.org/"));
    assert!(read.items[0].path.is_empty());
}

#[test]
fn a_persons_entry_keeps_its_other_fields_across_a_change() {
    let mut doc = Document::new([0xee; 16], "Vault");
    items::stamp(&mut doc, 1, None).unwrap();
    let e = bad(
        90,
        None,
        &[
            ("Password", "old", true),
            ("PIN", "1234", true),
            ("Floor", "3", false),
        ],
        &[],
    );
    doc.root_mut().push_entry(e).unwrap();
    let first = read(&doc);
    assert_eq!(
        first.items[0].extra,
        [
            Extra {
                name: "PIN".into(),
                protected: true
            },
            Extra {
                name: "Floor".into(),
                protected: false
            },
        ]
    );
    let mut doc = again(&doc);
    let mut r = record_of(&first, 90).clone();
    r.set(field::PASSWORD, b"new");
    items::put(&compute(), &mut doc, &place(90, "", ""), &Body::Record(r)).unwrap();
    let doc = again(&doc);
    let e = doc.root().entries().next().unwrap();
    assert_eq!(e.field("Password").unwrap().value(), "new");
    let pin = e.field("PIN").unwrap();
    assert_eq!((pin.value(), pin.protected()), ("1234", true));
    assert_eq!(e.field("Floor").unwrap().value(), "3");
    assert_eq!(read(&doc).items[0].extra.len(), 2);
}

#[test]
fn an_item_moved_or_renamed_in_a_keepass_app_keeps_its_kind() {
    let c = compute();
    let mut doc = Document::new([0xee; 16], "Vault");
    items::stamp(&mut doc, 1, None).unwrap();
    items::put(
        &c,
        &mut doc,
        &place(1, "Bitcoin keys", "Key 73c5da0a"),
        &Body::Record(seed_record()),
    )
    .unwrap();
    // What a KeePass app writes when the entry is dragged into a group of
    // the person's under a renamed group: the XML moved by hand, with
    // KeePassXC's own custom data beside Faraday's.
    let text = String::from_utf8(doc.write(KEY).to_vec()).unwrap();
    let start = text.find("<Entry>").unwrap();
    let end = text.find("</Entry>").unwrap() + "</Entry>".len();
    let entry = &text[start..end];
    let mut moved = text[..start].to_string() + &text[end..];
    moved = moved.replace("<Name>Bitcoin keys</Name>", "<Name>Clés</Name>");
    let inner = format!(
        "<Group><UUID>{}</UUID><Name>Cold</Name><CustomData><Item><Key>KPXC_RANDOM_SLUG</Key><Value>x</Value></Item></CustomData>{entry}</Group>",
        base64(&[0x44; 16])
    );
    let at = moved.find("<Name>Clés</Name>").unwrap();
    let close = at + moved[at..].find("</Group>").unwrap();
    moved.insert_str(close, &inner);
    let doc = Document::read(moved.as_bytes(), KEY).expect("reads");
    let read = items::read(&c, &doc);
    assert!(read.unreadable.is_empty(), "{:?}", read.unreadable);
    assert_eq!(read.items[0].path, ["Clés", "Cold"]);
    assert_eq!(fields(record_of(&read, 1)), fields(&seed_record()));
}

#[test]
fn an_entry_with_two_fields_of_one_key_or_in_a_group_sharing_a_uuid_is_unreadable() {
    let plain = Record::new(kind::ENTRY).with(field::TITLE, b"plain");
    let mut doc = full();
    items::put(
        &compute(),
        &mut doc,
        &place(92, "", ""),
        &Body::Record(plain),
    )
    .unwrap();
    let field = |k: &str, v: &str| format!("<String><Key>{k}</Key><Value>{v}</Value></String>");
    let twice = format!(
        "<Entry><UUID>{}</UUID>{}{}</Entry>",
        base64(&uuid(90)),
        field("Title", "first"),
        field("Title", "second")
    );
    let in_group = format!(
        "<Entry><UUID>{}</UUID>{}</Entry>",
        base64(&uuid(91)),
        field("Title", "grouped")
    );
    let g = base64(&[0x55; 16]);
    let groups = format!(
        "<Group><UUID>{g}</UUID><Name>A</Name>{in_group}</Group><Group><UUID>{g}</UUID><Name>B</Name></Group>"
    );
    let mut text = String::from_utf8(doc.write(KEY).to_vec()).unwrap();
    let at = text.rfind("</Group></Root>").unwrap();
    text.insert_str(at, &format!("{twice}{groups}"));
    let mut doc = Document::read(text.as_bytes(), KEY).expect("reads");
    let read = items::read(&compute(), &doc);
    let mut got: Vec<(u8, Fault)> = read
        .unreadable
        .iter()
        .map(|u| (u.uuid.unwrap()[0], u.what.fault))
        .collect();
    got.sort_by_key(|g| g.0);
    assert_eq!(got, [(90, Fault::SameField), (91, Fault::SameUuid)]);
    // A change elsewhere leaves both as they were read.
    let r = Record::new(kind::ENTRY).with(field::TITLE, b"plain, changed");
    items::put(&compute(), &mut doc, &place(92, "", ""), &Body::Record(r)).unwrap();
    let after = doc.write(KEY);
    for n in [90, 91] {
        assert_eq!(
            entry_bytes(text.as_bytes(), n),
            entry_bytes(&after, n),
            "entry {n}"
        );
    }
    let r = Record::new(kind::ENTRY).with(field::TITLE, b"x");
    assert_eq!(
        items::put(&compute(), &mut doc, &place(90, "", ""), &Body::Record(r)),
        Err(Unwritable::Unreadable)
    );
    // Neither is put or removed, and an entry in a group sharing a UUID
    // (91) is the same.
    let r = Record::new(kind::ENTRY).with(field::TITLE, b"x");
    assert_eq!(
        items::put(&compute(), &mut doc, &place(91, "", ""), &Body::Record(r)),
        Err(Unwritable::Unreadable)
    );
    for n in [90, 91] {
        assert_eq!(
            items::remove(&compute(), &mut doc, &uuid(n)),
            Err(Unwritable::Unreadable)
        );
    }
    assert_eq!(doc.write(KEY).len(), after.len());
}

#[test]
fn an_ambiguous_entry_is_unreadable_and_is_not_changed() {
    let doc = full();
    let one = format!(
        "<Entry><UUID>{}</UUID><UUID>{}</UUID><String><Key>Title</Key><Value>t</Value></String></Entry>",
        base64(&uuid(95)),
        base64(&uuid(96))
    );
    let mut text = String::from_utf8(doc.write(KEY).to_vec()).unwrap();
    let at = text.rfind("</Group></Root>").unwrap();
    text.insert_str(at, &one);
    let mut doc = Document::read(text.as_bytes(), KEY).expect("reads");
    let read = items::read(&compute(), &doc);
    let got: Vec<&Damage> = read
        .unreadable
        .iter()
        .filter(|u| u.title == "t")
        .map(|u| &u.what)
        .collect();
    assert_eq!(got.len(), 1);
    assert_eq!(got[0].fault, Fault::Ambiguous);
    assert!(!got[0].at.is_empty());
    assert!(!read.read_only());
    let r = Record::new(kind::ENTRY).with(field::TITLE, b"x");
    assert_eq!(
        items::put(&compute(), &mut doc, &place(95, "", ""), &Body::Record(r)),
        Err(Unwritable::Unreadable)
    );
}

#[test]
fn an_ambiguous_database_keeps_its_values_and_is_read_only() {
    let doc = full();
    let mut text = String::from_utf8(doc.write(KEY).to_vec()).unwrap();
    let at = text.find("</DatabaseName>").unwrap() + "</DatabaseName>".len();
    text.insert_str(at, "<DatabaseName>second</DatabaseName>");
    let mut doc = Document::read(text.as_bytes(), KEY).expect("reads");
    let read = items::read(&compute(), &doc);
    assert!(read.read_only());
    assert!(matches!(read.format, Format::Newer(_)));
    assert!(read.custom.is_empty());
    assert!(
        read.unreadable
            .iter()
            .any(|u| u.uuid.is_none() && u.what.fault == Fault::Ambiguous)
    );
    assert_eq!(items::stamp(&mut doc, 2, None), Err(Unwritable::Unreadable));
    assert_eq!(
        items::put_plan(&mut doc, 1, &plan()),
        Err(Unwritable::Unreadable)
    );
    assert_eq!(
        items::put_signed(&mut doc, &signed()),
        Err(Unwritable::Unreadable)
    );
}

/// An entry written into a document's text, before the last group's end.
fn with_text(doc: &Document, inserted: &str) -> Document {
    let mut text = String::from_utf8(doc.write(KEY).to_vec()).unwrap();
    let at = text.rfind("</Group></Root>").unwrap();
    text.insert_str(at, inserted);
    Document::read(text.as_bytes(), KEY).expect("reads")
}

#[test]
fn an_entry_in_an_ambiguous_group_is_not_put_or_removed() {
    let field = |k: &str, v: &str| format!("<String><Key>{k}</Key><Value>{v}</Value></String>");
    let group = format!(
        "<Group><UUID>{}</UUID><Name>A</Name><Name>B</Name><Entry><UUID>{}</UUID>{}</Entry></Group>",
        base64(&[0x66; 16]),
        base64(&uuid(97)),
        field("Title", "inside")
    );
    let mut doc = with_text(&full(), &group);
    let read = items::read(&compute(), &doc);
    assert!(
        read.unreadable
            .iter()
            .any(|u| u.uuid == Some(uuid(97)) && u.what.fault == Fault::Ambiguous)
    );
    let r = Record::new(kind::ENTRY).with(field::TITLE, b"x");
    assert_eq!(
        items::put(&compute(), &mut doc, &place(97, "", ""), &Body::Record(r)),
        Err(Unwritable::Unreadable)
    );
    assert_eq!(
        items::remove(&compute(), &mut doc, &uuid(97)),
        Err(Unwritable::Unreadable)
    );
}

#[test]
fn a_damaged_item_and_a_duplicate_are_not_removed_a_good_one_is_and_a_newer_vault_refuses() {
    let (mut doc, _) = with_damage();
    let c = compute();
    assert_eq!(
        items::remove(&c, &mut doc, &uuid(66)),
        Err(Unwritable::Unreadable)
    );
    assert_eq!(items::remove(&c, &mut doc, &uuid(1)), Ok(true));
    assert_eq!(items::remove(&c, &mut doc, &uuid(1)), Ok(false));
    let mut newer = Document::read(
        String::from_utf8(doc.write(KEY).to_vec())
            .unwrap()
            .replace(
                "<Key>faraday.format</Key><Value>1</Value>",
                "<Key>faraday.format</Key><Value>2</Value>",
            )
            .as_bytes(),
        KEY,
    )
    .expect("reads");
    assert_eq!(
        items::remove(&c, &mut newer, &uuid(2)),
        Err(Unwritable::ReadOnly)
    );
}

#[test]
fn two_format_items_make_every_item_unreadable_and_nothing_changes() {
    let doc = full();
    let mut text = String::from_utf8(doc.write(KEY).to_vec()).unwrap();
    let at = text.find("<Key>faraday.format</Key>").expect("format item");
    let end = text[at..].find("</Item>").unwrap() + at + "</Item>".len();
    let item_start = text[..at].rfind("<Item>").unwrap();
    let item = text[item_start..end].to_string();
    text.insert_str(end, &item);
    let mut doc = Document::read(text.as_bytes(), KEY).expect("reads");
    let read = items::read(&compute(), &doc);
    assert!(read.read_only());
    assert!(read.items.iter().all(|i| i.body.kind() == kind::ENTRY));
    assert!(read.unreadable.iter().any(|u| u.uuid.is_some()));
    let r = Record::new(kind::ENTRY).with(field::TITLE, b"x");
    assert_eq!(
        items::put(&compute(), &mut doc, &place(99, "", ""), &Body::Record(r)),
        Err(Unwritable::Unreadable)
    );
    assert_eq!(
        items::remove_custom(&mut doc, "faraday.plan.1"),
        Err(Unwritable::Unreadable)
    );
    assert_eq!(
        items::remove(&compute(), &mut doc, &uuid(1)),
        Err(Unwritable::Unreadable)
    );
}

#[test]
fn a_persons_entry_reads_with_the_totp_it_holds() {
    let field = |k: &str, v: &str| format!("<String><Key>{k}</Key><Value>{v}</Value></String>");
    let entry = |n: u8, fields: &[(&str, &str)]| {
        let f: String = fields.iter().map(|(k, v)| field(k, v)).collect();
        format!("<Entry><UUID>{}</UUID>{f}</Entry>", base64(&uuid(n)))
    };
    let b32 = "GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ";
    let xml = [
        entry(
            80,
            &[
                ("Title", "hotp"),
                ("otp", "otpauth://hotp/x?secret=JBSWY3DPEHPK3PXP"),
            ],
        ),
        entry(81, &[("Title", "junk"), ("otp", "not a key")]),
        entry(82, &[("Title", "kp2"), ("TimeOtp-Secret-Base32", b32)]),
        entry(
            83,
            &[
                ("Title", "kpxc"),
                ("TOTP Seed", b32),
                ("TOTP Settings", "30;8"),
            ],
        ),
        entry(84, &[("Title", "none")]),
    ]
    .concat();
    let doc = with_text(&full(), &xml);
    let read = items::read(&compute(), &doc);
    assert!(read.unreadable.is_empty(), "{:?}", read.unreadable);
    let totp = |n: u8| &read.items.iter().find(|i| i.uuid == uuid(n)).unwrap().totp;
    assert_eq!(
        totp(80).as_ref().unwrap().as_ref().unwrap_err(),
        &NotTotp::Hotp
    );
    assert_eq!(
        totp(81).as_ref().unwrap().as_ref().unwrap_err(),
        &NotTotp::Unreadable
    );
    // RFC 6238's SHA-1 vector at 59 s, 8 digits.
    assert_eq!(
        totp(83).as_ref().unwrap().as_ref().unwrap().code(59),
        "94287082"
    );
    assert_eq!(
        totp(82).as_ref().unwrap().as_ref().unwrap().code(59),
        "287082"
    );
    assert!(totp(84).is_none());
    // The other fields are listed by name.
    let kp2 = read.items.iter().find(|i| i.uuid == uuid(82)).unwrap();
    assert_eq!(kp2.extra[0].name, "TimeOtp-Secret-Base32");
}

fn uuid_of_kind(doc: &Document, k: u8) -> [u8; 16] {
    read(doc)
        .items
        .iter()
        .find(|i| matches!(&i.body, Body::Record(r) if r.kind == k))
        .expect("the item")
        .uuid
}

fn passphrase_for(fp: [u8; 4]) -> Body {
    Body::Record(
        Record::new(kind::PASSPHRASE)
            .with(field::PASSPHRASE_KEY, &fp)
            .with(field::PASSPHRASE, b"x"),
    )
}

fn share_of(string: &str) -> Body {
    Body::Record(
        Record::new(kind::CODEX32_SHARE)
            .with(field::CODEX32_STRING, string.as_bytes())
            .with(field::CODEX32_KEY, &FP),
    )
}

#[test]
fn a_second_passphrase_or_share_for_one_key_is_not_written() {
    let c = compute();
    let mut doc = full();
    let at = |n: u8| place(n, "Passphrases", "");
    assert_eq!(
        items::put(&c, &mut doc, &at(80), &passphrase_for(FP)),
        Err(Unwritable::Twice("for"))
    );
    assert_eq!(
        items::put(&c, &mut doc, &at(81), &share_of(SHARE_A)),
        Err(Unwritable::Twice("for"))
    );
    // Another key, another index, and the item itself again.
    assert_eq!(
        items::put(&c, &mut doc, &at(80), &passphrase_for([1, 2, 3, 4])),
        Ok(())
    );
    assert_eq!(
        items::put(&c, &mut doc, &at(81), &share_of(SHARE_C)),
        Ok(())
    );
    let own = Place {
        uuid: uuid_of_kind(&doc, kind::PASSPHRASE),
        ..at(0)
    };
    assert_eq!(items::put(&c, &mut doc, &own, &passphrase_for(FP)), Ok(()));
    assert!(read(&doc).unreadable.is_empty());
}

#[test]
fn one_of_two_items_for_one_key_can_be_removed_to_undo_it() {
    let c = compute();
    let mut doc = Document::new([0xee; 16], "Vault");
    items::stamp(&mut doc, 1, None).unwrap();
    let pass = |n: u8, fp: &str| {
        bad(
            n,
            Some("passphrase"),
            &[("Password", "a", true), ("for", fp, false)],
            &[],
        )
    };
    for n in [91, 92] {
        doc.root_mut().push_entry(pass(n, "73c5da0a")).unwrap();
    }
    doc.root_mut().push_entry(pass(95, "zz")).unwrap();
    let r = read(&doc);
    assert!(r.items.is_empty());
    assert_eq!(r.unreadable.len(), 3);
    // Neither is changed, and a third is not written.
    assert_eq!(
        items::put(
            &c,
            &mut doc,
            &place(91, "Passphrases", ""),
            &passphrase_for(FP)
        ),
        Err(Unwritable::Unreadable)
    );
    assert_eq!(
        items::put(
            &c,
            &mut doc,
            &place(93, "Passphrases", ""),
            &passphrase_for(FP)
        ),
        Err(Unwritable::Twice("for"))
    );
    // Any other damage is still refused.
    assert_eq!(
        items::remove(&c, &mut doc, &uuid(95)),
        Err(Unwritable::Unreadable)
    );
    assert_eq!(items::remove(&c, &mut doc, &uuid(91)), Ok(true));
    assert_eq!(read(&doc).items.len(), 1);
    assert_eq!(items::remove(&c, &mut doc, &uuid(92)), Ok(true));
}

#[test]
fn a_persons_entry_with_a_field_of_any_length_reads_and_keeps_it() {
    let c = compute();
    let mut doc = Document::new([0xee; 16], "Vault");
    items::stamp(&mut doc, 1, None).unwrap();
    let big = "x".repeat(65_536);
    doc.root_mut()
        .push_entry(bad(
            90,
            None,
            &[("Notes", &big, false), ("Password", "old", true)],
            &[],
        ))
        .unwrap();
    let first = read(&doc);
    assert!(first.unreadable.is_empty());
    assert_eq!(
        first.items[0].extra,
        [Extra {
            name: "Notes".into(),
            protected: false
        }]
    );
    assert!(record_of(&first, 90).field(field::NOTES).is_none());
    let mut doc = again(&doc);
    let mut r = record_of(&first, 90).clone();
    r.set(field::PASSWORD, b"new");
    items::put(&c, &mut doc, &place(90, "", ""), &Body::Record(r.clone())).unwrap();
    let doc = again(&doc);
    let e = doc.root().entries().next().unwrap();
    assert_eq!(e.field("Password").unwrap().value(), "new");
    assert_eq!(e.field("Notes").unwrap().value().len(), 65_536);
    // Given a value, the field is replaced.
    let mut doc = doc;
    r.set(field::NOTES, b"short");
    items::put(&c, &mut doc, &place(90, "", ""), &Body::Record(r)).unwrap();
    let e = doc.root().entries().next().unwrap();
    assert_eq!(e.field("Notes").unwrap().value(), "short");
}

#[test]
fn a_plan_or_signed_amount_that_does_not_read_is_neither_replaced_nor_removed() {
    let mut doc = full();
    let tx = "cd".repeat(32);
    let signed_key = format!("faraday.signed.{tx}");
    doc.custom_mut()
        .set("faraday.plan.2", "not a plan")
        .unwrap();
    doc.custom_mut().set(&signed_key, "x").unwrap();
    assert_eq!(
        items::put_plan(&mut doc, 2, &plan()),
        Err(Unwritable::Unreadable)
    );
    assert_eq!(
        items::remove_custom(&mut doc, "faraday.plan.2"),
        Err(Unwritable::Unreadable)
    );
    let amounts = Record::new(kind::AMOUNTS)
        .with(field::AMOUNTS_TX, &[0xcd; 32])
        .with(field::AMOUNTS_INPUT, &1u64.to_le_bytes());
    assert_eq!(
        items::put_signed(&mut doc, &amounts),
        Err(Unwritable::Unreadable)
    );
    assert_eq!(
        items::remove_custom(&mut doc, &signed_key),
        Err(Unwritable::Unreadable)
    );
    // A new number and a plan that reads are written and removed as before.
    assert_eq!(items::put_plan(&mut doc, 3, &plan()), Ok(()));
    assert_eq!(items::remove_custom(&mut doc, "faraday.plan.1"), Ok(true));
    assert_eq!(read(&doc).unreadable.len(), 2);
}
