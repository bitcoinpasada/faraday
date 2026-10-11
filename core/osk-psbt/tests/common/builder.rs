//! Builds test PSBTs from a master key: a fake funding transaction paying
//! one of our addresses, and a spend of it with the UTXO, script and
//! key-origin fields a coordinator would fill in.
//!
//! Shared by the integration tests (`mod common`) and the `inspect`
//! example (`#[path]` include), so the desktop demo can load the same
//! regtest example the tests sign.
#![allow(dead_code)]

use bitcoin::absolute::LockTime;
use bitcoin::bip32::{ChildNumber, DerivationPath, KeySource};
use bitcoin::opcodes::all::OP_CHECKMULTISIG;
use bitcoin::script::Builder;
use bitcoin::secp256k1::Secp256k1;
use bitcoin::transaction::Version;
use bitcoin::{
    Amount, OutPoint, PublicKey, ScriptBuf, Sequence, Transaction, TxIn, TxOut, Witness,
};
use osk_bip::keys::{MasterKey, Network, ScriptType};
use osk_bip::policy::WalletPolicy;
use osk_psbt::Psbt;

/// A funding transaction with one output to `script_pubkey`.
pub fn funding_tx(script_pubkey: ScriptBuf, value: u64) -> Transaction {
    Transaction {
        version: Version::TWO,
        lock_time: LockTime::ZERO,
        input: vec![TxIn {
            previous_output: OutPoint::null(),
            script_sig: Builder::new().push_int(101).into_script(),
            sequence: Sequence::MAX,
            witness: Witness::new(),
        }],
        output: vec![TxOut {
            value: Amount::from_sat(value),
            script_pubkey,
        }],
    }
}

/// What to build.
pub struct Build {
    pub script_type: ScriptType,
    pub network: Network,
    /// Value of the single input, sat.
    pub input_value: u64,
    /// Recipient outputs in order.
    pub recipients: Vec<(ScriptBuf, u64)>,
    /// A change output to `…/1/0` with key-origin data, appended last.
    pub change: Option<u64>,
    /// Absolute locktime.
    pub locktime: u32,
    /// Input sequence.
    pub sequence: u32,
}

impl Build {
    pub fn new(script_type: ScriptType, network: Network) -> Self {
        Self {
            script_type,
            network,
            input_value: 100_000,
            recipients: Vec::new(),
            change: None,
            locktime: 0,
            sequence: 0xffff_fffd,
        }
    }
}

/// The key origin for `account/change/index` of `master`.
fn origin(
    master: &MasterKey,
    script_type: ScriptType,
    change: bool,
    index: u32,
) -> (PublicKey, KeySource) {
    let account = master.account_xpub(script_type, 0).unwrap();
    let leaf = [
        ChildNumber::from_normal_idx(u32::from(change)).unwrap(),
        ChildNumber::from_normal_idx(index).unwrap(),
    ];
    let path: DerivationPath = account.path().extend(leaf);
    let key = account
        .xpub()
        .derive_pub(&Secp256k1::verification_only(), &leaf)
        .unwrap();
    (
        PublicKey::new(key.public_key),
        (master.fingerprint().into(), path),
    )
}

/// A single-signature spend of a fake funding output at `…/0/0`.
pub fn single_sig(master: &MasterKey, b: &Build) -> Psbt {
    let account = master.account_xpub(b.script_type, 0).unwrap();
    let receive = account.address(false, 0).unwrap().script_pubkey();
    let funding = funding_tx(receive.clone(), b.input_value);
    let (pk, source) = origin(master, b.script_type, false, 0);

    let mut output: Vec<TxOut> = b
        .recipients
        .iter()
        .map(|(spk, v)| TxOut {
            value: Amount::from_sat(*v),
            script_pubkey: spk.clone(),
        })
        .collect();
    if let Some(v) = b.change {
        output.push(TxOut {
            value: Amount::from_sat(v),
            script_pubkey: account.address(true, 0).unwrap().script_pubkey(),
        });
    }
    let tx = Transaction {
        version: Version::TWO,
        lock_time: LockTime::from_consensus(b.locktime),
        input: vec![TxIn {
            previous_output: OutPoint::new(funding.compute_txid(), 0),
            script_sig: ScriptBuf::new(),
            sequence: Sequence::from_consensus(b.sequence),
            witness: Witness::new(),
        }],
        output,
    };
    let mut psbt = bitcoin::Psbt::from_unsigned_tx(tx).unwrap();
    let input = &mut psbt.inputs[0];
    match b.script_type {
        ScriptType::Legacy => input.non_witness_utxo = Some(funding.clone()),
        // SegWit v0 signs the amount the PSBT states, so a coordinator
        // that expects a signature sends the transaction it spends with
        // it; taproot commits to every amount and needs only the output.
        ScriptType::NestedSegwit | ScriptType::NativeSegwit => {
            input.witness_utxo = Some(funding.output[0].clone());
            input.non_witness_utxo = Some(funding.clone());
        }
        ScriptType::Taproot => {
            input.witness_utxo = Some(funding.output[0].clone());
        }
    }
    if b.script_type == ScriptType::NestedSegwit {
        input.redeem_script = Some(ScriptBuf::new_p2wpkh(&pk.wpubkey_hash().unwrap()));
    }
    if b.script_type == ScriptType::Taproot {
        let xonly = pk.inner.x_only_public_key().0;
        input.tap_internal_key = Some(xonly);
        input.tap_key_origins.insert(xonly, (vec![], source));
    } else {
        input.bip32_derivation.insert(pk.inner, source);
    }
    if b.change.is_some() {
        let (pk, source) = origin(master, b.script_type, true, 0);
        let out = psbt.outputs.last_mut().unwrap();
        if b.script_type == ScriptType::Taproot {
            let xonly = pk.inner.x_only_public_key().0;
            out.tap_internal_key = Some(xonly);
            out.tap_key_origins.insert(xonly, (vec![], source));
        } else {
            out.bip32_derivation.insert(pk.inner, source);
        }
    }
    Psbt::from(psbt)
}

/// The BIP-48 p2wsh account path `m/48'/coin'/0'/2'`.
pub fn bip48_path(network: Network) -> DerivationPath {
    let h = |i| ChildNumber::from_hardened_idx(i).unwrap();
    DerivationPath::from(vec![h(48), h(network.coin_type()), h(0), h(2)])
}

/// The BIP-388 policy `wsh(sortedmulti(m, @0/**, …))` over the BIP-48
/// p2wsh account of every master, the way a coordinator would hand it to
/// the signer.
pub fn multisig_policy(masters: &[&MasterKey], m: usize, network: Network) -> WalletPolicy {
    let path = bip48_path(network);
    let mut text = format!("wsh(sortedmulti({m}");
    for i in 0..masters.len() {
        text.push_str(&format!(",@{i}/**"));
    }
    text.push_str("))");
    for master in masters {
        let xpub = master.derive(&path).to_xpub();
        text.push_str(&format!("\n[{}/{path:#}]{xpub}", master.fingerprint()));
    }
    WalletPolicy::parse(&text).expect("policy built from test keys")
}

/// The `sortedmulti(m, …)` witness script of `masters` at `path`.
pub fn sorted_multi_script(masters: &[&MasterKey], m: usize, path: &DerivationPath) -> ScriptBuf {
    let mut keys: Vec<PublicKey> = masters
        .iter()
        .map(|master| PublicKey::new(master.derive(path).to_xpub().public_key))
        .collect();
    keys.sort_by_key(|pk| pk.to_bytes());
    let mut b = Builder::new().push_int(m as i64);
    for pk in &keys {
        b = b.push_key(pk);
    }
    b.push_int(keys.len() as i64)
        .push_opcode(OP_CHECKMULTISIG)
        .into_script()
}

/// The same spend as [`sorted_multisig_change`], with the witness script
/// of the change output in the PSBT, the way a coordinator that expects
/// the signer to check its own change fills it in.
pub fn sorted_multisig_change_with_script(
    masters: &[&MasterKey],
    m: usize,
    network: Network,
    input_value: u64,
    recipients: &[(ScriptBuf, u64)],
    change: u64,
) -> Psbt {
    let mut psbt = sorted_multisig_change(masters, m, network, input_value, recipients, change);
    let leaf = [
        ChildNumber::from_normal_idx(1).unwrap(),
        ChildNumber::from_normal_idx(0).unwrap(),
    ];
    let path = bip48_path(network).extend(leaf);
    let script = sorted_multi_script(masters, m, &path);
    let inner = psbt.inner_mut();
    let last = inner.outputs.len() - 1;
    inner.outputs[last].witness_script = Some(script);
    psbt
}

/// A `sortedmulti(m, …)` p2wsh spend of a fake funding output at
/// `m/48'/coin'/0'/2'/0/0` of every master, paying `recipients` and, when
/// `change` is set, that many sat back to the same wallet's `…/1/0`.
pub fn sorted_multisig_change(
    masters: &[&MasterKey],
    m: usize,
    network: Network,
    input_value: u64,
    recipients: &[(ScriptBuf, u64)],
    change: u64,
) -> Psbt {
    let mut psbt = sorted_multisig(masters, m, network, input_value, recipients);
    let policy = multisig_policy(masters, m, network);
    let leaf = [
        ChildNumber::from_normal_idx(1).unwrap(),
        ChildNumber::from_normal_idx(0).unwrap(),
    ];
    let path = bip48_path(network).extend(leaf);
    let inner = psbt.inner_mut();
    inner.unsigned_tx.output.push(TxOut {
        value: Amount::from_sat(change),
        script_pubkey: policy.script_at(true, 0).unwrap(),
    });
    let mut out = bitcoin::psbt::Output::default();
    for master in masters {
        let derived = master.derive(&path);
        out.bip32_derivation.insert(
            derived.to_xpub().public_key,
            (master.fingerprint().into(), path.clone()),
        );
    }
    inner.outputs.push(out);
    psbt
}

/// A `sortedmulti(m, …)` p2wsh spend of a fake funding output at
/// `m/48'/coin'/0'/2'/0/0` of every master, paying `recipients`.
pub fn sorted_multisig(
    masters: &[&MasterKey],
    m: usize,
    network: Network,
    input_value: u64,
    recipients: &[(ScriptBuf, u64)],
) -> Psbt {
    let leaf = [
        ChildNumber::from_normal_idx(0).unwrap(),
        ChildNumber::from_normal_idx(0).unwrap(),
    ];
    let path = bip48_path(network).extend(leaf);
    let mut keys: Vec<(PublicKey, KeySource)> = masters
        .iter()
        .map(|master| {
            let derived = master.derive(&path);
            (
                PublicKey::new(derived.to_xpub().public_key),
                (master.fingerprint().into(), path.clone()),
            )
        })
        .collect();
    keys.sort_by_key(|(pk, _)| pk.to_bytes());

    let mut b = Builder::new().push_int(m as i64);
    for (pk, _) in &keys {
        b = b.push_key(pk);
    }
    let witness_script = b
        .push_int(keys.len() as i64)
        .push_opcode(OP_CHECKMULTISIG)
        .into_script();
    let spk = witness_script.to_p2wsh();
    let funding = funding_tx(spk, input_value);

    let tx = Transaction {
        version: Version::TWO,
        lock_time: LockTime::ZERO,
        input: vec![TxIn {
            previous_output: OutPoint::new(funding.compute_txid(), 0),
            script_sig: ScriptBuf::new(),
            sequence: Sequence::ENABLE_RBF_NO_LOCKTIME,
            witness: Witness::new(),
        }],
        output: recipients
            .iter()
            .map(|(spk, v)| TxOut {
                value: Amount::from_sat(*v),
                script_pubkey: spk.clone(),
            })
            .collect(),
    };
    let mut psbt = bitcoin::Psbt::from_unsigned_tx(tx).unwrap();
    let input = &mut psbt.inputs[0];
    input.witness_utxo = Some(funding.output[0].clone());
    input.non_witness_utxo = Some(funding.clone());
    input.witness_script = Some(witness_script);
    for (pk, source) in keys {
        input.bip32_derivation.insert(pk.inner, source);
    }
    Psbt::from(psbt)
}
