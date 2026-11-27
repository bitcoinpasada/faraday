//! An extended private key and a hex master seed read back as keys
//! (`docs/KEY-FORMS.md` §6, F300). Every refusal is decided here: what the
//! text is, which network it is for, whether it is a master key and
//! whether the origin written before it names it; the strict reader of
//! `faraday_bip::xkey` decides what is not an extended private key at all.
#![cfg_attr(not(test), warn(clippy::disallowed_methods))]

use faraday_bip::bitcoin::NetworkKind;
use faraday_bip::bitcoin::bip32::ChildNumber;
use faraday_bip::keys::{Fingerprint, MasterKey, Network};
use faraday_bip::xkey;
use faraday_crypto::work::Compute;
use zeroize::Zeroizing;

/// SLIP-132's private spellings, by version bytes and name.
const SLIP132_PRIVATE: [([u8; 4], &str); 8] = [
    ([0x04, 0x9d, 0x78, 0x78], "yprv"),
    ([0x04, 0xb2, 0x43, 0x0c], "zprv"),
    ([0x02, 0x95, 0xb0, 0x05], "Yprv"),
    ([0x02, 0xaa, 0x7a, 0x99], "Zprv"),
    ([0x04, 0x4a, 0x4e, 0x28], "uprv"),
    ([0x04, 0x5f, 0x18, 0xbc], "vprv"),
    ([0x02, 0x42, 0x85, 0xb5], "Uprv"),
    ([0x02, 0x57, 0x50, 0x48], "Vprv"),
];

/// Why an extended private key was not read as a master key.
#[derive(Debug, PartialEq, Eq)]
pub enum XprvRefusal {
    /// Not an extended private key by the strict reader: not base58check,
    /// not 78 bytes, an unknown version, a depth-0 key with a parent or an
    /// index, a pad byte that is not zero or a private key out of range.
    Unreadable(xkey::Error),
    /// A SLIP-132 private spelling (yprv, zprv, Yprv, Zprv, uprv, vprv,
    /// Uprv, Vprv), by its name.
    Slip132(&'static str),
    /// The version is the other network's: `found` is what it is written
    /// as, `wanted` what this network's is (`xprv` or `tprv`).
    WrongNetwork {
        /// The version the text is written in.
        found: &'static str,
        /// The version this network reads.
        wanted: &'static str,
    },
    /// A valid key that is not a master key: its depth, its parent's
    /// fingerprint and its child number, all public.
    NotMaster {
        /// Its depth.
        depth: u8,
        /// Its parent's fingerprint.
        parent: Fingerprint,
        /// Its child number without the hardened bit.
        index: u32,
        /// Whether the child number is hardened.
        hardened: bool,
    },
    /// The origin written before the key is not `[` and eight hex digits
    /// and `]`.
    OriginUnreadable,
    /// The origin carries a path; only a key's own fingerprint is taken.
    OriginPath,
    /// The origin names another fingerprint than the key's own.
    OriginDiffers {
        /// The fingerprint the origin names.
        named: Fingerprint,
        /// The key's own.
        own: Fingerprint,
    },
}

fn version_name(network: Network) -> &'static str {
    match network.kind() {
        NetworkKind::Main => "xprv",
        NetworkKind::Test => "tprv",
    }
}

/// The origin's fingerprint and the key text after it; the text itself
/// when it has no origin.
fn split_origin(text: &str) -> Result<(Option<Fingerprint>, &str), XprvRefusal> {
    let text = text.trim();
    let Some(rest) = text.strip_prefix('[') else {
        return Ok((None, text));
    };
    let (origin, key) = rest.split_once(']').ok_or(XprvRefusal::OriginUnreadable)?;
    if origin.contains('/') {
        return Err(XprvRefusal::OriginPath);
    }
    let fp: [u8; 4] =
        faraday_crypto::hex::decode_exact(origin).ok_or(XprvRefusal::OriginUnreadable)?;
    Ok((Some(Fingerprint(fp)), key.trim_start()))
}

/// Costs two base58 checks (the text is decoded twice) and an elliptic-curve
/// multiplication. The master key the extended private key `text` is, on
/// `network`, by the strict reader: its version is `network`'s, it is at
/// depth 0, and an origin written before it names its own fingerprint and
/// no path.
pub(crate) fn read_master(
    c: &Compute,
    text: &str,
    network: Network,
) -> Result<MasterKey, XprvRefusal> {
    let (origin, key_text) = split_origin(text)?;
    let mut x = xkey::decode_xpriv(key_text).map_err(|e| match e {
        xkey::Error::UnknownVersion(v) => SLIP132_PRIVATE
            .iter()
            .find(|(bytes, _)| *bytes == v)
            .map_or(XprvRefusal::Unreadable(e), |(_, name)| {
                XprvRefusal::Slip132(name)
            }),
        e => XprvRefusal::Unreadable(e),
    })?;
    let facts = (x.network, x.depth, x.parent_fingerprint, x.child_number);
    x.private_key.non_secure_erase();
    let (kind, depth, parent, child) = facts;
    if kind != network.kind() {
        let (found, wanted) = match kind {
            NetworkKind::Main => ("xprv", version_name(network)),
            NetworkKind::Test => ("tprv", version_name(network)),
        };
        return Err(XprvRefusal::WrongNetwork { found, wanted });
    }
    if depth != 0 {
        let (index, hardened) = match child {
            ChildNumber::Normal { index } => (index, false),
            ChildNumber::Hardened { index } => (index, true),
        };
        return Err(XprvRefusal::NotMaster {
            depth,
            parent: parent.into(),
            index,
            hardened,
        });
    }
    let master = MasterKey::decode(c, key_text).map_err(|e| match e {
        faraday_bip::keys::ImportError::Decode(e) => XprvRefusal::Unreadable(e),
        faraday_bip::keys::ImportError::NotMaster => XprvRefusal::NotMaster {
            depth: 0,
            parent: Fingerprint([0; 4]),
            index: 0,
            hardened: false,
        },
    })?;
    if let Some(named) = origin
        && named != master.fingerprint()
    {
        return Err(XprvRefusal::OriginDiffers {
            named,
            own: master.fingerprint(),
        });
    }
    Ok(master.for_network(network))
}

/// Why [`crate::Keys::add_xprv`] did not add a key.
#[derive(Debug, PartialEq, Eq)]
pub enum XprvError {
    /// The text is not a master key to load.
    Read(XprvRefusal),
    /// The session refuses the key: a stick attached, a key the demo does
    /// not take, or one already held.
    Session(crate::Refusal),
}

/// Why a hex master seed was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(
    clippy::disallowed_methods,
    reason = "a derived Clone: no secret type derives it"
)]
pub enum HexSeedRefusal {
    /// A character that is not a hex digit or a space.
    NotHex,
    /// An odd count of hex digits: the count.
    OddDigits(usize),
    /// A seed of another length than 16 to 64 bytes: its length in bytes.
    Length(usize),
}

/// A master seed's shortest and longest length in bytes (BIP-32).
const SEED_BYTES: std::ops::RangeInclusive<usize> = 16..=64;

/// The master seed written in hex in `text`: 32 to 128 digits, an even
/// count, either case, spaces ignored. The bytes wipe themselves when
/// dropped.
pub fn parse_hex_seed(text: &str) -> Result<Zeroizing<Vec<u8>>, HexSeedRefusal> {
    // Each digit's value, so no copy of the text is made.
    let mut digits = Zeroizing::new(Vec::with_capacity(text.len()));
    for ch in text.chars().filter(|&ch| ch != ' ') {
        digits.push(ch.to_digit(16).ok_or(HexSeedRefusal::NotHex)? as u8);
    }
    if !digits.len().is_multiple_of(2) {
        return Err(HexSeedRefusal::OddDigits(digits.len()));
    }
    let len = digits.len() / 2;
    if !SEED_BYTES.contains(&len) {
        return Err(HexSeedRefusal::Length(len));
    }
    Ok(Zeroizing::new(
        digits.chunks(2).map(|d| d[0] << 4 | d[1]).collect(),
    ))
}
