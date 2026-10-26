//! What New key hands out in Practice: a sample of the length chosen, for
//! every length it offers.

use crate::common::{PASSPHRASE, reference_fingerprint};
use faraday_bip::keys::Network;
use faraday_core::samples::repeated;
use faraday_core::testkit::{self, compute};
use faraday_keys::Keys;
use faraday_samples::SAMPLES;

const LENGTHS: [u8; 5] = [12, 15, 18, 21, 24];

fn of_length(n: u8) -> Vec<&'static str> {
    SAMPLES
        .handouts()
        .iter()
        .filter(|h| h.words == n)
        .map(|h| h.name)
        .collect()
}

#[test]
fn there_are_eleven_handouts_of_24_words_four_of_12_and_two_of_each_other_length() {
    let counts: Vec<usize> = LENGTHS.iter().map(|&n| of_length(n).len()).collect();
    assert_eq!(counts, [4, 2, 2, 2, 11]);
    assert_eq!(SAMPLES.handouts().len(), 21);
}

#[test]
fn each_handout_is_a_valid_phrase_and_the_24_word_ones_are_every_word_valid_that_often() {
    let lang = bip39::Language::English;
    for h in SAMPLES.handouts() {
        let phrase = repeated(h.name, h.words);
        bip39::Mnemonic::parse_in_normalized(lang, &phrase)
            .unwrap_or_else(|e| panic!("{} x{}: {e}", h.name, h.words));
        assert_eq!(h.plain.0, reference_fingerprint(lang, &phrase, ""));
        assert_eq!(
            h.with_passphrase.0,
            reference_fingerprint(lang, &phrase, PASSPHRASE)
        );
    }
    // Which words are valid 24 times, by trying all 2048.
    let valid: Vec<&str> = lang
        .word_list()
        .iter()
        .copied()
        .filter(|w| bip39::Mnemonic::parse_in_normalized(lang, &repeated(w, 24)).is_ok())
        .collect();
    let mut have = of_length(24);
    have.sort_unstable();
    assert_eq!(valid, have);
}

#[test]
fn the_handouts_are_those_the_cores_tests_use() {
    let want = testkit::demo_samples().handouts();
    let have = SAMPLES.handouts();
    assert_eq!(want.len(), have.len());
    for (w, h) in want.iter().zip(&have) {
        assert_eq!((w.name, w.words), (h.name, h.words));
        assert_eq!((w.plain, w.with_passphrase), (h.plain, h.with_passphrase));
    }
}

fn demo_keys() -> Keys {
    let mut keys = Keys::on(Network::Testnet);
    keys.set_demo(&SAMPLES.handouts(), &SAMPLES.public(), &SAMPLES.nodes());
    keys
}

#[test]
fn new_key_hands_out_the_next_sample_of_the_length_until_all_are_loaded() {
    let c = compute();
    for n in LENGTHS {
        let mut keys = demo_keys();
        for name in of_length(n) {
            let (next, fp) = keys.next_handout(n).expect("one is left");
            assert_eq!(next, name, "{n} words");
            let added = keys
                .add_words(&c, &repeated(next, n), "", None)
                .expect("loads");
            assert_eq!(added, fp);
        }
        assert!(keys.next_handout(n).is_none(), "{n} words, all loaded");
        // The other lengths are still there.
        for other in LENGTHS.into_iter().filter(|&o| o != n) {
            assert!(keys.next_handout(other).is_some(), "{other} words");
        }
    }
}

#[test]
fn a_sample_loaded_with_passphrase_counts_as_loaded_for_new_key() {
    let c = compute();
    let mut keys = demo_keys();
    let first = of_length(12)[0];
    keys.add_words_with(&c, &repeated(first, 12), PASSPHRASE, "", None)
        .expect("loads");
    assert_eq!(keys.next_handout(12).map(|h| h.0), Some(of_length(12)[1]));
}

#[test]
fn outside_demo_new_key_hands_out_nothing() {
    let keys = Keys::on(Network::Testnet);
    for n in LENGTHS {
        assert!(keys.next_handout(n).is_none());
    }
    assert!(keys.is_public(faraday_bip::keys::Fingerprint([1, 2, 3, 4])));
}
