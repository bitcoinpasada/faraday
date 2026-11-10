//! The nonce history (F227, `docs/FLOWS.md` "Nonce history"): the
//! signatures the nonce check found valid, as entries of key, nonce and
//! message, and what an open vault already holds of them. A signature
//! whose key used the same nonce over another message is refused.
//! `held` is filled by the vault's reader, `pending` by what is checked
//! here; neither is secret.

use std::collections::BTreeMap;
use std::sync::Arc;

use faraday_keys::vault::VaultId;
use faraday_psbt::musig::VerifiedPartial;
use faraday_psbt::verify::{SigKey, SignaturePresent, Verdict};

/// One signature that verified: the x coordinate of its key, its nonce
/// point's x (ECDSA's `r`, BIP-340's `R.x`) and the message it verified
/// against (its sighash; for a MuSig2 partial signature, SHA-256 of the
/// sighash and the aggregate nonce).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct NonceEntry {
    /// The x coordinate of the signing key.
    pub key: [u8; 32],
    /// The nonce point's x coordinate.
    pub r: [u8; 32],
    /// What it verified against.
    pub msg: [u8; 32],
}

impl NonceEntry {
    /// An entry of a vault's nonce record: key, nonce and message, 32
    /// bytes each.
    pub(crate) fn from_bytes(b: &[u8]) -> Option<NonceEntry> {
        let b: &[u8; 96] = b.try_into().ok()?;
        Some(NonceEntry {
            key: b[..32].try_into().ok()?,
            r: b[32..64].try_into().ok()?,
            msg: b[64..].try_into().ok()?,
        })
    }

    /// The title a vault gives the custom value of transaction `tx`'s nonce
    /// record: its key `faraday.nonces.<tx in hex>`, cut to the vault's 64
    /// characters.
    pub(crate) fn title_of(tx: &[u8; 32]) -> String {
        let hex: String = tx.iter().map(|b| format!("{b:02x}")).collect();
        format!("faraday.nonces.{hex}").chars().take(64).collect()
    }

    /// Whether `title`, an unreadable custom value's, is the one
    /// [`NonceEntry::title_of`] gives some transaction.
    pub(crate) fn is_record_title(title: &str) -> bool {
        title.len() == 64
            && title
                .strip_prefix("faraday.nonces.")
                .is_some_and(|h| h.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')))
    }

    /// The vault's nonce record of transaction `tx` holding `entries`.
    pub(crate) fn record_of(tx: &[u8; 32], entries: &[NonceEntry]) -> faraday_vault::Record {
        use faraday_vault::records::{field, kind};
        let mut r = faraday_vault::Record::new(kind::NONCES).with(field::NONCES_TX, tx);
        for e in entries {
            let mut b = [0u8; 96];
            b[..32].copy_from_slice(&e.key);
            b[32..64].copy_from_slice(&e.r);
            b[64..].copy_from_slice(&e.msg);
            r.push(field::NONCES_ENTRY, &b);
        }
        r
    }

    /// The entry of a signature that verified; `None` for any other row,
    /// or one whose key or nonce does not read.
    pub fn of_signature(sig: &SignaturePresent) -> Option<NonceEntry> {
        let (Verdict::Valid, Some(msg)) = (sig.verdict, sig.message) else {
            return None;
        };
        let key: [u8; 32] = match &sig.key {
            SigKey::Ecdsa(pk) => pk.inner.serialize()[1..33].try_into().ok()?,
            SigKey::Schnorr(x) => x.serialize(),
            SigKey::Unknown => return None,
        };
        let r: [u8; 32] = sig.nonce_point().try_into().ok()?;
        Some(NonceEntry { key, r, msg })
    }

    /// The entry of a MuSig2 partial signature that verified: the
    /// participant's x, the x of its first public nonce point, and the
    /// round.
    pub fn of_partial(p: &VerifiedPartial) -> NonceEntry {
        let mut key = [0u8; 32];
        key.copy_from_slice(&p.participant.serialize()[1..33]);
        NonceEntry {
            key,
            r: p.nonce_x,
            msg: p.round,
        }
    }
}

/// Every message a key's nonce was used over, by key and nonce.
type Index = BTreeMap<([u8; 32], [u8; 32]), Vec<[u8; 32]>>;

fn add(index: &mut Index, e: &NonceEntry) {
    let msgs = index.entry((e.key, e.r)).or_default();
    if !msgs.contains(&e.msg) {
        msgs.push(e.msg);
    }
}

/// The most entries one vault keeps (F227), and the most `pending` holds.
pub const CAP: usize = 10_000;

/// What the history says of a check ([`crate::wallet::NonceCheck::history`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum History {
    /// Not compared: no vault open, or the setting off.
    #[default]
    Off,
    /// Compared against `vaults` open vaults, nothing used before.
    Clean {
        /// How many open vaults.
        vaults: usize,
    },
    /// Compared, but `records` nonce records do not read.
    Unread {
        /// How many open vaults.
        vaults: usize,
        /// How many records do not read.
        records: usize,
    },
}

/// The nonce history: what each open vault held when it was read, and what
/// was checked here since.
#[derive(Debug)]
pub(crate) struct NonceHistory {
    /// What each open vault holds.
    held: Vec<(VaultId, Arc<Index>)>,
    /// Entries checked here and not yet in a vault, by transaction.
    pending: Vec<([u8; 32], Vec<NonceEntry>)>,
    /// The entries of `pending`, by key and nonce.
    pending_index: Arc<Index>,
    /// Records of a vault that do not read.
    unreadable: Vec<(VaultId, usize)>,
    /// Whether the history is kept at all (the setting).
    on: bool,
    /// Goes up when the history changes.
    generation: u64,
    /// How many distinct entries `held` and `pending` hold together,
    /// counted when they change, so drawing only reads it.
    remembered: usize,
}

impl Default for NonceHistory {
    fn default() -> Self {
        NonceHistory {
            held: Vec::new(),
            pending: Vec::new(),
            pending_index: Arc::default(),
            unreadable: Vec::new(),
            on: true,
            generation: 0,
            remembered: 0,
        }
    }
}

impl NonceHistory {
    /// Whether a vault is open and the setting is on: what is checked is
    /// compared and remembered.
    pub(crate) fn active(&self) -> bool {
        self.on && !self.held.is_empty()
    }

    /// Sets whether the history is kept.
    pub(crate) fn set_on(&mut self, on: bool) {
        if self.on != on {
            self.on = on;
            self.generation += 1;
        }
    }

    /// What open vault `id` holds, in place of what it was read to hold
    /// before, and how many of its records do not read.
    pub(crate) fn take_held(&mut self, id: VaultId, entries: &[NonceEntry], unreadable: usize) {
        let mut index = Index::new();
        for e in entries {
            add(&mut index, e);
        }
        self.held.retain(|(v, _)| *v != id);
        self.unreadable.retain(|(v, _)| *v != id);
        self.held.push((id, Arc::new(index)));
        if unreadable > 0 {
            self.unreadable.push((id, unreadable));
        }
        self.generation += 1;
        self.recount();
    }

    /// A vault closed: what it held leaves.
    pub(crate) fn drop_vault(&mut self, id: VaultId) {
        let before = self.held.len();
        self.held.retain(|(v, _)| *v != id);
        self.unreadable.retain(|(v, _)| *v != id);
        if self.held.len() != before {
            // Nothing is kept across a lock outside a vault.
            if self.held.is_empty() {
                self.pending.clear();
                self.pending_index = Arc::default();
            }
            self.generation += 1;
            self.recount();
        }
    }

    /// Adds the entries of transaction `tx` that `pending` lacks. Nothing
    /// is added unless [`active`](Self::active).
    pub(crate) fn note(&mut self, tx: [u8; 32], entries: &[NonceEntry]) {
        if !self.active() || entries.is_empty() {
            return;
        }
        let at = match self.pending.iter().position(|(t, _)| *t == tx) {
            Some(at) => at,
            None => {
                self.pending.push((tx, Vec::new()));
                self.pending.len() - 1
            }
        };
        let mut added = false;
        for e in entries {
            if !self.pending[at].1.contains(e) {
                self.pending[at].1.push(*e);
                add(Arc::make_mut(&mut self.pending_index), e);
                added = true;
            }
        }
        if added {
            self.trim(at);
            self.generation += 1;
            self.recount();
        }
    }

    /// Keeps `pending` to [`CAP`] entries, the oldest transaction dropped
    /// first; the transaction at `latest` is the last to go, and is cut
    /// to the cap when it alone passes it.
    fn trim(&mut self, mut latest: usize) {
        let total =
            |p: &[([u8; 32], Vec<NonceEntry>)]| p.iter().map(|(_, e)| e.len()).sum::<usize>();
        if total(&self.pending) <= CAP {
            return;
        }
        while total(&self.pending) > CAP && self.pending.len() > 1 {
            let first = if latest == 0 { 1 } else { 0 };
            self.pending.remove(first);
            if first < latest {
                latest -= 1;
            }
        }
        let over = total(&self.pending).saturating_sub(CAP);
        if over > 0 {
            let entries = &mut self.pending[0].1;
            entries.truncate(entries.len() - over);
        }
        let mut index = Index::new();
        for e in self.pending.iter().flat_map(|(_, es)| es) {
            add(&mut index, e);
        }
        self.pending_index = Arc::new(index);
    }

    /// Whether the history is kept at all (the setting).
    pub(crate) fn kept(&self) -> bool {
        self.on
    }

    /// How many signatures are remembered: what the open vaults held when
    /// they were read, and what waits for a vault, each signature once
    /// however many of them hold it.
    pub(crate) fn remembered(&self) -> usize {
        self.remembered
    }

    /// Costs a walk of every held map and of pending's. Counts the distinct
    /// entries again, when they change, never while drawing.
    fn recount(&mut self) {
        let mut seen = std::collections::BTreeSet::new();
        for index in self
            .held
            .iter()
            .map(|(_, h)| h)
            .chain([&self.pending_index])
        {
            for ((key, r), msgs) in index.iter() {
                for msg in msgs {
                    seen.insert((*key, *r, *msg));
                }
            }
        }
        self.remembered = seen.len();
    }

    /// The vaults whose records were read.
    pub(crate) fn held_ids(&self) -> Vec<VaultId> {
        self.held.iter().map(|(v, _)| *v).collect()
    }

    /// The transactions checked here and not yet in a vault.
    pub(crate) fn pending(&self) -> &[([u8; 32], Vec<NonceEntry>)] {
        &self.pending
    }

    /// What a check is compared against; cheap (the `Arc`s and counts).
    pub(crate) fn snapshot(&self) -> HistorySnapshot {
        HistorySnapshot {
            on: self.active(),
            held: self.held.iter().map(|(_, h)| h.clone()).collect(),
            pending: self.pending_index.clone(),
            unreadable: self.unreadable.iter().map(|(_, n)| n).sum(),
            generation: self.generation,
        }
    }
}

/// What a check is compared against ([`NonceHistory::snapshot`]).
#[derive(Debug, Clone, Default)]
pub struct HistorySnapshot {
    on: bool,
    held: Vec<Arc<Index>>,
    pending: Arc<Index>,
    unreadable: usize,
    generation: u64,
}

impl HistorySnapshot {
    /// The generation of the history it was taken at.
    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// Costs a map lookup per open vault. Whether `e` was used before over
    /// another message, one of `own` (the entries of the file `e` is
    /// from, whose own reuse is for the nonce check's other rule) not
    /// counting as before.
    pub(crate) fn used_before(&self, e: &NonceEntry, own: &[NonceEntry]) -> bool {
        let other = |i: &Index| {
            i.get(&(e.key, e.r)).is_some_and(|msgs| {
                msgs.iter().any(|m| {
                    *m != e.msg
                        && !own
                            .iter()
                            .any(|o| o.key == e.key && o.r == e.r && o.msg == *m)
                })
            })
        };
        self.held.iter().any(|h| other(h)) || other(&self.pending)
    }

    /// How the check reads when compared against this.
    pub(crate) fn history(&self) -> History {
        let vaults = self.held.len();
        if !self.on {
            History::Off
        } else if self.unreadable > 0 {
            History::Unread {
                vaults,
                records: self.unreadable,
            }
        } else {
            History::Clean { vaults }
        }
    }

    /// A snapshot of one open vault holding `entries`, as a test makes it.
    #[cfg(any(test, feature = "testkit"))]
    pub fn of_entries(entries: &[NonceEntry]) -> HistorySnapshot {
        let mut index = Index::new();
        for e in entries {
            add(&mut index, e);
        }
        HistorySnapshot {
            on: true,
            held: vec![Arc::new(index)],
            pending: Arc::default(),
            unreadable: 0,
            generation: 0,
        }
    }
}

/// One key's nonce used before ([`crate::wallet::NonceCheck::used_before`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UsedBefore {
    /// The x coordinate of the key: one line per key.
    pub key: [u8; 32],
    /// How the key is named: its fingerprint, or its first characters.
    pub name: String,
    /// The inputs that carry it.
    pub inputs: Vec<usize>,
}
