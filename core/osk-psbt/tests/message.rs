//! Signed messages against the vectors other implementations publish
//! (`tools/vectors/message/`): BIP-322's own file, and BIP-137
//! signatures made by Trezor and by Electrum.

mod common;

use std::str::FromStr;

use common::{hex, master, records, unhex};
use osk_bip::keys::{MasterKey, Network, ScriptType};
use osk_psbt::bitcoin::PrivateKey;
use osk_psbt::bitcoin::address::{Address, NetworkUnchecked};
use osk_psbt::bitcoin::bip32::DerivationPath;
use osk_psbt::bitcoin::secp256k1::SecretKey;
use osk_psbt::message::{self, Checked, Error, Format};
use osk_psbt::{Aux, Nonce};
use serde_json::Value;

const BIP322_KEY: &str = "L3VFeEujGtevx9w18HD1fhRbCH67Az2dpCymeRE1SoPK6XQtaN2k";

/// A signing context for a loose key from a vector file, which has no
/// master key to borrow one from. The blind is fixed: what it is set to
/// cannot change a signature, only the trace a machine would leak.
fn secp() -> osk_psbt::bitcoin::secp256k1::Secp256k1<osk_psbt::bitcoin::secp256k1::All> {
    let mut secp = osk_psbt::bitcoin::secp256k1::Secp256k1::new();
    secp.seeded_randomize(&[0x2a; 32]);
    secp
}

fn script_of(name: &str) -> ScriptType {
    ScriptType::ALL
        .into_iter()
        .find(|s| s.name() == name)
        .unwrap_or_else(|| panic!("unknown script type `{name}`"))
}

fn address_of(text: &str, network: Network) -> Address {
    Address::<NetworkUnchecked>::from_str(text)
        .unwrap_or_else(|e| panic!("{text}: {e}"))
        .require_network(network.into())
        .unwrap_or_else(|e| panic!("{text}: {e}"))
}

fn vectors_json(path: &str) -> Value {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let text = std::fs::read_to_string(root.join(path)).expect("vector file");
    serde_json::from_str(&text).expect("vector json")
}

/// The key one vector signs with: a WIF, or the mnemonic and path the
/// hardware wallet derived it at.
fn key_of(
    wif: Option<&str>,
    words: Option<&str>,
    path: Option<&str>,
    network: Network,
) -> Option<(SecretKey, bool)> {
    if let Some(wif) = wif {
        let key = PrivateKey::from_wif(wif).expect("wif");
        return Some((key.inner, key.compressed));
    }
    let words = words?;
    let path = DerivationPath::from_str(path?).expect("path");
    let master: MasterKey = master(words, network);
    let derived = master.derive(&path);
    Some((*derived.secret_key(), true))
}

/// Trezor's and Electrum's published BIP-137 signatures: every one of
/// them verifies against the address it names, and this crate signing
/// the same message with the same key produces the same bytes.
#[test]
fn bip137_vectors() {
    let recs = records("tools/vectors/message/bip137.txt");
    assert_eq!(recs.len(), 14);
    for rec in &recs {
        let case = rec.get("case");
        let network = match rec.opt("network") {
            Some("testnet") => Network::Testnet,
            _ => Network::Mainnet,
        };
        let script = script_of(rec.get("script"));
        let message = rec.get("message");
        let published = match (rec.opt("signature-hex"), rec.opt("signature-base64")) {
            (Some(h), _) => osk_psbt::base64::encode(&unhex(h)),
            (_, Some(b)) => b.to_string(),
            _ => panic!("{case}: no signature"),
        };
        let address = address_of(rec.get("address"), network);

        assert_eq!(
            message::verify(&address, message, &published),
            Ok(Checked {
                format: Format::Bip137,
                script,
            }),
            "{case}: the published signature does not verify"
        );
        assert_eq!(
            message::verify(&address, &format!("{message} "), &published),
            Err(Error::Invalid),
            "{case}: a changed message still verifies"
        );

        let Some((secret, compressed)) =
            key_of(rec.opt("wif"), rec.opt("words"), rec.opt("path"), network)
        else {
            continue;
        };
        if !compressed {
            // The address of an uncompressed key is not one this device
            // derives; the vector is here for the check above.
            continue;
        }
        // Trezor stops at the first RFC 6979 nonce; Electrum grinds for
        // a low R. Each vector is reproduced under the setting the
        // wallet that published it uses (§16.38).
        let nonce = if rec.get("source") == "trezor" {
            Nonce::First
        } else {
            Nonce::LowR
        };
        let signed = message::sign(
            &secp(),
            &secret,
            script,
            network,
            message,
            Format::Bip137,
            nonce,
            Aux::Deterministic,
        )
        .unwrap_or_else(|e| panic!("{case}: {e:?}"));
        assert_eq!(signed.address, rec.get("address"), "{case}: address");
        let ours = osk_psbt::base64::decode(&signed.signature).expect("base64");
        let theirs = osk_psbt::base64::decode(&published).expect("base64");
        if rec.opt("no-script-type").is_some() {
            // The signature is the same; the wallet that made it wrote a
            // p2pkh-range header for a segwit address.
            assert_eq!(hex(&ours[1..]), hex(&theirs[1..]), "{case}: signature");
            assert_ne!(ours[0], theirs[0], "{case}: header");
        } else {
            assert_eq!(hex(&ours), hex(&theirs), "{case}: signature");
        }
    }
}

/// BIP-322's message hash and the two transactions it commits to, for
/// the three messages the BIP tabulates.
#[test]
fn bip322_transaction_vectors() {
    let vectors = vectors_json("tools/vectors/message/bip322-basic.json");
    let cases = vectors["tx_hashes"].as_array().expect("tx_hashes");
    assert_eq!(cases.len(), 3);
    for case in cases {
        let message = case["message"].as_str().expect("message");
        let address = address_of(case["address"].as_str().expect("address"), Network::Mainnet);
        assert_eq!(
            hex(&message::message_hash(message)),
            case["message_hash"].as_str().expect("hash"),
            "message hash of {message:?}"
        );
        let (spend, sign) = message::virtual_txids(&address, message);
        assert_eq!(
            spend.to_string(),
            case["to_spend_tx_hash"].as_str().expect("to_spend"),
            "to_spend of {message:?}"
        );
        assert_eq!(
            sign.to_string(),
            case["to_sign_tx_hash"].as_str().expect("to_sign"),
            "to_sign of {message:?}"
        );
    }
}

/// BIP-322's "simple" signatures: each published one verifies, this
/// crate's own signature for the same key and message is one of them,
/// and the script address the variant cannot express is refused rather
/// than called invalid.
#[test]
fn bip322_simple_vectors() {
    let vectors = vectors_json("tools/vectors/message/bip322-basic.json");
    let cases = vectors["simple"].as_array().expect("simple");
    assert_eq!(cases.len(), 4);
    let mut signed_here = 0;
    for case in cases {
        let message = case["message"].as_str().expect("message");
        let kind = case["type"].as_str().expect("type");
        let address = address_of(case["address"].as_str().expect("address"), Network::Mainnet);
        let published: Vec<&str> = case["bip322_signatures"]
            .as_array()
            .expect("signatures")
            .iter()
            .map(|s| s.as_str().expect("signature"))
            .collect();
        if kind.starts_with("p2wsh") {
            for signature in &published {
                assert_eq!(
                    message::verify(&address, message, signature),
                    Err(Error::UnsupportedAddress),
                    "a p2wsh signature is not a simple signature this crate reads"
                );
            }
            continue;
        }
        let script = if kind == "p2tr" {
            ScriptType::Taproot
        } else {
            ScriptType::NativeSegwit
        };
        for signature in &published {
            assert_eq!(
                message::verify(&address, message, signature),
                Ok(Checked {
                    format: Format::Bip322,
                    script,
                }),
                "{message:?}: the published signature does not verify"
            );
            assert_eq!(
                message::verify(&address, "another message", signature),
                Err(Error::Invalid),
                "{message:?}: another message verifies"
            );
        }
        let wif = case["private_keys"][0].as_str().expect("private key");
        let secret = PrivateKey::from_wif(wif).expect("wif").inner;
        let signed = message::sign(
            &secp(),
            &secret,
            script,
            Network::Mainnet,
            message,
            Format::Bip322,
            Nonce::LowR,
            Aux::Deterministic,
        )
        .expect("signs");
        assert_eq!(signed.address, case["address"].as_str().expect("address"));
        if script == ScriptType::Taproot {
            // A Schnorr signature depends on the auxiliary randomness
            // BIP-340 lets the signer choose; under `Aux::Deterministic`
            // there is none, so its bytes are its own and it is the
            // check that has to hold.
            assert_eq!(
                message::verify(&address, message, &signed.signature),
                Ok(Checked {
                    format: Format::Bip322,
                    script,
                })
            );
        } else {
            assert!(
                published
                    .iter()
                    .any(|p| p.trim_start_matches("smp") == signed.signature),
                "{message:?}: {} is none of the published signatures",
                signed.signature
            );
        }
        signed_here += 1;
    }
    assert_eq!(signed_here, 3);
}

/// The signatures BIP-322 says must not be accepted are not accepted.
#[test]
fn bip322_error_vectors() {
    let vectors = vectors_json("tools/vectors/message/bip322-basic.json");
    let cases = vectors["error"].as_array().expect("error");
    assert!(cases.len() >= 6);
    for case in cases {
        let description = case["description"].as_str().expect("description");
        let message = case["message"].as_str().expect("message");
        let signature = case["signature"].as_str().expect("signature");
        let address = address_of(case["address"].as_str().expect("address"), Network::Mainnet);
        assert!(
            message::verify(&address, message, signature).is_err(),
            "accepted: {description}"
        );
    }
}

/// A message signed on this device verifies here for every script type,
/// every format that type has, and an address that is not the first.
#[test]
fn sign_then_verify_round_trip() {
    let master = master(common::ABANDON, Network::Mainnet);
    let text = "Two lines,\nthe second one indented\n  like this.";
    for script in ScriptType::ALL {
        for index in [0u32, 1, 7] {
            let path = master
                .account_path(script, 0)
                .expect("account path")
                .extend([
                    osk_psbt::bitcoin::bip32::ChildNumber::from_normal_idx(0).unwrap(),
                    osk_psbt::bitcoin::bip32::ChildNumber::from_normal_idx(index).unwrap(),
                ]);
            let derived = master.derive(&path);
            let expected = master
                .account_xpub(script, 0)
                .expect("account")
                .address(false, index)
                .expect("address")
                .to_string();
            for format in Format::for_script(script) {
                for nonce in [Nonce::LowR, Nonce::First] {
                    let signed = message::sign(
                        derived.secp(),
                        derived.secret_key(),
                        script,
                        Network::Mainnet,
                        text,
                        *format,
                        nonce,
                        Aux::Deterministic,
                    )
                    .unwrap_or_else(|e| panic!("{script} {format:?}: {e:?}"));
                    assert_eq!(signed.address, expected, "{script} at {index}");
                    let address = address_of(&signed.address, Network::Mainnet);
                    assert_eq!(
                        message::verify(&address, text, &signed.signature),
                        Ok(Checked {
                            format: *format,
                            script,
                        }),
                        "{script} at {index}, {format:?}"
                    );
                    assert_eq!(
                        message::verify(&address, "another message", &signed.signature),
                        Err(Error::Invalid),
                        "{script} at {index}, {format:?}: another message verifies"
                    );
                }
            }
        }
    }
}

/// A signature does not verify for an address it was not made for.
#[test]
fn a_signature_belongs_to_one_address() {
    let secret = PrivateKey::from_wif(BIP322_KEY).expect("wif").inner;
    let signed = message::sign(
        &secp(),
        &secret,
        ScriptType::NativeSegwit,
        Network::Mainnet,
        "Hello World",
        Format::Bip322,
        Nonce::LowR,
        Aux::Deterministic,
    )
    .expect("signs");
    let other = address_of(
        "bc1qannfxke2tfd4l7vhepehpvt05y83v3qsf6nfkk",
        Network::Mainnet,
    );
    assert!(
        message::verify(&other, "Hello World", &signed.signature).is_err(),
        "a signature verified for an address it was not made for"
    );
}
