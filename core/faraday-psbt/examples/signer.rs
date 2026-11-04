//! Writes the PSBTs the signer check's tests read: a Returned PSBT, and
//! for some a Sent one, each differing from a clean one in one way.
//!
//! ```text
//! cargo run -p faraday-psbt --example signer -- tools/vectors/psbt
//! ```
//!
//! Two files are inputs and are not rebuilt: `signer-core-signed.psbt` and
//! `signer-core-final.psbt` are what Bitcoin Core 31.1 made from
//! `wallet-2of3.psbt` and `demo-regtest.psbt`
//! (`tools/scripts/signer-core.py`). `wallet-tapmulti-signed.psbt` is read
//! as it stands. Everything else is built from them with fixed keys
//! (the "abandon … about" and "zoo … wrong" seeds and two fixed secret
//! keys) and fixed nonces, so the files are byte-identical from one run to
//! the next. `tests/all/fixtures.rs` checks the committed files against
//! this code.
//!
//! A signature the crate's own signer would refuse to make (a `NONE`
//! sighash, a repeated or non-deterministic nonce, a key no origin names)
//! is made here with the secp256k1 API directly, and verifies.
#![allow(dead_code)]

use std::process::ExitCode;

use bitcoin::bip32::Fingerprint;
use bitcoin::psbt::raw;
use bitcoin::secp256k1::{Message, PublicKey, Scalar, Secp256k1, SecretKey};
use bitcoin::sighash::{EcdsaSighashType, SighashCache};
use bitcoin::{Amount, OutPoint, ScriptBuf, Sequence, TxIn, TxOut, Witness, ecdsa};
use faraday_bip::keys::{MasterKey, Network, ScriptType};
use faraday_crypto::work::Compute;
use faraday_psbt::Psbt;

#[path = "../tests/all/common/builder.rs"]
mod builder;

use builder::{Build, funding_tx, single_sig};

const ABANDON: &str =
    "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
const ZOO: &str = "zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo wrong";
const NET: Network = Network::Regtest;

/// One committed file.
pub struct Fixture {
    /// File name, `signer-<kind in kebab case>.psbt`.
    pub name: &'static str,
    /// What the file is for, one line, repeated in the README.
    pub note: &'static str,
    /// The file's text: base64.
    pub text: String,
}

fn master(words: &str) -> MasterKey {
    let c = Compute::unchecked();
    let m = faraday_bip::bip39::Mnemonic::parse(&c, faraday_bip::bip39::Language::English, words)
        .expect("fixed mnemonic");
    MasterKey::from_seed(&c, &m.to_seed(&c, b"").expect("empty passphrase"), NET)
}

/// Where the vectors are, from this crate.
fn vectors() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tools/vectors/psbt")
}

/// A committed PSBT, as it stands.
fn read(name: &str) -> Psbt {
    let text =
        std::fs::read_to_string(vectors().join(name)).unwrap_or_else(|e| panic!("{name}: {e}"));
    Psbt::parse_base64(&Compute::unchecked(), &text).unwrap_or_else(|e| panic!("{name}: {e:?}"))
}

/// A key's secret for one of the input's own origins: the one under
/// `master`'s fingerprint.
fn own_key(psbt: &Psbt, index: usize, master: &MasterKey) -> (bitcoin::PublicKey, SecretKey) {
    let c = Compute::unchecked();
    let fingerprint: Fingerprint = master.fingerprint().into();
    let (pk, (_, path)) = psbt.inner().inputs[index]
        .bip32_derivation
        .iter()
        .find(|(_, (fp, _))| *fp == fingerprint)
        .expect("an origin of this master key");
    let derived = master.derive(&c, path);
    assert_eq!(
        derived.to_xpub(&c).public_key,
        *pk,
        "the origin names this key"
    );
    (bitcoin::PublicKey::new(*pk), *derived.secret_key())
}

/// The sighash of an ECDSA input (SegWit v0, P2WPKH or P2WSH) under `kind`.
fn sighash(psbt: &Psbt, index: usize, kind: EcdsaSighashType) -> Message {
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

/// ECDSA with the nonce `k` chosen here rather than by RFC 6979, so that
/// one nonce can sign two inputs, or a nonce can be one no rule of the
/// device gives. `s = k⁻¹(z + r·d)`, the inverse by Fermat over the curve
/// order.
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

/// Input `index` signed under `key` with sighash `kind`: by RFC 6979, or
/// with the nonce `k`. The signature verifies.
fn sign(
    psbt: &mut Psbt,
    index: usize,
    key: &SecretKey,
    kind: EcdsaSighashType,
    k: Option<&SecretKey>,
) {
    let secp = Secp256k1::new();
    let msg = sighash(psbt, index, kind);
    let signature = match k {
        Some(k) => ecdsa_with_k(&secp, key, *msg.as_ref(), k),
        None => secp.sign_ecdsa(&msg, key),
    };
    let pk = PublicKey::from_secret_key(&secp, key);
    secp.verify_ecdsa(&msg, &signature, &pk)
        .expect("a signature that verifies");
    psbt.inner_mut().inputs[index].partial_sigs.insert(
        bitcoin::PublicKey::new(pk),
        ecdsa::Signature {
            signature,
            sighash_type: kind,
        },
    );
}

fn fixed(byte: u8) -> SecretKey {
    SecretKey::from_slice(&[byte; 32]).expect("a fixed secret key")
}

/// An address of the "zoo" seed: not ours, and stable.
fn theirs() -> ScriptBuf {
    let c = Compute::unchecked();
    master(ZOO)
        .account_xpub(&c, ScriptType::NativeSegwit, 0)
        .expect("an account")
        .address(false, 1)
        .expect("an address")
        .script_pubkey()
}

/// A second input of the first one's address, so that one key signs twice.
fn two_inputs() -> Psbt {
    let c = Compute::unchecked();
    let abandon = master(ABANDON);
    let mut b = Build::new(ScriptType::NativeSegwit, NET);
    b.recipients.push((theirs(), 70_000));
    b.change = Some(29_000);
    let mut psbt = single_sig(&abandon, &b);
    let script = abandon
        .account_xpub(&c, ScriptType::NativeSegwit, 0)
        .expect("the account")
        .address(false, 0)
        .expect("an address")
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

/// A pair of an unknown type the check knows nothing of.
fn unknown_key() -> raw::Key {
    raw::Key {
        type_value: 0xf0,
        key: vec![0xaa, 0xbb],
    }
}

fn base64(bytes: &[u8]) -> String {
    const A: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for chunk in bytes.chunks(3) {
        let n = chunk
            .iter()
            .enumerate()
            .fold(0u32, |n, (i, b)| n | u32::from(*b) << (16 - 8 * i));
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(A[(n >> (18 - 6 * i) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

/// Every fixture the example writes, in the order of SIGNER-CHECK.md §9.1.
#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
pub fn fixtures() -> Vec<Fixture> {
    let mut out = Vec::new();
    let mut add = |name: &'static str, note: &'static str, psbt: &Psbt| {
        out.push(Fixture {
            name,
            note,
            text: psbt.to_base64(),
        });
    };
    let abandon = master(ABANDON);
    let zoo = master(ZOO);
    let core_signed = read("signer-core-signed.psbt");
    let demo = read("demo-regtest.psbt");
    let (_, zoo_secret) = own_key(&core_signed, 0, &zoo);

    // The Sent for everything built from core-signed: wallet-2of3.psbt with
    // the one witness script Core added to output 1 beside its signature,
    // which is a difference of its own (SIGNER-CHECK §1).
    let mut core_sent = read("wallet-2of3.psbt");
    core_sent.inner_mut().outputs[1].witness_script =
        core_signed.inner().outputs[1].witness_script.clone();
    add(
        "signer-core-sent.psbt",
        "Sent: wallet-2of3.psbt plus the witness script Core added to output 1, copied from signer-core-signed",
        &core_sent,
    );

    // Returned, against signer-core-sent.psbt as Sent.
    let mut psbt = core_signed.clone();
    let output = &mut psbt.inner_mut().unsigned_tx.output[0];
    output.value = Amount::from_sat(output.value.to_sat() - 1);
    add(
        "signer-output-changed.psbt",
        "Returned, against signer-core-sent.psbt: signer-core-signed with output 0 one sat less",
        &psbt,
    );

    let mut psbt = core_signed.clone();
    let sigs = &mut psbt.inner_mut().inputs[0].partial_sigs;
    let (pk, mut sig) = sigs
        .iter()
        .next()
        .map(|(k, v)| (*k, *v))
        .expect("a signature");
    let mut compact = sig.signature.serialize_compact();
    compact[63] ^= 1;
    sig.signature = bitcoin::secp256k1::ecdsa::Signature::from_compact(&compact)
        .expect("a well-formed signature");
    sigs.insert(pk, sig);
    add(
        "signer-invalid.psbt",
        "Returned, against signer-core-sent.psbt: signer-core-signed with the last bit of its signature's s flipped",
        &psbt,
    );

    let mut psbt = core_signed.clone();
    sign(&mut psbt, 0, &fixed(5), EcdsaSighashType::All, None);
    add(
        "signer-foreign-key.psbt",
        "Returned, against signer-core-sent.psbt: signer-core-signed plus a valid signature under a key no origin names",
        &psbt,
    );

    // Returned, against demo-regtest.psbt as Sent.
    let (_, key) = own_key(&demo, 0, &abandon);
    for (name, note, kind) in [
        (
            "signer-sighash-none.psbt",
            "Returned, against demo-regtest.psbt: signed by the \"abandon\" key with SIGHASH_NONE",
            EcdsaSighashType::None,
        ),
        (
            "signer-sighash-single.psbt",
            "Returned, against demo-regtest.psbt: signed by the \"abandon\" key with SIGHASH_SINGLE",
            EcdsaSighashType::Single,
        ),
    ] {
        let mut psbt = demo.clone();
        sign(&mut psbt, 0, &key, kind, None);
        add(name, note, &psbt);
    }
    let mut psbt = demo.clone();
    sign(&mut psbt, 0, &key, EcdsaSighashType::All, Some(&fixed(9)));
    add(
        "signer-random-nonce.psbt",
        "Returned, against demo-regtest.psbt: signed by the \"abandon\" key with a nonce RFC 6979 does not give",
        &psbt,
    );

    // Two inputs under one key.
    let sent = two_inputs();
    add(
        "signer-nonce-reuse-sent.psbt",
        "Sent: a spend of two inputs of one address of the \"abandon\" key, 70 000 sat out and 29 000 back",
        &sent,
    );
    let mut psbt = sent.clone();
    let (_, key) = own_key(&psbt, 0, &abandon);
    for index in 0..2 {
        sign(
            &mut psbt,
            index,
            &key,
            EcdsaSighashType::All,
            Some(&fixed(7)),
        );
    }
    add(
        "signer-nonce-reuse.psbt",
        "Returned, against signer-nonce-reuse-sent.psbt: both inputs signed under one key with one nonce",
        &psbt,
    );

    // sig-removed is wallet-tapmulti-signed.psbt less one signature,
    // against itself.
    let mut psbt = read("wallet-tapmulti-signed.psbt");
    let sigs = &mut psbt.inner_mut().inputs[0].tap_script_sigs;
    let first = *sigs.keys().next().expect("a script signature");
    sigs.remove(&first);
    add(
        "signer-sig-removed.psbt",
        "Returned, against wallet-tapmulti-signed.psbt: that file less one of its two script signatures",
        &psbt,
    );

    let mut psbt = core_signed.clone();
    psbt.inner_mut().inputs[0].partial_sigs.clear();
    sign(
        &mut psbt,
        0,
        &zoo_secret,
        EcdsaSighashType::All,
        Some(&fixed(3)),
    );
    add(
        "signer-sig-changed.psbt",
        "Returned, against signer-core-signed.psbt: its signature remade, as valid, with another nonce",
        &psbt,
    );

    let mut psbt = read("signer-core-final.psbt");
    let input = &mut psbt.inner_mut().inputs[0];
    let mut items: Vec<Vec<u8>> = input
        .final_script_witness
        .as_ref()
        .expect("a final witness")
        .to_vec();
    items.push(vec![0x01]);
    input.final_script_witness = Some(Witness::from_slice(&items));
    add(
        "signer-final-extra.psbt",
        "Returned, against demo-regtest.psbt: signer-core-final with one more item, a byte, at the end of the witness",
        &psbt,
    );

    let mut psbt = core_signed.clone();
    psbt.inner_mut().inputs[0].bip32_derivation.clear();
    add(
        "signer-origin-removed.psbt",
        "Returned, against signer-core-sent.psbt: signer-core-signed with input 0's key origins removed",
        &psbt,
    );

    let mut psbt = core_signed.clone();
    let c = Compute::unchecked();
    let account = abandon
        .account_xpub(&c, ScriptType::NativeSegwit, 0)
        .expect("the account");
    psbt.inner_mut().xpub.insert(
        *account.xpub(),
        (abandon.fingerprint().into(), account.path().clone()),
    );
    add(
        "signer-xpub-added.psbt",
        "Returned, against signer-core-sent.psbt: signer-core-signed plus one global extended public key",
        &psbt,
    );

    let mut psbt = core_signed.clone();
    psbt.inner_mut()
        .unknown
        .insert(unknown_key(), vec![1, 2, 3, 4]);
    add(
        "signer-unknown-added.psbt",
        "Returned, against signer-core-sent.psbt: signer-core-signed plus a global pair of type 0xf0, a 2-byte key and a 4-byte value",
        &psbt,
    );

    let mut sent = core_sent.clone();
    sent.inner_mut()
        .unknown
        .insert(unknown_key(), vec![1, 2, 3, 4]);
    add(
        "signer-unknown-sent.psbt",
        "Sent: signer-core-sent.psbt plus a global pair of type 0xf0, a 2-byte key and a 4-byte value",
        &sent,
    );
    let mut psbt = core_signed.clone();
    psbt.inner_mut()
        .unknown
        .insert(unknown_key(), vec![1, 2, 3, 4, 5, 6]);
    add(
        "signer-unknown-changed.psbt",
        "Returned, against signer-unknown-sent.psbt: signer-core-signed with that pair's value two bytes longer",
        &psbt,
    );

    let mut psbt = core_signed.clone();
    psbt.inner_mut().inputs[0].proprietary.insert(
        raw::ProprietaryKey {
            prefix: b"device".to_vec(),
            subtype: 1,
            key: vec![0x01],
        },
        vec![0xde, 0xad, 0xbe, 0xef],
    );
    add(
        "signer-proprietary-added.psbt",
        "Returned, against signer-core-sent.psbt: signer-core-signed plus an input pair under the prefix \"device\", subtype 1, a 4-byte value",
        &psbt,
    );

    let mut psbt = core_signed.clone();
    psbt.inner_mut().inputs[0].proprietary.insert(
        raw::ProprietaryKey {
            prefix: b"osk".to_vec(),
            subtype: 0,
            key: vec![0x02; 33],
        },
        vec![0x11; 32],
    );
    add(
        "signer-osk-added.psbt",
        "Returned, against signer-core-sent.psbt: signer-core-signed plus an input record under Faraday's own prefix \"osk\", subtype 0, a 32-byte value",
        &psbt,
    );

    let mut bytes = core_signed.to_bytes();
    bytes.extend_from_slice(b"trailing");
    out.push(Fixture {
        name: "signer-trailing.psbt",
        note: "Returned, against signer-core-sent.psbt: the bytes of signer-core-signed with eight bytes after the last map",
        text: base64(&bytes),
    });
    out
}

fn main() -> ExitCode {
    let Some(dir) = std::env::args().nth(1) else {
        eprintln!("usage: signer <directory>");
        return ExitCode::FAILURE;
    };
    for f in fixtures() {
        let path = std::path::Path::new(&dir).join(f.name);
        if let Err(e) = std::fs::write(&path, &f.text) {
            eprintln!("error: write {}: {e}", path.display());
            return ExitCode::FAILURE;
        }
        println!("{}  {}", f.name, f.note);
        // The file the parser may refuse.
        let c = Compute::unchecked();
        match Psbt::parse_base64(&c, &f.text) {
            Ok(_) => {}
            Err(e) => println!("  the parser refuses it: {e:?}"),
        }
    }
    ExitCode::SUCCESS
}
