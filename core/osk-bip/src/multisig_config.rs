//! The text file coordinators hand a signer to register a multisig
//! wallet: Coldcard's format, which Sparrow, Nunchuk and Specter all
//! write and read.
//!
//! ```text
//! Name: My wallet
//! Policy: 2 of 3
//! Derivation: m/48'/1'/0'/2'
//! Format: P2WSH
//!
//! 73c5da0a: tpub…
//! 3f635a63: tpub…
//! b8688df1: tpub…
//! ```
//!
//! The file states the quorum, the script form and one
//! `fingerprint: xpub` line per cosigner. A `Derivation:` line applies
//! to every key after it, so a wallet whose cosigners sit at different
//! paths writes one before each key. `#` comments and blank lines are
//! ignored, and `Name:` is the wallet's name, which the device shows
//! on Home in place of the wallet's shape.
//!
//! Every coordinator that writes this file sorts the keys in the script,
//! so the policy built here is always `sortedmulti`.
//!
//! The keys may be written as plain `xpub`/`tpub`, or with SLIP-132
//! version bytes: the single-signature `ypub`/`zpub`/`upub`/`vpub`, or
//! the multisignature `Ypub`/`Zpub`/`Upub`/`Vpub` Coldcard, Nunchuk and
//! Specter write for P2SH-P2WSH and P2WSH. The version bytes carry no
//! information the `Format:` line does not, so every key is read back
//! to its BIP-32 form before the policy is built, and the wallet is
//! the same whichever spelling the file used.

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use bitcoin::base58;

use crate::policy::{Error, WalletPolicy};
use crate::xkey::{self, TPUB, XPUB};

/// SLIP-132's multisignature version bytes, which the single-signature
/// table in [`crate::slip132`] leaves out: `Ypub`, `Zpub` on mainnet and
/// `Upub`, `Vpub` on test networks, each paired with the plain version
/// the key is read back to.
const MULTISIG_VERSIONS: [([u8; 4], [u8; 4]); 4] = [
    ([0x02, 0x95, 0xb4, 0x3f], XPUB), // Ypub
    ([0x02, 0xaa, 0x7e, 0xd3], XPUB), // Zpub
    ([0x02, 0x42, 0x89, 0xef], TPUB), // Upub
    ([0x02, 0x57, 0x54, 0x83], TPUB), // Vpub
];

/// Reads a coordinator's multisig config as a wallet policy.
pub fn parse(text: &str) -> Result<WalletPolicy, Error> {
    parse_named(text).map(|(policy, _)| policy)
}

/// The same, with the `Name:` line the file carries, where it has one
/// and it is not empty.
pub fn parse_named(text: &str) -> Result<(WalletPolicy, Option<String>), Error> {
    let mut name: Option<String> = None;
    let mut threshold = 0usize;
    let mut total = 0usize;
    let mut wrapper: Option<&str> = None;
    let mut derivation: Option<String> = None;
    let mut keys: Vec<String> = Vec::new();

    for line in text.lines() {
        let line = match line.split_once('#') {
            Some((head, _)) => head,
            None => line,
        }
        .trim();
        if line.is_empty() {
            continue;
        }
        let Some((label, value)) = line.split_once(':') else {
            return Err(Error::Key);
        };
        let (label, value) = (label.trim(), value.trim());
        if label.eq_ignore_ascii_case("name") {
            if !value.is_empty() {
                name = Some(String::from(value));
            }
            continue;
        } else if label.eq_ignore_ascii_case("policy") {
            let (m, n) = quorum(value).ok_or(Error::Threshold)?;
            threshold = m;
            total = n;
        } else if label.eq_ignore_ascii_case("derivation") {
            derivation = Some(String::from(path_body(value)));
        } else if label.eq_ignore_ascii_case("format") {
            wrapper = Some(wrapper_of(value).ok_or(Error::Template)?);
        } else if is_fingerprint(label) {
            let path = derivation.as_deref().ok_or(Error::Derivation)?;
            let xpub = plain_xpub(value)?;
            keys.push(format!("[{}/{path}]{xpub}", label.to_ascii_lowercase()));
        } else {
            return Err(Error::Key);
        }
    }

    let wrapper = wrapper.ok_or(Error::Template)?;
    if keys.is_empty() {
        return Err(Error::Placeholder);
    }
    if total != keys.len() {
        return Err(Error::Placeholder);
    }
    if threshold == 0 || threshold > keys.len() {
        return Err(Error::Threshold);
    }

    let mut args = String::new();
    for i in 0..keys.len() {
        if i > 0 {
            args.push(',');
        }
        args.push_str(&format!("@{i}{}", crate::policy::TEMPLATE_SUFFIX));
    }
    let template = match wrapper {
        "sh(wsh" => format!("sh(wsh(sortedmulti({threshold},{args})))"),
        "sh" => format!("sh(sortedmulti({threshold},{args}))"),
        _ => format!("wsh(sortedmulti({threshold},{args}))"),
    };
    let refs: Vec<&str> = keys.iter().map(String::as_str).collect();
    Ok((WalletPolicy::from_parts(&template, &refs)?, name))
}

/// Whether `text` reads as one of these files: the first line that says
/// anything is one of the four headers the format defines.
pub fn looks_like_config(text: &str) -> bool {
    for line in text.lines() {
        let line = match line.split_once('#') {
            Some((head, _)) => head,
            None => line,
        }
        .trim();
        if line.is_empty() {
            continue;
        }
        let Some((label, _)) = line.split_once(':') else {
            return false;
        };
        return ["name", "policy", "derivation", "format"]
            .iter()
            .any(|h| label.trim().eq_ignore_ascii_case(h));
    }
    false
}

/// `2 of 3`, and the `2/3` some coordinators write instead.
fn quorum(value: &str) -> Option<(usize, usize)> {
    let (m, n) = match value.split_once(" of ") {
        Some(parts) => parts,
        None => value.split_once('/')?,
    };
    Some((m.trim().parse().ok()?, n.trim().parse().ok()?))
}

/// The path without its `m/` head, which is how a key origin writes it.
fn path_body(value: &str) -> &str {
    value
        .trim()
        .trim_start_matches(['m', 'M'])
        .trim_start_matches('/')
}

/// The wrapper a `Format:` value names.
fn wrapper_of(value: &str) -> Option<&'static str> {
    let value = value.trim();
    for (name, wrapper) in [
        ("P2WSH", "wsh"),
        ("P2SH-P2WSH", "sh(wsh"),
        ("P2WSH-P2SH", "sh(wsh"),
        ("P2SH", "sh"),
    ] {
        if value.eq_ignore_ascii_case(name) {
            return Some(wrapper);
        }
    }
    None
}

/// The key as `xpub`/`tpub`, whichever SLIP-132 spelling the file used.
/// Only the version bytes change, and the key is checked as strictly as
/// any other extended key the device reads.
fn plain_xpub(value: &str) -> Result<String, Error> {
    let mut bytes = xkey::payload(value).map_err(|_| Error::Key)?;
    let version = xkey::version(&bytes);
    let plain = if version == XPUB || version == TPUB {
        version
    } else if let Some((_, plain)) = MULTISIG_VERSIONS.iter().find(|(v, _)| *v == version) {
        *plain
    } else {
        let (xpub, _) = crate::slip132::decode_xpub(value).map_err(|_| Error::Key)?;
        return Ok(alloc::format!("{xpub}"));
    };
    bytes[..4].copy_from_slice(&plain);
    xkey::xpub_from_bytes(&bytes).map_err(|_| Error::Key)?;
    Ok(base58::encode_check(&bytes))
}

/// Whether a label is a master fingerprint: eight hex characters.
fn is_fingerprint(label: &str) -> bool {
    label.len() == 8 && label.bytes().all(|b| b.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;

    const CONFIG: &str = "\
Name: Demo
Policy: 2 of 2
Derivation: m/48'/1'/0'/2'

73c5da0a: tpubDFH9dgzveyD8zTbPUFuLrGmCydNvxehyNdUXKJAQN8x4aZ4j6UZqGfnqFrD4NqyaTVGKbvEW54tsvPTK2UoSbCC1PJY8iCNiwTL3RWZEheQ
3f635a63: tpubDFPtPArj4GzBEFHohegg1Xatrc1Fi9oSox5LzuSRX91miwQxuUrEpBxpvDRsmZYJKYFhgdK3UStsjC8JKXfUbMinjFqiEM4uNwzVaCaHpys
";

    fn with_format(format: &str) -> String {
        CONFIG.replace("Policy:", &format!("Format: {format}\nPolicy:"))
    }

    #[test]
    fn the_three_script_forms_wrap_the_same_keys() {
        for (format, head) in [
            ("P2WSH", "wsh(sortedmulti(2,"),
            ("P2SH-P2WSH", "sh(wsh(sortedmulti(2,"),
            ("P2WSH-P2SH", "sh(wsh(sortedmulti(2,"),
            ("P2SH", "sh(sortedmulti(2,"),
        ] {
            let policy = parse(&with_format(format)).expect(format);
            assert!(
                policy.to_descriptor().starts_with(head),
                "{format}: {}",
                policy.to_descriptor()
            );
            assert_eq!(policy.quorum(), Some((2, 2)));
        }
    }

    #[test]
    fn a_config_without_a_format_or_with_a_quorum_it_cannot_meet_is_refused() {
        assert_eq!(parse(CONFIG), Err(Error::Template));
        assert_eq!(
            parse(&with_format("P2WSH").replace("2 of 2", "3 of 2")),
            Err(Error::Threshold)
        );
        assert_eq!(
            parse(&with_format("P2WSH").replace("2 of 2", "2 of 3")),
            Err(Error::Placeholder),
            "a quorum that names more keys than the file lists"
        );
        assert_eq!(
            parse(&with_format("P2TR")),
            Err(Error::Template),
            "a format no multisig script here pays to"
        );
    }

    /// The same key spelled `Vpub` (SLIP-132's multisig prefix for a test
    /// network) or `vpub` reads as the same wallet with the same checksum.
    #[test]
    fn slip132_spellings_of_a_key_read_as_the_same_wallet() {
        let plain = parse(&with_format("P2WSH")).unwrap();
        let tpub = "tpubDFH9dgzveyD8zTbPUFuLrGmCydNvxehyNdUXKJAQN8x4aZ4j6UZqGfnqFrD4NqyaTVGKbvEW54tsvPTK2UoSbCC1PJY8iCNiwTL3RWZEheQ";
        let respell = |version: [u8; 4]| {
            let mut bytes = xkey::payload(tpub).unwrap();
            bytes[..4].copy_from_slice(&version);
            base58::encode_check(&bytes)
        };
        let vpub_multi = respell([0x02, 0x57, 0x54, 0x83]);
        let vpub_single = respell([0x04, 0x5f, 0x1c, 0xf6]);
        assert!(vpub_multi.starts_with("Vpub"), "{vpub_multi}");
        assert!(vpub_single.starts_with("vpub"), "{vpub_single}");
        for spelled in [vpub_multi, vpub_single] {
            let policy = parse(&with_format("P2WSH").replace(tpub, &spelled)).unwrap();
            assert_eq!(policy, plain);
            assert_eq!(policy.checksum(), plain.checksum());
        }
        assert_eq!(
            parse(&with_format("P2WSH").replace(tpub, "tpubnotakey")),
            Err(Error::Key)
        );
    }

    #[test]
    fn only_the_four_headers_start_one_of_these_files() {
        assert!(looks_like_config(CONFIG));
        assert!(looks_like_config("# a comment\n\nFormat: P2WSH\n"));
        assert!(!looks_like_config(
            "wsh(sortedmulti(2,@0/**,@1/**))\n[73c5da0a/48'/1'/0'/2']tpub"
        ));
        assert!(!looks_like_config(""));
    }
}
