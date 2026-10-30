//! The record of a silent payments wallet: what the device keeps and
//! what its review states.
//!
//! ```text
//! faraday-silent 1
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
use faraday_crypto::work::Compute;

/// The first line's name.
pub const MAGIC: &str = "faraday-silent";
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

/// Why BIP-392's `sp()` descriptor, or its key expression alone, was not
/// read as a silent payments wallet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadError {
    /// No key origin before the key: which key the wallet came from, and
    /// at which account, is not stated, and is not guessed.
    NoOrigin,
    /// An origin that is not a fingerprint and BIP-352's path on the
    /// key's network to a hardened account.
    Origin,
    /// The key is not `spscan1q…` or `tspscan1q…` as BIP-392 writes it.
    Key(silent::Error),
    /// The descriptor's checksum does not hold.
    Checksum,
    /// Not `sp(KEY)` or `[origin]KEY` alone: two keys, another
    /// function, a key with a derivation after it.
    Shape,
}

/// Costs what [`silent::decode_scan_key`] costs: an elliptic-curve
/// multiplication. A silent payments wallet from BIP-392's
/// `sp([fingerprint/352h/coinh/accounth]spscan1q…)`, its BIP-380
/// checksum checked where it has one, or from the key expression alone,
/// `[origin]spscan1q…`, as Specter DIY writes it. The wallet has no labels
/// handed out. The scan private key the expression carries is read only
/// for its public key and wiped ([`silent::ScanKeyRead`]). An expression
/// with no origin is refused ([`ReadError::NoOrigin`]): the origin is
/// what a loaded key is checked against.
pub fn parse_descriptor(c: &Compute, text: &str) -> Result<SilentWallet, ReadError> {
    let text = text.trim();
    let (body, sum) = match text.split_once('#') {
        Some((body, sum)) => (body, Some(sum)),
        None => (text, None),
    };
    let expr = match body.strip_prefix("sp(") {
        Some(inner) => inner.strip_suffix(')').ok_or(ReadError::Shape)?,
        // A checksum belongs to a descriptor, not to a key alone.
        None if sum.is_none() => body,
        None => return Err(ReadError::Shape),
    };
    if let Some(sum) = sum {
        let want = crate::descriptor::descriptor_checksum(body).ok_or(ReadError::Checksum)?;
        if sum.as_bytes() != want {
            return Err(ReadError::Checksum);
        }
    }
    // One key expression, with nothing after the key: no second key, no
    // derivation, no function inside.
    let one_key = |k: &str| !k.contains([',', '(', ')', '/', '*', '[', ']']);
    let Some(rest) = expr.strip_prefix('[') else {
        return Err(if one_key(expr) {
            ReadError::NoOrigin
        } else {
            ReadError::Shape
        });
    };
    let (origin, key) = rest.split_once(']').ok_or(ReadError::Shape)?;
    if !one_key(key) {
        return Err(ReadError::Shape);
    }
    let read = silent::decode_scan_key(c, key).map_err(ReadError::Key)?;
    wallet_of(read, &alloc::format!("[{origin}]"))
}

/// Costs what [`silent::decode_scan_key`] costs. A silent payments wallet
/// from a scan key expression and the origin a file states beside it:
/// Coldcard's export, whose `bip352` account carries the key and its
/// `deriv` and is rooted at the master's `xfp`, the file's top-level one
/// (`docs/INTEROP.md` §5 I9).
pub fn from_scan_key(
    c: &Compute,
    key: &str,
    fingerprint: &str,
    deriv: &str,
) -> Result<SilentWallet, ReadError> {
    let path = deriv.trim().trim_start_matches('m').trim_start_matches('/');
    if fingerprint.contains(['[', ']', '/']) || path.contains(['[', ']']) {
        return Err(ReadError::Origin);
    }
    let read = silent::decode_scan_key(c, key).map_err(ReadError::Key)?;
    wallet_of(read, &alloc::format!("[{}/{path}]", fingerprint.trim()))
}

/// The wallet a scan key read and its origin `[…]` state, the origin
/// checked as a record's is.
fn wallet_of(read: silent::ScanKeyRead, origin: &str) -> Result<SilentWallet, ReadError> {
    let (fingerprint, account) = read_origin(origin, read.network).ok_or(ReadError::Origin)?;
    if account >= 1 << 31 {
        return Err(ReadError::Origin);
    }
    Ok(SilentWallet {
        network: read.network,
        fingerprint,
        account,
        scan: read.scan,
        spend: read.spend,
        labels: 0,
    })
}
