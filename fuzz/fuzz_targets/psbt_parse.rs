//! `faraday_psbt::Psbt::parse_bytes` / `parse_base64`, the version 2
//! reader `faraday_psbt::v2::PsbtV2::parse_bytes` / `parse_base64`, and
//! `faraday_psbt::inspect`.
//!
//! A PSBT arrives from a coordinator that is assumed hostile, and
//! `inspect` walks every input, output, key origin and script in it to
//! build the Sign screens. It is run twice: with no keys loaded, and
//! with the BIP-39 test vector key, which is the path that re-derives
//! origins and verifies change. A copy is then finalized, which is what
//! Sign does with a complete one.
//!
//! A version 2 PSBT is signed as its version 0 form (`to_v0`), so that
//! form is run the same way, and its signatures are taken back
//! (`merge_signed`); a version 0 PSBT is made version 2 (`from_v0`),
//! which must read back as version 2 of the same transaction.

#![no_main]

use std::sync::OnceLock;

use libfuzzer_sys::fuzz_target;
use faraday_bip::bip39::{Language, Mnemonic};
use faraday_bip::keys::{MasterKey, Network};
use faraday_crypto::work::Compute;
use faraday_psbt::v2::PsbtV2;
use faraday_psbt::{Context, KeyRef, Psbt, inspect};

/// The BIP-39 test vector seed, as a key reference. Deriving it costs
/// six hardened account derivations, so it is built once.
fn abandon() -> &'static KeyRef {
    static KEY: OnceLock<KeyRef> = OnceLock::new();
    KEY.get_or_init(|| {
        let words = "abandon abandon abandon abandon abandon abandon abandon abandon abandon \
                     abandon abandon about";
        let c = Compute::unchecked();
        let m = Mnemonic::parse(&c, Language::English, words).expect("test vector mnemonic");
        let master = MasterKey::from_seed(
            &c,
            &m.to_seed(&c, b"").expect("ascii passphrase"),
            Network::Testnet,
        );
        KeyRef::from_master(&c, &master, 0).expect("test vector accounts")
    })
}

fn run(psbt: &Psbt) {
    let c = Compute::unchecked();
    let empty = Context {
        network: Network::Testnet,
        keys: &[],
        wallets: &[],
        musig_session: None,
    };
    let _ = inspect(&c, psbt, &empty);

    let keys = core::slice::from_ref(abandon());
    let loaded = Context {
        network: Network::Testnet,
        keys,
        wallets: &[],
        musig_session: None,
    };
    let _ = inspect(&c, psbt, &loaded);

    let mut copy = psbt.clone();
    let _ = faraday_psbt::finalize(&mut copy);
}

/// A version 0 PSBT made version 2 reads back as version 2 of the same
/// transaction.
fn to_v2(psbt: &Psbt) {
    let c = Compute::unchecked();
    let v2 = PsbtV2::from_v0(&c, psbt);
    let read = PsbtV2::parse_bytes(&c, &v2.to_bytes()).expect("from_v0 writes a version 2 PSBT");
    if !psbt.unsigned_tx().input.is_empty() {
        assert_eq!(&read.unsigned_tx().expect("every output has its script"), psbt.unsigned_tx());
    }
}

fn run_v2(mut psbt: PsbtV2) {
    let c = Compute::unchecked();
    let _ = psbt.locktime();
    if let Ok(v0) = psbt.to_v0(&c) {
        run(&v0);
        let _ = psbt.merge_signed(&v0);
    }
}

fuzz_target!(|data: &[u8]| {
    let c = Compute::unchecked();
    if let Ok(psbt) = Psbt::parse_bytes(&c, data) {
        run(&psbt);
        to_v2(&psbt);
    }
    if let Ok(psbt) = PsbtV2::parse_bytes(&c, data) {
        run_v2(psbt);
    }
    if let Ok(text) = core::str::from_utf8(data) {
        if let Ok(psbt) = Psbt::parse_base64(&c, text) {
            run(&psbt);
        }
        if let Ok(psbt) = PsbtV2::parse_base64(&c, text) {
            run_v2(psbt);
        }
    }
});
