//! A raw transaction wrapped in a PSBT, so that it is inspected the way
//! a PSBT is.

use faraday_crypto::work::Compute;
use faraday_psbt::Psbt;
use faraday_psbt::transaction::wrap_raw;

/// The genesis coinbase transaction, which every implementation agrees
/// on.
const GENESIS: &str = "01000000010000000000000000000000000000000000000000000000000000000000000000ffffffff4d04ffff001d0104455468652054696d65732030332f4a616e2f32303039204368616e63656c6c6f72206f6e206272696e6b206f66207365636f6e64206261696c6f757420666f722062616e6b73ffffffff0100f2052a01000000434104678afdb0fe5548271967f1a67130b7105cd6a828e03909a67962e0ea1f61deb649f6bc3f4cef38c4f35504e51ec112de5c384df7ba0b8d578a4c702b6bf11d5fac00000000";

/// The id is the transaction's own, and the wrapper is a PSBT that
/// carries it with its signature data taken off.
#[test]
fn a_raw_transaction_is_wrapped_and_keeps_its_id() {
    let c = Compute::unchecked();
    let (wrapper, txid) = wrap_raw(GENESIS.as_bytes()).expect("a transaction");
    assert_eq!(
        format!("{txid}"),
        "4a5e1e4baab89f3a32518a88c31bc87f618f76673e2cc77ab2127b7afdeda33b"
    );
    let psbt = Psbt::parse_bytes(&c, &wrapper).expect("a PSBT");
    let tx = psbt.unsigned_tx();
    assert_eq!(tx.input.len(), 1);
    assert!(tx.input[0].script_sig.is_empty());
    assert_eq!(tx.output[0].value.to_sat(), 5_000_000_000);
    assert_eq!(wrap_raw(b"not a transaction"), None);
}

/// A PSBT input's previous transaction stands for the output it spends
/// only when it is that transaction: one with another id is not taken,
/// the output stays unknown, and the transaction it spends is asked for.
#[test]
fn a_previous_transaction_with_another_id_does_not_state_the_spent_output() {
    use bitcoin::{OutPoint, ScriptBuf, TxIn, TxOut, consensus};
    use faraday_psbt::transaction::Spending;

    let paid = ScriptBuf::from_bytes(vec![0x51]);
    let spent = crate::common::builder::funding_tx(paid.clone(), 1_000);
    let other = crate::common::builder::funding_tx(ScriptBuf::from_bytes(vec![0x52]), 2_000);
    let tx = bitcoin::Transaction {
        version: bitcoin::transaction::Version::TWO,
        lock_time: bitcoin::absolute::LockTime::ZERO,
        input: vec![TxIn {
            previous_output: OutPoint::new(spent.compute_txid(), 0),
            ..Default::default()
        }],
        output: vec![TxOut {
            value: bitcoin::Amount::from_sat(900),
            script_pubkey: ScriptBuf::new_op_return(b"x"),
        }],
    };
    let mut psbt = bitcoin::Psbt::from_unsigned_tx(tx).unwrap();
    psbt.inputs[0].non_witness_utxo = Some(other);
    let mut spending = Spending::read(&psbt.serialize()).expect("a PSBT");
    assert_eq!(spending.prevouts(), None);
    assert_eq!(spending.waiting_for(), Some(spent.compute_txid()));

    assert!(spending.add_previous(&consensus::serialize(&spent)));
    assert_eq!(spending.prevouts(), Some(vec![&paid]));
}

/// 91 bytes the cross-check found: version, the segwit marker and flag,
/// no inputs, one output, no witnesses, the lock time. Bitcoin Core 31.1's
/// `decoderawtransaction` refuses them read either way (`TX decode
/// failed`), since the flag says witnesses follow and none do.
const NO_INPUTS_FLAGGED: &str = "bb8f21310001000101b4043e00276b9646fac6f65ead1e9684d1734ec6a7fe146d9fa543c59e76ecadbaa4916d005d40f4250002212cbc15484cbea4da2f6fdc00744d42f96a43f4ce52d2e16421dcdb15d9ffa18f980033797ee2";

/// The same bytes without the flag. Core 31.1's `decoderawtransaction`
/// reads them, as it reads by default, to one input with a two-item
/// witness and no outputs: txid fd7b18…, hash 7d5db7…, weight 242.
const FLAG_TAKEN_OUT: &str = "bb8f2131000101b4043e00276b9646fac6f65ead1e9684d1734ec6a7fe146d9fa543c59e76ecadbaa4916d005d40f4250002212cbc15484cbea4da2f6fdc00744d42f96a43f4ce52d2e16421dcdb15d9ffa18f980033797ee2";

/// A raw transaction with no inputs written with the segwit flag is
/// refused, as Bitcoin Core refuses it, however it arrives: as a raw
/// transaction to check, wrapped to inspect, or as a previous
/// transaction. Bytes Core reads still read to what Core reads.
#[test]
fn a_transaction_core_refuses_for_a_superfluous_witness_flag_is_refused() {
    use bitcoin::hex::FromHex;
    use faraday_psbt::transaction::{Spending, decode, read_raw};

    let raw = Vec::<u8>::from_hex(NO_INPUTS_FLAGGED).unwrap();
    assert_eq!(decode(&raw), None);
    assert_eq!(read_raw(NO_INPUTS_FLAGGED.as_bytes()), None);
    assert_eq!(read_raw(&raw), None);
    assert_eq!(wrap_raw(NO_INPUTS_FLAGGED.as_bytes()), None);
    assert_eq!(Spending::read(&raw), None);

    let read = read_raw(FLAG_TAKEN_OUT.as_bytes()).expect("Core reads it");
    assert_eq!(
        read.compute_txid().to_string(),
        "fd7b1847502365212457b91a908c23eedcf663d29b93d10761fc6a87e5b601c6"
    );
    assert_eq!(
        read.compute_wtxid().to_string(),
        "7d5db7814ffec93a0fa0693bdca9bcf550649a7ee856ea42ba694df4c3c8ee28"
    );
    assert_eq!(read.weight().to_wu(), 242);
    assert_eq!((read.input.len(), read.output.len()), (1, 0));
    assert_eq!(read.input[0].witness.len(), 2);

    let mut spending = Spending::read(&bitcoin::consensus::serialize(&read)).unwrap();
    assert!(!spending.add_previous(&raw));
}
