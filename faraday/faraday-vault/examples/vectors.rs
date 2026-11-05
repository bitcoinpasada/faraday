//! Writes the file vault's vectors (`docs/VAULT.md` §10) into a directory:
//! KDBX 4 files, each one this build writes and `tools/vault/open.py` reads
//! with pykeepass.
//!
//! ```text
//! cargo run --locked -p faraday-vault --example vectors -- tools/vectors/vault
//! ```
//!
//! - `vault-all.kdbx`: a 256 KiB vault with one item of every kind of §7
//!   and a person's entry, a backup plan and signed amounts.
//! - `vault-small.kdbx`: a 64 KiB vault with an entry and a note.
//! - `vault-codex32.kdbx`: a seed with its codex32 set's facts and one share
//!   (`docs/CODEX32.md` §9.4, §9.5).
//! - `vault-nfd.kdbx`: made with "café" in NFD, which opens under its NFC
//!   form (§3).
//! - `vault-sealed.kdbx`: a vault made, opened and sealed once more through
//!   `file::seal`, so at the cost a seal writes (64 MiB, 3 passes, 4 lanes)
//!   with its seal count 2.
//! - `cut-stream.kdbx`: a vault whose block stream ends before its empty
//!   last block.
//! - `refused-*.kdbx`: one file per rule of §1, §3, §4, §7 and §7.1 that a
//!   file can break, each with the refusal [`refused`] names.
//!
//! Everything is fixed: the random bytes, the ids, the clock and the
//! passphrases, at 8 MiB, one pass and one lane (the least a reader
//! accepts), so a rerun writes the same bytes. The one exception to the
//! cost is `vault-sealed.kdbx`. Item vaults are written by putting each
//! item into a document with the functions `file::seal` applies a change
//! with, and sealing it with `kdbx::seal` at that cost, because
//! `file::seal` always writes the 64 MiB step. The words are BIP-39's
//! all-`abandon` test phrase; no coin is behind anything here, and the GPG
//! and Secure Boot keys are placeholders.
#![allow(dead_code)]

use faraday_crypto::work::Compute;
use faraday_vault::child::{self, ChildOf};
use faraday_vault::codex32::{SetFacts, State, set_payload};
use faraday_vault::file::{self, Change, Error, SIZES};
use faraday_vault::kdbx::items::{self, Body, GpgKey, Place};
use faraday_vault::kdbx::xml::{Document, Refused};
use faraday_vault::kdbx::{self, Contents, RANDOM_LEN, Unsupported};
use faraday_vault::records::{BACKUP_PENDING, NO_LOAD_AT_UNLOCK, field, kind};
use faraday_vault::{Cost, Record, layouts};
use zeroize::Zeroizing;

/// A token for the vectors' key work (`docs/COMPUTE.md` §3), counting
/// nothing. Cheap.
#[expect(
    clippy::disallowed_methods,
    reason = "a mint: a tool that draws nothing, and its test (docs/COMPUTE.md §3)"
)]
pub fn compute() -> Compute {
    Compute::new(|_| {})
}

/// The cost the vectors are written at: 8 MiB, one pass, one lane.
pub const COST: Cost = Cost {
    memory_kib: 8 * 1024,
    passes: 1,
    lanes: 1,
};

/// The clock the vectors are sealed at, 2023-11-14T22:13:20Z.
pub const NOW: u64 = 1_700_000_000;
const STAMP: &str = "2023-11-14T22:13:20Z";

/// The passphrase of every vector but the NFD one.
pub const PASSPHRASE: &str = "correct horse battery staple";
/// The NFD vault's passphrase, "café" with its e and the combining acute
/// apart (U+0065 U+0301).
pub const NFD_PASSPHRASE: &str = "cafe\u{301}";
/// [`NFD_PASSPHRASE`] in NFC (U+00E9), which opens the same file.
pub const NFD_AS_NFC: &str = "caf\u{e9}";

/// The 256 KiB vault with one item of every kind.
pub const ALL: &str = "vault-all.kdbx";
/// The 64 KiB vault.
pub const SMALL: &str = "vault-small.kdbx";
/// The codex32 vault.
pub const CODEX32: &str = "vault-codex32.kdbx";
/// The NFD vault.
pub const NFD: &str = "vault-nfd.kdbx";
/// The vault sealed through `file::seal`.
pub const SEALED: &str = "vault-sealed.kdbx";
/// The vault whose stream ends early.
pub const CUT: &str = "cut-stream.kdbx";

const XPUB: &str = "[73c5da0a/84h/0h/0h]xpub6CatWdiZiodmUeTDp8LT5or8nmbKNcuyvz7WyksVFkKB4RHwCD3XyuvPEbvqAQY3rAPshWcMLoP2fMFMKHPJ4ZeZXYVUhLv1VMrjPC7PW6V";
const WORDS: &str =
    "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
const FP: [u8; 4] = [0x73, 0xc5, 0xda, 0x0a];
const DER: &[u8] = &[0x30, 0x03, 0x02, 0x01, 0x01];
/// An armor's kind, built so that no whole armor line of a secret key is
/// written in the source.
const SECRET: &str = "PRIVATE";
const GPG_PUBLIC: &str =
    "-----BEGIN PGP PUBLIC KEY BLOCK-----\n\nxjME\n-----END PGP PUBLIC KEY BLOCK-----\n";

/// BIP 93's vector 2: its seed, a share of it, and the fingerprint of the
/// key the seed makes.
pub const VECTOR_2: (&str, &str, [u8; 4]) = (
    "d1808e096b35b209ca12132b264662a5",
    "ms12namecacdefghjklmnpqrstuvwxyz023ftr2gdzmpy6pn",
    [0xfa, 0xb6, 0x86, 0x8a],
);

/// 140 bytes counted from `seed`: the random bytes of one seal.
pub fn random(seed: u8) -> [u8; RANDOM_LEN] {
    std::array::from_fn(|i| (i as u8).wrapping_mul(7).wrapping_add(seed))
}

fn gpg_key() -> String {
    format!(
        "-----BEGIN PGP {SECRET} KEY BLOCK-----\n\nxVgEZ0\n-----END PGP {SECRET} KEY BLOCK-----\n"
    )
}

fn words_payload() -> Zeroizing<Vec<u8>> {
    let m = faraday_bip::bip39::Mnemonic::parse(
        &compute(),
        faraday_bip::bip39::Language::English,
        WORDS,
    )
    .expect("the BIP-39 test phrase");
    layouts::words_payload(&m)
}

/// One item: its record or key, the group it goes in and its title.
type Item = (Body, &'static str, &'static str);

/// One item of every kind of §7, a person's entry among them.
fn every_kind() -> Vec<Item> {
    let descriptor = format!("wpkh({XPUB}/<0;1>/*)");
    let facts = SetFacts {
        threshold: 2,
        identifier: *b"name",
        length: 48,
        unknown: 1,
        strings: vec![(b'a', State::Placed), (b'c', State::Made)],
    };
    let parent = ChildOf {
        parent: [1, 2, 3, 4],
        language: 0,
        words: 12,
        index: 7,
    };
    let seed = Record::new(kind::KEY)
        .with(field::KEY, &words_payload())
        .with(field::KEY_LABEL, b"Savings")
        .with(field::KEY_FLAGS, &[NO_LOAD_AT_UNLOCK | BACKUP_PENDING])
        .with(field::KEY_PASSPHRASE, b"TREZOR")
        .with(
            field::KEY_CODEX32,
            &set_payload(&facts).expect("a sound set"),
        )
        .with(field::KEY_CHILD, &child::payload(&parent).expect("a child"));
    let hex = Record::new(kind::KEY).with(
        field::KEY,
        &layouts::seed_payload(&[0x5a; 32]).expect("32 bytes"),
    );
    let sheet = layouts::sheet_payload(descriptor.as_bytes(), b"Spending", b"in the safe")
        .expect("a small sheet");
    let mut boot = Record::new(kind::SECURE_BOOT).with(1, &[0x11; 16]);
    for n in 2..=7 {
        boot.push(n, DER);
    }
    let gpg = GpgKey {
        title: "Test <test@example.com>".into(),
        key: Zeroizing::new(gpg_key()),
        public: Some(GPG_PUBLIC.into()),
        revocation: Some(Zeroizing::new(GPG_PUBLIC.into())),
        revoked: Some("2026-10-28T14:02:00Z".into()),
    };
    vec![
        (Body::Record(seed), "Bitcoin keys", "Key 73c5da0a"),
        (Body::Record(hex), "Bitcoin keys", "Key 01020304"),
        (
            Body::Record(
                Record::new(kind::WALLET)
                    .with(field::WALLET, format!("wpkh(@0/**)\n{XPUB}").as_bytes())
                    .with(field::WALLET_NAME, b"Spending"),
            ),
            "Wallets",
            "",
        ),
        (
            Body::Record(
                Record::new(kind::WALLET)
                    .with(field::WALLET, descriptor.as_bytes())
                    .with(field::WALLET_NAME, b"Descriptor wallet"),
            ),
            "Wallets",
            "",
        ),
        (
            Body::Record(
                Record::new(kind::NOTE).with(field::NOTE, b"The safe's code is in the drawer."),
            ),
            "Notes",
            "Note",
        ),
        (
            Body::Record(Record::new(kind::SHEET).with(field::SHEET, &sheet)),
            "Recovery sheets",
            "",
        ),
        (Body::Gpg(gpg), "GPG keys", ""),
        (Body::Record(boot), "Secure Boot", "Secure Boot keys"),
        (
            Body::Record(
                Record::new(kind::WALLET_SHARE)
                    .with(field::SHARE, b"share text")
                    .with(field::SHARE_WALLET_NAME, b"Family")
                    .with(field::SHARE_NUMBER, &[1])
                    .with(field::SHARE_OF, &[3])
                    .with(field::SHARE_WALLET, descriptor.as_bytes()),
            ),
            "Wallet shares",
            "",
        ),
        (
            Body::Record(
                Record::new(kind::PASSPHRASE)
                    .with(field::PASSPHRASE_KEY, &FP)
                    .with(field::PASSPHRASE, b"TREZOR"),
            ),
            "Passphrases",
            "Passphrase for 73c5da0a",
        ),
        (
            Body::Record(
                Record::new(kind::CODEX32_SHARE)
                    .with(field::CODEX32_STRING, VECTOR_2.1.as_bytes())
                    .with(field::CODEX32_KEY, &VECTOR_2.2),
            ),
            "Codex32 shares",
            "Codex32 share for fab6868a",
        ),
        (Body::Record(entry()), "", ""),
    ]
}

fn entry() -> Record {
    Record::new(kind::ENTRY)
        .with(field::TITLE, b"Email")
        .with(field::USERNAME, b"you@example.com")
        .with(field::PASSWORD, b"hunter2")
        .with(field::URL, b"https://mail.example.com")
        .with(field::NOTES, b"Recovery codes are in the note.")
        .with(
            field::TOTP,
            b"otpauth://totp/Example:you?secret=JBSWY3DPEHPK3PXP&issuer=Example",
        )
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

/// The UUID of the `n`-th item.
fn uuid(n: u8) -> [u8; 16] {
    [n; 16]
}

fn place<'a>(n: u8, group: &'a str, title: &'a str) -> Place<'a> {
    Place {
        uuid: uuid(n),
        group,
        group_uuid: [n.wrapping_add(100); 16],
        title,
        now: Some(NOW as i64),
    }
}

/// The document of a first seal (count 1, as `file::create` stamps it)
/// holding `items` and, when `more`, a plan and signed amounts.
fn document(id: u8, label: &str, items: &[Item], more: bool) -> Document {
    let c = compute();
    let mut doc = Document::new([id; 16], label);
    items::stamp(&mut doc, 1, Some(STAMP)).expect("stamps");
    // Items of one group share its UUID, as `file::seal` puts them in it.
    let mut groups: Vec<(&str, [u8; 16])> = vec![];
    for (i, (body, group, title)) in items.iter().enumerate() {
        let n = i as u8 + 1;
        let mut at = place(n, group, title);
        match groups.iter().find(|(g, _)| g == group) {
            Some((_, u)) => at.group_uuid = *u,
            None => groups.push((group, at.group_uuid)),
        }
        items::put(&c, &mut doc, &at, body).expect("puts");
    }
    if more {
        items::put_plan(&mut doc, 1, &plan()).expect("a plan");
        items::put_signed(&mut doc, &signed()).expect("amounts");
    }
    doc
}

/// A file of `doc`, sealed at [`COST`] and `size` under `passphrase`.
fn sealed(doc: &Document, passphrase: &str, size: usize, seed: u8) -> Vec<u8> {
    let random = random(seed);
    let xml = doc.write(kdbx::inner_key_of(&random));
    kdbx::seal(
        &compute(),
        passphrase,
        COST,
        &random,
        &Contents {
            document: &xml,
            binaries: faraday_vault::kdbx::Binaries::all(&[]),
            public_custom_data: None,
        },
        size,
    )
    .expect("the vectors")
}

fn codex32_items() -> Vec<Item> {
    let seed: Vec<u8> = (0..16)
        .map(|i| u8::from_str_radix(&VECTOR_2.0[2 * i..2 * i + 2], 16).expect("hex"))
        .collect();
    let facts = SetFacts {
        threshold: 2,
        identifier: *b"name",
        length: 48,
        unknown: 0,
        strings: vec![
            (b'a', State::Placed),
            (b'c', State::Placed),
            (b'd', State::Made),
        ],
    };
    vec![
        (
            Body::Record(
                Record::new(kind::KEY)
                    .with(field::KEY, &layouts::seed_payload(&seed).expect("16 bytes"))
                    .with(field::KEY_LABEL, b"Codex32 key")
                    .with(
                        field::KEY_CODEX32,
                        &set_payload(&facts).expect("a sound set"),
                    ),
            ),
            "Bitcoin keys",
            "",
        ),
        (
            Body::Record(
                Record::new(kind::CODEX32_SHARE)
                    .with(field::CODEX32_STRING, VECTOR_2.1.as_bytes())
                    .with(field::CODEX32_KEY, &VECTOR_2.2),
            ),
            "Codex32 shares",
            "Codex32 share for fab6868a",
        ),
    ]
}

/// The vault made and sealed again through `file::create` and
/// `file::seal`: a note and an entry added, the seal count 2.
pub fn sealed_through_file() -> Vec<u8> {
    let c = compute();
    let first = file::create(
        &c,
        [0x5a; 16],
        "Sealed",
        COST,
        SIZES[0],
        PASSPHRASE,
        &random(40),
        Some(NOW),
    )
    .expect("creates");
    let opened = file::open(&c, &first, PASSPHRASE).expect("opens");
    let note = Body::Record(Record::new(kind::NOTE).with(field::NOTE, b"Added at a seal."));
    let entry = Body::Record(entry());
    file::seal(
        &c,
        &opened,
        &[
            Change::Put {
                place: place(1, "Notes", "Note"),
                body: &note,
            },
            Change::Put {
                place: place(2, "", ""),
                body: &entry,
            },
            Change::Label("Sealed again"),
        ],
        &random(41),
        Some(NOW + 60),
    )
    .expect("seals")
}

// ------------------------------------------------------------- refused

const CHACHA20: [u8; 16] = [
    0xD6, 0x03, 0x8A, 0x2B, 0x8B, 0x6F, 0x4C, 0xB5, 0xA5, 0x24, 0x33, 0x9A, 0x31, 0xDB, 0xB5, 0x9A,
];
const ARGON2ID: [u8; 16] = [
    0x9E, 0x29, 0x8B, 0x19, 0x56, 0xDB, 0x47, 0x73, 0xB2, 0x3D, 0xFC, 0x3E, 0xC6, 0xF0, 0xA1, 0xE6,
];
const AES_KDF: [u8; 16] = [
    0xC9, 0xD9, 0xF3, 0x9A, 0x62, 0x8A, 0x44, 0x60, 0xBF, 0x74, 0x0D, 0x08, 0xC1, 0x8A, 0x4F, 0xEA,
];

/// A header with each part set: the refusals of §3 and §7.1 that are made
/// from the header alone.
#[derive(Clone)]
struct Head {
    version: u32,
    cipher: [u8; 16],
    compression: u32,
    kdf: [u8; 16],
    salt_len: usize,
    memory: u64,
    passes: u64,
    lanes: u32,
    argon_version: Option<u32>,
    secret_key: Option<Vec<u8>>,
    extra: Option<u8>,
}

fn head() -> Head {
    Head {
        version: 0x0004_0000,
        cipher: CHACHA20,
        compression: 0,
        kdf: ARGON2ID,
        salt_len: 32,
        memory: 8 * 1024 * 1024,
        passes: 1,
        lanes: 1,
        argon_version: Some(0x13),
        secret_key: None,
        extra: None,
    }
}

fn with(change: impl FnOnce(&mut Head)) -> Head {
    let mut h = head();
    change(&mut h);
    h
}

fn field_of(id: u8, value: &[u8], out: &mut Vec<u8>) {
    out.push(id);
    out.extend_from_slice(&(value.len() as u32).to_le_bytes());
    out.extend_from_slice(value);
}

fn item(kind: u8, name: &[u8], value: &[u8], out: &mut Vec<u8>) {
    out.push(kind);
    out.extend_from_slice(&(name.len() as u32).to_le_bytes());
    out.extend_from_slice(name);
    out.extend_from_slice(&(value.len() as u32).to_le_bytes());
    out.extend_from_slice(value);
}

/// A file of the header, its hash and a stand-in HMAC, nothing more: a
/// reader refuses it before any key work.
fn header_only(h: &Head) -> Vec<u8> {
    let mut out = vec![];
    out.extend_from_slice(&0x9AA2_D903u32.to_le_bytes());
    out.extend_from_slice(&0xB54B_FB67u32.to_le_bytes());
    out.extend_from_slice(&h.version.to_le_bytes());
    field_of(2, &h.cipher, &mut out);
    field_of(3, &h.compression.to_le_bytes(), &mut out);
    field_of(4, &[1; 32], &mut out);
    field_of(7, &[2; 12], &mut out);
    let mut kdf = vec![0, 1];
    item(0x42, b"$UUID", &h.kdf, &mut kdf);
    item(0x42, b"S", &vec![3; h.salt_len], &mut kdf);
    item(0x05, b"I", &h.passes.to_le_bytes(), &mut kdf);
    item(0x05, b"M", &h.memory.to_le_bytes(), &mut kdf);
    item(0x04, b"P", &h.lanes.to_le_bytes(), &mut kdf);
    if let Some(v) = h.argon_version {
        item(0x04, b"V", &v.to_le_bytes(), &mut kdf);
    }
    if let Some(k) = &h.secret_key {
        item(0x42, b"K", k, &mut kdf);
    }
    kdf.push(0);
    field_of(11, &kdf, &mut out);
    if let Some(id) = h.extra {
        field_of(id, b"x", &mut out);
    }
    field_of(0, &[0x0D, 0x0A, 0x0D, 0x0A], &mut out);
    let hash = faraday_crypto::sha256(&out);
    out.extend_from_slice(&hash);
    out.extend_from_slice(&[0; 32]);
    out
}

/// A sealed file whose XML document is `text`, 4 KiB of plaintext.
fn with_document(text: &[u8], seed: u8) -> Vec<u8> {
    let random = random(seed);
    kdbx::seal(
        &compute(),
        PASSPHRASE,
        COST,
        &random,
        &Contents {
            document: text,
            binaries: faraday_vault::kdbx::Binaries::all(&[]),
            public_custom_data: None,
        },
        4096,
    )
    .expect("the vectors")
}

const ID: &str = "AQEBAQEBAQEBAQEBAQEBAQ==";

fn skeleton(meta: &str, group: &str) -> Vec<u8> {
    format!(
        "<?xml version=\"1.0\" encoding=\"utf-8\" standalone=\"yes\"?>\n<KeePassFile><Meta>{meta}</Meta><Root><Group><UUID>{ID}</UUID><Name>V</Name>{group}</Group></Root></KeePassFile>\n"
    )
    .into_bytes()
}

fn in_name(name: &str) -> Vec<u8> {
    format!("<?xml version=\"1.0\"?><KeePassFile><Meta></Meta><Root><Group><UUID>{ID}</UUID><Name>{name}</Name></Group></Root></KeePassFile>").into_bytes()
}

/// The refusals made from a header alone (§1, §3, §7.1).
fn header_refusals() -> Vec<(&'static str, Vec<u8>, Error)> {
    let unsupported = |u| Error::Kdbx(kdbx::Error::Unsupported(u));
    let cost = || Error::Kdbx(kdbx::Error::Cost);
    vec![
        (
            "refused-not-kdbx.kdbx",
            b"This is not a KeePass file, whatever its name.".to_vec(),
            Error::Kdbx(kdbx::Error::NotKdbx),
        ),
        (
            "refused-version-5.kdbx",
            header_only(&with(|h| h.version = 0x0005_0000)),
            Error::Kdbx(kdbx::Error::Newer { major: 5, minor: 0 }),
        ),
        (
            "refused-version-4-2.kdbx",
            header_only(&with(|h| h.version = 0x0004_0002)),
            Error::Kdbx(kdbx::Error::Newer { major: 4, minor: 2 }),
        ),
        (
            "refused-header-field.kdbx",
            header_only(&with(|h| h.extra = Some(99))),
            Error::Kdbx(kdbx::Error::UnknownField(99)),
        ),
        (
            "refused-aes-kdf.kdbx",
            header_only(&with(|h| h.kdf = AES_KDF)),
            unsupported(Unsupported::AesKdf),
        ),
        (
            "refused-kdf-other.kdbx",
            header_only(&with(|h| h.kdf = [9; 16])),
            unsupported(Unsupported::OtherKdf),
        ),
        (
            "refused-cipher-other.kdbx",
            header_only(&with(|h| h.cipher = [9; 16])),
            unsupported(Unsupported::OtherCipher),
        ),
        (
            "refused-compression-other.kdbx",
            header_only(&with(|h| h.compression = 2)),
            unsupported(Unsupported::Compression),
        ),
        (
            "refused-memory-low.kdbx",
            header_only(&with(|h| h.memory = (8 << 20) - 1024)),
            cost(),
        ),
        (
            "refused-memory-high.kdbx",
            header_only(&with(|h| h.memory = (1 << 30) + 1024)),
            cost(),
        ),
        (
            "refused-passes-0.kdbx",
            header_only(&with(|h| h.passes = 0)),
            cost(),
        ),
        (
            "refused-passes-101.kdbx",
            header_only(&with(|h| h.passes = 101)),
            cost(),
        ),
        (
            "refused-lanes-0.kdbx",
            header_only(&with(|h| h.lanes = 0)),
            cost(),
        ),
        (
            "refused-lanes-65.kdbx",
            header_only(&with(|h| h.lanes = 65)),
            cost(),
        ),
        (
            "refused-argon2-version.kdbx",
            header_only(&with(|h| h.argon_version = Some(0x10))),
            cost(),
        ),
        (
            "refused-secret-key.kdbx",
            header_only(&with(|h| h.secret_key = Some(vec![1]))),
            cost(),
        ),
        (
            "refused-kdf-without-version.kdbx",
            header_only(&with(|h| h.argon_version = None)),
            Error::Kdbx(kdbx::Error::Damaged),
        ),
    ]
}

/// The files each rule of §1, §3, §4, §7 and §7.1 refuses that a file can
/// break, as (file name, bytes, the refusal `file::open` gives under
/// [`PASSPHRASE`]). KDBX 3.1 and Twofish are the foreign files of
/// `tools/vectors/kdbx/`.
pub fn refused() -> Vec<(&'static str, Vec<u8>, Error)> {
    let mut v = header_refusals();
    // A vault with one byte changed in each place a check covers.
    let good = vault_for_damage();
    // The header is 249 bytes, then its SHA-256 and HMAC (32 each), then the
    // first block: its HMAC (32), its length (4), its ciphertext.
    let (hash_at, hmac_at, block_at) = (249, 281, 313 + 36 + 8);
    let flip = |at: usize| {
        let mut f = good.clone();
        f[at] ^= 1;
        f
    };
    v.push((
        "refused-header-hash.kdbx",
        flip(hash_at),
        Error::Kdbx(kdbx::Error::Damaged),
    ));
    v.push((
        "refused-header-hmac.kdbx",
        flip(hmac_at),
        Error::Kdbx(kdbx::Error::Passphrase),
    ));
    v.push((
        "refused-block-hmac.kdbx",
        flip(313),
        Error::Kdbx(kdbx::Error::Damaged),
    ));
    v.push((
        "refused-block-data.kdbx",
        flip(block_at),
        Error::Kdbx(kdbx::Error::Damaged),
    ));
    v.push((
        "refused-cut-block.kdbx",
        good[..good.len() / 2].to_vec(),
        Error::Kdbx(kdbx::Error::Damaged),
    ));
    v.extend(xml_refusals());
    v
}

/// The refusals of the XML document (§7, §7.1), each in a file of its own.
fn xml_refusals() -> Vec<(&'static str, Vec<u8>, Error)> {
    let mut after = in_name("V");
    after.extend_from_slice(b"x");
    let entry = |inner: &str| skeleton("", &format!("<Entry><UUID>{ID}</UUID>{inner}</Entry>"));
    let deep = format!("{}{}", "<a>".repeat(70), "</a>".repeat(70));
    let xmls: Vec<(&'static str, Vec<u8>, Refused)> = vec![
        (
            "refused-xml-not-well-formed.kdbx",
            b"<KeePassFile><Meta></KeePassFile>".to_vec(),
            Refused::NotWellFormed,
        ),
        (
            "refused-xml-doctype.kdbx",
            b"<!DOCTYPE x [<!ENTITY e \"v\">]><KeePassFile/>".to_vec(),
            Refused::DocType,
        ),
        (
            "refused-xml-entity.kdbx",
            in_name("&foo;"),
            Refused::Entity(b"foo".to_vec()),
        ),
        (
            "refused-xml-declaration.kdbx",
            b"<KeePassFile><?xml version=\"1.0\"?></KeePassFile>".to_vec(),
            Refused::ProcessingInstruction(b"xml".to_vec()),
        ),
        (
            "refused-xml-encoding.kdbx",
            b"<?xml version=\"1.0\" encoding=\"iso-8859-1\"?><KeePassFile/>".to_vec(),
            Refused::Encoding(b"iso-8859-1".to_vec()),
        ),
        ("refused-xml-after-root.kdbx", after, Refused::AfterRoot),
        (
            "refused-xml-too-deep.kdbx",
            skeleton(&deep, ""),
            Refused::TooDeep,
        ),
        (
            "refused-xml-base64.kdbx",
            entry("<String><Key>Password</Key><Value Protected=\"True\">AA</Value></String>"),
            Refused::Base64("a protected value"),
        ),
        (
            "refused-xml-not-utf8.kdbx",
            vec![0xFF, 0xFE],
            Refused::NotUtf8,
        ),
        (
            "refused-xml-root.kdbx",
            b"<a><b></b></a>".to_vec(),
            Refused::Structure("the root element is not KeePassFile"),
        ),
    ];
    let mut v = vec![];
    for (i, (name, text, why)) in xmls.into_iter().enumerate() {
        v.push((name, with_document(&text, 60 + i as u8), Error::Xml(why)));
    }
    v
}

/// A small vault whose bytes are changed one at a time for the damage
/// vectors.
fn vault_for_damage() -> Vec<u8> {
    sealed(
        &document(0x77, "Damage", &[], false),
        PASSPHRASE,
        SIZES[0],
        50,
    )
}

/// The vault whose block stream ends early: all of it but its empty last
/// block.
pub fn cut_stream() -> Vec<u8> {
    let good = vault_for_damage();
    good[..good.len() - 36].to_vec()
}

/// Every vector, as (file name, bytes).
pub fn vectors() -> Vec<(&'static str, Vec<u8>)> {
    let small = vec![
        (Body::Record(entry()), "", ""),
        (
            Body::Record(Record::new(kind::NOTE).with(field::NOTE, b"A note in the small vault.")),
            "Notes",
            "Note",
        ),
    ];
    let mut out = vec![
        (
            ALL,
            sealed(
                &document(1, "Everything", &every_kind(), true),
                PASSPHRASE,
                SIZES[1],
                1,
            ),
        ),
        (
            SMALL,
            sealed(
                &document(2, "Small", &small, false),
                PASSPHRASE,
                SIZES[0],
                2,
            ),
        ),
        (
            CODEX32,
            sealed(
                &document(3, "Shares", &codex32_items(), false),
                PASSPHRASE,
                SIZES[0],
                3,
            ),
        ),
        (
            NFD,
            sealed(
                &document(4, "Accents", &[], false),
                NFD_PASSPHRASE,
                SIZES[0],
                4,
            ),
        ),
        (SEALED, sealed_through_file()),
        (CUT, cut_stream()),
    ];
    out.extend(refused().into_iter().map(|(name, bytes, _)| (name, bytes)));
    out
}

fn main() {
    let dir = std::env::args().nth(1).expect("a directory");
    for (name, bytes) in vectors() {
        let path = std::path::Path::new(&dir).join(name);
        std::fs::write(&path, &bytes).expect("write");
        println!("{} {} bytes", path.display(), bytes.len());
    }
}
