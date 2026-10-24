//! Randomness a person adds of their own (`docs/OWN-RANDOMNESS.md` §2.3,
//! F86), mixed into the session's by `faraday_core`'s `fresh.rs`: a
//! session driven step by step as the app is, with the system's answers
//! given as fixed bytes.
//!
//! A batch is taken as New key's sources take it (`faraday_entropy`):
//! dice rolls and coin flips through `DiceRolls` and `CoinFlips`, one
//! camera frame through `CameraNoise` (a frame with under 32 distinct
//! luma values is refused there), and added with `Faraday::own_add`
//! under that source's commitment. The bytes the session then draws are
//! read through the testkit: `fresh` (a pooled answer, used up) and
//! `seed_draw` (the session seed's draw).

use faraday_core::fresh::{OwnCounts, OwnSource};
use faraday_core::{Action, testkit};
use faraday_entropy::{CameraNoise, CoinFlips, DiceRolls};
use faraday_shell_api::{App, EntropyBytes, Event};
use serde_json::{Value, json};

use crate::Answer;
use crate::json::{b32, bytes, hex, text};

fn counts(c: OwnCounts) -> Value {
    json!({
        "camera_frames": c.camera_frames,
        "dice_rolls": c.dice_rolls,
        "coin_flips": c.coin_flips,
        "cards": c.cards,
        "pointer_keys": c.pointer_keys,
    })
}

/// Handles `own.<name>`.
pub fn handle(name: &str, req: &Value) -> Answer {
    match name {
        "session" => session(req),
        _ => Err(format!("unknown own op {name}")),
    }
}

/// A session from its start: each step of `steps` in order, and what
/// each one gave.
fn session(req: &Value) -> Answer {
    let mut app = testkit::started();
    let mut out = Vec::new();
    for step in req["steps"].as_array().expect("steps is a list") {
        let got = match text(step, "do") {
            "answer" => {
                app.event(Event::Entropy(EntropyBytes::new(b32(step, "bytes"))));
                Value::Null
            }
            "dice" => {
                let mut rolls = DiceRolls::new();
                let all = text(step, "rolls")
                    .bytes()
                    .all(|r| rolls.push(r.wrapping_sub(b'0')));
                let n = text(step, "rolls").len() as u32;
                json!(all && app.own_add(OwnSource::Dice, &rolls.commitment(), n))
            }
            "coins" => {
                let mut flips = CoinFlips::new();
                let all = text(step, "flips").bytes().all(|f| flips.push(f == b'H'));
                let n = text(step, "flips").len() as u32;
                json!(all && app.own_add(OwnSource::Coins, &flips.commitment(), n))
            }
            "frame" => {
                let mut camera = CameraNoise::new();
                let taken = camera.push(&bytes(step, "luma")).1;
                json!(taken && app.own_add(OwnSource::Camera, &camera.commitment(), 1))
            }
            "lock" => {
                app.press(Action::Lock);
                Value::Null
            }
            "draw" => json!(
                testkit::seed_draw(&app, &bytes(step, "what"), &bytes(step, "count"))
                    .map(|b| hex(&b))
            ),
            "fresh" => json!(testkit::fresh(&mut app, &bytes(step, "what")).map(|b| hex(&b))),
            "counts" => counts(app.own_counts()),
            other => return Err(format!("unknown step {other}")),
        };
        out.push(got);
    }
    Ok(json!({ "out": out }))
}
