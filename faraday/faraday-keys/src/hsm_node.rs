//! Core Lightning's `hsm_secret` as a source of Lightning node key, the
//! parts that carry text: BIP-39 words made into the secret, and the
//! demo's refusal of a node that is not a public one
//! (`docs/decisions/F228.md`). The file and its opening are in
//! [`crate::hsm_secret`].
#![cfg_attr(not(test), warn(clippy::disallowed_methods))]

use faraday_bip::bip39::{Language, Mnemonic};
use faraday_crypto::work::Compute;
use zeroize::Zeroizing;

use crate::hsm_secret::{HsmRead, PLAIN_LEN, Refusal, node_from_secret};
use crate::lightning::Node;

/// Why no node came from BIP-39 words.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(
    clippy::disallowed_methods,
    reason = "a derived Clone: no secret type derives it"
)]
pub enum WordsRefusal {
    /// The words are not in English: Core Lightning takes only the
    /// English list, so no node of its could come from them.
    NotEnglish,
    /// The words are not valid BIP-39 words; the line says why.
    Words(String),
}

/// Costs a PBKDF2, key derivations and an elliptic-curve multiplication.
/// The node Core Lightning has for the English BIP-39 `words` and
/// `passphrase`: `lightning-hsmtool generatehsm` keeps the words, and
/// Core Lightning's secret is the first 32 bytes of their BIP-39 seed.
/// Words in another language are refused, as Core Lightning refuses them.
pub fn node_from_words(
    c: &Compute,
    language: Language,
    words: &str,
    passphrase: &str,
) -> Result<Node, WordsRefusal> {
    if !matches!(language, Language::English) {
        return Err(WordsRefusal::NotEnglish);
    }
    let words_error = |e: &_| WordsRefusal::Words(faraday_strings::bip::bip39_error(e));
    let m = Mnemonic::parse(c, language, words).map_err(|e| words_error(&e))?;
    let seed = m
        .to_seed(c, passphrase.as_bytes())
        .map_err(|e| words_error(&e))?;
    let mut secret = Zeroizing::new([0u8; PLAIN_LEN]);
    secret.copy_from_slice(&seed.expose()[..PLAIN_LEN]);
    Ok(node_from_secret(c, &secret))
}

impl crate::Keys {
    /// The node key an `hsm_secret` read gave; while the demo is on,
    /// refused unless the node is a public one
    /// ([`crate::Keys::is_public_node`]), as a key that is not a sample
    /// is: whatever else is opened would be someone's real node. A
    /// refused key wipes as it drops.
    pub fn hsm_node(&self, read: HsmRead) -> Result<Node, NodeRefusal> {
        let node = read.found.map_err(NodeRefusal::File)?;
        self.refuse_unless_public(node)
    }

    /// Costs what [`node_from_words`] costs, and a parse of the words for
    /// each list tried (English once more in [`node_from_words`]). The node key for the BIP-39
    /// `words` typed, with the same demo refusal as [`Self::hsm_node`].
    /// The list is told as Add a key tells it, English first; words of
    /// another list are refused as [`WordsRefusal::NotEnglish`], and words
    /// of none give English's refusal.
    pub fn hsm_words_node(
        &self,
        c: &Compute,
        words: &str,
        passphrase: &str,
    ) -> Result<Node, NodeRefusal> {
        let language = Language::ALL
            .into_iter()
            .find(|l| Mnemonic::parse(c, *l, words).is_ok())
            .unwrap_or(Language::English);
        let node = node_from_words(c, language, words, passphrase).map_err(NodeRefusal::Words)?;
        self.refuse_unless_public(node)
    }

    /// Costs what [`node_from_words`] costs. Core Lightning's node from
    /// the words and passphrase of loaded key `i`, beside ldk-node's
    /// ([`Self::lightning_node`]). `None` for a key with no words. A
    /// loaded key is not refused in the demo, as its ldk-node is not;
    /// words of another list than English are refused.
    pub fn hsm_key_node(&self, c: &Compute, i: usize) -> Option<Result<Node, WordsRefusal>> {
        let s = self.secrets(i)?;
        let words = s.words?;
        Some(node_from_words(
            c,
            s.language,
            words,
            s.passphrase.unwrap_or(""),
        ))
    }

    fn refuse_unless_public(&self, node: Node) -> Result<Node, NodeRefusal> {
        if !self.is_public_node(node.public()) {
            let id: String = faraday_crypto::hex::encode_string(node.public());
            return Err(NodeRefusal::Demo(
                faraday_strings::EN.wallet_is_not_a_key.replace("{}", &id),
            ));
        }
        Ok(node)
    }
}

/// Why [`crate::Keys::hsm_node`] gave no node.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(
    clippy::disallowed_methods,
    reason = "a derived Clone: no secret type derives it"
)]
pub enum NodeRefusal {
    /// The file or passphrase.
    File(Refusal),
    /// The words.
    Words(WordsRefusal),
    /// The demo's line: the node is not a public one.
    Demo(String),
}
