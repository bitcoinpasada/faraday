//! Start-up self-test (`docs/PLANNING.md` §8.5 #5, §12; UX.md §7.1 A3):
//! a small fixed set of published vectors run before Home is shown. A
//! failure names the check and blocks the app; the only way out is Exit.
//!
//! The expected values are copied from the vector files the integration
//! tests use (`tools/vectors/bip39/vectors.json`,
//! `tools/vectors/bip39/test_JP_BIP39.json`,
//! `tools/vectors/psbt/rfc6979.txt`, `core/osk-bip/tests/bip32.rs`,
//! `core/osk-bip/tests/accounts.rs`) and from libsecp256k1's copy of the
//! BIP-340 vectors; nothing here is retyped. The whole set runs in a few
//! milliseconds in a release build: one PBKDF2 at 2048 rounds per BIP-39
//! seed, a handful of EC operations, two signatures.
//!
//! The curve context these checks use is randomized like every other in
//! the product (security review M1), from a fixed seed: a self-test has
//! no session to derive one from, and what a context is blinded with
//! cannot change what it computes, which is what the checks are here to
//! confirm.

use alloc::string::ToString;
use alloc::vec::Vec;
use core::str::FromStr;

use osk_bip::bip39::{Language, Mnemonic};
use osk_bip::keys::{MasterKey, Network, ScriptType};
use osk_psbt::bitcoin::bip32::{DerivationPath, Xpriv};
use osk_psbt::bitcoin::hashes::{Hash, sha256d};
use osk_psbt::bitcoin::secp256k1::{All, Keypair, Message, Secp256k1, XOnlyPublicKey};
use osk_psbt::bitcoin::{NetworkKind, PrivateKey};

/// One check: a name for the failure screen and the function that runs
/// it. `true` means passed.
pub struct Check {
    /// Shown as `Self-test failed: <name>`.
    pub name: &'static str,
    /// The check.
    pub run: fn() -> bool,
}

/// The checks, in the order they run.
pub const CHECKS: [Check; 6] = [
    Check {
        name: "BIP-39 vector (entropy → words → seed)",
        run: bip39_vector,
    },
    Check {
        name: "BIP-39 Japanese vector (entropy → words → seed)",
        run: bip39_japanese_vector,
    },
    Check {
        name: "BIP-32 vector 1 (m/0H/1/2H)",
        run: bip32_vector,
    },
    Check {
        name: "BIP-84 first address",
        run: bip84_address,
    },
    Check {
        name: "ECDSA sign and verify (RFC 6979)",
        run: ecdsa_vector,
    },
    Check {
        name: "Schnorr sign and verify (BIP-340)",
        run: schnorr_vector,
    },
];

/// The context the signing checks run in: blinded from a fixed seed,
/// since there is no session here to derive one from.
fn secp() -> Secp256k1<All> {
    let mut secp = Secp256k1::new();
    secp.seeded_randomize(&[0x53; 32]);
    secp
}

/// What the self-test found: the number of checks that passed, or the
/// name of the first that failed.
pub type Outcome = Result<usize, &'static str>;

/// Runs [`CHECKS`].
pub fn run() -> Outcome {
    run_checks(&CHECKS)
}

/// Runs `checks` in order and stops at the first failure.
pub fn run_checks(checks: &[Check]) -> Outcome {
    for c in checks {
        if !(c.run)() {
            return Err(c.name);
        }
    }
    Ok(checks.len())
}

// ----- BIP-39: Trezor vector 0, passphrase TREZOR -----

const BIP39_ENTROPY: &str = "00000000000000000000000000000000";
const BIP39_MNEMONIC: &str =
    "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
const BIP39_SEED: &str = "c55257c360c07c72029aebc1b53c05ed0362ada38ead3e3e9efa3708e53495531f09a6987599d18264c1e1c92f2cf141630c7a3c4ab7c81b2f001698e7463b04";
const BIP39_PASSPHRASE: &[u8] = b"TREZOR";

fn bip39_vector() -> bool {
    let Some(entropy) = unhex(BIP39_ENTROPY) else {
        return false;
    };
    let Ok(m) = Mnemonic::from_entropy(Language::English, &entropy) else {
        return false;
    };
    let words: Vec<&str> = m
        .indices()
        .iter()
        .map(|&i| Language::English.word(i))
        .collect();
    if words.join(" ") != BIP39_MNEMONIC {
        return false;
    }
    let Ok(seed) = m.to_seed(BIP39_PASSPHRASE) else {
        return false;
    };
    unhex(BIP39_SEED).as_deref() == Some(&seed.expose()[..])
}

// ----- BIP-39: bip32JP vector 0, a non-Latin wordlist and a non-ASCII
// passphrase (`tools/vectors/bip39/test_JP_BIP39.json`, first entry) -----
//
// The words are written as a reader sees them, with the ideographic
// space the Japanese list is displayed with. The passphrase is not
// ASCII, so it is stated in the NFKD form BIP-39 hashes — the same bytes
// `core/osk-bip/tests/vectors.rs` derives from the file — and goes
// through `to_seed_unchecked`, which is that form without the ASCII
// check.

const BIP39_JP_ENTROPY: &str = "00000000000000000000000000000000";
const BIP39_JP_MNEMONIC: &str = "あいこくしん　あいこくしん　あいこくしん　あいこくしん　あいこくしん　あいこくしん　あいこくしん　あいこくしん　あいこくしん　あいこくしん　あいこくしん　あおぞら";
const BIP39_JP_PASSPHRASE_NFKD: &str = "e383a1e383bce38388e383abe382abe38299e3838fe38299e382a6e38299e382a1\
    e381afe3829ae381afe38299e3818fe38299e3829de38299e381a1e381a1e38299e58d81e4babae58d81e889b2";
const BIP39_JP_SEED: &str = "a262d6fb6122ecf45be09c50492b31f92e9beb7d9a845987a02cefda57a15f9c467a17872029a9e92299b5cbdf306e3a0ee620245cbd508959b6cb7ca637bd55";

fn bip39_japanese_vector() -> bool {
    let Some(entropy) = unhex(BIP39_JP_ENTROPY) else {
        return false;
    };
    let Ok(m) = Mnemonic::from_entropy(Language::Japanese, &entropy) else {
        return false;
    };
    let words: Vec<&str> = m
        .indices()
        .iter()
        .map(|&i| Language::Japanese.word_display(i))
        .collect();
    if words.join(Language::Japanese.separator()) != BIP39_JP_MNEMONIC {
        return false;
    }
    let Some(passphrase) = unhex(BIP39_JP_PASSPHRASE_NFKD) else {
        return false;
    };
    let Ok(seed) = m.to_seed_unchecked(&passphrase) else {
        return false;
    };
    unhex(BIP39_JP_SEED).as_deref() == Some(&seed.expose()[..])
}

// ----- BIP-32: vector 1, chain m/0H/1/2H -----

const BIP32_SEED: &str = "000102030405060708090a0b0c0d0e0f";
const BIP32_PATH: &str = "m/0h/1/2h";
const BIP32_XPUB: &str = "xpub6D4BDPcP2GT577Vvch3R8wDkScZWzQzMMUm3PWbmWvVJrZwQY4VUNgqFJPMM3No2dFDFGTsxxpG5uJh7n7epu4trkrX7x7DogT5Uv6fcLW5";

fn bip32_vector() -> bool {
    let Some(seed) = unhex(BIP32_SEED) else {
        return false;
    };
    let Ok(xpriv) = Xpriv::new_master(NetworkKind::Main, &seed) else {
        return false;
    };
    let Ok(master) = MasterKey::from_xpriv(xpriv) else {
        return false;
    };
    let Ok(path) = DerivationPath::from_str(BIP32_PATH) else {
        return false;
    };
    master.derive(&path).to_xpub().to_string() == BIP32_XPUB
}

// ----- BIP-84: first receive address of the "abandon … about" seed -----

const BIP84_ADDRESS: &str = "bc1qcr8te4kr609gcawutmrza0j4xv80jy8z306fyu";

fn bip84_address() -> bool {
    let Ok(m) = Mnemonic::parse(Language::English, BIP39_MNEMONIC) else {
        return false;
    };
    let Ok(seed) = m.to_seed(b"") else {
        return false;
    };
    let master = MasterKey::from_seed(&seed, Network::Mainnet);
    let Ok(account) = master.account_xpub(ScriptType::NativeSegwit, 0) else {
        return false;
    };
    account
        .address(false, 0)
        .is_ok_and(|a| a.to_string() == BIP84_ADDRESS)
}

// ----- ECDSA: Bitcoin Core key_tests strSecret1C over SHA256d(message) -----

const ECDSA_WIF: &str = "Kwr371tjA9u2rFSMZjTNun2PXXP3WPZu2afRHTcta6KxEUdm1vEw";
const ECDSA_MESSAGE: &[u8] = b"Very deterministic message";
const ECDSA_DER: &str = "304402205dbbddda71772d95ce91cd2d14b592cfbc1dd0aabd6a394b6c2d377bbe59d31d022014ddda21494a4e221f0824f0b8b924c43fa43c0ad57dccdaa11f81a6bd4582f6";

fn ecdsa_vector() -> bool {
    let Ok(key) = PrivateKey::from_wif(ECDSA_WIF) else {
        return false;
    };
    let secp = secp();
    let msg = Message::from_digest(sha256d::Hash::hash(ECDSA_MESSAGE).to_byte_array());
    let sig = secp.sign_ecdsa(&msg, &key.inner);
    if unhex(ECDSA_DER).as_deref() != Some(&sig.serialize_der()[..]) {
        return false;
    }
    secp.verify_ecdsa(&msg, &sig, &key.public_key(&secp).inner)
        .is_ok()
}

// ----- BIP-340: test vector 1, copied from libsecp256k1's own copy of
// the BIP's vectors (secp256k1/src/modules/schnorrsig/tests_impl.h) -----
//
// The vector fixes the auxiliary randomness, so the signature is a fixed
// 64 bytes. Taproot signing in this product uses none (§16.14), which is
// the same code path with the aux hashed as zeros; what this check
// covers is that the curve library signs and verifies BIP-340 as
// published.

const SCHNORR_SECRET: &str = "b7e151628aed2a6abf7158809cf4f3c762e7160f38b4da56a784d9045190cfef";
const SCHNORR_PUBLIC: &str = "dff1d77f2a671c5f36183726db2341be58feae1da2deced843240f7b502ba659";
const SCHNORR_AUX: &str = "0000000000000000000000000000000000000000000000000000000000000001";
const SCHNORR_MESSAGE: &str = "243f6a8885a308d313198a2e03707344a4093822299f31d0082efa98ec4e6c89";
const SCHNORR_SIG: &str = "6896bd60eeae296db48a229ff71dfe071bde413e6d43f917dc8dcf8c78de3341\
    8906d11ac976abccb20b091292bff4ea897efcb639ea871cfa95f6de339e4b0a";

fn schnorr_vector() -> bool {
    let (Some(secret), Some(public), Some(aux), Some(message), Some(expected)) = (
        unhex(SCHNORR_SECRET),
        unhex(SCHNORR_PUBLIC),
        unhex(SCHNORR_AUX),
        unhex(SCHNORR_MESSAGE),
        unhex(SCHNORR_SIG),
    ) else {
        return false;
    };
    let (Ok(digest), Ok(aux)) = (
        <[u8; 32]>::try_from(&message[..]),
        <[u8; 32]>::try_from(&aux[..]),
    ) else {
        return false;
    };
    let secp = secp();
    let Ok(mut keypair) = Keypair::from_seckey_slice(&secp, &secret) else {
        return false;
    };
    let Ok(expected_key) = XOnlyPublicKey::from_slice(&public) else {
        return false;
    };
    let msg = Message::from_digest(digest);
    let (key, _) = keypair.x_only_public_key();
    let sig = secp.sign_schnorr_with_aux_rand(&msg, &keypair, &aux);
    keypair.non_secure_erase();
    key == expected_key
        && sig.serialize()[..] == expected[..]
        && secp.verify_schnorr(&sig, &msg, &key).is_ok()
}

fn unhex(s: &str) -> Option<Vec<u8>> {
    if !s.len().is_multiple_of(2) {
        return None;
    }
    s.as_bytes()
        .chunks(2)
        .map(|pair| {
            let hi = (pair[0] as char).to_digit(16)?;
            let lo = (pair[1] as char).to_digit(16)?;
            Some((hi * 16 + lo) as u8)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_check_passes_on_its_own() {
        for c in &CHECKS {
            assert!((c.run)(), "{}", c.name);
        }
        assert_eq!(run(), Ok(CHECKS.len()));
    }

    #[test]
    fn the_first_failure_is_named() {
        fn ok() -> bool {
            true
        }
        fn bad() -> bool {
            false
        }
        let checks = [
            Check { name: "a", run: ok },
            Check {
                name: "b",
                run: bad,
            },
            Check {
                name: "c",
                run: bad,
            },
        ];
        assert_eq!(run_checks(&checks), Err("b"));
        assert_eq!(run_checks(&checks[..1]), Ok(1));
    }
}
