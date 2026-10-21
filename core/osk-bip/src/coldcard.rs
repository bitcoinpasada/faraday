//! Coldcard's JSON account export, read as a single-sig wallet, and the
//! account keys it and Coldcard's multisig key file offer a cosigner.
//!
//! A Coldcard writes one file holding every account key it can offer:
//! the master fingerprint as `xfp`, one object per single-sig standard
//! (`bip44`, `bip49`, `bip84`, `bip86`, each with a `deriv`, an `xpub`
//! and the account's `first` address), and the multisig account keys as
//! `p2wsh_deriv`/`p2wsh` and their nested pair (Coldcard's
//! `docs/generic-wallet-export.md`). Nunchuk, Passport, Sparrow and
//! Unchained write the same file. Every key is rooted at the top-level
//! `xfp`, the master's: the `xfp` inside an account object is that
//! account key's own fingerprint, and an origin written with it would
//! match no seed.
//!
//! What [`read`] builds is the single-sig wallet of whichever of the
//! four standards the file offers, taking the first of `bip84`, `bip86`,
//! `bip49`, `bip44` that is there — the order people use them in — and
//! where the account states its `first` address, the wallet's own must be
//! it. The multisig keys are a cosigner's contribution to a policy rather
//! than a wallet of their own, so [`cosigner_key`] reads one of them as a
//! key expression for the wallet builder, and [`cosigner_keys`] every
//! account key a file offers.
//!
//! Every key's SLIP-132 version is checked against the script its field
//! or path names ([`crate::slip132`]), and each key is checked as every
//! extended key text is. The file is read as JSON ([`crate::json`]), so a
//! field is the one at its place in the file, escapes undone, and a file
//! that is not JSON, names a key twice or nests too deep is no export.
//! Faraday's since the fork (`docs/UPSTREAM.md` §3).

use alloc::string::String;
use alloc::vec::Vec;

use crate::json::{self, Value};
use crate::keys::{MultisigScriptType, Network, ScriptType};
use crate::policy::{DESCRIPTOR_SUFFIX, Error, PolicyKey, WalletPolicy};
use crate::slip132::{self, Script};

/// The four single-sig standards, in the order [`parse`] prefers them,
/// with the field each is written under.
const ACCOUNTS: &[(&str, ScriptType)] = &[
    ("bip84", ScriptType::NativeSegwit),
    ("bip86", ScriptType::Taproot),
    ("bip49", ScriptType::NestedSegwit),
    ("bip44", ScriptType::Legacy),
];

/// Whether `text` looks like one of these exports: a JSON object with a
/// master fingerprint and at least one account in it.
pub fn looks_like_export(text: &str) -> bool {
    let Some(file) = read_json(text) else {
        return false;
    };
    string_field(&file, "xfp").is_some()
        && ACCOUNTS
            .iter()
            .any(|(name, _)| object(&file, name).is_some())
}

/// Why an export gives no wallet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReadError {
    /// The file is no JSON object with a top-level `xfp`.
    NoXfp,
    /// No single-sig account has an `xpub`.
    NoAccount,
    /// The account (its field) has an `xpub` and no `deriv`.
    NoDeriv(&'static str),
    /// The account (its field) states a `first` address that is not the
    /// first receive address of the wallet its key makes, on any network.
    FirstAddress {
        /// The account's field, `bip84` and so on.
        account: &'static str,
        /// The address the file states.
        first: String,
    },
    /// The key does not make a wallet.
    Policy(Error),
}

/// The single-sig wallet the export offers, with the field it was read
/// from. Costs a JSON parse, a policy parse and an address derivation
/// per network until the stated first address is found.
pub fn read(text: &str) -> Result<(WalletPolicy, &'static str), ReadError> {
    let file = read_json(text).ok_or(ReadError::NoXfp)?;
    let xfp = string_field(&file, "xfp").ok_or(ReadError::NoXfp)?;
    for (name, script) in ACCOUNTS {
        let Some(account) = object(&file, name) else {
            continue;
        };
        let Some(xpub) = string_field(account, "xpub") else {
            continue;
        };
        let deriv = string_field(account, "deriv").ok_or(ReadError::NoDeriv(name))?;
        let origin = origin(xfp, deriv).ok_or(ReadError::NoDeriv(name))?;
        let key = alloc::format!("[{origin}]{xpub}{DESCRIPTOR_SUFFIX}");
        let template = match script {
            ScriptType::Legacy => alloc::format!("pkh({key})"),
            ScriptType::NestedSegwit => alloc::format!("sh(wpkh({key}))"),
            ScriptType::NativeSegwit => alloc::format!("wpkh({key})"),
            ScriptType::Taproot => alloc::format!("tr({key})"),
        };
        let policy = WalletPolicy::from_descriptor(&template).map_err(ReadError::Policy)?;
        if let Some(first) = string_field(account, "first") {
            let found = Network::ALL.iter().any(|&net| {
                policy
                    .address_at(net, false, 0)
                    .is_ok_and(|a| alloc::format!("{a}") == first)
            });
            if !found {
                return Err(ReadError::FirstAddress {
                    account: name,
                    first: String::from(first),
                });
            }
        }
        return Ok((policy, name));
    }
    Err(ReadError::NoAccount)
}

/// The single-sig wallet the export offers ([`read`]).
pub fn parse(text: &str) -> Result<WalletPolicy, Error> {
    read(text).map(|(policy, _)| policy).map_err(|e| match e {
        ReadError::NoAccount => Error::Template,
        ReadError::Policy(e) => e,
        _ => Error::Key,
    })
}

/// The `"name"` the export carries, where it has one that is not
/// empty. Coldcard writes none; other tools that write this file do,
/// and it is the wallet's name.
pub fn name(text: &str) -> Option<String> {
    let file = read_json(text)?;
    let name = string_field(&file, "name")?.trim();
    (!name.is_empty()).then(|| String::from(name))
}

/// Which account of an export a cosigner contributes: one of BIP-48's
/// two multisig accounts, or the BIP-86 account a MuSig2 wallet
/// aggregates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cosigner {
    /// A BIP-48 multisig account: `p2wsh` or `p2sh_p2wsh`.
    Multisig(MultisigScriptType),
    /// The BIP-86 account, which is `bip86` like any single-sig one.
    Taproot,
}

/// The multisig account keys, which Coldcard writes as two flat fields
/// rather than as an object.
const MULTISIG: [(MultisigScriptType, &str, &str); 2] = [
    (MultisigScriptType::NativeSegwit, "p2wsh_deriv", "p2wsh"),
    (
        MultisigScriptType::NestedSegwit,
        "p2sh_p2wsh_deriv",
        "p2sh_p2wsh",
    ),
];

/// The key expression `[fingerprint/path]xpub` the export offers for
/// `account`, which is one cosigner's contribution to a wallet policy.
///
/// The file names the master and the derivation of every account it
/// carries, so the key states its own origin and nothing is invented.
/// A file with no such account is [`Error::Key`].
pub fn cosigner_key(text: &str, account: Cosigner) -> Result<String, Error> {
    let file = read_json(text).ok_or(Error::Key)?;
    let xfp = string_field(&file, "xfp").ok_or(Error::Key)?;
    let (deriv, key) = match account {
        Cosigner::Taproot => {
            let object = object(&file, "bip86").ok_or(Error::Key)?;
            let key = string_field(object, "xpub").ok_or(Error::Key)?;
            (
                string_field(object, "deriv").ok_or(Error::Key)?,
                checked_key(key, Script::Taproot)?,
            )
        }
        Cosigner::Multisig(script) => {
            let (_, deriv, key) = MULTISIG
                .iter()
                .find(|(s, _, _)| *s == script)
                .expect("both BIP-48 script types are listed");
            let around = match script {
                MultisigScriptType::NativeSegwit => Script::NativeMultisig,
                MultisigScriptType::NestedSegwit => Script::NestedMultisig,
            };
            (
                string_field(&file, deriv).ok_or(Error::Key)?,
                checked_key(string_field(&file, key).ok_or(Error::Key)?, around)?,
            )
        }
    };
    let origin = origin(xfp, deriv).ok_or(Error::Key)?;
    Ok(alloc::format!("[{origin}]{key}"))
}

/// Every account key a Coldcard file offers a cosigner, each as
/// `[fingerprint/path]xpub` rooted at the file's top-level `xfp`: the
/// multisig key file's flat fields (`p2wsh`, `p2sh_p2wsh`, the
/// `p2wsh_p2sh` Coldcard has also written, and BIP-45's `p2sh`, each with
/// its `_deriv`), then the generic export's account objects (`bip48_2`,
/// `bip48_1`, `bip45`, `bip86`, `bip84`, `bip49`, `bip44`). A key whose
/// SLIP-132 version disagrees with its field or the purpose in its path
/// is left out, as is one that is no key. Costs a JSON parse and a base58
/// decode per key.
pub fn cosigner_keys(text: &str) -> Vec<String> {
    let Some(file) = read_json(text) else {
        return Vec::new();
    };
    let Some(master) = string_field(&file, "xfp") else {
        return Vec::new();
    };
    let key = |deriv: &str, xpub: &str, around: Option<Script>| {
        let line =
            slip132::key_line(&alloc::format!("[{}]{xpub}", origin(master, deriv)?))?.ok()?;
        if let Some(around) = around
            && !slip132::agrees(line.named, around)
        {
            return None;
        }
        PolicyKey::parse(&line.text).ok().map(|k| k.key_text())
    };
    let mut keys = Vec::new();
    for (field, around) in [
        ("p2wsh", Script::NativeMultisig),
        ("p2sh_p2wsh", Script::NestedMultisig),
        ("p2wsh_p2sh", Script::NestedMultisig),
        ("p2sh", Script::LegacyMultisig),
    ] {
        if let (Some(d), Some(x)) = (
            string_field(&file, &alloc::format!("{field}_deriv")),
            string_field(&file, field),
        ) && let Some(k) = key(d, x, Some(around))
        {
            keys.push(k);
        }
    }
    for field in [
        "bip48_2", "bip48_1", "bip45", "bip86", "bip84", "bip49", "bip44",
    ] {
        if let Some(account) = object(&file, field)
            && let (Some(d), Some(x)) = (
                string_field(account, "deriv"),
                string_field(account, "xpub"),
            )
            && let Some(k) = key(d, x, None)
        {
            keys.push(k);
        }
    }
    keys
}

/// `fingerprint/path` from the file's `xfp` and an account's `deriv`
/// (`m/84'/0'/0'`), the fingerprint in lower case as a descriptor writes
/// it; `None` for a `deriv` that names no step, which would claim the
/// account key is the master.
fn origin(xfp: &str, deriv: &str) -> Option<String> {
    let path = deriv.trim_start_matches('m').trim_start_matches('/');
    (!path.is_empty()).then(|| alloc::format!("{}/{path}", xfp.to_ascii_lowercase()))
}

/// An account key as `xpub` or `tpub`, whichever spelling the file wrote
/// it in: BIP-32's own, or the SLIP-132 version that names `around`
/// (`Zpub`/`Vpub` for P2WSH, `Ypub`/`Upub` for P2SH-P2WSH). A version
/// naming another script is refused, never converted
/// ([`slip132::agrees`]), and the key is checked as every extended key
/// text is.
fn checked_key(key: &str, around: Script) -> Result<String, Error> {
    let read = slip132::decode(key).map_err(|_| Error::Key)?;
    if !slip132::agrees(read.named, around) {
        return Err(Error::Key);
    }
    Ok(read.plain)
}

/// The text read as JSON, when it is a JSON object.
fn read_json(text: &str) -> Option<Value> {
    json::parse(text)
        .ok()
        .filter(|v| matches!(v, Value::Object(_)))
}

/// The string member `name` of an object.
fn string_field<'a>(object: &'a Value, name: &str) -> Option<&'a str> {
    object.get(name)?.as_str()
}

/// The object member `name` of an object.
fn object<'a>(object: &'a Value, name: &str) -> Option<&'a Value> {
    object.get(name).filter(|v| matches!(v, Value::Object(_)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keys::MultisigScriptType;

    /// A Coldcard export as the device writes it, cut to the fields this
    /// reader uses. Each `first` is its account key's first receive
    /// address, which [`read`] checks (the file as upstream wrote it
    /// stated addresses of no key here).
    const EXPORT: &str = r#"{
  "chain": "BTC",
  "xfp": "0F056943",
  "account": 0,
  "xpub": "xpub661MyMwAqRbcGVQTLtBFzc3ENvyZHoUEhWRdGwoqLZaf5wXP9VcDY2VJV7usvsFLZz2RUTVhCVXYXc3S8zpLyAFbDFcfrZUVyR4pjvUesBE",
  "bip44": {
    "name": "p2pkh",
    "xfp": "0F056943",
    "deriv": "m/44'/0'/0'",
    "xpub": "xpub6Br37sWxruYfT8ASpCjVHKGwgdnYFEn98DwiN76i2oyY6fgH1LAPmmDcF46xjxJr22gw4jmVjTE2E3URMnRPEPYyo1zoPSUba563ESMXCeb",
    "first": "13XmQRpaYhwGrzhrtoUcD8H2cUiK7Az8fK"
  },
  "bip84": {
    "name": "p2wpkh",
    "xfp": "0F056943",
    "deriv": "m/84'/0'/0'",
    "xpub": "xpub6CRQzb8u9dmMcq5XAwwRn9gcoYCjndJkhKgD11WKzbVGd932UmrExWFxCAvRnDN3ez6ZujLmMvmLBaSWdfWVn75L83Qxu1qSX4fJNrJg2Gt",
    "_pub": "zpub6rFR7y4Q2AijF6Gk1bofHLs1d66hKFamhXWdWBup1Em25wfabZqkDqvaieV63fDQFaYmaatCG7jVNUpUiM2hAMo6SAVHcrUpSnHDpNzucB7",
    "first": "bc1qrn466525s5kduhtp54qvc4jnj4cukufe35hzpq"
  },
  "p2wsh_deriv": "m/48'/0'/0'/2'",
  "p2wsh": "Zpub74Jru6aftwwHxCUCWEvP6DgrfFsdA4U6ZRtQ5i8qJpMcC39yZGv3egBhQfV3MS9pZtH5z8iV5qWkJsK6ESs6mSzt4qvGhzJxPeeVS2e1zUG"
}"#;

    /// The file names the master and the derivation of every account, so
    /// the wallet it makes states both. SegWit is the account a person
    /// means by "my wallet", so it is the one built.
    #[test]
    fn a_coldcard_export_is_the_segwit_account_of_its_master() {
        assert!(looks_like_export(EXPORT));
        let policy = parse(EXPORT).unwrap();
        assert_eq!(policy.script_type(), ScriptType::NativeSegwit);
        let key = &policy.keys()[0];
        assert_eq!(
            key.fingerprint()
                .expect("the export names a master")
                .to_hex(),
            *b"0f056943"
        );
        assert_eq!(
            policy.to_descriptor(),
            "wpkh([0f056943/84'/0'/0']xpub6CRQzb8u9dmMcq5XAwwRn9gcoYCjndJkhKgD11WKzbVGd932UmrExWFxCAvRnDN3ez6ZujLmMvmLBaSWdfWVn75L83Qxu1qSX4fJNrJg2Gt/<0;1>/*)"
        );
    }

    /// A file with only a legacy account is that account's wallet.
    #[test]
    fn an_export_of_one_account_is_that_account() {
        const ONLY_44: &str = r#"{"xfp":"0F056943","bip44":{"deriv":"m/44'/0'/0'","xpub":"xpub6Br37sWxruYfT8ASpCjVHKGwgdnYFEn98DwiN76i2oyY6fgH1LAPmmDcF46xjxJr22gw4jmVjTE2E3URMnRPEPYyo1zoPSUba563ESMXCeb"}}"#;
        assert!(looks_like_export(ONLY_44));
        assert_eq!(parse(ONLY_44).unwrap().script_type(), ScriptType::Legacy);
    }

    /// A whole export as a Coldcard writes one, for the BIP-39 test
    /// mnemonic `abandon` × 11 + `about` on mainnet: every account key
    /// is that master's at the derivation beside it, and the multisig
    /// keys are in the SLIP-132 spellings the device uses.
    const MULTISIG_EXPORT: &str = r#"{
  "chain": "BTC",
  "xfp": "73C5DA0A",
  "account": 0,
  "bip86": {
    "name": "p2tr",
    "xfp": "73C5DA0A",
    "deriv": "m/86'/0'/0'",
    "xpub": "xpub6BgBgsespWvERF3LHQu6CnqdvfEvtMcQjYrcRzx53QJjSxarj2afYWcLteoGVky7D3UKDP9QyrLprQ3VCECoY49yfdDEHGCtMMj92pReUsQ"
  },
  "p2wsh_deriv": "m/48'/0'/0'/2'",
  "p2wsh": "Zpub74Jru6aftwwHxCUCWEvP6DgrfFsdA4U6ZRtQ5i8qJpMcC39yZGv3egBhQfV3MS9pZtH5z8iV5qWkJsK6ESs6mSzt4qvGhzJxPeeVS2e1zUG",
  "p2sh_p2wsh_deriv": "m/48'/0'/0'/1'",
  "p2sh_p2wsh": "Ypub6jUbbRukkGPp4DgJDD4HL2NKkSZ1UPk111mg59XtJRQZHvJ6XqvJzrntik9U4jCFQkgrBqevdKLPMdYZXU9KAGhKpMhW5XujwqiQ7Csmm4Z"
}"#;

    /// A cosigner's contribution is the account key with the origin the
    /// file states, and the SLIP-132 spelling Coldcard writes a multisig
    /// key in is the same key as its `xpub` form.
    #[test]
    fn a_cosigner_key_carries_the_origin_the_export_states() {
        assert_eq!(
            cosigner_key(
                MULTISIG_EXPORT,
                Cosigner::Multisig(MultisigScriptType::NativeSegwit)
            )
            .unwrap(),
            "[73c5da0a/48'/0'/0'/2']xpub6DkFAXWQ2dHxq2vatrt9qyA3bXYU4ToWQwCHbf5XB2mSTexcHZCeKS1VZYcPoBd5X8yVcbXFHJR9R8UCVpt82VX1VhR28mCyxUFL4r6KFrf"
        );
        assert_eq!(
            cosigner_key(
                MULTISIG_EXPORT,
                Cosigner::Multisig(MultisigScriptType::NestedSegwit)
            )
            .unwrap(),
            "[73c5da0a/48'/0'/0'/1']xpub6DkFAXWQ2dHxnMKoSBogHrw1rgNJKR4umdbnNVNTYeCGcduxWnNUHgGptqEQWPKRmeW4Zn4FHSbLMBKEWYaMDYu47Ytg6DdFnPNt8hwn5mE"
        );
        assert_eq!(
            cosigner_key(MULTISIG_EXPORT, Cosigner::Taproot).unwrap(),
            "[73c5da0a/86'/0'/0']xpub6BgBgsespWvERF3LHQu6CnqdvfEvtMcQjYrcRzx53QJjSxarj2afYWcLteoGVky7D3UKDP9QyrLprQ3VCECoY49yfdDEHGCtMMj92pReUsQ"
        );
    }

    /// An export with no multisig account has no cosigner key to give.
    #[test]
    fn an_export_without_the_account_offers_no_key() {
        assert_eq!(
            cosigner_key(EXPORT, Cosigner::Taproot),
            Err(Error::Key),
            "this file has no bip86 account"
        );
        assert_eq!(
            cosigner_key(EXPORT, Cosigner::Multisig(MultisigScriptType::NestedSegwit)),
            Err(Error::Key),
            "nor a nested multisig one"
        );
    }

    /// Other JSON is not this file.
    #[test]
    fn other_text_is_not_an_export() {
        assert!(!looks_like_export("{\"hello\": \"world\"}"));
        assert!(!looks_like_export("xfp bip84"));
        assert!(!looks_like_export("{\"xfp\":\"0F056943\"}"));
    }
}
