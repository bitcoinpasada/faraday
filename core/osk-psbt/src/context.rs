//! What the inspector knows about the loaded keys: fingerprints and
//! account-level extended public keys. Nothing here is secret.

use alloc::vec::Vec;

use bitcoin::ScriptBuf;
use bitcoin::bip32::{ChildNumber, DerivationPath};
use bitcoin::secp256k1::{PublicKey, Secp256k1, Verification};
use osk_bip::account::{AccountXpub, MultisigAccountXpub};
use osk_bip::keys::{self, Fingerprint, MasterKey, MultisigScriptType, Network, ScriptType};
use osk_bip::policy::WalletPolicy;

/// A loaded key as the inspector sees it: its master fingerprint and the
/// account xpubs it can derive addresses from.
///
/// Change and input ownership are verified by re-deriving from these
/// xpubs (`docs/PLANNING.md` §8.4 #3), so only paths below a listed
/// account can be verified. The multisig accounts are BIP-48's two,
/// which let a multisig change output be checked before any wallet
/// policy is registered; the cosigners still come from the PSBT's own
/// witness script, so what they buy is the one key the device can prove
/// is its own.
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

    /// The four single-signature accounts (BIP-44/49/84/86) and BIP-48's
    /// two multisig accounts at index `account` on the master key's
    /// network.
    pub fn from_master(master: &MasterKey, account: u32) -> Result<Self, keys::Error> {
        let mut accounts = Vec::with_capacity(4);
        for script_type in ScriptType::ALL {
            accounts.push(master.account_xpub(script_type, account)?);
        }
        let mut multisig = Vec::with_capacity(2);
        for script_type in MultisigScriptType::ALL {
            multisig.push(master.multisig_account_xpub(script_type, account)?);
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

    /// Re-derives `path` if it is `change/index` below one of the BIP-48
    /// accounts, with both leaf components unhardened. The script is not
    /// part of the answer: a multisig script needs the cosigners.
    pub(crate) fn derive_multisig(&self, path: &DerivationPath) -> Option<(bool, u32, PublicKey)> {
        let components: Vec<ChildNumber> = path.to_u32_vec().iter().map(|&n| n.into()).collect();
        for account in &self.multisig {
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
            return Some((change, index, account.key_at(change, index).ok()?));
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

/// A loaded share of a threshold wallet as the inspector sees it: its
/// public share and nothing else.
///
/// A share derives nothing and has no master fingerprint, so what names
/// it on an input is the public share the group's record lists
/// (`docs/PLANNING.md` §16.103).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShareRef {
    /// `secshare · G`, which is the entry the record carries for it.
    pub pubshare: PublicKey,
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
    /// a multisig change output can only be claimed.
    pub wallets: &'a [WalletPolicy],
    /// What the open MuSig2 session holds, when one is open
    /// (`docs/PLANNING.md` §16.100). It decides whether a public nonce
    /// of this device's already on an input can still be signed under or
    /// has to be replaced.
    pub musig_session: Option<&'a crate::musig::MusigSessionView>,
    /// The loaded shares of threshold wallets (`docs/PLANNING.md`
    /// §16.103). A share is named on an input by its public share, so
    /// this is all the inspector needs to know it holds one.
    pub shares: &'a [ShareRef],
    /// The carry file's secret section, when the transaction arrived as
    /// one. It decides whether a threshold input this device holds a
    /// share of is a later location or a refusal.
    pub carry: Option<&'a crate::threshold::CarrySection>,
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

    /// The registered wallet a set of stated key origins belongs to, the
    /// chain and index they name, and whether the wallet's script at
    /// that index is `script_pubkey`.
    ///
    /// A wallet is claimed by any origin whose fingerprint and path
    /// belong to one of its keys; the claim is verified only by the
    /// script.
    pub(crate) fn wallet_claim<'o, I>(
        &self,
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
                    .script_at(change, index)
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
