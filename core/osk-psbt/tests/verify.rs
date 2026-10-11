//! What a signature already on a transaction is checked for
//! (`docs/PLANNING.md` §16.111): every signature the committed fixtures
//! carry verifies, a finalized witness is read back and verifies, one
//! byte changed is invalid, two signatures under one nonce are found, a
//! signature this device made is recognised as the rule that made it,
//! two PSBTs are compared on their contents, and an inscription
//! envelope is stated.

mod common;

use std::str::FromStr;

use bitcoin::bip32::DerivationPath;
use bitcoin::secp256k1::{PublicKey, Scalar, Secp256k1, SecretKey};
use bitcoin::sighash::{EcdsaSighashType, SighashCache};
use bitcoin::{Amount, OutPoint, ScriptBuf, Sequence, TxIn, TxOut, Witness, ecdsa};
use common::builder::{Build, funding_tx, single_sig, sorted_multisig_change};
use common::{ABANDON, LEGAL, ZOO, master};
use osk_bip::keys::{Network, ScriptType};
use osk_bip::policy::WalletPolicy;
use osk_psbt::verify::{
    Field, NonceMode, Unchecked, Verdict, compare, deterministic, inscriptions, repeated_nonces,
    verify_signatures,
};
use osk_psbt::{Aux, Context, KeyRef, Nonce, Psbt, WarningKind, finalize, inspect, sign};

const NET: Network = Network::Regtest;

const LIANA: &str = include_str!("../../../tools/vectors/psbt/wallet-liana.psbt");
const LIANA_POLICY: &str = include_str!("../../../tools/vectors/psbt/wallet-liana.policy");
const TREE: &str = include_str!("../../../tools/vectors/psbt/wallet-tree.psbt");
const TREE_POLICY: &str = include_str!("../../../tools/vectors/psbt/wallet-tree.policy");
const TAPMULTI: &str = include_str!("../../../tools/vectors/psbt/wallet-tapmulti-signed.psbt");
const TAPMULTI_POLICY: &str = include_str!("../../../tools/vectors/psbt/wallet-tapmulti.policy");
const MUSIG: &str = include_str!("../../../tools/vectors/psbt/wallet-musig-core-first-signed.psbt");
const MUSIG_POLICY: &str = include_str!("../../../tools/vectors/psbt/wallet-musig-regtest.policy");
const THRESHOLD: &str = include_str!("../../../tools/vectors/psbt/wallet-threshold-signed.psbt");

fn read(text: &str) -> Psbt {
    Psbt::parse_base64(text.trim()).expect("a committed fixture")
}

fn ctx<'a>(keys: &'a [KeyRef], wallets: &'a [WalletPolicy]) -> Context<'a> {
    Context {
        network: NET,
        keys,
        wallets,
        musig_session: None,
        shares: &[],
        carry: None,
    }
}

/// The verdict on every signature `psbt` carries, with nothing of this
/// device's in the context: what a signature is checked against is the
/// transaction, never what the reader holds.
fn verdicts(psbt: &Psbt) -> Vec<Verdict> {
    verify_signatures(psbt, &ctx(&[], &[]))
        .into_iter()
        .flat_map(|i| i.signatures)
        .map(|s| s.verdict)
        .collect()
}

fn valid(psbt: &Psbt, what: &str) {
    let found = verdicts(psbt);
    assert!(!found.is_empty(), "{what} carries no signature to check");
    assert!(
        found.iter().all(|v| *v == Verdict::Valid),
        "{what}: {found:?}"
    );
}

/// Another key's address, for a recipient that is not ours.
fn foreign() -> ScriptBuf {
    master(ZOO, NET)
        .account_xpub(ScriptType::NativeSegwit, 0)
        .expect("the account")
        .address(false, 1)
        .expect("an address")
        .script_pubkey()
}

/// A single-signature spend of `script_type` over the test key, signed
/// with `nonce` and `aux`.
fn signed_single(script_type: ScriptType, nonce: Nonce, aux: Aux, change: u64) -> Psbt {
    let abandon = master(ABANDON, NET);
    let keys = [KeyRef::from_master(&abandon, 0).expect("accounts")];
    let mut b = Build::new(script_type, NET);
    b.recipients.push((foreign(), 70_000));
    b.change = Some(change);
    let mut psbt = single_sig(&abandon, &b);
    sign(
        &mut psbt,
        &[&abandon],
        &[abandon.fingerprint()],
        &ctx(&keys, &[]),
        false,
        nonce,
        aux,
        &mut None,
        [0u8; 32],
        &[],
        &[],
    )
    .expect("the device signs its own transaction");
    psbt
}

/// The signatures the committed fixtures arrived with verify against the
/// keys they are under.
#[test]
fn every_signature_the_committed_fixtures_carry_verifies() {
    valid(&read(TAPMULTI), "wallet-tapmulti-signed");
    valid(&read(THRESHOLD), "wallet-threshold-signed");
    // What the MuSig2 fixture carries is one participant's partial
    // signature, a BIP-373 field the aggregation checks and this reader
    // does not: there is no signature of the transaction on it yet, and
    // the review says nothing about one.
    let musig = read(MUSIG);
    let policy = WalletPolicy::parse_any(MUSIG_POLICY).expect("the MuSig2 wallet");
    assert!(verdicts(&musig).is_empty());
    assert!(
        inspect(&musig, &ctx(&[], std::slice::from_ref(&policy)))
            .warnings
            .iter()
            .all(|w| !matches!(
                w.kind,
                WarningKind::InvalidSignature { .. } | WarningKind::SignatureUnchecked { .. }
            )),
    );
}

/// The wallets whose fixtures arrive unsigned are signed here, and every
/// signature that leaves this device verifies when it is read back.
#[test]
fn the_signatures_this_device_writes_verify_when_they_are_read_back() {
    for script_type in ScriptType::ALL {
        let psbt = signed_single(script_type, Nonce::LowR, Aux::Deterministic, 29_000);
        valid(&psbt, &format!("{script_type:?} partly signed"));
    }

    // A miniscript wallet and a taproot tree, each signed with the key
    // the path it takes names, partly signed and then finalized.
    for (text, policy_text, words) in [(LIANA, LIANA_POLICY, ABANDON), (TREE, TREE_POLICY, LEGAL)] {
        let mut psbt = read(text);
        let policy = WalletPolicy::parse_any(policy_text).expect("the fixture wallet");
        let key = master(words, NET);
        let keys = [KeyRef::from_master(&key, 0).expect("accounts")];
        let wallets = [policy];
        sign(
            &mut psbt,
            &[&key],
            &[key.fingerprint()],
            &ctx(&keys, &wallets),
            false,
            Nonce::LowR,
            Aux::Deterministic,
            &mut None,
            [0u8; 32],
            &[],
            &[],
        )
        .expect("the key the wallet names signs");
        valid(&psbt, "a recovery wallet's spend");
        finalize(&mut psbt).expect("the script is satisfied");
        valid(&psbt, "a recovery wallet's finalized spend");
    }
}

/// A finalized transaction has no partial-signature fields left: the
/// signatures are inside the witness or the scriptSig, and they are read
/// back out of it and checked there.
#[test]
fn the_signatures_inside_a_finalized_witness_are_read_back_and_verify() {
    for script_type in ScriptType::ALL {
        let mut psbt = signed_single(script_type, Nonce::LowR, Aux::Deterministic, 29_000);
        finalize(&mut psbt).expect("a single-signature input finalizes");
        assert!(
            psbt.inner().inputs[0].partial_sigs.is_empty()
                && psbt.inner().inputs[0].tap_key_sig.is_none(),
            "BIP-174 clears the signing fields"
        );
        valid(&psbt, &format!("{script_type:?} finalized"));
    }

    // A 2-of-2 p2wsh: two signatures in one witness, under two keys the
    // witness script names and the witness itself does not.
    let (abandon, zoo) = (master(ABANDON, NET), master(ZOO, NET));
    let mut psbt = sorted_multisig_change(
        &[&abandon, &zoo],
        2,
        NET,
        100_000,
        &[(foreign(), 70_000)],
        29_000,
    );
    let keys = [
        KeyRef::from_master(&abandon, 0).expect("accounts"),
        KeyRef::from_master(&zoo, 0).expect("accounts"),
    ];
    sign(
        &mut psbt,
        &[&abandon, &zoo],
        &[abandon.fingerprint(), zoo.fingerprint()],
        &ctx(&keys, &[]),
        false,
        Nonce::LowR,
        Aux::Deterministic,
        &mut None,
        [0u8; 32],
        &[],
        &[],
    )
    .expect("both keys sign");
    valid(&psbt, "a 2-of-2 p2wsh, partly signed");
    finalize(&mut psbt).expect("two of two finalizes");
    let found = verdicts(&psbt);
    assert_eq!(found, vec![Verdict::Valid, Verdict::Valid], "{found:?}");

    // A taproot multisig by script path: two leaf signatures in the
    // witness, before the leaf script and its control block.
    let mut tapmulti = read(TAPMULTI);
    let policy = WalletPolicy::parse_any(TAPMULTI_POLICY).expect("the taproot multisig wallet");
    finalize(&mut tapmulti).expect("two of three finalizes");
    let _ = inspect(&tapmulti, &ctx(&[], std::slice::from_ref(&policy)));
    let found = verdicts(&tapmulti);
    assert_eq!(found, vec![Verdict::Valid, Verdict::Valid], "{found:?}");
}

/// One byte of a signature changed: the key did not make it, and the
/// review says so as a danger.
#[test]
fn one_byte_changed_in_a_signature_is_invalid() {
    let signed = signed_single(
        ScriptType::NativeSegwit,
        Nonce::LowR,
        Aux::Deterministic,
        29_000,
    );
    assert_eq!(verdicts(&signed), vec![Verdict::Valid]);

    let mut altered = signed.clone();
    let input = &mut altered.inner_mut().inputs[0];
    let (pk, sig) = input
        .partial_sigs
        .iter()
        .map(|(k, v)| (*k, *v))
        .next()
        .expect("one signature");
    let mut bytes = sig.signature.serialize_compact();
    bytes[40] ^= 0x01;
    let flipped = bitcoin::secp256k1::ecdsa::Signature::from_compact(&bytes)
        .expect("a well-formed signature over other contents");
    input.partial_sigs.insert(
        pk,
        ecdsa::Signature {
            signature: flipped,
            sighash_type: sig.sighash_type,
        },
    );
    assert_eq!(verdicts(&altered), vec![Verdict::Invalid]);

    let abandon = master(ABANDON, NET);
    let keys = [KeyRef::from_master(&abandon, 0).expect("accounts")];
    let insp = inspect(&altered, &ctx(&keys, &[]));
    assert!(
        insp.dangers()
            .iter()
            .any(|k| matches!(k, WarningKind::InvalidSignature { input: 0, .. })),
        "{:?}",
        insp.warnings
    );
}

/// ECDSA with the nonce `k` chosen here rather than by RFC 6979, so that
/// one nonce can sign two different inputs. `s = k⁻¹(z + r·d)`, with the
/// inverse by Fermat over the curve order.
fn ecdsa_with_k(
    secp: &Secp256k1<bitcoin::secp256k1::All>,
    d: &SecretKey,
    z: [u8; 32],
    k: &SecretKey,
) -> bitcoin::secp256k1::ecdsa::Signature {
    /// The curve order less two, the exponent Fermat's inverse uses.
    const N_MINUS_2: [u8; 32] = [
        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
        0xfe, 0xba, 0xae, 0xdc, 0xe6, 0xaf, 0x48, 0xa0, 0x3b, 0xbf, 0xd2, 0x5e, 0x8c, 0xd0, 0x36,
        0x41, 0x3f,
    ];
    let point = PublicKey::from_secret_key(secp, k);
    let x: [u8; 32] = point.serialize()[1..33].try_into().expect("32 bytes");
    let r = SecretKey::from_slice(&x).expect("the x coordinate is a scalar");
    let z = SecretKey::from_slice(&z).expect("the message is a scalar");
    let rd = d
        .mul_tweak(&Scalar::from(r))
        .expect("a product inside the order");
    let zrd = rd
        .add_tweak(&Scalar::from(z))
        .expect("a sum inside the order");
    // k⁻¹ = k^(n-2).
    let mut inverse: Option<SecretKey> = None;
    for byte in N_MINUS_2 {
        for bit in (0..8).rev() {
            if let Some(acc) = inverse {
                inverse = Some(acc.mul_tweak(&Scalar::from(acc)).expect("a square"));
            }
            if byte >> bit & 1 == 1 {
                inverse = Some(match inverse {
                    Some(acc) => acc.mul_tweak(&Scalar::from(*k)).expect("a product"),
                    None => *k,
                });
            }
        }
    }
    let s = zrd
        .mul_tweak(&Scalar::from(inverse.expect("an inverse")))
        .expect("a product");
    let mut compact = [0u8; 64];
    compact[..32].copy_from_slice(&r.secret_bytes());
    compact[32..].copy_from_slice(&s.secret_bytes());
    let mut signature = bitcoin::secp256k1::ecdsa::Signature::from_compact(&compact)
        .expect("a well-formed signature");
    signature.normalize_s();
    signature
}

/// Two inputs signed with one nonce: both signatures are good, and
/// together they are the private key. The review blocks on it.
#[test]
fn two_signatures_under_one_nonce_are_found() {
    let abandon = master(ABANDON, NET);
    let keys = [KeyRef::from_master(&abandon, 0).expect("accounts")];
    let mut b = Build::new(ScriptType::NativeSegwit, NET);
    b.recipients.push((foreign(), 70_000));
    b.change = Some(29_000);
    let mut psbt = single_sig(&abandon, &b);

    // A second input of the same address, so that one key signs twice.
    let account = abandon
        .account_xpub(ScriptType::NativeSegwit, 0)
        .expect("the account");
    let script = account
        .address(false, 0)
        .expect("an address")
        .script_pubkey();
    let previous = funding_tx(script.clone(), 50_000);
    {
        let inner = psbt.inner_mut();
        inner.unsigned_tx.input.push(TxIn {
            previous_output: OutPoint::new(previous.compute_txid(), 0),
            script_sig: ScriptBuf::new(),
            sequence: Sequence::ENABLE_RBF_NO_LOCKTIME,
            witness: Witness::new(),
        });
        let mut second = inner.inputs[0].clone();
        second.witness_utxo = Some(TxOut {
            value: Amount::from_sat(50_000),
            script_pubkey: script,
        });
        second.non_witness_utxo = Some(previous);
        inner.inputs.push(second);
    }

    let secp = Secp256k1::new();
    let path = DerivationPath::from_str("m/84h/1h/0h/0/0").expect("the key's own path");
    let derived = abandon.derive(&path);
    let k = SecretKey::from_slice(&[7u8; 32]).expect("a nonce");
    for index in 0..2 {
        let tx = psbt.unsigned_tx().clone();
        let mut cache = SighashCache::new(&tx);
        let (msg, _) = psbt
            .inner()
            .sighash_ecdsa(index, &mut cache)
            .expect("the sighash");
        let signature = ecdsa_with_k(&secp, derived.secret_key(), *msg.as_ref(), &k);
        psbt.inner_mut().inputs[index].partial_sigs.insert(
            bitcoin::PublicKey::new(derived.to_xpub().public_key),
            ecdsa::Signature {
                signature,
                sighash_type: EcdsaSighashType::All,
            },
        );
    }

    let found = verdicts(&psbt);
    assert_eq!(found, vec![Verdict::Valid, Verdict::Valid], "{found:?}");
    let reuse = repeated_nonces(&psbt, &ctx(&keys, &[]));
    assert_eq!(reuse.len(), 1, "{reuse:?}");
    assert_eq!(reuse[0].inputs, vec![0, 1]);

    let insp = inspect(&psbt, &ctx(&keys, &[]));
    assert!(
        insp.dangers()
            .iter()
            .any(|k| matches!(k, WarningKind::NonceReuse { .. })),
        "{:?}",
        insp.warnings
    );
}

/// A signature this device made is recognised as the rule that made it:
/// low R, the first nonce, BIP-340 with zero auxiliary randomness — and
/// a taproot signature made with fresh randomness matches none of them.
#[test]
fn the_devices_own_signatures_are_recognised_by_their_nonce_rule() {
    let abandon = master(ABANDON, NET);
    let psbt = signed_single(
        ScriptType::NativeSegwit,
        Nonce::LowR,
        Aux::Deterministic,
        29_000,
    );
    let found = deterministic(&psbt, &[&abandon]);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].mode, Some(NonceMode::LowR));

    // The first nonce is only a different signature where grinding
    // would have moved it, so the transaction the rule is read off is
    // one whose first nonce gives a high `r`.
    let mut told = None;
    for change in 29_000..29_040u64 {
        let psbt = signed_single(
            ScriptType::NativeSegwit,
            Nonce::First,
            Aux::Deterministic,
            change,
        );
        let found = deterministic(&psbt, &[&abandon]);
        assert_eq!(found.len(), 1, "{found:?}");
        let mode = found[0].mode.expect("the device's own signature");
        if mode == NonceMode::First {
            told = Some(mode);
            break;
        }
        assert_eq!(
            mode,
            NonceMode::LowR,
            "a first nonce whose r is already low is the ground signature too"
        );
    }
    assert_eq!(told, Some(NonceMode::First), "no high r in forty tries");

    for (aux, mode) in [
        (Aux::Deterministic, Some(NonceMode::Bip340)),
        (Aux::Fresh([9u8; 32]), None),
    ] {
        let psbt = signed_single(ScriptType::Taproot, Nonce::LowR, aux, 29_000);
        let found = deterministic(&psbt, &[&abandon]);
        assert_eq!(found.len(), 1, "{found:?}");
        assert_eq!(found[0].mode, mode, "{found:?}");
    }
}

/// A key this device does not hold has no determinism to report: the
/// check needs the key, and nothing else stands in for it.
#[test]
fn a_signature_of_another_device_is_not_recomputed() {
    let psbt = signed_single(
        ScriptType::NativeSegwit,
        Nonce::LowR,
        Aux::Deterministic,
        29_000,
    );
    assert!(deterministic(&psbt, &[&master(ZOO, NET)]).is_empty());
}

/// Two PSBTs compared: the same transaction written twice is the same
/// transaction, an output's amount changed is that output, and a
/// signature added is the signing state.
#[test]
fn two_transactions_are_compared_on_what_they_decode_to() {
    let abandon = master(ABANDON, NET);
    let keys = [KeyRef::from_master(&abandon, 0).expect("accounts")];
    let mut b = Build::new(ScriptType::NativeSegwit, NET);
    b.recipients.push((foreign(), 70_000));
    b.change = Some(29_000);
    let unsigned = single_sig(&abandon, &b);

    let again = Psbt::parse_bytes(&unsigned.to_bytes()).expect("a round trip");
    assert!(compare(&unsigned, &again).same());

    let mut richer = unsigned.clone();
    richer.inner_mut().unsigned_tx.output[0].value = Amount::from_sat(69_000);
    let diff = compare(&unsigned, &richer);
    assert!(!diff.same());
    assert_eq!(diff.transaction.len(), 1, "{:?}", diff.transaction);
    assert_eq!(diff.transaction[0].field, Field::Amount);
    assert!(diff.signing.is_empty() && diff.metadata.is_empty());

    let mut signed = unsigned.clone();
    sign(
        &mut signed,
        &[&abandon],
        &[abandon.fingerprint()],
        &ctx(&keys, &[]),
        false,
        Nonce::LowR,
        Aux::Deterministic,
        &mut None,
        [0u8; 32],
        &[],
        &[],
    )
    .expect("signed");
    let diff = compare(&unsigned, &signed);
    assert!(diff.transaction.is_empty(), "{:?}", diff.transaction);
    assert_eq!(diff.signing.len(), 1, "{:?}", diff.signing);
}

/// An inscription envelope in a script is named, with the content type
/// it states and the size of its body. Nothing of it is rendered.
#[test]
fn an_inscription_envelope_is_stated() {
    let abandon = master(ABANDON, NET);
    let mut b = Build::new(ScriptType::Taproot, NET);
    b.recipients.push((foreign(), 70_000));
    b.change = Some(29_000);
    let mut psbt = single_sig(&abandon, &b);
    let body: Vec<u8> = vec![0x41u8; 400];
    let envelope = bitcoin::script::Builder::new()
        .push_opcode(bitcoin::opcodes::OP_FALSE)
        .push_opcode(bitcoin::opcodes::all::OP_IF)
        .push_slice(b"ord")
        .push_slice([1u8])
        .push_slice(b"text/plain;charset=utf-8")
        .push_slice([])
        .push_slice(bitcoin::script::PushBytesBuf::try_from(body.clone()).expect("a body"))
        .push_opcode(bitcoin::opcodes::all::OP_ENDIF)
        .into_script();
    psbt.inner_mut().inputs[0].witness_script = Some(envelope);

    let found = inscriptions(&psbt);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(
        found[0].content_type.as_deref(),
        Some("text/plain;charset=utf-8")
    );
    assert_eq!(found[0].size, Some(body.len()));

    let keys = [KeyRef::from_master(&abandon, 0).expect("accounts")];
    let insp = inspect(&psbt, &ctx(&keys, &[]));
    assert!(
        insp.warnings.iter().any(
            |w| matches!(w.kind, WarningKind::Inscription { input: 0, .. })
                && w.level == osk_psbt::Level::Info
        ),
        "{:?}",
        insp.warnings
    );
}

/// An input with no previous output states that its signature was not
/// checked, rather than passing it off as either answer.
#[test]
fn a_signature_with_no_previous_output_is_not_checked() {
    let mut psbt = signed_single(
        ScriptType::NativeSegwit,
        Nonce::LowR,
        Aux::Deterministic,
        29_000,
    );
    let input = &mut psbt.inner_mut().inputs[0];
    input.witness_utxo = None;
    input.non_witness_utxo = None;
    assert_eq!(
        verdicts(&psbt),
        vec![Verdict::Unchecked(Unchecked::NoPrevout)]
    );
    let insp = inspect(&psbt, &ctx(&[], &[]));
    assert!(
        insp.warnings
            .iter()
            .any(|w| matches!(w.kind, WarningKind::SignatureUnchecked { .. })),
        "{:?}",
        insp.warnings
    );
}
