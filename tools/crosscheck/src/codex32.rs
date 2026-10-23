//! Codex32 (BIP 93) through `faraday_bip::codex32`.

use faraday_bip::codex32::{
    self, ChecksumWorksheet, Codex32, Length, Scheme, TranslationWorksheet, fresh_set,
};
use faraday_crypto::work::Compute;
use serde_json::{Value, json};

use crate::Answer;
use crate::json::{bytes, hex, num, text, texts};

fn err<E: core::fmt::Debug>(e: E) -> String {
    format!("{e:?}")
}

fn strings(req: &Value, key: &str) -> Result<Vec<Codex32>, String> {
    texts(req, key)
        .into_iter()
        .map(|s| Codex32::parse(s).map_err(err))
        .collect()
}

fn four(req: &Value, key: &str) -> [u8; 4] {
    text(req, key)
        .as_bytes()
        .try_into()
        .expect("four characters")
}

/// What a string says about itself: its header, and its seed when it
/// is the secret.
fn describe(s: &Codex32) -> Value {
    let seed = s.to_seed().ok().map(|seed| hex(seed.expose().bytes()));
    json!({
        "string": s.encode().as_str(),
        "threshold": s.threshold(),
        "identifier": String::from_utf8_lossy(&s.identifier()),
        "index": (s.share_index() as char).to_string(),
        "seed": seed,
    })
}

const CHARSET: &[u8; 32] = b"qpzry9x8gf2tvdw0s3jn54khce6mua7l";

/// The symbols of bech32 text; a character outside the alphabet is a
/// refusal.
fn symbols(text: &str) -> Result<Vec<u8>, String> {
    text.bytes()
        .map(|b| {
            CHARSET
                .iter()
                .position(|&c| c == b.to_ascii_lowercase())
                .map(|v| v as u8)
                .ok_or_else(|| format!("{} is not bech32", b as char))
        })
        .collect()
}

fn chars(symbols: impl Iterator<Item = u8>) -> String {
    symbols.map(|v| CHARSET[usize::from(v)] as char).collect()
}

/// A checksum worksheet's rows, each as text: the residue of `ms`, the
/// sum rows (the last is the final row), the lookup rows, and each
/// step's width.
fn rows(sheet: &ChecksumWorksheet) -> Value {
    let l = sheet.checksum_len();
    let row = |f: &dyn Fn(usize) -> Option<u8>| -> Value {
        let cells: Option<Vec<u8>> = (0..l).map(f).collect();
        cells.map_or(Value::Null, |c| Value::String(chars(c.into_iter())))
    };
    json!({
        "hrp": chars(sheet.hrp_row().iter().copied()),
        "sums": (0..=sheet.steps()).map(|j| row(&|i| sheet.sum_cell(j, i))).collect::<Vec<_>>(),
        "lookups": (0..sheet.steps()).map(|j| row(&|i| sheet.lookup_cell(j, i))).collect::<Vec<_>>(),
        "widths": (0..sheet.steps()).map(|j| sheet.step_width(j)).collect::<Vec<_>>(),
        "holds": sheet.holds(),
    })
}

/// The whole worksheet of any string of codex32 characters, valid or
/// not, filled one character at a time as a person types it.
fn worksheet(c: &Compute, string: &str) -> Answer {
    if !string.is_ascii() || !string.to_ascii_lowercase().starts_with("ms1") {
        return Err("not ms1".into());
    }
    let mut sheet = ChecksumWorksheet::new(string.len()).map_err(err)?;
    for v in symbols(&string[3..])? {
        sheet.push(c, v).map_err(err)?;
    }
    Ok(rows(&sheet))
}

/// A checksum made on the worksheet from the boxes before it.
fn make(c: &Compute, prefix: &str, chars_total: usize) -> Answer {
    if !prefix.is_ascii() || !prefix.to_ascii_lowercase().starts_with("ms1") {
        return Err("not ms1".into());
    }
    let mut sheet = ChecksumWorksheet::new(chars_total).map_err(err)?;
    for v in symbols(&prefix[3..])? {
        sheet.push(c, v).map_err(err)?;
    }
    sheet.finish(c).map_err(err)?;
    let all = (4..=chars_total).map(|b| sheet.symbol(b).expect("every box"));
    Ok(json!({"string": format!("ms1{}", chars(all)), "holds": sheet.holds()}))
}

fn fresh(c: &Compute, req: &Value) -> Answer {
    let length =
        Length::of_chars(num(req, "chars") as usize).ok_or("no fresh set of this length")?;
    let scheme = Scheme::new(
        num(req, "threshold") as u8,
        num(req, "shares") as u8,
        four(req, "identifier"),
        length,
    )
    .map_err(err)?;
    let data = texts(req, "data")
        .into_iter()
        .map(symbols)
        .collect::<Result<Vec<_>, _>>()?;
    let refs: Vec<&[u8]> = data.iter().map(Vec::as_slice).collect();
    let set = fresh_set(c, &scheme, &refs).map_err(err)?;
    Ok(json!({
        "strings": set.strings().iter().map(|s| s.encode().as_str().to_string()).collect::<Vec<_>>(),
        "secret": set.secret().encode().as_str(),
    }))
}

fn translation_symbols(req: &Value) -> Answer {
    let indices = text(req, "indices").as_bytes();
    let target = text(req, "target").as_bytes()[0];
    let sheet = TranslationWorksheet::new(48, indices, target).map_err(err)?;
    let k = sheet.threshold();
    Ok(json!({
        "symbols": (0..k).map(|i| sheet.symbol(i)).collect::<Vec<_>>(),
        "pairs": (0..k).map(|i| (0..k).map(|j| if i == j { 0 } else { sheet.pair(i, j) }).collect::<Vec<_>>()).collect::<Vec<_>>(),
    }))
}

fn translate(c: &Compute, req: &Value) -> Answer {
    let strings = strings(req, "strings")?;
    let target = text(req, "target").as_bytes()[0];
    let sheet = TranslationWorksheet::of(c, &strings, target).map_err(err)?;
    let chars_total = strings[0].encode().as_str().len();
    let row = |f: &dyn Fn(usize) -> Option<u8>| {
        chars((4..=chars_total).map(|b| f(b).expect("a whole sheet")))
    };
    Ok(json!({
        "translated": (0..sheet.threshold()).map(|i| row(&|b| sheet.translated_cell(i, b))).collect::<Vec<_>>(),
        "sum": format!("ms1{}", row(&|b| sheet.sum_cell(b))),
    }))
}

/// The one correction of a damaged string (`?` for an unreadable box),
/// the boxes of `held` (a string of the same set) fixed where given.
fn correct(c: &Compute, req: &Value) -> Answer {
    let held = match req.get("held").and_then(Value::as_str) {
        Some(h) => Some(Codex32::parse(h).map_err(err)?),
        None => None,
    };
    let p = codex32::correct(c, text(req, "string"), held.as_ref()).map_err(err)?;
    let edits: Vec<Value> = p
        .edits()
        .iter()
        .map(|e| {
            json!([
                e.position(),
                (e.was() as char).to_string(),
                (e.now() as char).to_string()
            ])
        })
        .collect();
    Ok(json!({
        "string": p.string().encode().as_str(),
        "edits": edits,
        "run": p.assumes_the_rest(),
    }))
}

/// Handles `codex32.<name>`.
pub fn handle(name: &str, req: &Value) -> Answer {
    match name {
        "parse" => Ok(describe(&Codex32::parse(text(req, "string")).map_err(err)?)),
        "from_seed" => {
            let s = Codex32::from_seed(
                num(req, "threshold") as u8,
                four(req, "identifier"),
                &bytes(req, "seed"),
            )
            .map_err(err)?;
            Ok(describe(&s))
        }
        "recover" => Ok(describe(
            &Codex32::recover(&strings(req, "shares")?).map_err(err)?,
        )),
        "derive_share" => {
            let index = text(req, "index").as_bytes()[0];
            Ok(describe(
                &Codex32::derive_share(&strings(req, "strings")?, index).map_err(err)?,
            ))
        }
        "split" => {
            let secret = Codex32::parse(text(req, "secret")).map_err(err)?;
            let shares = secret
                .split(
                    num(req, "threshold") as u8,
                    num(req, "shares") as u8,
                    &bytes(req, "fresh"),
                )
                .map_err(err)?;
            Ok(
                json!({"shares": shares.iter().map(|s| s.encode().as_str().to_string()).collect::<Vec<_>>()}),
            )
        }
        "worksheet" => worksheet(&Compute::unchecked(), text(req, "string")),
        "make_checksum" => make(
            &Compute::unchecked(),
            text(req, "prefix"),
            num(req, "chars") as usize,
        ),
        "fresh" => fresh(&Compute::unchecked(), req),
        "correct" => correct(&Compute::unchecked(), req),
        "symbols" => translation_symbols(req),
        "translate" => translate(&Compute::unchecked(), req),
        "checksum_table" => {
            let pair = symbols(text(req, "pair"))?;
            Ok(json!({"row": chars(codex32::checksum_table(pair[0], pair[1]).into_iter())}))
        }
        "identifier_for" => {
            let fp: [u8; 4] = bytes(req, "fingerprint").try_into().expect("4 bytes");
            Ok(json!({"identifier": String::from_utf8_lossy(&codex32::identifier_for(fp))}))
        }
        _ => Err(format!("unknown codex32 op {name}")),
    }
}
