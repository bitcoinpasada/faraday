//! BIP-85 child keys through `faraday_bip::bip85`, from a master key
//! given as an xprv.

use faraday_bip::bip39::Language;
use faraday_bip::bip85;
use faraday_bip::keys::MasterKey;
use faraday_crypto::work::Compute;
use serde_json::{Value, json};

use crate::Answer;
use crate::json::{hex, num, text};

fn err<E: core::fmt::Debug>(e: E) -> String {
    format!("{e:?}")
}

/// Handles `bip85.<name>`.
pub fn handle(name: &str, req: &Value) -> Answer {
    let c = Compute::unchecked();
    let master = MasterKey::decode(&c, text(req, "xprv")).map_err(err)?;
    let index = u32::try_from(num(req, "index")).map_err(err)?;
    match name {
        "mnemonic" => {
            let language = *Language::ALL
                .get(num(req, "language") as usize)
                .ok_or("no such language")?;
            let m = bip85::child_mnemonic(&c, &master, language, num(req, "words") as usize, index)
                .map_err(err)?;
            Ok(json!({"indices": m.indices()}))
        }
        "wif" => {
            let wif = bip85::child_wif(&c, &master, index).map_err(err)?;
            Ok(json!({"wif": wif.as_str()}))
        }
        "xprv" => {
            let x = bip85::child_xprv(&c, &master, index).map_err(err)?;
            Ok(json!({"xprv": x.as_str()}))
        }
        "hex" => {
            let h =
                bip85::child_hex(&c, &master, num(req, "bytes") as usize, index).map_err(err)?;
            Ok(json!({"hex": hex(h.as_bytes())}))
        }
        "base64" => {
            let p = bip85::child_password_base64(&c, &master, num(req, "length") as usize, index)
                .map_err(err)?;
            Ok(json!({"password": p.as_str()}))
        }
        "base85" => {
            let p = bip85::child_password_base85(&c, &master, num(req, "length") as usize, index)
                .map_err(err)?;
            Ok(json!({"password": p.as_str()}))
        }
        _ => Err(format!("unknown bip85 op {name}")),
    }
}
