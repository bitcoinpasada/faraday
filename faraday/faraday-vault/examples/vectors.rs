//! Writes the vault vectors (`docs/VAULT.md` §10) into a directory: a
//! 64 KiB vault with one passphrase and one with three, every slot
//! holding one record of every type.
//!
//! ```text
//! cargo run -p faraday-vault --example vectors -- tools/vectors/vault
//! ```
//!
//! Everything is fixed: the seed, the nonces, the passphrases and the
//! records, at the cheapest cost a reader accepts. The words are BIP-39's
//! all-`abandon` test phrase; no coin is behind anything here.
#![allow(dead_code)]

use faraday_crypto::work::Compute;
use faraday_vault::records::{self, field, kind};
use faraday_vault::{Contents, Cost, MIN_MEMORY_KIB, Record, SLOT_SIZES};

/// A token for the vectors' key work (`docs/COMPUTE.md` §3), counting
/// nothing. Cheap.
#[expect(
    clippy::disallowed_methods,
    reason = "a mint: a tool that draws nothing, and its test (docs/COMPUTE.md §3)"
)]
pub fn compute() -> Compute {
    Compute::new(|_| {})
}

/// The cost the vectors are written at.
pub const COST: Cost = Cost {
    memory_kib: MIN_MEMORY_KIB,
    passes: 1,
    lanes: 1,
};

/// The one-passphrase vault's file and passphrase.
pub const ONE: (&str, &[&str]) = ("vault-one.ofv", &["correct horse battery staple"]);
/// The three-passphrase vault's file and passphrases.
pub const THREE: (&str, &[&str]) = (
    "vault-three.ofv",
    &["first passphrase", "second passphrase", "third passphrase"],
);

/// One record of every type, labelled `label`.
pub fn contents(label: &str) -> Contents {
    let c = compute();
    let words = faraday_bip::bip39::Mnemonic::parse(
        &c,
        faraday_bip::bip39::Language::English,
        "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about",
    )
    .expect("the BIP-39 test phrase");
    let sheet = faraday_vault::layouts::sheet_payload(
        b"wpkh([73c5da0a/84h/0h/0h]xpub6CatWdiZiodmUeTDp8LT5or8nmbKNcuyvz7WyksVFkKB4RHwCD3XyuvPEbvqAQY3rAPshWcMLoP2fMFMKHPJ4ZeZXYVUhLv1VMrjPC7PW6V/<0;1>/*)",
        label.as_bytes(),
        b"Third key is with the lawyer.",
    )
    .expect("a small sheet");
    let mut amounts = Record::new(kind::AMOUNTS).with(1, &[0xa5; 32]);
    amounts.push(2, &10_000_000u64.to_le_bytes());
    Contents {
        records: vec![
            Record::new(kind::SLOT_LABEL).with(field::LABEL, label.as_bytes()),
            Record::new(kind::KEY)
                .with(field::KEY, &records::words_payload(&words))
                .with(field::KEY_LABEL, b"Test key")
                .with(field::KEY_FLAGS, &[records::NO_LOAD_AT_UNLOCK]),
            Record::new(kind::WALLET)
                .with(field::WALLET, b"wpkh([73c5da0a/84h/0h/0h]xpub6CatWdiZiodmUeTDp8LT5or8nmbKNcuyvz7WyksVFkKB4RHwCD3XyuvPEbvqAQY3rAPshWcMLoP2fMFMKHPJ4ZeZXYVUhLv1VMrjPC7PW6V/<0;1>/*)")
                .with(field::WALLET_NAME, b"Spending"),
            Record::new(kind::NOTE).with(field::NOTE, format!("A note in the {label} slot.").as_bytes()),
            Record::new(kind::SHEET).with(field::SHEET, &sheet),
            Record::new(kind::ENTRY)
                .with(field::TITLE, b"Email")
                .with(field::USERNAME, b"you@example.com")
                .with(field::PASSWORD, label.as_bytes())
                .with(field::URL, b"https://mail.example.com")
                .with(field::NOTES, b"Recovery codes are in the note.")
                .with(field::TOTP, b"otpauth://totp/Example:you?secret=JBSWY3DPEHPK3PXP"),
            Record::new(kind::GPG)
                .with(1, &[0x11; 32])
                .with(2, &1_790_000_000u32.to_le_bytes())
                .with(3, &[0x22; 32])
                .with(4, &1_790_000_000u32.to_le_bytes())
                .with(5, b"Test <test@example.com>")
                .with(6, &0u32.to_le_bytes()),
            Record::new(kind::SECURE_BOOT).with(1, &[0x33; 16]),
            Record::new(kind::ROUND)
                .with(1, &[1])
                .with(2, &[0x44; 32])
                .with(3, &[0x73, 0xc5, 0xda, 0x0a])
                .with(4, &[0x55; 32]),
            amounts,
            Record::new(kind::WALLET_SHARE)
                .with(field::SHARE, b"wsh(sortedmulti(2,[73c5da0a/48h/0h/0h/2h]xpub...,...))")
                .with(field::SHARE_WALLET_NAME, b"Savings")
                .with(field::SHARE_NUMBER, &[1])
                .with(field::SHARE_OF, &[3])
                .with(field::SHARE_WALLET, b"wsh(sortedmulti(2,...))"),
            Record::new(kind::PASSPHRASE)
                .with(field::PASSPHRASE_KEY, &[0x9a, 0x6a, 0x25, 0x80])
                .with(field::PASSPHRASE, b"TREZOR"),
        ],
    }
}

/// The labels of the three-passphrase vault's slots, in passphrase order.
pub const LABELS: [&str; 3] = ["Main", "Second", "Decoy"];

/// A vault's bytes.
pub fn vault(passphrases: &[&str], labels: &[&str], seed: u8) -> Vec<u8> {
    let c = compute();
    let p: Vec<&[u8]> = passphrases.iter().map(|s| s.as_bytes()).collect();
    let all: Vec<Contents> = labels.iter().map(|l| contents(l)).collect();
    faraday_vault::create(&c, SLOT_SIZES[0], COST, &p, &all, &[seed; 32]).expect("the vectors")
}

/// Both vectors, as (file name, bytes).
pub fn vectors() -> Vec<(&'static str, Vec<u8>)> {
    vec![
        (ONE.0, vault(ONE.1, &["Main"], 1)),
        (THREE.0, vault(THREE.1, &LABELS, 3)),
    ]
}

fn main() {
    let dir = std::env::args().nth(1).expect("a directory");
    for (name, bytes) in vectors() {
        let path = std::path::Path::new(&dir).join(name);
        std::fs::write(&path, &bytes).expect("write");
        println!("{} {} bytes", path.display(), bytes.len());
    }
}
