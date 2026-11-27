//! The domain crate `faraday-bip`: Faraday's own code, copied from `main` in a later step.
//! Everything a crate of the domain may do is in `docs/spec/01-layers.md` section 2.

#![no_std]

pub mod keys {
    // Placeholder until the domain copy: `main`'s `core/faraday-bip/src/keys.rs`.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum Network {
        Mainnet,
        Testnet,
        Signet,
        Regtest,
    }

    // Placeholder until the domain copy: `main`'s `core/faraday-bip/src/keys.rs`.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct Fingerprint(pub [u8; 4]);
}

pub mod policy {
    // Placeholder until the domain copy: `main`'s `core/faraday-bip/src/policy.rs`.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct WalletPolicy(());
}
