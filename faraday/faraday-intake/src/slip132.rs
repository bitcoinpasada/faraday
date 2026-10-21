//! Extended keys in SLIP-0132's versions (`ypub`, `zpub`, `Ypub`, `Zpub`
//! and their test-network forms `upub`, `vpub`, `Upub`, `Vpub`), read as
//! BIP-32's own `xpub` or `tpub` (`docs/INTEROP.md` §5 I1).
//!
//! A SLIP-132 version names a script type. It is taken only where that
//! type agrees with the script around the key, or, for a key on its own,
//! with the purpose in its path; a key that disagrees is a [`Mismatch`],
//! never converted silently. `xpub` and `tpub` name no type and agree
//! with every script.

use faraday_strings::{EN, fill};

/// A wallet's script type, as far as SLIP-132 and the purposes in key
/// paths tell them apart.
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
    /// What the type is called in a refusal.
    pub fn name(self) -> &'static str {
        match self {
            Script::NativeSegwit => EN.wallet_native_segwit,
            Script::NestedSegwit => EN.wallet_nested_segwit,
            Script::NativeMultisig => EN.wallet_slip132_native_multisig,
            Script::NestedMultisig => EN.wallet_slip132_nested_multisig,
            Script::Taproot => EN.wallet_taproot,
            Script::Legacy => EN.wallet_legacy,
            Script::LegacyMultisig => EN.wallet_slip132_legacy_multisig,
            Script::SilentPayments => EN.wallet_slip132_silent_payments,
            Script::Other => EN.wallet_slip132_other,
        }
    }

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

/// The versions read: BIP-32's own first on each network, then SLIP-132's,
/// with the script type each names.
const VERSIONS: [([u8; 4], Option<Script>); 10] = [
    ([0x04, 0x88, 0xb2, 0x1e], None),
    ([0x04, 0x9d, 0x7c, 0xb2], Some(Script::NestedSegwit)),
    ([0x04, 0xb2, 0x47, 0x46], Some(Script::NativeSegwit)),
    ([0x02, 0x95, 0xb4, 0x3f], Some(Script::NestedMultisig)),
    ([0x02, 0xaa, 0x7e, 0xd3], Some(Script::NativeMultisig)),
    ([0x04, 0x35, 0x87, 0xcf], None),
    ([0x04, 0x4a, 0x52, 0x62], Some(Script::NestedSegwit)),
    ([0x04, 0x5f, 0x1c, 0xf6], Some(Script::NativeSegwit)),
    ([0x02, 0x42, 0x89, 0xef], Some(Script::NestedMultisig)),
    ([0x02, 0x57, 0x54, 0x83], Some(Script::NativeMultisig)),
];

/// The prefixes a SLIP-132 public key is written with.
const PREFIXES: [&str; 8] = [
    "ypub", "zpub", "Ypub", "Zpub", "upub", "vpub", "Upub", "Vpub",
];

/// An extended public key, decoded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Key {
    /// The key in BIP-32's own encoding, `xpub` or `tpub`.
    pub plain: String,
    /// The script type its version names; `None` for `xpub` and `tpub`.
    pub named: Option<Script>,
}

/// Decodes an extended public key in any of the versions above; `None`
/// for anything else. Costs a base58 decode and two SHA-256 checksums.
pub fn decode(key: &str) -> Option<Key> {
    use osk_bip::bitcoin::base58;
    let mut bytes = base58::decode_check(key.trim()).ok()?;
    if bytes.len() != 78 {
        return None;
    }
    let at = VERSIONS.iter().position(|(v, _)| bytes[..4] == *v)?;
    let named = VERSIONS[at].1;
    if named.is_none() {
        return Some(Key {
            plain: key.trim().to_string(),
            named,
        });
    }
    // Mainnet's versions are the first five, the test networks' the rest.
    let plain = if at < 5 { VERSIONS[0].0 } else { VERSIONS[5].0 };
    bytes[..4].copy_from_slice(&plain);
    Some(Key {
        plain: base58::encode_check(&bytes),
        named,
    })
}

/// `plain`, an `xpub` or `tpub`, in the SLIP-132 version that names
/// `script` on its network: `zpub`/`vpub` for native SegWit, `ypub`/`upub`
/// nested, `Zpub`/`Vpub` native multisig, `Ypub`/`Upub` nested multisig;
/// as it is for any other type, which SLIP-132 does not name. `None` for
/// a key that is neither. Costs a base58 decode and encode.
pub fn encode(plain: &str, script: Script) -> Option<String> {
    use osk_bip::bitcoin::base58;
    let mut bytes = base58::decode_check(plain.trim()).ok()?;
    if bytes.len() != 78 {
        return None;
    }
    let test = match &bytes[..4] {
        v if *v == VERSIONS[0].0 => false,
        v if *v == VERSIONS[5].0 => true,
        _ => return None,
    };
    let base = if test { 5 } else { 0 };
    let Some(at) = VERSIONS[base..base + 5]
        .iter()
        .position(|(_, named)| *named == Some(script))
    else {
        return Some(plain.trim().to_string());
    };
    bytes[..4].copy_from_slice(&VERSIONS[base + at].0);
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

impl Mismatch {
    /// The sentence a refusal shows: the key, shortened, and both types.
    pub fn text(&self) -> String {
        let k = &self.key;
        let short = if k.len() > 16 && k.is_ascii() {
            format!("{}…{}", &k[..8], &k[k.len() - 6..])
        } else {
            k.clone()
        };
        fill(
            EN.wallet_slip132_mismatch,
            &[&short, self.named.name(), self.around.name()],
        )
    }
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
    if !mentions(descriptor) {
        return Ok(descriptor.to_string());
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
    let checksum = osk_bip::miniscript::descriptor::checksum::desc_checksum;
    if let Some(s) = sum
        && checksum(desc.trim()).ok().as_deref() != Some(s)
    {
        return Ok(descriptor.to_string());
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
            .then(|| decode(word))
            .flatten()
        {
            Some(Key { plain, named }) => {
                if let Some(n) = named
                    && !agrees(named, around)
                {
                    return Err(Mismatch {
                        key: word.to_string(),
                        named: n,
                        around,
                    });
                }
                out.push_str(&plain);
            }
            None => out.push_str(word),
        }
    }
    let mut written = format!("{label}{out}");
    if sum.is_some()
        && let Ok(s) = checksum(out.trim())
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
    /// The line with its key as `xpub` or `tpub`.
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
    let Key { plain, named } = decode(key)?;
    let around = origin.and_then(|o| Script::of_path(o.split_once('/').map_or("", |(_, p)| p)));
    if let (Some(n), Some(a)) = (named, around)
        && n != a
    {
        return Some(Err(Mismatch {
            key: key.to_string(),
            named: n,
            around: a,
        }));
    }
    let text = match origin {
        Some(o) => format!("[{}]{plain}", o.to_ascii_lowercase()),
        None => plain,
    };
    Some(Ok(KeyLine { text, named }))
}
