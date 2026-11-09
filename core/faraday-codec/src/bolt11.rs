//! BOLT #11 Lightning invoices, read only.
//!
//! [`decode`] takes the text of an invoice (`lnbc`, `lntb`, `lntbs` or
//! `lnbcrt`, in one case, with or without a `lightning:` prefix) and
//! returns what it says. Nothing is paid, signed or stored here.
//!
//! BOLT #11 makes a bad invoice a refusal, and Faraday fails closed: a
//! Bech32 checksum that does not hold, a field cut short, a `p`, `h`, `s`
//! or `n` field of another length, a repeated `p` or `s`, an invoice
//! without a payment hash or a payment secret, one with both or neither
//! of a description and its hash, a feature the invoice requires that is
//! not listed in [`KNOWN_REQUIRED_FEATURES`], and a signature that does
//! not verify are each refused with an [`Error`], as BOLT #11 says.
//! A mixed-case text, an amount finer than a millisatoshi, and a `c`, `x`
//! or `9` field that begins with a zero value (BOLT #11 says to treat it
//! as invalid; the minimal form of zero is an empty field, read as 0) are
//! refused as BOLT #11 and BIP 173 say.
//! Faraday is stricter than BOLT #11 where it leaves a case open: a text
//! over 1023 characters (the length the Bech32 checksum guarantees), an
//! amount with a leading zero, a repeated `x`, `c`, `m`, `9`, `d`, `h` or
//! `n` field, a fallback address of a version it defines that is malformed, a route
//! hint that is empty or has a hop key that is not a curve point, an `n`
//! key that is not on the curve, and a non-zero padding bit in a field.
//! A field of a tag BOLT #11 does not define, and a fallback address of a
//! version it does not define, are listed in [`Invoice::unknown`], as
//! BOLT #11 says.
//!
//! The signature is checked against the `n` field when there is one
//! (which refuses a high-S signature) and otherwise the payee key is
//! recovered from it. Like message verification it takes no
//! `Compute` token: it runs on an event, never while drawing.

use alloc::string::String;
use alloc::vec::Vec;

use bitcoin::Address;
use bitcoin::PubkeyHash;
use bitcoin::ScriptBuf;
use bitcoin::ScriptHash;
use bitcoin::hashes::{Hash, sha256};
use bitcoin::secp256k1::ecdsa::{RecoverableSignature, RecoveryId, Signature};
use bitcoin::secp256k1::{Message, PublicKey, Secp256k1};
use bitcoin::{WitnessProgram, WitnessVersion};
use faraday_bip::keys::Network;

/// The longest text read: what the Bech32 checksum guarantees.
pub const MAX_LEN: usize = 1023;
/// Seconds an invoice is good for when it has no `x` field.
pub const DEFAULT_EXPIRY: u64 = 3600;
/// The `min_final_cltv_expiry_delta` when there is no `c` field.
pub const DEFAULT_MIN_FINAL_CLTV: u64 = 18;
/// The even (required) feature bits an invoice may carry: var_onion_optin,
/// payment_secret, basic_mpp, route_blinding, payment_metadata, and the
/// assumed ones (data_loss_protect, static_remotekey, channel_type).
pub const KNOWN_REQUIRED_FEATURES: [u16; 8] = [0, 8, 12, 14, 16, 24, 44, 48];

const CHARSET: &[u8; 32] = b"qpzry9x8gf2tvdw0s3jn54khce6mua7l";
const SIGNATURE_VALUES: usize = 104;

/// Why an invoice was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// Over [`MAX_LEN`] characters.
    TooLong,
    /// Upper and lower case in one text.
    MixedCase,
    /// Not Bech32: no separator, a character outside the alphabet.
    Bech32,
    /// The Bech32 checksum does not hold.
    Checksum,
    /// Does not start `ln` and a known network.
    Prefix,
    /// The amount in the human-readable part is malformed.
    Amount,
    /// Shorter than a timestamp and a signature.
    TooShort,
    /// A field runs past the end of the data.
    Truncated,
    /// A field is of a length or content its tag does not allow.
    Field(char),
    /// A field that may appear once appears twice.
    Duplicate(char),
    /// No `p` field.
    NoPaymentHash,
    /// No `s` field.
    NoPaymentSecret,
    /// Both or neither of `d` and `h`.
    Description,
    /// A required feature bit this reader does not know.
    UnknownRequiredFeature(u16),
    /// The signature does not verify, or no key can be recovered from it.
    Signature,
}

/// A fallback on-chain address.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Fallback {
    /// Pay to public key hash.
    P2pkh([u8; 20]),
    /// Pay to script hash.
    P2sh([u8; 20]),
    /// A witness program of a version 0 to 16.
    Witness {
        /// The version, 0 to 16.
        version: u8,
        /// The program.
        program: Vec<u8>,
    },
}

impl Fallback {
    /// The address text on `network`.
    pub fn address(&self, network: Network) -> String {
        let net = bitcoin::Network::from(network);
        let address = match self {
            Fallback::P2pkh(h) => {
                Address::from_script(&ScriptBuf::new_p2pkh(&PubkeyHash::from_byte_array(*h)), net)
            }
            Fallback::P2sh(h) => {
                Address::from_script(&ScriptBuf::new_p2sh(&ScriptHash::from_byte_array(*h)), net)
            }
            Fallback::Witness { version, program } => {
                let v = WitnessVersion::try_from(*version).expect("checked at decode");
                let p = WitnessProgram::new(v, program).expect("checked at decode");
                Ok(Address::from_witness_program(p, net))
            }
        };
        address.map(|a| a.to_string()).unwrap_or_default()
    }
}

/// One hop of a route hint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteHop {
    /// The node the hop starts at.
    pub node_id: [u8; 33],
    /// The short channel id.
    pub short_channel_id: u64,
    /// Base fee in millisatoshis.
    pub fee_base_msat: u32,
    /// Proportional fee in millionths.
    pub fee_proportional_millionths: u32,
    /// The channel's cltv_expiry_delta.
    pub cltv_expiry_delta: u16,
}

/// A field this reader does not define, kept as read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownField {
    /// The tag character (a fallback of an unknown version is `f`).
    pub tag: char,
    /// The data as 5-bit values.
    pub data: Vec<u8>,
}

/// What an invoice says.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Invoice {
    /// The network.
    pub network: Network,
    /// The amount in millisatoshis, or none for "any amount".
    pub amount_msat: Option<u64>,
    /// Seconds since the epoch the invoice was made.
    pub timestamp: u64,
    /// The payment hash.
    pub payment_hash: [u8; 32],
    /// The description, when there is one.
    pub description: Option<String>,
    /// The hash of a description kept elsewhere.
    pub description_hash: Option<[u8; 32]>,
    /// The payee's node key.
    pub payee: [u8; 33],
    /// Whether the payee key is the `n` field (otherwise it is recovered
    /// from the signature).
    pub payee_from_field: bool,
    /// Seconds from the timestamp the invoice is good for.
    pub expiry: u64,
    /// The `min_final_cltv_expiry_delta`.
    pub min_final_cltv_expiry: u64,
    /// Fallback addresses, in order.
    pub fallbacks: Vec<Fallback>,
    /// Route hints, one list of hops each.
    pub route_hints: Vec<Vec<RouteHop>>,
    /// The feature bits set, ascending.
    pub features: Vec<u16>,
    /// The payment secret.
    pub payment_secret: [u8; 32],
    /// The payment metadata.
    pub metadata: Option<Vec<u8>>,
    /// Fields of tags this reader does not define.
    pub unknown: Vec<UnknownField>,
}

/// Reads an invoice. The signature is verified or the invoice is refused.
pub fn decode(text: &str) -> Result<Invoice, Error> {
    let text = text.trim();
    let text = match text.get(..10) {
        Some(p) if p.eq_ignore_ascii_case("lightning:") => &text[10..],
        _ => text,
    };
    if text.len() > MAX_LEN {
        return Err(Error::TooLong);
    }
    let lower = text.to_ascii_lowercase();
    if lower != text && text.to_ascii_uppercase() != text {
        return Err(Error::MixedCase);
    }
    let (hrp, values) = bech32_values(&lower)?;
    let (network, amount_msat) = parse_hrp(hrp)?;
    if values.len() < 7 + SIGNATURE_VALUES {
        return Err(Error::TooShort);
    }
    let (body, sig_values) = values.split_at(values.len() - SIGNATURE_VALUES);
    let (sig, _) = to_bytes(sig_values);
    let timestamp = body[..7].iter().fold(0u64, |a, &v| a << 5 | u64::from(v));

    let mut inv = Invoice {
        network,
        amount_msat,
        timestamp,
        payment_hash: [0; 32],
        description: None,
        description_hash: None,
        payee: [0; 33],
        payee_from_field: false,
        expiry: DEFAULT_EXPIRY,
        min_final_cltv_expiry: DEFAULT_MIN_FINAL_CLTV,
        fallbacks: Vec::new(),
        route_hints: Vec::new(),
        features: Vec::new(),
        payment_secret: [0; 32],
        metadata: None,
        unknown: Vec::new(),
    };
    let seen = fields(body, &mut inv)?;
    if !seen[b'p' as usize] {
        return Err(Error::NoPaymentHash);
    }
    if !seen[b's' as usize] {
        return Err(Error::NoPaymentSecret);
    }
    if seen[b'd' as usize] == seen[b'h' as usize] {
        return Err(Error::Description);
    }
    for &bit in &inv.features {
        if bit.is_multiple_of(2) && !KNOWN_REQUIRED_FEATURES.contains(&bit) {
            return Err(Error::UnknownRequiredFeature(bit));
        }
    }

    verify(hrp, body, &sig, &mut inv)?;
    Ok(inv)
}

/// Reads the tagged fields into `inv`; which tags were seen.
fn fields(body: &[u8], inv: &mut Invoice) -> Result<[bool; 128], Error> {
    let mut seen = [false; 128];
    let mut at = 7;
    while at < body.len() {
        if at + 3 > body.len() {
            return Err(Error::Truncated);
        }
        let tag = CHARSET[usize::from(body[at])] as char;
        let len = usize::from(body[at + 1]) * 32 + usize::from(body[at + 2]);
        at += 3;
        if at + len > body.len() {
            return Err(Error::Truncated);
        }
        let data = &body[at..at + len];
        at += len;
        let known = matches!(tag, 'p' | 's' | 'h' | 'd' | 'n' | 'x' | 'c' | 'm' | '9');
        if known && core::mem::replace(&mut seen[tag as usize], true) {
            return Err(Error::Duplicate(tag));
        }
        let bad = Error::Field(tag);
        match tag {
            'p' => inv.payment_hash = fixed(data, 52).ok_or(bad)?,
            's' => inv.payment_secret = fixed(data, 52).ok_or(bad)?,
            'h' => inv.description_hash = Some(fixed(data, 52).ok_or(bad)?),
            'n' => {
                inv.payee = fixed(data, 53).ok_or(bad)?;
                PublicKey::from_slice(&inv.payee).map_err(|_| bad)?;
                inv.payee_from_field = true;
            }
            'd' => {
                let (b, zero) = to_bytes(data);
                if !zero {
                    return Err(bad);
                }
                inv.description = Some(String::from_utf8(b).map_err(|_| bad)?);
            }
            'm' => {
                let (b, zero) = to_bytes(data);
                if !zero {
                    return Err(bad);
                }
                inv.metadata = Some(b);
            }
            'x' | 'c' | '9' if data.first() == Some(&0) => return Err(bad),
            'x' => inv.expiry = number(data).ok_or(bad)?,
            'c' => inv.min_final_cltv_expiry = number(data).ok_or(bad)?,
            '9' => inv.features = feature_bits(data),
            'f' => match fallback(data).ok_or(bad)? {
                Some(f) => inv.fallbacks.push(f),
                None => inv.unknown.push(UnknownField {
                    tag,
                    data: data.to_vec(),
                }),
            },
            'r' => inv.route_hints.push(route(data).ok_or(bad)?),
            _ => inv.unknown.push(UnknownField {
                tag,
                data: data.to_vec(),
            }),
        }
    }
    Ok(seen)
}

/// Checks the signature, which covers the human-readable part and the data
/// before it zero-padded to whole bytes, and sets the payee when the `n`
/// field gave none.
fn verify(hrp: &str, body: &[u8], sig: &[u8], inv: &mut Invoice) -> Result<(), Error> {
    let mut signed = Vec::from(hrp.as_bytes());
    signed.extend(pack(body));
    let msg = Message::from_digest(sha256::Hash::hash(&signed).to_byte_array());
    let secp = Secp256k1::verification_only();
    if inv.payee_from_field {
        let key = PublicKey::from_slice(&inv.payee).map_err(|_| Error::Signature)?;
        let s = Signature::from_compact(&sig[..64]).map_err(|_| Error::Signature)?;
        secp.verify_ecdsa(&msg, &s, &key)
            .map_err(|_| Error::Signature)?;
    } else {
        let id = RecoveryId::from_i32(i32::from(sig[64])).map_err(|_| Error::Signature)?;
        let s = RecoverableSignature::from_compact(&sig[..64], id).map_err(|_| Error::Signature)?;
        inv.payee = secp
            .recover_ecdsa(&msg, &s)
            .map_err(|_| Error::Signature)?
            .serialize();
    }
    Ok(())
}

/// The human-readable part and the 5-bit data values with the checksum
/// checked and removed.
fn bech32_values(s: &str) -> Result<(&str, Vec<u8>), Error> {
    use bitcoin::bech32::Bech32;
    use bitcoin::bech32::primitives::decode::UncheckedHrpstring;
    let sep = s.rfind('1').ok_or(Error::Bech32)?;
    let unchecked = UncheckedHrpstring::new(s).map_err(|_| Error::Bech32)?;
    let checked = unchecked
        .validate_and_remove_checksum::<Bech32>()
        .map_err(|_| Error::Checksum)?;
    let values = checked
        .fe32_iter::<core::iter::Empty<u8>>()
        .map(|f| f.to_u8())
        .collect();
    Ok((&s[..sep], values))
}

/// `ln`, a network, then an optional amount.
fn parse_hrp(hrp: &str) -> Result<(Network, Option<u64>), Error> {
    let rest = hrp.strip_prefix("ln").ok_or(Error::Prefix)?;
    let (network, amount) = [
        ("bcrt", Network::Regtest),
        ("tbs", Network::Signet),
        ("tb", Network::Testnet),
        ("bc", Network::Mainnet),
    ]
    .into_iter()
    .find_map(|(p, n)| rest.strip_prefix(p).map(|a| (n, a)))
    .ok_or(Error::Prefix)?;
    if amount.is_empty() {
        return Ok((network, None));
    }
    let (digits, unit) = match amount.as_bytes()[amount.len() - 1] {
        b'm' | b'u' | b'n' | b'p' => amount.split_at(amount.len() - 1),
        _ => (amount, ""),
    };
    if digits.is_empty() || digits.len() > 20 || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return Err(Error::Amount);
    }
    if digits.starts_with('0') {
        return Err(Error::Amount);
    }
    let n: u128 = digits.parse().map_err(|_| Error::Amount)?;
    let msat = match unit {
        "" => n.checked_mul(100_000_000_000),
        "m" => n.checked_mul(100_000_000),
        "u" => n.checked_mul(100_000),
        "n" => n.checked_mul(100),
        _ => {
            if !n.is_multiple_of(10) {
                return Err(Error::Amount);
            }
            Some(n / 10)
        }
    };
    let msat = msat
        .and_then(|m| u64::try_from(m).ok())
        .ok_or(Error::Amount)?;
    Ok((network, Some(msat)))
}

/// 5-bit values to whole bytes, and whether the bits left over are zero.
fn to_bytes(values: &[u8]) -> (Vec<u8>, bool) {
    let mut out = Vec::with_capacity(values.len() * 5 / 8);
    let (mut acc, mut bits) = (0u32, 0u32);
    for &v in values {
        acc = acc << 5 | u32::from(v);
        bits += 5;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
            acc &= (1 << bits) - 1;
        }
    }
    (out, acc == 0)
}

/// 5-bit values to bytes with the last partial byte zero-padded.
fn pack(values: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(values.len() * 5 / 8 + 1);
    let (mut acc, mut bits) = (0u32, 0u32);
    for &v in values {
        acc = acc << 5 | u32::from(v);
        bits += 5;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
            acc &= (1 << bits) - 1;
        }
    }
    if bits > 0 {
        out.push((acc << (8 - bits)) as u8);
    }
    out
}

/// A field of exactly `n` values that is a whole-byte array with zero
/// padding.
fn fixed<const N: usize>(data: &[u8], n: usize) -> Option<[u8; N]> {
    if data.len() != n {
        return None;
    }
    let (b, zero) = to_bytes(data);
    if !zero {
        return None;
    }
    b.try_into().ok()
}

/// A big-endian number of up to 64 bits.
fn number(data: &[u8]) -> Option<u64> {
    let mut n = 0u128;
    for &v in data {
        n = n << 5 | u128::from(v);
        if n > u128::from(u64::MAX) {
            return None;
        }
    }
    Some(n as u64)
}

/// The set bits of a feature field, ascending.
fn feature_bits(data: &[u8]) -> Vec<u16> {
    let mut bits = Vec::new();
    let n = data.len();
    for (i, &v) in data.iter().enumerate() {
        for j in 0..5 {
            if v >> (4 - j) & 1 == 1 {
                bits.push(((n - 1 - i) * 5 + (4 - j)) as u16);
            }
        }
    }
    bits.sort_unstable();
    bits
}

/// `Some(Some(_))` a fallback, `Some(None)` one of a version this reader
/// does not define, `None` a malformed one.
fn fallback(data: &[u8]) -> Option<Option<Fallback>> {
    let (&version, rest) = data.split_first()?;
    let (bytes, zero) = to_bytes(rest);
    if !zero {
        return None;
    }
    Some(match version {
        0..=16 => {
            let v = WitnessVersion::try_from(version).ok()?;
            WitnessProgram::new(v, &bytes).ok()?;
            Some(Fallback::Witness {
                version,
                program: bytes,
            })
        }
        17 => Some(Fallback::P2pkh(bytes.try_into().ok()?)),
        18 => Some(Fallback::P2sh(bytes.try_into().ok()?)),
        _ => None,
    })
}

/// A route hint: whole hops of 51 bytes.
fn route(data: &[u8]) -> Option<Vec<RouteHop>> {
    let (b, zero) = to_bytes(data);
    if !zero || b.is_empty() || !b.len().is_multiple_of(51) {
        return None;
    }
    b.chunks(51)
        .map(|h| {
            let node_id: [u8; 33] = h[..33].try_into().ok()?;
            PublicKey::from_slice(&node_id).ok()?;
            Some(RouteHop {
                node_id,
                short_channel_id: u64::from_be_bytes(h[33..41].try_into().ok()?),
                fee_base_msat: u32::from_be_bytes(h[41..45].try_into().ok()?),
                fee_proportional_millionths: u32::from_be_bytes(h[45..49].try_into().ok()?),
                cltv_expiry_delta: u16::from_be_bytes(h[49..51].try_into().ok()?),
            })
        })
        .collect()
}
