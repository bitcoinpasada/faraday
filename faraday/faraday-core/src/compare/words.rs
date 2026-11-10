//! The words of Compare and the fee bump: every difference's, line's,
//! result's and fact's text, made from the typed values of
//! `faraday_psbt::diff` and `faraday_psbt::bump` (`docs/COMPARE.md` §2.5,
//! §2.7, §3.4). Each match is exhaustive, with no `_` arm, so a variant
//! added to the crate fails to compile here until it has words.

use crate::decode::check_words::outpoint;
use crate::decode::{btc, locktime_text, sequence_text};
use crate::strings::{EN, fill};
use faraday_bip::bitcoin::{Amount, OutPoint, Sequence};
use faraday_psbt::Locktime;
use faraday_psbt::bump::{Bump, Fee, Line, Outcome};
use faraday_psbt::diff::{Difference, Differences, OutputRef, Which};

/// A fact: a label over one value or several.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Fact {
    /// The fact's label.
    pub label: &'static str,
    /// Its values, each with its own label where there is more than one.
    pub values: Vec<Value>,
}

/// One value of a fact.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Value {
    /// "First", "Second", "Original" or "Replacement", where the fact has
    /// more than one value.
    pub label: Option<&'static str>,
    /// The value.
    pub text: String,
}

fn value(label: Option<&'static str>, text: String) -> Value {
    Value { label, text }
}

fn amount(a: Amount) -> String {
    btc(a.to_sat())
}

fn seq(s: Sequence) -> String {
    sequence_text(s.0)
}

fn lock(l: Locktime) -> String {
    locktime_text(l.raw)
}

/// An output by its address in short form, or by its script's length
/// where it has no address.
fn output(o: &OutputRef) -> String {
    if o.address.is_empty() {
        fill(EN.compare_script_bytes, &[&o.script.len().to_string()])
    } else {
        faraday_ui::short(&o.address)
    }
}

/// A fee rate in whole and tenths of sat/vB.
fn rate(sat_vb: f64) -> String {
    format!("{sat_vb:.1}")
}

/// A fee and its rate, or "Not known".
fn fee_value(fee: Option<(String, f64)>) -> String {
    match fee {
        Some((sat, r)) => fill(EN.compare_fee_value, &[&sat, &rate(r)]),
        None => EN.compare_fee_not_known.to_string(),
    }
}

fn op(o: &OutPoint) -> String {
    outpoint(o)
}

/// One difference of Compare as a line.
pub(crate) fn difference(d: &Difference) -> String {
    match d {
        Difference::Version { first, second } => fill(
            EN.compare_version,
            &[&first.to_string(), &second.to_string()],
        ),
        Difference::Locktime { first, second } => {
            fill(EN.compare_locktime, &[&lock(*first), &lock(*second)])
        }
        Difference::PsbtVersion { first, second } => fill(
            EN.compare_psbt_version,
            &[&first.to_string(), &second.to_string()],
        ),
        Difference::InputOnly { outpoint, side } => fill(
            match side {
                Which::First => EN.compare_input_first_only,
                Which::Second => EN.compare_input_second_only,
            },
            &[&op(outpoint)],
        ),
        Difference::Sequence {
            outpoint,
            first,
            second,
        } => fill(
            EN.compare_sequence,
            &[&op(outpoint), &seq(*first), &seq(*second)],
        ),
        Difference::Spends {
            outpoint,
            first,
            second,
        } => fill(
            EN.compare_spends,
            &[&op(outpoint), &amount(*first), &amount(*second)],
        ),
        Difference::AmountStatedOnly { outpoint, side } => fill(
            match side {
                Which::First => EN.compare_amount_first_only,
                Which::Second => EN.compare_amount_second_only,
            },
            &[&op(outpoint)],
        ),
        Difference::Signatures {
            outpoint,
            first,
            second,
        } => fill(
            EN.compare_signatures,
            &[&op(outpoint), &first.to_string(), &second.to_string()],
        ),
        Difference::FinalOnly { outpoint, side } => fill(
            match side {
                Which::First => EN.compare_final_first_only,
                Which::Second => EN.compare_final_second_only,
            },
            &[&op(outpoint)],
        ),
        Difference::OutputOnly {
            output: o,
            amount: a,
            side,
        } => fill(
            match side {
                Which::First => EN.compare_output_first_only,
                Which::Second => EN.compare_output_second_only,
            },
            &[&output(o), &amount(*a)],
        ),
        Difference::OutputAmount {
            output: o,
            first,
            second,
        } => fill(
            EN.compare_output_amount,
            &[&output(o), &amount(*first), &amount(*second)],
        ),
        Difference::SpentTwice { outpoint, side } => fill(
            match side {
                Which::First => EN.compare_spent_twice_first,
                Which::Second => EN.compare_spent_twice_second,
            },
            &[&op(outpoint)],
        ),
        Difference::YoursOnly { output: o, side } => fill(
            match side {
                Which::First => EN.compare_yours_first_only,
                Which::Second => EN.compare_yours_second_only,
            },
            &[&output(o)],
        ),
    }
}

/// The lines of the Differences section: one per difference, or "No
/// differences".
pub(crate) fn differences(d: &Differences) -> Vec<String> {
    if d.list.is_empty() {
        vec![EN.compare_no_differences.to_string()]
    } else {
        d.list.iter().map(difference).collect()
    }
}

/// A fee bump's result line in Compare.
pub(crate) fn result(o: Outcome) -> &'static str {
    match o {
        Outcome::OnlyFee => EN.bump_only_fee,
        Outcome::MoreChanged => EN.bump_more_changed,
        Outcome::CannotTell => EN.bump_cannot_tell,
    }
}

/// A fee bump's result line on the review: `name` is the original's file
/// name.
pub(crate) fn result_from(o: Outcome, name: &str) -> String {
    fill(
        match o {
            Outcome::OnlyFee => EN.bump_only_fee_from,
            Outcome::MoreChanged => EN.bump_more_changed_from,
            Outcome::CannotTell => EN.bump_cannot_tell_from,
        },
        &[name],
    )
}

/// A whole number of sat per 1,000 vB as sat/vB.
fn per_vb(sat_per_kvb: i64) -> f64 {
    sat_per_kvb as f64 / 1000.0
}

/// One line of a fee bump's result.
pub(crate) fn line(l: &Line) -> String {
    match l {
        Line::InputDropped(o) => fill(EN.bump_input_dropped, &[&op(o)]),
        Line::InputAddedNotYours(o) => fill(EN.bump_input_added, &[&op(o)]),
        Line::Sequence {
            outpoint,
            original,
            replacement,
        } => fill(
            EN.bump_sequence,
            &[&op(outpoint), &seq(*original), &seq(*replacement)],
        ),
        Line::Spends {
            outpoint,
            original,
            replacement,
        } => fill(
            EN.bump_spends,
            &[&op(outpoint), &amount(*original), &amount(*replacement)],
        ),
        Line::SpentTwice { outpoint, side } => fill(
            match side {
                Which::First => EN.bump_spent_twice_original,
                Which::Second => EN.bump_spent_twice_replacement,
            },
            &[&op(outpoint)],
        ),
        Line::Version {
            original,
            replacement,
        } => fill(
            EN.bump_version,
            &[&original.to_string(), &replacement.to_string()],
        ),
        Line::Locktime {
            original,
            replacement,
        } => fill(EN.bump_locktime, &[&lock(*original), &lock(*replacement)]),
        Line::PaysOther {
            output: o,
            original,
            replacement,
        } => fill(
            EN.bump_pays_other,
            &[&amount(*replacement), &output(o), &amount(*original)],
        ),
        Line::NoLongerPays {
            output: o,
            original,
        } => fill(EN.bump_no_longer_pays, &[&amount(*original), &output(o)]),
        Line::PaysNew {
            output: o,
            replacement,
        } => fill(EN.bump_pays_new, &[&amount(*replacement), &output(o)]),
        Line::Fee {
            original,
            replacement,
        } => fill(
            EN.bump_fee,
            &[&original.to_string(), &replacement.to_string()],
        ),
        Line::FeeRate {
            original,
            replacement,
        } => fill(
            EN.bump_fee_rate,
            &[&rate(per_vb(*original)), &rate(per_vb(*replacement))],
        ),
        Line::AmountNotStated(o) => fill(EN.bump_amount_not_stated, &[&op(o)]),
        Line::ChangeNotLoaded { output: o, key } => {
            fill(EN.bump_change_not_loaded, &[&output(o), &key.to_string()])
        }
        Line::InputNotLoaded { outpoint, key } => {
            fill(EN.bump_input_not_loaded, &[&op(outpoint), &key.to_string()])
        }
        Line::NotStatedYours(o) => fill(EN.bump_not_stated_yours, &[&output(o)]),
    }
}

fn bump_fee(f: Option<Fee>) -> String {
    fee_value(f.map(|f| (f.sat.to_string(), per_vb(f.sat_per_kvb))))
}

/// The facts under a fee bump's lines: the fees, the inputs added where
/// the replacement adds any, and whether the original signals
/// replace-by-fee.
pub(crate) fn bump_facts(b: &Bump) -> Vec<Fact> {
    let mut facts = vec![Fact {
        label: EN.compare_fact_fee,
        values: vec![
            value(Some(EN.bump_fact_original), bump_fee(b.original_fee)),
            value(Some(EN.bump_fact_replacement), bump_fee(b.replacement_fee)),
        ],
    }];
    if b.added_inputs > 0 {
        facts.push(Fact {
            label: EN.bump_fact_inputs_added,
            values: vec![value(None, b.added_inputs.to_string())],
        });
    }
    facts.push(Fact {
        label: EN.bump_fact_rbf,
        values: vec![value(
            None,
            if b.signalled {
                EN.bump_signalled
            } else {
                EN.bump_not_signalled
            }
            .to_string(),
        )],
    });
    facts
}

fn counted(both: usize, first: usize, second: usize) -> Vec<Value> {
    let mut v = vec![value(
        None,
        fill(EN.compare_fact_in_both, &[&both.to_string()]),
    )];
    if first > 0 {
        v.push(value(
            None,
            fill(EN.compare_fact_first_only, &[&first.to_string()]),
        ));
    }
    if second > 0 {
        v.push(value(
            None,
            fill(EN.compare_fact_second_only, &[&second.to_string()]),
        ));
    }
    v
}

/// The facts of Compare's Result card: the transaction (one id where the
/// two are one unsigned transaction, else both), the fees, and the counts
/// of inputs and outputs.
pub(crate) fn compare_facts(d: &Differences) -> Vec<Fact> {
    let id = |t: faraday_bip::bitcoin::Txid| faraday_ui::grouped(&t.to_string());
    let transaction = if d.same_transaction() {
        vec![value(None, id(d.first_txid))]
    } else {
        vec![
            value(Some(EN.compare_fact_first), id(d.first_txid)),
            value(Some(EN.compare_fact_second), id(d.second_txid)),
        ]
    };
    let side = |f: &faraday_psbt::diff::Facts| {
        fee_value(match (f.fee, f.fee_rate_sat_vb) {
            (Some(fee), Some(r)) => Some((fee.to_sat().to_string(), f64::from(r))),
            _ => None,
        })
    };
    let n = d.counts();
    vec![
        Fact {
            label: EN.compare_fact_transaction,
            values: transaction,
        },
        Fact {
            label: EN.compare_fact_fee,
            values: vec![
                value(Some(EN.compare_fact_first), side(&d.first)),
                value(Some(EN.compare_fact_second), side(&d.second)),
            ],
        },
        Fact {
            label: EN.compare_fact_inputs,
            values: counted(n.inputs_both, n.inputs_first_only, n.inputs_second_only),
        },
        Fact {
            label: EN.compare_fact_outputs,
            values: counted(n.outputs_both, n.outputs_first_only, n.outputs_second_only),
        },
    ]
}
