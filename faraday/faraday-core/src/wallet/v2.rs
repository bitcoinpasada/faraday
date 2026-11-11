//! A version 2 PSBT (BIP-370, `docs/INTEROP.md` §5, PSBT version 2) in a spend: read
//! as the version 0 PSBT of its transaction, which is what is signed,
//! and written out again as version 2.

use faraday_crypto::work::Compute;
use faraday_psbt::Psbt;
use faraday_psbt::silent::{
    GLOBAL_SP_DLEQ, GLOBAL_SP_ECDH_SHARE, IN_SP_DLEQ, IN_SP_ECDH_SHARE, OUT_SP_V0_INFO,
    OUT_SP_V0_LABEL,
};
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
/// not read here; nor is one with any BIP-375 silent payment field,
/// which the device does not sign ([`is_silent_payment_psbt`]).
pub fn read_psbt_kept(c: &Compute, bytes: &[u8]) -> Option<(Psbt, Option<PsbtV2>)> {
    let text = std::str::from_utf8(bytes).ok().map(str::trim);
    if let Some(p) = Psbt::parse_bytes(c, bytes)
        .ok()
        .or_else(|| Psbt::parse_base64(c, text?).ok())
    {
        return (!v0_has_silent_fields(&p)).then_some((p, None));
    }
    let v2 = PsbtV2::parse_bytes(c, bytes)
        .ok()
        .or_else(|| PsbtV2::parse_base64(c, text?).ok())?;
    if v2.has_silent_fields() {
        return None;
    }
    let v0 = v2.to_v0(c).ok()?;
    Some((v0, Some(v2)))
}

/// Whether a version 0 PSBT carries a field type of BIP-375 (global
/// 0x07 or 0x08, input 0x1d or 0x1e, output 0x09 or 0x0a) among its
/// unknown fields. BIP-375 allows them in version 2 only, and a version 0
/// file that has them is a silent payment's, to be refused as one.
fn v0_has_silent_fields(p: &Psbt) -> bool {
    let inner = p.inner();
    let has = |types: Vec<u8>, wanted: [u8; 2]| types.iter().any(|t| wanted.contains(t));
    has(
        inner.unknown.keys().map(|k| k.type_value).collect(),
        [GLOBAL_SP_ECDH_SHARE, GLOBAL_SP_DLEQ],
    ) || inner.inputs.iter().any(|i| {
        has(
            i.unknown.keys().map(|k| k.type_value).collect(),
            [IN_SP_ECDH_SHARE, IN_SP_DLEQ],
        )
    }) || inner.outputs.iter().any(|o| {
        has(
            o.unknown.keys().map(|k| k.type_value).collect(),
            [OUT_SP_V0_INFO, OUT_SP_V0_LABEL],
        )
    })
}

/// Costs a read of the PSBT. Whether `bytes` are a PSBT, version 2 or
/// version 0, that carries a BIP-375 silent payment field, which
/// [`read_psbt_kept`] refuses.
pub fn is_silent_payment_psbt(c: &Compute, bytes: &[u8]) -> bool {
    let text = std::str::from_utf8(bytes).ok().map(str::trim);
    if let Some(p) = Psbt::parse_bytes(c, bytes)
        .ok()
        .or_else(|| Psbt::parse_base64(c, text?).ok())
    {
        return v0_has_silent_fields(&p);
    }
    PsbtV2::parse_bytes(c, bytes)
        .ok()
        .or_else(|| PsbtV2::parse_base64(c, text?).ok())
        .is_some_and(|v2| v2.has_silent_fields())
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
        out.merge_signed_by(&self.psbt, &self.nonces_written)?;
        Ok(out.to_bytes())
    }
}
