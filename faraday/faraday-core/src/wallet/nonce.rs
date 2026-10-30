//! The nonce check's findings (`Session::nonce_check`), as Spend's Send
//! it card and the nonce check page draw them.

use crate::strings::{EN, fill};
use faraday_psbt::verify::Verdict;

/// What the nonce check found ([`Session::nonce_check`]).
pub struct NonceCheck {
    /// Every signature, in input order.
    pub rows: Vec<NonceRow>,
    /// Nonces used twice under one key: the key, given away.
    pub reused: Vec<faraday_psbt::verify::NonceReuse>,
    /// Inputs of a raw transaction that carry signature data but state
    /// no output they spend, so no signature on them could be read; 0
    /// for a PSBT, where an input with no output is a row of its own.
    pub unread: usize,
}

impl NonceCheck {
    /// Whether nothing is wrong and nothing was left out: every signature
    /// was checked and is valid, every one made here was recomputed, and
    /// no nonce repeats; a transaction with no signature is not sound
    /// either. A signature that could not be checked (no
    /// previous output, a script that names no key) is not sound: it is
    /// not known to be this transaction's. A lookup per row.
    pub fn sound(&self) -> bool {
        !self.rows.is_empty()
            && self.unread == 0
            && self.reused.is_empty()
            && self.rows.iter().all(|r| !r.wrong() && !r.unchecked())
    }

    /// Whether a signature is wrong or a nonce repeats: an invalid
    /// signature, one made here by no nonce rule, or a nonce used twice.
    /// A lookup per row.
    pub fn wrong(&self) -> bool {
        !self.reused.is_empty() || self.rows.iter().any(NonceRow::wrong)
    }

    /// How many signatures could not be checked. A lookup per row.
    pub fn unchecked(&self) -> usize {
        self.rows.iter().filter(|r| r.unchecked()).count()
    }

    /// The check's headline, as the spend screen shows it: how many
    /// signatures are valid and recomputed here when it is sound, that a
    /// signature is wrong when one is, and otherwise how many of them
    /// were not checked. A lookup per row.
    pub fn headline(&self) -> String {
        let total = self.rows.len();
        let noun = if total == 1 {
            EN.nonce_check_signature
        } else {
            EN.nonce_check_signatures
        };
        if self.wrong() {
            EN.nonce_check_nonce_check.to_string()
        } else if self.unread > 0 {
            fill(EN.nonce_unread_head, &[&self.unread.to_string()])
        } else if total == 0 {
            EN.nonce_check_no_signatures.to_string()
        } else if self.unchecked() > 0 {
            fill(
                EN.nonce_unchecked_head,
                &[&self.unchecked().to_string(), &total.to_string(), noun],
            )
        } else {
            let here = self.rows.iter().filter(|r| r.recomputed.is_some()).count();
            fill(
                EN.nonce_check_nonce_check_valid,
                &[&total.to_string(), noun, &here.to_string()],
            )
        }
    }
}

/// One signature, as the nonce check found it.
pub struct NonceRow {
    /// The input it signs.
    pub input: usize,
    /// Its key: the master fingerprint, or the key's first characters.
    pub name: String,
    /// Whether it is the key's over this transaction.
    pub verdict: Verdict,
    /// For a key held here: the nonce rule that recomputed it, or `None`
    /// inside when none did. `None` outside for a key held elsewhere,
    /// whose nonce cannot be recomputed without its private key.
    pub recomputed: Option<Option<faraday_psbt::verify::NonceMode>>,
    /// Why a signature not recomputed is not known to be another
    /// device's: its key has no origin here, or it is finalized under a
    /// key held here. `None` for every other.
    pub unmatched: Option<Unmatched>,
}

/// Why a signature's key could not be matched to a key held here, so its
/// nonce was not recomputed ([`NonceRow::unmatched`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unmatched {
    /// No key origin names its master: a final raw transaction, or a
    /// PSBT that left the origin out.
    NoOrigin,
    /// Its master is held here, but the signature is in a final
    /// `scriptSig` or witness, which no nonce rule is run on.
    Finalized,
}

impl NonceRow {
    /// Whether this signature is wrong: invalid, or made here by no
    /// nonce rule. A lookup.
    pub fn wrong(&self) -> bool {
        self.verdict.is_invalid() || matches!(self.recomputed, Some(None))
    }

    /// Whether this signature could not be checked. A lookup.
    pub fn unchecked(&self) -> bool {
        matches!(self.verdict, Verdict::Unchecked(_))
    }
}
