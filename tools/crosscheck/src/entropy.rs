//! The entropy procedures of `faraday_entropy`: dice under each of the
//! three procedures, coins, hex, cards, camera frames, the device's
//! answer, and a mix of sources by their commitments. Each request takes
//! the inputs one at a time, as the key-making flow's pad does, and the
//! answer says which were kept, the source's commitment, and the entropy
//! at the strength asked for (or why there is none).

use faraday_entropy::{
    CameraNoise, CardDraws, CoinFlips, DeviceRandom, DiceProcedure, DiceRolls, Mixed, RawEntropy,
    RawHex, Strength,
};
use serde_json::{Value, json};

use crate::Answer;
use crate::json::{b32, bytes_list, hex, num, nums, text};

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

/// One source's commitment from its inputs, as `kind` names it; the
/// sources a mix takes.
fn source(src: &Value) -> Result<[u8; 32], String> {
    Ok(match text(src, "kind") {
        "dice" => dice(src).0.commitment(),
        "coins" => coins(src).0.commitment(),
        "cards" => cards(src).0.commitment(),
        "camera" => camera(src).0.commitment(),
        "device" => {
            let mut d = DeviceRandom::new();
            d.set(&b32(src, "random"));
            d.commitment()
        }
        "hex" => hexes(src).0.commitment(),
        k => return Err(format!("no source {k}")),
    })
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

fn cards(req: &Value) -> (CardDraws, Vec<bool>) {
    let mut c = CardDraws::new();
    let kept = nums(req, "cards")
        .into_iter()
        .map(|i| u8::try_from(i).is_ok_and(|i| c.push(i)))
        .collect();
    (c, kept)
}

fn camera(req: &Value) -> (CameraNoise, Vec<Value>) {
    let mut c = CameraNoise::new();
    let taken = bytes_list(req, "frames")
        .iter()
        .map(|f| {
            let (stats, taken) = c.push(f);
            json!({"taken": taken, "distinct": stats.distinct, "pixels": stats.pixels})
        })
        .collect();
    (c, taken)
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
                "cards": CardDraws::needed(s),
                "frames": CameraNoise::needed(s),
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
        "cards" => {
            let s = strength(req)?;
            let (c, kept) = cards(req);
            Ok(json!({
                "kept": kept,
                "commitment": hex(&c.commitment()),
                "entropy": entropy(c.entropy(s)),
            }))
        }
        "camera" => {
            let s = strength(req)?;
            let (c, taken) = camera(req);
            Ok(json!({
                "frames": taken,
                "commitment": hex(&c.commitment()),
                "entropy": entropy(c.entropy(s)),
            }))
        }
        "device" => {
            let s = strength(req)?;
            let mut d = DeviceRandom::new();
            d.set(&b32(req, "random"));
            Ok(json!({
                "commitment": hex(&d.commitment()),
                "entropy": entropy(d.entropy(s)),
            }))
        }
        // Sources given by their inputs, each's commitment pushed in
        // order.
        "mix" => {
            let s = strength(req)?;
            let mut m = Mixed::new();
            let mut pushed = Vec::new();
            let mut commitments = Vec::new();
            for src in req["sources"].as_array().expect("sources is a list") {
                let c = source(src)?;
                commitments.push(hex(&c));
                pushed.push(m.push(c));
            }
            Ok(json!({
                "commitments": commitments,
                "pushed": pushed,
                "commitment": hex(&m.commitment()),
                "entropy": entropy(m.entropy(s)),
            }))
        }
        _ => Err(format!("unknown entropy op {name}")),
    }
}
