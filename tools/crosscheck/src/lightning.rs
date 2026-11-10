//! Lightning node keys and the aezeed, `faraday_bip::aezeed`: ldk-node's
//! node key from a BIP-39 seed, LND's from an aezeed's entropy, and the
//! aezeed itself read from its words.

use bitcoin::secp256k1::{PublicKey, Secp256k1};
use faraday_bip::aezeed::{self, ENTROPY_SIZE, NUM_WORDS};
use faraday_bip::keys::{DerivedKey, Network};
use faraday_codec::bolt11::{self, Fallback};
use faraday_codec::bolt12;
use faraday_crypto::Secret;
use faraday_crypto::work::Compute;
use faraday_intake::secret_buf::SecretText;
use faraday_keys::Reveal;
use faraday_keys::hsm_node;
use faraday_keys::hsm_secret::{self, Refusal};
use serde_json::{Value, json};

use crate::Answer;
use crate::json::{bytes, hex, num, nums, text};

fn network(req: &Value) -> Network {
    Network::from_name(text(req, "network")).expect("a network name")
}

fn node(key: &DerivedKey) -> Value {
    let secret = key.secret_key();
    let public = PublicKey::from_secret_key(&Secp256k1::new(), secret);
    json!({"secret": hex(&secret.secret_bytes()), "node_id": hex(&public.serialize())})
}

/// A BOLT #11 invoice as the cross-check compares it: fields as numbers,
/// text and lowercase hex, a fallback as its 5-bit version (17 pay to
/// public key hash, 18 pay to script hash) and its data.
fn invoice(i: &bolt11::Invoice) -> Value {
    let hrp = match i.network {
        Network::Mainnet => "bc",
        Network::Testnet => "tb",
        Network::Signet => "tbs",
        Network::Regtest => "bcrt",
    };
    let fallbacks: Vec<Value> = i
        .fallbacks
        .iter()
        .map(|f| match f {
            Fallback::P2pkh(h) => json!({"version": 17, "data": hex(h)}),
            Fallback::P2sh(h) => json!({"version": 18, "data": hex(h)}),
            Fallback::Witness { version, program } => {
                json!({"version": version, "data": hex(program)})
            }
        })
        .collect();
    let hints: Vec<Vec<Value>> = i
        .route_hints
        .iter()
        .map(|hops| {
            hops.iter()
                .map(|h| {
                    json!({
                        "node_id": hex(&h.node_id),
                        "short_channel_id": h.short_channel_id,
                        "fee_base_msat": h.fee_base_msat,
                        "fee_proportional_millionths": h.fee_proportional_millionths,
                        "cltv_expiry_delta": h.cltv_expiry_delta,
                    })
                })
                .collect()
        })
        .collect();
    let unknown: Vec<Value> = i
        .unknown
        .iter()
        .map(|u| json!({"tag": u.tag.to_string(), "data": u.data}))
        .collect();
    json!({
        "network": hrp,
        "amount_msat": i.amount_msat,
        "timestamp": i.timestamp,
        "payment_hash": hex(&i.payment_hash),
        "description": i.description,
        "description_hash": i.description_hash.map(|h| hex(&h)),
        "payee": hex(&i.payee),
        "payee_from_field": i.payee_from_field,
        "expiry": i.expiry,
        "min_final_cltv_expiry": i.min_final_cltv_expiry,
        "fallbacks": fallbacks,
        "route_hints": hints,
        "features": i.features,
        "payment_secret": hex(&i.payment_secret),
        "metadata": i.metadata.as_ref().map(|m| hex(m)),
        "unknown": unknown,
    })
}

/// A BOLT #12 offer as the cross-check compares it: chains by genesis hash
/// in the order BOLT #12 writes it, the amount as millisatoshis or as a
/// currency code, exponent and minor units, paths by first node and hops.
fn offer(o: &bolt12::Offer) -> Value {
    let amount = o.amount.as_ref().map(|a| match a {
        bolt12::Amount::Bitcoin { msat } => json!({"msat": msat}),
        bolt12::Amount::Currency {
            code,
            exponent,
            units,
        } => json!({"currency": code, "exponent": exponent, "units": units}),
    });
    let paths: Vec<Value> = o
        .paths
        .iter()
        .map(|p| match p.first {
            bolt12::FirstNode::Node(n) => json!({"node": hex(&n), "hops": p.hops}),
            bolt12::FirstNode::Channel {
                short_channel_id,
                direction,
            } => json!({"scid": short_channel_id, "direction": direction, "hops": p.hops}),
        })
        .collect();
    let unknown: Vec<Value> = o
        .unknown
        .iter()
        .map(|u| json!({"type": u.kind, "data": hex(&u.data)}))
        .collect();
    json!({
        "chains": o.chains.iter().map(|c| hex(&c.hash())).collect::<Vec<_>>(),
        "metadata": o.metadata.as_ref().map(|m| hex(m)),
        "amount": amount,
        "description": o.description,
        "features": o.features,
        "expiry": o.expiry,
        "paths": paths,
        "issuer": o.issuer,
        "quantity_max": o.quantity_max,
        "issuer_id": o.issuer_id.map(|k| hex(&k)),
        "unknown": unknown,
    })
}

/// A node as the answer shows it, from `faraday_keys`.
fn keys_node(node: &faraday_keys::lightning::Node) -> Value {
    let secret = node
        .private_hex(Reveal::LightningKey)
        .expect("the key is shown");
    json!({"secret": secret.as_str(), "node_id": hex(node.public())})
}

/// Handles `lightning.<name>`.
pub fn handle(name: &str, req: &Value) -> Answer {
    let c = Compute::unchecked();
    match name {
        // ldk-node's node key from a 64-byte BIP-39 seed.
        "ldk" => {
            let seed: [u8; 64] = bytes(req, "seed").try_into().expect("64 bytes");
            Ok(node(&aezeed::ldk_node_key(
                &c,
                &Secret::new(seed),
                network(req),
            )))
        }
        // LND's node key from an aezeed's sixteen bytes of entropy.
        "lnd" => {
            let entropy: [u8; ENTROPY_SIZE] = bytes(req, "entropy").try_into().expect("16 bytes");
            Ok(node(&aezeed::node_key(
                &c,
                &Secret::new(entropy),
                network(req),
            )))
        }
        // An aezeed read from its word indices at a scrypt cost.
        "aezeed" => {
            let words: [u16; NUM_WORDS] = nums(req, "words")
                .into_iter()
                .map(|w| w as u16)
                .collect::<Vec<_>>()
                .try_into()
                .expect("24 words");
            let seed = aezeed::decode_at_cost(
                &c,
                &words,
                &bytes(req, "passphrase"),
                num(req, "cost") as u32,
            )
            .map_err(|e| format!("{e:?}"))?;
            Ok(json!({
                "internal_version": seed.internal_version,
                "birthday": seed.birthday,
                "entropy": hex(seed.entropy().expose()),
            }))
        }
        // A BOLT #11 invoice read, or the name of the refusal.
        "bolt11" => bolt11::decode(text(req, "invoice"))
            .map(|i| invoice(&i))
            .map_err(|e| format!("{e:?}")),
        // A BOLT #12 offer read, or the name of the refusal.
        "bolt12" => bolt12::decode(text(req, "offer"))
            .map(|o| offer(&o))
            .map_err(|e| format!("{e:?}")),
        // Core Lightning's hsm_secret file (32 bytes plain, 73 encrypted
        // under a passphrase, longer the mnemonic form) read as `faraday_keys` reads it.
        "hsm_file" => {
            let pass = SecretText::of(text(req, "passphrase"));
            match hsm_secret::node_from_file(&c, &bytes(req, "file"), &pass) {
                Ok(node) => Ok(keys_node(&node)),
                Err(Refusal::Length) => Err("length".to_string()),
                Err(Refusal::Opening) => Err("opening".to_string()),
                Err(Refusal::Memory) => Err("memory".to_string()),
                Err(Refusal::Words) => Err("words".to_string()),
                Err(Refusal::Passphrase) => Err("passphrase".to_string()),
                Err(Refusal::PassphraseNotNeeded) => Err("passphrase not needed".to_string()),
                Err(Refusal::PassphraseNotAscii) => Err("passphrase not ascii".to_string()),
                Err(Refusal::PassphraseTooLong) => Err("passphrase too long".to_string()),
            }
        }
        // Core Lightning's node from BIP-39 words and a passphrase.
        "hsm_words" => {
            let node = hsm_node::node_from_words(
                &c,
                faraday_bip::bip39::Language::English,
                text(req, "words"),
                text(req, "passphrase"),
            )
            .map_err(|e| format!("{e:?}"))?;
            Ok(keys_node(&node))
        }
        // Which AES round this command runs: `soft` when the cross-check
        // built it with the aes crate's software backend (`run.sh`).
        "backend" => {
            #[allow(unexpected_cfgs)]
            let soft = cfg!(aes_backend = "soft");
            Ok(json!({"aes": if soft { "soft" } else { "default" }}))
        }
        _ => Err(format!("unknown lightning op {name}")),
    }
}
