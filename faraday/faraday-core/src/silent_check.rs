//! The check that says whether a transaction pays a silent payments
//! wallet (`docs/PLANNING.md` §16.113): the transaction being read, its
//! previous outputs as far as they are known, and the answer.
//!
//! Taken from OpenSigner's app (`opensigner-core/src/silent.rs` at
//! upstream `d26b84e`) when Faraday dropped that app (`docs/DECISIONS.md`
//! F68), unchanged; Faraday's flow over it is [`crate::silent`].
//!
//! Nothing here is a secret. The scan private key a check needs is
//! derived from the loaded key where the check runs and is dropped with
//! the [`osk_bip::silent::Receiver`] that held it, so this file holds
//! only a transaction, its previous outputs and the answer.

use osk_bip::bitcoin::{Amount, ScriptBuf, Transaction, Txid};
use osk_psbt::transaction::Spending;

/// One output of a checked transaction that pays this wallet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Paid {
    /// Which output of the transaction it is.
    pub vout: u32,
    /// What it pays.
    pub amount: Amount,
    /// The label it was paid to, and `None` for the address itself.
    pub label: Option<u32>,
}

/// What a check of one transaction came to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Checked {
    /// The outputs that pay this wallet.
    Paid(Vec<Paid>),
    /// The transaction was read in full and pays nothing here.
    NotPaid,
    /// The transaction spends nothing BIP-352 derives a shared secret
    /// from, so no silent payment can be in it.
    NoInputs,
    /// An input's public key is not in the transaction: the inputs are
    /// there but the signatures are not, which is a transaction that
    /// has not been signed.
    Unsigned,
    /// The key the wallet was built on is not loaded, so the scan key
    /// cannot be derived.
    KeyNotLoaded,
}

/// The Check a payment flow: the transaction being read with its
/// previous outputs as far as they are known, and the answer once every
/// one of them is.
pub struct Check {
    spending: Spending,
    result: Option<Checked>,
}

impl Check {
    /// The transaction `bytes` carry, with whatever previous outputs it
    /// states itself ([`Spending::read`]).
    pub fn read(bytes: &[u8]) -> Option<Check> {
        Some(Check {
            spending: Spending::read(bytes)?,
            result: None,
        })
    }

    /// The transaction being checked.
    pub fn tx(&self) -> &Transaction {
        self.spending.tx()
    }

    /// The transaction this check is still waiting for.
    pub fn waiting_for(&self) -> Option<Txid> {
        self.spending.waiting_for()
    }

    /// Takes a previous transaction. `false` where the bytes are no
    /// transaction, or none this check is waiting for.
    pub fn add_previous(&mut self, bytes: &[u8]) -> bool {
        self.spending.add_previous(bytes)
    }

    /// The answer, once one has been worked out.
    pub fn result(&self) -> Option<&Checked> {
        self.result.as_ref()
    }

    /// Records an answer.
    pub fn set_result(&mut self, result: Checked) {
        self.result = Some(result);
    }

    /// The previous outputs, in input order, once every one is known.
    pub fn prevouts(&self) -> Option<Vec<&ScriptBuf>> {
        self.spending.prevouts()
    }
}
