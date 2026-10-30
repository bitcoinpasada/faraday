//! Silent payments (BIP-352) through `faraday_bip::silent`.

use bitcoin::secp256k1::{All, Secp256k1};
use faraday_bip::keys::Network;
use faraday_bip::silent::{self, Receiver, ScanInput, SendKey};
use serde_json::{Value, json};

use crate::Answer;
use crate::json::{bytes_list, flag, hex, nums, pubkey, pubkeys, seckey, text, unhex};

fn err<E: core::fmt::Debug>(e: E) -> String {
    format!("{e:?}")
}

fn network(req: &Value) -> Network {
    if flag(req, "mainnet") {
        Network::Mainnet
    } else {
        Network::Testnet
    }
}

/// An input as the request writes it, its bytes owned.
struct Input {
    outpoint: [u8; 36],
    script_sig: Vec<u8>,
    witness: Vec<Vec<u8>>,
    prevout: Vec<u8>,
}

fn inputs(req: &Value) -> Vec<Input> {
    req["inputs"]
        .as_array()
        .expect("inputs is a list")
        .iter()
        .map(|i| Input {
            outpoint: unhex(i["outpoint"].as_str().expect("outpoint"))
                .try_into()
                .expect("36 bytes"),
            script_sig: unhex(i["script_sig"].as_str().expect("script_sig")),
            witness: bytes_list(i, "witness"),
            prevout: unhex(i["prevout"].as_str().expect("prevout")),
        })
        .collect()
}

/// Handles `sp.<name>`.
pub fn handle(name: &str, req: &Value) -> Answer {
    let secp = Secp256k1::new();
    match name {
        "address" => {
            let r = Receiver::new(
                seckey(req, "scan_sk")?,
                pubkey(req, "spend_pk")?,
                network(req),
            );
            let a = match req.get("label").and_then(Value::as_u64) {
                Some(m) => r.labelled_address(&secp, m as u32),
                None => r.address(&secp),
            }
            .map_err(err)?;
            Ok(json!({"address": a.as_str()}))
        }
        "decode" => {
            let (net, scan, spend) = silent::decode_address(text(req, "address")).map_err(err)?;
            Ok(json!({
                "mainnet": net.is_mainnet(),
                "scan": hex(&scan.serialize()),
                "spend": hex(&spend.serialize()),
            }))
        }
        "scan_key" => {
            let c = faraday_crypto::work::Compute::unchecked();
            let read = silent::decode_scan_key(&c, text(req, "text")).map_err(err)?;
            Ok(json!({
                "mainnet": read.network.is_mainnet(),
                "scan": hex(&read.scan.serialize()),
                "spend": hex(&read.spend.serialize()),
            }))
        }
        "descriptor" => {
            let c = faraday_crypto::work::Compute::unchecked();
            let w =
                faraday_bip::silent_wallet::parse_descriptor(&c, text(req, "text")).map_err(err)?;
            Ok(json!({
                "fingerprint": w.fingerprint.to_string(),
                "account": w.account,
                "mainnet": w.network.is_mainnet(),
                "scan": hex(&w.scan.serialize()),
                "spend": hex(&w.spend.serialize()),
            }))
        }
        "sender" => sender(&secp, req),
        "scan" | "input_hash" => {
            let owned = inputs(req);
            let witnesses: Vec<Vec<&[u8]>> = owned
                .iter()
                .map(|i| i.witness.iter().map(Vec::as_slice).collect())
                .collect();
            let scan_inputs: Vec<ScanInput<'_>> = owned
                .iter()
                .zip(&witnesses)
                .map(|(i, w)| ScanInput {
                    outpoint: i.outpoint,
                    script_sig: &i.script_sig,
                    witness: w,
                    prevout: &i.prevout,
                })
                .collect();
            if name == "input_hash" {
                let (sum, hash) = silent::input_sum_and_hash(&scan_inputs).map_err(err)?;
                return Ok(json!({"sum": hex(&sum.serialize()), "hash": hex(&hash)}));
            }
            scan(&secp, req, &scan_inputs)
        }
        _ => Err(format!("unknown sp op {name}")),
    }
}

fn sender(secp: &Secp256k1<All>, req: &Value) -> Answer {
    let keys = req["keys"]
        .as_array()
        .expect("keys is a list")
        .iter()
        .map(|k| {
            let key =
                bitcoin::secp256k1::SecretKey::from_slice(&unhex(k[0].as_str().expect("key")))
                    .map_err(err)?;
            Ok(SendKey {
                key,
                taproot: k[1].as_bool().expect("taproot"),
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let outpoints: Vec<[u8; 36]> = bytes_list(req, "outpoints")
        .into_iter()
        .map(|o| o.try_into().expect("36 bytes"))
        .collect();
    let out = silent::sender_outputs(
        secp,
        &keys,
        &outpoints,
        &pubkey(req, "scan")?,
        &pubkeys(req, "spends")?,
    )
    .map_err(err)?;
    Ok(json!({"outputs": out.iter().map(|o| hex(o)).collect::<Vec<_>>()}))
}

fn scan(secp: &Secp256k1<All>, req: &Value, inputs: &[ScanInput<'_>]) -> Answer {
    let receiver = Receiver::new(
        seckey(req, "scan_sk")?,
        pubkey(req, "spend_pk")?,
        Network::Testnet,
    );
    let outputs: Vec<[u8; 32]> = bytes_list(req, "outputs")
        .into_iter()
        .map(|o| o.try_into().expect("32 bytes"))
        .collect();
    let labels: Vec<u32> = nums(req, "labels").into_iter().map(|m| m as u32).collect();
    let found = silent::scan(secp, &receiver, inputs, &outputs, &labels).map_err(err)?;
    Ok(json!({"found": found.iter().map(|f| json!({
        "index": f.index,
        "key": hex(&f.key),
        "label": f.label,
    })).collect::<Vec<_>>()}))
}
