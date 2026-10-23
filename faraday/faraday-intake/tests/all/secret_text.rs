//! What a line of scanned text is when it is a secret: the BIP-39 vectors'
//! words and extended private keys against public keys and addresses.

use crate::common::{bip39_english, interop};
use faraday_intake::{TextSecret, secret_text};

#[test]
fn every_english_bip39_vector_phrase_is_a_secret_of_words() {
    let c = crate::common::compute();
    let rows = bip39_english();
    assert!(!rows.is_empty());
    for [_, words, _, _] in rows {
        assert_eq!(secret_text(&c, &words), Some(TextSecret::Words), "{words}");
        assert_eq!(
            secret_text(&c, &format!("  {words}\n")),
            Some(TextSecret::Words)
        );
    }
}

#[test]
fn every_bip39_vector_master_key_is_a_secret_extended_private_key() {
    let c = crate::common::compute();
    for [_, _, _, xprv] in bip39_english() {
        assert!(xprv.starts_with("xprv"));
        assert_eq!(secret_text(&c, &xprv), Some(TextSecret::ExtendedPrivateKey));
        // With the origin a descriptor writes before it.
        assert_eq!(
            secret_text(&c, &format!("[c55257c3/0h]{xprv}")),
            Some(TextSecret::ExtendedPrivateKey)
        );
    }
}

#[test]
fn a_public_key_or_an_address_is_not_a_secret() {
    let c = crate::common::compute();
    let xpub = "xpub6CqWTnie1ut9ZDD9xDeCn1VXk83VdAPSm8ZPfPNbb8w5z1e7jyy8zuX721uKj8u4GNxXqAevgEZjciUansnyz6ZhnSKyQWZwx2dpAxuCuDe";
    for text in [
        xpub,
        &format!("[086ee178/84h/0h/0h]{xpub}"),
        "bc1q5y4r767v5fzx74ez4nw36hjqrhr4ayeyut5px6",
        "3HJiAohE25FFBLPLZGVDwv7ZbSXVsSiZH7",
        "",
    ] {
        assert_eq!(secret_text(&c, text), None, "{text}");
    }
    // A descriptor with public keys only.
    let wallet = interop("bluewallet/coldcardmk4-descriptor.txt");
    assert_eq!(secret_text(&c, &wallet), None);
}

#[test]
fn twelve_words_that_are_not_a_valid_seed_are_not_a_secret() {
    let c = crate::common::compute();
    // The last word of the all-zero vector replaced: the checksum fails.
    let rows = bip39_english();
    let words = &rows[0][1];
    let bad = words.replace("about", "abandon");
    assert_eq!(secret_text(&c, &bad), None);
    let eleven = words.rsplit_once(' ').unwrap().0;
    assert_eq!(secret_text(&c, eleven), None);
}
