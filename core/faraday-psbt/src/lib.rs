//! The domain crate `faraday-psbt`: Faraday's own code, copied from `main` in a later step.
//! Everything a crate of the domain may do is in `docs/spec/01-layers.md` section 2.

#![no_std]

// Placeholder until the domain copy: `main`'s `core/faraday-psbt/src/musig.rs`. One open MuSig2
// signing session and the secret nonces drawn for it; not `Clone`, wiped when it drops.
pub struct MusigSession(());
