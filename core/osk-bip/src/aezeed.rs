//! LND's aezeed cipher seed, read; and the two Lightning node keys —
//! LND's own and ldk-node's (`docs/PLANNING.md` §16.116).
//!
//! An aezeed is twenty-four words over the BIP-39 English list, which
//! is not a BIP-39 mnemonic: the words carry no BIP-39 checksum and the
//! 264 bits they hold are a version byte, an AEZ ciphertext, the scrypt
//! salt and a CRC-32C over all of it. What comes out of the ciphertext
//! is an internal version, a birthday counted in days since the Bitcoin
//! genesis block, and sixteen bytes of entropy, which LND uses as the
//! BIP-32 seed itself.
//!
//! Reading only. This device never makes an aezeed and never loads one
//! as a key: a Lightning node's identity key is hot by nature, and what
//! the tool answers is which node a backup belongs to.
//!
//! | Step | What it is |
//! |---|---|
//! | Words to bytes | 24 × 11 bits, most significant first |
//! | Checksum | CRC-32C over the first 29 bytes |
//! | Key | scrypt(`N` 32768, `r` 8, `p` 1) over the passphrase and the 5-byte salt |
//! | Decipher | AEZ v5, `tau` 4, associated data `version ‖ salt` |
//! | LND node key | `m/1017'/coin'/6'/0/0` from the entropy as the seed |
//! | ldk-node node key | `m/0'` from the master private key of a BIP-39 seed |

use osk_crypto::{Secret, SeedBytes, Zeroize, scrypt};

use crate::aez;
use crate::keys::{DerivationPath, DerivedKey, MasterKey, Network};

/// Words an aezeed is written in.
pub const NUM_WORDS: usize = 24;
/// Bytes those words hold: 24 × 11 bits.
pub const CIPHER_SEED_SIZE: usize = 33;
/// Bytes of entropy a cipher seed carries, which is the BIP-32 seed.
pub const ENTROPY_SIZE: usize = 16;
/// The deciphered seed: version, birthday, entropy.
const PLAIN_SIZE: usize = 1 + 2 + ENTROPY_SIZE;
/// The scrypt salt, which travels in the words.
const SALT_SIZE: usize = 5;
/// AEZ's expansion, which is the whole of the internal authentication.
const TAU: usize = 4;
/// Where the salt starts.
const SALT_OFFSET: usize = CIPHER_SEED_SIZE - 4 - SALT_SIZE;
/// Where the CRC-32C starts.
const CHECKSUM_OFFSET: usize = CIPHER_SEED_SIZE - 4;
/// The external version this codec reads, and the only one LND has
/// ever written.
pub const VERSION: u8 = 0;
/// scrypt's cost, as version 0 of the scheme fixes it.
pub const SCRYPT_N: u32 = 32768;
/// scrypt's block size.
const SCRYPT_R: u32 = 8;
/// scrypt's parallelism.
const SCRYPT_P: u32 = 1;
/// The passphrase LND substitutes when none is given.
const DEFAULT_PASSPHRASE: &[u8] = b"aezeed";

/// The key family LND derives a node identity key under, which is its
/// `KeyFamilyNodeKey`.
const NODE_KEY_FAMILY: u32 = 6;
/// LND's BIP-43 purpose.
const LND_PURPOSE: u32 = 1017;

/// Unix time of the Bitcoin genesis block, which is where a birthday is
/// counted from.
pub const GENESIS_UNIX: i64 = 1_231_006_505;

/// Why an aezeed could not be read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// The first byte is not a version this codec knows.
    Version(u8),
    /// The CRC-32C over the words does not match: a word is wrong, or
    /// two are in the wrong order.
    Checksum,
    /// The words are right and the passphrase is not. AEZ's expansion
    /// is what says so, and it cannot tell a wrong passphrase from a
    /// seed that was never an aezeed.
    Passphrase,
    /// scrypt refused the parameters, which cannot happen at the ones
    /// version 0 fixes.
    Kdf,
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Error::Version(_) => f.write_str("this is not a version 0 cipher seed"),
            Error::Checksum => f.write_str("the words do not check out"),
            Error::Passphrase => f.write_str("wrong passphrase"),
            Error::Kdf => f.write_str("the key could not be stretched"),
        }
    }
}

impl core::error::Error for Error {}

/// What a decoded aezeed holds. The entropy is the BIP-32 seed and is
/// zeroized on drop; the version and the birthday are not secret.
pub struct CipherSeed {
    /// The plaintext version, which tells a wallet which derivation
    /// scheme the entropy belongs to. LND has written 0 and 1.
    pub internal_version: u8,
    /// Days from the Bitcoin genesis block to the day the seed was
    /// made.
    pub birthday: u16,
    entropy: Secret<[u8; ENTROPY_SIZE]>,
}

impl CipherSeed {
    /// The sixteen bytes LND uses as the BIP-32 seed.
    pub fn entropy(&self) -> &Secret<[u8; ENTROPY_SIZE]> {
        &self.entropy
    }

    /// The birthday as a calendar day in UTC: year, month, day.
    pub fn birthday_date(&self) -> (i32, u32, u32) {
        birthday_date(self.birthday)
    }
}

/// Reads an aezeed. `words` are indices into the BIP-39 English list,
/// which is the list LND publishes as its own; an empty `passphrase`
/// is LND's default, the word `aezeed`.
pub fn decode(words: &[u16; NUM_WORDS], passphrase: &[u8]) -> Result<CipherSeed, Error> {
    decode_at_cost(words, passphrase, SCRYPT_N)
}

/// The same, stretched at a cost other than [`SCRYPT_N`].
///
/// It exists because LND's own published vectors are not decodable by
/// LND: `aezeed/cipherseed_test.go` has an `init()` that sets
/// `scryptN = 16` for the whole package, so the twenty-four words the
/// vectors state were enciphered under a key no released wallet would
/// derive. Nothing in the product calls this; the vectors do.
pub fn decode_at_cost(
    words: &[u16; NUM_WORDS],
    passphrase: &[u8],
    cost: u32,
) -> Result<CipherSeed, Error> {
    let mut bytes = pack(words);
    let result = decode_bytes(&bytes, passphrase, cost);
    bytes.zeroize();
    result
}

/// The 33 bytes of a cipher seed, deciphered.
fn decode_bytes(
    bytes: &[u8; CIPHER_SEED_SIZE],
    passphrase: &[u8],
    cost: u32,
) -> Result<CipherSeed, Error> {
    if bytes[0] != VERSION {
        return Err(Error::Version(bytes[0]));
    }
    let mut stated = [0u8; 4];
    stated.copy_from_slice(&bytes[CHECKSUM_OFFSET..]);
    if crc32c(&bytes[..CHECKSUM_OFFSET]) != u32::from_be_bytes(stated) {
        return Err(Error::Checksum);
    }

    let salt = &bytes[SALT_OFFSET..CHECKSUM_OFFSET];
    let pass = if passphrase.is_empty() {
        DEFAULT_PASSPHRASE
    } else {
        passphrase
    };
    let mut key = Secret::new([0u8; 32]);
    scrypt(pass, salt, cost, SCRYPT_R, SCRYPT_P, key.expose_mut()).map_err(|_| Error::Kdf)?;

    // The associated data is the external version and the salt, so a
    // seed cannot be replayed under either changed.
    let mut ad = [0u8; 1 + SALT_SIZE];
    ad[0] = bytes[0];
    ad[1..].copy_from_slice(salt);

    let mut plain = Secret::new([0u8; PLAIN_SIZE]);
    let ok = aez::decrypt(
        key.expose(),
        &[],
        &[&ad],
        TAU,
        &bytes[1..SALT_OFFSET],
        plain.expose_mut(),
    );
    if !ok {
        return Err(Error::Passphrase);
    }

    let mut entropy = Secret::new([0u8; ENTROPY_SIZE]);
    entropy
        .expose_mut()
        .copy_from_slice(&plain.expose()[3..PLAIN_SIZE]);
    Ok(CipherSeed {
        internal_version: plain.expose()[0],
        birthday: u16::from_be_bytes([plain.expose()[1], plain.expose()[2]]),
        entropy,
    })
}

/// The twenty-four eleven-bit indices as 33 bytes, most significant bit
/// of the first word first.
fn pack(words: &[u16; NUM_WORDS]) -> [u8; CIPHER_SEED_SIZE] {
    let mut out = [0u8; CIPHER_SEED_SIZE];
    for (n, word) in words.iter().enumerate() {
        for bit in 0..11 {
            if word >> (10 - bit) & 1 == 1 {
                let at = n * 11 + bit;
                out[at / 8] |= 0x80 >> (at % 8);
            }
        }
    }
    out
}

/// CRC-32C (Castagnoli), which is what LND checks the words with:
/// reflected, initialised and finished with all ones.
fn crc32c(data: &[u8]) -> u32 {
    /// The reflected Castagnoli polynomial.
    const POLY: u32 = 0x82f6_3b78;
    let mut crc = !0u32;
    for byte in data {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            crc = (crc >> 1) ^ (POLY & (crc & 1).wrapping_neg());
        }
    }
    !crc
}

/// LND's path for a node identity key, `m/1017'/coin'/6'/0/0`.
pub fn node_path(coin: u32) -> DerivationPath {
    let hardened = |i: u32| {
        crate::keys::ChildNumber::from_hardened_idx(i).expect("the four indices are below 2^31")
    };
    let normal = |i: u32| crate::keys::ChildNumber::from_normal_idx(i).expect("0 is a valid index");
    DerivationPath::from(
        [
            hardened(LND_PURPOSE),
            hardened(coin),
            hardened(NODE_KEY_FAMILY),
            normal(0),
            normal(0),
        ]
        .as_slice(),
    )
}

/// LND's node identity key from a cipher seed's entropy.
///
/// The entropy is the BIP-32 seed with no stretching — LND hands those
/// sixteen bytes straight to btcwallet — and the coin type is the
/// network's, 0 on mainnet and 1 everywhere else, which is what LND's
/// own chain table says.
pub fn node_key(entropy: &Secret<[u8; ENTROPY_SIZE]>, network: Network) -> DerivedKey {
    let seed = Secret::new(SeedBytes::new(entropy.expose()).expect("16 bytes is a seed length"));
    let master = MasterKey::from_seed_bytes(&seed, network);
    master.derive(&node_path(network.coin_type()))
}

/// ldk-node's node secret from a BIP-39 seed.
///
/// ldk-node makes a master key from the 64-byte seed, hands the master
/// *private key* to `KeysManager` as its own 32-byte seed, and LDK
/// makes a second master key from that and takes `m/0'`. The network
/// changes nothing: both master keys are used for their private key and
/// chain code, never serialised.
pub fn ldk_node_key(seed: &Secret<[u8; 64]>, network: Network) -> DerivedKey {
    let outer = MasterKey::from_seed(seed, network);
    let root = outer.derive(&DerivationPath::master());
    let mut inner_seed = Secret::new(
        SeedBytes::new(&root.secret_key().secret_bytes()).expect("32 bytes is a seed length"),
    );
    let inner = MasterKey::from_seed_bytes(&inner_seed, network);
    inner_seed.zeroize();
    let path = DerivationPath::from(
        [crate::keys::ChildNumber::from_hardened_idx(0).expect("0 is a valid index")].as_slice(),
    );
    inner.derive(&path)
}

/// The calendar day, in UTC, that a birthday counts to: the genesis
/// block's day plus `birthday` days.
///
/// The genesis block is 2009-01-03 18:15:05 UTC, so the day a birthday
/// names is the day that timestamp falls on once the days are added.
pub fn birthday_date(birthday: u16) -> (i32, u32, u32) {
    let seconds = GENESIS_UNIX + i64::from(birthday) * 86_400;
    civil_from_days(seconds.div_euclid(86_400))
}

/// Howard Hinnant's `civil_from_days`: a count of days from
/// 1970-01-01 to a proleptic Gregorian year, month and day.
fn civil_from_days(days: i64) -> (i32, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if m <= 2 { y + 1 } else { y };
    (year as i32, m as u32, d as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The entropy both of LND's published vectors carry.
    const ENTROPY: [u8; ENTROPY_SIZE] = [
        0x81, 0xb6, 0x37, 0xd8, 0x63, 0x59, 0xe6, 0x96, 0x0d, 0xe7, 0x95, 0xe4, 0x1e, 0x0b, 0x4c,
        0xfd,
    ];

    /// LND's `aezeed_test.go` first vector, made at the cost its own
    /// `init()` sets: no passphrase, the genesis day as the birthday.
    const PUBLISHED_1: &str = "ability liquid travel stem barely drastic pact cupboard apple \
        thrive morning oak feature tissue couch old math inform success suggest drink motion \
        know royal";
    /// Its second vector, under a passphrase.
    const PUBLISHED_2: &str = "able tree stool crush transfer cloud cross three profit outside \
        hen citizen plate ride require leg siren drum success suggest drink require fiscal \
        upgrade";
    /// The cost `cipherseed_test.go` sets for the package, which is
    /// what the two above were enciphered at.
    const PUBLISHED_COST: u32 = 16;

    /// The same two seeds — same entropy, salt, birthday and
    /// passphrase — enciphered at the cost a released LND uses.
    /// Produced by running LND's own `aezeed` package here with
    /// `scryptN` set back to 32768 (`tools/reference/aezeed/README.md`).
    const PRODUCTION_1: &str = "above judge emerge veteran reform crunch system all snap please \
        shoulder vault hurt city quarter cover enlist swear success suggest drink wagon enrich \
        body";
    const PRODUCTION_2: &str = "absorb century submit father path glove gloom super divert \
        garden ice mirror wisdom grass dice kit ugly castle success suggest drink monster \
        congress flight";
    /// The same, with an internal version of 1, which is the scheme
    /// LND derives taproot keys under.
    const PRODUCTION_V1: &str = "ability toilet excite swear ostrich model long squeeze solid \
        memory kit pepper arena equal spider beauty satoshi romance success suggest drink photo \
        already biology";

    /// The words as indices into the BIP-39 English list, which is the
    /// list LND publishes as its own.
    fn indices(words: &str) -> [u16; NUM_WORDS] {
        let list = crate::bip39::Language::English;
        let mut iter = words.split_whitespace();
        core::array::from_fn(|_| {
            let word = iter.next().expect("twenty-four words");
            (0..2048u16)
                .find(|n| list.word(*n) == word)
                .expect("every aezeed word is a BIP-39 English word")
        })
    }

    #[test]
    fn lnd_vectors_decode_to_their_entropy_version_and_birthday() {
        let seed = decode_at_cost(&indices(PUBLISHED_1), b"", PUBLISHED_COST).unwrap();
        assert_eq!(seed.internal_version, 0);
        assert_eq!(seed.birthday, 0);
        assert_eq!(seed.entropy().expose(), &ENTROPY);
        assert_eq!(seed.birthday_date(), (2009, 1, 3));

        let seed = decode_at_cost(
            &indices(PUBLISHED_2),
            b"!very_safe_55345_password*",
            PUBLISHED_COST,
        )
        .unwrap();
        assert_eq!(seed.internal_version, 0);
        assert_eq!(seed.birthday, 3365);
        assert_eq!(seed.entropy().expose(), &ENTROPY);
        assert_eq!(seed.birthday_date(), (2018, 3, 22));
    }

    #[test]
    fn the_same_seeds_at_the_cost_a_released_lnd_uses() {
        let seed = decode(&indices(PRODUCTION_1), b"").unwrap();
        assert_eq!((seed.internal_version, seed.birthday), (0, 0));
        assert_eq!(seed.entropy().expose(), &ENTROPY);

        let seed = decode(&indices(PRODUCTION_2), b"!very_safe_55345_password*").unwrap();
        assert_eq!((seed.internal_version, seed.birthday), (0, 3365));
        assert_eq!(seed.entropy().expose(), &ENTROPY);

        let seed = decode(&indices(PRODUCTION_V1), b"hello").unwrap();
        assert_eq!((seed.internal_version, seed.birthday), (1, 3365));
        assert_eq!(seed.entropy().expose(), &ENTROPY);
    }

    #[test]
    fn a_wrong_passphrase_is_refused_and_a_wrong_word_is_caught_first() {
        assert!(matches!(
            decode(&indices(PRODUCTION_2), b"wrong"),
            Err(Error::Passphrase)
        ));
        // No passphrase at all is LND's word `aezeed`, which is not
        // the passphrase this one was made under either.
        assert!(matches!(
            decode(&indices(PRODUCTION_2), b""),
            Err(Error::Passphrase)
        ));
        // One word changed: the CRC-32C catches it before any
        // stretching runs.
        let mut words = indices(PRODUCTION_1);
        words[0] = words[0].wrapping_add(1);
        assert!(matches!(decode(&words, b""), Err(Error::Checksum)));
        // Two words swapped, which a checksum over a set would miss.
        let mut words = indices(PRODUCTION_1);
        words.swap(3, 9);
        assert!(matches!(decode(&words, b""), Err(Error::Checksum)));
    }

    /// A compressed public key from its hex, so that a vector reads as
    /// the source states it and no heap text is made.
    fn public_key(hex: &str) -> [u8; 33] {
        let digit = |c: u8| match c {
            b'0'..=b'9' => c - b'0',
            b'a'..=b'f' => c - b'a' + 10,
            _ => panic!("a vector is lower-case hex"),
        };
        let bytes = hex.as_bytes();
        core::array::from_fn(|i| digit(bytes[2 * i]) << 4 | digit(bytes[2 * i + 1]))
    }

    fn public_key_of(key: &DerivedKey) -> [u8; 33] {
        crate::bitcoin::secp256k1::PublicKey::from_secret_key(key.secp(), key.secret_key())
            .serialize()
    }

    /// Run once against LND at commit `88959aec`, with its own
    /// `aezeed` package and btcd's `hdkeychain` deriving
    /// `m/1017'/coin'/6'/0/0` from the vector's entropy
    /// (`tools/reference/aezeed/README.md`).
    #[test]
    fn the_node_key_matches_what_lnd_derives() {
        let seed = decode(&indices(PRODUCTION_1), b"").unwrap();
        assert_eq!(
            public_key_of(&node_key(seed.entropy(), Network::Mainnet)),
            public_key("024c7005923a074fd38b16ace7be4914ec9f929717692bfb585019be13290c9b1d")
        );
        assert_eq!(
            public_key_of(&node_key(seed.entropy(), Network::Testnet)),
            public_key("0226594d21c0862a11168ab07cdbc15e7c7af5ee561b741259e311f1614f4df3b7")
        );
    }

    /// Run once against ldk-node's own derivation, which is
    /// `lightning` 0.2's `KeysManager::get_node_id` over the master
    /// private key of the BIP-39 seed.
    #[test]
    fn the_ldk_node_key_matches_what_ldk_node_derives() {
        let words =
            crate::bip39::Mnemonic::from_entropy(crate::bip39::Language::English, &[0u8; 16])
                .unwrap();
        let seed = words.to_seed(b"").unwrap();
        assert_eq!(
            public_key_of(&ldk_node_key(&seed, Network::Mainnet)),
            public_key("027cf7c81dc777e46572ff964f17650a0f6f783319fcc140fb9a69425e2fbdc93c")
        );
        // The network changes nothing: both master keys are used for
        // their private key and chain code and never serialised.
        assert_eq!(
            public_key_of(&ldk_node_key(&seed, Network::Testnet)),
            public_key("027cf7c81dc777e46572ff964f17650a0f6f783319fcc140fb9a69425e2fbdc93c")
        );
    }
}
