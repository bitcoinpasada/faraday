//! The domain crate `faraday-keys`: Faraday's own code, copied from `main` in a later step.
//! Everything a crate of the domain may do is in `docs/spec/01-layers.md` section 2.

// Placeholder until the domain copy: `main`'s `faraday/faraday-keys/src/session.rs`.
pub struct Keys(());

// Placeholder until the domain copy: 09 §2.1 says `faraday-keys` gains `Refusal`; `main` has
// several refusals in its modules.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {}

pub mod vault {
    // Placeholder until the domain copy: `main`'s `faraday/faraday-keys/src/vault/mod.rs`.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct VaultId(u32);

    // Placeholder until the domain copy: `main`'s `faraday/faraday-keys/src/vault/mod.rs`.
    pub struct OpenVaults(());
}
