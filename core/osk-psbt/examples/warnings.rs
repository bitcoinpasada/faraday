//! Writes one PSBT per policy warning into a directory, so that every
//! warning card of the Sign flow has a file behind it.
//!
//! ```text
//! cargo run -p osk-psbt --example warnings -- tools/vectors/psbt
//! ```
//!
//! Everything is fixed: the "abandon … about" seed on regtest, the
//! "zoo … wrong" seed for the addresses that are not ours, one fake
//! funding output per input, and no randomness anywhere, so the files
//! are byte-identical from one run to the next. `tests/fixtures.rs`
//! checks the committed files against this code.
//!
//! Each fixture is built the way `tests/inspect.rs` builds the same
//! warning. A few warnings cannot appear alone; where a file carries
//! more than one, its note says so.
#![allow(dead_code)]

use std::process::ExitCode;

use bitcoin::bip32::{ChildNumber, DerivationPath};
use bitcoin::psbt::PsbtSighashType;
use bitcoin::sighash::EcdsaSighashType;
use bitcoin::{Amount, ScriptBuf, TapNodeHash};
use osk_bip::keys::{MasterKey, Network, ScriptType};
use osk_psbt::Psbt;

#[path = "../tests/common/builder.rs"]
mod builder;

use builder::{Build, single_sig};

const ABANDON: &str =
    "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
const ZOO: &str = "zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo wrong";

const NET: Network = Network::Regtest;
const SEGWIT: ScriptType = ScriptType::NativeSegwit;

/// One committed file.
pub struct Fixture {
    /// File name, `warn-<kind in kebab case>.psbt`.
    pub name: &'static str,
    /// What the file is for, one line, repeated in the README.
    pub note: &'static str,
    /// The transaction itself.
    pub psbt: Psbt,
}

fn master(words: &str, network: Network) -> MasterKey {
    let m = osk_bip::bip39::Mnemonic::parse(osk_bip::bip39::Language::English, words)
        .expect("fixed mnemonic");
    MasterKey::from_seed(&m.to_seed(b"").expect("empty passphrase"), network)
}

/// An address of the "zoo" seed: not ours, and stable.
fn theirs(index: u32, network: Network) -> ScriptBuf {
    master(ZOO, network)
        .account_xpub(SEGWIT, 0)
        .expect("account")
        .address(false, index)
        .expect("address")
        .script_pubkey()
}

/// The spend the other fixtures start from: 100 000 sat in, 60 000 to
/// someone else, 39 000 back as verified change, 1 000 fee.
fn basic(script_type: ScriptType) -> Psbt {
    let mut b = Build::new(script_type, NET);
    b.recipients.push((theirs(0, NET), 60_000));
    b.change = Some(39_000);
    single_sig(&master(ABANDON, NET), &b)
}

/// A spend of 100 000 sat paying `recipients`, with no change.
fn paying(recipients: &[(ScriptBuf, u64)]) -> Psbt {
    let mut b = Build::new(SEGWIT, NET);
    b.recipients.extend_from_slice(recipients);
    single_sig(&master(ABANDON, NET), &b)
}

/// Every fixture, in the order the README lists them.
pub fn fixtures() -> Vec<Fixture> {
    let mut out = Vec::new();
    let mut add = |name, note, psbt| out.push(Fixture { name, note, psbt });

    // Blocked: the change output claims our key origin but pays the
    // recipient's script. Redirecting it to a script already in the
    // transaction is what a coordinator would do, so the file carries
    // the address-reuse caution with the danger, and is the file that
    // shows the ranking.
    let mut psbt = basic(SEGWIT);
    let recipient = psbt.inner().unsigned_tx.output[0].script_pubkey.clone();
    psbt.inner_mut().unsigned_tx.output[1].script_pubkey = recipient;
    add(
        "warn-change-spoof.psbt",
        "blocked: change claims our key but pays the recipient; also the address-reuse caution",
        psbt,
    );

    // Caution: the change output names a fingerprint we have not loaded.
    let mut psbt = basic(SEGWIT);
    let zoo = master(ZOO, NET).fingerprint();
    for (fp, _) in psbt.inner_mut().outputs[1].bip32_derivation.values_mut() {
        *fp = zoo.into();
    }
    add(
        "warn-unverified-change.psbt",
        "caution: the change output claims a key we have not loaded",
        psbt,
    );

    // Danger: 80 000 sat to someone else and 20 000 sat of fee, a
    // quarter of what is being sent.
    add(
        "warn-high-fee.psbt",
        "danger: the fee is 25 % of the amount sent",
        paying(&[(theirs(0, NET), 80_000)]),
    );

    // Danger: 200 000 sat of fee on one small input, about 1 800 sat/vB.
    let mut b = Build::new(SEGWIT, NET);
    b.input_value = 1_000_000;
    b.recipients.push((theirs(0, NET), 800_000));
    add(
        "warn-absurd-fee-rate.psbt",
        "danger: about 1 800 sat/vB; the high-fee danger comes with it",
        single_sig(&master(ABANDON, NET), &b),
    );

    // Caution: 200 sat is below the dust limit of a p2wpkh output. The
    // second output keeps the fee small, so no fee warning joins it.
    add(
        "warn-dust-output.psbt",
        "caution: an output of 200 sat, below the dust limit",
        paying(&[(theirs(0, NET), 98_000), (theirs(1, NET), 200)]),
    );

    // Danger: SIGHASH_NONE leaves the outputs unsigned.
    let mut psbt = basic(SEGWIT);
    psbt.inner_mut().inputs[0].sighash_type = Some(PsbtSighashType::from(EcdsaSighashType::None));
    add(
        "warn-unusual-sighash.psbt",
        "danger: the input asks for SIGHASH_NONE",
        psbt,
    );

    // Info: a native-segwit input and a taproot input in one
    // transaction, which links the two addresses.
    let mut psbt = paying(&[(theirs(0, NET), 199_000)]);
    let other = basic(ScriptType::Taproot);
    psbt.inner_mut()
        .unsigned_tx
        .input
        .push(other.unsigned_tx().input[0].clone());
    psbt.inner_mut()
        .inputs
        .push(other.inner().inputs[0].clone());
    add(
        "warn-mixed-script-types.psbt",
        "info: the inputs spend two script types",
        psbt,
    );

    // Danger: mainnet key paths, read on a test network.
    let mainnet = master(ABANDON, Network::Mainnet);
    let mut b = Build::new(SEGWIT, Network::Mainnet);
    b.recipients.push((theirs(0, Network::Mainnet), 60_000));
    b.change = Some(39_000);
    add(
        "warn-network-mismatch.psbt",
        "danger: mainnet key paths (coin type 0), read on a test network; the paths are also not under a loaded account, so two cautions come with it",
        single_sig(&mainnet, &b),
    );

    // Danger: the whole transaction belongs to the "zoo" seed, so none
    // of our keys can sign any input.
    let mut b = Build::new(SEGWIT, NET);
    b.recipients.push((theirs(1, NET), 99_000));
    add(
        "warn-no-participating-key.psbt",
        "danger: every input belongs to a key we do not hold",
        single_sig(&master(ZOO, NET), &b),
    );

    // Caution: our fingerprint on a path under account 5, which is not
    // one of the loaded accounts.
    let mut psbt = basic(SEGWIT);
    for (_, path) in psbt.inner_mut().inputs[0].bip32_derivation.values_mut() {
        *path = reaccount(path, 5);
    }
    add(
        "warn-unknown-derivation.psbt",
        "caution: our key at a path under an account that is not loaded",
        psbt,
    );

    // Info: a locktime of block 800 000, with the input not signalling
    // replaceability.
    let mut b = Build::new(SEGWIT, NET);
    b.recipients.push((theirs(0, NET), 99_000));
    b.locktime = 800_000;
    b.sequence = 0xffff_fffe;
    add(
        "warn-locktime-in-future.psbt",
        "info: the transaction is locked until block 800 000",
        single_sig(&master(ABANDON, NET), &b),
    );

    // Info: an OP_RETURN output has no address to show.
    add(
        "warn-non-standard-script.psbt",
        "info: an OP_RETURN output, which has no address",
        paying(&[
            (theirs(0, NET), 99_000),
            (ScriptBuf::new_op_return(b"opensignerkit"), 0),
        ]),
    );

    // Caution: two outputs pay the same address.
    add(
        "warn-address-reuse.psbt",
        "caution: two outputs pay one address",
        paying(&[(theirs(0, NET), 49_500), (theirs(0, NET), 49_500)]),
    );

    // Blocked: the transaction the input carries is not the one it
    // spends, so its amount and script are unverifiable.
    let mut psbt = basic(ScriptType::Legacy);
    let mut fake = psbt.inner().inputs[0]
        .non_witness_utxo
        .clone()
        .expect("legacy input carries the transaction");
    fake.output[0].value = Amount::from_sat(5_000_000);
    psbt.inner_mut().inputs[0].non_witness_utxo = Some(fake);
    add(
        "warn-utxo-mismatch.psbt",
        "blocked: the input's transaction is not the one it spends, which leaves it with no UTXO and no key that can sign",
        psbt,
    );

    // Danger: no UTXO data at all, so the amounts and the fee are
    // unknown.
    let mut psbt = basic(SEGWIT);
    psbt.inner_mut().inputs[0].witness_utxo = None;
    psbt.inner_mut().inputs[0].non_witness_utxo = None;
    add(
        "warn-missing-utxo.psbt",
        "danger: the input carries no UTXO, so the fee cannot be checked and no key can be matched to it",
        psbt,
    );

    // Caution: a taproot input that claims a script tree the PSBT gives
    // no leaf of, and whose internal key with that tree is not the
    // output being spent, so there is nothing here to sign. A second,
    // ordinary input keeps a key participating, so the file shows the
    // caution on its own.
    let mut psbt = paying(&[(theirs(0, NET), 199_000)]);
    let mut other = basic(ScriptType::Taproot);
    other.inner_mut().inputs[0].tap_merkle_root = Some(TapNodeHash::assume_hidden([0x11; 32]));
    psbt.inner_mut()
        .unsigned_tx
        .input
        .push(other.unsigned_tx().input[0].clone());
    psbt.inner_mut()
        .inputs
        .push(other.inner().inputs[0].clone());
    add(
        "warn-unsupported-input.psbt",
        "caution: a taproot input with a tree this signer is given no way into",
        psbt,
    );

    // Blocked: two inputs state their amounts and carry nothing that
    // commits to them, which is what the two-round fee attack needs.
    let mut psbt = paying(&[(theirs(0, NET), 149_000)]);
    let mut b = Build::new(SEGWIT, NET);
    b.input_value = 50_000;
    b.recipients.push((theirs(0, NET), 49_000));
    let other = single_sig(&master(ABANDON, NET), &b);
    psbt.inner_mut()
        .unsigned_tx
        .input
        .push(other.unsigned_tx().input[0].clone());
    psbt.inner_mut()
        .inputs
        .push(other.inner().inputs[0].clone());
    for input in &mut psbt.inner_mut().inputs {
        input.non_witness_utxo = None;
    }
    add(
        "warn-amount-unverified.psbt",
        "blocked: two SegWit inputs with no previous transaction, so their amounts cannot be checked",
        psbt,
    );

    out
}

/// `path` with its third element, the account, replaced by `account'`.
fn reaccount(path: &DerivationPath, account: u32) -> DerivationPath {
    let mut parts: Vec<ChildNumber> = path.into_iter().copied().collect();
    parts[2] = ChildNumber::from_hardened_idx(account).expect("account index");
    DerivationPath::from(parts)
}

fn main() -> ExitCode {
    let Some(dir) = std::env::args().nth(1) else {
        eprintln!("usage: warnings <directory>");
        return ExitCode::FAILURE;
    };
    for f in fixtures() {
        let path = std::path::Path::new(&dir).join(f.name);
        if let Err(e) = std::fs::write(&path, f.psbt.to_base64()) {
            eprintln!("error: write {}: {e}", path.display());
            return ExitCode::FAILURE;
        }
        println!("{}  {}", f.name, f.note);
    }
    ExitCode::SUCCESS
}
