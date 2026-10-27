//! PSBT signing and the nonce check, which use the master keys and the
//! shares a session holds.

use faraday_bip::bip39::Language;
use faraday_bip::bitcoin::secp256k1::PublicKey;
use faraday_bip::keys::{Fingerprint, MasterKey};
use faraday_crypto::work::Compute;
use faraday_psbt::verify::Determinism;
use faraday_psbt::{
    Aux, Context, KeyRef, MusigSession, Nonce, Psbt, ShareDoor, ShareKey, SignResult,
};
use zeroize::Zeroizing;

/// A key that may sign as a FROST share: the public share its words are,
/// and the words and list they are in, from which the secret share is
/// rebuilt only inside a call (`with_secret_share`).
pub struct ShareHolder<'a> {
    /// `secshare · G`, which a group's record lists.
    pub pubshare: PublicKey,
    /// The BIP-39 list the words are from.
    pub language: Language,
    /// The words, when the key has them.
    pub words: Option<&'a str>,
}

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

/// Costs what `faraday_psbt::sign` costs, and what `with_secret_share`
/// costs for each share that signs. Signs `psbt` with the `masters`
/// chosen by `selection` and with the shares in `holders`; `others` are
/// the shares chosen to sign with this one, by identifier; `seed` is the
/// pass's own randomness for the nonces it draws. A share's scalar is
/// rebuilt from its words only inside the call the signer makes through
/// its door.
#[allow(clippy::too_many_arguments)]
pub fn sign(
    c: &Compute,
    psbt: &mut Psbt,
    masters: &[&MasterKey],
    selection: &[Fingerprint],
    ctx: &Context<'_>,
    musig: &mut Option<MusigSession>,
    seed: &Zeroizing<[u8; 32]>,
    holders: &[ShareHolder<'_>],
    others: &[u32],
) -> Result<SignResult, faraday_psbt::Error> {
    let doors: Vec<Box<ShareDoor<'_>>> = holders
        .iter()
        .map(|holder| -> Box<ShareDoor<'_>> {
            Box::new(move |take: &mut dyn FnMut(&faraday_bip::frost::SecShare)| {
                crate::frost::with_secret_share(c, holder.language, holder.words, take);
            })
        })
        .collect();
    let share_keys: Vec<ShareKey<'_>> = holders
        .iter()
        .zip(&doors)
        .map(|(holder, door)| ShareKey {
            pubshare: holder.pubshare,
            open: door.as_ref(),
        })
        .collect();
    faraday_psbt::sign(
        c,
        psbt,
        masters,
        selection,
        ctx,
        false,
        Nonce::default(),
        Aux::default(),
        musig,
        seed,
        &share_keys,
        others,
    )
}
