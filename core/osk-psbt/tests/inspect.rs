//! Change detection and the policy warnings, each built from a hostile or
//! unusual PSBT (`docs/PLANNING.md` §12, "hostile coordinator").

mod common;

use bitcoin::psbt::PsbtSighashType;
use bitcoin::secp256k1::{Message, Secp256k1, XOnlyPublicKey};
use bitcoin::sighash::{EcdsaSighashType, Prevouts, SighashCache, TapSighashType};
use bitcoin::{Amount, ScriptBuf};
use common::builder::{Build, single_sig, sorted_multisig};
use common::{ABANDON, LEGAL, ZOO, master};
use osk_bip::keys::{Network, ScriptType};
use osk_psbt::{
    ABSURD_FEE_RATE_SAT_VB, Aux, Context, Error, HIGH_FEE_CAUTION_PCT, HIGH_FEE_DANGER_ABS,
    Inspection, KeyRef, Level, LocktimeKind, Nonce, OutputKind, Psbt, Warning, WarningKind,
    inspect, sign,
};

const NET: Network = Network::Regtest;

fn foreign_script() -> ScriptBuf {
    master(ZOO, NET)
        .account_xpub(ScriptType::NativeSegwit, 0)
        .unwrap()
        .address(false, 0)
        .unwrap()
        .script_pubkey()
}

fn find<'a>(insp: &'a Inspection, kind: &WarningKind) -> Option<&'a Warning> {
    insp.warnings.iter().find(|w| &w.kind == kind)
}

fn has(insp: &Inspection, level: Level, kind: &WarningKind) -> bool {
    insp.warnings
        .iter()
        .any(|w| w.level == level && &w.kind == kind)
}

fn inspect_with_abandon(psbt: &Psbt) -> Inspection {
    let abandon = master(ABANDON, NET);
    let keys = [KeyRef::from_master(&abandon, 0).unwrap()];
    inspect(
        psbt,
        &Context {
            network: NET,
            keys: &keys,
            wallets: &[],
            musig_session: None,
            shares: &[],
            carry: None,
        },
    )
}

fn basic(script_type: ScriptType) -> Psbt {
    let mut b = Build::new(script_type, NET);
    b.recipients.push((foreign_script(), 60_000));
    b.change = Some(39_000);
    single_sig(&master(ABANDON, NET), &b)
}

/// [`basic`] with a second input of the same script type and the same
/// key, spending a different funding output, and the recipient paid the
/// extra so the fee stays ordinary.
fn two_inputs(script_type: ScriptType) -> Psbt {
    let mut psbt = basic(script_type);
    let mut b = Build::new(script_type, NET);
    b.input_value = 50_000;
    b.recipients.push((foreign_script(), 49_000));
    let other = single_sig(&master(ABANDON, NET), &b);
    let inner = psbt.inner_mut();
    inner
        .unsigned_tx
        .input
        .push(other.unsigned_tx().input[0].clone());
    inner.inputs.push(other.inner().inputs[0].clone());
    inner.unsigned_tx.output[0].value += Amount::from_sat(49_000);
    psbt
}

#[test]
fn verified_change_for_every_script_type() {
    let abandon = master(ABANDON, NET);
    for script_type in ScriptType::ALL {
        let insp = inspect_with_abandon(&basic(script_type));
        assert_eq!(
            insp.outputs[1].kind,
            OutputKind::Change {
                key: abandon.fingerprint(),
                change: true,
                index: 0
            },
            "{script_type}"
        );
        assert!(insp.outputs[1].is_ours());
        assert!(find(&insp, &WarningKind::UnverifiedChange).is_none());
        assert!(find(&insp, &WarningKind::ChangeSpoof).is_none());
        assert!(!insp.has_danger(), "{script_type}: {:?}", insp.warnings);
        assert!(
            insp.outputs[1].address.starts_with("bcrt1")
                || insp.outputs[1].address.starts_with('2')
                || insp.outputs[1].address.starts_with('m')
                || insp.outputs[1].address.starts_with('n')
        );
    }
}

#[test]
fn spoofed_change_blocks_signing() {
    for script_type in ScriptType::ALL {
        let mut psbt = basic(script_type);
        // Keep the origin claim, redirect the script to someone else.
        psbt.inner_mut().unsigned_tx.output[1].script_pubkey = foreign_script();
        let insp = inspect_with_abandon(&psbt);
        let abandon_fp = master(ABANDON, NET).fingerprint();
        assert_eq!(
            insp.outputs[1].kind,
            OutputKind::UnverifiedChange { key: abandon_fp }
        );
        assert!(
            has(&insp, Level::Blocked, &WarningKind::ChangeSpoof),
            "{script_type}"
        );
        assert_eq!(insp.amount_to_others, Amount::from_sat(99_000));
        assert_eq!(insp.change_total, Amount::ZERO);
        // Signing is refused, and no override changes that.
        let abandon = master(ABANDON, NET);
        let keys = [KeyRef::from_master(&abandon, 0).unwrap()];
        let ctx = Context {
            network: NET,
            keys: &keys,
            wallets: &[],
            musig_session: None,
            shares: &[],
            carry: None,
        };
        for force in [false, true] {
            let err = sign(
                &mut psbt,
                &[&abandon],
                &[abandon.fingerprint()],
                &ctx,
                force,
                Nonce::LowR,
                Aux::Deterministic,
                &mut None,
                [0u8; 32],
                &[],
                &[],
            )
            .unwrap_err();
            assert!(
                matches!(&err, Error::Blocked(kinds) if kinds.contains(&WarningKind::ChangeSpoof)),
                "{script_type} forced {force}: {err}"
            );
            assert!(psbt.inner().inputs[0].partial_sigs.is_empty());
            assert!(psbt.inner().inputs[0].tap_key_sig.is_none());
        }
    }
}

#[test]
fn foreign_fingerprint_is_unverified_change() {
    let zoo_fp = master(ZOO, NET).fingerprint();
    for script_type in ScriptType::ALL {
        let mut psbt = basic(script_type);
        let out = &mut psbt.inner_mut().outputs[1];
        for (fp, _) in out.bip32_derivation.values_mut() {
            *fp = zoo_fp.into();
        }
        for (_, (fp, _)) in out.tap_key_origins.values_mut() {
            *fp = zoo_fp.into();
        }
        let insp = inspect_with_abandon(&psbt);
        assert_eq!(
            insp.outputs[1].kind,
            OutputKind::UnverifiedChange { key: zoo_fp }
        );
        assert!(
            has(&insp, Level::Caution, &WarningKind::UnverifiedChange),
            "{script_type}"
        );
        assert!(!has(&insp, Level::Danger, &WarningKind::ChangeSpoof));
        assert!(!insp.has_danger(), "{script_type}: {:?}", insp.warnings);
    }
}

#[test]
fn change_under_an_unloaded_account_is_unverified() {
    let abandon = master(ABANDON, NET);
    let mut psbt = basic(ScriptType::NativeSegwit);
    // Only account 1 loaded: the change path is under account 0.
    let keys = [KeyRef::from_master(&abandon, 1).unwrap()];
    let insp = inspect(
        &psbt,
        &Context {
            network: NET,
            keys: &keys,
            wallets: &[],
            musig_session: None,
            shares: &[],
            carry: None,
        },
    );
    assert!(has(&insp, Level::Caution, &WarningKind::UnverifiedChange));
    assert!(has(&insp, Level::Caution, &WarningKind::UnknownDerivation));
    assert_eq!(insp.participating_keys, vec![abandon.fingerprint()]);
    // The signer still checks the key for real.
    let r = sign(
        &mut psbt,
        &[&abandon],
        &[abandon.fingerprint()],
        &insp_ctx(&keys),
        false,
        Nonce::LowR,
        Aux::Deterministic,
        &mut None,
        [0u8; 32],
        &[],
        &[],
    )
    .unwrap();
    assert_eq!(r.signed_inputs.len(), 1);
}

fn insp_ctx(keys: &[KeyRef]) -> Context<'_> {
    Context {
        network: NET,
        keys,
        wallets: &[],
        musig_session: None,
        shares: &[],
        carry: None,
    }
}

#[test]
fn self_transfer() {
    let mut b = Build::new(ScriptType::NativeSegwit, NET);
    b.change = Some(99_000);
    let psbt = single_sig(&master(ABANDON, NET), &b);
    let insp = inspect_with_abandon(&psbt);
    assert!(insp.is_self_transfer);
    assert_eq!(insp.amount_to_others, Amount::ZERO);
    assert_eq!(insp.change_total, Amount::from_sat(99_000));
    assert_eq!(insp.fee, Amount::from_sat(1_000));
    assert!(!insp.has_danger());
}

#[test]
fn high_fee_thresholds() {
    let case = |recipient: u64, change: Option<u64>| {
        let mut b = Build::new(ScriptType::NativeSegwit, NET);
        b.recipients.push((foreign_script(), recipient));
        b.change = change;
        inspect_with_abandon(&single_sig(&master(ABANDON, NET), &b))
    };
    // 1 % fee: nothing.
    assert!(
        find(
            &case(99_000, None),
            &WarningKind::HighFee { pct_of_amount: 1 }
        )
        .is_none()
    );
    // Exactly the caution threshold.
    let insp = case(100_000 * 100 / (100 + HIGH_FEE_CAUTION_PCT), None);
    let w = insp
        .warnings
        .iter()
        .find(|w| matches!(w.kind, WarningKind::HighFee { .. }))
        .unwrap();
    assert_eq!(w.level, Level::Caution);
    assert!(matches!(w.kind, WarningKind::HighFee { pct_of_amount } if pct_of_amount >= 5));
    // Danger by share: 80 000 to others, 20 000 fee = 25 %.
    let insp = case(80_000, None);
    assert!(matches!(
        find(&insp, &WarningKind::HighFee { pct_of_amount: 25 }),
        Some(Warning {
            level: Level::Danger,
            ..
        })
    ));
    // Danger by absolute amount: 2 BTC in, 1.98 BTC out, fee 0.02 BTC = 1 %.
    let mut b = Build::new(ScriptType::NativeSegwit, NET);
    b.input_value = 200_000_000;
    b.recipients.push((foreign_script(), 198_000_000));
    let insp = inspect_with_abandon(&single_sig(&master(ABANDON, NET), &b));
    assert!(insp.fee > HIGH_FEE_DANGER_ABS);
    assert!(matches!(
        find(&insp, &WarningKind::HighFee { pct_of_amount: 1 }),
        Some(Warning {
            level: Level::Danger,
            ..
        })
    ));
    // The share is measured against what leaves the wallet, not change.
    let insp = case(10_000, Some(80_000));
    assert!(matches!(
        find(&insp, &WarningKind::HighFee { pct_of_amount: 100 }),
        Some(Warning {
            level: Level::Danger,
            ..
        })
    ));
}

#[test]
fn absurd_fee_rate() {
    let mut b = Build::new(ScriptType::NativeSegwit, NET);
    b.input_value = 1_000_000;
    b.recipients.push((foreign_script(), 800_000));
    let insp = inspect_with_abandon(&single_sig(&master(ABANDON, NET), &b));
    assert!(insp.fee_rate_sat_vb.unwrap() >= ABSURD_FEE_RATE_SAT_VB as f32);
    assert!(has(&insp, Level::Danger, &WarningKind::AbsurdFeeRate));
}

#[test]
fn dust_output() {
    let mut b = Build::new(ScriptType::NativeSegwit, NET);
    b.recipients.push((foreign_script(), 200));
    b.change = Some(99_000);
    let insp = inspect_with_abandon(&single_sig(&master(ABANDON, NET), &b));
    assert!(insp.outputs[0].is_dust);
    assert!(!insp.outputs[1].is_dust);
    assert!(has(&insp, Level::Caution, &WarningKind::DustOutput));
}

#[test]
fn unusual_sighash() {
    let set = |t: EcdsaSighashType| {
        let mut psbt = basic(ScriptType::NativeSegwit);
        psbt.inner_mut().inputs[0].sighash_type = Some(PsbtSighashType::from(t));
        inspect_with_abandon(&psbt)
    };
    assert!(find(&set(EcdsaSighashType::All), &WarningKind::UnusualSighash).is_none());
    assert!(has(
        &set(EcdsaSighashType::None),
        Level::Danger,
        &WarningKind::UnusualSighash
    ));
    assert!(has(
        &set(EcdsaSighashType::NonePlusAnyoneCanPay),
        Level::Danger,
        &WarningKind::UnusualSighash
    ));
    assert!(has(
        &set(EcdsaSighashType::Single),
        Level::Caution,
        &WarningKind::UnusualSighash
    ));
    assert!(has(
        &set(EcdsaSighashType::AllPlusAnyoneCanPay),
        Level::Caution,
        &WarningKind::UnusualSighash
    ));
    let mut psbt = basic(ScriptType::NativeSegwit);
    psbt.inner_mut().inputs[0].sighash_type = Some(PsbtSighashType::from_u32(0x04));
    assert!(has(
        &inspect_with_abandon(&psbt),
        Level::Danger,
        &WarningKind::UnusualSighash
    ));
}

#[test]
fn network_mismatch() {
    // Keys and PSBT for mainnet, checked in a testnet context: the paths
    // say coin type 0 and the loaded accounts are mainnet.
    let mainnet = master(ABANDON, Network::Mainnet);
    let mut b = Build::new(ScriptType::NativeSegwit, Network::Mainnet);
    b.recipients.push((foreign_script(), 60_000));
    b.change = Some(39_000);
    let psbt = single_sig(&mainnet, &b);
    let keys = [KeyRef::from_master(&mainnet, 0).unwrap()];
    let insp = inspect(
        &psbt,
        &Context {
            network: Network::Testnet,
            keys: &keys,
            wallets: &[],
            musig_session: None,
            shares: &[],
            carry: None,
        },
    );
    assert!(has(&insp, Level::Danger, &WarningKind::NetworkMismatch));
    // The same PSBT is clean in its own network.
    let insp = inspect(
        &psbt,
        &Context {
            network: Network::Mainnet,
            keys: &keys,
            wallets: &[],
            musig_session: None,
            shares: &[],
            carry: None,
        },
    );
    assert!(find(&insp, &WarningKind::NetworkMismatch).is_none());
    assert!(insp.outputs[1].address.starts_with("bc1q"));
    // A testnet key with a PSBT whose paths use coin type 0.
    let testnet = master(ABANDON, Network::Testnet);
    let keys = [KeyRef::from_master(&testnet, 0).unwrap()];
    let insp = inspect(
        &psbt,
        &Context {
            network: Network::Testnet,
            keys: &keys,
            wallets: &[],
            musig_session: None,
            shares: &[],
            carry: None,
        },
    );
    assert!(has(&insp, Level::Danger, &WarningKind::NetworkMismatch));
}

#[test]
fn address_reuse() {
    let mut b = Build::new(ScriptType::NativeSegwit, NET);
    b.recipients.push((foreign_script(), 30_000));
    b.recipients.push((foreign_script(), 30_000));
    let insp = inspect_with_abandon(&single_sig(&master(ABANDON, NET), &b));
    let w = find(&insp, &WarningKind::AddressReuse).unwrap();
    assert_eq!(w.level, Level::Caution);
    assert!(w.text.contains("outputs 0 and 1"));

    // Paying back to the input's own address.
    let abandon = master(ABANDON, NET);
    let receive = abandon
        .account_xpub(ScriptType::NativeSegwit, 0)
        .unwrap()
        .address(false, 0)
        .unwrap()
        .script_pubkey();
    let mut b = Build::new(ScriptType::NativeSegwit, NET);
    b.recipients.push((receive, 99_000));
    let insp = inspect_with_abandon(&single_sig(&abandon, &b));
    let w = find(&insp, &WarningKind::AddressReuse).unwrap();
    assert!(w.text.contains("input 0"));
    // Without origin data the output is a recipient, not change.
    assert_eq!(insp.outputs[0].kind, OutputKind::Recipient);
}

#[test]
fn no_participating_key() {
    let zoo = master(ZOO, NET);
    let keys = [KeyRef::from_master(&zoo, 0).unwrap()];
    let ctx = Context {
        network: NET,
        keys: &keys,
        wallets: &[],
        musig_session: None,
        shares: &[],
        carry: None,
    };
    let mut psbt = basic(ScriptType::NativeSegwit);
    let insp = inspect(&psbt, &ctx);
    assert!(insp.participating_keys.is_empty());
    assert!(!insp.inputs[0].is_ours);
    assert!(has(&insp, Level::Danger, &WarningKind::NoParticipatingKey));
    assert_eq!(
        sign(
            &mut psbt,
            &[&zoo],
            &[zoo.fingerprint()],
            &ctx,
            false,
            Nonce::LowR,
            Aux::Deterministic,
            &mut None,
            [0u8; 32],
            &[],
            &[],
        ),
        Err(Error::Danger(vec![WarningKind::NoParticipatingKey]))
    );
    let r = sign(
        &mut psbt,
        &[&zoo],
        &[zoo.fingerprint()],
        &ctx,
        true,
        Nonce::LowR,
        Aux::Deterministic,
        &mut None,
        [0u8; 32],
        &[],
        &[],
    )
    .unwrap();
    assert!(r.signed_inputs.is_empty() && !r.complete);
    // No keys loaded at all.
    let insp = inspect(
        &psbt,
        &Context {
            network: NET,
            keys: &[],
            wallets: &[],
            musig_session: None,
            shares: &[],
            carry: None,
        },
    );
    assert!(has(&insp, Level::Danger, &WarningKind::NoParticipatingKey));
}

#[test]
fn locktime_rbf_and_non_standard_output() {
    let mut b = Build::new(ScriptType::NativeSegwit, NET);
    b.recipients.push((foreign_script(), 60_000));
    b.recipients.push((ScriptBuf::new_op_return(b"hi"), 0));
    b.change = Some(39_000);
    b.locktime = 800_000;
    b.sequence = 0xffff_fffe;
    let insp = inspect_with_abandon(&single_sig(&master(ABANDON, NET), &b));
    assert_eq!(insp.locktime.kind, LocktimeKind::Height(800_000));
    assert_eq!(insp.locktime.to_string(), "block 800000");
    assert!(!insp.rbf);
    assert!(has(&insp, Level::Info, &WarningKind::LocktimeInFuture));
    assert!(has(&insp, Level::Info, &WarningKind::NonStandardScript));
    assert!(
        insp.outputs[1]
            .address
            .starts_with("non-standard script 6a")
    );
    assert!(!insp.outputs[1].is_dust);
    assert!(!insp.has_danger(), "{:?}", insp.warnings);
    assert_eq!(insp.version, 2);

    let mut b = Build::new(ScriptType::NativeSegwit, NET);
    b.recipients.push((foreign_script(), 99_000));
    b.locktime = 1_700_000_000;
    let insp = inspect_with_abandon(&single_sig(&master(ABANDON, NET), &b));
    assert_eq!(insp.locktime.kind, LocktimeKind::Time(1_700_000_000));
    assert!(insp.rbf);
}

#[test]
fn missing_and_lying_utxo() {
    let mut psbt = basic(ScriptType::NativeSegwit);
    psbt.inner_mut().inputs[0].witness_utxo = None;
    psbt.inner_mut().inputs[0].non_witness_utxo = None;
    let insp = inspect_with_abandon(&psbt);
    assert!(has(&insp, Level::Danger, &WarningKind::MissingUtxo));
    assert_eq!(insp.inputs[0].amount, None);
    assert_eq!(insp.fee_rate_sat_vb, None);

    // A non_witness_utxo that is not the spent transaction.
    let mut psbt = basic(ScriptType::Legacy);
    let mut fake = psbt.inner().inputs[0].non_witness_utxo.clone().unwrap();
    fake.output[0].value = Amount::from_sat(5_000_000);
    psbt.inner_mut().inputs[0].non_witness_utxo = Some(fake);
    let insp = inspect_with_abandon(&psbt);
    assert!(has(&insp, Level::Blocked, &WarningKind::UtxoMismatch));
    let abandon = master(ABANDON, NET);
    let keys = [KeyRef::from_master(&abandon, 0).unwrap()];
    let ctx = Context {
        network: NET,
        keys: &keys,
        wallets: &[],
        musig_session: None,
        shares: &[],
        carry: None,
    };
    assert!(matches!(
        sign(
            &mut psbt,
            &[&abandon],
            &[abandon.fingerprint()],
            &ctx,
            true,
            Nonce::LowR,
            Aux::Deterministic,
            &mut None,
            [0u8; 32],
            &[],
            &[],
        ),
        Err(Error::Blocked(_))
    ));
}

/// A SegWit v0 input states its amount in `witness_utxo`, and BIP-143
/// signs that amount rather than the transaction it came from. Without
/// the transaction beside it the amount is the coordinator's word, and
/// with a second input to combine with, two rounds over two different
/// lies pay the difference to the miner, so the input blocks signing
/// whatever the caller forces. The lie can be on either input, so one
/// input carrying its transaction does not unblock the other.
#[test]
fn segwit_v0_inputs_without_their_previous_transaction_block_a_transaction_of_two() {
    let abandon = master(ABANDON, NET);
    let keys = [KeyRef::from_master(&abandon, 0).unwrap()];
    let ctx = insp_ctx(&keys);
    for script_type in [ScriptType::NestedSegwit, ScriptType::NativeSegwit] {
        for missing in [&[0, 1][..], &[0][..], &[1][..]] {
            let mut psbt = two_inputs(script_type);
            assert!(
                !inspect(&psbt, &ctx).has_blocked(),
                "{script_type}: both transactions present"
            );
            for i in missing {
                psbt.inner_mut().inputs[*i].non_witness_utxo = None;
            }
            let insp = inspect(&psbt, &ctx);
            assert!(
                has(&insp, Level::Blocked, &WarningKind::AmountUnverified),
                "{script_type} without {missing:?}: {:?}",
                insp.warnings
            );
            for force in [false, true] {
                let err = sign(
                    &mut psbt,
                    &[&abandon],
                    &[abandon.fingerprint()],
                    &ctx,
                    force,
                    Nonce::LowR,
                    Aux::Deterministic,
                    &mut None,
                    [0u8; 32],
                    &[],
                    &[],
                )
                .unwrap_err();
                assert!(
                    matches!(&err, Error::Blocked(kinds)
                        if kinds.iter().all(|k| *k == WarningKind::AmountUnverified)),
                    "{script_type} without {missing:?} forced {force}: {err:?}"
                );
                assert!(
                    psbt.inner()
                        .inputs
                        .iter()
                        .all(|i| i.partial_sigs.is_empty())
                );
            }
        }
    }
}

/// One input alone cannot be lied to: BIP-143 commits the signature to
/// that input's stated amount, so a wrong amount gives a signature no
/// node accepts and there is no second signature to combine it with. A
/// single-input SegWit v0 spend with `witness_utxo` alone is therefore
/// inspected and signed, the way taproot already is, and the signature
/// verifies against an independently computed sighash. Legacy has no
/// witness_utxo to fall back on and reports no amount at all.
#[test]
fn a_single_input_transaction_is_signed_without_its_previous_transaction() {
    let secp = Secp256k1::verification_only();
    let abandon = master(ABANDON, NET);
    let keys = [KeyRef::from_master(&abandon, 0).unwrap()];
    let ctx = insp_ctx(&keys);
    for script_type in ScriptType::ALL {
        let mut psbt = basic(script_type);
        psbt.inner_mut().inputs[0].non_witness_utxo = None;
        let insp = inspect(&psbt, &ctx);
        assert!(!has(&insp, Level::Blocked, &WarningKind::AmountUnverified));
        if script_type == ScriptType::Legacy {
            assert!(has(&insp, Level::Danger, &WarningKind::MissingUtxo));
            continue;
        }
        assert!(!insp.has_blocked(), "{script_type}: {:?}", insp.warnings);
        assert_eq!(insp.inputs[0].amount, Some(Amount::from_sat(100_000)));
        let unsigned = psbt.unsigned_tx().clone();
        let r = sign(
            &mut psbt,
            &[&abandon],
            &[abandon.fingerprint()],
            &ctx,
            false,
            Nonce::LowR,
            Aux::Deterministic,
            &mut None,
            [0u8; 32],
            &[],
            &[],
        )
        .unwrap();
        assert_eq!(r.signed_inputs.len(), 1, "{script_type}");
        let utxo = psbt.inner().inputs[0].witness_utxo.clone().unwrap();
        let mut cache = SighashCache::new(&unsigned);
        let bytes = &r.signed_inputs[0].sig_bytes;
        if script_type == ScriptType::Taproot {
            let msg = Message::from(
                cache
                    .taproot_key_spend_signature_hash(
                        0,
                        &Prevouts::All(core::slice::from_ref(&utxo)),
                        TapSighashType::Default,
                    )
                    .unwrap(),
            );
            let sig = bitcoin::secp256k1::schnorr::Signature::from_slice(bytes).unwrap();
            let output_key =
                XOnlyPublicKey::from_slice(&utxo.script_pubkey.as_bytes()[2..]).unwrap();
            secp.verify_schnorr(&sig, &msg, &output_key).unwrap();
        } else {
            let (pk, _) = psbt.inner().inputs[0]
                .bip32_derivation
                .iter()
                .next()
                .unwrap();
            let script_code = match script_type {
                ScriptType::NestedSegwit => psbt.inner().inputs[0].redeem_script.clone().unwrap(),
                _ => utxo.script_pubkey.clone(),
            };
            let msg = Message::from(
                cache
                    .p2wpkh_signature_hash(0, &script_code, utxo.value, EcdsaSighashType::All)
                    .unwrap(),
            );
            let sig = bitcoin::ecdsa::Signature::from_slice(bytes).unwrap();
            secp.verify_ecdsa(&msg, &sig.signature, pk).unwrap();
        }
    }
}

/// The same rule for multisig: one of two p2wsh inputs without the
/// transaction it spends blocks the transaction, and carrying it is what
/// the builder does.
#[test]
fn a_p2wsh_input_without_its_previous_transaction_is_blocked() {
    let abandon = master(ABANDON, NET);
    let zoo = master(ZOO, NET);
    let legal = master(LEGAL, NET);
    let keys = [KeyRef::from_master(&abandon, 0).unwrap()];
    let ctx = insp_ctx(&keys);
    let cosigners = [&abandon, &zoo, &legal];
    let mut psbt = sorted_multisig(&cosigners, 2, NET, 100_000, &[(foreign_script(), 148_000)]);
    let other = sorted_multisig(&cosigners, 2, NET, 50_000, &[(foreign_script(), 49_000)]);
    let inner = psbt.inner_mut();
    inner
        .unsigned_tx
        .input
        .push(other.unsigned_tx().input[0].clone());
    inner.inputs.push(other.inner().inputs[0].clone());
    assert!(!inspect(&psbt, &ctx).has_blocked());
    psbt.inner_mut().inputs[0].non_witness_utxo = None;
    let insp = inspect(&psbt, &ctx);
    assert!(has(&insp, Level::Blocked, &WarningKind::AmountUnverified));
    assert_eq!(
        sign(
            &mut psbt,
            &[&abandon],
            &[abandon.fingerprint()],
            &ctx,
            true,
            Nonce::LowR,
            Aux::Deterministic,
            &mut None,
            [0u8; 32],
            &[],
            &[],
        ),
        Err(Error::Blocked(vec![WarningKind::AmountUnverified]))
    );
}

#[test]
fn mixed_script_types_is_info() {
    // Two PSBTs cannot be merged simply; emulate by adding a second input
    // of another type to a native-segwit spend.
    let abandon = master(ABANDON, NET);
    let mut psbt = basic(ScriptType::NativeSegwit);
    let other = basic(ScriptType::Taproot);
    psbt.inner_mut()
        .unsigned_tx
        .input
        .push(other.unsigned_tx().input[0].clone());
    psbt.inner_mut()
        .inputs
        .push(other.inner().inputs[0].clone());
    let insp = inspect_with_abandon(&psbt);
    assert!(has(&insp, Level::Info, &WarningKind::MixedScriptTypes));
    assert_eq!(insp.total_in, Amount::from_sat(200_000));
    assert!(insp.inputs.iter().all(|i| i.is_ours));
    let keys = [KeyRef::from_master(&abandon, 0).unwrap()];
    let ctx = Context {
        network: NET,
        keys: &keys,
        wallets: &[],
        musig_session: None,
        shares: &[],
        carry: None,
    };
    let r = sign(
        &mut psbt,
        &[&abandon],
        &[abandon.fingerprint()],
        &ctx,
        true,
        Nonce::LowR,
        Aux::Deterministic,
        &mut None,
        [0u8; 32],
        &[],
        &[],
    )
    .unwrap();
    assert_eq!(r.signed_inputs.len(), 2);
    assert!(r.complete);
}
