//! A wallet imported from another wallet's file, and BIP 129's key
//! records: `faraday_intake::wallet::read_wallet`, which every screen that
//! takes a wallet's text calls (Files, Restore, Wallets), and
//! `faraday_bip::bsms`'s key record read, verified and written as the
//! Create screen and `faraday-keys` do.

use faraday_bip::bsms::{self, SignerRecord};
use faraday_bip::keys::{MasterKey, MultisigScriptType, Network};
use faraday_crypto::work::Compute;
use faraday_crypto::{Secret, SeedBytes};
use faraday_intake::wallet::read_wallet;
use serde_json::{Value, json};

use crate::Answer;
use crate::json::{bytes, hex, num, text};

/// Handles `import.<name>`.
pub fn handle(name: &str, req: &Value) -> Answer {
    let c = Compute::unchecked();
    match name {
        // The wallet a file or a text is: its keys with their origins, as
        // `[fingerprint/path]xpub`, and the scripts of its first receive
        // and change addresses, what it pays to and watches.
        "read" => {
            let policy = read_wallet(&c, text(req, "text")).map_err(|e| format!("{e:?}"))?;
            let n = u32::try_from(num(req, "n")).map_err(|_| "n is a u32")?;
            let chain = |change: bool| -> Result<Vec<String>, String> {
                (0..n)
                    .map(|i| {
                        policy
                            .script_at(&c, change, i)
                            .map(|s| hex(s.as_bytes()))
                            .map_err(|e| format!("{e:?}"))
                    })
                    .collect()
            };
            let keys: Vec<String> = policy.keys().iter().map(|k| k.key_text()).collect();
            Ok(json!({"keys": keys, "receive": chain(false)?, "change": chain(true)?}))
        }
        // A signer's key record, read and its signature checked. A record
        // that does not read is a refusal; one that reads and does not
        // verify is `valid: false`.
        "verify" => {
            let record = SignerRecord::parse(text(req, "text")).map_err(|e| format!("{e:?}"))?;
            Ok(json!({"valid": record.verify().is_ok()}))
        }
        // This device's own key record for a BIP-48 account, or with
        // `bip87` a BIP-87 one, as
        // `faraday-keys` writes it: signed by the account node's key.
        "write" => {
            let network = Network::from_name(text(req, "network")).ok_or("a network name")?;
            let seed = SeedBytes::new(&bytes(req, "seed")).ok_or("a seed past 64 bytes")?;
            let master = MasterKey::from_seed_bytes(&c, &Secret::new(seed), network);
            let script = match num(req, "script") {
                1 => MultisigScriptType::NestedSegwit,
                2 => MultisigScriptType::NativeSegwit,
                _ => return Err("a BIP-48 script type is 1 or 2".into()),
            };
            let account = u32::try_from(num(req, "account")).map_err(|_| "account is a u32")?;
            let bip87 = req.get("bip87").and_then(Value::as_bool).unwrap_or(false);
            let record = master
                .with_multisig_account_secret_at(&c, script, account, bip87, |secret, view| {
                    bsms::signer_record(
                        master.secp(),
                        view,
                        secret,
                        text(req, "token"),
                        text(req, "description"),
                    )
                })
                .map_err(|e| format!("{e:?}"))?
                .map_err(|e| format!("{e:?}"))?;
            Ok(json!({"text": &*record.to_text()}))
        }
        // Faraday's JSON reader on a text: the value it reads, numbers as
        // the floats they stand for, or its refusal.
        "json" => {
            let v = faraday_bip::json::parse(text(req, "text")).map_err(|e| format!("{e:?}"))?;
            Ok(json!({"value": value_of(&v)}))
        }
        _ => Err(format!("unknown import op {name}")),
    }
}

/// A `faraday_bip::json::Value` as a serde_json one, a number as the
/// float of its text.
fn value_of(v: &faraday_bip::json::Value) -> Value {
    use faraday_bip::json::Value as J;
    match v {
        J::Null => Value::Null,
        J::Bool(b) => Value::Bool(*b),
        J::Number(t) => json!(t.parse::<f64>().expect("a number of the reader is a float")),
        J::String(s) => Value::String(s.clone()),
        J::Array(items) => Value::Array(items.iter().map(value_of).collect()),
        J::Object(members) => Value::Object(
            members
                .iter()
                .map(|(k, x)| (k.clone(), value_of(x)))
                .collect(),
        ),
    }
}
