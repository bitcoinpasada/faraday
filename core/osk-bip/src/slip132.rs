//! SLIP-132 extended public key encoding: `ypub`/`zpub` on mainnet and
//! `upub`/`vpub` on test networks, which tell a receiving wallet the
//! script type without a descriptor.
//!
//! An extended key is 78 bytes; SLIP-132 only swaps the four version
//! bytes, so encoding is: serialize, replace the version, base58check.
//!
//! [`decode`] reads all ten public versions, the multisig `Ypub`/`Zpub`/
//! `Upub`/`Vpub` too, and says which [`Script`] a version names (none for
//! `xpub`/`tpub`). A version is taken only where its type agrees with the
//! script around the key ([`plain_descriptor`]) or the purpose in its
//! path ([`key_line`]); a key that disagrees is a [`Mismatch`], never
//! converted silently. Faraday's since the fork (`docs/UPSTREAM.md` §3).

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
/// meant for BIP-44 or BIP-86. The multisig versions (`Ypub`, `Zpub`,
/// `Upub`, `Vpub`) name no single-key script and are refused here as
/// unknown; [`decode`] reads all ten.
pub fn decode_xpub(s: &str) -> Result<(Xpub, ScriptType), Error> {
    let (bytes, at) = named_payload(s)?;
    let script_type = match NAMED[at].2 {
        None => ScriptType::Legacy,
        Some(Script::NestedSegwit) => ScriptType::NestedSegwit,
        Some(Script::NativeSegwit) => ScriptType::NativeSegwit,
        Some(_) => return Err(Error::UnknownVersion(NAMED[at].0)),
    };
    Ok((named_key(bytes, at)?.xpub, script_type))
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

/// A wallet's script type, as far as SLIP-132's versions and the
/// purposes in key paths tell them apart.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Script {
    /// `wpkh`, BIP-84: `zpub`, `vpub`.
    NativeSegwit,
    /// `sh(wpkh)`, BIP-49: `ypub`, `upub`.
    NestedSegwit,
    /// `wsh(…)`, BIP-48 script type 2: `Zpub`, `Vpub`.
    NativeMultisig,
    /// `sh(wsh(…))`, BIP-48 script type 1: `Ypub`, `Upub`.
    NestedMultisig,
    /// `tr`, BIP-86, BIP-48 script type 3.
    Taproot,
    /// `pkh`, BIP-44.
    Legacy,
    /// `sh(…)` of a script, BIP-45.
    LegacyMultisig,
    /// `sp()`, BIP-352.
    SilentPayments,
    /// Any other descriptor.
    Other,
}

impl Script {
    /// The script type a descriptor's outermost functions name, after any
    /// label before it (`Receive: wpkh(…)`).
    pub fn of_descriptor(descriptor: &str) -> Script {
        let d = descriptor
            .rsplit_once(": ")
            .map_or(descriptor, |(_, rest)| rest)
            .trim_start();
        if d.starts_with("sh(wpkh(") {
            Script::NestedSegwit
        } else if d.starts_with("sh(wsh(") {
            Script::NestedMultisig
        } else if d.starts_with("wpkh(") {
            Script::NativeSegwit
        } else if d.starts_with("wsh(") {
            Script::NativeMultisig
        } else if d.starts_with("tr(") || d.starts_with("rawtr(") {
            Script::Taproot
        } else if d.starts_with("pkh(") {
            Script::Legacy
        } else if d.starts_with("sh(") {
            Script::LegacyMultisig
        } else if d.starts_with("sp(") {
            Script::SilentPayments
        } else {
            Script::Other
        }
    }

    /// The script type the purpose in a key's path names (`84h/0h/0h`,
    /// `m/48'/1'/0'/2'`): 84 native SegWit, 49 nested, 48 by its script
    /// type step (2, 1, 3), 44 legacy, 45 legacy multisig, 86 Taproot,
    /// 352 silent payments. `None` for a path with no purpose this list
    /// names, which tells nothing.
    pub fn of_path(path: &str) -> Option<Script> {
        let steps: Vec<&str> = path
            .trim_start_matches('m')
            .split('/')
            .filter(|s| !s.is_empty())
            .map(|s| s.trim_end_matches(['h', 'H', '\'']))
            .collect();
        Some(match *steps.first()? {
            "84" => Script::NativeSegwit,
            "49" => Script::NestedSegwit,
            "48" => match *steps.get(3)? {
                "2" => Script::NativeMultisig,
                "1" => Script::NestedMultisig,
                "3" => Script::Taproot,
                _ => return None,
            },
            "44" => Script::Legacy,
            "45" => Script::LegacyMultisig,
            "86" => Script::Taproot,
            "352" => Script::SilentPayments,
            _ => return None,
        })
    }
}

/// Every public version read, SLIP-0132's ten: BIP-32's own, which name
/// no script, then the four SLIP-132 adds, on each network.
const NAMED: [([u8; 4], NetworkKind, Option<Script>); 10] = [
    (XPUB, NetworkKind::Main, None),
    (
        [0x04, 0x9d, 0x7c, 0xb2],
        NetworkKind::Main,
        Some(Script::NestedSegwit),
    ), // ypub
    (
        [0x04, 0xb2, 0x47, 0x46],
        NetworkKind::Main,
        Some(Script::NativeSegwit),
    ), // zpub
    (
        [0x02, 0x95, 0xb4, 0x3f],
        NetworkKind::Main,
        Some(Script::NestedMultisig),
    ), // Ypub
    (
        [0x02, 0xaa, 0x7e, 0xd3],
        NetworkKind::Main,
        Some(Script::NativeMultisig),
    ), // Zpub
    (TPUB, NetworkKind::Test, None),
    (
        [0x04, 0x4a, 0x52, 0x62],
        NetworkKind::Test,
        Some(Script::NestedSegwit),
    ), // upub
    (
        [0x04, 0x5f, 0x1c, 0xf6],
        NetworkKind::Test,
        Some(Script::NativeSegwit),
    ), // vpub
    (
        [0x02, 0x42, 0x89, 0xef],
        NetworkKind::Test,
        Some(Script::NestedMultisig),
    ), // Upub
    (
        [0x02, 0x57, 0x54, 0x83],
        NetworkKind::Test,
        Some(Script::NativeMultisig),
    ), // Vpub
];

/// The prefixes a SLIP-132 public key is written with.
const PREFIXES: [&str; 8] = [
    "ypub", "zpub", "Ypub", "Zpub", "upub", "vpub", "Upub", "Vpub",
];

/// An extended public key in any of the ten versions, decoded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Key {
    /// The key.
    pub xpub: Xpub,
    /// The key in BIP-32's own encoding, `xpub` or `tpub`.
    pub plain: String,
    /// The script type its version names; `None` for `xpub` and `tpub`,
    /// which name none (they are not Legacy).
    pub named: Option<Script>,
}

/// The payload and the row of [`NAMED`] its version is, before the key
/// itself is checked.
fn named_payload(s: &str) -> Result<(Vec<u8>, usize), Error> {
    let bytes = xkey::payload(s)?;
    let version = xkey::version(&bytes);
    let at = NAMED
        .iter()
        .position(|(v, _, _)| *v == version)
        .ok_or(Error::UnknownVersion(version))?;
    Ok((bytes, at))
}

/// The key behind a payload of row `at`, checked as every extended key
/// text is ([`xkey::xpub_from_bytes`]).
fn named_key(mut bytes: Vec<u8>, at: usize) -> Result<Key, Error> {
    let (_, kind, named) = NAMED[at];
    let plain = match kind {
        NetworkKind::Main => XPUB,
        NetworkKind::Test => TPUB,
    };
    bytes[..4].copy_from_slice(&plain);
    let xpub = xkey::xpub_from_bytes(&bytes)?;
    Ok(Key {
        xpub,
        plain: base58::encode_check(&bytes),
        named,
    })
}

/// Decodes an extended public key in any of SLIP-0132's ten public
/// versions, the white space around it ignored: the one decoder every
/// reader of a SLIP-132 key uses. Costs a base58 decode, two SHA-256
/// checksums and a point decompression.
pub fn decode(text: &str) -> Result<Key, Error> {
    let (bytes, at) = named_payload(text.trim())?;
    named_key(bytes, at)
}

/// `plain`, an `xpub` or `tpub`, in the SLIP-132 version that names
/// `script` on its network: `zpub`/`vpub` for native SegWit, `ypub`/`upub`
/// nested, `Zpub`/`Vpub` native multisig, `Ypub`/`Upub` nested multisig;
/// as it is for any other type, which SLIP-132 does not name. `None` for
/// a key that is neither `xpub` nor `tpub`. Costs a base58 decode and
/// encode.
pub fn encode(plain: &str, script: Script) -> Option<String> {
    let mut bytes = xkey::payload(plain.trim()).ok()?;
    let kind = match xkey::version(&bytes) {
        XPUB => NetworkKind::Main,
        TPUB => NetworkKind::Test,
        _ => return None,
    };
    let Some((version, _, _)) = NAMED
        .iter()
        .find(|(_, k, named)| *k == kind && *named == Some(script))
    else {
        return Some(String::from(plain.trim()));
    };
    bytes[..4].copy_from_slice(version);
    Some(base58::encode_check(&bytes))
}

/// A key whose version names one script type where another is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mismatch {
    /// The key as written.
    pub key: String,
    /// The type its version names.
    pub named: Script,
    /// The type the script or path around it is.
    pub around: Script,
}

/// Whether a key whose version names `named` may stand where `around` is.
pub fn agrees(named: Option<Script>, around: Script) -> bool {
    named.is_none_or(|n| n == around)
}

/// Whether text has a SLIP-132 public key's prefix anywhere in it: a
/// cheap test before [`plain_descriptor`] does any work.
pub fn mentions(text: &str) -> bool {
    PREFIXES.iter().any(|p| text.contains(p))
}

/// A descriptor with each SLIP-132 key in it written as `xpub` or
/// `tpub`, every one checked against the descriptor's script type, and
/// its checksum, when it has one, made again for the new text. A
/// descriptor with no such key comes back as it is; so does one whose
/// checksum is wrong as written, for the reader to refuse. Costs a
/// base58 decode per key and a checksum over the descriptor.
pub fn plain_descriptor(descriptor: &str) -> Result<String, Mismatch> {
    use miniscript::descriptor::checksum::desc_checksum;
    if !mentions(descriptor) {
        return Ok(String::from(descriptor));
    }
    let (body, sum) = match descriptor.rsplit_once('#') {
        Some((b, s)) => (b, Some(s.trim())),
        None => (descriptor, None),
    };
    // The label before a descriptor (`Receive: `) is no part of it.
    let (label, desc) = match body.rsplit_once(": ") {
        Some((l, d)) => (&body[..l.len() + 2], d),
        None => ("", body),
    };
    if let Some(s) = sum
        && desc_checksum(desc.trim()).ok().as_deref() != Some(s)
    {
        return Ok(String::from(descriptor));
    }
    let around = Script::of_descriptor(desc);
    let mut out = String::with_capacity(desc.len());
    let mut rest = desc;
    while !rest.is_empty() {
        let start = rest
            .find(|c: char| c.is_ascii_alphanumeric())
            .unwrap_or(rest.len());
        out.push_str(&rest[..start]);
        rest = &rest[start..];
        let len = rest
            .find(|c: char| !c.is_ascii_alphanumeric())
            .unwrap_or(rest.len());
        let word = &rest[..len];
        rest = &rest[len..];
        match PREFIXES
            .iter()
            .any(|p| word.starts_with(p))
            .then(|| decode(word).ok())
            .flatten()
        {
            Some(Key { plain, named, .. }) => {
                if let Some(n) = named
                    && !agrees(named, around)
                {
                    return Err(Mismatch {
                        key: String::from(word),
                        named: n,
                        around,
                    });
                }
                out.push_str(&plain);
            }
            None => out.push_str(word),
        }
    }
    let mut written = alloc::format!("{label}{out}");
    if sum.is_some()
        && let Ok(s) = desc_checksum(out.trim())
    {
        written.push('#');
        written.push_str(&s);
    }
    Ok(written)
}

/// A key on a line of its own, `[fingerprint/path]key` or the key alone,
/// as written by Specter DIY, BlueWallet or a person.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyLine {
    /// The line with its key as `xpub` or `tpub`, the origin in lower case.
    pub text: String,
    /// The script type the key's version names.
    pub named: Option<Script>,
}

/// Reads a line that is one extended public key with or without its
/// origin, the version checked against the purpose in the origin's path.
/// `None` for a line that is no such key. Costs a base58 decode.
pub fn key_line(line: &str) -> Option<Result<KeyLine, Mismatch>> {
    let t = line.trim();
    let (origin, key) = match t.strip_prefix('[') {
        Some(rest) => {
            let (o, k) = rest.split_once(']')?;
            (Some(o), k)
        }
        None => (None, t),
    };
    let Key { plain, named, .. } = decode(key).ok()?;
    let around = origin.and_then(|o| Script::of_path(o.split_once('/').map_or("", |(_, p)| p)));
    if let (Some(n), Some(a)) = (named, around)
        && n != a
    {
        return Some(Err(Mismatch {
            key: String::from(key),
            named: n,
            around: a,
        }));
    }
    let text = match origin {
        Some(o) => alloc::format!("[{}]{plain}", o.to_ascii_lowercase()),
        None => plain,
    };
    Some(Ok(KeyLine { text, named }))
}
