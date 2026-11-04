//! What a signature already on a transaction is checked for:
//! every signature the committed fixtures
//! carry verifies, a finalized witness is read back and verifies, one
//! byte changed is invalid, two signatures under one nonce are found, a
//! signature this device made is recognised as the rule that made it,
//! two PSBTs are compared on their contents, an inscription
//! envelope is stated, and a signature made under a sighash its input
//! did not ask for is flagged.

use std::str::FromStr;

use crate::common::builder::{
    Build, funding_tx, multisig_policy, single_sig, sorted_multisig_change,
};
use crate::common::{ABANDON, LEGAL, ZOO, master};
use bitcoin::bip32::DerivationPath;
use bitcoin::hashes::Hash;
use bitcoin::key::TapTweak;
use bitcoin::psbt::PsbtSighashType;
use bitcoin::secp256k1::{Keypair, Message, PublicKey, Scalar, Secp256k1, SecretKey};
use bitcoin::sighash::{EcdsaSighashType, Prevouts, SighashCache, TapSighashType};
use bitcoin::{Amount, OutPoint, ScriptBuf, Sequence, TxIn, TxOut, Witness, ecdsa, taproot};
use faraday_bip::keys::{Network, ScriptType};
use faraday_bip::policy::WalletPolicy;
use faraday_crypto::work::Compute;
use faraday_psbt::verify::{
    Field, NonceMode, SigKey, Unchecked, Verdict, compare, deterministic, final_invalid,
    final_unread, inscriptions, not_evaluated, repeated_nonces, states_false, utxos_disagree,
    verify_signatures, witness_mismatch, witness_shape_mismatch,
};
use faraday_psbt::{
    Aux, Context, Detail, KeyRef, Level, Nonce, Psbt, WarningKind, finalize, inspect, is_complete,
    sign,
};

const NET: Network = Network::Regtest;

const LIANA: &str = include_str!("../../../../tools/vectors/psbt/wallet-liana.psbt");
const LIANA_POLICY: &str = include_str!("../../../../tools/vectors/psbt/wallet-liana.policy");
const TREE: &str = include_str!("../../../../tools/vectors/psbt/wallet-tree.psbt");
const TREE_POLICY: &str = include_str!("../../../../tools/vectors/psbt/wallet-tree.policy");
const TAPMULTI: &str = include_str!("../../../../tools/vectors/psbt/wallet-tapmulti-signed.psbt");
const TAPMULTI_POLICY: &str = include_str!("../../../../tools/vectors/psbt/wallet-tapmulti.policy");
const MUSIG: &str =
    include_str!("../../../../tools/vectors/psbt/wallet-musig-core-first-signed.psbt");
const MUSIG_POLICY: &str =
    include_str!("../../../../tools/vectors/psbt/wallet-musig-regtest.policy");

fn read(text: &str) -> Psbt {
    let c = Compute::unchecked();
    Psbt::parse_base64(&c, text.trim()).expect("a committed fixture")
}

fn ctx<'a>(keys: &'a [KeyRef], wallets: &'a [WalletPolicy]) -> Context<'a> {
    Context {
        network: NET,
        keys,
        wallets,
        musig_session: None,
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
    let c = Compute::unchecked();
    master(ZOO, NET)
        .account_xpub(&c, ScriptType::NativeSegwit, 0)
        .expect("the account")
        .address(false, 1)
        .expect("an address")
        .script_pubkey()
}

/// A single-signature spend of `script_type` over the test key, signed
/// with `nonce` and `aux`.
fn signed_single(script_type: ScriptType, nonce: Nonce, aux: Aux, change: u64) -> Psbt {
    let c = Compute::unchecked();
    let abandon = master(ABANDON, NET);
    let keys = [KeyRef::from_master(&c, &abandon, 0).expect("accounts")];
    let mut b = Build::new(script_type, NET);
    b.recipients.push((foreign(), 70_000));
    b.change = Some(change);
    let mut psbt = single_sig(&abandon, &b);
    sign(
        &c,
        &mut psbt,
        &[&abandon],
        &[abandon.fingerprint()],
        &ctx(&keys, &[]),
        &[],
        nonce,
        aux,
        &mut None,
        &faraday_crypto::Zeroizing::new([0u8; 32]),
    )
    .expect("the device signs its own transaction");
    psbt
}

/// The signatures the committed fixtures arrived with verify against the
/// keys they are under.
#[test]
fn every_signature_the_committed_fixtures_carry_verifies() {
    let c = Compute::unchecked();
    valid(&read(TAPMULTI), "wallet-tapmulti-signed");
    // What the MuSig2 fixture carries is one participant's partial
    // signature, a BIP-373 field the aggregation checks and this reader
    // does not: there is no signature of the transaction on it yet, and
    // the review says nothing about one.
    let musig = read(MUSIG);
    let policy = WalletPolicy::parse_any(&c, MUSIG_POLICY).expect("the MuSig2 wallet");
    assert!(verdicts(&musig).is_empty());
    assert!(
        inspect(&c, &musig, &ctx(&[], std::slice::from_ref(&policy)))
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
    let c = Compute::unchecked();
    for script_type in ScriptType::ALL {
        let psbt = signed_single(script_type, Nonce::LowR, Aux::Deterministic, 29_000);
        valid(&psbt, &format!("{script_type:?} partly signed"));
    }

    // A miniscript wallet and a taproot tree, each signed with the key
    // the path it takes names, partly signed and then finalized.
    for (text, policy_text, words) in [(LIANA, LIANA_POLICY, ABANDON), (TREE, TREE_POLICY, LEGAL)] {
        let mut psbt = read(text);
        let policy = WalletPolicy::parse_any(&c, policy_text).expect("the fixture wallet");
        let key = master(words, NET);
        let keys = [KeyRef::from_master(&c, &key, 0).expect("accounts")];
        let wallets = [policy];
        sign(
            &c,
            &mut psbt,
            &[&key],
            &[key.fingerprint()],
            &ctx(&keys, &wallets),
            &[],
            Nonce::LowR,
            Aux::Deterministic,
            &mut None,
            &faraday_crypto::Zeroizing::new([0u8; 32]),
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
    let c = Compute::unchecked();
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
        KeyRef::from_master(&c, &abandon, 0).expect("accounts"),
        KeyRef::from_master(&c, &zoo, 0).expect("accounts"),
    ];
    sign(
        &c,
        &mut psbt,
        &[&abandon, &zoo],
        &[abandon.fingerprint(), zoo.fingerprint()],
        &ctx(&keys, &[multisig_policy(&[&abandon, &zoo], 2, NET)]),
        &[],
        Nonce::LowR,
        Aux::Deterministic,
        &mut None,
        &faraday_crypto::Zeroizing::new([0u8; 32]),
    )
    .expect("both keys sign their registered wallet");
    valid(&psbt, "a 2-of-2 p2wsh, partly signed");
    finalize(&mut psbt).expect("two of two finalizes");
    let found = verdicts(&psbt);
    assert_eq!(found, vec![Verdict::Valid, Verdict::Valid], "{found:?}");

    // A taproot multisig by script path: two leaf signatures in the
    // witness, before the leaf script and its control block.
    let mut tapmulti = read(TAPMULTI);
    let policy = WalletPolicy::parse_any(&c, TAPMULTI_POLICY).expect("the taproot multisig wallet");
    finalize(&mut tapmulti).expect("two of three finalizes");
    let _ = inspect(&c, &tapmulti, &ctx(&[], std::slice::from_ref(&policy)));
    let found = verdicts(&tapmulti);
    assert_eq!(found, vec![Verdict::Valid, Verdict::Valid], "{found:?}");
}

/// BIP-341 makes a 65-byte taproot signature with sighash byte 0
/// invalid, as Bitcoin Core refuses it: in a finalized witness too, for a
/// key path and for a script-path leaf.
#[test]
fn a_finalized_taproot_signature_with_sighash_byte_zero_is_invalid() {
    let with_zero = |psbt: &mut Psbt, at: &[usize], zero: bool| {
        let mut elements: Vec<Vec<u8>> = psbt.inner().inputs[0]
            .final_script_witness
            .as_ref()
            .expect("a finalized witness")
            .iter()
            .map(<[u8]>::to_vec)
            .collect();
        for i in at {
            elements[*i].truncate(64);
            if zero {
                elements[*i].push(0);
            }
        }
        psbt.inner_mut().inputs[0].final_script_witness = Some(Witness::from_slice(&elements));
    };

    let mut key_path = signed_single(ScriptType::Taproot, Nonce::LowR, Aux::Deterministic, 29_000);
    finalize(&mut key_path).expect("a single-signature input finalizes");
    with_zero(&mut key_path, &[0], false);
    valid(&key_path, "the 64-byte key-path signature");
    with_zero(&mut key_path, &[0], true);
    assert_eq!(verdicts(&key_path), vec![Verdict::Invalid]);
    let spk = &key_path.inner().inputs[0]
        .witness_utxo
        .as_ref()
        .expect("the spent output")
        .script_pubkey;
    let output_key =
        bitcoin::XOnlyPublicKey::from_slice(&spk.as_bytes()[2..34]).expect("a p2tr output key");
    let named: Vec<_> = verify_signatures(&key_path, &ctx(&[], &[]))
        .into_iter()
        .flat_map(|i| i.signatures)
        .map(|s| s.key)
        .collect();
    assert!(
        named == [SigKey::Schnorr(output_key)],
        "the key-path signature is named by the output key: {named:?}"
    );

    let mut leaf = read(TAPMULTI);
    finalize(&mut leaf).expect("two of three finalizes");
    with_zero(&mut leaf, &[0], false);
    valid(&leaf, "the 64-byte leaf signatures");
    with_zero(&mut leaf, &[0], true);
    let found = verdicts(&leaf);
    assert!(
        found.contains(&Verdict::Invalid) && found.len() == 2,
        "{found:?}"
    );
}

/// A finalized script-path spend of `script`, whose witness is
/// `elements(good_signature)` then the leaf script and a control block.
fn leaf_spend(
    script: impl Fn(&bitcoin::XOnlyPublicKey) -> ScriptBuf,
    elements: impl Fn(Vec<u8>) -> Vec<Vec<u8>>,
) -> Psbt {
    leaf_spend_two(|a, _| script(a), |sig_a, _, _| elements(sig_a))
}

/// As [`leaf_spend`] with a second key B: the script takes both x-only
/// keys and the elements take A's signature, B's signature and B's key.
fn leaf_spend_two(
    script: impl Fn(&bitcoin::XOnlyPublicKey, &bitcoin::XOnlyPublicKey) -> ScriptBuf,
    elements: impl Fn(Vec<u8>, Vec<u8>, Vec<u8>) -> Vec<Vec<u8>>,
) -> Psbt {
    leaf_spend_with(script, |l| {
        let mut witness = elements(
            l.sign(0, TapSighashType::Default, None, None),
            l.sign(1, TapSighashType::Default, None, None),
            l.xonly[1].serialize().to_vec(),
        );
        witness.push(l.script.to_bytes());
        witness.push(l.control.clone());
        witness
    })
}

/// What a finalized leaf spend's witness is made from: the leaf, its
/// control block, the keys A (0) and B (1) and the spend to sign.
struct LeafSpend {
    script: ScriptBuf,
    control: Vec<u8>,
    xonly: [bitcoin::XOnlyPublicKey; 2],
    keypairs: [Keypair; 2],
    tx: bitcoin::Transaction,
    prev: TxOut,
}

impl LeafSpend {
    /// A real signature of key `who` over the leaf spend, with the sighash
    /// type it claims (its byte appended unless DEFAULT), an annex and the
    /// last executed CODESEPARATOR position.
    fn sign(
        &self,
        who: usize,
        ty: TapSighashType,
        annex: Option<&[u8]>,
        codesep: Option<u32>,
    ) -> Vec<u8> {
        let leaf = taproot::TapLeafHash::from_script(&self.script, taproot::LeafVersion::TapScript);
        let annex = annex.map(|a| bitcoin::sighash::Annex::new(a).expect("an annex"));
        let hash = SighashCache::new(&self.tx)
            .taproot_signature_hash(
                0,
                &Prevouts::All(std::slice::from_ref(&self.prev)),
                annex,
                Some((leaf, codesep.unwrap_or(0xffff_ffff))),
                ty,
            )
            .expect("the sighash");
        let message = Message::from_digest(hash.to_byte_array());
        let mut sig = Secp256k1::new()
            .sign_schnorr_no_aux_rand(&message, &self.keypairs[who])
            .serialize()
            .to_vec();
        if ty != TapSighashType::Default {
            sig.push(ty as u8);
        }
        sig
    }
}

/// A finalized script-path spend of `script(A, B)` whose whole witness
/// `witness` makes.
fn leaf_spend_with(
    script: impl Fn(&bitcoin::XOnlyPublicKey, &bitcoin::XOnlyPublicKey) -> ScriptBuf,
    witness: impl Fn(&LeafSpend) -> Vec<Vec<u8>>,
) -> Psbt {
    let mut psbt = signed_single(ScriptType::Taproot, Nonce::LowR, Aux::Deterministic, 29_000);
    finalize(&mut psbt).expect("a single-signature input finalizes");
    leaf_spend_on(psbt, script, witness)
}

/// As [`leaf_spend_with`] on a PSBT already finalized, whose transaction
/// the signatures commit to. The output it spends is a real one: key A
/// tweaked by the tree that holds the leaf.
fn leaf_spend_on(
    psbt: Psbt,
    script: impl Fn(&bitcoin::XOnlyPublicKey, &bitcoin::XOnlyPublicKey) -> ScriptBuf,
    witness: impl Fn(&LeafSpend) -> Vec<Vec<u8>>,
) -> Psbt {
    leaf_spend_tree(psbt, 7, 0, script, witness)
}

/// As [`leaf_spend_on`] with key A made from `seed`, and the leaf `depth`
/// levels down a tree whose other leaves are `OP_TRUE`.
fn leaf_spend_tree(
    mut psbt: Psbt,
    seed: u8,
    depth: u8,
    script: impl Fn(&bitcoin::XOnlyPublicKey, &bitcoin::XOnlyPublicKey) -> ScriptBuf,
    witness: impl Fn(&LeafSpend) -> Vec<Vec<u8>>,
) -> Psbt {
    let secp = Secp256k1::new();
    let keypair =
        Keypair::from_secret_key(&secp, &SecretKey::from_slice(&[seed; 32]).expect("key"));
    let keypair_b = Keypair::from_secret_key(&secp, &SecretKey::from_slice(&[9; 32]).expect("key"));
    let (xonly, _) = keypair.x_only_public_key();
    let (xonly_b, _) = keypair_b.x_only_public_key();
    let script = script(&xonly, &xonly_b);
    let filler = ScriptBuf::from_bytes(vec![0x51]);
    let mut builder = taproot::TaprootBuilder::new();
    for d in 1..depth {
        builder = builder.add_leaf(d, filler.clone()).expect("a filler leaf");
    }
    builder = builder.add_leaf(depth, script.clone()).expect("the leaf");
    if depth > 0 {
        builder = builder.add_leaf(depth, filler).expect("a filler leaf");
    }
    let info = builder.finalize(&secp, xonly).expect("a complete tree");
    let control = info
        .control_block(&(script.clone(), taproot::LeafVersion::TapScript))
        .expect("the leaf is in the tree")
        .serialize();
    let mut prev = psbt.inner().inputs[0]
        .witness_utxo
        .clone()
        .expect("the output it spends");
    prev.script_pubkey = ScriptBuf::new_p2tr_tweaked(info.output_key());
    psbt.inner_mut().inputs[0].witness_utxo = Some(prev.clone());
    let spend = LeafSpend {
        script,
        control,
        xonly: [xonly, xonly_b],
        keypairs: [keypair, keypair_b],
        tx: psbt.unsigned_tx().clone(),
        prev,
    };
    let witness = witness(&spend);
    psbt.inner_mut().inputs[0].final_script_witness = Some(Witness::from_slice(&witness));
    psbt
}

/// `OP_SHA256 <hash of the preimage> OP_EQUALVERIFY <key> OP_CHECKSIG`:
/// no `SIZE` check, so not miniscript.
fn hashlock_leaf(preimage: &[u8]) -> impl Fn(&bitcoin::XOnlyPublicKey) -> ScriptBuf {
    let hash = bitcoin::hashes::sha256::Hash::hash(preimage).to_byte_array();
    move |key| {
        bitcoin::script::Builder::new()
            .push_opcode(bitcoin::opcodes::all::OP_SHA256)
            .push_slice(hash)
            .push_opcode(bitcoin::opcodes::all::OP_EQUALVERIFY)
            .push_x_only_key(key)
            .push_opcode(bitcoin::opcodes::all::OP_CHECKSIG)
            .into_script()
    }
}

/// A hashlock leaf pushing the hash `kind` makes of the preimage: 0 SHA256,
/// 1 HASH256, 2 RIPEMD160, 3 HASH160.
fn hashlock_leaf_of(kind: u8, preimage: &[u8]) -> impl Fn(&bitcoin::XOnlyPublicKey) -> ScriptBuf {
    use bitcoin::hashes::{hash160, ripemd160, sha256, sha256d};
    use bitcoin::opcodes::all::{OP_EQUALVERIFY, OP_HASH160, OP_HASH256, OP_RIPEMD160, OP_SHA256};
    let (op, hash) = match kind {
        0 => (
            OP_SHA256,
            sha256::Hash::hash(preimage).to_byte_array().to_vec(),
        ),
        1 => (
            OP_HASH256,
            sha256d::Hash::hash(preimage).to_byte_array().to_vec(),
        ),
        2 => (
            OP_RIPEMD160,
            ripemd160::Hash::hash(preimage).to_byte_array().to_vec(),
        ),
        _ => (
            OP_HASH160,
            hash160::Hash::hash(preimage).to_byte_array().to_vec(),
        ),
    };
    move |key| {
        bitcoin::script::Builder::new()
            .push_opcode(op)
            .push_slice(<&bitcoin::script::PushBytes>::try_from(hash.as_slice()).expect("a push"))
            .push_opcode(OP_EQUALVERIFY)
            .push_x_only_key(key)
            .push_opcode(bitcoin::opcodes::all::OP_CHECKSIG)
            .into_script()
    }
}

fn spoiled(mut sig: Vec<u8>) -> Vec<u8> {
    sig[5] ^= 1;
    sig
}

/// A witness element whose hash the leaf pushes is data: a preimage of 64
/// bytes is not called an invalid signature. An element whose hash is not
/// pushed is judged as a signature, so a bad one is invalid.
#[test]
fn a_64_byte_preimage_in_a_finalized_script_path_witness_is_not_an_invalid_signature() {
    let preimage = vec![0xab; 64];
    let psbt = leaf_spend(hashlock_leaf(&preimage), |sig| vec![sig, preimage.clone()]);
    let found = verdicts(&psbt);
    assert!(found.contains(&Verdict::Valid), "{found:?}");
    assert!(!found.contains(&Verdict::Invalid), "{found:?}");

    // A bad signature beside the preimage is invalid, the preimage is not.
    let psbt = leaf_spend(hashlock_leaf(&preimage), |sig| {
        vec![spoiled(sig), preimage.clone()]
    });
    let mut found = verdicts(&psbt);
    found.sort_by_key(|v| format!("{v:?}"));
    assert_eq!(
        found,
        vec![
            Verdict::Invalid,
            Verdict::Unchecked(Unchecked::UnsupportedScript)
        ],
        "{found:?}"
    );

    // The 65-byte form ending 0x00 is no signature either way: invalid.
    let psbt = leaf_spend(hashlock_leaf(&preimage), |mut sig| {
        sig.push(0);
        vec![sig, preimage.clone()]
    });
    assert!(verdicts(&psbt).contains(&Verdict::Invalid));
}

/// The same for the other hash the leaf may push, and a 64-byte element
/// whose hash is not the push is a bad signature as before.
#[test]
fn a_preimage_is_data_for_each_hash_and_an_element_hashing_to_nothing_is_not() {
    let preimage = vec![0xab; 64];
    for kind in 0..4 {
        let psbt = leaf_spend(hashlock_leaf_of(kind, &preimage), |sig| {
            vec![sig, preimage.clone()]
        });
        let found = verdicts(&psbt);
        assert!(!found.contains(&Verdict::Invalid), "{kind}: {found:?}");
        let insp = leaf_cards(psbt, PsbtSighashType::from_u32(1));
        assert!(
            !insp.warnings.iter().any(|w| w.level == Level::Blocked
                || (w.level == Level::Danger && w.kind != WarningKind::NoParticipatingKey)),
            "{kind}: {:?}",
            insp.warnings
        );
    }
    // The leaf pushes the hash of one preimage, the witness gives another.
    let other = vec![0xac; 64];
    let psbt = leaf_spend(hashlock_leaf(&preimage), |sig| vec![sig, other.clone()]);
    assert!(verdicts(&psbt).contains(&Verdict::Invalid));
    let insp = leaf_cards(psbt, PsbtSighashType::from_u32(0));
    assert!(insp.refuses(), "{:?}", insp.warnings);
}

/// A bad signature beside a 32-byte preimage is invalid.
#[test]
fn a_bad_signature_in_a_finalized_miniscript_leaf_is_invalid() {
    let preimage = vec![0xcd; 32];
    let hash = bitcoin::hashes::sha256::Hash::hash(&preimage).to_byte_array();
    // and_v(v:sha256(h),pk(K))
    let leaf = move |key: &bitcoin::XOnlyPublicKey| {
        use bitcoin::opcodes::all::{OP_CHECKSIG, OP_EQUALVERIFY, OP_SHA256, OP_SIZE};
        bitcoin::script::Builder::new()
            .push_opcode(OP_SIZE)
            .push_int(32)
            .push_opcode(OP_EQUALVERIFY)
            .push_opcode(OP_SHA256)
            .push_slice(hash)
            .push_opcode(OP_EQUALVERIFY)
            .push_x_only_key(key)
            .push_opcode(OP_CHECKSIG)
            .into_script()
    };
    let psbt = leaf_spend(leaf, |sig| vec![sig, preimage.clone()]);
    valid(&psbt, "the miniscript leaf's signature");
    let psbt = leaf_spend(leaf, |sig| vec![spoiled(sig), preimage.clone()]);
    assert!(verdicts(&psbt).contains(&Verdict::Invalid));
    let psbt = leaf_spend(leaf, |mut sig| {
        sig.push(0);
        vec![sig, preimage.clone()]
    });
    assert!(verdicts(&psbt).contains(&Verdict::Invalid));
}

/// The cards `inspect` gives for a finalized leaf spend whose input asks
/// `asked`.
fn leaf_cards(mut psbt: Psbt, asked: PsbtSighashType) -> faraday_psbt::Inspection {
    psbt.inner_mut().inputs[0].sighash_type = Some(asked);
    inspect(&Compute::unchecked(), &psbt, &ctx(&[], &[]))
}

fn sighash_cards(insp: &faraday_psbt::Inspection) -> usize {
    insp.warnings
        .iter()
        .filter(|w| matches!(w.kind, WarningKind::SighashMismatch { .. }))
        .count()
}

/// A data element whose hash the leaf pushes is not a signature,
/// so the sighash its last byte would read as is not compared with what
/// the input asks, and signing is not refused for it.
#[test]
fn a_65_byte_preimage_is_not_read_as_a_signature_with_a_sighash() {
    for last in [0x00, 0x02, 0x03, 0x81, 0x82, 0x83] {
        let mut preimage = vec![0xab; 64];
        preimage.push(last);
        let psbt = leaf_spend(hashlock_leaf(&preimage), |sig| vec![sig, preimage.clone()]);
        let insp = leaf_cards(psbt, PsbtSighashType::from_u32(0));
        assert_eq!(sighash_cards(&insp), 0, "{last:#x}: {:?}", insp.warnings);
        // Nothing is refused for the sighash (the test holds no key, so
        // the missing signer is the one refusal left).
        assert!(
            !insp.warnings.iter().any(|w| w.level == Level::Blocked
                || (w.level == Level::Danger && w.kind != WarningKind::NoParticipatingKey)),
            "{last:#x}: {:?}",
            insp.warnings
        );
    }
}

/// The real signature is DEFAULT and differs from ALL|ANYONECANPAY, so it
/// gets one card; the 64-byte preimage adds none.
#[test]
fn a_64_byte_preimage_has_no_sighash_card_when_the_input_asks_another() {
    let preimage = vec![0xab; 64];
    let psbt = leaf_spend(hashlock_leaf(&preimage), |sig| vec![sig, preimage.clone()]);
    let insp = leaf_cards(psbt, PsbtSighashType::from_u32(0x81));
    assert_eq!(sighash_cards(&insp), 1, "{:?}", insp.warnings);
}

/// A `pk(K)` leaf holding a bad 65-byte element (sighash 0x02) is a bad
/// signature, which the sighash cards still compare, under a key no
/// element verified under.
#[test]
fn a_bad_65_byte_element_in_a_pk_leaf_is_invalid_and_has_its_sighash_card() {
    let leaf = |key: &bitcoin::XOnlyPublicKey| {
        bitcoin::script::Builder::new()
            .push_x_only_key(key)
            .push_opcode(bitcoin::opcodes::all::OP_CHECKSIG)
            .into_script()
    };
    let bad = |_sig: Vec<u8>| {
        let mut e = vec![0x55; 64];
        e.push(0x02);
        vec![e]
    };
    let psbt = leaf_spend(leaf, bad);
    assert!(verdicts(&psbt).contains(&Verdict::Invalid));
    let insp = leaf_cards(psbt, PsbtSighashType::from_u32(1));
    assert!(
        insp.warnings
            .iter()
            .any(|w| w.level == Level::Blocked && matches!(w.detail, Detail::SignedNone { .. })),
        "{:?}",
        insp.warnings
    );
}

/// `DUP HASH160 <hash of B> EQUALVERIFY CHECKSIG`: `pkh(B)`.
fn pkh_leaf(key: &bitcoin::XOnlyPublicKey) -> ScriptBuf {
    use bitcoin::opcodes::all::{OP_CHECKSIG, OP_DUP, OP_EQUALVERIFY, OP_HASH160};
    let h = bitcoin::hashes::hash160::Hash::hash(&key.serialize()).to_byte_array();
    bitcoin::script::Builder::new()
        .push_opcode(OP_DUP)
        .push_opcode(OP_HASH160)
        .push_slice(h)
        .push_opcode(OP_EQUALVERIFY)
        .push_opcode(OP_CHECKSIG)
        .into_script()
}

/// The leaf's only key is a hash: the signature is checked under the key
/// the witness gives (B), and a real NONE signature under B, which the
/// input asking ALL does not, is refused.
#[test]
fn a_signature_in_a_pkh_leaf_is_valid_under_the_witness_key() {
    let psbt = leaf_spend_two(|_, b| pkh_leaf(b), |_, sig_b, b| vec![sig_b, b]);
    let c = ctx(&[], &[]);
    let key = verify_signatures(&psbt, &c)
        .into_iter()
        .flat_map(|i| i.signatures)
        .map(|s| s.key)
        .collect::<Vec<_>>();
    let b = Keypair::from_secret_key(
        &Secp256k1::new(),
        &SecretKey::from_slice(&[9; 32]).expect("key"),
    )
    .x_only_public_key()
    .0;
    assert_eq!(key, vec![SigKey::Schnorr(b)]);
    valid(&psbt, "the pkh leaf's signature");

    let psbt = leaf_spend_with(
        |_, b| pkh_leaf(b),
        |l| {
            vec![
                l.sign(1, TapSighashType::None, None, None),
                l.xonly[1].serialize().to_vec(),
                l.script.to_bytes(),
                l.control.clone(),
            ]
        },
    );
    valid(&psbt, "the NONE signature in the pkh leaf");
    let insp = leaf_cards(psbt, PsbtSighashType::from_u32(1));
    assert!(
        insp.warnings
            .iter()
            .any(|w| w.level == Level::Blocked && matches!(w.detail, Detail::SignedNone { .. })),
        "{:?}",
        insp.warnings
    );
    assert!(insp.refuses());
}

/// The ALL-asking input carries the blocking SignedNone card and refuses.
fn assert_none_refused(psbt: Psbt) {
    let insp = leaf_cards(psbt, PsbtSighashType::from_u32(1));
    assert!(
        insp.warnings
            .iter()
            .any(|w| w.level == Level::Blocked && matches!(w.detail, Detail::SignedNone { .. })),
        "{:?}",
        insp.warnings
    );
    assert!(insp.refuses());
}

/// Real NONE signatures that the code cannot verify stay refused when the
/// input asks ALL: a leaf that hashes the key, a CODESEPARATOR; one over an annex verifies.
#[test]
fn a_real_none_signature_it_cannot_verify_stays_refused() {
    use bitcoin::opcodes::all::{
        OP_CHECKSIG, OP_CODESEPARATOR, OP_DUP, OP_EQUALVERIFY, OP_NOP, OP_RIPEMD160, OP_SHA256,
    };
    let none = TapSighashType::None;
    // DUP SHA256 <sha256(B)> EQUALVERIFY CHECKSIG with [sig_B, B]: the
    // key is unknown to the code, as at base.
    let psbt = leaf_spend_with(
        |_, b| {
            let h = bitcoin::hashes::sha256::Hash::hash(&b.serialize()).to_byte_array();
            bitcoin::script::Builder::new()
                .push_opcode(OP_DUP)
                .push_opcode(OP_SHA256)
                .push_slice(h)
                .push_opcode(OP_EQUALVERIFY)
                .push_opcode(OP_CHECKSIG)
                .into_script()
        },
        |l| {
            vec![
                l.sign(1, none, None, None),
                l.xonly[1].serialize().to_vec(),
                l.script.to_bytes(),
                l.control.clone(),
            ]
        },
    );
    // sha256(B) lies on the curve, so it reads as a candidate key.
    assert_eq!(verdicts(&psbt), vec![Verdict::Invalid]);
    assert_none_refused(psbt);

    // CODESEPARATOR <A> CHECKSIG with [sig_A] signed at position 0.
    let psbt = leaf_spend_with(
        |a, _| {
            bitcoin::script::Builder::new()
                .push_opcode(OP_CODESEPARATOR)
                .push_x_only_key(a)
                .push_opcode(OP_CHECKSIG)
                .into_script()
        },
        |l| {
            vec![
                l.sign(0, none, None, Some(0)),
                l.script.to_bytes(),
                l.control.clone(),
            ]
        },
    );
    assert_eq!(verdicts(&psbt), vec![Verdict::Invalid]);
    assert_none_refused(psbt);

    // <A> CHECKSIG NOP with an annex, which the signature commits to.
    let annex = [0x50, 0x01, 0x02];
    let psbt = leaf_spend_with(
        |a, _| {
            bitcoin::script::Builder::new()
                .push_x_only_key(a)
                .push_opcode(OP_CHECKSIG)
                .push_opcode(OP_NOP)
                .into_script()
        },
        |l| {
            vec![
                l.sign(0, none, Some(&annex), None),
                l.script.to_bytes(),
                l.control.clone(),
                annex.to_vec(),
            ]
        },
    );
    // The signature commits to the annex, so it verifies; the card stays.
    assert_eq!(verdicts(&psbt), vec![Verdict::Valid]);
    assert_none_refused(psbt);

    // DUP RIPEMD160 <ripemd160(B)> EQUALVERIFY CHECKSIG with [sig_B, B]:
    // no key is taken from it, so the signature cannot be checked.
    let psbt = leaf_spend_with(
        |_, b| {
            let h = bitcoin::hashes::ripemd160::Hash::hash(&b.serialize()).to_byte_array();
            bitcoin::script::Builder::new()
                .push_opcode(OP_DUP)
                .push_opcode(OP_RIPEMD160)
                .push_slice(h)
                .push_opcode(OP_EQUALVERIFY)
                .push_opcode(OP_CHECKSIG)
                .into_script()
        },
        |l| {
            vec![
                l.sign(1, none, None, None),
                l.xonly[1].serialize().to_vec(),
                l.script.to_bytes(),
                l.control.clone(),
            ]
        },
    );
    assert_eq!(
        verdicts(&psbt),
        vec![Verdict::Unchecked(Unchecked::UnknownKey)]
    );
    assert_none_refused(psbt);
}

/// A hash of a key in the leaf is no `pkh` of it unless the leaf checks the
/// signature against that key: B's signature is Invalid in a leaf whose
/// only CHECKSIG is against A, and the NONE card refuses it.
#[test]
fn a_hash_of_the_witness_key_outside_the_pkh_shape_makes_no_key() {
    use bitcoin::opcodes::all::{OP_CHECKSIG, OP_DROP, OP_EQUALVERIFY, OP_HASH160, OP_SIZE};
    let hb = |b: &bitcoin::XOnlyPublicKey| {
        bitcoin::hashes::hash160::Hash::hash(&b.serialize()).to_byte_array()
    };
    let size_leaf = |a: &bitcoin::XOnlyPublicKey, b: &bitcoin::XOnlyPublicKey| {
        bitcoin::script::Builder::new()
            .push_opcode(OP_SIZE)
            .push_int(32)
            .push_opcode(OP_EQUALVERIFY)
            .push_opcode(OP_HASH160)
            .push_slice(hb(b))
            .push_opcode(OP_EQUALVERIFY)
            .push_x_only_key(a)
            .push_opcode(OP_CHECKSIG)
            .into_script()
    };
    let drop_leaf = |a: &bitcoin::XOnlyPublicKey, b: &bitcoin::XOnlyPublicKey| {
        bitcoin::script::Builder::new()
            .push_slice(hb(b))
            .push_opcode(OP_DROP)
            .push_x_only_key(a)
            .push_opcode(OP_CHECKSIG)
            .into_script()
    };
    for which in 0..2 {
        let psbt = leaf_spend_with(
            |a, b| {
                if which == 0 {
                    size_leaf(a, b)
                } else {
                    drop_leaf(a, b)
                }
            },
            |l| {
                vec![
                    l.sign(1, TapSighashType::None, None, None),
                    l.xonly[1].serialize().to_vec(),
                    l.script.to_bytes(),
                    l.control.clone(),
                ]
            },
        );
        assert_eq!(verdicts(&psbt), vec![Verdict::Invalid]);
        let insp = leaf_cards(psbt, PsbtSighashType::from_u32(1));
        assert!(
            insp.warnings.iter().any(
                |w| w.level == Level::Blocked && matches!(w.detail, Detail::SignedNone { .. })
            ),
            "{:?}",
            insp.warnings
        );
        assert!(insp.refuses());
    }
}

/// A 65-byte preimage ending 0x03, at an input with no output of the same
/// index, is data: no sighash card, no refusal.
#[test]
fn a_65_byte_preimage_ending_0x03_is_data_where_no_output_matches() {
    let mut preimage = vec![0xab; 64];
    preimage.push(0x03);
    let mut psbt = signed_single(ScriptType::Taproot, Nonce::LowR, Aux::Deterministic, 29_000);
    finalize(&mut psbt).expect("a single-signature input finalizes");
    // SINGLE's sighash cannot be computed with no output at the index.
    psbt.inner_mut().unsigned_tx.output.clear();
    psbt.inner_mut().outputs.clear();
    let psbt = leaf_spend_on(
        psbt,
        |a, _| hashlock_leaf(&preimage)(a),
        |l| {
            vec![
                l.sign(0, TapSighashType::Default, None, None),
                preimage.clone(),
                l.script.to_bytes(),
                l.control.clone(),
            ]
        },
    );
    let insp = leaf_cards(psbt, PsbtSighashType::from_u32(0));
    assert_eq!(sighash_cards(&insp), 0, "{:?}", insp.warnings);
    assert!(
        !insp.warnings.iter().any(|w| w.level == Level::Blocked),
        "{:?}",
        insp.warnings
    );
}

/// `and_v(v:pk(A),pkh(B))`: A's signature is Valid under A, B's under B,
/// and a spoiled one is the only Invalid.
#[test]
fn a_pkh_signature_beside_a_pk_signature_is_checked_on_its_own() {
    use bitcoin::opcodes::all::{OP_CHECKSIGVERIFY, OP_DUP, OP_EQUALVERIFY, OP_HASH160};
    let leaf = |a: &bitcoin::XOnlyPublicKey, b: &bitcoin::XOnlyPublicKey| {
        let h = bitcoin::hashes::hash160::Hash::hash(&b.serialize()).to_byte_array();
        bitcoin::script::Builder::new()
            .push_x_only_key(a)
            .push_opcode(OP_CHECKSIGVERIFY)
            .push_opcode(OP_DUP)
            .push_opcode(OP_HASH160)
            .push_slice(h)
            .push_opcode(OP_EQUALVERIFY)
            .push_opcode(bitcoin::opcodes::all::OP_CHECKSIG)
            .into_script()
    };
    let psbt = leaf_spend_two(leaf, |sig_a, sig_b, b| vec![sig_b, b, sig_a]);
    let found = verdicts(&psbt);
    assert_eq!(found, vec![Verdict::Valid, Verdict::Valid], "{found:?}");

    let psbt = leaf_spend_two(leaf, |sig_a, sig_b, b| vec![sig_b, b, spoiled(sig_a)]);
    let mut found = verdicts(&psbt);
    found.sort_by_key(|v| matches!(v, Verdict::Invalid));
    assert_eq!(found, vec![Verdict::Valid, Verdict::Invalid], "{found:?}");
}

/// A finalized key-path signature of 65 bytes ending 0x04 is no data
/// element: its sighash card stays.
#[test]
fn a_finalized_key_path_signature_with_another_sighash_keeps_its_card() {
    let mut psbt = signed_single(ScriptType::Taproot, Nonce::LowR, Aux::Deterministic, 29_000);
    finalize(&mut psbt).expect("a single-signature input finalizes");
    let mut sig = psbt.inner().inputs[0]
        .final_script_witness
        .as_ref()
        .and_then(|w| w.iter().next().map(|e| e.to_vec()))
        .expect("the key-path signature");
    sig.truncate(64);
    sig.push(0x81);
    psbt.inner_mut().inputs[0].final_script_witness = Some(Witness::from_slice(&[sig]));
    let insp = leaf_cards(psbt, PsbtSighashType::from_u32(1));
    assert!(
        insp.warnings
            .iter()
            .any(|w| w.level == Level::Danger && matches!(w.detail, Detail::SignedOther { .. })),
        "{:?}",
        insp.warnings
    );
}

/// One byte of a signature changed: the key did not make it, and the
/// review says so as a danger.
#[test]
fn one_byte_changed_in_a_signature_is_invalid() {
    let c = Compute::unchecked();
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
    let keys = [KeyRef::from_master(&c, &abandon, 0).expect("accounts")];
    let insp = inspect(&c, &altered, &ctx(&keys, &[]));
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
    let c = Compute::unchecked();
    let abandon = master(ABANDON, NET);
    let keys = [KeyRef::from_master(&c, &abandon, 0).expect("accounts")];
    let mut b = Build::new(ScriptType::NativeSegwit, NET);
    b.recipients.push((foreign(), 70_000));
    b.change = Some(29_000);
    let mut psbt = single_sig(&abandon, &b);

    // A second input of the same address, so that one key signs twice.
    let account = abandon
        .account_xpub(&c, ScriptType::NativeSegwit, 0)
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
    let derived = abandon.derive(&c, &path);
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
            bitcoin::PublicKey::new(derived.to_xpub(&c).public_key),
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

    let insp = inspect(&c, &psbt, &ctx(&keys, &[]));
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
    let c = Compute::unchecked();
    let abandon = master(ABANDON, NET);
    let psbt = signed_single(
        ScriptType::NativeSegwit,
        Nonce::LowR,
        Aux::Deterministic,
        29_000,
    );
    let found = deterministic(&c, &psbt, &[&abandon]);
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
        let found = deterministic(&c, &psbt, &[&abandon]);
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
        let found = deterministic(&c, &psbt, &[&abandon]);
        assert_eq!(found.len(), 1, "{found:?}");
        assert_eq!(found[0].mode, mode, "{found:?}");
    }
}

/// A key this device does not hold has no determinism to report: the
/// check needs the key, and nothing else stands in for it.
#[test]
fn a_signature_of_another_device_is_not_recomputed() {
    let c = Compute::unchecked();
    let psbt = signed_single(
        ScriptType::NativeSegwit,
        Nonce::LowR,
        Aux::Deterministic,
        29_000,
    );
    assert!(deterministic(&c, &psbt, &[&master(ZOO, NET)]).is_empty());
}

/// Two PSBTs compared: the same transaction written twice is the same
/// transaction, an output's amount changed is that output, and a
/// signature added is the signing state.
#[test]
fn two_transactions_are_compared_on_what_they_decode_to() {
    let c = Compute::unchecked();
    let abandon = master(ABANDON, NET);
    let keys = [KeyRef::from_master(&c, &abandon, 0).expect("accounts")];
    let mut b = Build::new(ScriptType::NativeSegwit, NET);
    b.recipients.push((foreign(), 70_000));
    b.change = Some(29_000);
    let unsigned = single_sig(&abandon, &b);

    let again = Psbt::parse_bytes(&c, &unsigned.to_bytes()).expect("a round trip");
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
        &c,
        &mut signed,
        &[&abandon],
        &[abandon.fingerprint()],
        &ctx(&keys, &[]),
        &[],
        Nonce::LowR,
        Aux::Deterministic,
        &mut None,
        &faraday_crypto::Zeroizing::new([0u8; 32]),
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
    let c = Compute::unchecked();
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

    let keys = [KeyRef::from_master(&c, &abandon, 0).expect("accounts")];
    let insp = inspect(&c, &psbt, &ctx(&keys, &[]));
    assert!(
        insp.warnings.iter().any(
            |w| matches!(w.kind, WarningKind::Inscription { input: 0, .. })
                && w.level == faraday_psbt::Level::Info
        ),
        "{:?}",
        insp.warnings
    );
}

/// An input with no previous output states that its signature was not
/// checked, rather than passing it off as either answer.
#[test]
fn a_signature_with_no_previous_output_is_not_checked() {
    let c = Compute::unchecked();
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
    let insp = inspect(&c, &psbt, &ctx(&[], &[]));
    assert!(
        insp.warnings
            .iter()
            .any(|w| matches!(w.kind, WarningKind::SignatureUnchecked { .. })),
        "{:?}",
        insp.warnings
    );
}

/// A single-signature spend over the test key whose input asks for
/// `asked` (no field when `None`), carrying a signature made with
/// rust-bitcoin and secp256k1 under `signed`: an ECDSA one for native
/// SegWit, a BIP-340 key-path one for taproot. The signature is good
/// under the sighash it states.
fn answered_with(
    script_type: ScriptType,
    asked: Option<PsbtSighashType>,
    signed: PsbtSighashType,
) -> Psbt {
    let c = Compute::unchecked();
    let secp = Secp256k1::new();
    let abandon = master(ABANDON, NET);
    let mut b = Build::new(script_type, NET);
    b.recipients.push((foreign(), 70_000));
    b.change = Some(29_000);
    let mut psbt = single_sig(&abandon, &b);
    psbt.inner_mut().inputs[0].sighash_type = asked;
    let tx = psbt.unsigned_tx().clone();
    let prev = psbt.inner().inputs[0]
        .witness_utxo
        .clone()
        .expect("the output it spends");
    let mut cache = SighashCache::new(&tx);
    if script_type == ScriptType::Taproot {
        let path = DerivationPath::from_str("m/86h/1h/0h/0/0").expect("the key's own path");
        let derived = abandon.derive(&c, &path);
        let keypair = Keypair::from_secret_key(&secp, derived.secret_key())
            .tap_tweak(&secp, None)
            .to_keypair();
        let ty = signed.taproot_hash_ty().expect("a taproot sighash");
        let hash = cache
            .taproot_key_spend_signature_hash(0, &Prevouts::All(&[prev]), ty)
            .expect("the sighash");
        let msg = Message::from_digest(hash.to_byte_array());
        psbt.inner_mut().inputs[0].tap_key_sig = Some(taproot::Signature {
            signature: secp.sign_schnorr_no_aux_rand(&msg, &keypair),
            sighash_type: ty,
        });
    } else {
        let path = DerivationPath::from_str("m/84h/1h/0h/0/0").expect("the key's own path");
        let derived = abandon.derive(&c, &path);
        let ty = signed.ecdsa_hash_ty().expect("an ECDSA sighash");
        let hash = cache
            .p2wpkh_signature_hash(0, &prev.script_pubkey, prev.value, ty)
            .expect("the sighash");
        let msg = Message::from_digest(hash.to_byte_array());
        psbt.inner_mut().inputs[0].partial_sigs.insert(
            bitcoin::PublicKey::new(derived.to_xpub(&c).public_key),
            ecdsa::Signature {
                signature: secp.sign_ecdsa(&msg, derived.secret_key()),
                sighash_type: ty,
            },
        );
    }
    assert_eq!(
        verdicts(&psbt),
        vec![Verdict::Valid],
        "the signature is good"
    );
    psbt
}

/// The levels at which the review flags input 0's signature for its
/// sighash.
fn mismatch_levels(psbt: &Psbt) -> Vec<Level> {
    let c = Compute::unchecked();
    let abandon = master(ABANDON, NET);
    let keys = [KeyRef::from_master(&c, &abandon, 0).expect("accounts")];
    inspect(&c, psbt, &ctx(&keys, &[]))
        .warnings
        .into_iter()
        .filter(|w| matches!(w.kind, WarningKind::SighashMismatch { input: 0, .. }))
        .map(|w| w.level)
        .collect()
}

/// A signer asked for nothing (or for ALL) that answers with a good
/// SIGHASH_NONE|ANYONECANPAY signature has signed no output: anyone can
/// redirect the funds, and the review blocks it. One under SINGLE where
/// ALL was asked is a danger. One stricter than asked (ALL where
/// ALL|ANYONECANPAY was asked) is a caution. One under the sighash asked
/// for passes.
#[test]
fn a_signature_under_a_sighash_not_asked_for_is_flagged_ecdsa() {
    let t = ScriptType::NativeSegwit;
    let ty = |t: EcdsaSighashType| PsbtSighashType::from(t);
    let none_acp = ty(EcdsaSighashType::NonePlusAnyoneCanPay);
    assert_eq!(
        mismatch_levels(&answered_with(t, None, none_acp)),
        vec![Level::Blocked]
    );
    assert_eq!(
        mismatch_levels(&answered_with(t, Some(ty(EcdsaSighashType::All)), none_acp)),
        vec![Level::Blocked]
    );
    assert_eq!(
        mismatch_levels(&answered_with(
            t,
            Some(ty(EcdsaSighashType::All)),
            ty(EcdsaSighashType::Single)
        )),
        vec![Level::Danger]
    );
    assert_eq!(
        mismatch_levels(&answered_with(
            t,
            Some(ty(EcdsaSighashType::AllPlusAnyoneCanPay)),
            ty(EcdsaSighashType::All)
        )),
        vec![Level::Caution]
    );
    assert_eq!(
        mismatch_levels(&answered_with(
            t,
            Some(ty(EcdsaSighashType::Single)),
            ty(EcdsaSighashType::All)
        )),
        vec![Level::Caution]
    );
    assert!(mismatch_levels(&answered_with(t, None, ty(EcdsaSighashType::All))).is_empty());
    assert!(
        mismatch_levels(&answered_with(
            t,
            Some(ty(EcdsaSighashType::Single)),
            ty(EcdsaSighashType::Single)
        ))
        .is_empty()
    );
}

/// The same for a taproot key-path signature; DEFAULT and ALL commit to
/// the same, so either answers an input that asks for nothing or ALL.
#[test]
fn a_signature_under_a_sighash_not_asked_for_is_flagged_taproot() {
    let t = ScriptType::Taproot;
    let ty = |t: TapSighashType| PsbtSighashType::from(t);
    let none_acp = ty(TapSighashType::NonePlusAnyoneCanPay);
    assert_eq!(
        mismatch_levels(&answered_with(t, None, none_acp)),
        vec![Level::Blocked]
    );
    assert_eq!(
        mismatch_levels(&answered_with(
            t,
            None,
            ty(TapSighashType::AllPlusAnyoneCanPay)
        )),
        vec![Level::Danger]
    );
    assert_eq!(
        mismatch_levels(&answered_with(
            t,
            Some(ty(TapSighashType::AllPlusAnyoneCanPay)),
            ty(TapSighashType::Default)
        )),
        vec![Level::Caution]
    );
    assert!(mismatch_levels(&answered_with(t, None, ty(TapSighashType::Default))).is_empty());
    assert!(mismatch_levels(&answered_with(t, None, ty(TapSighashType::All))).is_empty());
    assert!(
        mismatch_levels(&answered_with(
            t,
            Some(ty(TapSighashType::All)),
            ty(TapSighashType::Default)
        ))
        .is_empty()
    );
}

// ---- a finalized script-path witness is checked against the output ----

fn mismatch_cards(psbt: Psbt) -> usize {
    leaf_cards(psbt, PsbtSighashType::from_u32(0))
        .warnings
        .iter()
        .filter(|w| {
            w.level == Level::Danger
                && matches!(w.kind, WarningKind::WitnessMismatch { .. })
                && matches!(w.detail, Detail::WitnessMismatch { input: 0 })
        })
        .count()
}

fn pk_leaf(a: &bitcoin::XOnlyPublicKey, _: &bitcoin::XOnlyPublicKey) -> ScriptBuf {
    bitcoin::script::Builder::new()
        .push_x_only_key(a)
        .push_opcode(bitcoin::opcodes::all::OP_CHECKSIG)
        .into_script()
}

fn pk_spend(seed: u8, depth: u8) -> Psbt {
    let psbt = signed_single(ScriptType::Taproot, Nonce::LowR, Aux::Deterministic, 29_000);
    let mut psbt = psbt;
    finalize(&mut psbt).expect("a single-signature input finalizes");
    leaf_spend_tree(psbt, seed, depth, pk_leaf, |l| {
        vec![
            l.sign(0, TapSighashType::Default, None, None),
            l.script.to_bytes(),
            l.control.clone(),
        ]
    })
}

/// The witness with its last element (the control block) changed.
fn with_control(mut psbt: Psbt, change: impl Fn(&mut Vec<u8>)) -> Psbt {
    let mut elements: Vec<Vec<u8>> = psbt.inner().inputs[0]
        .final_script_witness
        .as_ref()
        .expect("a witness")
        .iter()
        .map(<[u8]>::to_vec)
        .collect();
    change(elements.last_mut().expect("a control block"));
    psbt.inner_mut().inputs[0].final_script_witness = Some(Witness::from_slice(&elements));
    psbt
}

#[test]
fn a_true_script_path_spend_has_no_witness_card() {
    for depth in [0, 1, 2] {
        let psbt = pk_spend(7, depth);
        valid(&psbt, "the true spend");
        assert_eq!(mismatch_cards(psbt), 0, "depth {depth}");
    }
    // The odd-parity output key.
    let odd = (1..=40u8)
        .map(|seed| pk_spend(seed, 1))
        .find(|p| {
            p.inner().inputs[0]
                .final_script_witness
                .as_ref()
                .and_then(Witness::last)
                .is_some_and(|c| c[0] & 1 == 1)
        })
        .expect("a seed with an odd output key");
    valid(&odd, "the odd-parity spend");
    assert_eq!(mismatch_cards(odd), 0);
}

#[test]
fn a_made_up_leaf_with_its_keys_real_signature_is_invalid() {
    // The output holds a leaf for key A; the witness spends a leaf of key
    // B's, signed by B, with a control block for it.
    let psbt = leaf_spend_with(pk_leaf, |l| {
        let made_up = bitcoin::script::Builder::new()
            .push_x_only_key(&l.xonly[1])
            .push_opcode(bitcoin::opcodes::all::OP_CHECKSIG)
            .into_script();
        assert_ne!(made_up, l.script);
        let theirs = LeafSpend {
            script: made_up.clone(),
            control: Vec::new(),
            xonly: l.xonly,
            keypairs: l.keypairs,
            tx: l.tx.clone(),
            prev: l.prev.clone(),
        };
        let mut control = vec![0xc0];
        control.extend_from_slice(&l.xonly[0].serialize());
        vec![
            theirs.sign(1, TapSighashType::Default, None, None),
            made_up.to_bytes(),
            control,
        ]
    });
    assert_eq!(verdicts(&psbt), vec![Verdict::Invalid]);
    assert_eq!(mismatch_cards(psbt), 1);
}

#[test]
fn an_output_key_not_on_the_curve_gets_the_card() {
    // BIP-340's not-on-the-curve x-coordinate as the output key.
    let mut psbt = pk_spend(7, 0);
    psbt.inner_mut().inputs[0]
        .witness_utxo
        .as_mut()
        .expect("the output")
        .script_pubkey =
        ScriptBuf::from_hex("5120eefdea4cdb677750a420fee807eacf21eb9898ae79b9768766e4faa04a2d4a34")
            .expect("hex");
    assert!(witness_mismatch(psbt.inner(), 0));
    assert!(!verdicts(&psbt).contains(&Verdict::Valid));
    assert_eq!(mismatch_cards(psbt), 1);
}

/// `pk_spend(7, 0)` with its taproot output wrapped in p2sh: the redeem
/// script is the p2tr script, in the `scriptSig`; the output hashes
/// `hashed` instead of it when given.
fn wrapped(hashed: Option<[u8; 20]>) -> Psbt {
    let mut psbt = pk_spend(7, 0);
    let input = &mut psbt.inner_mut().inputs[0];
    let utxo = input.witness_utxo.as_mut().expect("the output");
    let redeem = utxo.script_pubkey.clone();
    let hash = hashed.unwrap_or_else(|| redeem.script_hash().to_byte_array());
    utxo.script_pubkey = ScriptBuf::new_p2sh(&bitcoin::ScriptHash::from_byte_array(hash));
    let push = bitcoin::script::PushBytesBuf::try_from(redeem.to_bytes()).expect("a push");
    input.final_script_sig = Some(
        bitcoin::script::Builder::new()
            .push_slice(push)
            .into_script(),
    );
    psbt
}

#[test]
fn a_p2sh_whose_redeem_script_is_not_the_hashed_one_gets_the_card() {
    // The online side's scriptSig names the output key; the output hashes
    // something else.
    let psbt = wrapped(Some([0; 20]));
    assert!(witness_mismatch(psbt.inner(), 0));
    assert!(!verdicts(&psbt).contains(&Verdict::Valid));
    assert_eq!(mismatch_cards(psbt), 1);
}

#[test]
fn a_p2sh_with_the_right_hash_and_a_witness_that_does_not_commit_is_refused_all_the_same() {
    // Consensus would ignore the control block here; the card still stands.
    let psbt = with_control(wrapped(None), |c| c[3] ^= 1);
    assert!(witness_mismatch(psbt.inner(), 0));
    assert_eq!(mismatch_cards(psbt), 1);
    assert!(!witness_mismatch(wrapped(None).inner(), 0));
}

/// The witness of a finalized spend with `annex` after its control block.
fn with_annex(psbt: Psbt, annex: &[u8], spoil: bool) -> Psbt {
    let mut psbt = psbt;
    let mut elements: Vec<Vec<u8>> = psbt.inner().inputs[0]
        .final_script_witness
        .as_ref()
        .expect("a witness")
        .iter()
        .map(<[u8]>::to_vec)
        .collect();
    if spoil {
        elements.last_mut().expect("control")[2] ^= 1;
    }
    elements.push(annex.to_vec());
    psbt.inner_mut().inputs[0].final_script_witness = Some(Witness::from_slice(&elements));
    psbt
}

#[test]
fn an_annex_is_set_aside_before_the_witness_is_checked() {
    let annex = [0x50, 1, 2, 3];
    // The signature here is not over the annex (a true one over it is in
    // `signature_origins`), so the shape alone is what this checks.
    let shaped_spend = with_annex(pk_spend(7, 1), &annex, false);
    assert!(!witness_shape_mismatch(shaped_spend.inner(), 0));
    let spoiled = with_annex(pk_spend(7, 1), &annex, true);
    assert!(witness_mismatch(spoiled.inner(), 0));
    assert_eq!(mismatch_cards(spoiled), 1);
    // A key-path spend with an annex.
    let mut key_path = signed_single(ScriptType::Taproot, Nonce::LowR, Aux::Deterministic, 29_000);
    finalize(&mut key_path).expect("finalizes");
    let key_path = with_annex(key_path, &annex, false);
    assert_eq!(
        key_path.inner().inputs[0]
            .final_script_witness
            .as_ref()
            .map(Witness::len),
        Some(2)
    );
    assert!(!witness_shape_mismatch(key_path.inner(), 0));
}

#[test]
fn a_witness_with_nothing_to_check_it_against_is_no_mismatch() {
    let mut no_prev = pk_spend(7, 0);
    let input = &mut no_prev.inner_mut().inputs[0];
    input.witness_utxo = None;
    input.non_witness_utxo = None;
    assert!(!witness_mismatch(no_prev.inner(), 0));
    let mut key_path = signed_single(ScriptType::Taproot, Nonce::LowR, Aux::Deterministic, 29_000);
    finalize(&mut key_path).expect("finalizes");
    assert!(!witness_mismatch(key_path.inner(), 0));
    let mut segwit = signed_single(
        ScriptType::NativeSegwit,
        Nonce::LowR,
        Aux::Deterministic,
        29_000,
    );
    finalize(&mut segwit).expect("finalizes");
    assert!(!witness_mismatch(segwit.inner(), 0));
}

#[test]
fn a_spoiled_control_block_gets_the_card_and_invalid_rows() {
    type Spoil = fn(&mut Vec<u8>);
    let spoils: [(&str, u8, Spoil); 5] = [
        ("internal key", 1, |c| c[4] ^= 1),
        ("merkle node", 1, |c| c[40] ^= 1),
        ("parity", 1, |c| c[0] ^= 1),
        ("leaf version", 1, |c| c[0] ^= 0x02),
        ("length", 1, |c| {
            c.pop();
        }),
    ];
    for (what, depth, spoil) in spoils {
        let psbt = with_control(pk_spend(7, depth), spoil);
        assert_eq!(verdicts(&psbt), vec![Verdict::Invalid], "{what}");
        assert_eq!(mismatch_cards(psbt), 1, "{what}");
    }
}

#[test]
fn a_real_none_signature_hidden_before_a_junk_leaf_gets_the_card() {
    let psbt = leaf_spend_with(pk_leaf, |l| {
        vec![
            l.sign(0, TapSighashType::None, None, None),
            vec![0x51],
            l.control.clone(),
        ]
    });
    assert_eq!(verdicts(&psbt), vec![Verdict::Invalid]);
    assert_eq!(mismatch_cards(psbt), 1);
}

#[test]
fn a_witness_with_no_signature_is_checked_against_the_output_too() {
    let preimage = vec![0xcd; 32];
    let committing = leaf_spend(hashlock_leaf(&preimage), |_| vec![preimage.clone()]);
    assert!(verdicts(&committing).is_empty());
    assert!(!witness_mismatch(committing.inner(), 0));
    assert_eq!(mismatch_cards(committing.clone()), 0);
    let spoiled = with_control(committing, |c| c[2] ^= 1);
    assert!(verdicts(&spoiled).is_empty());
    assert!(witness_mismatch(spoiled.inner(), 0));
    assert_eq!(mismatch_cards(spoiled), 1);
}

#[test]
fn a_proven_preimage_in_a_witness_that_does_not_commit_stays_unchecked() {
    let preimage = vec![0xcd; 64];
    let psbt = leaf_spend(hashlock_leaf(&preimage), |sig| vec![sig, preimage.clone()]);
    let psbt = with_control(psbt, |c| c[2] ^= 1);
    let found = verdicts(&psbt);
    assert_eq!(found.len(), 2, "{found:?}");
    assert!(found.contains(&Verdict::Invalid), "{found:?}");
    assert!(
        found.contains(&Verdict::Unchecked(Unchecked::UnsupportedScript)),
        "{found:?}"
    );
    assert_eq!(mismatch_cards(psbt), 1);
}

/// BIP-341's wallet test vectors (`scriptPubKey`): the script-path spends
/// of its tests 1 (odd parity) and 3 (a leaf one level down) commit.
#[test]
fn bip_341_script_path_vectors_commit() {
    let cases = [
        (
            "5120147c9c57132f6e7ecddba9800bb0c4449251c92a1e60371ee77557b6620f3ea3",
            "20d85a959b0290bf19bb89ed43c916be835475d013da4b362117393e25a48229b8ac",
            "c1187791b6f712a8ea41c8ecdd0ee77fab3e85263b37e1ec18a3651926b3a6cf27",
        ),
        (
            "5120712447206d7a5238acc7ff53fbe94a3b64539ad291c7cdbc490b7577e4b17df5",
            "20387671353e273264c495656e27e39ba899ea8fee3bb69fb2a680e22093447d48ac",
            "c0ee4fe085983462a184015d1f782d6a5f8b9c2b60130aff050ce221ecf3786592f224a923cd0021ab202ab139cc56802ddb92dcfc172b9212261a539df79a112a",
        ),
    ];
    for (spk, script, control) in cases {
        let bytes = |h: &str| ScriptBuf::from_hex(h).expect("hex").into_bytes();
        let mut psbt = signed_single(ScriptType::Taproot, Nonce::LowR, Aux::Deterministic, 29_000);
        finalize(&mut psbt).expect("a single-signature input finalizes");
        let input = &mut psbt.inner_mut().inputs[0];
        input
            .witness_utxo
            .as_mut()
            .expect("the output it spends")
            .script_pubkey = ScriptBuf::from_hex(spk).expect("hex");
        input.final_script_witness = Some(Witness::from_slice(&[
            vec![1; 32],
            bytes(script),
            bytes(control),
        ]));
        assert!(!witness_shape_mismatch(psbt.inner(), 0), "{spk}");
        let spoiled = with_control(psbt, |c| c[1] ^= 1);
        assert!(witness_shape_mismatch(spoiled.inner(), 0), "{spk}");
    }
}

/// BIP-341's script-path vector whose leaf version is not 0xc0.
#[test]
fn a_true_spend_of_a_leaf_with_another_version_commits() {
    let bytes = |h: &str| ScriptBuf::from_hex(h).expect("hex").into_bytes();
    let mut psbt = signed_single(ScriptType::Taproot, Nonce::LowR, Aux::Deterministic, 29_000);
    finalize(&mut psbt).expect("a single-signature input finalizes");
    let input = &mut psbt.inner_mut().inputs[0];
    input
        .witness_utxo
        .as_mut()
        .expect("the output it spends")
        .script_pubkey =
        ScriptBuf::from_hex("5120712447206d7a5238acc7ff53fbe94a3b64539ad291c7cdbc490b7577e4b17df5")
            .expect("hex");
    input.final_script_witness = Some(Witness::from_slice(&[
        vec![1; 32],
        bytes("06424950333431"),
        bytes(
            "faee4fe085983462a184015d1f782d6a5f8b9c2b60130aff050ce221ecf37865928ad69ec7cf41c2a4001fd1f738bf1e505ce2277acdcaa63fe4765192497f47a7",
        ),
    ]));
    assert!(!witness_mismatch(psbt.inner(), 0));
}

/// A one-input PSBT whose input is final as given, spending `spk`.
fn final_input(spk: ScriptBuf, script_sig: ScriptBuf, witness: &[Vec<u8>]) -> bitcoin::Psbt {
    let tx = bitcoin::Transaction {
        version: bitcoin::transaction::Version::TWO,
        lock_time: bitcoin::absolute::LockTime::ZERO,
        input: vec![TxIn {
            previous_output: OutPoint::new(bitcoin::Txid::from_byte_array([3; 32]), 0),
            script_sig: ScriptBuf::new(),
            sequence: Sequence::MAX,
            witness: Witness::new(),
        }],
        output: vec![TxOut {
            value: Amount::from_sat(9_000),
            script_pubkey: spk.clone(),
        }],
    };
    let mut psbt = bitcoin::Psbt::from_unsigned_tx(tx).unwrap();
    psbt.inputs[0].witness_utxo = Some(TxOut {
        value: Amount::from_sat(10_000),
        script_pubkey: spk,
    });
    psbt.inputs[0].final_script_sig = Some(script_sig);
    if !witness.is_empty() {
        psbt.inputs[0].final_script_witness = Some(Witness::from_slice(witness));
    }
    psbt
}

#[test]
fn a_true_final_spend_of_a_non_taproot_output_is_never_a_witness_mismatch() {
    use bitcoin::opcodes::all::{OP_PUSHNUM_1, OP_PUSHNUM_2};
    use bitcoin::script::{Builder, PushBytesBuf};
    let push = |bytes: &[u8]| {
        Builder::new()
            .push_slice(PushBytesBuf::try_from(bytes.to_vec()).unwrap())
            .into_script()
    };
    let (sig, sig2, key) = (some_sig(), sig_bytes(6, [9; 32]), key_bytes(5, true));
    let pkh = bitcoin::PubkeyHash::hash(&key);
    let p2wpkh =
        ScriptBuf::new_p2wpkh(&bitcoin::WPubkeyHash::from_byte_array(*pkh.as_byte_array()));
    let multisig = Builder::new()
        .push_opcode(OP_PUSHNUM_2)
        .push_slice(PushBytesBuf::try_from(key.clone()).unwrap())
        .push_slice(PushBytesBuf::try_from(key_bytes(6, true)).unwrap())
        .push_opcode(OP_PUSHNUM_2)
        .push_opcode(bitcoin::opcodes::all::OP_CHECKMULTISIG)
        .into_script();
    let p2wsh = ScriptBuf::new_p2wsh(&multisig.wscript_hash());
    let cases = [
        (
            "p2sh-p2wpkh",
            ScriptBuf::new_p2sh(&p2wpkh.script_hash()),
            push(p2wpkh.as_bytes()),
            vec![sig.clone(), key.clone()],
        ),
        (
            "p2sh-p2wsh",
            ScriptBuf::new_p2sh(&p2wsh.script_hash()),
            push(p2wsh.as_bytes()),
            vec![vec![], sig.clone(), sig2.clone(), multisig.to_bytes()],
        ),
        (
            "p2sh",
            ScriptBuf::new_p2sh(
                &Builder::new()
                    .push_opcode(OP_PUSHNUM_1)
                    .into_script()
                    .script_hash(),
            ),
            push(&[OP_PUSHNUM_1.to_u8()]),
            vec![],
        ),
        (
            "p2wsh",
            p2wsh,
            ScriptBuf::new(),
            vec![vec![], sig.clone(), sig2, multisig.to_bytes()],
        ),
    ];
    for (what, spk, script_sig, witness) in cases {
        let psbt = final_input(spk, script_sig, &witness);
        assert!(
            !faraday_psbt::verify::witness_shape_mismatch(&psbt, 0),
            "{what}"
        );
        assert!(!faraday_psbt::verify::final_unread(&psbt, 0), "{what}");
    }
}

/// A finalized single-signature input of `script_type` that states a
/// previous transaction, with its `witness_utxo` stating `lie` sat where
/// the previous transaction says 100,000.
fn finalized_with_witness_utxo(script_type: ScriptType, lie: Option<u64>) -> Psbt {
    let mut psbt = signed_single(script_type, Nonce::LowR, Aux::Deterministic, 29_000);
    finalize(&mut psbt).expect("finalizes");
    let input = &mut psbt.inner_mut().inputs[0];
    let previous = input
        .non_witness_utxo
        .clone()
        .expect("the builder states the previous transaction");
    let vout = psbt.inner().unsigned_tx.input[0].previous_output.vout as usize;
    let stated = previous.output[vout].clone();
    let input = &mut psbt.inner_mut().inputs[0];
    input.witness_utxo = Some(TxOut {
        value: Amount::from_sat(lie.unwrap_or(stated.value.to_sat())),
        script_pubkey: stated.script_pubkey,
    });
    psbt
}

/// A `witness_utxo` that differs from a previous transaction of the right
/// txid is read as the previous transaction: the final input is neither
/// sound nor finished, and a true signature is still Valid, since a node
/// checks it over the previous transaction's amount. Where the two agree
/// the input reads as it always did, with `non_witness_utxo` only, with
/// `witness_utxo` only, and with both.
#[test]
fn a_witness_utxo_that_differs_from_its_previous_transaction_is_read_as_the_transaction() {
    // Taproot states no previous transaction here: its signature commits
    // to the outpoint, so one cannot be added after signing (the core
    // tests build one).
    for script_type in ScriptType::ALL {
        if script_type == ScriptType::Taproot {
            continue;
        }
        let what = format!("{script_type:?}");
        let agreeing = finalized_with_witness_utxo(script_type, None);
        let mut only_previous = agreeing.clone();
        only_previous.inner_mut().inputs[0].witness_utxo = None;
        let mut only_witness = agreeing.clone();
        only_witness.inner_mut().inputs[0].non_witness_utxo = None;
        for (psbt, how) in [
            (&agreeing, "both agreeing"),
            (&only_previous, "non_witness_utxo only"),
            (&only_witness, "witness_utxo only"),
        ] {
            assert!(!final_unread(psbt.inner(), 0), "{what}, {how}");
            assert!(!utxos_disagree(psbt.inner(), 0), "{what}, {how}");
            valid(psbt, &format!("{what}, {how}"));
        }
        let lying = finalized_with_witness_utxo(script_type, Some(99_999));
        assert!(final_unread(lying.inner(), 0), "{what}");
        assert!(utxos_disagree(lying.inner(), 0), "{what}");
        valid(&lying, &format!("{what} under a lying witness_utxo"));
        let spent = faraday_psbt::verify::spent_outputs(lying.inner());
        assert_eq!(
            spent[0].as_ref().map(|o| o.value),
            Some(Amount::from_sat(100_000)),
            "{what}: the previous transaction's amount"
        );
    }
}

/// A previous transaction of another txid beside a `witness_utxo`, final or
/// not, states no output: its signature has nothing to be checked against
/// (not "unsupported script"), the fee is not worked out from the
/// `witness_utxo`'s amount, and `finalize` refuses it with the reason
/// found, as it does a disagreement; none of them is complete.
#[test]
fn a_false_previous_transaction_reads_as_no_output_and_is_never_finalized() {
    let wrong_txid = |psbt: &mut Psbt| {
        let mut other = psbt.inner().inputs[0].non_witness_utxo.clone().unwrap();
        other.lock_time = bitcoin::absolute::LockTime::from_consensus(1);
        psbt.inner_mut().inputs[0].non_witness_utxo = Some(other);
    };
    for script_type in [ScriptType::NativeSegwit, ScriptType::Legacy] {
        let what = format!("{script_type:?}");
        let agreeing = finalized_with_witness_utxo(script_type, None);
        // Final, wrong txid beside the `witness_utxo`.
        let mut beside = agreeing.clone();
        wrong_txid(&mut beside);
        // Final, wrong txid alone.
        let mut alone = beside.clone();
        alone.inner_mut().inputs[0].witness_utxo = None;
        // Final, the right transaction against a lying `witness_utxo`.
        let disagree = finalized_with_witness_utxo(script_type, Some(99_999));
        assert!(is_complete(&agreeing), "{what}: honest stays complete");
        for (psbt, how, reason) in [
            (
                &beside,
                "wrong txid beside witness_utxo",
                faraday_psbt::Reason::PrevTxNotSpent,
            ),
            (
                &alone,
                "wrong txid alone",
                faraday_psbt::Reason::PrevTxNotSpent,
            ),
            (
                &disagree,
                "a disagreement",
                faraday_psbt::Reason::UtxosDisagree,
            ),
        ] {
            assert!(!is_complete(psbt), "{what}, {how}");
            match finalize(&mut psbt.clone()) {
                Err(faraday_psbt::Error::Finalize {
                    input: 0,
                    reason: r,
                }) => {
                    assert_eq!(r, reason, "{what}, {how}");
                }
                other => panic!("{what}, {how}: {other:?}"),
            }
        }
        // A final input that reads no output carries no row. One not yet
        // final reads "no previous output" for its signature, never
        // "unsupported script", whatever the `witness_utxo` beside says.
        assert!(verdicts(&beside).is_empty(), "{what}");
        let mut partial = signed_single(script_type, Nonce::LowR, Aux::Deterministic, 29_000);
        let stated = partial.inner().inputs[0].non_witness_utxo.clone().unwrap();
        let vout = partial.inner().unsigned_tx.input[0].previous_output.vout as usize;
        partial.inner_mut().inputs[0].witness_utxo = Some(stated.output[vout].clone());
        wrong_txid(&mut partial);
        assert_eq!(
            verdicts(&partial),
            vec![Verdict::Unchecked(Unchecked::NoPrevout)],
            "{what}"
        );
        let c = Compute::unchecked();
        let seen = inspect(&c, &partial, &ctx(&[], &[]));
        assert_eq!(seen.total_in, Amount::ZERO, "{what}: no amount is added");
        assert!(seen.fee_rate_sat_vb.is_none(), "{what}: no fee is shown");
        assert!(
            seen.warnings
                .iter()
                .any(|w| w.kind == WarningKind::MissingUtxo),
            "{what}"
        );
    }
}

/// The committed fixtures (the PSBT vectors, the samples and the interop
/// vectors) carry no false statement of what an input spends, as read and
/// after `finalize`, but those named here: `warn-utxo-mismatch.psbt`
/// states a previous transaction of another txid on purpose.
#[test]
fn no_committed_fixture_is_refused_for_agreeing_outputs() {
    const LYING: [&str; 1] = ["warn-utxo-mismatch.psbt"];
    // A final input with one witness item too many: the signer check's own case.
    const MISMATCHED: [&str; 1] = ["signer-final-extra.psbt"];
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let (mut walked, mut lying) = (0, 0);
    for dir in [
        "tools/vectors/psbt",
        "faraday/faraday-samples/data",
        "faraday/faraday-core/tests/vectors/interop/bluewallet",
    ] {
        for entry in std::fs::read_dir(root.join(dir)).expect("the vector directory") {
            let path = entry.expect("an entry").path();
            if path.extension().is_none_or(|e| e != "psbt") {
                continue;
            }
            let bytes = std::fs::read(&path).expect("a fixture");
            let text = String::from_utf8_lossy(&bytes).trim().to_string();
            let c = Compute::unchecked();
            let Ok(psbt) = Psbt::parse_bytes(&c, &bytes).or_else(|_| Psbt::parse_base64(&c, &text))
            else {
                continue;
            };
            let name = path.file_name().unwrap().to_string_lossy().into_owned();
            let mut finalized = psbt.clone();
            let done = finalize(&mut finalized);
            if LYING.contains(&name.as_str()) {
                assert!(done.is_err() && !is_complete(&psbt), "{name}");
            }
            for (what, p) in [("as read", &psbt), ("finalized", &finalized)] {
                for i in 0..p.inner().inputs.len() {
                    let false_ = states_false(p.inner(), i);
                    if LYING.contains(&name.as_str()) {
                        lying += usize::from(what == "as read" && false_);
                    } else {
                        assert!(!false_, "{} {what}, input {i}", path.display());
                    }
                    assert!(
                        LYING.contains(&name.as_str()) || !final_unread(p.inner(), i),
                        "{} {what}, input {i}",
                        path.display()
                    );
                    assert_eq!(
                        witness_mismatch(p.inner(), i),
                        MISMATCHED.contains(&name.as_str()),
                        "{} {what}, input {i}",
                        path.display()
                    );
                }
            }
            walked += 1;
        }
    }
    assert!(walked > 10, "{walked} fixtures walked");

    assert!(lying > 0, "the fixture meant to state a false output does");
}

/// `finalize` says why: a previous transaction of another txid is not
/// spent, whatever else it lacks.
#[test]
fn finalize_names_a_previous_transaction_of_another_txid_as_it_is() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tools/vectors/psbt/warn-utxo-mismatch.psbt");
    let text = std::fs::read_to_string(path).expect("the fixture");
    let mut psbt = Psbt::parse_base64(&Compute::unchecked(), text.trim()).expect("parses");
    match finalize(&mut psbt) {
        Err(faraday_psbt::Error::Finalize { reason, .. }) => {
            assert_eq!(reason, faraday_psbt::Reason::PrevTxNotSpent);
        }
        other => panic!("{other:?}"),
    }
}

/// A row of the shapes table: what, output, scriptSig, witness, mismatch.
type Row = (String, ScriptBuf, ScriptBuf, Vec<Vec<u8>>, bool);

/// A finalized non-taproot input is read against the output it spends by
/// hashes and shapes: each true shape reads no mismatch, each of the
/// others is one, and the review says so once.
#[test]
#[expect(clippy::too_many_lines, reason = "a table of cases")]
fn a_finalized_non_taproot_input_is_compared_with_its_output() {
    use bitcoin::hashes::{hash160, sha256};
    use bitcoin::script::{Builder, PushBytesBuf};
    use bitcoin::{PubkeyHash, ScriptHash, WPubkeyHash, WScriptHash};
    let push = |bytes: &[u8]| PushBytesBuf::try_from(bytes.to_vec()).unwrap();
    let (program, multisig) = (witness_program, multisig_script);
    let pair = |a: &[u8], b: &[u8]| {
        Builder::new()
            .push_slice(push(a))
            .push_slice(push(b))
            .into_script()
    };
    let one = |a: &[u8]| Builder::new().push_slice(push(a)).into_script();
    let (sig, key, other) = (some_sig(), key_bytes(5, true), key_bytes(6, true));
    let sig2 = sig_bytes(6, [9; 32]);
    let uncompressed = key_bytes(7, false);
    let three = |a: &[u8], b: &[u8], c: &[u8]| {
        Builder::new()
            .push_slice(push(a))
            .push_slice(push(b))
            .push_slice(push(c))
            .into_script()
    };
    let junk_key = vec![9u8; 7];
    let h160 = |b: &[u8]| *hash160::Hash::hash(b).as_byte_array();
    let p2pkh = |k: &[u8]| ScriptBuf::new_p2pkh(&PubkeyHash::from_byte_array(h160(k)));
    let p2wpkh = |k: &[u8]| ScriptBuf::new_p2wpkh(&WPubkeyHash::from_byte_array(h160(k)));
    let p2wsh = |s: &[u8]| {
        ScriptBuf::new_p2wsh(&WScriptHash::from_byte_array(
            *sha256::Hash::hash(s).as_byte_array(),
        ))
    };
    let p2sh = |s: &[u8]| ScriptBuf::new_p2sh(&ScriptHash::from_byte_array(h160(s)));
    let wpkh = p2wpkh(&key);
    let wsh_script = vec![0x51u8];
    let wsh = p2wsh(&wsh_script);
    let none = ScriptBuf::new;
    let p2pk = |k: &[u8]| {
        Builder::new()
            .push_slice(push(k))
            .push_opcode(bitcoin::opcodes::all::OP_CHECKSIG)
            .into_script()
    };
    let w = |e: &[&[u8]]| e.iter().map(|x| x.to_vec()).collect::<Vec<_>>();
    // (what, output, scriptSig, witness, mismatch)
    let cases = vec![
        ("p2pkh", p2pkh(&key), pair(&sig, &key), w(&[]), false),
        (
            "p2pkh, uncompressed key",
            p2pkh(&uncompressed),
            pair(&sig, &uncompressed),
            w(&[]),
            false,
        ),
        (
            "p2pkh of another key",
            p2pkh(&other),
            pair(&sig, &key),
            w(&[]),
            true,
        ),
        (
            "p2pkh, signature alone",
            p2pkh(&key),
            one(&sig),
            w(&[]),
            true,
        ),
        ("p2pkh, the key alone", p2pkh(&key), one(&key), w(&[]), true),
        (
            "p2pkh, three pushes",
            p2pkh(&key),
            three(&sig, &key, &key),
            w(&[]),
            true,
        ),
        (
            "p2pkh, a stray push first",
            p2pkh(&key),
            three(&[1], &sig, &key),
            w(&[]),
            true,
        ),
        (
            "p2pkh, a non-push opcode between",
            p2pkh(&key),
            Builder::new()
                .push_slice(push(&sig))
                .push_opcode(bitcoin::opcodes::all::OP_NOP)
                .push_slice(push(&key))
                .into_script(),
            w(&[]),
            true,
        ),
        (
            "p2pkh, a key that is not a key under its own hash",
            p2pkh(&junk_key),
            pair(&sig, &junk_key),
            w(&[]),
            true,
        ),
        (
            "p2pkh, an empty signature",
            p2pkh(&key),
            pair(&[], &key),
            w(&[]),
            true,
        ),
        (
            "p2pkh, a signature that does not parse",
            p2pkh(&key),
            pair(&[0x30; 71], &key),
            w(&[]),
            true,
        ),
        (
            "p2wpkh, a key that is not a key under its own hash",
            p2wpkh(&junk_key),
            none(),
            w(&[&sig, &junk_key]),
            true,
        ),
        (
            "p2wpkh, an empty signature",
            wpkh.clone(),
            none(),
            w(&[&[], &key]),
            true,
        ),
        (
            "p2wpkh, a signature that does not parse",
            wpkh.clone(),
            none(),
            w(&[&[0x30; 71], &key]),
            true,
        ),
        (
            "p2sh-p2wpkh, a key that is not a key under its own hash",
            p2sh(p2wpkh(&junk_key).as_bytes()),
            one(p2wpkh(&junk_key).as_bytes()),
            w(&[&sig, &junk_key]),
            true,
        ),
        (
            "p2pkh with a witness",
            p2pkh(&key),
            pair(&sig, &key),
            w(&[&sig]),
            true,
        ),
        ("p2wpkh", wpkh.clone(), none(), w(&[&sig, &key]), false),
        (
            "p2wpkh of another key",
            p2wpkh(&other),
            none(),
            w(&[&sig, &key]),
            true,
        ),
        (
            "p2wpkh, one element",
            wpkh.clone(),
            none(),
            w(&[&sig]),
            true,
        ),
        (
            "p2wpkh, three elements",
            wpkh.clone(),
            none(),
            w(&[&sig, &key, &key]),
            true,
        ),
        (
            "p2wpkh with a scriptSig",
            wpkh.clone(),
            one(&key),
            w(&[&sig, &key]),
            true,
        ),
        (
            "p2sh-p2wpkh",
            p2sh(wpkh.as_bytes()),
            one(wpkh.as_bytes()),
            w(&[&sig, &key]),
            false,
        ),
        (
            "p2sh-p2wpkh, other key's witness",
            p2sh(wpkh.as_bytes()),
            one(wpkh.as_bytes()),
            w(&[&sig, &other]),
            true,
        ),
        (
            "p2sh-p2wpkh, a second push",
            p2sh(wpkh.as_bytes()),
            pair(&[], wpkh.as_bytes()),
            w(&[&sig, &key]),
            true,
        ),
        ("p2wsh", wsh.clone(), none(), w(&[&sig, &wsh_script]), false),
        (
            "p2wsh of an empty script",
            p2wsh(&[]),
            none(),
            w(&[&sig, &[]]),
            false,
        ),
        (
            "p2wsh of all zeros",
            ScriptBuf::new_p2wsh(&WScriptHash::from_byte_array([0; 32])),
            none(),
            w(&[&sig, &wsh_script]),
            true,
        ),
        ("p2wsh, empty witness", wsh.clone(), none(), w(&[]), true),
        (
            "p2sh-p2wsh",
            p2sh(wsh.as_bytes()),
            one(wsh.as_bytes()),
            w(&[&sig, &wsh_script]),
            false,
        ),
        (
            "p2sh-p2wsh, another script",
            p2sh(wsh.as_bytes()),
            one(wsh.as_bytes()),
            w(&[&sig, &[0x52]]),
            true,
        ),
        (
            "plain p2sh",
            p2sh(&[0x51]),
            Builder::new()
                .push_int(0)
                .push_slice(push(&sig))
                .push_slice(push(&[0x51]))
                .into_script(),
            w(&[]),
            false,
        ),
        (
            "plain p2sh of all zeros",
            ScriptBuf::new_p2sh(&ScriptHash::from_byte_array([0; 20])),
            Builder::new()
                .push_int(0)
                .push_slice(push(&sig))
                .push_slice(push(&[0x51]))
                .into_script(),
            w(&[]),
            true,
        ),
        (
            "plain p2sh with a witness",
            p2sh(&[0x51]),
            one(&[0x51]),
            w(&[&sig]),
            true,
        ),
        (
            "plain p2sh, no scriptSig",
            p2sh(&[0x51]),
            none(),
            w(&[]),
            true,
        ),
        (
            "plain p2sh, a non-push opcode before the redeem script",
            p2sh(&[0x51]),
            Builder::new()
                .push_opcode(bitcoin::opcodes::all::OP_NOP)
                .push_slice(push(&[0x51]))
                .into_script(),
            w(&[]),
            true,
        ),
        (
            "plain p2sh, OP_1 OP_DUP under the hash of nothing",
            p2sh(&[]),
            Builder::new()
                .push_int(1)
                .push_opcode(bitcoin::opcodes::all::OP_DUP)
                .into_script(),
            w(&[]),
            true,
        ),
        (
            "plain p2sh, OP_1 OP_1 OP_1NEGATE under the hash of 0x81",
            p2sh(&[0x81]),
            Builder::new()
                .push_int(1)
                .push_int(1)
                .push_int(-1)
                .into_script(),
            w(&[]),
            false,
        ),
        (
            "plain p2sh, OP_1 OP_RESERVED under the hash of nothing",
            p2sh(&[]),
            Builder::new()
                .push_int(1)
                .push_opcode(bitcoin::opcodes::all::OP_RESERVED)
                .into_script(),
            w(&[]),
            true,
        ),
        (
            "plain p2sh, OP_1 as the redeem script itself",
            p2sh(&[1]),
            Builder::new().push_int(1).into_script(),
            w(&[]),
            false,
        ),
        (
            "plain p2sh, OP_1 OP_0 under the hash of nothing",
            p2sh(&[]),
            Builder::new().push_int(1).push_int(0).into_script(),
            w(&[]),
            false,
        ),
        ("bare p2pk", p2pk(&key), one(&sig), w(&[]), false),
        (
            "bare p2pk, a push that is no signature",
            p2pk(&key),
            one(&[7; 5]),
            w(&[]),
            true,
        ),
        (
            "bare p2pk, an empty scriptSig",
            p2pk(&key),
            none(),
            w(&[]),
            true,
        ),
        (
            "bare multisig 1-of-1",
            multisig(1, std::slice::from_ref(&key)),
            pair(&[], &sig),
            w(&[]),
            false,
        ),
        (
            "bare multisig 1-of-1, no signature",
            multisig(1, std::slice::from_ref(&key)),
            one(&[]),
            w(&[]),
            true,
        ),
        (
            "bare multisig 2-of-2, one signature",
            multisig(2, &[key.clone(), other.clone()]),
            pair(&[], &sig),
            w(&[]),
            true,
        ),
        (
            "bare multisig 2-of-2",
            multisig(2, &[key.clone(), other.clone()]),
            three(&[], &sig, &sig2),
            w(&[]),
            false,
        ),
        (
            "p2sh 2-of-2",
            p2sh(multisig(2, &[key.clone(), other.clone()]).as_bytes()),
            Builder::new()
                .push_int(0)
                .push_slice(push(&sig))
                .push_slice(push(&sig2))
                .push_slice(push(multisig(2, &[key.clone(), other.clone()]).as_bytes()))
                .into_script(),
            w(&[]),
            false,
        ),
        (
            "p2sh 2-of-2, one signature",
            p2sh(multisig(2, &[key.clone(), other.clone()]).as_bytes()),
            three(
                &[],
                &sig,
                multisig(2, &[key.clone(), other.clone()]).as_bytes(),
            ),
            w(&[]),
            true,
        ),
        (
            "p2sh 2-of-2, the script alone",
            p2sh(multisig(2, &[key.clone(), other.clone()]).as_bytes()),
            one(multisig(2, &[key.clone(), other.clone()]).as_bytes()),
            w(&[]),
            true,
        ),
        (
            "p2wsh 2-of-2",
            p2wsh(multisig(2, &[key.clone(), other.clone()]).as_bytes()),
            none(),
            w(&[
                &[],
                &sig,
                &sig2,
                multisig(2, &[key.clone(), other.clone()]).as_bytes(),
            ]),
            false,
        ),
        (
            "p2wsh 2-of-2, one signature",
            p2wsh(multisig(2, &[key.clone(), other.clone()]).as_bytes()),
            none(),
            w(&[
                &[],
                &sig,
                multisig(2, &[key.clone(), other.clone()]).as_bytes(),
            ]),
            true,
        ),
        (
            "p2wsh 2-of-2, the script alone",
            p2wsh(multisig(2, &[key.clone(), other.clone()]).as_bytes()),
            none(),
            w(&[multisig(2, &[key.clone(), other.clone()]).as_bytes()]),
            true,
        ),
        (
            "p2wsh 2-of-3, three signatures where two are asked",
            p2wsh(multisig(2, &[key.clone(), other.clone(), uncompressed.clone()]).as_bytes()),
            none(),
            w(&[
                &[],
                &sig,
                &sig,
                &sig,
                multisig(2, &[key.clone(), other.clone(), uncompressed.clone()]).as_bytes(),
            ]),
            true,
        ),
        (
            "p2wsh 2-of-2, an empty signature in place of one",
            p2wsh(multisig(2, &[key.clone(), other.clone()]).as_bytes()),
            none(),
            w(&[
                &[],
                &sig,
                &[],
                multisig(2, &[key.clone(), other.clone()]).as_bytes(),
            ]),
            true,
        ),
        (
            "p2wsh 2-of-2, one signature that does not parse",
            p2wsh(multisig(2, &[key.clone(), other.clone()]).as_bytes()),
            none(),
            w(&[
                &[],
                &sig,
                &[0x31; 71],
                multisig(2, &[key.clone(), other.clone()]).as_bytes(),
            ]),
            true,
        ),
        (
            "p2sh of a version 1 program, two pushes",
            p2sh(program(1, 32).as_bytes()),
            pair(&[], program(1, 32).as_bytes()),
            w(&[&sig]),
            true,
        ),
        (
            "p2sh of a version 2 program, two pushes",
            p2sh(program(2, 32).as_bytes()),
            pair(&[], program(2, 32).as_bytes()),
            w(&[&sig]),
            true,
        ),
        (
            "p2sh of a version 1, 32-byte program, two witness elements",
            p2sh(program(1, 32).as_bytes()),
            one(program(1, 32).as_bytes()),
            w(&[&sig, &sig]),
            true,
        ),
        (
            "a version 0 program of 25 bytes",
            program(0, 25),
            none(),
            w(&[&sig]),
            true,
        ),
        (
            "a version 0 program of 25 bytes, no witness",
            program(0, 25),
            none(),
            w(&[]),
            true,
        ),
        (
            "a version 0 program of 2 bytes in p2sh",
            p2sh(program(0, 2).as_bytes()),
            one(program(0, 2).as_bytes()),
            w(&[&sig]),
            true,
        ),
        (
            "a version 0 program of 40 bytes in p2sh",
            p2sh(program(0, 40).as_bytes()),
            one(program(0, 40).as_bytes()),
            w(&[&sig]),
            true,
        ),
    ];
    // Every other version, and version 1 of another length than 32, is a
    // program no rule here reads: spendable by anyone until a rule
    // defines it.
    let mut cases: Vec<Row> = cases
        .into_iter()
        .map(|(what, spk, sig, wit, bad)| (what.to_string(), spk, sig, wit, bad))
        .collect();
    for (version, len) in (1..=16u8)
        .map(|v| (v, 32))
        .chain([(1, 2), (1, 20), (1, 33), (1, 40)])
    {
        let name = format!("version {version} program of {len} bytes");
        let inner = program(version, len);
        if version == 1 && len == 32 {
            continue;
        }
        let native = inner.clone();
        let wrapped = p2sh(inner.as_bytes());
        for (extra, witness) in [("", w(&[&sig])), (", no witness", w(&[]))] {
            cases.push((
                format!("{name}{extra}"),
                native.clone(),
                none(),
                witness.clone(),
                false,
            ));
            cases.push((
                format!("{name} in p2sh{extra}"),
                wrapped.clone(),
                one(inner.as_bytes()),
                witness,
                false,
            ));
        }
        cases.push((
            format!("{name} with a scriptSig"),
            native,
            one(&[1]),
            w(&[&sig]),
            true,
        ));
    }
    for (what, spk, script_sig, witness, mismatch) in cases {
        let psbt = final_input(spk, script_sig, &witness);
        assert_eq!(witness_shape_mismatch(&psbt, 0), mismatch, "{what}");
        assert_eq!(
            mismatch_cards(Psbt::from(psbt)),
            usize::from(mismatch),
            "{what}"
        );
    }
}

// ---- real keys and signatures for the final-input shapes ----

fn secret(seed: u8) -> SecretKey {
    SecretKey::from_slice(&[seed; 32]).unwrap()
}

/// The public key of `secret(seed)`, serialized.
fn key_bytes(seed: u8, compressed: bool) -> Vec<u8> {
    let key = PublicKey::from_secret_key(&Secp256k1::new(), &secret(seed));
    if compressed {
        key.serialize().to_vec()
    } else {
        key.serialize_uncompressed().to_vec()
    }
}

/// A DER signature of `secret(seed)` over `digest`, with SIGHASH_ALL.
fn sig_bytes(seed: u8, digest: [u8; 32]) -> Vec<u8> {
    let sig = Secp256k1::new().sign_ecdsa(&Message::from_digest(digest), &secret(seed));
    let mut bytes = sig.serialize_der().to_vec();
    bytes.push(1);
    bytes
}

/// A signature that parses, over nothing in particular.
fn some_sig() -> Vec<u8> {
    sig_bytes(5, [9; 32])
}

/// `OP_m <key>… OP_n OP_CHECKMULTISIG`.
fn multisig_script(m: i64, keys: &[Vec<u8>]) -> ScriptBuf {
    use bitcoin::script::{Builder, PushBytesBuf};
    let mut b = Builder::new().push_int(m);
    for key in keys {
        b = b.push_slice(PushBytesBuf::try_from(key.clone()).unwrap());
    }
    b.push_int(i64::try_from(keys.len()).unwrap())
        .push_opcode(bitcoin::opcodes::all::OP_CHECKMULTISIG)
        .into_script()
}

/// A witness program of `version` with `len` program bytes.
fn witness_program(version: u8, len: usize) -> ScriptBuf {
    let mut bytes = vec![if version == 0 { 0 } else { 0x50 + version }, len as u8];
    bytes.extend(std::iter::repeat_n(7u8, len));
    ScriptBuf::from(bytes)
}

/// The digest the signer of input 0 of `psbt` signs for `script`: the
/// script code of a p2wsh `script` (or of the p2wpkh `script`) when
/// `segwit`, else a legacy signature hash.
fn digest_of(psbt: &bitcoin::Psbt, script: &ScriptBuf, segwit: bool) -> [u8; 32] {
    let amount = Amount::from_sat(10_000);
    let mut cache = SighashCache::new(&psbt.unsigned_tx);
    if !segwit {
        cache
            .legacy_signature_hash(0, script, 1)
            .unwrap()
            .to_byte_array()
    } else if script.is_p2wpkh() {
        cache
            .p2wpkh_signature_hash(0, script, amount, EcdsaSighashType::All)
            .unwrap()
            .to_byte_array()
    } else {
        cache
            .p2wsh_signature_hash(0, script, amount, EcdsaSighashType::All)
            .unwrap()
            .to_byte_array()
    }
}

fn push_script(parts: &[&[u8]]) -> ScriptBuf {
    use bitcoin::script::{Builder, PushBytesBuf};
    let mut b = Builder::new();
    for part in parts {
        b = b.push_slice(PushBytesBuf::try_from(part.to_vec()).unwrap());
    }
    b.into_script()
}

fn p2sh_of(script: &ScriptBuf) -> ScriptBuf {
    ScriptBuf::new_p2sh(&script.script_hash())
}

fn p2wsh_of(script: &ScriptBuf) -> ScriptBuf {
    ScriptBuf::new_p2wsh(&script.wscript_hash())
}

/// A p2sh input of the multisig `script`, final, signed by each seed of
/// `signers` over `script` (the signature a node checks).
fn final_p2sh_multisig(script: &ScriptBuf, signers: &[u8]) -> bitcoin::Psbt {
    let mut psbt = final_input(p2sh_of(script), ScriptBuf::new(), &[]);
    let digest = digest_of(&psbt, script, false);
    let mut parts: Vec<Vec<u8>> = vec![vec![]];
    parts.extend(signers.iter().map(|s| sig_bytes(*s, digest)));
    parts.push(script.to_bytes());
    let refs: Vec<&[u8]> = parts.iter().map(Vec::as_slice).collect();
    psbt.inputs[0].final_script_sig = Some(push_script(&refs));
    psbt
}

/// A p2wsh input of the multisig `script`, final and signed likewise.
fn final_p2wsh_multisig(script: &ScriptBuf, signers: &[u8]) -> bitcoin::Psbt {
    let mut psbt = final_input(p2wsh_of(script), ScriptBuf::new(), &[]);
    let digest = digest_of(&psbt, script, true);
    let mut parts: Vec<Vec<u8>> = vec![vec![]];
    parts.extend(signers.iter().map(|s| sig_bytes(*s, digest)));
    parts.push(script.to_bytes());
    psbt.inputs[0].final_script_witness = Some(Witness::from_slice(&parts));
    psbt
}

fn rows_of(psbt: &bitcoin::Psbt) -> Vec<Verdict> {
    verdicts(&Psbt::from(psbt.clone()))
}

/// A multisig script that holds an uncompressed key is read: the
/// signature of the uncompressed key reads Valid, and `Spend::finish`
/// takes the same input.
#[test]
fn an_uncompressed_key_in_a_p2sh_multisig_checks_its_signature() {
    let (a, b) = (key_bytes(5, false), key_bytes(6, true));
    for (what, script) in [
        ("1-of-2", multisig_script(1, &[a.clone(), b])),
        ("1-of-1", multisig_script(1, std::slice::from_ref(&a))),
    ] {
        let psbt = final_p2sh_multisig(&script, &[5]);
        assert!(!witness_mismatch(&psbt, 0), "{what}");
        assert_eq!(rows_of(&psbt), vec![Verdict::Valid], "{what}");
    }
}

/// A `redeem_script` or `witness_script` field beside a final input is
/// not read: the scripts are the ones in the `scriptSig` and the witness,
/// so a signature made over another script reads Invalid, and the true
/// input reads Valid with the right field, a stale one or none.
#[test]
#[expect(clippy::too_many_lines, reason = "a table of cases")]
fn a_stale_script_field_beside_a_final_input_is_not_read() {
    let (a, b) = (key_bytes(5, true), key_bytes(6, true));
    let real = multisig_script(1, std::slice::from_ref(&a));
    let other = multisig_script(1, std::slice::from_ref(&b));
    type Set = fn(&mut bitcoin::psbt::Input, &ScriptBuf);
    let redeem: Set = |i, s| i.redeem_script = Some(s.clone());
    let witness: Set = |i, s| i.witness_script = Some(s.clone());
    // (what, how the input is made over a script, which field is stale)
    let wsh = |code: &ScriptBuf| {
        let mut p = final_input(p2wsh_of(&real), ScriptBuf::new(), &[]);
        let digest = digest_of(&p, code, true);
        p.inputs[0].final_script_witness = Some(Witness::from_slice(&[
            vec![],
            sig_bytes(5, digest),
            real.to_bytes(),
        ]));
        p
    };
    let sh = |code: &ScriptBuf| {
        let mut p = final_input(p2sh_of(&real), ScriptBuf::new(), &[]);
        let digest = digest_of(&p, code, false);
        p.inputs[0].final_script_sig =
            Some(push_script(&[&[], &sig_bytes(5, digest), &real.to_bytes()]));
        p
    };
    let sh_wsh = |code: &ScriptBuf| {
        let inner = p2wsh_of(&real);
        let mut p = final_input(p2sh_of(&inner), push_script(&[inner.as_bytes()]), &[]);
        let digest = digest_of(&p, code, true);
        p.inputs[0].final_script_witness = Some(Witness::from_slice(&[
            vec![],
            sig_bytes(5, digest),
            real.to_bytes(),
        ]));
        p
    };
    let sh_wpkh = |code_key: &[u8]| {
        let inner = ScriptBuf::new_p2wpkh(
            &bitcoin::PublicKey::from_slice(&a)
                .unwrap()
                .wpubkey_hash()
                .unwrap(),
        );
        let code = ScriptBuf::new_p2wpkh(
            &bitcoin::PublicKey::from_slice(code_key)
                .unwrap()
                .wpubkey_hash()
                .unwrap(),
        );
        let mut p = final_input(p2sh_of(&inner), push_script(&[inner.as_bytes()]), &[]);
        let digest = digest_of(&p, &code, true);
        p.inputs[0].final_script_witness =
            Some(Witness::from_slice(&[sig_bytes(5, digest), a.clone()]));
        (p, code)
    };
    let kinds: Vec<(
        &str,
        bitcoin::Psbt,
        bitcoin::Psbt,
        Set,
        &ScriptBuf,
        &ScriptBuf,
    )> = vec![
        ("p2wsh", wsh(&real), wsh(&other), witness, &real, &other),
        ("plain p2sh", sh(&real), sh(&other), redeem, &real, &other),
        (
            "p2sh-p2wsh",
            sh_wsh(&real),
            sh_wsh(&other),
            witness,
            &real,
            &other,
        ),
    ];
    for (what, true_psbt, false_psbt, set, right, stale) in kinds {
        assert_eq!(rows_of(&true_psbt), vec![Verdict::Valid], "{what}");
        for field in [None, Some(right)] {
            let mut p = true_psbt.clone();
            if let Some(f) = field {
                set(&mut p.inputs[0], f);
            }
            assert_eq!(rows_of(&p), vec![Verdict::Valid], "{what}: true input");
            assert!(!witness_mismatch(&p, 0), "{what}");
        }
        let mut p = false_psbt;
        assert_eq!(rows_of(&p), vec![Verdict::Invalid], "{what}: control");
        set(&mut p.inputs[0], stale);
        assert!(!witness_shape_mismatch(&p, 0), "{what}: the hash is right");
        assert_eq!(
            rows_of(&p),
            vec![Verdict::Invalid],
            "{what}: signed over the stale field's script"
        );
    }
    // p2sh-p2wpkh: a stale redeem_script of another key's program.
    let (true_psbt, _) = sh_wpkh(&a);
    let (mut false_psbt, stale) = sh_wpkh(&b);
    assert_eq!(rows_of(&true_psbt), vec![Verdict::Valid]);
    assert_eq!(rows_of(&false_psbt), vec![Verdict::Invalid], "control");
    false_psbt.inputs[0].redeem_script = Some(stale);
    assert_eq!(
        rows_of(&false_psbt),
        vec![Verdict::Invalid],
        "p2sh-p2wpkh: signed over the stale redeem_script's program"
    );
    let mut with_right = true_psbt;
    with_right.inputs[0].redeem_script = with_right.inputs[0]
        .final_script_sig
        .as_ref()
        .map(|s| ScriptBuf::from(s.as_bytes()[1..].to_vec()));
    assert_eq!(rows_of(&with_right), vec![Verdict::Valid]);
}

/// A multisig input with fewer signatures than its script asks for is no
/// spend a node takes: the input is a mismatch, whether or not the
/// signature it holds is true. Two of three in key order are true.
#[test]
fn a_final_multisig_with_fewer_signatures_than_it_asks_for_is_a_mismatch() {
    let keys = |seeds: &[u8]| {
        seeds
            .iter()
            .map(|s| key_bytes(*s, true))
            .collect::<Vec<_>>()
    };
    let two = multisig_script(2, &keys(&[5, 6]));
    let three = multisig_script(2, &keys(&[5, 6, 7]));
    for (what, make) in [
        (
            "p2sh",
            final_p2sh_multisig as fn(&ScriptBuf, &[u8]) -> bitcoin::Psbt,
        ),
        ("p2wsh", final_p2wsh_multisig),
    ] {
        assert!(witness_mismatch(&make(&two, &[5]), 0), "{what}: one of two");
        assert_eq!(
            rows_of(&make(&two, &[5])),
            vec![Verdict::Valid],
            "{what}: the signature itself is true"
        );
        assert!(
            witness_mismatch(&make(&two, &[]), 0),
            "{what}: the script alone"
        );
        assert!(
            !witness_mismatch(&make(&two, &[5, 6]), 0),
            "{what}: two of two"
        );
        assert!(
            witness_mismatch(&make(&three, &[7]), 0),
            "{what}: one of three"
        );
        // Signatures stand in the script's key order, each key once, and
        // exactly as many as the script asks for.
        for (order, sound) in [
            (vec![5, 6], true),
            (vec![5, 7], true),
            (vec![6, 7], true),
            (vec![6, 5], false),
            (vec![7, 5], false),
            (vec![5, 5], false),
            (vec![5, 6, 7], false),
        ] {
            let psbt = make(&three, &order);
            assert_eq!(
                witness_mismatch(&psbt, 0) || final_invalid(&psbt, 0),
                !sound,
                "{what}: {order:?}"
            );
        }
    }
}

/// A taproot output's final witness is one signature of 64 or 65 bytes
/// (with or without an annex) or a script path; a missing or empty
/// witness, or one element of another size, is no spend.
#[test]
fn a_final_taproot_witness_is_a_signature_or_a_script_path() {
    let secp = Secp256k1::new();
    let (xonly, _) = Keypair::from_secret_key(&secp, &secret(5)).x_only_public_key();
    let spk = ScriptBuf::new_p2tr(&secp, xonly, None);
    let annex = {
        let mut a = vec![0x50];
        a.extend_from_slice(&[1; 5]);
        a
    };
    let w = |e: &[&[u8]]| e.iter().map(|x| x.to_vec()).collect::<Vec<_>>();
    let (s64, s65) = (vec![1u8; 64], vec![1u8; 65]);
    let cases = vec![
        ("a 64-byte signature", w(&[&s64]), false),
        ("a 65-byte signature", w(&[&s65]), false),
        (
            "a 64-byte signature and an annex",
            w(&[&s64, &annex]),
            false,
        ),
        (
            "a 65-byte signature and an annex",
            w(&[&s65, &annex]),
            false,
        ),
        ("63 bytes", w(&[&[1; 63]]), true),
        ("66 bytes", w(&[&[1; 66]]), true),
        ("32 bytes", w(&[&[1; 32]]), true),
        ("an empty element", w(&[&[]]), true),
        ("an annex alone", w(&[&annex]), true),
        ("63 bytes and an annex", w(&[&[1; 63], &annex]), true),
    ];
    for (what, witness, mismatch) in cases {
        let psbt = final_input(spk.clone(), ScriptBuf::new(), &witness);
        assert_eq!(witness_mismatch(&psbt, 0), mismatch, "{what}");
        assert_eq!(
            mismatch_cards(Psbt::from(psbt)),
            usize::from(mismatch),
            "{what}"
        );
    }
    let mut empty_witness = final_input(spk.clone(), ScriptBuf::new(), &[]);
    empty_witness.inputs[0].final_script_witness = Some(Witness::new());
    assert!(witness_mismatch(&empty_witness, 0), "an empty witness");
    let empty_sig = final_input(spk, ScriptBuf::new(), &[]);
    assert!(
        witness_mismatch(&empty_sig, 0),
        "an empty scriptSig and no witness"
    );
}

/// A multisig of `script` spent under `wrapper` ("p2sh", "p2wsh",
/// "p2sh-p2wsh" or "bare"), with the stack `make` gives from the digest
/// the signers sign: the dummy first, then the signatures.
fn multisig_spend(
    wrapper: &str,
    script: &ScriptBuf,
    make: &dyn Fn([u8; 32]) -> Vec<Vec<u8>>,
) -> bitcoin::Psbt {
    spend_at(wrapper, script, Sequence::MAX, make)
}

/// [`multisig_spend`] of any script, the input's sequence being `sequence`.
fn spend_at(
    wrapper: &str,
    script: &ScriptBuf,
    sequence: Sequence,
    make: &dyn Fn([u8; 32]) -> Vec<Vec<u8>>,
) -> bitcoin::Psbt {
    spend_in(wrapper, script, &|tx| tx.input[0].sequence = sequence, make)
}

/// [`spend_at`] with any change to the transaction (version, lock time,
/// sequence) made before the signers sign.
fn spend_in(
    wrapper: &str,
    script: &ScriptBuf,
    tweak: &dyn Fn(&mut bitcoin::Transaction),
    make: &dyn Fn([u8; 32]) -> Vec<Vec<u8>>,
) -> bitcoin::Psbt {
    let wsh = p2wsh_of(script);
    let spk = match wrapper {
        "p2sh" => p2sh_of(script),
        "p2wsh" => wsh.clone(),
        "p2sh-p2wsh" => p2sh_of(&wsh),
        _ => script.clone(),
    };
    let mut psbt = final_input(spk, ScriptBuf::new(), &[]);
    tweak(&mut psbt.unsigned_tx);
    let segwit = wrapper.ends_with("wsh");
    let mut parts = make(digest_of(&psbt, script, segwit));
    if wrapper == "bare" {
        let refs: Vec<&[u8]> = parts.iter().map(Vec::as_slice).collect();
        psbt.inputs[0].final_script_sig = Some(push_script(&refs));
        return psbt;
    }
    parts.push(script.to_bytes());
    let refs: Vec<&[u8]> = parts.iter().map(Vec::as_slice).collect();
    if segwit {
        psbt.inputs[0].final_script_witness = Some(Witness::from_slice(&parts));
        if wrapper == "p2sh-p2wsh" {
            psbt.inputs[0].final_script_sig = Some(push_script(&[wsh.as_bytes()]));
        }
    } else {
        psbt.inputs[0].final_script_sig = Some(push_script(&refs));
    }
    psbt
}

const WRAPPERS: [&str; 4] = ["p2sh", "p2wsh", "p2sh-p2wsh", "bare"];

/// A multisig stack padded with junk, a filler or a repeat in place of
/// its signatures is no spend, and an input that is read as a spend is
/// one signature of each key in the script's order.
#[test]
fn a_multisig_stack_that_is_not_the_signatures_it_asks_for_is_a_mismatch() {
    let two = multisig_script(2, &[key_bytes(5, true), key_bytes(6, true)]);
    let one = multisig_script(1, &[key_bytes(5, true)]);
    let junk = vec![0x11; 5];
    let sig = |seed: u8, d: [u8; 32]| sig_bytes(seed, d);
    for wrapper in WRAPPERS {
        let spend = |script: &ScriptBuf, f: &dyn Fn([u8; 32]) -> Vec<Vec<u8>>| {
            let psbt = multisig_spend(wrapper, script, f);
            (witness_mismatch(&psbt, 0), final_invalid(&psbt, 0))
        };
        let (a, b) = (|d| sig(5, d), |d| sig(6, d));
        assert_eq!(
            spend(&two, &|d| vec![vec![], a(d), b(d)]),
            (false, false),
            "{wrapper}: true"
        );
        assert_eq!(
            spend(&one, &|d| vec![vec![], a(d)]),
            (false, false),
            "{wrapper}: 1-of-1"
        );
        for (what, items) in [
            (
                "junk after the dummy",
                vec![vec![], junk.clone(), a([1; 32])],
            ),
            (
                "junk between",
                vec![vec![], a([1; 32]), junk.clone(), b([1; 32])],
            ),
            (
                "a non-empty dummy",
                vec![junk.clone(), a([1; 32]), b([1; 32])],
            ),
            ("all junk", vec![vec![], junk.clone(), junk.clone()]),
        ] {
            let psbt = multisig_spend(wrapper, &two, &|_| items.clone());
            assert!(witness_mismatch(&psbt, 0), "{wrapper}: {what}");
        }
        let (junk_one, _) = spend(&one, &|_| vec![vec![], junk.clone()]);
        assert!(junk_one, "{wrapper}: 1-of-1 with junk");
        // A sighash byte no node reads is refused, as p2pkh's is.
        let (odd, _) = spend(&two, &|d| {
            let mut s = a(d);
            *s.last_mut().unwrap() = 0x00;
            vec![vec![], s, b(d)]
        });
        assert!(odd, "{wrapper}: a sighash byte of 0");
        // The same signature twice, or the keys out of order.
        for (what, order) in [("twice", [5u8, 5]), ("out of order", [6, 5])] {
            let psbt = multisig_spend(wrapper, &two, &|d| {
                vec![vec![], sig(order[0], d), sig(order[1], d)]
            });
            assert!(
                witness_mismatch(&psbt, 0) || final_invalid(&psbt, 0),
                "{wrapper}: {what}"
            );
            assert!(
                rows_of(&psbt).contains(&Verdict::Invalid) || witness_mismatch(&psbt, 0),
                "{wrapper}: {what} reads red"
            );
        }
    }
}

/// A finalized signature that does not verify makes the input refuse to
/// finish: p2wpkh by another key, and a multisig with another key's.
#[test]
fn a_final_input_with_an_invalid_signature_reads_invalid() {
    use bitcoin::hashes::{Hash, hash160};
    let key = key_bytes(5, true);
    let wpkh = ScriptBuf::new_p2wpkh(&bitcoin::WPubkeyHash::from_byte_array(
        *hash160::Hash::hash(&key).as_byte_array(),
    ));
    let build = |seed: u8| {
        let mut psbt = final_input(wpkh.clone(), ScriptBuf::new(), &[]);
        let digest = digest_of(&psbt, &wpkh, true);
        psbt.inputs[0].final_script_witness =
            Some(Witness::from_slice(&[sig_bytes(seed, digest), key.clone()]));
        psbt
    };
    assert!(!final_invalid(&build(5), 0));
    assert!(final_invalid(&build(7), 0));
    let two = multisig_script(2, &[key_bytes(5, true), key_bytes(6, true)]);
    for wrapper in WRAPPERS {
        let psbt = multisig_spend(wrapper, &two, &|d| {
            vec![vec![], sig_bytes(7, d), sig_bytes(6, d)]
        });
        assert!(
            final_invalid(&psbt, 0),
            "{wrapper}: another key's signature"
        );
    }
    // A script holding OP_CODESEPARATOR is signed over a message not
    // worked out here: its true signature is not refused.
    let sep = bitcoin::script::Builder::new()
        .push_opcode(bitcoin::opcodes::all::OP_CODESEPARATOR)
        .push_slice(bitcoin::script::PushBytesBuf::try_from(key.clone()).unwrap())
        .push_opcode(bitcoin::opcodes::all::OP_CHECKSIG)
        .into_script();
    let mut psbt = final_input(p2wsh_of(&sep), ScriptBuf::new(), &[]);
    // Consensus signs the script after the separator.
    let after = ScriptBuf::from(sep.as_bytes()[1..].to_vec());
    let digest = digest_of(&psbt, &after, true);
    psbt.inputs[0].final_script_witness =
        Some(Witness::from_slice(&[sig_bytes(5, digest), sep.to_bytes()]));
    assert!(!final_invalid(&psbt, 0));
}

/// A bare p2pk and a bare multisig have their signatures read.
#[test]
fn the_signatures_of_a_bare_output_are_read() {
    let key = key_bytes(5, true);
    let p2pk = bitcoin::script::Builder::new()
        .push_slice(bitcoin::script::PushBytesBuf::try_from(key.clone()).unwrap())
        .push_opcode(bitcoin::opcodes::all::OP_CHECKSIG)
        .into_script();
    for (seed, row) in [(5u8, Verdict::Valid), (7, Verdict::Invalid)] {
        let mut psbt = final_input(p2pk.clone(), ScriptBuf::new(), &[]);
        let digest = digest_of(&psbt, &p2pk, false);
        psbt.inputs[0].final_script_sig = Some(push_script(&[&sig_bytes(seed, digest)]));
        assert_eq!(rows_of(&psbt), vec![row], "p2pk signed by seed {seed}");
        assert_eq!(
            final_invalid(&psbt, 0),
            row == Verdict::Invalid,
            "p2pk {seed}"
        );
    }
    let two = multisig_script(2, &[key_bytes(5, true), key_bytes(6, true)]);
    let psbt = multisig_spend("bare", &two, &|d| {
        vec![vec![], sig_bytes(5, d), sig_bytes(6, d)]
    });
    assert_eq!(rows_of(&psbt), vec![Verdict::Valid; 2]);
    let psbt = multisig_spend("bare", &two, &|d| {
        vec![vec![], sig_bytes(7, d), sig_bytes(6, d)]
    });
    assert_eq!(rows_of(&psbt), vec![Verdict::Invalid, Verdict::Valid]);
}

/// A multisig of 17 to 20 keys is read: one signature of two is short,
/// and every one of them in order is true.
#[test]
fn a_multisig_of_more_than_sixteen_keys_is_counted() {
    let keys: Vec<Vec<u8>> = (1..=20).map(|s| key_bytes(s, true)).collect();
    for (m, n, signers, mismatch) in [
        (2, 17, vec![3u8], true),
        (2, 17, vec![3, 17], false),
        (17, 20, (1..=17).collect(), false),
        (17, 20, (1..=16).collect(), true),
    ] {
        let script = multisig_script(m, &keys[..n]);
        for wrapper in ["p2wsh", "p2sh-p2wsh"] {
            let psbt = multisig_spend(wrapper, &script, &|d| {
                let mut items = vec![vec![]];
                items.extend(signers.iter().map(|s| sig_bytes(*s, d)));
                items
            });
            assert_eq!(witness_mismatch(&psbt, 0), mismatch, "{wrapper} {m}-of-{n}");
        }
    }
}

// ---- a final input's script is judged by the interpreter ----

/// Whether a final input is refused: a mismatch or a final signature that
/// is wrong, and whether any of its rows reads Invalid.
fn refused(psbt: &bitcoin::Psbt) -> bool {
    witness_mismatch(psbt, 0) || final_invalid(psbt, 0)
}

fn check_all(
    what: &str,
    wrappers: &[&str],
    script: &ScriptBuf,
    make: &dyn Fn([u8; 32]) -> Vec<Vec<u8>>,
    refuse: bool,
) {
    for wrapper in wrappers {
        let psbt = multisig_spend(wrapper, script, make);
        assert_eq!(refused(&psbt), refuse, "{wrapper}: {what}");
        if !refuse {
            assert!(
                !rows_of(&psbt).contains(&Verdict::Invalid),
                "{wrapper}: {what}: a row reads Invalid"
            );
        }
    }
}

fn script_of(parts: &[&[u8]], ops: &[bitcoin::opcodes::Opcode]) -> ScriptBuf {
    use bitcoin::script::{Builder, PushBytesBuf};
    let mut b = Builder::new();
    for (i, part) in parts.iter().enumerate() {
        b = b.push_slice(PushBytesBuf::try_from(part.to_vec()).unwrap());
        if let Some(op) = ops.get(i) {
            b = b.push_opcode(*op);
        }
    }
    b.into_script()
}

/// `and_v(v:pk(A),pk(B))`.
fn and_v_script(a: &[u8], b: &[u8]) -> ScriptBuf {
    use bitcoin::opcodes::all::{OP_CHECKSIG, OP_CHECKSIGVERIFY};
    script_of(&[a, b], &[OP_CHECKSIGVERIFY, OP_CHECKSIG])
}

/// `pk(A)`.
fn pk_script(a: &[u8]) -> ScriptBuf {
    script_of(&[a], &[bitcoin::opcodes::all::OP_CHECKSIG])
}

/// `pkh(A)`: `DUP HASH160 <hash> EQUALVERIFY CHECKSIG`.
fn pkh_script(a: &[u8]) -> ScriptBuf {
    use bitcoin::hashes::{Hash, hash160};
    use bitcoin::opcodes::all::{OP_CHECKSIG, OP_DUP, OP_EQUALVERIFY, OP_HASH160};
    bitcoin::script::Builder::new()
        .push_opcode(OP_DUP)
        .push_opcode(OP_HASH160)
        .push_slice(hash160::Hash::hash(a).to_byte_array())
        .push_opcode(OP_EQUALVERIFY)
        .push_opcode(OP_CHECKSIG)
        .into_script()
}

const SEGWIT_AND_P2SH: [&str; 3] = ["p2wsh", "p2sh-p2wsh", "p2sh"];

#[test]
fn a_miniscript_input_is_judged_by_the_interpreter() {
    let (a, b, c) = (key_bytes(5, true), key_bytes(6, true), key_bytes(7, true));
    let junk = vec![0xAA; 9];
    let and_v = and_v_script(&a, &b);
    // Both signatures, the later key's first: a true spend.
    check_all(
        "and_v",
        &SEGWIT_AND_P2SH,
        &and_v,
        &|d| vec![sig_bytes(6, d), sig_bytes(5, d)],
        false,
    );
    // One signature, or a junk element in place of the other: no spend.
    check_all(
        "and_v, one signature",
        &SEGWIT_AND_P2SH,
        &and_v,
        &|d| vec![sig_bytes(5, d)],
        true,
    );
    check_all(
        "and_v, junk",
        &SEGWIT_AND_P2SH,
        &and_v,
        &|d| vec![junk.clone(), sig_bytes(5, d)],
        true,
    );
    // thresh(2,pk(A),s:pk(B),s:pk(C)), the third left empty.
    let thresh = {
        use bitcoin::opcodes::all::{OP_ADD, OP_CHECKSIG, OP_EQUAL, OP_SWAP};
        bitcoin::script::Builder::new()
            .push_slice(bitcoin::script::PushBytesBuf::try_from(a.clone()).unwrap())
            .push_opcode(OP_CHECKSIG)
            .push_opcode(OP_SWAP)
            .push_slice(bitcoin::script::PushBytesBuf::try_from(b.clone()).unwrap())
            .push_opcode(OP_CHECKSIG)
            .push_opcode(OP_ADD)
            .push_opcode(OP_SWAP)
            .push_slice(bitcoin::script::PushBytesBuf::try_from(c.clone()).unwrap())
            .push_opcode(OP_CHECKSIG)
            .push_opcode(OP_ADD)
            .push_int(2)
            .push_opcode(OP_EQUAL)
            .into_script()
    };
    check_all(
        "thresh",
        &["p2wsh"],
        &thresh,
        &|d| vec![vec![], sig_bytes(6, d), sig_bytes(5, d)],
        false,
    );
    check_all(
        "thresh, one signature",
        &["p2wsh"],
        &thresh,
        &|d| vec![vec![], vec![], sig_bytes(5, d)],
        true,
    );
    // pk(A): a signature, or junk, nothing or an empty signature.
    let pk = pk_script(&a);
    check_all(
        "pk",
        &SEGWIT_AND_P2SH,
        &pk,
        &|d| vec![sig_bytes(5, d)],
        false,
    );
    check_all(
        "pk, junk",
        &SEGWIT_AND_P2SH,
        &pk,
        &|_| vec![junk.clone()],
        true,
    );
    check_all("pk, nothing", &SEGWIT_AND_P2SH, &pk, &|_| vec![], true);
    check_all("pk, empty", &SEGWIT_AND_P2SH, &pk, &|_| vec![vec![]], true);
}

#[test]
fn a_pkh_input_is_judged_by_the_interpreter() {
    let (a, b) = (key_bytes(5, true), key_bytes(6, true));
    let junk = vec![0xAA; 9];
    // pkh(A): the signature and the key; junk, a wrong key and a forged
    // signature are refused.
    let pkh = pkh_script(&a);
    check_all(
        "pkh",
        &SEGWIT_AND_P2SH,
        &pkh,
        &|d| vec![sig_bytes(5, d), a.clone()],
        false,
    );
    check_all(
        "pkh, junk",
        &["p2wsh"],
        &pkh,
        &|_| vec![junk.clone(), a.clone()],
        true,
    );
    check_all(
        "pkh, wrong key",
        &["p2wsh"],
        &pkh,
        &|d| vec![sig_bytes(5, d), b.clone()],
        true,
    );
    check_all(
        "pkh, forged",
        &["p2wsh", "p2sh"],
        &pkh,
        &|d| vec![sig_bytes(7, d), a.clone()],
        true,
    );
}

#[test]
fn a_key_named_twice_takes_its_signature_twice() {
    let (a, b) = (key_bytes(5, true), key_bytes(6, true));
    let two = multisig_script(2, &[a.clone(), a.clone()]);
    check_all(
        "2-of-2 [A,A]",
        &WRAPPERS,
        &two,
        &|d| vec![vec![], sig_bytes(5, d), sig_bytes(5, d)],
        false,
    );
    let aab = multisig_script(2, &[a.clone(), a.clone(), b.clone()]);
    let aba = multisig_script(2, &[a.clone(), b.clone(), a.clone()]);
    check_all(
        "[A,A,B] sigA sigA",
        &WRAPPERS,
        &aab,
        &|d| vec![vec![], sig_bytes(5, d), sig_bytes(5, d)],
        false,
    );
    check_all(
        "[A,A,B] sigA sigB",
        &WRAPPERS,
        &aab,
        &|d| vec![vec![], sig_bytes(5, d), sig_bytes(6, d)],
        false,
    );
    check_all(
        "[A,B,A] sigB sigA",
        &WRAPPERS,
        &aba,
        &|d| vec![vec![], sig_bytes(6, d), sig_bytes(5, d)],
        false,
    );
    check_all(
        "[A,B,A] sigA sigB",
        &WRAPPERS,
        &aba,
        &|d| vec![vec![], sig_bytes(5, d), sig_bytes(6, d)],
        false,
    );
    // The signatures walk the keys forward: B's before A's is out of order
    // in [A,B].
    let ab = multisig_script(2, &[a.clone(), b.clone()]);
    check_all(
        "[A,B] sigB sigA",
        &WRAPPERS,
        &ab,
        &|d| vec![vec![], sig_bytes(6, d), sig_bytes(5, d)],
        true,
    );
    check_all(
        "[A,B] sigA sigA",
        &WRAPPERS,
        &ab,
        &|d| vec![vec![], sig_bytes(5, d), sig_bytes(5, d)],
        true,
    );
    // ALL and ALL|ANYONECANPAY on [A,A].
    for wrapper in WRAPPERS {
        for ty in [1u8, 0x81] {
            let mut psbt = multisig_spend(wrapper, &two, &|_| vec![]);
            let segwit = wrapper.ends_with("wsh");
            let digest = typed_digest(&psbt, &two, segwit, ty);
            let sig = typed_sig(5, digest, ty);
            let items = [vec![], sig.clone(), sig];
            psbt = multisig_spend(wrapper, &two, &|_| items.to_vec());
            assert!(!refused(&psbt), "{wrapper}: type {ty:#x}");
        }
    }
}

/// The digest of input 0 for `script` under a sighash type byte.
fn typed_digest(psbt: &bitcoin::Psbt, script: &ScriptBuf, segwit: bool, ty: u8) -> [u8; 32] {
    let mut cache = SighashCache::new(&psbt.unsigned_tx);
    if segwit {
        let ty = EcdsaSighashType::from_consensus(u32::from(ty));
        cache
            .p2wsh_signature_hash(0, script, Amount::from_sat(10_000), ty)
            .unwrap()
            .to_byte_array()
    } else {
        cache
            .legacy_signature_hash(0, script, u32::from(ty))
            .unwrap()
            .to_byte_array()
    }
}

fn typed_sig(seed: u8, digest: [u8; 32], ty: u8) -> Vec<u8> {
    let mut sig = sig_bytes(seed, digest);
    *sig.last_mut().unwrap() = ty;
    sig
}

/// Uncompressed keys in a p2wsh script are not read as a miniscript: the
/// order of the signatures is still the script's.
#[test]
fn signatures_out_of_the_keys_order_are_refused_where_the_script_is_not_a_miniscript() {
    let (a, b) = (key_bytes(5, false), key_bytes(6, false));
    let ab = multisig_script(2, &[a, b]);
    check_all(
        "in order",
        &["p2wsh"],
        &ab,
        &|d| vec![vec![], sig_bytes(5, d), sig_bytes(6, d)],
        false,
    );
    check_all(
        "reversed",
        &["p2wsh"],
        &ab,
        &|d| vec![vec![], sig_bytes(6, d), sig_bytes(5, d)],
        true,
    );
    check_all(
        "a repeat",
        &["p2wsh"],
        &ab,
        &|d| vec![vec![], sig_bytes(5, d), sig_bytes(5, d)],
        true,
    );
    check_all(
        "one short",
        &["p2wsh"],
        &ab,
        &|d| vec![vec![], sig_bytes(5, d)],
        true,
    );
}

/// The key is in the witness: a recovery path of `or_d(pk(A),
/// and_v(v:pkh(B),older(144)))`.
#[test]
fn a_key_given_in_the_witness_is_the_one_the_script_hashes() {
    use bitcoin::hashes::{Hash, hash160};
    use bitcoin::opcodes::all::{
        OP_CHECKSIG, OP_CHECKSIGVERIFY, OP_CSV, OP_DUP, OP_ENDIF, OP_EQUALVERIFY, OP_HASH160,
        OP_IFDUP, OP_NOTIF,
    };
    let (a, b) = (key_bytes(5, true), key_bytes(6, true));
    let script = bitcoin::script::Builder::new()
        .push_slice(bitcoin::script::PushBytesBuf::try_from(a.clone()).unwrap())
        .push_opcode(OP_CHECKSIG)
        .push_opcode(OP_IFDUP)
        .push_opcode(OP_NOTIF)
        .push_opcode(OP_DUP)
        .push_opcode(OP_HASH160)
        .push_slice(hash160::Hash::hash(&b).to_byte_array())
        .push_opcode(OP_EQUALVERIFY)
        .push_opcode(OP_CHECKSIGVERIFY)
        .push_int(144)
        .push_opcode(OP_CSV)
        .push_opcode(OP_ENDIF)
        .into_script();
    for wrapper in ["p2wsh", "p2sh-p2wsh", "p2sh"] {
        let at = |seq: u32, items: &dyn Fn([u8; 32]) -> Vec<Vec<u8>>| {
            spend_at(wrapper, &script, Sequence(seq), items)
        };
        // The recovery key, after the delay.
        let recovery = |d| vec![sig_bytes(6, d), b.clone(), vec![]];
        let psbt = at(144, &recovery);
        assert!(!refused(&psbt), "{wrapper}: the recovery spend");
        assert_eq!(rows_of(&psbt), vec![Verdict::Valid], "{wrapper}: its row");
        // Before the delay, with another key's signature, or the wrong key.
        assert!(refused(&at(143, &recovery)), "{wrapper}: too early");
        assert!(
            refused(&at(144, &|d| vec![sig_bytes(7, d), b.clone(), vec![]])),
            "{wrapper}: forged"
        );
        assert!(
            refused(&at(144, &|d| vec![sig_bytes(6, d), a.clone(), vec![]])),
            "{wrapper}: wrong key"
        );
        // The primary key.
        assert!(
            !refused(&at(144, &|d| vec![sig_bytes(5, d)])),
            "{wrapper}: primary"
        );
    }
}

#[test]
fn a_redeem_script_over_520_bytes_is_a_mismatch_and_a_16_of_16_in_p2wsh_stands() {
    use bitcoin::opcodes::all::{OP_DROP, OP_PUSHNUM_1};
    let puzzle = |len: usize| {
        bitcoin::script::Builder::new()
            .push_slice(bitcoin::script::PushBytesBuf::try_from(vec![7u8; len]).unwrap())
            .push_opcode(OP_DROP)
            .push_opcode(OP_PUSHNUM_1)
            .into_script()
    };
    let spend =
        |script: &ScriptBuf| final_input(p2sh_of(script), push_script(&[script.as_bytes()]), &[]);
    // 500 + 4 bytes, and 517 + 4 = 521 with its push opcodes.
    assert!(!witness_mismatch(&spend(&puzzle(500)), 0));
    assert!(spend(&puzzle(517)).inputs[0].final_script_sig.is_some());
    assert!(witness_mismatch(&spend(&puzzle(517)), 0));
    let keys: Vec<Vec<u8>> = (1..=16).map(|s| key_bytes(s, true)).collect();
    let sixteen = multisig_script(16, &keys);
    let signers: Vec<u8> = (1..=16).collect();
    assert!(!refused(&final_p2wsh_multisig(&sixteen, &signers)));
    assert!(refused(&final_p2sh_multisig(&sixteen, &signers)));
}

#[test]
fn a_witness_script_over_consensus_limits_is_a_mismatch() {
    use bitcoin::opcodes::all::{OP_DROP, OP_NOP, OP_PUSHNUM_1};
    let nops = |n: usize| {
        let mut b = bitcoin::script::Builder::new();
        for _ in 0..n {
            b = b.push_opcode(OP_NOP);
        }
        b.push_opcode(OP_PUSHNUM_1).into_script()
    };
    let spend = |script: &ScriptBuf, stack: usize| {
        let mut witness = vec![vec![]; stack];
        witness.push(script.to_bytes());
        final_input(p2wsh_of(script), ScriptBuf::new(), &witness)
    };
    assert!(!witness_shape_mismatch(&spend(&nops(200), 0), 0));
    assert!(witness_shape_mismatch(&spend(&nops(202), 0), 0));
    // 1,001 stack elements, and a script of 10,001 bytes.
    let one = nops(0);
    assert!(!witness_shape_mismatch(&spend(&one, 1000), 0));
    assert!(witness_shape_mismatch(&spend(&one, 1001), 0));
    let big = {
        let mut b = bitcoin::script::Builder::new();
        for _ in 0..20 {
            b = b.push_slice(bitcoin::script::PushBytesBuf::try_from(vec![1u8; 500]).unwrap());
            b = b.push_opcode(OP_DROP);
        }
        b.push_opcode(OP_PUSHNUM_1).into_script()
    };
    assert!(big.len() > 10_000);
    assert!(witness_shape_mismatch(&spend(&big, 0), 0));
}

/// A file of many signature-shaped elements costs a bounded number of
/// verifications: the rows past the bound read Unchecked.
#[test]
fn rows_stop_verifying_at_a_bound_per_input() {
    use bitcoin::opcodes::all::OP_DROP;
    // 280 keys and 200 drops: within the limits, and no miniscript.
    let mut b = bitcoin::script::Builder::new();
    for s in 1..=250u8 {
        b = b.push_slice(bitcoin::script::PushBytesBuf::try_from(key_bytes(s, true)).unwrap());
    }
    for _ in 0..200 {
        b = b.push_opcode(OP_DROP);
    }
    let script = b.push_int(1).into_script();
    let mut psbt = final_input(p2wsh_of(&script), ScriptBuf::new(), &[]);
    let digest = digest_of(&psbt, &script, true);
    let mut witness: Vec<Vec<u8>> = (0..400).map(|_| sig_bytes(250, digest)).collect();
    witness.push(script.to_bytes());
    psbt.inputs[0].final_script_witness = Some(Witness::from_slice(&witness));
    assert!(!witness_shape_mismatch(&psbt, 0));
    let rows = rows_of(&psbt);
    assert_eq!(rows.len(), 400);
    assert!(rows[..201].iter().all(|r| *r == Verdict::Valid));
    assert!(
        rows[201..]
            .iter()
            .all(|r| matches!(r, Verdict::Unchecked(_)))
    );
}

#[test]
fn a_bare_multisig_of_more_than_sixteen_keys_is_read() {
    let keys: Vec<Vec<u8>> = (1..=18).map(|s| key_bytes(s, true)).collect();
    let script = multisig_script(2, &keys);
    check_all(
        "2-of-18",
        &["bare"],
        &script,
        &|d| vec![vec![], sig_bytes(3, d), sig_bytes(17, d)],
        false,
    );
    check_all(
        "2-of-18, short",
        &["bare"],
        &script,
        &|d| vec![vec![], sig_bytes(3, d)],
        true,
    );
}

/// A multisig by high-S signatures stays refused: Faraday's own rule.
#[test]
fn a_high_s_signature_in_a_final_multisig_stays_refused() {
    let (a, b) = (key_bytes(5, true), key_bytes(6, true));
    let two = multisig_script(2, &[a, b]);
    let n = bitcoin::secp256k1::constants::CURVE_ORDER;
    let flip = |sig: Vec<u8>| {
        let parsed = bitcoin::secp256k1::ecdsa::Signature::from_der(&sig[..sig.len() - 1]).unwrap();
        let mut compact = parsed.serialize_compact();
        // s -> n - s
        let mut s = [0u8; 32];
        s.copy_from_slice(&compact[32..]);
        let mut out = [0u8; 32];
        let mut borrow = 0i16;
        for i in (0..32).rev() {
            let d = i16::from(n[i]) - i16::from(s[i]) - borrow;
            borrow = i16::from(d < 0);
            out[i] = (d + if d < 0 { 256 } else { 0 }) as u8;
        }
        compact[32..].copy_from_slice(&out);
        let mut der = bitcoin::secp256k1::ecdsa::Signature::from_compact(&compact)
            .unwrap()
            .serialize_der()
            .to_vec();
        der.push(1);
        der
    };
    check_all(
        "high S",
        &["p2wsh"],
        &two,
        &|d| vec![vec![], flip(sig_bytes(5, d)), sig_bytes(6, d)],
        true,
    );
}

/// A 1-of-16, and a 2-of-2 of uncompressed keys, are true in every wrapping
/// that carries them.
#[test]
fn a_one_of_sixteen_and_uncompressed_multisigs_stand() {
    let keys: Vec<Vec<u8>> = (1..=16).map(|s| key_bytes(s, true)).collect();
    let one = multisig_script(1, &keys);
    check_all(
        "1-of-16, the last key",
        &["p2wsh", "p2sh-p2wsh"],
        &one,
        &|d| vec![vec![], sig_bytes(16, d)],
        false,
    );
    let ukeys = [key_bytes(5, false), key_bytes(6, false)];
    let two = multisig_script(2, &ukeys);
    check_all(
        "2-of-2 uncompressed",
        &["p2sh", "bare"],
        &two,
        &|d| vec![vec![], sig_bytes(5, d), sig_bytes(6, d)],
        false,
    );
}

/// A bare p2pk of a compressed or an uncompressed key.
#[test]
fn a_p2pk_of_either_key_form_stands() {
    for compressed in [true, false] {
        let script = pk_script(&key_bytes(5, compressed));
        let mut psbt = final_input(script.clone(), ScriptBuf::new(), &[]);
        let digest = digest_of(&psbt, &script, false);
        psbt.inputs[0].final_script_sig = Some(push_script(&[&sig_bytes(5, digest)]));
        assert!(!refused(&psbt), "compressed {compressed}");
        assert_eq!(rows_of(&psbt), vec![Verdict::Valid]);
    }
}

/// SIGHASH_SINGLE of an input past the outputs signs the constant one.
#[test]
fn a_legacy_sighash_single_past_the_outputs_stands() {
    let script = pk_script(&key_bytes(5, true));
    let mut psbt = final_input(script.clone(), ScriptBuf::new(), &[]);
    let mut second = psbt.unsigned_tx.input[0].clone();
    second.previous_output.vout = 1;
    psbt.unsigned_tx.input.push(second);
    psbt.inputs.push(psbt.inputs[0].clone());
    psbt.inputs[0].final_script_sig = None;
    let mut one = [0u8; 32];
    one[0] = 1;
    let cache = SighashCache::new(&psbt.unsigned_tx);
    let at_one = cache.legacy_signature_hash(1, &script, 3).unwrap();
    assert_eq!(at_one.to_byte_array(), one);
    psbt.inputs[1].final_script_sig = Some(push_script(&[&typed_sig(5, one, 3)]));
    assert!(!witness_mismatch(&psbt, 1));
    assert!(!final_invalid(&psbt, 1));
    assert_eq!(rows_of(&psbt), vec![Verdict::Valid]);
}

/// One input final and the other not, in each order.
#[test]
fn an_input_beside_an_unfinished_one_stands_in_either_order() {
    let two = multisig_script(2, &[key_bytes(5, true), key_bytes(6, true)]);
    for final_at in [0usize, 1] {
        let mut psbt = final_input(p2wsh_of(&two), ScriptBuf::new(), &[]);
        let mut second = psbt.unsigned_tx.input[0].clone();
        second.previous_output.vout = 1;
        psbt.unsigned_tx.input.push(second);
        psbt.inputs.push(psbt.inputs[0].clone());
        let digest = SighashCache::new(&psbt.unsigned_tx)
            .p2wsh_signature_hash(
                final_at,
                &two,
                Amount::from_sat(10_000),
                EcdsaSighashType::All,
            )
            .unwrap()
            .to_byte_array();
        let parts = [
            vec![],
            sig_bytes(5, digest),
            sig_bytes(6, digest),
            two.to_bytes(),
        ];
        for i in 0..2 {
            psbt.inputs[i].final_script_sig = None;
        }
        psbt.inputs[final_at].final_script_witness = Some(Witness::from_slice(&parts));
        for i in 0..2 {
            assert!(
                !witness_mismatch(&psbt, i),
                "final at {final_at}, input {i}"
            );
            assert!(!final_invalid(&psbt, i), "final at {final_at}, input {i}");
        }
        assert_eq!(
            rows_of(&psbt),
            vec![Verdict::Valid; 2],
            "final at {final_at}"
        );
    }
}

/// A p2wsh hash time lock on both branches.
#[test]
fn a_p2wsh_hash_time_lock_stands_on_both_branches() {
    use bitcoin::hashes::sha256;
    use bitcoin::opcodes::all::{
        OP_CHECKSIG, OP_CSV, OP_DROP, OP_ELSE, OP_ENDIF, OP_EQUALVERIFY, OP_IF, OP_SHA256,
    };
    let (a, b) = (key_bytes(5, true), key_bytes(6, true));
    let preimage = [7u8; 32];
    let hash = sha256::Hash::hash(&preimage).to_byte_array();
    let script = bitcoin::script::Builder::new()
        .push_opcode(OP_IF)
        .push_opcode(OP_SHA256)
        .push_slice(hash)
        .push_opcode(OP_EQUALVERIFY)
        .push_slice(bitcoin::script::PushBytesBuf::try_from(a).unwrap())
        .push_opcode(OP_CHECKSIG)
        .push_opcode(OP_ELSE)
        .push_int(144)
        .push_opcode(OP_CSV)
        .push_opcode(OP_DROP)
        .push_slice(bitcoin::script::PushBytesBuf::try_from(b).unwrap())
        .push_opcode(OP_CHECKSIG)
        .push_opcode(OP_ENDIF)
        .into_script();
    let hashed = spend_at("p2wsh", &script, Sequence::MAX, &|d| {
        vec![sig_bytes(5, d), preimage.to_vec(), vec![1]]
    });
    assert!(!refused(&hashed), "the hash branch");
    assert_eq!(rows_of(&hashed), vec![Verdict::Valid]);
    let timed = spend_at("p2wsh", &script, Sequence(144), &|d| {
        vec![sig_bytes(6, d), vec![]]
    });
    assert!(!refused(&timed), "the time branch");
    assert_eq!(rows_of(&timed), vec![Verdict::Valid]);
}

/// A p2sh puzzle whose scriptSig pushes with OP_2 and OP_3, and an anchor.
#[test]
fn a_p2sh_puzzle_with_number_pushes_and_an_anchor_stand() {
    use bitcoin::opcodes::all::{OP_ADD, OP_EQUAL, OP_PUSHNUM_2, OP_PUSHNUM_3, OP_PUSHNUM_5};
    let puzzle = bitcoin::script::Builder::new()
        .push_opcode(OP_ADD)
        .push_opcode(OP_PUSHNUM_5)
        .push_opcode(OP_EQUAL)
        .into_script();
    let script_sig = bitcoin::script::Builder::new()
        .push_opcode(OP_PUSHNUM_2)
        .push_opcode(OP_PUSHNUM_3)
        .push_slice(bitcoin::script::PushBytesBuf::try_from(puzzle.to_bytes()).unwrap())
        .into_script();
    let psbt = final_input(p2sh_of(&puzzle), script_sig, &[]);
    assert!(!refused(&psbt));
    let anchor = ScriptBuf::from(vec![0x51, 0x02, 0x4e, 0x73]);
    let psbt = final_input(anchor, ScriptBuf::new(), &[vec![]]);
    assert!(!refused(&psbt));
}

/// ALL|ACP, NONE, SINGLE and SINGLE|ACP on a 2-of-2, in each wrapping.
#[test]
fn every_sighash_type_stands_on_a_two_of_two_in_each_wrapping() {
    let two = multisig_script(2, &[key_bytes(5, true), key_bytes(6, true)]);
    for ty in [0x81u8, 2, 3, 0x83] {
        for wrapper in WRAPPERS {
            let segwit = wrapper.ends_with("wsh");
            let blank = multisig_spend(wrapper, &two, &|_| vec![vec![], vec![], vec![]]);
            let digest = typed_digest(&blank, &two, segwit, ty);
            let psbt = multisig_spend(wrapper, &two, &|_| {
                vec![vec![], typed_sig(5, digest, ty), typed_sig(6, digest, ty)]
            });
            assert!(!refused(&psbt), "{wrapper}: type {ty:#x}");
            assert_eq!(
                rows_of(&psbt),
                vec![Verdict::Valid; 2],
                "{wrapper}: {ty:#x}"
            );
        }
    }
}

/// A corrupt signature in a 3-of-3 reads Invalid on its own row only.
#[test]
fn a_corrupt_signature_in_a_three_of_three_keeps_the_rows() {
    let keys: Vec<Vec<u8>> = (5..=7).map(|s| key_bytes(s, true)).collect();
    let three = multisig_script(3, &keys);
    for wrapper in WRAPPERS {
        let psbt = multisig_spend(wrapper, &three, &|d| {
            vec![
                vec![],
                sig_bytes(5, d),
                sig_bytes(6, [9; 32]),
                sig_bytes(7, d),
            ]
        });
        assert!(refused(&psbt), "{wrapper}");
        assert_eq!(
            rows_of(&psbt),
            vec![Verdict::Valid, Verdict::Invalid, Verdict::Valid],
            "{wrapper}"
        );
    }
}

/// The interpreter's answer is the one the signing state and `finish` both
/// take: a short stack of a miniscript is refused by each reader alone, a
/// spend it accepts by none.
#[test]
fn each_reader_takes_the_interpreters_answer() {
    let (a, b) = (key_bytes(5, true), key_bytes(6, true));
    let script = and_v_script(&a, &b);
    for wrapper in SEGWIT_AND_P2SH {
        let readers = |make: &dyn Fn([u8; 32]) -> Vec<Vec<u8>>| {
            let psbt = multisig_spend(wrapper, &script, make);
            (witness_mismatch(&psbt, 0), final_invalid(&psbt, 0))
        };
        assert_eq!(
            readers(&|d| vec![sig_bytes(5, d)]),
            (true, true),
            "{wrapper}: one signature of two"
        );
        assert_eq!(
            readers(&|d| vec![vec![9; 3], sig_bytes(5, d)]),
            (true, true),
            "{wrapper}: a junk element"
        );
        assert_eq!(
            readers(&|d| vec![sig_bytes(6, d), sig_bytes(5, d)]),
            (false, false),
            "{wrapper}: the true spend"
        );
    }
}

/// `and_v(v:pk(A),after(n))` or `and_v(v:pk(A),older(n))`.
fn locked(a: &[u8], n: i64, relative: bool) -> ScriptBuf {
    use bitcoin::opcodes::all::{OP_CHECKSIGVERIFY, OP_CLTV, OP_CSV};
    script_of(&[a], &[OP_CHECKSIGVERIFY])
        .into_bytes()
        .into_iter()
        .chain(
            bitcoin::script::Builder::new()
                .push_int(n)
                .push_opcode(if relative { OP_CSV } else { OP_CLTV })
                .into_script()
                .into_bytes(),
        )
        .collect::<Vec<u8>>()
        .into()
}

/// A time lock is read in full: `after` needs this input's sequence not
/// final (BIP-65) and `older` a transaction of version 2 or more
/// (BIP-112), as well as the value, and the spend says so in each wrapping.
#[test]
fn a_time_lock_the_transaction_does_not_meet_is_refused_in_full() {
    let a = key_bytes(5, true);
    let sign = |d: [u8; 32]| vec![sig_bytes(5, d)];
    let after = locked(&a, 100, false);
    let older = locked(&a, 10, true);
    let at = |lock: u32, seq: u32, version: i32| {
        move |tx: &mut bitcoin::Transaction| {
            tx.version = bitcoin::transaction::Version(version);
            tx.lock_time = bitcoin::absolute::LockTime::from_consensus(lock);
            tx.input[0].sequence = Sequence::from_consensus(seq);
        }
    };
    for wrapper in ["p2wsh", "p2sh-p2wsh", "p2sh"] {
        // The lock time is 100 and the sequence final: consensus fails it.
        let psbt = spend_in(wrapper, &after, &at(100, 0xFFFF_FFFF, 2), &sign);
        assert!(refused(&psbt), "{wrapper}: after, final sequence");
        // The same, with a sequence that is not final: a true spend.
        let psbt = spend_in(wrapper, &after, &at(100, 0xFFFF_FFFE, 2), &sign);
        assert!(!refused(&psbt), "{wrapper}: after, non-final sequence");
        assert_eq!(rows_of(&psbt), vec![Verdict::Valid], "{wrapper}");
        // Version 1 with the sequence at 10: consensus fails it.
        let psbt = spend_in(wrapper, &older, &at(0, 10, 1), &sign);
        assert!(refused(&psbt), "{wrapper}: older, version 1");
        // Version 2: a true spend.
        let psbt = spend_in(wrapper, &older, &at(0, 10, 2), &sign);
        assert!(!refused(&psbt), "{wrapper}: older, version 2");
        assert_eq!(rows_of(&psbt), vec![Verdict::Valid], "{wrapper}");
        // A version with its top bit set is a large one to consensus.
        let psbt = spend_in(wrapper, &older, &at(0, 10, -1), &sign);
        assert!(!refused(&psbt), "{wrapper}: older, version 0xFFFFFFFF");
    }
}

/// Of `or_i(and_v(v:pk(A),after(100)),pk(B))`, the branch not taken has
/// its lock unread: the other branch spends with a final sequence.
#[test]
fn a_time_lock_on_the_branch_not_taken_is_not_counted() {
    use bitcoin::opcodes::all::{
        OP_CHECKSIG, OP_CHECKSIGVERIFY, OP_CLTV, OP_ELSE, OP_ENDIF, OP_IF,
    };
    let (a, b) = (key_bytes(5, true), key_bytes(6, true));
    let push = |v: &[u8]| bitcoin::script::PushBytesBuf::try_from(v.to_vec()).unwrap();
    let script = bitcoin::script::Builder::new()
        .push_opcode(OP_IF)
        .push_slice(push(&a))
        .push_opcode(OP_CHECKSIGVERIFY)
        .push_int(100)
        .push_opcode(OP_CLTV)
        .push_opcode(OP_ELSE)
        .push_slice(push(&b))
        .push_opcode(OP_CHECKSIG)
        .push_opcode(OP_ENDIF)
        .into_script();
    let psbt = spend_at("p2wsh", &script, Sequence::MAX, &|d| {
        vec![sig_bytes(6, d), vec![]]
    });
    assert!(!refused(&psbt), "the branch with no lock");
    assert_eq!(rows_of(&psbt), vec![Verdict::Valid]);
}

/// A script that is not run is never refused and never reads wrong: a
/// signature on it is Unchecked. Its spend by BOLT-3's offered HTLC
/// revocation path, and a hash time lock given a signature of a key it
/// does not name, are neither refused nor judged.
#[test]
fn a_script_not_run_is_never_refused_and_no_row_reads_wrong() {
    use bitcoin::hashes::{hash160, sha256};
    use bitcoin::opcodes::all::*;
    let push = |v: &[u8]| bitcoin::script::PushBytesBuf::try_from(v.to_vec()).unwrap();
    let h160 = |v: &[u8]| hash160::Hash::hash(v).to_byte_array();
    let (rev, remote, local) = (key_bytes(5, true), key_bytes(6, true), key_bytes(7, true));
    let htlc = bitcoin::script::Builder::new()
        .push_opcode(OP_DUP)
        .push_opcode(OP_HASH160)
        .push_slice(h160(&rev))
        .push_opcode(OP_EQUAL)
        .push_opcode(OP_IF)
        .push_opcode(OP_CHECKSIG)
        .push_opcode(OP_ELSE)
        .push_slice(push(&remote))
        .push_opcode(OP_SWAP)
        .push_opcode(OP_SIZE)
        .push_int(32)
        .push_opcode(OP_EQUAL)
        .push_opcode(OP_NOTIF)
        .push_opcode(OP_DROP)
        .push_int(2)
        .push_opcode(OP_SWAP)
        .push_slice(push(&local))
        .push_int(2)
        .push_opcode(OP_CHECKMULTISIG)
        .push_opcode(OP_ELSE)
        .push_opcode(OP_HASH160)
        .push_slice([0x22u8; 20])
        .push_opcode(OP_EQUALVERIFY)
        .push_opcode(OP_CHECKSIG)
        .push_opcode(OP_ENDIF)
        .push_opcode(OP_ENDIF)
        .into_script();
    let revoked = spend_at("p2wsh", &htlc, Sequence::MAX, &|d| {
        vec![sig_bytes(5, d), rev.clone()]
    });
    let lock = bitcoin::script::Builder::new()
        .push_opcode(OP_IF)
        .push_opcode(OP_SHA256)
        .push_slice(sha256::Hash::hash(&[9u8; 32]).to_byte_array())
        .push_opcode(OP_EQUALVERIFY)
        .push_slice(push(&remote))
        .push_opcode(OP_CHECKSIG)
        .push_opcode(OP_ELSE)
        .push_int(144)
        .push_opcode(OP_CSV)
        .push_opcode(OP_DROP)
        .push_slice(push(&local))
        .push_opcode(OP_CHECKSIG)
        .push_opcode(OP_ENDIF)
        .into_script();
    let unnamed = spend_at("p2wsh", &lock, Sequence::MAX, &|d| {
        vec![sig_bytes(8, d), vec![9u8; 32], vec![1]]
    });
    for (what, psbt) in [("a revocation spend", revoked), ("an unnamed key", unnamed)] {
        assert!(!witness_mismatch(&psbt, 0), "{what}");
        assert!(!final_invalid(&psbt, 0), "{what}");
        assert!(not_evaluated(&psbt, 0), "{what}: unjudged");
        assert!(
            !rows_of(&psbt).contains(&Verdict::Invalid),
            "{what}: a row reads Invalid"
        );
    }
    // A wrong script hash is a proved mismatch, still refused.
    let mut wrong = spend_at("p2wsh", &lock, Sequence::MAX, &|d| {
        vec![sig_bytes(7, d), vec![9u8; 32], vec![1]]
    });
    wrong.inputs[0].witness_utxo.as_mut().unwrap().script_pubkey = p2wsh_of(&htlc);
    assert!(witness_mismatch(&wrong, 0));
    assert!(!not_evaluated(&wrong, 0));
}

/// A bare p2pk of a key off the curve is spent by nothing and is not
/// settled by a shape check: the script is not run.
#[test]
fn a_bare_p2pk_of_a_key_off_the_curve_is_not_run() {
    let mut off = vec![2u8];
    off.extend([0u8; 32]);
    let spk = pk_script(&off);
    let psbt = final_input(spk, push_script(&[&some_sig()]), &[]);
    assert!(not_evaluated(&psbt, 0));
    let on = pk_script(&key_bytes(5, true));
    let psbt = final_input(on, push_script(&[&some_sig()]), &[]);
    assert!(!not_evaluated(&psbt, 0));
}

/// A scriptSig over 10,000 bytes is over consensus's limits: a p2sh
/// puzzle (`<100 bytes> DROP 1`, not run) with filler pushes below its
/// redeem script is a spend at 9,000 bytes of scriptSig and none at over
/// 10,000.
#[test]
fn a_script_sig_over_ten_thousand_bytes_is_refused() {
    use bitcoin::opcodes::all::{OP_DROP, OP_PUSHNUM_1};
    let puzzle = bitcoin::script::Builder::new()
        .push_slice(bitcoin::script::PushBytesBuf::try_from(vec![7u8; 100]).unwrap())
        .push_opcode(OP_DROP)
        .push_opcode(OP_PUSHNUM_1)
        .into_script();
    let with_fillers = |n: usize| {
        let filler = vec![9u8; 520];
        let mut parts: Vec<&[u8]> = vec![filler.as_slice(); n];
        parts.push(puzzle.as_bytes());
        final_input(p2sh_of(&puzzle), push_script(&parts), &[])
    };
    let under = with_fillers(17);
    assert!(under.inputs[0].final_script_sig.as_ref().unwrap().len() < 10_000);
    assert!(!witness_mismatch(&under, 0));
    let over = with_fillers(20);
    assert!(over.inputs[0].final_script_sig.as_ref().unwrap().len() > 10_000);
    assert!(witness_mismatch(&over, 0));
}

/// A signature in a partial field and again in a final witness is one
/// signature, and a hash preimage in a script-path witness is none.
#[test]
fn a_signature_in_two_places_counts_once_and_a_preimage_not_at_all() {
    use faraday_psbt::verify::count_signatures;
    for script_type in [ScriptType::NativeSegwit, ScriptType::Taproot] {
        let signed = signed_single(script_type, Nonce::LowR, Aux::Deterministic, 29_000);
        assert_eq!(count_signatures(&signed), 1, "{script_type:?}");
        let mut finalized = signed.clone();
        finalize(&mut finalized).expect("a single-signature input finalizes");
        assert_eq!(count_signatures(&finalized), 1, "{script_type:?}");

        let mut both = signed;
        let input = &mut both.inner_mut().inputs[0];
        let done = &finalized.inner().inputs[0];
        input.final_script_witness = done.final_script_witness.clone();
        input.final_script_sig = done.final_script_sig.clone();
        assert_eq!(count_signatures(&both), 1, "{script_type:?} in both places");
    }

    let preimage = vec![0xab; 64];
    let psbt = leaf_spend(hashlock_leaf(&preimage), |sig| vec![sig, preimage.clone()]);
    assert_eq!(count_signatures(&psbt), 1);
}
