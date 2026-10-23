//! A threshold carry file (`.osk`) from a stick: `Carry::parse`, then the
//! PSBT it carries read and inspected with the file's secret section as
//! the context's carry, which is what Sign does with one.

#![no_main]

use faraday_bip::keys::Network;
use faraday_crypto::work::Compute;
use faraday_psbt::threshold::Carry;
use faraday_psbt::{Context, Psbt, inspect};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let c = Compute::unchecked();
    if let Ok(carry) = Carry::parse(data) {
        let _ = carry.to_bytes();
        if let Ok(psbt) = Psbt::parse_bytes(&c, &carry.psbt) {
            let ctx = Context {
                network: Network::Testnet,
                keys: &[],
                wallets: &[],
                musig_session: None,
                shares: &[],
                carry: Some(&carry.section),
            };
            let _ = inspect(&c, &psbt, &ctx);
        }
    }
});
