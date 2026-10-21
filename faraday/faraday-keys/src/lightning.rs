//! Lightning node keys: ldk-node's from a BIP-39 seed, LND's from an
//! aezeed.

use faraday_bip::aezeed;
use faraday_bip::bip39::{Language, Mnemonic};
use faraday_bip::keys::{DerivedKey, Network};
use faraday_crypto::Secret;
use faraday_crypto::work::Compute;
use zeroize::{Zeroize, Zeroizing};

/// Words in an aezeed.
pub const AEZEED_WORDS: usize = aezeed::NUM_WORDS;

/// A node key.
pub struct Node {
    /// The node's public key, compressed: its node id.
    pub public: [u8; 33],
    /// Its private key.
    pub private: Zeroizing<[u8; 32]>,
    /// The day an aezeed says it was made.
    pub birthday: Option<(i32, u32, u32)>,
}

/// Costs an elliptic-curve multiplication (through `to_xpub`): the node
/// that `key` is, its public key worked out from the private one.
fn node_of(c: &Compute, key: &DerivedKey, birthday: Option<(i32, u32, u32)>) -> Node {
    let public = key.to_xpub(c).public_key.serialize();
    Node {
        public,
        private: Zeroizing::new(key.secret_key().secret_bytes()),
        birthday,
    }
}

/// Costs a parse of the words, a PBKDF2, key derivations and an
/// elliptic-curve multiplication. The node key ldk-node derives from a
/// BIP-39 seed: `words` in `language` with `passphrase`, on `network`.
pub fn node_from_words(
    c: &Compute,
    language: Language,
    words: &str,
    passphrase: &str,
    network: Network,
) -> Result<Node, String> {
    let m = Mnemonic::parse(c, language, words).map_err(|e| e.to_string())?;
    let seed = m
        .to_seed(c, passphrase.as_bytes())
        .map_err(|e| e.to_string())?;
    Ok(node_of(c, &aezeed::ldk_node_key(c, &seed, network), None))
}

/// Costs an scrypt run, key derivations and an elliptic-curve
/// multiplication. The node key an aezeed gives: `words` are indices into
/// the English list, `passphrase` is LND's (empty is its default). The
/// debug text of the aezeed's refusal when it does not read.
pub fn node_from_aezeed(
    c: &Compute,
    words: &[u16; AEZEED_WORDS],
    passphrase: &[u8],
    network: Network,
) -> Result<Node, String> {
    aezeed::decode(c, words, passphrase)
        .map_err(|e| format!("{e:?}"))
        .map(|seed| {
            let birthday = aezeed::birthday_date(seed.birthday);
            let mut entropy = Secret::new([0u8; aezeed::ENTROPY_SIZE]);
            entropy
                .expose_mut()
                .copy_from_slice(seed.entropy().expose());
            node_of(c, &aezeed::node_key(c, &entropy, network), Some(birthday))
        })
}

/// Wipes an aezeed's word indices once they are used.
pub fn wipe_words(words: &mut [u16; AEZEED_WORDS]) {
    words.zeroize();
}
