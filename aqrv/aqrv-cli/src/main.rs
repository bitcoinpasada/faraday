//! The Argon QR Vault's command line (`https://github.com/bitcoinpasada/aqrv`, `AQRV.md`): make, open and
//! inspect a code, run the test vectors, and remake a code from the text
//! of a printed sheet (§15). The passphrase is never an argument.

mod cli;
mod tty;
mod vectors;

use std::io::{self, BufRead, Read};
use std::process::ExitCode;

use aqrv::{
    Item, NotOpened, Refused, Size, Step, open, recognise, recognise_code, recognise_file, seal,
    to_text, workings,
};
use faraday_bip::bip39::{Language, Mnemonic};
use faraday_bip::codex32::Codex32;
use faraday_bip::json::{self, Value};
use faraday_bip::keys::{MasterKey, Network};
use faraday_codec::png::qr_png;
use faraday_codec::qr::{Ecc, Payload, encode};
use faraday_crypto::work::Compute;
use faraday_crypto::{Secret, SeedBytes};
use faraday_text::{Text, text};
use zeroize::Zeroizing;

use cli::{
    Args, Fail, Run, SIZE_NUMBERS, arguments, fail, io_fail, number, numbers, parse, unhex, usage,
};
use vectors::vectors;

fn main() -> ExitCode {
    let args = match arguments() {
        Ok(args) => args,
        Err(Fail(code, why)) => {
            eprintln!("aqrv: {}", why.as_str());
            return ExitCode::from(code);
        }
    };
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(Fail(code, why)) => {
            eprintln!("aqrv: {}", why.as_str());
            ExitCode::from(code)
        }
    }
}

fn run(args: &Args) -> Run {
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
        other => Err(usage(&text!("{other} is not a command"))),
    }
}

fn hex(b: &[u8]) -> Text {
    let mut out = Text::with_room(b.len() * 2);
    for x in b {
        let _ = core::fmt::Write::write_fmt(&mut out, format_args!("{x:02x}"));
    }
    out
}

/// One line of standard input, without its line ending, wiped when
/// dropped.
fn stdin_line(what: &str) -> Result<Text, Fail> {
    let mut bytes = Zeroizing::new(Vec::new());
    let n = io::stdin()
        .lock()
        .read_until(b'\n', &mut bytes)
        .map_err(|e| io_fail("standard input", &e))?;
    if n == 0 {
        return Err(Fail(74, text!("no {what} on standard input")));
    }
    if bytes.ends_with(b"\n") {
        bytes.pop();
        if bytes.ends_with(b"\r") {
            bytes.pop();
        }
    }
    core::str::from_utf8(&bytes)
        .map(Text::of)
        .map_err(|_| fail(74, "standard input is not UTF-8"))
}

fn tty_line(label: &str) -> Result<Text, Fail> {
    tty::prompt(label).map_err(|e| io_fail("the terminal", &e))
}

fn size_text(size: Size) -> Text {
    text!("{} bytes", size.bytes())
}

fn step_text(step: Step) -> Text {
    text!(
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
    let size = Size::of_len(envelope.len()).ok_or_else(|| fail(2, "not recognised"))?;
    // The 32-byte size is carried in a QR code as its text (§3).
    let text = <[u8; 32]>::try_from(envelope).map(|e| to_text(&e));
    let payload = match &text {
        Ok(text) => Payload::Alphanumeric(text.as_bytes()),
        Err(_) => Payload::Bytes(envelope),
    };
    let matrix =
        encode(c, payload, ecc(size)).map_err(|_| fail(3, "the code could not be drawn"))?;
    std::fs::write(path, qr_png(&matrix, scale)).map_err(|e| io_fail(path, &e))
}

// make

fn make(c: &Compute, args: &Args) -> Run {
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
            .ok_or_else(|| usage(&text!("--size is one of {}", numbers(&SIZE_NUMBERS))))?,
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
                usage(&text!("--memory is one of {}", numbers(&all)))
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
            return Err(fail(64, "the two passphrases differ"));
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
        println!("{}", hex(&envelope).as_str());
    }
    Ok(())
}

fn item_from(c: &Compute, kind: &str, line: &str) -> Result<Item, Fail> {
    let refused = |why: &str| Fail(3, text!("not written: {why}"));
    Ok(match kind {
        "words" => {
            let m = Mnemonic::parse(c, Language::English, line)
                .map_err(|_| refused("not a BIP-39 sentence"))?;
            Item::Entropy(Zeroizing::new(m.entropy().expose().as_bytes().to_vec()))
        }
        "key" => {
            let mut key = Zeroizing::new([0u8; 32]);
            if faraday_crypto::hex::decode_into(line, &mut *key) != Some(32) {
                return Err(refused("a key is 64 hexadecimal digits"));
            }
            Item::RawKey(key)
        }
        "codex32" => Item::Codex32(Text::of(line)),
        "passphrase" => Item::Passphrase(Text::of(line)),
        "note" => Item::Note(Text::of(line)),
        other => return Err(usage(&text!("{other} is not a kind"))),
    })
}

fn refused(r: Refused) -> Fail {
    match r {
        Refused::NotEnoughMemory => fail(4, "not enough memory"),
        Refused::EmptyPassphrase => fail(3, "not written: the passphrase is empty"),
        Refused::EmptyText => fail(3, "not written: the text is empty"),
        Refused::TooLong => fail(3, "not written: the text is too long for the size"),
        Refused::KindNotHeld => fail(3, "not written: the 32-byte size does not hold this kind"),
        Refused::EntropyLength => fail(3, "not written: the seed has no word count"),
        Refused::Codex32(_) => fail(3, "not written: not a codex32 string"),
        Refused::Codex32TooLongForSmall => fail(
            3,
            "not written: this codex32 string does not fit the 53-byte size",
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
            .map_err(|_| fail(2, "not recognised: no code in the picture"))?;
        return codes
            .iter()
            .find_map(|code| recognise_code(code))
            .ok_or_else(|| fail(2, "not recognised"));
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
    recognise(envelope).ok_or_else(|| fail(2, "not recognised"))
}

fn open_command(c: &Compute, args: &Args) -> Run {
    let p = parse(args, &["--text"], &["--show", "--passphrase-stdin"])?;
    let envelope = match (p.value("--text"), p.positional.as_slice()) {
        (Some(text), []) => envelope_of_text(text).ok_or_else(|| fail(2, "not recognised"))?,
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
        size_text(header.size).as_str(),
        step_text(header.step).as_str(),
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
        println!("pw {}", hex(&w.pw).as_str());
        println!("PRK {}", hex(&w.prk).as_str());
        println!("K_enc {}", hex(&w.k_enc).as_str());
        println!("K_mac {}", hex(&w.k_mac).as_str());
        println!("keystream {}", hex(&w.keystream).as_str());
        println!("P {}", hex(&w.p).as_str());
        println!("C {}", hex(&w.c).as_str());
        match &w.message {
            Some(m) => println!("hmac_message {}", hex(m).as_str()),
            None => println!("hmac_message none"),
        }
        for (kind, tag) in &w.tags {
            println!(
                "tag 0x{:02X} {}",
                *kind as u8,
                hex(&tag[..header.size.tag_len()]).as_str()
            );
        }
    }
    Ok(())
}

fn not_opened(e: NotOpened) -> Fail {
    match e {
        NotOpened::NotRecognised => fail(2, "not recognised"),
        NotOpened::NotEnoughMemory => fail(4, "not enough memory"),
        NotOpened::NoTag | NotOpened::TwoTags | NotOpened::Layout(..) => fail(
            1,
            "does not open: the passphrase is wrong or the code is damaged",
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
                println!("words {}", &*faraday_text::Text::joined(&words, " "));
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
                println!(
                    "identifier {}",
                    Text::from_utf8_lossy(&s.identifier()).as_str()
                );
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
        Item::RawKey(k) => println!("key {}", hex(&k[..]).as_str()),
    }
}

fn inspect(c: &Compute, args: &Args) -> Run {
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
            hex(salt).as_str(),
            hex(ct).as_str(),
            hex(tag).as_str()
        );
    } else {
        println!("header 0x{:02X}", envelope[0]);
        println!("size {}", size_text(size).as_str());
        println!(
            "step {}",
            step_text(header.step).as_str().trim_start_matches("step ")
        );
        println!("memory {} MiB", header.step.memory_kib() / 1024);
        println!("salt {}", hex(salt).as_str());
        println!("C {}", hex(ct).as_str());
        println!("T {}", hex(tag).as_str());
    }
    if let Some(path) = p.value("--png") {
        qr_file(c, &envelope, path, 8)?;
    }
    Ok(())
}

// from-sheet

fn from_sheet(args: &Args) -> Run {
    let p = parse(args, &["--out"], &[])?;
    let [file] = p.positional.as_slice() else {
        return Err(usage("from-sheet takes one file"));
    };
    let out = p.value("--out").ok_or_else(|| usage("--out is needed"))?;
    let text = std::fs::read_to_string(file).map_err(|e| io_fail(file, &e))?;
    let not = || fail(2, "not recognised: the sheet's values do not make a code");
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
