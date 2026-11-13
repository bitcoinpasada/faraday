//! A batch reader and writer for `tools/vectors/aqrv/diff.py`, which compares
//! this library with the second reader (`check.py`) on many codes. It reads
//! JSON lines from standard input and writes one JSON line for each. It runs
//! the library's `recognise_code`, `recognise_file`, `open` and `seal` as they
//! are, Argon2id included: only the process start is shared between cases.
//! The passphrases are test data given on the line.
//!
//! A read line: `{"op":"read","mode":"qr"|"file","payload":HEX,"passphrase":TEXT}`.
//! A seal line: `{"op":"seal","type":TYPE,...the item,"passphrase":TEXT,"step":N,
//! "size":BYTES,"salt":HEX,"fill":N,"padding":HEX}`, where the item is `hex`
//! (types `entropy`, `raw_key`), `text` (`passphrase`, `note`) or `string`
//! (`codex32`).

use std::io::{self, BufRead, Write};

use aqrv::{Item, Layout, NotOpened, Size, Step, open, recognise_code, recognise_file, seal};
use faraday_bip::codex32::Codex32;
use faraday_bip::json::{self, Value, quote};
use faraday_crypto::work::Compute;
use faraday_text::Text;
use zeroize::Zeroizing;

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

fn unhex(s: &str) -> Option<Vec<u8>> {
    if !s.len().is_multiple_of(2) || !s.is_ascii() {
        return None;
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).ok())
        .collect()
}

fn item_json(item: &Item) -> String {
    match item {
        Item::Entropy(b) => format!("{{\"type\":\"entropy\",\"hex\":\"{}\"}}", hex(b)),
        Item::RawKey(k) => format!("{{\"type\":\"raw_key\",\"hex\":\"{}\"}}", hex(&k[..])),
        Item::Passphrase(t) => format!("{{\"type\":\"passphrase\",\"text\":{}}}", quote(t)),
        Item::Note(t) => format!("{{\"type\":\"note\",\"text\":{}}}", quote(t)),
        Item::Codex32(s) => {
            let secret = Codex32::parse(s).map(|c| c.is_secret());
            format!(
                "{{\"type\":\"codex32\",\"string\":{},\"is_secret\":{}}}",
                quote(s),
                secret.map_or("null".to_owned(), |b| b.to_string())
            )
        }
    }
}

/// The rule's name in `check.py`'s words.
fn rule(l: Layout) -> &'static str {
    match l {
        Layout::LenZero | Layout::LenPastRoom => "len",
        Layout::NotUtf8 => "utf8",
        Layout::Codex32Size => "size",
        Layout::Codex32TooLong => "room",
        Layout::Codex32Threshold => "threshold",
        Layout::Codex32ThresholdZeroNotS => "threshold-index",
    }
}

fn read_line(c: &Compute, v: &Value) -> Option<String> {
    let payload = unhex(v.get("payload")?.as_str()?)?;
    let passphrase = v.get("passphrase")?.as_str()?;
    let envelope = match v.get("mode")?.as_str()? {
        "qr" => recognise_code(&payload),
        _ => recognise_file(&payload).map(|_| payload.clone()),
    };
    let Some(envelope) = envelope else {
        return Some("{\"recognised\":false}".to_owned());
    };
    let header = aqrv::recognise(&envelope)?;
    let steps = Step::ALL.iter().position(|s| *s == header.step)?;
    let head = format!(
        "{{\"recognised\":true,\"env\":\"{}\",\"size\":{},\"step\":{steps},",
        hex(&envelope),
        header.size.bytes()
    );
    let tail = match open(c, &envelope, passphrase) {
        Ok(o) => format!("\"answer\":\"opens\",\"item\":{}}}", item_json(&o.item)),
        Err(NotOpened::NoTag) => "\"answer\":\"no_match\"}".to_owned(),
        Err(NotOpened::TwoTags) => "\"answer\":\"ambiguous\"}".to_owned(),
        Err(NotOpened::Layout(kind, l)) => format!(
            "\"answer\":\"layout\",\"kind\":{},\"rule\":\"{}\"}}",
            kind as u8,
            rule(l)
        ),
        Err(other) => format!("\"answer\":\"other {other:?}\"}}"),
    };
    Some(head + &tail)
}

fn item_of(v: &Value) -> Option<Item> {
    let text = || Some(Text::of(v.get("text")?.as_str()?));
    Some(match v.get("type")?.as_str()? {
        "entropy" => Item::Entropy(Zeroizing::new(unhex(v.get("hex")?.as_str()?)?)),
        "raw_key" => Item::RawKey(Zeroizing::new(
            unhex(v.get("hex")?.as_str()?)?.try_into().ok()?,
        )),
        "passphrase" => Item::Passphrase(text()?),
        "note" => Item::Note(text()?),
        "codex32" => Item::Codex32(Text::of(v.get("string")?.as_str()?)),
        _ => return None,
    })
}

fn seal_line(c: &Compute, v: &Value) -> Option<String> {
    let item = item_of(v)?;
    let passphrase = v.get("passphrase")?.as_str()?;
    let step = *Step::ALL.get(usize::try_from(v.get("step")?.as_u64()?).ok()?)?;
    let size = Size::of_len(usize::try_from(v.get("size")?.as_u64()?).ok()?)?;
    let mut pieces = vec![unhex(v.get("salt")?.as_str()?)?];
    if matches!(item, Item::Codex32(_)) {
        pieces.push(vec![u8::try_from(v.get("fill")?.as_u64()?).ok()?]);
    }
    let padding = unhex(v.get("padding")?.as_str()?)?;
    if !padding.is_empty() {
        pieces.push(padding);
    }
    let mut pieces = pieces.into_iter();
    let mut wrong_draw = false;
    let mut random = |buf: &mut [u8]| match pieces.next() {
        Some(p) if p.len() == buf.len() => buf.copy_from_slice(&p),
        _ => wrong_draw = true,
    };
    let made = seal(c, &item, passphrase, step, size, &mut random);
    Some(match made {
        _ if wrong_draw => "{\"drew\":\"other bytes than the given ones\"}".to_owned(),
        Ok(env) => format!("{{\"envelope\":\"{}\"}}", hex(&env)),
        Err(why) => format!("{{\"refused\":\"{why:?}\"}}"),
    })
}

fn main() {
    let c = Compute::new(|_| {});
    let mut out = io::stdout().lock();
    for line in io::stdin().lock().lines() {
        let line = line.expect("standard input");
        let answer = json::parse(&line)
            .ok()
            .and_then(|v| match v.get("op")?.as_str()? {
                "read" => read_line(&c, &v),
                "seal" => seal_line(&c, &v),
                _ => None,
            });
        writeln!(
            out,
            "{}",
            answer.unwrap_or_else(|| "{\"error\":\"bad line\"}".to_owned())
        )
        .expect("standard output");
        out.flush().expect("standard output");
    }
}
