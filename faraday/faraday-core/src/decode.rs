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
use faraday_psbt::transaction::money_total;

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
///
/// Refused, with the line that says why, when an output or the outputs'
/// total is over 21,000,000 BTC, as Bitcoin Core's `CheckTransaction`
/// refuses it, or an amount stated for an input or the total of those
/// stated is, as its `CheckTxInputs` does
/// ([`faraday_psbt::transaction::money_total`]). So the totals and the
/// fee shown never overflow.
pub fn decode(
    c: &Compute,
    tx: &Transaction,
    prevouts: &[Option<TxOut>],
    session: &Session,
    source: &str,
) -> Result<Decoded, &'static str> {
    let paid_out = money_total(tx.output.iter().map(|o| o.value))
        .map_err(|_| EN.decode_outputs_over_max_money)?
        .to_sat();
    let stated = || (0..tx.input.len()).map(|i| prevouts.get(i).and_then(|p| p.as_ref()));
    let paid_in = money_total(stated().flatten().map(|o| o.value))
        .map_err(|_| EN.decode_inputs_over_max_money)?;
    let paid_in = stated().all(|o| o.is_some()).then_some(paid_in.to_sat());
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
    Ok(Decoded {
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
    })
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

/// A signed PSBT or transaction brought in to be checked rather than
/// signed (`Screen::NonceCheck`, `docs/FLOWS.md`, Nonce check): its nonce
/// check, worked out once when it opens, and what the screen offers with
/// it.
///
/// The check is made from the file and the session's keys and wallets as
/// they were then (`made_with`). The screen is reached only by opening
/// it, which makes it anew, or by Decode's way back, which makes it anew
/// when the keys or wallets have changed since
/// ([`Faraday::nonce_check_back`]). A file's bytes never change, so
/// nothing else goes out of step.
pub struct OutsideCheck {
    /// The file's name.
    pub source: String,
    /// What the check found.
    pub check: crate::wallet::NonceCheck,
    /// The finished transaction: the one that came, or the PSBT
    /// finished, when every input is final or has its signatures and no
    /// signature is wrong. `None` when the check refuses it or more
    /// signatures are wanted.
    pub finished: Option<Transaction>,
    /// The finished transaction in hex, kept with it.
    pub hex: Option<String>,
    /// The file Save to a stick puts For the stick: its name and bytes.
    pub out: (String, Vec<u8>),
    /// The screen its back link returns to.
    pub back: Screen,
    /// The hex is shown.
    pub show_hex: bool,
    /// The keys and wallets it was checked with.
    made_with: crate::wallet::InspectInputs,
    /// The PSBT checked (a raw transaction wrapped, its signatures
    /// final), kept to check again.
    psbt: faraday_psbt::Psbt,
    /// What the inputs spend, as far as is known, and from where, for
    /// Decode.
    prevouts: (Vec<Option<TxOut>>, AmountsFrom),
}

/// A press on the nonce check of a transaction brought in
/// (`Screen::NonceCheck`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckPress {
    /// The nonce check of this Inbox file, a signed PSBT or a finished
    /// transaction.
    Inbox(usize),
    /// Decode's way back to the nonce check, checked again when the keys
    /// or wallets changed.
    Back,
    /// Show or hide the checked transaction's hex.
    Hex,
    /// The checked transaction, or the PSBT when it is not finished, as
    /// a QR code.
    Qr,
    /// The checked file For the stick: the finished transaction, or the
    /// PSBT as it came.
    Out,
    /// Decode the checked transaction.
    Decode,
}

/// The Decode screen's area: the transaction taken apart, while it is
/// open, and the nonce check of one brought in.
#[derive(Default)]
pub(crate) struct DecodeArea {
    state: Option<DecodeState>,
    /// Boxed: it holds a PSBT and its check, and `Faraday` is built on
    /// the stack.
    outside: Option<Box<OutsideCheck>>,
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

    /// The nonce check of a transaction brought in, while it is kept.
    pub(crate) fn outside(&self) -> Option<&OutsideCheck> {
        self.outside.as_deref()
    }
}

/// The inputs that carry signature data, final, but state no output they
/// spend ([`crate::wallet::NonceCheck::unread`]). A lookup per input.
fn unread(psbt: &faraday_psbt::Psbt) -> usize {
    psbt.inner()
        .inputs
        .iter()
        .filter(|i| {
            (i.final_script_sig.is_some() || i.final_script_witness.is_some())
                && i.witness_utxo.is_none()
                && i.non_witness_utxo.is_none()
        })
        .count()
}

/// Costs a PSBT parse (through `parse_bytes`). A raw transaction as a
/// PSBT whose inputs are final with the transaction's own `scriptSig`s
/// and witnesses, each with the output it spends where `prevouts` states
/// it: what `faraday_psbt::verify` reads a finalized input from. `None`
/// where the bytes are no transaction.
fn raw_as_psbt(
    c: &Compute,
    bytes: &[u8],
    prevouts: &[Option<TxOut>],
) -> Option<(faraday_psbt::Psbt, Transaction)> {
    let tx = faraday_psbt::transaction::read_raw(bytes)?;
    let (wrapped, _) = faraday_psbt::transaction::wrap_raw(bytes)?;
    let mut psbt = faraday_psbt::Psbt::parse_bytes(c, &wrapped).ok()?;
    for (i, (input, txin)) in psbt
        .inner_mut()
        .inputs
        .iter_mut()
        .zip(&tx.input)
        .enumerate()
    {
        if !txin.script_sig.is_empty() {
            input.final_script_sig = Some(txin.script_sig.clone());
        }
        if !txin.witness.is_empty() {
            input.final_script_witness = Some(txin.witness.clone());
        }
        input.witness_utxo = prevouts.get(i).cloned().flatten();
    }
    Some((psbt, tx))
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

    /// The nonce check of a transaction brought in, while it is kept
    /// (`Screen::NonceCheck`).
    pub fn outside_check(&self) -> Option<&OutsideCheck> {
        self.decode.outside()
    }

    /// Costs what [`Faraday::nonce_check_open`] costs for a signed
    /// transaction. Where a file the camera read lands: a signed PSBT no
    /// key here is left to sign, or a finished transaction, opens its
    /// nonce check, its way back to where the scan started; the open
    /// spend's own PSBT come back stays with the spend; anything else is
    /// in Files (`docs/FLOWS.md`, Nonce check).
    pub(crate) fn scan_landed(&mut self, c: &Compute, at: usize, name: &str) {
        let the_spends = self.inbox[at].psbt().is_some_and(|p| {
            self.wallets
                .spend()
                .is_some_and(|s| s.spend.psbt.unsigned_tx().compute_txid() == p.txid)
        });
        if !the_spends && self.checked_not_signed(at) {
            self.nonce_check_open(c, at);
            return;
        }
        self.toast(&fill(EN.scanned_is_in_files, &[name]));
        if !matches!(
            self.screen,
            Screen::Spend | Screen::Visit | Screen::Restore | Screen::Home | Screen::Family
        ) {
            self.screen = Screen::Files;
        }
    }

    /// Whether Inbox file `k` is a transaction to check rather than to
    /// sign: a finished transaction, or a PSBT that carries a signature
    /// and that no key here is left to sign (`docs/FLOWS.md`, Nonce
    /// check). A lookup per master its origins name.
    pub fn checked_not_signed(&self, k: usize) -> bool {
        let Some(item) = self.inbox.get(k) else {
            return false;
        };
        match item.kind {
            wallet::FileKind::Transaction => true,
            wallet::FileKind::Psbt => item.psbt().is_some_and(|p| {
                p.signed_elsewhere(|fp| self.session.keys.key_label(fp).is_some())
            }),
            _ => false,
        }
    }

    /// Costs a PSBT read, what [`wallet::Session::nonce_check`] costs and
    /// a finalize of a copy. Opens the nonce check of Inbox file `k`, a
    /// signed PSBT or a finished transaction, its way back to the screen
    /// on show; a file that reads as neither opens nothing.
    pub(crate) fn nonce_check_open(&mut self, c: &Compute, k: usize) {
        let back = match self.screen {
            Screen::NonceCheck | Screen::Decode => {
                self.decode.outside().map_or(Screen::Catalog, |o| o.back)
            }
            s => s,
        };
        let Some(item) = self.inbox.get(k) else {
            return;
        };
        let (name, bytes, kind) = (item.name.clone(), item.bytes.clone(), item.kind);
        let (psbt, raw, prevouts) = match kind {
            wallet::FileKind::Psbt => {
                let Some(p) = wallet::read_psbt(c, &bytes) else {
                    return;
                };
                let from = AmountsFrom::Psbt(name.clone());
                let prevouts = decode::prevouts_of(&p);
                (p, None, (prevouts, from))
            }
            wallet::FileKind::Transaction => {
                let Some(tx) = faraday_psbt::transaction::read_raw(&bytes) else {
                    return;
                };
                let (prevouts, from) = self.amounts_in(c, &tx);
                let Some((p, tx)) = raw_as_psbt(c, &bytes, &prevouts) else {
                    return;
                };
                (p, Some(tx), (prevouts, from))
            }
            _ => return,
        };
        let mut check = self.session.nonce_check(c, &psbt);
        check.unread = unread(&psbt);
        // A wrong signature or a nonce used twice refuses the
        // transaction: nothing finished is offered.
        let finished = if check.wrong() {
            None
        } else if let Some(tx) = &raw {
            Some(tx.clone())
        } else {
            let mut copy = psbt.clone();
            faraday_psbt::finalize(&mut copy).ok().flatten()
        };
        let hex = finished.as_ref().map(serialize_hex);
        let out = match (&raw, &hex) {
            (None, Some(h)) => (
                format!("{}-final.txn", crate::result_stem(&name)),
                h.clone().into_bytes(),
            ),
            _ => (name.clone(), bytes),
        };
        self.decode.outside = Some(Box::new(OutsideCheck {
            source: name,
            check,
            finished,
            hex,
            out,
            back,
            show_hex: false,
            made_with: self.session.inspect_inputs(),
            psbt,
            prevouts,
        }));
        self.screen = Screen::NonceCheck;
        self.list_offset = 0.0;
    }

    /// Costs what [`wallet::Session::nonce_check`] costs when the keys or
    /// wallets changed. Decode's way back to the nonce check: the check
    /// made again when the keys or wallets are not those it was made with
    /// ([`OutsideCheck`]), so it names no key that has gone and misses
    /// none that came.
    pub(crate) fn nonce_check_back(&mut self, c: &Compute) {
        let now = self.session.inspect_inputs();
        if let Some(o) = self.decode.outside.as_mut()
            && o.made_with != now
        {
            o.check = self.session.nonce_check(c, &o.psbt);
            o.check.unread = unread(&o.psbt);
            if o.check.wrong() {
                o.finished = None;
                o.hex = None;
            }
            o.made_with = now;
        }
        self.screen = Screen::NonceCheck;
        self.list_offset = 0.0;
    }

    /// Costs what the presses below cost: the nonce check's own presses
    /// ([`CheckPress`]).
    pub(crate) fn nonce_check_act(&mut self, press: CheckPress, c: &Compute) {
        match press {
            CheckPress::Inbox(k) => self.nonce_check_open(c, k),
            CheckPress::Back => self.nonce_check_back(c),
            CheckPress::Hex => {
                if let Some(o) = self.decode.outside.as_mut() {
                    o.show_hex = !o.show_hex;
                }
            }
            CheckPress::Qr => {
                let Some(o) = self.decode.outside.as_deref().filter(|o| !o.check.wrong()) else {
                    return;
                };
                let view = match &o.hex {
                    Some(hex) => QrView::text(c, EN.act_transaction, hex),
                    None => QrView::psbt(c, &o.source, &o.psbt.to_bytes()),
                };
                self.open_qr(view);
            }
            // A transaction the check refuses is not offered to carry on.
            CheckPress::Out => {
                if let Some(o) = self.decode.outside.as_ref()
                    && !o.check.wrong()
                {
                    let (name, bytes) = o.out.clone();
                    self.put_outbox(&name, bytes);
                    self.toast_out(&name);
                }
            }
            CheckPress::Decode => {
                let Some(o) = self.decode.outside.as_ref() else {
                    return;
                };
                let Some(tx) = o.finished.as_ref() else {
                    return;
                };
                let (prevouts, from) = &o.prevouts;
                match decode::decode(c, tx, prevouts, &self.session, &o.source) {
                    Ok(mut decoded) => {
                        decoded.amounts_from = from.clone();
                        self.decode.show(decoded, &mut self.screen);
                        self.list_offset = 0.0;
                    }
                    Err(line) => self.toast(line),
                }
            }
        }
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
                    match decode::decode(c, tx, &prevouts, &self.session, &source) {
                        Ok(mut decoded) => {
                            decoded.amounts_from = AmountsFrom::Psbt(s.spend.source.to_string());
                            self.decode.show(decoded, &mut self.screen);
                        }
                        Err(line) => self.toast(line),
                    }
                }
            }
            Action::DecodeInbox(k) => {
                if let Some(item) = self.inbox.get(k)
                    && let Some(tx) = faraday_psbt::transaction::read_raw(&item.bytes)
                {
                    let (prevouts, from) = self.amounts_in(c, &tx);
                    match decode::decode(c, &tx, &prevouts, &self.session, &item.name) {
                        Ok(mut decoded) => {
                            decoded.amounts_from = from;
                            self.decode.show(decoded, &mut self.screen);
                            self.list_offset = 0.0;
                        }
                        Err(line) => self.toast(line),
                    }
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
