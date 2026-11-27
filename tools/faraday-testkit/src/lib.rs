//! A tool or test-support crate: the test data (09 §8; section 10 §6). The data half of `main`'s
//! `testkit`, over domain types only; it has no application edge.

use faraday_bip::policy::WalletPolicy;

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

/// Why a piece of test data could not be made.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KitError(pub String);

/// The three test seeds.
#[must_use]
pub fn test_seeds() -> [TestSeed; 3] {
    todo!()
}

/// The words of a test seed named by a word of its own.
#[must_use]
pub fn test_words(_word: &str) -> String {
    todo!()
}

/// The ten test wallets.
#[must_use]
pub fn kits() -> Vec<TestWallet> {
    todo!()
}

/// The 2-of-3 native SegWit multisig descriptor over the three test seeds.
#[must_use]
pub fn savings() -> String {
    todo!()
}

/// A spend of the wallet, unsigned, as PSBT bytes.
pub fn unsigned(_kit: &TestWallet) -> Result<Vec<u8>, KitError> {
    todo!()
}

/// The unsigned and the cosigned spend.
pub fn spend() -> Result<(Vec<u8>, Vec<u8>), KitError> {
    todo!()
}

/// The MuSig2 wallet's policy.
pub fn musig_policy() -> Result<WalletPolicy, KitError> {
    todo!()
}

/// The MuSig2 wallet's unsigned spend.
pub fn musig_unsigned() -> Result<Vec<u8>, KitError> {
    todo!()
}

/// The passphrase of the test vault.
pub const VAULT_PASSPHRASE: &str = "test vault";

/// A test vault, sealed.
pub fn test_vault() -> Result<Vec<u8>, KitError> {
    todo!()
}

/// The test EFI image.
#[must_use]
pub fn test_efi() -> Vec<u8> {
    todo!()
}
