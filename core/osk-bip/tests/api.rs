//! Behavioural tests of the `bip39` API: final-word candidates, checksum
//! bits, and every error case.

mod common;

use common::XorShift;
use osk_bip::bip39::{Error, Language, MAX_PASSPHRASE_BYTES, Mnemonic, last_word_candidates};

const EN: Language = Language::English;

#[test]
fn last_word_candidates_count_and_validity() {
    let mut rng = XorShift::new(7);
    for (entropy_len, expected) in [(16, 128), (20, 64), (24, 32), (28, 16), (32, 8)] {
        let mut entropy = vec![0u8; entropy_len];
        rng.fill(&mut entropy);
        let m = Mnemonic::from_entropy(EN, &entropy).unwrap();
        let n = m.word_count();
        let first = &m.indices()[..n - 1];

        let iter = last_word_candidates(first).unwrap();
        assert_eq!(iter.len(), expected);
        let candidates: Vec<u16> = iter.collect();
        assert_eq!(candidates.len(), expected);
        assert!(
            candidates.windows(2).all(|w| w[0] < w[1]),
            "sorted and unique"
        );
        assert!(candidates.contains(&m.indices()[n - 1]));

        let mut full = first.to_vec();
        full.push(0);
        let mut valid = 0;
        for last in 0..2048u16 {
            full[n - 1] = last;
            let ok = Mnemonic::from_indices(EN, &full).is_ok();
            assert_eq!(ok, candidates.contains(&last), "{n} words, last={last}");
            valid += usize::from(ok);
        }
        assert_eq!(valid, expected);
    }
}

#[test]
fn last_word_candidates_errors() {
    assert_eq!(
        last_word_candidates(&[0; 12]).err(),
        Some(Error::InvalidWordCount)
    );
    assert_eq!(
        last_word_candidates(&[0; 10]).err(),
        Some(Error::InvalidWordCount)
    );
    assert_eq!(
        last_word_candidates(&[]).err(),
        Some(Error::InvalidWordCount)
    );
    let mut bad = [0u16; 11];
    bad[3] = 2048;
    assert_eq!(
        last_word_candidates(&bad).err(),
        Some(Error::UnknownWord { position: 3 })
    );
}

#[test]
fn checksum_bits() {
    let m = Mnemonic::from_entropy(EN, &[0u8; 16]).unwrap();
    // SHA-256 of 16 zero bytes starts with 0x37; the top 4 bits are 0x3.
    assert_eq!(m.checksum_bits(), (0x3, 4));
    assert_eq!(EN.word(m.indices()[11]), "about");

    let m = Mnemonic::from_entropy(EN, &[0xffu8; 32]).unwrap();
    let (value, bits) = m.checksum_bits();
    assert_eq!(bits, 8);
    assert_eq!(value, osk_crypto::sha256(&[0xffu8; 32])[0]);
    assert_eq!(m.indices()[23] & 0xff, u16::from(value));
}

#[test]
fn entropy_round_trip_all_lengths() {
    let mut rng = XorShift::new(99);
    for len in [16, 20, 24, 28, 32] {
        for _ in 0..20 {
            let mut entropy = vec![0u8; len];
            rng.fill(&mut entropy);
            let m = Mnemonic::from_entropy(EN, &entropy).unwrap();
            assert_eq!(m.word_count(), len * 3 / 4);
            assert_eq!(m.entropy().expose().as_bytes(), &entropy[..]);
            let again = Mnemonic::from_indices(EN, m.indices()).unwrap();
            assert_eq!(again.entropy().expose().as_bytes(), &entropy[..]);
            assert!(m.entropy() == again.entropy());
        }
    }
}

#[test]
fn from_entropy_rejects_bad_lengths() {
    for len in [0, 1, 15, 17, 18, 31, 33, 64] {
        assert_eq!(
            Mnemonic::from_entropy(EN, &vec![0u8; len]).err(),
            Some(Error::InvalidEntropyLength)
        );
    }
}

#[test]
fn from_indices_rejects_bad_counts_and_words() {
    for n in [0, 1, 11, 13, 14, 16, 23, 25, 48] {
        assert_eq!(
            Mnemonic::from_indices(EN, &vec![0u16; n]).err(),
            Some(Error::InvalidWordCount),
            "{n}"
        );
    }
    let mut words = [0u16; 12];
    words[7] = 2048;
    assert_eq!(
        Mnemonic::from_indices(EN, &words).err(),
        Some(Error::UnknownWord { position: 7 })
    );
    words[7] = 0xffff;
    assert_eq!(
        Mnemonic::from_indices(EN, &words).err(),
        Some(Error::UnknownWord { position: 7 })
    );
}

#[test]
fn bad_checksum_is_detected() {
    let good = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
    assert!(Mnemonic::parse(EN, good).is_ok());
    let bad = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon";
    assert_eq!(Mnemonic::parse(EN, bad).err(), Some(Error::BadChecksum));

    // Flipping any single word of a valid mnemonic to a neighbour must be
    // caught unless the flip lands on another valid checksum.
    let m = Mnemonic::from_entropy(EN, &[0x42u8; 32]).unwrap();
    let mut words = m.indices().to_vec();
    words[0] ^= 1;
    assert_eq!(
        Mnemonic::from_indices(EN, &words).err(),
        Some(Error::BadChecksum)
    );
}

#[test]
fn parse_locates_unknown_words() {
    let typo = "abandon abandon abandon abandon abandom abandon abandon abandon abandon abandon abandon about";
    assert_eq!(
        Mnemonic::parse(EN, typo).err(),
        Some(Error::UnknownWord { position: 4 })
    );
    // Mixed and repeated whitespace, including the ideographic space, is fine.
    let spaced = "  abandon\tabandon abandon\u{3000}abandon abandon abandon\nabandon abandon abandon abandon abandon   about ";
    assert!(Mnemonic::parse(EN, spaced).is_ok());
    assert_eq!(Mnemonic::parse(EN, "").err(), Some(Error::InvalidWordCount));
    assert_eq!(
        Mnemonic::parse(EN, "abandon").err(),
        Some(Error::InvalidWordCount)
    );
    let too_many = std::iter::repeat_n("abandon", 25)
        .collect::<Vec<_>>()
        .join(" ");
    assert_eq!(
        Mnemonic::parse(EN, &too_many).err(),
        Some(Error::InvalidWordCount)
    );
    // A word from another language is unknown in this one.
    let mixed = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon ábaco";
    assert_eq!(
        Mnemonic::parse(EN, mixed).err(),
        Some(Error::UnknownWord { position: 11 })
    );
}

#[test]
fn passphrase_rules() {
    let m = Mnemonic::from_entropy(EN, &[0u8; 16]).unwrap();
    assert!(m.to_seed(b"").is_ok());
    assert!(
        m.to_seed(b" ~!@#$%^&*() printable ASCII 0x20..=0x7E")
            .is_ok()
    );
    assert_eq!(
        m.to_seed("caf\u{e9}".as_bytes()).err(),
        Some(Error::PassphraseNotAscii)
    );
    assert_eq!(
        m.to_seed(b"tab\there").err(),
        Some(Error::PassphraseNotAscii)
    );
    assert_eq!(m.to_seed(b"nl\n").err(), Some(Error::PassphraseNotAscii));
    assert_eq!(m.to_seed(&[0x7f]).err(), Some(Error::PassphraseNotAscii));
    assert!(m.to_seed(&[b'a'; MAX_PASSPHRASE_BYTES]).is_ok());
    assert_eq!(
        m.to_seed(&[b'a'; MAX_PASSPHRASE_BYTES + 1]).err(),
        Some(Error::PassphraseTooLong)
    );
    assert_eq!(
        m.to_seed_unchecked(&[0xe3; MAX_PASSPHRASE_BYTES + 1]).err(),
        Some(Error::PassphraseTooLong)
    );

    // Different passphrases give different seeds; the same gives the same.
    let a = m.to_seed(b"a").unwrap();
    let b = m.to_seed(b"b").unwrap();
    assert!(a != b);
    assert!(a == m.to_seed(b"a").unwrap());
}

#[test]
fn zeroize_on_drop_bounds() {
    fn assert_zod<T: osk_crypto::ZeroizeOnDrop>() {}
    assert_zod::<Mnemonic>();
    assert_zod::<osk_bip::bip39::LastWordCandidates>();
    assert_zod::<osk_crypto::Secret<osk_bip::bip39::Entropy>>();
}
