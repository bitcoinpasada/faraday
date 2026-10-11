//! The generator RSA key generation draws from: a ChaCha20 keystream
//! keyed by 32 bytes the caller takes from the system's entropy. Nothing
//! here draws randomness of its own.

use chacha20::ChaCha20;
use chacha20::cipher::{KeyIvInit, StreamCipher};
use core::convert::Infallible;
use rand_core::{TryCryptoRng, TryRng};

/// A ChaCha20 keystream as a generator.
pub struct Stream(ChaCha20);

impl Stream {
    /// The keystream of `seed`, under the nonce `what` names: one seed
    /// gives a different stream for each purpose.
    pub fn new(seed: &[u8; 32], what: u8) -> Stream {
        let mut nonce = [0u8; 12];
        nonce[0] = what;
        Stream(ChaCha20::new_from_slices(seed, &nonce).expect("a 32-byte key and a 12-byte nonce"))
    }
}

impl TryRng for Stream {
    type Error = Infallible;

    fn try_next_u32(&mut self) -> Result<u32, Infallible> {
        let mut b = [0u8; 4];
        self.0.apply_keystream(&mut b);
        Ok(u32::from_le_bytes(b))
    }

    fn try_next_u64(&mut self) -> Result<u64, Infallible> {
        let mut b = [0u8; 8];
        self.0.apply_keystream(&mut b);
        Ok(u64::from_le_bytes(b))
    }

    fn try_fill_bytes(&mut self, dst: &mut [u8]) -> Result<(), Infallible> {
        dst.fill(0);
        self.0.apply_keystream(dst);
        Ok(())
    }
}

impl TryCryptoRng for Stream {}
