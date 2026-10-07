//! The threshold spend through the crate (`docs/PLANNING.md` §16.103):
//! the two locations in both orders, three locations for a 3-of-3, the
//! finished transaction, and every refusal by the name it carries.

#[path = "../examples/threshold.rs"]
mod fixture;

use std::path::PathBuf;

use osk_bip::frost::{self, SecShare};
use osk_bip::keys::Network;
use osk_bip::policy::WalletPolicy;
use osk_bip::threshold::ThresholdRecord;
use osk_psbt::bitcoin::NetworkKind;
use osk_psbt::bitcoin::secp256k1::Secp256k1;
use osk_psbt::threshold::{Carry, CarrySection, carry_bytes};
use osk_psbt::{
    Aux, Context, Level, Nonce, Psbt, ShareKey, ShareRef, SignResult, WarningKind, finalize,
    inspect, sign,
};

const SEED: [u8; 32] = [0x54; 32];

fn dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tools/vectors/psbt")
}

fn funded() -> Psbt {
    let text = std::fs::read_to_string(dir().join(fixture::FIRST_NAME)).expect("the funded PSBT");
    Psbt::parse_base64(text.trim()).expect("a PSBT")
}

/// One location's pass: which share it holds, which others it chooses,
/// and the carry section it read.
fn pass(
    psbt: &mut Psbt,
    dealt: &frost::Dealt,
    policy: &WalletPolicy,
    holding: u32,
    others: &[u32],
    carry: Option<&CarrySection>,
) -> Result<SignResult, osk_psbt::Error> {
    let secp = Secp256k1::new();
    let share = &dealt.shares[holding as usize];
    let pubshare = share.public_share(&secp);
    let wallets = [policy.clone()];
    let shares = [ShareRef { pubshare }];
    let ctx = Context {
        network: Network::Regtest,
        keys: &[],
        wallets: &wallets,
        musig_session: None,
        shares: &shares,
        carry,
    };
    let door = |take: &mut dyn FnMut(&SecShare)| take(share);
    let keys = [ShareKey {
        pubshare,
        open: &door,
    }];
    let mut session = None;
    sign(
        psbt,
        &[],
        &[],
        &ctx,
        false,
        Nonce::LowR,
        Aux::Deterministic,
        &mut session,
        SEED,
        &keys,
        others,
    )
}

/// The review a location gives a transaction, which is where a refusal
/// shows up as a blocked warning.
fn review(
    psbt: &Psbt,
    dealt: &frost::Dealt,
    policy: &WalletPolicy,
    holding: u32,
    carry: Option<&CarrySection>,
) -> Vec<WarningKind> {
    let secp = Secp256k1::new();
    let pubshare = dealt.shares[holding as usize].public_share(&secp);
    let wallets = [policy.clone()];
    let shares = [ShareRef { pubshare }];
    let ctx = Context {
        network: Network::Regtest,
        keys: &[],
        wallets: &wallets,
        musig_session: None,
        shares: &shares,
        carry,
    };
    inspect(psbt, &ctx)
        .warnings
        .iter()
        .filter(|w| w.level == Level::Blocked)
        .map(|w| w.kind.clone())
        .collect()
}

fn policy() -> WalletPolicy {
    WalletPolicy::of_record(ThresholdRecord::new(
        fixture::dealt().info,
        NetworkKind::Test,
    ))
}

/// The whole route with `first` signing first and `second` finishing:
/// the carry file, the finished transaction, and the witness a node
/// would see.
fn route(first: u32, second: u32) -> (Vec<u8>, Psbt) {
    let dealt = fixture::dealt();
    let policy = policy();
    let mut psbt = funded();
    let result = pass(&mut psbt, &dealt, &policy, first, &[second], None).expect("first location");
    let outcome = result.threshold.expect("a threshold input");
    assert_eq!(outcome.t, 2);
    assert_eq!(outcome.partial_sigs, 1);
    let section = outcome.carry.expect("one signer is still to sign");
    let file = carry_bytes(&section, &psbt.to_bytes());
    drop(section);

    let carry = Carry::parse(&file).expect("the carry file");
    let mut psbt = Psbt::parse_bytes(&carry.psbt).expect("the carried PSBT");
    let result = pass(
        &mut psbt,
        &dealt,
        &policy,
        second,
        &[],
        Some(&carry.section),
    )
    .expect("second location");
    let outcome = result.threshold.expect("a threshold input");
    assert_eq!(outcome.partial_sigs, 2);
    assert!(outcome.carry.is_none(), "nobody is left to sign");
    assert!(result.complete, "the input is ready to finalize");
    (file, psbt)
}

#[test]
fn either_share_may_sign_first_and_the_transaction_is_accepted_either_way() {
    for (first, second) in [(0u32, 1u32), (1, 0)] {
        let (_, psbt) = route(first, second);
        // Nothing of the session is left on the finished transaction.
        for input in &psbt.inner().inputs {
            assert!(
                input.proprietary.keys().all(|k| k.prefix != b"osk"),
                "an osk record survived the finished transaction"
            );
        }
        let tx = finalize(&mut psbt.clone())
            .expect("finalize")
            .expect("a tx");
        assert_eq!(tx.input[0].witness.len(), 1, "one key-path signature");
        assert_eq!(tx.input[0].witness.to_vec()[0].len(), 64);
    }
}

#[test]
fn the_two_orders_give_the_same_group_and_a_valid_signature() {
    let (_, a) = route(0, 1);
    let (_, b) = route(1, 0);
    let sig_a = a.inner().inputs[0]
        .tap_key_sig
        .expect("a key-path signature");
    let sig_b = b.inner().inputs[0]
        .tap_key_sig
        .expect("a key-path signature");
    // Different nonce sets, so different signatures; both spend the
    // same output, which is what the finished transaction proves.
    assert_ne!(sig_a.signature, sig_b.signature);
    assert_eq!(
        a.unsigned_tx().compute_txid(),
        b.unsigned_tx().compute_txid()
    );
}

#[test]
fn three_of_three_signs_at_three_locations() {
    let secp = Secp256k1::new();
    // A 3-of-3 over the fixture scalars: a different group, a different
    // wallet, and a spend of its own first address.
    let chosen: Vec<(u32, SecShare)> = (0..3)
        .map(|i| {
            let bytes = fixture::dealt().shares[i as usize].secret_bytes();
            (i, SecShare::from_bytes(&bytes).expect("a scalar"))
        })
        .collect();
    let dealt = frost::deal(&secp, 3, 3, &chosen).expect("three chosen shares of three");
    let record = ThresholdRecord::new(dealt.info.clone(), NetworkKind::Test);
    let policy = WalletPolicy::of_record(record.clone());
    let mut psbt = spend_of(&record, &policy);

    // Location one chooses the whole set and draws every nonce.
    let result = pass(&mut psbt, &dealt, &policy, 0, &[1, 2], None).expect("first location");
    let section = result
        .threshold
        .expect("a threshold input")
        .carry
        .expect("two signers are still to sign");
    assert_eq!(section.entries.len(), 2);
    let mut file = carry_bytes(&section, &psbt.to_bytes());
    drop(section);

    // Locations two and three take their own nonce and add their own
    // partial signature; the last one aggregates.
    for (n, holding) in [1u32, 2].into_iter().enumerate() {
        let carry = Carry::parse(&file).expect("the carry file");
        let mut psbt = Psbt::parse_bytes(&carry.psbt).expect("the carried PSBT");
        let result = pass(
            &mut psbt,
            &dealt,
            &policy,
            holding,
            &[],
            Some(&carry.section),
        )
        .expect("a later location");
        let outcome = result.threshold.expect("a threshold input");
        assert_eq!(outcome.partial_sigs, n + 2);
        match outcome.carry {
            Some(section) => {
                assert_eq!(section.entries.len(), 1, "one signer is still to sign");
                file = carry_bytes(&section, &psbt.to_bytes());
            }
            None => {
                assert!(result.complete, "the last share finishes the input");
                let tx = finalize(&mut psbt).expect("finalize").expect("a tx");
                assert_eq!(tx.input[0].witness.to_vec()[0].len(), 64);
            }
        }
    }
}

/// A spend of the wallet's own first receive address, paying its own
/// change address: what a coordinator would build for a group this test
/// deals itself.
fn spend_of(record: &ThresholdRecord, policy: &WalletPolicy) -> Psbt {
    use osk_psbt::bitcoin::bip32::ChildNumber;
    use osk_psbt::bitcoin::hashes::Hash;
    use osk_psbt::bitcoin::transaction::Version;
    use osk_psbt::bitcoin::{
        Amount, OutPoint, ScriptBuf, Sequence, Transaction, TxIn, TxOut, Txid, Witness,
        absolute::LockTime,
    };

    let secp = Secp256k1::new();
    let steps = [
        ChildNumber::from_normal_idx(0).expect("0"),
        ChildNumber::from_normal_idx(0).expect("0"),
    ];
    let internal = record
        .xpub
        .derive_pub(&secp, &steps.to_vec())
        .expect("the group key at 0/0")
        .public_key
        .x_only_public_key()
        .0;
    let spent = TxOut {
        value: Amount::from_sat(100_000),
        script_pubkey: policy.script_at(false, 0).expect("the wallet's address"),
    };
    let tx = Transaction {
        version: Version::TWO,
        lock_time: LockTime::ZERO,
        input: vec![TxIn {
            previous_output: OutPoint {
                txid: Txid::all_zeros(),
                vout: 0,
            },
            script_sig: ScriptBuf::new(),
            sequence: Sequence::ENABLE_RBF_NO_LOCKTIME,
            witness: Witness::new(),
        }],
        output: vec![TxOut {
            value: Amount::from_sat(99_800),
            script_pubkey: policy.script_at(true, 0).expect("the change address"),
        }],
    };
    let mut inner = osk_psbt::bitcoin::Psbt::from_unsigned_tx(tx).expect("an unsigned tx");
    inner.inputs[0].witness_utxo = Some(spent);
    inner.inputs[0].tap_internal_key = Some(internal);
    inner.inputs[0].tap_key_origins.insert(
        internal,
        (
            vec![],
            (
                osk_psbt::bitcoin::bip32::Fingerprint::from(record.fingerprint().0),
                steps.into_iter().collect(),
            ),
        ),
    );
    Psbt::parse_bytes(&inner.serialize()).expect("a PSBT")
}

#[test]
fn a_tampered_carry_file_is_refused_because_the_nonce_is_not_its_record() {
    let dealt = fixture::dealt();
    let policy = policy();
    let (file, _) = route(0, 1);
    let mut tampered = file.clone();
    // The first secret nonce byte of the one entry the section holds:
    // magic (4), version (1), signers count (1), two shares (66),
    // entries count (1), input (4), share (33), sighash (32).
    let at = 4 + 1 + 1 + 66 + 1 + 4 + 33 + 32;
    tampered[at] ^= 0x01;
    let carry = Carry::parse(&tampered).expect("still a carry file");
    let psbt = Psbt::parse_bytes(&carry.psbt).expect("the carried PSBT");
    assert_eq!(
        review(&psbt, &dealt, &policy, 1, Some(&carry.section)),
        vec![WarningKind::ThresholdRefused(
            osk_psbt::threshold::Refusal::NonceMismatch
        )]
    );
}

#[test]
fn reading_the_same_carry_file_twice_gives_the_identical_signature() {
    let dealt = fixture::dealt();
    let policy = policy();
    let (file, _) = route(0, 1);
    let once = {
        let carry = Carry::parse(&file).expect("the carry file");
        let mut psbt = Psbt::parse_bytes(&carry.psbt).expect("the carried PSBT");
        pass(&mut psbt, &dealt, &policy, 1, &[], Some(&carry.section)).expect("second location");
        psbt
    };
    let twice = {
        let carry = Carry::parse(&file).expect("the carry file");
        let mut psbt = Psbt::parse_bytes(&carry.psbt).expect("the carried PSBT");
        pass(&mut psbt, &dealt, &policy, 1, &[], Some(&carry.section)).expect("second location");
        psbt
    };
    assert_eq!(once.to_bytes(), twice.to_bytes());
}

#[test]
fn a_substituted_nonce_breaks_the_first_share_s_partial_signature() {
    let dealt = fixture::dealt();
    let policy = policy();
    let (file, _) = route(0, 1);
    let carry = Carry::parse(&file).expect("the carry file");
    let mut psbt = Psbt::parse_bytes(&carry.psbt).expect("the carried PSBT");
    // Share 0's public nonce replaced by share 1's, which is a point on
    // the curve and a nonce nobody signed under.
    let secp = Secp256k1::new();
    let zero = dealt.shares[0].public_share(&secp);
    let one = dealt.shares[1].public_share(&secp);
    let input = &mut psbt.inner_mut().inputs[0];
    let theirs = osk_psbt::threshold::pub_nonces(input).expect("the records");
    let output_key = theirs[0].output_key;
    let other = theirs
        .iter()
        .find(|e| e.pubshare == one)
        .expect("share 1's nonce")
        .value;
    osk_psbt::threshold::write_pub_nonce(input, &zero, &output_key, &other);
    assert_eq!(
        review(&psbt, &dealt, &policy, 1, Some(&carry.section)),
        vec![WarningKind::ThresholdRefused(
            osk_psbt::threshold::Refusal::PartialSig
        )],
        "share 0's partial signature was made under the nonce that was replaced"
    );
}

#[test]
fn a_file_for_another_signer_set_is_refused() {
    let dealt = fixture::dealt();
    let policy = policy();
    let (file, _) = route(0, 1);
    let carry = Carry::parse(&file).expect("the carry file");
    let psbt = Psbt::parse_bytes(&carry.psbt).expect("the carried PSBT");
    let secp = Secp256k1::new();
    let mut section = CarrySection::new(vec![
        dealt.shares[0].public_share(&secp),
        dealt.shares[2].public_share(&secp),
    ]);
    for entry in &carry.section.entries {
        section.entries.push(osk_psbt::threshold::CarryEntry {
            input: entry.input,
            pubshare: entry.pubshare,
            sighash: entry.sighash,
            secnonce: frost::SecNonce::from_bytes(&entry.secnonce.serialize())
                .expect("the same nonce"),
        });
    }
    assert_eq!(
        review(&psbt, &dealt, &policy, 1, Some(&section)),
        vec![WarningKind::ThresholdRefused(
            osk_psbt::threshold::Refusal::SignerSet
        )]
    );
}

#[test]
fn a_file_for_another_transaction_is_refused() {
    let dealt = fixture::dealt();
    let policy = policy();
    let (file, _) = route(0, 1);
    let carry = Carry::parse(&file).expect("the carry file");
    let psbt = Psbt::parse_bytes(&carry.psbt).expect("the carried PSBT");
    let mut section = CarrySection::new(carry.section.signers.clone());
    for entry in &carry.section.entries {
        let mut sighash = entry.sighash;
        sighash[0] ^= 0xff;
        section.entries.push(osk_psbt::threshold::CarryEntry {
            input: entry.input,
            pubshare: entry.pubshare,
            sighash,
            secnonce: frost::SecNonce::from_bytes(&entry.secnonce.serialize())
                .expect("the same nonce"),
        });
    }
    assert_eq!(
        review(&psbt, &dealt, &policy, 1, Some(&section)),
        vec![WarningKind::ThresholdRefused(
            osk_psbt::threshold::Refusal::AnotherTransaction
        )]
    );
}

#[test]
fn a_file_for_another_share_is_refused() {
    let dealt = fixture::dealt();
    let policy = policy();
    let (file, _) = route(0, 1);
    let carry = Carry::parse(&file).expect("the carry file");
    let psbt = Psbt::parse_bytes(&carry.psbt).expect("the carried PSBT");
    // Share 2 is not one of the signers this transaction names.
    assert_eq!(
        review(&psbt, &dealt, &policy, 2, Some(&carry.section)),
        vec![WarningKind::ThresholdRefused(
            osk_psbt::threshold::Refusal::NotASigner
        )]
    );
    // Share 0 is a signer, and has signed, so it is offered nothing to
    // do and is refused nothing.
    assert!(review(&psbt, &dealt, &policy, 0, Some(&carry.section)).is_empty());
}

#[test]
fn nonces_with_no_file_and_a_file_with_no_nonces_are_both_refused() {
    let dealt = fixture::dealt();
    let policy = policy();
    let (file, _) = route(0, 1);
    let carry = Carry::parse(&file).expect("the carry file");
    let psbt = Psbt::parse_bytes(&carry.psbt).expect("the carried PSBT");
    assert_eq!(
        review(&psbt, &dealt, &policy, 1, None),
        vec![WarningKind::ThresholdRefused(
            osk_psbt::threshold::Refusal::NoSection
        )]
    );
    assert_eq!(
        review(&funded(), &dealt, &policy, 1, Some(&carry.section)),
        vec![WarningKind::ThresholdRefused(
            osk_psbt::threshold::Refusal::NoNonces
        )]
    );
}

#[test]
fn the_wrong_number_of_chosen_shares_is_refused() {
    let dealt = fixture::dealt();
    let policy = policy();
    for others in [&[][..], &[1, 2][..], &[0][..]] {
        let mut psbt = funded();
        let refused = pass(&mut psbt, &dealt, &policy, 0, others, None);
        assert!(
            matches!(
                refused,
                Err(osk_psbt::Error::Unsupported { reason, .. })
                    if reason == osk_psbt::threshold::Refusal::ChosenCount.reason()
            ),
            "chose {others:?}"
        );
    }
}

#[test]
fn a_threshold_input_of_an_unregistered_group_is_blocked() {
    let dealt = fixture::dealt();
    let secp = Secp256k1::new();
    let pubshare = dealt.shares[0].public_share(&secp);
    let shares = [ShareRef { pubshare }];
    let ctx = Context {
        network: Network::Regtest,
        keys: &[],
        wallets: &[],
        musig_session: None,
        shares: &shares,
        carry: None,
    };
    let (file, _) = route(0, 1);
    let carry = Carry::parse(&file).expect("the carry file");
    let psbt = Psbt::parse_bytes(&carry.psbt).expect("the carried PSBT");
    let blocked: Vec<WarningKind> = inspect(&psbt, &ctx)
        .warnings
        .iter()
        .filter(|w| w.level == Level::Blocked)
        .map(|w| w.kind.clone())
        .collect();
    assert_eq!(blocked, vec![WarningKind::ThresholdWalletNotRegistered]);
}

#[test]
fn change_back_to_the_wallet_is_verified() {
    let dealt = fixture::dealt();
    let policy = policy();
    let secp = Secp256k1::new();
    let pubshare = dealt.shares[0].public_share(&secp);
    let wallets = [policy];
    let shares = [ShareRef { pubshare }];
    let ctx = Context {
        network: Network::Regtest,
        keys: &[],
        wallets: &wallets,
        musig_session: None,
        shares: &shares,
        carry: None,
    };
    let inspection = inspect(&funded(), &ctx);
    assert!(
        inspection
            .outputs
            .iter()
            .any(|o| matches!(o.kind, osk_psbt::OutputKind::WalletChange { change, .. } if change)),
        "Core's change output pays the wallet's own 1/i: {:?}",
        inspection
            .outputs
            .iter()
            .map(|o| &o.kind)
            .collect::<Vec<_>>()
    );
}
