//! Coldcard's JSON account export, read as a single-sig wallet.
//!
//! A Coldcard writes one file holding every account key it can offer:
//! the master fingerprint as `xfp`, one object per single-sig standard
//! (`bip44`, `bip49`, `bip84`, `bip86`, each with a `deriv` and an
//! `xpub`), and the multisig account keys as `p2wsh_deriv`/`p2wsh` and
//! their nested pair. Each carries its origin, so a wallet built from
//! one states the master it came from.
//!
//! What [`parse`] builds is the single-sig wallet of whichever of the
//! four standards the file offers, taking the first of `bip84`, `bip86`,
//! `bip49`, `bip44` that is there — the order people use them in. The
//! multisig keys are a cosigner's contribution to a policy rather than a
//! wallet of their own, so [`cosigner_key`] reads one of them as a key
//! expression for the wallet builder to put in a policy.
//!
//! The reader is a scanner over the text rather than a JSON parser: the
//! file is a flat object of string fields written by one program, and a
//! dependency for it would be a dependency for one file format.

use alloc::string::String;

use bitcoin::base58;

use crate::keys::{MultisigScriptType, ScriptType};
use crate::policy::{DESCRIPTOR_SUFFIX, Error, WalletPolicy};
use crate::xkey;

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
    let text = text.trim();
    text.starts_with('{')
        && string_field(text, "xfp").is_some()
        && ACCOUNTS
            .iter()
            .any(|(name, _)| object(text, name).is_some())
}

/// The single-sig wallet the export offers.
pub fn parse(text: &str) -> Result<WalletPolicy, Error> {
    let text = text.trim();
    let xfp = string_field(text, "xfp").ok_or(Error::Key)?;
    for (name, script) in ACCOUNTS {
        let Some(account) = object(text, name) else {
            continue;
        };
        let Some(xpub) = string_field(account, "xpub") else {
            continue;
        };
        // The account's own `xfp`, where it has one, is the master's:
        // Coldcard repeats it per account.
        let xfp = string_field(account, "xfp").unwrap_or(xfp);
        let deriv = string_field(account, "deriv")
            .ok_or(Error::Key)?
            .trim_start_matches('m')
            .trim_start_matches('/');
        // The file writes the fingerprint in upper case; a descriptor
        // writes hex in lower, and the two must compare as one string.
        let xfp = String::from(xfp).to_ascii_lowercase();
        let origin = if deriv.is_empty() {
            xfp
        } else {
            alloc::format!("{xfp}/{deriv}")
        };
        let key = alloc::format!("[{origin}]{xpub}{DESCRIPTOR_SUFFIX}");
        let template = match script {
            ScriptType::Legacy => alloc::format!("pkh({key})"),
            ScriptType::NestedSegwit => alloc::format!("sh(wpkh({key}))"),
            ScriptType::NativeSegwit => alloc::format!("wpkh({key})"),
            ScriptType::Taproot => alloc::format!("tr({key})"),
        };
        return WalletPolicy::from_descriptor(&template);
    }
    Err(Error::Template)
}

/// The `"name"` the export carries, where it has one that is not
/// empty. Coldcard writes none; other tools that write this file do,
/// and it is the wallet's name.
pub fn name(text: &str) -> Option<String> {
    let name = string_field(text.trim(), "name")?.trim();
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

/// SLIP-132 version bytes for a multisig account key: `Ypub` and `Zpub`
/// on mainnet, `Upub` and `Vpub` on the test networks. Coldcard writes
/// its multisig keys in these, and a descriptor takes `xpub` or `tpub`,
/// so they are swapped back to BIP-32's own versions.
const MULTISIG_VERSIONS: [([u8; 4], [u8; 4]); 4] = [
    ([0x02, 0x95, 0xb4, 0x3f], xkey::XPUB),
    ([0x02, 0xaa, 0x7e, 0xd3], xkey::XPUB),
    ([0x02, 0x42, 0x89, 0xef], xkey::TPUB),
    ([0x02, 0x57, 0x54, 0x83], xkey::TPUB),
];

/// The key expression `[fingerprint/path]xpub` the export offers for
/// `account`, which is one cosigner's contribution to a wallet policy.
///
/// The file names the master and the derivation of every account it
/// carries, so the key states its own origin and nothing is invented.
/// A file with no such account is [`Error::Key`].
pub fn cosigner_key(text: &str, account: Cosigner) -> Result<String, Error> {
    let text = text.trim();
    let xfp = string_field(text, "xfp").ok_or(Error::Key)?;
    let (deriv, key) = match account {
        Cosigner::Taproot => {
            let object = object(text, "bip86").ok_or(Error::Key)?;
            (
                string_field(object, "deriv").ok_or(Error::Key)?,
                String::from(string_field(object, "xpub").ok_or(Error::Key)?),
            )
        }
        Cosigner::Multisig(script) => {
            let (_, deriv, key) = MULTISIG
                .iter()
                .find(|(s, _, _)| *s == script)
                .expect("both BIP-48 script types are listed");
            (
                string_field(text, deriv).ok_or(Error::Key)?,
                plain_xpub(string_field(text, key).ok_or(Error::Key)?)?,
            )
        }
    };
    let deriv = deriv.trim_start_matches('m').trim_start_matches('/');
    // The file writes the fingerprint in upper case; a descriptor writes
    // hex in lower, and the two must compare as one string.
    let xfp = String::from(xfp).to_ascii_lowercase();
    let origin = if deriv.is_empty() {
        xfp
    } else {
        alloc::format!("{xfp}/{deriv}")
    };
    Ok(alloc::format!("[{origin}]{key}"))
}

/// A multisig account key as `xpub` or `tpub`, whichever SLIP-132
/// spelling the file wrote it in. A key already in BIP-32's own
/// encoding is returned as it stands.
fn plain_xpub(key: &str) -> Result<String, Error> {
    if xkey::decode_xpub(key).is_ok() {
        return Ok(String::from(key));
    }
    let mut bytes = xkey::payload(key).map_err(|_| Error::Key)?;
    let version = xkey::version(&bytes);
    let plain = MULTISIG_VERSIONS
        .iter()
        .find(|(v, _)| *v == version)
        .map(|(_, plain)| *plain)
        .ok_or(Error::Key)?;
    bytes[..4].copy_from_slice(&plain);
    // The rules every extended key text is read by apply here too.
    xkey::xpub_from_bytes(&bytes).map_err(|_| Error::Key)?;
    Ok(base58::encode_check(&bytes))
}

/// The value of `"name": "…"`, unescaped only as far as these files go,
/// which is not at all: every value here is hex, a derivation path or a
/// base58 key.
fn string_field<'a>(text: &'a str, name: &str) -> Option<&'a str> {
    let rest = after_name(text, name)?;
    let rest = rest.strip_prefix('"')?;
    let end = rest.find('"')?;
    Some(&rest[..end])
}

/// The text of the object at `"name": { … }`, braces included.
fn object<'a>(text: &'a str, name: &str) -> Option<&'a str> {
    let rest = after_name(text, name)?;
    if !rest.starts_with('{') {
        return None;
    }
    let mut depth = 0usize;
    for (i, c) in rest.char_indices() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(&rest[..=i]);
                }
            }
            _ => {}
        }
    }
    None
}

/// What follows `"name":`, with the whitespace around the colon gone.
fn after_name<'a>(text: &'a str, name: &str) -> Option<&'a str> {
    let mut from = 0;
    let quoted = alloc::format!("\"{name}\"");
    while let Some(at) = text[from..].find(&quoted) {
        let after = &text[from + at + quoted.len()..];
        let after = after.trim_start();
        if let Some(after) = after.strip_prefix(':') {
            return Some(after.trim_start());
        }
        from += at + quoted.len();
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keys::MultisigScriptType;

    /// A Coldcard export as the device writes it, cut to the fields this
    /// reader uses.
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
    "first": "1Ehj3fBhGRdxwYfdpSNVPPbpvEVEHcNmiJ"
  },
  "bip84": {
    "name": "p2wpkh",
    "xfp": "0F056943",
    "deriv": "m/84'/0'/0'",
    "xpub": "xpub6CRQzb8u9dmMcq5XAwwRn9gcoYCjndJkhKgD11WKzbVGd932UmrExWFxCAvRnDN3ez6ZujLmMvmLBaSWdfWVn75L83Qxu1qSX4fJNrJg2Gt",
    "_pub": "zpub6rFR7y4Q2AijF6Gk1bofHLs1d66hKFamhXWdWBup1Em25wfabZqkDqvaieV63fDQFaYmaatCG7jVNUpUiM2hAMo6SAVHcrUpSnHDpNzucB7",
    "first": "bc1qm97vqzgj934vnaq9s53ynkyf9dgr05rargr04n"
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
