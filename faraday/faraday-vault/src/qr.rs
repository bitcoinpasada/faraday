//! The QR vault sealed and opened as one call (`docs/VAULT.md` §1.2, §2.1):
//! the item as the QR TOML of [`write_qr`], inside the OpenPGP message
//! [`faraday_pgp::message`] makes, under a passphrase put in Unicode NFC
//! (§3). Nothing here draws randomness: the caller passes the 95 bytes a
//! seal needs.
#![cfg_attr(not(test), warn(clippy::disallowed_methods))]

use alloc::vec::Vec;

use faraday_crypto::work::Compute;
use faraday_pgp::message::{self, RANDOM_LEN};

use crate::contents::{Refusal, read_qr, write_qr};
use crate::{Record, nfc};

/// Why a QR vault was not sealed or opened.
#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    /// The OpenPGP message: not a QR vault, a wrong passphrase, damage.
    Message(message::Error),
    /// The message opened and its contents are not a QR vault's (§2.1),
    /// or the item does not fit one.
    Contents(Refusal),
    /// The passphrase's NFC form could not be allocated: the MiB it needs.
    Memory(u32),
    /// The passphrase is empty (§8): it seals and opens nothing.
    EmptyPassphrase,
}

#[expect(
    clippy::disallowed_methods,
    reason = "a public value: its fields hold no secret"
)]
impl Clone for Error {
    fn clone(&self) -> Self {
        match self {
            Self::Message(a0) => Self::Message(*a0),
            Self::Contents(a0) => Self::Contents(a0.clone()),
            Self::Memory(a0) => Self::Memory(*a0),
            Self::EmptyPassphrase => Self::EmptyPassphrase,
        }
    }
}

/// Costs an Argon2id run at `exponent` (`docs/VAULT.md` §3; a step of
/// [`message::SEAL_EXPONENTS`]): seals the one item `record` as a QR vault
/// under `passphrase`, in its NFC form; an empty passphrase is refused.
/// An empty note seals the empty vault. Every random value comes from `random`.
pub fn seal(
    c: &Compute,
    passphrase: &str,
    exponent: u8,
    random: &[u8; RANDOM_LEN],
    record: &Record,
) -> Result<Vec<u8>, Error> {
    if passphrase.is_empty() {
        return Err(Error::EmptyPassphrase);
    }
    let text = write_qr(record).map_err(Error::Contents)?;
    let passphrase = nfc(passphrase).map_err(Error::Memory)?;
    message::seal(c, passphrase.as_bytes(), exponent, random, &text).map_err(Error::Message)
}

/// Costs an Argon2id run at the cost the message states (§3, checked
/// before anything is allocated), then a parse of the words: opens a QR
/// vault under `passphrase`, in its NFC form, and reads its item. The
/// empty vault is an empty note.
pub fn open(c: &Compute, passphrase: &str, bytes: &[u8]) -> Result<Record, Error> {
    if passphrase.is_empty() {
        return Err(Error::EmptyPassphrase);
    }
    let message = message::parse(bytes).map_err(Error::Message)?;
    let passphrase = nfc(passphrase).map_err(Error::Memory)?;
    let text = message
        .open(c, passphrase.as_bytes())
        .map_err(Error::Message)?;
    read_qr(c, &text).map_err(Error::Contents)
}
