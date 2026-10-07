//! Signs the MuSig2 fixture Bitcoin Core produced and writes what the
//! device wrote.
//!
//! ```text
//! cargo run -p osk-psbt --example musig -- tools/vectors/psbt
//! ```
//!
//! `tools/scripts/musig-regtest.sh setup` makes
//! `wallet-musig-regtest.policy`, `wallet-musig-device-first.psbt` and
//! `wallet-musig-core-first.psbt` with Bitcoin Core 31.1 on regtest.
//! This writes what the device wrote in both orders, under
//! [`Aux::Deterministic`] and a fixed session seed so that every byte is
//! reproducible, and `tests/musig.rs` checks the committed files against
//! this code:
//!
//! - Core first: `wallet-musig-core-first-signed.psbt`, the device
//!   signing last with no session.
//! - The device first: `wallet-musig-device-first-nonce.psbt`, round 1
//!   over the funded PSBT, and — once Core has answered it, which is
//!   `wallet-musig-device-first-core.psbt` —
//!   `wallet-musig-device-first-signed.psbt`, round 2 with the session
//!   round 1 opened, aggregated and final.
#![allow(dead_code)]

use std::path::Path;
use std::process::ExitCode;

use osk_bip::keys::{Fingerprint, MasterKey, Network};
use osk_bip::policy::WalletPolicy;
use osk_psbt::{Aux, Context, KeyRef, MusigSession, Nonce, Psbt, sign};

const ABANDON: &str =
    "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

const NET: Network = Network::Regtest;

/// The regtest wallet policy of the Core round trip.
pub const POLICY_NAME: &str = "wallet-musig-regtest.policy";
/// The funded PSBT, before any nonce.
pub const DEVICE_FIRST_NAME: &str = "wallet-musig-device-first.psbt";
/// Core's round-1 PSBT: Core's nonce present, this device's absent.
pub const CORE_FIRST_NAME: &str = "wallet-musig-core-first.psbt";
/// What the device writes from it: its nonce and its partial signature.
pub const CORE_FIRST_SIGNED_NAME: &str = "wallet-musig-core-first-signed.psbt";
/// The device's round 1 over the funded PSBT: its public nonce alone.
pub const DEVICE_FIRST_NONCE_NAME: &str = "wallet-musig-device-first-nonce.psbt";
/// Core's answer to it: Core's nonce and, seeing every nonce, Core's
/// partial signature.
pub const DEVICE_FIRST_CORE_NAME: &str = "wallet-musig-device-first-core.psbt";
/// The device's round 2 over that: its partial signature, the aggregate
/// as the key-path signature, and the input final.
pub const DEVICE_FIRST_SIGNED_NAME: &str = "wallet-musig-device-first-signed.psbt";

/// The session seed the fixture run draws its secret nonces from. A
/// device draws it from the session key; fixing it here is what makes
/// the two device-first fixtures reproducible.
pub const SESSION_SEED: [u8; 32] = [0x4d; 32];

/// The "abandon … about" key on regtest, which is `@0` of the wallet.
pub fn abandon() -> MasterKey {
    let m = osk_bip::bip39::Mnemonic::parse(osk_bip::bip39::Language::English, ABANDON)
        .expect("fixed mnemonic");
    MasterKey::from_seed(&m.to_seed(b"").expect("empty passphrase"), NET)
}

/// The registered wallet, read from the committed policy file.
pub fn policy(dir: &Path) -> WalletPolicy {
    let text = std::fs::read_to_string(dir.join(POLICY_NAME)).expect("the policy fixture");
    WalletPolicy::parse(&text).expect("a two-line BIP-388 policy")
}

/// Core's round-1 PSBT, signed by this device: its public nonce and its
/// partial signature, and nothing else.
pub fn core_first_signed(dir: &Path) -> Psbt {
    let master = abandon();
    let policy = policy(dir);
    let text = std::fs::read_to_string(dir.join(CORE_FIRST_NAME)).expect("Core's round-1 PSBT");
    let mut psbt = Psbt::parse_base64(text.trim()).expect("a PSBT");
    let keys = [KeyRef::from_master(&master, 0).expect("accounts")];
    let wallets = [policy];
    let ctx = Context {
        network: NET,
        keys: &keys,
        wallets: &wallets,
        musig_session: None,
        shares: &[],
        carry: None,
    };
    let selection = [master.fingerprint()];
    let out = sign(
        &mut psbt,
        &[&master],
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
    .expect("the device signs last");
    assert!(!out.signed_inputs.is_empty(), "nothing was signed");
    assert_eq!(
        selection[0], out.signed_inputs[0].fingerprint,
        "signed by another key"
    );
    let _: Fingerprint = selection[0];
    psbt
}

/// The device's round 1 over the funded PSBT: the PSBT with its public
/// nonce on it, and the session holding the secret.
pub fn device_first_nonce(dir: &Path) -> (Psbt, Option<MusigSession>) {
    let master = abandon();
    let policy = policy(dir);
    let text = std::fs::read_to_string(dir.join(DEVICE_FIRST_NAME)).expect("the funded PSBT");
    let mut psbt = Psbt::parse_base64(text.trim()).expect("a PSBT");
    let keys = [KeyRef::from_master(&master, 0).expect("accounts")];
    let wallets = [policy];
    let ctx = Context {
        network: NET,
        keys: &keys,
        wallets: &wallets,
        musig_session: None,
        shares: &[],
        carry: None,
    };
    let mut session = None;
    let out = sign(
        &mut psbt,
        &[&master],
        &[master.fingerprint()],
        &ctx,
        false,
        Nonce::LowR,
        Aux::Deterministic,
        &mut session,
        SESSION_SEED,
        &[],
        &[],
    )
    .expect("the device goes first");
    assert_eq!(out.nonces_shared, 1, "one nonce, for the one input");
    assert!(out.signed_inputs.is_empty(), "round 1 signs nothing");
    (psbt, session)
}

/// The device's round 2 over Core's answer, with the session round 1
/// opened: the partial signature, the aggregate and the final input.
pub fn device_first_signed(dir: &Path) -> Psbt {
    let master = abandon();
    let policy = policy(dir);
    let (_, mut session) = device_first_nonce(dir);
    let text = std::fs::read_to_string(dir.join(DEVICE_FIRST_CORE_NAME))
        .expect("Core's answer to the device's nonce");
    let mut psbt = Psbt::parse_base64(text.trim()).expect("a PSBT");
    let keys = [KeyRef::from_master(&master, 0).expect("accounts")];
    let wallets = [policy];
    let view = session.as_ref().map(|s: &MusigSession| s.view());
    let ctx = Context {
        network: NET,
        keys: &keys,
        wallets: &wallets,
        musig_session: view.as_ref(),
        shares: &[],
        carry: None,
    };
    let out = sign(
        &mut psbt,
        &[&master],
        &[master.fingerprint()],
        &ctx,
        false,
        Nonce::LowR,
        Aux::Deterministic,
        &mut session,
        SESSION_SEED,
        &[],
        &[],
    )
    .expect("the session signs");
    assert!(out.complete, "every partial signature was there");
    psbt
}

fn main() -> ExitCode {
    let Some(dir) = std::env::args().nth(1) else {
        eprintln!("usage: musig <directory>");
        return ExitCode::FAILURE;
    };
    let dir = std::path::PathBuf::from(dir);
    let mut written: Vec<(std::path::PathBuf, String)> = vec![
        (
            dir.join(CORE_FIRST_SIGNED_NAME),
            core_first_signed(&dir).to_base64(),
        ),
        (
            dir.join(DEVICE_FIRST_NONCE_NAME),
            device_first_nonce(&dir).0.to_base64(),
        ),
    ];
    // Round 2 waits on Core's answer to round 1, so the first run of a
    // new wallet writes the nonce and this one follows.
    if dir.join(DEVICE_FIRST_CORE_NAME).exists() {
        written.push((
            dir.join(DEVICE_FIRST_SIGNED_NAME),
            device_first_signed(&dir).to_base64(),
        ));
    }
    for (path, body) in written {
        if let Err(e) = std::fs::write(&path, body) {
            eprintln!("error: write {}: {e}", path.display());
            return ExitCode::FAILURE;
        }
        println!("{}", path.display());
    }
    ExitCode::SUCCESS
}
