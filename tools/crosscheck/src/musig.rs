//! MuSig2 (BIP-327) through `faraday_bip::musig`.

use bitcoin::secp256k1::Secp256k1;
use faraday_bip::musig::{self, AggNonce, PartialSig, PubNonce, SecNonce};
use serde_json::{Value, json};

use crate::Answer;
use crate::json::{b32, bytes, bytes_list, hex, num, opt_bytes, pubkey, pubkeys, seckey, tweaks};

fn err<E: core::fmt::Debug>(e: E) -> String {
    format!("{e:?}")
}

fn pubnonces(req: &Value, key: &str) -> Result<Vec<PubNonce>, String> {
    bytes_list(req, key)
        .iter()
        .map(|b| PubNonce::from_bytes(b).map_err(err))
        .collect()
}

/// Handles `musig.<name>`.
pub fn handle(name: &str, req: &Value) -> Answer {
    let secp = Secp256k1::new();
    match name {
        "key_sort" => {
            let keys = musig::sort_keys(&pubkeys(req, "pubkeys")?);
            Ok(json!({"pubkeys": keys.iter().map(|k| hex(&k.serialize())).collect::<Vec<_>>()}))
        }
        "key_agg" => {
            let mut agg = musig::key_agg_with(&secp, &pubkeys(req, "pubkeys")?).map_err(err)?;
            for t in tweaks(req)? {
                agg = agg.apply_tweak(&secp, &t.bytes, t.x_only).map_err(err)?;
            }
            Ok(json!({
                "q": hex(&agg.public_key().serialize()),
                "gacc": hex(&agg.gacc()),
                "tacc": hex(&agg.tacc()),
            }))
        }
        "nonce_gen" => {
            let sk = match opt_bytes(req, "sk") {
                Some(_) => Some(seckey(req, "sk")?),
                None => None,
            };
            let aggpk: Option<[u8; 32]> =
                opt_bytes(req, "aggpk").map(|a| a.try_into().expect("32"));
            let msg = opt_bytes(req, "msg");
            let extra = opt_bytes(req, "extra_in");
            let (sec, public) = musig::nonce_gen(
                &secp,
                &b32(req, "rand"),
                sk.as_ref(),
                &pubkey(req, "pk")?,
                aggpk.as_ref(),
                msg.as_deref(),
                extra.as_deref(),
            )
            .map_err(err)?;
            Ok(json!({"secnonce": hex(&sec.serialize()), "pubnonce": hex(&public.serialize())}))
        }
        "nonce_agg" => {
            let agg = musig::nonce_agg_with(&secp, &pubnonces(req, "pubnonces")?).map_err(err)?;
            Ok(json!({"aggnonce": hex(&agg.serialize())}))
        }
        "sign" => {
            let session = session(&secp, req)?;
            let secnonce = SecNonce::from_bytes(&bytes(req, "secnonce")).map_err(err)?;
            let psig = musig::sign(&secp, secnonce, &seckey(req, "sk")?, &session).map_err(err)?;
            Ok(json!({"psig": hex(&psig.serialize())}))
        }
        "verify" => {
            let psig = PartialSig::from_bytes(&bytes(req, "psig")).map_err(err)?;
            let ok = musig::partial_sig_verify(
                &secp,
                &psig,
                &pubnonces(req, "pubnonces")?,
                &pubkeys(req, "pubkeys")?,
                &tweaks(req)?,
                &bytes(req, "msg"),
                num(req, "i") as usize,
            );
            Ok(json!({"valid": ok.is_ok()}))
        }
        "agg" => {
            let session = session(&secp, req)?;
            let psigs = bytes_list(req, "psigs")
                .iter()
                .map(|b| PartialSig::from_bytes(b).map_err(err))
                .collect::<Result<Vec<_>, _>>()?;
            let sig = musig::partial_sig_agg(&psigs, &session).map_err(err)?;
            Ok(json!({"sig": hex(&sig)}))
        }
        "det_sign" => {
            let other = AggNonce::from_bytes(&bytes(req, "aggothernonce")).map_err(err)?;
            let rand: Option<[u8; 32]> = opt_bytes(req, "rand").map(|r| r.try_into().expect("32"));
            let (public, psig) = musig::deterministic_sign(
                &secp,
                &seckey(req, "sk")?,
                &other,
                &pubkeys(req, "pubkeys")?,
                &tweaks(req)?,
                &bytes(req, "msg"),
                rand.as_ref(),
            )
            .map_err(err)?;
            Ok(json!({"pubnonce": hex(&public.serialize()), "psig": hex(&psig.serialize())}))
        }
        _ => Err(format!("unknown musig op {name}")),
    }
}

fn session(
    secp: &Secp256k1<bitcoin::secp256k1::All>,
    req: &Value,
) -> Result<musig::SessionValues, String> {
    let aggnonce = AggNonce::from_bytes(&bytes(req, "aggnonce")).map_err(err)?;
    musig::session_values(
        secp,
        &pubkeys(req, "pubkeys")?,
        &tweaks(req)?,
        &aggnonce,
        &bytes(req, "msg"),
    )
    .map_err(err)
}
