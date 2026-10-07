//! Writes the threshold-wallet fixtures into a directory: a 2-of-3
//! group record on regtest and its three shares as words.
//!
//! ```text
//! cargo run -p osk-psbt --example threshold -- tools/vectors/psbt
//! ```
//!
//! Everything is fixed. The two chosen shares are the SHA-256 digests of
//! `osk threshold fixture share 0` and `osk threshold fixture share 1`
//! read as scalars; `osk_bip::frost::deal` computes the third share and
//! the group key from them, and the record is the group's public half.
//! Each share's own 32 bytes are the entropy of a 24-word English BIP-39
//! phrase, which is how §16.103 carries a share on paper.
//!
//! The share files are secret material of a fixture wallet: they exist
//! so that the passes that load a share have one to load, and no coin
//! this group can spend exists. `tests/fixtures.rs` checks the committed
//! files against this code.
#![allow(dead_code)]

use std::process::ExitCode;

use bitcoin::NetworkKind;
use bitcoin::hashes::{Hash, sha256};
use osk_bip::bip39::{Language, Mnemonic};
use osk_bip::frost::{self, SecShare};
use osk_bip::threshold::ThresholdRecord;
use osk_psbt::bitcoin;
use osk_psbt::bitcoin::secp256k1::Secp256k1;

/// The record file's name.
pub const RECORD_NAME: &str = "wallet-threshold-regtest.record";
/// One file per share, each 24 English words on one line.
pub const SHARE_NAMES: [&str; 3] = [
    "wallet-threshold-regtest-share-0.txt",
    "wallet-threshold-regtest-share-1.txt",
    "wallet-threshold-regtest-share-2.txt",
];

/// What the two chosen shares are derived from, so that every byte of
/// the group is reproducible from this file alone.
const CHOSEN: [&str; 2] = [
    "osk threshold fixture share 0",
    "osk threshold fixture share 1",
];

/// The group: three shares, any two of which sign, dealt from the two
/// chosen ones.
pub fn dealt() -> frost::Dealt {
    let secp = Secp256k1::new();
    let chosen: Vec<(u32, SecShare)> = CHOSEN
        .iter()
        .enumerate()
        .map(|(i, text)| {
            let digest = sha256::Hash::hash(text.as_bytes()).to_byte_array();
            (
                i as u32,
                SecShare::from_bytes(&digest).expect("a digest below the curve order"),
            )
        })
        .collect();
    frost::deal(&secp, 3, 2, &chosen).expect("two chosen shares of three")
}

/// The group's public record, on regtest.
pub fn record() -> ThresholdRecord {
    ThresholdRecord::new(dealt().info, NetworkKind::Test)
}

/// Each share's 24 English words, in identifier order.
pub fn share_words() -> Vec<String> {
    dealt()
        .shares
        .iter()
        .map(|share| {
            let m = Mnemonic::from_entropy(Language::English, &share.secret_bytes())
                .expect("32 bytes are 24 words");
            let words: Vec<&str> = m
                .indices()
                .iter()
                .map(|i| Language::English.word(*i))
                .collect();
            words.join(" ")
        })
        .collect()
}

/// Core's funded spend of the fixture wallet, before any nonce.
pub const FIRST_NAME: &str = "wallet-threshold-first.psbt";
/// What the device wrote at the first location with share 0, choosing
/// share 1: the PSBT with both public nonces and share 0's partial
/// signature, and share 1's secret nonce bound to it.
pub const CARRY_NAME: &str = "wallet-threshold-carry.osk";
/// What it wrote at the second location with share 1: aggregated,
/// final, and with every `osk` record stripped.
pub const SIGNED_NAME: &str = "wallet-threshold-signed.psbt";
/// What `tools/scripts/threshold-reference.py` checks the run against.
pub const TRANSCRIPT_NAME: &str = "wallet-threshold-transcript.json";

/// The seed the fixture run draws its nonces from. A device draws it
/// from the session key; fixing it here makes the run reproducible.
pub const SEED: [u8; 32] = [0x54; 32];

/// The share the first location holds, and the share it chooses.
pub const FIRST_SHARE: u32 = 0;
/// The share the second location holds.
pub const SECOND_SHARE: u32 = 1;

fn policy() -> osk_bip::policy::WalletPolicy {
    osk_bip::policy::WalletPolicy::of_record(record())
}

fn share_ref(id: u32) -> osk_psbt::ShareRef {
    let secp = Secp256k1::new();
    osk_psbt::ShareRef {
        pubshare: dealt().shares[id as usize].public_share(&secp),
    }
}

/// Runs one location's pass: the share it holds, the others it chooses,
/// and the carry section it read.
fn pass(
    bytes: &[u8],
    holding: u32,
    others: &[u32],
    carry: Option<&osk_psbt::threshold::CarrySection>,
) -> (osk_psbt::Psbt, osk_psbt::SignResult) {
    let secp = Secp256k1::new();
    let dealt = dealt();
    let share = &dealt.shares[holding as usize];
    let pubshare = share.public_share(&secp);
    let mut psbt = osk_psbt::Psbt::parse_base64(
        core::str::from_utf8(bytes)
            .expect("the PSBT is base64 text")
            .trim(),
    )
    .expect("a PSBT");
    let wallets = [policy()];
    let shares = [osk_psbt::ShareRef { pubshare }];
    let ctx = osk_psbt::Context {
        network: osk_bip::keys::Network::Regtest,
        keys: &[],
        wallets: &wallets,
        musig_session: None,
        shares: &shares,
        carry,
    };
    let door = |take: &mut dyn FnMut(&SecShare)| take(share);
    let keys = [osk_psbt::ShareKey {
        pubshare,
        open: &door,
    }];
    let mut session = None;
    let result = osk_psbt::sign(
        &mut psbt,
        &[],
        &[],
        &ctx,
        false,
        osk_psbt::Nonce::LowR,
        osk_psbt::Aux::Deterministic,
        &mut session,
        SEED,
        &keys,
        others,
    )
    .expect("the threshold pass");
    (psbt, result)
}

/// The first location: share 0 signs and chooses share 1, and the carry
/// file is what it hands the stick.
pub fn carry_file(dir: &std::path::Path) -> Vec<u8> {
    let funded = std::fs::read(dir.join(FIRST_NAME)).expect("the funded PSBT fixture");
    let (psbt, result) = pass(&funded, FIRST_SHARE, &[SECOND_SHARE], None);
    let section = result
        .threshold
        .and_then(|t| t.carry)
        .expect("a signer is still to sign");
    osk_psbt::threshold::carry_bytes(&section, &psbt.to_bytes())
}

/// The second location: share 1 reads the carry file, signs, aggregates
/// and finalizes.
pub fn signed(dir: &std::path::Path) -> (osk_psbt::Psbt, osk_psbt::SignResult) {
    let file = carry_file(dir);
    let carry = osk_psbt::threshold::Carry::parse(&file).expect("the carry file");
    let base64 = osk_psbt::base64::encode(&carry.psbt);
    pass(base64.as_bytes(), SECOND_SHARE, &[], Some(&carry.section))
}

/// The transcript BIP 445's reference implementation is replayed
/// against: every public value of the session, plus share 1's secret
/// share and secret nonce, which are this fixture group's and no one
/// else's.
pub fn transcript(dir: &std::path::Path) -> String {
    let secp = Secp256k1::new();
    let file = carry_file(dir);
    let carry = osk_psbt::threshold::Carry::parse(&file).expect("the carry file");
    let psbt = osk_psbt::Psbt::parse_bytes(&carry.psbt).expect("the carried PSBT");
    let wallets = [policy()];
    let shares = [share_ref(SECOND_SHARE)];
    let ctx = osk_psbt::Context {
        network: osk_bip::keys::Network::Regtest,
        keys: &[],
        wallets: &wallets,
        musig_session: None,
        shares: &shares,
        carry: Some(&carry.section),
    };
    let inspection = osk_psbt::inspect(&psbt, &ctx);
    let ti = inspection.inputs[0]
        .threshold
        .as_ref()
        .expect("a threshold input");
    let tx = psbt.unsigned_tx().clone();
    let prevouts: Vec<bitcoin::TxOut> = (0..tx.input.len())
        .map(|i| {
            psbt.inner()
                .spend_utxo(i)
                .expect("the previous output")
                .clone()
        })
        .collect();
    let msg = bitcoin::sighash::SighashCache::new(&tx)
        .taproot_key_spend_signature_hash(
            0,
            &bitcoin::sighash::Prevouts::All(&prevouts),
            bitcoin::sighash::TapSighashType::Default,
        )
        .expect("the key-path sighash");
    let msg: [u8; 32] = {
        use bitcoin::hashes::Hash;
        msg.to_byte_array()
    };

    let input = &psbt.inner().inputs[0];
    let nonce_records = osk_psbt::threshold::pub_nonces(input).expect("the nonce records");
    let sig_records = osk_psbt::threshold::partial_sigs(input).expect("the signature records");
    let dealt = dealt();
    let ids: Vec<u32> = ti.signers.clone();
    let pubshares: Vec<String> = ids
        .iter()
        .map(|id| hex(&ti.pubshare(*id).expect("a public share").serialize()))
        .collect();
    let pubnonces: Vec<String> = ids
        .iter()
        .map(|id| {
            let key = ti.pubshare(*id).expect("a public share");
            hex(&nonce_records
                .iter()
                .find(|e| e.pubshare == key)
                .expect("a nonce record")
                .value
                .serialize())
        })
        .collect();
    let tweaks: Vec<String> = ti
        .tweaks
        .iter()
        .map(|t| {
            format!(
                "{{\"tweak\": \"{}\", \"is_xonly\": {}}}",
                hex(&t.bytes),
                t.x_only
            )
        })
        .collect();
    let (signed_psbt, result) = signed(dir);
    let _ = signed_psbt;
    let mut psigs: Vec<String> = Vec::new();
    for id in &ids {
        let key = ti.pubshare(*id).expect("a public share");
        match sig_records.iter().find(|e| e.pubshare == key) {
            Some(entry) => psigs.push(hex(&entry.value.serialize())),
            // Share 1 signs at the second location, so its partial
            // signature is what that pass wrote and not what the carry
            // file carried.
            None => psigs.push(hex(&result
                .signed_inputs
                .iter()
                .find(|s| s.musig == Some(osk_psbt::MusigRole::Partial))
                .expect("the second location's partial signature")
                .sig_bytes)),
        }
    }
    let signature = hex(&result
        .signed_inputs
        .iter()
        .find(|s| s.musig == Some(osk_psbt::MusigRole::Aggregated))
        .expect("the aggregate")
        .sig_bytes);
    let entry = carry
        .section
        .entry(0, &ti.pubshare(SECOND_SHARE).expect("share 1"))
        .expect("share 1's stored nonce");
    let _ = &secp;
    format!(
        "{{\n  \"thresh_pk\": \"{}\",\n  \"n\": {},\n  \"t\": {},\n  \"ids\": [{}],\n  \"pubshares\": [{}],\n  \"pubnonces\": [{}],\n  \"tweaks\": [{}],\n  \"msg\": \"{}\",\n  \"my_id\": {},\n  \"secshare\": \"{}\",\n  \"secnonce\": \"{}\",\n  \"psigs\": [{}],\n  \"sig\": \"{}\"\n}}\n",
        hex(&ti.info.thresh_pk.serialize()),
        ti.info.n(),
        ti.info.t,
        ids.iter()
            .map(|i| i.to_string())
            .collect::<Vec<_>>()
            .join(", "),
        pubshares
            .iter()
            .map(|s| format!("\"{s}\""))
            .collect::<Vec<_>>()
            .join(", "),
        pubnonces
            .iter()
            .map(|s| format!("\"{s}\""))
            .collect::<Vec<_>>()
            .join(", "),
        tweaks.join(", "),
        hex(&msg),
        SECOND_SHARE,
        hex(&dealt.shares[SECOND_SHARE as usize].secret_bytes()),
        hex(&entry.secnonce.serialize()),
        psigs
            .iter()
            .map(|s| format!("\"{s}\""))
            .collect::<Vec<_>>()
            .join(", "),
        signature,
    )
}

/// Lower-case hex.
pub fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push(char::from(DIGITS[usize::from(b >> 4)]));
        out.push(char::from(DIGITS[usize::from(b & 15)]));
    }
    out
}

fn main() -> ExitCode {
    let Some(dir) = std::env::args().nth(1) else {
        eprintln!("usage: threshold <directory>");
        return ExitCode::FAILURE;
    };
    let dir = std::path::PathBuf::from(dir);
    let mut files = vec![(RECORD_NAME, format!("{}\n", record().to_text()))];
    for (name, words) in SHARE_NAMES.iter().zip(share_words()) {
        files.push((*name, format!("{words}\n")));
    }
    // The spend fixtures need Core's funded PSBT, which
    // `tools/scripts/threshold-regtest.sh setup` writes; without it the
    // record and the shares are still rewritten.
    if dir.join(FIRST_NAME).exists() {
        let carry = carry_file(&dir);
        let (psbt, _) = signed(&dir);
        if let Err(e) = std::fs::write(dir.join(CARRY_NAME), &carry) {
            eprintln!("error: write {}: {e}", dir.join(CARRY_NAME).display());
            return ExitCode::FAILURE;
        }
        println!("{}", dir.join(CARRY_NAME).display());
        files.push((SIGNED_NAME, psbt.to_base64()));
        files.push((TRANSCRIPT_NAME, transcript(&dir)));
    }
    for (name, body) in files {
        let path = dir.join(name);
        if let Err(e) = std::fs::write(&path, body) {
            eprintln!("error: write {}: {e}", path.display());
            return ExitCode::FAILURE;
        }
        println!("{}", path.display());
    }
    ExitCode::SUCCESS
}
