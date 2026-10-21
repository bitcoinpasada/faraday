//! A FROST spend's carry file: the secret nonce the next share signs
//! with, which travels with the PSBT on a stick.

use faraday_psbt::threshold;
use zeroize::Zeroizing;

pub use faraday_psbt::threshold::{Carry, CarrySection, Error};

/// Makes the bytes of a carry file: `section`'s nonces and `psbt`, binary.
pub fn bytes(section: &CarrySection, psbt: &[u8]) -> Zeroizing<Vec<u8>> {
    Zeroizing::new(threshold::carry_bytes(section, psbt))
}

/// Reads a carry file: the section it holds and the PSBT beside it.
pub fn parse(bytes: &[u8]) -> Result<Carry, Error> {
    Carry::parse(bytes)
}

/// The section a vault round's saved state holds, when it reads.
pub fn section_of(state: &[u8]) -> Option<CarrySection> {
    Carry::parse(state).ok().map(|c| c.section)
}
