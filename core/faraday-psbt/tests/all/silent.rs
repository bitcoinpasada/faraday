//! BIP-375 silent payments in a PSBT against the BIP's published
//! vectors (`tools/vectors/bip375/bip375_test_vectors.json`).
//!
//! Every invalid PSBT is refused, for the reason its description names,
//! and every valid one passes. Every "can finalize" PSBT is then taken
//! back to the state before any signer, its shares, proofs and silent
//! payment scripts removed; its inputs' private keys (the vectors'
//! supplementary data) add the shares again through this crate's
//! signer, and the shares and scripts it sets are the vector's.
//!
//! Several vectors pair a P2WPKH output with a key it does not commit
//! to. Faraday refuses such an input (`silent.rs`, `key_for_hash`), so
//! [`repair`] first binds each such output to the key the PSBT states,
//! which changes no outpoint and no key, and so no share, proof or
//! script. The vectors as published are checked too: each is refused
//! for exactly that reason or passes.

use crate::common::{ABANDON, master, unhex};
use faraday_bip::keys::Network;
use faraday_crypto::work::Compute;
use faraday_psbt::bitcoin::bip32::DerivationPath;
use faraday_psbt::bitcoin::hashes::{Hash, hash160};
use faraday_psbt::bitcoin::key::{Keypair, TapTweak};
use faraday_psbt::bitcoin::secp256k1::{All, Secp256k1, SecretKey};
use faraday_psbt::silent::{self, Error};
use faraday_psbt::v2::{self, PsbtV2};
use serde_json::Value;

const VECTORS: &str = include_str!("../../../../tools/vectors/bip375/bip375_test_vectors.json");

/// Why the published vectors as they stand are refused when they are.
const UNBOUND: &str = "no public key the PSBT states is the one the input's script commits to";

fn vectors() -> Value {
    serde_json::from_str(VECTORS).expect("the vectors read")
}

fn parse(c: &Compute, v: &Value) -> PsbtV2 {
    PsbtV2::parse_base64(c, v["psbt"].as_str().expect("a psbt")).expect("a version 2 PSBT")
}

fn description(v: &Value) -> &str {
    v["description"].as_str().expect("a description")
}

/// The one invalid vector the version 2 reader itself refuses: an
/// output that is not a silent payment's and has no PSBT_OUT_SCRIPT,
/// which BIP-370 refuses on reading (BIP-375 lets only a silent payment
/// output leave its script out).
fn refused_on_reading(c: &Compute, v: &Value) -> bool {
    let read = PsbtV2::parse_base64(c, v["psbt"].as_str().expect("a psbt"));
    let missing = description(v).contains("missing PSBT_OUT_SCRIPT");
    match read {
        Err(e) => {
            assert!(missing, "{}: refused on reading: {e:?}", description(v));
            assert_eq!(e, v2::Error::Field("PSBT_OUT_SCRIPT"));
            true
        }
        Ok(_) => {
            assert!(!missing, "{}: read", description(v));
            false
        }
    }
}

/// The check a vector asks for: its `checks` list names the input
/// rules alone for two of them, and the whole check for the rest.
fn run(c: &Compute, secp: &Secp256k1<All>, v: &Value, psbt: &PsbtV2) -> Result<(), Error> {
    match v["checks"].as_array() {
        Some(checks) => {
            assert_eq!(checks.len(), 1);
            assert_eq!(checks[0], "input_eligibility");
            silent::check_inputs(psbt)
        }
        None => silent::check(c, secp, psbt).map(|_| ()),
    }
}

/// Binds each P2WPKH and P2SH-P2WPKH input's previous output to the
/// first compressed key its key origins state, as the vectors'
/// generator did not. Neither the outpoints nor the keys change, so
/// neither do the input hash, the shares, the proofs or the scripts.
fn repair(psbt: &mut PsbtV2) {
    for map in &mut psbt.inputs {
        let Some(key) = map
            .all_of(v2::IN_BIP32_DERIVATION)
            .map(|(k, _)| k.to_vec())
            .find(|k| k.len() == 33)
        else {
            continue;
        };
        let Some(utxo) = map.get_type(v2::IN_WITNESS_UTXO).map(<[u8]>::to_vec) else {
            continue;
        };
        let (amount, spk) = (&utxo[..8], &utxo[9..]);
        let mut p2wpkh = vec![0x00, 0x14];
        p2wpkh.extend_from_slice(&hash160::Hash::hash(&key).to_byte_array());
        let new_spk = if spk.len() == 22 && spk[..2] == [0x00, 0x14] {
            p2wpkh
        } else if spk.len() == 23
            && map
                .get_type(v2::IN_REDEEM_SCRIPT)
                .is_some_and(|r| r.len() == 22 && r[..2] == [0x00, 0x14])
        {
            let mut p2sh = vec![0xa9, 0x14];
            p2sh.extend_from_slice(&hash160::Hash::hash(&p2wpkh).to_byte_array());
            p2sh.push(0x87);
            map.set(vec![v2::IN_REDEEM_SCRIPT], p2wpkh);
            p2sh
        } else {
            continue;
        };
        let mut utxo = amount.to_vec();
        utxo.push(new_spk.len() as u8);
        utxo.extend_from_slice(&new_spk);
        map.set(vec![v2::IN_WITNESS_UTXO], utxo);
    }
}

/// A test of the error a vector is refused with.
type Reason = fn(&Error) -> bool;

/// What each invalid vector is refused for, by its description.
fn expected(description: &str) -> Reason {
    let table: &[(&str, Reason)] = &[
        ("missing PSBT_OUT_SP_V0_INFO field when", |e| {
            matches!(e, Error::LabelWithoutAddress { output: 0 })
        }),
        ("byte length for PSBT_OUT_SP_V0_LABEL", |e| {
            *e == Error::Field("PSBT_OUT_SP_V0_LABEL")
        }),
        ("byte length for PSBT_OUT_SP_V0_INFO", |e| {
            *e == Error::Field("PSBT_OUT_SP_V0_INFO")
        }),
        ("byte length for PSBT_IN_SP_ECDH_SHARE", |e| {
            *e == Error::Field("PSBT_IN_SP_ECDH_SHARE")
        }),
        ("byte length for PSBT_IN_SP_DLEQ", |e| {
            *e == Error::Field("PSBT_IN_SP_DLEQ")
        }),
        ("PSBT_GLOBAL_TX_MODIFIABLE field is non-zero", |e| {
            *e == Error::Modifiable
        }),
        (
            "missing PSBT_OUT_SCRIPT field when sending to non-sp",
            |e| matches!(e, Error::NoDestination { output: 0 }),
        ),
        ("only one ineligible P2SH multisig input", |e| {
            matches!(e, Error::NoShare { output: 0 })
        }),
        ("missing PSBT_IN_SP_ECDH_SHARE field for input 0", |e| {
            matches!(e, Error::NoShare { output: 0 })
        }),
        ("missing PSBT_IN_SP_DLEQ field", |e| {
            matches!(e, Error::MissingProof { input: Some(0) })
        }),
        ("missing PSBT_GLOBAL_SP_DLEQ field", |e| {
            matches!(e, Error::MissingProof { input: None })
        }),
        ("invalid proof in PSBT_IN_SP_DLEQ", |e| {
            matches!(e, Error::BadProof { input: Some(0) })
        }),
        ("invalid proof in PSBT_GLOBAL_SP_DLEQ", |e| {
            matches!(e, Error::BadProof { input: None })
        }),
        ("missing PSBT_IN_BIP32_DERIVATION field", |e| {
            matches!(e, Error::Unverifiable { input: 0, .. })
        }),
        ("output 1 missing ECDH share", |e| {
            matches!(e, Error::NoShare { output: 1 })
        }),
        ("input 1 missing ECDH share for output 1", |e| {
            matches!(e, Error::NoShare { output: 1 })
        }),
        ("input 1 missing ECDH share for scan key", |e| {
            matches!(e, Error::NoShare { output: 0 })
        }),
        ("segwit version greater than 1", |e| {
            matches!(e, Error::FutureSegwit { input: 0 })
        }),
        ("non-SIGHASH_ALL signature", |e| {
            matches!(e, Error::Sighash { input: 0 })
        }),
        ("NUMS internal key cannot derive", |e| {
            matches!(e, Error::NoShare { output: 0 })
        }),
        ("PSBT_OUT_SCRIPT does not match derived", |e| {
            matches!(e, Error::ScriptMismatch { output: 0 })
        }),
        ("not sorted lexicographically by spend key", |e| {
            matches!(e, Error::ScriptMismatch { output: 0 })
        }),
        ("k values assigned to wrong output indices", |e| {
            matches!(e, Error::ScriptMismatch { output: 1 })
        }),
    ];
    let found: Vec<_> = table
        .iter()
        .filter(|(text, _)| description.contains(text))
        .collect();
    assert_eq!(found.len(), 1, "one reason for {description}");
    found[0].1
}

#[test]
fn every_invalid_vector_is_refused_for_its_reason() {
    let c = Compute::unchecked();
    let secp = Secp256k1::new();
    let vectors = vectors();
    let invalid = vectors["invalid"].as_array().expect("a list");
    assert_eq!(invalid.len(), 23);
    let mut on_reading = 0;
    for v in invalid {
        if refused_on_reading(&c, v) {
            on_reading += 1;
            continue;
        }
        let mut psbt = parse(&c, v);
        assert!(run(&c, &secp, v, &psbt).is_err(), "{}", description(v));
        repair(&mut psbt);
        let refused = run(&c, &secp, v, &psbt).expect_err(description(v));
        assert!(
            expected(description(v))(&refused),
            "{}: {refused:?}",
            description(v)
        );
    }
    assert_eq!(on_reading, 1);
}

#[test]
fn every_valid_vector_passes() {
    let c = Compute::unchecked();
    let secp = Secp256k1::new();
    let vectors = vectors();
    let valid = vectors["valid"].as_array().expect("a list");
    assert_eq!(valid.len(), 20);
    let mut unbound = 0;
    for v in valid {
        let mut psbt = parse(&c, v);
        match run(&c, &secp, v, &psbt) {
            Ok(()) => {}
            Err(Error::Unverifiable { reason, .. }) if reason == UNBOUND => unbound += 1,
            Err(e) => panic!("{}: {e:?}", description(v)),
        }
        repair(&mut psbt);
        run(&c, &secp, v, &psbt).unwrap_or_else(|e| panic!("{}: {e:?}", description(v)));
        if description(v).starts_with("can finalize") {
            let report = silent::check(&c, &secp, &psbt).expect("passes");
            assert!(
                report.has_silent() && report.complete(),
                "{}",
                description(v)
            );
            assert!(report.outputs.iter().all(|o| o.stated));
        }
    }
    // The P2PKH and taproot vectors bind their keys as published.
    assert_eq!(unbound, 16);
}

#[test]
fn every_vector_reads_and_writes_back_byte_for_byte() {
    let c = Compute::unchecked();
    let vectors = vectors();
    for kind in ["invalid", "valid"] {
        for v in vectors[kind].as_array().expect("a list") {
            if refused_on_reading(&c, v) {
                continue;
            }
            let text = v["psbt"].as_str().expect("a psbt");
            assert_eq!(parse(&c, v).to_base64(), text, "{}", description(v));
        }
    }
}

/// The PSBT before any signer: no shares, no proofs, no silent payment
/// script, still modifiable.
fn unsigned(psbt: &PsbtV2) -> PsbtV2 {
    let mut psbt = psbt.clone();
    psbt.global.remove_type(silent::GLOBAL_SP_ECDH_SHARE);
    psbt.global.remove_type(silent::GLOBAL_SP_DLEQ);
    psbt.global.set(vec![v2::GLOBAL_TX_MODIFIABLE], vec![0x03]);
    for map in &mut psbt.inputs {
        map.remove_type(silent::IN_SP_ECDH_SHARE);
        map.remove_type(silent::IN_SP_DLEQ);
    }
    for map in &mut psbt.outputs {
        if map.get_type(silent::OUT_SP_V0_INFO).is_some() {
            map.remove_type(v2::OUT_SCRIPT);
        }
    }
    psbt
}

/// The vector's private keys, by input.
fn keys(v: &Value) -> Vec<(usize, SecretKey)> {
    v["supplementary"]["inputs"]
        .as_array()
        .expect("inputs")
        .iter()
        .filter_map(|i| {
            let key = i["private_key"].as_str().filter(|k| !k.is_empty())?;
            let index = i["input_index"].as_u64().expect("an index") as usize;
            Some((index, SecretKey::from_slice(&unhex(key)).expect("a key")))
        })
        .collect()
}

#[test]
fn the_signer_adds_the_shares_and_sets_the_scripts_of_every_finished_vector() {
    let c = Compute::unchecked();
    let secp = Secp256k1::new();
    let vectors = vectors();
    let mut done = 0;
    for v in vectors["valid"].as_array().expect("a list") {
        if !description(v).starts_with("can finalize") {
            continue;
        }
        let mut finished = parse(&c, v);
        repair(&mut finished);
        let mut psbt = unsigned(&finished);
        let before = silent::check(&c, &secp, &psbt).expect("passes before any share");
        assert!(!before.complete(), "{}", description(v));
        assert!(matches!(
            silent::signable(&c, &secp, &psbt),
            Err(Error::NotComplete { .. })
        ));
        for (index, key) in keys(v) {
            match silent::add_shares_for_keys(&c, &secp, &mut psbt, &[(index, &key)], &[0u8; 32]) {
                Ok(_) | Err(Error::NotEligible { .. }) => {}
                Err(e) => panic!("{}: input {index}: {e:?}", description(v)),
            }
        }
        let report = silent::check(&c, &secp, &psbt).expect("passes after the shares");
        assert!(report.complete(), "{}", description(v));
        for out in &report.outputs {
            assert_eq!(
                psbt.output_script(out.index),
                finished.output_script(out.index),
                "{}: output {}",
                description(v),
                out.index
            );
        }
        // The shares are the vector's wherever it has per-input ones on
        // an input whose key it gives. (Two vectors put a share on an
        // ineligible P2SH input, which contributes nothing and is left
        // out of every sum.)
        let held: Vec<usize> = keys(v).iter().map(|(i, _)| *i).collect();
        for (i, map) in finished.inputs.iter().enumerate() {
            if !held.contains(&i) {
                continue;
            }
            for (scan, share) in map.all_of(silent::IN_SP_ECDH_SHARE) {
                let mut key = vec![silent::IN_SP_ECDH_SHARE];
                key.extend_from_slice(scan);
                assert_eq!(psbt.inputs[i].get(&key), Some(share), "{}", description(v));
            }
        }
        assert_eq!(
            psbt.global.get_type(v2::GLOBAL_TX_MODIFIABLE),
            Some(&[0u8][..])
        );
        // The version 0 form carries the same transaction, and a signing
        // pass that adds nothing brings nothing back.
        let v0 = silent::signable(&c, &secp, &psbt).expect("signable");
        assert_eq!(
            v0.unsigned_tx(),
            &psbt.unsigned_tx().expect("a transaction")
        );
        assert_eq!(psbt.merge_signed(&v0), Ok(0));
        done += 1;
    }
    assert_eq!(done, 14);
}

#[test]
fn a_key_that_is_not_the_inputs_is_refused() {
    let c = Compute::unchecked();
    let secp = Secp256k1::new();
    let vectors = vectors();
    let v = &vectors["valid"][1];
    assert_eq!(
        description(v),
        "can finalize: one P2PKH input single-signer"
    );
    let mut psbt = unsigned(&parse(&c, v));
    let other = SecretKey::from_slice(&[0x01; 32]).expect("a key");
    assert_eq!(
        silent::add_shares_for_keys(&c, &secp, &mut psbt, &[(0, &other)], &[0u8; 32]),
        Err(Error::KeyMismatch { input: 0 })
    );
    assert_eq!(psbt, unsigned(&parse(&c, v)), "nothing was added");
}

/// The vector "in progress: one P2TR input / one sp output with no ECDH
/// shares", its input moved to `spk` with `fields` set.
fn one_input(c: &Compute, spk: &[u8], fields: &[(Vec<u8>, Vec<u8>)]) -> PsbtV2 {
    let vectors = vectors();
    let v = vectors["valid"]
        .as_array()
        .expect("a list")
        .iter()
        .find(|v| description(v).contains("one P2TR input / one sp output with no ECDH"))
        .expect("the vector");
    let mut psbt = parse(c, v);
    let map = &mut psbt.inputs[0];
    let utxo = map.get_type(v2::IN_WITNESS_UTXO).expect("a utxo").to_vec();
    let mut new = utxo[..8].to_vec();
    new.push(spk.len() as u8);
    new.extend_from_slice(spk);
    map.set(vec![v2::IN_WITNESS_UTXO], new);
    for (k, value) in fields {
        map.set(k.clone(), value.clone());
    }
    psbt
}

/// A key origin's value: no fingerprint check is needed beyond the
/// four bytes, then the path little-endian.
fn origin(fp: [u8; 4], path: &DerivationPath) -> Vec<u8> {
    let mut out = fp.to_vec();
    for step in path {
        out.extend_from_slice(&u32::from(*step).to_le_bytes());
    }
    out
}

#[test]
fn a_master_key_adds_the_share_its_derived_key_gives() {
    let c = Compute::unchecked();
    let secp = Secp256k1::new();
    let master = master(ABANDON, Network::Testnet);
    let fp = master.fingerprint().0;

    // P2WPKH at m/84'/1'/0'/0/0.
    let path: DerivationPath = "m/84'/1'/0'/0/0".parse().expect("a path");
    let derived = master.derive(&c, &path);
    let key = *derived.secret_key();
    let public = key.public_key(&secp);
    let mut spk = vec![0x00, 0x14];
    spk.extend_from_slice(&hash160::Hash::hash(&public.serialize()).to_byte_array());
    let mut origin_key = vec![v2::IN_BIP32_DERIVATION];
    origin_key.extend_from_slice(&public.serialize());
    let fields = [(origin_key, origin(fp, &path))];
    let mut by_master = one_input(&c, &spk, &fields);
    let mut by_key = by_master.clone();
    silent::add_shares(&c, &mut by_master, &master, &[0u8; 32]).expect("shares added");
    silent::add_shares_for_keys(&c, &secp, &mut by_key, &[(0, &key)], &[0u8; 32])
        .expect("shares added");
    assert_eq!(by_master, by_key);
    assert!(
        silent::check(&c, &secp, &by_master)
            .expect("passes")
            .complete()
    );

    // Taproot at m/86'/1'/0'/0/0, key path, no script tree.
    let path: DerivationPath = "m/86'/1'/0'/0/0".parse().expect("a path");
    let derived = master.derive(&c, &path);
    let keypair = Keypair::from_secret_key(&secp, derived.secret_key());
    let internal = keypair.x_only_public_key().0;
    let tweaked = keypair.tap_tweak(&secp, None).to_keypair();
    let mut spk = vec![0x51, 0x20];
    spk.extend_from_slice(&tweaked.x_only_public_key().0.serialize());
    let mut origin_key = vec![v2::IN_TAP_BIP32_DERIVATION];
    origin_key.extend_from_slice(&internal.serialize());
    let mut value = vec![0u8];
    value.extend_from_slice(&origin(fp, &path));
    let fields = [
        (origin_key, value),
        (vec![v2::IN_TAP_INTERNAL_KEY], internal.serialize().to_vec()),
    ];
    let mut by_master = one_input(&c, &spk, &fields);
    let mut by_key = by_master.clone();
    silent::add_shares(&c, &mut by_master, &master, &[0u8; 32]).expect("shares added");
    let output_key = tweaked.secret_key();
    silent::add_shares_for_keys(&c, &secp, &mut by_key, &[(0, &output_key)], &[0u8; 32])
        .expect("shares added");
    assert_eq!(by_master, by_key);
    let report = silent::check(&c, &secp, &by_master).expect("passes");
    assert!(report.complete());

    // Another master names no input, and adds nothing.
    let other = crate::common::master(crate::common::ZOO, Network::Testnet);
    let mut untouched = one_input(&c, &spk, &fields);
    let before = untouched.clone();
    silent::add_shares(&c, &mut untouched, &other, &[0u8; 32]).expect("nothing to add");
    assert_eq!(untouched, before);
}

#[test]
fn a_silent_payment_is_signed_through_its_version_0_form() {
    use faraday_psbt::{Aux, Context, KeyRef, Nonce, sign};
    let c = Compute::unchecked();
    let secp = Secp256k1::new();
    let master = master(ABANDON, Network::Testnet);
    let path: DerivationPath = "m/84'/1'/0'/0/0".parse().expect("a path");
    let public = master.derive(&c, &path).secret_key().public_key(&secp);
    let mut spk = vec![0x00, 0x14];
    spk.extend_from_slice(&hash160::Hash::hash(&public.serialize()).to_byte_array());
    let mut origin_key = vec![v2::IN_BIP32_DERIVATION];
    origin_key.extend_from_slice(&public.serialize());
    let fields = [(origin_key, origin(master.fingerprint().0, &path))];
    let mut psbt = one_input(&c, &spk, &fields);
    silent::add_shares(&c, &mut psbt, &master, &[0u8; 32]).expect("shares added");

    let mut v0 = silent::signable(&c, &secp, &psbt).expect("every script is set");
    let keys = [KeyRef::from_master(&c, &master, 0).expect("a key")];
    let ctx = Context {
        network: Network::Testnet,
        keys: &keys,
        wallets: &[],
        musig_session: None,
        shares: &[],
        carry: None,
    };
    let signed = sign(
        &c,
        &mut v0,
        &[&master],
        &[master.fingerprint()],
        &ctx,
        true,
        Nonce::LowR,
        Aux::Deterministic,
        &mut None,
        &faraday_crypto::Zeroizing::new([0u8; 32]),
        &[],
        &[],
    )
    .expect("signed");
    assert_eq!(signed.signed_inputs.len(), 1);
    assert_eq!(psbt.merge_signed(&v0), Ok(1));
    let report = silent::check(&c, &secp, &psbt).expect("passes signed");
    assert!(report.complete());
    let mut sig_key = vec![v2::IN_PARTIAL_SIG];
    sig_key.extend_from_slice(&public.serialize());
    assert_eq!(
        psbt.inputs[0].get(&sig_key),
        Some(&signed.signed_inputs[0].sig_bytes[..])
    );
}
