//! The test wallets and the files a test stick carries: one wallet of
//! every kind the spend flow signs, each with an unsigned spend, and for
//! every wallet with more than one key a cosigner's signed copy; the three
//! test keys in every form a key arrives in; the wallets in every form a
//! wallet backup takes; and a vault.
//!
//! Everything is on testnet ([`NET`]) unless a tool chose Mainnet for
//! its process ([`use_network`]): then every key is at coin type 0 and
//! every file is as a Mainnet stick holds it. Every key comes from the
//! three test seeds ([`TEST_SEEDS`]) at coin type 1 (0 on Mainnet). Each
//! spend takes 0.1 tBTC from the
//! wallet's first receive address, pays 0.015 tBTC to [`RECIPIENT`] and
//! returns the rest, less the fee, to the wallet's first change address.
//! The funding transaction is made up: its one input spends an outpoint
//! that is the SHA-256 of a fixed phrase. Savings is the 2-of-3 over the
//! seeds `examples/wallet_2of3_demo.rs` uses, at coin type 1 rather
//! than that example's mainnet path; its spend's txid is [`SPEND_TXID`].

pub use crate::fresh::roller::{five, five_screen};
use faraday_bip::bip39::{Language, Mnemonic};
use faraday_bip::bitcoin::absolute::LockTime;
use faraday_bip::bitcoin::bip32::{ChildNumber, DerivationPath};
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

use crate::wallet::Session;

/// The network of every test wallet, unless a tool chose another for
/// its process ([`use_network`]): the tests' and the default tours'.
pub const NET: Network = Network::Testnet;

/// The network a tool chose for the test wallets, once, at its start.
static CHOSEN: std::sync::OnceLock<Network> = std::sync::OnceLock::new();

/// Makes every test wallet, key and file of this process on `network`:
/// the testkit generator's and the snapshot tool's `--network`, before
/// either makes anything. Refused (with the network already in use) when
/// another was chosen before.
pub fn use_network(network: Network) -> Result<(), Network> {
    let chosen = *CHOSEN.get_or_init(|| network);
    if chosen == network {
        Ok(())
    } else {
        Err(chosen)
    }
}

/// The network the test wallets are made on: [`NET`] unless a tool
/// chose another ([`use_network`]).
pub fn net() -> Network {
    CHOSEN.get().copied().unwrap_or(NET)
}

/// `path`, an account path written at coin type 1 as this kit writes
/// them (`m/48'/1'/0'/2'`), at [`net`]'s coin type: unchanged on the test
/// networks, `m/48'/0'/0'/2'` on Mainnet.
pub fn on_net(path: &str) -> String {
    let coin = net().coin_type();
    path.split('/')
        .enumerate()
        .map(|(i, step)| match (i, step) {
            (2, "1'") => format!("{coin}'"),
            (2, "1h") => format!("{coin}h"),
            _ => step.to_string(),
        })
        .collect::<Vec<_>>()
        .join("/")
}

/// The three dummy seeds of the test wallets: the QR test kit's signers
/// 1 to 3, each one word repeated twenty-four times.
pub const TEST_SEEDS: [(&str, &str); 3] = [
    ("bacon", "Test key 1"),
    ("zebra", "Test key 2"),
    ("summer", "Test key 3"),
];

/// A token for a test's costly work, counted as the app counts it
/// (`docs/COMPUTE.md` §3). Cheap.
#[expect(
    clippy::disallowed_methods,
    reason = "a mint: Faraday's tests (docs/COMPUTE.md §3)"
)]
pub fn compute() -> faraday_crypto::work::Compute {
    faraday_crypto::work::Compute::new(crate::work::core_count)
}

/// The words of a test seed.
pub fn test_words(word: &str) -> String {
    vec![word; 24].join(" ")
}

/// Moves `session` to `network` as a shell's start does (no press of a
/// person reaches this since F81: a mode change is a lock). Refused when
/// wallets are loaded and `network` is of another kind, or the network
/// is fixed.
pub fn move_to(session: &mut Session, network: Network) -> Result<(), crate::wallet::Refusal> {
    session
        .keys
        .set_network(network, !session.wallets.is_empty())
}

/// A session on the test wallets' network ([`net`]).
pub fn session() -> Session {
    Session::on(net())
}

/// The words samples made of one word, with how many times it is
/// repeated: the handouts of `docs/MODES.md` §5.1 (the same list as
/// `faraday-samples`, whose tests check the two agree).
pub const SAMPLE_WORDS: [(&str, u8); 21] = [
    ("bacon", 24),
    ("flag", 24),
    ("gas", 24),
    ("great", 24),
    ("slice", 24),
    ("solution", 24),
    ("summer", 24),
    ("they", 24),
    ("trade", 24),
    ("trap", 24),
    ("zebra", 24),
    ("twelve", 12),
    ("ocean", 12),
    ("piano", 12),
    ("silver", 12),
    ("lucky", 15),
    ("olive", 15),
    ("glad", 18),
    ("school", 18),
    ("brave", 21),
    ("sunset", 21),
];

/// Costs a PBKDF2 and a master key for each of 42 fingerprints, once per
/// process. The public set Demo takes in the core's tests: the handouts
/// of [`SAMPLE_WORDS`] only, derived here so core's tests do not link
/// the samples crate. The data a shell ships is `faraday-samples`'.
pub fn demo_samples() -> &'static crate::samples::Samples {
    use crate::samples::{SampleForm, SampleInputs, SampleKey, Samples};
    static SAMPLES: std::sync::OnceLock<Samples> = std::sync::OnceLock::new();
    SAMPLES.get_or_init(|| {
        let c = compute();
        let fp = |words: &str, passphrase: &str| {
            faraday_keys::identity::fingerprint_of_words(&c, words, passphrase)
                .expect("sample words")
                .0
        };
        let keys: Vec<SampleKey> = SAMPLE_WORDS
            .iter()
            .map(|&(word, count)| {
                let phrase: &'static str =
                    Box::leak(crate::samples::repeated(word, count).into_boxed_str());
                SampleKey {
                    name: "sample_words",
                    word,
                    count,
                    form: SampleForm::Words,
                    listed: true,
                    parts: Box::leak(vec![phrase].into_boxed_slice()),
                    combined: "",
                    plain: Some(fp(phrase, "")),
                    size: crate::samples::SampleSize::Words(count),
                    with_passphrase: Some(fp(phrase, crate::samples::PASSPHRASE)),
                    node: None,
                }
            })
            .collect();
        Samples {
            keys: Box::leak(keys.into_boxed_slice()),
            wallets: &[],
            inputs: SampleInputs {
                vault: &[],
                signed_message: "",
                address: "",
                totp: "",
            },
        }
    })
}

/// The Savings spend's txid on the test network ([`NET`]).
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
    let c = compute();
    let m = Mnemonic::parse(&c, Language::English, &test_words(word)).expect("test words");
    MasterKey::from_seed(&c, &m.to_seed(&c, b"").expect("seed"), net())
}

/// Whether session key `i` has test seed `n`'s words, with or without a
/// passphrase.
pub fn has_test_words(keys: &faraday_keys::Keys, i: usize, n: usize) -> bool {
    let words = keys.reveal(i, faraday_keys::Reveal::Words);
    !words.is_empty() && *words == *test_words(TEST_SEEDS[n].0)
}

/// `[fingerprint/path]tpub` for test seed `n` at `path`, its coin type
/// [`net`]'s ([`on_net`]): an `xpub` at coin type 0 on Mainnet.
pub fn key(n: usize, path: &str) -> String {
    let c = compute();
    let m = master(TEST_SEEDS[n].0);
    let path = on_net(path);
    let p: DerivationPath = path.parse().expect("path");
    let shown = path.trim_start_matches("m/").replace('\'', "h");
    format!(
        "[{}/{shown}]{}",
        m.fingerprint(),
        m.derive(&c, &p).to_xpub(&c)
    )
}

/// The test wallet: a 2-of-3 native SegWit multisig over the three test
/// seeds at BIP-48 account 0, with its checksum.
pub fn savings() -> String {
    let c = compute();
    let p = "m/48'/1'/0'/2'";
    let d = format!(
        "wsh(sortedmulti(2,{}/<0;1>/*,{}/<0;1>/*,{}/<0;1>/*))",
        key(0, p),
        key(1, p),
        key(2, p)
    );
    WalletPolicy::parse_any(&c, &d)
        .map(|w| w.to_descriptor_checksummed())
        .unwrap_or(d)
}

/// A 3-of-5 native SegWit multisig at BIP-48 account 0, over five keys
/// made from the seeds 01…01 to 05…05 (64 bytes each), with its checksum:
/// for the backup's shares, which a 3-of-5 can leave up to two keys off.
pub fn three_of_five() -> String {
    let c = compute();
    let p: DerivationPath = on_net("m/48'/1'/0'/2'").parse().expect("path");
    let shown = on_net("m/48h/1h/0h/2h");
    let shown = shown.trim_start_matches("m/");
    let keys: Vec<String> = (1u8..=5)
        .map(|b| {
            let m = MasterKey::from_seed(&c, &faraday_crypto::Secret::new([b; 64]), net());
            format!(
                "[{}/{shown}]{}/<0;1>/*",
                m.fingerprint(),
                m.derive(&c, &p).to_xpub(&c)
            )
        })
        .collect();
    let d = format!("wsh(sortedmulti(3,{}))", keys.join(","));
    WalletPolicy::parse_any(&c, &d)
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
#[expect(
    clippy::disallowed_types,
    clippy::disallowed_methods,
    reason = "a coordinator's own reading of the wallet, for the test kit's transactions (docs/COMPUTE.md §7)"
)]
fn definite(
    policy: &WalletPolicy,
    change: bool,
    index: u32,
) -> Result<
    faraday_bip::miniscript::Descriptor<faraday_bip::miniscript::descriptor::DefiniteDescriptorKey>,
    String,
> {
    use faraday_bip::miniscript::descriptor::{Descriptor, DescriptorPublicKey};
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
    let c = compute();
    let policy =
        WalletPolicy::parse_any(&c, &kit.descriptor).map_err(|e| format!("{}: {e}", kit.id))?;
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
    let mut psbt = faraday_bip::bitcoin::Psbt::from_unsigned_tx(tx).map_err(|e| e.to_string())?;
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
    let c = compute();
    let psbt = unsigned(kit)?;
    let Some(n) = kit.cosigner else {
        return Ok((psbt, None));
    };
    let mut session = session();
    let (word, label) = TEST_SEEDS[n];
    let fp = session
        .keys
        .add_words(&c, &test_words(word), label, None)
        .map_err(|e| e.text())?;
    session
        .add_wallet(
            &c,
            kit.name,
            &kit.descriptor,
            "Test wallet",
            crate::wallet::Origin::Named,
        )
        .map_err(|e| e.text())?;
    let mut signed = psbt.clone();
    let by = session.sign(&c, &mut signed, [0; 32], &mut None)?;
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
#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
pub fn files() -> Result<Vec<(String, Vec<u8>)>, String> {
    let c = compute();
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
        let policy = WalletPolicy::parse_any(&c, &kit.descriptor).map_err(|e| e.to_string())?;
        out.push((
            format!("{}-wallet.json", kit.id),
            crate::wallet_backup::wallet_json(kit.name, &policy).into_bytes(),
        ));
        let mut one = session();
        one.add_wallet(
            &c,
            kit.name,
            &kit.descriptor,
            "Test wallet",
            crate::wallet::Origin::Named,
        )
        .map_err(|e| e.text())?;
        if let Some(text) = crate::wallet_backup::multisig_config(&one.wallets[0], None) {
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
    one.add_wallet(
        &c,
        "Savings",
        &savings(),
        "Test wallet",
        crate::wallet::Origin::Named,
    )
    .map_err(|e| e.text())?;
    let w = &one.wallets[0];
    let plan = crate::wallet_backup::split_plan(3, 2, 1);
    for (i, row) in plan.iter().enumerate() {
        if let Some(text) = crate::wallet_backup::multisig_config(w, Some(row)) {
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
            let m = faraday_codec::qr::encode(
                &c,
                faraday_codec::qr::Payload::Bytes(kit.descriptor.as_bytes()),
                faraday_codec::qr::Ecc::Medium,
            )
            .map_err(|e| format!("{e:?}"))?;
            out.push((
                format!("{id}-wallet-qr.png"),
                faraday_codec::png::qr_png(&m, 6),
            ));
        }
    }
    // The three test seeds in every form a seed comes in, and their
    // account xpubs.
    for (n, (word, label)) in TEST_SEEDS.iter().enumerate() {
        let fp = crate::wallet::fp_text(master(word).fingerprint());
        let stem = format!("seed-{}-{fp}", n + 1);
        let xpub_stem = format!("xpub-{}-{fp}", n + 1);
        let m = Mnemonic::parse(&c, Language::English, &test_words(word))
            .map_err(|e| format!("{e:?}"))?;
        out.push((
            format!("{stem}-words.txt"),
            format!(
                "# {label}: dummy words, typed in Add a key\n{}\n",
                test_words(word)
            )
            .into_bytes(),
        ));
        let qr = faraday_codec::seedqr::encode_seedqr(&c, &m).map_err(|e| format!("{e:?}"))?;
        out.push((
            format!("{stem}-seedqr.png"),
            faraday_codec::png::qr_png(&qr, 8),
        ));
        let qr = faraday_codec::seedqr::encode_compact(&c, &m).map_err(|e| format!("{e:?}"))?;
        out.push((
            format!("{stem}-compactseedqr.png"),
            faraday_codec::png::qr_png(&qr, 8),
        ));
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
    one.keys
        .add_words(&c, &test_words(word), label, None)
        .map_err(|e| e.text())?;
    let spending = kits()
        .into_iter()
        .find(|k| k.id == "spending")
        .ok_or("no Spending kit")?;
    let i = one
        .add_wallet(
            &c,
            spending.name,
            &spending.descriptor,
            "Test wallet",
            crate::wallet::Origin::Named,
        )
        .map_err(|e| e.text())?;
    let text = "Faraday test message: this address belongs to test key 1.";
    let signed = one.sign_message(&c, i, 0, text, faraday_psbt::message::Format::Bip322)?;
    out.push((
        "spending-message.txt".to_string(),
        faraday_keys::message::signed_text(&signed.address, &signed.signature, text).into_bytes(),
    ));
    Ok(out)
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
    let c = compute();
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
    WalletPolicy::from_parts(&c, "tr(musig(@0,@1,@2)/**)", &refs).map_err(|e| e.to_string())
}

fn taproot_account(n: usize) -> faraday_bip::bitcoin::bip32::Xpub {
    let c = compute();
    *master(TEST_SEEDS[n].0)
        .account_xpub(&c, faraday_bip::keys::ScriptType::Taproot, 0)
        .expect("account")
        .xpub()
}

/// The tweaks from the root aggregate to the output key at chain/index,
/// and the internal key on the way (`faraday-psbt`'s MuSig2 test, for three).
fn musig_tweaks(
    participants: &[faraday_bip::bitcoin::secp256k1::PublicKey],
    change: bool,
    index: u32,
) -> Result<
    (
        Vec<faraday_bip::musig::Tweak>,
        faraday_bip::bitcoin::secp256k1::PublicKey,
    ),
    String,
> {
    use faraday_bip::bitcoin::taproot::TapTweakHash;
    let secp = Secp256k1::verification_only();
    let mut xpub =
        faraday_bip::musig::aggregate_xpub(participants, net()).map_err(|e| format!("{e:?}"))?;
    let mut tweaks = Vec::new();
    for step in [u32::from(change), index] {
        let child = ChildNumber::from_normal_idx(step).map_err(|e| e.to_string())?;
        let (tweak, _) = xpub.ckd_pub_tweak(child).map_err(|e| e.to_string())?;
        tweaks.push(faraday_bip::musig::Tweak {
            bytes: tweak.secret_bytes(),
            x_only: false,
        });
        xpub = xpub.ckd_pub(&secp, child).map_err(|e| e.to_string())?;
    }
    let internal = xpub.public_key;
    tweaks.push(faraday_bip::musig::Tweak {
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
    let c = compute();
    use std::collections::BTreeMap;
    let policy = musig_policy()?;
    let spk = policy.script_at(&c, false, 0).map_err(|e| e.to_string())?;
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
    let mut inner = faraday_bip::bitcoin::Psbt::from_unsigned_tx(tx).map_err(|e| e.to_string())?;
    inner.inputs[0].witness_utxo = Some(fund.output[0].clone());
    let keys: Vec<_> = (0..3).map(|n| taproot_account(n).public_key).collect();
    let participants = faraday_bip::musig::sort_keys(&keys);
    let (_, internal) = musig_tweaks(&participants, false, 0)?;
    let internal = internal.x_only_public_key().0;
    inner.inputs[0].tap_internal_key = Some(internal);
    let aggregate =
        faraday_bip::musig::aggregate_xpub(&participants, net()).map_err(|e| format!("{e:?}"))?;
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
                    faraday_bip::bitcoin::bip32::Fingerprint::from(m.fingerprint().0),
                    on_net("m/86'/1'/0'")
                        .parse::<DerivationPath>()
                        .map_err(|e| format!("{e}"))?,
                ),
            ),
        );
    }
    inner.inputs[0].tap_key_origins = origins;
    let secp = Secp256k1::verification_only();
    let root =
        faraday_bip::musig::key_agg_with(&secp, &participants).map_err(|e| format!("{e:?}"))?;
    faraday_psbt::musig::write_input_participants(
        &mut inner.inputs[0],
        &faraday_psbt::musig::Participants {
            aggregate: root.public_key(),
            keys: participants,
        },
    );
    Ok(Psbt::from(inner))
}

/// The key-path sighash of the first input.
fn key_path_sighash(psbt: &Psbt) -> Result<[u8; 32], String> {
    use faraday_bip::bitcoin::sighash::{Prevouts, SighashCache, TapSighashType};
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
#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
pub fn musig_rounds() -> Result<(Psbt, Vec<(String, Psbt)>, Vec<(String, Psbt)>), String> {
    let c = compute();
    let unsigned = musig_unsigned()?;
    let policy = musig_policy()?;
    let secp = faraday_bip::bitcoin::secp256k1::Secp256k1::new();
    // What the inspector establishes: output key and tweaks.
    let mut session = session();
    session
        .keys
        .add_words(&c, &test_words(TEST_SEEDS[0].0), TEST_SEEDS[0].1, None)
        .map_err(|e| e.text())?;
    session
        .add_wallet(
            &c,
            "MuSig2",
            &policy.to_descriptor_checksummed(),
            "Test wallet",
            crate::wallet::Origin::Named,
        )
        .map_err(|e| e.text())?;
    let established = session.inspect(&c, &unsigned).inputs[0]
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
            .derive(
                &c,
                &on_net("m/86'/1'/0'").parse().map_err(|e| format!("{e}"))?,
            )
            .secret_key();
        let pk = taproot_account(n).public_key;
        let (sec, pubn) = faraday_bip::musig::nonce_gen(
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
        faraday_psbt::musig::write_pub_nonce(
            &mut copy.inner_mut().inputs[0],
            &pk,
            &established.output_key,
            &pubn,
        );
        faraday_psbt::musig::write_pub_nonce(
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
    session.sign(&c, &mut device, [7; 32], &mut None)?;
    session.sign(&c, &mut again, [9; 32], &mut None)?;
    if device.to_bytes() != again.to_bytes() {
        return Err("the last signer's MuSig2 pass depends on the session seed".into());
    }
    // Round two for keys 2 and 3, over everyone's nonces.
    let nonces =
        faraday_psbt::musig::pub_nonces(&device.inner().inputs[0]).map_err(|e| format!("{e:?}"))?;
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
    let aggnonce = faraday_bip::musig::nonce_agg(&all).map_err(|e| format!("{e:?}"))?;
    let values = faraday_bip::musig::session_values(
        &secp,
        &established.participants,
        &established.tweaks,
        &aggnonce,
        &msg,
    )
    .map_err(|e| format!("{e:?}"))?;
    let mut partial_files = Vec::new();
    for (n, sec, sk, pk) in secnonces {
        let psig =
            faraday_bip::musig::sign(&secp, sec, &sk, &values).map_err(|e| format!("{e:?}"))?;
        let mut copy = device.clone();
        faraday_psbt::musig::write_partial_sig(
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
/// words as shares 1 and 2; share 3 is
/// computed. Its record is public and is the wallet's file.
pub fn threshold_record() -> Result<faraday_bip::threshold::ThresholdRecord, String> {
    let c = compute();
    let words: Vec<String> = TEST_SEEDS
        .iter()
        .take(2)
        .map(|(word, _)| test_words(word))
        .collect();
    let mut keys = faraday_keys::Keys::on(net());
    for (i, w) in words.iter().enumerate() {
        keys.add_words(&c, w, &format!("dealer {i}"), None)
            .map_err(|e| format!("{e:?}"))?;
    }
    let dealt = keys.deal(&c, &[0, 1], 3, 2).map_err(|e| format!("{e:?}"))?;
    faraday_bip::threshold::ThresholdRecord::parse(&c, &dealt.record).map_err(|e| format!("{e:?}"))
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
    let c = compute();
    let mut session = session();
    session
        .keys
        .add_words(&c, &test_words(TEST_SEEDS[0].0), TEST_SEEDS[0].1, None)
        .map_err(|e| e.text())?;
    session
        .add_wallet(
            &c,
            "Threshold",
            &threshold_record()?.to_text(),
            "Test wallet",
            crate::wallet::Origin::Named,
        )
        .map_err(|e| e.text())?;
    let mut psbt = threshold_unsigned()?;
    let signed = session.sign_with(&c, &mut psbt, [5; 32], &mut None, &[1], None, None)?;
    let section = signed
        .carry
        .ok_or("the first location left no carry section")?;
    Ok(faraday_keys::carry::bytes(&section, &psbt.to_bytes()).to_vec())
}

/// The backup test stick's vault passphrase.
pub const BACKUP_VAULT_PASSPHRASE: &str = "a";

/// The public files a wallet's backup puts in the Outbox, every one the
/// Back up step offers (`Faraday::public_out`), under the names it gives
/// them.
pub fn public_files(name: &str, descriptor: &str) -> Result<Vec<(String, Vec<u8>)>, String> {
    let c = compute();
    let mut app = crate::Faraday::new();
    app.session = session();
    let w = app
        .session
        .add_wallet(
            &c,
            name,
            descriptor,
            "Test wallet",
            crate::wallet::Origin::Named,
        )
        .map_err(|e| e.text())?;
    for what in 0..=8 {
        app.public_out(w, what);
    }
    Ok(app
        .outbox
        .iter()
        .map(|i| (i.name.clone(), i.bytes.clone()))
        .collect())
}

/// The backup test stick (2026-10-06): only what backing up the 2-of-3
/// Taproot multisig over the three test seeds leaves, as a person would
/// carry it. On the stick, every public file Faraday's Backup step
/// writes for that wallet (the descriptor as text and as a labelled QR
/// picture, the multisig config, the split shares with their pictures,
/// the wallet .json, the BIP 129 record, Bitcoin Core's
/// import file, the backup sheet and the blank template as PDFs);
/// `vault.ofv`, whose passphrase is `a`, holding the three seeds, each
/// loaded at unlock; and an unsigned spend from the wallet. All testnet.
/// The full kit is [`files`].
pub fn backup_files() -> Result<Vec<(String, Vec<u8>)>, String> {
    let kit = kits()
        .into_iter()
        .find(|k| k.id == "taproot-multisig")
        .ok_or("no Taproot multisig kit")?;
    let mut out = public_files(kit.name, &kit.descriptor)?;
    out.push(("vault.ofv".to_string(), seeds_vault()?));
    // A spend to try the Spend tab and Wallets on: what a coordinator
    // such as Sparrow hands over.
    let (unsigned, _) = spend_of(&kit)?;
    out.push((
        "taproot-multisig-unsigned.psbt".to_string(),
        unsigned.to_bytes(),
    ));
    Ok(out)
}

/// The backup test stick's vault: the three test seeds, loaded at
/// unlock, under one passphrase at the Light cost.
fn seeds_vault() -> Result<Vec<u8>, String> {
    let c = compute();
    use faraday_vault::records::{self, field, kind};
    use faraday_vault::{Contents, Cost, Record};
    let mut main = Contents {
        records: vec![Record::new(kind::SLOT_LABEL).with(field::LABEL, b"Test seeds")],
    };
    for (word, label) in TEST_SEEDS {
        let m = Mnemonic::parse(&c, Language::English, &test_words(word))
            .map_err(|e| format!("{e:?}"))?;
        main.records.push(
            Record::new(kind::KEY)
                .with(field::KEY, &records::words_payload(&m))
                .with(field::KEY_LABEL, label.as_bytes()),
        );
    }
    let cost = Cost {
        memory_kib: 64 * 1024,
        passes: 3,
        lanes: 1,
    };
    faraday_vault::create(
        &c,
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
/// Light cost with 64 KiB slots. The first opens test key 1 (ticked at
/// unlock), every test wallet, an entry and a note; the second opens a
/// slot with one entry. Everything is a dummy.
pub fn test_vault() -> Result<Vec<u8>, String> {
    let c = compute();
    use faraday_vault::records::{self, field, kind};
    use faraday_vault::{Contents, Cost, Record};
    let m = Mnemonic::parse(&c, Language::English, &test_words(TEST_SEEDS[0].0))
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
                .with(field::KEY_LABEL, TEST_SEEDS[0].1.as_bytes()),
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
        &c,
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
/// `field: value` lines (`docs/history/PLAN.md` §6.4).
pub const ENTRIES_FILE: &str = "otpauth://totp/Example%20Git:octo?secret=JBSWY3DPEHPK3PXP&issuer=Example%20Git\n\
\n\
title: Router admin\n\
username: admin\n\
password: router-password\n\
url: http://192.168.1.1\n";

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

/// The app as a shell starts it: made, then told its display, which is
/// when the start-up self-test runs and before which no key is accepted.
/// 1280 × 800 at 160 dpi, the size the app assumes before a display
/// arrives, so nothing on screen moves.
pub fn started() -> crate::Faraday {
    use faraday_shell_api::{App, BootState, DisplayInfo, Event, SecureHardware};
    let mut app = crate::Faraday::new();
    app.event(Event::Display(DisplayInfo {
        width: 1280,
        height: 800,
        dpi: 160,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
    app
}

/// Typing pauses: one tick of the clock past the pause after which the
/// key a passphrase or words typed make is worked out and shown
/// ([`crate::motion::TYPED_PAUSE_MS`]).
pub fn pause(app: &mut crate::Faraday) {
    use faraday_shell_api::{App, Event};
    let now = app.now_ms() + crate::motion::TYPED_PAUSE_MS;
    app.event(Event::Tick { now_ms: now });
}

/// The board on screen (Back up's plan, or a wallet flow's own) made into
/// the plan `preset` builds, by the presses a person makes now that the
/// presets are not offered (`docs/BACKUP-FLOW.md` §17.21 item 9, F53):
/// **Add watch-only software**, **Customize backup**, a SeedQR's toggle
/// on paper, **+ new vault** for each vault not set up before the keys,
/// and the ⋮ menu's **Add to…** for each vault's stick and for what each
/// vault holds. Works out the plan's map after each press. Panics where a
/// press is not offered, or where the plan reached is not the preset's.
#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
pub fn backup_as(app: &mut crate::Faraday, preset: crate::plan::Preset) {
    let c = compute();
    use crate::board::{BoardPress as P, Chip, MenuRow, Thing, Zone, things_of};
    use crate::plan::{self, Answers, At, Spot, What};
    use crate::{Action, Sheet};
    let press = |app: &mut crate::Faraday, p: P| {
        app.press(Action::Board(p));
        // A drop that lets a place see the balance asks (§19.5): kept.
        if app.sheet == Some(Sheet::BoardSees) {
            app.press(Action::Board(P::SeesKeep));
        }
    };
    let now = |app: &crate::Faraday| -> Vec<Spot> {
        let (shape, a) = app.board_plan().expect("a board");
        plan::map(&c, &shape, &a)
    };
    let held = |m: &[Spot], at: At| plan::holds_at(m, at);
    let press_q = |app: &mut crate::Faraday| app.press(Action::BQ(crate::qstep::BOARD));
    // The ⋮ menu of `t` in the list, and the first box it offers that
    // `pick` takes.
    let add_to = |app: &mut crate::Faraday, t: Thing, pick: &dyn Fn(Zone) -> bool| {
        press(app, P::Menu(Chip::List(t)));
        let to = app
            .board_state()
            .and_then(|st| st.menu.as_ref())
            .and_then(|m| {
                m.rows.iter().find_map(|r| match r {
                    MenuRow::Item(_, P::MenuTo(z)) if pick(*z) => Some(*z),
                    _ => None,
                })
            });
        let Some(to) = to else {
            panic!(
                "Add to… of {t:?} offers no such box: {:?}",
                app.board_state()
                    .and_then(|st| st.menu.as_ref())
                    .map(|m| &m.rows)
            );
        };
        press(app, P::MenuTo(to));
    };
    // Back up's board card opened, where another card is.
    if !app.board_shown() && app.board_of() == Some(crate::board::BoardOf::Backup) {
        press_q(app);
    }
    let (shape, _) = app.board_plan().expect("a board");
    let want = plan::map(&c, &shape, &Answers::preset(&shape, preset));
    // The plan reached is the preset's.
    let norm = |m: &[Spot]| -> Vec<String> {
        let mut out: Vec<String> = m
            .iter()
            .filter(|s| !s.holds.is_empty())
            .map(|s| {
                let mut h: Vec<String> = s.holds.iter().map(|x| format!("{x:?}")).collect();
                h.sort();
                format!("{:?} {}", s.at, h.join(" "))
            })
            .collect();
        out.sort();
        out
    };
    // Nothing to press where the board shows it already.
    if norm(&now(app)) == norm(&want) {
        return;
    }
    // The software place, on the read-only board's own button.
    if want.iter().any(|s| s.at == At::Software) && !now(app).iter().any(|s| s.at == At::Software) {
        if app.board_plan().is_some_and(|(_, a)| a.custom) {
            add_to(app, Thing::Software, &|z| z == Zone::Scanned);
        } else {
            press(app, P::Software);
        }
    }
    if app.board_plan().is_some_and(|(_, a)| !a.custom) {
        press(app, P::Customize);
    }
    // The SeedQR off the paper where the preset keeps the words alone.
    let m = now(app);
    for p in 0..plan::places_of(&m) {
        for w in held(&m, At::Place(p)) {
            if let What::SeedQr(i) = w
                && !held(&want, At::Place(p)).contains(&w)
            {
                press(app, P::Form(Zone::Paper(p), Thing::Seed(i), 1));
            }
        }
    }
    // Each vault: made, its stick where the preset puts it, and what it
    // holds.
    let vaults: Vec<usize> = want
        .iter()
        .filter_map(|s| match s.at {
            At::Vault(v) => Some(v),
            _ => None,
        })
        .collect();
    for &v in &vaults {
        for _ in 0..=v {
            if v < shape.vaults || now(app).iter().any(|s| s.at == At::Vault(v)) {
                break;
            }
            press(app, P::AddVault);
        }
        let stick_of = |m: &[Spot], p: usize| {
            (0..plan::sticks_at(m, p)).find(|&s| plan::stick_vault(m, p, s) == Some(v))
        };
        for p in 0..plan::places_of(&want) {
            if stick_of(&want, p).is_some() && stick_of(&now(app), p).is_none() {
                add_to(
                    app,
                    Thing::Vault(v),
                    &|z| matches!(z, Zone::Card(q) | Zone::NewStick(q) | Zone::Empty(q, _) if q == p),
                );
            }
        }
        let m = now(app);
        let Some((p, s)) = (0..plan::places_of(&m)).find_map(|p| stick_of(&m, p).map(|s| (p, s)))
        else {
            continue;
        };
        for w in held(&want, At::Vault(v)) {
            for t in things_of(w) {
                let there = held(&now(app), At::Vault(v))
                    .into_iter()
                    .any(|h| things_of(h).contains(&t));
                if !there {
                    add_to(app, t, &|z| z == Zone::Vault(p, s, v));
                }
            }
        }
    }
    assert_eq!(
        norm(&now(app)),
        norm(&want),
        "the presses reach the plan {preset:?} builds"
    );
}

/// Types `words` into the open word entry as a person does on a
/// keyboard: each word, then a space.
pub fn type_words(app: &mut crate::Faraday, words: &str) {
    use faraday_shell_api::{App, Event, Key};
    for word in words.split_whitespace() {
        for c in word.chars().chain([' ']) {
            app.event(Event::Key(Key::Char(c)));
        }
    }
}

/// Checks every passphrase the backup's checklist keeps on paper, as a
/// person with a right copy does: its item opened, Check my copy, the
/// key's own passphrase typed back and Enter (`docs/history/NEW-WALLET.md`
/// §14.7 item 5).
pub fn type_passphrases(app: &mut crate::Faraday) {
    use crate::{Action, bstep};
    use faraday_shell_api::{App, Event, Key};
    for n in app.backup_items_now(&compute()) {
        if !bstep::is_passphrase(n) {
            continue;
        }
        if app.backup().and_then(|b| b.open) != Some(n) {
            app.press(Action::BStep(n));
        }
        let pass = {
            let b = app.backup().expect("a backup is open");
            let key = app.session.keys.info(b.key).expect("the open seed");
            assert!(key.has_passphrase, "a key with a passphrase");
            app.session
                .keys
                .reveal(b.key, faraday_keys::Reveal::Passphrase)
                .to_string()
        };
        app.press(Action::BCheck);
        for c in pass.chars() {
            app.event(Event::Key(Key::Char(c)));
        }
        app.event(Event::Key(Key::Enter));
    }
}

/// Opens Check my copy on the backup's open copy item and types the
/// seed's own words back, as a person with a right copy does.
pub fn type_copy(app: &mut crate::Faraday) {
    use crate::Action;
    let words = {
        let b = app.backup().expect("a backup is open");
        let key = app.session.keys.info(b.key).expect("the open seed");
        assert!(key.has_words(), "a seed with words");
        app.session
            .keys
            .reveal(b.key, faraday_keys::Reveal::CopyByHand)
            .to_string()
    };
    app.press(Action::BCheck);
    type_words(app, &words);
}

/// A key kind's key text for a master key a test holds
/// (`Keys::master_for_tests`); the session's keys go through
/// [`crate::wallet::KeyTexts`]. Bring it in with `use
/// faraday_core::testkit::KindText`.
pub trait KindText {
    /// Costs a key derivation. `[fingerprint/path]xpub` of `master` at the
    /// kind's path, account 0.
    fn key_text(
        self,
        c: &faraday_crypto::work::Compute,
        master: &MasterKey,
    ) -> Result<String, String>;
}

impl KindText for crate::create::NewKind {
    fn key_text(
        self,
        c: &faraday_crypto::work::Compute,
        master: &MasterKey,
    ) -> Result<String, String> {
        faraday_keys::texts::key_text(c, master, &self.spec(master.network(), 0)?)
    }
}

/// The word-list sheet's state, while it is open.
pub fn wordlist(app: &crate::Faraday) -> Option<&crate::wordlist::WordListState> {
    app.wordlist.state()
}

/// Scrolls the Learn sheet to `y` design units, as a tool that scrolls it
/// from outside does.
pub fn learn_scroll_to(app: &mut crate::Faraday, y: f32) {
    *app.learn.scroll_mut() = y;
}

/// Opens the vanity screen's card `step`, as a press on it does.
pub fn vanity_open_card(app: &mut crate::Faraday, step: u8) {
    app.vanity.open_card(step);
}

/// Sets Explore's address list to start at index `from`.
pub fn explore_list_from(app: &mut crate::Faraday, from: u32) {
    app.explore.set_from(from);
}

/// Replaces the path Explore holds as typed with `path`, returning the
/// one it replaced.
pub fn explore_swap_path(app: &mut crate::Faraday, path: String) -> Option<String> {
    app.explore.swap_path(path)
}

/// The open calculator's state, while Tools is open.
pub fn tools(app: &crate::Faraday) -> Option<&crate::tools::ToolsState> {
    app.tools.state()
}

/// What is typed in Tools' Find a tool.
pub fn catalog_find(app: &crate::Faraday) -> &str {
    app.tools.find()
}

/// The open calculator's state, to set what it holds.
pub fn tools_mut(app: &mut crate::Faraday) -> Option<&mut crate::tools::ToolsState> {
    app.tools.state_mut()
}

/// Closes the QR view, as closing its sheet does.
pub fn qr_close(app: &mut crate::Faraday) {
    app.qr.close();
}

/// The message-signing flow's state, while it is open.
pub fn message(app: &crate::Faraday) -> Option<&crate::MessageState> {
    app.message.state()
}

/// Types `text` into the message being signed.
pub fn message_text(app: &mut crate::Faraday, text: &str) {
    app.message.set_text(text);
}

/// Add a key's state: what was typed and chosen.
pub fn entry(app: &crate::Faraday) -> &crate::EntryState {
    app.entry.state()
}

/// The secret waiting on the secret sheet, if one is.
pub fn secret_out(app: &crate::Faraday) -> Option<&crate::secrets::SecretOut> {
    app.secrets.out()
}

/// Opens Create's card `step`, as a press on it does.
pub fn create_open_card(app: &mut crate::Faraday, step: u8) {
    if let Some(c) = app.create.state_mut() {
        c.open = Some(step);
    }
}

/// Puts `source` in Create's key slot `slot`.
pub fn create_set_slot(app: &mut crate::Faraday, slot: usize, source: crate::create::Source) {
    if let Some(c) = app.create.state_mut() {
        c.slots[slot] = source;
    }
}

/// Chooses wallet `i` on the Wallets tab.
pub fn select_wallet(app: &mut crate::Faraday, i: usize) {
    app.wallets.set_wallet(i);
}

/// Puts `item` at the end of the Inbox.
pub fn inbox_push(app: &mut crate::Faraday, item: crate::Item) {
    app.inbox.push(item);
}

/// Puts `item` at place `at` in the Inbox.
pub fn inbox_insert(app: &mut crate::Faraday, at: usize, item: crate::Item) {
    app.inbox.insert(at, item);
}

/// Keeps the Inbox files `keep` says to.
pub fn inbox_retain(app: &mut crate::Faraday, keep: impl FnMut(&crate::Item) -> bool) {
    app.inbox.retain(keep);
}

/// Inbox file `at`, to change it.
pub fn inbox_item_mut(app: &mut crate::Faraday, at: usize) -> &mut crate::Item {
    app.inbox.item_mut(at)
}

/// Back up's plan answers, to set what they hold. Panics where no backup
/// is open.
pub fn backup_answers_mut(app: &mut crate::Faraday) -> &mut crate::plan::Answers {
    &mut app.backup.state_mut().expect("Back up is open").answers
}

/// Opens Back up's question `q`, or none.
pub fn backup_set_q(app: &mut crate::Faraday, q: Option<u8>) {
    if let Some(b) = app.backup.state_mut() {
        b.q = q;
    }
}

/// Drops the backup under way, as a lock does.
pub fn backup_drop(app: &mut crate::Faraday) {
    app.backup.close();
}

/// How long one Argon2id pass over 64 MiB takes here, ms, set as if
/// measured: the costs and waits shown are worked out from it.
pub fn vault_speed(app: &mut crate::Faraday, ms: Option<u64>) {
    app.vaults.set_speed(ms);
}

/// Changes open vault `id`'s records through `change`, as the vault's
/// own edits do.
pub fn vault_edit<T>(
    app: &mut crate::Faraday,
    id: faraday_keys::vault::VaultId,
    change: impl FnOnce(&mut faraday_keys::vault::OpenVaults) -> T,
) -> T {
    app.vaults.edit(id, change)
}

/// Open vault `o` has one more change not sealed.
pub fn vault_bump(app: &mut crate::Faraday, o: usize) {
    app.vaults.bump(o);
}

/// Closes the vault item's form.
pub fn vault_close_form(app: &mut crate::Faraday) {
    app.vaults.close_form();
}

/// The vaults' summaries, to change what they hold.
pub fn vault_summaries_mut(app: &mut crate::Faraday) -> &mut Vec<crate::vault::VaultSummary> {
    app.vaults.summaries_mut()
}

/// Open vault `id` changed, as the vault's own edits say it.
pub fn vault_changed(app: &mut crate::Faraday, id: faraday_keys::vault::VaultId) {
    app.vaults.changed(id);
}

/// Shows open vault `o` in the contents.
pub fn vault_current(app: &mut crate::Faraday, o: usize) {
    app.vaults.set_current(o);
}

/// Swaps open vaults `a` and `b` in the list.
pub fn vault_swap_open(app: &mut crate::Faraday, a: usize, b: usize) {
    app.vaults.swap_open(a, b);
}

/// Puts flow `f` at the end of the backup flows, returning its place.
pub fn flow_push(app: &mut crate::Faraday, f: crate::backup_flow::BackupFlow) -> usize {
    app.flows.all_mut().push(f);
    app.flows.len() - 1
}

/// The backup flows, to set what they hold.
pub fn flows_mut(app: &mut crate::Faraday) -> &mut Vec<crate::backup_flow::BackupFlow> {
    app.flows.all_mut()
}

/// The scan sheet open for `purpose`, as the press that opens it for that
/// purpose leaves it: what the fuzz target `app_scanned` sends scanned
/// codes into, one purpose at a time.
pub fn open_scan(app: &mut crate::Faraday, purpose: crate::ScanPurpose) {
    let mut scan = crate::ScanState::default();
    scan.purpose = purpose;
    app.camera.open(scan);
    app.sheet = Some(crate::Sheet::Scan);
}

/// The 32 bytes a key made now for `what` takes from the session's fresh
/// randomness ([`crate::Faraday::fresh`]): one pooled answer, used up.
pub fn fresh(app: &mut crate::Faraday, what: &[u8]) -> Option<[u8; 32]> {
    app.fresh(what)
}

/// The session seed's draw for `what` and `count`, as a seal nonce or a
/// keyboard's code draws it.
pub fn seed_draw(app: &crate::Faraday, what: &[u8], count: &[u8]) -> Option<[u8; 32]> {
    app.fresh.draw(what, count)
}
