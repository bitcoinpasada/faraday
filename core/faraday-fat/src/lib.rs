//! The domain crate `faraday-fat`: Faraday's own code, copied from `main` in a later step.
//! Everything a crate of the domain may do is in `docs/spec/01-layers.md` section 2.

#![no_std]

// Placeholder until the domain copy: `main`'s `faraday/faraday-fat/src/lib.rs`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Wiped {
    Zeroed,
    NameOnly,
}

// Placeholder until the domain copy: `main`'s `faraday/faraday-fat/src/lib.rs`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Zeroed {
    pub next: u32,
    pub total: u32,
    pub done: bool,
}
