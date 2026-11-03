//! The generator RSA key generation draws from: `chacha20`'s
//! `ChaCha20Rng`, seeded with 32 bytes the caller takes from the system's
//! entropy. Nothing here draws randomness of its own.

use chacha20::ChaCha20Rng;
use rand_core::SeedableRng;

/// The generator the key making draws from.
pub type Stream = ChaCha20Rng;

/// The generator of `seed` on the stream `what` names: one seed gives a
/// different stream for each purpose.
pub fn stream(seed: &[u8; 32], what: u8) -> Stream {
    let mut rng = ChaCha20Rng::from_seed(*seed);
    rng.set_stream(u64::from(what));
    rng
}
