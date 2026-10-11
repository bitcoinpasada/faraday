//! `osk_psbt::message::verify`: a signed message as a wallet hands it
//! over, checked against an address the device already holds.
//!
//! The addresses are the four single-signature script types of the
//! BIP-39 test vector key, built once. The input's first line is the
//! signature and the rest is the message, so one corpus entry carries
//! both.

#![no_main]

use std::str::FromStr;
use std::sync::OnceLock;

use libfuzzer_sys::fuzz_target;
use osk_bip::bip39::{Language, Mnemonic};
use osk_bip::keys::{MasterKey, Network, ScriptType};
use osk_psbt::bitcoin::Address;
use osk_psbt::message;

/// The first receive address of each script type of the test vector key.
fn addresses() -> &'static [Address] {
    static ADDRESSES: OnceLock<Vec<Address>> = OnceLock::new();
    ADDRESSES.get_or_init(|| {
        let words = "abandon abandon abandon abandon abandon abandon abandon abandon abandon \
                     abandon abandon about";
        let m = Mnemonic::parse(Language::English, words).expect("test vector mnemonic");
        let master =
            MasterKey::from_seed(&m.to_seed(b"").expect("ascii passphrase"), Network::Mainnet);
        let mut out: Vec<Address> = ScriptType::ALL
            .iter()
            .map(|&script_type| {
                master
                    .account_xpub(script_type, 0)
                    .expect("test vector account")
                    .address(false, 0)
                    .expect("test vector address")
            })
            .collect();
        // The BIP-137 vector's own address, so the corpus entry that
        // carries a real signature has something to check against.
        out.push(
            Address::from_str("bc1qcr8te4kr609gcawutmrza0j4xv80jy8z306fyu")
                .expect("vector address")
                .assume_checked(),
        );
        out
    })
}

fuzz_target!(|data: &[u8]| {
    let Ok(text) = core::str::from_utf8(data) else {
        return;
    };
    let (signature, body) = text.split_once('\n').unwrap_or((text, ""));
    for address in addresses() {
        let _ = message::verify(address, body, signature);
        let _ = message::verify(address, signature, body);
    }
});
