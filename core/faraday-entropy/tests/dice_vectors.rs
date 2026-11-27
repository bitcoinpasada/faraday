//! The dice vectors in `tools/vectors/entropy/dice.json`, produced by
//! Coldcard's `rolls.py` and EntropyLab's dice tools: the rolls a person
//! enters give the entropy those tools give, under each procedure.

use faraday_entropy::{DiceProcedure, DiceRolls, Strength};

const VECTORS: &str = include_str!("../../../tools/vectors/entropy/dice.json");

fn hex(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).expect("hex"))
        .collect()
}

fn typed(rolls: &str, procedure: DiceProcedure) -> DiceRolls {
    let mut d = DiceRolls::new();
    for c in rolls.bytes() {
        // A roll a procedure turns away is not kept, as when typed.
        d.push_under(c - b'0', procedure);
    }
    d
}

#[test]
fn rolls_give_the_entropy_the_other_dice_tools_give() {
    let file: serde_json::Value = serde_json::from_str(VECTORS).expect("json");
    let mut seen = [0usize; 3];
    for v in file["vectors"].as_array().expect("vectors") {
        let procedure = match v["procedure"].as_str().expect("procedure") {
            "hashed" => DiceProcedure::Hashed,
            "six_as_zero" => DiceProcedure::SixAsZero,
            "words" => DiceProcedure::Words,
            other => panic!("a procedure Faraday does not have: {other}"),
        };
        let strength = match v["bytes"].as_u64().expect("bytes") {
            16 => Strength::Bits128,
            32 => Strength::Bits256,
            n => panic!("{n} bytes"),
        };
        let rolls = v["rolls"].as_str().expect("rolls");
        let d = typed(rolls, procedure);
        let got = d.entropy_under(strength, procedure).expect("entropy");
        assert_eq!(
            got.as_bytes(),
            hex(v["entropy"].as_str().expect("entropy")),
            "{procedure:?} {rolls}"
        );
        let at = DiceProcedure::ALL
            .iter()
            .position(|p| *p == procedure)
            .expect("listed");
        seen[at] += 1;
    }
    assert!(seen.iter().all(|n| *n > 0), "{seen:?}");
}

#[test]
fn the_commitment_to_the_rolls_is_the_hash_the_other_tool_shows() {
    let file: serde_json::Value = serde_json::from_str(VECTORS).expect("json");
    for c in file["commitments"].as_array().expect("commitments") {
        let d = typed(c["rolls"].as_str().expect("rolls"), DiceProcedure::Hashed);
        assert_eq!(
            d.commitment()[..],
            hex(c["commitment"].as_str().expect("commitment"))
        );
    }
}
