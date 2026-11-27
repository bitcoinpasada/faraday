//! BOLT #12 offers, read only.
//!
//! [`decode`] takes the text of an offer (`lno1`, in one case, `+` joins
//! allowed, with or without a `lightning:` prefix) and returns what it
//! says. Nothing is requested, paid, signed or stored here; an offer
//! carries no signature, so nothing is verified and no [`Compute`] token is
//! taken. An invoice request (`lnr1`) is refused with [`Error::InvoiceRequest`]
//! and any other prefix, `lni1` included, with [`Error::Prefix`].
//!
//! BOLT #12 and BOLT #1 make these refusals, and Faraday refuses them: mixed
//! case; a `+` that does not stand between two Bech32 characters; a TLV
//! stream whose types do not strictly increase, whose length runs past its
//! end, or whose BigSize or truncated integer is not minimal; a type outside
//! 1 to 79 and 1,000,000,000 to 1,999,999,999; an unknown even type; a
//! feature field not minimally encoded or with an even bit set (BOLT #9
//! defines no offer feature, so every even bit is unknown); a description,
//! issuer or currency that is not BOLT #1's UTF-8; an amount without a
//! description, an amount of zero, a currency without an amount; neither an
//! issuer id nor a path; a path of no hops; a point that is not a compressed
//! secp256k1 key; chains present but empty or of a length that is not a
//! multiple of 32; more than four bits of padding.
//!
//! Where BOLT #12 leaves a case open Faraday refuses (fail closed): a text
//! over [`MAX_LEN`] characters, a non-zero padding bit, whitespace after a
//! `+` other than spaces, tabs and line breaks, a chain named twice, chains
//! naming no Bitcoin network Faraday knows, a currency that is not three
//! capital letters of [`CURRENCIES`], an amount in bitcoin over 21 million
//! bitcoin, an expiry past the year 9999, a field of a path that is not
//! well formed (a first node byte other than 0 to 3), an `offer_paths` that
//! holds no path, and a character of
//! Unicode's Cn category (unassigned in Unicode 16.0.0), which BOLT #1
//! allows a reader to refuse. Odd fields it does not define are listed in
//! [`Offer::unknown`], as BOLT #12 says to ignore them.
//!
//! The currency table is generated from the pinned file of the ISO 4217
//! maintenance agency (SIX), `tools/vectors/lightning-bolt12/iso4217-list-one.xml`:
//! each `Ccy` with a numeric `CcyMnrUnts`, sorted, once; a test reads the file
//! and requires the table to be exactly that.
//!
//! [`Compute`]: faraday_crypto::Compute

use alloc::string::String;
use alloc::vec::Vec;

use bitcoin::secp256k1::PublicKey;

use crate::bolt12_chars;

/// The longest text read: what a QR code holds in capital letters
/// (alphanumeric mode, version 40, error correction L).
pub const MAX_LEN: usize = 4296;
/// Most bitcoin an amount can be, in millisatoshis: 21 million bitcoin.
pub const MAX_MSAT: u64 = 21_000_000 * 100_000_000 * 1000;
/// The last second Faraday reads an expiry for: 9999-12-31 23:59:59 UTC.
pub const MAX_EXPIRY: u64 = 253_402_300_799;

const CHARSET: &[u8; 32] = b"qpzry9x8gf2tvdw0s3jn54khce6mua7l";

/// Why an offer was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// Over [`MAX_LEN`] characters.
    TooLong,
    /// Upper and lower case in one text.
    MixedCase,
    /// A `+` not between two Bech32 characters, or followed by something
    /// other than spaces, tabs and line breaks and then one.
    Join,
    /// Not Bech32: no separator, or a character outside the alphabet.
    Bech32,
    /// Not `lno1` (and not `lnr1`).
    Prefix,
    /// An invoice request, `lnr1`: Faraday reads offers.
    InvoiceRequest,
    /// More than four bits of padding, or a padding bit that is not zero.
    Padding,
    /// A field runs past the end of the data.
    Truncated,
    /// A BigSize or truncated integer that is not minimal.
    NotMinimal,
    /// Types do not strictly increase.
    Order,
    /// A type outside the ranges an offer may use.
    OutOfRange(u64),
    /// An even type this reader does not know.
    UnknownEven(u64),
    /// A field of a length or content its type does not allow.
    Field(u64),
    /// A description, issuer or currency that is not valid text.
    Text(u64),
    /// `offer_chains` present with no chain.
    ChainsEmpty,
    /// A chain named twice.
    ChainRepeated,
    /// No chain is a Bitcoin network Faraday knows.
    ChainUnknown,
    /// A currency that is not an ISO 4217 code Faraday knows.
    Currency,
    /// An amount of zero.
    AmountZero,
    /// An amount in bitcoin over 21 million bitcoin.
    AmountTooLarge,
    /// An amount without a description.
    AmountWithoutDescription,
    /// A currency without an amount.
    CurrencyWithoutAmount,
    /// An expiry past the year 9999.
    ExpiryTooLate,
    /// Features not minimally encoded.
    FeaturesNotMinimal,
    /// An even feature bit, which is required and unknown.
    UnknownRequiredFeature(u32),
    /// Neither an issuer id nor a path.
    NoIssuer,
    /// A path that is empty or not well formed, or has no hops.
    Path,
    /// A point that is not a compressed secp256k1 key (the type of the field).
    Point(u64),
}

/// A chain an offer is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Chain {
    /// Bitcoin mainnet.
    Bitcoin,
    /// Testnet3.
    Testnet3,
    /// Testnet4.
    Testnet4,
    /// The default signet.
    Signet,
    /// Regtest.
    Regtest,
    /// Any other chain, by its hash.
    Other([u8; 32]),
}

/// The genesis block hash of each known chain, in the order the blocks
/// are hashed (the reverse of the usual hex).
const CHAINS: [(Chain, [u8; 32]); 5] = [
    (
        Chain::Bitcoin,
        hash32(b"6fe28c0ab6f1b372c1a6a246ae63f74f931e8365e15a089c68d6190000000000"),
    ),
    (
        Chain::Testnet3,
        hash32(b"43497fd7f826957108f4a30fd9cec3aeba79972084e90ead01ea330900000000"),
    ),
    (
        Chain::Testnet4,
        hash32(b"43f08bdab050e35b567c864b91f47f50ae725ae2de53bcfbbaf284da00000000"),
    ),
    (
        Chain::Signet,
        hash32(b"f61eee3b63a380a477a063af32b2bbc97c9ff9f01f2c4225e973988108000000"),
    ),
    (
        Chain::Regtest,
        hash32(b"06226e46111a0b59caaf126043eb5bbf28c34f3a5e332a1fc7b2b73cf188910f"),
    ),
];

const fn hash32(hex: &[u8; 64]) -> [u8; 32] {
    const fn nibble(c: u8) -> u8 {
        if c >= b'a' { c - b'a' + 10 } else { c - b'0' }
    }
    let mut out = [0u8; 32];
    let mut i = 0;
    while i < 32 {
        out[i] = nibble(hex[2 * i]) << 4 | nibble(hex[2 * i + 1]);
        i += 1;
    }
    out
}

impl Chain {
    /// The chain with this genesis hash.
    pub fn from_hash(hash: [u8; 32]) -> Chain {
        CHAINS
            .iter()
            .find(|(_, h)| *h == hash)
            .map_or(Chain::Other(hash), |(c, _)| *c)
    }

    /// The genesis hash, as BOLT #12 writes it.
    pub fn hash(&self) -> [u8; 32] {
        match self {
            Chain::Other(h) => *h,
            known => CHAINS.iter().find(|(c, _)| c == known).expect("listed").1,
        }
    }
}

/// What an offer asks for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Amount {
    /// Millisatoshis.
    Bitcoin {
        /// The amount in millisatoshis (more than zero).
        msat: u64,
    },
    /// An amount in a currency, in the units its exponent gives.
    Currency {
        /// The ISO 4217 code, three capital letters.
        code: String,
        /// Digits after the decimal point in the currency's minor unit.
        exponent: u8,
        /// The amount in minor units (more than zero).
        units: u64,
    },
}

/// Where a blinded path starts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FirstNode {
    /// A node key.
    Node([u8; 33]),
    /// A channel and the direction of the node it leads from.
    Channel {
        /// The short channel id.
        short_channel_id: u64,
        /// 0 or 1.
        direction: u8,
    },
}

/// A blinded path to the node, shown by its first node and its hops.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlindedPath {
    /// Where the path starts.
    pub first: FirstNode,
    /// How many hops it has (at least one).
    pub hops: u8,
}

/// A field of an odd type this reader does not define, kept as read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownField {
    /// The TLV type.
    pub kind: u64,
    /// The value.
    pub data: Vec<u8>,
}

/// What an offer says.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Offer {
    /// The chains it is for, as written; empty means Bitcoin.
    pub chains: Vec<Chain>,
    /// The payee's own bytes, as written.
    pub metadata: Option<Vec<u8>>,
    /// What it asks for, or none for any amount.
    pub amount: Option<Amount>,
    /// The description.
    pub description: Option<String>,
    /// The feature bits set, ascending (all odd).
    pub features: Vec<u32>,
    /// The expiry in seconds since the epoch; not judged.
    pub expiry: Option<u64>,
    /// The blinded paths.
    pub paths: Vec<BlindedPath>,
    /// The issuer, in the issuer's own words.
    pub issuer: Option<String>,
    /// The most items per invoice; 0 is any number.
    pub quantity_max: Option<u64>,
    /// The node key invoices are asked from.
    pub issuer_id: Option<[u8; 33]>,
    /// Odd fields this reader does not define.
    pub unknown: Vec<UnknownField>,
}

/// Reads an offer.
pub fn decode(text: &str) -> Result<Offer, Error> {
    let text = text.trim();
    let text = match text.get(..10) {
        Some(p) if p.eq_ignore_ascii_case("lightning:") => &text[10..],
        _ => text,
    };
    if text.len() > MAX_LEN {
        return Err(Error::TooLong);
    }
    if text.bytes().any(|b| b.is_ascii_lowercase()) && text.bytes().any(|b| b.is_ascii_uppercase())
    {
        return Err(Error::MixedCase);
    }
    let joined = join(text)?;
    let sep = joined.find('1').ok_or(Error::Bech32)?;
    match &joined[..sep] {
        "lno" => {}
        "lnr" => return Err(Error::InvoiceRequest),
        _ => return Err(Error::Prefix),
    }
    let bytes = to_bytes(&joined[sep + 1..])?;
    offer(&bytes)
}

/// Whether a character is one of Bech32's 32, in either case.
fn is_bech32(c: char) -> bool {
    c.is_ascii() && CHARSET.contains(&(c.to_ascii_lowercase() as u8))
}

/// The text with each `+` and the spaces, tabs and line breaks after it
/// removed, in lower case.
fn join(text: &str) -> Result<String, Error> {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '+' {
            if i == 0 || !is_bech32(chars[i - 1]) {
                return Err(Error::Join);
            }
            i += 1;
            while i < chars.len() && matches!(chars[i], ' ' | '\t' | '\n' | '\r') {
                i += 1;
            }
            if i == chars.len() || !is_bech32(chars[i]) {
                return Err(Error::Join);
            }
        }
        out.push(chars[i].to_ascii_lowercase());
        i += 1;
    }
    Ok(out)
}

/// Bech32 characters without a checksum to bytes; at most four bits of
/// padding, all zero.
fn to_bytes(data: &str) -> Result<Vec<u8>, Error> {
    let mut out = Vec::with_capacity(data.len() * 5 / 8);
    let (mut acc, mut bits) = (0u32, 0u32);
    for c in data.bytes() {
        let v = CHARSET.iter().position(|&x| x == c).ok_or(Error::Bech32)?;
        acc = acc << 5 | v as u32;
        bits += 5;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
            acc &= (1 << bits) - 1;
        }
    }
    if bits > 4 || acc != 0 {
        return Err(Error::Padding);
    }
    Ok(out)
}

/// A BigSize at `*at`, minimal.
fn bigsize(b: &[u8], at: &mut usize) -> Result<u64, Error> {
    let first = *b.get(*at).ok_or(Error::Truncated)?;
    *at += 1;
    let (n, min) = match first {
        0..=0xfc => return Ok(u64::from(first)),
        0xfd => (2, 0xfd),
        0xfe => (4, 0x1_0000),
        _ => (8, 0x1_0000_0000),
    };
    let raw = b.get(*at..*at + n).ok_or(Error::Truncated)?;
    *at += n;
    let v = raw.iter().fold(0u64, |a, &x| a << 8 | u64::from(x));
    if v < min {
        return Err(Error::NotMinimal);
    }
    Ok(v)
}

/// A truncated unsigned integer: at most eight bytes, no leading zero.
fn tu64(v: &[u8], kind: u64) -> Result<u64, Error> {
    if v.len() > 8 {
        return Err(Error::Field(kind));
    }
    if v.first() == Some(&0) {
        return Err(Error::NotMinimal);
    }
    Ok(v.iter().fold(0u64, |a, &x| a << 8 | u64::from(x)))
}

fn point(v: &[u8], kind: u64) -> Result<[u8; 33], Error> {
    let p: [u8; 33] = v.try_into().map_err(|_| Error::Point(kind))?;
    PublicKey::from_slice(&p).map_err(|_| Error::Point(kind))?;
    Ok(p)
}

fn text(v: &[u8], kind: u64) -> Result<String, Error> {
    let s = core::str::from_utf8(v).map_err(|_| Error::Text(kind))?;
    if !s.chars().all(bolt12_chars::allowed) {
        return Err(Error::Text(kind));
    }
    Ok(String::from(s))
}

fn offer(b: &[u8]) -> Result<Offer, Error> {
    let mut offer = Offer {
        chains: Vec::new(),
        metadata: None,
        amount: None,
        description: None,
        features: Vec::new(),
        expiry: None,
        paths: Vec::new(),
        issuer: None,
        quantity_max: None,
        issuer_id: None,
        unknown: Vec::new(),
    };
    let mut currency = None;
    let mut amount = None;
    let (mut at, mut last) = (0, None);
    while at < b.len() {
        let kind = bigsize(b, &mut at)?;
        let len = bigsize(b, &mut at)?;
        if last.is_some_and(|l| kind <= l) {
            return Err(Error::Order);
        }
        last = Some(kind);
        let len = usize::try_from(len).map_err(|_| Error::Truncated)?;
        if len > b.len() - at {
            return Err(Error::Truncated);
        }
        let v = &b[at..at + len];
        at += len;
        if !(1..=79).contains(&kind) && !(1_000_000_000..=1_999_999_999).contains(&kind) {
            return Err(Error::OutOfRange(kind));
        }
        match kind {
            2 => offer.chains = chains(v)?,
            4 => offer.metadata = Some(v.to_vec()),
            6 => currency = Some(v),
            8 => amount = Some(tu64(v, kind)?),
            10 => offer.description = Some(text(v, kind)?),
            12 => offer.features = features(v)?,
            14 => {
                let e = tu64(v, kind)?;
                if e > MAX_EXPIRY {
                    return Err(Error::ExpiryTooLate);
                }
                offer.expiry = Some(e);
            }
            16 => offer.paths = paths(v)?,
            18 => offer.issuer = Some(text(v, kind)?),
            20 => offer.quantity_max = Some(tu64(v, kind)?),
            22 => offer.issuer_id = Some(point(v, kind)?),
            _ if kind % 2 == 1 => offer.unknown.push(UnknownField {
                kind,
                data: v.to_vec(),
            }),
            _ => return Err(Error::UnknownEven(kind)),
        }
    }
    offer.amount = match (currency, amount) {
        (None, None) => None,
        (Some(_), None) => return Err(Error::CurrencyWithoutAmount),
        (_, Some(0)) => return Err(Error::AmountZero),
        (None, Some(msat)) if msat > MAX_MSAT => return Err(Error::AmountTooLarge),
        (None, Some(msat)) => Some(Amount::Bitcoin { msat }),
        (Some(c), Some(units)) => {
            let code = core::str::from_utf8(c).map_err(|_| Error::Currency)?;
            let exponent = CURRENCIES
                .binary_search_by(|(k, _)| (*k).cmp(code))
                .map(|i| CURRENCIES[i].1)
                .map_err(|_| Error::Currency)?;
            Some(Amount::Currency {
                code: String::from(code),
                exponent,
                units,
            })
        }
    };
    if offer.amount.is_some() && offer.description.is_none() {
        return Err(Error::AmountWithoutDescription);
    }
    if offer.issuer_id.is_none() && offer.paths.is_empty() {
        return Err(Error::NoIssuer);
    }
    Ok(offer)
}

fn chains(v: &[u8]) -> Result<Vec<Chain>, Error> {
    if v.is_empty() {
        return Err(Error::ChainsEmpty);
    }
    if !v.len().is_multiple_of(32) {
        return Err(Error::Field(2));
    }
    let mut out: Vec<Chain> = Vec::new();
    for h in v.chunks_exact(32) {
        let c = Chain::from_hash(h.try_into().expect("32 bytes"));
        if out.contains(&c) {
            return Err(Error::ChainRepeated);
        }
        out.push(c);
    }
    if out.iter().all(|c| matches!(c, Chain::Other(_))) {
        return Err(Error::ChainUnknown);
    }
    Ok(out)
}

/// The bits set in a bitmap whose last byte holds bit 0, ascending; an
/// even bit is refused, as BOLT #9 defines no offer feature.
fn features(v: &[u8]) -> Result<Vec<u32>, Error> {
    if v.first() == Some(&0) {
        return Err(Error::FeaturesNotMinimal);
    }
    let mut bits = Vec::new();
    for (i, byte) in v.iter().rev().enumerate() {
        for bit in 0..8 {
            if byte >> bit & 1 == 1 {
                bits.push(i as u32 * 8 + bit);
            }
        }
    }
    if let Some(&even) = bits.iter().find(|b| b.is_multiple_of(2)) {
        return Err(Error::UnknownRequiredFeature(even));
    }
    Ok(bits)
}

fn paths(v: &[u8]) -> Result<Vec<BlindedPath>, Error> {
    let mut out = Vec::new();
    let mut at = 0;
    let take = |at: &mut usize, n: usize| -> Result<&[u8], Error> {
        let s = v.get(*at..*at + n).ok_or(Error::Path)?;
        *at += n;
        Ok(s)
    };
    while at < v.len() {
        let first = match take(&mut at, 1)?[0] {
            d @ (0 | 1) => {
                let id = take(&mut at, 8)?;
                FirstNode::Channel {
                    short_channel_id: id.iter().fold(0u64, |a, &x| a << 8 | u64::from(x)),
                    direction: d,
                }
            }
            2 | 3 => {
                at -= 1;
                let p = take(&mut at, 33)?;
                FirstNode::Node(point(p, 16).map_err(|_| Error::Path)?)
            }
            _ => return Err(Error::Path),
        };
        point(take(&mut at, 33)?, 16).map_err(|_| Error::Path)?;
        let hops = take(&mut at, 1)?[0];
        if hops == 0 {
            return Err(Error::Path);
        }
        for _ in 0..hops {
            point(take(&mut at, 33)?, 16).map_err(|_| Error::Path)?;
            let n = take(&mut at, 2)?;
            take(&mut at, usize::from(n[0]) << 8 | usize::from(n[1]))?;
        }
        out.push(BlindedPath { first, hops });
    }
    if out.is_empty() {
        return Err(Error::Path);
    }
    Ok(out)
}

/// The ISO 4217 currencies an offer may name, with the digits after the
/// decimal point of each one's minor unit, sorted by code. Funds with no
/// minor unit (metals, the testing code, the IMF's) are not listed. It is
/// ISO 4217's own list, `tools/vectors/lightning-bolt12/iso4217-list-one.xml`:
/// every `Ccy` whose `CcyMnrUnts` is a digit, once; a test requires the match.
#[rustfmt::skip]
pub const CURRENCIES: &[(&str, u8)] = &[
    ("AED", 2), ("AFN", 2), ("ALL", 2), ("AMD", 2), ("AOA", 2), ("ARS", 2), ("AUD", 2),
    ("AWG", 2), ("AZN", 2), ("BAM", 2), ("BBD", 2), ("BDT", 2), ("BHD", 3), ("BIF", 0),
    ("BMD", 2), ("BND", 2), ("BOB", 2), ("BOV", 2), ("BRL", 2), ("BSD", 2), ("BTN", 2),
    ("BWP", 2), ("BYN", 2), ("BZD", 2), ("CAD", 2), ("CDF", 2), ("CHE", 2), ("CHF", 2),
    ("CHW", 2), ("CLF", 4), ("CLP", 0), ("CNY", 2), ("COP", 2), ("COU", 2), ("CRC", 2),
    ("CUP", 2), ("CVE", 2), ("CZK", 2), ("DJF", 0), ("DKK", 2), ("DOP", 2), ("DZD", 2),
    ("EGP", 2), ("ERN", 2), ("ETB", 2), ("EUR", 2), ("FJD", 2), ("FKP", 2), ("GBP", 2),
    ("GEL", 2), ("GHS", 2), ("GIP", 2), ("GMD", 2), ("GNF", 0), ("GTQ", 2), ("GYD", 2),
    ("HKD", 2), ("HNL", 2), ("HTG", 2), ("HUF", 2), ("IDR", 2), ("ILS", 2), ("INR", 2),
    ("IQD", 3), ("IRR", 2), ("ISK", 0), ("JMD", 2), ("JOD", 3), ("JPY", 0), ("KES", 2),
    ("KGS", 2), ("KHR", 2), ("KMF", 0), ("KPW", 2), ("KRW", 0), ("KWD", 3), ("KYD", 2),
    ("KZT", 2), ("LAK", 2), ("LBP", 2), ("LKR", 2), ("LRD", 2), ("LSL", 2), ("LYD", 3),
    ("MAD", 2), ("MDL", 2), ("MGA", 2), ("MKD", 2), ("MMK", 2), ("MNT", 2), ("MOP", 2),
    ("MRU", 2), ("MUR", 2), ("MVR", 2), ("MWK", 2), ("MXN", 2), ("MXV", 2), ("MYR", 2),
    ("MZN", 2), ("NAD", 2), ("NGN", 2), ("NIO", 2), ("NOK", 2), ("NPR", 2), ("NZD", 2),
    ("OMR", 3), ("PAB", 2), ("PEN", 2), ("PGK", 2), ("PHP", 2), ("PKR", 2), ("PLN", 2),
    ("PYG", 0), ("QAR", 2), ("RON", 2), ("RSD", 2), ("RUB", 2), ("RWF", 0), ("SAR", 2),
    ("SBD", 2), ("SCR", 2), ("SDG", 2), ("SEK", 2), ("SGD", 2), ("SHP", 2), ("SLE", 2),
    ("SOS", 2), ("SRD", 2), ("SSP", 2), ("STN", 2), ("SVC", 2), ("SYP", 2), ("SZL", 2),
    ("THB", 2), ("TJS", 2), ("TMT", 2), ("TND", 3), ("TOP", 2), ("TRY", 2), ("TTD", 2),
    ("TWD", 2), ("TZS", 2), ("UAH", 2), ("UGX", 0), ("USD", 2), ("USN", 2), ("UYI", 0),
    ("UYU", 2), ("UYW", 4), ("UZS", 2), ("VED", 2), ("VES", 2), ("VND", 0), ("VUV", 0),
    ("WST", 2), ("XAD", 2), ("XAF", 0), ("XCD", 2), ("XCG", 2), ("XOF", 0), ("XPF", 0),
    ("YER", 2), ("ZAR", 2), ("ZMW", 2), ("ZWG", 2),
];
