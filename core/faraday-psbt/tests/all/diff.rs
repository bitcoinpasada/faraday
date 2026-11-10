//! How two PSBTs differ, by outpoint and by script.

use bitcoin::{Amount, Sequence, Witness, ecdsa};
use faraday_psbt::Psbt;
use faraday_psbt::diff::{self, Difference as D, Differences, OutputRef, Which};

use super::common::pair::*;

fn run(a: &Psbt, b: &Psbt) -> Differences {
    sides(a, true, b, true, diff::list)
}

fn out(address: &str, p: &Psbt, i: usize) -> OutputRef {
    OutputRef {
        address: address.into(),
        script: p.inner().unsigned_tx.output[i].script_pubkey.clone(),
    }
}

fn change_ref(p: &Psbt) -> OutputRef {
    let o = &look(p, true).outputs[1];
    OutputRef {
        address: o.address.clone(),
        script: o.script_pubkey.clone(),
    }
}

#[test]
fn the_same_transaction_has_no_differences() {
    let d = run(&original(), &original());
    assert!(d.list.is_empty());
    assert!(d.same_transaction());
    assert_eq!(d.first.fee, Some(Amount::from_sat(1_000)));
}

#[test]
fn transaction_level() {
    let o = original();
    let r = edit(&bumped(1_000), |x| {
        x.unsigned_tx.version = bitcoin::transaction::Version::ONE;
        x.unsigned_tx.lock_time = bitcoin::absolute::LockTime::from_consensus(5);
    });
    let d = run(&o, &r);
    assert!(matches!(
        d.list.as_slice(),
        [
            D::Version {
                first: 2,
                second: 1
            },
            D::Locktime { .. },
            D::OutputAmount { .. }
        ]
    ));
    // The PSBT version, where both are PSBTs of different versions.
    let (a, b) = (look(&o, true), look(&o, true));
    let side = |i, v| diff::Side {
        psbt: &o,
        inspection: i,
        psbt_version: v,
    };
    let d = diff::list(side(&a, Some(0)), side(&b, Some(2)));
    assert_eq!(
        d.list,
        vec![D::PsbtVersion {
            first: 0,
            second: 2
        }]
    );
    let d = diff::list(side(&a, Some(0)), side(&b, None));
    assert!(d.list.is_empty());
}

#[test]
fn inputs_by_outpoint() {
    let o = original();
    let extra = donor(60_000, super::common::ABANDON);
    let added = add_input(&bumped(1_000), &extra);
    let op = outpoint(&extra, 0);
    let d = run(&o, &added);
    assert!(d.list.contains(&D::InputOnly {
        outpoint: op,
        side: Which::Second
    }));
    let n = d.counts();
    assert_eq!(
        (n.inputs_both, n.inputs_first_only, n.inputs_second_only),
        (1, 0, 1)
    );
    // Reordered inputs are no difference.
    let swapped = edit(&added, |x| {
        x.unsigned_tx.input.swap(0, 1);
        x.inputs.swap(0, 1);
    });
    assert_eq!(run(&added, &swapped).list, vec![]);
    // Sequence, shared.
    let mut b = base();
    b.change = Some(48_000);
    b.sequence = 10;
    let d = run(&o, &build(&b));
    assert!(d.list.contains(&D::Sequence {
        outpoint: outpoint(&o, 0),
        first: Sequence(0xffff_fffd),
        second: Sequence(10)
    }));
}

#[test]
fn amounts_signatures_and_final() {
    let o = original();
    let op = outpoint(&o, 0);
    let state = |p: &Psbt, v: Option<u64>| {
        edit(p, |x| {
            x.inputs[0].non_witness_utxo = None;
            match v {
                Some(v) => x.inputs[0].witness_utxo.as_mut().unwrap().value = Amount::from_sat(v),
                None => x.inputs[0].witness_utxo = None,
            }
        })
    };
    let a = state(&o, Some(100_000));
    let d = run(&a, &state(&o, Some(100_500)));
    assert_eq!(
        d.list,
        vec![D::Spends {
            outpoint: op,
            first: Amount::from_sat(100_000),
            second: Amount::from_sat(100_500)
        }]
    );
    let d = run(&a, &state(&o, None));
    assert_eq!(
        d.list,
        vec![D::AmountStatedOnly {
            outpoint: op,
            side: Which::First
        }]
    );
    let d = run(&state(&o, None), &a);
    assert_eq!(
        d.list,
        vec![D::AmountStatedOnly {
            outpoint: op,
            side: Which::Second
        }]
    );
    assert_eq!(d.first.fee, None);

    let signed = edit(&o, |x| {
        let key = *x.inputs[0].bip32_derivation.keys().next().unwrap();
        let sig = ecdsa::Signature {
            signature: bitcoin::secp256k1::ecdsa::Signature::from_compact(&[1u8; 64]).unwrap(),
            sighash_type: bitcoin::EcdsaSighashType::All,
        };
        x.inputs[0]
            .partial_sigs
            .insert(bitcoin::PublicKey::new(key), sig);
    });
    assert_eq!(
        run(&o, &signed).list,
        vec![D::Signatures {
            outpoint: op,
            first: 0,
            second: 1
        }]
    );
    let finished = edit(&o, |x| {
        x.inputs[0].final_script_witness =
            Some(Witness::from_slice(&[vec![1u8; 71], vec![2u8; 33]]));
    });
    let d = run(&o, &finished);
    assert!(d.list.contains(&D::FinalOnly {
        outpoint: op,
        side: Which::Second
    }));
}

#[test]
fn one_outpoint_spent_twice() {
    let o = original();
    let twice = edit(&o, |x| {
        x.unsigned_tx.input.push(x.unsigned_tx.input[0].clone());
        x.inputs.push(x.inputs[0].clone());
    });
    let d = run(&o, &twice);
    assert!(d.list.contains(&D::SpentTwice {
        outpoint: outpoint(&o, 0),
        side: Which::Second
    }));
}

#[test]
fn outputs_by_script() {
    let o = original();
    // A changed amount is a pair; a position is no difference.
    let r = bumped(1_000);
    let d = run(&o, &r);
    assert_eq!(
        d.list,
        vec![D::OutputAmount {
            output: change_ref(&o),
            first: Amount::from_sat(49_000),
            second: Amount::from_sat(48_000)
        }]
    );
    let reordered = edit(&o, |x| {
        x.unsigned_tx.output.swap(0, 1);
        x.outputs.swap(0, 1);
    });
    assert_eq!(run(&o, &reordered).list, vec![]);
    // One output on one side only.
    let more = edit(&o, |x| {
        x.unsigned_tx.output.push(bitcoin::TxOut {
            value: Amount::from_sat(500),
            script_pubkey: other_payee(),
        });
        x.outputs.push(Default::default());
    });
    let d = run(&o, &more);
    assert!(matches!(
        d.list.as_slice(),
        [D::OutputOnly { amount, side: Which::Second, .. }] if *amount == Amount::from_sat(500)
    ));
    let _ = out;
}

#[test]
fn repeated_scripts_pair_by_amount_then_by_position() {
    let two = |a, b| {
        build(&{
            let mut x = base();
            x.recipients = vec![(payee(), a), (payee(), b)];
            x.change = None;
            x
        })
    };
    // Swapped between positions: no difference.
    assert_eq!(run(&two(10_000, 20_000), &two(20_000, 10_000)).list, vec![]);
    // Three against two: one left on one side only.
    let three = build(&{
        let mut x = base();
        x.recipients = vec![(payee(), 1_000), (payee(), 2_000), (payee(), 3_000)];
        x.change = None;
        x
    });
    let d = run(&three, &two(1_000, 2_000));
    assert_eq!(d.counts().outputs_first_only, 1);
    assert_eq!(d.counts().outputs_both, 2);
}

#[test]
fn yours_on_one_side_only() {
    let o = original();
    let d = sides(&o, true, &o, false, diff::list);
    assert!(matches!(
        d.list.as_slice(),
        [D::YoursOnly {
            side: Which::First,
            ..
        }]
    ));
}
