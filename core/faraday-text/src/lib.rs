//! The domain crate `faraday-text`: Faraday's own code, copied from `main` in a later step.
//! Everything a crate of the domain may do is in `docs/spec/01-layers.md` section 2.

#![no_std]

// Placeholder until the domain copy: `main`'s `core/faraday-text/src/text.rs`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Text(());

// Placeholder until the domain copy: `main` has it in `faraday/faraday-intake/src/secret_buf.rs`,
// and 02 P5 names `faraday-text` for it.
pub struct SecretText(Text);
