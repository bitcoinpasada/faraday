//! Silent payments in the app (`docs/PLANNING.md` §16.113): the form an
//! address is shown in, the DNS record's two questions, and the check
//! that says whether a transaction pays this wallet.
//!
//! Nothing here is a secret. The scan private key a check needs is
//! derived from the loaded key where the check runs and is dropped with
//! the [`osk_bip::silent::Receiver`] that held it, so this file holds
//! only a transaction, its previous outputs and the answer.

use alloc::string::String;
use alloc::vec::Vec;

use osk_bip::bitcoin::{Amount, ScriptBuf, Transaction, Txid};
use osk_psbt::transaction::Spending;

/// The forms a silent payment address is shown in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddressForm {
    /// The `sp1q…` string itself.
    Address,
    /// BIP-321's `bitcoin:?sp=…`, which a wallet that reads URIs takes.
    Uri,
}

impl AddressForm {
    /// Both forms, in the order the Choice lists them.
    pub const ALL: [AddressForm; 2] = [AddressForm::Address, AddressForm::Uri];
}

/// Where the DNS record's questions are.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DnsStep {
    /// The user name before the domain.
    User,
    /// The domain the record is published under.
    Domain,
}

/// What the person typed for the record, and which question is on
/// screen. Both are public: a name and a domain are what the record
/// publishes.
pub struct Dns {
    /// Which question.
    pub step: DnsStep,
    /// The user name.
    pub user: String,
    /// The domain.
    pub domain: String,
}

impl Default for Dns {
    fn default() -> Self {
        Dns {
            step: DnsStep::User,
            user: String::new(),
            domain: String::new(),
        }
    }
}

impl Dns {
    /// Whether both answers are given, which is what the record needs.
    pub fn ready(&self) -> bool {
        !self.user.trim().is_empty() && !self.domain.trim().is_empty()
    }

    /// The field the step on screen is typing into.
    pub fn field(&self) -> &str {
        match self.step {
            DnsStep::User => &self.user,
            DnsStep::Domain => &self.domain,
        }
    }

    /// Types one character of it. A name and a domain are ASCII here,
    /// because a record's owner name is.
    pub fn push(&mut self, c: char) {
        if !c.is_ascii_graphic() {
            return;
        }
        let field = match self.step {
            DnsStep::User => &mut self.user,
            DnsStep::Domain => &mut self.domain,
        };
        if field.len() < 64 {
            field.push(c);
        }
    }

    /// Deletes the last character of it.
    pub fn pop(&mut self) {
        match self.step {
            DnsStep::User => self.user.pop(),
            DnsStep::Domain => self.domain.pop(),
        };
    }
}

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
