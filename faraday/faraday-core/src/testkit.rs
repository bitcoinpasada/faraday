//! The test wallets and the files a test stick carries: one wallet of
//! every kind the spend flow signs, each with an unsigned spend, and for
//! every wallet with more than one key a cosigner's signed copy; the three
//! test keys in every form a key arrives in; the wallets in every form a
//! wallet backup takes; and a vault.
//!
//! Everything is on testnet. Every key comes from the three test seeds
//! ([`TEST_SEEDS`]) at coin type 1. Each spend takes 0.1 tBTC from the
//! wallet's first receive address, pays 0.015 tBTC to [`RECIPIENT`] and
//! returns the rest, less the fee, to the wallet's first change address.
//! The funding transaction is made up: its one input spends an outpoint
//! that is the SHA-256 of a fixed phrase. Savings is the 2-of-3 over the
//! seeds `examples/wallet_2of3_demo.rs` uses, at coin type 1 rather
//! than that example's mainnet path; its spend's txid is [`SPEND_TXID`].

use osk_bip::bip39::{Language, Mnemonic};
use osk_bip::bitcoin::absolute::LockTime;
use osk_bip::bitcoin::bip32::{ChildNumber, DerivationPath};
use osk_bip::bitcoin::hashes::{Hash, sha256};
use osk_bip::bitcoin::secp256k1::{Secp256k1, XOnlyPublicKey};
use osk_bip::bitcoin::transaction::Version;
use osk_bip::bitcoin::{
    Address, Amount, OutPoint, ScriptBuf, Sequence, Transaction, TxIn, TxOut, Txid, Witness,
};
use osk_bip::keys::{MasterKey, Network};
use osk_bip::miniscript::descriptor::{DefiniteDescriptorKey, Descriptor, DescriptorPublicKey};
use osk_bip::miniscript::psbt::PsbtExt;
use osk_bip::policy::WalletPolicy;
use osk_psbt::Psbt;

use crate::wallet::Session;

/// The network of every test wallet.
pub const NET: Network = Network::Testnet;

/// The three dummy seeds of the test wallets: the QR test kit's signers
/// 1 to 3, each one word repeated twenty-four times.
pub const TEST_SEEDS: [(&str, &str); 3] = [
    ("bacon", "Test key 1"),
    ("zebra", "Test key 2"),
    ("summer", "Test key 3"),
];

/// The words of a test seed.
pub fn test_words(word: &str) -> String {
    vec![word; 24].join(" ")
}

/// A session on the test network.
pub fn session() -> Session {
    Session::on(NET)
}

/// The Savings spend's txid.
pub const SPEND_TXID: &str = "e65b496fcae9385082643e05b11fac9bd90461bf3c940a805052fb78a673561f";
/// Where every spend pays 0.015 tBTC: the testnet address of BIP-84's
/// first test key.
pub const RECIPIENT: &str = "tb1qcr8te4kr609gcawutmrza0j4xv80jy8zmfp6l0";
/// BIP-341's unspendable internal key, for a wallet that spends only by
/// script.
const NUMS: &str = "50929b74c1a04954b78b4b6035e97a5e078a5a0f28ec96d547bfee9ace803ac0";

/// One test wallet.
pub struct Kit {
    /// The file stem on the test stick and the wallet's short name.
    pub id: &'static str,
    /// The name the wallet is loaded under.
    pub name: &'static str,
    /// The descriptor.
    pub descriptor: String,
    /// Which test seed signs the cosigner's copy, when there is one.
    pub cosigner: Option<usize>,
    /// The fee, in sat.
    pub fee: u64,
}

fn master(word: &str) -> MasterKey {
    let m = Mnemonic::parse(Language::English, &test_words(word)).expect("test words");
    MasterKey::from_seed(&m.to_seed(b"").expect("seed"), NET)
}

/// `[fingerprint/path]tpub` for test seed `n` at `path`.
pub fn key(n: usize, path: &str) -> String {
    let m = master(TEST_SEEDS[n].0);
    let p: DerivationPath = path.parse().expect("path");
    let shown = path.trim_start_matches("m/").replace('\'', "h");
    format!("[{}/{shown}]{}", m.fingerprint(), m.derive(&p).to_xpub())
}

/// The test wallet: a 2-of-3 native SegWit multisig over the three test
/// seeds at BIP-48 account 0, with its checksum.
pub fn savings() -> String {
    let p = "m/48'/1'/0'/2'";
    let d = format!(
        "wsh(sortedmulti(2,{}/<0;1>/*,{}/<0;1>/*,{}/<0;1>/*))",
        key(0, p),
        key(1, p),
        key(2, p)
    );
    WalletPolicy::parse_any(&d)
        .map(|w| w.to_descriptor_checksummed())
        .unwrap_or(d)
}

/// Every test wallet, in the order the session loads them.
pub fn kits() -> Vec<Kit> {
    let k1 = |p: &str| key(0, p);
    let multi = |p: &str| {
        format!(
            "{}/<0;1>/*,{}/<0;1>/*,{}/<0;1>/*",
            key(0, p),
            key(1, p),
            key(2, p)
        )
    };
    vec![
        Kit {
            id: "savings",
            name: "Savings",
            descriptor: savings(),
            cosigner: Some(1),
            fee: 2_140,
        },
        Kit {
            id: "spending",
            name: "Spending",
            descriptor: format!("wpkh({}/<0;1>/*)", k1("m/84'/1'/0'")),
            cosigner: None,
            fee: 1_410,
        },
        Kit {
            id: "taproot",
            name: "Taproot",
            descriptor: format!("tr({}/<0;1>/*)", k1("m/86'/1'/0'")),
            cosigner: None,
            fee: 1_540,
        },
        Kit {
            id: "nested",
            name: "Nested SegWit",
            descriptor: format!("sh(wpkh({}/<0;1>/*))", k1("m/49'/1'/0'")),
            cosigner: None,
            fee: 1_660,
        },
        Kit {
            id: "legacy",
            name: "Legacy",
            descriptor: format!("pkh({}/<0;1>/*)", k1("m/44'/1'/0'")),
            cosigner: None,
            fee: 2_260,
        },
        Kit {
            id: "vault-nested",
            name: "Nested multisig",
            descriptor: format!("sh(wsh(sortedmulti(2,{})))", multi("m/48'/1'/0'/1'")),
            cosigner: Some(1),
            fee: 2_560,
        },
        Kit {
            id: "vault-legacy",
            name: "Legacy multisig",
            descriptor: format!("sh(sortedmulti(2,{}))", multi("m/45'/1'/0'")),
            cosigner: Some(2),
            fee: 3_380,
        },
        Kit {
            id: "taproot-multisig",
            name: "Taproot multisig",
            descriptor: format!("tr({NUMS},sortedmulti_a(2,{}))", multi("m/48'/1'/0'/3'")),
            cosigner: Some(1),
            fee: 1_820,
        },
        Kit {
            id: "inheritance",
            name: "Inheritance",
            descriptor: format!(
                "wsh(or_d(pk({}/<0;1>/*),and_v(v:pkh({}/<0;1>/*),older(52560))))",
                key(0, "m/48'/1'/1'/2'"),
                key(1, "m/48'/1'/1'/2'")
            ),
            cosigner: None,
            fee: 1_980,
        },
        Kit {
            id: "tree",
            name: "Taproot tree",
            descriptor: format!(
                "tr({}/<0;1>/*,{{and_v(v:pk({}/<0;1>/*),older(4320)),pk({}/<0;1>/*)}})",
                key(0, "m/86'/1'/1'"),
                key(1, "m/86'/1'/1'"),
                key(2, "m/86'/1'/1'")
            ),
            cosigner: None,
            fee: 1_540,
        },
    ]
}

/// The wallet at one address, as a coordinator fills a PSBT from it. A
/// `sortedmulti_a` is written as the `multi_a` over its keys in the order
/// the sorted script puts them at this index, since `miniscript` has no
/// sorted form of it.
fn definite(
    policy: &WalletPolicy,
    change: bool,
    index: u32,
) -> Result<Descriptor<DefiniteDescriptorKey>, String> {
    let text = policy.to_descriptor();
    if text.contains("sortedmulti_a(") {
        let secp = Secp256k1::verification_only();
        let steps = [
            ChildNumber::from_normal_idx(u32::from(change)).map_err(|e| e.to_string())?,
            ChildNumber::from_normal_idx(index).map_err(|e| e.to_string())?,
        ];
        let mut keys: Vec<(XOnlyPublicKey, String)> = policy
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
                    format!("{}/{}/{index}", k.key_text(), u32::from(change)),
                )
            })
            .collect();
        keys.sort_by_key(|(d, _)| d.serialize());
        let (m, _) = policy.tapscript_quorum().ok_or("no quorum")?;
        let list: Vec<String> = keys.into_iter().map(|(_, t)| t).collect();
        return format!("tr({NUMS},multi_a({m},{}))", list.join(","))
            .parse()
            .map_err(|e| format!("{e}"));
    }
    let d: Descriptor<DescriptorPublicKey> = text.parse().map_err(|e| format!("{e}"))?;
    let chains = d.into_single_descriptors().map_err(|e| format!("{e}"))?;
    chains[usize::from(change)]
        .clone()
        .at_derivation_index(index)
        .map_err(|e| format!("{e}"))
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

/// The unsigned spend of a test wallet.
pub fn unsigned(kit: &Kit) -> Result<Psbt, String> {
    let policy =
        WalletPolicy::parse_any(&kit.descriptor).map_err(|e| format!("{}: {e}", kit.id))?;
    let input = definite(&policy, false, 0)?;
    let back = definite(&policy, true, 0)?;
    let fund = funding(input.script_pubkey());
    let recipient: Address = RECIPIENT
        .parse::<Address<_>>()
        .map_err(|e| e.to_string())?
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
    let mut psbt = osk_bip::bitcoin::Psbt::from_unsigned_tx(tx).map_err(|e| e.to_string())?;
    let segwit =
        !kit.descriptor.starts_with("pkh(") && !kit.descriptor.starts_with("sh(sortedmulti");
    if segwit {
        psbt.inputs[0].witness_utxo = Some(fund.output[0].clone());
    }
    psbt.inputs[0].non_witness_utxo = Some(fund);
    psbt.update_input_with_descriptor(0, &input)
        .map_err(|e| format!("{}: input: {e:?}", kit.id))?;
    psbt.update_output_with_descriptor(1, &back)
        .map_err(|e| format!("{}: change: {e:?}", kit.id))?;
    Ok(Psbt::from(psbt))
}

/// The unsigned spend of a test wallet and, when it has a cosigner, the
/// same transaction signed by that test key alone.
pub fn spend_of(kit: &Kit) -> Result<(Psbt, Option<(String, Psbt)>), String> {
    let psbt = unsigned(kit)?;
    let Some(n) = kit.cosigner else {
        return Ok((psbt, None));
    };
    let mut session = session();
    let (word, label) = TEST_SEEDS[n];
    let fp = session
        .add_words(&test_words(word), label, None)
        .map_err(|e| e.text())?;
    session
        .add_wallet(kit.name, &kit.descriptor, "Test wallet")
        .map_err(|e| e.text())?;
    let mut signed = psbt.clone();
    let by = session.sign(&mut signed, [0; 32], &mut None)?;
    if by.is_empty() {
        return Err(format!("{}: the cosigner did not sign", kit.id));
    }
    Ok((psbt, Some((crate::wallet::fp_text(fp), signed))))
}

/// The Savings spend and its cosigner's copy.
pub fn spend() -> Result<(Psbt, Psbt), String> {
    let kits = kits();
    let (u, c) = spend_of(&kits[0])?;
    Ok((u, c.ok_or("Savings has a cosigner")?.1))
}

/// The files of the test stick, by name.
pub fn files() -> Result<Vec<(String, Vec<u8>)>, String> {
    let mut out = Vec::new();
    for kit in kits() {
        out.push((
            format!("{}-wallet.txt", kit.id),
            format!(
                "# {}: test wallet over the dummy test seeds\n{}\n",
                kit.name, kit.descriptor
            )
            .into_bytes(),
        ));
        // The same wallet as a coordinator's export: the wallet .json
        // Sparrow imports, and a multisig's setup file.
        let policy = WalletPolicy::parse_any(&kit.descriptor).map_err(|e| e.to_string())?;
        out.push((
            format!("{}-wallet.json", kit.id),
            crate::backup::wallet_json(kit.name, &policy).into_bytes(),
        ));
        let mut one = session();
        one.add_wallet(kit.name, &kit.descriptor, "Test wallet")
            .map_err(|e| e.text())?;
        if let Some(text) = crate::backup::multisig_config(&one.wallets[0], None) {
            out.push((format!("{}-multisig-setup.txt", kit.id), text.into_bytes()));
        }
        let (u, c) = spend_of(&kit)?;
        out.push((format!("{}-unsigned.psbt", kit.id), u.to_bytes()));
        if let Some((fp, signed)) = c {
            out.push((format!("{}-cosigner-{fp}.psbt", kit.id), signed.to_bytes()));
        }
    }
    // The Inheritance wallet spent down its recovery path: the sequence
    // its older(52560) wants, and only key 2's origin, as a coordinator
    // building that spend hands it over.
    if let Some(kit) = kits().into_iter().find(|k| k.id == "inheritance") {
        out.push((
            "inheritance-recovery-unsigned.psbt".to_string(),
            recovery(&kit)?.to_bytes(),
        ));
    }
    // MuSig2: the wallet, the spend, and each round's copies from test
    // keys 2 and 3.
    let policy = musig_policy()?;
    out.push((
        "musig-wallet.txt".to_string(),
        format!(
            "# MuSig2: 3 of 3 over the dummy test seeds\n{}\n",
            policy.to_descriptor_checksummed()
        )
        .into_bytes(),
    ));
    let (mu, nonces, partials) = musig_rounds()?;
    out.push(("musig-unsigned.psbt".to_string(), mu.to_bytes()));
    for (fp, p) in nonces {
        out.push((format!("musig-nonce-{fp}.psbt"), p.to_bytes()));
    }
    for (fp, p) in partials {
        out.push((format!("musig-partial-{fp}.psbt"), p.to_bytes()));
    }
    // FROST: the group record, the spend, and the carry file test key 1
    // leaves for test key 2.
    out.push((
        "threshold-wallet.txt".to_string(),
        format!(
            "# Threshold: FROST 2 of 3, shares 1 and 2 are test keys 1 and 2\n{}\n",
            threshold_record()?.to_text()
        )
        .into_bytes(),
    ));
    out.push((
        "threshold-unsigned.psbt".to_string(),
        threshold_unsigned()?.to_bytes(),
    ));
    out.push((
        "threshold-partly-signed.osk".to_string(),
        threshold_carry()?,
    ));
    out.push(("vault.ofv".to_string(), test_vault()?));
    out.push((
        "vault-entries.txt".to_string(),
        ENTRIES_FILE.as_bytes().to_vec(),
    ));
    out.push(("BOOTX64.EFI".to_string(), test_efi()));
    // Savings split between its signers, one key left off each share.
    let mut one = session();
    one.add_wallet("Savings", &savings(), "Test wallet")
        .map_err(|e| e.text())?;
    let w = &one.wallets[0];
    let plan = crate::backup::split_plan(3, 2, 1);
    for (i, row) in plan.iter().enumerate() {
        if let Some(text) = crate::backup::multisig_config(w, Some(row)) {
            out.push((
                format!("savings-share-{}-of-3.txt", i + 1),
                text.into_bytes(),
            ));
        }
    }
    // Wallet descriptors as QR codes saved as pictures, the way a phone
    // or a coordinator's export leaves one on a stick.
    for id in ["savings", "spending", "nested"] {
        if let Some(kit) = kits().into_iter().find(|k| k.id == id) {
            let m = osk_codec::qr::encode(
                osk_codec::qr::Payload::Bytes(kit.descriptor.as_bytes()),
                osk_codec::qr::Ecc::Medium,
            )
            .map_err(|e| format!("{e:?}"))?;
            out.push((format!("{id}-wallet-qr.png"), osk_codec::png::qr_png(&m, 6)));
        }
    }
    // The three test seeds in every form a seed comes in, and their
    // account xpubs.
    for (n, (word, label)) in TEST_SEEDS.iter().enumerate() {
        let fp = crate::wallet::fp_text(master(word).fingerprint());
        let stem = format!("seed-{}-{fp}", n + 1);
        let xpub_stem = format!("xpub-{}-{fp}", n + 1);
        let m =
            Mnemonic::parse(Language::English, &test_words(word)).map_err(|e| format!("{e:?}"))?;
        out.push((
            format!("{stem}-words.txt"),
            format!(
                "# {label}: dummy words, typed in Add a key\n{}\n",
                test_words(word)
            )
            .into_bytes(),
        ));
        let qr = osk_codec::seedqr::encode_seedqr(&m).map_err(|e| format!("{e:?}"))?;
        out.push((format!("{stem}-seedqr.png"), osk_codec::png::qr_png(&qr, 8)));
        let qr = osk_codec::seedqr::encode_compact(&m).map_err(|e| format!("{e:?}"))?;
        out.push((
            format!("{stem}-compactseedqr.png"),
            osk_codec::png::qr_png(&qr, 8),
        ));
        out.push((format!("{stem}.oskb"), test_backup(n)?));
        // The account xpubs a cosigner's device exports.
        for (what, path) in [
            ("multisig", "m/48'/1'/0'/2'"),
            ("nested-multisig", "m/48'/1'/0'/1'"),
            ("taproot-multisig", "m/48'/1'/0'/3'"),
            ("single", "m/84'/1'/0'"),
        ] {
            out.push((
                format!("{xpub_stem}-{what}.txt"),
                format!("# {label}, {what} account xpub\n{}\n", key(n, path)).into_bytes(),
            ));
        }
    }
    // A signed message to check: the Spending wallet's first address,
    // BIP-322.
    let mut one = session();
    let (word, label) = TEST_SEEDS[0];
    one.add_words(&test_words(word), label, None)
        .map_err(|e| e.text())?;
    let spending = kits()
        .into_iter()
        .find(|k| k.id == "spending")
        .ok_or("no Spending kit")?;
    let i = one
        .add_wallet(spending.name, &spending.descriptor, "Test wallet")
        .map_err(|e| e.text())?;
    let text = "Faraday test message: this address belongs to test key 1.";
    let signed = one.sign_message(i, 0, text, osk_psbt::message::Format::Bip322)?;
    out.push((
        "spending-message.txt".to_string(),
        osk_psbt::message::signed_text(&signed.address, &signed.signature, text).into_bytes(),
    ));
    out.push(("README.txt".to_string(), readme(&out).into_bytes()));
    Ok(out)
}

/// The stick's index of its own files.
fn readme(files: &[(String, Vec<u8>)]) -> String {
    let mut s = String::from(
        "Faraday test stick. Testnet only; every key is a dummy.\n\n\
         Keys: test key 1 bacon, 2 zebra, 3 summer (each word 24 times).\n\
         Vault vault.ofv: passphrase \"test vault\" (test key 1 and every wallet), \
         \"decoy\" (a second slot).\n\
         .oskb backups: passphrase \"backup\".\n\n",
    );
    for (name, bytes) in files {
        s.push_str(&format!("{name}  {} bytes\n", bytes.len()));
    }
    s
}

/// A spend of a miniscript wallet down its timelocked path: the first
/// input's sequence set to the wait, and the first key's origin removed,
/// so only the recovery key can sign.
pub fn recovery(kit: &Kit) -> Result<Psbt, String> {
    let mut inner = unsigned(kit)?.into_inner();
    inner.unsigned_tx.input[0].sequence = Sequence::from_height(52_560);
    let first = master(TEST_SEEDS[0].0).fingerprint();
    inner.inputs[0]
        .bip32_derivation
        .retain(|_, (fp, _)| fp.to_bytes() != first.0);
    Ok(Psbt::from(inner))
}

/// The MuSig2 test wallet: one aggregate key over the three test seeds'
/// Taproot account keys, every participant signs (BIP-390).
pub fn musig_policy() -> Result<WalletPolicy, String> {
    let texts: Vec<String> = TEST_SEEDS
        .iter()
        .map(|(w, _)| {
            key(
                TEST_SEEDS.iter().position(|(x, _)| x == w).unwrap_or(0),
                "m/86'/1'/0'",
            )
        })
        .collect();
    let refs: Vec<&str> = texts.iter().map(String::as_str).collect();
    WalletPolicy::from_parts("tr(musig(@0,@1,@2)/**)", &refs).map_err(|e| e.to_string())
}

fn taproot_account(n: usize) -> osk_bip::bitcoin::bip32::Xpub {
    *master(TEST_SEEDS[n].0)
        .account_xpub(osk_bip::keys::ScriptType::Taproot, 0)
        .expect("account")
        .xpub()
}

/// The tweaks from the root aggregate to the output key at chain/index,
/// and the internal key on the way (`osk-psbt`'s MuSig2 test, for three).
fn musig_tweaks(
    participants: &[osk_bip::bitcoin::secp256k1::PublicKey],
    change: bool,
    index: u32,
) -> Result<
    (
        Vec<osk_bip::musig::Tweak>,
        osk_bip::bitcoin::secp256k1::PublicKey,
    ),
    String,
> {
    use osk_bip::bitcoin::taproot::TapTweakHash;
    let secp = Secp256k1::verification_only();
    let mut xpub =
        osk_bip::musig::aggregate_xpub(participants, NET).map_err(|e| format!("{e:?}"))?;
    let mut tweaks = Vec::new();
    for step in [u32::from(change), index] {
        let child = ChildNumber::from_normal_idx(step).map_err(|e| e.to_string())?;
        let (tweak, _) = xpub.ckd_pub_tweak(child).map_err(|e| e.to_string())?;
        tweaks.push(osk_bip::musig::Tweak {
            bytes: tweak.secret_bytes(),
            x_only: false,
        });
        xpub = xpub.ckd_pub(&secp, child).map_err(|e| e.to_string())?;
    }
    let internal = xpub.public_key;
    tweaks.push(osk_bip::musig::Tweak {
        bytes: TapTweakHash::from_key_and_tweak(internal.x_only_public_key().0, None)
            .to_byte_array(),
        x_only: true,
    });
    Ok((tweaks, internal))
}

/// An unsigned spend of the MuSig2 wallet: 0.1 BTC in at its first
/// address, everything less the fee out to the recipient, with the fields
/// a coordinator that speaks BIP-373 fills in.
pub fn musig_unsigned() -> Result<Psbt, String> {
    use std::collections::BTreeMap;
    let policy = musig_policy()?;
    let spk = policy.script_at(false, 0).map_err(|e| e.to_string())?;
    let fund = funding(spk);
    let recipient: Address = RECIPIENT
        .parse::<Address<_>>()
        .map_err(|e| e.to_string())?
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
    let mut inner = osk_bip::bitcoin::Psbt::from_unsigned_tx(tx).map_err(|e| e.to_string())?;
    inner.inputs[0].witness_utxo = Some(fund.output[0].clone());
    let keys: Vec<_> = (0..3).map(|n| taproot_account(n).public_key).collect();
    let participants = osk_bip::musig::sort_keys(&keys);
    let (_, internal) = musig_tweaks(&participants, false, 0)?;
    let internal = internal.x_only_public_key().0;
    inner.inputs[0].tap_internal_key = Some(internal);
    let aggregate =
        osk_bip::musig::aggregate_xpub(&participants, NET).map_err(|e| format!("{e:?}"))?;
    let mut origins = BTreeMap::new();
    origins.insert(
        internal,
        (
            Vec::new(),
            (
                aggregate.fingerprint(),
                DerivationPath::from(vec![
                    ChildNumber::from_normal_idx(0).map_err(|e| e.to_string())?,
                    ChildNumber::from_normal_idx(0).map_err(|e| e.to_string())?,
                ]),
            ),
        ),
    );
    for (n, (word, _)) in TEST_SEEDS.iter().enumerate() {
        let m = master(word);
        origins.insert(
            taproot_account(n).public_key.x_only_public_key().0,
            (
                Vec::new(),
                (
                    osk_bip::bitcoin::bip32::Fingerprint::from(m.fingerprint().0),
                    "m/86'/1'/0'"
                        .parse::<DerivationPath>()
                        .map_err(|e| format!("{e}"))?,
                ),
            ),
        );
    }
    inner.inputs[0].tap_key_origins = origins;
    let secp = Secp256k1::verification_only();
    let root = osk_bip::musig::key_agg_with(&secp, &participants).map_err(|e| format!("{e:?}"))?;
    osk_psbt::musig::write_input_participants(
        &mut inner.inputs[0],
        &osk_psbt::musig::Participants {
            aggregate: root.public_key(),
            keys: participants,
        },
    );
    Ok(Psbt::from(inner))
}

/// The key-path sighash of the first input.
fn key_path_sighash(psbt: &Psbt) -> Result<[u8; 32], String> {
    use osk_bip::bitcoin::sighash::{Prevouts, SighashCache, TapSighashType};
    let inner = psbt.inner();
    let prev = inner.inputs[0]
        .witness_utxo
        .clone()
        .ok_or("no witness utxo")?;
    let mut cache = SighashCache::new(&inner.unsigned_tx);
    let h = cache
        .taproot_key_spend_signature_hash(0, &Prevouts::All(&[prev]), TapSighashType::Default)
        .map_err(|e| e.to_string())?;
    Ok(h.to_byte_array())
}

/// The MuSig2 rounds as files: the unsigned spend; the copies on which
/// test keys 2 and 3 each put their round-one nonce; and the copies on
/// which they put their round-two partial signature, over the nonce test
/// key 1 adds when it signs last. Test key 1's own pass is deterministic,
/// so the partials made here fit the app's.
#[allow(clippy::type_complexity)]
pub fn musig_rounds() -> Result<(Psbt, Vec<(String, Psbt)>, Vec<(String, Psbt)>), String> {
    let unsigned = musig_unsigned()?;
    let policy = musig_policy()?;
    let secp = osk_bip::bitcoin::secp256k1::Secp256k1::new();
    // What the inspector establishes: output key and tweaks.
    let mut session = session();
    session
        .add_words(&test_words(TEST_SEEDS[0].0), TEST_SEEDS[0].1, None)
        .map_err(|e| e.text())?;
    session
        .add_wallet("MuSig2", &policy.to_descriptor_checksummed(), "Test wallet")
        .map_err(|e| e.text())?;
    let established = session.inspect(&unsigned).inputs[0]
        .musig
        .clone()
        .ok_or("no MuSig2 input")?;
    let msg = key_path_sighash(&unsigned)?;
    let agg32 = established.output_key.x_only_public_key().0.serialize();
    let mut secnonces = Vec::new();
    let mut nonce_files = Vec::new();
    let mut with_nonces = unsigned.clone();
    for n in [1usize, 2] {
        let m = master(TEST_SEEDS[n].0);
        let sk = *m
            .derive(&"m/86'/1'/0'".parse().map_err(|e| format!("{e}"))?)
            .secret_key();
        let pk = taproot_account(n).public_key;
        let (sec, pubn) = osk_bip::musig::nonce_gen(
            &secp,
            &[0x40 + n as u8; 32],
            Some(&sk),
            &pk,
            Some(&agg32),
            Some(&msg),
            None,
        )
        .map_err(|e| format!("{e:?}"))?;
        let mut copy = unsigned.clone();
        osk_psbt::musig::write_pub_nonce(
            &mut copy.inner_mut().inputs[0],
            &pk,
            &established.output_key,
            &pubn,
        );
        osk_psbt::musig::write_pub_nonce(
            &mut with_nonces.inner_mut().inputs[0],
            &pk,
            &established.output_key,
            &pubn,
        );
        nonce_files.push((crate::wallet::fp_text(m.fingerprint()), copy));
        secnonces.push((n, sec, sk, pk));
    }
    // Test key 1 signs last, twice with different seeds: the same bytes.
    let mut device = with_nonces.clone();
    let mut again = with_nonces.clone();
    session.sign(&mut device, [7; 32], &mut None)?;
    session.sign(&mut again, [9; 32], &mut None)?;
    if device.to_bytes() != again.to_bytes() {
        return Err("the last signer's MuSig2 pass depends on the session seed".into());
    }
    // Round two for keys 2 and 3, over everyone's nonces.
    let nonces =
        osk_psbt::musig::pub_nonces(&device.inner().inputs[0]).map_err(|e| format!("{e:?}"))?;
    let all: Vec<_> = established
        .participants
        .iter()
        .map(|p| {
            nonces
                .iter()
                .find(|x| x.participant == *p)
                .map(|x| x.value)
                .ok_or("a nonce is missing")
        })
        .collect::<Result<_, _>>()?;
    let aggnonce = osk_bip::musig::nonce_agg(&all).map_err(|e| format!("{e:?}"))?;
    let values = osk_bip::musig::session_values(
        &secp,
        &established.participants,
        &established.tweaks,
        &aggnonce,
        &msg,
    )
    .map_err(|e| format!("{e:?}"))?;
    let mut partial_files = Vec::new();
    for (n, sec, sk, pk) in secnonces {
        let psig = osk_bip::musig::sign(&secp, sec, &sk, &values).map_err(|e| format!("{e:?}"))?;
        let mut copy = device.clone();
        osk_psbt::musig::write_partial_sig(
            &mut copy.inner_mut().inputs[0],
            &pk,
            &established.output_key,
            &psig,
        );
        partial_files.push((
            crate::wallet::fp_text(master(TEST_SEEDS[n].0).fingerprint()),
            copy,
        ));
    }
    Ok((unsigned, nonce_files, partial_files))
}

/// The FROST test wallet: 2 of 3, dealt here from test keys 1 and 2's
/// words as shares 1 and 2 (`docs/PLANNING.md` §16.103); share 3 is
/// computed. Its record is public and is the wallet's file.
pub fn threshold_record() -> Result<osk_bip::threshold::ThresholdRecord, String> {
    let secp = Secp256k1::new();
    let mut chosen = Vec::new();
    for (id, (word, _)) in TEST_SEEDS.iter().take(2).enumerate() {
        let mut session = session();
        session
            .add_words(&test_words(word), "", None)
            .map_err(|e| e.text())?;
        let mut share = None;
        session.keys[0].with_secret_share(&mut |s| {
            share = osk_bip::frost::SecShare::from_bytes(&s.secret_bytes()).ok();
        });
        chosen.push((id as u32, share.ok_or("a test seed is not a share")?));
    }
    let dealt = osk_bip::frost::deal(&secp, 3, 2, &chosen).map_err(|e| format!("{e:?}"))?;
    Ok(osk_bip::threshold::ThresholdRecord::new(
        dealt.info,
        NET.kind(),
    ))
}

/// The FROST test wallet as a kit, for its spend.
fn threshold_kit() -> Result<Kit, String> {
    Ok(Kit {
        id: "threshold",
        name: "Threshold",
        descriptor: threshold_record()?.to_text(),
        cosigner: None,
        fee: 1540,
    })
}

/// The FROST wallet's unsigned spend.
pub fn threshold_unsigned() -> Result<Psbt, String> {
    unsigned(&threshold_kit()?)
}

/// The first location's pass on the FROST spend: test key 1 signs as
/// share 1 and chooses share 2; the carry file it leaves for share 2.
pub fn threshold_carry() -> Result<Vec<u8>, String> {
    let mut session = session();
    session
        .add_words(&test_words(TEST_SEEDS[0].0), TEST_SEEDS[0].1, None)
        .map_err(|e| e.text())?;
    session
        .add_wallet("Threshold", &threshold_record()?.to_text(), "Test wallet")
        .map_err(|e| e.text())?;
    let mut psbt = threshold_unsigned()?;
    let signed = session.sign_with(&mut psbt, [5; 32], &mut None, &[1], None)?;
    let section = signed
        .carry
        .ok_or("the first location left no carry section")?;
    Ok(osk_psbt::threshold::carry_bytes(&section, &psbt.to_bytes()))
}

/// The backup test stick's vault passphrase.
pub const BACKUP_VAULT_PASSPHRASE: &str = "a";

/// The backup test stick (2026-10-06): only what backing up the 2-of-3
/// Taproot multisig over the three test seeds leaves, as a person would
/// carry it. On the stick, every public file Faraday's Backup step
/// writes for that wallet (the descriptor, the multisig config, the
/// split shares, the wallet .json, the BIP 129 record, Bitcoin Core's
/// import file, the backup sheet and the blank template as PDFs);
/// `vault.ofv`, whose passphrase is `a`, holding the three seeds, each
/// loaded at unlock; and an unsigned spend from the wallet. All testnet.
/// The full kit is [`files`].
pub fn backup_files() -> Result<Vec<(String, Vec<u8>)>, String> {
    let kit = kits()
        .into_iter()
        .find(|k| k.id == "taproot-multisig")
        .ok_or("no Taproot multisig kit")?;
    let mut app = crate::Faraday::new();
    app.session = session();
    let w = app
        .session
        .add_wallet(kit.name, &kit.descriptor, "Test wallet")
        .map_err(|e| e.text())?;
    for what in 0..=7 {
        app.public_out(w, what);
    }
    let mut out: Vec<(String, Vec<u8>)> = app
        .outbox
        .iter()
        .map(|i| (i.name.clone(), i.bytes.clone()))
        .collect();
    out.push(("vault.ofv".to_string(), seeds_vault()?));
    // A spend to try the Spend tab and Wallets on: what a coordinator
    // such as Sparrow hands over.
    let (unsigned, _) = spend_of(&kit)?;
    out.push((
        "taproot-multisig-unsigned.psbt".to_string(),
        unsigned.to_bytes(),
    ));
    out.push((
        "README.txt".to_string(),
        format!(
            "Faraday backup test stick, testnet. Dummy seeds: nothing here holds value.\n\n\
             The backup of {}, a 2-of-3 Taproot multisig over the test seeds bacon, zebra and \
             summer (each word repeated 24 times): its public files, and vault.ofv, whose \
             passphrase is a, holding the three seeds. And \
             taproot-multisig-unsigned.psbt, an unsigned spend from it, as a coordinator \
             hands one over.\n",
            kit.name
        )
        .into_bytes(),
    ));
    Ok(out)
}

/// The backup test stick's vault: the three test seeds, loaded at
/// unlock, under one passphrase at the Light cost.
fn seeds_vault() -> Result<Vec<u8>, String> {
    use faraday_vault::records::{self, field, kind};
    use faraday_vault::{Contents, Cost, Record};
    let mut main = Contents {
        records: vec![Record::new(kind::SLOT_LABEL).with(field::LABEL, b"Test seeds")],
    };
    for (word, label) in TEST_SEEDS {
        let m =
            Mnemonic::parse(Language::English, &test_words(word)).map_err(|e| format!("{e:?}"))?;
        main.records.push(
            Record::new(kind::KEY)
                .with(field::KEY, &records::words_payload(&m))
                .with(field::KEY_LABEL, label.as_bytes())
                .with(field::KEY_FLAGS, &[records::LOAD_AT_UNLOCK]),
        );
    }
    let cost = Cost {
        memory_kib: 64 * 1024,
        passes: 3,
        lanes: 1,
    };
    faraday_vault::create(
        faraday_vault::SLOT_SIZES[0],
        cost,
        &[BACKUP_VAULT_PASSPHRASE.as_bytes()],
        &[main],
        &[0x5b; 32],
    )
    .map_err(|e| e.reason())
}

/// The test vault's passphrase, and the decoy passphrase that opens its
/// second slot.
pub const VAULT_PASSPHRASES: [&str; 2] = ["test vault", "decoy"];

/// The test stick's vault (`docs/VAULT.md`): two passphrases at the
/// Light cost with 64 KiB slots. The first opens test key 1 (loaded at
/// unlock), every test wallet, an entry and a note; the second opens a
/// slot with one entry. Everything is a dummy.
pub fn test_vault() -> Result<Vec<u8>, String> {
    use faraday_vault::records::{self, field, kind};
    use faraday_vault::{Contents, Cost, Record};
    let m = Mnemonic::parse(Language::English, &test_words(TEST_SEEDS[0].0))
        .map_err(|e| format!("{e:?}"))?;
    let mut wallets: Vec<(String, String)> = kits()
        .into_iter()
        .map(|k| (k.name.to_string(), k.descriptor))
        .collect();
    wallets.push((
        "MuSig2".to_string(),
        musig_policy()?.to_descriptor_checksummed(),
    ));
    wallets.push(("Threshold".to_string(), threshold_record()?.to_text()));
    let mut main = Contents {
        records: vec![
            Record::new(kind::SLOT_LABEL).with(field::LABEL, b"Test vault"),
            Record::new(kind::KEY)
                .with(field::KEY, &records::words_payload(&m))
                .with(field::KEY_LABEL, TEST_SEEDS[0].1.as_bytes())
                .with(field::KEY_FLAGS, &[records::LOAD_AT_UNLOCK]),
            Record::new(kind::ENTRY)
                .with(field::TITLE, b"Email")
                .with(field::USERNAME, b"you@example.com")
                .with(field::PASSWORD, b"correct-horse-battery-staple")
                .with(field::URL, b"https://mail.example.com")
                .with(
                    field::TOTP,
                    b"otpauth://totp/Example:you?secret=JBSWY3DPEHPK3PXP",
                ),
            Record::new(kind::NOTE).with(field::NOTE, b"Recovery codes\n1111-2222\n3333-4444"),
        ],
    };
    for (name, text) in &wallets {
        main.records.push(
            Record::new(kind::WALLET)
                .with(field::WALLET, text.as_bytes())
                .with(field::WALLET_NAME, name.as_bytes()),
        );
    }
    let decoy = Contents {
        records: vec![
            Record::new(kind::SLOT_LABEL).with(field::LABEL, b"Everyday"),
            Record::new(kind::ENTRY)
                .with(field::TITLE, b"Library card")
                .with(field::PASSWORD, b"1234"),
        ],
    };
    let cost = Cost {
        memory_kib: 64 * 1024,
        passes: 3,
        lanes: 1,
    };
    faraday_vault::create(
        faraday_vault::SLOT_SIZES[0],
        cost,
        &[
            VAULT_PASSPHRASES[0].as_bytes(),
            VAULT_PASSPHRASES[1].as_bytes(),
        ],
        &[main, decoy],
        &[0x7a; 32],
    )
    .map_err(|e| e.reason())
}

/// A text file of entries for a vault: a TOTP setup URI and an entry in
/// `field: value` lines (`PLAN.md` §6.4).
pub const ENTRIES_FILE: &str = "otpauth://totp/Example%20Git:octo?secret=JBSWY3DPEHPK3PXP&issuer=Example%20Git\n\
\n\
title: Router admin\n\
username: admin\n\
password: router-password\n\
url: http://192.168.1.1\n";

/// The passphrase of the test stick's OpenSigner backup.
pub const BACKUP_PASSPHRASE: &str = "backup";

/// Test key `n`'s words as an OpenSigner encrypted backup, at Argon2id's
/// smallest cost: a dummy.
pub fn test_backup(n: usize) -> Result<Vec<u8>, String> {
    let m = Mnemonic::parse(Language::English, &test_words(TEST_SEEDS[n].0))
        .map_err(|e| format!("{e:?}"))?;
    let mut seed = [0u8; osk_backup::oskb::SEED_LEN];
    for (i, b) in seed.iter_mut().enumerate() {
        *b = i as u8;
    }
    osk_backup::oskb::seal(
        &m,
        BACKUP_PASSPHRASE.as_bytes(),
        osk_backup::MIN_PARAMS,
        &seed,
    )
    .ok_or_else(|| "the backup would not seal".to_string())
}

/// A minimal PE32+ EFI application, for Secure Boot signing: headers and
/// one section holding a return. It does nothing if started.
pub fn test_efi() -> Vec<u8> {
    let mut pe = vec![0u8; 0x400];
    let put16 =
        |pe: &mut Vec<u8>, at: usize, v: u16| pe[at..at + 2].copy_from_slice(&v.to_le_bytes());
    let put32 =
        |pe: &mut Vec<u8>, at: usize, v: u32| pe[at..at + 4].copy_from_slice(&v.to_le_bytes());
    pe[0..2].copy_from_slice(b"MZ");
    put32(&mut pe, 0x3c, 0x40);
    pe[0x40..0x44].copy_from_slice(b"PE\0\0");
    let coff = 0x44;
    put16(&mut pe, coff, 0x8664);
    put16(&mut pe, coff + 2, 1);
    put16(&mut pe, coff + 16, 240);
    put16(&mut pe, coff + 18, 0x22);
    let opt = coff + 20;
    put16(&mut pe, opt, 0x20b);
    put32(&mut pe, opt + 16, 0x1000);
    put32(&mut pe, opt + 32, 0x1000);
    put32(&mut pe, opt + 36, 0x200);
    put32(&mut pe, opt + 56, 0x2000);
    put32(&mut pe, opt + 60, 0x200);
    put16(&mut pe, opt + 68, 10);
    put32(&mut pe, opt + 108, 16);
    let sec = opt + 240;
    pe[sec..sec + 5].copy_from_slice(b".text");
    put32(&mut pe, sec + 8, 0x10);
    put32(&mut pe, sec + 12, 0x1000);
    put32(&mut pe, sec + 16, 0x200);
    put32(&mut pe, sec + 20, 0x200);
    put32(&mut pe, sec + 36, 0x6000_0020);
    pe[0x200] = 0xc3;
    pe
}
