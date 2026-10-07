//! Fresh randomness for key material. The shell answers each
//! `RequestEntropy` with 32 bytes from the system's generator; the first
//! answer seeds the session, and the next few wait in a small pool. A key
//! made here (a vault's keys, a GPG key, Secure Boot keys) takes one
//! answer of its own, hashed with what it is for, and the pool is topped
//! up. Nothing secret is derived from the session's bytes and a counter.

use osk_bip::bitcoin::hashes::{Hash, HashEngine, sha256};
use osk_shell_api::Command;
use zeroize::Zeroizing;

use crate::Faraday;

/// Answers kept waiting for the next key.
pub const POOL: usize = 3;

impl Faraday {
    /// The shell answered: the session's seed first, then the pool.
    pub(crate) fn fresh_arrived(&mut self, bytes: [u8; 32]) {
        if !self.seeded {
            self.seed = bytes;
            self.seeded = true;
        } else if self.pool.len() < POOL * 2 {
            self.pool.push(Zeroizing::new(bytes));
        }
    }

    /// 32 bytes of their own for `what`, or `None` while the system has
    /// not answered yet; either way another answer is asked for.
    pub(crate) fn fresh(&mut self, what: &[u8]) -> Option<[u8; 32]> {
        self.commands.push_back(Command::RequestEntropy);
        let bytes = self.pool.pop()?;
        let mut e = sha256::Hash::engine();
        e.input(b"faraday fresh");
        e.input(what);
        e.input(&bytes[..]);
        Some(sha256::Hash::from_engine(e).to_byte_array())
    }

    /// The first display asks for the seed and fills the pool.
    pub(crate) fn fresh_ask(&mut self) {
        if self.seeded {
            return;
        }
        for _ in 0..=POOL {
            self.commands.push_back(Command::RequestEntropy);
        }
    }
}
