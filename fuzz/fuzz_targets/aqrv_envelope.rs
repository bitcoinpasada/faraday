//! `aqrv::open` after the key derivation (the Argon QR Vault,
//! `https://github.com/bitcoinpasada/aqrv`, `AQRV.md` §6 Reading): the first 32 bytes of the input stand for
//! Argon2id's output and the rest is the envelope, so a run does not pay
//! for 64 MiB of Argon2id each time (`aqrv_open` does, once). The
//! envelope split, the keystream, every kind's tag and the item's layout
//! never panic, an opened envelope is of a kind its size holds and
//! recognised, and a refusal is a refusal, not a panic.

#![no_main]

use aqrv::{NotOpened, fuzz_open_with_prk, recognise};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Some((prk, envelope)) = data.split_first_chunk::<32>() else {
        return;
    };
    match fuzz_open_with_prk(envelope, prk) {
        Ok(opened) => {
            assert_eq!(recognise(envelope), Some(opened.header));
            let kind = opened.item.kind().expect("an item read has a kind");
            assert!(opened.header.size.holds(kind));
        }
        Err(NotOpened::NotRecognised) => assert!(recognise(envelope).is_none()),
        Err(_) => assert!(recognise(envelope).is_some()),
    }
});
