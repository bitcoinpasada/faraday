//! The Argon QR Vault's command line (`https://github.com/bitcoinpasada/aqrv`, `AQRV.md`): make, open and
//! inspect a code, run the test vectors, and remake a code from the text
//! of a printed sheet (§15). The passphrase is never an argument.

mod tty;

use std::io::{self, BufRead, Read};
use std::process::ExitCode;

use aqrv::{
    Item, Kind, NotOpened, Refused, Size, Step, open, recognise, recognise_code, recognise_file,
    seal, to_text, workings,
};
use faraday_bip::bip39::{Language, Mnemonic};
use faraday_bip::codex32::Codex32;
use faraday_bip::json::{self, Value};
use faraday_bip::keys::{MasterKey, Network};
use faraday_codec::png::qr_png;
use faraday_codec::qr::{Ecc, Payload, encode};
use faraday_crypto::work::Compute;
use faraday_crypto::{Secret, SeedBytes};
use zeroize::Zeroizing;

/// The envelope lengths `--size` accepts. A size the library adds is
/// named here; `Size::of_len` says whether it is one.
const SIZE_NUMBERS: [usize; 3] = [32, 53, 62];

const USAGE: &str = "\
usage:
  aqrv make    --kind words|codex32|passphrase|note|key [--size 32|53|62]
               [--memory 64|128|256|512|1024] [--out FILE.aqrv] [--png FILE.png]
               [--scale 8] [--passphrase-stdin]
  aqrv open    FILE|- | --text TEXT [--show] [--passphrase-stdin]
  aqrv inspect FILE|- [--json] [--png FILE.png]
  aqrv vectors [PATH/aqrv.json] [--case NAME] [--skip-large]
  aqrv from-sheet FILE.json --out FILE.aqrv";

/// The default place of the vectors, from the repository's root.
const VECTORS: &str = "tools/vectors/aqrv/aqrv.json";

/// What the 512 MiB and 1 GiB cases start at.
const LARGE_KIB: u64 = 524_288;

/// An exit code and what is said with it.
struct Fail(u8, String);

type Run = Result<(), Fail>;

fn usage(why: impl Into<String>) -> Fail {
    Fail(64, format!("{}\n{USAGE}", why.into()))
}

fn io_fail(what: &str, e: &io::Error) -> Fail {
    Fail(74, format!("{what}: {e}"))
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(Fail(code, why)) => {
            eprintln!("aqrv: {why}");
            ExitCode::from(code)
        }
    }
}

fn run(args: &[String]) -> Run {
    let Some((command, rest)) = args.split_first() else {
        return Err(usage("no command"));
    };
    let c = Compute::new(|_| {});
    match command.as_str() {
        "make" => make(&c, rest),
        "open" => open_command(&c, rest),
        "inspect" => inspect(&c, rest),
        "vectors" => vectors(&c, rest),
        "from-sheet" => from_sheet(rest),
        other => Err(usage(format!("{other} is not a command"))),
    }
}

/// Arguments split into what takes a value, what is a switch, and the
/// rest.
struct Parsed {
    values: Vec<(String, String)>,
    switches: Vec<String>,
    positional: Vec<String>,
}

impl Parsed {
    fn value(&self, name: &str) -> Option<&str> {
        self.values
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, v)| v.as_str())
    }

    fn has(&self, name: &str) -> bool {
        self.switches.iter().any(|s| s == name)
    }
}

fn parse(args: &[String], with_value: &[&str], switches: &[&str]) -> Result<Parsed, Fail> {
    let mut parsed = Parsed {
        values: Vec::new(),
        switches: Vec::new(),
        positional: Vec::new(),
    };
    let mut it = args.iter();
    while let Some(arg) = it.next() {
        if with_value.contains(&arg.as_str()) {
            let Some(v) = it.next() else {
                return Err(usage(format!("{arg} needs a value")));
            };
            if parsed.value(arg).is_some() {
                return Err(usage(format!("{arg} is given twice")));
            }
            parsed.values.push((arg.clone(), v.clone()));
        } else if switches.contains(&arg.as_str()) {
            parsed.switches.push(arg.clone());
        } else if arg.starts_with("--") || (arg.starts_with('-') && arg != "-") {
            return Err(usage(format!("{arg} is not an option here")));
        } else {
            parsed.positional.push(arg.clone());
        }
    }
    Ok(parsed)
}

fn number(p: &Parsed, name: &str) -> Result<Option<usize>, Fail> {
    p.value(name)
        .map(|v| {
            v.parse::<usize>()
                .map_err(|_| usage(format!("{name} is a number")))
        })
        .transpose()
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

fn unhex(s: &str) -> Option<Vec<u8>> {
    if !s.is_ascii() || !s.len().is_multiple_of(2) {
        return None;
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).ok())
        .collect()
}

/// One line of standard input, without its line ending, wiped when
/// dropped.
fn stdin_line(what: &str) -> Result<Zeroizing<String>, Fail> {
    let mut bytes = Zeroizing::new(Vec::new());
    let n = io::stdin()
        .lock()
        .read_until(b'\n', &mut bytes)
        .map_err(|e| io_fail("standard input", &e))?;
    if n == 0 {
        return Err(Fail(74, format!("no {what} on standard input")));
    }
    if bytes.ends_with(b"\n") {
        bytes.pop();
        if bytes.ends_with(b"\r") {
            bytes.pop();
        }
    }
    String::from_utf8(std::mem::take(&mut *bytes))
        .map(Zeroizing::new)
        .map_err(|_| Fail(74, "standard input is not UTF-8".into()))
}

fn tty_line(label: &str) -> Result<Zeroizing<String>, Fail> {
    tty::prompt(label).map_err(|e| io_fail("the terminal", &e))
}

fn size_text(size: Size) -> String {
    format!("{} bytes", size.bytes())
}

fn step_text(step: Step) -> String {
    format!(
        "step {}, {} MiB",
        Step::ALL.iter().position(|s| *s == step).unwrap_or(0),
        step.memory_kib() / 1024
    )
}

/// §3: the QR level each size is written at.
fn ecc(size: Size) -> Ecc {
    match size {
        Size::Tiny | Size::Small => Ecc::Low,
        Size::Large => Ecc::Medium,
    }
}

fn qr_file(c: &Compute, envelope: &[u8], path: &str, scale: usize) -> Run {
    let size = Size::of_len(envelope.len()).ok_or_else(|| Fail(2, "not recognised".into()))?;
    // The 32-byte size is carried in a QR code as its text (§3).
    let text = <[u8; 32]>::try_from(envelope).map(|e| to_text(&e));
    let payload = match &text {
        Ok(text) => Payload::Alphanumeric(text.as_bytes()),
        Err(_) => Payload::Bytes(envelope),
    };
    let matrix =
        encode(c, payload, ecc(size)).map_err(|_| Fail(3, "the code could not be drawn".into()))?;
    std::fs::write(path, qr_png(&matrix, scale)).map_err(|e| io_fail(path, &e))
}

// make

fn make(c: &Compute, args: &[String]) -> Run {
    let p = parse(
        args,
        &["--kind", "--size", "--memory", "--out", "--png", "--scale"],
        &["--passphrase-stdin"],
    )?;
    if !p.positional.is_empty() {
        return Err(usage("make takes no file"));
    }
    let kind = p.value("--kind").ok_or_else(|| usage("--kind is needed"))?;
    let size = match number(&p, "--size")? {
        None => Size::Small,
        Some(n) => SIZE_NUMBERS
            .contains(&n)
            .then(|| Size::of_len(n))
            .flatten()
            .ok_or_else(|| usage(format!("--size is one of {}", numbers(&SIZE_NUMBERS))))?,
    };
    let step = match number(&p, "--memory")? {
        None => Step::DEFAULT,
        Some(n) => Step::ALL
            .into_iter()
            .find(|s| s.memory_kib() / 1024 == n as u32)
            .ok_or_else(|| {
                let all: Vec<usize> = Step::ALL
                    .iter()
                    .map(|s| (s.memory_kib() / 1024) as usize)
                    .collect();
                usage(format!("--memory is one of {}", numbers(&all)))
            })?,
    };
    let scale = number(&p, "--scale")?.unwrap_or(8);

    let line = stdin_line("item")?;
    let item = item_from(c, kind, &line)?;
    drop(line);
    let passphrase = if p.has("--passphrase-stdin") {
        stdin_line("passphrase")?
    } else {
        let first = tty_line("Passphrase: ")?;
        let again = tty_line("Passphrase again: ")?;
        if *first != *again {
            return Err(Fail(64, "the two passphrases differ".into()));
        }
        first
    };

    let mut random = |buf: &mut [u8]| {
        if getrandom::fill(buf).is_err() {
            eprintln!("aqrv: no random bytes");
            std::process::exit(74);
        }
    };
    let envelope = seal(c, &item, &passphrase, step, size, &mut random).map_err(refused)?;

    if let Some(path) = p.value("--out") {
        std::fs::write(path, &envelope).map_err(|e| io_fail(path, &e))?;
    }
    if let Some(path) = p.value("--png") {
        qr_file(c, &envelope, path, scale)?;
    }
    // The 32-byte size's text is what its QR code holds (§3): it is
    // printed beside the file and the picture, and `open --text` takes it.
    if let Ok(bytes) = <[u8; 32]>::try_from(envelope.as_slice()) {
        println!("{}", &*to_text(&bytes));
    } else if p.value("--out").is_none() && p.value("--png").is_none() {
        println!("{}", hex(&envelope));
    }
    Ok(())
}

fn numbers(all: &[usize]) -> String {
    all.iter()
        .map(usize::to_string)
        .collect::<Vec<_>>()
        .join(", ")
}

fn item_from(c: &Compute, kind: &str, line: &str) -> Result<Item, Fail> {
    let refused = |why: &str| Fail(3, format!("not written: {why}"));
    Ok(match kind {
        "words" => {
            let m = Mnemonic::parse(c, Language::English, line)
                .map_err(|_| refused("not a BIP-39 sentence"))?;
            Item::Entropy(Zeroizing::new(m.entropy().expose().as_bytes().to_vec()))
        }
        "key" => {
            let bytes = Zeroizing::new(
                unhex(line).ok_or_else(|| refused("a key is 64 hexadecimal digits"))?,
            );
            let key: [u8; 32] = bytes
                .as_slice()
                .try_into()
                .map_err(|_| refused("a key is 64 hexadecimal digits"))?;
            Item::RawKey(Zeroizing::new(key))
        }
        "codex32" => Item::Codex32(Zeroizing::new(line.to_owned())),
        "passphrase" => Item::Passphrase(Zeroizing::new(line.to_owned())),
        "note" => Item::Note(Zeroizing::new(line.to_owned())),
        other => return Err(usage(format!("{other} is not a kind"))),
    })
}

fn refused(r: Refused) -> Fail {
    match r {
        Refused::NotEnoughMemory => Fail(4, "not enough memory".into()),
        Refused::EmptyPassphrase => Fail(3, "not written: the passphrase is empty".into()),
        Refused::EmptyText => Fail(3, "not written: the text is empty".into()),
        Refused::TooLong => Fail(3, "not written: the text is too long for the size".into()),
        Refused::KindNotHeld => Fail(
            3,
            "not written: the 32-byte size does not hold this kind".into(),
        ),
        Refused::EntropyLength => Fail(3, "not written: the seed has no word count".into()),
        Refused::Codex32(_) => Fail(3, "not written: not a codex32 string".into()),
        Refused::Codex32TooLongForSmall => Fail(
            3,
            "not written: this codex32 string does not fit the 53-byte size".into(),
        ),
    }
}

// open and inspect

/// The envelope a text of 47 characters stands for (§3, §9).
fn envelope_of_text(text: &str) -> Option<Vec<u8>> {
    recognise_code(text.as_bytes())
}

/// The envelope's bytes from a file, a PNG or standard input. A file holds
/// the bytes (§9.1), or the 32-byte size's text with at most its one line
/// ending: a space or any other character is part of the text.
fn load(c: &Compute, source: &str) -> Result<Vec<u8>, Fail> {
    let bytes = if source == "-" {
        let mut all = Vec::new();
        io::stdin()
            .lock()
            .take(1 << 24)
            .read_to_end(&mut all)
            .map_err(|e| io_fail("standard input", &e))?;
        all
    } else {
        std::fs::read(source).map_err(|e| io_fail(source, &e))?
    };
    if bytes.starts_with(&[0x89, b'P', b'N', b'G']) {
        let codes = faraday_files::qr_in_png(c, &bytes)
            .map_err(|_| Fail(2, "not recognised: no code in the picture".into()))?;
        return codes
            .iter()
            .find_map(|code| recognise_code(code))
            .ok_or_else(|| Fail(2, "not recognised".into()));
    }
    if recognise_file(&bytes).is_some() {
        return Ok(bytes);
    }
    let line = bytes.strip_suffix(b"\n").unwrap_or(&bytes);
    let line = line.strip_suffix(b"\r").unwrap_or(line);
    let text = std::str::from_utf8(line).ok();
    Ok(text.and_then(envelope_of_text).unwrap_or(bytes))
}

fn header_of(envelope: &[u8]) -> Result<aqrv::Header, Fail> {
    recognise(envelope).ok_or_else(|| Fail(2, "not recognised".into()))
}

fn open_command(c: &Compute, args: &[String]) -> Run {
    let p = parse(args, &["--text"], &["--show", "--passphrase-stdin"])?;
    let envelope = match (p.value("--text"), p.positional.as_slice()) {
        (Some(text), []) => {
            envelope_of_text(text).ok_or_else(|| Fail(2, "not recognised".into()))?
        }
        (None, [source]) => {
            if source == "-" && p.has("--passphrase-stdin") {
                return Err(usage(
                    "the code and the passphrase cannot both be on standard input",
                ));
            }
            load(c, source)?
        }
        _ => {
            return Err(usage(
                "open takes one file, or - for standard input, or --text",
            ));
        }
    };
    let header = header_of(&envelope)?;
    eprintln!(
        "{}, {}, memory {} MiB",
        size_text(header.size),
        step_text(header.step),
        header.step.memory_kib() / 1024
    );
    let passphrase = if p.has("--passphrase-stdin") {
        stdin_line("passphrase")?
    } else {
        tty_line("Passphrase: ")?
    };
    let opened = match open(c, &envelope, &passphrase) {
        Ok(o) => o,
        Err(e) => return Err(not_opened(e)),
    };
    show_item(c, &opened.item);
    if p.has("--show") {
        let w = workings(c, &envelope, &passphrase).map_err(not_opened)?;
        println!("pw {}", hex(&w.pw));
        println!("PRK {}", hex(&w.prk));
        println!("K_enc {}", hex(&w.k_enc));
        println!("K_mac {}", hex(&w.k_mac));
        println!("keystream {}", hex(&w.keystream));
        println!("P {}", hex(&w.p));
        println!("C {}", hex(&w.c));
        match &w.message {
            Some(m) => println!("hmac_message {}", hex(m)),
            None => println!("hmac_message none"),
        }
        for (kind, tag) in &w.tags {
            println!(
                "tag 0x{:02X} {}",
                *kind as u8,
                hex(&tag[..header.size.tag_len()])
            );
        }
    }
    Ok(())
}

fn not_opened(e: NotOpened) -> Fail {
    match e {
        NotOpened::NotRecognised => Fail(2, "not recognised".into()),
        NotOpened::NotEnoughMemory => Fail(4, "not enough memory".into()),
        NotOpened::NoTag | NotOpened::TwoTags | NotOpened::Layout(..) => Fail(
            1,
            "does not open: the passphrase is wrong or the code is damaged".into(),
        ),
    }
}

/// §7: what a reader shows, by kind.
fn show_item(c: &Compute, item: &Item) {
    match item {
        Item::Entropy(bytes) => {
            if let Ok(m) = Mnemonic::from_entropy(Language::English, bytes) {
                let words: Vec<&str> = m
                    .indices()
                    .iter()
                    .map(|i| Language::English.word(*i))
                    .collect();
                println!("words {}", words.join(" "));
                if let Ok(seed) = m.to_seed(c, b"") {
                    let key = MasterKey::from_seed(c, &seed, Network::Mainnet);
                    println!("fingerprint {}", key.fingerprint());
                }
            }
        }
        Item::Codex32(string) => {
            println!("codex32 {}", string.as_str());
            if let Ok(s) = Codex32::parse(string) {
                println!("threshold {}", s.threshold());
                println!("identifier {}", String::from_utf8_lossy(&s.identifier()));
                println!("index {}", char::from(s.share_index()));
                println!("{}", if s.is_secret() { "secret" } else { "share" });
                if s.is_secret()
                    && let Ok(seed) = s.to_seed()
                    && let Some(bytes) = SeedBytes::new(seed.expose().bytes())
                {
                    let key = MasterKey::from_seed_bytes(c, &Secret::new(bytes), Network::Mainnet);
                    println!("fingerprint {}", key.fingerprint());
                }
            }
        }
        Item::Passphrase(t) => println!("passphrase {}", t.as_str()),
        Item::Note(t) => println!("note {}", t.as_str()),
        Item::RawKey(k) => println!("key {}", hex(&k[..])),
    }
}

fn inspect(c: &Compute, args: &[String]) -> Run {
    let p = parse(args, &["--png"], &["--json"])?;
    let [source] = p.positional.as_slice() else {
        return Err(usage("inspect takes one file, or - for standard input"));
    };
    let envelope = load(c, source)?;
    let header = header_of(&envelope)?;
    let size = header.size;
    let salt = &envelope[1..1 + size.salt_len()];
    let ct = &envelope[1 + size.salt_len()..][..size.room()];
    let tag = &envelope[1 + size.salt_len() + size.room()..];
    if p.has("--json") {
        println!(
            "{{\"format\":\"AQRV\",\"version\":1,\"memory_mib\":{},\"passes\":3,\"lanes\":4,\
             \"salt\":\"{}\",\"ciphertext\":\"{}\",\"tag\":\"{}\"}}",
            header.step.memory_kib() / 1024,
            hex(salt),
            hex(ct),
            hex(tag)
        );
    } else {
        println!("header 0x{:02X}", envelope[0]);
        println!("size {}", size_text(size));
        println!(
            "step {}",
            step_text(header.step).trim_start_matches("step ")
        );
        println!("memory {} MiB", header.step.memory_kib() / 1024);
        println!("salt {}", hex(salt));
        println!("C {}", hex(ct));
        println!("T {}", hex(tag));
    }
    if let Some(path) = p.value("--png") {
        qr_file(c, &envelope, path, 8)?;
    }
    Ok(())
}

// from-sheet

fn from_sheet(args: &[String]) -> Run {
    let p = parse(args, &["--out"], &[])?;
    let [file] = p.positional.as_slice() else {
        return Err(usage("from-sheet takes one file"));
    };
    let out = p.value("--out").ok_or_else(|| usage("--out is needed"))?;
    let text = std::fs::read_to_string(file).map_err(|e| io_fail(file, &e))?;
    let not = || {
        Fail(
            2,
            "not recognised: the sheet's values do not make a code".into(),
        )
    };
    let sheet = json::parse(&text).map_err(|_| not())?;
    let field = |k: &str| sheet.get(k).and_then(Value::as_str).ok_or_else(not);
    let num = |k: &str| sheet.get(k).and_then(Value::as_u64).ok_or_else(not);
    if field("format")? != "AQRV"
        || num("version")? != 1
        || num("passes")? != 3
        || num("lanes")? != 4
    {
        return Err(not());
    }
    let mib = num("memory_mib")?;
    let step = Step::ALL
        .into_iter()
        .find(|s| u64::from(s.memory_kib()) / 1024 == mib)
        .ok_or_else(not)?;
    let salt = unhex(field("salt")?).ok_or_else(not)?;
    let ct = unhex(field("ciphertext")?).ok_or_else(not)?;
    let tag = unhex(field("tag")?).ok_or_else(not)?;
    let mut envelope = vec![step.header()];
    envelope.extend_from_slice(&salt);
    envelope.extend_from_slice(&ct);
    envelope.extend_from_slice(&tag);
    let size = header_of(&envelope)?.size;
    if salt.len() != size.salt_len() || ct.len() != size.room() || tag.len() != size.tag_len() {
        return Err(not());
    }
    std::fs::write(out, &envelope).map_err(|e| io_fail(out, &e))
}

// vectors

fn vectors(c: &Compute, args: &[String]) -> Run {
    let p = parse(args, &["--case"], &["--skip-large"])?;
    let path = match p.positional.as_slice() {
        [] => VECTORS,
        [one] => one.as_str(),
        _ => return Err(usage("vectors takes one file")),
    };
    let text = std::fs::read_to_string(path).map_err(|e| io_fail(path, &e))?;
    let file = json::parse(&text).map_err(|_| Fail(74, format!("{path} is not JSON")))?;
    let cases = file
        .get("cases")
        .and_then(Value::as_array)
        .ok_or_else(|| Fail(74, format!("{path} has no cases")))?;
    let only = p.value("--case");
    if let Some(name) = only
        && !cases.iter().any(|case| text_of(case, "name") == name)
    {
        return Err(usage(format!("no case named {name}")));
    }
    let (mut agree, mut differ) = (0usize, 0usize);
    let mut tally = |name: &str, result: Result<(), String>| match result {
        Ok(()) => {
            agree += 1;
            println!("agrees {name}");
        }
        Err(why) => {
            differ += 1;
            println!("differs {name}: {why}");
        }
    };
    for case in cases {
        let name = text_of(case, "name");
        if only.is_some_and(|n| n != name) {
            continue;
        }
        let large = case
            .get("memory_kib")
            .and_then(Value::as_u64)
            .is_some_and(|k| k >= LARGE_KIB);
        if p.has("--skip-large") && large {
            println!("skipped {name}");
            continue;
        }
        tally(name, run_case(c, case));
    }
    if only.is_none() {
        let refusals = file.get("writer_refuses").and_then(Value::as_array);
        for entry in refusals.unwrap_or(&[]) {
            let name = format!("writer refuses {}", text_of(entry, "reason"));
            tally(&name, run_refusal(c, entry));
        }
    }
    println!("{} agree, {} differ", agree, differ);
    if differ > 0 {
        return Err(Fail(5, format!("{differ} vectors differ")));
    }
    Ok(())
}

fn text_of<'a>(v: &'a Value, key: &str) -> &'a str {
    v.get(key).and_then(Value::as_str).unwrap_or("")
}

fn run_refusal(c: &Compute, entry: &Value) -> Result<(), String> {
    // An entry with a size and an item is refused at that size.
    if let (Some(size), Some(item)) = (entry.get("size").and_then(Value::as_u64), entry.get("item"))
    {
        let size = Size::of_len(size as usize).ok_or("a size the library does not know")?;
        let got = seal(
            c,
            &expected_item(item)?,
            "pass",
            Step::Mib64,
            size,
            &mut |_| unreachable!("a refusal comes before any draw"),
        );
        return match got {
            Err(Refused::TooLong | Refused::KindNotHeld) => Ok(()),
            _ => Err("written".into()),
        };
    }
    for number in SIZE_NUMBERS {
        let size = Size::of_len(number).ok_or("a size the library does not know")?;
        let item = Item::Codex32(Zeroizing::new(text_of(entry, "string").to_owned()));
        let got = seal(c, &item, "pass", Step::Mib64, size, &mut |_| {
            unreachable!("a refusal comes before any draw")
        });
        // The 32-byte size does not hold the kind, so it refuses that first.
        let right = match size {
            Size::Tiny => matches!(got, Err(Refused::KindNotHeld)),
            _ => matches!(got, Err(Refused::Codex32(_))),
        };
        if !right {
            return Err(format!("written at {number} bytes"));
        }
    }
    Ok(())
}

fn expected_item(item: &Value) -> Result<Item, String> {
    let bytes = || unhex(text_of(item, "hex")).ok_or("hex");
    Ok(match text_of(item, "type") {
        "entropy" => Item::Entropy(Zeroizing::new(bytes()?)),
        "raw_key" => Item::RawKey(Zeroizing::new(
            bytes()?.try_into().map_err(|_| "a key of another length")?,
        )),
        "passphrase" => Item::Passphrase(Zeroizing::new(text_of(item, "text").to_owned())),
        "note" => Item::Note(Zeroizing::new(text_of(item, "text").to_owned())),
        "codex32" => Item::Codex32(Zeroizing::new(text_of(item, "string").to_owned())),
        other => return Err(format!("an item of type {other}")),
    })
}

/// The kind and the bytes or text of an item, to compare two.
fn shown(item: &Item) -> (Option<Kind>, Vec<u8>) {
    let bytes = match item {
        Item::Entropy(b) => b.to_vec(),
        Item::RawKey(k) => k.to_vec(),
        Item::Passphrase(t) | Item::Note(t) | Item::Codex32(t) => t.as_bytes().to_vec(),
    };
    (item.kind(), bytes)
}

/// §11: a case opens to what it says and seals again to its envelope, does
/// not open, or is not recognised.
fn run_case(c: &Compute, case: &Value) -> Result<(), String> {
    let envelope = unhex(text_of(case, "envelope")).ok_or("the envelope is not hex")?;
    let passphrase = text_of(case, "passphrase");
    let expect = case.get("expect").ok_or("no expectation")?;
    match text_of(expect, "outcome") {
        "opens" => {
            let opened = open(c, &envelope, passphrase).map_err(|e| format!("{e:?}"))?;
            let want = expected_item(expect.get("item").ok_or("no item")?)?;
            if shown(&opened.item) != shown(&want) {
                return Err("another item".into());
            }
            let mut draws = vec![unhex(text_of(case, "salt")).ok_or("salt")?];
            if let Some(fill) = case.get("last_byte_fill").and_then(Value::as_str) {
                let fill = u8::from_str_radix(fill.trim_start_matches("0x"), 16)
                    .map_err(|_| "last_byte_fill")?;
                draws.push(vec![fill]);
            }
            let padding = unhex(text_of(case, "padding")).ok_or("padding")?;
            if !padding.is_empty() {
                draws.push(padding);
            }
            draws.reverse();
            let mut wrong = false;
            let mut random = |buf: &mut [u8]| match draws.pop() {
                Some(next) if next.len() == buf.len() => buf.copy_from_slice(&next),
                _ => wrong = true,
            };
            let sealed = seal(
                c,
                &want,
                passphrase,
                opened.header.step,
                opened.header.size,
                &mut random,
            )
            .map_err(|e| format!("sealing again: {e:?}"))?;
            if wrong || !draws.is_empty() {
                return Err("the draws do not fit".into());
            }
            if sealed != envelope {
                return Err("sealing again gives another envelope".into());
            }
            Ok(())
        }
        "does_not_open" => match open(c, &envelope, passphrase) {
            Err(NotOpened::NoTag | NotOpened::Layout(..)) => Ok(()),
            Err(other) => Err(format!("{other:?}")),
            Ok(_) => Err("it opens".into()),
        },
        "not_recognised" => match open(c, &envelope, passphrase) {
            Err(NotOpened::NotRecognised) if recognise(&envelope).is_none() => Ok(()),
            _ => Err("it is recognised".into()),
        },
        other => Err(format!("an outcome {other}")),
    }
}
