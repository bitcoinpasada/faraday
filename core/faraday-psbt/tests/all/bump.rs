//! The fee bump result: whether a replacement only raises the fee.

use bitcoin::{Amount, Sequence, Witness};
use faraday_psbt::Psbt;
use faraday_psbt::bump::{self, Bump, Line, Outcome};
use faraday_psbt::diff::Which;

use super::common::builder::Build;
use super::common::pair::*;

fn judge(o: &Psbt, r: &Psbt, loaded: bool) -> Option<Bump> {
    let key = [fp()];
    let key: &[_] = if loaded { &key } else { &[] };
    sides(o, loaded, r, loaded, |a, b| bump::judge(a, b, key))
}

fn ids(b: &Bump) -> Vec<&'static str> {
    b.lines.iter().map(Line::id).collect()
}

fn more(o: &Psbt, r: &Psbt) -> Vec<&'static str> {
    let b = judge(o, r, true).expect("applies");
    assert_eq!(b.result, Outcome::MoreChanged);
    ids(&b)
}

fn with(f: impl FnOnce(&mut Build)) -> Psbt {
    let mut b = base();
    b.change = Some(48_000);
    f(&mut b);
    build(&b)
}

#[test]
fn only_the_fee_is_raised() {
    let b = judge(&original(), &bumped(1_000), true).unwrap();
    assert_eq!(b.result, Outcome::OnlyFee);
    assert!(b.lines.is_empty());
    assert_eq!(b.original_fee.unwrap().sat, 1_000);
    assert_eq!(b.replacement_fee.unwrap().sat, 2_000);
    assert_eq!(b.added_inputs, 0);
    assert!(b.signalled);
}

#[test]
fn pay_less_and_pay_more() {
    let o = original();
    assert_eq!(more(&o, &with(|b| b.recipients[0].1 = 49_999)), ["B8"]);
    assert_eq!(more(&o, &with(|b| b.recipients[0].1 = 50_001)), ["B8"]);
    let b = judge(&o, &with(|b| b.recipients[0].1 = 49_999), true).unwrap();
    assert!(matches!(
        &b.lines[0],
        Line::PaysOther { original, replacement, .. }
            if *original == Amount::from_sat(50_000) && *replacement == Amount::from_sat(49_999)
    ));
}

#[test]
fn other_script_and_payment_added() {
    let o = original();
    let r = with(|b| b.recipients[0].0 = other_payee());
    assert_eq!(more(&o, &r), ["B9", "B10"]);
    let r = with(|b| {
        b.recipients.push((other_payee(), 1_000));
        b.change = Some(47_000);
    });
    assert_eq!(more(&o, &r), ["B10"]);
}

#[test]
fn inputs_added_dropped_foreign() {
    let o = original();
    let extra = donor(60_000, super::common::ABANDON);
    let mut b = base();
    b.change = Some(108_000);
    let added = add_input(&build(&b), &extra);
    let ok = judge(&o, &added, true).unwrap();
    assert_eq!(ok.result, Outcome::OnlyFee);
    assert_eq!(ok.added_inputs, 1);

    // The original's input left out.
    let alone = with(|b| b.change = Some(47_000));
    let b = judge(&added, &alone, true).unwrap();
    assert_eq!(b.lines, vec![Line::InputDropped(outpoint(&extra, 0))]);

    // An input under no key origin.
    let foreign = edit(&added, |p| p.inputs[1].bip32_derivation.clear());
    let b = judge(&o, &foreign, true).unwrap();
    assert_eq!(b.result, Outcome::MoreChanged);
    assert_eq!(b.lines, vec![Line::InputAddedNotYours(outpoint(&extra, 0))]);

    // An input of a key that is not loaded can only be told when it is.
    let zoo = donor(60_000, super::common::ZOO);
    let theirs = add_input(&build(&b_with(108_000)), &zoo);
    let b = judge(&o, &theirs, true).unwrap();
    assert_eq!(b.result, Outcome::CannotTell);
    assert!(matches!(b.lines.as_slice(), [Line::InputNotLoaded { .. }]));
}

fn b_with(change: u64) -> Build {
    let mut b = base();
    b.change = Some(change);
    b
}

#[test]
fn locktime_rule() {
    let o = original();
    // Height to height moves (anti-fee-sniping), as Bitcoin Core does.
    let mut a = base();
    a.locktime = 800_000;
    let a = build(&a);
    let r = with(|b| b.locktime = 800_001);
    assert_eq!(judge(&a, &r, true).unwrap().result, Outcome::OnlyFee);
    // None to a height too.
    assert_eq!(
        judge(&o, &with(|b| b.locktime = 1), true).unwrap().result,
        Outcome::OnlyFee
    );
    // A height to a time, and a time moved, are another transaction.
    assert_eq!(more(&a, &with(|b| b.locktime = 600_000_000)), ["B7"]);
    let t1 = with(|b| b.locktime = 600_000_000);
    let t2 = with(|b| {
        b.locktime = 600_000_001;
        b.change = Some(47_000);
    });
    assert_eq!(more(&t1, &t2), ["B7"]);
}

#[test]
fn version_differs() {
    let r = edit(&bumped(1_000), |p| {
        p.unsigned_tx.version = bitcoin::transaction::Version::ONE
    });
    assert_eq!(more(&original(), &r), ["B6"]);
}

#[test]
fn sequence_lock() {
    let o = original();
    assert_eq!(more(&o, &with(|b| b.sequence = 10)), ["B3"]);
    // Signalling off or on is no line.
    let r = with(|b| b.sequence = 0xffff_fffe);
    assert_eq!(judge(&o, &r, true).unwrap().result, Outcome::OnlyFee);
    let b = judge(&o, &with(|b| b.sequence = 10), true).unwrap();
    assert!(matches!(
        b.lines[0],
        Line::Sequence { original, replacement, .. }
            if original == Sequence(0xffff_fffd) && replacement == Sequence(10)
    ));
}

#[test]
fn fee_and_rate() {
    let o = original();
    // Same fee, another sequence so it is another transaction.
    let same = build(&{
        let mut b = base();
        b.sequence = 0xffff_fffe;
        b
    });
    assert_eq!(more(&o, &same), ["B11", "B12"]);
    // An input added for one sat more: the fee rises, the rate falls.
    let extra = donor(60_000, super::common::ABANDON);
    let r = add_input(&build(&b_with(108_999)), &extra);
    assert_eq!(more(&o, &r), ["B12"]);
    // The pair swapped.
    assert_eq!(more(&bumped(1_000), &o), ["B11", "B12"]);
}

#[test]
fn change_not_loaded() {
    let b = judge(&original(), &bumped(1_000), false).unwrap();
    assert_eq!(b.result, Outcome::CannotTell);
    assert_eq!(ids(&b), ["U2"]);
    assert!(matches!(&b.lines[0], Line::ChangeNotLoaded { key, .. } if *key == fp()));
}

#[test]
fn no_amount() {
    let strip = |p: &Psbt| {
        edit(p, |x| {
            for i in &mut x.inputs {
                i.witness_utxo = None;
                i.non_witness_utxo = None;
            }
        })
    };
    let b = judge(&strip(&original()), &strip(&bumped(1_000)), true).unwrap();
    assert_eq!(b.result, Outcome::CannotTell);
    assert_eq!(
        b.lines,
        vec![Line::AmountNotStated(outpoint(&original(), 0))]
    );
    assert_eq!(b.original_fee, None);
}

#[test]
fn finished_original_gets_its_fee_from_the_replacement() {
    let finished = |keep_change_origin: bool| {
        edit(&original(), |x| {
            for i in &mut x.inputs {
                i.witness_utxo = None;
                i.non_witness_utxo = None;
                i.bip32_derivation.clear();
                i.final_script_witness = Some(Witness::from_slice(&[vec![1u8; 71], vec![2u8; 33]]));
            }
            if !keep_change_origin {
                for o in &mut x.outputs {
                    o.bip32_derivation.clear();
                }
            }
        })
    };
    let mut orig = finished(false);
    let r = bumped(1_000);
    bump::fill_amounts(&mut orig, &r);
    assert_eq!(
        orig.inner().inputs[0].witness_utxo,
        r.inner().inputs[0].witness_utxo
    );
    // The replacement keeps the change on the same script: the fee alone.
    let b = judge(&orig, &r, true).unwrap();
    assert_eq!(b.result, Outcome::OnlyFee);
    assert_eq!(b.original_fee.unwrap().sat, 1_000);

    // The replacement drops the change: not stated to be the person's.
    let dropped = with(|b| b.change = None);
    let b = judge(&orig, &dropped, true).unwrap();
    assert_eq!(b.result, Outcome::CannotTell);
    assert_eq!(ids(&b), ["U4"]);
}

#[test]
fn payments_swapped_between_positions_and_reordered() {
    let two = |a, b, change| {
        build(&{
            let mut x = base();
            x.recipients = vec![(payee(), a), (payee(), b)];
            x.change = Some(change);
            x
        })
    };
    let o = two(10_000, 20_000, 69_000);
    let r = two(20_000, 10_000, 68_000);
    assert_eq!(judge(&o, &r, true).unwrap().result, Outcome::OnlyFee);

    let ordered = |first_is_payee| {
        build(&{
            let mut x = base();
            x.recipients = if first_is_payee {
                vec![(payee(), 10_000), (other_payee(), 20_000)]
            } else {
                vec![(other_payee(), 20_000), (payee(), 10_000)]
            };
            x.change = Some(69_000 - u64::from(!first_is_payee) * 1_000);
            x
        })
    };
    assert_eq!(
        judge(&ordered(true), &ordered(false), true).unwrap().result,
        Outcome::OnlyFee
    );
}

#[test]
fn shared_amount_stated_differently() {
    let state = |p: &Psbt, v: u64| {
        edit(p, |x| {
            x.inputs[0].non_witness_utxo = None;
            x.inputs[0].witness_utxo.as_mut().unwrap().value = Amount::from_sat(v);
        })
    };
    let o = state(&original(), 100_000);
    let r = state(&bumped(1_000), 100_500);
    assert_eq!(more(&o, &r), ["B4"]);
}

#[test]
fn outpoint_spent_twice() {
    let r = edit(&bumped(1_000), |x| {
        x.unsigned_tx.input.push(x.unsigned_tx.input[0].clone());
        x.inputs.push(x.inputs[0].clone());
    });
    let b = judge(&original(), &r, true).unwrap();
    assert_eq!(b.result, Outcome::MoreChanged);
    assert_eq!(
        b.lines[0],
        Line::SpentTwice {
            outpoint: outpoint(&r, 0),
            side: Which::Second
        }
    );
}

#[test]
fn spoofed_change_is_a_payment() {
    let r = edit(&bumped(1_000), |x| {
        x.unsigned_tx.output[1].script_pubkey = other_payee();
    });
    assert_eq!(more(&original(), &r), ["B10"]);
}

#[test]
fn no_result_for_one_transaction_or_unrelated_ones() {
    assert!(judge(&original(), &original(), true).is_none());
    let other = build(&{
        let mut b = base();
        b.input_value = 90_000;
        b.change = Some(39_000);
        b
    });
    assert!(judge(&original(), &other, true).is_none());
}

#[test]
fn bitcoin_cores_replacement_only_raises_the_fee() {
    let original = core_psbt("bump-core-original.psbt");
    let replacement = core_psbt("bump-core-replacement.psbt");
    // The change output carries its key origin, which is how it is known as
    // the key's own.
    assert!(!original.inner().outputs[1].bip32_derivation.is_empty());
    let b = judge(&original, &replacement, true).expect("applies");
    assert_eq!(b.result, Outcome::OnlyFee, "{:?}", b.lines);
    assert!(b.lines.is_empty());
    assert_eq!(b.added_inputs, 0);
    assert!(b.signalled);
    assert!(b.replacement_fee.unwrap().sat > b.original_fee.unwrap().sat);

    // The finished payment, read as Decode reads it, gets its amounts from
    // the replacement and gives the same result.
    let mut finished = core_finished();
    bump::fill_amounts(&mut finished, &replacement);
    let f = judge(&finished, &replacement, true).expect("applies");
    assert_eq!(f.result, Outcome::OnlyFee, "{:?}", f.lines);
    assert_eq!(f.original_fee, b.original_fee);
    assert_eq!(f.replacement_fee, b.replacement_fee);
}

#[test]
fn bitcoin_cores_replacement_with_an_added_input() {
    let original = core_psbt("bump-core-original.psbt");
    let added = core_psbt("bump-core-added.psbt");
    let b = judge(&original, &added, true).expect("applies");
    assert_eq!(b.result, Outcome::OnlyFee, "{:?}", b.lines);
    assert_eq!(b.added_inputs, 1);

    let mut finished = core_finished();
    bump::fill_amounts(&mut finished, &added);
    let f = judge(&finished, &added, true).expect("applies");
    assert_eq!(f.result, Outcome::OnlyFee, "{:?}", f.lines);
    assert_eq!(f.added_inputs, 1);
}

#[test]
fn bitcoin_cores_pair_cannot_be_told_without_the_key() {
    let original = core_psbt("bump-core-original.psbt");
    for name in ["bump-core-replacement.psbt", "bump-core-added.psbt"] {
        let b = judge(&original, &core_psbt(name), false).expect("applies");
        assert_eq!(b.result, Outcome::CannotTell, "{name}");
        assert!(ids(&b).contains(&"U2"), "{name}: {:?}", ids(&b));
    }
}
