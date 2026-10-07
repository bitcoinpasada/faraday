//! The non-secret view of one account: its extended public key, key
//! origin, addresses and output descriptor.
//!
//! ```
//! use osk_bip::bip39::{Language, Mnemonic};
//! use osk_bip::keys::{MasterKey, Network, ScriptType};
//!
//! let m = Mnemonic::from_entropy(Language::English, &[0u8; 16]).unwrap();
//! let master = MasterKey::from_seed(&m.to_seed(b"").unwrap(), Network::Mainnet);
//! let account = master.account_xpub(ScriptType::NativeSegwit, 0).unwrap();
//! assert_eq!(
//!     account.address(false, 0).unwrap().to_string(),
//!     "bc1qcr8te4kr609gcawutmrza0j4xv80jy8z306fyu"
//! );
//! assert!(account.descriptor().starts_with("wpkh([73c5da0a/84h/0h/0h]xpub"));
//! ```

use alloc::string::{String, ToString};
use core::fmt::Write;

use bitcoin::Address;
use bitcoin::bip32::{ChildNumber, DerivationPath, Xpub};
use bitcoin::secp256k1::{PublicKey, Secp256k1};

use crate::descriptor::descriptor_checksum;
use crate::keys::{Error, Fingerprint, MultisigScriptType, Network, ScriptType};
use crate::slip132;

/// An account-level extended public key with everything needed to derive
/// its addresses and describe it to other wallets.
///
/// Nothing here is secret: it is what a watch-only wallet receives.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountXpub {
    xpub: Xpub,
    master_fingerprint: Fingerprint,
    path: DerivationPath,
    script_type: ScriptType,
    network: Network,
}

impl AccountXpub {
    pub(crate) fn new(
        xpub: Xpub,
        master_fingerprint: Fingerprint,
        path: DerivationPath,
        script_type: ScriptType,
        network: Network,
    ) -> Self {
        Self {
            xpub,
            master_fingerprint,
            path,
            script_type,
            network,
        }
    }

    /// The account extended public key.
    pub fn xpub(&self) -> &Xpub {
        &self.xpub
    }

    /// Fingerprint of the master key, as used in key origins.
    pub fn master_fingerprint(&self) -> Fingerprint {
        self.master_fingerprint
    }

    /// The account path, e.g. `84h/0h/0h`.
    pub fn path(&self) -> &DerivationPath {
        &self.path
    }

    /// The script type the account derives addresses for.
    pub fn script_type(&self) -> ScriptType {
        self.script_type
    }

    /// The network addresses are encoded for.
    pub fn network(&self) -> Network {
        self.network
    }

    /// The address at `change/index` below the account. `index` must be
    /// below 2³¹.
    pub fn address(&self, change: bool, index: u32) -> Result<Address, Error> {
        let secp = Secp256k1::verification_only();
        let chain = self.chain_xpub(&secp, change);
        self.address_from_chain(&secp, &chain, index)
    }

    /// Looks for `target` among the first `max_index + 1` receive and
    /// change addresses, nearest index first. Returns `(change, index)`.
    pub fn find_address(&self, target: &Address, max_index: u32) -> Option<(bool, u32)> {
        let secp = Secp256k1::verification_only();
        let chains = [self.chain_xpub(&secp, false), self.chain_xpub(&secp, true)];
        for index in 0..=max_index {
            for (change, chain) in [(false, &chains[0]), (true, &chains[1])] {
                match self.address_from_chain(&secp, chain, index) {
                    Ok(candidate) if &candidate == target => return Some((change, index)),
                    Ok(_) => {}
                    Err(_) => return None,
                }
            }
        }
        None
    }

    /// Standard BIP-32 encoding: `xpub…` on mainnet, `tpub…` elsewhere.
    pub fn xpub_string(&self) -> String {
        self.xpub.to_string()
    }

    /// SLIP-132 encoding: `ypub`/`upub` for nested SegWit and `zpub`/`vpub`
    /// for native SegWit. Legacy and Taproot have no SLIP-132 prefix and
    /// return the same as [`xpub_string`](Self::xpub_string).
    pub fn slip132_string(&self) -> String {
        slip132::encode_xpub(&self.xpub, self.script_type)
    }

    /// Ranged single-signature output descriptor with key origin and
    /// checksum, e.g. `wpkh([73c5da0a/84h/0h/0h]xpub…/<0;1>/*)#…`.
    pub fn descriptor(&self) -> String {
        let mut key = String::new();
        // Formatting into a String cannot fail; unwrap via expect.
        write!(key, "[{}", self.master_fingerprint).expect("string write");
        for child in &self.path {
            write!(key, "/{child:#}").expect("string write");
        }
        write!(key, "]{}/<0;1>/*", self.xpub).expect("string write");

        let mut desc = match self.script_type {
            ScriptType::Legacy => alloc::format!("pkh({key})"),
            ScriptType::NestedSegwit => alloc::format!("sh(wpkh({key}))"),
            ScriptType::NativeSegwit => alloc::format!("wpkh({key})"),
            ScriptType::Taproot => alloc::format!("tr({key})"),
        };
        let checksum = descriptor_checksum(&desc).expect("descriptor uses only charset characters");
        desc.push('#');
        desc.push_str(core::str::from_utf8(&checksum).expect("checksum is ascii"));
        desc
    }

    fn chain_xpub<C: bitcoin::secp256k1::Verification>(
        &self,
        secp: &Secp256k1<C>,
        change: bool,
    ) -> Xpub {
        let child = ChildNumber::from_normal_idx(u32::from(change)).expect("0 or 1");
        self.xpub
            .derive_pub(secp, &[child])
            .expect("normal child derivation yields a valid key")
    }

    fn address_from_chain<C: bitcoin::secp256k1::Verification>(
        &self,
        secp: &Secp256k1<C>,
        chain: &Xpub,
        index: u32,
    ) -> Result<Address, Error> {
        let child = ChildNumber::from_normal_idx(index).map_err(|_| Error::IndexOutOfRange)?;
        // Fails only on the 2^-128 event that a child key is invalid.
        let key = chain
            .derive_pub(secp, &[child])
            .expect("normal child derivation yields a valid key");
        let network = bitcoin::Network::from(self.network);
        Ok(match self.script_type {
            ScriptType::Legacy => Address::p2pkh(key.to_pub(), network),
            ScriptType::NestedSegwit => Address::p2shwpkh(&key.to_pub(), network),
            ScriptType::NativeSegwit => Address::p2wpkh(&key.to_pub(), network),
            ScriptType::Taproot => Address::p2tr(secp, key.to_x_only_pub(), None, network),
        })
    }
}

/// A BIP-48 multisig account: the account extended public key and where
/// it came from.
///
/// It is deliberately not an [`AccountXpub`]. That type's whole purpose
/// is to produce addresses and a single-signature descriptor from one
/// key, and a multisig account can do neither without the cosigners, so
/// giving it a second meaning would make `address` and `descriptor` lie.
/// What one key can honestly do alone is derive its own public key on a
/// chain, which is [`key_at`](Self::key_at) and is what change
/// verification needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MultisigAccountXpub {
    xpub: Xpub,
    master_fingerprint: Fingerprint,
    path: DerivationPath,
    script_type: MultisigScriptType,
    network: Network,
}

impl MultisigAccountXpub {
    pub(crate) fn new(
        xpub: Xpub,
        master_fingerprint: Fingerprint,
        path: DerivationPath,
        script_type: MultisigScriptType,
        network: Network,
    ) -> Self {
        Self {
            xpub,
            master_fingerprint,
            path,
            script_type,
            network,
        }
    }

    /// The account extended public key.
    pub fn xpub(&self) -> &Xpub {
        &self.xpub
    }

    /// Fingerprint of the master key, as used in key origins.
    pub fn master_fingerprint(&self) -> Fingerprint {
        self.master_fingerprint
    }

    /// The account path, e.g. `48h/0h/0h/2h`.
    pub fn path(&self) -> &DerivationPath {
        &self.path
    }

    /// Which of BIP-48's two script types the account is for.
    pub fn script_type(&self) -> MultisigScriptType {
        self.script_type
    }

    /// The network the account belongs to.
    pub fn network(&self) -> Network {
        self.network
    }

    /// This key's public key at `change/index` below the account. `index`
    /// must be below 2³¹.
    pub fn key_at(&self, change: bool, index: u32) -> Result<PublicKey, Error> {
        let secp = Secp256k1::verification_only();
        let chain = ChildNumber::from_normal_idx(u32::from(change)).expect("0 or 1");
        let leaf = ChildNumber::from_normal_idx(index).map_err(|_| Error::IndexOutOfRange)?;
        Ok(self
            .xpub
            .derive_pub(&secp, &[chain, leaf])
            .expect("normal child derivation yields a valid key")
            .public_key)
    }
}
