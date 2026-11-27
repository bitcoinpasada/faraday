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

    // Placeholder until the domain copy: `main`'s `Work` (`work.rs`).
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub struct Work(());

    // Spec: 05 §5.2 gives `Hooks`; `main`'s `Compute::new` takes `count` alone.
    pub struct Hooks {
        pub count: fn(Work),
        pub stop: fn() -> bool,
        pub progress: fn(u32, u32),
    }

    impl Compute {
        pub fn new(_hooks: Hooks) -> Compute {
            todo!()
        }

        /// The job was asked to stop.
        pub fn stop_asked(&self) -> bool {
            todo!()
        }

        pub fn progress(&self, _done: u32, _of: u32) {
            todo!()
        }
    }
}
