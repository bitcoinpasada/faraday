//! A key leaving the session by itself.

use faraday_bip::keys::Network;
use faraday_crypto::work::Compute;
use faraday_keys::session::Keys;

#[expect(
    clippy::disallowed_methods,
    reason = "a mint: faraday-keys' tests, which cannot reach faraday-core's testkit"
)]
fn compute() -> Compute {
    Compute::new(|_| {})
}

/// Two of BIP-39's published test vectors, neither a Faraday sample.
const WORDS: &str = "letter advice cage absurd amount doctor acoustic avoid letter advice \
                     cage above";
const OTHER_WORDS: &str =
    "legal winner thank year wave sausage worth useful legal winner thank yellow";

#[test]
fn remove_one_drops_only_that_key_and_its_answer_for_testing() {
    let c = compute();
    let mut keys = Keys::on(Network::Mainnet);
    let a = keys.add_words(&c, WORDS, "A", None).expect("adds");
    let b = keys.add_words(&c, OTHER_WORDS, "B", None).expect("adds");
    assert_eq!(keys.published_pending(), Some(a));
    keys.keep_published(a);
    keys.keep_published(b);
    assert_eq!(keys.published_pending(), None);
    assert!(keys.remove_one(a));
    let left: Vec<_> = keys.all().map(|k| k.fingerprint).collect();
    assert_eq!(left, [b]);
    assert!(!keys.remove_one(a), "no longer held");
    assert_eq!(keys.len(), 1);
    // Added again, it is asked about again; the other's answer stands.
    keys.add_words(&c, WORDS, "A", None).expect("adds");
    assert_eq!(keys.published_pending(), Some(a));
}
