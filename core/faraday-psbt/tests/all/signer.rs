//! Checking a signer (SIGNER-CHECK §9): every pair of committed PSBTs gives
//! the verdict and exactly the findings the fixture table names, each at
//! its level, and a clean pair is clean.

use bitcoin::bip32::{ChildNumber, DerivationPath, Fingerprint};
use bitcoin::hashes::{Hash, hash160, ripemd160, sha256, sha256d};
use bitcoin::psbt::raw;
use bitcoin::secp256k1::{Message, PublicKey, Scalar, Secp256k1, SecretKey};
use bitcoin::sighash::{EcdsaSighashType, SighashCache};
use bitcoin::taproot::{TapLeafHash, TapNodeHash};
use bitcoin::{Amount, OutPoint, ScriptBuf, Sequence, TxIn, TxOut, Witness, ecdsa};
use faraday_bip::keys::{MasterKey, Network, ScriptType};
use faraday_bip::policy::WalletPolicy;
use faraday_crypto::work::Compute;
use faraday_psbt::signer::{
    FieldName, Info, Kind, NotChecked, Report, Stated, Verdict, check, nonce_view, with_nonces,
};
use faraday_psbt::v2::PsbtV2;
use faraday_psbt::verify::{Field, Unchecked, Where, deterministic_all};
use faraday_psbt::{Context, Level, Psbt, finalize, inspect};

use crate::common::builder::{Build, funding_tx, single_sig, sorted_multisig};
use crate::common::{ABANDON, LEGAL, ZOO, master};

const MUSIG_POLICY: &str =
    include_str!("../../../../tools/vectors/psbt/wallet-musig-regtest.policy");

fn read(name: &str) -> Psbt {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tools/vectors/psbt")
        .join(name);
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{name}: {e}"));
    Psbt::parse_base64(&Compute::unchecked(), text.trim())
        .unwrap_or_else(|e| panic!("{name}: {e:?}"))
}

fn run_with(sent: &Psbt, returned: &Psbt, wallets: &[WalletPolicy]) -> Report {
    run_bytes(sent, returned, &returned.to_bytes(), wallets)
}

fn run_bytes(sent: &Psbt, returned: &Psbt, bytes: &[u8], wallets: &[WalletPolicy]) -> Report {
    run_files(sent, &sent.to_bytes(), returned, bytes, wallets)
}

/// Both files as the person has them, next to their version 0 forms.
fn run_files(
    sent: &Psbt,
    sent_bytes: &[u8],
    returned: &Psbt,
    bytes: &[u8],
    wallets: &[WalletPolicy],
) -> Report {
    let ctx = Context {
        network: Network::Regtest,
        keys: &[],
        wallets,
        musig_session: None,
    };
    check(
        &Compute::unchecked(),
        sent,
        returned,
        sent_bytes,
        bytes,
        &ctx,
    )
}

fn run(sent: &str, returned: &str) -> Report {
    run_with(&read(sent), &read(returned), &[])
}

/// SIGNER-CHECK §6's id of a finding.
fn id(kind: &Kind) -> &'static str {
    match kind {
        Kind::Version { .. } => "T1",
        Kind::Locktime { .. } => "T2",
        Kind::InputCount { .. } => "T3",
        Kind::OutputCount { .. } => "T4",
        Kind::Spends { .. } => "T5",
        Kind::Sequence { .. } => "T6",
        Kind::OutputAmount { .. } => "T7",
        Kind::OutputScript { .. } => "T8",
        Kind::SignatureInvalid { .. } => "S1",
        Kind::SignatureUnchecked { .. } => "S2",
        Kind::SignatureUnnamed { .. } => "S3",
        Kind::SignatureLeafUnknown { .. } => "S3l",
        Kind::SighashOther { .. } => "S4",
        Kind::SighashNone { .. } => "S4n",
        Kind::SighashStricter { .. } => "S4m",
        Kind::RepeatedNonce { .. } => "S5",
        Kind::NonceNotDeterministic { .. } => "S6",
        Kind::NonceNotRecomputed { .. } => "S6u",
        Kind::SignatureRemoved { .. } => "S7",
        Kind::SignatureChanged { .. } => "S8",
        Kind::NoSignatureAdded => "S9",
        Kind::FinalDiffers => "F1",
        Kind::FinalNotCompared => "F2",
        Kind::FinalChanged => "F3",
        Kind::FinalRemoved => "F4",
        Kind::FieldAdded(_) => "M1",
        Kind::FieldChanged(_) => "M2",
        Kind::FieldRemoved(_) => "M3",
        Kind::UnknownAdded { .. } => "P1",
        Kind::UnknownChanged { .. } => "P2",
        Kind::ProprietaryAdded { .. } => "P3",
        Kind::ProprietaryChanged { .. } => "P4",
        Kind::UnknownRemoved { .. } => "P5",
        Kind::ProprietaryRemoved { .. } => "P6",
        Kind::UnaccountedBytes { .. } => "E1",
        Kind::Unnamed(_) => "unnamed",
    }
}

fn found(report: &Report) -> Vec<(&'static str, Level, Where)> {
    report
        .findings
        .iter()
        .map(|f| (id(&f.kind), f.level, f.at))
        .collect()
}

fn assert_clean(report: &Report, finalized: usize, nonces: bool) {
    assert_eq!(report.findings, vec![], "findings");
    assert_eq!(report.verdict, Verdict::Clean { finalized, nonces });
}

fn assert_findings(report: &Report, want: &[(&'static str, Level, Where)], worst: Level) {
    assert_eq!(found(report), want);
    assert_eq!(
        report.verdict,
        Verdict::Findings {
            count: want.len(),
            worst
        }
    );
}

#[test]
fn what_core_signed_is_clean() {
    let report = run("signer-core-sent.psbt", "signer-core-signed.psbt");
    assert_clean(&report, 0, false);
    assert!(report.info.is_empty());
    assert_eq!(report.facts.added.len(), 1);
}

#[test]
fn what_core_finalized_is_clean_and_states_the_input() {
    let report = run("demo-regtest.psbt", "signer-core-final.psbt");
    assert_clean(&report, 1, false);
    assert_eq!(report.info, vec![Info::Finalized { input: 0 }]);
}

#[test]
fn a_witness_script_core_added_to_a_change_output_is_stated() {
    let report = run("wallet-2of3.psbt", "signer-core-signed.psbt");
    assert_clean(&report, 0, false);
    assert_eq!(
        report.info,
        vec![Info::FieldStated {
            at: Where::Output(1),
            field: Stated::WitnessScript
        }]
    );
}

#[test]
fn two_transactions_with_no_input_in_common_are_not_the_same() {
    let report = run("demo-regtest.psbt", "wallet-2of3.psbt");
    assert_eq!(report.verdict, Verdict::NotSameTransaction);
    assert_eq!(report.findings, vec![]);
    assert_eq!(report.info, vec![]);
}

#[test]
fn a_changed_output_is_blocked() {
    let report = run("signer-core-sent.psbt", "signer-output-changed.psbt");
    // The signature Core made was over the output as it was, so it no
    // longer verifies over this transaction either.
    assert_findings(
        &report,
        &[
            ("S1", Level::Blocked, Where::Input(0)),
            ("T7", Level::Blocked, Where::Output(0)),
        ],
        Level::Blocked,
    );
}

#[test]
fn a_signature_byte_changed_is_not_a_signature_of_the_transaction() {
    let report = run("signer-core-sent.psbt", "signer-invalid.psbt");
    assert_findings(
        &report,
        &[("S1", Level::Blocked, Where::Input(0))],
        Level::Blocked,
    );
}

#[test]
fn a_signature_under_a_key_no_origin_names_is_a_finding() {
    let report = run("signer-core-sent.psbt", "signer-foreign-key.psbt");
    assert_findings(
        &report,
        &[("S3", Level::Danger, Where::Input(0))],
        Level::Danger,
    );
}

#[test]
fn a_none_sighash_commits_to_no_output_and_is_blocked() {
    let report = run("demo-regtest.psbt", "signer-sighash-none.psbt");
    assert_findings(
        &report,
        &[("S4n", Level::Blocked, Where::Input(0))],
        Level::Blocked,
    );
}

#[test]
fn a_single_sighash_where_all_was_asked_is_danger() {
    let report = run("demo-regtest.psbt", "signer-sighash-single.psbt");
    assert_findings(
        &report,
        &[("S4", Level::Danger, Where::Input(0))],
        Level::Danger,
    );
}

#[test]
fn one_nonce_on_two_inputs_is_blocked() {
    let report = run("signer-nonce-reuse-sent.psbt", "signer-nonce-reuse.psbt");
    assert_findings(
        &report,
        &[("S5", Level::Blocked, Where::Transaction)],
        Level::Blocked,
    );
    let Kind::RepeatedNonce { inputs, .. } = &report.findings[0].kind else {
        panic!("a repeated nonce");
    };
    assert_eq!(inputs, &[0, 1]);
}

/// A signature of a loaded key that could not be recomputed (no prevout
/// to hash) is a finding even where Sent holds no origin to file it under.
#[test]
fn a_loaded_keys_signature_that_could_not_be_hashed_is_never_clean() {
    use faraday_psbt::verify::Determinism;
    let report = run_with(&demo(), &read("signer-random-nonce.psbt"), &[]);
    let mut report = report;
    let fp = Fingerprint::from(abandon().fingerprint());
    let added = &mut report.facts.added[0];
    added.fingerprint = None;
    let hit = Determinism {
        input: added.input,
        key: added.key.clone(),
        leaf: added.leaf,
        fingerprint: fp.into(),
        signature: added.bytes.clone(),
        hashed: false,
        mode: None,
    };
    let folded = with_nonces(report, &[hit], &[fp.into()]);
    assert_findings(
        &folded,
        &[("S6u", Level::Danger, Where::Input(0))],
        Level::Danger,
    );
}

#[test]
fn a_random_nonce_is_clean_without_the_key_and_a_finding_with_it() {
    let c = Compute::unchecked();
    let sent = read("demo-regtest.psbt");
    let returned = read("signer-random-nonce.psbt");
    let report = run_with(&sent, &returned, &[]);
    assert_clean(&report, 0, false);

    let none = with_nonces(report.clone(), &[], &[]);
    assert_clean(&none, 0, false);
    let recomputed = none.facts.recomputed.expect("the fact is stated");
    assert_eq!((recomputed.recomputed, recomputed.of), (0, 1));
    assert_eq!(recomputed.not_loaded.len(), 1);

    let abandon = master(ABANDON, Network::Regtest);
    let found = deterministic_all(&c, &returned, &[&abandon]);
    let folded = with_nonces(
        report,
        &found,
        &[Fingerprint::from(abandon.fingerprint()).into()],
    );
    assert_findings(
        &folded,
        &[("S6", Level::Danger, Where::Input(0))],
        Level::Danger,
    );
    let Kind::NonceNotDeterministic { fingerprint } = folded.findings[0].kind else {
        panic!("S6");
    };
    assert_eq!(fingerprint, Fingerprint::from(abandon.fingerprint()).into());
    assert!(
        folded
            .facts
            .recomputed
            .expect("stated")
            .not_loaded
            .is_empty()
    );
}

#[test]
fn a_signature_taken_out_is_caution() {
    let report = run("wallet-tapmulti-signed.psbt", "signer-sig-removed.psbt");
    // Nothing was added either.
    assert_findings(
        &report,
        &[
            ("S9", Level::Caution, Where::Transaction),
            ("S7", Level::Caution, Where::Input(0)),
        ],
        Level::Caution,
    );
}

#[test]
fn a_signature_remade_with_another_nonce_is_danger() {
    let report = run("signer-core-signed.psbt", "signer-sig-changed.psbt");
    // The signature is changed and nothing is added.
    assert_findings(
        &report,
        &[
            ("S8", Level::Danger, Where::Input(0)),
            ("S9", Level::Caution, Where::Transaction),
        ],
        Level::Danger,
    );
}

#[test]
fn an_extra_witness_item_is_danger() {
    let report = run("demo-regtest.psbt", "signer-final-extra.psbt");
    assert_findings(
        &report,
        &[("F1", Level::Danger, Where::Input(0))],
        Level::Danger,
    );
}

#[test]
fn key_origins_taken_out_are_caution() {
    let report = run("signer-core-sent.psbt", "signer-origin-removed.psbt");
    assert_findings(
        &report,
        &[("M3", Level::Caution, Where::Input(0))],
        Level::Caution,
    );
    assert_eq!(
        report.findings[0].kind,
        Kind::FieldRemoved(FieldName::Compared(Field::KeyOrigins))
    );
}

#[test]
fn a_global_xpub_added_is_danger() {
    let report = run("signer-core-sent.psbt", "signer-xpub-added.psbt");
    assert_findings(
        &report,
        &[("M1", Level::Danger, Where::Transaction)],
        Level::Danger,
    );
    assert_eq!(
        report.findings[0].kind,
        Kind::FieldAdded(FieldName::Compared(Field::Xpubs))
    );
}

#[test]
fn an_unknown_pair_added_is_named_by_its_type_and_sizes() {
    let report = run("signer-core-sent.psbt", "signer-unknown-added.psbt");
    assert_findings(
        &report,
        &[("P1", Level::Danger, Where::Transaction)],
        Level::Danger,
    );
    assert_eq!(
        report.findings[0].kind,
        Kind::UnknownAdded {
            type_value: 0xf0,
            key_len: 2,
            value_len: 4
        }
    );
}

#[test]
fn an_unknown_pair_whose_value_grew_is_changed() {
    let report = run("signer-unknown-sent.psbt", "signer-unknown-changed.psbt");
    assert_findings(
        &report,
        &[("P2", Level::Danger, Where::Transaction)],
        Level::Danger,
    );
    assert_eq!(
        report.findings[0].kind,
        Kind::UnknownChanged {
            type_value: 0xf0,
            from: 4,
            to: 6
        }
    );
}

#[test]
fn a_proprietary_pair_added_is_named_by_its_sizes() {
    let report = run("signer-core-sent.psbt", "signer-proprietary-added.psbt");
    assert_findings(
        &report,
        &[("P3", Level::Danger, Where::Input(0))],
        Level::Danger,
    );
    assert_eq!(
        report.findings[0].kind,
        Kind::ProprietaryAdded {
            prefix_len: 6,
            subtype: 1,
            value_len: 4
        }
    );
}

#[test]
fn an_osk_record_is_a_proprietary_pair_like_any_other() {
    let report = run("signer-core-sent.psbt", "signer-osk-added.psbt");
    assert_findings(
        &report,
        &[("P3", Level::Danger, Where::Input(0))],
        Level::Danger,
    );
}

#[test]
fn a_file_with_bytes_after_its_last_map_is_refused_at_read() {
    let text = std::fs::read_to_string(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../tools/vectors/psbt/signer-trailing.psbt"),
    )
    .expect("the fixture");
    assert!(Psbt::parse_base64(&Compute::unchecked(), text.trim()).is_err());
}

#[test]
fn a_length_written_in_more_bytes_than_it_needs_is_refused_at_read() {
    // The first key of the global map is one byte long (0x01); written as
    // 0xfd 0x01 0x00 it says the same in three bytes.
    let c = Compute::unchecked();
    let mut bytes = read("signer-core-signed.psbt").to_bytes();
    assert_eq!(bytes[5], 0x01);
    bytes.splice(5..6, [0xfd, 0x01, 0x00]);
    assert!(Psbt::parse_bytes(&c, &bytes).is_err(), "raw");
    let text = faraday_psbt::base64::encode(&bytes);
    assert!(Psbt::parse_base64(&c, &text).is_err(), "base64");
}

#[test]
fn swapping_a_clean_pair_gives_one_removal_per_signature() {
    let report = run("signer-core-signed.psbt", "signer-core-sent.psbt");
    assert_findings(
        &report,
        &[
            ("S9", Level::Caution, Where::Transaction),
            ("S7", Level::Caution, Where::Input(0)),
        ],
        Level::Caution,
    );
}

#[test]
fn a_pair_written_with_its_maps_in_another_order_is_clean() {
    let sent = read("signer-core-sent.psbt");
    let returned = read("signer-core-signed.psbt");
    let reordered = reverse_pairs(&returned.to_bytes());
    assert_ne!(reordered, returned.to_bytes());
    let again = Psbt::parse_bytes(&Compute::unchecked(), &reordered).expect("a valid PSBT");
    let report = run_with(&sent, &again, &[]);
    assert_clean(&report, 0, false);
}

/// The file with the key-value pairs of every map in the opposite order.
fn reverse_pairs(bytes: &[u8]) -> Vec<u8> {
    fn compact(bytes: &[u8], at: &mut usize) -> usize {
        let first = bytes[*at];
        *at += 1;
        match first {
            0xfd => {
                let n = usize::from(u16::from_le_bytes([bytes[*at], bytes[*at + 1]]));
                *at += 2;
                n
            }
            n => usize::from(n),
        }
    }
    let mut out = bytes[..5].to_vec();
    let mut at = 5;
    while at < bytes.len() {
        let mut pairs: Vec<&[u8]> = Vec::new();
        loop {
            let start = at;
            let key_len = compact(bytes, &mut at);
            if key_len == 0 {
                break;
            }
            at += key_len;
            let value_len = compact(bytes, &mut at);
            at += value_len;
            pairs.push(&bytes[start..at]);
        }
        for pair in pairs.iter().rev() {
            out.extend_from_slice(pair);
        }
        out.push(0);
    }
    out
}

#[test]
fn a_musig_nonce_and_partial_signature_added_is_clean() {
    let c = Compute::unchecked();
    let policy = WalletPolicy::parse_any(&c, MUSIG_POLICY).expect("the MuSig2 wallet");
    let report = run_with(
        &read("wallet-musig-core-first.psbt"),
        &read("wallet-musig-core-first-signed.psbt"),
        std::slice::from_ref(&policy),
    );
    assert_clean(&report, 0, true);
    assert_eq!(report.info.len(), 1);
    assert!(matches!(report.info[0], Info::NonceAdded { input: 0, .. }));
    assert_eq!(report.facts.partial_sigs_added, 1);
}

#[test]
fn a_musig_partial_signature_of_an_unregistered_wallet_is_not_checked() {
    let report = run(
        "wallet-musig-core-first.psbt",
        "wallet-musig-core-first-signed.psbt",
    );
    assert_findings(
        &report,
        &[("S2", Level::Caution, Where::Input(0))],
        Level::Caution,
    );
    assert!(matches!(
        report.findings[0].kind,
        Kind::SignatureUnchecked {
            reason: NotChecked::MusigWallet,
            ..
        }
    ));
}

#[test]
fn two_script_path_signatures_added_are_clean() {
    let report = run("wallet-tapmulti-first.psbt", "wallet-tapmulti-signed.psbt");
    assert_clean(&report, 0, false);
    assert_eq!(report.facts.added.len(), 2);
}

#[test]
fn a_musig_spend_finalized_by_core_drops_its_pairs_without_a_finding() {
    let report = run(
        "signer-core-musig-sent.psbt",
        "signer-core-musig-final.psbt",
    );
    assert_eq!(report.findings, vec![], "findings");
}

#[test]
fn musig_pairs_removed_from_an_input_that_is_not_final_are_caution() {
    let sent = read("signer-core-musig-sent.psbt");
    let mut returned = sent.clone();
    let input = &mut returned.inner_mut().inputs[0];
    input.unknown.clear();
    input.tap_key_sig = None;
    let report = run_with(&sent, &returned, &[]);
    assert!(
        found(&report).iter().any(|(id, level, at)| *id == "M3"
            && *level == Level::Caution
            && *at == Where::Input(0)),
        "{:?}",
        found(&report)
    );
}

#[test]
fn a_proprietary_pair_added_to_a_finalized_input_is_still_a_finding() {
    let sent = read("demo-regtest.psbt");
    let mut returned = read("signer-core-final.psbt");
    let key = bitcoin::psbt::raw::ProprietaryKey {
        prefix: b"fixture".to_vec(),
        subtype: 1,
        key: vec![],
    };
    returned.inner_mut().inputs[0]
        .proprietary
        .insert(key, vec![1, 2, 3, 4]);
    let report = run_with(&sent, &returned, &[]);
    assert_findings(
        &report,
        &[("P3", Level::Danger, Where::Input(0))],
        Level::Danger,
    );
}

// ---- Built in the tests, from the fixtures ----

const NET: Network = Network::Regtest;

fn sighash_of(psbt: &Psbt, index: usize, kind: EcdsaSighashType) -> Message {
    let input = &psbt.inner().inputs[index];
    let utxo = input.witness_utxo.as_ref().expect("a stated amount");
    let tx = psbt.unsigned_tx().clone();
    let mut cache = SighashCache::new(&tx);
    let hash = match &input.witness_script {
        Some(script) => cache
            .p2wsh_signature_hash(index, script, utxo.value, kind)
            .expect("the sighash"),
        None => cache
            .p2wpkh_signature_hash(index, &utxo.script_pubkey, utxo.value, kind)
            .expect("the sighash"),
    };
    Message::from(hash)
}

/// ECDSA with the nonce `k` chosen here: `s = k⁻¹(z + r·d)`.
fn ecdsa_with_k(
    secp: &Secp256k1<bitcoin::secp256k1::All>,
    d: &SecretKey,
    z: [u8; 32],
    k: &SecretKey,
) -> bitcoin::secp256k1::ecdsa::Signature {
    const N_MINUS_2: [u8; 32] = [
        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
        0xfe, 0xba, 0xae, 0xdc, 0xe6, 0xaf, 0x48, 0xa0, 0x3b, 0xbf, 0xd2, 0x5e, 0x8c, 0xd0, 0x36,
        0x41, 0x3f,
    ];
    let point = PublicKey::from_secret_key(secp, k);
    let x: [u8; 32] = point.serialize()[1..33].try_into().unwrap();
    let r = SecretKey::from_slice(&x).unwrap();
    let z = SecretKey::from_slice(&z).unwrap();
    let zrd = d
        .mul_tweak(&Scalar::from(r))
        .unwrap()
        .add_tweak(&Scalar::from(z))
        .unwrap();
    let mut inverse: Option<SecretKey> = None;
    for byte in N_MINUS_2 {
        for bit in (0..8).rev() {
            if let Some(acc) = inverse {
                inverse = Some(acc.mul_tweak(&Scalar::from(acc)).unwrap());
            }
            if byte >> bit & 1 == 1 {
                inverse = Some(match inverse {
                    Some(acc) => acc.mul_tweak(&Scalar::from(*k)).unwrap(),
                    None => *k,
                });
            }
        }
    }
    let s = zrd.mul_tweak(&Scalar::from(inverse.unwrap())).unwrap();
    let mut compact = [0u8; 64];
    compact[..32].copy_from_slice(&r.secret_bytes());
    compact[32..].copy_from_slice(&s.secret_bytes());
    let mut signature = bitcoin::secp256k1::ecdsa::Signature::from_compact(&compact).unwrap();
    signature.normalize_s();
    signature
}

/// How a test signature's nonce is made.
#[derive(Clone, Copy)]
enum Nonce {
    /// RFC 6979.
    Rfc6979,
    /// RFC 6979 with extra data: no rule of the device.
    Chosen,
    /// The nonce `k` of this byte.
    K(u8),
}

/// A signature of `master`'s key on input `index`, with its public key.
fn signature(
    psbt: &Psbt,
    index: usize,
    master: &MasterKey,
    kind: EcdsaSighashType,
    nonce: Nonce,
) -> (bitcoin::PublicKey, ecdsa::Signature) {
    let c = Compute::unchecked();
    let fingerprint: Fingerprint = master.fingerprint().into();
    let (pk, (_, path)) = psbt.inner().inputs[index]
        .bip32_derivation
        .iter()
        .find(|(_, (fp, _))| *fp == fingerprint)
        .expect("an origin of this master key");
    let derived = master.derive(&c, path);
    assert_eq!(derived.to_xpub(&c).public_key, *pk);
    let secp = Secp256k1::new();
    let key = *derived.secret_key();
    let msg = sighash_of(psbt, index, kind);
    let sig = match nonce {
        Nonce::Rfc6979 => secp.sign_ecdsa(&msg, &key),
        Nonce::Chosen => secp.sign_ecdsa_with_noncedata(&msg, &key, &[7; 32]),
        Nonce::K(byte) => ecdsa_with_k(
            &secp,
            &key,
            *msg.as_ref(),
            &SecretKey::from_slice(&[byte; 32]).unwrap(),
        ),
    };
    (
        bitcoin::PublicKey::new(*pk),
        ecdsa::Signature {
            signature: sig,
            sighash_type: kind,
        },
    )
}

fn signed(
    psbt: &Psbt,
    index: usize,
    master: &MasterKey,
    kind: EcdsaSighashType,
    nonce: Nonce,
) -> Psbt {
    let mut out = psbt.clone();
    let (pk, sig) = signature(psbt, index, master, kind, nonce);
    out.inner_mut().inputs[index].partial_sigs.insert(pk, sig);
    out
}

/// The input finalized as a finalizer writes it (the other fields it
/// clears gone), from `master`'s signature.
fn finalized(psbt: &Psbt, master: &MasterKey, nonce: Nonce) -> Psbt {
    let mut out = signed(psbt, 0, master, EcdsaSighashType::All, nonce);
    finalize(&mut out).expect("finalizes");
    assert!(out.inner().inputs[0].final_script_witness.is_some());
    out
}

fn abandon() -> MasterKey {
    master(ABANDON, NET)
}

fn demo() -> Psbt {
    read("demo-regtest.psbt")
}

/// Two inputs of one address and one output: input 1 has no output at
/// its index.
fn two_inputs_one_output() -> Psbt {
    let c = Compute::unchecked();
    let abandon = abandon();
    let theirs = master(ZOO, NET)
        .account_xpub(&c, ScriptType::NativeSegwit, 0)
        .unwrap()
        .address(false, 1)
        .unwrap()
        .script_pubkey();
    let mut b = Build::new(ScriptType::NativeSegwit, NET);
    b.recipients.push((theirs, 99_000));
    let mut psbt = single_sig(&abandon, &b);
    let script = abandon
        .account_xpub(&c, ScriptType::NativeSegwit, 0)
        .unwrap()
        .address(false, 0)
        .unwrap()
        .script_pubkey();
    let previous = funding_tx(script.clone(), 50_000);
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
    psbt
}

fn core_sent() -> Psbt {
    read("signer-core-sent.psbt")
}

fn core_signed() -> Psbt {
    read("signer-core-signed.psbt")
}

fn reparse(psbt: &Psbt) -> Psbt {
    Psbt::parse_bytes(&Compute::unchecked(), &psbt.to_bytes()).expect("a valid PSBT")
}

fn assert_one(report: &Report, id_: &'static str, level: Level, at: Where) {
    assert_findings(report, &[(id_, level, at)], level);
}

#[test]
fn a_chosen_signature_beside_final_fields_is_changed() {
    let sent = finalized(&demo(), &abandon(), Nonce::Rfc6979);
    let (pk, other) = signature(&demo(), 0, &abandon(), EcdsaSighashType::All, Nonce::Chosen);
    let mut returned = sent.clone();
    returned.inner_mut().inputs[0]
        .partial_sigs
        .insert(pk, other);
    let report = run_with(&sent, &returned, &[]);
    assert_findings(
        &report,
        &[
            ("S8", Level::Danger, Where::Input(0)),
            ("S9", Level::Caution, Where::Transaction),
        ],
        Level::Danger,
    );
}

#[test]
fn final_fields_dropped_for_another_signature_are_removed_and_changed() {
    let sent = finalized(&demo(), &abandon(), Nonce::Rfc6979);
    let returned = signed(&demo(), 0, &abandon(), EcdsaSighashType::All, Nonce::Chosen);
    let report = run_with(&sent, &returned, &[]);
    assert_findings(
        &report,
        &[
            ("M1", Level::Danger, Where::Input(0)),
            ("S8", Level::Danger, Where::Input(0)),
            ("S9", Level::Caution, Where::Transaction),
            ("F4", Level::Caution, Where::Input(0)),
        ],
        Level::Danger,
    );
}

#[test]
fn a_script_path_signature_over_a_leaf_the_sent_psbt_lacks_is_danger() {
    let sent = read("wallet-tapmulti-first.psbt");
    let mut returned = read("wallet-tapmulti-signed.psbt");
    let input = &mut returned.inner_mut().inputs[0];
    let ((x, leaf), sig) = input
        .tap_script_sigs
        .iter()
        .next()
        .map(|(k, v)| (*k, *v))
        .unwrap();
    input.tap_script_sigs.remove(&(x, leaf));
    let other = <TapLeafHash as bitcoin::hashes::Hash>::from_byte_array([0x42; 32]);
    input.tap_script_sigs.insert((x, other), sig);
    let report = run_with(&sent, &returned, &[]);
    // Its signature is not over that leaf either.
    assert_findings(
        &report,
        &[
            ("S1", Level::Blocked, Where::Input(0)),
            ("S3l", Level::Danger, Where::Input(0)),
        ],
        Level::Blocked,
    );
}

/// The MuSig2 pair of `ty` on input 0 of `psbt`, its key changed by `f`.
fn change_pair_key(psbt: &mut Psbt, ty: u8, f: impl Fn(&mut Vec<u8>)) {
    let unknown = &mut psbt.inner_mut().inputs[0].unknown;
    let keys: Vec<raw::Key> = unknown
        .keys()
        .filter(|k| k.type_value == ty)
        .cloned()
        .collect();
    assert!(!keys.is_empty());
    for key in keys {
        let value = unknown.remove(&key).unwrap();
        let mut changed = key.clone();
        f(&mut changed.key);
        unknown.insert(changed, value);
    }
}

fn musig_policy() -> WalletPolicy {
    WalletPolicy::parse_any(&Compute::unchecked(), MUSIG_POLICY).expect("the MuSig2 wallet")
}

#[test]
fn a_musig_pair_keyed_by_more_than_a_participant_is_a_field_not_a_nonce() {
    let policy = musig_policy();
    let sent = read("wallet-musig-core-first.psbt");
    let clean = read("wallet-musig-core-first-signed.psbt");
    let wallets = std::slice::from_ref(&policy);
    for (what, change) in [
        (
            "33 more bytes",
            Box::new(|k: &mut Vec<u8>| k.extend_from_slice(&[2; 33])) as Box<dyn Fn(&mut Vec<u8>)>,
        ),
        (
            "an aggregate the sent PSBT does not name",
            Box::new(|k: &mut Vec<u8>| {
                let g = PublicKey::from_secret_key(
                    &Secp256k1::new(),
                    &SecretKey::from_slice(&[9; 32]).unwrap(),
                );
                k[33..66].copy_from_slice(&g.serialize());
            }),
        ),
        (
            "a leaf hash the sent PSBT does not hold",
            Box::new(|k: &mut Vec<u8>| k.extend_from_slice(&[0x42; 32])),
        ),
    ] {
        let mut returned = clean.clone();
        change_pair_key(&mut returned, 0x1b, &change);
        let report = run_with(&sent, &reparse(&returned), wallets);
        let kinds: Vec<_> = report.findings.iter().map(|f| f.kind.clone()).collect();
        assert!(
            kinds.contains(&Kind::FieldAdded(FieldName::MusigNonce)),
            "{what}: {kinds:?}"
        );
        assert!(
            !report
                .info
                .iter()
                .any(|i| matches!(i, Info::NonceAdded { .. })),
            "{what}"
        );
        assert!(!matches!(report.verdict, Verdict::Clean { .. }), "{what}");
    }
}

#[test]
fn a_musig_partial_signature_keyed_by_more_than_a_participant_is_a_field() {
    let policy = musig_policy();
    let sent = read("wallet-musig-core-first.psbt");
    let mut returned = read("wallet-musig-core-first-signed.psbt");
    change_pair_key(&mut returned, 0x1c, |k| k.extend_from_slice(&[2; 33]));
    let report = run_with(&sent, &reparse(&returned), std::slice::from_ref(&policy));
    assert_eq!(report.facts.partial_sigs_added, 0);
    assert!(report.findings.iter().any(
        |f| f.kind == Kind::FieldAdded(FieldName::MusigPartialSig) && f.level == Level::Danger
    ));
    assert!(!matches!(report.verdict, Verdict::Clean { .. }));
}

#[test]
fn a_failing_musig_partial_signature_is_blocked() {
    let policy = musig_policy();
    let sent = read("wallet-musig-core-first.psbt");
    let mut returned = read("wallet-musig-core-first-signed.psbt");
    for value in returned.inner_mut().inputs[0]
        .unknown
        .iter_mut()
        .filter(|(k, _)| k.type_value == 0x1c)
        .map(|(_, v)| v)
    {
        value[31] ^= 1;
    }
    let report = run_with(&sent, &reparse(&returned), std::slice::from_ref(&policy));
    assert_one(&report, "S1", Level::Blocked, Where::Input(0));
}

#[test]
fn a_musig_nonce_and_partial_signature_taken_out_are_removed() {
    let policy = musig_policy();
    let sent = read("wallet-musig-core-first-signed.psbt");
    let returned = read("wallet-musig-core-first.psbt");
    let report = run_with(&sent, &returned, std::slice::from_ref(&policy));
    assert_findings(
        &report,
        &[
            ("S9", Level::Caution, Where::Transaction),
            ("M3", Level::Caution, Where::Input(0)),
            ("M3", Level::Caution, Where::Input(0)),
        ],
        Level::Caution,
    );
    let kinds: Vec<_> = report.findings.iter().map(|f| f.kind.clone()).collect();
    assert!(kinds.contains(&Kind::FieldRemoved(FieldName::MusigNonce)));
    assert!(kinds.contains(&Kind::FieldRemoved(FieldName::MusigPartialSig)));
}

#[test]
fn a_nonce_a_loaded_key_makes_is_recomputed_from_the_view() {
    let c = Compute::unchecked();
    let sent = demo();
    let returned = signed(&sent, 0, &abandon(), EcdsaSighashType::All, Nonce::Rfc6979);
    let report = run_with(&sent, &returned, &[]);
    let found = deterministic_all(&c, &nonce_view(&sent, &returned), &[&abandon()]);
    let folded = with_nonces(
        report,
        &found,
        &[Fingerprint::from(abandon().fingerprint()).into()],
    );
    assert_clean(&folded, 0, false);
    let recomputed = folded.facts.recomputed.expect("stated");
    assert_eq!((recomputed.recomputed, recomputed.of), (1, 1));
    assert!(recomputed.not_loaded.is_empty());
}

#[test]
fn a_nonce_in_final_fields_is_recomputed_with_the_sent_origins() {
    let c = Compute::unchecked();
    let sent = demo();
    let returned = finalized(&sent, &abandon(), Nonce::Rfc6979);
    assert!(returned.inner().inputs[0].bip32_derivation.is_empty());
    let report = run_with(&sent, &returned, &[]);
    let found = deterministic_all(&c, &nonce_view(&sent, &returned), &[&abandon()]);
    let folded = with_nonces(
        report,
        &found,
        &[Fingerprint::from(abandon().fingerprint()).into()],
    );
    assert_clean(&folded, 1, false);
    let recomputed = folded.facts.recomputed.expect("stated");
    assert_eq!((recomputed.recomputed, recomputed.of), (1, 1));
    assert!(recomputed.not_loaded.is_empty());
}

#[test]
fn a_finalized_input_with_a_chosen_nonce_is_s6() {
    let c = Compute::unchecked();
    let sent = demo();
    let returned = finalized(&sent, &abandon(), Nonce::Chosen);
    let report = run_with(&sent, &returned, &[]);
    let found = deterministic_all(&c, &nonce_view(&sent, &returned), &[&abandon()]);
    let folded = with_nonces(
        report,
        &found,
        &[Fingerprint::from(abandon().fingerprint()).into()],
    );
    assert_one(&folded, "S6", Level::Danger, Where::Input(0));
    let Kind::NonceNotDeterministic { fingerprint } = folded.findings[0].kind else {
        panic!("S6");
    };
    assert_eq!(
        fingerprint,
        Fingerprint::from(abandon().fingerprint()).into()
    );
}

#[test]
fn a_second_signature_under_one_key_in_another_leaf_is_judged_on_its_own() {
    let mut sent = read("wallet-tapmulti-first.psbt");
    let mut returned = read("wallet-tapmulti-signed.psbt");
    // A second leaf, which both PSBTs hold.
    let extra = {
        let (cb, (script, v)) = sent.inner().inputs[0]
            .tap_scripts
            .iter()
            .next()
            .map(|(c, s)| (c.clone(), s.clone()))
            .unwrap();
        let mut cb2 = cb;
        cb2.output_key_parity = bitcoin::key::Parity::Even;
        if sent.inner().inputs[0].tap_scripts.contains_key(&cb2) {
            cb2.output_key_parity = bitcoin::key::Parity::Odd;
        }
        let mut bytes = script.to_bytes();
        bytes.push(0x75);
        (cb2, (ScriptBuf::from_bytes(bytes), v))
    };
    for p in [&mut sent, &mut returned] {
        p.inner_mut().inputs[0]
            .tap_scripts
            .insert(extra.0.clone(), extra.1.clone());
    }
    let input = &mut returned.inner_mut().inputs[0];
    let (x, first, sig) = input
        .tap_script_sigs
        .iter()
        .next()
        .map(|((x, l), s)| (*x, *l, *s))
        .unwrap();
    let second = input
        .tap_scripts
        .values()
        .map(|(script, v)| TapLeafHash::from_script(script, *v))
        .find(|l| *l != first)
        .expect("a second leaf");
    input.tap_script_sigs.clear();
    input.tap_script_sigs.insert((x, first), sig);
    input.tap_script_sigs.insert((x, second), sig);
    let report = run_with(&sent, &returned, &[]);
    assert_eq!(report.facts.added.len(), 2);
    let masters = [abandon(), master(ZOO, NET), master(LEGAL, NET)];
    let view = nonce_view(&sent, &returned);
    let c = Compute::unchecked();
    let found: Vec<_> = masters
        .iter()
        .flat_map(|m| deterministic_all(&c, &view, &[m]))
        .collect();
    assert_eq!(found.len(), 2, "both signatures reach a result");
    assert!(found.iter().all(|d| d.hashed));
    let folded = with_nonces(
        report,
        &found,
        &[Fingerprint::from(abandon().fingerprint()).into()],
    );
    let recomputed = folded.facts.recomputed.clone().expect("stated");
    assert_eq!((recomputed.recomputed, recomputed.of), (1, 2));
    let s6: Vec<_> = folded
        .findings
        .iter()
        .filter(|f| matches!(f.kind, Kind::NonceNotDeterministic { .. }))
        .collect();
    assert_eq!(s6.len(), 1);
    assert_eq!((s6[0].level, s6[0].at), (Level::Danger, Where::Input(0)));
}

#[test]
fn a_single_signature_with_no_output_at_its_index_commits_to_none() {
    let abandon = abandon();
    // Asked: ALL (the default), then NONE.
    for asked in [None, Some(EcdsaSighashType::None)] {
        let mut sent = two_inputs_one_output();
        sent.inner_mut().inputs[1].sighash_type = asked.map(Into::into);
        let returned = signed(&sent, 1, &abandon, EcdsaSighashType::Single, Nonce::Rfc6979);
        let report = run_with(&sent, &returned, &[]);
        assert_one(&report, "S4n", Level::Blocked, Where::Input(1));
    }
}

#[test]
fn a_sighash_that_commits_to_more_than_asked_is_caution() {
    let mut sent = demo();
    sent.inner_mut().inputs[0].sighash_type = Some(EcdsaSighashType::None.into());
    let returned = signed(
        &sent,
        0,
        &abandon(),
        EcdsaSighashType::Single,
        Nonce::Rfc6979,
    );
    let report = run_with(&sent, &returned, &[]);
    assert_one(&report, "S4m", Level::Caution, Where::Input(0));
}

#[test]
fn a_signature_changed_to_one_with_the_same_nonce_gives_the_key_away() {
    let abandon = abandon();
    let sent = signed(&demo(), 0, &abandon, EcdsaSighashType::All, Nonce::K(5));
    let returned = signed(
        &demo(),
        0,
        &abandon,
        EcdsaSighashType::AllPlusAnyoneCanPay,
        Nonce::K(5),
    );
    let report = run_with(&sent, &returned, &[]);
    assert_findings(
        &report,
        &[
            ("S5", Level::Blocked, Where::Transaction),
            ("S8", Level::Danger, Where::Input(0)),
            ("S9", Level::Caution, Where::Transaction),
        ],
        Level::Blocked,
    );
}

#[test]
fn a_signature_not_checked_is_caution() {
    let mut sent = core_sent();
    let mut returned = core_signed();
    for psbt in [&mut sent, &mut returned] {
        let input = &mut psbt.inner_mut().inputs[0];
        input.witness_utxo = None;
        input.non_witness_utxo = None;
    }
    let report = run_with(&sent, &returned, &[]);
    assert_one(&report, "S2", Level::Caution, Where::Input(0));
    assert!(matches!(
        report.findings[0].kind,
        Kind::SignatureUnchecked {
            reason: NotChecked::Signature(Unchecked::NoPrevout),
            ..
        }
    ));
}

/// A 2-of-3 spend whose first key signed already.
fn multisig_with_one_signature() -> Psbt {
    let (a, z, l) = (abandon(), master(ZOO, NET), master(LEGAL, NET));
    let recipient = ScriptBuf::new_op_return(b"x");
    let psbt = sorted_multisig(&[&a, &z, &l], 2, NET, 100_000, &[(recipient, 99_000)]);
    signed(&psbt, 0, &a, EcdsaSighashType::All, Nonce::Rfc6979)
}

#[test]
fn finalizing_with_other_signatures_than_the_sent_ones_is_a_removal_only() {
    let (z, l) = (master(ZOO, NET), master(LEGAL, NET));
    let sent = multisig_with_one_signature();
    // The device finalizes with the second and third keys.
    let mut returned = sent.clone();
    returned.inner_mut().inputs[0].partial_sigs.clear();
    let returned = signed(&returned, 0, &z, EcdsaSighashType::All, Nonce::Rfc6979);
    let mut returned = signed(&returned, 0, &l, EcdsaSighashType::All, Nonce::Rfc6979);
    finalize(&mut returned).expect("finalizes");
    let report = run_with(&sent, &returned, &[]);
    assert_one(&report, "S7", Level::Caution, Where::Input(0));
}

#[test]
fn final_fields_with_too_few_signatures_cannot_be_compared() {
    let sent = multisig_with_one_signature();
    let mut returned = sent.clone();
    let input = &mut returned.inner_mut().inputs[0];
    let sig = *input.partial_sigs.values().next().unwrap();
    let script = input.witness_script.clone().unwrap();
    let mut witness = Witness::new();
    witness.push([]);
    witness.push(sig.to_vec());
    witness.push(script.as_bytes());
    input.partial_sigs.clear();
    input.final_script_witness = Some(witness);
    let report = run_with(&sent, &returned, &[]);
    assert!(
        report
            .findings
            .iter()
            .any(|f| f.kind == Kind::FinalNotCompared && f.level == Level::Danger),
        "{:?}",
        found(&report)
    );
}

#[test]
fn final_fields_changed_or_removed_are_findings() {
    let changed = run("signer-core-final.psbt", "signer-final-extra.psbt");
    assert_findings(
        &changed,
        &[
            ("F3", Level::Danger, Where::Input(0)),
            ("S9", Level::Caution, Where::Transaction),
            ("S7", Level::Caution, Where::Input(0)),
        ],
        Level::Danger,
    );
    let removed = run("signer-core-final.psbt", "demo-regtest.psbt");
    assert_findings(
        &removed,
        &[
            ("M1", Level::Danger, Where::Input(0)),
            ("S9", Level::Caution, Where::Transaction),
            ("F4", Level::Caution, Where::Input(0)),
        ],
        Level::Danger,
    );
}

#[test]
fn a_changed_script_is_changed_and_an_empty_one_is_not_absent() {
    let sent = core_sent();
    let field = FieldName::Compared(Field::WitnessScript);
    let mut changed = core_signed();
    changed.inner_mut().outputs[1].witness_script = Some(ScriptBuf::from_bytes(vec![0x51]));
    let report = run_with(&sent, &changed, &[]);
    assert_one(&report, "M2", Level::Danger, Where::Output(1));
    assert_eq!(report.findings[0].kind, Kind::FieldChanged(field));

    let mut emptied = core_signed();
    emptied.inner_mut().outputs[1].witness_script = Some(ScriptBuf::new());
    let report = run_with(&sent, &emptied, &[]);
    assert_one(&report, "M2", Level::Danger, Where::Output(1));

    let mut added = core_signed();
    added.inner_mut().inputs[0].redeem_script = Some(ScriptBuf::new());
    let report = run_with(&sent, &added, &[]);
    assert_one(&report, "M1", Level::Danger, Where::Input(0));
    assert_eq!(
        report.findings[0].kind,
        Kind::FieldAdded(FieldName::Compared(Field::RedeemScript))
    );
}

#[test]
fn unknown_and_proprietary_pairs_changed_or_removed_are_findings() {
    let key = raw::ProprietaryKey {
        prefix: b"fixture".to_vec(),
        subtype: 1,
        key: vec![],
    };
    let unknown = raw::Key {
        type_value: 0xf0,
        key: vec![0xaa],
    };
    let mut sent = core_sent();
    sent.inner_mut().inputs[0]
        .proprietary
        .insert(key.clone(), vec![1, 2]);
    let mut changed = core_signed();
    changed.inner_mut().inputs[0]
        .proprietary
        .insert(key.clone(), vec![1, 2, 3]);
    let report = run_with(&sent, &changed, &[]);
    assert_one(&report, "P4", Level::Danger, Where::Input(0));
    assert_eq!(
        report.findings[0].kind,
        Kind::ProprietaryChanged {
            prefix_len: 7,
            subtype: 1,
            from: 2,
            to: 3
        }
    );
    let report = run_with(&sent, &core_signed(), &[]);
    assert_one(&report, "P6", Level::Caution, Where::Input(0));

    let mut sent = core_sent();
    sent.inner_mut().inputs[0].unknown.insert(unknown, vec![1]);
    let report = run_with(&sent, &core_signed(), &[]);
    assert_one(&report, "P5", Level::Caution, Where::Input(0));
    assert_eq!(
        report.findings[0].kind,
        Kind::UnknownRemoved { type_value: 0xf0 }
    );
}

/// The returned file with a key written as 65 uncompressed bytes, which
/// reads as the same map.
fn with_uncompressed_key(psbt: &Psbt) -> Vec<u8> {
    let bytes = psbt.to_bytes();
    let pk = *psbt.inner().inputs[0]
        .bip32_derivation
        .keys()
        .next()
        .unwrap();
    let mut old = vec![0x22, 0x06];
    old.extend_from_slice(&pk.serialize());
    let mut new = vec![0x42, 0x06];
    new.extend_from_slice(&pk.serialize_uncompressed());
    let at = bytes
        .windows(old.len())
        .position(|w| w == old)
        .expect("the key in the file");
    let mut out = bytes[..at].to_vec();
    out.extend_from_slice(&new);
    out.extend_from_slice(&bytes[at + old.len()..]);
    out
}

#[test]
fn a_key_written_in_more_bytes_than_it_needs_is_unaccounted_bytes() {
    let sent = core_sent();
    let signed = core_signed();
    let bytes = with_uncompressed_key(&signed);
    let again = Psbt::parse_bytes(&Compute::unchecked(), &bytes).expect("it parses");
    assert_eq!(again, signed);
    let report = run_bytes(&sent, &again, &bytes, &[]);
    assert_one(&report, "E1", Level::Danger, Where::Transaction);
    assert_eq!(
        report.findings[0].kind,
        Kind::UnaccountedBytes { count: 32 }
    );
}

#[test]
fn an_explicit_version_of_zero_is_not_unaccounted_bytes() {
    let sent = core_sent();
    let signed = core_signed();
    let mut bytes = signed.to_bytes();
    // The global map's first pair, the version, before the others.
    bytes.splice(5..5, [0x01, 0xfb, 0x04, 0, 0, 0, 0]);
    let again = Psbt::parse_bytes(&Compute::unchecked(), &bytes).expect("it parses");
    let report = run_bytes(&sent, &again, &bytes, &[]);
    // Not E1; the explicit field itself is the one finding (M1).
    assert_one(&report, "M1", Level::Danger, Where::Transaction);
}

// ---- Fields a signer adds that are checked in full (SIGNER-CHECK §6.9, F199)

/// How much of the spent output Sent holds.
#[derive(Clone, Copy, PartialEq)]
enum Held {
    Both,
    NonWitness,
    WitnessOnly,
    Nothing,
}

/// Sent: one input spending a funding output paying `script`, holding
/// `held` of it, no signature and no key origin; and the funding
/// transaction.
fn sent_spending(script: ScriptBuf, held: Held) -> (Psbt, bitcoin::Transaction) {
    let mut b = Build::new(ScriptType::NativeSegwit, NET);
    b.recipients.push((ScriptBuf::new_op_return([1]), 99_000));
    let mut psbt = single_sig(&abandon(), &b);
    let funding = funding_tx(script, 100_000);
    let inner = psbt.inner_mut();
    inner.unsigned_tx.input[0].previous_output = OutPoint::new(funding.compute_txid(), 0);
    let input = &mut inner.inputs[0];
    input.bip32_derivation.clear();
    input.witness_utxo =
        matches!(held, Held::Both | Held::WitnessOnly).then(|| funding.output[0].clone());
    input.non_witness_utxo = matches!(held, Held::Both | Held::NonWitness).then(|| funding.clone());
    (reparse(&psbt), funding)
}

/// Sent against Returned made by `change` on input 0.
fn adding(sent: &Psbt, change: impl FnOnce(&mut bitcoin::psbt::Input)) -> Report {
    let mut returned = sent.clone();
    change(&mut returned.inner_mut().inputs[0]);
    run_with(sent, &reparse(&returned), &[])
}

/// What an added field that is stated gives: its Info line and, with no
/// signature in the pair, S9.
fn assert_stated(report: &Report, at: Where, field: Stated) {
    assert_eq!(
        found(report),
        vec![("S9", Level::Caution, Where::Transaction)]
    );
    assert_eq!(report.info, vec![Info::FieldStated { at, field }]);
}

/// What an added field that is not stated gives: M1 at Danger.
fn assert_added(report: &Report, at: Where) {
    assert_eq!(
        found(report),
        vec![
            ("M1", Level::Danger, at),
            ("S9", Level::Caution, Where::Transaction)
        ]
    );
    assert!(report.info.is_empty(), "{:?}", report.info);
}

fn script_of(byte: u8) -> ScriptBuf {
    ScriptBuf::from_bytes(vec![byte])
}

fn p2wsh_of(script: &ScriptBuf) -> ScriptBuf {
    ScriptBuf::new_p2wsh(&script.wscript_hash())
}

#[test]
fn a_witness_script_added_to_an_input_is_stated_when_it_hashes_to_the_program() {
    let (sent, _) = sent_spending(p2wsh_of(&script_of(0x51)), Held::NonWitness);
    let right = adding(&sent, |i| i.witness_script = Some(script_of(0x51)));
    assert_stated(&right, Where::Input(0), Stated::WitnessScript);
    let wrong = adding(&sent, |i| i.witness_script = Some(script_of(0x52)));
    assert_added(&wrong, Where::Input(0));
}

#[test]
fn a_witness_script_of_a_p2sh_p2wsh_input_is_anchored_without_a_redeem_script() {
    let ws = script_of(0x51);
    let spk = ScriptBuf::new_p2sh(&p2wsh_of(&ws).script_hash());
    let (sent, _) = sent_spending(spk, Held::WitnessOnly);
    let right = adding(&sent, |i| i.witness_script = Some(ws.clone()));
    assert_stated(&right, Where::Input(0), Stated::WitnessScript);
    let wrong = adding(&sent, |i| i.witness_script = Some(script_of(0x52)));
    assert_added(&wrong, Where::Input(0));
}

#[test]
fn a_witness_script_added_to_an_output_is_stated_when_it_hashes_to_the_program() {
    let ws = script_of(0x51);
    let (mut sent, _) = sent_spending(p2wsh_of(&script_of(0x51)), Held::Both);
    sent.inner_mut().unsigned_tx.output[0].script_pubkey = p2wsh_of(&ws);
    for (script, stated) in [(ws.clone(), true), (script_of(0x52), false)] {
        let mut returned = sent.clone();
        returned.inner_mut().outputs[0].witness_script = Some(script);
        let report = run_with(&sent, &reparse(&returned), &[]);
        if stated {
            assert_stated(&report, Where::Output(0), Stated::WitnessScript);
        } else {
            assert_added(&report, Where::Output(0));
        }
    }
}

#[test]
fn a_redeem_script_added_is_stated_when_its_hash_is_in_the_p2sh_script() {
    let rs = script_of(0x51);
    let spk = ScriptBuf::new_p2sh(&rs.script_hash());
    let (sent, _) = sent_spending(spk.clone(), Held::Both);
    let right = adding(&sent, |i| i.redeem_script = Some(rs.clone()));
    assert_stated(&right, Where::Input(0), Stated::RedeemScript);
    let wrong = adding(&sent, |i| i.redeem_script = Some(script_of(0x52)));
    assert_added(&wrong, Where::Input(0));
    let mut with_output = sent.clone();
    with_output.inner_mut().unsigned_tx.output[0].script_pubkey = spk;
    for (script, stated) in [(rs, true), (script_of(0x52), false)] {
        let mut returned = with_output.clone();
        returned.inner_mut().outputs[0].redeem_script = Some(script);
        let report = run_with(&with_output, &reparse(&returned), &[]);
        if stated {
            assert_stated(&report, Where::Output(0), Stated::RedeemScript);
        } else {
            assert_added(&report, Where::Output(0));
        }
    }
}

#[test]
fn a_witness_utxo_added_is_stated_when_it_is_the_output_sent_holds() {
    let (sent, funding) = sent_spending(p2wsh_of(&script_of(0x51)), Held::NonWitness);
    let right = adding(&sent, |i| i.witness_utxo = Some(funding.output[0].clone()));
    assert_stated(&right, Where::Input(0), Stated::SpentOutput);
    let wrong = adding(&sent, |i| {
        let mut other = funding.output[0].clone();
        other.value = Amount::from_sat(1);
        i.witness_utxo = Some(other);
    });
    assert_added(&wrong, Where::Input(0));
}

#[test]
fn a_non_witness_utxo_added_is_stated_when_the_txid_matches_and_no_witness_is_carried() {
    let (sent, funding) = sent_spending(p2wsh_of(&script_of(0x51)), Held::Nothing);
    let right = adding(&sent, |i| i.non_witness_utxo = Some(funding.clone()));
    assert_stated(&right, Where::Input(0), Stated::PreviousTransaction);
    let other = funding_tx(p2wsh_of(&script_of(0x52)), 100_000);
    let wrong = adding(&sent, |i| i.non_witness_utxo = Some(other));
    assert_added(&wrong, Where::Input(0));
}

#[test]
fn a_non_witness_utxo_with_the_right_txid_and_a_witness_is_danger() {
    let (sent, funding) = sent_spending(p2wsh_of(&script_of(0x51)), Held::Nothing);
    let mut witnessed = funding.clone();
    witnessed.input[0].witness = Witness::from_slice(&[vec![1, 2, 3]]);
    // The txid leaves the witness out: this is the transaction Sent spends.
    assert_eq!(witnessed.compute_txid(), funding.compute_txid());
    let report = adding(&sent, |i| i.non_witness_utxo = Some(witnessed));
    assert_added(&report, Where::Input(0));
}

#[test]
fn an_internal_key_and_merkle_root_added_are_stated_when_they_give_the_output_key() {
    let secp = Secp256k1::new();
    let internal = PublicKey::from_secret_key(&secp, &SecretKey::from_slice(&[7; 32]).unwrap())
        .x_only_public_key()
        .0;
    let root = TapNodeHash::from_byte_array([3; 32]);
    for (tree, spent_root) in [(None, None), (Some(root), Some(root))] {
        let spk = ScriptBuf::new_p2tr(&secp, internal, spent_root);
        let (sent, _) = sent_spending(spk, Held::WitnessOnly);
        let right = adding(&sent, |i| {
            i.tap_internal_key = Some(internal);
            i.tap_merkle_root = tree;
        });
        assert_stated(&right, Where::Input(0), Stated::TaprootKey);
        // A root where there is none, and no root where there is one.
        let wrong = adding(&sent, |i| {
            i.tap_internal_key = Some(internal);
            i.tap_merkle_root = if tree.is_some() { None } else { Some(root) };
        });
        // The internal key, and the root where one is added, each a field.
        let fields = if tree.is_some() { 1 } else { 2 };
        let mut want = vec![("M1", Level::Danger, Where::Input(0)); fields];
        want.push(("S9", Level::Caution, Where::Transaction));
        assert_eq!(found(&wrong), want);
        assert!(wrong.info.is_empty());
    }
    // A root added beside the internal key Sent held is one line.
    let spk = ScriptBuf::new_p2tr(&secp, internal, Some(root));
    let (mut sent, _) = sent_spending(spk, Held::WitnessOnly);
    sent.inner_mut().inputs[0].tap_internal_key = Some(internal);
    let held = adding(&sent, |i| i.tap_merkle_root = Some(root));
    assert_stated(&held, Where::Input(0), Stated::TaprootKey);
}

#[test]
fn a_sighash_type_added_is_stated_when_every_signature_has_it() {
    let (sent, _) = {
        let mut b = Build::new(ScriptType::NativeSegwit, NET);
        b.recipients.push((ScriptBuf::new_op_return([1]), 99_000));
        (single_sig(&abandon(), &b), ())
    };
    let signed_all = signed(&sent, 0, &abandon(), EcdsaSighashType::All, Nonce::Rfc6979);
    let mut right = signed_all.clone();
    right.inner_mut().inputs[0].sighash_type = Some(EcdsaSighashType::All.into());
    let report = run_with(&sent, &reparse(&right), &[]);
    assert_clean(&report, 0, false);
    assert_eq!(
        report.info,
        vec![Info::FieldStated {
            at: Where::Input(0),
            field: Stated::Sighash
        }]
    );
    // Another sighash than the signature's.
    let mut wrong = signed_all.clone();
    wrong.inner_mut().inputs[0].sighash_type = Some(EcdsaSighashType::AllPlusAnyoneCanPay.into());
    let report = run_with(&sent, &reparse(&wrong), &[]);
    assert!(
        report.findings.iter().any(|f| f.kind
            == Kind::FieldAdded(FieldName::Compared(Field::Sighash))
            && f.level == Level::Danger),
        "{:?}",
        found(&report)
    );
    assert!(report.info.is_empty());
    // No signature to compare it with.
    let mut none = sent.clone();
    none.inner_mut().inputs[0].sighash_type = Some(EcdsaSighashType::All.into());
    assert_added(&run_with(&sent, &reparse(&none), &[]), Where::Input(0));
}

#[test]
fn a_script_added_for_an_input_sent_holds_no_utxo_for_is_danger() {
    let ws = script_of(0x51);
    let (sent, _) = sent_spending(p2wsh_of(&ws), Held::Nothing);
    let report = adding(&sent, |i| i.witness_script = Some(ws));
    assert_added(&report, Where::Input(0));
}

#[test]
fn a_field_judged_only_by_the_returned_psbts_own_utxo_is_danger() {
    let ws = script_of(0x51);
    let (sent, funding) = sent_spending(p2wsh_of(&ws), Held::Nothing);
    let report = adding(&sent, |i| {
        i.non_witness_utxo = Some(funding.clone());
        i.witness_utxo = Some(funding.output[0].clone());
        i.witness_script = Some(ws);
    });
    // The non-witness UTXO is checked against the txid Sent spends; the
    // other two have only Returned's copy to be judged by.
    assert_eq!(
        found(&report),
        vec![
            ("M1", Level::Danger, Where::Input(0)),
            ("M1", Level::Danger, Where::Input(0)),
            ("S9", Level::Caution, Where::Transaction)
        ]
    );
    assert_eq!(
        report.info,
        vec![Info::FieldStated {
            at: Where::Input(0),
            field: Stated::PreviousTransaction
        }]
    );
}

/// The signed MuSig2 PSBT with the aggregate of every nonce and
/// partial-signature key Sent lacks set to `aggregate`.
fn musig_keyed_by(sent: &Psbt, aggregate: &PublicKey) -> Psbt {
    let mut returned = read("wallet-musig-core-first-signed.psbt");
    let held = &sent.inner().inputs[0].unknown;
    let unknown = &mut returned.inner_mut().inputs[0].unknown;
    let added: Vec<raw::Key> = unknown
        .keys()
        .filter(|k| matches!(k.type_value, 0x1b | 0x1c) && !held.contains_key(*k))
        .cloned()
        .collect();
    assert!(!added.is_empty());
    for key in added {
        let value = unknown.remove(&key).unwrap();
        let mut changed = key.clone();
        changed.key[33..66].copy_from_slice(&aggregate.serialize());
        unknown.insert(changed, value);
    }
    reparse(&returned)
}

#[test]
fn a_musig_pair_is_clean_under_each_of_the_three_aggregate_forms() {
    let policy = musig_policy();
    let wallets = std::slice::from_ref(&policy);
    let sent = read("wallet-musig-core-first.psbt");
    let ctx = Context {
        network: Network::Regtest,
        keys: &[],
        wallets,
        musig_session: None,
    };
    let established = inspect(&Compute::unchecked(), &sent, &ctx).inputs[0]
        .musig
        .clone()
        .expect("the wallet is established");
    let derived = {
        let mut bytes = vec![2];
        bytes.extend_from_slice(&established.internal_key.serialize());
        PublicKey::from_slice(&bytes).unwrap()
    };
    let flipped = {
        let mut bytes = established.output_key.serialize();
        bytes[0] ^= 1;
        PublicKey::from_slice(&bytes).unwrap()
    };
    for (what, aggregate) in [
        ("the root aggregate", established.root_aggregate),
        ("the derived aggregate", derived),
        ("the output key", established.output_key),
    ] {
        let report = run_with(&sent, &musig_keyed_by(&sent, &aggregate), wallets);
        assert_clean(&report, 0, true);
        assert_eq!(report.facts.partial_sigs_added, 1, "{what}");
    }
    // A fourth key, and the output key with the other parity: the reader
    // checks the output key's parity, so the pair is a field.
    let fourth =
        PublicKey::from_secret_key(&Secp256k1::new(), &SecretKey::from_slice(&[9; 32]).unwrap());
    for (what, aggregate) in [("a fourth key", fourth), ("the wrong parity", flipped)] {
        let report = run_with(&sent, &musig_keyed_by(&sent, &aggregate), wallets);
        let kinds: Vec<_> = report.findings.iter().map(|f| f.kind.clone()).collect();
        assert!(
            kinds.contains(&Kind::FieldAdded(FieldName::MusigNonce)),
            "{what}: {kinds:?}"
        );
        assert!(!matches!(report.verdict, Verdict::Clean { .. }), "{what}");
    }
}

/// `pk` in its uncompressed form.
fn uncompressed(pk: bitcoin::PublicKey) -> bitcoin::PublicKey {
    bitcoin::PublicKey {
        compressed: false,
        inner: pk.inner,
    }
}

#[test]
fn one_nonce_under_both_forms_of_a_key_is_blocked() {
    let sent = two_inputs_one_output();
    let (pk0, sig0) = signature(&sent, 0, &abandon(), EcdsaSighashType::All, Nonce::K(5));
    let (pk1, sig1) = signature(&sent, 1, &abandon(), EcdsaSighashType::All, Nonce::K(5));
    let mut returned = sent.clone();
    returned.inner_mut().inputs[0]
        .partial_sigs
        .insert(pk0, sig0);
    returned.inner_mut().inputs[1]
        .partial_sigs
        .insert(uncompressed(pk1), sig1);
    let report = run_with(&sent, &reparse(&returned), &[]);
    let kinds = found(&report);
    assert!(
        kinds.iter().any(|k| k.0 == "S5" && k.1 == Level::Blocked),
        "{kinds:?}"
    );
    assert!(matches!(
        report.verdict,
        Verdict::Findings {
            worst: Level::Blocked,
            ..
        }
    ));
}

#[test]
fn a_second_form_of_a_signed_key_is_not_clean() {
    let sent = signed(
        &demo(),
        0,
        &abandon(),
        EcdsaSighashType::All,
        Nonce::Rfc6979,
    );
    let (pk, _) = signature(
        &demo(),
        0,
        &abandon(),
        EcdsaSighashType::All,
        Nonce::Rfc6979,
    );
    let (_, other) = signature(&demo(), 0, &abandon(), EcdsaSighashType::All, Nonce::Chosen);
    let mut returned = sent.clone();
    returned.inner_mut().inputs[0]
        .partial_sigs
        .insert(uncompressed(pk), other);
    let report = run_with(&sent, &reparse(&returned), &[]);
    assert!(
        !matches!(report.verdict, Verdict::Clean { .. }),
        "{:?}",
        found(&report)
    );
    assert!(
        report.findings.iter().any(|f| f.level >= Level::Danger),
        "{:?}",
        found(&report)
    );
}

#[test]
fn two_signatures_under_one_key_in_other_fields_are_changed_and_not_recomputed_twice() {
    let c = Compute::unchecked();
    let sent = demo();
    let mut returned = finalized(&sent, &abandon(), Nonce::Rfc6979);
    let (pk, chosen) = signature(&sent, 0, &abandon(), EcdsaSighashType::All, Nonce::Chosen);
    returned.inner_mut().inputs[0]
        .partial_sigs
        .insert(pk, chosen);
    let report = run_with(&sent, &returned, &[]);
    assert!(
        report
            .findings
            .iter()
            .any(|f| matches!(f.kind, Kind::SignatureChanged { .. }) && f.level == Level::Danger),
        "{:?}",
        found(&report)
    );
    let found_ = deterministic_all(&c, &nonce_view(&sent, &returned), &[&abandon()]);
    let folded = with_nonces(report, &found_, &[]);
    let recomputed = folded.facts.recomputed.clone().expect("stated");
    assert_eq!((recomputed.recomputed, recomputed.of), (1, 2));
    // The final signature fills the view's place under the key: the
    // partial one (a chosen nonce) could not be recomputed there, which
    // is S6u (SIGNER-CHECK §6.2), once, and not S6.
    let ids: Vec<_> = folded
        .findings
        .iter()
        .filter(|f| {
            matches!(
                f.kind,
                Kind::NonceNotDeterministic { .. } | Kind::NonceNotRecomputed { .. }
            )
        })
        .map(|f| (id(&f.kind), f.level, f.at))
        .collect();
    assert_eq!(ids, [("S6u", Level::Danger, Where::Input(0))]);
}

#[test]
fn a_chosen_key_path_signature_beside_a_final_taproot_witness_is_changed() {
    let c = Compute::unchecked();
    let m = abandon();
    let mut b = Build::new(ScriptType::Taproot, NET);
    b.recipients.push((ScriptBuf::new_op_return([1]), 99_000));
    let sent = single_sig(&m, &b);
    let keys = [faraday_psbt::KeyRef::from_master(&c, &m, 0).expect("accounts")];
    let ctx = Context {
        network: NET,
        keys: &keys,
        wallets: &[],
        musig_session: None,
    };
    let sign_with = |aux| {
        let mut p = sent.clone();
        faraday_psbt::sign(
            &c,
            &mut p,
            &[&m],
            &[m.fingerprint()],
            &ctx,
            &[],
            faraday_psbt::Nonce::LowR,
            aux,
            &mut None,
            Some(&faraday_crypto::Zeroizing::new([0u8; 32])),
        )
        .expect("signs");
        p
    };
    let mut returned = sign_with(faraday_psbt::Aux::Deterministic);
    finalize(&mut returned).expect("finalizes");
    let chosen = sign_with(faraday_psbt::Aux::Fresh([9; 32]));
    returned.inner_mut().inputs[0].tap_key_sig = chosen.inner().inputs[0].tap_key_sig;
    assert!(returned.inner().inputs[0].final_script_witness.is_some());
    let report = run_with(&sent, &returned, &[]);
    assert!(
        report
            .findings
            .iter()
            .any(|f| matches!(f.kind, Kind::SignatureChanged { .. }) && f.level == Level::Danger),
        "{:?}",
        found(&report)
    );
    let found_ = deterministic_all(&c, &nonce_view(&sent, &returned), &[&m]);
    let folded = with_nonces(report, &found_, &[]);
    let recomputed = folded.facts.recomputed.expect("stated");
    assert!(recomputed.recomputed < recomputed.of, "{recomputed:?}");
}

#[test]
fn a_signature_that_needs_a_field_only_returned_has_is_recomputed_or_s6u() {
    let c = Compute::unchecked();
    // Sent names the key by origin but holds no UTXO; Returned adds the
    // non-witness UTXO (stated at Info) and a signature.
    let sent_full = demo();
    let mut sent = sent_full.clone();
    {
        let i = &mut sent.inner_mut().inputs[0];
        i.witness_utxo = None;
        i.non_witness_utxo = None;
    }
    let sent = reparse(&sent);
    for (nonce, want_s6) in [(Nonce::Rfc6979, false), (Nonce::Chosen, true)] {
        let (pk, sig) = signature(&sent_full, 0, &abandon(), EcdsaSighashType::All, nonce);
        let mut returned = sent.clone();
        {
            let i = &mut returned.inner_mut().inputs[0];
            i.witness_utxo = sent_full.inner().inputs[0].witness_utxo.clone();
            i.non_witness_utxo = sent_full.inner().inputs[0].non_witness_utxo.clone();
            i.partial_sigs.insert(pk, sig);
        }
        let report = run_with(&sent, &reparse(&returned), &[]);
        let found_ = deterministic_all(&c, &nonce_view(&sent, &returned), &[&abandon()]);
        let folded = with_nonces(report, &found_, &[]);
        let ids: Vec<_> = folded.findings.iter().map(|f| id(&f.kind)).collect();
        assert_eq!(ids.contains(&"S6"), want_s6, "{ids:?}");
        assert!(!ids.contains(&"S6u"), "{ids:?}");
        let r = folded.facts.recomputed.expect("stated");
        assert!(r.not_loaded.is_empty(), "{r:?}");
    }
    // A signature with no signature hash to recompute is S6u, whatever it is.
    let (pk, sig) = signature(
        &sent_full,
        0,
        &abandon(),
        EcdsaSighashType::All,
        Nonce::Rfc6979,
    );
    let mut returned = sent.clone();
    returned.inner_mut().inputs[0].partial_sigs.insert(pk, sig);
    let report = run_with(&sent, &reparse(&returned), &[]);
    let found_ = deterministic_all(&c, &nonce_view(&sent, &returned), &[&abandon()]);
    let folded = with_nonces(report, &found_, &[]);
    assert!(
        folded
            .findings
            .iter()
            .any(|f| matches!(f.kind, Kind::NonceNotRecomputed { .. }) && f.level == Level::Danger),
        "{:?}",
        found(&folded)
    );
    assert!(
        folded
            .facts
            .recomputed
            .expect("stated")
            .not_loaded
            .is_empty()
    );
}

#[test]
fn final_fields_with_too_few_signatures_and_other_bytes_are_danger() {
    let sent = multisig_with_one_signature();
    let mut returned = sent.clone();
    let input = &mut returned.inner_mut().inputs[0];
    let sig = *input.partial_sigs.values().next().unwrap();
    let script = input.witness_script.clone().unwrap();
    let mut witness = Witness::new();
    witness.push([]);
    witness.push(sig.to_vec());
    witness.push([0xde, 0xad]);
    witness.push(script.as_bytes());
    input.partial_sigs.clear();
    input.final_script_witness = Some(witness);
    let report = run_with(&sent, &returned, &[]);
    assert!(
        report
            .findings
            .iter()
            .any(|f| f.kind == Kind::FinalNotCompared && f.level == Level::Danger),
        "{:?}",
        found(&report)
    );
}

#[test]
fn a_musig_partial_signature_keyed_with_a_leaf_hash_is_a_field_whichever_leaf() {
    let policy = musig_policy();
    let sent = read("wallet-musig-core-first.psbt");
    let mut returned = read("wallet-musig-core-first-signed.psbt");
    change_pair_key(&mut returned, 0x1c, |k| k.extend_from_slice(&[0x42; 32]));
    let report = run_with(&sent, &reparse(&returned), std::slice::from_ref(&policy));
    assert_eq!(report.facts.partial_sigs_added, 0);
    assert!(report.findings.iter().any(
        |f| f.kind == Kind::FieldAdded(FieldName::MusigPartialSig) && f.level == Level::Danger
    ));
    assert!(!matches!(report.verdict, Verdict::Clean { .. }));
}

#[test]
fn an_added_signature_of_a_finalized_input_with_no_key_loaded_names_its_master() {
    let sent = demo();
    let returned = finalized(&sent, &abandon(), Nonce::Rfc6979);
    let report = run_with(&sent, &returned, &[]);
    let fp = Fingerprint::from(abandon().fingerprint());
    let c = Compute::unchecked();
    let folded = with_nonces(
        report,
        &deterministic_all(&c, &nonce_view(&sent, &returned), &[]),
        &[],
    );
    let r = folded.facts.recomputed.expect("stated");
    assert_eq!(r.not_loaded, vec![fp.into()]);
}

#[test]
fn a_sighash_type_added_is_stated_for_two_signatures_only_when_both_have_it() {
    let sent = two_inputs_one_output();
    let mut s = sent.clone();
    s.inner_mut().inputs[0].sighash_type = None;
    let both = |second: EcdsaSighashType| {
        let a = signed(&sent, 0, &abandon(), EcdsaSighashType::All, Nonce::Rfc6979);
        let mut p = a.clone();
        p.inner_mut().inputs[0].sighash_type = Some(EcdsaSighashType::All.into());
        let (pk, sig) = signature(&sent, 0, &abandon(), second, Nonce::Rfc6979);
        // A second signature under a second key of the input.
        let mut x = pk;
        x.inner = SecretKey::from_slice(&[3; 32])
            .unwrap()
            .public_key(&Secp256k1::new());
        p.inner_mut().inputs[0].partial_sigs.insert(x, sig);
        run_with(&sent, &reparse(&p), &[])
    };
    let same = both(EcdsaSighashType::All);
    assert!(
        same.info.contains(&Info::FieldStated {
            at: Where::Input(0),
            field: Stated::Sighash
        }),
        "{:?}",
        same.info
    );
    let differs = both(EcdsaSighashType::AllPlusAnyoneCanPay);
    assert!(
        differs
            .findings
            .iter()
            .any(|f| f.kind == Kind::FieldAdded(FieldName::Compared(Field::Sighash))),
        "{:?}",
        found(&differs)
    );
}

// ---- The fields `compare` did not read, and the file's own bytes (S1)

/// Sent and Returned (a Core signature carried across) with `set` applied
/// as the case says: `None` clears the field, `Some(n)` sets its variant.
fn cases(at: Where, field: Field, set: impl Fn(&mut Psbt, Option<u8>)) {
    let make = |base: Psbt, v: Option<u8>| {
        let mut p = base;
        set(&mut p, v);
        p
    };
    let kinds = [
        (
            None,
            Some(1),
            "M1",
            Level::Danger,
            Kind::FieldAdded(FieldName::Compared(field)),
        ),
        (
            Some(1),
            Some(2),
            "M2",
            Level::Danger,
            Kind::FieldChanged(FieldName::Compared(field)),
        ),
        (
            Some(1),
            None,
            "M3",
            Level::Caution,
            Kind::FieldRemoved(FieldName::Compared(field)),
        ),
    ];
    for (from, to, want, level, kind) in kinds {
        let report = run_with(&make(core_sent(), from), &make(core_signed(), to), &[]);
        assert_one(&report, want, level, at);
        assert_eq!(report.findings[0].kind, kind, "{field:?} {want}");
    }
}

#[test]
fn hash_preimages_of_every_kind_added_changed_and_removed_are_findings() {
    let at = Where::Input(0);
    let n = |v: Option<u8>| v.map(|n| vec![n; 3]);
    cases(at, Field::Preimages, |p, v| {
        let m = &mut p.inner_mut().inputs[0].ripemd160_preimages;
        m.clear();
        if let Some(v) = n(v) {
            m.insert(ripemd160::Hash::hash(b"r"), v);
        }
    });
    cases(at, Field::Preimages, |p, v| {
        let m = &mut p.inner_mut().inputs[0].sha256_preimages;
        m.clear();
        if let Some(v) = n(v) {
            m.insert(sha256::Hash::hash(b"s"), v);
        }
    });
    cases(at, Field::Preimages, |p, v| {
        let m = &mut p.inner_mut().inputs[0].hash160_preimages;
        m.clear();
        if let Some(v) = n(v) {
            m.insert(hash160::Hash::hash(b"h"), v);
        }
    });
    cases(at, Field::Preimages, |p, v| {
        let m = &mut p.inner_mut().inputs[0].hash256_preimages;
        m.clear();
        if let Some(v) = n(v) {
            m.insert(sha256d::Hash::hash(b"d"), v);
        }
    });
}

fn leaf_tree(n: u8) -> bitcoin::taproot::TapTree {
    bitcoin::taproot::TaprootBuilder::new()
        .add_leaf(0, ScriptBuf::from_bytes(vec![0x51, n]))
        .expect("a leaf")
        .try_into()
        .expect("a complete tree")
}

#[test]
fn an_output_tap_tree_added_changed_and_removed_is_a_finding() {
    cases(Where::Output(0), Field::TapTree, |p, v| {
        p.inner_mut().outputs[0].tap_tree = v.map(leaf_tree);
    });
}

#[test]
fn an_output_taproot_key_origins_added_changed_and_removed_is_a_finding() {
    cases(Where::Output(0), Field::TaprootKeyOrigins, |p, v| {
        let key = SecretKey::from_slice(&[9; 32]).unwrap();
        let (x, _) = key.public_key(&Secp256k1::new()).x_only_public_key();
        let m = &mut p.inner_mut().outputs[0].tap_key_origins;
        m.clear();
        if let Some(n) = v {
            m.insert(x, (vec![], (Fingerprint::from([n; 4]), Default::default())));
        }
    });
}

#[test]
fn two_scripts_that_differ_only_after_a_truncated_push_are_changed() {
    // 0x4c 0x05 says five bytes follow; two do, and `ScriptBuf`'s Debug
    // stops printing at the push.
    let (a, b) = (
        ScriptBuf::from_bytes(vec![0x4c, 0x05, 0x01, 0x02]),
        ScriptBuf::from_bytes(vec![0x4c, 0x05, 0x01, 0x03]),
    );
    let make = |base: Psbt, script: &ScriptBuf| {
        let mut p = base;
        let input = &mut p.inner_mut().inputs[0];
        input.redeem_script = Some(script.clone());
        input.witness_script = Some(script.clone());
        p.inner_mut().outputs[0].witness_script = Some(script.clone());
        p.inner_mut().outputs[0].redeem_script = Some(script.clone());
        p
    };
    let report = run_with(&make(core_sent(), &a), &make(core_signed(), &b), &[]);
    let changed = |at, f| {
        report
            .findings
            .iter()
            .any(|x| x.at == at && x.kind == Kind::FieldChanged(FieldName::Compared(f)))
    };
    assert!(changed(Where::Input(0), Field::RedeemScript));
    assert!(changed(Where::Input(0), Field::WitnessScript));
    assert!(changed(Where::Output(0), Field::WitnessScript));
    assert!(changed(Where::Output(0), Field::RedeemScript));

    // The same in a previous transaction's output, and a taproot leaf.
    let make = |base: Psbt, script: &ScriptBuf| {
        let mut p = base;
        let input = &mut p.inner_mut().inputs[0];
        let mut tx = input
            .non_witness_utxo
            .clone()
            .unwrap_or_else(|| funding_tx(ScriptBuf::new(), 1_000));
        tx.output.push(TxOut {
            value: Amount::from_sat(1),
            script_pubkey: script.clone(),
        });
        input.non_witness_utxo = Some(tx);
        p
    };
    let report = run_with(&make(core_sent(), &a), &make(core_signed(), &b), &[]);
    assert!(report.findings.iter().any(|x| {
        x.at == Where::Input(0)
            && x.kind == Kind::FieldChanged(FieldName::Compared(Field::PreviousTransaction))
    }));
}

#[test]
fn a_taproot_leaf_script_that_differs_after_a_truncated_push_is_changed() {
    use bitcoin::taproot::{LeafVersion, TaprootBuilder};
    let secp = Secp256k1::new();
    let internal = SecretKey::from_slice(&[9; 32])
        .unwrap()
        .public_key(&secp)
        .x_only_public_key()
        .0;
    let leaf = |n: u8| {
        let script = ScriptBuf::from_bytes(vec![0x4c, 0x05, 0x01, n]);
        let info = TaprootBuilder::new()
            .add_leaf(0, script.clone())
            .unwrap()
            .finalize(&secp, internal)
            .unwrap();
        let block = info
            .control_block(&(script.clone(), LeafVersion::TapScript))
            .unwrap();
        (block, (script, LeafVersion::TapScript))
    };
    let make = |base: Psbt, n: u8| {
        let mut p = base;
        let (block, value) = leaf(n);
        let m = &mut p.inner_mut().inputs[0].tap_scripts;
        m.clear();
        m.insert(block, value);
        p
    };
    let report = run_with(&make(core_sent(), 2), &make(core_signed(), 3), &[]);
    assert!(
        report
            .findings
            .iter()
            .any(|x| { x.kind == Kind::FieldChanged(FieldName::Compared(Field::LeafScripts)) })
    );
}

/// A version 2 file of `psbt`, with the signatures of `signed` (a version
/// 0 PSBT of the same transaction) carried across where it is given.
fn v2_of(psbt: &Psbt, signed: Option<&Psbt>) -> PsbtV2 {
    let c = Compute::unchecked();
    let mut file = PsbtV2::from_v0(&c, psbt);
    if let Some(signed) = signed {
        file.merge_signed(signed).expect("the same transaction");
    }
    file
}

/// The check on two version 2 files, each read as the person has it.
fn run_v2(sent: &PsbtV2, returned: &PsbtV2) -> Report {
    let c = Compute::unchecked();
    run_files(
        &sent.to_v0(&c).expect("bridges"),
        &sent.to_bytes(),
        &returned.to_v0(&c).expect("bridges"),
        &returned.to_bytes(),
        &[],
    )
}

fn core_v2_pair() -> (PsbtV2, PsbtV2) {
    (
        v2_of(&core_sent(), None),
        v2_of(&core_sent(), Some(&core_signed())),
    )
}

#[test]
fn the_psbt_version_changed_either_way_is_changed() {
    let c = Compute::unchecked();
    let (v2_sent, v2_returned) = core_v2_pair();
    let v0 = |p: &Psbt| p.to_bytes();
    // 0 to 2.
    let report = run_files(
        &core_sent(),
        &v0(&core_sent()),
        &v2_returned.to_v0(&c).unwrap(),
        &v2_returned.to_bytes(),
        &[],
    );
    assert_one(&report, "M2", Level::Danger, Where::Transaction);
    assert_eq!(
        report.findings[0].kind,
        Kind::FieldChanged(FieldName::Compared(Field::PsbtVersion))
    );
    // 2 to 0.
    let report = run_files(
        &v2_sent.to_v0(&c).unwrap(),
        &v2_sent.to_bytes(),
        &core_signed(),
        &v0(&core_signed()),
        &[],
    );
    assert_one(&report, "M2", Level::Danger, Where::Transaction);
}

fn with_explicit_zero(psbt: &Psbt) -> (Psbt, Vec<u8>) {
    let mut bytes = psbt.to_bytes();
    bytes.splice(5..5, [0x01, 0xfb, 0x04, 0, 0, 0, 0]);
    let again = Psbt::parse_bytes(&Compute::unchecked(), &bytes).expect("it parses");
    (again, bytes)
}

#[test]
fn an_explicit_version_of_zero_added_or_removed_is_a_finding() {
    let (plain_sent, plain_signed) = (core_sent(), core_signed());
    let (zero_sent, zero_sent_bytes) = with_explicit_zero(&plain_sent);
    let (zero_signed, zero_signed_bytes) = with_explicit_zero(&plain_signed);
    let added = run_files(
        &plain_sent,
        &plain_sent.to_bytes(),
        &zero_signed,
        &zero_signed_bytes,
        &[],
    );
    assert_one(&added, "M1", Level::Danger, Where::Transaction);
    let removed = run_files(
        &zero_sent,
        &zero_sent_bytes,
        &plain_signed,
        &plain_signed.to_bytes(),
        &[],
    );
    assert_one(&removed, "M3", Level::Caution, Where::Transaction);
    let both = run_files(
        &zero_sent,
        &zero_sent_bytes,
        &zero_signed,
        &zero_signed_bytes,
        &[],
    );
    assert_clean(&both, 0, false);
}

#[test]
fn a_conforming_version_2_pair_is_clean() {
    let (sent, returned) = core_v2_pair();
    let report = run_v2(&sent, &returned);
    assert_clean(&report, 0, false);
    assert_eq!(report.facts.added.len(), 1);
}

/// What an unsigned version 2 pair gives for `edit` applied to Returned
/// after `from` is applied to both. Input 0 requires a height locktime in
/// both, so the fallback locktime is not the transaction's. The findings
/// are those of the edit: S9 (nothing signed) is left out.
fn v2_edit(edit: impl Fn(&mut PsbtV2), from: impl Fn(&mut PsbtV2)) -> Report {
    let mut sent = v2_of(&core_sent(), None);
    sent.inputs[0].set(vec![0x12], 7u32.to_le_bytes().to_vec());
    let mut returned = sent.clone();
    from(&mut sent);
    from(&mut returned);
    edit(&mut returned);
    let mut report = run_v2(&sent, &returned);
    report.findings.retain(|f| f.kind != Kind::NoSignatureAdded);
    report
}

/// The one finding an edit gave, by id, level and place.
fn assert_edit(report: &Report, id_: &'static str, level: Level, at: Where) {
    assert_eq!(found(report), [(id_, level, at)]);
}

/// `edit` applied to one map of a version 2 file.
fn in_map(at: Where, p: &mut PsbtV2, edit: impl FnOnce(&mut faraday_psbt::v2::Map)) {
    edit(match at {
        Where::Transaction => &mut p.global,
        Where::Input(i) => &mut p.inputs[i],
        Where::Output(i) => &mut p.outputs[i],
    });
}

#[test]
fn every_version_2_only_pair_changed_added_or_removed_is_a_finding() {
    // (place, key, a value, another value): the fallback locktime, which
    // the transaction does not use where an input requires one, and
    // BIP-375's pairs, which version 0 has no name for.
    let global = Where::Transaction;
    type Place = (Where, Vec<u8>, Vec<u8>, Vec<u8>);
    let places: [Place; 4] = [
        (global, vec![0x03], vec![5, 0, 0, 0], vec![6, 0, 0, 0]),
        (global, vec![0x07, 1, 2], vec![1; 4], vec![2; 4]),
        (Where::Input(0), vec![0x1d, 1], vec![1; 4], vec![2; 4]),
        (Where::Output(0), vec![0x0a, 1], vec![1; 4], vec![2; 4]),
    ];
    let name = FieldName::Compared(Field::V2Fields);
    for (at, key, a, b) in places {
        let set = |value: Option<&Vec<u8>>| {
            let (key, value) = (key.clone(), value.cloned());
            move |p: &mut PsbtV2| {
                in_map(at, p, |m| match &value {
                    Some(v) => m.set(key.clone(), v.clone()),
                    None => m.remove(&key),
                });
            }
        };
        for (report, want, level, what) in [
            (
                v2_edit(set(Some(&a)), set(None)),
                "M1",
                Level::Danger,
                "added",
            ),
            (
                v2_edit(set(Some(&b)), set(Some(&a))),
                "M2",
                Level::Danger,
                "changed",
            ),
            (
                v2_edit(set(None), set(Some(&a))),
                "M3",
                Level::Caution,
                "removed",
            ),
        ] {
            assert_edit(&report, want, level, at);
            let kind = match want {
                "M1" => Kind::FieldAdded(name),
                "M2" => Kind::FieldChanged(name),
                _ => Kind::FieldRemoved(name),
            };
            assert_eq!(report.findings[0].kind, kind, "{at:?} {key:?} {what}");
        }
    }
}

#[test]
fn a_required_time_locktime_that_does_not_change_the_transaction_is_changed() {
    // Input 0 requires a height and a time; every input gives a height, so
    // the transaction takes the height and a change of the time leaves
    // its locktime alone.
    let set = |time: u32| {
        move |p: &mut PsbtV2| {
            p.inputs[0].set(vec![0x12], 7u32.to_le_bytes().to_vec());
            p.inputs[0].set(vec![0x11], time.to_le_bytes().to_vec());
        }
    };
    let report = v2_edit(set(600_000_001), set(600_000_000));
    assert_edit(&report, "M2", Level::Danger, Where::Input(0));
    assert_eq!(
        report.findings[0].kind,
        Kind::FieldChanged(FieldName::Compared(Field::V2Fields))
    );
}

/// Flags findings only: other findings (the sighash asked, the key) are
/// not what these cases are about.
fn flag_findings(report: &Report) -> Vec<&'static str> {
    report
        .findings
        .iter()
        .filter(|f| {
            matches!(
                f.kind,
                Kind::FieldAdded(FieldName::Compared(Field::ModifiableFlags))
                    | Kind::FieldChanged(FieldName::Compared(Field::ModifiableFlags))
                    | Kind::FieldRemoved(FieldName::Compared(Field::ModifiableFlags))
            )
        })
        .map(|f| id(&f.kind))
        .collect()
}

/// Sent (with `flags` where given) and Returned, signed with `kind`; the
/// flags Returned holds are `returned_flags` where given.
fn flags_case(
    kind: Option<EcdsaSighashType>,
    flags: Option<u8>,
    returned_flags: Option<u8>,
) -> Report {
    let base = demo();
    let mut sent = v2_of(&base, None);
    let signed_psbt = kind.map(|k| signed(&base, 0, &abandon(), k, Nonce::Rfc6979));
    let mut returned = v2_of(&base, signed_psbt.as_ref());
    if let Some(f) = flags {
        sent.global.set(vec![0x06], vec![f]);
    }
    match returned_flags {
        Some(f) => returned.global.set(vec![0x06], vec![f]),
        None => returned.global.remove(&[0x06]),
    }
    run_v2(&sent, &returned)
}

#[test]
fn the_modifiable_flags_follow_the_rule_for_a_signer() {
    use EcdsaSighashType as T;
    let ok = |kind, from: Option<u8>, to: Option<u8>| {
        let r = flags_case(Some(kind), from, to);
        assert_eq!(
            flag_findings(&r),
            Vec::<&str>::new(),
            "{kind:?} {from:?} {to:?}"
        );
    };
    // ALL clears bits 0 and 1.
    ok(T::All, Some(0b011), Some(0b000));
    // An undefined bit stays.
    ok(T::All, Some(0b1011), Some(0b1000));
    // ANYONECANPAY|ALL keeps bit 0, clears bit 1.
    ok(T::AllPlusAnyoneCanPay, Some(0b011), Some(0b001));
    // NONE keeps bit 1, clears bit 0.
    ok(T::None, Some(0b011), Some(0b010));
    // SINGLE clears bits 0 and 1 and sets bit 2.
    ok(T::Single, Some(0b011), Some(0b100));
    ok(T::Single, Some(0b000), Some(0b100));
    // No field on Sent: SINGLE gives the result worked out from 0.
    ok(T::Single, None, Some(0b100));
    // No field on either.
    ok(T::All, None, None);
    // SINGLE|ANYONECANPAY keeps bit 0, clears bit 1 and sets bit 2.
    ok(T::SinglePlusAnyoneCanPay, Some(0b011), Some(0b101));
}

#[test]
fn modifiable_flags_that_break_the_rule_are_findings() {
    use EcdsaSighashType as T;
    let got = |kind: Option<T>, from, to| flag_findings(&flags_case(kind, from, to));
    // Bits left as they were though ALL clears them.
    assert_eq!(got(Some(T::All), Some(0b011), Some(0b011)), ["M2"]);
    // An undefined bit changed.
    assert_eq!(got(Some(T::All), Some(0b1011), Some(0b0000)), ["M2"]);
    assert_eq!(got(Some(T::All), Some(0b0011), Some(0b1000)), ["M2"]);
    // The field gone.
    assert_eq!(got(Some(T::All), Some(0b011), None), ["M3"]);
    // Added with a value the rule does not give.
    assert_eq!(got(Some(T::All), None, Some(0b011)), ["M1"]);
    assert_eq!(got(Some(T::Single), None, Some(0b110)), ["M1"]);
    // No signature added: the byte stays, and a changed one is a finding.
    assert_eq!(got(None, Some(0b011), Some(0b011)), Vec::<&str>::new());
    assert_eq!(got(None, Some(0b011), Some(0b001)), ["M2"]);
    assert_eq!(got(None, None, Some(0b100)), ["M1"]);
}

#[test]
fn bytes_past_the_last_map_of_a_version_2_file_are_unaccounted() {
    let (sent, returned) = core_v2_pair();
    let c = Compute::unchecked();
    let mut bytes = returned.to_bytes();
    bytes.extend_from_slice(&[0xaa; 5]);
    // The parser refuses them, so the caller never reads such a file; the
    // check, handed the bytes, says E1 and not clean.
    assert!(PsbtV2::parse_bytes(&c, &bytes).is_err());
    let report = run_files(
        &sent.to_v0(&c).unwrap(),
        &sent.to_bytes(),
        &returned.to_v0(&c).unwrap(),
        &bytes,
        &[],
    );
    assert_eq!(
        report
            .findings
            .iter()
            .filter(|f| f.kind == Kind::UnaccountedBytes { count: 5 })
            .count(),
        1,
        "{:?}",
        found(&report)
    );
}

#[test]
fn a_version_2_pair_written_longer_than_it_needs_is_unaccounted() {
    let (sent, returned) = core_v2_pair();
    let c = Compute::unchecked();
    // The global map's first key (one byte, 0x02) written as 0xfd 0x01 0x00.
    let mut bytes = returned.to_bytes();
    assert_eq!(bytes[5], 0x01);
    bytes.splice(5..6, [0xfd, 0x01, 0x00]);
    assert!(
        PsbtV2::parse_bytes(&c, &bytes).is_ok(),
        "the parser takes it"
    );
    let report = run_files(
        &sent.to_v0(&c).unwrap(),
        &sent.to_bytes(),
        &returned.to_v0(&c).unwrap(),
        &bytes,
        &[],
    );
    assert!(
        report
            .findings
            .iter()
            .any(|f| f.kind == Kind::UnaccountedBytes { count: 2 }),
        "{:?}",
        found(&report)
    );
}

#[test]
fn a_file_that_cannot_be_read_is_never_clean() {
    let (sent, returned) = core_v2_pair();
    let c = Compute::unchecked();
    let mut bytes = returned.to_bytes();
    bytes.truncate(bytes.len() - 3);
    let report = run_files(
        &sent.to_v0(&c).unwrap(),
        &sent.to_bytes(),
        &returned.to_v0(&c).unwrap(),
        &bytes,
        &[],
    );
    assert!(!matches!(report.verdict, Verdict::Clean { .. }));
}

// ---- The third review: repeated nonces, forms of a key, flags, finalized MuSig2

/// Findings of one id, by level and place.
fn has(report: &Report, id_: &'static str, level: Level, at: Where) -> bool {
    found(report).contains(&(id_, level, at))
}

/// The nonce point `r` of an ECDSA signature made with the nonce of byte `k`.
fn r_of(psbt: &Psbt, k: u8) -> [u8; 32] {
    let (_, sig) = signature(psbt, 0, &abandon(), EcdsaSighashType::All, Nonce::K(k));
    sig.signature.serialize_compact()[..32].try_into().unwrap()
}

/// A BIP-340 signature of the bytes `r` and `s`, which is as much as the
/// nonce check reads of it.
fn schnorr_of(r: [u8; 32], s: u8) -> bitcoin::taproot::Signature {
    let mut bytes = r.to_vec();
    bytes.extend_from_slice(&[s; 32]);
    bitcoin::taproot::Signature::from_slice(&bytes).expect("64 bytes")
}

fn leaf_of(n: u8) -> TapLeafHash {
    <TapLeafHash as Hash>::from_byte_array([n; 32])
}

fn assert_s5(report: &Report) {
    assert!(
        has(report, "S5", Level::Blocked, Where::Transaction),
        "{:?}",
        found(report)
    );
    assert!(matches!(
        report.verdict,
        Verdict::Findings {
            worst: Level::Blocked,
            ..
        }
    ));
}

/// Two inputs of one wallet under the keys at index 0 and 1 of one chain.
fn sibling_inputs() -> Psbt {
    let c = Compute::unchecked();
    let mut psbt = two_inputs_one_output();
    let (fp, path) = psbt.inner().inputs[0]
        .bip32_derivation
        .values()
        .next()
        .cloned()
        .unwrap();
    let mut steps: Vec<ChildNumber> = path.into_iter().copied().collect();
    let last = steps.len() - 1;
    let ChildNumber::Normal { index } = steps[last] else {
        panic!("a non-hardened last step");
    };
    steps[last] = ChildNumber::Normal { index: index + 1 };
    let path = DerivationPath::from(steps);
    let pk = abandon().derive(&c, &path).to_xpub(&c).public_key;
    let script = ScriptBuf::new_p2wpkh(
        &bitcoin::PublicKey::new(pk)
            .wpubkey_hash()
            .expect("compressed"),
    );
    let previous = funding_tx(script.clone(), 50_000);
    let inner = psbt.inner_mut();
    inner.unsigned_tx.input[1].previous_output = OutPoint::new(previous.compute_txid(), 0);
    let input = &mut inner.inputs[1];
    input.bip32_derivation.clear();
    input.bip32_derivation.insert(pk, (fp, path));
    input.witness_utxo = Some(previous.output[0].clone());
    input.non_witness_utxo = Some(previous);
    reparse(&psbt)
}

#[test]
fn one_nonce_under_two_sibling_keys_is_blocked() {
    let sent = sibling_inputs();
    let mut returned = sent.clone();
    for i in 0..2 {
        let (pk, sig) = signature(&sent, i, &abandon(), EcdsaSighashType::All, Nonce::K(5));
        returned.inner_mut().inputs[i].partial_sigs.insert(pk, sig);
    }
    let r0 = returned.inner().inputs[0]
        .partial_sigs
        .values()
        .next()
        .unwrap();
    let r1 = returned.inner().inputs[1]
        .partial_sigs
        .values()
        .next()
        .unwrap();
    assert_eq!(
        r0.signature.serialize_compact()[..32],
        r1.signature.serialize_compact()[..32],
        "one nonce"
    );
    let report = run_with(&sent, &reparse(&returned), &[]);
    assert_s5(&report);
    let Kind::RepeatedNonce { key, inputs } = &report.findings[0].kind else {
        panic!("S5");
    };
    assert_eq!(inputs, &[0, 1]);
    assert_eq!(key.split(", ").count(), 2, "both keys are named: {key}");
}

#[test]
fn one_nonce_under_an_ecdsa_key_and_a_tapscript_key_of_one_point_is_blocked() {
    let sent = two_inputs_one_output();
    let (pk, sig) = signature(&sent, 0, &abandon(), EcdsaSighashType::All, Nonce::K(5));
    let r = sig.signature.serialize_compact()[..32].try_into().unwrap();
    let mut returned = sent.clone();
    returned.inner_mut().inputs[0].partial_sigs.insert(pk, sig);
    let x = pk.inner.x_only_public_key().0;
    returned.inner_mut().inputs[1]
        .tap_script_sigs
        .insert((x, leaf_of(1)), schnorr_of(r, 3));
    assert_s5(&run_with(&sent, &reparse(&returned), &[]));
}

#[test]
fn one_nonce_under_a_tweaked_and_an_internal_key_is_blocked() {
    let m = abandon();
    let mut b = Build::new(ScriptType::Taproot, NET);
    b.recipients.push((ScriptBuf::new_op_return([1]), 99_000));
    let sent = single_sig(&m, &b);
    let internal = sent.inner().inputs[0]
        .tap_internal_key
        .expect("internal key");
    let r = r_of(&demo(), 5);
    let mut returned = sent.clone();
    let input = &mut returned.inner_mut().inputs[0];
    input.tap_key_sig = Some(schnorr_of(r, 2));
    input
        .tap_script_sigs
        .insert((internal, leaf_of(1)), schnorr_of(r, 3));
    assert_s5(&run_with(&sent, &reparse(&returned), &[]));
}

#[test]
fn the_same_signature_in_two_fields_of_one_input_is_not_a_reuse() {
    let sent = demo();
    let mut returned = finalized(&sent, &abandon(), Nonce::Rfc6979);
    // A finalizer that left the partial signature beside the witness.
    let (pk, sig) = signature(&sent, 0, &abandon(), EcdsaSighashType::All, Nonce::Rfc6979);
    let input = &mut returned.inner_mut().inputs[0];
    input.partial_sigs.insert(pk, sig);
    let witness = input.final_script_witness.as_ref().unwrap();
    assert_eq!(witness.nth(0).unwrap(), sig.to_vec(), "one signature");
    let returned = reparse(&returned);
    let report = run_with(&sent, &returned, &[]);
    assert!(
        !found(&report).iter().any(|f| f.0 == "S5"),
        "{:?}",
        found(&report)
    );
}

#[test]
fn a_taproot_key_signature_that_reads_under_a_new_key_when_the_utxo_is_added_is_one_signature() {
    let mut b = Build::new(ScriptType::Taproot, NET);
    b.recipients.push((ScriptBuf::new_op_return([1]), 99_000));
    let full = single_sig(&abandon(), &b);
    let r = r_of(&demo(), 5);
    let mut sent = full.clone();
    let input = &mut sent.inner_mut().inputs[0];
    input.witness_utxo = None;
    input.non_witness_utxo = None;
    input.tap_key_sig = Some(schnorr_of(r, 2));
    let sent = reparse(&sent);
    let mut returned = sent.clone();
    returned.inner_mut().inputs[0].witness_utxo = full.inner().inputs[0].witness_utxo.clone();
    let report = run_with(&sent, &reparse(&returned), &[]);
    assert!(
        !found(&report).iter().any(|f| f.0 == "S5"),
        "{:?}",
        found(&report)
    );
}

/// `psbt` with input 0 copied to `count` inputs in all, each spending
/// another output of the same transaction.
fn more_inputs(psbt: &Psbt, count: usize) -> Psbt {
    let mut out = psbt.clone();
    let inner = out.inner_mut();
    for n in 1..count {
        let mut spend = inner.unsigned_tx.input[0].clone();
        spend.previous_output.vout += n as u32;
        inner.unsigned_tx.input.push(spend);
        let copy = inner.inputs[0].clone();
        inner.inputs.push(copy);
    }
    reparse(&out)
}

/// Every MuSig2 public nonce of input `i`, with its pair's key.
fn nonce_pairs(psbt: &Psbt, i: usize) -> Vec<(raw::Key, Vec<u8>)> {
    psbt.inner().inputs[i]
        .unknown
        .iter()
        .filter(|(k, _)| k.type_value == 0x1b)
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect()
}

fn musig_three_inputs() -> (Psbt, Psbt) {
    (
        more_inputs(&read("wallet-musig-core-first.psbt"), 3),
        more_inputs(&read("wallet-musig-core-first-signed.psbt"), 3),
    )
}

#[test]
fn a_musig_nonce_on_three_inputs_is_blocked() {
    let policy = musig_policy();
    let (sent, returned) = musig_three_inputs();
    assert_eq!(nonce_pairs(&returned, 0), nonce_pairs(&returned, 2));
    let report = run_with(&sent, &returned, std::slice::from_ref(&policy));
    assert_s5(&report);
    let s5 = report
        .findings
        .iter()
        .find_map(|f| match &f.kind {
            Kind::RepeatedNonce { inputs, .. } => Some(inputs.clone()),
            _ => None,
        })
        .unwrap();
    assert_eq!(s5, [0, 1, 2]);
}

/// Two inputs: Sent has Core's nonce pair on input 0 only, and Returned
/// has `second` made of each pair's points on input 1.
fn musig_nonce_on_a_second_input(second: impl Fn(&mut [u8])) -> (Psbt, Psbt) {
    let mut sent = more_inputs(&read("wallet-musig-core-first.psbt"), 2);
    sent.inner_mut().inputs[1]
        .unknown
        .retain(|k, _| k.type_value != 0x1b);
    let mut returned = more_inputs(&read("wallet-musig-core-first-signed.psbt"), 2);
    for value in returned.inner_mut().inputs[1]
        .unknown
        .iter_mut()
        .filter(|(k, _)| k.type_value == 0x1b)
        .map(|(_, v)| v)
    {
        assert_eq!(value.len(), 66);
        second(value);
    }
    (reparse(&sent), reparse(&returned))
}

#[test]
fn one_point_of_a_musig_nonce_on_two_inputs_is_blocked() {
    let policy = musig_policy();
    // Its first point stays; the second is another point.
    let other = PublicKey::from_secret_key(
        &Secp256k1::new(),
        &SecretKey::from_slice(&[11; 32]).unwrap(),
    );
    let (sent, returned) = musig_nonce_on_a_second_input(|v| {
        v[33..].copy_from_slice(&other.serialize());
    });
    assert_eq!(
        faraday_psbt::verify::repeated_pub_nonces(&[&sent]),
        vec![],
        "Sent alone repeats nothing"
    );
    let report = run_with(&sent, &returned, std::slice::from_ref(&policy));
    assert_s5(&report);
}

#[test]
fn the_negated_points_of_a_musig_nonce_on_another_input_are_blocked() {
    let policy = musig_policy();
    let (sent, returned) = musig_nonce_on_a_second_input(|v| {
        for point in [0, 33] {
            let negated = PublicKey::from_slice(&v[point..point + 33])
                .unwrap()
                .negate(&Secp256k1::new());
            v[point..point + 33].copy_from_slice(&negated.serialize());
        }
    });
    assert_eq!(faraday_psbt::verify::repeated_pub_nonces(&[&sent]), vec![]);
    assert_s5(&run_with(&sent, &returned, std::slice::from_ref(&policy)));
}

#[test]
fn a_musig_pair_whose_key_does_not_read_is_named_by_its_first_bytes() {
    let mut psbt = more_inputs(&read("wallet-musig-core-first.psbt"), 2);
    for input in &mut psbt.inner_mut().inputs {
        let (key, value) = nonce_pairs_of(input).remove(0);
        input.unknown.remove(&key);
        let mut bad = key.clone();
        // An x coordinate beyond the field's order is no point.
        bad.key[..33].copy_from_slice(&[&[2u8][..], &[0xff; 32]].concat());
        input.unknown.insert(bad, value);
    }
    let psbt = reparse(&psbt);
    let reused = faraday_psbt::verify::repeated_pub_nonces(&[&psbt]);
    assert_eq!(reused.len(), 2, "both points repeat");
    assert_eq!(reused[0].name(), "02ffffff");
}

fn nonce_pairs_of(input: &bitcoin::psbt::Input) -> Vec<(raw::Key, Vec<u8>)> {
    input
        .unknown
        .iter()
        .filter(|(k, _)| k.type_value == 0x1b)
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect()
}

#[test]
fn one_musig_nonce_under_two_participants_of_one_input_is_blocked() {
    let policy = musig_policy();
    let sent = read("wallet-musig-core-first.psbt");
    let mut returned = read("wallet-musig-core-first-signed.psbt");
    let (key, value) = nonce_pairs(&returned, 0).remove(0);
    let mut other = key.clone();
    let second =
        PublicKey::from_secret_key(&Secp256k1::new(), &SecretKey::from_slice(&[9; 32]).unwrap());
    other.key[..33].copy_from_slice(&second.serialize());
    returned.inner_mut().inputs[0].unknown.insert(other, value);
    let report = run_with(&sent, &reparse(&returned), std::slice::from_ref(&policy));
    assert_s5(&report);
}

#[test]
fn a_musig_nonce_unchanged_between_sent_and_returned_is_one_place() {
    let policy = musig_policy();
    let signed = read("wallet-musig-core-first-signed.psbt");
    let report = run_with(&signed, &signed, std::slice::from_ref(&policy));
    assert!(
        !found(&report).iter().any(|f| f.0 == "S5"),
        "{:?}",
        found(&report)
    );
}

/// Sent signed with the compressed key and Returned with `put` applied to
/// the same signature's place.
fn forms(
    keep: bool,
    put: impl Fn(
        &mut std::collections::BTreeMap<bitcoin::PublicKey, ecdsa::Signature>,
        bitcoin::PublicKey,
        ecdsa::Signature,
    ),
) -> Report {
    let sent = signed(
        &demo(),
        0,
        &abandon(),
        EcdsaSighashType::All,
        Nonce::Rfc6979,
    );
    let (pk, sig) = signature(
        &demo(),
        0,
        &abandon(),
        EcdsaSighashType::All,
        Nonce::Rfc6979,
    );
    let mut returned = sent.clone();
    let sigs = &mut returned.inner_mut().inputs[0].partial_sigs;
    if !keep {
        sigs.clear();
    }
    put(sigs, pk, sig);
    run_with(&sent, &reparse(&returned), &[])
}

#[test]
fn the_same_signature_moved_to_the_uncompressed_form_is_s3_and_s7() {
    let report = forms(false, |m, pk, sig| {
        m.insert(uncompressed(pk), sig);
    });
    assert!(
        has(&report, "S3", Level::Danger, Where::Input(0)),
        "{:?}",
        found(&report)
    );
    assert!(
        has(&report, "S7", Level::Caution, Where::Input(0)),
        "{:?}",
        found(&report)
    );
}

#[test]
fn the_same_signature_under_both_forms_is_s3() {
    let report = forms(true, |m, pk, sig| {
        m.insert(uncompressed(pk), sig);
    });
    assert!(
        has(&report, "S3", Level::Danger, Where::Input(0)),
        "{:?}",
        found(&report)
    );
    assert!(!matches!(report.verdict, Verdict::Clean { .. }));
}

/// A sent PSBT whose witness script holds the uncompressed form of the
/// key at origin, and a signature of that key over it.
fn uncompressed_script_sent() -> (Psbt, bitcoin::PublicKey, ecdsa::Signature) {
    let (pk, _) = signature(
        &demo(),
        0,
        &abandon(),
        EcdsaSighashType::All,
        Nonce::Rfc6979,
    );
    let mut script = vec![0x41];
    script.extend_from_slice(&pk.inner.serialize_uncompressed());
    script.push(0xac);
    let ws = ScriptBuf::from_bytes(script);
    let mut sent = demo();
    let input = &mut sent.inner_mut().inputs[0];
    let value = input.witness_utxo.as_ref().unwrap().value;
    input.witness_utxo = Some(TxOut {
        value,
        script_pubkey: ScriptBuf::new_p2wsh(&ws.wscript_hash()),
    });
    input.non_witness_utxo = None;
    input.witness_script = Some(ws);
    let sent = reparse(&sent);
    let (pk, sig) = signature(&sent, 0, &abandon(), EcdsaSighashType::All, Nonce::Rfc6979);
    (sent, pk, sig)
}

#[test]
fn a_signature_added_under_the_uncompressed_key_a_script_holds_is_clean() {
    let (sent, pk, sig) = uncompressed_script_sent();
    let mut returned = sent.clone();
    returned.inner_mut().inputs[0]
        .partial_sigs
        .insert(uncompressed(pk), sig);
    let report = run_with(&sent, &reparse(&returned), &[]);
    assert_eq!(report.findings, vec![], "findings");
    assert!(matches!(report.verdict, Verdict::Clean { .. }));
    assert_eq!(report.facts.added.len(), 1);
}

#[test]
fn a_signature_under_the_compressed_key_where_the_script_holds_the_uncompressed_is_s3() {
    let (sent, pk, sig) = uncompressed_script_sent();
    // Added alone.
    let mut returned = sent.clone();
    returned.inner_mut().inputs[0].partial_sigs.insert(pk, sig);
    let report = run_with(&sent, &reparse(&returned), &[]);
    assert!(
        has(&report, "S3", Level::Danger, Where::Input(0)),
        "{:?}",
        found(&report)
    );
    // Added beside the uncompressed form Sent already carries.
    let mut carried = sent.clone();
    carried.inner_mut().inputs[0]
        .partial_sigs
        .insert(uncompressed(pk), sig);
    let mut returned = carried.clone();
    returned.inner_mut().inputs[0].partial_sigs.insert(pk, sig);
    let report = run_with(&carried, &reparse(&returned), &[]);
    assert!(
        has(&report, "S3", Level::Danger, Where::Input(0)),
        "{:?}",
        found(&report)
    );
    assert!(!matches!(report.verdict, Verdict::Clean { .. }));
}

#[test]
fn an_unloaded_key_path_signature_does_not_hide_a_leaf_signature_of_a_loaded_key() {
    let c = Compute::unchecked();
    let sent = read("wallet-tapmulti-first.psbt");
    let signed_ = read("wallet-tapmulti-signed.psbt");
    // The leaf signature's key is one of these masters.
    let masters = [abandon(), master(ZOO, NET), master(LEGAL, NET)];
    let view = nonce_view(&sent, &signed_);
    let (loaded, own) = masters
        .iter()
        .find_map(|m| {
            deterministic_all(&c, &view, &[m])
                .into_iter()
                .next()
                .map(|d| (m, d))
        })
        .expect("a master of a signature");
    let Some(leaf) = own.leaf else {
        panic!("a leaf signature");
    };
    let faraday_psbt::verify::SigKey::Schnorr(x) = own.key else {
        panic!("an x-only key");
    };
    // Its nonce is not the rule's: one byte of s changed.
    let mut returned = signed_.clone();
    let internal = SecretKey::from_slice(&[9; 32])
        .unwrap()
        .public_key(&Secp256k1::new())
        .x_only_public_key()
        .0;
    let input = &mut returned.inner_mut().inputs[0];
    let key = (x, leaf);
    let sig = input.tap_script_sigs[&key];
    let mut bytes = sig.signature.serialize().to_vec();
    bytes[63] ^= 1;
    input.tap_script_sigs.insert(
        key,
        bitcoin::taproot::Signature::from_slice(&bytes).unwrap(),
    );
    input.tap_key_sig = Some(schnorr_of([1; 32], 2));
    input.tap_internal_key = Some(internal);
    let unloaded = Fingerprint::from([0xde, 0xad, 0xbe, 0xef]);
    input.tap_key_origins.insert(
        internal,
        (vec![], (unloaded, "m/86h/1h/0h/0/0".parse().unwrap())),
    );
    let mut sent = sent;
    let s_input = &mut sent.inner_mut().inputs[0];
    s_input.tap_internal_key = Some(internal);
    s_input.tap_key_origins.insert(
        internal,
        (vec![], (unloaded, "m/86h/1h/0h/0/0".parse().unwrap())),
    );
    let view = nonce_view(&sent, &returned);
    let found_ = deterministic_all(&c, &view, &[loaded]);
    assert!(!found_.is_empty(), "the leaf signature reaches a result");
    let report = run_with(&sent, &returned, &[]);
    let folded = with_nonces(report, &found_, &[]);
    assert!(
        has(&folded, "S6", Level::Danger, Where::Input(0)),
        "{:?}",
        found(&folded)
    );
    let loaded_fp = Fingerprint::from(loaded.fingerprint());
    let not_loaded = folded.facts.recomputed.clone().unwrap().not_loaded;
    assert!(
        !not_loaded.iter().any(|f| *f == loaded_fp.into()),
        "{not_loaded:?}"
    );
}

/// `signed` (a MuSig2 pair) with input 0 finalized the way a finalizer
/// leaves it: every pair of the types BIP-373 adds gone, a witness set.
fn musig_finalized(sent: &Psbt) -> Psbt {
    let mut out = sent.clone();
    let input = &mut out.inner_mut().inputs[0];
    input
        .unknown
        .retain(|k, _| !(0x1a..=0x1c).contains(&k.type_value));
    input.final_script_witness = Some(Witness::from_slice(&[[7u8; 64].to_vec()]));
    reparse(&out)
}

#[test]
fn musig_pairs_a_finalizer_clears_are_not_removed_on_a_finalized_input() {
    let policy = musig_policy();
    let sent = read("wallet-musig-core-first-signed.psbt");
    let report = run_with(
        &sent,
        &musig_finalized(&sent),
        std::slice::from_ref(&policy),
    );
    assert!(
        !report
            .findings
            .iter()
            .any(|f| matches!(f.kind, Kind::FieldRemoved(_))),
        "{:?}",
        found(&report)
    );
}

#[test]
fn a_finalized_musig_input_whose_pairs_the_finalizer_cleared_is_clean() {
    let policy = musig_policy();
    let sent = read("wallet-musig-device-first-signed.psbt");
    let mut returned = sent.clone();
    finalize(&mut returned).expect("finalizes");
    // The way Core's finalizer leaves it: the BIP-373 pairs gone.
    let input = &mut returned.inner_mut().inputs[0];
    assert!(input.final_script_witness.is_some());
    input
        .unknown
        .retain(|k, _| !(0x1a..=0x1c).contains(&k.type_value));
    let report = run_with(&sent, &reparse(&returned), std::slice::from_ref(&policy));
    assert!(
        matches!(report.verdict, Verdict::Clean { .. }),
        "{:?}",
        found(&report)
    );
}

#[test]
fn musig_pairs_removed_from_an_input_not_finalized_are_still_removed() {
    // The existing check of a taken-out nonce and partial signature.
    let policy = musig_policy();
    let sent = read("wallet-musig-core-first-signed.psbt");
    let mut returned = musig_finalized(&sent);
    returned.inner_mut().inputs[0].final_script_witness = None;
    let report = run_with(&sent, &reparse(&returned), std::slice::from_ref(&policy));
    let kinds: Vec<_> = report.findings.iter().map(|f| f.kind.clone()).collect();
    assert!(
        kinds.contains(&Kind::FieldRemoved(FieldName::MusigNonce)),
        "{kinds:?}"
    );
    assert!(
        kinds.contains(&Kind::FieldRemoved(FieldName::MusigPartialSig)),
        "{kinds:?}"
    );
}

#[test]
fn a_musig_pair_kept_on_a_finalized_input_is_compared() {
    let policy = musig_policy();
    let sent = read("wallet-musig-core-first-signed.psbt");
    let mut returned = musig_finalized(&sent);
    let (key, mut value) = nonce_pairs(&sent, 0).remove(0);
    value[0] ^= 1;
    returned.inner_mut().inputs[0].unknown.insert(key, value);
    let report = run_with(&sent, &reparse(&returned), std::slice::from_ref(&policy));
    let kinds: Vec<_> = report.findings.iter().map(|f| f.kind.clone()).collect();
    assert!(
        kinds.contains(&Kind::FieldChanged(FieldName::MusigNonce)),
        "{kinds:?}"
    );
}

#[test]
fn modifiable_flags_added_by_the_rule_with_a_zero_result_are_no_finding() {
    use EcdsaSighashType as T;
    // A signature added and the field the rule gives from 0, 00 included.
    assert_eq!(
        flag_findings(&flags_case(Some(T::All), None, Some(0))),
        Vec::<&str>::new()
    );
    // The field added with no signature added, whatever its value.
    assert_eq!(flag_findings(&flags_case(None, None, Some(0))), ["M1"]);
    assert_eq!(flag_findings(&flags_case(None, None, Some(0b100))), ["M1"]);
}

/// The flags findings of Sent (holding `from`) against Returned (holding
/// `to`) made from `base` and the signatures of `signed_`, with `wallets`
/// registered; and the whole report.
fn flags_between(
    base: &Psbt,
    signed_: &Psbt,
    wallets: &[WalletPolicy],
    from: u8,
    to: u8,
) -> (Vec<&'static str>, Report) {
    let c = Compute::unchecked();
    let mut sent = v2_of(base, None);
    let mut returned = v2_of(base, Some(signed_));
    sent.global.set(vec![0x06], vec![from]);
    returned.global.set(vec![0x06], vec![to]);
    let report = run_files(
        &sent.to_v0(&c).unwrap(),
        &sent.to_bytes(),
        &returned.to_v0(&c).unwrap(),
        &returned.to_bytes(),
        wallets,
    );
    (flag_findings(&report), report)
}

fn taproot_pair() -> (Psbt, Psbt) {
    let c = Compute::unchecked();
    let m = abandon();
    let mut b = Build::new(ScriptType::Taproot, NET);
    b.recipients.push((ScriptBuf::new_op_return([1]), 99_000));
    let sent = single_sig(&m, &b);
    let keys = [faraday_psbt::KeyRef::from_master(&c, &m, 0).expect("accounts")];
    let ctx = Context {
        network: NET,
        keys: &keys,
        wallets: &[],
        musig_session: None,
    };
    let mut signed_ = sent.clone();
    faraday_psbt::sign(
        &c,
        &mut signed_,
        &[&m],
        &[m.fingerprint()],
        &ctx,
        &[],
        faraday_psbt::Nonce::LowR,
        faraday_psbt::Aux::Deterministic,
        &mut None,
        Some(&faraday_crypto::Zeroizing::new([0u8; 32])),
    )
    .expect("signs");
    (sent, signed_)
}

#[test]
fn the_modifiable_flags_follow_the_rule_for_each_kind_of_signature() {
    // A taproot signature with no sighash byte (SIGHASH_DEFAULT) commits
    // like ALL: bits 0 and 1 cleared.
    let (sent, signed_) = taproot_pair();
    let (ids, report) = flags_between(&sent, &signed_, &[], 0b011, 0b000);
    assert_eq!(ids, Vec::<&str>::new());
    assert_eq!(report.facts.added.len(), 1);
    assert_eq!(flags_between(&sent, &signed_, &[], 0b011, 0b011).0, ["M2"]);

    // A MuSig2 partial signature states no sighash: the input's own.
    let policy = musig_policy();
    let first = read("wallet-musig-core-first.psbt");
    let both = read("wallet-musig-core-first-signed.psbt");
    let wallets = std::slice::from_ref(&policy);
    let (ids, report) = flags_between(&first, &both, wallets, 0b011, 0b000);
    assert_eq!(ids, Vec::<&str>::new());
    assert_eq!(report.facts.partial_sigs_added, 1);
    assert_eq!(
        flags_between(&first, &both, wallets, 0b011, 0b011).0,
        ["M2"]
    );

    // A signature that is only in final fields.
    let base = demo();
    let done = finalized(&base, &abandon(), Nonce::Rfc6979);
    let (ids, report) = flags_between(&base, &done, &[], 0b011, 0b000);
    assert_eq!(ids, Vec::<&str>::new());
    assert_eq!(
        report.facts.finalized,
        [0],
        "the input is final on Returned"
    );
    assert_eq!(flags_between(&base, &done, &[], 0b011, 0b011).0, ["M2"]);
}

#[test]
fn a_version_2_pair_with_its_flags_updated_by_the_rule_is_clean() {
    let (mut sent, mut returned) = core_v2_pair();
    sent.global.set(vec![0x06], vec![0b011]);
    // core-signed signs with ALL: bits 0 and 1 cleared.
    returned.global.set(vec![0x06], vec![0b000]);
    let report = run_v2(&sent, &returned);
    assert_clean(&report, 0, false);
    assert_eq!(report.facts.added.len(), 1);
}

#[test]
fn a_signature_that_cannot_be_filed_under_a_loaded_key_is_s6u_not_not_loaded() {
    let sent = demo();
    let returned = signed(&sent, 0, &abandon(), EcdsaSighashType::All, Nonce::Rfc6979);
    let report = run_with(&sent, &returned, &[]);
    // The view could not carry the signature, so no result came for it;
    // the key is loaded all the same.
    let loaded = [Fingerprint::from(abandon().fingerprint()).into()];
    let folded = with_nonces(report, &[], &loaded);
    assert_one(&folded, "S6u", Level::Danger, Where::Input(0));
    let recomputed = folded.facts.recomputed.expect("stated");
    assert_eq!((recomputed.recomputed, recomputed.of), (0, 1));
    assert!(recomputed.not_loaded.is_empty());
}
