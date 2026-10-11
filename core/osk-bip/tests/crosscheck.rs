//! Cross-check against the `bip39` crate for every language and length,
//! using a seeded xorshift so the run is deterministic.
//!
//! Mnemonics are compared for every sample. Seeds cost 2048 PBKDF2 rounds
//! each in both implementations, so they are compared for the first
//! `SEED_SAMPLES` entropies of each (language, length) pair.

mod common;

use common::{XorShift, sentence};

use osk_bip::bip39::{Language, Mnemonic};

const SAMPLES: usize = 200;
const SEED_SAMPLES: usize = 8;
const ENTROPY_LENS: [usize; 5] = [16, 20, 24, 28, 32];

fn theirs(lang: Language) -> bip39::Language {
    match lang {
        Language::English => bip39::Language::English,
        Language::Japanese => bip39::Language::Japanese,
        Language::Korean => bip39::Language::Korean,
        Language::Spanish => bip39::Language::Spanish,
        Language::ChineseSimplified => bip39::Language::SimplifiedChinese,
        Language::ChineseTraditional => bip39::Language::TraditionalChinese,
        Language::French => bip39::Language::French,
        Language::Italian => bip39::Language::Italian,
        Language::Czech => bip39::Language::Czech,
        Language::Portuguese => bip39::Language::Portuguese,
    }
}

#[test]
fn wordlists_match_bip39_crate() {
    for lang in Language::ALL {
        let list = theirs(lang).word_list();
        for i in 0..2048u16 {
            assert_eq!(lang.word(i), list[usize::from(i)], "{} #{i}", lang.name());
            assert_eq!(lang.word_nfkd(i), list[usize::from(i)]);
        }
    }
}

#[test]
fn mnemonics_and_seeds_match_bip39_crate() {
    let mut rng = XorShift::new(0x5EED_0000_0000_0001);
    let mut checked = 0usize;
    let mut seeds = 0usize;
    for lang in Language::ALL {
        for len in ENTROPY_LENS {
            for sample in 0..SAMPLES {
                let mut entropy = vec![0u8; len];
                rng.fill(&mut entropy);

                let ours = Mnemonic::from_entropy(lang, &entropy).unwrap();
                let reference = bip39::Mnemonic::from_entropy_in(theirs(lang), &entropy).unwrap();

                let ref_indices: Vec<u16> = reference.word_indices().map(|i| i as u16).collect();
                assert_eq!(
                    ours.indices(),
                    &ref_indices[..],
                    "{} {len}B #{sample}",
                    lang.name()
                );
                let ref_words: Vec<&str> = reference.words().collect();
                let our_words: Vec<&str> = ours.indices().iter().map(|&i| lang.word(i)).collect();
                assert_eq!(our_words, ref_words);
                // Their Display always joins with ASCII space.
                assert_eq!(our_words.join(" "), reference.to_string());

                // Round trips through our parser and their normalized parser.
                let text = sentence(lang, ours.indices());
                assert_eq!(
                    Mnemonic::parse(lang, &text).unwrap().indices(),
                    ours.indices()
                );
                // Not compared with their `to_entropy_array`: it re-detects the
                // language from the words and panics when the sentence is valid
                // in both Chinese lists.
                assert_eq!(ours.entropy().expose().as_bytes(), &entropy[..]);
                assert_eq!(ours.checksum_bits().0, reference.checksum());

                if sample < SEED_SAMPLES {
                    for passphrase in ["", "abc"] {
                        let our_seed = ours.to_seed(passphrase.as_bytes()).unwrap();
                        let ref_seed = reference.to_seed_normalized(passphrase);
                        assert_eq!(
                            our_seed.expose(),
                            &ref_seed,
                            "{} {len}B #{sample} seed {passphrase:?}",
                            lang.name()
                        );
                    }
                    seeds += 1;
                }
                checked += 1;
            }
        }
    }
    assert_eq!(checked, Language::ALL.len() * ENTROPY_LENS.len() * SAMPLES);
    assert_eq!(
        seeds,
        Language::ALL.len() * ENTROPY_LENS.len() * SEED_SAMPLES
    );
}
