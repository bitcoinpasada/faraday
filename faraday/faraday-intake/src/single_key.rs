//! One account key in a JSON file of its own (`docs/INTEROP.md` §5 I2):
//! Cobo's and Keystone's `{"ExtPubKey","MasterFingerprint","AccountKeyPath"}`,
//! and Coldcard's file for Wasabi,
//! `{"MasterFingerprint","ExtPubKey","ColdCardFirmwareVersion"}`, which
//! names no path.
//!
//! Wasabi takes the key of its file as BIP-84's account 0, `84h/0h/0h` on
//! mainnet and `84h/1h/0h` on the test networks, and reads its
//! `MasterFingerprint` with its bytes reversed when
//! `ColdCardFirmwareVersion` is missing or `2.1.0`, the firmware that wrote
//! it backwards (`WalletWasabi/Helpers/ImportWalletHelper.cs`,
//! `GetKeyManagerByColdcardJson`). Both are read here as Wasabi reads
//! them, and the file Faraday writes carries a version Wasabi parses.

use crate::slip132::{self, Script};
use faraday_strings::{EN, fill};
use osk_bip::bitcoin::NetworkKind;
use osk_bip::bitcoin::bip32::ChildNumber;
use osk_bip::json::{self, Value};
use osk_bip::keys::ScriptType;
use osk_bip::policy::{Template, WalletPolicy};

/// The firmware version the Wasabi file Faraday writes states: the one
/// Coldcard Mk4's file in BlueWallet's tests states. Wasabi parses it as a
/// version and reads the fingerprint as written for any but `2.1.0`.
pub const WASABI_FIRMWARE: &str = "5.2.2";

/// Which file the key came in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    /// Cobo's or Keystone's, with `AccountKeyPath`.
    Cobo,
    /// Coldcard's file for Wasabi, BIP-84's account 0.
    Wasabi,
}

/// The key a single-key file holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SingleKey {
    /// Which file it came in.
    pub source: Source,
    /// The key as `[fingerprint/path]xpub`, its version BIP-32's own.
    pub key: String,
    /// The script type its path or its version names.
    pub script: Script,
}

impl SingleKey {
    /// The single-key wallet the key is, `/<0;1>/*` after it.
    pub fn descriptor(&self) -> Option<String> {
        let wrap = match self.script {
            Script::NativeSegwit => "wpkh(K/<0;1>/*)",
            Script::NestedSegwit => "sh(wpkh(K/<0;1>/*))",
            Script::Legacy => "pkh(K/<0;1>/*)",
            Script::Taproot => "tr(K/<0;1>/*)",
            _ => return None,
        };
        Some(wrap.replace('K', &self.key))
    }
}

/// The key of a single-key file, read from its JSON; `None` for JSON that
/// is no such file (no `ExtPubKey` and `MasterFingerprint` strings); the
/// refusal's text for one that is and cannot be read. Costs a base58
/// decode.
pub fn read(v: &Value) -> Option<Result<SingleKey, String>> {
    let xpub = v.get("ExtPubKey")?.as_str()?;
    let fp = v.get("MasterFingerprint")?.as_str()?;
    Some(read_fields(v, xpub, fp))
}

fn read_fields(v: &Value, xpub: &str, fp: &str) -> Result<SingleKey, String> {
    let bad = |why: &str| fill(EN.wallet_single_key_file, &[why]);
    let mut bytes = hex4(fp).ok_or_else(|| bad(EN.wallet_single_key_fingerprint))?;
    let decoded = slip132::decode(xpub).ok_or_else(|| bad(EN.wallet_single_key_xpub))?;
    let test = decoded.plain.starts_with("tpub");
    let (source, path) = match v.get("AccountKeyPath") {
        Some(p) => {
            let p = p
                .as_str()
                .and_then(path_h)
                .ok_or_else(|| bad(EN.wallet_single_key_path))?;
            (Source::Cobo, p)
        }
        None => {
            // Wasabi's reading of the fingerprint (module doc).
            let version = v
                .get("ColdCardFirmwareVersion")
                .and_then(Value::as_str)
                .map(|s| s.replace('X', ""));
            if version.is_none_or(|s| s == "2.1.0") {
                bytes.reverse();
            }
            let coin = if test { 1 } else { 0 };
            (Source::Wasabi, format!("84h/{coin}h/0h"))
        }
    };
    let fp: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
    let line = slip132::key_line(&format!("[{fp}/{path}]{xpub}"))
        .ok_or_else(|| bad(EN.wallet_single_key_xpub))?
        .map_err(|m| m.text())?;
    let script = Script::of_path(&path)
        .or(line.named)
        .ok_or_else(|| bad(EN.wallet_single_key_no_type))?;
    Ok(SingleKey {
        source,
        key: line.text,
        script,
    })
}

/// Eight hex digits as four bytes.
fn hex4(s: &str) -> Option<[u8; 4]> {
    let s = s.trim();
    if s.len() != 8 || !s.is_ascii() {
        return None;
    }
    let mut out = [0u8; 4];
    for (i, b) in out.iter_mut().enumerate() {
        *b = u8::from_str_radix(&s[2 * i..2 * i + 2], 16).ok()?;
    }
    Some(out)
}

/// A derivation path as an origin writes it, `84h/0h/0h`, from `m/84'/0'/0'`,
/// `84'/0'/0'` or `84h/0h/0h`; `None` for anything else, or no step.
pub(crate) fn path_h(p: &str) -> Option<String> {
    let p = p.trim();
    let p = p
        .strip_prefix("m/")
        .or_else(|| p.strip_prefix("M/"))
        .unwrap_or(p);
    let mut steps = Vec::new();
    for s in p.split('/') {
        let (n, hard) = match s.strip_suffix(['h', 'H', '\'']) {
            Some(n) => (n, true),
            None => (s, false),
        };
        if n.is_empty() || !n.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        let n: u32 = n.parse().ok()?;
        if n >= 1 << 31 {
            return None;
        }
        steps.push(if hard { format!("{n}h") } else { n.to_string() });
    }
    Some(steps.join("/"))
}

/// The wallet's single key, when Wasabi's file can carry it: a `wpkh`
/// wallet whose key has an origin at exactly `84h/0h/0h` (an `xpub`) or
/// `84h/1h/0h` (a `tpub`), the account Wasabi assumes.
fn wasabi_key(policy: &WalletPolicy) -> Option<&osk_bip::policy::PolicyKey> {
    if policy.template()
        != (Template::Single {
            script: ScriptType::NativeSegwit,
        })
    {
        return None;
    }
    let [k] = policy.keys() else {
        return None;
    };
    let coin = match k.xpub().network {
        NetworkKind::Main => 0,
        NetworkKind::Test => 1,
    };
    let path: Vec<ChildNumber> = k.path()?.into_iter().copied().collect();
    let want = [84, coin, 0].map(|index| ChildNumber::Hardened { index });
    (path[..] == want[..] && k.fingerprint().is_some()).then_some(k)
}

/// Whether [`wasabi_file`] writes a file for the wallet.
pub fn wasabi_writes(policy: &WalletPolicy) -> bool {
    wasabi_key(policy).is_some()
}

/// The wallet as Coldcard's file for Wasabi, in the shape Coldcard Mk4
/// writes it; `None` for a wallet Wasabi's file cannot carry
/// ([`wasabi_writes`]).
pub fn wasabi_file(policy: &WalletPolicy) -> Option<String> {
    let k = wasabi_key(policy)?;
    let fp: String = k
        .fingerprint()?
        .0
        .iter()
        .map(|b| format!("{b:02X}"))
        .collect();
    Some(format!(
        "{{\"ColdCardFirmwareVersion\": {}, \"MasterFingerprint\": {}, \"ExtPubKey\": {}}}\n",
        json::quote(WASABI_FIRMWARE),
        json::quote(&fp),
        json::quote(&k.xpub().to_string()),
    ))
}
