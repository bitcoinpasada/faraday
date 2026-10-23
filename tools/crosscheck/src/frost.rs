//! FROST threshold signing (BIP 445) and the trusted dealer through
//! `faraday_bip::frost`.

use bitcoin::secp256k1::{All, PublicKey, Secp256k1};
use faraday_bip::frost::{
    self, AggNonce, PartialSig, PubNonce, SecNonce, SecShare, SessionContext,
};
use serde_json::{Value, json};

use crate::Answer;
use crate::json::{b32, bytes, bytes_list, hex, num, nums, opt_bytes, pubkey, pubkeys, tweaks};

fn err<E: core::fmt::Debug>(e: E) -> String {
    format!("{e:?}")
}

fn ids(req: &Value) -> Vec<u32> {
    nums(req, "ids").into_iter().map(|i| i as u32).collect()
}

fn share(req: &Value, key: &str) -> Result<SecShare, String> {
    SecShare::from_bytes(&b32(req, key)).map_err(err)
}

fn opt_pubshares(req: &Value) -> Result<Option<Vec<PublicKey>>, String> {
    match req.get("pubshares") {
        Some(Value::Array(_)) => Ok(Some(pubkeys(req, "pubshares")?)),
        _ => Ok(None),
    }
}

fn pubnonces(req: &Value) -> Result<Vec<PubNonce>, String> {
    bytes_list(req, "pubnonces")
        .iter()
        .enumerate()
        .map(|(i, b)| frost::parse_pubnonce(b, i).map_err(err))
        .collect()
}

fn context(req: &Value) -> Result<SessionContext, String> {
    Ok(SessionContext {
        n: num(req, "n") as usize,
        t: num(req, "t") as usize,
        ids: ids(req),
        pubshares: opt_pubshares(req)?,
        thresh_pk: pubkey(req, "thresh_pk")?,
        aggnonce: frost::parse_aggnonce(&bytes(req, "aggnonce"), frost::Contrib::AggNonce)
            .map_err(err)?,
        tweaks: tweaks(req)?,
        msg: bytes(req, "msg"),
    })
}

/// Handles `frost.<name>`.
pub fn handle(name: &str, req: &Value) -> Answer {
    let secp = Secp256k1::new();
    match name {
        "deal" => deal(&secp, req),
        "nonce_gen" => nonce_gen(&secp, req),
        "nonce_agg" => {
            let agg = frost::nonce_agg_with(&secp, &pubnonces(req)?).map_err(err)?;
            Ok(json!({"aggnonce": hex(&agg.serialize())}))
        }
        "sign" => {
            let secnonce = SecNonce::from_bytes(&bytes(req, "secnonce")).map_err(err)?;
            let ctx = context(req)?;
            let psig = frost::sign(
                &secp,
                secnonce,
                &share(req, "secshare")?,
                num(req, "my_id") as u32,
                &ctx,
            )
            .map_err(err)?;
            Ok(json!({"psig": hex(&psig.serialize())}))
        }
        "verify" => verify(&secp, req),
        "agg" => {
            let session = frost::session_values(&secp, &context(req)?).map_err(err)?;
            let psigs = bytes_list(req, "psigs")
                .iter()
                .enumerate()
                .map(|(i, b)| frost::parse_psig(b, i).map_err(err))
                .collect::<Result<Vec<PartialSig>, _>>()?;
            let sig = frost::partial_sig_agg(&psigs, &session).map_err(err)?;
            Ok(json!({"sig": hex(&sig)}))
        }
        "det_sign" => det_sign(&secp, req),
        "interpolating_value" => {
            let v = frost::derive_interpolating_value(&ids(req), num(req, "my_id") as u32)
                .map_err(err)?;
            Ok(json!({"lambda": hex(&v)}))
        }
        "thresh_pubkey" => {
            let pk = frost::derive_thresh_pubkey(&secp, &ids(req), &pubkeys(req, "pubshares")?)
                .map_err(err)?;
            Ok(json!({"thresh_pk": hex(&pk.serialize())}))
        }
        _ => Err(format!("unknown frost op {name}")),
    }
}

fn deal(secp: &Secp256k1<All>, req: &Value) -> Answer {
    let chosen_ids = nums(req, "chosen_ids");
    let chosen = bytes_list(req, "chosen_shares")
        .iter()
        .zip(chosen_ids)
        .map(|(b, id)| {
            let b: [u8; 32] = b.as_slice().try_into().expect("32 bytes");
            SecShare::from_bytes(&b)
                .map(|s| (id as u32, s))
                .map_err(err)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let dealt = frost::deal(
        secp,
        num(req, "n") as usize,
        num(req, "t") as usize,
        &chosen,
    )
    .map_err(err)?;
    Ok(json!({
        "shares": dealt.shares.iter().map(|s| hex(&s.secret_bytes())).collect::<Vec<_>>(),
        "pubshares": dealt.info.pubshares.iter()
            .map(|p| hex(&p.expect("the dealer writes every share").serialize()))
            .collect::<Vec<_>>(),
        "thresh_pk": hex(&dealt.info.thresh_pk.serialize()),
    }))
}

fn nonce_gen(secp: &Secp256k1<All>, req: &Value) -> Answer {
    let secshare = match opt_bytes(req, "secshare") {
        Some(_) => Some(share(req, "secshare")?),
        None => None,
    };
    let pubshare = match opt_bytes(req, "pubshare") {
        Some(_) => Some(pubkey(req, "pubshare")?),
        None => None,
    };
    let xonly: Option<[u8; 32]> = opt_bytes(req, "thresh_pk").map(|a| a.try_into().expect("32"));
    let msg = opt_bytes(req, "msg");
    let extra = opt_bytes(req, "extra_in");
    let (sec, public) = frost::nonce_gen(
        secp,
        &b32(req, "rand"),
        secshare.as_ref(),
        pubshare.as_ref(),
        xonly.as_ref(),
        msg.as_deref(),
        extra.as_deref(),
    )
    .map_err(err)?;
    Ok(json!({"secnonce": hex(&sec.serialize()), "pubnonce": hex(&public.serialize())}))
}

fn verify(secp: &Secp256k1<All>, req: &Value) -> Answer {
    let psig = frost::parse_psig(&bytes(req, "psig"), num(req, "i") as usize).map_err(err)?;
    let ok = frost::partial_sig_verify(
        secp,
        &psig,
        &pubnonces(req)?,
        num(req, "n") as usize,
        num(req, "t") as usize,
        &ids(req),
        &pubkeys(req, "pubshares")?,
        &pubkey(req, "thresh_pk")?,
        &tweaks(req)?,
        &bytes(req, "msg"),
        num(req, "i") as usize,
    );
    Ok(json!({"valid": ok.is_ok()}))
}

fn det_sign(secp: &Secp256k1<All>, req: &Value) -> Answer {
    let other: Option<AggNonce> = match opt_bytes(req, "aggothernonce") {
        Some(b) => Some(frost::parse_aggnonce(&b, frost::Contrib::AggOtherNonce).map_err(err)?),
        None => None,
    };
    let rand: Option<[u8; 32]> = opt_bytes(req, "rand").map(|r| r.try_into().expect("32"));
    let pubshares = opt_pubshares(req)?;
    let (public, psig) = frost::deterministic_sign(
        secp,
        &share(req, "secshare")?,
        num(req, "my_id") as u32,
        other.as_ref(),
        num(req, "n") as usize,
        num(req, "t") as usize,
        &ids(req),
        pubshares.as_deref(),
        &pubkey(req, "thresh_pk")?,
        &tweaks(req)?,
        &bytes(req, "msg"),
        rand.as_ref(),
    )
    .map_err(err)?;
    Ok(json!({"pubnonce": hex(&public.serialize()), "psig": hex(&psig.serialize())}))
}
