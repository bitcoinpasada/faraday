//! A tool or test-support crate: the test data (09 §8; section 10 §6). The data half of `main`'s
//! `testkit`, over domain types only; it has no application edge.
//!
//! Everything is on testnet. The seeds are dummies: each is one word repeated twenty-four times,
//! the QR test kit's signers 1 to 3.

use faraday_bip::policy::WalletPolicy;
use faraday_crypto::work::Compute;
use faraday_text::Text;

mod efi;
mod psbts;
mod vault;

/// One test seed: its name and its words.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TestSeed {
    pub name: &'static str,
    pub words: &'static str,
}

/// One test wallet (`main`'s `Kit`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TestWallet {
    /// The file stem on the test stick and the wallet's short name.
    pub id: &'static str,
    /// The name the wallet is loaded under.
    pub name: &'static str,
    pub descriptor: String,
    /// Which test seed signs the cosigner's copy, when there is one.
    pub cosigner: Option<usize>,
    /// The fee, in sat.
    pub fee: u64,
}

/// Why a piece of test data could not be made. It makes public fixtures and dummy seeds only, so
/// its words print in a failing test.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KitError(pub String);

impl From<Text> for KitError {
    fn from(text: Text) -> KitError {
        KitError(String::from(text.as_str()))
    }
}

/// A token for a test's costly work, which counts nothing.
#[expect(
    clippy::disallowed_methods,
    reason = "the token that counts nothing, for the test data (docs/COMPUTE.md §3)"
)]
pub(crate) fn compute() -> Compute {
    Compute::unchecked()
}

/// The three test seeds.
#[must_use]
pub fn test_seeds() -> [TestSeed; 3] {
    [
        TestSeed {
            name: "Test key 1",
            words: "bacon bacon bacon bacon bacon bacon bacon bacon bacon bacon bacon bacon bacon bacon bacon bacon bacon bacon bacon bacon bacon bacon bacon bacon",
        },
        TestSeed {
            name: "Test key 2",
            words: "zebra zebra zebra zebra zebra zebra zebra zebra zebra zebra zebra zebra zebra zebra zebra zebra zebra zebra zebra zebra zebra zebra zebra zebra",
        },
        TestSeed {
            name: "Test key 3",
            words: "summer summer summer summer summer summer summer summer summer summer summer summer summer summer summer summer summer summer summer summer summer summer summer summer",
        },
    ]
}

/// The words of a test seed named by a word of its own (`bacon`, `zebra`, `summer`): that word
/// twenty-four times.
#[must_use]
pub fn test_words(word: &str) -> String {
    vec![word; 24].join(" ")
}

/// `[fingerprint/path]xpub` for test seed `n`.
fn key(n: usize, path: &str) -> String {
    psbts::key(n, path).map_or_else(|e| String::from(e.as_str()), |t| String::from(t.as_str()))
}

/// The 2-of-3 native SegWit multisig descriptor over the three test seeds at BIP-48 account 0,
/// with its checksum.
#[must_use]
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
        .map_or(d, |w| String::from(w.to_descriptor_checksummed().as_str()))
}

/// The ten test wallets, in the order the session loads them.
#[must_use]
pub fn kits() -> Vec<TestWallet> {
    let k1 = |p: &str| key(0, p);
    let multi = |p: &str| {
        format!(
            "{}/<0;1>/*,{}/<0;1>/*,{}/<0;1>/*",
            key(0, p),
            key(1, p),
            key(2, p)
        )
    };
    let wallet = |id, name, descriptor, cosigner, fee| TestWallet {
        id,
        name,
        descriptor,
        cosigner,
        fee,
    };
    vec![
        wallet("savings", "Savings", savings(), Some(1), 2_140),
        wallet(
            "spending",
            "Spending",
            format!("wpkh({}/<0;1>/*)", k1("m/84'/1'/0'")),
            None,
            1_410,
        ),
        wallet(
            "taproot",
            "Taproot",
            format!("tr({}/<0;1>/*)", k1("m/86'/1'/0'")),
            None,
            1_540,
        ),
        wallet(
            "nested",
            "Nested SegWit",
            format!("sh(wpkh({}/<0;1>/*))", k1("m/49'/1'/0'")),
            None,
            1_660,
        ),
        wallet(
            "legacy",
            "Legacy",
            format!("pkh({}/<0;1>/*)", k1("m/44'/1'/0'")),
            None,
            2_260,
        ),
        wallet(
            "vault-nested",
            "Nested multisig",
            format!("sh(wsh(sortedmulti(2,{})))", multi("m/48'/1'/0'/1'")),
            Some(1),
            2_560,
        ),
        wallet(
            "vault-legacy",
            "Legacy multisig",
            format!("sh(sortedmulti(2,{}))", multi("m/45'/1'/0'")),
            Some(2),
            3_380,
        ),
        wallet(
            "taproot-multisig",
            "Taproot multisig",
            format!(
                "tr({},sortedmulti_a(2,{}))",
                psbts::NUMS,
                multi("m/48'/1'/0'/3'")
            ),
            Some(1),
            1_820,
        ),
        wallet(
            "inheritance",
            "Inheritance",
            format!(
                "wsh(or_d(pk({}/<0;1>/*),and_v(v:pkh({}/<0;1>/*),older(52560))))",
                key(0, "m/48'/1'/1'/2'"),
                key(1, "m/48'/1'/1'/2'")
            ),
            None,
            1_980,
        ),
        wallet(
            "tree",
            "Taproot tree",
            format!(
                "tr({}/<0;1>/*,{{and_v(v:pk({}/<0;1>/*),older(4320)),pk({}/<0;1>/*)}})",
                key(0, "m/86'/1'/1'"),
                key(1, "m/86'/1'/1'"),
                key(2, "m/86'/1'/1'")
            ),
            None,
            1_540,
        ),
    ]
}

/// A spend of the wallet, unsigned, as PSBT bytes.
pub fn unsigned(kit: &TestWallet) -> Result<Vec<u8>, KitError> {
    Ok(psbts::unsigned_psbt(kit)?.to_bytes())
}

/// The Savings spend: the unsigned and the cosigned PSBT bytes.
pub fn spend() -> Result<(Vec<u8>, Vec<u8>), KitError> {
    let kits = kits();
    let savings = &kits[0];
    let cosigner = savings
        .cosigner
        .ok_or_else(|| KitError(String::from("Savings has a cosigner")))?;
    Ok((
        psbts::unsigned_psbt(savings)?.to_bytes(),
        psbts::cosigned(savings, cosigner)?.to_bytes(),
    ))
}

/// The MuSig2 wallet's policy.
pub fn musig_policy() -> Result<WalletPolicy, KitError> {
    Ok(psbts::musig_policy()?)
}

/// The MuSig2 wallet's unsigned spend.
pub fn musig_unsigned() -> Result<Vec<u8>, KitError> {
    Ok(psbts::musig_unsigned_psbt()?.to_bytes())
}

/// The passphrase of the test vault.
pub const VAULT_PASSPHRASE: &str = "test vault";

/// A test vault, sealed: one passphrase at the test cost, holding test key 1, an entry, a note
/// and every test wallet.
pub fn test_vault() -> Result<Vec<u8>, KitError> {
    Ok(vault::test_vault()?)
}

/// The test EFI image: a minimal PE32+ EFI application for Secure Boot signing, which does
/// nothing if started.
#[must_use]
pub fn test_efi() -> Vec<u8> {
    efi::image()
}
