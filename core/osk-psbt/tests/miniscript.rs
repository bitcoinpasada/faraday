//! A spend of a miniscript wallet: whose the change is, and what the
//! device will and will not do with the input.

use osk_bip::keys::{MasterKey, Network};
use osk_bip::policy::WalletPolicy;
use osk_bip::spend::{Lock, SpendPath};
use osk_psbt::{
    Aux, Context, KeyRef, Nonce, OutputKind, Psbt, ScriptKind, SigKind, SpendRoute, WarningKind,
    Wrapper, finalize, inspect, sign,
};

const ABANDON: &str =
    "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
const ZOO: &str = "zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo wrong";
const LEGAL: &str = "legal winner thank year wave sausage worth useful legal winner thank yellow";
const NET: Network = Network::Regtest;

const LIANA_POLICY: &str = include_str!("../../../tools/vectors/psbt/wallet-liana.policy");
const LIANA_PSBT: &str = include_str!("../../../tools/vectors/psbt/wallet-liana.psbt");
const LIANA_RECOVERY_PSBT: &str =
    include_str!("../../../tools/vectors/psbt/wallet-liana-recovery.psbt");
const TREE_POLICY: &str = include_str!("../../../tools/vectors/psbt/wallet-tree.policy");
const TREE_PSBT: &str = include_str!("../../../tools/vectors/psbt/wallet-tree.psbt");
const TREE_KEYPATH_PSBT: &str =
    include_str!("../../../tools/vectors/psbt/wallet-tree-keypath.psbt");

fn path(keys: &[usize], locks: &[Lock]) -> SpendPath {
    SpendPath {
        keys: keys.to_vec(),
        locks: locks.to_vec(),
    }
}

fn master(words: &str) -> MasterKey {
    let m = osk_bip::bip39::Mnemonic::parse(osk_bip::bip39::Language::English, words)
        .expect("fixed mnemonic");
    MasterKey::from_seed(&m.to_seed(b"").expect("empty passphrase"), NET)
}

/// The change of a miniscript wallet is verified the same way a
/// multisig wallet's is: by re-deriving the wallet's own script at the
/// index the key origins name. With the wallet not in use there is
/// nothing to re-derive from, so the same output is only claimed.
#[test]
fn the_change_of_a_miniscript_wallet_is_verified_while_it_is_in_use() {
    let psbt = Psbt::parse_base64(LIANA_PSBT.trim()).expect("the fixture transaction");
    let policy = WalletPolicy::parse_any(LIANA_POLICY).expect("the fixture wallet");
    let keys = [KeyRef::from_master(&master(ABANDON), 0).expect("accounts")];

    let wallets = [policy];
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
    let mut psbt = Psbt::parse_base64(LIANA_PSBT.trim()).expect("the fixture transaction");
    let policy = WalletPolicy::parse_any(LIANA_POLICY).expect("the fixture wallet");
    let abandon = master(ABANDON);
    let keys = [KeyRef::from_master(&abandon, 0).expect("accounts")];
    let wallets = [policy];
    let ctx = Context {
        network: NET,
        keys: &keys,
        wallets: &wallets,
        musig_session: None,
        shares: &[],
        carry: None,
    };

    let insp = inspect(&psbt, &ctx);
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
    let policy = WalletPolicy::parse_any(LIANA_POLICY).expect("the fixture wallet");
    let (abandon, zoo) = (master(ABANDON), master(ZOO));
    let wallets = [policy];

    let mut psbt = Psbt::parse_base64(LIANA_RECOVERY_PSBT.trim()).expect("the fixture transaction");
    let alone = [KeyRef::from_master(&abandon, 0).expect("accounts")];
    let ctx = Context {
        network: NET,
        keys: &alone,
        wallets: &wallets,
        musig_session: None,
        shares: &[],
        carry: None,
    };
    let insp = inspect(&psbt, &ctx);
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
    .expect("nothing to sign is not a failure");
    assert!(result.signed_inputs.is_empty());
    assert!(!result.complete);

    let both = [
        KeyRef::from_master(&abandon, 0).expect("accounts"),
        KeyRef::from_master(&zoo, 0).expect("accounts"),
    ];
    let ctx = Context {
        network: NET,
        keys: &both,
        wallets: &wallets,
        musig_session: None,
        shares: &[],
        carry: None,
    };
    let result = sign(
        &mut psbt,
        &[&abandon, &zoo],
        &[abandon.fingerprint(), zoo.fingerprint()],
        &ctx,
        false,
        Nonce::LowR,
        Aux::Deterministic,
        &mut None,
        [0u8; 32],
        &[],
        &[],
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
    let policy = WalletPolicy::parse_any(LIANA_POLICY).expect("the fixture wallet");
    let (abandon, zoo) = (master(ABANDON), master(ZOO));
    let wallets = [policy];
    let mut psbt = Psbt::parse_base64(LIANA_RECOVERY_PSBT.trim()).expect("the fixture transaction");
    psbt.inner_mut().unsigned_tx.input[0].sequence = bitcoin::Sequence::from_height(52_559);

    let keys = [
        KeyRef::from_master(&abandon, 0).expect("accounts"),
        KeyRef::from_master(&zoo, 0).expect("accounts"),
    ];
    let ctx = Context {
        network: NET,
        keys: &keys,
        wallets: &wallets,
        musig_session: None,
        shares: &[],
        carry: None,
    };
    let insp = inspect(&psbt, &ctx);
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
        &mut psbt,
        &[&abandon, &zoo],
        &[abandon.fingerprint(), zoo.fingerprint()],
        &ctx,
        false,
        Nonce::LowR,
        Aux::Deterministic,
        &mut None,
        [0u8; 32],
        &[],
        &[],
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
    let mut psbt = Psbt::parse_base64(TREE_KEYPATH_PSBT.trim()).expect("the fixture transaction");
    let policy = WalletPolicy::parse_any(TREE_POLICY).expect("the fixture wallet");
    let abandon = master(ABANDON);
    let keys = [KeyRef::from_master(&abandon, 0).expect("accounts")];
    let wallets = [policy];
    let ctx = Context {
        network: NET,
        keys: &keys,
        wallets: &wallets,
        musig_session: None,
        shares: &[],
        carry: None,
    };

    let insp = inspect(&psbt, &ctx);
    assert_eq!(insp.inputs[0].script_type, ScriptKind::P2trScript);
    assert_eq!(insp.inputs[0].spend_route, Some(SpendRoute::KeyPath));

    let result = sign(
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
    let mut psbt = Psbt::parse_base64(TREE_PSBT.trim()).expect("the fixture transaction");
    let policy = WalletPolicy::parse_any(TREE_POLICY).expect("the fixture wallet");
    let legal = master(LEGAL);
    let keys = [KeyRef::from_master(&legal, 0).expect("accounts")];
    let wallets = [policy];
    let ctx = Context {
        network: NET,
        keys: &keys,
        wallets: &wallets,
        musig_session: None,
        shares: &[],
        carry: None,
    };

    let insp = inspect(&psbt, &ctx);
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
        &mut psbt,
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
