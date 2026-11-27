//! What a vault's open or make takes besides its passphrase, held here:
//! a key file's key (`docs/VAULT.md` §3.2, F208). The bytes of a key file
//! go in; what comes out is [`UnlockParts`], which holds the key, wipes as
//! it drops and is neither `Clone` nor `Debug`. A vault just made has its
//! parts kept in [`OpenVaults`] until the unlock takes them or a lock
//! clears them ([`OpenVaults::hold_made_parts`]).
#![cfg_attr(not(test), warn(clippy::disallowed_methods))]

use faraday_vault as fv;
use fv::kdbx::keyfile::{self, KeyFileError};

use crate::vault::OpenVaults;

/// A key file's key, as an open or a make takes it.
pub struct UnlockParts(pub(crate) fv::kdbx::Components);

impl UnlockParts {
    /// Costs a SHA-256 of the key file. The parts of the key file with
    /// these `bytes`, if any.
    pub fn read(key_file: Option<&[u8]>) -> Result<Self, KeyFileError> {
        Ok(Self(match key_file {
            None => fv::kdbx::Components::none(),
            Some(b) => fv::kdbx::Components::with_key_file(keyfile::read(b)?),
        }))
    }

    /// Whether a key file's key is held.
    pub fn has_key_file(&self) -> bool {
        self.0.has_key_file()
    }
}

impl From<fv::kdbx::Components> for UnlockParts {
    fn from(c: fv::kdbx::Components) -> Self {
        Self(c)
    }
}

impl OpenVaults {
    /// Keeps the parts of the vault just made (none: nothing kept) until
    /// [`OpenVaults::take_made_parts`], a clear or the next hold.
    pub fn hold_made_parts(&mut self, parts: Option<UnlockParts>) {
        self.made_parts = parts;
    }

    /// Whether parts of a vault just made are kept.
    pub fn has_made_parts(&self) -> bool {
        self.made_parts.is_some()
    }

    /// The parts of the vault just made, taken out: kept once only.
    pub fn take_made_parts(&mut self) -> Option<UnlockParts> {
        self.made_parts.take()
    }

    /// The parts of a vault just made, wiped.
    pub fn clear_made_parts(&mut self) {
        self.made_parts = None;
    }
}
