//! A count of the costly work the core does, by kind, for an application
//! that budgets it.
//!
//! Each costly function in the core calls [`count`] on its first line: a
//! PSBT parsed, inspected or signed, a QR code read from pixels, a key
//! stretched, a canvas blurred or copied whole. With the `count` feature
//! off (the default, and every `no_std` build) [`count`] is empty and
//! compiles to nothing. With it on, the application installs one counter
//! for the process ([`set_counter`]) and keeps the counts itself: per
//! thread, beside its own kinds, so that a test can say that drawing a
//! frame twice did none it should not.

/// A kind of costly work the core does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Work {
    /// A PSBT parsed from its bytes or its base64.
    PsbtParse,
    /// A PSBT inspected: every input's script, key origins, signatures
    /// and amounts read against the wallets and keys given.
    PsbtInspect,
    /// A PSBT signed (single key, MuSig2 or threshold share), or a
    /// message signed.
    Sign,
    /// QR codes looked for in a picture's pixels.
    QrDecode,
    /// A PNG's pixels decoded.
    PngDecode,
    /// A key stretched: PBKDF2, scrypt or Argon2.
    Kdf,
    /// A canvas reduced and blurred (a sheet's frosted backdrop).
    Blur,
    /// A whole canvas copied: a snapshot taken, restored or blended.
    FullCanvas,
}

#[cfg(feature = "count")]
static COUNTER: std::sync::OnceLock<fn(Work)> = std::sync::OnceLock::new();

/// Installs `f` as the process's counter, once: every later call keeps
/// the first. With the `count` feature off it does nothing.
pub fn set_counter(f: fn(Work)) {
    #[cfg(feature = "count")]
    {
        let _ = COUNTER.set(f);
    }
    #[cfg(not(feature = "count"))]
    let _ = f;
}

/// Notes one piece of work of kind `w` with the installed counter.
/// Costs a load and a call with the `count` feature on, nothing with it
/// off.
#[inline]
pub fn count(w: Work) {
    #[cfg(feature = "count")]
    if let Some(f) = COUNTER.get() {
        f(w);
    }
    #[cfg(not(feature = "count"))]
    let _ = w;
}
