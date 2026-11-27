//! What a signature row says about whose it is. A key origin is the online
//! side's word: a row takes a master only for a key of the script or leaf
//! being spent, and a key-path row only through the internal key its
//! output key is tweaked from (BIP-341). A final witness that fails, or
//! holds no signature, leaves nothing beside it valid, and says so in one
//! line.
//!
//! The signer is rust-bitcoin's secp256k1 over rust-bitcoin's sighash.

use bitcoin::bip32::{DerivationPath, Fingerprint};
use bitcoin::key::TapTweak;
use bitcoin::opcodes::all::OP_CHECKSIG;
use bitcoin::secp256k1::{Keypair, Message, Secp256k1, SecretKey};
use bitcoin::sighash::{EcdsaSighashType, Prevouts, SighashCache, TapSighashType};
use bitcoin::taproot::{LeafVersion, TapLeafHash, TaprootBuilder};
use bitcoin::{
    Amount, OutPoint, PublicKey, ScriptBuf, Sequence, Transaction, TxIn, TxOut, Witness,
    absolute::LockTime, ecdsa, hashes::Hash, script::Builder, transaction::Version,
};
use faraday_bip::keys::Network;
use faraday_crypto::work::Compute;
use faraday_psbt::verify::{
    SignaturePresent, Verdict, final_invalid, not_evaluated, verify_signatures, witness_mismatch,
};
use faraday_psbt::{Context, Detail, Psbt, WarningKind, inspect};
use std::str::FromStr;

fn pair(n: u8) -> Keypair {
    Keypair::from_secret_key(&Secp256k1::new(), &SecretKey::from_slice(&[n; 32]).unwrap())
}

fn fp(n: u8) -> Fingerprint {
    Fingerprint::from([n; 4])
}

fn path() -> DerivationPath {
    DerivationPath::from_str("m/48'/1'/0'/2'/0/0").unwrap()
}

fn rows(inner: &bitcoin::Psbt) -> Vec<SignaturePresent> {
    let ctx = Context {
        network: Network::Regtest,
        keys: &[],
        wallets: &[],
        musig_session: None,
    };
    verify_signatures(&Psbt::from(inner.clone()), &ctx)
        .into_iter()
        .flat_map(|i| i.signatures)
        .collect()
}

fn one_input(spk: ScriptBuf) -> (bitcoin::Psbt, Vec<TxOut>) {
    let prevs = vec![TxOut {
        value: Amount::from_sat(50_000),
        script_pubkey: spk,
    }];
    let tx = Transaction {
        version: Version::TWO,
        lock_time: LockTime::ZERO,
        input: vec![TxIn {
            previous_output: OutPoint::new(bitcoin::Txid::from_byte_array([1; 32]), 0),
            script_sig: ScriptBuf::new(),
            sequence: Sequence::MAX,
            witness: Witness::new(),
        }],
        output: vec![TxOut {
            value: Amount::from_sat(10_000),
            script_pubkey: ScriptBuf::new_p2tr(
                &Secp256k1::new(),
                pair(99).x_only_public_key().0,
                None,
            ),
        }],
    };
    let mut inner = bitcoin::Psbt::from_unsigned_tx(tx).unwrap();
    inner.inputs[0].witness_utxo = Some(prevs[0].clone());
    (inner, prevs)
}

fn schnorr(inner: &bitcoin::Psbt, hash: [u8; 32], pair: &Keypair) -> bitcoin::taproot::Signature {
    let _ = inner;
    bitcoin::taproot::Signature {
        signature: Secp256k1::new().sign_schnorr_no_aux_rand(&Message::from_digest(hash), pair),
        sighash_type: TapSighashType::Default,
    }
}

/// A one-leaf tree `<A> CHECKSIG` with internal key `internal`; the leaf's
/// key A has no origin of its own.
struct Leaf {
    inner: bitcoin::Psbt,
    prevs: Vec<TxOut>,
    leaf: TapLeafHash,
    script: ScriptBuf,
    control: Vec<u8>,
}

fn leaf(internal: &Keypair, a: &Keypair) -> Leaf {
    let script = Builder::new()
        .push_x_only_key(&a.x_only_public_key().0)
        .push_opcode(OP_CHECKSIG)
        .into_script();
    tree(internal, script)
}

fn tree(internal: &Keypair, script: ScriptBuf) -> Leaf {
    build(internal, script, LeafVersion::TapScript, None)
}

/// A tree of `script` at `version`, beside `sibling` (a second leaf at
/// depth one, which makes the control block 65 bytes) or alone.
fn build(
    internal: &Keypair,
    script: ScriptBuf,
    version: LeafVersion,
    sibling: Option<ScriptBuf>,
) -> Leaf {
    let secp = Secp256k1::new();
    let depth = u8::from(sibling.is_some());
    let mut builder = TaprootBuilder::new()
        .add_leaf_with_ver(depth, script.clone(), version)
        .unwrap();
    if let Some(sibling) = sibling {
        builder = builder.add_leaf(1, sibling).unwrap();
    }
    let info = builder
        .finalize(&secp, internal.x_only_public_key().0)
        .unwrap();
    let (mut inner, prevs) = one_input(ScriptBuf::new_p2tr_tweaked(info.output_key()));
    let leaf = TapLeafHash::from_script(&script, version);
    let cb = info.control_block(&(script.clone(), version)).unwrap();
    let input = &mut inner.inputs[0];
    input.tap_internal_key = Some(internal.x_only_public_key().0);
    input.tap_merkle_root = info.merkle_root();
    input
        .tap_scripts
        .insert(cb.clone(), (script.clone(), version));
    input
        .tap_key_origins
        .insert(internal.x_only_public_key().0, (vec![], (fp(70), path())));
    Leaf {
        inner,
        prevs,
        leaf,
        script,
        control: cb.serialize(),
    }
}

impl Leaf {
    fn sign(&self, k: &Keypair) -> bitcoin::taproot::Signature {
        let h = SighashCache::new(&self.inner.unsigned_tx)
            .taproot_script_spend_signature_hash(
                0,
                &Prevouts::All(&self.prevs),
                self.leaf,
                TapSighashType::Default,
            )
            .unwrap();
        schnorr(&self.inner, h.to_byte_array(), k)
    }
}

#[test]
fn a_leaf_key_with_no_origin_takes_no_master_but_one_with_its_own_origin_does() {
    let (i, a) = (pair(7), pair(5));
    let mut t = leaf(&i, &a);
    let ax = a.x_only_public_key().0;
    let s = t.sign(&a);
    t.inner.inputs[0].tap_script_sigs.insert((ax, t.leaf), s);
    let r = rows(&t.inner);
    assert_eq!((r.len(), r[0].verdict), (1, Verdict::Valid));
    assert_eq!(r[0].fingerprint, None);
    t.inner.inputs[0]
        .tap_key_origins
        .insert(ax, (vec![t.leaf], (fp(5), path())));
    assert_eq!(rows(&t.inner)[0].fingerprint, Some(fp(5).into()));
}

#[test]
fn an_origin_under_a_key_the_leaf_does_not_name_is_not_a_master() {
    let (i, a, evil) = (pair(7), pair(5), pair(6));
    let mut t = leaf(&i, &a);
    let ex = evil.x_only_public_key().0;
    t.inner.inputs[0]
        .tap_key_origins
        .insert(ex, (vec![t.leaf], (fp(9), path())));
    let s = t.sign(&evil);
    t.inner.inputs[0].tap_script_sigs.insert((ex, t.leaf), s);
    let r = rows(&t.inner);
    assert_eq!(r[0].verdict, Verdict::Valid);
    assert_eq!((r[0].fingerprint, r[0].ours), (None, false));
}

#[test]
fn a_p2wsh_row_under_a_key_the_script_does_not_name_is_not_a_master() {
    let (a, evil) = (pair(1), pair(5));
    let (apk, epk) = (
        PublicKey::new(a.public_key()),
        PublicKey::new(evil.public_key()),
    );
    let script = Builder::new()
        .push_int(1)
        .push_key(&apk)
        .push_int(1)
        .push_opcode(bitcoin::opcodes::all::OP_CHECKMULTISIG)
        .into_script();
    let (mut inner, _) = one_input(ScriptBuf::new_p2wsh(&script.wscript_hash()));
    inner.inputs[0].witness_script = Some(script.clone());
    inner.inputs[0]
        .bip32_derivation
        .insert(apk.inner, (fp(1), path()));
    inner.inputs[0]
        .bip32_derivation
        .insert(epk.inner, (fp(9), path()));
    for k in [&a, &evil] {
        let h = SighashCache::new(&inner.unsigned_tx)
            .p2wsh_signature_hash(0, &script, Amount::from_sat(50_000), EcdsaSighashType::All)
            .unwrap();
        let sig = ecdsa::Signature::sighash_all(
            Secp256k1::new().sign_ecdsa(&Message::from_digest(h.to_byte_array()), &k.secret_key()),
        );
        inner.inputs[0]
            .partial_sigs
            .insert(PublicKey::new(k.public_key()), sig);
    }
    let by: Vec<_> = rows(&inner)
        .into_iter()
        .map(|r| (r.fingerprint, r.verdict))
        .collect();
    assert_eq!(
        by.iter().filter(|(f, _)| *f == Some(fp(1).into())).count(),
        1
    );
    assert_eq!(by.iter().filter(|(f, _)| f.is_none()).count(), 1);
    assert!(by.iter().all(|(_, v)| *v == Verdict::Valid));
}

#[test]
fn a_key_path_row_names_the_internal_key_only_when_the_output_is_its_tweak() {
    let secp = Secp256k1::new();
    let i = pair(7);
    let ix = i.x_only_public_key().0;
    for merkle in [
        None,
        Some(
            TaprootBuilder::new()
                .add_leaf(0, ScriptBuf::from_bytes(vec![0x51]))
                .unwrap()
                .finalize(&secp, ix)
                .unwrap()
                .merkle_root()
                .unwrap(),
        ),
    ] {
        let tweaked = i.tap_tweak(&secp, merkle);
        let (mut inner, prevs) = one_input(ScriptBuf::new_p2tr_tweaked(tweaked.public_parts().0));
        let input = &mut inner.inputs[0];
        input.tap_internal_key = Some(ix);
        input.tap_merkle_root = merkle;
        input.tap_key_origins.insert(ix, (vec![], (fp(7), path())));
        let h = SighashCache::new(&inner.unsigned_tx)
            .taproot_key_spend_signature_hash(0, &Prevouts::All(&prevs), TapSighashType::Default)
            .unwrap();
        inner.inputs[0].tap_key_sig =
            Some(schnorr(&inner, h.to_byte_array(), &tweaked.to_keypair()));
        let r = rows(&inner);
        assert_eq!(
            (r[0].verdict, r[0].fingerprint),
            (Verdict::Valid, Some(fp(7).into()))
        );
        // The merkle root the input states is not the output's: no master.
        if merkle.is_some() {
            inner.inputs[0].tap_merkle_root = None;
            assert_eq!(rows(&inner)[0].fingerprint, None);
        }
    }
}

#[test]
fn a_partial_row_beside_a_failing_final_witness_stays_as_it_verifies_and_the_final_row_is_not_valid()
 {
    let (i, a) = (pair(7), pair(5));
    let mut t = leaf(&i, &a);
    let ax = a.x_only_public_key().0;
    t.inner.inputs[0]
        .tap_key_origins
        .insert(ax, (vec![t.leaf], (fp(5), path())));
    let s = t.sign(&a);
    t.inner.inputs[0].tap_script_sigs.insert((ax, t.leaf), s);
    assert_eq!(rows(&t.inner)[0].verdict, Verdict::Valid);
    let mut control = t.control.clone();
    control[2] ^= 1;
    t.inner.inputs[0].final_script_witness = Some(Witness::from_slice(&[
        s.to_vec(),
        t.script.to_bytes(),
        control,
    ]));
    assert!(witness_mismatch(&t.inner, 0));
    // The final rows are not valid; the partial one stays as it verifies
    // (that it is not counted is the signed count's, in `faraday-core`).
    assert!(
        rows(&t.inner)
            .iter()
            .filter(|r| r.finalized)
            .all(|r| r.verdict != Verdict::Valid)
    );
    assert!(
        rows(&t.inner)
            .iter()
            .filter(|r| !r.finalized)
            .all(|r| r.verdict == Verdict::Valid)
    );
}

#[test]
fn an_empty_or_junk_key_path_witness_is_a_mismatch_and_not_a_signature() {
    let i = pair(7);
    let tweaked = i.tap_tweak(&Secp256k1::new(), None);
    let (mut inner, _) = one_input(ScriptBuf::new_p2tr_tweaked(tweaked.public_parts().0));
    for w in [Witness::new(), Witness::from_slice(&[vec![3u8; 7]])] {
        inner.inputs[0].final_script_witness = Some(w);
        assert!(witness_mismatch(&inner, 0));
        assert!(rows(&inner).iter().all(|r| r.verdict != Verdict::Valid));
    }
}

#[test]
fn the_witness_line_alone_speaks_and_a_keyless_signature_has_its_own() {
    let c = Compute::unchecked();
    let ctx = Context {
        network: Network::Regtest,
        keys: &[],
        wallets: &[],
        musig_session: None,
    };
    let i = pair(7);
    let tweaked = i.tap_tweak(&Secp256k1::new(), None);
    let (mut inner, _) = one_input(ScriptBuf::new_p2tr_tweaked(tweaked.public_parts().0));
    // A 64-byte key-path witness that is no signature of this transaction
    // under a witness that confirms: no mismatch, so the signature line.
    inner.inputs[0].final_script_witness = Some(Witness::from_slice(&[vec![3u8; 64]]));
    let insp = inspect(&c, &Psbt::from(inner.clone()), &ctx);
    assert!(
        insp.warnings
            .iter()
            .any(|w| matches!(w.detail, Detail::SignatureInvalid { .. }))
            || insp
                .warnings
                .iter()
                .any(|w| matches!(w.detail, Detail::SignatureInvalidUnkeyed { .. }))
    );
    assert!(
        !insp
            .warnings
            .iter()
            .any(|w| matches!(w.kind, WarningKind::WitnessMismatch { .. }))
    );
    // A junk witness: the witness line, and no line about a signature.
    inner.inputs[0].final_script_witness = Some(Witness::from_slice(&[vec![3u8; 7]]));
    let insp = inspect(&c, &Psbt::from(inner.clone()), &ctx);
    assert!(
        insp.warnings
            .iter()
            .any(|w| matches!(w.kind, WarningKind::WitnessMismatch { .. }))
    );
    assert!(!insp.warnings.iter().any(|w| matches!(
        w.detail,
        Detail::SignatureInvalid { .. } | Detail::SignatureInvalidUnkeyed { .. }
    )));
    // The same with an annex after it, and the junk alone: the witness line
    // and nothing else (no unchecked line, no "signed with another sighash"
    // line, and no key left blank).
    for w in [
        vec![vec![3u8; 7], vec![0x50, 1, 2]],
        vec![vec![0x50, 1, 2]],
        vec![vec![3u8; 7]],
    ] {
        let mut again = inner.clone();
        again.inputs[0].final_script_witness = Some(Witness::from_slice(&w));
        let insp = inspect(&c, &Psbt::from(again), &ctx);
        assert!(
            insp.warnings
                .iter()
                .any(|w| matches!(w.kind, WarningKind::WitnessMismatch { .. }))
        );
        assert!(!insp.warnings.iter().any(|w| matches!(
            w.kind,
            WarningKind::SignatureUnchecked { .. }
                | WarningKind::SighashMismatch { .. }
                | WarningKind::InvalidSignature { .. }
        )));
    }
}

#[test]
fn a_final_multisig_signature_no_key_of_the_script_made_names_no_key() {
    let (a, evil) = (pair(1), pair(5));
    let apk = PublicKey::new(a.public_key());
    let script = Builder::new()
        .push_int(1)
        .push_key(&apk)
        .push_int(1)
        .push_opcode(bitcoin::opcodes::all::OP_CHECKMULTISIG)
        .into_script();
    let (mut inner, _) = one_input(ScriptBuf::new_p2wsh(&script.wscript_hash()));
    let h = SighashCache::new(&inner.unsigned_tx)
        .p2wsh_signature_hash(0, &script, Amount::from_sat(50_000), EcdsaSighashType::All)
        .unwrap();
    let sig = ecdsa::Signature::sighash_all(
        Secp256k1::new().sign_ecdsa(&Message::from_digest(h.to_byte_array()), &evil.secret_key()),
    );
    inner.inputs[0].final_script_witness = Some(Witness::from_slice(&[
        vec![],
        sig.to_vec(),
        script.to_bytes(),
    ]));
    let ctx = Context {
        network: Network::Regtest,
        keys: &[],
        wallets: &[],
        musig_session: None,
    };
    let insp = inspect(&Compute::unchecked(), &Psbt::from(inner), &ctx);
    assert!(
        insp.warnings
            .iter()
            .any(|w| matches!(w.detail, Detail::SignatureInvalidUnkeyed { input: 0 }))
    );
    assert!(
        !insp
            .warnings
            .iter()
            .any(|w| matches!(w.detail, Detail::SignatureInvalid { .. }))
    );
}

fn final_witness(l: &mut Leaf, elements: &[Vec<u8>]) {
    l.inner.inputs[0].final_script_witness = Some(Witness::from_slice(elements));
}

#[test]
fn a_leaf_the_output_does_not_hold_names_no_master() {
    let (internal, a, evil) = (pair(7), pair(1), pair(5));
    let mut l = leaf(&internal, &a);
    // A leaf of the signer's own filing, under a control block that does not
    // commit to the output key, with the honest master as its origin.
    let evil_script = Builder::new()
        .push_x_only_key(&evil.x_only_public_key().0)
        .push_opcode(OP_CHECKSIG)
        .into_script();
    let other = tree(&pair(8), evil_script.clone());
    let (cb, (script, version)) = other.inner.inputs[0].tap_scripts.iter().next().unwrap();
    l.inner.inputs[0]
        .tap_scripts
        .insert(cb.clone(), (script.clone(), *version));
    l.inner.inputs[0].tap_key_origins.insert(
        evil.x_only_public_key().0,
        (vec![other.leaf], (fp(1), path())),
    );
    let hash = SighashCache::new(&l.inner.unsigned_tx)
        .taproot_script_spend_signature_hash(
            0,
            &Prevouts::All(&l.prevs),
            other.leaf,
            TapSighashType::Default,
        )
        .unwrap();
    let sig = schnorr(&l.inner, hash.to_byte_array(), &evil);
    l.inner.inputs[0]
        .tap_script_sigs
        .insert((evil.x_only_public_key().0, other.leaf), sig);
    assert!(rows(&l.inner).iter().all(|r| r.fingerprint.is_none()));
    // A plain key-hash output names no leaf at all.
    let mut plain = l.inner.clone();
    plain.inputs[0].witness_utxo = Some(TxOut {
        value: Amount::from_sat(50_000),
        script_pubkey: ScriptBuf::new_p2wpkh(
            &PublicKey::new(a.public_key()).wpubkey_hash().unwrap(),
        ),
    });
    assert!(rows(&plain).iter().all(|r| r.fingerprint.is_none()));
}

#[test]
fn a_taproot_witness_without_a_signature_is_a_mismatch_with_or_without_an_annex() {
    let i = pair(7);
    let tweaked = i.tap_tweak(&Secp256k1::new(), None);
    let (mut inner, _) = one_input(ScriptBuf::new_p2tr_tweaked(tweaked.public_parts().0));
    let mut cases: Vec<Option<Witness>> = vec![None, Some(Witness::new())];
    for w in [
        vec![vec![3u8; 7]],
        vec![vec![3u8; 65]],
        vec![vec![3u8; 7], vec![0x50, 9]],
        vec![vec![3u8; 65], vec![0x50, 9]],
        vec![vec![0x50, 9]],
    ] {
        let mut w = w;
        if w[0].len() == 65 {
            w[0][64] = 0;
        }
        cases.push(Some(Witness::from_slice(&w)));
    }
    inner.inputs[0].final_script_sig = Some(ScriptBuf::new());
    for w in cases {
        inner.inputs[0].final_script_witness = w;
        assert!(witness_mismatch(&inner, 0), "{:?}", inner.inputs[0]);
    }
}

#[test]
fn a_true_taproot_signature_over_an_annex_is_valid_and_a_forged_one_is_refused() {
    let secp = Secp256k1::new();
    let annex = vec![0x50u8, 1, 2, 3];
    let i = pair(7);
    let tweaked = i.tap_tweak(&secp, None);
    let (mut inner, prevs) = one_input(ScriptBuf::new_p2tr_tweaked(tweaked.public_parts().0));
    let hash = SighashCache::new(&inner.unsigned_tx)
        .taproot_signature_hash(
            0,
            &Prevouts::All(&prevs),
            Some(bitcoin::sighash::Annex::new(&annex).unwrap()),
            None,
            TapSighashType::Default,
        )
        .unwrap();
    let sig = secp.sign_schnorr_no_aux_rand(
        &Message::from_digest(hash.to_byte_array()),
        &tweaked.to_keypair(),
    );
    inner.inputs[0].final_script_witness =
        Some(Witness::from_slice(&[sig.as_ref().to_vec(), annex.clone()]));
    assert!(rows(&inner).iter().all(|r| r.verdict == Verdict::Valid));
    assert!(!final_invalid(&inner, 0));
    // Not a signature of this transaction: refused.
    inner.inputs[0].final_script_witness = Some(Witness::from_slice(&[vec![3u8; 64]]));
    assert!(final_invalid(&inner, 0));
}

#[test]
fn a_script_path_signature_is_judged_by_the_interpreter_and_a_data_element_is_not_refused() {
    let (internal, a) = (pair(7), pair(1));
    let mut l = leaf(&internal, &a);
    let good = l.sign(&a).to_vec();
    let (sc, cb) = (l.script.to_bytes(), l.control.clone());
    final_witness(&mut l, &[good.clone(), sc.clone(), cb.clone()]);
    assert!(!final_invalid(&l.inner, 0));
    let mut bad = good;
    bad[10] ^= 1;
    final_witness(&mut l, &[bad, sc, cb]);
    assert!(final_invalid(&l.inner, 0));
    // A leaf that is no miniscript is not run: its 64-byte element is data,
    // and the input is one Faraday does not evaluate.
    let data = Builder::new()
        .push_opcode(bitcoin::opcodes::all::OP_DROP)
        .push_int(1)
        .into_script();
    let mut raw = tree(&internal, data);
    let control = raw.control.clone();
    let script = raw.script.to_bytes();
    final_witness(&mut raw, &[vec![3u8; 64], script, control]);
    assert!(!final_invalid(&raw.inner, 0));
    assert!(not_evaluated(&raw.inner, 0));
}

#[test]
fn a_final_p2wsh_witness_names_the_master_of_a_key_of_its_script() {
    let (a, evil) = (pair(1), pair(5));
    let (apk, epk) = (
        PublicKey::new(a.public_key()),
        PublicKey::new(evil.public_key()),
    );
    let script = Builder::new()
        .push_int(1)
        .push_key(&apk)
        .push_int(1)
        .push_opcode(bitcoin::opcodes::all::OP_CHECKMULTISIG)
        .into_script();
    let (mut inner, _) = one_input(ScriptBuf::new_p2wsh(&script.wscript_hash()));
    let h = SighashCache::new(&inner.unsigned_tx)
        .p2wsh_signature_hash(0, &script, Amount::from_sat(50_000), EcdsaSighashType::All)
        .unwrap();
    let sig = ecdsa::Signature::sighash_all(
        Secp256k1::new().sign_ecdsa(&Message::from_digest(h.to_byte_array()), &a.secret_key()),
    );
    inner.inputs[0].final_script_witness = Some(Witness::from_slice(&[
        vec![],
        sig.to_vec(),
        script.to_bytes(),
    ]));
    // The origins stay with the finalized input, the script with them or not.
    inner.inputs[0]
        .bip32_derivation
        .insert(apk.inner, (fp(1), path()));
    inner.inputs[0]
        .bip32_derivation
        .insert(epk.inner, (fp(9), path()));
    for kept in [false, true] {
        inner.inputs[0].witness_script = kept.then(|| script.clone());
        let r = rows(&inner);
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].fingerprint, Some(fp(1).into()), "script kept: {kept}");
        assert_eq!(r[0].verdict, Verdict::Valid);
    }
}

// A script-path signature on a Tap miniscript leaf is judged whatever sits
// beside it (an annex, a 65-byte element, a control block ending 0x00).

use bitcoin::sighash::Annex;
use miniscript::{Miniscript, Tap};

fn zero65() -> Vec<u8> {
    let mut z = vec![3u8; 65];
    z[64] = 0;
    z
}

fn tap_ms(s: &str) -> ScriptBuf {
    Miniscript::<bitcoin::key::XOnlyPublicKey, Tap>::from_str(s)
        .unwrap()
        .encode()
}

fn annex_sig(l: &Leaf, k: &Keypair, annex: &[u8]) -> Vec<u8> {
    let h = SighashCache::new(&l.inner.unsigned_tx)
        .taproot_signature_hash(
            0,
            &Prevouts::All(&l.prevs),
            Some(Annex::new(annex).unwrap()),
            Some((l.leaf, 0xFFFF_FFFF)),
            TapSighashType::Default,
        )
        .unwrap();
    schnorr(&l.inner, h.to_byte_array(), k).to_vec()
}

fn forge(sig: &[u8]) -> Vec<u8> {
    let mut bad = sig.to_vec();
    bad[10] ^= 1;
    bad
}

#[test]
fn a_forged_signature_on_a_miniscript_leaf_is_refused_beside_an_annex() {
    let (internal, a) = (pair(7), pair(1));
    let mut l = leaf(&internal, &a);
    let (sc, cb) = (l.script.to_bytes(), l.control.clone());
    let annex = vec![0x50u8, 1, 2, 3];
    let good = l.sign(&a).to_vec();
    // Forged; made without the annex; made over another annex.
    for sig in [forge(&good), good.clone(), annex_sig(&l, &a, &[0x50, 9, 9])] {
        final_witness(&mut l, &[sig, sc.clone(), cb.clone(), annex.clone()]);
        assert!(final_invalid(&l.inner, 0));
    }
    // A true signature over the annex is accepted.
    let true_sig = annex_sig(&l, &a, &annex);
    final_witness(&mut l, &[true_sig, sc.clone(), cb.clone(), annex]);
    assert!(!final_invalid(&l.inner, 0));
    assert!(rows(&l.inner).iter().all(|r| r.verdict != Verdict::Invalid));
}

#[test]
fn a_too_short_multi_a_with_an_annex_is_refused() {
    let (internal, a, b, c) = (pair(7), pair(1), pair(2), pair(3));
    let xs = [&a, &b, &c].map(|k| k.x_only_public_key().0);
    let mut l = tree(
        &internal,
        tap_ms(&format!("multi_a(2,{},{},{})", xs[0], xs[1], xs[2])),
    );
    let (sc, cb) = (l.script.to_bytes(), l.control.clone());
    let annex = vec![0x50u8, 1, 2, 3];
    // Keys are pushed last first: c, b, a.
    let (sa, sb) = (annex_sig(&l, &a, &annex), annex_sig(&l, &b, &annex));
    final_witness(
        &mut l,
        &[
            vec![],
            sb.clone(),
            sa.clone(),
            sc.clone(),
            cb.clone(),
            annex.clone(),
        ],
    );
    assert!(!final_invalid(&l.inner, 0));
    final_witness(&mut l, &[vec![], vec![], sa, sc, cb, annex]);
    assert!(final_invalid(&l.inner, 0));
}

#[test]
fn a_65_byte_element_ending_zero_in_the_stack_of_a_miniscript_leaf_is_refused() {
    let (internal, a, b, c) = (pair(7), pair(1), pair(2), pair(3));
    let z = zero65();
    let xs = [&a, &b, &c].map(|k| k.x_only_public_key().0);
    let hash = bitcoin::hashes::sha256::Hash::hash(&[7u8; 32]);
    // `<A> CHECKSIG`.
    let mut l = leaf(&internal, &a);
    let (sc, cb) = (l.script.to_bytes(), l.control.clone());
    let good = l.sign(&a).to_vec();
    for stack in [
        vec![forge(&good)],
        vec![z.clone(), forge(&good)],
        vec![z.clone(), good.clone()],
        vec![good.clone(), z.clone()],
        vec![z.clone()],
    ] {
        let mut w = stack;
        w.extend([sc.clone(), cb.clone()]);
        final_witness(&mut l, &w);
        assert!(final_invalid(&l.inner, 0), "{w:?}");
    }
    let mut w = vec![good.clone(), sc.clone(), cb.clone()];
    final_witness(&mut l, &w);
    assert!(!final_invalid(&l.inner, 0));
    // The signature itself with a zero sighash byte appended.
    let mut zero_sig = good.clone();
    zero_sig.push(0);
    w[0] = zero_sig;
    final_witness(&mut l, &w);
    assert!(final_invalid(&l.inner, 0));

    // multi_a 2-of-3 with two such elements.
    let l3 = tree(
        &internal,
        tap_ms(&format!("multi_a(2,{},{},{})", xs[0], xs[1], xs[2])),
    );
    let mut l3 = l3;
    let (sc, cb) = (l3.script.to_bytes(), l3.control.clone());
    final_witness(
        &mut l3,
        &[vec![], z.clone(), z.clone(), sc.clone(), cb.clone()],
    );
    assert!(final_invalid(&l3.inner, 0));
    let (sa, sb) = (l3.sign(&a).to_vec(), l3.sign(&b).to_vec());
    final_witness(&mut l3, &[vec![], sb, sa, sc, cb]);
    assert!(!final_invalid(&l3.inner, 0));

    // or_i with the selector; sha256 with the preimage; j:pk.
    let mut oi = tree(
        &internal,
        tap_ms(&format!("or_i(pk({}),pk({}))", xs[0], xs[1])),
    );
    let (sc, cb) = (oi.script.to_bytes(), oi.control.clone());
    let sa = oi.sign(&a).to_vec();
    final_witness(&mut oi, &[sa.clone(), vec![1], sc.clone(), cb.clone()]);
    assert!(!final_invalid(&oi.inner, 0));
    final_witness(&mut oi, &[sa, z.clone(), sc, cb]);
    assert!(final_invalid(&oi.inner, 0));

    let mut sh = tree(
        &internal,
        tap_ms(&format!("and_v(v:pk({}),sha256({hash}))", xs[0])),
    );
    let (sc, cb) = (sh.script.to_bytes(), sh.control.clone());
    let sa = sh.sign(&a).to_vec();
    final_witness(
        &mut sh,
        &[vec![7u8; 32], sa.clone(), sc.clone(), cb.clone()],
    );
    assert!(!final_invalid(&sh.inner, 0));
    final_witness(&mut sh, &[z.clone(), sa, sc, cb]);
    assert!(final_invalid(&sh.inner, 0));

    let mut jp = tree(&internal, tap_ms(&format!("j:pk({})", xs[0])));
    let (sc, cb) = (jp.script.to_bytes(), jp.control.clone());
    let sa = jp.sign(&a).to_vec();
    final_witness(&mut jp, &[sa, sc.clone(), cb.clone()]);
    assert!(!final_invalid(&jp.inner, 0));
    final_witness(&mut jp, &[z, sc, cb]);
    assert!(final_invalid(&jp.inner, 0));
}

#[test]
fn a_control_block_ending_zero_is_judged_and_a_data_leaf_with_such_an_element_is_not_run() {
    let (internal, a) = (pair(7), pair(1));
    let script = Builder::new()
        .push_x_only_key(&a.x_only_public_key().0)
        .push_opcode(OP_CHECKSIG)
        .into_script();
    // Grind a sibling leaf whose hash ends in 0x00: the control block is
    // 65 bytes ending 0x00.
    let mut n = 0i64;
    let mut l = loop {
        let sibling = Builder::new()
            .push_opcode(bitcoin::opcodes::all::OP_DROP)
            .push_int(n)
            .into_script();
        let l = build(
            &internal,
            script.clone(),
            LeafVersion::TapScript,
            Some(sibling),
        );
        if l.control.len() == 65 && l.control[64] == 0 {
            break l;
        }
        n += 1;
    };
    let (sc, cb) = (l.script.to_bytes(), l.control.clone());
    let good = l.sign(&a).to_vec();
    final_witness(&mut l, &[good.clone(), sc.clone(), cb.clone()]);
    assert!(!final_invalid(&l.inner, 0));
    final_witness(&mut l, &[forge(&good), sc, cb]);
    assert!(final_invalid(&l.inner, 0));

    // A leaf that is no miniscript is not run, whatever its stack holds.
    let data = Builder::new()
        .push_opcode(bitcoin::opcodes::all::OP_DROP)
        .push_int(1)
        .into_script();
    let mut raw = tree(&internal, data);
    let (sc, cb) = (raw.script.to_bytes(), raw.control.clone());
    final_witness(&mut raw, &[zero65(), sc, cb]);
    assert!(!final_invalid(&raw.inner, 0));
    assert!(not_evaluated(&raw.inner, 0));
}

#[test]
fn a_leaf_of_a_future_version_is_not_run_and_not_refused() {
    let (internal, a) = (pair(7), pair(1));
    let script = Builder::new()
        .push_x_only_key(&a.x_only_public_key().0)
        .push_opcode(OP_CHECKSIG)
        .into_script();
    let mut l = build(
        &internal,
        script,
        LeafVersion::from_consensus(0xc2).unwrap(),
        None,
    );
    let (sc, cb) = (l.script.to_bytes(), l.control.clone());
    assert_eq!(cb[0] & 0xfe, 0xc2);
    final_witness(&mut l, &[vec![3u8; 64], sc, cb]);
    assert!(!final_invalid(&l.inner, 0));
    assert!(not_evaluated(&l.inner, 0));
}

/// Two inputs and one output: `l`'s input 0 as it is, and a second input
/// that spends an output of the same script.
fn with_second_input(l: &mut Leaf) {
    let spk = l.prevs[0].script_pubkey.clone();
    let prev = TxOut {
        value: Amount::from_sat(50_000),
        script_pubkey: spk,
    };
    let mut txin = l.inner.unsigned_tx.input[0].clone();
    txin.previous_output = OutPoint::new(bitcoin::Txid::from_byte_array([2; 32]), 1);
    l.inner.unsigned_tx.input.push(txin);
    l.inner.inputs.push(bitcoin::psbt::Input {
        witness_utxo: Some(prev.clone()),
        ..Default::default()
    });
    l.prevs.push(prev);
}

#[test]
fn a_sighash_single_signature_with_no_output_of_its_index_is_refused() {
    let (internal, a) = (pair(7), pair(1));
    let mut l = leaf(&internal, &a);
    with_second_input(&mut l);
    // Input 1 is the one with no output at its index.
    let mut sig = vec![9u8; 64];
    sig.push(0x03);
    // Key path.
    let tweaked = internal.tap_tweak(&Secp256k1::new(), None);
    l.inner.unsigned_tx.input[1].witness = Witness::new();
    l.inner.inputs[1].witness_utxo = Some(TxOut {
        value: Amount::from_sat(50_000),
        script_pubkey: ScriptBuf::new_p2tr_tweaked(tweaked.public_parts().0),
    });
    l.inner.inputs[1].final_script_witness = Some(Witness::from_slice(&[sig.clone()]));
    assert!(final_invalid(&l.inner, 1));
    assert!(
        rows(&l.inner)
            .iter()
            .any(|r| r.input == 1 && r.verdict.is_invalid())
    );
    // Script path.
    l.inner.inputs[1].witness_utxo = Some(l.prevs[0].clone());
    let (sc, cb) = (l.script.to_bytes(), l.control.clone());
    final_witness_at(&mut l, 1, &[sig, sc, cb]);
    assert!(final_invalid(&l.inner, 1));
}

fn final_witness_at(l: &mut Leaf, i: usize, elements: &[Vec<u8>]) {
    l.inner.inputs[i].final_script_witness = Some(Witness::from_slice(elements));
}

#[test]
fn a_true_script_path_signature_is_not_refused_beside_an_open_input_with_no_utxo() {
    let (internal, a) = (pair(7), pair(1));
    let mut l = leaf(&internal, &a);
    with_second_input(&mut l);
    let (sc, cb) = (l.script.to_bytes(), l.control.clone());
    let good = l.sign(&a).to_vec();
    l.inner.inputs[1].witness_utxo = None;
    final_witness(&mut l, &[good.clone(), sc.clone(), cb.clone()]);
    assert!(!final_invalid(&l.inner, 0));
    // The forged one is refused once the other output is known.
    final_witness(&mut l, &[forge(&good), sc, cb]);
    l.inner.inputs[1].witness_utxo = Some(l.prevs[1].clone());
    assert!(final_invalid(&l.inner, 0));
}

/// The leaf `DUP HASH160 <hash160 of key> EQUALVERIFY CHECKSIG`.
fn pkh_leaf(internal: &Keypair, key: &[u8]) -> Leaf {
    use bitcoin::opcodes::all::{OP_DUP, OP_EQUALVERIFY, OP_HASH160};
    let h = bitcoin::hashes::hash160::Hash::hash(key).to_byte_array();
    let script = Builder::new()
        .push_opcode(OP_DUP)
        .push_opcode(OP_HASH160)
        .push_slice(h)
        .push_opcode(OP_EQUALVERIFY)
        .push_opcode(OP_CHECKSIG)
        .into_script();
    tree(internal, script)
}

#[test]
fn a_leaf_with_a_key_of_an_unknown_type_is_not_run_and_not_refused() {
    // BIP-342: a key that is not 32 bytes (and not empty) is of an unknown
    // type, and any non-empty signature passes it.
    let internal = pair(7);
    let at_65_00 = {
        let mut k = vec![3u8; 65];
        k[64] = 0;
        k
    };
    let keys = [
        vec![2u8; 33],
        vec![4u8; 65],
        at_65_00,
        vec![5u8; 64],
        vec![6u8; 31],
        vec![7u8],
    ];
    let sigs = [vec![9u8; 64], vec![9u8; 65], zero65(), vec![1u8]];
    for key in &keys {
        let mut l = pkh_leaf(&internal, key);
        let (sc, cb) = (l.script.to_bytes(), l.control.clone());
        for sig in &sigs {
            final_witness(&mut l, &[sig.clone(), key.clone(), sc.clone(), cb.clone()]);
            assert!(!final_invalid(&l.inner, 0), "key {key:?} sig {sig:?}");
            assert!(not_evaluated(&l.inner, 0), "key {key:?} sig {sig:?}");
        }
        // An empty signature fails the check.
        final_witness(&mut l, &[vec![], key.clone(), sc, cb]);
        assert!(final_invalid(&l.inner, 0), "key {key:?}");
    }
}

#[test]
fn a_pkh_leaf_with_an_empty_key_or_a_32_byte_key_off_the_curve_is_refused() {
    let internal = pair(7);
    // 32 bytes of 0xFF is no x coordinate on the curve.
    for key in [vec![], vec![0xFFu8; 32]] {
        let mut l = pkh_leaf(&internal, &key);
        let (sc, cb) = (l.script.to_bytes(), l.control.clone());
        final_witness(&mut l, &[vec![9u8; 64], key.clone(), sc, cb]);
        assert!(final_invalid(&l.inner, 0), "key {key:?}");
    }
}
