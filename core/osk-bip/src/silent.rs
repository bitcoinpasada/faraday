//! Silent payments (BIP-352), the receiving side, with BIP-392's key
//! expression, BIP-321's URI and BIP-353's record text.
//!
//! Four layers, each usable on its own:
//!
//! - [`Receiver`] is a scan key and a spend key: [`Receiver::derive`]
//!   takes them from a [`MasterKey`] at BIP-352's paths, and
//!   [`Receiver::address`] and [`Receiver::labelled_address`] write the
//!   `sp1q…` string the payer is given. [`decode_address`] reads one
//!   back.
//! - [`Receiver::scan_key_text`] and [`Receiver::descriptor`] are
//!   BIP-392's `spscan1q…` and the `sp(…)` descriptor a scanner is
//!   handed. Both carry the scan private key, so both are secrets.
//! - [`uri`] is BIP-321's `bitcoin:?sp=…` and [`dns_record`] the TXT
//!   record line BIP-353 puts it in.
//! - [`scan`] and [`find_payments`] are the receiver's arithmetic:
//!   the sum of the eligible inputs' public keys, `input_hash`, the
//!   shared secret, and every output of a transaction that pays this
//!   wallet, with the label it was paid to.
//!
//! [`sender_outputs`] is the sending side's arithmetic and nothing
//! more: no PSBT, no signing. It exists because the published vectors
//! construct the outputs a receiver then has to find, and because a
//! sender and a receiver agreeing is the only proof either is right.
//!
//! Every text this module writes lives in a fixed buffer that zeroizes
//! on drop ([`Text`]), because the same code writes the scan private
//! key and the address, and `tools/lint-secrets.sh` holds this file to
//! the rule that a secret never reaches the heap as text.

use alloc::vec::Vec;
use core::fmt;

use bitcoin::bip32::DerivationPath;
use bitcoin::hashes::{Hash, hash160};
use bitcoin::secp256k1::{All, Parity, PublicKey, Scalar, Secp256k1, SecretKey, XOnlyPublicKey};
use bitcoin::{Amount, Transaction};
use osk_crypto::Zeroize;

use crate::keys::{Fingerprint, MasterKey, Network};
use crate::musig::{add_mod_order, negate_mod_order, tagged_hash};

/// BIP-43's purpose for silent payments.
pub const PURPOSE: u32 = 352;

/// The label reserved for a wallet's own change, which is never handed
/// out (BIP-352, "Labels for change").
pub const CHANGE_LABEL: u32 = 0;

/// How far `k` runs before scanning stops: BIP-352's `K_max`, the most
/// silent payment outputs one transaction can carry for one scan key.
pub const K_MAX: u32 = 2323;

/// BIP-341's `H`, the internal key of a taproot output with no key
/// path. An input spending one is skipped.
const NUMS: [u8; 32] = [
    0x50, 0x92, 0x9b, 0x74, 0xc1, 0xa0, 0x49, 0x54, 0xb7, 0x8b, 0x4b, 0x60, 0x35, 0xe9, 0x7a, 0x5e,
    0x07, 0x8a, 0x5a, 0x0f, 0x28, 0xec, 0x96, 0xd5, 0x47, 0xbf, 0xee, 0x9a, 0xce, 0x80, 0x3a, 0xc0,
];

/// Why an address, a key expression or a transaction could not be read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// Not a bech32m string with the human-readable part of a silent
    /// payment address, or the checksum does not hold.
    Address,
    /// The address states a version this build does not read: `v31`, or
    /// a `v0` whose payload is not 66 bytes.
    Version,
    /// One of the two points is not on the curve.
    Key,
    /// A scalar the protocol derived is zero or at or above the curve
    /// order, which BIP-352 says to fail on.
    Scalar,
    /// The transaction has no input BIP-352 derives a shared secret
    /// from, or the sum of their public keys is the point at infinity.
    NoInputs,
    /// The text did not fit the buffer it was written into.
    TooLong,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Error::Address => "not a silent payment address",
            Error::Version => "silent payment address version is not one this build reads",
            Error::Key => "not a public key on the curve",
            Error::Scalar => "the derived value is not a usable scalar",
            Error::NoInputs => "the transaction has no input a shared secret comes from",
            Error::TooLong => "the text is longer than the buffer it is written into",
        })
    }
}

impl core::error::Error for Error {}

// ---------------------------------------------------------------------
// Fixed-size text
// ---------------------------------------------------------------------

/// Characters an address takes: 3 for the longest human-readable part,
/// 1 separator, 1 version, 106 payload, 6 checksum.
pub const ADDRESS_CHARS: usize = 117;

/// A string this module writes, in a buffer that zeroizes on drop.
///
/// The addresses are public and the key expressions are not, and both
/// are written by the same code, so all of it is held to the stricter
/// rule.
pub struct Text<const N: usize> {
    buf: [u8; N],
    len: u16,
}

impl<const N: usize> Text<N> {
    /// An empty buffer.
    fn new() -> Self {
        Text {
            buf: [0; N],
            len: 0,
        }
    }

    /// The text written so far.
    pub fn as_str(&self) -> &str {
        core::str::from_utf8(&self.buf[..usize::from(self.len)]).expect("ascii is written")
    }

    /// How many bytes it holds.
    pub fn len(&self) -> usize {
        usize::from(self.len)
    }

    /// Whether nothing has been written.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

impl<const N: usize> fmt::Write for Text<N> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let n = usize::from(self.len);
        let bytes = s.as_bytes();
        if n + bytes.len() > N {
            return Err(fmt::Error);
        }
        self.buf[n..n + bytes.len()].copy_from_slice(bytes);
        self.len += bytes.len() as u16;
        Ok(())
    }
}

impl<const N: usize> Drop for Text<N> {
    fn drop(&mut self) {
        self.buf.zeroize();
        self.len = 0;
    }
}

/// A silent payment address, `sp1q…` or `tsp1q…`.
pub type AddressText = Text<ADDRESS_CHARS>;
/// BIP-392's `spscan1q…` key expression, which carries the scan private
/// key.
pub type KeyText = Text<128>;
/// BIP-392's `sp(…)` descriptor, with the key origin before the key.
pub type DescriptorText = Text<224>;
/// BIP-321's `bitcoin:?sp=…`.
pub type UriText = Text<160>;
/// BIP-353's TXT record line.
pub type RecordText = Text<512>;

// ---------------------------------------------------------------------
// Bech32m with a version character
// ---------------------------------------------------------------------

/// Writes `payload` under `hrp` as bech32m with version 0 (`q`) first,
/// which is how BIP-352 and BIP-392 encode every string they define.
fn encode_v0<const N: usize>(hrp: &str, payload: &[u8]) -> Result<Text<N>, Error> {
    use bitcoin::bech32::primitives::iter::{ByteIterExt, Fe32IterExt};
    use bitcoin::bech32::{Bech32m, Fe32, Hrp};
    let hrp = Hrp::parse_unchecked(hrp);
    let mut out = Text::<N>::new();
    let chars = payload
        .iter()
        .copied()
        .bytes_to_fes()
        .with_checksum::<Bech32m>(&hrp)
        .with_witness_version(Fe32::Q)
        .chars();
    let mut buf = [0u8; 4];
    for c in chars {
        fmt::Write::write_str(&mut out, c.encode_utf8(&mut buf)).map_err(|_| Error::TooLong)?;
    }
    Ok(out)
}

/// Reads a bech32m string whose first data character is the version,
/// returning the version and the payload.
fn decode_v0(text: &str, hrp: &str, payload: &mut [u8]) -> Result<(), Error> {
    use bitcoin::bech32::Bech32m;
    use bitcoin::bech32::primitives::decode::CheckedHrpstring;
    let mut checked = CheckedHrpstring::new::<Bech32m>(text).map_err(|_| Error::Address)?;
    if !checked.hrp().as_str().eq_ignore_ascii_case(hrp) {
        return Err(Error::Address);
    }
    let version = checked.remove_witness_version().ok_or(Error::Version)?;
    if version.to_u8() != 0 {
        return Err(Error::Version);
    }
    let mut n = 0;
    for byte in checked.byte_iter() {
        if n == payload.len() {
            return Err(Error::Version);
        }
        payload[n] = byte;
        n += 1;
    }
    if n != payload.len() {
        return Err(Error::Version);
    }
    // The payload is not a whole number of five-bit groups, so the last
    // group carries padding bits that must be zero. Writing the bytes
    // back is the shortest way to say so.
    let again = encode_v0::<256>(hrp, payload)?;
    let mut same = again.as_str().len() == text.len();
    for (a, b) in again.as_str().bytes().zip(text.bytes()) {
        same &= a == b.to_ascii_lowercase();
    }
    if !same {
        return Err(Error::Address);
    }
    Ok(())
}

/// The human-readable part of an address on `network`.
pub fn hrp(network: Network) -> &'static str {
    if network.is_mainnet() { "sp" } else { "tsp" }
}

/// The human-readable part of BIP-392's scan key expression on
/// `network`.
fn scan_hrp(network: Network) -> &'static str {
    if network.is_mainnet() {
        "spscan"
    } else {
        "tspscan"
    }
}

/// Whether `text` begins the way a silent payment address does, which
/// is what routing reads before parsing anything.
pub fn looks_like_address(text: &str) -> bool {
    let text = text.trim();
    let lower = |p: &str| text.len() > p.len() && text[..p.len()].eq_ignore_ascii_case(p);
    lower("sp1q") || lower("tsp1q")
}

/// The silent payment address `text` carries, where it carries one:
/// the address itself, or the `sp` parameter of a BIP-321 URI.
///
/// The URI's other parameters are left alone: this reads the one thing
/// this device has a use for and says nothing about the rest.
pub fn address_of(text: &str) -> Option<&str> {
    let text = text.trim();
    if looks_like_address(text) {
        return Some(text);
    }
    let rest = text
        .get(..8)?
        .eq_ignore_ascii_case("bitcoin:")
        .then(|| &text[8..])?;
    let query = rest.split_once('?')?.1;
    for part in query.split('&') {
        if let Some(value) = part.strip_prefix("sp=")
            && looks_like_address(value)
        {
            return Some(value);
        }
    }
    None
}

/// The two public keys an address states, and the network its
/// human-readable part names.
pub fn decode_address(text: &str) -> Result<(Network, PublicKey, PublicKey), Error> {
    let text = text.trim();
    let (network, hrp) = if text.len() > 4 && text[..2].eq_ignore_ascii_case("sp") {
        (Network::Mainnet, "sp")
    } else {
        (Network::Testnet, "tsp")
    };
    let mut payload = [0u8; 66];
    decode_v0(text, hrp, &mut payload)?;
    let scan = PublicKey::from_slice(&payload[..33]).map_err(|_| Error::Key)?;
    let spend = PublicKey::from_slice(&payload[33..]).map_err(|_| Error::Key)?;
    Ok((network, scan, spend))
}

/// The address of a scan key and a spend key, which is all an address
/// is (BIP-352, "Address encoding").
pub fn encode_address(
    network: Network,
    scan: &PublicKey,
    spend: &PublicKey,
) -> Result<AddressText, Error> {
    let mut payload = [0u8; 66];
    payload[..33].copy_from_slice(&scan.serialize());
    payload[33..].copy_from_slice(&spend.serialize());
    encode_v0(hrp(network), &payload)
}

/// BIP-321's URI for an address: the scheme, no address before the
/// query, and the address as the `sp` parameter.
pub fn uri(address: &str) -> Result<UriText, Error> {
    let mut out = UriText::new();
    fmt::Write::write_str(&mut out, "bitcoin:?sp=").map_err(|_| Error::TooLong)?;
    fmt::Write::write_str(&mut out, address).map_err(|_| Error::TooLong)?;
    Ok(out)
}

/// BIP-353's TXT record: the owner name a resolver looks the address up
/// at, and the URI as the record's one character-string.
pub fn dns_record(user: &str, domain: &str, address: &str) -> Result<RecordText, Error> {
    let uri = uri(address)?;
    let mut out = RecordText::new();
    let mut write = |s: &str| fmt::Write::write_str(&mut out, s).map_err(|_| Error::TooLong);
    write(user)?;
    write(".user._bitcoin-payment.")?;
    write(domain)?;
    write(". IN TXT \"")?;
    write(uri.as_str())?;
    write("\"")?;
    Ok(out)
}

// ---------------------------------------------------------------------
// The receiver's keys
// ---------------------------------------------------------------------

/// The scan and spend keys of one silent payments wallet.
///
/// The scan key is private: it finds the payments, and anyone holding
/// it sees every payment this wallet receives. The spend key is here as
/// a public key alone, because receiving never needs the private half.
pub struct Receiver {
    scan: SecretKey,
    spend: PublicKey,
    network: Network,
}

impl Drop for Receiver {
    fn drop(&mut self) {
        self.scan.non_secure_erase();
    }
}

/// BIP-352's scan path for `network` and `account`.
pub fn scan_path(network: Network, account: u32) -> DerivationPath {
    path(network, account, 1)
}

/// BIP-352's spend path for `network` and `account`.
pub fn spend_path(network: Network, account: u32) -> DerivationPath {
    path(network, account, 0)
}

/// `m/352h/coin_typeh/accounth/changeh/0`, the shape both keys have.
fn path(network: Network, account: u32, change: u32) -> DerivationPath {
    use bitcoin::bip32::ChildNumber;
    let hardened = |i: u32| ChildNumber::from_hardened_idx(i).expect("index below 2^31");
    DerivationPath::from(alloc::vec![
        hardened(PURPOSE),
        hardened(network.coin_type()),
        hardened(account),
        hardened(change),
        ChildNumber::from_normal_idx(0).expect("zero"),
    ])
}

impl Receiver {
    /// The wallet at BIP-352's paths under `master`, for `account`.
    pub fn derive(master: &MasterKey, account: u32) -> Receiver {
        let network = master.network();
        let scan = master.derive(&scan_path(network, account));
        let spend = master.derive(&spend_path(network, account));
        Receiver {
            scan: *scan.secret_key(),
            spend: spend.secret_key().public_key(master.secp()),
            network,
        }
    }

    /// The wallet those two keys make, for a caller holding them
    /// already.
    pub fn new(scan: SecretKey, spend: PublicKey, network: Network) -> Receiver {
        Receiver {
            scan,
            spend,
            network,
        }
    }

    /// The network its addresses are on.
    pub fn network(&self) -> Network {
        self.network
    }

    /// The scan public key, which the address publishes.
    pub fn scan_public_key(&self, secp: &Secp256k1<All>) -> PublicKey {
        self.scan.public_key(secp)
    }

    /// The spend public key, which the address publishes.
    pub fn spend_public_key(&self) -> PublicKey {
        self.spend
    }

    /// The address with no label.
    pub fn address(&self, secp: &Secp256k1<All>) -> Result<AddressText, Error> {
        encode_address(self.network, &self.scan_public_key(secp), &self.spend)
    }

    /// The tweak label `m` adds to the spend key.
    pub fn label_tweak(&self, m: u32) -> Result<[u8; 32], Error> {
        let tweak = tagged_hash(
            b"BIP0352/Label",
            &[&self.scan.secret_bytes(), &m.to_be_bytes()],
        );
        checked(tweak)
    }

    /// The point that tweak stands for, which is what a received output
    /// is matched against.
    pub fn label_point(&self, secp: &Secp256k1<All>, m: u32) -> Result<PublicKey, Error> {
        let mut tweak = self.label_tweak(m)?;
        let key = SecretKey::from_slice(&tweak).map_err(|_| Error::Scalar)?;
        tweak.zeroize();
        Ok(key.public_key(secp))
    }

    /// The spend key with label `m` on it, which is what the labelled
    /// address publishes.
    pub fn labelled_spend_key(&self, secp: &Secp256k1<All>, m: u32) -> Result<PublicKey, Error> {
        let mut tweak = self.label_tweak(m)?;
        let scalar = Scalar::from_be_bytes(tweak).map_err(|_| Error::Scalar)?;
        tweak.zeroize();
        self.spend
            .add_exp_tweak(secp, &scalar)
            .map_err(|_| Error::Scalar)
    }

    /// The address for label `m`. Label 0 is the change label and is
    /// never handed out, so nothing here offers it a name; it is
    /// derivable all the same, which is what recovery needs.
    pub fn labelled_address(&self, secp: &Secp256k1<All>, m: u32) -> Result<AddressText, Error> {
        let spend = self.labelled_spend_key(secp, m)?;
        encode_address(self.network, &self.scan_public_key(secp), &spend)
    }

    /// BIP-392's `spscan1q…`: the scan private key and the spend public
    /// key. It is a secret, and the only one this wallet exports.
    pub fn scan_key_text(&self) -> Result<KeyText, Error> {
        let mut payload = [0u8; 65];
        payload[..32].copy_from_slice(&self.scan.secret_bytes());
        payload[32..].copy_from_slice(&self.spend.serialize());
        let out = encode_v0(scan_hrp(self.network), &payload);
        payload.zeroize();
        out
    }

    /// BIP-392's `sp(KEY)` over that key, with the key origin the BIP
    /// puts before it where the caller knows one.
    pub fn descriptor(&self, origin: Option<(Fingerprint, u32)>) -> Result<DescriptorText, Error> {
        let key = self.scan_key_text()?;
        let mut out = DescriptorText::new();
        let mut write = |s: &str| fmt::Write::write_str(&mut out, s).map_err(|_| Error::TooLong);
        write("sp(")?;
        if let Some((fingerprint, account)) = origin {
            write("[")?;
            let hex = fingerprint.to_hex();
            write(core::str::from_utf8(&hex).map_err(|_| Error::TooLong)?)?;
            write("/352h/")?;
            let mut digits = Text::<12>::new();
            fmt::Write::write_fmt(
                &mut digits,
                format_args!("{}h/{}h", self.network.coin_type(), account),
            )
            .map_err(|_| Error::TooLong)?;
            write(digits.as_str())?;
            write("]")?;
        }
        write(key.as_str())?;
        write(")")?;
        Ok(out)
    }
}

/// A scalar BIP-352 derived, refused where it is zero or at or above
/// the curve order.
fn checked(value: [u8; 32]) -> Result<[u8; 32], Error> {
    if value == [0u8; 32] || crate::musig::at_least_order(&value) {
        return Err(Error::Scalar);
    }
    Ok(value)
}

// ---------------------------------------------------------------------
// The inputs a shared secret comes from
// ---------------------------------------------------------------------

/// One input of a transaction, as much of it as BIP-352 reads.
#[derive(Debug, Clone, Copy)]
pub struct ScanInput<'a> {
    /// The outpoint as the transaction serializes it: 32 bytes of txid
    /// least significant byte first, then the index in four bytes least
    /// significant byte first.
    pub outpoint: [u8; 36],
    /// The input's scriptSig.
    pub script_sig: &'a [u8],
    /// The input's witness, item by item.
    pub witness: &'a [&'a [u8]],
    /// The scriptPubKey of the output it spends.
    pub prevout: &'a [u8],
}

/// `OP_DUP OP_HASH160 <20> … OP_EQUALVERIFY OP_CHECKSIG`.
fn is_p2pkh(script: &[u8]) -> bool {
    script.len() == 25
        && script[0] == 0x76
        && script[1] == 0xa9
        && script[2] == 0x14
        && script[23] == 0x88
        && script[24] == 0xac
}

/// `OP_HASH160 <20> OP_EQUAL`.
fn is_p2sh(script: &[u8]) -> bool {
    script.len() == 23 && script[0] == 0xa9 && script[1] == 0x14 && script[22] == 0x87
}

/// `OP_0 <20>`.
fn is_p2wpkh(script: &[u8]) -> bool {
    script.len() == 22 && script[0] == 0x00 && script[1] == 0x14
}

/// `OP_1 <32>`.
fn is_p2tr(script: &[u8]) -> bool {
    script.len() == 34 && script[0] == 0x51 && script[1] == 0x20
}

/// Whether the script pays to a SegWit version above 1, which BIP-352
/// says to skip the whole transaction for.
pub fn is_future_segwit(script: &[u8]) -> bool {
    if script.len() < 4 || script.len() > 42 {
        return false;
    }
    let version = script[0];
    // `OP_2` through `OP_16`.
    (0x52..=0x60).contains(&version) && usize::from(script[1]) == script.len() - 2
}

/// Whether BIP-352 reads this input's public key out of its signature
/// data rather than out of the output it spends.
///
/// A taproot input carries its public key in the output's own script,
/// so it is readable from an unsigned transaction; the other three
/// carry only a hash there, and the key is in the scriptSig or the
/// witness.
pub fn needs_signature_data(script: &[u8]) -> bool {
    is_p2pkh(script) || is_p2sh(script) || is_p2wpkh(script)
}

/// Whether BIP-352 derives a shared secret from an input spending this
/// output at all.
pub fn is_eligible(script: &[u8]) -> bool {
    needs_signature_data(script) || is_p2tr(script)
}

/// The public key BIP-352 takes from one input, where it takes one.
///
/// `None` is an input the protocol ignores: a script type not on its
/// list, a taproot input under BIP-341's `H`, an uncompressed key, or a
/// scriptSig that holds no key matching the hash it spends.
pub fn input_public_key(input: &ScanInput<'_>) -> Option<PublicKey> {
    let spk = input.prevout;
    if is_p2pkh(spk) {
        let hash = &spk[3..23];
        let sig = input.script_sig;
        // From the back, in a 33-byte window: the key is the last push
        // of a standard scriptSig and somewhere inside a malleated one.
        for i in (33..=sig.len()).rev() {
            let candidate = &sig[i - 33..i];
            if hash160::Hash::hash(candidate).to_byte_array() == hash
                && let Ok(key) = PublicKey::from_slice(candidate)
            {
                return Some(key);
            }
        }
    }
    if is_p2sh(spk) {
        let redeem = input.script_sig.get(1..).unwrap_or(&[]);
        if is_p2wpkh(redeem)
            && let Some(last) = input.witness.last()
            && let Ok(key) = compressed(last)
        {
            return Some(key);
        }
    }
    if is_p2wpkh(spk)
        && let Some(last) = input.witness.last()
        && let Ok(key) = compressed(last)
    {
        return Some(key);
    }
    if is_p2tr(spk) {
        let mut stack = input.witness;
        if stack.is_empty() {
            return None;
        }
        // BIP-341's annex, where there is one, is not part of the spend.
        if stack.len() > 1 && stack.last().is_some_and(|a| a.first() == Some(&0x50)) {
            stack = &stack[..stack.len() - 1];
        }
        if stack.len() > 1 {
            let control = stack.last().copied().unwrap_or(&[]);
            if control.get(1..33) == Some(&NUMS[..]) {
                return None;
            }
        }
        if let Ok(x_only) = XOnlyPublicKey::from_slice(&spk[2..]) {
            return Some(PublicKey::from_x_only_public_key(x_only, Parity::Even));
        }
    }
    None
}

/// A 33-byte compressed key, and nothing else: BIP-352 takes no
/// uncompressed key from any input.
fn compressed(bytes: &[u8]) -> Result<PublicKey, Error> {
    if bytes.len() != 33 {
        return Err(Error::Key);
    }
    PublicKey::from_slice(bytes).map_err(|_| Error::Key)
}

/// The sum of the eligible inputs' public keys, and `input_hash` over
/// it and the lexicographically smallest outpoint of the transaction.
///
/// `Err` is a transaction BIP-352 says to skip: no eligible input, a
/// sum that is the point at infinity, an input spending a SegWit
/// version this build does not read, or an `input_hash` that is not a
/// scalar.
pub fn input_sum_and_hash(inputs: &[ScanInput<'_>]) -> Result<(PublicKey, [u8; 32]), Error> {
    if inputs.iter().any(|i| is_future_segwit(i.prevout)) {
        return Err(Error::NoInputs);
    }
    let keys: Vec<PublicKey> = inputs.iter().filter_map(input_public_key).collect();
    let refs: Vec<&PublicKey> = keys.iter().collect();
    if refs.is_empty() {
        return Err(Error::NoInputs);
    }
    let sum = PublicKey::combine_keys(&refs).map_err(|_| Error::NoInputs)?;
    let smallest = inputs
        .iter()
        .map(|i| i.outpoint)
        .min()
        .ok_or(Error::NoInputs)?;
    let hash = tagged_hash(b"BIP0352/Inputs", &[&smallest, &sum.serialize()]);
    Ok((sum, checked(hash)?))
}

// ---------------------------------------------------------------------
// Scanning
// ---------------------------------------------------------------------

/// One output of a transaction that pays this wallet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Found {
    /// Its place among the outputs that were searched.
    pub index: usize,
    /// The taproot output key, x-only, which is what the chain carries.
    pub key: [u8; 32],
    /// The label it was paid to, and `None` for the address itself.
    /// Label 0 is the change label.
    pub label: Option<u32>,
}

/// The outputs of a transaction that pay `receiver`, in the order the
/// protocol finds them.
///
/// `outputs` are the taproot output keys of the transaction, x-only.
/// `labels` are the labels this wallet has handed out, which the caller
/// states; BIP-352 asks that the change label be among them whenever
/// anything is scanned.
pub fn scan(
    secp: &Secp256k1<All>,
    receiver: &Receiver,
    inputs: &[ScanInput<'_>],
    outputs: &[[u8; 32]],
    labels: &[u32],
) -> Result<Vec<Found>, Error> {
    let (sum, input_hash) = input_sum_and_hash(inputs)?;
    let scalar = Scalar::from_be_bytes(input_hash).map_err(|_| Error::Scalar)?;
    let tweaked = sum.mul_tweak(secp, &scalar).map_err(|_| Error::Scalar)?;
    let shared = tweaked
        .mul_tweak(secp, &Scalar::from(receiver.scan))
        .map_err(|_| Error::Scalar)?;
    // Every label the wallet knows, as the point a matched output is
    // compared against.
    let mut points: Vec<(u32, PublicKey)> = Vec::new();
    for m in labels {
        points.push((*m, receiver.label_point(secp, *m)?));
    }
    let mut left: Vec<usize> = (0..outputs.len()).collect();
    let mut found = Vec::new();
    let mut k: u32 = 0;
    while k < K_MAX && !left.is_empty() {
        let mut t_k = checked(tagged_hash(
            b"BIP0352/SharedSecret",
            &[&shared.serialize(), &k.to_be_bytes()],
        ))?;
        let scalar = Scalar::from_be_bytes(t_k).map_err(|_| Error::Scalar)?;
        t_k.zeroize();
        let p_k = receiver
            .spend
            .add_exp_tweak(secp, &scalar)
            .map_err(|_| Error::Scalar)?;
        let p_k_x = p_k.x_only_public_key().0.serialize();
        let mut hit = None;
        for (slot, index) in left.iter().enumerate() {
            let output = outputs[*index];
            if output == p_k_x {
                hit = Some((slot, *index, None));
                break;
            }
            if let Some(label) = label_of(secp, &output, &p_k, &points) {
                hit = Some((slot, *index, Some(label)));
                break;
            }
        }
        let Some((slot, index, label)) = hit else {
            break;
        };
        left.remove(slot);
        found.push(Found {
            index,
            key: outputs[index],
            label,
        });
        k += 1;
    }
    Ok(found)
}

/// Which of the wallet's labels `output` was paid to, where it was paid
/// to one: `output − P_k` under either parity of the output key.
fn label_of(
    secp: &Secp256k1<All>,
    output: &[u8; 32],
    p_k: &PublicKey,
    points: &[(u32, PublicKey)],
) -> Option<u32> {
    if points.is_empty() {
        return None;
    }
    let x_only = XOnlyPublicKey::from_slice(output).ok()?;
    let negated = p_k.negate(secp);
    for parity in [Parity::Even, Parity::Odd] {
        let point = PublicKey::from_x_only_public_key(x_only, parity);
        let Ok(difference) = point.combine(&negated) else {
            continue;
        };
        if let Some((m, _)) = points.iter().find(|(_, p)| *p == difference) {
            return Some(*m);
        }
    }
    None
}

/// One output of a transaction that pays this wallet, with what it pays.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Payment {
    /// Which output of the transaction it is.
    pub vout: u32,
    /// What it pays.
    pub amount: Amount,
    /// The label it was paid to, and `None` for the address itself.
    pub label: Option<u32>,
}

/// The outputs of `tx` that pay `receiver`, with their amounts.
///
/// `prevouts` are the scriptPubKeys of the outputs the transaction
/// spends, one per input in order.
pub fn find_payments(
    secp: &Secp256k1<All>,
    receiver: &Receiver,
    tx: &Transaction,
    prevouts: &[&[u8]],
    labels: &[u32],
) -> Result<Vec<Payment>, Error> {
    if prevouts.len() != tx.input.len() {
        return Err(Error::NoInputs);
    }
    let witnesses: Vec<Vec<&[u8]>> = tx
        .input
        .iter()
        .map(|i| i.witness.iter().collect::<Vec<&[u8]>>())
        .collect();
    let inputs: Vec<ScanInput<'_>> = tx
        .input
        .iter()
        .enumerate()
        .map(|(i, txin)| {
            let mut outpoint = [0u8; 36];
            let txid = txin.previous_output.txid.to_raw_hash().to_byte_array();
            outpoint[..32].copy_from_slice(&txid);
            outpoint[32..].copy_from_slice(&txin.previous_output.vout.to_le_bytes());
            ScanInput {
                outpoint,
                script_sig: txin.script_sig.as_bytes(),
                witness: witnesses[i].as_slice(),
                prevout: prevouts[i],
            }
        })
        .collect();
    // The taproot outputs, and which output of the transaction each is.
    let mut keys: Vec<[u8; 32]> = Vec::new();
    let mut vouts: Vec<u32> = Vec::new();
    for (vout, out) in tx.output.iter().enumerate() {
        let script = out.script_pubkey.as_bytes();
        if is_p2tr(script) {
            let mut key = [0u8; 32];
            key.copy_from_slice(&script[2..]);
            keys.push(key);
            vouts.push(vout as u32);
        }
    }
    let found = scan(secp, receiver, &inputs, &keys, labels)?;
    Ok(found
        .into_iter()
        .map(|f| Payment {
            vout: vouts[f.index],
            amount: tx.output[vouts[f.index] as usize].value,
            label: f.label,
        })
        .collect())
}

// ---------------------------------------------------------------------
// The sending side, as far as a vector needs it
// ---------------------------------------------------------------------

/// The private key of one eligible input, with whether it spends a
/// taproot output — which decides whether BIP-352 negates it first.
#[derive(Debug, Clone, Copy)]
pub struct SendKey {
    /// The key that spends the input.
    pub key: SecretKey,
    /// Whether the output it spends is taproot.
    pub taproot: bool,
}

/// The taproot output keys a sender makes for one group of addresses
/// sharing a scan key, in the order the spend keys are given.
///
/// `keys` are the private keys of the inputs BIP-352 derives a shared
/// secret from; `outpoints` are every outpoint of the transaction,
/// eligible or not, because the smallest one is what `input_hash`
/// commits to.
///
/// This is BIP-352's sender arithmetic and nothing else: it signs
/// nothing and writes no transaction. It exists so that the published
/// vectors can be checked from both sides, and so that a wallet finding
/// a payment can be held against a sender making it.
pub fn sender_outputs(
    secp: &Secp256k1<All>,
    keys: &[SendKey],
    outpoints: &[[u8; 36]],
    scan_key: &PublicKey,
    spend_keys: &[PublicKey],
) -> Result<Vec<[u8; 32]>, Error> {
    if keys.is_empty() {
        return Err(Error::NoInputs);
    }
    // The sum runs over raw scalars, not over private keys: an
    // intermediate sum of zero is no key and is still a step on the way
    // to a sum that is one.
    let mut acc = [0u8; 32];
    for input in keys {
        let mut bytes = input.key.secret_bytes();
        if input.taproot && input.key.public_key(secp).x_only_public_key().1 == Parity::Odd {
            bytes = negate_mod_order(&bytes);
        }
        let next = add_mod_order(&acc, &bytes);
        bytes.zeroize();
        acc = next;
    }
    let a = SecretKey::from_slice(&acc).map_err(|_| Error::Scalar)?;
    acc.zeroize();
    let point = a.public_key(secp);
    let smallest = outpoints.iter().copied().min().ok_or(Error::NoInputs)?;
    let input_hash = checked(tagged_hash(
        b"BIP0352/Inputs",
        &[&smallest, &point.serialize()],
    ))?;
    let scalar = Scalar::from_be_bytes(input_hash).map_err(|_| Error::Scalar)?;
    let shared = scan_key
        .mul_tweak(secp, &scalar)
        .and_then(|p| p.mul_tweak(secp, &Scalar::from(a)))
        .map_err(|_| Error::Scalar)?;
    let mut out = Vec::new();
    for (k, spend) in spend_keys.iter().enumerate() {
        let mut t_k = checked(tagged_hash(
            b"BIP0352/SharedSecret",
            &[&shared.serialize(), &(k as u32).to_be_bytes()],
        ))?;
        let tweak = Scalar::from_be_bytes(t_k).map_err(|_| Error::Scalar)?;
        t_k.zeroize();
        let p = spend
            .add_exp_tweak(secp, &tweak)
            .map_err(|_| Error::Scalar)?;
        out.push(p.x_only_public_key().0.serialize());
    }
    Ok(out)
}
