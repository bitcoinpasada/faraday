//! Decode a transaction: a finished
//! transaction taken apart field by field, so the person can see that it
//! spends what they meant: its inputs, each output's amount, address and
//! script type, which outputs come back to a loaded wallet, the witness
//! stacks, the locktime, and the fee when the amounts going in are known.
//!
//! The parsing is rust-bitcoin's, through `faraday_psbt::transaction`; this
//! module only arranges what it read.

use crate::strings::EN;
use crate::strings::fill;
use faraday_bip::bitcoin::consensus::encode::serialize_hex;
use faraday_bip::bitcoin::{Address, Amount, Script, Transaction, TxOut};
use faraday_bip::keys::Network;
use faraday_crypto::work::Compute;

use crate::Screen;
use crate::wallet::Session;
use crate::{Action, Faraday, QrView, decode, wallet};

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
    /// Where the amounts going in came from.
    pub amounts_from: AmountsFrom,
    /// The whole transaction in hex.
    pub hex: String,
}

/// Where a decoded transaction's amounts going in came from: a PSBT
/// whose unsigned transaction is exactly this one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AmountsFrom {
    /// This PSBT, by its file's name or the spend's.
    Psbt(String),
    /// No PSBT of this transaction is here.
    None,
    /// PSBTs of this transaction state different amounts: none is used.
    Disagree,
}

impl AmountsFrom {
    /// The row's value.
    pub fn text(&self) -> String {
        match self {
            AmountsFrom::Psbt(name) => fill(EN.decode_amounts_from, &[name]),
            AmountsFrom::None => EN.decode_amounts_none.to_string(),
            AmountsFrom::Disagree => EN.decode_amounts_disagree.to_string(),
        }
    }
}

/// Whether `unsigned`, a PSBT's unsigned transaction, is exactly `tx`
/// with its signatures taken off: the same version, locktime, inputs (each
/// the output it spends and its sequence) and outputs. A lookup.
pub fn same_unsigned(unsigned: &Transaction, tx: &Transaction) -> bool {
    unsigned.version == tx.version
        && unsigned.lock_time == tx.lock_time
        && unsigned.output == tx.output
        && unsigned.input.len() == tx.input.len()
        && unsigned
            .input
            .iter()
            .zip(&tx.input)
            .all(|(a, b)| a.previous_output == b.previous_output && a.sequence == b.sequence)
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
        EN.decode_other_script
    }
}

/// What a sequence number says.
fn sequence_text(n: u32) -> String {
    let what = if n == 0xffff_ffff {
        EN.decode_final
    } else if n == 0xffff_fffe {
        EN.decode_locktime_no_rbf
    } else if n & (1 << 31) == 0 && n & (1 << 22) != 0 {
        EN.decode_relative_time
    } else if n & (1 << 31) == 0 {
        EN.decode_relative_block
    } else {
        "replace-by-fee"
    };
    format!("0x{n:08x} · {what}")
}

/// What a locktime says.
fn locktime_text(n: u32) -> String {
    match n {
        0 => EN.decode_0_none.to_string(),
        n if n < 500_000_000 => fill(EN.decode_block_height, &[&n.to_string()]),
        n => fill(EN.decode_unix_time, &[&n.to_string()]),
    }
}

/// Costs an address derivation (through `find_address`). Takes `tx`
/// apart. `prevouts` are the outputs its inputs spend, in order, where
/// the source states them; the session marks the outputs that come back
/// to a loaded wallet.
pub fn decode(
    c: &Compute,
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
            let address = Address::from_script(
                &o.script_pubkey,
                faraday_bip::bitcoin::Network::from(network),
            );
            let ours = address.as_ref().ok().and_then(|a| {
                session.wallets.iter().find_map(|w| {
                    w.policy()
                        .find_address(c, network, a, 100)
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
        amounts_from: AmountsFrom::None,
        hex: serialize_hex(tx),
    }
}

/// The outputs a PSBT's inputs spend, in order, as far as it says.
pub fn prevouts_of(psbt: &faraday_psbt::Psbt) -> Vec<Option<TxOut>> {
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
    fill(
        EN.decode_btc,
        &[&Amount::from_sat(sats).to_btc().to_string()],
    )
}

fn hex(bytes: impl AsRef<[u8]>) -> String {
    bytes.as_ref().iter().map(|b| format!("{b:02x}")).collect()
}

/// A transaction being decoded, and where the person came from.
pub struct DecodeState {
    /// What it decoded to.
    pub decoded: Decoded,
    /// The screen its back link returns to.
    pub back: Screen,
    /// The hex is shown.
    pub show_hex: bool,
}

/// The Decode screen's area: the transaction taken apart, while it is
/// open.
#[derive(Default)]
pub(crate) struct DecodeArea {
    state: Option<DecodeState>,
}

impl DecodeArea {
    /// The transaction on show.
    pub(crate) fn state(&self) -> Option<&DecodeState> {
        self.state.as_ref()
    }

    /// Shows `decoded` on the Decode screen, whose back link returns to
    /// the screen on show.
    pub(crate) fn show(&mut self, decoded: Decoded, screen: &mut Screen) {
        self.state = Some(DecodeState {
            decoded,
            back: *screen,
            show_hex: false,
        });
        *screen = Screen::Decode;
    }

    /// The whole transaction in hex.
    pub(crate) fn hex(&self) -> Option<&str> {
        self.state.as_ref().map(|d| d.decoded.hex.as_str())
    }

    /// Shows or hides the hex.
    pub(crate) fn toggle_hex(&mut self) {
        if let Some(d) = self.state.as_mut() {
            d.show_hex = !d.show_hex;
        }
    }
}

impl Faraday {
    /// Costs a txid, and a PSBT read for each Inbox PSBT of the same txid.
    /// The amounts `tx`'s inputs spend, from every PSBT here whose
    /// unsigned transaction is exactly `tx` ([`same_unsigned`]): the
    /// spend's, then the Inbox's. An input's amount is used when no two of
    /// them state different ones for it (one that states none does not
    /// count), and the first that states one is named.
    fn amounts_in(&self, c: &Compute, tx: &Transaction) -> (Vec<Option<TxOut>>, AmountsFrom) {
        // The unsigned transaction's id: `tx` with its scriptSigs and
        // witnesses taken off, as a PSBT holds it.
        let mut bare = tx.clone();
        for i in &mut bare.input {
            i.script_sig = faraday_bip::bitcoin::ScriptBuf::new();
            i.witness = faraday_bip::bitcoin::Witness::new();
        }
        let txid = bare.compute_txid();
        let mut found: Vec<(String, Vec<Option<TxOut>>)> = Vec::new();
        if let Some(s) = self.wallets.spend()
            && same_unsigned(s.spend.psbt.unsigned_tx(), tx)
        {
            found.push((
                s.spend.source.to_string(),
                decode::prevouts_of(&s.spend.psbt),
            ));
        }
        for i in self
            .inbox
            .iter()
            .filter(|i| i.psbt().is_some_and(|p| p.txid == txid))
        {
            if let Some(p) = wallet::read_psbt(c, &i.bytes)
                && same_unsigned(p.unsigned_tx(), tx)
            {
                found.push((i.name.clone(), decode::prevouts_of(&p)));
            }
        }
        // Input by input, the amount any of them states: one that leaves
        // it out does not disagree, two that state different ones do.
        let mut amounts: Vec<Option<TxOut>> = vec![None; tx.input.len()];
        let mut name: Option<String> = None;
        for (from, prevouts) in &found {
            for (have, got) in amounts.iter_mut().zip(prevouts) {
                match (have.as_ref(), got) {
                    (Some(a), Some(b)) if a != b => {
                        return (Vec::new(), AmountsFrom::Disagree);
                    }
                    (None, Some(b)) => {
                        *have = Some(b.clone());
                        name.get_or_insert_with(|| from.clone());
                    }
                    _ => {}
                }
            }
        }
        match name {
            Some(name) => (amounts, AmountsFrom::Psbt(name)),
            None => (Vec::new(), AmountsFrom::None),
        }
    }

    /// The transaction on the Decode screen, while it is open.
    pub fn decode(&self) -> Option<&DecodeState> {
        self.decode.state()
    }

    /// Costs what the actions below cost, the keys' and wallets' work among
    /// it: handles the Decode screen's actions (`DecodeFinished`, `DecodeInbox`, `QrDecoded`, `DecodeHex`, `SpendHex`).
    pub(crate) fn decode_act(&mut self, action: Action, c: &Compute) {
        match action {
            Action::DecodeFinished => {
                if let Some(s) = self.wallets.spend()
                    && let Some(tx) = s.spend.finished.as_ref()
                {
                    let prevouts = decode::prevouts_of(&s.spend.psbt);
                    let source = fill(EN.act_finished_from_source, &[&s.spend.source.to_string()]);
                    let mut decoded = decode::decode(c, tx, &prevouts, &self.session, &source);
                    decoded.amounts_from = AmountsFrom::Psbt(s.spend.source.to_string());
                    self.decode.show(decoded, &mut self.screen);
                }
            }
            Action::DecodeInbox(k) => {
                if let Some(item) = self.inbox.get(k)
                    && let Some(tx) = faraday_psbt::transaction::read_raw(&item.bytes)
                {
                    let (prevouts, from) = self.amounts_in(c, &tx);
                    let mut decoded = decode::decode(c, &tx, &prevouts, &self.session, &item.name);
                    decoded.amounts_from = from;
                    self.decode.show(decoded, &mut self.screen);
                    self.list_offset = 0.0;
                }
            }
            Action::QrDecoded => {
                if let Some(hex) = self.decode.hex() {
                    let hex = hex.to_string();
                    self.open_qr(QrView::text(c, EN.act_transaction, &hex));
                }
            }
            Action::DecodeHex => {
                self.decode.toggle_hex();
            }
            Action::SpendHex => {
                if let Some(s) = self.wallets.spend_mut() {
                    s.show_hex = !s.show_hex;
                }
            }
            // `act_inner` passes only the actions matched above.
            _ => unreachable!(),
        }
    }
}
