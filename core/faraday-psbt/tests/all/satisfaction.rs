//! Which satisfaction a finalized miniscript input carries when its
//! signatures allow more than one: the smallest as written, and where
//! two weigh the same the one Bitcoin Core's `finalizepsbt` keeps: the
//! first branch an `or` names, the keys a `multi` names first, the
//! branches a `thresh` names last, the taproot leaf whose script sorts
//! first (F161).

use core::str::FromStr;

use bitcoin::absolute::LockTime;
use bitcoin::ecdsa::Signature;
use bitcoin::hashes::Hash;
use bitcoin::secp256k1::{Message, Secp256k1, SecretKey};
use bitcoin::sighash::{EcdsaSighashType, SighashCache};
use bitcoin::transaction::Version;
use bitcoin::{
    Amount, OutPoint, PublicKey, ScriptBuf, Sequence, Transaction, TxIn, TxOut, Txid, Witness,
};
use faraday_crypto::work::Compute;
use faraday_psbt::{Psbt, finalize};
use miniscript::{Miniscript, Segwitv0};

const OR_B_PSBT: &str = include_str!("../../../../tools/vectors/psbt/finalize-or-b.psbt");
const OR_B_FINAL: &str = include_str!("../../../../tools/vectors/psbt/finalize-or-b-final.psbt");

/// The cross-check's case: input 3 spends `or_b(pk(A),s:pk(B))` with a
/// 72-byte signature from A and a 71-byte one from B. The finalized
/// PSBT and its transaction are python-bip380's answer byte for byte:
/// B's signature and A's empty push, one byte under A's.
#[test]
fn or_b_carries_the_smaller_signature() {
    let c = Compute::unchecked();
    let mut psbt = Psbt::parse_base64(&c, OR_B_PSBT.trim()).expect("the vector");
    let reference = Psbt::parse_base64(&c, OR_B_FINAL.trim()).expect("the reference's answer");
    let sigs: Vec<usize> = psbt.inner().inputs[3]
        .partial_sigs
        .values()
        .map(|s| s.to_vec().len())
        .collect();
    assert_eq!(sigs.len(), 2);
    assert!(sigs.contains(&72) && sigs.contains(&71));

    let tx = finalize(&mut psbt)
        .expect("finalizes")
        .expect("every input final");
    for (i, (got, want)) in psbt
        .inner()
        .inputs
        .iter()
        .zip(&reference.inner().inputs)
        .enumerate()
    {
        assert_eq!(got.final_script_sig, want.final_script_sig, "input {i}");
        assert_eq!(
            got.final_script_witness, want.final_script_witness,
            "input {i}"
        );
    }
    let witness = tx.input[3].witness.to_vec();
    assert_eq!(witness.len(), 3);
    assert_eq!(witness[0].len(), 71);
    assert!(witness[1].is_empty());
    assert_eq!(
        bitcoin::consensus::serialize(&tx),
        bitcoin::consensus::serialize(&reference.inner().clone().extract_tx().expect("final")),
    );
}

const TAP_TIE_PSBT: &str =
    include_str!("../../../../tools/vectors/psbt/finalize-tap-leaf-tie.psbt");
const TAP_TIE_FINAL: &str =
    include_str!("../../../../tools/vectors/psbt/finalize-tap-leaf-tie-final.psbt");

/// The cross-check's case: input 1 spends a taproot tree whose leaves
/// tie by `miniscript`'s count, which leaves out each element's length
/// byte, and differ by one byte as written. Faraday's witness is the
/// 240-byte one the reference gives, not the 241-byte one.
#[test]
fn taproot_leaf_is_the_smaller_as_written() {
    let c = Compute::unchecked();
    let mut psbt = Psbt::parse_base64(&c, TAP_TIE_PSBT.trim()).expect("the vector");
    let reference = Psbt::parse_base64(&c, TAP_TIE_FINAL.trim()).expect("the reference's answer");

    finalize(&mut psbt).expect("finalizes");
    let got = &psbt.inner().inputs[1];
    let want = &reference.inner().inputs[1];
    assert_eq!(got.final_script_witness, want.final_script_witness);
    let written = bitcoin::consensus::serialize(got.final_script_witness.as_ref().expect("final"));
    assert_eq!(written.len(), 240);
}

/// Finalizes the vector `psbt` and checks every input's final scriptSig
/// and witness, and the extracted transaction, against `core`, Bitcoin
/// Core's `finalizepsbt` answer to it; gives input `input`'s witness.
fn finalized_as_core(psbt: &str, core: &str, input: usize) -> Vec<Vec<u8>> {
    let c = Compute::unchecked();
    let mut psbt = Psbt::parse_base64(&c, psbt.trim()).expect("the vector");
    let reference = Psbt::parse_base64(&c, core.trim()).expect("Core's answer");
    let tx = finalize(&mut psbt)
        .expect("finalizes")
        .expect("every input final");
    for (i, (got, want)) in psbt
        .inner()
        .inputs
        .iter()
        .zip(&reference.inner().inputs)
        .enumerate()
    {
        assert_eq!(got.final_script_sig, want.final_script_sig, "input {i}");
        assert_eq!(
            got.final_script_witness, want.final_script_witness,
            "input {i}"
        );
    }
    assert_eq!(
        bitcoin::consensus::serialize(&tx),
        bitcoin::consensus::serialize(&reference.inner().clone().extract_tx().expect("final")),
    );
    tx.input[input].witness.to_vec()
}

/// The lengths of input `input`'s partial signatures, shortest first.
fn sig_lengths(psbt: &str, input: usize) -> Vec<usize> {
    let c = Compute::unchecked();
    let psbt = Psbt::parse_base64(&c, psbt.trim()).expect("the vector");
    let mut lengths: Vec<usize> = psbt.inner().inputs[input]
        .partial_sigs
        .values()
        .map(|s| s.to_vec().len())
        .collect();
    lengths.sort_unstable();
    lengths
}

/// A witness as a transaction writes it.
fn written(witness: &[Vec<u8>]) -> usize {
    bitcoin::consensus::serialize(&Witness::from_slice(witness)).len()
}

const TAP_ORDER_PSBT: &str =
    include_str!("../../../../tools/vectors/psbt/finalize-tap-leaf-order.psbt");
const TAP_ORDER_FINAL: &str =
    include_str!("../../../../tools/vectors/psbt/finalize-tap-leaf-order-final.psbt");

/// The cross-check's case: input 0 spends a taproot tree by its script
/// path, and two of its leaves, each a key and a relative timelock at
/// the same depth, give witnesses of the same 235 bytes. Faraday keeps
/// the leaf whose script sorts first by its bytes, as Core does.
#[test]
fn taproot_leaves_that_weigh_the_same_keep_the_first_script_by_bytes() {
    let c = Compute::unchecked();
    let psbt = Psbt::parse_base64(&c, TAP_ORDER_PSBT.trim()).expect("the vector");
    let mut tied: Vec<Vec<u8>> = psbt.inner().inputs[0]
        .tap_scripts
        .iter()
        .filter(|(control, (script, _))| control.serialize().len() == 129 && script.len() == 38)
        .map(|(_, (script, _))| script.to_bytes())
        .collect();
    tied.sort();
    assert_eq!(tied.len(), 2);

    let witness = finalized_as_core(TAP_ORDER_PSBT, TAP_ORDER_FINAL, 0);
    assert_eq!(witness[1], tied[0]);
    assert_eq!(written(&witness), 235);
}

const MULTI_PSBT: &str =
    include_str!("../../../../tools/vectors/psbt/finalize-multi-smallest.psbt");
const MULTI_FINAL: &str =
    include_str!("../../../../tools/vectors/psbt/finalize-multi-smallest-final.psbt");

/// The cross-check's case: input 0 spends
/// `sh(wsh(or_d(multi(2,A,B,C),and_v(v:pkh(D),older(N)))))` with four
/// signatures on hand, one 72 bytes and three 71, the timelock not met.
/// The `multi` carries two 71-byte signatures, as Core's does: a witness
/// of 284 bytes, where the first two keys' would be 285.
#[test]
fn multi_inside_or_d_keeps_the_smallest_signatures() {
    assert_eq!(sig_lengths(MULTI_PSBT, 0), [71, 71, 71, 72]);
    let witness = finalized_as_core(MULTI_PSBT, MULTI_FINAL, 0);
    let lengths: Vec<usize> = witness.iter().map(Vec::len).collect();
    assert_eq!(lengths[..3], [0, 71, 71]);
    assert_eq!(written(&witness), 284);
}

const MULTI_WSH_PSBT: &str =
    include_str!("../../../../tools/vectors/psbt/finalize-multi-smallest-wsh.psbt");
const MULTI_WSH_FINAL: &str =
    include_str!("../../../../tools/vectors/psbt/finalize-multi-smallest-wsh-final.psbt");

/// The cross-check's case: input 0 spends
/// `wsh(or_d(multi(2,A,B,C),and_v(v:pkh(D),older(N))))` with four
/// signatures on hand, one 70 bytes and three 71. The `multi` carries
/// the 70-byte one and a 71-byte one, as Core's does: 283 bytes, not
/// 284.
#[test]
fn multi_keeps_the_shortest_signature() {
    assert_eq!(sig_lengths(MULTI_WSH_PSBT, 0), [70, 71, 71, 71]);
    let witness = finalized_as_core(MULTI_WSH_PSBT, MULTI_WSH_FINAL, 0);
    let lengths: Vec<usize> = witness.iter().map(Vec::len).collect();
    assert_eq!(lengths[..3], [0, 71, 70]);
    assert_eq!(written(&witness), 283);
}

const THRESH_TIE_PSBT: &str =
    include_str!("../../../../tools/vectors/psbt/finalize-thresh-tie.psbt");
const THRESH_TIE_FINAL: &str =
    include_str!("../../../../tools/vectors/psbt/finalize-thresh-tie-final.psbt");

/// The cross-check's case: input 2 spends
/// `sh(wsh(thresh(2,pk(A),s:pk(B),sln:older(N))))` with A and B signed,
/// both at 71 bytes, and the timelock met. Satisfying A and the timelock,
/// or B and the timelock, both give 161 bytes; Core satisfies the later
/// branches, B and the timelock, with an empty push for A on top.
#[test]
fn thresh_tie_satisfies_the_later_branches() {
    assert_eq!(sig_lengths(THRESH_TIE_PSBT, 2), [71, 71]);
    let witness = finalized_as_core(THRESH_TIE_PSBT, THRESH_TIE_FINAL, 2);
    let lengths: Vec<usize> = witness.iter().map(Vec::len).collect();
    assert_eq!(lengths[..3], [0, 71, 0]);
    assert_eq!(written(&witness), 161);
}

struct Spend {
    script: ScriptBuf,
    tx: Transaction,
    utxo: TxOut,
}

impl Spend {
    fn new(miniscript: &str) -> Self {
        let ms = Miniscript::<PublicKey, Segwitv0>::from_str(miniscript).expect("a miniscript");
        let script = ms.encode();
        let utxo = TxOut {
            value: Amount::from_sat(100_000),
            script_pubkey: ScriptBuf::new_p2wsh(&script.wscript_hash()),
        };
        let tx = Transaction {
            version: Version::TWO,
            lock_time: LockTime::ZERO,
            input: vec![TxIn {
                previous_output: OutPoint::new(Txid::from_byte_array([7; 32]), 0),
                script_sig: ScriptBuf::new(),
                sequence: Sequence::ENABLE_RBF_NO_LOCKTIME,
                witness: Witness::new(),
            }],
            output: vec![TxOut {
                value: Amount::from_sat(90_000),
                script_pubkey: ScriptBuf::new_p2wsh(&script.wscript_hash()),
            }],
        };
        Spend { script, tx, utxo }
    }

    /// The signature `secret` makes, with the first RFC 6979 nonce or
    /// ground to a low R.
    fn sign(&self, secret: &SecretKey, low_r: bool) -> Signature {
        let secp = Secp256k1::signing_only();
        let hash = SighashCache::new(&self.tx)
            .p2wsh_signature_hash(0, &self.script, self.utxo.value, EcdsaSighashType::All)
            .expect("one input");
        let msg = Message::from_digest(hash.to_byte_array());
        let signature = if low_r {
            secp.sign_ecdsa_low_r(&msg, secret)
        } else {
            secp.sign_ecdsa(&msg, secret)
        };
        Signature::sighash_all(signature)
    }

    /// The final witness, signed by `sigs`.
    fn finalize(&self, sigs: &[(PublicKey, Signature)]) -> Vec<Vec<u8>> {
        let mut inner = bitcoin::Psbt::from_unsigned_tx(self.tx.clone()).expect("unsigned");
        inner.inputs[0].witness_utxo = Some(self.utxo.clone());
        inner.inputs[0].witness_script = Some(self.script.clone());
        inner.inputs[0].partial_sigs = sigs.iter().copied().collect();
        let c = Compute::unchecked();
        let mut psbt = Psbt::parse_bytes(&c, &inner.serialize()).expect("a PSBT");
        let tx = finalize(&mut psbt).expect("finalizes").expect("final");
        let mut witness = tx.input[0].witness.to_vec();
        assert_eq!(witness.pop(), Some(self.script.to_bytes()));
        witness
    }
}

fn key(n: u8) -> (SecretKey, PublicKey) {
    let secret = SecretKey::from_slice(&[n; 32]).expect("a key");
    let public = PublicKey::new(secret.public_key(&Secp256k1::signing_only()));
    (secret, public)
}

/// Both of `or_b`'s satisfactions weigh the same when the signatures
/// are the same length: the first branch, A's signature over B's empty
/// push, every time.
#[test]
fn or_b_with_equal_signatures_takes_its_first_branch() {
    let (sa, a) = key(1);
    let (sb, b) = key(2);
    let spend = Spend::new(&format!("or_b(pk({a}),s:pk({b}))"));
    let sig_a = spend.sign(&sa, true);
    let sig_b = spend.sign(&sb, true);
    assert_eq!(sig_a.to_vec().len(), sig_b.to_vec().len());

    let want = vec![Vec::new(), sig_a.to_vec()];
    assert_eq!(spend.finalize(&[(a, sig_a), (b, sig_b)]), want);
    assert_eq!(spend.finalize(&[(b, sig_b), (a, sig_a)]), want);
}

/// `or_i(pk(A),pk(B))` writes A's signature over a 1 and B's over an
/// empty push, a byte less. With B's signature a byte longer than A's
/// the two weigh the same, and the first branch, A's, is taken; with
/// both the same length B's is the smaller and is taken.
#[test]
fn or_i_takes_its_first_branch_on_a_tie_and_the_smaller_otherwise() {
    let (sa, a) = key(1);
    // B's key is in the script and so in what is signed: the first of
    // B's keys from 2 up that signs this spend at 72 bytes.
    let spend_for = |b: PublicKey| Spend::new(&format!("or_i(pk({a}),pk({b}))"));
    let (b, sb) = (2..=u8::MAX)
        .map(key)
        .find(|(s, p)| spend_for(*p).sign(s, false).to_vec().len() == 72)
        .map(|(s, p)| (p, s))
        .expect("about half the keys sign at 72 bytes");
    let spend = spend_for(b);
    let sig_a = spend.sign(&sa, true);
    let sig_b = spend.sign(&sb, false);
    assert_eq!(sig_a.to_vec().len(), 71);
    assert_eq!(sig_b.to_vec().len(), 72);
    assert_eq!(
        spend.finalize(&[(a, sig_a), (b, sig_b)]),
        vec![sig_a.to_vec(), vec![1]],
    );

    let sig_b = spend.sign(&sb, true);
    assert_eq!(sig_b.to_vec().len(), 71);
    assert_eq!(
        spend.finalize(&[(a, sig_a), (b, sig_b)]),
        vec![sig_b.to_vec(), Vec::new()],
    );
}

/// A `multi` with more signatures on hand than it needs, all the same
/// length, carries the keys it names first, in key order.
#[test]
fn multi_with_equal_signatures_takes_the_first_keys() {
    let (sa, a) = key(1);
    let (sb, b) = key(2);
    let (sc, c) = key(3);
    let (_, d) = key(4);
    let spend = Spend::new(&format!(
        "or_d(multi(2,{a},{b},{c}),and_v(v:pkh({d}),older(144)))"
    ));
    let sigs = [
        (a, spend.sign(&sa, true)),
        (b, spend.sign(&sb, true)),
        (c, spend.sign(&sc, true)),
    ];
    assert!(sigs.iter().all(|(_, s)| s.to_vec().len() == 71));
    assert_eq!(
        spend.finalize(&sigs),
        vec![Vec::new(), sigs[0].1.to_vec(), sigs[1].1.to_vec()],
    );
}

/// A `thresh` whose choices weigh the same satisfies its later branches
/// and dissatisfies the earlier, as Core does: of
/// `thresh(2,pk(A),s:pk(B),s:pk(C))` with three signatures of one
/// length, B's and C's.
#[test]
fn thresh_with_equal_signatures_satisfies_the_last_branches() {
    let (sa, a) = key(1);
    let (sb, b) = key(2);
    let (sc, c) = key(3);
    let spend = Spend::new(&format!("thresh(2,pk({a}),s:pk({b}),s:pk({c}))"));
    let (sig_a, sig_b, sig_c) = (
        spend.sign(&sa, true),
        spend.sign(&sb, true),
        spend.sign(&sc, true),
    );
    assert!([sig_a, sig_b, sig_c].iter().all(|s| s.to_vec().len() == 71));
    // Bottom of the stack first: C's, then B's, then A's empty push on top.
    assert_eq!(
        spend.finalize(&[(a, sig_a), (b, sig_b), (c, sig_c)]),
        vec![sig_c.to_vec(), sig_b.to_vec(), Vec::new()],
    );
}
