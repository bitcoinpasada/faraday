//! Addresses and descriptor checksums: `faraday_bip::address::parse`
//! (an address as typed or scanned), `bitcoin`'s `Address::from_script`
//! (how Faraday writes the address of an output it inspects or a wallet
//! derives), `faraday_bip::account` (a single-key account's xpub, its
//! SLIP-132 form, its descriptor and its addresses) and
//! `faraday_bip::descriptor`'s BIP-380 checksum.

use bitcoin::{Address, ScriptBuf};
use faraday_bip::keys::{MasterKey, Network, ScriptType};
use faraday_bip::{address, descriptor};
use faraday_crypto::work::Compute;
use faraday_crypto::{Secret, SeedBytes};
use serde_json::{Value, json};

use crate::Answer;
use crate::json::{bytes, flag, hex, num, text};

fn err<E: core::fmt::Debug>(e: E) -> String {
    format!("{e:?}")
}

fn network(req: &Value) -> Network {
    Network::from_name(text(req, "network")).expect("a network name")
}

/// Handles `addr.<name>`.
pub fn handle(name: &str, req: &Value) -> Answer {
    match name {
        // Text to the output script it pays, on one network.
        "parse" => {
            let a = address::parse(text(req, "text"), network(req)).map_err(err)?;
            Ok(json!({"script": hex(a.script_pubkey().as_bytes())}))
        }
        // An output script's address, where it has one.
        "from_script" => {
            let script = ScriptBuf::from_bytes(bytes(req, "script"));
            let a =
                Address::from_script(&script, bitcoin::Network::from(network(req))).map_err(err)?;
            Ok(json!({"address": a.to_string()}))
        }
        // A single-key account of a seed: its xpub, SLIP-132 form,
        // descriptor and one address.
        "account" => {
            let c = Compute::unchecked();
            let seed = SeedBytes::new(&bytes(req, "seed")).ok_or("a seed past 64 bytes")?;
            let master = MasterKey::from_seed_bytes(&c, &Secret::new(seed), network(req));
            let script = *ScriptType::ALL
                .get(num(req, "script") as usize)
                .ok_or("no such script type")?;
            let account = u32::try_from(num(req, "account")).map_err(err)?;
            let acct = master.account_xpub(&c, script, account).map_err(err)?;
            let index = u32::try_from(num(req, "index")).map_err(err)?;
            let a = acct.address(flag(req, "change"), index).map_err(err)?;
            Ok(json!({
                "xpub": acct.xpub_string(),
                "slip132": acct.slip132_string(),
                "descriptor": acct.descriptor(),
                "address": a.to_string(),
            }))
        }
        // BIP-380's checksum of a descriptor without one.
        "descsum" => {
            let sum = descriptor::descriptor_checksum(text(req, "text"))
                .ok_or("a character outside the descriptor charset")?;
            Ok(json!({"checksum": String::from_utf8(sum.to_vec()).map_err(err)?}))
        }
        // Whether a descriptor ends in its own checksum.
        "descsum_verify" => Ok(json!({"valid": descriptor::verify_checksum(text(req, "text"))})),
        _ => Err(format!("unknown addr op {name}")),
    }
}
