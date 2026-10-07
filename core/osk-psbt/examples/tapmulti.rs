//! Writes the taproot-multisig fixtures into a directory: the wallet
//! policy, and — once Bitcoin Core has funded it — what this tree wrote
//! from Core's spend.
//!
//! ```text
//! cargo run -p osk-psbt --example tapmulti -- tools/vectors/psbt
//! ```
//!
//! The wallet is BIP 387's `tr(H,sortedmulti_a(2,…))` over the same
//! three regtest keys as `wallet-2of3.policy` — "abandon … about", "zoo
//! … wrong" and "legal … yellow" — at BIP 48's `2'` account, which is
//! the wallet Add a wallet › Taproot multisig builds.
//!
//! `tools/scripts/tapmulti-regtest.sh setup` imports that wallet into
//! Bitcoin Core 31.1 watch-only, funds it and writes
//! `wallet-tapmulti-first.psbt`. This signs it with two of the three
//! keys and writes `wallet-tapmulti-signed.psbt`: one `tap_script_sig`
//! per key under the leaf's hash, and the input not final, because
//! `sign` finalizes nothing. Two of three satisfies the leaf, so Core
//! finalizes it, and so does the Sign flow, which hands over the
//! finished transaction. `tests/tapmulti.rs` checks the committed files
//! against this code.
#![allow(dead_code)]

use std::path::Path;
use std::process::ExitCode;

use osk_bip::keys::{MasterKey, Network};
use osk_bip::policy::WalletPolicy;
use osk_psbt::bitcoin::bip32::{ChildNumber, DerivationPath};
use osk_psbt::{Aux, Context, KeyRef, Nonce, Psbt, SignResult, sign};

const ABANDON: &str =
    "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
const ZOO: &str = "zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo wrong";
const LEGAL: &str = "legal winner thank year wave sausage worth useful legal winner thank yellow";

const NET: Network = Network::Regtest;

/// BIP 341's `H` written out, which is what a script-only taproot
/// wallet puts where an internal key goes.
pub const NUMS: &str = "50929b74c1a04954b78b4b6035e97a5e078a5a0f28ec96d547bfee9ace803ac0";

/// The wallet policy file's name.
pub const POLICY_NAME: &str = "wallet-tapmulti.policy";
/// Core's funded spend of that wallet.
pub const FIRST_NAME: &str = "wallet-tapmulti-first.psbt";
/// What this tree wrote from it, signed by two of the three keys.
pub const SIGNED_NAME: &str = "wallet-tapmulti-signed.psbt";

/// The words of the three keys, in the order the policy names them.
pub const SEEDS: [&str; 3] = [ABANDON, ZOO, LEGAL];

/// One of the three keys on regtest.
pub fn master(words: &str) -> MasterKey {
    let m = osk_bip::bip39::Mnemonic::parse(osk_bip::bip39::Language::English, words)
        .expect("fixed mnemonic");
    MasterKey::from_seed(&m.to_seed(b"").expect("empty passphrase"), NET)
}

/// BIP 48's script-multisig account: `48'/1'/0'/2'` on regtest. BIP 48
/// names no script type for taproot, so a taproot multisig reads its
/// cosigners at the same `2'` account (`docs/PLANNING.md` §16.106).
pub fn account_path() -> DerivationPath {
    let h = |i| ChildNumber::from_hardened_idx(i).expect("a hardened index");
    DerivationPath::from(vec![h(48), h(NET.coin_type()), h(0), h(2)])
}

/// One key as a descriptor writes it: its origin and its account
/// extended public key.
fn key_text(m: &MasterKey) -> String {
    let path = account_path();
    format!(
        "[{}/{path:#}]{}",
        m.fingerprint(),
        m.derive(&path).to_xpub()
    )
}

/// The 2-of-3 taproot multisig over the three keys.
pub fn policy() -> WalletPolicy {
    let keys: Vec<String> = SEEDS.iter().map(|w| key_text(&master(w))).collect();
    let refs: Vec<&str> = keys.iter().map(String::as_str).collect();
    WalletPolicy::from_parts(
        &format!("tr({NUMS},sortedmulti_a(2,@0/**,@1/**,@2/**))"),
        &refs,
    )
    .expect("a taproot multisig over three keys")
}

/// Core's funded spend, read from the committed fixture.
pub fn first(dir: &Path) -> Psbt {
    let text = std::fs::read_to_string(dir.join(FIRST_NAME)).expect("Core's funded PSBT");
    Psbt::parse_base64(text.trim()).expect("a PSBT")
}

/// Core's spend signed by the keys `holders` names. `sign` writes one
/// `tap_script_sig` per key under the leaf's hash and finalizes
/// nothing, so this is the PSBT a coordinator is handed.
pub fn signed_by(dir: &Path, holders: &[&str]) -> (Psbt, SignResult) {
    let masters: Vec<MasterKey> = holders.iter().map(|w| master(w)).collect();
    let keys: Vec<KeyRef> = masters
        .iter()
        .enumerate()
        .map(|(i, m)| KeyRef::from_master(m, i as u32).expect("accounts"))
        .collect();
    let selection: Vec<osk_bip::keys::Fingerprint> =
        masters.iter().map(MasterKey::fingerprint).collect();
    let refs: Vec<&MasterKey> = masters.iter().collect();
    let wallets = [policy()];
    let ctx = Context {
        network: NET,
        keys: &keys,
        wallets: &wallets,
        musig_session: None,
        shares: &[],
        carry: None,
    };
    let mut psbt = first(dir);
    let result = sign(
        &mut psbt,
        &refs,
        &selection,
        &ctx,
        false,
        Nonce::LowR,
        Aux::Deterministic,
        &mut None,
        [0u8; 32],
        &[],
        &[],
    )
    .expect("the wallet holds these keys");
    (psbt, result)
}

/// The committed signed fixture: the "abandon" key and the "zoo" key,
/// which are two of the wallet's three.
pub fn signed(dir: &Path) -> Psbt {
    signed_by(dir, &[ABANDON, ZOO]).0
}

fn main() -> ExitCode {
    let Some(dir) = std::env::args().nth(1) else {
        eprintln!("usage: tapmulti <directory>");
        return ExitCode::FAILURE;
    };
    let dir = std::path::PathBuf::from(dir);
    let mut files = vec![(POLICY_NAME, format!("{}\n", policy().to_text()))];
    // The signed fixture needs Core's funded PSBT, which
    // `tools/scripts/tapmulti-regtest.sh setup` writes; without it the
    // policy is still rewritten.
    if dir.join(FIRST_NAME).exists() {
        files.push((SIGNED_NAME, signed(&dir).to_base64()));
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
