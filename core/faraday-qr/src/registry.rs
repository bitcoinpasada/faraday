//! The UR registry's key and wallet types, read into the text Faraday
//! already takes: a key expression or a descriptor.
//!
//! - `ur:psbt`, the 2023 name for `crypto-psbt`: the PSBT's bytes.
//! - `crypto-hdkey` (BCR-2020-007): one extended public key with its origin.
//! - `crypto-account` (BCR-2020-015): what SeedSigner shows for "Export
//!   Xpub": a master fingerprint and one or more script expressions, each
//!   around an `hdkey`. Read as cosigner key lines, the native SegWit
//!   multisig key first.
//! - `crypto-output` (BCR-2020-010): a wallet as Sparrow shows one. It
//!   cannot write `<0;1>`, so a key's `/0/*` (or no children at all) is read
//!   as both chains, `/<0;1>/*`. Read, never written.
//! - `crypto-seed` and a private `hdkey` are refused, saying what they are.
//!
//! The tag numbers are those of `urtypes`, the library SeedSigner reads
//! them with.

use faraday_bip::bitcoin::NetworkKind;
use faraday_bip::bitcoin::bip32::{ChainCode, ChildNumber, Fingerprint, Xpub};
use faraday_bip::bitcoin::secp256k1::PublicKey;

use crate::cbor::Value;

const HDKEY: u64 = 303;
const KEYPATH: u64 = 304;
const OUTPUT: u64 = 308;
const COININFO: u64 = 305;
// The same three types under the numbers `ur:hdkey` gives them (BCR-2020-007's
// vectors, and what the first reader here took): each means what its partner
// does.
const HDKEY_UR: u64 = 40303;
const KEYPATH_UR: u64 = 40304;
const COININFO_UR: u64 = 40305;
const ACCOUNT: u64 = 311;

/// `v` without the one tag (`n`, or its `ur:hdkey` number `m`) its type may
/// carry. Another tag, or two, is not that type.
fn strip(v: &Value, n: u64, m: u64) -> Option<&Value> {
    match v {
        Value::Tag(t, inner) if (*t == n || *t == m) && !matches!(**inner, Value::Tag(..)) => {
            Some(inner)
        }
        Value::Tag(..) => None,
        v => Some(v),
    }
}

/// What a registry type was read as.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Read {
    /// A PSBT's bytes.
    Psbt(Vec<u8>),
    /// Cosigner key lines, `[fingerprint/path]xpub`, with comments.
    Keys(String),
    /// A descriptor.
    Descriptor(String),
}

/// Why a registry type was refused: a kind, which the app words.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The code's CBOR, of this UR type, is not in the registry's shape.
    Shape(String),
    /// A `ur:psbt` that holds no bytes.
    Psbt,
    /// A seed (`ur:crypto-seed`).
    Seed,
    /// A key path not in the registry's shape.
    Path,
    /// A key not in the registry's shape.
    Key,
    /// A private key.
    Private,
    /// A key with no chain code.
    NoChainCode,
    /// A wallet not in the registry's shape.
    Wallet,
    /// A key in a wallet that is not an extended key.
    NotExtended,
    /// A multisig with no threshold.
    NoThreshold,
    /// A multisig with no keys.
    NoMultisigKeys,
    /// A wallet script of this tag, which Faraday does not read.
    Script(u64),
    /// An account with no master fingerprint.
    NoFingerprint,
    /// An account with no keys.
    NoAccountKeys,
    /// An account with no key Faraday reads.
    NoKeyRead,
}

/// Whether every map in `v` has unsigned integer keys, as the registry's
/// types do. Another key can read as a number in a second reader (a tag 2
/// bignum, `true` beside 1), and that reader would then see another value.
fn number_keys(v: &Value) -> bool {
    match v {
        Value::Map(m) => m
            .iter()
            .all(|(k, v)| matches!(k, Value::Uint(_)) && number_keys(v)),
        Value::Array(a) => a.iter().all(number_keys),
        Value::Tag(_, inner) => number_keys(inner),
        _ => true,
    }
}

/// Reads a UR of `ur_type` whose CBOR is `cbor`. `Ok(None)` for a type
/// this does not know; `Err` with why for one it refuses.
pub fn read(ur_type: &str, cbor: &[u8]) -> Result<Option<Read>, Error> {
    let v = || {
        Value::decode(cbor)
            .ok()
            .filter(number_keys)
            .ok_or_else(|| Error::Shape(ur_type.to_string()))
    };
    match ur_type {
        "psbt" => match v()? {
            Value::Bytes(b) => Ok(Some(Read::Psbt(b))),
            _ => Err(Error::Psbt),
        },
        "crypto-seed" | "seed" => Err(Error::Seed),
        "crypto-hdkey" | "hdkey" => {
            let k = hdkey(&v()?)?;
            Ok(Some(Read::Keys(format!(
                "# One account key (ur:{ur_type})\n{}\n",
                k.origin_text()
            ))))
        }
        "crypto-account" | "account-descriptor" => account(&v()?).map(|k| Some(Read::Keys(k))),
        "crypto-output" | "output-descriptor" => {
            output(&v()?).map(|d| Some(Read::Descriptor(format!("{d}\n"))))
        }
        _ => Ok(None),
    }
}

/// One extended public key, as an hdkey states it.
struct Key {
    xpub: Xpub,
    /// `fingerprint/path` of its origin, when stated.
    origin: Option<String>,
    /// `/0/*` and the like, when stated.
    children: Option<String>,
}

impl Key {
    /// `[fingerprint/path]xpub`, or the bare xpub.
    fn origin_text(&self) -> String {
        match &self.origin {
            Some(o) => format!("[{o}]{}", self.xpub),
            None => self.xpub.to_string(),
        }
    }

    /// The key expression in a wallet: both chains, unless the code says
    /// something other than the receive chain.
    fn in_wallet(&self) -> String {
        let children = match self.children.as_deref() {
            None | Some("/0/*") | Some("/<0;1>/*") => "/<0;1>/*",
            Some(c) => c,
        };
        format!("{}{children}", self.origin_text())
    }
}

/// `[index, hardened, …]` as `/48h/0h` text, a wildcard as `*`.
fn path(v: &Value) -> Result<String, Error> {
    let bad = || Error::Path;
    // The components are required; an empty array is a path with no steps.
    let Some(Value::Array(c)) = strip(v, KEYPATH, KEYPATH_UR).ok_or_else(bad)?.get(1) else {
        return Err(bad());
    };
    if c.len() % 2 != 0 {
        return Err(bad());
    }
    let mut out = String::new();
    for pair in c.chunks(2) {
        let Value::Bool(hardened) = pair[1] else {
            return Err(bad());
        };
        match &pair[0] {
            Value::Uint(i) if *i < 0x8000_0000 => out.push_str(&format!("/{i}")),
            Value::Array(a) if a.is_empty() => out.push_str("/*"),
            _ => return Err(bad()),
        }
        if hardened {
            out.push('h');
        }
    }
    Ok(out)
}

/// An hdkey map's public key and chain code, or why it holds neither.
fn hdkey_public(v: &Value) -> Result<(PublicKey, [u8; 32]), Error> {
    let bad = || Error::Key;
    match v.get(2) {
        None | Some(Value::Bool(false)) => {}
        Some(Value::Bool(true)) => return Err(Error::Private),
        Some(_) => return Err(bad()),
    }
    let master = match v.get(1) {
        None | Some(Value::Bool(false)) => false,
        Some(Value::Bool(true)) => true,
        Some(_) => return Err(bad()),
    };
    let Some(Value::Bytes(key)) = v.get(3) else {
        return Err(bad());
    };
    // Key data starting 00 is a private key, with or without the flags:
    // BCR-2020-007's master key says only `is-master`.
    if key.first() == Some(&0) {
        return Err(Error::Private);
    }
    // A master key is private, so `is-master` on a public one is refused.
    if master {
        return Err(bad());
    }
    let Some(Value::Bytes(chain)) = v.get(4) else {
        return Err(Error::NoChainCode);
    };
    // Key data is 33 bytes; `from_slice` would also take an uncompressed key.
    if key.len() != 33 {
        return Err(bad());
    }
    let public_key = PublicKey::from_slice(key).map_err(|_| bad())?;
    let chain: [u8; 32] = chain.as_slice().try_into().map_err(|_| bad())?;
    Ok((public_key, chain))
}

fn hdkey(v: &Value) -> Result<Key, Error> {
    let bad = || Error::Key;
    let v = strip(v, HDKEY, HDKEY_UR).ok_or_else(bad)?;
    let (public_key, chain) = hdkey_public(v)?;
    // The use-info's network: 0 mainnet (the default), 1 Bitcoin testnet.
    let network = match v.get(5) {
        None => NetworkKind::Main,
        Some(u) => match strip(u, COININFO, COININFO_UR).ok_or_else(bad)? {
            u @ Value::Map(_) => {
                // The coin type is 0 or 1; 1 is read on any path, as
                // wallets write it (no comparison with the path).
                match u.get(1) {
                    None | Some(Value::Uint(0 | 1)) => {}
                    Some(_) => return Err(bad()),
                }
                match u.get(2) {
                    None | Some(Value::Uint(0)) => NetworkKind::Main,
                    Some(Value::Uint(1)) => NetworkKind::Test,
                    Some(_) => return Err(bad()),
                }
            }
            _ => return Err(bad()),
        },
    };
    let origin = v.get(6);
    let (origin_path, depth, last) = match origin {
        Some(o) => {
            let p = path(o)?;
            let comps = p.split('/').filter(|s| !s.is_empty()).count();
            // A stated depth is the number of steps given. BCR-2020-007
            // allows a larger one, a partial path, but Faraday writes
            // `[fingerprint/path]`: the master's fingerprint and the path
            // from it, so a partial path would read as a false full origin.
            let depth = match strip(o, KEYPATH, KEYPATH_UR).ok_or_else(bad)?.get(3) {
                None => comps,
                Some(Value::Uint(d)) if *d as usize == comps => comps,
                Some(_) => return Err(Error::Path),
            };
            let last = p.rsplit('/').next().unwrap_or("");
            let child = match last.strip_suffix('h') {
                Some(n) => n
                    .parse()
                    .ok()
                    .and_then(|n| ChildNumber::from_hardened_idx(n).ok()),
                None => last
                    .parse()
                    .ok()
                    .and_then(|n| ChildNumber::from_normal_idx(n).ok()),
            };
            // An origin is the steps taken: a wildcard is not one.
            if p.contains('*') {
                return Err(Error::Path);
            }
            (Some(p), depth, child)
        }
        None => (None, 0, None),
    };
    // Parent fingerprint 0 is read: a key at depth 0 has one, and
    // SeedSigner writes it. BIP-32 allows no other at depth 0.
    let parent = match v.get(8) {
        None => [0; 4],
        Some(Value::Uint(f)) if *f <= u64::from(u32::MAX) => (*f as u32).to_be_bytes(),
        Some(_) => return Err(bad()),
    };
    if depth == 0 && parent != [0; 4] {
        return Err(bad());
    }
    let xpub = Xpub {
        network,
        depth: u8::try_from(depth).map_err(|_| bad())?,
        parent_fingerprint: Fingerprint::from(parent),
        child_number: last.unwrap_or(ChildNumber::from_normal_idx(0).map_err(|_| bad())?),
        public_key,
        chain_code: ChainCode::from(chain),
    };
    let source = match origin {
        Some(o) => match strip(o, KEYPATH, KEYPATH_UR).ok_or_else(bad)?.get(2) {
            None => None,
            // The specification says `uint32 .ne 0`, and 0 is the usual
            // placeholder for an unknown master: not an origin.
            Some(Value::Uint(f)) if (1..=u64::from(u32::MAX)).contains(f) => Some(*f as u32),
            Some(_) => return Err(Error::Path),
        },
        None => None,
    };
    // With one step the parent is the master: the two fingerprints, where
    // both are stated, are identical.
    if let (Some(s), Some(Value::Uint(p)), 1) = (source, v.get(8), depth)
        && u64::from(s) != *p
    {
        return Err(Error::Path);
    }
    let origin = match (source, origin_path) {
        (Some(fp), Some(p)) => Some(format!("{fp:08x}{p}")),
        _ => None,
    };
    let children = v.get(7).map(path).transpose()?;
    Ok(Key {
        xpub,
        origin,
        children,
    })
}

/// A script expression's descriptor.
fn output(v: &Value) -> Result<String, Error> {
    let (tag, inner) = match v {
        Value::Tag(OUTPUT, inner) => match &**inner {
            Value::Tag(t, i) => (*t, &**i),
            _ => return Err(Error::Wallet),
        },
        Value::Tag(t, i) => (*t, &**i),
        _ => return Err(Error::Wallet),
    };
    let key = |v: &Value| -> Result<String, Error> {
        match v {
            Value::Tag(HDKEY | HDKEY_UR, _) | Value::Map(_) => hdkey(v).map(|k| k.in_wallet()),
            _ => Err(Error::NotExtended),
        }
    };
    let multi = |name: &str, v: &Value| -> Result<String, Error> {
        let Some(Value::Uint(m)) = v.get(1) else {
            return Err(Error::NoThreshold);
        };
        let Some(Value::Array(keys)) = v.get(2) else {
            return Err(Error::NoMultisigKeys);
        };
        let keys: Vec<String> = keys.iter().map(key).collect::<Result<_, _>>()?;
        Ok(format!("{name}({m},{})", keys.join(",")))
    };
    Ok(match tag {
        400 => format!("sh({})", output(inner)?),
        401 => format!("wsh({})", output(inner)?),
        403 => format!("pkh({})", key(inner)?),
        404 => format!("wpkh({})", key(inner)?),
        409 => format!("tr({})", key(inner)?),
        406 => multi("multi", inner)?,
        407 => multi("sortedmulti", inner)?,
        HDKEY | HDKEY_UR => key(v)?,
        t => return Err(Error::Script(t)),
    })
}

/// A crypto-account's keys, one line each with its script above it, the
/// native SegWit multisig key first: a cosigner file.
fn account(v: &Value) -> Result<String, Error> {
    let v = strip(v, ACCOUNT, ACCOUNT).ok_or(Error::Wallet)?;
    let Some(Value::Uint(fp)) = v
        .get(1)
        .filter(|f| matches!(f, Value::Uint(f) if *f <= u64::from(u32::MAX)))
    else {
        return Err(Error::NoFingerprint);
    };
    let Some(Value::Array(outputs)) = v.get(2) else {
        return Err(Error::NoAccountKeys);
    };
    let mut lines: Vec<(u8, String, String)> = Vec::new();
    for o in outputs {
        let o = match o {
            Value::Tag(OUTPUT, inner) => &**inner,
            o => o,
        };
        // Every entry is a script expression; one that is not is refused,
        // not left out of the account.
        let Value::Tag(tag, inner) = o else {
            return Err(Error::Wallet);
        };
        // The key under any number of script wrappers, and the script.
        if !(400..=410).contains(tag) {
            return Err(Error::Wallet);
        }
        let (script, mut at) = (*tag, &**inner);
        let mut wrap = vec![script];
        while let Value::Tag(t, i) = at {
            if *t == HDKEY || *t == HDKEY_UR {
                break;
            }
            // Only the registry's script tags wrap a key.
            if !(400..=410).contains(t) {
                return Err(Error::Wallet);
            }
            wrap.push(*t);
            at = i;
        }
        let k = hdkey(at)?;
        let (rank, name) = match wrap.as_slice() {
            [401, ..] => (0, "wsh"),
            [400, 401, ..] => (1, "sh-wsh"),
            [404, ..] => (2, "wpkh"),
            [400, 404, ..] => (3, "sh-wpkh"),
            [409, ..] => (4, "tr"),
            [403, ..] => (5, "pkh"),
            [400, ..] => (6, "sh"),
            _ => (7, "other"),
        };
        lines.push((rank, name.to_string(), k.origin_text()));
    }
    if lines.is_empty() {
        return Err(Error::NoKeyRead);
    }
    lines.sort_by_key(|l| l.0);
    let mut out = format!("# Account keys of {fp:08x} (ur:crypto-account)\n");
    for (_, name, key) in lines {
        out.push_str(&format!("# {name}\n{key}\n"));
    }
    Ok(out)
}

/// One key as the `crypto-account` SeedSigner shows for it, as the
/// Wallets tab writes it: the master fingerprint, and the key under the script it was
/// derived for (`wsh` for a multisig key, `wpkh` for a single-key one).
pub fn account_cbor(
    master: [u8; 4],
    origin_path: &[ChildNumber],
    xpub: &Xpub,
    wpkh: bool,
) -> Vec<u8> {
    use crate::cbor::{bytes, head};
    let uint = |n: u64| head(0, n);
    let mut comps = Vec::new();
    for c in origin_path {
        let (i, h) = match c {
            ChildNumber::Normal { index } => (*index, false),
            ChildNumber::Hardened { index } => (*index, true),
        };
        comps.extend(uint(u64::from(i)));
        comps.push(if h { 0xf5 } else { 0xf4 });
    }
    let mut keypath = head(5, 3);
    keypath.extend(uint(1));
    keypath.extend(head(4, (origin_path.len() * 2) as u64));
    keypath.extend(comps);
    keypath.extend(uint(2));
    keypath.extend(uint(u64::from(u32::from_be_bytes(master))));
    keypath.extend(uint(3));
    keypath.extend(uint(origin_path.len() as u64));
    let test = xpub.network == NetworkKind::Test;
    let mut hd = head(5, if test { 5 } else { 4 });
    hd.extend(uint(3));
    hd.extend(bytes(&xpub.public_key.serialize()));
    hd.extend(uint(4));
    hd.extend(bytes(xpub.chain_code.as_bytes()));
    if test {
        // use-info: testnet. Its absence says mainnet.
        hd.extend(uint(5));
        hd.extend(head(6, 305));
        hd.extend(head(5, 1));
        hd.extend(uint(2));
        hd.extend(uint(1));
    }
    hd.extend(uint(6));
    hd.extend(head(6, KEYPATH));
    hd.extend(keypath);
    hd.extend(uint(8));
    hd.extend(uint(u64::from(u32::from_be_bytes(
        *xpub.parent_fingerprint.as_bytes(),
    ))));
    let mut out = head(5, 2);
    out.extend(uint(1));
    out.extend(uint(u64::from(u32::from_be_bytes(master))));
    out.extend(uint(2));
    out.extend(head(4, 1));
    out.extend(head(6, if wpkh { 404 } else { 401 }));
    out.extend(head(6, HDKEY));
    out.extend(hd);
    out
}
