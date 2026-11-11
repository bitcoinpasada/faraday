//! One named costly operation, run once on a fixed public test vector and
//! then the process ends. `tools/op-cost.py` runs each under valgrind's
//! callgrind and holds the instruction count to `tools/op-cost.counts`
//! (F238). `op-cost --list` names them.
//!
//! Every input is a published test vector: the BIP-39 vectors' words and
//! their passphrase, Coldcard's Seed XOR example, and a PSBT built from
//! the all-"abandon" test seed on regtest. No key of a person is read.

use std::hint::black_box;
use std::process::ExitCode;

use faraday_bip::bip39::{Language, Mnemonic};
use faraday_bip::keys::{MasterKey, Network};
use faraday_crypto::Zeroizing;
use faraday_crypto::work::Compute;
use faraday_entropy::SeedXor;
use faraday_psbt::{Aux, Context, KeyRef, Nonce, Psbt, sign};

/// The BIP-39 test vector for 256 bits of zeros, 24 words.
const WORDS_24: &str = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon art";
/// That vector's passphrase.
const PASSPHRASE: &[u8] = b"TREZOR";

/// Coldcard's Seed XOR example of three 24-word parts and the seed they give.
const XOR_PARTS: [&str; 3] = [
    "romance wink lottery autumn shop bring dawn tongue range crater truth ability miss spice fitness easy legal release recall obey exchange recycle dragon room",
    "lion misery divide hurry latin fluid camp advance illegal lab pyramid unaware eager fringe sick camera series noodle toy crowd jeans select depth lounge",
    "vault nominee cradle silk own frown throw leg cactus recall talent worry gadget surface shy planet purpose coffee drip few seven term squeeze educate",
];
const XOR_SEED: &str = "silent toe meat possible chair blossom wait occur this worth option bag nurse find fish scene bench asthma bike wage world quit primary indoor";

/// The 12-word BIP-39 test vector the PSBT's inputs spend from.
const ABANDON: &str =
    "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

/// A regtest spend of three native SegWit outputs of `ABANDON`, each with
/// its previous transaction, to one recipient and one change output.
const PSBT: &str = include_str!("psbt.txt");

/// The operations, by name.
const OPERATIONS: [(&str, fn()); 5] = [
    ("mnemonic-parse-24", mnemonic_parse),
    ("mnemonic-to-seed", mnemonic_to_seed),
    ("seed-xor-combine", seed_xor_combine),
    ("psbt-read", psbt_read),
    ("psbt-sign", psbt_sign),
];

fn mnemonic_parse() {
    let c = Compute::unchecked();
    let m = Mnemonic::parse(&c, Language::English, WORDS_24).expect("vector words");
    black_box(m);
}

fn mnemonic_to_seed() {
    let c = Compute::unchecked();
    let m = Mnemonic::parse(&c, Language::English, WORDS_24).expect("vector words");
    let seed = m.to_seed(&c, PASSPHRASE).expect("ascii passphrase");
    black_box(seed.expose());
}

fn seed_xor_combine() {
    let c = Compute::unchecked();
    let mut xor = SeedXor::new();
    for part in XOR_PARTS {
        let m = Mnemonic::parse(&c, Language::English, part).expect("vector words");
        xor.push(m.entropy().expose().as_bytes())
            .expect("same length");
    }
    let entropy = xor.entropy().expect("three parts");
    let m = Mnemonic::from_entropy(Language::English, entropy.as_bytes()).expect("length");
    let want = Mnemonic::parse(&c, Language::English, XOR_SEED).expect("vector words");
    assert_eq!(m.indices(), want.indices());
}

fn psbt_read() {
    let c = Compute::unchecked();
    let psbt = Psbt::parse_base64(&c, PSBT.trim()).expect("vector psbt");
    black_box(psbt);
}

fn psbt_sign() {
    let c = Compute::unchecked();
    let net = Network::Regtest;
    let m = Mnemonic::parse(&c, Language::English, ABANDON).expect("vector words");
    let seed = m.to_seed(&c, b"").expect("ascii passphrase");
    let key = MasterKey::from_seed(&c, &seed, net);
    let keys = [KeyRef::from_master(&c, &key, 0).expect("key")];
    let ctx = Context {
        network: net,
        keys: &keys,
        wallets: &[],
        musig_session: None,
    };
    let mut psbt = Psbt::parse_base64(&c, PSBT.trim()).expect("vector psbt");
    let result = sign(
        &c,
        &mut psbt,
        &[&key],
        &[key.fingerprint()],
        &ctx,
        &[],
        Nonce::LowR,
        Aux::Deterministic,
        &mut None,
        Some(&Zeroizing::new([0u8; 32])),
    )
    .expect("vector psbt signs");
    assert!(result.complete);
}

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    match (args.next().as_deref(), args.next()) {
        (Some("--list"), None) => {
            for (name, _) in OPERATIONS {
                println!("{name}");
            }
            ExitCode::SUCCESS
        }
        (Some(name), None) => match OPERATIONS.iter().find(|(n, _)| *n == name) {
            Some((_, run)) => {
                run();
                ExitCode::SUCCESS
            }
            None => {
                eprintln!("op-cost: no operation {name}; --list names them");
                ExitCode::FAILURE
            }
        },
        _ => {
            eprintln!("usage: op-cost <name> | op-cost --list");
            ExitCode::FAILURE
        }
    }
}
