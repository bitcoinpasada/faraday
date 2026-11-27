use subtle::ConstantTimeEq;
use zeroize::{Zeroize, ZeroizeOnDrop};

/// A value that is zeroized when dropped and cannot be printed, cloned or
/// compared in variable time.
///
/// `Secret` implements none of `Debug`, `Display`, `Clone`, `Copy` or any
/// serialisation trait, so a secret cannot leave through a formatting
/// macro or an accidental copy. Plaintext is reached only through
/// [`expose`](Self::expose), [`expose_mut`](Self::expose_mut) or
/// [`with`](Self::with), which keeps every access point greppable.
///
/// Equality is constant-time and available when `T: AsRef<[u8]>`.
pub struct Secret<T: Zeroize>(T);

impl<T: Zeroize> Secret<T> {
    /// Wraps `value`. The value is zeroized when the wrapper is dropped.
    #[inline]
    pub fn new(value: T) -> Self {
        Self(value)
    }

    /// Borrows the plaintext.
    #[inline]
    pub fn expose(&self) -> &T {
        &self.0
    }

    /// Mutably borrows the plaintext, for filling a buffer in place.
    #[inline]
    pub fn expose_mut(&mut self) -> &mut T {
        &mut self.0
    }

    /// Runs `f` on the plaintext and returns its result.
    ///
    /// Prefer this over [`expose`](Self::expose) when the caller only needs
    /// a derived, non-secret value: it keeps the plaintext borrow inside one
    /// expression.
    #[inline]
    pub fn with<R>(&self, f: impl FnOnce(&T) -> R) -> R {
        f(&self.0)
    }
}

impl<T: Zeroize> Drop for Secret<T> {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

impl<T: Zeroize> Zeroize for Secret<T> {
    fn zeroize(&mut self) {
        self.0.zeroize();
    }
}

impl<T: Zeroize> ZeroizeOnDrop for Secret<T> {}

impl<T: Zeroize + AsRef<[u8]>> PartialEq for Secret<T> {
    /// Constant-time comparison of the byte representations. Two secrets of
    /// different lengths compare unequal; the length itself is not hidden.
    fn eq(&self, other: &Self) -> bool {
        self.0.as_ref().ct_eq(other.0.as_ref()).into()
    }
}

impl<T: Zeroize + AsRef<[u8]>> Eq for Secret<T> {}

/// Most words a [`MnemonicBytes`] holds (BIP-39's maximum).
pub const MAX_MNEMONIC_WORDS: usize = 24;

/// The longest BIP-32 seed, which is BIP-39's.
pub const MAX_SEED_LEN: usize = 64;

/// A BIP-32 seed and its length: 64 bytes from BIP-39 words, or the 16
/// or 32 bytes of a SLIP-39 master secret, which is the seed itself.
/// One type carries both, so a
/// loaded key has one seed door whatever it was made from. Zeroized on
/// drop.
pub struct SeedBytes {
    buf: [u8; MAX_SEED_LEN],
    len: u8,
}

impl SeedBytes {
    /// A seed of `bytes`. Longer than [`MAX_SEED_LEN`] is refused.
    pub fn new(bytes: &[u8]) -> Option<Self> {
        if bytes.len() > MAX_SEED_LEN {
            return None;
        }
        let mut buf = [0u8; MAX_SEED_LEN];
        buf[..bytes.len()].copy_from_slice(bytes);
        Some(SeedBytes {
            buf,
            len: bytes.len() as u8,
        })
    }

    /// The seed.
    pub fn as_bytes(&self) -> &[u8] {
        &self.buf[..usize::from(self.len)]
    }
}

impl AsRef<[u8]> for SeedBytes {
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}

impl Zeroize for SeedBytes {
    fn zeroize(&mut self) {
        self.buf.zeroize();
        self.len = 0;
    }
}

impl Drop for SeedBytes {
    fn drop(&mut self) {
        self.zeroize();
    }
}

impl ZeroizeOnDrop for SeedBytes {}

/// A mnemonic in a fixed form: up to 24 word indices, the word count and
/// a language tag the application assigns. Zeroized on drop.
pub struct MnemonicBytes {
    /// Word indices, `0..2048`; only the first `len` are meaningful.
    pub words: [u16; MAX_MNEMONIC_WORDS],
    /// Word count, 12–24.
    pub len: u8,
    /// Wordlist tag, as the application numbers its languages.
    pub language: u8,
}

impl Zeroize for MnemonicBytes {
    fn zeroize(&mut self) {
        self.words.zeroize();
        self.len = 0;
        self.language = 0;
    }
}

impl Drop for MnemonicBytes {
    fn drop(&mut self) {
        self.zeroize();
    }
}

impl ZeroizeOnDrop for MnemonicBytes {}
