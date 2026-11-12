//! MuSig2 (BIP-327, BIP-373, BIP-390): what a wallet talking to this
//! device sees when it spends a `tr(musig(…))` wallet.
//!
//! Two counterparts. Bitcoin Core 31.1 on regtest, whose PSBTs are the
//! `wallet-musig-*` fixtures: Core writes its nonce, the device signs
//! last with no session, and Core finishes — the accepted transaction is
//! recorded in `tools/vectors/psbt/README.md`. And a second signer
//! played here, which carries the rounds the whole way so that the
//! aggregate signature this crate builds finalizes the input.

use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::common::builder::funding_tx;
use crate::common::{ABANDON, ZOO, master};
use bitcoin::absolute::LockTime;
use bitcoin::bip32::{ChildNumber, DerivationPath, Xpub};
use bitcoin::hashes::Hash;
use bitcoin::secp256k1::{PublicKey, Secp256k1, SecretKey};
use bitcoin::taproot::TapTweakHash;
use bitcoin::transaction::Version;
use bitcoin::{Amount, OutPoint, ScriptBuf, Sequence, Transaction, TxIn, TxOut, Witness};
use faraday_bip::keys::{Fingerprint, MasterKey, Network, ScriptType};
use faraday_bip::musig::{self, PubNonce, Tweak};
use faraday_bip::policy::WalletPolicy;
use faraday_crypto::work::Compute;
use faraday_psbt::{
    Aux, Context, Error, KeyRef, Level, MusigRole, Nonce, OutputKind, Psbt, WarningKind, finalize,
    inspect, sign,
};

#[path = "../../examples/musig.rs"]
mod example;

const NET: Network = Network::Regtest;

fn vectors() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tools/vectors/psbt")
}

fn fixture(name: &str) -> Psbt {
    let c = Compute::unchecked();
    let text = std::fs::read_to_string(vectors().join(name)).expect("the fixture");
    Psbt::parse_base64(&c, text.trim()).expect("a PSBT")
}

fn core_policy() -> WalletPolicy {
    example::policy(&vectors())
}

/// The context a person has who loaded the "abandon … about" key on
/// regtest and registered the wallet Core built.
fn core_context<'a>(keys: &'a [KeyRef], wallets: &'a [WalletPolicy]) -> Context<'a> {
    Context {
        network: NET,
        keys,
        wallets,
        musig_session: None,
    }
}

/// Core's round-1 PSBT is signed last: the device writes its public
/// nonce and its partial signature for its own participant, and the
/// transaction is not finished, because Core still has to aggregate.
#[test]
fn the_device_signs_last_after_bitcoin_core() {
    let c = Compute::unchecked();
    let abandon = master(ABANDON, NET);
    let keys = [KeyRef::from_master(&c, &abandon, 0).unwrap()];
    let wallets = [core_policy()];
    let ctx = core_context(&keys, &wallets);
    let mut psbt = fixture(example::CORE_FIRST_NAME);

    let insp = inspect(&c, &psbt, &ctx);
    assert!(!insp.has_blocked(), "{:?}", insp.warnings);
    assert_eq!(insp.participating_keys, vec![abandon.fingerprint()]);
    assert_eq!(insp.musig.as_ref().map(|m| m.keys), Some(2));

    let out = sign(
        &c,
        &mut psbt,
        &[&abandon],
        &[abandon.fingerprint()],
        &ctx,
        &[],
        Nonce::LowR,
        Aux::Deterministic,
        &mut None,
        Some(&faraday_crypto::Zeroizing::new([0u8; 32])),
    )
    .unwrap();
    assert_eq!(out.signed_inputs.len(), 1);
    assert_eq!(out.signed_inputs[0].musig, Some(MusigRole::Partial));
    assert_eq!(out.signed_inputs[0].fingerprint, abandon.fingerprint());
    assert!(!out.complete, "Core still has to sign and aggregate");

    let input = &psbt.inner().inputs[0];
    let ours = faraday_psbt::musig::pub_nonces(input).unwrap();
    assert_eq!(ours.len(), 2, "Core's nonce and ours");
    let sigs = faraday_psbt::musig::partial_sigs(input).unwrap();
    assert_eq!(sigs.len(), 1);
    // What this device writes is keyed by the taproot output key, which
    // is what Core writes and reads.
    let established = inspect(&c, &psbt, &ctx).inputs[0].musig.clone().unwrap();
    assert_eq!(sigs[0].aggregate, established.output_key);
    assert!(
        established
            .ours
            .iter()
            .any(|p| p.key == sigs[0].participant),
        "the partial signature is for one of this device's participants"
    );
    assert!(input.tap_key_sig.is_none(), "nothing to aggregate yet");
}

/// The committed signed fixture is byte for byte what signing Core's
/// round-1 PSBT produces today, which is what the Core round trip in
/// `tools/vectors/psbt/README.md` was run against.
#[test]
fn the_committed_signed_fixture_is_what_the_device_writes() {
    let dir = vectors();
    let committed = std::fs::read_to_string(dir.join(example::CORE_FIRST_SIGNED_NAME))
        .expect("the signed fixture; run `just psbt-fixtures`");
    assert_eq!(
        committed,
        example::core_first_signed(&dir).to_base64(),
        "run `cargo run -p faraday-psbt --example musig -- tools/vectors/psbt`"
    );
}

/// The committed device-first fixtures are byte for byte what the two
/// rounds write today: round 1 over the funded PSBT, and round 2 over
/// Core's answer to it, which is the run recorded in
/// `tools/vectors/psbt/README.md`.
#[test]
fn the_committed_device_first_fixtures_are_what_the_device_writes() {
    let dir = vectors();
    for (name, built) in [
        (
            example::DEVICE_FIRST_NONCE_NAME,
            example::device_first_nonce(&dir).0.to_base64(),
        ),
        (
            example::DEVICE_FIRST_SIGNED_NAME,
            example::device_first_signed(&dir).to_base64(),
        ),
    ] {
        let committed = std::fs::read_to_string(dir.join(name))
            .unwrap_or_else(|e| panic!("{name}: {e}; run `just psbt-fixtures`"));
        assert_eq!(
            committed, built,
            "{name}: run `cargo run -p faraday-psbt --example musig -- tools/vectors/psbt`"
        );
    }
}

/// What the device wrote in round 2 is a finished transaction: the
/// aggregate is the key-path signature, and `finalize` extracts the
/// transaction Bitcoin Core accepted.
#[test]
fn the_device_first_signed_fixture_finalizes() {
    let mut psbt = fixture(example::DEVICE_FIRST_SIGNED_NAME);
    assert!(psbt.inner().inputs[0].tap_key_sig.is_some());
    let tx = finalize(&mut psbt).unwrap().expect("every input is final");
    assert_eq!(tx.input[0].witness.len(), 1, "one key-path signature");
}

/// Signing the same PSBT twice under the deterministic auxiliary
/// randomness gives the same partial signature. This holds for signing
/// last, which draws no nonce to keep; a nonce drawn in round 1 is new
/// every time it is drawn.
#[test]
fn signing_twice_gives_the_same_partial_signature() {
    let dir = vectors();
    assert_eq!(
        example::core_first_signed(&dir).to_base64(),
        example::core_first_signed(&dir).to_base64()
    );
}

/// The funded PSBT, before anyone's nonce: the device goes first. It
/// writes its own public nonce, signs nothing, and keeps the secret
/// nonce in a session for the transaction (§16.100 round 1).
#[test]
fn a_psbt_with_no_nonce_on_it_shares_this_devices_nonce() {
    let c = Compute::unchecked();
    let abandon = master(ABANDON, NET);
    let keys = [KeyRef::from_master(&c, &abandon, 0).unwrap()];
    let wallets = [core_policy()];
    let ctx = core_context(&keys, &wallets);
    let mut psbt = fixture(example::DEVICE_FIRST_NAME);

    let insp = inspect(&c, &psbt, &ctx);
    assert!(!insp.has_blocked(), "{:?}", insp.warnings);
    assert!(
        insp.inputs[0]
            .musig
            .as_ref()
            .is_some_and(|m| m.will_share_nonce && !m.nonce_stale)
    );

    let mut session = None;
    let out = sign(
        &c,
        &mut psbt,
        &[&abandon],
        &[abandon.fingerprint()],
        &ctx,
        &[],
        Nonce::LowR,
        Aux::Deterministic,
        &mut session,
        Some(&faraday_crypto::Zeroizing::new([3u8; 32])),
    )
    .unwrap();
    assert_eq!(out.nonces_shared, 1);
    assert!(out.signed_inputs.is_empty(), "nothing is signed in round 1");
    assert!(!out.complete);

    let input = &psbt.inner().inputs[0];
    let nonces = faraday_psbt::musig::pub_nonces(input).unwrap();
    assert_eq!(nonces.len(), 1, "this device's nonce and nobody else's");
    assert!(faraday_psbt::musig::partial_sigs(input).unwrap().is_empty());
    let session = session.expect("the session is open");
    assert_eq!(session.len(), 1, "one secret nonce, for the one input");
    assert_eq!(session.txid(), psbt.unsigned_tx().compute_txid());
    let established = inspect(&c, &psbt, &ctx).inputs[0].musig.clone().unwrap();
    assert!(session.holds(0, &established.ours[0].key));
}

/// With no seed, a round that would draw a nonce is refused and nothing
/// is written or kept.
#[test]
fn a_round_that_draws_a_nonce_is_refused_with_no_seed() {
    let c = Compute::unchecked();
    let abandon = master(ABANDON, NET);
    let keys = [KeyRef::from_master(&c, &abandon, 0).unwrap()];
    let wallets = [core_policy()];
    let ctx = core_context(&keys, &wallets);
    let mut psbt = fixture(example::DEVICE_FIRST_NAME);
    let before = psbt.to_bytes();
    let mut session = None;
    let out = sign(
        &c,
        &mut psbt,
        &[&abandon],
        &[abandon.fingerprint()],
        &ctx,
        &[],
        Nonce::LowR,
        Aux::Deterministic,
        &mut session,
        None,
    );
    assert!(matches!(out, Err(faraday_psbt::Error::NoNonceSeed)));
    assert_eq!(psbt.to_bytes(), before);
    assert!(session.is_none());
}

/// The change output of the funded PSBT is the wallet's own address at
/// the chain and index its key origins name, so it is verified change
/// and not a payment to others.
#[test]
fn the_musig_change_output_is_verified() {
    let c = Compute::unchecked();
    let abandon = master(ABANDON, NET);
    let keys = [KeyRef::from_master(&c, &abandon, 0).unwrap()];
    let wallets = [core_policy()];
    let ctx = core_context(&keys, &wallets);
    let insp = inspect(&c, &fixture(example::DEVICE_FIRST_NAME), &ctx);
    let change = insp
        .outputs
        .iter()
        .find(|o| matches!(o.kind, OutputKind::WalletChange { .. }))
        .expect("one output is the wallet's change");
    assert!(change.amount > Amount::ZERO);
    assert_eq!(insp.change_total, change.amount);
}

/// Without the wallet registered, the participant set is nobody's, and
/// the input is blocked rather than signed for a set the coordinator
/// chose.
#[test]
fn an_unregistered_musig_wallet_is_blocked() {
    let c = Compute::unchecked();
    let abandon = master(ABANDON, NET);
    let keys = [KeyRef::from_master(&c, &abandon, 0).unwrap()];
    let ctx = core_context(&keys, &[]);
    let insp = inspect(&c, &fixture(example::CORE_FIRST_NAME), &ctx);
    assert!(
        insp.blocked()
            .contains(&WarningKind::MusigWalletNotRegistered),
        "{:?}",
        insp.warnings
    );
}

/// The fields this crate writes survive the binary encoding byte for
/// byte, which is what carrying them in the `unknown` maps is for.
#[test]
fn the_fields_survive_a_round_trip() {
    let c = Compute::unchecked();
    let signed = fixture(example::CORE_FIRST_SIGNED_NAME);
    let again = Psbt::parse_bytes(&c, &signed.to_bytes()).unwrap();
    assert_eq!(signed.to_bytes(), again.to_bytes());
    let input = &again.inner().inputs[0];
    assert_eq!(faraday_psbt::musig::pub_nonces(input).unwrap().len(), 2);
    assert_eq!(faraday_psbt::musig::partial_sigs(input).unwrap().len(), 1);
    assert_eq!(
        faraday_psbt::musig::input_participants(input).unwrap()[0]
            .keys
            .len(),
        2
    );
}

/// A nonce or partial-signature key with a tapleaf hash on it is a
/// script-path MuSig2 spend, which is not a wallet this tree loads.
#[test]
fn a_script_path_musig_field_is_refused() {
    let c = Compute::unchecked();
    let mut psbt = fixture(example::CORE_FIRST_SIGNED_NAME);
    let input = &mut psbt.inner_mut().inputs[0];
    let (key, value) = input
        .unknown
        .iter()
        .find(|(k, _)| k.type_value == faraday_psbt::musig::IN_PUB_NONCE)
        .map(|(k, v)| (k.clone(), v.clone()))
        .unwrap();
    let mut longer = key.clone();
    longer.key.extend_from_slice(&[0u8; 32]);
    input.unknown.remove(&key);
    input.unknown.insert(longer, value);
    assert!(faraday_psbt::musig::pub_nonces(&psbt.inner().inputs[0]).is_err());

    let abandon = master(ABANDON, NET);
    let keys = [KeyRef::from_master(&c, &abandon, 0).unwrap()];
    let wallets = [core_policy()];
    let ctx = core_context(&keys, &wallets);
    assert!(inspect(&c, &psbt, &ctx).has_blocked());
}

/// A malformed participants field blocks the input with a reason instead
/// of panicking.
#[test]
fn a_malformed_participants_field_blocks_the_input() {
    let c = Compute::unchecked();
    let mut psbt = fixture(example::CORE_FIRST_NAME);
    let input = &mut psbt.inner_mut().inputs[0];
    let key = input
        .unknown
        .keys()
        .find(|k| k.type_value == faraday_psbt::musig::IN_PARTICIPANT_PUBKEYS)
        .cloned()
        .unwrap();
    input.unknown.insert(key, vec![0u8; 20]);
    let abandon = master(ABANDON, NET);
    let keys = [KeyRef::from_master(&c, &abandon, 0).unwrap()];
    let wallets = [core_policy()];
    let ctx = core_context(&keys, &wallets);
    assert!(inspect(&c, &psbt, &ctx).has_blocked());
}

// ---------------------------------------------------------------------
// A second signer, played here, so that the rounds run the whole way
// ---------------------------------------------------------------------

/// The account path a `musig()` participant's origin names.
fn account_path() -> DerivationPath {
    DerivationPath::from(vec![
        ChildNumber::from_hardened_idx(86).unwrap(),
        ChildNumber::from_hardened_idx(1).unwrap(),
        ChildNumber::from_hardened_idx(0).unwrap(),
    ])
}

fn account_xpub(key: &MasterKey) -> Xpub {
    let c = Compute::unchecked();
    *key.account_xpub(&c, ScriptType::Taproot, 0).unwrap().xpub()
}

/// The two-key wallet the two seeds make, `@0` first.
fn two_key_policy(a: &MasterKey, b: &MasterKey) -> WalletPolicy {
    let c = Compute::unchecked();
    let text = |k: &MasterKey| format!("[{}/86'/1'/0']{}", k.fingerprint(), account_xpub(k));
    WalletPolicy::from_parts(&c, "tr(musig(@0,@1)/**)", &[&text(a), &text(b)]).unwrap()
}

/// The tweaks from the root aggregate to the output key at `chain/index`,
/// and the internal key on the way.
fn tweaks_at(participants: &[PublicKey], change: bool, index: u32) -> (Vec<Tweak>, PublicKey) {
    let secp = Secp256k1::verification_only();
    let mut xpub = musig::aggregate_xpub(participants, NET).unwrap();
    let mut tweaks = Vec::new();
    for step in [u32::from(change), index] {
        let child = ChildNumber::from_normal_idx(step).unwrap();
        let (tweak, _) = xpub.ckd_pub_tweak(child).unwrap();
        tweaks.push(Tweak {
            bytes: tweak.secret_bytes(),
            x_only: false,
        });
        xpub = xpub.ckd_pub(&secp, child).unwrap();
    }
    let internal = xpub.public_key;
    tweaks.push(Tweak {
        bytes: TapTweakHash::from_key_and_tweak(internal.x_only_public_key().0, None)
            .to_byte_array(),
        x_only: true,
    });
    (tweaks, internal)
}

/// A spend of the two-key wallet at receive index 0, with the fields a
/// coordinator that speaks BIP-373 fills in.
fn two_key_psbt(policy: &WalletPolicy, a: &MasterKey, b: &MasterKey) -> Psbt {
    let c = Compute::unchecked();
    let secp = Secp256k1::verification_only();
    let spk = policy.script_at(&c, false, 0).unwrap();
    let funding = funding_tx(spk.clone(), 100_000);
    let recipient = b
        .account_xpub(&c, ScriptType::NativeSegwit, 0)
        .unwrap()
        .address(false, 7)
        .unwrap()
        .script_pubkey();
    let tx = Transaction {
        version: Version::TWO,
        lock_time: LockTime::ZERO,
        input: vec![TxIn {
            previous_output: OutPoint {
                txid: funding.compute_txid(),
                vout: 0,
            },
            script_sig: ScriptBuf::new(),
            sequence: Sequence::ENABLE_RBF_NO_LOCKTIME,
            witness: Witness::new(),
        }],
        output: vec![TxOut {
            value: Amount::from_sat(99_000),
            script_pubkey: recipient,
        }],
    };
    let mut inner = bitcoin::Psbt::from_unsigned_tx(tx).unwrap();
    inner.inputs[0].witness_utxo = Some(funding.output[0].clone());

    let participants = musig::sort_keys(&[account_xpub(a).public_key, account_xpub(b).public_key]);
    let (_, internal) = tweaks_at(&participants, false, 0);
    let internal = internal.x_only_public_key().0;
    inner.inputs[0].tap_internal_key = Some(internal);

    let aggregate_xpub = musig::aggregate_xpub(&participants, NET).unwrap();
    let mut origins = BTreeMap::new();
    origins.insert(
        internal,
        (
            Vec::new(),
            (
                aggregate_xpub.fingerprint(),
                DerivationPath::from(vec![
                    ChildNumber::from_normal_idx(0).unwrap(),
                    ChildNumber::from_normal_idx(0).unwrap(),
                ]),
            ),
        ),
    );
    for key in [a, b] {
        origins.insert(
            account_xpub(key).public_key.x_only_public_key().0,
            (
                Vec::new(),
                (
                    bitcoin::bip32::Fingerprint::from(key.fingerprint().0),
                    account_path(),
                ),
            ),
        );
    }
    inner.inputs[0].tap_key_origins = origins;

    let root = musig::key_agg_with(&secp, &participants).unwrap();
    faraday_psbt::musig::write_input_participants(
        &mut inner.inputs[0],
        &faraday_psbt::musig::Participants {
            aggregate: root.public_key(),
            keys: participants,
        },
    );
    Psbt::from(inner)
}

/// The account-level secret key of `master`, which is what a `musig()`
/// participant signs with.
fn account_secret(master: &MasterKey) -> SecretKey {
    let c = Compute::unchecked();
    *master.derive(&c, &account_path()).secret_key()
}

/// A second signer's round 1: its public nonce, and the secret nonce it
/// keeps for round 2.
fn other_nonce(
    master: &MasterKey,
    aggregate: &[u8; 32],
    msg: &[u8],
    seed: u8,
) -> (musig::SecNonce, PubNonce) {
    let secp = Secp256k1::new();
    let sk = account_secret(master);
    musig::nonce_gen(
        &secp,
        &[seed; 32],
        Some(&sk),
        &account_xpub(master).public_key,
        Some(aggregate),
        Some(msg),
        None,
    )
    .unwrap()
}

/// The whole flow with a second signer that is not Bitcoin Core: the
/// other signer's nonce arrives, the device signs last, the other signer
/// signs, and the device aggregates, finalizes and extracts.
#[test]
fn a_second_signer_and_the_device_finish_the_transaction() {
    let c = Compute::unchecked();
    let secp = Secp256k1::new();
    let abandon = master(ABANDON, NET);
    let zoo = master(ZOO, NET);
    let policy = two_key_policy(&abandon, &zoo);
    let mut psbt = two_key_psbt(&policy, &abandon, &zoo);

    let keys = [KeyRef::from_master(&c, &abandon, 0).unwrap()];
    let wallets = [policy];
    let ctx = core_context(&keys, &wallets);

    // With nobody's nonce on it, a pass would go first and share one.
    assert!(
        inspect(&c, &psbt, &ctx).inputs[0]
            .musig
            .as_ref()
            .is_some_and(|m| m.will_share_nonce)
    );

    let established = {
        let mut ours = psbt.clone();
        let _ = &mut ours;
        inspect(&c, &psbt, &ctx).inputs[0].musig.clone().unwrap()
    };
    let msg = key_path_sighash(&psbt);

    // The other signer's round 1.
    let (secnonce, pubnonce) = other_nonce(
        &zoo,
        &established.output_key.x_only_public_key().0.serialize(),
        &msg,
        0x11,
    );
    let zoo_key = account_xpub(&zoo).public_key;
    faraday_psbt::musig::write_pub_nonce(
        &mut psbt.inner_mut().inputs[0],
        &zoo_key,
        &established.output_key,
        &pubnonce,
    );

    // The device signs last.
    let out = sign(
        &c,
        &mut psbt,
        &[&abandon],
        &[abandon.fingerprint()],
        &ctx,
        &[],
        Nonce::LowR,
        Aux::Deterministic,
        &mut None,
        Some(&faraday_crypto::Zeroizing::new([0u8; 32])),
    )
    .unwrap();
    assert_eq!(out.signed_inputs.len(), 1);
    assert_eq!(out.signed_inputs[0].musig, Some(MusigRole::Partial));
    assert!(!out.complete);

    // The other signer's round 2, over the aggregate nonce both are in.
    let nonces = faraday_psbt::musig::pub_nonces(&psbt.inner().inputs[0]).unwrap();
    let all: Vec<PubNonce> = established
        .participants
        .iter()
        .map(|p| nonces.iter().find(|n| n.participant == *p).unwrap().value)
        .collect();
    let aggnonce = musig::nonce_agg(&all).unwrap();
    let session = musig::session_values(
        &secp,
        &established.participants,
        &established.tweaks,
        &aggnonce,
        &msg,
    )
    .unwrap();
    let psig = musig::sign(&secp, secnonce, &account_secret(&zoo), &session).unwrap();
    faraday_psbt::musig::write_partial_sig(
        &mut psbt.inner_mut().inputs[0],
        &zoo_key,
        &established.output_key,
        &psig,
    );

    // The device reads it back, aggregates, and the input finalizes.
    let out = sign(
        &c,
        &mut psbt,
        &[&abandon],
        &[abandon.fingerprint()],
        &ctx,
        &[],
        Nonce::LowR,
        Aux::Deterministic,
        &mut None,
        Some(&faraday_crypto::Zeroizing::new([0u8; 32])),
    )
    .unwrap();
    assert_eq!(out.signed_inputs.len(), 1);
    assert_eq!(out.signed_inputs[0].musig, Some(MusigRole::Aggregated));
    assert!(out.complete);
    assert!(psbt.inner().inputs[0].tap_key_sig.is_some());
    let tx = finalize(&mut psbt).unwrap().expect("every input is final");
    assert_eq!(tx.input[0].witness.len(), 1, "one key-path signature");
}

/// A partial signature of another participant that does not verify names
/// that participant, so the person knows who has to sign again.
#[test]
fn a_bad_partial_signature_names_its_participant() {
    let c = Compute::unchecked();
    let abandon = master(ABANDON, NET);
    let zoo = master(ZOO, NET);
    let policy = two_key_policy(&abandon, &zoo);
    let mut psbt = two_key_psbt(&policy, &abandon, &zoo);
    let keys = [KeyRef::from_master(&c, &abandon, 0).unwrap()];
    let wallets = [policy];
    let ctx = core_context(&keys, &wallets);
    let established = inspect(&c, &psbt, &ctx).inputs[0].musig.clone().unwrap();
    let msg = key_path_sighash(&psbt);
    let (_, pubnonce) = other_nonce(
        &zoo,
        &established.output_key.x_only_public_key().0.serialize(),
        &msg,
        0x22,
    );
    let zoo_key = account_xpub(&zoo).public_key;
    faraday_psbt::musig::write_pub_nonce(
        &mut psbt.inner_mut().inputs[0],
        &zoo_key,
        &established.output_key,
        &pubnonce,
    );
    // A partial signature that is a scalar but not this signer's.
    let bogus = faraday_bip::musig::PartialSig::from_bytes(&[7u8; 32]).unwrap();
    faraday_psbt::musig::write_partial_sig(
        &mut psbt.inner_mut().inputs[0],
        &zoo_key,
        &established.output_key,
        &bogus,
    );
    let err = sign(
        &c,
        &mut psbt,
        &[&abandon],
        &[abandon.fingerprint()],
        &ctx,
        &[],
        Nonce::LowR,
        Aux::Deterministic,
        &mut None,
        Some(&faraday_crypto::Zeroizing::new([0u8; 32])),
    )
    .unwrap_err();
    match err {
        Error::MusigPartialSig { participant, .. } => {
            assert_eq!(participant, hex(&zoo_key.serialize()));
        }
        other => panic!("{other:?}"),
    }
}

/// The BIP-341 key-path sighash of the one input, under
/// `SIGHASH_DEFAULT`.
fn key_path_sighash(psbt: &Psbt) -> [u8; 32] {
    let tx = psbt.inner().unsigned_tx.clone();
    let prevouts: Vec<TxOut> = psbt
        .inner()
        .inputs
        .iter()
        .map(|i| i.witness_utxo.clone().unwrap())
        .collect();
    let mut cache = bitcoin::sighash::SighashCache::new(&tx);
    cache
        .taproot_key_spend_signature_hash(
            0,
            &bitcoin::sighash::Prevouts::All(&prevouts),
            bitcoin::sighash::TapSighashType::Default,
        )
        .unwrap()
        .to_byte_array()
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// The review reads a MuSig2 input as the wallet it is: a warning-free
/// inspection with the wallet's key count and no quorum.
#[test]
fn the_review_names_the_musig_wallet() {
    let c = Compute::unchecked();
    let abandon = master(ABANDON, NET);
    let keys = [KeyRef::from_master(&c, &abandon, 0).unwrap()];
    let wallets = [core_policy()];
    let ctx = core_context(&keys, &wallets);
    let insp = inspect(&c, &fixture(example::CORE_FIRST_NAME), &ctx);
    let musig = insp.musig.as_ref().unwrap();
    assert_eq!(musig.keys, 2);
    assert_eq!(musig.partial_signatures, 0);
    assert_eq!(musig.ours, vec![abandon.fingerprint()]);
    assert!(insp.multisig.is_none());
    assert_eq!(wallets[0].quorum(), None);
    assert!(
        insp.warnings.iter().all(|w| w.level < Level::Danger),
        "{:?}",
        insp.warnings
    );
    let _: Fingerprint = abandon.fingerprint();
}

// ---------------------------------------------------------------------
// The device first, with the session
// ---------------------------------------------------------------------

/// A context with the session this device holds, which is what decides
/// whether a nonce of ours on a PSBT is one we can still sign under.
fn context_with<'a>(
    keys: &'a [KeyRef],
    wallets: &'a [WalletPolicy],
    session: Option<&'a faraday_psbt::MusigSessionView>,
) -> Context<'a> {
    Context {
        network: NET,
        keys,
        wallets,
        musig_session: session,
    }
}

/// Signs `psbt` with `session` open, for the keys given.
fn pass(
    psbt: &mut Psbt,
    masters: &[&MasterKey],
    ctx: &Context,
    session: &mut Option<faraday_psbt::MusigSession>,
    seed: u8,
) -> faraday_psbt::SignResult {
    let c = Compute::unchecked();
    let selection: Vec<Fingerprint> = masters.iter().map(|m| m.fingerprint()).collect();
    sign(
        &c,
        psbt,
        masters,
        &selection,
        ctx,
        &[],
        Nonce::LowR,
        Aux::Deterministic,
        session,
        Some(&faraday_crypto::Zeroizing::new([seed; 32])),
    )
    .expect("the pass runs")
}

/// The whole flow with the device going first: round 1 writes this
/// device's nonce and keeps the secret, the other signer answers with
/// its own nonce and partial signature, and round 2 with the same
/// session signs, aggregates and finalizes, leaving the session empty.
#[test]
fn the_device_goes_first_and_finishes_when_the_transaction_returns() {
    let c = Compute::unchecked();
    let secp = Secp256k1::new();
    let abandon = master(ABANDON, NET);
    let zoo = master(ZOO, NET);
    let policy = two_key_policy(&abandon, &zoo);
    let mut psbt = two_key_psbt(&policy, &abandon, &zoo);
    let keys = [KeyRef::from_master(&c, &abandon, 0).unwrap()];
    let wallets = [policy];

    let mut session = None;
    let out = {
        let ctx = context_with(&keys, &wallets, None);
        pass(&mut psbt, &[&abandon], &ctx, &mut session, 0x41)
    };
    assert_eq!(out.nonces_shared, 1);
    assert!(out.signed_inputs.is_empty());
    let input = &psbt.inner().inputs[0];
    assert_eq!(faraday_psbt::musig::pub_nonces(input).unwrap().len(), 1);
    assert!(faraday_psbt::musig::partial_sigs(input).unwrap().is_empty());
    assert_eq!(session.as_ref().unwrap().len(), 1);

    // The other signer answers with its nonce and, seeing both, its
    // partial signature.
    let established = {
        let view = session.as_ref().unwrap().view();
        let ctx = context_with(&keys, &wallets, Some(&view));
        inspect(&c, &psbt, &ctx).inputs[0].musig.clone().unwrap()
    };
    let msg = key_path_sighash(&psbt);
    let (secnonce, pubnonce) = other_nonce(
        &zoo,
        &established.output_key.x_only_public_key().0.serialize(),
        &msg,
        0x31,
    );
    let zoo_key = account_xpub(&zoo).public_key;
    faraday_psbt::musig::write_pub_nonce(
        &mut psbt.inner_mut().inputs[0],
        &zoo_key,
        &established.output_key,
        &pubnonce,
    );
    let nonces = faraday_psbt::musig::pub_nonces(&psbt.inner().inputs[0]).unwrap();
    let all: Vec<PubNonce> = established
        .participants
        .iter()
        .map(|p| nonces.iter().find(|n| n.participant == *p).unwrap().value)
        .collect();
    let values = musig::session_values(
        &secp,
        &established.participants,
        &established.tweaks,
        &musig::nonce_agg(&all).unwrap(),
        &msg,
    )
    .unwrap();
    let psig = musig::sign(&secp, secnonce, &account_secret(&zoo), &values).unwrap();
    faraday_psbt::musig::write_partial_sig(
        &mut psbt.inner_mut().inputs[0],
        &zoo_key,
        &established.output_key,
        &psig,
    );

    // Round 2 with the same session.
    let out = {
        let view = session.as_ref().unwrap().view();
        let ctx = context_with(&keys, &wallets, Some(&view));
        pass(&mut psbt, &[&abandon], &ctx, &mut session, 0x41)
    };
    assert_eq!(out.nonces_shared, 0);
    assert_eq!(out.signed_inputs.len(), 2, "the partial and the aggregate");
    assert_eq!(out.signed_inputs[0].musig, Some(MusigRole::Partial));
    assert_eq!(out.signed_inputs[1].musig, Some(MusigRole::Aggregated));
    assert!(out.complete);
    assert!(
        session.as_ref().unwrap().is_empty(),
        "the secret nonce is gone once it has signed"
    );

    // A second pass over the same PSBT draws no nonce and makes no
    // second partial signature: the secret nonce it would need is gone,
    // and BIP-327 says a participant signs once.
    let mut again = Psbt::parse_bytes(&c, &psbt.to_bytes()).unwrap();
    {
        let view = session.as_ref().unwrap().view();
        let ctx = context_with(&keys, &wallets, Some(&view));
        let out = pass(&mut again, &[&abandon], &ctx, &mut session, 0x41);
        assert_eq!(out.nonces_shared, 0);
        assert!(
            out.signed_inputs
                .iter()
                .all(|s| s.musig == Some(MusigRole::Aggregated)),
            "nothing but the aggregate the input already carries"
        );
    }
    assert_eq!(again.to_bytes(), psbt.to_bytes(), "and nothing changed");

    let tx = finalize(&mut psbt).unwrap().expect("every input is final");
    assert_eq!(tx.input[0].witness.len(), 1, "one key-path signature");
}

/// A session is for one transaction: sharing a nonce for another
/// replaces it, and what the first one held is gone.
#[test]
fn a_session_for_another_transaction_replaces_this_one() {
    let c = Compute::unchecked();
    let abandon = master(ABANDON, NET);
    let zoo = master(ZOO, NET);
    let policy = two_key_policy(&abandon, &zoo);
    let keys = [KeyRef::from_master(&c, &abandon, 0).unwrap()];
    let wallets = [policy.clone()];
    let ctx = context_with(&keys, &wallets, None);

    let mut first = two_key_psbt(&policy, &abandon, &zoo);
    let mut session = None;
    pass(&mut first, &[&abandon], &ctx, &mut session, 0x51);
    let first_txid = session.as_ref().unwrap().txid();

    let mut second = two_key_psbt(&policy, &abandon, &zoo);
    second.inner_mut().unsigned_tx.lock_time = LockTime::from_height(1).unwrap();
    pass(&mut second, &[&abandon], &ctx, &mut session, 0x52);
    let open = session.as_ref().unwrap();
    assert_eq!(open.txid(), second.unsigned_tx().compute_txid());
    assert_ne!(open.txid(), first_txid);
    assert_eq!(open.len(), 1, "only the new transaction's nonce is held");
}

/// A PSBT carrying a public nonce of this device's that no session holds
/// is signed afresh: the review says the nonce is replaced, the round
/// writes a new one, and the partial signatures made against the old one
/// are dropped.
#[test]
fn a_nonce_no_session_holds_is_replaced_and_earlier_signatures_dropped() {
    let c = Compute::unchecked();
    let abandon = master(ABANDON, NET);
    let zoo = master(ZOO, NET);
    let policy = two_key_policy(&abandon, &zoo);
    let keys = [KeyRef::from_master(&c, &abandon, 0).unwrap()];
    let wallets = [policy.clone()];
    let mut psbt = two_key_psbt(&policy, &abandon, &zoo);

    // Round 1, and then the session is gone: the device locked, or was
    // powered off.
    let mut session = None;
    {
        let ctx = context_with(&keys, &wallets, None);
        pass(&mut psbt, &[&abandon], &ctx, &mut session, 0x61);
    }
    let stale = faraday_psbt::musig::pub_nonces(&psbt.inner().inputs[0]).unwrap()[0].value;
    session = None;

    // The other signer answers, against the nonce that is now stale.
    let ctx = context_with(&keys, &wallets, None);
    let established = inspect(&c, &psbt, &ctx).inputs[0].musig.clone().unwrap();
    let msg = key_path_sighash(&psbt);
    let (_, pubnonce) = other_nonce(
        &zoo,
        &established.output_key.x_only_public_key().0.serialize(),
        &msg,
        0x62,
    );
    let zoo_key = account_xpub(&zoo).public_key;
    faraday_psbt::musig::write_pub_nonce(
        &mut psbt.inner_mut().inputs[0],
        &zoo_key,
        &established.output_key,
        &pubnonce,
    );
    faraday_psbt::musig::write_partial_sig(
        &mut psbt.inner_mut().inputs[0],
        &zoo_key,
        &established.output_key,
        &faraday_bip::musig::PartialSig::from_bytes(&[9u8; 32]).unwrap(),
    );

    let insp = inspect(&c, &psbt, &ctx);
    assert!(
        insp.warnings
            .iter()
            .any(|w| w.kind == WarningKind::MusigNonceReplaced && w.level == Level::Caution),
        "{:?}",
        insp.warnings
    );
    assert!(!insp.has_blocked(), "{:?}", insp.warnings);

    let out = pass(&mut psbt, &[&abandon], &ctx, &mut session, 0x63);
    assert_eq!(out.nonces_shared, 1);
    let input = &psbt.inner().inputs[0];
    assert!(
        faraday_psbt::musig::partial_sigs(input).unwrap().is_empty(),
        "the other signer's partial signature went with the nonce it was made against"
    );
    let ours = faraday_psbt::musig::pub_nonces(input)
        .unwrap()
        .into_iter()
        .find(|n| n.participant == established.ours[0].key)
        .unwrap();
    assert_ne!(ours.value, stale, "a new nonce, not the one it replaced");
}

/// The same round over a version 2 file: the nonce of this device's the
/// file carries is replaced by the new one and the partial signatures
/// made against the old one go, while a changed nonce of the other signer
/// is still refused.
#[test]
fn a_version_2_file_takes_a_new_nonce_of_this_device_and_refuses_another_signers() {
    let c = Compute::unchecked();
    let abandon = master(ABANDON, NET);
    let zoo = master(ZOO, NET);
    let policy = two_key_policy(&abandon, &zoo);
    let keys = [KeyRef::from_master(&c, &abandon, 0).unwrap()];
    let wallets = [policy.clone()];
    let mut psbt = two_key_psbt(&policy, &abandon, &zoo);
    let mut session = None;
    let ctx = context_with(&keys, &wallets, None);
    pass(&mut psbt, &[&abandon], &ctx, &mut session, 0x61);
    session = None;

    let established = inspect(&c, &psbt, &ctx).inputs[0].musig.clone().unwrap();
    let msg = key_path_sighash(&psbt);
    let (_, pubnonce) = other_nonce(
        &zoo,
        &established.output_key.x_only_public_key().0.serialize(),
        &msg,
        0x62,
    );
    let zoo_key = account_xpub(&zoo).public_key;
    let input = &mut psbt.inner_mut().inputs[0];
    faraday_psbt::musig::write_pub_nonce(input, &zoo_key, &established.output_key, &pubnonce);
    faraday_psbt::musig::write_partial_sig(
        input,
        &zoo_key,
        &established.output_key,
        &faraday_bip::musig::PartialSig::from_bytes(&[9u8; 32]).unwrap(),
    );
    let file = faraday_psbt::v2::PsbtV2::from_v0(&c, &psbt);
    let old = faraday_psbt::musig::pub_nonces(&psbt.inner().inputs[0]).unwrap();

    let own = pass(&mut psbt, &[&abandon], &ctx, &mut session, 0x63).nonces_written;

    let mut without = file.clone();
    assert_eq!(
        without.merge_signed(&psbt),
        Err(faraday_psbt::v2::Error::Mismatch)
    );

    let mut merged = file.clone();
    assert!(merged.merge_signed_by(&psbt, &own).is_ok());
    let back = merged.to_v0(&c).unwrap();
    let new = faraday_psbt::musig::pub_nonces(&psbt.inner().inputs[0]).unwrap();
    let got = faraday_psbt::musig::pub_nonces(&back.inner().inputs[0]).unwrap();
    let ours = |v: &[faraday_psbt::musig::Entry<faraday_bip::musig::PubNonce>]| {
        v.iter()
            .find(|n| n.participant == established.ours[0].key)
            .unwrap()
            .value
    };
    assert_ne!(ours(&old), ours(&new));
    assert_eq!(ours(&got), ours(&new));
    assert!(
        faraday_psbt::musig::partial_sigs(&back.inner().inputs[0])
            .unwrap()
            .is_empty()
    );

    // The other signer's nonce changed in the signed copy: refused.
    let mut forged = psbt.clone();
    let (_, other) = other_nonce(
        &zoo,
        &established.output_key.x_only_public_key().0.serialize(),
        &msg,
        0x64,
    );
    faraday_psbt::musig::write_pub_nonce(
        &mut forged.inner_mut().inputs[0],
        &zoo_key,
        &established.output_key,
        &other,
    );
    let mut refused = file.clone();
    assert_eq!(
        refused.merge_signed_by(&forged, &own),
        Err(faraday_psbt::v2::Error::Mismatch)
    );
    assert_eq!(refused, file, "nothing changed");

    // The same, with the file's label for the other signer's key
    // changed to this device's fingerprint: a label widens nothing.
    for (_, (_, (fp, _))) in forged.inner_mut().inputs[0]
        .tap_key_origins
        .iter_mut()
        .filter(|(x, _)| **x == zoo_key.x_only_public_key().0)
    {
        *fp = (*abandon.fingerprint().as_bytes()).into();
    }
    let mut relabelled = file.clone();
    assert_eq!(
        relabelled.merge_signed_by(&forged, &own),
        Err(faraday_psbt::v2::Error::Mismatch)
    );
    assert_eq!(relabelled, file, "nothing changed");

    // A nonce of this device's under another aggregate key than the one
    // it was made for is not replaced: the new nonce is written only
    // under the exact key it was made for.
    let mut elsewhere = psbt.clone();
    let mut elsewhere_file = file.to_v0(&c).unwrap();
    let ours_key = established.ours[0].key;
    faraday_psbt::musig::write_pub_nonce(
        &mut elsewhere_file.inner_mut().inputs[0],
        &ours_key,
        &zoo_key,
        &ours(&old),
    );
    faraday_psbt::musig::write_pub_nonce(
        &mut elsewhere.inner_mut().inputs[0],
        &ours_key,
        &zoo_key,
        &ours(&new),
    );
    let elsewhere_file = faraday_psbt::v2::PsbtV2::from_v0(&c, &elsewhere_file);
    let mut other_key = elsewhere_file.clone();
    assert_eq!(
        other_key.merge_signed_by(&elsewhere, &own),
        Err(faraday_psbt::v2::Error::Mismatch)
    );
    assert_eq!(other_key, elsewhere_file, "nothing changed");
}

/// A cosigner that strips this device's partial signature and changes
/// its own nonce, while the session stays open under one seed, gets a
/// new nonce from this device: two partial signatures under one secret
/// nonce would give the key away.
#[test]
fn a_nonce_drawn_again_in_one_session_is_a_new_nonce() {
    let c = Compute::unchecked();
    let abandon = master(ABANDON, NET);
    let zoo = master(ZOO, NET);
    let policy = two_key_policy(&abandon, &zoo);
    let keys = [KeyRef::from_master(&c, &abandon, 0).unwrap()];
    let wallets = [policy.clone()];
    let mut psbt = two_key_psbt(&policy, &abandon, &zoo);
    let ours_key = account_xpub(&abandon).public_key;
    let zoo_key = account_xpub(&zoo).public_key;
    let our_nonce = |p: &Psbt| {
        faraday_psbt::musig::pub_nonces(&p.inner().inputs[0])
            .unwrap()
            .into_iter()
            .find(|n| n.participant == ours_key)
            .unwrap()
            .value
    };

    let mut session = None;
    let ctx = context_with(&keys, &wallets, None);
    pass(&mut psbt, &[&abandon], &ctx, &mut session, 0x61);
    let first = our_nonce(&psbt);

    // The other signer answers and round 2 signs.
    let established = inspect(&c, &psbt, &ctx).inputs[0].musig.clone().unwrap();
    let msg = key_path_sighash(&psbt);
    let agg = established.output_key.x_only_public_key().0.serialize();
    let (_, other1) = other_nonce(&zoo, &agg, &msg, 0x71);
    faraday_psbt::musig::write_pub_nonce(
        &mut psbt.inner_mut().inputs[0],
        &zoo_key,
        &established.output_key,
        &other1,
    );
    {
        let view = session.as_ref().unwrap().view();
        let ctx = context_with(&keys, &wallets, Some(&view));
        let out = pass(&mut psbt, &[&abandon], &ctx, &mut session, 0x61);
        assert_eq!(out.signed_inputs.len(), 1);
    }

    // The transaction comes back with our partial signature gone and the
    // other signer's nonce changed, and the same seed is passed.
    faraday_psbt::musig::clear_partial_sigs(&mut psbt.inner_mut().inputs[0]);
    let (_, other2) = other_nonce(&zoo, &agg, &msg, 0x72);
    faraday_psbt::musig::write_pub_nonce(
        &mut psbt.inner_mut().inputs[0],
        &zoo_key,
        &established.output_key,
        &other2,
    );
    let out = {
        let view = session.as_ref().unwrap().view();
        let ctx = context_with(&keys, &wallets, Some(&view));
        pass(&mut psbt, &[&abandon], &ctx, &mut session, 0x61)
    };
    assert_eq!(out.nonces_shared, 1);
    assert_ne!(
        our_nonce(&psbt),
        first,
        "a new nonce for the second signature"
    );
}

/// Two of this device's keys in one wallet: neither can derive its nonce
/// from the other's, so one round 1 shares both and one round 2 signs
/// both.
#[test]
fn two_keys_of_one_wallet_share_and_sign_together() {
    let c = Compute::unchecked();
    let abandon = master(ABANDON, NET);
    let zoo = master(ZOO, NET);
    let policy = two_key_policy(&abandon, &zoo);
    let mut psbt = two_key_psbt(&policy, &abandon, &zoo);
    let keys = [
        KeyRef::from_master(&c, &abandon, 0).unwrap(),
        KeyRef::from_master(&c, &zoo, 0).unwrap(),
    ];
    let wallets = [policy];

    let mut session = None;
    let out = {
        let ctx = context_with(&keys, &wallets, None);
        pass(&mut psbt, &[&abandon, &zoo], &ctx, &mut session, 0x71)
    };
    assert_eq!(out.nonces_shared, 2, "one nonce per key of ours");
    assert!(out.signed_inputs.is_empty());
    assert_eq!(session.as_ref().unwrap().len(), 2);
    assert_eq!(
        faraday_psbt::musig::pub_nonces(&psbt.inner().inputs[0])
            .unwrap()
            .len(),
        2
    );

    let view = session.as_ref().unwrap().view();
    let ctx = context_with(&keys, &wallets, Some(&view));
    let out = pass(&mut psbt, &[&abandon, &zoo], &ctx, &mut session, 0x71);
    assert_eq!(out.nonces_shared, 0);
    assert_eq!(
        out.signed_inputs.len(),
        3,
        "a partial signature each, and the aggregate"
    );
    assert!(out.complete);
    assert!(session.as_ref().unwrap().is_empty());
    let tx = finalize(&mut psbt).unwrap().expect("every input is final");
    assert_eq!(tx.input[0].witness.len(), 1);
}

/// Both keys of a wallet are this device's. One shared its nonce in an
/// earlier pass, and this session holds it; the other's nonce on the
/// PSBT is one no session holds. The pass that holds both replaces the
/// second key's nonce and signs with the first, and every partial
/// signature it reports is on the PSBT it hands back, whichever key
/// went first.
#[test]
fn a_pass_that_shares_and_signs_reports_what_the_psbt_holds() {
    let c = Compute::unchecked();
    let abandon = master(ABANDON, NET);
    let zoo = master(ZOO, NET);
    let policy = two_key_policy(&abandon, &zoo);
    let both = [
        KeyRef::from_master(&c, &abandon, 0).unwrap(),
        KeyRef::from_master(&c, &zoo, 0).unwrap(),
    ];
    let wallets = [policy.clone()];
    for (first, second) in [(&abandon, &zoo), (&zoo, &abandon)] {
        let mut psbt = two_key_psbt(&policy, &abandon, &zoo);
        let alone = [KeyRef::from_master(&c, first, 0).unwrap()];
        let mut session = None;
        {
            let ctx = context_with(&alone, &wallets, None);
            let out = pass(&mut psbt, &[first], &ctx, &mut session, 0x81);
            assert_eq!(out.nonces_shared, 1);
        }
        // The second key's nonce, from a session that is gone.
        let established = {
            let ctx = context_with(&alone, &wallets, None);
            inspect(&c, &psbt, &ctx).inputs[0].musig.clone().unwrap()
        };
        let msg = key_path_sighash(&psbt);
        let (_, stale) = other_nonce(
            second,
            &established.output_key.x_only_public_key().0.serialize(),
            &msg,
            0x83,
        );
        faraday_psbt::musig::write_pub_nonce(
            &mut psbt.inner_mut().inputs[0],
            &account_xpub(second).public_key,
            &established.output_key,
            &stale,
        );

        let view = session.as_ref().unwrap().view();
        let ctx = context_with(&both, &wallets, Some(&view));
        let out = pass(&mut psbt, &[&abandon, &zoo], &ctx, &mut session, 0x82);
        assert_eq!(out.nonces_shared, 1, "the second key's nonce, replaced");
        let held: Vec<Vec<u8>> = faraday_psbt::musig::partial_sigs(&psbt.inner().inputs[0])
            .unwrap()
            .into_iter()
            .map(|e| e.value.serialize().to_vec())
            .collect();
        let reported: Vec<Vec<u8>> = out
            .signed_inputs
            .iter()
            .filter(|s| s.musig == Some(MusigRole::Partial))
            .map(|s| s.sig_bytes.clone())
            .collect();
        assert_eq!(reported, held, "first {}", first.fingerprint());
        assert!(!out.complete);
    }
}

// ---------------------------------------------------------------------
// A partial signature in the file is the file's word
// ---------------------------------------------------------------------

/// Everything a partial-signature test needs after round 1: the device
/// (abandon) has shared a nonce and holds the secret, and the other
/// signer (zoo) has answered with its nonce and its true partial
/// signature.
struct Round {
    psbt: Psbt,
    session: Option<faraday_psbt::MusigSession>,
    established: faraday_psbt::MusigInput,
    abandon_key: PublicKey,
    zoo_key: PublicKey,
    zoo_psig: faraday_bip::musig::PartialSig,
}

fn after_round_one(abandon: &MasterKey, zoo: &MasterKey) -> Round {
    let c = Compute::unchecked();
    let secp = Secp256k1::new();
    let policy = two_key_policy(abandon, zoo);
    let mut psbt = two_key_psbt(&policy, abandon, zoo);
    let keys = [KeyRef::from_master(&c, abandon, 0).unwrap()];
    let wallets = [policy];
    let mut session = None;
    {
        let ctx = context_with(&keys, &wallets, None);
        pass(&mut psbt, &[abandon], &ctx, &mut session, 0x41);
    }
    let established = {
        let view = session.as_ref().unwrap().view();
        let ctx = context_with(&keys, &wallets, Some(&view));
        inspect(&c, &psbt, &ctx).inputs[0].musig.clone().unwrap()
    };
    let msg = key_path_sighash(&psbt);
    let (secnonce, pubnonce) = other_nonce(
        zoo,
        &established.output_key.x_only_public_key().0.serialize(),
        &msg,
        0x31,
    );
    let zoo_key = account_xpub(zoo).public_key;
    faraday_psbt::musig::write_pub_nonce(
        &mut psbt.inner_mut().inputs[0],
        &zoo_key,
        &established.output_key,
        &pubnonce,
    );
    let nonces = faraday_psbt::musig::pub_nonces(&psbt.inner().inputs[0]).unwrap();
    let all: Vec<PubNonce> = established
        .participants
        .iter()
        .map(|p| nonces.iter().find(|n| n.participant == *p).unwrap().value)
        .collect();
    let values = musig::session_values(
        &secp,
        &established.participants,
        &established.tweaks,
        &musig::nonce_agg(&all).unwrap(),
        &msg,
    )
    .unwrap();
    let zoo_psig = musig::sign(&secp, secnonce, &account_secret(zoo), &values).unwrap();
    faraday_psbt::musig::write_partial_sig(
        &mut psbt.inner_mut().inputs[0],
        &zoo_key,
        &established.output_key,
        &zoo_psig,
    );
    Round {
        psbt,
        session,
        established,
        abandon_key: account_xpub(abandon).public_key,
        zoo_key,
        zoo_psig,
    }
}

fn write_partial(r: &mut Round, key: &PublicKey, psig: &faraday_bip::musig::PartialSig) {
    faraday_psbt::musig::write_partial_sig(
        &mut r.psbt.inner_mut().inputs[0],
        key,
        &r.established.output_key,
        psig,
    );
}

fn forged() -> faraday_bip::musig::PartialSig {
    faraday_bip::musig::PartialSig::from_bytes(&[5u8; 32]).unwrap()
}

/// Whether the signature the finished input carries verifies as BIP-340
/// under the output key.
fn aggregate_verifies(psbt: &Psbt, r: &faraday_psbt::MusigInput) -> bool {
    let secp = Secp256k1::verification_only();
    let Some(sig) = psbt.inner().inputs[0].tap_key_sig else {
        return false;
    };
    let msg = bitcoin::secp256k1::Message::from_digest(key_path_sighash(psbt));
    secp.verify_schnorr(&sig.signature, &msg, &r.output_key.x_only_public_key().0)
        .is_ok()
}

/// An entry for this participant in the file, while the device holds its
/// secret nonce, is false whatever it says: Sign overwrites it with the
/// kept nonce and the aggregate is a valid signature, with no nonce held
/// after.
#[test]
fn a_made_up_entry_for_a_participant_whose_nonce_is_held_is_overwritten() {
    let c = Compute::unchecked();
    let abandon = master(ABANDON, NET);
    let zoo = master(ZOO, NET);
    let policy = two_key_policy(&abandon, &zoo);
    let keys = [KeyRef::from_master(&c, &abandon, 0).unwrap()];
    let wallets = [policy];
    let mut r = after_round_one(&abandon, &zoo);
    let key = r.abandon_key;
    write_partial(&mut r, &key, &forged());

    let view = r.session.as_ref().unwrap().view();
    let ctx = context_with(&keys, &wallets, Some(&view));
    let out = pass(&mut r.psbt, &[&abandon], &ctx, &mut r.session, 0x41);
    assert!(out.complete);
    assert_eq!(out.nonces_shared, 0);
    assert!(aggregate_verifies(&r.psbt, &r.established));
    assert!(
        r.session.as_ref().unwrap().is_empty(),
        "no nonce held after"
    );
}

/// An entry that does not verify, with no nonce held (the device was
/// locked since round 1), is not the end of the round: Sign makes a new,
/// different nonce and clears the partial signatures.
#[test]
fn a_failing_entry_without_a_held_nonce_gets_a_new_nonce() {
    let c = Compute::unchecked();
    let abandon = master(ABANDON, NET);
    let zoo = master(ZOO, NET);
    let policy = two_key_policy(&abandon, &zoo);
    let keys = [KeyRef::from_master(&c, &abandon, 0).unwrap()];
    let wallets = [policy];
    let mut r = after_round_one(&abandon, &zoo);
    let key = r.abandon_key;
    write_partial(&mut r, &key, &forged());
    let before = faraday_psbt::musig::pub_nonces(&r.psbt.inner().inputs[0])
        .unwrap()
        .into_iter()
        .find(|n| n.participant == key)
        .unwrap()
        .value;
    r.session = None;

    let ctx = context_with(&keys, &wallets, None);
    let insp = inspect(&c, &r.psbt, &ctx);
    assert!(
        insp.warnings
            .iter()
            .any(|w| w.kind == WarningKind::MusigNonceReplaced),
        "{:?}",
        insp.warnings
    );
    let out = pass(&mut r.psbt, &[&abandon], &ctx, &mut r.session, 0x42);
    assert_eq!(out.nonces_shared, 1);
    let input = &r.psbt.inner().inputs[0];
    assert!(faraday_psbt::musig::partial_sigs(input).unwrap().is_empty());
    let after = faraday_psbt::musig::pub_nonces(input)
        .unwrap()
        .into_iter()
        .find(|n| n.participant == key)
        .unwrap()
        .value;
    assert_ne!(after, before);
    assert_eq!(r.session.as_ref().unwrap().len(), 1);
}

/// A pass that refuses because another signer's partial signature does
/// not verify takes no nonce: it is still held, and once the true
/// partial signature is back the next Sign succeeds.
#[test]
fn a_refused_pass_leaves_the_nonce_held() {
    let c = Compute::unchecked();
    let abandon = master(ABANDON, NET);
    let zoo = master(ZOO, NET);
    let policy = two_key_policy(&abandon, &zoo);
    let keys = [KeyRef::from_master(&c, &abandon, 0).unwrap()];
    let wallets = [policy];
    let mut r = after_round_one(&abandon, &zoo);
    let (zoo_key, zoo_psig) = (r.zoo_key, r.zoo_psig);
    write_partial(&mut r, &zoo_key, &forged());

    let view = r.session.as_ref().unwrap().view();
    let ctx = context_with(&keys, &wallets, Some(&view));
    let err = sign(
        &c,
        &mut r.psbt,
        &[&abandon],
        &[abandon.fingerprint()],
        &ctx,
        &[],
        Nonce::LowR,
        Aux::Deterministic,
        &mut r.session,
        Some(&faraday_crypto::Zeroizing::new([0x41u8; 32])),
    )
    .unwrap_err();
    assert!(matches!(err, Error::MusigPartialSig { .. }), "{err:?}");
    assert_eq!(r.session.as_ref().unwrap().len(), 1, "the nonce is held");

    write_partial(&mut r, &zoo_key, &zoo_psig);
    let view = r.session.as_ref().unwrap().view();
    let ctx = context_with(&keys, &wallets, Some(&view));
    let out = pass(&mut r.psbt, &[&abandon], &ctx, &mut r.session, 0x41);
    assert!(out.complete);
    assert!(aggregate_verifies(&r.psbt, &r.established));
}

/// `aggregate` finishes with a signature that verifies when every
/// partial signature is true, and does not finish when one is made up.
#[test]
fn aggregate_finishes_on_true_partials_and_not_on_a_made_up_one() {
    let c = Compute::unchecked();
    let abandon = master(ABANDON, NET);
    let zoo = master(ZOO, NET);
    let policy = two_key_policy(&abandon, &zoo);
    let keys = [KeyRef::from_master(&c, &abandon, 0).unwrap()];
    let wallets = [policy];
    let mut r = after_round_one(&abandon, &zoo);
    let view = r.session.as_ref().unwrap().view();
    let ctx = context_with(&keys, &wallets, Some(&view));
    pass(&mut r.psbt, &[&abandon], &ctx, &mut r.session, 0x41);

    let sig = faraday_psbt::musig::aggregate(&c, &r.psbt, 0, &r.established)
        .expect("every partial signature is true");
    let msg = bitcoin::secp256k1::Message::from_digest(key_path_sighash(&r.psbt));
    Secp256k1::verification_only()
        .verify_schnorr(
            &sig.signature,
            &msg,
            &r.established.output_key.x_only_public_key().0,
        )
        .expect("a BIP-340 signature of the output key");

    let key = r.abandon_key;
    write_partial(&mut r, &key, &forged());
    assert!(faraday_psbt::musig::aggregate(&c, &r.psbt, 0, &r.established).is_none());
}

// ---------------------------------------------------------------------
// Passes over several inputs, and over several held keys
// ---------------------------------------------------------------------

/// The two-key spend with a second input of the same wallet.
fn two_key_two_inputs(policy: &WalletPolicy, a: &MasterKey, b: &MasterKey) -> Psbt {
    let mut psbt = two_key_psbt(policy, a, b);
    let inner = psbt.inner_mut();
    let mut again = inner.unsigned_tx.input[0].clone();
    again.previous_output.vout = 1;
    inner.unsigned_tx.input.push(again);
    inner.unsigned_tx.output[0].value = Amount::from_sat(199_000);
    let second = inner.inputs[0].clone();
    inner.inputs.push(second);
    psbt
}

fn sighash_at(psbt: &Psbt, index: usize) -> [u8; 32] {
    let tx = psbt.inner().unsigned_tx.clone();
    let prevouts: Vec<TxOut> = psbt
        .inner()
        .inputs
        .iter()
        .map(|i| i.witness_utxo.clone().unwrap())
        .collect();
    bitcoin::sighash::SighashCache::new(&tx)
        .taproot_key_spend_signature_hash(
            index,
            &bitcoin::sighash::Prevouts::All(&prevouts),
            bitcoin::sighash::TapSighashType::Default,
        )
        .unwrap()
        .to_byte_array()
}

/// The other signer's answer on input `index`: its nonce and, true or made
/// up, its partial signature. Needs every other nonce already on the input.
fn other_signer_answers(
    psbt: &mut Psbt,
    established: &faraday_psbt::MusigInput,
    zoo: &MasterKey,
    index: usize,
    true_partial: bool,
) {
    let secp = Secp256k1::new();
    let msg = sighash_at(psbt, index);
    let zoo_key = account_xpub(zoo).public_key;
    let (secnonce, pubnonce) = other_nonce(
        zoo,
        &established.output_key.x_only_public_key().0.serialize(),
        &msg,
        0x31 + index as u8,
    );
    let input = &mut psbt.inner_mut().inputs[index];
    faraday_psbt::musig::write_pub_nonce(input, &zoo_key, &established.output_key, &pubnonce);
    let nonces = faraday_psbt::musig::pub_nonces(input).unwrap();
    let all: Vec<PubNonce> = established
        .participants
        .iter()
        .map(|p| nonces.iter().find(|n| n.participant == *p).unwrap().value)
        .collect();
    let values = musig::session_values(
        &secp,
        &established.participants,
        &established.tweaks,
        &musig::nonce_agg(&all).unwrap(),
        &msg,
    )
    .unwrap();
    let psig = if true_partial {
        musig::sign(&secp, secnonce, &account_secret(zoo), &values).unwrap()
    } else {
        forged()
    };
    faraday_psbt::musig::write_partial_sig(input, &zoo_key, &established.output_key, &psig);
}

/// A pass over a spend of two inputs that refuses on the second takes no
/// nonce on the first: both are held after, and with the second input's
/// partial signature true the next pass finishes both.
#[test]
fn a_refused_pass_over_two_inputs_leaves_both_nonces_held() {
    let c = Compute::unchecked();
    let abandon = master(ABANDON, NET);
    let zoo = master(ZOO, NET);
    let policy = two_key_policy(&abandon, &zoo);
    let keys = [KeyRef::from_master(&c, &abandon, 0).unwrap()];
    let wallets = [policy.clone()];
    let mut psbt = two_key_two_inputs(&policy, &abandon, &zoo);
    let mut session = None;
    {
        let ctx = context_with(&keys, &wallets, None);
        pass(&mut psbt, &[&abandon], &ctx, &mut session, 0x41);
    }
    assert_eq!(session.as_ref().unwrap().len(), 2);
    let established: Vec<_> = {
        let view = session.as_ref().unwrap().view();
        let ctx = context_with(&keys, &wallets, Some(&view));
        let insp = inspect(&c, &psbt, &ctx);
        insp.inputs
            .iter()
            .map(|i| i.musig.clone().unwrap())
            .collect()
    };
    other_signer_answers(&mut psbt, &established[0], &zoo, 0, true);
    other_signer_answers(&mut psbt, &established[1], &zoo, 1, false);

    let view = session.as_ref().unwrap().view();
    let ctx = context_with(&keys, &wallets, Some(&view));
    let err = sign(
        &c,
        &mut psbt,
        &[&abandon],
        &[abandon.fingerprint()],
        &ctx,
        &[],
        Nonce::LowR,
        Aux::Deterministic,
        &mut session,
        Some(&faraday_crypto::Zeroizing::new([0x41u8; 32])),
    )
    .unwrap_err();
    assert!(
        matches!(err, Error::MusigPartialSig { input: 1, .. }),
        "{err:?}"
    );
    assert_eq!(session.as_ref().unwrap().len(), 2, "both nonces are held");

    other_signer_answers(&mut psbt, &established[1], &zoo, 1, true);
    drop(view);
    let view = session.as_ref().unwrap().view();
    let ctx = context_with(&keys, &wallets, Some(&view));
    let out = pass(&mut psbt, &[&abandon], &ctx, &mut session, 0x41);
    assert!(out.complete);
    assert!(psbt.inner().inputs.iter().all(|i| i.tap_key_sig.is_some()));
}

/// Two keys of one wallet held on one device, a made-up public nonce in
/// the file for the later one: both partial signatures are made under the
/// nonces the device kept, and the spend finishes.
#[test]
fn a_made_up_nonce_for_a_second_held_key_does_not_spoil_the_first_partial() {
    let c = Compute::unchecked();
    let abandon = master(ABANDON, NET);
    let zoo = master(ZOO, NET);
    let policy = two_key_policy(&abandon, &zoo);
    let keys = [
        KeyRef::from_master(&c, &abandon, 0).unwrap(),
        KeyRef::from_master(&c, &zoo, 0).unwrap(),
    ];
    let wallets = [policy.clone()];
    let mut psbt = two_key_psbt(&policy, &abandon, &zoo);
    let mut session = None;
    {
        let ctx = context_with(&keys, &wallets, None);
        pass(&mut psbt, &[&abandon, &zoo], &ctx, &mut session, 0x41);
    }
    assert_eq!(session.as_ref().unwrap().len(), 2);
    let established = {
        let view = session.as_ref().unwrap().view();
        let ctx = context_with(&keys, &wallets, Some(&view));
        inspect(&c, &psbt, &ctx).inputs[0].musig.clone().unwrap()
    };
    let msg = sighash_at(&psbt, 0);
    let zoo_key = account_xpub(&zoo).public_key;
    let (_, made_up) = other_nonce(
        &zoo,
        &established.output_key.x_only_public_key().0.serialize(),
        &msg,
        0x77,
    );
    faraday_psbt::musig::write_pub_nonce(
        &mut psbt.inner_mut().inputs[0],
        &zoo_key,
        &established.output_key,
        &made_up,
    );
    let view = session.as_ref().unwrap().view();
    let ctx = context_with(&keys, &wallets, Some(&view));
    let out = pass(&mut psbt, &[&abandon, &zoo], &ctx, &mut session, 0x41);
    assert!(out.complete);
    assert!(aggregate_verifies(&psbt, &established));
}

// ---------------------------------------------------------------------
// Entries under more than one form of the aggregate key
// ---------------------------------------------------------------------

/// A form of the aggregate key the input accepts that sorts before the
/// output key, so a reader taking the first entry takes it.
fn lower_form(r: &faraday_psbt::MusigInput) -> PublicKey {
    let (x, _) = r
        .internal_key
        .public_key(bitcoin::secp256k1::Parity::Even)
        .x_only_public_key();
    let forms = [
        r.root_aggregate,
        PublicKey::from_x_only_public_key(x, bitcoin::secp256k1::Parity::Even),
        PublicKey::from_x_only_public_key(x, bitcoin::secp256k1::Parity::Odd),
    ];
    forms
        .into_iter()
        .find(|f| *f != r.output_key && f.serialize() < r.output_key.serialize())
        .expect("a form that sorts first")
}

/// A made-up nonce for the held key under another form of the aggregate,
/// beside the true one: the true one is the one used, and the spend
/// finishes.
#[test]
fn a_made_up_nonce_under_another_aggregate_form_is_not_the_one_used() {
    let c = Compute::unchecked();
    let abandon = master(ABANDON, NET);
    let zoo = master(ZOO, NET);
    let policy = two_key_policy(&abandon, &zoo);
    let keys = [KeyRef::from_master(&c, &abandon, 0).unwrap()];
    let wallets = [policy];
    let mut r = after_round_one(&abandon, &zoo);
    let msg = key_path_sighash(&r.psbt);
    let (_, made_up) = other_nonce(&abandon, &[9u8; 32], &msg, 0x66);
    let form = lower_form(&r.established);
    faraday_psbt::musig::write_pub_nonce(
        &mut r.psbt.inner_mut().inputs[0],
        &r.abandon_key,
        &form,
        &made_up,
    );
    let view = r.session.as_ref().unwrap().view();
    let ctx = context_with(&keys, &wallets, Some(&view));
    let out = pass(&mut r.psbt, &[&abandon], &ctx, &mut r.session, 0x41);
    assert!(out.complete);
    assert!(aggregate_verifies(&r.psbt, &r.established));
}

/// A made-up partial signature under another form of the aggregate first
/// and the true one under the output key second: the screen and the
/// aggregation read the same one, and the true one finishes.
#[test]
fn a_made_up_partial_under_another_aggregate_form_does_not_hide_the_true_one() {
    let c = Compute::unchecked();
    let abandon = master(ABANDON, NET);
    let zoo = master(ZOO, NET);
    let policy = two_key_policy(&abandon, &zoo);
    let keys = [KeyRef::from_master(&c, &abandon, 0).unwrap()];
    let wallets = [policy];
    let mut r = after_round_one(&abandon, &zoo);
    let view = r.session.as_ref().unwrap().view();
    let ctx = context_with(&keys, &wallets, Some(&view));
    pass(&mut r.psbt, &[&abandon], &ctx, &mut r.session, 0x41);
    let form = lower_form(&r.established);
    faraday_psbt::musig::write_partial_sig(
        &mut r.psbt.inner_mut().inputs[0],
        &r.zoo_key,
        &form,
        &forged(),
    );
    let verified = faraday_psbt::musig::verified_partials(&c, &r.psbt, 0, &r.established);
    let finished = faraday_psbt::musig::aggregate(&c, &r.psbt, 0, &r.established).is_some();
    assert_eq!(
        verified.contains(&r.zoo_key),
        finished,
        "the screen and the aggregation read the same entry"
    );
    assert!(finished, "the true partial signature finishes");
}

/// A sighash type the pass refuses on a later input is refused before the
/// earlier input takes its kept nonce: both are held after, and the same
/// nonce signs once the file is honest.
#[test]
fn a_sighash_refused_on_a_later_input_leaves_every_nonce_held() {
    let c = Compute::unchecked();
    let abandon = master(ABANDON, NET);
    let zoo = master(ZOO, NET);
    let policy = two_key_policy(&abandon, &zoo);
    let keys = [KeyRef::from_master(&c, &abandon, 0).unwrap()];
    let wallets = [policy.clone()];
    let mut psbt = two_key_two_inputs(&policy, &abandon, &zoo);
    let mut session = None;
    {
        let ctx = context_with(&keys, &wallets, None);
        pass(&mut psbt, &[&abandon], &ctx, &mut session, 0x41);
    }
    let established: Vec<_> = {
        let view = session.as_ref().unwrap().view();
        let ctx = context_with(&keys, &wallets, Some(&view));
        inspect(&c, &psbt, &ctx)
            .inputs
            .iter()
            .map(|i| i.musig.clone().unwrap())
            .collect()
    };
    other_signer_answers(&mut psbt, &established[0], &zoo, 0, true);
    other_signer_answers(&mut psbt, &established[1], &zoo, 1, true);
    let honest = psbt.clone();
    let nonce_before = faraday_psbt::musig::pub_nonces(&psbt.inner().inputs[0]).unwrap();
    psbt.inner_mut().inputs[1].sighash_type = Some(bitcoin::psbt::PsbtSighashType::from_u32(1));

    let view = session.as_ref().unwrap().view();
    let ctx = context_with(&keys, &wallets, Some(&view));
    let err = sign(
        &c,
        &mut psbt,
        &[&abandon],
        &[abandon.fingerprint()],
        &ctx,
        &[],
        Nonce::LowR,
        Aux::Deterministic,
        &mut session,
        Some(&faraday_crypto::Zeroizing::new([0x41u8; 32])),
    )
    .unwrap_err();
    assert!(
        matches!(err, Error::Unsupported { input: 1, .. }),
        "{err:?}"
    );
    assert_eq!(session.as_ref().unwrap().len(), 2, "both nonces are held");

    let mut psbt = honest;
    let view = session.as_ref().unwrap().view();
    let ctx = context_with(&keys, &wallets, Some(&view));
    let out = pass(&mut psbt, &[&abandon], &ctx, &mut session, 0x41);
    assert!(out.complete);
    let nonce_after = faraday_psbt::musig::pub_nonces(&psbt.inner().inputs[0]).unwrap();
    assert_eq!(
        nonce_before
            .iter()
            .find(|n| n.participant == account_xpub(&abandon).public_key)
            .map(|n| n.value),
        nonce_after
            .iter()
            .find(|n| n.participant == account_xpub(&abandon).public_key)
            .map(|n| n.value),
        "the nonce the next Sign used is the one kept"
    );
}

/// A partial signature under a key that is none of the input's participants
/// is no reason to mend: with an older nonce of ours in the file and no
/// participant's partial failing, the pass signs.
#[test]
fn a_partial_of_a_non_participant_does_not_make_the_pass_mend() {
    let c = Compute::unchecked();
    let abandon = master(ABANDON, NET);
    let zoo = master(ZOO, NET);
    let policy = two_key_policy(&abandon, &zoo);
    let keys = [KeyRef::from_master(&c, &abandon, 0).unwrap()];
    let wallets = [policy];
    let mut r = after_round_one(&abandon, &zoo);
    let ours = r.abandon_key;
    let (_, older) = other_nonce(
        &abandon,
        &r.established.output_key.x_only_public_key().0.serialize(),
        &key_path_sighash(&r.psbt),
        0x77,
    );
    faraday_psbt::musig::write_pub_nonce(
        &mut r.psbt.inner_mut().inputs[0],
        &ours,
        &r.established.output_key,
        &older,
    );
    let stranger = faraday_bip::bitcoin::secp256k1::SecretKey::from_slice(&[9u8; 32])
        .unwrap()
        .public_key(&faraday_bip::bitcoin::secp256k1::Secp256k1::new());
    write_partial(&mut r, &stranger, &forged());

    let view = r.session.as_ref().unwrap().view();
    let ctx = context_with(&keys, &wallets, Some(&view));
    let out = pass(&mut r.psbt, &[&abandon], &ctx, &mut r.session, 0x41);
    assert!(!out.signed_inputs.is_empty(), "the pass signed");
}

/// A file that carries another nonce of this device's than the one it
/// kept (an older copy won a merge) is not signed and not refused: the
/// kept nonce goes back in, the partial signature that cannot verify under
/// it goes out, the secret stays held, and once the cosigner signs again
/// the same nonce signs.
#[test]
fn a_file_with_an_older_nonce_of_ours_gets_the_kept_one_back_and_loses_the_partials_that_fail() {
    let c = Compute::unchecked();
    let abandon = master(ABANDON, NET);
    let zoo = master(ZOO, NET);
    let policy = two_key_policy(&abandon, &zoo);
    let keys = [KeyRef::from_master(&c, &abandon, 0).unwrap()];
    let wallets = [policy];
    let mut r = after_round_one(&abandon, &zoo);
    let ours = r.abandon_key;
    let kept = faraday_psbt::musig::pub_nonces(&r.psbt.inner().inputs[0])
        .unwrap()
        .into_iter()
        .find(|n| n.participant == ours)
        .unwrap()
        .value;
    let zoo_nonce = faraday_psbt::musig::pub_nonces(&r.psbt.inner().inputs[0])
        .unwrap()
        .into_iter()
        .find(|n| n.participant == r.zoo_key)
        .unwrap()
        .value;
    let (_, older) = other_nonce(
        &abandon,
        &r.established.output_key.x_only_public_key().0.serialize(),
        &key_path_sighash(&r.psbt),
        0x77,
    );
    assert_ne!(older, kept);
    // The cosigner signed under the older nonce set.
    let (zoo_key, zoo_psig) = (r.zoo_key, r.zoo_psig);
    faraday_psbt::musig::write_pub_nonce(
        &mut r.psbt.inner_mut().inputs[0],
        &ours,
        &r.established.output_key,
        &older,
    );
    write_partial(&mut r, &zoo_key, &forged());
    let file = faraday_psbt::v2::PsbtV2::from_v0(&c, &r.psbt);

    let view = r.session.as_ref().unwrap().view();
    let ctx = context_with(&keys, &wallets, Some(&view));
    let out = pass(&mut r.psbt, &[&abandon], &ctx, &mut r.session, 0x41);
    assert!(out.signed_inputs.is_empty(), "nothing is signed");
    assert!(!out.complete);
    assert_eq!(r.session.as_ref().unwrap().len(), 1, "the nonce is kept");
    let input = &r.psbt.inner().inputs[0];
    let now = faraday_psbt::musig::pub_nonces(input)
        .unwrap()
        .into_iter()
        .find(|n| n.participant == ours)
        .unwrap()
        .value;
    assert_eq!(now, kept, "the kept public nonce is in the file");
    assert!(
        faraday_psbt::musig::partial_sigs(input).unwrap().is_empty(),
        "the partial that cannot verify is gone"
    );
    assert!(
        faraday_psbt::musig::pub_nonces(input)
            .unwrap()
            .iter()
            .all(|n| n.participant != zoo_key),
        "the nonce of the cosigner whose partial went is spent and gone"
    );

    // The version 2 file the older copy came in takes the same: the kept
    // nonce for this device's key, and no nonce or partial signature of
    // the cosigner whose partial went.
    let mut merged = file.clone();
    assert!(merged.merge_signed_by(&r.psbt, &out.nonces_written).is_ok());
    let back = merged.to_v0(&c).unwrap();
    let back_nonces = faraday_psbt::musig::pub_nonces(&back.inner().inputs[0]).unwrap();
    assert_eq!(
        back_nonces
            .iter()
            .find(|n| n.participant == ours)
            .map(|n| n.value),
        Some(kept)
    );
    assert!(back_nonces.iter().all(|n| n.participant != zoo_key));
    assert!(
        faraday_psbt::musig::partial_sigs(&back.inner().inputs[0])
            .unwrap()
            .is_empty()
    );

    // The cosigner shares again and answers the kept nonce; the same
    // nonce signs.
    faraday_psbt::musig::write_pub_nonce(
        &mut r.psbt.inner_mut().inputs[0],
        &zoo_key,
        &r.established.output_key,
        &zoo_nonce,
    );
    write_partial(&mut r, &zoo_key, &zoo_psig);
    let view = r.session.as_ref().unwrap().view();
    let ctx = context_with(&keys, &wallets, Some(&view));
    let out = pass(&mut r.psbt, &[&abandon], &ctx, &mut r.session, 0x41);
    assert!(out.complete);
    assert!(aggregate_verifies(&r.psbt, &r.established));
}
