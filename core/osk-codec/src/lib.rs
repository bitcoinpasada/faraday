//! QR payloads for OpenSignerKit (`docs/PLANNING.md` §4.1, §7, §8.2 #2,
//! §8.4 #1/#12/#16; `docs/UX.md` §4 "Scan routing"):
//!
//! - [`png`]: a QR code as a PNG file, for a public code saved as a
//!   picture.
//! - [`qr`]: an in-house QR encoder (no published one is `no_std`) whose
//!   module grid zeroizes on drop, because a SeedQR is the seed.
//! - [`seedqr`]: SeedQR (word indices as zero-padded digits, numeric
//!   mode) and CompactSeedQR (raw entropy bytes) to and from a
//!   [`Mnemonic`](osk_bip::bip39::Mnemonic).
//! - [`classify`]: what a scanned payload is, so the application can
//!   route it: SeedQR, PSBT, BC-UR, address, descriptor, xpub, words.
//! - [`ur`]: BC-UR `crypto-psbt` and `bytes`, single-part and animated
//!   multi-part, over an in-house fountain codec (§16.61).
//! - [`decode`] (feature `decode`, on by default): QR detection and
//!   decoding from an 8-bit luma frame through `rqrr`. This is the one
//!   place in the core that needs `std`, recorded as a deviation in
//!   `docs/PLANNING.md` §16.20 and `docs/deps/rqrr.md`.
//!
//! Secrets: SeedQR payloads live in fixed buffers inside [`Secret`]
//! wrappers, the encoder's scratch space zeroizes, and the resulting
//! [`qr::QrMatrix`] zeroizes on drop. Everything else here (PSBTs, URs,
//! addresses, keys) is public data.
//!
//! [`Secret`]: osk_crypto::Secret

#![cfg_attr(not(feature = "decode"), no_std)]

extern crate alloc;

pub mod classify;
#[cfg(feature = "decode")]
pub mod decode;
pub mod encodings;
pub mod png;
pub mod qr;
pub mod seedqr;
pub mod ur;

pub use classify::{PayloadKind, classify};
#[cfg(feature = "decode")]
pub use decode::{Decoded, decode_luma};
pub use qr::{Ecc, Payload, QrMatrix};
