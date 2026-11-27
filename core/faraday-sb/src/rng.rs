//! The generator RSA key generation draws from: `chacha20`'s
//! `ChaCha20Rng`, seeded with 32 bytes the caller takes from the system's
//! entropy. Nothing here draws randomness of its own.
//!
//! `ChaCha20Rng::from_seed` takes the seed by value, so a copy of it is
//! left in the stack frames of that call. The generator's own state wipes
//! itself on drop (chacha20's `zeroize` feature). The seed is copied and the
//! generator made in an inner function; [`stream`] overwrites a 2 KiB stack
//! region below its own frame after that returns, where the inner frames
//! were. This is best effort: the compiler does not promise where the copy
//! was. The generator is returned by value, so the moves of its state that
//! returning makes are not overwritten.

use chacha20::ChaCha20Rng;
use rand_core::SeedableRng;
use zeroize::Zeroize;

/// The generator the key making draws from.
pub type Stream = ChaCha20Rng;

/// The generator of `seed` on the stream `what` names: one seed gives a
/// different stream for each purpose.
pub fn stream(seed: &[u8; 32], what: u8) -> Stream {
    let mut rng = seeded(seed);
    scrub_stack();
    rng.set_stream(u64::from(what));
    rng
}

/// Makes the generator, so the by-value copy of `seed` is in this frame,
/// below the caller's, where the scrub reaches.
#[inline(never)]
fn seeded(seed: &[u8; 32]) -> Stream {
    ChaCha20Rng::from_seed(*seed)
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
