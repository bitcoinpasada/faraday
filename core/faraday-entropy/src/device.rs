//! The device's own generator: the bytes the shell answered the entropy request with.

use crate::{Error, RawEntropy, Strength, Warnings};
use faraday_crypto::{Zeroize, ZeroizeOnDrop, sha256};

/// The 32 bytes the shell answered the core's entropy request with
/// (`faraday_shell_api::Command::RequestEntropy`), taken as a key's
/// entropy.
///
/// The entropy is SHA-256 of those bytes, truncated to the strength, so
/// that the key is not the shell's bytes themselves and one definition
/// covers both word counts. This source trusts the device: nobody
/// watching can tell a faulty or a rigged generator from a good one.
pub struct DeviceRandom {
    bytes: [u8; 32],
    have: bool,
}

impl Default for DeviceRandom {
    fn default() -> Self {
        Self::new()
    }
}

impl Zeroize for DeviceRandom {
    fn zeroize(&mut self) {
        self.bytes.zeroize();
        self.have = false;
    }
}

impl Drop for DeviceRandom {
    fn drop(&mut self) {
        self.zeroize();
    }
}

impl ZeroizeOnDrop for DeviceRandom {}

impl DeviceRandom {
    /// Nothing answered yet.
    pub fn new() -> Self {
        DeviceRandom {
            bytes: [0u8; 32],
            have: false,
        }
    }

    /// The shell answered.
    pub fn set(&mut self, bytes: &[u8; 32]) {
        self.bytes = *bytes;
        self.have = true;
    }

    /// Whether an answer is in.
    pub fn is_enough(&self, _strength: Strength) -> bool {
        self.have
    }

    /// Forgets the answer.
    pub fn clear(&mut self) {
        self.zeroize();
    }

    /// SHA-256 of the bytes.
    pub fn commitment(&self) -> [u8; 32] {
        sha256(&self.bytes)
    }

    /// The commitment truncated to the strength.
    pub fn entropy(&self, strength: Strength) -> Result<RawEntropy, Error> {
        if !self.have {
            return Err(Error::TooFew { have: 0, need: 1 });
        }
        let mut hash = self.commitment();
        let e = RawEntropy::new(&hash[..strength.bytes()]);
        hash.zeroize();
        Ok(e)
    }

    /// The sanity flags: whether an answer is in, and nothing else. No
    /// test of a generator's output tells anyone whether to trust it.
    pub fn warnings(&self, strength: Strength) -> Warnings {
        Warnings {
            too_few: !self.is_enough(strength),
            ..Warnings::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Error, Strength};
    use faraday_crypto::sha256;

    #[test]
    fn the_device_answers_differently_every_time_and_hashes_what_it_answered() {
        let mut a = DeviceRandom::new();
        assert!(!a.is_enough(Strength::Bits128));
        assert!(a.warnings(Strength::Bits128).too_few);
        assert_eq!(
            a.entropy(Strength::Bits128).err(),
            Some(Error::TooFew { have: 0, need: 1 })
        );
        let first = [1u8; 32];
        a.set(&first);
        assert_eq!(a.commitment(), sha256(&first));
        let key_a = a.entropy(Strength::Bits256).unwrap();
        assert_eq!(key_a.as_bytes(), &sha256(&first)[..]);

        // A second run gets a second answer, and so a second key.
        let mut b = DeviceRandom::new();
        let second = [2u8; 32];
        b.set(&second);
        let key_b = b.entropy(Strength::Bits256).unwrap();
        assert_ne!(key_a.as_bytes(), key_b.as_bytes());
        a.clear();
        assert!(!a.is_enough(Strength::Bits256));
    }
}
