//! Whether a replacement only raises the fee of the transaction it
//! replaces (`docs/COMPARE.md` §3).
//!
//! Faraday's own review rules (F233): every rule that cannot be worked
//! out from what the two files state gives Cannot tell, never Only the
//! fee is raised. The module reads two inspections and the PSBTs they
//! were made from, pairs inputs and outputs as [`crate::diff`] does, and
//! returns typed lines, one variant per id of `docs/COMPARE.md` §3.4 and
//! no text.

use alloc::collections::{BTreeMap, BTreeSet};
use alloc::vec::Vec;

use bitcoin::{Amount, OutPoint, Sequence, TxOut};
use faraday_bip::keys::Fingerprint;

use crate::diff::{self, Difference, OutputRef, Side, Which};
use crate::inspect::{InputInfo, Inspection, Locktime, OutputInfo, OutputKind};
use crate::psbt::Psbt;

/// What the judgement came to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// Only the fee is raised.
    OnlyFee,
    /// More than the fee changed.
    MoreChanged,
    /// Cannot tell whether only the fee is raised.
    CannotTell,
}

/// A fee as the rules compare it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Fee {
    /// The fee in sat: the inputs' amounts less the outputs.
    pub sat: i64,
    /// Its rate in whole sat per 1,000 vB over the inspection's virtual
    /// size.
    pub sat_per_kvb: i64,
}

/// One line of the result, a variant per id of `docs/COMPARE.md` §3.4.
/// "Original" and "replacement" are the two sides, in that order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Line {
    /// B1: the original spends it and the replacement does not.
    InputDropped(OutPoint),
    /// B2: the replacement adds it, and it is not the person's.
    InputAddedNotYours(OutPoint),
    /// B3: a shared input's sequence differs where a relative lock is
    /// enabled.
    Sequence {
        /// The outpoint.
        outpoint: OutPoint,
        /// In the original.
        original: Sequence,
        /// In the replacement.
        replacement: Sequence,
    },
    /// B4: the two state different amounts for a shared input.
    Spends {
        /// The outpoint.
        outpoint: OutPoint,
        /// In the original.
        original: Amount,
        /// In the replacement.
        replacement: Amount,
    },
    /// B5: one side spends the outpoint twice.
    SpentTwice {
        /// The outpoint.
        outpoint: OutPoint,
        /// The side that does (first is the original).
        side: Which,
    },
    /// B6: the versions differ.
    Version {
        /// In the original.
        original: i32,
        /// In the replacement.
        replacement: i32,
    },
    /// B7: the locktimes differ.
    Locktime {
        /// In the original.
        original: Locktime,
        /// In the replacement.
        replacement: Locktime,
    },
    /// B8: a payment's amount differs.
    PaysOther {
        /// The payee.
        output: OutputRef,
        /// In the original.
        original: Amount,
        /// In the replacement.
        replacement: Amount,
    },
    /// B9: a payment of the original is not in the replacement.
    NoLongerPays {
        /// The payee.
        output: OutputRef,
        /// What the original paid.
        original: Amount,
    },
    /// B10: a payment the original does not make.
    PaysNew {
        /// The payee.
        output: OutputRef,
        /// What the replacement pays.
        replacement: Amount,
    },
    /// B11: the fee does not rise.
    Fee {
        /// The original's fee in sat.
        original: i64,
        /// The replacement's fee in sat.
        replacement: i64,
    },
    /// B12: the fee rate does not rise.
    FeeRate {
        /// The original's, in whole sat per 1,000 vB.
        original: i64,
        /// The replacement's, in whole sat per 1,000 vB.
        replacement: i64,
    },
    /// U1: no side states the amount of the input.
    AmountNotStated(OutPoint),
    /// U2: an output of a key that is not loaded.
    ChangeNotLoaded {
        /// The output.
        output: OutputRef,
        /// The key its origin names.
        key: Fingerprint,
    },
    /// U3: an input the replacement adds, of a key that is not loaded.
    InputNotLoaded {
        /// The outpoint.
        outpoint: OutPoint,
        /// The key its origin names.
        key: Fingerprint,
    },
    /// U4: an output of the original that is in the replacement nowhere,
    /// and not stated to be the person's.
    NotStatedYours(OutputRef),
}

impl Line {
    /// The id `docs/COMPARE.md` §3.4 gives the line.
    pub fn id(&self) -> &'static str {
        match self {
            Line::InputDropped(_) => "B1",
            Line::InputAddedNotYours(_) => "B2",
            Line::Sequence { .. } => "B3",
            Line::Spends { .. } => "B4",
            Line::SpentTwice { .. } => "B5",
            Line::Version { .. } => "B6",
            Line::Locktime { .. } => "B7",
            Line::PaysOther { .. } => "B8",
            Line::NoLongerPays { .. } => "B9",
            Line::PaysNew { .. } => "B10",
            Line::Fee { .. } => "B11",
            Line::FeeRate { .. } => "B12",
            Line::AmountNotStated(_) => "U1",
            Line::ChangeNotLoaded { .. } => "U2",
            Line::InputNotLoaded { .. } => "U3",
            Line::NotStatedYours(_) => "U4",
        }
    }

    /// Where the line stands: the lines of what more changed in the ids'
    /// order, then the lines of what cannot be told.
    fn rank(&self) -> u8 {
        match self {
            Line::InputDropped(_) => 1,
            Line::InputAddedNotYours(_) => 2,
            Line::Sequence { .. } => 3,
            Line::Spends { .. } => 4,
            Line::SpentTwice { .. } => 5,
            Line::Version { .. } => 6,
            Line::Locktime { .. } => 7,
            Line::PaysOther { .. } => 8,
            Line::NoLongerPays { .. } => 9,
            Line::PaysNew { .. } => 10,
            Line::Fee { .. } => 11,
            Line::FeeRate { .. } => 12,
            Line::AmountNotStated(_) => 13,
            Line::ChangeNotLoaded { .. } => 14,
            Line::InputNotLoaded { .. } => 15,
            Line::NotStatedYours(_) => 16,
        }
    }

    /// Whether the line says more changed, not that it cannot be told.
    fn is_change(&self) -> bool {
        self.rank() <= 12
    }
}

/// The result of judging a replacement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bump {
    /// What it came to.
    pub result: Outcome,
    /// The lines, in the order of the rules: what more changed, then what
    /// cannot be told. None under Only the fee is raised.
    pub lines: Vec<Line>,
    /// The original's fee, where known.
    pub original_fee: Option<Fee>,
    /// The replacement's fee, where known.
    pub replacement_fee: Option<Fee>,
    /// How many inputs the replacement adds.
    pub added_inputs: usize,
    /// Whether the original signals replace-by-fee.
    pub signalled: bool,
}

/// Whether a sequence enables a relative lock (BIP-68) in a transaction
/// of this version.
fn locks(version: i32, sequence: Sequence) -> bool {
    version as u32 >= 2 && sequence.0 & 0x8000_0000 == 0
}

enum Class {
    Yours,
    Payment,
    Unknown(Fingerprint),
}

/// What each output of `own` is to the person: theirs where the
/// inspection verifies it or the script is that of one verified theirs
/// on the other side, unknown where its origin names a key not loaded,
/// else a payment.
fn classes(own: &[OutputInfo], other: &[OutputInfo], loaded: &[Fingerprint]) -> Vec<Class> {
    let theirs: BTreeSet<&[u8]> = other
        .iter()
        .filter(|o| o.is_ours())
        .map(|o| o.script_pubkey.as_bytes())
        .collect();
    own.iter()
        .map(|o| {
            if o.is_ours() || theirs.contains(o.script_pubkey.as_bytes()) {
                return Class::Yours;
            }
            match o.kind {
                OutputKind::UnverifiedChange { key } if !loaded.contains(&key) => {
                    Class::Unknown(key)
                }
                _ => Class::Payment,
            }
        })
        .collect()
}

fn fee_of(side: &Side<'_>, amounts: &BTreeMap<OutPoint, Option<Amount>>) -> Option<Fee> {
    let i = side.inspection;
    // Summed in i128 from each output: `total_out` saturates.
    let mut total: i128 = 0;
    for input in &i.inputs {
        let a = amounts
            .get(&OutPoint::new(input.txid, input.vout))?
            .as_ref()?;
        total += i128::from(a.to_sat());
    }
    for out in &i.outputs {
        total -= i128::from(out.amount.to_sat());
    }
    let size = (i.vsize_estimate as i128).max(1);
    Some(Fee {
        sat: i64::try_from(total).ok()?,
        sat_per_kvb: i64::try_from(total * 1000 / size).ok()?,
    })
}

/// I3 and I4 for an input both spend.
fn shared_input_lines(
    lines: &mut Vec<Line>,
    op: OutPoint,
    (x, y): (&InputInfo, &InputInfo),
    (version_x, version_y): (i32, i32),
) {
    if x.sequence != y.sequence && (locks(version_x, x.sequence) || locks(version_y, y.sequence)) {
        lines.push(Line::Sequence {
            outpoint: op,
            original: x.sequence,
            replacement: y.sequence,
        });
    }
    if let (Some(m), Some(n)) = (x.amount, y.amount)
        && m != n
    {
        lines.push(Line::Spends {
            outpoint: op,
            original: m,
            replacement: n,
        });
    }
}

/// F2 and F3.
fn fee_lines(lines: &mut Vec<Line>, a: Fee, b: Fee) {
    if b.sat <= a.sat {
        lines.push(Line::Fee {
            original: a.sat,
            replacement: b.sat,
        });
    }
    if b.sat_per_kvb <= a.sat_per_kvb {
        lines.push(Line::FeeRate {
            original: a.sat_per_kvb,
            replacement: b.sat_per_kvb,
        });
    }
}

/// The lines of P1 and P2: the payments, and the outputs whose owner is
/// not known.
fn output_lines(
    lines: &mut Vec<Line>,
    d: &diff::Differences,
    (o, r): (&Inspection, &Inspection),
    loaded: &[Fingerprint],
) {
    let class_o = classes(&o.outputs, &r.outputs, loaded);
    let class_r = classes(&r.outputs, &o.outputs, loaded);
    let no_origin = o.outputs.iter().all(|x| x.kind == OutputKind::Recipient);
    let mut unloaded: BTreeSet<Vec<u8>> = BTreeSet::new();
    let mut unknown = |lines: &mut Vec<Line>, out: &OutputInfo, key: Fingerprint| {
        if unloaded.insert(out.script_pubkey.to_bytes()) {
            lines.push(Line::ChangeNotLoaded {
                output: OutputRef::of(out),
                key,
            });
        }
    };
    for p in &d.outputs {
        let x = p.first.map(|i| (&o.outputs[i], &class_o[i]));
        let y = p.second.map(|j| (&r.outputs[j], &class_r[j]));
        for side in [x, y].into_iter().flatten() {
            if let (out, Class::Unknown(key)) = side {
                unknown(lines, out, *key);
            }
        }
        match (x, y) {
            (Some((a, Class::Payment)), Some((b, Class::Payment))) if a.amount != b.amount => {
                lines.push(Line::PaysOther {
                    output: OutputRef::of(a),
                    original: a.amount,
                    replacement: b.amount,
                });
            }
            (Some((a, Class::Payment)), None) if no_origin => {
                lines.push(Line::NotStatedYours(OutputRef::of(a)));
            }
            (Some((a, Class::Payment)), None) => lines.push(Line::NoLongerPays {
                output: OutputRef::of(a),
                original: a.amount,
            }),
            (None, Some((b, Class::Payment))) => lines.push(Line::PaysNew {
                output: OutputRef::of(b),
                replacement: b.amount,
            }),
            _ => {}
        }
    }
}

/// Whether `replacement` only raises the fee of `original`, or `None`
/// where they are one transaction or spend no outpoint in common
/// (`docs/COMPARE.md` §3.1). `loaded` is the fingerprints of the loaded
/// keys.
pub fn judge(original: Side<'_>, replacement: Side<'_>, loaded: &[Fingerprint]) -> Option<Bump> {
    let d = diff::list(original, replacement);
    if d.same_transaction()
        || !d
            .inputs
            .iter()
            .any(|p| p.first.is_some() && p.second.is_some())
    {
        return None;
    }
    let (o, r) = (original.inspection, replacement.inspection);
    let mut lines: Vec<Line> = Vec::new();
    let mut added_inputs = 0;
    // What each outpoint spends, from whichever side states it (I4).
    let mut amounts: BTreeMap<OutPoint, Option<Amount>> = BTreeMap::new();
    let mut amounts_r: BTreeMap<OutPoint, Option<Amount>> = BTreeMap::new();

    for p in &d.inputs {
        let op = p.outpoint;
        let x = p.first.map(|i| &o.inputs[i]);
        let y = p.second.map(|j| &r.inputs[j]);
        let (ax, ay) = (x.and_then(|x| x.amount), y.and_then(|y| y.amount));
        amounts.insert(op, ax.or(ay));
        amounts_r.insert(op, ay.or(ax));
        match (x, y) {
            (Some(_), None) => lines.push(Line::InputDropped(op)),
            (None, Some(y)) => {
                added_inputs += 1;
                if !y.is_ours {
                    match &y.origin {
                        Some(origin) if !loaded.contains(&origin.fingerprint) => {
                            lines.push(Line::InputNotLoaded {
                                outpoint: op,
                                key: origin.fingerprint,
                            });
                        }
                        _ => lines.push(Line::InputAddedNotYours(op)),
                    }
                }
            }
            (Some(x), Some(y)) => {
                shared_input_lines(&mut lines, op, (x, y), (o.version, r.version))
            }
            (None, None) => {}
        }
        if amounts[&op].is_none() && amounts_r[&op].is_none() {
            lines.push(Line::AmountNotStated(op));
        }
    }
    for line in &d.list {
        if let Difference::SpentTwice { outpoint, side } = line {
            lines.push(Line::SpentTwice {
                outpoint: *outpoint,
                side: *side,
            });
        }
    }

    if o.version != r.version {
        lines.push(Line::Version {
            original: o.version,
            replacement: r.version,
        });
    }
    // A height may move (anti-fee-sniping); a time that differs, or a
    // height that becomes a time, is another transaction.
    const TIME: u32 = 500_000_000;
    if o.locktime.raw != r.locktime.raw && (o.locktime.raw >= TIME || r.locktime.raw >= TIME) {
        lines.push(Line::Locktime {
            original: o.locktime,
            replacement: r.locktime,
        });
    }

    output_lines(&mut lines, &d, (o, r), loaded);

    let original_fee = fee_of(&original, &amounts);
    let replacement_fee = fee_of(&replacement, &amounts_r);
    if let (Some(a), Some(b)) = (original_fee, replacement_fee) {
        fee_lines(&mut lines, a, b);
    } else if !lines.iter().any(|l| matches!(l, Line::AmountNotStated(_)))
        && let Some(p) = d.inputs.first()
    {
        // Every amount is stated but a fee is out of range: U1 is the
        // line for an amount that cannot be used.
        lines.push(Line::AmountNotStated(p.outpoint));
    }

    lines.sort_by_key(Line::rank);
    let result = if lines.iter().any(Line::is_change) {
        Outcome::MoreChanged
    } else if lines.is_empty() {
        Outcome::OnlyFee
    } else {
        Outcome::CannotTell
    };
    Some(Bump {
        result,
        lines,
        original_fee,
        replacement_fee,
        added_inputs,
        signalled: o.rbf,
    })
}

/// Sets the `witness_utxo` of each input of `original` that states no
/// amount from the input of `from` spending the same outpoint, where
/// `from` states one (`docs/COMPARE.md` §3.2, rule I4). A finished
/// transaction states no amounts: this is how it gets its fee. The caller
/// runs it before inspecting the original.
pub fn fill_amounts(original: &mut Psbt, from: &Psbt) {
    use crate::classify::spent_output;
    let have: BTreeMap<OutPoint, (Option<bitcoin::Transaction>, TxOut)> =
        (0..from.inner().inputs.len())
            .filter_map(|j| {
                let f = from.inner();
                let out = spent_output(f, j)?.clone();
                let tx = f.inputs[j].non_witness_utxo.clone();
                Some((f.unsigned_tx.input[j].previous_output, (tx, out)))
            })
            .collect();
    let psbt = original.inner_mut();
    for i in 0..psbt.inputs.len() {
        if crate::classify::spent_output(psbt, i).is_some() {
            continue;
        }
        let op = psbt.unsigned_tx.input[i].previous_output;
        if let Some((tx, out)) = have.get(&op) {
            let input = &mut psbt.inputs[i];
            match tx {
                Some(_) => input.non_witness_utxo = tx.clone(),
                None => {
                    input.non_witness_utxo = None;
                    input.witness_utxo = Some(out.clone());
                }
            }
        }
    }
}
