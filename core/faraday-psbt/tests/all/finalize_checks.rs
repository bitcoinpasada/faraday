//! What finalizing refuses or leaves partial: a spent output a PSBT does
//! not carry, a stray script field, an invalid signature among the ones
//! on hand and an invalid taproot key-path signature.

use bitcoin::absolute::LockTime;
use bitcoin::ecdsa::Signature;
use bitcoin::hashes::Hash;
use bitcoin::opcodes::all::OP_CHECKMULTISIG;
use bitcoin::script::Builder;
use bitcoin::secp256k1::{Keypair, Message, Secp256k1, SecretKey};
use bitcoin::sighash::{EcdsaSighashType, Prevouts, SighashCache, TapSighashType};
use bitcoin::transaction::Version;
use bitcoin::{
    Amount, OutPoint, PublicKey, ScriptBuf, Sequence, Transaction, TxIn, TxOut, Txid, Witness,
    XOnlyPublicKey,
};
use faraday_crypto::work::Compute;
use faraday_psbt::{Error, Psbt, Reason, finalize, is_complete};

fn key(n: u8) -> (SecretKey, PublicKey) {
    let secret = SecretKey::from_slice(&[n; 32]).expect("a key");
    let public = PublicKey::new(secret.public_key(&Secp256k1::signing_only()));
    (secret, public)
}

fn spending(inputs: Vec<OutPoint>, out: ScriptBuf) -> Transaction {
    Transaction {
        version: Version::TWO,
        lock_time: LockTime::ZERO,
        input: inputs
            .into_iter()
            .map(|previous_output| TxIn {
                previous_output,
                script_sig: ScriptBuf::new(),
                sequence: Sequence::ENABLE_RBF_NO_LOCKTIME,
                witness: Witness::new(),
            })
            .collect(),
        output: vec![TxOut {
            value: Amount::from_sat(1_000),
            script_pubkey: out,
        }],
    }
}

fn parse(inner: &bitcoin::Psbt) -> Psbt {
    Psbt::parse_bytes(&Compute::unchecked(), &inner.serialize()).expect("a PSBT")
}

fn refused(psbt: &mut Psbt, want: Reason) {
    assert!(matches!(
        finalize(psbt),
        Err(Error::Finalize { reason, .. }) if reason == want
    ));
}

fn multi(m: i64, keys: &[PublicKey]) -> ScriptBuf {
    let mut b = Builder::new().push_int(m);
    for k in keys {
        b = b.push_key(k);
    }
    b.push_int(keys.len() as i64)
        .push_opcode(OP_CHECKMULTISIG)
        .into_script()
}

/// Two inputs: the first a signed p2wpkh, the second names a previous
/// transaction with one output while the transaction spends output 5.
/// This is refused, before any input is finalized, and does not panic.
#[test]
fn a_previous_transaction_without_the_spent_output_is_refused() {
    let secp = Secp256k1::signing_only();
    let (sk, pk) = key(1);
    let spk = ScriptBuf::new_p2wpkh(&pk.wpubkey_hash().expect("compressed"));
    let prev = spending(vec![OutPoint::null()], spk.clone());
    let tx = spending(
        vec![
            OutPoint::new(Txid::from_byte_array([9; 32]), 0),
            OutPoint::new(prev.compute_txid(), 5),
        ],
        spk.clone(),
    );
    let utxo = TxOut {
        value: Amount::from_sat(5_000),
        script_pubkey: spk.clone(),
    };
    let hash = SighashCache::new(&tx)
        .p2wpkh_signature_hash(0, &spk, utxo.value, EcdsaSighashType::All)
        .expect("one input");
    let sig = secp.sign_ecdsa(&Message::from_digest(hash.to_byte_array()), &sk);
    let mut inner = bitcoin::Psbt::from_unsigned_tx(tx).expect("unsigned");
    inner.inputs[0].witness_utxo = Some(utxo);
    inner.inputs[0]
        .partial_sigs
        .insert(pk, Signature::sighash_all(sig));
    inner.inputs[1].non_witness_utxo = Some(prev);
    let mut psbt = parse(&inner);
    assert!(!is_complete(&psbt));
    refused(&mut psbt, Reason::PrevTxNoOutput);
}

/// A complete `wsh` multisig that also states a redeem script, and an
/// `sh` multisig that also states a witness script, are refused.
#[test]
fn a_multisig_with_the_other_script_field_is_refused() {
    let secp = Secp256k1::signing_only();
    let (sa, a) = key(1);
    let (sb, b) = key(2);
    let script = multi(2, &[a, b]);
    for wsh in [true, false] {
        let spk = if wsh {
            ScriptBuf::new_p2wsh(&script.wscript_hash())
        } else {
            ScriptBuf::new_p2sh(&script.script_hash())
        };
        let utxo = TxOut {
            value: Amount::from_sat(5_000),
            script_pubkey: spk,
        };
        let prev = Transaction {
            output: vec![utxo.clone()],
            ..spending(vec![OutPoint::null()], ScriptBuf::new())
        };
        let tx = spending(
            vec![OutPoint::new(prev.compute_txid(), 0)],
            ScriptBuf::new(),
        );
        let hash = if wsh {
            SighashCache::new(&tx)
                .p2wsh_signature_hash(0, &script, utxo.value, EcdsaSighashType::All)
                .expect("one input")
                .to_byte_array()
        } else {
            SighashCache::new(&tx)
                .legacy_signature_hash(0, &script, EcdsaSighashType::All as u32)
                .expect("one input")
                .to_byte_array()
        };
        let msg = Message::from_digest(hash);
        let mut inner = bitcoin::Psbt::from_unsigned_tx(tx).expect("unsigned");
        inner.inputs[0].non_witness_utxo = Some(prev);
        for (s, p) in [(&sa, a), (&sb, b)] {
            inner.inputs[0]
                .partial_sigs
                .insert(p, Signature::sighash_all(secp.sign_ecdsa(&msg, s)));
        }
        if wsh {
            inner.inputs[0].witness_script = Some(script.clone());
        } else {
            inner.inputs[0].redeem_script = Some(script.clone());
        }
        assert!(finalize(&mut parse(&inner)).expect("finalizes").is_some());
        let mut stray = inner.clone();
        if wsh {
            stray.inputs[0].redeem_script = Some(script.clone());
        } else {
            stray.inputs[0].witness_script = Some(script.clone());
        }
        refused(
            &mut parse(&stray),
            if wsh {
                Reason::StrayRedeemScript
            } else {
                Reason::StrayWitnessScript
            },
        );
    }
}

/// A 2-of-3 `wsh` where A and B signed validly and C's entry, shorter
/// than theirs, signs another message: A and B finalize, the first two
/// signers in script order, and C's invalid entry is left out, as Core
/// does.
#[test]
fn an_invalid_signature_beyond_the_first_signers_is_left_out() {
    let secp = Secp256k1::signing_only();
    let (sc, c) = key(250);
    let signers: Vec<(SecretKey, PublicKey)> = (1..=200).map(key).collect();
    let tx_for = |a: PublicKey, b: PublicKey| {
        let script = multi(2, &[a, b, c]);
        let utxo = TxOut {
            value: Amount::from_sat(5_000),
            script_pubkey: ScriptBuf::new_p2wsh(&script.wscript_hash()),
        };
        let tx = spending(
            vec![OutPoint::new(Txid::from_byte_array([7; 32]), 0)],
            ScriptBuf::new(),
        );
        let hash = SighashCache::new(&tx)
            .p2wsh_signature_hash(0, &script, utxo.value, EcdsaSighashType::All)
            .expect("one input");
        (script, utxo, tx, Message::from_digest(hash.to_byte_array()))
    };
    // Keys whose signatures are 72 bytes, so C's can be shorter.
    let (a, b) = {
        let mut long = signers.iter().filter(|(s, p)| {
            *p != c && {
                let (_, _, _, m) = tx_for(*p, *p);
                Signature::sighash_all(secp.sign_ecdsa(&m, s))
                    .to_vec()
                    .len()
                    == 72
            }
        });
        (
            *long.next().expect("a key"),
            *long.next().expect("a second key"),
        )
    };
    let (script, utxo, tx, msg) = tx_for(a.1, b.1);
    let sig_a = Signature::sighash_all(secp.sign_ecdsa(&msg, &a.0));
    let sig_b = Signature::sighash_all(secp.sign_ecdsa(&msg, &b.0));
    let other = Message::from_digest([5; 32]);
    let sig_c = Signature::sighash_all(secp.sign_ecdsa_low_r(&other, &sc));
    assert!(sig_c.to_vec().len() < 72);
    let mut inner = bitcoin::Psbt::from_unsigned_tx(tx).expect("unsigned");
    inner.inputs[0].witness_utxo = Some(utxo);
    inner.inputs[0].witness_script = Some(script.clone());
    inner.inputs[0].partial_sigs = [(a.1, sig_a), (b.1, sig_b), (c, sig_c)]
        .into_iter()
        .collect();
    let mut psbt = parse(&inner);
    assert!(is_complete(&psbt));
    let tx = finalize(&mut psbt).expect("finalizes").expect("final");
    assert_eq!(
        tx.input[0].witness.to_vec(),
        vec![
            Vec::new(),
            sig_a.to_vec(),
            sig_b.to_vec(),
            script.to_bytes()
        ]
    );
}

/// A taproot key-path signature is finalized when it signs the input and
/// left partial when it signs another message.
#[test]
fn a_taproot_key_path_signature_is_checked() {
    let secp = Secp256k1::new();
    let keypair = Keypair::from_seckey_slice(&secp, &[1; 32]).expect("a key");
    let (output_key, _) = XOnlyPublicKey::from_keypair(&keypair);
    let utxo = TxOut {
        value: Amount::from_sat(5_000),
        script_pubkey: ScriptBuf::new_p2tr_tweaked(
            bitcoin::key::TweakedPublicKey::dangerous_assume_tweaked(output_key),
        ),
    };
    let tx = spending(
        vec![OutPoint::new(Txid::from_byte_array([7; 32]), 0)],
        ScriptBuf::new(),
    );
    let hash = SighashCache::new(&tx)
        .taproot_key_spend_signature_hash(0, &Prevouts::All(&[&utxo]), TapSighashType::Default)
        .expect("one input");
    let sign = |m: [u8; 32]| bitcoin::taproot::Signature {
        signature: secp.sign_schnorr_no_aux_rand(&Message::from_digest(m), &keypair),
        sighash_type: TapSighashType::Default,
    };
    let finalize_with = |sig| {
        let mut inner = bitcoin::Psbt::from_unsigned_tx(tx.clone()).expect("unsigned");
        inner.inputs[0].witness_utxo = Some(utxo.clone());
        inner.inputs[0].tap_key_sig = Some(sig);
        finalize(&mut parse(&inner)).expect("not refused")
    };
    assert!(finalize_with(sign(hash.to_byte_array())).is_some());
    assert!(finalize_with(sign([5; 32])).is_none());
}
