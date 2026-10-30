//! `faraday_bip::address::parse` and `parses`: an address as it is scanned
//! or typed, on each of the four networks, silent payment addresses
//! included.

#![no_main]

use faraday_bip::address;
use faraday_bip::keys::Network;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(text) = core::str::from_utf8(data) else {
        return;
    };
    let _ = address::parses(text);
    for network in Network::ALL {
        let _ = address::parse(text, network);
    }
});
