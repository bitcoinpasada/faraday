//! PSBT signing and the nonce check, which use the master keys a session
//! holds.

use faraday_bip::keys::{Fingerprint, MasterKey};
use faraday_crypto::work::Compute;
use faraday_psbt::verify::Determinism;
use faraday_psbt::{Aux, Context, KeyRef, MusigSession, Nonce, Psbt, SignResult, WarningKind};
use zeroize::Zeroizing;

/// Costs a key derivation per master key (through `from_master`). The
/// key references a context needs, for the keys' first account.
pub fn key_refs(c: &Compute, masters: &[&MasterKey]) -> Vec<KeyRef> {
    masters
        .iter()
        .filter_map(|m| KeyRef::from_master(c, m, 0).ok())
        .collect()
}

/// Costs what `deterministic` costs. Every signature `psbt` carries by
/// one of `masters`, recomputed from that key (RFC 6979 or BIP-340 with
/// no extra randomness), for the nonce check to compare
/// (`faraday_psbt::verify`).
pub fn recompute_signatures(c: &Compute, psbt: &Psbt, masters: &[&MasterKey]) -> Vec<Determinism> {
    faraday_psbt::verify::deterministic(c, psbt, masters)
}

/// Costs what `faraday_psbt::sign` costs. Signs `psbt` with the
/// `masters` chosen by `selection`; `seed` is the pass's own randomness
/// for the nonces it draws. `accepted`, beside the context, is the
/// holdable dangers the person held past (F136); every other danger
/// refuses.
pub fn sign(
    c: &Compute,
    psbt: &mut Psbt,
    masters: &[&MasterKey],
    selection: &[Fingerprint],
    (ctx, accepted): (&Context<'_>, &[WarningKind]),
    musig: &mut Option<MusigSession>,
    seed: &Zeroizing<[u8; 32]>,
) -> Result<SignResult, faraday_psbt::Error> {
    faraday_psbt::sign(
        c,
        psbt,
        masters,
        selection,
        ctx,
        accepted,
        Nonce::default(),
        Aux::default(),
        musig,
        seed,
    )
}
