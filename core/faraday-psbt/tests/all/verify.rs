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
    Field, NonceMode, SigKey, Unchecked, Verdict, compare, deterministic, inscriptions,
    repeated_nonces, verify_signatures,
};
use faraday_psbt::{
    Aux, Context, Detail, KeyRef, Level, Nonce, Psbt, WarningKind, finalize, inspect, sign,
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
/// the signatures commit to.
fn leaf_spend_on(
    mut psbt: Psbt,
    script: impl Fn(&bitcoin::XOnlyPublicKey, &bitcoin::XOnlyPublicKey) -> ScriptBuf,
    witness: impl Fn(&LeafSpend) -> Vec<Vec<u8>>,
) -> Psbt {
    let secp = Secp256k1::new();
    let keypair = Keypair::from_secret_key(&secp, &SecretKey::from_slice(&[7; 32]).expect("key"));
    let keypair_b = Keypair::from_secret_key(&secp, &SecretKey::from_slice(&[9; 32]).expect("key"));
    let (xonly, _) = keypair.x_only_public_key();
    let (xonly_b, _) = keypair_b.x_only_public_key();
    let script = script(&xonly, &xonly_b);
    let mut control = vec![0xc0];
    control.extend_from_slice(&xonly.serialize());
    let spend = LeafSpend {
        script,
        control,
        xonly: [xonly, xonly_b],
        keypairs: [keypair, keypair_b],
        tx: psbt.unsigned_tx().clone(),
        prev: psbt.inner().inputs[0]
            .witness_utxo
            .clone()
            .expect("the output it spends"),
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
/// input asks ALL: a leaf that hashes the key, a CODESEPARATOR, an annex.
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
    assert_eq!(verdicts(&psbt), vec![Verdict::Invalid]);
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
    sig.push(0x04);
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
