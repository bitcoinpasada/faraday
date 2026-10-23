//! Signed messages through `faraday_psbt::message`: BIP-137 and BIP-322
//! "simple" signatures made, checked, and BIP-322's message hash and
//! virtual transaction ids.

use std::str::FromStr;

use bitcoin::Address;
use bitcoin::address::NetworkUnchecked;
use bitcoin::secp256k1::{Secp256k1, SecretKey};
use faraday_bip::keys::{Network, ScriptType};
use faraday_crypto::work::Compute;
use faraday_psbt::message::{self, Format};
use faraday_psbt::{Aux, Nonce};
use serde_json::{Value, json};

use crate::Answer;
use crate::json::{b32, hex, num, opt_bytes, text};

fn err<E: core::fmt::Debug>(e: E) -> String {
    format!("{e:?}")
}

fn address(req: &Value) -> Result<Address, String> {
    Address::<NetworkUnchecked>::from_str(text(req, "address"))
        .map(Address::assume_checked)
        .map_err(err)
}

fn format_name(f: Format) -> &'static str {
    match f {
        Format::Bip137 => "bip137",
        Format::Bip322 => "bip322",
    }
}

/// Handles `msg.<name>`.
pub fn handle(name: &str, req: &Value) -> Answer {
    match name {
        "sign" => {
            let c = Compute::unchecked();
            let secp = Secp256k1::new();
            let secret = SecretKey::from_slice(&b32(req, "secret")).map_err(err)?;
            let script = *ScriptType::ALL
                .get(num(req, "script") as usize)
                .ok_or("no such script type")?;
            let network = Network::from_name(text(req, "network")).ok_or("no such network")?;
            let format = match text(req, "format") {
                "bip137" => Format::Bip137,
                _ => Format::Bip322,
            };
            let nonce = match text(req, "nonce") {
                "first" => Nonce::First,
                _ => Nonce::LowR,
            };
            let aux = match opt_bytes(req, "aux") {
                Some(b) => Aux::Fresh(b.try_into().expect("32 bytes of aux")),
                None => Aux::Deterministic,
            };
            let s = message::sign(
                &c,
                &secp,
                &secret,
                script,
                network,
                text(req, "message"),
                format,
                nonce,
                aux,
            )
            .map_err(err)?;
            Ok(json!({"address": s.address, "signature": s.signature}))
        }
        "verify" => {
            let checked =
                message::verify(&address(req)?, text(req, "message"), text(req, "signature"))
                    .map_err(err)?;
            Ok(json!({"format": format_name(checked.format), "script": checked.script.name()}))
        }
        "bip322" => {
            let a = address(req)?;
            let (spend, sign) = message::virtual_txids(&a, text(req, "message"));
            Ok(json!({
                "message_hash": hex(&message::message_hash(text(req, "message"))),
                "to_spend": spend.to_string(),
                "to_sign": sign.to_string(),
            }))
        }
        _ => Err(format!("unknown msg op {name}")),
    }
}
