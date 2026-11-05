//! A cosigner's file brought into a spend with Collect, where an input
//! spends a taproot output by a leaf that is run (M, `<A> CHECKSIG`) or one
//! that is not (N, `OP_DROP <A> CHECKSIG`): what the spend finishes with, or
//! the refusal of a file, does not depend on which file came first, on
//! another input's fault, on an output stated by a later file, or on what a
//! file files for the spend's own leaves. Where the spend and a file differ
//! and the device cannot judge between them (two different final witnesses
//! for leaves not run, a row the spend lacks or holds differently or different
//! scripts with no output stated), the file is refused in either order, with
//! one case where file order still decides: two different finals on leaves not
//! run keep the first, the second file is refused, and Finish stays open on the
//! one held, reading "a script was not run".

use b::bip32::{DerivationPath, Fingerprint as BFp};
use b::hashes::Hash;
use b::key::TapTweak;
use b::opcodes::all::*;
use b::secp256k1::{Keypair, Message, Secp256k1, SecretKey};
use b::sighash::{Prevouts, SighashCache, TapSighashType};
use b::taproot::{ControlBlock, LeafVersion, TapLeafHash, TaprootBuilder, TaprootSpendInfo};
use b::{
    Amount, OutPoint, ScriptBuf, Sequence, Transaction, TxIn, TxOut, Witness, absolute::LockTime,
    transaction::Version,
};
use faraday_bip::bitcoin as b;
use faraday_core::testkit;
use faraday_core::wallet::Spend;
use std::str::FromStr;

const NO: u32 = 0xffff_ffff;
const PATH: &str = "m/86'/0'/0'/0/0";

const C0: LeafVersion = LeafVersion::TapScript;
fn pair(n: u8) -> Keypair {
    Keypair::from_secret_key(&Secp256k1::new(), &SecretKey::from_slice(&[n; 32]).unwrap())
}
fn x(k: &Keypair) -> b::XOnlyPublicKey {
    k.x_only_public_key().0
}
fn bld() -> b::script::Builder {
    b::script::Builder::new()
}
fn pk_leaf(k: &Keypair) -> ScriptBuf {
    bld()
        .push_x_only_key(&x(k))
        .push_opcode(OP_CHECKSIG)
        .into_script()
}
fn drop_leaf(k: &Keypair) -> ScriptBuf {
    bld()
        .push_opcode(OP_DROP)
        .push_x_only_key(&x(k))
        .push_opcode(OP_CHECKSIG)
        .into_script()
}
fn sep_leaf(k: &Keypair) -> ScriptBuf {
    bld()
        .push_opcode(OP_CODESEPARATOR)
        .push_x_only_key(&x(k))
        .push_opcode(OP_CHECKSIG)
        .into_script()
}
fn zebra() -> faraday_bip::keys::MasterKey {
    let c = testkit::compute();
    let m = faraday_bip::bip39::Mnemonic::parse(
        &c,
        faraday_bip::bip39::Language::English,
        &testkit::test_words("zebra"),
    )
    .unwrap();
    faraday_bip::keys::MasterKey::from_seed(&c, &m.to_seed(&c, b"").unwrap(), testkit::net())
}
fn held() -> (Keypair, BFp) {
    let c = testkit::compute();
    let z = zebra();
    let path = DerivationPath::from_str(PATH).unwrap();
    let d = z.derive(&c, &path);
    (
        Keypair::from_secret_key(&Secp256k1::new(), d.secret_key()),
        BFp::from(z.fingerprint().0),
    )
}
fn foreign() -> BFp {
    BFp::from_str("aabbccdd").unwrap()
}
fn tree(
    internal: u8,
    leaves: &[(ScriptBuf, LeafVersion)],
) -> (ScriptBuf, Vec<ControlBlock>, TaprootSpendInfo) {
    let secp = Secp256k1::new();
    let mut t = TaprootBuilder::new();
    let n = leaves.len();
    let d: Vec<u8> = match n {
        1 => vec![0],
        2 => vec![1, 1],
        3 => vec![1, 2, 2],
        _ => panic!(),
    };
    for (i, (s, v)) in leaves.iter().enumerate() {
        t = t.add_leaf_with_ver(d[i], s.clone(), *v).unwrap();
    }
    let info = t.finalize(&secp, x(&pair(internal))).unwrap();
    let cbs = leaves
        .iter()
        .map(|l| info.control_block(l).unwrap())
        .collect();
    (ScriptBuf::new_p2tr_tweaked(info.output_key()), cbs, info)
}

fn base_n(spk: &ScriptBuf, inputs: u8) -> (b::Psbt, Transaction, Vec<TxOut>) {
    let prev = TxOut {
        value: Amount::from_sat(20_000),
        script_pubkey: spk.clone(),
    };
    let tx = Transaction {
        version: Version::TWO,
        lock_time: LockTime::ZERO,
        input: (0..inputs)
            .map(|i| TxIn {
                previous_output: OutPoint::new(b::Txid::from_byte_array([i + 1; 32]), 0),
                script_sig: ScriptBuf::new(),
                sequence: Sequence::MAX,
                witness: Witness::new(),
            })
            .collect(),
        output: vec![TxOut {
            value: Amount::from_sat(19_900 * u64::from(inputs)),
            script_pubkey: ScriptBuf::new_p2tr_tweaked(
                pair(40).x_only_public_key().0.dangerous_assume_tweaked(),
            ),
        }],
    };
    let mut p = b::Psbt::from_unsigned_tx(tx.clone()).unwrap();
    for i in 0..inputs as usize {
        p.inputs[i].witness_utxo = Some(prev.clone());
    }
    (p, tx, vec![prev; inputs as usize])
}
fn base(spk: &ScriptBuf) -> (b::Psbt, Transaction, Vec<TxOut>) {
    base_n(spk, 1)
}

fn sig_at(
    tx: &Transaction,
    prev: &[TxOut],
    idx: usize,
    leaf: TapLeafHash,
    sep: u32,
    who: &Keypair,
    forged: bool,
) -> Vec<u8> {
    let h = SighashCache::new(tx)
        .taproot_signature_hash(
            idx,
            &Prevouts::All(prev),
            None,
            Some((leaf, sep)),
            TapSighashType::Default,
        )
        .unwrap();
    let mut s = Secp256k1::new()
        .sign_schnorr_with_aux_rand(&Message::from_digest(h.to_byte_array()), who, &[0u8; 32])
        .serialize()
        .to_vec();
    if forged {
        s[5] ^= 1;
    }
    s
}
fn sig(
    tx: &Transaction,
    prev: &[TxOut],
    leaf: TapLeafHash,
    sep: u32,
    who: &Keypair,
    forged: bool,
) -> Vec<u8> {
    sig_at(tx, prev, 0, leaf, sep, who, forged)
}
fn origin(p: &mut b::Psbt, i: usize, key: b::XOnlyPublicKey, leaves: Vec<TapLeafHash>, fp: BFp) {
    p.inputs[i]
        .tap_key_origins
        .insert(key, (leaves, (fp, DerivationPath::from_str(PATH).unwrap())));
}
fn file(p: &mut b::Psbt, i: usize, key: b::XOnlyPublicKey, leaf: TapLeafHash, s: &[u8]) {
    p.inputs[i]
        .tap_script_sigs
        .insert((key, leaf), b::taproot::Signature::from_slice(s).unwrap());
}
fn lh(s: &ScriptBuf, v: LeafVersion) -> TapLeafHash {
    TapLeafHash::from_script(s, v)
}
fn app(with_held: bool) -> faraday_core::Faraday {
    let mut a = testkit::started();
    if with_held {
        crate::common::add_key(&mut a, "zebra");
    }
    a
}

struct S {
    a: Keypair,
    n: ScriptBuf,
    spk: ScriptBuf,
    cbs: Vec<ControlBlock>,
    p0: b::Psbt,
    tx: Transaction,
    prev: Vec<TxOut>,
    lm: TapLeafHash,
    ln: TapLeafHash,
}
fn s2(inputs: u8) -> S {
    let a = pair(7);
    let (m, n) = (pk_leaf(&a), drop_leaf(&a));
    let (spk, cbs, _) = tree(9, &[(m.clone(), C0), (n.clone(), C0)]);
    let (mut p0, tx, prev) = base_n(&spk, inputs);
    let (lm, ln) = (lh(&m, C0), lh(&n, C0));
    for i in 0..inputs as usize {
        p0.inputs[i]
            .tap_scripts
            .insert(cbs[0].clone(), (m.clone(), C0));
        p0.inputs[i]
            .tap_scripts
            .insert(cbs[1].clone(), (n.clone(), C0));
        origin(&mut p0, i, x(&a), vec![lm, ln], foreign());
    }
    S {
        a,
        n,
        spk,
        cbs,
        p0,
        tx,
        prev,
        lm,
        ln,
    }
}
fn fw(p: &mut b::Psbt, i: usize, sig: Vec<u8>, n: &ScriptBuf, cb: &ControlBlock) {
    p.inputs[i].final_script_witness = Some(Witness::from_slice(&[
        sig,
        vec![0x42],
        n.to_bytes(),
        cb.serialize(),
    ]));
}
fn bytes(p: &b::Psbt) -> Vec<u8> {
    faraday_psbt::Psbt::from(p.clone()).to_bytes()
}
fn sp(p: &b::Psbt) -> Spend {
    Spend::new(faraday_psbt::Psbt::from(p.clone()), "own.psbt")
}
fn sig_aux(
    tx: &Transaction,
    prev: &[TxOut],
    leaf: TapLeafHash,
    who: &Keypair,
    aux: [u8; 32],
) -> Vec<u8> {
    let h = SighashCache::new(tx)
        .taproot_signature_hash(
            0,
            &Prevouts::All(prev),
            None,
            Some((leaf, NO)),
            TapSighashType::Default,
        )
        .unwrap();
    Secp256k1::new()
        .sign_schnorr_with_aux_rand(&Message::from_digest(h.to_byte_array()), who, &aux)
        .serialize()
        .to_vec()
}
fn num_drop(i: i64, k: &Keypair) -> ScriptBuf {
    bld()
        .push_int(i)
        .push_opcode(OP_DROP)
        .push_x_only_key(&x(k))
        .push_opcode(OP_CHECKSIG)
        .into_script()
}
fn wit(sp: &Spend, i: usize) -> Vec<Vec<u8>> {
    sp.finished
        .as_ref()
        .map(|t| t.input[i].witness.iter().map(|e| e.to_vec()).collect())
        .unwrap_or_default()
}

fn puzzle() -> (ScriptBuf, Vec<u8>) {
    let pre = b"hello".to_vec();
    let h = b::hashes::sha256::Hash::hash(&pre);
    (
        bld()
            .push_opcode(OP_SHA256)
            .push_slice(h.to_byte_array())
            .push_opcode(OP_EQUAL)
            .into_script(),
        pre,
    )
}

fn collect(o: &mut Spend, a: &faraday_core::Faraday, f: &b::Psbt) -> Result<(), String> {
    o.collect(&testkit::compute(), &a.session, &bytes(f), "f")
        .map(|_| ())
}

fn first(o: &Spend, i: usize) -> Vec<u8> {
    wit(o, i).into_iter().next().unwrap_or_default()
}

#[test]
fn a_forged_final_witness_on_a_leaf_not_run_gives_way_to_a_true_row_in_either_order() {
    let s = s2(1);
    let a2 = app(false);
    let good = sig(&s.tx, &s.prev, s.lm, NO, &s.a, false);
    let bad = sig(&s.tx, &s.prev, s.ln, NO, &s.a, true);
    let mut forged = base(&s.spk).0;
    fw(&mut forged, 0, bad, &s.n, &s.cbs[1]);
    let mut row = s.p0.clone();
    file(&mut row, 0, x(&s.a), s.lm, &good);
    for order in [[&forged, &row], [&row, &forged]] {
        let mut o = sp(&s.p0);
        for f in order {
            assert_eq!(collect(&mut o, &a2, f), Ok(()));
        }
        assert_eq!(o.finish(), Ok(()));
        assert_eq!(first(&o, 0), good);
    }
}

#[test]
fn a_fault_on_another_input_does_not_turn_the_protection_off() {
    let s = s2(1);
    let a2 = app(false);
    let (pz, pre) = puzzle();
    let pzspk = ScriptBuf::new_p2wsh(&pz.wscript_hash());
    let prev = vec![
        s.prev[0].clone(),
        TxOut {
            value: Amount::from_sat(20_000),
            script_pubkey: pzspk,
        },
    ];
    let mut tx = s.tx.clone();
    tx.input.push(TxIn {
        previous_output: OutPoint::new(b::Txid::from_byte_array([2; 32]), 0),
        script_sig: ScriptBuf::new(),
        sequence: Sequence::MAX,
        witness: Witness::new(),
    });
    let mut p = b::Psbt::from_unsigned_tx(tx.clone()).unwrap();
    p.inputs[0] = s.p0.inputs[0].clone();
    p.inputs[1].witness_utxo = Some(prev[1].clone());
    let still_to_come = p.clone();
    p.inputs[1].witness_script = Some(pz.clone());
    let good = sig(&tx, &prev, s.lm, NO, &s.a, false);
    let bad = sig(&tx, &prev, s.ln, NO, &s.a, true);
    file(&mut p, 0, x(&s.a), s.lm, &good);
    let mut f = b::Psbt::from_unsigned_tx(tx.clone()).unwrap();
    f.inputs[0].witness_utxo = Some(prev[0].clone());
    f.inputs[1].witness_utxo = Some(prev[1].clone());
    f.inputs[1].final_script_witness = Some(Witness::from_slice(&[pre, pz.to_bytes()]));
    fw(&mut f, 0, bad, &s.n, &s.cbs[1]);
    // Input 1 is unknown to the spend's own finalizer, and again lacks its
    // witness script: input 0 still finishes by its own row.
    let mut lacking = still_to_come;
    file(&mut lacking, 0, x(&s.a), s.lm, &good);
    for own in [&p, &lacking] {
        let mut o = sp(own);
        assert_eq!(collect(&mut o, &a2, &f), Ok(()));
        assert_eq!(o.finish(), Ok(()));
        assert_eq!(first(&o, 0), good);
    }
    // Input 1 stays unfinished by the file as well: input 0 is still read
    // on its own, and what Finish says is about input 1.
    let mut only0 = f.clone();
    only0.inputs[1].final_script_witness = None;
    let mut o = sp(&p);
    assert_eq!(collect(&mut o, &a2, &only0), Ok(()));
    assert!(o.psbt.inner().inputs[0].final_script_witness.is_none());
}

#[test]
fn an_output_the_file_states_does_not_let_its_forged_final_witness_in() {
    let a2 = app(false);
    for inputs in [1u8, 2] {
        let s = s2(inputs);
        let last = inputs as usize - 1;
        let mut own = s.p0.clone();
        own.inputs[last].witness_utxo = None;
        let good: Vec<Vec<u8>> = (0..inputs as usize)
            .map(|i| sig_at(&s.tx, &s.prev, i, s.lm, NO, &s.a, false))
            .collect();
        for (i, g) in good.iter().enumerate() {
            file(&mut own, i, x(&s.a), s.lm, g);
        }
        let mut f = base_n(&s.spk, inputs).0;
        fw(
            &mut f,
            0,
            sig(&s.tx, &s.prev, s.ln, NO, &s.a, true),
            &s.n,
            &s.cbs[1],
        );
        let mut o = sp(&own);
        assert_eq!(collect(&mut o, &a2, &f), Ok(()));
        assert_eq!(o.finish(), Ok(()));
        assert_eq!(first(&o, 0), good[0]);
    }
}

#[test]
fn a_witness_utxo_the_file_states_otherwise_is_refused_and_named() {
    let s = s2(1);
    let a2 = app(false);
    let mut wrong = s.prev.clone();
    wrong[0].value = Amount::from_sat(25_000);
    let mut own = s.p0.clone();
    own.inputs[0].witness_utxo = Some(wrong[0].clone());
    file(
        &mut own,
        0,
        x(&s.a),
        s.lm,
        &sig(&s.tx, &wrong, s.lm, NO, &s.a, false),
    );
    let mut f = base(&s.spk).0;
    fw(
        &mut f,
        0,
        sig(&s.tx, &s.prev, s.ln, NO, &s.a, false),
        &s.n,
        &s.cbs[1],
    );
    let mut o = sp(&own);
    assert_eq!(collect(&mut o, &a2, &f), Err(DIFFER_OUTPUTS.to_string()));
    // The spend is as it was.
    assert!(o.psbt.inner().inputs[0].final_script_witness.is_none());
    assert_eq!(
        o.psbt.inner().inputs[0].witness_utxo,
        Some(wrong[0].clone())
    );
    // The same with a previous transaction of another txid beside the
    // spend's false amount: it proves nothing.
    let mut other = prev_tx_of_another_txid(&s.spk);
    other.output[0].value = Amount::from_sat(25_000);
    let mut with_other = own.clone();
    with_other.inputs[0].non_witness_utxo = Some(other);
    let mut o = sp(&with_other);
    assert_eq!(collect(&mut o, &a2, &f), Err(DIFFER_OUTPUTS.to_string()));
    // And the other way round: the file's amount is the false one.
    let mut file_wrong = f.clone();
    file_wrong.inputs[0].witness_utxo = Some(wrong[0].clone());
    let mut o = sp(&base(&s.spk).0);
    assert_eq!(
        collect(&mut o, &a2, &file_wrong),
        Err(DIFFER_OUTPUTS.to_string())
    );
    // A false amount taken while the spend stated none, then the true file:
    // the two state different outputs and neither is proven.
    let mut bare = s.p0.clone();
    bare.inputs[0].witness_utxo = None;
    let mut o = sp(&bare);
    assert_eq!(collect(&mut o, &a2, &file_wrong), Ok(()));
    assert_eq!(collect(&mut o, &a2, &f), Err(DIFFER_OUTPUTS.to_string()));
}

const DIFFER_OUTPUTS: &str =
    "f: input 0: the spend and this file state different outputs for it, and neither is proven";

fn prev_tx_of_another_txid(spk: &ScriptBuf) -> Transaction {
    Transaction {
        version: Version::TWO,
        lock_time: LockTime::ZERO,
        input: Vec::new(),
        output: vec![TxOut {
            value: Amount::from_sat(20_000),
            script_pubkey: spk.clone(),
        }],
    }
}

#[test]
fn a_spend_keeps_its_true_row_on_a_run_leaf_beside_the_files_final_witnesses() {
    let s = s2(2);
    let a2 = app(false);
    let g0 = sig_at(&s.tx, &s.prev, 0, s.lm, NO, &s.a, false);
    let mut own = s.p0.clone();
    file(&mut own, 0, x(&s.a), s.lm, &g0);
    let n0 = sig_at(&s.tx, &s.prev, 0, s.ln, NO, &s.a, false);
    let n1 = sig_at(&s.tx, &s.prev, 1, s.ln, NO, &s.a, false);
    let mut f = base_n(&s.spk, 2).0;
    fw(&mut f, 0, n0, &s.n, &s.cbs[1]);
    fw(&mut f, 1, n1.clone(), &s.n, &s.cbs[1]);
    let mut o = sp(&own);
    assert_eq!(collect(&mut o, &a2, &f), Ok(()));
    assert_eq!(o.finish(), Ok(()));
    assert_eq!(first(&o, 0), g0);
    assert_eq!(first(&o, 1), n1);
}

#[test]
fn an_own_true_row_stays_when_neither_row_can_be_read() {
    let s = s2(1);
    let a2 = app(false);
    // Separator leaf: both rows read not checked.
    let a = pair(7);
    let sep = sep_leaf(&a);
    let (spk, cbs, _) = tree(9, &[(sep.clone(), C0)]);
    let (mut p0, tx, prev) = base(&spk);
    let l = lh(&sep, C0);
    p0.inputs[0].tap_scripts.insert(cbs[0].clone(), (sep, C0));
    origin(&mut p0, 0, x(&a), vec![l], foreign());
    let good = sig(&tx, &prev, l, 0, &a, false);
    let bad = sig(&tx, &prev, l, NO, &a, true);
    let mut own = p0.clone();
    file(&mut own, 0, x(&a), l, &good);
    let mut f = p0.clone();
    file(&mut f, 0, x(&a), l, &bad);
    let mut o = sp(&own);
    assert_eq!(collect(&mut o, &a2, &f), Ok(()));
    assert_eq!(
        o.psbt.inner().inputs[0].tap_script_sigs[&(x(&a), l)].to_vec(),
        good
    );
    // With an output stated by neither, different rows for one key and leaf
    // are refused whichever is true: the order of the two changes nothing.
    let good = sig(&s.tx, &s.prev, s.ln, NO, &s.a, false);
    let bad = sig(&s.tx, &s.prev, s.ln, NO, &s.a, true);
    let mut with_good = s.p0.clone();
    with_good.inputs[0].witness_utxo = None;
    let mut with_bad = with_good.clone();
    file(&mut with_good, 0, x(&s.a), s.ln, &good);
    file(&mut with_bad, 0, x(&s.a), s.ln, &bad);
    for (own, f, kept) in [
        (&with_good, &with_bad, &good),
        (&with_bad, &with_good, &bad),
    ] {
        let mut o = sp(own);
        assert_eq!(collect(&mut o, &a2, f), Err(UNCHECKED_ROW.to_string()));
        assert_eq!(
            o.psbt.inner().inputs[0].tap_script_sigs[&(x(&s.a), s.ln)].to_vec(),
            *kept
        );
    }
}

const UNCHECKED_ROW: &str = "f: input 0: it holds a signature and no output is stated to check it";

#[test]
fn a_forged_row_with_no_output_stated_is_refused_and_the_honest_file_after_it_collects() {
    let s = s2(1);
    let a2 = app(false);
    let good = sig(&s.tx, &s.prev, s.lm, NO, &s.a, false);
    let bad = sig(&s.tx, &s.prev, s.lm, NO, &s.a, true);
    let mut own = s.p0.clone();
    own.inputs[0].witness_utxo = None;
    file(&mut own, 0, x(&s.a), s.lm, &good);
    let mut forged = own.clone();
    file(&mut forged, 0, x(&s.a), s.lm, &bad);
    let honest = base(&s.spk).0;
    // The forged file differs from the spend's row and no output is stated:
    // refused, and the spend is as it was. The honest file then collects.
    let mut o = sp(&own);
    assert_eq!(
        collect(&mut o, &a2, &forged),
        Err(UNCHECKED_ROW.to_string())
    );
    assert_eq!(collect(&mut o, &a2, &honest), Ok(()));
    assert_eq!(o.finish(), Ok(()));
    assert_eq!(first(&o, 0), good);
}

#[test]
fn two_different_rows_with_no_output_stated_are_refused_in_either_order() {
    let s = s2(1);
    let a2 = app(false);
    let good = sig(&s.tx, &s.prev, s.lm, NO, &s.a, false);
    let bad = sig(&s.tx, &s.prev, s.lm, NO, &s.a, true);
    let mut start = s.p0.clone();
    start.inputs[0].witness_utxo = None;
    let mut with_good = start.clone();
    let mut with_bad = start;
    file(&mut with_good, 0, x(&s.a), s.lm, &good);
    file(&mut with_bad, 0, x(&s.a), s.lm, &bad);
    let honest = base(&s.spk).0;
    // Refused whichever row the spend holds. Where the spend holds the
    // true row, an honest file that states the output then finishes it;
    // where it holds the forged one, the honest file is refused for the
    // forged row it now reads, and the spend does not finish.
    for (own, f) in [(&with_bad, &with_good), (&with_good, &with_bad)] {
        let mut o = sp(own);
        assert_eq!(collect(&mut o, &a2, f), Err(UNCHECKED_ROW.to_string()));
    }
    let mut o = sp(&with_good);
    assert_eq!(
        collect(&mut o, &a2, &with_bad),
        Err(UNCHECKED_ROW.to_string())
    );
    assert_eq!(collect(&mut o, &a2, &honest), Ok(()));
    assert_eq!(o.finish(), Ok(()));
    assert_eq!(first(&o, 0), good);
    let mut o = sp(&with_bad);
    assert_eq!(
        collect(&mut o, &a2, &with_good),
        Err(UNCHECKED_ROW.to_string())
    );
    assert!(collect(&mut o, &a2, &honest).is_err());
    assert!(o.finish().is_err());
    // A row the spend holds and a file holds alike is no difference.
    let mut o = sp(&with_good);
    assert_eq!(collect(&mut o, &a2, &with_good), Ok(()));
}

#[test]
fn a_file_that_files_a_run_leafs_control_block_with_another_script_does_not_stop_the_spend() {
    let s = s2(1);
    let a2 = app(false);
    let good = sig(&s.tx, &s.prev, s.lm, NO, &s.a, false);
    let mut own = s.p0.clone();
    file(&mut own, 0, x(&s.a), s.lm, &good);
    let mut f = base(&s.spk).0;
    f.inputs[0]
        .tap_scripts
        .insert(s.cbs[0].clone(), (pk_leaf(&pair(30)), C0));
    let mut f2 = f.clone();
    fw(
        &mut f2,
        0,
        sig(&s.tx, &s.prev, s.ln, NO, &s.a, true),
        &s.n,
        &s.cbs[1],
    );
    for g in [&f, &f2] {
        let mut o = sp(&own);
        assert_eq!(collect(&mut o, &a2, g), Ok(()));
        assert_eq!(o.finish(), Ok(()));
        assert_eq!(first(&o, 0), good);
    }
}

#[test]
fn a_hostile_row_on_a_leaf_not_run_does_not_hide_a_true_row_that_a_recompute_refutes() {
    let c = testkit::compute();
    let (h, hfp) = held();
    let a2 = app(true);
    let m = pk_leaf(&h);
    let lm = lh(&m, C0);
    let pick = |below: bool| {
        (1..40)
            .map(|i| num_drop(i, &h))
            .find(|s| (lh(s, C0) < lm) == below)
            .unwrap()
    };
    for n in [pick(true), pick(false)] {
        let ln = lh(&n, C0);
        let (spk, cbs, _) = tree(9, &[(m.clone(), C0), (n.clone(), C0)]);
        let (mut p0, tx, prev) = base(&spk);
        p0.inputs[0]
            .tap_scripts
            .insert(cbs[0].clone(), (m.clone(), C0));
        p0.inputs[0]
            .tap_scripts
            .insert(cbs[1].clone(), (n.clone(), C0));
        origin(&mut p0, 0, x(&h), vec![lm, ln], hfp);
        let good = sig_aux(&tx, &prev, lm, &h, [0; 32]);
        let random = sig_aux(&tx, &prev, lm, &h, [7; 32]);
        let ngood = sig_aux(&tx, &prev, ln, &h, [0; 32]);
        let mut nbad = ngood.clone();
        nbad[5] ^= 1;
        let check = |p: &b::Psbt| {
            a2.session
                .nonce_check(&c, &faraday_psbt::Psbt::from(p.clone()))
                .wrong()
        };
        // A forged row on N beside a true M row made here is not wrong.
        let mut both = p0.clone();
        file(&mut both, 0, x(&h), lm, &good);
        file(&mut both, 0, x(&h), ln, &nbad);
        assert!(!check(&both));
        // A true M row not made here is wrong beside a true N row made here.
        let mut danger = p0.clone();
        file(&mut danger, 0, x(&h), lm, &random);
        file(&mut danger, 0, x(&h), ln, &ngood);
        assert!(check(&danger));
    }
}

const DIFFER_FINALS: &str = "f: input 0: the spend and this file hold different final witnesses for it, and neither can be checked";

#[test]
fn a_final_witness_the_output_judges_true_stands_over_one_for_a_leaf_not_run_in_either_order() {
    let a2 = app(false);
    let s = s2(1);
    let info = tree(9, &[(pk_leaf(&s.a), C0), (s.n.clone(), C0)]).2;
    let true_m = sig(&s.tx, &s.prev, s.lm, NO, &s.a, false);
    let bad_n = sig(&s.tx, &s.prev, s.ln, NO, &s.a, true);
    let true_n = sig(&s.tx, &s.prev, s.ln, NO, &s.a, false);
    let mut forged = base(&s.spk).0;
    fw(&mut forged, 0, bad_n, &s.n, &s.cbs[1]);
    let mut on_n = base(&s.spk).0;
    fw(&mut on_n, 0, true_n, &s.n, &s.cbs[1]);
    let mut on_m = base(&s.spk).0;
    on_m.inputs[0].final_script_witness = Some(Witness::from_slice(&[
        true_m.clone(),
        pk_leaf(&s.a).to_bytes(),
        s.cbs[0].serialize(),
    ]));
    let secp = Secp256k1::new();
    let tweaked = pair(9).tap_tweak(&secp, info.merkle_root());
    let h = SighashCache::new(&s.tx)
        .taproot_key_spend_signature_hash(0, &Prevouts::All(&s.prev), TapSighashType::Default)
        .unwrap();
    let key_sig = secp
        .sign_schnorr_with_aux_rand(
            &Message::from_digest(h.to_byte_array()),
            &tweaked.to_keypair(),
            &[0u8; 32],
        )
        .serialize()
        .to_vec();
    let mut on_key = base(&s.spk).0;
    on_key.inputs[0].final_script_witness =
        Some(Witness::from_slice(std::slice::from_ref(&key_sig)));
    let bare = base(&s.spk).0;
    for (true_final, first_element) in [(&on_m, &true_m), (&on_key, &key_sig)] {
        for order in [[&forged, true_final], [true_final, &forged]] {
            let mut o = sp(&bare);
            for f in order {
                assert_eq!(collect(&mut o, &a2, f), Ok(()));
            }
            assert_eq!(o.finish(), Ok(()));
            assert_eq!(first(&o, 0), *first_element);
        }
    }
    // Two different finals for a leaf not run: neither can be checked, and
    // the file that comes second is refused whichever it is.
    for order in [[&forged, &on_n], [&on_n, &forged]] {
        let mut o = sp(&bare);
        assert_eq!(collect(&mut o, &a2, order[0]), Ok(()));
        let held = o.psbt.inner().inputs[0].final_script_witness.clone();
        assert_eq!(
            collect(&mut o, &a2, order[1]),
            Err(DIFFER_FINALS.to_string())
        );
        assert_eq!(o.psbt.inner().inputs[0].final_script_witness, held);
    }
    // The same final in two files is no difference.
    let mut o = sp(&bare);
    assert_eq!(collect(&mut o, &a2, &on_n), Ok(()));
    assert_eq!(collect(&mut o, &a2, &on_n), Ok(()));
}

#[test]
fn two_different_scripts_for_one_control_block_with_no_output_stated_are_refused_in_either_order() {
    let a2 = app(false);
    let s = s2(1);
    let other_script = num_drop(1, &s.a);
    let mut one = s.p0.clone();
    one.inputs[0].witness_utxo = None;
    let mut two = one.clone();
    two.inputs[0]
        .tap_scripts
        .insert(s.cbs[1].clone(), (other_script, C0));
    for (own, f) in [(&one, &two), (&two, &one)] {
        let mut o = sp(own);
        assert_eq!(
            collect(&mut o, &a2, f),
            Err("f: input 0: the spend and this file hold different scripts for one control block, and no output is stated to check them".to_string())
        );
    }
    // With the output read, the entry that commits to it stands in either order.
    let mut with_output = two.clone();
    with_output.inputs[0].witness_utxo = Some(s.prev[0].clone());
    for (own, f) in [(&with_output, &s.p0), (&s.p0, &with_output)] {
        let mut o = sp(own);
        assert_eq!(collect(&mut o, &a2, f), Ok(()));
        assert_eq!(o.psbt.inner().inputs[0].tap_scripts[&s.cbs[1]].0, s.n);
    }
}

#[test]
fn two_different_final_witnesses_with_no_output_stated_are_refused_in_either_order() {
    let a2 = app(false);
    let s = s2(1);
    let mut one = s.p0.clone();
    one.inputs[0].witness_utxo = None;
    let mut two = one.clone();
    fw(
        &mut one,
        0,
        sig(&s.tx, &s.prev, s.ln, NO, &s.a, false),
        &s.n,
        &s.cbs[1],
    );
    fw(
        &mut two,
        0,
        sig(&s.tx, &s.prev, s.ln, NO, &s.a, true),
        &s.n,
        &s.cbs[1],
    );
    for (own, f) in [(&one, &two), (&two, &one)] {
        let mut o = sp(own);
        assert_eq!(collect(&mut o, &a2, f), Err(DIFFER_FINALS.to_string()));
    }
}

#[test]
fn an_own_true_row_on_a_leaf_not_run_stays_over_the_files_forged_row_when_the_output_is_read() {
    let s = s2(1);
    let a2 = app(false);
    let good = sig(&s.tx, &s.prev, s.ln, NO, &s.a, false);
    let bad = sig(&s.tx, &s.prev, s.ln, NO, &s.a, true);
    let mut own = s.p0.clone();
    file(&mut own, 0, x(&s.a), s.ln, &good);
    let mut f = s.p0.clone();
    file(&mut f, 0, x(&s.a), s.ln, &bad);
    let mut o = sp(&own);
    assert_eq!(collect(&mut o, &a2, &f), Ok(()));
    assert_eq!(
        o.psbt.inner().inputs[0].tap_script_sigs[&(x(&s.a), s.ln)].to_vec(),
        good
    );
}

#[test]
fn a_forged_row_on_a_two_of_three_with_no_output_stated_is_refused_and_the_others_finish() {
    let (a, bk, c) = (pair(7), pair(8), pair(11));
    let ms = bld()
        .push_x_only_key(&x(&a))
        .push_opcode(OP_CHECKSIG)
        .push_x_only_key(&x(&bk))
        .push_opcode(OP_CHECKSIGADD)
        .push_x_only_key(&x(&c))
        .push_opcode(OP_CHECKSIGADD)
        .push_int(2)
        .push_opcode(OP_NUMEQUAL)
        .into_script();
    let (spk, cbs, _) = tree(9, &[(ms.clone(), C0)]);
    let (with_output, tx, prev) = base(&spk);
    let l = lh(&ms, C0);
    let mut start = with_output.clone();
    start.inputs[0].witness_utxo = None;
    start.inputs[0].tap_scripts.insert(cbs[0].clone(), (ms, C0));
    let row = |p: &b::Psbt, k: &Keypair, forged: bool| {
        let mut q = p.clone();
        file(&mut q, 0, x(k), l, &sig(&tx, &prev, l, NO, k, forged));
        q
    };
    let forged = row(&start, &a, true);
    let (fb, fc) = (row(&with_output, &bk, false), row(&with_output, &c, false));
    let a2 = app(false);
    // The forged file comes first: refused, and B's and C's files finish.
    let mut o = sp(&start);
    assert_eq!(
        collect(&mut o, &a2, &forged),
        Err(UNCHECKED_ROW.to_string())
    );
    assert_eq!(collect(&mut o, &a2, &fb), Ok(()));
    assert_eq!(collect(&mut o, &a2, &fc), Ok(()));
    assert_eq!(o.finish(), Ok(()));
    // After B's file states the output, the forged row is read and refused
    // as a wrong signature, and C's file still finishes.
    let mut o = sp(&start);
    assert_eq!(collect(&mut o, &a2, &fb), Ok(()));
    assert!(collect(&mut o, &a2, &forged).is_err());
    assert_eq!(collect(&mut o, &a2, &fc), Ok(()));
    assert_eq!(o.finish(), Ok(()));
}

#[test]
fn two_different_script_sigs_for_a_p2sh_script_not_run_are_refused_in_either_order() {
    let a2 = app(false);
    let (redeem, pre) = puzzle();
    let spk = ScriptBuf::new_p2sh(&redeem.script_hash());
    let (p0, _, _) = base(&spk);
    let with = |preimage: &[u8]| {
        let mut p = p0.clone();
        p.inputs[0].final_script_sig = Some(
            bld()
                .push_slice(b::script::PushBytesBuf::try_from(preimage.to_vec()).unwrap())
                .push_slice(b::script::PushBytesBuf::try_from(redeem.to_bytes()).unwrap())
                .into_script(),
        );
        p
    };
    let (one, two) = (with(&pre), with(b"other"));
    for (own, f) in [(&one, &two), (&two, &one)] {
        let mut o = sp(own);
        assert_eq!(collect(&mut o, &a2, f), Err(DIFFER_FINALS.to_string()));
    }
}

#[test]
fn a_spend_whose_previous_transaction_is_of_another_txid_is_not_said_to_lack_an_output() {
    let s = s2(1);
    let a2 = app(false);
    let good = sig(&s.tx, &s.prev, s.lm, NO, &s.a, false);
    let bad = sig(&s.tx, &s.prev, s.lm, NO, &s.a, true);
    let mut own = s.p0.clone();
    own.inputs[0].non_witness_utxo = Some(prev_tx_of_another_txid(&s.spk));
    let mut own_row = own.clone();
    file(&mut own_row, 0, x(&s.a), s.lm, &good);
    let mut row = s.p0.clone();
    file(&mut row, 0, x(&s.a), s.lm, &bad);
    let mut script = s.p0.clone();
    script.inputs[0]
        .tap_scripts
        .insert(s.cbs[0].clone(), (pk_leaf(&pair(30)), C0));
    let why = "input 0: its witness_utxo and its previous transaction state different outputs";
    for (spend, f) in [(&own_row, &row), (&own, &script)] {
        let mut o = sp(spend);
        assert_eq!(collect(&mut o, &a2, f), Err(why.to_string()));
    }
}
