//! AEZ v5 against the vectors the reference implementation publishes,
//! vendored under `tools/reference/aezeed/aez-testdata/` with their
//! digests (`docs/PLANNING.md` §16.116).
//!
//! Four files, four layers: `Extract` turns a key of any length into
//! the three subkeys, `AEZ-hash` turns the nonce and the associated
//! data into the tweak, `AEZ-prf` is what an empty message enciphers
//! to, and `encrypt.json` is every message length from 0 to 511 bytes
//! at both of the expansions the reference tests.

mod common;

use common::unhex as hex;
use osk_bip::aez;
use serde_json::Value;

const EXTRACT: &str = include_str!("../../../tools/reference/aezeed/aez-testdata/extract.json");
const HASH: &str = include_str!("../../../tools/reference/aezeed/aez-testdata/hash.json");
const PRF: &str = include_str!("../../../tools/reference/aezeed/aez-testdata/prf.json");
const ENCRYPT: &str = include_str!("../../../tools/reference/aezeed/aez-testdata/encrypt.json");
const ENCRYPT_16: &str =
    include_str!("../../../tools/reference/aezeed/aez-testdata/encrypt_16_byte_key.json");
const ENCRYPT_NO_AD: &str =
    include_str!("../../../tools/reference/aezeed/aez-testdata/encrypt_no_ad.json");
const ENCRYPT_33: &str =
    include_str!("../../../tools/reference/aezeed/aez-testdata/encrypt_33_byte_ad.json");

fn cases(file: &str) -> Vec<Value> {
    serde_json::from_str::<Value>(file)
        .unwrap()
        .as_array()
        .unwrap()
        .clone()
}

/// `Extract(a) = b`: a 48-byte key is itself, any other length is
/// BLAKE2b of it.
#[test]
fn extract_vectors() {
    for case in cases(EXTRACT) {
        let key = hex(case["a"].as_str().unwrap());
        let want = hex(case["b"].as_str().unwrap());
        assert_eq!(aez::extract(&key).as_slice(), want.as_slice());
    }
}

/// `AEZ-hash(K, ([tau]_128, N, A…)) = V`. The file states tau in bits.
#[test]
fn hash_vectors() {
    for case in cases(HASH) {
        let key = hex(case["k"].as_str().unwrap());
        let data: Vec<Vec<u8>> = case["data"]
            .as_array()
            .unwrap()
            .iter()
            .map(|d| hex(d.as_str().unwrap()))
            .collect();
        let (nonce, rest) = match data.split_first() {
            Some((n, rest)) => (n.as_slice(), rest),
            None => ([].as_slice(), [].as_slice()),
        };
        let ad: Vec<&[u8]> = rest.iter().map(Vec::as_slice).collect();
        let tau = case["tau"].as_u64().unwrap() as u32;
        let want = hex(case["v"].as_str().unwrap());
        assert_eq!(aez::hash(&key, nonce, &ad, tau).as_slice(), want.as_slice());
    }
}

/// `AEZ-prf(K, delta, tau) = R`. Here tau is in bytes.
#[test]
fn prf_vectors() {
    for case in cases(PRF) {
        let key = hex(case["k"].as_str().unwrap());
        let delta: [u8; 16] = hex(case["delta"].as_str().unwrap()).try_into().unwrap();
        let want = hex(case["r"].as_str().unwrap());
        let mut out = vec![0u8; case["tau"].as_u64().unwrap() as usize];
        aez::prf(&key, &delta, &mut out);
        assert_eq!(out, want);
    }
}

/// `Encrypt(K, N, A, tau, M) = C` for every message length the
/// reference publishes, and the decryption that returns each one.
#[test]
fn encrypt_vectors() {
    for file in [ENCRYPT, ENCRYPT_16, ENCRYPT_NO_AD, ENCRYPT_33] {
        for case in cases(file) {
            let key = hex(case["k"].as_str().unwrap());
            let nonce = hex(case["nonce"].as_str().unwrap());
            let data: Vec<Vec<u8>> = case["data"]
                .as_array()
                .unwrap()
                .iter()
                .map(|d| hex(d.as_str().unwrap()))
                .collect();
            let ad: Vec<&[u8]> = data.iter().map(Vec::as_slice).collect();
            let tau = case["tau"].as_u64().unwrap() as usize;
            let message = hex(case["m"].as_str().unwrap());
            let want = hex(case["c"].as_str().unwrap());

            let mut out = vec![0u8; message.len() + tau];
            aez::encrypt(&key, &nonce, &ad, tau, &message, &mut out);
            assert_eq!(out, want, "{} bytes, tau {tau}", message.len());

            let mut back = vec![0u8; message.len()];
            assert!(aez::decrypt(&key, &nonce, &ad, tau, &want, &mut back));
            assert_eq!(back, message);
        }
    }
}
