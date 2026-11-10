//! Core Lightning's `hsm_secret` as a source of Lightning node key
//! (`docs/decisions/F228.md`).

use faraday_bip::bip39::Language;
use faraday_crypto::work::Compute;
use faraday_intake::secret_buf::SecretText;
use faraday_keys::hsm_node::{WordsRefusal, node_from_words};
use faraday_keys::hsm_secret::{MNEMONIC_MAX_LEN, Refusal, needs_passphrase, node_from_file};

#[expect(
    clippy::disallowed_methods,
    reason = "a mint: faraday-keys' tests, which cannot reach faraday-core's testkit"
)]
fn compute() -> Compute {
    Compute::new(|_| {})
}

fn hex(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).expect("hex"))
        .collect()
}

fn id_of(node: &faraday_keys::lightning::Node) -> String {
    faraday_crypto::hex::encode_string(node.public())
}

/// A plain secret of 0x01 bytes, as Core Lightning's own test of
/// `lightning-hsmtool encrypt` writes it.
const ONES_ID: &str = "026f61d7ee82f937f9697f4f3e44bfaaa25849cc4f526b3a57326130eba6346002";
/// That secret sealed by libsodium under the passphrase `test_password`,
/// as `lightning-hsmtool encrypt` seals it.
const ONES_SEALED: &str = "2c9ac8c7092b7c78712ceb9178be048d98c4a5871ae4eea739d8aa473e0fd750242028ee9acd86ecc6131d72cbc725c6efda4fb84ff06ba6c9b8eba2bfad6001a618eab8d79b30b911";

#[test]
fn lightning_a_plain_hsm_secret_gives_its_node_id() {
    let c = compute();
    let node = node_from_file(&c, &[1u8; 32], &SecretText::new()).expect("plain");
    assert_eq!(id_of(&node), ONES_ID);
}

#[test]
fn lightning_an_encrypted_hsm_secret_opens_under_its_passphrase() {
    let c = compute();
    let node =
        node_from_file(&c, &hex(ONES_SEALED), &SecretText::of("test_password")).expect("opens");
    assert_eq!(id_of(&node), ONES_ID);
}

#[test]
fn lightning_a_wrong_passphrase_or_a_changed_byte_is_one_refusal() {
    let c = compute();
    let sealed = hex(ONES_SEALED);
    let wrong = node_from_file(&c, &sealed, &SecretText::of("test_passwore"));
    assert_eq!(wrong.err(), Some(Refusal::Opening));
    for at in [0, 23, 24, 40, 56, 57, 72] {
        let mut changed = sealed.clone();
        changed[at] ^= 0x01;
        let got = node_from_file(&c, &changed, &SecretText::of("test_password"));
        assert_eq!(got.err(), Some(Refusal::Opening), "byte {at}");
    }
}

#[test]
fn lightning_a_file_shorter_than_32_bytes_or_too_long_is_refused() {
    let c = compute();
    let pass = SecretText::of("test_password");
    for len in [0, 1, 31, MNEMONIC_MAX_LEN + 1, 1000] {
        let got = node_from_file(&c, &vec![1u8; len], &pass);
        assert_eq!(got.err(), Some(Refusal::Length), "{len} bytes");
    }
}

const ABANDON: &str =
    "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
const RITUAL: &str = "ritual idle hat sunny universe pluck key alpha wing cake have wedding";
/// SHA-256 of the 64-byte BIP-39 seed of `RITUAL` under `test_passphrase`,
/// what Core Lightning writes before the words.
const RITUAL_HASH: &str = "61ce9e277eb9894701400a422e43ba27c7d9d10b72bad52c16a411d2dbaf82f1";

fn mnemonic_file(hash: &[u8], text: &str) -> Vec<u8> {
    let mut file = hash.to_vec();
    file.extend_from_slice(text.as_bytes());
    file
}

/// The two node ids Core Lightning's test suite (`tests/test_wallet.py`,
/// `test_hsmtool_generatehsm_variants`) names for a node made from
/// these words, with and without a passphrase.
#[test]
fn lightning_a_mnemonic_hsm_secret_gives_the_node_id_core_lightning_has() {
    let c = compute();
    let none = mnemonic_file(&[0u8; 32], ABANDON);
    assert!(!needs_passphrase(&none));
    let node = node_from_file(&c, &none, &SecretText::new()).expect("no passphrase");
    assert_eq!(
        id_of(&node),
        "03653e90c1ce4660fd8505dd6d643356e93cfe202af109d382787639dd5890e87d"
    );

    let with = mnemonic_file(&hex(RITUAL_HASH), RITUAL);
    assert!(needs_passphrase(&with));
    let node = node_from_file(&c, &with, &SecretText::of("test_passphrase")).expect("passphrase");
    assert_eq!(
        id_of(&node),
        "039020371fb803cd4ce1e9a909b502d7b0a9e0f10cccc35c3e9be959c52d3ba6bd"
    );
}

#[test]
fn lightning_a_mnemonic_hsm_secret_with_the_wrong_passphrase_is_refused() {
    let c = compute();
    let with = mnemonic_file(&hex(RITUAL_HASH), RITUAL);
    for wrong in ["", "test_passphrasf", "test_passphrase "] {
        let got = node_from_file(&c, &with, &SecretText::of(wrong));
        assert_eq!(got.err(), Some(Refusal::Passphrase), "{wrong:?}");
    }
    // Core Lightning refuses a passphrase for words that have none.
    let none = mnemonic_file(&[0u8; 32], ABANDON);
    let got = node_from_file(&c, &none, &SecretText::of("test_passphrase"));
    assert_eq!(got.err(), Some(Refusal::PassphraseNotNeeded));
}

#[test]
fn lightning_a_mnemonic_hsm_secret_with_a_passphrase_bip39_cannot_take_has_its_own_refusal() {
    let c = compute();
    let with = mnemonic_file(&hex(RITUAL_HASH), RITUAL);
    for bad in ["café", "tab\there", "bell\u{7}"] {
        let got = node_from_file(&c, &with, &SecretText::of(bad));
        assert_eq!(got.err(), Some(Refusal::PassphraseNotAscii), "{bad:?}");
    }
    let long = SecretText::of(&"a".repeat(257));
    let got = node_from_file(&c, &with, &long);
    assert_eq!(got.err(), Some(Refusal::PassphraseTooLong));
}

#[test]
fn lightning_a_mnemonic_hsm_secret_of_27_words_is_refused_as_words() {
    let c = compute();
    // Core Lightning (libwally) takes 27 words; Faraday's BIP-39 takes 12 to 24.
    let words = vec!["abandon"; 27].join(" ");
    let got = node_from_file(&c, &mnemonic_file(&[0u8; 32], &words), &SecretText::new());
    assert_eq!(got.err(), Some(Refusal::Words));
}

#[test]
fn lightning_a_mnemonic_hsm_secret_with_broken_words_is_refused() {
    let c = compute();
    let zero = [0u8; 32];
    let none = SecretText::new();
    let bad_checksum = ABANDON.replace("about", "abandon");
    let not_english = "ábaco ábaco ábaco ábaco ábaco ábaco ábaco ábaco ábaco ábaco ábaco abierto";
    let nul = ABANDON.replace("about", "abo\0ut");
    let cases = [
        ("bad checksum", mnemonic_file(&zero, &bad_checksum)),
        ("another language", mnemonic_file(&zero, not_english)),
        (
            "eleven words",
            mnemonic_file(&zero, &ABANDON[..ABANDON.len() - 6]),
        ),
        (
            "trailing newline",
            mnemonic_file(&zero, &format!("{ABANDON}\n")),
        ),
        (
            "trailing space",
            mnemonic_file(&zero, &format!("{ABANDON} ")),
        ),
        (
            "leading space",
            mnemonic_file(&zero, &format!(" {ABANDON}")),
        ),
        (
            "double space",
            mnemonic_file(&zero, &ABANDON.replacen(' ', "  ", 1)),
        ),
        ("upper case", mnemonic_file(&zero, &ABANDON.to_uppercase())),
        ("a NUL", mnemonic_file(&zero, &nul)),
        (
            "a NUL after the words",
            mnemonic_file(&zero, &format!("{ABANDON}\0")),
        ),
        ("not UTF-8", {
            let mut f = mnemonic_file(&zero, ABANDON);
            f.push(0xff);
            f
        }),
        ("one byte of words", mnemonic_file(&zero, "a")),
        ("too long", mnemonic_file(&zero, &"abandon ".repeat(40))),
    ];
    for (what, file) in cases {
        assert!(node_from_file(&c, &file, &none).is_err(), "{what}");
    }
}

/// Core Lightning's own test suite (`tests/test_wallet.py`) gives these
/// node ids for the words and passphrases.
#[test]
fn lightning_words_give_the_node_id_core_lightning_has_for_them() {
    let c = compute();
    let abandon = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
    let node = node_from_words(&c, Language::English, abandon, "").expect("words");
    assert_eq!(
        id_of(&node),
        "03653e90c1ce4660fd8505dd6d643356e93cfe202af109d382787639dd5890e87d"
    );
    let ritual = "ritual idle hat sunny universe pluck key alpha wing cake have wedding";
    let node = node_from_words(&c, Language::English, ritual, "test_passphrase").expect("words");
    assert_eq!(
        id_of(&node),
        "039020371fb803cd4ce1e9a909b502d7b0a9e0f10cccc35c3e9be959c52d3ba6bd"
    );
    assert!(node_from_words(&c, Language::English, "abandon abandon", "").is_err());
}

/// Core Lightning takes only English words, so words in another
/// language give no node.
#[test]
fn lightning_words_in_another_language_are_refused() {
    let c = compute();
    let spanish = "ábaco ábaco ábaco ábaco ábaco ábaco ábaco ábaco ábaco ábaco ábaco abierto";
    let got = node_from_words(&c, Language::Spanish, spanish, "");
    assert_eq!(got.err(), Some(WordsRefusal::NotEnglish));
    let japanese = "あいこくしん あいこくしん あいこくしん あいこくしん あいこくしん あいこくしん あいこくしん あいこくしん あいこくしん あいこくしん あいこくしん あおぞら";
    let got = node_from_words(&c, Language::Japanese, japanese, "");
    assert_eq!(got.err(), Some(WordsRefusal::NotEnglish));
}
