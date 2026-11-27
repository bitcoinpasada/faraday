//! What wipes (02 §6): the wiping payload types of an event or an effect.

use alloc::vec::Vec;
use core::fmt;

/// Bytes that may hold a secret: a file, a picture, a PDF, a code's payload, a camera frame.
/// Wipes on drop and is not `Clone` (P14).
pub type Bytes = faraday_crypto::Secret<Vec<u8>>;

/// Randomness. Wipes on drop and is not `Clone` (02 §6).
pub struct EntropyBytes(faraday_crypto::Secret<[u8; 32]>);

impl EntropyBytes {
    pub fn new(bytes: [u8; 32]) -> Self {
        Self(faraday_crypto::Secret::new(bytes))
    }

    /// Borrows the bytes: a type that wipes on drop cannot give them up by value.
    pub fn expose(&self) -> &[u8; 32] {
        self.0.expose()
    }
}

impl fmt::Debug for EntropyBytes {
    /// The length only (P16).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "EntropyBytes({} bytes)", self.0.expose().len())
    }
}

/// What `Ask::Entropy` is answered with (02 §4.7).
pub struct Entropy {
    pub bytes: EntropyBytes,
    pub source: EntropySource,
}

/// Which source gave the randomness, which New key words.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum EntropySource {
    DevRandom,
    Os,
}
