//! The Faraday vault (`docs/VAULT.md`): one KDBX 4 file under one
//! passphrase, which the app opens, changes and seals.
//!
//! - [`file`] is the vault as the app uses it: [`file::inspect`] checks a
//!   file's header and cost before anything is allocated, [`file::open`]
//!   opens it, [`file::seal`] writes it again and [`file::create`] makes a
//!   new one.
//! - [`kdbx`] is the KDBX 4 container and its items: the format's header,
//!   key derivation, hashed blocks, inner header and XML, and how items
//!   map to records.
//! - [`qr`] seals and opens a QR vault, an OpenPGP message holding one item
//!   (§1.2, §2.1).
//! - [`records`] is the app's record model: typed records of fields (§7).
//! - [`contents`], [`entries`], [`layouts`], [`child`], [`codex32`] and
//!   [`totp`] are what the records hold and how each reads and writes.
//!
//! Every passphrase is put in Unicode NFC before Argon2id, at creation
//! and at opening, so a passphrase typed in either form derives one key
//! (§3). Nothing here draws randomness: the caller passes what a new file
//! or a seal needs.
//!
//! `no_std` + `alloc`.

#![no_std]
// clippy.toml has the refusal of a copy, text or second buffer of a secret
// (docs/SECRET-LEAKS.md §6); it is on for the whole crate outside tests. A
// legitimate use carries `#[expect(clippy::disallowed_methods, reason = "...")]`.
#![cfg_attr(not(test), warn(clippy::disallowed_methods))]
#![cfg_attr(test, allow(clippy::disallowed_methods))]

extern crate alloc;

pub mod child;
pub mod codex32;
pub mod contents;
pub mod entries;
pub mod file;
pub mod kdbx;
pub mod layouts;
pub mod qr;
pub mod records;
pub mod totp;

use unicode_normalization::UnicodeNormalization;
use zeroize::Zeroizing;

pub use faraday_crypto::Argon2FileCost as Cost;
pub use records::{Field, Record};

/// A copy of `bytes` in a wiping buffer: the one place a slice is copied
/// straight into wiping memory.
#[expect(
    clippy::disallowed_methods,
    reason = "the copy is made in a Zeroizing buffer and held by nothing else"
)]
pub(crate) fn wiped_copy(bytes: &[u8]) -> Zeroizing<alloc::vec::Vec<u8>> {
    Zeroizing::new(bytes.to_vec())
}

/// The Unicode version a passphrase is put in NFC under: the
/// normaliser's tables.
pub use unicode_normalization::UNICODE_VERSION;

/// A passphrase in Unicode Normalization Form C (UAX #15), in a buffer
/// sized once and wiped on drop. Nothing else is done to it: no character
/// is mapped or refused. The normaliser's own buffer of combining marks
/// is not wiped. `Err` holds the MiB the buffer needs when it cannot be
/// allocated.
fn nfc(passphrase: &str) -> Result<Zeroizing<alloc::vec::Vec<u8>>, u32> {
    let len: usize = passphrase.nfc().map(char::len_utf8).sum();
    let mut out = Zeroizing::new(alloc::vec::Vec::new());
    out.try_reserve_exact(len)
        .map_err(|_| (len / (1024 * 1024)) as u32 + 1)?;
    // The capacity is the whole form, so the buffer never moves and
    // leaves no copy behind.
    let mut one = Zeroizing::new([0u8; 4]);
    for ch in passphrase.nfc() {
        out.extend_from_slice(ch.encode_utf8(&mut *one).as_bytes());
    }
    Ok(out)
}

#[cfg(test)]
mod tests;
