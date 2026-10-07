//! Nonce validation in deterministic mode (`docs/PLANNING.md` §8.4 #8,
//! §16.38): signing twice gives identical bytes under either nonce
//! setting, low R gives the shortest form of every signature, the
//! RFC 6979 nonces libsecp256k1 produces match vectors from two other
//! implementations, and the Schnorr setting decides whether a taproot
//! signature is reproducible or new every time.

mod common;

use std::str::FromStr;

use bitcoin::hashes::{Hash, sha256, sha256d};
use bitcoin::secp256k1::{Message, PublicKey, Secp256k1, SecretKey};
use bitcoin::{PrivateKey, ScriptBuf};
use common::builder::{Build, single_sig};
use common::{ABANDON, ZOO, master, records, unhex};
use osk_bip::keys::{Network, ScriptType};
use osk_psbt::{Aux, Context, KeyRef, Nonce, SigKind, sign};

const NET: Network = Network::Regtest;

fn foreign_script() -> ScriptBuf {
    master(ZOO, NET)
        .account_xpub(ScriptType::NativeSegwit, 0)
        .unwrap()
        .address(false, 1)
        .unwrap()
        .script_pubkey()
}

#[test]
fn signing_twice_is_byte_identical() {
    let abandon = master(ABANDON, NET);
    let keys = [KeyRef::from_master(&abandon, 0).unwrap()];
    let ctx = Context {
        network: NET,
        keys: &keys,
        wallets: &[],
        musig_session: None,
        shares: &[],
        carry: None,
    };
    for nonce in [Nonce::LowR, Nonce::First] {
        for script_type in ScriptType::ALL {
            let mut b = Build::new(script_type, NET);
            b.recipients.push((foreign_script(), 70_000));
            b.change = Some(29_000);
            let unsigned = single_sig(&abandon, &b);
            let mut runs = Vec::new();
            for _ in 0..2 {
                let mut psbt = unsigned.clone();
                let r = sign(
                    &mut psbt,
                    &[&abandon],
                    &[abandon.fingerprint()],
                    &ctx,
                    false,
                    nonce,
                    Aux::Deterministic,
                    &mut None,
                    [0u8; 32],
                    &[],
                    &[],
                )
                .unwrap();
                assert!(
                    r.signed_inputs
                        .iter()
                        .all(|s| s.deterministic_ok && s.verified)
                );
                runs.push((r.signed_inputs[0].sig_bytes.clone(), psbt.to_bytes()));
            }
            assert_eq!(runs[0], runs[1], "{script_type} {nonce:?}");
            // A fresh master key from the same seed gives the same bytes.
            let again = master(ABANDON, NET);
            let mut psbt = unsigned.clone();
            let r = sign(
                &mut psbt,
                &[&again],
                &[again.fingerprint()],
                &ctx,
                false,
                nonce,
                Aux::Deterministic,
                &mut None,
                [0u8; 32],
                &[],
                &[],
            )
            .unwrap();
            assert_eq!(
                r.signed_inputs[0].sig_bytes, runs[0].0,
                "{script_type} {nonce:?}"
            );
        }
    }
}

/// Under Low R every ECDSA signature the device makes is the shortest
/// form: `r` below 2^255, so its DER encoding is 70 bytes or fewer. That
/// is what a wallet comparing bytes against Bitcoin Core, Sparrow or
/// Electrum sees.
#[test]
fn low_r_gives_the_shortest_signature() {
    let abandon = master(ABANDON, NET);
    let keys = [KeyRef::from_master(&abandon, 0).unwrap()];
    let ctx = Context {
        network: NET,
        keys: &keys,
        wallets: &[],
        musig_session: None,
        shares: &[],
        carry: None,
    };
    // Enough different transactions that a first attempt with a high R
    // comes up and has to be ground away.
    let mut ecdsa = 0;
    for amount in 70_000..70_012u64 {
        for script_type in ScriptType::ALL {
            let mut b = Build::new(script_type, NET);
            b.recipients.push((foreign_script(), amount));
            b.change = Some(29_000);
            let mut psbt = single_sig(&abandon, &b);
            let r = sign(
                &mut psbt,
                &[&abandon],
                &[abandon.fingerprint()],
                &ctx,
                false,
                Nonce::LowR,
                Aux::Deterministic,
                &mut None,
                [0u8; 32],
                &[],
                &[],
            )
            .unwrap();
            for s in &r.signed_inputs {
                if s.kind != SigKind::Ecdsa {
                    continue;
                }
                let der = &s.sig_bytes[..s.sig_bytes.len() - 1];
                assert!(
                    der.len() <= 70,
                    "{script_type} paying {amount}: {} bytes",
                    der.len()
                );
                let sig = bitcoin::secp256k1::ecdsa::Signature::from_der(der).unwrap();
                assert!(
                    sig.serialize_compact()[0] < 0x80,
                    "{script_type} paying {amount}: high R"
                );
                ecdsa += 1;
            }
        }
    }
    assert_eq!(ecdsa, 36, "three ECDSA script types over twelve amounts");
}

/// python-ecdsa's RFC 6979 vectors state the nonce `k`; the signature's
/// `r` must be the x coordinate of `k·G`. Bitcoin Core's `key_tests`
/// state whole DER signatures over `SHA256d(message)`.
#[test]
fn rfc6979_cross_implementation_vectors() {
    let secp = Secp256k1::new();
    let recs = records("tools/vectors/psbt/rfc6979.txt");
    let mut checked = 0;
    for rec in &recs {
        let message = rec.get("message").as_bytes();
        if let Some(k) = rec.opt("k") {
            let sk = SecretKey::from_slice(&unhex(rec.get("privkey"))).unwrap();
            let msg = Message::from_digest(sha256::Hash::hash(message).to_byte_array());
            let sig = secp.sign_ecdsa(&msg, &sk);
            let r = &sig.serialize_compact()[..32];
            let nonce_point =
                PublicKey::from_secret_key(&secp, &SecretKey::from_slice(&unhex(k)).unwrap());
            let x = &nonce_point.serialize()[1..33];
            assert_eq!(r, x, "{}", rec.get("case"));
            secp.verify_ecdsa(&msg, &sig, &PublicKey::from_secret_key(&secp, &sk))
                .unwrap();
            checked += 1;
        }
        if let Some(der) = rec.opt("der") {
            let key = PrivateKey::from_wif(rec.get("wif")).unwrap();
            let msg = Message::from_digest(sha256d::Hash::hash(message).to_byte_array());
            let sig = secp.sign_ecdsa(&msg, &key.inner);
            assert_eq!(
                sig.serialize_der().to_vec(),
                unhex(der),
                "{}",
                rec.get("case")
            );
            checked += 1;
        }
    }
    assert_eq!(checked, 6);
    // Sanity: the two Core WIFs encode the same key, uncompressed and compressed.
    let a = PrivateKey::from_wif(recs[2].get("wif")).unwrap();
    let b = PrivateKey::from_wif(recs[3].get("wif")).unwrap();
    assert_eq!(a.inner, b.inner);
    assert!(!a.compressed && b.compressed);
    let _ = SecretKey::from_str("0000000000000000000000000000000000000000000000000000000000000001")
        .unwrap();
}

/// The Schnorr setting (security review 2026-09-11, L3): with fresh
/// auxiliary randomness the same taproot input signed twice gives two
/// different signatures, each verified against the key that made it,
/// and with the deterministic setting the two are the same bytes.
#[test]
fn fresh_auxiliary_randomness_changes_a_taproot_signature() {
    let abandon = master(ABANDON, NET);
    let keys = [KeyRef::from_master(&abandon, 0).unwrap()];
    let ctx = Context {
        network: NET,
        keys: &keys,
        wallets: &[],
        musig_session: None,
        shares: &[],
        carry: None,
    };
    let mut b = Build::new(ScriptType::Taproot, NET);
    b.recipients.push((foreign_script(), 70_000));
    b.change = Some(29_000);
    let unsigned = single_sig(&abandon, &b);

    let signatures = |aux| {
        let mut psbt = unsigned.clone();
        let r = sign(
            &mut psbt,
            &[&abandon],
            &[abandon.fingerprint()],
            &ctx,
            false,
            Nonce::LowR,
            aux,
            &mut None,
            [0u8; 32],
            &[],
            &[],
        )
        .unwrap();
        assert!(r.signed_inputs.iter().all(|s| s.verified));
        assert!(r.signed_inputs.iter().all(|s| s.kind == SigKind::Schnorr));
        r.signed_inputs
            .iter()
            .map(|s| s.sig_bytes.clone())
            .collect::<Vec<_>>()
    };

    let first = signatures(Aux::Fresh([0x11; 32]));
    let second = signatures(Aux::Fresh([0x22; 32]));
    assert_ne!(first[0], second[0], "same bytes from different aux");

    let once = signatures(Aux::Deterministic);
    let twice = signatures(Aux::Deterministic);
    assert_eq!(once, twice, "deterministic signing is not reproducible");
    assert_ne!(once[0], first[0], "fresh aux gave the deterministic bytes");
}
