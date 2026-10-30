//! A version 2 PSBT (BIP-370, `docs/INTEROP.md` §5 I10) in a spend: read
//! as the version 0 PSBT of its transaction, which is what is signed,
//! and written out again as version 2.

use faraday_crypto::work::Compute;
use faraday_psbt::Psbt;
use faraday_psbt::v2::PsbtV2;

use super::Spend;

/// Costs a PSBT parse (through `parse_bytes`, `parse_base64` and
/// `PsbtV2::to_v0`). Reads a PSBT from a file's bytes, binary or base64,
/// version 0 or version 2 (as the version 0 PSBT of its transaction:
/// [`read_psbt_kept`]).
pub fn read_psbt(c: &Compute, bytes: &[u8]) -> Option<Psbt> {
    read_psbt_kept(c, bytes).map(|(p, _)| p)
}

/// Costs a PSBT parse (through `parse_bytes`, `parse_base64` and
/// `PsbtV2::to_v0`). [`read_psbt`], with a version 2 PSBT as read
/// beside the version 0 PSBT it is signed as: `faraday_psbt` and
/// `bitcoin` sign version 0, and what goes back out is version 2 again
/// ([`Spend::out_bytes`]). A version 2 PSBT with an output that has no
/// script yet, a silent payment's, has no transaction to sign and is
/// not read here.
pub fn read_psbt_kept(c: &Compute, bytes: &[u8]) -> Option<(Psbt, Option<PsbtV2>)> {
    if let Ok(p) = Psbt::parse_bytes(c, bytes) {
        return Some((p, None));
    }
    let text = std::str::from_utf8(bytes).ok().map(str::trim);
    if let Some(p) = text.and_then(|t| Psbt::parse_base64(c, t).ok()) {
        return Some((p, None));
    }
    let v2 = PsbtV2::parse_bytes(c, bytes)
        .ok()
        .or_else(|| PsbtV2::parse_base64(c, text?).ok())?;
    let v0 = v2.to_v0(c).ok()?;
    Some((v0, Some(v2)))
}

impl Spend {
    /// Starts a spend from a PSBT read with [`read_psbt_kept`]: `v2` is
    /// the version 2 PSBT as read, when it came as one.
    pub fn new_kept(psbt: Psbt, v2: Option<PsbtV2>, source: &str) -> Spend {
        Spend {
            v2,
            ..Spend::new(psbt, source)
        }
    }

    /// Writes the PSBT out as it stands, in the version it came in: a
    /// version 2 PSBT as it came with only the signing fields this
    /// spend added (partial signatures, final scripts, taproot and
    /// MuSig2 signatures and nonces: `PsbtV2::merge_signed`), every
    /// other pair as read. Refuses where the signatures do not go back
    /// into the version 2 PSBT (`PsbtV2::merge_signed`'s refusals: the
    /// version 0 PSBT is of another transaction, or changes a pair the
    /// version 2 one has); a version 2 PSBT is never answered with a
    /// version 0 one, so the caller writes and shows nothing.
    pub fn out_bytes(&self) -> Result<Vec<u8>, faraday_psbt::v2::Error> {
        let Some(v2) = &self.v2 else {
            return Ok(self.psbt.to_bytes());
        };
        let mut out = v2.clone();
        out.merge_signed(&self.psbt)?;
        Ok(out.to_bytes())
    }
}
