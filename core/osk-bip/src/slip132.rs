//! SLIP-132 extended public key encoding: `ypub`/`zpub` on mainnet and
//! `upub`/`vpub` on test networks, which tell a receiving wallet the
//! script type without a descriptor.
//!
//! An extended key is 78 bytes; SLIP-132 only swaps the four version
//! bytes, so encoding is: serialize, replace the version, base58check.

use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;

use bitcoin::NetworkKind;
use bitcoin::base58;
use bitcoin::bip32::Xpub;

use crate::keys::{Network, ScriptType};
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

/// What [`key_facts`] made of the text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyReading {
    /// An extended public key, in every spelling.
    Public(Box<KeyFacts>),
    /// An extended private key, which is not converted.
    Private,
    /// Not an extended key at all.
    None,
}

/// Every spelling of one extended public key, and what it says about
/// itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyFacts {
    /// The BIP-32 form: `xpub` or `tpub`.
    pub bip32: String,
    /// The SLIP-132 form for each script type, in [`ScriptType::ALL`]
    /// order.
    pub slip132: Vec<(ScriptType, String)>,
    /// Which chain the version bytes name.
    pub network: Network,
    /// How far below the master the key is.
    pub depth: u8,
    /// The key's own fingerprint, as eight hex digits.
    pub fingerprint: String,
    /// The child number the key was derived at.
    pub child: String,
}

/// Reads `text` as an extended key and writes it every other way.
///
/// The four test networks share one set of version bytes, so a test key
/// cannot say which of them it is: `network`, the chain the caller is
/// set to, decides, and a test key read while set to mainnet is called
/// testnet.
pub fn key_facts(text: &str, network: Network) -> KeyReading {
    let text = text.trim();
    if text.is_empty() {
        return KeyReading::None;
    }
    if xkey::decode_xpriv(text).is_ok() {
        return KeyReading::Private;
    }
    let Ok((xpub, _)) = decode_xpub(text) else {
        return KeyReading::None;
    };
    let on = match xpub.network {
        NetworkKind::Main => Network::Mainnet,
        _ if network == Network::Mainnet => Network::Testnet,
        _ => network,
    };
    let child = xpub.child_number;
    KeyReading::Public(Box::new(KeyFacts {
        bip32: encode_xpub(&xpub, ScriptType::Legacy),
        slip132: ScriptType::ALL
            .iter()
            .map(|s| (*s, encode_xpub(&xpub, *s)))
            .collect(),
        network: on,
        depth: xpub.depth,
        fingerprint: alloc::format!("{}", xpub.fingerprint()),
        child: alloc::format!("{child}"),
    }))
}

/// Whether an extended public key, in BIP-32 or any SLIP-132 spelling,
/// is a mainnet or a test-network key, by its version bytes.
pub fn network_kind(text: &str) -> Option<NetworkKind> {
    xkey::decode_xpub(text)
        .map(|x| x.network)
        .or_else(|_| decode_xpub(text).map(|(x, _)| x.network))
        .ok()
}
