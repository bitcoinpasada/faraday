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
