//! The nonce check's findings (`Session::nonce_check`), as Spend's Send
//! it card and the nonce check page draw them.

use crate::memory::nonces::{History, HistorySnapshot, NonceEntry, UsedBefore};
use crate::strings::{EN, fill};
use faraday_psbt::verify::Verdict;

/// What the nonce check found ([`Session::nonce_check`]).
#[derive(Clone)]
pub struct NonceCheck {
    /// Every signature, in input order.
    pub rows: Vec<NonceRow>,
    /// Nonces used twice under one key: the key, given away.
    pub reused: Vec<faraday_psbt::verify::NonceReuse>,
    /// Inputs whose finalized `scriptSig` or witness does not match the
    /// output it spends, with or without a signature row: no node relays
    /// them, but for a few shapes refused all the same.
    pub mismatched: Vec<usize>,
    /// Inputs whose spent output is not known to be the one the
    /// transaction spends: a final input that states no output (a raw
    /// transaction's, or a PSBT's), and any input, final or not, whose
    /// previous transaction is of another txid or lacks the output, or
    /// whose `witness_utxo` differs from its previous transaction. Nothing
    /// finished is made from them, and a signature on them is not read
    /// over a wrong amount.
    pub unread: usize,
    /// Of `unread`, the inputs, final or not, whose `witness_utxo` and
    /// previous transaction (of the right txid) state different outputs.
    pub disagree: usize,
    /// Final inputs that spend a script Faraday does not evaluate (not a
    /// miniscript, not a key, a key hash or a multisig): a signature on
    /// them is not known to be true. Not a refusal.
    pub not_evaluated: Vec<usize>,
    /// Of `mismatched`, the inputs whose shape matches the output they
    /// spend and that the script's interpreter refuses: a forged signature
    /// among them also has its Invalid row.
    pub fields_refused: Vec<usize>,
    /// The unsigned transaction's SHA-256: what its entries are kept under.
    pub tx: [u8; 32],
    /// MuSig2 partial signatures that verified, as entries by input and
    /// the participant's name: those of a spend's file
    /// ([`Session::nonce_partials`]), brought in or made here; the Nonce
    /// check page has none.
    pub partials: Vec<(usize, String, NonceEntry)>,
    /// Keys whose nonce an open vault holds over another message
    /// ([`NonceCheck::against`]).
    pub used_before: Vec<UsedBefore>,
    /// What the nonce history said of this check.
    pub history: History,
    /// The generation of the history the check was last compared with.
    pub history_gen: u64,
}

impl NonceCheck {
    /// Costs a map lookup per signature. Compares every signature that
    /// verified with the history `h`: a key whose nonce was used before
    /// over another message is named in `used_before`, and nothing is
    /// offered. With the history off or no vault open, nothing changes.
    pub fn against(&mut self, h: &HistorySnapshot) {
        self.used_before.clear();
        self.history = h.history();
        self.history_gen = h.generation();
        if self.history == History::Off {
            return;
        }
        let rows = self.rows.iter().map(|r| (r.input, &r.name, r.entry));
        let partials = self.partials.iter().map(|(i, n, e)| (*i, n, Some(*e)));
        let own = self.entries();
        let mut found: Vec<UsedBefore> = Vec::new();
        for (input, name, entry) in rows.chain(partials) {
            let Some(e) = entry else { continue };
            if !h.used_before(&e, &own) {
                continue;
            }
            match found.iter_mut().find(|u| u.key == e.key) {
                Some(u) => {
                    if !u.inputs.contains(&input) {
                        u.inputs.push(input);
                    }
                }
                None => found.push(UsedBefore {
                    key: e.key,
                    name: name.clone(),
                    inputs: vec![input],
                }),
            }
        }
        for u in &mut found {
            u.inputs.sort_unstable();
        }
        self.used_before = found;
    }

    /// Whether a nonce is used twice in the transaction or was used before
    /// over another message in an open vault: nothing is finished, saved or
    /// shown as a code, with no way past (F227). A lookup.
    pub fn held_back(&self) -> bool {
        !self.used_before.is_empty() || !self.reused.is_empty()
    }

    /// Every signature that verified, as an entry. A lookup per row.
    pub fn entries(&self) -> Vec<NonceEntry> {
        let rows = self.rows.iter().filter_map(|r| r.entry);
        rows.chain(self.partials.iter().map(|(_, _, e)| *e))
            .collect()
    }

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
            && self.used_before.is_empty()
            && self.mismatched.is_empty()
            && self.not_evaluated.is_empty()
            && self.rows.iter().all(|r| !r.wrong() && !r.unchecked())
    }

    /// Whether a signature is wrong or a nonce repeats: an invalid
    /// signature, one made here by no nonce rule, a nonce used twice, or
    /// a `scriptSig` or witness that does not match its output. A lookup
    /// per row.
    pub fn wrong(&self) -> bool {
        !self.reused.is_empty()
            || !self.used_before.is_empty()
            || !self.mismatched.is_empty()
            || self.rows.iter().any(NonceRow::wrong)
    }

    /// Whether a `scriptSig` or witness that does not match its output is
    /// what makes the check wrong, and no signature that stands is wrong
    /// and no nonce repeats: an input of a failing witness reads as a row
    /// of its own (its signatures read as they verify, or Invalid), and does not count
    /// against the rest. It answers false beside an input whose `witness_utxo` and
    /// previous transaction differ ([`NonceCheck::disagree_first`] speaks
    /// first). It also answers true beside an unread input or an unchecked
    /// row on another input. A lookup per row.
    pub fn only_witness_wrong(&self) -> bool {
        !self.mismatched.is_empty()
            && self.disagree == 0
            && self.reused.is_empty()
            && self.used_before.is_empty()
            && !self
                .rows
                .iter()
                .any(|r| r.wrong() && !self.mismatched.contains(&r.input))
    }

    /// Whether every input of `mismatched` is one the interpreter refuses
    /// with its shape matching (`fields_refused`). A lookup per input.
    pub(crate) fn only_fields_refused(&self) -> bool {
        self.mismatched
            .iter()
            .all(|i| self.fields_refused.contains(i))
    }

    /// Whether no signature is wrong and no nonce repeats, but an input
    /// states an output its previous transaction does not: that is what
    /// the check says first, as it is what is false of the transaction.
    /// An input of a failing witness reads as a row of its own, wrong
    /// by its witness, and does not stop this; a wrong signature on
    /// any other row does, a signature over the stated amount on the
    /// disagreeing input itself included. A lookup per row.
    pub(crate) fn disagree_first(&self) -> bool {
        self.disagree > 0
            && self.reused.is_empty()
            && self.used_before.is_empty()
            && !self
                .rows
                .iter()
                .any(|r| r.wrong() && !self.mismatched.contains(&r.input))
    }

    /// Whether nothing is offered to carry on: the check is wrong, or a
    /// PSBT has a final input whose output does not read (`has_finished`
    /// is false for it; a raw transaction keeps its offers, and a PSBT
    /// whose inputs all state a true output is offered as it is). A lookup per row.
    pub fn refused(&self, has_finished: bool) -> bool {
        self.wrong() || (self.unread > 0 && !has_finished)
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
        if self.only_witness_wrong() {
            if self.only_fields_refused() {
                EN.nonce_check_fields_refused.to_string()
            } else {
                EN.nonce_check_witness_wrong.to_string()
            }
        } else if self.disagree_first() {
            fill(EN.nonce_disagree_head, &[&self.disagree.to_string()])
        } else if self.wrong() {
            EN.nonce_check_nonce_check.to_string()
        } else if self.unread > 0 {
            fill(EN.nonce_unread_head, &[&self.unread.to_string()])
        } else if !self.not_evaluated.is_empty() {
            EN.nonce_check_script_not_run.to_string()
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
#[derive(Clone)]
pub struct NonceRow {
    /// The input it signs.
    pub input: usize,
    /// Its key: the master fingerprint, or the key's first characters.
    pub name: String,
    /// Whether it is the key's over this transaction.
    pub verdict: Verdict,
    /// The element is a proven hash preimage, not a signature.
    pub preimage: bool,
    /// For a key held here: the nonce rule that recomputed it, or `None`
    /// inside when none did. `None` outside for a key held elsewhere,
    /// whose nonce cannot be recomputed without its private key.
    pub recomputed: Option<Option<faraday_psbt::verify::NonceMode>>,
    /// Why a signature not recomputed is not known to be another
    /// device's: its key has no origin here, it is finalized under a
    /// key held here, or its script was not run. `None` for every other.
    pub unmatched: Option<Unmatched>,
    /// The signature as the nonce history keeps it, where it verified.
    pub entry: Option<NonceEntry>,
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
    /// The file states a key origin, but the script the signature is on
    /// was not run, so the key is not counted.
    NotRun,
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
