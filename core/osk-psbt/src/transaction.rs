//! A transaction as it arrives to be checked rather than signed: a raw
//! transaction in hex or in bytes, or a PSBT read as the transaction it
//! carries, and the previous output each of its inputs spends as far as
//! the source states them.
//!
//! A check that needs every previous output — whether a transaction
//! pays a silent payments wallet (`docs/PLANNING.md` §16.113) — is
//! handed the previous transactions one at a time
//! ([`Spending::add_previous`]) until none is missing.

use alloc::vec;
use alloc::vec::Vec;

use bitcoin::hex::FromHex;
use bitcoin::psbt::Psbt;
use bitcoin::{ScriptBuf, Transaction, Txid, Witness, consensus};

/// A raw transaction, in hex text or in bytes, with whatever signature
/// data it carries.
pub fn read_raw(bytes: &[u8]) -> Option<Transaction> {
    let trimmed = bytes.trim_ascii();
    let raw = match core::str::from_utf8(trimmed)
        .ok()
        .and_then(|t| Vec::<u8>::from_hex(t.trim()).ok())
    {
        Some(decoded) => decoded,
        None => trimmed.to_vec(),
    };
    consensus::deserialize(&raw).ok()
}

/// Whether an input carries signature data: a scriptSig, a witness, or
/// both.
pub fn is_signed(script_sig: &ScriptBuf, witness: &Witness) -> bool {
    !script_sig.is_empty() || !witness.is_empty()
}

/// A transaction and the previous outputs its inputs spend, each one
/// known or not yet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Spending {
    tx: Transaction,
    prevouts: Vec<Option<ScriptBuf>>,
}

impl Spending {
    /// The transaction `bytes` carry, with whatever previous outputs it
    /// states itself.
    ///
    /// A PSBT, binary or base64, states them: `witness_utxo` is the
    /// output an input spends, and `non_witness_utxo` the whole
    /// transaction it came from. Its transaction carries the signature
    /// data each input has been finalized with. A raw transaction states
    /// no previous output.
    pub fn read(bytes: &[u8]) -> Option<Spending> {
        if let Some(spending) = Spending::from_psbt(bytes) {
            return Some(spending);
        }
        let tx = read_raw(bytes)?;
        let prevouts = vec![None; tx.input.len()];
        Some(Spending { tx, prevouts })
    }

    fn from_psbt(bytes: &[u8]) -> Option<Spending> {
        let trimmed = bytes.trim_ascii();
        let psbt = if trimmed.starts_with(b"psbt\xff") {
            Psbt::deserialize(trimmed).ok()?
        } else {
            let text = core::str::from_utf8(trimmed).ok()?;
            Psbt::deserialize(&osk_bip::base64::decode(text.trim()).ok()?).ok()?
        };
        let mut tx = psbt.unsigned_tx.clone();
        let mut prevouts = Vec::new();
        for (i, input) in psbt.inputs.iter().enumerate() {
            if let Some(script) = &input.final_script_sig {
                tx.input[i].script_sig = script.clone();
            }
            if let Some(witness) = &input.final_script_witness {
                tx.input[i].witness = witness.clone();
            }
            prevouts.push(prevout_of(&psbt, i));
        }
        Some(Spending { tx, prevouts })
    }

    /// The transaction.
    pub fn tx(&self) -> &Transaction {
        &self.tx
    }

    /// The previous transaction of the first input whose previous output
    /// is not known.
    pub fn waiting_for(&self) -> Option<Txid> {
        let i = self.prevouts.iter().position(Option::is_none)?;
        Some(self.tx.input[i].previous_output.txid)
    }

    /// Takes a previous transaction, raw, filling in every input that
    /// spends it. `false` where the bytes are no transaction, or none an
    /// input spends.
    pub fn add_previous(&mut self, bytes: &[u8]) -> bool {
        let Some(previous) = read_raw(bytes) else {
            return false;
        };
        let txid = previous.compute_txid();
        let mut used = false;
        for (i, input) in self.tx.input.iter().enumerate() {
            if input.previous_output.txid != txid {
                continue;
            }
            let Some(out) = previous.output.get(input.previous_output.vout as usize) else {
                continue;
            };
            self.prevouts[i] = Some(out.script_pubkey.clone());
            used = true;
        }
        used
    }

    /// The previous outputs, in input order, once every one is known.
    pub fn prevouts(&self) -> Option<Vec<&ScriptBuf>> {
        self.prevouts.iter().map(Option::as_ref).collect()
    }
}

/// The output a PSBT input spends, where the PSBT states it.
fn prevout_of(psbt: &Psbt, i: usize) -> Option<ScriptBuf> {
    let input = psbt.inputs.get(i)?;
    if let Some(utxo) = &input.witness_utxo {
        return Some(utxo.script_pubkey.clone());
    }
    let tx = input.non_witness_utxo.as_ref()?;
    let vout = psbt.unsigned_tx.input.get(i)?.previous_output.vout;
    Some(tx.output.get(vout as usize)?.script_pubkey.clone())
}
