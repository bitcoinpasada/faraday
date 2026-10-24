//! Lightning node keys and the aezeed, `faraday_bip::aezeed`: ldk-node's
//! node key from a BIP-39 seed, LND's from an aezeed's entropy, and the
//! aezeed itself read from its words.

use bitcoin::secp256k1::{PublicKey, Secp256k1};
use faraday_bip::aezeed::{self, ENTROPY_SIZE, NUM_WORDS};
use faraday_bip::keys::{DerivedKey, Network};
use faraday_crypto::Secret;
use faraday_crypto::work::Compute;
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
        _ => Err(format!("unknown lightning op {name}")),
    }
}
