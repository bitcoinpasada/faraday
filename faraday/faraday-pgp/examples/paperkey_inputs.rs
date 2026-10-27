//! Writes the Faraday key that `tools/vectors/paperkey/make.sh` hands to
//! paperkey: its transferable secret key, as `gpg --export-secret-keys`
//! writes one with no passphrase, into the directory given.
//!
//! ```text
//! cargo run -p faraday-pgp --example paperkey_inputs -- DIR
//! ```
//!
//! The key is `tests/paperkey.rs`'s: fixed seeds with leading zero bytes
//! (a shorter secret MPI), made at 1,780,000,000 with one user ID and a
//! two-year term. It guards nothing.

use faraday_pgp::Key;

fn main() {
    let dir = std::env::args().nth(1).expect("a directory to write into");
    let mut key = Key {
        primary_seed: [0x11; 32],
        created: 1_780_000_000,
        subkey_seed: [0x22; 32],
        subkey_created: 1_780_000_000,
        user_ids: vec!["Test Person <test@example.com>".to_string()],
        expiry: 2 * 31_536_000,
    };
    key.primary_seed[0] = 0;
    key.subkey_seed[0] = 0;
    key.subkey_seed[1] = 0;
    let path = std::path::Path::new(&dir).join("faraday-key.gpg");
    std::fs::write(&path, &*key.secret_key(key.created)).expect("written");
}
