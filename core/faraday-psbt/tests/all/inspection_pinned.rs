//! The inspection a signer is shown for every known PSBT stays the same.
//!
//! What a person is shown before signing is `inspect`'s answer: every
//! input, output, warning and total. This runs it over the PSBTs this
//! crate's sources keep (`tools/vectors/psbt`, the BIP-174 vectors, the
//! BBQr and own-randomness vectors, the PSBT fuzz corpora of `psbt_parse`,
//! `psbt_v2`, `classify` and `signer_check`, the spends the wallet example
//! builds, and spends built here for each script type, a signed-message
//! request and, run on its own, a spend into another registered wallet),
//! under each network
//! with no key, with the test key loaded and with three keys and three
//! wallets registered; the MuSig2 PSBTs also with the MuSig2 wallet
//! registered, without and with an open session. A digest of all the
//! answers is compared against the one recorded, and so is the count of
//! files that did not parse, so a source that stops parsing shows.
//!
//! The fuzz corpus files are a fixed list, so a file a fuzz run adds is not
//! read. The constant is remade only when a change to what `inspect` reports
//! is meant, and then the commit says what is reported differently. The
//! digest is over Debug text, so a bitcoin or miniscript upgrade that changes
//! Debug output changes it with no change to `inspect`; that commit remakes
//! DIGEST and says so.

use std::path::{Path, PathBuf};

use bitcoin::hashes::{Hash, sha256};
use faraday_bip::keys::{Network, ScriptType};
use faraday_crypto::work::Compute;
use faraday_psbt::v2::PsbtV2;
use faraday_psbt::{Context, KeyRef, MusigSession, MusigSessionView, Psbt, inspect};

use crate::common::builder::{
    Build, multisig_policy, single_sig, sorted_multisig_change, with_multisig_output,
};
use crate::common::{ABANDON, LEGAL, ZOO, master, records, unhex};

#[path = "../../examples/musig.rs"]
mod musig_example;
#[path = "../../examples/wallet.rs"]
mod wallet;

/// SHA-256 over the Debug text of every inspection, in order.
const DIGEST: &str = "dd4a164b8a322b0c8f9431a1fe06c2622c6227791a899835273afa9494b8fac1";
/// How many PSBTs are inspected (each under every context).
const PSBTS: usize = 149;
/// How many files read as PSBT sources did not parse.
const SKIPPED: usize = 40;

/// The fuzz corpus files read, by folder, each sorted. A fixed list: a corpus
/// file added by a fuzz run is not read, and one removed or renamed fails the
/// test, so only a change to what a signer sees moves the digest.
const PSBT_PARSE: &[&str] = &[
    "bip370-all-fields.psbt.bin",
    "demo-regtest.psbt.bin",
    "demo-regtest.psbt.txt",
    "final-input-spent-output-missing.psbt.bin",
    "finalize-prevout-output-missing.psbt.bin",
    "wallet-2of3.psbt.bin",
    "wallet-2of3.psbt.txt",
    "warn-change-spoof.psbt.bin",
    "warn-change-spoof.psbt.txt",
    "warn-unsupported-input.psbt.bin",
    "warn-unsupported-input.psbt.txt",
    "warn-utxo-mismatch.psbt.bin",
    "warn-utxo-mismatch.psbt.txt",
];
const PSBT_V2: &[&str] = &[
    "bip370-0.psbt.bin",
    "bip370-1.psbt.bin",
    "bip370-2.psbt.bin",
    "bip370-3.psbt.bin",
    "bip370-4.psbt.bin",
    "bip370-5.psbt.bin",
];
const CLASSIFY: &[&str] = &["psbt-base64.txt", "psbt-binary.bin"];
const SIGNER_CHECK: &[&str] = &[
    "demo-regtest.psbt.seed",
    "finalize-multi-smallest-final.psbt.seed",
    "finalize-multi-smallest-wsh-final.psbt.seed",
    "finalize-multi-smallest-wsh.psbt.seed",
    "finalize-multi-smallest.psbt.seed",
    "finalize-or-b-final.psbt.seed",
    "finalize-or-b.psbt.seed",
    "finalize-tap-leaf-order-final.psbt.seed",
    "finalize-tap-leaf-order.psbt.seed",
    "finalize-tap-leaf-tie-final.psbt.seed",
    "finalize-tap-leaf-tie.psbt.seed",
    "finalize-thresh-tie-final.psbt.seed",
];

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// A PSBT in any of the forms the repository keeps it in: binary or
/// base64, version 0 or version 2 (read as the version 0 form a signer
/// takes).
fn read(c: &Compute, bytes: &[u8]) -> Option<Psbt> {
    if let Ok(p) = Psbt::parse_bytes(c, bytes) {
        return Some(p);
    }
    if let Ok(v2) = PsbtV2::parse_bytes(c, bytes) {
        return v2.to_v0(c).ok();
    }
    let text = std::str::from_utf8(bytes).ok()?;
    if let Ok(p) = Psbt::parse_base64(c, text) {
        return Some(p);
    }
    PsbtV2::parse_base64(c, text).ok()?.to_v0(c).ok()
}

fn files(dir: &Path) -> Vec<PathBuf> {
    let mut all: Vec<PathBuf> = std::fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("{}: {e}", dir.display()))
        .map(|e| e.expect("a directory entry").path())
        .filter(|p| p.is_file())
        .collect();
    all.sort();
    all
}

/// Every PSBT the sources keep, and how many files read did not parse.
fn known_psbts(c: &Compute) -> (Vec<(String, Psbt)>, usize) {
    let mut out: Vec<(String, Psbt)> = Vec::new();
    let mut skipped = 0;
    let root = root();

    // (directory, only files ending in one of these; empty is all)
    let dirs: [(&str, &[&str]); 3] = [
        ("tools/vectors/psbt", &[]),
        ("tools/vectors/bbqr", &[".psbt"]),
        ("tools/vectors/own-randomness", &["musig-unsigned.psbt"]),
    ];
    for (dir, only) in dirs {
        for path in files(&root.join(dir)) {
            let name = path.file_name().unwrap().to_string_lossy().into_owned();
            if name.ends_with(".policy") || name.ends_with(".py") || name.ends_with(".md") {
                continue;
            }
            if !only.is_empty() && !only.iter().any(|e| name.ends_with(e)) {
                continue;
            }
            let bytes = std::fs::read(&path).unwrap();
            match read(c, &bytes) {
                Some(p) => out.push((format!("{dir}/{name}"), p)),
                None => skipped += 1,
            }
        }
    }

    for (dir, names) in [
        ("fuzz/corpus/psbt_parse", PSBT_PARSE),
        ("fuzz/corpus/psbt_v2", PSBT_V2),
        ("fuzz/corpus/classify", CLASSIFY),
        ("fuzz/corpus/signer_check", SIGNER_CHECK),
    ] {
        for name in names {
            let bytes = std::fs::read(root.join(dir).join(name))
                .unwrap_or_else(|e| panic!("{dir}/{name} is missing: {e}"));
            match read(c, &bytes) {
                Some(p) => out.push((format!("{dir}/{name}"), p)),
                None => skipped += 1,
            }
        }
    }

    // The spends the wallet example builds, for the wallets registered below.
    for (name, psbt) in [
        ("wallet/psbt", wallet::psbt()),
        ("wallet/liana", wallet::liana_psbt()),
        ("wallet/liana-recovery", wallet::liana_recovery_psbt()),
        ("wallet/tree", wallet::tree_psbt()),
        ("wallet/tree-keypath", wallet::tree_keypath_psbt()),
    ] {
        out.push((name.to_string(), psbt));
    }

    for file in ["bip174", "bip174-roles"] {
        for rec in records(&format!("tools/vectors/psbt/{file}.txt")) {
            if let Some(hex) = rec.opt("hex") {
                match Psbt::parse_bytes(c, &unhex(hex)) {
                    Ok(p) => out.push((format!("{file}/{}", rec.get("case")), p)),
                    Err(_) => skipped += 1,
                }
            }
        }
    }

    // Spends built here: each script type with and without change, a fee
    // far too high, dust, a locktime, and a multisig with change.
    let net = Network::Regtest;
    let abandon = master(ABANDON, net);
    let zoo = master(ZOO, net);
    let legal = master(LEGAL, net);
    let foreign = zoo
        .account_xpub(c, ScriptType::NativeSegwit, 0)
        .unwrap()
        .address(false, 0)
        .unwrap()
        .script_pubkey();
    for script_type in ScriptType::ALL {
        for (label, recipient, change, locktime) in [
            ("plain", 60_000, None, 0),
            ("change", 60_000, Some(39_000), 0),
            ("high-fee", 60_000, Some(1_000), 0),
            ("dust", 99_800, Some(100), 0),
            ("locktime", 60_000, Some(39_000), 500_000_000),
        ] {
            let mut b = Build::new(script_type, net);
            b.recipients.push((foreign.clone(), recipient));
            b.change = change;
            b.locktime = locktime;
            out.push((
                format!("built/{script_type:?}/{label}"),
                single_sig(&abandon, &b),
            ));
        }
    }
    let masters = [&abandon, &zoo, &legal];
    for (label, recipients, change) in [
        ("multisig-change", vec![(foreign.clone(), 60_000)], 39_000),
        (
            "multisig-reuse",
            vec![(foreign.clone(), 30_000), (foreign.clone(), 30_000)],
            39_000,
        ),
    ] {
        out.push((
            format!("built/{label}"),
            sorted_multisig_change(&masters, 2, net, 100_000, &recipients, change),
        ));
    }

    // A lone 0 OP_RETURN output spending from the person's own address: a
    // signed-message request (BIP-322's `to_sign` for "Hello World").
    // On a version 0 transaction (BIP-322's own) and on a version 2 one, so
    // each half of "version 0 or looks like a message request" shows alone.
    for version in [0, 2] {
        out.push((
            format!("built/message-request-v{version}"),
            message_request(&abandon, version),
        ));
    }
    (out, skipped)
}

/// BIP-322's `to_sign` for "Hello World" and the first receive address of
/// `abandon`, as a PSBT the key signs for.
fn message_request(abandon: &faraday_bip::keys::MasterKey, version: i32) -> Psbt {
    use bitcoin::{Amount, OutPoint, ScriptBuf, Sequence, Transaction, TxIn, TxOut, Witness};
    let c = Compute::unchecked();
    let net = Network::Regtest;
    let mine = abandon
        .account_xpub(&c, ScriptType::NativeSegwit, 0)
        .unwrap()
        .address(false, 0)
        .unwrap();
    let hash = faraday_psbt::message::message_hash("Hello World");
    let push: &bitcoin::script::PushBytes = (&hash).into();
    let to_spend = Transaction {
        version: bitcoin::transaction::Version(0),
        lock_time: bitcoin::absolute::LockTime::ZERO,
        input: vec![TxIn {
            previous_output: OutPoint {
                txid: bitcoin::Txid::from_byte_array([0; 32]),
                vout: 0xFFFF_FFFF,
            },
            script_sig: bitcoin::script::Builder::new()
                .push_opcode(bitcoin::opcodes::OP_0)
                .push_slice(push)
                .into_script(),
            sequence: Sequence(0),
            witness: Witness::new(),
        }],
        output: vec![TxOut {
            value: Amount::ZERO,
            script_pubkey: mine.script_pubkey(),
        }],
    };
    let to_sign = Transaction {
        version: bitcoin::transaction::Version(version),
        lock_time: bitcoin::absolute::LockTime::ZERO,
        input: vec![TxIn {
            previous_output: OutPoint::new(to_spend.compute_txid(), 0),
            script_sig: ScriptBuf::new(),
            sequence: Sequence(0),
            witness: Witness::new(),
        }],
        output: vec![TxOut {
            value: Amount::ZERO,
            script_pubkey: bitcoin::script::Builder::new()
                .push_opcode(bitcoin::opcodes::all::OP_RETURN)
                .into_script(),
        }],
    };
    let mut inner = bitcoin::Psbt::from_unsigned_tx(to_sign).unwrap();
    inner.inputs[0].witness_utxo = Some(to_spend.output[0].clone());
    inner.inputs[0].non_witness_utxo = Some(to_spend);
    let h = |i| bitcoin::bip32::ChildNumber::from_hardened_idx(i).unwrap();
    let n = |i| bitcoin::bip32::ChildNumber::from_normal_idx(i).unwrap();
    let path =
        bitcoin::bip32::DerivationPath::from(vec![h(84), h(net.coin_type()), h(0), n(0), n(0)]);
    let key = abandon.derive(&c, &path).to_xpub(&c).public_key;
    inner.inputs[0]
        .bip32_derivation
        .insert(key, (abandon.fingerprint().into(), path));
    Psbt::from(inner)
}

#[test]
fn the_inspection_a_signer_is_shown_for_every_known_psbt_stays_the_same() {
    let c = Compute::unchecked();
    let (psbts, skipped) = known_psbts(&c);

    let wallets = [
        wallet::policy(),
        wallet::liana_policy(),
        wallet::tree_policy(),
    ];
    // The MuSig2 wallet and every MuSig2 PSBT the vectors keep, and the
    // session the device-first round leaves open.
    let dir = root().join("tools/vectors/psbt");
    let musig_wallets = [musig_example::policy(&dir)];
    let (device_first, session) = musig_example::device_first_nonce(&dir);
    let mut musig_psbts = vec![("device-first-nonce".to_string(), device_first)];
    for path in files(&dir) {
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        if name.contains("musig") && name.ends_with(".psbt") {
            let bytes = std::fs::read(&path).unwrap();
            musig_psbts.push((name, read(&c, &bytes).expect("a MuSig2 PSBT")));
        }
    }
    let bytes =
        std::fs::read(root().join("tools/vectors/own-randomness/musig-unsigned.psbt")).unwrap();
    musig_psbts.push((
        "own-randomness".to_string(),
        read(&c, &bytes).expect("a MuSig2 PSBT"),
    ));

    let mut text = String::new();
    for network in [Network::Mainnet, Network::Testnet, Network::Regtest] {
        let one = [KeyRef::from_master(&c, &master(ABANDON, network), 0).unwrap()];
        let three =
            [ABANDON, ZOO, LEGAL].map(|w| KeyRef::from_master(&c, &master(w, network), 0).unwrap());
        let contexts: [(&str, Context); 3] = [
            (
                "no key",
                Context {
                    network,
                    keys: &[],
                    wallets: &[],
                    musig_session: None,
                },
            ),
            (
                "one key",
                Context {
                    network,
                    keys: &one,
                    wallets: &[],
                    musig_session: None,
                },
            ),
            (
                "three keys, three wallets",
                Context {
                    network,
                    keys: &three,
                    wallets: &wallets,
                    musig_session: None,
                },
            ),
        ];
        for (name, psbt) in &psbts {
            for (label, ctx) in &contexts {
                let insp = inspect(&c, psbt, ctx);
                text.push_str(&format!("{network:?} {label} {name}\n{insp:?}\n"));
            }
        }

        // The MuSig2 PSBTs with the MuSig2 wallet registered: no session,
        // the session of the device-first round (another transaction's
        // for all but one), and a session once that round's nonce is spent.
        let musig_keys = [KeyRef::from_master(&c, &musig_example::abandon(), 0).unwrap()];
        let musig_view = session.as_ref().map(MusigSession::view);
        let sessions: [(&str, Option<&MusigSessionView>); 2] =
            [("no session", None), ("session", musig_view.as_ref())];
        for (name, psbt) in &musig_psbts {
            for (label, session) in sessions {
                let ctx = Context {
                    network,
                    keys: &musig_keys,
                    wallets: &musig_wallets,
                    musig_session: session,
                };
                let insp = inspect(&c, psbt, &ctx);
                text.push_str(&format!("{network:?} musig {label} {name}\n{insp:?}\n"));
            }
        }
    }

    // Coins sent to another of the person's own registered wallets: a
    // spend with an output to a 2-of-2 multisig, once with that wallet
    // registered and once with the spend's own wallet only.
    let net = Network::Regtest;
    let (abandon, zoo) = (master(ABANDON, net), master(ZOO, net));
    let foreign = zoo
        .account_xpub(&c, ScriptType::NativeSegwit, 0)
        .unwrap()
        .address(false, 0)
        .unwrap()
        .script_pubkey();
    let mut b = Build::new(ScriptType::NativeSegwit, net);
    b.recipients.push((foreign, 60_000));
    let transfer =
        with_multisig_output(single_sig(&abandon, &b), &[&abandon, &zoo], 2, net, 39_000);
    let multisig = [multisig_policy(&[&abandon, &zoo], 2, net)];
    let key = [KeyRef::from_master(&c, &abandon, 0).unwrap()];
    for (label, wallets) in [
        ("wallet registered", &multisig[..]),
        ("not registered", &[]),
    ] {
        let ctx = Context {
            network: net,
            keys: &key,
            wallets,
            musig_session: None,
        };
        let insp = inspect(&c, &transfer, &ctx);
        text.push_str(&format!("transfer {label}\n{insp:?}\n"));
    }
    let digest = sha256::Hash::hash(text.as_bytes()).to_string();
    assert_eq!(digest, DIGEST, "what inspect reports has changed");
    assert_eq!(psbts.len(), PSBTS, "the PSBTs inspected");
    assert_eq!(skipped, SKIPPED, "the files that did not parse");
}
