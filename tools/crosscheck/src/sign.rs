//! PSBT signing and finalizing end to end: `faraday_psbt::sign` with
//! every key the request holds selected, as the Sign screen calls it once
//! the person has confirmed (deterministic Schnorr auxiliary randomness,
//! the request's ECDSA nonce rule, the inspection's dangers confirmed),
//! then `faraday_psbt::finalize`, which builds each final scriptSig and
//! witness and extracts the transaction.

use core::str::FromStr;

use bitcoin::bip32::Xpriv;
use faraday_bip::keys::{MasterKey, Network};
use faraday_bip::policy::WalletPolicy;
use faraday_crypto::work::Compute;
use faraday_crypto::{Secret, SeedBytes};
use faraday_psbt::{Aux, Context, KeyRef, Nonce, Psbt, finalize, sign};
use serde_json::{Value, json};

use crate::Answer;
use crate::json::{bytes, bytes_list, hex, text, texts};

fn err<E: core::fmt::Debug>(e: E) -> String {
    format!("{e:?}")
}

/// Handles `sign.<name>`.
pub fn handle(name: &str, req: &Value) -> Answer {
    let c = Compute::unchecked();
    match name {
        "sign" => {
            let network = Network::from_name(text(req, "network")).expect("a network name");
            let mut masters = Vec::new();
            for seed in bytes_list(req, "seeds") {
                let seed = SeedBytes::new(&seed).ok_or("a seed past 64 bytes")?;
                masters.push(MasterKey::from_seed_bytes(&c, &Secret::new(seed), network));
            }
            for xprv in texts(req, "xprvs") {
                let xprv = Xpriv::from_str(xprv).map_err(err)?;
                masters.push(MasterKey::from_xpriv(&c, xprv).map_err(err)?);
            }
            let refs: Vec<KeyRef> = masters
                .iter()
                .map(|m| KeyRef::from_master(&c, m, 0))
                .collect::<Result<_, _>>()
                .map_err(err)?;
            // The wallets in use, as the app holds a registered one: a
            // change output of one is then verified rather than refused.
            let wallets: Vec<WalletPolicy> = texts(req, "wallets")
                .into_iter()
                .map(|t| WalletPolicy::parse_any(&c, t))
                .collect::<Result<_, _>>()
                .map_err(err)?;
            let ctx = Context {
                network,
                keys: &refs,
                wallets: &wallets,
                musig_session: None,
            };
            let nonce = match text(req, "nonce") {
                "low_r" => Nonce::LowR,
                "first" => Nonce::First,
                other => return Err(format!("unknown nonce rule {other}")),
            };
            let held: Vec<&MasterKey> = masters.iter().collect();
            let selection: Vec<_> = masters.iter().map(MasterKey::fingerprint).collect();
            let mut psbt = Psbt::parse_bytes(&c, &bytes(req, "psbt")).map_err(err)?;
            let done = sign(
                &c,
                &mut psbt,
                &held,
                &selection,
                &ctx,
                true,
                nonce,
                Aux::Deterministic,
                &mut None,
                &faraday_crypto::Zeroizing::new([0u8; 32]),
            )
            .map_err(err)?;
            Ok(json!({
                "psbt": hex(&psbt.to_bytes()),
                "signatures": done.signed_inputs.len(),
                "complete": done.complete,
            }))
        }
        "finalize" => {
            let mut psbt = Psbt::parse_bytes(&c, &bytes(req, "psbt")).map_err(err)?;
            let tx = finalize(&mut psbt).map_err(err)?;
            Ok(json!({
                "psbt": hex(&psbt.to_bytes()),
                "tx": tx.map(|t| hex(&bitcoin::consensus::serialize(&t))),
            }))
        }
        _ => Err(format!("unknown sign op {name}")),
    }
}
