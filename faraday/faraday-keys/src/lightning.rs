//! Lightning node keys: ldk-node's from a BIP-39 seed, LND's from an
//! aezeed.
#![cfg_attr(not(test), warn(clippy::disallowed_methods))]

use faraday_bip::aezeed;
use faraday_bip::bip39::{Language, Mnemonic};
use faraday_bip::bitcoin::secp256k1::{PublicKey, Secp256k1};
use faraday_bip::keys::{DerivedKey, MasterKey, Network, SecretKey};
use faraday_crypto::work::Compute;
use faraday_crypto::{Secret, SeedBytes};
use zeroize::{Zeroize, Zeroizing};

use crate::session::{KeyId, KeyInfo, Secrets};
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
        .then(|| faraday_crypto::hex::encode_secret(&self.private[..]))
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

/// Costs an elliptic-curve multiplication: the node that the private key
/// `key` is, its public key worked out from it.
#[expect(
    clippy::disallowed_methods,
    reason = "the multiplication is counted by the token this function takes"
)]
pub(crate) fn node_of_secret_key(c: &Compute, key: &SecretKey) -> Node {
    c.count(faraday_crypto::work::Work::KeyDerive);
    let secp = Secp256k1::signing_only();
    Node {
        public: PublicKey::from_secret_key(&secp, key).serialize(),
        private: Zeroizing::new(key.secret_bytes()),
        birthday: None,
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
    let m =
        Mnemonic::parse(c, language, words).map_err(|e| faraday_strings::bip::bip39_error(&e))?;
    let seed = m
        .to_seed(c, passphrase.as_bytes())
        .map_err(|e| faraday_strings::bip::bip39_error(&e))?;
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

/// An aezeed read to run on another thread (`docs/WORKERS.md` §6): the
/// word indices and the passphrase, moved in, wiped as they drop.
pub struct AezeedJob {
    words: Zeroizing<[u16; AEZEED_WORDS]>,
    passphrase: faraday_intake::secret_buf::SecretText,
    network: Network,
    /// Whether the read is for Convert: its entropy held as a seed, not
    /// its node key.
    for_convert: bool,
}

/// What an [`AezeedJob`] gives back: the node key or the held seed, or why
/// the words are no aezeed. It joins the screen through
/// [`crate::Keys::aezeed_node`] or [`crate::Keys::aezeed_hold`], on the
/// main thread, and wipes as it drops.
pub struct AezeedRead {
    found: Found,
}

enum Found {
    Node(Result<Node, String>),
    Seed(Result<(HeldAezeed, [u8; 33]), String>),
}

/// An aezeed read for Convert (`docs/SEED-CONVERSION.md` §4.5): its 16
/// bytes of entropy, which LND uses as the BIP-32 seed itself, and its
/// birthday. It is never a key of the session: it is not in the list, the
/// vault, Explore or Lock in, and it leaves only through the conversions
/// of [`crate::operations`], for `Reveal::PaperForm`. Wiped as it drops.
pub struct HeldAezeed {
    entropy: Zeroizing<[u8; aezeed::ENTROPY_SIZE]>,
    birthday: (i32, u32, u32),
    master: MasterKey,
    info: KeyInfo,
}

impl HeldAezeed {
    /// Lends the entropy and master key as a conversion's source.
    pub(crate) fn secrets(&self) -> Secrets<'_> {
        Secrets {
            master: &self.master,
            words: None,
            passphrase: None,
            seed: Some(&self.entropy[..]),
            language: faraday_bip::bip39::Language::English,
        }
    }
}

/// Costs what [`node_from_aezeed`] costs and a master key. The aezeed read
/// as a seed to hold, with its node's public key (for the demo's check).
fn seed_from_aezeed(
    c: &Compute,
    words: &[u16; AEZEED_WORDS],
    passphrase: &[u8],
    network: Network,
) -> Result<(HeldAezeed, [u8; 33]), String> {
    let seed = aezeed::decode(c, words, passphrase).map_err(|e| format!("{e:?}"))?;
    let mut entropy = Secret::new([0u8; aezeed::ENTROPY_SIZE]);
    entropy
        .expose_mut()
        .copy_from_slice(seed.entropy().expose());
    let public = node_of(c, &aezeed::node_key(c, &entropy, network), None).public;
    let bytes = SeedBytes::new(&entropy.expose()[..]).ok_or_else(|| String::from("length"))?;
    let master = MasterKey::from_seed_bytes(c, &Secret::new(bytes), network);
    let info = KeyInfo {
        id: KeyId(crate::identity::master_id(c, &master)),
        fingerprint: master.fingerprint(),
        label: String::new(),
        language: faraday_bip::bip39::Language::English,
        word_count: None,
        seed_len: Some(aezeed::ENTROPY_SIZE),
        has_passphrase: false,
        codex32: None,
        child_of: None,
    };
    Ok((
        HeldAezeed {
            entropy: Zeroizing::new(*entropy.expose()),
            birthday: aezeed::birthday_date(seed.birthday),
            master,
            info,
        },
        public,
    ))
}

impl AezeedJob {
    /// Costs what [`node_from_aezeed`] costs. The read, with its inputs
    /// wiped as it ends.
    pub fn run(self, c: &Compute) -> AezeedRead {
        let pass = self.passphrase.as_bytes();
        AezeedRead {
            found: if self.for_convert {
                Found::Seed(seed_from_aezeed(c, &self.words, pass, self.network))
            } else {
                Found::Node(node_from_aezeed(c, &self.words, pass, self.network))
            },
        }
    }
}

impl crate::Keys {
    /// A read of the aezeed `words` (indices into the English list) under
    /// `passphrase`, which is moved in, on the store's network.
    pub fn aezeed_job(
        &self,
        words: &[u16; AEZEED_WORDS],
        passphrase: faraday_intake::secret_buf::SecretText,
    ) -> AezeedJob {
        AezeedJob {
            words: Zeroizing::new(*words),
            passphrase,
            network: self.network(),
            for_convert: false,
        }
    }

    /// [`Keys::aezeed_job`] for Convert: the read is held as a seed
    /// ([`Keys::aezeed_hold`]), not as a node key.
    pub fn aezeed_seed_job(
        &self,
        words: &[u16; AEZEED_WORDS],
        passphrase: faraday_intake::secret_buf::SecretText,
    ) -> AezeedJob {
        AezeedJob {
            for_convert: true,
            ..self.aezeed_job(words, passphrase)
        }
    }

    /// The node key an aezeed read gave; while the demo is on, refused
    /// unless the node is a public one ([`crate::Keys::is_public_node`]),
    /// as a key that is not a sample is: whatever else is typed would be
    /// someone's real node. The refused key wipes as it drops.
    pub fn aezeed_node(&self, read: AezeedRead) -> Result<Node, String> {
        let Found::Node(found) = read.found else {
            return Err(String::new());
        };
        let node = found?;
        if !self.is_public_node(node.public()) {
            let id: String = faraday_crypto::hex::encode_string(node.public());
            return Err(faraday_strings::EN.wallet_is_not_a_key.replace("{}", &id));
        }
        Ok(node)
    }

    /// An aezeed read for Convert, held as the source of its conversions,
    /// with the demo's refusal as [`Keys::aezeed_node`] has it, and refused
    /// while a stick is attached, as every way a key is added is; the
    /// fingerprint of its BIP-32 seed. Any aezeed held before is dropped,
    /// and any SLIP-39 set held as a source. It is not a key: [`Keys::len`] and the lists do not change.
    pub fn aezeed_hold(
        &mut self,
        read: AezeedRead,
    ) -> Result<faraday_bip::keys::Fingerprint, String> {
        let Found::Seed(found) = read.found else {
            return Err(String::new());
        };
        if let Some(noun) = self.stick_attached() {
            return Err(faraday_strings::EN.stick_keys_keys_load.replace("{}", noun));
        }
        let (held, public) = found.map_err(|e| {
            faraday_strings::EN
                .lightning_not_an_aezeed
                .replace("{}", &e)
        })?;
        if !self.is_public_node(&public) {
            let id: String = faraday_crypto::hex::encode_string(&public);
            return Err(faraday_strings::EN.wallet_is_not_a_key.replace("{}", &id));
        }
        let fp = held.info.fingerprint;
        self.aezeed = Some(held);
        self.slip39_set = None;
        Ok(fp)
    }

    /// The held aezeed is wiped and dropped.
    pub fn aezeed_forget(&mut self) {
        self.aezeed = None;
    }

    /// The place that names the held aezeed as a conversion's source.
    pub const AEZEED_SOURCE: usize = usize::MAX;

    /// The public facts of conversion source `i`: key `i`, or the held
    /// aezeed at [`Keys::AEZEED_SOURCE`].
    pub fn source_info(&self, i: usize) -> Option<&KeyInfo> {
        if i == Self::AEZEED_SOURCE {
            self.aezeed.as_ref().map(|a| &a.info)
        } else {
            self.all().nth(i)
        }
    }

    /// The held aezeed's birthday, as year, month and day.
    pub fn aezeed_birthday(&self) -> Option<(i32, u32, u32)> {
        self.aezeed.as_ref().map(|a| a.birthday)
    }

    /// Costs what [`node_from_aezeed`] costs. The node key an aezeed gives,
    /// on the store's network, with [`crate::Keys::aezeed_node`]'s refusal.
    pub fn node_from_aezeed(
        &self,
        c: &Compute,
        words: &[u16; AEZEED_WORDS],
        passphrase: &[u8],
    ) -> Result<Node, String> {
        self.aezeed_node(AezeedRead {
            found: Found::Node(node_from_aezeed(c, words, passphrase, self.network())),
        })
    }
}
