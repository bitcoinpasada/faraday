//! The spends of the test wallets, as serialized PSBTs: the unsigned spend of each, the Savings
//! spend with its cosigner's copy, and the MuSig2 wallet's policy and unsigned spend. Over the
//! domain crates only (the data half of `main`'s `testkit`).
#![expect(
    clippy::disallowed_methods,
    clippy::disallowed_types,
    reason = "the test kit builds keys and seeds directly and stays out of every shipped binary (docs/KEYS.md)"
)]

use std::collections::BTreeMap;

use faraday_bip::bip39::{Language, Mnemonic};
use faraday_bip::bitcoin::absolute::LockTime;
use faraday_bip::bitcoin::bip32::{ChildNumber, DerivationPath, Xpub};
use faraday_bip::bitcoin::hashes::{Hash, sha256};
use faraday_bip::bitcoin::secp256k1::{Secp256k1, XOnlyPublicKey};
use faraday_bip::bitcoin::transaction::Version;
use faraday_bip::bitcoin::{
    Address, Amount, OutPoint, ScriptBuf, Sequence, Transaction, TxIn, TxOut, Txid, Witness,
};
use faraday_bip::keys::{MasterKey, Network};
use faraday_bip::miniscript::psbt::PsbtExt;
use faraday_bip::policy::WalletPolicy;
use faraday_psbt::Psbt;
use faraday_text::{Text, text};

use crate::{TestWallet, compute, test_seeds};

/// Every test wallet is on testnet.
pub(crate) const NET: Network = Network::Testnet;

/// Where every spend pays 0.015 tBTC: the testnet address of BIP-84's first test key.
const RECIPIENT: &str = "tb1qcr8te4kr609gcawutmrza0j4xv80jy8zmfp6l0";

/// BIP-341's unspendable internal key, for a wallet that spends only by script.
pub(crate) const NUMS: &str = "50929b74c1a04954b78b4b6035e97a5e078a5a0f28ec96d547bfee9ace803ac0";

pub(crate) fn master(n: usize) -> Result<MasterKey, Text> {
    let c = compute();
    let words = Text::of(test_seeds()[n].words);
    let m = Mnemonic::parse(&c, Language::English, &words).map_err(|e| text!("{e:?}"))?;
    let seed = m.to_seed(&c, b"").map_err(|e| text!("{e:?}"))?;
    Ok(MasterKey::from_seed(&c, &seed, NET))
}

/// `[fingerprint/path]xpub` for test seed `n` at `path`.
pub(crate) fn key(n: usize, path: &str) -> Result<Text, Text> {
    let c = compute();
    let m = master(n)?;
    let p: DerivationPath = path.parse().map_err(|e| text!("{e}"))?;
    let shown = Text::replaced(path.trim_start_matches("m/"), "'", "h");
    let shown = shown.as_str();
    Ok(text!(
        "[{}/{shown}]{}",
        m.fingerprint(),
        m.derive(&c, &p).to_xpub(&c)
    ))
}

/// The wallet at one address, as a coordinator fills a PSBT from it. A
/// `sortedmulti_a` is written as the `multi_a` over its keys in the order
/// the sorted script puts them at this index, since `miniscript` has no
/// sorted form of it.
fn definite(
    policy: &WalletPolicy,
    change: bool,
    index: u32,
) -> Result<
    faraday_bip::miniscript::Descriptor<faraday_bip::miniscript::descriptor::DefiniteDescriptorKey>,
    Text,
> {
    use faraday_bip::miniscript::descriptor::{Descriptor, DescriptorPublicKey};
    let text = policy.to_descriptor();
    if text.contains("sortedmulti_a(") {
        let secp = Secp256k1::verification_only();
        let steps = [
            ChildNumber::from_normal_idx(u32::from(change)).map_err(|e| text!("{e}"))?,
            ChildNumber::from_normal_idx(index).map_err(|e| text!("{e}"))?,
        ];
        let mut keys: Vec<(XOnlyPublicKey, Text)> = policy
            .keys()
            .iter()
            .map(|k| {
                let derived = k
                    .xpub()
                    .derive_pub(&secp, &steps)
                    .expect("normal child")
                    .public_key
                    .x_only_public_key()
                    .0;
                (
                    derived,
                    text!("{}/{}/{index}", &*k.key_text(), u32::from(change)),
                )
            })
            .collect();
        keys.sort_by_key(|(d, _)| d.serialize());
        let (m, _) = policy
            .tapscript_quorum()
            .ok_or_else(|| Text::of("no quorum"))?;
        let list: Vec<Text> = keys.into_iter().map(|(_, t)| t).collect();
        return text!("tr({NUMS},multi_a({m},{}))", &*Text::joined(&list, ","))
            .parse()
            .map_err(|e| text!("{e}"));
    }
    let d: Descriptor<DescriptorPublicKey> = text.parse().map_err(|e| text!("{e}"))?;
    let chains = d.into_single_descriptors().map_err(|e| text!("{e}"))?;
    chains[usize::from(change)]
        .clone()
        .at_derivation_index(index)
        .map_err(|e| text!("{e}"))
}

fn funding(script_pubkey: ScriptBuf) -> Transaction {
    // The prototype's script read the digest as a txid in display order.
    // The phrase keeps the project's old name: examples/wallet_2of3_demo.rs
    // hashes the same bytes, and the txids depend on them.
    let mut b = sha256::Hash::hash(b"openfaraday prototype funding").to_byte_array();
    b.reverse();
    Transaction {
        version: Version::TWO,
        lock_time: LockTime::ZERO,
        input: vec![TxIn {
            previous_output: OutPoint {
                txid: Txid::from_byte_array(b),
                vout: 0,
            },
            script_sig: ScriptBuf::new(),
            sequence: Sequence(0xffff_fffd),
            witness: Witness::new(),
        }],
        output: vec![TxOut {
            value: Amount::from_sat(10_000_000),
            script_pubkey,
        }],
    }
}

/// The unsigned spend of a test wallet, as a `Psbt`.
pub(crate) fn unsigned_psbt(kit: &TestWallet) -> Result<Psbt, Text> {
    let c = compute();
    let policy =
        WalletPolicy::parse_any(&c, &kit.descriptor).map_err(|e| text!("{}: {e:?}", kit.id))?;
    let input = definite(&policy, false, 0)?;
    let back = definite(&policy, true, 0)?;
    let fund = funding(input.script_pubkey());
    let recipient: Address = RECIPIENT
        .parse::<Address<_>>()
        .map_err(|e| text!("{e}"))?
        .assume_checked();
    let tx = Transaction {
        version: Version::TWO,
        lock_time: LockTime::ZERO,
        input: vec![TxIn {
            previous_output: OutPoint {
                txid: fund.compute_txid(),
                vout: 0,
            },
            script_sig: ScriptBuf::new(),
            sequence: Sequence(0xffff_fffd),
            witness: Witness::new(),
        }],
        output: vec![
            TxOut {
                value: Amount::from_sat(1_500_000),
                script_pubkey: recipient.script_pubkey(),
            },
            TxOut {
                value: Amount::from_sat(10_000_000 - 1_500_000 - kit.fee),
                script_pubkey: back.script_pubkey(),
            },
        ],
    };
    let mut psbt = faraday_bip::bitcoin::Psbt::from_unsigned_tx(tx).map_err(|e| text!("{e}"))?;
    let segwit =
        !kit.descriptor.starts_with("pkh(") && !kit.descriptor.starts_with("sh(sortedmulti");
    if segwit {
        psbt.inputs[0].witness_utxo = Some(fund.output[0].clone());
    }
    psbt.inputs[0].non_witness_utxo = Some(fund);
    psbt.update_input_with_descriptor(0, &input)
        .map_err(|e| text!("{}: input: {e:?}", kit.id))?;
    psbt.update_output_with_descriptor(1, &back)
        .map_err(|e| text!("{}: change: {e:?}", kit.id))?;
    Ok(Psbt::from(psbt))
}

/// The wallet's spend signed by test seed `n` alone.
pub(crate) fn cosigned(kit: &TestWallet, n: usize) -> Result<Psbt, Text> {
    let c = compute();
    let policy =
        WalletPolicy::parse_any(&c, &kit.descriptor).map_err(|e| text!("{}: {e:?}", kit.id))?;
    let master = master(n)?;
    let refs = faraday_keys::psbt::key_refs(&c, &[&master]);
    let policies = [policy];
    let context = faraday_psbt::Context {
        network: NET,
        keys: &refs,
        wallets: &policies,
        musig_session: None,
    };
    let mut signed = unsigned_psbt(kit)?;
    let done = faraday_keys::psbt::sign(
        &c,
        &mut signed,
        &[&master],
        &[master.fingerprint()],
        (&context, &[]),
        &mut None,
        None,
    )
    .map_err(|e| text!("{}: {e:?}", kit.id))?;
    if done.signed_inputs.is_empty() {
        return Err(text!("{}: the cosigner did not sign", kit.id));
    }
    Ok(signed)
}

/// The MuSig2 test wallet: one aggregate key over the three test seeds' Taproot account keys,
/// every participant signs (BIP-390).
pub(crate) fn musig_policy() -> Result<WalletPolicy, Text> {
    let c = compute();
    let texts = (0..3)
        .map(|n| key(n, "m/86'/1'/0'"))
        .collect::<Result<Vec<Text>, Text>>()?;
    let refs: Vec<&str> = texts.iter().map(|t| t.as_str()).collect();
    WalletPolicy::from_parts(&c, "tr(musig(@0,@1,@2)/**)", &refs).map_err(|e| text!("{e:?}"))
}

fn taproot_account(n: usize) -> Result<Xpub, Text> {
    let c = compute();
    let account = master(n)?
        .account_xpub(&c, faraday_bip::keys::ScriptType::Taproot, 0)
        .map_err(|e| text!("{e:?}"))?;
    Ok(*account.xpub())
}

/// The tweaks from the root aggregate to the output key at chain/index, and the internal key on
/// the way.
fn musig_internal(
    participants: &[faraday_bip::bitcoin::secp256k1::PublicKey],
    index: u32,
) -> Result<faraday_bip::bitcoin::secp256k1::PublicKey, Text> {
    let secp = Secp256k1::verification_only();
    let mut xpub =
        faraday_bip::musig::aggregate_xpub(participants, NET).map_err(|e| text!("{e:?}"))?;
    for step in [0, index] {
        let child = ChildNumber::from_normal_idx(step).map_err(|e| text!("{e}"))?;
        xpub = xpub.ckd_pub(&secp, child).map_err(|e| text!("{e}"))?;
    }
    Ok(xpub.public_key)
}

/// An unsigned spend of the MuSig2 wallet: 0.1 BTC in at its first address, everything less the
/// fee out to the recipient, with the fields a coordinator that speaks BIP-373 fills in.
pub(crate) fn musig_unsigned_psbt() -> Result<Psbt, Text> {
    let c = compute();
    let policy = musig_policy()?;
    let spk = policy.script_at(&c, false, 0).map_err(|e| text!("{e:?}"))?;
    let fund = funding(spk);
    let recipient: Address = RECIPIENT
        .parse::<Address<_>>()
        .map_err(|e| text!("{e}"))?
        .assume_checked();
    let tx = Transaction {
        version: Version::TWO,
        lock_time: LockTime::ZERO,
        input: vec![TxIn {
            previous_output: OutPoint {
                txid: fund.compute_txid(),
                vout: 0,
            },
            script_sig: ScriptBuf::new(),
            sequence: Sequence(0xffff_fffd),
            witness: Witness::new(),
        }],
        output: vec![TxOut {
            value: Amount::from_sat(10_000_000 - 1_200),
            script_pubkey: recipient.script_pubkey(),
        }],
    };
    let mut inner = faraday_bip::bitcoin::Psbt::from_unsigned_tx(tx).map_err(|e| text!("{e}"))?;
    inner.inputs[0].witness_utxo = Some(fund.output[0].clone());
    let mut keys = Vec::new();
    for n in 0..3 {
        keys.push(taproot_account(n)?.public_key);
    }
    let participants = faraday_bip::musig::sort_keys(&keys);
    let internal = musig_internal(&participants, 0)?.x_only_public_key().0;
    inner.inputs[0].tap_internal_key = Some(internal);
    let aggregate =
        faraday_bip::musig::aggregate_xpub(&participants, NET).map_err(|e| text!("{e:?}"))?;
    let mut origins = BTreeMap::new();
    origins.insert(
        internal,
        (
            Vec::new(),
            (
                aggregate.fingerprint(),
                DerivationPath::from(vec![
                    ChildNumber::from_normal_idx(0).map_err(|e| text!("{e}"))?,
                    ChildNumber::from_normal_idx(0).map_err(|e| text!("{e}"))?,
                ]),
            ),
        ),
    );
    for n in 0..3 {
        let path: DerivationPath = "m/86'/1'/0'".parse().map_err(|e| text!("{e}"))?;
        origins.insert(
            taproot_account(n)?.public_key.x_only_public_key().0,
            (
                Vec::new(),
                (
                    faraday_bip::bitcoin::bip32::Fingerprint::from(master(n)?.fingerprint().0),
                    path,
                ),
            ),
        );
    }
    inner.inputs[0].tap_key_origins = origins;
    let secp = Secp256k1::verification_only();
    let root =
        faraday_bip::musig::key_agg_with(&secp, &participants).map_err(|e| text!("{e:?}"))?;
    faraday_psbt::musig::write_input_participants(
        &mut inner.inputs[0],
        &faraday_psbt::musig::Participants {
            aggregate: root.public_key(),
            keys: participants,
        },
    );
    Ok(Psbt::from(inner))
}
