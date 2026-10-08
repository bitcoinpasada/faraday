//! `osk_psbt::Psbt::parse_bytes` / `parse_base64` and `osk_psbt::inspect`.
//!
//! A PSBT arrives from a coordinator that is assumed hostile, and
//! `inspect` walks every input, output, key origin and script in it to
//! build the Sign screens. It is run twice: with no keys loaded, and
//! with the BIP-39 test vector key, which is the path that re-derives
//! origins and verifies change.

#![no_main]

use std::sync::OnceLock;

use libfuzzer_sys::fuzz_target;
use osk_bip::bip39::{Language, Mnemonic};
use osk_bip::keys::{MasterKey, Network};
use osk_psbt::{Context, KeyRef, Psbt, inspect};

/// The BIP-39 test vector seed, as a key reference. Deriving it costs
/// six hardened account derivations, so it is built once.
fn abandon() -> &'static KeyRef {
    static KEY: OnceLock<KeyRef> = OnceLock::new();
    KEY.get_or_init(|| {
        let words = "abandon abandon abandon abandon abandon abandon abandon abandon abandon \
                     abandon abandon about";
        let m = Mnemonic::parse(Language::English, words).expect("test vector mnemonic");
        let master =
            MasterKey::from_seed(&m.to_seed(b"").expect("ascii passphrase"), Network::Testnet);
        KeyRef::from_master(&master, 0).expect("test vector accounts")
    })
}

fn run(psbt: &Psbt) {
    let empty = Context {
        network: Network::Testnet,
        keys: &[],
        wallets: &[],
        musig_session: None,
        shares: &[],
        carry: None,
    };
    let _ = inspect(psbt, &empty);

    let keys = core::slice::from_ref(abandon());
    let loaded = Context {
        network: Network::Testnet,
        keys,
        wallets: &[],
        musig_session: None,
        shares: &[],
        carry: None,
    };
    let _ = inspect(psbt, &loaded);
}

fuzz_target!(|data: &[u8]| {
    if let Ok(psbt) = Psbt::parse_bytes(data) {
        run(&psbt);
    }
    if let Ok(text) = core::str::from_utf8(data)
        && let Ok(psbt) = Psbt::parse_base64(text)
    {
        run(&psbt);
    }
});
