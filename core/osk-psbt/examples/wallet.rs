//! Writes the registered-wallet fixtures into a directory: BIP-388
//! policies and a transaction of each wallet with change of its own.
//!
//! ```text
//! cargo run -p osk-psbt --example wallet -- tools/vectors/psbt
//! ```
//!
//! Everything is fixed, as in `examples/warnings.rs`: the "abandon …
//! about" seed with "zoo … wrong" and "legal … yellow" as its two
//! cosigners, on regtest, and one fake funding output. `tests/fixtures.rs`
//! checks the committed files against this code.
#![allow(dead_code)]

use std::process::ExitCode;

use osk_bip::keys::{MasterKey, Network, ScriptType};
use osk_bip::policy::WalletPolicy;
use osk_psbt::Psbt;

#[path = "../tests/common/builder.rs"]
mod builder;

use builder::{bip48_path, funding_tx, multisig_policy, sorted_multisig_change};

use bitcoin::absolute::LockTime;
use bitcoin::bip32::ChildNumber;
use bitcoin::transaction::Version;
use bitcoin::{Amount, OutPoint, ScriptBuf, Sequence, Transaction, TxIn, TxOut, Witness};
use osk_bip::keys::Fingerprint;
use osk_bip::miniscript::descriptor::{DefiniteDescriptorKey, Descriptor, DescriptorPublicKey};
use osk_bip::miniscript::psbt::PsbtExt;

const ABANDON: &str =
    "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
const ZOO: &str = "zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo wrong";
const LEGAL: &str = "legal winner thank year wave sausage worth useful legal winner thank yellow";

const NET: Network = Network::Regtest;

/// The policy file's name.
pub const POLICY_NAME: &str = "wallet-2of3.policy";
/// The transaction of that wallet.
pub const PSBT_NAME: &str = "wallet-2of3.psbt";
/// The two cosigners that are not the loaded key, each on its own, as a
/// coordinator hands one over: what the wallet builder's scanner reads.
pub const COSIGNER_NAMES: [&str; 2] = ["wallet-2of3-cosigner-1.txt", "wallet-2of3-cosigner-2.txt"];
/// The Liana-shaped miniscript wallet: one key now, a second key after a
/// year.
pub const LIANA_POLICY_NAME: &str = "wallet-liana.policy";
/// A spend of that wallet.
pub const LIANA_PSBT_NAME: &str = "wallet-liana.psbt";
/// The taproot tree wallet: a key path and two leaves.
pub const TREE_POLICY_NAME: &str = "wallet-tree.policy";
/// A spend of the Liana wallet down its recovery path.
pub const LIANA_RECOVERY_PSBT_NAME: &str = "wallet-liana-recovery.psbt";
/// A spend of the tree wallet through one of its leaves.
pub const TREE_PSBT_NAME: &str = "wallet-tree.psbt";
/// A spend of the tree wallet by its key path.
pub const TREE_KEYPATH_PSBT_NAME: &str = "wallet-tree-keypath.psbt";

/// The template of the Liana-shaped wallet: the first key spends at any
/// time, the second spends alone after 52 560 blocks — about a year.
pub const LIANA_TEMPLATE: &str = "wsh(or_d(pk(@0/**),and_v(v:pkh(@1/**),older(52560))))";
/// The template of the taproot tree wallet: the first key on the key
/// path, then a leaf that wants the second key after 4 320 blocks and a
/// leaf that wants the third key alone.
pub const TREE_TEMPLATE: &str = "tr(@0/**,{and_v(v:pk(@1/**),older(4320)),pk(@2/**)})";

fn master(words: &str) -> MasterKey {
    let m = osk_bip::bip39::Mnemonic::parse(osk_bip::bip39::Language::English, words)
        .expect("fixed mnemonic");
    MasterKey::from_seed(&m.to_seed(b"").expect("empty passphrase"), NET)
}

/// The 2-of-3 wallet: the loaded key and two fixed cosigners.
pub fn policy() -> WalletPolicy {
    let (abandon, zoo, legal) = (master(ABANDON), master(ZOO), master(LEGAL));
    multisig_policy(&[&abandon, &zoo, &legal], 2, NET)
}

/// The Liana-shaped wallet over the loaded key and "zoo … wrong", both
/// at the BIP-48 path a `wsh` script's keys are written at.
pub fn liana_policy() -> WalletPolicy {
    let path = bip48_path(NET);
    let keys: Vec<String> = [master(ABANDON), master(ZOO)]
        .iter()
        .map(|m| {
            format!(
                "[{}/{path:#}]{}",
                m.fingerprint(),
                m.derive(&path).to_xpub()
            )
        })
        .collect();
    let keys: Vec<&str> = keys.iter().map(String::as_str).collect();
    WalletPolicy::from_parts(LIANA_TEMPLATE, &keys).expect("policy built from test keys")
}

/// The taproot tree wallet over all three keys, at the BIP-86 account
/// each of them pays taproot from.
pub fn tree_policy() -> WalletPolicy {
    let keys: Vec<String> = [master(ABANDON), master(ZOO), master(LEGAL)]
        .iter()
        .map(|m| {
            let a = m.account_xpub(ScriptType::Taproot, 0).expect("account");
            let path = a.path();
            format!("[{}/{path:#}]{}", a.master_fingerprint(), a.xpub())
        })
        .collect();
    let keys: Vec<&str> = keys.iter().map(String::as_str).collect();
    WalletPolicy::from_parts(TREE_TEMPLATE, &keys).expect("policy built from test keys")
}

/// A spend of the Liana-shaped wallet: 100 000 sat in, 60 000 to someone
/// else, 39 000 back to its own change address, 1 000 fee.
///
/// The input is the wallet's own receive address with the witness script
/// and both key origins a coordinator fills in, which is what lets the
/// review say whose the input is; signing it is another pass.
pub fn liana_psbt() -> Psbt {
    let policy = liana_policy();
    let theirs = master(ZOO)
        .account_xpub(ScriptType::NativeSegwit, 0)
        .expect("account")
        .address(false, 0)
        .expect("address")
        .script_pubkey();
    let funding = funding_tx(policy.script_at(false, 0).expect("receive script"), 100_000);
    let tx = Transaction {
        version: Version::TWO,
        lock_time: LockTime::ZERO,
        input: vec![TxIn {
            previous_output: OutPoint::new(funding.compute_txid(), 0),
            script_sig: ScriptBuf::new(),
            sequence: Sequence::ENABLE_RBF_NO_LOCKTIME,
            witness: Witness::new(),
        }],
        output: vec![
            TxOut {
                value: Amount::from_sat(60_000),
                script_pubkey: theirs,
            },
            TxOut {
                value: Amount::from_sat(39_000),
                script_pubkey: policy.script_at(true, 0).expect("change script"),
            },
        ],
    };
    let mut psbt = bitcoin::Psbt::from_unsigned_tx(tx).expect("unsigned");
    psbt.inputs[0].witness_utxo = Some(funding.output[0].clone());
    psbt.inputs[0].non_witness_utxo = Some(funding.clone());
    psbt.inputs[0].witness_script = Some(witness_script(&policy, false, 0));
    for (chain, at) in [(false, 0usize), (true, 1usize)] {
        for key in policy.keys() {
            let path = key
                .path()
                .expect("the policy's keys carry origins")
                .clone()
                .extend([
                    ChildNumber::from_normal_idx(u32::from(chain)).unwrap(),
                    ChildNumber::from_normal_idx(0).unwrap(),
                ]);
            let derived = key
                .xpub()
                .derive_pub(
                    &bitcoin::secp256k1::Secp256k1::verification_only(),
                    &[
                        ChildNumber::from_normal_idx(u32::from(chain)).unwrap(),
                        ChildNumber::from_normal_idx(0).unwrap(),
                    ],
                )
                .expect("normal child")
                .public_key;
            let source = (key.fingerprint().expect("an origin").into(), path);
            if chain {
                psbt.outputs[at].bip32_derivation.insert(derived, source);
            } else {
                psbt.inputs[at].bip32_derivation.insert(derived, source);
            }
        }
    }
    Psbt::from(psbt)
}

/// The witness script of a `wsh` wallet at one address, which a
/// coordinator puts in the PSBT so a signer can read the input.
fn witness_script(policy: &WalletPolicy, change: bool, index: u32) -> ScriptBuf {
    let descriptor: Descriptor<DescriptorPublicKey> =
        policy.to_descriptor().parse().expect("a descriptor");
    descriptor.into_single_descriptors().expect("two chains")[usize::from(change)]
        .clone()
        .at_derivation_index(index)
        .expect("index")
        .explicit_script()
        .expect("a wsh script")
}

/// The wallet's descriptor at one address, with no wildcard left in it,
/// which is the form a coordinator fills a PSBT in from.
fn definite(policy: &WalletPolicy, change: bool, index: u32) -> Descriptor<DefiniteDescriptorKey> {
    let descriptor: Descriptor<DescriptorPublicKey> =
        policy.to_descriptor().parse().expect("a descriptor");
    descriptor.into_single_descriptors().expect("two chains")[usize::from(change)]
        .clone()
        .at_derivation_index(index)
        .expect("index")
}

/// A spend of any registered wallet: 100 000 sat in at its first receive
/// address, `theirs` sat out to the "zoo … wrong" key, `change` sat back
/// to the wallet's own first change address, and every script, key
/// origin and leaf a coordinator fills in, from the descriptor itself.
fn wallet_spend(
    policy: &WalletPolicy,
    sequence: Sequence,
    theirs: u64,
    change: u64,
) -> bitcoin::Psbt {
    let spend = definite(policy, false, 0);
    let back = definite(policy, true, 0);
    let funding = funding_tx(spend.script_pubkey(), 100_000);
    let elsewhere = master(ZOO)
        .account_xpub(ScriptType::NativeSegwit, 0)
        .expect("account")
        .address(false, 0)
        .expect("address")
        .script_pubkey();
    let tx = Transaction {
        version: Version::TWO,
        lock_time: LockTime::ZERO,
        input: vec![TxIn {
            previous_output: OutPoint::new(funding.compute_txid(), 0),
            script_sig: ScriptBuf::new(),
            sequence,
            witness: Witness::new(),
        }],
        output: vec![
            TxOut {
                value: Amount::from_sat(theirs),
                script_pubkey: elsewhere,
            },
            TxOut {
                value: Amount::from_sat(change),
                script_pubkey: back.script_pubkey(),
            },
        ],
    };
    let mut psbt = bitcoin::Psbt::from_unsigned_tx(tx).expect("unsigned");
    psbt.inputs[0].witness_utxo = Some(funding.output[0].clone());
    psbt.inputs[0].non_witness_utxo = Some(funding);
    psbt.update_input_with_descriptor(0, &spend)
        .expect("the input is that address");
    psbt.update_output_with_descriptor(1, &back)
        .expect("the change is that address");
    psbt
}

/// The Liana wallet spent down its recovery path: the sequence the
/// script's `older(52560)` wants, and only the recovery key's origin,
/// which is what a coordinator building this spend hands over. Without
/// the first key's origin no signature of it can be asked for, so the
/// recovery key is the only way through.
pub fn liana_recovery_psbt() -> Psbt {
    let policy = liana_policy();
    let now = policy.keys()[0].fingerprint().expect("an origin");
    let mut psbt = liana_psbt().into_inner();
    psbt.unsigned_tx.input[0].sequence = Sequence::from_height(52_560);
    psbt.inputs[0]
        .bip32_derivation
        .retain(|_, (fp, _)| Fingerprint::from(*fp) != now);
    Psbt::from(psbt)
}

/// The tree wallet spent through the leaf that wants the third key
/// alone. A coordinator that has chosen a leaf carries that leaf's
/// script and nothing else, which is what says which way the
/// transaction goes.
pub fn tree_psbt() -> Psbt {
    let mut psbt = wallet_spend(
        &tree_policy(),
        Sequence::ENABLE_RBF_NO_LOCKTIME,
        60_000,
        39_000,
    );
    let policy = tree_policy();
    let leaf_key = policy.keys()[2]
        .xpub()
        .derive_pub(
            &bitcoin::secp256k1::Secp256k1::verification_only(),
            &vec![
                ChildNumber::from_normal_idx(0).unwrap(),
                ChildNumber::from_normal_idx(0).unwrap(),
            ],
        )
        .expect("normal child")
        .public_key
        .x_only_public_key()
        .0;
    let input = &mut psbt.inputs[0];
    input.tap_scripts.retain(|_, (script, _)| {
        script
            .to_bytes()
            .windows(32)
            .any(|w| w == leaf_key.serialize())
    });
    let leaves: Vec<bitcoin::taproot::TapLeafHash> = input
        .tap_scripts
        .values()
        .map(|(script, version)| bitcoin::taproot::TapLeafHash::from_script(script, *version))
        .collect();
    input
        .tap_key_origins
        .retain(|_, (claimed, _)| claimed.iter().any(|l| leaves.contains(l)));
    Psbt::from(psbt)
}

/// The tree wallet spent by its key path: the internal key, the tree's
/// root, and no leaf script at all, which is all a key-path spend needs
/// and all it reveals.
pub fn tree_keypath_psbt() -> Psbt {
    let mut psbt = wallet_spend(
        &tree_policy(),
        Sequence::ENABLE_RBF_NO_LOCKTIME,
        60_000,
        39_000,
    );
    let input = &mut psbt.inputs[0];
    let internal = input.tap_internal_key;
    input.tap_scripts.clear();
    input
        .tap_key_origins
        .retain(|xonly, _| Some(*xonly) == internal);
    for (claimed, _) in input.tap_key_origins.values_mut() {
        claimed.clear();
    }
    Psbt::from(psbt)
}

/// A spend of that wallet: 100 000 sat in, 60 000 to someone else,
/// 39 000 back to the wallet's own change address, 1 000 fee.
pub fn psbt() -> Psbt {
    let (abandon, zoo, legal) = (master(ABANDON), master(ZOO), master(LEGAL));
    let theirs = master(ZOO)
        .account_xpub(ScriptType::NativeSegwit, 0)
        .expect("account")
        .address(false, 0)
        .expect("address")
        .script_pubkey();
    sorted_multisig_change(
        &[&abandon, &zoo, &legal],
        2,
        NET,
        100_000,
        &[(theirs, 60_000)],
        39_000,
    )
}

fn main() -> ExitCode {
    let Some(dir) = std::env::args().nth(1) else {
        eprintln!("usage: wallet <directory>");
        return ExitCode::FAILURE;
    };
    let dir = std::path::PathBuf::from(dir);
    let policy = policy();
    let mut files = vec![
        (POLICY_NAME, format!("{}\n", policy.to_text())),
        (PSBT_NAME, psbt().to_base64()),
        (LIANA_POLICY_NAME, format!("{}\n", liana_policy().to_text())),
        (LIANA_PSBT_NAME, liana_psbt().to_base64()),
        (TREE_POLICY_NAME, format!("{}\n", tree_policy().to_text())),
        (LIANA_RECOVERY_PSBT_NAME, liana_recovery_psbt().to_base64()),
        (TREE_PSBT_NAME, tree_psbt().to_base64()),
        (TREE_KEYPATH_PSBT_NAME, tree_keypath_psbt().to_base64()),
    ];
    for (i, name) in COSIGNER_NAMES.iter().enumerate() {
        files.push((*name, format!("{}\n", policy.keys()[i + 1].key_text())));
    }
    for (name, body) in files {
        let path = dir.join(name);
        if let Err(e) = std::fs::write(&path, body) {
            eprintln!("error: write {}: {e}", path.display());
            return ExitCode::FAILURE;
        }
        println!("{}", path.display());
    }
    ExitCode::SUCCESS
}
