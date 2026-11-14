//! The entropy procedures of `faraday_entropy`: dice under each of the
//! three procedures, coins and hex. Each request takes
//! the inputs one at a time, as the key-making flow's pad does, and the
//! answer says which were kept, the source's commitment, and the entropy
//! at the strength asked for (or why there is none).

use faraday_entropy::{CoinFlips, DiceProcedure, DiceRolls, RawEntropy, RawHex, Strength};
use serde_json::{Value, json};

use crate::Answer;
use crate::json::{hex, num, nums, text};

fn strength(req: &Value) -> Result<Strength, String> {
    Strength::for_bytes(num(req, "bytes") as usize).ok_or_else(|| "no such strength".into())
}

fn entropy(e: Result<RawEntropy, faraday_entropy::Error>) -> Value {
    match e {
        Ok(e) => json!(hex(e.as_bytes())),
        Err(_) => Value::Null,
    }
}

fn procedure(req: &Value) -> DiceProcedure {
    DiceProcedure::ALL[num(req, "procedure") as usize]
}

fn dice(req: &Value) -> (DiceRolls, Vec<bool>) {
    let p = procedure(req);
    let mut d = DiceRolls::new();
    let kept = nums(req, "rolls")
        .into_iter()
        .map(|r| u8::try_from(r).is_ok_and(|r| d.push_under(r, p)))
        .collect();
    (d, kept)
}

fn coins(req: &Value) -> (CoinFlips, Vec<bool>) {
    let mut c = CoinFlips::new();
    let kept = text(req, "flips")
        .chars()
        .map(|f| c.push(f == 'H'))
        .collect();
    (c, kept)
}

fn hexes(req: &Value) -> (RawHex, Vec<bool>) {
    let mut h = RawHex::new();
    let kept = text(req, "digits").chars().map(|c| h.push(c)).collect();
    (h, kept)
}

/// Handles `entropy.<name>`.
pub fn handle(name: &str, req: &Value) -> Answer {
    match name {
        // How many of each input a strength asks for.
        "needed" => {
            let s = strength(req)?;
            Ok(json!({
                "rolls": DiceRolls::needed(s),
                "word_rolls": DiceProcedure::Words.needed(s),
                "flips": CoinFlips::needed(s),
                "digits": s.hex_digits(),
                "words": s.words(),
            }))
        }
        "dice" => {
            let s = strength(req)?;
            let p = procedure(req);
            let (d, kept) = dice(req);
            Ok(json!({
                "kept": kept,
                "commitment": hex(&d.commitment()),
                "entropy": entropy(d.entropy_under(s, p)),
            }))
        }
        "coins" => {
            let s = strength(req)?;
            let (c, kept) = coins(req);
            Ok(json!({
                "kept": kept,
                "commitment": hex(&c.commitment()),
                "entropy": entropy(c.entropy(s)),
            }))
        }
        "hex" => {
            let (h, kept) = hexes(req);
            Ok(json!({
                "kept": kept,
                "commitment": hex(&h.commitment()),
                "entropy": entropy(h.entropy()),
                "parsed": entropy(RawHex::parse(text(req, "digits"))),
            }))
        }
        _ => Err(format!("unknown entropy op {name}")),
    }
}
