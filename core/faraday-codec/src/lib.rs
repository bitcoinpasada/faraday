//! QR payloads:
//!
//! - [`bip21`]: a `bitcoin:` payment URI read to BIP-21.
//! - [`bolt11`]: a BOLT #11 Lightning invoice read, its signature checked.
//! - [`bolt12`]: a BOLT #12 offer read.
//! - [`png`]: a QR code as a PNG file, for a public code saved as a
//!   picture.
//! - [`qr`]: an in-house QR encoder (no published one is `no_std`) whose
//!   module grid zeroizes on drop, because a SeedQR is the seed.
//! - [`seedqr`]: SeedQR (word indices as zero-padded digits, numeric
//!   mode) and CompactSeedQR (raw entropy bytes) to and from a
//!   [`Mnemonic`](faraday_bip::bip39::Mnemonic).
//! - [`classify`]: what a scanned payload is, so the application can
//!   route it: SeedQR, PSBT, BC-UR, address, descriptor, xpub, words.
//! - [`ur`]: BC-UR `crypto-psbt` and `bytes`, single-part and animated
//!   multi-part, over an in-house fountain codec (§16.61).
//! - [`kef`]: Krux's encrypted code as bytes and as text, and whether a
//!   payload is one; `classify` does not ask it.
//! - [`vault_file`]: the signature and the lengths that make a file a vault,
//!   whatever its name.
//! - [`decode`] (feature `decode`, on by default): QR detection and
//!   decoding from an 8-bit luma frame through `rqrr`. This is the one
//!   place in the core that needs `std`, recorded in
//!   `docs/deps/rqrr.md`.
//!
//! Secrets: SeedQR payloads live in fixed buffers inside [`Secret`]
//! wrappers, the encoder's scratch space zeroizes, and the resulting
//! [`qr::QrMatrix`] zeroizes on drop. A code read from a frame
//! ([`decode::Decoded`]) and a UR's parts being put together
//! ([`ur::Decoder`]) are wiped when dropped: what a camera reads may be a
//! seed. Everything else here (PSBTs, URs, addresses, keys) is public
//! data.
//!
//! [`Secret`]: faraday_crypto::Secret

#![cfg_attr(not(feature = "decode"), no_std)]

extern crate alloc;

pub mod bip21;
pub mod bolt11;
pub mod bolt12;
mod bolt12_chars;
pub mod cbor;
pub mod classify;
#[cfg(feature = "decode")]
pub mod decode;
pub mod encodings;
pub mod kef;
pub mod png;
pub mod qr;
pub mod seedqr;
pub mod ur;
pub mod vault_file;

/// The largest file read from a stick other than a vault, which has its own
/// limit ([`vault_file::MAX_FILE_LEN`]). A file other than a vault is read
/// whole and travels in one frame to the app (the disk process's frame is
/// this and a megabyte for a name), where the app holds it with the frame's
/// own copy; this bounds that memory. A PSBT or a wallet is kilobytes, a
/// picture of a QR code a few megabytes. It lives here because the app and
/// the disk process both need it, and the app does not link the disk
/// process's picture decoder.
pub const MAX_READ: u64 = 18 * 1024 * 1024;

pub use classify::{PayloadKind, classify};
#[cfg(feature = "decode")]
pub use decode::{Decoded, decode_luma};
pub use qr::{Ecc, Payload, QrMatrix};
