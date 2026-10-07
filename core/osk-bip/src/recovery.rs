//! Recovery wallets: a primary spending path and one or more recovery
//! paths behind relative timelocks, built from keys and delays.
//!
//! This is the shape Liana writes, and the shape Nunchuk and Bitcoin
//! Keeper call an inheritance or a decaying multisig: coins move today
//! on the primary keys, and after a wait they move on the recovery keys
//! instead. [`WalletPolicy`] already reads such a wallet — its
//! addresses, its keys and its spend paths — and this module makes one
//! from its parts and reads the parts back out of one.
//!
//! ```
//! use osk_bip::recovery::{Form, Path, Recovery, RecoveryPolicy};
//!
//! let primary = "[73c5da0a/48'/1'/0'/2']tpubDFH9dgzveyD8zTbPUFuLrGmCydNvxehyNdUXKJAQN8x4aZ4j6UZqGfnqFrD4NqyaTVGKbvEW54tsvPTK2UoSbCC1PJY8iCNiwTL3RWZEheQ";
//! let recovery = "[3f635a63/48'/1'/0'/2']tpubDFPtPArj4GzBEFHohegg1Xatrc1Fi9oSox5LzuSRX91miwQxuUrEpBxpvDRsmZYJKYFhgdK3UStsjC8JKXfUbMinjFqiEM4uNwzVaCaHpys";
//! let wanted = RecoveryPolicy {
//!     primary: Path::single(primary),
//!     recovery: vec![Recovery {
//!         delay: 52_560,
//!         path: Path::single(recovery),
//!     }],
//! };
//! let policy = wanted.to_wallet_policy(Form::SegWit).unwrap();
//! assert!(policy.to_descriptor().starts_with("wsh(or_d(pk("));
//! assert_eq!(RecoveryPolicy::from_wallet_policy(&policy), Some(wanted));
//! ```
//!
//! # How the script is chosen
//!
//! Liana does not write these scripts by hand. It states the wallet as a
//! miniscript *policy* — the primary path `or`ed with each recovery
//! path `and`ed with its timelock, the primary weighted 99 against the
//! recovery's 1 — and lets the miniscript compiler choose the script.
//! The compiler's answer is not the same shape every time: a one-key
//! primary with one recovery path becomes
//! `or_d(pk(A),and_v(v:pkh(B),older(N)))`, but a two-of-two primary
//! becomes `or_i(and_v(v:pkh(B),older(N)),and_v(v:pk(A),pk(C)))`, with
//! the recovery branch written first, and a second recovery path wraps
//! the whole in another `or_i`. Writing those shapes out by hand would
//! produce descriptors Liana would not, for wallets a Liana user could
//! not then open.
//!
//! So this module states the same policy Liana states and compiles it
//! the same way. The compiler is already in this crate's `miniscript`
//! dependency, it runs once when a wallet is made and never when an
//! address is derived, and what comes out is byte-for-byte the
//! descriptor Liana writes for the same keys and delays.
//!
//! Taproot follows Liana too: the primary key path when the primary is
//! a single key, and otherwise BIP-341's NUMS point with a chain code
//! derived from the leaves' keys, so that the internal key is provably
//! unspendable and every path is a leaf.

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::fmt;
use core::str::FromStr;

use bitcoin::bip32::{ChainCode, Xpub};
use bitcoin::hashes::{Hash, sha256};
use bitcoin::secp256k1::PublicKey;

use miniscript::descriptor::{
    DerivPaths, Descriptor, DescriptorMultiXKey, DescriptorPublicKey, DescriptorXKey, Tr, Wildcard,
};
use miniscript::policy::{Concrete, Liftable, Semantic};
use miniscript::{RelLockTime, Segwitv0, Threshold};

use crate::policy::{DESCRIPTOR_SUFFIX, WalletPolicy};

/// Blocks a day of mining is expected to be worth.
pub const BLOCKS_A_DAY: u32 = 144;

/// The largest relative block delay `older()` can hold: BIP-68 keeps the
/// count in sixteen bits, so a wait longer than this cannot be written
/// at all, and a wallet that asked for one would be unspendable by the
/// path that asked.
pub const MAX_DELAY: u32 = 65_535;

/// The longest wait a relative timelock can state, in whole days:
/// [`MAX_DELAY`] blocks at [`BLOCKS_A_DAY`] blocks a day, 65 535 / 144,
/// rounded down to 455. A day more than this is more blocks than
/// `older()` can hold.
pub const MAX_DAYS: u32 = MAX_DELAY / BLOCKS_A_DAY;

/// The most keys one path may hold, which is Liana's limit and the most
/// `multi_a` and a `thresh` of `pkh`s stay readable at.
pub const MAX_KEYS: usize = 20;

/// BIP-341's NUMS point, the `H` every provably unspendable taproot
/// internal key is built on.
const NUMS: &str = "0250929b74c1a04954b78b4b6035e97a5e078a5a0f28ec96d547bfee9ace803ac0";

/// Why a recovery wallet could not be built.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// No recovery path was given. A wallet with none is an ordinary
    /// single-sig or multisig wallet, which is built elsewhere.
    NoRecovery,
    /// A delay is zero, above [`MAX_DELAY`], or repeated: two recovery
    /// paths that open on the same block are one path.
    Delay,
    /// A path has no keys, more than [`MAX_KEYS`], or a threshold
    /// outside 1..=keys.
    Path,
    /// A key is not `[fingerprint/path]xpub`, or two paths name the same
    /// key: a key in two paths makes the review of who can spend a lie.
    Key,
    /// The keys and delays given make no script this form can hold.
    Script,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Error::NoRecovery => "a recovery wallet needs at least one recovery path",
            Error::Delay => "a delay must be 1 to 65535 blocks, and no two the same",
            Error::Path => "a path needs 1 to 20 keys and a threshold within them",
            Error::Key => "key is not [fingerprint/path]xpub, or is named twice",
            Error::Script => "these keys and delays make no script of this form",
        })
    }
}

impl core::error::Error for Error {}

/// Which script the wallet pays to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Form {
    /// `wsh(<miniscript>)`: one script, every path in it.
    SegWit,
    /// `tr(<key>,<tree>)`: the primary key as the key path when the
    /// primary is one key, an unspendable internal key otherwise, and
    /// one leaf per remaining path.
    Taproot,
}

/// One way the coins can move: how many of these keys must sign.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Path {
    /// Signatures this path wants, 1 to the number of keys.
    pub threshold: usize,
    /// The keys, each `[fingerprint/path]xpub` with no multipath suffix
    /// — the form [`crate::policy::PolicyKey::key_text`] writes.
    pub keys: Vec<String>,
}

impl Path {
    /// The path one key spends alone.
    pub fn single(key: &str) -> Path {
        Path {
            threshold: 1,
            keys: alloc::vec![String::from(key)],
        }
    }

    /// The path `threshold` of `keys` spend.
    pub fn multi(threshold: usize, keys: &[&str]) -> Path {
        Path {
            threshold,
            keys: keys.iter().map(|k| String::from(*k)).collect(),
        }
    }

    fn check(&self) -> Result<(), Error> {
        if self.keys.is_empty()
            || self.keys.len() > MAX_KEYS
            || self.threshold == 0
            || self.threshold > self.keys.len()
        {
            return Err(Error::Path);
        }
        Ok(())
    }

    fn to_concrete(&self) -> Result<Concrete<DescriptorPublicKey>, Error> {
        self.check()?;
        let keys: Vec<DescriptorPublicKey> = self
            .keys
            .iter()
            .map(|key| {
                let text = alloc::format!("{key}{DESCRIPTOR_SUFFIX}");
                DescriptorPublicKey::from_str(&text).map_err(|_| Error::Key)
            })
            .collect::<Result<_, _>>()?;
        if self.threshold == 1 && keys.len() == 1 {
            return Ok(Concrete::Key(keys.into_iter().next().expect("one key")));
        }
        let subs = keys
            .into_iter()
            .map(|k| Arc::new(Concrete::Key(k)))
            .collect();
        Threshold::new(self.threshold, subs)
            .map(Concrete::Thresh)
            .map_err(|_| Error::Path)
    }
}

/// A recovery path and the wait that opens it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Recovery {
    /// Blocks after the coins were received before this path can spend.
    pub delay: u32,
    /// Who spends once the wait has passed.
    pub path: Path,
}

/// A wallet whose coins move on the primary keys today and on a
/// recovery path's keys after that path's wait.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoveryPolicy {
    /// Who spends with no wait.
    pub primary: Path,
    /// The recovery paths, shortest wait first.
    pub recovery: Vec<Recovery>,
}

/// The blocks `days` of mining is expected to be, or `None` when that is
/// more than [`MAX_DELAY`] — about 455 days, which is the longest wait
/// a relative timelock can state.
pub fn blocks_for_days(days: u32) -> Option<u32> {
    let blocks = days.checked_mul(BLOCKS_A_DAY)?;
    (blocks > 0 && blocks <= MAX_DELAY).then_some(blocks)
}

/// The whole days `blocks` of mining is expected to be, rounded down.
pub fn days_of_blocks(blocks: u32) -> u32 {
    blocks / BLOCKS_A_DAY
}

impl RecoveryPolicy {
    /// The wallet these keys and delays make, in `form`.
    ///
    /// The recovery paths are taken shortest wait first whatever order
    /// they arrive in, because that is the order the script's branches
    /// open in and the order a person reads them.
    pub fn to_wallet_policy(&self, form: Form) -> Result<WalletPolicy, Error> {
        let concrete = self.to_concrete()?;
        let descriptor = match form {
            Form::SegWit => {
                let ms = concrete.compile::<Segwitv0>().map_err(|_| Error::Script)?;
                Descriptor::new_wsh(ms).map_err(|_| Error::Script)?
            }
            Form::Taproot => taproot(concrete)?,
        };
        WalletPolicy::from_descriptor(&alloc::format!("{descriptor}")).map_err(|_| Error::Script)
    }

    /// The primary path, the recovery paths and their delays of a wallet
    /// that has them, so that a review can name who spends and when
    /// rather than showing a miniscript.
    ///
    /// `None` for every wallet that is not of this shape, including one
    /// with no timelocked path at all.
    pub fn from_wallet_policy(policy: &WalletPolicy) -> Option<RecoveryPolicy> {
        let semantic = lift(policy.script()?)?;
        // Every recovery wallet is "one of these ways", with the
        // primary the one way that imposes no wait.
        let subs = match semantic.normalized() {
            Semantic::Thresh(thresh) if thresh.k() == 1 && thresh.n() > 1 => thresh.into_data(),
            _ => return None,
        };
        let mut primary: Option<Path> = None;
        let mut recovery: BTreeMap<u32, Path> = BTreeMap::new();
        for sub in subs {
            let sub = sub.as_ref().clone();
            if is_keys(&sub) {
                // A one-of-N primary is flattened into the top-level
                // `or` by normalization, so its keys arrive one by one.
                match &mut primary {
                    Some(path) => match sub {
                        Semantic::Key(key) => path.keys.push(key_text(&key)?),
                        _ => return None,
                    },
                    None => primary = Some(read_path(&sub)?),
                }
            } else {
                let (delay, path) = read_recovery(&sub)?;
                if recovery.insert(delay, path).is_some() {
                    return None;
                }
            }
        }
        let primary = primary?;
        if recovery.is_empty() {
            return None;
        }
        Some(RecoveryPolicy {
            primary,
            recovery: recovery
                .into_iter()
                .map(|(delay, path)| Recovery { delay, path })
                .collect(),
        })
    }

    /// The miniscript policy Liana states for this wallet: the primary
    /// path, then each recovery path `and`ed with its timelock and
    /// `or`ed on, shortest wait first, the branch already built weighted
    /// 99 against the new branch's 1.
    fn to_concrete(&self) -> Result<Concrete<DescriptorPublicKey>, Error> {
        if self.recovery.is_empty() {
            return Err(Error::NoRecovery);
        }
        let mut delays: Vec<u32> = self.recovery.iter().map(|r| r.delay).collect();
        delays.sort_unstable();
        delays.dedup();
        if delays.len() != self.recovery.len() || delays.iter().any(|d| *d == 0 || *d > MAX_DELAY) {
            return Err(Error::Delay);
        }
        let mut seen: Vec<&str> = Vec::new();
        for path in core::iter::once(&self.primary).chain(self.recovery.iter().map(|r| &r.path)) {
            for key in &path.keys {
                if seen.contains(&key.as_str()) {
                    return Err(Error::Key);
                }
                seen.push(key);
            }
        }
        let mut sorted: Vec<&Recovery> = self.recovery.iter().collect();
        sorted.sort_by_key(|r| r.delay);
        let mut policy = self.primary.to_concrete()?;
        for path in sorted {
            let delay = RelLockTime::from_height(path.delay as u16);
            let branch = Concrete::And(alloc::vec![
                Arc::new(path.path.to_concrete()?),
                Arc::new(Concrete::Older(delay)),
            ]);
            policy = Concrete::Or(alloc::vec![(99, Arc::new(policy)), (1, Arc::new(branch))]);
        }
        Ok(policy)
    }
}

/// The taproot descriptor for `concrete`, with the internal key Liana
/// chooses: a key of the policy where the compiler can use one, and
/// otherwise the unspendable key the leaves determine.
fn taproot(
    concrete: Concrete<DescriptorPublicKey>,
) -> Result<Descriptor<DescriptorPublicKey>, Error> {
    // The compiler needs an internal key before it knows the tree, and
    // the unspendable key is made from the tree. So it is compiled once
    // with a placeholder; if the placeholder survives, no key of the
    // policy could serve as the key path and the real unspendable key is
    // computed from the tree and the policy compiled again.
    let placeholder = DescriptorPublicKey::XPub(DescriptorXKey::<Xpub> {
        origin: None,
        xkey: Xpub {
            public_key: nums(),
            chain_code: ChainCode::from([0u8; 32]),
            depth: 0,
            parent_fingerprint: [0u8; 4].into(),
            child_number: 0.into(),
            network: bitcoin::Network::Regtest.into(),
        },
        derivation_path: Vec::new().into(),
        wildcard: Wildcard::None,
    });
    let descriptor = concrete
        .clone()
        .compile_tr(Some(placeholder.clone()))
        .map_err(|_| Error::Script)?;
    let Descriptor::Tr(ref tr) = descriptor else {
        return Err(Error::Script);
    };
    if tr.internal_key() != &placeholder {
        return Ok(descriptor);
    }
    let unspendable = DescriptorPublicKey::MultiXPub(DescriptorMultiXKey {
        origin: None,
        xkey: unspendable_xpub(tr).ok_or(Error::Script)?,
        derivation_paths: DerivPaths::new(alloc::vec![
            [0.into()][..].into(),
            [1.into()][..].into()
        ])
        .ok_or(Error::Script)?,
        wildcard: Wildcard::Unhardened,
    });
    concrete
        .compile_tr(Some(unspendable))
        .map_err(|_| Error::Script)
}

fn nums() -> PublicKey {
    PublicKey::from_str(NUMS).expect("BIP-341's NUMS point")
}

/// The internal key of a taproot wallet no key of which can take the key
/// path: BIP-341's NUMS point with a chain code that is the SHA-256 of
/// every leaf key's public key, so that anyone holding the descriptor
/// can recompute it and see that the key path cannot be spent.
fn unspendable_xpub(tr: &Tr<DescriptorPublicKey>) -> Option<Xpub> {
    let tree = tr.tap_tree().as_ref()?;
    let first = tree.iter().flat_map(|(_, ms)| ms.iter_pk()).next()?;
    let network = multi_xkey(&first)?.network;
    let mut concat = Vec::new();
    for key in tree.iter().flat_map(|(_, ms)| ms.iter_pk()) {
        concat.extend_from_slice(&multi_xkey(&key)?.public_key.serialize());
    }
    Some(Xpub {
        public_key: nums(),
        chain_code: ChainCode::from(sha256::Hash::hash(&concat).to_byte_array()),
        depth: 0,
        parent_fingerprint: [0u8; 4].into(),
        child_number: 0.into(),
        network,
    })
}

fn multi_xkey(key: &DescriptorPublicKey) -> Option<Xpub> {
    match key {
        DescriptorPublicKey::MultiXPub(multi) => Some(multi.xkey),
        _ => None,
    }
}

/// The semantic policy of a wallet's script, with an unspendable taproot
/// internal key left out of it, as Liana reads one.
///
/// A key path no one can take is not a way to spend, so a review built
/// on this never offers one.
pub(crate) fn lift(
    script: &Descriptor<DescriptorPublicKey>,
) -> Option<Semantic<DescriptorPublicKey>> {
    match script {
        Descriptor::Tr(tr) if tr.tap_tree().is_some() => {
            let tree = tr.tap_tree().as_ref()?;
            let leaves = tree.lift().ok()?;
            let internal = multi_xkey(tr.internal_key())?;
            if Some(internal) == unspendable_xpub(tr) {
                Some(leaves)
            } else {
                Some(Semantic::Thresh(Threshold::or(
                    Arc::new(Semantic::Key(tr.internal_key().clone())),
                    Arc::new(leaves),
                )))
            }
        }
        _ => script.lift().ok(),
    }
}

/// Whether this branch is a key or a threshold of keys and nothing else,
/// which is what a primary path is and no recovery path can be.
fn is_keys(policy: &Semantic<DescriptorPublicKey>) -> bool {
    match policy {
        Semantic::Key(_) => true,
        Semantic::Thresh(thresh) => thresh
            .iter()
            .all(|s| matches!(s.as_ref(), Semantic::Key(_))),
        _ => false,
    }
}

fn read_path(policy: &Semantic<DescriptorPublicKey>) -> Option<Path> {
    match policy {
        Semantic::Key(key) => Some(Path {
            threshold: 1,
            keys: alloc::vec![key_text(key)?],
        }),
        Semantic::Thresh(thresh) if thresh.k() > 0 && thresh.n() >= thresh.k() => Some(Path {
            threshold: thresh.k(),
            keys: thresh
                .iter()
                .map(|sub| match sub.as_ref() {
                    Semantic::Key(key) => key_text(key),
                    _ => None,
                })
                .collect::<Option<_>>()?,
        }),
        _ => None,
    }
}

/// A recovery branch as its wait and its keys.
///
/// Normalization writes such a branch as `thresh(2, older(N), <keys>)`,
/// or, when the keys are an N-of-N, flattens them into
/// `thresh(n+1, older(N), key, key, …)`.
fn read_recovery(policy: &Semantic<DescriptorPublicKey>) -> Option<(u32, Path)> {
    let Semantic::Thresh(thresh) = policy else {
        return None;
    };
    let (k, subs) = (thresh.k(), thresh.iter().cloned().collect::<Vec<_>>());
    if k != subs.len() {
        // A branch whose wait is optional is not a recovery path.
        return None;
    }
    let mut delay = None;
    let mut keys: Vec<Arc<Semantic<DescriptorPublicKey>>> = Vec::new();
    for sub in subs {
        match sub.as_ref() {
            Semantic::Older(rel) if !rel.is_time_locked() => {
                if delay.is_some() {
                    return None;
                }
                delay = Some(rel.to_consensus_u32() & 0xffff);
            }
            _ => keys.push(sub),
        }
    }
    let delay = delay?;
    if keys.len() == 1 {
        return Some((delay, read_path(keys[0].as_ref())?));
    }
    // The N-of-N case: every key is its own sub of the flattened
    // threshold, so all of them must sign.
    let threshold = keys.len();
    let path = Path {
        threshold,
        keys: keys
            .iter()
            .map(|sub| match sub.as_ref() {
                Semantic::Key(key) => key_text(key),
                _ => None,
            })
            .collect::<Option<_>>()?,
    };
    Some((delay, path))
}

/// A descriptor key as the wallet writes it: `[fingerprint/path]xpub`
/// with the multipath suffix taken off.
fn key_text(key: &DescriptorPublicKey) -> Option<String> {
    let text = alloc::format!("{key}");
    Some(String::from(text.strip_suffix(DESCRIPTOR_SUFFIX)?))
}
