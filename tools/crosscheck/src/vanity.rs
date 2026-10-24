//! The vanity address search, `faraday_bip::vanity`: the counter written
//! in EntropyLab's order, which prefixes an address of a kind can begin
//! with and the work they are expected to take, and the search itself on
//! either dial, from a starting counter with a budget.

use faraday_bip::bip39::{Language, Mnemonic};
use faraday_bip::keys::{MasterKey, Network, ScriptType};
use faraday_bip::vanity::{self, Key, Method};
use faraday_crypto::work::Compute;
use faraday_crypto::{Secret, SeedBytes};
use serde_json::{Value, json};

use crate::Answer;
use crate::json::{bytes, num, text};

fn err<E: core::fmt::Debug>(e: E) -> String {
    format!("{e:?}")
}

fn network(req: &Value) -> Network {
    Network::from_name(text(req, "network")).expect("a network name")
}

fn script(req: &Value) -> ScriptType {
    ScriptType::ALL[num(req, "script") as usize]
}

/// Handles `vanity.<name>`.
pub fn handle(name: &str, req: &Value) -> Answer {
    let c = Compute::unchecked();
    match name {
        "counter" => {
            let t = vanity::counter_text(num(req, "counter")).ok_or("past eight characters")?;
            Ok(json!({"text": t.as_str()}))
        }
        "prefix" => {
            let (p, s, n) = (text(req, "prefix"), script(req), network(req));
            Ok(json!({
                "can_begin": vanity::can_begin(p, s, n),
                "expected": vanity::expected_candidates(p, s, n).to_string(),
            }))
        }
        // The passphrase dial over the words of `entropy` (English) with
        // `base` before the counter, or the account dial over the master
        // of `seed`.
        "grind" => {
            let (s, n) = (script(req), network(req));
            let (from, budget) = (num(req, "from"), num(req, "budget"));
            let outcome = if text(req, "dial") == "passphrase" {
                let words = Mnemonic::from_entropy(Language::English, &bytes(req, "entropy"))
                    .map_err(err)?;
                let base = bytes(req, "base");
                let method = Method::Passphrase { base: &base };
                vanity::grind(
                    &c,
                    &Key::Words(&words),
                    &method,
                    s,
                    text(req, "prefix"),
                    n,
                    from,
                    budget,
                )
            } else {
                let seed = SeedBytes::new(&bytes(req, "seed")).ok_or("a seed past 64 bytes")?;
                let master = MasterKey::from_seed_bytes(&c, &Secret::new(seed), n);
                let key = Key::Master(&master);
                vanity::grind(
                    &c,
                    &key,
                    &Method::Account,
                    s,
                    text(req, "prefix"),
                    n,
                    from,
                    budget,
                )
            };
            Ok(json!({
                "tested": outcome.tested,
                "find": outcome.find.map(|f| json!({
                    "counter": f.counter,
                    "suffix": f.suffix.as_str(),
                    "account": f.account,
                    "address": f.address.as_str(),
                })),
            }))
        }
        _ => Err(format!("unknown vanity op {name}")),
    }
}
