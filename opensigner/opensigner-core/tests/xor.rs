//! Seed XOR against `tools/vectors/xor/` (`docs/PLANNING.md` §8.2
//! item 14): the parts XOR back to the seed, a part of the wrong length
//! is refused, and a key combined through the Create wizard is the key
//! the parts came from.
//!
//! The vectors were constructed rather than taken from Coldcard's
//! published examples, which this build box has no copy of, so nothing
//! here trusts one computation. Each mnemonic's entropy is recovered by
//! packing its 11-bit word indices in this file, its BIP-39 checksum is
//! checked here, and the XOR is a plain byte fold; only then is the
//! answer compared with the file's hex and with `osk_entropy::SeedXor`.
//! See `tools/vectors/xor/README.md`.

mod common;

use common::{Harness, PANEL, PHONE};
use opensigner_core::{ScreenKind, ids};
use osk_bip::bip39::{Language, Mnemonic};
use osk_bip::keys::{MasterKey, Network};
use osk_entropy::{Error, SeedXor};
use sha2::{Digest, Sha256};

const V12: &str = include_str!("../../../tools/vectors/xor/12-words-2-parts.txt");
const V24: &str = include_str!("../../../tools/vectors/xor/24-words-3-parts.txt");

/// One vector: the parts and the seed they XOR to, each as the hex the
/// file states and the words it states.
struct Vector {
    parts: Vec<(Vec<u8>, String)>,
    seed: (Vec<u8>, String),
}

fn unhex(s: &str) -> Vec<u8> {
    (0..s.len() / 2)
        .map(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16).expect("hex"))
        .collect()
}

fn read(text: &str) -> Vector {
    let mut parts: Vec<(Vec<u8>, String)> = Vec::new();
    let mut seed = (Vec::new(), String::new());
    for line in text.lines() {
        let Some((head, value)) = line.split_once(": ") else {
            continue;
        };
        let mut words = head.split_whitespace();
        match (words.next(), words.next_back()) {
            (Some("part"), Some(field)) => {
                let i: usize = words
                    .next()
                    .expect("a part number")
                    .parse()
                    .expect("number");
                if parts.len() < i {
                    parts.resize(i, (Vec::new(), String::new()));
                }
                if field == "entropy" {
                    parts[i - 1].0 = unhex(value);
                } else {
                    parts[i - 1].1 = value.to_string();
                }
            }
            (Some("seed"), Some("entropy")) => seed.0 = unhex(value),
            (Some("seed"), Some("words")) => seed.1 = value.to_string(),
            _ => {}
        }
    }
    assert!(parts.len() >= 2, "a vector has at least two parts");
    Vector { parts, seed }
}

/// The entropy behind a mnemonic, worked out here rather than asked for:
/// the words' 11-bit indices packed in order, split into the entropy and
/// the checksum, and the checksum verified against SHA-256 of the
/// entropy. Panics on a mnemonic that does not check out.
fn entropy_of(words: &str) -> Vec<u8> {
    let indices: Vec<u16> = words
        .split_whitespace()
        .map(|w| {
            Language::English
                .index_of(w)
                .unwrap_or_else(|| panic!("{w} is not an English BIP-39 word"))
        })
        .collect();
    let bits: Vec<bool> = indices
        .iter()
        .flat_map(|i| (0..11).rev().map(move |b| i >> b & 1 == 1))
        .collect();
    let entropy_bits = bits.len() * 32 / 33;
    assert_eq!(entropy_bits % 8, 0, "{} words", indices.len());
    let mut bytes = vec![0u8; entropy_bits / 8];
    for (i, bit) in bits[..entropy_bits].iter().enumerate() {
        if *bit {
            bytes[i / 8] |= 1 << (7 - i % 8);
        }
    }
    let digest = Sha256::digest(&bytes);
    for (i, bit) in bits[entropy_bits..].iter().enumerate() {
        assert_eq!(
            *bit,
            digest[i / 8] >> (7 - i % 8) & 1 == 1,
            "checksum bit {i} of {words}"
        );
    }
    bytes
}

/// The XOR of every slice, folded byte by byte here.
fn fold(parts: &[Vec<u8>]) -> Vec<u8> {
    let mut out = vec![0u8; parts[0].len()];
    for part in parts {
        assert_eq!(part.len(), out.len(), "parts are of one length");
        for (o, b) in out.iter_mut().zip(part) {
            *o ^= b;
        }
    }
    out
}

fn fingerprint(entropy: &[u8]) -> String {
    let m = Mnemonic::from_entropy(Language::English, entropy).expect("a mnemonic");
    let seed = m.to_seed(b"").expect("a seed");
    let hex = MasterKey::from_seed(&seed, Network::Mainnet)
        .fingerprint()
        .to_hex();
    String::from_utf8(hex.to_vec()).expect("hex")
}

/// Each vector's parts XOR back to its seed, by this file's arithmetic
/// and by `SeedXor`, and the words the file states are the words that
/// entropy encodes.
#[test]
fn the_vectors_combine_to_their_seed() {
    for text in [V12, V24] {
        let v = read(text);
        let mut entropies = Vec::new();
        for (hex, words) in &v.parts {
            let mine = entropy_of(words);
            assert_eq!(&mine, hex, "{words}");
            entropies.push(mine);
        }
        let seed = entropy_of(&v.seed.1);
        assert_eq!(seed, v.seed.0, "{}", v.seed.1);
        assert_eq!(fold(&entropies), seed, "the parts fold to the seed");

        let mut xor = SeedXor::new();
        for e in &entropies {
            xor.push(e).expect("a part");
        }
        assert_eq!(xor.entropy().expect("combined").as_bytes(), &seed[..]);
    }
}

/// Splitting a seed and combining the parts again gives the seed back,
/// for 12 words and for 24: the last part is what the others and the
/// seed XOR to, which is how the device makes a split.
#[test]
fn a_split_combines_back_to_the_seed() {
    for text in [V12, V24] {
        let v = read(text);
        let seed = v.seed.0.clone();
        let randoms: Vec<Vec<u8>> = v.parts[..v.parts.len() - 1]
            .iter()
            .map(|(e, _)| e.clone())
            .collect();
        let mut split = SeedXor::new();
        for r in &randoms {
            split.push(r).expect("a random part");
        }
        split.push(&seed).expect("the seed");
        let last = split.entropy().expect("the last part");
        assert_eq!(last.as_bytes(), &v.parts[v.parts.len() - 1].0[..]);

        let mut back = SeedXor::new();
        for r in &randoms {
            back.push(r).expect("a part");
        }
        back.push(last.as_bytes()).expect("the last part");
        assert_eq!(back.entropy().expect("combined").as_bytes(), &seed[..]);
    }
}

/// A part of another length is refused, and so is one part on its own.
#[test]
fn a_part_of_the_wrong_length_is_refused() {
    let v24 = read(V24);
    let v12 = read(V12);
    let mut xor = SeedXor::new();
    xor.push(&v24.parts[0].0).expect("a 32-byte part");
    assert_eq!(xor.push(&v12.parts[0].0), Err(Error::BadLength));
    assert_eq!(xor.push(&[0u8; 7]), Err(Error::BadLength));
    assert_eq!(
        xor.entropy().err(),
        Some(Error::TooFew { have: 1, need: 2 }),
        "one part is the seed itself"
    );
}

/// The English words of an entropy, as one line.
fn words_of(entropy: &[u8]) -> String {
    let m = Mnemonic::from_entropy(Language::English, entropy).expect("a mnemonic");
    let words: Vec<&str> = m
        .indices()
        .iter()
        .map(|&i| Language::English.word(i))
        .collect();
    words.join(" ")
}

/// A 15-word key splits into parts of twenty bytes, and typing those two
/// parts through Load › Seed XOR parts adds the key they came from.
#[test]
fn a_fifteen_word_key_splits_into_twenty_byte_parts() {
    let seed: Vec<u8> = (0..20u8)
        .map(|i| i.wrapping_mul(37).wrapping_add(11))
        .collect();
    let first: Vec<u8> = (0..20u8)
        .map(|i| i.wrapping_mul(97).wrapping_add(7))
        .collect();

    // The split the device makes: the random part and the seed pushed
    // in, the last part read out.
    let mut split = SeedXor::new();
    split.push(&first).expect("a random part");
    split.push(&seed).expect("the seed");
    let last = split.entropy().expect("the last part");
    let second = last.as_bytes().to_vec();
    assert_eq!(second.len(), 20);
    assert_eq!(fold(&[first.clone(), second.clone()]), seed);

    let parts = [words_of(&first), words_of(&second)];
    for (words, entropy) in parts.iter().zip([&first, &second]) {
        assert_eq!(words.split_whitespace().count(), 15);
        assert_eq!(&entropy_of(words), entropy);
    }

    let mut h = Harness::new(PHONE);
    h.open_load();
    h.choose(ids::LOAD_SOURCE_XOR, ids::LOAD_SOURCE_CONTINUE);
    // Row 2 of "How many words?" is 15.
    h.choose(
        ids::at(ids::CREATE_COUNT_BASE, 2),
        ids::CREATE_COUNT_CONTINUE,
    );
    h.choose(ids::at(ids::CREATE_LANG_BASE, 0), ids::CREATE_LANG_CONTINUE);
    for (i, words) in parts.iter().enumerate() {
        assert_eq!(h.app.screen(), ScreenKind::Scan);
        h.tap(ids::SCAN_TYPE);
        // This part is typed with Enter: §16.118 rule 3 keeps a hardware
        // keyboard a way in, so one test takes its words that way while
        // the others tap the candidate a finger would.
        for w in words.split_whitespace() {
            h.type_text(w);
            h.key(osk_shell_api::Key::Enter);
        }
        let texts = h.app.texts();
        let print = fingerprint(if i == 0 { &first } else { &second });
        assert!(
            texts.contains(&print),
            "part {} is {print}: {texts:?}",
            i + 1
        );
        if i == 0 {
            h.tap(ids::XOR_ADD_ANOTHER);
        } else {
            h.tap(ids::XOR_COMBINE);
        }
    }
    h.choose(ids::LOAD_SKIP, ids::LOAD_PASS_CONTINUE);
    h.add_key();
    if h.app.screen() == ScreenKind::Keep {
        h.tap(ids::BACK);
    }
    let added: Vec<String> = h
        .app
        .fingerprints()
        .iter()
        .map(|f| String::from_utf8(f.to_hex().to_vec()).expect("hex"))
        .collect();
    assert_eq!(added, vec![fingerprint(&seed)]);
}

/// Add › Load a key › "Seed XOR parts": combining is a load, the parts
/// are typed as words with a tap on each candidate, each part lands with
/// its own fingerprint, and the key that is added is the key the parts
/// came from — on the phone, where one tap takes a word, and on the
/// panel, where the first tap selects and the second accepts.
#[test]
fn typing_the_parts_adds_the_key_they_combine_to() {
    let v = read(V12);
    for display in [PHONE, PANEL] {
        let mut h = Harness::new(display);
        h.open_load();
        // Source, word count, wordlist.
        h.choose(ids::LOAD_SOURCE_XOR, ids::LOAD_SOURCE_CONTINUE);
        h.choose(
            ids::at(ids::CREATE_COUNT_BASE, 0),
            ids::CREATE_COUNT_CONTINUE,
        );
        h.choose(ids::at(ids::CREATE_LANG_BASE, 0), ids::CREATE_LANG_CONTINUE);
        for (i, (entropy, words)) in v.parts.iter().enumerate() {
            // Every part opens on the scanner; "Type" is the word entry.
            assert_eq!(h.app.screen(), ScreenKind::Scan);
            h.tap(ids::SCAN_TYPE);
            // Each word is taken by a tap on its candidate, as a finger
            // takes it.
            for w in words.split_whitespace() {
                h.type_word(w);
            }
            let texts = h.app.texts();
            let print = fingerprint(entropy);
            assert!(
                texts.contains(&print),
                "part {} is {print}: {texts:?}",
                i + 1
            );
            if i + 1 < v.parts.len() {
                h.tap(ids::XOR_ADD_ANOTHER);
            } else {
                h.tap(ids::XOR_COMBINE);
            }
        }
        // The combined key finishes like any other: no passphrase, then
        // the hold that adds it.
        h.choose(ids::LOAD_SKIP, ids::LOAD_PASS_CONTINUE);
        h.add_key();
        if h.app.screen() == opensigner_core::ScreenKind::Keep {
            h.tap(ids::BACK);
        }
        let added: Vec<String> = h
            .app
            .fingerprints()
            .iter()
            .map(|f| String::from_utf8(f.to_hex().to_vec()).expect("hex"))
            .collect();
        assert_eq!(added, vec![fingerprint(&v.seed.0)]);
    }
}
