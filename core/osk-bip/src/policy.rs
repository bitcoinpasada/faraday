//! BIP-388 wallet policies: the form Sparrow, Ledger and Coldcard
//! exchange a multisig wallet in.
//!
//! A policy is a descriptor template whose keys are the placeholders
//! `@0/**`, `@1/**` …, plus the vector of keys those placeholders stand
//! for, each `[fingerprint/path]xpub`. The template is what a person
//! reads and compares; the keys are what the device derives from.
//!
//! ```
//! use osk_bip::policy::WalletPolicy;
//!
//! let policy = WalletPolicy::parse(
//!     "pkh(@0/**)\n\
//!      [d34db33f/44'/0'/0']xpub6ERApfZwUNrhLCkDtcHTcxd75RbzS1ed54G1LkBUHQVHQKqhMkhgbmJbZRkrgZw4koxb5JaHWkY4ALHY2grBGRjaDMzQLcgJvLJuZZvRcEL",
//! )
//! .unwrap();
//! assert_eq!(
//!     policy.to_descriptor(),
//!     "pkh([d34db33f/44'/0'/0']xpub6ERApfZwUNrhLCkDtcHTcxd75RbzS1ed54G1LkBUHQVHQKqhMkhgbmJbZRkrgZw4koxb5JaHWkY4ALHY2grBGRjaDMzQLcgJvLJuZZvRcEL/<0;1>/*)"
//! );
//! ```
//!
//! Accepted templates: `multi` and `sortedmulti` inside `wsh`,
//! `sh(wsh(…))` or `sh(…)`, the four single-key templates,
//! `tr(musig(@0,@1,…)/**)` — the BIP-390 MuSig2 wallet whose addresses
//! come from the aggregate key — and, through `miniscript`,
//! `wsh(<miniscript>)`, `sh(wsh(<miniscript>))` and
//! `tr(<key>,<tree of leaves>)`. The last three are the wallets Liana,
//! Sparrow and Nunchuk write for timelocked recovery and inheritance:
//! their addresses, their keys and the ways they can be spent
//! ([`WalletPolicy::spend_paths`]) are read here.
//!
//! A tap tree's leaves include BIP 387's `multi_a` and `sortedmulti_a`,
//! the k-of-n taproot multisig, whose internal key is usually the NUMS
//! point ([`WalletPolicy::key_path_unspendable`]). `miniscript` reads
//! `multi_a`; the sorted form is read through [`crate::tapmulti`],
//! which puts the keys in the script's order at every index.

use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;
use core::str::FromStr;

use bitcoin::bip32::{ChildNumber, DerivationPath, Xpub};
use bitcoin::opcodes::all::OP_CHECKMULTISIG;
use bitcoin::script::Builder;
use bitcoin::secp256k1::{Secp256k1, Verification};
use bitcoin::{Address, PublicKey, ScriptBuf};

use miniscript::descriptor::{DescriptorMultiXKey, DescriptorPublicKey, Wildcard};
use miniscript::{Descriptor, ForEachKey, ToPublicKey};

use crate::descriptor::descriptor_checksum;
use crate::keys::{Fingerprint, Network, ScriptType};
use crate::silent_wallet::SilentWallet;
use crate::spend::SpendPath;
use crate::threshold::ThresholdRecord;

/// Why a policy or a descriptor could not be read as a wallet policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// The template is not one of the supported script forms.
    Template,
    /// A key is not `[fingerprint/path]xpub`, or two keys are the same.
    Key,
    /// A placeholder is repeated, out of order, or names no key; or a
    /// `musig()` holds fewer than two participants.
    Placeholder,
    /// A key expression's derivation is not `/**` or `/<0;1>/*`.
    Derivation,
    /// The threshold is not between 1 and the number of keys (at most
    /// 15, the most `OP_CHECKMULTISIG` takes).
    Threshold,
    /// The descriptor carries a `#checksum` that does not match.
    Checksum,
    /// A threshold record's shares are not numbered 0, 1, … in order
    /// with none missing.
    Share,
    /// A threshold record's shares are not one group: they do not lie on
    /// one polynomial, or that polynomial does not give its group key.
    Group,
    /// A threshold record's descriptor is not the extended public key
    /// its group key makes.
    Xpub,
    /// An address index is 2³¹ or larger.
    IndexOutOfRange,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Error::Template => "unsupported descriptor template",
            Error::Key => "key is not [fingerprint/path]xpub",
            Error::Placeholder => "key placeholders are not @0, @1, … in order",
            Error::Derivation => "derivation must be /** or /<0;1>/*",
            Error::Threshold => "threshold must be between 1 and the number of keys",
            Error::Checksum => "descriptor checksum does not match",
            Error::Share => "shares must be numbered 0, 1, \u{2026} with none missing",
            Error::Group => "the shares do not give this group key",
            Error::Xpub => "the extended public key is not the group key's",
            Error::IndexOutOfRange => "index must be below 2^31",
        })
    }
}

impl core::error::Error for Error {}

/// The multipath suffix a template writes: `/**`, BIP-388's shorthand
/// for the receive chain then the change chain.
pub const TEMPLATE_SUFFIX: &str = "/**";
/// The same two chains as a descriptor writes them (BIP-389).
pub const DESCRIPTOR_SUFFIX: &str = "/<0;1>/*";

/// How a multisig script is wrapped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Wrapper {
    /// `sh(multi(…))`.
    Sh,
    /// `wsh(multi(…))`.
    Wsh,
    /// `sh(wsh(multi(…)))`.
    ShWsh,
}

/// What a policy's template pays to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Template {
    /// `multi` or `sortedmulti` inside a wrapper.
    Multi {
        /// Signatures required.
        threshold: usize,
        /// Whether the keys are sorted in the script (`sortedmulti`).
        sorted: bool,
        /// How the multisig script is wrapped.
        wrapper: Wrapper,
    },
    /// One of the four single-key templates.
    Single {
        /// Which of them.
        script: ScriptType,
    },
    /// `tr(musig(@0,@1,…)/**)`: the BIP-390 MuSig2 key expression, with
    /// the wallet's derivation on the `musig()` and not on any
    /// participant, which is how BIP-388 and BIP-390 write a wallet of
    /// one aggregate key. Every participant signs.
    MuSig,
    /// `tr(XPUB/<0;1>/*)` over the synthetic extended public key of a
    /// FROST group key, which the wallet's record states along with the
    /// public share of every participant (`docs/PLANNING.md` §16.103).
    /// Any `threshold` of `participants` shares sign, and the chain sees
    /// one key and one signature.
    Threshold {
        /// Shares that sign.
        threshold: usize,
        /// Participants the group was dealt to.
        participants: usize,
    },
    /// A silent payments wallet (BIP-352): one scan key and one spend
    /// key derived from one of this device's keys, which pays to
    /// taproot outputs a sender computes and no descriptor lists. Its
    /// text is [`crate::silent_wallet::SilentWallet`]'s record and its
    /// addresses are the `sp1q…` string and its labels, so it has no
    /// key expression and derives no address at an index.
    Silent,
    /// `wsh(<miniscript>)` or `sh(wsh(<miniscript>))`: a script whose
    /// spend paths are what [`WalletPolicy::spend_paths`] lists.
    Miniscript {
        /// Which of the two wrappers the script is in.
        kind: MiniscriptKind,
    },
    /// `tr(<key>,<tree of miniscript leaves>)`: the key path first, then
    /// one leaf per branch of the tree.
    Tree,
}

/// How a miniscript is wrapped. A bare `sh(<miniscript>)` is not one of
/// these: its spends are legacy script, which nothing else here reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MiniscriptKind {
    /// `wsh(<miniscript>)`.
    Wsh,
    /// `sh(wsh(<miniscript>))`.
    ShWsh,
}

/// Where a key came from: the master it descends from and the path down
/// to the xpub, as the descriptor writes them between brackets.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Origin {
    text: String,
    fingerprint: Fingerprint,
    path: DerivationPath,
}

/// One key of a policy: its origin where it has one, and its extended
/// public key.
///
/// A key read from a bare xpub has no origin. Addresses derive without
/// one; what an origin is needed for is matching a PSBT's key paths and
/// writing the BIP-388 form, and neither is guessed at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyKey {
    origin: Option<Origin>,
    xpub: Xpub,
}

impl PolicyKey {
    /// The master fingerprint the origin names, or `None` when the key
    /// arrived with no origin. This is never the xpub's own identifier:
    /// a key whose master is unknown says so.
    pub fn fingerprint(&self) -> Option<Fingerprint> {
        self.origin.as_ref().map(|o| o.fingerprint)
    }

    /// The BIP-32 fingerprint of the xpub itself — the first four bytes
    /// of its own identifier — which every key has and which names the
    /// key as a parent, not as a master.
    pub fn identifier_fingerprint(&self) -> Fingerprint {
        Fingerprint(self.xpub.fingerprint().to_bytes())
    }

    /// The derivation path from that master down to the xpub, when the
    /// key has an origin.
    pub fn path(&self) -> Option<&DerivationPath> {
        self.origin.as_ref().map(|o| &o.path)
    }

    /// The account extended public key.
    pub fn xpub(&self) -> &Xpub {
        &self.xpub
    }

    /// Reads one key expression, `[fingerprint/path]xpub` or a bare
    /// xpub, without a multipath suffix: the form a BSMS key record's
    /// third line carries.
    pub fn parse(text: &str) -> Result<Self, Error> {
        parse_key(text)
    }

    /// The key as a descriptor key expression, without the multipath
    /// suffix: `[fingerprint/path]xpub`, or the bare xpub when there is
    /// no origin, which BIP-380 allows.
    pub fn key_text(&self) -> String {
        match &self.origin {
            Some(o) => alloc::format!("[{}]{}", o.text, self.xpub),
            None => alloc::format!("{}", self.xpub),
        }
    }
}

/// A registered wallet: a descriptor template and its keys.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WalletPolicy {
    template: Template,
    keys: Vec<PolicyKey>,
    /// The parsed descriptor, for the two templates whose script,
    /// addresses and spend paths come from `miniscript` rather than from
    /// this module. `None` for every other template.
    script: Option<Descriptor<DescriptorPublicKey>>,
    /// The threshold of a `sortedmulti_a` leaf, which `script` holds as
    /// the `multi_a` over the same keys because `miniscript` has no
    /// `sortedmulti_a`. `None` for every other wallet, including the
    /// `multi_a` a descriptor states in that order.
    sorted_a: Option<usize>,
    /// The group record, for [`Template::Threshold`] and no other
    /// template, which is the text such a wallet is written as.
    record: Option<ThresholdRecord>,
    /// The silent payments record, for [`Template::Silent`] and no
    /// other template, which is likewise the text it is written as.
    silent: Option<SilentWallet>,
}

impl WalletPolicy {
    /// Reads the two-part form: the descriptor template on the first
    /// line, then one `[fingerprint/path]xpub` per line (or several,
    /// separated by commas).
    pub fn parse(text: &str) -> Result<Self, Error> {
        let mut lines = text
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty() && !l.starts_with('#'));
        let template = lines.next().ok_or(Error::Template)?;
        let mut keys = Vec::new();
        for line in lines {
            for key in line.split(',') {
                let key = key.trim().trim_matches('"').trim();
                if !key.is_empty() {
                    keys.push(key);
                }
            }
        }
        Self::from_parts(template, &keys)
    }

    /// Reads a template and its key vector as separate pieces.
    pub fn from_parts(template: &str, keys: &[&str]) -> Result<Self, Error> {
        let (structure, placeholders) = match parse_template(template) {
            Ok(parsed) => parsed,
            // A template none of the named shapes matches is a
            // miniscript, which is read from the descriptor the keys
            // make of it.
            Err(Error::Template) => return Self::from_descriptor(&fill(template, keys)?),
            Err(error) => return Err(error),
        };
        if placeholders != keys.len() {
            return Err(Error::Placeholder);
        }
        let keys: Vec<PolicyKey> = keys
            .iter()
            .map(|key| parse_key(key))
            .collect::<Result<_, _>>()?;
        check(&structure, &keys)?;
        Ok(WalletPolicy {
            template: structure,
            keys,
            script: None,
            sorted_a: None,
            record: None,
            silent: None,
        })
    }

    /// Reads a plain multipath descriptor as a policy, so that a
    /// descriptor and the policy made from it show the same review.
    /// A `#checksum`, if present, must hold.
    pub fn from_descriptor(text: &str) -> Result<Self, Error> {
        let body = match text.trim().split_once('#') {
            Some((body, _)) => {
                if !crate::descriptor::verify_checksum(text.trim()) {
                    return Err(Error::Checksum);
                }
                body
            }
            None => text.trim(),
        };
        let mut keys = Vec::new();
        let template = match parse_structure(body, &mut |expr| {
            keys.push(parse_key(expr)?);
            Ok(())
        }) {
            Ok(template) => template,
            Err(Error::Template) => return parse_miniscript(body),
            Err(error) => return Err(error),
        };
        check(&template, &keys)?;
        Ok(WalletPolicy {
            template,
            keys,
            script: None,
            sorted_a: None,
            record: None,
            silent: None,
        })
    }

    /// Reads any of the three forms: a threshold wallet's record, the
    /// two-part policy, or a plain descriptor.
    pub fn parse_any(text: &str) -> Result<Self, Error> {
        if ThresholdRecord::looks_like_record(text) {
            return Self::from_record(text);
        }
        if SilentWallet::looks_like_record(text) {
            return SilentWallet::parse(text)
                .map(Self::of_silent)
                .ok_or(Error::Template);
        }
        if text.contains('@') {
            Self::parse(text)
        } else {
            Self::from_descriptor(text)
        }
    }

    /// The single-key wallet of one extended public key at `script`,
    /// with no origin: what a bare `xpub` (or its SLIP-132 spelling)
    /// arrives as. Its addresses derive; its BIP-388 form does not
    /// exist, and nothing invents one.
    pub fn from_xpub(xpub: Xpub, script: ScriptType) -> Result<Self, Error> {
        let keys = alloc::vec![PolicyKey { origin: None, xpub }];
        let template = Template::Single { script };
        check(&template, &keys)?;
        Ok(WalletPolicy {
            template,
            keys,
            script: None,
            sorted_a: None,
            record: None,
            silent: None,
        })
    }

    /// The threshold wallet a group record states: one key, which is the
    /// group key's synthetic extended public key, and the record itself
    /// as the wallet's text.
    pub fn from_record(text: &str) -> Result<Self, Error> {
        let record = ThresholdRecord::parse(text)?;
        Ok(Self::of_record(record))
    }

    /// The same, for a record already read.
    pub fn of_record(record: ThresholdRecord) -> Self {
        let template = Template::Threshold {
            threshold: record.t(),
            participants: record.n(),
        };
        WalletPolicy {
            template,
            keys: alloc::vec![PolicyKey {
                origin: None,
                xpub: record.xpub,
            }],
            script: None,
            sorted_a: None,
            record: Some(record),
            silent: None,
        }
    }

    /// The group record, for a threshold wallet and for nothing else.
    pub fn record(&self) -> Option<&ThresholdRecord> {
        self.record.as_ref()
    }

    /// The silent payments wallet a record states: no key expression at
    /// all, because the two public keys the address carries are not
    /// extended keys and nothing derives from them at an index.
    pub fn of_silent(silent: SilentWallet) -> Self {
        WalletPolicy {
            template: Template::Silent,
            keys: Vec::new(),
            script: None,
            sorted_a: None,
            record: None,
            silent: Some(silent),
        }
    }

    /// That record, for a silent payments wallet and for nothing else.
    pub fn silent(&self) -> Option<&SilentWallet> {
        self.silent.as_ref()
    }

    /// What the template pays to.
    pub fn template(&self) -> Template {
        self.template
    }

    /// The keys, in placeholder order.
    pub fn keys(&self) -> &[PolicyKey] {
        &self.keys
    }

    /// The internal key of a `tr()` wallet whose internal key is a plain
    /// public key rather than one of the wallet's own extended keys:
    /// the NUMS point of a script-only wallet, or another key a
    /// descriptor states. `None` for every other wallet, including a
    /// `tr()` over an extended key, whose key path is
    /// [`keys`](Self::keys)`[0]`.
    pub fn internal_key(&self) -> Option<bitcoin::secp256k1::XOnlyPublicKey> {
        let Some(Descriptor::Tr(tr)) = &self.script else {
            return None;
        };
        match tr.internal_key() {
            DescriptorPublicKey::Single(_) => Some(
                tr.internal_key()
                    .clone()
                    .at_derivation_index(0)
                    .ok()?
                    .to_x_only_pubkey(),
            ),
            _ => None,
        }
    }

    /// Whether the key path cannot be spent: the internal key is BIP
    /// 341's NUMS point, so the tree is the only way to spend.
    ///
    /// `false` for every wallet whose key path someone holds, and for a
    /// wallet whose internal key is some other key nobody claims — a
    /// device cannot tell that one from a key path it simply does not
    /// have.
    pub fn key_path_unspendable(&self) -> bool {
        self.internal_key()
            .as_ref()
            .is_some_and(crate::tapmulti::is_nums)
    }

    /// `(signatures required, keys)` for a BIP 387 tapscript multisig:
    /// a `tr()` whose tree is one `multi_a` or `sortedmulti_a` leaf.
    pub fn tapscript_quorum(&self) -> Option<(usize, usize)> {
        if let Some(threshold) = self.sorted_a {
            return Some((threshold, self.keys.len()));
        }
        let Some(Descriptor::Tr(tr)) = &self.script else {
            return None;
        };
        let text = alloc::format!("{tr}");
        let mut leaves = tr.iter_scripts();
        let (_, leaf) = leaves.next()?;
        if leaves.next().is_some() || !text.contains("multi_a(") {
            return None;
        }
        let threshold = multi_a_threshold(&text)?;
        let keys = leaf.iter_pk().count();
        (keys == self.keys.len()).then_some((threshold, keys))
    }

    /// Whether the tapscript multisig's keys are sorted in the script
    /// (`sortedmulti_a`) rather than kept in the order the descriptor
    /// writes them (`multi_a`).
    pub fn tapscript_sorted(&self) -> bool {
        self.sorted_a.is_some()
    }

    /// `(signatures required, keys)` for a multisig policy.
    ///
    /// A MuSig2 wallet has none: the output carries one key and one
    /// signature, so "2 of 2" would state a script fact that is not
    /// there. What it has instead is the number of keys, which
    /// [`keys`](Self::keys) gives.
    pub fn quorum(&self) -> Option<(usize, usize)> {
        match self.template {
            Template::Multi { threshold, .. } => Some((threshold, self.keys.len())),
            // A threshold wallet has none either, and for the same
            // reason: the output carries one key and one signature, and
            // the `t` of `n` the template states is a fact about the
            // shares, not about the script.
            Template::Single { .. }
            | Template::MuSig
            | Template::Threshold { .. }
            | Template::Silent
            | Template::Miniscript { .. }
            | Template::Tree => None,
        }
    }

    /// The script type the template's outermost function names, in the
    /// one vocabulary the screens use.
    pub fn script_type(&self) -> ScriptType {
        match self.template {
            Template::Single { script } => script,
            Template::MuSig | Template::Threshold { .. } | Template::Silent => ScriptType::Taproot,
            Template::Multi { wrapper, .. } => match wrapper {
                Wrapper::Sh => ScriptType::Legacy,
                Wrapper::Wsh => ScriptType::NativeSegwit,
                Wrapper::ShWsh => ScriptType::NestedSegwit,
            },
            Template::Miniscript { kind } => match kind {
                MiniscriptKind::Wsh => ScriptType::NativeSegwit,
                MiniscriptKind::ShWsh => ScriptType::NestedSegwit,
            },
            Template::Tree => ScriptType::Taproot,
        }
    }

    /// The descriptor template, with `@i` in place of every key.
    pub fn template_text(&self) -> String {
        if self.script.is_some() {
            // The descriptor is what `miniscript` wrote, so the template
            // is that text with each key expression — which is unique
            // within it — put back as its placeholder.
            let mut text = self.to_descriptor();
            for (i, key) in self.keys.iter().enumerate().rev() {
                let expr = alloc::format!("{}{DESCRIPTOR_SUFFIX}", key.key_text());
                text = text.replace(&expr, &alloc::format!("@{i}{TEMPLATE_SUFFIX}"));
            }
            return text;
        }
        let keys: Vec<String> = (0..self.keys.len())
            .map(|i| alloc::format!("@{i}"))
            .collect();
        self.render(&keys, TEMPLATE_SUFFIX)
    }

    /// The policy in the two-part form [`parse`](Self::parse) reads: the
    /// template, then one key per line.
    ///
    /// BIP-388 requires an origin on every key, so a single-key wallet
    /// read from a bare extended public key has no two-part form; what
    /// comes back for it is [`to_descriptor`](Self::to_descriptor),
    /// which is a valid descriptor and which
    /// [`parse_any`](Self::parse_any) reads back as the same wallet. A
    /// refusal would leave the caller with nothing to write down, and
    /// inventing an origin would state a fact the device does not have.
    pub fn to_text(&self) -> String {
        if let Some(record) = &self.record {
            return record.to_text();
        }
        if let Some(silent) = &self.silent {
            return silent.to_text();
        }
        if self.keys.iter().any(|k| k.origin.is_none()) {
            return self.to_descriptor();
        }
        let mut out = self.template_text();
        for key in &self.keys {
            out.push('\n');
            out.push_str(&key.key_text());
        }
        out
    }

    /// The multipath descriptor the policy stands for, without a
    /// checksum.
    pub fn to_descriptor(&self) -> String {
        // A silent payments wallet has no descriptor anyone can be
        // handed: BIP-392's `sp(spscan1q…)` carries the scan private
        // key, so it is an export and not an identity. What identifies
        // the wallet is its address, which is in the descriptor
        // character set and is what its checksum is taken over.
        if let Some(silent) = &self.silent {
            return silent.address();
        }
        if let Some(script) = &self.script {
            let text = alloc::format!("{script}");
            // `miniscript` writes the checksum; this crate's callers ask
            // for it separately.
            let text = String::from(text.split('#').next().unwrap_or(&text));
            // A sorted leaf is held as its `multi_a` twin and written
            // back as what it is.
            if self.sorted_a.is_some() {
                return text.replacen("multi_a(", "sortedmulti_a(", 1);
            }
            return text;
        }
        let keys: Vec<String> = self.keys.iter().map(PolicyKey::key_text).collect();
        self.render(&keys, DESCRIPTOR_SUFFIX)
    }

    /// The template written out with `keys` in place of its keys and
    /// `suffix` as the multipath derivation. A MuSig2 wallet derives
    /// from the aggregate, so its suffix follows the `musig()`; every
    /// other template derives per key.
    fn render(&self, keys: &[String], suffix: &str) -> String {
        match self.template {
            Template::MuSig => alloc::format!("tr(musig({}){suffix})", keys.join(",")),
            template => {
                let args: Vec<String> =
                    keys.iter().map(|k| alloc::format!("{k}{suffix}")).collect();
                wrap(template, &args.join(","))
            }
        }
    }

    /// The same descriptor with BIP-388's `/**` in place of the two
    /// chains: the form a BSMS descriptor record's second line carries.
    /// A wallet whose script comes from `miniscript` has no such form
    /// here, and gets the plain descriptor.
    pub fn to_descriptor_template(&self) -> String {
        if self.silent.is_some() {
            return self.to_descriptor();
        }
        if self.script.is_some() {
            return self.to_descriptor();
        }
        let keys: Vec<String> = self.keys.iter().map(PolicyKey::key_text).collect();
        self.render(&keys, TEMPLATE_SUFFIX)
    }

    /// That descriptor with its BIP-380 checksum, the form a person
    /// compares against another wallet's.
    pub fn to_descriptor_checksummed(&self) -> String {
        let mut desc = self.to_descriptor();
        desc.push('#');
        desc.push_str(&self.checksum());
        desc
    }

    /// The eight-character checksum of that descriptor.
    pub fn checksum(&self) -> String {
        let desc = self.to_descriptor();
        let sum = descriptor_checksum(&desc).expect("descriptor uses only charset characters");
        String::from_utf8_lossy(&sum).into_owned()
    }

    /// The script the policy pays to at `change`/`index`.
    pub fn script_at(&self, change: bool, index: u32) -> Result<ScriptBuf, Error> {
        let secp = Secp256k1::verification_only();
        self.script_at_with(&secp, change, index)
    }

    /// The same script, with a caller's secp context.
    pub fn script_at_with<C: Verification>(
        &self,
        secp: &Secp256k1<C>,
        change: bool,
        index: u32,
    ) -> Result<ScriptBuf, Error> {
        if let Some(script) = &self.script {
            let chains = script
                .clone()
                .into_single_descriptors()
                .map_err(|_| Error::Derivation)?;
            let chain = chains
                .get(usize::from(change))
                .ok_or(Error::Derivation)?
                .clone();
            let derived = chain
                .at_derivation_index(index)
                .map_err(|_| Error::IndexOutOfRange)?
                .derived_descriptor(secp)
                .map_err(|_| Error::Key)?;
            let Some(threshold) = self.sorted_a else {
                return Ok(derived.script_pubkey());
            };
            let (internal, keys) =
                crate::tapmulti::derived_keys(&derived).ok_or(Error::Template)?;
            let leaf = crate::tapmulti::sorted_multi_a_script(threshold, &keys);
            return crate::tapmulti::output_script(secp, internal, &leaf).ok_or(Error::Template);
        }
        if self.template == Template::MuSig {
            let xpubs: Vec<Xpub> = self.keys.iter().map(|k| k.xpub).collect();
            let expr = crate::musig::MusigExpr::from_xpubs(&xpubs).map_err(musig_error)?;
            return crate::musig::script_pubkey_at_with(secp, &expr, change, index)
                .map_err(musig_error);
        }
        let mut keys = Vec::with_capacity(self.keys.len());
        for key in &self.keys {
            keys.push(derive(secp, key, change, index)?);
        }
        Ok(match self.template {
            Template::Multi {
                threshold,
                sorted,
                wrapper,
            } => {
                if sorted {
                    keys.sort_by_key(|k| k.to_bytes());
                }
                let script = multisig_script(threshold, &keys);
                match wrapper {
                    Wrapper::Sh => script.to_p2sh(),
                    Wrapper::Wsh => script.to_p2wsh(),
                    Wrapper::ShWsh => script.to_p2wsh().to_p2sh(),
                }
            }
            // Both returned above: the MuSig2 aggregate key rather than
            // the per-key list is what pays, and a miniscript's script
            // is `miniscript`'s to build.
            Template::MuSig | Template::Silent | Template::Miniscript { .. } | Template::Tree => {
                return Err(Error::Template);
            }
            Template::Single { script } => single_script(secp, script, keys[0])?,
            // The group key pays where a single taproot key pays: the
            // synthetic xpub derived at this chain and index, tweaked by
            // BIP 341 with no tree, which is what `tr(XPUB/<0;1>/*)`
            // stands for.
            Template::Threshold { .. } => single_script(secp, ScriptType::Taproot, keys[0])?,
        })
    }

    /// The address that script is, on `network`.
    pub fn address_at(&self, network: Network, change: bool, index: u32) -> Result<Address, Error> {
        let script = self.script_at(change, index)?;
        Address::from_script(&script, bitcoin::Network::from(network)).map_err(|_| Error::Template)
    }

    /// Looks for `target` among the first `max_index + 1` receive and
    /// change addresses on `network`, nearest index first. Returns
    /// `(change, index)`.
    pub fn find_address(
        &self,
        network: Network,
        target: &Address,
        max_index: u32,
    ) -> Option<(bool, u32)> {
        (0..=max_index)
            .flat_map(|index| [(false, index), (true, index)])
            .find(|&(change, index)| {
                self.address_at(network, change, index)
                    .is_ok_and(|a| &a == target)
            })
    }

    /// The key of this policy whose origin names `fingerprint`, if any.
    /// A key with no origin names no master and so matches nothing.
    pub fn key(&self, fingerprint: Fingerprint) -> Option<&PolicyKey> {
        // A threshold wallet's one key descends from no master, so what
        // names it is its own BIP-32 fingerprint, which is the wallet's.
        if matches!(self.template, Template::Threshold { .. }) {
            return self
                .keys
                .iter()
                .find(|k| k.identifier_fingerprint() == fingerprint);
        }
        self.keys
            .iter()
            .find(|k| k.fingerprint() == Some(fingerprint))
    }

    /// `(change, index)` when `path` is `key/change/index` for one of
    /// the policy's keys, with both leaf steps unhardened and the chain
    /// 0 or 1.
    ///
    /// A MuSig2 wallet has no such path: the chain and the index are
    /// steps of the aggregate key, and a participant's origin names the
    /// participant's own account key and stops there. So there is no
    /// leaf of a participant to read an address off, and this is `None`
    /// for every MuSig2 policy. What names the address in a transaction
    /// is BIP-373's participant-public-keys field, which the PSBT reader
    /// does not have yet (`docs/PLANNING.md` §16.70).
    ///
    /// A threshold wallet does have one: the chain and the index are
    /// steps of the group key's own synthetic extended public key, which
    /// is the only key the descriptor names.
    pub fn leaf_of(&self, fingerprint: Fingerprint, path: &DerivationPath) -> Option<(bool, u32)> {
        if self.template == Template::MuSig {
            return None;
        }
        // A threshold wallet's key has no origin, so the whole path is
        // the two steps below the synthetic xpub: that is what a
        // coordinator holding `tr(XPUB/<0;1>/*)` writes for its keys.
        if matches!(self.template, Template::Threshold { .. }) {
            self.key(fingerprint)?;
            let steps: Vec<ChildNumber> = path.into_iter().copied().collect();
            return match steps[..] {
                [
                    ChildNumber::Normal { index: c @ (0 | 1) },
                    ChildNumber::Normal { index },
                ] => Some((c == 1, index)),
                _ => None,
            };
        }
        let key = self.key(fingerprint)?;
        let steps: Vec<ChildNumber> = path.into_iter().copied().collect();
        let prefix: Vec<ChildNumber> = key.path()?.into_iter().copied().collect();
        if steps.len() != prefix.len() + 2 || steps[..prefix.len()] != prefix[..] {
            return None;
        }
        match (steps[prefix.len()], steps[prefix.len() + 1]) {
            (ChildNumber::Normal { index: c @ (0 | 1) }, ChildNumber::Normal { index }) => {
                Some((c == 1, index))
            }
            _ => None,
        }
    }

    /// The parsed descriptor, for the miniscript and taproot-tree
    /// templates and for no other, which is what
    /// [`crate::recovery`] reads a recovery wallet out of.
    pub(crate) fn script(&self) -> Option<&Descriptor<DescriptorPublicKey>> {
        self.script.as_ref()
    }

    /// The ways this wallet's coins can be spent: one entry per
    /// combination of keys and waits the script allows, each key named
    /// by its index in [`keys`](Self::keys).
    ///
    /// A taproot tree lists the key path first and then one entry per
    /// leaf, because that is the order `tr()` writes them in — unless
    /// the internal key is the unspendable one a wallet whose every path
    /// is a leaf carries, which is no way to spend and is not listed.
    ///
    /// Empty for every template whose spend is one fixed set of
    /// signatures — the single-key wallets, `multi`, `sortedmulti` and
    /// MuSig2 — where the quorum and the key list already say it, and
    /// for a script with more ways to spend it than a person would read.
    pub fn spend_paths(&self) -> Vec<SpendPath> {
        let Some(script) = &self.script else {
            return Vec::new();
        };
        let Some(semantic) = crate::recovery::lift(script) else {
            return Vec::new();
        };
        let keys = descriptor_keys(script);
        crate::spend::spend_paths(&semantic, &keys).unwrap_or_default()
    }
}

impl fmt::Display for WalletPolicy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_text())
    }
}

/// What one of the four single-key templates pays to at a derived key.
fn single_script<C: Verification>(
    secp: &Secp256k1<C>,
    script: ScriptType,
    key: PublicKey,
) -> Result<ScriptBuf, Error> {
    Ok(match script {
        ScriptType::Legacy => ScriptBuf::new_p2pkh(&key.pubkey_hash()),
        ScriptType::NestedSegwit => {
            ScriptBuf::new_p2wpkh(&key.wpubkey_hash().map_err(|_| Error::Key)?).to_p2sh()
        }
        ScriptType::NativeSegwit => {
            ScriptBuf::new_p2wpkh(&key.wpubkey_hash().map_err(|_| Error::Key)?)
        }
        ScriptType::Taproot => ScriptBuf::new_p2tr(secp, key.inner.x_only_public_key().0, None),
    })
}

fn derive<C: Verification>(
    secp: &Secp256k1<C>,
    key: &PolicyKey,
    change: bool,
    index: u32,
) -> Result<PublicKey, Error> {
    let chain = ChildNumber::from_normal_idx(u32::from(change)).expect("0 or 1");
    let index = ChildNumber::from_normal_idx(index).map_err(|_| Error::IndexOutOfRange)?;
    let derived = key
        .xpub
        .derive_pub(secp, &[chain, index])
        .expect("normal child derivation yields a valid key");
    Ok(PublicKey::new(derived.public_key))
}

/// What a failure inside the `musig()` machinery is in this module's
/// vocabulary.
fn musig_error(error: crate::musig::Error) -> Error {
    match error {
        crate::musig::Error::IndexOutOfRange => Error::IndexOutOfRange,
        crate::musig::Error::Derivation => Error::Derivation,
        crate::musig::Error::Key | crate::musig::Error::NoKeys => Error::Key,
        _ => Error::Template,
    }
}

fn multisig_script(threshold: usize, keys: &[PublicKey]) -> ScriptBuf {
    let mut b = Builder::new().push_int(threshold as i64);
    for key in keys {
        b = b.push_key(key);
    }
    b.push_int(keys.len() as i64)
        .push_opcode(OP_CHECKMULTISIG)
        .into_script()
}

/// Writes `args` (the key expressions or placeholders, comma-separated)
/// inside the template's functions. A MuSig2 template is written by
/// [`WalletPolicy::render`], which puts the derivation outside the
/// `musig()`.
fn wrap(template: Template, args: &str) -> String {
    match template {
        Template::Miniscript { .. } | Template::Tree => {
            unreachable!("a miniscript wallet is written by `miniscript`")
        }
        Template::Silent => unreachable!("a silent payments wallet has no key expression"),
        Template::MuSig => alloc::format!("tr(musig({args}))"),
        // The group key stands where a single key stands, because that
        // is what the chain sees.
        Template::Threshold { .. } => alloc::format!("tr({args})"),
        Template::Multi {
            threshold,
            sorted,
            wrapper,
        } => {
            let name = if sorted { "sortedmulti" } else { "multi" };
            let inner = alloc::format!("{name}({threshold},{args})");
            match wrapper {
                Wrapper::Sh => alloc::format!("sh({inner})"),
                Wrapper::Wsh => alloc::format!("wsh({inner})"),
                Wrapper::ShWsh => alloc::format!("sh(wsh({inner}))"),
            }
        }
        Template::Single { script } => match script {
            ScriptType::Legacy => alloc::format!("pkh({args})"),
            ScriptType::NestedSegwit => alloc::format!("sh(wpkh({args}))"),
            ScriptType::NativeSegwit => alloc::format!("wpkh({args})"),
            ScriptType::Taproot => alloc::format!("tr({args})"),
        },
    }
}

/// The keys a descriptor names, in the order its text writes them.
///
/// `for_each_key` visits a `tr()`'s leaves before its internal key, so
/// the order it gives is not the order a person reads; the position of
/// each key expression in the descriptor is.
fn descriptor_keys(script: &Descriptor<DescriptorPublicKey>) -> Vec<DescriptorPublicKey> {
    let text = alloc::format!("{script}");
    let mut keys: Vec<DescriptorPublicKey> = Vec::new();
    script.for_each_key(|key| {
        if !keys.contains(key) {
            keys.push(key.clone());
        }
        true
    });
    keys.sort_by_key(|key| text.find(&alloc::format!("{key}")).unwrap_or(usize::MAX));
    keys
}

/// Reads `wsh(<miniscript>)`, `sh(wsh(<miniscript>))` or
/// `tr(<key>,<tree>)` as a wallet, with `miniscript` doing the parsing
/// and the type check.
fn parse_miniscript(text: &str) -> Result<WalletPolicy, Error> {
    // `sortedmulti_a` is BIP 387's sorted tapscript multisig, which
    // `miniscript` does not read. Its keys and its tree are those of the
    // `multi_a` over the same keys; what differs is the order they are
    // put in the script, which [`WalletPolicy::script_at_with`] applies
    // at every index.
    let sorted_a = crate::tapmulti::unsorted_text(text);
    let (text, sorted) = match &sorted_a {
        Some(rewritten) => (rewritten.as_str(), true),
        None => (text, false),
    };
    let script = Descriptor::<DescriptorPublicKey>::from_str(text).map_err(|_| Error::Template)?;
    let template = match &script {
        Descriptor::Wsh(_) => Template::Miniscript {
            kind: MiniscriptKind::Wsh,
        },
        Descriptor::Sh(sh) => match sh.as_inner() {
            miniscript::descriptor::ShInner::Wsh(_) => Template::Miniscript {
                kind: MiniscriptKind::ShWsh,
            },
            _ => return Err(Error::Template),
        },
        // A `tr()` with no tree is the single-key template, which
        // `parse_structure` already reads.
        Descriptor::Tr(tr) if tr.tap_tree().is_some() => Template::Tree,
        _ => return Err(Error::Template),
    };
    // A `tr()`'s internal key may be a plain public key — the NUMS point
    // a script-only wallet writes there. It is not one of the wallet's
    // keys: nobody holds it and nothing derives from it, so it stays in
    // the template's text where a person can read it, and the keys are
    // the extended keys of the tree.
    let internal = internal_key_text(&script);
    let keys: Vec<PolicyKey> = descriptor_keys(&script)
        .iter()
        .filter(|key| {
            !matches!(key, DescriptorPublicKey::Single(_))
                || internal.as_deref() != Some(alloc::format!("{key}").as_str())
        })
        .map(wallet_key)
        .collect::<Result<_, _>>()?;
    let sorted_a = match sorted {
        true => Some(multi_a_threshold(text).ok_or(Error::Threshold)?),
        false => None,
    };
    if let Some(threshold) = sorted_a
        && (threshold == 0 || threshold > keys.len())
    {
        return Err(Error::Threshold);
    }
    check(&template, &keys)?;
    Ok(WalletPolicy {
        template,
        keys,
        script: Some(script),
        sorted_a,
        record: None,
        silent: None,
    })
}

/// The internal key of a `tr()`, as the descriptor writes it.
fn internal_key_text(script: &Descriptor<DescriptorPublicKey>) -> Option<String> {
    match script {
        Descriptor::Tr(tr) => Some(alloc::format!("{}", tr.internal_key())),
        _ => None,
    }
}

/// The threshold of the one `multi_a` in a descriptor's text.
fn multi_a_threshold(text: &str) -> Option<usize> {
    let after = text.split_once("multi_a(")?.1;
    after.split_once(',')?.0.parse().ok()
}

/// One descriptor key as a wallet's key.
///
/// A wallet needs both chains: its addresses are the receive chain and
/// its change is verified on the change chain, so a key written with one
/// path only is refused rather than half-read.
fn wallet_key(key: &DescriptorPublicKey) -> Result<PolicyKey, Error> {
    let multi: &DescriptorMultiXKey<Xpub> = match key {
        DescriptorPublicKey::MultiXPub(multi) => multi,
        DescriptorPublicKey::XPub(_) => return Err(Error::Derivation),
        DescriptorPublicKey::Single(_) => return Err(Error::Key),
    };
    let paths = multi.derivation_paths.paths();
    let chains: Vec<Vec<ChildNumber>> = paths
        .iter()
        .map(|p| p.into_iter().copied().collect())
        .collect();
    let expected = [
        alloc::vec![ChildNumber::Normal { index: 0 }],
        alloc::vec![ChildNumber::Normal { index: 1 }],
    ];
    if multi.wildcard != Wildcard::Unhardened || chains != expected {
        return Err(Error::Derivation);
    }
    let origin = multi.origin.as_ref().map(|(fingerprint, path)| {
        let mut text = String::new();
        for byte in fingerprint.as_bytes() {
            text.push_str(&alloc::format!("{byte:02x}"));
        }
        for step in path {
            text.push_str(&alloc::format!("/{step}"));
        }
        Origin {
            text,
            fingerprint: Fingerprint(fingerprint.to_bytes()),
            path: path.clone(),
        }
    });
    Ok(PolicyKey {
        origin,
        xpub: multi.xkey,
    })
}

/// A BIP-388 template with its keys written in, so that `miniscript`
/// reads the descriptor the two halves stand for.
///
/// The placeholders must be `@0`, `@1`, … first used in order, and each
/// must carry the multipath suffix a wallet policy writes. A miniscript
/// may name one key more than once — a recovery key that is also in a
/// threshold is the usual reason — so a placeholder already seen may
/// appear again.
fn fill(template: &str, keys: &[&str]) -> Result<String, Error> {
    let template = template.trim();
    let mut out = String::with_capacity(template.len());
    let mut rest = template;
    let mut next = 0usize;
    while let Some(at) = rest.find('@') {
        out.push_str(&rest[..at]);
        let tail = &rest[at + 1..];
        let digits = tail
            .find(|c: char| !c.is_ascii_digit())
            .unwrap_or(tail.len());
        let index: usize = tail[..digits]
            .parse()
            .ok()
            .filter(|_| digits == 1 || !tail.starts_with('0'))
            .ok_or(Error::Placeholder)?;
        if index > next {
            return Err(Error::Placeholder);
        }
        let key = keys.get(index).ok_or(Error::Placeholder)?;
        if index == next {
            next += 1;
        }
        let after = &tail[digits..];
        let after = after
            .strip_prefix(TEMPLATE_SUFFIX)
            .or_else(|| after.strip_prefix(DESCRIPTOR_SUFFIX))
            .ok_or(Error::Derivation)?;
        out.push_str(key);
        out.push_str(DESCRIPTOR_SUFFIX);
        rest = after;
    }
    if next != keys.len() {
        return Err(Error::Placeholder);
    }
    out.push_str(rest);
    Ok(out)
}

/// The rules BIP-388 states about the whole policy: at least one key,
/// keys pairwise distinct, and a threshold that a script can hold.
fn check(template: &Template, keys: &[PolicyKey]) -> Result<(), Error> {
    if keys.is_empty() {
        return Err(Error::Placeholder);
    }
    for (i, key) in keys.iter().enumerate() {
        if keys[..i].iter().any(|k| k.xpub == key.xpub) {
            return Err(Error::Key);
        }
        // BIP-388 writes every key of a policy with its origin. Only a
        // single-key wallet is allowed without one, because that is the
        // wallet a bare extended public key makes and its addresses need
        // nothing else.
        // A threshold wallet's one key is the group's synthetic
        // extended public key, which descends from no master, so it has
        // no origin to write either.
        // So is a taproot tree's internal key where no key of the wallet
        // can take the key path: the descriptor then writes a key
        // derived from the leaves so that anyone can see the key path is
        // unspendable, and it descends from no master. `tr()` writes the
        // internal key first, so it is the key at index zero and no
        // other.
        let originless = matches!(
            template,
            Template::Single { .. } | Template::Threshold { .. }
        ) || (matches!(template, Template::Tree) && i == 0);
        if key.origin.is_none() && !originless {
            return Err(Error::Key);
        }
    }
    match template {
        Template::Multi { threshold, .. } => {
            if *threshold == 0 || *threshold > keys.len() || keys.len() > 15 {
                return Err(Error::Threshold);
            }
        }
        // A MuSig2 wallet is two or more participants aggregating into
        // one key; one key aggregates with nothing.
        Template::MuSig => {
            if keys.len() < 2 {
                return Err(Error::Placeholder);
            }
        }
        Template::Single { .. } => {
            if keys.len() != 1 {
                return Err(Error::Placeholder);
            }
        }
        // The record states the threshold and the participants, and
        // [`ThresholdRecord::parse`] has checked them against the shares.
        Template::Threshold { .. } => {
            if keys.len() != 1 {
                return Err(Error::Placeholder);
            }
        }
        // A miniscript states its own rules; what this function has left
        // to say about one is that its keys are distinct and carry the
        // origin BIP-388 writes, which the loop above checked.
        // A silent payments wallet has no key expression at all, so it
        // never reaches this function: it is built from its record.
        Template::Silent => return Err(Error::Template),
        Template::Miniscript { .. } | Template::Tree => {}
    }
    Ok(())
}

/// Parses a template, checking that its placeholders are `@0`, `@1`, …
/// each used once and in order. Returns how many there are.
fn parse_template(text: &str) -> Result<(Template, usize), Error> {
    let mut next = 0usize;
    let template = parse_structure(text.trim(), &mut |key| {
        let index: usize = key
            .strip_prefix('@')
            .filter(|n| *n == "0" || !n.starts_with('0'))
            .and_then(|n| n.parse().ok())
            .ok_or(Error::Placeholder)?;
        if index != next {
            return Err(Error::Placeholder);
        }
        next += 1;
        Ok(())
    })?;
    Ok((template, next))
}

/// A key expression without its multipath suffix, which BIP-388 allows
/// only as `/**` or its spelled-out form.
fn strip_suffix(expr: &str) -> Result<&str, Error> {
    expr.strip_suffix(TEMPLATE_SUFFIX)
        .or_else(|| expr.strip_suffix(DESCRIPTOR_SUFFIX))
        .ok_or(Error::Derivation)
}

/// Parses `[fingerprint/path]xpub`, or a bare `xpub` with no origin.
/// Which templates accept an origin-less key is [`check`]'s business.
fn parse_key(text: &str) -> Result<PolicyKey, Error> {
    let Some(rest) = text.strip_prefix('[') else {
        return Ok(PolicyKey {
            origin: None,
            xpub: crate::xkey::decode_xpub(text).map_err(|_| Error::Key)?,
        });
    };
    let (origin, xpub) = rest.split_once(']').ok_or(Error::Key)?;
    let fingerprint = origin.get(..8).ok_or(Error::Key)?;
    let path = &origin[8..];
    if fingerprint.len() != 8 || !fingerprint.bytes().all(|c| c.is_ascii_hexdigit()) {
        return Err(Error::Key);
    }
    let mut bytes = [0u8; 4];
    for (i, b) in bytes.iter_mut().enumerate() {
        *b = u8::from_str_radix(&fingerprint[2 * i..2 * i + 2], 16).map_err(|_| Error::Key)?;
    }
    let path = if path.is_empty() {
        DerivationPath::master()
    } else {
        DerivationPath::from_str(path.strip_prefix('/').ok_or(Error::Key)?)
            .map_err(|_| Error::Key)?
    };
    Ok(PolicyKey {
        origin: Some(Origin {
            text: String::from(origin),
            fingerprint: Fingerprint(bytes),
            path,
        }),
        xpub: crate::xkey::decode_xpub(xpub).map_err(|_| Error::Key)?,
    })
}

/// Walks the supported template shapes, handing every key expression —
/// without the multipath suffix a wallet policy writes on it — to `key`
/// in the order it appears.
fn parse_structure(
    text: &str,
    key: &mut dyn FnMut(&str) -> Result<(), Error>,
) -> Result<Template, Error> {
    if let Some(inner) = inside(text, "sh(") {
        if let Some(inner) = inside(inner, "wsh(") {
            let (threshold, sorted) = multi_args(inner, key)?;
            return Ok(Template::Multi {
                threshold,
                sorted,
                wrapper: Wrapper::ShWsh,
            });
        }
        if let Some(arg) = inside(inner, "wpkh(") {
            key(strip_suffix(arg)?)?;
            return Ok(Template::Single {
                script: ScriptType::NestedSegwit,
            });
        }
        let (threshold, sorted) = multi_args(inner, key)?;
        return Ok(Template::Multi {
            threshold,
            sorted,
            wrapper: Wrapper::Sh,
        });
    }
    if let Some(inner) = inside(text, "wsh(") {
        let (threshold, sorted) = multi_args(inner, key)?;
        return Ok(Template::Multi {
            threshold,
            sorted,
            wrapper: Wrapper::Wsh,
        });
    }
    if let Some(inner) = inside(text, "tr(")
        && inner.starts_with("musig(")
    {
        musig_args(inner, key)?;
        return Ok(Template::MuSig);
    }
    for (prefix, script) in [
        ("pkh(", ScriptType::Legacy),
        ("wpkh(", ScriptType::NativeSegwit),
        ("tr(", ScriptType::Taproot),
    ] {
        if let Some(arg) = inside(text, prefix) {
            if arg.contains(['(', ',', '{']) {
                return Err(Error::Template);
            }
            key(strip_suffix(arg)?)?;
            return Ok(Template::Single { script });
        }
    }
    Err(Error::Template)
}

/// `musig(KEY,…)/**`: every participant handed to `key`.
///
/// BIP-390 lets a participant carry a derivation of its own, and lets
/// the `musig()` carry one, but not both. A wallet policy is the second:
/// the addresses are a run off the aggregate key, which is what BIP-388
/// writes `/**` for. A `musig()` whose participants are ranged names one
/// aggregate key per participant index and is refused here.
fn musig_args(text: &str, key: &mut dyn FnMut(&str) -> Result<(), Error>) -> Result<(), Error> {
    let rest = text.strip_prefix("musig(").ok_or(Error::Template)?;
    let close = rest.find(')').ok_or(Error::Template)?;
    let (args, tail) = rest.split_at(close);
    if args.contains(['(', '{']) {
        return Err(Error::Template);
    }
    let tail = &tail[1..];
    if tail != TEMPLATE_SUFFIX && tail != DESCRIPTOR_SUFFIX {
        return Err(Error::Derivation);
    }
    let mut n = 0;
    for part in args.split(',') {
        let part = part.trim();
        // The origin is the only place a `/` belongs in a participant of
        // a wallet policy's `musig()`.
        let after_origin = part.rsplit(']').next().unwrap_or(part);
        if after_origin.contains('/') {
            return Err(Error::Derivation);
        }
        key(part)?;
        n += 1;
    }
    if n < 2 {
        return Err(Error::Placeholder);
    }
    Ok(())
}

/// `multi(k,…)` or `sortedmulti(k,…)`: the threshold, whether it sorts,
/// and every key expression handed to `key`.
fn multi_args(
    text: &str,
    key: &mut dyn FnMut(&str) -> Result<(), Error>,
) -> Result<(usize, bool), Error> {
    let (args, sorted) = match inside(text, "sortedmulti(") {
        Some(args) => (args, true),
        None => (inside(text, "multi(").ok_or(Error::Template)?, false),
    };
    let mut parts = args.split(',');
    let threshold: usize = parts
        .next()
        .ok_or(Error::Template)?
        .parse()
        .map_err(|_| Error::Threshold)?;
    let mut n = 0;
    for part in parts {
        if part.contains(['(', ')', '{']) {
            return Err(Error::Template);
        }
        key(strip_suffix(part)?)?;
        n += 1;
    }
    if n == 0 {
        return Err(Error::Placeholder);
    }
    Ok((threshold, sorted))
}

/// The text between `prefix` and the matching final `)`, when `text` is
/// exactly that function call.
fn inside<'a>(text: &'a str, prefix: &str) -> Option<&'a str> {
    text.strip_prefix(prefix)?.strip_suffix(')')
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The BIP-388 "Examples" section: every template with its keys
    /// produces the descriptor the BIP lists.
    #[test]
    fn bip388_examples_produce_their_descriptors() {
        const CASES: &[(&str, &[&str], &str)] = &[
            (
                "pkh(@0/**)",
                &[
                    "[6738736c/44'/0'/0']xpub6Br37sWxruYfT8ASpCjVHKGwgdnYFEn98DwiN76i2oyY6fgH1LAPmmDcF46xjxJr22gw4jmVjTE2E3URMnRPEPYyo1zoPSUba563ESMXCeb",
                ],
                "pkh([6738736c/44'/0'/0']xpub6Br37sWxruYfT8ASpCjVHKGwgdnYFEn98DwiN76i2oyY6fgH1LAPmmDcF46xjxJr22gw4jmVjTE2E3URMnRPEPYyo1zoPSUba563ESMXCeb/<0;1>/*)",
            ),
            (
                "sh(wpkh(@0/**))",
                &[
                    "[6738736c/49'/0'/1']xpub6Bex1CHWGXNNwGVKHLqNC7kcV348FxkCxpZXyCWp1k27kin8sRPayjZUKDjyQeZzGUdyeAj2emoW5zStFFUAHRgd5w8iVVbLgZ7PmjAKAm9",
                ],
                "sh(wpkh([6738736c/49'/0'/1']xpub6Bex1CHWGXNNwGVKHLqNC7kcV348FxkCxpZXyCWp1k27kin8sRPayjZUKDjyQeZzGUdyeAj2emoW5zStFFUAHRgd5w8iVVbLgZ7PmjAKAm9/<0;1>/*))",
            ),
            (
                "wpkh(@0/**)",
                &[
                    "[6738736c/84'/0'/2']xpub6CRQzb8u9dmMcq5XAwwRn9gcoYCjndJkhKgD11WKzbVGd932UmrExWFxCAvRnDN3ez6ZujLmMvmLBaSWdfWVn75L83Qxu1qSX4fJNrJg2Gt",
                ],
                "wpkh([6738736c/84'/0'/2']xpub6CRQzb8u9dmMcq5XAwwRn9gcoYCjndJkhKgD11WKzbVGd932UmrExWFxCAvRnDN3ez6ZujLmMvmLBaSWdfWVn75L83Qxu1qSX4fJNrJg2Gt/<0;1>/*)",
            ),
            (
                "tr(@0/**)",
                &[
                    "[6738736c/86'/0'/0']xpub6CryUDWPS28eR2cDyojB8G354izmx294BdjeSvH469Ty3o2E6Tq5VjBJCn8rWBgesvTJnyXNAJ3QpLFGuNwqFXNt3gn612raffLWfdHNkYL",
                ],
                "tr([6738736c/86'/0'/0']xpub6CryUDWPS28eR2cDyojB8G354izmx294BdjeSvH469Ty3o2E6Tq5VjBJCn8rWBgesvTJnyXNAJ3QpLFGuNwqFXNt3gn612raffLWfdHNkYL/<0;1>/*)",
            ),
            (
                "wsh(sortedmulti(2,@0/**,@1/**))",
                &[
                    "[6738736c/48'/0'/0'/2']xpub6FC1fXFP1GXLX5TKtcjHGT4q89SDRehkQLtbKJ2PzWcvbBHtyDsJPLtpLtkGqYNYZdVVAjRQ5kug9CsapegmmeRutpP7PW4u4wVF9JfkDhw",
                    "[b2b1f0cf/48'/0'/0'/2']xpub6EWhjpPa6FqrcaPBuGBZRJVjzGJ1ZsMygRF26RwN932Vfkn1gyCiTbECVitBjRCkexEvetLdiqzTcYimmzYxyR1BZ79KNevgt61PDcukmC7",
                ],
                "wsh(sortedmulti(2,[6738736c/48'/0'/0'/2']xpub6FC1fXFP1GXLX5TKtcjHGT4q89SDRehkQLtbKJ2PzWcvbBHtyDsJPLtpLtkGqYNYZdVVAjRQ5kug9CsapegmmeRutpP7PW4u4wVF9JfkDhw/<0;1>/*,[b2b1f0cf/48'/0'/0'/2']xpub6EWhjpPa6FqrcaPBuGBZRJVjzGJ1ZsMygRF26RwN932Vfkn1gyCiTbECVitBjRCkexEvetLdiqzTcYimmzYxyR1BZ79KNevgt61PDcukmC7/<0;1>/*))",
            ),
        ];
        for (template, keys, descriptor) in CASES {
            let policy = WalletPolicy::from_parts(template, keys).expect(template);
            assert_eq!(&policy.to_descriptor(), descriptor);
            assert_eq!(&policy.template_text(), template);
            // The descriptor read back is the same policy, so both forms
            // of the same wallet show the same review.
            let from_descriptor = WalletPolicy::from_descriptor(descriptor).expect(descriptor);
            assert_eq!(from_descriptor.keys(), policy.keys());
            assert_eq!(from_descriptor.to_descriptor(), *descriptor);
        }
    }

    /// The BIP-388 "Invalid policies" section, restricted to the forms
    /// this crate accepts at all.
    #[test]
    fn bip388_invalid_policies_are_refused() {
        const KEY: &str = "[d34db33f/44'/0'/0']xpub6ERApfZwUNrhLCkDtcHTcxd75RbzS1ed54G1LkBUHQVHQKqhMkhgbmJbZRkrgZw4koxb5JaHWkY4ALHY2grBGRjaDMzQLcgJvLJuZZvRcEL";
        const OTHER: &str = "[b2b1f0cf/48'/0'/0'/2']xpub6EWhjpPa6FqrcaPBuGBZRJVjzGJ1ZsMygRF26RwN932Vfkn1gyCiTbECVitBjRCkexEvetLdiqzTcYimmzYxyR1BZ79KNevgt61PDcukmC7";
        for (template, keys) in [
            ("pkh(@0)", &[KEY][..]),
            ("pkh(@0/0/**)", &[KEY]),
            ("pkh(@0/<0;1;2>/*)", &[KEY]),
            ("sh(multi(1,@1/**,@0/**))", &[KEY, OTHER]),
            ("sh(multi(1,@0/**,@2/**))", &[KEY, OTHER]),
            ("sh(multi(1,@0/**,@0/**))", &[KEY, OTHER]),
            ("sh(multi(1,@0/<0;1>/*,@0/<1;2>/*))", &[KEY, OTHER]),
            (
                "sh(multi(1,@0/**,xpub6AHA9hZDN11k2ijHMeS5QqHx2KP9aMBRhTDqANMnwVtdyw2TDYRmF8PjpvwUFcL1Et8Hj59S3gTSMcUQ5gAqTz3Wd8EsMTmF3DChhqPQBnU/<0;1>/*))",
                &[KEY, OTHER],
            ),
        ] {
            assert!(
                WalletPolicy::from_parts(template, keys).is_err(),
                "{template} was accepted"
            );
        }
    }

    /// A group record is neither a BIP-388 policy nor a descriptor, and
    /// the two readers that expect one of those refuse it. What reads it
    /// is [`WalletPolicy::parse_any`], which
    /// `core/osk-bip/tests/threshold.rs` carries with the committed
    /// fixture.
    #[test]
    fn a_threshold_record_is_not_a_policy_or_a_descriptor() {
        const RECORD: &str = "osk-threshold 1\nthreshold 2\ngroup 02\ntr(tpub)#aaaaaaaa";
        assert!(WalletPolicy::parse(RECORD).is_err());
        assert!(WalletPolicy::from_descriptor(RECORD).is_err());
        assert!(
            WalletPolicy::parse_any(RECORD).is_err(),
            "and it is no group"
        );
    }

    /// A miniscript template whose key carries one chain is refused: a
    /// wallet with no change chain cannot verify its own change.
    #[test]
    fn a_miniscript_key_with_one_chain_is_refused() {
        const KEY: &str = "[d34db33f/48'/0'/0'/2']xpub6ERApfZwUNrhLCkDtcHTcxd75RbzS1ed54G1LkBUHQVHQKqhMkhgbmJbZRkrgZw4koxb5JaHWkY4ALHY2grBGRjaDMzQLcgJvLJuZZvRcEL";
        let one_chain = alloc::format!("wsh(and_v(v:pk({KEY}/0/*),older(12960)))");
        assert_eq!(
            WalletPolicy::from_descriptor(&one_chain),
            Err(Error::Derivation)
        );
    }

    /// The two BIP-390 vector participants, with origins a wallet
    /// policy requires.
    const MUSIG_A: &str = "[6738736c/86'/0'/0']xpub6ERApfZwUNrhLCkDtcHTcxd75RbzS1ed54G1LkBUHQVHQKqhMkhgbmJbZRkrgZw4koxb5JaHWkY4ALHY2grBGRjaDMzQLcgJvLJuZZvRcEL";
    const MUSIG_B: &str = "[b2b1f0cf/86'/0'/0']xpub68NZiKmJWnxxS6aaHmn81bvJeTESw724CRDs6HbuccFQN9Ku14VQrADWgqbhhTHBaohPX4CjNLf9fq9MYo6oDaPPLPxSb7gwQN3ih19Zm4Y";

    /// A BIP-390 MuSig2 wallet reads the same from its policy and from
    /// its descriptor, and each is written back the way it arrived: the
    /// derivation follows the `musig()`, because the addresses come from
    /// the aggregate key.
    #[test]
    fn a_musig_wallet_is_one_policy_in_both_forms() {
        let policy = WalletPolicy::from_parts("tr(musig(@0,@1)/**)", &[MUSIG_A, MUSIG_B]).unwrap();
        assert_eq!(policy.template_text(), "tr(musig(@0,@1)/**)");
        assert_eq!(
            policy.to_descriptor(),
            alloc::format!("tr(musig({MUSIG_A},{MUSIG_B})/<0;1>/*)")
        );
        assert_eq!(policy.script_type(), ScriptType::Taproot);
        assert_eq!(policy.quorum(), None);
        assert_eq!(policy.keys().len(), 2);

        let text = policy.to_text();
        assert_eq!(WalletPolicy::parse_any(&text).unwrap(), policy);
        let descriptor = policy.to_descriptor_checksummed();
        assert_eq!(WalletPolicy::parse_any(&descriptor).unwrap(), policy);
        // The spelled-out multipath is the same wallet as `/**`.
        assert_eq!(
            WalletPolicy::from_parts("tr(musig(@0,@1)/<0;1>/*)", &[MUSIG_A, MUSIG_B]).unwrap(),
            policy
        );
    }

    /// The `musig()` forms that are not a wallet: no derivation to run
    /// addresses over, a derivation on the participants instead of the
    /// aggregate, one participant, and a participant with no origin.
    #[test]
    fn musig_expressions_that_are_not_a_wallet_are_refused() {
        for (template, keys) in [
            ("tr(musig(@0,@1))", &[MUSIG_A, MUSIG_B][..]),
            ("tr(musig(@0/**,@1/**))", &[MUSIG_A, MUSIG_B]),
            ("tr(musig(@0,@1)/0/**)", &[MUSIG_A, MUSIG_B]),
            ("tr(musig(@0,@1)/**/*)", &[MUSIG_A, MUSIG_B]),
            ("tr(musig(@0)/**)", &[MUSIG_A]),
        ] {
            assert!(
                WalletPolicy::from_parts(template, keys).is_err(),
                "{template} was accepted"
            );
        }
        let no_origin = alloc::format!(
            "tr(musig({},{})/<0;1>/*)",
            MUSIG_A.split_once(']').unwrap().1,
            MUSIG_B.split_once(']').unwrap().1
        );
        assert_eq!(
            WalletPolicy::from_descriptor(&no_origin),
            Err(Error::Key),
            "a participant with no origin is no wallet key"
        );
    }

    /// BIP-84's test vector for `abandon … about`: the account key as
    /// SLIP-132 writes it, and the first receive and change addresses it
    /// pays to. A wallet over the key alone derives the same run as one
    /// with an origin, because the origin is not what addresses come
    /// from.
    #[test]
    fn a_wallet_over_a_bare_key_derives_the_published_addresses() {
        const ZPUB: &str = "zpub6rFR7y4Q2AijBEqTUquhVz398htDFrtymD9xYYfG1m4wAcvPhXNfE3EfH1r1ADqtfSdVCToUG868RvUUkgDKf31mGDtKsAYz2oz2AGutZYs";
        let (xpub, script) = crate::slip132::decode_xpub(ZPUB).unwrap();
        assert_eq!(script, ScriptType::NativeSegwit);
        let policy = WalletPolicy::from_xpub(xpub, script).unwrap();
        assert_eq!(
            alloc::format!("{}", policy.address_at(Network::Mainnet, false, 0).unwrap()),
            "bc1qcr8te4kr609gcawutmrza0j4xv80jy8z306fyu"
        );
        assert_eq!(
            alloc::format!("{}", policy.address_at(Network::Mainnet, true, 0).unwrap()),
            "bc1q8c6fshw2dlwun7ekn9qwf37cu2rn755upcp6el"
        );

        // The key writes with no origin, which is a descriptor of its
        // own, and the same wallet reads back out of it. BIP-388 has no
        // form for it, so the two-part text is that descriptor.
        let descriptor = policy.to_descriptor();
        assert_eq!(
            descriptor,
            alloc::format!("wpkh({xpub}{DESCRIPTOR_SUFFIX})")
        );
        assert_eq!(policy.to_text(), descriptor);
        assert_eq!(WalletPolicy::parse_any(&descriptor).unwrap(), policy);
        assert_eq!(
            WalletPolicy::parse_any(&policy.to_descriptor_checksummed()).unwrap(),
            policy
        );

        // The master it came from is not among the facts; what it has is
        // its own identifier, which is not a master fingerprint and is
        // never offered as one.
        let key = &policy.keys()[0];
        assert_eq!(key.fingerprint(), None);
        assert_eq!(key.path(), None);
        assert_eq!(
            key.identifier_fingerprint().to_hex(),
            *b"fd13aac9",
            "the xpub's own BIP-32 fingerprint"
        );
        assert_eq!(policy.key(key.identifier_fingerprint()), None);
        assert_eq!(
            policy.leaf_of(
                key.identifier_fingerprint(),
                &DerivationPath::from_str("0/0").unwrap()
            ),
            None,
            "a wallet with no origin matches no output"
        );
    }

    /// The other three single-key templates read a bare key too, and
    /// every template that needs more than one key still requires the
    /// origin BIP-388 writes.
    #[test]
    fn only_a_single_key_template_reads_a_key_with_no_origin() {
        const A: &str = "xpub6FC1fXFP1GXLX5TKtcjHGT4q89SDRehkQLtbKJ2PzWcvbBHtyDsJPLtpLtkGqYNYZdVVAjRQ5kug9CsapegmmeRutpP7PW4u4wVF9JfkDhw";
        const B: &str = "xpub6EWhjpPa6FqrcaPBuGBZRJVjzGJ1ZsMygRF26RwN932Vfkn1gyCiTbECVitBjRCkexEvetLdiqzTcYimmzYxyR1BZ79KNevgt61PDcukmC7";
        for (template, script) in [
            ("pkh", ScriptType::Legacy),
            ("wpkh", ScriptType::NativeSegwit),
            ("tr", ScriptType::Taproot),
        ] {
            let descriptor = alloc::format!("{template}({A}{DESCRIPTOR_SUFFIX})");
            let policy = WalletPolicy::from_descriptor(&descriptor).expect(&descriptor);
            assert_eq!(policy.script_type(), script);
            assert_eq!(policy.to_descriptor(), descriptor);
        }
        let nested = alloc::format!("sh(wpkh({A}{DESCRIPTOR_SUFFIX}))");
        assert_eq!(
            WalletPolicy::from_descriptor(&nested)
                .unwrap()
                .script_type(),
            ScriptType::NestedSegwit
        );
        for descriptor in [
            alloc::format!("wsh(sortedmulti(2,{A}{DESCRIPTOR_SUFFIX},{B}{DESCRIPTOR_SUFFIX}))"),
            alloc::format!("sh(multi(1,{A}{DESCRIPTOR_SUFFIX},{B}{DESCRIPTOR_SUFFIX}))"),
            alloc::format!("tr(musig({A},{B}){DESCRIPTOR_SUFFIX})"),
        ] {
            assert_eq!(
                WalletPolicy::from_descriptor(&descriptor),
                Err(Error::Key),
                "{descriptor} was accepted with no origin on its keys"
            );
        }
    }

    /// A key with two different masters behind the same xpub is the same
    /// key twice; BIP-388 requires the keys to be distinct.
    #[test]
    fn repeated_keys_are_refused() {
        const KEY: &str = "[6738736c/48'/0'/0'/2']xpub6FC1fXFP1GXLX5TKtcjHGT4q89SDRehkQLtbKJ2PzWcvbBHtyDsJPLtpLtkGqYNYZdVVAjRQ5kug9CsapegmmeRutpP7PW4u4wVF9JfkDhw";
        assert_eq!(
            WalletPolicy::from_parts("wsh(sortedmulti(2,@0/**,@1/**))", &[KEY, KEY]),
            Err(Error::Key)
        );
    }

    /// The threshold a wallet states must be one a script can hold.
    #[test]
    fn a_threshold_above_the_keys_is_refused() {
        const A: &str = "[6738736c/48'/0'/0'/2']xpub6FC1fXFP1GXLX5TKtcjHGT4q89SDRehkQLtbKJ2PzWcvbBHtyDsJPLtpLtkGqYNYZdVVAjRQ5kug9CsapegmmeRutpP7PW4u4wVF9JfkDhw";
        const B: &str = "[b2b1f0cf/48'/0'/0'/2']xpub6EWhjpPa6FqrcaPBuGBZRJVjzGJ1ZsMygRF26RwN932Vfkn1gyCiTbECVitBjRCkexEvetLdiqzTcYimmzYxyR1BZ79KNevgt61PDcukmC7";
        assert_eq!(
            WalletPolicy::from_parts("wsh(sortedmulti(3,@0/**,@1/**))", &[A, B]),
            Err(Error::Threshold)
        );
        assert_eq!(
            WalletPolicy::from_parts("wsh(sortedmulti(0,@0/**,@1/**))", &[A, B]),
            Err(Error::Threshold)
        );
    }

    /// A descriptor whose checksum does not hold is not a wallet.
    #[test]
    fn a_wrong_checksum_is_refused() {
        let policy = WalletPolicy::from_descriptor(
            "wpkh([6738736c/84'/0'/2']xpub6CRQzb8u9dmMcq5XAwwRn9gcoYCjndJkhKgD11WKzbVGd932UmrExWFxCAvRnDN3ez6ZujLmMvmLBaSWdfWVn75L83Qxu1qSX4fJNrJg2Gt/<0;1>/*)",
        )
        .unwrap();
        let good = policy.to_descriptor_checksummed();
        assert!(WalletPolicy::from_descriptor(&good).is_ok());
        let mut bad = good.clone();
        bad.pop();
        bad.push('q');
        assert_eq!(WalletPolicy::from_descriptor(&bad), Err(Error::Checksum));
    }

    /// The two-part form and the descriptor form of one wallet are one
    /// policy, and the policy is written back the way it arrived.
    #[test]
    fn both_forms_are_the_same_wallet() {
        let text = "wsh(sortedmulti(2,@0/**,@1/**))\n\
            [6738736c/48'/0'/0'/2']xpub6FC1fXFP1GXLX5TKtcjHGT4q89SDRehkQLtbKJ2PzWcvbBHtyDsJPLtpLtkGqYNYZdVVAjRQ5kug9CsapegmmeRutpP7PW4u4wVF9JfkDhw\n\
            [b2b1f0cf/48'/0'/0'/2']xpub6EWhjpPa6FqrcaPBuGBZRJVjzGJ1ZsMygRF26RwN932Vfkn1gyCiTbECVitBjRCkexEvetLdiqzTcYimmzYxyR1BZ79KNevgt61PDcukmC7";
        let policy = WalletPolicy::parse_any(text).unwrap();
        assert_eq!(policy.to_text(), text);
        assert_eq!(policy.quorum(), Some((2, 2)));
        let descriptor = policy.to_descriptor_checksummed();
        assert_eq!(WalletPolicy::parse_any(&descriptor).unwrap(), policy);
    }
}
