//! A spend of a miniscript wallet: whose the change is, and what the
//! device will and will not do with the input.

use faraday_bip::keys::{MasterKey, Network};
use faraday_bip::policy::WalletPolicy;
use faraday_bip::spend::{Lock, SpendPath};
use faraday_crypto::work::Compute;
use faraday_psbt::{
    Aux, Context, KeyRef, Nonce, OutputKind, Psbt, ScriptKind, SigKind, SpendRoute, WarningKind,
    Wrapper, finalize, inspect, sign,
};

const ABANDON: &str =
    "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
const ZOO: &str = "zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo wrong";
const LEGAL: &str = "legal winner thank year wave sausage worth useful legal winner thank yellow";
const NET: Network = Network::Regtest;

const LIANA_POLICY: &str = include_str!("../../../../tools/vectors/psbt/wallet-liana.policy");
const LIANA_PSBT: &str = include_str!("../../../../tools/vectors/psbt/wallet-liana.psbt");
const LIANA_RECOVERY_PSBT: &str =
    include_str!("../../../../tools/vectors/psbt/wallet-liana-recovery.psbt");
const TREE_POLICY: &str = include_str!("../../../../tools/vectors/psbt/wallet-tree.policy");
const TREE_PSBT: &str = include_str!("../../../../tools/vectors/psbt/wallet-tree.psbt");
const TREE_KEYPATH_PSBT: &str =
    include_str!("../../../../tools/vectors/psbt/wallet-tree-keypath.psbt");

fn path(keys: &[usize], locks: &[Lock]) -> SpendPath {
    SpendPath {
        keys: keys.to_vec(),
        locks: locks.to_vec(),
    }
}

fn master(words: &str) -> MasterKey {
    let c = Compute::unchecked();
    let m = faraday_bip::bip39::Mnemonic::parse(&c, faraday_bip::bip39::Language::English, words)
        .expect("fixed mnemonic");
    MasterKey::from_seed(&c, &m.to_seed(&c, b"").expect("empty passphrase"), NET)
}

/// The change of a miniscript wallet is verified the same way a
/// multisig wallet's is: by re-deriving the wallet's own script at the
/// index the key origins name. With the wallet not in use there is
/// nothing to re-derive from, so the same output is only claimed.
#[test]
fn the_change_of_a_miniscript_wallet_is_verified_while_it_is_in_use() {
    let c = Compute::unchecked();
    let psbt = Psbt::parse_base64(&c, LIANA_PSBT.trim()).expect("the fixture transaction");
    let policy = WalletPolicy::parse_any(&c, LIANA_POLICY).expect("the fixture wallet");
    let keys = [KeyRef::from_master(&c, &master(ABANDON), 0).expect("accounts")];

    let wallets = [policy];
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
    assert_eq!(
        insp.outputs[1].kind,
        OutputKind::WalletChange {
            wallet: 0,
            change: true,
            index: 0,
        }
    );
    assert_eq!(insp.change_total.to_sat(), 39_000);

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
    assert!(
        !insp.outputs[1].is_ours(),
        "with no wallet in use the change is a claim, not a fact"
    );
}

/// A Liana spend the first key can make on its own: `pk(A)` is one of
/// the script's two ways, the device signs it, and nothing else is
/// waited for, so the transaction comes out whole.
#[test]
fn the_liana_spend_completes_with_the_first_key_alone() {
    let c = Compute::unchecked();
    let mut psbt = Psbt::parse_base64(&c, LIANA_PSBT.trim()).expect("the fixture transaction");
    let policy = WalletPolicy::parse_any(&c, LIANA_POLICY).expect("the fixture wallet");
    let abandon = master(ABANDON);
    let keys = [KeyRef::from_master(&c, &abandon, 0).expect("accounts")];
    let wallets = [policy];
    let ctx = Context {
        network: NET,
        keys: &keys,
        wallets: &wallets,
        musig_session: None,
    };

    let insp = inspect(&c, &psbt, &ctx);
    assert_eq!(
        insp.inputs[0].script_type,
        ScriptKind::Miniscript {
            wrapper: Wrapper::P2wsh
        }
    );
    assert_eq!(
        insp.inputs[0].spend_route,
        Some(SpendRoute::Script(path(&[0], &[]))),
        "the sequence arms no wait, so the first key's path is the one it takes"
    );

    let result = sign(
        &c,
        &mut psbt,
        &[&abandon],
        &[abandon.fingerprint()],
        &ctx,
        false,
        Nonce::LowR,
        Aux::Deterministic,
        &mut None,
        &faraday_crypto::Zeroizing::new([0u8; 32]),
    )
    .expect("the input is ours to sign");
    assert_eq!(result.signed_inputs.len(), 1);
    assert_eq!(result.signed_inputs[0].kind, SigKind::Ecdsa);
    assert!(result.complete);

    let tx = finalize(&mut psbt)
        .expect("a satisfied script")
        .expect("every input final");
    assert_eq!(tx.input[0].witness.len(), 2, "the signature and the script");
}

/// The recovery spend is the same wallet down its other path. The
/// coordinator states only the recovery key's origin, so the first key
/// has nothing to sign and the transaction stays partial until the
/// recovery key is loaded.
#[test]
fn the_liana_recovery_spend_waits_for_the_recovery_key() {
    let c = Compute::unchecked();
    let policy = WalletPolicy::parse_any(&c, LIANA_POLICY).expect("the fixture wallet");
    let (abandon, zoo) = (master(ABANDON), master(ZOO));
    let wallets = [policy];

    let mut psbt =
        Psbt::parse_base64(&c, LIANA_RECOVERY_PSBT.trim()).expect("the fixture transaction");
    let alone = [KeyRef::from_master(&c, &abandon, 0).expect("accounts")];
    let ctx = Context {
        network: NET,
        keys: &alone,
        wallets: &wallets,
        musig_session: None,
    };
    let insp = inspect(&c, &psbt, &ctx);
    assert_eq!(
        insp.inputs[0].spend_route,
        Some(SpendRoute::Script(path(&[1], &[Lock::Blocks(52_560)]))),
        "the only key the transaction names is the recovery key"
    );
    assert!(
        insp.warnings
            .iter()
            .all(|w| !matches!(w.kind, WarningKind::TimelockNotMet { .. })),
        "the sequence is the one the path wants: {:?}",
        insp.warnings
    );
    // No key of ours is in the path, which is the danger the review
    // raises; signing past it writes nothing and leaves the PSBT whole.
    let result = sign(
        &c,
        &mut psbt,
        &[&abandon],
        &[abandon.fingerprint()],
        &ctx,
        true,
        Nonce::LowR,
        Aux::Deterministic,
        &mut None,
        &faraday_crypto::Zeroizing::new([0u8; 32]),
    )
    .expect("nothing to sign is not a failure");
    assert!(result.signed_inputs.is_empty());
    assert!(!result.complete);

    let both = [
        KeyRef::from_master(&c, &abandon, 0).expect("accounts"),
        KeyRef::from_master(&c, &zoo, 0).expect("accounts"),
    ];
    let ctx = Context {
        network: NET,
        keys: &both,
        wallets: &wallets,
        musig_session: None,
    };
    let result = sign(
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
    .expect("the recovery key is ours");
    assert_eq!(result.signed_inputs.len(), 1);
    assert!(result.complete);
    let tx = finalize(&mut psbt)
        .expect("a satisfied script")
        .expect("every input final");
    assert_eq!(tx.input[0].sequence.to_consensus_u32(), 52_560);
}

/// The same recovery spend with a sequence below the wait the path
/// imposes: the review cautions that the timelock is not met, and the
/// recovery key's signature does not finish it, because the chain would
/// refuse the transaction.
#[test]
fn a_recovery_spend_below_its_timelock_stays_partial() {
    let c = Compute::unchecked();
    let policy = WalletPolicy::parse_any(&c, LIANA_POLICY).expect("the fixture wallet");
    let (abandon, zoo) = (master(ABANDON), master(ZOO));
    let wallets = [policy];
    let mut psbt =
        Psbt::parse_base64(&c, LIANA_RECOVERY_PSBT.trim()).expect("the fixture transaction");
    psbt.inner_mut().unsigned_tx.input[0].sequence = bitcoin::Sequence::from_height(52_559);

    let keys = [
        KeyRef::from_master(&c, &abandon, 0).expect("accounts"),
        KeyRef::from_master(&c, &zoo, 0).expect("accounts"),
    ];
    let ctx = Context {
        network: NET,
        keys: &keys,
        wallets: &wallets,
        musig_session: None,
    };
    let insp = inspect(&c, &psbt, &ctx);
    assert!(
        insp.warnings.iter().any(|w| w.kind
            == WarningKind::TimelockNotMet {
                path: path(&[1], &[Lock::Blocks(52_560)]),
                lock: Lock::Blocks(52_560),
            }),
        "{:?}",
        insp.warnings
    );

    let result = sign(
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
    .expect("the recovery key is ours");
    assert_eq!(result.signed_inputs.len(), 1);
    assert!(!result.complete, "the wait is not over");
    assert!(finalize(&mut psbt).expect("no error").is_none());
}

/// The tree wallet spent by its key path: one Schnorr signature over the
/// internal key tweaked by the tree, in `tap_key_sig`.
#[test]
fn the_tree_key_path_spend_signs_with_the_internal_key() {
    let c = Compute::unchecked();
    let mut psbt =
        Psbt::parse_base64(&c, TREE_KEYPATH_PSBT.trim()).expect("the fixture transaction");
    let policy = WalletPolicy::parse_any(&c, TREE_POLICY).expect("the fixture wallet");
    let abandon = master(ABANDON);
    let keys = [KeyRef::from_master(&c, &abandon, 0).expect("accounts")];
    let wallets = [policy];
    let ctx = Context {
        network: NET,
        keys: &keys,
        wallets: &wallets,
        musig_session: None,
    };

    let insp = inspect(&c, &psbt, &ctx);
    assert_eq!(insp.inputs[0].script_type, ScriptKind::P2trScript);
    assert_eq!(insp.inputs[0].spend_route, Some(SpendRoute::KeyPath));

    let result = sign(
        &c,
        &mut psbt,
        &[&abandon],
        &[abandon.fingerprint()],
        &ctx,
        false,
        Nonce::LowR,
        Aux::Deterministic,
        &mut None,
        &faraday_crypto::Zeroizing::new([0u8; 32]),
    )
    .expect("the internal key is ours");
    assert_eq!(result.signed_inputs.len(), 1);
    assert_eq!(result.signed_inputs[0].kind, SigKind::Schnorr);
    assert!(psbt.inner().inputs[0].tap_key_sig.is_some());
    assert!(psbt.inner().inputs[0].tap_script_sigs.is_empty());
    assert!(result.complete);

    let tx = finalize(&mut psbt)
        .expect("a satisfied script")
        .expect("every input final");
    assert_eq!(
        tx.input[0].witness.len(),
        1,
        "one signature and nothing else"
    );
}

/// The tree wallet spent through the leaf that wants the third key: the
/// signature goes under that leaf's hash, and the review names the path
/// the leaf is.
#[test]
fn the_tree_leaf_spend_signs_under_the_leaf_hash() {
    let c = Compute::unchecked();
    let mut psbt = Psbt::parse_base64(&c, TREE_PSBT.trim()).expect("the fixture transaction");
    let policy = WalletPolicy::parse_any(&c, TREE_POLICY).expect("the fixture wallet");
    let legal = master(LEGAL);
    let keys = [KeyRef::from_master(&c, &legal, 0).expect("accounts")];
    let wallets = [policy];
    let ctx = Context {
        network: NET,
        keys: &keys,
        wallets: &wallets,
        musig_session: None,
    };

    let insp = inspect(&c, &psbt, &ctx);
    assert_eq!(insp.inputs[0].script_type, ScriptKind::P2trScript);
    assert_eq!(
        insp.inputs[0].spend_route,
        Some(SpendRoute::Script(path(&[2], &[])))
    );

    let leaf = psbt.inner().inputs[0]
        .tap_scripts
        .values()
        .map(|(script, version)| bitcoin::taproot::TapLeafHash::from_script(script, *version))
        .next()
        .expect("the leaf the coordinator chose");

    let result = sign(
        &c,
        &mut psbt,
        &[&legal],
        &[legal.fingerprint()],
        &ctx,
        false,
        Nonce::LowR,
        Aux::Deterministic,
        &mut None,
        &faraday_crypto::Zeroizing::new([0u8; 32]),
    )
    .expect("the leaf key is ours");
    assert_eq!(result.signed_inputs.len(), 1);
    assert_eq!(result.signed_inputs[0].kind, SigKind::Schnorr);
    assert!(psbt.inner().inputs[0].tap_key_sig.is_none());
    assert_eq!(
        psbt.inner().inputs[0]
            .tap_script_sigs
            .keys()
            .map(|(_, l)| *l)
            .collect::<Vec<_>>(),
        vec![leaf]
    );
    assert!(result.complete);

    let tx = finalize(&mut psbt)
        .expect("a satisfied script")
        .expect("every input final");
    assert_eq!(
        tx.input[0].witness.len(),
        3,
        "signature, leaf script and control block"
    );
}

/// A taproot leaf that checks its key by hash, `and_v(v:pkh(K),older(N))`,
/// with the delay waited out and the device holding `K`: the device signs
/// the leaf and leaves the input for the coordinator to finalize, since
/// `miniscript`'s finalizer cannot write a taproot `pkh` (it looks for the
/// key among the ECDSA origins and stops the program when it finds none).
#[test]
fn a_taproot_pkh_leaf_is_signed_and_left_to_finalize() {
    use bitcoin::bip32::DerivationPath;
    use bitcoin::hashes::Hash;
    use bitcoin::secp256k1::{Secp256k1, XOnlyPublicKey};
    use bitcoin::taproot::{LeafVersion, TapLeafHash, TaprootBuilder};
    use bitcoin::{Amount, OutPoint, ScriptBuf, Sequence, Transaction, TxIn, TxOut, absolute};
    use faraday_bip::miniscript::{Miniscript, Tap};
    use std::str::FromStr;

    let c = Compute::unchecked();
    let abandon = master(ABANDON);
    let path = DerivationPath::from_str("m/86'/1'/0'/0/0").unwrap();
    let x = abandon
        .derive(&c, &path)
        .to_xpub(&c)
        .public_key
        .x_only_public_key()
        .0;
    let leaf = Miniscript::<XOnlyPublicKey, Tap>::from_str(&format!("and_v(v:pkh({x}),older(10))"))
        .unwrap()
        .encode();
    let secp = Secp256k1::verification_only();
    let internal = XOnlyPublicKey::from_slice(&faraday_bip::tapmulti::NUMS).unwrap();
    let info = TaprootBuilder::new()
        .add_leaf(0, leaf.clone())
        .unwrap()
        .finalize(&secp, internal)
        .unwrap();
    let control = info
        .control_block(&(leaf.clone(), LeafVersion::TapScript))
        .unwrap();
    let tx = Transaction {
        version: bitcoin::transaction::Version::TWO,
        lock_time: absolute::LockTime::ZERO,
        input: vec![TxIn {
            previous_output: OutPoint::new(bitcoin::Txid::from_byte_array([7; 32]), 0),
            sequence: Sequence::from_height(10),
            ..TxIn::default()
        }],
        output: vec![TxOut {
            value: Amount::from_sat(90_000),
            script_pubkey: ScriptBuf::new_p2wpkh(&bitcoin::WPubkeyHash::from_byte_array([1; 20])),
        }],
    };
    let mut inner = bitcoin::Psbt::from_unsigned_tx(tx).unwrap();
    let leaf_hash = TapLeafHash::from_script(&leaf, LeafVersion::TapScript);
    let input = &mut inner.inputs[0];
    input.witness_utxo = Some(TxOut {
        value: Amount::from_sat(100_000),
        script_pubkey: ScriptBuf::new_p2tr_tweaked(info.output_key()),
    });
    input.tap_internal_key = Some(internal);
    input.tap_merkle_root = info.merkle_root();
    input
        .tap_scripts
        .insert(control, (leaf.clone(), LeafVersion::TapScript));
    input
        .tap_key_origins
        .insert(x, (vec![leaf_hash], (abandon.fingerprint().into(), path)));
    let mut psbt = Psbt::parse_bytes(&c, &inner.serialize()).unwrap();
    let keys = [KeyRef::from_master(&c, &abandon, 0).unwrap()];
    let ctx = Context {
        network: NET,
        keys: &keys,
        wallets: &[],
        musig_session: None,
    };
    let result = sign(
        &c,
        &mut psbt,
        &[&abandon],
        &[abandon.fingerprint()],
        &ctx,
        true,
        Nonce::LowR,
        Aux::Deterministic,
        &mut None,
        &faraday_crypto::Zeroizing::new([0u8; 32]),
    )
    .expect("the leaf is signed");
    assert_eq!(result.signed_inputs.len(), 1);
    assert_eq!(result.signed_inputs[0].kind, SigKind::Schnorr);
    assert!(!result.complete);
    assert!(
        psbt.inner().inputs[0]
            .tap_script_sigs
            .contains_key(&(x, leaf_hash))
    );
    assert_eq!(finalize(&mut psbt).expect("nothing wrong"), None);
    assert!(psbt.inner().inputs[0].final_script_witness.is_none());
}

/// The Liana recovery spend's review with the given transaction
/// version and the recovery key loaded beside the first.
fn recovery_review(version: i32) -> faraday_psbt::Inspection {
    let c = Compute::unchecked();
    let policy = WalletPolicy::parse_any(&c, LIANA_POLICY).expect("the fixture wallet");
    let keys = [
        KeyRef::from_master(&c, &master(ABANDON), 0).expect("accounts"),
        KeyRef::from_master(&c, &master(ZOO), 0).expect("accounts"),
    ];
    let wallets = [policy];
    let mut psbt =
        Psbt::parse_base64(&c, LIANA_RECOVERY_PSBT.trim()).expect("the fixture transaction");
    psbt.inner_mut().unsigned_tx.version = bitcoin::transaction::Version(version);
    let ctx = Context {
        network: NET,
        keys: &keys,
        wallets: &wallets,
        musig_session: None,
    };
    inspect(&c, &psbt, &ctx)
}

/// A relative wait is a rule of version 2 transactions (BIP 68): at
/// version 1 the chain reads no wait from the sequence, so the
/// recovery path's `older` is not met whatever the sequence says.
#[test]
fn a_relative_wait_is_not_met_by_a_version_one_transaction() {
    let not_met = |insp: &faraday_psbt::Inspection| {
        insp.warnings.iter().any(|w| {
            w.kind
                == WarningKind::TimelockNotMet {
                    path: path(&[1], &[Lock::Blocks(52_560)]),
                    lock: Lock::Blocks(52_560),
                }
        })
    };
    let one = recovery_review(1);
    assert!(not_met(&one), "{:?}", one.warnings);
    let two = recovery_review(2);
    assert!(!not_met(&two), "{:?}", two.warnings);

    // Signed by the recovery key, the version 1 spend stays partial: the
    // chain would refuse it.
    let c = Compute::unchecked();
    let policy = WalletPolicy::parse_any(&c, LIANA_POLICY).expect("the fixture wallet");
    let (abandon, zoo) = (master(ABANDON), master(ZOO));
    let keys = [
        KeyRef::from_master(&c, &abandon, 0).expect("accounts"),
        KeyRef::from_master(&c, &zoo, 0).expect("accounts"),
    ];
    let wallets = [policy];
    let ctx = Context {
        network: NET,
        keys: &keys,
        wallets: &wallets,
        musig_session: None,
    };
    let mut psbt =
        Psbt::parse_base64(&c, LIANA_RECOVERY_PSBT.trim()).expect("the fixture transaction");
    psbt.inner_mut().unsigned_tx.version = bitcoin::transaction::Version::ONE;
    let result = sign(
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
    .expect("the recovery key is ours");
    assert_eq!(result.signed_inputs.len(), 1);
    assert!(!result.complete, "version 1 arms no relative wait");
    assert!(finalize(&mut psbt).expect("no error").is_none());
}

/// The Liana wallet with its recovery path behind `after(height)`
/// rather than `older`, and its recovery spend: the fixture's input
/// with the script that policy derives at the same index, the
/// locktime `height`, and `sequence` on the input.
fn absolute_recovery(height: u32, sequence: u32) -> (WalletPolicy, Psbt) {
    use miniscript::descriptor::{Descriptor, DescriptorPublicKey};
    let c = Compute::unchecked();
    let secp = bitcoin::secp256k1::Secp256k1::verification_only();
    let text = LIANA_POLICY.replace("older(52560)", &format!("after({height})"));
    let policy = WalletPolicy::parse_any(&c, &text).expect("the wallet with an absolute wait");
    let mut psbt =
        Psbt::parse_base64(&c, LIANA_RECOVERY_PSBT.trim()).expect("the fixture transaction");
    let inner = psbt.inner_mut();
    let (fp, origin) = inner.inputs[0]
        .bip32_derivation
        .values()
        .next()
        .cloned()
        .expect("the recovery key's origin");
    let (change, index) = policy
        .leaf_of(fp.into(), &origin)
        .expect("the origin is the wallet's");
    let descriptor: Descriptor<DescriptorPublicKey> = policy
        .to_descriptor()
        .parse()
        .expect("the wallet's descriptor");
    let chain = descriptor
        .into_single_descriptors()
        .expect("two chains")
        .remove(usize::from(change));
    let witness_script = chain
        .at_derivation_index(index)
        .expect("an unhardened index")
        .derived_descriptor(&secp)
        .expect("derived keys")
        .explicit_script()
        .expect("a wsh script");
    let script_pubkey = witness_script.to_p2wsh();
    assert_eq!(
        policy
            .script_at(&c, change, index)
            .expect("the wallet's script"),
        script_pubkey
    );
    inner.inputs[0].witness_script = Some(witness_script);
    inner.inputs[0].non_witness_utxo = None;
    inner.inputs[0]
        .witness_utxo
        .as_mut()
        .expect("the spent output")
        .script_pubkey = script_pubkey;
    inner.unsigned_tx.version = bitcoin::transaction::Version::TWO;
    inner.unsigned_tx.lock_time =
        bitcoin::absolute::LockTime::from_height(height).expect("a height");
    inner.unsigned_tx.input[0].sequence = bitcoin::Sequence(sequence);
    (policy, psbt)
}

/// An absolute wait is met only when the input's sequence leaves the
/// locktime in force: with the final sequence, 0xFFFFFFFF, the chain
/// ignores the locktime and `after` fails, however late the locktime.
#[test]
fn an_absolute_wait_is_not_met_with_a_final_sequence() {
    let c = Compute::unchecked();
    let keys = [
        KeyRef::from_master(&c, &master(ABANDON), 0).expect("accounts"),
        KeyRef::from_master(&c, &master(ZOO), 0).expect("accounts"),
    ];
    let height = 900_000;
    let not_met = |sequence: u32| {
        let (policy, psbt) = absolute_recovery(height, sequence);
        let wallets = [policy];
        let ctx = Context {
            network: NET,
            keys: &keys,
            wallets: &wallets,
            musig_session: None,
        };
        let insp = inspect(&c, &psbt, &ctx);
        assert_eq!(
            insp.inputs[0].spend_route,
            Some(SpendRoute::Script(path(&[1], &[Lock::Height(height)]))),
            "{:?}",
            insp.warnings
        );
        insp.warnings.iter().any(|w| {
            w.kind
                == WarningKind::TimelockNotMet {
                    path: path(&[1], &[Lock::Height(height)]),
                    lock: Lock::Height(height),
                }
        })
    };
    assert!(
        not_met(0xFFFF_FFFF),
        "the final sequence disables the locktime"
    );
    assert!(!not_met(0xFFFF_FFFE), "a sequence below final keeps it");
}

/// Where a spend could take either path, a version 1 transaction arms
/// no relative wait, so the review names the path with no wait as the
/// one it spends, not the recovery path that version cannot meet.
#[test]
fn a_version_one_spend_does_not_take_a_path_its_version_cannot_meet() {
    let c = Compute::unchecked();
    let policy = WalletPolicy::parse_any(&c, LIANA_POLICY).expect("the fixture wallet");
    let keys = [
        KeyRef::from_master(&c, &master(ABANDON), 0).expect("accounts"),
        KeyRef::from_master(&c, &master(ZOO), 0).expect("accounts"),
    ];
    let wallets = [policy];
    let ctx = Context {
        network: NET,
        keys: &keys,
        wallets: &wallets,
        musig_session: None,
    };
    let route = |version: i32| {
        let mut psbt = Psbt::parse_base64(&c, LIANA_PSBT.trim()).expect("the fixture transaction");
        let inner = psbt.inner_mut();
        inner.unsigned_tx.version = bitcoin::transaction::Version(version);
        inner.unsigned_tx.input[0].sequence = bitcoin::Sequence::from_height(52_560);
        inspect(&c, &psbt, &ctx).inputs[0].spend_route.clone()
    };
    assert_eq!(
        route(2),
        Some(SpendRoute::Script(path(&[1], &[Lock::Blocks(52_560)]))),
        "at version 2 the sequence arms the recovery path"
    );
    assert_eq!(
        route(1),
        Some(SpendRoute::Script(path(&[0], &[]))),
        "at version 1 it arms nothing"
    );
}
