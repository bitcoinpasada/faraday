//! Multisig: a 2-of-3 `sortedmulti` p2wsh built from three test seeds,
//! and the BIP-174 role chain (p2sh and p2sh-p2wsh 2-of-2) signed,
//! finalized and extracted against the vectors Bitcoin Core produced.
//! The role chain's p2sh-p2wsh input carries no previous transaction, so
//! the signer blocks it and the chain is followed with that input final.

mod common;

use std::str::FromStr;

use bitcoin::bip32::Xpriv;
use bitcoin::consensus::{deserialize, serialize};
use bitcoin::secp256k1::{Message, Secp256k1};
use bitcoin::sighash::{EcdsaSighashType, SighashCache};
use bitcoin::{ScriptBuf, Transaction};
use common::builder::{
    multisig_policy, sorted_multisig, sorted_multisig_change, sorted_multisig_change_with_script,
};
use common::{ABANDON, LEGAL, ZOO, master, records, unhex};
use osk_bip::keys::{MasterKey, Network, ScriptType};
use osk_psbt::{
    Aux, Context, Error, KeyRef, Level, Nonce, OutputKind, Psbt, ScriptKind, WarningKind, Wrapper,
    finalize, inspect, is_complete, sign,
};

const NET: Network = Network::Regtest;

#[test]
fn two_of_three_sortedmulti() {
    let secp = Secp256k1::new();
    let abandon = master(ABANDON, NET);
    let zoo = master(ZOO, NET);
    let legal = master(LEGAL, NET);
    let recipient = legal
        .account_xpub(ScriptType::NativeSegwit, 0)
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
        KeyRef::from_master(&abandon, 0).unwrap(),
        KeyRef::from_master(&zoo, 0).unwrap(),
    ];
    let ctx = Context {
        network: NET,
        keys: &keys,
        wallets: &[],
        musig_session: None,
        shares: &[],
        carry: None,
    };
    let insp = inspect(&psbt, &ctx);
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
        &mut psbt,
        &[&abandon, &zoo],
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
    assert_eq!(r.signed_inputs.len(), 1);
    assert!(!r.complete);
    assert!(!is_complete(&psbt));
    assert_eq!(finalize(&mut psbt).unwrap(), None);
    let insp = inspect(&psbt, &ctx);
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
        &mut psbt,
        &[&abandon, &zoo],
        &[zoo.fingerprint()],
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
    assert_eq!(r.signed_inputs.len(), 1);
    assert!(r.complete);
    let insp = inspect(&psbt, &ctx);
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
    let keys = [KeyRef::from_master(&legal, 0).unwrap()];
    let ctx = Context {
        network: NET,
        keys: &keys,
        wallets: &[],
        musig_session: None,
        shares: &[],
        carry: None,
    };
    let mut fresh = unsigned.clone();
    let r = sign(
        &mut fresh,
        &[&legal],
        &[legal.fingerprint()],
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
    assert_eq!(r.signed_inputs.len(), 1);
    assert!(!r.complete);
}

#[test]
fn multisig_participation_by_fingerprint_only() {
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
    let keys = [KeyRef::from_master(&other, 0).unwrap()];
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
fn bip174_signer_finalizer_extractor() {
    let recs = records("tools/vectors/psbt/bip174-roles.txt");
    let get = |name: &str| {
        let rec = recs
            .iter()
            .find(|r| r.opt("case") == Some(&format!("role {name}")))
            .unwrap_or_else(|| panic!("no role {name}"));
        rec.clone()
    };
    let tprv = Xpriv::from_str(recs[0].get("master-tprv")).unwrap();
    let master = MasterKey::from_xpriv(tprv).unwrap();
    let fp = master.fingerprint();

    let mut psbt = Psbt::parse_bytes(&unhex(get("updater-sighash-all").get("hex"))).unwrap();
    let keys = [KeyRef::from_master(&master, 0).unwrap()];
    let ctx = Context {
        network: Network::Testnet,
        keys: &keys,
        wallets: &[],
        musig_session: None,
        shares: &[],
        carry: None,
    };
    let insp = inspect(&psbt, &ctx);
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
                &mut psbt,
                &[&master],
                &[fp],
                &ctx,
                force,
                Nonce::First,
                Aux::Deterministic,
                &mut None,
                [0u8; 32],
                &[],
                &[],
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

    // The same PSBT with the p2sh-p2wsh input already final, which is
    // how the finalizer vector leaves it. A final input is signed by
    // nobody, so the amount it states is nobody's business any more.
    let finalizer = Psbt::parse_bytes(&unhex(get("finalizer").get("hex"))).unwrap();
    let mut psbt = Psbt::parse_bytes(&unhex(get("updater-sighash-all").get("hex"))).unwrap();
    psbt.inner_mut().inputs[1] = finalizer.inner().inputs[1].clone();

    let r = sign(
        &mut psbt,
        &[&master],
        &[fp],
        &ctx,
        false,
        Nonce::First,
        Aux::Deterministic,
        &mut None,
        [0u8; 32],
        &[],
        &[],
    )
    .unwrap();
    assert_eq!(r.signed_inputs.len(), 2, "both keys of the one open input");
    assert!(r.complete);
    // Both signatures are the bytes Bitcoin Core produced for that input.
    let combiner = Psbt::parse_bytes(&unhex(get("combiner").get("hex"))).unwrap();
    assert_eq!(
        psbt.inner().inputs[0].partial_sigs,
        combiner.inner().inputs[0].partial_sigs,
        "signer vectors"
    );

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

/// The same chain under Low R, on the input the signer will sign. One of
/// its vector signatures already has a low `r` and comes out unchanged;
/// the other, whose vector `r` is 33 bytes, is ground to a different,
/// shorter signature that still verifies, and the PSBT still finalizes
/// and extracts a valid transaction.
#[test]
fn bip174_under_low_r() {
    let recs = records("tools/vectors/psbt/bip174-roles.txt");
    let get = |name: &str| {
        recs.iter()
            .find(|r| r.opt("case") == Some(&format!("role {name}")))
            .unwrap_or_else(|| panic!("no role {name}"))
            .clone()
    };
    let tprv = Xpriv::from_str(recs[0].get("master-tprv")).unwrap();
    let master = MasterKey::from_xpriv(tprv).unwrap();
    let fp = master.fingerprint();
    let keys = [KeyRef::from_master(&master, 0).unwrap()];
    let ctx = Context {
        network: Network::Testnet,
        keys: &keys,
        wallets: &[],
        musig_session: None,
        shares: &[],
        carry: None,
    };

    let finalizer = Psbt::parse_bytes(&unhex(get("finalizer").get("hex"))).unwrap();
    let mut psbt = Psbt::parse_bytes(&unhex(get("updater-sighash-all").get("hex"))).unwrap();
    psbt.inner_mut().inputs[1] = finalizer.inner().inputs[1].clone();
    let unsigned = psbt.clone();
    sign(
        &mut psbt,
        &[&master],
        &[fp],
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
    let combiner = Psbt::parse_bytes(&unhex(get("combiner").get("hex"))).unwrap();

    let secp = Secp256k1::new();
    let (mut same, mut ground) = (0, 0);
    for (i, input) in psbt.inner().inputs.iter().enumerate() {
        let theirs = &combiner.inner().inputs[i].partial_sigs;
        for (pk, sig) in &input.partial_sigs {
            let der = sig.signature.serialize_der();
            assert!(der.len() <= 70, "input {i}: {} DER bytes", der.len());
            assert!(
                sig.signature.serialize_compact()[0] < 0x80,
                "input {i}: high R"
            );
            let vector = &theirs[pk];
            if vector.signature.serialize_der().len() <= 70 {
                assert_eq!(sig, vector, "input {i}: a low-R vector signature changed");
                same += 1;
            } else {
                assert_ne!(sig, vector, "input {i}: the high-R vector was not ground");
                ground += 1;
            }
            // The ground signature is a signature of the same input.
            let msg = sighash_of(&unsigned, i);
            secp.verify_ecdsa(&msg, &sig.signature, &pk.inner)
                .unwrap_or_else(|e| panic!("input {i}: {e}"));
        }
    }
    assert_eq!((same, ground), (1, 1));

    let tx = finalize(&mut psbt).unwrap().expect("complete");
    assert_eq!(tx.input.len(), 2);
    assert_ne!(serialize(&tx), unhex(get("extractor").get("tx")));
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
    let abandon = master(ABANDON, NET);
    let zoo = master(ZOO, NET);
    let legal = master(LEGAL, NET);
    let recipient = legal
        .account_xpub(ScriptType::NativeSegwit, 0)
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
    let keys = [KeyRef::from_master(&abandon, 0).unwrap()];

    // Nothing registered: the change output is a claim the device cannot
    // check, so it counts as a payment to others.
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
        shares: &[],
        carry: None,
    };
    let insp = inspect(&psbt, &ctx);
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
        &psbt,
        &Context {
            network: NET,
            keys: &keys,
            wallets: &wallets,
            musig_session: None,
            shares: &[],
            carry: None,
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

/// Change of a multisig wallet with nothing registered, when the
/// coordinator states the script it pays to: the device derives its own
/// key on the BIP-48 change branch, finds it in that script, and sees the
/// output commits to it, so the money counts as change and not as a
/// payment to others. A wallet none of whose keys is ours stays a payment
/// to others whatever it claims.
#[test]
fn bip48_change_is_ours_before_a_wallet_is_registered() {
    let abandon = master(ABANDON, NET);
    let zoo = master(ZOO, NET);
    let legal = master(LEGAL, NET);
    let recipient = legal
        .account_xpub(ScriptType::NativeSegwit, 0)
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
    let keys = [KeyRef::from_master(&abandon, 0).unwrap()];
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
    assert!(!insp.has_danger(), "{:?}", insp.warnings);
    assert_eq!(
        insp.outputs[1].kind,
        OutputKind::Change {
            key: abandon.fingerprint(),
            change: true,
            index: 0
        }
    );
    assert!(insp.outputs[1].is_ours());
    assert_eq!(insp.change_total.to_sat(), 39_000);
    assert_eq!(insp.amount_to_others.to_sat(), 60_000);
    assert!(
        insp.warnings
            .iter()
            .all(|w| w.kind != WarningKind::UnverifiedChange)
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
        &theirs,
        &Context {
            network: NET,
            keys: &keys,
            wallets: &[],
            musig_session: None,
            shares: &[],
            carry: None,
        },
    );
    assert!(matches!(
        insp.outputs[1].kind,
        OutputKind::UnverifiedChange { .. }
    ));
    assert_eq!(insp.change_total.to_sat(), 0);
    assert_eq!(insp.amount_to_others.to_sat(), 99_000);
}
