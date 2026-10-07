//! MuSig2 (BIP-327, BIP-373, BIP-390): what a wallet talking to this
//! device sees when it spends a `tr(musig(…))` wallet.
//!
//! Two counterparts. Bitcoin Core 31.1 on regtest, whose PSBTs are the
//! `wallet-musig-*` fixtures: Core writes its nonce, the device signs
//! last with no session, and Core finishes — the accepted transaction is
//! recorded in `tools/vectors/psbt/README.md`. And a second signer
//! played here, which carries the rounds the whole way so that the
//! aggregate signature this crate builds finalizes the input.

mod common;

use std::collections::BTreeMap;
use std::path::PathBuf;

use bitcoin::absolute::LockTime;
use bitcoin::bip32::{ChildNumber, DerivationPath, Xpub};
use bitcoin::hashes::Hash;
use bitcoin::secp256k1::{PublicKey, Secp256k1, SecretKey};
use bitcoin::taproot::TapTweakHash;
use bitcoin::transaction::Version;
use bitcoin::{Amount, OutPoint, ScriptBuf, Sequence, Transaction, TxIn, TxOut, Witness};
use common::builder::funding_tx;
use common::{ABANDON, ZOO, master};
use osk_bip::keys::{Fingerprint, MasterKey, Network, ScriptType};
use osk_bip::musig::{self, PubNonce, Tweak};
use osk_bip::policy::WalletPolicy;
use osk_psbt::{
    Aux, Context, Error, KeyRef, Level, MusigRole, Nonce, OutputKind, Psbt, WarningKind, finalize,
    inspect, sign,
};

#[path = "../examples/musig.rs"]
mod example;

const NET: Network = Network::Regtest;

fn vectors() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tools/vectors/psbt")
}

fn fixture(name: &str) -> Psbt {
    let text = std::fs::read_to_string(vectors().join(name)).expect("the fixture");
    Psbt::parse_base64(text.trim()).expect("a PSBT")
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
        shares: &[],
        carry: None,
    }
}

/// Core's round-1 PSBT is signed last: the device writes its public
/// nonce and its partial signature for its own participant, and the
/// transaction is not finished, because Core still has to aggregate.
#[test]
fn the_device_signs_last_after_bitcoin_core() {
    let abandon = master(ABANDON, NET);
    let keys = [KeyRef::from_master(&abandon, 0).unwrap()];
    let wallets = [core_policy()];
    let ctx = core_context(&keys, &wallets);
    let mut psbt = fixture(example::CORE_FIRST_NAME);

    let insp = inspect(&psbt, &ctx);
    assert!(!insp.has_blocked(), "{:?}", insp.warnings);
    assert_eq!(insp.participating_keys, vec![abandon.fingerprint()]);
    assert_eq!(insp.musig.as_ref().map(|m| m.keys), Some(2));

    let out = sign(
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
    .unwrap();
    assert_eq!(out.signed_inputs.len(), 1);
    assert_eq!(out.signed_inputs[0].musig, Some(MusigRole::Partial));
    assert_eq!(out.signed_inputs[0].fingerprint, abandon.fingerprint());
    assert!(!out.complete, "Core still has to sign and aggregate");

    let input = &psbt.inner().inputs[0];
    let ours = osk_psbt::musig::pub_nonces(input).unwrap();
    assert_eq!(ours.len(), 2, "Core's nonce and ours");
    let sigs = osk_psbt::musig::partial_sigs(input).unwrap();
    assert_eq!(sigs.len(), 1);
    // What this device writes is keyed by the taproot output key, which
    // is what Core writes and reads.
    let established = inspect(&psbt, &ctx).inputs[0].musig.clone().unwrap();
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
        "run `cargo run -p osk-psbt --example musig -- tools/vectors/psbt`"
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
            "{name}: run `cargo run -p osk-psbt --example musig -- tools/vectors/psbt`"
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
/// randomness gives the same partial signature.
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
    let abandon = master(ABANDON, NET);
    let keys = [KeyRef::from_master(&abandon, 0).unwrap()];
    let wallets = [core_policy()];
    let ctx = core_context(&keys, &wallets);
    let mut psbt = fixture(example::DEVICE_FIRST_NAME);

    let insp = inspect(&psbt, &ctx);
    assert!(!insp.has_blocked(), "{:?}", insp.warnings);
    assert!(
        insp.inputs[0]
            .musig
            .as_ref()
            .is_some_and(|m| m.will_share_nonce && !m.nonce_stale)
    );

    let mut session = None;
    let out = sign(
        &mut psbt,
        &[&abandon],
        &[abandon.fingerprint()],
        &ctx,
        false,
        Nonce::LowR,
        Aux::Deterministic,
        &mut session,
        [3u8; 32],
        &[],
        &[],
    )
    .unwrap();
    assert_eq!(out.nonces_shared, 1);
    assert!(out.signed_inputs.is_empty(), "nothing is signed in round 1");
    assert!(!out.complete);

    let input = &psbt.inner().inputs[0];
    let nonces = osk_psbt::musig::pub_nonces(input).unwrap();
    assert_eq!(nonces.len(), 1, "this device's nonce and nobody else's");
    assert!(osk_psbt::musig::partial_sigs(input).unwrap().is_empty());
    let session = session.expect("the session is open");
    assert_eq!(session.len(), 1, "one secret nonce, for the one input");
    assert_eq!(session.txid(), psbt.unsigned_tx().compute_txid());
    let established = inspect(&psbt, &ctx).inputs[0].musig.clone().unwrap();
    assert!(session.holds(0, &established.ours[0].key));
}

/// The change output of the funded PSBT is the wallet's own address at
/// the chain and index its key origins name, so it is verified change
/// and not a payment to others.
#[test]
fn the_musig_change_output_is_verified() {
    let abandon = master(ABANDON, NET);
    let keys = [KeyRef::from_master(&abandon, 0).unwrap()];
    let wallets = [core_policy()];
    let ctx = core_context(&keys, &wallets);
    let insp = inspect(&fixture(example::DEVICE_FIRST_NAME), &ctx);
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
    let abandon = master(ABANDON, NET);
    let keys = [KeyRef::from_master(&abandon, 0).unwrap()];
    let ctx = core_context(&keys, &[]);
    let insp = inspect(&fixture(example::CORE_FIRST_NAME), &ctx);
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
    let signed = fixture(example::CORE_FIRST_SIGNED_NAME);
    let again = Psbt::parse_bytes(&signed.to_bytes()).unwrap();
    assert_eq!(signed.to_bytes(), again.to_bytes());
    let input = &again.inner().inputs[0];
    assert_eq!(osk_psbt::musig::pub_nonces(input).unwrap().len(), 2);
    assert_eq!(osk_psbt::musig::partial_sigs(input).unwrap().len(), 1);
    assert_eq!(
        osk_psbt::musig::input_participants(input).unwrap()[0]
            .keys
            .len(),
        2
    );
}

/// A nonce or partial-signature key with a tapleaf hash on it is a
/// script-path MuSig2 spend, which is not a wallet this tree loads.
#[test]
fn a_script_path_musig_field_is_refused() {
    let mut psbt = fixture(example::CORE_FIRST_SIGNED_NAME);
    let input = &mut psbt.inner_mut().inputs[0];
    let (key, value) = input
        .unknown
        .iter()
        .find(|(k, _)| k.type_value == osk_psbt::musig::IN_PUB_NONCE)
        .map(|(k, v)| (k.clone(), v.clone()))
        .unwrap();
    let mut longer = key.clone();
    longer.key.extend_from_slice(&[0u8; 32]);
    input.unknown.remove(&key);
    input.unknown.insert(longer, value);
    assert!(osk_psbt::musig::pub_nonces(&psbt.inner().inputs[0]).is_err());

    let abandon = master(ABANDON, NET);
    let keys = [KeyRef::from_master(&abandon, 0).unwrap()];
    let wallets = [core_policy()];
    let ctx = core_context(&keys, &wallets);
    assert!(inspect(&psbt, &ctx).has_blocked());
}

/// A malformed participants field blocks the input with a reason instead
/// of panicking.
#[test]
fn a_malformed_participants_field_blocks_the_input() {
    let mut psbt = fixture(example::CORE_FIRST_NAME);
    let input = &mut psbt.inner_mut().inputs[0];
    let key = input
        .unknown
        .keys()
        .find(|k| k.type_value == osk_psbt::musig::IN_PARTICIPANT_PUBKEYS)
        .cloned()
        .unwrap();
    input.unknown.insert(key, vec![0u8; 20]);
    let abandon = master(ABANDON, NET);
    let keys = [KeyRef::from_master(&abandon, 0).unwrap()];
    let wallets = [core_policy()];
    let ctx = core_context(&keys, &wallets);
    assert!(inspect(&psbt, &ctx).has_blocked());
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
    *key.account_xpub(ScriptType::Taproot, 0).unwrap().xpub()
}

/// The two-key wallet the two seeds make, `@0` first.
fn two_key_policy(a: &MasterKey, b: &MasterKey) -> WalletPolicy {
    let text = |k: &MasterKey| format!("[{}/86'/1'/0']{}", k.fingerprint(), account_xpub(k));
    WalletPolicy::from_parts("tr(musig(@0,@1)/**)", &[&text(a), &text(b)]).unwrap()
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
    let secp = Secp256k1::verification_only();
    let spk = policy.script_at(false, 0).unwrap();
    let funding = funding_tx(spk.clone(), 100_000);
    let recipient = b
        .account_xpub(ScriptType::NativeSegwit, 0)
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
    osk_psbt::musig::write_input_participants(
        &mut inner.inputs[0],
        &osk_psbt::musig::Participants {
            aggregate: root.public_key(),
            keys: participants,
        },
    );
    Psbt::from(inner)
}

/// The account-level secret key of `master`, which is what a `musig()`
/// participant signs with.
fn account_secret(master: &MasterKey) -> SecretKey {
    *master.derive(&account_path()).secret_key()
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
    let secp = Secp256k1::new();
    let abandon = master(ABANDON, NET);
    let zoo = master(ZOO, NET);
    let policy = two_key_policy(&abandon, &zoo);
    let mut psbt = two_key_psbt(&policy, &abandon, &zoo);

    let keys = [KeyRef::from_master(&abandon, 0).unwrap()];
    let wallets = [policy];
    let ctx = core_context(&keys, &wallets);

    // With nobody's nonce on it, a pass would go first and share one.
    assert!(
        inspect(&psbt, &ctx).inputs[0]
            .musig
            .as_ref()
            .is_some_and(|m| m.will_share_nonce)
    );

    let established = {
        let mut ours = psbt.clone();
        let _ = &mut ours;
        inspect(&psbt, &ctx).inputs[0].musig.clone().unwrap()
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
    osk_psbt::musig::write_pub_nonce(
        &mut psbt.inner_mut().inputs[0],
        &zoo_key,
        &established.output_key,
        &pubnonce,
    );

    // The device signs last.
    let out = sign(
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
    .unwrap();
    assert_eq!(out.signed_inputs.len(), 1);
    assert_eq!(out.signed_inputs[0].musig, Some(MusigRole::Partial));
    assert!(!out.complete);

    // The other signer's round 2, over the aggregate nonce both are in.
    let nonces = osk_psbt::musig::pub_nonces(&psbt.inner().inputs[0]).unwrap();
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
    osk_psbt::musig::write_partial_sig(
        &mut psbt.inner_mut().inputs[0],
        &zoo_key,
        &established.output_key,
        &psig,
    );

    // The device reads it back, aggregates, and the input finalizes.
    let out = sign(
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
    let abandon = master(ABANDON, NET);
    let zoo = master(ZOO, NET);
    let policy = two_key_policy(&abandon, &zoo);
    let mut psbt = two_key_psbt(&policy, &abandon, &zoo);
    let keys = [KeyRef::from_master(&abandon, 0).unwrap()];
    let wallets = [policy];
    let ctx = core_context(&keys, &wallets);
    let established = inspect(&psbt, &ctx).inputs[0].musig.clone().unwrap();
    let msg = key_path_sighash(&psbt);
    let (_, pubnonce) = other_nonce(
        &zoo,
        &established.output_key.x_only_public_key().0.serialize(),
        &msg,
        0x22,
    );
    let zoo_key = account_xpub(&zoo).public_key;
    osk_psbt::musig::write_pub_nonce(
        &mut psbt.inner_mut().inputs[0],
        &zoo_key,
        &established.output_key,
        &pubnonce,
    );
    // A partial signature that is a scalar but not this signer's.
    let bogus = osk_bip::musig::PartialSig::from_bytes(&[7u8; 32]).unwrap();
    osk_psbt::musig::write_partial_sig(
        &mut psbt.inner_mut().inputs[0],
        &zoo_key,
        &established.output_key,
        &bogus,
    );
    let err = sign(
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
    let abandon = master(ABANDON, NET);
    let keys = [KeyRef::from_master(&abandon, 0).unwrap()];
    let wallets = [core_policy()];
    let ctx = core_context(&keys, &wallets);
    let insp = inspect(&fixture(example::CORE_FIRST_NAME), &ctx);
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
// The device first, with the session (docs/PLANNING.md §16.100)
// ---------------------------------------------------------------------

/// A context with the session this device holds, which is what decides
/// whether a nonce of ours on a PSBT is one we can still sign under.
fn context_with<'a>(
    keys: &'a [KeyRef],
    wallets: &'a [WalletPolicy],
    session: Option<&'a osk_psbt::MusigSessionView>,
) -> Context<'a> {
    Context {
        network: NET,
        keys,
        wallets,
        musig_session: session,
        shares: &[],
        carry: None,
    }
}

/// Signs `psbt` with `session` open, for the keys given.
fn pass(
    psbt: &mut Psbt,
    masters: &[&MasterKey],
    ctx: &Context,
    session: &mut Option<osk_psbt::MusigSession>,
    seed: u8,
) -> osk_psbt::SignResult {
    let selection: Vec<Fingerprint> = masters.iter().map(|m| m.fingerprint()).collect();
    sign(
        psbt,
        masters,
        &selection,
        ctx,
        false,
        Nonce::LowR,
        Aux::Deterministic,
        session,
        [seed; 32],
        &[],
        &[],
    )
    .expect("the pass runs")
}

/// The whole flow with the device going first: round 1 writes this
/// device's nonce and keeps the secret, the other signer answers with
/// its own nonce and partial signature, and round 2 with the same
/// session signs, aggregates and finalizes, leaving the session empty.
#[test]
fn the_device_goes_first_and_finishes_when_the_transaction_returns() {
    let secp = Secp256k1::new();
    let abandon = master(ABANDON, NET);
    let zoo = master(ZOO, NET);
    let policy = two_key_policy(&abandon, &zoo);
    let mut psbt = two_key_psbt(&policy, &abandon, &zoo);
    let keys = [KeyRef::from_master(&abandon, 0).unwrap()];
    let wallets = [policy];

    let mut session = None;
    let out = {
        let ctx = context_with(&keys, &wallets, None);
        pass(&mut psbt, &[&abandon], &ctx, &mut session, 0x41)
    };
    assert_eq!(out.nonces_shared, 1);
    assert!(out.signed_inputs.is_empty());
    let input = &psbt.inner().inputs[0];
    assert_eq!(osk_psbt::musig::pub_nonces(input).unwrap().len(), 1);
    assert!(osk_psbt::musig::partial_sigs(input).unwrap().is_empty());
    assert_eq!(session.as_ref().unwrap().len(), 1);

    // The other signer answers with its nonce and, seeing both, its
    // partial signature.
    let established = {
        let view = session.as_ref().unwrap().view();
        let ctx = context_with(&keys, &wallets, Some(&view));
        inspect(&psbt, &ctx).inputs[0].musig.clone().unwrap()
    };
    let msg = key_path_sighash(&psbt);
    let (secnonce, pubnonce) = other_nonce(
        &zoo,
        &established.output_key.x_only_public_key().0.serialize(),
        &msg,
        0x31,
    );
    let zoo_key = account_xpub(&zoo).public_key;
    osk_psbt::musig::write_pub_nonce(
        &mut psbt.inner_mut().inputs[0],
        &zoo_key,
        &established.output_key,
        &pubnonce,
    );
    let nonces = osk_psbt::musig::pub_nonces(&psbt.inner().inputs[0]).unwrap();
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
    osk_psbt::musig::write_partial_sig(
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
    let mut again = Psbt::parse_bytes(&psbt.to_bytes()).unwrap();
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
    let abandon = master(ABANDON, NET);
    let zoo = master(ZOO, NET);
    let policy = two_key_policy(&abandon, &zoo);
    let keys = [KeyRef::from_master(&abandon, 0).unwrap()];
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
    let abandon = master(ABANDON, NET);
    let zoo = master(ZOO, NET);
    let policy = two_key_policy(&abandon, &zoo);
    let keys = [KeyRef::from_master(&abandon, 0).unwrap()];
    let wallets = [policy.clone()];
    let mut psbt = two_key_psbt(&policy, &abandon, &zoo);

    // Round 1, and then the session is gone: the device locked, or was
    // powered off.
    let mut session = None;
    {
        let ctx = context_with(&keys, &wallets, None);
        pass(&mut psbt, &[&abandon], &ctx, &mut session, 0x61);
    }
    let stale = osk_psbt::musig::pub_nonces(&psbt.inner().inputs[0]).unwrap()[0].value;
    session = None;

    // The other signer answers, against the nonce that is now stale.
    let ctx = context_with(&keys, &wallets, None);
    let established = inspect(&psbt, &ctx).inputs[0].musig.clone().unwrap();
    let msg = key_path_sighash(&psbt);
    let (_, pubnonce) = other_nonce(
        &zoo,
        &established.output_key.x_only_public_key().0.serialize(),
        &msg,
        0x62,
    );
    let zoo_key = account_xpub(&zoo).public_key;
    osk_psbt::musig::write_pub_nonce(
        &mut psbt.inner_mut().inputs[0],
        &zoo_key,
        &established.output_key,
        &pubnonce,
    );
    osk_psbt::musig::write_partial_sig(
        &mut psbt.inner_mut().inputs[0],
        &zoo_key,
        &established.output_key,
        &osk_bip::musig::PartialSig::from_bytes(&[9u8; 32]).unwrap(),
    );

    let insp = inspect(&psbt, &ctx);
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
        osk_psbt::musig::partial_sigs(input).unwrap().is_empty(),
        "the other signer's partial signature went with the nonce it was made against"
    );
    let ours = osk_psbt::musig::pub_nonces(input)
        .unwrap()
        .into_iter()
        .find(|n| n.participant == established.ours[0].key)
        .unwrap();
    assert_ne!(ours.value, stale, "a new nonce, not the one it replaced");
}

/// Two of this device's keys in one wallet: neither can derive its nonce
/// from the other's, so one round 1 shares both and one round 2 signs
/// both.
#[test]
fn two_keys_of_one_wallet_share_and_sign_together() {
    let abandon = master(ABANDON, NET);
    let zoo = master(ZOO, NET);
    let policy = two_key_policy(&abandon, &zoo);
    let mut psbt = two_key_psbt(&policy, &abandon, &zoo);
    let keys = [
        KeyRef::from_master(&abandon, 0).unwrap(),
        KeyRef::from_master(&zoo, 0).unwrap(),
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
        osk_psbt::musig::pub_nonces(&psbt.inner().inputs[0])
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
