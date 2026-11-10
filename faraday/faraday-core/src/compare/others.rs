//! Other transactions spending what the open PSBT spends, on Sign a
//! transaction's review (`docs/COMPARE.md` §4): which files count, and
//! each one's fee bump result in words.

use super::words::{self, Fact};
use crate::Faraday;
use crate::decode::raw_as_psbt;
use crate::item::Item;
use crate::wallet::{self, FileKind};
use faraday_bip::bitcoin::{OutPoint, Txid};
use faraday_bip::keys::Fingerprint;
use faraday_crypto::work::Compute;
use faraday_psbt::bump::{self, Outcome};
use faraday_psbt::diff::Side;
use faraday_psbt::{Inspection, Psbt};

/// One other transaction's result on the review, in words.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Other {
    /// The original's file name.
    pub name: String,
    /// Where the file is in Files when the result was made: whether in
    /// the Outbox, and its place there (the press on **Compare** names
    /// it; a change of Files makes the results again).
    pub place: (bool, usize),
    /// The original's unsigned transaction id.
    pub txid: Txid,
    /// What the result came to.
    pub outcome: Outcome,
    /// The result line, "… from {name}".
    pub result: String,
    /// The lines, in the rules' order.
    pub lines: Vec<String>,
    /// The facts under them.
    pub facts: Vec<Fact>,
}

/// The files that count as another transaction spending the same inputs (§4.1): PSBTs and
/// finished transactions, From the stick and then For the stick, whose
/// unsigned transaction is not `txid` and that spend at least one of
/// `outpoints`, with the files of one transaction taken once (the first
/// PSBT among them, else the first finished transaction), each with its
/// unsigned transaction's id, in Files' order. Reads no file again: the
/// outpoints and the unsigned transaction's id are the ones kept when the file came in.
pub(crate) fn counting<'a>(
    files: impl IntoIterator<Item = &'a Item>,
    txid: Txid,
    outpoints: &[OutPoint],
) -> Vec<(&'a Item, Txid)> {
    let mut chosen: Vec<(usize, &'a Item, Txid)> = Vec::new();
    for (place, item) in files.into_iter().enumerate() {
        if !matches!(item.kind, FileKind::Psbt | FileKind::Transaction)
            || !shares(&item.outpoints, outpoints)
        {
            continue;
        }
        let Some(id) = item.unsigned_txid else {
            continue;
        };
        if id == txid {
            continue;
        }
        match chosen.iter_mut().find(|(_, _, t)| *t == id) {
            None => chosen.push((place, item, id)),
            Some(slot) => {
                // A PSBT stands for the transaction over a finished one.
                if slot.1.kind == FileKind::Transaction && item.kind == FileKind::Psbt {
                    *slot = (place, item, id);
                }
            }
        }
    }
    chosen.sort_by_key(|(place, _, _)| *place);
    chosen.into_iter().map(|(_, i, t)| (i, t)).collect()
}

/// Whether two sorted lists of outpoints have one in common.
pub(crate) fn shares(a: &[OutPoint], b: &[OutPoint]) -> bool {
    let (mut i, mut j) = (0, 0);
    while i < a.len() && j < b.len() {
        match a[i].cmp(&b[j]) {
            std::cmp::Ordering::Less => i += 1,
            std::cmp::Ordering::Greater => j += 1,
            std::cmp::Ordering::Equal => return true,
        }
    }
    false
}

/// Costs a PSBT read and an `inspect` of each
/// other file that counts, and, for a finished transaction, what
/// `amounts_in` costs (a transaction id of each finished transaction is
/// taken beside). The review's results in words, one per other
/// transaction in Files, each with the original taken from its file and
/// the PSBT under review (`open`, with its `inspection` and whether it
/// came as version 2) as the replacement. A file that does not read, or
/// to which §3.1 gives no result, has none. Each other file is inspected
/// once.
pub(crate) fn make(
    c: &Compute,
    app: &Faraday,
    open: &Psbt,
    open_v2: bool,
    inspection: &Inspection,
) -> Vec<Other> {
    let txid = open.unsigned_tx().compute_txid();
    let mut spent: Vec<OutPoint> = open
        .unsigned_tx()
        .input
        .iter()
        .map(|i| i.previous_output)
        .collect();
    spent.sort();
    let loaded: Vec<Fingerprint> = app.session.keys.all().map(|k| k.fingerprint).collect();
    let files = app.inbox().iter().chain(app.outbox.iter());
    let replacement = Side {
        psbt: open,
        inspection,
        psbt_version: Some(if open_v2 { 2 } else { 0 }),
    };
    let mut out = Vec::new();
    for (item, id) in counting(files, txid, &spent) {
        let Some((mut psbt, version)) = original_of(c, app, item) else {
            continue;
        };
        if version.is_none() {
            bump::fill_amounts(&mut psbt, open);
        }
        let seen = app.session.inspect(c, &psbt);
        let original = Side {
            psbt: &psbt,
            inspection: &seen,
            psbt_version: version,
        };
        let Some(b) = bump::judge(original, replacement, &loaded) else {
            continue;
        };
        let Some(place) = place_of(app, item) else {
            continue;
        };
        out.push(Other {
            name: item.name.clone(),
            place,
            txid: id,
            outcome: b.result,
            result: words::result_from(b.result, &item.name),
            lines: b.lines.iter().map(words::line).collect(),
            facts: words::bump_facts(&b),
        });
    }
    out
}

/// Where `item` is in Files: whether in the Outbox, and its place in the
/// Inbox or the Outbox.
pub(crate) fn place_of(app: &Faraday, item: &Item) -> Option<(bool, usize)> {
    let same = |i: &Item| std::ptr::eq(i, item);
    app.inbox()
        .iter()
        .position(same)
        .map(|k| (false, k))
        .or_else(|| app.outbox.iter().position(same).map(|k| (true, k)))
}

/// Costs a PSBT read; for a finished transaction also what `amounts_in`
/// costs. The file as a PSBT, with its version (`None` for a finished
/// transaction).
fn original_of(c: &Compute, app: &Faraday, item: &Item) -> Option<(Psbt, Option<u8>)> {
    match item.kind {
        FileKind::Psbt => {
            let (psbt, v2) = wallet::read_psbt_kept(c, &item.bytes)?;
            Some((psbt, Some(if v2.is_some() { 2 } else { 0 })))
        }
        FileKind::Transaction => {
            let tx = faraday_psbt::transaction::read_raw(&item.bytes)?;
            let (prevouts, _) = app.amounts_in(c, &tx);
            let (psbt, _) = raw_as_psbt(c, &item.bytes, &prevouts)?;
            Some((psbt, None))
        }
        _ => None,
    }
}
