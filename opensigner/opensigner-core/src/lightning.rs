//! Tools › Lightning node key (`docs/PLANNING.md` §16.116): the state
//! behind the tool that says which Lightning node a backup belongs to.
//!
//! Two sources. An LND cipher seed arrives as twenty-four words typed
//! on the Load wizard's entry over the English list, followed by the
//! passphrase; a loaded key gives ldk-node's node secret from the seed
//! it already holds. Either way the answer is one public key, and
//! nothing is added to Keys.
//!
//! The words, the passphrase, the entropy and the node private key live
//! inline here and are zeroized when the tool is left. The answer is
//! worked out once — an LND cipher seed costs a 32 MiB scrypt run, which
//! is not a thing to do on every frame — and kept beside the screen
//! until the screen goes. This file is listed in
//! `tools/lint-secrets.sh` and may hold no heap text.

use osk_bip::aezeed::{self, ENTROPY_SIZE, NUM_WORDS};
use osk_bip::bip39::MAX_PASSPHRASE_BYTES;
use osk_bip::keys::Network;
use osk_crypto::{Secret, Zeroize, ZeroizeOnDrop};

pub use crate::finish::MASK_MS;

/// Where the node key comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    /// Twenty-four words of an LND cipher seed, typed here.
    Aezeed,
    /// The loaded key at this index, read the way ldk-node reads a
    /// BIP-39 phrase.
    Loaded(usize),
}

/// Which screen of the tool is up.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// The Entry that types the cipher seed's passphrase.
    Passphrase,
    /// The Result: the node public key, and for an aezeed its version
    /// and birthday.
    Result,
    /// The Secret screen behind "Show secret", showing one of the two
    /// secrets a cipher seed has.
    Secret(SecretKind),
}

/// Which secret the Secret screen's panel holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecretKind {
    /// The node private key.
    NodeKey,
    /// The cipher seed's entropy, which is the BIP-32 seed LND builds
    /// every one of its keys from.
    Entropy,
}

impl Step {
    /// A number that tells one screen of the tool from another, so the
    /// app bar's eye stops when the screen changes.
    pub fn scope(self) -> u8 {
        match self {
            Step::Passphrase => 0,
            Step::Result => 1,
            Step::Secret(SecretKind::NodeKey) => 2,
            Step::Secret(SecretKind::Entropy) => 3,
        }
    }
}

/// What the tool worked out, or why it could not.
pub enum Answer {
    /// The node key, and what else the source stated.
    Node(Node),
    /// The cipher seed could not be read.
    Refused(aezeed::Error),
}

/// One node identity, as the Result and the Secret screen state it.
pub struct Node {
    /// The compressed public key, which is the node's identity on the
    /// network and is not a secret.
    pub public: [u8; 33],
    /// The private key behind it.
    pub private: Secret<[u8; 32]>,
    /// The cipher seed's own version and birthday, and the entropy it
    /// carried. `None` for a loaded key, which is not a cipher seed.
    pub seed: Option<Cipher>,
}

/// What an LND cipher seed says about itself.
pub struct Cipher {
    /// The plaintext version, which names LND's derivation scheme.
    pub version: u8,
    /// Days from the Bitcoin genesis block.
    pub birthday: u16,
    /// The sixteen bytes LND uses as the BIP-32 seed.
    pub entropy: Secret<[u8; ENTROPY_SIZE]>,
}

/// The tool, while it is on screen.
pub struct Lightning {
    source: Source,
    step: Step,
    /// The twenty-four indices, once the entry hands them over.
    words: [u16; NUM_WORDS],
    has_words: bool,
    passphrase: [u8; MAX_PASSPHRASE_BYTES],
    passphrase_len: u16,
    /// When the last character typed stops showing (§4.6).
    mask_until: Option<u64>,
    answer: Option<Answer>,
}

impl Zeroize for Lightning {
    fn zeroize(&mut self) {
        self.words.zeroize();
        self.has_words = false;
        self.passphrase.zeroize();
        self.passphrase_len = 0;
        self.answer = None;
    }
}

impl Drop for Lightning {
    fn drop(&mut self) {
        self.zeroize();
    }
}

impl ZeroizeOnDrop for Lightning {}

impl Lightning {
    /// The tool over a cipher seed whose words have not been typed yet.
    pub fn aezeed() -> Self {
        Lightning {
            source: Source::Aezeed,
            step: Step::Passphrase,
            words: [0; NUM_WORDS],
            has_words: false,
            passphrase: [0; MAX_PASSPHRASE_BYTES],
            passphrase_len: 0,
            mask_until: None,
            answer: None,
        }
    }

    /// The tool over the loaded key at `key`, which needs no typing.
    pub fn loaded(key: usize) -> Self {
        let mut it = Lightning::aezeed();
        it.source = Source::Loaded(key);
        it.step = Step::Result;
        it
    }

    /// Where the node key comes from.
    pub fn source(&self) -> Source {
        self.source
    }

    /// Which screen of the tool is up.
    pub fn step(&self) -> Step {
        self.step
    }

    /// Goes to `step`.
    pub fn go(&mut self, step: Step) {
        self.step = step;
    }

    /// The words the entry typed, once they are all in.
    pub fn set_words(&mut self, indices: &[u16]) {
        if indices.len() != NUM_WORDS {
            return;
        }
        self.words.copy_from_slice(indices);
        self.has_words = true;
        self.step = Step::Passphrase;
    }

    /// Whether the twenty-four words are in hand.
    pub fn has_words(&self) -> bool {
        self.has_words
    }

    // ----- the passphrase -----

    /// Characters typed so far.
    pub fn passphrase_len(&self) -> usize {
        usize::from(self.passphrase_len)
    }

    /// Adds one printable ASCII character, `false` when the field is
    /// full.
    pub fn passphrase_push(&mut self, c: char, now_ms: u64) -> bool {
        let n = usize::from(self.passphrase_len);
        if n >= MAX_PASSPHRASE_BYTES || !c.is_ascii_graphic() && c != ' ' {
            return false;
        }
        self.passphrase[n] = c as u8;
        self.passphrase_len += 1;
        self.mask_until = Some(now_ms + MASK_MS);
        self.answer = None;
        true
    }

    /// Deletes the last character.
    pub fn passphrase_pop(&mut self) {
        if self.passphrase_len > 0 {
            self.passphrase_len -= 1;
            self.passphrase[usize::from(self.passphrase_len)] = 0;
        }
        self.mask_until = None;
        self.answer = None;
    }

    /// The last character typed, while it is still showing.
    pub fn passphrase_visible_char(&self, now_ms: u64) -> Option<char> {
        if self.passphrase_len == 0 || now_ms >= self.mask_until? {
            return None;
        }
        Some(self.passphrase[usize::from(self.passphrase_len) - 1] as char)
    }

    /// When the shell should redraw so the last character stops
    /// showing.
    pub fn mask_deadline(&self) -> Option<u64> {
        (self.step == Step::Passphrase && self.passphrase_len > 0).then_some(self.mask_until)?
    }

    // ----- the answer -----

    /// What the tool worked out, if it has.
    pub fn answer(&self) -> Option<&Answer> {
        self.answer.as_ref()
    }

    /// Reads the cipher seed and derives LND's node key, at whatever the
    /// passphrase field holds. Called once, when ✓ leaves the entry.
    pub fn resolve_aezeed(&mut self, network: Network) {
        if !self.has_words {
            return;
        }
        let passphrase = &self.passphrase[..usize::from(self.passphrase_len)];
        self.answer = Some(match aezeed::decode(&self.words, passphrase) {
            Err(e) => Answer::Refused(e),
            Ok(seed) => {
                let key = aezeed::node_key(seed.entropy(), network);
                let mut entropy = Secret::new([0u8; ENTROPY_SIZE]);
                entropy
                    .expose_mut()
                    .copy_from_slice(seed.entropy().expose());
                Answer::Node(Node {
                    public: osk_bip::bitcoin::secp256k1::PublicKey::from_secret_key(
                        key.secp(),
                        key.secret_key(),
                    )
                    .serialize(),
                    private: Secret::new(key.secret_key().secret_bytes()),
                    seed: Some(Cipher {
                        version: seed.internal_version,
                        birthday: seed.birthday,
                        entropy,
                    }),
                })
            }
        });
        self.step = Step::Result;
    }

    /// ldk-node's node secret from a loaded key's 64-byte BIP-39 seed.
    pub fn resolve_loaded(&mut self, seed: &[u8], network: Network) {
        if seed.len() != 64 {
            return;
        }
        let mut bytes = Secret::new([0u8; 64]);
        bytes.expose_mut().copy_from_slice(seed);
        let key = aezeed::ldk_node_key(&bytes, network);
        self.answer = Some(Answer::Node(Node {
            public: osk_bip::bitcoin::secp256k1::PublicKey::from_secret_key(
                key.secp(),
                key.secret_key(),
            )
            .serialize(),
            private: Secret::new(key.secret_key().secret_bytes()),
            seed: None,
        }));
    }
}
