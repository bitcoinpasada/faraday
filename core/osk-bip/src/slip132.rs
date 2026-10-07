//! SLIP-132 extended public key encoding: `ypub`/`zpub` on mainnet and
//! `upub`/`vpub` on test networks, which tell a receiving wallet the
//! script type without a descriptor.
//!
//! An extended key is 78 bytes; SLIP-132 only swaps the four version
//! bytes, so encoding is: serialize, replace the version, base58check.

use alloc::string::String;

use bitcoin::NetworkKind;
use bitcoin::base58;
use bitcoin::bip32::Xpub;

use crate::keys::ScriptType;
use crate::xkey::{self, TPUB, XPUB};

/// The vocabulary a rejected extended key is reported in, shared with
/// [`crate::xkey`] because both decoders apply the same rules.
pub use crate::xkey::Error;

/// Public-key version bytes indexed by `(script type, network kind)`.
/// Legacy and Taproot use the BIP-32 versions; the rest are SLIP-132.
const VERSIONS: [(ScriptType, NetworkKind, [u8; 4]); 8] = [
    (ScriptType::Legacy, NetworkKind::Main, XPUB),
    (ScriptType::Legacy, NetworkKind::Test, TPUB),
    (
        ScriptType::NestedSegwit,
        NetworkKind::Main,
        [0x04, 0x9d, 0x7c, 0xb2],
    ), // ypub
    (
        ScriptType::NestedSegwit,
        NetworkKind::Test,
        [0x04, 0x4a, 0x52, 0x62],
    ), // upub
    (
        ScriptType::NativeSegwit,
        NetworkKind::Main,
        [0x04, 0xb2, 0x47, 0x46],
    ), // zpub
    (
        ScriptType::NativeSegwit,
        NetworkKind::Test,
        [0x04, 0x5f, 0x1c, 0xf6],
    ), // vpub
    (ScriptType::Taproot, NetworkKind::Main, XPUB),
    (ScriptType::Taproot, NetworkKind::Test, TPUB),
];

/// The public version bytes for `script_type` on `kind`.
pub fn version_bytes(script_type: ScriptType, kind: NetworkKind) -> [u8; 4] {
    VERSIONS
        .iter()
        .find(|(s, k, _)| *s == script_type && *k == kind)
        .map(|(_, _, v)| *v)
        .expect("every (script type, kind) pair is listed")
}

/// Encodes `xpub` with the version bytes for `script_type`. The network
/// comes from the key itself.
pub fn encode_xpub(xpub: &Xpub, script_type: ScriptType) -> String {
    let mut bytes = xpub.encode();
    bytes[..4].copy_from_slice(&version_bytes(script_type, xpub.network));
    base58::encode_check(&bytes)
}

/// Decodes an `xpub`, `tpub`, `ypub`, `upub`, `zpub` or `vpub` string.
///
/// Plain `xpub`/`tpub` report [`ScriptType::Legacy`]; BIP-32 encoding
/// carries no script type, so the caller must know whether such a key is
/// meant for BIP-44 or BIP-86.
pub fn decode_xpub(s: &str) -> Result<(Xpub, ScriptType), Error> {
    let mut bytes = xkey::payload(s)?;
    let version = xkey::version(&bytes);
    let (script_type, kind) = VERSIONS
        .iter()
        .find(|(_, _, v)| *v == version)
        .map(|(s, k, _)| (*s, *k))
        .ok_or(Error::UnknownVersion(version))?;
    let plain = match kind {
        NetworkKind::Main => XPUB,
        NetworkKind::Test => TPUB,
    };
    bytes[..4].copy_from_slice(&plain);
    let xpub = xkey::xpub_from_bytes(&bytes)?;
    Ok((xpub, script_type))
}
