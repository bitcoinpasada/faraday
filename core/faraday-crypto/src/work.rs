//! The costly work the core does, by kind, and the token a costly
//! function takes (`docs/COMPUTE.md` §2).
//!
//! [`Compute`] is the token: an application mints one where it may do
//! costly work, and code that cannot reach one cannot call work that
//! needs it. Each costly function takes it first and notes its work
//! through it on its first line ([`Compute::count`]), with the counter
//! the token was minted with; the application keeps the counts itself,
//! per thread, beside its own kinds, so that a test can say that drawing
//! a frame twice did none it should not. A canvas's pixel work
//! ([`Work::Blur`], [`Work::FullCanvas`]) is drawing's own and takes no
//! token: the canvas notes it with the counter its owner sets on it.

use core::marker::PhantomData;

/// A kind of costly work the core does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Work {
    /// A PSBT parsed from its bytes or its base64.
    PsbtParse,
    /// A PSBT inspected: every input's script, key origins, signatures
    /// and amounts read against the wallets and keys given.
    PsbtInspect,
    /// A PSBT signed (single key or MuSig2), or a
    /// message signed.
    Sign,
    /// QR codes looked for in a picture's pixels.
    QrDecode,
    /// A QR code made: a payload, a SeedQR or a Compact SeedQR encoded.
    QrEncode,
    /// A whole file's or transaction's bytes hashed.
    FileHash,
    /// The first bytes of a file on a stick read, for a listing to tell
    /// a vault by its contents (`docs/VAULT.md` §1.1).
    FileRead,
    /// A PNG's pixels decoded.
    PngDecode,
    /// A key stretched: PBKDF2, scrypt or Argon2.
    Kdf,
    /// Seed words parsed: each word looked up in its list and the
    /// checksum hashed.
    WordsParse,
    /// A master key made (an HMAC-SHA-512 of the seed) and its
    /// fingerprint worked out (an elliptic-curve multiplication).
    MasterKey,
    /// A child key derived from a master key, or an extended public key
    /// made from a private one: elliptic-curve multiplications.
    KeyDerive,
    /// A wallet description read: a descriptor, a BIP-388 policy or a
    /// record parsed, its keys decoded and its shape checked.
    DescriptorParse,
    /// A wallet's script or address at an index derived: a derivation of
    /// each of its keys and the script built.
    Address,
    /// A canvas reduced and blurred (a sheet's frosted backdrop).
    Blur,
    /// A whole canvas copied: a snapshot taken, restored or blended.
    FullCanvas,
    /// A codex32 piece of work: a checksum, a worksheet column or an
    /// interpolation of strings (`faraday-bip`'s `codex32`).
    Codex32,
    /// A key's signing context randomized from a blind: an
    /// elliptic-curve multiplication.
    Blind,
}

/// The token a costly function takes: proof that its caller may do
/// costly work, carrying the counter that work is noted with
/// (`docs/COMPUTE.md` §2.1).
///
/// It is not `Clone`, `Copy`, `Default`, `Send` or `Sync`, so a token is
/// minted where it is used and cannot be kept in a `static`.
///
/// ```
/// use faraday_crypto::work::{Compute, Work};
/// use faraday_crypto::work::Hooks;
/// let c = Compute::new(Hooks { count: |_| {}, stop: || false, progress: |_, _| {} });
/// c.count(Work::Kdf);
/// ```
///
/// It cannot be copied,
///
/// ```compile_fail,E0599
/// use faraday_crypto::work::{Compute, Hooks};
/// let c = Compute::new(Hooks { count: |_| {}, stop: || false, progress: |_, _| {} });
/// let d = c.clone();
/// ```
///
/// sent to another thread,
///
/// ```compile_fail,E0277
/// use faraday_crypto::work::Compute;
/// fn send<T: Send>() {}
/// send::<Compute>();
/// ```
///
/// or made from nothing:
///
/// ```compile_fail,E0277
/// use faraday_crypto::work::Compute;
/// let c: Compute = Default::default();
/// ```
pub struct Compute {
    hooks: Hooks,
    // A raw pointer is neither `Send` nor `Sync`, so neither is the token.
    _local: PhantomData<*const ()>,
}

/// What a token reaches the runtime through: function pointers, so the
/// crate stays `no_std` and the token stays `!Send`.
#[derive(Clone, Copy)]
pub struct Hooks {
    /// Notes one piece of work of a kind.
    pub count: fn(Work),
    /// Whether the job running was asked to stop.
    pub stop: fn() -> bool,
    /// Tells how many pieces of how many are done.
    pub progress: fn(u32, u32),
}

impl Compute {
    /// A mint, noting work with `hooks.count`. An application calls this
    /// at its entry points only (`docs/COMPUTE.md` §3). Cheap.
    #[must_use]
    pub fn new(hooks: Hooks) -> Compute {
        Compute {
            hooks,
            _local: PhantomData,
        }
    }

    /// A token that counts nothing, for core's tests, fuzz
    /// targets and tools: code with no drawing guarantee to keep.
    /// Cheap.
    #[cfg(feature = "unchecked")]
    #[must_use]
    #[expect(
        clippy::disallowed_methods,
        reason = "the token that counts nothing is minted here, for tests and tools"
    )]
    pub fn unchecked() -> Compute {
        Compute::new(Hooks {
            count: |_| {},
            stop: || false,
            progress: |_, _| {},
        })
    }

    /// Notes one piece of work of kind `w`: a call through the pointer.
    /// Cheap.
    #[inline]
    pub fn count(&self, w: Work) {
        (self.hooks.count)(w);
    }

    /// Whether the job was asked to stop. Cheap.
    #[must_use]
    pub fn stop_asked(&self) -> bool {
        (self.hooks.stop)()
    }

    /// Tells that `done` of `of` pieces are done. Cheap.
    pub fn progress(&self, done: u32, of: u32) {
        (self.hooks.progress)(done, of);
    }
}
