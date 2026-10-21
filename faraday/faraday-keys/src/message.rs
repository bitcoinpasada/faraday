//! Signing a message with a key below a master key.

use faraday_bip::bitcoin::bip32::DerivationPath;
use faraday_bip::keys::{MasterKey, Network, ScriptType};
use faraday_crypto::work::Compute;
use faraday_psbt::message::{Error, Format, Signed};
use faraday_psbt::{Aux, Nonce};

/// The three-line text a signed message travels as.
pub use faraday_psbt::message::signed_text;

/// Costs a key derivation and a signature. Signs `message` in `format`
/// with the key at `path` below `master`, for an address of `script` on
/// `network`.
pub fn sign(
    c: &Compute,
    master: &MasterKey,
    path: &DerivationPath,
    script: ScriptType,
    network: Network,
    message: &str,
    format: Format,
) -> Result<Signed, Error> {
    let derived = master.derive(c, path);
    faraday_psbt::message::sign(
        c,
        derived.secp(),
        derived.secret_key(),
        script,
        network,
        message,
        format,
        Nonce::default(),
        Aux::default(),
    )
}
