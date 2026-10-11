//! A taproot multisig by script path: `tr(NUMS, sortedmulti_a(2,…))`,
//! the wallet Nunchuk and Coldcard build. The device loads it, verifies
//! its change, signs with two of its three keys and finalizes.

use bitcoin::absolute::LockTime;
use bitcoin::bip32::ChildNumber;
use bitcoin::secp256k1::{Secp256k1, XOnlyPublicKey};
use bitcoin::transaction::Version;
use bitcoin::{Amount, OutPoint, ScriptBuf, Sequence, Transaction, TxIn, TxOut, Witness};
use osk_bip::keys::{MasterKey, Network};
use osk_bip::miniscript::descriptor::{DefiniteDescriptorKey, Descriptor};
use osk_bip::miniscript::psbt::PsbtExt;
use osk_bip::policy::WalletPolicy;
use osk_psbt::{Aux, Context, KeyRef, Nonce, OutputKind, Psbt, SigKind, finalize, inspect, sign};

mod common;

use common::builder::{bip48_path, funding_tx};
use common::{ABANDON, LEGAL, ZOO, master};

#[path = "../examples/tapmulti.rs"]
mod example;

const NET: Network = Network::Regtest;
/// The first receive address of the committed wallet, which is the
/// address Bitcoin Core 31.1 funded on 2026-09-19.
const FUNDED: &str = "bcrt1p6srr8gqakft982yss7sh0sl8urqjj8fk5l2n9dumnj2fljvfdf2swezdl4";
/// The checksum Core's `getdescriptorinfo` gave that wallet.
const CHECKSUM: &str = "uygt73j8";
/// The transaction `testmempoolaccept` allowed in that run.
const ACCEPTED: &str = "42096a2e92f0ae357cef0e3403cb528876e7f1fe9ab0dcbd629016d697efe64d";

/// Where the committed fixtures live.
fn vectors() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tools/vectors/psbt")
}

/// One key's x-only public key at the first receive address, which is
/// what a signature under the leaf is keyed by.
fn derived_key(m: &MasterKey) -> XOnlyPublicKey {
    let secp = Secp256k1::verification_only();
    m.derive(&bip48_path(NET))
        .to_xpub()
        .derive_pub(
            &secp,
            &[
                ChildNumber::from_normal_idx(0).expect("a chain"),
                ChildNumber::from_normal_idx(0).expect("an index"),
            ],
        )
        .expect("normal child")
        .public_key
        .x_only_public_key()
        .0
}
/// BIP 341's NUMS point, the internal key of a script-only wallet.
const NUMS: &str = "50929b74c1a04954b78b4b6035e97a5e078a5a0f28ec96d547bfee9ace803ac0";

/// One key of the wallet as a descriptor writes it: its origin and its
/// account extended public key, at the BIP-48 account the field writes
/// script multisig keys at.
fn key_text(m: &MasterKey) -> String {
    let path = bip48_path(NET);
    format!(
        "[{}/{path:#}]{}",
        m.fingerprint(),
        m.derive(&path).to_xpub()
    )
}

/// The 2-of-3 taproot multisig over the three test keys.
fn policy() -> WalletPolicy {
    let keys: Vec<String> = [ABANDON, ZOO, LEGAL]
        .iter()
        .map(|w| key_text(&master(w, NET)))
        .collect();
    let text = format!(
        "tr({NUMS},sortedmulti_a(2,{}/<0;1>/*,{}/<0;1>/*,{}/<0;1>/*))",
        keys[0], keys[1], keys[2]
    );
    WalletPolicy::parse_any(&text).expect("a taproot multisig wallet")
}

/// The wallet at one address, as a coordinator fills a PSBT in from it:
/// the `multi_a` over the same keys, written in the order the sorted
/// script puts them at this index.
fn definite(policy: &WalletPolicy, change: bool, index: u32) -> Descriptor<DefiniteDescriptorKey> {
    let secp = Secp256k1::verification_only();
    let steps = [
        ChildNumber::from_normal_idx(u32::from(change)).expect("0 or 1"),
        ChildNumber::from_normal_idx(index).expect("an index"),
    ];
    let mut keys: Vec<(XOnlyPublicKey, String)> = policy
        .keys()
        .iter()
        .map(|key| {
            let derived = key
                .xpub()
                .derive_pub(&secp, &steps)
                .expect("normal child")
                .public_key
                .x_only_public_key()
                .0;
            (
                derived,
                format!("{}/{}/{index}", key.key_text(), u32::from(change)),
            )
        })
        .collect();
    keys.sort_by_key(|(derived, _)| derived.serialize());
    let text = format!(
        "tr({NUMS},multi_a(2,{},{},{}))",
        keys[0].1, keys[1].1, keys[2].1
    );
    text.parse().expect("a definite descriptor")
}

/// A spend of that wallet: 100 000 sat in at its first receive address,
/// 60 000 out, 39 000 back to its own first change address.
fn spend() -> (WalletPolicy, Psbt) {
    let policy = policy();
    let input = definite(&policy, false, 0);
    let back = definite(&policy, true, 0);
    let funding = funding_tx(input.script_pubkey(), 100_000);
    let elsewhere = master(ZOO, NET)
        .account_xpub(osk_bip::keys::ScriptType::NativeSegwit, 0)
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
            sequence: Sequence::ENABLE_RBF_NO_LOCKTIME,
            witness: Witness::new(),
        }],
        output: vec![
            TxOut {
                value: Amount::from_sat(60_000),
                script_pubkey: elsewhere,
            },
            TxOut {
                value: Amount::from_sat(39_000),
                script_pubkey: back.script_pubkey(),
            },
        ],
    };
    let mut psbt = bitcoin::Psbt::from_unsigned_tx(tx).expect("unsigned");
    psbt.inputs[0].witness_utxo = Some(funding.output[0].clone());
    psbt.inputs[0].non_witness_utxo = Some(funding);
    psbt.update_input_with_descriptor(0, &input)
        .expect("the input is that address");
    psbt.update_output_with_descriptor(1, &back)
        .expect("the change is that address");
    (policy, Psbt::from(psbt))
}

/// The address the wallet pays to is the address the script it pays to
/// makes, and the key path cannot be spent.
#[test]
fn a_taproot_multisig_wallet_states_what_it_is() {
    let policy = policy();
    assert!(policy.key_path_unspendable());
    assert_eq!(policy.tapscript_quorum(), Some((2, 3)));
    assert!(policy.tapscript_sorted());
    assert!(
        policy
            .to_descriptor()
            .starts_with(&format!("tr({NUMS},sortedmulti_a(2,")),
        "the wallet is written back as the sorted form it was read as: {}",
        policy.to_descriptor()
    );
    assert_eq!(
        WalletPolicy::parse_any(&policy.to_descriptor_checksummed())
            .expect("its own descriptor reads back")
            .to_descriptor(),
        policy.to_descriptor()
    );
    let two_part = WalletPolicy::parse_any(&format!(
        "tr({NUMS},sortedmulti_a(2,@0/**,@1/**,@2/**))\n{}\n{}\n{}",
        policy.keys()[0].key_text(),
        policy.keys()[1].key_text(),
        policy.keys()[2].key_text()
    ))
    .expect("the same wallet in the two-part form");
    assert_eq!(two_part.to_descriptor(), policy.to_descriptor());
    assert_eq!(
        policy.address_at(NET, false, 0).expect("an address"),
        bitcoin::Address::from_script(
            &definite(&policy, false, 0).script_pubkey(),
            bitcoin::Network::Regtest
        )
        .expect("a taproot address")
    );
}

/// The change of the transaction is the wallet's own change address, and
/// signing with two of the three keys finishes the input: one Schnorr
/// signature per key, and a witness that holds an empty vector where the
/// third key's signature would be.
#[test]
fn two_of_the_three_keys_sign_and_the_input_finalizes() {
    let (policy, mut psbt) = spend();
    let (abandon, zoo) = (master(ABANDON, NET), master(ZOO, NET));
    let keys = [
        KeyRef::from_master(&abandon, 0).expect("accounts"),
        KeyRef::from_master(&zoo, 1).expect("accounts"),
    ];
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
        insp.outputs[1].kind,
        OutputKind::WalletChange {
            wallet: 0,
            change: true,
            index: 0,
        }
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
    .expect("two of the wallet's keys are ours");
    assert_eq!(result.signed_inputs.len(), 2, "one signature per key");
    assert!(
        result
            .signed_inputs
            .iter()
            .all(|s| s.kind == SigKind::Schnorr)
    );
    assert!(result.complete);

    let tx = finalize(&mut psbt)
        .expect("a satisfied script")
        .expect("every input final");
    assert_eq!(
        tx.input[0].witness.len(),
        5,
        "three key slots, the leaf script and the control block"
    );
    assert!(
        tx.input[0]
            .witness
            .iter()
            .filter(|item| item.is_empty())
            .count()
            == 1,
        "the key that did not sign leaves an empty vector"
    );
}

/// The wallet Bitcoin Core funded is the wallet this tree builds: the
/// committed policy over the same three keys, and the first receive
/// address is the one Core paid into.
#[test]
fn the_committed_policy_is_the_wallet_core_imported() {
    let committed = WalletPolicy::parse_any(
        &std::fs::read_to_string(vectors().join(example::POLICY_NAME)).expect("the fixture"),
    )
    .expect("a taproot multisig wallet");
    assert_eq!(committed.to_descriptor(), policy().to_descriptor());
    assert_eq!(
        committed
            .address_at(NET, false, 0)
            .expect("an address")
            .to_string(),
        FUNDED,
        "the address Core funded"
    );
    assert!(
        committed
            .to_descriptor_checksummed()
            .ends_with(&format!("#{CHECKSUM}")),
        "the checksum Bitcoin Core computed for the same descriptor: {}",
        committed.to_descriptor_checksummed()
    );
}

/// What the device wrote from Core's spend: one Schnorr signature per
/// key under the leaf's hash, the input left unfinalized for whoever
/// collects it, and the same bytes every time.
#[test]
fn two_keys_sign_cores_spend_and_the_fixture_is_what_they_wrote() {
    let dir = vectors();
    let committed = std::fs::read_to_string(dir.join(example::SIGNED_NAME)).expect("the fixture");
    let (psbt, result) = example::signed_by(&dir, &[example::SEEDS[0], example::SEEDS[1]]);
    assert_eq!(committed, psbt.to_base64(), "byte for byte");
    assert_eq!(result.signed_inputs.len(), 2, "one signature per key");
    assert!(result.complete, "two of three satisfies the leaf");

    let input = &psbt.inner().inputs[0];
    assert!(
        input.final_script_witness.is_none(),
        "signing finalizes nothing"
    );
    let leaf = input
        .tap_scripts
        .values()
        .map(|(script, version)| bitcoin::taproot::TapLeafHash::from_script(script, *version))
        .next()
        .expect("the one leaf Core wrote");
    assert_eq!(input.tap_script_sigs.len(), 2);
    assert!(
        input.tap_script_sigs.keys().all(|(_, hash)| *hash == leaf),
        "both signatures are for that leaf"
    );
    let signers: Vec<XOnlyPublicKey> = input.tap_script_sigs.keys().map(|(key, _)| *key).collect();
    for (words, signed) in [
        (example::SEEDS[0], true),
        (example::SEEDS[1], true),
        (example::SEEDS[2], false),
    ] {
        let key = derived_key(&example::master(words));
        assert_eq!(
            signers.contains(&key),
            signed,
            "the third key of a 2-of-3 signs nothing here"
        );
    }
}

/// One key of the three leaves the transaction partly signed: the leaf
/// wants two, so nothing can be finalized from it yet.
#[test]
fn one_key_leaves_the_transaction_partly_signed() {
    let (psbt, result) = example::signed_by(&vectors(), &[example::SEEDS[0]]);
    assert_eq!(result.signed_inputs.len(), 1);
    assert!(!result.complete, "one signature of the two the leaf wants");
    assert_eq!(psbt.inner().inputs[0].tap_script_sigs.len(), 1);
}

/// All three keys on one device sign all three ways: the leaf takes what
/// it needs and the finalizer drops the rest, so the transaction is the
/// same one Core accepted.
#[test]
fn a_third_key_signs_too_and_the_witness_still_holds_two() {
    let (mut psbt, result) = example::signed_by(
        &vectors(),
        &[example::SEEDS[0], example::SEEDS[1], example::SEEDS[2]],
    );
    assert_eq!(result.signed_inputs.len(), 3);
    let tx = finalize(&mut psbt)
        .expect("a satisfied leaf")
        .expect("every input final");
    assert_eq!(tx.input[0].witness.len(), 5);
    assert_eq!(
        tx.compute_txid().to_string(),
        ACCEPTED,
        "the transaction Bitcoin Core accepted"
    );
}
