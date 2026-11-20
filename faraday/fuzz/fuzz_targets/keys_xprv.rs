//! An extended private key or a hex master seed taken in as text
//! (`docs/KEY-FORMS.md` §6): any text on either network. It never panics,
//! and a key that is read is a key the session holds, a refused one leaves
//! none. A key that reads loads again with its own fingerprint written
//! before it, is refused with a fingerprint one bit off and with a path
//! after it; the seeds in the corpus are the keys that reach these checks.

#![no_main]

use faraday_bip::keys::Network;
use faraday_crypto::work::Compute;
use faraday_keys::Keys;
use faraday_keys::xprv::{XprvError, XprvRefusal, parse_hex_seed};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    let c = Compute::unchecked();
    for network in [Network::Mainnet, Network::Testnet] {
        let mut keys = Keys::on(network);
        let read = keys.add_xprv(&c, text, "");
        assert_eq!(keys.is_empty(), read.is_err());
        // A key that reads names its fingerprint: written before the key it
        // loads, one bit off it is another key's, and with a path after it
        // it is refused as a path.
        if let Ok(fp) = read
            && !text.trim().starts_with('[')
        {
            let [a, b, c1, d] = fp.0;
            let own = format!("[{a:02x}{b:02x}{c1:02x}{d:02x}]{text}");
            let other = format!("[{:02x}{b:02x}{c1:02x}{d:02x}]{text}", a ^ 1);
            let path = format!("[{a:02x}{b:02x}{c1:02x}{d:02x}/0h]{text}");
            assert!(Keys::on(network).add_xprv(&c, &own, "").is_ok());
            assert!(matches!(
                Keys::on(network).add_xprv(&c, &other, ""),
                Err(XprvError::Read(XprvRefusal::OriginDiffers { .. }))
            ));
            assert!(matches!(
                Keys::on(network).add_xprv(&c, &path, ""),
                Err(XprvError::Read(XprvRefusal::OriginPath))
            ));
        }
    }
    if let Ok(seed) = parse_hex_seed(text) {
        assert!((16..=64).contains(&seed.len()));
        let mut keys = Keys::on(Network::Mainnet);
        let _ = keys.add_seed(&c, &seed, "");
    }
});
