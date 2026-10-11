//! Key explorer: mnemonic in, fingerprint, account xpubs, descriptors and
//! addresses out.
//!
//! ```text
//! cargo run -p osk-bip --example explore -- --words "abandon … about" [--passphrase X] [--network testnet] [--count 5]
//! cargo run -p osk-bip --example explore -- --entropy <hex>
//! cargo run -p osk-bip --example explore -- --last-word "abandon … (11 or 23 words)"
//! ```
//!
//! This is a demo for humans at a terminal. It prints secrets. Examples
//! are outside the secret lint's scope (`src/` only) and may use `std`.

use std::process::ExitCode;

use osk_bip::bip39::{Language, Mnemonic, last_word_candidates};
use osk_bip::keys::{MasterKey, Network, ScriptType};
use unicode_normalization::UnicodeNormalization;

const USAGE: &str = "\
usage: explore --words \"<12..24 words>\" [--passphrase <ascii>] [--network <mainnet|testnet|signet|regtest>] [--count <n>]
       explore --entropy <hex> [--passphrase <ascii>] [--network <name>] [--count <n>]
       explore --last-word \"<11|14|17|20|23 words>\"";

enum Source {
    Words(String),
    Entropy(String),
    LastWord(String),
}

struct Args {
    source: Source,
    passphrase: String,
    network: Network,
    count: u32,
}

fn parse_args() -> Result<Args, String> {
    let mut args = std::env::args().skip(1);
    let mut source = None;
    let mut passphrase = String::new();
    let mut network = Network::Mainnet;
    let mut count = 5u32;

    while let Some(flag) = args.next() {
        let mut value = |name: &str| args.next().ok_or_else(|| format!("{name} needs a value"));
        match flag.as_str() {
            "--words" => source = Some(Source::Words(value("--words")?)),
            "--entropy" => source = Some(Source::Entropy(value("--entropy")?)),
            "--last-word" => source = Some(Source::LastWord(value("--last-word")?)),
            "--passphrase" => passphrase = value("--passphrase")?,
            "--network" => {
                let name = value("--network")?;
                network = Network::from_name(&name).ok_or(format!("unknown network {name}"))?;
            }
            "--count" => {
                let n = value("--count")?;
                count = n.parse().map_err(|_| format!("bad count {n}"))?;
            }
            "-h" | "--help" => return Err(USAGE.into()),
            other => return Err(format!("unknown argument {other}\n{USAGE}")),
        }
    }
    let source = source
        .ok_or_else(|| format!("one of --words, --entropy or --last-word is required\n{USAGE}"))?;
    Ok(Args {
        source,
        passphrase,
        network,
        count,
    })
}

/// NFKD form of typed text, so a precomposed "á" matches the wordlists,
/// which are stored decomposed. The core avoids Unicode tables (its
/// keyboard only offers listed words); this demo takes free text and so
/// uses the test-only `unicode-normalization` crate.
fn nfkd(text: &str) -> String {
    text.nfkd().collect()
}

/// The first language whose wordlist contains every word.
fn detect_language(words: &[&str]) -> Option<Language> {
    Language::ALL
        .into_iter()
        .find(|lang| words.iter().all(|w| lang.index_of(w).is_some()))
}

fn unhex(s: &str) -> Result<Vec<u8>, String> {
    if !s.len().is_multiple_of(2) {
        return Err("entropy hex has odd length".into());
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).map_err(|_| format!("bad hex at {i}")))
        .collect()
}

/// Groups of four characters separated by spaces, for reading aloud or
/// comparing against a screen.
fn chunked(s: &str) -> String {
    s.as_bytes()
        .chunks(4)
        .map(|c| std::str::from_utf8(c).unwrap())
        .collect::<Vec<_>>()
        .join(" ")
}

fn sentence(m: &Mnemonic) -> String {
    let lang = m.language();
    m.indices()
        .iter()
        .map(|&i| lang.word(i))
        .collect::<Vec<_>>()
        .join(lang.separator())
}

fn run(args: Args) -> Result<(), String> {
    let mnemonic = match &args.source {
        Source::LastWord(text) => {
            let text = nfkd(text);
            let words: Vec<&str> = text.split_whitespace().collect();
            let lang = detect_language(&words).ok_or("no wordlist contains every word")?;
            let indices: Vec<u16> = words.iter().map(|w| lang.index_of(w).unwrap()).collect();
            let candidates = last_word_candidates(&indices).map_err(|e| e.to_string())?;
            println!("{} words given, language {}", words.len(), lang.name());
            println!("{} valid final words:", candidates.len());
            for idx in candidates {
                println!("  {:4}  {}", idx + 1, lang.word(idx));
            }
            return Ok(());
        }
        Source::Words(text) => {
            let text = nfkd(text);
            let words: Vec<&str> = text.split_whitespace().collect();
            let lang = detect_language(&words).ok_or("no wordlist contains every word")?;
            let m = Mnemonic::parse(lang, &text).map_err(|e| e.to_string())?;
            println!("{} words, language {}", m.word_count(), lang.name());
            m
        }
        Source::Entropy(hex) => {
            let entropy = unhex(hex)?;
            let m =
                Mnemonic::from_entropy(Language::English, &entropy).map_err(|e| e.to_string())?;
            println!(
                "{} bytes of entropy, {} words, language english",
                entropy.len(),
                m.word_count()
            );
            println!("mnemonic: {}", sentence(&m));
            m
        }
    };

    let seed = mnemonic
        .to_seed(args.passphrase.as_bytes())
        .map_err(|e| e.to_string())?;
    let master = MasterKey::from_seed(&seed, args.network);
    println!("network: {}", args.network);
    println!(
        "passphrase: {}",
        if args.passphrase.is_empty() {
            "(none)"
        } else {
            "(set)"
        }
    );
    println!("master fingerprint: {}", master.fingerprint());

    for script_type in ScriptType::ALL {
        let account = master
            .account_xpub(script_type, 0)
            .map_err(|e| e.to_string())?;
        println!();
        println!("== {} (BIP-{}) ==", script_type, script_type.purpose());
        print!("path: m");
        for child in account.path() {
            print!("/{child:#}");
        }
        println!();
        println!("xpub: {}", account.xpub_string());
        let slip = account.slip132_string();
        if slip != account.xpub_string() {
            println!("slip132: {slip}");
        }
        println!("descriptor: {}", account.descriptor());
        for (change, label) in [(false, "receive"), (true, "change")] {
            for index in 0..args.count {
                let address = account.address(change, index).map_err(|e| e.to_string())?;
                println!("  {label} {index}: {}", chunked(&address.to_string()));
            }
        }
    }
    Ok(())
}

fn main() -> ExitCode {
    println!("Demo tool: prints secrets to the terminal. Use test seeds only.");
    let args = match parse_args() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::FAILURE;
        }
    };
    match run(args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}
