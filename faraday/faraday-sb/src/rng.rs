//! The generator RSA key generation draws from: `chacha20`'s
//! `ChaCha20Rng`, seeded with 32 bytes the caller takes from the system's
//! entropy. Nothing here draws randomness of its own.
//!
//! `ChaCha20Rng::from_seed` takes the seed by value, so a copy of it is
//! left in the stack frames of that call. The generator's own state wipes
//! itself on drop (chacha20's `zeroize` feature). [`stream`] overwrites a
//! 2 KiB stack region after the generator is made. This is best effort: it
//! wipes the region where those frames were, and the compiler does not
//! promise the copy was there.

use chacha20::ChaCha20Rng;
use rand_core::SeedableRng;
use zeroize::Zeroize;

/// The generator the key making draws from.
pub type Stream = ChaCha20Rng;

/// The generator of `seed` on the stream `what` names: one seed gives a
/// different stream for each purpose.
pub fn stream(seed: &[u8; 32], what: u8) -> Stream {
    let mut rng = ChaCha20Rng::from_seed(*seed);
    scrub_stack();
    rng.set_stream(u64::from(what));
    rng
}

/// Overwrites the stack below the caller with zeros, where the by-value
/// seed's copies were. 2 KiB covers the frames of `from_seed` and what it
/// calls, with room over. Best effort, see the module doc.
#[inline(never)]
fn scrub_stack() {
    let mut s = [0u8; 2048];
    s.zeroize();
    core::hint::black_box(&s);
}
