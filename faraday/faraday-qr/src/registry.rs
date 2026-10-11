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

use osk_bip::bitcoin::NetworkKind;
use osk_bip::bitcoin::bip32::{ChainCode, ChildNumber, Fingerprint, Xpub};
use osk_bip::bitcoin::secp256k1::PublicKey;

use crate::cbor::Value;

const HDKEY: u64 = 303;
const KEYPATH: u64 = 304;
const OUTPUT: u64 = 308;

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

/// Reads a UR of `ur_type` whose CBOR is `cbor`. `Ok(None)` for a type
/// this does not know; `Err` with the sentence a screen shows for one it
/// refuses.
pub fn read(ur_type: &str, cbor: &[u8]) -> Result<Option<Read>, String> {
    let bad = |what: &str| format!("The {what} in that code is not in the registry's shape");
    let v = || Value::decode(cbor).map_err(|_| bad(ur_type));
    match ur_type {
        "psbt" => match v()? {
            Value::Bytes(b) => Ok(Some(Read::Psbt(b))),
            _ => Err(bad("PSBT")),
        },
        "crypto-seed" | "seed" => Err(
            "That is a seed (ur:crypto-seed). Faraday takes a seed as its words or a SeedQR, in Add a key"
                .to_string(),
        ),
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
fn path(v: &Value) -> Result<String, String> {
    let bad = || "A key path in that code is not in the registry's shape".to_string();
    let Some(Value::Array(c)) = v.untag().1.get(1) else {
        return Ok(String::new());
    };
    if c.len() % 2 != 0 {
        return Err(bad());
    }
    let mut out = String::new();
    for pair in c.chunks(2) {
        let hardened = matches!(pair[1], Value::Bool(true));
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

fn hdkey(v: &Value) -> Result<Key, String> {
    let bad = || "The key in that code is not in the registry's shape".to_string();
    let v = v.untag().1;
    if matches!(v.get(2), Some(Value::Bool(true))) {
        return Err(
            "That is a private key. Faraday takes keys as their words or a SeedQR, in Add a key"
                .to_string(),
        );
    }
    let Some(Value::Bytes(key)) = v.get(3) else {
        return Err(bad());
    };
    let Some(Value::Bytes(chain)) = v.get(4) else {
        return Err(
            "The key in that code has no chain code, so no addresses come from it".to_string(),
        );
    };
    let public_key = PublicKey::from_slice(key).map_err(|_| bad())?;
    let chain: [u8; 32] = chain.as_slice().try_into().map_err(|_| bad())?;
    let network = match v.get(5).map(|u| u.untag().1.get(2)) {
        Some(Some(Value::Uint(1))) => NetworkKind::Test,
        _ => NetworkKind::Main,
    };
    let origin = v.get(6);
    let (origin_path, depth, last) = match origin {
        Some(o) => {
            let p = path(o)?;
            let comps = p.split('/').filter(|s| !s.is_empty()).count();
            let depth = match o.untag().1.get(3) {
                Some(Value::Uint(d)) => *d as usize,
                _ => comps,
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
            (Some(p), depth, child)
        }
        None => (None, 0, None),
    };
    let parent = match v.get(8) {
        Some(Value::Uint(f)) if *f <= u64::from(u32::MAX) => (*f as u32).to_be_bytes(),
        _ => [0; 4],
    };
    let xpub = Xpub {
        network,
        depth: u8::try_from(depth).map_err(|_| bad())?,
        parent_fingerprint: Fingerprint::from(parent),
        child_number: last.unwrap_or(ChildNumber::from_normal_idx(0).map_err(|_| bad())?),
        public_key,
        chain_code: ChainCode::from(chain),
    };
    let source = origin.and_then(|o| match o.untag().1.get(2) {
        Some(Value::Uint(f)) if *f <= u64::from(u32::MAX) => Some(*f as u32),
        _ => None,
    });
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
fn output(v: &Value) -> Result<String, String> {
    let (tag, inner) = match v {
        Value::Tag(OUTPUT, inner) => match &**inner {
            Value::Tag(t, i) => (*t, &**i),
            _ => return Err("A wallet in that code is not in the registry's shape".to_string()),
        },
        Value::Tag(t, i) => (*t, &**i),
        _ => return Err("A wallet in that code is not in the registry's shape".to_string()),
    };
    let key = |v: &Value| -> Result<String, String> {
        match v {
            Value::Tag(HDKEY, _) | Value::Map(_) => hdkey(v).map(|k| k.in_wallet()),
            _ => {
                Err("A key in that wallet is not an extended key, which Faraday reads".to_string())
            }
        }
    };
    let multi = |name: &str, v: &Value| -> Result<String, String> {
        let Some(Value::Uint(m)) = v.get(1) else {
            return Err("A multisig in that code has no threshold".to_string());
        };
        let Some(Value::Array(keys)) = v.get(2) else {
            return Err("A multisig in that code has no keys".to_string());
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
        HDKEY => key(v)?,
        t => return Err(format!("A wallet script (tag {t}) Faraday does not read")),
    })
}

/// A crypto-account's keys, one line each with its script above it, the
/// native SegWit multisig key first: a cosigner file.
fn account(v: &Value) -> Result<String, String> {
    let v = v.untag().1;
    let Some(Value::Uint(fp)) = v.get(1) else {
        return Err("The account in that code has no master fingerprint".to_string());
    };
    let Some(Value::Array(outputs)) = v.get(2) else {
        return Err("The account in that code has no keys".to_string());
    };
    let mut lines: Vec<(u8, String, String)> = Vec::new();
    for o in outputs {
        let o = match o {
            Value::Tag(OUTPUT, inner) => &**inner,
            o => o,
        };
        let Value::Tag(tag, inner) = o else { continue };
        // The key under any number of script wrappers, and the script.
        let (script, mut at) = (*tag, &**inner);
        let mut wrap = vec![script];
        while let Value::Tag(t, i) = at {
            if *t == HDKEY {
                break;
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
        return Err("The account in that code holds no key Faraday reads".to_string());
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
