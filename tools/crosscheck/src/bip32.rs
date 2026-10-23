//! BIP-39 through `faraday_bip::bip39`, and BIP-32 through
//! `faraday_bip::keys` (the master key and private derivation),
//! `faraday_bip::xkey` (the strict decoders of extended keys written as
//! text) and `bitcoin`'s `Xpub::derive_pub`, which Faraday's account and
//! wallet code derives public children with.

use std::str::FromStr;

use bitcoin::bip32::DerivationPath;
use bitcoin::secp256k1::Secp256k1;
use faraday_bip::bip39::{self, Language, Mnemonic};
use faraday_bip::keys::{MasterKey, Network};
use faraday_bip::xkey;
use faraday_crypto::work::Compute;
use faraday_crypto::{Secret, SeedBytes};
use serde_json::{Value, json};

use crate::Answer;
use crate::json::{bytes, hex, num, text};

fn err<E: core::fmt::Debug>(e: E) -> String {
    format!("{e:?}")
}

fn language(req: &Value) -> Result<Language, String> {
    Language::ALL
        .get(num(req, "language") as usize)
        .copied()
        .ok_or_else(|| "no such language".to_string())
}

fn network(req: &Value) -> Network {
    Network::from_name(text(req, "network")).expect("a network name")
}

fn path(req: &Value) -> Result<DerivationPath, String> {
    DerivationPath::from_str(text(req, "path")).map_err(err)
}

fn master(c: &Compute, req: &Value) -> Result<MasterKey, String> {
    let seed = SeedBytes::new(&bytes(req, "seed")).ok_or("a seed longer than 64 bytes")?;
    Ok(MasterKey::from_seed_bytes(
        c,
        &Secret::new(seed),
        network(req),
    ))
}

/// Handles `bip39.<name>`.
pub fn handle_bip39(name: &str, req: &Value) -> Answer {
    let c = Compute::unchecked();
    match name {
        "wordlist" => {
            let lang = language(req)?;
            Ok(json!({"words": lang.words().to_vec(), "nfkd": lang.words_nfkd().to_vec()}))
        }
        "from_entropy" => {
            let m = Mnemonic::from_entropy(language(req)?, &bytes(req, "entropy")).map_err(err)?;
            Ok(json!({"indices": m.indices()}))
        }
        "parse" => {
            let m = Mnemonic::parse(&c, language(req)?, text(req, "sentence")).map_err(err)?;
            Ok(json!({
                "entropy": hex(m.entropy().expose().as_bytes()),
                "indices": m.indices(),
            }))
        }
        // The seed of words and a passphrase as a person gives one: the
        // product path, printable ASCII only.
        "seed" => {
            let m = Mnemonic::parse(&c, language(req)?, text(req, "sentence")).map_err(err)?;
            let seed = m
                .to_seed(&c, text(req, "passphrase").as_bytes())
                .map_err(err)?;
            Ok(json!({"seed": hex(seed.expose())}))
        }
        // The same over a passphrase already in NFKD, as the vector tests
        // and the start-up self-test call it.
        "seed_nfkd" => {
            let m = Mnemonic::parse(&c, language(req)?, text(req, "sentence")).map_err(err)?;
            let seed = m
                .to_seed_unchecked(&c, &bytes(req, "passphrase"))
                .map_err(err)?;
            Ok(json!({"seed": hex(seed.expose())}))
        }
        "last_words" => {
            let first: Vec<u16> = crate::json::nums(req, "indices")
                .into_iter()
                .map(|n| u16::try_from(n).unwrap_or(u16::MAX))
                .collect();
            let words: Vec<u16> = bip39::last_word_candidates(&first).map_err(err)?.collect();
            Ok(json!({"indices": words}))
        }
        _ => Err(format!("unknown bip39 op {name}")),
    }
}

/// Handles `bip32.<name>`.
pub fn handle_bip32(name: &str, req: &Value) -> Answer {
    let c = Compute::unchecked();
    match name {
        // The master key of a seed of 16 to 64 bytes.
        "master" => {
            let m = master(&c, req)?;
            Ok(json!({
                "xprv": m.xpriv_ascii().as_str(),
                "xpub": m.xpub(&c).to_string(),
                "fingerprint": hex(m.fingerprint().as_bytes()),
            }))
        }
        // The private child at a path, and its public key.
        "derive" => {
            let m = master(&c, req)?;
            let d = m.derive(&c, &path(req)?);
            Ok(json!({
                "xprv": d.xpriv_ascii().as_str(),
                "xpub": d.to_xpub(&c).to_string(),
                "fingerprint": hex(d.fingerprint().as_bytes()),
                "master_fingerprint": hex(d.master_fingerprint().as_bytes()),
            }))
        }
        // A public child of an xpub read by the strict decoder.
        "derive_pub" => {
            let xpub = xkey::decode_xpub(text(req, "xpub")).map_err(err)?;
            let child = xpub
                .derive_pub(&Secp256k1::verification_only(), &path(req)?)
                .map_err(err)?;
            Ok(json!({"xpub": child.to_string()}))
        }
        // Extended keys written as text, read by the rules of BIP-32's
        // test vector 5 and written back.
        "decode_xprv" => {
            let k = xkey::decode_xpriv(text(req, "key")).map_err(err)?;
            Ok(json!({"key": k.to_string()}))
        }
        "decode_xpub" => {
            let k = xkey::decode_xpub(text(req, "key")).map_err(err)?;
            Ok(json!({"key": k.to_string()}))
        }
        // A master key typed as an xprv: read strictly, and refused when
        // it is not at depth 0.
        "import" => {
            let m = MasterKey::decode(&c, text(req, "key")).map_err(err)?;
            Ok(json!({
                "xprv": m.xpriv_ascii().as_str(),
                "fingerprint": hex(m.fingerprint().as_bytes()),
            }))
        }
        _ => Err(format!("unknown bip32 op {name}")),
    }
}
