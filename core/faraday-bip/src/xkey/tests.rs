use alloc::string::String;
use alloc::vec::Vec;

use bitcoin::base58;

use super::{Error, decode_check, payload};

// BIP-32 test vector 1, chain m.
const XPUB: &str = "xpub661MyMwAqRbcFtXgS5sYJABqqG9YLmC4Q1Rdap9gSE8NqtwybGhePY2gZ29ESFjqJoCu1Rupje8YtGqsefD265TMg7usUDFdp6W1EGMcet8";
const XPRV: &str = "xprv9s21ZrQH143K3QTDL4LXw2F7HEK3wJUD2nW2nRk4stbPy6cq3jPPqjiChkVvvNKmPGJxWUtg6LnF5kejMRNNU3TGtRBeJgk33yuGBxrMPHi";

fn same_as_bitcoin(text: &str) {
    let ours = decode_check(text);
    let theirs = base58::decode_check(text).ok();
    assert_eq!(
        ours.as_deref().map(Vec::as_slice),
        theirs.as_deref(),
        "{text}"
    );
}

#[test]
fn the_published_keys_decode_as_the_bitcoin_crate_does() {
    for text in [XPUB, XPRV] {
        assert_eq!(
            payload(text).unwrap().as_slice(),
            base58::decode_check(text).unwrap()
        );
    }
}

#[test]
fn a_bad_checksum_a_bad_character_and_a_wrong_length_are_refused() {
    let mut bad = String::from(XPRV);
    bad.pop();
    bad.push('j');
    assert_eq!(payload(&bad).err(), Some(Error::Base58));
    let mut sum = String::from(XPUB);
    sum.pop();
    sum.push('s');
    assert_eq!(payload(&sum).err(), Some(Error::Base58));
    for text in ["", "1", "111", "0OIl", "xpub 661", "é"] {
        assert_eq!(decode_check(text), None, "{text}");
        same_as_bitcoin(text);
    }
    let short = base58::encode_check(&[7u8; 20]);
    assert_eq!(payload(&short).err(), Some(Error::Length(20)));
}

#[test]
fn random_texts_decode_or_fail_as_the_bitcoin_crate_does() {
    let mut state = 0x9e37_79b9_7f4a_7c15u64;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    for round in 0..3000usize {
        let len = round % 90;
        let mut data: Vec<u8> = (0..len).map(|_| next() as u8).collect();
        // Leading zero bytes are leading ones in the text.
        let zeros = (next() % 4) as usize;
        for b in data.iter_mut().take(zeros) {
            *b = 0;
        }
        let text = base58::encode_check(&data);
        same_as_bitcoin(&text);
        assert_eq!(
            decode_check(&text).as_deref().map(Vec::as_slice),
            Some(&data[..])
        );
        // The same text with one character changed.
        let mut chars: Vec<char> = text.chars().collect();
        let at = (next() as usize) % chars.len();
        let alphabet = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz0Ol";
        let pick = alphabet
            .chars()
            .nth((next() as usize) % alphabet.len())
            .unwrap();
        chars[at] = pick;
        same_as_bitcoin(&chars.iter().collect::<String>());
        // Arbitrary text from the alphabet.
        let free: String = (0..(next() % 120) as usize)
            .map(|_| alphabet.chars().nth((next() as usize) % 58).unwrap())
            .collect();
        same_as_bitcoin(&free);
    }
}
