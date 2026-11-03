//! SLIP-39 Shamir backup through `faraday_bip::slip39`.

use faraday_bip::slip39::{self, Ems, GroupSpec, Share};
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

fn shares(req: &Value) -> Result<Vec<Share>, String> {
    texts(req, "mnemonics")
        .into_iter()
        .map(|m| Share::parse(m).map_err(err))
        .collect()
}

/// Handles `slip39.<name>`.
pub fn handle(name: &str, req: &Value) -> Answer {
    let c = Compute::unchecked();
    match name {
        "split" => split(&c, req),
        "split_ems" => split_ems(req),
        "recover" => {
            let secret = slip39::recover(&c, &shares(req)?, text(req, "passphrase").as_bytes())
                .map_err(err)?;
            Ok(json!({"secret": hex(secret.expose().as_bytes())}))
        }
        "recover_ems" => {
            let ems = slip39::recover_ems(&shares(req)?).map_err(err)?;
            Ok(json!({
                "ciphertext": hex(ems.ciphertext().expose().as_bytes()),
                "identifier": ems.identifier(),
                "extendable": ems.is_extendable(),
                "exponent": ems.iteration_exponent(),
            }))
        }
        "derive_member" => {
            let share =
                slip39::derive_member(&shares(req)?, num(req, "index") as u8).map_err(err)?;
            Ok(json!({"mnemonic": words(&share)}))
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
                text(req, "passphrase").as_bytes(),
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

fn groups(req: &Value) -> Vec<GroupSpec> {
    req["groups"]
        .as_array()
        .expect("groups is a list")
        .iter()
        .map(|g| GroupSpec {
            threshold: g[0].as_u64().expect("threshold") as u8,
            count: g[1].as_u64().expect("count") as u8,
        })
        .collect()
}

/// The randomness a split draws, in the order it draws it, read from the
/// request so that the reference can draw the same bytes; and the shares
/// it writes, with how much of the tape it used.
fn with_tape(
    req: &Value,
    groups: &[GroupSpec],
    run: impl FnOnce(&mut dyn FnMut(&mut [u8]), &mut [Share]) -> Result<usize, slip39::Error>,
) -> Answer {
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
    let written = run(&mut random, &mut out).map_err(err)?;
    Ok(json!({
        "mnemonics": out[..written].iter().map(words).collect::<Vec<_>>(),
        "random_used": at,
    }))
}

fn split(c: &Compute, req: &Value) -> Answer {
    let groups = groups(req);
    with_tape(req, &groups, |random, out| {
        slip39::split(
            c,
            &bytes(req, "secret"),
            text(req, "passphrase").as_bytes(),
            num(req, "identifier") as u16,
            flag(req, "extendable"),
            num(req, "exponent") as u8,
            num(req, "group_threshold") as u8,
            &groups,
            random,
            out,
        )
    })
}

fn split_ems(req: &Value) -> Answer {
    let groups = groups(req);
    let ems = Ems::new(
        num(req, "identifier") as u16,
        flag(req, "extendable"),
        num(req, "exponent") as u8,
        &bytes(req, "ciphertext"),
    )
    .map_err(err)?;
    with_tape(req, &groups, |random, out| {
        slip39::split_ems(
            &ems,
            num(req, "group_threshold") as u8,
            &groups,
            random,
            out,
        )
    })
}
