//! `faraday-intake`'s wallet reader over any text (`docs/INTEROP.md` §5):
//! a single-key file, a Caravan config or an Electrum wallet file is
//! another wallet's file, whose every byte its writer chose. The JSON
//! they are read with is fuzzed on its own in the root `fuzz/` (`json`).

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    let c = osk_crypto::work::Compute::unchecked();
    let _ = faraday_intake::wallet::read_wallet(&c, text);
});
