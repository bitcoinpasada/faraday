//! The record of a silent payments wallet: what the device keeps and
//! what its review states (`docs/PLANNING.md` §16.113).
//!
//! ```text
//! osk-silent 1
//! network mainnet
//! key [73c5da0a/352h/0h/0h]
//! labels 2
//! sp1qqgste7k9hx0qftg6qmwlkqtwuy6cycyavzmzj85c6qdf…
//! ```
//!
//! A silent payments wallet has no descriptor a miniscript library
//! reads and no address a chain index watches, so there is nothing for
//! [`crate::policy::WalletPolicy`] to hold but this: the kind, the
//! network, the origin of the key the two halves were derived from, how
//! many labels have been handed out, and the address itself, which
//! carries both public keys.
//!
//! Everything here is public. The scan private key, which is what
//! finding a payment needs, is never in this record: it is derived from
//! the loaded key whenever a label or a payment is checked, which is
//! why a silent payments wallet whose key is not loaded shows its
//! address and nothing else.
//!
//! BIP-392's `sp(spscan1q…)` descriptor would carry the scan private
//! key and is therefore an export rather than a record
//! ([`crate::silent::Receiver::descriptor`]).

use alloc::string::String;

use bitcoin::secp256k1::PublicKey;

use crate::keys::{Fingerprint, Network};
use crate::silent;

/// The first line's name.
pub const MAGIC: &str = "osk-silent";
/// The version this build writes and the only one it reads.
pub const VERSION: &str = "1";

/// The most labels one wallet hands out. A label is a row of the Labels
/// screen and a line of the kept record; twenty is what both hold.
pub const MAX_LABELS: u32 = 20;

const NETWORK_LINE: &str = "network ";
const KEY_LINE: &str = "key ";
const LABELS_LINE: &str = "labels ";

/// A silent payments wallet, as the device keeps it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SilentWallet {
    /// The network its addresses are on.
    pub network: Network,
    /// The master fingerprint of the key the two halves came from.
    pub fingerprint: Fingerprint,
    /// Which BIP-352 account they came from.
    pub account: u32,
    /// `B_scan`, which the address publishes.
    pub scan: PublicKey,
    /// `B_spend`, which the address publishes.
    pub spend: PublicKey,
    /// How many labels have been handed out: the wallet has labels 1
    /// through this number. The change label is 0 and is not among
    /// them.
    pub labels: u32,
}

impl SilentWallet {
    /// Whether `text` is offered as a record: its first line that is
    /// neither blank nor a comment names the format.
    pub fn looks_like_record(text: &str) -> bool {
        text.lines()
            .map(str::trim)
            .find(|l| !l.is_empty() && !l.starts_with('#'))
            .is_some_and(|l| l.starts_with(MAGIC))
    }

    /// The wallet's address, with no label.
    pub fn address(&self) -> String {
        match silent::encode_address(self.network, &self.scan, &self.spend) {
            Ok(text) => String::from(text.as_str()),
            Err(_) => String::new(),
        }
    }

    /// The key's origin as a descriptor writes it, which is the one
    /// fact the record states about the key.
    pub fn origin(&self) -> String {
        alloc::format!(
            "[{}/{}h/{}h/{}h]",
            self.fingerprint,
            silent::PURPOSE,
            self.network.coin_type(),
            self.account
        )
    }

    /// The whole record, as the blob and a file carry it.
    pub fn to_text(&self) -> String {
        alloc::format!(
            "{MAGIC} {VERSION}\n{NETWORK_LINE}{}\n{KEY_LINE}{}\n{LABELS_LINE}{}\n{}",
            self.network.name(),
            self.origin(),
            self.labels,
            self.address()
        )
    }

    /// Reads a record, checking everything it states: the network is
    /// one this build knows, the origin is a fingerprint and BIP-352's
    /// path on that network, the label count is within
    /// [`MAX_LABELS`], and the last line is an address on that
    /// network.
    pub fn parse(text: &str) -> Option<SilentWallet> {
        let mut lines = text
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty() && !l.starts_with('#'));
        match lines.next()?.split_once(' ') {
            Some((MAGIC, VERSION)) => {}
            _ => return None,
        }
        let network = Network::from_name(lines.next()?.strip_prefix(NETWORK_LINE)?)?;
        let origin = lines.next()?.strip_prefix(KEY_LINE)?;
        let labels: u32 = lines.next()?.strip_prefix(LABELS_LINE)?.parse().ok()?;
        if labels > MAX_LABELS {
            return None;
        }
        let address = lines.next()?;
        if lines.next().is_some() {
            return None;
        }
        let (stated, scan, spend) = silent::decode_address(address).ok()?;
        // A test network's addresses all carry the same part, so the
        // network the record names is the one that stands, as long as
        // the address does not contradict it.
        if stated.is_mainnet() != network.is_mainnet() {
            return None;
        }
        let (fingerprint, account) = read_origin(origin, network)?;
        Some(SilentWallet {
            network,
            fingerprint,
            account,
            scan,
            spend,
            labels,
        })
    }
}

/// `[73c5da0a/352h/0h/0h]`, read back to the fingerprint and the
/// account. The purpose and the coin type are BIP-352's and the
/// network's, and a record that states others is not this wallet.
fn read_origin(text: &str, network: Network) -> Option<(Fingerprint, u32)> {
    let inside = text.strip_prefix('[')?.strip_suffix(']')?;
    let mut parts = inside.split('/');
    let hex = parts.next()?;
    if hex.len() != 8 {
        return None;
    }
    let mut bytes = [0u8; 4];
    for (i, b) in bytes.iter_mut().enumerate() {
        *b = u8::from_str_radix(hex.get(i * 2..i * 2 + 2)?, 16).ok()?;
    }
    let hardened = |step: &str| -> Option<u32> {
        step.strip_suffix('h')
            .or_else(|| step.strip_suffix('\''))?
            .parse()
            .ok()
    };
    if hardened(parts.next()?)? != silent::PURPOSE {
        return None;
    }
    if hardened(parts.next()?)? != network.coin_type() {
        return None;
    }
    let account = hardened(parts.next()?)?;
    if parts.next().is_some() {
        return None;
    }
    Some((Fingerprint(bytes), account))
}
