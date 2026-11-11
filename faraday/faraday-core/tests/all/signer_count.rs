//! The Spend screen's signed count and its list of keys still to sign
//! (`Spend::signers_by`): a key counts as having signed only when a
//! signature that verifies is under a key of the script being spent, made
//! over every input it is asked on. Everything in the PSBT is the online
//! side's word, so a key origin it files under a key of its own choosing,
//! or beside a witness that confirms nowhere, counts for nothing.
//!
//! The signer is rust-bitcoin's secp256k1 over rust-bitcoin's sighash.

use faraday_bip::bitcoin::bip32::{DerivationPath, Fingerprint as Fp};
use faraday_bip::bitcoin::key::TapTweak;
use faraday_bip::bitcoin::opcodes::all::{OP_CHECKSIG, OP_CHECKSIGADD, OP_NUMEQUAL};
use faraday_bip::bitcoin::secp256k1::{Keypair, Message, Secp256k1, SecretKey};
use faraday_bip::bitcoin::sighash::{EcdsaSighashType, Prevouts, SighashCache, TapSighashType};
use faraday_bip::bitcoin::taproot::{LeafVersion, TapLeafHash, TaprootBuilder};
use faraday_bip::bitcoin::{
    Amount, OutPoint, PublicKey, ScriptBuf, Sequence, Transaction, TxIn, TxOut, Witness,
    XOnlyPublicKey, absolute::LockTime, ecdsa, hashes::Hash, script::Builder, transaction::Version,
};
use faraday_bip::keys::{Fingerprint, Network};
use faraday_core::wallet::{PsbtSeen, Session, Spend};
use faraday_psbt::Psbt;
use faraday_psbt::verify::witness_mismatch;
use std::str::FromStr;

type Inner = faraday_bip::bitcoin::Psbt;

struct Key {
    pk: PublicKey,
    pair: Keypair,
    x: XOnlyPublicKey,
}

fn key(n: u8) -> Key {
    let secp = Secp256k1::new();
    let pair = Keypair::from_secret_key(&secp, &SecretKey::from_slice(&[n; 32]).unwrap());
    Key {
        pk: PublicKey::new(pair.public_key()),
        x: pair.x_only_public_key().0,
        pair,
    }
}

fn fp(n: u8) -> Fp {
    Fp::from([n; 4])
}

fn path() -> DerivationPath {
    DerivationPath::from_str("m/48'/1'/0'/2'/0/0").unwrap()
}

/// A PSBT spending one output per `spks`, each worth 50,000.
fn psbt_of(spks: &[ScriptBuf]) -> (Inner, Vec<TxOut>) {
    let prevs: Vec<TxOut> = spks
        .iter()
        .map(|s| TxOut {
            value: Amount::from_sat(50_000),
            script_pubkey: s.clone(),
        })
        .collect();
    let tx = Transaction {
        version: Version::TWO,
        lock_time: LockTime::ZERO,
        input: (0..spks.len())
            .map(|i| TxIn {
                previous_output: OutPoint::new(
                    faraday_bip::bitcoin::Txid::from_byte_array([1; 32]),
                    i as u32,
                ),
                script_sig: ScriptBuf::new(),
                sequence: Sequence::MAX,
                witness: Witness::new(),
            })
            .collect(),
        output: vec![TxOut {
            value: Amount::from_sat(10_000),
            script_pubkey: ScriptBuf::new_p2tr(&Secp256k1::new(), key(99).x, None),
        }],
    };
    let mut inner = Inner::from_unsigned_tx(tx).unwrap();
    for (i, p) in prevs.iter().enumerate() {
        inner.inputs[i].witness_utxo = Some(p.clone());
    }
    (inner, prevs)
}

fn count(inner: &Inner) -> Vec<Fingerprint> {
    let c = faraday_core::testkit::compute();
    let session = Session::on(Network::Testnet);
    let mut got = Spend::new(Psbt::from(inner.clone()), "t").signers_by(&c, &session, None);
    got.sort_by_key(|f| format!("{f}"));
    got
}

fn fps(ns: &[u8]) -> Vec<Fingerprint> {
    let mut v: Vec<Fingerprint> = ns.iter().map(|n| Fingerprint::from(fp(*n))).collect();
    v.sort_by_key(|f| format!("{f}"));
    v
}

fn wsh_multisig(keys: &[&Key], m: i64) -> ScriptBuf {
    let mut b = Builder::new().push_int(m);
    for k in keys {
        b = b.push_key(&k.pk);
    }
    b.push_int(keys.len() as i64)
        .push_opcode(faraday_bip::bitcoin::opcodes::all::OP_CHECKMULTISIG)
        .into_script()
}

fn ecdsa_wsh(inner: &Inner, i: usize, script: &ScriptBuf, k: &Key) -> ecdsa::Signature {
    let h = SighashCache::new(&inner.unsigned_tx)
        .p2wsh_signature_hash(i, script, Amount::from_sat(50_000), EcdsaSighashType::All)
        .unwrap();
    ecdsa::Signature::sighash_all(Secp256k1::new().sign_ecdsa(
        &Message::from_digest(h.to_byte_array()),
        &k.pair.secret_key(),
    ))
}

fn ecdsa_wpkh(inner: &Inner, i: usize, k: &Key) -> ecdsa::Signature {
    let spk = ScriptBuf::new_p2wpkh(&k.pk.wpubkey_hash().unwrap());
    let h = SighashCache::new(&inner.unsigned_tx)
        .p2wpkh_signature_hash(i, &spk, Amount::from_sat(50_000), EcdsaSighashType::All)
        .unwrap();
    ecdsa::Signature::sighash_all(Secp256k1::new().sign_ecdsa(
        &Message::from_digest(h.to_byte_array()),
        &k.pair.secret_key(),
    ))
}

fn schnorr(msg: [u8; 32], pair: &Keypair) -> faraday_bip::bitcoin::taproot::Signature {
    faraday_bip::bitcoin::taproot::Signature {
        signature: Secp256k1::new().sign_schnorr_no_aux_rand(&Message::from_digest(msg), pair),
        sighash_type: TapSighashType::Default,
    }
}

fn leaf_sig(
    inner: &Inner,
    prevs: &[TxOut],
    i: usize,
    leaf: TapLeafHash,
    k: &Key,
) -> faraday_bip::bitcoin::taproot::Signature {
    let h = SighashCache::new(&inner.unsigned_tx)
        .taproot_script_spend_signature_hash(
            i,
            &Prevouts::All(prevs),
            leaf,
            TapSighashType::Default,
        )
        .unwrap();
    schnorr(h.to_byte_array(), &k.pair)
}

fn key_sig(
    inner: &Inner,
    prevs: &[TxOut],
    i: usize,
    tweaked: &Keypair,
) -> faraday_bip::bitcoin::taproot::Signature {
    let h = SighashCache::new(&inner.unsigned_tx)
        .taproot_key_spend_signature_hash(i, &Prevouts::All(prevs), TapSighashType::Default)
        .unwrap();
    schnorr(h.to_byte_array(), tweaked)
}

/// `k1 CHECKSIG k2 CHECKSIGADD … m NUMEQUAL`.
fn multi_a(keys: &[&Key], m: i64) -> ScriptBuf {
    let mut b = Builder::new();
    for (n, k) in keys.iter().enumerate() {
        b = b
            .push_x_only_key(&k.x)
            .push_opcode(if n == 0 { OP_CHECKSIG } else { OP_CHECKSIGADD });
    }
    b.push_int(m).push_opcode(OP_NUMEQUAL).into_script()
}

/// A taproot input spending the tree of one leaf `script` under internal
/// key `internal`, with the origins `origins` filed for the leaf (each
/// under its own key) and the internal key's under `fp(internal_fp)`.
struct Tap {
    inner: Inner,
    prevs: Vec<TxOut>,
    leaf: TapLeafHash,
    script: ScriptBuf,
    control: Vec<u8>,
}

fn tap_leaf(internal: &Key, internal_fp: u8, script: ScriptBuf, origins: &[(&Key, u8)]) -> Tap {
    let secp = Secp256k1::new();
    let info = TaprootBuilder::new()
        .add_leaf(0, script.clone())
        .unwrap()
        .finalize(&secp, internal.x)
        .unwrap();
    let (mut inner, prevs) = psbt_of(&[ScriptBuf::new_p2tr_tweaked(info.output_key())]);
    let leaf = TapLeafHash::from_script(&script, LeafVersion::TapScript);
    let cb = info
        .control_block(&(script.clone(), LeafVersion::TapScript))
        .unwrap();
    let input = &mut inner.inputs[0];
    input.tap_internal_key = Some(internal.x);
    input.tap_merkle_root = info.merkle_root();
    input
        .tap_scripts
        .insert(cb.clone(), (script.clone(), LeafVersion::TapScript));
    input
        .tap_key_origins
        .insert(internal.x, (vec![], (fp(internal_fp), path())));
    for (k, n) in origins {
        input
            .tap_key_origins
            .insert(k.x, (vec![leaf], (fp(*n), path())));
    }
    Tap {
        inner,
        prevs,
        leaf,
        script,
        control: cb.serialize(),
    }
}

// True cases, which stay as they were.

#[test]
fn a_single_key_p2wpkh_counts_its_master_once_it_has_signed() {
    let k = key(1);
    let (mut inner, _) = psbt_of(&[ScriptBuf::new_p2wpkh(&k.pk.wpubkey_hash().unwrap())]);
    inner.inputs[0]
        .bip32_derivation
        .insert(k.pk.inner, (fp(1), path()));
    assert_eq!(count(&inner), fps(&[]));
    let sig = ecdsa_wpkh(&inner, 0, &k);
    inner.inputs[0].partial_sigs.insert(k.pk, sig);
    assert_eq!(count(&inner), fps(&[1]));
}

#[test]
fn a_two_of_three_p2wsh_counts_one_then_two_masters() {
    let (a, b, c) = (key(1), key(2), key(3));
    let script = wsh_multisig(&[&a, &b, &c], 2);
    let (mut inner, _) = psbt_of(&[ScriptBuf::new_p2wsh(&script.wscript_hash())]);
    inner.inputs[0].witness_script = Some(script.clone());
    for (k, n) in [(&a, 1), (&b, 2), (&c, 3)] {
        inner.inputs[0]
            .bip32_derivation
            .insert(k.pk.inner, (fp(n), path()));
    }
    let sig = ecdsa_wsh(&inner, 0, &script, &a);
    inner.inputs[0].partial_sigs.insert(a.pk, sig);
    assert_eq!(count(&inner), fps(&[1]));
    let sig = ecdsa_wsh(&inner, 0, &script, &c);
    inner.inputs[0].partial_sigs.insert(c.pk, sig);
    assert_eq!(count(&inner), fps(&[1, 3]));
}

#[test]
fn a_two_of_three_taproot_script_path_counts_one_then_two_masters() {
    let (i, a, b, c) = (key(7), key(1), key(2), key(3));
    let mut t = tap_leaf(
        &i,
        70,
        multi_a(&[&a, &b, &c], 2),
        &[(&a, 1), (&b, 2), (&c, 3)],
    );
    let s = leaf_sig(&t.inner, &t.prevs, 0, t.leaf, &b);
    t.inner.inputs[0].tap_script_sigs.insert((b.x, t.leaf), s);
    assert_eq!(count(&t.inner), fps(&[2]));
    let s = leaf_sig(&t.inner, &t.prevs, 0, t.leaf, &c);
    t.inner.inputs[0].tap_script_sigs.insert((c.x, t.leaf), s);
    assert_eq!(count(&t.inner), fps(&[2, 3]));
}

/// A key-path spend of `internal` tweaked with `root`, signed.
fn key_path(root: Option<&TapLeafHash>) -> (Inner, Vec<TxOut>, Keypair) {
    let secp = Secp256k1::new();
    let i = key(7);
    let merkle = root.map(|_| {
        TaprootBuilder::new()
            .add_leaf(0, ScriptBuf::from_bytes(vec![0x51]))
            .unwrap()
            .finalize(&secp, i.x)
            .unwrap()
            .merkle_root()
            .unwrap()
    });
    let tweaked = i.pair.tap_tweak(&secp, merkle);
    let (mut inner, prevs) = psbt_of(&[ScriptBuf::new_p2tr_tweaked(tweaked.public_parts().0)]);
    let input = &mut inner.inputs[0];
    input.tap_internal_key = Some(i.x);
    input.tap_merkle_root = merkle;
    input.tap_key_origins.insert(i.x, (vec![], (fp(7), path())));
    (inner, prevs, tweaked.to_keypair())
}

#[test]
fn a_taproot_key_path_spend_counts_the_internal_keys_master_with_and_without_a_merkle_root() {
    for root in [None, Some(&TapLeafHash::all_zeros())] {
        let (mut inner, prevs, tweaked) = key_path(root);
        assert_eq!(count(&inner), fps(&[]));
        inner.inputs[0].tap_key_sig = Some(key_sig(&inner, &prevs, 0, &tweaked));
        assert_eq!(count(&inner), fps(&[7]), "merkle root {}", root.is_some());
    }
}

// The reported bugs.

#[test]
fn a_leaf_key_with_no_origin_does_not_take_the_internal_keys_master() {
    // The online side's own key in a pushed-key leaf, filed with no
    // origin, beside the internal key's real one.
    let (i, evil) = (key(7), key(5));
    let mut t = tap_leaf(&i, 70, multi_a(&[&evil], 1), &[]);
    let s = leaf_sig(&t.inner, &t.prevs, 0, t.leaf, &evil);
    t.inner.inputs[0]
        .tap_script_sigs
        .insert((evil.x, t.leaf), s);
    assert_eq!(count(&t.inner), fps(&[]));
}

#[test]
fn a_key_path_row_under_an_output_key_that_is_not_the_tweaked_internal_key_has_no_master() {
    let (mut inner, prevs, _) = key_path(None);
    // The output pays a key the online side holds; the internal key and
    // its origin are left as they were.
    let evil = key(5);
    let spk = ScriptBuf::new_p2tr(&Secp256k1::new(), evil.x, None);
    let tweaked = evil.pair.tap_tweak(&Secp256k1::new(), None).to_keypair();
    let mut prevs = prevs;
    prevs[0].script_pubkey = spk;
    inner.inputs[0].witness_utxo = Some(prevs[0].clone());
    inner.inputs[0].tap_key_sig = Some(key_sig(&inner, &prevs, 0, &tweaked));
    assert_eq!(count(&inner), fps(&[]));
}

#[test]
fn an_origin_filed_under_a_key_that_is_not_in_the_p2wsh_script_counts_for_nobody() {
    let (a, b, evil) = (key(1), key(2), key(5));
    let script = wsh_multisig(&[&a, &b], 1);
    let (mut inner, _) = psbt_of(&[ScriptBuf::new_p2wsh(&script.wscript_hash())]);
    inner.inputs[0].witness_script = Some(script.clone());
    inner.inputs[0]
        .bip32_derivation
        .insert(evil.pk.inner, (fp(9), path()));
    let sig = ecdsa_wsh(&inner, 0, &script, &evil);
    inner.inputs[0].partial_sigs.insert(evil.pk, sig);
    assert_eq!(count(&inner), fps(&[]));
}

#[test]
fn an_origin_filed_under_a_key_that_is_not_in_the_taproot_leaf_counts_for_nobody() {
    let (i, a, b, evil) = (key(7), key(1), key(2), key(5));
    let mut t = tap_leaf(&i, 70, multi_a(&[&a, &b], 1), &[(&a, 1), (&b, 2)]);
    t.inner.inputs[0]
        .tap_key_origins
        .insert(evil.x, (vec![t.leaf], (fp(9), path())));
    let s = leaf_sig(&t.inner, &t.prevs, 0, t.leaf, &evil);
    t.inner.inputs[0]
        .tap_script_sigs
        .insert((evil.x, t.leaf), s);
    assert_eq!(count(&t.inner), fps(&[]));
}

#[test]
fn a_partial_signature_beside_a_failing_final_witness_is_not_counted() {
    let (i, a) = (key(7), key(1));
    let mut t = tap_leaf(&i, 70, multi_a(&[&a], 1), &[(&a, 1)]);
    let s = leaf_sig(&t.inner, &t.prevs, 0, t.leaf, &a);
    t.inner.inputs[0].tap_script_sigs.insert((a.x, t.leaf), s);
    assert_eq!(count(&t.inner), fps(&[1]), "before it is final");
    let mut control = t.control.clone();
    control[2] ^= 1;
    t.inner.inputs[0].final_script_witness = Some(Witness::from_slice(&[
        s.to_vec(),
        t.script.to_bytes(),
        control,
    ]));
    assert!(witness_mismatch(&t.inner, 0));
    assert_eq!(count(&t.inner), fps(&[]));
}

#[test]
fn a_final_taproot_witness_with_no_signature_signs_nothing() {
    let (mut inner, prevs, tweaked) = key_path(None);
    let good = key_sig(&inner, &prevs, 0, &tweaked);
    for witness in [Witness::new(), Witness::from_slice(&[vec![1u8; 10]])] {
        inner.inputs[0].final_script_witness = Some(witness);
        let seen = PsbtSeen::of(&Psbt::from(inner.clone()));
        assert!(!seen.signed, "{:?}", inner.inputs[0].final_script_witness);
        assert!(witness_mismatch(&inner, 0));
        let insp = Session::on(Network::Testnet).inspect(
            &faraday_core::testkit::compute(),
            &Psbt::from(inner.clone()),
        );
        assert!(
            insp.warnings
                .iter()
                .any(|w| matches!(w.kind, faraday_psbt::WarningKind::WitnessMismatch { .. }))
        );
    }
    inner.inputs[0].final_script_witness = Some(Witness::from_slice(&[good.to_vec()]));
    assert!(PsbtSeen::of(&Psbt::from(inner.clone())).signed);
    assert!(!witness_mismatch(&inner, 0));
}

#[test]
fn a_master_seen_between_two_others_counts_once() {
    // Three keys of one 2-of-3 whose masters in key order are A, B, A.
    let mut keys = [key(1), key(2), key(3)];
    keys.sort_by_key(|k| k.pk.inner.serialize());
    let order = [1u8, 2, 1];
    let refs: Vec<&Key> = keys.iter().collect();
    let script = wsh_multisig(&refs, 2);
    let (mut inner, _) = psbt_of(&[ScriptBuf::new_p2wsh(&script.wscript_hash())]);
    inner.inputs[0].witness_script = Some(script.clone());
    for (k, n) in keys.iter().zip(order) {
        inner.inputs[0]
            .bip32_derivation
            .insert(k.pk.inner, (fp(n), path()));
    }
    for k in &keys {
        let sig = ecdsa_wsh(&inner, 0, &script, k);
        inner.inputs[0].partial_sigs.insert(k.pk, sig);
    }
    assert_eq!(count(&inner), fps(&[1, 2]));
}

#[test]
fn a_key_counts_only_if_it_signed_every_input_it_is_asked_on() {
    let (a, b) = (key(1), key(2));
    let script = wsh_multisig(&[&a, &b], 2);
    let spk = ScriptBuf::new_p2wsh(&script.wscript_hash());
    let (mut inner, _) = psbt_of(&[spk.clone(), spk]);
    for i in 0..2 {
        inner.inputs[i].witness_script = Some(script.clone());
        inner.inputs[i]
            .bip32_derivation
            .insert(a.pk.inner, (fp(1), path()));
        inner.inputs[i]
            .bip32_derivation
            .insert(b.pk.inner, (fp(2), path()));
    }
    for (i, k) in [(0, &a), (1, &a), (0, &b)] {
        let sig = ecdsa_wsh(&inner, i, &script, k);
        inner.inputs[i].partial_sigs.insert(k.pk, sig);
    }
    assert_eq!(count(&inner), fps(&[1]));
    let sig = ecdsa_wsh(&inner, 1, &script, &b);
    inner.inputs[1].partial_sigs.insert(b.pk, sig);
    assert_eq!(count(&inner), fps(&[1, 2]));
}

// Inputs with different owners.

fn state_in(session: &Session, inner: &Inner) -> (Vec<Fingerprint>, Vec<Fingerprint>) {
    let c = faraday_core::testkit::compute();
    let got = Spend::new(Psbt::from(inner.clone()), "t").signing(&c, session, None, None);
    let (mut s, mut t) = (got.signers, got.to_sign);
    s.sort_by_key(|f| format!("{f}"));
    t.sort_by_key(|f| format!("{f}"));
    (s, t)
}

/// Signs with every key the session holds, past the dangers a hold takes.
fn sign_held(session: &Session, p: &mut Psbt) -> Result<Vec<Fingerprint>, String> {
    let c = faraday_core::testkit::compute();
    let held = session.inspect(&c, p).holdable();
    session
        .sign_with(
            &c,
            p,
            &zeroize::Zeroizing::new([0; 32]),
            &mut None,
            None,
            &held,
        )
        .map(|s| s.by)
}

/// A session holding test seed 0, and a p2wpkh PSBT whose input 0 is that
/// seed's key and input 1 is `other`'s (master `fp(2)`).
fn two_owners(other: &Key) -> (Session, Fingerprint, Inner) {
    use faraday_core::testkit::{TEST_SEEDS, test_words};
    let c = faraday_core::testkit::compute();
    let mut session = faraday_core::testkit::session();
    let held = session
        .keys
        .add_words(&c, &test_words(TEST_SEEDS[0].0), TEST_SEEDS[0].1, None)
        .unwrap();
    let p = DerivationPath::from_str("m/84'/1'/0'/0/0").unwrap();
    let x = session
        .keys
        .xpub_at(&c, session.keys.position(held).unwrap(), &p)
        .unwrap();
    let mine = PublicKey::new(x.public_key);
    let (mut inner, _) = psbt_of(&[
        ScriptBuf::new_p2wpkh(&mine.wpubkey_hash().unwrap()),
        ScriptBuf::new_p2wpkh(&other.pk.wpubkey_hash().unwrap()),
    ]);
    // Each amount is vouched for by the whole transaction it comes from.
    for i in 0..2 {
        let prev = Transaction {
            version: Version::TWO,
            lock_time: LockTime::from_height(i as u32 + 1).unwrap(),
            input: vec![],
            output: vec![inner.inputs[i].witness_utxo.clone().unwrap()],
        };
        inner.unsigned_tx.input[i].previous_output = OutPoint::new(prev.compute_txid(), 0);
        inner.inputs[i].non_witness_utxo = Some(prev);
    }
    inner.unsigned_tx.output[0].value = Amount::from_sat(99_000);
    inner.inputs[0]
        .bip32_derivation
        .insert(mine.inner, (Fp::from(held.0), p.clone()));
    inner.inputs[1]
        .bip32_derivation
        .insert(other.pk.inner, (fp(2), p));
    (session, held, inner)
}

#[test]
fn inputs_of_two_owners_offer_sign_only_to_a_master_with_an_input_still_open() {
    let b = key(2);
    let (session, a, mut inner) = two_owners(&b);
    let b_fp = Fingerprint::from(fp(2));
    // Nothing signed: this device's master has input 0 open.
    assert_eq!(
        state_in(&session, &inner),
        (vec![], {
            let mut v = vec![a, b_fp];
            v.sort_by_key(|f| format!("{f}"));
            v
        })
    );
    // This device signs what it holds.
    let mut p = Psbt::from(inner.clone());
    let by = sign_held(&session, &mut p).unwrap();
    assert_eq!(by, vec![a]);
    inner = p.inner().clone();
    // The count stays low (no master signed every open input), and what is
    // left to sign is the other owner's alone: nothing for this device.
    assert_eq!(state_in(&session, &inner), (vec![], vec![b_fp]));
    // A second sign adds no signature: the master is not offered Sign again
    // (it is in no input's list of keys still to sign).
    let mut again = Psbt::from(inner.clone());
    assert_eq!(sign_held(&session, &mut again).unwrap(), vec![a]);
    assert_eq!(again.inner().inputs, inner.inputs);
    // The other owner signs its own input: nobody is left to sign.
    let sig = ecdsa_wpkh(&inner, 1, &b);
    inner.inputs[1].partial_sigs.insert(b.pk, sig);
    assert_eq!(state_in(&session, &inner), (vec![], vec![]));
    let mut spend = Spend::new(Psbt::from(inner), "t");
    assert!(spend.finish().is_ok());
}

#[test]
fn a_final_witness_with_too_few_signatures_counts_no_signer() {
    let (a, b, d) = (key(1), key(2), key(3));
    let script = wsh_multisig(&[&a, &b, &d], 2);
    let (mut inner, _) = psbt_of(&[ScriptBuf::new_p2wsh(&script.wscript_hash())]);
    let sig = ecdsa_wsh(&inner, 0, &script, &a);
    inner.inputs[0].final_script_witness = Some(Witness::from_slice(&[
        vec![],
        sig.to_vec(),
        script.to_bytes(),
    ]));
    for (n, k) in [&a, &b, &d].into_iter().enumerate() {
        inner.inputs[0]
            .bip32_derivation
            .insert(k.pk.inner, (fp(n as u8 + 1), path()));
    }
    // The signature is a true one, and the witness is still refused.
    assert!(witness_mismatch(&inner, 0));
    assert_eq!(count(&inner), fps(&[]));
}

// What `finish` refuses of a taproot input already final.

fn finish_of(inner: &Inner) -> Result<(), String> {
    Spend::new(Psbt::from(inner.clone()), "t").finish()
}

fn with_witness(inner: &Inner, elements: &[Vec<u8>]) -> Inner {
    let mut copy = inner.clone();
    copy.inputs[0].final_script_witness = Some(Witness::from_slice(elements));
    copy
}

#[test]
fn finish_refuses_a_final_key_path_signature_that_fails() {
    let (inner, prevs, tweaked) = key_path(None);
    let sig = key_sig(&inner, &prevs, 0, &tweaked).to_vec();
    assert_eq!(finish_of(&with_witness(&inner, &[sig])), Ok(()));
    assert!(finish_of(&with_witness(&inner, &[vec![3u8; 64]])).is_err());
}

#[test]
fn finish_refuses_a_forged_script_path_signature() {
    let (a, internal) = (key(1), key(7));
    let t = tap_leaf(&internal, 7, multi_a(&[&a], 1), &[(&a, 1)]);
    let sc = t.script.to_bytes();
    let good = leaf_sig(&t.inner, &t.prevs, 0, t.leaf, &a).to_vec();
    let ok = with_witness(&t.inner, &[good.clone(), sc.clone(), t.control.clone()]);
    assert_eq!(finish_of(&ok), Ok(()));
    let mut forged = good;
    forged[10] ^= 1;
    let bad = with_witness(&t.inner, &[forged, sc, t.control.clone()]);
    assert!(finish_of(&bad).is_err());
}

#[test]
fn finish_does_not_refuse_a_64_byte_data_element_on_a_leaf_that_is_no_miniscript() {
    let internal = key(7);
    let script = Builder::new()
        .push_opcode(faraday_bip::bitcoin::opcodes::all::OP_DROP)
        .push_int(1)
        .into_script();
    let t = tap_leaf(&internal, 7, script, &[]);
    let w = with_witness(
        &t.inner,
        &[vec![3u8; 64], t.script.to_bytes(), t.control.clone()],
    );
    assert_eq!(finish_of(&w), Ok(()));
}

// With a wallet loaded, and the shapes the kits make.

/// The count and keys still to sign of `inner` under `session`.
fn state_with(session: &Session, inner: &Inner) -> Vec<Fingerprint> {
    state_in(session, inner).0
}

/// A session holding test seed `n`, with `kit`'s wallet loaded when given.
fn held(n: usize, kit: Option<&faraday_core::testkit::Kit>) -> Session {
    use faraday_core::testkit::{TEST_SEEDS, test_words};
    let c = faraday_core::testkit::compute();
    let mut session = faraday_core::testkit::session();
    session
        .keys
        .add_words(&c, &test_words(TEST_SEEDS[n].0), TEST_SEEDS[n].1, None)
        .unwrap();
    if let Some(kit) = kit {
        session
            .add_wallet(
                &c,
                kit.name,
                &kit.descriptor,
                "Test wallet",
                faraday_core::wallet::Origin::Named,
            )
            .unwrap();
    }
    session
}

#[test]
fn a_taproot_pkh_leaf_counts_the_master_of_the_key_its_hash_names() {
    use faraday_bip::bitcoin::hashes::{Hash as _, hash160};
    use faraday_bip::bitcoin::opcodes::all::{OP_DUP, OP_EQUALVERIFY, OP_HASH160};
    let (a, internal) = (key(1), key(7));
    let h = hash160::Hash::hash(&a.x.serialize());
    let script = Builder::new()
        .push_opcode(OP_DUP)
        .push_opcode(OP_HASH160)
        .push_slice(h.to_byte_array())
        .push_opcode(OP_EQUALVERIFY)
        .push_opcode(OP_CHECKSIG)
        .into_script();
    let mut t = tap_leaf(&internal, 7, script, &[(&a, 1)]);
    assert_eq!(count(&t.inner), fps(&[]));
    let s = leaf_sig(&t.inner, &t.prevs, 0, t.leaf, &a);
    t.inner.inputs[0].tap_script_sigs.insert((a.x, t.leaf), s);
    assert_eq!(count(&t.inner), fps(&[1]));
}

#[test]
fn a_wallet_that_knows_the_script_refuses_an_origin_that_names_another_master() {
    use faraday_core::testkit::{kits, unsigned};
    let c = faraday_core::testkit::compute();
    let kit = &kits()[0];
    let session = held(1, Some(kit));
    let mut p = unsigned(kit).unwrap();
    let by = session
        .sign(&c, &mut p, &zeroize::Zeroizing::new([0; 32]), &mut None)
        .unwrap();
    assert_eq!(by.len(), 1);
    let mut inner = p.inner().clone();
    assert_eq!(state_with(&session, &inner), by);
    // The file now says master 0 made the signature: the wallet's key for
    // master 0 is another key.
    let zero = faraday_core::testkit::key(0, "m/48'/1'/0'/2'");
    let zero_fp = Fp::from_str(&zero[1..9]).unwrap();
    for (f, _) in inner.inputs[0].bip32_derivation.values_mut() {
        if Fingerprint::from(*f) == by[0] {
            *f = zero_fp;
        }
    }
    assert_eq!(state_with(&session, &inner), fps(&[]));
}

#[test]
fn a_fake_key_path_signature_on_a_musig_spend_counts_nobody() {
    let c = faraday_core::testkit::compute();
    let policy = faraday_core::testkit::musig_policy().unwrap();
    let mut session = faraday_core::testkit::session();
    session
        .add_wallet(
            &c,
            "MuSig2",
            &policy.to_descriptor_checksummed(),
            "Test wallet",
            faraday_core::wallet::Origin::Named,
        )
        .unwrap();
    let mut inner = faraday_core::testkit::musig_unsigned()
        .unwrap()
        .inner()
        .clone();
    assert_eq!(state_with(&session, &inner), fps(&[]));
    inner.inputs[0].tap_key_sig =
        faraday_bip::bitcoin::taproot::Signature::from_slice(&[3u8; 64]).ok();
    assert_eq!(state_with(&session, &inner), fps(&[]));
}

/// Each test seed signs `psbt` in turn, with `kit`'s wallet loaded or not:
/// after each the count is the masters that signed so far.
fn counted_as_seeds_sign(kit: &faraday_core::testkit::Kit, first: &Psbt, wallet: bool) -> usize {
    let c = faraday_core::testkit::compute();
    let mut psbt = first.clone();
    let mut signed: Vec<Fingerprint> = Vec::new();
    for n in 0..faraday_core::testkit::TEST_SEEDS.len() {
        // The wallet is loaded to sign, and left out for the count when
        // the count is asked of a device without it.
        let signer = held(n, Some(kit));
        let Ok(by) = signer.sign(&c, &mut psbt, &zeroize::Zeroizing::new([0; 32]), &mut None)
        else {
            continue;
        };
        let session = held(n, wallet.then_some(kit));
        for f in by {
            if !signed.contains(&f) {
                signed.push(f);
            }
        }
        let mut want = signed.clone();
        want.sort_by_key(|f| format!("{f}"));
        assert_eq!(
            state_with(&session, psbt.inner()),
            want,
            "{} (wallet {wallet}) after seed {n}",
            kit.id
        );
    }
    signed.len()
}

#[test]
fn every_kit_counts_the_masters_that_signed_with_and_without_its_wallet() {
    for kit in faraday_core::testkit::kits() {
        for wallet in [false, true] {
            let first = faraday_core::testkit::unsigned(&kit).unwrap();
            assert!(
                counted_as_seeds_sign(&kit, &first, wallet) > 0,
                "{}",
                kit.id
            );
        }
    }
}

#[test]
fn the_inheritance_recovery_spend_counts_the_master_that_signed_it() {
    let kit = faraday_core::testkit::kits()
        .into_iter()
        .find(|k| k.id == "inheritance")
        .unwrap();
    let first = faraday_core::testkit::recovery(&kit).unwrap();
    for wallet in [false, true] {
        assert!(counted_as_seeds_sign(&kit, &first, wallet) > 0);
    }
}

#[test]
fn a_final_input_beside_an_open_one_counts_the_masters_of_the_open_one() {
    // Input 0 finalized by another device, input 1 still open and signed.
    let (a, b) = (key(1), key(2));
    let script = wsh_multisig(&[&a, &b], 2);
    let spk = ScriptBuf::new_p2wsh(&script.wscript_hash());
    let (mut inner, _) = psbt_of(&[spk.clone(), spk]);
    for i in 0..2 {
        for (k, n) in [(&a, 1), (&b, 2)] {
            inner.inputs[i]
                .bip32_derivation
                .insert(k.pk.inner, (fp(n), path()));
        }
        inner.inputs[i].witness_script = Some(script.clone());
    }
    let sigs = [&a, &b].map(|k| ecdsa_wsh(&inner, 0, &script, k).to_vec());
    inner.inputs[0].final_script_witness = Some(Witness::from_slice(&[
        vec![],
        sigs[0].clone(),
        sigs[1].clone(),
        script.to_bytes(),
    ]));
    inner.inputs[0].witness_script = None;
    let sig = ecdsa_wsh(&inner, 1, &script, &a);
    inner.inputs[1].partial_sigs.insert(a.pk, sig);
    assert_eq!(count(&inner), fps(&[1]));
    let sig = ecdsa_wsh(&inner, 1, &script, &b);
    inner.inputs[1].partial_sigs.insert(b.pk, sig);
    assert_eq!(count(&inner), fps(&[1, 2]));
}

#[test]
fn musig_rounds_count_a_participant_from_its_partial_signature_and_not_its_nonce() {
    let c = faraday_core::testkit::compute();
    let policy = faraday_core::testkit::musig_policy().unwrap();
    let mut session = faraday_core::testkit::session();
    session
        .add_wallet(
            &c,
            "MuSig2",
            &policy.to_descriptor_checksummed(),
            "Test wallet",
            faraday_core::wallet::Origin::Named,
        )
        .unwrap();
    let (_, nonces, partials) = faraday_core::testkit::musig_rounds().unwrap();
    for (_, file) in &nonces {
        assert_eq!(state_with(&session, file.inner()), fps(&[]));
    }
    for (name, file) in &partials {
        // A cosigner's partial signature is the file's word: none counts.
        assert_eq!(state_with(&session, file.inner()), fps(&[]), "{name}");
        let c = faraday_core::testkit::compute();
        let got = Spend::new(file.clone(), "t").signing(&c, &session, None, None);
        assert!(
            got.in_file
                .iter()
                .any(|f| format!("{f}").to_lowercase() == name.to_lowercase()),
            "{name}: {:?}",
            got.in_file
        );
    }
}

#[test]
fn a_master_this_device_holds_counts_only_for_the_key_it_derives_at_the_stated_place() {
    let evil = key(5);
    let (session, a, _) = two_owners(&evil);
    // A key that is not the held master's, filed under the held master's
    // fingerprint and a place; no wallet is loaded.
    let (mut inner, _) = psbt_of(&[ScriptBuf::new_p2wpkh(&evil.pk.wpubkey_hash().unwrap())]);
    let p = DerivationPath::from_str("m/84'/1'/0'/0/0").unwrap();
    inner.inputs[0]
        .bip32_derivation
        .insert(evil.pk.inner, (Fp::from(a.0), p));
    let sig = ecdsa_wpkh(&inner, 0, &evil);
    inner.inputs[0].partial_sigs.insert(evil.pk, sig);
    assert_eq!(state_with(&session, &inner), fps(&[]));
}

// A master a loaded wallet names, one input's wallet, two keys of one master.

fn master_fp(n: usize, path: &str) -> Fp {
    let k = faraday_core::testkit::key(n, path);
    Fp::from_str(&k[1..9]).unwrap()
}

#[test]
fn a_loaded_wallets_cosigner_is_not_counted_on_an_origin_its_key_does_not_derive_to() {
    let kit = faraday_core::testkit::kits()
        .into_iter()
        .find(|k| k.id == "inheritance")
        .unwrap();
    let session = held(0, Some(&kit));
    let b = master_fp(1, "m/48'/1'/1'/2'");
    let evil = key(5);
    let (mut inner, _) = psbt_of(&[ScriptBuf::new_p2wpkh(&evil.pk.wpubkey_hash().unwrap())]);
    let p = DerivationPath::from_str("m/48'/1'/1'/2'/0/0").unwrap();
    inner.inputs[0]
        .bip32_derivation
        .insert(evil.pk.inner, (b, p));
    let sig = ecdsa_wpkh(&inner, 0, &evil);
    inner.inputs[0].partial_sigs.insert(evil.pk, sig);
    assert_eq!(state_with(&session, &inner), fps(&[]));
    // With no wallet loaded, the master is nobody's word to check.
    let bare = held(0, None);
    assert_eq!(state_with(&bare, &inner), vec![Fingerprint::from(b)]);
}

/// A single-key wallet of test seed `n` at BIP-84 account 0.
fn wpkh_kit(n: usize, name: &'static str) -> faraday_core::testkit::Kit {
    faraday_core::testkit::Kit {
        id: "wpkh",
        name,
        descriptor: format!(
            "wpkh({}/<0;1>/*)",
            faraday_core::testkit::key(n, "m/84'/1'/0'")
        ),
        cosigner: None,
        fee: 0,
    }
}

#[test]
#[expect(
    clippy::needless_range_loop,
    reason = "the index names the input and its wallet"
)]
fn each_input_is_judged_by_the_wallet_it_spends_from() {
    let c = faraday_core::testkit::compute();
    let (k0, k1) = (wpkh_kit(0, "Wallet 0"), wpkh_kit(1, "Wallet 1"));
    let mut both = held(0, Some(&k0));
    both.keys
        .add_words(
            &c,
            &faraday_core::testkit::test_words(faraday_core::testkit::TEST_SEEDS[1].0),
            faraday_core::testkit::TEST_SEEDS[1].1,
            None,
        )
        .unwrap();
    both.add_wallet(
        &c,
        k1.name,
        &k1.descriptor,
        "Test wallet",
        faraday_core::wallet::Origin::Named,
    )
    .unwrap();
    // Input 0 is wallet 0's, input 1 wallet 1's.
    let policies: Vec<_> = [&k0, &k1]
        .iter()
        .map(|k| faraday_bip::policy::WalletPolicy::parse_any(&c, &k.descriptor).unwrap())
        .collect();
    let spks: Vec<ScriptBuf> = policies
        .iter()
        .map(|p| p.script_at(&c, false, 0).unwrap())
        .collect();
    let (mut inner, _) = psbt_of(&spks);
    for i in 0..2 {
        let prev = Transaction {
            version: Version::TWO,
            lock_time: LockTime::from_height(i as u32 + 1).unwrap(),
            input: vec![],
            output: vec![inner.inputs[i].witness_utxo.clone().unwrap()],
        };
        inner.unsigned_tx.input[i].previous_output = OutPoint::new(prev.compute_txid(), 0);
        inner.inputs[i].non_witness_utxo = Some(prev);
        let (f, pk) = policies[i].keys_at(false, 0)[0];
        let origin = policies[i].keys()[0].path().unwrap().clone();
        inner.inputs[i].bip32_derivation.insert(
            pk.inner,
            (
                Fp::from(f.0),
                origin
                    .child(faraday_bip::bitcoin::bip32::ChildNumber::from_normal_idx(0).unwrap())
                    .child(faraday_bip::bitcoin::bip32::ChildNumber::from_normal_idx(0).unwrap()),
            ),
        );
    }
    inner.unsigned_tx.output[0].value = Amount::from_sat(99_000);
    let mut p = Psbt::from(inner);
    sign_held(&both, &mut p).unwrap();
    // Each master signed its own input, the other did not sign it: nothing
    // is left to sign, and no master signed every input.
    assert_eq!(state_in(&both, p.inner()), (vec![], vec![]));
    // A device that holds only master 0 sees wallet 1's true signature.
    let mut only0 = held(0, Some(&k0));
    only0
        .add_wallet(
            &c,
            k1.name,
            &k1.descriptor,
            "Test wallet",
            faraday_core::wallet::Origin::Named,
        )
        .unwrap();
    assert_eq!(state_in(&only0, p.inner()), (vec![], vec![]));
}

#[test]
fn the_second_key_of_a_master_in_a_wallet_counts_on_the_recovery_branch() {
    let a0 = faraday_core::testkit::key(0, "m/48'/1'/0'/2'");
    let a1 = faraday_core::testkit::key(0, "m/48'/1'/1'/2'");
    let kit = faraday_core::testkit::Kit {
        id: "two-accounts",
        name: "Two accounts",
        descriptor: format!("wsh(or_d(pk({a0}/<0;1>/*),and_v(v:pk({a1}/<0;1>/*),older(100))))"),
        cosigner: None,
        fee: 1_980,
    };
    let session = held(0, Some(&kit));
    let mut inner = faraday_core::testkit::unsigned(&kit).unwrap().into_inner();
    inner.unsigned_tx.input[0].sequence = Sequence::from_height(100);
    // Only the recovery key's origin is named.
    inner.inputs[0]
        .bip32_derivation
        .retain(|_, (_, p)| p.to_string().starts_with("48'/1'/1'"));
    assert_eq!(inner.inputs[0].bip32_derivation.len(), 1);
    let mut p = Psbt::from(inner);
    let got = sign_held(&session, &mut p);
    let f = Fingerprint::from(master_fp(0, "m/48'/1'/1'/2'"));
    assert_eq!(got, Ok(vec![f]));
    assert_eq!(state_in(&session, p.inner()), (vec![f], vec![]));
}

#[test]
fn a_signature_on_another_owners_input_signs_nothing_for_a_musig_wallet() {
    let c = faraday_core::testkit::compute();
    let policy = faraday_core::testkit::musig_policy().unwrap();
    let mut session = faraday_core::testkit::session();
    session
        .add_wallet(
            &c,
            "MuSig2",
            &policy.to_descriptor_checksummed(),
            "Test wallet",
            faraday_core::wallet::Origin::Named,
        )
        .unwrap();
    let mut inner = faraday_core::testkit::musig_unsigned()
        .unwrap()
        .inner()
        .clone();
    let secp = Secp256k1::new();
    let attacker = key(7);
    let tweaked = attacker.pair.tap_tweak(&secp, None);
    let spk = ScriptBuf::new_p2tr_tweaked(tweaked.public_parts().0);
    inner.unsigned_tx.input.push(TxIn {
        previous_output: OutPoint::new(faraday_bip::bitcoin::Txid::from_byte_array([2; 32]), 0),
        script_sig: ScriptBuf::new(),
        sequence: Sequence::MAX,
        witness: Witness::new(),
    });
    inner.inputs.push(faraday_bip::bitcoin::psbt::Input {
        witness_utxo: Some(TxOut {
            value: Amount::from_sat(50_000),
            script_pubkey: spk,
        }),
        tap_internal_key: Some(attacker.x),
        ..Default::default()
    });
    inner.inputs[0].final_script_witness = Some(Witness::from_slice(&[vec![3u8; 64]]));
    let prevs: Vec<TxOut> = inner
        .inputs
        .iter()
        .map(|i| i.witness_utxo.clone().unwrap())
        .collect();
    inner.inputs[1].tap_key_sig = Some(key_sig(&inner, &prevs, 1, &tweaked.to_keypair()));
    assert_eq!(state_in(&session, &inner).0, fps(&[]));
}

// What `Spend::finish` and `collect` refuse of a taproot input already
// final, beside the rows `final_invalid` reads.

/// Input 0 spends the tree of the one leaf `<A> CHECKSIG` (as `multi_a` of
/// one) under internal key 7, with the leaf's script and control block; the
/// signature is made over `annex` when given.
fn annexed(annex: Option<&[u8]>, a: &Key, t: &Tap) -> Vec<u8> {
    use faraday_bip::bitcoin::sighash::Annex;
    let h = SighashCache::new(&t.inner.unsigned_tx)
        .taproot_signature_hash(
            0,
            &Prevouts::All(&t.prevs),
            annex.map(|a| Annex::new(a).unwrap()),
            Some((t.leaf, 0xffff_ffff)),
            TapSighashType::Default,
        )
        .unwrap();
    schnorr(h.to_byte_array(), &a.pair).to_vec()
}

#[test]
fn finish_judges_a_script_path_signature_made_over_an_annex() {
    let (a, internal) = (key(1), key(7));
    let t = tap_leaf(&internal, 7, multi_a(&[&a], 1), &[(&a, 1)]);
    let (sc, cb) = (t.script.to_bytes(), t.control.clone());
    let annex = vec![0x50, 1, 2, 3];
    let good = annexed(Some(&annex), &a, &t);
    let with = |sig: Vec<u8>| with_witness(&t.inner, &[sig, sc.clone(), cb.clone(), annex.clone()]);
    assert_eq!(finish_of(&with(good.clone())), Ok(()));
    // Made without the annex, or forged: refused.
    assert!(finish_of(&with(annexed(None, &a, &t))).is_err());
    let mut forged = good;
    forged[10] ^= 1;
    assert!(finish_of(&with(forged)).is_err());
}

#[test]
fn finish_refuses_a_65_byte_element_ending_00_on_a_tap_miniscript_leaf() {
    let (a, internal) = (key(1), key(7));
    let t = tap_leaf(&internal, 7, multi_a(&[&a], 1), &[(&a, 1)]);
    let (sc, cb) = (t.script.to_bytes(), t.control.clone());
    let good = leaf_sig(&t.inner, &t.prevs, 0, t.leaf, &a).to_vec();
    // The signature with an explicit zero sighash byte.
    let mut zero = good.clone();
    zero.push(0);
    assert!(finish_of(&with_witness(&t.inner, &[zero, sc.clone(), cb.clone()])).is_err());
    // A true signature beside a 65-byte element ending 00.
    let junk = [vec![3u8; 64], vec![0u8]].concat();
    let w = with_witness(&t.inner, &[junk, good.clone(), sc.clone(), cb.clone()]);
    assert!(finish_of(&w).is_err());
    // The same element on a leaf that is no miniscript is only data.
    assert_eq!(finish_of(&with_witness(&t.inner, &[good, sc, cb])), Ok(()));
}

#[test]
fn finish_judges_a_script_path_signature_whose_control_block_ends_00() {
    let (a, internal) = (key(1), key(7));
    let secp = Secp256k1::new();
    let script = multi_a(&[&a], 1);
    // A second leaf grinds the sibling hash until the control block ends 00.
    let (info, cb) = (0u32..)
        .find_map(|n| {
            let other = Builder::new()
                .push_int(i64::from(n))
                .push_opcode(faraday_bip::bitcoin::opcodes::all::OP_DROP)
                .push_int(1)
                .into_script();
            let info = TaprootBuilder::new()
                .add_leaf(1, script.clone())
                .unwrap()
                .add_leaf(1, other)
                .unwrap()
                .finalize(&secp, internal.x)
                .unwrap();
            let cb = info
                .control_block(&(script.clone(), LeafVersion::TapScript))
                .unwrap();
            (cb.serialize().last() == Some(&0)).then_some((info, cb))
        })
        .unwrap();
    assert_eq!(cb.serialize().len(), 65);
    let (mut inner, prevs) = psbt_of(&[ScriptBuf::new_p2tr_tweaked(info.output_key())]);
    let leaf = TapLeafHash::from_script(&script, LeafVersion::TapScript);
    inner.inputs[0].tap_internal_key = Some(internal.x);
    let good = leaf_sig(&inner, &prevs, 0, leaf, &a).to_vec();
    let (sc, cb) = (script.to_bytes(), cb.serialize());
    assert_eq!(
        finish_of(&with_witness(
            &inner,
            &[good.clone(), sc.clone(), cb.clone()]
        )),
        Ok(())
    );
    let mut forged = good;
    forged[10] ^= 1;
    assert!(finish_of(&with_witness(&inner, &[forged, sc, cb])).is_err());
}

#[test]
fn finish_refuses_a_sighash_single_key_path_signature_with_no_output_to_sign() {
    let (inner, _, tweaked) = key_path(None);
    let spk = inner.inputs[0].witness_utxo.clone().unwrap().script_pubkey;
    let (mut two, prevs) = psbt_of(&[spk.clone(), spk]);
    two.inputs[0].tap_internal_key = Some(key(7).x);
    two.inputs[1].tap_internal_key = Some(key(7).x);
    let first = key_sig(&two, &prevs, 0, &tweaked).to_vec();
    let second = key_sig(&two, &prevs, 1, &tweaked).to_vec();
    two.inputs[0].final_script_witness = Some(Witness::from_slice(&[first]));
    let mut ok = two.clone();
    ok.inputs[1].final_script_witness = Some(Witness::from_slice(&[second]));
    assert_eq!(finish_of(&ok), Ok(()));
    // Input 1 asks SIGHASH_SINGLE with one output only: nothing to sign.
    let mut bad = two;
    bad.inputs[1].final_script_witness =
        Some(Witness::from_slice(&[[vec![7u8; 64], vec![3]].concat()]));
    assert!(finish_of(&bad).is_err());
}

/// A cosigner's file for a spend of a true `<A> CHECKSIG` leaf at input 0,
/// beside input 1 whose output the file does not state (the spend's own
/// copy does): `forge` flips a bit of the signature.
fn collected_beside_an_input_with_no_utxo(forge: bool) -> Result<Vec<Fingerprint>, String> {
    let c = faraday_core::testkit::compute();
    let (a, internal, other) = (key(1), key(7), key(2));
    let script = multi_a(&[&a], 1);
    let secp = Secp256k1::new();
    let info = TaprootBuilder::new()
        .add_leaf(0, script.clone())
        .unwrap()
        .finalize(&secp, internal.x)
        .unwrap();
    let (mut own, prevs) = psbt_of(&[
        ScriptBuf::new_p2tr_tweaked(info.output_key()),
        ScriptBuf::new_p2wpkh(&other.pk.wpubkey_hash().unwrap()),
    ]);
    let leaf = TapLeafHash::from_script(&script, LeafVersion::TapScript);
    let cb = info
        .control_block(&(script.clone(), LeafVersion::TapScript))
        .unwrap();
    own.inputs[0].tap_internal_key = Some(internal.x);
    own.inputs[0].tap_merkle_root = info.merkle_root();
    own.inputs[0]
        .tap_scripts
        .insert(cb.clone(), (script.clone(), LeafVersion::TapScript));
    own.inputs[0]
        .tap_key_origins
        .insert(a.x, (vec![leaf], (fp(1), path())));
    let mut sig = leaf_sig(&own, &prevs, 0, leaf, &a).to_vec();
    if forge {
        sig[10] ^= 1;
    }
    let mut file = own.clone();
    file.inputs[1].witness_utxo = None;
    file.inputs[0] = faraday_bip::bitcoin::psbt::Input {
        witness_utxo: own.inputs[0].witness_utxo.clone(),
        final_script_witness: Some(Witness::from_slice(&[
            sig,
            script.to_bytes(),
            cb.serialize(),
        ])),
        ..Default::default()
    };
    let session = Session::on(Network::Testnet);
    Spend::new(Psbt::from(own), "t").collect(
        &c,
        &session,
        &Psbt::from(file).to_bytes(),
        "cosigner.psbt",
    )
}

#[test]
fn a_cosigners_true_final_script_path_signature_is_not_refused_beside_an_input_with_no_utxo() {
    assert!(collected_beside_an_input_with_no_utxo(false).is_ok());
}

#[test]
fn a_cosigners_forged_final_script_path_signature_is_refused_beside_an_input_with_no_utxo() {
    assert!(collected_beside_an_input_with_no_utxo(true).is_err());
}

// A spend with a MuSig2 wallet's input beside inputs of other owners: each
// input is read by the wallet that holds its script.

/// The test seed `n`'s secret key at `path`.
fn seed_secret(n: usize, path: &str) -> SecretKey {
    use faraday_bip::bip39::{Language, Mnemonic};
    use faraday_bip::keys::MasterKey;
    use faraday_core::testkit::{TEST_SEEDS, compute, net, on_net, test_words};
    let c = compute();
    let m = Mnemonic::parse(&c, Language::English, &test_words(TEST_SEEDS[n].0)).unwrap();
    let mk = MasterKey::from_seed(&c, &m.to_seed(&c, b"").unwrap(), net());
    *mk.derive(&c, &on_net(path).parse().unwrap()).secret_key()
}

fn tagged(tag: &str, msg: &[u8]) -> [u8; 32] {
    use faraday_bip::bitcoin::hashes::sha256;
    let t = sha256::Hash::hash(tag.as_bytes()).to_byte_array();
    let mut v = t.to_vec();
    v.extend(t);
    v.extend(msg);
    sha256::Hash::hash(&v).to_byte_array()
}

/// The secret key that signs for the MuSig2 wallet's output at `0/index`
/// (BIP-327 key aggregation of the three test seeds at m/86'/1'/0', the
/// wallet's derivation, the BIP-86 tweak), and the internal key it signs
/// under.
fn musig_output_secret(index: u32) -> (Keypair, XOnlyPublicKey) {
    use faraday_bip::bitcoin::bip32::ChildNumber;
    use faraday_bip::bitcoin::secp256k1::{PublicKey as Pk, Scalar};
    let secp = Secp256k1::new();
    let sks: Vec<SecretKey> = (0..3).map(|n| seed_secret(n, "m/86'/1'/0'")).collect();
    let pks: Vec<Pk> = sks.iter().map(|s| s.public_key(&secp)).collect();
    let sorted = faraday_bip::musig::sort_keys(&pks);
    let mut l = vec![];
    for k in &sorted {
        l.extend(k.serialize());
    }
    let list = tagged("KeyAgg list", &l);
    let second = sorted.iter().find(|k| **k != sorted[0]).copied();
    let mut d: Option<SecretKey> = None;
    for k in &sorted {
        let sk = sks[pks.iter().position(|p| p == k).unwrap()];
        let term = if Some(*k) == second {
            sk
        } else {
            let mut m = list.to_vec();
            m.extend(k.serialize());
            sk.mul_tweak(&Scalar::from_be_bytes(tagged("KeyAgg coefficient", &m)).unwrap())
                .unwrap()
        };
        d = Some(match d {
            None => term,
            Some(x) => x
                .add_tweak(&Scalar::from_be_bytes(term.secret_bytes()).unwrap())
                .unwrap(),
        });
    }
    let mut d = d.unwrap();
    let mut xp = faraday_bip::musig::aggregate_xpub(&sorted, faraday_core::testkit::net()).unwrap();
    for step in [0u32, index] {
        let child = ChildNumber::from_normal_idx(step).unwrap();
        let (t, _) = xp.ckd_pub_tweak(child).unwrap();
        d = d
            .add_tweak(&Scalar::from_be_bytes(t.secret_bytes()).unwrap())
            .unwrap();
        xp = xp.ckd_pub(&secp, child).unwrap();
    }
    let internal = xp.public_key.x_only_public_key().0;
    let kp = Keypair::from_secret_key(&secp, &d);
    (kp.tap_tweak(&secp, None).to_keypair(), internal)
}

/// A session with the MuSig2 wallet loaded (and test seed 0 held when
/// `hold`), and a PSBT whose input 0 is the wallet's output at 0/0 and
/// input 1 is `spk1`; the MuSig input carries the aggregate signature the
/// wallet's key makes when `sign`.
fn musig_beside(hold: bool, spk1: ScriptBuf, sign: bool) -> (Session, Inner, Vec<TxOut>) {
    use faraday_bip::bitcoin::bip32::ChildNumber;
    use faraday_core::testkit::{TEST_SEEDS, test_words};
    let c = faraday_core::testkit::compute();
    let policy = faraday_core::testkit::musig_policy().unwrap();
    let mut session = faraday_core::testkit::session();
    if hold {
        session
            .keys
            .add_words(&c, &test_words(TEST_SEEDS[0].0), TEST_SEEDS[0].1, None)
            .unwrap();
    }
    session
        .add_wallet(
            &c,
            "MuSig2",
            &policy.to_descriptor_checksummed(),
            "Test wallet",
            faraday_core::wallet::Origin::Named,
        )
        .unwrap();
    let (kp, internal) = musig_output_secret(0);
    let spk0 = policy.script_at(&c, false, 0).unwrap();
    let (mut inner, prevs) = psbt_of(&[spk0, spk1]);
    let mut input = faraday_core::testkit::musig_unsigned()
        .unwrap()
        .inner()
        .inputs[0]
        .clone();
    input.witness_utxo = Some(prevs[0].clone());
    let old = input.tap_internal_key.unwrap();
    let (leaves, (f, _)) = input.tap_key_origins.remove(&old).unwrap();
    input.tap_internal_key = Some(internal);
    let zero = ChildNumber::from_normal_idx(0).unwrap();
    input.tap_key_origins.insert(
        internal,
        (leaves, (f, DerivationPath::from(vec![zero, zero]))),
    );
    if sign {
        input.tap_key_sig = Some(key_sig(&inner, &prevs, 0, &kp));
    }
    inner.inputs[0] = input;
    (session, inner, prevs)
}

#[test]
fn an_open_input_of_no_known_owner_beside_a_signed_musig_input_leaves_nobody_signed() {
    // Input 1 is the MuSig wallet's own output at 0/1, open, unsigned, and
    // states no origins: nothing says who may sign it.
    let c = faraday_core::testkit::compute();
    let policy = faraday_core::testkit::musig_policy().unwrap();
    let spk1 = policy.script_at(&c, false, 1).unwrap();
    let (session, mut inner, _) = musig_beside(false, spk1, true);
    inner.inputs[1].tap_internal_key = Some(musig_output_secret(1).1);
    assert_eq!(state_in(&session, &inner), (vec![], vec![]));
}

#[test]
fn a_signed_musig_spend_stating_no_origins_reads_nobody_and_still_finishes() {
    let c = faraday_core::testkit::compute();
    let policy = faraday_core::testkit::musig_policy().unwrap();
    let spk1 = policy.script_at(&c, false, 1).unwrap();
    let (session, mut inner, mut prevs) = musig_beside(false, spk1, true);
    prevs[1].script_pubkey = policy.script_at(&c, false, 1).unwrap();
    inner.inputs[0].tap_key_origins.clear();
    let (kp1, internal1) = musig_output_secret(1);
    inner.inputs[1].tap_internal_key = Some(internal1);
    inner.inputs[1].tap_key_sig = Some(key_sig(&inner, &prevs, 1, &kp1));
    assert_eq!(state_in(&session, &inner), (vec![], vec![]));
    assert_eq!(finish_of(&inner), Ok(()));
}

#[test]
fn the_aggregate_key_is_no_masters_to_sign() {
    // Input 0 signed and stating no origins; input 1 the wallet's, open,
    // with its origins: the three participants are left to sign, and the
    // aggregate key's own fingerprint is no person's master.
    let c = faraday_core::testkit::compute();
    let policy = faraday_core::testkit::musig_policy().unwrap();
    let spk1 = policy.script_at(&c, false, 1).unwrap();
    let (session, mut inner, _) = musig_beside(false, spk1, true);
    let (_, internal1) = musig_output_secret(1);
    inner.inputs[1] = inner.inputs[0].clone();
    inner.inputs[1].tap_key_sig = None;
    inner.inputs[1].tap_internal_key = Some(internal1);
    inner.inputs[1].witness_utxo = inner.inputs[1].witness_utxo.clone().map(|mut o| {
        o.script_pubkey = policy.script_at(&c, false, 1).unwrap();
        o
    });
    let old = inner.inputs[0].tap_internal_key.unwrap();
    let (l, (f, _)) = inner.inputs[1].tap_key_origins.remove(&old).unwrap();
    let step = |n| faraday_bip::bitcoin::bip32::ChildNumber::from_normal_idx(n).unwrap();
    inner.inputs[1].tap_key_origins.insert(
        internal1,
        (l, (f, DerivationPath::from(vec![step(0), step(1)]))),
    );
    inner.inputs[0].tap_key_origins.clear();
    let (signers, to_sign) = state_in(&session, &inner);
    assert_eq!(signers, vec![]);
    assert_eq!(to_sign.len(), 3);
    assert!(!to_sign.contains(&Fingerprint::from(f)));
}

/// A MuSig input signed, and a p2wpkh input of the single-key wallet of
/// test seed 0, open and unsigned, with its true origin; both wallets
/// loaded and seed 0 held.
fn musig_beside_a_wpkh_of_a_held_seed() -> (Session, Inner, Fingerprint) {
    use faraday_bip::bitcoin::bip32::ChildNumber;
    let c = faraday_core::testkit::compute();
    let kit = wpkh_kit(0, "One");
    let wp = faraday_bip::policy::WalletPolicy::parse_any(&c, &kit.descriptor).unwrap();
    let wspk = wp.script_at(&c, false, 0).unwrap();
    let (mut session, mut inner, _) = musig_beside(true, wspk.clone(), false);
    session
        .add_wallet(
            &c,
            kit.name,
            &kit.descriptor,
            "Test wallet",
            faraday_core::wallet::Origin::Named,
        )
        .unwrap();
    // A segwit v0 input is signed only with the transaction it spends in
    // the file: without it the screen refuses ("input 2 carries no previous
    // transaction; its amount cannot be checked").
    let prev = Transaction {
        version: Version::TWO,
        lock_time: LockTime::from_height(1).unwrap(),
        input: vec![],
        output: vec![inner.inputs[1].witness_utxo.clone().unwrap()],
    };
    inner.unsigned_tx.input[1].previous_output = OutPoint::new(prev.compute_txid(), 0);
    inner.inputs[1].non_witness_utxo = Some(prev);
    // The MuSig input is signed over the transaction as it now stands.
    let prevs: Vec<TxOut> = inner
        .inputs
        .iter()
        .map(|i| i.witness_utxo.clone().unwrap())
        .collect();
    let (kp, _) = musig_output_secret(0);
    inner.inputs[0].tap_key_sig = Some(key_sig(&inner, &prevs, 0, &kp));
    let (master, pk) = wp.keys_at(false, 0)[0];
    let origin = wp.keys()[0].path().unwrap().clone();
    let zero = ChildNumber::from_normal_idx(0).unwrap();
    inner.inputs[1].bip32_derivation.insert(
        pk.inner,
        (Fp::from(master.0), origin.child(zero).child(zero)),
    );
    (session, inner, Fingerprint::from(Fp::from(master.0)))
}

#[test]
fn a_held_master_with_a_single_key_input_beside_a_signed_musig_input_is_offered_sign() {
    let (session, inner, seed0) = musig_beside_a_wpkh_of_a_held_seed();
    let (signers, to_sign) = state_in(&session, &inner);
    assert_eq!(signers, vec![]);
    assert_eq!(to_sign, vec![seed0]);
}

#[test]
fn a_master_outside_the_musig_wallet_on_an_open_input_is_offered_sign() {
    let outsider = key(5);
    let spk1 = ScriptBuf::new_p2wpkh(&outsider.pk.wpubkey_hash().unwrap());
    let (session, mut inner, _) = musig_beside(false, spk1, true);
    inner.inputs[1]
        .bip32_derivation
        .insert(outsider.pk.inner, (fp(5), path()));
    let (signers, to_sign) = state_in(&session, &inner);
    assert_eq!(signers, vec![]);
    assert_eq!(to_sign, fps(&[5]));
}

#[test]
fn this_device_signs_its_own_input_beside_a_signed_musig_input() {
    let (session, inner, seed0) = musig_beside_a_wpkh_of_a_held_seed();
    let mut p = Psbt::from(inner);
    assert_eq!(sign_held(&session, &mut p), Ok(vec![seed0]));
    assert_eq!(state_in(&session, p.inner()), (vec![seed0], vec![]));
}

#[test]
fn a_pkh_leaf_element_over_520_bytes_is_refused_and_at_520_finishes() {
    use faraday_bip::bitcoin::hashes::hash160;
    use faraday_bip::bitcoin::opcodes::all::{OP_DUP, OP_EQUALVERIFY, OP_HASH160};
    let internal = key(7);
    // (key length, signature length, finishes): Bitcoin Core refuses an
    // element over 520 bytes ("Push value size limit exceeded").
    let rows = [
        (520usize, 64usize, true),
        (33, 520, true),
        (521, 64, false),
        (600, 64, false),
        (5000, 64, false),
        (33, 521, false),
        (33, 600, false),
        (33, 5000, false),
        (521, 521, false),
    ];
    for (key_len, sig_len, ok) in rows {
        let k = vec![2u8; key_len];
        let h = hash160::Hash::hash(&k).to_byte_array();
        let script = Builder::new()
            .push_opcode(OP_DUP)
            .push_opcode(OP_HASH160)
            .push_slice(h)
            .push_opcode(OP_EQUALVERIFY)
            .push_opcode(OP_CHECKSIG)
            .into_script();
        let t = tap_leaf(&internal, 7, script, &[]);
        let w = with_witness(
            &t.inner,
            &[
                vec![9u8; sig_len],
                k,
                t.script.to_bytes(),
                t.control.clone(),
            ],
        );
        assert_eq!(
            finish_of(&w).is_ok(),
            ok,
            "key {key_len} sig {sig_len}: {:?}",
            finish_of(&w)
        );
    }
}

#[test]
fn a_pkh_leaf_with_a_key_of_an_unknown_type_is_finished_and_collected() {
    use faraday_bip::bitcoin::hashes::hash160;
    use faraday_bip::bitcoin::opcodes::all::{OP_DUP, OP_EQUALVERIFY, OP_HASH160};
    let c = faraday_core::testkit::compute();
    let internal = key(7);
    // BIP-342: any non-empty signature passes a key of an unknown type.
    for (key_len, sig_len) in [(33usize, 64usize), (65, 65), (64, 1), (31, 64)] {
        let k = vec![2u8; key_len];
        let h = hash160::Hash::hash(&k).to_byte_array();
        let script = Builder::new()
            .push_opcode(OP_DUP)
            .push_opcode(OP_HASH160)
            .push_slice(h)
            .push_opcode(OP_EQUALVERIFY)
            .push_opcode(OP_CHECKSIG)
            .into_script();
        let t = tap_leaf(&internal, 7, script, &[]);
        let w = with_witness(
            &t.inner,
            &[
                vec![9u8; sig_len],
                k,
                t.script.to_bytes(),
                t.control.clone(),
            ],
        );
        assert_eq!(finish_of(&w), Ok(()), "key {key_len} sig {sig_len}");
        let session = Session::on(Network::Testnet);
        let got = Spend::new(Psbt::from(t.inner.clone()), "t").collect(
            &c,
            &session,
            &Psbt::from(w).to_bytes(),
            "cosigner.psbt",
        );
        assert!(got.is_ok(), "key {key_len} sig {sig_len}: {got:?}");
    }
}

// ---- Partial signatures of a MuSig2 wallet are read per open input.

/// A MuSig2 input spending the wallet's output at `0/index`, origins kept.
fn musig_input_at(
    policy: &faraday_bip::policy::WalletPolicy,
    prev: &TxOut,
    index: u32,
) -> faraday_bip::bitcoin::psbt::Input {
    use faraday_bip::bitcoin::bip32::ChildNumber;
    let _ = policy;
    let mut input = faraday_core::testkit::musig_unsigned()
        .unwrap()
        .inner()
        .inputs[0]
        .clone();
    input.witness_utxo = Some(prev.clone());
    let old = input.tap_internal_key.unwrap();
    let (leaves, (f, _)) = input.tap_key_origins.remove(&old).unwrap();
    let internal = musig_output_secret(index).1;
    input.tap_internal_key = Some(internal);
    let step = |n| ChildNumber::from_normal_idx(n).unwrap();
    input.tap_key_origins.insert(
        internal,
        (
            leaves,
            (f, DerivationPath::from(vec![step(0), step(index)])),
        ),
    );
    input
}

/// Gives participant `n` of the MuSig2 wallet a partial signature on `input`.
fn add_partial(
    policy: &faraday_bip::policy::WalletPolicy,
    input: &mut faraday_bip::bitcoin::psbt::Input,
    n: usize,
) {
    let parts: Vec<_> = policy.keys().iter().map(|k| k.xpub().public_key).collect();
    let agg = input
        .tap_internal_key
        .unwrap_or_else(|| key(9).x)
        .public_key(faraday_bip::bitcoin::secp256k1::Parity::Even);
    let fake = faraday_bip::musig::PartialSig::from_bytes(&[1u8; 32]).unwrap();
    faraday_psbt::musig::write_partial_sig(input, &parts[n], &agg, &fake);
}

fn participants(policy: &faraday_bip::policy::WalletPolicy, ns: &[usize]) -> Vec<Fingerprint> {
    let mut v: Vec<Fingerprint> = ns
        .iter()
        .map(|n| policy.keys()[*n].fingerprint().unwrap())
        .collect();
    v.sort_by_key(|f| format!("{f}"));
    v
}

/// Two MuSig2 inputs (origins kept), nothing signed.
fn two_musig_inputs() -> (Session, Inner, faraday_bip::policy::WalletPolicy) {
    let c = faraday_core::testkit::compute();
    let policy = faraday_core::testkit::musig_policy().unwrap();
    let spk1 = policy.script_at(&c, false, 1).unwrap();
    let (session, mut inner, prevs) = musig_beside(false, spk1, false);
    inner.inputs[1] = musig_input_at(&policy, &prevs[1], 1);
    (session, inner, policy)
}

fn the_rest(policy: &faraday_bip::policy::WalletPolicy, ns: &[usize]) -> Vec<Fingerprint> {
    let rest: Vec<usize> = (0..3).filter(|n| !ns.contains(n)).collect();
    participants(policy, &rest)
}

/// Input 0 final: a true key-path spend of another key; input 1 the MuSig2
/// wallet's, open.
fn final_other_then_musig() -> (Session, Inner, faraday_bip::policy::WalletPolicy) {
    let c = faraday_core::testkit::compute();
    let policy = faraday_core::testkit::musig_policy().unwrap();
    let spk1 = policy.script_at(&c, false, 1).unwrap();
    let tw = key(9).pair.tap_tweak(&Secp256k1::new(), None);
    let spk9 = ScriptBuf::new_p2tr_tweaked(tw.public_parts().0);
    let (session, mut inner, mut prevs) = musig_beside(false, spk1, false);
    prevs[0].script_pubkey = spk9.clone();
    inner.unsigned_tx.input[0].previous_output.vout = 0;
    inner.inputs[0] = Default::default();
    inner.inputs[0].witness_utxo = Some(prevs[0].clone());
    inner.inputs[1] = musig_input_at(&policy, &prevs[1], 1);
    let sig = key_sig(&inner, &prevs, 0, &tw.to_keypair());
    inner.inputs[0].final_script_witness = Some(Witness::from_slice(&[sig.to_vec()]));
    (session, inner, policy)
}

#[test]
fn a_partial_signature_on_a_final_input_counts_for_nobody() {
    let (session, inner, policy) = final_other_then_musig();
    let all = the_rest(&policy, &[]);
    assert_eq!(state_in(&session, &inner), (vec![], all.clone()));
    let mut one = inner.clone();
    add_partial(&policy, &mut one.inputs[0], 0);
    assert_eq!(state_in(&session, &one), (vec![], all.clone()));
    let mut every = inner.clone();
    for n in 0..3 {
        add_partial(&policy, &mut every.inputs[0], n);
    }
    assert_eq!(state_in(&session, &every), (vec![], all));
}

#[test]
fn a_made_up_partial_signature_on_the_only_open_input_counts_nobody() {
    let (session, mut inner, policy) = final_other_then_musig();
    add_partial(&policy, &mut inner.inputs[1], 0);
    assert_eq!(state_in(&session, &inner), (vec![], the_rest(&policy, &[])));
}

/// Test seed 0 signs the p2wpkh input `at` of its single-key wallet.
fn seed_signs_wpkh(inner: &mut Inner, spk: &ScriptBuf, at: usize) {
    let c = faraday_core::testkit::compute();
    let kit = wpkh_kit(0, "One");
    let wp = faraday_bip::policy::WalletPolicy::parse_any(&c, &kit.descriptor).unwrap();
    let sk = seed_secret(0, "m/84'/1'/0'/0/0");
    let value = inner.inputs[at].witness_utxo.as_ref().unwrap().value;
    let h = SighashCache::new(&inner.unsigned_tx)
        .p2wpkh_signature_hash(at, spk, value, EcdsaSighashType::All)
        .unwrap();
    let sig = ecdsa::Signature::sighash_all(
        Secp256k1::new().sign_ecdsa(&Message::from_digest(h.to_byte_array()), &sk),
    );
    inner.inputs[at]
        .partial_sigs
        .insert(wp.keys_at(false, 0)[0].1, sig);
}

#[test]
fn a_held_participant_is_offered_sign_on_a_musig_input_and_on_a_mixed_spend() {
    use faraday_core::testkit::{TEST_SEEDS, test_words};
    let c = faraday_core::testkit::compute();
    let offered = |session: &Session, inner: &Inner| {
        let p = Psbt::from(inner.clone());
        let ins = session.inspect(&c, &p);
        let to_sign = Spend::new(p, "t")
            .signing(&c, session, Some(&ins), None)
            .to_sign;
        ins.participating_keys
            .iter()
            .copied()
            .filter(|f| session.keys.key_label(*f).is_some() && to_sign.contains(f))
            .collect::<Vec<_>>()
    };
    // Two MuSig inputs, origins kept, seed 0 held.
    let (mut session, inner, policy) = two_musig_inputs();
    session
        .keys
        .add_words(&c, &test_words(TEST_SEEDS[0].0), TEST_SEEDS[0].1, None)
        .unwrap();
    let seed0 = Fingerprint::from(master_fp(0, "m/86'/1'/0'"));
    assert_eq!(offered(&session, &inner), vec![seed0]);
    // Seed 0 has a made-up partial on one input: still offered.
    let mut x = inner.clone();
    add_partial(&policy, &mut x.inputs[0], 0);
    assert_eq!(offered(&session, &x), vec![seed0]);
    // Mixed: a p2wpkh input signed by seed 0 and a MuSig input with no
    // partial of its own: offered; with a made-up one: offered too.
    let (session, mut mixed, seed0) = musig_beside_a_wpkh_of_a_held_seed();
    let kit = wpkh_kit(0, "One");
    let wp = faraday_bip::policy::WalletPolicy::parse_any(&c, &kit.descriptor).unwrap();
    let wspk = wp.script_at(&c, false, 0).unwrap();
    mixed.inputs[0].tap_key_sig = None;
    seed_signs_wpkh(&mut mixed, &wspk, 1);
    assert_eq!(offered(&session, &mixed), vec![seed0]);
    assert_eq!(state_in(&session, &mixed).0, vec![]);
    let mpol = faraday_core::testkit::musig_policy().unwrap();
    add_partial(&mpol, &mut mixed.inputs[0], 0);
    assert_eq!(state_in(&session, &mixed).0, vec![]);
    assert_eq!(offered(&session, &mixed), vec![seed0]);
}

// A MuSig2 partial signature counts its participant only when it verifies
// (BIP-327 `PartialSigVerify`) on every input still open.

mod musig_partials {
    use super::*;
    use faraday_bip::policy::WalletPolicy;
    use faraday_psbt::musig;

    const POLICY: &str = include_str!("../../../../tools/vectors/psbt/wallet-musig-regtest.policy");
    const ABANDON_FP: &str = "73c5da0a";
    const CORE_FP: &str = "04c04143";

    fn fixture(name: &str) -> Psbt {
        let c = faraday_core::testkit::compute();
        let path = format!(
            "{}/../../tools/vectors/psbt/{name}.psbt",
            env!("CARGO_MANIFEST_DIR")
        );
        let text = std::fs::read_to_string(path).unwrap();
        Psbt::parse_base64(&c, text.trim()).unwrap()
    }

    fn core_session() -> Session {
        let c = faraday_core::testkit::compute();
        let mut session = Session::on(Network::Regtest);
        session
            .add_policy(
                &c,
                "Core",
                WalletPolicy::parse(&c, POLICY).unwrap(),
                "Test wallet",
                faraday_core::wallet::Origin::Named,
            )
            .unwrap();
        session
    }

    fn names(fps: Vec<Fingerprint>) -> Vec<String> {
        let mut v: Vec<String> = fps.iter().map(|f| format!("{f}").to_lowercase()).collect();
        v.sort();
        v
    }

    /// Signers and masters still to sign of `psbt` in the Core wallet.
    fn state(psbt: &Psbt) -> (Vec<String>, Vec<String>) {
        let c = faraday_core::testkit::compute();
        let got = Spend::new(psbt.clone(), "t").signing(&c, &core_session(), None, None);
        (names(got.signers), names(got.to_sign))
    }

    fn strs(v: &[&str]) -> Vec<String> {
        let mut v: Vec<String> = v.iter().map(|s| (*s).to_string()).collect();
        v.sort();
        v
    }

    /// The partial signatures on `psbt` for the participants whose master
    /// is in `fps`, as this device having written them: what signing here
    /// notes.
    fn wrote(
        policy: &WalletPolicy,
        psbt: &Psbt,
        fps: &[Fingerprint],
    ) -> Vec<faraday_core::wallet::Written> {
        let mut out = Vec::new();
        for k in policy.keys() {
            if !k.fingerprint().is_some_and(|f| fps.contains(&f)) {
                continue;
            }
            for (i, x) in psbt.inner().inputs.iter().enumerate() {
                for e in musig::partial_sigs(x).unwrap() {
                    if e.participant == k.xpub().public_key {
                        out.push(faraday_core::wallet::Written {
                            input: i,
                            participant: e.participant,
                            partial: e.value,
                        });
                    }
                }
            }
        }
        out
    }

    /// The (participant, aggregate) key of the first partial signature.
    fn first_partial(psbt: &Psbt) -> musig::Entry<faraday_bip::musig::PartialSig> {
        musig::partial_sigs(&psbt.inner().inputs[0])
            .unwrap()
            .remove(0)
    }

    /// What `signing` says of `psbt` in the Core wallet, with `here` the
    /// masters this device signed for.
    fn full(psbt: &Psbt, here: &[&str]) -> faraday_core::wallet::Signing {
        use std::str::FromStr as _;
        let c = faraday_core::testkit::compute();
        let mut spend = Spend::new(psbt.clone(), "t");
        let fps: Vec<Fingerprint> = here
            .iter()
            .map(|f| {
                Fingerprint::from(faraday_bip::bitcoin::bip32::Fingerprint::from_str(f).unwrap())
            })
            .collect();
        spend.wrote = wrote(&WalletPolicy::parse(&c, POLICY).unwrap(), psbt, &fps);
        spend.signed_here = fps;
        spend.signing(&c, &core_session(), None, None)
    }

    // F220: a partial signature in the file is the file's word.
    #[test]
    fn a_cosigners_true_partial_is_in_the_file_unconfirmed_and_one_signed_here_counts() {
        for (file, signer, other) in [
            ("wallet-musig-core-first-signed", ABANDON_FP, CORE_FP),
            ("wallet-musig-device-first-core", CORE_FP, ABANDON_FP),
        ] {
            let psbt = fixture(file);
            // Opened again: nobody here wrote it, so it is the file's word.
            let got = full(&psbt, &[]);
            assert_eq!(names(got.signers), strs(&[]), "{file}");
            assert_eq!(names(got.in_file), strs(&[signer]), "{file}");
            assert_eq!(names(got.to_sign), strs(&[other]), "{file}");
            assert!(got.failed.is_empty(), "{file}");
            // This device wrote it: it counts.
            let got = full(&psbt, &[signer]);
            assert_eq!(names(got.signers), strs(&[signer]), "{file}");
            assert!(got.in_file.is_empty(), "{file}");
            assert_eq!(names(got.to_sign), strs(&[other]), "{file}");
        }
    }

    #[test]
    fn an_entry_for_a_participant_whose_secret_nonce_is_held_is_still_to_sign() {
        let c = faraday_core::testkit::compute();
        let psbt = fixture("wallet-musig-core-first-signed");
        let held = faraday_psbt::MusigSessionView {
            txid: psbt.unsigned_tx().compute_txid(),
            held: vec![(0, first_partial(&psbt).participant)],
        };
        let got = Spend::new(psbt, "t").signing(&c, &core_session(), None, Some(&held));
        assert!(got.signers.is_empty() && got.in_file.is_empty());
        assert_eq!(names(got.to_sign), strs(&[ABANDON_FP, CORE_FP]));
    }

    #[test]
    fn a_partial_signature_with_a_flipped_byte_counts_nobody_and_its_master_is_offered_sign() {
        let mut psbt = fixture("wallet-musig-device-first-core");
        let e = first_partial(&psbt);
        let mut bytes = e.value.serialize();
        bytes[31] ^= 1;
        let input = &mut psbt.inner_mut().inputs[0];
        musig::clear_partial_sigs(input);
        musig::write_partial_sig(
            input,
            &e.participant,
            &e.aggregate,
            &faraday_bip::musig::PartialSig::from_bytes(&bytes).unwrap(),
        );
        assert_eq!(state(&psbt), (vec![], strs(&[ABANDON_FP, CORE_FP])));
        assert_eq!(names(full(&psbt, &[]).failed), strs(&[CORE_FP]));
    }

    #[test]
    fn a_partial_signature_under_another_nonce_counts_nobody() {
        let mut psbt = fixture("wallet-musig-device-first-core");
        let e = first_partial(&psbt);
        let nonces = musig::pub_nonces(&psbt.inner().inputs[0]).unwrap();
        let other = nonces
            .iter()
            .find(|n| n.participant != e.participant)
            .unwrap();
        // The signer's nonce replaced by the other participant's.
        musig::write_pub_nonce(
            &mut psbt.inner_mut().inputs[0],
            &e.participant,
            &e.aggregate,
            &other.value,
        );
        assert_eq!(state(&psbt), (vec![], strs(&[ABANDON_FP, CORE_FP])));
    }

    #[test]
    fn a_partial_signature_over_another_message_counts_nobody() {
        let mut psbt = fixture("wallet-musig-device-first-core");
        psbt.inner_mut().unsigned_tx.lock_time = LockTime::from_consensus(1);
        assert_eq!(state(&psbt), (vec![], strs(&[ABANDON_FP, CORE_FP])));
    }

    #[test]
    fn a_partial_signature_with_its_nonce_field_gone_counts_nobody() {
        let mut psbt = fixture("wallet-musig-device-first-core");
        let e = first_partial(&psbt);
        let input = &mut psbt.inner_mut().inputs[0];
        input.unknown.retain(|k, _| {
            k.type_value != musig::IN_PUB_NONCE || k.key[..33] != e.participant.serialize()
        });
        assert_eq!(state(&psbt), (vec![], strs(&[ABANDON_FP, CORE_FP])));
    }

    #[test]
    fn a_true_partial_on_one_of_two_inputs_counts_nobody_and_on_both_counts() {
        let (c, s) = (
            faraday_core::testkit::compute(),
            faraday_core::testkit::session(),
        );
        let (one, both) = faraday_core::testkit::musig_two_inputs().unwrap();
        let fp = |n| format!("{}", faraday_core::testkit::seed_fingerprint(n)).to_lowercase();
        let mut session = s;
        session
            .keys
            .add_words(
                &c,
                &faraday_core::testkit::test_words(faraday_core::testkit::TEST_SEEDS[0].0),
                faraday_core::testkit::TEST_SEEDS[0].1,
                None,
            )
            .unwrap();
        let policy = faraday_core::testkit::musig_policy().unwrap();
        session
            .add_wallet(
                &c,
                "MuSig2",
                &policy.to_descriptor_checksummed(),
                "Test wallet",
                faraday_core::wallet::Origin::Named,
            )
            .unwrap();
        let of = |p: &Psbt| {
            let mut spend = Spend::new(p.clone(), "t");
            spend.signed_here = vec![faraday_core::testkit::seed_fingerprint(0)];
            spend.wrote = wrote(&policy, p, &spend.signed_here);
            let got = spend.signing(&c, &session, None, None);
            (names(got.signers), names(got.to_sign), names(got.in_file))
        };
        // This device signed both inputs; test key 2's partial is on the
        // first only, so it is still to sign, and on both it is the file's
        // word.
        assert_eq!(
            of(&one),
            (strs(&[&fp(0)]), strs(&[&fp(1), &fp(2)]), strs(&[]))
        );
        assert_eq!(
            of(&both),
            (strs(&[&fp(0)]), strs(&[&fp(2)]), strs(&[&fp(1)]))
        );
        // A partial on the second input only is still to sign.
        let key2 = policy
            .keys()
            .iter()
            .find(|k| k.fingerprint().map(|f| format!("{f}").to_lowercase()) == Some(fp(1)))
            .unwrap()
            .xpub()
            .public_key;
        let mut second = both.clone();
        let input = &mut second.inner_mut().inputs[0];
        let kept = musig::partial_sigs(input).unwrap();
        musig::clear_partial_sigs(input);
        for e in kept.iter().filter(|e| e.participant != key2) {
            musig::write_partial_sig(input, &e.participant, &e.aggregate, &e.value);
        }
        assert_eq!(
            of(&second),
            (strs(&[&fp(0)]), strs(&[&fp(1), &fp(2)]), strs(&[]))
        );
    }

    /// The three partial signatures of one round, all true, as the files
    /// of [`faraday_core::testkit::musig_rounds`] give them, and a session
    /// holding no key of the wallet.
    fn all_partials() -> (Session, Psbt) {
        all_partials_at(0)
    }

    /// [`all_partials`] for the wallet's output at `0/index`.
    fn all_partials_at(index: u32) -> (Session, Psbt) {
        let c = faraday_core::testkit::compute();
        let policy = faraday_core::testkit::musig_policy().unwrap();
        let mut session = faraday_core::testkit::session();
        session
            .add_wallet(
                &c,
                "MuSig2",
                &policy.to_descriptor_checksummed(),
                "Test wallet",
                faraday_core::wallet::Origin::Named,
            )
            .unwrap();
        let (_, _, files) = faraday_core::testkit::musig_rounds_at(index).unwrap();
        let mut psbt = files[0].1.clone();
        for e in musig::partial_sigs(&files[1].1.inner().inputs[0]).unwrap() {
            musig::write_partial_sig(
                &mut psbt.inner_mut().inputs[0],
                &e.participant,
                &e.aggregate,
                &e.value,
            );
        }
        assert_eq!(
            musig::partial_sigs(&psbt.inner().inputs[0]).unwrap().len(),
            3
        );
        (session, psbt)
    }

    #[test]
    fn every_true_partial_in_the_file_aggregates_with_no_key_used() {
        let c = faraday_core::testkit::compute();
        let (session, psbt) = all_partials();
        let mut spend = Spend::new(psbt, "t");
        let seen = session.inspect(&c, &spend.psbt);
        // Before: the file's word, nobody counted.
        let before = spend.signing(&c, &session, Some(&seen), None);
        assert!(before.signers.is_empty() && before.in_file.len() == 3);
        assert!(spend.aggregate_musig(&c, &seen));
        let now = spend.signing(&c, &session, None, None);
        assert_eq!(
            (now.signers.len(), now.in_file.len(), now.to_sign.len()),
            (3, 0, 0)
        );
    }

    #[test]
    fn a_false_partial_among_the_rest_does_not_aggregate_and_the_spend_stays_open() {
        let c = faraday_core::testkit::compute();
        let (session, mut psbt) = all_partials();
        let e = first_partial(&psbt);
        let mut bytes = e.value.serialize();
        bytes[31] ^= 1;
        musig::write_partial_sig(
            &mut psbt.inner_mut().inputs[0],
            &e.participant,
            &e.aggregate,
            &faraday_bip::musig::PartialSig::from_bytes(&bytes).unwrap(),
        );
        let mut spend = Spend::new(psbt, "t");
        let seen = session.inspect(&c, &spend.psbt);
        assert!(!spend.aggregate_musig(&c, &seen));
        assert!(spend.psbt.inner().inputs[0].tap_key_sig.is_none());
        let now = spend.signing(&c, &session, None, None);
        assert!(now.signers.is_empty() && now.failed.len() == 1 && now.in_file.len() == 2);
        assert_eq!(now.to_sign, now.failed);
    }

    // The partials below are false and verify all the same: made from the
    // file's public data with no key (`testkit::musig_forged`).

    /// Test seed `n`'s participant key and master fingerprint in the
    /// MuSig2 wallet.
    fn who(n: usize) -> (faraday_bip::bitcoin::secp256k1::PublicKey, Fingerprint) {
        let policy = faraday_core::testkit::musig_policy().unwrap();
        let k = &policy.keys()[n];
        (k.xpub().public_key, k.fingerprint().unwrap())
    }

    /// A session with the MuSig2 wallet, and test seed 0 when `hold`.
    fn wallet_session(hold: bool) -> Session {
        use faraday_core::testkit::{TEST_SEEDS, test_words};
        let c = faraday_core::testkit::compute();
        let mut session = faraday_core::testkit::session();
        if hold {
            session
                .keys
                .add_words(&c, &test_words(TEST_SEEDS[0].0), TEST_SEEDS[0].1, None)
                .unwrap();
        }
        session
            .add_wallet(
                &c,
                "MuSig2",
                &faraday_core::testkit::musig_policy()
                    .unwrap()
                    .to_descriptor_checksummed(),
                "Test wallet",
                faraday_core::wallet::Origin::Named,
            )
            .unwrap();
        session
    }

    fn seed() -> faraday_crypto::Zeroizing<[u8; 32]> {
        faraday_crypto::Zeroizing::new([7; 32])
    }

    /// Whether every partial of `forged` verifies, which the forgery is
    /// worth nothing without.
    fn forgery_verifies(session: &Session, psbt: &Psbt, forged: &[usize]) {
        let c = faraday_core::testkit::compute();
        let seen = session.inspect(&c, psbt);
        let round = seen.inputs[0].musig.as_ref().unwrap();
        let mut want: Vec<_> = forged.iter().map(|n| who(*n).0).collect();
        want.sort();
        let mut got = musig::verified_partials(&c, psbt, 0, round);
        got.sort();
        assert_eq!(got, want);
    }

    #[test]
    fn a_partial_forged_from_public_data_for_a_held_key_is_the_files_word_and_never_verified() {
        let c = faraday_core::testkit::compute();
        let session = wallet_session(true);
        let psbt = faraday_core::testkit::musig_forged(&[0]).unwrap();
        forgery_verifies(&session, &psbt, &[0]);
        let seen = session.inspect(&c, &psbt);
        let mut spend = Spend::new(psbt.clone(), "t");
        let got = spend.signing(&c, &session, Some(&seen), None);
        let fp0 = who(0).1;
        assert!(got.signers.is_empty(), "not counted");
        assert_eq!(got.in_file, vec![fp0], "In file, unconfirmed");
        assert!(!got.to_sign.contains(&fp0), "no Sign offered");
        assert!(got.failed.is_empty());
        // Signing does nothing for it.
        let mut again = psbt.clone();
        session.sign(&c, &mut again, &seed(), &mut None).unwrap();
        assert_eq!(again.to_bytes(), psbt.to_bytes());
        // It aggregates to nothing, and the spend stays open.
        let round = seen.inputs[0].musig.as_ref().unwrap();
        assert!(musig::aggregate(&c, &psbt, 0, round).is_none());
        assert!(!spend.aggregate_musig(&c, &seen));
        assert!(spend.psbt.inner().inputs[0].tap_key_sig.is_none());
        assert!(spend.finish().is_err());
    }

    #[test]
    fn a_forged_partial_for_a_participant_whose_nonce_is_held_is_overwritten_by_sign() {
        let c = faraday_core::testkit::compute();
        let session = wallet_session(true);
        // Round one: this device shares its nonce and holds the secret.
        let mut round_one = faraday_core::testkit::musig_unsigned().unwrap();
        let mut held = None;
        session
            .sign(&c, &mut round_one, &seed(), &mut held)
            .unwrap();
        let view = held.as_ref().unwrap().view();
        let (pk0, fp0) = who(0);
        let mine = musig::pub_nonces(&round_one.inner().inputs[0])
            .unwrap()
            .into_iter()
            .find(|n| n.participant == pk0)
            .unwrap()
            .value;
        // The file the online side hands back: a forged partial for it.
        let mut psbt = faraday_core::testkit::musig_forged(&[0]).unwrap();
        forgery_verifies(&session, &psbt, &[0]);
        let seen = session.inspect(&c, &psbt);
        let got = Spend::new(psbt.clone(), "t").signing(&c, &session, Some(&seen), Some(&view));
        assert!(got.signers.is_empty() && got.in_file.is_empty());
        assert!(got.to_sign.contains(&fp0), "Sign is offered");
        let forged = first_in(&psbt, &pk0);
        session.sign(&c, &mut psbt, &seed(), &mut held).unwrap();
        assert!(held.as_ref().unwrap().is_empty(), "no nonce held after");
        let after = first_in(&psbt, &pk0);
        assert_ne!(after, forged, "overwritten");
        let nonce = musig::pub_nonces(&psbt.inner().inputs[0])
            .unwrap()
            .into_iter()
            .find(|n| n.participant == pk0)
            .unwrap()
            .value;
        assert_eq!(nonce, mine, "under the nonce it held");
        // The partial it wrote verifies and counts as signed here.
        let seen = session.inspect(&c, &psbt);
        let mut spend = Spend::new(psbt, "t");
        spend.signed_here = vec![fp0];
        spend.wrote = wrote(
            &faraday_core::testkit::musig_policy().unwrap(),
            &spend.psbt,
            &[fp0],
        );
        let got = spend.signing(&c, &session, Some(&seen), None);
        assert_eq!(got.signers, vec![fp0]);
    }

    fn first_in(
        psbt: &Psbt,
        pk: &faraday_bip::bitcoin::secp256k1::PublicKey,
    ) -> faraday_bip::musig::PartialSig {
        musig::partial_sigs(&psbt.inner().inputs[0])
            .unwrap()
            .into_iter()
            .find(|e| e.participant == *pk)
            .unwrap()
            .value
    }

    #[test]
    fn every_participant_but_one_forged_reads_in_file_unconfirmed_and_does_not_aggregate() {
        let c = faraday_core::testkit::compute();
        let session = wallet_session(false);
        let mut psbt = faraday_core::testkit::musig_forged(&[0, 1]).unwrap();
        forgery_verifies(&session, &psbt, &[0, 1]);
        for with_last in [false, true] {
            if with_last {
                // The last one's entry is made up: all three are there.
                let (pk2, _) = who(2);
                let seen = session.inspect(&c, &psbt);
                let key = seen.inputs[0].musig.as_ref().unwrap().output_key;
                musig::write_partial_sig(
                    &mut psbt.inner_mut().inputs[0],
                    &pk2,
                    &key,
                    &faraday_bip::musig::PartialSig::from_bytes(&[1u8; 32]).unwrap(),
                );
            }
            let seen = session.inspect(&c, &psbt);
            let mut spend = Spend::new(psbt.clone(), "t");
            let got = spend.signing(&c, &session, Some(&seen), None);
            let mut forged = vec![who(0).1, who(1).1];
            forged.sort_by_key(|f| format!("{f}"));
            let mut in_file = got.in_file.clone();
            in_file.sort_by_key(|f| format!("{f}"));
            assert_eq!(in_file, forged);
            assert!(got.signers.is_empty());
            let round = seen.inputs[0].musig.as_ref().unwrap();
            assert!(musig::aggregate(&c, &psbt, 0, round).is_none());
            assert!(!spend.aggregate_musig(&c, &seen));
            assert!(spend.finish().is_err());
        }
    }

    #[test]
    fn a_true_partial_beside_another_kind_of_input_is_signed_here_and_otherwise_the_files_word() {
        let c = faraday_core::testkit::compute();
        let (session, mut inner, _) = musig_beside_a_wpkh_of_a_held_seed();
        inner.inputs[0].tap_key_sig = None;
        // A fee that is not a danger: 1,000 of 100,000.
        inner.unsigned_tx.output[0].value = Amount::from_sat(99_000);
        let mut psbt = Psbt::from(inner);
        let (pk0, fp0) = who(0);
        // Round one, then arbitrary nonces for the cosigners (nobody
        // signs for them here), then this device's partial.
        let mut held = None;
        session.sign(&c, &mut psbt, &seed(), &mut held).unwrap();
        let key = session.inspect(&c, &psbt).inputs[0]
            .musig
            .as_ref()
            .unwrap()
            .output_key;
        for (n, k) in [(1usize, 3u8), (2, 5)] {
            let mut b = [0u8; 32];
            b[31] = k;
            let p = SecretKey::from_slice(&b)
                .unwrap()
                .public_key(&Secp256k1::new());
            let nonce =
                faraday_bip::musig::PubNonce::from_bytes(&[p.serialize(), p.serialize()].concat())
                    .unwrap();
            musig::write_pub_nonce(&mut psbt.inner_mut().inputs[0], &who(n).0, &key, &nonce);
        }
        session.sign(&c, &mut psbt, &seed(), &mut held).unwrap();
        assert!(held.as_ref().unwrap().is_empty());
        let seen = session.inspect(&c, &psbt);
        let round = seen.inputs[0].musig.as_ref().unwrap();
        assert_eq!(musig::verified_partials(&c, &psbt, 0, round), vec![pk0]);
        let others = {
            let mut v = vec![who(1).1, who(2).1];
            v.sort_by_key(|f| format!("{f}"));
            v
        };
        let sorted = |mut v: Vec<Fingerprint>| {
            v.sort_by_key(|f| format!("{f}"));
            v
        };
        // Written here: counted on both inputs; the cosigners still sign.
        let mut spend = Spend::new(psbt.clone(), "t");
        spend.signed_here = vec![fp0];
        spend.wrote = wrote(
            &faraday_core::testkit::musig_policy().unwrap(),
            &psbt,
            &[fp0],
        );
        let got = spend.signing(&c, &session, Some(&seen), None);
        assert_eq!(got.signers, vec![fp0]);
        assert_eq!(sorted(got.to_sign), others);
        // Opened again: the file's word on the MuSig2 input, so not counted.
        let got = Spend::new(psbt, "t").signing(&c, &session, Some(&seen), None);
        assert!(got.signers.is_empty());
        assert_eq!(got.in_file, vec![fp0]);
        assert!(!got.to_sign.contains(&fp0));
    }

    #[test]
    fn an_output_key_with_an_odd_y_aggregates_from_true_partials() {
        use faraday_bip::bitcoin::secp256k1::Parity;
        let c = faraday_core::testkit::compute();
        let session = wallet_session(false);
        let index = (0u32..40)
            .find(|i| {
                let p = faraday_core::testkit::musig_unsigned_at(*i).unwrap();
                session.inspect(&c, &p).inputs[0]
                    .musig
                    .as_ref()
                    .unwrap()
                    .output_key
                    .x_only_public_key()
                    .1
                    == Parity::Odd
            })
            .expect("an odd output key among forty");
        let (session, psbt) = all_partials_at(index);
        let mut spend = Spend::new(psbt, "t");
        let seen = session.inspect(&c, &spend.psbt);
        let round = seen.inputs[0].musig.as_ref().unwrap();
        assert_eq!(round.index, index);
        assert_eq!(musig::verified_partials(&c, &spend.psbt, 0, round).len(), 3);
        assert!(spend.aggregate_musig(&c, &seen));
        assert_eq!(spend.finish(), Ok(()));
    }

    #[test]
    fn a_partial_this_device_did_not_write_reads_in_file_though_its_master_signed_another_input() {
        let c = faraday_core::testkit::compute();
        let session = wallet_session(true);
        let (one, _) = faraday_core::testkit::musig_two_inputs().unwrap();
        let (_, fp0) = who(0);
        // This device wrote its partial on the first input; the second
        // carries one for the same master that came in a file.
        let mut spend = Spend::new(one.clone(), "t");
        spend.signed_here = vec![fp0];
        spend.wrote = wrote(
            &faraday_core::testkit::musig_policy().unwrap(),
            &one,
            &[fp0],
        )
        .into_iter()
        .filter(|w| w.input == 0)
        .collect();
        assert_eq!(spend.wrote.len(), 1);
        let got = spend.signing(&c, &session, None, None);
        assert!(!got.signers.contains(&fp0), "not signed on every input");
        assert!(got.in_file.contains(&fp0), "In file, unconfirmed");
        assert!(!got.to_sign.contains(&fp0));
    }

    #[test]
    fn a_collect_that_replaces_this_devices_entry_with_a_forged_one_reads_in_file() {
        let c = faraday_core::testkit::compute();
        let session = wallet_session(true);
        let psbt = faraday_core::testkit::musig_forged(&[0]).unwrap();
        forgery_verifies(&session, &psbt, &[0]);
        let (pk0, fp0) = who(0);
        // What this device wrote is not what the file now holds.
        let mut spend = Spend::new(psbt, "t");
        spend.signed_here = vec![fp0];
        spend.wrote = vec![faraday_core::wallet::Written {
            input: 0,
            participant: pk0,
            partial: faraday_bip::musig::PartialSig::from_bytes(&[2u8; 32]).unwrap(),
        }];
        let got = spend.signing(&c, &session, None, None);
        assert!(got.signers.is_empty());
        assert_eq!(got.in_file, vec![fp0]);
        assert!(!got.to_sign.contains(&fp0));
    }

    #[test]
    fn a_false_key_path_signature_beside_every_true_partial_is_replaced_by_the_aggregate() {
        let c = faraday_core::testkit::compute();
        let (session, mut psbt) = all_partials();
        psbt.inner_mut().inputs[0].tap_key_sig = Some(faraday_bip::bitcoin::taproot::Signature {
            signature: faraday_bip::bitcoin::secp256k1::schnorr::Signature::from_slice(&[3u8; 64])
                .unwrap(),
            sighash_type: TapSighashType::Default,
        });
        let mut spend = Spend::new(psbt, "t");
        let seen = session.inspect(&c, &spend.psbt);
        assert!(spend.aggregate_musig(&c, &seen));
        assert_eq!(spend.finish(), Ok(()));
    }

    #[test]
    fn an_entry_under_a_foreign_aggregate_key_reads_as_none_and_its_master_is_offered_sign() {
        let c = faraday_core::testkit::compute();
        let (session, mut psbt) = all_partials();
        let (pk1, fp1) = who(1);
        let e = musig::partial_sigs(&psbt.inner().inputs[0])
            .unwrap()
            .into_iter()
            .find(|e| e.participant == pk1)
            .unwrap();
        let input = &mut psbt.inner_mut().inputs[0];
        let rest: Vec<_> = musig::partial_sigs(input)
            .unwrap()
            .into_iter()
            .filter(|x| x.participant != pk1)
            .collect();
        musig::clear_partial_sigs(input);
        for x in &rest {
            musig::write_partial_sig(input, &x.participant, &x.aggregate, &x.value);
        }
        let foreign = who(2).0;
        musig::write_partial_sig(input, &pk1, &foreign, &e.value);
        let got = Spend::new(psbt, "t").signing(&c, &session, None, None);
        assert!(got.failed.is_empty(), "no entry for it is read");
        assert!(got.to_sign.contains(&fp1), "Sign is offered");
        assert_eq!(got.in_file.len(), 2);
    }

    /// An app with no key and the files `files` in its Inbox.
    fn app_with(files: Vec<(String, Vec<u8>)>) -> faraday_core::Faraday {
        let mut app = faraday_core::testkit::started();
        app.storage(faraday_core::StorageEvent::Restored {
            inbox: files,
            outbox: Vec::new(),
            kept: Vec::new(),
        });
        app
    }

    fn inbox_at(app: &faraday_core::Faraday, name: &str) -> usize {
        app.inbox().iter().position(|i| i.name == name).unwrap()
    }

    fn wallet_file() -> (String, Vec<u8>) {
        let policy = faraday_core::testkit::musig_policy().unwrap();
        (
            "wallet.txt".to_string(),
            policy.to_descriptor_checksummed().into_bytes(),
        )
    }

    #[test]
    fn an_honest_file_opened_before_its_wallet_is_registered_completes_when_it_is() {
        use faraday_core::Action;
        let (_, psbt) = all_partials();
        let mut app = app_with(vec![
            ("all.psbt".to_string(), psbt.to_bytes()),
            wallet_file(),
        ]);
        app.press(Action::StartSpend(inbox_at(&app, "all.psbt")));
        assert!(!app.spend().unwrap().complete, "no wallet, no aggregate");
        app.press(Action::LoadWallet(inbox_at(&app, "wallet.txt")));
        let s = app.spend().unwrap();
        assert!(s.complete, "aggregated once the wallet is known");
        assert_eq!(
            (s.in_file.len(), s.to_sign.len(), s.signers.len()),
            (0, 0, 3)
        );
    }

    #[test]
    fn the_count_and_the_foot_read_what_is_in_the_file_and_what_is_still_to_come() {
        use faraday_core::Action;
        let forged = faraday_core::testkit::musig_forged(&[0, 1]).unwrap();
        let mut app = app_with(vec![
            ("two.psbt".to_string(), forged.to_bytes()),
            wallet_file(),
        ]);
        app.press(Action::LoadWallet(inbox_at(&app, "wallet.txt")));
        crate::common::add_key(&mut app, "bacon");
        app.press(Action::StartSpend(inbox_at(&app, "two.psbt")));
        for n in [1u8, 2, 3] {
            app.press(Action::StepNext(n));
        }
        let drawn = app.drawn_texts().join("\n");
        assert!(
            drawn.contains("2 of 3 in the file · 1 more needed"),
            "{drawn}"
        );
        assert!(drawn.contains("Collect 1 signature"), "{drawn}");
    }

    #[test]
    fn a_collect_that_brings_the_round_in_uses_the_file_after_the_merge() {
        use faraday_core::Action;
        let (_, full) = all_partials();
        // The spend as opened lacks the participants field; the file has it.
        let mut bare = full.clone();
        bare.inner_mut().inputs[0]
            .unknown
            .retain(|k, _| k.type_value != musig::IN_PARTICIPANT_PUBKEYS);
        let mut app = app_with(vec![
            ("bare.psbt".to_string(), bare.to_bytes()),
            ("full.psbt".to_string(), full.to_bytes()),
            wallet_file(),
        ]);
        app.press(Action::LoadWallet(inbox_at(&app, "wallet.txt")));
        app.press(Action::StartSpend(inbox_at(&app, "bare.psbt")));
        assert!(!app.spend().unwrap().complete);
        app.press(Action::Collect(inbox_at(&app, "full.psbt")));
        assert!(app.spend().unwrap().complete, "aggregated after the merge");
    }

    #[test]
    fn collecting_a_cosigners_partial_names_whose_it_added_and_the_same_file_again_adds_nothing() {
        let c = faraday_core::testkit::compute();
        let session = wallet_session(false);
        let (unsigned, _, partials) = faraday_core::testkit::musig_rounds().unwrap();
        let mut spend = Spend::new(unsigned, "t");
        let name = |f: &Fingerprint| format!("{f}").to_lowercase();
        // Each file carries the last signer's partial and one cosigner's.
        let first = spend
            .collect(&c, &session, &partials[0].1.to_bytes(), "a.psbt")
            .unwrap();
        assert_eq!(first.len(), 2);
        assert!(
            first
                .iter()
                .any(|f| name(f) == partials[0].0.to_lowercase())
        );
        let again = spend
            .collect(&c, &session, &partials[0].1.to_bytes(), "a.psbt")
            .unwrap();
        assert!(again.is_empty());
        let second = spend
            .collect(&c, &session, &partials[1].1.to_bytes(), "b.psbt")
            .unwrap();
        assert_eq!(second.len(), 1);
        assert_eq!(name(&second[0]), partials[1].0.to_lowercase());
    }

    #[test]
    fn two_forms_of_the_aggregate_key_for_one_participant_read_as_the_signer_reads() {
        let c = faraday_core::testkit::compute();
        let (session, psbt) = all_partials();
        let seen = session.inspect(&c, &psbt);
        let round = seen.inputs[0].musig.as_ref().unwrap();
        let (pk1, fp1) = who(1);
        let true_one = first_in(&psbt, &pk1);
        let mut junk = true_one.serialize();
        junk[31] ^= 1;
        let junk = faraday_bip::musig::PartialSig::from_bytes(&junk).unwrap();
        // A false entry under the root aggregate beside the true one under
        // the output key: the output key's counts, the spend completes.
        let mut beside = psbt.clone();
        musig::write_partial_sig(
            &mut beside.inner_mut().inputs[0],
            &pk1,
            &round.root_aggregate,
            &junk,
        );
        let got = Spend::new(beside.clone(), "t").signing(&c, &session, None, None);
        assert!(got.failed.is_empty() && got.in_file.len() == 3);
        let mut spend = Spend::new(beside, "t");
        assert!(spend.aggregate_musig(&c, &seen));
        // The false one under the output key hides the true one under the
        // root aggregate: it is the one that counts, and it fails.
        let mut hidden = psbt.clone();
        musig::write_partial_sig(
            &mut hidden.inner_mut().inputs[0],
            &pk1,
            &round.root_aggregate,
            &true_one,
        );
        musig::write_partial_sig(
            &mut hidden.inner_mut().inputs[0],
            &pk1,
            &round.output_key,
            &junk,
        );
        let got = Spend::new(hidden.clone(), "t").signing(&c, &session, None, None);
        assert_eq!(got.failed, vec![fp1]);
        assert!(got.to_sign.contains(&fp1));
        assert!(!Spend::new(hidden, "t").aggregate_musig(&c, &seen));
    }

    #[test]
    fn a_spend_with_every_input_final_has_nobody_to_sign() {
        let mut psbt = fixture("wallet-musig-device-first-signed");
        psbt.inner_mut().inputs[0].final_script_witness = Some(Witness::new());
        assert_eq!(state(&psbt), (vec![], vec![]));
    }
}
