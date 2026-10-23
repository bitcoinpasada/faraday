//! SLIP-39 Shamir backup through `faraday_bip::slip39`.

use faraday_bip::slip39::{self, GroupSpec, Share};
use faraday_crypto::work::Compute;
use serde_json::{Value, json};

use crate::Answer;
use crate::json::{bytes, flag, hex, num, text, texts};

fn err<E: core::fmt::Debug>(e: E) -> String {
    format!("{e:?}")
}

fn words(share: &Share) -> String {
    share
        .indices()
        .iter()
        .map(|&i| slip39::word(i))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Handles `slip39.<name>`.
pub fn handle(name: &str, req: &Value) -> Answer {
    let c = Compute::unchecked();
    let passphrase = text(req, "passphrase").as_bytes();
    match name {
        "split" => split(&c, req, passphrase),
        "recover" => {
            let shares = texts(req, "mnemonics")
                .into_iter()
                .map(|m| Share::parse(m).map_err(err))
                .collect::<Result<Vec<_>, _>>()?;
            let secret = slip39::recover(&c, &shares, passphrase).map_err(err)?;
            Ok(json!({"secret": hex(secret.expose().as_bytes())}))
        }
        "encrypt" | "decrypt" => {
            let f = if name == "encrypt" {
                slip39::encrypt
            } else {
                slip39::decrypt
            };
            let out = f(
                &c,
                &bytes(req, "value"),
                passphrase,
                num(req, "identifier") as u16,
                flag(req, "extendable"),
                num(req, "exponent") as u8,
            )
            .map_err(err)?;
            Ok(json!({"value": hex(out.expose().as_bytes())}))
        }
        _ => Err(format!("unknown slip39 op {name}")),
    }
}

fn split(c: &Compute, req: &Value, passphrase: &[u8]) -> Answer {
    let groups: Vec<GroupSpec> = req["groups"]
        .as_array()
        .expect("groups is a list")
        .iter()
        .map(|g| GroupSpec {
            threshold: g[0].as_u64().expect("threshold") as u8,
            count: g[1].as_u64().expect("count") as u8,
        })
        .collect();
    // The randomness the split draws, in the order it draws it, read
    // from the request so that the reference can draw the same bytes.
    let tape = bytes(req, "random");
    let mut at = 0usize;
    let mut random = |out: &mut [u8]| {
        let end = at + out.len();
        assert!(end <= tape.len(), "the random tape ran out");
        out.copy_from_slice(&tape[at..end]);
        at = end;
    };
    let total: usize = groups.iter().map(|g| g.count as usize).sum();
    let mut out: Vec<Share> = (0..total.max(1)).map(|_| Share::default()).collect();
    let written = slip39::split(
        c,
        &bytes(req, "secret"),
        passphrase,
        num(req, "identifier") as u16,
        flag(req, "extendable"),
        num(req, "exponent") as u8,
        num(req, "group_threshold") as u8,
        &groups,
        &mut random,
        &mut out,
    )
    .map_err(err)?;
    Ok(json!({
        "mnemonics": out[..written].iter().map(words).collect::<Vec<_>>(),
        "random_used": at,
    }))
}
