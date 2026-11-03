//! rust-codex32's answers, one request a line on standard input, one
//! answer a line on standard output (`ok ...` or `err ...`), for the
//! cross-check's `codex32` area. Words are separated by one space.
//!
//!     parse <string>                        ok <threshold> <identifier> <index> <payload as hex>
//!     from_seed <k> <identifier> <seed hex> ok <string>
//!     make <string without its checksum>    ok <string>
//!     fresh <targets> <string without checksum>...
//!                                           ok <made strings> | <derived strings> | <the secret>
//!
//! `parse` is `Codex32String::from_string` and `Parts::data`; the
//! threshold, identifier and index are read from the string it accepted.
//! `from_seed` is `Codex32String::from_seed` with the index `s`; `make`
//! is `from_unchecksummed_string`; `fresh` makes each string so, then
//! `interpolate_at` each target index (a run of characters, `-` for
//! none) and index `s`. Nothing here decides what the crate does not:
//! the threshold, the share index of a threshold of 0 and the lengths
//! are the crate's.

use codex32::{Codex32String, Fe};
use std::io::{BufRead, Write};

fn fe(c: char) -> Result<Fe, String> {
    Fe::from_char(c).map_err(|e| format!("{e:?}"))
}

fn unhex(s: &str) -> Result<Vec<u8>, String> {
    if s.len() % 2 != 0 || !s.is_ascii() {
        return Err("odd hex".into());
    }
    (0..s.len() / 2)
        .map(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16).map_err(|e| e.to_string()))
        .collect()
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn parse(words: &[&str]) -> Result<String, String> {
    let [s] = words else { return Err("parse: one string".into()) };
    let got = Codex32String::from_string((*s).to_string()).map_err(|e| format!("{e:?}"))?;
    // The crate accepted it: after the last `1` come the threshold,
    // four identifier characters and the share index.
    let tail = s.rsplit('1').next().ok_or("no separator")?.to_ascii_lowercase();
    Ok(format!("{} {} {} {}", &tail[..1], &tail[1..5], &tail[5..6], hex(&got.parts().data())))
}

fn from_seed(words: &[&str]) -> Result<String, String> {
    let [k, id, seed] = words else { return Err("from_seed: k, identifier, seed".into()) };
    let k: usize = k.parse().map_err(|_| "threshold".to_string())?;
    let made = Codex32String::from_seed("ms", k, id, Fe::S, &unhex(seed)?)
        .map_err(|e| format!("{e:?}"))?;
    Ok(made.to_string())
}

fn make(words: &[&str]) -> Result<String, String> {
    let [s] = words else { return Err("make: one string".into()) };
    let made = Codex32String::from_unchecksummed_string((*s).to_string())
        .map_err(|e| format!("{e:?}"))?;
    Ok(made.to_string())
}

fn fresh(words: &[&str]) -> Result<String, String> {
    let Some((targets, strings)) = words.split_first() else {
        return Err("fresh: targets, strings".into());
    };
    let made = strings
        .iter()
        .map(|s| Codex32String::from_unchecksummed_string((*s).to_string()))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("{e:?}"))?;
    let mut derived = Vec::new();
    for t in targets.chars().filter(|c| *c != '-') {
        derived.push(Codex32String::interpolate_at(&made, fe(t)?).map_err(|e| format!("{e:?}"))?);
    }
    let secret = Codex32String::interpolate_at(&made, Fe::S).map_err(|e| format!("{e:?}"))?;
    let join = |v: &[Codex32String]| v.iter().map(ToString::to_string).collect::<Vec<_>>().join(" ");
    Ok(format!("{} | {} | {secret}", join(&made), join(&derived)))
}

fn main() {
    let stdin = std::io::stdin();
    let mut out = std::io::stdout().lock();
    for line in stdin.lock().lines() {
        let line = line.expect("a line");
        let words: Vec<&str> = line.split(' ').filter(|w| !w.is_empty()).collect();
        // A panic inside the crate on an input it should have refused
        // is told as a refusal.
        let answer = std::panic::catch_unwind(|| match words.split_first() {
            Some((&"parse", rest)) => parse(rest),
            Some((&"from_seed", rest)) => from_seed(rest),
            Some((&"make", rest)) => make(rest),
            Some((&"fresh", rest)) => fresh(rest),
            _ => Err(format!("unknown request: {line}")),
        })
        .unwrap_or_else(|_| Err("the crate panicked".into()));
        match answer {
            Ok(a) => writeln!(out, "ok {a}"),
            Err(e) => writeln!(out, "err {e}"),
        }
        .expect("standard output");
        out.flush().expect("standard output");
    }
}
