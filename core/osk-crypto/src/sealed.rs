//! In-memory sealing (`docs/PLANNING.md` §5.1, §5.2, §16.21).
//!
//! A [`Sealed<T>`] is `T` encrypted under a [`SessionKey`] with
//! ChaCha20-Poly1305. The plaintext exists only inside
//! [`Sealed::with`], in a stack buffer that is zeroized when the closure
//! returns; nothing escapes but the closure's result. A wrong key, or a
//! ciphertext that was changed, fails authentication and yields no
//! plaintext at all.
//!
//! **Nonces without a random number generator.** The crate has no RNG
//! (§4.2: the shell supplies entropy, the core never asks the OS). Each
//! seal uses the 96-bit nonce `counter ‖ prefix`: a 32-bit big-endian
//! count of seals under this key, followed by 64 bits derived once per key
//! as `HMAC-SHA512(key, "osk-sealed-nonce")[..8]`. Uniqueness is what the
//! AEAD needs, and the counter never repeats within a key's lifetime
//! (sealing refuses after 2³² − 1 seals, a number no session approaches);
//! a rotated key starts a new counter under a new prefix, and the old
//! key's ciphertexts are re-sealed under the new one, so `(key, nonce)`
//! pairs never repeat. The prefix adds nothing against a compromised key
//! and is not meant to; it keeps nonces from being predictable constants
//! and separates two keys' nonce spaces by more than the counter.
//!
//! **Weak keys.** Until the shell answers `Command::RequestEntropy`, the
//! application only has what it can gather itself (tick timestamps, the
//! display parameters). A key built from that is marked weak and refuses
//! to seal: a secret sealed under a guessable key is a secret in the
//! clear with extra steps. The application keeps such secrets in
//! plaintext `Secret`s, as it did before sealing existed, and seals them
//! the moment real entropy arrives. The weak mix is *not* a substitute for
//! the shell's entropy; it exists so that the application can start, draw
//! and derive the non-secret values it needs (a PIN pad scramble seed)
//! before the first entropy event.

use core::marker::PhantomData;

use chacha20poly1305::aead::inout::InOutBuf;
use chacha20poly1305::{AeadInOut, ChaCha20Poly1305, KeyInit, Nonce, Tag};
use zeroize::{Zeroize, ZeroizeOnDrop};

use crate::hash::hmac_sha512;
use crate::pinned::Pinned;
use crate::secret::Secret;

/// Session key length in bytes.
pub const KEY_LEN: usize = 32;
/// AEAD nonce length in bytes.
pub const NONCE_LEN: usize = 12;
/// Authentication tag length in bytes.
pub const TAG_LEN: usize = 16;
/// Most words a [`MnemonicBytes`] holds (BIP-39's maximum).
pub const MAX_MNEMONIC_WORDS: usize = 24;

/// Domain separation for the per-key nonce prefix.
const NONCE_LABEL: &[u8] = b"osk-sealed-nonce";
/// Domain separation for a key mixed from weak material.
const WEAK_LABEL: &[u8] = b"osk-session-key-weak";
/// Domain separation for the libsecp256k1 context blind.
const SECP_BLIND_LABEL: &[u8] = b"osk-secp-blind";
/// Associated data bound to every seal, so that a future format change
/// cannot be confused with this one.
const AAD: &[u8] = b"osk-sealed-v1";

/// Why a seal or unseal failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SealError {
    /// The session key was built without shell entropy; sealing waits.
    WeakKey,
    /// The key has sealed 2³² − 1 times and must be rotated first.
    Exhausted,
    /// The tag did not verify: wrong key, or a changed ciphertext.
    Authentication,
}

impl core::fmt::Display for SealError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(match self {
            SealError::WeakKey => "session key has no shell entropy yet",
            SealError::Exhausted => "session key exhausted its nonces",
            SealError::Authentication => "sealed data did not authenticate",
        })
    }
}

impl core::error::Error for SealError {}

/// The key every [`Sealed`] value of a session is encrypted under
/// (`docs/PLANNING.md` §5.2). Created from 32 bytes of shell entropy, or
/// weakly from whatever the application has before that; rotated with
/// fresh entropy on lock. Zeroized on drop, never printed, cloned or
/// compared.
///
/// The key is assembled inside its own pinned page and exists nowhere
/// else: a constructor takes the entropy it is mixed from, not the key,
/// and no copy of the key is made on the way in (security review M3).
pub struct SessionKey {
    /// Pinned rather than merely zeroized: it is held for the whole
    /// session and everything else the session keeps secret is sealed
    /// under it, so a copy of this key in swap would open the seed, the
    /// words and the PIN wherever their ciphertext ended up (§16.49).
    key: Pinned<[u8; KEY_LEN]>,
    prefix: [u8; 8],
    counter: u32,
    weak: bool,
}

impl SessionKey {
    /// A key from 32 bytes of entropy the shell supplied.
    pub fn from_entropy(entropy: &[u8; KEY_LEN]) -> Self {
        Self::build(|key| key.copy_from_slice(entropy), false)
    }

    /// A key mixed from `material` the application gathered itself
    /// (timestamps, display parameters). It is marked weak: it will not
    /// seal anything, and [`rotate`](Self::rotate) replaces it as soon as
    /// real entropy arrives. See the module documentation.
    pub fn weak(material: &[u8]) -> Self {
        Self::build(
            |key| {
                let mut mixed = hmac_sha512(WEAK_LABEL, material);
                key.copy_from_slice(&mixed[..KEY_LEN]);
                mixed.zeroize();
            },
            true,
        )
    }

    /// The one place a key is made. `fill` writes the 32 bytes straight
    /// into the pinned page, so the key itself never exists on the
    /// stack: what the caller holds is the entropy it was mixed from,
    /// which is the caller's to wipe.
    fn build(fill: impl FnOnce(&mut [u8; KEY_LEN]), weak: bool) -> Self {
        let key = Pinned::new_with(fill);
        let mut prefix = [0u8; 8];
        let mut derived = key.with(|k| hmac_sha512(k, NONCE_LABEL));
        prefix.copy_from_slice(&derived[..8]);
        derived.zeroize();
        SessionKey {
            key,
            prefix,
            counter: 0,
            weak,
        }
    }

    /// Replaces the key with one built from `fresh` entropy: the old key
    /// is zeroized, the nonce counter restarts under a new prefix, and
    /// the key is no longer weak. Everything sealed under the old key
    /// must be [re-sealed](Sealed::reseal) before this is called.
    pub fn rotate(&mut self, fresh: &[u8; KEY_LEN]) {
        let mut next = Self::from_entropy(fresh);
        core::mem::swap(self, &mut next);
        // `next` now holds the old key and zeroizes on drop.
    }

    /// Whether the key was built without shell entropy.
    pub fn is_weak(&self) -> bool {
        self.weak
    }

    /// How many values were sealed under this key so far.
    pub fn seals(&self) -> u32 {
        self.counter
    }

    /// 64 bytes derived from the key for `label` (HMAC-SHA512). For salts
    /// and seeds that must be secret per session but are not themselves
    /// sealed material.
    pub fn derive(&self, label: &[u8]) -> Secret<[u8; 64]> {
        Secret::new(hmac_sha512(self.key.expose(), label))
    }

    /// The 32 bytes a libsecp256k1 context is randomized with, so that
    /// the intermediate values of a signature are blinded by something
    /// an attacker cannot predict (security review M1). Derived like any
    /// other per-session value, so it is new after every rotation and
    /// tells nothing about the key.
    ///
    /// It is a blinding factor, not key material: it is copied out
    /// because the context keeps its own copy anyway, and knowing it
    /// opens nothing.
    pub fn secp_blind(&self) -> [u8; 32] {
        let derived = self.derive(SECP_BLIND_LABEL);
        let mut blind = [0u8; 32];
        blind.copy_from_slice(&derived.expose()[..32]);
        blind
    }

    /// The next nonce: `counter ‖ prefix`. See the module documentation.
    fn next_nonce(&mut self) -> Result<[u8; NONCE_LEN], SealError> {
        if self.weak {
            return Err(SealError::WeakKey);
        }
        let n = self.counter.checked_add(1).ok_or(SealError::Exhausted)?;
        self.counter = n;
        let mut nonce = [0u8; NONCE_LEN];
        nonce[..4].copy_from_slice(&n.to_be_bytes());
        nonce[4..].copy_from_slice(&self.prefix);
        Ok(nonce)
    }

    fn cipher(&self) -> ChaCha20Poly1305 {
        ChaCha20Poly1305::new_from_slice(self.key.expose()).expect("32-byte key")
    }
}

impl Zeroize for SessionKey {
    fn zeroize(&mut self) {
        self.key.zeroize();
        self.prefix.zeroize();
        self.counter = 0;
        self.weak = true;
    }
}

impl Drop for SessionKey {
    fn drop(&mut self) {
        self.zeroize();
    }
}

impl ZeroizeOnDrop for SessionKey {}

/// A fixed-size value that can be sealed: it serialises to and from a
/// fixed byte buffer, and both the value and the buffer zeroize.
///
/// Implemented for the 64-byte BIP-39 seed and for [`MnemonicBytes`].
/// Sealing is generic over this trait rather than over `AsRef<[u8]>` so
/// that the plaintext buffer inside [`Sealed::with`] has a size known at
/// compile time and lives on the stack (`docs/PLANNING.md` §5.3).
pub trait SealedBytes: Zeroize + Sized {
    /// The fixed-size byte form, `[u8; N]`.
    type Bytes: AsRef<[u8]> + AsMut<[u8]> + Zeroize;
    /// An all-zero buffer, the starting point of every serialisation.
    const ZEROED: Self::Bytes;
    /// Writes the value into `out`.
    fn write_bytes(&self, out: &mut Self::Bytes);
    /// Rebuilds the value from `bytes`.
    fn from_bytes(bytes: &Self::Bytes) -> Self;
}

impl SealedBytes for [u8; 64] {
    type Bytes = [u8; 64];
    const ZEROED: [u8; 64] = [0; 64];

    fn write_bytes(&self, out: &mut [u8; 64]) {
        out.copy_from_slice(self);
    }

    fn from_bytes(bytes: &[u8; 64]) -> Self {
        *bytes
    }
}

/// The longest BIP-32 seed, which is BIP-39's.
pub const MAX_SEED_LEN: usize = 64;

/// A BIP-32 seed and its length: 64 bytes from BIP-39 words, or the 16
/// or 32 bytes of a SLIP-39 master secret, which is the seed itself
/// (`docs/PLANNING.md` §16.107). One sealed type carries both, so a
/// loaded key has one seed door whatever it was made from. Zeroized on
/// drop.
pub struct SeedBytes {
    buf: [u8; MAX_SEED_LEN],
    len: u8,
}

/// The seed's bytes, then its length.
const SEED_BYTES: usize = MAX_SEED_LEN + 1;

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

impl SealedBytes for SeedBytes {
    type Bytes = [u8; SEED_BYTES];
    const ZEROED: [u8; SEED_BYTES] = [0; SEED_BYTES];

    fn write_bytes(&self, out: &mut [u8; SEED_BYTES]) {
        out[..MAX_SEED_LEN].copy_from_slice(&self.buf);
        out[MAX_SEED_LEN] = self.len;
    }

    fn from_bytes(bytes: &[u8; SEED_BYTES]) -> Self {
        let mut buf = [0u8; MAX_SEED_LEN];
        buf.copy_from_slice(&bytes[..MAX_SEED_LEN]);
        SeedBytes {
            buf,
            len: bytes[MAX_SEED_LEN].min(MAX_SEED_LEN as u8),
        }
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

/// A mnemonic in sealable form: up to 24 word indices, the word count and
/// a language tag the application assigns (`docs/PLANNING.md` §16.2:
/// words are `u16` indices, never text). Zeroized on drop.
pub struct MnemonicBytes {
    /// Word indices, `0..2048`; only the first `len` are meaningful.
    pub words: [u16; MAX_MNEMONIC_WORDS],
    /// Word count, 12–24.
    pub len: u8,
    /// Wordlist tag, as the application numbers its languages.
    pub language: u8,
}

/// `words` as little-endian pairs, then `len`, then `language`.
const MNEMONIC_BYTES: usize = MAX_MNEMONIC_WORDS * 2 + 2;

impl SealedBytes for MnemonicBytes {
    type Bytes = [u8; MNEMONIC_BYTES];
    const ZEROED: [u8; MNEMONIC_BYTES] = [0; MNEMONIC_BYTES];

    fn write_bytes(&self, out: &mut [u8; MNEMONIC_BYTES]) {
        for (i, w) in self.words.iter().enumerate() {
            out[2 * i..2 * i + 2].copy_from_slice(&w.to_le_bytes());
        }
        out[MNEMONIC_BYTES - 2] = self.len;
        out[MNEMONIC_BYTES - 1] = self.language;
    }

    fn from_bytes(bytes: &[u8; MNEMONIC_BYTES]) -> Self {
        let mut words = [0u16; MAX_MNEMONIC_WORDS];
        for (i, w) in words.iter_mut().enumerate() {
            *w = u16::from_le_bytes([bytes[2 * i], bytes[2 * i + 1]]);
        }
        MnemonicBytes {
            words,
            len: bytes[MNEMONIC_BYTES - 2],
            language: bytes[MNEMONIC_BYTES - 1],
        }
    }
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

/// `T` encrypted under a [`SessionKey`]: ciphertext, nonce and tag. The
/// plaintext is reachable only through [`with`](Self::with). Implements
/// none of `Debug`, `Clone` or `PartialEq`.
pub struct Sealed<T: SealedBytes> {
    ciphertext: T::Bytes,
    nonce: [u8; NONCE_LEN],
    tag: [u8; TAG_LEN],
    plain: PhantomData<T>,
}

impl<T: SealedBytes> Sealed<T> {
    /// Seals `value` under `key`, consuming and zeroizing it. Fails with
    /// [`SealError::WeakKey`] while the key has no shell entropy; the
    /// value is zeroized either way.
    pub fn seal(key: &mut SessionKey, mut value: T) -> Result<Self, SealError> {
        let mut buf = T::ZEROED;
        value.write_bytes(&mut buf);
        value.zeroize();
        let sealed = Self::seal_bytes(key, &mut buf);
        buf.zeroize();
        sealed
    }

    /// Encrypts `buf` in place and wraps it. `buf` holds ciphertext
    /// afterwards, whatever the outcome (plaintext only until the first
    /// error check, and the caller zeroizes it regardless).
    fn seal_bytes(key: &mut SessionKey, buf: &mut T::Bytes) -> Result<Self, SealError> {
        let nonce = key.next_nonce()?;
        let tag = key
            .cipher()
            .encrypt_inout_detached(&Nonce::from(nonce), AAD, InOutBuf::from(buf.as_mut()))
            .map_err(|_| SealError::Authentication)?;
        let mut ciphertext = T::ZEROED;
        ciphertext.as_mut().copy_from_slice(buf.as_ref());
        Ok(Sealed {
            ciphertext,
            nonce,
            tag: tag.0,
            plain: PhantomData,
        })
    }

    /// Decrypts into a stack buffer, runs `f` on the value, and zeroizes
    /// both before returning `f`'s result. A wrong key or a modified
    /// ciphertext is [`SealError::Authentication`] and `f` never runs.
    pub fn with<R>(&self, key: &SessionKey, f: impl FnOnce(&T) -> R) -> Result<R, SealError> {
        let mut buf = T::ZEROED;
        buf.as_mut().copy_from_slice(self.ciphertext.as_ref());
        let opened = key.cipher().decrypt_inout_detached(
            &Nonce::from(self.nonce),
            AAD,
            InOutBuf::from(buf.as_mut()),
            &Tag::from(self.tag),
        );
        if opened.is_err() {
            buf.zeroize();
            return Err(SealError::Authentication);
        }
        let mut value = T::from_bytes(&buf);
        buf.zeroize();
        let result = f(&value);
        value.zeroize();
        Ok(result)
    }

    /// The same value sealed under `new` instead of `old`, for key
    /// rotation. The plaintext lives in one stack buffer between the two
    /// operations and is zeroized after.
    pub fn reseal(&self, old: &SessionKey, new: &mut SessionKey) -> Result<Self, SealError> {
        let mut buf = T::ZEROED;
        buf.as_mut().copy_from_slice(self.ciphertext.as_ref());
        let opened = old.cipher().decrypt_inout_detached(
            &Nonce::from(self.nonce),
            AAD,
            InOutBuf::from(buf.as_mut()),
            &Tag::from(self.tag),
        );
        if opened.is_err() {
            buf.zeroize();
            return Err(SealError::Authentication);
        }
        let sealed = Self::seal_bytes(new, &mut buf);
        buf.zeroize();
        sealed
    }

    /// The nonce this value was sealed with. Public data, for tests of
    /// the nonce scheme.
    pub fn nonce(&self) -> &[u8; NONCE_LEN] {
        &self.nonce
    }
}

impl<T: SealedBytes> Zeroize for Sealed<T> {
    fn zeroize(&mut self) {
        self.ciphertext.zeroize();
        self.nonce.zeroize();
        self.tag.zeroize();
    }
}

impl<T: SealedBytes> Drop for Sealed<T> {
    fn drop(&mut self) {
        self.zeroize();
    }
}

impl<T: SealedBytes> ZeroizeOnDrop for Sealed<T> {}

#[cfg(test)]
mod tests {
    use super::*;
    use core::sync::atomic::{AtomicUsize, Ordering};

    fn key(byte: u8) -> SessionKey {
        SessionKey::from_entropy(&[byte; KEY_LEN])
    }

    fn seed() -> [u8; 64] {
        let mut s = [0u8; 64];
        for (i, b) in s.iter_mut().enumerate() {
            *b = i as u8 ^ 0xA5;
        }
        s
    }

    #[test]
    fn seed_round_trips() {
        let mut k = key(1);
        let sealed = Sealed::seal(&mut k, seed()).unwrap();
        assert_ne!(
            sealed.ciphertext,
            seed(),
            "ciphertext differs from plaintext"
        );
        let out = sealed.with(&k, |s| *s).unwrap();
        assert_eq!(out, seed());
        assert_eq!(k.seals(), 1);
    }

    #[test]
    fn mnemonic_bytes_round_trip() {
        let mut k = key(2);
        let mut words = [0u16; MAX_MNEMONIC_WORDS];
        words[0] = 2047;
        words[11] = 3;
        words[23] = 1024;
        let m = MnemonicBytes {
            words,
            len: 24,
            language: 7,
        };
        let sealed = Sealed::seal(&mut k, m).unwrap();
        let (w, len, lang) = sealed.with(&k, |m| (m.words, m.len, m.language)).unwrap();
        assert_eq!(w, words);
        assert_eq!((len, lang), (24, 7));
    }

    #[test]
    fn wrong_key_and_tampering_are_rejected() {
        let mut k = key(3);
        let mut sealed = Sealed::seal(&mut k, seed()).unwrap();
        let other = key(4);
        assert_eq!(
            sealed.with(&other, |_| ()).unwrap_err(),
            SealError::Authentication
        );
        sealed.ciphertext[10] ^= 1;
        assert_eq!(
            sealed.with(&k, |_| ()).unwrap_err(),
            SealError::Authentication
        );
        sealed.ciphertext[10] ^= 1;
        sealed.tag[0] ^= 1;
        assert_eq!(
            sealed.with(&k, |_| ()).unwrap_err(),
            SealError::Authentication
        );
        sealed.tag[0] ^= 1;
        sealed.nonce[0] ^= 1;
        assert_eq!(
            sealed.with(&k, |_| ()).unwrap_err(),
            SealError::Authentication
        );
        sealed.nonce[0] ^= 1;
        assert!(sealed.with(&k, |_| ()).is_ok(), "restored");
    }

    #[test]
    fn nonces_are_unique_per_seal_and_per_key() {
        let mut k = key(5);
        let a = Sealed::seal(&mut k, seed()).unwrap();
        let b = Sealed::seal(&mut k, seed()).unwrap();
        assert_ne!(a.nonce(), b.nonce());
        assert_eq!(a.nonce()[..4], 1u32.to_be_bytes());
        assert_eq!(b.nonce()[..4], 2u32.to_be_bytes());
        assert_eq!(a.nonce()[4..], b.nonce()[4..], "same prefix under one key");
        // Same plaintext, different nonce: different ciphertext.
        assert_ne!(a.ciphertext, b.ciphertext);
        let mut k2 = key(6);
        let c = Sealed::seal(&mut k2, seed()).unwrap();
        assert_ne!(a.nonce()[4..], c.nonce()[4..], "prefix differs per key");
        assert_eq!(c.nonce()[..4], 1u32.to_be_bytes());
    }

    #[test]
    fn rotation_reseals_under_the_new_key_only() {
        let mut k = key(7);
        let sealed = Sealed::seal(&mut k, seed()).unwrap();
        let mut fresh = key(8);
        let resealed = sealed.reseal(&k, &mut fresh).unwrap();
        assert_eq!(resealed.with(&fresh, |s| *s).unwrap(), seed());
        assert!(resealed.with(&k, |_| ()).is_err());
        k.rotate(&[8; KEY_LEN]);
        assert_eq!(k.seals(), 0);
        assert!(!k.is_weak());
        assert!(sealed.with(&k, |_| ()).is_err(), "old ciphertext is dead");
        assert_eq!(resealed.with(&k, |s| *s).unwrap(), seed());
    }

    #[test]
    fn weak_keys_refuse_to_seal_until_rotated() {
        let mut k = SessionKey::weak(b"display 640x480 tick 0");
        assert!(k.is_weak());
        assert!(matches!(
            Sealed::seal(&mut k, seed()).err(),
            Some(SealError::WeakKey)
        ));
        // Derived values are still available (a scramble seed).
        let a = k.derive(b"scramble");
        let b = SessionKey::weak(b"display 640x480 tick 0").derive(b"scramble");
        assert!(a == b, "weak keys are deterministic in their material");
        k.rotate(&[9; KEY_LEN]);
        assert!(!k.is_weak());
        assert!(Sealed::seal(&mut k, seed()).is_ok());
        assert!(k.derive(b"scramble") != a);
    }

    #[test]
    fn exhaustion_is_refused() {
        let mut k = key(10);
        k.counter = u32::MAX;
        assert!(matches!(
            Sealed::seal(&mut k, seed()).err(),
            Some(SealError::Exhausted)
        ));
    }

    /// A value whose `zeroize` reports itself, to observe that the
    /// plaintext built inside `with` is wiped after the closure.
    struct Counted([u8; 64]);

    static ZEROIZED: AtomicUsize = AtomicUsize::new(0);

    impl Zeroize for Counted {
        fn zeroize(&mut self) {
            self.0.zeroize();
            ZEROIZED.fetch_add(1, Ordering::SeqCst);
        }
    }

    impl SealedBytes for Counted {
        type Bytes = [u8; 64];
        const ZEROED: [u8; 64] = [0; 64];
        fn write_bytes(&self, out: &mut [u8; 64]) {
            *out = self.0;
        }
        fn from_bytes(bytes: &[u8; 64]) -> Self {
            Counted(*bytes)
        }
    }

    #[test]
    fn plaintext_is_zeroized_after_seal_and_with() {
        fn assert_zeroize<T: Zeroize>() {}
        assert_zeroize::<Sealed<[u8; 64]>>();
        assert_zeroize::<SessionKey>();
        assert_zeroize::<MnemonicBytes>();
        let mut k = key(11);
        let before = ZEROIZED.load(Ordering::SeqCst);
        let sealed = Sealed::seal(&mut k, Counted(seed())).unwrap();
        assert_eq!(ZEROIZED.load(Ordering::SeqCst), before + 1, "input wiped");
        let first = sealed.with(&k, |c| c.0[0]).unwrap();
        assert_eq!(first, seed()[0]);
        assert_eq!(
            ZEROIZED.load(Ordering::SeqCst),
            before + 2,
            "closure value wiped"
        );
    }
}
