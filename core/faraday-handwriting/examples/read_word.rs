//! Reads the word in a binary PGM (P5, 8 bits) crop of one handwritten line.
//!
//! `read_word <file.pgm> [bip39|slip39]`: prints `clear <word>` or
//! `unclear <word> <word> ...` (the likeliest first; none if the crop holds
//! no word). The word list defaults to the English BIP39 list.

use faraday_handwriting::{Reading, bip39_words, read_crop, slip39_words};

/// Next whitespace-separated token of a PGM header, skipping `#` comments.
fn token(data: &[u8], at: &mut usize) -> Option<String> {
    loop {
        while data.get(*at).is_some_and(u8::is_ascii_whitespace) {
            *at += 1;
        }
        if data.get(*at) == Some(&b'#') {
            while data.get(*at).is_some_and(|&c| c != b'\n') {
                *at += 1;
            }
        } else {
            break;
        }
    }
    let start = *at;
    while data.get(*at).is_some_and(|c| !c.is_ascii_whitespace()) {
        *at += 1;
    }
    let t = data.get(start..*at)?;
    (!t.is_empty()).then(|| String::from_utf8_lossy(t).into_owned())
}

/// Width, height and pixels of a binary 8-bit PGM.
fn parse_pgm(data: &[u8]) -> Option<(usize, usize, &[u8])> {
    let mut at = 0;
    if token(data, &mut at)? != "P5" {
        return None;
    }
    let width: usize = token(data, &mut at)?.parse().ok()?;
    let height: usize = token(data, &mut at)?.parse().ok()?;
    if token(data, &mut at)?.parse::<usize>().ok()? != 255 {
        return None;
    }
    // One whitespace byte ends the header.
    let pixels = data.get(at + 1..)?;
    let n = width.checked_mul(height)?;
    Some((width, height, pixels.get(..n)?))
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let Some(path) = args.get(1) else {
        eprintln!("usage: read_word <file.pgm> [bip39|slip39]");
        std::process::exit(2);
    };
    let slip39 = slip39_words();
    let words: &[&str] = match args.get(2).map(String::as_str) {
        None | Some("bip39") => bip39_words(),
        Some("slip39") => &slip39,
        Some(_) => {
            eprintln!("the list is bip39 or slip39");
            std::process::exit(2);
        }
    };
    let data = match std::fs::read(path) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("{path}: {e}");
            std::process::exit(1);
        }
    };
    let Some((width, height, gray)) = parse_pgm(&data) else {
        eprintln!("{path}: not a binary 8-bit PGM");
        std::process::exit(1);
    };
    match read_crop(width, height, gray, words) {
        Ok(Reading::Clear(c)) => println!("clear {}", c.word()),
        Ok(Reading::Unclear(list)) => {
            let names: Vec<&str> = list.iter().map(|c| c.word()).collect();
            println!("unclear {}", names.join(" "));
        }
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    }
}
