//! `faraday_pgp::read_public` (F22, F30): a stick file that looks like an
//! armoured OpenPGP block. Whatever it reads, a public result holds no
//! secret key packet (tag 5 or 7), and what `armor` writes for any bytes
//! reads back as the kind it was written as, or not at all.

#![no_main]

use faraday_pgp::{Armor, Hash, Key, armor, read_public};
use libfuzzer_sys::fuzz_target;

fn no_secret_packets(tags: &[u8]) {
    assert!(!tags.iter().any(|t| matches!(t, 5 | 7)), "{tags:?}");
}

fuzz_target!(|data: &[u8]| {
    // The bytes as a file.
    if let Some(block) = read_public(data) {
        no_secret_packets(&block.tags);
    }
    // The bytes as packets, armoured both ways: a read result is the kind
    // it was armoured as.
    // A key made from the bytes: its certificate reads back as a public
    // key and a signature by it as a signature, whatever the user ID.
    if data.len() >= 72 {
        let int = |at: usize| u32::from_be_bytes(data[at..at + 4].try_into().unwrap());
        let key = Key {
            primary_seed: data[..32].try_into().unwrap(),
            created: int(64),
            subkey_seed: data[32..64].try_into().unwrap(),
            subkey_created: int(68),
            user_ids: vec![String::from_utf8_lossy(&data[72..]).into_owned()],
            expiry: int(64) ^ int(68),
        };
        let cert = armor(Armor::PublicKey, &key.certificate(int(64)));
        let block = read_public(cert.as_bytes()).expect("a certificate reads");
        assert_eq!(block.kind, Armor::PublicKey);
        no_secret_packets(&block.tags);
        let sig = armor(Armor::Signature, &key.sign(data, int(68), Hash::Sha256));
        let block = read_public(sig.as_bytes()).expect("a signature reads");
        assert_eq!(block.kind, Armor::Signature);
    }
    let Some((&first, packets)) = data.split_first() else {
        return;
    };
    let kind = if first & 1 == 0 {
        Armor::PublicKey
    } else {
        Armor::Signature
    };
    let text = armor(kind, packets);
    if let Some(block) = read_public(text.as_bytes()) {
        assert_eq!(block.kind, kind);
        no_secret_packets(&block.tags);
    }
});
