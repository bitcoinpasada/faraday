//! Taproot outputs with script trees, built as Faraday builds them:
//! `bitcoin`'s `TaprootBuilder` and `TaprootSpendInfo` (what
//! `faraday_bip::tapmulti` builds a taproot multisig with, and what
//! `miniscript`'s `tr()` descriptors build every tree with), the output
//! script `faraday_psbt::sign` checks a key-path spend against
//! (`ScriptBuf::new_p2tr`, the internal key tweaked by a Merkle root), the
//! address `Address::p2tr_tweaked` writes, and a control block read and
//! checked against an output key as `miniscript`'s finalizer and
//! interpreter check the leaf of a script-path spend.

use bitcoin::hashes::Hash;
use bitcoin::secp256k1::{Secp256k1, XOnlyPublicKey};
use bitcoin::taproot::{ControlBlock, LeafVersion, TapLeafHash, TaprootBuilder};
use bitcoin::{Address, ScriptBuf};
use serde_json::{Value, json};

use crate::Answer;
use crate::json::{bytes, hex};

fn err<E: core::fmt::Debug>(e: E) -> String {
    format!("{e:?}")
}

fn xonly(req: &Value, key: &str) -> Result<XOnlyPublicKey, String> {
    XOnlyPublicKey::from_slice(&bytes(req, key)).map_err(|_| format!("{key} is not an x-only key"))
}

/// Handles `taproot.<name>`.
pub fn handle(name: &str, req: &Value) -> Answer {
    let secp = Secp256k1::verification_only();
    match name {
        // A tree given as its leaves in depth-first order, each
        // `[depth, script, leaf version]`, over an internal key.
        "tree" => {
            let internal = xonly(req, "internal")?;
            let mut leaves = Vec::new();
            let mut builder = TaprootBuilder::new();
            for leaf in req["leaves"].as_array().expect("leaves") {
                let depth = u8::try_from(leaf[0].as_u64().expect("a depth")).map_err(err)?;
                let script =
                    ScriptBuf::from_bytes(crate::json::unhex(leaf[1].as_str().expect("a script")));
                let version = u8::try_from(leaf[2].as_u64().expect("a version")).map_err(err)?;
                let version = LeafVersion::from_consensus(version).map_err(err)?;
                builder = builder
                    .add_leaf_with_ver(depth, script.clone(), version)
                    .map_err(err)?;
                leaves.push((script, version));
            }
            let info = builder
                .finalize(&secp, internal)
                .map_err(|_| "the leaves are not a whole tree".to_string())?;
            let merkle = info.merkle_root();
            let output = info.output_key();
            let mut controls = Vec::new();
            let mut hashes = Vec::new();
            for (script, version) in &leaves {
                hashes.push(hex(
                    &TapLeafHash::from_script(script, *version).to_byte_array()
                ));
                let control = info
                    .control_block(&(script.clone(), *version))
                    .ok_or("a leaf has no control block")?;
                controls.push(hex(&control.serialize()));
            }
            Ok(json!({
                "output_key": hex(&output.to_x_only_public_key().serialize()),
                "parity": u8::from(info.output_key_parity() == bitcoin::secp256k1::Parity::Odd),
                "tweak": hex(&info.tap_tweak().to_byte_array()),
                "merkle_root": merkle.map(|m| hex(&m.to_byte_array())),
                "leaf_hashes": hashes,
                "control_blocks": controls,
                "script_pubkey": hex(ScriptBuf::new_p2tr(&secp, internal, merkle).as_bytes()),
                "address": Address::p2tr_tweaked(output, bitcoin::Network::Bitcoin).to_string(),
            }))
        }
        // A control block and its leaf script, checked against an output
        // key.
        "verify" => {
            let output = xonly(req, "output_key")?;
            let control = ControlBlock::decode(&bytes(req, "control")).map_err(err)?;
            let script = ScriptBuf::from_bytes(bytes(req, "script"));
            Ok(json!({"valid": control.verify_taproot_commitment(&secp, output, &script)}))
        }
        _ => Err(format!("unknown taproot op {name}")),
    }
}
