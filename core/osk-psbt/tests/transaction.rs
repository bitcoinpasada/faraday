//! A raw transaction wrapped in a PSBT, so that it is inspected the way
//! a PSBT is.

use osk_psbt::Psbt;
use osk_psbt::transaction::wrap_raw;

/// The genesis coinbase transaction, which every implementation agrees
/// on.
const GENESIS: &str = "01000000010000000000000000000000000000000000000000000000000000000000000000ffffffff4d04ffff001d0104455468652054696d65732030332f4a616e2f32303039204368616e63656c6c6f72206f6e206272696e6b206f66207365636f6e64206261696c6f757420666f722062616e6b73ffffffff0100f2052a01000000434104678afdb0fe5548271967f1a67130b7105cd6a828e03909a67962e0ea1f61deb649f6bc3f4cef38c4f35504e51ec112de5c384df7ba0b8d578a4c702b6bf11d5fac00000000";

/// The id is the transaction's own, and the wrapper is a PSBT that
/// carries it with its signature data taken off.
#[test]
fn a_raw_transaction_is_wrapped_and_keeps_its_id() {
    let (wrapper, txid) = wrap_raw(GENESIS.as_bytes()).expect("a transaction");
    assert_eq!(
        format!("{txid}"),
        "4a5e1e4baab89f3a32518a88c31bc87f618f76673e2cc77ab2127b7afdeda33b"
    );
    let psbt = Psbt::parse_bytes(&wrapper).expect("a PSBT");
    let tx = psbt.unsigned_tx();
    assert_eq!(tx.input.len(), 1);
    assert!(tx.input[0].script_sig.is_empty());
    assert_eq!(tx.output[0].value.to_sat(), 5_000_000_000);
    assert_eq!(wrap_raw(b"not a transaction"), None);
}
