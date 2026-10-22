//! A FROST spend's carry file: the secret nonce the next share signs
//! with, which travels with the PSBT on a stick.

use faraday_crypto::work::Compute;
use faraday_psbt::threshold::{self, Carry, CarrySection};
use faraday_psbt::{Context, Inspection, Psbt};
use zeroize::Zeroizing;

pub use faraday_psbt::threshold::Error;

/// A carry section: the signer set and the secret nonces still to be
/// used. It holds secret nonces, so it is neither `Clone` nor `Debug`,
/// wipes on drop, and tells only whether it is empty; inspection
/// ([`inspect`]), signing ([`Keys::sign`](crate::Keys::sign)) and the
/// carry file ([`bytes`]) read it inside this crate.
pub struct Section(pub(crate) CarrySection);

impl Section {
    /// Whether it holds nothing, which is a pass that left nobody to sign.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// Makes the bytes of a carry file: `section`'s nonces and `psbt`, binary.
pub fn bytes(section: &Section, psbt: &[u8]) -> Zeroizing<Vec<u8>> {
    Zeroizing::new(threshold::carry_bytes(&section.0, psbt))
}

/// Reads a carry file: the section it holds and the PSBT beside it,
/// binary.
pub fn parse(bytes: &[u8]) -> Result<(Section, Vec<u8>), Error> {
    Carry::parse(bytes).map(|c| (Section(c.section), c.psbt))
}

/// The section a vault round's saved state holds, when it reads.
pub(crate) fn section_of(state: &[u8]) -> Option<Section> {
    Carry::parse(state).ok().map(|c| Section(c.section))
}

/// Costs what `faraday_psbt::inspect` costs. The inspection of `psbt`
/// under `ctx`, with the carry section the spend arrived with.
pub fn inspect(c: &Compute, psbt: &Psbt, ctx: Context<'_>, carry: Option<&Section>) -> Inspection {
    let mut ctx = ctx;
    ctx.carry = carry.map(|s| &s.0);
    faraday_psbt::inspect(c, psbt, &ctx)
}
