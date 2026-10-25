//! Faraday's side of the crypto cross-check (F79, `docs/CROSSCHECK.md`
//! §10): one JSON request a line on stdin, one JSON answer a line on
//! stdout, each computed by Faraday's own code. `tools/crosscheck/crosscheck.py`
//! sends the requests and recomputes every answer with the vendored
//! references in `tools/reference/`.
//!
//! A request is `{"id": n, "op": "area.name", ...}`; the answer is
//! `{"id": n, "ok": {...}}`, or `{"id": n, "err": "..."}` when Faraday
//! refuses the input. The cross-check compares refusals as well as
//! results: a refusal one side makes and the other does not is a
//! disagreement.
//!
//! Test-only: nothing ships this, and the secrets it handles are test
//! data. `FARADAY_CROSSCHECK_SABOTAGE=<op>` changes every answer to that
//! operation, which is how the harness's self-test proves that a wrong
//! answer is caught.

mod addr;
mod bip32;
mod bip85;
mod codex32;
mod entropy;
mod frost;
mod json;
mod kdbx;
mod lightning;
mod msg;
mod musig;
mod own;
mod parts;
mod psbt;
mod roller;
mod seedqr;
mod silent;
mod slip39;
mod tx;
mod ur;
mod vanity;
mod vault;
mod xor;

use std::io::{BufRead, Write};
use std::panic::{AssertUnwindSafe, catch_unwind};

use serde_json::{Map, Value, json};

/// The answer to one request: the result, or why Faraday refused it.
pub type Answer = Result<Value, String>;

fn dispatch(op: &str, req: &Value) -> Answer {
    let (area, name) = op.split_once('.').ok_or("an op is area.name")?;
    match area {
        "musig" => musig::handle(name, req),
        "frost" => frost::handle(name, req),
        "bip85" => bip85::handle(name, req),
        "bip39" => bip32::handle_bip39(name, req),
        "bip32" => bip32::handle_bip32(name, req),
        "addr" => addr::handle(name, req),
        "tx" => tx::handle(name, req),
        "psbt" => psbt::handle(name, req),
        "msg" => msg::handle(name, req),
        "slip39" => slip39::handle(name, req),
        "codex32" => codex32::handle(name, req),
        "xor" => xor::handle(name, req),
        "sp" => silent::handle(name, req),
        "vault" => vault::handle(name, req),
        "own" => own::handle(name, req),
        "entropy" => entropy::handle(name, req),
        "roller" => roller::handle(name, req),
        "seedqr" => seedqr::handle(name, req),
        "bbqr" => parts::handle_bbqr(name, req),
        "specter" => parts::handle_specter(name, req),
        "ur" => ur::handle(name, req),
        "vanity" => vanity::handle(name, req),
        "kdbx" => kdbx::handle(name, req),
        "lightning" => lightning::handle(name, req),
        _ => Err(format!("unknown area {area}")),
    }
}

/// Changes one value of an answer so that it is wrong: the first string
/// (its last character), boolean or number found, depth first.
fn sabotage(v: &mut Value) -> bool {
    match v {
        Value::String(s) => {
            let last = s.pop().unwrap_or('0');
            s.push(if last == '0' { '1' } else { '0' });
            true
        }
        Value::Bool(b) => {
            *b = !*b;
            true
        }
        Value::Number(n) => {
            *v = json!(n.as_i64().unwrap_or(0) + 1);
            true
        }
        Value::Array(items) => items.iter_mut().any(sabotage),
        Value::Object(map) => map.values_mut().any(sabotage),
        Value::Null => {
            *v = json!("sabotaged");
            true
        }
    }
}

fn answer(line: &str, sabotaged: Option<&str>) -> Value {
    let req: Value = match serde_json::from_str(line) {
        Ok(v) => v,
        Err(e) => return json!({"id": null, "fatal": format!("bad request: {e}")}),
    };
    let id = req.get("id").cloned().unwrap_or(Value::Null);
    let op = req
        .get("op")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let result = catch_unwind(AssertUnwindSafe(|| dispatch(&op, &req)));
    let mut out = Map::new();
    out.insert("id".into(), id);
    match result {
        Ok(Ok(mut v)) => {
            if sabotaged == Some(op.as_str()) {
                sabotage(&mut v);
            }
            out.insert("ok".into(), v);
        }
        Ok(Err(e)) => {
            out.insert("err".into(), Value::String(e));
        }
        Err(_) => {
            out.insert("fatal".into(), Value::String(format!("{op} panicked")));
        }
    }
    Value::Object(out)
}

fn main() {
    // A panic is reported in the answer; the default hook's message on
    // stderr would only repeat it.
    std::panic::set_hook(Box::new(|_| {}));
    let sabotaged = std::env::var("FARADAY_CROSSCHECK_SABOTAGE").ok();
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout().lock();
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        if line.trim().is_empty() {
            continue;
        }
        let out = answer(&line, sabotaged.as_deref());
        if writeln!(stdout, "{out}")
            .and_then(|()| stdout.flush())
            .is_err()
        {
            break;
        }
    }
}
