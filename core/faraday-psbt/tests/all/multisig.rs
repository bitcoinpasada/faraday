//! Multisig: a 2-of-3 `sortedmulti` p2wsh built from three test seeds,
//! and the BIP-174 role chain (p2sh and p2sh-p2wsh 2-of-2) signed,
//! finalized and extracted against the vectors Bitcoin Core produced.
//! The role chain's p2sh-p2wsh input carries no previous transaction, so
//! the signer blocks it, finalized or not, and the chain is followed from
//! the combiner vector.

use std::str::FromStr;

use crate::common::builder::{
    multisig_policy, sorted_multisig, sorted_multisig_change, sorted_multisig_change_with_script,
};
use crate::common::{ABANDON, LEGAL, ZOO, master, records, unhex};
use bitcoin::bip32::Xpriv;
use bitcoin::consensus::{deserialize, serialize};
use bitcoin::secp256k1::{Message, Secp256k1};
use bitcoin::sighash::{EcdsaSighashType, SighashCache};
use bitcoin::{ScriptBuf, Transaction};
use faraday_bip::keys::{MasterKey, Network, ScriptType};
use faraday_crypto::work::Compute;
use faraday_psbt::{
    Aux, Context, Error, KeyRef, Level, Nonce, OutputKind, Psbt, ScriptKind, WarningKind, Wrapper,
    finalize, inspect, is_complete, sign,
};

const NET: Network = Network::Regtest;

#[test]
#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
fn two_of_three_sortedmulti() {
    let c = Compute::unchecked();
    let secp = Secp256k1::new();
    let abandon = master(ABANDON, NET);
    let zoo = master(ZOO, NET);
    let legal = master(LEGAL, NET);
    let recipient = legal
        .account_xpub(&c, ScriptType::NativeSegwit, 0)
        .unwrap()
        .address(false, 3)
        .unwrap()
        .script_pubkey();
    let mut psbt = sorted_multisig(
        &[&abandon, &zoo, &legal],
        2,
        NET,
        100_000,
        &[(recipient, 99_000)],
    );
    let unsigned = psbt.clone();

    // Two of the three keys are loaded.
    let keys = [
        KeyRef::from_master(&c, &abandon, 0).unwrap(),
        KeyRef::from_master(&c, &zoo, 0).unwrap(),
    ];
    let ctx = Context {
        network: NET,
        keys: &keys,
        wallets: &[],
        musig_session: None,
    };
    let insp = inspect(&c, &psbt, &ctx);
    assert!(!insp.has_danger(), "{:?}", insp.warnings);
    assert_eq!(
        insp.inputs[0].script_type,
        ScriptKind::Multisig {
            wrapper: Wrapper::P2wsh,
            m: 2,
            n: 3
        }
    );
    assert!(insp.inputs[0].is_ours);
    let ms = insp.multisig.as_ref().unwrap();
    assert_eq!((ms.m, ms.n), (2, 3));
    assert!(ms.ours);
    assert_eq!(ms.signed(), 0);
    let mut fps: Vec<_> = ms
        .cosigners
        .iter()
        .map(|c| c.fingerprint.unwrap())
        .collect();
    fps.sort_by_key(|f| f.0);
    let mut expected = vec![
        abandon.fingerprint(),
        zoo.fingerprint(),
        legal.fingerprint(),
    ];
    expected.sort_by_key(|f| f.0);
    assert_eq!(fps, expected);
    assert_eq!(ms.cosigners.iter().filter(|c| c.ours).count(), 2);
    let mut participating = insp.participating_keys.clone();
    participating.sort_by_key(|f| f.0);
    let mut ours = vec![abandon.fingerprint(), zoo.fingerprint()];
    ours.sort_by_key(|f| f.0);
    assert_eq!(participating, ours);
    assert_eq!(insp.outputs[0].kind, OutputKind::Recipient);

    // First signature: not complete.
    let r = sign(
        &c,
        &mut psbt,
        &[&abandon, &zoo],
        &[abandon.fingerprint()],
        &ctx,
        false,
        Nonce::LowR,
        Aux::Deterministic,
        &mut None,
        &faraday_crypto::Zeroizing::new([0u8; 32]),
    )
    .unwrap();
    assert_eq!(r.signed_inputs.len(), 1);
    assert!(!r.complete);
    assert!(!is_complete(&psbt));
    assert_eq!(finalize(&mut psbt).unwrap(), None);
    let insp = inspect(&c, &psbt, &ctx);
    let ms = insp.multisig.as_ref().unwrap();
    assert_eq!(ms.signed(), 1);
    assert!(
        ms.cosigners
            .iter()
            .any(|c| c.signed && c.fingerprint == Some(abandon.fingerprint()))
    );
    assert_eq!(
        insp.inputs[0].already_signed_by,
        vec![abandon.fingerprint()]
    );
    assert_eq!(insp.inputs[0].signatures, 1);

    // Second signature: complete.
    let r = sign(
        &c,
        &mut psbt,
        &[&abandon, &zoo],
        &[zoo.fingerprint()],
        &ctx,
        false,
        Nonce::LowR,
        Aux::Deterministic,
        &mut None,
        &faraday_crypto::Zeroizing::new([0u8; 32]),
    )
    .unwrap();
    assert_eq!(r.signed_inputs.len(), 1);
    assert!(r.complete);
    let insp = inspect(&c, &psbt, &ctx);
    assert_eq!(insp.multisig.as_ref().unwrap().signed(), 2);
    assert_eq!(insp.inputs[0].signatures, 2);

    // Both signatures verify against the p2wsh sighash.
    let ws = unsigned.inner().inputs[0].witness_script.clone().unwrap();
    let utxo = unsigned.inner().spend_utxo(0).unwrap().clone();
    let mut cache = SighashCache::new(unsigned.unsigned_tx());
    let msg = Message::from(
        cache
            .p2wsh_signature_hash(0, &ws, utxo.value, EcdsaSighashType::All)
            .unwrap(),
    );
    for (pk, sig) in &psbt.inner().inputs[0].partial_sigs {
        secp.verify_ecdsa(&msg, &sig.signature, &pk.inner).unwrap();
    }

    let tx = finalize(&mut psbt).unwrap().expect("complete");
    let again: Transaction = deserialize(&serialize(&tx)).unwrap();
    assert_eq!(again, tx);
    let w = &tx.input[0].witness;
    assert_eq!(w.len(), 4, "empty, sig, sig, script");
    assert!(w.nth(0).unwrap().is_empty());
    assert_eq!(w.nth(3).unwrap(), ws.as_bytes());
    assert!(tx.input[0].script_sig.is_empty());
    // Signatures are in script key order and each is 71-73 bytes with a
    // trailing SIGHASH_ALL.
    for sig in [w.nth(1).unwrap(), w.nth(2).unwrap()] {
        assert!((70..=73).contains(&sig.len()));
        assert_eq!(*sig.last().unwrap(), 1);
    }

    // The third key alone cannot complete a fresh copy.
    let keys = [KeyRef::from_master(&c, &legal, 0).unwrap()];
    let ctx = Context {
        network: NET,
        keys: &keys,
        wallets: &[],
        musig_session: None,
    };
    let mut fresh = unsigned.clone();
    let r = sign(
        &c,
        &mut fresh,
        &[&legal],
        &[legal.fingerprint()],
        &ctx,
        false,
        Nonce::LowR,
        Aux::Deterministic,
        &mut None,
        &faraday_crypto::Zeroizing::new([0u8; 32]),
    )
    .unwrap();
    assert_eq!(r.signed_inputs.len(), 1);
    assert!(!r.complete);
}

#[test]
fn multisig_participation_by_fingerprint_only() {
    let c = Compute::unchecked();
    let abandon = master(ABANDON, NET);
    let zoo = master(ZOO, NET);
    let legal = master(LEGAL, NET);
    let recipient = ScriptBuf::new_op_return(b"x");
    let psbt = sorted_multisig(
        &[&abandon, &zoo, &legal],
        2,
        NET,
        100_000,
        &[(recipient, 99_000)],
    );
    // A loaded key that is not a cosigner.
    let other = master(
        "letter advice cage absurd amount doctor acoustic avoid letter advice cage above",
        NET,
    );
    let keys = [KeyRef::from_master(&c, &other, 0).unwrap()];
    let insp = inspect(
        &c,
        &psbt,
        &Context {
            network: NET,
            keys: &keys,
            wallets: &[],
            musig_session: None,
        },
    );
    let ms = insp.multisig.as_ref().unwrap();
    assert!(!ms.ours);
    assert!(!insp.inputs[0].is_ours);
    assert!(
        insp.warnings
            .iter()
            .any(|w| w.level == Level::Danger && w.kind == WarningKind::NoParticipatingKey)
    );
}

/// The BIP-174 chain, as far as this signer will go down it. The
/// vector's p2sh-p2wsh input carries `witness_utxo` and no previous
/// transaction, which this signer refuses, so the vector as published is
/// blocked. With that input arriving already final — the state the
/// finalizer vector leaves it in — the legacy input beside it is signed,
/// and its two signatures, the finalizer PSBT and the extracted
/// transaction are the vectors' own bytes.
///
/// The vectors were made without grinding — one of the legacy input's
/// signatures carries a 33-byte `r` — so they are the First setting's
/// bytes.
#[test]
#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
fn bip174_signer_finalizer_extractor() {
    let c = Compute::unchecked();
    let recs = records("tools/vectors/psbt/bip174-roles.txt");
    let get = |name: &str| {
        let rec = recs
            .iter()
            .find(|r| r.opt("case") == Some(&format!("role {name}")))
            .unwrap_or_else(|| panic!("no role {name}"));
        rec.clone()
    };
    let tprv = Xpriv::from_str(recs[0].get("master-tprv")).unwrap();
    let master = MasterKey::from_xpriv(&c, tprv).unwrap();
    let fp = master.fingerprint();

    let mut psbt = Psbt::parse_bytes(&c, &unhex(get("updater-sighash-all").get("hex"))).unwrap();
    let keys = [KeyRef::from_master(&c, &master, 0).unwrap()];
    let ctx = Context {
        network: Network::Testnet,
        keys: &keys,
        wallets: &[],
        musig_session: None,
    };
    let insp = inspect(&c, &psbt, &ctx);
    assert!(!insp.has_danger(), "{:?}", insp.warnings);
    assert_eq!(
        insp.inputs[0].script_type,
        ScriptKind::Multisig {
            wrapper: Wrapper::P2sh,
            m: 2,
            n: 2
        }
    );
    assert_eq!(
        insp.inputs[1].script_type,
        ScriptKind::Multisig {
            wrapper: Wrapper::P2shP2wsh,
            m: 2,
            n: 2
        }
    );
    assert!(insp.inputs.iter().all(|i| i.is_ours));
    assert_eq!(insp.participating_keys, vec![fp]);
    // The outputs claim origins under m/0'/0'/4' and 5': not verifiable
    // from BIP-44/49/84/86 accounts, so unverified, not spoofed.
    assert!(
        insp.outputs
            .iter()
            .all(|o| matches!(o.kind, OutputKind::UnverifiedChange { key } if key == fp))
    );
    assert!(
        insp.warnings
            .iter()
            .all(|w| w.kind != WarningKind::ChangeSpoof)
    );

    // Input 1 states its amount and carries nothing that commits to it,
    // so the vector as published is refused however it is asked.
    assert_eq!(insp.blocked(), vec![WarningKind::AmountUnverified]);
    for force in [false, true] {
        assert_eq!(
            sign(
                &c,
                &mut psbt,
                &[&master],
                &[fp],
                &ctx,
                force,
                Nonce::First,
                Aux::Deterministic,
                &mut None,
                &faraday_crypto::Zeroizing::new([0u8; 32]),
            ),
            Err(Error::Blocked(vec![WarningKind::AmountUnverified])),
            "forced {force}"
        );
    }
    assert!(
        psbt.inner()
            .inputs
            .iter()
            .all(|i| i.partial_sigs.is_empty())
    );

    // The same PSBT with the p2sh-p2wsh input already final, which is how
    // the finalizer vector leaves it. Its final witness is the
    // coordinator's, so the amount it states is held to the same rule and
    // the PSBT is refused until the input carries its previous transaction.
    let finalizer = Psbt::parse_bytes(&c, &unhex(get("finalizer").get("hex"))).unwrap();
    let mut psbt = Psbt::parse_bytes(&c, &unhex(get("updater-sighash-all").get("hex"))).unwrap();
    psbt.inner_mut().inputs[1] = finalizer.inner().inputs[1].clone();
    assert_eq!(
        inspect(&c, &psbt, &ctx).blocked(),
        vec![WarningKind::AmountUnverified]
    );
    assert_eq!(
        sign(
            &c,
            &mut psbt,
            &[&master],
            &[fp],
            &ctx,
            true,
            Nonce::First,
            Aux::Deterministic,
            &mut None,
            &faraday_crypto::Zeroizing::new([0u8; 32]),
        ),
        Err(Error::Blocked(vec![WarningKind::AmountUnverified]))
    );

    // The rest of the chain from the combiner vector, which holds both
    // signatures Bitcoin Core produced.
    let mut psbt = Psbt::parse_bytes(&c, &unhex(get("combiner").get("hex"))).unwrap();
    let tx = finalize(&mut psbt).unwrap().expect("complete");
    assert_eq!(
        psbt.to_bytes(),
        unhex(get("finalizer").get("hex")),
        "finalizer vector"
    );
    assert_eq!(
        serialize(&tx),
        unhex(get("extractor").get("tx")),
        "extractor vector"
    );
}

/// A 2-of-3 p2wsh spend signed under Low R: every signature has a low `r`
/// and a short DER encoding, verifies against an independently computed
/// sighash, and the PSBT finalizes and extracts a transaction that
/// round-trips.
#[test]
fn multisig_under_low_r() {
    let c = Compute::unchecked();
    let abandon = master(ABANDON, NET);
    let zoo = master(ZOO, NET);
    let legal = master(LEGAL, NET);
    let keys = [
        KeyRef::from_master(&c, &abandon, 0).unwrap(),
        KeyRef::from_master(&c, &zoo, 0).unwrap(),
    ];
    let ctx = Context {
        network: NET,
        keys: &keys,
        wallets: &[],
        musig_session: None,
    };
    let recipient = legal
        .account_xpub(&c, ScriptType::NativeSegwit, 0)
        .unwrap()
        .address(false, 3)
        .unwrap()
        .script_pubkey();
    let mut psbt = sorted_multisig(
        &[&abandon, &zoo, &legal],
        2,
        NET,
        100_000,
        &[(recipient, 99_000)],
    );
    let unsigned = psbt.clone();
    sign(
        &c,
        &mut psbt,
        &[&abandon, &zoo],
        &[abandon.fingerprint(), zoo.fingerprint()],
        &ctx,
        false,
        Nonce::LowR,
        Aux::Deterministic,
        &mut None,
        &faraday_crypto::Zeroizing::new([0u8; 32]),
    )
    .unwrap();

    let secp = Secp256k1::new();
    let mut signatures = 0;
    for (i, input) in psbt.inner().inputs.iter().enumerate() {
        for (pk, sig) in &input.partial_sigs {
            assert!(
                sig.signature.serialize_der().len() <= 70,
                "input {i}: long DER"
            );
            assert!(
                sig.signature.serialize_compact()[0] < 0x80,
                "input {i}: high R"
            );
            let msg = sighash_of(&unsigned, i);
            secp.verify_ecdsa(&msg, &sig.signature, &pk.inner)
                .unwrap_or_else(|e| panic!("input {i}: {e}"));
            signatures += 1;
        }
    }
    assert_eq!(signatures, 2);

    let tx = finalize(&mut psbt).unwrap().expect("complete");
    assert_eq!(tx.input.len(), 1);
    assert_eq!(
        deserialize::<Transaction>(&serialize(&tx)).unwrap(),
        tx,
        "the extracted transaction round-trips"
    );
}

/// The ECDSA sighash of input `index` of a PSBT, whatever its script.
fn sighash_of(psbt: &Psbt, index: usize) -> Message {
    let tx = psbt.inner().unsigned_tx.clone();
    let mut cache = SighashCache::new(&tx);
    psbt.inner().sighash_ecdsa(index, &mut cache).unwrap().0
}

/// Change of a registered wallet: without the wallet the device can only
/// repeat the coordinator's claim, with it the device has derived the
/// address itself, and a wallet whose cosigner is not the one that
/// signed pays somewhere else than it says.
#[test]
fn change_of_a_registered_wallet() {
    let c = Compute::unchecked();
    let abandon = master(ABANDON, NET);
    let zoo = master(ZOO, NET);
    let legal = master(LEGAL, NET);
    let recipient = legal
        .account_xpub(&c, ScriptType::NativeSegwit, 0)
        .unwrap()
        .address(false, 3)
        .unwrap()
        .script_pubkey();
    let psbt = sorted_multisig_change(
        &[&abandon, &zoo, &legal],
        2,
        NET,
        100_000,
        &[(recipient, 60_000)],
        39_000,
    );
    let keys = [KeyRef::from_master(&c, &abandon, 0).unwrap()];

    // Nothing registered: the change output is a claim the device cannot
    // check, so it counts as a payment to others.
    let insp = inspect(
        &c,
        &psbt,
        &Context {
            network: NET,
            keys: &keys,
            wallets: &[],
            musig_session: None,
        },
    );
    assert!(matches!(
        insp.outputs[1].kind,
        OutputKind::UnverifiedChange { .. }
    ));
    assert_eq!(insp.change_total.to_sat(), 0);
    assert_eq!(insp.amount_to_others.to_sat(), 99_000);

    // The wallet in use: the same output is verified change of it, the
    // input is one of its spends, and the loaded key is a participant.
    let wallets = [multisig_policy(&[&abandon, &zoo, &legal], 2, NET)];
    let ctx = Context {
        network: NET,
        keys: &keys,
        wallets: &wallets,
        musig_session: None,
    };
    let insp = inspect(&c, &psbt, &ctx);
    assert!(!insp.has_danger(), "{:?}", insp.warnings);
    assert_eq!(
        insp.outputs[1].kind,
        OutputKind::WalletChange {
            wallet: 0,
            change: true,
            index: 0
        }
    );
    assert_eq!(insp.change_total.to_sat(), 39_000);
    assert_eq!(insp.amount_to_others.to_sat(), 60_000);
    assert_eq!(insp.multisig.as_ref().unwrap().wallet, Some(0));
    assert!(insp.inputs[0].is_ours);
    assert_eq!(insp.participating_keys, vec![abandon.fingerprint()]);

    // A wallet with one cosigner swapped is a different wallet: the
    // output that says it is change of it is not its address.
    let other = master(
        "letter advice cage absurd amount doctor acoustic avoid letter advice cage above",
        NET,
    );
    let wallets = [multisig_policy(&[&abandon, &zoo, &other], 2, NET)];
    let insp = inspect(
        &c,
        &psbt,
        &Context {
            network: NET,
            keys: &keys,
            wallets: &wallets,
            musig_session: None,
        },
    );
    assert!(matches!(
        insp.outputs[1].kind,
        OutputKind::UnverifiedChange { .. }
    ));
    assert!(
        insp.warnings
            .iter()
            .any(|w| w.level == Level::Blocked && w.kind == WarningKind::ChangeSpoof)
    );
    assert_eq!(insp.multisig.as_ref().unwrap().wallet, None);
}

/// Change of a multisig wallet with nothing registered is unverified,
/// even when the coordinator states the script it pays to: the threshold
/// and the cosigners are the coordinator's word until the wallet is
/// registered, so the output counts as a payment to others and carries a
/// caution that says what fixes it.
#[test]
fn bip48_change_is_unverified_before_a_wallet_is_registered() {
    let c = Compute::unchecked();
    let abandon = master(ABANDON, NET);
    let zoo = master(ZOO, NET);
    let legal = master(LEGAL, NET);
    let recipient = legal
        .account_xpub(&c, ScriptType::NativeSegwit, 0)
        .unwrap()
        .address(false, 3)
        .unwrap()
        .script_pubkey();
    let psbt = sorted_multisig_change_with_script(
        &[&abandon, &zoo, &legal],
        2,
        NET,
        100_000,
        &[(recipient.clone(), 60_000)],
        39_000,
    );
    let keys = [KeyRef::from_master(&c, &abandon, 0).unwrap()];
    let insp = inspect(
        &c,
        &psbt,
        &Context {
            network: NET,
            keys: &keys,
            wallets: &[],
            musig_session: None,
        },
    );
    assert!(!insp.has_danger(), "{:?}", insp.warnings);
    assert!(matches!(
        insp.outputs[1].kind,
        OutputKind::UnverifiedChange { .. }
    ));
    assert!(!insp.outputs[1].is_ours());
    assert_eq!(insp.change_total.to_sat(), 0);
    assert_eq!(insp.amount_to_others.to_sat(), 99_000);
    assert!(
        insp.warnings.iter().any(|w| w.level == Level::Caution
            && w.kind == WarningKind::UnverifiedChange
            && w.text.contains("Register the wallet to check its change")),
        "{:?}",
        insp.warnings
    );

    // The key that derives the change branch is the loaded one, not a
    // path string we trusted: the same output under a wallet of three
    // other keys is a payment to others.
    let other = master(
        "letter advice cage absurd amount doctor acoustic avoid letter advice cage above",
        NET,
    );
    let theirs = sorted_multisig_change_with_script(
        &[&other, &zoo, &legal],
        2,
        NET,
        100_000,
        &[(recipient, 60_000)],
        39_000,
    );
    let insp = inspect(
        &c,
        &theirs,
        &Context {
            network: NET,
            keys: &keys,
            wallets: &[],
            musig_session: None,
        },
    );
    assert!(matches!(
        insp.outputs[1].kind,
        OutputKind::UnverifiedChange { .. }
    ));
    assert_eq!(insp.change_total.to_sat(), 0);
    assert_eq!(insp.amount_to_others.to_sat(), 99_000);
}

/// A change output whose script merely holds this key, or whose threshold
/// is lower than the wallet's, is unverified with no wallet registered
/// and a blocked spoof once the real wallet is.
#[test]
fn multisig_change_scripts_that_only_hold_our_key_are_unverified() {
    use bitcoin::PublicKey;
    use bitcoin::opcodes::OP_TRUE;
    use bitcoin::opcodes::all::{OP_CHECKMULTISIG, OP_DROP};
    use bitcoin::script::Builder;

    let c = Compute::unchecked();
    let abandon = master(ABANDON, NET);
    let zoo = master(ZOO, NET);
    let legal = master(LEGAL, NET);
    let recipient = legal
        .account_xpub(&c, ScriptType::NativeSegwit, 0)
        .unwrap()
        .address(false, 3)
        .unwrap()
        .script_pubkey();
    let keys = [KeyRef::from_master(&c, &abandon, 0).unwrap()];
    let path = crate::common::builder::bip48_path(NET).extend([
        bitcoin::bip32::ChildNumber::from_normal_idx(1).unwrap(),
        bitcoin::bip32::ChildNumber::from_normal_idx(0).unwrap(),
    ]);
    let ours = PublicKey::new(abandon.derive(&c, &path).to_xpub(&c).public_key);
    let theirs = PublicKey::new(zoo.derive(&c, &path).to_xpub(&c).public_key);
    let scripts = [
        Builder::new()
            .push_key(&ours)
            .push_opcode(OP_DROP)
            .push_opcode(OP_TRUE)
            .into_script(),
        Builder::new()
            .push_int(1)
            .push_key(&ours)
            .push_key(&theirs)
            .push_int(2)
            .push_opcode(OP_CHECKMULTISIG)
            .into_script(),
    ];
    let wallets = [multisig_policy(&[&abandon, &zoo, &legal], 2, NET)];
    for script in scripts {
        let mut psbt = sorted_multisig_change_with_script(
            &[&abandon, &zoo, &legal],
            2,
            NET,
            100_000,
            &[(recipient.clone(), 60_000)],
            39_000,
        );
        let inner = psbt.inner_mut();
        inner.unsigned_tx.output[1].script_pubkey = script.to_p2wsh();
        inner.outputs[1].witness_script = Some(script);
        inner.outputs[1]
            .bip32_derivation
            .retain(|k, _| *k == ours.inner);

        let ctx = |wallets| Context {
            network: NET,
            keys: &keys,
            wallets,
            musig_session: None,
        };
        let insp = inspect(&c, &psbt, &ctx(&[]));
        assert!(matches!(
            insp.outputs[1].kind,
            OutputKind::UnverifiedChange { .. }
        ));
        assert_eq!(insp.change_total.to_sat(), 0);
        assert_eq!(insp.amount_to_others.to_sat(), 99_000);

        let insp = inspect(&c, &psbt, &ctx(&wallets));
        assert!(matches!(
            insp.outputs[1].kind,
            OutputKind::UnverifiedChange { .. }
        ));
        assert!(
            insp.warnings
                .iter()
                .any(|w| w.level == Level::Blocked && w.kind == WarningKind::ChangeSpoof)
        );
        assert_eq!(insp.amount_to_others.to_sat(), 99_000);
    }
}
