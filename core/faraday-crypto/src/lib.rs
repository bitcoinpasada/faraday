//! Secret wrapper types, in-memory sealing, and thin wrappers over vetted
//! hash primitives.
//!
//! This is the bottom crate of Faraday's core. It has three jobs:
//!
//! - [`Secret`]: hold a value that must be wiped from memory when it is no
//!   longer needed, and make accidental leaks (formatting, cloning,
//!   serialising) impossible by construction.
//! - [`Pinned`]: hold a long-lived secret on pages of its own that the
//!   kernel is asked not to write to swap. Its five small
//!   `unsafe` blocks — the page-aligned allocation, its release, the two
//!   dereferences and the `mlock`/`munlock` pair — are the crate's
//!   audited `unsafe` and the only `unsafe` in the core. The syscalls
//!   themselves are behind the `pin-pages` feature, which the desktop
//!   and Pi shells turn on and the browser and Android builds leave off.
//! - [`Sealed`] and [`SessionKey`]: keep a secret encrypted while it is
//!   not in use, so that plaintext exists only inside a closure and only
//!   for as long as the closure runs (§5.1, §5.2, §16.21).
//! - Hashes and KDFs: [`sha256`], [`hmac_sha256`], [`hmac_sha512`],
//!   [`pbkdf2_hmac_sha256`], [`pbkdf2_hmac_sha512`], [`blake2b`] and
//!   [`scrypt`] are the only entry points the rest of the core uses, so
//!   the choice of implementation (RustCrypto today, for the first
//!   five) is confined to this crate. The last two are written here:
//!   BLAKE2b and scrypt each have one caller, LND's aezeed, and each is
//!   small enough that a crate would cost more than it saves.
//!
//! - [`work`]: [`work::Compute`], the token a costly function takes,
//!   which carries the counter the application notes that work with
//!   (`docs/COMPUTE.md` §2), and the kinds of work ([`work::Work`]).
//!
//! The crate is `no_std` and allocates for one reason
//! only: to give a long-lived secret whole pages of its own, so that
//! they can be pinned. Nothing else here takes an allocation. It has no random number
//! generator: entropy is handed in by the shell through the session key.
//! Signing and curve operations are deliberately absent for now; they
//! arrive with the `secp256k1` decision.

#![no_std]

extern crate alloc;

mod blake2b;
mod hash;
mod pinned;
mod scrypt;
mod sealed;
mod secret;
pub mod work;

pub use blake2b::{MAX_OUT as BLAKE2B_MAX_OUT, blake2b};
pub use hash::{
    hmac_sha256, hmac_sha512, pbkdf2_hmac_sha256, pbkdf2_hmac_sha512, sha256, sha256_whole,
};
pub use pinned::{Pinned, pin_failure_to_report};
pub use scrypt::{Error as ScryptError, scrypt};
pub use sealed::{
    KEY_LEN, MAX_MNEMONIC_WORDS, MAX_SEED_LEN, MnemonicBytes, NONCE_LEN, SealError, Sealed,
    SealedBytes, SeedBytes, SessionKey, TAG_LEN,
};
pub use secret::Secret;
pub use zeroize::{Zeroize, ZeroizeOnDrop};
