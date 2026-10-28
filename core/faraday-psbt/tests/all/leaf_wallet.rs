//! A taproot leaf is signed only through a registered wallet whose script
//! it is (F135, F141): a single-key leaf that no registered wallet gives
//! is left unsigned and named, beside an input that is signed; alone,
//! nothing here takes part; with its wallet registered, it is signed.

use std::str::FromStr;

use bitcoin::bip32::DerivationPath;
use bitcoin::hashes::Hash;
use bitcoin::secp256k1::{Secp256k1, XOnlyPublicKey};
use bitcoin::taproot::{LeafVersion, TapLeafHash, TaprootBuilder};
use bitcoin::{Amount, OutPoint, ScriptBuf, Transaction, TxIn, TxOut, absolute};
use faraday_bip::keys::{MasterKey, Network};
use faraday_bip::miniscript::{Miniscript, Tap};
use faraday_bip::policy::WalletPolicy;
use faraday_crypto::work::Compute;
use faraday_psbt::{
    Aux, Context, Error, Inspection, KeyRef, Level, Nonce, Psbt, SignResult, WarningKind, inspect,
    sign,
};

const ABANDON: &str =
    "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
const NET: Network = Network::Regtest;
const VALUE: u64 = 100_000;

fn master() -> MasterKey {
    let c = Compute::unchecked();
    let m = faraday_bip::bip39::Mnemonic::parse(&c, faraday_bip::bip39::Language::English, ABANDON)
        .expect("fixed mnemonic");
    MasterKey::from_seed(&c, &m.to_seed(&c, b"").expect("empty passphrase"), NET)
}

fn xonly(m: &MasterKey, path: &str) -> XOnlyPublicKey {
    let c = Compute::unchecked();
    m.derive(&c, &DerivationPath::from_str(path).unwrap())
        .to_xpub(&c)
        .public_key
        .x_only_public_key()
        .0
}

/// The key the leaf checks, the leaf's hash and the PSBT: when
/// `key_path`, input 0 is a BIP-86 key-path spend of the loaded key and
/// the leaf input is input 1; otherwise the leaf input alone. The leaf is
/// `pk(K)` under BIP 341's unspendable internal key, `K` the loaded key
/// at `m/86'/1'/0'/0/1`.
fn spend(m: &MasterKey, key_path: bool) -> (XOnlyPublicKey, TapLeafHash, Psbt) {
    let c = Compute::unchecked();
    let secp = Secp256k1::verification_only();
    let fp = m.fingerprint().into();
    let mut inputs = Vec::new();
    if key_path {
        let path = "m/86'/1'/0'/0/0";
        let x = xonly(m, path);
        let input = bitcoin::psbt::Input {
            witness_utxo: Some(TxOut {
                value: Amount::from_sat(VALUE),
                script_pubkey: ScriptBuf::new_p2tr(&secp, x, None),
            }),
            tap_internal_key: Some(x),
            tap_key_origins: [(x, (vec![], (fp, DerivationPath::from_str(path).unwrap())))]
                .into_iter()
                .collect(),
            ..Default::default()
        };
        inputs.push(input);
    }
    let path = "m/86'/1'/0'/0/1";
    let x = xonly(m, path);
    let leaf = Miniscript::<XOnlyPublicKey, Tap>::from_str(&format!("pk({x})"))
        .unwrap()
        .encode();
    let internal = XOnlyPublicKey::from_slice(&faraday_bip::tapmulti::NUMS).unwrap();
    let info = TaprootBuilder::new()
        .add_leaf(0, leaf.clone())
        .unwrap()
        .finalize(&secp, internal)
        .unwrap();
    let control = info
        .control_block(&(leaf.clone(), LeafVersion::TapScript))
        .unwrap();
    let leaf_hash = TapLeafHash::from_script(&leaf, LeafVersion::TapScript);
    let mut input = bitcoin::psbt::Input {
        witness_utxo: Some(TxOut {
            value: Amount::from_sat(VALUE),
            script_pubkey: ScriptBuf::new_p2tr_tweaked(info.output_key()),
        }),
        tap_internal_key: Some(internal),
        tap_merkle_root: info.merkle_root(),
        ..Default::default()
    };
    input
        .tap_scripts
        .insert(control, (leaf, LeafVersion::TapScript));
    input.tap_key_origins.insert(
        x,
        (
            vec![leaf_hash],
            (fp, DerivationPath::from_str(path).unwrap()),
        ),
    );
    inputs.push(input);

    let tx = Transaction {
        version: bitcoin::transaction::Version::TWO,
        lock_time: absolute::LockTime::ZERO,
        input: (0..inputs.len())
            .map(|i| TxIn {
                previous_output: OutPoint::new(bitcoin::Txid::from_byte_array([7; 32]), i as u32),
                ..TxIn::default()
            })
            .collect(),
        output: vec![TxOut {
            value: Amount::from_sat(VALUE * inputs.len() as u64 - 2_000),
            script_pubkey: ScriptBuf::new_p2wpkh(&bitcoin::WPubkeyHash::from_byte_array([1; 20])),
        }],
    };
    let mut inner = bitcoin::Psbt::from_unsigned_tx(tx).unwrap();
    inner.inputs = inputs;
    (
        x,
        leaf_hash,
        Psbt::parse_bytes(&c, &inner.serialize()).unwrap(),
    )
}

/// The leaf's wallet: `tr(NUMS, pk(K))` over the loaded key's BIP-86
/// account, which gives the leaf at chain 0, index 1.
fn leaf_wallet(m: &MasterKey) -> WalletPolicy {
    let c = Compute::unchecked();
    let xpub = m
        .derive(&c, &DerivationPath::from_str("m/86'/1'/0'").unwrap())
        .to_xpub(&c);
    let nums = XOnlyPublicKey::from_slice(&faraday_bip::tapmulti::NUMS).unwrap();
    let fp = m.fingerprint();
    WalletPolicy::parse_any(
        &c,
        &format!("tr({nums},pk([{fp}/86'/1'/0']{xpub}/<0;1>/*))"),
    )
    .expect("the leaf's wallet")
}

/// Inspects `psbt` and signs it with the loaded key, `wallets`
/// registered.
fn review_and_sign(
    m: &MasterKey,
    psbt: &mut Psbt,
    wallets: &[WalletPolicy],
) -> (Inspection, Result<SignResult, Error>) {
    let c = Compute::unchecked();
    let keys = [KeyRef::from_master(&c, m, 0).expect("accounts")];
    let ctx = Context {
        network: NET,
        keys: &keys,
        wallets,
        musig_session: None,
    };
    let insp = inspect(&c, psbt, &ctx);
    let result = sign(
        &c,
        psbt,
        &[m],
        &[m.fingerprint()],
        &ctx,
        &[],
        Nonce::LowR,
        Aux::Deterministic,
        &mut None,
        &faraday_crypto::Zeroizing::new([0u8; 32]),
    );
    (insp, result)
}

fn uncovered(insp: &Inspection, input: usize) -> bool {
    insp.warnings.iter().any(|w| {
        w.level == Level::Caution
            && w.kind
                == WarningKind::NoWalletForScript {
                    input,
                    taproot: true,
                }
    })
}

/// Beside a key-path input of the person's own, the leaf no registered
/// wallet gives is left unsigned: the key-path input is signed, and the
/// review names the leaf's input as left unsigned rather than refusing
/// the transaction.
#[test]
fn a_leaf_without_its_wallet_is_left_unsigned_beside_a_signed_input() {
    let m = master();
    let (_, _, mut psbt) = spend(&m, true);
    let (insp, result) = review_and_sign(&m, &mut psbt, &[]);
    assert!(uncovered(&insp, 1), "{:?}", insp.warnings);
    assert!(!insp.refuses(), "{:?}", insp.warnings);
    assert!(insp.inputs[0].is_ours);
    assert!(!insp.inputs[1].is_ours);
    let result = result.expect("the key-path input is signed");
    assert_eq!(
        result
            .signed_inputs
            .iter()
            .map(|s| s.index)
            .collect::<Vec<_>>(),
        vec![0]
    );
    assert!(!result.complete);
    let inputs = &psbt.inner().inputs;
    assert!(inputs[0].tap_key_sig.is_some());
    assert!(inputs[1].tap_script_sigs.is_empty());
    assert!(inputs[1].tap_key_sig.is_none());
}

/// The same leaf alone: no input takes part, and signing refuses with
/// nothing written.
#[test]
fn a_leaf_without_its_wallet_alone_is_refused() {
    let m = master();
    let (_, _, mut psbt) = spend(&m, false);
    let before = psbt.to_bytes();
    let (insp, result) = review_and_sign(&m, &mut psbt, &[]);
    assert!(uncovered(&insp, 0), "{:?}", insp.warnings);
    assert_eq!(
        result,
        Err(Error::Danger(vec![WarningKind::NoParticipatingKey]))
    );
    assert_eq!(psbt.to_bytes(), before);
}

/// With the leaf's wallet registered, the leaf is the person's and is
/// signed under its hash, beside the key-path input.
#[test]
fn a_leaf_with_its_wallet_registered_is_signed() {
    let m = master();
    let wallets = [leaf_wallet(&m)];
    let (x, leaf_hash, mut psbt) = spend(&m, false);
    let (insp, result) = review_and_sign(&m, &mut psbt, &wallets);
    assert!(!uncovered(&insp, 0), "{:?}", insp.warnings);
    assert!(insp.inputs[0].is_ours);
    let result = result.expect("the leaf is signed");
    assert_eq!(result.signed_inputs.len(), 1);
    assert!(
        psbt.inner().inputs[0]
            .tap_script_sigs
            .contains_key(&(x, leaf_hash))
    );

    let (x, leaf_hash, mut psbt) = spend(&m, true);
    let (insp, result) = review_and_sign(&m, &mut psbt, &wallets);
    assert!(!uncovered(&insp, 1), "{:?}", insp.warnings);
    let result = result.expect("both inputs are signed");
    assert_eq!(
        result
            .signed_inputs
            .iter()
            .map(|s| s.index)
            .collect::<Vec<_>>(),
        vec![0, 1]
    );
    assert!(result.complete);
    assert!(
        psbt.inner().inputs[1]
            .tap_script_sigs
            .contains_key(&(x, leaf_hash))
    );
}
