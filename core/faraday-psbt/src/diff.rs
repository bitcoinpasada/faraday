//! How two PSBTs, or finished transactions, differ (`docs/COMPARE.md`
//! §2.3 to §2.6).
//!
//! Inputs are matched by the outpoint they spend and outputs by script,
//! so a position is never a difference. The result is typed values, one
//! variant per line of the spec, and no text; the app words them. The
//! module reads two inspections the caller made and the PSBTs they were
//! made from, and does no work beyond walking them and one transaction id
//! each.

use alloc::collections::btree_map::Entry;
use alloc::collections::{BTreeMap, BTreeSet};
use alloc::string::String;
use alloc::vec::Vec;

use bitcoin::{Amount, OutPoint, ScriptBuf, Sequence, Txid};

use crate::inspect::{InputInfo, Inspection, Locktime, OutputInfo};
use crate::psbt::Psbt;

/// One side of a comparison.
#[derive(Debug, Clone, Copy)]
pub struct Side<'a> {
    /// The PSBT, or the finished transaction as one.
    pub psbt: &'a Psbt,
    /// Its inspection, made against the loaded keys and wallets.
    pub inspection: &'a Inspection,
    /// The PSBT's own version, 0 or 2; `None` for a finished transaction.
    pub psbt_version: Option<u8>,
}

/// Which side a line is about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Which {
    /// The first file (the original of a fee bump).
    First,
    /// The second file (the replacement of a fee bump).
    Second,
}

/// An output as a line names it: by its address, or by its script where
/// it has none.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputRef {
    /// The address, empty where the script has no address form.
    pub address: String,
    /// The script.
    pub script: ScriptBuf,
}

impl OutputRef {
    pub(crate) fn of(output: &OutputInfo) -> Self {
        Self {
            address: output.address.clone(),
            script: output.script_pubkey.clone(),
        }
    }
}

/// One difference, a variant per id of `docs/COMPARE.md` §2.5.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Difference {
    /// C1: the transaction's version differs.
    Version {
        /// In the first.
        first: i32,
        /// In the second.
        second: i32,
    },
    /// C2: the locktime differs.
    Locktime {
        /// In the first.
        first: Locktime,
        /// In the second.
        second: Locktime,
    },
    /// C3: both are PSBTs, of versions 0 and 2.
    PsbtVersion {
        /// In the first.
        first: u8,
        /// In the second.
        second: u8,
    },
    /// C4: the outpoint is spent on one side only.
    InputOnly {
        /// The outpoint.
        outpoint: OutPoint,
        /// The side that spends it.
        side: Which,
    },
    /// C5: a shared input's sequence differs.
    Sequence {
        /// The outpoint.
        outpoint: OutPoint,
        /// In the first.
        first: Sequence,
        /// In the second.
        second: Sequence,
    },
    /// C6: both state the amount a shared input spends, and they differ.
    Spends {
        /// The outpoint.
        outpoint: OutPoint,
        /// In the first.
        first: Amount,
        /// In the second.
        second: Amount,
    },
    /// C7: one side states the amount a shared input spends, the other
    /// none.
    AmountStatedOnly {
        /// The outpoint.
        outpoint: OutPoint,
        /// The side that states it.
        side: Which,
    },
    /// C8: a shared input's signature count differs.
    Signatures {
        /// The outpoint.
        outpoint: OutPoint,
        /// In the first.
        first: usize,
        /// In the second.
        second: usize,
    },
    /// C9: a shared input is final on one side only.
    FinalOnly {
        /// The outpoint.
        outpoint: OutPoint,
        /// The side where it is final.
        side: Which,
    },
    /// C10: an output left unpaired.
    OutputOnly {
        /// The output.
        output: OutputRef,
        /// Its amount.
        amount: Amount,
        /// The side that has it.
        side: Which,
    },
    /// C11: a pair of outputs whose amounts differ.
    OutputAmount {
        /// The output.
        output: OutputRef,
        /// In the first.
        first: Amount,
        /// In the second.
        second: Amount,
    },
    /// C12: one side spends one outpoint twice.
    SpentTwice {
        /// The outpoint.
        outpoint: OutPoint,
        /// The side that does.
        side: Which,
    },
    /// C13: a paired output is verified the person's own on one side
    /// only.
    YoursOnly {
        /// The output.
        output: OutputRef,
        /// The side where it is verified theirs.
        side: Which,
    },
}

/// An input as the two sides pair it: by outpoint, with the index it has
/// on each side.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InputPair {
    /// The outpoint both spend, or the one side does.
    pub outpoint: OutPoint,
    /// Its index in the first, where the first spends it.
    pub first: Option<usize>,
    /// Its index in the second, where the second spends it.
    pub second: Option<usize>,
}

/// An output as the two sides pair it (`docs/COMPARE.md` §2.4): the
/// indexes on each side, both for a pair.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OutputPair {
    /// Its index in the first.
    pub first: Option<usize>,
    /// Its index in the second.
    pub second: Option<usize>,
}

/// The facts of one side (`docs/COMPARE.md` §2.7).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Facts {
    /// The fee, where every input states its amount.
    pub fee: Option<Amount>,
    /// The fee rate over the estimated virtual size, where the fee is
    /// known.
    pub fee_rate_sat_vb: Option<f32>,
    /// The sum of the amounts the inputs state.
    pub total_in: Amount,
    /// The sum of the outputs.
    pub total_out: Amount,
}

/// How many inputs and outputs stand where.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// Inputs in both.
    pub inputs_both: usize,
    /// Inputs in the first only.
    pub inputs_first_only: usize,
    /// Inputs in the second only.
    pub inputs_second_only: usize,
    /// Outputs paired.
    pub outputs_both: usize,
    /// Outputs in the first only.
    pub outputs_first_only: usize,
    /// Outputs in the second only.
    pub outputs_second_only: usize,
}

/// What two sides differ in.
#[derive(Debug, Clone, PartialEq)]
pub struct Differences {
    /// The first's unsigned transaction id.
    pub first_txid: Txid,
    /// The second's unsigned transaction id.
    pub second_txid: Txid,
    /// The first's facts.
    pub first: Facts,
    /// The second's facts.
    pub second: Facts,
    /// The inputs by outpoint: the first's in its order, then the
    /// second's own in its order.
    pub inputs: Vec<InputPair>,
    /// The outputs: the first's in its order, then the second's own.
    pub outputs: Vec<OutputPair>,
    /// The differences, in the order `docs/COMPARE.md` §2.7 lists them:
    /// the transaction's, the inputs', the outputs'.
    pub list: Vec<Difference>,
}

impl Differences {
    /// Whether the two are one unsigned transaction.
    pub fn same_transaction(&self) -> bool {
        self.first_txid == self.second_txid
    }

    /// The counts of inputs and outputs in both and on one side.
    pub fn counts(&self) -> Counts {
        let mut n = Counts {
            inputs_both: 0,
            inputs_first_only: 0,
            inputs_second_only: 0,
            outputs_both: 0,
            outputs_first_only: 0,
            outputs_second_only: 0,
        };
        for p in &self.inputs {
            match (p.first, p.second) {
                (Some(_), Some(_)) => n.inputs_both += 1,
                (Some(_), None) => n.inputs_first_only += 1,
                _ => n.inputs_second_only += 1,
            }
        }
        for p in &self.outputs {
            match (p.first, p.second) {
                (Some(_), Some(_)) => n.outputs_both += 1,
                (Some(_), None) => n.outputs_first_only += 1,
                _ => n.outputs_second_only += 1,
            }
        }
        n
    }
}

fn outpoint(input: &InputInfo) -> OutPoint {
    OutPoint::new(input.txid, input.vout)
}

/// The inputs of a side by outpoint: the index of the first input spending
/// each, and the outpoints spent more than once.
fn by_outpoint(inputs: &[InputInfo]) -> (BTreeMap<OutPoint, usize>, BTreeSet<OutPoint>) {
    let mut first = BTreeMap::new();
    let mut twice = BTreeSet::new();
    for (i, input) in inputs.iter().enumerate() {
        match first.entry(outpoint(input)) {
            Entry::Vacant(e) => {
                e.insert(i);
            }
            Entry::Occupied(_) => {
                twice.insert(outpoint(input));
            }
        }
    }
    (first, twice)
}

/// Pairs the outputs by `docs/COMPARE.md` §2.4: by script; an output of
/// the first with the first unpaired one of the second with the same
/// amount; then what is left of each script in index order as a pair
/// whose amount changed; the rest on one side only.
pub(crate) fn pair_outputs(first: &[OutputInfo], second: &[OutputInfo]) -> Vec<OutputPair> {
    let mut of_script: BTreeMap<&[u8], Vec<usize>> = BTreeMap::new();
    for (j, o) in second.iter().enumerate() {
        of_script
            .entry(o.script_pubkey.as_bytes())
            .or_default()
            .push(j);
    }
    let mut paired: Vec<Option<usize>> = alloc::vec![None; first.len()];
    for (i, o) in first.iter().enumerate() {
        if let Some(list) = of_script.get_mut(o.script_pubkey.as_bytes())
            && let Some(at) = list.iter().position(|&j| second[j].amount == o.amount)
        {
            paired[i] = Some(list.remove(at));
        }
    }
    for (i, o) in first.iter().enumerate() {
        if paired[i].is_none()
            && let Some(list) = of_script.get_mut(o.script_pubkey.as_bytes())
            && !list.is_empty()
        {
            paired[i] = Some(list.remove(0));
        }
    }
    let mut taken = alloc::vec![false; second.len()];
    let mut pairs: Vec<OutputPair> = Vec::with_capacity(first.len());
    for (i, p) in paired.iter().enumerate() {
        if let Some(j) = p {
            taken[*j] = true;
        }
        pairs.push(OutputPair {
            first: Some(i),
            second: *p,
        });
    }
    for (j, t) in taken.iter().enumerate() {
        if !t {
            pairs.push(OutputPair {
                first: None,
                second: Some(j),
            });
        }
    }
    pairs
}

fn facts(side: &Side<'_>) -> Facts {
    let i = side.inspection;
    let known = i.inputs.iter().all(|x| x.amount.is_some());
    Facts {
        // `Inspection::fee` stays 0 where outputs exceed inputs.
        fee: (known && i.total_in >= i.total_out).then_some(i.fee),
        fee_rate_sat_vb: if known { i.fee_rate_sat_vb } else { None },
        total_in: i.total_in,
        total_out: i.total_out,
    }
}

/// The number of signatures an input carries, as
/// [`crate::verify::count_signatures`] counts them: each by its bytes
/// once, a hash preimage not at all.
fn signature_count(side: &Side<'_>, index: usize) -> usize {
    side.inspection
        .signatures
        .iter()
        .filter(|s| s.index == index)
        .flat_map(|s| s.signatures.iter())
        .filter(|p| !p.preimage)
        .map(|p| &p.bytes)
        .collect::<BTreeSet<_>>()
        .len()
}

/// C5 to C9 for an input both sides spend.
fn shared_input(
    out: &mut Vec<Difference>,
    op: OutPoint,
    (first, x): (&Side<'_>, &InputInfo),
    (second, y): (&Side<'_>, &InputInfo),
) {
    if x.sequence != y.sequence {
        out.push(Difference::Sequence {
            outpoint: op,
            first: x.sequence,
            second: y.sequence,
        });
    }
    match (x.amount, y.amount) {
        (Some(m), Some(n)) if m != n => out.push(Difference::Spends {
            outpoint: op,
            first: m,
            second: n,
        }),
        (Some(_), None) => out.push(Difference::AmountStatedOnly {
            outpoint: op,
            side: Which::First,
        }),
        (None, Some(_)) => out.push(Difference::AmountStatedOnly {
            outpoint: op,
            side: Which::Second,
        }),
        _ => {}
    }
    let (m, n) = (
        signature_count(first, x.index),
        signature_count(second, y.index),
    );
    if m != n {
        out.push(Difference::Signatures {
            outpoint: op,
            first: m,
            second: n,
        });
    }
    match (x.finalized, y.finalized) {
        (true, false) => out.push(Difference::FinalOnly {
            outpoint: op,
            side: Which::First,
        }),
        (false, true) => out.push(Difference::FinalOnly {
            outpoint: op,
            side: Which::Second,
        }),
        _ => {}
    }
}

/// C10, C11 and C13 for one pair of outputs.
fn output_pair(out: &mut Vec<Difference>, p: &OutputPair, (a, b): (&[OutputInfo], &[OutputInfo])) {
    match (p.first, p.second) {
        (Some(i), Some(j)) => {
            let (x, y) = (&a[i], &b[j]);
            if x.amount != y.amount {
                out.push(Difference::OutputAmount {
                    output: OutputRef::of(x),
                    first: x.amount,
                    second: y.amount,
                });
            }
            match (x.is_ours(), y.is_ours()) {
                (true, false) => out.push(Difference::YoursOnly {
                    output: OutputRef::of(x),
                    side: Which::First,
                }),
                (false, true) => out.push(Difference::YoursOnly {
                    output: OutputRef::of(y),
                    side: Which::Second,
                }),
                _ => {}
            }
        }
        (Some(i), None) => out.push(Difference::OutputOnly {
            output: OutputRef::of(&a[i]),
            amount: a[i].amount,
            side: Which::First,
        }),
        (None, Some(j)) => out.push(Difference::OutputOnly {
            output: OutputRef::of(&b[j]),
            amount: b[j].amount,
            side: Which::Second,
        }),
        (None, None) => {}
    }
}

/// How `first` and `second` differ.
pub fn list(first: Side<'_>, second: Side<'_>) -> Differences {
    let a = first.inspection;
    let b = second.inspection;
    let mut out = Vec::new();

    if a.version != b.version {
        out.push(Difference::Version {
            first: a.version,
            second: b.version,
        });
    }
    if a.locktime.raw != b.locktime.raw {
        out.push(Difference::Locktime {
            first: a.locktime,
            second: b.locktime,
        });
    }
    if let (Some(x), Some(y)) = (first.psbt_version, second.psbt_version)
        && x != y
    {
        out.push(Difference::PsbtVersion {
            first: x,
            second: y,
        });
    }

    let (in_a, twice_a) = by_outpoint(&a.inputs);
    let (in_b, twice_b) = by_outpoint(&b.inputs);
    let mut inputs = Vec::new();
    for (i, input) in a.inputs.iter().enumerate() {
        let op = outpoint(input);
        if in_a[&op] != i {
            continue;
        }
        inputs.push(InputPair {
            outpoint: op,
            first: Some(i),
            second: in_b.get(&op).copied(),
        });
    }
    for (j, input) in b.inputs.iter().enumerate() {
        let op = outpoint(input);
        if in_b[&op] == j && !in_a.contains_key(&op) {
            inputs.push(InputPair {
                outpoint: op,
                first: None,
                second: Some(j),
            });
        }
    }
    for p in &inputs {
        let op = p.outpoint;
        if twice_a.contains(&op) {
            out.push(Difference::SpentTwice {
                outpoint: op,
                side: Which::First,
            });
        }
        if twice_b.contains(&op) {
            out.push(Difference::SpentTwice {
                outpoint: op,
                side: Which::Second,
            });
        }
        let (x, y) = match (p.first, p.second) {
            (Some(i), Some(j)) => (&a.inputs[i], &b.inputs[j]),
            (Some(_), None) => {
                out.push(Difference::InputOnly {
                    outpoint: op,
                    side: Which::First,
                });
                continue;
            }
            (None, _) => {
                out.push(Difference::InputOnly {
                    outpoint: op,
                    side: Which::Second,
                });
                continue;
            }
        };
        shared_input(&mut out, op, (&first, x), (&second, y));
    }

    let outputs = pair_outputs(&a.outputs, &b.outputs);
    for p in &outputs {
        output_pair(&mut out, p, (&a.outputs, &b.outputs));
    }

    Differences {
        first_txid: first.psbt.unsigned_tx().compute_txid(),
        second_txid: second.psbt.unsigned_tx().compute_txid(),
        first: facts(&first),
        second: facts(&second),
        inputs,
        outputs,
        list: out,
    }
}
