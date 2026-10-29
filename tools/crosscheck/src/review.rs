//! What Faraday shows before signing a PSBT: `faraday_psbt::inspect`'s
//! amounts in and out, fee, which outputs are verified change, and the two
//! warnings Bitcoin Core can be asked about (a missing UTXO, a spoofed
//! change claim). The keys and registered wallets are loaded as the Sign
//! screen's session holds them.

use core::str::FromStr;

use bitcoin::bip32::Xpriv;
use faraday_bip::keys::{MasterKey, Network};
use faraday_bip::policy::WalletPolicy;
use faraday_crypto::work::Compute;
use faraday_crypto::{Secret, SeedBytes};
use faraday_psbt::{Context, KeyRef, Psbt, WarningKind, inspect};
use serde_json::{Value, json};

use crate::Answer;
use crate::json::{bytes, bytes_list, text, texts};

fn err<E: core::fmt::Debug>(e: E) -> String {
    format!("{e:?}")
}

/// Handles `review.<name>`.
pub fn handle(name: &str, req: &Value) -> Answer {
    let c = Compute::unchecked();
    match name {
        "inspect" => {
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
            let wallets: Vec<WalletPolicy> = texts(req, "wallets")
                .into_iter()
                .map(|t| WalletPolicy::parse_any(&c, t))
                .collect::<Result<_, _>>()
                .map_err(err)?;
            let mut refs: Vec<KeyRef> = Vec::new();
            for m in &masters {
                let mut r = KeyRef::from_master(&c, m, 0).map_err(err)?;
                for key in wallets.iter().flat_map(WalletPolicy::keys) {
                    let Some(path) = key.path() else { continue };
                    if key.fingerprint() == Some(m.fingerprint()) && r.xpub_at(path).is_none() {
                        r.extra.push((path.clone(), m.derive(&c, path).to_xpub(&c)));
                    }
                }
                refs.push(r);
            }
            let ctx = Context {
                network,
                keys: &refs,
                wallets: &wallets,
                musig_session: None,
            };
            let psbt = Psbt::parse_bytes(&c, &bytes(req, "psbt")).map_err(err)?;
            let review = inspect(&c, &psbt, &ctx);
            let has = |kind: &WarningKind| review.warnings.iter().any(|w| &w.kind == kind);
            Ok(json!({
                "fee": review.fee.to_sat(),
                "total_in": review.total_in.to_sat(),
                "total_out": review.total_out.to_sat(),
                "to_others": review.amount_to_others.to_sat(),
                "change": review.change_total.to_sat(),
                "inputs": review.inputs.iter().map(|i| i.amount.map(|a| a.to_sat())).collect::<Vec<_>>(),
                "outputs": review.outputs.iter().map(|o| json!({
                    "amount": o.amount.to_sat(),
                    "ours": o.is_ours(),
                })).collect::<Vec<_>>(),
                "missing_utxo": has(&WarningKind::MissingUtxo),
                "change_spoof": has(&WarningKind::ChangeSpoof),
            }))
        }
        _ => Err(format!("unknown review op {name}")),
    }
}
