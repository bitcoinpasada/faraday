//! Decode a transaction: a finished
//! transaction taken apart field by field, so the person can see that it
//! spends what they meant: its inputs, each output's amount, address and
//! script type, which outputs come back to a loaded wallet, the witness
//! stacks, the locktime, and the fee when the amounts going in are known.
//!
//! The parsing is rust-bitcoin's, through `faraday_psbt::transaction`; this
//! module only arranges what it read.

pub mod check;
pub(crate) mod check_others;
pub(crate) mod check_screen;
pub(crate) mod check_words;
pub(crate) mod diff_words;
pub(crate) mod screen;

use crate::strings::EN;
use faraday_bip::bitcoin::consensus::encode::serialize_hex;
use faraday_bip::bitcoin::{Address, Amount, Script, ScriptBuf, Transaction, TxOut};
use faraday_bip::keys::Network;
use faraday_crypto::work::Compute;
use faraday_psbt::transaction::money_total;
use faraday_text::{Text, text};
use zeroize::Zeroizing;

use crate::Screen;
use crate::wallet::Session;
use crate::{Action, Faraday, Item, QrView, decode, wallet};

/// One input, as decoded.
#[derive(Debug, Clone)]
pub struct DecodedInput {
    /// The output it spends, `txid:vout`.
    pub outpoint: Text,
    /// Its sequence, and what it says.
    pub sequence: Text,
    /// The amount it spends, when the source says.
    pub amount: Option<u64>,
    /// The witness items, in hex.
    pub witness: Vec<Text>,
    /// The scriptSig's length in bytes.
    pub script_sig: usize,
}

/// One output, as decoded.
#[derive(Debug, Clone)]
pub struct DecodedOutput {
    /// Its amount in satoshis.
    pub amount: u64,
    /// Its address on the session's network, or the script in hex.
    pub address: Text,
    /// The script's type.
    pub kind: &'static str,
    /// The loaded wallet it pays: its name, change or receive, index.
    pub ours: Option<(Text, bool, u32)>,
    /// Its script, to name the address and the wallet again when a wallet
    /// is loaded after the decoding.
    pub script: ScriptBuf,
}

/// A transaction taken apart.
#[derive(Debug, Clone)]
pub struct Decoded {
    /// Where it came from: a file's name, or the spend it finished.
    pub source: Text,
    /// The transaction id.
    pub txid: Text,
    /// The witness transaction id.
    pub wtxid: Text,
    /// The version.
    pub version: i32,
    /// The locktime, and what it says.
    pub locktime: Text,
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
    pub hex: Text,
}

/// Where a decoded transaction's amounts going in came from: a PSBT
/// whose unsigned transaction is exactly this one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AmountsFrom {
    /// This PSBT, by its file's name or the spend's.
    Psbt(Text),
    /// No PSBT of this transaction is here.
    None,
    /// PSBTs of this transaction state different amounts: none is used.
    Disagree,
}

impl AmountsFrom {
    /// The row's value.
    pub fn text(&self) -> Text {
        match self {
            AmountsFrom::Psbt(name) => Text::fill(EN.decode_amounts_from, &[name]),
            AmountsFrom::None => Text::of(EN.decode_amounts_none),
            AmountsFrom::Disagree => Text::of(EN.decode_amounts_disagree),
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
pub(crate) fn sequence_text(n: u32) -> Text {
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
    text!("0x{n:08x} · {what}")
}

/// What a locktime says.
pub(crate) fn locktime_text(n: u32) -> Text {
    match n {
        0 => Text::of(EN.decode_0_none),
        n if n < 500_000_000 => Text::fill(EN.decode_block_height, &[&Text::num(u64::from(n))]),
        n => Text::fill(EN.decode_unix_time, &[&Text::num(u64::from(n))]),
    }
}

/// Costs an address derivation: the loaded wallet `address` pays, its
/// name, whether it is change and its index.
fn ours_of(c: &Compute, session: &Session, address: &Address) -> Option<(Text, bool, u32)> {
    let network: Network = session.network();
    session.wallets.iter().find_map(|w| {
        w.policy()
            .find_address(c, network, address, 100)
            .map(|(change, index)| (Text::of(&w.name), change, index))
    })
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
            outpoint: text!(
                "{}:{}",
                txin.previous_output.txid,
                txin.previous_output.vout
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
            let ours = address.as_ref().ok().and_then(|a| ours_of(c, session, a));
            DecodedOutput {
                amount: o.value.to_sat(),
                address: address
                    .map(|a| text!("{a}"))
                    .unwrap_or_else(|_| hex(o.script_pubkey.as_bytes())),
                kind: script_kind(&o.script_pubkey),
                ours,
                script: o.script_pubkey.clone(),
            }
        })
        .collect();
    Ok(Decoded {
        source: Text::of(source),
        txid: text!("{}", tx.compute_txid()),
        wtxid: text!("{}", tx.compute_wtxid()),
        version: tx.version.0,
        locktime: locktime_text(tx.lock_time.to_consensus_u32()),
        size: (tx.total_size(), tx.vsize(), tx.weight().to_wu()),
        inputs,
        outputs,
        fee: paid_in.and_then(|i| i.checked_sub(paid_out)),
        amounts_from: AmountsFrom::None,
        hex: Text::of(&serialize_hex(tx)),
    })
}

/// Costs a classify per input. The outputs a PSBT's inputs spend, in
/// order, as far as it says: read as its own checks read them
/// ([`faraday_psbt::verify::spent_outputs`]).
pub fn prevouts_of(psbt: &faraday_psbt::Psbt) -> Vec<Option<TxOut>> {
    faraday_psbt::verify::spent_outputs(psbt.inner())
}

/// Satoshis as bitcoin, eight places.
pub fn btc(sats: u64) -> Text {
    Text::fill(
        EN.decode_btc,
        &[&text!("{}", Amount::from_sat(sats).to_btc())],
    )
}

fn hex(bytes: impl AsRef<[u8]>) -> Text {
    faraday_crypto::hex::encode_string(bytes.as_ref())
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
    pub source: Text,
    /// What the check found.
    pub check: crate::wallet::NonceCheck,
    /// The finished transaction: the one that came, or the PSBT
    /// finished, when every input is final or has its signatures and no
    /// signature is wrong. `None` when the check refuses it or more
    /// signatures are wanted.
    pub finished: Option<Transaction>,
    /// The finished transaction in hex, kept with it.
    pub hex: Option<Text>,
    /// The name the file went out under, beside any other file of its name;
    /// `None` until Save to a stick.
    pub out_name: Option<Text>,
    /// The file Save to a stick puts For the stick: its name and bytes.
    pub out: (Text, Zeroizing<Vec<u8>>),
    /// The screen its back link returns to.
    pub back: Screen,
    /// The hex is shown.
    pub show_hex: bool,
    /// `finished`, `hex` and `out` as worked out with no regard to the
    /// nonce history, put back when the history stops refusing the check.
    worked: Worked,
    /// The keys and wallets it was checked with.
    made_with: crate::wallet::InspectInputs,
    /// The PSBT checked (a raw transaction wrapped, its signatures
    /// final), kept to check again.
    psbt: faraday_psbt::Psbt,
    /// What the inputs spend, as far as is known, and from where, for
    /// Decode.
    prevouts: (Vec<Option<TxOut>>, AmountsFrom),
}

/// What [`OutsideCheck`] offers when the nonce history refuses nothing.
struct Worked {
    finished: Option<Transaction>,
    hex: Option<Text>,
    out: (Text, Zeroizing<Vec<u8>>),
}

impl OutsideCheck {
    /// Costs a map lookup per signature. Compares the check with the
    /// history `h` when the history has changed since: what is offered
    /// goes when the check turns wrong, and comes back as worked out when
    /// it turns right again. Whether it was compared again.
    pub(crate) fn sync(&mut self, h: &crate::memory::nonces::HistorySnapshot) -> bool {
        if self.check.history_gen == h.generation() {
            return false;
        }
        self.check.against(h);
        if self.check.wrong() {
            self.finished = None;
            self.hex = None;
        } else {
            self.finished = self.worked.finished.clone();
            self.hex = self.worked.hex.clone();
            self.out = self.worked.out.clone();
        }
        true
    }
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
    /// The tool is open with no transaction chosen: the files to choose
    /// from, and the screen its back link and Done return to.
    landing: Option<Screen>,
    /// Boxed: it holds a PSBT and its check, and `Faraday` is built on
    /// the stack.
    outside: Option<Box<OutsideCheck>>,
    /// Check a signer's flow: two PSBTs or transactions, their comparison
    /// and the report on them, public, kept until a lock or power-off. Boxed as `outside` is.
    check: Option<Box<check::CheckState>>,
}

impl DecodeArea {
    /// The screen the landing returns to, while the tool is open with no
    /// transaction chosen.
    pub(crate) fn landing(&self) -> Option<Screen> {
        self.landing
    }

    /// The transaction on show.
    pub(crate) fn state(&self) -> Option<&DecodeState> {
        self.state.as_ref()
    }

    /// Shows `decoded` on the Decode screen, whose back link returns to
    /// the screen on show.
    pub(crate) fn show(&mut self, decoded: Decoded, screen: &mut Screen) {
        // Chosen on the landing: back is where the tool was opened from.
        let landing = self.landing.take();
        let back = if *screen == Screen::Decode {
            landing.unwrap_or(*screen)
        } else {
            *screen
        };
        self.state = Some(DecodeState {
            decoded,
            back,
            show_hex: false,
        });
        *screen = Screen::Decode;
    }

    /// Costs an address derivation per output. The outputs named again
    /// after a wallet was loaded: its network's address and whose it is.
    pub(crate) fn reread_ours(&mut self, c: &Compute, session: &Session) {
        let network: Network = session.network();
        let Some(d) = self.state.as_mut() else {
            return;
        };
        for o in &mut d.decoded.outputs {
            let net = faraday_bip::bitcoin::Network::from(network);
            if let Ok(a) = Address::from_script(&o.script, net) {
                o.address = text!("{a}");
                o.ours = ours_of(c, session, &a);
            }
        }
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

    /// Check a signer's state, while the flow is kept.
    pub(crate) fn check(&self) -> Option<&check::CheckState> {
        self.check.as_deref()
    }

    /// How far Check a signer's column is scrolled, units, while the flow is open.
    pub(crate) fn check_scroll_y(&self) -> Option<f32> {
        self.check.as_ref().map(|x| x.scroll.y)
    }

    /// Check a signer's column scrolled to `y` units, while the flow is open.
    pub(crate) fn set_check_scroll_y(&mut self, y: f32) {
        if let Some(x) = self.check.as_mut() {
            x.scroll.y = y;
        }
    }

    /// Forgets Check a signer's flow: its two sides and the result.
    pub(crate) fn clear_check(&mut self) {
        self.check = None;
    }

    /// Takes the column's scroll as the drawing pass left it.
    pub(crate) fn set_check_scroll(&mut self, to: crate::flow::Scroll) {
        if let Some(c) = self.check.as_mut() {
            c.scroll = to;
        }
    }

    /// The nonce check of a transaction brought in, while it is kept.
    pub(crate) fn outside(&self) -> Option<&OutsideCheck> {
        self.outside.as_deref()
    }
}

/// Costs a PSBT parse (through `parse_bytes`). A raw transaction as a
/// PSBT whose inputs are final with the transaction's own `scriptSig`s
/// and witnesses, each with the output it spends where `prevouts` states
/// it: what `faraday_psbt::verify` reads a finalized input from. `None`
/// where the bytes are no transaction.
pub(crate) fn raw_as_psbt(
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
    /// Costs a txid, a PSBT read for each Inbox PSBT of the same txid, and a
    /// txid per input with a previous transaction in each.
    /// The amounts `tx`'s inputs spend, from every PSBT here whose
    /// unsigned transaction is exactly `tx` ([`same_unsigned`]): the
    /// spend's, then the Inbox's. An input's amount is used when no two of
    /// them state different ones for it (one that states none does not
    /// count), and the first that states one is named.
    pub(crate) fn amounts_in(
        &self,
        c: &Compute,
        tx: &Transaction,
    ) -> (Vec<Option<TxOut>>, AmountsFrom) {
        // The unsigned transaction's id: `tx` with its scriptSigs and
        // witnesses taken off, as a PSBT holds it.
        let mut bare = tx.clone();
        for i in &mut bare.input {
            i.script_sig = faraday_bip::bitcoin::ScriptBuf::new();
            i.witness = faraday_bip::bitcoin::Witness::new();
        }
        let txid = bare.compute_txid();
        let mut found: Vec<(Text, Vec<Option<TxOut>>)> = Vec::new();
        if let Some(s) = self.wallets.spend()
            && same_unsigned(s.spend.psbt.unsigned_tx(), tx)
        {
            found.push((
                Text::of(&s.spend.source),
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
                found.push((Text::of(&i.name), decode::prevouts_of(&p)));
            }
        }
        // Input by input, the amount any of them states: one that leaves
        // it out does not disagree, two that state different ones do.
        let mut amounts: Vec<Option<TxOut>> = vec![None; tx.input.len()];
        let mut name: Option<Text> = None;
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

    /// Whether the Decode screen shows a transaction opened from the open
    /// spend while the nonce history or a nonce used twice holds the spend
    /// back. A lookup.
    pub fn decode_from_spend_held(&self) -> bool {
        self.decode
            .state()
            .is_some_and(|d| matches!(d.back, Screen::Spend | Screen::SpendBackup))
            && self.wallets.spend().is_some_and(|s| s.spend.nonce_hold)
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
        self.toast(&Text::fill(EN.scanned_is_in_files, &[name]));
        if !matches!(
            self.screen,
            Screen::Spend | Screen::Restore | Screen::Home | Screen::SpendBackup
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
                let from = AmountsFrom::Psbt(Text::of(&name));
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
        // Costs a map lookup per signature: each against the nonce history.
        check.against(&self.nonce_snapshot());
        self.note_nonces(&check);
        // A wrong signature, a witness or a nonce used twice refuses the
        // transaction: nothing finished is offered. What would be offered
        // with the history set aside is kept, for when it stops refusing.
        let mut plain = check.clone();
        plain.used_before.clear();
        let finished = if plain.wrong() {
            None
        } else if let Some(tx) = &raw {
            Some(tx.clone())
        } else if plain.unread > 0 {
            // An input whose output does not read, or states one falsely,
            // is one nothing of can be checked: no transaction is finished
            // from the PSBT.
            None
        } else {
            let mut copy = psbt.clone();
            faraday_psbt::finalize(&mut copy).ok().flatten()
        };
        let hex = finished.as_ref().map(|t| Text::of(&serialize_hex(t)));
        let out = match (&raw, &hex) {
            (None, Some(h)) => (
                text!("{}-final.txn", crate::result_stem(&name).as_str()),
                Zeroizing::new(h.as_bytes().to_vec()),
            ),
            _ => (Text::of(&name), bytes),
        };
        let worked = Worked {
            finished: finished.clone(),
            hex: hex.clone(),
            out: out.clone(),
        };
        let (finished, hex) = if check.wrong() {
            (None, None)
        } else {
            (finished, hex)
        };
        self.decode.outside = Some(Box::new(OutsideCheck {
            source: Text::of(&name),
            check,
            finished,
            hex,
            out,
            worked,
            out_name: None,
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
        let history = self.nonce_snapshot();
        let mut noted = None;
        if let Some(o) = self.decode.outside.as_mut() {
            if o.made_with != now {
                let found = std::mem::take(&mut o.check.used_before);
                o.check = self.session.nonce_check(c, &o.psbt);
                // A refusal found stays while the page holds the file (F227).
                o.check.used_before = found;
                o.made_with = now;
                o.check.history_gen = u64::MAX;
            }
            // The history may have changed since the check was made.
            o.sync(&history);
            noted = Some(o.check.entries());
        }
        if let (Some(entries), Some(o)) = (noted, self.decode.outside.as_ref()) {
            let tx = o.check.tx;
            self.memory.nonces.note(tx, &entries);
        }
        self.screen = Screen::NonceCheck;
        self.list_offset = 0.0;
    }

    /// Costs a map lookup per signature. The nonce check page's check
    /// compared again when the history changed under it (a vault opened or
    /// closed, the setting turned), and what it now says noted. Run after
    /// every input and before the presses that carry the check on.
    pub(crate) fn nonce_outside_sync(&mut self) {
        let history = self.nonce_snapshot();
        let Some(o) = self.decode.outside.as_mut() else {
            return;
        };
        if o.sync(&history) {
            let (tx, entries) = (o.check.tx, o.check.entries());
            self.memory.nonces.note(tx, &entries);
        }
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
                self.nonce_outside_sync();
                let Some(o) = self
                    .decode
                    .outside
                    .as_deref()
                    .filter(|o| !o.check.refused(o.hex.is_some()))
                else {
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
                self.nonce_outside_sync();
                if let Some(o) = self.decode.outside.as_ref()
                    && !o.check.refused(o.hex.is_some())
                {
                    let (name, bytes) = o.out.clone();
                    let name = self.put_outbox_beside(&name, bytes);
                    if let Some(o) = self.decode.outside.as_mut() {
                        o.out_name = Some(Text::of(&name));
                    }
                    self.toast_out(&name);
                }
            }
            CheckPress::Decode => {
                self.nonce_outside_sync();
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

    /// Costs a PSBT read or a transaction's decode. Opens the Decode page
    /// on a file's transaction: a finished one as it is, a PSBT as the
    /// transaction it carries with the amounts it states. A file that is
    /// neither, or a PSBT that is refused, opens nothing.
    pub(crate) fn decode_file(
        &mut self,
        c: &Compute,
        name: &str,
        bytes: &[u8],
        kind: wallet::FileKind,
    ) {
        let (tx, prevouts, from) = match kind {
            wallet::FileKind::Psbt => {
                let Some(p) = wallet::read_psbt(c, bytes) else {
                    return;
                };
                let prevouts = decode::prevouts_of(&p);
                (
                    p.unsigned_tx().clone(),
                    prevouts,
                    AmountsFrom::Psbt(Text::of(name)),
                )
            }
            _ => {
                let Some(tx) = faraday_psbt::transaction::read_raw(bytes) else {
                    return;
                };
                let (prevouts, from) = self.amounts_in(c, &tx);
                (tx, prevouts, from)
            }
        };
        match decode::decode(c, &tx, &prevouts, &self.session, name) {
            Ok(mut decoded) => {
                decoded.amounts_from = from;
                self.decode.show(decoded, &mut self.screen);
                self.list_offset = 0.0;
            }
            Err(line) => self.toast(line),
        }
    }

    /// Costs what the actions below cost, the keys' and wallets' work among
    /// it: handles the Decode screen's actions (`DecodeFinished`, `DecodeInbox`, `QrDecoded`, `DecodeHex`, `SpendHex`).
    pub(crate) fn decode_act(&mut self, action: Action, c: &Compute) {
        match action {
            Action::NonceCheck(p) => self.nonce_check_act(p, c),
            Action::SignerCheck(p) => self.signer_check_act(p, c),
            Action::DecodeFinished => {
                if let Some(s) = self.wallets.spend()
                    && let Some(tx) = s.spend.finished.as_ref()
                {
                    let prevouts = decode::prevouts_of(&s.spend.psbt);
                    let source = Text::fill(EN.act_finished_from_source, &[&s.spend.source]);
                    match decode::decode(c, tx, &prevouts, &self.session, &source) {
                        Ok(mut decoded) => {
                            decoded.amounts_from = AmountsFrom::Psbt(Text::of(&s.spend.source));
                            self.decode.show(decoded, &mut self.screen);
                        }
                        Err(line) => self.toast(line),
                    }
                }
            }
            Action::DecodeInbox(k) => {
                if let Some(item) = self.inbox.get(k) {
                    let (name, bytes, kind) = (item.name.clone(), item.bytes.clone(), item.kind);
                    self.decode_file(c, &name, &bytes, kind);
                }
            }
            Action::DecodeAnother => {
                let back = self.decode.state.as_ref().map(|d| d.back);
                if let Some(back) = back {
                    self.decode.state = None;
                    self.decode.landing = Some(back);
                    self.list_offset = 0.0;
                }
            }
            Action::QrDecoded => {
                // Opened from the nonce check page: the history is compared
                // again first, as the page's presses do.
                if self
                    .decode
                    .state
                    .as_ref()
                    .is_some_and(|d| d.back == Screen::NonceCheck)
                {
                    self.nonce_outside_sync();
                    if self
                        .decode
                        .outside
                        .as_deref()
                        .is_some_and(|o| o.check.refused(o.hex.is_some()))
                    {
                        return;
                    }
                }
                // Opened from the spend: held back, it gives out no code.
                if self.decode_from_spend_held() {
                    return;
                }
                if let Some(hex) = self.decode.hex() {
                    self.open_qr(QrView::text(c, EN.act_transaction, hex));
                }
            }
            Action::DecodeHex => {
                self.decode.toggle_hex();
            }
            Action::SpendHex => {
                self.wallets.toggle_show_hex();
            }
            // `act_inner` passes only the actions matched above.
            _ => unreachable!(),
        }
    }
}

/// Check a signer: the presses of `Screen::SignerCheck`
/// (`docs/SIGNER-CHECK.md` §3).
impl Faraday {
    /// Costs what `act` costs. The Nonce check tile: the first PSBT in Files not signed here, or
    /// the scanner.
    pub(crate) fn nonce_check_tile(&mut self) {
        match (0..self.inbox.len()).find(|k| self.checked_not_signed(*k)) {
            Some(k) => self.act(Action::NonceCheck(CheckPress::Inbox(k))),
            None => self.act(Action::Scan),
        }
    }

    /// Costs what `act` costs. The Tools tile opens the flow.
    pub(crate) fn signer_check_tile(&mut self) {
        self.act(Action::SignerCheck(check::SignerPress::Open));
    }

    /// Whether Check a signer's flow is kept: its two PSBTs and report.
    pub fn signer_check_kept(&self) -> bool {
        self.decode.check().is_some()
    }

    /// Check a signer's state, while the flow is kept.
    pub(crate) fn signer_check(&self) -> Option<&check::CheckState> {
        self.decode.check()
    }

    /// Costs what reading a file and the run cost, when a press completes
    /// the second card. Does what the press does.
    pub(crate) fn signer_check_act(&mut self, press: check::SignerPress, c: &Compute) {
        use check::SignerPress as P;
        match press {
            P::Open => self.signer_check_open(c),
            P::Card(n) => {
                let ready = self.decode.check().is_some_and(|s| s.shown.is_some());
                if let Some(s) = self.decode.check.as_mut()
                    && (n < check::RESULT || ready)
                {
                    s.open_card(n);
                }
            }
            P::Pick(slot, k) => {
                let picked = {
                    let all = check::files(&self.inbox, &self.outbox);
                    (k < all.len()).then(|| Self::check_file_at(&all, k))
                };
                if let Some((bytes, name, origin)) = picked {
                    self.signer_check_take(c, slot, &bytes, name, origin);
                }
            }
            P::Scan(slot) => {
                let mut scan = crate::qr_view::ScanState::default();
                scan.purpose = match slot {
                    check::Slot::Sent => crate::ScanPurpose::CheckSent,
                    check::Slot::Returned => crate::ScanPurpose::CheckReturned,
                };
                self.camera.open(scan);
                self.sheet = Some(crate::Sheet::Scan);
                self.out
                    .commands
                    .push_back(faraday_shell_api::Command::CameraOn);
            }
            P::Back => {
                if let Some(s) = self.decode.check() {
                    self.screen = s.back;
                    self.list_offset = 0.0;
                }
            }
            P::Qr => {
                let returned = self
                    .decode
                    .check()
                    .and_then(|s| s.returned.as_ref())
                    .filter(|r| r.version.is_some())
                    .map(|r| r.bytes.clone());
                if let Some(bytes) = returned {
                    self.open_qr(QrView::psbt(c, EN.signer_returned, &bytes));
                }
            }
            P::Swap => {
                if let Some(s) = self.decode.check.as_mut() {
                    s.swap();
                    self.signer_check_run(c);
                }
            }
            P::Again => {
                if let Some(s) = self.decode.check.as_mut() {
                    s.open_card(check::Slot::Returned.card());
                }
            }
            // Add a key opens with this screen as its way back; the key
            // loaded makes the result again (`signer_check_sync`).
            P::Load(_) => self.act(Action::Entry(None)),
            P::Seed(n) => {
                let fp = self
                    .decode
                    .check()
                    .and_then(|s| s.shown.as_ref())
                    .and_then(|s| s.seeds.get(n).copied());
                if let (Some(fp), true) = (fp, self.may_load_keys()) {
                    if let Some(s) = self.decode.check.as_mut() {
                        s.seed = Some(fp);
                    }
                    self.sheet = Some(crate::Sheet::SignerSeed);
                }
            }
            P::SeedWords | P::SeedScan => {
                let fp = self.decode.check().and_then(|s| s.seed);
                self.sheet = None;
                if let Some(fp) = fp {
                    self.act(Action::Entry(Some(fp.0)));
                    if matches!(press, P::SeedScan) {
                        self.act(Action::ScanSeed);
                    }
                }
            }
            P::SeedVault => {
                self.sheet = None;
                self.signer_seed_from_vault(true);
            }
            P::File(outbox, k) => self.signer_check_file(c, outbox, k),
            P::FromReview(outbox, k) => self.signer_check_from_review(c, outbox, k),
        }
    }

    /// Costs what the load of a key from a vault costs. The seed the
    /// sheet is for, loaded from the open vault that holds it; where none
    /// does and `list` is true, the vaults' list opens to unlock one,
    /// coming back here (a vault unlocked loads the seed on its return).
    pub(crate) fn signer_seed_from_vault(&mut self, list: bool) {
        use crate::vault::{SlotOf, VaultAction as V};
        let Some(fp) = self.decode.check().and_then(|s| s.seed) else {
            return;
        };
        let c = self.compute();
        let slot = wallet::Slot::bare(fp);
        if let Some((v, rec)) = self.vault_key_for(&c, &slot) {
            // The seed loads with no sheet, or the vault's Load sheet
            // comes up with it ticked; the Result checks again after.
            if !self.signer_load_sheet(v, rec) {
                self.vault_act(V::LoadKeyOf(fp.0, SlotOf::Bare));
            }
        } else if list {
            self.vault_act(V::ListFrom(Screen::SignerCheck));
        }
    }

    /// Costs what the default's reading of two files and the run cost.
    /// Opens the flow, closed on the default pair where Files has one.
    fn signer_check_open(&mut self, c: &Compute) {
        // A flow kept opens as it was (its result is the one against the
        // keys loaded now: `signer_check_sync`), with the way back set
        // anew. An empty one (no side chosen) is not one to keep: the
        // default is worked out anew.
        if let Some(s) = self.decode.check.as_mut()
            && (s.sent.is_some() || s.returned.is_some())
        {
            s.back = self.screen;
            s.refused = [None, None];
            self.screen = Screen::SignerCheck;
            self.list_offset = 0.0;
            self.signer_check_sync();
            return;
        }
        let pair = {
            let all = check::files(&self.inbox, &self.outbox);
            check::default_pair(&all)
                .map(|(a, b)| (Self::check_file_at(&all, a), Self::check_file_at(&all, b)))
        };
        match pair {
            Some((sent, returned)) => self.signer_check_start(c, Some(sent), Some(returned)),
            None => self.signer_check_start(c, None, None),
        }
    }

    /// Costs what [`Faraday::signer_check_open`] costs. Files' ⋮ **Check a
    /// signer** on a PSBT or a finished transaction: the file is
    /// Returned; Sent is the one other PSBT of its transaction with fewer
    /// signatures, where there is exactly one, else the one file of
    /// another transaction that spends an input of it (SIGNER-CHECK
    /// §11.2); the result has run when Sent is found.
    fn signer_check_file(&mut self, c: &Compute, outbox: bool, k: usize) {
        let picked = {
            let all = check::files(&self.inbox, &self.outbox);
            all.iter()
                .position(|(o, i, _)| *o == outbox && *i == k)
                .map(|n| {
                    let item = all[n].2;
                    let mine = item.psbt();
                    // Exactly one other file of its transaction with fewer
                    // signatures is Sent.
                    let fewer: Vec<usize> = (0..all.len())
                        .filter(|m| {
                            *m != n
                                && mine.is_some_and(|a| {
                                    all[*m]
                                        .2
                                        .psbt()
                                        .is_some_and(|p| p.txid == a.txid && p.sigs < a.sigs)
                                })
                        })
                        .collect();
                    let sent = match fewer.as_slice() {
                        [m] => Some(Self::check_file_at(&all, *m)),
                        // Else the one file of another unsigned
                        // transaction that spends an input of it.
                        _ => item.unsigned_txid.and_then(|txid| {
                            let files = self.inbox.iter().chain(self.outbox.iter());
                            match check_others::counting(files, txid, &item.outpoints).as_slice() {
                                [(one, _)] => all
                                    .iter()
                                    .position(|(_, _, i)| std::ptr::eq(*i, *one))
                                    .map(|m| Self::check_file_at(&all, m)),
                                _ => None,
                            }
                        }),
                    };
                    (sent, Self::check_file_at(&all, n))
                })
        };
        if let Some((sent, returned)) = picked {
            self.signer_check_start(c, sent, Some(returned));
        }
    }

    /// Costs what [`Faraday::signer_check_open`] costs. The review's
    /// **Compare**: the file is Sent and the PSBT under review, named by
    /// its source, is Returned; the result has run.
    fn signer_check_from_review(&mut self, c: &Compute, outbox: bool, k: usize) {
        let item = if outbox {
            self.outbox.get(k)
        } else {
            self.inbox.get(k)
        };
        let Some(item) = item.filter(|i| {
            matches!(
                i.kind,
                wallet::FileKind::Psbt | wallet::FileKind::Transaction
            )
        }) else {
            return;
        };
        let sent = (
            item.bytes.clone(),
            Text::of(&item.name),
            check::Origin::File {
                outbox,
                name: Text::of(&item.name),
            },
        );
        // The file at this place is the one the result was made from.
        let held = self.wallets.spend().is_some_and(|s| {
            s.others_from.as_ref().is_some_and(|from| {
                from.iter().any(|(name, txid, place)| {
                    *place == (outbox, k)
                        && item.name == name.as_str()
                        && item.unsigned_txid == Some(*txid)
                })
            })
        });
        if !held {
            return;
        }
        let returned = self.wallets.spend().map(|s| {
            let bytes = s
                .spend
                .out_bytes()
                .unwrap_or_else(|_| s.spend.psbt.to_bytes());
            (
                Zeroizing::new(bytes),
                Text::of(&s.spend.source),
                check::Origin::Scanned,
            )
        });
        self.signer_check_start(c, Some(sent), returned);
    }

    /// The file at this place of [`check::files`]: its bytes, name and
    /// origin.
    fn check_file_at(
        all: &[(bool, usize, &Item)],
        n: usize,
    ) -> (Zeroizing<Vec<u8>>, Text, check::Origin) {
        let (out, _, item) = all[n];
        (
            item.bytes.clone(),
            Text::of(&item.name),
            check::Origin::File {
                outbox: out,
                name: Text::of(&item.name),
            },
        )
    }

    /// Costs what two reads and the run cost. Opens the flow from the
    /// screen on show, with these two (either may be left out).
    fn signer_check_start(
        &mut self,
        c: &Compute,
        sent: Option<(Zeroizing<Vec<u8>>, Text, check::Origin)>,
        returned: Option<(Zeroizing<Vec<u8>>, Text, check::Origin)>,
    ) {
        let back = match (self.screen, self.decode.check()) {
            (Screen::SignerCheck, Some(s)) => s.back,
            (s, _) => s,
        };
        self.decode.check = Some(Box::new(check::CheckState::new(back)));
        self.screen = Screen::SignerCheck;
        self.list_offset = 0.0;
        self.signer_check_sync();
        for (slot, file) in [(check::Slot::Sent, sent), (check::Slot::Returned, returned)] {
            if let Some((bytes, name, origin)) = file {
                self.signer_check_take(c, slot, &bytes, name, origin);
            }
        }
    }

    /// Costs a read of the file (a PSBT parse, or a transaction's) and the
    /// run when this completes the second card. Reads `bytes` as the file
    /// of `slot` (SIGNER-CHECK §11.1), or says on the card why not.
    /// Returns whether it was one.
    pub(crate) fn signer_check_take(
        &mut self,
        c: &Compute,
        slot: check::Slot,
        bytes: &[u8],
        name: Text,
        origin: check::Origin,
    ) -> bool {
        // A PSBT is kept as `check::raw_bytes` has it; a finished
        // transaction as Files holds it, never through `raw_bytes`, since
        // hex text can also decode as base64.
        let read = match wallet::read_psbt_kept(c, bytes) {
            Some((psbt, v2)) => Some((
                psbt,
                Some(if v2.is_some() { 2 } else { 0 }),
                check::raw_bytes(bytes),
            )),
            None => decode::raw_as_psbt(c, bytes, &[])
                .map(|(psbt, _)| (psbt, None, Zeroizing::new(bytes.to_vec()))),
        };
        let Some((psbt, version, kept)) = read else {
            if let Some(s) = self.decode.check.as_mut() {
                s.refused[usize::from(slot.card())] = Some(Text::of(EN.compare_not_a_transaction));
            }
            return false;
        };
        let outpoints = {
            let mut spent: Vec<_> = psbt
                .unsigned_tx()
                .input
                .iter()
                .map(|i| i.previous_output)
                .collect();
            spent.sort();
            spent
        };
        let txid = psbt.unsigned_tx().compute_txid();
        let side = check::Side {
            bytes: kept,
            psbt,
            name,
            origin,
            differs: false,
            txid,
            version,
            outpoints,
        };
        let Some(s) = self.decode.check.as_mut() else {
            return false;
        };
        if s.put(slot, side) {
            self.signer_check_run(c);
        }
        true
    }

    /// Costs what `amounts_in` costs for a finished transaction, an
    /// `inspect` of each side, and where both are PSBTs what
    /// `faraday_psbt::signer::check` costs: a signature verification per
    /// signature and a finalization per finalized input. Makes the
    /// comparison and the fee bump of the two sides kept against the keys
    /// and wallets loaded, and where both are PSBTs the check, and keeps
    /// them in words.
    fn signer_check_run(&mut self, c: &Compute) {
        let refs = self.session.key_refs(c);
        let policies = self.session.policies();
        let network = self.session.network();
        let ctx = wallet::context(network, &refs, &policies);
        let made_with = self.session.inspect_inputs();
        let loaded: Vec<_> = refs.iter().map(|r| r.fingerprint).collect();
        let made = {
            let Some(s) = self.decode.check() else {
                return;
            };
            let (Some(a), Some(b)) = (&s.sent, &s.returned) else {
                return;
            };
            // A finished transaction takes the amounts it spends from the
            // PSBTs of its unsigned transaction in Files, and for an input
            // still without one from the other card's PSBT (rule I4).
            let finished = |side: &check::Side| {
                if side.version.is_some() {
                    return None;
                }
                let tx = faraday_psbt::transaction::read_raw(&side.bytes);
                let prevouts = tx.map(|t| self.amounts_in(c, &t).0).unwrap_or_default();
                Some(
                    decode::raw_as_psbt(c, &side.bytes, &prevouts)
                        .map_or_else(|| side.psbt.clone(), |(p, _)| p),
                )
            };
            let (mut oa, mut ob) = (finished(a), finished(b));
            if let Some(pa) = oa.as_mut()
                && b.version.is_some()
            {
                faraday_psbt::bump::fill_amounts(pa, &b.psbt);
            }
            if let Some(pb) = ob.as_mut()
                && a.version.is_some()
            {
                faraday_psbt::bump::fill_amounts(pb, &a.psbt);
            }
            let (pa, pb) = (
                oa.as_ref().unwrap_or(&a.psbt),
                ob.as_ref().unwrap_or(&b.psbt),
            );
            let inspect = |p: &faraday_psbt::Psbt| {
                crate::review::name_scripts(faraday_psbt::inspect(c, p, &ctx))
            };
            let (ia, ib) = (inspect(pa), inspect(pb));
            let (sa, sb) = (
                faraday_psbt::diff::Side {
                    psbt: pa,
                    inspection: &ia,
                    psbt_version: a.version,
                },
                faraday_psbt::diff::Side {
                    psbt: pb,
                    inspection: &ib,
                    psbt_version: b.version,
                },
            );
            let d = faraday_psbt::diff::list(sa, sb);
            let bump = faraday_psbt::bump::judge(sa, sb, &loaded);
            // The signer check only where both are PSBTs.
            let report = (a.version.is_some() && b.version.is_some()).then(|| {
                let report =
                    faraday_psbt::signer::check(c, &a.psbt, &b.psbt, &a.bytes, &b.bytes, &ctx);
                // The nonce lines: every added signature recomputed with
                // the keys loaded, through the session (no key leaves it).
                let view = faraday_psbt::signer::nonce_view(&a.psbt, &b.psbt);
                let found = self.session.recompute_all(c, &view);
                faraday_psbt::signer::with_nonces(report, &found, &loaded)
            });
            // What the Result shows of the Returned PSBT against the
            // wallets loaded, worked out here and not while drawing.
            let shown = check::shown(
                &d,
                bump.as_ref(),
                report.as_ref().map(|r| (r, &ib)),
                network,
                b.version.is_some(),
            );
            (shown, report)
        };
        if let Some(s) = self.decode.check.as_mut() {
            s.shown = Some(made.0);
            s.report = made.1;
            s.made_with = Some(made_with);
        }
    }

    /// Costs a read of every PSBT file in Files (a base64 decode of each
    /// text one), the PSBT's parse-free reading, a transaction id for each
    /// finished transaction in Files not seen yet and what
    /// [`Faraday::signer_check_run`] costs when the keys or wallets
    /// changed. The one place every load and drop of a key
    /// reaches: `act` calls it after each press, the event loop after each
    /// input and `take_answer` after each worker answer, whichever flow
    /// loaded or forgot the key, so a kept result is the one against the
    /// keys loaded now (SIGNER-CHECK §8.2). It also marks a side whose file
    /// was replaced or taken out of Files, keeping its copy and result.
    /// Work with its token; drawing never calls it.
    pub(crate) fn signer_check_sync(&mut self) {
        if self.decode.check().is_none() {
            return;
        }
        // Only the flow drawn needs the marks and the ids, and one
        // arriving works them out through its open.
        if self.screen != Screen::SignerCheck {
            self.signer_check_stale();
            return;
        }
        let differs =
            |side: &Option<check::Side>| match side.as_ref().map(|s| (&s.origin, &s.bytes)) {
                // The same name and bytes in either box is the file.
                Some((check::Origin::File { name, .. }, bytes)) => !self
                    .inbox
                    .iter()
                    .chain(self.outbox.iter())
                    .any(|i| i.name == name.as_str() && check::same_bytes(&i.bytes, bytes)),
                _ => false,
            };
        let marks = self
            .decode
            .check()
            .map(|s| (differs(&s.sent), differs(&s.returned)));
        // The ids of the finished transactions the lists show.
        let ids: Vec<(Zeroizing<Vec<u8>>, faraday_bip::bitcoin::Txid)> = {
            let kept = self.decode.check().map(|s| &s.ids[..]).unwrap_or(&[]);
            self.inbox
                .iter()
                .chain(self.outbox.iter())
                .filter(|i| i.kind == wallet::FileKind::Transaction)
                .filter_map(|i| {
                    kept.iter()
                        .find(|(b, _)| *b == i.bytes)
                        .cloned()
                        .or_else(|| {
                            faraday_psbt::transaction::read_raw(&i.bytes)
                                .map(|t| (i.bytes.clone(), t.compute_txid()))
                        })
                })
                .collect()
        };
        if let (Some((a, b)), Some(s)) = (marks, self.decode.check.as_mut()) {
            if let Some(side) = s.sent.as_mut() {
                side.differs = a;
            }
            if let Some(side) = s.returned.as_mut() {
                side.differs = b;
            }
            s.ids = ids;
        }
        self.signer_check_stale();
    }

    /// Costs what [`Faraday::signer_check_run`] costs when the keys or
    /// wallets changed since the result was made.
    fn signer_check_stale(&mut self) {
        let stale = self.decode.check().is_some_and(|s| {
            s.shown.is_some() && s.made_with.as_ref() != Some(&self.session.inspect_inputs())
        });
        if stale {
            let rows = |app: &Self| {
                app.decode
                    .check()
                    .and_then(|s| s.shown.as_ref())
                    .map_or(0, |s| s.seeds.len())
            };
            let before = rows(self);
            let c = self.compute();
            self.signer_check_run(&c);
            // A seed loaded turns its row into a fact: the Result opens
            // at the facts, not where the row was.
            if self.screen == Screen::SignerCheck && rows(self) < before {
                self.list_offset = 0.0;
            }
        }
    }

    /// Costs what [`Faraday::signer_check_take`] costs. A code the
    /// Check a signer scanners read: a PSBT or a finished transaction goes
    /// to that card as "Scanned" and nowhere else; anything else is named
    /// on the scanner. Returns whether the scanner on screen is one of
    /// them.
    pub(crate) fn signer_check_scanned(&mut self, bytes: &[u8]) -> bool {
        let slot = match self.camera.scan().map(|s| s.purpose) {
            Some(crate::ScanPurpose::CheckSent) => check::Slot::Sent,
            Some(crate::ScanPurpose::CheckReturned) => check::Slot::Returned,
            _ => return false,
        };
        let c = self.compute();
        let name = Text::of(EN.signer_scanned);
        if self.signer_check_take(&c, slot, bytes, name, check::Origin::Scanned) {
            self.scan_close();
        } else {
            // The scanner says it; the card says nothing of it after.
            if let Some(s) = self.decode.check.as_mut() {
                s.refused[usize::from(slot.card())] = None;
            }
            self.scan_say(EN.compare_not_a_transaction);
        }
        true
    }
}

/// What an input leaves to settle.
impl Faraday {
    /// Costs what [`Faraday::signer_check_sync`] and
    /// [`Faraday::refresh_others`] cost. An input that may have loaded or
    /// dropped a key or changed Files: the backup's checklist, the
    /// review's other transactions (a file brought into Files or removed
    /// from it while the review is open, `docs/COMPARE.md` §4.3), then
    /// Check a signer's kept report.
    pub(crate) fn event_settled(&mut self) {
        self.backup_settle();
        let c = self.compute();
        self.refresh_others(&c);
        self.signer_check_sync();
    }
}

/// Costs what `act` costs. Tools' Decode a transaction: the transaction
/// itself when exactly one waits in Files, the files to choose from when
/// several do, and the empty part (Scan, Open Files, the stick) when none.
pub(crate) fn open_tile(app: &mut Faraday) {
    let waiting: Vec<usize> = (0..app.inbox.len())
        .filter(|&k| app.inbox[k].kind == wallet::FileKind::Transaction)
        .collect();
    if let [k] = waiting[..] {
        app.act(Action::DecodeInbox(k));
    } else {
        app.decode.state = None;
        let back = match app.screen {
            Screen::Decode => app.decode.landing.unwrap_or(Screen::Catalog),
            s => s,
        };
        app.decode.landing = Some(back);
        app.list_offset = 0.0;
        app.screen = Screen::Decode;
    }
}

// A decoded transaction is public; a lock drops the check under way.
impl crate::at_lock::AtLock for DecodeArea {
    fn holds(&self) -> crate::at_lock::Holds {
        crate::at_lock::Holds::NOTHING
    }
    fn lock(&mut self) {
        self.clear_check();
    }
}
