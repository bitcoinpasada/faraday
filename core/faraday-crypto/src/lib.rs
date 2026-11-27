//! The domain crate `faraday-crypto`: Faraday's own code, copied from `main` in a later step.
//! Everything a crate of the domain may do is in `docs/spec/01-layers.md` section 2.

#![no_std]

// Placeholder until the domain copy: `main`'s `core/faraday-crypto/src/secret.rs`.
pub struct Secret<T>(T);

pub mod work {
    use core::marker::PhantomData;

    // Placeholder until the domain copy: `main`'s `core/faraday-crypto/src/work.rs`. The token is
    // not `Send`, as on `main`.
    pub struct Compute(PhantomData<*const ()>);
}
