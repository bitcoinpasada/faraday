//! Which satisfaction a finalized miniscript input carries when its
//! signatures allow more than one: the smallest as written, the first
//! branch the script names when two weigh the same.

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
