//! `faraday_pgp::read_secret_at` (VAULT §7, `gpg`): the armoured secret key
//! of a GPG entry in a vault file, which is online input. Whatever the
//! bytes are, a key that reads is exactly what `armor_secret` writes for
//! it, and a key made from the bytes reads back as it was made.

#![no_main]

use faraday_pgp::{Key, armor_secret, read_secret_at};
use libfuzzer_sys::fuzz_target;

fn same(a: &Key, b: &Key) -> bool {
    a.primary_seed == b.primary_seed
        && a.created == b.created
        && a.subkey_seed == b.subkey_seed
        && a.subkey_created == b.subkey_created
        && a.user_ids == b.user_ids
        && a.expiry == b.expiry
}

fuzz_target!(|data: &[u8]| {
    // The bytes as a file: when it reads, writing it again gives a block
    // that reads as the same key.
    if let Some((key, at)) = read_secret_at(data) {
        let (again, again_at) = read_secret_at(&armor_secret(&key, at)).expect("reads again");
        assert!(same(&key, &again) && at == again_at);
    }
    // A key Faraday writes reads back, whatever the user ID.
    if data.len() >= 72 {
        let int = |at: usize| u32::from_be_bytes(data[at..at + 4].try_into().unwrap());
        let key = Key {
            primary_seed: data[..32].try_into().unwrap(),
            created: int(64),
            subkey_seed: data[32..64].try_into().unwrap(),
            subkey_created: int(68),
            user_ids: vec![String::from_utf8_lossy(&data[72..]).into_owned()],
            expiry: int(64) ^ int(68),
        };
        let at = int(68);
        let (back, back_at) = read_secret_at(&armor_secret(&key, at)).expect("reads back");
        assert!(same(&key, &back) && at == back_at);
    }
});
