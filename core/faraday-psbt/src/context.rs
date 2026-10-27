//! What the inspector knows about the loaded keys: fingerprints and
//! account-level extended public keys. Nothing here is secret.

use alloc::vec::Vec;

use bitcoin::ScriptBuf;
use bitcoin::bip32::{ChildNumber, DerivationPath};
use bitcoin::secp256k1::{PublicKey, Secp256k1, Verification};
use faraday_bip::account::{AccountXpub, MultisigAccountXpub};
use faraday_bip::keys::{self, Fingerprint, MasterKey, MultisigScriptType, Network, ScriptType};
use faraday_bip::policy::WalletPolicy;
use faraday_crypto::work::{Compute, Work};

/// A loaded key as the inspector sees it: its master fingerprint and the
/// account xpubs it can derive addresses from.
///
/// Change and input ownership are verified by re-deriving from these
/// xpubs, so only paths below a listed
/// account can be verified. The multisig accounts are BIP-48's two; a
/// multisig change output is verified only against a registered wallet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyRef {
    /// The master fingerprint, as it appears in key origins.
    pub fingerprint: Fingerprint,
    /// Account xpubs, usually the four single-signature script types for
    /// account 0.
    pub accounts: Vec<AccountXpub>,
    /// BIP-48 account xpubs, usually both script types for account 0.
    pub multisig: Vec<MultisigAccountXpub>,
}

impl KeyRef {
    /// A reference with explicit accounts.
    pub fn new(
        fingerprint: Fingerprint,
        accounts: Vec<AccountXpub>,
        multisig: Vec<MultisigAccountXpub>,
    ) -> Self {
        Self {
            fingerprint,
            accounts,
            multisig,
        }
    }

    /// Costs a key derivation (through `account_xpub` and
    /// `multisig_account_xpub`). The four single-signature accounts
    /// (BIP-44/49/84/86) and BIP-48's two multisig accounts at index
    /// `account` on the master key's network.
    pub fn from_master(c: &Compute, master: &MasterKey, account: u32) -> Result<Self, keys::Error> {
        let mut accounts = Vec::with_capacity(4);
        for script_type in ScriptType::ALL {
            accounts.push(master.account_xpub(c, script_type, account)?);
        }
        let mut multisig = Vec::with_capacity(2);
        for script_type in MultisigScriptType::ALL {
            multisig.push(master.multisig_account_xpub(c, script_type, account)?);
        }
        Ok(Self::new(master.fingerprint(), accounts, multisig))
    }

    /// Re-derives `path` if it is `account/change/index` below one of the
    /// listed accounts, with both leaf components unhardened.
    pub(crate) fn derive<C: Verification>(
        &self,
        secp: &Secp256k1<C>,
        path: &DerivationPath,
    ) -> Option<Derived> {
        let components: Vec<ChildNumber> = path.to_u32_vec().iter().map(|&n| n.into()).collect();
        for account in &self.accounts {
            let prefix: Vec<ChildNumber> = account.path().into_iter().copied().collect();
            if components.len() != prefix.len() + 2 || components[..prefix.len()] != prefix[..] {
                continue;
            }
            let (change, index) = match (components[prefix.len()], components[prefix.len() + 1]) {
                (ChildNumber::Normal { index: c @ (0 | 1) }, ChildNumber::Normal { index }) => {
                    (c == 1, index)
                }
                _ => continue,
            };
            let key = account
                .xpub()
                .derive_pub(secp, &components[prefix.len()..].to_vec())
                .ok()?
                .public_key;
            let script_pubkey = account.address(change, index).ok()?.script_pubkey();
            return Some(Derived {
                change,
                index,
                key,
                script_pubkey,
            });
        }
        None
    }

    /// Costs a key derivation: the public key at `path`, if it is
    /// `account/change/index` below one of the listed accounts, the
    /// single-signature ones or BIP-48's, with both leaf components
    /// unhardened. The script is not part of the answer: a multisig
    /// script needs the cosigners.
    pub(crate) fn derive_key<C: Verification>(
        &self,
        c: &Compute,
        secp: &Secp256k1<C>,
        path: &DerivationPath,
    ) -> Option<PublicKey> {
        c.count(Work::KeyDerive);
        let components: Vec<ChildNumber> = path.to_u32_vec().iter().map(|&n| n.into()).collect();
        let accounts = self
            .accounts
            .iter()
            .map(|a| (a.path(), a.xpub()))
            .chain(self.multisig.iter().map(|a| (a.path(), a.xpub())));
        for (account, xpub) in accounts {
            let prefix: Vec<ChildNumber> = account.into_iter().copied().collect();
            if components.len() != prefix.len() + 2 || components[..prefix.len()] != prefix[..] {
                continue;
            }
            let leaf = &components[prefix.len()..];
            if !matches!(
                (leaf[0], leaf[1]),
                (
                    ChildNumber::Normal { index: 0 | 1 },
                    ChildNumber::Normal { .. }
                )
            ) {
                continue;
            }
            return Some(xpub.derive_pub(secp, &leaf.to_vec()).ok()?.public_key);
        }
        None
    }
}

/// The result of re-deriving a key-origin path from an account xpub.
pub(crate) struct Derived {
    pub change: bool,
    pub index: u32,
    /// The public key at the path.
    pub key: PublicKey,
    /// The script the account's type produces for that key.
    pub script_pubkey: ScriptBuf,
}

/// Everything [`crate::inspect`] needs besides the PSBT: the network the
/// user expects, the loaded keys, and the wallets registered for the
/// session.
#[derive(Debug, Clone, Copy)]
pub struct Context<'a> {
    /// Addresses are rendered for this network and origins are checked
    /// against its coin type.
    pub network: Network,
    /// The loaded keys.
    pub keys: &'a [KeyRef],
    /// The BIP-388 wallet policies in use this session. Change of one of
    /// them is verified by re-deriving its script; with none registered,
    /// a multisig change output is unverified and counted as sent.
    pub wallets: &'a [WalletPolicy],
    /// What the open MuSig2 session holds, when one is open.
    /// It decides whether a public nonce
    /// of this device's already on an input can still be signed under or
    /// has to be replaced.
    pub musig_session: Option<&'a crate::musig::MusigSessionView>,
}

impl<'a> Context<'a> {
    /// The loaded key with `fingerprint`, if any.
    pub fn key(&self, fingerprint: Fingerprint) -> Option<&'a KeyRef> {
        self.keys.iter().find(|k| k.fingerprint == fingerprint)
    }

    /// Whether `fingerprint` belongs to a loaded key.
    pub fn is_ours(&self, fingerprint: Fingerprint) -> bool {
        self.key(fingerprint).is_some()
    }

    /// Costs a script derivation for the wallet claimed: the registered
    /// wallet a set of stated key origins belongs to, the chain and index
    /// they name, and whether the wallet's script at that index is
    /// `script_pubkey`.
    ///
    /// A wallet is claimed by any origin whose fingerprint and path
    /// belong to one of its keys; the claim is verified only by the
    /// script.
    pub(crate) fn wallet_claim<'o, I>(
        &self,
        c: &Compute,
        origins: I,
        script_pubkey: &bitcoin::Script,
    ) -> Option<WalletClaim>
    where
        I: Iterator<Item = (Fingerprint, &'o DerivationPath)> + Clone,
    {
        for (wallet, policy) in self.wallets.iter().enumerate() {
            for (fingerprint, path) in origins.clone() {
                let Some((change, index)) = policy.leaf_of(fingerprint, path) else {
                    continue;
                };
                let verified = policy
                    .script_at(c, change, index)
                    .is_ok_and(|s| s == *script_pubkey);
                return Some(WalletClaim {
                    wallet,
                    fingerprint,
                    change,
                    index,
                    verified,
                });
            }
        }
        None
    }
}

/// What a stated key origin says about a registered wallet.
pub(crate) struct WalletClaim {
    /// Position in [`Context::wallets`].
    pub wallet: usize,
    /// The fingerprint that named it.
    pub fingerprint: Fingerprint,
    /// The change chain rather than the receive chain.
    pub change: bool,
    /// Address index.
    pub index: u32,
    /// Whether the wallet's own script at that index is the script in
    /// front of us.
    pub verified: bool,
}
