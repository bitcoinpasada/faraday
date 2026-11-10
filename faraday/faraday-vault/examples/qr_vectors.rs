//! Writes the QR vault's vectors (`docs/VAULT.md` §10) into a directory:
//! OpenPGP messages, each one this build writes and `tools/vault/open.py`
//! reads with Sequoia's `sq decrypt`.
//!
//! ```text
//! cargo run --locked -p faraday-vault --example qr_vectors -- tools/vectors/vault/qr
//! ```
//!
//! - `qr-seed-12.pgp`, `qr-seed-24.pgp`, `qr-codex32.pgp`,
//!   `qr-passphrase.pgp`, `qr-note.pgp`: one vault of each item kind.
//! - `qr-empty.pgp`: a vault with no item (§2.1).
//! - `qr-nfd.pgp`: made with "café" in NFD, which opens under its NFC form
//!   (§3).
//! - `refused-*.pgp`: a message of §1.2's list that Faraday does not open,
//!   each with the refusal [`refused`] names: a SKESK v4 and a SEIPD v1, a
//!   second SKESK, a compressed data packet, data after the SEIPD packet.
//!
//! Every file is 396 bytes. Everything is fixed: the random bytes and the
//! passphrases, at 64 MiB (memory exponent 16, the least a seal writes), 3
//! passes and 4 lanes, so a rerun writes the same bytes. The words are
//! BIP-39's test phrases; no coin is behind anything here. The encrypted
//! part of the SKESK v4 and SEIPD v1 file is filler: it has the packets
//! GnuPG writes and is refused before anything is decrypted.
#![allow(dead_code)]

use faraday_bip::bip39::{Language, Mnemonic};
use faraday_crypto::work::Compute;
use faraday_pgp::message::{self, RANDOM_LEN, Unsupported};
use faraday_vault::contents::QR_LEN;
use faraday_vault::layouts::words_payload;
use faraday_vault::records::{field, kind};
use faraday_vault::{Record, qr};

/// A token for the vectors' key work (`docs/COMPUTE.md` §3), counting
/// nothing. Cheap.
#[expect(
    clippy::disallowed_methods,
    reason = "a mint: a tool that draws nothing, and its test (docs/COMPUTE.md §3)"
)]
pub fn compute() -> Compute {
    Compute::new(|_| {})
}

/// The memory exponent of every file: 64 MiB, the least a seal writes.
pub const EXPONENT: u8 = 16;
/// The length of every file (§1.2).
pub const LEN: usize = 396;

/// The passphrase of every vector but the NFD one.
pub const PASSPHRASE: &str = "correct horse battery staple";
/// The NFD vector's passphrase, "café" with its e and the combining acute
/// apart (U+0065 U+0301).
pub const NFD_PASSPHRASE: &str = "cafe\u{301}";
/// [`NFD_PASSPHRASE`] in NFC (U+00E9), which opens the same file.
pub const NFD_AS_NFC: &str = "caf\u{e9}";

/// A 12-word seed.
pub const SEED_12: &str = "qr-seed-12.pgp";
/// A 24-word seed.
pub const SEED_24: &str = "qr-seed-24.pgp";
/// A codex32 share and the fingerprint of its key.
pub const CODEX32: &str = "qr-codex32.pgp";
/// A passphrase and the fingerprint of its key.
pub const PASSPHRASE_ITEM: &str = "qr-passphrase.pgp";
/// A note.
pub const NOTE: &str = "qr-note.pgp";
/// No item.
pub const EMPTY: &str = "qr-empty.pgp";
/// A note, made under a passphrase in NFD.
pub const NFD: &str = "qr-nfd.pgp";

/// The 12-word test phrase.
pub const TWELVE: &str =
    "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
/// The 24-word test phrase.
pub const TWENTY_FOUR: &str = "zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo vote";
/// BIP 93 vector 2's share.
pub const SHARE: &str = "ms12namecacdefghjklmnpqrstuvwxyz023ftr2gdzmpy6pn";
/// The fingerprint of the key BIP 93 vector 2's shares make (the master
/// key of its secret `d1808e096b35b209ca12132b264662a5`).
pub const SHARE_FINGERPRINT: [u8; 4] = [0xfa, 0xb6, 0x86, 0x8a];
/// The fingerprint of the key the passphrase vector names: the key the
/// 12-word test phrase makes with the passphrase `TREZOR` (its master
/// key; BIP-39's TREZOR vector seed `c55257c3...`).
pub const FINGERPRINT: [u8; 4] = [0xb4, 0xe3, 0xf5, 0xed];
/// The note vector's text.
pub const NOTE_TEXT: &str = "The safe's code is in the drawer.";
/// The passphrase vector's passphrase.
pub const PASSPHRASE_TEXT: &str = "TREZOR";

/// 95 bytes counted from `seed`: the random bytes of one seal.
pub fn random(seed: u8) -> [u8; RANDOM_LEN] {
    std::array::from_fn(|i| (i as u8).wrapping_mul(11).wrapping_add(seed))
}

fn words(phrase: &str) -> Vec<u8> {
    let m = Mnemonic::parse(&compute(), Language::English, phrase).expect("a BIP-39 test phrase");
    words_payload(&m).to_vec()
}

/// The item of each vector, by file name.
pub fn items() -> Vec<(&'static str, Record)> {
    vec![
        (
            SEED_12,
            Record::new(kind::KEY).with(field::KEY, &words(TWELVE)),
        ),
        (
            SEED_24,
            Record::new(kind::KEY).with(field::KEY, &words(TWENTY_FOUR)),
        ),
        (
            CODEX32,
            Record::new(kind::CODEX32_SHARE)
                .with(field::CODEX32_STRING, SHARE.as_bytes())
                .with(field::CODEX32_KEY, &SHARE_FINGERPRINT),
        ),
        (
            PASSPHRASE_ITEM,
            Record::new(kind::PASSPHRASE)
                .with(field::PASSPHRASE_KEY, &FINGERPRINT)
                .with(field::PASSPHRASE, PASSPHRASE_TEXT.as_bytes()),
        ),
        (
            NOTE,
            Record::new(kind::NOTE).with(field::NOTE, NOTE_TEXT.as_bytes()),
        ),
        (EMPTY, Record::new(kind::NOTE).with(field::NOTE, b"")),
        (
            NFD,
            Record::new(kind::NOTE).with(field::NOTE, "caf\u{e9}".as_bytes()),
        ),
    ]
}

/// A QR TOML of `len` bytes holding a note: a message whose plaintext is
/// shorter than a QR vault's, so that a refused file is 396 bytes too.
fn short_plain(len: usize) -> Vec<u8> {
    let mut b = b"note = \"short\"\n".to_vec();
    b.resize(len, b'\n');
    b
}

/// A packet's header, in the new format, for a body of `len` bytes.
fn header(tag: u8, len: usize) -> Vec<u8> {
    let tag = 0xC0 | tag;
    if len < 192 {
        vec![tag, len as u8]
    } else {
        let n = len - 192;
        vec![tag, 192 + (n >> 8) as u8, n as u8]
    }
}

/// The valid files, with the passphrase of each.
pub fn vectors() -> Vec<(&'static str, Vec<u8>)> {
    items()
        .into_iter()
        .enumerate()
        .map(|(i, (name, record))| {
            let passphrase = if name == NFD {
                NFD_PASSPHRASE
            } else {
                PASSPHRASE
            };
            let bytes = qr::seal(
                &compute(),
                passphrase,
                EXPONENT,
                &random(i as u8 + 1),
                &record,
            )
            .expect("a QR vault");
            (name, bytes)
        })
        .collect()
}

/// A SKESK v4 and a SEIPD v1.
pub const SKESK4_SEIPD1: &str = "refused-skesk4-seipd1.pgp";
/// Two SKESK packets.
pub const SECOND_SKESK: &str = "refused-second-skesk.pgp";
/// A compressed data packet where the SEIPD goes.
pub const COMPRESSED: &str = "refused-compressed.pgp";
/// A packet that is not padding after the SEIPD.
pub const TRAILING: &str = "refused-trailing.pgp";

/// The refused files, each with the refusal `message::parse` gives it.
pub fn refused() -> Vec<(&'static str, Vec<u8>, Unsupported)> {
    let c = compute();
    let sealed = |plain: &[u8]| {
        message::seal(&c, PASSPHRASE.as_bytes(), EXPONENT, &random(40), plain).expect("a message")
    };
    // A SKESK v4 (iterated and salted S2K, SHA-256) and a SEIPD v1 whose
    // encrypted part is filler, as GnuPG writes the packets.
    let mut v4 = header(3, 13);
    v4.extend([4, 9, 3, 8]);
    v4.extend([0x5a; 8]);
    v4.push(0xff);
    let body = LEN - v4.len() - 3;
    v4.extend(header(18, body));
    v4.push(1);
    v4.extend((0..body - 1).map(|i| (i as u8).wrapping_mul(5).wrapping_add(3)));
    // The SKESK packet repeated before the data packet.
    let one = sealed(&short_plain(QR_LEN - 89));
    let mut second = one[..90].to_vec();
    second.extend(&one);
    // The data packet a compressed one ("uncompressed" algorithm 0, a
    // literal data packet inside), in the SEIPD's place.
    let mut compressed = sealed(&short_plain(QR_LEN))[..90].to_vec();
    let body = LEN - 90 - 3;
    compressed.extend(header(8, body));
    compressed.push(0);
    let inner = body - 1;
    compressed.extend(header(11, inner - 3));
    compressed.extend([b'b', 0, 0, 0, 0, 0]);
    compressed.extend(short_plain(inner - 3 - 6));
    // A packet that is not padding after the SEIPD packet.
    let mut trailing = sealed(&short_plain(QR_LEN - 2));
    trailing.extend([0xC0, 0x00]);
    vec![
        (SKESK4_SEIPD1, v4, Unsupported::Skesk4),
        (SECOND_SKESK, second, Unsupported::SecondSkesk),
        (COMPRESSED, compressed, Unsupported::Compressed),
        (TRAILING, trailing, Unsupported::Trailing),
    ]
}

/// Every file to write: the valid ones, then the refused.
pub fn files() -> Vec<(&'static str, Vec<u8>)> {
    let mut all = vectors();
    all.extend(refused().into_iter().map(|(n, b, _)| (n, b)));
    all
}

fn main() {
    let dir = std::env::args().nth(1).expect("a directory");
    for (name, bytes) in files() {
        let path = std::path::Path::new(&dir).join(name);
        std::fs::write(&path, &bytes).expect("write");
        println!("{} {} bytes", path.display(), bytes.len());
    }
}
