//! An address as a person types or scans it: whether the text so far
//! can still become an address on a network, and what the finished text
//! is (`docs/UX.md` §7.4, G1).
//!
//! Where an address is looked for is [`crate::account::AccountXpub::find_address`]
//! for a key's own accounts and [`crate::policy::WalletPolicy::find_address`]
//! for a wallet.

use alloc::string::String;
use core::str::FromStr;

use bitcoin::address::{Address, NetworkUnchecked};

use crate::keys::Network;

/// Why a text is not an address on the network asked about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// Not a Bitcoin address.
    Invalid,
    /// An address of another network.
    WrongNetwork,
    /// A silent payment address (BIP-352): an address, but no chain of
    /// addresses holds one, because nothing is ever paid to it directly.
    Silent,
}

/// The bech32 alphabet, which is every lower-case letter but `b`, `i`
/// and `o` and every digit but `1`.
const BECH32: &str = "qpzry9x8gf2tvdw0s3jn54khce6mua7l";

/// The base58 alphabet: the digits and both cases of the letters,
/// without the four characters that are read for one another.
const BASE58: &str = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";

/// Characters a bech32 address takes in all, BIP-173's limit.
const MAX_BECH32: usize = 90;

/// Characters a base58 address takes in all: twenty-five bytes.
const MAX_BASE58: usize = 35;

/// The human-readable part of a bech32 address on `network`.
pub fn hrp(network: Network) -> &'static str {
    match network {
        Network::Mainnet => "bc",
        Network::Testnet | Network::Signet => "tb",
        Network::Regtest => "bcrt",
    }
}

/// The characters a base58 address on `network` begins with: the
/// version byte of a legacy or a nested address.
fn base58_leads(network: Network) -> &'static str {
    match network {
        Network::Mainnet => "13",
        _ => "mn2",
    }
}

/// Whether `text` is the beginning of a bech32 address on `network`:
/// the human-readable part and the separator as far as they have been
/// typed, then the alphabet.
pub fn is_bech32_prefix(text: &str, network: Network) -> bool {
    let mut start = String::from(hrp(network));
    start.push('1');
    if start.starts_with(text) {
        return true;
    }
    match text.strip_prefix(start.as_str()) {
        Some(rest) => text.len() <= MAX_BECH32 && rest.chars().all(|c| BECH32.contains(c)),
        None => false,
    }
}

/// Whether `text` is the beginning of an address on `network` in either
/// encoding. An empty text is the beginning of both.
pub fn is_address_prefix(text: &str, network: Network) -> bool {
    if is_bech32_prefix(text, network) {
        return true;
    }
    let mut chars = text.chars();
    match chars.next() {
        None => true,
        Some(first) if base58_leads(network).contains(first) => {
            text.len() <= MAX_BASE58 && chars.all(|c| BASE58.contains(c))
        }
        Some(_) => false,
    }
}

/// Whether `text` is an address on some network, silent payment
/// addresses included.
pub fn parses(text: &str) -> bool {
    Address::<NetworkUnchecked>::from_str(text.trim()).is_ok()
        || crate::silent::decode_address(text.trim()).is_ok()
}

/// `text` as an address on `network`, the spaces at its ends ignored.
pub fn parse(text: &str, network: Network) -> Result<Address, Error> {
    if crate::silent::address_of(text.trim()).is_some() {
        return Err(Error::Silent);
    }
    let unchecked =
        Address::<NetworkUnchecked>::from_str(text.trim()).map_err(|_| Error::Invalid)?;
    unchecked
        .require_network(network.into())
        .map_err(|_| Error::WrongNetwork)
}
