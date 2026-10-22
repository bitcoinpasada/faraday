//! Lightning node keys: ldk-node's from a BIP-39 seed, LND's from an
//! aezeed.

use faraday_bip::aezeed;
use faraday_bip::bip39::{Language, Mnemonic};
use faraday_bip::keys::{DerivedKey, Network};
use faraday_crypto::Secret;
use faraday_crypto::work::Compute;
use zeroize::{Zeroize, Zeroizing};

use crate::{Part, Reveal};

/// Words in an aezeed.
pub const AEZEED_WORDS: usize = aezeed::NUM_WORDS;

/// A node key. It holds the private key, so it is neither `Clone` nor
/// `Debug` and gives out its public key and birthday; the private key
/// leaves only through [`Node::private_hex`], for a reason that names it.
pub struct Node {
    /// The node's public key, compressed: its node id.
    public: [u8; 33],
    /// Its private key.
    private: Zeroizing<[u8; 32]>,
    /// The day an aezeed says it was made.
    birthday: Option<(i32, u32, u32)>,
}

impl Node {
    /// The node's public key, compressed: its node id.
    pub fn public(&self) -> &[u8; 33] {
        &self.public
    }

    /// The day an aezeed says it was made, as year, month and day.
    pub fn birthday(&self) -> Option<(i32, u32, u32)> {
        self.birthday
    }

    /// The private key as lower-case hex, for `why`: shown on screen
    /// (`Reveal::LightningKey`), or into a vault record or a file to the
    /// Outbox (`Reveal::VaultEntry` or `Reveal::OutboxFile` of
    /// `Part::NodeKey`). `None` for another reason.
    pub fn private_hex(&self, why: Reveal) -> Option<Zeroizing<String>> {
        matches!(
            why,
            Reveal::LightningKey
                | Reveal::VaultEntry(Part::NodeKey)
                | Reveal::OutboxFile(Part::NodeKey)
        )
        .then(|| faraday_intake::secret_buf::hex(&self.private[..]))
    }
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

impl crate::Keys {
    /// Costs what [`node_from_aezeed`] costs. The node key an aezeed gives,
    /// on the store's network; while the demo is on, refused unless the
    /// node is a public one ([`crate::Keys::is_public_node`]), as a key
    /// that is not a sample is: whatever else is typed would be someone's
    /// real node.
    pub fn node_from_aezeed(
        &self,
        c: &Compute,
        words: &[u16; AEZEED_WORDS],
        passphrase: &[u8],
    ) -> Result<Node, String> {
        let node = node_from_aezeed(c, words, passphrase, self.network())?;
        if !self.is_public_node(node.public()) {
            let id: String = node.public().iter().map(|b| format!("{b:02x}")).collect();
            return Err(faraday_strings::EN.wallet_is_not_a_key.replace("{}", &id));
        }
        Ok(node)
    }
}
