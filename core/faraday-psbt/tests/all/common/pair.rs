//! Pairs of PSBTs for the compare and fee bump tests: one payment with
//! change from the "abandon … about" key, and edits of it.
#![allow(dead_code)]

use bitcoin::{OutPoint, ScriptBuf};
use faraday_bip::keys::{Fingerprint, Network, ScriptType};
use faraday_crypto::work::Compute;
use faraday_psbt::diff::Side;
use faraday_psbt::{Context, Inspection, KeyRef, Psbt, inspect};

use super::builder::{Build, single_sig};
use super::{ABANDON, LEGAL, ZOO, master};

pub fn fp() -> Fingerprint {
    master(ABANDON, Network::Regtest).fingerprint()
}

/// The first receive script of another key.
pub fn script_of(words: &str) -> ScriptBuf {
    let c = Compute::unchecked();
    master(words, Network::Regtest)
        .account_xpub(&c, ScriptType::NativeSegwit, 0)
        .unwrap()
        .address(false, 0)
        .unwrap()
        .script_pubkey()
}

pub fn payee() -> ScriptBuf {
    script_of(ZOO)
}

pub fn other_payee() -> ScriptBuf {
    script_of(LEGAL)
}

/// 100,000 in, 50,000 to the payee, 49,000 change, fee 1,000.
pub fn base() -> Build {
    let mut b = Build::new(ScriptType::NativeSegwit, Network::Regtest);
    b.recipients = vec![(payee(), 50_000)];
    b.change = Some(49_000);
    b
}

pub fn build(b: &Build) -> Psbt {
    single_sig(&master(ABANDON, Network::Regtest), b)
}

pub fn original() -> Psbt {
    build(&base())
}

/// The original with its change lower by `more`: that much more fee.
pub fn bumped(more: u64) -> Psbt {
    let mut b = base();
    b.change = Some(49_000 - more);
    build(&b)
}

pub fn edit(p: &Psbt, f: impl FnOnce(&mut bitcoin::Psbt)) -> Psbt {
    let mut inner = p.inner().clone();
    f(&mut inner);
    Psbt::from(inner)
}

/// `p` with the input of `donor` added.
pub fn add_input(p: &Psbt, donor: &Psbt) -> Psbt {
    edit(p, |x| {
        x.unsigned_tx
            .input
            .push(donor.inner().unsigned_tx.input[0].clone());
        x.inputs.push(donor.inner().inputs[0].clone());
    })
}

/// A PSBT whose one input is another coin of the key's.
pub fn donor(value: u64, words: &str) -> Psbt {
    let mut b = base();
    b.input_value = value;
    single_sig(&master(words, Network::Regtest), &b)
}

pub fn outpoint(p: &Psbt, index: usize) -> OutPoint {
    p.inner().unsigned_tx.input[index].previous_output
}

pub fn keys(loaded: bool) -> Vec<KeyRef> {
    if loaded {
        vec![
            KeyRef::from_master(&Compute::unchecked(), &master(ABANDON, Network::Regtest), 0)
                .unwrap(),
        ]
    } else {
        Vec::new()
    }
}

pub fn look(p: &Psbt, loaded: bool) -> Inspection {
    let k = keys(loaded);
    let ctx = Context {
        network: Network::Regtest,
        keys: &k,
        wallets: &[],
        musig_session: None,
    };
    inspect(&Compute::unchecked(), p, &ctx)
}

/// Both inspected, then `f` on the two sides.
pub fn sides<R>(
    a: &Psbt,
    a_loaded: bool,
    b: &Psbt,
    b_loaded: bool,
    f: impl FnOnce(Side<'_>, Side<'_>) -> R,
) -> R {
    let (ia, ib) = (look(a, a_loaded), look(b, b_loaded));
    f(
        Side {
            psbt: a,
            inspection: &ia,
            psbt_version: Some(0),
        },
        Side {
            psbt: b,
            inspection: &ib,
            psbt_version: Some(0),
        },
    )
}

/// A Bitcoin Core fixture of `tools/vectors/psbt/` (`bump-core-*`).
pub fn core_file(name: &str) -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tools/vectors/psbt")
        .join(name);
    std::fs::read_to_string(path).expect("a committed fixture")
}

pub fn core_psbt(name: &str) -> Psbt {
    let c = Compute::unchecked();
    Psbt::parse_base64(&c, core_file(name).trim()).expect("Core's PSBT")
}

/// Core's signed payment, read as Decode reads a finished transaction: a
/// PSBT whose inputs are final, stating no amounts.
pub fn core_finished() -> Psbt {
    let c = Compute::unchecked();
    let hex = core_file("bump-core-original.tx");
    let raw = faraday_crypto::hex::decode_vec(hex.trim()).expect("hex");
    let tx = faraday_psbt::transaction::read_raw(&raw).expect("a transaction");
    let (wrapped, _) = faraday_psbt::transaction::wrap_raw(&raw).expect("wrapped");
    let mut psbt = Psbt::parse_bytes(&c, &wrapped).expect("wrapped PSBT");
    for (input, txin) in psbt.inner_mut().inputs.iter_mut().zip(&tx.input) {
        if !txin.witness.is_empty() {
            input.final_script_witness = Some(txin.witness.clone());
        }
    }
    psbt
}
