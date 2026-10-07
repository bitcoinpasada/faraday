//! `osk_keep::header`, `challenge` and `open`: the blob a
//! Tier B device keeps its keys in, which is a file on storage the
//! application does not control.
//!
//! `header` is run on the raw input, which is the real gate an arbitrary
//! file meets. `open` is then run on a blob-shaped buffer with the input
//! laid over a template written here, so a short input tampers with the
//! header of the records and a full-length one replaces everything. The mac is what a captured or hostile secure element
//! would answer.

#![no_main]

use std::sync::OnceLock;

use libfuzzer_sys::fuzz_target;
use osk_backup::Cost;
use osk_crypto::{MnemonicBytes, SealedBytes};
use osk_keep::notes::KeptNotes;
use osk_keep::{Kept, KeptKeys, KeptSecret};

/// The cost every blob this target writes and reads carries: the
/// cheapest Argon2id accepts, because 64 MiB per guess is a suite
/// nobody runs, let alone a fuzzer.
const CHEAP: Cost = Cost {
    memory_kib: 8,
    passes: 1,
    lanes: 1,
};

/// A blob written under a known PIN and a known element tag.
fn template() -> &'static [u8] {
    static BLOB: OnceLock<Vec<u8>> = OnceLock::new();
    BLOB.get_or_init(|| {
        let header = osk_keep::new_header(CHEAP, &[7u8; 32]);
        let guess = osk_keep::challenge(&header, b"1234").expect("cheap parameters");
        let mut keys = KeptKeys::new();
        keys.push(Kept {
            secret: KeptSecret::Words(MnemonicBytes::from_bytes(
                &<MnemonicBytes as SealedBytes>::ZEROED,
            )),
            backup_verified: true,
        });
        let wallets = osk_keep::wallets::body(&[], &osk_keep::names::WalletNames::new());
        let notes = KeptNotes::new();
        osk_keep::write(
            &header,
            &[9u8; 32],
            &guess,
            &keys,
            &wallets,
            &notes,
            &[11u8; 32],
        )
        .0
    })
}

fuzz_target!(|data: &[u8]| {
    let _ = osk_keep::header(data);

    let mut blob = template().to_vec();
    let n = data.len().min(blob.len());
    blob[..n].copy_from_slice(&data[..n]);

    let Some(header) = osk_keep::header(&blob) else {
        return;
    };
    let mut mac = [0u8; 32];
    let m = data.len().min(32);
    mac[..m].copy_from_slice(&data[..m]);

    for pin in [&b""[..], b"1234", data] {
        if let Some(guess) = osk_keep::challenge(&header, pin) {
            let _ = guess.challenge();
            let _ = osk_keep::open(&blob, &mac, &guess);
        }
    }
    let _ = osk_keep::count_attempt(&mut blob);
});
