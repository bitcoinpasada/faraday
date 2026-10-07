//! Decode a transaction: a finished
//! transaction taken apart field by field, so the person can see that it
//! spends what they meant: its inputs, each output's amount, address and
//! script type, which outputs come back to a loaded wallet, the witness
//! stacks, the locktime, and the fee when the amounts going in are known.
//!
//! The parsing is rust-bitcoin's, through `osk_psbt::transaction`; this
//! module only arranges what it read.

use osk_bip::bitcoin::consensus::encode::serialize_hex;
use osk_bip::bitcoin::{Address, Amount, Script, Transaction, TxOut};
use osk_bip::keys::Network;

use crate::wallet::Session;

/// One input, as decoded.
#[derive(Debug, Clone)]
pub struct DecodedInput {
    /// The output it spends, `txid:vout`.
    pub outpoint: String,
    /// Its sequence, and what it says.
    pub sequence: String,
    /// The amount it spends, when the source says.
    pub amount: Option<u64>,
    /// The witness items, in hex.
    pub witness: Vec<String>,
    /// The scriptSig's length in bytes.
    pub script_sig: usize,
}

/// One output, as decoded.
#[derive(Debug, Clone)]
pub struct DecodedOutput {
    /// Its amount in satoshis.
    pub amount: u64,
    /// Its address on the session's network, or the script in hex.
    pub address: String,
    /// The script's type.
    pub kind: &'static str,
    /// The loaded wallet it pays: its name, change or receive, index.
    pub ours: Option<(String, bool, u32)>,
}

/// A transaction taken apart.
#[derive(Debug, Clone)]
pub struct Decoded {
    /// Where it came from: a file's name, or the spend it finished.
    pub source: String,
    /// The transaction id.
    pub txid: String,
    /// The witness transaction id.
    pub wtxid: String,
    /// The version.
    pub version: i32,
    /// The locktime, and what it says.
    pub locktime: String,
    /// Bytes, virtual bytes, weight units.
    pub size: (usize, usize, u64),
    /// The inputs.
    pub inputs: Vec<DecodedInput>,
    /// The outputs.
    pub outputs: Vec<DecodedOutput>,
    /// The fee, when every input's amount is known.
    pub fee: Option<u64>,
    /// The whole transaction in hex.
    pub hex: String,
}

/// What kind of script an output pays to.
fn script_kind(s: &Script) -> &'static str {
    if s.is_p2wpkh() {
        "P2WPKH"
    } else if s.is_p2wsh() {
        "P2WSH"
    } else if s.is_p2tr() {
        "P2TR"
    } else if s.is_p2sh() {
        "P2SH"
    } else if s.is_p2pkh() {
        "P2PKH"
    } else if s.is_op_return() {
        "OP_RETURN"
    } else {
        "Other script"
    }
}

/// What a sequence number says.
fn sequence_text(n: u32) -> String {
    let what = if n == 0xffff_ffff {
        "final"
    } else if n == 0xffff_fffe {
        "locktime on, no replace-by-fee"
    } else if n & (1 << 31) == 0 && n & (1 << 22) != 0 {
        "relative time lock, replace-by-fee"
    } else if n & (1 << 31) == 0 {
        "relative block lock, replace-by-fee"
    } else {
        "replace-by-fee"
    };
    format!("0x{n:08x} · {what}")
}

/// What a locktime says.
fn locktime_text(n: u32) -> String {
    match n {
        0 => "0 · none".to_string(),
        n if n < 500_000_000 => format!("{n} · block height"),
        n => format!("{n} · Unix time"),
    }
}

/// Takes `tx` apart. `prevouts` are the outputs its inputs spend, in
/// order, where the source states them; the session marks the outputs
/// that come back to a loaded wallet.
pub fn decode(
    tx: &Transaction,
    prevouts: &[Option<TxOut>],
    session: &Session,
    source: &str,
) -> Decoded {
    let network: Network = session.network();
    let inputs: Vec<DecodedInput> = tx
        .input
        .iter()
        .enumerate()
        .map(|(i, txin)| DecodedInput {
            outpoint: format!(
                "{}:{}",
                txin.previous_output.txid, txin.previous_output.vout
            ),
            sequence: sequence_text(txin.sequence.0),
            amount: prevouts
                .get(i)
                .and_then(|p| p.as_ref())
                .map(|o| o.value.to_sat()),
            witness: txin.witness.iter().map(hex).collect(),
            script_sig: txin.script_sig.len(),
        })
        .collect();
    let outputs: Vec<DecodedOutput> = tx
        .output
        .iter()
        .map(|o| {
            let address =
                Address::from_script(&o.script_pubkey, osk_bip::bitcoin::Network::from(network));
            let ours = address.as_ref().ok().and_then(|a| {
                session.wallets.iter().find_map(|w| {
                    w.policy
                        .find_address(network, a, 100)
                        .map(|(change, index)| (w.name.clone(), change, index))
                })
            });
            DecodedOutput {
                amount: o.value.to_sat(),
                address: address
                    .map(|a| a.to_string())
                    .unwrap_or_else(|_| hex(o.script_pubkey.as_bytes())),
                kind: script_kind(&o.script_pubkey),
                ours,
            }
        })
        .collect();
    let paid_in: Option<u64> = inputs.iter().map(|i| i.amount).sum();
    let paid_out: u64 = outputs.iter().map(|o| o.amount).sum();
    Decoded {
        source: source.to_string(),
        txid: tx.compute_txid().to_string(),
        wtxid: tx.compute_wtxid().to_string(),
        version: tx.version.0,
        locktime: locktime_text(tx.lock_time.to_consensus_u32()),
        size: (tx.total_size(), tx.vsize(), tx.weight().to_wu()),
        inputs,
        outputs,
        fee: paid_in.and_then(|i| i.checked_sub(paid_out)),
        hex: serialize_hex(tx),
    }
}

/// The outputs a PSBT's inputs spend, in order, as far as it says.
pub fn prevouts_of(psbt: &osk_psbt::Psbt) -> Vec<Option<TxOut>> {
    let inner = psbt.inner();
    inner
        .inputs
        .iter()
        .zip(&inner.unsigned_tx.input)
        .map(|(input, txin)| {
            input.witness_utxo.clone().or_else(|| {
                input
                    .non_witness_utxo
                    .as_ref()
                    .and_then(|t| t.output.get(txin.previous_output.vout as usize).cloned())
            })
        })
        .collect()
}

/// Satoshis as bitcoin, eight places.
pub fn btc(sats: u64) -> String {
    format!("{} BTC", Amount::from_sat(sats).to_btc())
}

fn hex(bytes: impl AsRef<[u8]>) -> String {
    bytes.as_ref().iter().map(|b| format!("{b:02x}")).collect()
}
