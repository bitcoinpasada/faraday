//! The Seed XOR vectors in `tools/vectors/xor/`, read whole. The files
//! were built from Seed XOR's definition, not taken from a published
//! source (`tools/vectors/xor/README.md`), so no single answer is trusted:
//! each part's words are decoded by rust-bitcoin's `bip39`, which checks
//! their checksum, and must be the file's hex; the parts XORed by a plain
//! byte fold must be the seed's hex and the seed's words; and `SeedXor`
//! must give the same seed.

use bip39::{Language, Mnemonic};
use faraday_entropy::SeedXor;

const TWELVE: &str = include_str!("../../../tools/vectors/xor/12-words-2-parts.txt");
const TWENTY_FOUR: &str = include_str!("../../../tools/vectors/xor/24-words-3-parts.txt");

/// One vector file: each part's entropy and words, then the seed's.
struct Vector {
    parts: Vec<(Vec<u8>, String)>,
    seed: (Vec<u8>, String),
}

fn hex(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).expect("hex"))
        .collect()
}

fn read(file: &str) -> Vector {
    let value = |key: &str| -> Vec<&str> {
        file.lines()
            .filter_map(|l| l.strip_prefix(key))
            .map(str::trim)
            .collect()
    };
    let mut parts = Vec::new();
    for n in 1.. {
        let entropy = value(&format!("part {n} entropy:"));
        let words = value(&format!("part {n} words:"));
        let (Some(e), Some(w)) = (entropy.first(), words.first()) else {
            break;
        };
        parts.push((hex(e), (*w).to_string()));
    }
    let seed = (
        hex(value("seed entropy:")[0]),
        value("seed words:")[0].to_string(),
    );
    Vector { parts, seed }
}

/// What `bip39` decodes the words to, its checksum checked.
fn decoded(words: &str) -> Vec<u8> {
    let m = Mnemonic::parse_in_normalized(Language::English, words).expect("valid BIP-39 words");
    let (bytes, len) = m.to_entropy_array();
    bytes[..len].to_vec()
}

fn check(file: &str, words: usize, count: usize) {
    let v = read(file);
    assert_eq!(v.parts.len(), count, "the file lists {count} parts");
    for (i, (entropy, w)) in v.parts.iter().enumerate() {
        assert_eq!(
            w.split(' ').count(),
            words,
            "part {} has {words} words",
            i + 1
        );
        assert_eq!(
            &decoded(w),
            entropy,
            "part {}'s words are its entropy",
            i + 1
        );
    }
    assert_eq!(
        decoded(&v.seed.1),
        v.seed.0,
        "the seed's words are its entropy"
    );

    let mut folded = vec![0u8; v.seed.0.len()];
    for (entropy, _) in &v.parts {
        for (a, b) in folded.iter_mut().zip(entropy) {
            *a ^= b;
        }
    }
    assert_eq!(folded, v.seed.0, "the parts XOR to the seed");

    let mut xor = SeedXor::new();
    for (entropy, _) in &v.parts {
        xor.push(entropy).expect("a part of the seed's length");
    }
    assert_eq!(
        xor.entropy().expect("enough parts").as_bytes(),
        v.seed.0.as_slice(),
        "SeedXor combines the parts into the seed"
    );
}

#[test]
fn twelve_words_in_two_parts_combine_into_the_seed() {
    check(TWELVE, 12, 2);
}

#[test]
fn twenty_four_words_in_three_parts_combine_into_the_seed() {
    check(TWENTY_FOUR, 24, 3);
}
