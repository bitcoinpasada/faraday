//! Signing with a WIF key for a sweep (`faraday_psbt::sweep`), and the
//! refusal of a legacy input whose script code holds `OP_CODESEPARATOR`
//! (`docs/INTEROP.md` §5 I11).

use crate::common::builder::funding_tx;
use crate::common::{ABANDON, hex, master, unhex};
use bitcoin::absolute::LockTime;
use bitcoin::bip32::{DerivationPath, Fingerprint as BipFingerprint};
use bitcoin::consensus::{deserialize, serialize};
use bitcoin::opcodes::all::{OP_CHECKSIG, OP_CODESEPARATOR};
use bitcoin::script::Builder;
use bitcoin::transaction::Version;
use bitcoin::{Amount, OutPoint, ScriptBuf, Sequence, Transaction, TxIn, TxOut, Witness};
use faraday_bip::keys::{Network, ScriptType};
use faraday_bip::wif::SingleKey;
use faraday_crypto::work::Compute;
use faraday_psbt::{Context, Error, KeyRef, Nonce, Psbt, Reason, finalize, sign, sweep};

fn ctx(network: Network) -> Context<'static> {
    Context {
        network,
        keys: &[],
        wallets: &[],
        musig_session: None,
    }
}

/// The WIF of 32 key bytes, compressed or not, on mainnet or the test
/// networks, written by the `bitcoin` crate's base58check.
fn wif(key: &[u8], compressed: bool, main: bool) -> String {
    let mut p = vec![if main { 0x80 } else { 0xef }];
    p.extend_from_slice(key);
    if compressed {
        p.push(0x01);
    }
    bitcoin::base58::encode_check(&p)
}

/// BIP-143's P2SH-P2WPKH example signed as a sweep of its key: the
/// signature is the one BIP-143 publishes, and the finished transaction
/// is BIP-143's signed one, byte for byte.
#[test]
fn bip143s_nested_segwit_example_sweeps_to_its_published_transaction() {
    let c = Compute::unchecked();
    let unsigned = "0100000001db6b1b20aa0fd7b23880be2ecbd4a98130974cf4748fb66092ac4d3ceb1a54770100000000feffffff02b8b4eb0b000000001976a914a457b684d7f0d539a46a45bbc043f35b59d0d96388ac0008af2f000000001976a914fd270b1ee6abcaea97fea7ad0402e8bd8ad6d77c88ac92040000";
    let signed = "01000000000101db6b1b20aa0fd7b23880be2ecbd4a98130974cf4748fb66092ac4d3ceb1a5477010000001716001479091972186c449eb1ded22b78e40d009bdf0089feffffff02b8b4eb0b000000001976a914a457b684d7f0d539a46a45bbc043f35b59d0d96388ac0008af2f000000001976a914fd270b1ee6abcaea97fea7ad0402e8bd8ad6d77c88ac02473044022047ac8e878352d3ebbde1c94ce3a10d057c24175747116f8288e5d794d12d482f0220217f36a485cae903c713331d877c1f64677e3622ad4010726870540656fe9dcb012103ad1d8e89212f0b92c74d23bb710c00662ad1470198ac48c43f7d6f93a2a2687392040000";
    let key = unhex("eb696a065ef48a2192da5b28b694f87544b30fae8327c4510137a922f32c6dcf");
    let k = SingleKey::decode(&c, &wif(&key, true, true)).unwrap();
    assert_eq!(
        hex(&k.public_key().to_bytes()),
        "03ad1d8e89212f0b92c74d23bb710c00662ad1470198ac48c43f7d6f93a2a26873"
    );
    let tx: Transaction = deserialize(&unhex(unsigned)).unwrap();
    let mut inner = bitcoin::Psbt::from_unsigned_tx(tx).unwrap();
    inner.inputs[0].witness_utxo = Some(TxOut {
        value: Amount::from_sat(1_000_000_000),
        script_pubkey: ScriptBuf::from_bytes(unhex(
            "a9144733f37cf4db86fbc2efed2500b4f4e49f31202387",
        )),
    });
    inner.inputs[0].redeem_script = Some(ScriptBuf::from_bytes(unhex(
        "001479091972186c449eb1ded22b78e40d009bdf0089",
    )));
    for nonce in [Nonce::LowR, Nonce::First] {
        let mut psbt = Psbt::from(inner.clone());
        let done = sweep::sign(&c, &mut psbt, &k, &ctx(Network::Mainnet), &[], nonce).unwrap();
        assert_eq!(done.signed_inputs.len(), 1);
        assert_eq!(
            hex(&done.signed_inputs[0].sig_bytes),
            "3044022047ac8e878352d3ebbde1c94ce3a10d057c24175747116f8288e5d794d12d482f0220217f36a485cae903c713331d877c1f64677e3622ad4010726870540656fe9dcb01"
        );
        let tx = finalize(&mut psbt).unwrap().expect("one input, signed");
        assert_eq!(hex(&serialize(&tx)), signed);
    }
}

/// A sweep PSBT whose inputs spend the key's legacy, native and nested
/// SegWit outputs, and one output of another key: the key's three are
/// signed, each keyed by the public key in the WIF's form, and the other
/// is left as it came.
fn three_and_one(k: &SingleKey, network: Network) -> (Psbt, Vec<Transaction>) {
    let other = master(ABANDON, network);
    let c = Compute::unchecked();
    let other_spk = other
        .account_xpub(&c, ScriptType::NativeSegwit, 0)
        .unwrap()
        .address(false, 0)
        .unwrap()
        .script_pubkey();
    let mut scripts = k.scripts();
    scripts.push(other_spk);
    let funding: Vec<Transaction> = scripts
        .iter()
        .enumerate()
        .map(|(i, s)| funding_tx(s.clone(), 50_000 + i as u64))
        .collect();
    let tx = Transaction {
        version: Version::TWO,
        lock_time: LockTime::ZERO,
        input: funding
            .iter()
            .map(|f| TxIn {
                previous_output: OutPoint::new(f.compute_txid(), 0),
                script_sig: ScriptBuf::new(),
                sequence: Sequence::MAX,
                witness: Witness::new(),
            })
            .collect(),
        output: vec![TxOut {
            value: Amount::from_sat(
                funding
                    .iter()
                    .map(|f| f.output[0].value.to_sat())
                    .sum::<u64>()
                    - 1_000,
            ),
            script_pubkey: scripts[scripts.len() - 1].clone(),
        }],
    };
    let mut inner = bitcoin::Psbt::from_unsigned_tx(tx).unwrap();
    for (i, f) in funding.iter().enumerate() {
        inner.inputs[i].non_witness_utxo = Some(f.clone());
        if !f.output[0].script_pubkey.is_p2pkh() {
            inner.inputs[i].witness_utxo = Some(f.output[0].clone());
        }
    }
    if k.compressed() {
        inner.inputs[2].redeem_script = Some(k.scripts()[1].clone());
    }
    (Psbt::from(inner), funding)
}

#[test]
fn a_sweep_signs_the_keys_inputs_alone_and_finishes_them() {
    let c = Compute::unchecked();
    let key = unhex("0000000000000000000000000000000000000000000000000000000000000001");
    let k = SingleKey::decode(&c, &wif(&key, true, false)).unwrap();
    let (mut psbt, _) = three_and_one(&k, Network::Regtest);
    assert_eq!(sweep::inputs_of(&psbt, &k), [0, 1, 2]);
    let done = sweep::sign(&c, &mut psbt, &k, &ctx(Network::Regtest), &[], Nonce::LowR).unwrap();
    assert_eq!(
        done.signed_inputs
            .iter()
            .map(|s| s.index)
            .collect::<Vec<_>>(),
        [0, 1, 2]
    );
    for i in 0..3 {
        assert!(
            psbt.inner().inputs[i]
                .partial_sigs
                .contains_key(&k.public_key()),
            "input {i}"
        );
    }
    assert!(psbt.inner().inputs[3].partial_sigs.is_empty());
    assert!(!done.complete, "the other key's input waits");
}

#[test]
fn an_uncompressed_key_sweeps_its_legacy_output_and_needs_its_previous_transaction() {
    let c = Compute::unchecked();
    let key = unhex("0000000000000000000000000000000000000000000000000000000000000002");
    let k = SingleKey::decode(&c, &wif(&key, false, false)).unwrap();
    assert_eq!(k.scripts().len(), 1);
    let (psbt, _) = three_and_one(&k, Network::Regtest);
    assert_eq!(sweep::inputs_of(&psbt, &k), [0]);
    let mut signed = psbt.clone();
    sweep::sign(
        &c,
        &mut signed,
        &k,
        &ctx(Network::Regtest),
        &[],
        Nonce::LowR,
    )
    .unwrap();
    let pk = k.public_key();
    assert!(!pk.compressed);
    assert!(signed.inner().inputs[0].partial_sigs.contains_key(&pk));
    // Without the transaction it spends, a legacy input is refused, and
    // nothing is signed.
    let mut bare = psbt.clone();
    bare.inner_mut().inputs[0].non_witness_utxo = None;
    bare.inner_mut().inputs[0].witness_utxo = Some(TxOut {
        value: Amount::from_sat(50_000),
        script_pubkey: k.scripts()[0].clone(),
    });
    let got = sweep::sign(&c, &mut bare, &k, &ctx(Network::Regtest), &[], Nonce::LowR);
    assert!(
        matches!(got, Err(Error::Blocked(_)) | Err(Error::Unsupported { .. })),
        "{got:?}"
    );
    assert!(
        bare.inner()
            .inputs
            .iter()
            .all(|i| i.partial_sigs.is_empty())
    );
}

/// A PSBT no input of which spends the key signs nothing.
#[test]
fn a_psbt_of_another_key_signs_nothing() {
    let c = Compute::unchecked();
    let key = unhex("0000000000000000000000000000000000000000000000000000000000000003");
    let k = SingleKey::decode(&c, &wif(&key, true, false)).unwrap();
    let mine = SingleKey::decode(&c, &wif(&unhex(&"07".repeat(32)), true, false)).unwrap();
    let (mut psbt, _) = three_and_one(&mine, Network::Regtest);
    let done = sweep::sign(&c, &mut psbt, &k, &ctx(Network::Regtest), &[], Nonce::LowR).unwrap();
    assert!(done.signed_inputs.is_empty());
    assert!(
        psbt.inner()
            .inputs
            .iter()
            .all(|i| i.partial_sigs.is_empty())
    );
}

/// A legacy P2SH input whose redeem script holds `OP_CODESEPARATOR`,
/// naming a held key, refuses the whole PSBT before anything is signed:
/// its other input, a plain one of the same key, is not signed either.
#[test]
fn a_legacy_input_with_a_code_separator_is_refused_and_nothing_is_signed() {
    let c = Compute::unchecked();
    let m = master(ABANDON, Network::Regtest);
    let account = m.account_xpub(&c, ScriptType::Legacy, 0).unwrap();
    let path: DerivationPath = "m/44'/1'/0'/0/0".parse().unwrap();
    let leaf = account
        .xpub()
        .derive_pub(
            &bitcoin::secp256k1::Secp256k1::verification_only(),
            &[
                bitcoin::bip32::ChildNumber::from_normal_idx(0).unwrap(),
                bitcoin::bip32::ChildNumber::from_normal_idx(0).unwrap(),
            ],
        )
        .unwrap()
        .public_key;
    let pkh = account.address(false, 0).unwrap().script_pubkey();
    let redeem = Builder::new()
        .push_key(&bitcoin::PublicKey::new(leaf))
        .push_opcode(OP_CHECKSIG)
        .push_opcode(OP_CODESEPARATOR)
        .into_script();
    let fund = [
        funding_tx(pkh.clone(), 40_000),
        funding_tx(ScriptBuf::new_p2sh(&redeem.script_hash()), 40_000),
    ];
    let tx = Transaction {
        version: Version::TWO,
        lock_time: LockTime::ZERO,
        input: fund
            .iter()
            .map(|f| TxIn {
                previous_output: OutPoint::new(f.compute_txid(), 0),
                script_sig: ScriptBuf::new(),
                sequence: Sequence::MAX,
                witness: Witness::new(),
            })
            .collect(),
        output: vec![TxOut {
            value: Amount::from_sat(79_000),
            script_pubkey: pkh,
        }],
    };
    let mut inner = bitcoin::Psbt::from_unsigned_tx(tx).unwrap();
    let origin = (BipFingerprint::from(m.fingerprint()), path);
    for (i, f) in fund.iter().enumerate() {
        inner.inputs[i].non_witness_utxo = Some(f.clone());
        inner.inputs[i]
            .bip32_derivation
            .insert(leaf, origin.clone());
    }
    inner.inputs[1].redeem_script = Some(redeem);
    let mut psbt = Psbt::from(inner);
    let refs = [KeyRef::from_master(&c, &m, 0).unwrap()];
    let context = Context {
        network: Network::Regtest,
        keys: &refs,
        wallets: &[],
        musig_session: None,
    };
    let got = sign(
        &c,
        &mut psbt,
        &[&m],
        &[m.fingerprint()],
        &context,
        &[],
        Nonce::LowR,
        faraday_psbt::Aux::Deterministic,
        &mut None,
        &faraday_crypto::Zeroizing::new([7u8; 32]),
    );
    assert!(
        matches!(
            got,
            Err(Error::Unsupported {
                input: 1,
                reason: Reason::CodeSeparator
            })
        ),
        "{got:?}"
    );
    assert!(
        psbt.inner()
            .inputs
            .iter()
            .all(|i| i.partial_sigs.is_empty())
    );
}
